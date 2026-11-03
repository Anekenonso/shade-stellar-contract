use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum EscrowError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    Overflow = 4,
    Overfunded = 5,
    InvalidStatus = 6,
}
