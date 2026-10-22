# 0004. Two-step admin transfer

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

The admin controls fees, pausing, upgrades, and whitelists. Transferring admin to a mistyped or unusable address would permanently lock those controls.

## Decision

Admin transfer is a two-step flow: the current admin nominates a pending admin, and the nominee must call to accept (with `require_auth`) before ownership changes. See [Admin and ownership](../../security/admin-and-ownership.md).

## Consequences

- A transfer cannot complete unless the new address can actually sign.
- The current admin can cancel or overwrite a pending nomination.
- Transfers take two transactions instead of one.

## Alternatives considered

- **Single-call transfer:** one typo away from losing admin permanently.
- **Multisig-only admin:** safer but forces every deployment into a multisig setup.
