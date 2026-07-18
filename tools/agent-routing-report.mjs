#!/usr/bin/env node

import {
  appendFileSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";

const SCHEMA_VERSION = 2;
const DEFAULT_RECORDS_PATH = join("tools", "reports", "agent-routing", "outcomes.jsonl");
const DEFAULT_REPORT_PATH = join("tools", "reports", "agent-routing", "report.md");
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const ROUTING_OUTPUT_ROOT = resolve(REPO_ROOT, "tools", "reports", "agent-routing");
const TERMINAL_STATES = new Set([
  "success",
  "correction",
  "failure",
  "timeout",
  "cancellation",
  "unavailable",
  "missing-return",
]);
const ROUTE_SOURCES = new Set(["named", "inherited", "ce-managed", "unverified"]);
const TASK_CLASSES = new Set(["bounded", "normal", "consequential", "critical"]);
const EFFORTS = new Set(["low", "medium", "high", "xhigh", "max", "ultra"]);
const VERIFICATION_STATES = new Set(["passed", "failed", "not-run", "unavailable"]);
const SAFE_IDENTIFIER = /^[a-z0-9][a-z0-9._-]{0,79}$/;
const OUTCOME_FIELDS = [
  "attempt_id",
  "work_id",
  "attempt_sequence",
  "workflow",
  "role",
  "route_source",
  "task_class",
  "risk_class",
  "requested_route",
  "actual_route",
  "selection_reasons",
  "escalation_reasons",
  "terminal_state",
  "verification",
  "corrections",
  "failure_tags",
  "usage",
  "classification_audit",
];
const OPTIONAL_OUTCOME_FIELDS = ["usage", "classification_audit"];
const RECORD_FIELDS = ["schema_version", "recorded_at", ...OUTCOME_FIELDS];
const FORBIDDEN_KEYS = new Set([
  "prompt",
  "prompts",
  "source",
  "sources",
  "snippet",
  "snippets",
  "tool_output",
  "tool_outputs",
  "output",
  "outputs",
  "path",
  "paths",
  "absolute_path",
  "absolute_paths",
  "uri",
  "uris",
  "cwd",
]);

if (isCliInvocation()) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`agent-routing-report: ${error.message}`);
    process.exitCode = 1;
  }
}

function isCliInvocation() {
  return process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
}

export function main(rawArgs) {
  const { command, options } = parseArgs(rawArgs);
  if (command === "record") {
    const input = readJson(options.input, "--input");
    const finalized = finalizeOutcome(input, new Date());
    const recordsPath = options.records ?? DEFAULT_RECORDS_PATH;
    assertSafeRepositoryOutputPath(recordsPath, "--records");
    const appended = appendRecord(recordsPath, finalized);
    console.log(`${appended ? "Recorded" : "Already recorded"} ${finalized.attempt_id}`);
    return finalized;
  }
  if (command === "report") {
    const recordsPath = options.records ?? DEFAULT_RECORDS_PATH;
    assertSafeRepositoryOutputPath(recordsPath, "--records");
    const records = readJsonl(recordsPath);
    const report = renderReport(records, new Date(), recordsPath, {
      lastReviewedCompleted: options.lastReviewedCompleted,
    });
    const output = options.out ?? DEFAULT_REPORT_PATH;
    assertSafeRepositoryOutputPath(output, "--out");
    mkdirSync(dirname(output), { recursive: true });
    writeFileSync(output, report, "utf8");
    console.log(`Wrote ${output}`);
    return report;
  }
  throw new Error(`Unknown command: ${command}`);
}

export function parseArgs(rawArgs) {
  const [command, ...args] = rawArgs;
  if (!command || command === "--help" || command === "-h") {
    printUsage();
    process.exit(0);
  }
  const options = {};
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--input" || arg === "--records" || arg === "--out" || arg === "--last-reviewed-completed") {
      const value = args[++index];
      if (!value || value.startsWith("--")) {
        throw new Error(`${arg} requires a value`);
      }
      const key = arg === "--last-reviewed-completed" ? "lastReviewedCompleted" : arg.slice(2);
      options[key] = arg === "--last-reviewed-completed"
        ? parseNonNegativeInteger(value, arg)
        : value;
    } else if (arg === "--help" || arg === "-h") {
      printUsage();
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  if (command === "record" && !options.input) {
    throw new Error("record requires --input <outcome.json>");
  }
  if (command !== "record" && command !== "report") {
    throw new Error(`Unknown command: ${command}`);
  }
  return { command, options };
}

