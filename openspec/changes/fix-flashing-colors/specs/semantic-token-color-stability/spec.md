## ADDED Requirements

### Requirement: Stable rich semantic colors during edits
For an open document that has previously established a rich semantic-token
display, the language server SHALL NOT replace that display with a lexical-only
token response while a newer revision's matching rich projection is pending.

#### Scenario: External type is edited near
- **WHEN** a user edits an open document containing an externally resolved type
  such as `SCR_GameModeEndData`
- **THEN** the editor SHALL retain its existing semantic color until matching
  rich tokens for the new revision are available

### Requirement: Current-revision rich token publication
The language server SHALL answer deferred semantic-token requests only with
rich token data whose document revision and external-index generation match the
current document state.

#### Scenario: Rich analysis completes
- **WHEN** foreground and semantic analysis complete for the requested
  document revision
- **THEN** the server SHALL publish the matching rich token projection and
  request a semantic-token refresh

### Requirement: Revision-safe token request supersession
The language server SHALL cancel or discard pending semantic-token requests
when their document revision is replaced or closed, and SHALL NOT publish prior
revision token ranges for current source text.

#### Scenario: User continues typing
- **WHEN** a newer edit arrives before rich token analysis for a prior revision
  completes
- **THEN** the server SHALL suppress the prior result and wait for the newest
  revision's rich projection

### Requirement: Initial and unavailable token fallback
The language server SHALL provide a current-text lexical token response only
when no prior rich display exists for the document or rich token production is
not yet available for an initial document. It SHALL preserve an existing rich
display rather than downgrade it because current rich work is overloaded,
cancelled, or unavailable.

#### Scenario: First document token request
- **WHEN** VS Code requests semantic tokens for a newly opened document before
  its first rich projection is available
- **THEN** the server SHALL return a current-text lexical baseline and schedule
  matching rich refinement without a fixed idle delay
