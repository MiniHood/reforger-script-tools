# Agent Workflow

## Purpose

This document explains how strict repository policy, Compound Engineering artifacts, source-reference documentation, and Reforger evidence fit together in this repository.
[AGENTS.md](../AGENTS.md) is the active policy contract; this file provides rationale and orientation.

## Document Ownership

`AGENTS.md` owns enforceable repository rules and should stay short and direct.
Put rationale, tradeoffs, and examples here or in the matching subsystem documentation instead of expanding the policy document.

[Architecture overview](reference/architecture.md) owns the canonical runtime flow and layer boundaries.
Matching `docs/reference/<source path>.md` pages own file-level behavior and boundaries.

`docs/reference/` owns durable source and subsystem context.
It explains why files exist, what they own, what behavior must be preserved, and what boundaries future work must respect.
It mirrors the source tree so relevant context is easy to find.

`tools/reports/` owns ignored generated report output.
It is a developer-output path, not a documentation tier and not a source-reference home.
For example, `docs/reference/server/examples/parser_report.md` documents the report generator, while `tools/reports/parser-fixtures.report.md` is generated investigation output.

`docs/plans/` owns Compound Engineering plan artifacts.
Plans capture scoped work, implementation units, verification contracts, and definition of done; they are not source ownership docs.

`CONCEPTS.md` may later capture stable project vocabulary, but it must not replace plans or source-reference documentation.

## Documentation Routing

Use active reference docs deliberately rather than treating every Markdown file as mandatory context.

| Situation | Documentation action |
|---|---|
| Non-trivial or architecture-sensitive source change | Read the matching active reference page in full when it exists. Update it when ownership, behavior, boundaries, or direction change. |
| Change across a subsystem boundary | Read the matching page and the relevant subsystem context. |
| Trivial metadata, generated output, dependency artifact, or formatting-only mechanical edit | No mirror page is required solely because a file changed. |
| Investigation of report output | Read the source-owner page under `docs/reference/`; inspect `tools/reports/` only when its output is needed as evidence. |
| Missing, stale, duplicative, or harmful page | Use current source, policy, accepted direction, or a current CE artifact before creating, rewriting, moving, or deleting it. |

A source change is non-trivial when it changes executed logic, commands, settings, data contracts, parser/LSP behavior, process/tool flow, or subsystem boundaries.
Architecture-sensitive work includes `server/`, `src/languageClient/`, `src/gameData/`, `src/extensionConfig/`, `tools/fixtures/`, and new non-trivial source folders.

## Compound Engineering Lifecycle

Use CE to keep broad work from collapsing into ungrounded edits.

- `ce-brainstorm` settles ambiguous user outcome, scope, and success criteria.
- `ce-plan` turns settled requirements into implementation units, file scope, tests, and verification gates.
- `ce-work` executes implementation-ready plans and leaves progress in code, verification output, and git rather than mutating the plan.
- `ce-debug` handles failures, regressions, and uncertain behavior.
- `ce-code-review` performs structured review; `ce-resolve-pr-feedback` handles review comments.
- CE commit and PR skills apply only when the user asks for the related Git operation.

When CE is unavailable, use equivalent structure for broad or architecture-sensitive work: scoped requirements, an implementation approach, focused verification evidence, and recorded uncertainty.

## Evidence Discipline

Rigor comes from distinct evidence, not from repeating tools or reviews. Before editing, define the smallest intended slice, the behavior or invariant to prove, the verification set, and the stop condition. Re-open scope only when a check fails, the implementation reveals a new uncertainty, or the user changes the task.

Every command must answer a question not already answered by earlier evidence. Select the smallest non-overlapping project commands:

- `npm test` is the final extension workflow when extension behavior requires it; its pretest path already runs type checking, linting, and compilation, so those commands must not run separately first.
- Rust behavior is verified from `server/` with the relevant `cargo test` invocation. Do not invoke Cargo from the repository root.
- Docs-only work uses `git diff --check` plus manual link/path review.
- The verified auto-commit helper owns one final selected check. Arm it with that check instead of manually running the same command and then repeating it through the helper.

