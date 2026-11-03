# Architecture overview

How the two contracts in this workspace fit together, where state lives, how a request travels from the network boundary down to business logic, and why the protocol is shaped the way it is. Read this before [Workspace and Crate Layout](workspace-layout.md) or [the component pattern](component-pattern.md) if you want the "why" before the "how."

## The contracts, and how they relate

The workspace has two deployable contract crates under `contracts/`:

| Contract | Role |
|---|---|
| **`shade`** | The entry point and hub: merchants, invoices, fee routing, refunds, analytics and access control live here as components (see [the component pattern](component-pattern.md)). |
| **`account`** | Per-merchant vault, deployed by `shade`'s `account_factory` component. Holds merchant balances and services withdrawals and refunds. |

The hub calls two kinds of external contract: the merchant's `account`, through narrow traits declared at their call sites, and SEP-41 tokens. `invoice.rs` also calls a `PriceOracleClient` for fiat-priced invoices, an admin-configured oracle that is not part of this repository.

| Component | Account functions it calls |
|---|---|
| `invoice` | `refund` |
| `auto_withdrawal` | `sweep_to` |
| `merchant_account` | `restrict_account`, `verify_account`, `add_token`, `set_withdrawal_threshold`, `approve_withdrawal` |
| `account_factory` | `initialize` |

Contracts outside the gateway, such as escrow, subscriptions, ticketing and crowdfunding, are in [`example/`](../../example/README.md). See [ADR 0007](decisions/0007-core-gateway-scope.md).

## Where persistent state lives

Every contract owns its own persistent storage; Soroban gives each deployed contract address its own isolated ledger-entry namespace, so there is no shared database between `shade`, `account`, and the standalone contracts. Within `shade`:

