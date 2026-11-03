# Reentrancy protection

This page documents the reentrancy guard, where it is applied, and the rules a contributor must follow when adding new externally-calling functions.

## Reentrancy risk in Soroban

> **Note:** The Soroban host rejects a call back into a contract that is already on the call stack. The guard below is defense in depth, not the only protection.

In Soroban, a contract can re-enter itself or other contracts during execution. Re-entrant calls can occur through:

- **SEP-41 token transfers:** `transfer`, `transfer_from`, and `approve` callbacks to token contracts can re-enter the Shade contract.
- **Factory-deployed contracts:** Newly deployed account contracts may invoke Shade functions during their constructor.
- **Cross-contract calls:** Any `invoke_contract` or `call` to an external contract that calls back into Shade.

Reentrancy is dangerous when a function updates state *after* an external call. An attacker could re-enter during the external call and observe or manipulate stale state.

## Guard implementation

The guard lives in `components/reentrancy.rs` and uses a persistent storage flag:

```rust
pub fn enter(env: &Env) {
    if env.storage().persistent().has(&DataKey::ReentrancyStatus) {
        panic_with_error!(env, ContractError::Reentrancy);
    }
    env.storage()
        .persistent()
        .set(&DataKey::ReentrancyStatus, &true);
}

pub fn exit(env: &Env) {
    env.storage()
        .persistent()
        .remove(&DataKey::ReentrancyStatus);
}
```

| Function | Behaviour |
|----------|-----------|
| `enter` | Panics with `Reentrancy` (error code 4) if the flag is already set. Otherwise sets it to `true`. |
| `exit` | Removes the flag from storage. |

The guard is a simple mutex: only one execution path may hold it at a time.

## Currently guarded functions

Every function that makes external calls (token transfers, cross-contract invocations) wraps its body in `enter`/`exit`:

| Component | Functions guarded |
|-----------|-------------------|
| `admin` | `add_accepted_token`, `add_accepted_tokens`, `remove_accepted_token`, `set_fee`, `set_platform_account`, `set_token_oracle`, `propose_fee`, `execute_fee` |
| `platform_fee` | `set_merchant_platform_fee`, `clear_merchant_platform_fee` |

Functions that only read state or write state without external calls (e.g., `get_invoice`, `is_paused`) do not use the guard.

## Contributor rules

When adding a new function that makes external calls, follow this checklist:

- [ ] **Guard the function.** Call `reentrancy::enter(env)` before any external call.
- [ ] **Always pair enter with exit.** Call `reentrancy::exit(env)` in a finalisation path that runs on both success and the happy path.
- [ ] **Prefer checks-effects-interactions ordering.** Perform all validation and state updates before the external call. The reentrancy guard is a safety net, not a substitute for correct ordering.
- [ ] **Never skip exit on error paths.** If the function can fail after `enter`, ensure `exit` is still called (or rely on Soroban's transaction rollback — see below).
- [ ] **Test with a re-entrant contract.** Write a test that deploys a malicious token or callback contract that attempts re-entry. Verify the `Reentrancy` error is raised.

## Guard state after a panic

If a function panics (via `panic_with_error!` or any other mechanism) after calling `enter`, the `exit` call is never reached. However, **Soroban rolls back all storage changes on panic**, so the `ReentrancyStatus` flag is also rolled back. This means:

- A panicking function does NOT leave the guard set.
- Subsequent transactions can call `enter` successfully.
- There is no stuck-guard scenario.

This is safe because Soroban transactions are atomic: either all storage changes commit, or none do.

## Transaction rollback guarantee

Soroban transactions are atomic. If any instruction panics:
1. All storage changes within the transaction are reverted.
2. The `ReentrancyStatus` flag set by `enter` is reverted.
3. The contract state is exactly as it was before the transaction started.

This means the reentrancy guard cannot be left in a stuck state by a failed transaction.

## Testing guidance

The reentrancy test suite (`tests/test_reentrancy.rs`) provides the model for testing newly guarded functions:

1. **Happy path:** Call the guarded function normally and verify it succeeds.
2. **Re-entry attempt:** Deploy a contract that, during a callback, attempts to call the same guarded function again. Verify it panics with `Reentrancy`.
3. **Exit verification:** After a successful call, verify `ReentrancyStatus` is not set (the guard was properly released).

## Error code

| Error | Code | Meaning |
|-------|------|---------|
| `Reentrancy` | 4 | The function was re-entered while the guard was held. |

← [Back to security](README.md)
