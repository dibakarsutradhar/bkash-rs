//! Checkout (URL-based): Payment request and response types.
//!
//! URL-based Checkout is the classic bKash flow that completes with a single
//! customer redirect to bKash's hosted wallet page (no tokenization, no
//! agreement). The flow is:
//!
//! 1. [`CreatePaymentRequest`] (`mode = "0011"`) → [`CreatePaymentResponse`]
//!    returns a `paymentID` and a `bkashURL`.
//! 2. Customer completes the wallet-side approval at `bkashURL`.
//! 3. [`ExecutePaymentRequest`] → [`ExecutePaymentResponse`] to capture the
//!    final state.
//! 4. [`QueryPaymentRequest`] → [`QueryPaymentResponse`] for current status.
//!
//! Note: there is **no agreement step** for URL-based Checkout — use the
//! [`crate::models::tokenized`] module for the tokenized (agreement-based)
//! flow.

use serde::{Deserialize, Serialize};

use crate::models::common::{Currency, Intent, Money};

/// `mode` discriminator value for `POST /tokenized/checkout/create` when
/// creating a **URL-based Checkout payment** (no agreement).
pub const PAYMENT_MODE: &str = "0011";

/// Request body for creating a URL-based checkout payment.
///
/// Unlike [`CreatePaymentRequest`](crate::models::tokenized::CreatePaymentRequest)
/// in the tokenized product, this request **does not** carry an `agreementID` —
/// URL-based checkout is a single, one-shot payment flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CreatePaymentRequest {
    /// Discriminator. Always [`PAYMENT_MODE`] (`"0011"`).
    #[serde(rename = "mode")]
    pub mode: String,

    /// Merchant's reference for the payer (e.g. internal customer ID).
    #[serde(rename = "payerReference")]
    pub payer_reference: String,

    /// URL bKash redirects to once the customer has completed the
    /// wallet-side flow.
    #[serde(rename = "callbackURL")]
    pub callback_url: String,

    /// Payment amount.
    pub amount: Money,

    /// Currency. bKash currently only supports BDT.
    pub currency: Currency,

    /// Intent. URL-based checkout only supports [`Intent::Sale`].
    pub intent: Intent,

    /// Optional merchant invoice number.
    #[serde(
        rename = "merchantInvoiceNumber",
        skip_serializing_if = "Option::is_none"
    )]
    pub merchant_invoice_number: Option<String>,

    /// Optional merchant association info in TLV format. The bKash TLV
    /// format is `tag1value1tag2value2...`, where each tag is a 4-byte ASCII
    /// string and each value is a UTF-8 string.
    #[serde(
        rename = "merchantAssociationInfo",
        skip_serializing_if = "Option::is_none"
    )]
    pub merchant_association_info: Option<String>,
}

impl CreatePaymentRequest {
    /// Construct a new create-payment request with sensible defaults.
    ///
    /// `mode` is automatically set to [`PAYMENT_MODE`] (`"0011"`) and
    /// `intent` defaults to [`Intent::Sale`].
    #[must_use]
    pub fn new(
        payer_reference: impl Into<String>,
        callback_url: impl Into<String>,
        amount: Money,
        currency: Currency,
    ) -> Self {
        Self {
            mode: PAYMENT_MODE.to_string(),
            payer_reference: payer_reference.into(),
            callback_url: callback_url.into(),
            amount,
            currency,
            intent: Intent::Sale,
            merchant_invoice_number: None,
            merchant_association_info: None,
        }
    }

    /// Override the merchant invoice number.
    #[must_use]
    pub fn with_merchant_invoice_number(mut self, n: impl Into<String>) -> Self {
        self.merchant_invoice_number = Some(n.into());
        self
    }

    /// Attach TLV-formatted merchant association info.
    #[must_use]
    pub fn with_merchant_association_info(mut self, tlv: impl Into<String>) -> Self {
        self.merchant_association_info = Some(tlv.into());
        self
    }
}

/// Response from creating a URL-based checkout payment. Contains the
/// `paymentID` that the client passes to
/// [`execute_payment`](super::super::super::checkout::CheckoutClient::execute_payment)
/// and the `bkashURL` the customer is redirected to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CreatePaymentResponse {
    /// `paymentID` to be passed to the execute step. Valid for 24 hours and
    /// for one execution only.
    #[serde(rename = "paymentID")]
    pub payment_id: String,

    /// bKash-generated URL for the customer's wallet-side approval.
    #[serde(rename = "bkashURL", default)]
    pub bkash_url: String,

    /// Echoed callback URL.
    #[serde(rename = "callbackURL", default)]
    pub callback_url: String,

    /// Payment creation timestamp (ISO-8601).
    #[serde(rename = "paymentCreateTime", default)]
    pub payment_create_time: String,

    /// Echoed `payerReference`.
    #[serde(rename = "payerReference", default)]
    pub payer_reference: String,

    /// Echoed organization short code.
    #[serde(rename = "orgShortCode", default)]
    pub org_short_code: String,

    /// Echoed currency.
    #[serde(default)]
    pub currency: Currency,

    /// Echoed intent.
    #[serde(default)]
    pub intent: Intent,

    /// Echoed merchant invoice number.
    #[serde(rename = "merchantInvoiceNumber", default)]
    pub merchant_invoice_number: String,
}

/// Request body for executing a payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExecutePaymentRequest {
    /// `paymentID` returned by [`CreatePaymentResponse::payment_id`].
    #[serde(rename = "paymentID")]
    pub payment_id: String,
}

