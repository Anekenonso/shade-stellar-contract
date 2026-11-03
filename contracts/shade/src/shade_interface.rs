//! Public contract interface.
//!
//! This trait is the single source of truth for the contract's exported
//! functions; `shade.rs` implements it verbatim. Keep the two in sync — adding
//! a method to the impl without declaring it here is a compile error.

use crate::types::{
    Invoice, InvoiceFilter, InvoicePage, Merchant, MerchantAnalytics, MerchantAnalyticsSummary,
    MerchantFilter, MerchantPage, OracleConfig, PendingFee, PlatformFeeSplit, Role, TokenAnalytics,
    Transaction,
};
use soroban_sdk::{contracttrait, Address, BytesN, Env, String, Vec};

#[contracttrait]
pub trait ShadeTrait {
    fn initialize(env: Env, admin: Address);
    fn get_admin(env: Env) -> Address;
    fn add_accepted_token(env: Env, admin: Address, token: Address);
    fn add_accepted_tokens(env: Env, admin: Address, tokens: Vec<Address>);
    fn remove_accepted_token(env: Env, admin: Address, token: Address);
    fn is_accepted_token(env: Env, token: Address) -> bool;
    fn set_account_wasm_hash(env: Env, admin: Address, wasm_hash: soroban_sdk::BytesN<32>);
    fn set_fee(env: Env, admin: Address, token: Address, fee: i128);
    fn get_fee(env: Env, token: Address) -> i128;
    fn set_platform_account(env: Env, admin: Address, account: Address);
    fn get_platform_account(env: Env) -> Address;
    fn set_token_oracle(env: Env, admin: Address, token: Address, oracle: OracleConfig);
    fn get_token_oracle(env: Env, token: Address) -> OracleConfig;
    fn propose_fee(env: Env, admin: Address, token: Address, fee: i128);
    fn execute_fee(env: Env, admin: Address, token: Address);
    fn get_pending_fee(env: Env, token: Address) -> PendingFee;
    fn register_merchant(env: Env, merchant: Address);
    fn get_merchant(env: Env, merchant_id: u64) -> Merchant;
    fn get_merchants(env: Env, filter: MerchantFilter) -> Vec<Merchant>;
    fn is_merchant(env: Env, merchant: Address) -> bool;
    fn set_merchant_status(env: Env, admin: Address, merchant_id: u64, status: bool);
    fn is_merchant_active(env: Env, merchant_id: u64) -> bool;
    fn verify_merchant(env: Env, admin: Address, merchant_id: u64, status: bool);
    fn is_merchant_verified(env: Env, merchant_id: u64) -> bool;
    fn create_invoice(
        env: Env,
        merchant: Address,
        description: String,
        amount: i128,
        token: Address,
        expires_at: Option<u64>,
    ) -> u64;
    fn create_fiat_invoice(
        env: Env,
        merchant: Address,
        description: String,
        fiat_amount: i128,
        fiat_currency: String,
        fiat_decimals: u32,
        token: Address,
        expires_at: Option<u64>,
    ) -> u64;
    fn create_invoice_draft(
        env: Env,
        merchant: Address,
        description: String,
        amount: i128,
        token: Address,
        expires_at: Option<u64>,
    ) -> u64;
    fn finalize_invoice(env: Env, merchant: Address, invoice_id: u64);
    #[allow(clippy::too_many_arguments)]
    fn create_invoice_signed(
        env: Env,
        caller: Address,
        merchant: Address,
        description: String,
        amount: i128,
        token: Address,
        nonce: BytesN<32>,
        signature: BytesN<64>,
    ) -> u64;
    fn get_invoice(env: Env, invoice_id: u64) -> Invoice;
    fn resolve_invoice_amount(env: Env, invoice_id: u64) -> i128;
    fn refund_invoice(env: Env, merchant: Address, invoice_id: u64);
    fn claim_refund(env: Env, buyer: Address, invoice_id: u64);
    fn set_merchant_key(env: Env, merchant: Address, key: BytesN<32>);
    fn get_merchant_key(env: Env, merchant: Address) -> BytesN<32>;
    fn grant_role(env: Env, admin: Address, user: Address, role: Role);
    fn revoke_role(env: Env, admin: Address, user: Address, role: Role);
    fn has_role(env: Env, user: Address, role: Role) -> bool;
    fn get_invoices(env: Env, filter: InvoiceFilter) -> Vec<Invoice>;
    fn refund_invoice_partial(env: Env, merchant: Address, invoice_id: u64, amount: i128);
    fn pause(env: Env, admin: Address);
    fn unpause(env: Env, admin: Address);
    fn is_paused(env: Env) -> bool;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>);
    fn restrict_merchant_account(
        env: Env,
        caller: Address,
        merchant_address: Address,
        status: bool,
    );
    fn calculate_fee(env: Env, merchant: Address, token: Address, amount: i128) -> i128;
    fn compute_platform_fee_split(
        env: Env,
        merchant: Address,
        token: Address,
        amount: i128,
    ) -> PlatformFeeSplit;
    fn set_merchant_platform_fee(
        env: Env,
        caller: Address,
        merchant_id: u64,
        token: Address,
        fee_bps: i128,
    );
    fn get_merchant_platform_fee(env: Env, merchant_id: u64, token: Address) -> Option<i128>;
    fn clear_merchant_platform_fee(env: Env, caller: Address, merchant_id: u64, token: Address);
    fn get_merchant_volume(env: Env, merchant: Address, token: Address) -> i128;
    fn get_merchant_analytics(env: Env, merchant: Address, token: Address) -> MerchantAnalytics;
    fn get_merchant_analytics_summary(env: Env, merchant: Address) -> MerchantAnalyticsSummary;
    fn set_merchant_account(env: Env, merchant: Address, account: Address);
    fn get_merchant_account(env: Env, merchant_id: u64) -> Address;
    fn set_auto_withdrawal_threshold(env: Env, merchant: Address, token: Address, threshold: i128);
    fn get_auto_withdrawal_threshold(env: Env, merchant_id: u64, token: Address) -> Option<i128>;
    fn set_auto_withdrawal_recipient(env: Env, merchant: Address, recipient: Address);
    fn get_auto_withdrawal_recipient(env: Env, merchant_id: u64) -> Option<Address>;
    fn pay_invoice(env: Env, payer: Address, invoice_id: u64);
    fn pay_invoices_batch(env: Env, payer: Address, invoice_ids: Vec<u64>);
    fn pay_invoice_partial(env: Env, payer: Address, invoice_id: u64, amount: i128);
    fn validate_payment_payload(env: Env, payload: crate::types::PaymentPayload);
    fn void_invoice(env: Env, merchant: Address, invoice_id: u64);
    fn amend_invoice(
        env: Env,
        merchant: Address,
        invoice_id: u64,
        new_amount: Option<i128>,
        new_description: Option<String>,
    );
    fn propose_admin_transfer(env: Env, admin: Address, new_admin: Address);
    fn accept_admin_transfer(env: Env, new_admin: Address);
    fn set_merchant_webhook(env: Env, merchant: Address, webhook: String);
    fn get_merchant_webhook(env: Env, merchant_id: u64) -> String;
    fn set_merchant_accepted_tokens(env: Env, merchant: Address, tokens: Vec<Address>);
    fn get_merchant_accepted_tokens(env: Env, merchant: Address) -> Vec<Address>;
    fn remove_merchant_accepted_token(env: Env, merchant: Address, token: Address);
    fn is_token_accepted_for_merchant(env: Env, merchant: Address, token: Address) -> bool;
    fn get_user_transactions(env: Env, user: Address) -> Vec<Transaction>;
    fn get_token_analytics(env: Env, token: Address) -> TokenAnalytics;
    fn get_token_volume(env: Env, token: Address) -> i128;
    fn get_token_dominance_metrics(env: Env, tokens: Vec<Address>) -> Vec<(Address, i128)>;
    fn get_top_tokens_by_volume(env: Env, limit: u32) -> Vec<(Address, i128)>;
    fn get_token_market_share(env: Env, token: Address) -> i128;
    fn search_invoices_paginated(
        env: Env,
        caller: Address,
        filter: InvoiceFilter,
        cursor: u64,
        page_size: u32,
    ) -> InvoicePage;
    fn search_merchants_paginated(
        env: Env,
        filter: MerchantFilter,
        cursor: u64,
        page_size: u32,
    ) -> MerchantPage;
    fn find_merchant_id(env: Env, address: Address) -> u64;
}
