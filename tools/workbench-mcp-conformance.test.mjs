import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import test from "node:test";
import { join } from "node:path";
import {
  buildLiveCoverageReport,
  buildContractReport,
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
          response: { result: { isError: true } },
          timing: { durationMs: 3, requestBytes: 40, responseBytes: 96 },
        };
      },
    },
    name: "expected unavailable Workbench",
    steps: [
      {
        name: "read unavailable status",
        tool: "workbench_status",
        expect: { isError: true },
      },
    ],
  });

  assert.equal(report.ok, true);
  assert.equal(report.steps[0].outcome, "expected-error");
  assert.equal(report.performance[0].failureCount, 0);
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
    [{ steps: [{ tool: "workbench_status" }] }],
  );

  assert.deepEqual(report, {
    ok: false,
    expectedCount: 2,
    coveredCount: 1,
    successfulCount: 0,
    expectedErrorCount: 0,
    missing: ["workbench_create_entity"],
    unexpected: [],
  });
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
