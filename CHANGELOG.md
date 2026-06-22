# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Subscriptions product accessor (`bkash::subscriptions()`) behind the
  `subscriptions` cargo feature. Exposes
  `create_subscription` / `execute_subscription` / `query_subscription` /
  `cancel_subscription` (thin wrapper over the tokenized-checkout
  agreement endpoints, which is how bKash's subscriptions product is
  hosted).
- Initial crate scaffolding (`bkash-rs`).
- Module skeleton: `client`, `config`, `error`, `token`, `transport`, `webhooks`,
  `models` (with `common`, `token`, `checkout`, `tokenized`, `auth_capture`,
  `subscriptions`), and a `prelude` re-export module.
- Cargo features: `rustls-tls` (default), `native-tls`,
  `tokenized-checkout` (default), `checkout` (default), `auth-capture`,
  `subscriptions`, `webhooks`.
- Dual licensing under MIT OR Apache-2.0.
