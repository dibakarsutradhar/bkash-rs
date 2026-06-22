# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial crate scaffolding (`bkash-rs`).
- Module skeleton: `client`, `config`, `error`, `token`, `transport`, `webhooks`,
  `models` (with `common`, `token`, `checkout`, `tokenized`, `auth_capture`,
  `subscriptions`), and a `prelude` re-export module.
- Cargo features: `rustls-tls` (default), `native-tls`,
  `tokenized-checkout` (default), `checkout` (default), `auth-capture`,
  `subscriptions`, `webhooks`.
- Dual licensing under MIT OR Apache-2.0.
