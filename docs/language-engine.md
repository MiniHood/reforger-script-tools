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

Class-like language keywords, currently `string` and `vector`, retain keyword
spelling but are indexed runtime classes. Semantic tokens, hover, and
definition use that shared classification even while an editor line is an
incomplete declaration; scalar primitive keywords remain keyword-classified.

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

The semantic-token legend is a classification contract, not a color palette.
It uses the standard `function` token for both global and class functions while
retaining their distinct `Function` and `Method` symbol kinds inside the
language model. Reforger-facing custom types are `reforgerField`,
`reforgerPunctuation`, and `reforgerPreprocessor`; their VS Code supertypes are
`property`, `operator`, and `keyword`. Attribute expressions keep their
detailed class, function, enum-value, variable, and field classifications
rather than collapsing to a decorator token. The Rust engine emits no
foreground values. `package.json` owns the Enforce-qualified default palette,
which users may override through native VS Code semantic-token settings.
For hover Markdown, Rust emits the same semantic type names as inert
`data-semantic-token` role markers. The TypeScript hover bridge resolves and
applies the effective native foreground rules because VS Code does not project
editor semantic tokens into Markdown content. Rust still owns classification,
the client still owns presentation, and neither layer carries a second color
table.

Scope delimiters are one shared syntax projection used by semantic tokens and
the active-pair request. Parser-proven `{}`, `()`, `[]`, and generic `<>`
inherit the semantic token kind of their immediate owner; when a construct has
no distinct owner, it inherits the nearest enclosing semantic scope. This
keeps declaration bodies, control bodies, calls, constructors, indexing,
attributes, initializers, and nested generic types aligned with theme-defined
semantic colors without hard-coded foregrounds. Comments and strings remain
lexical, preprocessor directives are excluded, and comparison operators are
not reclassified as generic delimiters. A parser-proven unmatched opener may
be colored for recovery, but only matched pairs can become active.
Call, construction, and index anchors are admitted only after the resolver
selects a compatible symbol; unresolved or category-invalid candidates remain
`reforgerPunctuation`. Attribute delimiters inherit the attribute class
classification without changing the attribute expression's detailed token
roles.

`reforgerScriptTools.bracketColoring` selects one presentation for the whole
projection. `semantic`, the default, uses the owner classification above.
`punctuation` emits every code delimiter as `reforgerPunctuation` while
retaining parser-proven active-pair matching.
`vscode` omits code delimiters from the custom semantic-token projection so
VS Code owns both their foreground colors and matching presentation. No mode
hard-codes a foreground color in Rust, so the manifest palette and user
customization remain authoritative.

`reforger/activeScopeDelimiters` is a bounded, version-aware projection for all
current carets. It selects the innermost matched semantic pair at each caret,
coalesces duplicate pairs, and may use the current foreground parse while
whole-document semantic analysis is pending. The response contains the document
version, foreground-readiness state, and delimiter ranges; semantic ownership
and classification remain in Rust tokens while VS Code owns their foreground
presentation. The editor bridge retries a current pending snapshot until its
foreground projection is ready. While a request or foreground retry is
pending, the bridge preserves the settled active-pair decoration; only a
current terminal response atomically replaces or clears it. A stale response
does not alter the visible pair. A rejected foreground task returns a terminal
empty result rather than a retry signal.
After every source edit, the current snapshot's lexical baseline is cached as
an internal input to rich projection, but it is not published as an interim
full-document response. The semantic-token request remains pending while the
already-scheduled foreground and semantic workers produce the current rich
overlay, then publishes that overlay as one replacement. This keeps the
editor's settled semantic presentation visible instead of briefly replacing it
with lexical or theme-default colors. A newer edit, an external-index
generation change, worker overload, or document close rejects the superseded
pending request with `Content modified`; waiting is revision- and
generation-bounded and never moves parsing or resolution onto the request
path.
For punctuation and native VS Code modes, that baseline consumes parser-proven
generic-angle offsets only when the foreground worker has published them for
the current snapshot. While foreground syntax is pending, angle operators keep
their lexical operator classification in the cached projection; the request
path neither reparses nor reuses stale delimiter facts.
New text typed beside or inside a delimiter therefore cannot inherit that
delimiter's foreground while the richer projection is pending.
Active-pair requests decline documents larger than 128 KiB and cap caret input.
Pair selection depends only on parser-proven structure, so foreground and
analyzed snapshots return the same active ranges; resolver-dependent foreground
classification remains `reforgerPunctuation` until matching analysis exists.
Background semantic projection retains its existing cancellation contract and
bounded token output.

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
parameter name, and an empty slot of an indexed generic class, so completion remains available while the user is
constructing these otherwise valid type positions. The same prospective
parameter-type recovery runs on the current-snapshot completion lane before
argument-label lookup, so its initial character receives the generic snippet
rather than a plain indexed class completion.