function printUsage() {
  console.log("Usage:\n  node tools/agent-routing-report.mjs record --input <outcome.json> [--records <outcomes.jsonl>]\n  node tools/agent-routing-report.mjs report [--records <outcomes.jsonl>] [--out <report.md>] [--last-reviewed-completed <count>]");
}

function parseNonNegativeInteger(value, flag) {
  if (!/^\d+$/.test(value)) {
    throw new Error(`${flag} must be a non-negative integer`);
  }
  return Number(value);
}

function readJson(path, flag) {
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    throw new Error(`${flag} must name valid JSON: ${error.message}`);
  }
  return parsed;
}

export function appendRecord(recordsPath, record) {
  validateFinalRecord(record);
  const existing = readJsonl(recordsPath);
  // Validate before append so a corrupt shared observation file is never extended.
  for (const existingRecord of existing) {
    validateFinalRecord(existingRecord);
  }
  validateRecordSet(existing);
  const duplicate = existing.find((existingRecord) => existingRecord.attempt_id === record.attempt_id);
  if (duplicate) {
    if (isDeepStrictEqual(withoutRecordedAt(duplicate), withoutRecordedAt(record))) {
      return false;
    }
    throw new Error(`record.attempt_id ${record.attempt_id} already exists with different data`);
  }
  if (existing.some((existingRecord) => existingRecord.work_id === record.work_id
    && existingRecord.attempt_sequence === record.attempt_sequence)) {
    throw new Error(`record work sequence ${record.work_id}/${record.attempt_sequence} already exists`);
  }
  mkdirSync(dirname(recordsPath), { recursive: true });
  appendFileSync(recordsPath, `${JSON.stringify(record)}\n`, "utf8");
  return true;
}

function withoutRecordedAt(record) {
  const { recorded_at: _recordedAt, ...outcome } = record;
  return outcome;
}

export function readJsonl(path) {
  let content;
  try {
    content = readFileSync(path, "utf8");
  } catch (error) {
    if (error.code === "ENOENT") {
      return [];
    }
    throw error;
  }
  const lines = content.split(/\r?\n/).filter(Boolean);
  return lines.map((line, index) => {
    try {
      return JSON.parse(line);
    } catch (error) {
      throw new Error(`${path}:${index + 1} is not valid JSONL: ${error.message}`);
    }
  });
}

export function finalizeOutcome(outcome, now) {
  assertObject(outcome, "outcome");
  rejectForbiddenKeys(outcome, "outcome");
  assertExactKeys(outcome, OUTCOME_FIELDS, OPTIONAL_OUTCOME_FIELDS, "outcome");

  const record = {
    schema_version: SCHEMA_VERSION,
    recorded_at: now.toISOString(),
    ...outcome,
  };
  validateFinalRecord(record);
  return record;
}

export function validateFinalRecord(record) {
  assertObject(record, "record");
  rejectForbiddenKeys(record, "record");
  assertExactKeys(record, RECORD_FIELDS, OPTIONAL_OUTCOME_FIELDS, "record");
  if (record.schema_version !== SCHEMA_VERSION) {
    throw new Error(`record.schema_version must be ${SCHEMA_VERSION}`);
  }
  if (!Number.isFinite(Date.parse(record.recorded_at))) {
    throw new Error("record.recorded_at must be an ISO timestamp");
  }
  assertIdentifier(record.attempt_id, "record.attempt_id");
  assertIdentifier(record.work_id, "record.work_id");
  assertPositiveInteger(record.attempt_sequence, "record.attempt_sequence");
  assertIdentifier(record.workflow, "record.workflow");
  assertIdentifier(record.role, "record.role");
  assertEnum(record.route_source, ROUTE_SOURCES, "record.route_source");
  assertEnum(record.task_class, TASK_CLASSES, "record.task_class");
  assertEnum(record.risk_class, TASK_CLASSES, "record.risk_class");
  validateRoute(record.requested_route, "record.requested_route", true);
  validateRoute(record.actual_route, "record.actual_route", false);
  validateIdentifierArray(record.selection_reasons, "record.selection_reasons", true);
  validateIdentifierArray(record.escalation_reasons, "record.escalation_reasons", false);
  assertEnum(record.terminal_state, TERMINAL_STATES, "record.terminal_state");
  validateVerification(record.verification);
  validateCorrections(record.corrections);
  validateIdentifierArray(record.failure_tags, "record.failure_tags", false);
  if (record.usage !== undefined) {
    validateUsage(record.usage);
  }
  if (record.classification_audit !== undefined) {
    validateClassificationAudit(record.classification_audit);
  }
  validateRecordInvariants(record);
}

