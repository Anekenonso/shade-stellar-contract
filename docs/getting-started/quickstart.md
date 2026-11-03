# Quickstart Tutorial: Deploy, Register a Merchant, Pay an Invoice

This guide walks you end-to-end through setting up a local development environment, deploying the Shade smart contracts, configuring tokens and platform accounts, registering a merchant, creating an invoice, and executing a payment on the Stellar network using Soroban CLI.

---

## Prerequisites

Before starting, ensure you have installed:
- **Rust & Cargo**: `rustc 1.80+` with target `wasm32v1-none`
- **Soroban / Stellar CLI**: `stellar-cli` or `soroban-cli` v21.0+
- **Docker**: For running a local standalone Stellar network (or use Stellar Testnet)

```bash
rustup target add wasm32v1-none
cargo install --locked stellar-cli --features opt
```

---

## Step 1: Configure Network & Identities

Start a local standalone node or configure Testnet RPC endpoints:

```bash
# Configure local standalone network
stellar network add \
  --global standalone \
  --rpc-url "http://localhost:8000/soroban/rpc" \
  --network-passphrase "Standalone Network ; February 2022"

# Generate test identities
stellar keys generate --global admin --network standalone
stellar keys generate --global merchant --network standalone
stellar keys generate --global payer --network standalone
stellar keys generate --global platform --network standalone

# Fund identities with native XLM (standalone auto-funds via friendbot)
stellar keys fund admin --network standalone
stellar keys fund merchant --network standalone
stellar keys fund payer --network standalone
```

Expected output:
```text
Identity 'admin' funded successfully.
Identity 'merchant' funded successfully.
Identity 'payer' funded successfully.
```

---

## Step 2: Deploy Payment Asset (Test Token)

Deploy a standard SAC or mock payment token (e.g. mock USDC):

```bash
TOKEN_ID=$(stellar contract id asset \
  --asset "USDC:$(stellar keys address admin)" \
  --network standalone)

stellar contract deploy \
  --wasm target/wasm32v1-none/release/soroban_token_contract.wasm \
  --source admin \
  --network standalone
```

Mint 10,000 units (with 7 decimals) to the payer:
```bash
stellar contract invoke \
  --id $TOKEN_ID \
  --source admin \
  --network standalone \
  -- mint \
  --to $(stellar keys address payer) \
  --amount 1000000000
```

---

## Step 3: Build & Deploy Shade Hub Contract

Build the workspace and deploy the main `shade` contract:

```bash
cargo build --target wasm32v1-none --release -p shade

SHADE_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/shade.wasm \
  --source admin \
  --network standalone)

echo "Shade Hub Contract ID: $SHADE_ID"
```

Initialize the contract with the `admin` address:
```bash
stellar contract invoke \
  --id $SHADE_ID \
  --source admin \
  --network standalone \
  -- initialize \
  --admin $(stellar keys address admin)
```

---

## Step 4: Configure Platform Accounts, Tokens, and Fees

Whitelist the accepted payment token and configure platform routing:

```bash
# 1. Add accepted payment token
stellar contract invoke \
  --id $SHADE_ID \
  --source admin \
  --network standalone \
  -- add_accepted_token \
  --admin $(stellar keys address admin) \
  --token $TOKEN_ID

# 2. Set platform fee recipient account
stellar contract invoke \
  --id $SHADE_ID \
  --source admin \
  --network standalone \
  -- set_platform_account \
  --admin $(stellar keys address admin) \
  --account $(stellar keys address platform)

# 3. Set platform transaction fee (e.g. 50 basis points = 0.5% or flat 100000 stroops)
stellar contract invoke \
  --id $SHADE_ID \
  --source admin \
  --network standalone \
  -- set_fee \
  --admin $(stellar keys address admin) \
  --token $TOKEN_ID \
  --fee 100000
```

Verify token acceptance:
```bash
stellar contract invoke \
  --id $SHADE_ID \
  --source admin \
  --network standalone \
  -- is_accepted_token \
  --token $TOKEN_ID
```
Expected output:
```text
true
```

---

## Step 5: Register Merchant

Register a new merchant profile:

```bash
stellar contract invoke \
  --id $SHADE_ID \
  --source merchant \
  --network standalone \
  -- register_merchant \
  --merchant $(stellar keys address merchant)
```

Verify merchant status:
```bash
stellar contract invoke \
  --id $SHADE_ID \
  --source merchant \
  --network standalone \
  -- get_merchant \
  --merchant_id 1
```

Expected output:
```json
{
  "id": 1,
  "address": "G...",
  "is_active": true,
  "is_verified": false,
  "registered_at": 1740000000
}
```

---

## Step 6: Create an Invoice

Create an invoice for 500 units (500000000 stroops) of the accepted token:

```bash
INVOICE_ID=$(stellar contract invoke \
  --id $SHADE_ID \
  --source merchant \
  --network standalone \
  -- create_invoice \
  --merchant $(stellar keys address merchant) \
  --description "Monthly Enterprise Cloud Subscription" \
  --amount 500000000 \
  --token $TOKEN_ID \
  --expires_at 1780000000)

echo "Created Invoice ID: $INVOICE_ID"
```

Read back the invoice details:
```bash
stellar contract invoke \
  --id $SHADE_ID \
  --source merchant \
  --network standalone \
  -- get_invoice \
  --invoice_id 1
```

Expected output:
```json
{
  "id": 1,
  "merchant_id": 1,
  "amount": 500000000,
  "token": "C...",
  "status": "Unpaid",
  "description": "Monthly Enterprise Cloud Subscription"
}
```

---

## Step 7: Pay the Invoice

As the `payer`, execute the payment:

```bash
stellar contract invoke \
  --id $SHADE_ID \
  --source payer \
  --network standalone \
  -- pay_invoice \
  --payer $(stellar keys address payer) \
  --invoice_id 1
```

Verify the invoice status has updated to `Paid`:
```bash
stellar contract invoke \
  --id $SHADE_ID \
  --source merchant \
  --network standalone \
  -- get_invoice \
  --invoice_id 1
```

Expected output:
```json
{
  "id": 1,
  "status": "Paid",
  "payer": "G...",
  "paid_at": 1740000050
}
```

---

## Troubleshooting & Common Errors

| Error Code / Symptom | Root Cause | Resolution |
| :--- | :--- | :--- |
| **`Error(Contract, #2)` (UnacceptedToken)** | The payment token has not been whitelisted by the admin. | Call `add_accepted_token` as admin with the token contract address before invoice creation or payment. |
| **`Error(Contract, #5)` (InactiveMerchant)** | Merchant account is deactivated or unverified under restrictive policy. | Call `set_merchant_status` with `status: true` as admin to activate the merchant profile. |
| **`Error(Contract, #12)` (InsufficientBalance)** | Payer wallet does not hold enough token balance (or fee amount) to cover the invoice. | Mint or fund additional tokens to the payer address prior to calling `pay_invoice`. |
| **`Error(Contract, #21)` (InvoiceExpired)** | Ledger timestamp has exceeded the `expires_at` Unix epoch. | Create a new invoice with a future expiration timestamp or pass `expires_at: null` for no expiration. |

---

## Next Steps
- Learn more about the [Workspace and Crate Layout](../architecture/workspace-layout.md).
- Explore [Cross-Contract Calls and Factory Architecture](../architecture/cross-contract-calls.md).
- Read the [Merchant Account Contract Reference](../contracts/account.md).
