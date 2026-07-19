## Context

The repository already has local Codex skills for targeted workflows such as debugging and durable fixes, but no repeatable code-review command. Ad-hoc review is necessarily serial and driven by one perspective. The new command must obtain four bounded, read-only assessments without allowing one persona's conclusions to bias another, while still producing one concise report for the user.

The implementation will be Codex tooling under `.codex/skills/`; it is not part of the extension or Rust language-server runtime. The orchestration environment supports a limited number of concurrent agents, so parallelism must be best-effort rather than assumed.

## Goals / Non-Goals

**Goals:**

- Provide a `/review` skill that accepts a user-provided scope, or determines a safe review scope from repository context.
- Run Architecture, Correctness, Performance & Reliability, and Developer Experience reviews independently and concurrently when capacity permits.
- Give every reviewer the same scoped evidence package and a persona-specific lens, while preventing cross-review communication or report access.
- Produce auditable findings: severity, confidence, concrete evidence, impact, durable direction, and validation needed.
- Synthesize reports without erasing disagreement, duplicating findings, or creating speculative work.

**Non-Goals:**

- Editing source, OpenSpec artifacts, configuration, or external systems during review.
- Replacing focused `/debug`, `/fix`, or test workflows.
- Adding a security persona to the default review set; security remains a future opt-in review mode.
- Treating a reviewer recommendation as authorization to implement it.

## Decisions

### One coordinator skill with four fixed persona contracts

`/review` will be a repository-local coordinator skill and will use four separately stored prompt assets: Architecture, Correctness, Performance & Reliability, and Developer Experience. The coordinator owns scope discovery and synthesis; personas own only their assigned review.

Fixed contracts prevent drift and make reports comparable across sessions. A free-form multi-agent review was considered, but it would provide inconsistent coverage and make it difficult to distinguish evidence from opinion.

### Read-only evidence package before parallel fan-out

The coordinator will first collect the review target, repository status, relevant source and reference documentation, tests, and bounded diagnostics. It will then provide the same immutable evidence scope to each reviewer.

This prevents four agents from performing redundant broad scans and avoids timing-dependent review scopes. Reviewers may perform bounded read-only inspection inside that scope, but MUST not modify files, run destructive actions, contact external systems, or delegate further work.

### Independent parallel reviewers with best-effort scheduling

The coordinator will launch all four reviewers concurrently when agent capacity permits. Each reviewer receives no other reviewer identity, status, findings, or communication channel. If runtime capacity cannot run all four at once, the coordinator will schedule the remaining reviewers without changing their evidence package and will disclose that execution was capacity-limited.

Parallel fan-out reduces review latency; isolation reduces anchoring bias. The alternative of sequential reviewer handoff was rejected because later reviewers would be influenced by earlier findings.

### Structured evidence-first reports and coordinator-only synthesis

Each persona report will separate facts, inferences, and unknowns. Every finding MUST name its severity, confidence, evidence location, user or system impact, durable direction, and validation requirement. Reviewers MUST explicitly report when they find no meaningful issue.

Only the coordinator reads all reports. It groups duplicates, retains material disagreement, ranks items using severity, confidence, and user impact, and proposes one best next step. It must not silently turn an uncertain concern into a defect.

### Scope and outcome safeguards

The command will require either an explicit scope or a bounded inference from current work (for example, changed files plus their owning documentation). For broad requests, it will state the selected scope and omissions. The final report will contain a coverage section, findings, strengths, unresolved evidence, and recommended follow-up. It MUST state that the review is advisory and made no code changes.

## Risks / Trade-offs

- [Four agents may exceed runtime capacity] → Launch concurrently when possible, queue only the remainder, and report the scheduling limitation rather than skipping a persona.
- [Reviewers may duplicate exploratory work] → Provide a common evidence package and bounded scope before fan-out.
- [Independent reports may conflict] → Preserve disagreements with their evidence and confidence; coordinator does not force consensus.
- [Reports may become verbose or speculative] → Require evidence-backed findings and a small fixed report structure; no generic recommendations without a concrete concern.
- [Reviewer prompts may become a second source of architecture policy] → Require them to defer to `AGENTS.md` and current reference documentation.
- [Review commands could accidentally mutate a workspace] → Declare review read-only, prohibit edits and state-changing commands, and report this guarantee in the final output.

## Migration Plan

1. Add the coordinator skill and four persona prompt assets.
2. Validate skill instructions, prompt isolation, and repository links through a dry review of a bounded fixture or known changed file set.
3. Use `/review <scope>` for advisory reviews; no extension update, migration, or rollback is required.

Rollback consists of removing the repository-local review skill and persona assets. No runtime state or user data is introduced.

## Open Questions

- Whether a future `/review --security` opt-in should add a fifth persona or use a separate specialized command.
- Whether review reports should be persisted under ignored `tools/reports/` when a user explicitly requests an artifact, rather than only returned in chat.
