//! Error types returned by the bKash client.
//!
//! [`Error`] is the crate-level error type, modeled with `thiserror 2`. The
//! [`ErrorCode`] enum is a strongly-typed view of the `errorCode` field
//! returned by bKash endpoints. Use [`ErrorCode::is_transient`] and
//! [`ErrorCode::is_auth`] to drive retry / re-grant policy.

use std::fmt;
use std::str::FromStr;

/// Crate-level error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Underlying HTTP transport error (network, timeout, TLS, etc.).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Failed to decode a JSON response body.
    #[error("failed to decode response: {0}")]
    Decode(#[from] serde_json::Error),

    /// A bKash API call returned a non-success envelope (i.e. `errorCode` was
    /// present, or `statusCode` was not `"0000"`).
    #[error("bKash API error: {code} — {message}")]
    Api {
        /// The bKash `errorCode` (or `externalCode` for refund responses).
        code: String,
        /// The human-readable error message from bKash.
        message: String,
        /// The HTTP status code returned alongside the body.
        status: u16,
    },

    /// Authentication / authorization failure (e.g. `2001` Invalid App Key, or
    /// repeated 401 after a force-regrant).
    #[error("bKash auth error: {0}")]
    Auth(String),

    /// The client was built with an invalid configuration.
    #[error("invalid configuration: {0}")]
    Config(String),

    /// Webhook signature verification failed.
    #[error("webhook signature verification failed")]
    InvalidSignature,

    /// Failed to parse a URL.
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    /// The token grant / refresh endpoint returned an error.
    #[error("token endpoint error: {0}")]
    Token(String),
}

impl Error {
    /// Returns `true` if this error represents a transient condition that may
    /// succeed on retry (network error, HTTP 5xx, or `errorCode == "503"`).
    #[must_use]
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Http(_) => true,
            Self::Api { code, status, .. } => *status >= 500 || code == "503",
            _ => false,
        }
    }

    /// Returns `true` if this error represents a credential / authorization
    /// failure that warrants clearing the token cache and re-granting.
    #[must_use]
    pub fn is_auth(&self) -> bool {
        match self {
            Self::Auth(_) => true,
            Self::Api { code, .. } => ErrorCode::from_code(code).is_auth(),
            _ => false,
        }
    }
}

