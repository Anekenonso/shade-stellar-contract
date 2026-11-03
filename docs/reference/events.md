# Events reference

Every event emitted anywhere in this workspace — the `shade` hub contract and all eight satellite contracts — with its topic, data payload, emitting function, and the state transition it marks. Built by reading every `#[contractevent]` struct and every `publish_*`/`.publish(env)` call site in the repository, not just `contracts/shade/src/events.rs`.

## How Soroban events work here (read before the tables)

Every event in this workspace uses the SDK's `#[contractevent]` derive macro on a plain struct, then calls `.publish(env)` on an instance of it (usually through a `publish_*` helper function). Two conventions appear:

- **Default (used almost everywhere):** `#[contractevent]` with no arguments. The macro derives the event's topic from the **struct name in snake_case** — e.g. `InvoicePaidEvent` publishes under the topic `invoice_paid_event`. [`contracts/shade/src/events.rs`](../../contracts/shade/src/events.rs)'s own module doc comment states this explicitly and warns that Soroban caps topic Symbols at 32 characters, which is why struct names in that file are kept short.
- **Explicit topic override (used only in `account`):** `#[contractevent(topics = ["account_restricted"])]` on `AccountRestrictedEvent` sets the topic string directly rather than deriving it from the struct name.
- **`data_format` override:** `account`'s `AccountVerifiedEvent` uses `#[contractevent(data_format = "single-value")]`, publishing its one field (`timestamp`) as a bare value instead of the default field-map encoding.

**None of these events declare a multi-field topic tuple.** Every event in this workspace publishes under a single topic (the derived or overridden Symbol above); the event's *data* — every field on the struct — is what carries the rich, multi-field payload, encoded by default as a `Map<Symbol, Val>` keyed by field name (this is what lets tests decode a field by name, as shown in [Decoding an event](#decoding-a-real-event) below). "Filter by topic" in this workspace therefore means "filter by event name / contract address," not "filter by a structured topic tuple" — see [Filter examples](#filter-examples) for what this means in practice.

## Index by domain

