## Context

`/review` currently has four broad personas and a strict four-reviewer cap.
Language-server work frequently needs direct scrutiny of Enfusion truth, while
bug fixes need an independent check that diagnostics and tests prove the claim.

## Goals / Non-Goals

**Goals:** add two narrow optional personas, select them from evidence rather
than keywords, and preserve independent read-only review and the four-reviewer
cap.

**Non-Goals:** make either specialist a default core reviewer, add a security
persona, weaken Workbench evidence requirements, or run more than four agents
in one review.

## Decisions

- Add `language-fidelity.md` for parser, semantic model, LSP behavior,
  formatting, source/data truth, and Workbench evidence. It is selected for
  Enfusion language behavior and toolchain-fidelity risk.
- Add `verification-observability.md` for reproductions, test or fixture
  adequacy, logging/diagnostic evidence, and regression proof. It is selected
  for defects, evidence gaps, tests, logs, or scheduler/lifecycle claims.
- Retain Correctness and Architecture as the core. Auto mode adds at most two
  directly relevant specialists. Full mode is the deepest four-person review:
  core plus the two most relevant specialists, not all catalog entries.
- An explicit roster that would exceed four is not silently truncated; request
  a narrower roster or a second review. This preserves independent depth and
  clear coverage claims.

## Risks / Trade-offs

- [Specialists overlap broad reviewers] -> give each contract a distinct
  question set and require duplicate findings to be merged only when defect and
  durable direction match.
- [Full mode no longer means every catalog member] -> define it as the deepest
  bounded review and report selected/skipped rationale.
- [Evidence review becomes generic test advice] -> require a concrete missing
  proof, failed claim, or unreproducible path before reporting a finding.
