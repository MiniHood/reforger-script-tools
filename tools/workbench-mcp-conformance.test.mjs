import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import test from "node:test";
import { join } from "node:path";
import {
  buildWorkbenchEndpointPlan,
  buildWorkbenchCorpusReport,
  buildContractReport,
  classifyTool,
  groupWorkbenchScenarioSteps,
  validateWorkbenchEndpointPlan,
  loadFixtureManifest,
  runScenario,
  runWorkbenchWorkflows,
  summarizePerformance,
  summarizeSamples,
  verifyOwnedStopRestartLifecycle,
  waitForWorkbenchEditors,
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

test("requires history readback and inverse cleanup evidence", () => {
  const plan = buildWorkbenchEndpointPlan(["workbench_undo", "workbench_redo"]);
  const undo = plan.find((entry) => entry.tool === "workbench_undo");
  const redo = plan.find((entry) => entry.tool === "workbench_redo");
  assert.deepEqual(undo.cases[0].readbackTools, ["workbench_inspect_entity"]);
  assert.deepEqual(undo.cases[0].cleanupTools, ["workbench_redo"]);
  assert.deepEqual(redo.cases[0].readbackTools, ["workbench_inspect_entity"]);
  assert.deepEqual(redo.cases[0].cleanupTools, ["workbench_undo"]);
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

test("compares structured pointer oracles by value and infers opaque resource facts", async () => {
  const report = await runScenario({
    client: {
      async callToolTimed() {
        return {
          response: {
            result: {
              isError: false,
              structuredContent: {
                components: [],
                results: [{ entity: { entityId: "shape-1" }, resourceName: "{GUID}Prefabs/Test.et" }],
              },
            },
          },
          timing: { durationMs: 1, requestBytes: 10, responseBytes: 20 },
        };
      },
    },
    name: "structured oracle",
    steps: [{
      name: "discover shape",
      tool: "workbench_search_world_entities",
      capture: {
        shapeEntityId: "/result/structuredContent/results/0/entity/entityId",
        windowId: "/result/structuredContent/results/0/entity/entityId",
        genericPrefabResourceName: "/result/structuredContent/results/0/resourceName",
      },
      expect: {
        pointers: {
          "/result/structuredContent/components": [],
        },
      },
    }],
    includeInvocationMetadata: true,
  });

  assert.equal(report.ok, true);
  assert.deepEqual(report.steps[0].facts, ["entity", "shape", "window", "prefabResource", "canonicalResource"]);
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

test("materializes explicit references embedded in disposable names", async () => {
  let received;
  await runScenario({
    client: {
      async callToolTimed(_name, argumentsValue) {
        received = argumentsValue;
        return {
          response: { result: { isError: false, structuredContent: { status: "ok" } } },
          timing: { durationMs: 1, requestBytes: 1, responseBytes: 1 },
        };
      },
    },
    context: { fixture: { entityName: "entity-123" } },
    name: "embedded references",
    steps: [{
      name: "create",
      tool: "workbench_create_entity",
      arguments: { name: "$fixture.entityName-Renamed" },
    }],
  });
  assert.deepEqual(received, { name: "entity-123-Renamed" });
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

test("requires the executable endpoint plan to match the published catalogue exactly", () => {
  const plan = buildWorkbenchEndpointPlan([
    "workbench_status",
    "workbench_create_entity",
  ]);

  assert.deepEqual(validateWorkbenchEndpointPlan(
    ["workbench_status", "workbench_create_entity"],
    plan,
  ), {
    ok: true,
    missing: [],
    unexpected: [],
    duplicates: [],
    invalid: [],
  });

  assert.equal(
    validateWorkbenchEndpointPlan(
      ["workbench_status"],
      [...plan, plan[1]],
    ).ok,
    false,
  );
});

test("aggregates required cases into passed, failed, blocked, and not-run statuses", () => {
  const plan = [
    {
      tool: "workbench_status",
      workflow: "readiness",
      dependencies: [],
      cases: [{ id: "success", kind: "success" }],
    },
    {
      tool: "workbench_reload",
      workflow: "reload",
      dependencies: ["managedBridge"],
      cases: [
        { id: "success", kind: "success" },
        { id: "guard", kind: "guard" },
      ],
    },
    {
      tool: "workbench_save",
      workflow: "save",
      dependencies: [],
      cases: [{ id: "success", kind: "success" }],
    },
    {
      tool: "workbench_stop",
      workflow: "lifecycle",
      dependencies: ["replacementProcess"],
      cases: [{ id: "success", kind: "success" }],
    },
  ];

  const report = buildWorkbenchCorpusReport(plan, [
    {
      name: "corpus",
      steps: [
        { tool: "workbench_status", case: "success", role: "test", outcome: "success" },
        { tool: "workbench_reload", case: "success", role: "test", outcome: "expected-unavailable", blockedBy: ["managedBridge"] },
        { tool: "workbench_reload", case: "guard", role: "test", outcome: "expected-error" },
        { tool: "workbench_save", case: "success", role: "test", outcome: "failure", reasons: ["save failed"] },
      ],
    },
  ]);

  assert.deepEqual(report.counts, {
    passed: 1,
    failed: 1,
    blocked: 1,
    "not-run": 1,
  });
  assert.deepEqual(
    report.endpoints.map(({ tool, status }) => ({ tool, status })),
    [
      { tool: "workbench_status", status: "passed" },
      { tool: "workbench_reload", status: "blocked" },
      { tool: "workbench_save", status: "failed" },
      { tool: "workbench_stop", status: "not-run" },
    ],
  );
});

test("preserves invocation role, case, and fact evidence in corpus observations", async () => {
  const report = await runScenario({
    client: {
      async callToolTimed() {
        return {
          response: {
            result: {
              isError: false,
              structuredContent: { status: "created", entityId: "entity-1" },
            },
          },
          timing: { durationMs: 1, requestBytes: 10, responseBytes: 20 },
        };
      },
    },
    name: "metadata",
    steps: [{
      name: "create",
      tool: "workbench_create_entity",
      role: "setup",
      case: "success",
      facts: ["entity"],
      expect: { pointers: { "/result/structuredContent/status": "created" } },
    }],
  });

  assert.deepEqual(report.steps[0], {
    name: "create",
    tool: "workbench_create_entity",
    role: "setup",
    roles: ["setup"],
    case: "success",
    facts: ["entity"],
    outcome: "success",
    durationMs: 1,
    requestBytes: 10,
    responseBytes: 20,
    serves: [],
    arguments: {},
    captures: {},
    target: null,
  });
});

test("keeps inspection endpoints as tests while marking mutation-serving reads", async () => {
  const report = await runScenario({
    client: {
      async callToolTimed() {
        return {
          response: { result: { isError: false, structuredContent: { status: "available" } } },
          timing: { durationMs: 1, requestBytes: 1, responseBytes: 1 },
        };
      },
    },
    name: "inspection",
    steps: [{
      name: "inspect-created-entity",
      tool: "workbench_inspect_entity",
      expect: { pointers: { "/result/structuredContent/status": "available" } },
    }],
    includeInvocationMetadata: true,
  });

  assert.equal(report.steps[0].role, "test");
  assert.deepEqual(report.steps[0].roles, ["test", "readback"]);
  assert.deepEqual(report.steps[0].serves, ["workbench_create_entity"]);
});

test("records a successful structured unavailable response as blocked evidence", async () => {
  const report = await runScenario({
    client: {
      async callToolTimed() {
        return {
          response: {
            result: {
              isError: false,
              structuredContent: { status: "prefab-edit-unavailable" },
            },
          },
          timing: { durationMs: 1, requestBytes: 10, responseBytes: 20 },
        };
      },
    },
    name: "blocked structured result",
    steps: [{
      name: "outside-edit guard",
      tool: "workbench_set_prefab_property",
      expect: { completion: false },
    }],
  });

  assert.equal(report.steps[0].outcome, "expected-unavailable");
  assert.equal(report.steps[0].completion, false);
});

test("allows a later successful case observation to supersede an earlier unavailable probe", () => {
  const report = buildWorkbenchCorpusReport([{
    tool: "workbench_selected_entity_hierarchy",
    workflow: "entity",
    dependencies: [],
    cases: [{ id: "success", kind: "success" }],
  }], [{
    steps: [
      { tool: "workbench_selected_entity_hierarchy", case: "success", role: "test", outcome: "expected-unavailable" },
      { tool: "workbench_selected_entity_hierarchy", case: "success", role: "test", outcome: "success" },
    ],
  }]);

  assert.equal(report.endpoints[0].status, "passed");
});

test("does not approve a case from a readback-role invocation alone", () => {
  const report = buildWorkbenchCorpusReport([
    {
      tool: "workbench_create_entity",
      workflow: "entity",
      dependencies: [],
      cases: [{ id: "success", kind: "success" }],
    },
  ], [{
    name: "readback-only",
    steps: [{
      tool: "workbench_create_entity",
      case: "success",
      role: "readback",
      outcome: "success",
    }],
  }]);

  assert.equal(report.ok, false);
  assert.equal(report.endpoints[0].status, "not-run");
});

test("orders scenario calls into named dependency workflows", () => {
  const groups = groupWorkbenchScenarioSteps([
    { name: "save", tool: "workbench_save" },
    { name: "create", tool: "workbench_create_entity" },
    { name: "status", tool: "workbench_status" },
  ], [
    { tool: "workbench_save", workflow: "save-play-reload" },
    { tool: "workbench_create_entity", workflow: "entity" },
    { tool: "workbench_status", workflow: "owned-process" },
  ]);

  assert.deepEqual(groups.map((group) => group.name), [
    "owned-process",
    "entity",
    "save-play-reload",
  ]);
  assert.deepEqual(groups[0].steps.map((step) => step.name), ["status"]);
});

test("keeps prefab guard cases runnable without a prefab-edit dependency", () => {
  const plan = buildWorkbenchEndpointPlan([
    "workbench_set_prefab_property",
    "workbench_set_prefab_component_property",
  ]);

  for (const entry of plan) {
    assert.deepEqual(
      entry.cases.find((acceptanceCase) => acceptanceCase.id === "success")?.dependencies,
      ["prefabEditEntity"],
    );
    assert.deepEqual(
      entry.cases.find((acceptanceCase) => acceptanceCase.id === "outside-edit-guard")?.dependencies,
      [],
    );
  }
});

test("gates workflow calls on captured facts and preserves the blocked branch", async () => {
  const calls = [];
  const result = await runWorkbenchWorkflows({
    client: {
      async callToolTimed(toolName) {
        calls.push(toolName);
        return {
          response: { result: { isError: false, structuredContent: { status: "ok" } } },
          timing: { durationMs: 1, requestBytes: 1, responseBytes: 1 },
        };
      },
    },
    steps: [
      { name: "needs-entity", tool: "workbench_inspect_entity", arguments: {} },
      { name: "make-entity", tool: "workbench_create_entity", arguments: {}, facts: ["entity"] },
    ],
    plan: [
      { tool: "workbench_inspect_entity", workflow: "entity", requiredFacts: ["entity"], dependencies: ["entity"] },
      { tool: "workbench_create_entity", workflow: "entity", requiredFacts: [], dependencies: [] },
    ],
  });

  assert.deepEqual(calls, ["workbench_create_entity"]);
  assert.equal(result.runs[0].steps[0].synthetic, true);
  assert.deepEqual(result.runs[0].steps[0].blockedBy, ["entity"]);
});

test("requires declared readback and cleanup evidence before passing a mutation case", () => {
  const plan = [{
    tool: "workbench_create_entity",
    workflow: "entity",
    dependencies: [],
    cases: [{
      id: "success",
      kind: "success",
      readbackTools: ["workbench_inspect_entity"],
      cleanupTools: ["workbench_delete_entity"],
    }],
  }];
  const incomplete = buildWorkbenchCorpusReport(plan, [{
    steps: [{
      tool: "workbench_create_entity",
      case: "success",
      role: "test",
      outcome: "success",
    }],
  }]);
  assert.equal(incomplete.endpoints[0].status, "failed");
  assert.deepEqual(incomplete.endpoints[0].blockers, [
    "readback:workbench_inspect_entity",
    "cleanup:workbench_delete_entity",
  ]);

  const complete = buildWorkbenchCorpusReport(plan, [{
    steps: [
      {
        tool: "workbench_create_entity",
        case: "success",
        role: "test",
        outcome: "success",
      },
      {
        tool: "workbench_inspect_entity",
        role: "readback",
        serves: ["workbench_create_entity"],
        outcome: "success",
      },
      {
        tool: "workbench_delete_entity",
        role: "teardown",
        outcome: "success",
      },
    ],
  }]);
  assert.equal(complete.endpoints[0].status, "passed");
});

test("does not infer unrelated facts from a successful producer tool", () => {
  const report = buildWorkbenchCorpusReport([
    {
      tool: "workbench_create_entity",
      workflow: "entity",
      dependencies: [],
      cases: [{ id: "success", kind: "success" }],
    },
    {
      tool: "workbench_get_shape_points",
      workflow: "shape",
      dependencies: ["shape"],
      cases: [{ id: "success", kind: "success" }],
    },
  ], [{
    steps: [{
      tool: "workbench_create_entity",
      case: "success",
      role: "test",
      outcome: "success",
      facts: ["entity"],
    }],
  }]);

  assert.equal(report.endpoints[1].status, "blocked");
});

test("waits for the editor catalogue after Workbench NET readiness", async () => {
  let attempts = 0;
  const result = await waitForWorkbenchEditors(
    {
      async callTool() {
        attempts += 1;
        if (attempts === 1) {
          return {
            result: {
              isError: true,
              structuredContent: { code: "workbench_error", phase: "list_editors" },
            },
          };
        }
        return {
          result: {
            isError: false,
            structuredContent: { editors: [{ id: "world", displayName: "World Editor" }] },
          },
        };
      },
    },
    { timeoutMs: 100, intervalMs: 0 },
  );

  assert.equal(attempts, 2);
  assert.deepEqual(result.editors, [{ id: "world", displayName: "World Editor" }]);
});

test("verifies restart and stop against the owned replacement process", async () => {
  const calls = [];
  const session = { ownsProcess: true, processId: 41 };
  const client = {
    async callTool(name, argumentsValue) {
      calls.push({ name, argumentsValue });
      if (name === "workbench_restart") {
        return {
          result: {
            isError: false,
            structuredContent: {
              processId: 42,
              alreadyRunning: false,
              exited: false,
              netApiConnected: true,
              userInteractionRequired: false,
            },
          },
        };
      }
      return {
        result: {
          isError: false,
          structuredContent: { processId: 42, exited: true },
        },
      };
    },
  };

  const result = await verifyOwnedStopRestartLifecycle({ client, session });

  assert.deepEqual(calls, [
    { name: "workbench_restart", argumentsValue: { processId: 41 } },
    { name: "workbench_stop", argumentsValue: { processId: 42 } },
  ]);
  assert.equal(result.originalProcessId, 41);
  assert.equal(result.restartedProcessId, 42);
  assert.equal(session.processId, null);
  assert.equal(session.ownsProcess, false);
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
        useProfile: false,
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
    assert.equal(manifest.useProfile, false);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
