# server/

## Purpose

Owns the Rust language engine: compiler-style language understanding, external
source indexing, and Language Server Protocol handling for Reforger Script
Tools.

## Ownership

The crate owns lexing, parsing, syntax/AST views, file-local models, scopes,
symbol indexes, type and resolver facts, diagnostics, formatting, and LSP
feature projection. `src/lib.rs` composes these layers; `src/lsp.rs` owns the
stdio protocol boundary and `src/lsp/` owns feature and lifecycle mechanics.
`examples/` contains developer-facing reports that exercise the same engine.

It does not own VS Code activation or UI, user settings, server-process
management, Workbench downloads, or game-data acquisition. Those concerns stay
in the TypeScript extension and its `gameData` and `languageClient` owners.

## Current Behavior

The lexer produces full-fidelity tokens and byte spans. The parser preserves
trivia and malformed input in a concrete syntax tree and emits diagnostics; AST
wrappers expose declaration-oriented views without reparsing. Model, scope, and
index layers turn those views into source-backed symbols and lexical visibility
facts. Type, expression-type, resolver, reference-finder, and symbol-display
layers provide the semantic facts shared by editor features.

The LSP layer maintains versioned open-document analysis, projects parser
diagnostics and document symbols, and serves resolver-backed hover, definition,
completion, signature help, and semantic tokens. It combines file-local facts
with a background-maintained overlay of workspace and game-data indexes. Feature
responses use LSP UTF-16 positions and reject stale background results rather
than allowing an old document or overlay revision to replace current state.

Formatting remains a planned engine capability; its intended boundary is
documented in [formatting.md](server/src/formatting.md).

## Dependencies and Boundaries

The crate has minimal Rust dependencies for serialization and JSON protocol
support. Lower compiler-style layers must not depend on LSP types or VS Code.
LSP adapters translate protocol data at the boundary and delegate language
decisions to the engine layers. External source roots and cache locations arrive
as resolved inputs; indexing consumes them but does not discover or download
them.

## Verification

Run Rust verification from `server/` with `cargo test`. Feature-specific
fixtures and report binaries under `examples/` are developer investigation
tools, not an alternate runtime path. Validate uncertain Enfusion Script
behavior with Workbench/compiler evidence before changing parser or semantic
claims.

## Future Direction

Add language features as small verified vertical slices over shared parser,
model, resolver, and index facts. References and rename must use
resolver-confirmed symbol identities rather than text matching. Keep future
type-inference work in semantic/type layers, and split the crate only when a
concrete multi-crate ownership boundary exists.
