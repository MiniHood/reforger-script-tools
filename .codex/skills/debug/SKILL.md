---
name: debug
description: Diagnose Reforger Script Tools extension, language-server, completion, hover, indexing, game-data, startup, or performance failures. Use when asked to debug an extension symptom, inspect diagnostic logs, reproduce a language-feature issue, or establish a root cause before implementing a fix.
---

# Debug Reforger Script Tools

Run an evidence-first diagnosis. Do not edit code until the causal chain is
explained and the user authorizes a fix, unless their request explicitly asks
to fix the issue.

## Standard loop

1. State the observed symptom, expected behavior, reproduction conditions, and
   whether it is a regression.
2. Inspect `git status`, the relevant owner documentation, and recent history
   for the affected subsystem.
3. Reproduce with the smallest relevant test, command, or editor action.
4. Capture evidence at the ownership boundary:
   - Extension/process/configuration: extension diagnostics and the language
     client startup log.
   - LSP lifecycle, indexing, request timing, or queueing: language-server
     diagnostics and `language-server.log`.
   - Hover: `logs/hover-debug/latest.md` or the Debug Hover command.
   - Completion or signature help: `logs/completion-debug/latest.md` or the
     Debug Completion command.
   - Game-data/API behavior: inspect extracted game data; invoke `reforger`
     before making Enfusion language claims.
5. Separate evidence by layer. The TypeScript host owns activation, settings,
   editor events, and process transport; Rust owns language facts and LSP
   results. Do not diagnose a Rust behavior by adding TypeScript language
   logic.
6. Review the relevant design and architecture before selecting a root cause:
   identify duplicated language paths, broken ownership boundaries, stale-data
   joins, lifecycle coupling, unbounded work, and abstractions that make the
   symptom likely to recur. Compare the failing path with its authoritative
   counterpart and distinguish a local defect from a systemic design concern.
7. When the diagnosis may be systemic, invoke `/review` as a read-only
   independent check before settling on a fix. It is useful when the symptom
   crosses TypeScript/Rust/runtime boundaries, has survived more than one
   attempted fix, recurs across files or features, exposes a performance or
   lifecycle trade-off, or the logs prove the failing layer but not why that
   layer failed. Give reviewers the smallest explicit scope and relevant log
   window; select `verification-observability` for missing evidence and add
   the owning specialist such as `architecture`, `performance-reliability`,
   or `language-fidelity`. Do not invoke reviewers for a clearly local,
   directly reproducible defect with an already-proven causal chain. Treat the
   review as advisory evidence, not as a replacement for the debug result.
8. Form a ranked root-cause hypothesis. For uncertain links, state a prediction
   and test it with an independent observation.
9. Report root cause, affected ownership boundary, architecture/design concerns
   (or explicitly state none found), the smallest regression test, and a focused
   fix plan. Ask whether to implement if the user asked for
   diagnosis only.

## Log locations

Build the global-storage log root from the active VS Code profile. In the
default Windows profile it is:

`$env:APPDATA\Code\User\globalStorage\undefined_publisher.reforger-sript-tools\logs`

Read only the bounded tail needed for the event window. Treat general JSONL
diagnostics as operational metadata; do not expect them to contain source text
or completion candidates. Use cursor reports for feature-specific details.

## Performance diagnosis

Measure before optimizing. Record startup/index duration, request elapsed time,
queue delay, cache status, document byte size, and whether the response used
matching-revision analysis or a pending fallback. Distinguish:

- startup or external-index build time;
- request-thread latency;
- background analysis convergence;
- stale/pending fallback behavior;
- repeated watcher or restart churn.

Do not add synchronous logging, source serialization, or broad tracing on the
typing path. Keep diagnostic logging optional, bounded, asynchronous, and
outside the workspace.

## Fix and verification

After authorization, add or strengthen the narrowest regression test before
changing behavior. Preserve current document revision safety: never combine
current document text with prior local semantic facts. For Rust server changes,
run the focused test and `cargo test`; for extension changes, run `npm test`
and lint. Rebuild and reload the existing active Extension Development Host in
place when the active server binary or client lifecycle changes. Never open a
replacement `--extensionDevelopmentPath` window or close VS Code as a reload
substitute: preserve the user's debug-session UI and state. If no available
automation can target the existing host, report live verification as pending.

## Handoff

Finish every debug task with:

1. What was worked on.
2. What was completed and verified.
3. Remaining uncertainty, limitations, or next steps.
