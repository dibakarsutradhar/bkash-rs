//! Wiremock-backed integration tests for the URL-based Checkout product.
//!
//! Each test mounts a single endpoint, drives a `Bkash::checkout()` call,
//! and asserts on the parsed response. The grant-token endpoint is mounted
//! via a helper and uses `POST /checkout/token/grant` (NOT the tokenized
//! variant) — that is the key difference from Phase 3.
//!
//! These tests only compile when the `checkout` feature is enabled.

#![cfg(feature = "checkout")]

use bkash_rs::config::{Config, Environment};
use bkash_rs::models::checkout::{
    CreatePaymentRequest, ExecutePaymentRequest, QueryPaymentRequest, RefundRequest,
    RefundStatusRequest, SearchTransactionRequest,
};
use bkash_rs::models::common::{Currency, Money};
use bkash_rs::models::token::{GrantTokenRequest, RefreshTokenRequest};
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

/// Mount a `POST /checkout/token/grant` mock that returns a fixed id_token.
/// This is the URL-based checkout token endpoint — distinct from the
/// tokenized product's `/tokenized/checkout/token/grant`.
async fn mount_grant(server: &MockServer, id_token: &str) {
    Mock::given(method("POST"))
        .and(path("/checkout/token/grant"))
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

async fn mount_refresh(server: &MockServer, id_token: &str) {
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/token/refresh"))
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
// Token management
// ============================================================

#[tokio::test]
async fn grant_token_returns_token_response() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-grant").await;

    let bkash = bkash_for(&server).await;
    let req = GrantTokenRequest::new("test-app-key", "test-app-secret");
    let resp = bkash.checkout().grant_token(req).await.unwrap();
    assert_eq!(resp.id_token, "id-grant");
    assert_eq!(resp.token_type, "Bearer");
}

#[tokio::test]
async fn refresh_token_returns_new_token() {
    let server = MockServer::start().await;
    mount_refresh(&server, "id-refreshed").await;

    let bkash = bkash_for(&server).await;
    let req = RefreshTokenRequest::new("test-app-key", "test-app-secret", "old-refresh");
    let resp = bkash.checkout().refresh_token(req).await.unwrap();
    assert_eq!(resp.id_token, "id-refreshed");
}

// ============================================================
// Payment lifecycle
// ============================================================

#[tokio::test]
async fn create_payment_uses_mode_0011() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/create"))
        .and(header("Authorization", "Bearer id-abc"))
        .and(header("X-APP-Key", "test-app-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0100",
            "bkashURL": "https://example.test/bkash",
            "callbackURL": "https://merchant.test/cb",
            "paymentCreateTime": "2026-06-22T10:00:00:000 GMT+06:00",
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
    let req = CreatePaymentRequest::new(
        "cust-1",
        "https://merchant.test/cb",
        Money::bdt("50.00"),
        Currency::Bdt,
    )
    .with_merchant_invoice_number("INV-PAY-1")
    .with_merchant_association_info("tag1v1");
    let resp = bkash.checkout().create_payment(req).await.unwrap();
    assert_eq!(resp.payment_id, "TR0100");
    assert_eq!(resp.bkash_url, "https://example.test/bkash");
    assert_eq!(resp.intent, Intent::Sale);
}

#[tokio::test]
async fn execute_payment_returns_trx_id() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/execute"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0100",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "sale",
            "paymentExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00",
            "customerMsisdn": "01700000000",
            "orgShortCode": "0123",
            "merchantInvoiceNumber": ""
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash.checkout().execute_payment("TR0100").await.unwrap();
    assert_eq!(resp.payment_id, "TR0100");
    assert_eq!(resp.trx_id, "8A00ABCD");
    assert_eq!(resp.transaction_status, "Completed");
    assert_eq!(resp.amount.as_str(), "50.00");
}

#[tokio::test]
async fn query_payment_returns_status() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/status"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0100",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "sale",
            "payerReference": "cust-1",
            "customerMsisdn": "01700000000",
            "orgShortCode": "0123",
            "callbackURL": "https://merchant.test/cb",
            "merchantInvoiceNumber": "",
            "paymentExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash.checkout().query_payment("TR0100").await.unwrap();
    assert_eq!(resp.payment_id, "TR0100");
    assert_eq!(resp.trx_id, "8A00ABCD");
    assert_eq!(resp.transaction_status, "Completed");
    assert_eq!(resp.amount.as_str(), "50.00");

    let _ = QueryPaymentRequest::new("TR0100");
}

#[tokio::test]
async fn execute_payment_request_serialises() {
    let req = ExecutePaymentRequest::new("TR0100");
    let v: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(v["paymentID"], "TR0100");
}

// ============================================================
// Search transaction
// ============================================================

