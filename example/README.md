# Example contracts

Contracts and modules that are not part of the core Shade payment gateway. This is a separate Cargo workspace: the root workspace excludes it, and CI, pre-commit and the docs checks do not run against it.

```bash
cd example
cargo test -p escrow
```

## Standalone contracts

`crowdfund`, `crowdfund_factory`, `escrow`, `escrow_factory`, `subscription`, `ticketing` and `ticketing_factory` were moved here unchanged. `crowdfund` and `crowdfund_factory` did not compile before the move and still do not.

## Code moved out of the Shade hub

Each `src/from_shade/` folder holds modules that used to live inside `contracts/shade`, along with their types, events, errors, tests and the hub entry points that exposed them (`entrypoints.rs`).

| Crate | Modules |
|---|---|
| `crowdfund` | `analytics`, `backer_rewards`, `campaign`, `campaign_royalties`, `campaigns`, `comments`, `cross_chain_pledge`, `fiat_goals`, `kyc`, `leaderboard`, `nft`, `pledge`, `stretch_goals`, `vesting`, `voting` |
| `escrow` | `escrow` |
| `subscription` | `subscription`, subscription searches |
| `ticketing` | `event`, event searches |
| `platform_extras` | `governance`, `bridge`, `multisig_withdrawal`, withdrawal proposal searches |

Calls into the gateway use the `shade` crate (`shade::components::merchant`, `shade::types::Invoice`, and so on). This code is reference material and has not been fixed. It sits behind the `from-shade` feature, which is off by default, so the default build of each crate is unaffected. With the feature on, `escrow` compiles; the other crates still have errors from the code itself.
