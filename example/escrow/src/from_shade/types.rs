use soroban_sdk::{contracttype, Address, BytesN, String, Vec};
use shade::types::*;

/// Storage keys that moved out of the hub's `DataKey`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Escrow(u64),
    EscrowCount,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum EscrowStatus {
    Created = 0,
    Funded = 1,
    Released = 2,
    Refunded = 3,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub token: Address,
    pub amount: i128,
    pub status: EscrowStatus,
    pub invoice_id: Option<u64>,
    pub date_created: u64,
    pub date_funded: Option<u64>,
    pub date_released: Option<u64>,
}
