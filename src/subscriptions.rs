//! Subscriptions product accessor.
//!
//! Use [`Bkash::subscriptions`] to obtain a [`SubscriptionsClient`] that
//! exposes one method per bKash subscription-agreement endpoint. The
//! client borrows the parent [`Bkash`] and reuses its
//! [`Transport`](crate::transport::Transport) and
//! [`TokenCache`](crate::token::TokenCache).
//!
//! bKash's Subscriptions product is built on top of the tokenized-checkout
//! agreement infrastructure: a customer grants a one-time authorization
//! for recurring payments, and the merchant can then charge against the
//! resulting `agreementID` (e.g. via
//! [`crate::tokenized::TokenizedCheckoutClient::create_payment`],
//! `mode = "0001"`). On the wire, the request and response shapes are
//! identical to the tokenized-checkout agreement types.
//!
//! ```no_run
//! use bkash_rs::prelude::*;
//! use bkash_rs::models::subscriptions::CreateAgreementRequest;
//!
//! # async fn run() -> Result<(), bkash_rs::Error> {
//! # let bkash: Bkash = todo!();
//! let req = CreateAgreementRequest::new(
//!     "cust-1",
//!     "https://merchant.test/callback",
//!     Money::bdt("100.00"),
//!     Currency::Bdt,
//! );
//! let resp = bkash.subscriptions().create_subscription(req).await?;
//! # Ok(())
//! # }
//! ```

use reqwest::Method;

use crate::client::Bkash;
use crate::config::Product;
use crate::error::Error;
use crate::models::subscriptions::{
    AgreementStatusResponse, CancelAgreementRequest, CancelAgreementResponse,
    CreateAgreementRequest, CreateAgreementResponse, ExecuteAgreementRequest,
    ExecuteAgreementResponse, QueryAgreementRequest,
};

/// Endpoints for the bKash Subscriptions product.
///
/// Constructed via [`Bkash::subscriptions`]; borrowed from the parent
/// client.
#[derive(Debug, Clone, Copy)]
pub struct SubscriptionsClient<'a> {
    client: &'a Bkash,
}

impl<'a> SubscriptionsClient<'a> {
    /// Construct a client borrowing the given `Bkash`.
    #[must_use]
    pub(crate) fn new(client: &'a Bkash) -> Self {
        Self { client }
    }

    /// Create a subscription agreement (one-time authorization for
    /// recurring payments; `mode = "0000"`).
    ///
    /// Returns a `paymentID` and a `bkashURL` that the customer is
    /// redirected to for wallet-side approval.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # use bkash_rs::models::subscriptions::CreateAgreementRequest;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let req = CreateAgreementRequest::new(
    ///     "cust-1",
    ///     "https://merchant.test/callback",
    ///     Money::bdt("100.00"),
    ///     Currency::Bdt,
    /// );
    /// let resp = bkash.subscriptions().create_subscription(req).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_subscription(
        &self,
        req: CreateAgreementRequest,
    ) -> Result<CreateAgreementResponse, Error> {
        self.client
            .transport()
            .request(
                Product::Subscriptions,
                Method::POST,
                "tokenized/checkout/create",
                Some(&req),
            )
            .await
    }

    /// Execute a subscription agreement using the `paymentID` returned
    /// from [`create_subscription`](Self::create_subscription).
    ///
    /// After this call, the agreement is active and can be charged via
    /// recurring payments.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.subscriptions().execute_subscription("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_subscription(
        &self,
        payment_id: &str,
    ) -> Result<ExecuteAgreementResponse, Error> {
        let req = ExecuteAgreementRequest::new(payment_id);
        self.client
            .transport()
            .request(
                Product::Subscriptions,
                Method::POST,
                "tokenized/checkout/execute",
                Some(&req),
            )
            .await
    }

    /// Query the current state of a subscription agreement.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.subscriptions().query_subscription("AG0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_subscription(
        &self,
        agreement_id: &str,
    ) -> Result<AgreementStatusResponse, Error> {
        let req = QueryAgreementRequest::new(agreement_id);
        self.client
            .transport()
            .request(
                Product::Subscriptions,
                Method::POST,
                "tokenized/checkout/agreement/status",
                Some(&req),
            )
            .await
    }

    /// Cancel a subscription agreement.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.subscriptions().cancel_subscription("AG0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_subscription(
        &self,
        agreement_id: &str,
    ) -> Result<CancelAgreementResponse, Error> {
        let req = CancelAgreementRequest::new(agreement_id);
        self.client
            .transport()
            .request(
                Product::Subscriptions,
                Method::POST,
                "tokenized/checkout/agreement/cancel",
                Some(&req),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_is_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<SubscriptionsClient<'_>>();
    }
}
