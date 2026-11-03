# The component pattern

How business logic is organized inside the `shade` contract, and the rules that keep independently-owned modules from turning into a tangle of cross-calls. This page is for anyone adding or reviewing a feature under `contracts/shade/src/components/`.

## The three-file pattern

Every feature in `shade` is split across three files with a fixed division of labor:

1. **A component module** under `contracts/shade/src/components/` — the actual logic: storage reads/writes, validation, authorization checks specific to the feature, and event emission. Components are plain Rust modules (`pub fn`, no `#[contract]` or `#[contractimpl]`); they are never invoked directly by a client.
2. **`shade_interface.rs`** — the single `#[contracttrait] pub trait ShadeTrait` that is the contract's complete public surface. It declares every exported function's signature (parameter order, types, return type) with no bodies.
3. **`shade.rs`** — the `#[contractimpl] impl ShadeTrait for Shade` block: one method per trait function, each just wide enough to apply cross-cutting guards (mainly `pausable_component::assert_not_paused`) and then delegate to the matching component function.

## Conventions every component follows

- **`&Env` is always the first parameter.** Components take `env: &Env` (a borrow), not an owned `Env` — `shade.rs` owns the `Env` value and hands out borrows to every component call in a method.
- **Auth is asserted inside the component, not in `shade.rs`.** `shade.rs` methods do not call `require_auth()` themselves . The `Address.require_auth()` call and any role/ownership check lives at the top of the component function, so the same authorization logic runs whether the function is reached through `ShadeTrait` or called directly in a test.
- **Failure is `panic_with_error!`, not `Result`.** No function in a component returns `Result<T, E>`. Every error path calls `panic_with_error!(env, SomeError::Variant)`, which aborts the whole invocation and rolls back all state changes made so far in the transaction — see [Cross-Contract Calls: Failure Semantics](cross-contract-calls.md#failure-semantics--atomic-guarantees). There is no partial-success return value to check.
- **Storage access goes through the owning component's helpers.** A component reads and writes only the `DataKey` variants that belong to its feature (see [Storage and layering](#storage-and-layering) below). Other components read that state through public functions the owning component exports (e.g. `merchant::get_merchant_id`, `admin::is_accepted_token`), not by reaching into storage directly.
- **Events are emitted after the state write that makes them true**, generally as the last step of the function (occasionally followed only by a call into another component, such as `history::record_transaction` or `auto_withdrawal::check_and_trigger_auto_withdrawal`, that itself emits further events). A component never emits an event and then goes on to a step that can still panic.
- **Read-only query functions still validate their input** (existence, e.g. `merchant::get_merchant` panics with `MerchantNotFound` for an unknown ID) but never emit events or require auth, except where a query is deliberately telemetry-generating (see `search.rs`'s `*_search_executed_event`s).

## Worked example: `pause`

`pause` is one of the smallest complete slices in the codebase and touches all three files without any of the extra machinery (fee routing, cross-contract calls) that makes bigger examples noisy.

**1. Component (`contracts/shade/src/components/pausable.rs`):**

```rust
pub fn pause(env: &Env, admin: &Address) {
    admin.require_auth();

    if core::get_admin(env) != admin.clone() {
        panic_with_error!(env, ContractError::NotAuthorized);
    }

    assert_not_paused(env);

    env.storage().persistent().set(&DataKey::Paused, &true);

    events::publish_contract_paused_event(env, admin.clone(), env.ledger().timestamp());
}
```

Auth, the admin check, the state guard (`assert_not_paused` — you can't pause an already-paused contract), the storage write, then the event. Everything the function needs to be correct on its own, independent of how it's reached.

**2. Interface (`contracts/shade/src/shade_interface.rs`):**

```rust
fn pause(env: Env, admin: Address);
```

Just the signature, as one line among ~180 others in the trait.

**3. Wiring (`contracts/shade/src/shade.rs`):**

```rust
fn pause(env: Env, admin: Address) {
    pausable_component::pause(&env, &admin);
}
```

No guard in front of `pause` itself — pausing has nothing to pause-guard against — just the delegation. Contrast with a state-changing function like `add_accepted_token`, which is guarded:

```rust
fn add_accepted_token(env: Env, admin: Address, token: Address) {
    pausable_component::assert_not_paused(&env);
    admin_component::add_accepted_token(&env, &admin, &token);
}
```

That one extra line — `pausable_component::assert_not_paused(&env)` — is the entire job of `shade.rs` for the ~150 state-changing methods that need it. `shade.rs` is deliberately a thin wiring layer: if a `shade.rs` method body is more than a guard call plus a delegation, that logic almost always belongs in the component instead.

```mermaid
flowchart LR
    Client -->|pause(admin)| ShadeTrait["shade_interface.rs\nShadeTrait::pause"]
    ShadeTrait -.signature only.-> Shade["shade.rs\nimpl ShadeTrait for Shade"]
    Shade -->|delegates| Pausable["components/pausable.rs\npause()"]
    Pausable -->|require_auth, assert_admin, assert_not_paused| Pausable
    Pausable -->|storage write| Storage[(DataKey::Paused)]
    Pausable -->|emit| Event[[ContractPausedEvent]]
```

A guarded example follows the same shape with one extra hop: `shade.rs` calls `pausable_component::assert_not_paused(&env)` before delegating to the target component, so the pause check runs once, centrally, rather than being duplicated inside every component.

## Component inventory

Every module declared in [`contracts/shade/src/components/mod.rs`](../../contracts/shade/src/components/mod.rs).

| Module | Responsibility | Key public functions | Related concept doc |
|---|---|---|---|
| `access_control` | Grants, revokes and checks `Role` beyond the single contract admin | `grant_role`, `revoke_role`, `has_role`, `assert_has_role` | [Access control](../security/access-control.md) |
| `account_factory` | Deploys a merchant's `account` contract from the stored WASM hash | `deploy_account` | [Upgradeability](upgradeability.md#set_account_wasm_hash) |
| `admin` | Accepted-token whitelist, account WASM hash, platform account, oracle config, admin transfer | `add_accepted_token`, `set_platform_account`, `set_token_oracle`, `propose_admin_transfer` | [Token whitelisting](../concepts/token-whitelisting.md) |
| `analytics` | Merchant and token payment analytics | `record_merchant_payment`, `get_merchant_analytics`, `get_token_analytics` | [Token analytics](../concepts/token-analytics.md) |
| `auto_withdrawal` | Per-merchant, per-token sweep threshold; sweeps an account after a payment crosses it | `set_auto_withdrawal_threshold`, `check_and_trigger_auto_withdrawal` | [Auto-withdrawal](../concepts/auto-withdrawal.md) |
| `core` | Initialization, reading the admin address and asserting a caller is the admin | `initialize`, `get_admin`, `assert_admin` | — |
| `history` | Append-only per-user transaction log | `record_transaction`, `get_user_transactions` | [Analytics and history](../concepts/analytics-and-history.md) |
| `invoice` | The core payment object: creation (crypto, fiat, draft, signed), payment (full, partial, batch), refund, void, amend | `create_invoice`, `pay_invoice`, `refund_invoice`, `resolve_invoice_amount` | [Invoice lifecycle](../concepts/invoice-lifecycle.md) |
| `merchant` | Merchant registration, activation, verification, webhook and accepted-token settings | `register_merchant`, `is_merchant_active`, `get_merchant_account`, `set_merchant_accepted_tokens` | [Merchants](../concepts/merchants.md) |
| `merchant_account` | The hub's side of a merchant's `account`: deployment and manager actions | `deploy_merchant_account`, `restrict_merchant_account`, `approve_account_withdrawal` | [Account withdrawals](../concepts/merchant-account-withdrawals.md) |
| `pausable` | The global emergency pause flag and its guard | `pause`, `unpause`, `assert_not_paused` | [Pausable](../security/pausable.md) |
| `payment` | Validates a `PaymentPayload` (direct or swap route, slippage bounds) | `validate_payment_payload` | [Payment payloads](../concepts/payment-payloads-and-routing.md) |
| `platform_fee` | Fee configuration, including the time lock, and the merchant/platform split | `set_fee`, `propose_fee`, `execute_fee`, `calculate_fee`, `compute_split`, `route_from_payer` | [Fees](../concepts/fees.md) |
| `reentrancy` | The storage-flag guard used by admin and fee functions | `enter`, `exit` | [Reentrancy](../security/reentrancy.md) |
| `search` | Cursor-based pagination over invoices and merchants | `search_invoices_paginated`, `search_merchants_paginated` | — |
| `signature_util` | Builds the signed-invoice message and verifies the merchant's ed25519 signature and nonce | `verify_invoice_signature`, `invalidate_nonce` | [Signatures](../security/signatures.md) |
| `upgrade` | The admin path for replacing the hub's WASM | `upgrade` | [Upgradeability](upgradeability.md#the-admin-path) |

## What makes a good component boundary

The pattern that holds is: **a component owns exactly the storage keys for its feature domain, and every other component reaches that state only through its public functions** — never through a raw `env.storage()` call on a key another module owns. Concretely:

- **Direct function call is the norm.** `invoice::pay_invoice_partial_inner` calls `merchant::get_merchant_account`, `admin::is_accepted_token`, and `platform_fee::route_from_payer` directly — components import each other's public functions like any other Rust module dependency. There is no message-passing or event-driven indirection between components; it's all synchronous, in-process function calls within the same contract invocation.
- **No component reaches into another's storage.** `auto_withdrawal` needs a merchant's record, so it calls `merchant::get_merchant` and `merchant::save_merchant` rather than reading `DataKey::Merchant` itself. This keeps a component's storage layout an implementation detail.
- **Shared cross-cutting concerns go through their own single-purpose component**, not through copy-pasted logic: every fund-moving admin/fee function calls `reentrancy::enter`/`exit`; every merchant-facing mutator calls `pausable_component::assert_not_paused` (via `shade.rs`, not inline); every fee-computing path goes through `platform_fee::compute_split` rather than each caller doing its own basis-point math.
- **Cross-contract calls (to `account`, price oracles, etc.) are declared as a local `#[contractclient]` trait right where they're used.** `invoice.rs` declares `MerchantAccountRefund` and `PriceOracle`, `auto_withdrawal.rs` declares `MerchantAccountSweep`, and `merchant_account.rs` declares the manager actions. See [Cross-Contract Calls](cross-contract-calls.md).
- **A component that becomes two concerns should split.** Fee logic and analytics were split out of `admin` for this reason.

## How authorization, guards, storage, and events interact with the boundary

These four concerns are handled at different layers, and the layering is what the pattern buys you:

- **Pause guard** — centralized in `shade.rs`. This is the *only* cross-cutting check `shade.rs` applies itself rather than delegating; every other state-changing method's first real line is `pausable_component::assert_not_paused(&env)`, so a reviewer can audit pause coverage by scanning one file instead of every component.
- **Reentrancy guard** — decentralized, opt-in per function. `reentrancy::enter`/`exit` is called from inside the handful of functions that make untrusted external calls after a state read but need to prevent being re-entered before they finish (`admin.rs`'s token and config functions, `platform_fee.rs`'s fee setters). It is not applied uniformly — `invoice::pay_invoice` does not call `reentrancy::enter`, for instance — so don't assume every fund-moving path is guarded; check the specific component.
- **Feature-specific authorization** — always inside the component, always as close to the top of the function as possible, using `require_auth()` plus (where relevant) an ownership or role check against stored state (`core::assert_admin`, `merchant::get_merchant_id(...) == invoice.merchant_id`, `access_control::has_role`).
- **Storage ownership** — one component per key; see [Storage model](storage-model.md).
- **Event emission** — always inside the component, at the point where the state change it describes has just been committed, using the helpers in [`contracts/shade/src/events.rs`](../../contracts/shade/src/events.rs). See [Events reference](../reference/events.md) for the full catalog and topic conventions.

## Storage and layering

- **`types.rs`** defines every stored record and the storage-key enum, `DataKey`. Soroban caps every `#[contracttype]` enum at 50 cases, so split keys into a second enum if `DataKey` nears that.
- **`errors.rs`** holds `ContractError`. Append new codes and never renumber, so a code observed on-chain always means the same thing.
- **`events.rs`** holds every `#[contractevent]` struct and its `publish_*` helper. A component never constructs an event struct outside this file's helpers.
- **`components/`** holds the logic, one module per feature, per the inventory above.
- **`shade_interface.rs`** and **`shade.rs`** are the two files described in [The three-file pattern](#the-three-file-pattern) — the public surface and its wiring.

For the workspace-level view of how these files relate to the rest of the contracts (not just `shade`), see [Architecture overview](overview.md).

## Related pages

- [Architecture overview](overview.md)
- [Cross-Contract Calls and the Factory Pattern](cross-contract-calls.md)
- [Upgradeability, WASM hash management, and versioning](upgradeability.md)
- [Workspace and Crate Layout](workspace-layout.md)
- [`ShadeTrait` function reference](../reference/shade-interface.md)
- [Events reference](../reference/events.md)

← [Back to Architecture](README.md)
