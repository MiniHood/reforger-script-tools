# Development

This guide covers the repository's stable local verification paths. Choose the
smallest command that proves the change, then run broader checks when the
change crosses a boundary.

## Build and Test

For a fresh checkout, install a current Node.js LTS release and the Rust toolchain
(including Cargo), then run `npm ci` from the repository root. Rust is a
development requirement only; packaged extension users receive the built server.

From the repository root:

| Command | Verifies |
| --- | --- |
| `npm run check-types` | TypeScript type checking. |
| `npm run lint` | Extension-source linting. |
| `npm run compile` | Type checking, linting, the bundled Rust server, and the extension bundle. |
| `npm test` | Extension test setup and test suite; its pretest step compiles tests and runs the full compile path. |
| `npm run package` | Production Rust server and production extension bundle. |
| `npm run test:packaged-official-wiki` | Build a VSIX, verify every Official Wiki Markdown byte is packaged, and launch the installed MCP runtime from an unrelated working directory. |
| `npm run mcp-api:generate` | Regenerate the committed MCP API Reference from the live Rust descriptors. |
| `npm run mcp-api:check` | Fail when the committed MCP API Reference has drifted. |

For Rust-only work, run `cargo test` from `server/` in addition to focused
tests. For extension-facing TypeScript, language-client, or bundled-server
changes, run `npm run compile` after the final source edit.

### Cargo build artifacts

Keep every Cargo build cache beneath the ignored `server/target/` tree. Normal
Cargo commands use that directory automatically. When an investigation needs
an isolated cache, set `CARGO_TARGET_DIR` to
`server/target/tasks/<short-purpose>` instead of creating a sibling
`server/target-*` directory. These directories contain generated compiler
artifacts only, can grow by several gigabytes, and may be removed with
`cargo clean --manifest-path server/Cargo.toml --target-dir <target-path>`.

The development extension launches
`server/target/debug/reforger_language_server.exe`; `npm run compile` rebuilds
and restores it. Packaged installations use the executable copied beneath
`dist/server/`.

The Rust development profile retains debug information and assertions but uses
optimized code. This keeps the bundled development language server
representative enough for large-file editor latency while preserving the
release profile as the packaging authority.

### Developer report programs

Developer-only Rust reports and benchmarks live in `tools/server-reports/`,
outside the packaged server source tree. `server/Cargo.toml` declares them as
explicit example targets, so existing commands such as
`cargo run --manifest-path server/Cargo.toml --example <name> -- ...` remain
the supported way to run them. They are developer tooling and never a runtime
dependency of the extension.

The bundled executable selects a protocol before either protocol starts:

```powershell
# Existing LSP mode (default)
dist/server/win32-x64/reforger_language_server.exe

# One MCP stdio session owned by the launching client
dist/server/win32-x64/reforger_language_server.exe mcp `
  --game-data-scripts <scripts-path> `
  --game-data-metadata <optional-metadata-path> `
  --index-cache <global-storage-cache-path>
```

In VS Code, run **Reforger Script Tools: Copy MCP Configuration** and choose
Codex TOML or generic MCP JSON. The copied command contains absolute packaged
runtime and stable Game Data/cache inputs, so the client does not depend on a
running VS Code process. Restart that MCP process after Game Data changes.
After an extension upgrade, rerun the command and replace the client entry:
the versioned installed runtime path changes deliberately, and the extension
does not edit third-party client configuration itself.
The generated [MCP API Reference](mcp-api.md) is the inspectable agent-facing
contract; standard `tools/list` remains authoritative at runtime.

## Official Wiki Corpus

`resources/official-wiki/` contains copied Markdown from the official Arma
Reforger pages on `community.bistudio.com`. It remains the packaged source of
truth for the MCP Official Wiki authority; preserve each page's canonical
source URL in its H1 when updating it. Before redistributing an updated corpus,
review the upstream site terms and attribution requirements, retain required
notices, run the corpus validation/package tests, and verify the generated MCP
API reference. `wiki-index.md` is a rough AI navigation aid only and is never
authoritative runtime metadata.

For a repeatable release-binary LSP initialization measurement, run:

```powershell
node tools/lsp-startup-baseline.mjs `
  server/target/release/reforger_language_server.exe 7
```

