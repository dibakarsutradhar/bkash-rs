//! # bkash-rs
//!
//! Idiomatic async-first Rust client for the bKash Payment Gateway API.
//!
//! `bkash-rs` provides a strongly-typed, ergonomic interface to the bKash
//! Payment Gateway. It supports tokenized checkout, classic checkout, payment
//! authentication & capture, subscriptions, and webhook verification.
//!
//! ## Quickstart
//!
//! ```no_run
//! use bkash_rs::prelude::*;
//! # async fn run() -> Result<(), bkash_rs::Error> {
//! let bkash = Bkash::builder()
//!     .environment(Environment::Sandbox)
//!     .app_key("your_app_key")
//!     .app_secret("your_app_secret")
//!     .username("your_username")
//!     .password("your_password")
//!     .build()?;
//! let bkash = Bkash::new(bkash).await?;
//! # let _ = bkash;
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! - `rustls-tls` (default): Use `rustls` for TLS.
//! - `native-tls`: Use the platform's native TLS implementation.
//! - `tokenized-checkout` (default): Enable tokenized checkout endpoints.
//! - `checkout` (default): Enable classic checkout endpoints.
//! - `auth-capture`: Enable authorization & capture endpoints.
//! - `subscriptions`: Enable subscription endpoints.
//! - `webhooks`: Enable webhook signature verification.

#![deny(missing_docs)]
#![warn(rust_2018_idioms)]

#[cfg(feature = "auth-capture")]
pub mod auth_capture;
#[cfg(feature = "checkout")]
pub mod checkout;
pub mod client;
pub mod config;
pub mod error;
pub mod models;
pub mod prelude;
#[cfg(feature = "subscriptions")]
pub mod subscriptions;
pub mod token;
#[cfg(feature = "tokenized-checkout")]
pub mod tokenized;
pub mod transport;
#[cfg(feature = "webhooks")]
pub mod webhooks;

pub use crate::client::Bkash;
pub use crate::config::{Config, ConfigBuilder, Environment, Product};
pub use crate::error::{Error, ErrorCode};
