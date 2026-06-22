//! bKash webhook verification.
//!
//! bKash webhooks are delivered as **AWS Simple Notification Service (SNS)**
//! messages. The HTTP POST body is an [`SnsEnvelope`] JSON document; the
//! actual transaction payload is the JSON string stored in the envelope's
//! [`SnsEnvelope::message`] field, which is parsed separately via
//! [`parse_event`].
#![cfg(feature = "webhooks")]

use std::fmt;
use std::str::FromStr;

use base64::Engine as _;
use rsa::pkcs1::DecodeRsaPublicKey as _;
use rsa::pkcs1v15::VerifyingKey;
use rsa::signature::Verifier as _;
use rsa::RsaPublicKey;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use x509_cert::der::DecodePem;
use x509_cert::Certificate;

use crate::error::Error;
use crate::models::common::Money;

/// Signable keys for an SNS `Notification` message, in the canonical
/// order required by the AWS SNS spec.
///
/// See: <https://docs.aws.amazon.com/sns/latest/dg/sns-verify-signature-of-message.html>
const NOTIFICATION_SIGNABLE_KEYS: &[&str] = &[
    "Message",
    "MessageId",
    "Subject",
    "SubscribeURL",
    "Timestamp",
    "TopicArn",
    "Type",
];

/// Signable keys for an SNS `SubscriptionConfirmation` or
/// `UnsubscribeConfirmation` message, in the canonical order required by the
/// AWS SNS spec.
const SUBSCRIPTION_SIGNABLE_KEYS: &[&str] = &[
    "Message",
    "MessageId",
    "Subject",
    "SubscribeURL",
    "Timestamp",
    "Token",
    "TopicArn",
    "Type",
];

/// A raw SNS HTTP message body, deserialized from the JSON sent by bKash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnsEnvelope {
    /// `"Notification"` | `"SubscriptionConfirmation"` |
    /// `"UnsubscribeConfirmation"`.
    #[serde(rename = "Type")]
    pub r#type: String,

    /// Globally unique SNS message id.
    #[serde(rename = "MessageId")]
    pub message_id: String,

    /// Token used to confirm a subscription (only on subscription
    /// control messages).
    #[serde(rename = "Token", default)]
    pub token: String,

    /// SNS topic ARN (e.g. `arn:aws:sns:ap-southeast-1:...:bpt_xxx`).
    #[serde(rename = "TopicArn")]
    pub topic_arn: String,

    /// The actual bKash transaction payload, as a JSON string.
    #[serde(rename = "Message")]
    pub message: String,

    /// URL the merchant must `GET` to confirm a subscription.
    #[serde(rename = "SubscribeURL", default)]
    pub subscribe_url: String,

    /// ISO-8601 timestamp the message was published.
    #[serde(rename = "Timestamp")]
    pub timestamp: String,

    /// Always `"1"` for current bKash webhooks (RSA + SHA-1).
    #[serde(rename = "SignatureVersion")]
    pub signature_version: String,

    /// Base64-encoded RSA-SHA1 signature.
    #[serde(rename = "Signature")]
    pub signature: String,

    /// URL of the X.509 cert used to sign the message. Must be on
    /// `sns.*.amazonaws.com` and end in `.pem`.
    #[serde(rename = "SigningCertURL")]
    pub signing_cert_url: String,

    /// URL the merchant can `GET` to unsubscribe.
    #[serde(rename = "UnsubscribeURL", default)]
    pub unsubscribe_url: String,

    /// Optional subject line (rarely set for bKash notifications).
    #[serde(rename = "Subject", default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

impl SnsEnvelope {
    /// Returns `true` if the envelope is a subscription confirmation.
    #[must_use]
    pub fn is_subscription_confirmation(&self) -> bool {
        self.r#type == "SubscriptionConfirmation"
    }

    /// Returns `true` if the envelope is a notification.
    #[must_use]
    pub fn is_notification(&self) -> bool {
        self.r#type == "Notification"
    }

