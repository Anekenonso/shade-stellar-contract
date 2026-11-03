# Payment payloads and swap routing

`PaymentPayload`, `SwapRoute`, and `PaymentRoute` describe pay-in and settle-out tokens with routing paths and slippage tolerance, validated by `validate_payment_payload`. `CrossChainBridgePayload` and `emit_bridge_placeholder` mark the intended cross-chain integration point. This page documents both and is explicit about what is implemented today versus reserved for future work.

## Why it exists

Shade supports payments where the payer's token differs from the [merchant](../glossary.md#merchant)'s settlement token. The payload types encode the routing intent so the contract can validate path correctness and slippage bounds before settlement. The bridge placeholder defines the event shape for future cross-chain flows.

## How it works

### Payment routes

A `PaymentPayload` describes how a payment should be routed:

- **Direct payment.** The input token equals the settlement token. No swap is needed.
- **Swap payment.** The input token differs from the settlement token. A `SwapRoute` specifies the router contract and the hop path.

### Validation

`validate_payment_payload` enforces correctness for both route types. The function does not execute a swap — it only validates that the payload is well-formed.

**Direct route rules:**

1. `input_token` must equal `settlement_token`. If not → error `InvalidSwapPath`.
2. `max_slippage_bps` must be `None`. If set → error `InvalidSlippage`.

**Swap route rules:**

1. `route.path.len()` must be ≥ 2. If not → error `InvalidSwapPath`.
2. First hop must equal `input_token`. If not → error `InvalidSwapPath`.
3. Last hop must equal `settlement_token`. If not → error `InvalidSwapPath`.
4. `input_token` must not equal `settlement_token`. If equal → error `InvalidSwapPath`.
5. `max_slippage_bps` must be `Some`. If `None` → error `InvalidSlippage`.
6. `max_slippage_bps` value must be ≤ 10,000 (100%). If exceeded → error `InvalidSlippage`.

## Relevant types and storage

| Type / key | Defined in | Purpose |
|---|---|---|
| `PaymentPayload` | [`contracts/shade/src/types.rs#L676-L681`](../../contracts/shade/src/types.rs) | Describes input token, settlement token, route, and slippage |
| `PaymentRoute` | [`contracts/shade/src/types.rs#L644-L647`](../../contracts/shade/src/types.rs) | Enum: `Direct` or `Swap(SwapRoute)` |
| `SwapRoute` | [`contracts/shade/src/types.rs#L651-L654`](../../contracts/shade/src/types.rs) | Router address and hop path |

### PaymentPayload fields

| Field | Type | Description |
|---|---|---|
| `input_token` | `Address` | Token the payer is sending |
| `settlement_token` | `Address` | Token the [merchant](../glossary.md#merchant) receives |
| `route` | `PaymentRoute` | `Direct` or `Swap` routing |
| `max_slippage_bps` | `Option<u32>` | Maximum slippage in basis points (required for swaps, must be `None` for direct) |

### SwapRoute fields

| Field | Type | Description |
|---|---|---|
| `router` | `Address` | Address of the swap router contract |
| `path` | `Vec<Address>` | Ordered list of token addresses forming the swap hop path |

## Relevant functions

| Function | Defined in | Purpose |
|---|---|---|
| `validate_payment_payload` | [`contracts/shade/src/components/payment.rs#L7-L48`](../../contracts/shade/src/components/payment.rs) | Validate payload correctness without executing swaps |

## What is implemented today

- **Payload validation.** `validate_payment_payload` is fully implemented and enforced.
- **Direct payments.** Payer sends the settlement token directly; no swap occurs.

## What is reserved for future work

- **On-chain swap execution.** The `SwapRoute` path is validated but no swap is executed on-chain. An integrator must perform the swap off-chain or through a separate swap contract before calling the Shade payment function.

> **Warning:** Do not assume that a swap route in a `PaymentPayload` will execute a token swap on-chain. The contract only validates the payload. You must execute the swap off-chain and ensure the payer sends the correct settlement token.

## Forward-compatibility contract

Integrators can rely on:

- The `PaymentPayload` struct and `validate_payment_payload` behaviour will remain stable.

What may change when real swap support lands:

- New validation rules may be added for swap execution.
- The `SwapRoute` type may be extended with execution parameters (deadline, minimum output).

> **Note:** Slippage is represented in basis points where 10,000 = 100%. A `max_slippage_bps` of 250 means 2.5% maximum slippage.

## Constraints and edge cases

- **Direct route slippage.** Must be `None`; providing a value is rejected with `InvalidSlippage`.
- **Swap route slippage.** Must be `Some` and ≤ 10,000. Values above 10,000 are rejected with `InvalidSlippage`.
- **Single-hop swap.** A path of length 1 is rejected with `InvalidSwapPath` (minimum 2 hops).

## Related pages

- [Auto-withdrawal and merchant settlement](auto-withdrawal.md)
