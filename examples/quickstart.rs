//! # Quickstart
//!
//! Hard-coded credentials demo for the bKash Sandbox.
//!
//! Run with:
//!
//! ```bash
//! cargo run --example quickstart
//! ```
//!
//! Edit the constants below with your sandbox credentials before running.

use bkash_rs::models::common::{Currency, Money};
use bkash_rs::models::token::GrantTokenRequest;
use bkash_rs::models::tokenized::CreatePaymentRequest;
use bkash_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -- 1. Configure the client -----------------------------------------
    //
    // Replace these values with your own bKash sandbox credentials.
    // App key/secret come from the bKash developer portal; username/password
    // are the merchant-panel credentials.
    const APP_KEY: &str = "your_app_key_here";
    const APP_SECRET: &str = "your_app_secret_here";
    const USERNAME: &str = "your_username_here";
    const PASSWORD: &str = "your_password_here";

    // A previously created agreement ID. Create one through the bKash portal
    // or via `bkash.tokenized().create_agreement(...)` and paste the ID here.
    const AGREEMENT_ID: &str = "your_agreement_id_here";

    println!("Building bkash client...");
    let bkash = Bkash::builder()
        .environment(Environment::Sandbox)
        .app_key(APP_KEY)
        .app_secret(APP_SECRET)
        .username(USERNAME)
        .password(PASSWORD)
        .build_and_connect()
        .await?;
    println!(
        "Client ready: {}",
        bkash.config().environment.base_url(Product::Tokenized)
    );

    // -- 2. Grant an OAuth token -----------------------------------------
    //
    // The transport also handles automatic re-grant on 401, but calling
    // grant_token explicitly is a good first smoke test.
    println!("\n[1/4] Granting token...");
    let token_req = GrantTokenRequest::new(APP_KEY, APP_SECRET);
    let token = bkash.tokenized().grant_token(token_req).await?;
    println!("  -> got token, expires in {}s", token.expires_in);

    // -- 3. Create a payment against an existing agreement ---------------
    println!("\n[2/4] Creating payment...");
    let amount = Money::bdt("100.00");
    let create_req = CreatePaymentRequest::new(
        AGREEMENT_ID,
        "demo-payer-001",
        "https://merchant.test/callback",
        amount.clone(),
        Currency::Bdt,
    )
    .with_merchant_invoice_number(format!("INV-{}", chrono_like_now()));

    let created = bkash.tokenized().create_payment(create_req).await?;
    println!("  -> paymentID = {}", created.payment_id);
    println!("  -> bkashURL  = {}", created.bkash_url);

    // -- 4. Query the payment --------------------------------------------
    println!("\n[3/4] Querying payment...");
    let status = bkash.tokenized().query_payment(&created.payment_id).await?;
    println!("  -> transactionStatus = {}", status.transaction_status);
    println!("  -> trxID             = {}", status.trx_id);

    // -- 5. (Optionally) refund ------------------------------------------
    //
    // Real refunds require a successful payment execution. If you have one,
    // uncomment the block below and provide the trxID returned from
    // `execute_payment()`. You'll also need to add the import at the top:
    //
    //     use bkash_rs::models::tokenized::RefundRequest;
    //
    // let refund_req = RefundRequest::new(
    //     &created.payment_id,
    //     "<trxID-from-execute_payment>",
    //     amount,
    //     "SKU-001",
    //     "Demo refund",
    // );
    // let refund = bkash.tokenized().refund(refund_req).await?;
    // println!("  -> refundTrxID = {}", refund.refund_trx_id);
    let _ = amount; // silence unused-variable warning when the block above is commented

    println!("\nDone.");
    Ok(())
}

/// Tiny epoch-seconds helper so we don't pull in the `chrono`/`time` crates
/// just for an invoice number. Replace with whatever your project uses.
fn chrono_like_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
