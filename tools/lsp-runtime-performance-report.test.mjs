import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const script = join(process.cwd(), "tools", "lsp-runtime-performance-report.mjs");

function runReport(log) {
  const directory = mkdtempSync(join(tmpdir(), "lsp-runtime-report-"));
  const logPath = join(directory, "language-server.log");
  const outPath = join(directory, "report.md");
  writeFileSync(logPath, log, "utf8");
  const inputBefore = readFileSync(logPath, "utf8");
  const result = spawnSync(process.execPath, [script, "--log", logPath, "--out", outPath], {
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  const report = readFileSync(outPath, "utf8");
  assert.equal(readFileSync(logPath, "utf8"), inputBefore, "reporting must not modify the input log");
  rmSync(directory, { recursive: true, force: true });
  return report;
}

function qualifiedBurst(uri, revision, timestamp) {
  const records = [
    `[${timestamp}] notification didChange uri=${uri} version=7 revision=${revision} analysis_elapsed_ms=120`,
  ];
  for (let index = 0; index < 10; index += 1) {
    records.push(`[${timestamp + 10 + index}] request completion uri=${uri} revision=${revision} queue_ms=15 elapsed_ms=35`);
  }
  return records.join("\n");
}

test("groups typing operations by URI and revision and defaults fixed didChange fields", () => {
  const markerUri = "file:///workspace/GC_MarkerArea.c";
  const soundUri = "file:///workspace/GC_Sounds.c";
  const report = runReport([
    qualifiedBurst(markerUri, 42, 1000),
    qualifiedBurst(soundUri, 9, 2000),
  ].join("\n"));

  assert.match(report, /## Burst Comparison/);
  assert.match(report, /GC_MarkerArea\.c.*7.*42.*10/);
  assert.match(report, /GC_Sounds\.c.*7.*9.*10/);
  assert.match(report, /GC_MarkerArea\.c.*1.*10.*Sufficient/);
  assert.match(report, /GC_Sounds\.c.*1.*10.*Sufficient/);
  assert.match(report, /\| GC_MarkerArea\.c.*\| 0 \| 0 \|/);
  assert.match(report, /didChange queue total: 0 ms/);
});

test("marks captures with fewer than ten completion requests as insufficient", () => {
  const report = runReport([
    "[1000] notification didChange uri=file:///workspace/GC_MarkerArea.c version=7 revision=42 analysis_elapsed_ms=120",
    "[1010] request completion uri=file:///workspace/GC_MarkerArea.c revision=42 elapsed_ms=35",
  ].join("\n"));

  assert.match(report, /GC_MarkerArea\.c.*1.*1.*Insufficient/);
  assert.match(report, /at least ten qualified completion requests/i);
});

test("separates background document analysis from foreground didChange acceptance", () => {
  const report = runReport([
    "[1000] notification didChange uri=file:///workspace/GC_MarkerArea.c version=7 revision=42 cached_analysis=false analysis_state=pending queue_ms=2 analysis_elapsed_ms=2",
    "[1160] documentAnalysis ready uri=file:///workspace/GC_MarkerArea.c version=7 revision=42 analysis_catalog_ms=120 elapsed_ms=160",
    "[1170] documentAnalysis skipped uri=file:///workspace/GC_MarkerArea.c revision=41 reason=superseded-during-analysis elapsed_ms=80",
  ].join("\n"));

  assert.match(report, /didChange total: 2 ms/);
  assert.match(report, /Background analysis ready: 1/);
  assert.match(report, /Background analysis ready total: 160 ms/);
  assert.match(report, /Background analysis superseded: 1/);
});

test("does not reproduce source or completion payload fields in the report", () => {
  const report = runReport([
    "[1000] notification didChange uri=file:///workspace/GC_MarkerArea.c version=7 revision=42 text=DO_NOT_COPY analysis_elapsed_ms=120",
    "[1010] request completion uri=file:///workspace/GC_MarkerArea.c revision=42 prefix=DO_NOT_COPY completion_items=DO_NOT_COPY elapsed_ms=35",
  ].join("\n"));

  assert.doesNotMatch(report, /DO_NOT_COPY/);
});

test("reports source-free game-data cache bytes for rebuilt, loaded, and migrated indexes", () => {
  const report = runReport([
    "[1000] externalIndex gameData ready cache_status=rebuilt cache_detail=cache-missing files=6495 symbols=143145 cache_file_bytes=29360128 cache_total_ms=17000 elapsed_ms=17000 source=DO_NOT_COPY",
    "[2000] externalIndex gameData ready cache_status=loaded cache_detail=<none> files=6495 symbols=143145 cache_file_bytes=29360128 cache_total_ms=800 elapsed_ms=800 source=DO_NOT_COPY",
    "[3000] externalIndex gameData ready cache_status=loaded cache_detail=migrated-v10 files=6495 symbols=143145 cache_file_bytes=29360128 cache_total_ms=1200 elapsed_ms=1200 source=DO_NOT_COPY",
  ].join("\n"));

  assert.match(report, /## External Game-Data Cache/);
  assert.match(report, /rebuilt.*6495.*143145.*28\.0 MiB.*17000 ms/);
  assert.match(report, /loaded.*6495.*143145.*28\.0 MiB.*800 ms/);
  assert.doesNotMatch(report, /DO_NOT_COPY/);
});

test("reports rich semantic-token phase timings without source payloads", () => {
  const report = runReport(
    "[1000] semanticTokensRich ready uri=file:///workspace/GC_MarkerArea.c revision=42 external_generation=9 resolver_ms=31 resolver_calls=96 type_detail_ms=1 declaration_symbols_ms=0 delimiter_ms=140 delimiter_resolver_calls=601 source=DO_NOT_COPY elapsed_ms=182",
  );

  assert.match(report, /resolver_ms=31/);
  assert.match(report, /resolver_calls=96/);
  assert.match(report, /type_detail_ms=1/);
  assert.match(report, /declaration_symbols_ms=0/);
  assert.match(report, /delimiter_ms=140/);
  assert.match(report, /delimiter_resolver_calls=601/);
  assert.doesNotMatch(report, /DO_NOT_COPY/);
});

test("reports rich projections reused across identical self-save updates", () => {
  const report = runReport([
    "[995] semanticTokensRich ready uri=file:///workspace/GC_MarkerArea.c revision=42 external_generation=8 workspace_excludes_document=true elapsed_ms=182",
    "[1000] semanticTokens self-save reused uri=file:///workspace/GC_MarkerArea.c revision=42 previous_external_generation=8 external_generation=9 state=ready reference_elapsed_ms=182",
    "[1000] semanticTokens external overlay changed generation=9 status=ready documents=2 preserved_self_save=1 requesting_refresh=true",
    "[1005] semanticTokens self-save retargeted uri=file:///workspace/GC_Sounds.c revision=7 previous_external_generation=9 external_generation=10 state=pending",
    "[1010] semanticTokensRich ready uri=file:///workspace/GC_Sounds.c revision=7 external_generation=10 task_external_generation=9 workspace_excludes_document=true elapsed_ms=175",
    "[1010] semanticTokens self-save reused uri=file:///workspace/GC_Sounds.c revision=7 previous_external_generation=9 external_generation=10 state=completed reference_elapsed_ms=175",
    "[1020] semanticTokens self-save retargeted uri=file:///workspace/Cancelled.c revision=3 previous_external_generation=10 external_generation=11 state=pending",
    "[1030] semanticTokens self-save reused uri=file:///workspace/Uncorrelated.c revision=4 previous_external_generation=11 external_generation=12 state=completed reference_elapsed_ms=999",
  ].join("\n"));

  assert.match(report, /Self-save rich projections reused: 2/);
  assert.match(report, /Self-save in-flight projections retargeted: 2/);
  assert.match(report, /Reused rich elapsed-time reference: 357 ms/);
  assert.match(report, /## Self-Save Rich Projection Reuse/);
  assert.match(report, /GC_MarkerArea\.c.*42.*8.*9.*ready.*182 ms/);
  assert.match(report, /GC_Sounds\.c.*7.*9.*10.*pending.*Pending/);
  assert.match(report, /GC_Sounds\.c.*7.*9.*10.*completed.*175 ms/);
});

test("correlates first current-snapshot token and completion responses without payload data", () => {
  const report = runReport([
    "[1000] notification didChange uri=file:///workspace/GC_MarkerArea.c version=7 revision=42 cached_analysis=false analysis_state=pending elapsed_ms=2",
    "[1015] request completion uri=file:///workspace/GC_MarkerArea.c revision=42 context=top-level candidates=3 elapsed_ms=5",
    "[1020] request semanticTokens uri=file:///workspace/GC_MarkerArea.c revision=42 mode=lexical-pending tokens=3 elapsed_ms=2",
    "[1040] documentAnalysis ready uri=file:///workspace/GC_MarkerArea.c revision=42 elapsed_ms=40",
  ].join("\n"));
  assert.match(report, /Usable tokens.*1.*0.*20 ms.*20 ms.*1/);
  assert.match(report, /Completion.*1.*0.*15 ms.*15 ms/);
  assert.match(report, /Accepted snapshots.*1/);
  assert.match(report, /Current foreground responses.*2/);
  assert.match(report, /Matching semantic publications.*1/);
});

test("keeps unpaired revisions distinct and parses explicit admission plus cancellation-tail telemetry", () => {
  const report = runReport([
    "[1000] notification didChange uri=file:///workspace/GC_MarkerArea.c version=7 revision=42 elapsed_ms=2",
    "[1010] request semanticTokens uri=file:///workspace/GC_MarkerArea.c revision=41 mode=lexical-pending elapsed_ms=2",
    "[1020] analysisRuntime admission uri=file:///workspace/GC_MarkerArea.c revision=42 disposition=admitted lane=foreground",
    "[1030] analysisRuntime admission uri=file:///workspace/GC_MarkerArea.c revision=42 disposition=overloaded lane=rich",
    "[1040] semanticTokensRich discarded uri=file:///workspace/GC_MarkerArea.c revision=41 reason=cancelled-superseded cancellation_tail_ms=12 elapsed_ms=80",
  ].join("\n"));
  assert.match(report, /Unpaired foreground responses.*1/);
  assert.match(report, /Admitted.*1/);
  assert.match(report, /Overloaded\/rejected\/dropped.*1/);
  assert.match(report, /Cancelled terminal records: 1/);
  assert.match(report, /Measured tails: 1/);
  assert.match(report, /Cancellation tail p95: 12 ms/);
});

test("reports declared query quality, admission lanes, and rich identity without exposing payload data", () => {
  const report = runReport([
    "[1000] notification didChange uri=file:///workspace/GC_MarkerArea.c version=7 revision=42 elapsed_ms=2",
    "[1010] request completion uri=file:///workspace/GC_MarkerArea.c revision=42 query_quality=Exact prefix=DO_NOT_COPY elapsed_ms=5",
    "[1020] request hover uri=file:///workspace/GC_MarkerArea.c revision=42 query_quality=Unavailable label=DO_NOT_COPY elapsed_ms=5",
    "[1030] analysisRuntime admission uri=file:///workspace/GC_MarkerArea.c revision=42 disposition=admitted lane=foreground",
    "[1040] semanticTokensRich discarded uri=file:///workspace/GC_MarkerArea.c revision=42 external_generation=9 reason=cancelled-superseded cancellation_tail_ms=12 elapsed_ms=8",
  ].join("\n"));

  assert.match(report, /## Foreground Query Quality/);
  assert.match(report, /completion.*1.*1.*0.*0.*0.*0/);
  assert.match(report, /hover.*1.*0.*0.*1.*0.*0/);
  assert.match(report, /foreground.*1.*0/);
  assert.match(report, /Rich document identity.*1.*0/);
  assert.match(report, /Rich external-generation identity.*1.*0/);
  assert.match(report, /Cancelled rich tail.*1.*0/);
  assert.doesNotMatch(report, /DO_NOT_COPY/);
});

test("flags missing capture markers instead of defaulting them", () => {
  const report = runReport([
    "[1000] notification didChange uri=file:///workspace/GC_MarkerArea.c revision=42 elapsed_ms=2",
    "[1010] request completion uri=file:///workspace/GC_MarkerArea.c revision=42 elapsed_ms=5",
    "[1020] analysisRuntime admission uri=file:///workspace/GC_MarkerArea.c revision=42 lane=foreground",
    "[1030] semanticTokensRich discarded uri=file:///workspace/GC_MarkerArea.c revision=42 reason=cancelled-superseded elapsed_ms=8",
  ].join("\n"));

  assert.match(report, /## Capture Field Completeness/);
  assert.match(report, /Accepted snapshots.*1.*1.*Incomplete/);
  assert.match(report, /Query-quality feature responses.*1.*1.*Incomplete/);
  assert.match(report, /Runtime admission.*1.*1.*Incomplete/);
  assert.match(report, /Rich semantic-token terminals.*1.*1.*Incomplete/);
  assert.match(report, /Cancelled rich terminals.*1.*2.*Incomplete/);
});
