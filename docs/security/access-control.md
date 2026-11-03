# Access control and role model

This page documents the complete authorization model so contributors and auditors can reason about who can do what.

## Role enum

The contract defines three roles in `Role` (`types.rs`):

| Variant | Description |
|---------|-------------|
| `Admin` | Full protocol authority. Implicitly passes every role check. |
| `Manager` | Mid-level administrative capability (reserved for future use). |
| `Operator` | Operational capability (reserved for future use). |

## Admin supremacy

The admin address is stored under `DataKey::Admin` and set once during `initialize`. The `has_role` check (`components/access_control.rs`) treats the admin as implicitly holding every role:

```rust
pub fn has_role(env: &Env, user: &Address, role: Role) -> bool {
    let admin = core::get_admin(env);
    if *user == admin {
        return true;
    }
    // ... storage lookup
}
```

This means:
- The admin can call any function gated by `assert_has_role` for any role without an explicit grant.
- Granting `Role::Admin` to another address is redundant — they already implicitly hold it.

## Role management functions

### `grant_role`

```rust
fn grant_role(env: Env, admin: Address, user: Address, role: Role);
```

- **Authorization:** Only the admin (`core::assert_admin`) may call this.
- **Effect:** Stores `DataKey::Role(user, role) = true`.
- **Event:** Emits `RoleGrantedEvent`.

### `revoke_role`

```rust
fn revoke_role(env: Env, admin: Address, user: Address, role: Role);
```

- **Authorization:** Only the admin may call this.
- **Effect:** Removes `DataKey::Role(user, role)` from persistent storage.
- **Event:** Emits `RoleRevokedEvent`.

### `has_role`

```rust
fn has_role(env: Env, user: Address, role: Role) -> bool;
```

- **Authorization:** None required (read-only).
- Returns `true` if `user` is the admin or has the role stored.

## Internal assertion helpers

### `core::assert_admin`

Called by the admin component and role management functions. Does two things:
1. Calls `admin.require_auth()` — proves the caller signed the transaction.
2. Checks `admin == get_admin()` — proves the signer is the current admin.

### `access_control_component::assert_has_role`

Calls `user.require_auth()` then checks `has_role(user, role)`. Panics with `NotAuthorized` if either fails.

## Role checks vs `require_auth`

Both are required but serve different purposes:

| Check | Proves | Example |
|-------|--------|---------|
| `require_auth` | *Who signed* the transaction | `payer.require_auth()` in `pay_invoice` |
| Role/admin check | *Who may* perform the operation | `core::assert_admin(env, &admin)` in `set_fee` |

A function may use one or both. For example, `set_fee` requires both admin authorization (role) and the admin to sign (auth). `pay_invoice` only requires the payer to sign — no role check.

## Permission matrix

Every public function in `ShadeTrait` is listed below. "Admin" means only the stored admin address may call it. "Merchant" means the registered merchant. "Payer" means the transaction signer. "Anyone" means any address.

| Function | Caller | Auth check |
|----------|--------|------------|
| `initialize` | Anyone (once) | None (but panics if already initialized) |
| `get_admin` | Anyone | None |
| `add_accepted_token` | Admin | `assert_admin` |
| `add_accepted_tokens` | Admin | `assert_admin` |
| `remove_accepted_token` | Admin | `assert_admin` |
| `is_accepted_token` | Anyone | None |
| `set_account_wasm_hash` | Admin | `assert_admin` |
| `set_fee` | Admin | `assert_admin` |
| `get_fee` | Anyone | None |
| `set_platform_account` | Admin | `assert_admin` |
| `get_platform_account` | Anyone | None |
| `set_token_oracle` | Admin | `assert_admin` |
| `get_token_oracle` | Anyone | None |
| `propose_fee` | Admin | `assert_admin` |
| `execute_fee` | Admin | `assert_admin` |
| `get_pending_fee` | Anyone | None |
| `register_merchant` | Anyone | `require_auth` |
| `get_merchant` | Anyone | None |
| `get_merchants` | Anyone | None |
| `is_merchant` | Anyone | None |
| `set_merchant_status` | Admin | `assert_admin` |
| `is_merchant_active` | Anyone | None |
| `verify_merchant` | Admin | `assert_admin` |
| `is_merchant_verified` | Anyone | None |
| `create_invoice` | Merchant | `require_auth` |
| `create_fiat_invoice` | Merchant | `require_auth` |
| `create_invoice_draft` | Merchant | `require_auth` |
| `finalize_invoice` | Merchant | `require_auth` |
| `create_invoice_signed` | Anyone (with valid signature) | Signature verification |
| `get_invoice` | Anyone | None |
| `resolve_invoice_amount` | Anyone | None |
| `refund_invoice` | Merchant | `require_auth` |
| `claim_refund` | Buyer | `require_auth` |
| `set_merchant_key` | Merchant | `require_auth` |
| `get_merchant_key` | Anyone | None |
| `grant_role` | Admin | `assert_admin` |
| `revoke_role` | Admin | `assert_admin` |
| `has_role` | Anyone | None |
| `get_invoices` | Anyone | None |
| `refund_invoice_partial` | Merchant | `require_auth` |
| `pause` | Admin | `assert_admin` |
| `unpause` | Admin | `assert_admin` |
| `is_paused` | Anyone | None |
| `upgrade` | Admin | `assert_admin` (via upgrade component) |
| `restrict_merchant_account` | Admin | `assert_admin` |
| `calculate_fee` | Anyone | None |
| `compute_platform_fee_split` | Anyone | None |
| `set_merchant_platform_fee` | Admin | `assert_admin` |
| `get_merchant_platform_fee` | Anyone | None |
| `clear_merchant_platform_fee` | Admin | `assert_admin` |
| `pay_invoice` | Payer | `require_auth` |
| `pay_invoices_batch` | Payer | `require_auth` |
| `pay_invoice_partial` | Payer | `require_auth` |
| `void_invoice` | Merchant | `require_auth` |
| `amend_invoice` | Merchant | `require_auth` |
| `propose_admin_transfer` | Admin | `assert_admin` |
| `accept_admin_transfer` | New admin | `require_auth` + pending check |
| `set_merchant_webhook` | Merchant | `require_auth` |
| `get_merchant_webhook` | Anyone | None |
| `set_merchant_accepted_tokens` | Merchant | `require_auth` |
| `get_merchant_accepted_tokens` | Anyone | None |
| `remove_merchant_accepted_token` | Merchant | `require_auth` |
| `is_token_accepted_for_merchant` | Anyone | None |
| `get_user_transactions` | Anyone | None |
| `approve_withdrawal` | Signer | `require_auth` |
| `search_invoices_paginated` | Caller | `require_auth` |
| `search_merchants_paginated` | Anyone | None |
| `find_merchant_id` | Anyone | None |

## Known gaps and recommendations

- **Role enumeration:** There is no on-chain query to list all addresses holding a given role. Off-chain indexing is required.
- **Timelocked role changes:** Role grants and revocations take effect immediately. A timelock would allow monitoring before changes apply.
- **Emergency role revocation:** There is no emergency mechanism to bulk-revoke all roles. The admin must revoke each individually.
- **Unused roles:** `Manager` and `Operator` are defined but not yet gated on any contract function. They exist as reserved slots for future use.

← [Back to security](README.md)
