# Frequently asked questions

Short answers to the questions that come up most often in issues and pull requests, for contributors, integrators, and operators. Each answer links to the page that covers the topic in depth. If you are hitting an error rather than asking a question, go to [Troubleshooting](troubleshooting.md).

## About the protocol

### What is Shade, and what is it not?

Shade is a payment protocol implemented as Soroban smart contracts. A [merchant](../glossary.md#merchant) registers, issues [invoices](../glossary.md#invoice), and gets paid in a whitelisted token, with the protocol fee split out in the same transaction. The commerce primitives on top of that core are subscriptions, event ticketing, escrow, and crowdfunding campaigns.

Shade is **not**:

- a wallet or key manager — payers and merchants bring their own Stellar accounts;
- a fiat on-ramp or off-ramp — it can *price* an invoice in fiat through an oracle, but settlement is always in a token;
- a token swapper — a payment payload can *describe* a swap route, but the contract does not execute the swap;
- a token issuer — it settles in tokens issued elsewhere;
- a chargeback system — there is no unilateral reversal of a settled payment.

The full list is in [Introduction § Non-goals](introduction.md#non-goals).

### Which tokens are supported?

Any [SEP-41](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md) token contract, including the Stellar Asset Contract (SAC) of a classic asset such as XLM or a stablecoin, **once the [admin](../glossary.md#admin) has whitelisted it** with `add_accepted_token` or `add_accepted_tokens`. There is no hardcoded token list in the contracts. Two checks apply to every invoice:

1. The token must be on the global list — check with `is_accepted_token(token)`. Failing this raises `TokenNotAccepted` (code 12).
2. The token must also be in the merchant's own accepted-tokens list, when the merchant has narrowed it with `set_merchant_accepted_tokens`. Failing this raises `TokenNotAcceptedByMerchant` (code 41).

See [Token whitelisting](../concepts/token-whitelisting.md).

### Which networks can I use?

The contracts are plain Soroban WASM, so they run on any network that supports Soroban: a local network, testnet, futurenet, and mainnet. The repository's [`Makefile`](../../Makefile) targets `testnet` and `futurenet` through its `NETWORK` variable. The repository does not publish canonical contract IDs — you deploy your own instance. Follow the [Deployment Runbook](../operations/deployment.md), and read its mainnet section before deploying to a public network.

Use the Stellar CLI 23.x line to match the pinned `soroban-sdk` — see [Prerequisites § Version compatibility](prerequisites.md#version-compatibility-with-soroban-sdk).

### Is there a hosted testnet deployment I can point my app at?

Not from this repository. Deploy your own `shade` instance to testnet or a local network, `initialize` it, and whitelist a test token. [Quickstart](quickstart.md) walks through it, and the [Deployment Runbook](../operations/deployment.md) lists the post-deployment configuration steps.

## Fees and money movement

### What does Shade charge?

A [protocol fee](../glossary.md#protocol-fee) in basis points (`10_000` bps = 100%) on the gross payment amount, configured per token with `set_fee`. A token with no fee configured charges nothing. The fee is **deducted from** the invoice amount, not added to it: the payer is debited `amount`, and the merchant account receives `amount − platform_fee`.

The effective rate can be lower than the configured one — a per-merchant override (`set_merchant_platform_fee`) replaces the base rate, and cumulative volume discounts of 10%, 25%, and 50% apply at 10,000, 50,000, and 200,000 units of volume. Use `calculate_fee(merchant, token, amount)` to preview the fee. The fee rounds down. See [Fees](../concepts/fees.md).

### Can the admin change fees without warning?

`set_fee` takes effect immediately. The time-locked path is `propose_fee` followed by `execute_fee` after a 48-hour delay (`FEE_UPDATE_DELAY` = `172_800` seconds). Calling `execute_fee` early raises `FeeUpdateTooEarly` (code 42). Integrators who want advance notice should watch for the fee-proposed event — see [Event indexing](../guides/indexing-events.md).

### Who holds the money?

For invoice payments, nobody in the middle. The `shade` contract never receives the funds: it computes the split and instructs the token contract to transfer from the payer to the merchant's account and to the platform account in one transaction. The merchant's balance sits in an account the merchant controls, and a withdrawal needs the merchant's own authorization.

The admin **cannot** move a merchant's funds. Features that are designed to hold a balance until a condition is met — escrow, crowdfunding campaigns, and subscription allowances — hold or pull funds under the rules of their own components; read the matching concept page before relying on them. See [Escrow](../concepts/escrow.md).

### Do I need to deploy an account contract for every merchant?

No. `register_merchant` sets the merchant's settlement `account` to the merchant's own wallet address, which is enough to receive payments. Deploy the `account` contract and link it with `set_merchant_account` only if you want an on-chain vault with withdrawal thresholds, restriction flags, or auto-withdrawal. `register_merchant` does not deploy an account contract itself, even if `set_account_wasm_hash` has been called. See [Merchants](../concepts/merchants.md) and [Account contract](../contracts/account.md).

### Can a payment be reversed?

Not unilaterally. A paid invoice can be refunded by the merchant with `refund_invoice` (full) or `refund_invoice_partial`, and only within the refund window of 7 days (`MAX_REFUND_DURATION` = `604_800` seconds) measured from `date_paid`. After that, `refund_invoice` raises `RefundPeriodExpired` (code 17). The refund is paid out of the merchant's account balance, so a merchant account without enough funds raises `InsufficientBalance` (code 30). See [Refunds and voids](../concepts/refunds-and-voids.md).

### Can I price an invoice in USD?

Yes, with `create_fiat_invoice`. The invoice stores a fiat amount and currency, and the token amount is re-derived from the token's oracle price at payment time until the first payment lands. The admin must register an oracle for the token with `set_token_oracle`, and the oracle contract must expose `get_price(token, quote_currency) -> i128`. Without a configured oracle, the call raises `OracleNotConfigured` (code 34); a zero or negative price raises `OraclePriceUnavailable` (code 35). See [Fiat pricing and oracles](../concepts/fiat-pricing-and-oracles.md).

## Control, pausing, and upgrades

### Who can pause the contract?

Only the admin, with `pause(admin)`; `unpause(admin)` reverses it. A pause blocks every state-changing function that checks the pause flag, and those calls fail with `ContractPaused` (code 9). Read-only getters keep working. `upgrade` is deliberately **not** blocked by a pause, so an operator can ship a fix while paused. See [Pausable](../security/pausable.md) and the [Admin Runbook § Pause procedure](../operations/admin-runbook.md#pause-procedure).

### Who can upgrade the contract?

There are two routes to the same host call, `update_current_contract_wasm`:

- **Admin route.** `upgrade(new_wasm_hash)` takes no admin argument — it reads the admin from storage and requires that address's authorization. It is the emergency lever.
- **Governance route.** A council member calls `propose_upgrade`, members cast `vote_on_upgrade`, and anyone calls `finalize_upgrade` once voting closes and quorum is met.

> **Security:** The admin route needs one signature and no vote. Treat the admin key as the protocol's root of trust and read [Admin and ownership](../security/admin-and-ownership.md) before choosing who holds it.

An upgrade replaces code but keeps storage, so the new WASM must read the old layout correctly — see [Upgradeability](../architecture/upgradeability.md) and the [Upgrade Runbook](../operations/upgrade-runbook.md).

### Can the admin be changed?

Yes, through a two-step transfer: the current admin calls `propose_admin_transfer(admin, new_admin)`, then the new admin calls `accept_admin_transfer(new_admin)`. The admin does not change until the second call. See [Admin Runbook § Admin transfer](../operations/admin-runbook.md#admin-transfer).

### Does a merchant need approval to register?

No. `register_merchant` is permissionless and self-authorized. A new merchant starts `active: true` and `verified: false`; verification (`verify_merchant`) and activation (`set_merchant_status`) are admin actions, and some features require them. Registering twice raises `MerchantAlreadyRegistered` (code 5).

## Development

### Why are there no `test_snapshots/` files in git?

The SDK writes a JSON snapshot for every `Env` when a test finishes, and this repository ignores the whole directory in [`.gitignore`](../../.gitignore). Behavior is asserted explicitly in test bodies instead of through committed snapshots. A `test_snapshots/` directory appearing after `cargo test` is expected; leave it unstaged. See [Running tests § Test snapshots](running-tests.md#test-snapshots).

### How do I find out what a contract error code means?

Pair the number with the contract that raised it, then look it up in the [Errors reference](../reference/errors.md). Codes are scoped per contract, and `shade` partitions its enums into non-overlapping ranges (`1–99`, `100–119`, `120–139`, and so on), so a code observed against the `shade` contract ID maps to exactly one variant.

### Where do I ask a question or report a problem?

Search the [issue tracker](https://github.com/ShadeProtocol/shade-stellar-contract/issues) first, then follow [Troubleshooting § Still stuck?](troubleshooting.md#still-stuck). To contribute a fix, start with [CONTRIBUTING.md](../../CONTRIBUTING.md).

← [Back to Getting Started](README.md)
