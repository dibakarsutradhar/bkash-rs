//! # bKash sandbox smoke test — staged
//!
//! Reads credentials from the environment (load via `set -a; source .env; set +a`)
//! and drives the full tokenized checkout flow in clearly-numbered stages.
//!
//! Subscription stages require the `subscriptions` feature:
//!
//! ```bash
//! cargo run --example from_env --features subscriptions -- sub-create
//! ```
//!
//! Classic (URL-based) Checkout stages require the `checkout` feature:
//!
//! ```bash
//! cargo run --example from_env --features checkout -- checkout-create
//! ```
//!
//! ## Tokenized checkout stages
//!
//! 1. `create-agreement`  -> prints `bkashURL`. Approve on your phone, then
//! 2. `execute-agreement` -> prints `agreementID`.
//! 3. `create-payment`    -> prints `bkashURL`. Approve on your phone, then
//! 4. `execute-payment`   -> prints `trxID`. From here you can also `query-payment`
//!    or `refund`.
//!
//! ## Subscription stages (same wire endpoints as tokenized agreement; the
//! `subscriptions` feature routes through the tokenized subdomain)
//!
//! 1. `sub-create`  -> prints `bkashURL`. Approve on your phone, then
//! 2. `sub-execute` -> prints `agreementID`.
//! 3. `sub-query`   -> prints current status.
//! 4. `sub-cancel`  -> prints cancelled status.
//!
//! ## Classic (URL-based) Checkout stages (requires `--features checkout`)
//!
//! One-shot URL-redirect flow — no agreement step.
//!
//! 1. `checkout-create`  -> prints `bkashURL`. Approve on your phone, then
//! 2. `checkout-execute` -> prints `trxID`.
//! 3. `checkout-query`   -> prints current status.
//! 4. `checkout-search`  -> looks up a transaction by `trxID`.
//! 5. `checkout-refund`  -> refunds a captured payment.
//! 6. `checkout-refund-status` -> queries the status of a refund.
//!
//! ## Running
//!
//! ```bash
//! set -a; source .env; set +a
//!
//! cargo run --example from_env -- create-agreement
//! # open the printed URL on your phone, approve with wallet + PIN + OTP
//!
//! cargo run --example from_env -- execute-agreement --payment-id <ID>
//! cargo run --example from_env -- create-payment --agreement-id <ID>
//! # open the printed URL on your phone, approve again
//!
//! cargo run --example from_env -- execute-payment --payment-id <ID>
//! cargo run --example from_env -- query-payment --payment-id <ID>
//! cargo run --example from_env -- refund --payment-id <ID> --trx-id <TRX> --amount 100.00
//!
//! # subscriptions (requires --features subscriptions)
//! cargo run --example from_env --features subscriptions -- sub-create
//! cargo run --example from_env --features subscriptions -- sub-execute --payment-id <ID>
//! cargo run --example from_env --features subscriptions -- sub-query   --agreement-id <ID>
//! cargo run --example from_env --features subscriptions -- sub-cancel  --agreement-id <ID>
//!
//! # classic (URL-based) checkout (requires --features checkout)
//! cargo run --example from_env --features checkout -- checkout-create
//! cargo run --example from_env --features checkout -- checkout-execute --payment-id <ID>
//! cargo run --example from_env --features checkout -- checkout-query   --payment-id <ID>
//! cargo run --example from_env --features checkout -- checkout-search  --trx-id <TRX>
//! cargo run --example from_env --features checkout -- checkout-refund  --payment-id <ID> --trx-id <TRX> --amount <BDT>
//! cargo run --example from_env --features checkout -- checkout-refund-status --payment-id <ID> --trx-id <TRX>
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "checkout")]
use bkash_rs::models::checkout::{
    CreatePaymentRequest as CheckoutCreatePaymentRequest, RefundRequest as CheckoutRefundRequest,
};
use bkash_rs::models::common::{Currency, Money};
use bkash_rs::models::token::GrantTokenRequest;
use bkash_rs::models::tokenized::{CreateAgreementRequest, CreatePaymentRequest, RefundRequest};
use bkash_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stage = args.first().map(String::as_str).unwrap_or("help");

    match stage {
        "create-agreement" => stage_create_agreement().await,
        "execute-agreement" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            stage_execute_agreement(&payment_id).await
        }
        "create-payment" => {
            let agreement_id = require_flag(&args, "--agreement-id")?;
            stage_create_payment(&agreement_id).await
        }
        "execute-payment" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            stage_execute_payment(&payment_id).await
        }
        "query-payment" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            stage_query_payment(&payment_id).await
        }
        "refund" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            let trx_id = require_flag(&args, "--trx-id")?;
            let amount = require_flag(&args, "--amount")?;
            stage_refund(&payment_id, &trx_id, &amount).await
        }
        "refund-status" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            let trx_id = require_flag(&args, "--trx-id")?;
            stage_refund_status(&payment_id, &trx_id).await
        }
        #[cfg(feature = "subscriptions")]
        "loop-billing" => {
            let agreement_id = require_flag(&args, "--agreement-id")?;
            let cycles: usize = optional_flag(&args, "--cycles", "2")?
                .parse()
                .map_err(|_| "invalid --cycles (expected positive integer)")?;
            let interval_secs: u64 = optional_flag(&args, "--interval-secs", "5")?
                .parse()
                .map_err(|_| "invalid --interval-secs (expected non-negative integer)")?;
            let approve_wait_secs: u64 = optional_flag(&args, "--approve-wait-secs", "20")?
                .parse()
                .map_err(|_| "invalid --approve-wait-secs (expected non-negative integer)")?;
            let amount = optional_flag(&args, "--amount", "1000.00")?;
            stage_loop_billing(
                &agreement_id,
                cycles,
                interval_secs,
                approve_wait_secs,
                &amount,
            )
            .await
        }
        #[cfg(feature = "subscriptions")]
        "sub-create" => stage_sub_create().await,
        #[cfg(feature = "subscriptions")]
        "sub-execute" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            stage_sub_execute(&payment_id).await
        }
        #[cfg(feature = "subscriptions")]
        "sub-query" => {
            let agreement_id = require_flag(&args, "--agreement-id")?;
            stage_sub_query(&agreement_id).await
        }
        #[cfg(feature = "subscriptions")]
        "sub-cancel" => {
            let agreement_id = require_flag(&args, "--agreement-id")?;
            stage_sub_cancel(&agreement_id).await
        }
        #[cfg(feature = "checkout")]
        "checkout-create" => stage_checkout_create().await,
        #[cfg(feature = "checkout")]
        "checkout-execute" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            stage_checkout_execute(&payment_id).await
        }
        #[cfg(feature = "checkout")]
        "checkout-query" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            stage_checkout_query(&payment_id).await
        }
        #[cfg(feature = "checkout")]
        "checkout-search" => {
            let trx_id = require_flag(&args, "--trx-id")?;
            stage_checkout_search(&trx_id).await
        }
        #[cfg(feature = "checkout")]
        "checkout-refund" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            let trx_id = require_flag(&args, "--trx-id")?;
            let amount = require_flag(&args, "--amount")?;
            stage_checkout_refund(&payment_id, &trx_id, &amount).await
        }
        #[cfg(feature = "checkout")]
        "checkout-refund-status" => {
            let payment_id = require_flag(&args, "--payment-id")?;
            let trx_id = require_flag(&args, "--trx-id")?;
            stage_checkout_refund_status(&payment_id, &trx_id).await
        }
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!(
        "bKash sandbox smoke test — staged\n\n\
         Tokenized checkout:\n  \
           create-agreement\n  \
           execute-agreement  --payment-id <ID>\n  \
           create-payment     --agreement-id <ID>\n  \
           execute-payment    --payment-id <ID>\n  \
           query-payment      --payment-id <ID>\n  \
           refund             --payment-id <ID> --trx-id <TRX> --amount <BDT>\n  \
           refund-status      --payment-id <ID> --trx-id <TRX>\n\n\
         Subscriptions:\n  \
           sub-create\n  \
           sub-execute        --payment-id <ID>\n  \
           sub-query          --agreement-id <ID>\n  \
           sub-cancel         --agreement-id <ID>\n\n\
         Classic (URL-based) Checkout:\n  \
           checkout-create\n  \
           checkout-execute   --payment-id <ID>\n  \
           checkout-query     --payment-id <ID>\n  \
           checkout-search    --trx-id <TRX>\n  \
           checkout-refund    --payment-id <ID> --trx-id <TRX> --amount <BDT>\n  \
           checkout-refund-status --payment-id <ID> --trx-id <TRX>\n\n\
         Recurring billing loop:\n  \
           loop-billing       --agreement-id <ID>\n                           [--cycles N=2]\n\
                               [--interval-secs S=5]\n\
                               [--approve-wait-secs A=20]\n\
                               [--amount BDT=1000.00]\n\
         Drives N create_payment -> wait A seconds for wallet approval ->\n\
         execute_payment cycles, sleeping S seconds between cycles.\n\
         KNOWN LIMITATION: bKash's sandbox execute_payment locks each\n\
         paymentID on the first call (2056 if not approved, 2117 forever\n\
         after). No retry path. This loop is best-effort and is NOT a\n\
         production-ready recurring-billing solution. For real billing,\n\
         register a bKash webhook for wallet-side completion signals.\n"
    );
}

