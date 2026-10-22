# Guides

Task-oriented how-tos, written from the [how-to template](../contributing/templates/how-to-template.md).

- [Generating and using contract bindings](contract-bindings.md) — generate a typed TypeScript client, understand the type mappings, and keep it in sync with the deployed WASM.
- [Testing integrations against Shade locally](testing-integrations.md) — import the compiled WASMs into your own crate's tests, reuse a setup helper, mock an oracle, assert on events, and run the full stack on a local network.
- [Wallet integration guide](wallet-integration.md) — connect Freighter or any Stellar Wallets Kit wallet, build, simulate, sign, and submit `pay_invoice`, and handle authorization, token, and network prerequisites.
- How to register and verify a merchant — *planned*.
- [Merchant onboarding guide](merchant-onboarding.md) — ordered checklist from registration to accepting payments.
- [Merchant webhooks and off-chain notifications](webhooks.md) — configuring `set_merchant_webhook` and building a notifier.
- [Event indexing guide](indexing-events.md) — building a reliable off-chain indexer over Shade's events.
- How to create and pay an invoice — *planned*.
- How to set up a subscription plan — *planned*.
- How to create and sell event tickets — *planned*.
- How to launch a crowdfunding campaign — *planned*.

← [Back to documentation home](../README.md)
