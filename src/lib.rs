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
//! ```ignore
//! use bkash_rs::prelude::*;
//!
//! # async fn run() -> Result<(), bkash_rs::Error> {
//! let client = Client::builder()
//!     .environment(Environment::Sandbox)
//!     .credentials(/* ... */)
//!     .build()
//!     .await?;
//! # let _ = client;
//! # Ok(())
//! # }
//! ```
//!
//! The real quickstart will be wired up in Phase 3 once `Client::builder`,
//! `Environment`, and the credential types land.
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
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod client;
pub mod config;
pub mod error;
pub mod models;
pub mod prelude;
pub mod token;
pub mod transport;
pub mod webhooks;