For repeatable semantic-token latency checks against an installed game-data
index, use the dedicated benchmark example:

```powershell
cargo run --manifest-path server/Cargo.toml --example lsp_semantic_tokens_benchmark -- `
  --scripts <game-data-scripts-path> `
  --file <large-enfusion-file> `
  --iterations 7 `
  --max-median-resolver-ms <local-budget>
```

The command builds the external index once, warms the projection, reports
minimum/median/p95/maximum wall and resolver phase timings, and verifies that
token count, resolver-call count, and encoded-token fingerprint remain stable
between iterations. The optional latency budget makes the command fail for a
local regression loop; it is deliberately supplied by the caller rather than
treated as a portable machine-independent threshold.

For repeatable game-data cache startup measurements, use:

```powershell
node tools/index-cache-baseline.mjs --out <report-path>
```

The report compares warm-cache loading with a direct source rebuild in both
development and release profiles. It separates file read, binary decode, and
runtime-index construction time, and verifies that file, public-symbol,
parameter, local-variable-pruning, and lookup-map counts remain consistent.

For a live editor capture, generate the runtime report from the local
language-server log:

```powershell
node tools/lsp-runtime-performance-report.mjs --since-minutes 10
```

The first-response section distinguishes rich-token convergence from the
editor's later collection request and reports how often rich work finished
before that request. The extension diagnostic log records the corresponding
edit-to-middleware age and event-loop-turn delay without document paths, source
text, identifiers, or LSP payloads.

## Ticket Completion

Break a ticket into small, behavior-preserving implementation slices when that
reduces risk, but keep an explicit checklist of its full requirements. Verify a
slice before starting the next one; do not treat that verification or its commit
as ticket completion. Before handoff, compare the completed code and tests with
the whole ticket, run the applicable final checks, and record any editor or
Workbench validation that still requires a live session.

## Local Extension Session

The `Run Extension` launch configuration runs `npm run compile` before it starts
an Extension Development Host, so the bundled development server exists before
the client starts. Use `npm run watch` separately when iterating on TypeScript
or the extension bundle, then use the existing host for live editor checks.

The server build refreshes the development and packaged server binaries. It
also stops repository-owned running language-server processes before replacing
them, so verify the active development session after a server rebuild rather
than assuming a previous process reflects the change.

When game data is installed or a manual game-data folder is chosen, the client
restarts the language server so its external index uses the new source. The
**Reforger game data** progress notification remains visible through the index
phases and closes when the replacement index is published; wait for it to close
before judging game-API language features.

## Workbench Integration Verification

The deterministic Gateway tests run against a local TCP peer and verify real
wire framing, response decoding, typed failures, deadlines, and sanitized
outcomes. The compiler tests run in the Extension Development Host with
`src/test/workspace` opened as the single addon workspace. Use a focused
iteration command such as:

```powershell
npm run compile-tests
node esbuild.js
npx vscode-test --grep "Workbench Gateway|Workbench compiler validation" --timeout 15000
```

The full `npm test` gate remains required before completion.

Automated peers do not replace live Workbench acceptance. With NET API enabled
in Workbench, open the same addon folder in VS Code and verify the configured
endpoint (default `127.0.0.1:5775`) before relying on protocol assumptions.
Run a clean `WORKBENCH` validation, introduce and save a deliberate compiler
error, and confirm the reported file and line. Then edit again to observe a
stale finding and fix the error to observe atomic replacement. Also verify
disabled integration, a deliberately wrong configured port, and a save
conflict. Record any Workbench version-specific error-code, line-number, or
readiness behavior in the existing Workbench research journals before changing
the codec or diagnostic projection.

## Documentation Changes

For documentation-only changes, run `git diff --check` and verify changed
Markdown links and paths manually. Add documentation only for a lasting module
contract, workflow, decision, or reusable evidence format; route readers from
the [documentation index](README.md) instead of duplicating the same fact.
