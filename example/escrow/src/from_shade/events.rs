use soroban_sdk::{contractevent, Address, BytesN, Env, String, Vec};
use super::types::*;
use shade::types::*;

#[contractevent]
pub struct EscrowCreatedEvent {
    pub id: u64,
    pub seller: Address,
    pub buyer: Address,
    pub token: Address,
    pub amount: i128,
    pub invoice_id: Option<u64>,
    pub timestamp: u64,
}

pub fn publish_escrow_created_event(
    env: &Env,
    id: u64,
    seller: Address,
    buyer: Address,
    token: Address,
    amount: i128,
    invoice_id: Option<u64>,
    timestamp: u64,
) {
    EscrowCreatedEvent {
        id,
        seller,
        buyer,
        token,
        amount,
        invoice_id,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct EscrowFundedEvent {
    pub escrow_id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub token: Address,
    pub amount: i128,
    pub timestamp: u64,
}

pub fn publish_escrow_funded_event(
    env: &Env,
    escrow_id: u64,
    buyer: Address,
    seller: Address,
    token: Address,
    amount: i128,
    timestamp: u64,
) {
    EscrowFundedEvent {
        escrow_id,
        buyer,
        seller,
        token,
        amount,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct EscrowRefundedEvent {
    pub escrow_id: u64,
    pub seller: Address,
    pub buyer: Address,
    pub token: Address,
    pub amount: i128,
    pub timestamp: u64,
}

pub fn publish_escrow_refunded_event(
    env: &Env,
    escrow_id: u64,
    seller: Address,
    buyer: Address,
    token: Address,
    amount: i128,
    timestamp: u64,
) {
    EscrowRefundedEvent {
        escrow_id,
        seller,
        buyer,
        token,
        amount,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct EscrowReleasedEvent {
    pub escrow_id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub token: Address,
    pub merchant_amount: i128,
    pub fee: i128,
    pub timestamp: u64,
}

pub fn publish_escrow_released_event(
    env: &Env,
    escrow_id: u64,
    buyer: Address,
    seller: Address,
    token: Address,
    merchant_amount: i128,
    fee: i128,
    timestamp: u64,
) {
    EscrowReleasedEvent {
        escrow_id,
        buyer,
        seller,
        token,
        merchant_amount,
        fee,
        timestamp,
    }
    .publish(env);
}
