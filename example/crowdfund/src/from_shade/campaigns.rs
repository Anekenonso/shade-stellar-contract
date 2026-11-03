use shade::components::admin;
use shade::components::merchant;
use shade::components::reentrancy;
use crate::from_shade::errors::{CampaignError, ContractError};
use crate::from_shade::events;
use crate::from_shade::types::{Campaign, CampaignCategory, CampaignFilter, CampaignKey, CampaignTag};
use soroban_sdk::{panic_with_error, Address, Env, String, Vec};

/// Validation bounds for free-form user strings. Kept conservative to minimise
/// Soroban rent/cpu overhead on event emission and storage.
const MAX_NAME_LEN: u32 = 64;
const MAX_DESCRIPTION_LEN: u32 = 512;
const MAX_TITLE_LEN: u32 = 128;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_category_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&CampaignKey::CampaignCategoryCount)
        .unwrap_or(0)
}

fn get_tag_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&CampaignKey::CampaignTagCount)
        .unwrap_or(0)
}

fn get_campaign_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&CampaignKey::CampaignCount)
        .unwrap_or(0)
}

/// Push `value` onto the `Vec<u64>` stored under `list_key` if it isn't
/// already present. Returns true iff the value was added.
fn push_unique_u64(env: &Env, list_key: &CampaignKey, value: u64) -> bool {
    let mut list: Vec<u64> = env
        .storage()
        .persistent()
        .get(list_key)
        .unwrap_or_else(|| Vec::new(env));
    for v in list.iter() {
        if v == value {
            return false;
        }
    }
    list.push_back(value);
    env.storage().persistent().set(list_key, &list);
    true
}

// ── Category management (#352) ────────────────────────────────────────────────

pub fn create_category(env: &Env, admin: &Address, name: &String, description: &String) -> u64 {
    reentrancy::enter(env);
    shade::components::core::assert_admin(env, admin);

    if name.is_empty() || name.len() > MAX_NAME_LEN {
        panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
    }
    if description.len() > MAX_DESCRIPTION_LEN {
        panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
    }

    let name_key = CampaignKey::CampaignCategoryName(name.clone());
    if env.storage().persistent().has(&name_key) {
        panic_with_error!(env, CampaignError::CampaignCategoryAlreadyExists);
    }

    let id = get_category_count(env) + 1;
    let category = CampaignCategory {
        id,
        name: name.clone(),
        description: description.clone(),
        active: true,
        timestamp: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&CampaignKey::CampaignCategory(id), &category);
    env.storage()
        .persistent()
        .set(&CampaignKey::CampaignCategoryCount, &id);
    env.storage().persistent().set(&name_key, &id);
    env.storage()
        .persistent()
        .set(&CampaignKey::CategoryCampaigns(id), &Vec::<u64>::new(env));

    events::publish_campaign_category_created_event(
        env,
        id,
        admin.clone(),
        name.clone(),
        description.clone(),
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
    id
}

#[allow(clippy::too_many_arguments)]
pub fn update_category(
    env: &Env,
    admin: &Address,
    category_id: u64,
    name: Option<String>,
    description: Option<String>,
    active: Option<bool>,
) {
    reentrancy::enter(env);
    shade::components::core::assert_admin(env, admin);

    let key = CampaignKey::CampaignCategory(category_id);
    if !env.storage().persistent().has(&key) {
        panic_with_error!(env, CampaignError::CampaignCategoryNotFound);
    }
    let mut category: CampaignCategory = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignCategoryNotFound));

    if let Some(new_name) = name.as_ref() {
        if new_name.is_empty() || new_name.len() > MAX_NAME_LEN {
            panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
        }
        if new_name != &category.name {
            let name_key = CampaignKey::CampaignCategoryName(new_name.clone());
            if env.storage().persistent().has(&name_key) {
                panic_with_error!(env, CampaignError::CampaignCategoryAlreadyExists);
            }
            env.storage()
                .persistent()
                .remove(&CampaignKey::CampaignCategoryName(category.name.clone()));
            env.storage().persistent().set(&name_key, &category_id);
            category.name = new_name.clone();
        }
    }
    if let Some(new_desc) = description.as_ref() {
        if new_desc.len() > MAX_DESCRIPTION_LEN {
            panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
        }
        category.description = new_desc.clone();
    }
    if let Some(active_flag) = active {
        category.active = active_flag;
    }

    env.storage().persistent().set(&key, &category);

    events::publish_campaign_category_updated_event(
        env,
        category_id,
        admin.clone(),
        category.name.clone(),
        category.description.clone(),
        category.active,
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

pub fn get_category(env: &Env, category_id: u64) -> CampaignCategory {
    let key = CampaignKey::CampaignCategory(category_id);
    if !env.storage().persistent().has(&key) {
        panic_with_error!(env, CampaignError::CampaignCategoryNotFound);
    }
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignCategoryNotFound))
}

