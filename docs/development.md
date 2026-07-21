# Development

This guide covers the repository's stable local verification paths. Choose the
smallest command that proves the change, then run broader checks when the
change crosses a boundary.

## Build and Test

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

## Local Extension Session

The `Run Extension` launch configuration uses the default build task and starts
an Extension Development Host. The default task runs the TypeScript and
extension-bundle watchers. Use it for live editor checks after compilation.

The server build refreshes the development and packaged server binaries. It
also stops repository-owned running language-server processes before replacing
them, so verify the active development session after a server rebuild rather
than assuming a previous process reflects the change.

## Documentation Changes

For documentation-only changes, run `git diff --check` and verify changed
Markdown links and paths manually. Add documentation only for a lasting module
contract, workflow, decision, or reusable evidence format; route readers from
the [documentation index](README.md) instead of duplicating the same fact.
