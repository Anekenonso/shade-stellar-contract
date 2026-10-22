# 0002. Dedicated account contract per merchant deployed by the hub

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

Merchants receive payments in multiple tokens and need to withdraw, refund, and track balances. Holding every merchant's funds in the hub contract would mix balances and make a single bug or compromise affect all merchants.

## Decision

On merchant registration the hub deploys a dedicated `account` contract (via the `account_factory` component) from a stored WASM hash. Settled funds go to that account, which the merchant controls for withdrawals and refunds.

## Consequences

- Merchant funds are isolated per contract; balances are simply token balances of the account address.
- Per-merchant restriction, verification, and analytics live with the account.
- Each registration pays a deployment cost, and account upgrades must be rolled out per merchant.

## Alternatives considered

- **Pooled balances in the hub with internal ledgering:** cheaper but concentrates risk and complicates accounting.
- **Merchant-supplied wallet address only:** no on-chain refund or withdrawal controls, and no protocol-level restrictions.