fn require_flag(args: &[String], flag: &str) -> Result<String, Box<dyn std::error::Error>> {
    let i = args
        .iter()
        .position(|a| a == flag)
        .ok_or_else(|| format!("missing flag: {flag}"))?;
    args.get(i + 1)
        .cloned()
        .ok_or_else(|| format!("flag {flag} has no value").into())
}

#[cfg_attr(not(feature = "subscriptions"), allow(dead_code))]
fn optional_flag(
    args: &[String],
    flag: &str,
    default: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    match args.iter().position(|a| a == flag) {
        Some(i) => args
            .get(i + 1)
            .cloned()
            .ok_or_else(|| format!("flag {flag} has no value").into()),
        None => Ok(default.to_string()),
    }
}

// -- Shared client construction ------------------------------------------

struct Ctx {
    bkash: Bkash,
    app_key: String,
    callback_url: String,
    payer_reference: String,
    amount_str: String,
}

async fn build_ctx() -> Result<Ctx, Box<dyn std::error::Error>> {
    let app_key = std::env::var("BKASH_APP_KEY")?;
    let app_secret = std::env::var("BKASH_APP_SECRET")?;
    let username = std::env::var("BKASH_USERNAME")?;
    let password = std::env::var("BKASH_PASSWORD")?;

    let environment = match std::env::var("BKASH_ENVIRONMENT")
        .unwrap_or_else(|_| "sandbox".to_string())
        .to_lowercase()
        .as_str()
    {
        "production" | "prod" => Environment::Production,
        _ => Environment::Sandbox,
    };

    let callback_url = std::env::var("BKASH_CALLBACK_URL")
        .unwrap_or_else(|_| "https://merchant.test/callback".to_string());
    let payer_reference =
        std::env::var("BKASH_PAYER_REFERENCE").unwrap_or_else(|_| "demo-payer-001".to_string());
    let amount_str = std::env::var("BKASH_AMOUNT").unwrap_or_else(|_| "100.00".to_string());

    println!(
        "== bKash {} ==",
        if matches!(environment, Environment::Production) {
            "production"
        } else {
            "sandbox"
        }
    );
    println!("username    : {username}");
    println!("amount      : {amount_str} BDT");
    println!();

    let bkash = Bkash::builder()
        .environment(environment)
        .app_key(&app_key)
        .app_secret(&app_secret)
        .username(&username)
        .password(&password)
        .build_and_connect()
        .await?;
    // Smoke-grant so we fail fast if credentials are wrong.
    let _ = bkash
        .tokenized()
        .grant_token(GrantTokenRequest::new(&app_key, &app_secret))
        .await?;

    Ok(Ctx {
        bkash,
        app_key,
        callback_url,
        payer_reference,
        amount_str,
    })
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// -- Stage 1: create_agreement ------------------------------------------

async fn stage_create_agreement() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[1/4 create-agreement]");

    let req = CreateAgreementRequest::new(
        ctx.payer_reference.clone(),
        ctx.callback_url.clone(),
        Money::bdt(&ctx.amount_str),
        Currency::Bdt,
    )
    .with_merchant_invoice_number(format!("AGR-{}", now_secs()));

    let resp = ctx.bkash.tokenized().create_agreement(req).await?;
    println!("  paymentID : {}", resp.payment_id);
    println!("  bkashURL  : {}", resp.bkash_url);
    println!();
    println!(">>> Open the bkashURL above on your phone.");
    println!(">>> Approve with wallet 01770618575, PIN 12121, OTP 123456.");
    println!(
        ">>> Then run:\n    cargo run --example from_env -- execute-agreement --payment-id {}",
        resp.payment_id
    );
    Ok(())
}

