# Contributing to Shade Stellar Contract

Thank you for your interest in contributing to Shade Protocol! This document is the canonical contributor guide designed to take you from discovering an issue to getting your pull request merged into `main`.

---

## Table of Contents

1. [Finding and Claiming an Issue](#1-finding-and-claiming-an-issue)
2. [Development Setup & Toolchain](#2-development-setup--toolchain)
3. [Branch Naming Conventions](#3-branch-naming-conventions)
4. [Commit Guidelines](#4-commit-guidelines)
5. [Local Pre-PR Verification Checklist](#5-local-pre-pr-verification-checklist)
6. [Contract & Documentation Synchronization Rules](#6-contract--documentation-synchronization-rules)
7. [Submitting a Pull Request](#7-submitting-a-pull-request)
8. [Review Process & Expectations](#8-review-process--expectations)
9. [Reference Links](#9-reference-links)
10. [Code of Conduct](#10-code-of-conduct)

---

## 1. Finding and Claiming an Issue

All active engineering tasks and bounty tracks are tracked in the [GitHub Issue Tracker](https://github.com/ShadeProtocol/shade-stellar-contract/issues).

### Finding Work
* **Good First Issues**: Labeled `good first issue` — ideal for new contributors to familiarize themselves with the codebase.
* **Documentation & Guides**: Labeled `documentation` — expanding guides, reference pages, and architectural diagrams.
* **Bounties & Stellar Wave**: Labeled `Stellar Wave` — tracked for Stellar Wave and ecosystem developer rewards.

### Claiming an Issue
1. Leave a comment on the issue stating your interest and an overview of your planned approach.
2. A maintainer will assign the issue to you or confirm that you can proceed.
3. **Turnaround & Stale Policy**: To keep development moving, contributors are expected to open a draft PR or post a progress update within **7 calendar days** of claiming an issue (or within the timeframe designated for specific bounty tracks). Inactive assignments will be unassigned and reopened to the community.

---

## 2. Development Setup & Toolchain

### Prerequisites
* **Rust**: Active stable toolchain (`rustup update stable`).
* **WASM Target**: `rustup target add wasm32-unknown-unknown`.
* **Stellar CLI**: Installed and on `PATH` (`cargo install --locked stellar-cli` or via package manager).
* **Git & Pre-commit**: Python pre-commit hooks configured.

### Quick Setup
```bash
# Clone the repository
git clone https://github.com/ShadeProtocol/shade-stellar-contract.git
cd shade-stellar-contract

# Install git pre-commit hooks
pip install pre-commit
pre-commit install

# Verify build
cargo build --workspace
cargo test --workspace
```

---

## 3. Branch Naming Conventions

Always branch from the latest `main` branch. Use descriptive branch names prefixed with the change type and referencing the issue number:

| Prefix | Usage | Example |
|---|---|---|
| `feat/` | New smart contract feature or capability | `feat/refund-routing-204` |
| `fix/` | Bug fix in contracts or scripts | `fix/auth-reentrancy-105` |
| `docs/` | Documentation additions or updates | `docs/contributor-workflow-507` |
| `test/` | Adding, updating, or fixing tests | `test/storage-replay-210` |
| `refactor/` | Code refactoring without behavior changes | `refactor/invoice-storage-301` |
| `ci/` | CI/CD and automation workflow changes | `ci/coverage-threshold-412` |
| `chore/` | Tooling, dependencies, or maintenance | `chore/update-sdk-22` |

---

## 4. Commit Guidelines

We enforce the **Conventional Commits 1.0.0** specification.

### Format
```text
<type>(<scope>): <short description> (#<issue_number>)

[optional longer body explaining context and rationale]

[optional footer(s), e.g., Fixes #<issue_number>]
```

### Allowed Types
* `feat`: A new feature or contract capability.
* `fix`: A bug fix.
* `docs`: Documentation-only changes.
* `style`: Formatting, whitespace, or naming (no production logic change).
* `refactor`: Restructuring code without changing observable behavior.
* `test`: Adding or modifying test suites.
* `chore`: Build scripts, dependencies, or repository hygiene.

### Examples
```text
feat(contract): add refund routing for unspent deposit balances (#204)

docs(contributing): document full contributor workflow and PR lifecycle (#507)

fix(shade): prevent payer address mutation during partial payments (#314)
```

---

## 5. Local Pre-PR Verification Checklist

Before pushing your branch and opening a pull request, run the exact test and lint suite executed by CI:

```bash
# 1. Format code check
make fmt-check
# Or: cargo fmt --all -- --check

# 2. Clippy lint checks
make lint
# Or: cargo clippy --workspace --all-targets -- -D warnings

# 3. Workspace tests
make test
# Or: cargo test --workspace

# 4. Snapshot review (if snapshots changed)
cargo insta review

# 5. Build optimized WASM binaries
make build-optimized
```

All 5 checks must pass locally with zero warnings or errors.

---

## 6. Contract & Documentation Synchronization Rules

To prevent code and documentation drift, Shade enforces strict synchronization requirements on every PR:

1. **Public Interface Changes (`ShadeTrait`)**:
   * Any change that adds, removes, or renames a public function on `ShadeTrait` must update [`docs/reference/shade-interface.md`](docs/reference/shade-interface.md) in the **same pull request**.
2. **Data Types & Storage Layout**:
   * Any change to data structures or `DataKey` in `contracts/shade/src/types.rs` must update [`docs/reference/data-types.md`](docs/reference/data-types.md) and [`docs/architecture/storage-model.md`](docs/architecture/storage-model.md).
3. **Error Codes**:
   * Any new variant added to `contracts/shade/src/errors.rs` must be documented with its numeric code, trigger, and corrective action in [`docs/reference/errors.md`](docs/reference/errors.md).
4. **Style Guide Compliance**:
   * New documentation pages must adhere to the [Documentation Style Guide](docs/contributing/documentation-style-guide.md) and use the templates under [`docs/contributing/templates/`](docs/contributing/templates/).

---

## 7. Submitting a Pull Request

1. **Push your branch** to your fork:
   ```bash
   git push -u origin <branch-name>
   ```
2. **Open the PR**: Use GitHub to open a pull request targeting `ShadeProtocol/shade-stellar-contract:main`.
3. **Fill out the Pull Request Template**:
   * Complete every section in [`.github/pull_request_template.md`](.github/pull_request_template.md).
   * Explicitly link the issue using `Fixes #<number>` or `Closes #<number>`.
   * Check off all applicable checklist items.
   * Provide verifiable test commands or output snippets under **Testing**.
4. **No Sensitive Payout Information**: Do **not** post raw cryptocurrency wallet addresses in PR descriptions or comments. All bounty reward tracking is resolved via Drips Network profiles.

---

## 8. Review Process & Expectations

* **Triage & Response Time**: Maintainers review open PRs continuously. Initial feedback is typically provided within **48 to 72 hours**.
* **Review Requirements**: At least **1 approving review** from a repository maintainer is required before merge.
* **Handling Feedback**:
  * Address feedback by pushing additional commits to your PR branch.
  * Avoid force-pushing over review discussions while active reviews are in progress so maintainers can see incremental diffs.
  * Once discussions are resolved, maintainers will re-review.
* **Merging**: Pull requests are merged using **Squash and Merge** to keep the `main` commit history clean and linear.

---

## 9. Reference Links

* [Architecture Overview](docs/architecture/README.md)
* [System & Flow Diagrams](docs/architecture/diagrams.md)
* [Getting Started Guide](docs/getting-started/README.md)
* [Developer Troubleshooting Guide](docs/getting-started/troubleshooting.md)
* [Frequently Asked Questions (FAQ)](docs/getting-started/faq.md)
* [Makefile and Scripts Reference](docs/reference/makefile-and-scripts.md)
* [Documentation Style Guide](docs/contributing/documentation-style-guide.md)
* [Security Policy](SECURITY.md)

---

## 10. Code of Conduct

We are committed to providing a welcoming, inclusive, and harassment-free environment for all contributors. Please treat fellow community members with respect, communicate constructively, and adhere to open-source collaboration best practices.
