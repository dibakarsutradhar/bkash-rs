# bkash-rs

[![CI](https://github.com/dibakarsutradhar/bkash-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/dibakarsutradhar/bkash-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/bkash-rs.svg)](https://crates.io/crates/bkash-rs)
[![docs.rs](https://docs.rs/bkash-rs/badge.svg)](https://docs.rs/bkash-rs)
[![MSRV 1.75](https://img.shields.io/badge/MSRV-1.75-blue.svg)](#msrv)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Idiomatic async-first Rust client for the **bKash Payment Gateway API**.

`bkash-rs` covers every documented bKash payment product — tokenized
checkout, classic URL-based checkout, authorization & capture,
subscriptions, and SNS-style webhook signature verification — behind a
single, cheaply-cloneable [`Bkash`] handle with transparent token
caching, idempotent retries on transient failures, and typed error
mapping for bKash's `statusCode` envelope.

```rust
use bkash_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Configure credentials from your bKash developer portal.
    let bkash = Bkash::builder()
        .environment(Environment::Sandbox)
        .app_key("your_app_key")
        .app_secret("your_app_secret")
        .username("your_username")
        .password("your_password")
        .build_and_connect()       // validates config, warms the token cache
        .await?;

    // 2. Drive a payment through Tokenized Checkout.
    let resp = bkash
        .tokenized()
        .query_payment("TR0011")  // bKash paymentID, not trxID
        .await?;

    println!("status = {:?}, trxID = {}", resp.transaction_status, resp.trx_id);
    Ok(())
}
```

A runnable version of this flow lives in
[`examples/quickstart.rs`](examples/quickstart.rs); a staged sandbox
walk-through that prints `bkashURL`s for wallet approval lives in
[`examples/from_env.rs`](examples/from_env.rs).

## Features

| Feature flag           | Default | What it enables                                                                                                  |
| ---------------------- | :-----: | ---------------------------------------------------------------------------------------------------------------- |
| `rustls-tls`           |   ✅    | TLS via [`rustls`](https://github.com/rustls/rustls) — pure Rust, no OpenSSL dependency.                         |
| `native-tls`           |   ❌    | TLS via the platform's native stack (Secure Transport, SChannel, OpenSSL).                                       |
| `tokenized-checkout`   |   ✅    | `bkash.tokenized()` — Tokenized Checkout product accessor (the modern TLV-based flow).                          |
| `checkout`             |   ✅    | `bkash.checkout()` — classic URL-based Checkout product accessor.                                                |
| `auth-capture`         |   ❌    | `bkash.auth_capture()` — Authorization & Capture product accessor (two-step authorize-then-capture).            |
| `subscriptions`        |   ❌    | `bkash.subscriptions()` — Subscriptions product accessor (recurring billing agreements).                         |
| `webhooks`             |   ❌    | `bkash_rs::webhooks` — verify SNS-style webhook signatures and parse typed `WebhookEvent`s.                     |

The two TLS features are mutually exclusive — pick exactly one. The
product features are independent and additive; enabling more shrinks
your binary but gives you more API surface. A minimal build that
disables the default checkout products and keeps only Auth & Capture
plus webhook verification:

```toml
[dependencies]
bkash-rs = { version = "0.1", default-features = false, features = ["rustls-tls", "auth-capture", "webhooks"] }
```

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
bkash-rs = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Or with `cargo add`:

```bash
cargo add bkash-rs
cargo add tokio --features macros,rt-multi-thread
```

## Supported products

| Product                                            | Accessor                | Feature flag         | What it's for                                                                  |
| -------------------------------------------------- | ----------------------- | -------------------- | ------------------------------------------------------------------------------ |
| [Tokenized Checkout](#tokenized-checkout)          | `bkash.tokenized()`     | `tokenized-checkout` | Modern TLV-based checkout. The recommended path for new integrations.          |
| [URL-based Checkout](#url-based-checkout)          | `bkash.checkout()`      | `checkout`           | Classic one-shot URL redirect flow (legacy but still fully supported).         |
| [Auth & Capture](#auth--capture)                   | `bkash.auth_capture()`  | `auth-capture`       | Two-step authorize-then-capture for delayed settlement.                        |
| [Subscriptions](#subscriptions)                    | `bkash.subscriptions()` | `subscriptions`      | Agreement-based recurring billing. Typed façade over Tokenized Checkout.       |
| [Webhooks](#webhooks)                              | `bkash_rs::webhooks::*` | `webhooks`           | Verify SNS-style signatures on bKash webhook deliveries.                       |

Each accessor borrows the parent `Bkash` and reuses its `Transport`
and token cache — there is no per-product connection state. Calling
two different accessors from the same `Bkash` is just as cheap as
calling one twice.

## Sandbox vs Production

```rust
use bkash_rs::Environment;

Bkash::builder()
    .environment(Environment::Sandbox)     // or Environment::Production
    .app_key("...").app_secret("...")
    .username("...").password("...")
    .build_and_connect().await?;
```

`Environment::Sandbox` points at `https://tokenized.sandbox.bka.sh/v1.2.0-beta`
(per-product subdomains are selected automatically — see
[`Config::base_url`]). `Environment::Production` points at the
equivalent `*.pay.bka.sh` host. The crate does **not** gate live calls
or add a dry-run mode — choose the correct environment for your
context.

## API reference

Every supported bKash endpoint maps to one crate method. All methods
are `async`, take their request body by value, and return a typed
`Result<_, Error>`. HTTP method and path are what bKash's API
actually exposes; the crate method sits one level above that.

### Tokenized Checkout

Requires the `tokenized-checkout` feature. Endpoints route through the
`tokenized.sandbox.bka.sh` / `tokenized.pay.bka.sh` subdomain.

| HTTP | Endpoint path                                   | Crate method                                         | Notes                                                 |
| :--: | ----------------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------- |
| POST | `/tokenized/checkout/token/grant`               | `bkash.tokenized().grant_token(req)`                 | Auto-invoked by the transport on first request.       |
| POST | `/tokenized/checkout/token/refresh`             | `bkash.tokenized().refresh_token(req)`               | Refresh a near-expiry `id_token`.                     |
| POST | `/tokenized/checkout/create`                    | `bkash.tokenized().create_agreement(req)`            | Returns `paymentID` (a.k.a. `agreementCreateID`).     |
| POST | `/tokenized/checkout/execute`                   | `bkash.tokenized().execute_agreement(payment_id)`    | Returns the `agreementID`.                            |
| POST | `/tokenized/checkout/agreement/status`          | `bkash.tokenized().query_agreement(agreement_id)`    | Query current agreement state.                        |
| POST | `/tokenized/checkout/agreement/cancel`          | `bkash.tokenized().cancel_agreement(agreement_id)`   | Cancel a `DRAFT` or `ACTIVE` agreement.               |
| POST | `/tokenized/checkout/create`                    | `bkash.tokenized().create_payment(req)`              | Create a payment against an agreement (`mode=0001`). |
| POST | `/tokenized/checkout/execute`                   | `bkash.tokenized().execute_payment(payment_id)`      | Returns the `trxID`.                                  |
| POST | `/tokenized/checkout/payment/status`            | `bkash.tokenized().query_payment(payment_id)`        | Query payment state.                                  |
| POST | `/tokenized/checkout/general/searchTransaction` | `bkash.tokenized().search_transaction(trx_id)`       | Look up a transaction by `trxID`.                     |
| POST | `/tokenized/checkout/payment/refund`            | `bkash.tokenized().refund(req)`                      | Refund a captured payment. Up to 10 partials per txn. |
| POST | `/tokenized/checkout/payment/refund/status`     | `bkash.tokenized().refund_status(payment_id, trx_id)`| Look up a previously-issued refund.                   |

### URL-based Checkout

Requires the `checkout` feature. Wire endpoints are identical to
Tokenized Checkout — only the base URL subdomain (`checkout.*` vs.
`tokenized.*`) differs. There is **no agreement step**; this is the
classic one-shot `create → execute` flow.

| HTTP | Endpoint path                                   | Crate method                                       |
| :--: | ----------------------------------------------- | -------------------------------------------------- |
| POST | `/tokenized/checkout/token/grant`               | `bkash.checkout().grant_token(req)`                |
| POST | `/tokenized/checkout/token/refresh`             | `bkash.checkout().refresh_token(req)`              |
| POST | `/tokenized/checkout/create`                    | `bkash.checkout().create_payment(req)`             |
| POST | `/tokenized/checkout/execute`                   | `bkash.checkout().execute_payment(payment_id)`     |
| POST | `/tokenized/checkout/payment/status`            | `bkash.checkout().query_payment(payment_id)`       |
| POST | `/tokenized/checkout/general/searchTransaction` | `bkash.checkout().search_transaction(trx_id)`      |
| POST | `/tokenized/checkout/payment/refund`            | `bkash.checkout().refund(req)`                     |
| POST | `/tokenized/checkout/payment/refund/status`     | `bkash.checkout().refund_status(payment_id, trx_id)` |

`create_payment` automatically sets `mode = "0011"`; on Tokenized
Checkout `create_payment` automatically sets `mode = "0001"`.

### Auth & Capture

Requires the `auth-capture` feature. Routes through the same
`tokenized.*` subdomain as Tokenized Checkout. Use this when you want
to authorize a payment now and capture (settle) it later — e.g. for
marketplace escrow or delayed-charge flows.

| HTTP | Endpoint path                                       | Crate method                                              |
| :--: | --------------------------------------------------- | --------------------------------------------------------- |
| POST | `/tokenized/checkout/token/grant`                   | `bkash.auth_capture().grant_token(req)`                   |
| POST | `/tokenized/checkout/token/refresh`                 | `bkash.auth_capture().refresh_token(req)`                 |
| POST | `/tokenized/checkout/payment/create`                | `bkash.auth_capture().create_payment(req)`                |
| POST | `/tokenized/checkout/execute/{paymentID}`           | `bkash.auth_capture().execute_payment(payment_id)`        |
| POST | `/tokenized/checkout/payment/status`                | `bkash.auth_capture().query_payment(payment_id)`          |
| POST | `/tokenized/checkout/payment/confirm/capture`       | `bkash.auth_capture().capture(payment_id)`                |
| POST | `/tokenized/checkout/payment/confirm/capture/void`  | `bkash.auth_capture().void(payment_id)`                   |
| GET  | `/checkout/payment/search/{trxID}`                  | `bkash.auth_capture().search_transaction(trx_id)`         |

`execute_payment` here uses `paymentID` as a URL path parameter
(other products pass it in the request body).

### Subscriptions

Requires the `subscriptions` feature. bKash hosts Subscriptions on the
tokenized-checkout agreement endpoints, so `SubscriptionsClient` is a
typed façade over the same wire calls — it just routes to the
subscriptions product subdomain and tags requests so they appear under
the merchant's recurring-billing dashboard.

| HTTP | Endpoint path                            | Crate method                                                |
| :--: | ---------------------------------------- | ----------------------------------------------------------- |
| POST | `/tokenized/checkout/create`             | `bkash.subscriptions().create_subscription(req)`            |
| POST | `/tokenized/checkout/execute`            | `bkash.subscriptions().execute_subscription(payment_id)`    |
| POST | `/tokenized/checkout/agreement/status`   | `bkash.subscriptions().query_subscription(agreement_id)`    |
| POST | `/tokenized/checkout/agreement/cancel`   | `bkash.subscriptions().cancel_subscription(agreement_id)`   |

### Webhooks

Requires the `webhooks` feature. These are synchronous helpers, not
HTTP calls — they verify and parse inbound bKash SNS envelopes
delivered to your endpoint.

| Helper                                                | Description                                                  |
| ----------------------------------------------------- | ------------------------------------------------------------ |
| `webhooks::verify_sns_signature(envelope)`            | Fetch the signing certificate and verify the signature.     |
| `webhooks::verify_signature_with_key(envelope, key)`  | Verify against a pre-fetched `rsa::RsaPublicKey`.            |
| `webhooks::parse_event(envelope)`                     | Parse a verified envelope into a typed `WebhookEvent`.       |
| `webhooks::build_string_to_sign(envelope)`            | Build the canonical string-to-sign for manual verification.  |
| `webhooks::cert_pem_to_public_key(pem)`               | Parse a PEM-encoded X.509 cert into a `RsaPublicKey`.        |
| `webhooks::confirm_subscription(envelope, url)`       | POST to a `SubscribeURL` to confirm a new subscription.      |
| `WebhookEvent::trx_id()` / `as_code()` / `is_*()`     | Typed accessors on a parsed event.                           |

## Error handling

Every crate method returns `Result<T, bkash_rs::Error>`. The
[`Error`](https://docs.rs/bkash-rs/latest/bkash_rs/enum.Error.html)
enum covers:

- `Error::Auth(String)` — invalid app key/secret/username/password
  (bKash `errorCode` 2001–2004 etc).
- `Error::Api { code, message, http_status }` — typed bKash error
  envelope; `code` is a [`ErrorCode`] (auto-mapped from bKash's
  `statusCode` / `errorCode` strings).
- `Error::Transport(reqwest::Error)` — network / TLS failure.
- `Error::Timeout(Duration)` — request exceeded configured timeout.
- `Error::Config(String)` — invalid `Config` (missing credentials,
  invalid URL, etc).
- `Error::Json { kind, path }` — response body failed to deserialize
  (the raw payload is preserved).

[`Error::is_transient`](https://docs.rs/bkash-rs/latest/bkash_rs/enum.Error.html#method.is_transient)
and
[`Error::is_auth`](https://docs.rs/bkash-rs/latest/bkash_rs/enum.Error.html#method.is_auth)
help with retry / re-auth decisions. Transient errors are retried
automatically by the transport with exponential backoff (configurable
via `ConfigBuilder::max_retries`).

## Examples

The `examples/` directory ships two runnable smoke tests:

- **`quickstart.rs`** — hard-coded credentials, drives a Tokenized
  Checkout payment from grant → create → query → (optional) refund.

  ```bash
  # Edit the constants at the top of the file first.
  cargo run --example quickstart
  ```

- **`from_env.rs`** — staged sandbox walk-through that reads
  credentials from a `.env` (see `set -a; source .env; set +a`).
  Walks the customer through wallet approval on the phone between
  steps. Covers Tokenized Checkout, Subscriptions, and the classic
  URL-based Checkout product.

  ```bash
  set -a; source .env; set +a

  cargo run --example from_env -- create-agreement     # opens bkashURL
  # approve on phone with PIN 12121 / OTP 123456 ...
  cargo run --example from_env -- execute-agreement --payment-id <ID>
  cargo run --example from_env -- create-payment    --agreement-id <AG>
  cargo run --example from_env -- execute-payment   --payment-id <ID>
  cargo run --example from_env -- query-payment     --payment-id <ID>
  cargo run --example from_env -- refund            --payment-id <ID> --trx-id <TRX> --amount 100.00
  ```

  Subscriptions and classic Checkout stages require their feature
  flags (`--features subscriptions`, `--features checkout`); the
  example prints a `help` listing when invoked with no subcommand.

## Configuration

[`ConfigBuilder`](https://docs.rs/bkash-rs/latest/bkash_rs/struct.ConfigBuilder.html)
covers everything. All four credentials (`app_key`, `app_secret`,
`username`, `password`) are required.

```rust
use std::time::Duration;
use bkash_rs::Environment;

let config = Bkash::builder()
    .environment(Environment::Sandbox)
    .app_key("...").app_secret("...")
    .username("...").password("...")
    .timeout(Duration::from_secs(30))   // default: 30s
    .max_retries(3)                     // default: 3
    .build()?;                          // returns Result<Config, Error>
```

For advanced setups (custom `reqwest::Client`, proxy, base-URL
override for a local bKash mock) use `with_http_client(...)` or
`with_base_url(...)`.

## Project layout

```
src/
├── lib.rs              # crate-level docs + module declarations
├── client.rs           # `Bkash` handle (cheap-clone Arc<Inner>)
├── config.rs           # Config, ConfigBuilder, Environment, Product
├── error.rs            # Error, ErrorCode
├── token.rs            # TokenCache + CachedToken
├── transport.rs        # Transport (reqwest + retry + auth)
├── prelude.rs          # `use bkash_rs::prelude::*;`
├── checkout.rs         # `bkash.checkout()` accessor
├── tokenized.rs        # `bkash.tokenized()` accessor
├── auth_capture.rs     # `bkash.auth_capture()` accessor
├── subscriptions.rs    # `bkash.subscriptions()` accessor
├── webhooks.rs         # `bkash_rs::webhooks::*` helpers
└── models/             # request/response types, one submod per product

tests/                  # wiremock integration tests + fixtures + #[ignore]'d live sandbox smoke tests
examples/               # quickstart + from_env staged driver
```

## MSRV

The minimum supported Rust version is **1.75**. Bumping the MSRV is a
breaking change and will require a minor version bump.

## Roadmap

`bkash-rs` is on the `0.1.x` track. The 0.2 release will:

- Cut the first **crates.io publish**.
- Lock the public API surface (semver starts mattering for real).
- Drop any deprecation warnings introduced during 0.1.x.

Until then, the API is permitted to evolve. See
[`CHANGELOG.md`](CHANGELOG.md) for the current state, including all
breaking-change notes between 0.1.x releases.

## Contributing

Contributions welcome — please read [`CONTRIBUTING.md`](CONTRIBUTING.md)
first. It covers building from source, the test layout, fixture
recording, and the opt-in live-sandbox smoke tests.

## Security

Report security issues privately — see [`SECURITY.md`](SECURITY.md).
**Do not** file public GitHub issues for vulnerabilities.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual-licensed as above, without any
additional terms or conditions.
