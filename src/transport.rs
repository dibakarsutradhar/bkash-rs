//! HTTP transport layer built on `reqwest`.
//!
//! The [`Transport`] owns a [`reqwest::Client`] and a [`Config`], and
//! handles:
//!
//! - Attaching `Authorization: Bearer <id_token>` and `X-APP-Key` headers.
//! - Decoding the bKash response envelope (success: `statusCode == "0000"` /
//!   no `errorCode`; failure: any other body).
//! - Mapping HTTP / API errors to [`Error`].
//! - Retrying once on HTTP 401 after a force-regrant.
//! - Retrying transient failures (network, 5xx, `503` errorCode) with
//!   exponential backoff.
//! - Granting / refreshing OAuth tokens via [`TokenManager`].

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::{debug, warn};

use crate::config::{Config, Product};
use crate::error::{Error, ErrorCode};
use crate::token::{TokenCache, TokenManager, TokenTransport};

/// Header name used by bKash to carry the app key.
pub const X_APP_KEY: HeaderName = HeaderName::from_static("x-app-key");
/// Header name used by bKash to carry the bearer token.
const BEARER_PREFIX: &str = "Bearer ";

/// Per-request options.
#[derive(Debug, Default, Clone)]
pub struct RequestOptions {
    /// Skip transient retries (used internally for the post-401 retry).
    pub skip_retry: bool,
    /// Override the configured timeout for this single request.
    pub timeout: Option<Duration>,
}

impl RequestOptions {
    /// Create options with no overrides.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// HTTP transport. Cheap to clone (it owns an `Arc` internally).
#[derive(Clone)]
pub struct Transport {
    inner: Arc<TransportInner>,
}

struct TransportInner {
    http: reqwest::Client,
    config: Config,
    token_cache: TokenCache,
}

impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("config", &self.inner.config)
            .field("http", &"<reqwest::Client>")
            .finish_non_exhaustive()
    }
}

impl Transport {
    /// Build a [`Transport`] from a [`Config`]. The HTTP client is created
    /// (or reused) per the config.
    pub async fn new(config: Config) -> Result<Self, Error> {
        let http = if let Some(c) = &config.http_client {
            c.clone()
        } else {
            reqwest::Client::builder()
                .timeout(config.timeout)
                .build()
                .map_err(Error::from)?
        };
        Ok(Self {
            inner: Arc::new(TransportInner {
                http,
                config,
                token_cache: TokenCache::new(),
            }),
        })
    }

    /// Access the configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// Access the shared token cache.
    #[must_use]
    pub fn token_cache(&self) -> &TokenCache {
        &self.inner.token_cache
    }

    /// Construct a [`TokenManager`] backed by this transport.
    #[must_use]
    pub fn token_manager(&self) -> TokenManager<Transport> {
        TokenManager::new(
            self.inner.config.clone(),
            self.inner.token_cache.clone(),
            Arc::new(self.clone()),
        )
    }

    /// Build the full URL for a `(product, path)` pair.
    pub fn url_for(&self, product: Product, path: &str) -> String {
        let base = self
            .inner
            .config
            .base_url
            .clone()
            .unwrap_or_else(|| self.inner.config.environment.base_url(product));
        let trimmed_base = base.trim_end_matches('/');
        let trimmed_path = path.trim_start_matches('/');
        let url = format!("{trimmed_base}/{trimmed_path}");
        tracing::debug!(%url, "url_for");
        url
    }

