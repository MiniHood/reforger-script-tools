# Architecture

## Purpose

The system separates VS Code integration from language understanding. The
extension shell owns editor and storage integration; the bundled Rust server
owns all Enfusion language decisions.

## Runtime Flow

```text
VS Code editor
  -> TypeScript extension shell
  -> TypeScript language-client bridge
  -> bundled Rust language server
  -> LSP results
  -> VS Code editor
```

Game data and workspace scripts enter through resolved paths and document/file
notifications. The server turns them into immutable language facts; the client
renders or transports the resulting editor behavior.

When the extension installs game data or the user selects a manual source, the
top-level wiring requests a language-server restart. The replacement server then
builds its external game-data layer from that source; game-data acquisition does
not perform language analysis itself.

## Module Boundaries

| Module | Owns | Must not own |
| --- | --- | --- |
| `src/extension.ts` | Activation and top-level wiring | Language behavior or game-data workflows |
| `src/extensionConfig/` | Extension-facing names, defaults, and limits | Runtime logic |
| `src/gameData/` | Game-data acquisition and source resolution | Parsing or semantic analysis |
| `src/languageClient/` | Server lifecycle, transport, file notifications, and thin editor bridges | Syntax, lookup, completion ranking, or type reasoning |
| `server/` | Language analysis, external indexes, formatting, diagnostics, and LSP results | VS Code UI, settings, or game-data acquisition |
| `tools/` | Development and investigation support | Extension runtime behavior |

`src/extension.ts` composes these modules; it is not a feature owner.

Within the language-client bridge, the composition root retains server lifecycle
and restart policy. Focused bridges own workspace-script notifications, hover
rendering, and diagnostic command UI. Each bridge transports Rust-authored
facts or applies editor behavior; none interprets Enfusion source.

Within the LSP, protocol framing, operational logging, request-local
document-query admission, and feature projection are separate concerns. A
document query captures both the open-document snapshot and the external-index
snapshot at the request boundary so downstream feature code cannot accidentally
combine facts from different generations. Logging is best-effort observation;
it cannot participate in request admission or response delivery.

## Language Engine

The Rust engine is organized as a compiler-style pipeline:

```text
source text
  -> lexer and parser
  -> syntax and semantic file
  -> scopes and symbol indexes
  -> resolver/type facts
  -> LSP feature projection
```

Features such as completion, hover, definition, signature help, diagnostics,
semantic tokens, and formatting consume shared engine facts. They must not
create separate language models or move language rules into TypeScript.

## State and Concurrency

Open documents are immutable, revisioned snapshots. The analysis runtime owns
their admission, cancellation, and publication. A feature may use facts proven
for the current snapshot, but must never pair current text with stale local
semantic facts.

Workspace and game-data indexes are separate immutable external layers. A
feature captures one layer snapshot for a request; background indexing may
publish a later generation without changing that request's meaning.

## Change Rules

- Put a change in the module that owns its contract.
- Preserve one language authority: Rust.
- Add a cross-module path only when a concrete contract requires it.
- Keep editor bridges event-driven and narrow; they transport or apply
  Rust-authored results rather than classify source.
- Keep expensive analysis out of extension activation and the editor UI path.
