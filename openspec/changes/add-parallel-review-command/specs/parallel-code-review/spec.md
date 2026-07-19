## ADDED Requirements

### Requirement: Scoped parallel review command
The repository SHALL provide a `/review` Codex skill that accepts a user-specified review scope and coordinates Architecture, Correctness, Performance & Reliability, and Developer Experience reviews.

#### Scenario: Explicit source scope
- **WHEN** a user invokes `/review` with a file, subsystem, change, or defect scope
- **THEN** the coordinator SHALL review that scope and its directly relevant documentation, tests, and diagnostics

#### Scenario: Broad scope request
- **WHEN** a user invokes `/review` without a sufficiently bounded target
- **THEN** the coordinator SHALL state the inferred review scope and any material omissions before reporting findings

### Requirement: Independent reviewer personas
The command SHALL give each of the four reviewer personas the same bounded evidence package and a persona-specific review contract without exposing another persona's report, status, or conclusions.

#### Scenario: Isolated review execution
- **WHEN** the coordinator launches the four reviewers
- **THEN** each reviewer SHALL perform an independent read-only review and SHALL NOT communicate with or consume output from another reviewer

#### Scenario: No meaningful findings
- **WHEN** a reviewer finds no evidence-backed issue in its assigned lens
- **THEN** that reviewer SHALL explicitly report no meaningful finding rather than inventing a recommendation

### Requirement: Best-effort parallel execution
The command SHALL launch the four independent reviewers concurrently when runtime capacity permits and SHALL preserve reviewer independence if capacity requires queued execution.

#### Scenario: Sufficient agent capacity
- **WHEN** four reviewer slots are available
- **THEN** the coordinator SHALL start all four reviewer tasks without waiting for another reviewer to complete

#### Scenario: Limited agent capacity
- **WHEN** fewer than four reviewer slots are available
- **THEN** the coordinator SHALL schedule outstanding reviewers as capacity becomes available and SHALL disclose the capacity limitation in the final report

### Requirement: Evidence-based reviewer reports
Each reviewer report SHALL distinguish facts, inferences, and unknowns, and every finding SHALL include severity, confidence, evidence location, impact, durable direction, and required validation.

#### Scenario: Evidence-backed issue
- **WHEN** a reviewer identifies a potential issue
- **THEN** the report SHALL cite the relevant file and symbol, test, log, or other concrete evidence and SHALL label unverified reasoning as inference

#### Scenario: Severity classification
- **WHEN** a reviewer reports a finding
- **THEN** it SHALL classify it as Critical, High, Medium, or Low according to its likely user and system impact

### Requirement: Coordinator synthesis
The coordinator SHALL synthesize the four reports into one advisory result that preserves material disagreement, deduplicates overlapping findings, and recommends a next step based on severity, confidence, and user impact.

#### Scenario: Overlapping findings
- **WHEN** two or more reviewers identify the same underlying issue
- **THEN** the coordinator SHALL present one combined finding with all relevant evidence and identify the contributing review lenses

#### Scenario: Material disagreement
- **WHEN** reviewers reach materially different conclusions about a risk or direction
- **THEN** the coordinator SHALL present the competing conclusions and their evidence rather than claiming consensus

### Requirement: Read-only review operation
The `/review` command and its reviewer personas SHALL not modify source code, OpenSpec artifacts, configuration, external state, or repository history.

#### Scenario: Review completion
- **WHEN** the coordinator returns the final review result
- **THEN** it SHALL state that the review was advisory and that no implementation changes were made
