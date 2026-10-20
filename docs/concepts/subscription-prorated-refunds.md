# Subscription prorated cancellation refunds

`cancel_with_prorated_refund` cancels a subscription and returns the unused part of the current billing cycle to the customer. `quote_prorated_refund` previews that amount without changing state. Both live in the standalone `subscription` contract ([`contracts/subscription/src/lib.rs`](../../contracts/subscription/src/lib.rs)).

## Why it exists

`cancel_subscription` stops future charges but returns nothing for a cycle that was already paid. Merchants who want to offer fair mid-cycle cancellations need a predictable formula, and UIs need to show the amount before the customer commits.

## How it works

### The formula

The shared helper `prorated_refund(sub, plan, now)` computes the refund. Inputs:

| Input | Source |
|---|---|
| `last_charged` | `Subscription.last_charged`: ledger timestamp of the last successful charge (seconds). `0` means never charged. |
| `interval` | `Plan.interval`: billing interval in seconds. |
| `amount` | `Plan.amount`: the plan's **current** amount per interval, in token base units. |
| `now` | `env.ledger().timestamp()` at execution. |

```
if last_charged == 0:            refund = 0      (never charged)
cycle_end = last_charged + interval              (saturating add)
if now >= cycle_end:             refund = 0      (cycle fully consumed)
if interval == 0:                refund = 0
remaining = cycle_end - now
refund    = (amount * remaining) / interval      (integer division)
```

If `amount * remaining` overflows `i128`, the helper returns `0` rather than panicking.

### Rounding

The division is integer (floor) division after multiplying, so the result is rounded **down** to the nearest base unit. The remainder, always less than 1 base unit, stays with whoever funded the refund source (the merchant, or the plan creator when set). Customers therefore never receive more than the exact proportional amount, and lose at most 1 base unit to rounding. Because the multiplication happens first, no precision is lost before the division.

### Worked example

A plan charges `amount = 1_000` base units every `interval = 2_592_000` seconds (30 days). The customer was charged at `last_charged = 1_000_000`, so `cycle_end = 3_592_000`.

| Time of cancellation | `now` | `remaining` | Calculation | Refund |
|---|---|---|---|---|
| Immediately | 1_000_000 | 2_592_000 | 1_000 × 2_592_000 / 2_592_000 | **1_000** |
| One third elapsed | 1_864_000 | 1_728_000 | 1_000 × 1_728_000 / 2_592_000 = 666.67 | **666** (merchant keeps 334) |
| Halfway | 2_296_000 | 1_296_000 | 1_000 × 1_296_000 / 2_592_000 | **500** |
| Cycle over | 3_592_000 | 0 | `now >= cycle_end` | **0** (`NothingToRefund`) |

These cases match the tests in [`contracts/subscription/src/test_refund.rs`](../../contracts/subscription/src/test_refund.rs) (`test_quote_full_refund_immediately_after_charge`, `test_quote_half_refund_at_mid_cycle`, `test_quote_zero_after_full_cycle_elapsed`, `test_cancel_with_refund_uses_exact_quote_amount`). The tests allow a 1-unit tolerance on mid-cycle quotes because of the integer division.

> **Note:** The formula uses the plan's *current* `amount`, not the amount of the last charge, and expresses the refund in the subscription's billing token (`preferred_token` if set, otherwise `Plan.token`). If the plan amount changed after the last charge, the refund is computed from the new amount.

### Authorization and funding

`cancel_with_prorated_refund(caller, sub_id)` requires `caller.require_auth()`, and `caller` must be one of:

- the **customer** (`Subscription.customer`),
- the **merchant** (`Plan.merchant`), or
- the plan's **creator** (`Plan.creator`, when set).

Any other address fails with `NotAuthorized`.

Refunds need both sides to agree:

- If the customer calls, the refund source must also authorize in the same transaction.
- If the merchant or creator calls, the customer must also authorize.

