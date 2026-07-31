import { createInterface } from "node:readline";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { spawn } from "node:child_process";
import { arch, platform, release } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const defaultApiReference = join(repositoryRoot, "docs", "mcp-api.md");
const defaultReportPath = join(
  repositoryRoot,
  ".cache",
  "reports",
  "workbench-mcp-contract.json",
);
const defaultServerCandidates = [
  join(repositoryRoot, "server", "target", "debug", "reforger_language_server.exe"),
  join(repositoryRoot, "server", "target", "release", "reforger_language_server.exe"),
  join(repositoryRoot, "dist", "server", "win32-x64", "reforger_language_server.exe"),
];

const workbenchRouterRow =
  /^\| \[\x60(workbench_[^\x60]+)\x60\]\(mcp-api\/tools\/(workbench_[^)]+)\.md\) \|/;

const toolFamilyRules = [
  ["lifecycle", /^(launch|stop|restart)$/],
  ["maintenance", /^(install_bridge|reload|read_logs)$/],
  ["status", /^(status|state|project_context)$/],
  ["validation", /^validate_scripts$/],
  ["resource", /^(inspect_resource|list_resources|search_resources|open_resource)$/],
  ["editor", /^(list_editors|open_editor)$/],
  [
    "world-read",
    /^(world_selection_summary|selected_entity_hierarchy|list_entities|search_world_entities|layer_state|find_entities_by_radius|sample_terrain|get_viewport_context|trace|inspect_prefab_context|inspect_prefab_component|inspect_entity|list_components|inspect_component|list_entity_properties|get_shape_points)$/,
  ],
  ["prefab-write", /^(create_(generic_)?|save_)prefab$|^(add|remove|set)_prefab_/],
  [
    "entity-write",
    /^(set|clear)_selection$|^(create|rename|delete|move|rotate|reparent|duplicate)_entity$|^(add|set|remove)_(component|entity)/,
  ],
  ["shape-write", /^(edit_shape_points|set_polyline_regular_polygon|convert_shape_points|transform_shape_points|resample_polyline)$/],
  ["play-session", /^(start|stop)_play_session$/],
  ["save", /^save$/],
  ["window", /^(list_windows|capture_window)$/],
];

export function extractWorkbenchToolNames(apiReference) {
  const names = [];
  for (const line of apiReference.split(/\r?\n/)) {
    const match = workbenchRouterRow.exec(line);
    if (match && match[1] === match[2]) {
      names.push(match[1]);
    }
  }
  return names;
}

export function buildContractReport({ apiReference, listedTools }) {
  const expectedNames = extractWorkbenchToolNames(apiReference);
  const workbenchTools = listedTools.filter(
    (tool) => typeof tool?.name === "string" && tool.name.startsWith("workbench_"),
  );
  const actualNames = workbenchTools.map((tool) => tool.name);
  const expected = new Set(expectedNames);
  const actual = new Set(actualNames);
  const missing = expectedNames.filter((name) => !actual.has(name));
  const unexpected = actualNames
    .filter((name) => !expected.has(name))
    .filter((name, index, names) => names.indexOf(name) === index);
  const duplicates = actualNames.filter(
    (name, index, names) => names.indexOf(name) !== index,
  );
  const invalid = workbenchTools
    .map((tool) => {
      const reasons = [];
      if (typeof tool.description !== "string" || tool.description.length === 0) {
        reasons.push("missing description");
      }
      if (!isObject(tool.annotations)) {
        reasons.push("missing annotations");
      }
      if (!isObject(tool.inputSchema)) {
        reasons.push("missing inputSchema");
      }
      if (!isObject(tool.outputSchema)) {
        reasons.push("missing outputSchema");
      }
      return reasons.length === 0 ? undefined : { name: tool.name, reasons };
    })
    .filter(Boolean);

  const coverage = expectedNames.map((name) => ({
    tool: name,
    family: classifyTool(name),
    contractEvidence: actual.has(name) && !invalid.some((item) => item.name === name)
      ? "tools/list"
      : "missing",
    liveEvidence: "not-run",
  }));
  const uncategorized = coverage
    .filter((entry) => entry.family === "uncategorized")
    .map((entry) => entry.tool);

  return {
    ok:
      missing.length === 0 &&
      unexpected.length === 0 &&
      duplicates.length === 0 &&
      invalid.length === 0 &&
      uncategorized.length === 0,
    expectedNames,
    actualNames,
    missing,
    unexpected,
    duplicates: [...new Set(duplicates)],
    invalid,
    expectedCount: expectedNames.length,
    actualCount: actualNames.length,
    coverage,
    uncategorized,
  };
}

export function classifyTool(name) {
  const suffix = name.startsWith("workbench_")
    ? name.slice("workbench_".length)
    : name;
  const match = toolFamilyRules.find(([, rule]) => rule.test(suffix));
  return match?.[0] ?? "uncategorized";
}

export function percentile(samples, percentileValue) {
  if (samples.length === 0) {
    return null;
  }
  const sorted = [...samples].sort((left, right) => left - right);
  const rank = Math.max(0, Math.ceil(percentileValue * sorted.length) - 1);
  return sorted[rank];
}