impl ExecutePaymentRequest {
    /// Construct a new execute-payment request.
    #[must_use]
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

/// Response from executing a payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExecutePaymentResponse {
    /// `paymentID` that was executed.
    #[serde(rename = "paymentID")]
    pub payment_id: String,

    /// bKash transaction ID (`trxID`).
    #[serde(rename = "trxID", default)]
    pub trx_id: String,

    /// Customer MSISDN that completed the payment.
    #[serde(rename = "customerMsisdn", default)]
    pub customer_msisdn: String,

    /// Echoed `payerReference`.
    #[serde(rename = "payerReference", default)]
    pub payer_reference: String,

    /// Organization short code.
    #[serde(rename = "orgShortCode", default)]
    pub org_short_code: String,

    /// Echoed merchant invoice number.
    #[serde(rename = "merchantInvoiceNumber", default)]
    pub merchant_invoice_number: String,

    /// Payment execution timestamp (ISO-8601).
    #[serde(rename = "paymentExecuteTime", default)]
    pub payment_execute_time: String,

    /// Echoed currency.
    #[serde(default)]
    pub currency: Currency,

    /// Echoed intent.
    #[serde(default)]
    pub intent: Intent,

    /// Final transaction status (e.g. `"Completed"`).
    #[serde(rename = "transactionStatus", default)]
    pub transaction_status: String,

    /// Final amount charged.
    #[serde(default)]
    pub amount: Money,
}

/// Request body for querying a payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QueryPaymentRequest {
    /// `paymentID` to query.
    #[serde(rename = "paymentID")]
    pub payment_id: String,
}

impl QueryPaymentRequest {
    /// Construct a new query-payment request.
    #[must_use]
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

/// Response from querying a payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QueryPaymentResponse {
    /// `paymentID` queried.
    #[serde(rename = "paymentID")]
    pub payment_id: String,

    /// bKash transaction ID.
    #[serde(rename = "trxID", default)]
    pub trx_id: String,

    /// Customer MSISDN.
    #[serde(rename = "customerMsisdn", default)]
    pub customer_msisdn: String,

    /// Echoed `payerReference`.
    #[serde(rename = "payerReference", default)]
    pub payer_reference: String,

    /// Echoed organization short code.
    #[serde(rename = "orgShortCode", default)]
    pub org_short_code: String,

    /// Echoed merchant invoice number.
    #[serde(rename = "merchantInvoiceNumber", default)]
    pub merchant_invoice_number: String,

    /// Echoed callback URL.
    #[serde(rename = "callbackURL", default)]
    pub callback_url: String,

    /// Echoed currency.
    #[serde(default)]
    pub currency: Currency,

    /// Echoed intent.
    #[serde(default)]
    pub intent: Intent,

    /// Echoed amount.
    #[serde(default)]
    pub amount: Money,

    /// Final transaction status (e.g. `"Completed"`).
    #[serde(rename = "transactionStatus", default)]
    pub transaction_status: String,

    /// Timestamp the payment was executed (ISO-8601).
    #[serde(rename = "paymentExecuteTime", default)]
    pub payment_execute_time: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::Currency;

    #[test]
    fn create_payment_serialises_with_mode_0011_and_intent_sale() {
        let req = CreatePaymentRequest::new(
            "cust-1",
            "https://example.test/cb",
            Money::bdt("50.00"),
            Currency::Bdt,
        );
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["mode"], "0011");
        assert_eq!(json["intent"], "sale");
        assert_eq!(json["payerReference"], "cust-1");
        assert_eq!(json["callbackURL"], "https://example.test/cb");
        assert_eq!(json["amount"], "50.00");
        assert_eq!(json["currency"], "BDT");
        // No `agreementID` should be present in URL-based checkout.
        assert!(json.get("agreementID").is_none());
        assert!(json.get("merchantInvoiceNumber").is_none());
        assert!(json.get("merchantAssociationInfo").is_none());
    }

    #[test]
    fn create_payment_with_optional_fields_serialises() {
        let req = CreatePaymentRequest::new(
            "cust-1",
            "https://example.test/cb",
            Money::bdt("50.00"),
            Currency::Bdt,
        )
        .with_merchant_invoice_number("INV-2")
        .with_merchant_association_info("tag1value1");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["merchantInvoiceNumber"], "INV-2");
        assert_eq!(json["merchantAssociationInfo"], "tag1value1");
    }

    #[test]
    fn execute_payment_request_serialises() {
        let req = ExecutePaymentRequest::new("TR0001");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["paymentID"], "TR0001");
    }

    #[test]
    fn execute_payment_response_parses_minimal() {
        let body = r#"{
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "sale",
            "paymentExecuteTime": "2026-06-22T10:00:00:000 GMT+06:00"
        }"#;
        let resp: ExecutePaymentResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
        assert_eq!(resp.trx_id, "8A00ABCD");
        assert_eq!(resp.transaction_status, "Completed");
        assert_eq!(resp.amount.as_str(), "50.00");
    }

    #[test]
    fn query_payment_response_parses_minimal() {
        let body = r#"{
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "sale",
            "payerReference": "cust-1",
            "callbackURL": "https://example.test/cb",
            "merchantInvoiceNumber": "",
            "paymentExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00"
        }"#;
        let resp: QueryPaymentResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
        assert_eq!(resp.trx_id, "8A00ABCD");
        assert_eq!(resp.amount.as_str(), "50.00");
        assert_eq!(resp.callback_url, "https://example.test/cb");
    }
}
