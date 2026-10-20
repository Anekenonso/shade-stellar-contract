# Merchant Account Contract Reference (`account`)

The `account` crate provides a dedicated, self-contained vault contract for individual merchants. Instead of pooling all merchant revenues into a single monolithic contract, Shade deploys an isolated `account` contract per merchant to maximize security, isolate risk, and enable customizable withdrawal rules.

---

## Architectural Purpose & Benefits

- **Risk Isolation**: By isolating merchant balances in distinct contract instances, a potential issue affecting one merchant vault cannot compromise or drain funds belonging to other merchants.
- **Customizable Governance**: Merchants can configure individualized withdrawal thresholds, multi-signature manager approvals, and automated sweep recipients.
- **Auditability**: Each merchant account maintains its own transaction history and token balance records on-chain.

---

## Contract Public Interface

The `account` contract implements the `MerchantAccountTrait` defined in `contracts/account/src/interface.rs`:

### 1. Initialization & Identity
```rust
fn initialize(env: Env, merchant: Address, manager: Address, merchant_id: u64);
fn get_merchant(env: Env) -> Address;
```
- `initialize`: Called exclusively during contract deployment by the `account_factory` to bind the merchant address, manager/operator address, and merchant ID.
- `get_merchant`: Returns the registered owner `Address` of the account.

### 2. Balance & Token Management
```rust
fn add_token(env: Env, token: Address);
fn has_token(env: Env, token: Address) -> bool;
fn get_balance(env: Env, token: Address) -> i128;
fn get_balances(env: Env) -> Vec<TokenBalance>;
```
- `add_token`: Whitelists an accepted payment token for this merchant vault.
- `get_balance`: Returns the current token balance held by the vault.
- `get_balances`: Returns a list of all tracked token balances (`TokenBalance { token, balance }`).

### 3. Withdrawals & Governance
```rust
fn withdraw_to(env: Env, token: Address, amount: i128, recipient: Address);
fn set_withdrawal_threshold(env: Env, threshold: i128);
fn get_withdrawal_threshold(env: Env) -> i128;
fn approve_withdrawal(env: Env, request_id: u64);
fn get_withdrawal_request(env: Env, request_id: u64) -> WithdrawalRequest;
fn get_withdrawal_analytics(env: Env, token: Address) -> WithdrawalAnalytics;
```
- `withdraw_to`: Initiates a transfer from the vault to `recipient`. Requires `merchant.require_auth()`. If `amount > withdrawal_threshold`, it triggers a pending `WithdrawalRequest` requiring manager approval.
- `approve_withdrawal`: Called by `manager` to approve a high-value withdrawal request.
See [Merchant account withdrawals, thresholds, and approval requests](../concepts/merchant-account-withdrawals.md) for the full flow, state diagram, and restriction effects.

### 4. Verification & Restriction Controls
```rust
fn verify_account(env: Env);
fn is_verified_account(env: Env) -> bool;
fn restrict_account(env: Env, status: bool);
fn is_restricted_account(env: Env) -> bool;
fn refund(env: Env, token: Address, amount: i128, to: Address);
```
- `restrict_account`: Toggles restriction state. When `is_restricted_account` is `true`, all outbound withdrawals (`withdraw_to`) are blocked. Only the designated manager or hub admin can update restriction status.
- `refund`: Dispatches an authorized customer refund back to the buyer address.

---

## Lifecycle: Deployment to Settlement

1. **Upload WASM**: Admin uploads `account.wasm` bytecode to the network and invokes `set_account_wasm_hash` on the `shade` hub contract.
2. **Factory Deployment**: The merchant or hub invokes the factory to deploy a new instance deterministically salted by `merchant_id`.
3. **Association**: The newly deployed address is mapped in the hub contract via `set_merchant_account`.
4. **Receiving Funds**: When customer invoices are settled via `pay_invoice`, net proceeds are routed directly to the merchant's `account` contract address.
5. **Withdrawal**: Merchant invokes `withdraw_to` to transfer funds to cold storage or exchange addresses.

---

## Errors & Integration Handling

| Error Code | Meaning | Integrator Action |
| :--- | :--- | :--- |
| **`AccountRestricted`** | Account has been flagged or paused due to compliance/security. | Contact platform administrators to resolve account status. |
| **`ThresholdExceeded`** | Withdrawal amount exceeds instant single-tx limit. | Query `get_withdrawal_request` and notify account manager for secondary approval. |
| **`InsufficientVaultBalance`** | Vault does not hold enough token balance for requested payout. | Check `get_balance` before submitting withdrawal transactions. |
| **`UnauthorizedCaller`** | Signature provided does not match the merchant or manager address. | Ensure transaction envelope is signed by the registered merchant secret key. |
