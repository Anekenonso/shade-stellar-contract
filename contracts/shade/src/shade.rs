use crate::components::{
    access_control as access_control_component, admin as admin_component,
    analytics as analytics_component, auto_withdrawal as auto_withdrawal_component,
    core as core_component, history as history_component, invoice as invoice_component,
    merchant as merchant_component, pausable as pausable_component,
    platform_fee as platform_fee_component, search as search_component,
    upgrade as upgrade_component,
};
use crate::shade_interface::ShadeTrait;
use crate::types::{
    Invoice, InvoiceFilter, InvoicePage, Merchant, MerchantAnalytics, MerchantAnalyticsSummary,
    MerchantFilter, MerchantPage, OracleConfig, PendingFee, PlatformFeeSplit, Role, TokenAnalytics,
    Transaction,
};
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Vec};

#[contract]
pub struct Shade;

#[contractimpl]
impl ShadeTrait for Shade {
    fn initialize(env: Env, admin: Address) {
        core_component::initialize(&env, &admin);
    }

    fn get_admin(env: Env) -> Address {
        core_component::get_admin(&env)
    }

    fn add_accepted_token(env: Env, admin: Address, token: Address) {
        pausable_component::assert_not_paused(&env);
        admin_component::add_accepted_token(&env, &admin, &token);
    }

    fn add_accepted_tokens(env: Env, admin: Address, tokens: Vec<Address>) {
        pausable_component::assert_not_paused(&env);
        admin_component::add_accepted_tokens(&env, &admin, &tokens);
    }

    fn remove_accepted_token(env: Env, admin: Address, token: Address) {
        pausable_component::assert_not_paused(&env);
        admin_component::remove_accepted_token(&env, &admin, &token);
    }

    fn is_accepted_token(env: Env, token: Address) -> bool {
        admin_component::is_accepted_token(&env, &token)
    }

    fn set_account_wasm_hash(env: Env, admin: Address, wasm_hash: soroban_sdk::BytesN<32>) {
        admin_component::set_account_wasm_hash(&env, &admin, &wasm_hash);
    }

    fn set_fee(env: Env, admin: Address, token: Address, fee: i128) {
        pausable_component::assert_not_paused(&env);
        platform_fee_component::set_fee(&env, &admin, &token, fee);
    }

    fn get_fee(env: Env, token: Address) -> i128 {
        platform_fee_component::get_fee(&env, &token)
    }

    fn set_platform_account(env: Env, admin: Address, account: Address) {
        pausable_component::assert_not_paused(&env);
        admin_component::set_platform_account(&env, &admin, &account);
    }

    fn get_platform_account(env: Env) -> Address {
        admin_component::get_platform_account(&env)
    }

    fn set_token_oracle(env: Env, admin: Address, token: Address, oracle: OracleConfig) {
        pausable_component::assert_not_paused(&env);
        admin_component::set_token_oracle(&env, &admin, &token, &oracle);
    }

    fn get_token_oracle(env: Env, token: Address) -> OracleConfig {
        admin_component::get_token_oracle(&env, &token)
    }

    fn propose_fee(env: Env, admin: Address, token: Address, fee: i128) {
        pausable_component::assert_not_paused(&env);
        platform_fee_component::propose_fee(&env, &admin, &token, fee);
    }

    fn execute_fee(env: Env, admin: Address, token: Address) {
        pausable_component::assert_not_paused(&env);
        platform_fee_component::execute_fee(&env, &admin, &token);
    }

    fn get_pending_fee(env: Env, token: Address) -> PendingFee {
        platform_fee_component::get_pending_fee(&env, &token)
    }

    fn register_merchant(env: Env, merchant: Address) {
        pausable_component::assert_not_paused(&env);
        merchant_component::register_merchant(&env, &merchant);
    }

    fn get_merchant(env: Env, merchant_id: u64) -> Merchant {
        merchant_component::get_merchant(&env, merchant_id)
    }

    fn get_merchants(env: Env, filter: MerchantFilter) -> Vec<Merchant> {
        merchant_component::get_merchants(&env, filter)
    }

    fn is_merchant(env: Env, merchant: Address) -> bool {
        merchant_component::is_merchant(&env, &merchant)
    }

