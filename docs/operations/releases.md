# Contract Release & Versioning Process

This document outlines the formal release lifecycle, versioning semantics, reproducible build procedures, artifact verification standards, and changelog maintenance rules for Shade Protocol smart contracts.

---

## 1. Versioning Philosophy

Smart contracts deployed on immutable distributed ledgers require heightened version discipline compared to standard off-chain applications. Shade Protocol follows [Semantic Versioning 2.0.0](https://semver.org/) with on-chain adaptations:

$$\text{v}\langle\text{MAJOR}\rangle.\langle\text{MINOR}\rangle.\langle\text{PATCH}\rangle$$

- **`MAJOR`**: Incompatible API breaks, storage layout shifts, event payload alterations, or error code renumbering requiring integrator code updates or storage migration transactions.
- **`MINOR`**: Backward-compatible feature additions (e.g., new contract entrypoints, query views, optional arguments, or support for additional token interfaces).
- **`PATCH`**: Backward-compatible internal bug fixes, gas/CPU instruction optimizations, internal refactoring, or documentation updates with zero change to public function signatures or storage schemas.

---

## 2. Breaking Changes Definition for Integrators

A release **MUST** be classified as a breaking change (`MAJOR` version increment) if it introduces any of the following:

1. **Public Function Signature Changes**:
   - Renaming or removing any public contract entrypoint in `contracts/*/src/lib.rs`.
   - Modifying parameter types, parameter order, or return types.
   - Changing authentication requirements (e.g., changing caller signature from single account to multi-party `require_auth`).

2. **Storage Layout & Schema Alterations**:
   - Altering existing `DataKey` enum discriminant values or data layouts in contract instance/persistent storage.
   - Changing the serialized representation of structs stored under existing keys (which would cause deserialization panics on live contracts).

3. **Event Schema Modifications**:
   - Changing the `topics` tuple structure or types emitted by contract events.
   - Altering the data payload shape or serialization of contract events consumed by indexers.

4. **Error Code Remapping**:
   - Re-numbering or deleting variants in contract `Error` enums. Integrators mapping errors by numeric code (e.g., `Error::NotAuthorized = 1`) will experience silent contract failures if codes shift.

---

## 3. Mapping Releases to WASM Hashes & Git Tags

To ensure complete cryptographic provenance between source code and on-chain bytecode, every official release strictly binds three elements:

$$\text{Git Commit SHA} \longleftrightarrow \text{Git Tag (e.g., \texttt{v0.4.0})} \longleftrightarrow \text{Optimized WASM SHA-256 Hash}$$

When contracts are deployed or upgraded on Stellar Network, Soroban references the WASM code by its 32-byte hash (`Install` operation). Operators and integrators verify that deployed WASMs match the git release by checking published SHA-256 hashes against reproducible builds.

---

## 4. Step-by-Step Release Procedure

### Phase 1: Pre-Release Verification

Before tagging any release, the candidate commit must pass full verification locally and in CI:

```bash
# 1. Ensure working directory is completely clean
git status

# 2. Check code formatting across the entire workspace
cargo fmt --all -- --check

# 3. Run strict Clippy linter
cargo clippy --workspace --all-features -- -D warnings

# 4. Execute all workspace unit and integration tests
cargo test --workspace --all-features
```

### Phase 2: Reproducible WASM Build & Optimization

All production WASM binaries must be compiled using the pinned toolchain and optimized with the Soroban CLI:

```bash
# Compile contracts for wasm32 target
cargo build --workspace --release --target wasm32-unknown-unknown

# Optimize contract WASMs (stripping debug info and minimizing bytecode size)
mkdir -p target/optimized-wasm
for contract in shade account escrow escrow_factory subscription ticketing ticketing_factory crowdfund crowdfund_factory; do
  soroban contract optimize \
    --wasm target/wasm32-unknown-unknown/release/${contract}.wasm \
    --output target/optimized-wasm/${contract}.optimized.wasm
done
```

### Phase 3: Checksum Generation & Recording

Generate cryptographic hashes for all production WASMs:

```bash
# Generate SHA-256 checksums
cd target/optimized-wasm
sha256sum *.optimized.wasm > SHA256SUMS.txt
cat SHA256SUMS.txt
```

Example `SHA256SUMS.txt` output:
```
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  shade.optimized.wasm
4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a  account.optimized.wasm
ef2d127de37b942baad06145e54b0c619a1f22327b2ebbcfbec78f5564afe39d  escrow.optimized.wasm
```

### Phase 4: Changelog Updating & Tagging

1. Update [`CHANGELOG.md`](../../CHANGELOG.md):
   - Move entries from `[Unreleased]` into the new version header with the release date: `## [X.Y.Z] - YYYY-MM-DD`.
   - Ensure breaking changes (if any) are highlighted under a dedicated **Breaking Changes** callout.
2. Commit the changelog update:
   ```bash
   git add CHANGELOG.md
   git commit -m "chore(release): prepare vX.Y.Z"
   ```
3. Create an annotated and signed Git tag:
   ```bash
   git tag -s vX.Y.Z -m "Release vX.Y.Z: Shade Protocol smart contracts"
   git push origin vX.Y.Z
   ```

### Phase 5: Publishing GitHub Release

Create the GitHub Release corresponding to the git tag:
1. Navigate to **Releases** > **Draft a new release**.
2. Select tag `vX.Y.Z`.
3. Set the release title to `Release vX.Y.Z — <Summary Theme>`.
4. Paste release notes extracted from `CHANGELOG.md`.
5. Attach the optimized WASM files (`*.optimized.wasm`) and the signed `SHA256SUMS.txt` as release binary assets.
6. Publish release.

### Phase 6: Post-Release Deployment & Upgrades

Once the release is tagged and verified:
1. **Staging / Testnet Deployment**: Deploy the new WASM hashes to Stellar Testnet and run the integration test suite.
2. **Mainnet Upgrade**: For existing live contracts, execute the authorized upgrade transaction following the [Upgrade Runbook](upgrade-runbook.md) and [Admin Runbook](admin-runbook.md).
3. **Record Contract Addresses**: Update deployed addresses and corresponding WASM hashes in `.stellar/contract-ids/`.

---

## 5. Hotfix Release Protocol

In the event of an urgent security patch:
1. Branch directly from the live release tag: `git checkout -b hotfix/vX.Y.Z+1 vX.Y.Z`.
2. Implement the minimal targeted fix and regression test.
3. Validate against pre-commit checks and tests.
4. Bump the `PATCH` version number in `CHANGELOG.md`.
5. Tag and publish the hotfix following Phases 2–5.
6. Back-port the hotfix into `main`.
