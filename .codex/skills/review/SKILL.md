---
name: review
description: Run an evidence-based, read-only review of a Reforger Script Tools scope using a relevant roster of Architecture, Correctness, Performance & Reliability, Developer Experience, Language Fidelity, and Verification & Observability personas, then synthesize their findings. Use when the user invokes /review, asks for an independent code or architecture review, or wants multi-perspective review before deciding on a fix.
---

# Parallel Review

Run a bounded, advisory review. Never edit source, planning artifacts,
configuration, Git state, or external systems during this skill. Generated
evidence journals under `tools/reports/review/` are the sole exception.

## 1. Establish the review package

1. Parse `depth:auto` (default), `depth:full`, and optional `personas:` tokens.
   The catalog is Correctness, Architecture, Performance & Reliability,
   Developer Experience, Language Fidelity, and Verification & Observability.
   In auto mode select Correctness and Architecture, then add at most two
   specialists when changed paths, requirements, or risk surfaces make their
   lens relevant. Select Language Fidelity for Enfusion syntax, parser,
   semantic, formatting, language-feature, Workbench, game-data, or API-truth
   work. Select Verification & Observability for defects, tests, fixtures,
   logs, diagnostics, reproducibility, lifecycle, or scheduler claims. Cap the
   roster at four and state why each persona was selected or skipped. Full is
   the deepest bounded review: core plus the two most relevant specialists.
   Explicit personas select the named lenses; retain the core unless the user
   explicitly asks a narrow persona-only review. If the resulting explicit
   roster exceeds four, request a narrower roster or a second review rather
   than silently omitting a persona.
   Canonical explicit values are `architecture`, `correctness`,
   `performance-reliability`, `developer-experience`, `language-fidelity`, and
   `verification-observability`, supplied as a comma-separated token. For
   example, `/review <scope> personas:language-fidelity,verification-observability`
   runs the core plus both named specialists. To split an over-cap review, run
   a focused follow-up with the omitted specialist rather than weakening the
   first review's coverage claim.
2. Read `AGENTS.md`, `git status --short`, and the explicit user scope.
3. If scope is explicit, include only its direct implementation, owning
   reference documentation, relevant tests, and bounded recent diagnostics.
4. If scope is absent or broad, infer the smallest defensible scope from
   current changed files, the active OpenSpec change, and their owning docs.
   State the selected scope and material omissions before reviewing.
5. Prepare and disclose one immutable review contract: scope/base, intent,
   requirements, symbols/callers, tests, docs, diagnostics, exclusions, and
   unknowns. Include `AGENTS.md` and every relevant owning reference page in
   the package. Do not include the coordinator's diagnosis or recommendations.

For Enfusion Script, Workbench, or game API claims, invoke `reforger` and give
every reviewer the verified API/source evidence rather than an unsupported
claim.

## 2. Load contracts and fan out independently

Read [common-review-contract.md](references/common-review-contract.md) and the
contracts for selected personas:

- [architecture.md](references/architecture.md)
- [correctness.md](references/correctness.md)
- [performance-reliability.md](references/performance-reliability.md)
- [developer-experience.md](references/developer-experience.md)
- [language-fidelity.md](references/language-fidelity.md)
- [verification-observability.md](references/verification-observability.md)

Create a unique run ID. Launch all selected reviewer sub-agents concurrently when capacity permits. Each
call MUST use `fork_turns: "none"`. Give each agent only:

- its persona contract;
- the shared evidence package; and
- the common review contract.

Give each reviewer only `tools/reports/review/<run-id>/<persona>.md`. It MUST
update that journal after every completed evidence slice and MUST NOT read a
peer journal. This is operational, not filesystem-security, isolation.

Do not provide parent conversation, the coordinator's provisional conclusion,
another persona's identity, status, output, or any review artifact from a
previous run. Instruct every reviewer to remain read-only, not spawn or message
agents, and return only its final structured report.

If capacity prevents all selected reviewers from starting, start the available
reviewers without delay and launch each remaining persona when a slot becomes available.
Use the exact same evidence package for delayed reviewers. Record this as a
capacity limitation; never omit a reviewer silently. In runtimes where the
coordinator itself occupies one of four agent slots, expect three reviewers to
start immediately and the fourth to begin after a slot is released.

If a selected reviewer fails, is interrupted, or does not return a conforming
report, retain any journal it produced and mark that persona unavailable. Then
synthesize only completed reports, disclose incomplete coverage and the absent
lens, and label the result a partial review. Do not silently retry with a
different persona or describe a partial review as complete.

## 3. Synthesize, do not debate

After all final reports arrive:

1. Reject findings without priority, confidence, evidence, impact, durable
   direction, and validation. Assign stable IDs.
2. Group findings by underlying issue only when defect and fix path match, retaining the evidence and contributing
   persona for each group.
3. Keep materially different conclusions visible. Do not manufacture a
   consensus or upgrade an inference to a fact.
4. Rank groups by priority, confidence, and likely user/system impact. Agreement
   can raise confidence, never priority.
5. Keep no-finding reports as coverage evidence and report meaningful strengths.
6. Give every unresolved P1-P3 item one disposition: fix now, planned task,
   accepted residual with owner/reason, or needs evidence.
7. Recommend one best next step. List alternatives only when they represent a
   genuine trade-off.

Return this format:

```md
## Review Scope
<reviewed paths, evidence, and material exclusions>

## Coverage
<selected personas, those completed, those unavailable, and capacity-limited scheduling if applicable>

## Strengths
- ...

## Findings
| Priority | ID | Finding | Evidence | Impact | Next Step | Confidence | Reviewers |
|---|---|---|---|---|---|---|---|
| P2 | P2-01 | ... | `path:symbol` | ... | Fix now / planned task / needs evidence | High | Correctness, Architecture |

## Disagreements and Unknowns
- ...

## Residual Work
- <P1-P3 ID>: fix now | planned task | accepted residual | needs evidence

## Recommended Next Step
<one concrete, evidence-backed action>

No source, configuration, Git, runtime, or external state changed; generated
review evidence was recorded. This review is advisory.
```

Do not implement the recommendation. If the user wants a change, direct them
to `/fix`, an OpenSpec proposal, or an explicit implementation request.

## 4. Validate roster behavior after a contract change

When changing persona selection, cap, or partial-coverage behavior, execute
the bounded scenarios in [roster-acceptance.md](references/roster-acceptance.md).
Record the selected, skipped, unavailable, and completed personas plus the
observed outcome in the final validation handoff. This is a documentation
workflow check, not a replacement for reviewing the actual change scope.
