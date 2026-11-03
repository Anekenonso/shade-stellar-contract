# Prerequisites and local toolchain setup

Everything you must install before the Shade Protocol workspace will build or test, with the exact versions the repository is pinned to. Work through this page once on a fresh machine, then run the [verification snippet](#verify-your-environment) at the end to confirm the toolchain matches what the workspace expects.

Once your environment checks out, continue to [Building the Contracts](building.md).

## What you need

| Tool | Required version | Why |
|---|---|---|
| Rust (`rustc`, `cargo`) | 1.84.0 or newer, stable channel | Minimum supported Rust version of `soroban-sdk` 23.5.3 and its `soroban-env-host` / `stellar-xdr` dependencies. |
| `wasm32v1-none` target | Matching your toolchain | Soroban executes WASM; the workspace cannot produce a deployable artifact without it. |
| Stellar CLI (`stellar`) | 23.x | Optimizes, installs, deploys, and invokes contracts. The CLI major version tracks the protocol/SDK major — pair 23.x with `soroban-sdk` 23.x. |
| `rustfmt`, `clippy` | Shipped with the stable toolchain | CI fails the build on unformatted code and on any clippy warning. |
| `pre-commit` (optional) | Any recent release; needs Python 3 | Runs the same hooks locally that [`.github/workflows/pre-commit.yml`](../../.github/workflows/pre-commit.yml) runs on every PR. |

> **Note:** Everything below assumes you have already cloned the repository and are running commands from its root.

## Install Rust

Install the toolchain through `rustup`, which is what CI uses (via `dtolnay/rust-toolchain@stable`) and what lets you manage targets and components later.

### macOS and Linux

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### Windows (WSL)

Build inside a WSL 2 Linux distribution rather than native Windows — the Stellar CLI, the pre-commit hooks, and the `Makefile` all assume a POSIX shell. From your WSL terminal:

```bash
sudo apt-get update && sudo apt-get install -y build-essential pkg-config libssl-dev curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### Select the stable toolchain

The workspace does not pin a toolchain file, so `rustup`'s default channel is what builds it. Set it to stable explicitly:

```bash
rustup default stable
rustup update stable
```

Your `rustc` must be **1.84.0 or newer**. That floor comes from the `rust-version` declared by `soroban-sdk` 23.5.3 and by `soroban-env-common`, `soroban-env-host`, and `stellar-xdr` 23.x, all of which are resolved in [`Cargo.lock`](../../Cargo.lock). An older compiler fails to resolve the dependency graph rather than failing to compile, so the error message points at Cargo, not at Shade's code.

### Add the WASM target

Soroban contracts compile to `wasm32v1-none`. Add the target once per toolchain:

```bash
rustup target add wasm32v1-none
```

### Add the formatting and lint components

```bash
rustup component add rustfmt clippy
```

## Install the Stellar CLI

The `stellar` binary is the current name for what was previously the `soroban` CLI; the standalone `soroban-cli` is superseded by it. Install `stellar` — the [`Makefile`](../../Makefile) invokes `stellar contract optimize`, `stellar contract deploy`, `stellar contract install`, and `stellar contract invoke` by that name.

### From source (all platforms, including WSL)

`cargo install` works everywhere Rust does and is the most reliable way to get a specific version:

```bash
cargo install --locked stellar-cli --version '^23'
```

The `'^23'` requirement takes the newest 23.x release and refuses to jump to a different major line. Building the CLI from source needs the system packages installed above (`build-essential`, `pkg-config`, `libssl-dev` on Debian/Ubuntu).

### macOS (Homebrew)

```bash
brew install stellar-cli
```

If the formula is not found, tap Stellar's repository first:

```bash
brew install stellar/tap/stellar-cli
```

### Linux (prebuilt binary)

Download the archive matching your architecture from the [stellar-cli releases page](https://github.com/stellar/stellar-cli/releases), pick a `23.x` tag, and put the extracted `stellar` binary on your `PATH`:

```bash
tar -xzf stellar-cli-*-x86_64-unknown-linux-gnu.tar.gz
sudo mv stellar /usr/local/bin/
```

### Windows (WSL)

Use the Linux instructions above from inside your WSL distribution. Do not install the Windows-native binary and call it across the WSL boundary — path translation breaks `--wasm target/...` arguments.

### Verify the install

```bash
stellar --version
```

Confirm the reported version starts with `23.`. If you have an older `soroban` binary still on your `PATH`, remove it (`cargo uninstall soroban-cli`) so shell completions and docs don't send you to the wrong tool.

## Version compatibility with `soroban-sdk`

Two different versions matter, and they are not the same number:

| Where | Value | Meaning |
|---|---|---|
| [`Cargo.toml`](../../Cargo.toml) `[workspace.dependencies]` | `soroban-sdk = "23.4.0"` | The **requirement**. Caret semantics, so any `23.x.y` at or above 23.4.0 satisfies it. |
| [`Cargo.lock`](../../Cargo.lock) | `soroban-sdk 23.5.3` | The **resolved** version every contributor and CI actually builds against. |

Every crate under [`contracts/`](../../contracts) consumes the SDK through `soroban-sdk = { workspace = true }`, so there is exactly one SDK version in the workspace. Tests additionally enable the SDK's `testutils` feature via each crate's `[dev-dependencies]`.

Rules that follow from this pinning:

- **Use Stellar CLI 23.x.** The CLI's major version tracks the Soroban protocol version that `soroban-sdk` 23.x compiles for. A CLI from a different major line may reject the WASM at `stellar contract deploy`, or deploy a contract that traps on invocation.
- **Do not delete or regenerate `Cargo.lock` casually.** It is deliberately committed — see the note in [`.gitignore`](../../.gitignore). `soroban-env-host` 23.0.1 declares `ed25519-dalek = ">=2.0.0"` but only compiles against 2.x, so an unpinned resolve picks 3.0.0 and the build fails.
- **Bump the SDK in one place.** Change `[workspace.dependencies]` in the root [`Cargo.toml`](../../Cargo.toml), never in an individual contract crate.

## Recommended tooling

### Formatting and linting

CI runs both as hard gates ([`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)), so run them before you push:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-features -- -D warnings
```

The [`Makefile`](../../Makefile) wraps the first and third as `make fmt` and `make lint`.

### Pre-commit hooks

The repository ships [`.pre-commit-config.yaml`](../../.pre-commit-config.yaml). Installing the hooks catches the same failures locally that the `Pre-commit Checks` workflow catches on your PR:

```bash
pip install pre-commit
pre-commit install
```

Run them across the whole tree once after installing, so you start from a clean state:

```bash
pre-commit run --all-files
```

The configured hooks are:

| Repo | Pinned rev | Hooks |
|---|---|---|
| `pre-commit/pre-commit-hooks` | `v4.5.0` | `check-yaml`, `check-added-large-files` (max 1000 KB), `check-merge-conflict`, `check-case-conflict`, `mixed-line-ending` (`--fix=lf`), `detect-private-key` |
| `doublify/pre-commit-rust` | `v1.0` | `clippy` (`--all-features -- -D warnings`), `cargo-check` (`--all-features`) |
| `pre-commit/mirrors-prettier` | `v4.0.0-alpha.8` | `prettier` on TOML files, excluding `Cargo.lock` |

> **Note:** The `clippy` and `cargo-check` hooks compile the workspace, so your first commit after installing them is slow. Subsequent commits reuse the `target/` cache.

## Verify your environment

Run this snippet from the repository root. It checks every requirement above and prints a pass/fail line for each.

```bash
#!/usr/bin/env bash
echo "rustc:        $(rustc --version 2>/dev/null || echo 'MISSING')"
echo "cargo:        $(cargo --version 2>/dev/null || echo 'MISSING')"
echo "stellar CLI:  $(stellar --version 2>/dev/null | head -1 || echo 'MISSING')"
echo "rustfmt:      $(cargo fmt --version 2>/dev/null || echo 'MISSING')"
echo "clippy:       $(cargo clippy --version 2>/dev/null || echo 'MISSING')"
echo "pre-commit:   $(pre-commit --version 2>/dev/null || echo 'not installed (optional)')"

