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

For Rust-only work, run `cargo test` from `server/` in addition to focused
tests. For extension-facing TypeScript, language-client, or bundled-server
changes, run `npm run compile` after the final source edit.

The Rust development profile retains debug information and assertions but uses
optimized code. This keeps the bundled development language server
representative enough for large-file editor latency while preserving the
release profile as the packaging authority.

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
