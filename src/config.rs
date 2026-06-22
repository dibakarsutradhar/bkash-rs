//! Client configuration: environment, credentials, timeouts, and policy knobs.

use std::fmt;
use std::time::Duration;

/// Identifies a bKash API product. Different products use different
/// subdomains and token-grant paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Product {
    /// Tokenized Checkout (`tokenized.sandbox.bka.sh`).
    Tokenized,
    /// Classic / URL-based Checkout (`checkout.sandbox.bka.sh`).
    Checkout,
    /// Authorization & Capture (lives on the `checkout` subdomain).
    AuthCapture,
    /// Subscriptions (lives on the `tokenized` subdomain).
    Subscriptions,
}

impl Product {
    /// Service subdomain for this product.
    #[must_use]
    pub fn service_subdomain(&self) -> &'static str {
        match self {
            Self::Tokenized | Self::Subscriptions => "tokenized",
            Self::Checkout | Self::AuthCapture => "checkout",
        }
    }

    /// Path component for the token-grant endpoint.
    #[must_use]
    pub fn token_path(&self) -> &'static str {
        match self {
            Self::Checkout | Self::AuthCapture => "checkout/token/grant",
            Self::Tokenized | Self::Subscriptions => "tokenized/checkout/token/grant",
        }
    }

    /// Path component for the token-refresh endpoint.
    #[must_use]
    pub fn token_refresh_path(&self) -> &'static str {
        // All products share the same refresh endpoint, hosted on the
        // tokenized subdomain.
        "tokenized/checkout/token/refresh"
    }
}

/// bKash API environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Environment {
    /// bKash sandbox (`*.sandbox.bka.sh`).
    Sandbox,
    /// bKash production (`*.pay.bka.sh`).
    Production,
}

impl Environment {
    /// Construct a sandbox environment.
    #[must_use]
    pub fn sandbox() -> Self {
        Self::Sandbox
    }

    /// Construct a production environment.
    #[must_use]
    pub fn production() -> Self {
        Self::Production
    }

    /// Returns the full base URL (with `/v1.2.0-beta/` segment) for the given
    /// product on this environment.
    #[must_use]
    pub fn base_url(&self, product: Product) -> String {
        let host = match self {
            Self::Sandbox => "sandbox.bka.sh",
            Self::Production => "pay.bka.sh",
        };
        format!(
            "https://{}.{}/v1.2.0-beta/",
            product.service_subdomain(),
            host
        )
    }
}

/// Client configuration.
#[derive(Clone)]
pub struct Config {
    /// API environment.
    pub environment: Environment,
    /// bKash `app_key`.
    pub app_key: String,
    /// bKash `app_secret`.
    pub app_secret: String,
    /// bKash username.
    pub username: String,
    /// bKash password.
    pub password: String,
    /// Per-request HTTP timeout.
    pub timeout: Duration,
    /// Maximum number of transient retries.
    pub max_retries: u32,
    /// Optional pre-built HTTP client (for connection pooling, custom TLS,
    /// proxies). When `None`, a default client is created.
    pub http_client: Option<reqwest::Client>,
    /// Optional base URL override (for tests / wiremock).
    pub base_url: Option<String>,
}

impl Config {
    /// Construct a new [`ConfigBuilder`].
    #[must_use]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Construct a sandbox config builder pre-populated for the sandbox
    /// environment. Credentials must still be supplied.
    #[must_use]
    pub fn sandbox() -> ConfigBuilder {
        Self::builder().environment(Environment::Sandbox)
    }

    /// Construct a production config builder pre-populated for the
    /// production environment. Credentials must still be supplied.
    #[must_use]
    pub fn production() -> ConfigBuilder {
        Self::builder().environment(Environment::Production)
    }

