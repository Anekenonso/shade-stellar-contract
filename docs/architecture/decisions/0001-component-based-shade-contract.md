# 0001. Component-based structure inside the shade contract

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

The shade hub contract covers invoices, payments, fees, merchants, subscriptions, escrow, KYC, governance, and more. A single `#[contractimpl]` file holding all of that logic would be hard to review, test, and extend.

## Decision

`contracts/shade/src/shade.rs` exposes the public `ShadeTrait` surface as thin entry points that delegate to modules in `contracts/shade/src/components/` (for example `admin`, `invoice`, `merchant`, `pausable`, `reentrancy`). Each component owns its storage helpers and business rules. See [Component pattern](../component-pattern.md).

## Consequences

- Features are isolated and reviewable on their own; cross-cutting guards (pause, reentrancy, access control) are reused.
- Everything still ships as one WASM, so size and resource limits are shared across all components.
- Contributors must keep the entry point, interface trait, and component in sync.

## Alternatives considered

- **Monolithic contract file:** simplest to start but unmanageable at the current feature count.
- **One contract per feature:** stronger isolation but adds cross-contract call cost, more deployments, and more addresses for integrators to track.
