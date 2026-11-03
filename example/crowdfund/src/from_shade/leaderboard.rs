use crate::from_shade::errors::{CampaignError, ContractError};
use crate::from_shade::events::publish_leaderboard_updated_event;
use crate::from_shade::types::{CampaignKey, DataKey, DonorInfo};
use soroban_sdk::{panic_with_error, Address, Env, Vec};

const MAX_TOP_DONORS: u32 = 10;

pub fn init_campaign(env: &Env, merchant: Address, campaign_id: u64) {
    merchant.require_auth();

    // Verify merchant is actually registered
    if !env
        .storage()
        .persistent()
        .has(&shade::types::DataKey::MerchantId(merchant.clone()))
    {
        panic_with_error!(env, shade::errors::ContractError::MerchantNotFound);
    }

    let owner_key = CampaignKey::CampaignOwner(campaign_id);
    if env.storage().persistent().has(&owner_key) {
        panic_with_error!(env, shade::errors::ContractError::AlreadyInitialized);
    }

    env.storage().persistent().set(&owner_key, &merchant);

    let top_donors: Vec<DonorInfo> = Vec::new(env);
    env.storage()
        .persistent()
        .set(&CampaignKey::CampaignTopDonors(campaign_id), &top_donors);
}

pub fn track_donation(
    env: &Env,
    merchant: Address,
    campaign_id: u64,
    donor: Address,
    amount: i128,
) {
    merchant.require_auth();

    let owner_key = CampaignKey::CampaignOwner(campaign_id);
    let owner: Address = env
        .storage()
        .persistent()
        .get(&owner_key)
        .unwrap_or_else(|| {
            panic_with_error!(env, CampaignError::CampaignNotFound);
        });

    if owner != merchant {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    if amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    let amount_key = CampaignKey::CampaignDonorAmount(campaign_id, donor.clone());
    let mut current_total: i128 = env.storage().persistent().get(&amount_key).unwrap_or(0);
    current_total += amount;

    env.storage().persistent().set(&amount_key, &current_total);

    // Update top donors leaderboard
    update_top_donors(env, campaign_id, donor.clone(), current_total);

    publish_leaderboard_updated_event(
        env,
        campaign_id,
        donor,
        amount,
        current_total,
        env.ledger().timestamp(),
    );
}

pub fn get_top_donors(env: &Env, campaign_id: u64) -> Vec<DonorInfo> {
    let top_key = CampaignKey::CampaignTopDonors(campaign_id);
    env.storage()
        .persistent()
        .get(&top_key)
        .unwrap_or_else(|| Vec::new(env))
}

fn update_top_donors(env: &Env, campaign_id: u64, donor: Address, new_total: i128) {
    let top_key = CampaignKey::CampaignTopDonors(campaign_id);
    let mut top_donors: Vec<DonorInfo> = env
        .storage()
        .persistent()
        .get(&top_key)
        .unwrap_or_else(|| Vec::new(env));

    // Remove the donor if they are already in the list
    let mut index_to_remove = None;
    for (i, d) in top_donors.iter().enumerate() {
        if d.donor == donor {
            index_to_remove = Some(i as u32);
            break;
        }
    }

    if let Some(index) = index_to_remove {
        top_donors.remove(index);
    }

    // Insert sorted (descending order)
    let mut insert_index = top_donors.len();
    for (i, d) in top_donors.iter().enumerate() {
        if new_total > d.total_donated {
            insert_index = i as u32;
            break;
        }
    }

    top_donors.insert(
        insert_index,
        DonorInfo {
            donor,
            total_donated: new_total,
        },
    );

    // Truncate
    if top_donors.len() > MAX_TOP_DONORS {
        top_donors.pop_back();
    }

    env.storage().persistent().set(&top_key, &top_donors);
}
