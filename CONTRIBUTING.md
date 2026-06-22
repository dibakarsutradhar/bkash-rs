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

Sanitized response fixtures live in `tests/common/fixtures/` as raw JSON
files. They are loaded by tests via the
[`Fixture::load`](tests/common/mod.rs) helper:

```rust
use common::Fixture;

let body = Fixture::load("tokenized_create_agreement.json");
```

To add or update a fixture:

1. Drop a sanitized JSON body into `tests/common/fixtures/`, e.g.
   `tokenized_create_agreement.json`. The file name should follow the
   `{product}_{operation}.json` convention.
2. Ensure the file is **valid JSON**, uses `statusCode: "0000"` and
   `statusMessage: "Successful"` for success responses, and uses
   sanitized IDs (e.g. `trxID: "TESTTRX000001"`). Real tokens, customer
   data, or PII must never be committed.
3. Reference the fixture from your test via `Fixture::load(name)`.

`Fixture::load` returns a `serde_json::Value` that you can pass directly
to `wiremock`'s `set_body_json`. `Fixture::load_string` returns the raw
JSON as a `String`.

## How to run live sandbox tests

> **Warning:** Live sandbox tests hit the real bKash sandbox API and
> require valid credentials. They are *not* run in CI.

1. Register for a bKash sandbox account and obtain:
   - `BKASH_SANDBOX_USERNAME`
   - `BKASH_SANDBOX_PASSWORD`
   - `BKASH_SANDBOX_APP_KEY`
   - `BKASH_SANDBOX_APP_SECRET`
2. Export them in your shell, e.g. via a local `.env` (already
   git-ignored). Optional:
   - `BKASH_SANDBOX_AGREEMENT_ID` — an existing agreement ID, used by the
     `live_tokenized_create_payment` smoke test.
   - `BKASH_SANDBOX_TRX_ID` — a real sandbox transaction ID, used by the
     `live_search_transaction_via_tokenized` smoke test.
3. Run the integration tests with `--ignored` to opt into the live tests:
   ```bash
   cargo test --all-features --test live -- --ignored --nocapture
   ```

The live tests live in `tests/live/sandbox.rs` and are wired in via the
top-level `tests/live.rs`. Each test is marked `#[ignore]` so the
default `cargo test` invocation never hits the network. If the
`BKASH_SANDBOX_*` env vars are missing, each test logs and returns
early (a no-op) rather than failing.

Treat any data returned by the sandbox as non-production. Never commit
credentials.

## CI is fully offline

The CI workflow (`.github/workflows/ci.yml`) runs only the offline test
suite:

- `cargo build --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features` (no `--ignored`)
- `cargo doc --no-deps -- -D warnings`

Live tests are deliberately excluded from CI — they require real
credentials and should only be run locally by contributors who have a
bKash sandbox account.

## Pull requests

- Branch from `main`.
- One logical change per PR.
- Update the `[Unreleased]` section of `CHANGELOG.md` under "Added" /
  "Changed" / "Fixed" / "Removed".
- Run the full local checklist above before opening the PR.
- Fill in the PR template.

## Reporting vulnerabilities

See [SECURITY.md](SECURITY.md).
