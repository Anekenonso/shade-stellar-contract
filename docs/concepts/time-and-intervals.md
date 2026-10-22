# Time, Ledger Timestamps, and Interval Semantics

Invoice expiry, time-locked fee changes, subscription intervals, grace periods, refund windows, and crowdfunding deadlines all depend on `env.ledger().timestamp()`. This page explains how time works on Soroban and how each Shade feature interprets it.

## Ledger timestamps

- **Unit:** seconds since the Unix epoch (UTC), as a `u64`.
- **Source:** the close time of the ledger the transaction is included in. Every invocation inside the same ledger sees the same value.
- **Granularity:** Stellar ledgers close roughly every 5–6 seconds, so the timestamp advances in steps, not continuously.
- **Precision:** the close time is agreed by validators and is not wall-clock precise. Expect drift of a few seconds and never rely on sub-ledger precision.
- **Monotonic:** the timestamp never decreases between ledgers.

Practical rule: design time windows with at least one ledger (≈ 5 s) of slack, and treat any deadline as "approximately at" rather than "exactly at".

## Time-dependent fields and parameters

All values are in seconds. "Comparison" is the check the contract performs against `now = env.ledger().timestamp()`.

| Contract / component | Field or constant | Meaning | Comparison |
|---|---|---|---|
| Shade `admin` | `FEE_UPDATE_DELAY = 172_800` (48 h) | Delay between `propose_fee` and `execute_fee` | Executes when `now - proposed_at >= delay` (rejects if `<`) |
| Shade `invoice` | `expires_at: Option<u64>` | Invoice expiry | Creation rejects `expires_at < now`; payment rejects `now >= expires_at` |
| Shade `invoice` | `expires_at` (escrow refund) | `claim_refund` window opens | Allowed when `now >= expires_at` (rejects if `<`) |
| Shade `invoice` | `MAX_REFUND_DURATION = 604_800` (7 d) | Refund window after payment | Rejects when `elapsed > duration` |
| Shade `subscription` | `interval` | Billing period | Charge allowed when `now >= last_charged + interval` |
| Subscription contract | `interval` (must be `> 0`) | Billing period | Charge allowed when `now >= last_charged + interval` |
| Subscription contract | `grace_period` (`0` = none) | Time a `PastDue` subscription survives | Terminated when `now > past_due_since + grace_period` |
| Subscription contract | `created_at`, `last_charged`, `past_due_since` | Recorded timestamps | Informational / inputs to the checks above |
| Subscription contract | prorated refund | `remaining = (last_charged + interval) - now`, clamped to `[0, interval]` | Zero refund when `now >= last_charged + interval` |
| Shade `campaigns` / Crowdfund | `deadline` | Campaign end | Creation rejects `deadline <= now`; pledges rejected when `now > deadline`; finalize/refund require `now > deadline` |
| Crowdfund factory | `voting_deadline` | Proposal voting end | Votes rejected when `now > deadline`; execution requires `now > deadline` |
| Ticketing | `start_time`, `end_time` | Pricing / sale window | Requires `end_time >= start_time` |
| Ticketing | check-in time | Recorded at `check_in` | Informational |
| Factories, Account, events | `deployed_at`, `date_created`, `timestamp` | Audit timestamps | Informational |

## Fee time-lock

`propose_fee` stores a pending fee with `proposed_at = now`. `execute_fee` succeeds only once `now - proposed_at >= FEE_UPDATE_DELAY` (48 hours). Executing at exactly `proposed_at + 172_800` succeeds; one second earlier fails. Proposing again replaces the pending fee and restarts the clock.

## Subscription intervals and grace periods

- **Next charge:** due at `last_charged + interval`. A charge at exactly that second succeeds (`now < due` is the rejection test).
- **Missed charges do not accumulate:** a late charge sets `last_charged = now`, so the schedule shifts forward rather than catching up.
- **Grace:** when a charge fails, the subscription becomes `PastDue` with `past_due_since = now`. With `grace_period = 0` it terminates immediately. Otherwise `enforce_grace` (or the next charge attempt) terminates it only when `now > past_due_since + grace_period`; at exactly the boundary the subscription is still in grace.
- **Prorated refunds:** at exactly `last_charged + interval` the remaining time is `0`, so the refund is `0`.

See [Subscriptions](subscriptions.md) and [Subscription Grace Periods](subscription-grace-periods.md).

## Testing time-based logic

Tests control time with the ledger test utilities:

```rust
use soroban_sdk::testutils::Ledger;

env.ledger().set_timestamp(1_000);
// ... propose_fee ...
env.ledger().set_timestamp(1_000 + 172_800); // exactly at the boundary
// ... execute_fee succeeds ...
```

Always test three points around every boundary: one second before, exactly at, and one second after. Examples in the suite:

- [`contracts/shade/src/tests/test_governance.rs`](../../contracts/shade/src/tests/test_governance.rs)
- [`contracts/shade/src/tests/test_invoice_filter.rs`](../../contracts/shade/src/tests/test_invoice_filter.rs)
- [`contracts/shade/src/tests/test_invoice_partial_refund.rs`](../../contracts/shade/src/tests/test_invoice_partial_refund.rs)
- [`contracts/subscription/src/test_grace.rs`](../../contracts/subscription/src/test_grace.rs)
- [`contracts/account/src/tests/test_withdrawal.rs`](../../contracts/account/src/tests/test_withdrawal.rs)
- [`contracts/escrow/src/test.rs`](../../contracts/escrow/src/test.rs)

## Frontend guidance

- Contract timestamps are UTC seconds. JavaScript uses milliseconds: `new Date(ts * 1000)` and `Math.floor(Date.now() / 1000)`.
- Convert to the user's time zone only for display (`Intl.DateTimeFormat`); always send UTC seconds to the contract.
- When a user picks a local date such as "expires end of day", convert that local instant to UTC before submitting.
- Do not compare against the browser clock to decide whether an action will succeed; read the latest ledger close time from RPC (`getLatestLedger`) and add a margin of a few ledgers.
- Show durations (intervals, grace periods) in human units but store them as seconds (for example `2_592_000` for 30 days).
