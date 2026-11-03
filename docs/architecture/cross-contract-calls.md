# Cross-Contract Calls and the Factory Pattern

This document details how the Shade Protocol executes cross-contract calls, manages deterministic contract deployments via the Factory Pattern, interacts with SEP-41 token contracts, and handles authorization propagation and atomic failure semantics across the Stellar network.

---

## The Factory Pattern in Shade

Shade uses a deterministic on-chain contract factory to deploy one isolated `account` contract per merchant.

### How On-Chain Factories Work
1. **WASM Hash Registration**: An administrator uploads the compiled bytecode of the child contract to the Stellar ledger, producing a 32-byte `wasm_hash`. The admin then registers this hash in the hub contract (e.g. `set_account_wasm_hash`).
2. **Deterministic Salt Derivation**: The factory hashes the merchant ID, so each merchant has one predictable address:
   ```rust
   let salt = env.crypto().keccak256(&Bytes::from_slice(env, &merchant_id.to_be_bytes()));
   ```
3. **Deployer API**: The contract calls Soroban's `deployer().with_current_contract(salt).deploy_v2(wasm_hash, ())` to instantiate the new contract instance.
4. **Initialization & Address Storage**: The factory immediately invokes the child contract's `initialize` entrypoint and stores the resulting `Address` in persistent storage indexed by the entity's ID.

```
       ┌──────────────────────────────────────────────────┐
       │                 Shade Hub Contract               │
       └─────────┬──────────────────────────────┬─────────┘
                 │ (1) deploy(wasm_hash, salt)  │ (3) cross-contract call
                 ▼                              ▼
      ┌────────────────────┐          ┌────────────────────┐
      │  Merchant Account  │          │   SEP-41 Token     │
      │  Contract (Vault)  │          │   Contract (USDC)  │
      └────────────────────┘          └────────────────────┘
```

---

## Deployed Factory Components

| Factory Module | Target Crate | Deployment Trigger | Salt & ID Derivation | Primary Responsibility |
| :--- | :--- | :--- | :--- | :--- |
| **`account_factory`** *(Component)* | `account` | `deploy_merchant_account` | `keccak256(merchant_id)` | Deploys isolated merchant balance vault with restricted withdrawals. |

---

## Token Interactions (SEP-41)

All asset movements utilize the Soroban SEP-41 standard Token Client (`soroban_sdk::token::Client`).

### Payment Routing Architecture (`pay_invoice`)
When a buyer executes an invoice payment, the transaction executes up to three atomic token transfers:

1. **Payer $\rightarrow$ Shade Contract**: The contract transfers the total invoiced amount from the payer's wallet.
   ```rust
   token_client.transfer(&payer, &env.current_contract_address(), &total_amount);
   ```
2. **Shade Contract $\rightarrow$ Platform Fee Account**: The contract deducts the platform fee (if configured) and routes it to `platform_account`.
   ```rust
   if fee_amount > 0 {
       token_client.transfer(&env.current_contract_address(), &platform_account, &fee_amount);
   }
   ```
3. **Shade Contract $\rightarrow$ Merchant (or Merchant Account)**: The net amount (`total_amount - fee_amount`) is transferred directly to the merchant address or credited to their dedicated `account` vault.
   ```rust
   token_client.transfer(&env.current_contract_address(), &merchant_destination, &net_amount);
   ```

---

## Authorization Propagation Across Contract Boundaries

Soroban enforces an explicit authorization model where signatures and permissions do not automatically flow across cross-contract boundaries:

- **Caller Signatures (`require_auth`)**: When the user calls `pay_invoice`, the payer must authorize the invocation. The token contract's `transfer` verifies that the `payer` approved the transfer to the Shade contract.
- **Contract as Authorizer**: Once tokens are held by the `shade` contract address (`env.current_contract_address()`), subsequent transfers (`contract -> merchant` and `contract -> platform`) are authorized directly by the Shade contract itself without requiring further payer signatures.
- **Merchant Account Authorization**: In the `account` contract, withdrawals require `merchant.require_auth()`, ensuring only the rightful owner can withdraw accumulated revenue.

---

## Failure Semantics & Atomic Guarantees

- **All-or-Nothing Atomicity**: Soroban transactions execute within a single atomic host environment. If any cross-contract call fails (e.g. token contract rejects transfer due to insufficient balance, or a merchant account is restricted), the entire call stack panics and all state changes and balances roll back.
- **No Partial Payouts**: In multi-step settlements like `pay_invoice` or batch refunds, failure at the fee deduction step rolls back the initial payer transfer, preventing stranded funds or orphaned accounting records.
