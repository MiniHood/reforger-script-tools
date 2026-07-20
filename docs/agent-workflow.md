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
uses a relevant roster from four independent lenses: architecture, correctness,
performance and reliability, and developer experience, without sharing reviewer
conclusions. Correctness and architecture are the default core; the other
lenses are selected for relevant risk surfaces, or all four can be requested
for a full review. The coordinator then combines evidence, preserves material
disagreement, and recommends a next step; it never implements that
recommendation.

Deep review records concise generated evidence slices and a coverage verdict.
Its final report uses one deduplicated, priority-ordered findings table with
evidence, impact, next step, confidence, and contributing reviewers, plus an
explicit disposition for every unresolved P1-P3 finding. If a selected
reviewer cannot return a conforming report, the result explicitly reports
partial coverage rather than implying a complete review.

Use `/debug` to establish a causal chain for an observed failure and `/fix` to
design, implement, and verify an authorized durable solution. Use `/review`
when the goal is broad, independent scrutiny rather than diagnosis or a code
change.

## Preserving useful history

Documentation is valuable when it explains current ownership or constraints
that still shape the code. It becomes a liability when it preserves a
superseded architecture as if it were current. The documentation procedure
therefore favors concise, current owner pages and Git history for past changes.
