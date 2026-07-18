# src/languageClient/languageClient.ts

## Purpose

Owns VS Code-side startup and shutdown of the bundled Rust language server.

## Architecture Role

This file is TypeScript shell code. It resolves the packaged or development server binary, configures `vscode-languageclient`, passes extension-owned paths to the server, and starts the LSP process. Serious language intelligence remains in Rust.

## Current Behavior

On activation, the module creates a VS Code log output channel and a separate hover-debug output channel. In Extension Development Host mode, it resolves `server/target/debug/reforger_language_server(.exe)` first so newly compiled server changes are used without depending on the packaged `dist` copy. Outside development mode, it resolves `dist/server/<platform>-<arch>/reforger_language_server(.exe)` first and falls back to the development binary only when needed. It starts the server over stdio for file documents whose VS Code language id is `enforce`. The client enables VS Code Markdown HTML support for LSP hover content so Rust-produced colored kind labels can render; TypeScript does not generate or classify hover content.

The module writes TypeScript-side startup timing records to `globalStorageUri/logs/language-client-startup.log` as JSON lines. Records are session-stamped and cover activation start/end, language-client registration, server path and launch argument preparation, language-client construction, server process start request, first initialize response, first Enforce document opened, and first semantic-token response. This log is separate from the Rust `language-server.log`; it exists to identify extension-host startup and protocol-boundary delays without adding Rust analysis noise.

In Extension Development Host mode, the module watches the resolved development server binary. When `npm run compile` replaces that binary, the client stops the current language-server process and starts a new one against the updated executable. This is development-only restart plumbing; marketplace/runtime users still use the packaged binary path without a binary watcher.

The language client uses a custom `vscode-languageclient` error handler with the same restart policy as the library default: restart up to four times, then stop after the fifth crash inside a three-minute window. The final user-facing notification is intentionally concise: `Reforger Script Tools Language Server Crashed`. The handler shows this notification itself and marks the close result as handled so the language-client library does not display its default long crash text. Detailed failure information remains in the language-client output channel and the Rust language-server log.

The `enforce` language id is contributed through `package.json` and is path-associated only for `.c` files under `Scripts/` or `scripts/`. The language client should target the language id, not duplicate path-glob logic.

The client passes `globalStorageUri/logs/language-server.log`, `globalStorageUri/index-cache/game-data-symbol-index.v9.bin`, and the resolved game-data scripts path to the server. The game-data path uses the manual-folder setting when present, otherwise the downloaded global-storage `game-data/scripts` folder. For downloaded game data, the client also passes `game-data/metadata.json` so the Rust cache can invalidate by commit SHA. For manual folders, metadata is omitted and Rust uses a file-metadata fingerprint. The v9 binary runtime cache prunes external game-data local variables, strips source-only detail spans, rebuilds lookup maps after load, stores repeated strings through an interned string table, stores an explicit index-shape marker, preserves compacted per-file symbol ranges, and preserves parameters and declaration facts used by hover/signature-style display.

LSP hover content comes from Rust as Markdown. The built-in language-client hover provider is suppressed by middleware, and this module registers one explicit VS Code hover provider that sends the standard `textDocument/hover` request to the Rust server, then converts the returned Markdown into fresh `MarkdownString` objects with `supportHtml` enabled. This is a rendering bridge only: TypeScript must not create hover text, classify symbols, or duplicate Rust language analysis. It exists because VS Code hover color uses sanitized Markdown HTML spans, not semantic-token coloring inside hover code blocks.

Rust hover Markdown may include trusted `command:` links for source-backed symbol targets. This module registers `reforger-sript-tools.openSymbolLocation` as a thin editor command that opens the URI supplied by Rust and converts Rust byte offsets into VS Code positions from the target document text. The command must not resolve names, inspect syntax, or guess targets; it only navigates to target metadata produced by the language server.

The client discovers workspace script roots from each VS Code workspace folder. If a workspace folder itself is named `Scripts` or `scripts`, that folder is passed directly; otherwise existing `Scripts/` and `scripts/` children are passed as repeatable `--workspace-scripts` arguments. The client also registers file-system watchers for workspace `.c` files under those script folders. Create/change/delete events are debounced and sent to Rust as `reforger/workspaceFileChanged` or `reforger/workspaceFileDeleted`; TypeScript sends full file text but does not parse it. Each captured event carries a monotonically increasing sequence for an absolute, lexically normalized path key (with Windows case normalization). The sequence survives debounce and delayed reads, so an older read cannot restore a file deleted or replaced by a later event; watcher registration creates a fresh sequence state for each language-client lifecycle.

The module also registers `Reforger Script Tools: Debug Hover At Cursor` and `Reforger Script Tools: Debug Completion At Cursor`. The hover command sends the active Enforce editor URI and cursor position to the Rust server through the custom `reforger/debugHover` request, writes the returned report to the hover-debug output channel, and overwrites `globalStorageUri/logs/hover-debug/latest.md`. The completion command sends the same cursor shape through `reforger/debugCompletion`, writes the returned report to the completion-debug output channel, and overwrites `globalStorageUri/logs/completion-debug/latest.md`. The completion debug report intentionally includes both autocomplete and Signature Help context so Ctrl+F2 is the single command for callable editing troubleshooting. These files are single-record debug artifacts; each command run replaces its matching file completely so Codex and humans have one stable place to inspect the latest hover, autocomplete, or signature-help pipeline state. TypeScript does not inspect source text or duplicate language analysis.

