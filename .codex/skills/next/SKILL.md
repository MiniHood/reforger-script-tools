---
name: next
description: Continue the most relevant unfinished Reforger Script Tools task from the current conversation, or route ambiguous work through fix, review, and research workflows to identify and begin the highest-value next task. Use when the user invokes /next, asks what to work on next, or wants Codex to continue before changing topics.
---

# Continue the Next Task

Treat the current conversation as the primary backlog. Preserve the active
topic until it is complete, blocked by a real dependency, or the user explicitly
pivots. Do not let an older active OpenSpec change silently replace the most
recent significant user-directed thread.

## Select the Active Thread

Read only enough context to route the work: the latest handoff, explicit user
direction, active OpenSpec tasks, current Git state, and established evidence,
tests, logs, reviews, or research.

Select in this order:

1. Explicit current direction wins. Continue a named feature, an accepted plan,
   a user-reported follow-up, or the next step from the immediately preceding
   handoff.
2. A recent concrete residual or validation gate stays active. Do not replace it
   with unrelated backlog work.
3. Treat OpenSpec as contextual, not a global override. Use an active change
   only when it is named, clearly active, or directly continues the current
   thread.
4. Resolve a real tie only between materially related candidates, using value
   and current evidence. Do not enumerate a broad backlog merely to find work.

## Route and Start

Route the selected work in this order:

1. Continue a clear unfinished request or linked active OpenSpec task through
   its owning workflow.
2. Route a verified defect, accepted review finding, or failed validation to
   /fix with its symptom, evidence, scope, exclusions, and required checks.
3. Route a clear but uncertain candidate to /review with the bounded scope and
   evidence, without giving it a preferred answer.
4. If no concrete candidate exists, route to /researcher for the highest-value
   task in the bounded scope, then route the strongest result through /review.

## Execution Contract

When the selected work is the same active chain and has an established plan,
accepted residual, or clearly ordered next slice, start it immediately. Do not
ask the user to confirm a continuation already requested through /next. This
includes invoking the owning /fix, /review, or /researcher workflow.

When the chain moves from implementation to independent review or research,
invoke that workflow directly. Use its result to continue the chain or identify
the next decisive step. Review and research remain advisory; they do not
silently expand product scope.

Before taking action, state:

Continuing: <task>. Why: <current evidence and expected value>. Owner: <workflow or layer>.

Only pause for user direction when the next action needs new authority, a
material product or design choice, a user-controlled external session/result,
or a genuine blocker. State the exact missing decision or observation and why
it is necessary. Do not pause merely because continuation needs source changes,
verification, review, research, or a different owning workflow.

After a review or research stage identifies a clear durable path, continue to
the appropriate implementation or fix workflow. If it leaves a decisive
unknown, run the smallest investigation that resolves it rather than inventing
work to keep momentum.

## Guardrails

- Preserve the TypeScript shell and Rust language-engine boundary.
- Invoke reforger before making Enfusion Script behavior claims.
- Keep unrelated dirty work out of scope.
- Treat review and research as discovery, not direct authorization to alter
  product scope.
- Prefer evidence-backed vertical slices over broad refactors.
- Do not create a second implementation path merely to keep momentum.

## Handoff

State what was selected, why it was selected, which workflow owned the work,
what was completed and verified, and the remaining or newly discovered next
step. Do not imply a feature is complete when a required external validation
or concrete follow-up remains.
