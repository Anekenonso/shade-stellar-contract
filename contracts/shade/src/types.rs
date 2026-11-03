//! Contract data types and persistent storage keys.

use soroban_sdk::{contracttype, Address, BytesN, String, Vec};

/// Core payment-engine storage keys.

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    ContractInfo,
    /// Current contract administrator.
    Admin,
    /// Administrator nominated by `propose_admin_transfer`, pending acceptance.
    PendingAdmin,
    /// Emergency pause flag.
    Paused,
    AcceptedTokens,
    ReentrancyStatus,
    AccountWasmHash,
    PlatformAccount,
    Role(Address, Role),
    UsedNonce(Address, BytesN<32>),
    TokenFee(Address),
    /// Time-locked pending fee update for a token.
    PendingTokenFee(Address),
    /// Per-merchant platform fee override in basis points for a token.
    MerchantPlatformFee(u64, Address),
    TokenOracle(Address),
    // --- Merchants ---
    Merchant(u64),
    MerchantKey(Address),
    MerchantCount,
    MerchantId(Address),
    MerchantTokens(Address),
    MerchantAccount(u64),
    // --- Invoices ---
    Invoice(u64),
    InvoiceCount,
    // --- Analytics & fee discounts ---
    MerchantVolume(Address, Address),
    UserTransactions(Address),
    MerchantAnalytics(Address, Address),
    MerchantAnalyticsSummary(Address),
    TokenAnalytics(Address),
    TokenVolume(Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInfo {
    pub admin: Address,
    pub timestamp: u64,
}

/// Per-token auto-withdrawal trigger for a merchant.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoWithdrawalThreshold {
    pub token: Address,
    pub threshold: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Merchant {
    pub id: u64,
    pub address: Address,
    pub active: bool,
    pub verified: bool,
    pub date_registered: u64,
    pub account: Address,
    pub webhook: String,
    /// Optional recipient for auto-withdrawals. Defaults to the merchant
    /// address when unset.
    pub auto_withdrawal_recipient: Option<Address>,
    /// Per-token auto-withdrawal thresholds.
    pub auto_withdrawal_thresholds: Vec<AutoWithdrawalThreshold>,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum InvoiceStatus {
    Pending = 0,
    Paid = 1,
    Cancelled = 2,
    Refunded = 3,
    PartiallyRefunded = 4,
    PartiallyPaid = 5,
    Draft = 6,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum InvoicePricingMode {
    FixedCrypto = 0,
    FixedFiat = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiatPricing {
    pub currency: String,
    pub amount: i128,
    pub decimals: u32,
}

/// Soroban-compatible optional wrapper for FiatPricing.
/// `Option<FiatPricing>` cannot be used directly inside a `#[contracttype]`
/// struct because the SDK does not implement the required XDR conversions for
/// `Option<T>` where T is a user-defined struct. An explicit enum variant is
/// the idiomatic workaround.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FiatPricingData {
    None,
    Some(FiatPricing),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invoice {
    pub id: u64,
    pub description: String,
    pub amount: i128,
    pub token: Address,
    pub status: InvoiceStatus,
    pub merchant_id: u64,
    pub payer: Option<Address>,
    pub date_created: u64,
    pub date_paid: Option<u64>,
    pub amount_paid: i128,
    pub amount_refunded: i128,
    pub expires_at: Option<u64>,
    pub pricing_mode: InvoicePricingMode,
    pub fiat_pricing: FiatPricingData,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerchantFilter {
    pub is_active: Option<bool>,
    pub is_verified: Option<bool>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoiceFilter {
    pub status: Option<u32>,
    pub merchant: Option<Address>,
    pub min_amount: Option<u128>,
    pub max_amount: Option<u128>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    Admin,
    Manager,
    Operator,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub contract: Address,
    pub price_decimals: u32,
    pub token_decimals: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerchantAnalytics {
    pub merchant: Address,
    pub token: Address,
    pub total_volume: i128,
    pub total_fees: i128,
    pub transaction_count: u64,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerchantAnalyticsSummary {
    pub merchant: Address,
    pub total_volume: i128,
    pub total_fees: i128,
    pub transaction_count: u64,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingFee {
    pub token: Address,
    pub fee: i128,
    pub proposed_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenAnalytics {
    pub token: Address,
    pub total_volume: i128,
    pub total_fees: i128,
    pub transaction_count: u64,
    pub unique_merchants: u64,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TransactionType {
    InvoicePayment = 0,
    SubscriptionCharge = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    pub transaction_type: TransactionType,
    pub ref_id: u64,
    pub amount: i128,
    pub token: Address,
    pub description: String,
    pub date: u64,
    pub merchant_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaymentRoute {
    Direct,
    Swap(SwapRoute),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapRoute {
    pub router: Address,
    pub path: Vec<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformFeeSplit {
    pub gross_amount: i128,
    pub platform_fee: i128,
    pub merchant_amount: i128,
    pub fee_bps_applied: i128,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PlatformFeeRouteKind {
    Invoice = 0,
    Subscription = 1,
    TicketPurchase = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentPayload {
    pub input_token: Address,
    pub settlement_token: Address,
    pub route: PaymentRoute,
    pub max_slippage_bps: Option<u32>,
}

/// A page of results together with cursor metadata for keyset pagination.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageInfo {
    /// Total items returned in this page.
    pub count: u32,
    /// ID of the last item in this page; pass as `cursor` in the next call.
    /// `0` indicates there are no more pages.
    pub next_cursor: u64,
    pub has_next_page: bool,
}

/// A paginated slice of invoices.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoicePage {
    pub items: Vec<Invoice>,
    pub page_info: PageInfo,
}

/// A paginated slice of merchants.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerchantPage {
    pub items: Vec<Merchant>,
    pub page_info: PageInfo,
}