export function summarizeSamples(samples) {
  const values = samples
    .map((sample) => Number(sample))
    .filter((sample) => Number.isFinite(sample) && sample >= 0);
  if (values.length === 0) {
    return {
      count: 0,
      minimumMs: null,
      maximumMs: null,
      p50Ms: null,
      p95Ms: null,
      p99Ms: null,
    };
  }
  return {
    count: values.length,
    minimumMs: Math.min(...values),
    maximumMs: Math.max(...values),
    p50Ms: percentile(values, 0.5),
    p95Ms: percentile(values, 0.95),
    p99Ms: percentile(values, 0.99),
  };
}

export function summarizePerformance(scenarios) {
  const byTool = new Map();
  for (const scenario of scenarios) {
    for (const step of scenario.steps ?? []) {
      const entry = byTool.get(step.tool) ?? { samples: [], failureCount: 0 };
      entry.samples.push(step.durationMs);
      entry.requestBytes ??= [];
      entry.responseBytes ??= [];
      if (step.requestBytes !== null && step.requestBytes !== undefined) {
        entry.requestBytes.push(step.requestBytes);
      }
      if (step.responseBytes !== null && step.responseBytes !== undefined) {
        entry.responseBytes.push(step.responseBytes);
      }
      if (step.outcome === "failure") {
        entry.failureCount += 1;
      }
      byTool.set(step.tool, entry);
    }
  }
  return [...byTool.entries()].sort(([left], [right]) => left.localeCompare(right)).map(
    ([tool, entry]) => ({
      tool,
      ...summarizeSamples(entry.samples),
      requestBytes: summarizeRange(entry.requestBytes),
      responseBytes: summarizeRange(entry.responseBytes),
      successCount: entry.samples.length - entry.failureCount,
      failureCount: entry.failureCount,
    }),
  );
}

function summarizeRange(values = []) {
  const numeric = values.filter((value) => Number.isFinite(value) && value >= 0);
  return numeric.length === 0
    ? { minimum: null, maximum: null }
    : { minimum: Math.min(...numeric), maximum: Math.max(...numeric) };
}

function isLiveEvidence(step) {
  return ["success", "expected-error", "expected-unavailable"].includes(
    step.outcome,
  );
}

function validateStructuredError(response, expectedError) {
  if (!expectedError) {
    return [];
  }
  const actual = response?.result?.structuredContent;
  if (!isObject(actual) || typeof actual.code !== "string") {
    return ["expected a structured Workbench error with a stable code"];
  }
  const expectedCodes = expectedError.codes ??
    (expectedError.code ? [expectedError.code] : []);
  const expectedPhases = expectedError.phases ??
    (expectedError.phase ? [expectedError.phase] : []);
  const reasons = [];
  if (expectedCodes.length > 0 && !expectedCodes.includes(actual.code)) {
    reasons.push(
      "expected error code " +
        JSON.stringify(expectedCodes) +
        " but received " +
        JSON.stringify(actual.code),
    );
  }
  if (
    expectedPhases.length > 0 &&
    !expectedPhases.includes(actual.phase)
  ) {
    reasons.push(
      "expected error phase " +
        JSON.stringify(expectedPhases) +
        " but received " +
        JSON.stringify(actual.phase),
    );
  }
  if (
    expectedError.retryable !== undefined &&
    actual.retryable !== expectedError.retryable
  ) {
    reasons.push(
      "expected error retryable=" +
        expectedError.retryable +
        " but received " +
        actual.retryable,
    );
  }
  return reasons;
}

export function buildLiveCoverageReport(contract, scenarios = []) {
  const evidence = new Set();
  const failed = new Set();
  const incomplete = new Set();
  for (const scenario of scenarios) {
    for (const step of scenario.steps ?? []) {
      if (isLiveEvidence(step)) {
        evidence.add(step.tool);
        if (step.completion === false) {
          incomplete.add(step.tool);
        }
      } else if (step.outcome === "failure") {
        failed.add(step.tool);
      }
    }
  }
  const missing = contract.expectedNames.filter((name) => !evidence.has(name));
  const published = new Set(contract.expectedNames);
  const unexpected = [...new Set([...evidence, ...failed])].filter(
    (name) => !published.has(name),
  );
  const failedPublished = [...failed].filter((name) => published.has(name));
  const incompletePublished = [...incomplete].filter((name) =>
    published.has(name),
  );
  return {
    ok: missing.length === 0 && unexpected.length === 0 && failedPublished.length === 0,
    expectedCount: contract.expectedNames.length,
    coveredCount: contract.expectedNames.length - missing.length,
    successfulCount: scenarios.reduce(
      (count, scenario) =>
        count + (scenario.steps ?? []).filter((step) => step.outcome === "success").length,
      0,
    ),
    expectedErrorCount: scenarios.reduce(
      (count, scenario) =>
        count +
        (scenario.steps ?? []).filter((step) => step.outcome === "expected-error").length,
      0,
    ),
    expectedUnavailableCount: scenarios.reduce(
      (count, scenario) =>
        count +
        (scenario.steps ?? []).filter(
          (step) => step.outcome === "expected-unavailable",
        ).length,
      0,
    ),
    incomplete: incompletePublished,
    complete: incompletePublished.length === 0,
    missing,
    unexpected,
    failed: failedPublished,
  };
}

