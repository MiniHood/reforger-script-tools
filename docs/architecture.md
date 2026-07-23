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
rendering, diagnostic command UI, development-server watching, completion UI
transactions, and typing-assist transactions. Typing-assist bridges share a
small versioned editor-edit transaction contract while retaining their own
trigger and Rust request policy. Each bridge transports
Rust-authored facts or applies editor behavior; none interprets Enfusion source.

Within the LSP, transport framing, incoming-message scheduling, request routing,
runtime scheduling, background-event publication, response writing, operational
logging, request-local document-query admission, and feature projection are
separate concerns. A document query captures both the open-document snapshot
and the external-index snapshot at the request boundary so downstream feature
code cannot accidentally combine facts from different generations. Logging is
best-effort observation; it cannot participate in request admission or response
delivery.

The Document Runtime owns the mutable lifecycle of open-document snapshots:
admission, cancellation, deferred document-backed work, semantic-token refresh
state, semantic-token projection selection, and query capture. Request routing
decodes supported lifecycle, document, workspace, and feature payloads into
typed commands before dispatch. Background-event interpretation and request
routing are short-lived coordinator contracts invoked by the composition root;
they do not own durable server state. The composition root is the only owner of
JSON-RPC framing and delivers typed runtime effects such as notifications and
asynchronous responses.

LSP tests live in a test-only child module rather than the production
composition root. They are organized by observable behavior domains—protocol,
documents, runtime, and features—and use framed LSP traffic by default. A
separate test module may use only narrow `#[cfg(test)]` seams where deterministic
runtime or document-state setup cannot be observed through the protocol.

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
- Prefer one authoritative implementation path. Use a fallback only as
  explicitly provisional recovery when the authoritative facts are proven
  unavailable; it must not compete with normal behavior.
- Generalize from semantic or structural facts. Do not branch on feature, API,
  or owner labels unless a proven distinction requires different behavior.
- Keep editor bridges event-driven and narrow; they transport or apply
  Rust-authored results rather than classify source.
- Keep expensive analysis out of extension activation and the editor UI path.
