---
name: researcher
description: Run parallel, independent research on Reforger Script Tools ideas, defects, feature directions, game APIs, examples, external practices, architecture, and implementation options. Use when the user invokes /researcher or wants evidence-led brainstorming, concrete options, action items, or further investigation before choosing a fix or proposal.
---

# Parallel Research

Investigate a question broadly enough to make a decision, without silently
turning research into implementation. Researchers are curious evidence seekers:
they should follow promising contradictions, alternatives, and gaps rather than
defending a preferred solution. Their conclusions are advisory suggestions, not
orders, commitments, or a guarantee that the main thread will follow them.
The output is a practical research brief, not a generic list of ideas.

## Input and Scope

Accept a research question plus optional tokens:

- `depth:auto` (default) or `depth:full`
- `personas:<comma-separated names>` or `personas-only:<...>`
- `sources:local`, `sources:online`, or `sources:both` (default)

Valid personas are `game-api-examples`, `online-research`, `codebase`,
`architecture`, `language-semantics`, `performance-reliability`,
`developer-experience`, and `verification`. Reject duplicate or malformed
tokens. If the question is too vague to establish a useful boundary, ask one
open question; otherwise make and state a bounded assumption.

Research is read-only. Do not edit source, settings, planning artifacts, Git
state, or external systems. Generated evidence journals under
`tools/reports/research/<run-id>/` are the sole exception. A user must invoke
`/fix`, `/opsx:propose`, or explicitly request implementation after the brief.

## Build an Evidence Package

1. Read `AGENTS.md`, `git status --short`, the stated question, relevant
   reference pages, active OpenSpec artifacts, and the smallest direct code
   paths.
2. State the research contract: question, decision it supports, included
   surfaces, constraints, existing evidence, exclusions, and unknowns.
3. For Enfusion/Workbench/game-data questions, invoke `reforger` before
   researching language or API truth. Give agents verified game-data signatures
   and source examples, not assumed behavior.
4. Use current local logs or reports only when they directly answer the
   question. Treat logs as observations, not ground truth.

## Select Independent Personas

Select three or four personas in `auto` mode. Start with Codebase and
Architecture, then add Game API & Examples for Enfusion/game-data scope,
Online Research when external practice can inform the decision, Language
Semantics for language truth, Performance & Reliability for typing/runtime
cost, Developer Experience for editor interaction, or Verification for a
reproduction/proof gap. Choose only lenses that can materially change the
decision and state selected and displaced relevant personas. Explicit
`personas:` retains Codebase and Architecture; `personas-only:` uses exactly
the named roster. Never exceed four in one run; split a larger request into a
follow-up research pass.

Persona contracts:

| Persona | Investigates | Required output |
|---|---|---|
| Game API & Examples | Extracted APIs, game source, real usage patterns | At least five distinct relevant examples; for broad claims, seek 10+ across different owners or use cases, then report saturation and gaps. |
| Online Research | Official docs, upstream projects, standards, maintainer guidance, comparable tools | Direct links, source quality, date/context, and what transfers or does not transfer to this project. |
| Codebase | Current implementation, callers, tests, logs, active changes | Ownership map, concrete integration points, and existing reusable mechanisms. |
| Architecture | Boundaries, authority, lifecycle, evolution cost | Durable design constraints and rejected coupling. |
| Language Semantics | Parser/model behavior and language truth | Verified rules versus inference; required compiler/Workbench checks. |
| Performance & Reliability | Typing path, memory, concurrency, cancellation, error modes | Cost model, measurable risks, and a bounded validation plan. |
| Developer Experience | User flow, discoverability, editor behavior | Expected interaction, failure modes, and acceptance examples. |
| Verification | Reproduction, fixtures, diagnostics, testability | Minimal proof plan and remaining observability gaps. |

For game-example research, count independent examples rather than repeated
occurrences of one pattern. If the user asks whether a pattern applies broadly,
expand across subsystems/owners until the evidence is representative or the
search is saturated. Never claim universality from a small sample.

For online research, use the web tool. Prefer primary documentation, official
repositories, standards, maintainer statements, and release notes. Cite every
external claim with a direct link. Separate an observed practice from a
recommendation for this repository.

## Fan Out and Synthesize

Create a unique run ID. Launch selected researchers independently with
`fork_turns: "none"` when capacity permits. Give each only its persona contract,
the shared evidence package, and its own journal path. Do not provide peer
identities, findings, or a preferred conclusion. If capacity delays a persona,
launch it when capacity frees and disclose the delay. Mark an unavailable
persona as incomplete rather than substituting an unlabelled conclusion.

Require each researcher to distinguish facts, inferences, examples, source
quality, uncertainties, options, and validation. Ask curious questions, seek
counterexamples, and report where an apparently attractive option does not fit.
They must not implement, message agents, read peer journals, or present a
recommendation as an instruction.

After all researchers finish or are unavailable:

1. Deduplicate evidence by underlying question, retaining source quality and
   disagreements.
2. Build two to four viable options. For each, state expected benefit, cost,
   risk, architectural fit, performance implications, and proof needed.
3. Turn the strongest evidence into concrete action items. Include discovery
   work when evidence is not sufficient for a safe change.
4. Perform an independent main-thread assessment. Re-check the question against
   the repository's constraints and evidence without treating researcher
   suggestions as instructions. Explicitly accept, reject, or reframe the
   leading suggestions and identify any missing trade-off.
5. Recommend one next action only when the independent assessment supports it;
   otherwise recommend the smallest decisive investigation. A follow-up
   `/review` remains the separate option for a full independent persona review
   before a high-risk decision.

Use this output format:

```md
## Research Question
<decision being supported>

## Scope and Coverage
<sources, personas, examples found, exclusions, unavailable lenses>

## Evidence
| Topic | Evidence | Source Quality | What It Means |
|---|---|---|---|

## Options
| Option | Benefits | Costs / Risks | Fit | Validation |
|---|---|---|---|---|

## Concrete Action Items
1. ...

## Independent Assessment
<what the main thread accepts, rejects, or reframes, and why>

## Unknowns and Follow-up Research
- ...

## Recommended Next Step
<one evidence-backed action, or the smallest decisive investigation>

No implementation was performed; generated research evidence was recorded.
```

Do not overstate certainty. Preserve disagreements and explicitly state when
Workbench/compiler or live-editor validation is required. The final
recommendation remains advice for the user; never imply that research output
authorizes implementation.
