# Changelog

All notable changes to the Shade Protocol smart contract workspace will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) adapted for Soroban smart contracts.

---

## [Unreleased]

### Added
- Root `SECURITY.md` defining supported contract versions, private vulnerability reporting channels, and safe harbor protections ([#510](https://github.com/ShadeProtocol/shade-stellar-contract/issues/510)).
- Formal `CHANGELOG.md` and release management documentation in `docs/operations/releases.md` ([#508](https://github.com/ShadeProtocol/shade-stellar-contract/issues/508)).
- Pull request template checklist requirement for changelog tracking.

---

## [0.4.0] - 2026-10-02

### Added
- **Operations & Runbooks**:
  - Comprehensive Admin Operations and Incident Response Runbook covering key custody, two-step timelocked fee adjustments, emergency pause operations, and role management ([#482](https://github.com/ShadeProtocol/shade-stellar-contract/pull/482), [#468](https://github.com/ShadeProtocol/shade-stellar-contract/issues/468)).
  - End-to-end contract upgrade playbook with storage migration validation and testnet/mainnet verification protocols ([#482](https://github.com/ShadeProtocol/shade-stellar-contract/pull/482), [#467](https://github.com/ShadeProtocol/shade-stellar-contract/issues/467)).
  - Contract bindings generation guide and SDK distribution standards ([#482](https://github.com/ShadeProtocol/shade-stellar-contract/pull/482), [#465](https://github.com/ShadeProtocol/shade-stellar-contract/issues/465)).
- **Payment & Fee Engine**:
  - Complete documentation and specification of the protocol fee model, volume discount tiers, and timelocked fee proposal/execution mechanisms ([#483](https://github.com/ShadeProtocol/shade-stellar-contract/pull/483)).
  - End-to-end specifications for full payments, partial payments, and batch invoice settlements ([#483](https://github.com/ShadeProtocol/shade-stellar-contract/pull/483)).
  - Merchant onboarding walkthrough, error-code catalog, webhook architecture, and Soroban event indexing guides ([#478](https://github.com/ShadeProtocol/shade-stellar-contract/pull/478)).
- **Contract Specifications**:
  - Full interface references and lifecycle state-machine documentation for `invoice`, `subscription`, `ticketing`, and `escrow` contracts ([#480](https://github.com/ShadeProtocol/shade-stellar-contract/pull/480)).
  - Deep dive into instance/persistent/temporary storage models, refunds/voids handling, and merchant account isolation ([#479](https://github.com/ShadeProtocol/shade-stellar-contract/pull/479)).

---

## [0.3.0] - 2026-09-29

### Added
- **Architecture & Foundation**:
  - Hub-and-spoke component architecture specification detailing cross-contract call patterns between the central `shade` gateway and modular contracts ([#481](https://github.com/ShadeProtocol/shade-stellar-contract/pull/481)).
  - Comprehensive event logging specification covering all emitted contract events and topics ([#481](https://github.com/ShadeProtocol/shade-stellar-contract/pull/481)).
  - Developer Quickstart guide, repository workspace directory layout, and developer onboarding instructions ([#474](https://github.com/ShadeProtocol/shade-stellar-contract/pull/474)).
  - Security architecture documentation covering role-based access control (RBAC), pausable emergency stops, reentrancy guards, and cryptographic signature verifications ([#475](https://github.com/ShadeProtocol/shade-stellar-contract/pull/475)).
  - Payment payloads, multi-token swap routing abstractions, and fiat oracle price settlement designs ([#472](https://github.com/ShadeProtocol/shade-stellar-contract/pull/472)).

---

## [0.2.0] - 2026-09-15

### Added
- **Multi-Contract Ecosystem**:
  - Initial factory pattern implementation for merchant escrow deployment (`contracts/escrow_factory`).
  - Event ticketing and resale price-cap contracts (`contracts/ticketing`, `contracts/ticketing_factory`).
  - Milestone-based crowdfund campaign contracts (`contracts/crowdfund`, `contracts/crowdfund_factory`).
  - Recurring subscription payment authorization contracts (`contracts/subscription`).
- **Merchant Accounts**:
  - Isolated merchant account contract (`contracts/account`) supporting independent withdrawal thresholds and authorized withdrawal addresses.

---

## [0.1.0] - 2026-08-20

### Added
- **Initial Protocol Foundation**:
  - Core `shade` payment router contract on Soroban.
  - Multi-token payment routing with token allowlisting.
  - Off-chain signature verification for merchant invoices using ed25519 nonces.
  - Admin governance, two-step ownership transfer, and emergency pause controls.
  - Automated unit and integration test harnesses with `soroban-sdk`.

---

## Breaking Change Classification Guide

Changes to smart contracts are classified as **Breaking** if they require client, SDK, or downstream integrator modifications:

| Category | Breaking Changes (Major Version Bump) | Non-Breaking Changes (Minor/Patch Bump) |
| :--- | :--- | :--- |
| **Public API** | Renaming/removing functions, altering argument types, changing return value structures. | Adding new optional functions, adding new view methods. |
| **Events** | Changing event topics, modifying payload tuple/struct fields, altering serialization. | Emitting new distinct event types with separate topics. |
| **Error Codes** | Renaming/re-numbering error variants in `Error` enums, changing error conditions. | Adding new error variants for new functionality. |
| **Storage Layout** | Modifying `DataKey` enum variants, altering storage entry layout, changing storage TTL classes. | Appending non-conflicting new storage keys with default fallbacks. |
| **Authentication** | Altering required `require_auth` signatories or invocation auth hierarchies. | Relaxing signature requirements without compromising funds. |
