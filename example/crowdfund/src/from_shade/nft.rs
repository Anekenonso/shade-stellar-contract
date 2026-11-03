use shade::components::merchant;
use crate::from_shade::errors::{CampaignError, ContractError};
use crate::from_shade::events;
use shade::types::{Merchant};
use crate::from_shade::types::{DataKey, Nft, NftCollection, NftKey, NftStatus};
use soroban_sdk::{panic_with_error, Address, Env, String, Vec};

const MAX_BPS: u32 = 10_000;

pub fn create_nft_collection(
    env: &Env,
    merchant_addr: &Address,
    name: &String,
    base_uri: &String,
    max_supply: u64,
    royalty_bps: u32,
) -> u64 {
    merchant_addr.require_auth();
    if royalty_bps > MAX_BPS {
        panic_with_error!(env, CampaignError::NftError);
    }
    if base_uri.is_empty() {
        panic_with_error!(env, CampaignError::NftError);
    }
    let merchant_id = merchant::get_merchant_id(env, merchant_addr);
    let merchant_record: Merchant = env
        .storage()
        .persistent()
        .get(&shade::types::DataKey::Merchant(merchant_id))
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::MerchantNotFound));
    if !merchant_record.active {
        panic_with_error!(env, shade::errors::ContractError::MerchantNotActive);
    }
    let id: u64 = env
        .storage()
        .persistent()
        .get(&NftKey::NftCollectionCount)
        .unwrap_or(0u64)
        + 1;
    let collection = NftCollection {
        id,
        merchant_id,
        merchant: merchant_addr.clone(),
        name: name.clone(),
        base_uri: base_uri.clone(),
        max_supply,
        minted: 0,
        royalty_bps,
        active: true,
        created_at: env.ledger().timestamp(),
    };
    env.storage()
        .persistent()
        .set(&NftKey::NftCollection(id), &collection);
    env.storage()
        .persistent()
        .set(&NftKey::NftCollectionCount, &id);
    env.storage()
        .persistent()
        .set(&NftKey::CollectionNfts(id), &Vec::<u64>::new(env));
    events::publish_nft_collection_created_event(
        env,
        id,
        merchant_id,
        merchant_addr.clone(),
        name.clone(),
        base_uri.clone(),
        max_supply,
        royalty_bps,
        env.ledger().timestamp(),
    );
    id
}

pub fn mint_nft(
    env: &Env,
    merchant_addr: &Address,
    collection_id: u64,
    recipient: &Address,
    token_uri: &String,
) -> u64 {
    merchant_addr.require_auth();
    mint_nft_inner(env, merchant_addr, collection_id, recipient, token_uri)
}

/// Mint without re-authorizing. Callers must have already required auth for
/// `merchant_addr`; `require_auth` twice in one invocation is an auth error,
/// which is why the batch path uses this directly.
fn mint_nft_inner(
    env: &Env,
    merchant_addr: &Address,
    collection_id: u64,
    recipient: &Address,
    token_uri: &String,
) -> u64 {
    if token_uri.is_empty() {
        panic_with_error!(env, CampaignError::NftError);
    }
    let mut collection: NftCollection = env
        .storage()
        .persistent()
        .get::<_, NftCollection>(&NftKey::NftCollection(collection_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::NftError));
    let merchant_id = merchant::get_merchant_id(env, merchant_addr);
    if collection.merchant_id != merchant_id {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }
    if !collection.active {
        panic_with_error!(env, CampaignError::NftError);
    }
    if collection.max_supply > 0 && collection.minted >= collection.max_supply {
        panic_with_error!(env, CampaignError::NftError);
    }
    let nft_id: u64 = env
        .storage()
        .persistent()
        .get(&NftKey::NftCount)
        .unwrap_or(0u64)
        + 1;
    let nft = Nft {
        id: nft_id,
        collection_id,
        owner: recipient.clone(),
        uri: token_uri.clone(),
        status: NftStatus::Active,
        minted_at: env.ledger().timestamp(),
        recipient: recipient.clone(),
    };
    collection.minted += 1;
    env.storage().persistent().set(&NftKey::Nft(nft_id), &nft);
    env.storage().persistent().set(&NftKey::NftCount, &nft_id);
    env.storage()
        .persistent()
        .set(&NftKey::NftCollection(collection_id), &collection);
    let mut col_nfts: Vec<u64> = env
        .storage()
        .persistent()
        .get(&NftKey::CollectionNfts(collection_id))
        .unwrap_or_else(|| Vec::new(env));
    col_nfts.push_back(nft_id);
    env.storage()
        .persistent()
        .set(&NftKey::CollectionNfts(collection_id), &col_nfts);
    let mut user_nfts: Vec<u64> = env
        .storage()
        .persistent()
        .get(&NftKey::UserNfts(recipient.clone()))
        .unwrap_or_else(|| Vec::new(env));
    user_nfts.push_back(nft_id);
    env.storage()
        .persistent()
        .set(&NftKey::UserNfts(recipient.clone()), &user_nfts);
    events::publish_nft_minted_event(
        env,
        nft_id,
        collection_id,
        merchant_id,
        recipient.clone(),
        token_uri.clone(),
        env.ledger().timestamp(),
    );
    nft_id
}

