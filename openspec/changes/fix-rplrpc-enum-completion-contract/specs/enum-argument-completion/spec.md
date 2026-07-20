## ADDED Requirements

### Requirement: Valid selected-enum completion edits
The language server SHALL emit only completion edits that satisfy VS Code's
range contract. For every `InsertReplaceEdit`, the insert range MUST start at
the replace range start and MUST be a prefix of the replace range.

#### Scenario: RplRpc selected enum default
- **WHEN** the RplRpc snippet opens completion for its selected
  `RplChannel.Reliable` argument
- **THEN** every returned completion item uses an editor-valid replacement
  range for that complete selected expression

### Requirement: Enum-specific RplRpc suggestions
The RplRpc first-argument follow-up completion SHALL present qualified
`RplChannel` members in deterministic enum-first order and SHALL retain the
normal contextual value candidates beneath them: visible locals and
parameters, containing-class members, top-level symbols, and keywords. Every
candidate SHALL replace the complete selected enum expression when accepted.

#### Scenario: RplRpc channel selection
- **WHEN** the user accepts RplRpc and the first enum field is selected
- **THEN** `RplChannel.Reliable` and `RplChannel.Unreliable` are available
  before normal contextual value and keyword alternatives

### Requirement: Responsive snippet-to-Suggest bridge
The extension SHALL dispatch at most one Suggest request after the expected
RplRpc snippet selection becomes active and SHALL not add a typing-path delay
or retry loop.

#### Scenario: Selected field dispatch
- **WHEN** VS Code selects the expected RplRpc first placeholder
- **THEN** the extension dispatches one Suggest request and releases the
  temporary transaction after its response, cancellation, or bounded expiry

### Requirement: Completion boundary verification
The project SHALL verify the RplRpc snippet-to-editor completion boundary with
wire-level range tests and a documented fresh Extension Development Host
verification of visible choices, acceptance, typed replacement, and progression
to the second placeholder.

#### Scenario: Regression verification
- **WHEN** the RplRpc completion behavior changes
- **THEN** verification proves valid edit ranges and the visible enum choice
  journey without relying only on server candidate counts