rustup target list --installed | grep -q '^wasm32v1-none$' \
  && echo "wasm target:  installed" \
  || echo "wasm target:  MISSING - run: rustup target add wasm32v1-none"

grep -A1 '^name = "soroban-sdk"$' Cargo.lock | tail -1 \
  | sed 's/^version = /resolved sdk: /'
```

Expected output looks like this — exact patch versions vary, the major versions must not:

```text
rustc:        rustc 1.84.0 (9fc6b4312 2025-01-07)
cargo:        cargo 1.84.0 (66221abde 2024-11-19)
stellar CLI:  stellar 23.5.1
rustfmt:      rustfmt 1.8.0-stable (9fc6b431 2025-01-07)
clippy:       clippy 0.1.84 (9fc6b4312 2025-01-07)
pre-commit:   pre-commit 3.7.1
wasm target:  installed
resolved sdk: "23.5.3"
```

Then confirm the workspace actually builds and tests:

```bash
cargo build --target wasm32v1-none --release
cargo test --workspace --all-features
```

If both succeed, your environment is correct.

## Troubleshooting

### `can't find crate for 'core'` / `the wasm32v1-none target may not be installed`

The WASM target is missing from the active toolchain. Targets are per-toolchain, so adding it under one toolchain does not add it under another:

```bash
rustup target add wasm32v1-none
rustup show          # confirms which toolchain is active
```

