## Context

The deep four-person workflow is valuable for broad changes but expensive for narrow scopes. The coordinator already knows the reviewed contract, so it can select an appropriate bounded roster before fan-out and render one actionable report after reviewer completion.

## Goals / Non-Goals

**Goals:** select Correctness and Architecture by default, add relevant specialists up to four, permit `depth:full` or explicit persona overrides, require repository policy and owning documentation in each evidence package, and summarize deduplicated results in priority order.

**Non-Goals:** automatic fixes, keyword-only selection, silently omitting unavailable reviewers, or adding security/API specialists before their contracts exist.

## Decisions

- Use judgment from the review contract, changed paths, requirements, and risk surfaces; record the reason for each selected or skipped persona.
- The current four personas form the initial catalog. Correctness and Architecture are core; Performance & Reliability and Developer Experience are conditional.
- `depth:auto` selects the core plus relevant specialists; `depth:full` selects all four; `personas:` explicitly selects from the catalog but cannot omit core reviewers unless the user explicitly requests a narrow persona-only review.
- The final report table contains only deduplicated findings. Individual journals remain the evidence trail.
- A reviewer that fails, is interrupted, or returns a nonconforming report is marked unavailable. The coordinator preserves any partial journal, reports partial coverage, and never represents the synthesized result as a complete review.

## Risks / Trade-offs

- [Incorrect selection] → disclose selection rationale and support full/explicit overrides.
- [Sparse table] → preserve strengths, coverage, unknowns, and residual-work sections below it.
- [Unavailable reviewer] → produce an incomplete final report with explicit coverage, never pretend all personas completed.
