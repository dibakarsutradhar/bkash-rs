//! Top-level entry point for the **live** bKash sandbox smoke tests.
//!
//! These tests hit the real bKash sandbox API. They are **not** run in CI
//! (the default `cargo test` invocation skips them). They are also
//! skipped at runtime if the required `BKASH_SANDBOX_*` environment
//! variables are not set.
//!
//! Run with:
//!
//! ```bash
//! cargo test --all-features --test live -- --ignored --nocapture
//! ```
//!
//! See `CONTRIBUTING.md` for the full instructions and the list of
//! required env vars.

#![cfg(feature = "tokenized-checkout")]

#[path = "live/sandbox.rs"]
mod sandbox;
