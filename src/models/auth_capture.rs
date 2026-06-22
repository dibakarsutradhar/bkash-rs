//! Authorization & Capture: request and response models.
//!
//! The Auth & Capture product implements a **delayed-capture reservation
//! model** (see plan §1.9). The verified flow is:
//!
//! 1. [`CreatePaymentRequest`] (`mode = "0011"`, `intent = "authorization"`)
//!    → [`CreatePaymentResponse`] returns a `paymentID` and `bkashURL`.
//! 2. Customer completes wallet approval at `bkashURL`.
//! 3. [`execute_payment`](super::super::auth_capture::AuthCaptureClient::execute_payment)
//!    → `paymentID` is **passed as a path param** to
//!    `/tokenized/checkout/execute/{paymentID}` (not as a body field, as
//!    in other products). Authorization is now complete.
//! 4. [`query_payment`](super::super::auth_capture::AuthCaptureClient::query_payment)
//!    → status should be `"Authorized"`. If still `"Initiated"`, retry
//!    from the create step.
//! 5. Either:
//!    - [`capture`](super::super::auth_capture::AuthCaptureClient::capture)
//!      (if the service was provided) → transaction completed.
//!    - [`void`](super::super::auth_capture::AuthCaptureClient::void)
//!      (if the service was not provided) → authorization cancelled.
//! 6. If no response is found from the capture/void API, **retry** the
//!    corresponding API unless the operation is successful (per §1.9).
//!
//! Note: the create-payment endpoint
//! (`/tokenized/checkout/payment/create`) is **distinct** from the
//! URL-based Checkout create endpoint
//! (`/tokenized/checkout/create`).
//!
//! ## Capture / Void response casing
//!
//! The Capture and Void response bodies use a lowercase `paymentId` field
//! (likely a documentation typo — other endpoints use `paymentID`). The
//! models in this module accept both casings via `#[serde(alias)]`.

use serde::{Deserialize, Serialize};

use crate::models::common::{Currency, Intent, Money, TransactionStatus};

/// `mode` discriminator value for `POST /tokenized/checkout/payment/create`
/// when creating an **Auth & Capture** payment (delayed-capture
/// reservation).
///
/// Note: the same `mode` value (`"0011"`) is used for URL-based Checkout
/// payments; the Auth & Capture flow is distinguished by the `intent`
/// field and the create-payment endpoint path.
pub const PAYMENT_MODE: &str = "0011";

// =====================================================================
// Create Payment (POST /tokenized/checkout/payment/create)
// =====================================================================

/// Request body for creating an Auth & Capture payment.
///
/// `mode` is automatically set to [`PAYMENT_MODE`] (`"0011"`) and `intent`
/// is forced to [`Intent::Authorization`].
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

    /// Intent. For Auth & Capture, this is always
    /// [`Intent::Authorization`].
    pub intent: Intent,

    /// Optional merchant invoice number.
    #[serde(
        rename = "merchantInvoiceNumber",
        skip_serializing_if = "Option::is_none"
    )]
    pub merchant_invoice_number: Option<String>,
}

impl CreatePaymentRequest {
    /// Construct a new create-payment request with sensible defaults.
    ///
    /// `mode` is automatically set to [`PAYMENT_MODE`] (`"0011"`) and
    /// `intent` is forced to [`Intent::Authorization`].
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
            intent: Intent::Authorization,
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

/// Response from creating an Auth & Capture payment. Contains the
/// `paymentID` that the client passes to
/// [`execute_payment`](super::super::auth_capture::AuthCaptureClient::execute_payment)
/// and the `bkashURL` the customer is redirected to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CreatePaymentResponse {
    /// `paymentID` to be passed to the execute step.
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

// =====================================================================
// Execute Payment (POST /tokenized/checkout/execute/{paymentID})
// =====================================================================

/// Response from executing an Auth & Capture payment.
///
/// Note: unlike other products, the `paymentID` is **not** in the request
/// body. It is interpolated into the URL path
/// (`/tokenized/checkout/execute/{paymentID}`), so the execute endpoint
/// has **no** request body model.
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

    /// Echoed callback URL.
    #[serde(rename = "callbackURL", default)]
    pub callback_url: String,

    /// Payment execution timestamp (ISO-8601).
    #[serde(rename = "paymentExecuteTime", default)]
    pub payment_execute_time: String,

    /// Echoed currency.
    #[serde(default)]
    pub currency: Currency,

    /// Echoed intent.
    #[serde(default)]
    pub intent: Intent,

    /// Echoed amount.
    #[serde(default)]
    pub amount: Money,

