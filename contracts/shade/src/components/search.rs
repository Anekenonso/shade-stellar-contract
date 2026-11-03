//! On-chain search and filtering utilities component.
//!
//! Provides cursor-based pagination and filtering over invoices and merchants.
//!
//! All read functions are side-effect-free except for emitting an
//! informational search event so off-chain indexers can track query patterns.

use crate::events;
use crate::types::{
    DataKey, Invoice, InvoiceFilter, InvoicePage, Merchant, MerchantFilter, MerchantPage, PageInfo,
};
use soroban_sdk::{Address, Env, Vec};

// ── Paginated invoice search ──────────────────────────────────────────────────

/// Return a page of invoices matching `filter`, starting after `cursor` (exclusive).
///
/// Pass `cursor = 0` for the first page.  Each page contains at most `page_size`
/// items.  The returned `PageInfo.next_cursor` is the ID of the last item
/// returned; use it as `cursor` in the next call.  A `next_cursor` of `0` in
/// the response means there are no more pages.
pub fn search_invoices_paginated(
    env: &Env,
    caller: &Address,
    filter: InvoiceFilter,
    cursor: u64,
    page_size: u32,
) -> InvoicePage {
    let invoice_count: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::InvoiceCount)
        .unwrap_or(0);

    let start = if cursor == 0 { 1u64 } else { cursor + 1 };
    let mut items: Vec<Invoice> = Vec::new(env);
    let mut last_id: u64 = 0;
    let limit = page_size as usize;

    for i in start..=invoice_count {
        if items.len() as usize >= limit {
            break;
        }
        if let Some(invoice) = env
            .storage()
            .persistent()
            .get::<_, Invoice>(&DataKey::Invoice(i))
        {
            if invoice_matches(&invoice, &filter, env) {
                last_id = invoice.id;
                items.push_back(invoice);
            }
        }
    }

    // Determine whether more pages follow by checking if any matching invoice
    // exists beyond the last returned one.
    let has_next = if last_id == 0 || last_id >= invoice_count {
        false
    } else {
        let mut found_next = false;
        for j in (last_id + 1)..=invoice_count {
            if let Some(inv) = env
                .storage()
                .persistent()
                .get::<_, Invoice>(&DataKey::Invoice(j))
            {
                if invoice_matches(&inv, &filter, env) {
                    found_next = true;
                    break;
                }
            }
        }
        found_next
    };

    let count = items.len();
    let page_info = PageInfo {
        count,
        next_cursor: if has_next { last_id } else { 0 },
        has_next_page: has_next,
    };

    events::publish_invoice_search_executed_event(
        env,
        caller.clone(),
        count,
        has_next,
        env.ledger().timestamp(),
    );

    InvoicePage { items, page_info }
}

/// Internal: test whether an invoice matches all active filter predicates.
fn invoice_matches(invoice: &Invoice, filter: &InvoiceFilter, env: &Env) -> bool {
    if let Some(status) = filter.status {
        if invoice.status as u32 != status {
            return false;
        }
    }
    if let Some(merchant_addr) = &filter.merchant {
        match env
            .storage()
            .persistent()
            .get::<_, u64>(&DataKey::MerchantId(merchant_addr.clone()))
        {
            Some(mid) if mid == invoice.merchant_id => {}
            _ => return false,
        }
    }
    if let Some(min) = filter.min_amount {
        if invoice.amount < min as i128 {
            return false;
        }
    }
    if let Some(max) = filter.max_amount {
        if invoice.amount > max as i128 {
            return false;
        }
    }
    if let Some(start) = filter.start_date {
        if invoice.date_created < start {
            return false;
        }
    }
    if let Some(end) = filter.end_date {
        if invoice.date_created > end {
            return false;
        }
    }
    true
}

// ── Paginated merchant search ─────────────────────────────────────────────────

/// Return a page of merchants matching `filter`, starting after `cursor`.
pub fn search_merchants_paginated(
    env: &Env,
    filter: MerchantFilter,
    cursor: u64,
    page_size: u32,
) -> MerchantPage {
    let merchant_count: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::MerchantCount)
        .unwrap_or(0);

    let start = if cursor == 0 { 1u64 } else { cursor + 1 };
    let mut items: Vec<Merchant> = Vec::new(env);
    let mut last_id: u64 = 0;
    let limit = page_size as usize;

    for i in start..=merchant_count {
        if items.len() as usize >= limit {
            break;
        }
        if let Some(m) = env
            .storage()
            .persistent()
            .get::<_, Merchant>(&DataKey::Merchant(i))
        {
            if merchant_matches(&m, &filter) {
                last_id = m.id;
                items.push_back(m);
            }
        }
    }

    let has_next = if last_id == 0 || last_id >= merchant_count {
        false
    } else {
        let mut found = false;
        for j in (last_id + 1)..=merchant_count {
            if let Some(m) = env
                .storage()
                .persistent()
                .get::<_, Merchant>(&DataKey::Merchant(j))
            {
                if merchant_matches(&m, &filter) {
                    found = true;
                    break;
                }
            }
        }
        found
    };

    let count = items.len();
    MerchantPage {
        items,
        page_info: PageInfo {
            count,
            next_cursor: if has_next { last_id } else { 0 },
            has_next_page: has_next,
        },
    }
}

fn merchant_matches(m: &Merchant, filter: &MerchantFilter) -> bool {
    if let Some(active) = filter.is_active {
        if m.active != active {
            return false;
        }
    }
    if let Some(verified) = filter.is_verified {
        if m.verified != verified {
            return false;
        }
    }
    true
}

// ── Subscription plan search ──────────────────────────────────────────────────

// ── Subscription search ───────────────────────────────────────────────────────

// ── Event (ticketing) search ──────────────────────────────────────────────────

// ── Withdrawal proposal search ────────────────────────────────────────────────

// ── Convenience: merchant ID lookup ──────────────────────────────────────────

/// Look up a merchant ID from an address, returning `None` if not registered.
pub fn find_merchant_id(env: &Env, address: &Address) -> Option<u64> {
    env.storage()
        .persistent()
        .get::<_, u64>(&DataKey::MerchantId(address.clone()))
}
