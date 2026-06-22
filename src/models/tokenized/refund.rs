//! Tokenized Checkout: Refund and refund-status types.
//!
//! **Note:** the refund endpoint uses a different error-envelope shape than
//! the rest of the bKash API. Failure bodies carry `errorCode`,
//! `errorMessageEn`, and `errorMessageBn` (Bangla) fields. The crate's
//! standard [`Error`](crate::Error) decoder currently surfaces only the
//! English message; a Phase-8 enhancement will map the Bangla message too.

use serde::{Deserialize, Serialize};

use crate::models::common::Money;

/// Request body for refunding a captured payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RefundRequest {
    /// The original `paymentID`.
    #[serde(rename = "paymentId")]
    pub payment_id: String,

    /// The bKash transaction ID (`trxID`).
    #[serde(rename = "trxId")]
    pub trx_id: String,

    /// Amount to refund. Must be ≤ the original amount and ≤
    /// `maxRefundableAmount`. Up to 2 decimals.
    #[serde(rename = "refundAmount")]
    pub refund_amount: Money,

    /// Stock-keeping unit or merchant-defined identifier (max 255 chars).
    pub sku: String,

    /// Human-readable reason (max 255 chars).
    pub reason: String,
}

impl RefundRequest {
    /// Construct a new refund request.
    ///
    /// `sku` and `reason` are stored verbatim; the caller is responsible for
    /// ensuring each is ≤ 255 characters (bKash-side limit).
    #[must_use]
    pub fn new(
        payment_id: impl Into<String>,
        trx_id: impl Into<String>,
        refund_amount: Money,
        sku: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            payment_id: payment_id.into(),
            trx_id: trx_id.into(),
            refund_amount,
            sku: sku.into(),
            reason: reason.into(),
        }
    }
}

/// Response from the refund endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RefundResponse {
    /// Original `paymentID`.
    #[serde(rename = "paymentID", default)]
    pub payment_id: String,

    /// Original `trxID`.
    #[serde(rename = "trxID", default)]
    pub trx_id: String,

    /// New refund transaction ID (`trxID`) — different from the original.
    #[serde(rename = "refundTrxID", default)]
    pub refund_trx_id: String,

    /// Refunded amount.
    #[serde(rename = "refundAmount", default)]
    pub refund_amount: Money,

    /// Echoed SKU.
    #[serde(default)]
    pub sku: String,

    /// Echoed reason.
    #[serde(default)]
    pub reason: String,

    /// Remaining amount that can be refunded.
    #[serde(rename = "maxRefundableAmount", default)]
    pub max_refundable_amount: Money,

    /// Echoed `organizationShortCode`.
    #[serde(rename = "organizationShortCode", default)]
    pub organization_short_code: String,

    /// Refund transaction status (e.g. `"Completed"`).
    #[serde(rename = "transactionStatus", default)]
    pub transaction_status: String,

    /// Refund initiation timestamp (ISO-8601).
    #[serde(rename = "initiationTime", default)]
    pub initiation_time: String,

    /// Refund completion timestamp (ISO-8601).
    #[serde(rename = "completedTime", default)]
    pub completed_time: String,
}

/// Request body for the refund-status endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RefundStatusRequest {
    /// Original `paymentID`.
    #[serde(rename = "paymentID")]
    pub payment_id: String,

    /// Original `trxID`.
    #[serde(rename = "trxID")]
    pub trx_id: String,
}

impl RefundStatusRequest {
    /// Construct a new refund-status request.
    #[must_use]
    pub fn new(payment_id: impl Into<String>, trx_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            trx_id: trx_id.into(),
        }
    }
}

/// Response from the refund-status endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RefundStatusResponse {
    /// Original `paymentID`.
    #[serde(rename = "paymentID", default)]
    pub payment_id: String,

    /// Original `trxID`.
    #[serde(rename = "trxID", default)]
    pub trx_id: String,

    /// Refund transaction ID.
    #[serde(rename = "refundTrxID", default)]
    pub refund_trx_id: String,

    /// Refunded amount.
    #[serde(rename = "refundAmount", default)]
    pub refund_amount: Money,

    /// Echoed SKU.
    #[serde(default)]
    pub sku: String,

    /// Echoed reason.
    #[serde(default)]
    pub reason: String,

    /// Remaining amount that can still be refunded.
    #[serde(rename = "maxRefundableAmount", default)]
    pub max_refundable_amount: Money,

    /// Refund transaction status (e.g. `"Completed"`).
    #[serde(rename = "transactionStatus", default)]
    pub transaction_status: String,

    /// Refund initiation timestamp (ISO-8601).
    #[serde(rename = "initiationTime", default)]
    pub initiation_time: String,

    /// Refund completion timestamp (ISO-8601).
    #[serde(rename = "completedTime", default)]
    pub completed_time: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refund_request_serialises() {
        let req = RefundRequest::new(
            "TR0001",
            "8A00ABCD",
            Money::bdt("25.00"),
            "sku-1",
            "customer-return",
        );
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["paymentId"], "TR0001");
        assert_eq!(json["trxId"], "8A00ABCD");
        assert_eq!(json["refundAmount"], "25.00");
        assert_eq!(json["sku"], "sku-1");
        assert_eq!(json["reason"], "customer-return");
    }

    #[test]
    fn refund_response_parses_minimal() {
        let body = r#"{
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "refundTrxID": "8B00EFGH",
            "refundAmount": "25.00",
            "sku": "sku-1",
            "reason": "test",
            "maxRefundableAmount": "75.00",
            "transactionStatus": "Completed",
            "initiationTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "completedTime": "2026-06-22T10:01:00:000 GMT+06:00"
        }"#;
        let resp: RefundResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.refund_trx_id, "8B00EFGH");
        assert_eq!(resp.refund_amount.as_str(), "25.00");
        assert_eq!(resp.max_refundable_amount.as_str(), "75.00");
    }

    #[test]
    fn refund_status_request_serialises() {
        let req = RefundStatusRequest::new("TR0001", "8A00ABCD");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["paymentID"], "TR0001");
        assert_eq!(json["trxID"], "8A00ABCD");
    }

    // ---- proptest round-trips -----------------------------------------

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn refund_request_roundtrip(
            payment_id in ".*",
            trx_id in ".*",
            amount in ".*",
            sku in ".*",
            reason in ".*",
        ) {
            let req = RefundRequest::new(
                payment_id,
                trx_id,
                Money::new(amount),
                sku,
                reason,
            );
            let json = serde_json::to_string(&req).unwrap();
            let back: RefundRequest = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&back).unwrap();
            prop_assert_eq!(json, json2);
        }
    }
}
