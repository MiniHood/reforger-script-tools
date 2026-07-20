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

`depth:auto` uses the smallest source-compatible roster and evidence search
that can reach a justified stopping condition. `depth:full` expands to every
material source-compatible lens, explores at least
one credible competing direction and counterexample for each leading direction,
and records why any remaining lens or corpus was not pursued. Full depth does
not manufacture irrelevant personas or weak alternatives.

`sources:` is a hard evidence boundary, not a preference:

- `local`: use repository files, supplied logs, extracted data, and local
  tools only. Do not select Online Research or browse the web.
- `online`: use direct external sources only for substantive research evidence.
  Read `AGENTS.md` and Git status solely to enforce repository policy; do not
  inspect local implementation, logs, game data, or fixtures as evidence.
- `both`: use each eligible source deliberately and label its origin.

Reject an explicit persona that requires a forbidden source instead of silently
weakening the selected mode. In `online` mode, only Online Research is eligible;
use `sources:both` when a question also needs project-specific evidence.

Research is read-only. Do not edit source, settings, planning artifacts, Git
state, or external systems. Generated evidence journals under
`tools/reports/research/<run-id>/` are the sole exception. A user must invoke
`/fix`, `/opsx:propose`, or explicitly request implementation after the brief.

## Build an Evidence Package

1. Read `AGENTS.md`, `git status --short`, and the stated question. In `local`
   or `both` mode, then read relevant reference pages, active OpenSpec
   artifacts, and the smallest direct code paths; in `online` mode, keep local
   reading to policy/status control-plane facts only.
2. Pin one immutable evidence package: run ID, base commit (or `unborn`), Git
   status summary, source mode, supplied paths/sources, constraints,
   exclusions, and unknowns. For each supplied local file, record its path,
   Git revision when clean, or SHA-256 content fingerprint when dirty or
   untracked. Before citing a local file, verify the recorded identity; label a
   changed or newly read file as post-snapshot evidence rather than merging it
   into the baseline.
3. State the research contract: question, decision it supports, included
   surfaces, constraints, existing evidence, exclusions, and unknowns.
4. For Enfusion/Workbench/game-data questions, invoke `reforger` before
   researching language or API truth. Give agents verified game-data signatures
   and source examples, not assumed behavior.
5. Use current local logs or reports only when they directly answer the
   question. Treat logs as observations, not ground truth.

## Select Independent Personas

Select every source-compatible persona in `auto` mode that can materially
change the decision. With `local` or
`both`, start with Codebase and Architecture; with `online`, start with Online
Research. Add only a lens that can materially change the decision: Game API &
Examples for Enfusion/game-data scope, Online Research when external practice
can inform the decision, Language Semantics for language truth, Performance &
Reliability for typing/runtime cost, Developer Experience for editor
interaction, or Verification for a reproduction/proof gap. A justified
one- or two-persona roster is preferable to performative coverage.

Explicit `personas:` retains source-compatible Codebase and Architecture;
`personas-only:` uses exactly the named source-compatible roster. Never
silently omit a requested or materially relevant lens because of a numeric
cap. If runtime capacity delays a selected persona, launch it when capacity
frees and disclose the delay or partial coverage.
State selected, displaced, and source-ineligible personas with reasons. Also
record every materially considered but unselected persona and its reason; do
not imply complete coverage from the selected roster alone.

Persona contracts are intentionally distinct. Read the common contract and the
selected persona files before fan-out; give each agent only its own contract:

- [Common research contract](references/common-research-contract.md)
- [Game API & Examples](references/game-api-examples.md)
- [Online Research](references/online-research.md)
- [Codebase](references/codebase.md)
- [Architecture](references/architecture.md)
- [Language Semantics](references/language-semantics.md)
- [Performance & Reliability](references/performance-reliability.md)
- [Developer Experience](references/developer-experience.md)
- [Verification](references/verification.md)

The contract defines the lens, evidence threshold, counter-evidence to seek,
and deliverable. Do not flatten the roster into generic code reviews. A
persona may identify an adjacent concern, but must hand it off as a question
for the appropriate lens instead of duplicating its investigation.

After changing source-mode, roster, partial-coverage, evidence-package, or
synthesis behavior, run the bounded scenarios in
[Researcher Contract Acceptance](references/roster-acceptance.md).

