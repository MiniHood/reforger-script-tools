# tools/build-language-server.mjs

## Purpose

Builds the current-platform Rust language server binary and copies it into the extension distribution folder.

## Ownership

This is repo-only developer/build tooling. It bridges Cargo output into the VS Code extension package layout so marketplace users receive a bundled binary and do not need Rust installed.

## Current Behavior

The script runs Cargo for the `reforger_language_server` binary, using debug mode by default or release mode with `--release`. It builds into `server/target/build-language-server` instead of the live `server/target/debug` or `server/target/release` folder so an already-running development language server does not lock Cargo output on Windows.

Before building and before each binary replacement attempt, the script force-stops repo-owned `reforger_language_server` processes. It then copies the resulting executable to both the development binary path under `server/target/<profile>/` and the packaged extension path under `dist/server/<platform>-<arch>/reforger_language_server(.exe)`, marking it executable on non-Windows platforms.

On Windows, the Extension Development Host can restart the language server quickly enough to relock the old development binary between stop and copy. The script treats `EBUSY`/permission-style copy failures as a retryable development race: it stops repo-owned server processes again, waits briefly, and retries replacement before failing.

## Dependencies and Boundaries

Uses Node built-in modules and local Cargo. It must not become runtime extension code, register a VS Code command, download game data, or manage cross-platform release matrices. `tools/` remains excluded from VSIX packages.

The force-stop behavior is development/build protection only. It filters Windows processes by executable path under this repo before killing them and is not a runtime extension feature.

## Verification

Run `npm run compile` and confirm both the development and packaged binary locations contain the expected current-platform executable.