    /// Send an authenticated request. The token is obtained (and cached)
    /// before the request is sent.
    pub async fn request<P, R>(
        &self,
        product: Product,
        method: reqwest::Method,
        path: &str,
        body: Option<&P>,
    ) -> Result<R, Error>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        self.request_with(product, method, path, body, &RequestOptions::default())
            .await
    }

    /// Send an authenticated request with a dynamically-formatted path.
    ///
    /// Identical to [`request`](Self::request) except the path is taken as
    /// an owned `String` so callers can interpolate path parameters
    /// (e.g. `format!("/tokenized/checkout/execute/{paymentID}")`).
    pub async fn request_path<P, R>(
        &self,
        product: Product,
        method: reqwest::Method,
        path: String,
        body: Option<&P>,
    ) -> Result<R, Error>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        self.request_with_path(product, method, path, body, &RequestOptions::default())
            .await
    }

    /// Send an authenticated request with extra per-request options.
    pub async fn request_with<P, R>(
        &self,
        product: Product,
        method: reqwest::Method,
        path: &str,
        body: Option<&P>,
        options: &RequestOptions,
    ) -> Result<R, Error>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        self.request_with_path(product, method, path.to_string(), body, options)
            .await
    }

    /// Send an authenticated request with extra per-request options and a
    /// dynamically-formatted path.
    pub async fn request_with_path<P, R>(
        &self,
        product: Product,
        method: reqwest::Method,
        path: String,
        body: Option<&P>,
        options: &RequestOptions,
    ) -> Result<R, Error>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        let tm = self.token_manager();
        let token = tm.ensure_token(product).await?;

        let max_attempts = if options.skip_retry {
            1
        } else {
            1 + self.inner.config.max_retries
        };

        let mut attempt: u32 = 0;
        let mut force_regrant_done = false;
        let mut current_token = token;
        loop {
            attempt += 1;
            match self
                .send_once::<P, R>(
                    product,
                    method.clone(),
                    &path,
                    body,
                    &current_token.id_token,
                    options,
                )
                .await
            {
                Ok(r) => return Ok(r),
                Err(Error::Api { status: 401, .. }) if !force_regrant_done => {
                    warn!(%path, "401 from bKash; forcing re-grant and retrying");
                    self.inner.token_cache.clear().await;
                    current_token = tm.force_grant(product).await?;
                    force_regrant_done = true;
                    // The post-regrant call still participates in the
                    // transient retry loop below; do not return here.
                    continue;
                }
                Err(Error::Api {
                    status: 401,
                    message,
                    ..
                }) if force_regrant_done => {
                    // We already force-regranted once and still got 401;
                    // surface as a credential failure rather than retrying.
                    return Err(Error::Auth(format!("401 after force-regrant: {message}")));
                }
                Err(e) if e.is_transient() && attempt < max_attempts => {
                    let backoff = backoff_for(attempt);
                    warn!(
                        error = %e,
                        attempt,
                        backoff_ms = backoff.as_millis() as u64,
                        "transient error; retrying"
                    );
                    tokio::time::sleep(backoff).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn send_once<P, R>(
        &self,
        product: Product,
        method: reqwest::Method,
        path: &str,
        body: Option<&P>,
        id_token: &str,
        options: &RequestOptions,
    ) -> Result<R, Error>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        let url = self.url_for(product, path);
        let mut headers = HeaderMap::new();
        let auth_value = format!("{BEARER_PREFIX}{id_token}");
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .map_err(|_| Error::Auth("invalid token characters".into()))?,
        );
        headers.insert(
            X_APP_KEY,
            HeaderValue::from_str(&self.inner.config.app_key)
                .map_err(|_| Error::Config("invalid app_key characters".into()))?,
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("application/json"),
        );

        let mut req = self.inner.http.request(method, &url).headers(headers);
        if let Some(t) = options.timeout {
            req = req.timeout(t);
        }
        if let Some(b) = body {
            req = req.json(b);
        }

        debug!(%url, "sending bKash request");
        let resp = req.send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;

        // 401 — separate path so retry logic above can detect.
        if status.as_u16() == 401 {
            return Err(Error::Api {
                code: "401".to_string(),
                message: "Unauthorized".to_string(),
                status: 401,
            });
        }

        if status.is_success() {
            return decode_envelope(&bytes, status.as_u16());
        }

        if let Ok(env) = serde_json::from_slice::<ApiResponse<serde_json::Value>>(&bytes) {
            let code = env
                .error_code
                .clone()
                .unwrap_or_else(|| status.as_u16().to_string());
            let message = env
                .error_message
                .or(env.status_message)
                .unwrap_or_else(|| status.to_string());
            if ErrorCode::from_code(&code).is_auth() {
                return Err(Error::Auth(format!("{code}: {message}")));
            }
            return Err(Error::Api {
                code,
                message,
                status: status.as_u16(),
            });
        }

        Err(Error::Api {
            code: status.as_u16().to_string(),
            message: status.to_string(),
            status: status.as_u16(),
        })
    }
}

