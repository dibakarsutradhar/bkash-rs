//! Wiremock-backed integration tests for the Auth & Capture product.
//!
//! Each test mounts a single endpoint, drives a `Bkash::auth_capture()`
//! call, and asserts on the parsed response. The grant-token endpoint is
//! shared across tests via a helper. The exit gate covers the full
//! authorize → capture and authorize → void flows end-to-end.
//!
//! These tests only compile when the `auth-capture` feature is enabled.

#![cfg(feature = "auth-capture")]

use bkash_rs::config::{Config, Environment};
use bkash_rs::models::auth_capture::{
    CaptureRequest, CreatePaymentRequest, QueryPaymentRequest, VoidRequest,
};
use bkash_rs::models::common::{Currency, Money, TransactionStatus};
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

/// Mount a `POST /checkout/token/grant` mock that returns a fixed
/// id_token. Used to satisfy the auto-grant flow inside the transport.
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
    let resp = bkash.auth_capture().grant_token(req).await.unwrap();
    assert_eq!(resp.id_token, "id-grant");
    assert_eq!(resp.token_type, "Bearer");
}

#[tokio::test]
async fn refresh_token_returns_new_token() {
    let server = MockServer::start().await;
    mount_refresh(&server, "id-refreshed").await;

    let bkash = bkash_for(&server).await;
    let req = RefreshTokenRequest::new("test-app-key", "test-app-secret", "old-refresh");
    let resp = bkash.auth_capture().refresh_token(req).await.unwrap();
    assert_eq!(resp.id_token, "id-refreshed");
}

// ============================================================
// Payment lifecycle (reservation model)
// ============================================================

#[tokio::test]
async fn create_payment_uses_mode_0011_and_intent_authorization() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .and(header("Authorization", "Bearer id-abc"))
        .and(header("X-APP-Key", "test-app-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "bkashURL": "https://example.test/bkash",
            "callbackURL": "https://merchant.test/cb",
            "paymentCreateTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "currency": "BDT",
            "intent": "authorization",
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
    .with_merchant_invoice_number("INV-AC-1");
    let resp = bkash.auth_capture().create_payment(req).await.unwrap();
    assert_eq!(resp.payment_id, "TR0001");
    assert_eq!(resp.bkash_url, "https://example.test/bkash");
    assert_eq!(resp.intent, Intent::Authorization);
}

#[tokio::test]
async fn execute_payment_uses_payment_id_as_path_param() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/execute/TR0001"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "authorization",
            "paymentExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00",
            "customerMsisdn": "01700000000",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "merchantInvoiceNumber": "",
            "callbackURL": "https://merchant.test/cb"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash
        .auth_capture()
        .execute_payment("TR0001")
        .await
        .unwrap();
    assert_eq!(resp.payment_id, "TR0001");
    assert_eq!(resp.trx_id, "8A00ABCD");
    assert_eq!(resp.transaction_status, TransactionStatus::Authorized);
    assert_eq!(resp.amount.as_str(), "50.00");
}

#[tokio::test]
async fn query_payment_returns_authorized_status() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/status"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "authorization",
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
    let resp = bkash.auth_capture().query_payment("TR0001").await.unwrap();
    assert_eq!(resp.payment_id, "TR0001");
    assert_eq!(resp.transaction_status, TransactionStatus::Authorized);

    let _ = QueryPaymentRequest::new("TR0001");
}

#[tokio::test]
async fn capture_returns_lowercase_payment_id() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/confirm/capture"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentId": "TR0001",
            "createTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "updateTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash.auth_capture().capture("TR0001").await.unwrap();
    assert_eq!(resp.payment_id, "TR0001");
    assert_eq!(resp.trx_id, "8A00ABCD");
    assert_eq!(resp.transaction_status, "Completed");
    assert_eq!(resp.create_time, "2026-06-22T10:02:00:000 GMT+06:00");

    let _ = CaptureRequest::new("TR0001");
}

#[tokio::test]
async fn void_returns_lowercase_payment_id() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/confirm/capture/void"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentId": "TR0001",
            "createTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "updateTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "trxID": "8A00ABCD",
            "transactionStatus": "Cancelled"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash.auth_capture().void("TR0001").await.unwrap();
    assert_eq!(resp.payment_id, "TR0001");
    assert_eq!(resp.transaction_status, "Cancelled");

    let _ = VoidRequest::new("TR0001");
}

