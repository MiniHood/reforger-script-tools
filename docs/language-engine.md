# Language Engine

`server/` is the repository's language authority. It turns Enfusion source,
workspace scripts, and resolved game data into the language facts consumed by
LSP features. The TypeScript extension transports those results and applies
editor-only behavior; it does not duplicate parsing or semantic decisions.

## Contract

The engine accepts source snapshots and external source layers, then provides
diagnostics, formatting, symbols, completion, hover, definition, signature
help, and semantic tokens. Features project shared analysis facts instead of
building independent text-based models.

Its common analysis path is:

```text
source text
  -> lex and parse
  -> semantic file, scopes, and symbol index
  -> resolver and type facts
  -> LSP feature result
```

New language behavior belongs in the appropriate shared layer when more than
one feature can benefit. A feature-specific adapter is appropriate only when
it projects existing facts into an LSP response.

## Snapshot Rules

Open documents are immutable, revisioned snapshots. The analysis runtime owns
admission, cancellation, and publication of those snapshots. A request may use
local semantic facts only when they are known to match its current snapshot;
recovery-quality results are usable only where the feature explicitly permits
them.

Workspace and game-data indexes are immutable external layers. Each request
uses the layer snapshot it captured, even if background indexing publishes a
newer generation while the request is running. Do not introduce competing
revision tables or mutable shared feature state.

## Boundaries and Evidence

The engine owns language behavior, not VS Code UI, extension settings, or
game-data downloads. Enfusion behavior must be established from
Workbench/compiler evidence first; see the [system overview](overview.md) for
the complete evidence order.

For the `if` keyword completion, Rust owns the snippet and an opaque
caret-local Space-commit contract. The TypeScript client may remove only the
single committed Space identified by that contract, whether VS Code applies it
before or after the snippet edit; it must not infer or rewrite ordinary source.

Run focused Rust tests while iterating and `cargo test` from `server/` for the
engine suite. Use the [development guide](development.md) for extension-level
verification.
