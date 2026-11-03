# Threat model and security assumptions

This page consolidates the Shade Protocol's security posture into a single document. It is the page an auditor or integrator's security reviewer reads first.

## Assets at risk

| Asset | Description |
|-------|-------------|
| Merchant balances | Funds held in merchant account contracts, credited from invoice payments. |
| In-flight payments | Tokens transferred from a payer during `pay_invoice` before settlement. |
| Fee revenue | Platform fees routed to `PlatformAccount` from every payment. |
| Protocol configuration | Admin-controlled settings: accepted tokens, fee rates, oracle configs, role assignments, WASM upgrades. |

## Trust assumptions

The protocol assumes the following hold true:

| Assumption | Description |
|------------|-------------|
| Honest admin | The admin address acts in the protocol's interest. A compromised admin can pause the contract, change fees, upgrade WASM, and assign arbitrary roles. |
| Honest oracles | Configured oracles return accurate price feeds. A compromised oracle can manipulate fiat-denominated invoice amounts. |
| Well-behaved SEP-41 tokens | Accepted tokens conform to the SEP-41 standard. A malicious token contract could drain funds via reentrancy or return unexpected values. |
| Soroban host guarantees | The Soroban runtime correctly enforces authorization, storage isolation, and contract sandboxing. |

## Attacker classes

| Attacker class | Description |
|----------------|-------------|
| Malicious payer | A user who pays invoices but attempts to manipulate payment amounts, exploit partial payments, or replay signed payloads. |
| Malicious merchant | A registered merchant who attempts to create fraudulent invoices, claim unauthorized refunds, or manipulate analytics. |
| Compromised admin | An attacker who gains control of the admin key and can reconfigure the entire protocol. |
| Compromised oracle | An attacker who manipulates price feeds to alter fiat-denominated amounts. |
| Adversarial token contract | A SEP-41 token that behaves maliciously during transfers or approvals. |

## Attack surfaces

### Invoice creation and payment

| Threat | Mitigation | Residual risk |
|--------|------------|---------------|
| Payer pays a different amount than the invoice requires | `pay_invoice` transfers the exact `amount` field from the payer. Partial payments use `pay_invoice_partial` with explicit amount. | None — amounts are enforced. |
| Replay of a signed invoice payload | Each signed invoice requires a unique 32-byte nonce. Used nonces are stored and rejected (`NonceAlreadyUsed`). | If a merchant reuses a nonce off-chain, the second submission fails. The contract cannot prevent off-chain nonce reuse — it only prevents on-chain replay. |
| Payment after invoice expiry | `pay_invoice` panics with `InvoiceExpired` if `expires_at` has passed. | Invoice expiry depends on the ledger timestamp, which may drift slightly from wall-clock time. |
| Double-spend via batch payment | `pay_invoices_batch` pays each invoice individually. A failed payment for one invoice does not affect others. | If the payer has insufficient balance for the full batch, earlier invoices may succeed while later ones fail, leaving a partial state. |

### Refunds

| Threat | Mitigation | Residual risk |
|--------|------------|---------------|
| Unauthorized refund | `refund_invoice` requires the merchant address to authorize the call. `claim_refund` requires the buyer address. | None — authorization is enforced. |
| Refund after expiry | `refund_invoice` panics with `RefundPeriodExpired` if the refund window has passed. | The refund window is configurable and must be set appropriately by the merchant. |
| Double refund | `amount_refunded` is tracked on the invoice. The contract checks that `amount_refunded + refund_amount <= amount_paid`. | None — cumulative refund tracking prevents over-refunding. |

### Fee configuration

| Threat | Mitigation | Residual risk |
|--------|------------|---------------|
| Admin sets excessively high fee | No on-chain cap on fee basis points. | Relies on honest admin assumption. An admin could set fees to 100% (10_000 bps). |
| Fee change applied immediately | `set_fee` applies changes immediately. `propose_fee` adds a time-lock delay via `PendingFee`. | If `set_fee` is used instead of `propose_fee`, fee changes are instant. Integrators should monitor fee events. |
| Per-merchant fee override abuse | `set_merchant_platform_fee` allows admin to set per-merchant fees. | Relies on honest admin. No cap on per-merchant fees. |

### Contract upgrade

| Threat | Mitigation | Residual risk |
|--------|------------|---------------|
| Admin upgrades to malicious WASM | `upgrade` requires admin auth and replaces the contract WASM immediately. | Relies on honest admin. A compromised admin can deploy arbitrary code. |

### Roles and access control

| Threat | Mitigation | Residual risk |
|--------|------------|---------------|
| Privilege escalation | `grant_role` and `revoke_role` require admin auth. Roles are checked per-function. | Relies on honest admin. Admin can grant any role to any address. |
| Role confusion | Three roles exist: `Admin`, `Manager`, `Operator`. Each function checks the specific role needed. | The boundary between `Manager` and `Operator` is not always strictly enforced in every function — some functions only check for admin. |

## Known limitations

| Limitation | Description |
|------------|-------------|
| Unbounded collections | `get_merchants`, `get_invoices`, and similar list functions return unbounded `Vec` results. For large datasets, this may exceed Soroban resource limits. |
| Oracle staleness | Fiat-denominated amounts depend on oracle prices. Stale oracle data can lead to incorrect conversions. The contract does not enforce a freshness requirement. |
| No on-chain swap execution | The `PaymentRoute::Swap` path is defined but actual swap execution depends on external DEX router contracts. The contract validates the route structure but cannot guarantee the swap completes successfully. |
| Batch size limits | `pay_invoices_batch` processes all items in a single transaction. Very large batches may exceed Soroban resource limits. |
| Admin centralisation | The admin has unilateral control over fees, token whitelisting, role assignments, and WASM upgrades. |
| Refund window | Refund eligibility is based on a time window that must be configured per invoice. If not configured, invoices may become non-refundable. |

## Pre-audit checklist

Before engaging an auditor, verify:

- [ ] All `#[contracttype]` structs match their on-chain serialization (field order, types).
- [ ] All authorization checks are present on admin-restricted functions.
- [ ] Nonce invalidation works for signed invoices.
- [ ] Reentrancy guard is active on all state-mutating functions.
- [ ] Fee calculations do not overflow for extreme values (e.g. `i128::MAX` amount).
- [ ] Oracle price conversions handle zero prices and extreme decimal values.
- [ ] Batch operations handle empty input vectors gracefully.
- [ ] All error codes are unique and map to the correct error enum.

## Responsible disclosure

If you discover a security vulnerability in the Shade Protocol contracts:

1. **Do not** open a public GitHub issue.
2. Email the security team at `security@shadeprotocol.org` with a description of the vulnerability.
3. Include proof-of-concept code or transaction simulations if possible.
4. Allow 90 days for a fix before any public disclosure.

← [Back to security](README.md)
