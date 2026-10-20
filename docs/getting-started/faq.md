# Frequently Asked Questions (FAQ)

Frequently asked questions about Shade Protocol, covering architectural boundaries, custody, supported assets, fee logic, governance, and integration models.

---

## General & Protocol Architecture

### 1. What is Shade Protocol, and what is it NOT?
* **What Shade is**: Shade is a decentralized, non-custodial payment gateway built on Stellar/Soroban. It provides on-chain merchant billing, multi-token invoice settlement, partial payments, recurring subscriptions, milestone escrow, event ticketing, and crowdfunding pledges enforced entirely by smart contracts.
* **What Shade is NOT**:
  * **Not a custodial payment processor**: Shade never pools funds into a centralized custody account. Payments move directly from the payer to the merchant (or the merchant's dedicated smart contract account), splitting protocol fees atomically.
  * **Not an off-chain payment rail**: Shade does not process credit cards, bank wires, or fiat ACH directly. Fiat amounts are converted to crypto amounts at settlement time using on-chain SEP-40 price feeds.
  * **Not an AMM or DEX**: While Shade supports swap-routing payloads via payment payloads, the core contracts do not operate liquidity pools; they integrate with Stellar Asset Contracts and external liquidity.

### 2. Is Shade Protocol custodial or non-custodial?
**Completely non-custodial**. When a customer calls `pay_invoice`, the Soroban runtime executes token transfers directly from the payer's address. Funds are split in that exact transaction: the net amount reaches the merchant's address (or their dedicated `account` contract), and the platform fee is routed to the platform treasury. No intermediary holds custody of funds during transit.

### 3. Which networks does Shade Protocol support?
Shade Protocol is deployed and tested on:
* **Stellar Mainnet**: Production settlement.
* **Stellar Testnet**: Public test environment for integrators and developers.
* **Stellar Futurenet**: Sandbox for upcoming Protocol upgrades and bleeding-edge Soroban features.
* **Local Sandbox**: Standalone local networks using `stellar container` for fast, reproducible automated testing.

### 4. Which tokens can be used for payments?
Any token issued on Stellar that complies with the **Stellar Asset Contract (SAC)** or **SEP-41 Token Interface** can be used, provided it has been added to the protocol's accepted tokens list by the administrator (`admin::add_accepted_token`). Common tokens include:
* **USDC** (Circle's USD Coin on Stellar)
* **XLM** (Native Lumens via SAC)
* Regulated stablecoins and custom merchant loyalty tokens.
Additionally, merchants can configure per-merchant accepted token whitelists (`set_merchant_accepted_tokens`).

---

## Fees, Invoicing & Settlement

### 5. How are platform fees calculated and collected?
Platform fees are configured per token as basis points (bps, where 100 bps = 1%) up to a protocol-defined safety ceiling.
* **Calculation**: Fee = `(Payment Amount * Token Fee BPS) / 10,000`.
* **Volume Discounts**: High-volume merchants can earn volume tier discounts, reducing their effective fee basis points.
* **Collection**: The fee is deducted atomically during payment execution. If an invoice is for 100 USDC and the platform fee is 1% (1 USDC), 99 USDC is transferred to the merchant and 1 USDC to the treasury. The payer always spends the exact invoice amount.

### 6. Who can change the protocol fees, and can they be changed instantly?
Platform fees are governed by a **two-phase timelock**:
1. The admin calls `propose_fee(token, new_fee_bps)`.
2. A mandatory timelock delay must elapse before the change can take effect.
3. The admin calls `execute_fee(token)` after the timelock window expires.
This prevents sudden fee increases and gives merchants transparent notice before any adjustment.

### 7. How does fiat-pegged pricing work?
Merchants can issue invoices denominated in fiat (e.g., $50.00 USD) using `create_fiat_invoice`.
* The contract queries a configured **SEP-40 Oracle** (such as Band Protocol or Reflector) for the real-time exchange rate between the fiat currency and the settlement token.
* On the first payment call (`pay_invoice`), the contract locks the conversion rate and verifies that the oracle price has not exceeded its maximum allowed staleness threshold (`max_price_staleness_seconds`).

### 8. What happens with partial payments and overpayments?
* **Partial Payments**: Supported via `pay_invoice_partial`. The first payer's address is recorded and locked as the designated payer for all subsequent partial payments to prevent multi-payer collision.
* **Overpayments**: Strictly rejected by the contract. If a payer attempts to pay more than the remaining balance (`amount - amount_paid`), the contract aborts with `ContractError::InvalidAmount`.

### 9. Can payments be refunded?
Yes, subject to merchant configuration:
* **Merchant-initiated refunds**: A merchant can issue a full or partial refund using `refund_invoice` or `refund_invoice_partial` within the designated refund time window (`refund_window_seconds`).
* **Platform fee handling**: Refunds transfer funds back from the merchant to the original payer. The contract configuration determines whether platform fees are reimbursed.
* **Buyer claims on expired invoices**: Unpaid or expired invoices cannot be refunded because funds were never transferred, but cancelled escrow deposits can be reclaimed.

---

## Merchants & Accounts

### 10. Why does each merchant have a dedicated `account` contract?
When a merchant registers on the hub, Shade deploys a standalone `account` contract for that merchant using the account WASM hash. This provides:
* **Isolated balance accounting**: Funds collected across invoices, subscriptions, and tickets can accumulate securely in the merchant's dedicated smart contract.
* **Automated settlement**: Merchants can configure auto-withdrawal thresholds and payout schedules (`set_auto_withdrawal`).
* **Granular access control**: Multiple operational keys (manager, cashier, withdrawal bot) can interact with the merchant account without exposing the root administrative key.

---

## Security, Upgrades & Governance

### 11. Who has the authority to pause the protocol?
The protocol implements role-based access control with an emergency pausable pattern:
* Addresses holding the **Emergency Pauser** role or the **Admin** role can call `pause` in the event of an detected vulnerability or oracle failure.
* When paused, all fund-moving actions (`pay_invoice`, `create_escrow`, `withdraw`) are instantly frozen.
* Read operations and administrative diagnostics remain operational.
* Only the **Admin** can call `unpause` once the issue has been resolved.

### 12. How are smart contract upgrades executed?
Shade contracts are upgradeable using Soroban's native `env.deployer().update_current_contract_wasm(new_wasm_hash)`:
* The upgrade process requires administrative authorization.
* Deployed storage keys conform to forward-compatible layouts (`DataKey` enum definitions).
* State migrations are verified against storage layout golden tests before any upgrade is executed on-chain.

---

## Still Stuck?

If your question is not answered here:
* Check the [Developer Troubleshooting Guide](troubleshooting.md) for concrete error messages and fixes.
* Search existing discussions and report new issues on the [GitHub Issue Tracker](https://github.com/ShadeProtocol/shade-stellar-contract/issues).
* Consult the [Error Codes Reference](../reference/errors.md) for exhaustive error documentation.
