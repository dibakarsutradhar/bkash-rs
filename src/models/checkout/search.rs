//! Checkout (URL-based): Search-transaction types.

use serde::{Deserialize, Serialize};

use crate::models::common::{Currency, Money, PayerType, TransactionStatus};

/// Request body for the search-transaction endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SearchTransactionRequest {
    /// bKash transaction ID (`trxID`).
    #[serde(rename = "trxID")]
    pub trx_id: String,
}

impl SearchTransactionRequest {
    /// Construct a new search-transaction request.
    #[must_use]
    pub fn new(trx_id: impl Into<String>) -> Self {
        Self {
            trx_id: trx_id.into(),
        }
    }
}

/// Response from the search-transaction endpoint.
///
/// The shape includes base transaction fields plus an optional set of
/// coupon-specific fields, all flattened in the bKash body. Use
/// [`SearchTransactionResponse::is_coupon`] to detect coupon transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SearchTransactionResponse {
    /// bKash transaction ID.
    #[serde(rename = "trxID")]
    pub trx_id: String,

    /// Transaction status.
    #[serde(rename = "transactionStatus", default)]
    pub transaction_status: TransactionStatus,

    /// Transaction type (e.g. `"Payment"`, `"Refund"`).
    #[serde(rename = "transactionType", default)]
    pub transaction_type: String,

    /// Echoed amount.
    #[serde(default)]
    pub amount: Money,

    /// Echoed currency.
    #[serde(default)]
    pub currency: Currency,

    /// Customer MSISDN.
    #[serde(rename = "customerMsisdn", default)]
    pub customer_msisdn: String,

    /// Organization short code.
    #[serde(rename = "organizationShortCode", default)]
    pub organization_short_code: String,

    /// Initiation timestamp (ISO-8601).
    #[serde(rename = "initiationTime", default)]
    pub initiation_time: String,

    /// Completion timestamp (ISO-8601).
    #[serde(rename = "completedTime", default)]
    pub completed_time: String,

    /// Payer type.
    #[serde(rename = "payerType", default)]
    pub payer_type: PayerType,

    /// Maximum amount that can currently be refunded.
    #[serde(rename = "maxRefundableAmount", default)]
    pub max_refundable_amount: Money,

    /// Sale amount (regular transactions only).
    #[serde(rename = "saleAmount", default)]
    pub sale_amount: Money,

    /// Service fee charged.
    #[serde(rename = "serviceFee", default)]
    pub service_fee: Money,

    /// Payer account identifier.
    #[serde(rename = "payerAccount", default)]
    pub payer_account: String,

    // --- coupon-specific (optional) ---
    /// Coupon amount (coupon transactions only).
    #[serde(rename = "couponAmount", default)]
    pub coupon_amount: Money,

    /// Merchant share amount (coupon transactions only).
    #[serde(rename = "merchantShareAmount", default)]
    pub merchant_share_amount: Money,

    /// Credited amount (coupon transactions only).
    #[serde(rename = "creditedAmount", default)]
    pub credited_amount: Money,
}

impl SearchTransactionResponse {
    /// Returns `true` if this response has coupon-specific fields populated
    /// (i.e. it represents a coupon transaction).
    #[must_use]
    pub fn is_coupon(&self) -> bool {
        !self.coupon_amount.as_str().is_empty()
            || !self.merchant_share_amount.as_str().is_empty()
            || !self.credited_amount.as_str().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{Currency, PayerType, TransactionStatus};

    #[test]
    fn search_request_serialises() {
        let req = SearchTransactionRequest::new("8A00ABCD");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["trxID"], "8A00ABCD");
    }

    #[test]
    fn search_response_round_trip_regular_transaction() {
        let body = r#"{
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed",
            "transactionType": "Payment",
            "amount": "100.00",
            "currency": "BDT",
            "customerMsisdn": "01700000000",
            "organizationShortCode": "0123",
            "initiationTime": "2026-06-22T09:00:00:000 GMT+06:00",
            "completedTime": "2026-06-22T09:01:00:000 GMT+06:00",
            "payerType": "Customer",
            "maxRefundableAmount": "100.00",
            "saleAmount": "100.00",
            "serviceFee": "0.00",
            "payerAccount": "123456789"
        }"#;
        let resp: SearchTransactionResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.trx_id, "8A00ABCD");
        assert_eq!(resp.transaction_status, TransactionStatus::Completed);
        assert_eq!(resp.currency, Currency::Bdt);
        assert_eq!(resp.payer_type, PayerType::Customer);
        assert_eq!(resp.amount.as_str(), "100.00");
        assert!(!resp.is_coupon());

        // Round-trip via serialize-then-deserialize.
        let json = serde_json::to_string(&resp).unwrap();
        let back: SearchTransactionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.trx_id, resp.trx_id);
        assert_eq!(back.transaction_status, resp.transaction_status);
        assert_eq!(back.amount.as_str(), resp.amount.as_str());
    }

    #[test]
    fn search_response_parses_coupon_transaction() {
        let body = r#"{
            "trxID": "8A00EFGH",
            "transactionStatus": "Completed",
            "transactionType": "Payment",
            "amount": "100.00",
            "currency": "BDT",
            "customerMsisdn": "01700000000",
            "organizationShortCode": "0123",
            "initiationTime": "2026-06-22T09:00:00:000 GMT+06:00",
            "completedTime": "2026-06-22T09:01:00:000 GMT+06:00",
            "payerType": "Customer",
            "maxRefundableAmount": "100.00",
            "saleAmount": "90.00",
            "serviceFee": "0.00",
            "payerAccount": "123456789",
            "couponAmount": "10.00",
            "merchantShareAmount": "85.00",
            "creditedAmount": "95.00"
        }"#;
        let resp: SearchTransactionResponse = serde_json::from_str(body).unwrap();
        assert!(resp.is_coupon());
        assert_eq!(resp.coupon_amount.as_str(), "10.00");
        assert_eq!(resp.merchant_share_amount.as_str(), "85.00");
        assert_eq!(resp.credited_amount.as_str(), "95.00");
    }
}
