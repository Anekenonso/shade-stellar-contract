#![no_std]

mod errors;

use crate::errors::FactoryError;
use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, Address, Bytes, BytesN, Env, IntoVal,
    Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
enum DataKey {
    EscrowWasmHash,
    Escrows,
}

fn get_escrow_wasm_hash(env: &Env) -> BytesN<32> {
    env.storage()
        .persistent()
        .get(&DataKey::EscrowWasmHash)
        .unwrap_or_else(|| panic_with_error!(env, FactoryError::NotInitialized))
}

fn get_escrows(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::Escrows)
        .unwrap_or_else(|| Vec::new(env))
}

#[contract]
pub struct EscrowFactoryContract;

#[contractimpl]
impl EscrowFactoryContract {
    pub fn initialize(env: Env, escrow_wasm_hash: BytesN<32>) {
        if env.storage().persistent().has(&DataKey::EscrowWasmHash) {
            panic_with_error!(&env, FactoryError::AlreadyInitialized);
        }

        env.storage()
            .persistent()
            .set(&DataKey::EscrowWasmHash, &escrow_wasm_hash);
        env.storage()
            .persistent()
            .set(&DataKey::Escrows, &Vec::<Address>::new(&env));
    }

    pub fn deploy_escrow(
        env: Env,
        buyer: Address,
        seller: Address,
        required_amount: i128,
    ) -> Address {
        let wasm_hash = get_escrow_wasm_hash(&env);
        let random: BytesN<32> = env.prng().gen();
        let salt = env
            .crypto()
            .keccak256(&Bytes::from_slice(&env, &random.to_array()));

        let escrow_address = env
            .deployer()
            .with_current_contract(salt)
            .deploy_v2(wasm_hash, ());

        env.invoke_contract::<()>(
            &escrow_address,
            &Symbol::new(&env, "initialize"),
            (buyer, seller, required_amount).into_val(&env),
        );

        let mut escrows = get_escrows(&env);
        escrows.push_back(escrow_address.clone());
        env.storage().persistent().set(&DataKey::Escrows, &escrows);

        escrow_address
    }

    pub fn get_escrows(env: Env) -> Vec<Address> {
        get_escrows(&env)
    }
}
