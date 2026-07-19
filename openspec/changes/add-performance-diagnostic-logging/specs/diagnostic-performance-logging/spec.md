## ADDED Requirements

### Requirement: Separate local diagnostic streams
The extension SHALL write an extension-host diagnostic stream and the language
server SHALL write a separate language-server diagnostic stream under VS Code
global storage when diagnostics are enabled.

#### Scenario: Normal startup
- **WHEN** the extension activates with diagnostics enabled
- **THEN** it SHALL create or append only to the extension-host diagnostic file
  and pass the separate language-server diagnostic path to the server process

### Requirement: Structured operational records
Each diagnostic record SHALL be one JSON object line containing a timestamp,
component, event name, session identifier, and relevant safe scalar fields.
The streams SHALL record lifecycle activity, commands or LSP methods, operation
outcomes, failures, and elapsed time for completed performance-relevant work.

#### Scenario: Completed LSP request
- **WHEN** the language server completes a request
- **THEN** its diagnostic stream SHALL record the request method, outcome, and
  elapsed time without recording the request or response payload

### Requirement: Source-content privacy
Diagnostic streams SHALL NOT record document text, arbitrary JSON-RPC
parameters, feature result bodies, game-data contents, environment values, or
other user source content.

#### Scenario: Workspace file notification
- **WHEN** a workspace file change is observed
- **THEN** the diagnostic record SHALL contain only safe metadata such as event
  kind, byte count, sequence, and normalized path classification

### Requirement: Optional default-on diagnostics
The extension SHALL expose a diagnostics-enabled setting that defaults to true.
When disabled, the extension SHALL not create new diagnostic records and SHALL
start the language server with diagnostics disabled.

#### Scenario: Diagnostics disabled
- **WHEN** a user disables the diagnostics setting and reloads the extension
- **THEN** neither runtime SHALL append diagnostic records for that session

### Requirement: Bounded low-impact retention
Each diagnostic stream SHALL retain only bounded recent history and SHALL emit
records at operation boundaries rather than in per-token, per-symbol, or other
unbounded inner loops. A diagnostic write failure SHALL NOT fail a language
feature or extension operation.

#### Scenario: Diagnostic storage reaches its limit
- **WHEN** a diagnostic file would exceed its configured retention limit
- **THEN** its owner SHALL retain bounded recent records and continue normal
  extension or language-server operation
