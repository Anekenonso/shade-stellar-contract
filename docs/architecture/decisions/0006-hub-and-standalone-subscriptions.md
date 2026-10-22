# 0006. Hub subscription module alongside the standalone subscription contract

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

Some merchants want recurring billing integrated with their Shade invoices, fees, and merchant accounts. Others want richer billing features (grace periods, billing allowances, prorated refunds) without depending on the hub.

## Decision

Keep both: the `subscription` component inside the shade hub for simple, hub-integrated plans, and the standalone `subscription` contract for advanced billing. See [Choosing a subscription model](../../concepts/choosing-a-subscription-model.md).

## Consequences

- Integrators can pick the model that fits their needs.
- Two code paths must be maintained, and their semantics can drift; docs must state which one applies.
- Features added to one model are not automatically available in the other.

## Alternatives considered

- **Hub-only subscriptions:** increases hub size and forces every advanced feature into the hub upgrade cycle.
- **Standalone-only subscriptions:** loses tight integration with hub fees and merchant accounts, and breaks existing hub plans.
