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

CI checks links on every PR that touches Markdown (`.github/workflows/docs-quality.yml`), using [lychee](https://github.com/lycheeverse/lychee) with `lychee.toml`.

Run it locally from the repository root:

```bash
cargo install lychee

lychee --config lychee.toml docs README.md CONTRIBUTING.md
```

Or run it with `make docs-check`.

To allowlist a URL that fails the link checker (for example a site that blocks bots), add it to `exclude` in `lychee.toml`.
