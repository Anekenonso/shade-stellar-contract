#![cfg(test)]

use shade::shade::Shade;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn setup() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Shade, ());
    let admin = Address::generate(&env);
    let client = shade::shade::ShadeClient::new(&env, &contract_id);
    client.initialize(&admin);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.add_accepted_token(&admin, &token);
    // Register a merchant
    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);
    (env, contract_id, merchant)
}

#[test]
fn test_create_collection_and_mint() {
    let (env, contract_id, merchant) = setup();
    let client = shade::shade::ShadeClient::new(&env, &contract_id);

    let col_id = client.create_nft_collection(
        &merchant,
        &String::from_str(&env, "Genesis"),
        &String::from_str(&env, "ipfs://bafybeig/"),
        &100u64,
        &500u32,
    );
    assert_eq!(col_id, 1);

    let backer = Address::generate(&env);
    let nft_id = client.mint_nft(
        &merchant,
        &col_id,
        &backer,
        &String::from_str(&env, "ipfs://bafybeig/1.json"),
    );
    assert_eq!(nft_id, 1);

    let nft = client.get_nft(&nft_id);
    assert_eq!(nft.owner, backer);
    assert_eq!(nft.collection_id, col_id);

    let user_nfts = client.get_user_nfts(&backer);
    assert_eq!(user_nfts.len(), 1);
}

#[test]
fn test_batch_mint() {
    let (env, contract_id, merchant) = setup();
    let client = shade::shade::ShadeClient::new(&env, &contract_id);

    let col_id = client.create_nft_collection(
        &merchant,
        &String::from_str(&env, "Batch"),
        &String::from_str(&env, "ipfs://abc/"),
        &0u64,
        &0u32,
    );

    let b1 = Address::generate(&env);
    let b2 = Address::generate(&env);
    let mut recipients: Vec<Address> = Vec::new(&env);
    recipients.push_back(b1.clone());
    recipients.push_back(b2.clone());
    let mut uris: Vec<String> = Vec::new(&env);
    uris.push_back(String::from_str(&env, "ipfs://abc/1.json"));
    uris.push_back(String::from_str(&env, "ipfs://abc/2.json"));

    let ids = client.batch_mint_nfts(&merchant, &col_id, &recipients, &uris);
    assert_eq!(ids.len(), 2);

    let col = client.get_nft_collection(&col_id);
    assert_eq!(col.minted, 2);
}

#[test]
fn test_transfer_nft() {
    let (env, contract_id, merchant) = setup();
    let client = shade::shade::ShadeClient::new(&env, &contract_id);

    let col_id = client.create_nft_collection(
        &merchant,
        &String::from_str(&env, "Transfer"),
        &String::from_str(&env, "ipfs://xyz/"),
        &10u64,
        &250u32,
    );
    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let nft_id = client.mint_nft(
        &merchant,
        &col_id,
        &owner,
        &String::from_str(&env, "ipfs://xyz/1.json"),
    );

    client.transfer_nft(&owner, &new_owner, &nft_id);

    let nft = client.get_nft(&nft_id);
    assert_eq!(nft.owner, new_owner);
    assert_eq!(client.get_user_nfts(&owner).len(), 0);
    assert_eq!(client.get_user_nfts(&new_owner).len(), 1);
}

#[test]
fn test_burn_nft() {
    let (env, contract_id, merchant) = setup();
    let client = shade::shade::ShadeClient::new(&env, &contract_id);

    let col_id = client.create_nft_collection(
        &merchant,
        &String::from_str(&env, "Burn"),
        &String::from_str(&env, "ipfs://burn/"),
        &5u64,
        &0u32,
    );
    let owner = Address::generate(&env);
    let nft_id = client.mint_nft(
        &merchant,
        &col_id,
        &owner,
        &String::from_str(&env, "ipfs://burn/1.json"),
    );
    client.burn_nft(&owner, &nft_id);

    let nft = client.get_nft(&nft_id);
    assert_eq!(nft.status, crate::from_shade::types::NftStatus::Burned);
}

#[test]
fn test_deactivate_collection() {
    let (env, contract_id, merchant) = setup();
    let client = shade::shade::ShadeClient::new(&env, &contract_id);

    let col_id = client.create_nft_collection(
        &merchant,
        &String::from_str(&env, "Deactivate"),
        &String::from_str(&env, "ipfs://deact/"),
        &0u64,
        &0u32,
    );
    client.deactivate_nft_collection(&merchant, &col_id);
    let col = client.get_nft_collection(&col_id);
    assert!(!col.active);
}
