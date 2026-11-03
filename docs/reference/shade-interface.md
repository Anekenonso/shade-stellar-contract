# ShadeTrait function reference

Complete API reference for the `shade` contract's public interface — every function in `ShadeTrait` with its signature, parameters, return value, authorization, errors, and emitted events.

## Alphabetical index

| Function | Domain |
|----------|--------|
| [`accept_admin_transfer`](#accept_admin_transfer) | Admin |
| [`add_accepted_token`](#add_accepted_token) | Admin |
| [`add_accepted_tokens`](#add_accepted_tokens) | Admin |
| [`amend_invoice`](#amend_invoice) | Invoice |
| [`calculate_fee`](#calculate_fee) | Fee |
| [`claim_refund`](#claim_refund) | Invoice |
| [`clear_merchant_platform_fee`](#clear_merchant_platform_fee) | Fee |
| [`compute_platform_fee_split`](#compute_platform_fee_split) | Fee |
| [`create_fiat_invoice`](#create_fiat_invoice) | Invoice |
| [`create_invoice`](#create_invoice) | Invoice |
| [`create_invoice_draft`](#create_invoice_draft) | Invoice |
| [`create_invoice_signed`](#create_invoice_signed) | Invoice |
| [`execute_fee`](#execute_fee) | Fee |
| [`finalize_invoice`](#finalize_invoice) | Invoice |
| [`find_merchant_id`](#find_merchant_id) | Merchant |
| [`get_admin`](#get_admin) | Admin |
| [`get_auto_withdrawal_recipient`](#get_auto_withdrawal_recipient) | Merchant |
| [`get_auto_withdrawal_threshold`](#get_auto_withdrawal_threshold) | Merchant |
| [`get_fee`](#get_fee) | Fee |
| [`get_invoice`](#get_invoice) | Invoice |
| [`get_invoices`](#get_invoices) | Invoice |
| [`get_merchant`](#get_merchant) | Merchant |
| [`get_merchant_account`](#get_merchant_account) | Merchant |
| [`get_merchant_accepted_tokens`](#get_merchant_accepted_tokens) | Merchant |
| [`get_merchant_analytics`](#get_merchant_analytics) | Analytics |
| [`get_merchant_analytics_summary`](#get_merchant_analytics_summary) | Analytics |
| [`get_merchant_key`](#get_merchant_key) | Security |
| [`get_merchant_platform_fee`](#get_merchant_platform_fee) | Fee |
| [`get_merchant_volume`](#get_merchant_volume) | Analytics |
| [`get_merchant_webhook`](#get_merchant_webhook) | Merchant |
| [`get_merchants`](#get_merchants) | Merchant |
| [`get_pending_fee`](#get_pending_fee) | Fee |
| [`get_platform_account`](#get_platform_account) | Admin |
| [`get_token_analytics`](#get_token_analytics) | Analytics |
| [`get_token_dominance_metrics`](#get_token_dominance_metrics) | Analytics |
| [`get_token_market_share`](#get_token_market_share) | Analytics |
| [`get_token_oracle`](#get_token_oracle) | Admin |
| [`get_token_volume`](#get_token_volume) | Analytics |
| [`get_top_tokens_by_volume`](#get_top_tokens_by_volume) | Analytics |
| [`get_user_transactions`](#get_user_transactions) | Analytics |
| [`grant_role`](#grant_role) | Access control |
| [`has_role`](#has_role) | Access control |
| [`initialize`](#initialize) | Admin |
| [`is_accepted_token`](#is_accepted_token) | Admin |
| [`is_merchant`](#is_merchant) | Merchant |
| [`is_merchant_active`](#is_merchant_active) | Merchant |
| [`is_merchant_verified`](#is_merchant_verified) | Merchant |
| [`is_paused`](#is_paused) | Admin |
| [`is_token_accepted_for_merchant`](#is_token_accepted_for_merchant) | Merchant |
| [`pay_invoice`](#pay_invoice) | Invoice |
| [`pay_invoice_partial`](#pay_invoice_partial) | Invoice |
| [`pay_invoices_batch`](#pay_invoices_batch) | Invoice |
| [`pause`](#pause) | Admin |
| [`propose_admin_transfer`](#propose_admin_transfer) | Admin |
| [`propose_fee`](#propose_fee) | Fee |
| [`register_merchant`](#register_merchant) | Merchant |
| [`refund_invoice`](#refund_invoice) | Invoice |
| [`refund_invoice_partial`](#refund_invoice_partial) | Invoice |
| [`remove_accepted_token`](#remove_accepted_token) | Admin |
| [`remove_merchant_accepted_token`](#remove_merchant_accepted_token) | Merchant |
| [`restrict_merchant_account`](#restrict_merchant_account) | Access control |
| [`revoke_role`](#revoke_role) | Access control |
| [`search_invoices_paginated`](#search_invoices_paginated) | Search |
| [`search_merchants_paginated`](#search_merchants_paginated) | Search |
| [`set_accepted_tokens`](#set_accepted_tokens) | Admin |
| [`set_account_wasm_hash`](#set_account_wasm_hash) | Admin |
| [`set_auto_withdrawal_recipient`](#set_auto_withdrawal_recipient) | Merchant |
| [`set_auto_withdrawal_threshold`](#set_auto_withdrawal_threshold) | Merchant |
| [`set_fee`](#set_fee) | Fee |
| [`set_merchant_account`](#set_merchant_account) | Merchant |
| [`set_merchant_accepted_tokens`](#set_merchant_accepted_tokens) | Merchant |
| [`set_merchant_key`](#set_merchant_key) | Security |
| [`set_merchant_platform_fee`](#set_merchant_platform_fee) | Fee |
| [`set_merchant_status`](#set_merchant_status) | Merchant |
| [`set_merchant_webhook`](#set_merchant_webhook) | Merchant |
| [`set_platform_account`](#set_platform_account) | Admin |
| [`set_token_oracle`](#set_token_oracle) | Admin |
| [`unpause`](#unpause) | Admin |
| [`upgrade`](#upgrade) | Admin |
| [`validate_payment_payload`](#validate_payment_payload) | Invoice |
| [`void_invoice`](#void_invoice) | Invoice |

## Admin / configuration

### `initialize`

```rust
fn initialize(env: Env, admin: Address);
```

Initialize the contract. Must be called exactly once.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `admin` | `Address` | — | Initial admin address. |

**Returns:** nothing. **Events:** `InitalizedEvent`. **Errors:** `AlreadyInitialized` (2).

### `get_admin`

```rust
fn get_admin(env: Env) -> Address;
```

Returns the current admin address.

**Returns:** `Address`. **Auth:** none. **Errors:** none.

### `add_accepted_token`

```rust
fn add_accepted_token(env: Env, admin: Address, token: Address);
```

Add a single token to the accepted-tokens whitelist.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `admin` | `Address` | Admin | Admin address. |
| `token` | `Address` | — | Token contract address to accept. |

**Events:** `TokenAddedEvent`. **Errors:** `NotAuthorized` (1).

### `add_accepted_tokens`

```rust
fn add_accepted_tokens(env: Env, admin: Address, tokens: Vec<Address>);
```

Add multiple tokens to the whitelist in one call.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `admin` | `Address` | Admin | Admin address. |
| `tokens` | `Vec<Address>` | — | Token addresses to accept. |

**Events:** `TokenAddedEvent` per token. **Errors:** `NotAuthorized` (1).

### `remove_accepted_token`

```rust
fn remove_accepted_token(env: Env, admin: Address, token: Address);
```

Remove a token from the accepted-tokens whitelist.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `admin` | `Address` | Admin | Admin address. |
| `token` | `Address` | — | Token to remove. |

**Events:** `TokenRemovedEvent`. **Errors:** `NotAuthorized` (1).

### `is_accepted_token`

```rust
fn is_accepted_token(env: Env, token: Address) -> bool;
```

Whether the token is in the accepted whitelist.

**Returns:** `bool`. **Auth:** none.

### `set_account_wasm_hash`

```rust
fn set_account_wasm_hash(env: Env, admin: Address, wasm_hash: soroban_sdk::BytesN<32>);
```

Set the WASM hash used to deploy merchant account contracts.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `admin` | `Address` | Admin | Admin address. |
| `wasm_hash` | `BytesN<32>` | — | WASM hash for account contracts. |

**Events:** `AccountWasmHashSetEvent`. **Errors:** `NotAuthorized` (1).

### `set_platform_account`

```rust
fn set_platform_account(env: Env, admin: Address, account: Address);
```

Set the address that receives platform fees.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `admin` | `Address` | Admin | Admin address. |
| `account` | `Address` | — | Platform fee recipient address. |

**Events:** `PlatformAccountSetEvent`. **Errors:** `NotAuthorized` (1).

### `get_platform_account`

```rust
fn get_platform_account(env: Env) -> Address;
```

Returns the platform fee recipient address.

**Returns:** `Address`. **Auth:** none.

### `set_token_oracle`

```rust
fn set_token_oracle(env: Env, admin: Address, token: Address, oracle: OracleConfig);
```

Configure a price oracle for a token.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `admin` | `Address` | Admin | Admin address. |
| `token` | `Address` | — | Token to configure. |
| `oracle` | `OracleConfig` | — | Oracle config (contract, price_decimals, token_decimals). |

**Events:** `TokenOracleSetEvent`. **Errors:** `NotAuthorized` (1).

### `get_token_oracle`

```rust
fn get_token_oracle(env: Env, token: Address) -> OracleConfig;
```

Returns the oracle config for a token.

**Returns:** `OracleConfig`. **Errors:** `OracleNotConfigured` (34).

### `pause` / `unpause` / `is_paused`

```rust
fn pause(env: Env, admin: Address);
fn unpause(env: Env, admin: Address);
fn is_paused(env: Env) -> bool;
```

Emergency pause controls. When paused, most state-mutating functions panic with `ContractPaused`.

**Events:** `ContractPausedEvent` / `ContractUnpausedEvent`. **Errors:** `NotAuthorized` (1), `ContractNotPaused` (10), `ContractPaused` (9).

### `upgrade`

```rust
fn upgrade(env: Env, new_wasm_hash: BytesN<32>);
```

Replace the contract's WASM. Requires admin auth. Takes effect immediately.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `new_wasm_hash` | `BytesN<32>` | Admin | New WASM hash. |

**Events:** `ContractUpgradedEvent`. **Errors:** `NotAuthorized` (1), `WasmHashNotSet` (18).

> **Warning:** This takes effect immediately for anyone with admin auth.

### `propose_admin_transfer` / `accept_admin_transfer`

```rust
fn propose_admin_transfer(env: Env, admin: Address, new_admin: Address);
fn accept_admin_transfer(env: Env, new_admin: Address);
```

Two-step admin handover. Step 1 proposes; step 2 accepts.

**Events:** `AdminTransferProposedEvent`, `AdminTransferAcceptedEvent`. **Errors:** `NotAuthorized` (1).

## Fee management

### `set_fee`

```rust
fn set_fee(env: Env, admin: Address, token: Address, fee: i128);
```

Set the platform fee for a token (in basis points). Applies immediately.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `admin` | `Address` | Admin | Admin address. |
| `token` | `Address` | — | Token address. |
| `fee` | `i128` | — | Fee in basis points (10_000 = 100%). |

**Events:** `FeeSetEvent`. **Errors:** `NotAuthorized` (1).

### `get_fee`

```rust
fn get_fee(env: Env, token: Address) -> i128;
```

Returns the current fee for a token.

**Returns:** `i128`. **Errors:** none (returns 0 if not set).

### `propose_fee` / `execute_fee` / `get_pending_fee`

```rust
fn propose_fee(env: Env, admin: Address, token: Address, fee: i128);
fn execute_fee(env: Env, admin: Address, token: Address);
fn get_pending_fee(env: Env, token: Address) -> PendingFee;
```

Time-locked fee update. `propose_fee` stages the change; `execute_fee` applies it after the time-lock.

**Events:** `FeeProposedEvent`, `FeeSetEvent`. **Errors:** `FeeUpdateTooEarly` (42), `NoPendingFeeUpdate` (43).

### `set_merchant_platform_fee` / `get_merchant_platform_fee` / `clear_merchant_platform_fee`

```rust
fn set_merchant_platform_fee(env: Env, caller: Address, merchant_id: u64, token: Address, fee_bps: i128);
fn get_merchant_platform_fee(env: Env, merchant_id: u64, token: Address) -> Option<i128>;
fn clear_merchant_platform_fee(env: Env, caller: Address, merchant_id: u64, token: Address);
```

Per-merchant fee overrides. Requires admin auth for set/clear.

**Events:** `MerchantPlatformFeeSetEvent`, `PlatformFeeClearedEvent`.

### `calculate_fee`

```rust
fn calculate_fee(env: Env, merchant: Address, token: Address, amount: i128) -> i128;
```

Compute the fee for a given amount. Accounts for volume discounts and per-merchant overrides.

**Returns:** `i128` fee amount.

### `compute_platform_fee_split`

```rust
fn compute_platform_fee_split(env: Env, merchant: Address, token: Address, amount: i128) -> PlatformFeeSplit;
```

Compute the full fee split (gross, platform fee, merchant amount, effective bps).

**Returns:** `PlatformFeeSplit`.

## Merchant management

### `register_merchant`

```rust
fn register_merchant(env: Env, merchant: Address);
```

Register a new merchant. Requires the caller to authorize.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `merchant` | `Address` | Merchant | Merchant's address. |

**Returns:** nothing. **Events:** `MerchantRegisteredEvent`, `MerchantAccountDeployedEvent`. **Errors:** `MerchantAlreadyRegistered` (5).

### `get_merchant`

```rust
fn get_merchant(env: Env, merchant_id: u64) -> Merchant;
```

Returns a merchant by ID.

**Returns:** `Merchant`. **Errors:** `MerchantNotFound` (6).

### `get_merchants`

```rust
fn get_merchants(env: Env, filter: MerchantFilter) -> Vec<Merchant>;
```

List merchants matching the filter.

**Returns:** `Vec<Merchant>`.

### `is_merchant`

```rust
fn is_merchant(env: Env, merchant: Address) -> bool;
```

Whether the address is registered as a merchant.

**Returns:** `bool`.

### `set_merchant_status`

```rust
fn set_merchant_status(env: Env, admin: Address, merchant_id: u64, status: bool);
```

Enable or disable a merchant. Requires admin auth.

**Events:** `MerchantStatusChangedEvent`. **Errors:** `MerchantNotFound` (6).

### `is_merchant_active` / `is_merchant_verified`

```rust
fn is_merchant_active(env: Env, merchant_id: u64) -> bool;
fn is_merchant_verified(env: Env, merchant_id: u64) -> bool;
```

Check merchant status flags.

### `verify_merchant`

```rust
fn verify_merchant(env: Env, admin: Address, merchant_id: u64, status: bool);
```

Set the verified flag on a merchant. Requires admin auth.

**Events:** `MerchantVerifiedEvent`.

### `deploy_merchant_account`

```rust
fn deploy_merchant_account(env: Env, merchant: Address) -> Address;
```

Deploys the merchant's `account` contract with the hub as manager and links it. Requires the merchant's authorization and a set account WASM hash. Each merchant can deploy once.

**Errors:** `ContractPaused` (9), `MerchantNotFound` (6), `WasmHashNotSet` (18), `MerchantNotActive` (32), `MerchantAccountAlreadySet` (52).

**Events:** `MerchantAccountDeployedEvent`.

### `get_merchant_account`

```rust
fn get_merchant_account(env: Env, merchant_id: u64) -> Address;
```

Returns the merchant's linked account. Fails with `MerchantAccountNotSet` (20) if none has been deployed.

### `verify_merchant_account`

```rust
fn verify_merchant_account(env: Env, admin: Address, merchant_id: u64);
```

Marks the merchant's account contract as verified. Admin only.

### `add_account_token`

```rust
fn add_account_token(env: Env, merchant: Address, token: Address);
```

Adds a globally accepted token to the balances the merchant's account tracks. Requires the merchant's authorization.

### `set_account_withdraw_threshold`

```rust
fn set_account_withdraw_threshold(env: Env, caller: Address, merchant_id: u64, threshold: i128);
```

Sets the amount above which the merchant's withdrawals need approval. `0` disables the check. The caller needs the admin or manager role.

### `approve_account_withdrawal`

```rust
fn approve_account_withdrawal(env: Env, caller: Address, merchant_id: u64, request_id: u64);
```

Approves and executes a pending withdrawal on the merchant's account. The caller needs the admin or manager role. Fails while the account is restricted.

### `set_merchant_webhook` / `get_merchant_webhook`

```rust
fn set_merchant_webhook(env: Env, merchant: Address, webhook: String);
fn get_merchant_webhook(env: Env, merchant_id: u64) -> String;
```

Manage the merchant's webhook URL.

**Events:** `MerchantWebhookSetEvent`.

### `set_merchant_accepted_tokens` / `get_merchant_accepted_tokens` / `remove_merchant_accepted_token` / `is_token_accepted_for_merchant`

```rust
fn set_merchant_accepted_tokens(env: Env, merchant: Address, tokens: Vec<Address>);
fn get_merchant_accepted_tokens(env: Env, merchant: Address) -> Vec<Address>;
fn remove_merchant_accepted_token(env: Env, merchant: Address, token: Address);
fn is_token_accepted_for_merchant(env: Env, merchant: Address, token: Address) -> bool;
```

Per-merchant token whitelist management.

**Events:** `MerchantTokensSetEvent`, `MerchantTokenRemovedEvent`.

### `set_merchant_key` / `get_merchant_key`

```rust
fn set_merchant_key(env: Env, merchant: Address, key: BytesN<32>);
fn get_merchant_key(env: Env, merchant: Address) -> BytesN<32>;
```

Register or retrieve the ed25519 public key for signed invoices.

**Events:** `MerchantKeySetEvent`. **Errors:** `MerchantKeyNotFound` (11).

### `restrict_merchant_account`

```rust
fn restrict_merchant_account(env: Env, caller: Address, merchant_address: Address, status: bool);
```

Restrict or unrestrict a merchant account. Requires admin auth.

**Events:** `AccountRestrictedEvent`.

### `find_merchant_id`

```rust
fn find_merchant_id(env: Env, address: Address) -> u64;
```

Look up a merchant ID from an address. Returns 0 if not registered.

**Returns:** `u64`.

### Auto-withdrawal

```rust
fn set_auto_withdrawal_threshold(env: Env, merchant: Address, token: Address, threshold: i128);
fn get_auto_withdrawal_threshold(env: Env, merchant_id: u64, token: Address) -> Option<i128>;
fn set_auto_withdrawal_recipient(env: Env, merchant: Address, recipient: Address);
fn get_auto_withdrawal_recipient(env: Env, merchant_id: u64) -> Option<Address>;
```

Configure auto-withdrawal thresholds and recipients.

**Events:** `AutoWithdrawThresholdEvent`, `AutoWithdrawRecipientEvent`.

## Invoice management

### `create_invoice`

```rust
fn create_invoice(env: Env, merchant: Address, description: String, amount: i128, token: Address, expires_at: Option<u64>) -> u64;
```

Create a payable invoice. Requires merchant auth.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `merchant` | `Address` | Merchant | Merchant address. |
| `description` | `String` | — | Invoice description. |
| `amount` | `i128` | — | Amount in token base units. |
| `token` | `Address` | — | Accepting token. |
| `expires_at` | `Option<u64>` | — | Optional expiration timestamp. |

**Returns:** `u64` invoice ID. **Events:** `InvoiceCreatedEvent`. **Errors:** `InvalidAmount` (7), `TokenNotAccepted` (12), `MerchantNotActive` (32).

### `create_fiat_invoice`

```rust
fn create_fiat_invoice(env: Env, merchant: Address, description: String, fiat_amount: i128, fiat_currency: String, fiat_decimals: u32, token: Address, expires_at: Option<u64>) -> u64;
```

Create an invoice denominated in fiat. Converted to crypto at payment time via oracle.

**Returns:** `u64` invoice ID. **Events:** `InvoiceCreatedEvent`.

### `create_invoice_draft`

```rust
fn create_invoice_draft(env: Env, merchant: Address, description: String, amount: i128, token: Address, expires_at: Option<u64>) -> u64;
```

Create a draft invoice (not yet finalized for payment). Requires merchant auth.

**Returns:** `u64` invoice ID. **Events:** `InvoiceCreatedEvent`.

### `finalize_invoice`

```rust
fn finalize_invoice(env: Env, merchant: Address, invoice_id: u64);
```

Finalize a draft invoice, making it payable. Requires merchant auth.

**Errors:** `InvalidInvoiceStatus` (16).

### `create_invoice_signed`

```rust
fn create_invoice_signed(env: Env, caller: Address, merchant: Address, description: String, amount: i128, token: Address, nonce: BytesN<32>, signature: BytesN<64>) -> u64;
```

Create an invoice from an off-chain signed payload. The signature is verified against the merchant's registered ed25519 key.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `caller` | `Address` | Caller | Address submitting the transaction. |
| `merchant` | `Address` | — | Merchant address. |
| `description` | `String` | — | Invoice description. |
| `amount` | `i128` | — | Amount in token base units. |
| `token` | `Address` | — | Accepting token. |
| `nonce` | `BytesN<32>` | — | Unique 32-byte nonce (single-use). |
| `signature` | `BytesN<64>` | — | Ed25519 signature. |

**Returns:** `u64` invoice ID. **Events:** `InvoiceCreatedEvent`, `NonceInvalidatedEvent`. **Errors:** `MerchantKeyNotFound` (11), `NonceAlreadyUsed` (14).

See [Signed invoices](../security/signatures.md) for the message construction and signing scheme.

### `get_invoice`

```rust
fn get_invoice(env: Env, invoice_id: u64) -> Invoice;
```

Returns an invoice by ID.

**Errors:** `InvoiceNotFound` (8).

### `resolve_invoice_amount`

```rust
fn resolve_invoice_amount(env: Env, invoice_id: u64) -> i128;
```

Resolve the payment amount for an invoice. For fiat-priced invoices, converts via oracle.

**Returns:** `i128`.

### `get_invoices`

```rust
fn get_invoices(env: Env, filter: InvoiceFilter) -> Vec<Invoice>;
```

List invoices matching the filter.

**Returns:** `Vec<Invoice>`.

### `pay_invoice`

```rust
fn pay_invoice(env: Env, payer: Address, invoice_id: u64);
```

Pay an invoice in full. Transfers the exact amount from the payer.

| Parameter | Type | Auth | Description |
|-----------|------|------|-------------|
| `payer` | `Address` | Payer | Payer address. |
| `invoice_id` | `u64` | — | Invoice to pay. |

**Events:** `InvoicePaidEvent`, `PlatformFeeRoutedEvent`. **Errors:** `InvoiceNotFound` (8), `InvalidInvoiceStatus` (16), `InvoiceExpired` (27), `TokenNotAcceptedByMerchant` (41), `InsufficientBalance` (30).

### `pay_invoice_partial`

```rust
fn pay_invoice_partial(env: Env, payer: Address, invoice_id: u64, amount: i128);
```

Make a partial payment toward an invoice.

**Events:** `InvoicePaidEvent`. **Errors:** same as `pay_invoice` plus `InvalidAmount` (7).

### `pay_invoices_batch`

```rust
fn pay_invoices_batch(env: Env, payer: Address, invoice_ids: Vec<u64>);
```

Pay multiple invoices in a single call. Each invoice is paid individually.

**Events:** `InvoicePaidEvent` per invoice. **Errors:** per-invoice errors.

### `refund_invoice`

```rust
fn refund_invoice(env: Env, merchant: Address, invoice_id: u64);
```

Fully refund a paid invoice. Requires merchant auth.

**Events:** `InvoiceRefundedEvent`. **Errors:** `InvalidInvoiceStatus` (16), `RefundPeriodExpired` (17).

### `refund_invoice_partial`

```rust
fn refund_invoice_partial(env: Env, merchant: Address, invoice_id: u64, amount: i128);
```

Partially refund a paid invoice. Requires merchant auth.

**Events:** `InvoicePartiallyRefundedEvent`.

### `claim_refund`

```rust
fn claim_refund(env: Env, buyer: Address, invoice_id: u64);
```

Claim a refund for a refunded invoice. Requires buyer auth.

**Errors:** `InvalidInvoiceStatus` (16).

### `void_invoice`

```rust
fn void_invoice(env: Env, merchant: Address, invoice_id: u64);
```

Cancel (void) a pending invoice. Requires merchant auth.

**Events:** `InvoiceCancelledEvent`. **Errors:** `InvalidInvoiceStatus` (16).

### `amend_invoice`

```rust
fn amend_invoice(env: Env, merchant: Address, invoice_id: u64, new_amount: Option<i128>, new_description: Option<String>);
```

Amend an unfinalized invoice's amount and/or description. Requires merchant auth.

**Events:** `InvoiceAmendedEvent`.

### `validate_payment_payload`

```rust
fn validate_payment_payload(env: Env, payload: PaymentPayload);
```

Validate a payment payload structure without executing the payment.

**Errors:** `InvalidSwapPath` (44), `InvalidSlippage` (45).

## Access control

### `grant_role` / `revoke_role` / `has_role`

```rust
fn grant_role(env: Env, admin: Address, user: Address, role: Role);
fn revoke_role(env: Env, admin: Address, user: Address, role: Role);
fn has_role(env: Env, user: Address, role: Role) -> bool;
```

Manage access-control roles. `grant_role` and `revoke_role` require admin auth.

**Events:** `RoleGrantedEvent`, `RoleRevokedEvent`. **Errors:** `NotAuthorized` (1).

## Analytics

### `get_merchant_volume`

```rust
fn get_merchant_volume(env: Env, merchant: Address, token: Address) -> i128;
```

Cumulative volume for a merchant-token pair.

### `get_merchant_analytics`

```rust
fn get_merchant_analytics(env: Env, merchant: Address, token: Address) -> MerchantAnalytics;
```

Detailed analytics for a merchant-token pair.

### `get_merchant_analytics_summary`

```rust
fn get_merchant_analytics_summary(env: Env, merchant: Address) -> MerchantAnalyticsSummary;
```

Aggregate analytics across all tokens for a merchant.

### `get_token_analytics`

```rust
fn get_token_analytics(env: Env, token: Address) -> TokenAnalytics;
```

Aggregate analytics for a specific token.

### `get_token_volume`

```rust
fn get_token_volume(env: Env, token: Address) -> i128;
```

Total volume for a specific token.

### `get_token_dominance_metrics`

```rust
fn get_token_dominance_metrics(env: Env, tokens: Vec<Address>) -> Vec<(Address, i128)>;
```

Volume-sorted token dominance metrics.

### `get_top_tokens_by_volume`

```rust
fn get_top_tokens_by_volume(env: Env, limit: u32) -> Vec<(Address, i128)>;
```

Top tokens by volume with a limit.

### `get_token_market_share`

```rust
fn get_token_market_share(env: Env, token: Address) -> i128;
```

Market share of a token in basis points (10_000 = 100%).

### `get_user_transactions`

```rust
fn get_user_transactions(env: Env, user: Address) -> Vec<Transaction>;
```

All transactions executed by a specific address.

## Search and filtering

### `search_invoices_paginated`

```rust
fn search_invoices_paginated(env: Env, caller: Address, filter: InvoiceFilter, cursor: u64, page_size: u32) -> InvoicePage;
```

Paginated invoice search with full filter support. Pass `cursor = 0` for the first page.

**Returns:** `InvoicePage`.

### `search_merchants_paginated`

```rust
fn search_merchants_paginated(env: Env, filter: MerchantFilter, cursor: u64, page_size: u32) -> MerchantPage;
```

Paginated merchant search.

**Returns:** `MerchantPage`.

