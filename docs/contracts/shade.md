# Shade contract reference

The `shade` contract (crate at [`contracts/shade/`](../../contracts/shade)) is Shade Protocol's main payment-gateway contract: merchant registration, invoicing, payments, refunds, fee routing and analytics. It is deployed once per protocol instance; there is no factory for `shade` itself (contrast with `account`, deployed per-merchant by `account_factory`).

## Contract interface

The trait defining this contract's public interface is [`ShadeTrait`](../../contracts/shade/src/shade_interface.rs), implemented verbatim by `Shade` in [`contracts/shade/src/shade.rs`](../../contracts/shade/src/shade.rs). Per the doc comment at the top of `shade_interface.rs`, that trait — not `contracts/shade/src/interface.rs` — is the single source of truth for the contract's exported functions.

> **Note:** `contracts/shade/src/interface.rs` also exists in the repository but is **not** referenced by [`contracts/shade/src/lib.rs`](../../contracts/shade/src/lib.rs) (`lib.rs` declares `pub mod shade_interface;` and does not declare an `interface` module at all). It is dead code as far as the compiled crate is concerned, and as checked out it contains malformed, duplicated `use` statements and a trait body that will not parse. Do not treat `interface.rs` as authoritative for anything; every function list on this page is drawn from `shade_interface.rs`.

`ShadeTrait` declares 86 functions, grouped below by domain: Admin & Config, Fees, Merchants, Merchant accounts, Invoices & Payments, Token Analytics, and Search.

For groups this page traces into their component implementation (Admin & Config, Merchants, Invoices & Payments, Fees), the **Auth** column reflects verified `require_auth`/role-check behavior. For groups not traced into their component source for this page (Bridge, Governance, Ticketing, Escrow, NFTs, Crowdfunding, Backer Rewards, Multi-Sig, Search, Stretch Goals, Creator Vesting, Fiat Goals, Analytics Exports), the **Auth** column reflects the parameter shape and doc comments in `shade_interface.rs` only — verify against the relevant `components/*.rs` file before relying on it for a security-sensitive decision.

## Deployment and initialization

`shade` has no constructor arguments beyond the Soroban deploy step itself. After deployment, `initialize(admin)` must be the first call:

```rust
fn initialize(env: Env, admin: Address);
```

- Panics with `ContractError::AlreadyInitialized` if `DataKey::Admin` is already set — `initialize` is callable exactly once per contract instance.
- Sets `DataKey::Admin` and `DataKey::PlatformAccount` to `admin`, and writes a `ContractInfo { admin, timestamp }` record to `DataKey::ContractInfo`.
- Emits `InitalizedEvent { admin, timestamp }`.
- Verified at [`shade.rs:35-51`](../../contracts/shade/src/shade.rs).

### Post-deploy configuration checklist