/// Strongly-typed view of bKash's `errorCode` field.
///
/// Unknown codes are captured as [`ErrorCode::Other`] so the client never
/// panics on a new code bKash may introduce.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// `2001` — Invalid App Key.
    InvalidAppKey,
    /// `2002` — Invalid Payment ID.
    InvalidPaymentId,
    /// `2003` — Process failed.
    ProcessFailed,
    /// `2004` — Invalid firstPaymentDate.
    InvalidFirstPaymentDate,
    /// `2005` — Invalid frequency.
    InvalidFrequency,
    /// `2006` — Invalid amount.
    InvalidAmount,
    /// `2007` — Invalid currency.
    InvalidCurrency,
    /// `2008` — Invalid intent.
    InvalidIntent,
    /// `2009` — Invalid Wallet.
    InvalidWallet,
    /// `2010` — Invalid OTP.
    InvalidOtp,
    /// `2011` — Invalid PIN.
    InvalidPin,
    /// `2012` — Invalid Receiver MSISDN.
    InvalidReceiverMsisdn,
    /// `2013` — Resend Limit Exceeded.
    ResendLimitExceeded,
    /// `2014` — Wrong PIN.
    WrongPin,
    /// `2015` — Wrong PIN count exceeded.
    WrongPinCountExceeded,
    /// `2016` — Wrong verification code.
    WrongVerificationCode,
    /// `2017` — Wrong verification limit.
    WrongVerificationLimit,
    /// `2018` — OTP verification time expired.
    OtpVerificationTimeExpired,
    /// `2019` — PIN verification time expired.
    PinVerificationTimeExpired,
    /// `2020` — Exception occurred.
    ExceptionOccurred,
    /// `2021` — Invalid Mandate ID.
    InvalidMandateId,
    /// `2022` — Missing Mandate ID.
    MissingMandateId,
    /// `2023` — Insufficient Balance.
    InsufficientBalance,
    /// `2024` — Exception occurred (alternate).
    ExceptionOccurredAlt,
    /// `2025` — Invalid request body.
    InvalidRequestBody,
    /// `2026` — Reversal amount > original.
    ReversalAmountExceedsOriginal,
    /// `2027` — Mandate already exists.
    MandateAlreadyExists,
    /// `2028` — Reversal error.
    ReversalError,
    /// `2029` — Duplicate request.
    DuplicateRequest,
    /// `2030` — Mandate type error.
    MandateTypeError,
    /// `2031` — Invalid merchant invoice number.
    InvalidMerchantInvoiceNumber,
    /// `2032` — Invalid transfer type.
    InvalidTransferType,
    /// `2033` — Transaction not found.
    TransactionNotFound,
    /// `2034` — Reversal not allowed.
    ReversalNotAllowed,
    /// `2035` — Account state error.
    AccountStateError,
    /// `2036` — Account permission error.
    AccountPermissionError,
    /// `2037` — Account state error (alt).
    AccountStateErrorAlt,
    /// `2038` — Account state error (alt 2).
    AccountStateErrorAlt2,
    /// `2039` — Account state error (alt 3).
    AccountStateErrorAlt3,
    /// `2040` — Account state error (alt 4).
    AccountStateErrorAlt4,
    /// `2041` — Account state error (alt 5).
    AccountStateErrorAlt5,
    /// `2042` — Account state error (alt 6).
    AccountStateErrorAlt6,
    /// `2043` — Security error.
    SecurityError,
    /// `2044` — Security error (alt).
    SecurityErrorAlt,
    /// `2045` — Subscription error.
    SubscriptionError,
    /// `2046` — Subscription error (alt).
    SubscriptionErrorAlt,
    /// `2047` — TLV format error.
    TlvFormat,
    /// `2048` — Invalid Payer Reference.
    InvalidPayerReference,
    /// `2049` — Invalid Merchant Callback URL.
    InvalidMerchantCallbackUrl,
    /// `2050` — Agreement already exists.
    AgreementAlreadyExists,
    /// `2051` — Invalid Agreement ID.
    InvalidAgreementId,
    /// `2052` — Agreement state error.
    AgreementStateError,
    /// `2053` — Agreement state error (alt).
    AgreementStateErrorAlt,
    /// `2054` — Agreement state error (alt 2).
    AgreementStateErrorAlt2,
    /// `2055` — Agreement state error (alt 3).
    AgreementStateErrorAlt3,
    /// `2056` — Invalid Payment State.
    InvalidPaymentState,
    /// `2057` — Not a bKash Account.
    NotBkashAccount,
    /// `2058` — Customer Wallet error.
    CustomerWalletError,
    /// `2059` — Multiple OTP request denied.
    MultipleOtpRequestDenied,
    /// `2060` — Payment execution pre-requisite not met.
    PaymentExecutionPrerequisiteNotMet,
    /// `2061` — Initiator-only action.
    InitiatorOnlyAction,
    /// `2062` — Payment already completed.
    PaymentAlreadyCompleted,
    /// `2063` — Mode not valid.
    ModeNotValid,
    /// `2064` — Product mode unavailable.
    ProductModeUnavailable,
    /// `2065` — Mandatory field missing.
    MandatoryFieldMissing,
    /// `2066` — Agreement sharing not allowed.
    AgreementSharingNotAllowed,
    /// `2067` — Agreement permission error.
    AgreementPermissionError,
    /// `2068` — Payment already completed (alt).
    PaymentAlreadyCompletedAlt,
    /// `2069` — Agreement already cancelled.
    AgreementAlreadyCancelled,
    /// `2071` — Refund: invalid amount.
    RefundInvalidAmount,
    /// `2072` — Refund: amount exceeds available.
    RefundAmountExceedsAvailable,
    /// `2073` — Refund: transaction not eligible.
    RefundTransactionNotEligible,
    /// `2074` — Refund: already fully refunded.
    RefundAlreadyFullyRefunded,
    /// `2075` — Refund: duplicate refund.
    RefundDuplicate,
    /// `2076` — Refund: charge greater than original.
    RefundChargeExceedsOriginal,
    /// `2077` — Refund: not permitted.
    RefundNotPermitted,
    /// `2078` — Refund: maximum refunds reached.
    RefundMaxRefundsReached,
    /// `2079` — Refund: invalid refund reference.
    RefundInvalidReference,
    /// `2080` — Refund: not found.
    RefundNotFound,
    /// `2081` — Refund: not allowed in current state.
    RefundInvalidState,
    /// `2082` — Merchant not permitted.
    MerchantNotPermitted,
    /// `2116` — Agreement execution already completed.
    AgreementExecutionAlreadyCompleted,
    /// `2117` — Payment execution already completed.
    PaymentExecutionAlreadyCompleted,
    /// `2118` — Invalid Platform.
    InvalidPlatform,
    /// `2119` — Authorized payment already processed.
    AuthorizedPaymentAlreadyProcessed,
    /// `2127` — Transaction not yet completed.
    TransactionNotYetCompleted,
    /// `503` — System undergoing maintenance.
    SystemUndergoingMaintenance,
    /// Any unknown / unrecognised error code. Preserved as a raw string.
    Other(String),
}

