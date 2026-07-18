---
title: Use Distinct Verification Evidence
date: 2026-07-18
category: workflow-issues
module: agent-workflow
problem_type: workflow_issue
component: development_workflow
severity: medium
applies_when:
  - "Selecting checks for a bounded implementation or documentation change"
  - "Deciding whether a validator, review, rebuild, or reload adds new evidence"
  - "Using the verified Refactor auto-commit protocol"
tags: [verification, workflow, code-review, lsp, agent-routing]
---

# Use Distinct Verification Evidence

## Context

Strong engineering evidence can become wasteful when several tools prove the
same fact. In this repository, `npm test` already runs type checking, linting,
compilation, and the extension test host through its pretest path. Running each
of those commands first and then running `npm test` repeats the same work
without improving confidence.

The same rule applies to review and lifecycle work. A concrete review finding
needs a focused check of its correction and regression test, not an automatic
repeat of the entire review. A final extension build replaces the packaged
language-server binary; reload the active extension host once after that build
instead of repeatedly restarting or inspecting the same process.

## Guidance

Before editing, define the smallest intended slice, the invariant to prove, the
non-overlapping verification set, and the stop condition. Each command, agent,
or manual check must answer a question not already answered by earlier
evidence.

Use these command boundaries:

1. For extension behavior, use `npm test` as the final extension workflow; do
   not separately run its type, lint, or compile prerequisites.
2. For Rust behavior, run the relevant `cargo test` command from `server/`.
3. For docs-only work, use `git diff --check` and manual link/path review.
4. Arm the verified auto-commit helper with the final selected check instead of
   manually running that same command and then repeating it through the helper.

Use subagents and validators only when they provide independent evidence, such
as an unfamiliar source-of-truth investigation, a competing architectural
option, a broad review, or an independently executed check that tests a
different concern. One review question has one owner.

## Why This Matters

Repeating commands, broad reviews, or lifecycle actions raises wall time and
token use while making the real evidence harder to identify. A clear evidence
boundary preserves the capacity for deep reviews and architectural work where
additional investigation genuinely changes the decision.

## When to Apply

- A finding already identifies the affected code, expected correction, and
  focused regression test.
- Several candidate commands share prerequisites or prove the same behavior.
- A completed extension build has already replaced the language-server binary.
- A broad review has narrowed the follow-up to one known issue.

## Examples

| Change | Distinct evidence | Avoid |
|---|---|---|
| Rust LSP behavior fix | Targeted `cargo test` from `server/`, one final `npm test`, one extension reload | Repeating `cargo test` from the repository root or running `npm` prerequisites before `npm test` |
| Review finding correction | Inspect changed lines and run the new regression test | Restarting the full review without a new risk signal |
| Docs-only policy update | `git diff --check` and manual link review | Building the extension or restarting the language server |

## Related

- [Verification policy](../../../AGENTS.md)
- [Evidence discipline](../../agent-workflow.md#evidence-discipline)
- [Verified auto-commit helper](../../reference/tools/verified-refactor-auto-commit.md)
