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

Plans and solutions serve different kinds of memory. [Plans](plans/) record why
a bounded piece of work was chosen; they are historical once source and policy
move on. The [solution store](solutions/) records reusable conclusions from
resolved problems. Searching it before reopening familiar territory helps
preserve hard-won constraints without mistaking an old plan for current
architecture.

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
more useful than repeating broad commands that answer the same question. Plans,
reviews, tests, and runtime checks each have a distinct role: they establish a
decision, challenge it, prove behavior, and validate integration respectively.

## Compound Engineering as continuity

Compound Engineering artifacts make significant work understandable after the
original session ends. Brainstorming resolves uncertain outcomes, plans record
the chosen slice and verification intent, execution leaves the result in source
and tests, and review or debugging supplies focused challenge when needed.
The resulting plan is useful historical context, while current source-owner
pages and solution records remain the places to look for present behavior and
durable lessons.

## Preserving useful history

Documentation is valuable when it explains current ownership, constraints, or
decisions that still shape the code. It becomes a liability when it preserves a
superseded architecture as if it were current. The documentation procedure
therefore favors concise current owners, historical plans for decisions, and
solutions for recurring lessons. This separation lets the project discard
obsolete timelines without discarding the reasoning future maintainers need.