Ordered by dependency, derived from what each function reads/requires (verified for the first five rows against `admin.rs`/`account_factory.rs`; the remainder reflect the trait's own grouping and are not independently traced for this page):

| Step | Function | Authorizer | Configures | Required? |
|---|---|---|---|---|
| 1 | `initialize` | Deployer (sets `admin`) | `DataKey::Admin`, `DataKey::PlatformAccount` (defaults to admin), `ContractInfo` | Required — nothing else can be called until this succeeds. |
| 2 | `add_accepted_token` / `add_accepted_tokens` | `admin` | `DataKey::AcceptedTokens` | Required before any invoice or payment can use that token — `create_invoice` etc. call `admin::is_accepted_token` and panic with `TokenNotAccepted` otherwise. |
| 3 | `set_account_wasm_hash` | `admin` | `DataKey::AccountWasmHash` | Required before `account_factory::deploy_account` can deploy a per-merchant `account` contract — panics with `WasmHashNotSet` otherwise. Note: `register_merchant` itself does **not** call `deploy_account`; it defaults `Merchant.account` to the merchant's own address (see [Merchants](../concepts/merchants.md#registration)). Configure this if the deployment flow this operator uses depends on `account_factory::deploy_account`. |
| 4 | `set_platform_account` | `admin` | `DataKey::PlatformAccount` | Optional — `get_platform_account` falls back to the current admin address if never set explicitly. Configure explicitly if platform fees should not accrue to the admin's own wallet. |
| 5 | `set_fee` (per token) | `admin` | `DataKey::TokenFee(token)` | Optional — `get_fee` defaults to `0` if unset, and `calculate_fee`/`effective_fee_bps` treat a `0` base fee as no fee for that token. Configure per accepted token if the protocol should charge a fee. |
| 6 | `set_token_oracle` (per token) | `admin` | `DataKey::TokenOracle(token)` | Required only for tokens used with `create_fiat_invoice` (fiat-pegged pricing) — `resolve_fiat_invoice_amount` panics with `OracleNotConfigured` otherwise. Not required for crypto-priced (`create_invoice`) flows. |
| 7 | `grant_role` (Manager/Operator, as needed) | `admin` | `DataKey::Role(user, role)` | Optional — required only if the deployment needs `create_invoice_signed` (needs `Role::Manager`) or other role-gated calls beyond the admin itself. |

> **Warning:** Steps 2–7 are not enforced in any particular order by the contract itself beyond their individual read-time checks (e.g. `set_fee` requires the token to already be accepted, so step 2 must precede step 5 for that token). There is no single "setup complete" flag; an operator must track which of the optional steps their deployment needs.

## Contract-boundary guards

Verified by reading every guard call site in `shade.rs`, `pausable.rs`, `reentrancy.rs`, `access_control.rs`, and `core.rs`:

| Guard | Implementation | Where enforced |
|---|---|---|
| **Pause check** | `pausable_component::assert_not_paused(&env)`, panics with `ContractError::ContractPaused` if `DataKey::Paused == true`. | Called at the top of nearly every state-changing `ShadeTrait` method in `shade.rs` (invoice creation/payment/refund/void/amend, merchant registration and config, fee/token admin, subscriptions, bridge, governance, ticketing, escrow, campaigns, etc.). **Not** called on read-only getters (`get_invoice`, `get_merchant`, `is_paused`, etc.) or on `set_account_wasm_hash`, `restrict_merchant_account`, `set_merchant_status`, `verify_merchant`, `set_merchant_key`/`get_merchant_key`, `deploy_merchant_account`, `upgrade`, `propose_admin_transfer`/`accept_admin_transfer`, `has_role`/`grant_role`/`revoke_role` — verified absent from those call sites in `shade.rs`. |
| **Admin check** | `core::assert_admin(env, admin)` — calls `admin.require_auth()` then compares against `DataKey::Admin`, panicking with `NotAuthorized` on mismatch. | Every admin-only function in `admin.rs` (`add_accepted_token(s)`, `remove_accepted_token`, `set_account_wasm_hash`, `set_fee`, `set_platform_account`, `set_token_oracle`, `propose_fee`, `execute_fee`, `propose_admin_transfer`) and `access_control.rs` (`grant_role`, `revoke_role`). |
| **Role check** | `access_control::has_role(env, caller, role)`, or the stricter `assert_has_role` which also calls `require_auth`. The contract `Admin` address always satisfies `has_role` for any role (see `access_control.rs:38-42`). | `merchant::restrict_merchant_account` requires `Role::Admin` or `Role::Manager`. `invoice::create_invoice_signed` requires `Role::Manager`. `platform_fee::set_merchant_platform_fee`/`clear_merchant_platform_fee` require `Role::Admin` or `Role::Manager` via `assert_fee_operator`. |
| **`require_auth`** | Soroban's own authorization primitive; asserts the named `Address` authorized this invocation. | Used directly (not through `assert_admin`) by every function whose first authorization step is "the caller must be this specific address" — e.g. `merchant.require_auth()` in `register_merchant`, `set_merchant_key`, `set_merchant_accepted_tokens`; `merchant_address.require_auth()` in every invoice-mutating function (`create_invoice`, `refund_invoice`, `refund_invoice_partial`, `void_invoice`, `amend_invoice`); `payer.require_auth()` in `pay_invoice`/`pay_invoice_partial`/`pay_invoices_batch`; `buyer.require_auth()` in `claim_refund`. |
| **Reentrancy guard** | `reentrancy::enter`/`reentrancy::exit`, backed by `DataKey::ReentrancyStatus` (presence = locked). Panics with `ContractError::Reentrancy` if already entered. | Verified wrapping fund/config-moving `admin.rs` functions (`add_accepted_token(s)`, `remove_accepted_token`, `set_account_wasm_hash`, `set_fee`, `set_platform_account`, `set_token_oracle`, `propose_fee`, `execute_fee`) and `platform_fee.rs` (`set_merchant_platform_fee`, `clear_merchant_platform_fee`). **Not** verified present in `invoice.rs`'s refund/payment paths (`refund_invoice`, `refund_invoice_partial`, `pay_invoice_partial_inner`, `claim_refund`) — those call external token-transfer clients without an `enter`/`exit` pair in the code read for this page. |
| **Merchant status/verification checks** | `merchant::is_merchant_active`, `merchant::is_merchant_verified` — plain reads of `Merchant.active`/`Merchant.verified`, not auth guards. | See [Merchants: state-to-operation matrix](../concepts/merchants.md#state-to-operation-matrix) for exactly which calls enforce which flag; enforcement is per-function, not uniform. |

**Ordering, where it matters:** guards are called sequentially at the top of each function, in the order pause → auth/admin/role → business-logic validation. For example, `admin::add_accepted_token` calls `reentrancy::enter` first, then `core::assert_admin`, so a paused-but-reentered call is not possible (pause is checked one layer up, in `shade.rs`, before either). No function reorders this pattern in the components read for this page.

## Functions by domain

Each function is listed exactly once. See [`shade_interface.rs`](../../contracts/shade/src/shade_interface.rs) for full signatures — this table does not repeat parameter lists that add no information beyond the function name.

### Admin & configuration

| Function | Purpose | Auth |
|---|---|---|
| `initialize` | One-time contract bootstrap; sets the admin. | Deployer (no on-chain check beyond "not already initialized"). |
| `get_admin` | Read the current admin address. | None (public read). |
| `add_accepted_token` / `add_accepted_tokens` | Whitelist one or more tokens for payments. | `admin` (`assert_admin`). |
| `remove_accepted_token` | De-whitelist a token. | `admin`. |
| `is_accepted_token` | Whether a token is globally accepted. | None. |
| `set_account_wasm_hash` | Set the WASM hash `account_factory` deploys for merchant accounts. | `admin`. |
| `set_fee` / `get_fee` | Set/read a token's base platform fee (basis points). | Set: `admin`. Get: none. |
| `set_platform_account` / `get_platform_account` | Set/read the address platform fees are paid to. | Set: `admin`. Get: none. |
| `set_token_oracle` / `get_token_oracle` | Set/read the price oracle used for fiat-pegged invoices in a token. | Set: `admin`. Get: none. |
| `propose_fee` / `execute_fee` / `get_pending_fee` | Time-locked fee change: propose now, execute after `FEE_UPDATE_DELAY` (48h, `admin.rs:9`). | `admin` for propose/execute; get is public. |
| `propose_admin_transfer` / `accept_admin_transfer` | Two-step admin handover. | Propose: current `admin`. Accept: the proposed `new_admin` (`require_auth`), must match `DataKey::PendingAdmin`. |
| `pause` / `unpause` / `is_paused` | Emergency-stop the contract (see [Contract-boundary guards](#contract-boundary-guards)). | Pause/unpause: `admin` (`require_auth` + address match). Read: none. |
| `upgrade` | Replace the contract's WASM. | Not traced into `upgrade.rs` for this page. |
| `grant_role` / `revoke_role` / `has_role` | Assign/revoke/check `Role::Admin`/`Manager`/`Operator` for an address. | Grant/revoke: `admin`. Check: none. |
| `restrict_merchant_account` | Freeze/unfreeze a merchant's `account` contract via `MerchantAccountClient::restrict_account`. | `caller` with `Role::Admin` or `Role::Manager` (`require_auth` + role check). |
| `calculate_fee` | Pure query: platform fee that would apply to an amount for a merchant/token. | None (read-only, never panics on non-merchant addresses). |
| `compute_platform_fee_split` | Pure query: full gross/fee/merchant-amount split for an amount. | None. |
| `set_merchant_platform_fee` / `get_merchant_platform_fee` / `clear_merchant_platform_fee` | Per-merchant fee-bps override for a token. | Set/clear: `caller` with `Role::Admin` or `Role::Manager`. Get: none. |
| `get_merchant_volume` / `get_merchant_analytics` / `get_merchant_analytics_summary` | Per-merchant payment analytics. | None. |
| `get_token_analytics` / `get_token_volume` / `get_token_dominance_metrics` / `get_top_tokens_by_volume` / `get_token_market_share` | Global per-token analytics. | None. |
| `get_user_transactions` | A customer's recorded transaction history. | None. |

### Merchants

See [Merchants](../concepts/merchants.md) for the full behavioral model (registration, activation, verification, config, queries, state diagram, and enforcement matrix). Trait methods, listed once here for completeness:

`register_merchant`, `get_merchant`, `get_merchants`, `is_merchant`, `set_merchant_status`, `is_merchant_active`, `verify_merchant`, `is_merchant_verified`, `set_merchant_key`, `get_merchant_key`, `deploy_merchant_account`, `get_merchant_account`, `set_merchant_webhook`, `get_merchant_webhook`, `set_merchant_accepted_tokens`, `get_merchant_accepted_tokens`, `remove_merchant_accepted_token`, `is_token_accepted_for_merchant`, `set_auto_withdrawal_threshold`, `get_auto_withdrawal_threshold`, `set_auto_withdrawal_recipient`, `get_auto_withdrawal_recipient`, `find_merchant_id`, `search_merchants_paginated`.

### Invoices, payments, refunds, voids, amendments

See [Refunds and voids](../concepts/refunds-and-voids.md) for the full behavioral model of `refund_invoice`, `refund_invoice_partial`, `void_invoice`, and `amend_invoice`. Trait methods, listed once here for completeness:

| Function | Purpose | Auth |
|---|---|---|
| `create_invoice` | Create a crypto-priced invoice. | `merchant` (`require_auth`). |
| `create_fiat_invoice` | Create a fiat-pegged invoice, priced against the token's oracle at creation. | `merchant`. |
| `create_invoice_draft` / `finalize_invoice` | Create an editable draft, then finalize it into a payable `Pending` invoice. | `merchant` for both. |
| `create_invoice_signed` | Create an invoice via a relayer, authorized by the merchant's off-chain Ed25519 signature over a nonce rather than a direct Soroban auth entry. | `caller` requires `Role::Manager` and `require_auth`; the signature is separately verified against the merchant's registered key. |
| `get_invoice` / `get_invoices` / `search_invoices_paginated` | Look up one or many invoices. | None. |
| `resolve_invoice_amount` | The invoice's current payable amount — re-resolves the fiat quote for an unpaid `FixedFiat` invoice. | None. |
| `pay_invoice` / `pay_invoice_partial` / `pay_invoices_batch` | Pay an invoice in full, in part, or pay several invoices in one call. | `payer` (`require_auth`, once per batch for `pay_invoices_batch`). |
| `validate_payment_payload` | Validate a `PaymentPayload` (input/settlement token, route, slippage) without executing a payment. | Not traced into `payment.rs` for this page. |
| `refund_invoice` | Full refund of a `Paid` invoice within the refund window. | `merchant`. |
| `refund_invoice_partial` | Partial (or completing) refund of a `Paid`/`PartiallyRefunded` invoice. | `merchant`. |
| `claim_refund` | Buyer-initiated refund of an expired, unfulfilled `Paid`/`PartiallyPaid` invoice. | `buyer` (must be the recorded `payer`). |
| `void_invoice` | Cancel a `Pending` (unpaid) invoice. | `merchant`. |
| `amend_invoice` | Change the amount and/or description of a `Pending` invoice. | `merchant`. |

### Merchant accounts

`deploy_merchant_account` (merchant), `get_merchant_account`, `restrict_merchant_account` (admin or manager role), `verify_merchant_account` (admin), `add_account_token` (merchant), `set_account_withdraw_threshold` and `approve_account_withdrawal` (admin or manager role). See [Merchant account withdrawals](../concepts/merchant-account-withdrawals.md).

### Search and filtering

`search_invoices_paginated` and `search_merchants_paginated` are cursor-paginated: `InvoicePage` and `MerchantPage` carry a `PageInfo` with `next_cursor` and `has_next_page`. They are implemented in `search.rs`.

## Related pages

- [Storage model](../architecture/storage-model.md) — canonical reference for every `DataKey` variant this contract reads and writes.
- [Refunds and voids](../concepts/refunds-and-voids.md) — full behavior of `refund_invoice`, `refund_invoice_partial`, `void_invoice`, `amend_invoice`.
- [Merchants](../concepts/merchants.md) — full behavior of registration, activation, verification, and merchant configuration.