    /// Authorization status (typically `"Authorized"` after a successful
    /// execute).
    #[serde(rename = "transactionStatus", default)]
    pub transaction_status: TransactionStatus,
}

// =====================================================================
// Query Payment (POST /tokenized/checkout/payment/status)
// =====================================================================

/// Request body for querying an Auth & Capture payment.
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

/// Response from querying an Auth & Capture payment.
///
/// The `transactionStatus` field is the key state machine indicator for
/// the reservation flow. The expected values are:
/// `"Initiated"`, `"Authorized"`, `"Completed"`. `"Initiated"` means the
/// customer has not yet completed the wallet flow; `"Authorized"` means
/// funds are reserved (ready for capture or void); `"Completed"` means
/// the funds have been captured.
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

    /// Authorization / transaction status. Use the variant
    /// [`TransactionStatus::Authorized`] to confirm that a payment has
    /// been authorized and is ready for capture or void.
    #[serde(rename = "transactionStatus", default)]
    pub transaction_status: TransactionStatus,

    /// Timestamp the payment was executed (ISO-8601).
    #[serde(rename = "paymentExecuteTime", default)]
    pub payment_execute_time: String,
}

// =====================================================================
// Capture (POST /tokenized/checkout/payment/confirm/capture)
// =====================================================================

/// Request body for capturing a previously-authorized payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CaptureRequest {
    /// `paymentID` to capture.
    #[serde(rename = "paymentID")]
    pub payment_id: String,
}

impl CaptureRequest {
    /// Construct a new capture request.
    #[must_use]
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

/// Response from capturing a payment.
///
/// Note: the field is named `paymentId` (lowercase `i`) in the bKash
/// documentation samples (likely a doc typo), whereas the rest of the
/// bKash API uses `paymentID`. The model accepts both casings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CaptureResponse {
    /// `paymentId` / `paymentID` that was captured.
    #[serde(rename = "paymentId", alias = "paymentID")]
    pub payment_id: String,

    /// Capture / creation timestamp (ISO-8601).
    #[serde(rename = "createTime", default)]
    pub create_time: String,

    /// Capture / update timestamp (ISO-8601).
    #[serde(rename = "updateTime", default)]
    pub update_time: String,

    /// bKash transaction ID.
    #[serde(rename = "trxID", default)]
    pub trx_id: String,

    /// Final transaction status (typically `"Completed"` after capture).
    #[serde(rename = "transactionStatus", default)]
    pub transaction_status: String,
}

// =====================================================================
// Void (POST /tokenized/checkout/payment/confirm/capture/void)
// =====================================================================

/// Request body for voiding a previously-authorized payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VoidRequest {
    /// `paymentID` to void.
    #[serde(rename = "paymentID")]
    pub payment_id: String,
}

impl VoidRequest {
    /// Construct a new void request.
    #[must_use]
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

/// Response from voiding a payment.
///
/// Note: the field is named `paymentId` (lowercase `i`) in the bKash
/// documentation samples (likely a doc typo), whereas the rest of the
/// bKash API uses `paymentID`. The model accepts both casings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VoidResponse {
    /// `paymentId` / `paymentID` that was voided.
    #[serde(rename = "paymentId", alias = "paymentID")]
    pub payment_id: String,

    /// Void / creation timestamp (ISO-8601).
    #[serde(rename = "createTime", default)]
    pub create_time: String,

    /// Void / update timestamp (ISO-8601).
    #[serde(rename = "updateTime", default)]
    pub update_time: String,

    /// bKash transaction ID.
    #[serde(rename = "trxID", default)]
    pub trx_id: String,

    /// Final transaction status (typically `"Cancelled"` after void).
    #[serde(rename = "transactionStatus", default)]
    pub transaction_status: String,
}

// =====================================================================
// Search Transaction (GET /checkout/payment/search/{trxID})
// =====================================================================

