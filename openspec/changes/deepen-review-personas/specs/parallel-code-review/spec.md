## MODIFIED Requirements

### Requirement: Scoped parallel review command
The repository SHALL provide a `/review` Codex skill that accepts a user-specified review scope and coordinates Architecture, Correctness, Performance & Reliability, and Developer Experience reviews. Before dispatch, the coordinator SHALL create and disclose a bounded review contract containing intent, relevant requirements, implementation scope, callers, tests, documentation, diagnostics, exclusions, and unknowns.

#### Scenario: Explicit source scope
- **WHEN** a user invokes `/review` with a file, subsystem, change, or defect scope
- **THEN** the coordinator SHALL review that scope and its directly relevant documentation, tests, and diagnostics

#### Scenario: Broad scope request
- **WHEN** a user invokes `/review` without a sufficiently bounded target
- **THEN** the coordinator SHALL state the inferred review scope and any material omissions before reporting findings

#### Scenario: Requirements grounding
- **WHEN** a relevant OpenSpec artifact, explicit requirement, or documented contract exists
- **THEN** the review contract SHALL identify it and reviewers SHALL assess the implementation against it

### Requirement: Independent reviewer personas
The command SHALL give each of the four reviewer personas the same bounded evidence package and a persona-specific review contract without exposing another persona's report, status, or conclusions. Each contract SHALL require multiple focused evidence slices, persona-specific exclusions, and a coverage verdict.

#### Scenario: Isolated review execution
- **WHEN** the coordinator launches the four reviewers
- **THEN** each reviewer SHALL perform an independent read-only review and SHALL NOT communicate with or consume output from another reviewer

#### Scenario: Evidence journal
- **WHEN** a reviewer completes an evidence slice
- **THEN** it SHALL update only its own generated Markdown journal with the question, evidence inspected, conclusion, linked finding if any, and next slice

#### Scenario: No meaningful findings
- **WHEN** a reviewer finds no evidence-backed issue in its assigned lens
- **THEN** that reviewer SHALL explicitly report no meaningful finding rather than inventing a recommendation

### Requirement: Evidence-based reviewer reports
Each reviewer report SHALL distinguish facts, inferences, and unknowns, and every finding SHALL include priority, confidence, evidence location, impact, durable direction, and required validation. P1 SHALL identify immediate stop/mitigation work; P2 SHALL identify critical work required before release; P3 SHALL identify material planned work; P4 SHALL identify low-impact improvements.

#### Scenario: Evidence-backed issue
- **WHEN** a reviewer identifies a potential issue
- **THEN** the report SHALL cite the relevant file and symbol, test, log, or other concrete evidence and SHALL label unverified reasoning as inference

#### Scenario: Unsupported concern
- **WHEN** a concern lacks sufficient evidence for its claimed impact
- **THEN** the reviewer SHALL record it as an unknown or suppress it rather than presenting it as a finding

### Requirement: Coordinator synthesis
The coordinator SHALL synthesize the four reports into one advisory result that preserves material disagreement, deduplicates overlapping findings, assigns stable identifiers, groups related findings, and recommends a next step based on priority, confidence, and user impact. Every P1-P3 finding SHALL receive a residual-work disposition.

#### Scenario: Overlapping findings
- **WHEN** two or more reviewers identify the same underlying issue and fix path
- **THEN** the coordinator SHALL present one combined finding with all relevant evidence and identify the contributing review lenses

#### Scenario: Material disagreement
- **WHEN** reviewers reach materially different conclusions about a risk or direction
- **THEN** the coordinator SHALL present the competing conclusions and their evidence rather than claiming consensus

#### Scenario: Material residual work
- **WHEN** a P1, P2, or P3 finding remains unresolved
- **THEN** the coordinator SHALL mark it as fix now, planned task, accepted residual with owner and reason, or needs additional evidence