The refund is paid from the **refund source**: `Plan.creator` when set, otherwise `Plan.merchant`. The contract moves the tokens with `transfer_from(contract, source, customer, refund)`, so the source must have approved the subscription contract for at least the refund amount beforehand. Without that allowance (or without the balance) the token call fails and the whole call reverts: the subscription stays active and no tokens move. The tests approve `PLAN_AMOUNT` from the merchant before cancelling.

On success the subscription status becomes `Cancelled` and a `SubExpired` event is emitted.

### Errors

| Error | Code | Raised when |
|---|---|---|
| `SubscriptionNotFound` | 8 | `sub_id` does not exist. |
| `SubscriptionNotActive` | 9 | Status is not `Active` or `PastDue`. A subscription can be cancelled with a refund only once. |
| `NotAuthorized` | 3 | `caller` is not the customer, merchant, or creator. |
| `NothingToRefund` | 15 | The computed refund is `<= 0`: never charged, cycle over, or rounds to zero. |

## Relevant functions

| Function | Defined in | Purpose |
|---|---|---|
| `cancel_with_prorated_refund` | [`contracts/subscription/src/lib.rs`](../../contracts/subscription/src/lib.rs) | Cancel and pay the prorated refund. |
| `quote_prorated_refund` | [`contracts/subscription/src/lib.rs`](../../contracts/subscription/src/lib.rs) | Read-only refund preview. |
| `cancel_subscription` | [`contracts/subscription/src/lib.rs`](../../contracts/subscription/src/lib.rs) | Cancel without a refund. |

## Choosing a cancellation function

| | `cancel_subscription` | `cancel_with_prorated_refund` |
|---|---|---|
| Who may call | Customer or merchant | Customer, merchant, or creator |
| Counterparty must also authorize | No | Yes |
| Refund | None | Unused part of the current cycle |
| Funding needed | None | Source allowance and balance for the refund |
| Customer's token allowance to the contract | Zeroed when the **customer** calls | Not changed |
| Works for a never-charged or fully elapsed cycle | Yes | No, panics with `NothingToRefund` |
| Allowed statuses | `Active`, `PastDue` | `Active`, `PastDue` |

Use `cancel_subscription` when no refund is owed, such as the end of a cycle, a subscription that was never charged, or a subscription in grace (`PastDue`) whose cycle has expired. Use `cancel_with_prorated_refund` when your terms promise a refund for unused time and the merchant has agreed to fund it.

> **Warning:** `cancel_with_prorated_refund` does not zero the customer's remaining allowance. A cancelled subscription rejects further charges, but a customer who wants the approval removed should also call `revoke_billing_authorization`.

## Showing a quote in a UI

1. Call `quote_prorated_refund(sub_id)` through a simulation (no signature needed). It returns the refund in base units, or `0` if none would be paid.
2. Display the amount, formatted with the token's decimals, and disable the "cancel with refund" option when the quote is `0`.
3. Make sure the refund source has approved the contract for at least the quoted amount. Otherwise the cancel transaction will fail.
4. Collect both signatures (the caller and the counterparty) and submit `cancel_with_prorated_refund`.

> **Warning:** The quote is a function of the ledger timestamp. It shrinks every second, and the amount actually paid is computed at execution time, not at simulation time. The paid amount will usually be slightly lower than the quote the user saw. Present it as an estimate and, where the funding allowance is exact, approve a small buffer (up to the full plan `amount`), since the refund can never exceed `Plan.amount`.

`quote_prorated_refund` does not check the subscription's status. For a subscription that is already `Cancelled` it still returns the time-based number, even though `cancel_with_prorated_refund` would fail with `SubscriptionNotActive`. Check `get_subscription(sub_id).status` before showing the quote.

## Related pages

- [Subscriptions and recurring billing](./subscriptions.md)
- [Subscription billing authorization and allowances](./subscription-billing-authorization.md)
- [Subscription grace periods and past-due handling](./subscription-grace-periods.md)
- [Refunds and voids](./refunds-and-voids.md)
- [Subscription contract reference](../contracts/subscription.md)