export function buildEndpointCorpusReport(contract, scenarios = []) {
  const observationsByTool = new Map();
  for (const scenario of scenarios) {
    for (const step of scenario.steps ?? []) {
      const observations = observationsByTool.get(step.tool) ?? [];
      observations.push({
        scenario: scenario.name ?? null,
        name: step.name ?? null,
        outcome: step.outcome ?? "unknown",
        completion: step.completion ?? true,
        durationMs: step.durationMs ?? null,
        error: step.error ?? null,
        reasons: step.reasons ?? [],
      });
      observationsByTool.set(step.tool, observations);
    }
  }

  const endpoints = contract.expectedNames.map((tool) => {
    const observations = observationsByTool.get(tool) ?? [];
    const hasFailure = observations.some((observation) =>
      observation.outcome === "failure",
    );
    const hasIncomplete = observations.some((observation) =>
      observation.completion === false,
    );
    const hasLiveEvidence = observations.some((observation) =>
      ["success", "expected-error", "expected-unavailable"].includes(
        observation.outcome,
      ),
    );
    const status = hasFailure
      ? "failed"
      : hasIncomplete
        ? "incomplete"
        : hasLiveEvidence
          ? "approved"
          : "not-tested";
    const contractEntry = contract.coverage?.find((entry) => entry.tool === tool);
    return {
      tool,
      family: contractEntry?.family ?? classifyTool(tool),
      contractEvidence: contractEntry?.contractEvidence ?? "missing",
      status,
      observationCount: observations.length,
      observations,
    };
  });

  const counts = Object.fromEntries(
    ["approved", "failed", "incomplete", "not-tested"].map((status) => [
      status,
      endpoints.filter((endpoint) => endpoint.status === status).length,
    ]),
  );
  return {
    endpointCount: endpoints.length,
    counts,
    endpoints,
  };
}

export class McpStdioClient {
  constructor({ serverPath, args = [], env = {}, requestTimeoutMs = 120000 }) {
    this.child = spawn(serverPath, args, {
      cwd: repositoryRoot,
      env: { ...process.env, ...env },
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });
    this.lines = createInterface({ input: this.child.stdout });
    this.messages = this.lines[Symbol.asyncIterator]();
    this.stderr = "";
    this.child.stderr.setEncoding("utf8");
    this.child.stderr.on("data", (chunk) => {
      this.stderr += chunk;
    });
    this.nextId = 1;
    this.requestTimeoutMs = requestTimeoutMs;
  }

  async initialize() {
    const result = await this.request("initialize", {
      protocolVersion: "2025-11-25",
      capabilities: {},
      clientInfo: {
        name: "reforger-script-tools-workbench-conformance",
        version: "1.0.0",
      },
    });
    this.send({
      jsonrpc: "2.0",
      method: "notifications/initialized",
    });
    return result;
  }

  async listTools() {
    return this.request("tools/list", {});
  }

  async callTool(name, argumentsValue = {}) {
    return (await this.callToolTimed(name, argumentsValue)).response;
  }

  async callToolTimed(name, argumentsValue = {}) {
    return this.requestTimed("tools/call", {
      name,
      arguments: argumentsValue,
    });
  }

  async request(method, params) {
    return (await this.requestTimed(method, params)).response;
  }

  async requestTimed(method, params) {
    const id = this.nextId++;
    const requestText =
      JSON.stringify({
        jsonrpc: "2.0",
        id,
        method,
        params,
      }) + "\n";
    const started = performanceNow();
    this.send({
      jsonrpc: "2.0",
      id,
      method,
      params,
    });
    while (true) {
      const next = await new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          reject(
            new Error(
              "MCP request timed out after " +
                this.requestTimeoutMs +
                "ms: " +
                method,
            ),
          );
        }, this.requestTimeoutMs);
        this.messages.next().then(
          (value) => {
            clearTimeout(timer);
            resolve(value);
          },
          (error) => {
            clearTimeout(timer);
            reject(error);
          },
        );
      });
      if (next.done) {
        throw new Error(
          "MCP process ended before responding to " +
            method +
            ": " +
            this.stderr.trim(),
        );
      }
      const message = JSON.parse(next.value);
      if (message.id === id) {
        return {
          response: message,
          timing: {
            durationMs: performanceNow() - started,
            requestBytes: Buffer.byteLength(requestText),
            responseBytes: Buffer.byteLength(next.value),
          },
        };
      }
    }
  }

  send(message) {
    this.child.stdin.write(JSON.stringify(message) + "\n");
  }

  async close() {
    this.lines.close();
    this.child.stdin.end();
    await waitForClose(this.child);
  }
}

