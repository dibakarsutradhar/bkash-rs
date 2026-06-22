//! Tokenized Checkout: Agreement request and response types.
//!
//! An *agreement* is a recurring-billing mandate set up with a customer.
//! The bKash tokenized checkout flow is:
//!
//! 1. [`CreateAgreementRequest`] (`mode = "0000"`) → [`CreateAgreementResponse`]
//!    returns a `paymentID`.
//! 2. Customer completes the wallet-side approval on their device.
//! 3. [`ExecuteAgreementRequest`] → [`ExecuteAgreementResponse`] returns
//!    the `agreementID` once the customer has approved.
//! 4. Optionally [`QueryAgreementRequest`] → [`AgreementStatusResponse`] to
//!    inspect current state.
//! 5. [`CancelAgreementRequest`] → [`CancelAgreementResponse`] revokes an
//!    existing agreement.

use serde::{Deserialize, Serialize};

use crate::models::common::{Currency, Intent, Money};

/// `mode` discriminator value for `POST /tokenized/checkout/create` when
/// creating a **recurring billing agreement**.
pub const AGREEMENT_MODE: &str = "0000";

/// Request body for creating a tokenized checkout agreement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CreateAgreementRequest {
    /// Discriminator. Always [`AGREEMENT_MODE`] (`"0000"`).
    #[serde(rename = "mode")]
    pub mode: String,

    /// Merchant's reference for the payer (e.g. internal customer ID).
    #[serde(rename = "payerReference")]
    pub payer_reference: String,

    /// URL bKash redirects to once the customer has completed the
    /// wallet-side approval flow.
    #[serde(rename = "callbackURL")]
    pub callback_url: String,

    /// Agreement amount (the maximum amount that can be charged per cycle).
    pub amount: Money,

    /// Currency. bKash currently only supports BDT.
    pub currency: Currency,

    /// Intent. Agreements must use `Sale`.
    pub intent: Intent,

    /// Optional merchant invoice number (max length enforced by bKash).
    #[serde(
        rename = "merchantInvoiceNumber",
        skip_serializing_if = "Option::is_none"
    )]
    pub merchant_invoice_number: Option<String>,
}

impl CreateAgreementRequest {
    /// Construct a new create-agreement request with sensible defaults.
    ///
    /// `mode` is automatically set to [`AGREEMENT_MODE`] (`"0000"`) and
    /// `intent` defaults to [`Intent::Sale`].
    #[must_use]
    pub fn new(
        payer_reference: impl Into<String>,
        callback_url: impl Into<String>,
        amount: Money,
        currency: Currency,
    ) -> Self {
        Self {
            mode: AGREEMENT_MODE.to_string(),
            payer_reference: payer_reference.into(),
            callback_url: callback_url.into(),
            amount,
            currency,
            intent: Intent::Sale,
            merchant_invoice_number: None,
        }
    }

    /// Override the merchant invoice number.
    #[must_use]
    pub fn with_merchant_invoice_number(mut self, n: impl Into<String>) -> Self {
        self.merchant_invoice_number = Some(n.into());
        self
    }
}

/// Response from creating an agreement. Contains the `paymentID` that the
/// client passes to [`execute_agreement`](super::super::super::tokenized::TokenizedCheckoutClient::execute_agreement).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CreateAgreementResponse {
    /// `paymentID` to be passed to the execute step.
    #[serde(rename = "paymentID")]
    pub payment_id: String,

    /// bKash-generated URL where the customer approves the agreement.
    #[serde(rename = "bkashURL", default)]
    pub bkash_url: String,

    /// Callback URL echoed back for verification.
    #[serde(rename = "callbackURL", default)]
    pub callback_url: String,

    /// Agreement creation timestamp (ISO-8601).
    #[serde(rename = "agreementCreateTime", default)]
    pub agreement_create_time: String,

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

/// Request body for executing an agreement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExecuteAgreementRequest {
    /// `paymentID` returned by [`CreateAgreementResponse::payment_id`].
    #[serde(rename = "paymentID")]
    pub payment_id: String,
}

impl ExecuteAgreementRequest {
    /// Construct a new execute-agreement request.
    #[must_use]
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

/// Response from executing an agreement. Carries the `agreementID` used in
/// subsequent payment or query operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExecuteAgreementResponse {
    /// bKash-issued agreement ID. Persist this for future payments.
    #[serde(rename = "agreementID")]
    pub agreement_id: String,

    /// `paymentID` that was executed.
    #[serde(rename = "paymentID", default)]
    pub payment_id: String,

    /// Customer MSISDN that approved the agreement (when available).
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

    /// Timestamp the agreement was executed (ISO-8601).
    #[serde(rename = "agreementExecuteTime", default)]
    pub agreement_execute_time: String,

    /// Final agreement status string returned by bKash (e.g. `"Completed"`).
    #[serde(rename = "agreementStatus", default)]
    pub agreement_status: String,
}

/// Request body for querying an agreement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QueryAgreementRequest {
    /// The agreement ID returned from [`ExecuteAgreementResponse`].
    #[serde(rename = "agreementID")]
    pub agreement_id: String,
}

impl QueryAgreementRequest {
    /// Construct a new query-agreement request.
    #[must_use]
    pub fn new(agreement_id: impl Into<String>) -> Self {
        Self {
            agreement_id: agreement_id.into(),
        }
    }
}