function validateRecordInvariants(record) {
  const verification = record.verification;
  const noChecks = verification.checks_requested === 0
    && verification.checks_passed === 0
    && verification.checks_failed === 0;
  if (verification.status === "passed"
    && (verification.checks_requested === 0 || verification.checks_failed !== 0
      || verification.checks_passed !== verification.checks_requested)) {
    throw new Error("record.verification passed requires every requested check to pass");
  }
  if (verification.status === "failed" && verification.checks_failed === 0) {
    throw new Error("record.verification failed requires at least one failed check");
  }
  if (["not-run", "unavailable"].includes(verification.status) && !noChecks) {
    throw new Error(`record.verification ${verification.status} requires zero check counts`);
  }
  if (record.terminal_state === "unavailable"
    && (record.actual_route !== null || verification.status !== "unavailable" || record.corrections.count !== 0)) {
    throw new Error("record unavailable requires no actual route, unavailable verification, and zero corrections");
  }
  if (record.terminal_state === "missing-return"
    && (!["not-run", "unavailable"].includes(verification.status) || !noChecks || record.corrections.count !== 0)) {
    throw new Error("record missing-return requires no executed checks and zero corrections");
  }
  if (record.terminal_state === "success" && record.corrections.count !== 0) {
    throw new Error("record success requires zero corrections");
  }
  if (record.terminal_state === "correction" && record.corrections.count === 0) {
    throw new Error("record correction requires at least one correction");
  }
}

function validateRoute(route, name, required) {
  if (route === null && !required) {
    return;
  }
  assertObject(route, name);
  assertExactKeys(route, ["model", "effort"], [], name);
  assertIdentifier(route.model, `${name}.model`);
  assertEnum(route.effort, EFFORTS, `${name}.effort`);
}

function validateVerification(verification) {
  assertObject(verification, "record.verification");
  assertExactKeys(verification, ["status", "checks_requested", "checks_passed", "checks_failed"], [], "record.verification");
  assertEnum(verification.status, VERIFICATION_STATES, "record.verification.status");
  for (const key of ["checks_requested", "checks_passed", "checks_failed"]) {
    assertNonNegativeInteger(verification[key], `record.verification.${key}`);
  }
  if (verification.checks_passed + verification.checks_failed > verification.checks_requested) {
    throw new Error("record.verification passed and failed checks cannot exceed checks requested");
  }
}

function validateCorrections(corrections) {
  assertObject(corrections, "record.corrections");
  assertExactKeys(corrections, ["count"], [], "record.corrections");
  assertNonNegativeInteger(corrections.count, "record.corrections.count");
}

function validateUsage(usage) {
  assertObject(usage, "record.usage");
  assertExactKeys(usage, ["trustworthy", "receipt", "input_tokens", "output_tokens", "total_tokens", "latency_ms", "cost_usd"], ["input_tokens", "output_tokens", "total_tokens", "latency_ms", "cost_usd"], "record.usage");
  if (usage.trustworthy !== true) {
    throw new Error("record.usage.trustworthy must be true when usage is recorded");
  }
  assertIdentifier(usage.receipt, "record.usage.receipt");
  const metricKeys = ["input_tokens", "output_tokens", "total_tokens", "latency_ms", "cost_usd"];
  if (!metricKeys.some((key) => usage[key] !== undefined)) {
    throw new Error("record.usage must include at least one trustworthy metric");
  }
  for (const key of metricKeys) {
    if (usage[key] !== undefined && (!Number.isFinite(usage[key]) || usage[key] < 0)) {
      throw new Error(`record.usage.${key} must be a non-negative number`);
    }
  }
}