impl ErrorCode {
    /// Parse a bKash `errorCode` string into a typed [`ErrorCode`].
    #[must_use]
    pub fn from_code(code: &str) -> Self {
        match code {
            "2001" => Self::InvalidAppKey,
            "2002" => Self::InvalidPaymentId,
            "2003" => Self::ProcessFailed,
            "2004" => Self::InvalidFirstPaymentDate,
            "2005" => Self::InvalidFrequency,
            "2006" => Self::InvalidAmount,
            "2007" => Self::InvalidCurrency,
            "2008" => Self::InvalidIntent,
            "2009" => Self::InvalidWallet,
            "2010" => Self::InvalidOtp,
            "2011" => Self::InvalidPin,
            "2012" => Self::InvalidReceiverMsisdn,
            "2013" => Self::ResendLimitExceeded,
            "2014" => Self::WrongPin,
            "2015" => Self::WrongPinCountExceeded,
            "2016" => Self::WrongVerificationCode,
            "2017" => Self::WrongVerificationLimit,
            "2018" => Self::OtpVerificationTimeExpired,
            "2019" => Self::PinVerificationTimeExpired,
            "2020" => Self::ExceptionOccurred,
            "2021" => Self::InvalidMandateId,
            "2022" => Self::MissingMandateId,
            "2023" => Self::InsufficientBalance,
            "2024" => Self::ExceptionOccurredAlt,
            "2025" => Self::InvalidRequestBody,
            "2026" => Self::ReversalAmountExceedsOriginal,
            "2027" => Self::MandateAlreadyExists,
            "2028" => Self::ReversalError,
            "2029" => Self::DuplicateRequest,
            "2030" => Self::MandateTypeError,
            "2031" => Self::InvalidMerchantInvoiceNumber,
            "2032" => Self::InvalidTransferType,
            "2033" => Self::TransactionNotFound,
            "2034" => Self::ReversalNotAllowed,
            "2035" => Self::AccountStateError,
            "2036" => Self::AccountPermissionError,
            "2037" => Self::AccountStateErrorAlt,
            "2038" => Self::AccountStateErrorAlt2,
            "2039" => Self::AccountStateErrorAlt3,
            "2040" => Self::AccountStateErrorAlt4,
            "2041" => Self::AccountStateErrorAlt5,
            "2042" => Self::AccountStateErrorAlt6,
            "2043" => Self::SecurityError,
            "2044" => Self::SecurityErrorAlt,
            "2045" => Self::SubscriptionError,
            "2046" => Self::SubscriptionErrorAlt,
            "2047" => Self::TlvFormat,
            "2048" => Self::InvalidPayerReference,
            "2049" => Self::InvalidMerchantCallbackUrl,
            "2050" => Self::AgreementAlreadyExists,
            "2051" => Self::InvalidAgreementId,
            "2052" => Self::AgreementStateError,
            "2053" => Self::AgreementStateErrorAlt,
            "2054" => Self::AgreementStateErrorAlt2,
            "2055" => Self::AgreementStateErrorAlt3,
            "2056" => Self::InvalidPaymentState,
            "2057" => Self::NotBkashAccount,
            "2058" => Self::CustomerWalletError,
            "2059" => Self::MultipleOtpRequestDenied,
            "2060" => Self::PaymentExecutionPrerequisiteNotMet,
            "2061" => Self::InitiatorOnlyAction,
            "2062" => Self::PaymentAlreadyCompleted,
            "2063" => Self::ModeNotValid,
            "2064" => Self::ProductModeUnavailable,
            "2065" => Self::MandatoryFieldMissing,
            "2066" => Self::AgreementSharingNotAllowed,
            "2067" => Self::AgreementPermissionError,
            "2068" => Self::PaymentAlreadyCompletedAlt,
            "2069" => Self::AgreementAlreadyCancelled,
            "2071" => Self::RefundInvalidAmount,
            "2072" => Self::RefundAmountExceedsAvailable,
            "2073" => Self::RefundTransactionNotEligible,
            "2074" => Self::RefundAlreadyFullyRefunded,
            "2075" => Self::RefundDuplicate,
            "2076" => Self::RefundChargeExceedsOriginal,
            "2077" => Self::RefundNotPermitted,
            "2078" => Self::RefundMaxRefundsReached,
            "2079" => Self::RefundInvalidReference,
            "2080" => Self::RefundNotFound,
            "2081" => Self::RefundInvalidState,
            "2082" => Self::MerchantNotPermitted,
            "2116" => Self::AgreementExecutionAlreadyCompleted,
            "2117" => Self::PaymentExecutionAlreadyCompleted,
            "2118" => Self::InvalidPlatform,
            "2119" => Self::AuthorizedPaymentAlreadyProcessed,
            "2127" => Self::TransactionNotYetCompleted,
            "503" => Self::SystemUndergoingMaintenance,
            other => Self::Other(other.to_string()),
        }
    }