The package contribution activates the extension directly for the hover and completion debug commands. `Ctrl+F1` is scoped to Enforce editor text focus. `Ctrl+F2` is scoped to Enforce editor text focus and to the visible suggest widget, so completion troubleshooting can be run while autocomplete owns focus.

VS Code does not always request LSP completion after text deletion or plain identifier insertion in custom-language files. The client therefore listens for deletion edits and single identifier-character insertions in Enforce documents and, after a short debounce, runs VS Code's normal `editor.action.triggerSuggest` command when the changed document is still the active editor and the cursor is still in a likely code position. The trigger bridge avoids obvious comments and strings with lightweight editor-side checks, but it does not decide candidates. Rust completion remains the single source for candidate context, lookup, ranking, and item rendering, and it still returns no items for invalid contexts such as comments or strings.

Rust completion items may include an extension-owned completion follow-up command for enum placeholders inside callable snippets. `reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholderEnd` preserves a selected `EnumOwner.` placeholder, orients the selection's active end after the dot, and runs normal suggest so Rust member completion sees the correct position while free typing can still replace the whole placeholder. Rust may also use the same command after accepting an enum member when that completion item appended the next required enum placeholder, such as `EnumOwner.Member, ${1:NextEnum.}`. The command only runs VS Code snippet navigation and normal suggest; it does not inspect source text, choose completion candidates, or duplicate Rust completion behavior. It waits briefly around snippet navigation because VS Code applies completion text and snippet state asynchronously. It writes one concise debug record to the language-client output channel when it runs or fails, and a small `completionFollowup...` timing record to `language-client-startup.log` so enum-placeholder command execution can be confirmed from global-storage logs.

## Dependencies and Boundaries

Uses VS Code APIs, Node path/filesystem APIs, `vscode-languageclient`, and extension config constants. It must not parse Enfusion Script, build indexes, inspect symbols, or implement language features directly.

Workspace file watchers and development binary watchers are client-owned process/editor integration. They must dispose old watcher registrations when the language client restarts so stale clients do not receive file-change notifications.

Startup timing logging must stay concise and protocol-boundary focused. It may record paths, counts, durations, and event names, but it must not serialize source text, ASTs, indexes, completion lists, or semantic-token payloads.

## Change Notes

- Added the first VS Code language-client startup path for the bundled Rust LSP server.
- Kept document selection conservative so the extension does not claim every `.c` file globally.
- Switched the client document selector to the contributed `enforce` language id.
- Added the cursor-position hover debug command that delegates analysis to the Rust server.
- Added the cursor-position completion debug command that delegates autocomplete analysis to the Rust server and writes a separate single-record completion debug report.
- Extended the completion debug command's Rust report so Ctrl+F2 includes Signature Help context without adding another command.
- Added command activation and suggest-widget keybinding coverage for the completion debug command so `Ctrl+F2` can run when autocomplete has focus.
- Changed development-host server resolution to prefer `server/target/debug` before the packaged `dist` binary, avoiding stale custom-request behavior while iterating on Rust LSP code.
- Added global-storage hover-debug report writing to `logs/hover-debug/latest.md`, overwriting the file on each command run.
- Added game-data index cache and metadata paths to the server launch arguments so runtime hover can use a cached external game-data index.
- Added workspace script root discovery and debounced file-watch notifications for the live Rust workspace overlay index.
- Updated the runtime game-data cache path to v8 after replacing the JSON payload with the binary runtime cache format.
- Updated the runtime game-data cache path to v9 after adding binary string-table storage.
- Added development-host binary watching so replacing `server/target/debug/reforger_language_server(.exe)` restarts the language client automatically.
- Added a custom language-client crash handler so repeated server crashes show the concise notification `Reforger Script Tools Language Server Crashed` instead of the default long `server crashed 5 times...` message.
- Enabled safe/trusted HTML rendering for LSP hover Markdown so Rust-produced colored kind labels display in VS Code. The built-in language-client hover provider is suppressed and replaced by an explicit provider that sends the same Rust `textDocument/hover` request, then rebuilds returned contents as HTML-capable Markdown strings; it must not build hover text or duplicate language analysis.
- Added the hover symbol-link command bridge for Rust-generated trusted Markdown links.
- Added narrow completion retriggers after deletion edits and identifier insertions in Enforce documents so editing a prefix can reopen normal LSP autocomplete without implementing completion in TypeScript.
- Added TypeScript-side startup timing records under `logs/language-client-startup.log` for activation, language-client construction/start, initialize completion, first document open, and first semantic-token response.
- Added enum placeholder completion command bridges that keep selected enum defaults replaceable while triggering member completion at the dot, and advance callable snippets to the next enum parameter after accepting an enum member. The bridges now force matching selected `EnumOwner.` placeholders to have their active side after the dot before triggering suggest, and log command execution to the client timing log for troubleshooting.

## Future Improvements

- Add user-facing restart/status commands only after there is a concrete need.
- Add richer runtime logging controls after the language server owns more expensive work.
- Add user-facing workspace index status only after there is a concrete need.