#[async_trait::async_trait]
impl TokenTransport for Transport {
    async fn execute_raw<P, R>(
        &self,
        product: Product,
        method: reqwest::Method,
        path: &str,
        body: Option<&P>,
    ) -> Result<R, Error>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        let url = self.url_for(product, path);
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("application/json"),
        );
        // Token grant needs the app credentials in the body, not the
        // `Authorization` header. We do not attach `X-APP-Key` here.

        let mut req = self.inner.http.request(method, &url).headers(headers);
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            return Err(Error::Api {
                code: status.as_u16().to_string(),
                message: String::from_utf8_lossy(&bytes).into_owned(),
                status: status.as_u16(),
            });
        }
        decode_envelope(&bytes, status.as_u16())
    }
}

/// Internal response envelope. bKash success bodies set `statusCode`; failure
/// bodies set `errorCode`. Either is optional in this deserialiser to
/// tolerate the refund shape.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    #[serde(default, rename = "statusCode")]
    status_code: Option<String>,
    #[serde(default, rename = "statusMessage")]
    status_message: Option<String>,
    #[serde(default, rename = "errorCode")]
    error_code: Option<String>,
    #[serde(default, rename = "errorMessage")]
    error_message: Option<String>,
    #[serde(flatten)]
    data: T,
}

fn decode_envelope<R: DeserializeOwned>(bytes: &[u8], http_status: u16) -> Result<R, Error> {
    if bytes.is_empty() {
        return Err(Error::Api {
            code: "empty".into(),
            message: "empty response body".into(),
            status: http_status,
        });
    }
    let env: ApiResponse<serde_json::Value> =
        serde_json::from_slice(bytes).map_err(Error::Decode)?;
    if let Some(err_code) = env.error_code.as_deref() {
        let message = env
            .error_message
            .clone()
            .or(env.status_message.clone())
            .unwrap_or_default();
        if ErrorCode::from_code(err_code).is_auth() {
            return Err(Error::Auth(format!("{err_code}: {message}")));
        }
        return Err(Error::Api {
            code: err_code.to_string(),
            message,
            status: http_status,
        });
    }
    if env.status_code.as_deref() != Some("0000") {
        let code = env.status_code.clone().unwrap_or_else(|| "unknown".into());
        let message = env
            .status_message
            .clone()
            .unwrap_or_else(|| "no statusMessage".into());
        return Err(Error::Api {
            code,
            message,
            status: http_status,
        });
    }
    // The body has statusCode == 0000 — decode the flattened data.
    let value = env.data;
    serde_json::from_value::<R>(value).map_err(Error::Decode)
}

/// Compute exponential-backoff delay for attempt number (1-indexed).
fn backoff_for(attempt: u32) -> Duration {
    // 200ms, 400ms, 800ms, ...
    let exp = 2u64.saturating_pow(attempt.saturating_sub(1));
    Duration::from_millis(200u64 * exp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows() {
        assert_eq!(backoff_for(1), Duration::from_millis(200));
        assert_eq!(backoff_for(2), Duration::from_millis(400));
        assert_eq!(backoff_for(3), Duration::from_millis(800));
    }

    #[test]
    fn request_options_default() {
        let opts = RequestOptions::default();
        assert!(!opts.skip_retry);
        assert!(opts.timeout.is_none());
    }

    #[test]
    fn decode_envelope_succeeds_on_0000() {
        let body = r#"{"statusCode":"0000","statusMessage":"OK","foo":"bar"}"#;
        let v: serde_json::Value = decode_envelope(body.as_bytes(), 200).unwrap();
        assert_eq!(v["foo"], "bar");
    }

    #[test]
    fn decode_envelope_maps_error_code() {
        let body = r#"{"errorCode":"2001","errorMessage":"Invalid App Key"}"#;
        let err = decode_envelope::<serde_json::Value>(body.as_bytes(), 200).unwrap_err();
        match err {
            Error::Auth(msg) => assert!(msg.contains("2001")),
            other => panic!("expected Auth, got {other:?}"),
        }
    }

    #[test]
    fn decode_envelope_maps_non_0000_status() {
        let body = r#"{"statusCode":"9999","statusMessage":"weird"}"#;
        let err = decode_envelope::<serde_json::Value>(body.as_bytes(), 200).unwrap_err();
        assert!(matches!(err, Error::Api { ref code, .. } if code == "9999"));
    }
}