    /// Returns `true` if the envelope is an unsubscribe confirmation.
    #[must_use]
    pub fn is_unsubscribe_confirmation(&self) -> bool {
        self.r#type == "UnsubscribeConfirmation"
    }
}

/// Transaction type code from the bKash `Message.transactionType` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TransactionType {
    /// `10002294` — Payment via API.
    #[serde(rename = "10002294")]
    PaymentViaApi,

    /// `10003126` — Payment via QR.
    #[serde(rename = "10003126")]
    PaymentViaQr,

    /// `10002175` — Payment via USSD.
    #[serde(rename = "10002175")]
    PaymentViaUssd,

    /// `10002809` — Redeem Voucher.
    #[serde(rename = "10002809")]
    RedeemVoucher,

    /// `10002264` — M2M Transfer via API.
    #[serde(rename = "10002264")]
    M2mTransferViaApi,

    /// `10003209` — M2M Transfer via QR.
    #[serde(rename = "10003209")]
    M2mTransferViaQr,

    /// `10002177` — M2M Transfer via USSD.
    #[serde(rename = "10002177")]
    M2mTransferViaUssd,

    /// `10003476` — Payment via Bank.
    #[serde(rename = "10003476")]
    PaymentViaBank,

    /// `10003237` — BC2M (B2B Collection Wallet to Merchant Plus).
    #[serde(rename = "10003237")]
    Bc2m,

    /// `10003236` — D2BC (Distributor to B2B Collection Wallet Transfer via
    /// USSD).
    #[serde(rename = "10003236")]
    D2bc,

    /// `10004036` — DSO to Merchant Plus-B2BC via API.
    #[serde(rename = "10004036")]
    DsoToMerchantPlusB2bc,

    /// Any other transaction type code bKash may introduce. Preserved
    /// as the raw numeric value.
    #[serde(untagged)]
    Other(u32),
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PaymentViaApi => f.write_str("10002294"),
            Self::PaymentViaQr => f.write_str("10003126"),
            Self::PaymentViaUssd => f.write_str("10002175"),
            Self::RedeemVoucher => f.write_str("10002809"),
            Self::M2mTransferViaApi => f.write_str("10002264"),
            Self::M2mTransferViaQr => f.write_str("10003209"),
            Self::M2mTransferViaUssd => f.write_str("10002177"),
            Self::PaymentViaBank => f.write_str("10003476"),
            Self::Bc2m => f.write_str("10003237"),
            Self::D2bc => f.write_str("10003236"),
            Self::DsoToMerchantPlusB2bc => f.write_str("10004036"),
            Self::Other(code) => write!(f, "{code}"),
        }
    }
}

impl TransactionType {
    /// Returns the wire code (e.g. `"10002294"`).
    #[must_use]
    pub fn as_code(&self) -> String {
        match self {
            Self::PaymentViaApi => "10002294".to_string(),
            Self::PaymentViaQr => "10003126".to_string(),
            Self::PaymentViaUssd => "10002175".to_string(),
            Self::RedeemVoucher => "10002809".to_string(),
            Self::M2mTransferViaApi => "10002264".to_string(),
            Self::M2mTransferViaQr => "10003209".to_string(),
            Self::M2mTransferViaUssd => "10002177".to_string(),
            Self::PaymentViaBank => "10003476".to_string(),
            Self::Bc2m => "10003237".to_string(),
            Self::D2bc => "10003236".to_string(),
            Self::DsoToMerchantPlusB2bc => "10004036".to_string(),
            Self::Other(code) => code.to_string(),
        }
    }