function validateClassificationAudit(audit) {
  assertObject(audit, "record.classification_audit");
  assertExactKeys(audit, ["audit_id", "blinded_input", "independent_task_class", "independent_risk_class"], [], "record.classification_audit");
  assertIdentifier(audit.audit_id, "record.classification_audit.audit_id");
  assertObject(audit.blinded_input, "record.classification_audit.blinded_input");
  assertExactKeys(audit.blinded_input, [
    "file_count",
    "public_contract",
    "uncertain_api",
    "semantic_core",
    "process_lifecycle",
    "security_or_data_loss",
    "verification_strength",
  ], [], "record.classification_audit.blinded_input");
  assertNonNegativeInteger(audit.blinded_input.file_count, "record.classification_audit.blinded_input.file_count");
  for (const key of ["public_contract", "uncertain_api", "semantic_core", "process_lifecycle", "security_or_data_loss"]) {
    if (typeof audit.blinded_input[key] !== "boolean") {
      throw new Error(`record.classification_audit.blinded_input.${key} must be boolean`);
    }
  }
  assertEnum(audit.blinded_input.verification_strength, new Set(["focused", "adequate", "weak", "none"]), "record.classification_audit.blinded_input.verification_strength");
  assertEnum(audit.independent_task_class, TASK_CLASSES, "record.classification_audit.independent_task_class");
  assertEnum(audit.independent_risk_class, TASK_CLASSES, "record.classification_audit.independent_risk_class");
}

function assertObject(value, name) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${name} must be an object`);
  }
}

function assertExactKeys(value, allowed, optional, name) {
  const allowedKeys = new Set(allowed);
  for (const key of Object.keys(value)) {
    if (!allowedKeys.has(key)) {
      throw new Error(`${name}.${key} is not allowed; content-bearing fields are forbidden`);
    }
  }
  for (const key of allowed) {
    if (!optional.includes(key) && value[key] === undefined) {
      throw new Error(`${name}.${key} is required`);
    }
  }
}

function rejectForbiddenKeys(value, name) {
  for (const [key, child] of Object.entries(value)) {
    if (FORBIDDEN_KEYS.has(key)) {
      throw new Error(`${name}.${key} is forbidden; do not record prompts, source, tool output, or paths`);
    }
    if (child && typeof child === "object") {
      rejectForbiddenKeys(child, `${name}.${key}`);
    }
  }
}

function assertIdentifier(value, name) {
  if (typeof value !== "string" || !SAFE_IDENTIFIER.test(value)) {
    throw new Error(`${name} must be a compact identifier, not content or an absolute path`);
  }
}

function isAbsolutePath(value) {
  return /^(?:[a-z]:[\\/]|\\\\|\/|file:)/i.test(value);
}

function reportPathLabel(path) {
  return isAbsolutePath(path) ? "external-records" : path;
}

function assertEnum(value, allowed, name) {
  if (!allowed.has(value)) {
    throw new Error(`${name} must be one of: ${Array.from(allowed).join(", ")}`);
  }
}

function assertNonNegativeInteger(value, name) {
  if (!Number.isInteger(value) || value < 0) {
    throw new Error(`${name} must be a non-negative integer`);
  }
}

function assertPositiveInteger(value, name) {
  if (!Number.isInteger(value) || value < 1) {
    throw new Error(`${name} must be a positive integer`);
  }
}

function isWithin(parent, target) {
  const pathFromParent = relative(parent, target);
  return pathFromParent === "" || (!pathFromParent.startsWith("..") && !isAbsolute(pathFromParent));
}

function assertSafeRepositoryOutputPath(path, name) {
  const target = resolve(path);
  if (isWithin(REPO_ROOT, target) && !isWithin(ROUTING_OUTPUT_ROOT, target)) {
    throw new Error(`${name} inside the repository must stay under tools/reports/agent-routing`);
  }
}

function validateIdentifierArray(value, name, required) {
  if (!Array.isArray(value)) {
    throw new Error(`${name} must be an array`);
  }
  if (required && value.length === 0) {
    throw new Error(`${name} must contain at least one reason`);
  }
  for (const entry of value) {
    assertIdentifier(entry, `${name} entry`);
  }
}

export function renderReport(records, generatedAt = new Date(), recordsPath = DEFAULT_RECORDS_PATH, options = {}) {
  for (const record of records) {
    validateFinalRecord(record);
  }
  validateRecordSet(records);
  const summary = summarize(records);
  validateReviewCheckpoint(options.lastReviewedCompleted, summary.completedCount);
  const lines = [
    "# Agent Routing Outcome Report",
    "",
    "Content-free repository routing outcomes. This report observes routes; it does not edit Codex configuration or routing policy.",
    "",
    "## Summary",
    "",
    `- Generated: ${generatedAt.toISOString()}`,
    `- Records: ${records.length}`,
    `- Records file: \`${reportPathLabel(recordsPath)}\``,
    `- Completed delegations: ${summary.completedCount}`,
    `- Trustworthy usage receipts: ${summary.usageCount}/${records.length}`,
    `- Usage comparison: ${summary.usageCount === records.length ? "receipts are present for every record" : "quality-only where usage is missing; no usage is estimated"}.`,
    `- Reported-cost comparison: ${summary.costCount === records.length ? "trustworthy cost receipts are present for every record" : "quality-only where reported cost is missing"}.`,
    "",
    "## Completed Work Cost",
    "",
  ];
  renderCompletedWork(lines, records);
  lines.push("", "## Route Summary", "");
  renderMetricsTable(lines, summarizeBy(records, (record) => routeLabel(record)));
  for (const [heading, label] of [
    ["Workflow", (record) => record.workflow],
    ["Role", (record) => record.role],
    ["Route Source", (record) => record.route_source],
    ["Requested And Actual Model/Effort", (record) => modelLabel(record)],
    ["Task Class", (record) => record.task_class],
    ["Risk Class", (record) => record.risk_class],
  ]) {
    lines.push("", `## By ${heading}`, "");
    renderMetricsTable(lines, summarizeBy(records, label));
  }
  lines.push("", "## Failure Tags", "");
  renderFailureTags(lines, records);
  lines.push("", "## Review Thresholds", "");
  renderThresholds(lines, records, summary.completedCount, options.lastReviewedCompleted);
  lines.push("", "## Blinded Classification Audit", "");
  renderClassificationAudits(lines, records);
  lines.push("", "## Critical And Recovery Records", "");
  renderCriticalAndRecovery(lines, records);
  return `${lines.join("\n")}\n`;
}

