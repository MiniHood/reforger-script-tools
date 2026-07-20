---
name: next
description: Continue the most relevant unfinished Reforger Script Tools task from the current conversation, or route ambiguous work through fix, review, and research workflows to identify the highest-value next task. Use when the user invokes /next, asks what to work on next, or wants Codex to continue before changing topics.
---

# Continue the Next Task

Treat the current conversation as the primary backlog. Do not switch topics
until checking whether the prior task left an explicit next step, residual, or
required validation.

## Route Work

`/next` is a thin router, not a second implementation, diagnosis, review, or
research workflow. Read only enough context to select the owner: the latest
handoff, explicit user requests, active OpenSpec tasks, current Git status, and
already-established tests, logs, or review residuals. Do not repeat deep work
that `/fix`, `/review`, or `/researcher` owns.

Route in this order:

1. **Clear unfinished request or OpenSpec task:** continue it through its
   owning workflow.
2. **Open defect:** when context contains a verified failing test, debug root
   cause, unresolved P1-P3 review item, explicit bug report, or an accepted
   residual that needs correction, invoke `/fix`. Pass the concrete symptom,
   evidence, scope, exclusions, and required validation; let `/fix` diagnose,
   design, implement, and verify the durable solution.
3. **Clear candidate, uncertain value or design:** invoke `/review` before
   recommending or beginning it. Pass only the bounded scope and evidence, not
   a preferred answer; let `/review` establish whether the candidate deserves
   work or needs a different direction.
4. **No concrete candidate:** invoke `/researcher` for the question "what is
   the highest-value next task in this bounded scope?" Use `sources:local` by
   default and `sources:both` only when outside practice can materially affect
   the choice. Let `/researcher` explore opportunities, alternatives,
   counterexamples, and durable options. Then invoke `/review` independently on
   the strongest research candidate, or on the decisive uncertainty when no
   candidate is safe to name.

Keep unrelated dirty work out of scope. Research and review are discovery and
challenge stages, not authorization to implement. If they disagree or leave a
decisive unknown, route to the smallest missing investigation rather than
inventing work to keep momentum.

Create one concrete recommended next task before prompting the user or
suggesting a topic change. State the routing evidence, research favorability
and evidence IDs when used, review findings or no-finding coverage when used,
expected value, owner layer, durable target, and smallest first verification.
Include alternatives only when they represent a material scope or risk
trade-off.

Choose and start the recommendation only when it is clearly a continuation or
a safe, user-authorized maintenance step. Otherwise present the recommendation
and ask whether to pursue it. Do not invent product work from a vague research
or review signal, and do not merely say there is nothing to do without routing
the available evidence to the appropriate workflow.

## Guardrails

- Preserve TypeScript shell/Rust language-engine boundaries.
- Invoke `reforger` before making Enfusion Script behavior claims.
- Treat user changes as owned by the user; do not stage, commit, or rewrite
  unrelated work.
- Prefer evidence-backed vertical slices over broad refactors.
- Do not create a second implementation path merely to keep momentum.

## Handoff

State what was selected, why it was selected, which workflow owned the work,
what was completed, verification performed, and the remaining or newly
discovered next step.
