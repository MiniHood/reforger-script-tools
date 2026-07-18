import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { finalizeOutcome, renderReport } from "./agent-routing-report.mjs";

const tool = join(process.cwd(), "tools", "agent-routing-report.mjs");

function outcome(overrides = {}) {
  return {
    attempt_id: "attempt-001",
    work_id: "work-001",
    attempt_sequence: 1,
    workflow: "ce-work",
    role: "worker",
    route_source: "named",
    task_class: "normal",
    risk_class: "normal",
    requested_route: { model: "gpt-5.6-terra", effort: "high" },
    actual_route: { model: "gpt-5.6-terra", effort: "high" },
    selection_reasons: ["normal-scope"],
    escalation_reasons: [],
    terminal_state: "success",
    verification: { status: "passed", checks_requested: 2, checks_passed: 2, checks_failed: 0 },
    corrections: { count: 0 },
    failure_tags: [],
    ...overrides,
  };
}

function run(args) {
  return spawnSync(process.execPath, [tool, ...args], { encoding: "utf8" });
}

function writeOutcome(directory, name, value) {
  const path = join(directory, name);
  writeFileSync(path, JSON.stringify(value), "utf8");
  return path;
}

test("record mode finalizes validated outcomes serially and report mode aggregates routes", () => {
  const directory = mkdtempSync(join(tmpdir(), "agent-routing-"));
  const records = join(directory, "outcomes.jsonl");
  const first = writeOutcome(directory, "first.json", outcome({
    usage: { trustworthy: true, receipt: "codex-receipt", total_tokens: 100, cost_usd: 0.12 },
  }));
  const second = writeOutcome(directory, "second.json", outcome({
    attempt_id: "attempt-002",
    work_id: "work-002",
    role: "high-risk-implementer",
    route_source: "inherited",
    task_class: "consequential",
    risk_class: "consequential",
    requested_route: { model: "gpt-5.6-sol", effort: "high" },
    actual_route: { model: "gpt-5.6-sol", effort: "high" },
    terminal_state: "correction",
    corrections: { count: 1 },
    escalation_reasons: ["semantic-reclassification"],
    verification: { status: "failed", checks_requested: 1, checks_passed: 0, checks_failed: 1 },
    failure_tags: ["verification-failure"],
    usage: { trustworthy: true, receipt: "codex-receipt", total_tokens: 200, cost_usd: 0.3 },
  }));

  assert.equal(run(["record", "--input", first, "--records", records]).status, 0);
  assert.equal(run(["record", "--input", second, "--records", records]).status, 0);
  const recorded = readFileSync(records, "utf8").trim().split(/\r?\n/).map(JSON.parse);
  assert.equal(recorded.length, 2);
  assert.equal(recorded[0].schema_version, 2);
  assert.ok(recorded[0].recorded_at);

  const report = join(directory, "report.md");
  assert.equal(run(["report", "--records", records, "--out", report]).status, 0);
  const text = readFileSync(report, "utf8");
  assert.match(text, /## Route Summary/);
  assert.match(text, /gpt-5\.6-terra\/high -> gpt-5\.6-terra\/high/);
  assert.match(text, /verification-failure/);
  assert.match(text, /trustworthy cost receipts are present for every record/);
  assert.match(text, /\$0\.1200/);
});

test("record mode rejects malformed JSONL before it appends", () => {
  const directory = mkdtempSync(join(tmpdir(), "agent-routing-"));
  const records = join(directory, "outcomes.jsonl");
  writeFileSync(records, "not-json\n", "utf8");
  const input = writeOutcome(directory, "outcome.json", outcome());
  const result = run(["record", "--input", input, "--records", records]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /not valid JSONL/);
  assert.equal(readFileSync(records, "utf8"), "not-json\n");
});

test("record mode rejects content-bearing fields and absolute paths", () => {
  const directory = mkdtempSync(join(tmpdir(), "agent-routing-"));
  const prompt = writeOutcome(directory, "prompt.json", outcome({ prompt: "do work" }));
  const path = writeOutcome(directory, "path.json", outcome({ attempt_id: "C:\\secret\\attempt" }));
  assert.match(run(["record", "--input", prompt, "--records", join(directory, "one.jsonl")]).stderr, /forbidden/);
  assert.match(run(["record", "--input", path, "--records", join(directory, "two.jsonl")]).stderr, /absolute path/);
});

test("record mode accepts every terminal state as a finalized outcome", () => {
  const directory = mkdtempSync(join(tmpdir(), "agent-routing-"));
  const records = join(directory, "outcomes.jsonl");
  const states = ["success", "correction", "failure", "timeout", "cancellation", "unavailable", "missing-return"];
  for (const [index, terminalState] of states.entries()) {
    const input = writeOutcome(directory, `${terminalState}.json`, outcome({
      attempt_id: `attempt-state-${index}`,
      work_id: `work-state-${index}`,
      terminal_state: terminalState,
      corrections: { count: terminalState === "correction" ? 1 : 0 },
      actual_route: ["unavailable", "missing-return"].includes(terminalState) ? null : { model: "gpt-5.6-terra", effort: "high" },
      verification: {
        status: terminalState === "unavailable" ? "unavailable" : "not-run",
        checks_requested: 0,
        checks_passed: 0,
        checks_failed: 0,
      },
    }));
    assert.equal(run(["record", "--input", input, "--records", records]).status, 0);
  }
  assert.equal(readFileSync(records, "utf8").trim().split(/\r?\n/).length, states.length);
});

test("unavailable, missing-return, and optional trustworthy usage are reported without estimates", () => {
  const directory = mkdtempSync(join(tmpdir(), "agent-routing-"));
  const records = join(directory, "outcomes.jsonl");
  const unavailable = writeOutcome(directory, "unavailable.json", outcome({
    attempt_id: "attempt-003",
    work_id: "work-003",
    route_source: "ce-managed",
    actual_route: null,
    terminal_state: "unavailable",
    verification: { status: "unavailable", checks_requested: 0, checks_passed: 0, checks_failed: 0 },
    failure_tags: ["model-unavailable"],
  }));
  const missing = writeOutcome(directory, "missing.json", outcome({
    attempt_id: "attempt-004",
    work_id: "work-004",
    route_source: "unverified",
    actual_route: null,
    terminal_state: "missing-return",
    verification: { status: "not-run", checks_requested: 0, checks_passed: 0, checks_failed: 0 },
    usage: { trustworthy: true, receipt: "codex-receipt", total_tokens: 42 },
  }));
  assert.equal(run(["record", "--input", unavailable, "--records", records]).status, 0);
  assert.equal(run(["record", "--input", missing, "--records", records]).status, 0);
  const report = join(directory, "report.md");
  assert.equal(run(["report", "--records", records, "--out", report]).status, 0);
  const text = readFileSync(report, "utf8");
  assert.match(text, /ce-managed/);
  assert.match(text, /unverified/);
  assert.match(text, /Trustworthy usage receipts: 1\/2/);
  assert.match(text, /Reported-cost comparison: quality-only/);
  assert.match(text, /\| ce-work \| 2 .* \| 1 \(50%\) \|/);
  assert.doesNotMatch(text, new RegExp(directory.replaceAll("\\", "\\\\")));
});

test("report flags insufficient samples, repeated comparable failures, audits, and critical recovery records", () => {
  const directory = mkdtempSync(join(tmpdir(), "agent-routing-"));
  const records = join(directory, "outcomes.jsonl");
  for (let index = 0; index < 3; index += 1) {
    const input = writeOutcome(directory, `failure-${index}.json`, outcome({
      attempt_id: `attempt-failure-${index}`,
      work_id: `work-failure-${index}`,
      terminal_state: "failure",
      verification: { status: "failed", checks_requested: 1, checks_passed: 0, checks_failed: 1 },
      failure_tags: ["same-failure"],
    }));
    assert.equal(run(["record", "--input", input, "--records", records]).status, 0);
  }
  const critical = writeOutcome(directory, "critical.json", outcome({
    attempt_id: "attempt-critical",
    work_id: "work-critical",
    role: "recovery-implementer",
    task_class: "critical",
    risk_class: "critical",
    requested_route: { model: "gpt-5.6-sol", effort: "xhigh" },
    actual_route: { model: "gpt-5.6-sol", effort: "xhigh" },
    escalation_reasons: ["recovery"],
    classification_audit: {
      audit_id: "audit-001",
      blinded_input: {
        file_count: 2,
        public_contract: false,
        uncertain_api: true,
        semantic_core: true,
        process_lifecycle: false,
        security_or_data_loss: false,
        verification_strength: "weak",
      },
      independent_task_class: "critical",
      independent_risk_class: "critical",
    },
  }));
  assert.equal(run(["record", "--input", critical, "--records", records]).status, 0);
  const report = join(directory, "report.md");
  assert.equal(run(["report", "--records", records, "--out", report]).status, 0);
  const text = readFileSync(report, "utf8");
  assert.match(text, /First review: not ready/);
  assert.match(text, /same-failure occurred 3 times/);
  assert.match(text, /## Blinded Classification Audit/);
  assert.match(text, /audit-001/);
  assert.match(text, /## Critical And Recovery Records/);
  assert.match(text, /attempt-critical/);
});

test("record mode is idempotent for identical attempt IDs and rejects conflicting replays", () => {
  const directory = mkdtempSync(join(tmpdir(), "agent-routing-"));
  const records = join(directory, "outcomes.jsonl");
  const input = writeOutcome(directory, "outcome.json", outcome());
  assert.equal(run(["record", "--input", input, "--records", records]).status, 0);
  const replay = run(["record", "--input", input, "--records", records]);
  assert.equal(replay.status, 0);
  assert.match(replay.stdout, /Already recorded/);
  assert.equal(readFileSync(records, "utf8").trim().split(/\r?\n/).length, 1);

  const conflict = writeOutcome(directory, "conflict.json", outcome({ role: "quick-implementer" }));
  const result = run(["record", "--input", conflict, "--records", records]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /already exists with different data/);
});

test("record mode rejects contradictory terminal and verification states", () => {
  const directory = mkdtempSync(join(tmpdir(), "agent-routing-"));
  const cases = [
    outcome({ verification: { status: "passed", checks_requested: 2, checks_passed: 1, checks_failed: 0 } }),
    outcome({ verification: { status: "failed", checks_requested: 1, checks_passed: 0, checks_failed: 0 } }),
    outcome({ terminal_state: "unavailable", actual_route: null, verification: { status: "not-run", checks_requested: 0, checks_passed: 0, checks_failed: 0 } }),
    outcome({ terminal_state: "correction", corrections: { count: 0 } }),
  ];
  for (const [index, value] of cases.entries()) {
    value.attempt_id = `attempt-invalid-${index}`;
    value.work_id = `work-invalid-${index}`;
    const input = writeOutcome(directory, `invalid-${index}.json`, value);
    assert.notEqual(run(["record", "--input", input, "--records", join(directory, "outcomes.jsonl")]).status, 0);
  }
});

test("report links failed and recovery attempts into completed work cost", () => {
  const now = new Date("2026-07-18T12:00:00.000Z");
  const failed = finalizeOutcome(outcome({
    attempt_id: "attempt-linked-1",
    work_id: "work-linked",
    terminal_state: "failure",
    verification: { status: "failed", checks_requested: 1, checks_passed: 0, checks_failed: 1 },
    failure_tags: ["implementation-failure"],
    usage: { trustworthy: true, receipt: "receipt-1", cost_usd: 1.25 },
  }), now);
  const recovered = finalizeOutcome(outcome({
    attempt_id: "attempt-linked-2",
    work_id: "work-linked",
    attempt_sequence: 2,
    role: "recovery-implementer",
    requested_route: { model: "gpt-5.6-sol", effort: "xhigh" },
    actual_route: { model: "gpt-5.6-sol", effort: "xhigh" },
    terminal_state: "correction",
    corrections: { count: 1 },
    escalation_reasons: ["recovery"],
    usage: { trustworthy: true, receipt: "receipt-2", cost_usd: 2.75 },
  }), now);
  const report = renderReport([failed, recovered], now);
  assert.match(report, /## Completed Work Cost/);
  assert.match(report, /\| work-linked \| 2 \| correction \| yes \| 1 \| 1 \| \$4\.0000 \|/);
  assert.match(report, /Reported attempt cost/);
});

test("review cadence uses the last completed-delegation checkpoint", () => {
  const now = new Date("2026-07-18T12:00:00.000Z");
  const records = Array.from({ length: 80 }, (_, index) => finalizeOutcome(outcome({
    attempt_id: `attempt-cadence-${index}`,
    work_id: `work-cadence-${index}`,
  }), new Date(now.getTime() + index)));
  assert.match(renderReport(records.slice(0, 79), now, "outcomes.jsonl", { lastReviewedCompleted: 30 }), /49\/50 completed since the last review/);
  assert.match(renderReport(records, now, "outcomes.jsonl", { lastReviewedCompleted: 30 }), /Fifty-task cadence: review due/);
  assert.match(renderReport(records.slice(0, 30), now), /Fifty-task cadence: unknown/);
  assert.throws(() => renderReport(records.slice(0, 30), now, "outcomes.jsonl", { lastReviewedCompleted: 31 }), /cannot exceed/);
});

test("CLI path overrides cannot write tracked repository locations", () => {
  const directory = mkdtempSync(join(tmpdir(), "agent-routing-"));
  const input = writeOutcome(directory, "outcome.json", outcome());
  const result = run(["record", "--input", input, "--records", "routing-outcomes.jsonl"]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /must stay under tools\/reports\/agent-routing/);
});
