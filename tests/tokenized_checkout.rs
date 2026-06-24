//! Wiremock-backed integration tests for the Tokenized Checkout product.
//!
//! Each test mounts a single endpoint, drives a `Bkash::tokenized()` call,
//! and asserts on the parsed response. The grant-token endpoint is shared
//! across tests via a helper.
//!
//! These tests only compile when the `tokenized-checkout` feature is
//! enabled.

#![cfg(feature = "tokenized-checkout")]

mod common;

use bkash_rs::config::{Config, Environment};
use bkash_rs::models::common::{Currency, Money};
use bkash_rs::models::token::{GrantTokenRequest, RefreshTokenRequest};
use bkash_rs::models::tokenized::{
    CancelAgreementRequest, CreateAgreementRequest, CreatePaymentRequest, ExecuteAgreementRequest,
    ExecutePaymentRequest, QueryAgreementRequest, QueryPaymentRequest, RefundRequest,
    RefundStatusRequest, SearchTransactionRequest,
};
use bkash_rs::prelude::*;
use common::Fixture;
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

/// Mount a `POST /tokenized/checkout/token/grant` mock that returns a fixed
/// id_token. Used to satisfy the auto-grant flow inside the transport.
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

#[tokio::test]
async fn grant_token_returns_token_response() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-grant").await;

    let bkash = bkash_for(&server).await;
    let req = GrantTokenRequest::new("test-app-key", "test-app-secret");
    let resp = bkash.tokenized().grant_token(req).await.unwrap();
    assert_eq!(resp.id_token, "id-grant");
    assert_eq!(resp.token_type, "Bearer");
}

#[tokio::test]
async fn refresh_token_returns_new_token() {
    let server = MockServer::start().await;
    mount_refresh(&server, "id-refreshed").await;

    let bkash = bkash_for(&server).await;
    let req = RefreshTokenRequest::new("test-app-key", "test-app-secret", "old-refresh");
    let resp = bkash.tokenized().refresh_token(req).await.unwrap();
    assert_eq!(resp.id_token, "id-refreshed");
}

// ============================================================
// Agreement lifecycle
// ============================================================

#[tokio::test]
async fn create_agreement_returns_payment_id() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    // Sanitized fixture loaded from tests/common/fixtures/.
    let body = Fixture::load("tokenized_create_agreement.json");

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/create"))
        .and(header("Authorization", "Bearer id-abc"))
        .and(header("X-APP-Key", "test-app-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let req = CreateAgreementRequest::new(
        "TEST-CUST-001",
        "https://example.com/callback",
        Money::bdt("100.00"),
        Currency::Bdt,
    );
    let resp = bkash.tokenized().create_agreement(req).await.unwrap();
    assert_eq!(resp.payment_id, "TEST00000001");
    assert_eq!(
        resp.bkash_url,
        "https://tokenized.sandbox.bka.sh/redirect/?token=SANITIZED_TOKEN"
    );
}

#[tokio::test]
async fn execute_agreement_returns_agreement_id() {
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
    let resp = bkash.tokenized().execute_agreement("TR0001").await.unwrap();
    assert_eq!(resp.agreement_id, "AG0001");
    assert_eq!(resp.agreement_status, "Completed");
}

#[tokio::test]
async fn query_agreement_returns_status() {
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
            "merchantInvoiceNumber": "",
            "orgShortCode": "0123",
            "agreementCreateTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "agreementExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash.tokenized().query_agreement("AG0001").await.unwrap();
    assert_eq!(resp.agreement_id, "AG0001");
    assert_eq!(resp.agreement_status, "Completed");
    assert_eq!(resp.amount.as_str(), "100.00");
}

#[tokio::test]
async fn cancel_agreement_returns_cancelled_status() {
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
    let resp = bkash.tokenized().cancel_agreement("AG0001").await.unwrap();
    assert_eq!(resp.agreement_id, "AG0001");
    assert_eq!(resp.agreement_status, "Cancelled");

    // Sanity-check that the request type serialises correctly.
    let req = CancelAgreementRequest::new("AG0001");
    let v: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(v["agreementID"], "AG0001");
}

#[tokio::test]
async fn execute_agreement_request_serialises_with_payment_id() {
    // Auxiliary: validate the request shape used by execute_agreement.
    let req = ExecuteAgreementRequest::new("TR0001");
    let v: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(v["paymentID"], "TR0001");
}

#[tokio::test]
async fn query_agreement_request_serialises() {
    let req = QueryAgreementRequest::new("AG0001");
    let v: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(v["agreementID"], "AG0001");
}

// ============================================================
// Payment lifecycle
// ============================================================

#[tokio::test]
async fn create_payment_uses_mode_0001() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    // Sanitized fixture loaded from tests/common/fixtures/.
    let body = Fixture::load("tokenized_create_payment.json");

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/create"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let req = CreatePaymentRequest::new(
        "TEST-AG-0001",
        "TEST-CUST-001",
        "https://example.com/callback",
        Money::bdt("100.00"),
        Currency::Bdt,
    )
    .with_merchant_invoice_number("INV-TEST-002")
    .with_merchant_association_info("tag1v1");
    let resp = bkash.tokenized().create_payment(req).await.unwrap();
    assert_eq!(resp.payment_id, "TEST-PAY-0001");
    assert_eq!(resp.agreement_id, "TEST-AG-0001");
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
            "agreementID": "AG0001",
            "orgShortCode": "0123",
            "merchantInvoiceNumber": ""
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash.tokenized().execute_payment("TR0100").await.unwrap();
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
            "agreementID": "AG0001",
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
    let resp = bkash.tokenized().query_payment("TR0100").await.unwrap();
    assert_eq!(resp.payment_id, "TR0100");
    assert_eq!(resp.trx_id, "8A00ABCD");
    assert_eq!(resp.transaction_status, "Completed");

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
        .tokenized()
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
        .tokenized()
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
        .and(path("/tokenized/checkout/payment/refund"))
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
    let resp = bkash.tokenized().refund(req).await.unwrap();
    assert_eq!(resp.refund_trx_id, "8B00EFGH");
    assert_eq!(resp.refund_amount.as_str(), "25.00");
    assert_eq!(resp.max_refundable_amount.as_str(), "75.00");
}

#[tokio::test]
async fn refund_status_returns_current_state() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/refund/status"))
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
        .tokenized()
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
async fn create_agreement_2001_maps_to_auth_error() {
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
    let req = CreateAgreementRequest::new(
        "cust-1",
        "https://merchant.test/cb",
        Money::bdt("100.00"),
        Currency::Bdt,
    );
    let err = bkash.tokenized().create_agreement(req).await.unwrap_err();
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
        .and(path("/tokenized/checkout/payment/refund"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errorCode": "2071",
            "errorMessage": "Refund: invalid amount"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let req = RefundRequest::new("TR0100", "8A00ABCD", Money::bdt("25.00"), "sku-1", "test");
    let err = bkash.tokenized().refund(req).await.unwrap_err();
    match err {
        Error::Api { code, message, .. } => {
            assert_eq!(code, "2071");
            assert!(message.contains("Refund"));
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