pub fn get_categories(env: &Env) -> Vec<CampaignCategory> {
    let count = get_category_count(env);
    let mut out: Vec<CampaignCategory> = Vec::new(env);
    for i in 1..=count {
        if let Some(cat) = env
            .storage()
            .persistent()
            .get::<_, CampaignCategory>(&CampaignKey::CampaignCategory(i))
        {
            out.push_back(cat);
        }
    }
    out
}

// ── Tag management (#352) ─────────────────────────────────────────────────────

pub fn create_tag(env: &Env, creator: &Address, name: &String) -> u64 {
    creator.require_auth();

    // Check merchant membership first to avoid the storage-cost of resolving
    // the admin on the common path.
    if !merchant::is_merchant(env, creator) && shade::components::core::get_admin(env) != *creator {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    if name.is_empty() || name.len() > MAX_NAME_LEN {
        panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
    }

    let name_key = CampaignKey::CampaignTagName(name.clone());
    if env.storage().persistent().has(&name_key) {
        panic_with_error!(env, CampaignError::CampaignTagAlreadyExists);
    }

    let id = get_tag_count(env) + 1;
    let tag = CampaignTag {
        id,
        name: name.clone(),
        creator: creator.clone(),
        timestamp: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&CampaignKey::CampaignTag(id), &tag);
    env.storage()
        .persistent()
        .set(&CampaignKey::CampaignTagCount, &id);
    env.storage().persistent().set(&name_key, &id);
    env.storage()
        .persistent()
        .set(&CampaignKey::TagCampaigns(id), &Vec::<u64>::new(env));

    events::publish_campaign_tag_created_event(
        env,
        id,
        creator.clone(),
        name.clone(),
        env.ledger().timestamp(),
    );
    id
}

pub fn get_tag(env: &Env, tag_id: u64) -> CampaignTag {
    let key = CampaignKey::CampaignTag(tag_id);
    if !env.storage().persistent().has(&key) {
        panic_with_error!(env, CampaignError::CampaignTagNotFound);
    }
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignTagNotFound))
}

pub fn get_tags(env: &Env) -> Vec<CampaignTag> {
    let count = get_tag_count(env);
    let mut out: Vec<CampaignTag> = Vec::new(env);
    for i in 1..=count {
        if let Some(tag) = env
            .storage()
            .persistent()
            .get::<_, CampaignTag>(&CampaignKey::CampaignTag(i))
        {
            out.push_back(tag);
        }
    }
    out
}

