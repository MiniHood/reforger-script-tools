# tools/lsp-runtime-performance-report.mjs

## Purpose

Builds a dev-only Markdown report from the Rust language-server runtime log so high CPU or typing-latency sessions can be reviewed after the fact.

## Ownership

This tool is outside the packaged runtime path. It reads `globalStorageUri/logs/language-server.log`, groups multi-line Rust log records, parses key timing fields, and writes `tools/reports/lsp-runtime-performance.report.md`.

## Current Behavior

The report summarizes logged elapsed work by operation, file/URI, one-second time window, completion latency, edit-analysis latency, and semantic-token worker health. It specifically separates:

- foreground request/notification time
- queue wait time for live stdio requests when available
- background rich semantic-token time
- stale/skipped rich semantic-token work
- cancelled rich semantic-token work
- completion candidate lookup time
- `didChange` parse/catalog/index/scope timing
- lazy document-symbol projection spikes
- first usable token and completion responses after an accepted snapshot
- current-snapshot versus unpaired foreground-response correlation
- explicit runtime admission/overload dispositions when emitted
- explicit rich/debug cancellation tails when emitted
- declared foreground `QueryQuality` by feature, including missing declarations
- rich document/external-generation identity and cancellation-marker coverage
- source-free capture-field completeness, so legacy/defaulted fields cannot be
  mistaken for comparable U7 evidence

It accepts:

```powershell
node tools/lsp-runtime-performance-report.mjs
node tools/lsp-runtime-performance-report.mjs --since-minutes 10
node tools/lsp-runtime-performance-report.mjs --log <path> --out <path>
```

## Controlled Typing Capture

For comparable large-file latency evidence, start a fresh language server, wait until external indexing is idle, and avoid unrelated editor activity. For each control file, perform three bursts of ten gibberish-to-real-prefix completion cycles, then use `Ctrl+F2` to retain the completion capture. Generate one report window for `GC_MarkerArea.c` and one for `GC_Sounds.c` with `--since-minutes`.

The report preserves aggregate sections and also provides two capture-oriented summaries:

- **Burst Comparison** attributes `didChange`, completion, queue, coalescing, and perceived-latency observations by URI and accepted document revision. Missing fixed `didChange` fields (`queue_ms`, `coalesced_changes`, `superseded_changes`, selected version, and selected revision) safely default to zero or the legacy version/revision field.
- **Capture Evidence Quality** aggregates those revision rows by URI in the explicit report window. A file requires at least ten completion requests to be classified **Sufficient**; an **Insufficient** result is useful context but not before/after proof.

When the server uses deferred open-document analysis, **Edit Analysis Latency**
separates cheap foreground `didChange` acceptance from `documentAnalysis`
ready/superseded worker records. Do not interpret a low `didChange` total as
zero analysis work; compare it with the background-analysis totals.

The report reads the runtime log without modifying it. It never emits source text, completion prefixes, or completion payload fields, even if legacy log records contain them.

The report does not sample OS CPU directly. It uses logged elapsed timings as the first debugging pass for identifying which LSP subsystem likely caused CPU use or visible delay. When logs include `queue_ms`, completion sections separate execution time from perceived latency caused by waiting behind earlier LSP work.

## Foreground, Snapshot, and Cancellation Evidence

**First Usable Foreground Response** correlates an accepted `didOpen` or
`didChange` snapshot with the first later `semanticTokens` or `completion`
record for the same URI and revision. It measures observed log time only; the
report never inspects source text, typed prefixes, or completion payloads.
`lexical-pending` counts show how often the first token response was available
before semantic analysis completed.

**Snapshot Quality** marks a foreground response current only when its URI and
revision match accepted ingress in the selected window. A missing match is
unpaired, not assumed stale. Semantic ready, skipped, and discarded records are
reported separately.

**Admission and Overload** consumes explicit `disposition`, `admission`, or
`outcome` fields (or recognized runtime admission records). If a legacy log
does not contain them, the report says that admission evidence is unavailable;
worker completion is not mistaken for admission.

**Cancellation Tails** use only `cancellation_tail_ms` or `tail_ms` on a
terminal cancellation record. Total worker elapsed time is deliberately not
used as a tail proxy because it includes useful work before cancellation.

## Query, Admission, and Rich Capture Contracts

**Foreground Query Quality** counts the server-declared `Exact`,
`RecoveryExact`, and `Unavailable` quality for feature responses that emit a
`query_quality` field. An absent or unrecognized value remains missing; the
report does not infer a guarantee from cache state, elapsed time, or a matching
revision.

**Admission and Overload** additionally groups explicit dispositions by lane
and reports absent admission identity/disposition markers. It does not promote
worker completion or a rich-overload skip into an admission record.

**Rich Cancellation and Identity** requires `uri` plus `revision` for document
identity and `external_generation` for overlay identity. Cancelled terminals
also require a reason and a measured tail for full cancellation evidence.

**Capture Field Completeness** audits only source-free marker names and counts:
accepted snapshot identity, query-quality response identity/quality, admission
identity/lane/disposition, and rich terminal/cancellation identity. It never
prints source text, cursor prefixes, labels, completion items, or token data.
Older summary sections may preserve zero-value compatibility defaults, but this
audit intentionally flags the corresponding omitted fields.

## Dependencies and Boundaries

Uses only Node built-ins. It must not register a package script or VS Code command unless runtime performance reporting becomes a user-facing feature. It must not add logging overhead to the LSP server; it consumes the existing log.

## Verification

Run the script against a captured language-server log and inspect that malformed records are reported without changing the log or extension runtime.