function validateRecordSet(records) {
  const attempts = new Set();
  const workSequences = new Set();
  for (const record of records) {
    if (attempts.has(record.attempt_id)) {
      throw new Error(`duplicate record.attempt_id: ${record.attempt_id}`);
    }
    attempts.add(record.attempt_id);
    const workSequence = `${record.work_id}/${record.attempt_sequence}`;
    if (workSequences.has(workSequence)) {
      throw new Error(`duplicate record work sequence: ${workSequence}`);
    }
    workSequences.add(workSequence);
  }
}

function renderCompletedWork(lines, records) {
  const groups = new Map();
  for (const record of records) {
    if (!groups.has(record.work_id)) {
      groups.set(record.work_id, []);
    }
    groups.get(record.work_id).push(record);
  }
  const rows = Array.from(groups.entries()).map(([workId, attempts]) => {
    attempts.sort((left, right) => left.attempt_sequence - right.attempt_sequence);
    const finalAttempt = attempts.at(-1);
    const completed = ["success", "correction"].includes(finalAttempt.terminal_state);
    const hasCompleteCost = completed && attempts.every((record) => record.usage?.cost_usd !== undefined);
    return [
      workId,
      attempts.length,
      finalAttempt.terminal_state,
      completed ? "yes" : "no",
      attempts.reduce((total, record) => total + record.corrections.count, 0),
      attempts.filter((record) => record.escalation_reasons.length > 0).length,
      hasCompleteCost ? `$${attempts.reduce((total, record) => total + record.usage.cost_usd, 0).toFixed(4)}` : "quality-only",
    ];
  }).sort((left, right) => String(left[0]).localeCompare(String(right[0])));
  table(lines, ["Work", "Attempts", "Final terminal", "Completed", "Corrections", "Escalations", "Completed work cost"], rows);
}

function summarize(records) {
  return {
    completedCount: records.filter(isCompletedDelegation).length,
    usageCount: records.filter((record) => record.usage !== undefined).length,
    costCount: records.filter((record) => record.usage?.cost_usd !== undefined).length,
  };
}

