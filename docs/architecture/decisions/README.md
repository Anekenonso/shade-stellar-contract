# Architecture Decision Records

This directory records significant design decisions for the Shade contracts, so future contributors understand why things are the way they are.

## When an ADR is required

Write a new ADR (copy [`template.md`](template.md)) when a change:

- Adds, removes, or restructures a contract, factory, or shade component.
- Changes storage layout, upgrade strategy, or cross-contract call patterns.
- Changes admin, ownership, fee, or other privileged flows.
- Introduces a new dependency or toolchain requirement.
- Reverses or supersedes an existing ADR.

ADRs are numbered sequentially (`NNNN-short-title.md`) and never deleted. To change a decision, add a new ADR and mark the old one `Superseded by NNNN`.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-component-based-shade-contract.md) | Component-based structure inside the shade contract | Accepted |
| [0002](0002-account-contract-per-merchant.md) | Dedicated account contract per merchant deployed by the hub | Accepted |
| [0003](0003-time-locked-fee-changes.md) | Time-locked fee changes via `propose_fee` / `execute_fee` | Accepted |
| [0004](0004-two-step-admin-transfer.md) | Two-step admin transfer | Accepted |
| [0005](0005-separate-escrow-and-ticketing-factories.md) | Separate factories for escrow and ticketing contracts | Accepted |
| [0006](0006-hub-and-standalone-subscriptions.md) | Hub subscription module alongside the standalone subscription contract | Accepted |