// ── Campaign management (#352) ────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn create_campaign(
    env: &Env,
    merchant_addr: &Address,
    title: &String,
    description: &String,
    category_id: u64,
    tags: &Vec<u64>,
    goal_amount: i128,
    token: &Address,
    deadline: u64,
) -> u64 {
    merchant_addr.require_auth();

    if !merchant::is_merchant(env, merchant_addr) {
        panic_with_error!(env, shade::errors::ContractError::MerchantNotFound);
    }
    let merchant_id = merchant::get_merchant_id(env, merchant_addr);
    if !merchant::is_merchant_active(env, merchant_id) {
        panic_with_error!(env, shade::errors::ContractError::MerchantNotActive);
    }

    if title.is_empty() || title.len() > MAX_TITLE_LEN {
        panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
    }
    if description.len() > MAX_DESCRIPTION_LEN {
        panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
    }
    if goal_amount <= 0 {
        panic_with_error!(env, CampaignError::InvalidCampaignGoal);
    }
    if deadline <= env.ledger().timestamp() {
        panic_with_error!(env, CampaignError::InvalidCampaignDeadline);
    }
    if !admin::is_accepted_token(env, token) {
        panic_with_error!(env, shade::errors::ContractError::TokenNotAccepted);
    }

    let category_key = CampaignKey::CampaignCategory(category_id);
    if !env.storage().persistent().has(&category_key) {
        panic_with_error!(env, CampaignError::CampaignCategoryNotFound);
    }
    let category: CampaignCategory = env
        .storage()
        .persistent()
        .get(&category_key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignCategoryNotFound));
    if !category.active {
        panic_with_error!(env, CampaignError::CampaignCategoryInactive);
    }

    // Validate tags + de-dupe per campaign.
    let mut deduped_tags: Vec<u64> = Vec::new(env);
    for tag_id in tags.iter() {
        let key = CampaignKey::CampaignTag(tag_id);
        if !env.storage().persistent().has(&key) {
            panic_with_error!(env, CampaignError::CampaignTagNotFound);
        }
        let mut found = false;
        for existing in deduped_tags.iter() {
            if existing == tag_id {
                found = true;
                break;
            }
        }
        if !found {
            deduped_tags.push_back(tag_id);
        }
    }

    let campaign_id = get_campaign_count(env) + 1;
    let campaign = Campaign {
        id: campaign_id,
        merchant_id,
        merchant: merchant_addr.clone(),
        title: title.clone(),
        description: description.clone(),
        category_id,
        tags: deduped_tags.clone(),
        goal_amount,
        token: token.clone(),
        deadline,
        raised_amount: 0,
        active: true,
        created_at: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&CampaignKey::Campaign(campaign_id), &campaign);
    env.storage()
        .persistent()
        .set(&CampaignKey::CampaignCount, &campaign_id);
    // `push_unique_u64` treats a missing index as empty, so no seeding write is
    // needed here — and an unconditional one would reset the index on every
    // create, leaving `get_merchant_campaigns` reporting only the newest.
    push_unique_u64(
        env,
        &CampaignKey::MerchantCampaigns(merchant_id),
        campaign_id,
    );
    push_unique_u64(
        env,
        &CampaignKey::CategoryCampaigns(category_id),
        campaign_id,
    );
    env.storage()
        .persistent()
        .set(&CampaignKey::CampaignTagList(campaign_id), &deduped_tags);
    for tag_id in deduped_tags.iter() {
        push_unique_u64(env, &CampaignKey::TagCampaigns(tag_id), campaign_id);
    }

    events::publish_campaign_created_event(
        env,
        campaign_id,
        merchant_addr.clone(),
        merchant_id,
        title.clone(),
        description.clone(),
        category_id,
        deduped_tags.clone(),
        goal_amount,
        token.clone(),
        deadline,
        env.ledger().timestamp(),
    );

    campaign_id
}

/// Update only the mutable text fields of a campaign. Goal/token/deadline are
/// immutable after creation to keep the published fundraising target stable.
#[allow(clippy::too_many_arguments)]
pub fn update_campaign(
    env: &Env,
    merchant_addr: &Address,
    campaign_id: u64,
    title: Option<String>,
    description: Option<String>,
) {
    merchant_addr.require_auth();

    let key = CampaignKey::Campaign(campaign_id);
    if !env.storage().persistent().has(&key) {
        panic_with_error!(env, CampaignError::CampaignNotFound);
    }
    let mut campaign: Campaign = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignNotFound));

    if campaign.merchant != *merchant_addr {
        panic_with_error!(env, CampaignError::NotCampaignMerchant);
    }

    if let Some(new_title) = title.as_ref() {
        if new_title.is_empty() || new_title.len() > MAX_TITLE_LEN {
            panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
        }
        campaign.title = new_title.clone();
    }
    if let Some(new_desc) = description.as_ref() {
        if new_desc.len() > MAX_DESCRIPTION_LEN {
            panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
        }
        campaign.description = new_desc.clone();
    }

    env.storage().persistent().set(&key, &campaign);
    events::publish_campaign_updated_event(
        env,
        campaign_id,
        merchant_addr.clone(),
        campaign.title.clone(),
        campaign.description.clone(),
        env.ledger().timestamp(),
    );
}