Indexed generic classes use their declared type-parameter count to supply type
slots through the same type-completion and final-caret bridge as collections.
That includes base-game tuples and game-defined generic classes such as
`SCR_BTParam<T>`; only collections receive collection construction or
declaration-tail behavior.

Constructor completion is a semantic projection at a proven `new` operand.
It begins on the exact partial keyword prefixes `n` and `ne`, remains active
on a bare `new`, and continues after the operand space. This lets the first
word-completion request carry the full constructor preview while VS Code
locally filters the same list through the remaining keyword characters. The
server also advertises Space as a completion trigger, but returns a list for
that trigger only when the current snapshot establishes one contextual type
from a declaration, resolved assignment target, callable return, call
argument, or collection-initializer element. The exact accessible class is
preselected; compatible classes with accessible constructors follow.
Inaccessible or unindexed contextual classes have no default, but compatible
constructible subtypes remain available. Dynamic collections preserve their
complete generic spelling.

Constructor edits contain only the parenthesized expression. Required
parameters become ordered named snippet fields, optional parameters are
omitted, and a class without an indexed constructor signature remains
available as `Type()`. The label plus label details previews that accepted
source text. When completion is invoked on a partial or bare typed `new`, the
item instead previews the whole `new Type(...)` expression and atomically
replaces the typed prefix; after `new `, it edits only the operand. No
constructor item owns a semicolon, assignment, initializer brace, or fluent
suffix. Space-triggered requests outside this exact context return an empty
complete list; manual prefix and explicit completion retain their existing
behavior.

When a user retypes an already-present completion label, Rust keeps the
existing syntax authoritative. A type completion immediately before a generic
closer replaces that closer and uses its final tabstop; this also handles the
inner half of a lexed `>>` closer. A callable or control-header completion
replaces an immediately following empty `()` with its authored snippet, while
a non-empty existing argument list is preserved and only the label changes.
Active named-argument labels already insert only their name, preserving a
following `:`. Plain values and members do not receive structural replacement
edits because they do not author a structural suffix.

### Completion gotchas

- Inside an overriding method, the `super` prefix is an override-aware callable
  completion rather than a plain keyword: it inserts the matching indexed base
  call, including required parameter placeholders (for example,
  `super.OnPostInit(${1:owner})`). The bounded current-snapshot path uses the
  enclosing class/method shape and external index signature so this remains
  available before whole-document analysis publishes; the analyzed path
  produces the same result.

- A completion produced without matching current-document semantic facts is a
  provisional fallback, even when it has fewer items than the normal cap. It
  must set LSP `isIncomplete: true` and preserve that state through every
  merge. Otherwise VS Code can retain a plain callable label after analysis is
  ready, hiding the callable snippet's parentheses, parameters, and follow-up
  command. Treat `isIncomplete` as result-fidelity metadata, not only as an
  overflow signal.
- Normal `textDocument/completion` requests wait for and replay against the
  matching document analysis. Bounded foreground recovery may prove names,
  but it cannot generally prove callable signatures; sending that partial list
  first lets a plain identifier completion race and replace the parameterized
  snippet in the UI. Keep that recovery for explicitly provisional paths and
  debug inspection, never as the final normal-completion response.
- An active positional `func` parameter is slot metadata, not a function
  candidate. Exclude its parameter-label item before argument labels merge
  with ordinary value completion; do not solve that category error by adding
  an RPC-specific rank or changing the general completion ranking.
- An empty generic slot is a type position, not a value position. Returning
  value completion there exposes statement keywords and hides the standard
  types, `ref`, and indexed classes.
- Do not grow a hard-coded owner-name list when another generic class is
  discovered. Generic snippet arity and empty-slot recovery come from the
  indexed generic-owner declaration. Built-in collection keyword completion
  remains a lexical bridge because those names are language keywords as well
  as source-defined classes.
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
