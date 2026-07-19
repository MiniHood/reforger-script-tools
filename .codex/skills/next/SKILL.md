---
name: next
description: Continue the most relevant unfinished Reforger Script Tools task from the current conversation, or identify the highest-value next task through a bounded review of the extension, documentation, and code. Use when the user invokes /next, asks what to work on next, or wants Codex to continue before changing topics.
---

# Continue the Next Task

Treat the current conversation as the primary backlog. Do not switch topics
until checking whether the prior task left an explicit next step, residual, or
required validation.

## Select work in this order

1. The latest explicit user request that is incomplete.
2. A remaining or recommended next step from the prior handoff.
3. An active OpenSpec change with unchecked tasks.
4. A verified failing test, broken build, or debug root cause already found in
   the current conversation.
5. A narrowly attributable uncommitted change that needs verification,
   documentation, or commit handling.

If an item is clear and within the user’s authority, state the selected task in
one sentence and begin it. Use its applicable workflow skill. Do not reopen a
completed task or repeat checks already recorded as passing.

## No clear continuation

Perform a bounded orientation before proposing a new topic:

1. Read `AGENTS.md`, `git status`, the latest handoff, and the current
   architecture/reference pages for recently changed subsystems.
2. Inspect active OpenSpec changes and their task lists.
3. Check targeted repository health: relevant tests or lint only when they
   answer a concrete uncertainty; do not run broad suites by default.
4. Review recent code/docs changes and existing diagnostic evidence for one
   concrete gap, regression risk, performance bottleneck, or stale contract.

Create one concrete recommended next task before prompting the user or
suggesting a topic change. The recommendation must state its evidence,
expected value, owner layer, and smallest first verification. You may include
up to two alternatives only when they materially change scope or risk.

Choose and start the recommendation only when it is clearly a continuation or
a safe, user-authorized maintenance step. Otherwise present the recommendation
and ask whether to pursue it. Do not invent product work from a vague review
signal, and do not merely say there is nothing to do without completing the
bounded review.

## Guardrails

- Preserve TypeScript shell/Rust language-engine boundaries.
- Invoke `reforger` before making Enfusion Script behavior claims.
- Treat user changes as owned by the user; do not stage, commit, or rewrite
  unrelated work.
- Prefer evidence-backed vertical slices over broad refactors.
- Do not create a second implementation path merely to keep momentum.

## Handoff

State what was selected, why it was selected, what was completed, verification
performed, and the remaining or newly discovered next step.