/// Response from the search-transaction endpoint (Auth & Capture).
///
/// Note: this endpoint is **GET** (not POST) and takes `trxID` as a
/// **path param** (not a body field). It returns the full transaction
/// shape.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_payment_serialises_with_mode_0011_and_intent_authorization() {
        let req = CreatePaymentRequest::new(
            "cust-1",
            "https://example.test/cb",
            Money::bdt("50.00"),
            Currency::Bdt,
        );
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["mode"], "0011");
        assert_eq!(json["intent"], "authorization");
        assert_eq!(json["payerReference"], "cust-1");
        assert_eq!(json["callbackURL"], "https://example.test/cb");
        assert_eq!(json["amount"], "50.00");
        assert_eq!(json["currency"], "BDT");
        assert!(json.get("merchantInvoiceNumber").is_none());
    }

    #[test]
    fn create_payment_with_optional_fields_serialises() {
        let req = CreatePaymentRequest::new(
            "cust-1",
            "https://example.test/cb",
            Money::bdt("50.00"),
            Currency::Bdt,
        )
        .with_merchant_invoice_number("INV-AC-1");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["merchantInvoiceNumber"], "INV-AC-1");
    }

    #[test]
    fn create_payment_response_parses_minimal() {
        let body = r#"{
            "paymentID": "TR0001",
            "bkashURL": "https://example.test/bkash",
            "callbackURL": "https://example.test/cb",
            "paymentCreateTime": "2026-06-22T10:00:00:000 GMT+06:00",
            "payerReference": "cust-1",
            "orgShortCode": "0123",
            "currency": "BDT",
            "intent": "authorization",
            "merchantInvoiceNumber": ""
        }"#;
        let resp: CreatePaymentResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
        assert_eq!(resp.intent, Intent::Authorization);
        assert_eq!(resp.bkash_url, "https://example.test/bkash");
    }

    #[test]
    fn execute_payment_response_parses_minimal() {
        let body = r#"{
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "authorization",
            "paymentExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00"
        }"#;
        let resp: ExecutePaymentResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
        assert_eq!(resp.trx_id, "8A00ABCD");
        assert_eq!(resp.transaction_status, TransactionStatus::Authorized);
        assert_eq!(resp.amount.as_str(), "50.00");
    }

    #[test]
    fn query_payment_request_serialises() {
        let req = QueryPaymentRequest::new("TR0001");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["paymentID"], "TR0001");
    }

    #[test]
    fn query_payment_response_parses_authorized() {
        let body = r#"{
            "paymentID": "TR0001",
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "amount": "50.00",
            "currency": "BDT",
            "intent": "authorization",
            "payerReference": "cust-1",
            "customerMsisdn": "01700000000",
            "orgShortCode": "0123",
            "callbackURL": "https://example.test/cb",
            "merchantInvoiceNumber": "",
            "paymentExecuteTime": "2026-06-22T10:01:00:000 GMT+06:00"
        }"#;
        let resp: QueryPaymentResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
        assert_eq!(resp.transaction_status, TransactionStatus::Authorized);
    }

    #[test]
    fn capture_request_serialises() {
        let req = CaptureRequest::new("TR0001");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["paymentID"], "TR0001");
    }

    #[test]
    fn capture_response_accepts_lowercase_payment_id() {
        // The bKash sample uses lowercase `paymentId` for capture/void.
        let body = r#"{
            "paymentId": "TR0001",
            "createTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "updateTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed"
        }"#;
        let resp: CaptureResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
        assert_eq!(resp.trx_id, "8A00ABCD");
        assert_eq!(resp.transaction_status, "Completed");
    }

    #[test]
    fn capture_response_accepts_uppercase_payment_id() {
        // The alias must accept the more typical `paymentID` form too.
        let body = r#"{
            "paymentID": "TR0001",
            "createTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "updateTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "trxID": "8A00ABCD",
            "transactionStatus": "Completed"
        }"#;
        let resp: CaptureResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
    }

    #[test]
    fn void_request_serialises() {
        let req = VoidRequest::new("TR0001");
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["paymentID"], "TR0001");
    }

    #[test]
    fn void_response_accepts_lowercase_payment_id() {
        let body = r#"{
            "paymentId": "TR0001",
            "createTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "updateTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "trxID": "8A00ABCD",
            "transactionStatus": "Cancelled"
        }"#;
        let resp: VoidResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
        assert_eq!(resp.transaction_status, "Cancelled");
    }

    #[test]
    fn void_response_accepts_uppercase_payment_id() {
        let body = r#"{
            "paymentID": "TR0001",
            "createTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "updateTime": "2026-06-22T10:02:00:000 GMT+06:00",
            "trxID": "8A00ABCD",
            "transactionStatus": "Cancelled"
        }"#;
        let resp: VoidResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.payment_id, "TR0001");
    }

    #[test]
    fn search_transaction_response_parses() {
        let body = r#"{
            "trxID": "8A00ABCD",
            "transactionStatus": "Authorized",
            "transactionType": "Payment",
            "amount": "100.00",
            "currency": "BDT",
            "customerMsisdn": "01700000000",
            "organizationShortCode": "0123",
            "initiationTime": "2026-06-22T09:00:00:000 GMT+06:00",
            "completedTime": ""
        }"#;
        let resp: SearchTransactionResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.trx_id, "8A00ABCD");
        assert_eq!(resp.transaction_status, TransactionStatus::Authorized);
        assert_eq!(resp.amount.as_str(), "100.00");
    }
}
