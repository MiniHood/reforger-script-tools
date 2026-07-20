# src/languageClient/languageClient.ts

## Purpose

Owns VS Code-side startup and shutdown of the bundled Rust language server.

## Ownership

This file is TypeScript shell code. It resolves the packaged or development server binary, configures `vscode-languageclient`, passes extension-owned paths to the server, and starts the LSP process. Serious language intelligence remains in Rust.

## Current Behavior

On activation, the module creates a VS Code log output channel and a separate hover-debug output channel. In Extension Development Host mode, it resolves `server/target/debug/reforger_language_server(.exe)` first so newly compiled server changes are used without depending on the packaged `dist` copy. Outside development mode, it resolves `dist/server/<platform>-<arch>/reforger_language_server(.exe)` first and falls back to the development binary only when needed. It starts the server over stdio for file documents whose VS Code language id is `enforce`. The client enables VS Code Markdown HTML support for LSP hover content so Rust-produced colored kind labels can render; TypeScript does not generate or classify hover content.

The module writes TypeScript-side startup timing records to `globalStorageUri/logs/language-client-startup.log` as JSON lines. Records are session-stamped and cover activation start/end, language-client registration, server path and launch argument preparation, language-client construction, server process start request, first initialize response, first Enforce document opened, and first semantic-token response. This log is separate from the Rust `language-server.log`; it exists to identify extension-host startup and protocol-boundary delays without adding Rust analysis noise.

In Extension Development Host mode, the module watches the resolved development server binary. When `npm run compile` replaces that binary, the client stops the current language-server process and starts a new one against the updated executable. This is development-only restart plumbing; marketplace/runtime users still use the packaged binary path without a binary watcher.

The language client uses a custom `vscode-languageclient` error handler with the same restart policy as the library default: restart up to four times, then stop after the fifth crash inside a three-minute window. The final user-facing notification is intentionally concise: `Reforger Script Tools Language Server Crashed`. The handler shows this notification itself and marks the close result as handled so the language-client library does not display its default long crash text. Detailed failure information remains in the language-client output channel and the Rust language-server log.

The `enforce` language id is contributed through `package.json` and is path-associated only for `.c` files under `Scripts/` or `scripts/`. The language client should target the language id, not duplicate path-glob logic.

The client passes `globalStorageUri/logs/language-server.log`, `globalStorageUri/index-cache/game-data-symbol-index.v9.bin`, and the resolved game-data scripts path to the server. The game-data path uses the manual-folder setting when present, otherwise the downloaded global-storage `game-data/scripts` folder. For downloaded game data, the client also passes `game-data/metadata.json` so the Rust cache can invalidate by commit SHA. For manual folders, metadata is omitted and Rust uses a file-metadata fingerprint. The stable filename is content-versioned by Rust magic, not by its historical `.v9.bin` suffix: the current `RSTIDX11` payload stores one canonical binary public-fact record per file and rebuilds transient lookup maps after load. It prunes external local variables and source-only spans, interns repeated strings, stores an explicit index-shape marker, and preserves parameters and declaration facts used by hover/signature-style display. The Rust server may atomically convert exactly validated `RSTIDX10` or `RSTIDX09` bytes at that same path; the TypeScript client has no cache-format logic.

LSP hover content comes from Rust as Markdown. The built-in language-client hover provider is suppressed by middleware, and this module registers one explicit VS Code hover provider that sends the standard `textDocument/hover` request to the Rust server, then converts the returned Markdown into fresh `MarkdownString` objects with `supportHtml` enabled. This is a rendering bridge only: TypeScript must not create hover text, classify symbols, or duplicate Rust language analysis. It exists because VS Code hover color uses sanitized Markdown HTML spans, not semantic-token coloring inside hover code blocks.

Rust hover Markdown may include trusted `command:` links for source-backed symbol targets. This module registers `reforger-sript-tools.openSymbolLocation` as a thin editor command that opens the URI supplied by Rust and converts Rust byte offsets into VS Code positions from the target document text. The command must not resolve names, inspect syntax, or guess targets; it only navigates to target metadata produced by the language server.

The client discovers workspace script roots from each VS Code workspace folder. If a workspace folder itself is named `Scripts` or `scripts`, that folder is passed directly; otherwise existing `Scripts/` and `scripts/` children are passed as repeatable `--workspace-scripts` arguments. The client also registers file-system watchers for workspace `.c` files under those script folders. Create/change/delete events are debounced and sent to Rust as `reforger/workspaceFileChanged` or `reforger/workspaceFileDeleted`; TypeScript sends full file text but does not parse it. Each captured event carries a monotonically increasing sequence for an absolute, lexically normalized path key (with Windows case normalization). The sequence survives debounce and delayed reads, so an older read cannot restore a file deleted or replaced by a later event; watcher registration creates a fresh sequence state for each language-client lifecycle.