export function loadFixtureManifest(manifestPath) {
  const manifestFile = resolve(manifestPath);
  const manifest = JSON.parse(readFileSync(manifestFile, "utf8"));
  if (!isObject(manifest)) {
    throw new Error("Workbench fixture manifest must be an object");
  }
  for (const field of ["name", "revision", "fixtureRoot", "project", "profileRoot"]) {
    if (manifest[field] === undefined) {
      throw new Error("Workbench fixture manifest is missing " + field);
    }
  }
  const manifestRoot = dirname(manifestFile);
  const fixtureRoot = resolve(manifestRoot, manifest.fixtureRoot);
  const projectPath = resolve(fixtureRoot, manifest.project.gproj);
  const profileRoot = resolve(fixtureRoot, manifest.profileRoot);
  const externalProfileRoot = manifest.profileRootOutsideFixture === true
    ? resolve(manifestRoot, manifest.profileRoot)
    : profileRoot;
  const addonsDir = resolve(manifestRoot, manifest.project.addonsDir);
  if (!existsSync(fixtureRoot)) {
    throw new Error("Workbench fixture root does not exist: " + fixtureRoot);
  }
  if (!existsSync(projectPath)) {
    throw new Error("Workbench fixture project does not exist: " + projectPath);
  }
  if (!existsSync(addonsDir)) {
    throw new Error("Workbench fixture add-ons directory does not exist: " + addonsDir);
  }
  if (!isObject(manifest.expected) || typeof manifest.expected.worldResource !== "string") {
    throw new Error("Workbench fixture manifest must define expected.worldResource");
  }
  if (
    !Array.isArray(manifest.expected.loadedAddonIds) ||
    !manifest.expected.loadedAddonIds.includes("ArmaReforger")
  ) {
    throw new Error(
      "Workbench fixture manifest must require the ArmaReforger base-game add-on",
    );
  }
  if (!isWithin(fixtureRoot, projectPath)) {
    throw new Error("Workbench fixture project must be inside fixtureRoot");
  }
  if (!isWithin(fixtureRoot, externalProfileRoot) && manifest.profileRootOutsideFixture !== true) {
    throw new Error("Workbench fixture profileRoot must be inside fixtureRoot");
  }
  mkdirSync(externalProfileRoot, { recursive: true });
  return {
    name: String(manifest.name),
    revision: String(manifest.revision),
    fixtureRoot,
    projectPath,
    profileRoot: externalProfileRoot,
    useProfile: manifest.useProfile !== false,
    allowExistingProcess: manifest.allowExistingProcess === true,
    addonsDir,
    expected: isObject(manifest.expected) ? manifest.expected : {},
    readiness: {
      timeoutMs: manifest.readiness?.timeoutMs ?? 120000,
      intervalMs: manifest.readiness?.intervalMs ?? 1000,
    },
  };
}

export class WorkbenchMcpSession {
  constructor(manifest) {
    this.manifest = manifest;
    this.processId = null;
    this.ownsProcess = false;
    this.launch = undefined;
  }

  async start(client) {
    const response = await client.callTool("workbench_launch", {
      projectPath: this.manifest.projectPath,
    });
    const launch = response?.result?.structuredContent;
    if (response?.result?.isError === true || !launch) {
      throw new Error(
        "Workbench MCP launch failed: " +
          JSON.stringify(launch ?? response?.result ?? null),
      );
    }
    this.launch = launch;
    this.processId = launch.processId ?? null;
    this.ownsProcess = launch.alreadyRunning !== true;
    if (launch.alreadyRunning === true && !this.manifest.allowExistingProcess) {
      throw new Error(
        "Workbench fixture launch reused an existing process; " +
          "refusing to run without disposable ownership",
      );
    }
    return {
      processId: this.processId,
      ownsProcess: this.ownsProcess,
      launch,
    };
  }

  async stop(client) {
    if (!this.processId || !this.ownsProcess) {
      return {
        outcome: this.processId ? "reused-existing-process" : "no-process",
        processId: this.processId,
      };
    }
    const response = await client.callTool("workbench_stop", {
      processId: this.processId,
    });
    const stopped = response?.result?.structuredContent;
    if (response?.result?.isError === true || !stopped?.exited) {
      throw new Error(
        "Workbench MCP stop did not confirm process exit: " +
          JSON.stringify(stopped ?? response?.result ?? null),
      );
    }
    return {
      outcome: "graceful",
      processId: this.processId,
    };
  }
}

