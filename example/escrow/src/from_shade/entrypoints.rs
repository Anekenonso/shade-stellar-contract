//! The entry points these modules exposed on the Shade hub, kept for reference.

use super::types::*;
use shade::types::*;
use soroban_sdk::{Address, BytesN, Env, String, Vec};

pub struct HubEntrypoints;

impl HubEntrypoints {
    pub fn create_escrow(
        env: Env,
        seller: Address,
        buyer: Address,
        token: Address,
        amount: i128,
        invoice_id: Option<u64>,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::escrow::create_escrow(&env, &seller, &buyer, &token, amount, invoice_id)
    }

    pub fn get_escrow(env: Env, escrow_id: u64) -> Escrow {
        crate::from_shade::escrow::get_escrow(&env, escrow_id)
    }

    pub fn fund_escrow(env: Env, buyer: Address, escrow_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::escrow::fund_escrow(&env, &buyer, escrow_id)
    }

    pub fn release_escrow(env: Env, buyer: Address, escrow_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::escrow::release_escrow(&env, &buyer, escrow_id)
    }

    pub fn refund_escrow(env: Env, seller: Address, escrow_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::escrow::refund_escrow(&env, &seller, escrow_id)
    }
}
