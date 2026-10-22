# How to test your integration against Shade locally

How to stand up the `shade` and `account` contracts inside your own crate's tests and on a local Stellar network, so your contracts and backend can be tested end to end without touching testnet.

There are two environments, and most teams use both:

| | In-process (`soroban-sdk` test `Env`) | Local network (`stellar container`) |
|---|---|---|
| Best for | Your own Soroban contracts that call Shade; fast, deterministic unit and integration tests | Backends, indexers, and wallets that talk to Soroban RPC |
| Speed | Milliseconds per test | Seconds per ledger |
| Time and auth | Fully controllable (`env.ledger()`, `mock_all_auths`) | Real ledger close times, real signatures |
| Needs Docker | No | Yes |
| Uses | The compiled WASMs, imported with `contractimport!` | The same compiled WASMs, deployed with the CLI |

## Prerequisites

- The toolchain from [Prerequisites](../getting-started/prerequisites.md): Rust 1.84.0 or newer, the `wasm32-unknown-unknown` target, and Stellar CLI 23.x.
- A checkout of this repository, to build the WASMs from.
- Docker, for the [local network](#part-2-test-against-a-local-network) section only.
- A [merchant](../glossary.md#merchant) and [payer](../glossary.md#payer) flow in mind — this guide reuses the registration, invoice, and payment calls from [Accepting payments](accepting-payments.md).

## Part 1: Test in-process with the Soroban test environment

### 1. Build the WASMs and vendor them into your crate

Build and optimize `shade` and `account` from this repository, then copy them into your crate. Pinning the files in your repository means your tests run against a known Shade version rather than whatever happens to be in a sibling checkout.

```bash
cargo build --target wasm32-unknown-unknown --release -p shade -p account
stellar contract optimize --wasm target/wasm32-unknown-unknown/release/shade.wasm
stellar contract optimize --wasm target/wasm32-unknown-unknown/release/account.wasm

mkdir -p ../my-integration/wasm
cp target/wasm32-unknown-unknown/release/shade.optimized.wasm   ../my-integration/wasm/shade.wasm
cp target/wasm32-unknown-unknown/release/account.optimized.wasm ../my-integration/wasm/account.wasm
```

See [Building the Contracts](../getting-started/building.md) for the build profiles. Record the SHA-256 of each file (`shasum -a 256 wasm/*.wasm`) next to the Shade commit or release tag they came from, so a failing test can be traced to an exact contract version.

### 2. Add the dependencies

Use the same `soroban-sdk` line the Shade workspace pins (`23.4.0`, resolved to `23.5.3` in this repository's `Cargo.lock`). A different major line compiles against a different host and fails at registration time — see [Troubleshooting § Soroban SDK version mismatch](../getting-started/troubleshooting.md#3-soroban-sdk-version-mismatch).

```toml
# my-integration/Cargo.toml
[package]
name = "my-integration"
version = "0.0.0"
edition = "2021"
publish = false

[lib]
crate-type = ["lib", "cdylib"]
doctest = false

[dependencies]
soroban-sdk = "23.4.0"

[dev-dependencies]
soroban-sdk = { version = "23.4.0", features = ["testutils"] }
```

The `testutils` feature is required for `Address::generate`, `mock_all_auths`, `env.ledger()`, and `env.events()`. Keep it in `[dev-dependencies]` so it never reaches your deployable WASM.

### 3. Write the reusable setup helper

`contractimport!` reads a compiled WASM at build time and generates a typed client, the contract's types, and a `WASM` constant holding its bytes. The path is relative to your crate's `Cargo.toml`.

Put the helper in `tests/common/mod.rs` so every integration test file can share it with `mod common;`:

```rust
// my-integration/tests/common/mod.rs
#![allow(dead_code)]

use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::{token, Address, Env, Map, Symbol, TryIntoVal, Val};

pub mod shade_wasm {
    soroban_sdk::contractimport!(file = "wasm/shade.wasm");
}

pub mod account_wasm {
    soroban_sdk::contractimport!(file = "wasm/account.wasm");
}

/// Platform fee configured for the test token: 500 bps = 5%.
pub const FEE_BPS: i128 = 500;

/// Everything an integration test usually needs, already wired together.
pub struct Fixture {
    pub env: Env,
    pub shade: shade_wasm::Client<'static>,
    pub shade_id: Address,
    pub account: account_wasm::Client<'static>,
    pub admin: Address,
    pub platform: Address,
    pub merchant: Address,
    pub merchant_id: u64,
    pub payer: Address,
    pub token: Address,
}

impl Fixture {
    /// Deploys and initializes the hub, whitelists a Stellar Asset Contract
    /// token, registers a merchant with a real account contract, and funds a
    /// payer. All authorization is mocked.
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // 1. Register the hub from its compiled WASM and initialize it.
        let shade_id = env.register(shade_wasm::WASM, ());
        let shade = shade_wasm::Client::new(&env, &shade_id);
        let admin = Address::generate(&env);
        shade.initialize(&admin);

        // 2. Send protocol fees to a dedicated address, so fee assertions do
        //    not mix with the admin's balance.
        let platform = Address::generate(&env);
        shade.set_platform_account(&admin, &platform);

        // 3. Create a test token (a Stellar Asset Contract), whitelist it, and
        //    configure its fee.
        let token_issuer = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(token_issuer).address();
        shade.add_accepted_token(&admin, &token);
        shade.set_fee(&admin, &token, &FEE_BPS);

        // 4. Upload the account WASM and record its hash on the hub.
        let account_hash = env.deployer().upload_contract_wasm(account_wasm::WASM);
        shade.set_account_wasm_hash(&admin, &account_hash);

        // 5. Register a merchant, then deploy and link its account contract.
        let merchant = Address::generate(&env);
        shade.register_merchant(&merchant);
        let merchant_id = shade.find_merchant_id(&merchant);

        let account_id = env.register(account_wasm::WASM, ());
        let account = account_wasm::Client::new(&env, &account_id);
        account.initialize(&merchant, &shade_id, &merchant_id);
        shade.set_merchant_account(&merchant, &account_id);

        // 6. Fund a payer.
        let payer = Address::generate(&env);
        token::StellarAssetClient::new(&env, &token).mint(&payer, &1_000_000_000);

        Fixture {
            env,
            shade,
            shade_id,
            account,
            admin,
            platform,
            merchant,
            merchant_id,
            payer,
            token,
        }
    }

    /// A SEP-41 client for the test token, for balance assertions.
    pub fn token(&self) -> token::Client<'_> {
        token::Client::new(&self.env, &self.token)
    }
}

/// Returns the data map of every event with topic `name` emitted by
/// `contract`, oldest first.
///
/// `#[contractevent]` derives the topic from the struct name in snake case, so
/// `InvoicePaidEvent` is published under `invoice_paid_event`. The payload is a
/// `Map<Symbol, Val>` keyed by the struct's field names.
pub fn events_named(env: &Env, contract: &Address, name: &str) -> std::vec::Vec<Map<Symbol, Val>> {
    let wanted = Symbol::new(env, name);
    let mut found = std::vec::Vec::new();

    for (emitter, topics, data) in env.events().all().iter() {
        if &emitter != contract {
            continue;
        }
        let Some(first) = topics.get(0) else { continue };
        let Ok(topic) = TryIntoVal::<Env, Symbol>::try_into_val(&first, env) else {
            continue;
        };
        if topic != wanted {
            continue;
        }
        if let Ok(map) = TryIntoVal::<Env, Map<Symbol, Val>>::try_into_val(&data, env) {
            found.push(map);
        }
    }
    found
}
```

What each step does and why it is there:

- **`env.register(shade_wasm::WASM, ())`** deploys the compiled hub into the test ledger. `shade` has no constructor, so the constructor arguments are `()`; you call `initialize` afterwards. `initialize` can run only once per instance and raises `AlreadyInitialized` (code 2) on a second call.
- **`set_platform_account`** redirects fees. `initialize` defaults the platform account to the admin address.
- **`register_stellar_asset_contract_v2`** creates a real token contract without a trustline or an issuing account to manage. See [Token whitelisting](../concepts/token-whitelisting.md) for why `add_accepted_token` is required before any invoice in that token can be created.
- **`upload_contract_wasm` and `set_account_wasm_hash`** record the account WASM hash on the hub. `register_merchant` does **not** deploy an account contract from this hash today — the merchant's settlement account defaults to the merchant's own address — so step 5 deploys and links an account contract explicitly. Keep step 4 so your fixture matches a production deployment that has the hash configured.
- **`account.initialize(&merchant, &shade_id, &merchant_id)`** binds the account to its merchant and to the hub as manager. Its signature is `initialize(env, merchant, manager, merchant_id)`.
- **`mock_all_auths`** approves every `require_auth` in the test. It is the right default for exercising flows; use `mock_auths` when authorization is the thing under test. See [Running tests § `mock_all_auths` versus explicit auth](../getting-started/running-tests.md#mock_all_auths-versus-explicit-auth).

> **Note:** The helper follows the registration, token, and event patterns used by this repository's own tests — [`test_payment.rs`](../../contracts/shade/src/tests/test_payment.rs), [`test_auto_withdrawal.rs`](../../contracts/shade/src/tests/test_auto_withdrawal.rs), and [`test_fiat_pricing.rs`](../../contracts/shade/src/tests/test_fiat_pricing.rs) — but it targets an imported WASM rather than the in-crate `Shade` type. Run `cargo test` in your crate once after copying it in, and adjust the `wasm/` paths if your layout differs.

### 4. Write a first test

```rust
// my-integration/tests/pay_invoice.rs
mod common;

use common::{shade_wasm, Fixture, FEE_BPS};
use soroban_sdk::{testutils::Ledger as _, String};

#[test]
fn payer_settles_an_invoice_and_the_fee_is_split() {
    let f = Fixture::new();

    let amount: i128 = 1_000_000;
    let invoice_id = f.shade.create_invoice(
        &f.merchant,
        &String::from_str(&f.env, "order #1"),
        &amount,
        &f.token,
        &None,
    );

    f.shade.pay_invoice(&f.payer, &invoice_id);

    let invoice = f.shade.get_invoice(&invoice_id);
    assert_eq!(invoice.status, shade_wasm::InvoiceStatus::Paid);
    assert_eq!(invoice.amount_paid, amount);

    // The fee is deducted from the gross amount and rounds down.
    let fee = amount * FEE_BPS / 10_000;
    assert_eq!(f.token().balance(&f.platform), fee);
    assert_eq!(f.token().balance(&f.account.address), amount - fee);
    assert_eq!(f.token().balance(&f.payer), 1_000_000_000 - amount);
}

#[test]
fn an_expired_invoice_cannot_be_paid() {
    let f = Fixture::new();

    let expires_at = f.env.ledger().timestamp() + 100;
    let invoice_id = f.shade.create_invoice(
        &f.merchant,
        &String::from_str(&f.env, "short-lived"),
        &1_000_000,
        &f.token,
        &Some(expires_at),
    );

    f.env.ledger().set_timestamp(expires_at + 1);

    // InvoiceExpired is code 27 in the shade errors reference.
    let result = f.shade.try_pay_invoice(&f.payer, &invoice_id);
    let expected = soroban_sdk::Error::from_contract_error(27);
    assert!(matches!(result, Err(Ok(err)) if err == expected));
}
```

The first test shows the shape of every payment assertion. The second shows the `try_*` form: every generated client method has a `try_` twin that returns a `Result` instead of trapping, so you can assert on the specific [error code](../reference/errors.md) and keep going. Name codes with a constant in your own helper rather than repeating the number.

### 5. Mock an oracle for fiat-priced invoices

`create_fiat_invoice` and `resolve_invoice_amount` call a foreign oracle contract. Shade defines the interface it expects — `get_price(token: Address, quote_currency: String) -> i128` — in [`contracts/shade/src/components/invoice.rs`](../../contracts/shade/src/components/invoice.rs). Any contract that exposes that function works, so a mock is a few lines of Soroban code in your test crate:

```rust
// my-integration/tests/fiat_invoice.rs
mod common;

use common::{shade_wasm, Fixture};
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String};

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    /// Fiat price of one whole token, scaled by the oracle's `price_decimals`.
    pub fn get_price(env: Env, _token: Address, _quote_currency: String) -> i128 {
        env.storage()
            .instance()
            .get(&symbol_short!("price"))
            .unwrap_or(100_000_000)
    }

    pub fn set_price(env: Env, price: i128) {
        env.storage().instance().set(&symbol_short!("price"), &price);
    }
}

#[test]
fn fiat_invoice_requotes_until_the_first_payment() {
    let f = Fixture::new();

    let oracle_id = f.env.register(MockOracle, ());
    let oracle = MockOracleClient::new(&f.env, &oracle_id);
    oracle.set_price(&200_000_000); // $2.00 with 8 price decimals

    f.shade.set_token_oracle(
        &f.admin,
        &f.token,
        &shade_wasm::OracleConfig {
            contract: oracle_id,
            price_decimals: 8,
            token_decimals: 7,
        },
    );

    // $10.00: 1000 with 2 fiat decimals.
    let invoice_id = f.shade.create_fiat_invoice(
        &f.merchant,
        &String::from_str(&f.env, "fiat order"),
        &1000,
        &String::from_str(&f.env, "USD"),
        &2,
        &f.token,
        &None,
    );

    // 1000 * 10^7 * 10^8 / (200_000_000 * 10^2) = 50_000_000 (5 tokens).
    assert_eq!(f.shade.resolve_invoice_amount(&invoice_id), 50_000_000);

    // The price moves; with no payment yet, the amount is re-derived.
    oracle.set_price(&500_000_000); // $5.00
    assert_eq!(f.shade.resolve_invoice_amount(&invoice_id), 20_000_000);
}
```

The conversion is `fiat_amount × 10^token_decimals × 10^price_decimals ÷ (price × 10^fiat_decimals)`; see [Fiat pricing and oracles](../concepts/fiat-pricing-and-oracles.md) for when the amount re-quotes and when it freezes. To test the failure paths, set the price to `0` and expect `OraclePriceUnavailable` (code 35), or skip `set_token_oracle` and expect `OracleNotConfigured` (code 34).

### 6. Assert on Shade events

Shade publishes events with `#[contractevent]`, so the topic is the snake-case struct name and the data is a map of field names to values. The `events_named` helper above filters by emitting contract and topic. Convert each field with `try_into_val` and compare it against a typed value:

```rust
use common::events_named;
use soroban_sdk::{Symbol, TryIntoVal};

let paid = events_named(&f.env, &f.shade_id, "invoice_paid_event");
assert_eq!(paid.len(), 1);

let data = &paid[0];
let fee: i128 = data
    .get(Symbol::new(&f.env, "fee"))
    .unwrap()
    .try_into_val(&f.env)
    .unwrap();
let merchant_amount: i128 = data
    .get(Symbol::new(&f.env, "merchant_amount"))
    .unwrap()
    .try_into_val(&f.env)
    .unwrap();

assert_eq!(fee, 50_000);
assert_eq!(merchant_amount, 950_000);
```

`InvoicePaidEvent` carries `invoice_id`, `merchant_id`, `merchant_account`, `payer`, `amount`, `fee`, `merchant_amount`, `token`, and `timestamp`. A single payment also emits other events — a split-routed event and a history event among them — so always filter by topic rather than assuming the last event is the one you want. The full catalogue is in the [Events reference](../reference/events.md), and [Event indexing](indexing-events.md) covers consuming the same events off chain.

> **Warning:** `env.events().all()` reflects the most recent top-level invocation. If you call a read-only getter after the payment, the list you read may no longer include the payment's events. Read events immediately after the call you are asserting on.

### 7. Verify it worked

```bash
cargo test
```

Expect every test above to pass. If a test fails with `Error(Auth, InvalidAction)`, an `Env` was built without `mock_all_auths` — see [Troubleshooting § `Error(Auth, InvalidAction)`](../getting-started/troubleshooting.md#8-hosterror-errorauth-invalidaction).

## Part 2: Test against a local network

Use a local network when your code talks to Soroban RPC — a backend, a keeper, an indexer, or a wallet flow — and you want real ledger closes, real signatures, and real event streams.

### 1. Start the network

```bash
stellar container start local
```

This runs the Stellar quickstart image in Docker with Soroban RPC and Friendbot enabled. Wait until RPC reports healthy:

```bash
curl -s -X POST http://localhost:8000/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

A healthy node answers with `"status":"healthy"`. The first start downloads the image and takes a few minutes. If the CLI does not know a network named `local`, add it:

```bash
stellar network add local \
  --rpc-url http://localhost:8000/rpc \
  --network-passphrase "Standalone Network ; February 2017"
```

### 2. Create and fund identities

```bash
stellar keys generate admin    --network local --fund
stellar keys generate merchant --network local --fund
stellar keys generate payer    --network local --fund
```

`--fund` asks the local Friendbot for XLM. See [Troubleshooting § CLI identity or network not configured](../getting-started/troubleshooting.md#12-cli-identity-or-network-not-configured) if a command reports a missing identity.

### 3. Deploy the stack

Deploy from the optimized WASMs you built in [Part 1, step 1](#1-build-the-wasms-and-vendor-them-into-your-crate):

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/shade.optimized.wasm \
  --source-account admin --network local --alias shade

stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/account.optimized.wasm \
  --source-account admin --network local --alias account

# The native XLM token is already a SEP-41 contract once its asset contract exists.
stellar contract asset deploy --asset native --source-account admin --network local
TOKEN=$(stellar contract id asset --asset native --network local)
```

Configure the hub, as the [Deployment Runbook](../operations/deployment.md#post-deployment-configuration) describes:

```bash
ADMIN=$(stellar keys address admin)
MERCHANT=$(stellar keys address merchant)

stellar contract invoke --id shade --source-account admin --network local -- \
  initialize --admin "$ADMIN"

stellar contract invoke --id shade --source-account admin --network local -- \
  add_accepted_token --admin "$ADMIN" --token "$TOKEN"

stellar contract invoke --id shade --source-account admin --network local -- \
  set_fee --admin "$ADMIN" --token "$TOKEN" --fee 500

stellar contract invoke --id shade --source-account merchant --network local -- \
  register_merchant --merchant "$MERCHANT"
```

Using the native token avoids creating an issuer and trustlines. When you test a classic issued asset instead, the payer's account needs a trustline first — see [Wallet integration § Token prerequisites](wallet-integration.md#token-prerequisites).

### 4. Run your backend tests against it

Generate a typed client once (see [Generating and using contract bindings](contract-bindings.md)), then point it at the local RPC. The local node is plain HTTP, so the client needs `allowHttp`:

```bash
stellar contract bindings typescript \
  --wasm target/wasm32-unknown-unknown/release/shade.optimized.wasm \
  --output-dir ./bindings/shade --overwrite
(cd bindings/shade && npm install && npm run build)
```

```typescript
// test/pay-invoice.test.ts — runs with `node --test` after compiling, or under your own runner
import test from "node:test";
import assert from "node:assert/strict";
import { Keypair, Networks, rpc, scValToNative } from "@stellar/stellar-sdk";
import { basicNodeSigner } from "@stellar/stellar-sdk/contract";
import { Client } from "../bindings/shade/src/index.js";

const RPC_URL = "http://localhost:8000/rpc";
const PASSPHRASE = "Standalone Network ; February 2017";
const SHADE_ID = process.env.SHADE_ID!; // from `stellar contract alias show shade`
const TOKEN_ID = process.env.TOKEN_ID!; // the $TOKEN value above

const merchantKey = Keypair.fromSecret(process.env.MERCHANT_SECRET!);
const payerKey = Keypair.fromSecret(process.env.PAYER_SECRET!);

function clientFor(key: Keypair) {
  return new Client({
    contractId: SHADE_ID,
    networkPassphrase: PASSPHRASE,
    rpcUrl: RPC_URL,
    allowHttp: true,
    publicKey: key.publicKey(),
    ...basicNodeSigner(key, PASSPHRASE),
  });
}

test("payer settles an invoice and the paid event is emitted", async () => {
  const server = new rpc.Server(RPC_URL, { allowHttp: true });
  const { sequence: startLedger } = await server.getLatestLedger();

  const merchant = clientFor(merchantKey);
  const created = await merchant.create_invoice({
    merchant: merchantKey.publicKey(),
    description: "integration test",
    amount: 10_000_000n,
    token: TOKEN_ID,
    expires_at: undefined,
  });
  const { result: invoiceId } = await created.signAndSend();

  const payer = clientFor(payerKey);
  const paid = await payer.pay_invoice({
    payer: payerKey.publicKey(),
    invoice_id: invoiceId,
  });
  await paid.signAndSend();

  const { result: invoice } = await payer.get_invoice({ invoice_id: invoiceId });
  assert.equal(invoice.amount_paid, 10_000_000n);

  const { events } = await server.getEvents({
    startLedger,
    filters: [{ type: "contract", contractIds: [SHADE_ID] }],
  });
  const topics = events.map((e) => scValToNative(e.topic[0]));
  assert.ok(topics.includes("invoice_paid_event"));
});
```

> **Note:** Generated-client option names and `getEvents` field types vary slightly across `@stellar/stellar-sdk` major versions. If the compiler rejects an option, check the generated `bindings/shade/package.json` for the SDK version it installed and match that version's documentation.

Export the contract and key values before running the test, for example `export SHADE_ID=$(stellar contract alias show shade)` and `export MERCHANT_SECRET=$(stellar keys secret merchant)`.

### 5. Reset between runs

State on the local network persists across your test runs, so invoice IDs and merchant registrations accumulate. Either make tests independent of absolute IDs (read the ID `create_invoice` returns, as above), or stop the container and start a fresh one for a clean ledger:

```bash
stellar container stop local
stellar container start local
```

After a reset, redeploy the stack from step 3.

## Verify it worked

In-process: `cargo test` passes in your crate. Local network: confirm the deployed hub answers and is configured.

```bash
stellar contract invoke --id shade --network local -- get_admin
stellar contract invoke --id shade --network local -- is_accepted_token --token "$TOKEN"
# "G…"   (your admin address)
# true
```

## Extracting the helper into a shared test-utils crate

Every integrator ends up copying the fixture above. A published helper would let downstream crates depend on one maintained implementation and would let Shade keep its fixture in step with interface changes. Two designs are worth weighing, and neither exists in the workspace today:

- a `testutils` Cargo feature on the `shade` crate that exposes a `Fixture`-style constructor, so downstream crates take a path or git dependency on `shade` and register the in-crate `Shade` type directly (no WASM import, but compiles Shade into the test binary);
- a separate `shade-testutils` crate that wraps the imported WASM, versioned alongside releases.

This is a design change with maintenance cost, so raise it as its own issue rather than folding it into a docs change.

## Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| `contractimport!` fails with a file-not-found error | The `file` path is resolved relative to your crate's `Cargo.toml`, not the source file. | Put the WASM under your crate (for example `wasm/shade.wasm`) and use a path relative to the manifest. |
| `HostError: Error(Auth, InvalidAction)` in a test | The `Env` was created without `mock_all_auths`, or a cross-contract call needs non-root authorization. | Call `env.mock_all_auths()`; use `env.mock_all_auths_allowing_non_root_auth()` for nested auto-withdrawal paths. |
| `HostError: Error(Contract, #12)` creating an invoice | The token was never whitelisted. | Call `add_accepted_token(&admin, &token)` in your fixture. |
| `HostError: Error(Contract, #2)` in setup | `initialize` ran twice on the same instance. | Create one `Fixture` per test; each test needs its own `Env`. |
| Event helper returns an empty list | The topic name is wrong, or a later call replaced the recorded events. | Use the snake-case struct name (`invoice_paid_event`) and read events straight after the call. |
| Local-network deploy fails with an account error | The identity exists but was not funded on `local`. | Run `stellar keys generate <name> --network local --fund`, or fund it with Friendbot. |
| Local-network invoke reports the contract does not exist | A fresh container was started and the contracts were not redeployed. | Redeploy the stack from step 3. |

## Related pages

- [Running the test suite](../getting-started/running-tests.md) — how this repository's own tests are organized and written.
- [Generating and using contract bindings](contract-bindings.md) — the typed TypeScript client used in the backend test.
- [Accepting payments](accepting-payments.md) — the payment flow these tests exercise.
- [Fiat pricing and oracles](../concepts/fiat-pricing-and-oracles.md) — how the oracle mock's price becomes a token amount.
- [Events reference](../reference/events.md) — every event Shade emits and its fields.
- [Deployment Runbook](../operations/deployment.md) — configuring a real deployment.

← [Back to Guides](README.md)
