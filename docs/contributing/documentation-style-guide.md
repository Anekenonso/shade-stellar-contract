# Documentation Style Guide

This guide defines how documentation is written across the Shade Protocol `docs/` tree. Shade has 50+ documentation pages contributed by many people; consistent voice, structure, and formatting are what let a reader trust a page they've never seen before as much as one they wrote themselves.

Every docs PR is expected to follow this guide. See [Review Checklist](#review-checklist) before requesting review.

## Voice and tense

- Write in **second person** ("you configure the oracle"), not first person plural ("we configure the oracle") or passive voice ("the oracle is configured").
- Write in **present tense**. Describe what the code does now, not what it will do or did: "`pay_invoice` transfers the invoice amount from the payer to the merchant account," not "will transfer" or "transfers (as of v2)."
- Be direct and imperative in instructional content: "Run `cargo build`," not "You should run `cargo build`" or "One would run `cargo build`."
- Avoid hedging ("might," "should probably," "in most cases") when the code's behavior is deterministic. If behavior genuinely varies, say what it depends on.

## Heading hierarchy

- One `#` (H1) per page, matching the page title.
- Use **sentence case** for all headings: "Building the contracts," not "Building The Contracts" or "BUILDING THE CONTRACTS."
- Nest headings sequentially — don't skip from `##` to `####`.
- Keep headings short enough to appear in a table of contents without wrapping.

## Structure

- Open with one or two sentences stating what the page covers and who it's for.
- Order sections from "what a reader needs first" to "what a reader needs once they're deeper in." For a how-to, that means prerequisites before steps before troubleshooting.
- Prefer several short sections over one long undifferentiated block of prose.
- End procedural pages with a way to verify success (a command to run, an expected output) rather than leaving the reader to guess whether it worked.

## Code samples

- **Always tag the language** on fenced code blocks (` ```rust `, ` ```bash `, ` ```toml `). An untagged block does not get syntax highlighting and reads as an oversight.
- **Show real signatures copied from the source.** Every function signature, type definition, or CLI invocation in a doc must match what's actually in the repository at the path you cite. Copy it — don't retype it from memory.
- **Never invent APIs.** If a function, field, or CLI flag doesn't exist in the codebase, don't document it as if it does, even as a "future" example. If you need an illustrative example, base it on a real call and mark clearly what is illustrative versus literal.
- Keep examples minimal — enough to demonstrate the point, not a full production integration.
- When a code sample is meant to be run as-is (a build command, a CLI invocation), it must be copy-pasteable: no placeholder syntax like `<your-value>` inside a block that's otherwise meant to run verbatim unless every such placeholder is called out immediately below the block.

## Referencing source code

- When you refer to a type, function, or module, name its file path relative to the repo root, e.g. `contracts/shade/src/types.rs`.
- **Link to a line range** when you're pointing at a specific definition (a struct, a function body) that a reader would otherwise have to search for, using GitHub's `#L<start>-L<end>` anchor syntax, e.g. `contracts/shade/src/types.rs#L398-L413` for the `Invoice` struct.
- Do not link to line ranges for content that moves frequently (e.g. "the components directory") — link to the directory or file instead.
- Because line numbers drift as code changes, re-verify a line-range link points at the claimed content before merging a docs PR that cites one, and again during the accuracy pass of any PR that touches the referenced file.

## Callouts

Use blockquote-style callouts with a bold label, one per paragraph of callout content:

```markdown
> **Note:** Fee changes take effect only after `execute_fee` is called; `propose_fee` alone does not change the active fee.

> **Warning:** `upgrade` replaces the contract's WASM immediately for anyone calling it with admin auth. Coordinate upgrades through the DAO governance flow (`propose_upgrade` / `vote_on_upgrade` / `finalize_upgrade`) rather than calling `upgrade` directly in production.

> **Security:** `claim_refund` and `refund_invoice` both move funds and must be reviewed for reentrancy on every change — see the [Reentrancy Guard](../security/reentrancy.md) concept page.
```

- **Note** — supplementary information that helps understanding but isn't required to proceed.
- **Warning** — a mistake a reader could easily make that causes broken behavior or lost funds, but isn't a security vulnerability per se.
- **Security** — anything touching authorization, fund custody, reentrancy, or signature/nonce validation. Security callouts should link to the relevant page under `docs/security/`.

## Tables

- Use tables for anything enumerable with 2+ consistent attributes per row (storage keys, function parameters, error codes, profile settings).
- Always include a header row.
- Keep cell content short; move long explanations to prose below the table.

## Diagrams

- Use [Mermaid](https://mermaid.js.org/) fenced blocks (` ```mermaid `) for diagrams — flowcharts for control flow, sequence diagrams for cross-contract calls, state diagrams for status enums like `InvoiceStatus` or `EscrowStatus`.
- Keep diagrams focused on one concept. If a diagram needs a legend to explain unrelated symbols, split it.
- Every diagram needs a one-sentence caption above it stating what it shows.

## Terminology

Use these terms consistently across all documentation, matching the domain language used in the Rust source (see [`docs/glossary.md`](../glossary.md)):

- **merchant** — a registered business/individual accepting payments through Shade. Do not use "seller," "vendor," or "business" interchangeably.
- **payer** — the party paying an invoice. Do not use "buyer" or "customer" except where the code itself uses that term (e.g. escrow's `buyer` field).
- **invoice** — a payable request created by a merchant. Do not use "bill" or "charge" for this concept.
- **admin** — the contract administrator role (`DataKey::Admin`). Do not use "owner" or "superuser."

**Link the first use of any glossary term on a page to [`docs/glossary.md`](../glossary.md)** (e.g. `[merchant](../glossary.md#merchant)`). Subsequent uses on the same page don't need to be linked.

## Review checklist

A docs PR must satisfy all of the following before merge:

- [ ] **Accuracy against code** — every signature, type, storage key, and behavior claim was checked against the current source, not memory or an older doc.
- [ ] **Working links** — every relative link and line-range link resolves; no link points at a renamed or deleted file.
- [ ] **Spell-check** — page has been run through a spell-checker; domain terms match [`docs/glossary.md`](../glossary.md) spelling exactly.
- [ ] **Template compliance** — the page uses the correct template from [`docs/contributing/templates/`](templates) for its page type, with all required sections present.
- [ ] **Terminology** — first use of each glossary term links to `docs/glossary.md`; terminology is consistent with [Terminology](#terminology) above.
- [ ] **Style compliance** — sentence-case headings, tagged code blocks, second-person present-tense voice.
- [ ] **Index updated** — new pages are linked from the relevant `docs/<section>/README.md` and, if top-level, from [`docs/README.md`](../README.md).
