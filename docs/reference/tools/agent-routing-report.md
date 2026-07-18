# tools/agent-routing-report.mjs

## Purpose

Provides the dev-only, content-free observation loop for project-local agent routing. The tool records finalized delegated-task outcomes as JSONL and renders aggregate Markdown under the ignored `tools/reports/agent-routing/` directory.

## Architecture Role

This is not extension runtime code and does not select agents, dispatch work, or edit Codex configuration. The parent creates an in-memory content-free attempt observation at dispatch, merges the subagent's terminal `routing_result`, and invokes `record` for every terminal state, including timeout, cancellation, unavailable route, and missing return. Subagents must not write the shared JSONL file themselves.

The schema intentionally accepts only compact identifiers, enums, booleans, and numeric counts. It rejects unknown keys plus prompts, source snippets, tool output, URIs, and absolute paths. This keeps raw observations useful for route-quality review without collecting task content.

## Current Behavior

The CLI uses only Node built-ins:

```powershell
node tools/agent-routing-report.mjs record --input <outcome.json>
node tools/agent-routing-report.mjs record --input <outcome.json> --records tools/reports/agent-routing/outcomes.jsonl
node tools/agent-routing-report.mjs report
node tools/agent-routing-report.mjs report --records tools/reports/agent-routing/outcomes.jsonl --out tools/reports/agent-routing/report.md
node tools/agent-routing-report.mjs report --last-reviewed-completed 30
```

`record` adds schema version 2 and `recorded_at`, validates the supplied outcome and every existing JSONL row, then synchronously appends one terminal attempt. An identical replay of an existing `attempt_id` is a no-op; conflicting replays and duplicate work/sequence pairs fail. Accepted terminal states are `success`, `correction`, `failure`, `timeout`, `cancellation`, `unavailable`, and `missing-return`. Records require a parent-owned work ID and positive attempt sequence so recovery cost can be joined to the work that caused it. Actual routes may be `null` only when the surface did not provide one.

The parent submits this exact full-outcome shape. Values shown are examples, not prescribed identifiers:

```json
{
  "attempt_id": "attempt-001",
  "work_id": "work-001",
  "attempt_sequence": 1,
  "workflow": "ce-work",
  "role": "worker",
  "route_source": "named",
  "task_class": "normal",
  "risk_class": "normal",
  "requested_route": { "model": "gpt-5.6-terra", "effort": "high" },
  "actual_route": { "model": "gpt-5.6-terra", "effort": "high" },
  "selection_reasons": ["normal-scope"],
  "escalation_reasons": [],
  "terminal_state": "success",
  "verification": { "status": "passed", "checks_requested": 2, "checks_passed": 2, "checks_failed": 0 },
  "corrections": { "count": 0 },
  "failure_tags": [],
  "usage": { "trustworthy": true, "receipt": "codex-receipt", "total_tokens": 1200, "cost_usd": 0.42 }
}
```

`usage` is optional. `classification_audit` is also optional and follows the blinded structure described below. Verification status must agree with its counts. `success` requires zero corrections; `correction` requires at least one. `unavailable` requires a null actual route, unavailable verification with zero counts, and zero corrections. `missing-return` requires not-run or unavailable verification with zero counts and zero corrections.

The subagent returns only this terminal fragment, which the parent merges with its dispatch-owned fields:

```json
{
  "routing_result": {
    "terminal_state": "success",
    "actual_route": { "model": "gpt-5.6-terra", "effort": "high" },
    "verification": { "status": "passed", "checks_requested": 2, "checks_passed": 2, "checks_failed": 0 },
    "corrections": { "count": 0 },
    "failure_tags": []
  }
}
```

When dispatch is unavailable, or a return is missing, the parent synthesizes the terminal fragment because no trustworthy subagent return exists:

```json
{
  "routing_result": {
    "terminal_state": "unavailable",
    "actual_route": null,
    "verification": { "status": "unavailable", "checks_requested": 0, "checks_passed": 0, "checks_failed": 0 },
    "corrections": { "count": 0 },
    "failure_tags": ["model-unavailable"]
  }
}
```

Route source distinguishes `named`, `inherited`, `ce-managed`, and `unverified`; the report never guesses an internal CE route. Usage is optional. When present it must be explicitly marked trustworthy, identify the compact receipt label, and contain only numeric receipt values. Missing usage is never estimated, and report comparisons are labeled quality-only wherever receipts are incomplete. CLI paths outside the repository are allowed for isolated tests and investigations. Paths inside the repository must remain under ignored `tools/reports/agent-routing/`, and absolute paths are never copied into generated report content.

`report` groups first-pass success, correction, escalation, verification failure, availability, failure-tag rates, trustworthy usage receipts, and reported attempt cost by the full route plus workflow, role, route source, requested/actual model and effort, task class, and risk class. It separately joins attempts by `work_id` and reports completed-work cost only when the final attempt completed and every linked attempt has a trustworthy cost receipt; otherwise it reports quality-only. This prevents a cheap failed attempt from hiding the cost of its recovery.

Review thresholds include the initial review after 30 completed delegations with five samples for each high-volume role, where high-volume means at least 10% of completed delegations at review time. After the first review, pass its completed-delegation count through `--last-reviewed-completed`; the report marks the next review due after 50 additional completed delegations. Without a checkpoint it reports subsequent cadence as unknown. Three matching failure tags in the latest ten comparable records trigger an earlier review.

Classification audit data is deliberately blinded: the input records observable risk signals and verification strength, not task text or selected route. The report shows each audit input, independent classification, recorded classification, and agreement. It also includes every critical or recovery record for explicit human review.

## Dependencies and Boundaries

Uses `node:fs`, `node:path`, `node:url`, and the Node test runner only. No package script, extension command, runtime dependency, telemetry, or automatic policy/configuration edit is registered. `tools/reports/` is already ignored, so raw observations and generated reports remain untracked.

## Change Notes

Added for U4 of the cost-aware agent-routing plan. Schema v2 links attempts to completed work and adds idempotent recording, state invariants, repository path containment, and explicit review checkpoints. The focused `node:test` file exercises valid aggregation, serial append behavior, replay handling, corrupt JSONL, privacy rejection, terminal invariants, unavailable and unverified routes, optional usage, linked recovery cost, review cadence, repeated failures, classification audits, and critical/recovery visibility.

## Future Improvements

Keep future fields compact and structured. Any schema expansion must preserve the no-content boundary, continue to validate existing JSONL before append, and avoid turning observations into automatic route changes. Controlled route comparisons belong to `ce-optimize`; accepted lessons belong to `ce-compound`.