export async function verifyOwnedStopRestartLifecycle({ client, session }) {
  if (!session?.ownsProcess || !session.processId) {
    throw new Error("Owned Workbench lifecycle requires a process started by MCP launch");
  }
  const originalProcessId = session.processId;
  const restartResponse = await client.callTool("workbench_restart", {
    processId: originalProcessId,
  });
  const restarted = restartResponse?.result?.structuredContent;
  if (
    restartResponse?.result?.isError === true ||
    !restarted ||
    restarted.alreadyRunning === true ||
    restarted.netApiConnected !== true ||
    !Number.isInteger(restarted.processId) ||
    restarted.processId <= 0
  ) {
    throw new Error(
      "Workbench MCP restart did not confirm an owned replacement process: " +
        JSON.stringify(restarted ?? restartResponse?.result ?? null),
    );
  }
  session.processId = restarted.processId;

  const stopResponse = await client.callTool("workbench_stop", {
    processId: session.processId,
  });
  const stopped = stopResponse?.result?.structuredContent;
  if (
    stopResponse?.result?.isError === true ||
    !stopped ||
    stopped.exited !== true
  ) {
    throw new Error(
      "Workbench MCP stop did not confirm the restarted owned process exit: " +
        JSON.stringify(stopped ?? stopResponse?.result ?? null),
    );
  }
  const restartedProcessId = session.processId;
  session.processId = null;
  session.ownsProcess = false;
  return {
    outcome: "graceful",
    originalProcessId,
    restartedProcessId,
    restart: restarted,
    stop: stopped,
  };
}

export async function waitForWorkbenchReady(client, readiness = {}) {
  const timeoutMs = readiness.timeoutMs ?? 120000;
  const intervalMs = readiness.intervalMs ?? 1000;
  const started = performanceNow();
  let attempts = 0;
  let lastError;
  while (performanceNow() - started <= timeoutMs) {
    attempts += 1;
    try {
      const response = await client.callTool("workbench_status", {});
      if (
        response?.result?.isError !== true &&
        response?.result?.structuredContent?.isRunning === true
      ) {
        return {
          ready: true,
          attempts,
          elapsedMs: performanceNow() - started,
        };
      }
      lastError = "Workbench status did not report isRunning=true";
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }
    await delay(intervalMs);
  }
  throw new Error(
    "Workbench NET API readiness timed out after " +
      timeoutMs +
      "ms" +
      (lastError ? ": " + lastError : ""),
  );
}

export async function openFixtureWorld(client, worldResource, readiness = {}) {
  const { editors } = await waitForWorkbenchEditors(client, readiness);
  const worldEditor = editors.find((editor) =>
    /world editor/i.test(String(editor?.displayName ?? "")),
  );
  if (!worldEditor || typeof worldEditor.id !== "string") {
    throw new Error(
      "Fixture Workbench did not expose a World Editor: " +
        JSON.stringify(editors),
    );
  }
  const openEditorResponse = await client.callTool("workbench_open_editor", {
    editorId: worldEditor.id,
  });
  const openedEditor = openEditorResponse?.result?.structuredContent;
  if (
    openEditorResponse?.result?.isError === true ||
    openedEditor?.opened !== true
  ) {
    throw new Error(
      "Fixture Workbench could not open World Editor " +
      JSON.stringify({ editor: worldEditor, response: openedEditor ?? null }),
    );
  }
  await waitForWorkbenchReady(client, readiness);
  const openResourceResponse = await client.callTool("workbench_open_resource", {
    resourcePath: worldResource,
  });
  const openedResource = openResourceResponse?.result?.structuredContent;
  if (
    openResourceResponse?.result?.isError === true ||
    openedResource?.opened !== true
  ) {
    const discoveredResponse = await client.callTool("workbench_search_resources", {
      kinds: ["world"],
      query: "McpConformance",
      limit: 20,
    });
    throw new Error(
      "Fixture Workbench could not open world resource " +
        JSON.stringify({
          worldResource,
          response: openedResource ?? null,
          discovered: discoveredResponse?.result?.structuredContent ?? null,
        }),
    );
  }
  const state = await waitForActiveWorld(client, worldResource, readiness);
  return {
    editor: worldEditor,
    openedEditor,
    openedResource,
    state,
  };
}

export async function waitForWorkbenchEditors(client, readiness = {}) {
  const timeoutMs = readiness.timeoutMs ?? 120000;
  const intervalMs = readiness.intervalMs ?? 1000;
  const started = performanceNow();
  let attempts = 0;
  let lastResponse;
  while (performanceNow() - started <= timeoutMs) {
    attempts += 1;
    try {
      const response = await client.callTool("workbench_list_editors", {});
      const editors = response?.result?.structuredContent?.editors;
      if (response?.result?.isError !== true && Array.isArray(editors)) {
        return {
          editors,
          attempts,
          elapsedMs: performanceNow() - started,
        };
      }
      lastResponse = response?.result?.structuredContent ?? response?.result ?? null;
    } catch (error) {
      lastResponse = error instanceof Error ? error.message : String(error);
    }
    await delay(intervalMs);
  }
  throw new Error(
    "Workbench editor catalogue readiness timed out after " +
      timeoutMs +
      "ms" +
      (lastResponse ? ": " + JSON.stringify(lastResponse) : ""),
  );
}

