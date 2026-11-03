# Workspace and Crate Layout

The Cargo workspace, directory structure, crate boundaries, and standard file conventions of this repository.

## Workspace configuration (`Cargo.toml`)

- **Members:** `members = ["contracts/*"]` picks up the `shade` and `account` crates.
- **Excluded:** `exclude = ["example"]`. The `example/` directory has its own workspace, so root builds, tests and CI never touch it.
- **Dependency pinning:** `[workspace.dependencies]` sets `soroban-sdk = "23.4.0"`, and `Cargo.lock` is committed.
- **Release profile:** `opt-level = "z"`, `codegen-units = 1`, `lto = true` and `strip = "symbols"` keep the WASM small.

## Directory tree

```text
shade-stellar-contract/
├── .github/workflows/      # CI: tests, formatting, clippy, WASM build, docs
├── contracts/
│   ├── account/            # Per-merchant vault contract
│   └── shade/              # Payment gateway hub
├── docs/                   # This documentation
├── example/                # Non-core contracts and modules (separate workspace, outside CI)
├── scripts/                # Developer tooling
├── Cargo.lock
├── Cargo.toml
├── Makefile
└── README.md
```

## Crates

| Crate | Purpose | Key modules | Reference |
|---|---|---|---|
| `shade` | Merchants, invoices, fee routing, refunds, analytics, access control | `shade`, `shade_interface`, `types`, `errors`, `events`, `components/` | [Shade](../contracts/shade.md) |
| `account` | Per-merchant vault: balances, withdrawals, refunds, restrictions | `account`, `interface`, `types`, `errors`, `events` | [Account](../contracts/account.md) |

`shade` depends on `account` only in tests. At runtime the hub reaches accounts through small `#[contractclient]` traits and deploys them from a stored WASM hash.

## Contract crate structure

1. **`lib.rs`** declares the modules.
2. **The interface** (`shade_interface.rs`, `interface.rs`) is the `#[contracttrait]` listing every exported function.
3. **The contract** (`shade.rs`, `account.rs`) implements that trait.
4. **`types.rs`**, **`errors.rs`** and **`events.rs`** hold stored records and keys, error codes, and events.
5. **`components/`** (`shade` only) holds the logic for each feature.
6. **`tests/`** holds tests, using `soroban_sdk::testutils`.

## Related pages

- [Architecture overview](overview.md)
- [The component pattern](component-pattern.md)