// -- Stage 2: execute_agreement ------------------------------------------

async fn stage_execute_agreement(payment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[2/4 execute-agreement] paymentID={payment_id}");

    let resp = ctx.bkash.tokenized().execute_agreement(payment_id).await?;
    println!("  agreementID : {}", resp.agreement_id);
    println!("  payerReference : {}", resp.payer_reference);
    println!();
    println!(
        ">>> Next: cargo run --example from_env -- create-payment --agreement-id {}",
        resp.agreement_id
    );
    // Keep app_key alive so the env-var requirement stays visible if someone
    // refactors the example.
    let _ = ctx.app_key;
    Ok(())
}

// -- Stage 3: create_payment ---------------------------------------------

async fn stage_create_payment(agreement_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[3a/4 create-payment] agreementID={agreement_id}");

    let amount = Money::bdt(&ctx.amount_str);
    let req = CreatePaymentRequest::new(
        agreement_id,
        ctx.payer_reference.clone(),
        ctx.callback_url.clone(),
        amount,
        Currency::Bdt,
    )
    .with_merchant_invoice_number(format!("PAY-{}", now_secs()));

    let resp = ctx.bkash.tokenized().create_payment(req).await?;
    println!("  paymentID : {}", resp.payment_id);
    println!("  bkashURL  : {}", resp.bkash_url);
    println!();
    println!(">>> Open the bkashURL above on your phone.");
    println!(">>> Approve with wallet 01770618575, PIN 12121, OTP 123456.");
    println!(
        ">>> Then run:\n    cargo run --example from_env -- execute-payment --payment-id {}",
        resp.payment_id
    );
    Ok(())
}

