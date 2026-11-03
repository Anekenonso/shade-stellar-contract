//! Code moved out of the Shade hub, wired to the `shade` crate. Build with `--features from-shade`.

pub mod errors;
pub mod events;
pub mod types;
pub mod analytics;
pub mod backer_rewards;
pub mod campaign;
pub mod campaign_royalties;
pub mod campaigns;
pub mod comments;
pub mod cross_chain_pledge;
pub mod entrypoints;
pub mod fiat_goals;
pub mod kyc;
pub mod leaderboard;
pub mod nft;
pub mod pledge;
pub mod stretch_goals;
pub mod vesting;
pub mod voting;

#[cfg(test)]
mod tests;
