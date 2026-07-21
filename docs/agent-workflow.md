# Agent Workflow

## Purpose

This page explains why the repository uses its current policy, documentation,
and evidence model. [AGENTS.md](../AGENTS.md) is the enforceable contract;
[the documentation procedure](documentation.md) defines how documentation is
created and maintained. This page does not add operational rules.

## A map that follows ownership

The project is deliberately arranged so the place that explains a concern is
also the place that owns it. [Architecture](reference/architecture.md) gives a
maintainer the cross-layer map. `docs/reference/` then mirrors source and
subsystem ownership, making it possible to start with a file and find the
behavior and boundaries that matter before changing it.

Generated investigation output belongs under `tools/reports/`, not in the
documentation hierarchy. A reference page can explain the generator and its
contract while its output remains disposable evidence.

## Deliberate evidence

The toolchain depends on high-fidelity language behavior, so evidence is
chosen for the claim at hand rather than accumulated as ceremony. Workbench or
compiler behavior is the strongest evidence for Enfusion Script behavior;
official documentation, verified extracted APIs, and labeled source samples
provide progressively weaker supporting context. This ordering keeps the
language server from silently becoming an authority on language truth.

The same principle applies to implementation work. A small, focused check is
more useful than repeating broad commands that answer the same question. Code
reviews, tests, and runtime checks each have a distinct role: they challenge a
change, prove behavior, and validate integration respectively.

## Independent review

`/review` is a read-only, advisory workflow for challenging a bounded scope
before deciding whether to change it. It prepares one evidence package and
uses a relevant roster from six independent lenses: architecture, correctness,
performance and reliability, developer experience, language fidelity, and
verification and observability, without sharing reviewer conclusions.
Correctness and architecture are the default core; the other lenses are
selected for relevant risk surfaces. A review runs no more than four personas:
full depth means the core plus the two most relevant specialists, not every
catalog entry. The coordinator then combines evidence, preserves material
disagreement, and recommends a next step; it never implements that
recommendation.

Deep review records concise generated evidence slices and a coverage verdict.
Its final report uses one deduplicated, priority-ordered findings table with
evidence, impact, next step, confidence, and contributing reviewers, plus an
explicit disposition for every unresolved P1-P3 finding. If a selected
reviewer cannot return a conforming report, the result explicitly reports
partial coverage rather than implying a complete review.

The coordinator waits for every selected reviewer to complete or become
formally unavailable before presenting one synthesized report. While a review
is active, it may provide operational status but must not disclose individual
findings, implement recommendations, commit, or otherwise change project
state. A fix requires a separate user-authorized action after the review.

Roster requests are deterministic: an explicit `personas:` selection controls
specialists while `depth:` controls thoroughness; `personas-only:` is the only
form that omits core reviewers. Ambiguous tokens require clarification. When
more than two specialists fit, the coordinator ranks direct scope ownership,
explicit user concern, then demonstrated failure or release risk and records
any displaced lens. A reviewer without a report or journal progress after two
coordinator wait intervals becomes unavailable and produces a partial review.

To request specialists directly, pass comma-separated canonical names in a
`personas:` token. For example,
`/review <scope> personas:language-fidelity,verification-observability` retains
the core reviewers and adds those two specialists. If a requested roster would
exceed four, split the omitted lens into a focused follow-up review; the
coordinator must not silently drop it.

Use `/debug` to establish a causal chain for an observed failure and `/fix` to
design, implement, and verify an authorized durable solution. Use `/review`
when the goal is broad, independent scrutiny rather than diagnosis or a code
change.

## Next-task discovery

`/next` is a thin router. It continues a clear unfinished user-authorized task,
sends verified or accepted defects to `/fix`, and sends a clear but uncertain
candidate to `/review`. Only when no candidate exists does it ask `/researcher`
to explore bounded opportunities, then asks `/review` to independently
challenge the strongest candidate without receiving the research ranking.
Unresolved evidence becomes the recommended investigation instead of invented
work.

## Parallel research

`/researcher` is the evidence-gathering companion to `/review`. It is used
before a decision when the question benefits from independent game-data/source
examples, codebase investigation, external practice, or option comparison. It
is read-only and produces a bounded research brief with evidence quality,
options, concrete action items, and the smallest recommended next step. Its
`sources:` mode is a hard evidence boundary, its brief records the source mix
and base revision, and incomplete personas produce explicitly partial coverage.
Research ranks a supported durable target state above a temporary workaround;
any mitigation must disclose its limitation and removal condition. Researchers
continue past a first plausible answer until evidence saturates or a different
authority is needed, then rank options transparently with supporting and
conflicting evidence, persona/source provenance, and favorability. For
Enfusion questions, game API/example research uses at least five independent
examples and expands across owners when the question asks about broad
applicability; online research cites direct sources and distinguishes an
observed outside practice from a repository recommendation. Research does not
implement its conclusion: use `/fix` or an OpenSpec proposal once a direction
is chosen. Researchers are curious evidence gatherers, not authorities: their
ideas remain suggestions. The main thread independently assesses and may reject
or reframe them before presenting a recommendation; `/review` remains available
when a separate full persona review is warranted.

## Development-host continuity

An Extension Development Host is part of the developer's live debugging
context, not a disposable test window. After the final extension-facing source
change, Codex runs `npm run compile` itself before handoff, even if an earlier
test command already built the project. The rebuild updates the extension
artifacts and replaces the development server binary for the existing host's
watcher; this must not be deferred to the developer or presented as a manual
reload step. Do not launch a new `--extensionDevelopmentPath` window, close VS
Code, or replace the host: those actions lose debug-menu state and do not prove
the active session loaded the rebuilt artifacts. If no available tool can
perform a live editor interaction afterward, only that interaction remains
pending; the build itself is complete.

## Preserving useful history

Documentation is valuable when it explains current ownership or constraints
that still shape the code. It becomes a liability when it preserves a
superseded architecture as if it were current. The documentation procedure
therefore favors concise, current owner pages and Git history for past changes.
