## Why

Reviews currently depend on one agent's perspective, which can miss architecture, correctness, performance, or developer-experience concerns. A reusable `/review` command should obtain independent specialist assessments in parallel, then present one evidence-based decision-ready report.

## What Changes

- Add a `/review` Codex skill that accepts a scoped review request and coordinates four independent review personas.
- Define Architecture, Correctness, Performance & Reliability, and Developer Experience reviewer contracts with shared evidence, severity, and reporting rules.
- Run the four reviews concurrently when agent capacity permits, without reviewers reading or influencing one another's work.
- Require the coordinator to deduplicate findings, retain material disagreement, rank issues by impact and confidence, and state a recommended next step.
- Keep review work read-only: it must not modify source, planning artifacts, or external state.

## Capabilities

### New Capabilities

- `parallel-code-review`: Run a structured, independent, parallel multi-persona review and synthesize its evidence into a single actionable report.

### Modified Capabilities

None.

## Impact

- Adds a repository-local Codex skill and supporting reviewer prompt assets under `.codex/skills/`.
- Establishes a consistent review workflow for source, architecture, performance, and developer-experience changes.
- Uses Codex sub-agents only for bounded, read-only review work; no extension or language-server runtime behavior changes.
