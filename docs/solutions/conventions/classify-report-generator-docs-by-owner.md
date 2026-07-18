---
title: Classify Report Generator Documentation by Owner
date: 2026-07-18
category: conventions
module: documentation
problem_type: convention
component: documentation
severity: medium
applies_when:
  - "Classifying, moving, or deleting repository documentation"
  - "Reviewing report-named source-owner pages or generated output"
  - "Migrating a documentation root to a new path"
tags: [documentation, report-generators, source-ownership, migrations]
---

# Classify Report Generator Documentation by Owner

## Context

Report-like filenames do not prove that a Markdown page is generated output.
In this repository, pages such as `docs/reference/server/examples/parser_report.md` document the behavior, boundaries, and default output of a real source generator: `server/examples/parser_report.rs`.
They are active source-owner context, not a generated report to relocate or discard.

The generator's run output is a different artifact. `parser_report.rs` writes its default result under `tools/reports/`, which is ignored by [`.gitignore`](../../../.gitignore).
The ownership boundary is defined in [AGENTS.md](../../../AGENTS.md) and explained in [the agent workflow](../../agent-workflow.md).

## Guidance

Classify a documentation page by what it documents, not by its filename.

1. Identify the page's claimed source owner from its title and content.
2. Confirm the source or subsystem still exists, then check that the page's purpose and boundaries still agree with current source and policy.
3. Keep source-owner pages under `docs/reference/`, including pages for developer-only report generators.
4. Keep generated run output under ignored `tools/reports/` or another explicitly ignored output path.
5. Do not move or delete a page solely because its path contains `report`, `baseline`, `debug`, or `corpus`.

For documentation-root migrations, preserve more than file count. Build a per-file mapping that records the old path, new path, source owner, classification, and preservation rationale. Before staging or committing, verify that every deleted legacy page has exactly one replacement and that no replacement remains untracked while its old page is deleted.

## Why This Matters

Filename-based cleanup can remove the exact context needed to maintain developer tools and language infrastructure. Conversely, treating generated output as active reference material causes unnecessary reads and stale context.

Path-for-path migration checks are necessary but insufficient on their own. The mapped replacement must retain the same source-owner role, or the documentation can be preserved at the wrong location and still mislead future changes.

## When to Apply

- A page under `docs/reference/` looks like a report, debug artifact, baseline, or corpus output.
- A tool writes Markdown files and the repository is deciding whether to track those results.
- A documentation folder is being renamed, consolidated, or moved.
- A cleanup proposal recommends bulk deletion or relocation from filename patterns.

## Examples

| Item | Classification | Reason |
|---|---|---|
| `docs/reference/server/examples/parser_report.md` | Active source-owner context | It documents the Rust generator's behavior and boundaries. |
| `tools/reports/parser-fixtures.report.md` | Ignored generated output | It is a result produced by the generator for investigation. |
| A stale page with no valid source owner or policy role | Rewrite/delete candidate | Change it only after recording evidence and replacement context, if needed. |

## Prevention

- Require per-file classification before bulk documentation moves.
- Treat current source, accepted policy, explicit user direction, and applicable CE artifacts as the evidence gate for rewrites or deletions.
- Make migration preservation a pre-commit check, not an after-the-fact review note.
- Keep broad stale-document pruning in a separate plan unless the inventory identifies concrete high-confidence candidates.

## Related

- [Documentation policy](../../../AGENTS.md)
- [Agent workflow and migration preservation](../../agent-workflow.md)
- [Documentation Context Diet plan](../../plans/2026-07-18-003-docs-reference-context-plan.md)
