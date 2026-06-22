//! Wiremock-backed integration tests for the `Transport`.
//!
//! These tests exercise the full grant-token + authenticated-call flow
//! against a fake bKash server.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use bkash_rs::config::{Config, Environment, Product};
use bkash_rs::error::Error;
use bkash_rs::transport::Transport;
use serde::{Deserialize, Serialize};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// A sample response type used to verify successful decoding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SampleReply {
    payment_id: String,
    amount: String,
}

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

async fn transport_for(server: &MockServer) -> Transport {
    let cfg = base_test_config(&server.uri());
    Transport::new(cfg).await.unwrap()
}

#[tokio::test]
async fn grant_token_then_authenticated_call() {
    let server = MockServer::start().await;

    // 1) Grant token endpoint. The transport does NOT set X-APP-Key on
    // the grant request (credentials are in the body), so we don't assert
    // that header here.
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/token/grant"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "id_token": "id-abc",
            "refresh_token": "refresh-xyz",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // 2) Authenticated endpoint
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .and(header("Authorization", "Bearer id-abc"))
        .and(header("X-APP-Key", "test-app-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "payment_id": "TRX0001",
            "amount": "100.00"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let transport = transport_for(&server).await;
    let body = serde_json::json!({
        "amount": "100.00",
        "currency": "BDT",
        "intent": "sale"
    });
    let resp: SampleReply = transport
        .request(
            Product::Tokenized,
            reqwest::Method::POST,
            "tokenized/checkout/payment/create",
            Some(&body),
        )
        .await
        .unwrap();
    assert_eq!(resp.payment_id, "TRX0001");
    assert_eq!(resp.amount, "100.00");
}

#[tokio::test]
async fn api_error_2001_maps_to_auth() {
    let server = MockServer::start().await;

    // 1) Grant token (Checkout product uses /checkout/token/grant)
    Mock::given(method("POST"))
        .and(path("/checkout/token/grant"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "id_token": "id-abc",
            "refresh_token": "refresh-xyz",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // 2) Authenticated endpoint returns 2001 in body
    Mock::given(method("POST"))
        .and(path("/checkout/payment/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "errorCode": "2001",
            "errorMessage": "Invalid App Key"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let transport = transport_for(&server).await;
    let err = transport
        .request::<serde_json::Value, serde_json::Value>(
            Product::Checkout,
            reqwest::Method::POST,
            "checkout/payment/create",
            Some(&serde_json::json!({})),
        )
        .await
        .unwrap_err();
    // 2001 Invalid App Key is mapped to Error::Auth.
    match &err {
        Error::Auth(msg) => assert!(msg.contains("2001")),
        other => panic!("expected Error::Auth, got {other:?}"),
    }
}

#[tokio::test]
async fn api_error_non_auth_maps_to_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/checkout/token/grant"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "id_token": "id-abc",
            "refresh_token": "refresh-xyz",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/checkout/payment/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "errorCode": "2023",
            "errorMessage": "Insufficient Balance"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let transport = transport_for(&server).await;
    let err = transport
        .request::<serde_json::Value, serde_json::Value>(
            Product::Checkout,
            reqwest::Method::POST,
            "checkout/payment/create",
            Some(&serde_json::json!({})),
        )
        .await
        .unwrap_err();
    match &err {
        Error::Api { code, message, .. } => {
            assert_eq!(code, "2023");
            assert!(message.contains("Insufficient"));
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn http_401_triggers_force_regrant_and_retry() {
    let server = MockServer::start().await;
    let grants = Arc::new(AtomicUsize::new(0));

    // Grant endpoint that counts hits.
    struct GrantCounter {
        counter: Arc<AtomicUsize>,
    }
    impl Respond for GrantCounter {
        fn respond(&self, _: &Request) -> ResponseTemplate {
            self.counter.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "statusCode": "0000",
                "statusMessage": "Success",
                "id_token": "id-abc",
                "refresh_token": "refresh-xyz",
                "expires_in": 3600,
                "token_type": "Bearer"
            }))
        }
    }
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/token/grant"))
        .respond_with(GrantCounter {
            counter: grants.clone(),
        })
        .mount(&server)
        .await;

    // First call returns 401, then 200. We use up_to_n_times for the 401 mock.
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "payment_id": "TRX0002",
            "amount": "50.00"
        })))
        .mount(&server)
        .await;

    let transport = transport_for(&server).await;
    let body = serde_json::json!({});
    let resp: SampleReply = transport
        .request(
            Product::Tokenized,
            reqwest::Method::POST,
            "tokenized/checkout/payment/create",
            Some(&body),
        )
        .await
        .unwrap();
    assert_eq!(resp.payment_id, "TRX0002");
    // grant called once initially; the 401 forced a re-grant → at least 2.
    let n = grants.load(Ordering::SeqCst);
    assert!(n >= 2, "expected ≥ 2 grants, got {n}");
}

#[tokio::test]
async fn post_regrant_transient_error_is_retried() {
    let server = MockServer::start().await;

    // Grant endpoint
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/token/grant"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "id_token": "id-abc",
            "refresh_token": "refresh-xyz",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    // First call: 401 → forces a re-grant
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Second call (post-regrant): 503 (transient) → should retry
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Third call: success
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "payment_id": "TRX0003",
            "amount": "75.00"
        })))
        .mount(&server)
        .await;

    // Configure the transport with at least one transient retry so the
    // post-regrant 503 is eligible for retry.
    let cfg = Config::builder()
        .environment(Environment::Sandbox)
        .app_key("test-app-key")
        .app_secret("test-app-secret")
        .username("test-user")
        .password("test-pass")
        .with_base_url(server.uri())
        .max_retries(2)
        .build()
        .unwrap();
    let transport = Transport::new(cfg).await.unwrap();

    let body = serde_json::json!({});
    let resp: SampleReply = transport
        .request(
            Product::Tokenized,
            reqwest::Method::POST,
            "tokenized/checkout/payment/create",
            Some(&body),
        )
        .await
        .unwrap();
    assert_eq!(resp.payment_id, "TRX0003");
}

#[tokio::test]
async fn double_401_after_regrant_surfaces_as_auth() {
    let server = MockServer::start().await;

    // Grant endpoint
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/token/grant"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "statusCode": "0000",
            "statusMessage": "Success",
            "id_token": "id-abc",
            "refresh_token": "refresh-xyz",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    // Every payment/create call returns 401 — including after re-grant.
    Mock::given(method("POST"))
        .and(path("/tokenized/checkout/payment/create"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&server)
        .await;

    let transport = transport_for(&server).await;
    let body = serde_json::json!({});
    let err = transport
        .request::<serde_json::Value, SampleReply>(
            Product::Tokenized,
            reqwest::Method::POST,
            "tokenized/checkout/payment/create",
            Some(&body),
        )
        .await
        .unwrap_err();
    match &err {
        Error::Auth(msg) => assert!(msg.contains("401 after force-regrant")),
        other => panic!("expected Error::Auth, got {other:?}"),
    }
}
