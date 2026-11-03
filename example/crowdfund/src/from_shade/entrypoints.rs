//! The entry points these modules exposed on the Shade hub, kept for reference.

use super::types::*;
use shade::types::*;
use soroban_sdk::{Address, BytesN, Env, String, Vec};

pub struct HubEntrypoints;

impl HubEntrypoints {
    pub fn create_nft_collection(
        env: Env,
        merchant: Address,
        name: String,
        base_uri: String,
        max_supply: u64,
        royalty_bps: u32,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::nft::create_nft_collection(
            &env,
            &merchant,
            &name,
            &base_uri,
            max_supply,
            royalty_bps,
        )
    }

    pub fn mint_nft(
        env: Env,
        merchant: Address,
        collection_id: u64,
        recipient: Address,
        token_uri: String,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::nft::mint_nft(&env, &merchant, collection_id, &recipient, &token_uri)
    }

    pub fn batch_mint_nfts(
        env: Env,
        merchant: Address,
        collection_id: u64,
        recipients: Vec<Address>,
        token_uris: Vec<String>,
    ) -> Vec<u64> {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::nft::batch_mint_nfts(&env, &merchant, collection_id, &recipients, &token_uris)
    }

    pub fn transfer_nft(env: Env, from: Address, to: Address, nft_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::nft::transfer_nft(&env, &from, &to, nft_id)
    }

    pub fn burn_nft(env: Env, owner: Address, nft_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::nft::burn_nft(&env, &owner, nft_id)
    }

    pub fn claim_nft_reward(env: Env, claimer: Address, nft_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::nft::claim_nft_reward(&env, &claimer, nft_id)
    }

    pub fn deactivate_nft_collection(env: Env, merchant: Address, collection_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::nft::deactivate_nft_collection(&env, &merchant, collection_id)
    }

    pub fn get_nft_collection(env: Env, collection_id: u64) -> NftCollection {
        crate::from_shade::nft::get_nft_collection(&env, collection_id)
    }

    pub fn get_nft(env: Env, nft_id: u64) -> Nft {
        crate::from_shade::nft::get_nft(&env, nft_id)
    }

    pub fn get_collection_nfts(env: Env, collection_id: u64) -> Vec<u64> {
        crate::from_shade::nft::get_collection_nfts(&env, collection_id)
    }

    pub fn get_user_nfts(env: Env, user: Address) -> Vec<u64> {
        crate::from_shade::nft::get_user_nfts(&env, &user)
    }

    pub fn create_backer_campaign(
        env: Env,
        merchant: Address,
        name: String,
        token: Address,
        deadline: u64,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::backer_rewards::create_backer_campaign(
            &env, merchant, name, token, deadline,
        )
    }

    pub fn get_backer_campaign(env: Env, campaign_id: u64) -> BackerCampaign {
        crate::from_shade::backer_rewards::get_backer_campaign(&env, campaign_id)
    }

    pub fn set_backer_reward_tiers(
        env: Env,
        merchant: Address,
        campaign_id: u64,
        tiers: Vec<BackerRewardTier>,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::backer_rewards::set_backer_reward_tiers(
            &env,
            merchant,
            campaign_id,
            tiers,
        );
    }

    pub fn get_backer_reward_tiers(env: Env, campaign_id: u64) -> Vec<BackerRewardTier> {
        crate::from_shade::backer_rewards::get_backer_reward_tiers(&env, campaign_id)
    }

