# 0007. Limit the workspace to the core payment gateway

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

The workspace had grown to nine contract crates, and the `shade` hub to about 230 functions covering crowdfunding, ticketing, NFT rewards, vesting, governance and more. Most of this has nothing to do with the gateway the README describes. It kept `main` from compiling, and every addition grew one contract's audit surface.

## Decision

`contracts/` holds only the gateway: the `shade` hub and the per-merchant `account` contract. Everything else lives in `example/`, a separate Cargo workspace that the root workspace excludes and CI does not run against.

The test for whether code belongs in `shade` is whether a merchant needs it to accept a payment, settle it, refund it, or see it on a dashboard. That keeps merchants, invoices (crypto, fiat-priced, draft and signed), fee routing, refunds, the merchant account, auto-withdrawal, payment analytics and history, swap-ready payload validation, and the admin, access-control, pause and upgrade controls those depend on.

Modules that failed the test moved to the `example` crate closest to their domain, under `src/from_shade/`, with imports pointing at the `shade` crate.

## Consequences

- `shade` has 86 public functions and builds to a WASM of about 80 KB.
- CI can require the whole workspace to pass, because code that is not maintained is no longer in it.
- Subscriptions, escrow and DAO-governed upgrades are no longer available from the hub. Integrators who need them must deploy an example contract or restore the module.
- `example/` is not tested and may drift from the gateway. It is reference material, not a supported API.

## Alternatives considered

- **Fix everything in place:** keeps features, but leaves unrelated code in the audited contract and in every release.
- **Feature flags inside `shade`:** contract interfaces and storage would differ between builds, which complicates bindings and audits.
- **Delete the code:** simplest, but loses working reference implementations.
