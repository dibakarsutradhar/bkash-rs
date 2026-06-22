//! Live bKash sandbox smoke tests.
//!
//! These tests exercise the **real** bKash sandbox API. They are:
//!
//! 1. Marked `#[ignore]` so that `cargo test` runs the offline wiremock
//!    suite by default and never hits the network.
//! 2. Gated on the four `BKASH_SANDBOX_*` environment variables described
//!    in `CONTRIBUTING.md`. If any are missing, each test logs and returns
//!    early (effectively a no-op) instead of failing.
//! 3. Skeletons — they verify the client can be constructed and the
//!    high-level flow runs to the first blocking call, but **do not**
//!    perform the wallet-side customer approval, capture, or refund
//!    completion (those require manual user interaction with the bKash
//!    wallet or a fully populated sandbox account).
//!
//! To run the full live suite (when you have real credentials and want to
//! execute the customer-approval step manually):
//!
//! ```bash
//! cargo test --all-features --test live -- --ignored --nocapture
//! ```
//!
//! See `CONTRIBUTING.md` for the full instructions.

#![cfg(feature = "tokenized-checkout")]

use bkash_rs::config::{Config, Environment};
use bkash_rs::models::common::{Currency, Money};
use bkash_rs::models::token::GrantTokenRequest;
use bkash_rs::models::tokenized::{CreateAgreementRequest, CreatePaymentRequest};
use bkash_rs::prelude::*;

/// Read the four required env vars. Returns `None` if any are unset; the
/// caller should `return` and treat the test as a no-op in that case.
fn sandbox_credentials() -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    let app_key = std::env::var("BKASH_SANDBOX_APP_KEY").ok()?;
    let app_secret = std::env::var("BKASH_SANDBOX_APP_SECRET").ok()?;
    let username = std::env::var("BKASH_SANDBOX_USERNAME").ok()?;
    let password = std::env::var("BKASH_SANDBOX_PASSWORD").ok()?;
    // We have to leak the strings to get `&'static str`, but these strings
    // are tiny and the program is short-lived; this is the simplest way to
    // satisfy the `Option<(&'static str, ...)>` signature.
    Some((
        Box::leak(app_key.into_boxed_str()),
        Box::leak(app_secret.into_boxed_str()),
        Box::leak(username.into_boxed_str()),
        Box::leak(password.into_boxed_str()),
    ))
}

async fn bkash_for_sandbox() -> Option<Bkash> {
    let (app_key, app_secret, username, password) = sandbox_credentials()?;
    let config = Config::builder()
        .environment(Environment::Sandbox)
        .app_key(app_key)
        .app_secret(app_secret)
        .username(username)
        .password(password)
        .build()
        .expect("invalid sandbox config");
    Some(
        Bkash::new(config)
            .await
            .expect("failed to build Bkash client"),
    )
}

#[tokio::test]
#[ignore = "requires BKASH_SANDBOX_* env vars; see CONTRIBUTING.md"]
async fn live_tokenized_grant_token() {
    let Some(bkash) = bkash_for_sandbox().await else {
        eprintln!("skipping live test: BKASH_SANDBOX_* env vars are not set");
        return;
    };
    let req = GrantTokenRequest::new(
        bkash.config().app_key.clone(),
        bkash.config().app_secret.clone(),
    );
    let resp = bkash
        .tokenized()
        .grant_token(req)
        .await
        .expect("tokenized grant_token failed against sandbox");
    assert!(
        !resp.id_token.is_empty(),
        "sandbox returned an empty id_token"
    );
    assert!(
        resp.expires_in > 0,
        "sandbox returned non-positive expires_in"
    );
}

#[tokio::test]
#[ignore = "requires BKASH_SANDBOX_* env vars; see CONTRIBUTING.md"]
async fn live_tokenized_create_agreement() {
    let Some(bkash) = bkash_for_sandbox().await else {
        eprintln!("skipping live test: BKASH_SANDBOX_* env vars are not set");
        return;
    };
    // Note: completing this flow requires the user to approve the
    // agreement in the bKash wallet. We only assert the create step.
    let req = CreateAgreementRequest::new(
        "live-test-cust",
        "https://example.com/cb",
        Money::bdt("100.00"),
        Currency::Bdt,
    );
    let resp = bkash
        .tokenized()
        .create_agreement(req)
        .await
        .expect("tokenized create_agreement failed against sandbox");
    assert!(!resp.payment_id.is_empty());
    assert!(!resp.bkash_url.is_empty());
}

#[tokio::test]
#[ignore = "requires BKASH_SANDBOX_* env vars; see CONTRIBUTING.md"]
async fn live_tokenized_create_payment() {
    let Some(bkash) = bkash_for_sandbox().await else {
        eprintln!("skipping live test: BKASH_SANDBOX_* env vars are not set");
        return;
    };
    // Note: completing this flow requires the user to approve the payment
    // in the bKash wallet. We only assert the create step against an
    // existing agreement ID (which the operator must provide via
    // BKASH_SANDBOX_AGREEMENT_ID for the full path to be useful).
    let req = CreatePaymentRequest::new(
        std::env::var("BKASH_SANDBOX_AGREEMENT_ID").unwrap_or_default(),
        "live-test-cust",
        "https://example.com/cb",
        Money::bdt("10.00"),
        Currency::Bdt,
    );
    let resp = bkash
        .tokenized()
        .create_payment(req)
        .await
        .expect("tokenized create_payment failed against sandbox");
    assert!(!resp.payment_id.is_empty());
    assert!(!resp.bkash_url.is_empty());
}

#[tokio::test]
#[ignore = "requires BKASH_SANDBOX_* env vars; see CONTRIBUTING.md"]
async fn live_checkout_grant_token() {
    let Some(bkash) = bkash_for_sandbox().await else {
        eprintln!("skipping live test: BKASH_SANDBOX_* env vars are not set");
        return;
    };
    let req = GrantTokenRequest::new(
        bkash.config().app_key.clone(),
        bkash.config().app_secret.clone(),
    );
    let resp = bkash
        .checkout()
        .grant_token(req)
        .await
        .expect("checkout grant_token failed against sandbox");
    assert!(!resp.id_token.is_empty());
}

#[tokio::test]
#[ignore = "requires BKASH_SANDBOX_* env vars; see CONTRIBUTING.md"]
async fn live_search_transaction_via_tokenized() {
    let Some(bkash) = bkash_for_sandbox().await else {
        eprintln!("skipping live test: BKASH_SANDBOX_* env vars are not set");
        return;
    };
    // Operator must supply a real sandbox trxID to actually exercise
    // search; otherwise the call will return an API error which we
    // tolerate (this is a smoke test of connectivity, not of business
    // logic).
    let trx_id = std::env::var("BKASH_SANDBOX_TRX_ID").unwrap_or_else(|_| "SANITIZED".to_string());
    let _ = bkash.tokenized().search_transaction(&trx_id).await;
}
