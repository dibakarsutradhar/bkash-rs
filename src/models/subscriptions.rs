//! Subscriptions: one-time authorization for recurring payments.
//!
//! bKash's Subscriptions product is built on top of the tokenized-checkout
//! agreement infrastructure: a customer grants a one-time authorization
//! (an *agreement*) which the merchant can then charge against on a
//! recurring basis.
//!
//! On the wire, the request and response shapes are identical to the
//! [`tokenized`](super::tokenized) agreement types — a subscription
//! agreement **is** a tokenized-checkout agreement. The methods on
//! [`SubscriptionsClient`](crate::subscriptions::SubscriptionsClient)
//! delegate to the same `POST /tokenized/checkout/*` endpoints that
//! [`TokenizedCheckoutClient::create_agreement`](crate::tokenized::TokenizedCheckoutClient::create_agreement)
//! (and friends) call.
//!
//! We re-export the underlying types here so callers using the
//! `subscriptions` feature can write idiomatic code without importing
//! the tokenized-checkout module:
//!
//! ```
//! use bkash_rs::models::subscriptions::CreateAgreementRequest;
//! ```
//!
//! The flow (mirroring plan §1.7):
//!
//! 1. [`CreateAgreementRequest`] (`mode = "0000"`) →
//!    [`CreateAgreementResponse`] returns a `paymentID`.
//! 2. Customer completes the wallet-side approval.
//! 3. [`ExecuteAgreementRequest`] → [`ExecuteAgreementResponse`] returns
//!    the `agreementID`.
//! 4. Use [`crate::tokenized::TokenizedCheckoutClient::create_payment`]
//!    (`mode = "0001"`) for each recurring charge against the
//!    `agreementID`.
//! 5. Optionally [`AgreementStatusResponse`] to inspect current state, or
//!    [`CancelAgreementResponse`] to revoke the agreement.

pub use crate::models::tokenized::agreement::{
    AgreementStatusResponse, CancelAgreementRequest, CancelAgreementResponse,
    CreateAgreementRequest, CreateAgreementResponse, ExecuteAgreementRequest,
    ExecuteAgreementResponse, QueryAgreementRequest, AGREEMENT_MODE,
};