// -- Stage 4: execute_payment --------------------------------------------

async fn stage_execute_payment(payment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[3b/4 execute-payment] paymentID={payment_id}");

    let resp = ctx.bkash.tokenized().execute_payment(payment_id).await?;
    println!("  trxID      : {}", resp.trx_id);
    println!("  trxStatus  : {}", resp.transaction_status);
    println!();
    println!(">>> Next: cargo run --example from_env -- query-payment --payment-id {payment_id}");
    println!(
        ">>> Or:    cargo run --example from_env -- refund --payment-id {payment_id} --trx-id {} --amount {}",
        resp.trx_id, ctx.amount_str
    );
    Ok(())
}

// -- Stage 5: query_payment ----------------------------------------------

async fn stage_query_payment(payment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[query-payment] paymentID={payment_id}");

    let resp = ctx.bkash.tokenized().query_payment(payment_id).await?;
    println!("  trxID      : {}", resp.trx_id);
    println!("  trxStatus  : {}", resp.transaction_status);
    println!("  amount     : {:?}", resp.amount);
    println!("  currency   : {:?}", resp.currency);
    Ok(())
}

// -- Stage 6: refund -----------------------------------------------------

async fn stage_refund(
    payment_id: &str,
    trx_id: &str,
    amount: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[refund] paymentID={payment_id} trxID={trx_id} amount={amount}");

    let req = RefundRequest::new(
        payment_id,
        trx_id,
        Money::bdt(amount),
        format!("SKU-{}", now_secs()),
        "Sandbox smoke test refund",
    );
    let resp = ctx.bkash.tokenized().refund(req).await?;
    println!("  originalTrxID : {}", resp.trx_id);
    println!("  refundTrxID   : {}", resp.refund_trx_id);
    println!("  refundAmount  : {:?}", resp.refund_amount);
    Ok(())
}

