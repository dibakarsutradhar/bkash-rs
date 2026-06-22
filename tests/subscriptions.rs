//! Wiremock-backed integration tests for the Subscriptions product.
//!
//! Each test mounts a single endpoint, drives a `Bkash::subscriptions()`
//! call, and asserts on the parsed response. The grant-token endpoint is
//! shared across tests via a helper. The exit gate covers the full
//! agreement lifecycle: create → execute → query → cancel.
//!
//! These tests only compile when the `subscriptions` feature is enabled.

#![cfg(feature = "subscriptions")]

use bkash_rs::config::{Config, Environment};
use bkash_rs::models::common::{Currency, Money};
use bkash_rs::models::subscriptions::{
    CancelAgreementRequest, CreateAgreementRequest, ExecuteAgreementRequest, QueryAgreementRequest,
};
use bkash_rs::prelude::*;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn base_test_config(base_url: &str) -> Config {
    Config::builder()
        .environment(Environment::Sandbox)
        .app_key("test-app-key")
        .app_secret("test-app-secret")
        .username("test-user")
        .password("test-pass")
        .with_base_url(base_url.to_string())
        .max_retries(0)
        .build()
        .unwrap()
}

async fn bkash_for(server: &MockServer) -> Bkash {
    Bkash::new(base_test_config(&server.uri())).await.unwrap()
}

/// Mount a `POST /tokenized/checkout/token/grant` mock that returns a
/// fixed id_token. Used to satisfy the auto-grant flow inside the
/// transport. (Subscriptions live on the tokenized subdomain.)
async fn mount_grant(server: &MockServer, id_token: &str) {
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/token/grant"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "id_token": id_token,
            "refresh_token": "refresh-xyz",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .expect(1..)
        .mount(server)
        .await;
}

// ============================================================
// Agreement lifecycle
// ============================================================

#[tokio::test]
async fn create_subscription_returns_payment_id() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/create"))
        .and(header("Authorization", "Bearer id-abc"))
        .and(header("X-APP-Key", "test-app-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "bkashURL": "https://example.test/bkash",
            "callbackURL": "https://merchant.test/cb",
            "agreementCreateTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "currency": "BDT",
            "intent": "sale",
            "merchantInvoiceNumber": ""
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let req = CreateAgreementRequest::new(
        "cust-1",
        "https://merchant.test/cb",
        Money::bdt("100.00"),
        Currency::Bdt,
    )
    .with_merchant_invoice_number("INV-SUB-1");
    let resp = bkash
        .subscriptions()
        .create_subscription(req)
        .await
        .unwrap();
    assert_eq!(resp.payment_id, "TR0001");
    assert_eq!(resp.bkash_url, "https://example.test/bkash");
    assert_eq!(resp.payer_reference, "cust-1");
}

#[tokio::test]
async fn execute_subscription_returns_agreement_id() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/execute"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "agreementID": "AG0001",
            "agreementExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00",
            "agreementStatus": "Completed",
            "customerMsisdn": "01700000000",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "merchantInvoiceNumber": ""
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash
        .subscriptions()
        .execute_subscription("TR0001")
        .await
        .unwrap();
    assert_eq!(resp.agreement_id, "AG0001");
    assert_eq!(resp.payment_id, "TR0001");
    assert_eq!(resp.agreement_status, "Completed");
}

#[tokio::test]
async fn query_subscription_returns_status() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/agreement/status"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "agreementID": "AG0001",
            "agreementStatus": "Completed",
            "payerReference": "cust-1",
            "customerMsisdn": "01700000000",
            "callbackURL": "https://merchant.test/cb",
            "amount": "100.00",
            "currency": "BDT",
            "intent": "sale",
            "merchantInvoiceNumber": "INV-SUB-1",
            "orgShortCode": "0123",
            "agreementCreateTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "agreementExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash
        .subscriptions()
        .query_subscription("AG0001")
        .await
        .unwrap();
    assert_eq!(resp.agreement_id, "AG0001");
    assert_eq!(resp.agreement_status, "Completed");
    assert_eq!(resp.amount.as_str(), "100.00");
    assert_eq!(resp.merchant_invoice_number, "INV-SUB-1");
}

#[tokio::test]
async fn cancel_subscription_returns_cancelled_status() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/agreement/cancel"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "agreementID": "AG0001",
            "agreementStatus": "Cancelled"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash
        .subscriptions()
        .cancel_subscription("AG0001")
        .await
        .unwrap();
    assert_eq!(resp.agreement_id, "AG0001");
    assert_eq!(resp.agreement_status, "Cancelled");

    // Sanity-check that the request type serialises correctly.
    let req = CancelAgreementRequest::new("AG0001");
    let v: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(v["agreementID"], "AG0001");
}

// ============================================================
// Request shape regression tests
// ============================================================

#[tokio::test]
async fn execute_subscription_request_serialises_with_payment_id() {
    let req = ExecuteAgreementRequest::new("TR0001");
    let v: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(v["paymentID"], "TR0001");
}

#[tokio::test]
async fn query_subscription_request_serialises() {
    let req = QueryAgreementRequest::new("AG0001");
    let v: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(v["agreementID"], "AG0001");
}

// ============================================================
// Error envelope mapping
// ============================================================

#[tokio::test]
async fn create_subscription_error_maps_to_api_error() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errorCode": "2036",
            "errorMessage": "Subscription: invalid payer reference"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let req = CreateAgreementRequest::new(
        "cust-bad",
        "https://merchant.test/cb",
        Money::bdt("100.00"),
        Currency::Bdt,
    );
    let err = bkash
        .subscriptions()
        .create_subscription(req)
        .await
        .unwrap_err();
    match err {
        Error::Api { code, message, .. } => {
            assert_eq!(code, "2036");
            assert!(message.contains("payer"));
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
