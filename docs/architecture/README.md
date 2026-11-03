# Architecture

How the Shade Protocol contracts and shared components fit together.

- [Workspace and crate layout](workspace-layout.md).
- [The component pattern](component-pattern.md) — the `shade` contract's component structure.
- [Storage model](storage-model.md) — every `DataKey` variant, storage tier usage, ID-counter and collection patterns, and TTL/rent behavior in the `shade` contract.
- [Upgradeability, WASM Hash Management, and Versioning](upgradeability.md) — the admin upgrade path, storage compatibility rules for persisted types, migration patterns, and how a deployed build is identified.

← [Back to documentation home](../README.md)
