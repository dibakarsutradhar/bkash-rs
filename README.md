# bkash-rs

[![CI](https://github.com/dibakarsutradhar/bkash-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/dibakarsutradhar/bkash-rs/actions/workflows/ci.yml)
[![docs.rs](https://docs.rs/bkash-rs/badge.svg)](https://docs.rs/bkash-rs)
[![crates.io](https://img.shields.io/crates/v/bkash-rs.svg)](https://crates.io/crates/bkash-rs)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![MSRV 1.75](https://img.shields.io/badge/MSRV-1.75-blue.svg)](#msrv)

Idiomatic async-first Rust client for the **bKash Payment Gateway API**.
`bkash-rs` provides a strongly-typed, ergonomic interface to bKash's payment
products — tokenized checkout, classic URL-based checkout, authorization
& capture, subscriptions, and SNS-style webhook verification — with
transparent token caching, idempotent retries on `5xx` / network errors,
and built-in test fixtures.

> **Status:** Pre-1.0 development. The API may change before `0.2.0`.
> See [Roadmap](#roadmap--v02-publish) for details. Not yet published to
> crates.io — v0.2 is gated on internal validation.

## Quickstart

Add `bkash-rs` to `Cargo.toml`:

```toml
[dependencies]
bkash-rs = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Then create a client and call a method:

```rust
use bkash_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Build configuration for the sandbox environment.
    let config = Bkash::builder()
        .environment(Environment::Sandbox)
        .app_key("your_app_key")
        .app_secret("your_app_secret")
        .username("your_username")
        .password("your_password")
        .build()?;

    // 2. Construct the client (verifies credentials, warms the token cache).
    let bkash = Bkash::new(config).await?;

    // 3. Call a product method. Each product is a separate feature-gated
    //    accessor.
    let tokenized = bkash.tokenized();
    let resp = tokenized
        .query_payment("TRX000001")
        .await?;

    println!("{:?}", resp);
    Ok(())
}
```

The same pattern works for `bkash.checkout()`, `bkash.auth_capture()`, and
`bkash.subscriptions()`. See [API Reference](#api-reference) below for the
full endpoint → method index.

## Sandbox vs Production

The bKash Payment Gateway exposes two environments. Switch with
[`Environment`]:

| Environment | Base URL                                                      | Use for                           |
| ----------- | ------------------------------------------------------------- | --------------------------------- |
| `Sandbox`   | `https://tokenized.sandbox.bka.sh/v1.2.0-beta`                | Development & integration testing |
| `Production`| `https://tokenized.pay.bka.sh/v1.2.0-beta`                    | Live transactions (BDT)           |

```rust
use bkash_rs::Environment;

let config = Bkash::builder()
    .environment(Environment::Production) // switch to live
    // ...
    .build()?;
```

`bkash-rs` is **environment-agnostic** beyond the base URL — it does not
gate "live" calls, add dry-run modes, or warn you. You are responsible for
choosing the correct environment for your use case.

## Supported products

| Product                                | Accessor           | Feature flag         | Notes                                                                 |
| -------------------------------------- | ------------------ | -------------------- | --------------------------------------------------------------------- |
| [Tokenized Checkout](#tokenized-checkout) | `bkash.tokenized()`    | `tokenized-checkout` | Modern TLV-based checkout flow (the only bKash flow with new merchants). |
| [URL-based Checkout](#url-based-checkout) | `bkash.checkout()`     | `checkout`           | Classic `create → execute` URL flow (legacy but still supported).     |
| [Auth & Capture](#auth--capture)           | `bkash.auth_capture()` | `auth-capture`       | Two-step authorize-then-capture for delayed settlement.              |
| [Subscriptions](#subscriptions)            | `bkash.subscriptions()`| `subscriptions`      | Agreement-based recurring payments.                                  |
| [Webhooks](#webhooks)                     | `bkash_rs::webhooks::*`| `webhooks`           | Verify SNS-style signature envelopes delivered to your endpoint.     |

`tokenized` and `checkout` are thin façades over the same internal
`Transport`; they differ only in their default `Product` (which selects
the per-product base URL). `auth-capture` and `subscriptions` use
`tokenized`-style endpoints under the hood — they are product
*accessors*, not separate wire protocols.

## Cargo Features

| Feature             | Default | Description                                                                                              |
| ------------------- | :-----: | -------------------------------------------------------------------------------------------------------- |
| `rustls-tls`        |   ✅    | Use [`rustls`](https://github.com/rustls/rustls) for TLS (pure Rust, no OpenSSL dep).                    |
| `native-tls`        |   ❌    | Use the platform's native TLS (e.g. Secure Transport on macOS, SChannel on Windows, OpenSSL on Linux).    |
| `tokenized-checkout`|   ✅    | Enables `bkash.tokenized()` — Tokenized Checkout product accessor.                                       |
| `checkout`          |   ✅    | Enables `bkash.checkout()` — classic URL-based Checkout product accessor.                                |
| `auth-capture`      |   ❌    | Enables `bkash.auth_capture()` — Authorization & Capture product accessor.                                |
| `subscriptions`     |   ❌    | Enables `bkash.subscriptions()` — Subscriptions product accessor.                                        |
| `webhooks`          |   ❌    | Enables the `webhooks` module: `verify_sns_signature`, `parse_event`, `WebhookEvent`, `SnsEnvelope`, etc. |

TLS features are mutually exclusive — pick **one** of `rustls-tls` or
`native-tls`. The default (`rustls-tls`) works on every platform Rust
supports; switch to `native-tls` only if you need OS-native cert stores.

The product features are independent — enable exactly the products you
use. Disabling a feature removes its accessor at compile time:

```toml
# Minimal build: just Auth & Capture + Webhook verification, no checkout.
bkash-rs = { version = "0.1", default-features = false, features = ["rustls-tls", "auth-capture", "webhooks"] }
```

## MSRV

The minimum supported Rust version is **1.75**. Bumping MSRV is a breaking
change and will require a minor version bump.

## API Reference

This table maps every supported bKash endpoint to its crate method. HTTP
method and path are what bKash's API actually exposes; the crate method
sits one level above that and returns a typed response.

### Tokenized Checkout

Requires the `tokenized-checkout` feature.

| HTTP | Endpoint path                                        | Crate method                                  | Notes |
| :--: | ---------------------------------------------------- | --------------------------------------------- | ----- |
| POST | `/tokenized/checkout/token/grant`                    | `Bkash::tokenized().grant_token(req)`         | Auto-invoked by [`Transport`] on first request; rarely called directly. |
| POST | `/tokenized/checkout/token/refresh`                  | `Bkash::tokenized().refresh_token(req)`       | Refresh a near-expiry `id_token` (>= `refreshTokenExpiryTime`). |
| POST | `/tokenized/checkout/create`                         | `Bkash::tokenized().create_agreement(req)`    | Create a billing agreement (`agreementID`). |
| POST | `/tokenized/checkout/execute`                        | `Bkash::tokenized().execute_agreement(req)`   | Execute an agreement → returns `paymentID`. |
| POST | `/tokenized/checkout/agreement/status`               | `Bkash::tokenized().query_agreement(req)`    | Query agreement state. |
| POST | `/tokenized/checkout/agreement/cancel`               | `Bkash::tokenized().cancel_agreement(req)`    | Cancel a `DRAFT` or `ACTIVE` agreement. |
| POST | `/tokenized/checkout/create`                         | `Bkash::tokenized().create_payment(req)`      | Create a payment (after `paymentID` exists). |
| POST | `/tokenized/checkout/execute/{paymentID}`            | `Bkash::tokenized().execute_payment(payment_id)` | Execute payment → returns `trxID`. |
| POST | `/tokenized/checkout/payment/status`                 | `Bkash::tokenized().query_payment(payment_id)` | Query payment state. |
| POST | `/tokenized/checkout/general/searchTransaction`      | `Bkash::tokenized().search_transaction(req)`  | Look up a transaction by `trxID`. |
| POST | `/tokenized/checkout/payment/refund`                 | `Bkash::tokenized().refund(req)`              | Refund a captured payment. |
| POST | `/tokenized/checkout/payment/refund/status`          | `Bkash::tokenized().refund_status(req)`       | Look up a refund by `refundID`. |

### URL-based Checkout

Requires the `checkout` feature. Endpoints are identical to the
tokenized checkout paths — only the base URL product differs.

| HTTP | Endpoint path                                        | Crate method                                  |
| :--: | ---------------------------------------------------- | --------------------------------------------- |
| POST | `/tokenized/checkout/token/grant`                    | `Bkash::checkout().grant_token(req)`          |
| POST | `/tokenized/checkout/token/refresh`                  | `Bkash::checkout().refresh_token(req)`        |
| POST | `/tokenized/checkout/create`                         | `Bkash::checkout().create_payment(req)`       |
| POST | `/tokenized/checkout/execute/{paymentID}`            | `Bkash::checkout().execute_payment(payment_id)` |
| POST | `/tokenized/checkout/payment/status`                 | `Bkash::checkout().query_payment(payment_id)` |
| POST | `/tokenized/checkout/general/searchTransaction`      | `Bkash::checkout().search_transaction(req)`   |
| POST | `/tokenized/checkout/payment/refund`                 | `Bkash::checkout().refund(req)`               |
| POST | `/tokenized/checkout/payment/refund/status`          | `Bkash::checkout().refund_status(req)`        |

### Auth & Capture

Requires the `auth-capture` feature.

| HTTP | Endpoint path                                            | Crate method                                       |
| :--: | -------------------------------------------------------- | -------------------------------------------------- |
| POST | `/tokenized/checkout/token/grant`                        | `Bkash::auth_capture().grant_token(req)`           |
| POST | `/tokenized/checkout/token/refresh`                      | `Bkash::auth_capture().refresh_token(req)`         |
| POST | `/tokenized/checkout/payment/create`                     | `Bkash::auth_capture().create_payment(req)`        |
| POST | `/tokenized/checkout/execute/{paymentID}`                | `Bkash::auth_capture().execute_payment(payment_id)` |
| POST | `/tokenized/checkout/payment/status`                     | `Bkash::auth_capture().query_payment(payment_id)`  |
| POST | `/tokenized/checkout/payment/confirm/capture`             | `Bkash::auth_capture().capture(payment_id)`        |
| POST | `/tokenized/checkout/payment/confirm/capture/void`       | `Bkash::auth_capture().void(payment_id)`           |
| GET  | `/checkout/payment/search/{trxID}`                       | `Bkash::auth_capture().search_transaction(req)`    |

### Subscriptions

Requires the `subscriptions` feature. bKash hosts Subscriptions on the
tokenized-checkout agreement endpoints; the accessor is a typed façade
over those same calls.

| HTTP | Endpoint path                            | Crate method                                       |
| :--: | ---------------------------------------- | -------------------------------------------------- |
| POST | `/tokenized/checkout/create`             | `Bkash::subscriptions().create_subscription(req)`  |
| POST | `/tokenized/checkout/execute`            | `Bkash::subscriptions().execute_subscription(req)` |
| POST | `/tokenized/checkout/agreement/status`   | `Bkash::subscriptions().query_subscription(req)`   |
| POST | `/tokenized/checkout/agreement/cancel`   | `Bkash::subscriptions().cancel_subscription(req)`  |

### Webhooks

Requires the `webhooks` feature. These are helpers, not HTTP calls — they
verify and parse incoming SNS envelopes delivered to your endpoint.

| Helper                                            | Description                                                |
| ------------------------------------------------- | ---------------------------------------------------------- |
| `webhooks::verify_sns_signature(envelope)`        | Fetch the signing certificate and verify the signature.   |
| `webhooks::verify_signature_with_key(envelope, k)`| Verify a signature against a pre-fetched `RsaPublicKey`.   |
| `webhooks::parse_event(envelope)`                 | Parse a verified envelope into a typed `WebhookEvent`.     |
| `webhooks::build_string_to_sign(envelope)`        | Build the canonical string-to-sign for manual verification.|
| `webhooks::cert_pem_to_public_key(pem)`           | Parse a PEM-encoded X.509 cert into a `RsaPublicKey`.      |
| `webhooks::confirm_subscription(envelope, url)`   | POST to a `SubscribeURL` to confirm a new subscription.    |
| `WebhookEvent`                                    | Parsed event: `trx_id()`, `as_code()`, `from_code(s)`, etc.|

## Roadmap / v0.2 publish

`bkash-rs` is on the `0.1.x` track. The `0.2.0` release — and the first
**crates.io publish** — is gated on internal validation:

- A real Rust web service must integrate the crate end-to-end against
  the bKash sandbox **and** production environments.
- The integration must exercise at least one of each: tokenized checkout,
  auth & capture, subscription creation, and a webhook delivery.
- Any rough edges uncovered (e.g. config ergonomics, error mapping,
  request builder patterns) should be filed as issues and resolved before
  the 0.2 cut.

Until then, the crate is **not published to crates.io** and the API is
permitted to evolve. The 0.1.x series on this repository is treated as
"internal preview" — useful for integration but not yet a public
contract. See [`CHANGELOG.md`](CHANGELOG.md) for the current state.

## Contributing

Contributions are welcome — please read [`CONTRIBUTING.md`](CONTRIBUTING.md)
first. It covers building from source, the test layout, fixture
recording, and the opt-in live-sandbox smoke tests.

## Security

Report security issues privately — see
[`SECURITY.md`](SECURITY.md). **Do not** file public GitHub issues for
vulnerabilities.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT) at your option.
