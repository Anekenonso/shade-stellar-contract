# 0005. Separate factories for escrow and ticketing contracts

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

Escrow agreements and ticketed events are independent products with different lifecycles, storage needs, and upgrade cadences. Each instance should be its own contract so funds and state are isolated.

## Decision

`escrow_factory` and `ticketing_factory` are separate crates, each storing its own WASM hash and deploying and indexing its own instances. They do not share a generic factory.

## Consequences

- Each product can be upgraded, paused, or redeployed without touching the other.
- Factory registries stay small and type-specific.
- Some deployment and registry logic is duplicated between factories.

## Alternatives considered

- **One generic factory with a contract-type parameter:** less code but couples upgrades and widens the blast radius of a factory bug.
- **Deploying instances from the shade hub:** increases hub WASM size and couples unrelated products to hub upgrades.
