//! Common types shared across model modules.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Money amount as a raw string, preserving bKash's loose format
/// (e.g. `"15"`, `"100.00"`, `"1234.5"`). Use [`Money::as_str`] to access
/// the raw value.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
#[non_exhaustive]
pub enum Currency {
    /// Bangladeshi Taka.
    #[default]
    #[serde(rename = "BDT")]
    Bdt,
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Bdt => "BDT",
        })
    }
}

impl FromStr for Currency {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BDT" => Ok(Self::Bdt),
            _ => Err(()),
        }
    }
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

impl fmt::Display for Intent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Sale => "sale",
            Self::Authorization => "authorization",
        })
    }
}

impl FromStr for Intent {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sale" => Ok(Self::Sale),
            "authorization" => Ok(Self::Authorization),
            _ => Err(()),
        }
    }
}

/// Transaction status as returned by bKash search-transaction endpoints.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TransactionStatus {
    /// Transaction has been initiated but not yet completed.
    #[default]
    Initiated,
    /// Transaction completed successfully.
    Completed,
    /// Transaction is in pending-authorized state.
    PendingAuthorized,
    /// Transaction has been authorized (Auth & Capture flow).
    ///
    /// This is the auth-capture-specific name for the state where the
    /// customer has approved the wallet charge and the funds are reserved
    /// but not yet captured. Some bKash documents refer to the same state
    /// as [`TransactionStatus::PendingAuthorized`]; both are modeled
    /// explicitly per the plan (§1.8 / §1.9).
    Authorized,
    /// Transaction has expired.
    Expired,
    /// Transaction was cancelled.
    Cancelled,
    /// Transaction was declined.
    Declined,
}

impl fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Initiated => "Initiated",
            Self::Completed => "Completed",
            Self::PendingAuthorized => "PendingAuthorized",
            Self::Authorized => "Authorized",
            Self::Expired => "Expired",
            Self::Cancelled => "Cancelled",
            Self::Declined => "Declined",
        })
    }
}

impl FromStr for TransactionStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Initiated" => Ok(Self::Initiated),
            "Completed" => Ok(Self::Completed),
            "PendingAuthorized" => Ok(Self::PendingAuthorized),
            "Authorized" => Ok(Self::Authorized),
            "Expired" => Ok(Self::Expired),
            "Cancelled" => Ok(Self::Cancelled),
            "Declined" => Ok(Self::Declined),
            _ => Err(()),
        }
    }
}

/// Payer type. bKash wire format: `"Customer"` | `"Merchant"`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PayerType {
    /// Customer's bKash account.
    #[default]
    #[serde(rename = "Customer")]
    Customer,
    /// Merchant's bKash account.
    #[serde(rename = "Merchant")]
    Merchant,
}

impl fmt::Display for PayerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Customer => "Customer",
            Self::Merchant => "Merchant",
        })
    }
}

impl FromStr for PayerType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Customer" => Ok(Self::Customer),
            "Merchant" => Ok(Self::Merchant),
            _ => Err(()),
        }
    }
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
        let j = serde_json::to_string(&TransactionStatus::Authorized).unwrap();
        assert_eq!(j, "\"Authorized\"");
    }

    #[test]
    fn intent_from_str_parses_known() {
        assert_eq!("sale".parse::<Intent>().unwrap(), Intent::Sale);
        assert_eq!(
            "authorization".parse::<Intent>().unwrap(),
            Intent::Authorization
        );
        assert!("unknown".parse::<Intent>().is_err());
    }

    #[test]
    fn transaction_status_from_str_parses_all_six() {
        assert_eq!(
            "Initiated".parse::<TransactionStatus>().unwrap(),
            TransactionStatus::Initiated
        );
        assert_eq!(
            "Completed".parse::<TransactionStatus>().unwrap(),
            TransactionStatus::Completed
        );
        assert_eq!(
            "PendingAuthorized".parse::<TransactionStatus>().unwrap(),
            TransactionStatus::PendingAuthorized
        );
        assert_eq!(
            "Authorized".parse::<TransactionStatus>().unwrap(),
            TransactionStatus::Authorized
        );
        assert_eq!(
            "Expired".parse::<TransactionStatus>().unwrap(),
            TransactionStatus::Expired
        );
        assert_eq!(
            "Cancelled".parse::<TransactionStatus>().unwrap(),
            TransactionStatus::Cancelled
        );
        assert_eq!(
            "Declined".parse::<TransactionStatus>().unwrap(),
            TransactionStatus::Declined
        );
        assert!("Foo".parse::<TransactionStatus>().is_err());
    }

    #[test]
    fn payer_type_display_and_from_str() {
        assert_eq!(PayerType::Customer.to_string(), "Customer");
        assert_eq!(PayerType::Merchant.to_string(), "Merchant");
        assert_eq!(
            "Customer".parse::<PayerType>().unwrap(),
            PayerType::Customer
        );
    }

    // ---- proptest round-trips -----------------------------------------

    use proptest::prelude::*;

    proptest! {
        // `Money` is a transparent newtype around `String`; any printable
        // string should round-trip cleanly through serde. We restrict to
        // printable chars because bKash's amount strings are ASCII digits
        // and dots in practice, and full-utf-8 control chars don't survive
        // a naive string concat (serde_json escapes them).
        #[test]
        fn money_round_trip_arbitrary(s in "[0-9.]{0,32}") {
            let m = Money::new(s.clone());
            let json = serde_json::to_string(&m).unwrap();
            // Transparent: serialises as a plain JSON string.
            prop_assert_eq!(json.clone(), format!("\"{}\"", s));
            let back: Money = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, m);
        }
    }
}