- `types.rs` defines the storage-key enum, `DataKey`. Soroban caps every `#[contracttype]` enum at 50 cases.
- Each component reads and writes only the key variants that belong to its domain (`pausable` owns `DataKey::Paused`, `invoice` owns `DataKey::Invoice`/`DataKey::InvoiceCount`, and so on). Cross-component reads go through the owning component's public getters — see [the component pattern's boundary rules](component-pattern.md#what-makes-a-good-component-boundary).
- See [Storage model](storage-model.md) for each key.

## How a request reaches business logic: the VM boundary to component logic

A client invokes a `shade` contract function (directly, or via a Soroban RPC `simulateTransaction`/`sendTransaction`); the Soroban host loads the contract's WASM, resolves the function against the exported `#[contractimpl]` methods, and calls into `shade.rs`. From there:

```mermaid
flowchart TD
    Client[Client / wallet] -->|invoke function| Host[Soroban host / VM]
    Host -->|dispatch to exported fn| ShadeImpl["shade.rs\n#[contractimpl] impl ShadeTrait for Shade"]
    ShadeImpl -->|"pausable_component::assert_not_paused(&env)\n(most state-changing methods)"| Guard{Paused?}
    Guard -->|yes: panic ContractPaused| Host
    Guard -->|no| Component["components/*.rs\n(auth, validation, storage, events)"]
    Component -->|storage read/write| Ledger[(Persistent storage\nfor this contract address)]
    Component -->|cross-contract call, where needed| Account["account contract\n(MerchantAccountRefund / \nMerchantAccountContract / withdraw_to)"]
    Component -->|emit| Events[[contractevent]]
    Component -->|return value or panic| ShadeImpl
    ShadeImpl --> Host
    Host --> Client
```

`shade_interface.rs`'s `ShadeTrait` is what makes the WASM's exported function list and the `shade.rs` implementation provably consistent — the trait is the single source of truth for the public surface, and `#[contractimpl] impl ShadeTrait for Shade` is a compile error if `shade.rs` doesn't implement every trait method with a matching signature.

## Request lifecycle: `pay_invoice`

Tracing an actual state-changing call end to end, verified against [`contracts/shade/src/shade.rs`](../../contracts/shade/src/shade.rs) and [`contracts/shade/src/components/invoice.rs`](../../contracts/shade/src/components/invoice.rs) — this is the exact guard and step order, not an assumed one:

1. Client calls `pay_invoice(payer, invoice_id)`.
2. `shade.rs::pay_invoice` calls `pausable_component::assert_not_paused(&env)` — panics with `ContractPaused` (error 9) if the contract is paused. This is the *only* guard `shade.rs` applies itself before delegating.
3. `shade.rs::pay_invoice` delegates to `invoice_component::pay_invoice(&env, &payer, invoice_id)`.
4. `invoice::pay_invoice` calls `payer.require_auth()` — the payer must have authorized this specific invocation.
5. `invoice::pay_invoice` calls `pay_invoice_inner`, which loads the invoice, checks its status is `Pending` or `PartiallyPaid` (else panics `InvalidInvoiceStatus`), computes the remaining amount via `resolve_invoice_amount` (which, for a fiat-priced invoice, re-queries the oracle), and calls `pay_invoice_partial_inner` with that remaining amount.
6. `pay_invoice_partial_inner` re-validates: amount positive, invoice not expired, status still payable, amount does not exceed the invoice total, token still globally accepted. It re-resolves the fiat quote (`refresh_fiat_invoice_quote`) if applicable.
7. It resolves the merchant's address and merchant-account address, then calls `platform_fee::route_from_payer`, which computes the merchant/platform split (`compute_split`) and **executes the token transfers** — this is where funds actually move, via the SEP-41 token contract's `transfer`.
8. It updates the `Invoice` record (`amount_paid`, `status`, `payer`, `date_paid` if now fully paid) and writes it back to storage.
9. It emits `InvoicePaidEvent`, then `PaymentSplitRoutedEvent` (via `finalize_route` inside `route_from_payer`, which itself emits `PlatformFeeRoutedEvent` before returning — so a single `pay_invoice` call can emit `PlatformFeeRoutedEvent`, then `InvoicePaidEvent`, then `PaymentSplitRoutedEvent`, in that order).
10. It calls `history::record_transaction`, appending a `Transaction` to the payer's log (no event).
11. It calls `auto_withdrawal::check_and_trigger_auto_withdrawal`, which may itself move funds again and emit `AutoWithdrawalTriggeredEvent` if the merchant's configured threshold was just crossed, or `AutoWithdrawalFailedEvent` if the sweep failed. A failed sweep does not revert the payment.

**Guard order for `pay_invoice` specifically: pause check (step 2) → `require_auth` (step 4) → status/amount/expiry/token validation (steps 5–6) → fee computation and token transfer (step 7) → storage write (step 8) → events (step 9) → history (step 10) → auto-withdrawal sweep (step 11).** `pay_invoice` does not call the reentrancy guard (`reentrancy::enter`/`exit`) — that guard is used selectively by a different set of functions (see [the component pattern's guard discussion](component-pattern.md#how-authorization-guards-storage-and-events-interact-with-the-boundary)); do not assume it wraps every fund-moving path.

## Layering responsibilities, for reviewing a PR

| Layer | Owns | A PR touching it should... |
|---|---|---|
| `shade_interface.rs` | The complete public function signature list (`ShadeTrait`) | Add exactly one line per new public function; never contain logic. |
| `shade.rs` | Wiring: the pause guard (where applicable) plus delegation to one component call | Stay at "guard, then delegate" — if a method body does anything else, that logic belongs in a component. |
| `components/` | All actual logic: auth, validation, storage access, cross-contract calls, event emission | Own only its domain's storage keys; reach other domains through the owning component's public functions, never raw storage access. |
| `types.rs` | Every `#[contracttype]` record and storage-key enum | New persisted fields/variants only ever added, never renamed/retyped/moved — see [Upgradeability's storage compatibility rules](upgradeability.md#storage-compatibility-rules). |
| `events.rs` | Every `#[contractevent]` struct and its `publish_*` helper | New events added here, never constructed ad hoc inside a component. |
| `errors.rs` | Every `#[contracterror]` enum, partitioned into non-overlapping numeric ranges per domain | New error variants appended within the domain's assigned range; never renumber an existing one. |

## Architectural rationale

**One hub contract with components, rather than many small contracts, for the core payment domain.** Invoices, merchants and fees all need to agree on shared concepts — a merchant's ID and active/verified state, the accepted-token whitelist, the platform fee split — cheaply and atomically. Keeping them in one contract with one storage namespace means `pay_invoice` can read the merchant's account address, compute the fee, transfer tokens, and record the payment in a single atomic call with no cross-contract round trip for the shared state. The component pattern (see [component-pattern.md](component-pattern.md)) buys the maintainability of separate modules without paying the deployment, cross-contract-call, and atomicity cost of separate contracts for logic that is this interdependent.

**A factory for per-merchant contracts, where isolation matters more than shared state.** A merchant's `account` isolates that merchant's balance under its own address, so a restriction or bug in one instance cannot touch another's funds. `account_factory` deploys one instance per merchant from a registered WASM hash. See [Cross-Contract Calls and the Factory Pattern](cross-contract-calls.md) for the mechanics.

**Events as the off-chain integration surface.** Nothing in this contract exposes a webhook or push mechanism; every state transition that matters to an integrator is instead published as a `#[contractevent]` (see [Events reference](../reference/events.md)). An indexer or backend is expected to watch the event stream rather than polling every query function, which is both cheaper for the integrator and keeps the contract itself free of any notion of "who is listening."

## Related pages

- [The component pattern](component-pattern.md) — the module-level structure inside `shade`, with a full inventory of the components.
- [Workspace and Crate Layout](workspace-layout.md) — the Cargo workspace, directory tree, and per-crate file conventions.
- [Cross-Contract Calls and the Factory Pattern](cross-contract-calls.md) — factory deployment mechanics, SEP-41 token interactions, and authorization propagation across contract boundaries.
- [Upgradeability, WASM hash management, and versioning](upgradeability.md) — how the hub's code (and, separately, factory-deployed WASM) can change after deployment, and the storage-compatibility rules that govern it.
- [`ShadeTrait` function reference](../reference/shade-interface.md)
- [Events reference](../reference/events.md)

← [Back to Architecture](README.md)
