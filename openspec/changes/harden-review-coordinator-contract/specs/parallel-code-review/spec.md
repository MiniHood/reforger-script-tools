## MODIFIED Requirements

### Requirement: Independent reviewer personas
The command SHALL resolve a request into one deterministic roster of no more
than four personas. A normal `personas:` token SHALL retain Correctness and
Architecture; `personas-only:` SHALL be the only form that omits core
reviewers. Duplicate, malformed, or unknown roster tokens SHALL require
clarification before fan-out. When more than two specialists are relevant, the
coordinator SHALL rank direct scope ownership, explicit user concern, then
demonstrated failure or release risk, and record displaced specialists.

#### Scenario: Explicit roster precedence
- **WHEN** a valid `personas:` token is supplied with a depth token
- **THEN** the explicit roster SHALL determine the selected specialists and
  depth SHALL determine only review thoroughness

#### Scenario: Ambiguous tokens
- **WHEN** a request has duplicate, malformed, or unknown roster tokens
- **THEN** the coordinator SHALL request clarification and SHALL NOT fan out

#### Scenario: More than two relevant specialists
- **WHEN** auto or full selection identifies more than two specialists
- **THEN** the coordinator SHALL select the two highest-ranked specialists and
  record each displaced relevant specialist with its ranking reason

### Requirement: Review completion and evidence
The coordinator SHALL wait for selected reviewers to reach a terminal state.
A reviewer with no final report or progress update after two coordinator wait
intervals SHALL be marked unavailable; its journal SHALL be retained and the
final synthesis SHALL be labelled partial. Acceptance validation SHALL cover
token parsing, specialist displacement, unavailable reviewers, package-only
inputs, peer-journal isolation, withheld interim findings, and journal-only
mutation.

#### Scenario: Unresponsive reviewer
- **WHEN** a selected reviewer misses two coordinator wait intervals without a
  final report or journal progress update
- **THEN** the coordinator SHALL mark it unavailable and publish a partial
  synthesis from the completed reports