/// Toggle campaign active state. Deactivated campaigns cannot accept new tag
/// edits or contributions, but their existing data is preserved.
pub fn set_campaign_active(env: &Env, merchant_addr: &Address, campaign_id: u64, active: bool) {
    merchant_addr.require_auth();

    let key = CampaignKey::Campaign(campaign_id);
    if !env.storage().persistent().has(&key) {
        panic_with_error!(env, CampaignError::CampaignNotFound);
    }
    let mut campaign: Campaign = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignNotFound));
    if campaign.merchant != *merchant_addr {
        panic_with_error!(env, CampaignError::NotCampaignMerchant);
    }
    if campaign.active == active {
        return;
    }
    campaign.active = active;
    env.storage().persistent().set(&key, &campaign);
    events::publish_campaign_status_changed_event(
        env,
        campaign_id,
        merchant_addr.clone(),
        active,
        env.ledger().timestamp(),
    );
}

/// Attach an existing tag to a campaign. De-duplicated; reverse index updated.
pub fn add_campaign_tag(env: &Env, merchant_addr: &Address, campaign_id: u64, tag_id: u64) {
    merchant_addr.require_auth();

    let key = CampaignKey::Campaign(campaign_id);
    if !env.storage().persistent().has(&key) {
        panic_with_error!(env, CampaignError::CampaignNotFound);
    }
    let mut campaign: Campaign = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignNotFound));
    if campaign.merchant != *merchant_addr {
        panic_with_error!(env, CampaignError::NotCampaignMerchant);
    }

    let tag_key = CampaignKey::CampaignTag(tag_id);
    if !env.storage().persistent().has(&tag_key) {
        panic_with_error!(env, CampaignError::CampaignTagNotFound);
    }

    let attached = push_unique_u64(env, &CampaignKey::CampaignTagList(campaign_id), tag_id);
    // Only update the inverse index when this tag was newly attached, so the
    // two indices stay in lockstep and events fire exactly once per attach.
    if attached {
        push_unique_u64(env, &CampaignKey::TagCampaigns(tag_id), campaign_id);

        // Refresh the campaign.tags snapshot from authoritative storage so
        // reads see the updated tag list without an extra fetch.
        let current_tags: Vec<u64> = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&CampaignKey::CampaignTagList(campaign_id))
            .unwrap_or_else(|| Vec::new(env));
        campaign.tags = current_tags;
        env.storage().persistent().set(&key, &campaign);

        events::publish_campaign_tag_added_event(
            env,
            campaign_id,
            merchant_addr.clone(),
            tag_id,
            env.ledger().timestamp(),
        );
    }
}

/// Detach a tag from a campaign. Reverse index updated to remove the
/// campaign_id from the tag's index list.
pub fn remove_campaign_tag(env: &Env, merchant_addr: &Address, campaign_id: u64, tag_id: u64) {
    merchant_addr.require_auth();

    let key = CampaignKey::Campaign(campaign_id);
    if !env.storage().persistent().has(&key) {
        panic_with_error!(env, CampaignError::CampaignNotFound);
    }
    let mut campaign: Campaign = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignNotFound));
    if campaign.merchant != *merchant_addr {
        panic_with_error!(env, CampaignError::NotCampaignMerchant);
    }

    let current: Vec<u64> = env
        .storage()
        .persistent()
        .get::<_, Vec<u64>>(&CampaignKey::CampaignTagList(campaign_id))
        .unwrap_or_else(|| Vec::new(env));
    let mut remaining: Vec<u64> = Vec::new(env);
    let mut was_present = false;
    for t in current.iter() {
        if t == tag_id {
            was_present = true;
        } else {
            remaining.push_back(t);
        }
    }
    if !was_present {
        return;
    }
    env.storage()
        .persistent()
        .set(&CampaignKey::CampaignTagList(campaign_id), &remaining);
    campaign.tags = remaining.clone();
    env.storage().persistent().set(&key, &campaign);

    let tag_list: Vec<u64> = env
        .storage()
        .persistent()
        .get::<_, Vec<u64>>(&CampaignKey::TagCampaigns(tag_id))
        .unwrap_or_else(|| Vec::new(env));
    let mut new_tag_list: Vec<u64> = Vec::new(env);
    for tid in tag_list.iter() {
        if tid != campaign_id {
            new_tag_list.push_back(tid);
        }
    }
    env.storage()
        .persistent()
        .set(&CampaignKey::TagCampaigns(tag_id), &new_tag_list);

    events::publish_campaign_tag_removed_event(
        env,
        campaign_id,
        merchant_addr.clone(),
        tag_id,
        env.ledger().timestamp(),
    );
}