For game-example research, count independent examples rather than repeated
occurrences of one pattern. If the user asks whether a pattern applies broadly,
expand across subsystems/owners until the evidence is representative or the
search is saturated. Never claim universality from a small sample.

When Online Research is selected in a source mode that permits it, use the web
tool. Prefer primary documentation, official repositories, standards,
maintainer statements, and release notes. Cite every external claim with a
direct link. Separate an observed practice from a recommendation for this
repository.

## Remain Curious Until Evidence Saturates

Do not stop after finding the first plausible solution. Treat it as a working
hypothesis. Each selected researcher must seek a credible alternative,
counterexample, failure mode, or condition under which the leading direction
does not fit. Explore adjacent implementations or source corpora when that can
materially change the decision.

Stop only when the coordinator can state all of the following: selected lenses
examined the leading hypothesis and its meaningful challenge; new evidence is
mostly confirmatory or the remaining uncertainty requires a different authority
(such as Workbench, a live editor, or user decision); rejected directions have
an evidence-backed reason; and no unexamined material lens remains. If those
conditions are not met, report incomplete research and recommend the smallest
decisive next investigation instead of a premature solution.

## Fan Out and Synthesize

Create a unique run ID. Launch selected researchers independently with
`fork_turns: "none"` when capacity permits. Give each its persona contract,
the common contract, the immutable evidence package, source-mode allowance,
and its own journal path. Do not provide peer
identities, findings, or a preferred conclusion. If capacity delays a persona,
launch it when capacity frees and disclose the delay. Mark an unavailable
persona as incomplete rather than substituting an unlabelled conclusion.

A persona is unavailable when it fails, is interrupted, returns a malformed
report, or has neither a final report nor a journal update after two coordinator
wait intervals. Retain any journal, record the reason, and synthesize a clearly
labelled partial brief from completed reports; do not silently retry with a
different persona.

Require each researcher to distinguish facts, inferences, examples, source
quality, uncertainties, options, and validation. Ask curious questions, seek
counterexamples, and report where an apparently attractive option does not fit.
They must not implement, message agents, read peer journals, or present a
recommendation as an instruction.

After all researchers finish or are unavailable:

1. Report coverage: selected, completed, delayed, unavailable with reason,
   materially considered but unselected personas, source mode, actual source
   classes, base revision, local file identities, post-snapshot evidence,
   saturation rationale, and material exclusions.
2. Assign stable evidence IDs and deduplicate by underlying question, retaining
   source quality, persona, source class, and disagreements.
3. Build one to four viable options. Rank a durable target state first whenever
   evidence supports it. A workaround is allowed only when explicitly labelled
   temporary, with its limitation, cost of delay, and concrete removal
   condition; never present it as equivalent to the durable option merely
   because it is cheaper. If only one option is evidence-supported, say so and
   explain why alternatives were rejected.
4. Give every option a favorability: **Strongly favored**, **Favored**,
   **Conditional**, or **Not favored**. Cite its supporting and conflicting
   evidence IDs with persona/source-class provenance, then state the ranking
   rationale across language correctness, architecture, performance,
   developer experience, delivery risk, and proof burden as applicable. The
   rank is a transparent synthesis of independent evidence, not a vote or an
   instruction from a researcher.
5. Turn the strongest evidence into concrete action items. Include discovery
   work when evidence is not sufficient for a safe change.
6. Perform an independent main-thread assessment. Re-check the question against
   the repository's constraints and evidence without treating researcher
   suggestions as instructions. Explicitly accept, reject, or reframe the
   leading suggestions, including why a durable direction outweighs or does not
   outweigh a temporary mitigation.
7. Recommend one next action only when the independent assessment supports it;
   otherwise recommend the smallest decisive investigation. A follow-up
   `/review` remains the separate option for a full independent persona review
   before a high-risk decision.

Use this output format:

```md
## Research Question
<decision being supported>

## Scope and Coverage
<source mode, actual source classes, base revision, selected/completed/delayed/
unavailable personas with reasons, materially considered but unselected
personas, examples found, exclusions, local file identities, post-snapshot
evidence, and saturation/stopping rationale>

## Evidence
| ID | Topic | Evidence | Persona / Source Class | Source Quality | What It Means |
|---|---|---|---|---|---|

## Options
| Rank / Favorability | Option | Durable or temporary | Supporting / Conflicting Evidence | Ranking Rationale | Benefits | Costs / Risks | Fit | Validation |
|---|---|---|---|---|---|---|---|---|

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
