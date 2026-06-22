# Contributing to bkash-rs

Thanks for your interest in contributing! This document covers the day-to-day
workflow for hacking on `bkash-rs`.

## Code of conduct

Be kind, be respectful. Assume good faith. We follow the
[Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## Prerequisites

- Rust **1.75** or newer (`rustup toolchain install stable` is fine for
  day-to-day work; CI runs on stable).
- `cargo`, `rustfmt`, and `clippy` (installed by default with Rust).

## How to run tests

```bash
# Build everything (all features)
cargo build --all-features

# Run the test suite (all features)
cargo test --all-features

# Format check
cargo fmt --all -- --check

# Lints (deny warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Docs (deny warnings, no deps)
cargo doc --no-deps -- -D warnings
```

`cargo test --all-features` runs the unit test suite. Integration tests use
[`wiremock`](https://crates.io/crates/wiremock) to stub the bKash endpoints —
no network access is required for the default suite.

## How to record a fixture

Recorded fixtures (raw JSON request/response pairs) live in
`tests/fixtures/`. To add or update a fixture:

1. Drop the raw JSON body into the appropriate subdirectory, e.g.
   `tests/fixtures/tokenized/create_agreement_success.json`.
2. Add a matching entry in `tests/fixtures/manifest.json` describing the
   endpoint, HTTP method, status, and the file name.
3. Reference the fixture from your test via the manifest helper.

> Fixture support is being added in Phase 8 (Testing hardening). For now,
> keep stubs inline in test files or under `tests/stubs/`.

## How to run live sandbox tests

> **Warning:** Live sandbox tests hit the real bKash sandbox API and require
> valid credentials. They are *not* run in CI.

1. Register for a bKash sandbox account and obtain:
   - `BKASH_SANDBOX_USERNAME`
   - `BKASH_SANDBOX_PASSWORD`
   - `BKASH_SANDBOX_APP_KEY`
   - `BKASH_SANDBOX_APP_SECRET`
2. Export them in your shell, e.g. via a local `.env` (already git-ignored).
3. Run the integration tests with the `live` feature:
   ```bash
   cargo test --features live -- --ignored
   ```

Treat any data returned by the sandbox as non-production. Never commit
credentials.

## Pull requests

- Branch from `main`.
- One logical change per PR.
- Update the `[Unreleased]` section of `CHANGELOG.md` under "Added" /
  "Changed" / "Fixed" / "Removed".
- Run the full local checklist above before opening the PR.
- Fill in the PR template.

## Reporting vulnerabilities

See [SECURITY.md](SECURITY.md).
