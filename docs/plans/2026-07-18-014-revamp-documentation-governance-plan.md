# Revamp documentation governance plan

## Goal

Make repository documentation easy for Codex and human maintainers to navigate:
one architecture source of truth, clear ownership for every documentation tier,
and reference pages that describe current behavior rather than accumulated work
history.

## Decisions

- `AGENTS.md` remains short, enforceable policy; it is not a second workflow or
  architecture manual.
- `docs/reference/architecture.md` is the sole top-level architecture and
  runtime-boundary document.
- `docs/reference/` owns current source/subsystem context and mirrors source
  ownership where that context is valuable.
- `docs/plans/` is an immutable decision history, not current documentation.
- `docs/solutions/` is a searchable learning store, not an architecture or
  process-manual substitute.
- Add `docs/documentation.md` as the authoritative documentation procedure and
  routing map. `docs/agent-workflow.md` keeps rationale only.

## Implementation units

### U1 - Build the inventory and keep/delete/rewrite map

- Classify every tracked Markdown page by tier, owner, source of truth,
  audience, freshness, and replacement target when applicable.
- Identify exact duplication, obsolete change-history sections, broken links,
  orphaned pages, and pages that only document generated output.
- Do not delete or move a page without recorded replacement rationale.

### U2 - Establish the documentation procedure

- Create `docs/documentation.md` with the tier taxonomy, audience, routing
  rules, creation/update triggers, retirement rules, required page shape, and
  docs-only verification checklist.
- Specify that reference pages are written for both Codex and maintainers:
  concise ownership/boundary/current-behavior context, not tutorials or logs.

### U3 - Align policy and workflow entry points

- Reduce `AGENTS.md` documentation policy to policy plus links to the
  architecture overview, procedure, workflow rationale, and solutions store.
- Remove duplicated procedural instructions from `docs/agent-workflow.md` and
  retain rationale, evidence discipline, and migration safeguards.

### U4 - Consolidate architecture and source-reference pages

- Keep cross-layer ownership and data flow only in `architecture.md`.
- For each reference page, retain Purpose, Architecture Role, Current Behavior,
  Dependencies/Boundaries, and narrowly useful Future Direction.
- Remove stale change-note timelines; preserve durable historical decisions in
  plans or solutions through links where they remain useful.
- Treat report-generator pages as source-owner context and generated reports as
  ignored output, following the existing report-owner convention.

### U5 - Normalize plans and solutions discovery

- Add concise indexes/readme-style entry points only where inventory evidence
  shows navigation is otherwise poor; do not create a second architecture map.
- Ensure plans are labelled historical/decision artifacts and solution pages
  link to related current owners without restating their whole behavior.

### U6 - Verify migration completeness

- Run a full link/path check, `git diff --check`, and manual before/after
  review against the inventory map.
- Confirm every deletion has an intentional replacement or a documented reason
  it no longer carries durable context.
- Review the final documentation tree from three entry points: new Codex
  orientation, subsystem change, and post-incident learning lookup.

## Risks

- Aggressive pruning can erase useful source history; use the per-file map and
  preserve durable decisions in plans/solutions.
- Mechanical mirror pages can be useful for complex owners; judge them by
  whether a future change needs the context, not by filename or size alone.

## Verification

- Docs-only checks: `git diff --check`, manual Markdown-link/path validation,
  and inventory-to-tree reconciliation.
- No build is required unless the migration changes generated documentation,
  source, packaging, or extension behavior.
