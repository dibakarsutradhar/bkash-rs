//! Common types shared across model modules.

use serde::{Deserialize, Serialize};

/// Money amount as a raw string, preserving bKash's loose format
/// (e.g. `"15"`, `"100.00"`, `"1234.5"`). Use [`Money::as_str`] to access
/// the raw value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Money(String);

impl Money {
    /// Wrap an arbitrary string as a `Money` value.
    #[must_use]
    pub fn new(amount: impl Into<String>) -> Self {
        Self(amount.into())
    }

    /// Convenience constructor for BDT amounts. Does not validate or
    /// normalise the input.
    #[must_use]
    pub fn bdt(amount: &str) -> Self {
        Self(amount.to_string())
    }

    /// Return the raw underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Money {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Money {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for Money {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Currency code. bKash primarily supports BDT.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    /// Bangladeshi Taka.
    #[default]
    #[serde(rename = "BDT")]
    Bdt,
}

/// Payment intent.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// Sale: authorize and capture in one step.
    #[default]
    Sale,
    /// Authorization: authorize only, capture later (Auth & Capture only).
    Authorization,
}

/// Transaction status as returned by bKash search-transaction endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TransactionStatus {
    /// Transaction has been initiated but not yet completed.
    Initiated,
    /// Transaction completed successfully.
    Completed,
    /// Transaction is in pending-authorized state.
    PendingAuthorized,
    /// Transaction has expired.
    Expired,
    /// Transaction was cancelled.
    Cancelled,
    /// Transaction was declined.
    Declined,
}

/// Payer type.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayerType {
    /// Customer's bKash account.
    #[default]
    Customer,
    /// Merchant's bKash account.
    Merchant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_round_trip_with_decimals() {
        let m = Money::bdt("100.00");
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, "\"100.00\"");
        let back: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn money_round_trip_without_decimals() {
        let m = Money::bdt("15");
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, "\"15\"");
        let back: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn money_round_trip_with_one_decimal() {
        let m = Money::bdt("1234.5");
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, "\"1234.5\"");
        let back: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn money_display_and_as_str() {
        let m = Money::bdt("42");
        assert_eq!(m.as_str(), "42");
        assert_eq!(m.to_string(), "42");
    }

    #[test]
    fn money_from_conversions() {
        let m: Money = "10".into();
        assert_eq!(m.as_str(), "10");
        let m: Money = String::from("20").into();
        assert_eq!(m.as_str(), "20");
    }

    #[test]
    fn currency_serialises_as_bdt() {
        let j = serde_json::to_string(&Currency::Bdt).unwrap();
        assert_eq!(j, "\"BDT\"");
    }

    #[test]
    fn intent_default_is_sale() {
        assert_eq!(Intent::default(), Intent::Sale);
    }

    #[test]
    fn intent_serialisation() {
        assert_eq!(serde_json::to_string(&Intent::Sale).unwrap(), "\"sale\"");
        assert_eq!(
            serde_json::to_string(&Intent::Authorization).unwrap(),
            "\"authorization\""
        );
    }

    #[test]
    fn transaction_status_serialisation() {
        let j = serde_json::to_string(&TransactionStatus::PendingAuthorized).unwrap();
        assert_eq!(j, "\"PendingAuthorized\"");
    }
}
