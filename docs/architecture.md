# Architecture

## Purpose

The system separates VS Code integration from language understanding. The
extension shell owns editor and storage integration; the bundled Rust server
owns Enfusion language decisions. This boundary keeps editor behaviour useful
without creating a second language implementation in TypeScript.

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
notifications. Rust turns them into immutable language facts; the client
transports or presents the resulting editor behaviour. Game-data acquisition
only resolves source material—it does not analyse Enfusion.

The packaged executable also has an independent MCP mode. An MCP client starts
its own local `stdio` process; it neither attaches to the editor-owned LSP nor
requires VS Code to remain running. LSP and MCP reuse the same Rust language
and evidence modules, so they do not establish competing semantic authorities.
The generated [MCP API Reference](mcp-api.md) is the public tool contract.

## MCP and Workbench Boundary

MCP tools may combine bounded project-file facts, language-engine facts, and
packaged evidence. Each result must identify its source and must not present a
file-derived fact as live Workbench state.

Workbench is the authority for running-editor and engine facts. Its NET API is
a private route to Workbench, never a second public MCP server or a generic
handler proxy. A missing or incompatible Workbench integration makes only the
affected live capability unavailable; it must not block offline language or
evidence tools.

```mermaid
flowchart LR
    Client[MCP client] --> Host[Local MCP runtime]
    Host --> Rust[Rust language and evidence modules]
    Host --> Files[Bounded project-file access]
    Host --> Gateway[Rust typed Workbench Gateway]
    Gateway --> Workbench[Running Reforger Workbench]
    Workbench --> Plugin[Versioned profile handler package]
```

The Rust Workbench Gateway exposes named, typed capabilities and is the only
owner of NET API framing. MCP calls it directly. The existing TypeScript
compiler integration invokes the packaged Rust executable through its private
`workbench-api` process mode, so it remains a thin editor-facing bridge rather
than a second codec. Detailed protocol evidence and compiler-validation
acceptance remain in the relevant research journals.

The optional managed handler package lives under the current Windows user's
`Documents\My Games\ArmaReforgerWorkbench\profile\scripts\reforger-script-tools`
directory. Its manifest is both file ownership and continuing-maintenance
consent: after a successful native connection, the VS Code extension owns the
one-time first-install prompt and invokes a private Rust installer only when the
user accepts. Public MCP cannot create that first manifest; it may maintain an
existing consented installation. A successful later connection may likewise
repair or upgrade only manifest-owned files. Writing that profile package and
running native compiler validation does not register its `NetApiHandler`s in
the already-running Workbench; the extension reports successful installation
and asks the user to refresh Workbench with `Ctrl+Shift+R`. It deliberately
does not probe the just-written capability handler before that refresh.
When no consent manifest exists, status reports installation as available only
if the existing profile and native connection make the approval-bearing
operation usable; status itself creates nothing.
Unknown profile files are preserved, newer package versions are never
downgraded, and failed activation is left installed for diagnosis rather than
rolled back. Version precedence follows semantic-version ordering; an
unrecognized installed version is preserved because automatic downgrade safety
cannot be proven.

Compiler validation is captured once per invocation and exposed as bounded,
opaque-cursor pages so an MCP client can retrieve every finding without
recompiling between pages. Process shutdown is bound to a process identity that
includes both PID and observed start time; only graceful main-window close is
supported.

Every failed public Workbench operation returns a unique support reference.
The same reference is written to the default-on rotating integration log with
the operation, stable outcome, timing, versions, and logical managed filenames
needed for diagnosis. Raw NET API payloads and source text are not logged.

## Module Boundaries

| Module | Owns | Must not own |
| --- | --- | --- |
| `src/extension.ts` | Activation and top-level wiring | Language behaviour or game-data workflows |
| `src/extensionConfig/` | Extension-facing names, defaults, and limits | Runtime logic |
| `src/gameData/` | Game-data acquisition and source resolution | Parsing or semantic analysis |
| `src/languageClient/` | Server lifecycle, transport, file notifications, and thin editor bridges | Syntax, lookup, completion ranking, or type reasoning |
| `src/mcp/` | MCP client configuration from the packaged runtime and stable source/cache inputs | Protocol serving, indexing, or semantic queries |
| `src/workbenchNetApi/gateway/` | Thin TypeScript process bridge from editor compiler features to the bundled Rust Workbench Gateway | NET API framing, VS Code UI, raw endpoint dispatch, or Enfusion language decisions |
| `src/workbenchNetApi/compiler/` | VS Code scheduling, compiler diagnostic rendering, and Workbench status UI | NET API framing, endpoint discovery, or language-engine diagnostics |
| `src/workbenchNetApi/integration/` | One-session first-install prompt and progress/notification presentation after a confirmed connection | Profile writes, consent persistence outside the manifest, NET API framing, or automatic process lifecycle |
| `server/src/bin/reforger_language_server.rs` | Process-mode parsing and dispatch to one protocol adapter | Protocol behaviour, language analysis, or tool definitions |
| `server/src/lsp/` | LSP transport, document lifecycle, and language-feature projection | MCP serving or a second Enfusion analysis implementation |
| `server/src/mcp/` | MCP schemas, protocol serving, and bounded result mapping | LSP lifecycle or a second Game Data/Official Wiki authority |
| `server/src/workbench.rs` | Workbench discovery, process lifecycle, NET API framing, native capabilities, managed handler lifecycle, and bounded support logs | VS Code UI, arbitrary handler dispatch, force termination, or Enfusion language analysis |
| `server/src/*.rs` (except protocol adapters) | Shared Enfusion analysis, evidence catalogues, indexes, formatting, and diagnostics | VS Code UI, settings, or client-protocol ownership |
| `tools/` | Development and investigation support | Extension runtime behaviour |

`src/extension.ts` composes modules; it is not a feature owner. Workbench
compiler diagnostics are extension-owned evidence, separate from Rust parser
diagnostics. They may be consumed by an MCP Workbench adapter, but never used
to emulate compiler facts from files.

## Engine Invariants

- Rust is the one Enfusion language authority.
- Open documents and external indexes are revisioned immutable snapshots; a
  request uses facts from the snapshot it captured.
- TypeScript bridges transport Rust-authored facts or apply editor behaviour;
  they do not classify source.
- Evidence follows the source hierarchy in [the system overview](overview.md).
- Workbench capabilities are typed and versioned; raw NET API handler dispatch
  is not an extension point.

Exact algorithms, scheduling, protocol framing, cache behaviour, and feature
results belong to code and tests. This document records the boundaries that
make changes to those details safe.