    /// Parse a bKash transaction type code into a typed [`TransactionType`].
    #[must_use]
    pub fn from_code(code: &str) -> Self {
        match code {
            "10002294" => Self::PaymentViaApi,
            "10003126" => Self::PaymentViaQr,
            "10002175" => Self::PaymentViaUssd,
            "10002809" => Self::RedeemVoucher,
            "10002264" => Self::M2mTransferViaApi,
            "10003209" => Self::M2mTransferViaQr,
            "10002177" => Self::M2mTransferViaUssd,
            "10003476" => Self::PaymentViaBank,
            "10003237" => Self::Bc2m,
            "10003236" => Self::D2bc,
            "10004036" => Self::DsoToMerchantPlusB2bc,
            other => Self::Other(other.parse::<u32>().unwrap_or(0)),
        }
    }
}

impl FromStr for TransactionType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_code(s))
    }
}

/// Regular bKash payment webhook payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct PaymentEvent {
    /// Transaction timestamp in `YYYYMMDDHHmmss` format (bKash wire
    /// format).
    #[serde(rename = "dateTime")]
    pub date_time: String,

    /// MSISDN of the payer's bKash account.
    #[serde(rename = "debitMSISDN")]
    pub debit_msisdn: String,

    /// Name of the receiving organization.
    #[serde(rename = "creditOrganizationName")]
    pub credit_organization_name: String,

    /// Masked bKash short code of the receiving wallet.
    #[serde(rename = "creditShortCode")]
    pub credit_short_code: String,

    /// bKash transaction ID.
    #[serde(rename = "trxID")]
    pub trx_id: String,

    /// Always `"Completed"` for webhook deliveries.
    #[serde(rename = "transactionStatus")]
    pub transaction_status: String,

    /// bKash transaction type code.
    #[serde(rename = "transactionType")]
    pub transaction_type: TransactionType,

    /// Transaction amount.
    #[serde(rename = "amount")]
    pub amount: Money,

    /// Currency code (always `"BDT"` for bKash).
    #[serde(rename = "currency")]
    pub currency: String,

    /// bKash-side transaction reference.
    #[serde(rename = "transactionReference")]
    pub transaction_reference: String,

    /// Merchant-supplied invoice number.
    #[serde(rename = "merchantInvoiceNumber")]
    pub merchant_invoice_number: String,
}

/// Coupon-associated bKash payment webhook payload.
///
/// Adds three additional monetary fields on top of [`PaymentEvent`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct CouponPaymentEvent {
    /// Transaction timestamp in `YYYYMMDDHHmmss` format.
    #[serde(rename = "dateTime")]
    pub date_time: String,

    /// MSISDN of the payer's bKash account.
    #[serde(rename = "debitMSISDN")]
    pub debit_msisdn: String,

    /// Name of the receiving organization.
    #[serde(rename = "creditOrganizationName")]
    pub credit_organization_name: String,

    /// Masked bKash short code of the receiving wallet.
    #[serde(rename = "creditShortCode")]
    pub credit_short_code: String,

    /// bKash transaction ID.
    #[serde(rename = "trxID")]
    pub trx_id: String,

    /// Always `"Completed"` for webhook deliveries.
    #[serde(rename = "transactionStatus")]
    pub transaction_status: String,

    /// bKash transaction type code.
    #[serde(rename = "transactionType")]
    pub transaction_type: TransactionType,

    /// Transaction amount.
    #[serde(rename = "amount")]
    pub amount: Money,

    /// Currency code (always `"BDT"`).
    #[serde(rename = "currency")]
    pub currency: String,

    /// bKash-side transaction reference.
    #[serde(rename = "transactionReference")]
    pub transaction_reference: String,

    /// Merchant-supplied invoice number.
    #[serde(rename = "merchantInvoiceNumber")]
    pub merchant_invoice_number: String,

    /// Coupon discount amount.
    #[serde(rename = "couponAmount")]
    pub coupon_amount: Money,

    /// Merchant's share of the transaction.
    #[serde(rename = "merchantShareAmount")]
    pub merchant_share_amount: Money,

    /// Net sale amount after coupon discount.
    #[serde(rename = "saleAmount")]
    pub sale_amount: Money,
}

