# Docs Tooling

## Previewing the site

The docs are built with [mdBook](https://rust-lang.github.io/mdBook/) and [mdbook-mermaid](https://github.com/badboy/mdbook-mermaid).

```bash
cargo install mdbook mdbook-mermaid
mdbook-mermaid install docs   # once, writes mermaid.min.js and mermaid-init.js
mdbook serve docs --open      # or: make docs-serve
```

`make docs` builds the book into `target/book/` and the rustdoc API reference (`cargo doc --workspace --no-deps`) into `target/book/api/`.

When you add a page, also add it to [`docs/SUMMARY.md`](../SUMMARY.md) or it will not appear in the site navigation.

## Quality checks

CI runs these on every PR that touches Markdown (`.github/workflows/docs-quality.yml`):

| Check | Tool | Config |
|---|---|---|
| Links | [lychee](https://github.com/lycheeverse/lychee) | `lychee.toml` |
| Markdown lint | [markdownlint-cli2](https://github.com/DavidAnson/markdownlint-cli2) | `.markdownlint-cli2.jsonc` |
| Spelling | [typos](https://github.com/crate-ci/typos) | `_typos.toml` |

Run them locally from the repository root:

```bash
cargo install lychee typos-cli
npm install -g markdownlint-cli2

lychee --config lychee.toml docs README.md CONTRIBUTING.md
markdownlint-cli2 "docs/**/*.md" README.md CONTRIBUTING.md
typos docs README.md CONTRIBUTING.md
```

Or run all three with `make docs-check`.

## Adding dictionary words

If `typos` flags a valid project term (for example a Soroban or Stellar name), add it to `[default.extend-words]` in `_typos.toml` mapped to itself:

```toml
[default.extend-words]
stroops = "stroops"
```

To allowlist a URL that fails the link checker (for example a site that blocks bots), add it to `exclude` in `lychee.toml`.
