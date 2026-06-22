//! Authorization & Capture product accessor.
//!
//! Use [`Bkash::auth_capture`] to obtain an [`AuthCaptureClient`] that
//! exposes one method per documented bKash Auth & Capture endpoint. The
//! client borrows the parent [`Bkash`] and reuses its
//! [`Transport`](crate::transport::Transport) and
//! [`TokenCache`](crate::token::TokenCache).
//!
//! Auth & Capture is a **delayed-capture reservation model** (see plan
//! §1.9): create → execute (authorize) → query (status `Authorized`) →
//! capture (commit) or void (cancel). It is distinct from the
//! [tokenized](crate::tokenized) and [checkout](crate::checkout) products
//! in that:
//!
//! - the create-payment endpoint is
//!   `/tokenized/checkout/payment/create` (not `/tokenized/checkout/create`),
//! - the execute-payment endpoint takes the `paymentID` as a **path
//!   param** (`/tokenized/checkout/execute/{paymentID}`),
//! - the search-transaction endpoint is a **GET** to
//!   `/checkout/payment/search/{trxID}`.
//!
//! ```no_run
//! use bkash_rs::prelude::*;
//! use bkash_rs::models::auth_capture::CreatePaymentRequest;
//!
//! # async fn run() -> Result<(), bkash_rs::Error> {
//! # let bkash: Bkash = todo!();
//! let req = CreatePaymentRequest::new(
//!     "cust-1",
//!     "https://merchant.test/callback",
//!     Money::bdt("100.00"),
//!     Currency::Bdt,
//! );
//! let resp = bkash.auth_capture().create_payment(req).await?;
//! let _payment_id = resp.payment_id;
//! # Ok(())
//! # }
//! ```

use reqwest::Method;

use crate::client::Bkash;
use crate::config::Product;
use crate::error::Error;
use crate::models::auth_capture::{
    CaptureRequest, CaptureResponse, CreatePaymentRequest, CreatePaymentResponse,
    ExecutePaymentResponse, QueryPaymentRequest, QueryPaymentResponse, SearchTransactionResponse,
    VoidRequest, VoidResponse,
};
use crate::models::token::{GrantTokenRequest, RefreshTokenRequest, TokenResponse};
use crate::token::TokenTransport;

/// Endpoints for the bKash Auth & Capture product.
///
/// Constructed via [`Bkash::auth_capture`]; borrowed from the parent client.
#[derive(Debug, Clone, Copy)]
pub struct AuthCaptureClient<'a> {
    client: &'a Bkash,
}

impl<'a> AuthCaptureClient<'a> {
    /// Construct a client borrowing the given `Bkash`.
    #[must_use]
    pub(crate) fn new(client: &'a Bkash) -> Self {
        Self { client }
    }

    // ===== Token management ===========================================

