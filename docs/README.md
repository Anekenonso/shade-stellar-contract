# Shade Protocol Documentation

Shade is a decentralized payment gateway built on Stellar/Soroban that lets merchants accept crypto payments through customizable invoices, with funds settling to a per-merchant vault contract. This is the documentation home for the `shade-stellar-contract` repository: everything you need to build, integrate with, operate, or extend the protocol lives under one of the sections below.

## Where do I start?

- **Everyone, first:** [Getting Started → Introduction](getting-started/introduction.md) explains what Shade is, who uses it, and how a payment flows end to end. Every other page assumes it.
- **Contract contributors** (writing or reviewing Rust/Soroban code): after the introduction, set up your toolchain with [Prerequisites](getting-started/prerequisites.md), then [Building](getting-started/building.md) and [Running Tests](getting-started/running-tests.md), then read [Architecture](architecture/README.md) and the [Glossary](glossary.md) before diving into [Contracts](contracts/README.md).
- **Merchant integrators** (building an app or backend against Shade): start with [Getting Started](getting-started/README.md), then [Guides](guides/README.md) for task-oriented walkthroughs and [Reference](reference/README.md) for the full contract interface.
- **Protocol operators** (running deployments, admin duties): start with [Operations](operations/README.md) and [Security](security/README.md).

## Table of contents

| Section | What lives here |
|---|---|
| [Getting Started](getting-started/README.md) | Installing tooling, building the workspace, running a local deployment. |
| [Architecture](architecture/README.md) | How the hub, the merchant account, and the components fit together. |
| [Contracts](contracts/README.md) | Per-contract reference pages (Shade, Account). |
| [Concepts](concepts/README.md) | Deep dives into individual mechanisms (invoices, fees, refunds, auto-withdrawal). |
| [Security](security/README.md) | Threat model, access control, reentrancy protections, and audit-relevant notes. |
| [Reference](reference/README.md) | Glossary, error codes, storage key layout, and other lookup material. |
| [Guides](guides/README.md) | Task-oriented how-tos for common integration and contribution tasks. |
| [Operations](operations/README.md) | Runbooks for deploying, pausing, upgrading, and administering live contracts. |

## Glossary

Domain and Soroban terminology used throughout these docs is defined in [`docs/glossary.md`](glossary.md). Link to it on first use of a term, per the [style guide](contributing/documentation-style-guide.md#terminology).

## Contributing to the docs

See the [Documentation Style Guide](contributing/documentation-style-guide.md) for tone, formatting, and terminology rules, and [`docs/contributing/templates/`](contributing/templates) for the page templates to start a new page from.