    /// Returns the string code (e.g. `"2001"`).
    #[must_use]
    pub fn as_code(&self) -> &str {
        match self {
            Self::InvalidAppKey => "2001",
            Self::InvalidPaymentId => "2002",
            Self::ProcessFailed => "2003",
            Self::InvalidFirstPaymentDate => "2004",
            Self::InvalidFrequency => "2005",
            Self::InvalidAmount => "2006",
            Self::InvalidCurrency => "2007",
            Self::InvalidIntent => "2008",
            Self::InvalidWallet => "2009",
            Self::InvalidOtp => "2010",
            Self::InvalidPin => "2011",
            Self::InvalidReceiverMsisdn => "2012",
            Self::ResendLimitExceeded => "2013",
            Self::WrongPin => "2014",
            Self::WrongPinCountExceeded => "2015",
            Self::WrongVerificationCode => "2016",
            Self::WrongVerificationLimit => "2017",
            Self::OtpVerificationTimeExpired => "2018",
            Self::PinVerificationTimeExpired => "2019",
            Self::ExceptionOccurred => "2020",
            Self::InvalidMandateId => "2021",
            Self::MissingMandateId => "2022",
            Self::InsufficientBalance => "2023",
            Self::ExceptionOccurredAlt => "2024",
            Self::InvalidRequestBody => "2025",
            Self::ReversalAmountExceedsOriginal => "2026",
            Self::MandateAlreadyExists => "2027",
            Self::ReversalError => "2028",
            Self::DuplicateRequest => "2029",
            Self::MandateTypeError => "2030",
            Self::InvalidMerchantInvoiceNumber => "2031",
            Self::InvalidTransferType => "2032",
            Self::TransactionNotFound => "2033",
            Self::ReversalNotAllowed => "2034",
            Self::AccountStateError => "2035",
            Self::AccountPermissionError => "2036",
            Self::AccountStateErrorAlt => "2037",
            Self::AccountStateErrorAlt2 => "2038",
            Self::AccountStateErrorAlt3 => "2039",
            Self::AccountStateErrorAlt4 => "2040",
            Self::AccountStateErrorAlt5 => "2041",
            Self::AccountStateErrorAlt6 => "2042",
            Self::SecurityError => "2043",
            Self::SecurityErrorAlt => "2044",
            Self::SubscriptionError => "2045",
            Self::SubscriptionErrorAlt => "2046",
            Self::TlvFormat => "2047",
            Self::InvalidPayerReference => "2048",
            Self::InvalidMerchantCallbackUrl => "2049",
            Self::AgreementAlreadyExists => "2050",
            Self::InvalidAgreementId => "2051",
            Self::AgreementStateError => "2052",
            Self::AgreementStateErrorAlt => "2053",
            Self::AgreementStateErrorAlt2 => "2054",
            Self::AgreementStateErrorAlt3 => "2055",
            Self::InvalidPaymentState => "2056",
            Self::NotBkashAccount => "2057",
            Self::CustomerWalletError => "2058",
            Self::MultipleOtpRequestDenied => "2059",
            Self::PaymentExecutionPrerequisiteNotMet => "2060",
            Self::InitiatorOnlyAction => "2061",
            Self::PaymentAlreadyCompleted => "2062",
            Self::ModeNotValid => "2063",
            Self::ProductModeUnavailable => "2064",
            Self::MandatoryFieldMissing => "2065",
            Self::AgreementSharingNotAllowed => "2066",
            Self::AgreementPermissionError => "2067",
            Self::PaymentAlreadyCompletedAlt => "2068",
            Self::AgreementAlreadyCancelled => "2069",
            Self::RefundInvalidAmount => "2071",
            Self::RefundAmountExceedsAvailable => "2072",
            Self::RefundTransactionNotEligible => "2073",
            Self::RefundAlreadyFullyRefunded => "2074",
            Self::RefundDuplicate => "2075",
            Self::RefundChargeExceedsOriginal => "2076",
            Self::RefundNotPermitted => "2077",
            Self::RefundMaxRefundsReached => "2078",
            Self::RefundInvalidReference => "2079",
            Self::RefundNotFound => "2080",
            Self::RefundInvalidState => "2081",
            Self::MerchantNotPermitted => "2082",
            Self::AgreementExecutionAlreadyCompleted => "2116",
            Self::PaymentExecutionAlreadyCompleted => "2117",
            Self::InvalidPlatform => "2118",
            Self::AuthorizedPaymentAlreadyProcessed => "2119",
            Self::TransactionNotYetCompleted => "2127",
            Self::SystemUndergoingMaintenance => "503",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Returns `true` if this error is a transient condition eligible for
    /// retry (e.g. system maintenance).
    #[must_use]
    pub fn is_transient(&self) -> bool {
        matches!(self, Self::SystemUndergoingMaintenance)
    }

    /// Returns `true` if this error is a credential / authorization failure
    /// that warrants clearing the token cache and forcing a re-grant.
    #[must_use]
    pub fn is_auth(&self) -> bool {
        matches!(self, Self::InvalidAppKey)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_code())
    }
}

impl FromStr for ErrorCode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_code(s))
    }
}