// -- Stage 7: refund_status ----------------------------------------------

async fn stage_refund_status(
    payment_id: &str,
    trx_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[refund-status] paymentID={payment_id} trxID={trx_id}");

    let resp = ctx
        .bkash
        .tokenized()
        .refund_status(payment_id, trx_id)
        .await?;
    println!("  status : {:?}", resp);
    Ok(())
}

// -- Subscription stages -------------------------------------------------
//
// Subscriptions use the same wire endpoints as the tokenized agreement flow
// (POST /tokenized/checkout/create, /execute, /agreement/status, /agreement/cancel)
// but are routed through the `Subscriptions` product so they appear under
// the merchant's recurring-billing dashboard. The request/response models
// are identical to the tokenized agreement types.

#[cfg(feature = "subscriptions")]
async fn stage_sub_create() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[sub-create]");

    let req = CreateAgreementRequest::new(
        ctx.payer_reference.clone(),
        ctx.callback_url.clone(),
        Money::bdt(&ctx.amount_str),
        Currency::Bdt,
    )
    .with_merchant_invoice_number(format!("SUB-{}", now_secs()));

    let resp = ctx.bkash.subscriptions().create_subscription(req).await?;
    println!("  paymentID : {}", resp.payment_id);
    println!("  bkashURL  : {}", resp.bkash_url);
    println!();
    println!(">>> Open the bkashURL above on your phone.");
    println!(">>> Approve with wallet 01770618575, PIN 12121, OTP 123456.");
    println!(
        ">>> Then run:\n    cargo run --example from_env -- sub-execute --payment-id {}",
        resp.payment_id
    );
    Ok(())
}

#[cfg(feature = "subscriptions")]
async fn stage_sub_execute(payment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[sub-execute] paymentID={payment_id}");

    let resp = ctx
        .bkash
        .subscriptions()
        .execute_subscription(payment_id)
        .await?;
    println!("  agreementID     : {}", resp.agreement_id);
    println!("  agreementStatus : {}", resp.agreement_status);
    println!("  payerReference  : {}", resp.payer_reference);
    println!();
    println!(
        ">>> Next: cargo run --example from_env -- sub-query --agreement-id {}",
        resp.agreement_id
    );
    println!(
        ">>> Or:    cargo run --example from_env -- sub-cancel --agreement-id {}",
        resp.agreement_id
    );
    Ok(())
}

#[cfg(feature = "subscriptions")]
async fn stage_sub_query(agreement_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[sub-query] agreementID={agreement_id}");

    let resp = ctx
        .bkash
        .subscriptions()
        .query_subscription(agreement_id)
        .await?;
    println!("  agreementID       : {}", resp.agreement_id);
    println!("  agreementStatus   : {}", resp.agreement_status);
    println!("  amount            : {:?}", resp.amount);
    println!("  currency          : {:?}", resp.currency);
    println!("  payerReference    : {}", resp.payer_reference);
    println!("  customerMsisdn    : {}", resp.customer_msisdn);
    Ok(())
}

#[cfg(feature = "subscriptions")]
async fn stage_sub_cancel(agreement_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[sub-cancel] agreementID={agreement_id}");

    let resp = ctx
        .bkash
        .subscriptions()
        .cancel_subscription(agreement_id)
        .await?;
    println!("  agreementID     : {}", resp.agreement_id);
    println!("  agreementStatus : {}", resp.agreement_status);
    Ok(())
}

// -- Recurring billing loop ---------------------------------------------
//
// Drives N cycles of `create_payment` -> wait `--approve-wait-secs` for
// wallet approval -> `execute_payment` against an existing subscription
// `agreementID`. Between cycles the program sleeps for `--interval-secs`
// (default 5s) so that the caller can simulate a monthly cadence without
// waiting a real month.
//
// IMPORTANT: bKash locks each paymentID on the first `execute_payment`
// call. If you call too early (customer hasn't approved yet) the server
// returns 2056 Invalid Payment State and the payment is dead — every
// subsequent call returns 2117. So the operator MUST approve on the
// wallet within `--approve-wait-secs` seconds of seeing the bkashURL.

