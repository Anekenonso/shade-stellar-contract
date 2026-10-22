# 0003. Time-locked fee changes via `propose_fee` / `execute_fee`

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

Protocol fees directly affect merchant revenue. An instant fee change by the admin, whether malicious or a mistake, could surprise merchants with in-flight invoices.

## Decision

Fee changes are two calls: `propose_fee` records a pending fee with `proposed_at`, and `execute_fee` applies it only after `FEE_UPDATE_DELAY` (172 800 seconds, 48 hours) has elapsed. Both emit events. See [Time and intervals](../../concepts/time-and-intervals.md#fee-time-lock).

## Consequences

- Merchants and indexers get a 48-hour notice window for every fee change.
- Urgent fee corrections are delayed by at least 48 hours.
- Re-proposing restarts the clock.

## Alternatives considered

- **Immediate `set_fee`:** simplest, but gives merchants no notice.
- **Governance vote on every fee change:** stronger guarantees but too slow and complex for routine adjustments.
