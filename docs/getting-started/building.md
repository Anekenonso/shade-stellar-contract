# Building the Contracts

This page covers building the Shade Protocol workspace for development and for deployment, and explains every setting in the workspace's release profiles.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- The `wasm32v1-none` target: `rustup target add wasm32v1-none`
- [Stellar CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) (`stellar`) for optimizing and deploying WASM

## The workspace

The repository is a Cargo workspace (see [`Cargo.toml`](../../Cargo.toml)) whose members are every crate under `contracts/`:

```text
contracts/
├── account/
├── shade/
```

Each crate builds as both a `lib` and a `cdylib` (see each crate's `Cargo.toml`, e.g. [`contracts/shade/Cargo.toml`](../../contracts/shade/Cargo.toml)) — `lib` for native builds and tests, `cdylib` for the WASM binary Soroban actually deploys.

## Native build

A native build compiles the workspace for your host platform. This is what `cargo test` uses, and it's the fastest way to check that the code compiles while iterating:

```bash
cargo build
```

To build a single crate rather than the whole workspace, use `--manifest-path` (or `-p <crate-name>` from anywhere in the workspace):

```bash
cargo build --manifest-path contracts/shade/Cargo.toml
# equivalently:
cargo build -p shade
```

Native build artifacts land under `target/debug/`. These binaries are not deployable to Soroban — they exist for compilation checks and running tests.

## WASM build

Soroban contracts must be compiled to `wasm32v1-none`. To build every contract in the workspace as WASM:

```bash
cargo build --target wasm32v1-none --release
```

This uses the `[profile.release]` settings in the workspace [`Cargo.toml`](../../Cargo.toml) (see [Release profile settings](#release-profile-settings) below). Artifacts land under:

```text
target/wasm32v1-none/release/<crate-name>.wasm
```

For example:

```text
target/wasm32v1-none/release/shade.wasm
target/wasm32v1-none/release/account.wasm
```

Each crate's WASM file name matches its `[package].name` in that crate's `Cargo.toml` (`shade`, `account`, and so on), with `-` replaced by `_`.

To build only one contract crate's WASM instead of the whole workspace:

```bash
cargo build --target wasm32v1-none --release --manifest-path contracts/shade/Cargo.toml
# equivalently:
cargo build --target wasm32v1-none --release -p shade
```

## The `release-with-logs` profile

The workspace also defines a `[profile.release-with-logs]` in [`Cargo.toml`](../../Cargo.toml), which inherits from `release` but re-enables `debug-assertions`:

```toml
[profile.release-with-logs]
inherits = "release"
debug-assertions = true
```

Use this profile when you need `log!` output or debug assertions while testing a contract on a local network — a plain `release` build strips debug assertions along with everything else, so failures that only show up with assertions enabled (or diagnostic logging) go silent. Build with it the same way, naming the profile explicitly:

```bash
cargo build --target wasm32v1-none --profile release-with-logs
```

> **Note:** Don't ship a `release-with-logs` build to a live network. It exists for local debugging; use the plain `release` profile (via `stellar contract optimize`, below) for anything deployed to testnet, futurenet, or mainnet.

## Release profile settings

Every key in `[profile.release]` (see [`Cargo.toml`](../../Cargo.toml)) exists to make the deployed WASM as small and as cheap to execute as possible — both size and CPU/instruction count feed directly into what a Soroban transaction costs to submit and what a contract instance costs in storage rent for its code. Changing any of these has a real cost; understand what you're trading before touching one.

| Setting | Value | Effect | Cost of changing it |
|---|---|---|---|
| `opt-level` | `"z"` | Optimizes for the smallest possible binary size, over `"3"`'s optimization for speed. | Raising this toward `"s"` or `"3"` trades a smaller WASM for faster execution — rarely worth it for Soroban, where WASM size and instruction count both cost more than native CPU time would suggest. |
| `overflow-checks` | `true` | Keeps arithmetic overflow checks in the compiled WASM, panicking on overflow instead of silently wrapping. | Turning this off would shrink the binary slightly and shave some instructions, but risks a silent integer wraparound in fee, balance, or amount arithmetic — a correctness and security regression, not just a size one. Leave this on. |
| `debug` | `0` | Emits no debug info into the binary. | Raising this bloats the WASM with symbol/line data useless in production; only relevant for local debugging (see `release-with-logs` above, which does not itself change this). |
| `strip` | `"symbols"` | Strips symbol tables from the compiled binary. | Reduces binary size further with no behavioral effect; the trade-off is losing symbol names in a native crash backtrace, which doesn't apply to Soroban WASM execution the way it would for a native binary. |
| `debug-assertions` | `false` | Disables `debug_assert!` checks and other debug-only guards. | `release-with-logs` (above) flips this back on for local debugging; a production build should keep it `false` so those checks don't inflate size or leak debug-only panics into a live contract. |
| `panic` | `"abort"` | Panics abort execution immediately instead of unwinding the stack. | Unwinding requires extra generated code (landing pads, unwind tables) that a Soroban contract has no use for — it can't catch a panic and keep running regardless. `abort` is smaller and is the only sensible choice here. |
| `codegen-units` | `1` | Forces the whole crate to be optimized as a single compilation unit. | Fewer codegen units means slower compile times but lets the optimizer see across the whole crate, producing smaller/faster code than the parallel-compilation default. Worth the slower build for a WASM binary that ships to chain. |
| `lto` | `true` | Enables link-time optimization across the whole dependency graph. | Substantially increases build time in exchange for cross-crate inlining and dead-code elimination that shrinks the final WASM meaningfully. This is the single biggest lever on final binary size in this profile. |

See the [Stellar docs on the release profile](https://developers.stellar.org/docs/build/guides/conventions) and [logging during development](https://developers.stellar.org/docs/build/guides/testing/debugging) for further background on why these particular settings are conventional for Soroban contracts.

## Optimizing the WASM for deployment

A `cargo build --release` WASM is already using the profile above, but the Stellar CLI applies further WASM-specific optimization (e.g. `wasm-opt`-style trimming) on top of it before deployment. Run it per contract:

```bash
stellar contract optimize --wasm target/wasm32v1-none/release/shade.wasm
stellar contract optimize --wasm target/wasm32v1-none/release/account.wasm
```

This writes an optimized copy alongside the input, suffixed `.optimized.wasm`:

```text
target/wasm32v1-none/release/shade.optimized.wasm
target/wasm32v1-none/release/account.optimized.wasm
```

Check the resulting size per contract:

```bash
ls -lh target/wasm32v1-none/release/*.optimized.wasm
```

The `.optimized.wasm` file — not the plain `.wasm` file — is what you deploy with `stellar contract deploy` or `stellar contract install`.

## Using the Makefile

The repository's [`Makefile`](../../Makefile) wraps the commands above for the `shade` and `account` contracts specifically:

```bash
make build      # cargo build --target wasm32v1-none --release, for shade + account
make optimize   # build, then stellar contract optimize for shade + account
```

Run `make help` to see every available target, including deployment, pausing, and upgrade helpers that consume the optimized WASM produced here.

## Running tests

Tests run against the native build, not WASM:

```bash
cargo test --workspace --all-features
```

To test a single contract crate:

```bash
cargo test --manifest-path contracts/shade/Cargo.toml
```
