use soroban_sdk::{contractevent, Address, BytesN, Env, String, Vec};
use super::types::*;
use shade::types::*;

#[contractevent]
pub struct EventCreatedEvent {
    pub event_id: u64,
    pub merchant: Address,
    pub merchant_id: u64,
    pub name: String,
    pub ticket_price: i128,
    pub token: Address,
    pub capacity: u32,
    pub event_date: u64,
    pub royalty_bps: u32,
    pub timestamp: u64,
}

#[allow(clippy::too_many_arguments)]
pub fn publish_event_created_event(
    env: &Env,
    event_id: u64,
    merchant: Address,
    merchant_id: u64,
    name: String,
    ticket_price: i128,
    token: Address,
    capacity: u32,
    event_date: u64,
    royalty_bps: u32,
    timestamp: u64,
) {
    EventCreatedEvent {
        event_id,
        merchant,
        merchant_id,
        name,
        ticket_price,
        token,
        capacity,
        event_date,
        royalty_bps,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct TicketPurchasedEvent {
    pub ticket_id: u64,
    pub event_id: u64,
    pub merchant_id: u64,
    pub buyer: Address,
    pub amount: i128,
    pub fee: i128,
    pub merchant_amount: i128,
    pub token: Address,
    pub timestamp: u64,
}

#[allow(clippy::too_many_arguments)]
pub fn publish_ticket_purchased_event(
    env: &Env,
    ticket_id: u64,
    event_id: u64,
    merchant_id: u64,
    buyer: Address,
    amount: i128,
    fee: i128,
    merchant_amount: i128,
    token: Address,
    timestamp: u64,
) {
    TicketPurchasedEvent {
        ticket_id,
        event_id,
        merchant_id,
        buyer,
        amount,
        fee,
        merchant_amount,
        token,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct TicketResoldEvent {
    pub ticket_id: u64,
    pub event_id: u64,
    pub merchant_id: u64,
    pub seller: Address,
    pub buyer: Address,
    pub resale_price: i128,
    pub royalty: i128,
    pub seller_proceeds: i128,
    pub token: Address,
    pub timestamp: u64,
}

#[allow(clippy::too_many_arguments)]
pub fn publish_ticket_resold_event(
    env: &Env,
    ticket_id: u64,
    event_id: u64,
    merchant_id: u64,
    seller: Address,
    buyer: Address,
    resale_price: i128,
    royalty: i128,
    seller_proceeds: i128,
    token: Address,
    timestamp: u64,
) {
    TicketResoldEvent {
        ticket_id,
        event_id,
        merchant_id,
        seller,
        buyer,
        resale_price,
        royalty,
        seller_proceeds,
        token,
        timestamp,
    }
    .publish(env);
}

/// Emitted when an event (ticketing) query is executed.
#[contractevent]
pub struct EventSearchExecutedEvent {
    pub caller: Address,
    pub result_count: u32,
    pub timestamp: u64,
}

pub fn publish_event_search_executed_event(
    env: &Env,
    caller: Address,
    result_count: u32,
    timestamp: u64,
) {
    EventSearchExecutedEvent {
        caller,
        result_count,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct TicketListedEvent {
    pub ticket_id: u64,
    pub seller: Address,
    pub price: i128,
    pub timestamp: u64,
}

pub fn publish_ticket_listed_event(
    env: &Env,
    ticket_id: u64,
    seller: Address,
    price: i128,
    timestamp: u64,
) {
    TicketListedEvent {
        ticket_id,
        seller,
        price,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct TicketListingCancelledEvent {
    pub ticket_id: u64,
    pub seller: Address,
    pub timestamp: u64,
}

pub fn publish_ticket_listing_cancelled_event(
    env: &Env,
    ticket_id: u64,
    seller: Address,
    timestamp: u64,
) {
    TicketListingCancelledEvent {
        ticket_id,
        seller,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct TicketListingSoldEvent {
    pub ticket_id: u64,
    pub seller: Address,
    pub buyer: Address,
    pub resale_price: i128,
    pub royalty: i128,
    pub timestamp: u64,
}

pub fn publish_ticket_listing_sold_event(
    env: &Env,
    ticket_id: u64,
    seller: Address,
    buyer: Address,
    resale_price: i128,
    royalty: i128,
    timestamp: u64,
) {
    TicketListingSoldEvent {
        ticket_id,
        seller,
        buyer,
        resale_price,
        royalty,
        timestamp,
    }
    .publish(env);
}
