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

test("does not reproduce source or completion payload fields in the report", () => {
  const report = runReport([
    "[1000] notification didChange uri=file:///workspace/GC_MarkerArea.c version=7 revision=42 text=DO_NOT_COPY analysis_elapsed_ms=120",
    "[1010] request completion uri=file:///workspace/GC_MarkerArea.c revision=42 prefix=DO_NOT_COPY completion_items=DO_NOT_COPY elapsed_ms=35",
  ].join("\n"));

  assert.doesNotMatch(report, /DO_NOT_COPY/);
});
