# Data types reference

Every public type in `contracts/shade/src/types.rs` — structs, enums, and their fields — so integrators can decode contract return values and build correct client-side models.

## Conventions

- **`i128` amounts:** All monetary values use `i128` in token base units (the smallest indivisible unit). For tokens with 7 decimals, `1_0000000` equals 1 whole token.
- **`u64` IDs:** Auto-incremented identifiers for merchants and invoices.
- **Timestamps:** All `u64` timestamp fields are Unix seconds since epoch.
- **`String`/`Bytes`:** Soroban strings are UTF-8 `String`; raw bytes use `Bytes` or `BytesN<N>`. Length limits are Soroban-imposed (1 MB for `Bytes`, 32 bytes for `BytesN<32>`).
- **Persisted types:** Types stored on-chain are subject to [upgrade-compatibility rules](../architecture/upgradeability.md). Changing field order or types of a persisted struct requires a storage migration.

## Enums

### `InvoiceStatus`

Lifecycle of an invoice (`contracts/shade/src/types.rs#L356-L366`).

| Variant | `u32` | Description |
|---------|-------|-------------|
| `Pending` | 0 | Created, awaiting payment. |
| `Paid` | 1 | Fully paid. |
| `Cancelled` | 2 | Voided by the merchant. |
| `Refunded` | 3 | Fully refunded after payment. |
| `PartiallyRefunded` | 4 | Some amount refunded. |
| `PartiallyPaid` | 5 | Partial payment received. |
| `Draft` | 6 | Created as a draft; not yet finalized for payment. |

### `InvoicePricingMode`

How the invoice amount is determined (`types.rs#L369-L374`).

| Variant | `u32` | Description |
|---------|-------|-------------|
| `FixedCrypto` | 0 | Amount is denominated in the payment token. |
| `FixedFiat` | 1 | Amount is denominated in fiat; converted to crypto at payment time via oracle. |

### `FiatPricingData`

Optional wrapper for fiat pricing (Soroban does not support `Option<T>` for user-defined structs in `#[contracttype]`) (`types.rs#L391-L394`).

| Variant | Description |
|---------|-------------|
| `None` | No fiat pricing. |
| `Some(FiatPricing)` | Fiat pricing details present. |

### `Role`

Access-control roles (`types.rs#L435-L439`).

| Variant | Description |
|---------|-------------|
| `Admin` | Full administrative control. |
| `Manager` | Can manage merchants and invoices. |
| `Operator` | Can process payments and refunds. |

### `PaymentRoute`

How a payment is routed (`types.rs#L644-L647`).

| Variant | Description |
|---------|-------------|
| `Direct` | Payer sends the settlement token directly. |
| `Swap(SwapRoute)` | Payment is routed through a DEX router with a swap path. |

### `TransactionType`

Type of on-chain financial transaction (`types.rs#L548-L551`).

| Variant | `u32` | Description |
|---------|-------|-------------|
| `InvoicePayment` | 0 | Payment of an invoice. |
| `SubscriptionCharge` | 1 | Recurring subscription charge. |

### `PlatformFeeRouteKind`

Category of platform fee routing (`types.rs#L667-L672`).

| Variant | `u32` | Description |
|---------|-------|-------------|
| `Invoice` | 0 | Fee from an invoice payment. |
| `TicketPurchase` | 2 | Fee from a ticket purchase. |

## Structs

### `ContractInfo`

Singleton contract metadata (`types.rs#L324-L328`).

| Field | Type | Description |
|-------|------|-------------|
| `admin` | `Address` | Current contract administrator. |
| `timestamp` | `u64` | Ledger timestamp of initialization (Unix seconds). |

### `Merchant`

Registered merchant record (`types.rs#L340-L353`).