Each review question has one owner. When a review produces an exact finding and the corrective scope is understood, inspect the changed lines and regression test after the fix. Restart broad review only when the correction changes architecture, expands the contract, fails focused verification, or introduces a separate concern.

Do not add additional process steps solely for ceremony. Final extension builds replace repo-owned language-server binaries, and the active extension/development host reloads once after that final build.

Stop when the declared invariant and final verification set pass. A passing command, completed review, or successful reload does not justify another confidence pass without a new signal. Final reports record verification, retries, and remaining uncertainty; only report timing or usage when the active surface provides trustworthy values.

## Reforger Evidence

Workbench/compiler behavior is final authority, and the `reforger` skill is the first grounding layer for Reforger facts.
Use it before reasoning about Enfusion Script syntax, APIs, attributes, callbacks, lifecycle, replication, resources, prefabs, configs, game data, fixtures, parser/model/index behavior, diagnostics, formatting, or LSP behavior.

Use evidence in this order:

1. Workbench/compiler behavior when available.
2. Official Reforger documentation.
3. Extracted APIs and verified game-data records.
4. Source samples and fixtures, labeled by confidence.

Do not infer Enfusion behavior from C#, Unity, Unreal, Arma 3, SQF, or generic scripting-language conventions.

## Documentation Preservation

Documentation should preserve useful project memory, not obsolete architecture.
When docs match current ownership, behavior, or constraints, keep them current.
When docs encode harmful legacy direction, rewrite or delete that content instead of preserving it as a burden.

Deletion or major rewrite needs an evidence gate. Ground the change in at least one of:

- current source behavior
- current accepted policy
- explicit user direction
- an applicable CE artifact that is current, names the affected path or subsystem, and does not contradict current source behavior, accepted policy, or source-reference ownership docs
- Workbench/compiler or Reforger source-backed evidence for language behavior

Preserve replacement context in the correct owner whenever any part of the old documentation remains useful.
If no replacement context is needed, say why in the change notes, commit message, or plan result.

CE plans are decision artifacts, not permanent proof.
Treat a plan as historical when a newer artifact supersedes its scope or current source/policy contradicts it.
An applicable plan supports a documentation decision only when it is current, names the affected path or subsystem, and remains consistent with source truth.

## Migration Preservation

For a documentation-root migration, create a per-file old-to-new mapping before moving files.
Before staging or committing, verify that every deleted legacy path has exactly one expected replacement and that no replacement remains merely untracked while its old path is deleted.
This checkpoint prevents a partial commit from dropping durable project context.

## Verification Philosophy

The verification loop in `AGENTS.md` is the default for repository work.
Code and behavior-bearing changes should run the relevant project commands and targeted manual validation needed for the touched subsystem, once each and only when they add evidence not supplied by another selected command.

Docs-only changes can be verified with `git diff --check` and manual link/path review when no source, package, build, or runtime behavior changed.
That exception is narrow. It does not justify skipping tests for code, language behavior, extension activation, packaging, or user-visible workflows.

For Reforger-language behavior, local tests and repo checks are not enough when the claim depends on compiler or engine truth.
Use Workbench/compiler validation whenever available and record what remains unverified when it is not.

## Practical Routing

For an ambiguous architecture change, begin with `ce-brainstorm` or `ce-plan` and create a plan under `docs/plans/` when a durable decision artifact is useful.

For parser, model, index, or LSP work, invoke `reforger`, read the matching `docs/reference/server/...` pages, inspect the relevant Rust source, and use a CE plan when the change is non-trivial.

For docs-only cleanup, read the affected docs in full, determine whether the content is current context or harmful legacy direction, apply the evidence gate, and verify links and whitespace.

For small mechanical source changes, read relevant reference docs, make the smallest change, run focused checks, and update documentation only when the source ownership or behavior changes.
