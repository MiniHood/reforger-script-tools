## Why

The initial `/review` command establishes independent personas but its contracts are too shallow to guarantee a deep, repeatable review. It needs explicit evidence slices, scope and requirement grounding, calibrated P1-P4 priorities, and a durable disposition for important findings.

## What Changes

- Strengthen the four fixed reviewer contracts with persona-specific investigation slices, evidence thresholds, exclusions, and coverage outputs.
- Add a frozen review contract that links the requested scope to relevant requirements, changed symbols, callers, tests, docs, diagnostics, and exclusions.
- Require each reviewer to maintain an isolated generated Markdown evidence journal while reviewing.
- Add finding validation, P1-P4 priority definitions, independent confidence, stable identifiers, synthesis grouping, and residual-work dispositions.
- Preserve report-only behavior and best-effort parallel scheduling; do not add default reviewers beyond the four core personas.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `parallel-code-review`: Deepen the existing review command's evidence collection, persona contracts, priority system, and synthesis workflow.

## Impact

- Updates `.codex/skills/review/` and its generated review-artifact instructions.
- Updates the existing `parallel-code-review` OpenSpec capability through a delta spec.
- Does not modify extension or language-server runtime behavior.