| Field | Type | Description |
|-------|------|-------------|
| `id` | `u64` | Auto-incremented merchant ID. |
| `address` | `Address` | Merchant's Stellar address. |
| `active` | `bool` | Whether the merchant is active. |
| `verified` | `bool` | Whether the merchant has been verified by admin. |
| `date_registered` | `u64` | Unix timestamp of registration. |
| `webhook` | `String` | Webhook URL for payment notifications. |
| `auto_withdrawal_recipient` | `Option<Address>` | Optional override for auto-withdrawal destination. Defaults to merchant address. |
| `auto_withdrawal_thresholds` | `Vec<AutoWithdrawalThreshold>` | Per-token thresholds that trigger automatic withdrawal. |

### `AutoWithdrawalThreshold`

Per-token auto-withdrawal trigger (`types.rs#L332-L336`).

| Field | Type | Description |
|-------|------|-------------|
| `token` | `Address` | The token this threshold applies to. |
| `threshold` | `i128` | Balance at or above which withdrawal is triggered. |

### `Invoice`

A payable request created by a merchant (`types.rs#L398-L413`).

| Field | Type | Description |
|-------|------|-------------|
| `id` | `u64` | Auto-incremented invoice ID. |
| `description` | `String` | Human-readable description. |
| `amount` | `i128` | Invoice amount in token base units. |
| `token` | `Address` | Accepting payment token address. |
| `status` | `InvoiceStatus` | Current lifecycle status. |
| `merchant_id` | `u64` | Owning merchant's numeric ID. |
| `payer` | `Option<Address>` | Address of the payer once paid. |
| `date_created` | `u64` | Unix timestamp of creation. |
| `date_paid` | `Option<u64>` | Unix timestamp of payment. |
| `amount_paid` | `i128` | Cumulative amount paid (supports partial payments). |
| `amount_refunded` | `i128` | Cumulative amount refunded. |
| `expires_at` | `Option<u64>` | Optional expiration timestamp. |
| `pricing_mode` | `InvoicePricingMode` | Fixed crypto or fixed fiat pricing. |
| `fiat_pricing` | `FiatPricingData` | Fiat pricing details (if applicable). |

### `FiatPricing`

Fiat-denominated pricing details (`types.rs#L378-L382`).

| Field | Type | Description |
|-------|------|-------------|
| `currency` | `String` | ISO 4217 currency code (e.g. `"USD"`). |
| `amount` | `i128` | Fiat amount in minor units scaled by `decimals`. |
| `decimals` | `u32` | Number of fractional digits. |

### `MerchantFilter`

Filter for merchant queries (`types.rs#L416-L420`).

| Field | Type | Description |
|-------|------|-------------|
| `is_active` | `Option<bool>` | Filter by active status. |
| `is_verified` | `Option<bool>` | Filter by verification status. |

### `InvoiceFilter`

Filter for invoice queries (`types.rs#L423-L431`).

| Field | Type | Description |
|-------|------|-------------|
| `status` | `Option<u32>` | Filter by `InvoiceStatus` as `u32`. |
| `merchant` | `Option<Address>` | Filter by merchant address. |
| `min_amount` | `Option<u128>` | Minimum invoice amount. |
| `max_amount` | `Option<u128>` | Maximum invoice amount. |
| `start_date` | `Option<u64>` | Start of date range (Unix seconds). |
| `end_date` | `Option<u64>` | End of date range (Unix seconds). |

### `OracleConfig`

Price oracle configuration (`types.rs#L449-L454`).

| Field | Type | Description |
|-------|------|-------------|
| `contract` | `Address` | Oracle contract address. |
| `price_decimals` | `u32` | Decimal precision of the oracle's price output. |
| `token_decimals` | `u32` | Decimal precision of the token. |

### `MerchantAnalytics`

Per-merchant per-token analytics (`types.rs#L457-L465`).

| Field | Type | Description |
|-------|------|-------------|
| `merchant` | `Address` | Merchant address. |
| `token` | `Address` | Token address. |
| `total_volume` | `i128` | Cumulative volume in token base units. |
| `total_fees` | `i128` | Cumulative fees collected. |
| `transaction_count` | `u64` | Number of transactions. |
| `last_updated` | `u64` | Unix timestamp of last update. |

