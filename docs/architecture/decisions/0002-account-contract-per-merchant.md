# 0002. Dedicated account contract per merchant deployed by the hub

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

Merchants receive payments in multiple tokens and need to withdraw, refund, and track balances. Holding every merchant's funds in the hub contract would mix balances and make a single bug or compromise affect all merchants.

## Decision

After registering, a merchant calls `deploy_merchant_account`, and the hub deploys a dedicated `account` contract (via the `account_factory` component) from a stored WASM hash, with itself as the account's manager. This is the only way to link an account: the hub never accepts an address it did not deploy. Settled funds go to that account, which the merchant controls for withdrawals.

## Consequences

- Merchant funds are isolated per contract; balances are simply token balances of the account address.
- Per-merchant restriction, verification, and analytics live with the account.
- Deployment is a separate call, so registration stays cheap and works before a WASM hash is set. A merchant cannot be paid until it has deployed.
- The hub is the manager, so it must expose every manager action as its own entry point.
- The manager can move funds through `refund` and `sweep_to`, so account isolation does not protect against a bug in, or a malicious upgrade of, the hub.
- `account` has no upgrade function. Existing accounts keep the code they were deployed with.

## Alternatives considered

- **Pooled balances in the hub with internal ledgering:** cheaper but concentrates risk and complicates accounting.
- **Merchant-supplied wallet address only:** no on-chain refund or withdrawal controls, and no protocol-level restrictions.
