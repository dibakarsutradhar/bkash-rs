//! Classic (URL-based) Checkout product accessor.
//!
//! Use [`Bkash::checkout`] to obtain a [`CheckoutClient`] that exposes one
//! method per documented bKash endpoint. The client borrows the parent
//! [`Bkash`] and reuses its [`Transport`](crate::transport::Transport) and
//! [`TokenCache`](crate::token::TokenCache).
//!
//! URL-based Checkout is the classic one-shot flow: create → customer
//! approval → execute → (optionally) query / refund. Unlike the
//! [tokenized](crate::tokenized) product, there is **no agreement step**.
//!
//! ```no_run
//! use bkash_rs::prelude::*;
//! use bkash_rs::models::checkout::CreatePaymentRequest;
//!
//! # async fn run() -> Result<(), bkash_rs::Error> {
//! # let bkash: Bkash = todo!();
//! let req = CreatePaymentRequest::new(
//!     "cust-1",
//!     "https://merchant.test/callback",
//!     Money::bdt("100.00"),
//!     Currency::Bdt,
//! );
//! let resp = bkash.checkout().create_payment(req).await?;
//! let _payment_id = resp.payment_id;
//! # Ok(())
//! # }
//! ```

use reqwest::Method;

use crate::client::Bkash;
use crate::config::Product;
use crate::error::Error;
use crate::models::checkout::{
    CreatePaymentRequest, CreatePaymentResponse, ExecutePaymentRequest, ExecutePaymentResponse,
    QueryPaymentRequest, QueryPaymentResponse, RefundRequest, RefundResponse, RefundStatusRequest,
    RefundStatusResponse, SearchTransactionRequest, SearchTransactionResponse,
};
use crate::models::token::{GrantTokenRequest, RefreshTokenRequest, TokenResponse};
use crate::token::TokenTransport;

/// Endpoints for the bKash URL-based Checkout product.
///
/// Constructed via [`Bkash::checkout`]; borrowed from the parent client.
#[derive(Debug, Clone, Copy)]
pub struct CheckoutClient<'a> {
    client: &'a Bkash,
}

impl<'a> CheckoutClient<'a> {
    /// Construct a client borrowing the given `Bkash`.
    #[must_use]
    pub(crate) fn new(client: &'a Bkash) -> Self {
        Self { client }
    }

    // ===== Token management ===========================================

    /// Grant a new OAuth token.
    ///
    /// The bKash Checkout (and Auth & Capture) product uses
    /// `POST /checkout/token/grant` on the `checkout` subdomain.
    pub async fn grant_token(&self, req: GrantTokenRequest) -> Result<TokenResponse, Error> {
        self.client
            .transport()
            .execute_raw(
                Product::Checkout,
                Method::POST,
                Product::Checkout.token_path(),
                Some(&req),
            )
            .await
    }

    /// Refresh an existing OAuth token using a long-lived `refresh_token`.
    pub async fn refresh_token(&self, req: RefreshTokenRequest) -> Result<TokenResponse, Error> {
        self.client
            .transport()
            .execute_raw(
                Product::Checkout,
                Method::POST,
                Product::Checkout.token_refresh_path(),
                Some(&req),
            )
            .await
    }

    // ===== Payment lifecycle =========================================

    /// Create a URL-based checkout payment (`mode = "0011"`).
    ///
    /// Returns a `paymentID` and a `bkashURL` that the customer is
    /// redirected to. The `paymentID` is valid for 24 hours and for one
    /// execution only.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # use bkash_rs::models::checkout::CreatePaymentRequest;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let req = CreatePaymentRequest::new(
    ///     "cust-1",
    ///     "https://merchant.test/callback",
    ///     Money::bdt("50.00"),
    ///     Currency::Bdt,
    /// );
    /// let resp = bkash.checkout().create_payment(req).await?;
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
                Product::Checkout,
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
    /// let resp = bkash.checkout().execute_payment("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_payment(&self, payment_id: &str) -> Result<ExecutePaymentResponse, Error> {
        let req = ExecutePaymentRequest::new(payment_id);
        self.client
            .transport()
            .request(
                Product::Checkout,
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
    /// let resp = bkash.checkout().query_payment("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_payment(&self, payment_id: &str) -> Result<QueryPaymentResponse, Error> {
        let req = QueryPaymentRequest::new(payment_id);
        self.client
            .transport()
            .request(
                Product::Checkout,
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
    /// let resp = bkash.checkout().search_transaction("8A00ABCD").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_transaction(
        &self,
        trx_id: &str,
    ) -> Result<SearchTransactionResponse, Error> {
        let req = SearchTransactionRequest::new(trx_id);
        self.client
            .transport()
            .request(
                Product::Checkout,
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
    /// # use bkash_rs::models::checkout::RefundRequest;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let req = RefundRequest::new(
    ///     "TR0001",
    ///     "8A00ABCD",
    ///     Money::bdt("25.00"),
    ///     "sku-1",
    ///     "customer-return",
    /// );
    /// let resp = bkash.checkout().refund(req).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, Error> {
        self.client
            .transport()
            .request(
                Product::Checkout,
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
    /// let resp = bkash.checkout().refund_status("TR0001", "8A00ABCD").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refund_status(
        &self,
        payment_id: &str,
        trx_id: &str,
    ) -> Result<RefundStatusResponse, Error> {
        let req = RefundStatusRequest::new(payment_id, trx_id);
        self.client
            .transport()
            .request(
                Product::Checkout,
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
        assert_copy::<CheckoutClient<'_>>();
    }
}
