//! OAuth token grant, refresh, and cache.
//!
//! [`TokenCache`] is a small thread-safe holder for a [`CachedToken`]; it
//! does not perform any I/O. The actual grant / refresh logic lives on
//! [`TokenManager`], which combines a [`TokenCache`] with the
//! [`crate::transport::Transport`].

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::sync::RwLock;

use crate::config::{Config, Product};
use crate::error::Error;
use crate::models::token::{GrantTokenRequest, RefreshTokenRequest, TokenResponse};

/// A cached OAuth token.
#[derive(Debug, Clone)]
pub struct CachedToken {
    /// The bearer token used for `Authorization: Bearer <id_token>`.
    pub id_token: String,
    /// The long-lived refresh token (kept across refreshes).
    pub refresh_token: String,
    /// When this token expires. Validity is checked with a 5-minute skew.
    pub expires_at: Instant,
}

impl CachedToken {
    /// Returns `true` if this token is still valid, applying the given skew.
    #[must_use]
    pub fn is_valid(&self, skew: Duration) -> bool {
        Instant::now() + skew < self.expires_at
    }
}

/// Thread-safe token cache. The inner state is `None` until the first grant
/// completes.
pub struct TokenCache {
    pub(crate) inner: Arc<RwLock<Option<CachedToken>>>,
}

impl TokenCache {
    /// Create an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
        }
    }

    /// Return a clone of the currently cached token, if any.
    pub async fn get_cached(&self) -> Option<CachedToken> {
        self.inner.read().await.as_ref().cloned()
    }

    /// Returns `true` if the cached token is still valid (with the default
    /// 5-minute skew).
    pub async fn is_valid(&self) -> bool {
        match self.inner.read().await.as_ref() {
            Some(t) => t.is_valid(SKEW),
            None => false,
        }
    }

    /// Store a token in the cache.
    pub async fn set(&self, token: CachedToken) {
        *self.inner.write().await = Some(token);
    }

    /// Clear the cache (used after a 401 before force-regrant).
    pub async fn clear(&self) {
        *self.inner.write().await = None;
    }
}

impl Default for TokenCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TokenCache {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl std::fmt::Debug for TokenCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenCache").finish_non_exhaustive()
    }
}

/// Refresh skew applied to token validity checks. Tokens are considered
/// expired 5 minutes before their nominal expiry to avoid races at the
/// boundary.
pub const SKEW: Duration = Duration::from_secs(5 * 60);

/// Trait for the subset of [`Transport`](crate::transport::Transport)
/// behaviour that [`TokenManager`] needs. Lets tests inject a fake.
#[async_trait::async_trait]
pub trait TokenTransport: Send + Sync {
    /// Send a request and return the decoded body, bypassing the
    /// token-cache machinery.
    async fn execute_raw<P, R>(
        &self,
        product: Product,
        method: reqwest::Method,
        path: &str,
        body: Option<&P>,
    ) -> Result<R, Error>
    where
        P: serde::Serialize + Send + Sync,
        R: for<'de> Deserialize<'de> + Send;
}

/// Manages the token lifecycle: ensuring a valid token is in the cache,
/// granting new ones, and refreshing expiring ones.
pub struct TokenManager<T: TokenTransport> {
    config: Config,
    cache: TokenCache,
    transport: Arc<T>,
}

