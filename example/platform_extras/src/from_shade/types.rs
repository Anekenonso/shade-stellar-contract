use soroban_sdk::{contracttype, Address, BytesN, String, Vec};
use shade::types::*;

/// Ticketing / event storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventKey {
    Event(u64),
    EventCount,
    Ticket(u64),
    TicketCount,
    EventTickets(u64),
    UserTickets(Address),
    /// Active secondary-market listing for a ticket.
    TicketListing(u64),
}

/// DAO governance storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovKey {
    /// Singleton governance config and counters.
    State,
    /// Whether an address is a council member.
    Member(Address),
    Proposal(u64),
    Vote(u64, Address),
}

/// Cross-chain bridge storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BridgeKey {
    BridgeListener(Address),
    BridgeListenerCount,
    BridgeDeposit(u64),
    BridgeDepositCount,
    /// Credited balance from bridged deposits, keyed by `(recipient, token)`.
    BridgeCredit(Address, Address),
    /// Idempotency guard keyed by the origin-chain transaction hash.
    ProcessedBridgeDeposit(BytesN<32>),
    CrossChainPledge(u64),
    CrossChainPledgeCount,
    /// `(source_chain, source_pledge_id)` -> local pledge ID.
    PledgeIdBySourceChain(String, u64),
}

