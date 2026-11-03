# Shade Protocol

Shade is a cutting-edge decentralized payment gateway designed to facilitate seamless, secure, and borderless crypto payments for businesses and individuals. Built on the Stellar blockchain, Shade empowers users with fast, cost-effective, and transparent transactions using smart contracts and layer 2 scalability.

## Overview

Shade enables merchants, freelancers, and enterprises to accept digital payments effortlessly without intermediaries, high fees, or delays. The platform leverages blockchain technology to ensure a permissionless flow of funds, enhanced by features like customizable invoices, SDKs, and a user-friendly dashboard.

## Key Features

- **Decentralized Payment Gateway**: Accept crypto payments directly via smart contracts.
- **Borderless Transactions**: Send and receive payments globally without traditional banking barriers.
- **Fast & Cost-Effective**: Powered by Stellar for high speed and low transaction fees.
- **Merchant Tools**: access to customizable invoices, SDKs for integration, and a comprehensive dashboard.
- **Permissionless**: No intermediaries; ensuring full control over your funds.
- **Enhanced Capabilities**: Optional fiat off-ramping and email-based notifications.
- **Swap-ready Payment Interfaces**: Payment payload types can describe pay-in and settle-out tokens, routing paths, and slippage tolerance for automated swap integrations.


## Getting Started

### Prerequisites

Ensure you have the following installed:
- [Rust](https://www.rust-lang.org/tools/install)
- `wasm32v1-none` target: `rustup target add wasm32v1-none`
- [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup#install-the-soroban-cli)

### Installation

1.  Clone the repository:
    ```bash
    git clone https://github.com/ShadeProtocol/shade-stellar-contract.git
    cd shade-stellar-contract
    ```

2.  Build the project:
    ```bash
    cargo build --workspace --target wasm32v1-none --release
    ```

3.  Run tests:
    ```bash
    cargo test
    ```

## Development

The project is organized as a Cargo workspace containing Soroban smart contracts.

### Project Structure

The repository is structured as a Cargo workspace with modular crates. For a detailed breakdown of crate roles, dependencies, and file standards, see the [Workspace and Crate Layout Guide](docs/architecture/workspace-layout.md).

```text
shade-stellar-contract/
├── contracts/          # The gateway: the shade hub and the per-merchant account
├── example/            # Non-core contracts and modules, in their own workspace outside CI
├── docs/               # Architecture, quickstart tutorials, and contract references
└── scripts/            # Deployment and testing automation
```

### Building Contracts

To build the optimized WASM binary for deployment:

```bash
cargo build --workspace --target wasm32v1-none --release
```


## Documentation

Full documentation, including architecture, per-contract references, concept deep dives, security notes, and operational runbooks, lives under [`docs/`](docs/README.md).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1.  Fork the repository
2.  Create your feature branch (`git checkout -b feat/AmazingFeature`)
3.  Commit your changes (`git commit -m 'feat: Add some AmazingFeature'`)
4.  Push to the branch (`git push origin feat/AmazingFeature`)
5.  Open a Pull Request