export async function waitForActiveWorld(client, expectedWorldResource, readiness = {}) {
  const timeoutMs = readiness.timeoutMs ?? 120000;
  const intervalMs = readiness.intervalMs ?? 1000;
  const started = performanceNow();
  let attempts = 0;
  let lastState;
  let lastError;
  while (performanceNow() - started <= timeoutMs) {
    attempts += 1;
    try {
      const response = await client.callTool("workbench_state", {});
      lastState = response?.result?.structuredContent;
      if (
        response?.result?.isError !== true &&
        lastState?.activeWorldPath === expectedWorldResource
      ) {
        return {
          ...lastState,
          attempts,
          elapsedMs: performanceNow() - started,
        };
      }
      lastError = "Workbench state did not report the expected active world";
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }
    await delay(intervalMs);
  }
  throw new Error(
    "Fixture active world did not become ready after " +
      timeoutMs +
      "ms: " +
      JSON.stringify({
        expectedWorldResource,
        attempts,
        lastState: lastState ?? null,
        lastError: lastError ?? null,
      }),
  );
}

export async function runScenario({
  client,
  name,
  steps,
  iterations = 1,
  warmup = 0,
  context = {},
}) {
  const observations = [];
  let ok = true;
  const scenarioContext = {
    ...context,
    scenario: {},
  };
  const totalIterations = Math.max(1, Number(iterations));
  const warmupIterations = Math.max(0, Number(warmup));
  for (let iteration = 0; iteration < warmupIterations + totalIterations; iteration += 1) {
    for (const step of steps) {
      const started = performanceNow();
      let timed;
      let response;
      let reasons = [];
      try {
        timed = await client.callToolTimed(
          step.tool,
          materialize(step.arguments ?? {}, scenarioContext),
        );
        response = timed.response;
        const actualIsError = response?.result?.isError === true;
        const expectedIsError = step.expect?.isError;
        if (
          expectedIsError !== undefined &&
          actualIsError !== expectedIsError
        ) {
          reasons.push(
            "expected isError=" + expectedIsError + " but received " + actualIsError,
          );
        } else if (
          expectedIsError === undefined &&
          actualIsError &&
          step.expect?.allowError !== true
        ) {
          reasons.push("expected a successful result but received isError=true");
        }
        if (actualIsError && step.expect?.allowError === true && !step.expect.error) {
          reasons.push(
            "allowError requires an explicit structured error oracle",
          );
        }
        if (actualIsError) {
          reasons.push(
            ...validateStructuredError(response, step.expect?.error),
          );
        }
        for (const [pointer, expectedValue] of Object.entries(
          step.expect?.pointers ?? {},
        )) {
          const expected = materialize(expectedValue, scenarioContext);
          const actual = readJsonPointer(response, pointer);
          if (!Object.is(actual, expected)) {
            reasons.push(
              "expected " + pointer + " to equal " + JSON.stringify(expected),
            );
          }
        }
        for (const [pointer, expectedValues] of Object.entries(
          step.expect?.contains ?? {},
        )) {
          const actual = readJsonPointer(response, pointer);
          if (
            !Array.isArray(actual) ||
            !expectedValues.every((expectedValue) =>
              actual.includes(materialize(expectedValue, scenarioContext)),
            )
          ) {
            reasons.push(
              "expected " + pointer + " to contain " + JSON.stringify(expectedValues),
            );
          }
        }
        for (const pointer of step.expect?.exists ?? []) {
          if (readJsonPointer(response, pointer) === undefined) {
            reasons.push("expected " + pointer + " to exist");
          }
        }
        if (reasons.length === 0 && isObject(step.capture)) {
          for (const [nameToCapture, pointer] of Object.entries(step.capture)) {
            const captured = readJsonPointer(response, pointer);
            if (captured === undefined) {
              reasons.push(
                "capture " + nameToCapture + " could not read " + pointer,
              );
            } else {
              scenarioContext.scenario[nameToCapture] = captured;
            }
          }
        }
      } catch (error) {
        reasons = [error instanceof Error ? error.message : String(error)];
      }
      if (iteration < warmupIterations) {
        if (reasons.length > 0) {
          throw new Error(
            "Workbench scenario warmup failed for " +
              step.tool +
              " (" +
              (step.name ?? "unnamed") +
              "): " +
              reasons.join("; "),
          );
        }
        continue;
      }
      const actualIsError = response?.result?.isError === true;
      const observation = {
        name: step.name,
        tool: step.tool,
        outcome:
          reasons.length === 0
            ? actualIsError
              ? step.expect?.completion === false
                ? "expected-unavailable"
                : "expected-error"
              : "success"
            : "failure",
        durationMs: timed?.timing.durationMs ?? performanceNow() - started,
        requestBytes: timed?.timing.requestBytes ?? null,
        responseBytes: timed?.timing.responseBytes ?? null,
      };
      const errorContent = response?.result?.structuredContent;
      if (actualIsError && isObject(errorContent)) {
        observation.error = {
          code: errorContent.code ?? null,
          phase: errorContent.phase ?? null,
          retryable: errorContent.retryable ?? null,
          logReference: errorContent.logReference ?? null,
        };
      }
      if (step.expect?.completion === false) {
        observation.completion = false;
        if (step.expect.completionReason) {
          observation.completionReason = step.expect.completionReason;
        }
      }
      if (totalIterations > 1) {
        observation.iteration = iteration - warmupIterations + 1;
      }
      if (reasons.length > 0) {
        observation.reasons = reasons;
        ok = false;
      }
      observations.push(observation);
    }
  }
  return {
    name,
    ok,
    iterations: totalIterations,
    warmup: warmupIterations,
    steps: observations,
    performance: summarizePerformance([{ name, steps: observations }]),
  };
}

