//! Shared helpers for integration tests.
//!
//! This module is included as a submodule of each integration test target
//! via `mod common;`. It provides:
//!
//! - [`Fixture::load`] / [`Fixture::load_string`] — read a sanitized JSON
//!   fixture from `tests/common/fixtures/`. The fixtures live alongside
//!   the test code so that they are committed with the rest of the crate
//!   and reviewable in code review.

#![allow(dead_code)]

use std::path::PathBuf;

/// Helper for reading sanitized JSON fixtures from
/// `tests/common/fixtures/`.
pub struct Fixture;

impl Fixture {
    /// Load a sanitized JSON fixture as a [`serde_json::Value`].
    ///
    /// # Panics
    ///
    /// Panics if the file is missing or cannot be parsed as JSON.
    #[must_use]
    pub fn load(name: &str) -> serde_json::Value {
        let path: PathBuf = ["tests", "common", "fixtures", name].iter().collect();
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e));
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("failed to parse fixture {}: {}", path.display(), e))
    }

    /// Load a sanitized JSON fixture as a raw [`String`].
    ///
    /// Useful when a test wants to embed the fixture in a string-based
    /// assertion (e.g. comparing to a generated body).
    ///
    /// # Panics
    ///
    /// Panics if the file is missing.
    #[must_use]
    pub fn load_string(name: &str) -> String {
        let path: PathBuf = ["tests", "common", "fixtures", name].iter().collect();
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e))
    }
}
