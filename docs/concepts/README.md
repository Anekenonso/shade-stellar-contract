# Concepts

Deep dives into individual mechanisms, written from the [concept template](../contributing/templates/concept-template.md). Reference pages and guides link here for the "why," not just the "how."

- [Invoice lifecycle and statuses](invoice-lifecycle.md)
- [Payment flows: full, partial, and batch](payments.md) — preconditions, fee splitting, rounding, and batch atomicity for `pay_invoice`, `pay_invoice_partial`, and `pay_invoices_batch`.
- [Time, ledger timestamps, and interval semantics](time-and-intervals.md) — units, boundary comparisons, the fee time-lock, and testing time-based logic.
- Invoices, drafts, and signed invoices — *planned*.
- [Refunds and voids](refunds-and-voids.md) — full refunds, partial refunds, voids, amendments, and buyer-initiated expiry claims.
- [Fees, volume discounts, and time-locked fee changes](fees.md) — `calculate_fee`, merchant volume discounts, and the `propose_fee` / `execute_fee` timelock.
- [Merchants](merchants.md) — registration, activation, verification, configuration, and the state-to-operation matrix.
- [Merchant account withdrawals, thresholds, and approval requests](merchant-account-withdrawals.md) — direct vs approval-based withdrawals, `WithdrawalRequest` states, restriction effects (account contract).
- [Merchant analytics and transaction history](analytics-and-history.md)
- [Token analytics and market share metrics](token-analytics.md) — protocol-wide token volume, dominance, and basis-point market share.
- [Amounts, token decimals, and rounding](amounts-and-rounding.md) — base units, per-path rounding direction, remainders, and conversion helpers.
- [Auto-withdrawal and merchant settlement](auto-withdrawal.md)
- Reentrancy guard — *planned*.

← [Back to documentation home](../README.md)