The module also registers `Reforger Script Tools: Debug Hover At Cursor` and `Reforger Script Tools: Debug Completion At Cursor`. The hover command sends the active Enforce editor URI and cursor position to the Rust server through the custom `reforger/debugHover` request, writes the returned report to the hover-debug output channel, and overwrites `globalStorageUri/logs/hover-debug/latest.md`. The completion command sends the same cursor shape through `reforger/debugCompletion`, writes the returned report to the completion-debug output channel, and overwrites `globalStorageUri/logs/completion-debug/latest.md`. The completion debug report intentionally includes both autocomplete and Signature Help context so Ctrl+F2 is the single command for callable editing troubleshooting. The Rust server executes both developer captures off its serialized LSP message loop, so a slow capture cannot block typing or normal language requests. These files are single-record debug artifacts; each command run replaces its matching file completely so Codex and humans have one stable place to inspect the latest hover, autocomplete, or signature-help pipeline state. TypeScript does not inspect source text or duplicate language analysis.

The package contribution activates the extension directly for the hover and completion debug commands. `Ctrl+F1` is scoped to Enforce editor text focus. `Ctrl+F2` is scoped to Enforce editor text focus and to the visible suggest widget, so completion troubleshooting can be run while autocomplete owns focus.

Rust owns the completion contract, including current-document admission,
context, candidate eligibility, ranking, and rendering. TypeScript ordinarily
does not invoke VS Code Suggest for document edits. It has two narrow,
protocol-driven UI exceptions: the Rust-selected `RplRpc` enum snippet, and a
refreshable empty LSP completion response followed by a deletion in the same
active Enforce document. The latter exists because VS Code can receive the
fresh shorter-prefix response after Backspace without reopening its native
widget. The client records only the response's `isIncomplete`/empty shape and
the editor's deletion event; it does not read source text, classify symbols,
choose candidates, or use a typing-path delay. It clears the marker before it
dispatches Suggest, so each empty response can cause at most one recovery
request. The snippet bridge still waits for each exact Rust-authored selected
enum default and invokes Suggest once per placeholder at the next event-loop
boundary; duplicate events and duplicate responses cannot create a competing
request. A 30-second cleanup timer only releases an abandoned snippet
transaction; it never triggers Suggest. While those transactions are active,
the client records bounded command/placeholder/request/response metadata
through the existing optional diagnostic logger. Response metadata includes
bounded completion-range shape counts, validity of any converted
insert/replace pairs, and filter-text lengths for at most three items; it never
records source text or completion payloads. This separates an absent response
from an invalid protocol edit, while fresh Extension Development Host
verification remains the authority for native widget visibility.

While the bridge transaction is active, the client wraps only that response's
completion-item commands. VS Code runs an item command after applying its
completion edit, so the wrapper preserves any original command and then invokes
the built-in next-snippet-placeholder command. Accepting an enum suggestion
therefore selects the next RplRpc argument immediately; it does not consume a
second Tab. Manually typed values retain normal VS Code snippet navigation.

Until the RplRpc multi-placeholder journey has been proven in a freshly loaded
Extension Development Host, that metadata also carries a temporary trace
version plus at most eight same-document rejected selection probes per
transaction. The probes record only counts, selected and expected lengths, and
the expected placeholder index—never source or selection text. OpenSpec change
`fix-rplrpc-enum-completion-contract` task 3.3 requires removing this forensic
trace before release.

Rust completion items may still use VS Code's built-in parameter-hints command after ordinary callable insertion.

## Dependencies and Boundaries

Uses VS Code APIs, Node path/filesystem APIs, `vscode-languageclient`, and extension config constants. It must not parse Enfusion Script, build indexes, inspect symbols, or implement language features directly.

Workspace file watchers and development binary watchers are client-owned process/editor integration. They must dispose old watcher registrations when the language client restarts so stale clients do not receive file-change notifications.

Startup timing logging must stay concise and protocol-boundary focused. It may record paths, counts, durations, and event names, but it must not serialize source text, ASTs, indexes, completion lists, or semantic-token payloads.

## Verification

Run `npm test`. For client lifecycle, watcher, or server-launch changes,
compile the Rust server, then reload the existing active Extension Development
Host in place and inspect the relevant global-storage log or debug report. Do
not open a replacement `--extensionDevelopmentPath` window or close the user's
host: that loses debug-session state and is not live verification of the
existing session. If the available tools cannot send an in-place reload command,
record live-host validation as pending.
