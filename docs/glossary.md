# Protocol Glossary

Alphabetised definitions of the domain and Soroban terms used across Shade Protocol documentation and source. Link to a term here on its first use in any page — see the [style guide](contributing/documentation-style-guide.md#terminology).

## Domain terms

### Accepted Token

A token address the protocol admin has whitelisted for payments. Invoices may only settle in an accepted token.

- Storage: `DataKey::AcceptedTokens` — [`contracts/shade/src/types.rs:40`](../contracts/shade/src/types.rs)
- Interface: `add_accepted_token`, `add_accepted_tokens`, `remove_accepted_token`, `is_accepted_token` — [`contracts/shade/src/interface.rs:27-30`](../contracts/shade/src/interface.rs)

### Admin

The single address with contract-wide administrative authority: whitelisting tokens, setting fees, verifying merchants, pausing the contract, and more. Transferred via a two-step propose/accept handover rather than a direct write.

- Storage: `DataKey::Admin`, `DataKey::PendingAdmin` — [`contracts/shade/src/types.rs:35-36`](../contracts/shade/src/types.rs)
- Interface: `get_admin`, `propose_admin_transfer`, `accept_admin_transfer` — [`contracts/shade/src/interface.rs:163-166`](../contracts/shade/src/interface.rs)

### DataKey

The storage-key enum (admin, merchants, invoices, fees, analytics). Soroban caps every `#[contracttype]` enum at 50 cases.

- Definition and rationale: [`contracts/shade/src/types.rs:1-84`](../contracts/shade/src/types.rs)
- Concept: `docs/architecture/` storage layout page (planned)

### Draft Invoice

An invoice created in `InvoiceStatus::Draft` state via `create_invoice_draft`, editable by the merchant before it is finalized and made payable by `finalize_invoice`.

- Interface: `create_invoice_draft`, `finalize_invoice` — [`contracts/shade/src/interface.rs:67-75`](../contracts/shade/src/interface.rs)
- Type: `InvoiceStatus::Draft` — [`contracts/shade/src/types.rs:358-366`](../contracts/shade/src/types.rs)

### Fiat Pricing

An invoice denominated in a fiat currency (e.g. USD) rather than raw token units, valued against an oracle price at the moment of the relevant action.

- Type: `FiatPricing`, `FiatPricingData` — [`contracts/shade/src/types.rs`](../contracts/shade/src/types.rs)
- Interface: `create_fiat_invoice` — [`contracts/shade/src/interface.rs:57-66`](../contracts/shade/src/interface.rs)
- Concept: `docs/concepts/` fiat-pegged goals page (planned)

### Invoice

A payable request created by a merchant for a fixed amount (crypto or fiat-pegged) in an accepted token, tracked through its [Invoice Status](#invoice-status) lifecycle.

- Type: `Invoice` — [`contracts/shade/src/types.rs:396-413`](../contracts/shade/src/types.rs)
- Interface: `create_invoice`, `get_invoice`, `pay_invoice` — [`contracts/shade/src/interface.rs:49-56,87,139`](../contracts/shade/src/interface.rs)

### Invoice Status

The lifecycle state of an invoice: `Pending`, `Paid`, `Cancelled`, `Refunded`, `PartiallyRefunded`, `PartiallyPaid`, or `Draft`.

- Type: `InvoiceStatus` — [`contracts/shade/src/types.rs:355-366`](../contracts/shade/src/types.rs)

### Merchant

A registered business or individual that can create invoices and receive payments. Identified both by wallet address and by a numeric merchant ID.

- Type: `Merchant` — [`contracts/shade/src/types.rs:338-353`](../contracts/shade/src/types.rs)
- Interface: `register_merchant`, `get_merchant`, `is_merchant` — [`contracts/shade/src/interface.rs:41-44`](../contracts/shade/src/interface.rs)

### Merchant Account

The per-merchant `account` contract instance (deployed from the `account` crate) that holds and disburses a merchant's funds, distinct from the merchant's own wallet address.

- Storage: `DataKey::MerchantAccount(u64)` — [`contracts/shade/src/types.rs:62`](../contracts/shade/src/types.rs)
- Interface: `deploy_merchant_account`, `get_merchant_account` — [`contracts/shade/src/interface.rs:132-133`](../contracts/shade/src/interface.rs)
- Related crate: [`contracts/account/`](../contracts/account)

### Merchant Key

A 32-byte key a merchant registers to verify signed invoices and webhook payloads, distinct from their on-chain address.

- Storage: `DataKey::MerchantKey(Address)` — [`contracts/shade/src/types.rs:57`](../contracts/shade/src/types.rs)
- Interface: `set_merchant_key`, `get_merchant_key` — [`contracts/shade/src/interface.rs:91-92`](../contracts/shade/src/interface.rs)

### Oracle Config

The price-feed contract and decimal configuration Shade uses to convert between a token's base units and a fiat currency.

- Type: `OracleConfig` — [`contracts/shade/src/types.rs:448-454`](../contracts/shade/src/types.rs)
- Interface: `set_token_oracle`, `get_token_oracle` — [`contracts/shade/src/interface.rs:36-37`](../contracts/shade/src/interface.rs)

### Payment Payload

Describes how an incoming payment should be routed and settled: the input token, the settlement token, a direct or swap `PaymentRoute`, and an optional maximum slippage in basis points.

- Type: `PaymentPayload`, `PaymentRoute` — [`contracts/shade/src/types.rs:642-681`](../contracts/shade/src/types.rs)
- Interface: `validate_payment_payload` — [`contracts/shade/src/interface.rs:142`](../contracts/shade/src/interface.rs)

### Payer

The address paying an invoice. Recorded on the `Invoice` as `payer` once payment lands. See also [Merchant](#merchant) (the receiving party).

- Type: `Invoice::payer` — [`contracts/shade/src/types.rs:405`](../contracts/shade/src/types.rs)
- Interface: `pay_invoice`, `pay_invoices_batch`, `pay_invoice_partial` — [`contracts/shade/src/interface.rs:139-141`](../contracts/shade/src/interface.rs)

### Pending Fee (time-locked)

A proposed fee change for a token that must be explicitly executed after being proposed, giving merchants advance notice before a new fee takes effect.

- Type: `PendingFee` — [`contracts/shade/src/types.rs:479-485`](../contracts/shade/src/types.rs)
- Storage: `DataKey::PendingTokenFee(Address)` — [`contracts/shade/src/types.rs:51`](../contracts/shade/src/types.rs)
- Interface: `propose_fee`, `execute_fee`, `get_pending_fee` — [`contracts/shade/src/interface.rs:38-40`](../contracts/shade/src/interface.rs)

### Platform Account

The protocol-level address that receives platform fees split out of merchant payments.

- Storage: `DataKey::PlatformAccount` — [`contracts/shade/src/types.rs:43`](../contracts/shade/src/types.rs)
- Interface: `set_platform_account`, `get_platform_account` — [`contracts/shade/src/interface.rs:34-35`](../contracts/shade/src/interface.rs)

### Protocol Fee

The fee (in basis points or a flat amount) the protocol takes from a payment before crediting the merchant, computed per token and optionally overridden per merchant.

- Storage: `DataKey::FeeInBasisPoints`, `DataKey::FeeAmount`, `DataKey::TokenFee`, `DataKey::MerchantPlatformFee` — [`contracts/shade/src/types.rs:47-53`](../contracts/shade/src/types.rs)
- Type: `PlatformFeeSplit` — [`contracts/shade/src/types.rs:657-663`](../contracts/shade/src/types.rs)
- Interface: `calculate_fee`, `compute_platform_fee_split` — [`contracts/shade/src/interface.rs:108-114`](../contracts/shade/src/interface.rs)

### Reentrancy Guard

A storage-flag guard (`DataKey::ReentrancyStatus`) that a fund-moving function sets on entry and clears on exit, panicking if it's already set — preventing a call from re-entering itself via a token contract callback.

- Storage: `DataKey::ReentrancyStatus` — [`contracts/shade/src/types.rs:41`](../contracts/shade/src/types.rs)
- Implementation: [`contracts/shade/src/components/reentrancy.rs`](../contracts/shade/src/components/reentrancy.rs)
- Concept: `docs/security/` reentrancy page (planned)

### Role

An access-control tier grantable to any address beyond the single `Admin`: `Manager` or `Operator`, each authorized for a narrower set of privileged calls.

- Type: `Role` — [`contracts/shade/src/types.rs:433-439`](../contracts/shade/src/types.rs)
- Storage: `DataKey::Role(Address, Role)` — [`contracts/shade/src/types.rs:44`](../contracts/shade/src/types.rs)
- Interface: `grant_role`, `revoke_role`, `has_role` — [`contracts/shade/src/interface.rs:93-95`](../contracts/shade/src/interface.rs)

### Signed Invoice

An invoice created off-chain by a merchant and submitted on-chain with an Ed25519 signature and a single-use nonce, letting a relayer submit the transaction on the merchant's behalf without the merchant needing to sign the Soroban transaction itself.

- Interface: `create_invoice_signed` — [`contracts/shade/src/interface.rs:76-86`](../contracts/shade/src/interface.rs)
- Storage: `DataKey::UsedNonce(Address, BytesN<32>)` — [`contracts/shade/src/types.rs:45`](../contracts/shade/src/types.rs)

### Swap Route

A `PaymentRoute::Swap` variant naming the router contract and token path a payment should be swapped through before settlement, as opposed to `PaymentRoute::Direct`.

- Type: `SwapRoute`, `PaymentRoute` — [`contracts/shade/src/types.rs:642-654`](../contracts/shade/src/types.rs)

## Stellar / Soroban terms

These are Stellar-platform concepts referenced throughout the docs but defined by Stellar itself, not by this repository. See the linked official docs for authoritative detail.

### Contract ID

The address a Soroban smart contract is deployed at and invoked through. See [Stellar docs: Contract Interactions](https://developers.stellar.org/docs/build/guides/conventions).

### Event

An on-chain record published for indexers and clients to observe (e.g. a merchant registration, an invoice payment). See [`contracts/shade/src/events.rs`](../contracts/shade/src/events.rs).

### Host Function

A function the Soroban runtime exposes to a contract (storage access, cryptography, auth checks) rather than one the contract implements itself. See [Stellar docs: Soroban environment](https://developers.stellar.org/docs/learn/fundamentals/contract-development/environment-concepts).

### Instance / Persistent / Temporary Storage

The three storage durability tiers Soroban offers: instance storage (small, tied to the contract instance's own lifetime), persistent storage (long-lived, rent-paying, used for most Shade records), and temporary storage (cheap, expires quickly, unsuitable for anything that must survive). See [Stellar docs: State archival](https://developers.stellar.org/docs/learn/encyclopedia/storage/state-archival).

### require_auth

The Soroban host call a contract uses to assert that a given `Address` has authorized the current invocation, either directly or via a signed authorization entry. Used throughout Shade wherever a call must be attributable to a specific merchant, admin, or signer (e.g. `merchant.require_auth()`).

### Soroban

The smart contract platform built into the Stellar network that Shade's contracts are written for and deployed to. See [Stellar docs: Soroban overview](https://developers.stellar.org/docs/learn/fundamentals/contract-development/overview).

### TTL / Rent

The time-to-live mechanism by which Soroban charges "rent" to keep persistent (and temporary) storage entries alive, and archives entries whose TTL expires. Several comments in `contracts/shade/src/types.rs` (e.g. around `AnalyticsKey` and `CreatorVesting`) explicitly trade off extra fields against extra storage reads because of this cost — see [`contracts/shade/src/types.rs:234-258`](../contracts/shade/src/types.rs). See [Stellar docs: State archival](https://developers.stellar.org/docs/learn/encyclopedia/storage/state-archival) for the underlying mechanism.

### WASM Hash

The hash identifying a specific compiled contract WASM binary, used both to install/deploy new merchant `account` instances (`DataKey::AccountWasmHash`) and to identify the target of a governance-approved contract upgrade (`UpgradeProposal::wasm_hash`).

- Storage: `DataKey::AccountWasmHash` — [`contracts/shade/src/types.rs:42`](../contracts/shade/src/types.rs)
- Interface: `set_account_wasm_hash`, `propose_upgrade` — [`contracts/shade/src/interface.rs:31,264`](../contracts/shade/src/interface.rs)