pub fn batch_mint_nfts(
    env: &Env,
    merchant_addr: &Address,
    collection_id: u64,
    recipients: &Vec<Address>,
    token_uris: &Vec<String>,
) -> Vec<u64> {
    merchant_addr.require_auth();
    if recipients.len() != token_uris.len() {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }
    let mut minted_ids: Vec<u64> = Vec::new(env);
    let count = recipients.len();
    for i in 0..count {
        let id = mint_nft_inner(
            env,
            merchant_addr,
            collection_id,
            &recipients.get(i).unwrap(),
            &token_uris.get(i).unwrap(),
        );
        minted_ids.push_back(id);
    }
    let merchant_id = merchant::get_merchant_id(env, merchant_addr);
    events::publish_nft_batch_minted_event(
        env,
        collection_id,
        merchant_id,
        count,
        env.ledger().timestamp(),
    );
    minted_ids
}

pub fn transfer_nft(env: &Env, from: &Address, to: &Address, nft_id: u64) {
    from.require_auth();
    let mut nft: Nft = env
        .storage()
        .persistent()
        .get::<_, Nft>(&NftKey::Nft(nft_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::NftError));
    if nft.owner != *from {
        panic_with_error!(env, CampaignError::NftError);
    }
    if nft.status == NftStatus::Burned {
        panic_with_error!(env, CampaignError::NftError);
    }
    let collection_id = nft.collection_id;
    nft.owner = to.clone();
    env.storage().persistent().set(&NftKey::Nft(nft_id), &nft);
    let from_nfts: Vec<u64> = env
        .storage()
        .persistent()
        .get(&NftKey::UserNfts(from.clone()))
        .unwrap_or_else(|| Vec::new(env));
    let mut new_from: Vec<u64> = Vec::new(env);
    for i in 0..from_nfts.len() {
        if from_nfts.get(i).unwrap() != nft_id {
            new_from.push_back(from_nfts.get(i).unwrap());
        }
    }
    env.storage()
        .persistent()
        .set(&NftKey::UserNfts(from.clone()), &new_from);
    let mut to_nfts: Vec<u64> = env
        .storage()
        .persistent()
        .get(&NftKey::UserNfts(to.clone()))
        .unwrap_or_else(|| Vec::new(env));
    to_nfts.push_back(nft_id);
    env.storage()
        .persistent()
        .set(&NftKey::UserNfts(to.clone()), &to_nfts);
    events::publish_nft_transferred_event(
        env,
        nft_id,
        collection_id,
        from.clone(),
        to.clone(),
        env.ledger().timestamp(),
    );
}

pub fn burn_nft(env: &Env, owner: &Address, nft_id: u64) {
    owner.require_auth();
    let mut nft: Nft = env
        .storage()
        .persistent()
        .get::<_, Nft>(&NftKey::Nft(nft_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::NftError));
    if nft.owner != *owner {
        panic_with_error!(env, CampaignError::NftError);
    }
    if nft.status == NftStatus::Burned {
        panic_with_error!(env, CampaignError::NftError);
    }
    let collection_id = nft.collection_id;
    nft.status = NftStatus::Burned;
    env.storage().persistent().set(&NftKey::Nft(nft_id), &nft);
    events::publish_nft_burned_event(
        env,
        nft_id,
        collection_id,
        owner.clone(),
        env.ledger().timestamp(),
    );
}

pub fn claim_nft_reward(env: &Env, claimer: &Address, nft_id: u64) {
    claimer.require_auth();
    let claimed_key = NftKey::NftClaimed(nft_id, claimer.clone());
    if env.storage().persistent().has(&claimed_key) {
        panic_with_error!(env, CampaignError::NftError);
    }
    let nft: Nft = env
        .storage()
        .persistent()
        .get::<_, Nft>(&NftKey::Nft(nft_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::NftError));
    if nft.recipient != *claimer {
        panic_with_error!(env, CampaignError::NftError);
    }
    if nft.status == NftStatus::Burned {
        panic_with_error!(env, CampaignError::NftError);
    }
    env.storage().persistent().set(&claimed_key, &true);
    events::publish_nft_reward_claimed_event(
        env,
        nft_id,
        nft.collection_id,
        claimer.clone(),
        env.ledger().timestamp(),
    );
}

pub fn deactivate_nft_collection(env: &Env, merchant_addr: &Address, collection_id: u64) {
    merchant_addr.require_auth();
    let mut collection: NftCollection = env
        .storage()
        .persistent()
        .get::<_, NftCollection>(&NftKey::NftCollection(collection_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::NftError));
    let merchant_id = merchant::get_merchant_id(env, merchant_addr);
    if collection.merchant_id != merchant_id {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }
    collection.active = false;
    env.storage()
        .persistent()
        .set(&NftKey::NftCollection(collection_id), &collection);
    events::publish_nft_collection_deactivated_event(
        env,
        collection_id,
        merchant_addr.clone(),
        env.ledger().timestamp(),
    );
}

pub fn get_nft_collection(env: &Env, collection_id: u64) -> NftCollection {
    env.storage()
        .persistent()
        .get::<_, NftCollection>(&NftKey::NftCollection(collection_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::NftError))
}

pub fn get_nft(env: &Env, nft_id: u64) -> Nft {
    env.storage()
        .persistent()
        .get::<_, Nft>(&NftKey::Nft(nft_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::NftError))
}

pub fn get_collection_nfts(env: &Env, collection_id: u64) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&NftKey::CollectionNfts(collection_id))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_user_nfts(env: &Env, user: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&NftKey::UserNfts(user.clone()))
        .unwrap_or_else(|| Vec::new(env))
}
