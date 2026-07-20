## Context

The review coordinator is specified in Markdown. It must make the same roster
decision from the same request and must eventually produce a full or partial
result without weakening isolation.

## Goals / Non-Goals

**Goals:** deterministic parsing and selection, a bounded unavailable state,
and regression scenarios for all review invariants.

**Non-Goals:** add runtime orchestration code, change persona content, or run
more than four reviewers.

## Decisions

- Parse at most one depth token and at most one persona token. Reject duplicate,
  malformed, and unknown tokens with a clarification; do not guess.
- `personas:` overrides depth selection. `personas-only:` is the explicit form
  that omits the core reviewers. A normal `personas:` request retains the core.
- With more than two relevant specialists, rank direct scope ownership first,
  then explicit user concern, then demonstrated failure/release risk. Record
  every displaced relevant specialist and why.
- Treat a reviewer as unavailable after two coordinator waits with no final
  report or progress update. Preserve its journal and synthesize a partial
  result.
- Extend acceptance scenarios rather than introduce an implementation harness.

## Risks / Trade-offs

- [Strict parsing needs clarification] -> reject ambiguity before fan-out.
- [Fixed liveness threshold ends a slow review] -> require two missed progress
  intervals and preserve partial evidence; the user can request a new review.
