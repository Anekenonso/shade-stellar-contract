# Resource costs and Soroban limits

Soroban transactions are bounded by per-transaction limits on CPU instructions, memory, ledger entry reads and writes, written bytes, and event size. Batch operations grow linearly with batch size and hit those limits first. This page publishes measured costs and the largest safe batch size, and explains how to regenerate and check them.

> **Status:** only `process_billing_cycle` in the standalone `subscription` contract is measured so far. The Shade hub operations (`register_merchant`, `create_invoice`, `pay_invoice`, `pay_invoices_batch`, `purchase_tickets_bulk`, `get_invoices`) are **not yet measured** because the `shade` crate's test target does not currently compile on `main` (missing `DataKey` variants referenced from `components/kyc.rs` and `components/campaign_royalties.rs`). See [Unmeasured operations](#unmeasured-operations).

## Limits used

Per-transaction limits assumed below (Stellar network settings, Protocol 23). They are network configuration, not contract constants, and validators can vote to change them. Confirm the current values with `stellar network settings` or the network's `getNetworkConfig` before relying on them.

| Resource | Limit |
|---|---|
| CPU instructions | 100,000,000 |
| Memory | 41,943,040 bytes |
| Ledger entries read | 100 |
| Ledger entries written | 50 |
| Bytes written | 132,096 |
| Contract event bytes | 16,384 |

A batch is "safe" only if it is within **every** limit.

## How the numbers were measured

Measurements come from the Soroban test environment: `env.cost_estimate().resources()` after a single top-level invocation, with the budget reset to unlimited so that over-limit batches can still be measured. The test is [`contracts/subscription/src/test_resource_costs.rs`](../../contracts/subscription/src/test_resource_costs.rs).

> **Warning:** The test environment runs the contract as native Rust, not compiled WASM, and uses the built-in token contract. Instruction and memory counts for a deployed WASM contract are higher, so treat these numbers as a lower bound for CPU and memory. Entry counts, written bytes and event bytes do not depend on native vs WASM execution and are directly comparable. Always confirm a chosen batch size with a testnet simulation.

## `process_billing_cycle`

Every subscription in the batch is charged successfully (the first charge of a new subscription, funded and approved). Plan interval is 30 days, amount 1,000.

| Batch size | Instructions | Memory (bytes) | Read entries | Write entries | Bytes written | Event bytes | Within limits |
|---|---|---|---|---|---|---|---|
| 1 | 390,786 | 56,971 | 8 | 4 | 1,160 | 472 | yes |
| 5 | 2,168,477 | 332,299 | 20 | 16 | 4,904 | 2,360 | yes |
| 10 | 4,738,881 | 815,284 | 35 | 31 | 9,584 | 4,720 | yes |
| 16 | 8,342,406 | 1,598,476 | 53 | 49 | 15,200 | 7,552 | yes |
| 17 | 8,997,681 | 1,750,603 | 56 | 52 | 16,136 | 8,024 | **no** (writes) |
| 20 | 11,066,591 | 2,244,004 | 65 | 61 | 18,944 | 9,440 | **no** (writes) |
| 40 | 27,181,378 | 6,952,444 | 125 | 121 | 37,664 | 18,880 | **no** (reads, writes, events) |
| 80 | 73,958,742 | 23,773,324 | 245 | 241 | 75,104 | 37,760 | **no** (reads, writes, events) |

Costs per additional charged subscription are roughly 3 write entries, 3 read entries, 470 event bytes and about 0.5M instructions in this configuration. Write entries grow as `3n + 1`, so they are the first limit reached.

**Maximum safe batch size: 16 subscriptions per call** in this configuration. The next limits reached are read entries (about 31) and event bytes (about 34), then instructions on native execution (about 100 subscriptions). Because deployed WASM costs more CPU than the native figure, a margin below 16 is prudent when subscriptions use more expensive tokens, plans or paths (for example a pending downgrade, which loads a second plan). Size batches for the all-charged case.

## Unmeasured operations

| Operation | Contract | Batch? | Status |
|---|---|---|---|
| `register_merchant` | `shade` | No | Not measured |
| `create_invoice` | `shade` | No | Not measured |
| `pay_invoice` | `shade` | No | Not measured |
| `pay_invoices_batch` | `shade` | Yes (loop over `pay_invoice` logic, one auth) | **Safe batch size not yet determined** |
| `purchase_tickets_bulk` | `shade` | Quantity, not a loop | Not measured |
| `get_invoices` | `shade` | Read query over all invoices | Not measured |

Notes from the source that bear on cost, pending measurement:

- `pay_invoices_batch` runs the full single-payment logic (token transfer, fee split, storage updates, events) once per invoice, so its cost grows linearly like `process_billing_cycle` and will likely be limited by write entries as well.
- `purchase_tickets_bulk` performs one token transfer and one event update regardless of `quantity`, so its cost should be roughly flat in `quantity`.
- `get_invoices` iterates `1..=InvoiceCount` and reads one ledger entry per stored invoice, whatever the filter. Its read-entry count therefore equals the total number of invoices, so with a 100-entry read limit it cannot succeed on-chain once roughly 100 invoices exist. Use an off-chain index for larger sets.

These are expectations from reading the code, not measurements. To measure them, once the `shade` tests compile, add a test next to `test_resource_costs.rs` following the same pattern (build state, call `env.cost_estimate().budget().reset_unlimited()`, invoke once, read `env.cost_estimate().resources()`).

## Regenerating the numbers

```bash
cargo test -p subscription resource_costs -- --nocapture
```

The test prints the table above in Markdown and fails only if even a batch of one exceeds the limits. Copy the printed rows into this page. Update the limit constants at the top of the test file when the network settings change.

## Estimating fees

Fees depend on the resources above and on the network's current fee settings, so they cannot be derived from the test environment. To estimate a real transaction:

1. Build the transaction (for example with the Stellar CLI or an SDK).
2. Simulate it (`stellar contract invoke ... --sim-only`, or the RPC `simulateTransaction` method). The response contains the exact resource footprint and the minimum resource fee.
3. Compare the simulated resources to the limits above. If simulation fails with a budget or limit error, reduce the batch size.
4. Add an inclusion fee on top of the resource fee and a safety margin for ledger-state drift between simulation and submission.

No fee range is published here because it has not been measured on testnet.

## Most expensive operations

Among the measured set, `process_billing_cycle` is the only operation measured, and it is dominated by ledger writes per charge. Link any open optimisation issues here when they exist; none are referenced yet.

## Related pages

- [Subscriptions and recurring billing](../concepts/subscriptions.md)
- [Payment flows: full, partial, and batch](../concepts/payments.md)
