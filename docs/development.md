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
restarts the language server so its external index uses the new source. Wait for
that restart before judging game-API language features.

## Documentation Changes

For documentation-only changes, run `git diff --check` and verify changed
Markdown links and paths manually. Add documentation only for a lasting module
contract, workflow, decision, or reusable evidence format; route readers from
the [documentation index](README.md) instead of duplicating the same fact.
