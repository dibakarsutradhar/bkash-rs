# Contributing to bkash-rs

Thanks for your interest in contributing! This document covers the day-to-day
workflow for hacking on `bkash-rs`: building, testing, fixture recording,
and the conventions every PR must follow.

## Code of conduct

Be kind, be respectful. Assume good faith. We follow the
[Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## Prerequisites

- Rust **1.75** or newer (`rustup toolchain install stable` is fine for
  day-to-day work; CI runs on stable).
- `cargo`, `rustfmt`, and `clippy` (installed by default with Rust).
- A Unix-like shell for the commands below (Linux, macOS, WSL — all fine).
- *Optional:* a bKash sandbox account, if you want to run the live smoke
  tests described further down.

## Building from source

```bash
# Clone
git clone https://github.com/dibakarsutradhar/bkash-rs
cd bkash-rs

# Build with default features (rustls + tokenized + checkout)
cargo build

# Build every feature (used by CI)
cargo build --all-features
```

## Running tests

### The full offline suite

This is what CI runs and what you should run before opening a PR:

```bash
cargo build --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
```

### By scope

If you only want a slice of the test pyramid:

```bash
# Unit tests only (everything inside `src/`).
cargo test --all-features --lib

# Integration tests only (everything under `tests/`).
cargo test --all-features --tests

# A single integration test file.
cargo test --all-features --test tokenized_checkout
cargo test --all-features --test checkout
cargo test --all-features --test auth_capture
cargo test --all-features --test subscriptions
cargo test --all-features --test webhooks
cargo test --all-features --test transport_wiremock

# Doc tests only (the examples in `///` blocks).
cargo test --all-features --doc

# Property tests (the proptests are tagged `proptest` and live alongside
# unit tests; running `--lib` runs them).
cargo test --all-features --lib proptest

# All tests, excluding the opt-in live-sandbox suite (default — the
# live tests are `#[ignore]`-gated).
cargo test --all-features
```

The default `cargo test` invocation is fully **offline** — `wiremock`
stubs every bKash endpoint and `Fixture::load` reads sanitized JSON
files from `tests/common/fixtures/`. No network access is required.

## Recording new fixtures

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

## Running live sandbox tests

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

## Adding a new endpoint

When bKash ships a new endpoint (or we discover one is missing), follow
this checklist to wire it through the crate end-to-end:

1. **Pick the right product module.** Most endpoints belong to one of
   `src/tokenized.rs`, `src/checkout.rs`, `src/auth_capture.rs`, or
   `src/subscriptions.rs`. If it's truly orthogonal, propose a new
   product module in an issue first.
2. **Add request/response types** under
   `src/models/<product>/<endpoint>.rs` and re-export them from
   `src/models/<product>/mod.rs`. Keep types `#[derive(Debug, Clone,
   Serialize, Deserialize)]` and use `#[serde(rename_all = "camelCase")]`
   to match bKash's wire format.
3. **Implement the method** on the relevant `*Client<'a>`. Pick the
   right `Product` for `Transport::request`, and use `request_path` only
   when the path has interpolated parameters (e.g. `{paymentID}`).
4. **Add a unit test** in the same file, ideally one that exercises the
   error mapping too (a `wiremock` server returning a `4xx` body).
5. **Add a wiremock integration test** under `tests/<product>.rs`.
   Stub the new endpoint, call the method, assert the response shape.
6. **Record a fixture** in `tests/common/fixtures/` (see the section
   above). Run the wiremock test once against the bKash sandbox, save
   the (sanitized) JSON response body, and commit it.
7. **Update the API Reference** table in `README.md` — add the new row
   with HTTP method, endpoint path, and crate method.
8. **Update `CHANGELOG.md`** under `[Unreleased]` → `Added`.

## Bumping dependencies

We pin minor versions with caret semantics in `Cargo.toml`. To bump:

1. Run `cargo update -p <crate>` (or `cargo update` to bump everything).
2. Re-run the full local checklist (`build`, `fmt`, `clippy`, `test`,
   `doc`).
3. If the bump changes public behavior or breaks MSRV, document it in
   `CHANGELOG.md` under `[Unreleased]`.
4. Major-version bumps of any public dependency require their own PR
   and a CHANGELOG entry — they may require a corresponding MSRV bump.

## Code style

`rustfmt` and `clippy` are non-negotiable — CI runs both with `-D
warnings` and PRs that fail either are auto-blocked.

- **Formatting:** `cargo fmt --all` before committing. The repo pins
  defaults via `rustfmt.toml`.
- **Lints:** `cargo clippy --all-targets --all-features -- -D warnings`
  must pass. Avoid `#[allow(...)]` unless you have a clear justification
  in a comment.
- **Docs:** every public item must have a doc comment (`#![deny(missing_docs)]`
  is set in `src/lib.rs`). Run
  `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` to
  catch broken intra-doc links.
- **Tests:** prefer the test pyramid — fast unit tests in the same file
  for each new method, `wiremock`-based integration tests under
  `tests/`, and `proptest` round-trip properties for serializable
  types.
- **Errors:** map bKash's `statusCode` to [`ErrorCode`] and bubble them
  up as [`Error::Api`]. Don't silently swallow errors.
- **No new dependencies** without prior discussion in an issue —
  `reqwest`, `tokio`, `serde`, and friends are already in the tree.

## CI is fully offline

The CI workflow (`.github/workflows/ci.yml`) runs only the offline test
suite:

- `cargo build --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features` (no `--ignored`)
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`

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