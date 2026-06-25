# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Nothing yet.

## [0.2.1] — 2026-06-26

Patch release. No public API changes; only the docs.rs build is
fixed so the published crate renders documentation correctly.

### Fixed

- docs.rs build: removed `#![cfg_attr(docsrs, feature(doc_cfg))]`
  and the `#[cfg_attr(docsrs, doc(cfg(...)))]` annotations on
  feature-gated modules and accessor methods. `doc(cfg(...))` and
  `feature(doc_cfg)` are nightly-only, so docs.rs (stable Rust) was
  crashing on the docs build and rendering an empty page.
- `Cargo.toml`: dropped `rustdoc-args = ["--cfg", "docsrs", "-D",
  "warnings"]` from `[package.metadata.docs.rs]`. docs.rs sets
  `--cfg docsrs` automatically; the explicit injection activated
  the nightly feature gate.

## [0.2.0] — 2026-06-26

First **published** release to crates.io. Builds on the `0.1.0`
preview tag with documentation, CI, and packaging fixes required for
publication.

### Added

- Initial crates.io publication as `bkash-rs` 0.2.0.

### Changed

- Documentation: full README rewrite with quickstart, feature flags,
  sandbox vs production guidance, and API reference.
- `CONTRIBUTING.md` expanded with build / test / fixture / live-test
  sections and an endpoint recipe.
- `Cargo.toml` author email corrected; examples included in the
  published package.
- CI workflow passes `--all-features` to `cargo doc`.

### Fixed

- License detection on GitHub (added explicit `LICENSE` pointer and
  adjusted `LICENSE-APACHE` / `LICENSE-MIT`).
- `auth_capture` and `tokenized-checkout` refund paths aligned with
  bKash v1.2.0-beta docs.
- `transport`: username/password headers now sent on every request.

## [0.1.0] — 2026-06-22

First tagged preview. **Not published to crates.io** — superseded by
the `0.2.0` release for the first crates.io publication.

### Added

- Initial crate: `bkash-rs` — idiomatic async-first Rust client for the
  bKash Payment Gateway API.
- High-level [`Bkash`] client (cheap-clone `Arc<Inner>` handle) backed
  by a transparent token cache and a single shared [`Transport`].
- Configurable [`Config`] via [`ConfigBuilder`] with required
  `app_key` / `app_secret` / `username` / `password` and an
  [`Environment`] selector (`Sandbox` / `Production`).
- Tokenized Checkout product (feature `tokenized-checkout`, enabled by
  default): `grant_token`, `refresh_token`, `create_agreement`,
  `execute_agreement`, `query_agreement`, `cancel_agreement`,
  `create_payment`, `execute_payment`, `query_payment`,
  `search_transaction`, `refund`, `refund_status`.
- URL-based Checkout product (feature `checkout`, enabled by default):
  `grant_token`, `refresh_token`, `create_payment`, `execute_payment`,
  `query_payment`, `search_transaction`, `refund`, `refund_status`.
- Authorization & Capture product (feature `auth-capture`):
  `grant_token`, `refresh_token`, `create_payment`, `execute_payment`,
  `query_payment`, `capture`, `void`, `search_transaction`.
- Subscriptions product (feature `subscriptions`):
  `create_subscription`, `execute_subscription`, `query_subscription`,
  `cancel_subscription` (typed façade over the tokenized agreement
  endpoints, which is how bKash hosts Subscriptions).
- Webhooks (feature `webhooks`): `verify_sns_signature`,
  `verify_signature_with_key`, `parse_event`, `build_string_to_sign`,
  `cert_pem_to_public_key`, `confirm_subscription`, and a typed
  `WebhookEvent` with `trx_id()`, `as_code()`, `from_code()`,
  `is_subscription_confirmation()`, `is_notification()`, and
  `is_unsubscribe_confirmation()`.
- Transport-level reliability:
  - Transparent `id_token` caching and re-grant on `401`.
  - Idempotent retries on transient failures (`5xx`, network errors,
    timeout) with exponential backoff.
  - Per-product base URL selection.
- Module skeleton: `client`, `config`, `error`, `token`, `transport`,
  `webhooks`, `models` (with `common`, `token`, `checkout`, `tokenized`,
  `auth_capture`, `subscriptions`), and a `prelude` re-export module.
- Cargo features: `rustls-tls` (default), `native-tls`,
  `tokenized-checkout` (default), `checkout` (default), `auth-capture`,
  `subscriptions`, `webhooks`.
- Comprehensive offline test suite — **220 tests passing** —
  combining unit tests, [`wiremock`](https://crates.io/crates/wiremock)
  integration tests, sanitized JSON fixtures
  (`tests/common/fixtures/`), and `proptest` round-trip property tests.
- Opt-in live sandbox smoke tests (`tests/live/sandbox.rs`,
  `#[ignore]`-gated) for contributors with a bKash sandbox account.
- `clippy`, `rustfmt`, and `cargo doc` configured and CI-enforced.
- Dual licensing under MIT OR Apache-2.0.
- `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, and `SECURITY.md`.

### Notes

- The full bKash endpoint → crate-method index lives in the
  [API Reference section of README.md](README.md#api-reference).
- MSRV: **1.75**.