function summarizeBy(records, label) {
  const groups = new Map();
  for (const record of records) {
    const key = label(record);
    if (!groups.has(key)) {
      groups.set(key, []);
    }
    groups.get(key).push(record);
  }
  return Array.from(groups.entries())
    .map(([key, group]) => ({ key, ...metrics(group) }))
    .sort((left, right) => right.records - left.records || left.key.localeCompare(right.key));
}

function metrics(records) {
  const count = (predicate) => records.filter(predicate).length;
  const failureCount = count((record) => record.failure_tags.length > 0);
  return {
    records: records.length,
    firstPass: count((record) => record.terminal_state === "success" && record.corrections.count === 0 && record.escalation_reasons.length === 0),
    corrections: count((record) => record.terminal_state === "correction" || record.corrections.count > 0),
    escalations: count((record) => record.escalation_reasons.length > 0),
    verificationFailures: count((record) => record.verification.status === "failed"),
    available: count((record) => record.terminal_state !== "unavailable"),
    failureCount,
    usageCount: count((record) => record.usage !== undefined),
    costCount: count((record) => record.usage?.cost_usd !== undefined),
    costUsd: records.reduce((total, record) => total + (record.usage?.cost_usd ?? 0), 0),
  };
}

function renderMetricsTable(lines, rows) {
  table(lines, ["Route", "Records", "First pass", "Correction", "Escalation", "Verification failed", "Available", "Failure tags", "Usage receipts", "Reported attempt cost"], rows.map((row) => [
    row.key,
    row.records,
    countRate(row.firstPass, row.records),
    countRate(row.corrections, row.records),
    countRate(row.escalations, row.records),
    countRate(row.verificationFailures, row.records),
    countRate(row.available, row.records),
    countRate(row.failureCount, row.records),
    countRate(row.usageCount, row.records),
    row.costCount === row.records ? `$${row.costUsd.toFixed(4)}` : "quality-only",
  ]));
}

function renderFailureTags(lines, records) {
  const tags = new Map();
  for (const record of records) {
    for (const tag of record.failure_tags) {
      tags.set(tag, (tags.get(tag) ?? 0) + 1);
    }
  }
  table(lines, ["Failure tag", "Records", "Rate"], Array.from(tags.entries())
    .map(([tag, count]) => [tag, count, `${percent(rate(count, records.length))}`])
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0])));
}

function renderThresholds(lines, records, completedCount, lastReviewedCompleted) {
  const highVolumeRoles = summarizeBy(records.filter(isCompletedDelegation), (record) => record.role)
    .filter((row) => row.records / Math.max(completedCount, 1) >= 0.1);
  const rolesReady = highVolumeRoles.length > 0 && highVolumeRoles.every((row) => row.records >= 5);
  const firstReviewReady = completedCount >= 30 && rolesReady;
  lines.push(`- First review: ${firstReviewReady ? "ready" : "not ready"} (${completedCount}/30 completed; high-volume roles need five samples each).`);
  if (highVolumeRoles.length > 0) {
    lines.push(`- High-volume roles (at least 10% of completed records): ${highVolumeRoles.map((row) => `${row.key}=${row.records}`).join(", ")}.`);
  } else {
    lines.push("- High-volume roles: insufficient completed records to assess.");
  }
  if (lastReviewedCompleted === undefined) {
    lines.push("- Fifty-task cadence: unknown until the first review checkpoint is supplied with --last-reviewed-completed.");
  } else {
    const sinceReview = completedCount - lastReviewedCompleted;
    lines.push(`- Fifty-task cadence: ${sinceReview >= 50 ? "review due" : `${sinceReview}/50 completed since the last review`} (checkpoint=${lastReviewedCompleted}).`);
  }
  const triggers = repeatedFailureTriggers(records);
  if (triggers.length === 0) {
    lines.push("- Repeated-failure trigger: none (requires three matching failure tags in the latest ten comparable records).");
  } else {
    lines.push("- Repeated-failure trigger: review due for:");
    for (const trigger of triggers) {
      lines.push(`  - ${trigger.key}; ${trigger.tag} occurred ${trigger.count} times in its latest ${trigger.windowSize} comparable records.`);
    }
  }
}

function validateReviewCheckpoint(lastReviewedCompleted, completedCount) {
  if (lastReviewedCompleted === undefined) {
    return;
  }
  assertNonNegativeInteger(lastReviewedCompleted, "lastReviewedCompleted");
  if (lastReviewedCompleted > completedCount) {
    throw new Error("lastReviewedCompleted cannot exceed completed delegations");
  }
}

