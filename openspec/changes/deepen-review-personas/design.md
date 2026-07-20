## Context

The existing `/review` skill has four isolated personas but each receives only a brief focus list. It lacks an auditable review scope, a deep-review stopping condition, calibrated priority semantics, and durable handling of important findings. The revised design takes the useful parts of Compound Engineering's evidence and synthesis discipline without adding its large adaptive reviewer catalog or review-time mutation.

## Goals / Non-Goals

**Goals:**

- Make each of the four fixed reviewers perform a deep, persona-specific, slice-based investigation.
- Keep reviewer reasoning independent while preserving concise, inspectable evidence journals.
- Ground review in the requested change, requirements, affected symbols, callers, tests, docs, and diagnostics.
- Separate priority from confidence and require a disposition for every material issue.

**Non-Goals:**

- Adding a fifth default reviewer, cross-model review, automatic fixes, or repository mutations.
- Requiring private chain-of-thought disclosure; journals record questions, inspected evidence, conclusions, and next slices only.

## Decisions

### Frozen review contract and coverage map

Before dispatch, the coordinator creates a shared immutable contract: target/base, intent, requirements source, changed or relevant symbols, callers, tests, docs, diagnostics, exclusions, and settled decisions. Every reviewer receives that package without coordinator conclusions. Each reviewer must mark every assigned item as inspected, intentionally excluded, or unknown before finishing.

### Persona journals in generated output

Each persona updates a Markdown journal at `tools/reports/review/<run-id>/<persona>.md` after every evidence slice. A slice contains a focused question, sources inspected, concise conclusion, linked finding ID if any, and next slice. Generated journals are not source, planning, or runtime state. Reviewers receive only their own journal path and MUST not inspect another journal.

### Fixed reviewer roster with deep contracts

The fixed roster remains Architecture, Correctness, Performance & Reliability, and Developer Experience. Each contract supplies its own mandatory slices, evidence sources, priority examples, exclusions, and a coverage verdict. Deep review adds slices rather than reviewers, avoiding overlapping default personas.

### P1-P4 priority plus independent confidence

P1 means active stop/mitigate now; P2 is critical and must resolve before release; P3 is material planned work; P4 is low-impact improvement. Confidence is high, medium, or low and measures the proof quality. Agreement can raise confidence but never priority. Unsupported low-confidence concerns are recorded as unknowns or suppressed rather than promoted to findings.

### Coordinator validation and residual work

The coordinator accepts only findings with evidence, impact, durable direction, and validation. It assigns stable IDs, merges only equal defect-and-fix-path findings, groups related remaining findings, preserves disagreement, and routes every P1-P3 item to fix now, plan task, accepted residual with owner/reason, or needs evidence.

## Risks / Trade-offs

- [Journals increase review I/O] → Keep entries concise and write only generated Markdown per completed slice.
- [More slices create ceremony] → Require a coverage map and meaningful inspection slices, not raw thought logging or fixed arbitrary counts.
- [Shared filesystem weakens absolute isolation] → Use fresh agent contexts, unique journal paths, no cross-agent paths in prompts, and explicit no-read rules; disclose that this is operational rather than cryptographic isolation.
- [Four deep reviews take time] → Preserve capacity-aware parallel fan-out and allow an explicit light depth later; deep is the default only when requested.

## Migration Plan

1. Extend the coordinator and common contract with review-contract, journal, priority, and synthesis requirements.
2. Replace each brief persona focus list with a deep, differentiated investigation contract.
3. Validate the skill structure and execute a read-only dry run on a bounded scope.

## Open Questions

- Whether a future explicit `depth:light` mode should use reduced slice checklists.
- Whether report journals should be retained automatically or cleaned after final synthesis.