    fn set_merchant_status(env: Env, admin: Address, merchant_id: u64, status: bool) {
        merchant_component::set_merchant_status(&env, &admin, merchant_id, status);
    }

    fn is_merchant_active(env: Env, merchant_id: u64) -> bool {
        merchant_component::is_merchant_active(&env, merchant_id)
    }

    fn verify_merchant(env: Env, admin: Address, merchant_id: u64, status: bool) {
        merchant_component::verify_merchant(&env, &admin, merchant_id, status);
    }

    fn is_merchant_verified(env: Env, merchant_id: u64) -> bool {
        merchant_component::is_merchant_verified(&env, merchant_id)
    }

    fn create_invoice(
        env: Env,
        merchant: Address,
        description: String,
        amount: i128,
        token: Address,
        expires_at: Option<u64>,
    ) -> u64 {
        pausable_component::assert_not_paused(&env);
        invoice_component::create_invoice(&env, &merchant, &description, amount, &token, expires_at)
    }

    fn create_fiat_invoice(
        env: Env,
        merchant: Address,
        description: String,
        fiat_amount: i128,
        fiat_currency: String,
        fiat_decimals: u32,
        token: Address,
        expires_at: Option<u64>,
    ) -> u64 {
        pausable_component::assert_not_paused(&env);
        invoice_component::create_fiat_invoice(
            &env,
            &merchant,
            &description,
            fiat_amount,
            &fiat_currency,
            fiat_decimals,
            &token,
            expires_at,
        )
    }

    fn create_invoice_draft(
        env: Env,
        merchant: Address,
        description: String,
        amount: i128,
        token: Address,
        expires_at: Option<u64>,
    ) -> u64 {
        pausable_component::assert_not_paused(&env);
        invoice_component::create_invoice_draft(
            &env,
            &merchant,
            &description,
            amount,
            &token,
            expires_at,
        )
    }

