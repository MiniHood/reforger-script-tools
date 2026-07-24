#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { homedir } from "node:os";

const args = parseArgs(process.argv.slice(2));
const globalStorage = args.globalStorage ?? defaultGlobalStorage();
const logPath = args.log ?? join(globalStorage, "logs", "language-server.log");
const out = args.out ?? join("tools", "reports", "lsp-runtime-performance.report.md");
const sinceMinutes = args.sinceMinutes;
const records = readLogRecords(logPath);
const filteredRecords = filterByTime(records, sinceMinutes);
const report = renderReport({
  generatedAt: new Date(),
  globalStorage,
  logPath,
  sinceMinutes,
  records: filteredRecords,
  totalRecords: records.length,
});

mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, report, "utf8");
console.log(`Wrote ${out}`);

function parseArgs(rawArgs) {
  const parsed = {};
  for (let index = 0; index < rawArgs.length; index += 1) {
    const arg = rawArgs[index];
    if (arg === "--global-storage") {
      parsed.globalStorage = rawArgs[++index];
    } else if (arg === "--log") {
      parsed.log = rawArgs[++index];
    } else if (arg === "--out") {
      parsed.out = rawArgs[++index];
    } else if (arg === "--since-minutes") {
      parsed.sinceMinutes = Number(rawArgs[++index]);
      if (!Number.isFinite(parsed.sinceMinutes) || parsed.sinceMinutes < 0) {
        throw new Error("--since-minutes must be a positive number");
      }
    } else if (arg === "--help" || arg === "-h") {
      console.log("Usage: node tools/lsp-runtime-performance-report.mjs [--global-storage <path>] [--log <path>] [--out <path>] [--since-minutes <n>]");
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return parsed;
}

function defaultGlobalStorage() {
  if (process.platform === "win32") {
    return join(process.env.APPDATA ?? join(homedir(), "AppData", "Roaming"), "Code", "User", "globalStorage", "undefined_publisher.reforger-sript-tools");
  }
  return join(homedir(), ".config", "Code", "User", "globalStorage", "undefined_publisher.reforger-sript-tools");
}

function readLogRecords(path) {
  if (!existsSync(path)) {
    return [];
  }
  const lines = readFileSync(path, "utf8").split(/\r?\n/).filter((line) => line.length > 0);
  const records = [];
  let current;
  for (const line of lines) {
    const match = line.match(/^\[(\d+)]\s+(.*)$/);
    if (match) {
      if (current) {
        records.push(finalizeRecord(current));
      }
      current = {
        timestamp: Number(match[1]),
        text: match[2],
      };
    } else if (current) {
      current.text += ` ${line.trim()}`;
    }
  }
  if (current) {
    records.push(finalizeRecord(current));
  }
  return records;
}

function finalizeRecord(record) {
  record.text = record.text.replace(/\s+/g, " ").trim();
  record.operation = operationName(record.text);
  record.fields = parseFields(record.text);
  record.elapsedMs = numberField(record.fields, "elapsed_ms")
    ?? numberField(record.fields, "analysis_elapsed_ms")
    ?? numberField(record.fields, "cache_total_ms")
    ?? numberField(record.fields, "document_symbol_ms")
    ?? 0;
  record.uri = record.fields.uri ?? record.fields.path ?? record.fields.scripts ?? "";
  record.fileName = basenameFromUri(record.uri);
  record.revision = record.fields.selected_revision ?? record.fields.revision ?? "";
  record.version = record.fields.selected_version ?? record.fields.version ?? "";
  if (record.operation === "notification didChange") {
    record.didChange = {
      queueMs: numberField(record.fields, "queue_ms") ?? 0,
      coalescedChanges: numberField(record.fields, "coalesced_changes") ?? 0,
      supersededChanges: numberField(record.fields, "superseded_changes") ?? 0,
      selectedVersion: numberField(record.fields, "selected_version") ?? numberField(record.fields, "version") ?? 0,
      selectedRevision: numberField(record.fields, "selected_revision") ?? numberField(record.fields, "revision") ?? 0,
    };
  }
  return record;
}

function operationName(text) {
  const keyIndex = text.search(/\s[a-zA-Z_][a-zA-Z0-9_]*=/);
  return (keyIndex >= 0 ? text.slice(0, keyIndex) : text).trim();
}

function parseFields(text) {
  const fields = {};
  const matches = text.matchAll(/(^|\s)([a-zA-Z_][a-zA-Z0-9_]*)=(.*?)(?=\s[a-zA-Z_][a-zA-Z0-9_]*=|$)/g);
  for (const match of matches) {
    fields[match[2]] = match[3].trim();
  }
  return fields;
}

function numberField(fields, key) {
  const raw = fields[key];
  if (raw === undefined) {
    return undefined;
  }
  const value = Number(raw);
  return Number.isFinite(value) ? value : undefined;
}

function basenameFromUri(uri) {
  if (!uri) {
    return "";
  }
  const normalized = uri.replaceAll("\\", "/");
  const decoded = safeDecodeURIComponent(normalized);
  return decoded.split("/").filter(Boolean).at(-1) ?? decoded;
}

function safeDecodeURIComponent(value) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function filterByTime(records, sinceMinutes) {
  if (!sinceMinutes || records.length === 0) {
    return records;
  }
  const latest = records.at(-1).timestamp;
  const cutoff = latest - sinceMinutes * 60_000;
  return records.filter((record) => record.timestamp >= cutoff);
}

function renderReport(input) {
  const records = input.records;
  const summary = summarize(records);
  const slowRecords = records
    .filter((record) => record.elapsedMs > 0)
    .sort((left, right) => right.elapsedMs - left.elapsedMs)
    .slice(0, 30);
  const operationRows = Array.from(summary.byOperation.values())
    .sort((left, right) => right.totalMs - left.totalMs)
    .slice(0, 30);
  const fileRows = Array.from(summary.byFile.values())
    .sort((left, right) => right.totalMs - left.totalMs)
    .slice(0, 30);
  const windows = summarizeWindows(records, 1000)
    .sort((left, right) => right.totalMs - left.totalMs)
    .slice(0, 20);
  const revisionRows = summarizeRevisionGroups(records);
  const captureRows = summarizeCaptureWindows(revisionRows);
  const snapshotQuality = summarizeSnapshotQuality(records);
  const queryQuality = summarizeQueryQuality(records);
  const admission = summarizeAdmission(records);
  const cancellation = summarizeCancellation(records);
  const richIdentity = summarizeRichIdentity(records);
  const selfSaveRichReuse = summarizeSelfSaveRichReuse(records);
  const captureFields = summarizeCaptureFields(records);
  const externalCache = summarizeExternalCache(records);

  const lines = [];
  lines.push("# LSP Runtime Performance Report");
  lines.push("");
  lines.push("This report parses the Rust language-server runtime log and estimates where foreground latency and background CPU-like work are coming from. It does not sample OS CPU directly; it uses logged elapsed timings, request counts, stale worker records, and per-operation phase fields.");
  lines.push("");
  lines.push("## Summary");
  lines.push("");
  lines.push(`- Generated: ${input.generatedAt.toISOString()}`);
  lines.push(`- Log: \`${input.logPath}\`${existsSync(input.logPath) ? "" : " (missing)"}`);
  lines.push(`- Scope: ${input.sinceMinutes ? `last ${input.sinceMinutes} minutes` : "entire log"}`);
  lines.push(`- Capture window: ${input.sinceMinutes ? "explicit --since-minutes window" : "entire log (use --since-minutes for a controlled capture)"}`);
  lines.push(`- Records analyzed: ${records.length} / ${input.totalRecords}`);
  lines.push(`- Timed work total: ${formatMs(summary.totalMs)}`);
  lines.push(`- Foreground request/notification time: ${formatMs(summary.foregroundMs)}`);
  lines.push(`- Background rich semantic-token time: ${formatMs(summary.richSemanticMs)}`);
  lines.push(`- Self-save rich projections reused: ${selfSaveRichReuse.reusedCount}`);
  lines.push(`- Self-save in-flight projections retargeted: ${selfSaveRichReuse.retargetedCount}`);
  lines.push(`- Reused rich elapsed-time reference: ${formatMs(selfSaveRichReuse.referenceElapsedMs)}`);
  lines.push(`- Stale/skipped rich semantic-token records: ${summary.staleRichCount}`);
  lines.push(`- Cancelled rich semantic-token records: ${summary.cancelledRichCount}`);
  lines.push(`- Foreground responses with declared query quality: ${queryQuality.declared} / ${queryQuality.records}`);
  lines.push(`- Capture field gaps: ${captureFields.totalMissing}`);
  lines.push(`- First usable token observations: ${snapshotQuality.firstToken.latencies.length} / ${snapshotQuality.acceptedSnapshots}`);
  lines.push(`- First completion observations: ${snapshotQuality.firstCompletion.latencies.length} / ${snapshotQuality.acceptedSnapshots}`);
  lines.push(`- Slowest operation: ${slowRecords[0] ? `${slowRecords[0].operation} (${formatMs(slowRecords[0].elapsedMs)})` : "None"}`);
  lines.push("");
  lines.push("## External Game-Data Cache");
  lines.push("");
  lines.push("Ready-state telemetry is source-free: it records only cache lifecycle, index counts, byte size, and timings. A missing byte count remains visible as unavailable rather than being inferred.");
  lines.push("");
  if (externalCache.rows.length === 0) {
    lines.push("No game-data ready records were present in this capture.");
  } else {
    table(lines, ["Cache status", "Files", "Symbols", "Cache bytes", "Cache total"], externalCache.rows.map((row) => [
      row.status,
      row.files,
      row.symbols,
      row.cacheBytes === undefined ? "Unavailable" : formatBytes(row.cacheBytes),
      formatMs(row.cacheTotalMs),
    ]));
  }
  lines.push("");
  lines.push("## Interpretation");
  lines.push("");
  for (const note of interpretation(summary, slowRecords)) {
    lines.push(`- ${note}`);
  }
  lines.push("");
  lines.push("## Work By Operation");
  lines.push("");
  table(lines, ["Operation", "Count", "Total ms", "Avg", "P95", "Max"], operationRows.map((row) => [
    row.name,
    row.count,
    formatMs(row.totalMs),
    formatMs(row.totalMs / row.count),
    formatMs(percentile(row.elapsed, 0.95)),
    formatMs(Math.max(...row.elapsed)),
  ]));
  lines.push("");
  lines.push("## Top Files / URIs By Timed Work");
  lines.push("");
  table(lines, ["File", "Count", "Total ms", "Avg", "Max"], fileRows.map((row) => [
    row.name || "<none>",
    row.count,
    formatMs(row.totalMs),
    formatMs(row.totalMs / row.count),
    formatMs(Math.max(...row.elapsed)),
  ]));
  lines.push("");
  lines.push("## Slowest Records");
  lines.push("");
  table(lines, ["Timestamp", "Operation", "File", "Elapsed", "Detail"], slowRecords.map((record) => [
    String(record.timestamp),
    record.operation,
    record.fileName || "",
    formatMs(record.elapsedMs),
    compactDetail(record),
  ]));
  lines.push("");
  lines.push("## Hottest One-Second Windows");
  lines.push("");
  table(lines, ["Window Start", "Records", "Total ms", "Top Operation", "Top File"], windows.map((window) => [
    String(window.start),
    window.count,
    formatMs(window.totalMs),
    window.topOperation,
    window.topFile,
  ]));
  lines.push("");
  lines.push("## Semantic Token Worker Health");
  lines.push("");
  lines.push(`- Rich ready records: ${summary.richReadyCount}`);
  lines.push(`- Rich stale/skipped records: ${summary.staleRichCount}`);
  lines.push(`- Rich cancelled records: ${summary.cancelledRichCount}`);
  lines.push(`- Rich ready total: ${formatMs(summary.richReadyMs)}`);
  lines.push(`- Rich stale/skipped total: ${formatMs(summary.staleRichMs)}`);
  lines.push(`- Rich cancelled total: ${formatMs(summary.cancelledRichMs)}`);
  lines.push(`- Fast semantic-token request total: ${formatMs(summary.fastSemanticMs)}`);
  lines.push("");
  lines.push("## Self-Save Rich Projection Reuse");
  lines.push("");
  lines.push("A `ready` row records an already-complete overlay carried across a self-save. A `pending` row records retargeting only and is not counted as reuse unless a matching rich-ready terminal produces a `completed` row. Observed rich elapsed time includes original scheduling and is reference evidence, not a counterfactual CPU-savings measurement.");
  lines.push("");
  if (selfSaveRichReuse.rows.length === 0) {
    lines.push("No detailed self-save reuse records were present in this capture.");
  } else {
    table(lines, ["File", "Revision", "From generation", "To generation", "State", "Observed rich elapsed"], selfSaveRichReuse.rows.map((row) => [
      row.fileName || "<none>",
      row.revision || "<missing>",
      row.previousExternalGeneration ?? "<missing>",
      row.externalGeneration ?? "<missing>",
      row.state || "<missing>",
      row.referenceElapsedMs === undefined ? "Pending" : formatMs(row.referenceElapsedMs),
    ]));
  }
  lines.push("");
  lines.push("## Completion Latency");
  lines.push("");
  lines.push(`- Completion requests: ${summary.completion.count}`);
  lines.push(`- Completion total: ${formatMs(summary.completion.totalMs)}`);
  lines.push(`- Completion queue total: ${formatMs(summary.completion.queueMs)}`);
  lines.push(`- Completion p95: ${formatMs(percentile(summary.completion.elapsed, 0.95))}`);
  lines.push(`- Completion queue p95: ${formatMs(percentile(summary.completion.queueElapsed, 0.95))}`);
  lines.push(`- Completion perceived p95: ${formatMs(percentile(summary.completion.perceivedElapsed, 0.95))}`);
  lines.push(`- Completion max: ${formatMs(summary.completion.elapsed.length ? Math.max(...summary.completion.elapsed) : 0)}`);
  lines.push(`- Completion candidate lookup total: ${formatMs(summary.completion.lookupMs)}`);
  lines.push("");
  lines.push("## First Usable Foreground Response");
  lines.push("");
  lines.push("Latency is observed from an accepted `didOpen`/`didChange` snapshot to the first matching token or completion response in this log. It measures end-to-end log time, not source text or completion payload contents. Missing pairs are reported as unavailable rather than estimated.");
  lines.push("");
  table(lines, ["Response", "Observed", "Unpaired", "P95", "Max", "Lexical-pending"], [
    ["Usable tokens", snapshotQuality.firstToken.latencies.length, snapshotQuality.firstToken.unpaired, formatMs(percentile(snapshotQuality.firstToken.latencies, 0.95)), formatMs(maxOrZero(snapshotQuality.firstToken.latencies)), snapshotQuality.firstToken.lexicalPending],
    ["Completion", snapshotQuality.firstCompletion.latencies.length, snapshotQuality.firstCompletion.unpaired, formatMs(percentile(snapshotQuality.firstCompletion.latencies, 0.95)), formatMs(maxOrZero(snapshotQuality.firstCompletion.latencies)), "n/a"],
  ]);
  lines.push("");
  lines.push("## Snapshot Quality");
  lines.push("");
  lines.push("A request is **current** only when its URI and revision match an accepted open-document snapshot in the selected window. Older logs without a revision remain **unpaired**, not stale by assumption.");
  lines.push("");
  table(lines, ["Measure", "Count"], [
    ["Accepted snapshots", snapshotQuality.acceptedSnapshots],
    ["Current foreground responses", snapshotQuality.currentForegroundResponses],
    ["Unpaired foreground responses", snapshotQuality.unpairedForegroundResponses],
    ["Matching semantic publications", snapshotQuality.matchingSemanticPublications],
    ["Stale/discarded semantic publications", snapshotQuality.staleSemanticPublications],
  ]);
  lines.push("");
  lines.push("## Foreground Query Quality");
  lines.push("");
  lines.push("Only feature responses that emit `query_quality` are counted. `Exact`, `RecoveryExact`, and `Unavailable` are server-declared current-snapshot guarantees; missing quality is a capture gap, not an inferred result. This section never reads source text, prefixes, labels, or payloads.");
  lines.push("");
  table(lines, ["Feature", "Responses", "Exact", "RecoveryExact", "Unavailable", "Missing quality", "Unpaired identity"], queryQuality.rows.map((row) => [
    row.feature,
    row.records,
    row.exact,
    row.recoveryExact,
    row.unavailable,
    row.missing,
    row.unpaired,
  ]));
  lines.push("");
  lines.push("## Admission and Overload");
  lines.push("");
  lines.push("Admission metrics use explicit runtime disposition fields (`disposition`, `admission`, or `outcome`) or runtime admission operation names. Legacy logs without those fields are shown as unavailable; worker completion records are not treated as admission evidence.");
  lines.push("");
  if (admission.records === 0) {
    lines.push("No explicit admission records were present in this capture.");
  } else {
    table(lines, ["Disposition", "Count"], [
      ["Admitted", admission.admitted], ["Queued", admission.queued], ["Overloaded/rejected/dropped", admission.overloaded], ["Cancelled", admission.cancelled], ["Other", admission.other],
    ]);
    lines.push("");
    table(lines, ["Lane", "Records", "Missing identity/disposition"], admission.lanes.map((row) => [row.lane, row.records, row.missing]));
  }
  lines.push("");
  lines.push("## Cancellation Tails");
  lines.push("");
  lines.push("A cancellation tail is reported only when a terminal cancellation record includes `cancellation_tail_ms` or `tail_ms`. Total worker elapsed time is not a cancellation-tail proxy.");
  lines.push("");
  lines.push(`- Cancelled terminal records: ${cancellation.cancelledRecords}`);
  lines.push(`- Measured tails: ${cancellation.tails.length}`);
  lines.push(`- Cancellation tail p95: ${formatMs(percentile(cancellation.tails, 0.95))}`);
  lines.push(`- Cancellation tail max: ${formatMs(maxOrZero(cancellation.tails))}`);
  lines.push("");
  lines.push("## Rich Cancellation and Identity");
  lines.push("");
  lines.push("Rich work is attributable only when its document identity (`uri` + `revision`) and external-overlay identity (`external_generation`) are logged. Cancellation is attributable only when a terminal reason is present. Missing markers are reported as evidence gaps; no text or token payload is retained.");
  lines.push("");
  table(lines, ["Marker", "Present", "Missing"], [
    ["Rich document identity", richIdentity.documentIdentity, richIdentity.records - richIdentity.documentIdentity],
    ["Rich external-generation identity", richIdentity.externalGeneration, richIdentity.records - richIdentity.externalGeneration],
    ["Cancelled rich terminal reason", richIdentity.cancelledReason, richIdentity.cancelled - richIdentity.cancelledReason],
    ["Cancelled rich tail", richIdentity.cancelledTail, richIdentity.cancelled - richIdentity.cancelledTail],
  ]);
  lines.push("");
  lines.push("## Burst Comparison");
  lines.push("");
  lines.push("Groups are attributed by URI and accepted document revision. A capture is qualified only when its URI has at least ten completion requests in the selected report window; this preserves revision attribution while judging the controlled burst as a whole.");
  lines.push("");
  table(lines, ["File", "Version", "Revision", "Changes", "Completions", "Completion queue p95", "Perceived p95", "Coalesced", "Superseded"], revisionRows.map((row) => [
    row.fileName || "<none>",
    row.version || "<missing>",
    row.revision || "<missing>",
    row.didChangeCount,
    row.completionCount,
    formatMs(percentile(row.completionQueueElapsed, 0.95)),
    formatMs(percentile(row.completionPerceivedElapsed, 0.95)),
    row.coalescedChanges,
    row.supersededChanges,
  ]));
  lines.push("");
  lines.push("## Capture Evidence Quality");
  lines.push("");
  lines.push("A controlled file capture needs at least ten qualified completion requests. Treat an insufficient row as diagnostic context, not before/after proof.");
  lines.push("");
  table(lines, ["File", "Revisions", "Qualified completions", "Classification"], captureRows.map((row) => [
    row.fileName || "<none>",
    row.revisionCount,
    row.completionCount,
    row.completionCount >= 10 ? "Sufficient" : "Insufficient",
  ]));
  lines.push("");
  lines.push("## Capture Field Completeness");
  lines.push("");
  lines.push("These are source-free telemetry requirements for a comparable U7 capture. A missing field is not defaulted or inferred in this audit, even where older report summaries retain a compatibility default.");
  lines.push("");
  table(lines, ["Record family", "Records", "Missing markers", "Status"], captureFields.rows.map((row) => [
    row.name,
    row.records,
    row.missing,
    row.missing === 0 ? "Complete" : "Incomplete",
  ]));
  lines.push("");
  lines.push("## Edit Analysis Latency");
  lines.push("");
  lines.push(`- didChange count: ${summary.didChange.count}`);
  lines.push(`- didChange total: ${formatMs(summary.didChange.totalMs)}`);
  lines.push(`- didChange queue total: ${formatMs(summary.didChange.queueMs)}`);
  lines.push(`- didChange p95: ${formatMs(percentile(summary.didChange.elapsed, 0.95))}`);
  lines.push(`- didChange queue p95: ${formatMs(percentile(summary.didChange.queueElapsed, 0.95))}`);
  lines.push(`- Analysis catalog total: ${formatMs(summary.didChange.catalogMs)}`);
  lines.push(`- Analysis parse total: ${formatMs(summary.didChange.parseMs)}`);
  lines.push(`- Analysis scope total: ${formatMs(summary.didChange.scopeMs)}`);
  lines.push(`- Background analysis ready: ${summary.documentAnalysis.readyCount}`);
  lines.push(`- Background analysis ready total: ${formatMs(summary.documentAnalysis.readyMs)}`);
  lines.push(`- Background analysis ready p95: ${formatMs(percentile(summary.documentAnalysis.readyElapsed, 0.95))}`);
  lines.push(`- Background analysis superseded: ${summary.documentAnalysis.skippedCount}`);
  lines.push(`- Background analysis superseded total: ${formatMs(summary.documentAnalysis.skippedMs)}`);
  lines.push("");
  return `${lines.join("\n")}\n`;
}

function summarize(records) {
  const summary = {
    totalMs: 0,
    foregroundMs: 0,
    richSemanticMs: 0,
    richReadyCount: 0,
    richReadyMs: 0,
    staleRichCount: 0,
    staleRichMs: 0,
    cancelledRichCount: 0,
    cancelledRichMs: 0,
    fastSemanticMs: 0,
    completion: {
      count: 0,
      totalMs: 0,
      queueMs: 0,
      elapsed: [],
      queueElapsed: [],
      perceivedElapsed: [],
      lookupMs: 0,
    },
    didChange: {
      count: 0,
      totalMs: 0,
      queueMs: 0,
      elapsed: [],
      queueElapsed: [],
      catalogMs: 0,
      parseMs: 0,
      scopeMs: 0,
    },
    documentAnalysis: {
      readyCount: 0,
      readyMs: 0,
      readyElapsed: [],
      skippedCount: 0,
      skippedMs: 0,
    },
    byOperation: new Map(),
    byFile: new Map(),
  };
  for (const record of records) {
    const elapsed = record.elapsedMs;
    summary.totalMs += elapsed;
    addRow(summary.byOperation, record.operation, elapsed);
    addRow(summary.byFile, record.fileName || record.uri || "<none>", elapsed);

    if (record.operation.startsWith("request ") || record.operation.startsWith("notification ")) {
      summary.foregroundMs += elapsed;
    }
    if (record.operation === "request completion") {
      const queue = numberField(record.fields, "queue_ms") ?? 0;
      summary.completion.count += 1;
      summary.completion.totalMs += elapsed;
      summary.completion.queueMs += queue;
      summary.completion.elapsed.push(elapsed);
      summary.completion.queueElapsed.push(queue);
      summary.completion.perceivedElapsed.push(queue + elapsed);
      summary.completion.lookupMs += numberField(record.fields, "lookup_ms") ?? 0;
    }
    if (record.operation === "notification didChange") {
      const queue = record.didChange.queueMs;
      summary.didChange.count += 1;
      summary.didChange.totalMs += elapsed;
      summary.didChange.queueMs += queue;
      summary.didChange.elapsed.push(elapsed);
      summary.didChange.queueElapsed.push(queue);
      summary.didChange.catalogMs += numberField(record.fields, "analysis_catalog_ms") ?? 0;
      summary.didChange.parseMs += numberField(record.fields, "analysis_parse_ms") ?? 0;
      summary.didChange.scopeMs += numberField(record.fields, "analysis_scope_ms") ?? 0;
    }
    if (record.operation === "documentAnalysis ready") {
      summary.documentAnalysis.readyCount += 1;
      summary.documentAnalysis.readyMs += elapsed;
      summary.documentAnalysis.readyElapsed.push(elapsed);
    } else if (record.operation === "documentAnalysis skipped" || record.operation === "documentAnalysis discarded") {
      summary.documentAnalysis.skippedCount += 1;
      summary.documentAnalysis.skippedMs += elapsed;
    }
    if (record.operation === "request semanticTokens" && record.fields.mode === "fast-compute") {
      summary.fastSemanticMs += elapsed;
    }
    if (record.operation === "semanticTokensRich ready") {
      summary.richReadyCount += 1;
      summary.richReadyMs += elapsed;
      summary.richSemanticMs += elapsed;
    } else if (record.operation === "semanticTokensRich discarded" || record.operation === "semanticTokensRich skipped") {
      const reason = record.fields.reason ?? "";
      if (reason.startsWith("cancelled-")) {
        summary.cancelledRichCount += 1;
        summary.cancelledRichMs += elapsed;
      } else {
        summary.staleRichCount += 1;
        summary.staleRichMs += elapsed;
      }
      summary.richSemanticMs += elapsed;
    }
  }
  return summary;
}

function summarizeSelfSaveRichReuse(records) {
  const richReadyKeys = new Set(records
    .filter((record) => record.operation === "semanticTokensRich ready")
    .map((record) => [
      record.uri,
      record.revision,
      record.fields.task_external_generation ?? record.fields.external_generation ?? "",
      record.fields.external_generation ?? "",
    ].join("\u0000")));
  const rows = records
    .filter((record) =>
      record.operation === "semanticTokens self-save reused"
      || record.operation === "semanticTokens self-save retargeted")
    .map((record) => ({
      operation: record.operation,
      fileName: record.fileName,
      uri: record.uri,
      revision: record.revision,
      previousExternalGeneration: numberField(record.fields, "previous_external_generation"),
      externalGeneration: numberField(record.fields, "external_generation"),
      state: record.fields.state ?? "",
      referenceElapsedMs: numberField(record.fields, "reference_elapsed_ms"),
    }));
  const completedReuseRows = rows.filter((row) => {
    if (row.operation !== "semanticTokens self-save reused" || row.state !== "completed") {
      return row.operation === "semanticTokens self-save reused";
    }
    return richReadyKeys.has([
      row.uri,
      row.revision,
      row.previousExternalGeneration ?? "",
      row.externalGeneration ?? "",
    ].join("\u0000"));
  });
  const evidenceRows = rows.filter((row) =>
    row.operation === "semanticTokens self-save retargeted"
    || completedReuseRows.includes(row));
  return {
    rows: evidenceRows,
    reusedCount: completedReuseRows.length,
    retargetedCount: rows.filter((row) => row.operation === "semanticTokens self-save retargeted").length,
    referenceElapsedMs: completedReuseRows.reduce(
      (total, row) => total + (row.referenceElapsedMs ?? 0),
      0,
    ),
  };
}

function summarizeRevisionGroups(records) {
  const groups = new Map();
  for (const record of records) {
    if (!record.uri || !record.revision) {
      continue;
    }
    const key = `${record.uri}\u0000${record.revision}`;
    if (!groups.has(key)) {
      groups.set(key, {
        uri: record.uri,
        fileName: record.fileName,
        version: record.version,
        revision: record.revision,
        didChangeCount: 0,
        completionCount: 0,
        completionQueueElapsed: [],
        completionPerceivedElapsed: [],
        coalescedChanges: 0,
        supersededChanges: 0,
      });
    }
    const group = groups.get(key);
    if (record.operation === "notification didChange") {
      group.didChangeCount += 1;
      group.coalescedChanges += record.didChange.coalescedChanges;
      group.supersededChanges += record.didChange.supersededChanges;
    } else if (record.operation === "request completion") {
      const queue = numberField(record.fields, "queue_ms") ?? 0;
      group.completionCount += 1;
      group.completionQueueElapsed.push(queue);
      group.completionPerceivedElapsed.push(queue + record.elapsedMs);
    }
  }
  return Array.from(groups.values())
    .sort((left, right) => left.uri.localeCompare(right.uri) || Number(left.revision) - Number(right.revision));
}

function summarizeCaptureWindows(revisionRows) {
  const captures = new Map();
  for (const row of revisionRows) {
    if (!captures.has(row.uri)) {
      captures.set(row.uri, {
        fileName: row.fileName,
        revisionCount: 0,
        completionCount: 0,
      });
    }
    const capture = captures.get(row.uri);
    capture.revisionCount += 1;
    capture.completionCount += row.completionCount;
  }
  return Array.from(captures.values()).sort((left, right) => left.fileName.localeCompare(right.fileName));
}

function summarizeSnapshotQuality(records) {
  const snapshots = new Map();
  const firstToken = { latencies: [], unpaired: 0, lexicalPending: 0 };
  const firstCompletion = { latencies: [], unpaired: 0 };
  let acceptedSnapshots = 0;
  let currentForegroundResponses = 0;
  let unpairedForegroundResponses = 0;
  let matchingSemanticPublications = 0;
  let staleSemanticPublications = 0;

  for (const record of records) {
    if (isAcceptedSnapshot(record)) {
      const key = snapshotKey(record);
      if (key) {
        snapshots.set(key, { timestamp: record.timestamp, firstToken: false, firstCompletion: false });
        acceptedSnapshots += 1;
      }
      continue;
    }
    if (record.operation === "documentAnalysis ready") {
      if (snapshots.has(snapshotKey(record))) matchingSemanticPublications += 1;
      else staleSemanticPublications += 1;
      continue;
    }
    if (isStaleSemanticTerminal(record)) {
      staleSemanticPublications += 1;
      continue;
    }
    const isToken = record.operation === "request semanticTokens";
    const isCompletion = record.operation === "request completion";
    if (!isToken && !isCompletion) continue;
    const snapshot = snapshots.get(snapshotKey(record));
    if (!snapshot) {
      unpairedForegroundResponses += 1;
      if (isToken) firstToken.unpaired += 1;
      else firstCompletion.unpaired += 1;
      continue;
    }
    currentForegroundResponses += 1;
    const latency = Math.max(0, record.timestamp - snapshot.timestamp);
    if (isToken && !snapshot.firstToken) {
      snapshot.firstToken = true;
      firstToken.latencies.push(latency);
      if (record.fields.mode === "lexical-pending") firstToken.lexicalPending += 1;
    }
    if (isCompletion && !snapshot.firstCompletion) {
      snapshot.firstCompletion = true;
      firstCompletion.latencies.push(latency);
    }
  }
  return { acceptedSnapshots, currentForegroundResponses, unpairedForegroundResponses, matchingSemanticPublications, staleSemanticPublications, firstToken, firstCompletion };
}

function isAcceptedSnapshot(record) {
  return (record.operation === "notification didOpen" || record.operation === "notification didChange") && record.fields.uri !== undefined && record.revision !== "";
}

function isStaleSemanticTerminal(record) {
  return record.operation === "documentAnalysis skipped" || record.operation === "documentAnalysis discarded";
}

function snapshotKey(record) {
  if (!record.uri || record.revision === "") return "";
  return `${record.uri}\u0000${record.revision}`;
}

function summarizeAdmission(records) {
  const summary = { records: 0, admitted: 0, queued: 0, overloaded: 0, cancelled: 0, other: 0, byLane: new Map() };
  for (const record of records) {
    const explicitDisposition = record.fields.disposition ?? record.fields.admission ?? record.fields.outcome;
    const admissionOperation = /(?:analysisRuntime|analysisAdmission|runtimeAdmission)/i.test(record.operation);
    if (!explicitDisposition && !admissionOperation) continue;
    summary.records += 1;
    const lane = record.fields.lane ?? "<missing>";
    const missing = (!explicitDisposition ? 1 : 0) + (!record.uri ? 1 : 0) + (record.revision === "" ? 1 : 0);
    const laneRow = summary.byLane.get(lane) ?? { lane, records: 0, missing: 0 };
    laneRow.records += 1;
    laneRow.missing += missing;
    summary.byLane.set(lane, laneRow);
    const disposition = String(explicitDisposition ?? record.operation).toLowerCase();
    if (disposition.includes("admit")) summary.admitted += 1;
    else if (disposition.includes("queue")) summary.queued += 1;
    else if (/(?:overload|reject|drop|limit)/.test(disposition)) summary.overloaded += 1;
    else if (disposition.includes("cancel")) summary.cancelled += 1;
    else summary.other += 1;
  }
  return { ...summary, lanes: Array.from(summary.byLane.values()).sort((left, right) => left.lane.localeCompare(right.lane)) };
}

function summarizeQueryQuality(records) {
  const rows = new Map();
  for (const record of records) {
    if (!isQualityFeatureRecord(record)) continue;
    const feature = record.operation.slice("request ".length);
    const row = rows.get(feature) ?? {
      feature, records: 0, exact: 0, recoveryExact: 0, unavailable: 0, missing: 0, unpaired: 0,
    };
    row.records += 1;
    const quality = String(record.fields.query_quality ?? "").toLowerCase();
    if (quality === "exact") row.exact += 1;
    else if (quality === "recoveryexact" || quality === "recovery-exact") row.recoveryExact += 1;
    else if (quality === "unavailable") row.unavailable += 1;
    else row.missing += 1;
    if (!record.uri || record.revision === "") row.unpaired += 1;
    rows.set(feature, row);
  }
  const orderedRows = Array.from(rows.values()).sort((left, right) => left.feature.localeCompare(right.feature));
  return {
    records: orderedRows.reduce((total, row) => total + row.records, 0),
    declared: orderedRows.reduce((total, row) => total + row.exact + row.recoveryExact + row.unavailable, 0),
    rows: orderedRows,
  };
}

function isQualityFeatureRecord(record) {
  return /^(?:request )(?:completion|hover|definition|documentSymbol)$/.test(record.operation);
}

function summarizeRichIdentity(records) {
  const richRecords = records.filter((record) => record.operation.startsWith("semanticTokensRich "));
  const cancelled = richRecords.filter((record) => isCancelledRecord(record));
  return {
    records: richRecords.length,
    documentIdentity: richRecords.filter((record) => record.uri && record.revision !== "").length,
    externalGeneration: richRecords.filter((record) => record.fields.external_generation !== undefined).length,
    cancelled: cancelled.length,
    cancelledReason: cancelled.filter((record) => record.fields.reason !== undefined).length,
    cancelledTail: cancelled.filter((record) => numberField(record.fields, "cancellation_tail_ms") !== undefined || numberField(record.fields, "tail_ms") !== undefined).length,
  };
}

function summarizeCaptureFields(records) {
  const families = [
    { name: "Accepted snapshots", records: records.filter((record) => record.operation === "notification didOpen" || record.operation === "notification didChange"), fields: ["uri", "version", "revision"] },
    { name: "Query-quality feature responses", records: records.filter(isQualityFeatureRecord), fields: ["uri", "revision", "query_quality"] },
    { name: "Runtime admission", records: records.filter((record) => /(?:analysisRuntime|analysisAdmission|runtimeAdmission)/i.test(record.operation) || record.fields.disposition !== undefined || record.fields.admission !== undefined || record.fields.outcome !== undefined), fields: ["uri", "revision", "lane", "disposition"] },
    { name: "Rich semantic-token terminals", records: records.filter((record) => record.operation.startsWith("semanticTokensRich ")), fields: ["uri", "revision", "external_generation"] },
    { name: "Cancelled rich terminals", records: records.filter((record) => record.operation.startsWith("semanticTokensRich ") && isCancelledRecord(record)), fields: ["uri", "revision", "external_generation", "reason", "cancellation_tail_ms"] },
  ];
  const rows = families.map((family) => ({
    name: family.name,
    records: family.records.length,
    missing: family.records.reduce((total, record) => total + family.fields.filter((field) => field === "revision" ? record.revision === "" : record.fields[field] === undefined).length, 0),
  }));
  return { rows, totalMissing: rows.reduce((total, row) => total + row.missing, 0) };
}

function summarizeExternalCache(records) {
  return {
    rows: records
      .filter((record) => record.operation === "externalIndex gameData ready")
      .map((record) => ({
        status: record.fields.cache_status ?? "<missing>",
        files: numberField(record.fields, "files") ?? 0,
        symbols: numberField(record.fields, "symbols") ?? 0,
        cacheBytes: numberField(record.fields, "cache_file_bytes"),
        cacheTotalMs: numberField(record.fields, "cache_total_ms") ?? record.elapsedMs,
      })),
  };
}

function isCancelledRecord(record) {
  return String(record.fields.reason ?? record.fields.disposition ?? record.fields.outcome ?? "").toLowerCase().includes("cancel");
}

function summarizeCancellation(records) {
  const tails = [];
  let cancelledRecords = 0;
  for (const record of records) {
    if (!isCancelledRecord(record)) continue;
    cancelledRecords += 1;
    const tail = numberField(record.fields, "cancellation_tail_ms") ?? numberField(record.fields, "tail_ms");
    if (tail !== undefined) tails.push(tail);
  }
  return { cancelledRecords, tails };
}

function addRow(map, name, elapsed) {
  if (!map.has(name)) {
    map.set(name, { name, count: 0, totalMs: 0, elapsed: [] });
  }
  const row = map.get(name);
  row.count += 1;
  row.totalMs += elapsed;
  row.elapsed.push(elapsed);
}

function summarizeWindows(records, windowMs) {
  const windows = new Map();
  for (const record of records) {
    const start = Math.floor(record.timestamp / windowMs) * windowMs;
    if (!windows.has(start)) {
      windows.set(start, { start, count: 0, totalMs: 0, operations: new Map(), files: new Map() });
    }
    const window = windows.get(start);
    window.count += 1;
    window.totalMs += record.elapsedMs;
    addCounter(window.operations, record.operation, record.elapsedMs);
    addCounter(window.files, record.fileName || record.uri || "<none>", record.elapsedMs);
  }
  return Array.from(windows.values()).map((window) => ({
    start: window.start,
    count: window.count,
    totalMs: window.totalMs,
    topOperation: topCounter(window.operations),
    topFile: topCounter(window.files),
  }));
}

function addCounter(map, name, elapsed) {
  map.set(name, (map.get(name) ?? 0) + elapsed);
}

function topCounter(map) {
  let bestName = "";
  let bestValue = -1;
  for (const [name, value] of map) {
    if (value > bestValue) {
      bestName = name;
      bestValue = value;
    }
  }
  return bestName ? `${bestName} (${formatMs(bestValue)})` : "";
}

function interpretation(summary, slowRecords) {
  const notes = [];
  if (summary.richSemanticMs > summary.foregroundMs) {
    notes.push("Background rich semantic-token work dominates logged elapsed time. If CPU remains high while typing, tune rich-token scheduling/cancellation before optimizing completion.");
  }
  if (summary.staleRichMs > summary.richReadyMs) {
    notes.push("More rich semantic-token time is stale/skipped than useful. This points to edit churn creating obsolete rich work.");
  }
  if (summary.cancelledRichCount > 0 && summary.cancelledRichMs < summary.staleRichMs) {
    notes.push("Rich semantic-token cancellation is active. If discarded stale work remains high, add more cancellation checks inside the expensive projection path.");
  }
  if (summary.didChange.elapsed.length && percentile(summary.didChange.elapsed, 0.95) > 150) {
    notes.push("didChange p95 is above 150 ms. Open-document catalog/model rebuild is a likely typing-latency source for large files.");
  }
  if (summary.completion.elapsed.length && percentile(summary.completion.elapsed, 0.95) > 150) {
    notes.push("completion p95 is above 150 ms. Candidate lookup/ranking should be inspected for broad prefixes.");
  }
  if (slowRecords.some((record) => record.operation === "request documentSymbol" && record.elapsedMs > 150)) {
    notes.push("Document-symbol projection still has slow lazy rebuilds. That should not block every keystroke, but Outline requests can still create short CPU spikes.");
  }
  if (notes.length === 0) {
    notes.push("No dominant runtime offender was obvious in the analyzed log window.");
  }
  return notes;
}

function percentile(values, fraction) {
  if (values.length === 0) {
    return 0;
  }
  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1);
  return sorted[index];
}

function table(lines, headers, rows) {
  if (rows.length === 0) {
    lines.push("None.");
    return;
  }
  lines.push(`| ${headers.join(" |")} |`);
  lines.push(`| ${headers.map((header) => header.match(/^(Count|Total|Avg|P95|Max|Elapsed|Records)/) ? "---:" : "---").join(" | ")} |`);
  for (const row of rows) {
    lines.push(`| ${row.map((value) => escapeMd(value)).join(" | ")} |`);
  }
}

function compactDetail(record) {
  const details = [];
  for (const key of [
    "mode",
    "context",
    "candidates",
    "analysis_build_ms",
    "analysis_catalog_ms",
    "document_symbol_ms",
    "queue_ms",
    "resolver_ms",
    "resolver_calls",
    "type_detail_ms",
    "declaration_symbols_ms",
    "delimiter_ms",
    "delimiter_resolver_calls",
    "reason",
  ]) {
    if (record.fields[key] !== undefined) {
      details.push(`${key}=${record.fields[key]}`);
    }
  }
  return details.join(" ");
}

function formatMs(value) {
  if (!Number.isFinite(value)) {
    return "";
  }
  return `${Math.round(value)} ms`;
}

function formatBytes(value) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MiB`;
}

function maxOrZero(values) {
  return values.length ? Math.max(...values) : 0;
}

function escapeMd(value) {
  return String(value).replaceAll("|", "\\|").replaceAll("\n", " ");
}
