use crate::components::{core, reentrancy};
use crate::errors::ContractError;
use crate::events;
use crate::types::{DataKey, OracleConfig};
use soroban_sdk::{panic_with_error, token, Address, Env, Vec};

// TODO: create the functionality for withdrawing revenue by admin.

pub fn add_accepted_token(env: &Env, admin: &Address, token: &Address) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    let mut accepted_tokens = get_accepted_tokens(env);
    if !contains_token(&accepted_tokens, token) {
        let _ = token::Client::new(env, token).symbol();
        accepted_tokens.push_back(token.clone());
        env.storage()
            .persistent()
            .set(&DataKey::AcceptedTokens, &accepted_tokens);
        events::publish_token_added_event(env, token.clone(), env.ledger().timestamp());
    }
    reentrancy::exit(env);
}

pub fn add_accepted_tokens(env: &Env, admin: &Address, tokens: &Vec<Address>) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    let mut accepted_tokens = get_accepted_tokens(env);
    let mut changed = false;
    let timestamp = env.ledger().timestamp();

    for token in tokens.iter() {
        if !contains_token(&accepted_tokens, &token) {
            let _ = token::Client::new(env, &token).symbol();
            accepted_tokens.push_back(token.clone());
            events::publish_token_added_event(env, token.clone(), timestamp);
            changed = true;
        }
    }

    if changed {
        env.storage()
            .persistent()
            .set(&DataKey::AcceptedTokens, &accepted_tokens);
    }
    reentrancy::exit(env);
}

pub fn remove_accepted_token(env: &Env, admin: &Address, token: &Address) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    let accepted_tokens = get_accepted_tokens(env);
    let mut updated_tokens = Vec::new(env);
    let mut removed = false;

    for accepted_token in accepted_tokens.iter() {
        if accepted_token == *token {
            removed = true;
        } else {
            updated_tokens.push_back(accepted_token);
        }
    }

    if removed {
        env.storage()
            .persistent()
            .set(&DataKey::AcceptedTokens, &updated_tokens);
        events::publish_token_removed_event(env, token.clone(), env.ledger().timestamp());
    }
    reentrancy::exit(env);
}

pub fn is_accepted_token(env: &Env, token: &Address) -> bool {
    contains_token(&get_accepted_tokens(env), token)
}

fn contains_token(accepted_tokens: &Vec<Address>, token: &Address) -> bool {
    for accepted_token in accepted_tokens.iter() {
        if accepted_token == *token {
            return true;
        }
    }
    false
}

pub fn set_account_wasm_hash(env: &Env, admin: &Address, wasm_hash: &soroban_sdk::BytesN<32>) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);
    env.storage()
        .persistent()
        .set(&DataKey::AccountWasmHash, wasm_hash);
    events::publish_account_wasm_hash_set_event(
        env,
        admin.clone(),
        wasm_hash.clone(),
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

pub fn set_platform_account(env: &Env, admin: &Address, account: &Address) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);
    env.storage()
        .persistent()
        .set(&DataKey::PlatformAccount, account);
    events::publish_platform_account_set_event(
        env,
        admin.clone(),
        account.clone(),
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

pub fn get_platform_account(env: &Env) -> Address {
    env.storage()
        .persistent()
        .get(&DataKey::PlatformAccount)
        .unwrap_or_else(|| core::get_admin(env))
}

pub fn set_token_oracle(env: &Env, admin: &Address, token: &Address, oracle: &OracleConfig) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    if !is_accepted_token(env, token) {
        panic_with_error!(env, ContractError::TokenNotAccepted);
    }

    env.storage()
        .persistent()
        .set(&DataKey::TokenOracle(token.clone()), oracle);

    events::publish_token_oracle_set_event(
        env,
        admin.clone(),
        token.clone(),
        oracle.contract.clone(),
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

pub fn get_token_oracle(env: &Env, token: &Address) -> OracleConfig {
    env.storage()
        .persistent()
        .get(&DataKey::TokenOracle(token.clone()))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::OracleNotConfigured))
}

pub fn propose_admin_transfer(env: &Env, admin: &Address, new_admin: &Address) {
    core::assert_admin(env, admin);
    env.storage()
        .persistent()
        .set(&DataKey::PendingAdmin, new_admin);
    events::publish_admin_transfer_proposed_event(
        env,
        admin.clone(),
        new_admin.clone(),
        env.ledger().timestamp(),
    );
}

pub fn accept_admin_transfer(env: &Env, new_admin: &Address) {
    new_admin.require_auth();
    let pending: Address = env
        .storage()
        .persistent()
        .get(&DataKey::PendingAdmin)
        .unwrap_or_else(|| panic_with_error!(env, ContractError::NotAuthorized));

    if *new_admin != pending {
        panic_with_error!(env, ContractError::NotAuthorized);
    }

    let old_admin: Address = core::get_admin(env);
    env.storage().persistent().set(&DataKey::Admin, new_admin);
    env.storage().persistent().remove(&DataKey::PendingAdmin);
    events::publish_admin_transfer_accepted_event(
        env,
        old_admin,
        new_admin.clone(),
        env.ledger().timestamp(),
    );
}

pub fn get_accepted_tokens(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::AcceptedTokens)
        .unwrap_or_else(|| Vec::new(env))
}