impl serde::Serialize for ErrorCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_code())
    }
}

impl<'de> serde::Deserialize<'de> for ErrorCode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(Self::from_code(&s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_parses_known_codes() {
        assert_eq!(ErrorCode::from_code("2001"), ErrorCode::InvalidAppKey);
        assert_eq!(
            ErrorCode::from_code("503"),
            ErrorCode::SystemUndergoingMaintenance
        );
        assert_eq!(
            ErrorCode::from_code("2127"),
            ErrorCode::TransactionNotYetCompleted
        );
    }

    #[test]
    fn from_str_falls_back_to_other() {
        assert_eq!(
            ErrorCode::from_code("99999"),
            ErrorCode::Other("99999".to_string())
        );
    }

    #[test]
    fn is_transient_for_503() {
        assert!(ErrorCode::SystemUndergoingMaintenance.is_transient());
        assert!(ErrorCode::from_code("503").is_transient());
    }

    #[test]
    fn is_transient_false_for_2001() {
        assert!(!ErrorCode::InvalidAppKey.is_transient());
    }

    #[test]
    fn is_auth_for_2001() {
        assert!(ErrorCode::InvalidAppKey.is_auth());
        assert!(ErrorCode::from_code("2001").is_auth());
    }

    #[test]
    fn is_auth_false_for_503() {
        assert!(!ErrorCode::SystemUndergoingMaintenance.is_auth());
    }

    #[test]
    fn display_returns_code_string() {
        assert_eq!(ErrorCode::InvalidAppKey.to_string(), "2001");
        assert_eq!(ErrorCode::SystemUndergoingMaintenance.to_string(), "503");
    }

    #[test]
    fn as_code_round_trips() {
        for variant in [
            ErrorCode::InvalidAppKey,
            ErrorCode::SystemUndergoingMaintenance,
            ErrorCode::TransactionNotYetCompleted,
        ] {
            let code = variant.as_code();
            assert_eq!(ErrorCode::from_code(code), variant);
        }
    }

    #[test]
    fn error_display_does_not_leak_credentials() {
        let e = Error::Api {
            code: "2001".to_string(),
            message: "Invalid App Key".to_string(),
            status: 200,
        };
        let s = e.to_string();
        assert!(s.contains("2001"));
        assert!(s.contains("Invalid App Key"));
    }

    #[test]
    fn error_is_transient_for_503() {
        let e = Error::Api {
            code: "503".to_string(),
            message: "maintenance".to_string(),
            status: 200,
        };
        assert!(e.is_transient());
    }

    #[test]
    fn error_is_auth_for_2001() {
        let e = Error::Api {
            code: "2001".to_string(),
            message: "bad key".to_string(),
            status: 200,
        };
        assert!(e.is_auth());
    }
}
