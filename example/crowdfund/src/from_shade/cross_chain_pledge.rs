use soroban_sdk::{panic_with_error, Address, Env, String, Vec};

use crate::{
    errors::ContractError,
    events,
    types::{BridgeKey, CrossChainPledge, CrossChainPledgeStatus},
};

pub fn create_pledge(
    env: &Env,
    source_chain: String,
    source_pledge_id: u64,
    destination_chain: String,
    merchant: Address,
    payer: Address,
    token: Address,
    amount: i128,
    memo: Option<String>,
) -> u64 {
    merchant.require_auth();

    let pledge_id = get_next_pledge_id(env);
    let now = env.ledger().timestamp();

    let pledge = CrossChainPledge {
        id: pledge_id,
        source_chain: source_chain.clone(),
        source_pledge_id,
        destination_chain,
        merchant: merchant.clone(),
        payer,
        token,
        amount,
        status: CrossChainPledgeStatus::Pending,
        created_at: now,
        updated_at: now,
        memo,
    };

    env.storage()
        .persistent()
        .set(&BridgeKey::CrossChainPledge(pledge_id), &pledge);
    env.storage().persistent().set(
        &BridgeKey::PledgeIdBySourceChain(source_chain, source_pledge_id),
        &pledge_id,
    );

    events::publish_cross_chain_pledge_created(env, &pledge);

    pledge_id
}

pub fn update_pledge_status(env: &Env, pledge_id: u64, new_status: CrossChainPledgeStatus) {
    let admin = shade::components::core::get_admin(env);
    admin.require_auth();

    let mut pledge = get_pledge(env, pledge_id);
    pledge.status = new_status;
    pledge.updated_at = env.ledger().timestamp();

    env.storage()
        .persistent()
        .set(&BridgeKey::CrossChainPledge(pledge_id), &pledge);

    events::publish_cross_chain_pledge_updated(env, &pledge);
}

pub fn get_pledge(env: &Env, pledge_id: u64) -> CrossChainPledge {
    env.storage()
        .persistent()
        .get(&BridgeKey::CrossChainPledge(pledge_id))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::NotFound))
}

pub fn get_pledge_by_source(
    env: &Env,
    source_chain: String,
    source_pledge_id: u64,
) -> CrossChainPledge {
    let pledge_id: u64 = env
        .storage()
        .persistent()
        .get(&BridgeKey::PledgeIdBySourceChain(
            source_chain,
            source_pledge_id,
        ))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::NotFound));
    get_pledge(env, pledge_id)
}

fn get_next_pledge_id(env: &Env) -> u64 {
    let count: u64 = env
        .storage()
        .persistent()
        .get(&BridgeKey::CrossChainPledgeCount)
        .unwrap_or(0);
    env.storage()
        .persistent()
        .set(&BridgeKey::CrossChainPledgeCount, &(count + 1));
    count
}

pub fn get_all_pledges(env: &Env) -> Vec<CrossChainPledge> {
    let count: u64 = env
        .storage()
        .persistent()
        .get(&BridgeKey::CrossChainPledgeCount)
        .unwrap_or(0);
    let mut pledges = Vec::new(env);
    for i in 0..count {
        pledges.push_back(get_pledge(env, i));
    }
    pledges
}
