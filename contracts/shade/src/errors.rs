use soroban_sdk::contracterror;

/// Core payment-engine errors: initialization, auth, merchants, invoices, fees,
/// tokens and oracles. Codes are stable: append new variants, never renumber.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    NotAuthorized = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    Reentrancy = 4,
    MerchantAlreadyRegistered = 5,
    MerchantNotFound = 6,
    InvalidAmount = 7,
    InvoiceNotFound = 8,
    ContractPaused = 9,
    ContractNotPaused = 10,
    MerchantKeyNotFound = 11,
    TokenNotAccepted = 12,
    NonceAlreadyUsed = 14,
    InvalidInvoiceStatus = 16,
    RefundPeriodExpired = 17,
    WasmHashNotSet = 18,
    MerchantAccountNotSet = 20,
    InvoiceExpired = 27,
    InvoiceNotPaid = 28,
    PayerNotAvailable = 29,
    InsufficientBalance = 30,
    MerchantNotActive = 32,
    InvalidDescription = 33,
    OracleNotConfigured = 34,
    OraclePriceUnavailable = 35,
    TokenNotAcceptedByMerchant = 41,
    FeeUpdateTooEarly = 42,
    NoPendingFeeUpdate = 43,
    InvalidSwapPath = 44,
    InvalidSlippage = 45,
    /// The invoice's expiry has not passed yet, so its payer cannot claim a refund.
    InvoiceNotExpired = 50,
    /// The invoice has already been fully refunded.
    InvoiceAlreadyRefunded = 51,
}