/// Strongly-typed bKash webhook event, parsed from the
/// [`SnsEnvelope::message`] field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum WebhookEvent {
    /// Regular payment webhook (no coupon).
    Payment(PaymentEvent),
    /// Coupon-associated payment webhook.
    CouponPayment(CouponPaymentEvent),
    /// Unrecognised message shape. The raw `serde_json::Value` of the
    /// `Message` body is preserved so the caller can inspect it.
    Unknown(serde_json::Value),
}

impl WebhookEvent {
    /// Returns the bKash transaction ID (`trxID`) if this event has one.
    #[must_use]
    pub fn trx_id(&self) -> Option<&str> {
        match self {
            Self::Payment(p) => Some(&p.trx_id),
            Self::CouponPayment(p) => Some(&p.trx_id),
            Self::Unknown(_) => None,
        }
    }
}

/// Build the canonical SNS string-to-sign for a given envelope, ready to be
/// fed into the RSA-SHA1 verifier.
///
/// The canonical form is `KEY\nVALUE\nKEY\nVALUE\n…` for the keys listed in
/// [`NOTIFICATION_SIGNABLE_KEYS`] or [`SUBSCRIPTION_SIGNABLE_KEYS`], skipping
/// any key whose value is absent (e.g. `Subject`, `Token`,
/// `SubscribeURL`).
///
/// This is exposed publicly because callers writing custom verification
/// flows (e.g. replay-cached signatures) may want it. It is not part of the
/// stable API surface — treat as `#[doc(hidden)]` if you don't need it.
#[doc(hidden)]
pub fn build_string_to_sign(envelope: &SnsEnvelope) -> String {
    let keys: &[&str] = if envelope.is_notification() {
        NOTIFICATION_SIGNABLE_KEYS
    } else {
        // SubscriptionConfirmation, UnsubscribeConfirmation, and any future
        // subscription-control message type share the same key set per the
        // AWS SNS spec.
        SUBSCRIPTION_SIGNABLE_KEYS
    };

    let mut out = String::new();
    for &key in keys {
        let value = envelope_field(envelope, key);
        if let Some(v) = value {
            out.push_str(key);
            out.push('\n');
            out.push_str(&v);
            out.push('\n');
        }
    }
    out
}