#[cfg(feature = "subscriptions")]
async fn stage_loop_billing(
    agreement_id: &str,
    cycles: usize,
    interval_secs: u64,
    approve_wait_secs: u64,
    amount: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if cycles == 0 {
        return Err("--cycles must be at least 1".into());
    }
    let ctx = build_ctx().await?;
    println!("[loop-billing]");
    println!("  agreementID : {agreement_id}");
    println!("  cycles      : {cycles}");
    println!("  amount/cycle: {amount} BDT");
    println!("  interval    : {interval_secs}s between cycles");
    println!();
    println!("NOTE: each cycle pauses for you to approve a bkashURL on your phone.");
    println!("      Use --interval-secs 2592000 to simulate a real monthly cadence.");
    println!();

    let money = Money::bdt(amount);
    let mut completed: Vec<(usize, String, String)> = Vec::new(); // (cycle, trxID, paymentID)

    for cycle in 1..=cycles {
        println!("=== Cycle {cycle}/{cycles} ===");

        // 1) Create the payment against the agreement
        let create_req = CreatePaymentRequest::new(
            agreement_id,
            ctx.payer_reference.clone(),
            ctx.callback_url.clone(),
            money.clone(),
            Currency::Bdt,
        )
        .with_merchant_invoice_number(format!("BILL-{cycle}-{}", now_secs()));

        let created = ctx.bkash.tokenized().create_payment(create_req).await?;
        println!("  paymentID : {}", created.payment_id);
        println!("  bkashURL  : {}", created.bkash_url);
        println!();
        println!(">>> Approve the bkashURL above on your phone");
        println!(">>> (wallet 01770618575, PIN 12121, OTP 123456)");

        // Wait -- interval_secs seconds for the operator to approve on the
        // wallet before we call execute_payment. This is a poor man's
        // replacement for a webhook. The interval needs to be tuned:
        // too short and you haven't approved yet (2056), too long and
        // the test runs slowly. Use `--approve-wait-secs 30` if you need
        // more time.
        if approve_wait_secs > 0 {
            println!(">>> waiting {approve_wait_secs}s for wallet approval...");
            tokio::time::sleep(std::time::Duration::from_secs(approve_wait_secs)).await;
        }

        // 2) Execute the payment — single shot. bKash locks the payment on
        // first call: too early returns 2056, after that 2117 forever.
        let executed = ctx
            .bkash
            .tokenized()
            .execute_payment(&created.payment_id)
            .await?;
        println!(
            "  -> cycle {cycle} charged: trxID = {}, trxStatus = {}, amount = {}",
            executed.trx_id, executed.transaction_status, executed.amount
        );
        completed.push((cycle, executed.trx_id.clone(), created.payment_id.clone()));

        if cycle < cycles && interval_secs > 0 {
            println!(
                "  sleeping {interval_secs}s before next cycle (simulating next billing period)..."
            );
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
        println!();
    }

    println!("=== Summary ===");
    println!("  agreementID: {agreement_id}");
    println!("  cycles     : {cycles}");
    println!("  amount/cyc : {amount} BDT");
    let amount_num: u64 = amount
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    println!(
        "  total      : {} BDT (estimate)",
        cycles as u64 * amount_num
    );
    println!();
    println!("  {:>4}  {:<22}  paymentID", "cyc", "trxID");
    for (cycle, trx_id, payment_id) in &completed {
        println!("  {cycle:>4}  {trx_id:<22}  {payment_id}");
    }
    Ok(())
}

// -- Classic (URL-based) Checkout stages -------------------------------
//
// The classic product uses the `checkout` subdomain with its own token
// (`POST /checkout/token/grant`) and the `mode = "0011"` create-payment
// payload. There is no agreement step — it's a one-shot URL-redirect
// flow identical in shape to the tokenized product but routed through
// `bkash.checkout()` instead of `bkash.tokenized()`.

#[cfg(feature = "checkout")]
async fn stage_checkout_create() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[checkout-create]");

    let req = CheckoutCreatePaymentRequest::new(
        ctx.payer_reference.clone(),
        ctx.callback_url.clone(),
        Money::bdt(&ctx.amount_str),
        Currency::Bdt,
    )
    .with_merchant_invoice_number(format!("CHK-{}", now_secs()));

    let resp = ctx.bkash.checkout().create_payment(req).await?;
    println!("  paymentID : {}", resp.payment_id);
    println!("  bkashURL  : {}", resp.bkash_url);
    println!();
    println!(">>> Open the bkashURL above on your phone.");
    println!(">>> Approve with wallet 01770618575, PIN 12121, OTP 123456.");
    println!(
        ">>> Then run:\n    cargo run --example from_env --features checkout -- checkout-execute --payment-id {}",
        resp.payment_id
    );
    Ok(())
}

