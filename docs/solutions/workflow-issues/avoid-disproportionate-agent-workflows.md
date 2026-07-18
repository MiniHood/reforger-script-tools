---
title: Keep Agent Workflow Proportional to the Change
date: 2026-07-18
category: workflow-issues
module: agent-workflow
problem_type: workflow_issue
component: development_workflow
severity: medium
applies_when:
  - "Fixing a bounded documentation, policy, or configuration inconsistency"
  - "Choosing whether a CE workflow, subagent pass, or live smoke test is justified"
  - "A reviewer finding has already narrowed the required correction"
tags: [agent-workflow, proportionality, verification, review-scope]
---

# Keep Agent Workflow Proportional to the Change

## Context

A small policy correction expanded into repeated review passes, broad validation,
and an attempted live delegation smoke test. The correction itself was one
wording change: align the critical-recovery rule with the already-established
Sol High escalation rule.

The extra work did uncover useful defects earlier in the routing implementation,
but once the remaining issue was isolated, repeating the broad workflow added
delay without materially increasing confidence. A bounded change must retain a
bounded evidence budget.

## Guidance

Choose the workflow from the current change surface, not from the name of the
skill that was invoked earlier in the session.

1. Classify the immediate change before acting. A one-file policy or document
   correction with an authoritative source in the same repository is bounded.
2. Define the smallest evidence that proves that correction. For a policy
   contradiction, read both statements, make one exact edit, search for the
   stale wording, and run `git diff --check`.
3. Escalate only for a new signal: changed runtime behavior, an unresolved
   technical uncertainty, a cross-subsystem contract, or a failed focused
   check. A reviewer finding that narrows the patch is not by itself a reason
   to restart a full review or validation cycle.
4. Treat optional expensive checks separately. If a live model-dispatch smoke
   test would validate a different concern than the edited wording, record it
   as unverified rather than running it as part of the small fix.
5. Stop when the planned focused evidence passes. Do not add another review
   pass solely to reconfirm a correction already checked against its
   authoritative source.

A narrowly scoped mechanical action can follow the focused check when it has
its own explicit evidence boundary. The verified `Refactor` auto-commit helper
is one example: it records the result of the selected final check and the
trusted stop hook only consumes that fresh receipt. It does not turn a passing
command into permission for broader review, pushes, or branch operations.

## Why This Matters

Overworking a simple correction burns time, model usage, and attention that
should remain available for consequential engineering work. It also makes the
actual change harder to review because a narrow fix becomes mixed with process
noise and unrelated validation.

The repository already distinguishes bounded work from normal and
consequential work in [AGENTS.md](../../../AGENTS.md). This practice applies
that same risk discipline to agent orchestration and verification effort.

## When to Apply

- A plan, policy, or documentation review identifies one internal
  contradiction with an unambiguous authoritative statement elsewhere.
- A change affects one or two files and has no runtime, public-contract, or
  semantic-language impact.
- A broad workflow has already produced the concrete correction and further
  work would only repeat checks rather than test a new risk.

## Examples

| Change | Proportionate response | Avoid |
|---|---|---|
| Correct one policy rule to match the detailed escalation state machine | Read the conflicting statements, patch the rule, search for stale wording, run `git diff --check` | Re-running the complete routing implementation review and live-agent smoke matrix |
| Add a new named agent registration | Parse strict configuration and verify the registration path | Treating configuration parsing as proof that every role behavior was exercised |
| Change language-server routing or runtime behavior | Use the relevant plan, focused tests, and independent validation | Reducing verification merely because the diff is small |

## Related

- [Agent workflow](../../agent-workflow.md)
- [Cost-aware routing plan](../../plans/2026-07-18-004-feat-cost-aware-agent-routing-plan.md)
- [Verified auto-commit helper](../../reference/tools/verified-refactor-auto-commit.md)
