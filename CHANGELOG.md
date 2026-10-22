# Changelog

All notable changes to the Shade Protocol contracts are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow the scheme in [Releases](docs/operations/releases.md#versioning-scheme).

This file exists so that integrators and operators can see what each deployed WASM version contains. Each release section lists which contract changed, so you can skip entries for contracts you do not use. Read these first:

- **Breaking changes** are prefixed with `**BREAKING:**` and always say what to do about them. The criteria are in [Releases § Breaking changes](docs/operations/releases.md#breaking-changes-for-integrators).
- **Entries are grouped by contract** with a bold label: **shade**, **account**, **escrow**, **escrow_factory**, **subscription**, **ticketing**, **ticketing_factory**, **crowdfund**, **crowdfund_factory**. A label of **workspace** covers the toolchain, CI, and anything that touches every contract.
- **Pull requests** are referenced as `#NNN`.

> **Note:** No release has been tagged yet. Every crate is still at `0.0.0`, and the history before the first tag is recorded under [Pre-release history](#pre-release-history), backfilled from merged pull requests. Dates are merge dates. The first tagged release will move the contents of [Unreleased](#unreleased) into a dated version section.

## [Unreleased]

### Added

- **workspace:** Documentation set under `docs/` — architecture, concepts, contract and interface references, security notes, operator runbooks, and guides (#469–#475, #478–#483, #520–#525).
- **workspace:** FAQ and developer troubleshooting guide under `docs/getting-started/`.
- **workspace:** Guide to testing integrations against Shade locally, in-process and on a local network (`docs/guides/testing-integrations.md`).
- **workspace:** Wallet integration guide for Freighter and Stellar Wallets Kit (`docs/guides/wallet-integration.md`).
- **workspace:** This changelog and the release process (`docs/operations/releases.md`), including the breaking-change criteria and an "Update CHANGELOG" item in the pull request template.

## Pre-release history

Everything below predates the first tagged release. It is grouped by month, then by contract, at the granularity of a merged feature.

### July 2026 — campaigns, vesting, and KYC

- **shade:** Campaign KYC and verification system with a reviewer lifecycle (#379, #418).
- **shade:** Stretch goals with automated unlocking and milestones (#411, #417).
- **shade:** Token vesting schedules, admin-configurable vesting timelines, and a creator vesting test suite (#412, #416).
- **shade:** Fiat-pegged campaign goals valued through the token oracle registry (#413).
- **shade:** Analytics data exports (#415).
- **shade:** Campaign secondary-sale royalties, with royalty-config validation on resale (#410).
- **shade:** Tiered backer rewards and perks, reward NFT minting and distribution, and a reward NFT secondary market with test suites (#395, #406, #408).
- **shade:** Campaign categories and tags, on-chain campaign announcements, multi-token pledges, and failed-campaign refund automation (#380, #381, #386, #394).
- **shade:** Top-donor leaderboard tracking, deep read-only campaign statistics, backer badges and achievements, and on-chain search and filtering (#404).
- **shade:** Multi-sig requirements for large withdrawals, with tests.
- **shade:** Platform-fee abstraction and routing module, fee waivers and charity campaign discounts, and a platform-fee test suite (#390).
- **shade:** DAO governance for protocol upgrades: `propose_upgrade`, `vote_on_upgrade`, `finalize_upgrade`.
- **shade:** Cross-chain pledges, a bridge listener interface for external deposits, and bridge integration tests (#393).
- **shade:** Oracle integration test suite (#405).
- **crowdfund:** Early-bird discount and time-based pricing (#378).
- **crowdfund:** Affiliate links and referral tracking, social recovery for campaign creator accounts, and slashing mechanism tests (#387, #391, #403).
- **crowdfund_factory:** Campaign approval proposals and DAO governance (#403).
- **workspace:** CI fix for a redundant `u32` cast flagged by newer clippy.

### May–June 2026 — crowdfunding and multi-token subscriptions

- **crowdfund:** New `crowdfund` contract with campaign execution, refunds, stretch goals, and pledge tests (#318, #319).
- **crowdfund:** Reward tiers, per-backer reward fulfillment tracking, and milestone-based fund release with voting tests (#320, #322).
- **crowdfund:** Pledge gateway and tracking, batch refunds, and resolution tests (#321).
- **crowdfund_factory:** New `crowdfund_factory` contract with factory deployment, sponsor matching, and pledge comments (#323).
- **subscription:** Creator fee routing, renewal and expiry events, upgrades and downgrades, and trial periods (#317).
- **subscription:** Multi-token subscriptions and integration tests (#318).
- **shade:** Analytics aggregation tests (#291).

### April 2026 — escrow, ticketing, and merchant tooling

- **escrow:** New `escrow` contract (2026-04-24), followed by deposit, completion routing, dispute initialization, buyer approval, arbiter split resolution, deadlines, and expired-escrow refunds (#218, #224, #232–#236, #244, #258, #275).
- **escrow_factory:** New `escrow_factory` contract (2026-04-30).
- **ticketing:** New `ticketing` contract (2026-04-27) with ticket tiers, transfer and check-in, dynamic pricing, batch refunds, event cancellation, capacity waitlists, and group bookings (#245, #257, #276, #277, #279, #280, #287, #288).
- **ticketing_factory:** New `ticketing_factory` contract (2026-04-29).
- **subscription:** New standalone `subscription` contract (2026-04-28), with subscription billing and cancellation, grace periods, and prorated refunds (#288, #290).
- **shade:** Merchant webhook configuration, withdraw to another account, partial refunds, and user transaction history queries (#219, #222, #225, #226).
- **shade:** Payment-token volume tracking and volume aggregations (#221, #227).
- **shade:** Multi-sig withdrawals and events, oracle analytics routing proxy, and merchant auto-withdrawal thresholds (#223, #229, #256).
- **account:** Merchant restriction tests and auto-withdrawal thresholds (#229, #230).

### March 2026 — fees, drafts, and token lists

- **shade:** Draft invoices, merchant-defined accepted-token lists, and invoice filtering by date range (#163, #164, #166, #167).
- **shade:** Fee calculation, volume-based fee discounts, and time-locked fee changes (`propose_fee` / `execute_fee`) (#165, #168, #169, #175).
- **shade:** Batch invoice payment and two-step admin ownership transfer (#173, #174).
- **shade:** Merchant integration support (#182).
- **shade:** Test coverage for the reentrancy guard, merchant restriction, invoice expiry, invoice amendments, batch token whitelisting, and date-range filtering (#172, #176–#181).
- **workspace:** Codebase review and follow-up tests (#160, #161).

### February 2026 — core protocol

- **workspace:** Repository setup, contract initialization, and core data structures (#1–#4).
- **account:** New merchant `account` contract (#6).
- **shade:** Accepted-token management and token balance tracking (#36–#38, #40).
- **shade:** Invoice creation and management, amendments, and filtering (#45, #46, #120).
- **shade:** Merchant public key management, merchant verification, and merchant account restriction (#49, #52, #58, #93, #94, #132).
- **shade:** Contract upgrade support, with upgrade tests (#51, #53).
- **shade:** Fee management (#59, #60).
- **shade:** Invoice refunds, partial refunds, and void invoice (#90, #96, #102).
- **shade:** Signed invoice creation with signature verification and nonce-based replay protection (#91, #124, #125).
- **shade:** Partial payments, invoice payment, and a granular transfer-status event on `pay_invoice` (#128, #131, #155).
- **shade:** Admin-led invoice payment, merchant withdrawal, and admin-transfer expiry (#97, #98, #133).
- **shade:** Batch token whitelisting (#121).
- **shade:** Subscription plans and recurring billing logic (#158).
- **account:** Merchant account factory and merchant account test coverage (#151, #157).
