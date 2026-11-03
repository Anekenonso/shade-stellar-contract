#![no_std]

mod errors;

use crate::errors::EscrowError;
use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, panic_with_error, Address, Env,
};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Buyer,
    Seller,
    RequiredAmount,
    DepositedAmount,
    Status,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[contracttype]
#[repr(u32)]
pub enum EscrowStatus {
    Created = 0,
    Funded = 1,
    Released = 2,
}

#[contractevent]
pub struct ReleasedEvent {
    pub buyer: Address,
    pub seller: Address,
    pub amount: i128,
}

fn get_buyer(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Buyer)
        .unwrap_or_else(|| panic_with_error!(env, EscrowError::NotInitialized))
}

fn get_seller(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Seller)
        .unwrap_or_else(|| panic_with_error!(env, EscrowError::NotInitialized))
}

fn get_required_amount(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::RequiredAmount)
        .unwrap_or_else(|| panic_with_error!(env, EscrowError::NotInitialized))
}

fn get_deposited_amount(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::DepositedAmount)
        .unwrap_or_else(|| panic_with_error!(env, EscrowError::NotInitialized))
}

fn get_status(env: &Env) -> EscrowStatus {
    env.storage()
        .instance()
        .get(&DataKey::Status)
        .unwrap_or_else(|| panic_with_error!(env, EscrowError::NotInitialized))
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    pub fn initialize(env: Env, buyer: Address, seller: Address, required_amount: i128) {
        if env.storage().instance().has(&DataKey::Buyer) {
            panic_with_error!(&env, EscrowError::AlreadyInitialized);
        }
        if required_amount <= 0 {
            panic_with_error!(&env, EscrowError::InvalidAmount);
        }

        env.storage().instance().set(&DataKey::Buyer, &buyer);
        env.storage().instance().set(&DataKey::Seller, &seller);
        env.storage()
            .instance()
            .set(&DataKey::RequiredAmount, &required_amount);
        env.storage()
            .instance()
            .set(&DataKey::DepositedAmount, &0i128);
        env.storage()
            .instance()
            .set(&DataKey::Status, &EscrowStatus::Created);
    }

    pub fn deposit(env: Env, amount: i128) {
        if amount <= 0 {
            panic_with_error!(&env, EscrowError::InvalidAmount);
        }

        let status = get_status(&env);
        if status != EscrowStatus::Created {
            panic_with_error!(&env, EscrowError::InvalidStatus);
        }

        let required_amount = get_required_amount(&env);
        let deposited_amount = get_deposited_amount(&env);
        let updated_amount = deposited_amount
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(&env, EscrowError::Overflow));

        if updated_amount > required_amount {
            panic_with_error!(&env, EscrowError::Overfunded);
        }

        env.storage()
            .instance()
            .set(&DataKey::DepositedAmount, &updated_amount);

        if updated_amount == required_amount {
            env.storage()
                .instance()
                .set(&DataKey::Status, &EscrowStatus::Funded);
        }
    }

    pub fn release(env: Env) {
        let buyer = get_buyer(&env);
        let seller = get_seller(&env);
        let status = get_status(&env);
        let deposited_amount = get_deposited_amount(&env);

        buyer.require_auth();

        if status != EscrowStatus::Funded {
            panic_with_error!(&env, EscrowError::InvalidStatus);
        }

        // Minimal MVP behavior: emit release event as transfer signal.
        ReleasedEvent {
            buyer,
            seller,
            amount: deposited_amount,
        }
        .publish(&env);

        env.storage()
            .instance()
            .set(&DataKey::Status, &EscrowStatus::Released);
    }

    pub fn buyer(env: Env) -> Address {
        get_buyer(&env)
    }

    pub fn seller(env: Env) -> Address {
        get_seller(&env)
    }

    pub fn required_amount(env: Env) -> i128 {
        get_required_amount(&env)
    }

    pub fn deposited_amount(env: Env) -> i128 {
        get_deposited_amount(&env)
    }

    pub fn status(env: Env) -> EscrowStatus {
        get_status(&env)
    }
}
