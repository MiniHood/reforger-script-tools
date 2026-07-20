## MODIFIED Requirements

### Requirement: Independent reviewer personas
The command SHALL select a bounded relevant roster from its persona catalog.
Correctness and Architecture SHALL be core reviewers for `depth:auto`; the
remaining personas SHALL be selected when the scope's risk surfaces warrant
their lens. The catalog SHALL include Performance & Reliability, Developer
Experience, Language Fidelity, and Verification & Observability as optional
specialists. The coordinator SHALL state why every persona was selected or
skipped and SHALL run no more than four reviewers in one review.

#### Scenario: Enfusion language behavior
- **WHEN** a scope changes or investigates parser, semantic, formatting,
  completion, hover, definition, Workbench, game-data, or Enfusion API behavior
- **THEN** the coordinator SHALL select Language Fidelity unless it records a
  concrete reason that the behavior is out of scope

#### Scenario: Evidence or regression risk
- **WHEN** a scope fixes a defect or concerns tests, fixtures, logs,
  diagnostics, reproducibility, lifecycle, or scheduler claims
- **THEN** the coordinator SHALL select Verification & Observability unless it
  records a concrete reason that proof of behavior is out of scope

#### Scenario: More than four requested personas
- **WHEN** an explicit roster would exceed four personas
- **THEN** the coordinator SHALL request a narrower roster or a follow-up
  review and SHALL NOT silently omit a selected persona

#### Scenario: Full review
- **WHEN** a user requests `depth:full`
- **THEN** the coordinator SHALL run Correctness and Architecture plus the two
  most relevant optional specialists, disclose the selection rationale, and
  preserve the four-reviewer cap
