//! High-level client for interacting with the bKash Payment Gateway.
//!
//! [`Bkash`] is a cheaply-cloneable handle (`Arc<Inner>`) that owns a
//! [`Transport`] and a [`Config`]. Product accessor methods are added in
//! their respective phases (3–6).

use std::sync::Arc;

use crate::config::{Config, ConfigBuilder};
use crate::error::Error;
use crate::token::TokenCache;
use crate::transport::Transport;

/// Top-level bKash client. Clone-cheap.
pub struct Bkash {
    inner: Arc<Inner>,
}

struct Inner {
    config: Config,
    transport: Transport,
}

impl Clone for Bkash {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl std::fmt::Debug for Bkash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bkash")
            .field("config", &self.inner.config)
            .field("transport", &self.inner.transport)
            .finish_non_exhaustive()
    }
}

impl Bkash {
    /// Build a new client from a validated [`Config`].
    pub async fn new(config: Config) -> Result<Self, Error> {
        config.validate()?;
        let transport = Transport::new(config.clone()).await?;
        Ok(Self {
            inner: Arc::new(Inner { config, transport }),
        })
    }

    /// Start a builder flow. Equivalent to `Config::builder()` — provided for
    /// discoverability.
    #[must_use]
    pub fn builder() -> ConfigBuilder {
        Config::builder()
    }

    /// Access the configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// Access the transport (used by product accessor methods in later
    /// phases).
    #[must_use]
    pub fn transport(&self) -> &Transport {
        &self.inner.transport
    }

    /// Access the shared token cache.
    #[must_use]
    pub fn token_cache(&self) -> &TokenCache {
        self.inner.transport.token_cache()
    }

    /// Access the Tokenized Checkout product accessor.
    #[cfg(feature = "tokenized-checkout")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokenized-checkout")))]
    #[must_use]
    pub fn tokenized(&self) -> crate::tokenized::TokenizedCheckoutClient<'_> {
        crate::tokenized::TokenizedCheckoutClient::new(self)
    }

    /// Access the URL-based Checkout product accessor.
    #[cfg(feature = "checkout")]
    #[cfg_attr(docsrs, doc(cfg(feature = "checkout")))]
    #[must_use]
    pub fn checkout(&self) -> crate::checkout::CheckoutClient<'_> {
        crate::checkout::CheckoutClient::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Environment;

    #[tokio::test]
    async fn new_rejects_missing_credentials() {
        let cfg = Config::builder().environment(Environment::Sandbox).build();
        assert!(cfg.is_err());
    }

    #[tokio::test]
    async fn new_constructs_valid_client() {
        let cfg = Config::builder()
            .environment(Environment::Sandbox)
            .app_key("k")
            .app_secret("s")
            .username("u")
            .password("p")
            .build()
            .unwrap();
        let bkash = Bkash::new(cfg).await.unwrap();
        assert_eq!(bkash.config().environment, Environment::Sandbox);
    }

    #[tokio::test]
    async fn client_is_clone() {
        let cfg = Config::builder()
            .environment(Environment::Sandbox)
            .app_key("k")
            .app_secret("s")
            .username("u")
            .password("p")
            .build()
            .unwrap();
        let bkash = Bkash::new(cfg).await.unwrap();
        let _b2 = bkash.clone();
    }

    #[test]
    fn builder_helper_returns_config_builder() {
        let _b: ConfigBuilder = Bkash::builder();
    }
}
