use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAuthorized = 3,
    InsufficientBalance = 4,
    AccountRestricted = 5,
    WithdrawalRequestNotFound = 6,
    InvalidWithdrawalStatus = 7,
    InvalidThreshold = 8,
}
