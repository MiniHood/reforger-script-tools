# server/

## Purpose

Owns the Rust language-tooling foundation for Reforger Script Tools.

## Architecture Role

This folder is the future language-engine side of the project. It should contain compiler-style language intelligence that must stay out of the TypeScript VS Code shell. Current slices include the lexer, the full-fidelity parser scaffold, the first syntax-backed AST declaration wrapper layer, the first file-local declaration model/symbol catalog layer, and the first in-memory symbol index layer.

## Current Behavior

The Rust crate exposes a full-fidelity lexer that returns token kinds and byte spans without copying source text into tokens. It also exposes a parser that consumes lexer tokens and returns a full-fidelity syntax tree preserving trivia, tokens, declarations, callable-body statements, and expressions. The AST layer provides file-local declaration views over that syntax tree without semantic resolution. The model layer converts those AST views into source-backed file-local symbol records with stable IDs and parent-child relationships. The scope layer builds lexical callable/block scopes for indexed parameters and locals. The index layer aggregates many file-local catalogs into global symbol handles and lookup maps without semantic resolution. The index build layer owns reusable source-root scanning, parser/AST/model/index construction, and build timing summaries. The index cache layer owns disposable runtime cache loading/rebuilding for game-data indexes. The index query layer wraps the raw index with editor-facing lookup APIs so future LSP features use preferred, overlay-aware paths instead of debug aggregate maps by accident. The type-facts layer exposes raw indexed type/detail facts through a stable read-only API for future semantic work. The expression-type layer owns source-backed expression owner/type inference for resolver receiver chains, including raw type text, collection/index facts, typedef/base expansion, primitive values, auto locals, and generic substitution. The resolver layer maps identifier cursor positions to candidate symbols and source-backed selection reasons for hover, definition, member completion, and type/top-level prefix completion features. The reference-finder layer scans file-local identifier tokens and keeps only resolver-confirmed references to an exact symbol id for future references/rename work. The symbol display layer converts copied indexed facts into editor-ready labels, details, signatures, documentation previews, and provenance output. The LSP layer is the first stdio protocol boundary for VS Code and currently exposes document symbols, resolver-first hover, resolver-first definition, and source-backed completion over open-file symbols plus a background-loaded external overlay of live workspace symbols and cached game-data symbols.

## Dependencies and Boundaries

The crate currently uses only minimal bundled Rust dependencies for JSON protocol support. It must not import VS Code APIs, implement extension activation, perform Workbench downloads, or mix LSP behavior into lower compiler-style layers.

## Change Notes

- Added the initial `server/` crate as a single focused Rust library for language tooling.
- Added lexer-only tokenization for identifiers, keywords, literals, trivia, punctuation, operators, preprocessor marker tokens, and unterminated string/comment errors.
- Added the first parser scaffold for declaration-level syntax, full-fidelity token preservation, parse diagnostics, and parser fixture reporting.
- Added statement and expression parsing inside callable bodies plus dev-only expression fixture/corpus reports.
- Added the first AST declaration wrapper layer over parser syntax nodes.
- Added the first file-local model/symbol catalog layer over AST declarations.
- Added the first in-memory symbol index over file-local catalogs.
- Added the reusable index build pipeline for explicit game-data/workspace source roots.
- Added disposable game-data index cache support for runtime LSP hover.
- Added the first editor-facing index query facade over the raw symbol index.
- Added the first indexed symbol display layer for shared hover/completion/debug presentation facts.
- Added the first identifier-only reference resolver scaffold for future hover and definition behavior.
- Added a dev-only index build baseline report for measuring debug/release game-data index construction cost without corpus report rendering.
- Added the first stdio LSP scaffold and bundled binary entrypoint with document-symbol support.
- Added a dev-only LSP fixture report for reviewing document-symbol output across committed parser fixtures.
- Added a dev-only LSP corpus report for reviewing document-symbol projection over downloaded game-data scripts.
- Added file-local and external game-data LSP hover support plus a dev-only hover fixture report.
- Added a dev-only LSP hover corpus report for sampled corpus-scale resolver-first hover review.
- Added resolver-backed LSP definition support plus a dev-only definition fixture report.
- Added live runtime workspace index overlay support for hover and definition.
- Added member, type-prefix, and top-level-prefix LSP completion support plus a dev-only completion fixture report.
- Added parser diagnostic projection for LSP publishDiagnostics plus a dev-only diagnostics fixture report.
- Added the first read-only type-facts layer over copied indexed type/detail text.
- Added the expression-type environment and corpus report over source-backed type facts used by resolver receiver inference.
- Added a lexical scope model and dev-only scope corpus report for proving local/parameter visibility over game-data bodies.
- Added the first file-local reference finder and fixture report for resolver-backed reference/rename groundwork.

## Future Improvements

- Expand LSP behavior through small verified features such as diagnostics, completion, and references.
- Route future hover expansion and Ctrl+click definition through the resolver layer instead of adding protocol-local lookup shortcuts.
- Keep future expression typing improvements in `expression_type` or a later semantic/type facts layer rather than moving type inference back into resolver.
- Keep future references and rename routed through `reference_finder` / resolver-confirmed symbol ids rather than text-only matching.
- Consider splitting into `crates/` only if multiple Rust crates become necessary.
