# How to integrate a browser wallet

How to connect a browser wallet, then build, simulate, sign, and submit a `pay_invoice` transaction against the `shade` contract with a generated TypeScript client — including the authorization, token, and network checks that decide whether the payer's signature is actually enough. This guide is for frontend developers building a checkout for [payers](../glossary.md#payer) or a dashboard for [merchants](../glossary.md#merchant).

## Prerequisites

- A generated TypeScript client for `shade` — see [Generating and using contract bindings](contract-bindings.md). The examples import it from `./bindings/shade`.
- A deployed, initialized `shade` contract with the invoice's token whitelisted — see [Deployment Runbook](../operations/deployment.md).
- An invoice ID to pay. This guide starts from "an invoice exists"; the contract side of that flow is in [Accepting payments](accepting-payments.md).
- A browser wallet installed: [Freighter](https://www.freighter.app/), or any wallet supported by [Stellar Wallets Kit](https://github.com/Creit-Tech/Stellar-Wallets-Kit).
- Node.js, and these packages:

  ```bash
  npm install @stellar/stellar-sdk @stellar/freighter-api @creit.tech/stellar-wallets-kit
  ```

> **Note:** Wallet SDKs change their return shapes between major versions. The examples use `@stellar/freighter-api` 3.x or later, where every call resolves to an object that may carry an `error` field, and `@creit.tech/stellar-wallets-kit` 1.x, where you construct a `StellarWalletsKit` instance. Check the installed version's README if a call does not type-check.

## What the wallet is asked to sign

Read this section before writing any code. Shade's entry points call `require_auth()` on a specific address, and that address decides **whose wallet must approve the call**. A client that builds the transaction for one account but names another address in the arguments produces a transaction the contract rejects.

### Who authorizes what

| You call | Address that must authorize | Where it comes from |
|---|---|---|
| `pay_invoice(payer, invoice_id)`, `pay_invoice_partial`, `pay_invoices_batch` | `payer` | [`invoice.rs`](../../contracts/shade/src/components/invoice.rs) calls `payer.require_auth()` |
| `register_merchant(merchant)`, `create_invoice(merchant, …)`, `set_merchant_account`, `refund_invoice` | `merchant` | The merchant is the argument that is authorized |
| `claim_refund(buyer, invoice_id)` | `buyer` | The refund claimant authorizes |
| `subscribe(customer, plan_id)` | `customer` | The subscriber authorizes the subscription |
| `set_fee(admin, …)`, `add_accepted_token(admin, …)`, `pause(admin)` | `admin` | `core::assert_admin` requires the stored admin's auth |
| `upgrade(new_wasm_hash)` | the stored admin | `upgrade` takes no admin argument — it reads the admin and requires that address |
| `withdraw_to(token, amount, recipient)` on an `account` contract | that account's merchant | `account.rs` calls `merchant.require_auth()` |

The pattern is consistent: the address you pass as the actor argument is the address that must sign. Pass the connected wallet's address, never a value copied from the invoice or from user input.

> **Security:** The contract cannot know your UI's intent — it only checks that the address in the argument authorized *this exact invocation*. Never let a client construct `pay_invoice` with a `payer` that differs from the address that will actually sign. See [Access control](../security/access-control.md).

### What a Soroban authorization entry is

A Soroban transaction carries one authorization entry for each address whose `require_auth` the call hits. An entry binds an address to a **specific invocation tree**: a contract, a function, its arguments, and every sub-invocation the signer is approving. Signing the transaction (or the entry) approves that tree and nothing else.

For `pay_invoice` the tree has two levels, and the wallet shows both:

1. `shade.pay_invoice(payer, invoice_id)` — the payer approves the call itself.
2. `token.transfer(payer, merchant_account, merchant_amount)` and `token.transfer(payer, platform_account, platform_fee)` — the payer approves moving their own tokens. The `shade` contract never holds the funds; the token contract checks that the payer authorized each transfer.

The payer therefore sees **two token movements** adding up to the invoice amount, not one: the merchant's share and the protocol fee. That is expected. If your UI shows one number and the wallet shows two transfers, explain it. See [Fees](../concepts/fees.md) for the split.

### Two ways the signature is delivered

- **Source-account authorization.** When the payer is also the transaction's source account — the common checkout case — the transaction signature itself authorizes the invocation. One wallet prompt signs everything. Everything in this guide uses this path.
- **Address-credential authorization.** When a different account pays the network fee (a relayer or sponsor), the payer must sign their authorization entries separately and the relayer signs the transaction. The generated client exposes this through `tx.needsNonInvokerSigningBy()` and `tx.signAuthEntries(...)`, and Freighter supports `signAuthEntry`. If `needsNonInvokerSigningBy()` returns addresses other than the source account, you are on this path.

## Connect Freighter

```typescript
// src/wallet.ts
import {
  isConnected,
  requestAccess,
  getAddress,
  getNetworkDetails,
  signTransaction,
} from "@stellar/freighter-api";

export const NETWORK = {
  name: "TESTNET",
  passphrase: "Test SDF Network ; September 2015",
  rpcUrl: "https://soroban-testnet.stellar.org",
  horizonUrl: "https://horizon-testnet.stellar.org",
} as const;

export class WalletError extends Error {}

/** Opens the Freighter permission prompt and returns the user's address. */
export async function connectFreighter(): Promise<string> {
  const connected = await isConnected();
  if (connected.error || !connected.isConnected) {
    throw new WalletError(
      "Freighter wasn't found. Install it from freighter.app, then reload this page.",
    );
  }

  const access = await requestAccess();
  if (access.error || !access.address) {
    throw new WalletError("You declined the connection request in your wallet.");
  }
  return access.address;
}

/** Refuses to continue when the wallet is on a different network than the app. */
export async function assertWalletNetwork(): Promise<void> {
  const details = await getNetworkDetails();
  if (details.error) {
    throw new WalletError("Couldn't read your wallet's network. Unlock it and try again.");
  }
  if (details.networkPassphrase !== NETWORK.passphrase) {
    throw new WalletError(
      `Your wallet is on ${details.network}. Switch it to ${NETWORK.name} to continue.`,
    );
  }
}

/** Adapts Freighter to the `signTransaction` callback the generated client expects. */
export async function freighterSign(xdr: string, address: string) {
  const result = await signTransaction(xdr, {
    networkPassphrase: NETWORK.passphrase,
    address,
  });
  if (result.error) {
    throw new WalletError("You declined the signature request in your wallet.");
  }
  return { signedTxXdr: result.signedTxXdr, signerAddress: result.signerAddress };
}
```

`requestAccess` is the call that shows the permission prompt; `getAddress` returns the address silently once the site is already allowed, which is what you want on page load. Persist nothing secret — only the public address — and re-read the address and network before each signature, because a user can switch accounts or networks in the wallet at any time.

## Support multiple wallets with Stellar Wallets Kit

Stellar Wallets Kit puts Freighter, xBull, Albedo, Lobstr, and other wallets behind one interface, so the rest of your code does not branch on wallet type.

```typescript
// src/kit.ts
import {
  StellarWalletsKit,
  WalletNetwork,
  allowAllModules,
  FREIGHTER_ID,
} from "@creit.tech/stellar-wallets-kit";
import { NETWORK } from "./wallet";

export const kit = new StellarWalletsKit({
  network: WalletNetwork.TESTNET, // use WalletNetwork.PUBLIC for mainnet
  selectedWalletId: FREIGHTER_ID,
  modules: allowAllModules(),
});

/** Shows the wallet picker and resolves with the chosen wallet's address. */
export function connectWithKit(): Promise<string> {
  return new Promise((resolve, reject) => {
    kit
      .openModal({
        onWalletSelected: async (option) => {
          try {
            kit.setWallet(option.id);
            const { address } = await kit.getAddress();
            resolve(address);
          } catch (e) {
            reject(e);
          }
        },
        onClosed: () => reject(new Error("You closed the wallet picker.")),
      })
      .catch(reject);
  });
}

export async function kitSign(xdr: string, address: string) {
  const { signedTxXdr } = await kit.signTransaction(xdr, {
    networkPassphrase: NETWORK.passphrase,
    address,
  });
  return { signedTxXdr, signerAddress: address };
}
```

Both `freighterSign` and `kitSign` have the same shape, so you choose one when the user connects and pass it into the payment code below. Keep the `WalletNetwork` value and `NETWORK.passphrase` in lockstep — they are two spellings of the same decision.

## Select the network deliberately

A transaction's signature covers the **network passphrase**, so a transaction signed for testnet cannot be replayed on mainnet, and the reverse. That protects you from replay, but it does not protect the user from mistakes of intent:

- **Real money on the wrong network.** A user who believes they are on testnet, with a wallet pointed at mainnet, signs a transaction that moves real tokens. Display the network name prominently on every payment screen.
- **Contract IDs are per network.** The same contract ID string means nothing on another network. A wallet on the wrong network simulates against a ledger where the contract does not exist, and the failure looks like a contract error rather than a network error.
- **The app chooses the passphrase.** The passphrase the wallet signs under comes from your code. Define it once, in one constant, and do not accept it from a URL parameter or user input.

Check the wallet's network with `assertWalletNetwork()` before building the transaction **and** immediately before signing. Testnet and mainnet passphrases are:

| Network | Passphrase |
|---|---|
| Testnet | `Test SDF Network ; September 2015` |
| Mainnet | `Public Global Stellar Network ; September 2015` |
| Local (`stellar container`) | `Standalone Network ; February 2017` |

## Token prerequisites

A payment can fail for reasons that have nothing to do with Shade's own rules: the token contract refuses a transfer the payer or a recipient cannot take part in. Check these before asking for a signature, and show a message the user can act on.

### Trustlines for classic assets

A classic Stellar asset (USDC, for example) is exposed to Soroban through its Stellar Asset Contract (SAC). **A classic account (`G…` address) must hold a trustline to the asset to hold or receive it.** Three addresses are involved in a Shade payment:

| Address | Needs a trustline when… |
|---|---|
| The payer | always — they hold the tokens being sent |
| The merchant's settlement account | the account is a `G…` address. `register_merchant` sets it to the merchant's own wallet address by default, so the **merchant's wallet** needs the trustline unless an `account` contract was linked with `set_merchant_account` |
| The platform account | the platform account is a `G…` address (it is the admin's address unless `set_platform_account` changed it) |

Contract addresses (`C…`) — including an `account` contract — hold SAC balances in contract storage and need no trustline. Native XLM needs no trustline either.

### Sufficient balance

The payer needs at least the amount `resolve_invoice_amount(invoice_id)` returns — which, for a fiat-priced invoice, can change between page load and signature — in the invoice's token.

### Fee reserves

Every Soroban transaction costs a fee paid in XLM, whatever token is being transferred, and a Stellar account cannot spend below its **minimum balance** (base reserve × `2 + subentries`). A payer holding plenty of USDC and almost no XLM cannot pay. When the invoice token is XLM itself, the payer needs the amount **plus** the fee **plus** the reserve.

### A pre-flight check

Run this before building the transaction. It reads state; it does not sign anything.

```typescript
// src/preflight.ts
import {
  Address,
  Asset,
  BASE_FEE,
  Contract,
  Horizon,
  TransactionBuilder,
  rpc,
  scValToNative,
} from "@stellar/stellar-sdk";
import { NETWORK } from "./wallet";

const rpcServer = new rpc.Server(NETWORK.rpcUrl);
const horizon = new Horizon.Server(NETWORK.horizonUrl);

const STROOPS = 10_000_000n;
const BASE_RESERVE_STROOPS = 5_000_000n; // 0.5 XLM
const FEE_BUFFER_STROOPS = 5n * STROOPS; // generous allowance for the network fee

export class PreflightError extends Error {}

/** Reads a SEP-41 balance by simulating `balance(address)`; works for any token. */
export async function readTokenBalance(tokenId: string, holder: string): Promise<bigint> {
  const source = await rpcServer.getAccount(holder);
  const tx = new TransactionBuilder(source, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK.passphrase,
  })
    .addOperation(new Contract(tokenId).call("balance", new Address(holder).toScVal()))
    .setTimeout(30)
    .build();

  const sim = await rpcServer.simulateTransaction(tx);
  if (!rpc.Api.isSimulationSuccess(sim) || !sim.result) {
    throw new PreflightError("Couldn't read your token balance. Try again in a moment.");
  }
  return scValToNative(sim.result.retval) as bigint;
}

/** True when a `G…` account holds a trustline to `asset` (or the asset is XLM). */
export async function hasTrustline(address: string, asset: Asset): Promise<boolean> {
  if (asset.isNative() || !address.startsWith("G")) return true;
  const account = await horizon.loadAccount(address);
  return account.balances.some(
    (b) =>
      "asset_code" in b && b.asset_code === asset.getCode() && b.asset_issuer === asset.getIssuer(),
  );
}

export interface PaymentCheck {
  payer: string;
  tokenId: string;
  asset: Asset; // the classic asset behind the token's SAC, or Asset.native()
  amount: bigint; // from shade.resolve_invoice_amount, in the token's base units
  merchantAccount: string; // from shade.get_merchant_account
  platformAccount: string; // from shade.get_platform_account
}

/** Throws a PreflightError whose message is safe to show to the payer. */
export async function checkCanPay(c: PaymentCheck): Promise<void> {
  const code = c.asset.isNative() ? "XLM" : c.asset.getCode();

  if (!(await hasTrustline(c.payer, c.asset))) {
    throw new PreflightError(
      `Your account can't hold ${code} yet. Add a ${code} trustline in your wallet, then try again.`,
    );
  }
  if (!(await hasTrustline(c.merchantAccount, c.asset))) {
    throw new PreflightError(
      `This merchant can't receive ${code} right now. Please contact the merchant.`,
    );
  }
  if (!(await hasTrustline(c.platformAccount, c.asset))) {
    throw new PreflightError("This payment can't be processed right now. Please try again later.");
  }

  const balance = await readTokenBalance(c.tokenId, c.payer);
  if (balance < c.amount) {
    throw new PreflightError(
      `Your ${code} balance is ${fmt(balance)}, but this invoice needs ${fmt(c.amount)}.`,
    );
  }

  const account = await horizon.loadAccount(c.payer);
  const xlm = account.balances.find((b) => b.asset_type === "native");
  const spendable = xlm ? toStroops(xlm.balance) - toStroops(xlm.selling_liabilities) : 0n;
  const reserve = BASE_RESERVE_STROOPS * BigInt(2 + account.subentry_count);
  const needsXlmForAmount = c.asset.isNative() ? c.amount : 0n;
  if (spendable < reserve + FEE_BUFFER_STROOPS + needsXlmForAmount) {
    throw new PreflightError(
      "You need more XLM to cover network fees and your account reserve. Add XLM and try again.",
    );
  }
}

