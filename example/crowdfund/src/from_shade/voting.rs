use shade::components::core;
use crate::from_shade::errors::{CampaignError, ContractError};
use crate::from_shade::events;
use crate::from_shade::types::{CampaignKey, DynamicHardCapConfig, HardCapVote, HardCapVoting, VoteDirection, VotingStatus};
use soroban_sdk::{panic_with_error, Address, Env};

pub fn initiate_hard_cap_voting(
    env: &Env,
    crowdfund_id: u64,
    proposed_cap: i128,
    voting_duration: u64,
) {
    let _caller = env.current_contract_address();
    if proposed_cap <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    if voting_duration == 0 {
        panic_with_error!(env, ContractError::InvalidInterval);
    }

    let now = env.ledger().timestamp();
    let voting_end = now.saturating_add(voting_duration);

    let config = HardCapVoting {
        crowdfund_id,
        current_cap: 0,
        proposed_cap,
        voting_start: now,
        voting_end,
        votes_for: 0,
        votes_against: 0,
        status: VotingStatus::Active,
    };

    env.storage()
        .persistent()
        .set(&CampaignKey::HardCapVoting(crowdfund_id), &config);

    events::publish_hard_cap_voting_initiated_event(
        env,
        crowdfund_id,
        proposed_cap,
        voting_duration,
        voting_end,
        now,
    );
}

pub fn get_hard_cap_voting(env: &Env, crowdfund_id: u64) -> HardCapVoting {
    env.storage()
        .persistent()
        .get(&CampaignKey::HardCapVoting(crowdfund_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::VotingNotFound))
}

pub fn vote_on_hard_cap(env: &Env, voter: Address, crowdfund_id: u64, support: bool) {
    voter.require_auth();

    let mut voting = get_hard_cap_voting(env, crowdfund_id);
    let now = env.ledger().timestamp();

    if voting.status != VotingStatus::Active {
        panic_with_error!(env, CampaignError::VotingNotActive);
    }

    if now > voting.voting_end {
        voting.status = VotingStatus::Failed;
        env.storage()
            .persistent()
            .set(&CampaignKey::HardCapVoting(crowdfund_id), &voting);
        panic_with_error!(env, CampaignError::VotingNotActive);
    }

    let has_voted: bool = env
        .storage()
        .persistent()
        .has(&CampaignKey::HardCapVote(crowdfund_id, voter.clone()));

    if has_voted {
        panic_with_error!(env, CampaignError::AlreadyVoted);
    }

    let vote = HardCapVote {
        crowdfund_id,
        voter: voter.clone(),
        proposed_cap: voting.proposed_cap,
        direction: if support {
            VoteDirection::Increase
        } else {
            VoteDirection::Decrease
        },
        created_at: now,
    };

    env.storage().persistent().set(
        &CampaignKey::HardCapVote(crowdfund_id, voter.clone()),
        &vote,
    );

    if support {
        voting.votes_for = voting.votes_for.saturating_add(1);
    } else {
        voting.votes_against = voting.votes_against.saturating_add(1);
    }

    env.storage()
        .persistent()
        .set(&CampaignKey::HardCapVoting(crowdfund_id), &voting);

    events::publish_hard_cap_voted_event(env, crowdfund_id, voter, voting.proposed_cap, now);
}

pub fn finalize_hard_cap_voting(env: &Env, admin: Address, crowdfund_id: u64) {
    admin.require_auth();
    let contract_admin = core::get_admin(env);
    if admin != contract_admin {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    let mut voting = get_hard_cap_voting(env, crowdfund_id);
    let now = env.ledger().timestamp();

    if voting.status != VotingStatus::Active {
        panic_with_error!(env, shade::errors::ContractError::InvalidInvoiceStatus);
    }

    let _total_votes = voting.votes_for.saturating_add(voting.votes_against);
    let votes_passed = voting.votes_for > voting.votes_against;

    if votes_passed {
        voting.status = VotingStatus::Passed;
        let cap_config = DynamicHardCapConfig {
            crowdfund_id,
            hard_cap: voting.proposed_cap,
            voting_duration: voting.voting_end.saturating_sub(voting.voting_start),
            min_votes_required: 1,
            last_updated: now,
        };

        env.storage()
            .persistent()
            .set(&CampaignKey::DynamicHardCap(crowdfund_id), &cap_config);

        events::publish_dynamic_hard_cap_updated_event(
            env,
            crowdfund_id,
            voting.proposed_cap,
            voting.current_cap,
            now,
        );
    } else {
        voting.status = VotingStatus::Failed;
    }

    env.storage()
        .persistent()
        .set(&CampaignKey::HardCapVoting(crowdfund_id), &voting);

    events::publish_hard_cap_voting_finalized_event(
        env,
        crowdfund_id,
        voting.votes_for,
        voting.votes_against,
        votes_passed,
        if votes_passed {
            voting.proposed_cap
        } else {
            voting.current_cap
        },
        now,
    );
}

pub fn get_dynamic_hard_cap(env: &Env, crowdfund_id: u64) -> DynamicHardCapConfig {
    env.storage()
        .persistent()
        .get(&CampaignKey::DynamicHardCap(crowdfund_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::VotingNotFound))
}

pub fn get_crowdfund_hard_cap(env: &Env, crowdfund_id: u64) -> i128 {
    match env
        .storage()
        .persistent()
        .get::<_, DynamicHardCapConfig>(&CampaignKey::DynamicHardCap(crowdfund_id))
    {
        Some(config) => config.hard_cap,
        None => 0,
    }
}
