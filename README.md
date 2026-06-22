# bkash-rs

[![CI](https://github.com/dibakarsutradhar/bkash-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/dibakarsutradhar/bkash-rs/actions/workflows/ci.yml)
[![docs.rs](https://docs.rs/bkash-rs/badge.svg)](https://docs.rs/bkash-rs)
[![crates.io](https://img.shields.io/crates/v/bkash-rs.svg)](https://crates.io/crates/bkash-rs)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Idiomatic async-first Rust client for the bKash Payment Gateway API. `bkash-rs`
provides a strongly-typed, ergonomic interface to the bKash Payment Gateway,
supporting tokenized checkout, classic checkout, authorization & capture,
subscriptions, and webhook verification.

> **Status:** Pre-1.0 development. APIs may change before the `0.2.0` release.
> Not yet published to crates.io.

## Quickstart

> *Coming soon — see the design spec and Phase 2/3 issues for what is being
> built first.*

```rust
// Placeholder example; will be expanded in Phase 3.
use bkash_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), bkash_rs::Error> {
    let _client = Client::builder()
        .environment(Environment::Sandbox)
        // .credentials(...)
        .build()
        .await?;
    Ok(())
}
```

## Cargo Features

| Feature            | Default | Description                                              |
| ------------------ | :-----: | -------------------------------------------------------- |
| `rustls-tls`       |   yes   | Use `rustls` for TLS (pure Rust).                        |
| `native-tls`       |    -    | Use the platform's native TLS implementation.            |
| `tokenized-checkout` | yes   | Tokenized checkout endpoints.                            |
| `checkout`         |   yes   | Classic (URL-based) checkout endpoints.                  |
| `auth-capture`     |    -    | Authorization & capture endpoints.                       |
| `subscriptions`    |    -    | Subscription endpoints.                                  |
| `webhooks`         |    -    | SNS-style webhook signature verification.                |

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at
your option.
