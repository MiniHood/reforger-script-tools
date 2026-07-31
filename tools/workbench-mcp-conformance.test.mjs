import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import test from "node:test";
import { join } from "node:path";
import {
  buildLiveCoverageReport,
  buildContractReport,
  classifyTool,
  loadFixtureManifest,
  runScenario,
  summarizePerformance,
  summarizeSamples,
} from "./workbench-mcp-conformance.mjs";

const reference = [
  "| [`workbench_status`](mcp-api/tools/workbench_status.md) | — | `isRunning` | Check status. |",
  "| [`workbench_open_resource`](mcp-api/tools/workbench_open_resource.md) | `resourcePath` | `opened` | Open resource. |",
  "| [`workbench_create_entity`](mcp-api/tools/workbench_create_entity.md) | `position` | `entity` | Create entity. |",
].join("\n");

function tool(name, overrides = {}) {
  return {
    name,
    description: "A Workbench capability.",
    annotations: {
      readOnlyHint: true,
      openWorldHint: false,
    },
    inputSchema: {
      type: "object",
      additionalProperties: false,
    },
    outputSchema: {
      type: "object",
    },
    ...overrides,
  };
}

test("reports complete Workbench MCP tool contract coverage", () => {
  const report = buildContractReport({
    apiReference: reference,
    listedTools: [
      tool("workbench_status"),
      tool("workbench_status"),
      tool("workbench_open_resource", { outputSchema: undefined }),
      tool("workbench_unexpected"),
    ],
  });

  assert.deepEqual(report.expectedNames, [
    "workbench_status",
    "workbench_open_resource",
    "workbench_create_entity",
  ]);
  assert.deepEqual(report.missing, ["workbench_create_entity"]);
  assert.deepEqual(report.unexpected, ["workbench_unexpected"]);
  assert.deepEqual(report.duplicates, ["workbench_status"]);
  assert.deepEqual(report.invalid, [
    {
      name: "workbench_open_resource",
      reasons: ["missing outputSchema"],
    },
  ]);
  assert.equal(report.coverage[0].family, "status");
  assert.equal(report.ok, false);
});

test("accepts a complete generated catalogue and tools/list result", () => {
  const report = buildContractReport({
    apiReference: reference,
    listedTools: [
      tool("workbench_status"),
      tool("workbench_open_resource"),
      tool("workbench_create_entity"),
    ],
  });

  assert.equal(report.ok, true);
  assert.deepEqual(report.missing, []);
  assert.deepEqual(report.unexpected, []);
  assert.deepEqual(report.duplicates, []);
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.uncategorized, []);
});

test("classifies the parameterless save operation", () => {
  assert.equal(classifyTool("workbench_save"), "save");
});

test("records public MCP scenario observations and verifies returned state", async () => {
  const calls = [];
  const client = {
    async callToolTimed(name, argumentsValue) {
      calls.push({ name, argumentsValue });
      return {
        response: {
          result: {
            isError: false,
            structuredContent: {
              status: "available",
            },
          },
        },
        timing: {
          durationMs: 7,
          requestBytes: 88,
          responseBytes: 144,
        },
      };
    },
  };

  const report = await runScenario({
    client,
    name: "status smoke",
    steps: [
      {
        name: "read status",
        tool: "workbench_status",
        arguments: {},
        expect: {
          isError: false,
          pointers: {
            "/result/structuredContent/status": "available",
          },
        },
      },
    ],
  });

  assert.deepEqual(calls, [
    { name: "workbench_status", argumentsValue: {} },
  ]);
  assert.equal(report.ok, true);
  assert.deepEqual(report.steps, [
    {
      name: "read status",
      tool: "workbench_status",
      outcome: "success",
      durationMs: 7,
      requestBytes: 88,
      responseBytes: 144,
    },
  ]);
});

test("treats an expected structured Workbench error as tested behavior", async () => {
  const report = await runScenario({
    client: {
      async callToolTimed() {
        return {
          response: {
            result: {
              isError: true,
              structuredContent: {
                code: "workbench_unavailable",
                phase: "status",
              },
            },
          },
          timing: { durationMs: 3, requestBytes: 40, responseBytes: 96 },
        };
      },
    },
    name: "expected unavailable Workbench",
    steps: [
      {
        name: "read unavailable status",
        tool: "workbench_status",
        expect: {
          isError: true,
          error: { code: "workbench_unavailable", phase: "status" },
        },
      },
    ],
  });

  assert.equal(report.ok, true);
  assert.equal(report.steps[0].outcome, "expected-error");
  assert.equal(report.performance[0].failureCount, 0);
});

test("allows explicitly environment-dependent capabilities to return either result", async () => {
  const report = await runScenario({
    client: {
      async callToolTimed() {
        return {
          response: {
            result: {
              isError: true,
              structuredContent: {
                code: "workbench_capture_unavailable",
                phase: "capture_window",
              },
            },
          },
          timing: { durationMs: 3, requestBytes: 40, responseBytes: 96 },
        };
      },
    },
    name: "optional window capture",
    steps: [
      {
        name: "capture when visible",
        tool: "workbench_capture_window",
        expect: {
          allowError: true,
          error: {
            code: "workbench_capture_unavailable",
            phase: "capture_window",
          },
        },
      },
    ],
  });

  assert.equal(report.ok, true);
  assert.equal(report.steps[0].outcome, "expected-error");
});