If you switched toolchains (for example `rustup default 1.84.0`), re-add the target for the new one.

### `package requires rustc 1.84.0 or newer`

Your compiler is older than the SDK's minimum supported version. Update:

```bash
rustup update stable
rustup default stable
rustc --version
```

### Linker errors (`cc: command not found`, `linking with 'cc' failed`)

Native builds and `cargo test` link against host libraries, so a C toolchain is required even though the deployed artifact is WASM.

```bash
# Debian / Ubuntu / WSL
sudo apt-get install -y build-essential pkg-config libssl-dev

# Fedora / RHEL
sudo dnf install -y gcc gcc-c++ pkgconf-pkg-config openssl-devel

# macOS
xcode-select --install
```

### Stale or conflicting `Cargo.lock`

Symptom: a resolution error naming `ed25519-dalek`, or a build that fails only for you after a merge.

Never "fix" this by deleting the lock file. Restore the committed one and let Cargo reuse it:

```bash
git checkout -- Cargo.lock
cargo build --locked
```

`--locked` makes Cargo fail rather than silently re-resolve, which is what you want when reproducing a CI failure. If a merge left conflict markers in `Cargo.lock`, take the version from `main` and re-run `cargo build` to re-add only your new dependencies:

```bash
git checkout --theirs Cargo.lock   # or: git checkout main -- Cargo.lock
cargo build
```

### Apple Silicon (M-series) specifics

- **Homebrew paths.** On Apple Silicon Homebrew installs to `/opt/homebrew`, not `/usr/local`. If `stellar` is not found after `brew install stellar-cli`, add it to your shell profile:

  ```bash
  echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
  eval "$(/opt/homebrew/bin/brew shellenv)"
  ```

- **Use the native `aarch64-apple-darwin` toolchain.** A Rust installed under Rosetta produces an `x86_64` toolchain that builds far more slowly and can conflict with native Homebrew libraries. Check with `rustup show`; if it reports `x86_64-apple-darwin` on an M-series Mac, reinstall `rustup` from a native (non-Rosetta) terminal.

- **OpenSSL for `cargo install stellar-cli`.** If the CLI build fails looking for OpenSSL:

  ```bash
  brew install openssl@3 pkg-config
  export OPENSSL_DIR="$(brew --prefix openssl@3)"
  cargo install --locked stellar-cli
  ```

- The `wasm32v1-none` output itself is architecture-independent — a WASM built on Apple Silicon is byte-identical to one built on x86-64 Linux with the same toolchain and lock file.

### `pre-commit` hooks fail immediately after install

The `clippy` and `cargo-check` hooks run against the whole workspace with `--all-features`. If they fail on code you did not touch, confirm the failure also exists on `main`:

```bash
git stash && cargo clippy --workspace --all-features -- -D warnings; git stash pop
```

If it fails on `main` too, it is a pre-existing failure, not something your change introduced — open an issue rather than disabling the hook.

## Next steps

- [Building the Contracts](building.md) — native and WASM builds, release profiles, and producing an optimized deployment WASM.
- [Running the Test Suite](running-tests.md) — how the test suite is organized and how to run subsets of it.
- [Contributing guidelines](../../CONTRIBUTING.md) — branch naming, commit format, and the PR checklist.

← [Back to Getting Started](README.md)
