# Time, Ledger Timestamps, and Interval Semantics

Invoice expiry, time-locked fee changes, and refund windows all depend on `env.ledger().timestamp()`. This page explains how time works on Soroban and how each Shade feature interprets it.

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
| Shade `invoice` | `expires_at` (expiry refund) | `claim_refund` window opens | Allowed when `now >= expires_at` (rejects if `<`) |
| Shade `invoice` | `MAX_REFUND_DURATION = 604_800` (7 d) | Refund window after payment | Rejects when `elapsed > duration` |
| Account, events | `deployed_at`, `date_created`, `timestamp` | Audit timestamps | Informational |

## Fee time-lock

`propose_fee` stores a pending fee with `proposed_at = now`. `execute_fee` succeeds only once `now - proposed_at >= FEE_UPDATE_DELAY` (48 hours). Executing at exactly `proposed_at + 172_800` succeeds; one second earlier fails. Proposing again replaces the pending fee and restarts the clock.

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

- [`contracts/shade/src/tests/test_invoice_filter.rs`](../../contracts/shade/src/tests/test_invoice_filter.rs)
- [`contracts/shade/src/tests/test_invoice_partial_refund.rs`](../../contracts/shade/src/tests/test_invoice_partial_refund.rs)
- [`contracts/account/src/tests/test_withdrawal.rs`](../../contracts/account/src/tests/test_withdrawal.rs)

## Frontend guidance

- Contract timestamps are UTC seconds. JavaScript uses milliseconds: `new Date(ts * 1000)` and `Math.floor(Date.now() / 1000)`.
- Convert to the user's time zone only for display (`Intl.DateTimeFormat`); always send UTC seconds to the contract.
- When a user picks a local date such as "expires end of day", convert that local instant to UTC before submitting.
- Do not compare against the browser clock to decide whether an action will succeed; read the latest ledger close time from RPC (`getLatestLedger`) and add a margin of a few ledgers.
- Show durations (intervals, grace periods) in human units but store them as seconds (for example `2_592_000` for 30 days).