export async function runContractReport({
  serverPath = resolveServerPath(),
  apiReferencePath = defaultApiReference,
  reportPath = defaultReportPath,
  indexCachePath,
  scenarioPath,
  fixturePath,
  requireLiveCoverage = false,
} = {}) {
  const args = ["mcp"];
  if (indexCachePath) {
    args.push("--index-cache", indexCachePath);
  }
  const started = performanceNow();
  const fixture = fixturePath
    ? new WorkbenchMcpSession(loadFixtureManifest(fixturePath))
    : undefined;
  if (fixture) {
    if (!fixture.manifest.allowExistingProcess && fixture.manifest.useProfile) {
      args.push("--workbench-profile-directory", fixture.manifest.profileRoot);
    }
  }
  const client = new McpStdioClient({ serverPath, args });
  let fixtureLaunch;
  let fixtureCleanup;
  let clientInitialized = false;
  let cleanupError;
  let report;
  try {
    const initialize = await client.initialize();
    clientInitialized = true;
    if (fixture) {
      fixtureLaunch = await fixture.start(client);
    }
    const listed = await client.listTools();
    const tools = listed?.result?.tools;
    if (!Array.isArray(tools)) {
      throw new Error("MCP tools/list returned no tools array");
    }
    report = {
      kind: "workbench-mcp-contract",
      server: basename(serverPath),
      protocolVersion: initialize?.result?.protocolVersion ?? null,
      machine: {
        platform: platform(),
        architecture: arch(),
        osRelease: release(),
      },
      elapsedMs: performanceNow() - started,
      contract: buildContractReport({
        apiReference: readFileSync(apiReferencePath, "utf8"),
        listedTools: tools,
      }),
    };
    if (fixture) {
      const readiness = await waitForWorkbenchReady(
        client,
        fixture.manifest.readiness,
      );
      const worldOpen = await openFixtureWorld(
        client,
        fixture.manifest.expected.worldResource,
        fixture.manifest.readiness,
      );
      const state = worldOpen.state;
      const projectResponse = await client.callTool("workbench_project_context", {});
      const project = projectResponse?.result?.structuredContent;
      const expectedLoadedAddons = fixture.manifest.expected.loadedAddonIds ?? [];
      if (
        projectResponse?.result?.isError === true ||
        !expectedLoadedAddons.every((addonId) => project?.loadedAddons?.includes(addonId))
      ) {
        throw new Error("Fixture loaded addon identities did not match the manifest");
      }
      report.fixture = {
        name: fixture.manifest.name,
        revision: fixture.manifest.revision,
        expected: fixture.manifest.expected,
        processId: fixtureLaunch.processId,
        readiness,
        editor: worldOpen.editor,
        openedEditor: worldOpen.openedEditor,
        openedResource: worldOpen.openedResource,
        activeWorldPath: state.activeWorldPath,
        bridgeVersion: state.bridgeVersion ?? null,
        bridgeProtocolVersion: state.protocolVersion ?? null,
        loadedAddons: project.loadedAddons ?? [],
      };
    }
    const scenarios = scenarioPath
      ? JSON.parse(readFileSync(scenarioPath, "utf8"))
      : undefined;
    if (scenarios) {
      const definitions = Array.isArray(scenarios.scenarios)
        ? scenarios.scenarios
        : [scenarios];
      const runs = [];
      for (const scenario of definitions) {
        runs.push(
          await runScenario({
            client,
            name: scenario.name,
            steps: scenario.steps,
            iterations: scenario.iterations,
            warmup: scenario.warmup,
            context: fixture
              ? {
                  fixture: {
                    processId: fixtureLaunch.processId,
                    projectPath: fixture.manifest.projectPath,
                    worldResource: fixture.manifest.expected.worldResource,
                  },
                }
              : {},
          }),
        );
      }
      report.scenarios = runs;
      if (runs.length === 1) {
        report.scenario = runs[0];
      }
      report.performance = summarizePerformance(runs);
      report.liveCoverage = {
        required: requireLiveCoverage,
        ...buildLiveCoverageReport(report.contract, runs),
      };
      report.endpointCorpus = buildEndpointCorpusReport(report.contract, runs);
      const evidenceTools = new Set(
        runs.flatMap((run) =>
          run.steps
            .filter(isLiveEvidence)
            .map((step) => step.tool),
        ),
      );
      report.contract.coverage = report.contract.coverage.map((entry) =>
        evidenceTools.has(entry.tool)
          ? { ...entry, liveEvidence: "scenario" }
          : entry,
      );
    } else {
      report.liveCoverage = {
        required: requireLiveCoverage,
        ...buildLiveCoverageReport(report.contract),
      };
      report.endpointCorpus = buildEndpointCorpusReport(report.contract);
    }
    if (fixture && !fixture.manifest.allowExistingProcess) {
      report.ownedLifecycle = await verifyOwnedStopRestartLifecycle({
        client,
        session: fixture,
      });
    }
  } finally {
    try {
      if (fixture) {
        fixtureCleanup = await fixture.stop(clientInitialized ? client : undefined);
      }
    } catch (error) {
      cleanupError = error;
    }
    try {
      await client.close();
    } catch (error) {
      cleanupError ??= error;
    }
  }
  if (cleanupError) {
    throw cleanupError;
  }
  if (report && fixtureCleanup) {
    report.fixture.cleanup = fixtureCleanup;
  }
  mkdirSync(dirname(reportPath), { recursive: true });
  writeFileSync(reportPath, JSON.stringify(report, null, 2) + "\n", "utf8");
  return report;
}