const toStroops = (decimal: string): bigint => {
  const [whole, frac = ""] = decimal.split(".");
  return BigInt(whole) * STROOPS + BigInt(frac.padEnd(7, "0").slice(0, 7));
};
const fmt = (stroops: bigint): string => (Number(stroops) / Number(STROOPS)).toFixed(7);
```

`fmt` assumes a 7-decimal token, which is the Stellar convention for SAC tokens; read the token's `decimals` when you support other SEP-41 tokens. The fee buffer is deliberately generous — the exact fee comes from simulation, and this check only needs to catch accounts that are plainly short on XLM.

## Pay an invoice end to end

Each step maps to one stage of the Soroban transaction lifecycle: **build and simulate**, **sign**, **submit and confirm**. The generated client performs the first stage when you call the method, and the second and third in `signAndSend`.

```typescript
// src/pay.ts
import { Asset, rpc } from "@stellar/stellar-sdk";
import { Client } from "../bindings/shade/src/index.js";
import { NETWORK, WalletError, assertWalletNetwork } from "./wallet";
import { checkCanPay, PreflightError } from "./preflight";
import { friendlyError } from "./errors";

type Signer = (xdr: string, address: string) => Promise<{ signedTxXdr: string; signerAddress?: string }>;

export function shadeClient(contractId: string, address: string, sign: Signer) {
  return new Client({
    contractId,
    networkPassphrase: NETWORK.passphrase,
    rpcUrl: NETWORK.rpcUrl,
    publicKey: address, // the transaction's source account
    signTransaction: (xdr) => sign(xdr, address),
  });
}

