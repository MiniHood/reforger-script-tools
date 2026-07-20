---
name: next
description: Continue the most relevant unfinished Reforger Script Tools task from the current conversation, or identify the highest-value next task through independent research and review of the extension, documentation, and code. Use when the user invokes /next, asks what to work on next, or wants Codex to continue before changing topics.
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

Use independent discovery before proposing a new topic:

1. Establish a small discovery contract from `AGENTS.md`, `git status`, the
   latest handoff, active OpenSpec work, recently changed ownership pages, and
   existing diagnostics. Keep unrelated dirty work out of scope.
2. Invoke `/researcher` for the question “what is the highest-value next task
   in this bounded scope?” Use `sources:local` by default; use `sources:both`
   only when outside practice can materially affect the choice. Let researcher
   explore every helpful persona, alternatives, counterexamples, and durable
   options. Do not give it a preferred answer.
3. Invoke `/review` independently on the strongest research candidate, or on
   the decisive uncertainty if research cannot safely name one. Give review the
   bounded scope and evidence package, but not the research ranking or
   recommendation. Use its relevant roster to challenge correctness,
   architecture, user impact, performance, language fidelity, and proof gaps.
4. Compare the completed research brief and independent review. Preserve
   disagreement. Select a next task only when both establish a concrete value,
   authoritative owner, durable direction, and smallest decisive verification.
   Otherwise recommend the missing investigation, not speculative product work.

Create one concrete recommended next task before prompting the user or
suggesting a topic change. State the research favorability/evidence IDs, review
findings or no-finding coverage, expected value, owner layer, durable target,
and smallest first verification. Include alternatives only when they represent
a material scope or risk trade-off.

Choose and start the recommendation only when it is clearly a continuation or
a safe, user-authorized maintenance step. Otherwise present the recommendation
and ask whether to pursue it. Do not invent product work from a vague research
or review signal, and do not merely say there is nothing to do without the
independent discovery pass.

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