/// Response from the agreement-status query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AgreementStatusResponse {
    /// The agreement ID queried.
    #[serde(rename = "agreementID")]
    pub agreement_id: String,

    /// Current status (e.g. `"Initiated"`, `"Completed"`, `"Cancelled"`).
    #[serde(rename = "agreementStatus", default)]
    pub agreement_status: String,

    /// Echoed `payerReference`.
    #[serde(rename = "payerReference", default)]
    pub payer_reference: String,

    /// Customer MSISDN.
    #[serde(rename = "customerMsisdn", default)]
    pub customer_msisdn: String,

    /// Echoed callback URL.
    #[serde(rename = "callbackURL", default)]
    pub callback_url: String,

    /// Echoed amount.
    #[serde(default)]
    pub amount: Money,

    /// Echoed currency.
    #[serde(default)]
    pub currency: Currency,

    /// Echoed intent.
    #[serde(default)]
    pub intent: Intent,

    /// Echoed merchant invoice number.
    #[serde(rename = "merchantInvoiceNumber", default)]
    pub merchant_invoice_number: String,

    /// Echoed organization short code.
    #[serde(rename = "orgShortCode", default)]
    pub org_short_code: String,

    /// Agreement creation timestamp (ISO-8601).
    #[serde(rename = "agreementCreateTime", default)]
    pub agreement_create_time: String,

    /// Agreement execution timestamp (ISO-8601).
    #[serde(rename = "agreementExecuteTime", default)]
    pub agreement_execute_time: String,
}

/// Request body for cancelling an agreement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CancelAgreementRequest {
    /// The agreement ID to cancel.
    #[serde(rename = "agreementID")]
    pub agreement_id: String,
}

impl CancelAgreementRequest {
    /// Construct a new cancel-agreement request.
    #[must_use]
    pub fn new(agreement_id: impl Into<String>) -> Self {
        Self {
            agreement_id: agreement_id.into(),
        }
    }
}

/// Response from cancelling an agreement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CancelAgreementResponse {
    /// The cancelled agreement ID.
    #[serde(rename = "agreementID")]
    pub agreement_id: String,

    /// Updated status (typically `"Cancelled"`).
    #[serde(rename = "agreementStatus", default)]
    pub agreement_status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::Currency;

    #[test]
    fn create_agreement_serialises_with_mode_0000_and_intent_sale() {
        let req = CreateAgreementRequest::new(
            "cust-1",
            "https://example.test/cb",
            Money::bdt("100.00"),
            Currency::Bdt,
        );
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["mode"], "0000");
        assert_eq!(json["intent"], "sale");
        assert_eq!(json["payerReference"], "cust-1");
        assert_eq!(json["callbackURL"], "https://example.test/cb");
        assert_eq!(json["amount"], "100.00");
        assert_eq!(json["currency"], "BDT");
        assert!(json.get("merchantInvoiceNumber").is_none());
    }

    #[test]
    fn create_agreement_with_invoice_serialises_optional() {
        let req = CreateAgreementRequest::new(
            "cust-1",
            "https://example.test/cb",
            Money::bdt("100.00"),
            Currency::Bdt,
        )
        .with_merchant_invoice_number("INV-1");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["merchantInvoiceNumber"], "INV-1");
    }

    #[test]
    fn execute_agreement_request_serialises() {
        let req = ExecuteAgreementRequest::new("TR0001");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["paymentID"], "TR0001");
    }

    #[test]
    fn cancel_agreement_request_serialises() {
        let req = CancelAgreementRequest::new("AG0001");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["agreementID"], "AG0001");
    }

    #[test]
    fn create_agreement_response_parses_minimal() {
        let body = r#"{
            "paymentID": "TR0001",
            "bkashURL": "https://example.test/bkash",
            "callbackURL": "https://example.test/cb",
            "agreementCreateTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "currency": "BDT",
            "intent": "sale",
            "merchantInvoiceNumber": ""
        }"#;
        let resp: CreateAgreementResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
        assert_eq!(resp.intent, Intent::Sale);
        assert_eq!(resp.currency, Currency::Bdt);
    }

    #[test]
    fn execute_agreement_response_parses_minimal() {
        let body = r#"{
            "agreementID": "AG0001",
            "paymentID": "TR0001",
            "agreementExecuteTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "agreementStatus": "Completed"
        }"#;
        let resp: ExecuteAgreementResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.agreement_id, "AG0001");
        assert_eq!(resp.agreement_status, "Completed");
    }

    #[test]
    fn cancel_agreement_response_parses() {
        let body = r#"{
            "agreementID": "AG0001",
            "agreementStatus": "Cancelled"
        }"#;
        let resp: CancelAgreementResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.agreement_id, "AG0001");
        assert_eq!(resp.agreement_status, "Cancelled");
    }

    // ---- proptest round-trips -----------------------------------------

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn create_agreement_request_roundtrip(
            payer_ref in ".*",
            callback in ".*",
            amount in ".*",
            inv in proptest::option::of(".*"),
        ) {
            let mut req = CreateAgreementRequest::new(
                payer_ref,
                callback,
                Money::new(amount),
                Currency::Bdt,
            );
            req.merchant_invoice_number = inv;
            let json = serde_json::to_string(&req).unwrap();
            let back: CreateAgreementRequest = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&back).unwrap();
            prop_assert_eq!(json, json2);
        }
    }
}
