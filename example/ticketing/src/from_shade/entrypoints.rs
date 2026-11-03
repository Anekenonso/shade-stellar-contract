//! The entry points these modules exposed on the Shade hub, kept for reference.

use super::types::*;
use shade::types::*;
use soroban_sdk::{Address, BytesN, Env, String, Vec};

pub struct HubEntrypoints;

impl HubEntrypoints {
    #[allow(clippy::too_many_arguments)]
    pub fn create_event(
        env: Env,
        merchant: Address,
        name: String,
        ticket_price: i128,
        token: Address,
        capacity: u32,
        event_date: u64,
        royalty_bps: u32,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::event::create_event(
            &env,
            &merchant,
            &name,
            &ticket_price,
            &token,
            &capacity,
            &event_date,
            &royalty_bps,
        )
    }

    pub fn purchase_ticket(env: Env, event_id: u64, buyer: Address) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::event::purchase_ticket(&env, &event_id, &buyer)
    }

    pub fn configure_dynamic_pricing(
        env: Env,
        merchant: Address,
        event_id: u64,
        early_bird_end: u64,
        early_bird_discount_bps: u32,
        late_markup_bps: u32,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::event::configure_dynamic_pricing(
            &env,
            &merchant,
            event_id,
            early_bird_end,
            early_bird_discount_bps,
            late_markup_bps,
        );
    }

    pub fn get_current_ticket_price(env: Env, event_id: u64) -> i128 {
        crate::from_shade::event::get_current_ticket_price(&env, event_id)
    }

    pub fn cancel_event_and_batch_refund(env: Env, merchant: Address, event_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::event::cancel_event_and_batch_refund(&env, &merchant, event_id);
    }

    pub fn resell_ticket(
        env: Env,
        seller: Address,
        buyer: Address,
        ticket_id: u64,
        resale_price: i128,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::event::resell_ticket(&env, &seller, &buyer, ticket_id, resale_price);
    }

    pub fn get_event(env: Env, event_id: u64) -> Event {
        crate::from_shade::event::get_event(&env, &event_id)
    }

    pub fn get_ticket(env: Env, ticket_id: u64) -> Ticket {
        crate::from_shade::event::get_ticket(&env, ticket_id)
    }

    pub fn get_event_tickets(env: Env, event_id: u64) -> Vec<u64> {
        crate::from_shade::event::get_event_tickets(&env, event_id)
    }

    pub fn get_user_tickets(env: Env, user: Address) -> Vec<u64> {
        crate::from_shade::event::get_user_tickets(&env, &user)
    }

    pub fn purchase_tickets_bulk(
        env: Env,
        event_id: u64,
        buyer: Address,
        quantity: u32,
        shade_token: Address,
        merchant_account: Address,
    ) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::event::purchase_tickets_bulk(
            &env,
            &event_id,
            &buyer,
            quantity,
            &shade_token,
            &merchant_account,
        );
    }

    pub fn search_events(env: Env, caller: Address, filter: EventFilter) -> Vec<Event> {
        crate::from_shade::search::search_events(&env, &caller, filter)
    }
}