### `MerchantAnalyticsSummary`

Aggregate merchant analytics across all tokens (`types.rs#L468-L475`).

| Field | Type | Description |
|-------|------|-------------|
| `merchant` | `Address` | Merchant address. |
| `total_volume` | `i128` | Total volume across all tokens. |
| `total_fees` | `i128` | Total fees across all tokens. |
| `transaction_count` | `u64` | Total transaction count. |
| `last_updated` | `u64` | Unix timestamp of last update. |

### `PendingFee`

Time-locked pending fee update (`types.rs#L480-L485`).

| Field | Type | Description |
|-------|------|-------------|
| `token` | `Address` | Token the fee change applies to. |
| `fee` | `i128` | Proposed new fee in basis points. |
| `proposed_at` | `u64` | Unix timestamp when the fee was proposed. |

### `TokenAnalytics`

Aggregate analytics for a token (`types.rs#L535-L543`).

| Field | Type | Description |
|-------|------|-------------|
| `token` | `Address` | Token address. |
| `total_volume` | `i128` | Cumulative volume. |
| `total_fees` | `i128` | Cumulative fees. |
| `transaction_count` | `u64` | Number of transactions. |
| `unique_merchants` | `u64` | Number of distinct merchants. |
| `last_updated` | `u64` | Unix timestamp of last update. |

### `Transaction`

Recorded financial transaction (`types.rs#L555-L563`).

| Field | Type | Description |
|-------|------|-------------|
| `transaction_type` | `TransactionType` | Invoice payment. |
| `ref_id` | `u64` | Invoice ID. |
| `amount` | `i128` | Transaction amount in token base units. |
| `token` | `Address` | Token address. |
| `description` | `String` | Human-readable description. |
| `date` | `u64` | Unix timestamp. |
| `merchant_id` | `u64` | Owning merchant's numeric ID. |

### `SwapRoute`

DEX swap route details (`types.rs#L650-L654`).

| Field | Type | Description |
|-------|------|-------------|
| `router` | `Address` | DEX router contract address. |
| `path` | `Vec<Address>` | Ordered list of token addresses for the swap path. |

### `PlatformFeeSplit`

Computed fee split for a payment (`types.rs#L658-L663`).

| Field | Type | Description |
|-------|------|-------------|
| `gross_amount` | `i128` | Total payment amount. |
| `platform_fee` | `i128` | Fee amount deducted for the platform. |
| `merchant_amount` | `i128` | Amount credited to the merchant. |
| `fee_bps_applied` | `i128` | Effective fee rate in basis points. |

### `PaymentPayload`

Payment routing parameters (`types.rs#L676-L681`).

| Field | Type | Description |
|-------|------|-------------|
| `input_token` | `Address` | Token the payer sends. |
| `settlement_token` | `Address` | Token the merchant receives. |
| `route` | `PaymentRoute` | Direct or swap routing. |
| `max_slippage_bps` | `Option<u32>` | Maximum acceptable slippage for swap routes. |

### `PlatformFeeSplit`

Computed fee split (also documented above under Structs).

### `PageInfo`

Keyset pagination cursor (`types.rs#L1539-L1547`).

| Field | Type | Description |
|-------|------|-------------|
| `count` | `u32` | Items returned in this page. |
| `next_cursor` | `u64` | ID of the last item; 0 = no more pages. |
| `has_next_page` | `bool` | Whether more pages exist. |

### `InvoicePage`

Paginated invoice results (`types.rs#L1551-L1555`).

| Field | Type | Description |
|-------|------|-------------|
| `items` | `Vec<Invoice>` | Invoice records in this page. |
| `page_info` | `PageInfo` | Pagination metadata. |

### `MerchantPage`

Paginated merchant results (`types.rs#L1559-L1563`).

| Field | Type | Description |
|-------|------|-------------|
| `items` | `Vec<Merchant>` | Merchant records in this page. |
| `page_info` | `PageInfo` | Pagination metadata. |

