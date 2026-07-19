## 1. Diagnostic contract and configuration

- [x] 1.1 Add the default-on extension diagnostics setting and centralized log
  file/retention configuration.
- [x] 1.2 Define the safe structured-record envelope and path-redaction rules
  shared by extension-host diagnostic call sites.
- [x] 1.3 Document log ownership, local storage, retention, privacy limits,
  and support-bundle use in the relevant reference documentation.

## 2. Extension-host diagnostics

- [x] 2.1 Implement a failure-isolated, serialized JSONL diagnostic writer
  with bounded retention under global storage.
- [x] 2.2 Record activation, game-data, client launch/restart, commands,
  watcher batches, and language-client request outcomes with elapsed times.
- [x] 2.3 Pass the language-server diagnostic setting and separate log path to
  the Rust process without exposing user content in arguments or records.

## 3. Language-server diagnostics

- [x] 3.1 Add optional Rust diagnostic-log configuration and a buffered,
  bounded JSONL writer that leaves existing developer logging behavior intact.
- [x] 3.2 Record server lifecycle, JSON-RPC method outcomes, scheduler/admission
  decisions, external-index lifecycle, and errors at operation boundaries.
- [x] 3.3 Ensure response/error logging includes elapsed time and safe result
  classification without serializing request or response payloads.

## 4. Verification

- [x] 4.1 Add TypeScript tests for enabled/disabled behavior, record safety,
  retention, and failure isolation.
- [x] 4.2 Add Rust tests for enabled/disabled behavior, record safety,
  retention, and request outcome timing.
- [x] 4.3 Run targeted TypeScript checks, Rust tests, formatting/lint checks,
  and a manual extension-host validation that confirms separate log files.