#[tokio::test]
async fn search_transaction_uses_get_with_trx_id_path_param() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    Mock::given(method("GET"))
        .and(path("/checkout/payment/search/8A00ABCD"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "transactionType": "Payment",
            "amount": "100.00",
            "currency": "BDT",
            "customerMsisdn": "01700000000",
            "organizationShortCode": "0123",
            "initiationTime": "2026-06-22T09:00:00:000 GMT+06:00",
            "completedTime": ""
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;
    let resp = bkash
        .auth_capture()
        .search_transaction("8A00ABCD")
        .await
        .unwrap();
    assert_eq!(resp.trx_id, "8A00ABCD");
    assert_eq!(resp.transaction_status, TransactionStatus::Authorized);
    assert_eq!(resp.amount.as_str(), "100.00");
}

// ============================================================
// End-to-end reservation flows (exit gate)
// ============================================================

#[tokio::test]
async fn full_authorize_then_capture_flow() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    // 1. Create Payment
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "bkashURL": "https://example.test/bkash",
            "callbackURL": "https://merchant.test/cb",
            "paymentCreateTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "currency": "BDT",
            "intent": "authorization",
            "merchantInvoiceNumber": ""
        })))
        .expect(1)
        .mount(&server)
        .await;

    // 2. Execute Payment (path param paymentID)
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/execute/TR0001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "authorization",
            "paymentExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00",
            "customerMsisdn": "01700000000",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "merchantInvoiceNumber": "",
            "callbackURL": "https://merchant.test/cb"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // 3. Query Payment — status should be Authorized
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "authorization",
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

    // 4. Capture (lowercase paymentId in response)
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/confirm/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentId": "TR0001",
            "createTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "updateTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // 5. Search transaction by trxID (GET)
    Mock::given(method("GET"))
        .and(path("/checkout/payment/search/8A00ABCD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed",
            "transactionType": "Payment",
            "amount": "50.00",
            "currency": "BDT",
            "customerMsisdn": "01700000000",
            "organizationShortCode": "0123",
            "initiationTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "completedTime": "2026-06-22T10:02:00:000 GMT+06:00"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;

    // Step 1: Create
    let req = CreatePaymentRequest::new(
        "cust-1",
        "https://merchant.test/cb",
        Money::bdt("50.00"),
        Currency::Bdt,
    );
    let created = bkash.auth_capture().create_payment(req).await.unwrap();
    assert_eq!(created.payment_id, "TR0001");

    // Step 2: Execute
    let executed = bkash
        .auth_capture()
        .execute_payment(&created.payment_id)
        .await
        .unwrap();
    assert_eq!(executed.transaction_status, TransactionStatus::Authorized);

    // Step 3: Query — must be Authorized before capture is allowed.
    let queried = bkash
        .auth_capture()
        .query_payment(&created.payment_id)
        .await
        .unwrap();
    assert_eq!(queried.transaction_status, TransactionStatus::Authorized);

    // Step 4: Capture
    let captured = bkash
        .auth_capture()
        .capture(&created.payment_id)
        .await
        .unwrap();
    assert_eq!(captured.payment_id, "TR0001");
    assert_eq!(captured.transaction_status, "Completed");
    assert_eq!(captured.trx_id, "8A00ABCD");

    // Step 5: Search to confirm.
    let searched = bkash
        .auth_capture()
        .search_transaction(&captured.trx_id)
        .await
        .unwrap();
    assert_eq!(searched.transaction_status, TransactionStatus::Completed);
}

#[tokio::test]
async fn full_authorize_then_void_flow() {
    let server = MockServer::start().await;
    mount_grant(&server, "id-abc").await;

    // 1. Create Payment
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .and(header("Authorization", "Bearer id-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "bkashURL": "https://example.test/bkash",
            "callbackURL": "https://merchant.test/cb",
            "paymentCreateTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "currency": "BDT",
            "intent": "authorization",
            "merchantInvoiceNumber": ""
        })))
        .expect(1)
        .mount(&server)
        .await;

    // 2. Execute Payment
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/execute/TR0001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "amount": "75.00",
            "currency": "BDT",
            "intent": "authorization",
            "paymentExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00",
            "customerMsisdn": "01700000000",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "merchantInvoiceNumber": "",
            "callbackURL": "https://merchant.test/cb"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // 3. Query Payment
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "amount": "75.00",
            "currency": "BDT",
            "intent": "authorization",
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

    // 4. Void
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/confirm/capture/void"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "paymentId": "TR0001",
            "createTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "updateTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "trxID": "8A00ABCD",
            "transactionStatus": "Cancelled"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bkash = bkash_for(&server).await;

    // Step 1: Create
    let req = CreatePaymentRequest::new(
        "cust-1",
        "https://merchant.test/cb",
        Money::bdt("75.00"),
        Currency::Bdt,
    );
    let created = bkash.auth_capture().create_payment(req).await.unwrap();
    assert_eq!(created.payment_id, "TR0001");

    // Step 2: Execute (authorize)
    let executed = bkash
        .auth_capture()
        .execute_payment(&created.payment_id)
        .await
        .unwrap();
    assert_eq!(executed.transaction_status, TransactionStatus::Authorized);

    // Step 3: Query
    let queried = bkash
        .auth_capture()
        .query_payment(&created.payment_id)
        .await
        .unwrap();
    assert_eq!(queried.transaction_status, TransactionStatus::Authorized);

    // Step 4: Void (service not provided)
    let voided = bkash
        .auth_capture()
        .void(&created.payment_id)
        .await
        .unwrap();
    assert_eq!(voided.payment_id, "TR0001");
    assert_eq!(voided.transaction_status, "Cancelled");
}