export async function payInvoice(
  contractId: string,
  invoiceId: bigint,
  payer: string,
  sign: Signer,
  asset: Asset,
): Promise<void> {
  try {
    // 0. The wallet must be on the network this app targets.
    await assertWalletNetwork();

    const shade = shadeClient(contractId, payer, sign);

    // 1. Read what the payer will actually be charged, right now.
    const { result: invoice } = await shade.get_invoice({ invoice_id: invoiceId });
    const { result: amount } = await shade.resolve_invoice_amount({ invoice_id: invoiceId });
    const { result: merchantAccount } = await shade.get_merchant_account({
      merchant_id: invoice.merchant_id,
    });
    const { result: platformAccount } = await shade.get_platform_account();

    // 2. Check trustlines, balance, and XLM for fees before asking for a signature.
    await checkCanPay({
      payer,
      tokenId: invoice.token,
      asset,
      amount,
      merchantAccount,
      platformAccount,
    });

    // 3. Build and simulate. `pay_invoice` returns an AssembledTransaction that has
    //    already been simulated against the RPC node.
    const tx = await shade.pay_invoice({ payer, invoice_id: invoiceId });
    if (tx.simulation && rpc.Api.isSimulationError(tx.simulation)) {
      throw new Error(tx.simulation.error); // e.g. "HostError: Error(Contract, #27)"
    }

    // 4. Confirm only the payer must authorize. Anything else needs its own signature.
    const others = tx.needsNonInvokerSigningBy().filter((a) => a !== payer);
    if (others.length > 0) {
      throw new Error(`Unexpected additional signer required: ${others.join(", ")}`);
    }

    // 5. Re-check the network, then sign in the wallet and submit.
    await assertWalletNetwork();
    await tx.signAndSend();

    // 6. Confirm from chain state, not from the call returning.
    const { result: after } = await shade.get_invoice({ invoice_id: invoiceId });
    if (after.amount_paid < after.amount) {
      throw new Error("Payment landed but the invoice is not fully paid.");
    }
  } catch (e) {
    if (e instanceof WalletError || e instanceof PreflightError) throw e;
    throw new Error(friendlyError(e));
  }
}
```

Points worth knowing:

- **`get_invoice` and `resolve_invoice_amount` are reads.** They simulate and return a value without a signature. For a fiat-priced invoice, `resolve_invoice_amount` re-quotes from the oracle until the first payment lands; call it again right before signing if the user lingered on the confirmation screen. See [Accepting payments § Fiat-priced invoices](accepting-payments.md#fiat-priced-invoices).
- **`pay_invoice` simulates when you call it.** The simulation produces the resource footprint, the fee, and the authorization entries. If the contract would reject the call — an expired invoice, a non-whitelisted token — the failure appears here, before the wallet opens.
- **`signAndSend` re-uses the simulation.** If the ledger moves on in a way that changes the footprint, submission can fail even though simulation passed. See [Troubleshooting § Simulation succeeds but submission fails](../getting-started/troubleshooting.md#10-simulation-succeeds-but-submission-fails).
- **Confirm from state.** `pay_invoice` returns `()`, so success is "did not panic" plus a follow-up `get_invoice` read. For partial payments use `pay_invoice_partial(payer, invoice_id, amount)` and compare `amount_paid` to `amount`.
- **Amounts are `bigint`.** Use `n`-suffixed literals for every amount; `number` silently loses precision above 2^53.

## Turn failures into messages

A Shade panic reaches the browser as text containing `Error(Contract, #N)`. Map the number to a sentence the payer can act on, and fall back to a generic message — never show raw host errors.

```typescript
// src/errors.ts
const CONTRACT_MESSAGES: Record<number, string> = {
  1: "This payment couldn't be authorized. Check that you're signed in with the right account.",
  7: "That amount isn't valid for this invoice.",
  9: "Payments are temporarily unavailable. Please try again later.",
  12: "This invoice's payment token is no longer accepted. Please contact the merchant.",
  16: "This invoice has already been paid or is no longer payable.",
  27: "This invoice has expired. Please ask the merchant for a new one.",
  34: "This invoice's price can't be resolved right now. Please contact the merchant.",
  35: "This invoice's price can't be resolved right now. Please try again shortly.",
  41: "The merchant doesn't accept this token. Please contact the merchant.",
};

export function friendlyError(e: unknown): string {
  const text = e instanceof Error ? e.message : String(e);

  const contract = /Error\(Contract, #(\d+)\)/.exec(text);
  if (contract) {
    return (
      CONTRACT_MESSAGES[Number(contract[1])] ??
      "The payment was rejected by the contract. Please contact support."
    );
  }
  if (/Error\(Auth, InvalidAction\)/.test(text)) {
    return "Your wallet's approval didn't match this payment. Reconnect your wallet and try again.";
  }
  if (/txBadSeq|tx_bad_seq/.test(text)) {
    return "Another transaction from your account was in progress. Please try again.";
  }
  if (/Error\(Budget|ExceededLimit/.test(text)) {
    return "The network was too busy to process this payment. Please try again.";
  }
  return "Something went wrong submitting your payment. Please try again.";
}
```

The numbers are the `shade` contract's `ContractError` codes — the full list is in the [Errors reference](../reference/errors.md), and the payment-path subset with recommended UI wording is in [Accepting payments § Error handling](accepting-payments.md#error-handling). These codes are not global: if you also call `account` or another Shade contract, key the table by contract ID.

> **Warning:** Token-contract failures — a missing trustline, an underfunded account — surface as the *token's* error codes, which differ from Shade's and are not stable across token implementations. That is why the pre-flight check above tests those conditions directly instead of trying to decode them after the fact.

## A minimal example app

A complete, single-page checkout that ties the pieces together. Create a Vite project, add the packages from [Prerequisites](#prerequisites), generate the bindings into `./bindings/shade`, and add these files.

```html
<!-- index.html -->
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Shade checkout</title>
  </head>
  <body>
    <main>
      <h1>Pay invoice</h1>
      <p id="network"></p>
      <button id="connect">Connect wallet</button>
      <p id="account"></p>
      <label>Invoice ID <input id="invoice" type="number" min="1" value="1" /></label>
      <button id="pay" disabled>Pay</button>
      <p id="status" role="status"></p>
    </main>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

```typescript
// src/main.ts
import { Asset } from "@stellar/stellar-sdk";
import { NETWORK, connectFreighter, freighterSign } from "./wallet";
import { payInvoice } from "./pay";

// Replace with your deployment's values.
const SHADE_ID = import.meta.env.VITE_SHADE_ID as string;
const ASSET = Asset.native(); // or new Asset("USDC", ISSUER) for an issued asset

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
let payer: string | undefined;

$("network").textContent = `Network: ${NETWORK.name}`;

$("connect").addEventListener("click", async () => {
  try {
    payer = await connectFreighter();
    $("account").textContent = `Connected: ${payer}`;
    $<HTMLButtonElement>("pay").disabled = false;
  } catch (e) {
    $("status").textContent = (e as Error).message;
  }
});

$("pay").addEventListener("click", async () => {
  if (!payer) return;
  const invoiceId = BigInt($<HTMLInputElement>("invoice").value);
  $("status").textContent = "Waiting for your wallet…";
  try {
    await payInvoice(SHADE_ID, invoiceId, payer, freighterSign, ASSET);
    $("status").textContent = "Paid. Thank you!";
  } catch (e) {
    $("status").textContent = (e as Error).message;
  }
});
```

```bash
# .env.local
VITE_SHADE_ID=<your shade contract ID>
```

`<your shade contract ID>` is the only placeholder: use the contract ID printed when you deployed `shade` to the network in `NETWORK`. Then run the app with `npm run dev`. To try it without real funds, use testnet, the native token, and an invoice created from the CLI:

```bash
stellar contract invoke --id <shade-id> --network testnet --source-account merchant -- create_invoice \
  --merchant <merchant-address> --description "wallet demo" --amount 10000000 --token <native-token-id>
```

The native token's contract ID comes from `stellar contract id asset --asset native --network testnet`. If you would rather run against a local network, see [Testing integrations against Shade locally](testing-integrations.md#part-2-test-against-a-local-network) and swap the `NETWORK` constants.

## Verify it worked

1. Open the app, click **Connect wallet**, and approve the prompt. The page shows your address.
2. Enter the invoice ID and click **Pay**. Your wallet shows an invocation of `pay_invoice` and two token transfers.
3. Approve. The page reports success, and the invoice reads back as paid:

   ```bash
   stellar contract invoke --id <shade-id> --network testnet -- get_invoice --invoice_id 1
   # status: Paid (1), amount_paid equals the invoice amount
   ```

## Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| "Freighter wasn't found" | The extension is not installed or is disabled for this site. | Install Freighter, allow it on the site, and reload. |
| "Your wallet is on PUBLIC. Switch it to TESTNET" | The wallet's network differs from `NETWORK.passphrase`. | Switch the network in the wallet. Do not change the app's passphrase to match. |
| `Error(Auth, InvalidAction)` on submit | The `payer` argument is not the address that signed, or the signed transaction was altered after simulation. | Use the connected address for both `publicKey` and the `payer` argument; do not modify the XDR between simulate and sign. |
| Wallet shows two token transfers for one payment | The invoice amount is split into the merchant's share and the protocol fee. | Expected — see [Fees](../concepts/fees.md). |
| Preflight reports a missing trustline for the merchant | The merchant's settlement account is a `G…` address without a trustline to the asset. | The merchant adds the trustline, or links an `account` contract with `set_merchant_account`. |
| Simulation fails with `Error(Contract, #12)` or `#41` | The token is not whitelisted globally (12) or by this merchant (41). | The admin calls `add_accepted_token`; the merchant calls `set_merchant_accepted_tokens`. |
| Simulation passes but the transaction fails | State changed between simulation and inclusion. | Re-run the flow; see [Troubleshooting](../getting-started/troubleshooting.md#10-simulation-succeeds-but-submission-fails). |

## Related pages

- [Accepting payments](accepting-payments.md) — the contract-side payment flow, partial and batch payments, and fiat-priced invoices.
- [Generating and using contract bindings](contract-bindings.md) — producing the `Client` used throughout this guide.
- [Access control](../security/access-control.md) — which roles and addresses authorize which functions.
- [Errors reference](../reference/errors.md) — every contract error code.
- [Testing integrations against Shade locally](testing-integrations.md) — running this flow against a local network.

← [Back to Guides](README.md)