/// Records a contribution amount against `campaign_id`. This is an accounting
/// helper - it does not move tokens. Campaigns may use any off-chain payment
/// rail while benefiting from the on-chain metadata + indexed totals.
pub fn record_contribution(env: &Env, campaign_id: u64, contributor: &Address, amount: i128) {
    if amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    let key = CampaignKey::Campaign(campaign_id);
    if !env.storage().persistent().has(&key) {
        panic_with_error!(env, CampaignError::CampaignNotFound);
    }
    let mut campaign: Campaign = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignNotFound));
    if !campaign.active {
        panic_with_error!(env, CampaignError::CampaignInactive);
    }
    if env.ledger().timestamp() > campaign.deadline {
        panic_with_error!(env, CampaignError::CampaignExpired);
    }

    campaign.raised_amount = campaign.raised_amount.saturating_add(amount);
    env.storage().persistent().set(&key, &campaign);

    events::publish_campaign_contribution_event(
        env,
        campaign_id,
        contributor.clone(),
        amount,
        campaign.raised_amount,
        campaign.goal_amount,
        env.ledger().timestamp(),
    );
}

// ── Read accessors ────────────────────────────────────────────────────────────

pub fn get_campaign(env: &Env, campaign_id: u64) -> Campaign {
    let key = CampaignKey::Campaign(campaign_id);
    if !env.storage().persistent().has(&key) {
        panic_with_error!(env, CampaignError::CampaignNotFound);
    }
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignNotFound))
}

pub fn get_campaigns_by_category(env: &Env, category_id: u64) -> Vec<Campaign> {
    collect_campaigns(env, &CampaignKey::CategoryCampaigns(category_id))
}

pub fn get_campaigns_by_tag(env: &Env, tag_id: u64) -> Vec<Campaign> {
    collect_campaigns(env, &CampaignKey::TagCampaigns(tag_id))
}

pub fn get_merchant_campaigns(env: &Env, merchant_id: u64) -> Vec<Campaign> {
    collect_campaigns(env, &CampaignKey::MerchantCampaigns(merchant_id))
}

fn collect_campaigns(env: &Env, index_key: &CampaignKey) -> Vec<Campaign> {
    let ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(index_key)
        .unwrap_or_else(|| Vec::new(env));
    let mut out: Vec<Campaign> = Vec::new(env);
    for id in ids.iter() {
        if let Some(c) = env
            .storage()
            .persistent()
            .get::<_, Campaign>(&CampaignKey::Campaign(id))
        {
            out.push_back(c);
        }
    }
    out
}

pub fn get_campaigns(env: &Env, filter: CampaignFilter) -> Vec<Campaign> {
    let count = get_campaign_count(env);

    let seeded_ids: Vec<u64> = match (filter.category_id, filter.tag_id) {
        (Some(cat), Some(tag)) => {
            let cat_ids = env
                .storage()
                .persistent()
                .get::<_, Vec<u64>>(&CampaignKey::CategoryCampaigns(cat))
                .unwrap_or_else(|| Vec::new(env));
            let tag_ids = env
                .storage()
                .persistent()
                .get::<_, Vec<u64>>(&CampaignKey::TagCampaigns(tag))
                .unwrap_or_else(|| Vec::new(env));
            let mut intersection: Vec<u64> = Vec::new(env);
            for c_id in cat_ids.iter() {
                for t_id in tag_ids.iter() {
                    if c_id == t_id {
                        intersection.push_back(c_id);
                        break;
                    }
                }
            }
            intersection
        }
        (Some(cat), None) => env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&CampaignKey::CategoryCampaigns(cat))
            .unwrap_or_else(|| Vec::new(env)),
        (None, Some(tag)) => env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&CampaignKey::TagCampaigns(tag))
            .unwrap_or_else(|| Vec::new(env)),
        (None, None) => {
            let mut all: Vec<u64> = Vec::new(env);
            for i in 1..=count {
                all.push_back(i);
            }
            all
        }
    };

    let mut out: Vec<Campaign> = Vec::new(env);
    for id in seeded_ids.iter() {
        if let Some(c) = env
            .storage()
            .persistent()
            .get::<_, Campaign>(&CampaignKey::Campaign(id))
        {
            if let Some(active) = filter.is_active {
                if c.active != active {
                    continue;
                }
            }
            if let Some(mid) = filter.merchant_id {
                if c.merchant_id != mid {
                    continue;
                }
            }
            out.push_back(c);
        }
    }
    out
}