#[tokio::test]
async fn search_transaction_returns_regular_shape() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/general/searchTransaction"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed",
            "transactionType": "Payment",
            "amount": "100.00",
            "currency": "BDT",
            "customerMsisdn": "01700000000",
            "organizationShortCode": "0123",
            "initiationTime": "2026-06-22T09:00:00:000 GMT+06:00",
            "completedTime": "2026-06-22T09:01:00:000 GMT+06:00",
            "payerType": "Customer",
            "maxRefundableAmount": "100.00",
            "saleAmount": "100.00",
            "serviceFee": "0.00",
            "payerAccount": "123456789"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash
        .checkout()
        .search_transaction("8A00ABCD")
        .await
        .unwrap();
    assert_eq!(resp.trx_id, "8A00ABCD");
    assert!(!resp.is_coupon());
    assert_eq!(resp.sale_amount.as_str(), "100.00");

    let _ = SearchTransactionRequest::new("8A00ABCD");
}

#[tokio::test]
async fn search_transaction_returns_coupon_shape() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/general/searchTransaction"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "trxID": "8A00EFGH",
            "transactionStatus": "Completed",
            "transactionType": "Payment",
            "amount": "100.00",
            "currency": "BDT",
            "customerMsisdn": "01700000000",
            "organizationShortCode": "0123",
            "initiationTime": "2026-06-22T09:00:00:000 GMT+06:00",
            "completedTime": "2026-06-22T09:01:00:000 GMT+06:00",
            "payerType": "Customer",
            "maxRefundableAmount": "100.00",
            "saleAmount": "90.00",
            "serviceFee": "0.00",
            "payerAccount": "123456789",
            "couponAmount": "10.00",
            "merchantShareAmount": "85.00",
            "creditedAmount": "95.00"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash
        .checkout()
        .search_transaction("8A00EFGH")
        .await
        .unwrap();
    assert!(resp.is_coupon());
    assert_eq!(resp.coupon_amount.as_str(), "10.00");
    assert_eq!(resp.merchant_share_amount.as_str(), "85.00");
    assert_eq!(resp.credited_amount.as_str(), "95.00");
}

// ============================================================
// Refund lifecycle
// ============================================================

#[tokio::test]
async fn refund_returns_refund_trx_id() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/v2/tokenized-checkout/refund/payment/transaction"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0100",
            "trxID": "8A00ABCD",
            "refundTrxID": "8B00EFGH",
            "refundAmount": "25.00",
            "sku": "sku-1",
            "reason": "customer-return",
            "maxRefundableAmount": "75.00",
            "organizationShortCode": "0123",
            "transactionStatus": "Completed",
            "initiationTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "completedTime": "2026-06-22T10:01:00:000 GMT+06:00"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let req = RefundRequest::new(
        "TR0100",
        "8A00ABCD",
        Money::bdt("25.00"),
        "sku-1",
        "customer-return",
    );
    let resp = bkash.checkout().refund(req).await.unwrap();
    assert_eq!(resp.refund_trx_id, "8B00EFGH");
    assert_eq!(resp.refund_amount.as_str(), "25.00");
    assert_eq!(resp.max_refundable_amount.as_str(), "75.00");
}

#[tokio::test]
async fn refund_status_returns_current_state() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/v2/tokenized-checkout/refund/payment/status"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0100",
            "trxID": "8A00ABCD",
            "refundTrxID": "8B00EFGH",
            "refundAmount": "25.00",
            "sku": "sku-1",
            "reason": "customer-return",
            "maxRefundableAmount": "75.00",
            "transactionStatus": "Completed",
            "initiationTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "completedTime": "2026-06-22T10:01:00:000 GMT+06:00"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash
        .checkout()
        .refund_status("TR0100", "8A00ABCD")
        .await
        .unwrap();
    assert_eq!(resp.refund_trx_id, "8B00EFGH");
    assert_eq!(resp.transaction_status, "Completed");

    let _ = RefundStatusRequest::new("TR0100", "8A00ABCD");
}

// ============================================================
// Error envelope mapping
// ============================================================

#[tokio::test]
async fn create_payment_2001_maps_to_auth_error() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errorCode": "2001",
            "errorMessage": "Invalid App Key"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let req = CreatePaymentRequest::new(
        "cust-1",
        "https://merchant.test/cb",
        Money::bdt("100.00"),
        Currency::Bdt,
    );
    let err = bkash.checkout().create_payment(req).await.unwrap_err();
    match err {
        Error::Auth(msg) => assert!(msg.contains("2001"), "got: {msg}"),
        other => panic!("expected Error::Auth, got {other:?}"),
    }
}

#[tokio::test]
async fn refund_invalid_amount_maps_to_api_error() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/v2/tokenized-checkout/refund/payment/transaction"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errorCode": "2071",
            "errorMessage": "Refund: invalid amount"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let req = RefundRequest::new("TR0100", "8A00ABCD", Money::bdt("25.00"), "sku-1", "test");
    let err = bkash.checkout().refund(req).await.unwrap_err();
    match err {
        Error::Api { code, message, .. } => {
            assert_eq!(code, "2071");
            assert!(message.contains("Refund"));
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