/// Multi-sig massive-withdrawal storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MultiSigKey {
    /// Threshold (in token base units) above which a withdrawal needs multi-sig.
    MultiSigThreshold(Address),
    /// Ordered list of addresses registered as multi-sig signers.
    MultiSigSigners,
    /// Required number of approvals before a pending withdrawal can execute.
    MultiSigQuorum,
    /// A specific pending withdrawal proposal, keyed by proposal ID.
    WithdrawalProposal(u64),
    /// Running counter for withdrawal proposal IDs.
    WithdrawalProposalCount,
    /// Whether a particular signer has approved a particular proposal.
    WithdrawalApproval(u64, Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionPlan {
    pub id: u64,
    /// Numeric merchant ID - used to look up the merchant's account contract.
    pub merchant_id: u64,
    /// The merchant's wallet address - needed for event emission and auth checks.
    pub merchant: Address,
    /// Human-readable description of the plan.
    pub description: String,
    /// Token used for billing.
    pub token: Address,
    /// Amount charged per interval (in token base units).
    pub amount: i128,
    /// Billing interval in seconds (e.g. 2_592_000 = 30 days).
    pub interval: u64,
    /// Whether this plan is accepting new subscribers.
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subscription {
    pub id: u64,
    pub plan_id: u64,
    pub customer: Address,
    /// Copied from the plan for quick access during auth checks.
    pub merchant_id: u64,
    pub status: SubscriptionStatus,
    pub date_created: u64,
    /// Ledger timestamp of the last successful charge.
    /// Starts at 0 so the first charge is available immediately.
    pub last_charged: u64,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SubscriptionStatus {
    Active = 0,
    Cancelled = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub id: u64,
    pub merchant_id: u64,
    pub name: String,
    pub ticket_price: i128,
    pub token: Address,
    pub capacity: u32,
    pub sold: u32,
    pub date: u64,
    /// Scheduled event date (unix seconds). Must be >= ledger timestamp at creation.
    pub event_date: u64,
    /// Royalty paid to the organizer on each resale, in basis points (10_000 = 100%).
    pub royalty_bps: u32,
    /// Early-bird cutoff timestamp. `0` disables early-bird pricing.
    pub early_bird_end: u64,
    /// Discount during early-bird period, in basis points.
    pub early_bird_discount_bps: u32,
    /// Markup applied after early-bird period, in basis points.
    pub late_markup_bps: u32,
    /// True once the event is cancelled.
    pub cancelled: bool,
    /// True once all ticket refunds have been processed.
    pub refunds_processed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ticket {
    pub id: u64,
    pub event_id: u64,
    pub owner: Address,
    pub minted_at: u64,
    /// Amount paid on primary purchase, used for cancellation refunds.
    pub purchase_price: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CrossChainPledgeStatus {
    Pending,
    Completed,
    Failed,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrossChainPledge {
    pub id: u64,
    pub source_chain: String,
    pub source_pledge_id: u64,
    pub destination_chain: String,
    pub merchant: Address,
    pub payer: Address,
    pub token: Address,
    pub amount: i128,
    pub status: CrossChainPledgeStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub memo: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrossChainBridgePayload {
    pub invoice_id: u64,
    pub merchant: Address,
    pub payer: Option<Address>,
    pub source_chain: String,
    pub destination_chain: String,
    pub token: Address,
    pub amount: i128,
    pub destination_recipient: String,
    pub memo: Option<String>,
}

/// A confirmed external-chain deposit recorded by an authorized bridge listener.
///
/// The `source_tx_id` is the 32-byte transaction hash on the origin chain and
/// doubles as the global idempotency key
/// (see [`BridgeKey::ProcessedBridgeDeposit`]).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeDeposit {
    pub id: u64,
    pub source_chain: String,
    pub source_tx_id: BytesN<32>,
    pub listener: Address,
    pub token: Address,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

/// Current lifecycle state of a withdrawal proposal.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum WithdrawalProposalStatus {
    /// Awaiting the required number of signer approvals.
    Pending = 0,
    /// Quorum reached; funds have been transferred.
    Executed = 1,
    /// Cancelled by the proposer or an admin before execution.
    Cancelled = 2,
}

/// A pending or completed massive-withdrawal proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawalProposal {
    pub id: u64,
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub recipient: Address,
    pub approvals: u32,
    pub status: WithdrawalProposalStatus,
    pub created_at: u64,
    /// Ledger timestamp of the last status change.
    pub updated_at: u64,
    pub note: String,
}

/// Singleton governance configuration and counters. `voting_period == 0` is the
/// sentinel for "not yet configured".
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovState {
    pub voting_period: u64,
    pub quorum_bps: u32,
    pub member_count: u32,
    pub proposal_count: u64,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ProposalStatus {
    /// Open for voting.
    Active = 0,
    /// Passed quorum + majority and the upgrade was applied.
    Executed = 1,
    /// Failed quorum or majority after the voting window closed.
    Defeated = 2,
}

/// A council-governed proposal to upgrade the contract's WASM to `wasm_hash`.
/// Voting is one-member-one-vote; `approvals`/`rejections` are head counts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposal {
    pub id: u64,
    pub proposer: Address,
    pub wasm_hash: BytesN<32>,
    pub created_at: u64,
    pub voting_ends_at: u64,
    pub approvals: u32,
    pub rejections: u32,
    pub status: ProposalStatus,
}

/// Filter parameters for querying subscription plans.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionPlanFilter {
    pub merchant: Option<Address>,
    pub active: Option<bool>,
    pub token: Option<Address>,
}

/// Filter parameters for querying individual subscriptions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionFilter {
    pub plan_id: Option<u64>,
    pub customer: Option<Address>,
    /// Restrict to Active (0) or Cancelled (1) subscriptions.
    pub status: Option<u32>,
}

/// Filter parameters for querying on-chain events (ticketing).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventFilter {
    pub merchant: Option<Address>,
    /// When `true` only cancelled events are returned; `false` for active ones.
    pub cancelled: Option<bool>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
    /// Only include events with at least this many remaining seats.
    pub min_available: Option<u32>,
}

/// Filter parameters for querying withdrawal proposals.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawalProposalFilter {
    pub merchant: Option<Address>,
    /// 0=Pending, 1=Executed, 2=Cancelled.
    pub status: Option<u32>,
    pub token: Option<Address>,
    pub created_after: Option<u64>,
}

/// An active secondary-market listing for a ticket.
/// Stored under [`EventKey::TicketListing`], keyed by ticket ID, and removed
/// once the listing is sold or cancelled.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicketListing {
    pub ticket_id: u64,
    pub seller: Address,
    /// Asking price in the event's token base units.
    pub price: i128,
}