function repeatedFailureTriggers(records) {
  const groups = new Map();
  for (const record of [...records].sort((left, right) => Date.parse(left.recorded_at) - Date.parse(right.recorded_at))) {
    const key = [record.workflow, record.role, record.route_source, record.task_class, record.risk_class, modelLabel(record)].join(" | ");
    if (!groups.has(key)) {
      groups.set(key, []);
    }
    groups.get(key).push(record);
  }
  const triggers = [];
  for (const [key, group] of groups) {
    const window = group.slice(-10);
    const tagCounts = new Map();
    for (const record of window) {
      for (const tag of record.failure_tags) {
        tagCounts.set(tag, (tagCounts.get(tag) ?? 0) + 1);
      }
    }
    for (const [tag, count] of tagCounts) {
      if (count >= 3) {
        triggers.push({ key, tag, count, windowSize: window.length });
      }
    }
  }
  return triggers;
}

function renderClassificationAudits(lines, records) {
  const audits = records.filter((record) => record.classification_audit !== undefined);
  if (audits.length === 0) {
    lines.push("No blinded classification audits recorded.");
    return;
  }
  table(lines, ["Audit", "Blinded input", "Independent result", "Recorded class", "Agreement"], audits.map((record) => {
    const audit = record.classification_audit;
    const input = audit.blinded_input;
    const matches = audit.independent_task_class === record.task_class && audit.independent_risk_class === record.risk_class;
    return [
      audit.audit_id,
      `files=${input.file_count}; public=${input.public_contract}; api=${input.uncertain_api}; semantic=${input.semantic_core}; lifecycle=${input.process_lifecycle}; security=${input.security_or_data_loss}; verify=${input.verification_strength}`,
      `${audit.independent_task_class}/${audit.independent_risk_class}`,
      `${record.task_class}/${record.risk_class}`,
      matches ? "match" : "mismatch",
    ];
  }));
}

function renderCriticalAndRecovery(lines, records) {
  const selected = records.filter((record) => record.task_class === "critical"
    || record.risk_class === "critical"
    || record.role === "recovery-implementer"
    || record.requested_route.effort === "xhigh"
    || record.escalation_reasons.includes("recovery"));
  table(lines, ["Attempt", "When", "Workflow", "Role", "Route", "Classes", "Terminal", "Reasons", "Verification", "Corrections", "Failure tags"], selected.map((record) => [
    record.attempt_id,
    record.recorded_at,
    record.workflow,
    record.role,
    modelLabel(record),
    `${record.task_class}/${record.risk_class}`,
    record.terminal_state,
    `select=${record.selection_reasons.join(",")}; escalate=${record.escalation_reasons.join(",") || "none"}`,
    record.verification.status,
    record.corrections.count,
    record.failure_tags.join(",") || "none",
  ]));
}

function isCompletedDelegation(record) {
  return !["unavailable", "missing-return"].includes(record.terminal_state);
}

function routeLabel(record) {
  return `${record.workflow} | ${record.role} | ${record.route_source} | ${modelLabel(record)} | ${record.task_class}/${record.risk_class}`;
}

function modelLabel(record) {
  const requested = `${record.requested_route.model}/${record.requested_route.effort}`;
  const actual = record.actual_route ? `${record.actual_route.model}/${record.actual_route.effort}` : "unreported";
  return `${requested} -> ${actual}`;
}

function rate(value, total) {
  return total === 0 ? 0 : value / total;
}

function countRate(value, total) {
  return `${value} (${percent(rate(value, total))})`;
}

function percent(value) {
  return `${Math.round(value * 1000) / 10}%`;
}

function table(lines, headers, rows) {
  if (rows.length === 0) {
    lines.push("None.");
    return;
  }
  lines.push(`| ${headers.join(" | ")} |`);
  lines.push(`| ${headers.map((header) => /Records|pass|Correction|Escalation|failed|Available|tags|Rate|Corrections|receipts|cost/.test(header) ? "---:" : "---").join(" | ")} |`);
  for (const row of rows) {
    lines.push(`| ${row.map((value) => escapeMarkdown(value)).join(" | ")} |`);
  }
}

function escapeMarkdown(value) {
  return String(value).replaceAll("|", "\\|").replaceAll("\n", " ");
}