    fn finalize_invoice(env: Env, merchant: Address, invoice_id: u64) {
        pausable_component::assert_not_paused(&env);
        invoice_component::finalize_invoice(&env, &merchant, invoice_id);
    }

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
    ) -> u64 {
        pausable_component::assert_not_paused(&env);
        invoice_component::create_invoice_signed(
            &env,
            &caller,
            &merchant,
            &description,
            amount,
            &token,
            &nonce,
            &signature,
        )
    }

    fn get_invoice(env: Env, invoice_id: u64) -> Invoice {
        invoice_component::get_invoice(&env, invoice_id)
    }

    fn resolve_invoice_amount(env: Env, invoice_id: u64) -> i128 {
        invoice_component::resolve_invoice_amount(&env, invoice_id)
    }

    fn refund_invoice(env: Env, merchant: Address, invoice_id: u64) {
        pausable_component::assert_not_paused(&env);
        invoice_component::refund_invoice(&env, &merchant, invoice_id);
    }

    fn claim_refund(env: Env, buyer: Address, invoice_id: u64) {
        pausable_component::assert_not_paused(&env);
        invoice_component::claim_refund(&env, &buyer, invoice_id);
    }

    fn set_merchant_key(env: Env, merchant: Address, key: BytesN<32>) {
        merchant_component::set_merchant_key(&env, &merchant, &key);
    }

    fn get_merchant_key(env: Env, merchant: Address) -> BytesN<32> {
        merchant_component::get_merchant_key(&env, &merchant)
    }

    fn grant_role(env: Env, admin: Address, user: Address, role: Role) {
        access_control_component::grant_role(&env, &admin, &user, role);
    }

    fn revoke_role(env: Env, admin: Address, user: Address, role: Role) {
        access_control_component::revoke_role(&env, &admin, &user, role);
    }

    fn has_role(env: Env, user: Address, role: Role) -> bool {
        access_control_component::has_role(&env, &user, role)
    }

    fn get_invoices(env: Env, filter: InvoiceFilter) -> Vec<Invoice> {
        invoice_component::get_invoices(&env, filter)
    }

    fn refund_invoice_partial(env: Env, merchant: Address, invoice_id: u64, amount: i128) {
        pausable_component::assert_not_paused(&env);
        invoice_component::refund_invoice_partial(&env, &merchant, invoice_id, amount);
    }

    fn pause(env: Env, admin: Address) {
        pausable_component::pause(&env, &admin);
    }

    fn unpause(env: Env, admin: Address) {
        pausable_component::unpause(&env, &admin);
    }

    fn is_paused(env: Env) -> bool {
        pausable_component::is_paused(&env)
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        upgrade_component::upgrade(&env, &new_wasm_hash);
    }

    fn restrict_merchant_account(
        env: Env,
        caller: Address,
        merchant_address: Address,
        status: bool,
    ) {
        merchant_component::restrict_merchant_account(&env, &caller, &merchant_address, status);
    }

    fn calculate_fee(env: Env, merchant: Address, token: Address, amount: i128) -> i128 {
        platform_fee_component::calculate_fee(&env, &merchant, &token, amount)
    }

    fn compute_platform_fee_split(
        env: Env,
        merchant: Address,
        token: Address,
        amount: i128,
    ) -> PlatformFeeSplit {
        platform_fee_component::compute_split(&env, &merchant, &token, amount)
    }

    fn set_merchant_platform_fee(
        env: Env,
        caller: Address,
        merchant_id: u64,
        token: Address,
        fee_bps: i128,
    ) {
        pausable_component::assert_not_paused(&env);
        platform_fee_component::set_merchant_platform_fee(
            &env,
            &caller,
            merchant_id,
            &token,
            fee_bps,
        );
    }

    fn get_merchant_platform_fee(env: Env, merchant_id: u64, token: Address) -> Option<i128> {
        platform_fee_component::get_merchant_platform_fee(&env, merchant_id, &token)
    }

    fn clear_merchant_platform_fee(env: Env, caller: Address, merchant_id: u64, token: Address) {
        pausable_component::assert_not_paused(&env);
        platform_fee_component::clear_merchant_platform_fee(&env, &caller, merchant_id, &token);
    }

    fn get_merchant_volume(env: Env, merchant: Address, token: Address) -> i128 {
        analytics_component::get_merchant_volume(&env, &merchant, &token)
    }

    fn get_merchant_analytics(env: Env, merchant: Address, token: Address) -> MerchantAnalytics {
        analytics_component::get_merchant_analytics(&env, &merchant, &token)
    }

    fn get_merchant_analytics_summary(env: Env, merchant: Address) -> MerchantAnalyticsSummary {
        analytics_component::get_merchant_analytics_summary(&env, &merchant)
    }

    fn set_merchant_account(env: Env, merchant: Address, account: Address) {
        merchant_component::set_merchant_account(&env, &merchant, &account);
    }

    fn get_merchant_account(env: Env, merchant_id: u64) -> Address {
        merchant_component::get_merchant_account(&env, merchant_id)
    }

    fn set_auto_withdrawal_threshold(env: Env, merchant: Address, token: Address, threshold: i128) {
        pausable_component::assert_not_paused(&env);
        auto_withdrawal_component::set_auto_withdrawal_threshold(
            &env, &merchant, &token, threshold,
        );
    }

    fn get_auto_withdrawal_threshold(env: Env, merchant_id: u64, token: Address) -> Option<i128> {
        auto_withdrawal_component::get_auto_withdrawal_threshold(&env, merchant_id, &token)
    }

    fn set_auto_withdrawal_recipient(env: Env, merchant: Address, recipient: Address) {
        pausable_component::assert_not_paused(&env);
        auto_withdrawal_component::set_auto_withdrawal_recipient(&env, &merchant, &recipient);
    }

    fn get_auto_withdrawal_recipient(env: Env, merchant_id: u64) -> Option<Address> {
        auto_withdrawal_component::get_auto_withdrawal_recipient(&env, merchant_id)
    }

    fn pay_invoice(env: Env, payer: Address, invoice_id: u64) {
        pausable_component::assert_not_paused(&env);
        invoice_component::pay_invoice(&env, &payer, invoice_id);
    }

    fn pay_invoices_batch(env: Env, payer: Address, invoice_ids: Vec<u64>) {
        pausable_component::assert_not_paused(&env);
        invoice_component::pay_invoices_batch(&env, &payer, &invoice_ids);
    }

    fn pay_invoice_partial(env: Env, payer: Address, invoice_id: u64, amount: i128) {
        pausable_component::assert_not_paused(&env);
        invoice_component::pay_invoice_partial(&env, &payer, invoice_id, amount);
    }

    fn validate_payment_payload(env: Env, payload: crate::types::PaymentPayload) {
        crate::components::payment::validate_payment_payload(&env, &payload);
    }

    fn void_invoice(env: Env, merchant: Address, invoice_id: u64) {
        pausable_component::assert_not_paused(&env);
        invoice_component::void_invoice(&env, &merchant, invoice_id);
    }

    fn amend_invoice(
        env: Env,
        merchant: Address,
        invoice_id: u64,
        new_amount: Option<i128>,
        new_description: Option<String>,
    ) {
        pausable_component::assert_not_paused(&env);
        invoice_component::amend_invoice(&env, &merchant, invoice_id, new_amount, new_description);
    }

    fn propose_admin_transfer(env: Env, admin: Address, new_admin: Address) {
        admin_component::propose_admin_transfer(&env, &admin, &new_admin);
    }

    fn accept_admin_transfer(env: Env, new_admin: Address) {
        admin_component::accept_admin_transfer(&env, &new_admin);
    }

    fn set_merchant_webhook(env: Env, merchant: Address, webhook: String) {
        pausable_component::assert_not_paused(&env);
        merchant_component::set_merchant_webhook(&env, &merchant, &webhook);
    }

    fn get_merchant_webhook(env: Env, merchant_id: u64) -> String {
        merchant_component::get_merchant_webhook(&env, merchant_id)
    }

    fn set_merchant_accepted_tokens(env: Env, merchant: Address, tokens: Vec<Address>) {
        pausable_component::assert_not_paused(&env);
        merchant_component::set_merchant_accepted_tokens(&env, &merchant, &tokens);
    }

    fn get_merchant_accepted_tokens(env: Env, merchant: Address) -> Vec<Address> {
        merchant_component::get_merchant_accepted_tokens(&env, &merchant)
    }

    fn remove_merchant_accepted_token(env: Env, merchant: Address, token: Address) {
        pausable_component::assert_not_paused(&env);
        merchant_component::remove_merchant_accepted_token(&env, &merchant, &token);
    }

    fn is_token_accepted_for_merchant(env: Env, merchant: Address, token: Address) -> bool {
        merchant_component::is_token_accepted_for_merchant(&env, &merchant, &token)
    }

    fn get_user_transactions(env: Env, user: Address) -> Vec<Transaction> {
        history_component::get_user_transactions(&env, user)
    }

    fn get_token_analytics(env: Env, token: Address) -> TokenAnalytics {
        analytics_component::get_token_analytics(&env, &token)
    }

    fn get_token_volume(env: Env, token: Address) -> i128 {
        analytics_component::get_token_volume(&env, &token)
    }

    fn get_token_dominance_metrics(env: Env, tokens: Vec<Address>) -> Vec<(Address, i128)> {
        analytics_component::get_token_dominance_metrics(&env, &tokens)
    }

    fn get_top_tokens_by_volume(env: Env, limit: u32) -> Vec<(Address, i128)> {
        analytics_component::get_top_tokens_by_volume(&env, limit)
    }

    fn get_token_market_share(env: Env, token: Address) -> i128 {
        analytics_component::get_token_market_share(&env, &token)
    }

    // ── On-chain search and filtering utilities ───────────────────────

    fn search_invoices_paginated(
        env: Env,
        caller: Address,
        filter: InvoiceFilter,
        cursor: u64,
        page_size: u32,
    ) -> InvoicePage {
        search_component::search_invoices_paginated(&env, &caller, filter, cursor, page_size)
    }

    fn search_merchants_paginated(
        env: Env,
        filter: MerchantFilter,
        cursor: u64,
        page_size: u32,
    ) -> MerchantPage {
        search_component::search_merchants_paginated(&env, filter, cursor, page_size)
    }

    fn find_merchant_id(env: Env, address: Address) -> u64 {
        merchant_component::find_merchant_id(&env, &address).unwrap_or(0)
    }
}
