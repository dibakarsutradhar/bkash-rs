//! Classic (URL-based) Checkout models.
//!
//! URL-based Checkout is a single, one-shot payment flow with no
//! agreement. The endpoint inventory for this product:
//!
//! | Operation        | Method | Endpoint                                                |
//! |------------------|--------|---------------------------------------------------------|
//! | Grant Token      | POST   | `/checkout/token/grant`                                 |
//! | Refresh Token    | POST   | `/tokenized/checkout/token/refresh`                     |
//! | Create Payment   | POST   | `/tokenized/checkout/create` (`mode = "0011"`)          |
//! | Execute Payment  | POST   | `/tokenized/checkout/execute`                           |
//! | Query Payment    | POST   | `/tokenized/checkout/payment/status`                    |
//! | Search Tx        | POST   | `/tokenized/checkout/general/searchTransaction`         |
//! | Refund           | POST   | `/tokenized/checkout/payment/refund`                    |
//! | Refund Status    | POST   | `/tokenized/checkout/payment/refund/status`             |
//!
//! Each submodule covers a logical group of endpoints. See the
//! [`crate::checkout`] module for the high-level client.

pub mod payment;
pub mod refund;
pub mod search;

pub use payment::{
    CreatePaymentRequest, CreatePaymentResponse, ExecutePaymentRequest, ExecutePaymentResponse,
    QueryPaymentRequest, QueryPaymentResponse, PAYMENT_MODE,
};
pub use refund::{RefundRequest, RefundResponse, RefundStatusRequest, RefundStatusResponse};
pub use search::{SearchTransactionRequest, SearchTransactionResponse};