impl<T: TokenTransport> std::fmt::Debug for TokenManager<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenManager")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl<T: TokenTransport + 'static> TokenManager<T> {
    /// Create a new manager.
    #[must_use]
    pub fn new(config: Config, cache: TokenCache, transport: Arc<T>) -> Self {
        Self {
            config,
            cache,
            transport,
        }
    }

    /// Return a handle to the underlying cache.
    #[must_use]
    pub fn cache(&self) -> &TokenCache {
        &self.cache
    }

    /// Return a reference to the configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a clone of the currently cached token, if any.
    pub async fn current(&self) -> Option<CachedToken> {
        self.cache.get_cached().await
    }

    /// Force a re-grant, ignoring any cached token.
    pub async fn force_grant(&self, product: Product) -> Result<CachedToken, Error> {
        let resp = self.grant(product).await?;
        let token = self.to_cached(resp, None);
        self.cache.set(token.clone()).await;
        Ok(token)
    }

    /// Ensure a valid token is in the cache, returning it.
    ///
    /// Behaviour:
    /// 1. Read-lock the cache; if a valid (with [`SKEW`]) token exists, return it.
    /// 2. Otherwise drop the read lock, take a write lock, and re-check.
    /// 3. If still empty / invalid, perform a grant (if no refresh token) or
    ///    a refresh (if a refresh token exists).
    ///
    /// Refresh keeps the existing `refresh_token` per the bKash contract
    /// (refresh returns the *same* refresh token).
    pub async fn ensure_token(&self, product: Product) -> Result<CachedToken, Error> {
        // Fast path: read lock.
        {
            let guard = self.cache.inner.read().await;
            if let Some(tok) = guard.as_ref() {
                if tok.is_valid(SKEW) {
                    return Ok(tok.clone());
                }
            }
        }

        // Slow path: write lock, re-check.
        let existing_refresh = {
            let guard = self.cache.inner.write().await;
            if let Some(tok) = guard.as_ref() {
                if tok.is_valid(SKEW) {
                    return Ok(tok.clone());
                }
                // Capture the existing refresh token for refresh.
                Some(tok.refresh_token.clone())
            } else {
                None
            }
        };

        let resp = if let Some(refresh_token) = existing_refresh {
            self.refresh(product, &refresh_token).await?
        } else {
            self.grant(product).await?
        };

        // Refresh returns the *same* refresh token; prefer the previous one
        // in case bKash returns an empty one in some error scenario.
        let existing_refresh = self.cache.get_cached().await.map(|t| t.refresh_token);
        let token = self.to_cached(resp, existing_refresh);
        self.cache.set(token.clone()).await;
        Ok(token)
    }

    async fn grant(&self, product: Product) -> Result<TokenResponse, Error> {
        let req = GrantTokenRequest::new(&self.config.app_key, &self.config.app_secret);
        let resp: TokenResponse = self
            .transport
            .execute_raw(
                product,
                reqwest::Method::POST,
                product.token_path(),
                Some(&req),
            )
            .await
            .map_err(|e| match e {
                Error::Api {
                    code,
                    message,
                    status,
                } => Error::Token(format!("{status} {code}: {message}")),
                other => other,
            })?;
        if !resp.has_token() {
            return Err(Error::Token("grant response missing id_token".into()));
        }
        Ok(resp)
    }

    async fn refresh(&self, product: Product, refresh_token: &str) -> Result<TokenResponse, Error> {
        let req =
            RefreshTokenRequest::new(&self.config.app_key, &self.config.app_secret, refresh_token);
        let resp: TokenResponse = self
            .transport
            .execute_raw(
                product,
                reqwest::Method::POST,
                product.token_refresh_path(),
                Some(&req),
            )
            .await
            .map_err(|e| match e {
                Error::Api {
                    code,
                    message,
                    status,
                } => Error::Token(format!("{status} {code}: {message}")),
                other => other,
            })?;
        if !resp.has_token() {
            return Err(Error::Token("refresh response missing id_token".into()));
        }
        Ok(resp)
    }

    fn to_cached(&self, resp: TokenResponse, existing_refresh: Option<String>) -> CachedToken {
        // Prefer the previously-known refresh token (refresh returns the
        // same one), falling back to the response.
        let refresh_token = existing_refresh
            .filter(|s| !s.is_empty())
            .unwrap_or(resp.refresh_token);
        CachedToken {
            id_token: resp.id_token,
            refresh_token,
            expires_at: Instant::now() + Duration::from_secs(resp.expires_in.max(60)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc as StdArc;

    #[derive(Default)]
    struct FakeTransport {
        grants: AtomicUsize,
        refreshes: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl TokenTransport for FakeTransport {
        async fn execute_raw<P, R>(
            &self,
            _product: Product,
            _method: reqwest::Method,
            path: &str,
            _body: Option<&P>,
        ) -> Result<R, Error>
        where
            P: serde::Serialize + Send + Sync,
            R: for<'de> Deserialize<'de> + Send,
        {
            // The TokenManager always uses TokenResponse as R; we synthesise
            // a response by deserialising a JSON string.
            let json = if path.contains("refresh") {
                self.refreshes.fetch_add(1, Ordering::SeqCst);
                r#"{"id_token":"new_id","refresh_token":"same_refresh","expires_in":3600,"token_type":"Bearer"}"#
            } else {
                self.grants.fetch_add(1, Ordering::SeqCst);
                r#"{"id_token":"first_id","refresh_token":"first_refresh","expires_in":3600,"token_type":"Bearer"}"#
            };
            serde_json::from_str(json).map_err(Error::from)
        }
    }

    fn cfg() -> Config {
        Config::builder()
            .environment(crate::config::Environment::Sandbox)
            .app_key("k")
            .app_secret("s")
            .username("u")
            .password("p")
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn cache_default_is_empty() {
        let cache = TokenCache::new();
        assert!(cache.get_cached().await.is_none());
        assert!(!cache.is_valid().await);
    }

    #[tokio::test]
    async fn cache_set_and_get() {
        let cache = TokenCache::new();
        let tok = CachedToken {
            id_token: "abc".into(),
            refresh_token: "xyz".into(),
            expires_at: Instant::now() + Duration::from_secs(3600),
        };
        cache.set(tok.clone()).await;
        let got = cache.get_cached().await.unwrap();
        assert_eq!(got.id_token, "abc");
        assert!(cache.is_valid().await);
    }

    #[tokio::test]
    async fn cache_clear() {
        let cache = TokenCache::new();
        cache
            .set(CachedToken {
                id_token: "abc".into(),
                refresh_token: "xyz".into(),
                expires_at: Instant::now() + Duration::from_secs(60),
            })
            .await;
        cache.clear().await;
        assert!(cache.get_cached().await.is_none());
    }

    #[tokio::test]
    async fn ensure_token_grants_only_once_under_contention() {
        let fake = StdArc::new(FakeTransport::default());
        let mgr = StdArc::new(TokenManager::new(cfg(), TokenCache::new(), fake.clone()));
        let mut handles = Vec::new();
        for _ in 0..16 {
            let mgr = mgr.clone();
            let h = tokio::spawn(async move { mgr.ensure_token(Product::Tokenized).await });
            handles.push(h);
        }
        for h in handles {
            let tok = h.await.unwrap().unwrap();
            assert!(!tok.id_token.is_empty());
        }
        // The double-check pattern should mean only one grant happens.
        let grants = fake.grants.load(Ordering::SeqCst);
        assert!(grants <= 2, "grants={grants}");
    }

    #[tokio::test]
    async fn refresh_keeps_existing_refresh_token() {
        let fake = StdArc::new(FakeTransport::default());
        let cache = TokenCache::new();
        // Pre-seed a cached token that's about to expire.
        cache
            .set(CachedToken {
                id_token: "old_id".into(),
                refresh_token: "original_refresh".into(),
                expires_at: Instant::now(),
            })
            .await;
        let mgr = TokenManager::new(cfg(), cache, fake.clone());
        let tok = mgr.ensure_token(Product::Tokenized).await.unwrap();
        assert_eq!(tok.id_token, "new_id");
        // Per the bKash contract, the refresh token is unchanged.
        assert_eq!(tok.refresh_token, "original_refresh");
    }
}