    pub fn pledge_to_campaign(env: Env, backer: Address, campaign_id: u64, amount: i128) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::backer_rewards::pledge_to_campaign(&env, backer, campaign_id, amount);
    }

    pub fn get_backer_pledge(env: Env, campaign_id: u64, backer: Address) -> i128 {
        crate::from_shade::backer_rewards::get_backer_pledge(&env, campaign_id, backer)
    }

    pub fn select_backer_reward_tier(env: Env, backer: Address, campaign_id: u64, tier_index: u32) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::backer_rewards::select_backer_reward_tier(
            &env,
            backer,
            campaign_id,
            tier_index,
        );
    }

    pub fn get_backer_selected_tier(env: Env, campaign_id: u64, backer: Address) -> Option<u32> {
        crate::from_shade::backer_rewards::get_backer_selected_tier(&env, campaign_id, backer)
    }

    pub fn fulfill_backer_reward(env: Env, merchant: Address, campaign_id: u64, backer: Address) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::backer_rewards::fulfill_backer_reward(
            &env,
            merchant,
            campaign_id,
            backer,
        );
    }

    pub fn is_backer_reward_fulfilled(env: Env, campaign_id: u64, backer: Address) -> bool {
        crate::from_shade::backer_rewards::is_backer_reward_fulfilled(&env, campaign_id, backer)
    }

    pub fn claim_backer_perk(env: Env, backer: Address, campaign_id: u64, perk_index: u32) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::backer_rewards::claim_backer_perk(&env, backer, campaign_id, perk_index);
    }

    pub fn is_backer_perk_claimed(
        env: Env,
        campaign_id: u64,
        backer: Address,
        perk_index: u32,
    ) -> bool {
        crate::from_shade::backer_rewards::is_backer_perk_claimed(
            &env,
            campaign_id,
            backer,
            perk_index,
        )
    }

    pub fn create_pledge_campaign(
        env: Env,
        merchant: Address,
        title: String,
        goal: i128,
        token: Address,
        deadline: u64,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::pledge::create_campaign(&env, &merchant, &title, goal, &token, deadline)
    }

    pub fn get_pledge_campaign(env: Env, campaign_id: u64) -> PledgeCampaign {
        crate::from_shade::pledge::get_campaign(&env, campaign_id)
    }

    pub fn pledge(
        env: Env,
        contributor: Address,
        campaign_id: u64,
        amount: i128,
        token: Address,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::pledge::pledge(&env, &contributor, campaign_id, amount, &token)
    }

    pub fn execute_campaign(env: Env, merchant: Address, campaign_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::pledge::execute_campaign(&env, &merchant, campaign_id);
    }

    pub fn cancel_pledge_campaign(env: Env, merchant: Address, campaign_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::pledge::cancel_campaign(&env, &merchant, campaign_id);
    }

    pub fn claim_pledge_refund(env: Env, contributor: Address, campaign_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::pledge::claim_refund(&env, &contributor, campaign_id);
    }

    pub fn batch_refund(env: Env, campaign_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::pledge::batch_refund(&env, campaign_id);
    }

    pub fn get_pledge(env: Env, pledge_id: u64) -> Pledge {
        crate::from_shade::pledge::get_pledge(&env, pledge_id)
    }

    pub fn get_campaign_pledges(env: Env, campaign_id: u64) -> Vec<Pledge> {
        crate::from_shade::pledge::get_campaign_pledges(&env, campaign_id)
    }

    pub fn get_contributor_pledges(env: Env, contributor: Address) -> Vec<Pledge> {
        crate::from_shade::pledge::get_contributor_pledges(&env, &contributor)
    }

    pub fn get_merchant_campaigns(env: Env, merchant: Address) -> Vec<Campaign> {
        crate::from_shade::campaigns::get_merchant_campaigns(
            &env,
            shade::components::merchant::get_merchant_id(&env, &merchant),
        )
    }

    pub fn create_campaign_category(
        env: Env,
        admin: Address,
        name: String,
        description: String,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaigns::create_category(&env, &admin, &name, &description)
    }

    pub fn update_campaign_category(
        env: Env,
        admin: Address,
        category_id: u64,
        name: Option<String>,
        description: Option<String>,
        active: Option<bool>,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaigns::update_category(&env, &admin, category_id, name, description, active);
    }

    pub fn get_campaign_category(env: Env, category_id: u64) -> CampaignCategory {
        crate::from_shade::campaigns::get_category(&env, category_id)
    }

    pub fn get_campaign_categories(env: Env) -> Vec<CampaignCategory> {
        crate::from_shade::campaigns::get_categories(&env)
    }

    pub fn create_campaign_tag(env: Env, creator: Address, name: String) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaigns::create_tag(&env, &creator, &name)
    }

    pub fn get_campaign_tag(env: Env, tag_id: u64) -> CampaignTag {
        crate::from_shade::campaigns::get_tag(&env, tag_id)
    }

    pub fn get_campaign_tags(env: Env) -> Vec<CampaignTag> {
        crate::from_shade::campaigns::get_tags(&env)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_campaign(
        env: Env,
        merchant: Address,
        title: String,
        description: String,
        category_id: u64,
        tags: Vec<u64>,
        goal_amount: i128,
        token: Address,
        deadline: u64,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaigns::create_campaign(
            &env,
            &merchant,
            &title,
            &description,
            category_id,
            &tags,
            goal_amount,
            &token,
            deadline,
        )
    }

    pub fn update_campaign(
        env: Env,
        merchant: Address,
        campaign_id: u64,
        title: Option<String>,
        description: Option<String>,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaigns::update_campaign(&env, &merchant, campaign_id, title, description);
    }

    pub fn set_campaign_active(env: Env, merchant: Address, campaign_id: u64, active: bool) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaigns::set_campaign_active(&env, &merchant, campaign_id, active);
    }

    pub fn add_campaign_tag(env: Env, merchant: Address, campaign_id: u64, tag_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaigns::add_campaign_tag(&env, &merchant, campaign_id, tag_id);
    }

    pub fn remove_campaign_tag(env: Env, merchant: Address, campaign_id: u64, tag_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaigns::remove_campaign_tag(&env, &merchant, campaign_id, tag_id);
    }

    pub fn record_campaign_contribution(
        env: Env,
        campaign_id: u64,
        contributor: Address,
        amount: i128,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        contributor.require_auth();
        crate::from_shade::campaigns::record_contribution(&env, campaign_id, &contributor, amount);
    }

    pub fn get_campaign(env: Env, campaign_id: u64) -> Campaign {
        crate::from_shade::campaigns::get_campaign(&env, campaign_id)
    }

    pub fn get_campaigns(env: Env, filter: CampaignFilter) -> Vec<Campaign> {
        crate::from_shade::campaigns::get_campaigns(&env, filter)
    }

    pub fn init_campaign(env: Env, merchant: Address, campaign_id: u64) {
        crate::from_shade::leaderboard::init_campaign(&env, merchant, campaign_id);
    }

    pub fn track_donation(env: Env, merchant: Address, campaign_id: u64, donor: Address, amount: i128) {
        crate::from_shade::leaderboard::track_donation(&env, merchant, campaign_id, donor, amount);
    }

    pub fn get_top_donors(env: Env, campaign_id: u64) -> Vec<DonorInfo> {
        crate::from_shade::leaderboard::get_top_donors(&env, campaign_id)
    }

    /// Defines a funding milestone beyond a campaign's base goal.
    /// Only the campaign's owning merchant may call this.
    pub fn create_stretch_goal(
        env: Env,
        merchant: Address,
        campaign_id: u64,
        target_amount: i128,
        description: String,
        reward_description: String,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::stretch_goals::create_stretch_goal(
            &env,
            merchant,
            campaign_id,
            target_amount,
            description,
            reward_description,
        )
    }

    /// Unlocks a goal whose campaign has reached its target. The raised amount
    /// is read from the campaign, not supplied by the caller.
    pub fn unlock_stretch_goal(env: Env, merchant: Address, goal_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::stretch_goals::unlock_stretch_goal(&env, merchant, goal_id);
    }

    /// Retires a goal that has not been unlocked yet.
    pub fn cancel_stretch_goal(env: Env, merchant: Address, goal_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::stretch_goals::cancel_stretch_goal(&env, merchant, goal_id);
    }

    /// Grants a reward to one backer for an unlocked goal.
    pub fn grant_stretch_goal_reward(
        env: Env,
        merchant: Address,
        goal_id: u64,
        backer: Address,
        reward_amount: i128,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::stretch_goals::grant_stretch_goal_reward(
            &env,
            merchant,
            goal_id,
            backer,
            reward_amount,
        );
    }

    /// Marks the caller's reward for a goal as claimed.
    pub fn claim_stretch_goal_reward(env: Env, backer: Address, goal_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::stretch_goals::claim_stretch_goal_reward(&env, backer, goal_id);
    }

    pub fn get_stretch_goal(env: Env, goal_id: u64) -> StretchGoal {
        crate::from_shade::stretch_goals::get_stretch_goal(&env, goal_id)
    }

    pub fn get_campaign_stretch_goals(env: Env, campaign_id: u64) -> Vec<u64> {
        crate::from_shade::stretch_goals::get_campaign_stretch_goals(&env, campaign_id)
    }

    pub fn get_campaign_stretch_goal_data(env: Env, campaign_id: u64) -> Vec<StretchGoal> {
        crate::from_shade::stretch_goals::get_campaign_stretch_goal_data(&env, campaign_id)
    }

    /// The campaign's next un-unlocked milestone, if any.
    pub fn get_next_stretch_goal(env: Env, campaign_id: u64) -> Option<StretchGoal> {
        crate::from_shade::stretch_goals::get_next_stretch_goal(&env, campaign_id)
    }

    pub fn get_stretch_goal_reward(
        env: Env,
        goal_id: u64,
        backer: Address,
    ) -> Option<StretchGoalReward> {
        crate::from_shade::stretch_goals::get_stretch_goal_reward(&env, goal_id, backer)
    }

    pub fn create_fee_campaign(
        env: Env,
        caller: Address,
        name: String,
        charity: bool,
        fee_waiver_bps: u32,
        discount_bps: u32,
        stake_required: i128,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaign::create_campaign(
            &env,
            &caller,
            &name,
            charity,
            fee_waiver_bps,
            discount_bps,
            stake_required,
        )
    }

    pub fn get_fee_campaign(env: Env, campaign_id: u64) -> FeeCampaign {
        crate::from_shade::campaign::get_campaign(&env, campaign_id)
    }

    pub fn configure_campaign_fee_policy(
        env: Env,
        caller: Address,
        campaign_id: u64,
        fee_waiver_bps: u32,
        discount_bps: u32,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaign::configure_campaign_fee_policy(
            &env,
            &caller,
            campaign_id,
            fee_waiver_bps,
            discount_bps,
        );
    }

    pub fn calculate_campaign_discount(env: Env, campaign_id: u64, amount: i128) -> i128 {
        crate::from_shade::campaign::calculate_campaign_discount(&env, campaign_id, amount)
    }

    pub fn record_fee_campaign_contribution(env: Env, caller: Address, campaign_id: u64, amount: i128) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaign::record_campaign_contribution(&env, &caller, campaign_id, amount);
    }

    pub fn stake_campaign(env: Env, caller: Address, campaign_id: u64, amount: i128) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaign::stake_campaign(&env, &caller, campaign_id, amount);
    }

    pub fn slash_campaign_stake(
        env: Env,
        caller: Address,
        campaign_id: u64,
        participant: Address,
        amount: i128,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaign::slash_campaign_stake(&env, &caller, campaign_id, &participant, amount);
    }

    pub fn register_affiliate(
        env: Env,
        caller: Address,
        campaign_id: u64,
        affiliate: Address,
        commission_bps: u32,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaign::register_affiliate(
            &env,
            &caller,
            campaign_id,
            &affiliate,
            commission_bps,
        );
    }

    pub fn pay_affiliate_commission(
        env: Env,
        caller: Address,
        campaign_id: u64,
        affiliate: Address,
        amount: i128,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::campaign::pay_affiliate_commission(
            &env,
            &caller,
            campaign_id,
            &affiliate,
            amount,
        );
    }

    pub fn get_campaign_participant(
        env: Env,
        campaign_id: u64,
        participant: Address,
    ) -> CampaignParticipant {
        crate::from_shade::campaign::get_campaign_participant(&env, campaign_id, &participant)
    }

    pub fn get_campaign_affiliate(env: Env, campaign_id: u64, affiliate: Address) -> CampaignAffiliate {
        crate::from_shade::campaign::get_campaign_affiliate(&env, campaign_id, &affiliate)
    }

    pub fn get_campaign_leaderboard(env: Env, campaign_id: u64, limit: u32) -> Vec<(Address, i128)> {
        crate::from_shade::campaign::get_campaign_leaderboard(&env, campaign_id, limit)
    }

    /// Commits a backer campaign's raised funds to a cliff-plus-linear vesting
    /// schedule paying out to its creator. Owning merchant only; the terms are
    /// fixed once published.
    pub fn create_creator_vesting(
        env: Env,
        creator: Address,
        campaign_id: u64,
        total_amount: i128,
        start_time: u64,
        cliff_duration: u64,
        vesting_duration: u64,
        initial_unlock_bps: u32,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::vesting::create_creator_vesting(
            &env,
            creator,
            campaign_id,
            total_amount,
            start_time,
            cliff_duration,
            vesting_duration,
            initial_unlock_bps,
        );
    }

    /// Pays the creator everything vested since their last release, returning
    /// the amount transferred.
    pub fn release_creator_vesting(env: Env, creator: Address, campaign_id: u64) -> i128 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::vesting::release_creator_vesting(&env, creator, campaign_id)
    }

    /// Freezes a schedule so nothing further vests. Admin only; the
    /// already-vested balance stays claimable by the creator.
    pub fn revoke_creator_vesting(env: Env, admin: Address, campaign_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::vesting::revoke_creator_vesting(&env, admin, campaign_id);
    }

    pub fn get_creator_vesting(env: Env, campaign_id: u64) -> CreatorVesting {
        crate::from_shade::vesting::get_creator_vesting(&env, campaign_id)
    }

    /// Amount vested as of now, released or not.
    pub fn get_vested_amount(env: Env, campaign_id: u64) -> i128 {
        crate::from_shade::vesting::get_vested_amount(&env, campaign_id)
    }

    /// Amount the creator could release right now.
    pub fn get_releasable_amount(env: Env, campaign_id: u64) -> i128 {
        crate::from_shade::vesting::get_releasable_amount(&env, campaign_id)
    }

    /// Campaign IDs this creator vests funds from, in creation order.
    pub fn get_creator_vesting_campaigns(env: Env, creator: Address) -> Vec<u64> {
        crate::from_shade::vesting::get_creator_vesting_campaigns(&env, creator)
    }

    /// Pegs a campaign's funding target to a fiat currency, valued through the
    /// campaign token's price oracle. Owning merchant only, once per campaign;
    /// the published target is immutable.
    pub fn set_campaign_fiat_goal(
        env: Env,
        merchant: Address,
        campaign_id: u64,
        currency: String,
        goal_amount: i128,
        decimals: u32,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::fiat_goals::set_campaign_fiat_goal(
            &env,
            &merchant,
            campaign_id,
            &currency,
            goal_amount,
            decimals,
        );
    }

    /// Values a token contribution against a campaign's fiat peg and credits
    /// the fiat figure to the goal, returning it.
    pub fn record_fiat_contribution(
        env: Env,
        contributor: Address,
        campaign_id: u64,
        token_amount: i128,
    ) -> i128 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::fiat_goals::record_fiat_contribution(
            &env,
            &contributor,
            campaign_id,
            token_amount,
        )
    }

    /// Re-reads the oracle and publishes a fresh on-ledger valuation. Owning
    /// merchant only; use `get_campaign_fiat_goal_quote` for a read-only view.
    pub fn refresh_campaign_fiat_quote(env: Env, merchant: Address, campaign_id: u64) -> FiatGoalQuote {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::fiat_goals::refresh_campaign_fiat_quote(&env, &merchant, campaign_id)
    }

    /// Stops further contributions being valued against a peg. Owning merchant
    /// or admin; the raised total is preserved.
    pub fn close_campaign_fiat_goal(env: Env, caller: Address, campaign_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::fiat_goals::close_campaign_fiat_goal(&env, &caller, campaign_id);
    }

    pub fn get_campaign_fiat_goal(env: Env, campaign_id: u64) -> CampaignFiatGoal {
        crate::from_shade::fiat_goals::get_campaign_fiat_goal(&env, campaign_id)
    }

    /// Whether this campaign's goal is fiat-pegged.
    pub fn has_campaign_fiat_goal(env: Env, campaign_id: u64) -> bool {
        crate::from_shade::fiat_goals::has_campaign_fiat_goal(&env, campaign_id)
    }

    /// Live valuation of a peg at the current oracle price.
    pub fn get_campaign_fiat_goal_quote(env: Env, campaign_id: u64) -> FiatGoalQuote {
        crate::from_shade::fiat_goals::get_campaign_fiat_goal_quote(&env, campaign_id)
    }

    /// Fiat a token contribution would be credited at right now.
    pub fn quote_fiat_contribution(env: Env, campaign_id: u64, token_amount: i128) -> i128 {
        crate::from_shade::fiat_goals::quote_fiat_contribution(&env, campaign_id, token_amount)
    }

    /// Cumulative fiat one backer has contributed to a campaign.
    pub fn get_backer_fiat_contribution(env: Env, campaign_id: u64, backer: Address) -> i128 {
        crate::from_shade::fiat_goals::get_backer_fiat_contribution(&env, campaign_id, &backer)
    }

    /// Pegs across a merchant's campaigns, in campaign-creation order.
    pub fn get_merchant_fiat_goals(env: Env, merchant_id: u64) -> Vec<CampaignFiatGoal> {
        crate::from_shade::fiat_goals::get_merchant_fiat_goals(&env, merchant_id)
    }

    /// Snapshots a campaign's analytics into an immutable export record and
    /// emits it for off-chain rendering, returning the export's ID. Owning
    /// merchant only; each export reports the delta since the campaign's
    /// previous one alongside the cumulative figures.
    pub fn export_campaign_analytics(
        env: Env,
        creator: Address,
        campaign_id: u64,
        format: ExportFormat,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::analytics::export_campaign_analytics(&env, &creator, campaign_id, format)
    }

    /// A campaign's running contribution aggregate, as exports snapshot it.
    pub fn get_campaign_stats(env: Env, campaign_id: u64) -> CampaignStats {
        crate::from_shade::analytics::get_campaign_stats(&env, campaign_id)
    }

    pub fn get_analytics_export(env: Env, export_id: u64) -> AnalyticsExport {
        crate::from_shade::analytics::get_analytics_export(&env, export_id)
    }

    /// Export IDs for a campaign, in the order they were run.
    pub fn get_campaign_exports(env: Env, campaign_id: u64) -> Vec<u64> {
        crate::from_shade::analytics::get_campaign_exports(&env, campaign_id)
    }

    /// The most recent export for a campaign.
    pub fn get_latest_campaign_export(env: Env, campaign_id: u64) -> AnalyticsExport {
        crate::from_shade::analytics::get_latest_campaign_export(&env, campaign_id)
    }
}
