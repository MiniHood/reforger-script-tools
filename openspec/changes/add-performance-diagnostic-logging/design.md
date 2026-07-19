## Context

The extension currently writes startup timing records and passes a server log
path to Rust, but the records are split by purpose and do not consistently
identify request handling, elapsed time, or failure outcomes. Support cases
need a short, ordered account of extension-host activity and a separate,
correlatable account of language-server activity. The architecture requires
runtime state to stay under VS Code global storage and keeps TypeScript from
owning language analysis.

## Goals / Non-Goals

**Goals:**

- Produce separate extension-host and language-server diagnostic streams.
- Capture lifecycle, commands, LSP traffic metadata, scheduler/index events,
  failures, and completed-operation elapsed time.
- Keep logging enabled by default while allowing users to disable it.
- Bound retention and avoid document text, hover markdown, completion lists,
  or other source-derived payloads.
- Avoid synchronous file I/O in the extension request path and avoid logging
  per-token/per-symbol work in Rust.

**Non-Goals:**

- Persisting document contents, request parameters, or user source code.
- Replacing VS Code's output channel, existing targeted hover captures, or
  Rust's detailed internal developer log.
- Adding remote telemetry, upload, or automatic issue reporting.
- Establishing performance thresholds or changing LSP scheduling behavior.

## Decisions

### One JSON-lines file per runtime owner

The extension host SHALL write `extension-diagnostics.jsonl`; Rust SHALL write
`language-server-diagnostics.jsonl`. Both live in `globalStorageUri/logs/` and
each record includes timestamp, process/session identifier, component, event,
and structured scalar fields.

JSON lines retain temporal order, are easy to attach to support requests, and
permit tooling to filter or aggregate events. Separate ownership avoids
interleaving concurrent process writes and preserves the TypeScript/Rust
architecture boundary. A shared file was rejected because the processes would
need locking or a forwarding protocol solely for diagnostics.

### Lightweight event envelope and privacy boundary

Records SHALL include only operation metadata: method/command name, URI scheme
and normalized path category where useful, byte counts, document version or
revision, result classification, queue/scheduling state, and elapsed time.
They SHALL NOT include document text, completion/hover content, arbitrary RPC
parameters, file contents, or environment values.

This supports performance reconstruction without making support logs a source
export mechanism. Full request/response logging was rejected for privacy,
size, and hot-path overhead.

### Asynchronous, bounded extension writes; buffered Rust writes

The extension logger SHALL enqueue serialized records behind one promise chain
after ensuring its log directory. A failed write disables no language feature
and is swallowed after best-effort reporting to the existing output channel.
Rust SHALL retain one buffered file writer behind the existing logger boundary
and emit at operation boundaries, never inside token/symbol loops.

This avoids blocking VS Code providers on disk while keeping ordering within
each process. Console-only logging was rejected because it is not reliably
available in user support bundles.

### Default-on setting and bounded retention

`reforgerScriptTools.diagnostics.enabled` SHALL default to `true`. At startup,
each owner shall rotate/truncate its own diagnostic file before it exceeds a
small configured size, retaining the most recent bounded history. Disabling
the setting prevents new diagnostic records and is passed to Rust at startup.

Default-on supplies data during the current performance-investigation phase;
the setting provides a clear escape hatch. Unbounded append-only logs were
rejected because long-lived editor sessions would eventually make diagnostics
the performance/storage problem being investigated.

### Correlate by session and request identity without a new protocol

The extension generates one activation session identifier and records server
launch arguments only as safe presence/count fields. Rust generates its own
server session identifier and records JSON-RPC request IDs where already
available. Lifecycle records include the server process launch timing from the
extension and initialization completion from Rust.

A cross-process correlation protocol was rejected: matching activation/server
start time and per-process streams is sufficient initially and avoids new LSP
traffic.

## Risks / Trade-offs

- [High-frequency requests enlarge logs] → Record request start/end only with
  compact scalar fields; bound the file and omit payloads.
- [File-system failure prevents investigation] → Logging failures are isolated
  from editor behavior and surfaced through existing developer output where
  possible.
- [Default logging concerns users] → Document the local-only setting, exact
  storage location, and source-content exclusion.
- [Separate clocks make ordering approximate] → Use ISO timestamps and
  per-process elapsed milliseconds; do not imply a total cross-process order.

## Migration Plan

1. Add the default-on setting and shared TypeScript configuration constants.
2. Add extension-host lifecycle, command, watcher, server-start, and request
   outcome records.
3. Pass Rust the enabled state and language-server diagnostic path.
4. Add Rust transport, scheduler, indexing, and response-outcome records.
5. Add targeted tests for disabled logging, record shape/privacy, retention,
   and failure isolation; update reference documentation.
6. Users can disable new records immediately through the setting; no stored
   data migration is required.

## Open Questions

- None for the initial local diagnostic log. Log size and record detail can be
  tuned from observed support bundles without changing the ownership model.