test("rejects an unscoped optional error", async () => {
  const report = await runScenario({
    client: {
      async callToolTimed() {
        return {
          response: {
            result: {
              isError: true,
              structuredContent: {
                code: "workbench_timeout",
                phase: "reload",
              },
            },
          },
          timing: { durationMs: 3, requestBytes: 40, responseBytes: 96 },
        };
      },
    },
    name: "unscoped optional error",
    steps: [
      {
        name: "reload",
        tool: "workbench_reload",
        expect: { allowError: true },
      },
    ],
  });

  assert.equal(report.ok, false);
  assert.match(report.steps[0].reasons[0], /explicit structured error oracle/);
});

test("materializes only explicit fixture references for lifecycle arguments and oracles", async () => {
  const calls = [];
  const report = await runScenario({
    client: {
      async callToolTimed(name, argumentsValue) {
        calls.push({ name, argumentsValue });
        return {
          response: {
            result: {
              isError: false,
              structuredContent: { activeWorldPath: "Fixture/World.ent" },
            },
          },
          timing: { durationMs: 2, requestBytes: 10, responseBytes: 20 },
        };
      },
    },
    context: { fixture: { processId: 77, worldResource: "Fixture/World.ent" } },
    name: "fixture references",
    steps: [
      {
        name: "observe world",
        tool: "workbench_state",
        arguments: { processId: "$fixture.processId" },
        expect: {
          pointers: {
            "/result/structuredContent/activeWorldPath": "$fixture.worldResource",
          },
        },
      },
    ],
  });

  assert.equal(report.ok, true);
  assert.deepEqual(calls, [
    { name: "workbench_state", argumentsValue: { processId: 77 } },
  ]);
});

test("summarizes latency distributions while retaining failed samples", () => {
  assert.deepEqual(summarizeSamples([9, 1, 7, 3, 5]), {
    count: 5,
    minimumMs: 1,
    maximumMs: 9,
    p50Ms: 5,
    p95Ms: 9,
    p99Ms: 9,
  });
  assert.deepEqual(
    summarizePerformance([
      {
        name: "timing",
        steps: [
          { tool: "workbench_status", durationMs: 4, outcome: "success" },
          { tool: "workbench_status", durationMs: 8, outcome: "failure" },
        ],
      },
    ]),
    [
      {
        tool: "workbench_status",
        count: 2,
        minimumMs: 4,
        maximumMs: 8,
        p50Ms: 4,
        p95Ms: 8,
        p99Ms: 8,
        requestBytes: { minimum: null, maximum: null },
        responseBytes: { minimum: null, maximum: null },
        successCount: 1,
        failureCount: 1,
      },
    ],
  );
});

test("reports the exact published tools still missing live evidence", () => {
  const report = buildLiveCoverageReport(
    { expectedNames: ["workbench_status", "workbench_create_entity"] },
    [{ steps: [{ tool: "workbench_status", outcome: "success" }] }],
  );

  assert.deepEqual(report, {
    ok: false,
    expectedCount: 2,
    coveredCount: 1,
    successfulCount: 1,
    expectedErrorCount: 0,
    expectedUnavailableCount: 0,
    incomplete: [],
    complete: true,
    missing: ["workbench_create_entity"],
    unexpected: [],
    failed: [],
  });
});

test("does not count a failed step as live evidence", () => {
  const report = buildLiveCoverageReport(
    { expectedNames: ["workbench_status"] },
    [{ steps: [{ tool: "workbench_status", outcome: "failure" }] }],
  );

  assert.equal(report.ok, false);
  assert.deepEqual(report.missing, ["workbench_status"]);
  assert.deepEqual(report.failed, ["workbench_status"]);
});

test("reports covered but incomplete capabilities separately", () => {
  const report = buildLiveCoverageReport(
    { expectedNames: ["workbench_reload"] },
    [
      {
        steps: [
          {
            tool: "workbench_reload",
            outcome: "expected-unavailable",
            completion: false,
          },
        ],
      },
    ],
  );

  assert.equal(report.ok, true);
  assert.equal(report.complete, false);
  assert.deepEqual(report.incomplete, ["workbench_reload"]);
  assert.equal(report.expectedUnavailableCount, 1);
});

test("loads a disposable fixture manifest with an isolated profile and world", () => {
  const root = mkdtempSync(join(process.cwd(), ".cache", "workbench-fixture-test-"));
  try {
    mkdirSync(join(root, "project"), { recursive: true });
    mkdirSync(join(root, "addons"), { recursive: true });
    writeFileSync(join(root, "project", "fixture.gproj"), "{}\n");
    writeFileSync(
      join(root, "fixture.manifest.json"),
      JSON.stringify({
        name: "fixture",
        revision: "test",
        fixtureRoot: ".",
        profileRoot: "profile",
        project: { gproj: "project/fixture.gproj", addonsDir: "addons" },
        expected: {
          worldResource: "Fixture/World.ent",
          loadedAddonIds: ["ArmaReforger"],
        },
      }),
    );

    const manifest = loadFixtureManifest(join(root, "fixture.manifest.json"));
    assert.equal(manifest.name, "fixture");
    assert.equal(manifest.expected.worldResource, "Fixture/World.ent");
    assert.equal(manifest.profileRoot, join(root, "profile"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