#[cfg(feature = "checkout")]
async fn stage_checkout_execute(payment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[checkout-execute] paymentID={payment_id}");

    let resp = ctx.bkash.checkout().execute_payment(payment_id).await?;
    println!("  paymentID : {}", resp.payment_id);
    println!("  trxID     : {}", resp.trx_id);
    println!("  trxStatus : {}", resp.transaction_status);
    println!("  amount    : {:?}", resp.amount);
    println!();
    println!(
        ">>> Next: cargo run --example from_env --features checkout -- checkout-query --payment-id {payment_id}"
    );
    println!(
        ">>> Or:    cargo run --example from_env --features checkout -- checkout-refund --payment-id {payment_id} --trx-id {} --amount {}",
        resp.trx_id, ctx.amount_str
    );
    Ok(())
}

#[cfg(feature = "checkout")]
async fn stage_checkout_query(payment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[checkout-query] paymentID={payment_id}");

    let resp = ctx.bkash.checkout().query_payment(payment_id).await?;
    println!("  paymentID : {}", resp.payment_id);
    println!("  trxID     : {}", resp.trx_id);
    println!("  trxStatus : {}", resp.transaction_status);
    println!("  amount    : {:?}", resp.amount);
    println!("  currency  : {:?}", resp.currency);
    Ok(())
}

#[cfg(feature = "checkout")]
async fn stage_checkout_search(trx_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[checkout-search] trxID={trx_id}");

    let resp = ctx.bkash.checkout().search_transaction(trx_id).await?;
    println!("  trxID            : {}", resp.trx_id);
    println!("  trxStatus        : {}", resp.transaction_status);
    println!("  amount           : {:?}", resp.amount);
    println!("  saleAmount       : {:?}", resp.sale_amount);
    println!("  customerMsisdn   : {}", resp.customer_msisdn);
    println!("  isCoupon         : {}", resp.is_coupon());
    Ok(())
}

#[cfg(feature = "checkout")]
async fn stage_checkout_refund(
    payment_id: &str,
    trx_id: &str,
    amount: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[checkout-refund] paymentID={payment_id} trxID={trx_id} amount={amount}");

    let req = CheckoutRefundRequest::new(
        payment_id,
        trx_id,
        Money::bdt(amount),
        format!("SKU-{}", now_secs()),
        "Sandbox smoke test refund (classic)",
    );
    let resp = ctx.bkash.checkout().refund(req).await?;
    println!("  originalTrxID : {}", resp.trx_id);
    println!("  refundTrxID   : {}", resp.refund_trx_id);
    println!("  refundAmount  : {:?}", resp.refund_amount);
    Ok(())
}

#[cfg(feature = "checkout")]
async fn stage_checkout_refund_status(
    payment_id: &str,
    trx_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = build_ctx().await?;
    println!("[checkout-refund-status] paymentID={payment_id} trxID={trx_id}");

    let resp = ctx
        .bkash
        .checkout()
        .refund_status(payment_id, trx_id)
        .await?;
    println!("  refundTrxID : {}", resp.refund_trx_id);
    println!("  status      : {:?}", resp);
    Ok(())
}