export function resolveServerPath(explicitPath) {
  if (explicitPath) {
    if (!existsSync(explicitPath)) {
      throw new Error("MCP server does not exist: " + explicitPath);
    }
    return resolve(explicitPath);
  }
  const candidate = defaultServerCandidates.find((path) => existsSync(path));
  if (!candidate) {
    throw new Error(
      "No bundled MCP server found. Build it first or pass --server <path>.",
    );
  }
  return candidate;
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isWithin(root, child) {
  const relativePath = relative(resolve(root), resolve(child));
  return (
    relativePath === "" ||
    (!relativePath.startsWith("..") && !isAbsolute(relativePath))
  );
}

function performanceNow() {
  return Number(process.hrtime.bigint() / 1000000n);
}

function readJsonPointer(value, pointer) {
  if (pointer === "") {
    return value;
  }
  if (!pointer.startsWith("/")) {
    return undefined;
  }
  return pointer
    .slice(1)
    .split("/")
    .map((part) => part.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((current, part) => current?.[part], value);
}

function materialize(value, context) {
  if (Array.isArray(value)) {
    return value.map((item) => materialize(item, context));
  }
  if (isObject(value)) {
    return Object.fromEntries(
      Object.entries(value).map(([key, child]) => [key, materialize(child, context)]),
    );
  }
  if (typeof value !== "string" || !value.startsWith("$")) {
    return value;
  }
  const path = value.slice(1).split(".");
  let current = context;
  for (const part of path) {
    current = current?.[part];
  }
  return current === undefined ? value : current;
}

function waitForClose(child) {
  if (child.exitCode !== null) {
    return Promise.resolve();
  }
  return new Promise((resolvePromise, reject) => {
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error("MCP process did not exit after stdin closed"));
    }, 5000);
    child.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.once("close", () => {
      clearTimeout(timer);
      resolvePromise();
    });
  });
}

function waitForCloseWithin(child, timeoutMs) {
  if (child.exitCode !== null) {
    return Promise.resolve();
  }
  return new Promise((resolvePromise, reject) => {
    const timer = setTimeout(() => {
      reject(new Error("process did not exit before cleanup deadline"));
    }, timeoutMs);
    child.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.once("close", () => {
      clearTimeout(timer);
      resolvePromise();
    });
  });
}

function delay(milliseconds) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, milliseconds));
}

function parseArguments(argumentsList) {
  const options = {};
  for (let index = 0; index < argumentsList.length; index += 1) {
    const argument = argumentsList[index];
    if (argument === "--help" || argument === "-h") {
      options.help = true;
      continue;
    }
    if (argument === "--require-live-coverage") {
      options.requirelivecoverage = true;
      continue;
    }
    if (
      ![
        "--server",
        "--api-reference",
        "--out",
        "--index-cache",
        "--scenario",
        "--fixture",
      ].includes(argument)
    ) {
      throw new Error("Unknown argument: " + argument);
    }
    const value = argumentsList[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(argument + " requires a value");
    }
    options[argument.slice(2).replaceAll("-", "")] = value;
    index += 1;
  }
  return options;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const options = parseArguments(process.argv.slice(2));
    if (options.help) {
      console.log(
        "Usage: node tools/workbench-mcp-conformance.mjs [--server PATH] " +
          "[--api-reference PATH] [--index-cache PATH] [--scenario PATH] [--fixture PATH] " +
          "[--require-live-coverage] [--out PATH]",
      );
      process.exit(0);
    }
    if (options.scenario && !options.fixture) {
      throw new Error("--scenario requires --fixture so live operations are disposable");
    }
    const report = await runContractReport({
      serverPath: resolveServerPath(options.server),
      apiReferencePath: options.apireference ?? defaultApiReference,
      indexCachePath: options.indexcache,
      scenarioPath: options.scenario,
      fixturePath: options.fixture,
      requireLiveCoverage: options.requirelivecoverage === true,
      reportPath: options.out ?? defaultReportPath,
    });
    console.log(JSON.stringify(report, null, 2));
    process.exit(
      report.contract.ok &&
        (!report.scenarios || report.scenarios.every((scenario) => scenario.ok)) &&
        (!options.requirelivecoverage || report.liveCoverage.ok)
        ? 0
        : 1,
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