    /// Grant a new OAuth token.
    ///
    /// The bKash Auth & Capture product uses the same
    /// `POST /checkout/token/grant` endpoint as the URL-based Checkout
    /// product (it lives on the `checkout` subdomain).
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # use bkash_rs::models::token::GrantTokenRequest;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let req = GrantTokenRequest::new("test-app-key", "test-app-secret");
    /// let resp = bkash.auth_capture().grant_token(req).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn grant_token(&self, req: GrantTokenRequest) -> Result<TokenResponse, Error> {
        self.client
            .transport()
            .execute_raw(
                Product::AuthCapture,
                Method::POST,
                Product::AuthCapture.token_path(),
                Some(&req),
            )
            .await
    }

    /// Refresh an existing OAuth token using a long-lived `refresh_token`.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # use bkash_rs::models::token::RefreshTokenRequest;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let req = RefreshTokenRequest::new("test-app-key", "test-app-secret", "old-refresh");
    /// let resp = bkash.auth_capture().refresh_token(req).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refresh_token(&self, req: RefreshTokenRequest) -> Result<TokenResponse, Error> {
        self.client
            .transport()
            .execute_raw(
                Product::AuthCapture,
                Method::POST,
                Product::AuthCapture.token_refresh_path(),
                Some(&req),
            )
            .await
    }

    // ===== Payment lifecycle (reservation model) =====================

    /// Create a payment in authorization-only mode
    /// (`mode = "0011"`, `intent = "authorization"`).
    ///
    /// This is a **reservation**: the funds are not yet captured. Returns
    /// a `paymentID` and a `bkashURL` that the customer is redirected to
    /// for wallet approval.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # use bkash_rs::models::auth_capture::CreatePaymentRequest;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let req = CreatePaymentRequest::new(
    ///     "cust-1",
    ///     "https://merchant.test/callback",
    ///     Money::bdt("50.00"),
    ///     Currency::Bdt,
    /// );
    /// let resp = bkash.auth_capture().create_payment(req).await?;
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
                Product::AuthCapture,
                Method::POST,
                "tokenized/checkout/payment/create",
                Some(&req),
            )
            .await
    }

    /// Execute the payment, completing authorization.
    ///
    /// After this call returns, the funds are reserved and the status
    /// (queried via [`query_payment`](Self::query_payment)) should be
    /// [`TransactionStatus::Authorized`](crate::models::common::TransactionStatus::Authorized).
    ///
    /// **Note:** unlike other products, the `paymentID` is **not** in the
    /// request body — it is interpolated into the URL path
    /// (`/tokenized/checkout/execute/{paymentID}`).
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.auth_capture().execute_payment("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_payment(&self, payment_id: &str) -> Result<ExecutePaymentResponse, Error> {
        let path = format!("tokenized/checkout/execute/{payment_id}");
        self.client
            .transport()
            .request_path(Product::AuthCapture, Method::POST, path, None::<&()>)
            .await
    }

    /// Query the current payment status.
    ///
    /// In the reservation flow, the expected statuses are `"Initiated"`
    /// (the customer has not yet completed the wallet flow — retry from
    /// the create step), `"Authorized"` (funds are reserved — proceed to
    /// capture or void), or `"Completed"` (the funds have already been
    /// captured).
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.auth_capture().query_payment("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_payment(&self, payment_id: &str) -> Result<QueryPaymentResponse, Error> {
        let req = QueryPaymentRequest::new(payment_id);
        self.client
            .transport()
            .request(
                Product::AuthCapture,
                Method::POST,
                "tokenized/checkout/payment/status",
                Some(&req),
            )
            .await
    }

    /// Capture (commit) a previously-authorized payment.
    ///
    /// If no response is found from the capture API, **retry** this call
    /// (per §1.9) until the operation is successful.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.auth_capture().capture("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn capture(&self, payment_id: &str) -> Result<CaptureResponse, Error> {
        let req = CaptureRequest::new(payment_id);
        self.client
            .transport()
            .request(
                Product::AuthCapture,
                Method::POST,
                "tokenized/checkout/payment/confirm/capture",
                Some(&req),
            )
            .await
    }

    /// Void (cancel) a previously-authorized payment.
    ///
    /// Use this when the service will not be provided. If no response is
    /// found from the void API, **retry** this call (per §1.9) until the
    /// operation is successful.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.auth_capture().void("TR0001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn void(&self, payment_id: &str) -> Result<VoidResponse, Error> {
        let req = VoidRequest::new(payment_id);
        self.client
            .transport()
            .request(
                Product::AuthCapture,
                Method::POST,
                "tokenized/checkout/payment/confirm/capture/void",
                Some(&req),
            )
            .await
    }

    /// Search for a transaction by its bKash `trxID`.
    ///
    /// **Note:** this is a **GET** request (not POST), and the `trxID` is
    /// interpolated into the URL path
    /// (`/checkout/payment/search/{trxID}`), not passed in the body.
    ///
    /// ```no_run
    /// # use bkash_rs::prelude::*;
    /// # async fn run() -> Result<(), bkash_rs::Error> {
    /// # let bkash: Bkash = todo!();
    /// let resp = bkash.auth_capture().search_transaction("8A00ABCD").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_transaction(
        &self,
        trx_id: &str,
    ) -> Result<SearchTransactionResponse, Error> {
        let path = format!("checkout/payment/search/{trx_id}");
        self.client
            .transport()
            .request_path(Product::AuthCapture, Method::GET, path, None::<&()>)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_is_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<AuthCaptureClient<'_>>();
    }
}
