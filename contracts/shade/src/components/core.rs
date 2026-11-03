use crate::errors::ContractError;
use crate::events;
use crate::types::{ContractInfo, DataKey};
use soroban_sdk::{panic_with_error, Address, Env};

pub fn initialize(env: &Env, admin: &Address) {
    if env.storage().persistent().has(&DataKey::Admin) {
        panic_with_error!(env, ContractError::AlreadyInitialized);
    }
    let contract_info = ContractInfo {
        admin: admin.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.storage().persistent().set(&DataKey::Admin, admin);
    env.storage()
        .persistent()
        .set(&DataKey::PlatformAccount, admin);
    env.storage()
        .persistent()
        .set(&DataKey::ContractInfo, &contract_info);
    events::publish_initialized_event(env, admin.clone(), env.ledger().timestamp());
}

pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .persistent()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, ContractError::NotInitialized))
}

pub fn assert_admin(env: &Env, admin: &Address) {
    admin.require_auth();
    if *admin != get_admin(env) {
        panic_with_error!(env, ContractError::NotAuthorized);
    }
}