    /// Validate the configuration. Returns `Err` if any required field is
    /// missing or invalid.
    pub fn validate(&self) -> Result<(), crate::Error> {
        if self.app_key.trim().is_empty() {
            return Err(crate::Error::Config("app_key is required".into()));
        }
        if self.app_secret.trim().is_empty() {
            return Err(crate::Error::Config("app_secret is required".into()));
        }
        if self.username.trim().is_empty() {
            return Err(crate::Error::Config("username is required".into()));
        }
        if self.password.trim().is_empty() {
            return Err(crate::Error::Config("password is required".into()));
        }
        if self.timeout.is_zero() {
            return Err(crate::Error::Config("timeout must be non-zero".into()));
        }
        Ok(())
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("environment", &self.environment)
            .field("app_key", &"***redacted***")
            .field("app_secret", &"***redacted***")
            .field("username", &"***redacted***")
            .field("password", &"***redacted***")
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field(
                "http_client",
                &self.http_client.as_ref().map(|_| "<client>"),
            )
            .field("base_url", &self.base_url)
            .finish()
    }
}

/// Builder for [`Config`].
#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    environment: Option<Environment>,
    app_key: Option<String>,
    app_secret: Option<String>,
    username: Option<String>,
    password: Option<String>,
    timeout: Duration,
    max_retries: u32,
    http_client: Option<reqwest::Client>,
    base_url: Option<String>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigBuilder {
    /// Create a new builder with default timeouts / retry policy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            environment: None,
            app_key: None,
            app_secret: None,
            username: None,
            password: None,
            timeout: Duration::from_secs(30),
            max_retries: 2,
            http_client: None,
            base_url: None,
        }
    }

    /// Set the environment.
    #[must_use]
    pub fn environment(mut self, env: Environment) -> Self {
        self.environment = Some(env);
        self
    }

    /// Set the `app_key`.
    #[must_use]
    pub fn app_key(mut self, key: impl Into<String>) -> Self {
        self.app_key = Some(key.into());
        self
    }

    /// Set the `app_secret`.
    #[must_use]
    pub fn app_secret(mut self, secret: impl Into<String>) -> Self {
        self.app_secret = Some(secret.into());
        self
    }

    /// Set the username.
    #[must_use]
    pub fn username(mut self, u: impl Into<String>) -> Self {
        self.username = Some(u.into());
        self
    }

    /// Set the password.
    #[must_use]
    pub fn password(mut self, p: impl Into<String>) -> Self {
        self.password = Some(p.into());
        self
    }

    /// Set the per-request timeout.
    #[must_use]
    pub fn timeout(mut self, t: Duration) -> Self {
        self.timeout = t;
        self
    }

    /// Set the maximum number of transient retries.
    #[must_use]
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// Provide a pre-built HTTP client.
    #[must_use]
    pub fn http_client(mut self, c: reqwest::Client) -> Self {
        self.http_client = Some(c);
        self
    }

    /// Override the base URL (e.g. for tests pointing at wiremock).
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Build the [`Config`].
    ///
    /// Returns [`crate::Error::Config`] if any required field is missing.
    pub fn build(self) -> Result<Config, crate::Error> {
        let environment = self
            .environment
            .ok_or_else(|| crate::Error::Config("environment is required".into()))?;
        let app_key = self
            .app_key
            .ok_or_else(|| crate::Error::Config("app_key is required".into()))?;
        let app_secret = self
            .app_secret
            .ok_or_else(|| crate::Error::Config("app_secret is required".into()))?;
        let username = self
            .username
            .ok_or_else(|| crate::Error::Config("username is required".into()))?;
        let password = self
            .password
            .ok_or_else(|| crate::Error::Config("password is required".into()))?;
        let cfg = Config {
            environment,
            app_key,
            app_secret,
            username,
            password,
            timeout: self.timeout,
            max_retries: self.max_retries,
            http_client: self.http_client,
            base_url: self.base_url,
        };
        cfg.validate()?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Config {
        Config {
            environment: Environment::Sandbox,
            app_key: "secret-app-key".into(),
            app_secret: "super-secret".into(),
            username: "secret-user".into(),
            password: "secret-pass".into(),
            timeout: Duration::from_secs(10),
            max_retries: 1,
            http_client: None,
            base_url: None,
        }
    }

    #[test]
    fn environment_base_url_tokenized_sandbox() {
        let url = Environment::Sandbox.base_url(Product::Tokenized);
        assert_eq!(url, "https://tokenized.sandbox.bka.sh/v1.2.0-beta/");
    }

    #[test]
    fn environment_base_url_tokenized_production() {
        let url = Environment::Production.base_url(Product::Tokenized);
        assert_eq!(url, "https://tokenized.pay.bka.sh/v1.2.0-beta/");
    }

    #[test]
    fn environment_base_url_checkout_sandbox() {
        let url = Environment::Sandbox.base_url(Product::Checkout);
        assert_eq!(url, "https://checkout.sandbox.bka.sh/v1.2.0-beta/");
    }

    #[test]
    fn environment_base_url_checkout_production() {
        let url = Environment::Production.base_url(Product::Checkout);
        assert_eq!(url, "https://checkout.pay.bka.sh/v1.2.0-beta/");
    }

    #[test]
    fn environment_base_url_auth_capture() {
        assert!(Environment::Sandbox
            .base_url(Product::AuthCapture)
            .starts_with("https://checkout.sandbox.bka.sh/"));
    }

    #[test]
    fn environment_base_url_subscriptions() {
        assert!(Environment::Sandbox
            .base_url(Product::Subscriptions)
            .starts_with("https://tokenized.sandbox.bka.sh/"));
    }

    #[test]
    fn config_debug_redacts_credentials() {
        let cfg = sample_config();
        let s = format!("{cfg:?}");
        assert!(!s.contains("secret-app-key"), "app_key leaked: {s}");
        assert!(!s.contains("super-secret"), "app_secret leaked: {s}");
        assert!(!s.contains("secret-user"), "username leaked: {s}");
        assert!(!s.contains("secret-pass"), "password leaked: {s}");
        assert!(
            s.contains("***redacted***"),
            "expected redaction marker: {s}"
        );
    }

    #[test]
    fn builder_validates_required_fields() {
        let r = Config::builder().build();
        assert!(r.is_err());
    }

    #[test]
    fn builder_validates_blank_credentials() {
        let r = Config::builder()
            .environment(Environment::Sandbox)
            .app_key("   ")
            .app_secret("x")
            .username("x")
            .password("x")
            .build();
        assert!(r.is_err());
    }

    #[test]
    fn builder_produces_valid_config() {
        let cfg = Config::builder()
            .environment(Environment::Sandbox)
            .app_key("k")
            .app_secret("s")
            .username("u")
            .password("p")
            .build()
            .unwrap();
        assert_eq!(cfg.environment, Environment::Sandbox);
        assert_eq!(cfg.timeout, Duration::from_secs(30));
        assert_eq!(cfg.max_retries, 2);
    }

    #[test]
    fn sandbox_and_production_helpers() {
        let cfg = Config::sandbox()
            .app_key("k")
            .app_secret("s")
            .username("u")
            .password("p")
            .build()
            .unwrap();
        assert_eq!(cfg.environment, Environment::Sandbox);
        let cfg = Config::production()
            .app_key("k")
            .app_secret("s")
            .username("u")
            .password("p")
            .build()
            .unwrap();
        assert_eq!(cfg.environment, Environment::Production);
    }

    #[test]
    fn with_base_url_overrides() {
        let cfg = Config::sandbox()
            .app_key("k")
            .app_secret("s")
            .username("u")
            .password("p")
            .with_base_url("https://example.test/")
            .build()
            .unwrap();
        assert_eq!(cfg.base_url.as_deref(), Some("https://example.test/"));
    }
}
