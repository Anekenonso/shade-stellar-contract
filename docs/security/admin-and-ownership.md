# Admin ownership and two-step admin transfer

This page documents how admin authority is established, the two-step transfer flow, and operational safeguards.

## Initialization

### `initialize`

```rust
fn initialize(env: Env, admin: Address);
```

- **Authorization:** None required (but can only be called once).
- **Effect:** Stores the admin address under `DataKey::Admin`, sets the platform account to the same address, and records `ContractInfo` with the creation timestamp.
- **Protection:** Panics with `AlreadyInitialized` (error code 2) if `DataKey::Admin` already exists.
- **Event:** Emits `InitializedEvent` with the admin address and timestamp.

The first admin is whoever deploys and calls `initialize`. There is no ceremony or multisig requirement — the deployer is trusted.

## Admin retrieval

### `get_admin`

```rust
fn get_admin(env: Env) -> Address;
```

- **Authorization:** None (read-only).
- **Implementation:** `core::get_admin` reads `DataKey::Admin` from persistent storage. Panics with `NotInitialized` if no admin has been set.

## Two-step admin transfer

The transfer uses a propose/accept pattern to prevent handing control to an unusable address (e.g., a typo or contract address that cannot sign transactions).

### Step 1: `propose_admin_transfer`

```rust
fn propose_admin_transfer(env: Env, admin: Address, new_admin: Address);
```

- **Authorization:** `admin.require_auth()` + `assert_admin`.
- **Effect:** Stores `new_admin` under `DataKey::PendingAdmin`.
- **Event:** Emits `AdminTransferProposedEvent` with the current admin, proposed new admin, and timestamp.
- **Superseding:** Calling this again with a different `new_admin` overwrites the previous proposal. There is only one pending proposal at a time.

### Step 2: `accept_admin_transfer`

```rust
fn accept_admin_transfer(env: Env, new_admin: Address);
```

- **Authorization:** `new_admin.require_auth()` — the proposed new admin must sign.
- **Effect:** Verifies `new_admin == PendingAdmin`, then:
  1. Writes `new_admin` to `DataKey::Admin`.
  2. Removes `DataKey::PendingAdmin`.
- **Event:** Emits `AdminTransferAcceptedEvent` with the old admin, new admin, and timestamp.
- **Failure cases:**
  - Panics with `NotAuthorized` if no transfer has been proposed.
  - Panics with `NotAuthorized` if the caller does not match the pending admin.

## Why two-step transfer is unsafe to do in one step

A single-step `transfer_admin(new_admin)` would immediately change the admin. If `new_admin` is:
- A typo → the admin key is permanently lost.
- A contract that cannot sign → no one can call admin functions.
- An address the team doesn't control → protocol is compromised.

The two-step flow guarantees that the new admin can prove control by signing `accept_admin_transfer` before ownership changes.

## What happens to the pending proposal

| Scenario | Behaviour |
|----------|-----------|
| New admin calls `accept_admin_transfer` | Transfer completes. `PendingAdmin` is removed. |
| Current admin proposes a different address | Previous proposal is overwritten. The old candidate can no longer accept. |
| No one calls `accept` | The pending proposal remains indefinitely. No timeout or expiry exists. |

## Admin-only capabilities

All functions gated by `core::assert_admin` (which requires `admin.require_auth()` + admin address match):

| Function | Description |
|----------|-------------|
| `add_accepted_token` | Add a token to the accepted list |
| `add_accepted_tokens` | Batch-add tokens |
| `remove_accepted_token` | Remove a token |
| `set_account_wasm_hash` | Set the WASM hash for sub-account deployment |
| `set_fee` | Set the fee for a token |
| `propose_fee` | Propose a time-locked fee change |
| `execute_fee` | Execute a previously proposed fee change |
| `set_platform_account` | Change the platform fee recipient |
| `set_token_oracle` | Configure an oracle for a token |
| `grant_role` | Grant a role to an address |
| `revoke_role` | Revoke a role from an address |
| `pause` | Pause the contract |
| `unpause` | Unpause the contract |
| `set_merchant_status` | Activate/deactivate a merchant |
| `verify_merchant` | Toggle merchant verification |
| `restrict_merchant_account` | Restrict a merchant account |
| `set_merchant_platform_fee` | Set per-merchant fee override |
| `clear_merchant_platform_fee` | Clear per-merchant fee override |
| `propose_admin_transfer` | Propose a new admin |
| `upgrade` | Upgrade the contract WASM |

## Operational recommendations

### Key management

- Use a multisig or hardware wallet for the admin key. A single EOA key is a single point of failure.
- Document the admin key recovery procedure. If the key is lost and no transfer was proposed, the protocol is permanently locked.
- Consider using a smart contract as the admin address with a governance mechanism.

### Rehearsal

- Always rehearse the admin transfer on testnet before executing on mainnet.
- Verify the new admin address can sign transactions by calling a simple function (e.g., `is_paused`) before initiating the transfer.

### Post-transfer verification

After `accept_admin_transfer` succeeds:
1. Call `get_admin` and verify it returns the new address.
2. Call a function gated by `assert_admin` (e.g., `is_paused`) using the new key.
3. Verify the old admin key can no longer perform admin operations.

## Compromised admin key — blast radius

A compromised admin key grants full control:

| Capability | Impact |
|------------|--------|
| Pause/unpause | Can freeze all protocol operations. |
| Fee changes | Can set fees to 100% or zero. |
| Token acceptlist | Can add malicious tokens or remove legitimate ones. |
| Role grants | Can grant any role to any address. |
| WASM upgrade | Can deploy arbitrary code. |
| Admin transfer | Can propose a new admin (but the real admin could propose a different one). |
| Oracle config | Can manipulate price feeds. |
| Merchant management | Can deactivate or restrict any merchant. |

### Immediate response actions

1. **Pause the contract** using the admin key (if you still have access) or coordinate with multisig signers.
2. **Rotate all roles** — grant admin to a new key and revoke the compromised one.
3. **Review recent transactions** — check for unauthorized fee changes, token list modifications, or WASM upgrades.
4. **Communicate** — notify merchants and users of the compromise and any actions taken.
5. **If the admin key is fully lost** — initiate the two-step transfer from the compromised address if it is still accessible, or accept that the protocol is locked without an upgrade path.

← [Back to security](README.md)
