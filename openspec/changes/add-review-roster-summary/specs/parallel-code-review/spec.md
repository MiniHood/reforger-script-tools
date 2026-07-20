## MODIFIED Requirements

### Requirement: Independent reviewer personas
The command SHALL select a bounded relevant roster from its persona catalog. Correctness and Architecture SHALL be core reviewers for `depth:auto`; Performance & Reliability and Developer Experience SHALL be selected when the scope's risk surfaces warrant them. The coordinator SHALL state why every persona was selected or skipped.

#### Scenario: Narrow scope
- **WHEN** a scope has no performance or user-visible risk surface
- **THEN** the coordinator SHALL run the core reviewers without dispatching irrelevant specialists

#### Scenario: Full review
- **WHEN** a user requests `depth:full`
- **THEN** the coordinator SHALL select all four available personas

#### Scenario: Repository policy evidence
- **WHEN** the coordinator prepares a reviewer package
- **THEN** it SHALL include `AGENTS.md` and the relevant owning documentation with the scope evidence

### Requirement: Coordinator synthesis
The coordinator SHALL render completed review findings as one deduplicated table ordered by P1 through P4. Each row SHALL include stable ID, finding, evidence, impact, next step, confidence, and contributing reviewers.

#### Scenario: Incomplete roster
- **WHEN** a selected reviewer is unavailable or fails
- **THEN** the coordinator SHALL preserve any partial journal, render available findings, disclose incomplete coverage and the unavailable persona, and label the result a partial review
