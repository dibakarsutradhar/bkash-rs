//! Token grant / refresh request and response models.

use serde::{Deserialize, Serialize};

/// Request body for the token grant endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantTokenRequest {
    /// Always `"authorization_code"` for bKash.
    #[serde(rename = "grant_type")]
    pub grant_type: String,
    /// bKash `app_key`.
    pub app_key: String,
    /// bKash `app_secret`.
    pub app_secret: String,
}

impl GrantTokenRequest {
    /// Construct a grant-token request with the standard bKash grant type.
    #[must_use]
    pub fn new(app_key: impl Into<String>, app_secret: impl Into<String>) -> Self {
        Self {
            grant_type: "client_credentials".to_string(),
            app_key: app_key.into(),
            app_secret: app_secret.into(),
        }
    }
}

/// Request body for the token refresh endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    /// Always `"refresh_token"`.
    pub grant_type: String,
    /// bKash `app_key`.
    pub app_key: String,
    /// bKash `app_secret`.
    pub app_secret: String,
    /// The previously issued `refresh_token`.
    pub refresh_token: String,
}

impl RefreshTokenRequest {
    /// Construct a refresh-token request.
    #[must_use]
    pub fn new(
        app_key: impl Into<String>,
        app_secret: impl Into<String>,
        refresh_token: impl Into<String>,
    ) -> Self {
        Self {
            grant_type: "refresh_token".to_string(),
            app_key: app_key.into(),
            app_secret: app_secret.into(),
            refresh_token: refresh_token.into(),
        }
    }
}

/// Response from the token grant / refresh endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// Short-lived bearer token (valid ~1 hour).
    #[serde(default)]
    pub id_token: String,
    /// Long-lived refresh token (valid ~28 days).
    #[serde(default)]
    pub refresh_token: String,
    /// Token lifetime in seconds (typically `3600`).
    #[serde(default)]
    pub expires_in: u64,
    /// Token type, usually `"Bearer"`.
    #[serde(default)]
    pub token_type: String,
}

impl TokenResponse {
    /// Returns `true` if this response carries a usable `id_token`.
    #[must_use]
    pub fn has_token(&self) -> bool {
        !self.id_token.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_token_request_serialises() {
        let r = GrantTokenRequest::new("k", "s");
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("\"grant_type\":\"client_credentials\""));
        assert!(j.contains("\"app_key\":\"k\""));
        assert!(j.contains("\"app_secret\":\"s\""));
    }

    #[test]
    fn refresh_token_request_serialises() {
        let r = RefreshTokenRequest::new("k", "s", "rt");
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("\"grant_type\":\"refresh_token\""));
        assert!(j.contains("\"refresh_token\":\"rt\""));
    }

    #[test]
    fn token_response_parses_minimal_body() {
        let body =
            r#"{"id_token":"abc","refresh_token":"xyz","expires_in":3600,"token_type":"Bearer"}"#;
        let r: TokenResponse = serde_json::from_str(body).unwrap();
        assert_eq!(r.id_token, "abc");
        assert_eq!(r.refresh_token, "xyz");
        assert_eq!(r.expires_in, 3600);
        assert!(r.has_token());
    }

    #[test]
    fn token_response_handles_missing_fields() {
        let r: TokenResponse = serde_json::from_str("{}").unwrap();
        assert!(!r.has_token());
    }
}
