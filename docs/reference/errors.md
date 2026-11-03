# Errors reference

Every [`#[contracterror]`](https://docs.rs/soroban-sdk) enum in the workspace, the numeric code each variant raises, the functions that can raise it, the condition that triggers it, and the corrective action. Read this page after a failed transaction to go from an error code to a fix.

## Code uniqueness

Error codes are scoped to the contract that raised them, not global. `1` in [`account`](#account-contracts-src-errorsrs) means `AlreadyInitialized`; `1` in [`shade`](#shade-contracts-shade-src-errorsrs) means `NotAuthorized`. Always pair a numeric code with the contract it came from — the contract ID in the failed transaction tells you which table below to read.

The `shade` crate is the one exception with internal structure: it partitions nine error enums into non-overlapping numeric ranges within the same contract, specifically so a raw code observed on-chain against the `shade` contract ID maps back to exactly one variant. See [`contracts/shade/src/errors.rs#L1-L23`](../../contracts/shade/src/errors.rs):

| Range | Enum |
|---|---|
| 1–99 | `ContractError` |

`account` has its own error codes, which overlap with `shade`'s. A code alone does not identify the variant; check which contract produced it.

## `shade` (`contracts/shade/src/errors.rs`)

The `shade` hub contract is the primary payment-processing entry point most integrators call against. Its errors are split into nine enums because Soroban caps a single `#[contracterror]` enum at 50 variants.

### `ContractError`

| Error | Code | Raised by | Trigger | Corrective action |
|---|---:|---|---|---|
| `NotAuthorized` | 1 | Most admin-only and role-gated functions (e.g. `set_fee`, `add_accepted_token`, `pause`, `grant_role`, `restrict_merchant_account`) | Caller does not hold the required role, or `require_auth` fails for the claimed address. | Call with the correct signer — usually the current admin (`get_admin`) or a granted [role](../glossary.md#role). |
| `AlreadyInitialized` | 2 | `initialize` | `initialize` was already called once on this contract instance. | Do not re-initialize; use `get_admin` to confirm the current admin. |
| `NotInitialized` | 3 | Internal reads of `DataKey::Admin` before `initialize` has run | The contract has never been initialized. | Call `initialize` first. |
| `Reentrancy` | 4 | Fund-moving functions guarded by [`reentrancy::enter`/`exit`](../../contracts/shade/src/components/reentrancy.rs) | A guarded function is re-entered before its first call completes (e.g. via a malicious token callback). | Indicates an attempted reentrant call; not user-recoverable. Review the calling transaction for unexpected cross-contract calls. |
| `MerchantAlreadyRegistered` | 5 | `register_merchant` | The calling address already has a `DataKey::MerchantId` entry. | Use `find_merchant_id` or `is_merchant` to check registration before calling `register_merchant` again. |
| `MerchantNotFound` | 6 | `get_merchant`, `set_merchant_status`, `verify_merchant`, `set_merchant_key`, `deploy_merchant_account`, `set_merchant_accepted_tokens`, `set_merchant_webhook`, and most merchant/invoice/escrow/campaign functions that resolve a merchant by ID or address | The merchant ID exceeds the registered count, or the address has no `DataKey::MerchantId` entry. | Confirm registration with `is_merchant` / `find_merchant_id`, or call `register_merchant` first. |
| `InvalidAmount` | 7 | Most amount-taking functions across invoices and fees (e.g. `create_invoice`, `pay_invoice_partial`) | The supplied amount is zero, negative, or otherwise fails the function's amount validation. | Supply a positive amount consistent with the function's documented constraints. |
| `InvoiceNotFound` | 8 | `get_invoice` and every function that loads an invoice by ID | The invoice ID does not exist. | Confirm the ID from `create_invoice`'s return value or `get_invoices`/`search_invoices_paginated`. |
| `ContractPaused` | 9 | Every state-mutating function guarded by [`pausable::assert_not_paused`](../../contracts/shade/src/components/pausable.rs) (does **not** guard `upgrade` — see [Upgradeability](../architecture/upgradeability.md)) | Admin has called `pause` and not yet called `unpause`. | Wait for `unpause`, or contact the admin. Check with `is_paused`. |
| `ContractNotPaused` | 10 | `unpause` | `unpause` called while the contract is not paused. | No action needed; the contract is already active. |
| `MerchantKeyNotFound` | 11 | `get_merchant_key`, `create_invoice_signed` (via `verify_invoice_signature`) | The merchant never called `set_merchant_key`. | Call `set_merchant_key` before using signed-invoice creation. See [Signed invoices](../security/signatures.md). |
| `TokenNotAccepted` | 12 | `set_merchant_accepted_tokens`, `create_invoice` (token whitelist check) | The token is not in the global `DataKey::AcceptedTokens` list. | Ask the admin to call `add_accepted_token` for this token first. |
| `NonceAlreadyUsed` | 14 | `create_invoice_signed` | The 32-byte nonce was already consumed by a prior signed invoice from this merchant. | Use a fresh nonce; see [Nonce and replay protection](../security/signatures.md#nonce-and-replay-protection). |
| `InvalidInvoiceStatus` | 16 | `finalize_invoice`, `pay_invoice`, `refund_invoice`, `claim_refund`, `void_invoice` | The invoice is not in the status the operation requires (e.g. refunding an invoice that was never paid). | Read `get_invoice` and confirm `status` matches what the operation expects before calling. |
| `RefundPeriodExpired` | 17 | `refund_invoice` | The refund window (measured from `date_paid`) has elapsed. | No further refund is possible through this path once the window closes. |
| `WasmHashNotSet` | 18 | `upgrade`, merchant-account deployment inside `register_merchant` | `set_account_wasm_hash` (for deployment) or a valid upgrade target has not been configured. | Admin must call `set_account_wasm_hash` before merchants can register, or supply a valid hash to `upgrade`. |
| `MerchantAccountNotSet` | 20 | `get_merchant_account` | No `DataKey::MerchantAccount` entry exists for this merchant ID. | Call `deploy_merchant_account` first. |
| `InvoiceExpired` | 27 | `pay_invoice`, `pay_invoice_partial` | The current ledger timestamp exceeds the invoice's `expires_at`. | The merchant must create a new invoice; expired invoices cannot be paid. |
| `InvoiceNotPaid` | 28 | Internal invoice-settlement paths | An operation that requires a paid invoice was attempted on one that is not paid. | Confirm invoice status is `Paid` via `get_invoice` before calling. |
| `PayerNotAvailable` | 29 | Internal invoice-settlement paths | No payer is recorded on the invoice at a point where one is required. | Indicates the invoice was never paid; not directly user-triggerable through a normal flow. |
| `InsufficientBalance` | 30 | `pay_invoice`, `pay_invoice_partial` | The payer/customer's token balance or SEP-41 allowance is insufficient. | Fund the paying address, or increase the token allowance for the contract. |
| `MerchantNotActive` | 32 | `create_invoice`, `set_merchant_accepted_tokens`, `remove_merchant_accepted_token` | `set_merchant_status` has set `active = false` for this merchant. | Ask the admin to call `set_merchant_status(admin, merchant_id, true)`. |
| `InvalidDescription` | 33 | `create_invoice`, `create_fiat_invoice`, `create_invoice_draft` | The description string fails length or content validation. | Shorten or otherwise correct the description. |
| `OracleNotConfigured` | 34 | `get_token_oracle`, `create_fiat_invoice`, `resolve_invoice_amount` | No `OracleConfig` has been set for the token via `set_token_oracle`. | Admin must call `set_token_oracle` for this token before fiat pricing can resolve. |
| `OraclePriceUnavailable` | 35 | `resolve_invoice_amount` and other fiat-resolution paths | The configured oracle contract returned no usable price. | Retry once the oracle publishes a price, or contact the oracle operator. |
| `TokenNotAcceptedByMerchant` | 41 | `pay_invoice` | The token is globally accepted but not in this merchant's own accepted-tokens list (see [`is_token_accepted_for_merchant`](../../contracts/shade/src/components/merchant.rs)). | Merchant must call `set_merchant_accepted_tokens` to include this token, or pay in a token the merchant already accepts. |
| `FeeUpdateTooEarly` | 42 | `execute_fee` | The fee time-lock configured by `propose_fee` has not elapsed. | Wait for the time-lock to elapse before calling `execute_fee`. |
| `NoPendingFeeUpdate` | 43 | `execute_fee`, `get_pending_fee` | No fee change is currently staged for this token. | Call `propose_fee` first. |
| `InvalidSwapPath` | 44 | `validate_payment_payload` | The `PaymentPayload`'s swap route is malformed. | Correct the `PaymentRoute` before resubmitting. |
| `InvalidSlippage` | 45 | `validate_payment_payload` | The requested slippage basis points exceed the accepted maximum. | Lower the slippage tolerance. |
| `InvoiceNotExpired` | 50 | `claim_refund` | The invoice has not expired yet. |
| `InvoiceAlreadyRefunded` | 51 | `claim_refund` | The invoice was already fully refunded. |
| `MerchantAccountAlreadySet` | 52 | `deploy_merchant_account` | The merchant already has an account contract. |

## `account` (`contracts/account/src/errors.rs`)

Errors from the per-merchant `account` contract (deployed via the hub's `set_account_wasm_hash`/`register_merchant` flow). This contract's `ContractError` enum is independent of `shade`'s — code `1` here is `AlreadyInitialized`, not `NotAuthorized`.

| Error | Code | Trigger | Corrective action |
|---|---:|---|---|
| `AlreadyInitialized` | 1 | The account contract's `initialize` was already called. | No action needed; the account is already set up. |
| `NotInitialized` | 2 | An operation was attempted before `initialize` ran. | The hub deploys and initializes accounts automatically during `register_merchant`; this indicates a deployment issue, not a merchant-side fix. |
| `NotAuthorized` | 3 | Caller is not the account's owning merchant or manager. | Call from the merchant address the account was deployed for. |
| `InsufficientBalance` | 4 | A withdrawal or disbursement exceeds the account's held balance. | Reduce the requested amount, or wait for further settlement. |
| `AccountRestricted` | 5 | The account was restricted via the hub's `restrict_merchant_account`. | Contact the platform admin; only an admin/manager role can lift the restriction. |
| `WithdrawalRequestNotFound` | 6 | `approve_withdrawal`, `get_withdrawal_request` | No withdrawal request has that ID. |
| `InvalidWithdrawalStatus` | 7 | `approve_withdrawal` | The request is no longer pending. |
| `InvalidThreshold` | 8 | `set_withdrawal_threshold` | The threshold is negative. |

## Reading an error from a failed transaction

### Soroban CLI

When a `stellar contract invoke` call panics with a `#[contracterror]` variant, the CLI prints the error inline in its failure output, in the form `HostError: Error(Contract, #<code>)`. For example, calling `pay_invoice` against an invoice paid in a token the merchant does not personally accept (only globally whitelisted) fails like this against a local/testnet deployment:

```bash
stellar contract invoke \
  --id $SHADE_ID \
  --source payer \
  --network testnet \
  -- pay_invoice \
  --payer $(stellar keys address payer) \
  --invoice_id 1
```

```text
❌ error: transaction simulation failed: HostError: Error(Contract, #41)
```

The `#41` is the raw discriminant. Match it against the **contract that raised it** — `$SHADE_ID` here is the `shade` contract, so `41` resolves to `ContractError::TokenNotAcceptedByMerchant` in the table above, not to any other contract's code `41`.

> **Note:** The exact wording of CLI failure output (`HostError: Error(Contract, #N)` vs. a differently formatted panic message) depends on the installed `stellar-cli` version. The example above reflects the CLI version pinned in [Prerequisites](../getting-started/prerequisites.md) (23.x) at the time of writing; always confirm against your installed `stellar --version` if the format looks different.

### SDK / RPC client errors

A Soroban RPC client (for example a TypeScript or Rust SDK built against `soroban-rpc`) distinguishes several failure layers, and only the last one maps to this page's tables:

| Layer | What it means | Where the contract error code appears |
|---|---|---|
| Transport/RPC failure | The RPC endpoint was unreachable, rate-limited, or returned malformed JSON. | Not applicable — no contract code was ever reached. |
| Transaction simulation/submission failure | The transaction failed before or during execution for a reason outside contract logic (insufficient fee, bad sequence number, expired ledger footprint). | Not applicable — the failure is at the Soroban/Stellar transaction layer, not a `#[contracterror]` variant. |
| Contract error (host trap) | The invoked contract function panicked via `panic_with_error!`. | Present in the transaction result's `resultXdr`, decodable to `Error(Contract, #<code>)` — this is what this page documents. |
| Application-level validation error | A client-side check (e.g. a webhook consumer validating a payload) rejects data before or after the on-chain call. | Not a contract error at all; this is validation logic outside Soroban, documented per-integration rather than here. |

Regardless of SDK, the contract-error code lives in the transaction result and is the same numeric discriminant the CLI prints. Decode it against the specific contract ID that raised it, then look it up in this page.

## Troubleshooting shortlist

A practical shortlist of failures most likely during common flows — not a claim that these are globally the most frequent, just the ones worth checking first for these operations.

| Symptom | Likely error / cause | Fix |
|---|---|---|
| `pay_invoice` fails during merchant onboarding smoke-testing | `TokenNotAcceptedByMerchant` (41) — merchant hasn't called `set_merchant_accepted_tokens`, or `TokenNotAccepted` (12) — token was never globally whitelisted | Check `is_token_accepted_for_merchant`; have admin run `add_accepted_token` and/or merchant run `set_merchant_accepted_tokens` |
| `create_invoice` fails right after registration | `MerchantNotActive` (32) | Admin must call `set_merchant_status(admin, merchant_id, true)` |
| `pay_invoice` fails with a balance-looking error | `InsufficientBalance` (30) | Fund or approve the payer's token balance |
| Invoice payment rejected without an obvious reason | `InvoiceExpired` (27) | Check `get_invoice`'s `expires_at` against the current ledger time |
| `create_invoice_signed` always fails | `MerchantKeyNotFound` (11) — merchant never called `set_merchant_key` | Register a key first; see [Signed invoices](../security/signatures.md) |
| Admin operation rejected unexpectedly | `NotAuthorized` (1, or contract-specific equivalent) | Confirm you're signing with the actual current admin (`get_admin`), not a stale or assumed address |

## Append-only numbering convention

New error variants must be **appended** with a new, unused number in the enum's range — never inserted with a renumbered existing variant, and never reusing a retired number for a different meaning. This mirrors the `#[repr(u32)]` integer-enum rule documented in [Upgradeability: integer enums](../architecture/upgradeability.md#integer-enums-the-number-is-the-identity):

- A stored or previously-observed error code is effectively part of the protocol's wire format — client code, indexers, and alerting rules match on it.
- Renumbering `TokenNotAccepted` from 12 to some other value would silently break every integrator's existing `if code == 12` check without any compile-time signal.
- Removing a variant and later adding a new one with the same number changes its meaning for anyone who cached the old mapping.

The Rust compiler does **not** enforce this — nothing prevents a future PR from reusing a freed number or reordering `= N` discriminants. It is a required convention for this workspace, not a compiler guarantee, and should be checked in review the same way the [upgrade checklist](../architecture/upgradeability.md#checklist-before-shipping-an-upgrade) checks storage-key stability.

When a domain enum approaches its 50-variant cap (see the note in [Upgradeability](../architecture/upgradeability.md#key-enums-the-variant-name-is-the-identity)), start a new domain enum with a fresh numeric range rather than renumbering the existing one.

## Related pages

- [Upgradeability, WASM hash management, and versioning](../architecture/upgradeability.md) — the append-only convention for `#[repr(u32)]` enums that this page's numbering convention mirrors.
- [Signed invoices](../security/signatures.md) — detail on `MerchantKeyNotFound` and `NonceAlreadyUsed`.
- [Merchant Onboarding Guide](../guides/merchant-onboarding.md) — the errors most likely to appear during onboarding.

← [Back to reference](README.md)
