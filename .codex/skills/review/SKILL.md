---
name: review
description: Run an evidence-based, read-only review of a Reforger Script Tools scope using four independent personas—architecture, correctness, performance and reliability, and developer experience—and synthesize their findings. Use when the user invokes /review, asks for an independent code or architecture review, or wants multi-perspective review before deciding on a fix.
---

# Parallel Review

Run a bounded, advisory review. Never edit source, planning artifacts,
configuration, Git state, or external systems during this skill.

## 1. Establish the review package

1. Read `AGENTS.md`, `git status --short`, and the explicit user scope.
2. If scope is explicit, include only its direct implementation, owning
   reference documentation, relevant tests, and bounded recent diagnostics.
3. If scope is absent or broad, infer the smallest defensible scope from
   current changed files, the active OpenSpec change, and their owning docs.
   State the selected scope and material omissions before reviewing.
4. Prepare one immutable evidence package: scope, revision/changed-file state,
   relevant paths, observed behavior or logs, and known constraints. Do not
   include the coordinator's diagnosis or recommendations.

For Enfusion Script, Workbench, or game API claims, invoke `reforger` and give
every reviewer the verified API/source evidence rather than an unsupported
claim.

## 2. Load contracts and fan out independently

Read [common-review-contract.md](references/common-review-contract.md) and all
four persona contracts:

- [architecture.md](references/architecture.md)
- [correctness.md](references/correctness.md)
- [performance-reliability.md](references/performance-reliability.md)
- [developer-experience.md](references/developer-experience.md)

Launch all four reviewer sub-agents concurrently when capacity permits. Each
call MUST use `fork_turns: "none"`. Give each agent only:

- its persona contract;
- the shared evidence package; and
- the common review contract.

Do not provide parent conversation, the coordinator's provisional conclusion,
another persona's identity, status, output, or any review artifact from a
previous run. Instruct every reviewer to remain read-only, not spawn or message
agents, and return only its final structured report.

If capacity prevents all four from starting, start the available reviewers
without delay and launch each remaining persona when a slot becomes available.
Use the exact same evidence package for delayed reviewers. Record this as a
capacity limitation; never omit a reviewer silently. In runtimes where the
coordinator itself occupies one of four agent slots, expect three reviewers to
start immediately and the fourth to begin after a slot is released.

## 3. Synthesize, do not debate

After all final reports arrive:

1. Group findings by underlying issue, retaining the evidence and contributing
   persona for each group.
2. Keep materially different conclusions visible. Do not manufacture a
   consensus or upgrade an inference to a fact.
3. Rank groups by severity, confidence, and likely user/system impact.
4. Keep no-finding reports as coverage evidence and report meaningful strengths.
5. Recommend one best next step. List alternatives only when they represent a
   genuine trade-off.

Return this format:

```md
## Review Scope
<reviewed paths, evidence, and material exclusions>

## Coverage
<personas run; disclose capacity-limited scheduling if applicable>

## Strengths
- ...

## Findings
- [Severity | confidence] Title
  - Evidence: ...
  - Impact: ...
  - Durable direction: ...
  - Validation: ...

## Disagreements and Unknowns
- ...

## Recommended Next Step
<one concrete, evidence-backed action>

No files were changed; this review is advisory.
```

Do not implement the recommendation. If the user wants a change, direct them
to `/fix`, an OpenSpec proposal, or an explicit implementation request.
