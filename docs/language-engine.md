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

For control-header keyword completions (`if`, `for`, `foreach`, `while`, and
`switch`), Rust owns the parenthesized snippet and an opaque caret-local
Space-commit contract. The TypeScript client may remove only the single
committed Space identified by that contract, whether VS Code applies it before
or after the snippet edit; it must not infer or rewrite ordinary source.

The Enter typing-assist request is a bounded structural edit, not a formatter.
For `for`, `foreach`, `while`, and `switch` headers with a matched closing
parenthesis, Rust may append a braced body only while the caret remains on the
header's physical line, preserving the header exactly.
It declines existing-brace, non-header, comment/string, multi-caret, stale, and
disabled-setting cases. A generated `switch` body begins with a Rust-authored
`default` snippet: typing replaces its selected arm, while Tab retains it and
moves to the body. At that arm, Rust offers the structural `case value` snippet
and opens ordinary value completion for its selected value. The client owns only
applying the returned versioned edit and snippet, never inferring source shape.

Collection type completion is a parser/resolver-backed type-position feature.
`array<T>`, `set<T>`, and `map<K, V>` insert snippets with selected type slots;
the client opens ordinary type completion at each slot and, after the final
type is accepted, places the caret after the closing `>`. Exact and prefix
matches retain priority, then the engine ranks standard value types, `ref`,
nested collections, and indexed enums/classes. `void` is excluded from a
collection type argument. Recovery recognizes only an incomplete operand of
`new`, a lone prospective callable-parameter type before its required
parameter name, and an empty type slot of a supported built-in collection or
base-game tuple, so completion remains available while the user is
constructing these otherwise valid type positions.

Base-game `Tuple1` through `Tuple6` are a separate generic-class completion
family. Their source-defined arity supplies one through six type slots using
the same type-completion and final-caret bridge as collections; they do not
receive collection construction or declaration-tail behavior.

### Completion gotchas

- An empty generic slot is a type position, not a value position. Returning
  value completion there exposes statement keywords and hides the standard
  types, `ref`, and indexed classes.
- Do not grow a hard-coded owner-name list when another generic class is
  discovered. The base-game corpus includes generic classes beyond collections
  and tuples, so generic snippet arity and empty-slot recovery must ultimately
  come from the indexed generic-owner declaration. The current built-in
  collection and `Tuple1`–`Tuple6` recognition is a bounded compatibility
  bridge, with removal condition: replace it when generic-class metadata drives
  this path.
- Collection-only construction and declaration-tail behavior remains limited to
  `array`, `set`, and `map`; generic classes and tuples receive type slots only.

The collection declaration-tail owner is similarly bounded: it lexically
proves a complete single `array`, `set`, or `map` field/local (including nested
generic arguments such as `array<array<int>>`) and rejects all other contexts
before returning the one native Space edit plus a suggestion request. The tail
choices are Rust-authored completion edits, not a formatter or a client-side
post-edit rewrite.

Document-symbol responses enforce the LSP invariant that a symbol's full range
contains its selection range, including parser-recovery states. When recovery
requires that range repair, the server emits a bounded structured diagnostic
record with only structural range coordinates and symbol kinds; it never logs
source text or symbol names.

Run focused Rust tests while iterating and `cargo test` from `server/` for the
engine suite. Use the [development guide](development.md) for extension-level
verification.