/// Look up a field of the envelope by its SNS JSON key name, returning
/// `None` if the field is absent or empty.
fn envelope_field(envelope: &SnsEnvelope, key: &str) -> Option<String> {
    match key {
        "Message" => Some(envelope.message.clone()),
        "MessageId" => Some(envelope.message_id.clone()),
        "Subject" => envelope.subject.clone().filter(|s| !s.is_empty()),
        "SubscribeURL" => {
            if envelope.subscribe_url.is_empty() {
                None
            } else {
                Some(envelope.subscribe_url.clone())
            }
        }
        "Timestamp" => Some(envelope.timestamp.clone()),
        "Token" => {
            if envelope.token.is_empty() {
                None
            } else {
                Some(envelope.token.clone())
            }
        }
        "TopicArn" => Some(envelope.topic_arn.clone()),
        "Type" => Some(envelope.r#type.clone()),
        _ => None,
    }
}

/// Validate that the `SigningCertURL` points at the expected
/// `https://sns.<region>.amazonaws.com/...pem` location.
fn validate_signing_cert_url(raw: &str) -> Result<url::Url, Error> {
    let url = url::Url::parse(raw).map_err(|_| Error::InvalidSignature)?;
    if url.scheme() != "https" {
        return Err(Error::InvalidSignature);
    }
    let host = url.host_str().ok_or(Error::InvalidSignature)?;
    if !host.starts_with("sns.") || !host.ends_with(".amazonaws.com") {
        return Err(Error::InvalidSignature);
    }
    if !url.path().ends_with(".pem") {
        return Err(Error::InvalidSignature);
    }
    Ok(url)
}

/// Fetch the X.509 cert at the given URL and extract its RSA public key.
async fn fetch_public_key(http: &reqwest::Client, cert_url: &str) -> Result<RsaPublicKey, Error> {
    let cert_pem = http
        .get(cert_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    cert_pem_to_public_key(&cert_pem)
}

/// Verify the SNS signature on an envelope.
///
/// Steps:
/// 1. Validate the `SigningCertURL` is on `sns.*.amazonaws.com` and ends in
///    `.pem`.
/// 2. Fetch the X.509 cert at the URL.
/// 3. Build the canonical string-to-sign.
/// 4. Verify the base64-decoded `Signature` with **RSA + SHA-1**.
///
/// Returns [`Error::InvalidSignature`] on any failure.
pub async fn verify_sns_signature(
    envelope: &SnsEnvelope,
    http: &reqwest::Client,
) -> Result<(), Error> {
    // 1. URL safety check.
    let _ = validate_signing_cert_url(&envelope.signing_cert_url)?;

    // Only SignatureVersion "1" (RSA + SHA-1) is currently supported.
    if envelope.signature_version != "1" {
        return Err(Error::InvalidSignature);
    }

    // 2. Fetch cert + extract RSA public key.
    let public_key = fetch_public_key(http, &envelope.signing_cert_url).await?;

    verify_signature_with_key(envelope, &public_key)
}

/// Verify an envelope's signature against a pre-fetched RSA public key.
///
/// This is exposed for callers that already hold the public key (e.g. via
/// their own cert-cache) and want to skip the HTTPS fetch.
#[doc(hidden)]
pub fn verify_signature_with_key(
    envelope: &SnsEnvelope,
    public_key: &RsaPublicKey,
) -> Result<(), Error> {
    if envelope.signature_version != "1" {
        return Err(Error::InvalidSignature);
    }

    // Build the canonical string-to-sign.
    let string_to_sign = build_string_to_sign(envelope);

    // Verify RSA-SHA1 signature.
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(envelope.signature.as_bytes())
        .map_err(|_| Error::InvalidSignature)?;
    let signature = rsa::pkcs1v15::Signature::try_from(signature_bytes.as_slice())
        .map_err(|_| Error::InvalidSignature)?;

    let verifying_key = VerifyingKey::<Sha1>::new(public_key.clone());
    verifying_key
        .verify(string_to_sign.as_bytes(), &signature)
        .map_err(|_| Error::InvalidSignature)
}

/// Parse a PEM-encoded X.509 certificate into an RSA public key.
///
/// Exposed for callers that want to cache the public key separately from
/// the URL (e.g. for replay protection).
#[doc(hidden)]
pub fn cert_pem_to_public_key(cert_pem: &str) -> Result<RsaPublicKey, Error> {
    let cert = Certificate::from_pem(cert_pem.as_bytes()).map_err(|_| Error::InvalidSignature)?;
    let spki = &cert.tbs_certificate.subject_public_key_info;
    let raw_key = spki.subject_public_key.raw_bytes();
    RsaPublicKey::from_pkcs1_der(raw_key).map_err(|_| Error::InvalidSignature)
}

/// Confirm a subscription by `GET`-ing the envelope's `SubscribeURL`.
///
/// AWS will return an empty body with HTTP 200 on success. Any non-2xx
/// response (or network error) bubbles up as an [`Error::Http`] or
/// [`Error::Api`].
pub async fn confirm_subscription(
    envelope: &SnsEnvelope,
    http: &reqwest::Client,
) -> Result<(), Error> {
    let resp = http.get(&envelope.subscribe_url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error::Api {
            code: status.as_u16().to_string(),
            message: format!("subscription confirmation failed: {status}"),
            status: status.as_u16(),
        });
    }
    Ok(())
}

/// Parse the JSON string in [`SnsEnvelope::message`] into a typed
/// [`WebhookEvent`].
///
/// If the message body is a regular payment, returns
/// [`WebhookEvent::Payment`]. If it contains any of the coupon-specific
/// fields (`couponAmount`, `merchantShareAmount`, `saleAmount`), returns
/// [`WebhookEvent::CouponPayment`]. Otherwise returns
/// [`WebhookEvent::Unknown`] with the raw JSON preserved.
pub fn parse_event(envelope: &SnsEnvelope) -> Result<WebhookEvent, Error> {
    let value: serde_json::Value = serde_json::from_str(&envelope.message)?;
    let obj = match value.as_object() {
        Some(obj) => obj,
        None => return Ok(WebhookEvent::Unknown(value)),
    };

    let has_coupon = obj.contains_key("couponAmount")
        || obj.contains_key("merchantShareAmount")
        || obj.contains_key("saleAmount");

    if has_coupon {
        let event: CouponPaymentEvent = serde_json::from_value(value)?;
        Ok(WebhookEvent::CouponPayment(event))
    } else if obj.contains_key("trxID") {
        let event: PaymentEvent = serde_json::from_value(value)?;
        Ok(WebhookEvent::Payment(event))
    } else {
        Ok(WebhookEvent::Unknown(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payment_message() -> &'static str {
        r#"{
            "dateTime": "20180419122247",
            "debitMSISDN": "8801700000001",
            "creditOrganizationName": "Org 01",
            "creditShortCode": "01929918***",
            "trxID": "4J420ANOXC",
            "transactionStatus": "Completed",
            "transactionType": "10002294",
            "amount": "100",
            "currency": "BDT",
            "transactionReference": "Test_Payment",
            "merchantInvoiceNumber": "orderId1233"
        }"#
    }

    fn sample_coupon_message() -> &'static str {
        r#"{
            "dateTime": "20180419122247",
            "debitMSISDN": "8801700000001",
            "creditOrganizationName": "Org 01",
            "creditShortCode": "01929918***",
            "trxID": "4J420ANOXC",
            "transactionStatus": "Completed",
            "transactionType": "10002294",
            "amount": "100",
            "currency": "BDT",
            "transactionReference": "Test_Payment",
            "merchantInvoiceNumber": "orderId1233",
            "couponAmount": "10",
            "merchantShareAmount": "90",
            "saleAmount": "90"
        }"#
    }

    fn notification_envelope() -> SnsEnvelope {
        SnsEnvelope {
            r#type: "Notification".into(),
            message_id: "msg-1".into(),
            token: String::new(),
            topic_arn: "arn:aws:sns:ap-southeast-1:797962984373:bpt_xxx".into(),
            message: sample_payment_message().into(),
            subscribe_url: String::new(),
            timestamp: "2026-06-22T12:00:00.000Z".into(),
            signature_version: "1".into(),
            signature: "ignored".into(),
            signing_cert_url:
                "https://sns.ap-southeast-1.amazonaws.com/SimpleNotificationService-xyz.pem".into(),
            unsubscribe_url: "https://sns.ap-southeast-1.amazonaws.com/?Action=Unsubscribe&..."
                .into(),
            subject: None,
        }
    }

    fn subscription_envelope() -> SnsEnvelope {
        SnsEnvelope {
            r#type: "SubscriptionConfirmation".into(),
            message_id: "msg-2".into(),
            token: "TOKEN".into(),
            topic_arn: "arn:aws:sns:ap-southeast-1:797962984373:bpt_xxx".into(),
            message: "You have chosen to subscribe to the topic ...".into(),
            subscribe_url:
                "https://sns.ap-southeast-1.amazonaws.com/?Action=ConfirmSubscription&...".into(),
            timestamp: "2026-06-22T12:00:00.000Z".into(),
            signature_version: "1".into(),
            signature: "ignored".into(),
            signing_cert_url:
                "https://sns.ap-southeast-1.amazonaws.com/SimpleNotificationService-xyz.pem".into(),
            unsubscribe_url: String::new(),
            subject: None,
        }
    }

    #[test]
    fn envelope_deserializes_from_json() {
        let json = r#"{
            "Type": "Notification",
            "MessageId": "abc",
            "Token": "",
            "TopicArn": "arn:aws:sns:ap-southeast-1:1:topic",
            "Message": "{\"trxID\":\"X\"}",
            "SubscribeURL": "",
            "Timestamp": "2026-06-22T12:00:00.000Z",
            "SignatureVersion": "1",
            "Signature": "sig",
            "SigningCertURL": "https://sns.ap-southeast-1.amazonaws.com/SimpleNotificationService-x.pem",
            "UnsubscribeURL": "https://example.com/unsub"
        }"#;
        let env: SnsEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(env.r#type, "Notification");
        assert_eq!(env.message_id, "abc");
        assert!(env.is_notification());
        assert!(!env.is_subscription_confirmation());
    }

    #[test]
    fn parse_event_returns_payment_for_regular_message() {
        let env = notification_envelope();
        let event = parse_event(&env).unwrap();
        match event {
            WebhookEvent::Payment(p) => {
                assert_eq!(p.trx_id, "4J420ANOXC");
                assert_eq!(p.amount.as_str(), "100");
                assert_eq!(p.currency, "BDT");
                assert_eq!(p.transaction_type, TransactionType::PaymentViaApi);
            }
            other => panic!("expected Payment, got {other:?}"),
        }
    }

    #[test]
    fn parse_event_returns_coupon_for_coupon_message() {
        let env = SnsEnvelope {
            message: sample_coupon_message().into(),
            ..notification_envelope()
        };
        let event = parse_event(&env).unwrap();
        match event {
            WebhookEvent::CouponPayment(p) => {
                assert_eq!(p.coupon_amount.as_str(), "10");
                assert_eq!(p.merchant_share_amount.as_str(), "90");
                assert_eq!(p.sale_amount.as_str(), "90");
            }
            other => panic!("expected CouponPayment, got {other:?}"),
        }
    }

    #[test]
    fn parse_event_returns_unknown_for_unrecognised_shape() {
        let env = SnsEnvelope {
            message: r#"{"unrelated":"value"}"#.into(),
            ..notification_envelope()
        };
        let event = parse_event(&env).unwrap();
        match event {
            WebhookEvent::Unknown(v) => assert_eq!(v["unrelated"], "value"),
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn transaction_type_from_code_maps_known_codes() {
        assert_eq!(
            TransactionType::from_code("10002294"),
            TransactionType::PaymentViaApi
        );
        assert_eq!(
            TransactionType::from_code("10003126"),
            TransactionType::PaymentViaQr
        );
        assert_eq!(
            TransactionType::from_code("10002175"),
            TransactionType::PaymentViaUssd
        );
        assert_eq!(
            TransactionType::from_code("10002809"),
            TransactionType::RedeemVoucher
        );
        assert_eq!(
            TransactionType::from_code("10002264"),
            TransactionType::M2mTransferViaApi
        );
        assert_eq!(
            TransactionType::from_code("10003209"),
            TransactionType::M2mTransferViaQr
        );
        assert_eq!(
            TransactionType::from_code("10002177"),
            TransactionType::M2mTransferViaUssd
        );
        assert_eq!(
            TransactionType::from_code("10003476"),
            TransactionType::PaymentViaBank
        );
        assert_eq!(
            TransactionType::from_code("10003237"),
            TransactionType::Bc2m
        );
        assert_eq!(
            TransactionType::from_code("10003236"),
            TransactionType::D2bc
        );
        assert_eq!(
            TransactionType::from_code("10004036"),
            TransactionType::DsoToMerchantPlusB2bc
        );
    }

    #[test]
    fn transaction_type_from_code_falls_back_to_other() {
        match TransactionType::from_code("99999999") {
            TransactionType::Other(n) => assert_eq!(n, 99_999_999),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn transaction_type_display_round_trip() {
        let variants = [
            TransactionType::PaymentViaApi,
            TransactionType::PaymentViaQr,
            TransactionType::PaymentViaUssd,
            TransactionType::RedeemVoucher,
            TransactionType::M2mTransferViaApi,
            TransactionType::M2mTransferViaQr,
            TransactionType::M2mTransferViaUssd,
            TransactionType::PaymentViaBank,
            TransactionType::Bc2m,
            TransactionType::D2bc,
            TransactionType::DsoToMerchantPlusB2bc,
        ];
        for v in variants {
            assert_eq!(TransactionType::from_code(&v.as_code()), v);
        }
    }

    #[test]
    fn build_string_to_sign_for_notification() {
        let env = notification_envelope();
        let s = build_string_to_sign(&env);
        // The `Message` value is the raw JSON string exactly as it appears in
        // the envelope (whitespace-preserving, no canonicalisation).
        let expected = format!(
            "Message\n{}\nMessageId\nmsg-1\nTimestamp\n2026-06-22T12:00:00.000Z\nTopicArn\narn:aws:sns:ap-southeast-1:797962984373:bpt_xxx\nType\nNotification\n",
            sample_payment_message()
        );
        assert_eq!(s, expected);
    }

    #[test]
    fn build_string_to_sign_for_subscription_confirmation() {
        let env = subscription_envelope();
        let s = build_string_to_sign(&env);
        let expected = "Message\nYou have chosen to subscribe to the topic ...\n\
                        MessageId\nmsg-2\n\
                        SubscribeURL\nhttps://sns.ap-southeast-1.amazonaws.com/?Action=ConfirmSubscription&...\n\
                        Timestamp\n2026-06-22T12:00:00.000Z\n\
                        Token\nTOKEN\n\
                        TopicArn\narn:aws:sns:ap-southeast-1:797962984373:bpt_xxx\n\
                        Type\nSubscriptionConfirmation\n";
        assert_eq!(s, expected);
    }

    #[test]
    fn build_string_to_sign_includes_subject_when_present() {
        let env = SnsEnvelope {
            subject: Some("Hello".into()),
            ..notification_envelope()
        };
        let s = build_string_to_sign(&env);
        assert!(s.contains("Subject\nHello\n"));
    }

    #[test]
    fn validate_signing_cert_url_accepts_sns() {
        assert!(validate_signing_cert_url(
            "https://sns.ap-southeast-1.amazonaws.com/SimpleNotificationService-x.pem"
        )
        .is_ok());
    }

    #[test]
    fn validate_signing_cert_url_rejects_non_sns_host() {
        assert!(validate_signing_cert_url(
            "https://evil.example.com/SimpleNotificationService-x.pem"
        )
        .is_err());
    }

    #[test]
    fn validate_signing_cert_url_rejects_non_pem_path() {
        assert!(
            validate_signing_cert_url("https://sns.ap-southeast-1.amazonaws.com/cert.txt").is_err()
        );
    }

    #[test]
    fn validate_signing_cert_url_rejects_http() {
        assert!(
            validate_signing_cert_url("http://sns.ap-southeast-1.amazonaws.com/x.pem").is_err()
        );
    }

    #[test]
    fn validate_signing_cert_url_rejects_garbage() {
        assert!(validate_signing_cert_url("not a url").is_err());
    }

    #[test]
    fn envelope_flag_helpers() {
        let n = notification_envelope();
        assert!(n.is_notification());
        assert!(!n.is_subscription_confirmation());
        assert!(!n.is_unsubscribe_confirmation());

        let s = subscription_envelope();
        assert!(!s.is_notification());
        assert!(s.is_subscription_confirmation());
        assert!(!s.is_unsubscribe_confirmation());
    }

    #[test]
    fn webhook_event_trx_id_accessor() {
        let env = notification_envelope();
        let event = parse_event(&env).unwrap();
        assert_eq!(event.trx_id(), Some("4J420ANOXC"));
    }
}
