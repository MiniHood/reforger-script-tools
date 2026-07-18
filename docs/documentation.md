# Documentation Procedure

## Documentation map

- `AGENTS.md`: enforceable repository policy and entry links.
- `docs/reference/architecture.md`: cross-layer runtime architecture and ownership.
- `docs/reference/`: current subsystem and source-owner context.
- `docs/plans/`: historical implementation and decision artifacts.
- `docs/solutions/`: reusable resolved-problem learnings.
- `docs/agent-workflow.md`: rationale for the repository workflow.
- `tools/reports/`: ignored generated investigation output.

## Creating and updating pages

Every new non-trivial source file or script must have matching reference context
in the same change. Prefer a page mirroring its source path, for example
`server/src/lsp/hover.rs` -> `docs/reference/server/src/lsp/hover.md`. A small
file with no independent ownership may be covered by its nearest subsystem page;
state that ownership there rather than creating noise. Do not create pages for
generated output, dependencies, trivial metadata, or mechanical formatting.

Every source-owner page must contain these sections: Purpose, Ownership,
Current Behavior, Dependencies and Boundaries, and Verification. `Architecture
Role` may replace `Ownership` only for a contribution/configuration file whose
relationship to a higher-level owner is clearer than a separate ownership
statement. `Future Direction` is optional and appears only for a genuine
remaining limitation or planned boundary change. Update a page when those
facts change.

Keep architecture pages conceptual and cross-layer; their headings may be
tailored to explain the system rather than repeat the source-owner contract.
Keep source-reference pages current and concise. Do not retain change-note
timelines that Git history, plans, or solutions already preserve. Start from
the [reference index](reference/README.md) when navigating by subsystem rather
than source path.

Use plans for intended work and decisions, never as current architecture. Use
solutions for durable lessons, linking to current owners instead of duplicating
their behavior. Search solutions when working in a documented problem area.

## Retirement and verification

Rewrite, move, or delete stale documentation only with evidence from current
source, accepted policy, explicit user direction, or a current CE artifact.
Preserve useful replacement context in its correct owner. For docs-only work,
run `git diff --check` and manually validate links, paths, and the affected
entry-point journey.
