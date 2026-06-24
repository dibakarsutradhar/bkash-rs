//! Tokenized Checkout product accessor.
//!
//! Use [`Bkash::tokenized`] to obtain a [`TokenizedCheckoutClient`] that
//! exposes one method per documented bKash endpoint. The client borrows the
//! parent [`Bkash`] and reuses its [`Transport`](crate::transport::Transport)
//! and [`TokenCache`](crate::token::TokenCache).
//!
//! ```no_run
//! use bkash_rs::prelude::*;
//! use bkash_rs::models::tokenized::{CreateAgreementRequest, CreatePaymentRequest};
//!
//! # async fn run() -> Result<(), bkash_rs::Error> {
//! # let bkash: Bkash = todo!();
//! let req = CreateAgreementRequest::new(
//!     "cust-1",
//!     "https://merchant.test/callback",
//!     bkash_rs::models::common::Money::bdt("100.00"),
//!     bkash_rs::models::common::Currency::Bdt,
//! );
//! let resp = bkash.tokenized().create_agreement(req).await?;
//! let _payment_id = resp.payment_id;
//! # Ok(())
//! # }
//! ```

use reqwest::Method;

use crate::client::Bkash;
use crate::config::Product;
use crate::error::Error;
use crate::models::token::{GrantTokenRequest, RefreshTokenRequest, TokenResponse};
use crate::models::tokenized::{
    AgreementStatusResponse, CancelAgreementResponse, CreateAgreementRequest,
    CreateAgreementResponse, CreatePaymentRequest, CreatePaymentResponse, ExecuteAgreementResponse,
    ExecutePaymentResponse, QueryPaymentResponse, RefundRequest, RefundResponse,
    RefundStatusResponse, SearchTransactionResponse,
};
use crate::token::TokenTransport;

/// Endpoints for the bKash Tokenized Checkout product.
///
/// Constructed via [`Bkash::tokenized`]; borrowed from the parent client.
#[derive(Debug, Clone, Copy)]
pub struct TokenizedCheckoutClient<'a> {
    client: &'a Bkash,
}

impl<'a> TokenizedCheckoutClient<'a> {
    /// Construct a client borrowing the given `Bkash`.
    #[must_use]
    pub(crate) fn new(client: &'a Bkash) -> Self {
        Self { client }
    }

    // ===== Token management ===========================================

    /// Grant a new OAuth token.
    ///
    /// The bKash `Tokenized` product uses `POST /checkout/token/grant`. The
    /// client also supports an `Authorization: Basic base64(app_key:app_secret)`
    /// alternative, but bKash's body-based credential flow is what the
    /// SDK uses by default.
    pub async fn grant_token(&self, req: GrantTokenRequest) -> Result<TokenResponse, Error> {
        self.client
            .transport()
            .execute_raw(
                Product::Tokenized,
                Method::POST,
                Product::Tokenized.token_path(),
                Some(&req),
            )
            .await
    }

    /// Refresh an existing OAuth token using a long-lived `refresh_token`.
    pub async fn refresh_token(&self, req: RefreshTokenRequest) -> Result<TokenResponse, Error> {
        self.client
            .transport()
            .execute_raw(
                Product::Tokenized,
                Method::POST,
                Product::Tokenized.token_refresh_path(),
                Some(&req),
            )
            .await
    }

    // ===== Agreement lifecycle =======================================

