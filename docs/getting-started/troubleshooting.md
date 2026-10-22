# Developer troubleshooting guide

Symptom, cause, and fix for the build, test, authorization, simulation, and network-setup failures that contributors and integrators hit most often. Search this page for the exact error text you see. For conceptual questions rather than failures, read the [FAQ](faq.md).

Each entry gives the **symptom** (the error text to search for), the **cause**, and the **fix**.

> **Note:** Compiler, Cargo, and Soroban host errors (`E0463`, `HostError: Error(...)`) are stable across releases. Stellar CLI and RPC message wording changes between releases, so the CLI entries quote the wording you are likely to see and describe the stable signal to look for.

## Quick index

| Area | Entry |
|---|---|
| Build | [WASM target missing](#1-wasm-target-missing) · [rustc too old](#2-rustc-too-old-for-the-sdk) · [SDK version mismatch](#3-soroban-sdk-version-mismatch) · [Stale lockfile](#4-stale-or-conflicting-cargolock) · [Linker errors](#5-linker-errors) |
| Tests | [Snapshot files in git](#6-test_snapshots-files-appear-in-git-status-or-a-diff) · [Zero tests run](#7-a-test-module-runs-zero-tests) |
| Authorization | [`Error(Auth, InvalidAction)`](#8-hosterror-errorauth-invalidaction) |
| Contract errors | [`Error(Contract, #N)`](#9-hosterror-errorcontract-n) |
| Networks | [Simulation passes, submission fails](#10-simulation-succeeds-but-submission-fails) · [Archived entries](#11-archived-entries-and-expired-ttl) · [CLI identity or network](#12-cli-identity-or-network-not-configured) |

## Build

### 1. WASM target missing

**Symptom**

```text
error[E0463]: can't find crate for `core`
  = note: the `wasm32-unknown-unknown` target may not be installed
```

**Cause.** The WebAssembly target is not installed for the *active* toolchain. Targets are per-toolchain, so switching toolchains (for example `rustup default 1.84.0`) loses the target. The same error appears for `wasm32v1-none` when you build with `stellar contract build`, which [`contracts/shade/Makefile`](../../contracts/shade/Makefile) uses and which writes to `target/wasm32v1-none/release/`.

**Fix**

```bash
rustup target add wasm32-unknown-unknown
rustup show
```

If the failing target is `wasm32v1-none`, add that one instead:

```bash
rustup target add wasm32v1-none
```

You do not need the WASM target to run `cargo test` — tests build natively. See [Prerequisites § Add the WASM target](prerequisites.md#add-the-wasm-target).

### 2. rustc too old for the SDK

**Symptom**

```text
error: package `soroban-sdk v23.5.3` cannot be built because it requires rustc 1.84.0 or newer, while the currently active rustc version is 1.75.0
```

**Cause.** The pinned `soroban-sdk` and its `soroban-env-*` and `stellar-xdr` dependencies declare a minimum Rust version of 1.84.0. The failure comes from Cargo's resolver, not from Shade's code, so the message names a dependency.

**Fix**

```bash
rustup update stable
rustup default stable
rustc --version
```

### 3. Soroban SDK version mismatch

**Symptom** — one of:

```text
error[E0308]: mismatched types
  = note: there are multiple different versions of crate `soroban_sdk` in the dependency graph
```

```text
HostError: Error(WasmVm, InvalidAction)
```

The second appears when `stellar contract deploy` or an invocation rejects a WASM that was compiled against a different protocol line than the network or CLI expects.

**Cause.** Two copies of `soroban-sdk` ended up in the graph, usually because a crate declared its own version instead of inheriting the workspace's, or the CLI major version does not match the SDK major version. This workspace pins `soroban-sdk = "23.4.0"` in the root [`Cargo.toml`](../../Cargo.toml), resolves to 23.5.3 in `Cargo.lock`, and expects Stellar CLI 23.x.

**Fix**

1. Find duplicate SDK versions:

   ```bash
   cargo tree -d -p soroban-sdk
   ```

2. In the offending crate's `Cargo.toml`, inherit the workspace version rather than naming one:

   ```toml
   [dependencies]
   soroban-sdk = { workspace = true }
   ```

3. Check the CLI line with `stellar --version`; it should start with `23.`.
4. If you built with a newer Rust toolchain and the network rejects the WASM, rebuild it with `stellar contract build`, which selects the WASM target the CLI expects.

Bump the SDK in one place only — the root `Cargo.toml`. See [Prerequisites § Version compatibility](prerequisites.md#version-compatibility-with-soroban-sdk).

### 4. Stale or conflicting `Cargo.lock`

**Symptom**

```text
error: failed to select a version for `ed25519-dalek`
```

or a build that fails only for you after a merge, or conflict markers (`<<<<<<<`) in `Cargo.lock`.

**Cause.** `soroban-env-host` declares `ed25519-dalek = ">=2.0.0"` but only compiles against 2.x; a fresh resolve picks 3.0.0. The committed `Cargo.lock` is what prevents that.

**Fix.** Never delete the lockfile. Restore the committed one and build with `--locked`:

```bash
git checkout -- Cargo.lock
cargo build --locked
```

After a merge conflict, take the lockfile from `main` and let Cargo re-add only your new dependencies:

```bash
git checkout main -- Cargo.lock
cargo build
```

### 5. Linker errors

**Symptom**

```text
error: linker `cc` not found
error: linking with `cc` failed: exit status: 1
```

**Cause.** Native builds and `cargo test` link against host libraries, so a C toolchain is required even though the deployed artifact is WASM.

**Fix**

```bash
# Debian / Ubuntu / WSL
sudo apt-get install -y build-essential pkg-config libssl-dev

# macOS
xcode-select --install
```

## Tests

### 6. `test_snapshots/` files appear in `git status` or a diff

**Symptom.** A reviewer asks you to remove `contracts/shade/test_snapshots/...json` from your PR, `pre-commit` fails `check-added-large-files`, or snapshot JSON shows up in `git diff` after you ran `cargo test`.

**Cause.** The SDK writes a snapshot JSON file for every `Env` dropped at the end of a test, under `<crate>/test_snapshots/`. [`.gitignore`](../../.gitignore) ignores the directory, so snapshots only reach git through `git add -f` or a force-add by a tool. They are regenerated on every run, so a changed snapshot after an SDK upgrade or a contract edit is expected and is **not** a failure.

**Fix.** Leave the directory alone and unstage anything from it:

```bash
git rm -r --cached contracts/shade/test_snapshots
git status --short | grep test_snapshots
```

If a test passes locally but you suspect a behavior change, compare snapshots before and after rather than committing them:

```bash
cargo test -p shade
cp -r contracts/shade/test_snapshots "$TMPDIR/snapshots-before"
# make your change, then:
cargo test -p shade
diff -r "$TMPDIR/snapshots-before" contracts/shade/test_snapshots
```

The PR template has a "No `test_snapshots/` file is staged in this PR" checkbox for this reason. See [Running tests § Test snapshots](running-tests.md#test-snapshots).

### 7. A test module runs zero tests

**Symptom**

```text
running 0 tests
```

for a crate or filter that should match your new tests, or your new test never appears in `cargo test -p shade -- --list`.

**Cause.** The file was never declared. In-crate test files under `src/tests/` must be registered with a `pub mod test_<subject>;` line in that crate's `src/tests/mod.rs`; an unregistered file compiles as nothing. A related trap: [`contracts/escrow/src/test.rs`](../../contracts/escrow/src/test.rs) and `integration_test.rs` are not declared in `escrow`'s `lib.rs`, so they never run.

**Fix.** Add the module declaration, and make sure the file starts with `#![cfg(test)]`:

```rust
// contracts/shade/src/tests/mod.rs
pub mod test_my_feature;
```

For the `escrow` crate, put tests in `contracts/escrow/tests/`, where each file is its own binary and needs no registration. See [Running tests § Writing a new test module](running-tests.md#writing-a-new-test-module).

## Authorization

### 8. `HostError: Error(Auth, InvalidAction)`

**Symptom**

```text
HostError: Error(Auth, InvalidAction)
```

often followed in the diagnostic log by:

```text
[recording authorization only] encountered authorization not tied to the root contract invocation for an address. Use `require_auth()` in the top invocation or enable non-root authorization.
```

**Cause.** A `require_auth` check in the contract was not satisfied by the authorization the caller provided. Every state-changing Shade entry point calls `require_auth()` on the acting address — `payer` in `pay_invoice`, `merchant` in `create_invoice`, `admin` in `set_fee` — so the call fails when:

- **In a test:** `env.mock_all_auths()` is missing, or the `MockAuth` passed to `env.mock_auths(...)` names a different address, `fn_name`, or `args` than the call actually makes.
- **In a test, nested call:** the authorization is required deep in a call tree rather than at the root. Auto-withdrawal sweeps and other cross-contract paths need `env.mock_all_auths_allowing_non_root_auth()` — see the setup in [`test_auto_withdrawal.rs`](../../contracts/shade/src/tests/test_auto_withdrawal.rs).
- **On a network:** the transaction was submitted without the signature for the address the contract checks — for example the `payer` argument names one account while a different account signed.
- **Wrong argument:** the address passed as an argument is not the address that will sign. The contract authorizes whatever address it is given.

**Fix**

- In tests where authorization is not the subject, call `env.mock_all_auths()` before the first contract call. In tests where it *is* the subject, use `mock_auths` with the exact address, function name, and arguments.
- For nested calls in tests, switch to `env.mock_all_auths_allowing_non_root_auth()`.
- On a network, make sure the signing account matches the address argument, and sign the *simulated* transaction so its authorization entries are included. See [Accepting payments § Authorization](../guides/accepting-payments.md#authorization).

> **Security:** A negative authorization test written under `mock_all_auths()` proves nothing about access control. When the assertion is "this caller must be rejected," use `mock_auths` or no mocking — see [Running tests § `mock_all_auths` versus explicit auth](running-tests.md#mock_all_auths-versus-explicit-auth).

## Contract errors

### 9. `HostError: Error(Contract, #N)`

**Symptom**

```text
HostError: Error(Contract, #2)
```

or a generated-client `try_*` call returning `Err(Ok(...))` with a numeric code.

**Cause.** The contract panicked deliberately with one of its `#[contracterror]` variants. `#N` is the variant's discriminant. The number alone is ambiguous: codes are scoped to the contract that raised them, so `#1` is `AlreadyInitialized` in `account` but `NotAuthorized` in `shade`.

**Fix.** Identify the contract from the contract ID in the failed transaction, then look the code up in the [Errors reference](../reference/errors.md). The codes you will hit most often on the `shade` contract:

| Code | Variant | Usual cause |
|---:|---|---|
| 1 | `NotAuthorized` | Caller is not the admin or lacks the required role, or a second address tried to pay an invoice already claimed by another payer. |
| 2 | `AlreadyInitialized` | `initialize` was called twice on the same instance. |
| 3 | `NotInitialized` | A call ran before `initialize`. |
| 9 | `ContractPaused` | The admin paused the contract; check `is_paused`. |
| 12 | `TokenNotAccepted` | The token is not on the global whitelist — the admin must call `add_accepted_token`. |
| 16 | `InvalidInvoiceStatus` | The invoice is already paid, cancelled, or refunded. |
| 27 | `InvoiceExpired` | Ledger time is at or past `expires_at`. |
| 34 | `OracleNotConfigured` | A fiat invoice's token has no `set_token_oracle` entry. |
| 35 | `OraclePriceUnavailable` | The oracle returned a zero or negative price. |
| 41 | `TokenNotAcceptedByMerchant` | The merchant narrowed its accepted tokens and this one is excluded. |

In tests, prefer the typed form over matching the number, so the test survives renumbering:

```rust
let expected = soroban_sdk::Error::from_contract_error(ContractError::NotAuthorized as u32);
let result = client.try_add_accepted_token(&non_admin, &token);
assert!(matches!(result, Err(Ok(err)) if err == expected));
```

## Networks and the CLI

### 10. Simulation succeeds but submission fails

**Symptom.** `stellar contract invoke` or an RPC `simulateTransaction` returns a result, but the signed transaction then fails with an error such as:

```text
HostError: Error(Contract, #16)
HostError: Error(Budget, ExceededLimit)
txBadSeq
```

**Cause.** A simulation is a dry run against the ledger state at that moment; submission executes against whatever state exists when the transaction is included. The common differences:

- **State changed in between.** Another transaction paid or cancelled the invoice (`Error(Contract, #16)` `InvalidInvoiceStatus`), or the ledger timestamp crossed `expires_at` (`#27` `InvoiceExpired`).
- **Fiat re-quote.** For a fiat-priced invoice the token amount is re-derived from the oracle at payment time, so a price move between simulation and inclusion changes the amount the payer's balance must cover.
- **Resource limits.** Resources and fees are estimated from the simulation. A transaction that sits near a limit can fail with `Error(Budget, ExceededLimit)` when state differs slightly at inclusion.
- **Authorization shape.** Simulation in recording mode accepts any authorization; submission checks the real signed entries (see [entry 8](#8-hosterror-errorauth-invalidaction)).
- **Sequence number.** `txBadSeq` means the source account's sequence number moved — typically two transactions built in parallel from the same account.

**Fix.** Re-simulate immediately before signing, and call `resolve_invoice_amount` right before presenting a fiat amount. Rebuild the transaction with a fresh sequence number and the fresh resource footprint, add resource-fee headroom if you are near a limit, and handle the failed-submission path by re-reading the invoice with `get_invoice` rather than assuming the simulated outcome. See [Accepting payments § Fiat-priced invoices](../guides/accepting-payments.md#fiat-priced-invoices).

### 11. Archived entries and expired TTL

**Symptom.** A read or invoke on an old record fails in simulation, and the simulation response carries a `restorePreamble` instead of a result. The diagnostic mentions an *archived* ledger entry, and depending on the CLI version you may see a prompt or message about restoring the entry.

**Cause.** Every entry the `shade` contract writes goes into persistent storage, and the contract never calls `extend_ttl`. Soroban archives a persistent entry once its TTL runs out, and an archived entry cannot be read until it is restored. Records that are rarely touched — an old invoice, a dormant merchant — are the most exposed. See [Storage model § TTL and rent](../architecture/storage-model.md#ttl-and-rent).

**Fix.** Restore the entry, then retry. With the CLI, restore and (optionally) extend a specific key:

```bash
stellar contract restore --id <contract-id> --key <storage-key> --durability persistent --network testnet --source-account default
stellar contract extend  --id <contract-id> --key <storage-key> --durability persistent --ledgers-to-extend 535680 --network testnet --source-account default
```

Replace `<contract-id>`, `<storage-key>`, the network, and the identity with your own values. From an SDK, apply the simulation's `restorePreamble` as a restore transaction first, then resubmit the original. A deployment that relies on long-term queryability should run a keeper that extends TTLs externally; the contract provides no mechanism to do it.

### 12. CLI identity or network not configured

**Symptom** — wording varies by CLI release, but the message names a missing identity or network:

```text
error: identity `default` not found
error: network `testnet` not found
error: account not found
```

**Cause.** The [`Makefile`](../../Makefile) and the runbooks pass `--source-account default` and `--network testnet`. Neither exists on a fresh machine: the identity has to be generated, and a local network has to be added by name. `account not found` (or a `txNoAccount` result) means the identity exists locally but the account was never funded on that network.

**Fix**

```bash
# See what exists
stellar keys ls
stellar network ls

# Create and fund the identity the Makefile expects
stellar keys generate default --network testnet --fund

# Add a local network (after starting a local node) if it is not listed
stellar network add local \
  --rpc-url http://localhost:8000/rpc \
  --network-passphrase "Standalone Network ; February 2017"
```

Make sure `--network` matches the network the contract was deployed to; a contract ID from testnet does not exist on a local network. See [Deployment Runbook](../operations/deployment.md) and the `NETWORK` variable in the [`Makefile`](../../Makefile).

## Still stuck?

1. **Search existing issues.** Paste the exact error text into the [issue tracker](https://github.com/ShadeProtocol/shade-stellar-contract/issues) search, including closed issues — most build and test failures have been reported.
2. **Check the references.** The [Errors reference](../reference/errors.md) covers every contract error code, and [Prerequisites § Troubleshooting](prerequisites.md#troubleshooting) covers toolchain installation.
3. **Open an issue.** Use the bug-report template and include: the exact command, the full error output, `rustc --version`, `stellar --version`, the resolved `soroban-sdk` version from `Cargo.lock`, the network, and — for on-chain failures — the transaction hash and contract ID.
4. **Ask in a pull request.** If you are mid-change, open a draft PR and ask the question in a comment so the context is attached. See [CONTRIBUTING.md](../../CONTRIBUTING.md).

The repository does not currently list a chat server or forum; the issue tracker is the supported channel. If your deployment has one, link it from your team's runbook.

← [Back to Getting Started](README.md)