- [Core payments (shade)](#core-payments-shade) — admin, tokens, fees, merchants, invoices, payments
- [Access control & pause (shade)](#access-control--pause-shade)
- [`account` contract](#account-contract)

## Core payments (shade)

All defined and emitted from [`contracts/shade/src/events.rs`](../../contracts/shade/src/events.rs) and [`contracts/shade/src/components/`](../../contracts/shade/src/components).

| Event | Data fields | Emitting function | Emission point | State transition |
|---|---|---|---|---|
| `InitalizedEvent` *(sic)* | `admin: Address`, `timestamp: u64` | `core::initialize` | After writing `DataKey::Admin`/`PlatformAccount`/`ContractInfo` | Contract initialized |
| `TokenAddedEvent` | `token: Address`, `timestamp: u64` | `admin::add_accepted_token`/`add_accepted_tokens` | After appending to `DataKey::AcceptedTokens`, inside the reentrancy guard | Token added to the global accepted-token whitelist |
| `TokenRemovedEvent` | `token: Address`, `timestamp: u64` | `admin::remove_accepted_token` | After rewriting `DataKey::AcceptedTokens` without the token | Token removed from the whitelist |
| `MerchantRegisteredEvent` | `merchant: Address`, `merchant_id: u64`, `timestamp: u64` | `merchant::register_merchant` | After writing `Merchant`/`MerchantId`/`MerchantCount` | New merchant registered |
| `MerchantAccountDeployedEvent` | `merchant: Address`, `contract: Address`, `timestamp: u64` | `account_factory::deploy_account` | — | Merchant `account` contract deployed via `account_factory` |
| `MerchantStatusChangedEvent` | `merchant_id: u64`, `active: bool`, `timestamp: u64` | `merchant::set_merchant_status` | After writing the updated `Merchant.active` | Merchant activated/deactivated |
| `InvoiceCreatedEvent` | `invoice_id: u64`, `merchant: Address`, `amount: i128`, `token: Address` | `invoice::create_invoice`, `create_fiat_invoice`, `create_invoice_signed`, `finalize_invoice` | After writing the new/finalized `Invoice` record | Invoice created (or a draft finalized into `Pending`) |
| `InvoiceRefundedEvent` | `invoice_id: u64`, `merchant: Address`, `amount: i128`, `timestamp: u64` | `invoice::refund_invoice`, `refund_invoice_partial` (full-refund branch) | After the refund token transfer and `Invoice.status = Refunded` write | Invoice fully refunded |
| `InvoicePartiallyRefundedEvent` | `invoice_id: u64`, `merchant: Address`, `amount: i128`, `total_amount_refunded: i128`, `timestamp: u64` | `invoice::refund_invoice_partial` (partial branch) | Same point, when `total_refund < invoice.amount` | Invoice partially refunded |
| `MerchantVerifiedEvent` | `merchant_id: u64`, `status: bool`, `timestamp: u64` | `merchant::verify_merchant` | After writing `Merchant.verified` | Merchant verification status changed |
| `MerchantWebhookSetEvent` | `merchant: Address`, `merchant_id: u64`, `webhook: String`, `timestamp: u64` | `merchant::set_merchant_webhook` | After writing `Merchant.webhook` | Merchant webhook URL updated |
| `MerchantKeySetEvent` | `merchant: Address`, `key: BytesN<32>`, `timestamp: u64` | `merchant::set_merchant_key` | After writing `DataKey::MerchantKey` | Merchant's signed-invoice verification key set |
| `RoleGrantedEvent` | `admin: Address`, `user: Address`, `role: Role`, `timestamp: u64` | `access_control::grant_role` | After writing `DataKey::Role` | Role granted to an address |
| `RoleRevokedEvent` | `admin: Address`, `user: Address`, `role: Role`, `timestamp: u64` | `access_control::revoke_role` | After removing `DataKey::Role` | Role revoked |
| `ContractPausedEvent` | `admin: Address`, `timestamp: u64` | `pausable::pause` | After writing `DataKey::Paused = true` | Contract paused |
| `ContractUnpausedEvent` | `admin: Address`, `timestamp: u64` | `pausable::unpause` | After writing `DataKey::Paused = false` | Contract unpaused |
| `FeeProposedEvent` | `admin: Address`, `token: Address`, `fee: i128`, `timestamp: u64` | `admin::propose_fee` | After writing `DataKey::PendingTokenFee` | Time-locked fee change proposed |
| `FeeSetEvent` | `admin: Address`, `token: Address`, `fee: i128`, `timestamp: u64` | `admin::set_fee`, `execute_fee` | After writing `DataKey::TokenFee` | Fee set (directly, or via executing a pending proposal) |
| `PlatformAccountSetEvent` | `admin: Address`, `account: Address`, `timestamp: u64` | `admin::set_platform_account` | After writing `DataKey::PlatformAccount` | Platform fee-recipient account changed |
| `TokenOracleSetEvent` | `admin: Address`, `token: Address`, `oracle: Address`, `timestamp: u64` | `admin::set_token_oracle` | After writing `DataKey::TokenOracle` | Price oracle configured for a token |
| `ContractUpgradedEvent` | `new_wasm_hash: BytesN<32>`, `timestamp: u64` | `upgrade::upgrade` | After `env.deployer().update_current_contract_wasm(...)` | Hub contract's own WASM replaced |
| `AccountRestrictedEvent` | `merchant: Address`, `status: bool`, `caller: Address`, `timestamp: u64` | `merchant::restrict_merchant_account` | After the cross-contract `restrict_account` call on the merchant's `account` contract | Merchant account restricted/unrestricted |
| `InvoicePaidEvent` | `invoice_id: u64`, `merchant_id: u64`, `merchant_account: Address`, `payer: Address`, `amount: i128`, `fee: i128`, `merchant_amount: i128`, `token: Address`, `timestamp: u64` | `invoice::pay_invoice_partial_inner` | After the `Invoice` write (status now `Paid` or `PartiallyPaid`) | A payment (full or partial) landed on an invoice |
| `FiatInvoicePricedEvent` | `invoice_id: u64`, `token: Address`, `resolved_amount: i128`, `timestamp: u64` | `invoice::create_fiat_invoice`, `refresh_fiat_invoice_quote` | Immediately after resolving the invoice's crypto amount from the fiat/oracle price | A fiat-priced invoice's crypto amount was (re-)resolved |
| `PaymentSplitRoutedEvent` | `invoice_id: u64`, `merchant_account: Address`, `platform_account: Address`, `merchant_amount: i128`, `platform_amount: i128`, `token: Address`, `timestamp: u64` | `invoice::pay_invoice_partial_inner` | Right after `InvoicePaidEvent`, same call | Fee/merchant split for an invoice payment was routed |
| `PlatformFeeRoutedEvent` | `route_kind: u32`, `ref_id: u64`, `merchant_id: u64`, `merchant: Address`, `merchant_account: Address`, `platform_account: Address`, `payer: Address`, `gross_amount: i128`, `platform_fee: i128`, `merchant_amount: i128`, `token: Address`, `fee_bps_applied: i128`, `timestamp: u64` | `platform_fee::finalize_route` (called from `route_from_payer`/`route_from_allowance`) | Right after the split token transfers execute, before the caller's own event | Any fee-routed payment (invoice, subscription charge, ticket purchase) settled |
| `MerchantPlatformFeeSetEvent` | `caller: Address`, `merchant_id: u64`, `token: Address`, `fee_bps: i128`, `timestamp: u64` | `platform_fee::set_merchant_platform_fee` | After writing `DataKey::MerchantPlatformFee` | Per-merchant fee override set |
| `PlatformFeeClearedEvent` | `caller: Address`, `merchant_id: u64`, `token: Address`, `timestamp: u64` | `platform_fee::clear_merchant_platform_fee` | After removing `DataKey::MerchantPlatformFee` | Per-merchant fee override cleared |
| `InvoiceCancelledEvent` | `invoice_id: u64`, `merchant: Address`, `timestamp: u64` | `invoice::void_invoice` | After `Invoice.status = Cancelled` | Pending invoice voided |
| `InvoiceAmendedEvent` | `invoice_id: u64`, `merchant: Address`, `old_amount: i128`, `new_amount: i128`, `timestamp: u64` | `invoice::amend_invoice` | After writing the amended `Invoice` | Pending invoice's amount/description edited |
| `NonceInvalidatedEvent` | `merchant: Address`, `nonce: BytesN<32>`, `timestamp: u64` | `signature_util::invalidate_nonce` | After writing `DataKey::UsedNonce` | A signed-invoice nonce was consumed |
| `AdminTransferProposedEvent` | `current_admin: Address`, `proposed_admin: Address`, `timestamp: u64` | `admin::propose_admin_transfer` | After writing `DataKey::PendingAdmin` | Admin handover proposed |
| `AdminTransferAcceptedEvent` | `old_admin: Address`, `new_admin: Address`, `timestamp: u64` | `admin::accept_admin_transfer` | After writing `DataKey::Admin` | Admin handover completed |
| `AccountWasmHashSetEvent` | `admin: Address`, `wasm_hash: BytesN<32>`, `timestamp: u64` | `admin::set_account_wasm_hash` | After writing `DataKey::AccountWasmHash` | WASM hash for future merchant-account deployments updated |
| `MerchantTokensSetEvent` | `merchant: Address`, `tokens: Vec<Address>`, `timestamp: u64` | `merchant::set_merchant_accepted_tokens` | After writing `DataKey::MerchantTokens` | Merchant's own accepted-token list set |
| `MerchantTokenRemovedEvent` | `merchant: Address`, `token: Address`, `timestamp: u64` | `merchant::remove_merchant_accepted_token` | After rewriting `DataKey::MerchantTokens` | One token removed from a merchant's accepted list |
| `AutoWithdrawThresholdEvent` | `merchant_id: u64`, `token: Address`, `threshold: i128` | `auto_withdrawal::set_auto_withdrawal_threshold` | After writing the merchant's threshold list | Auto-withdrawal threshold set |
| `AutoWithdrawRecipientEvent` | `merchant_id: u64`, `recipient: Address` | `auto_withdrawal::set_auto_withdrawal_recipient` | After writing `Merchant.auto_withdrawal_recipient` | Auto-withdrawal recipient set |
| `AutoWithdrawalTriggeredEvent` | `merchant_id: u64`, `token: Address`, `amount: i128`, `recipient: Address` | `auto_withdrawal::trigger_auto_withdrawal` | After the cross-contract `withdraw_to` call on the merchant's `account` contract | Balance crossed the threshold; funds swept |

## Access control & pause (shade)

Covered above under core payments (`RoleGrantedEvent`, `RoleRevokedEvent`, `ContractPausedEvent`, `ContractUnpausedEvent`) — listed there because they live in the same `events.rs` section and share the same emission points.

## Auto-withdrawal (shade)

| Event | Emitted when |
|---|---|
| `AutoWithdrawalTriggeredEvent` | A payment crossed the threshold and the sweep succeeded. |
| `AutoWithdrawalFailedEvent` | A payment crossed the threshold and the sweep failed. The payment still settled. |

## `account` contract

All from [`contracts/account/src/events.rs`](../../contracts/account/src/events.rs).

| Event | Topic | Data fields | Notes |
|---|---|---|---|
| `AccountInitializedEvent` | derived: `account_initialized_event` | `merchant: Address`, `merchant_id: u64`, `timestamp: u64` | Emitted from `initialize` |
| `AccountRestrictedEvent` | **explicit:** `account_restricted` | `status: bool`, `timestamp: u64` | The one event in the workspace with an explicit `topics = [...]` override |
| `AccountVerifiedEvent` | derived: `account_verified_event` | `timestamp: u64` (published via `#[contractevent(data_format = "single-value")]`, so the payload is the bare `u64`, not a field map) | Also aliased as `AccountVerified` for use in tests |
| `TokenAddedEvent` | derived | `token: Address`, `timestamp: u64` | Distinct type from `shade`'s own `TokenAddedEvent` — same name, different contract, different struct |
| `RefundProcessedEvent` | derived | `token: Address`, `amount: i128`, `recipient: Address`, `timestamp: u64` | Emitted by the account contract's `refund` entry point, which `shade`'s `invoice::refund_invoice`/`claim_refund` call into via `MerchantAccountRefundClient` |
| `WithdrawalToEvent` | derived | `token: Address`, `merchant: Address`, `recipient: Address`, `amount: i128`, `timestamp: u64` | Emitted by `withdraw_to`, which `shade`'s auto-withdrawal and multi-sig components call into |
| `PendingWithdrawalCreatedEvent` | derived | `id: u64`, `token: Address`, `amount: i128`, `recipient: Address`, `initiator: Address`, `timestamp: u64` | Account-contract-local pending-withdrawal flow |
| `WithdrawalThresholdSetEvent` | `threshold: i128`, `timestamp: u64` | `set_withdrawal_threshold` | After writing the threshold | Threshold updated |

## Topic conventions

Because every event here publishes under a single derived-or-explicit topic Symbol (see [How Soroban events work here](#how-soroban-events-work-here-read-before-the-tables)), "filtering by topic" in the way a multi-topic event system would support does not apply. What you actually have available to filter on:

1. **The emitting contract's address** — every event carries the address of the contract that emitted it (returned alongside the event by `env.events().all()` in tests, and by the RPC/Horizon event-streaming APIs in production).
2. **The event's topic Symbol** (the struct name in snake_case, or the explicit override) — lets you distinguish "this was an `InvoicePaidEvent`" from "this was a `PaymentSplitRoutedEvent`" without inspecting the payload.
3. **Fields inside the data payload** — `merchant_id`, `invoice_id`, `token`, and so on are ordinary named fields in the event's data map, not part of the topic. Matching "all payments for one merchant" means decoding each `InvoicePaidEvent`'s data and comparing its `merchant_id` field — there is no server-side topic-based filter for this in the contract itself.

### Filter examples

**All payments for one merchant** — filter the stream to events with topic `invoice_paid_event` emitted by the `shade` contract address, then decode each one's data and keep those whose `merchant_id` field matches:

```rust
// Illustrative decode logic, following the same pattern the test helper
// `assert_latest_paid_event` in contracts/shade/src/tests/test_payment.rs uses.
use soroban_sdk::{Map, Symbol, TryIntoVal, Val};

let target_merchant_id: u64 = 1;
for (contract_id, _topics, data) in env.events().all().iter() {
    if contract_id != shade_contract_id {
        continue;
    }
    let Ok(fields): Result<Map<Symbol, Val>, _> = data.try_into_val(&env) else { continue };
    let Some(merchant_id_val) = fields.get(Symbol::new(&env, "merchant_id")) else { continue };
    let merchant_id: u64 = merchant_id_val.try_into_val(&env).unwrap();
    if merchant_id == target_merchant_id {
        // this is an InvoicePaidEvent (or any other event carrying merchant_id) for our merchant
    }
}
```

**All status changes for one invoice** — the events that mark an invoice status transition are `InvoiceCreatedEvent`, `InvoicePaidEvent`, `InvoiceCancelledEvent`, `InvoiceRefundedEvent`/`InvoicePartiallyRefundedEvent`, and `InvoiceAmendedEvent`; every one of them carries `invoice_id`. Decode each event on the `shade` contract address whose data map contains an `invoice_id` field equal to the target ID, in emission order (see [Ordering guarantees](#precise-semantics)) to reconstruct the invoice's full status history.

## Precise semantics

- **Only successful transactions leave events.** A transaction that panics (any `panic_with_error!` — status/amount/auth/expiry failures, `InsufficientBalance`, etc.) rolls back its entire effect, including every event it would have emitted. A reverted `pay_invoice` call, for example, produces **no** `InvoicePaidEvent`, `PaymentSplitRoutedEvent`, or `PlatformFeeRoutedEvent` — there is nothing to observe from a failed call except the transaction's own failure status.
- **Multiple events in one successful call preserve emission order.** [`env.events().all()`](../../contracts/shade/src/tests/test_payment.rs) returns events in the order they were published, and a single `pay_invoice` call is a concrete example: `PlatformFeeRoutedEvent` (emitted inside `finalize_route`, called from `route_from_payer`) is published before `InvoicePaidEvent` and `PaymentSplitRoutedEvent` (both emitted directly in `pay_invoice_partial_inner`, after the call to `route_from_payer` returns) — so the exact order for one full invoice payment is `PlatformFeeRoutedEvent`, then `InvoicePaidEvent`, then `PaymentSplitRoutedEvent`. [`test_payment.rs`](../../contracts/shade/src/tests/test_payment.rs)'s own `assert_latest_paid_event` helper explicitly does **not** assume the `InvoicePaidEvent` is the last event in the list — it searches from the end for the event whose data map has both `merchant_id` and `payer` fields — precisely because more than one event is emitted per call and their relative order, while deterministic, is not "the interesting one is always last."
- **Do not assume stronger guarantees than this.** There is no on-chain event log a contract can read back — events are a write-only, off-chain-observable side channel. A component cannot decide behavior based on "was this event already emitted," and nothing in this workspace does. Order is guaranteed only *within* one successful invocation, not across separate transactions unless the ledger's own transaction ordering is what you're relying on.

## Decoding a real event

Using this repository's own test conventions (`soroban-sdk = "23.4.0"`, per the root `Cargo.toml`) — the exact pattern `contracts/shade/src/tests/test_payment.rs` and `test_invoice.rs` use to assert on emitted events in an integration test:

```rust
use soroban_sdk::testutils::Events; // brings `.events()` into scope on Env
use soroban_sdk::{Map, Symbol, TryIntoVal, Val};

// After calling shade_client.pay_invoice(&customer, &invoice_id):
let events = env.events().all();
assert!(!events.is_empty());

// Each entry is (emitting_contract_address, topics, data).
let (event_contract_id, _topics, data) = events.get(events.len() - 1).unwrap();

// The default encoding is a Map<Symbol, Val> keyed by the struct's field names.
let fields: Map<Symbol, Val> = data.try_into_val(&env).unwrap();

let invoice_id: u64 = fields
    .get(Symbol::new(&env, "invoice_id"))
    .unwrap()
    .try_into_val(&env)
    .unwrap();
let amount: i128 = fields
    .get(Symbol::new(&env, "amount"))
    .unwrap()
    .try_into_val(&env)
    .unwrap();
```

This is exactly the shape `assert_latest_paid_event` and `assert_latest_invoice_event` use in the test suite — nothing here is an invented SDK API; every call (`env.events().all()`, `Map<Symbol, Val>`, `TryIntoVal`, `Symbol::new`) is copied from those test files. Outside a test harness (e.g. reading events from an RPC provider in a client application), the payload arrives as XDR that decodes to the same field-map shape; consult the Stellar SDK for your client language for its own equivalent of `try_into_val`.

## Related pages

- [Architecture overview](../architecture/overview.md) — the `pay_invoice` lifecycle these events instrument.
- [The component pattern](../architecture/component-pattern.md) — where in a component's code an event is emitted, and why.
- [Accepting payments](../guides/accepting-payments.md) — consuming these events from a payer-facing client.
- [`ShadeTrait` function reference](shade-interface.md)

← [Back to Reference](README.md)
