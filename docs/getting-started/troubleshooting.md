# Developer Troubleshooting Guide

A practical troubleshooting manual for Shade Protocol contributors and integrators. Every entry contains the **exact error text**, root **cause**, and the step-by-step **fix** to unblock development.

---

## Table of Contents

1. [Compilation: `wasm32-unknown-unknown` Target Missing](#1-compilation-wasm32-unknown-unknown-target-missing)
2. [Compilation: Soroban SDK Dependency Version Mismatch](#2-compilation-soroban-sdk-dependency-version-mismatch)
3. [Testing: Snapshot Mismatch in `test_snapshots/`](#3-testing-snapshot-mismatch-in-test_snapshots)
4. [Runtime: `HostError: Error(Auth, InvalidAction)`](#4-runtime-hosterror-errorauth-invalidaction)
5. [Runtime: Contract Error Codes (`Error(Contract, #N)`)](#5-runtime-contract-error-codes-errorcontract-n)
6. [Submission: Simulation Succeeds but On-Chain Submission Fails](#6-submission-simulation-succeeds-but-on-chain-submission-fails)
7. [Storage: Archived State & Expired TTL (`Error(Storage, MissingValue)`)](#7-storage-archived-state--expired-ttl-errorstorage-missingvalue)
8. [CLI: Network or Identity Configuration Error](#8-cli-network-or-identity-configuration-error)
9. [Budget: Resource Limit Exceeded (`HostError: Error(Budget, ExceededLimit)`)](#9-budget-resource-limit-exceeded-hosterror-errorbudget-exceededlimit)
10. [Tokens: Missing Trustline for Stellar Asset Contract (SAC)](#10-tokens-missing-trustline-for-stellar-asset-contract-sac)
11. [Security: Reentrancy Guard Abort (`Error(Contract, #4)`)](#11-security-reentrancy-guard-abort-errorcontract-4)
12. [Lifecycle: Transaction Blocked by Emergency Pause](#12-lifecycle-transaction-blocked-by-emergency-pause)
13. [Still Stuck?](#still-stuck)

---

## 1. Compilation: `wasm32-unknown-unknown` Target Missing

### Exact Error Text
```text
error[E0463]: can't find crate for 'core'
  |
  = note: the 'wasm32-unknown-unknown' target may not be installed
  = help: consider downloading the target with `rustup target add wasm32-unknown-unknown`
```

### Cause
The Rust toolchain requires the `wasm32-unknown-unknown` compilation target to build WebAssembly binaries for Soroban smart contracts.

### Fix
Install the WebAssembly target via `rustup`:
```bash
rustup target add wasm32-unknown-unknown
```
Then verify your build:
```bash
cargo build --target wasm32-unknown-unknown --release
```

---

## 2. Compilation: Soroban SDK Dependency Version Mismatch

### Exact Error Text
```text
error: package `soroban-sdk v22.0.0` cannot be built because it requires rustc 1.81.0 or newer, while the currently active rustc version is 1.79.0
```
*or cross-crate type mismatch:*
```text
error[E0308]: mismatched types
  --> contracts/shade/src/lib.rs:42:15
   |
42 |     env: soroban_sdk::Env
   |          ^^^^^^^^^^^^^^^^ expected `soroban_sdk::Env` from crate `soroban_sdk v21.x`, found `soroban_sdk::Env` from crate `soroban_sdk v22.x`
```

### Cause
The Shade Protocol workspace pins exact versions of `soroban-sdk`. If a contributor updates one crate's `Cargo.toml` independently or uses an incompatible `rustc` compiler version, Cargo resolves conflicting dependencies or fails MSRV (Minimum Supported Rust Version) checks.

### Fix
1. Update Rust to the latest stable toolchain:
   ```bash
   rustup update stable
   rustup default stable
   ```
2. Ensure all workspace crates (`shade`, `account`, `escrow`, `subscription`, `ticketing`, etc.) reference the identical `soroban-sdk` version defined in the workspace root `Cargo.toml`.
3. Clean the build cache and re-lock:
   ```bash
   cargo clean
   cargo update
   cargo check --workspace
   ```

---

## 3. Testing: Snapshot Mismatch in `test_snapshots/`

### Exact Error Text
```text
test test_invoice_creation ... FAILED

failures:
---- test_invoice_creation stdout ----
Snapshot assertion failed
- Old Snapshot
+ New Snapshot
@@ -12,4 +12,4 @@
-  amount: 10000000
+  amount: 20000000
```

### Cause
Shade uses snapshot testing (via `insta`) to detect accidental changes in serialized contract state, events, or interface outputs. If you intentionally changed an event schema or contract behavior, the stored snapshot in `test_snapshots/` no longer matches the test run.

### Fix
1. Inspect the diff to confirm that the change in behavior is intentional and correct.
2. If intentional, accept the new snapshot using `cargo-insta`:
   ```bash
   cargo install cargo-insta
   cargo insta review
   # Or accept all pending snapshots automatically:
   cargo insta accept
   ```
3. Re-run tests to ensure the suite passes:
   ```bash
   cargo test --workspace
   ```
4. Commit the updated snapshot files located in `contracts/<contract>/src/test_snapshots/`.

---

## 4. Runtime: `HostError: Error(Auth, InvalidAction)`

### Exact Error Text
```text
HostError: Error(Auth, InvalidAction)
Event log:
  [diagnostic] contract:CB...: auth failed for address GB...: function requires authorization from GB...
```

### Cause
The invoking transaction failed a `require_auth()` check. Common causes:
* The transaction envelope does not include the signature of the address specified in the arguments.
* An integrator attempted to sign `pay_invoice` using a different address than the payer.
* In multi-operation batches (`pay_invoices_batch`), `require_auth` was incorrectly called multiple times for the same address within the same call frame.

### Fix
* **In Rust Tests**: Explicitly mock authorization on the client mock before the function call:
  ```rust
  client.mock_all_auths().pay_invoice(&payer, &invoice_id);
  ```
* **In TypeScript/JavaScript SDK**: Ensure the transaction includes an auth entry matching the signer:
  ```typescript
  const tx = await contract.pay_invoice({
    payer: payerPublicKey,
    invoice_id: 1n,
  });
  // Sign with payer keypair before submitting
  await tx.sign(payerKeypair);
  ```

---

## 5. Runtime: Contract Error Codes (`Error(Contract, #N)`)

### Exact Error Text
```text
HostError: Error(Contract, #1)
```
*or*
```text
HostError: Error(Contract, #7)
```

### Cause
The contract aborted with a custom `#[contracterror]`. In Soroban, custom contract errors are reported to the host as integer codes.

### Fix
Map the numeric code to the contract definition using the [Errors Reference](../reference/errors.md):
* **Code `#1` (`NotAuthorized`)**: The caller lacks the required role or administrative permission. Confirm the caller has been granted the required role via `grant_role` or matches `get_admin`.
* **Code `#2` (`AlreadyInitialized`)**: You called `initialize` on an already active contract instance.
* **Code `#5` (`MerchantAlreadyRegistered`)**: The merchant address is already registered on the hub. Use `find_merchant_id` to query existing records.
* **Code `#6` (`MerchantNotFound`)**: The specified merchant ID or address is not registered. Register the merchant first using `register_merchant`.
* **Code `#7` (`InvalidAmount`)**: The amount passed is zero, negative, exceeds the remaining invoice balance, or is smaller than the calculated platform fee. Pass an amount satisfying: `0 < amount <= (invoice.amount - invoice.amount_paid)`.
* **Code `#12` (`InvoiceExpired`)**: The ledger timestamp has exceeded the invoice's `expires_at` timestamp.

---

## 6. Submission: Simulation Succeeds but On-Chain Submission Fails

### Exact Error Text
```text
error: transaction failed
Result code: txBAD_SEQ
```
*or*
```text
error: transaction failed
Result code: txINTERNAL_ERROR
HostError: Error(Contract, #12)  # InvoiceExpired
```

### Cause
Soroban transactions must be simulated via RPC to calculate the footprint (storage keys read/written) and resource budget before being signed and submitted.
* `txBAD_SEQ`: The source account's sequence number changed between simulation and submission (common in concurrent pipelines).
* `InvoiceExpired` or state drift: The ledger timestamp or state progressed during the delay between simulation and submission, invalidating the simulation result.

### Fix
1. **Refresh sequence number**: Always query the latest account sequence number immediately before signing:
   ```bash
   stellar keys fund <ACCOUNT> --network testnet
   ```
2. **Buffer timestamps**: For time-sensitive transactions (such as fiat-priced invoices or expiring milestone escrows), ensure the transaction validity window (`min_ledger` / `max_ledger` or `valid_until`) accounts for network latency (typically 10–30 seconds buffer).
3. **Re-simulate**: If simulation succeeded more than 15 seconds prior, re-simulate against the latest ledger state.

---

## 7. Storage: Archived State & Expired TTL (`Error(Storage, MissingValue)`)

### Exact Error Text
```text
HostError: Error(Storage, MissingValue)
Event log:
  [diagnostic] key DataKey::Invoice(104) not found or entry is archived
```

### Cause
Soroban uses a State Archival model. Storage entries have a Time-To-Live (TTL). If an entry's TTL is not bumped before it lapses, the entry is archived and can no longer be read or written by contracts until restored.

### Fix
1. **Prolong TTL (Live Entry)**: If the entry is still active, extend its rent TTL from the contract or CLI:
   ```bash
   stellar contract extend-ttl \
     --id <CONTRACT_ID> \
     --durability persistent \
     --ledgers-to-extend 100000 \
     --network testnet
   ```
2. **Restore Entry (Archived Entry)**: If the entry has already been archived:
   ```bash
   stellar contract restore \
     --id <CONTRACT_ID> \
     --durability persistent \
     --network testnet
   ```
3. In code, Shade automatically calls `extend_persistent_ttl` during critical read/write paths, but long-dormant merchant accounts or old invoices should be monitored.

---

## 8. CLI: Network or Identity Configuration Error

### Exact Error Text
```text
error: network 'testnet' not found. Available networks: none
```
*or*
```text
error: identity 'alice' not found
```

### Cause
The local Stellar CLI configuration (`~/.config/stellar/`) has not been populated with the default RPC endpoints or key identities.

### Fix
1. Add the Stellar Testnet configuration:
   ```bash
   stellar network add \
     --global testnet \
     --rpc-url https://soroban-testnet.stellar.org:443 \
     --network-passphrase "Test SDF Network ; September 2015"
   ```
2. Generate or import a developer identity:
   ```bash
   stellar keys generate --global alice --network testnet
   stellar keys fund alice --network testnet
   ```
3. Verify the balance and network connectivity:
   ```bash
   stellar keys address alice
   ```

---

## 9. Budget: Resource Limit Exceeded (`HostError: Error(Budget, ExceededLimit)`)

### Exact Error Text
```text
HostError: Error(Budget, ExceededLimit)
Event log:
  [diagnostic] CPU instructions limit exceeded: 100000000 > 100000000
```

### Cause
The transaction exceeded Soroban's transaction instruction ceiling or memory byte limits. This can occur in Shade when:
* Attempting an excessively large batch payment (`pay_invoices_batch`) containing dozens of invoices.
* Iterating over an unbounded vector stored in a single persistent storage key.

### Fix
1. **Reduce Batch Size**: Chunk large payments into smaller batches (recommended: $\le 10$ invoices per call).
2. **Profile Resource Consumption**: Run local simulation with diagnostic logging:
   ```bash
   stellar contract invoke \
     --id <CONTRACT_ID> \
     --source alice \
     --network testnet \
     --cost \
     -- <FUNCTION> <ARGS>
   ```
3. Optimize contract loops and memory allocations.

---

## 10. Tokens: Missing Trustline for Stellar Asset Contract (SAC)

### Exact Error Text
```text
HostError: Error(WasmVm, Trap)
Event log:
  [diagnostic] contract:CD...: token transfer failed: AccountMissingTrustline
```

### Cause
On Stellar, non-native classic assets (like USDC) wrapped through the Stellar Asset Contract (SAC) require destination accounts to hold an established trustline for the asset before they can receive tokens. If a merchant's payout address does not hold a trustline for the settlement asset, the contract transfer aborts.

### Fix
Establish a trustline from the recipient account before attempting settlement:
```bash
stellar xdr change-trust \
  --source merchant_payout_account \
  --asset "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5" \
  --network testnet
```
*For native XLM, no trustline is required.*

---

## 11. Security: Reentrancy Guard Abort (`Error(Contract, #4)`)

### Exact Error Text
```text
HostError: Error(Contract, #4)
Event log:
  [diagnostic] reentrancy guard: active call detected in component
```

### Cause
Shade guards all fund-moving entrypoints (`pay_invoice`, `withdraw`, `refund_invoice`) with an on-chain reentrancy guard (`reentrancy::enter(&env)`). If an external contract call attempts to re-enter a protected Shade function before the original execution frame has returned, the call immediately panics with code `#4`.

### Fix
* Ensure external contract callbacks or token hooks do not attempt to invoke Shade functions re-entrantly.
* Check transaction composition: do not bundle multiple interdependent Shade invocations inside an intermediate router contract without allowing each frame to exit cleanly.

---

## 12. Lifecycle: Transaction Blocked by Emergency Pause

### Exact Error Text
```text
HostError: Error(Contract, #1)  # Or contract-level PausedError
Event log:
  [diagnostic] pausable: contract is currently paused
```

### Cause
The Shade protocol hub or individual contract module has been placed in an emergency paused state by an administrator or emergency pauser using `pause()`.

### Fix
1. Query the contract pause state:
   ```bash
   stellar contract invoke \
     --id <CONTRACT_ID> \
     --source alice \
     --network testnet \
     -- is_paused
   ```
2. If the contract is legitimately paused for maintenance or incident response, wait for the admin to complete the resolution and call `unpause()`.
3. Check the project status announcements or Discord/GitHub alerts for status updates.

---

## Still Stuck?

If your error is not resolved by the steps above:
* Search the [GitHub Issues](https://github.com/ShadeProtocol/shade-stellar-contract/issues) for similar reports.
* Review the [Errors Reference](../reference/errors.md) for full descriptions of all contract error codes.
* Open a new issue with your exact `HostError`, contract ID, network, and CLI command output.