    /// Create a recurring-billing agreement (`mode = "0000"`).
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # use bkash_rs::models::tokenized::CreateAgreementRequest;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let req = CreateAgreementRequest::new(
    ///     "cust-1",
    ///     "https://merchant.test/callback",
    ///     Money::bdt("100.00"),
    ///     Currency::Bdt,
    /// );
    /// let resp = bkash.tokenized().create_agreement(req).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_agreement(
        &self,
        req: CreateAgreementRequest,
    ) -> Result<CreateAgreementResponse, Error> {
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/create",
                Some(&req),
            )
            .await
    }

    /// Execute an agreement using the `paymentID` returned from
    /// [`create_agreement`](Self::create_agreement).
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.tokenized().execute_agreement("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_agreement(
        &self,
        payment_id: &str,
    ) -> Result<ExecuteAgreementResponse, Error> {
        let req = crate::models::tokenized::ExecuteAgreementRequest::new(payment_id);
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/execute",
                Some(&req),
            )
            .await
    }

    /// Query the current state of an agreement.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.tokenized().query_agreement("AG0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_agreement(
        &self,
        agreement_id: &str,
    ) -> Result<AgreementStatusResponse, Error> {
        let req = crate::models::tokenized::QueryAgreementRequest::new(agreement_id);
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/agreement/status",
                Some(&req),
            )
            .await
    }

    /// Cancel an existing agreement.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.tokenized().cancel_agreement("AG0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_agreement(
        &self,
        agreement_id: &str,
    ) -> Result<CancelAgreementResponse, Error> {
        let req = crate::models::tokenized::CancelAgreementRequest::new(agreement_id);
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/agreement/cancel",
                Some(&req),
            )
            .await
    }

    // ===== Payment lifecycle =========================================

    /// Create a payment against an existing agreement (`mode = "0001"`).
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # use bkash_rs::models::tokenized::CreatePaymentRequest;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let req = CreatePaymentRequest::new(
    ///     "AG0001",
    ///     "cust-1",
    ///     "https://merchant.test/callback",
    ///     Money::bdt("50.00"),
    ///     Currency::Bdt,
    /// );
    /// let resp = bkash.tokenized().create_payment(req).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_payment(
        &self,
        req: CreatePaymentRequest,
    ) -> Result<CreatePaymentResponse, Error> {
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/create",
                Some(&req),
            )
            .await
    }

    /// Execute a payment using the `paymentID` returned from
    /// [`create_payment`](Self::create_payment).
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.tokenized().execute_payment("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_payment(&self, payment_id: &str) -> Result<ExecutePaymentResponse, Error> {
        let req = crate::models::tokenized::ExecutePaymentRequest::new(payment_id);
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/execute",
                Some(&req),
            )
            .await
    }

    /// Query the current state of a payment.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.tokenized().query_payment("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_payment(&self, payment_id: &str) -> Result<QueryPaymentResponse, Error> {
        let req = crate::models::tokenized::QueryPaymentRequest::new(payment_id);
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/payment/status",
                Some(&req),
            )
            .await
    }

    /// Search for a transaction by its bKash `trxID`.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.tokenized().search_transaction("8A00ABCD").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_transaction(
        &self,
        trx_id: &str,
    ) -> Result<SearchTransactionResponse, Error> {
        let req = crate::models::tokenized::SearchTransactionRequest::new(trx_id);
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/general/searchTransaction",
                Some(&req),
            )
            .await
    }

    // ===== Refund lifecycle ==========================================

    /// Refund a captured payment (full or partial). Up to 10 partial refunds
    /// are permitted per transaction.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # use bkash_rs::models::tokenized::RefundRequest;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let req = RefundRequest::new(
    ///     "TR0001",
    ///     "8A00ABCD",
    ///     Money::bdt("25.00"),
    ///     "sku-1",
    ///     "customer-return",
    /// );
    /// let resp = bkash.tokenized().refund(req).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, Error> {
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/payment/refund",
                Some(&req),
            )
            .await
    }

    /// Query the status of a previously-issued refund.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.tokenized().refund_status("TR0001", "8A00ABCD").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refund_status(
        &self,
        payment_id: &str,
        trx_id: &str,
    ) -> Result<RefundStatusResponse, Error> {
        let req = crate::models::tokenized::RefundStatusRequest::new(payment_id, trx_id);
        self.client
            .transport()
            .request(
                Product::Tokenized,
                Method::POST,
                "tokenized/checkout/payment/refund/status",
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
        assert_copy::<TokenizedCheckoutClient<'_>>();
    }
}
