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

const workbenchHeading = /^## \x60(workbench_[^\x60]+)\x60$/;

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
  ["save", /^save_(all|world)$/],
];

export function extractWorkbenchToolNames(apiReference) {
  const names = [];
  for (const line of apiReference.split(/\r?\n/)) {
    const match = workbenchHeading.exec(line);
    if (match) {
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

export function buildLiveCoverageReport(contract, scenarios = []) {
  const covered = new Set(
    scenarios.flatMap((scenario) =>
      (scenario.steps ?? []).map((step) => step.tool),
    ),
  );
  const missing = contract.expectedNames.filter((name) => !covered.has(name));
  const published = new Set(contract.expectedNames);
  const unexpected = [...covered].filter((name) => !published.has(name));
  return {
    ok: missing.length === 0 && unexpected.length === 0,
    expectedCount: contract.expectedNames.length,
    coveredCount: contract.expectedNames.length - missing.length,
    missing,
    unexpected,
  };
}

export class McpStdioClient {
  constructor({ serverPath, args = [], env = {} }) {
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
      const next = await this.messages.next();
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
  for (const field of [
    "name",
    "revision",
    "fixtureRoot",
    "project",
    "workbench",
    "profileRoot",
  ]) {
    if (manifest[field] === undefined) {
      throw new Error("Workbench fixture manifest is missing " + field);
    }
  }
  const manifestRoot = dirname(manifestFile);
  const fixtureRoot = resolve(manifestRoot, manifest.fixtureRoot);
  const projectPath = resolve(fixtureRoot, manifest.project.gproj);
  const profileRoot = resolve(fixtureRoot, manifest.profileRoot);
  const addonsDir = resolve(manifestRoot, manifest.project.addonsDir);
  const workbenchExecutable = resolve(
    manifestRoot,
    manifest.workbench.executable,
  );
  const workingDirectory = resolve(
    manifestRoot,
    manifest.workbench.workingDirectory ?? dirname(workbenchExecutable),
  );
  if (!existsSync(fixtureRoot)) {
    throw new Error("Workbench fixture root does not exist: " + fixtureRoot);
  }
  if (!existsSync(projectPath)) {
    throw new Error("Workbench fixture project does not exist: " + projectPath);
  }
  if (!existsSync(addonsDir)) {
    throw new Error("Workbench fixture add-ons directory does not exist: " + addonsDir);
  }
  if (!existsSync(workbenchExecutable)) {
    throw new Error("Workbench executable does not exist: " + workbenchExecutable);
  }
  if (!existsSync(workingDirectory)) {
    throw new Error("Workbench working directory does not exist: " + workingDirectory);
  }
  if (!isObject(manifest.expected) || typeof manifest.expected.worldResource !== "string") {
    throw new Error("Workbench fixture manifest must define expected.worldResource");
  }
  if (!isWithin(fixtureRoot, projectPath)) {
    throw new Error("Workbench fixture project must be inside fixtureRoot");
  }
  if (!isWithin(fixtureRoot, profileRoot)) {
    throw new Error("Workbench fixture profileRoot must be inside fixtureRoot");
  }
  mkdirSync(profileRoot, { recursive: true });
  return {
    name: String(manifest.name),
    revision: String(manifest.revision),
    fixtureRoot,
    projectPath,
    profileRoot,
    addonsDir,
    workbenchExecutable,
    workingDirectory,
    expected: isObject(manifest.expected) ? manifest.expected : {},
    readiness: {
      timeoutMs: manifest.readiness?.timeoutMs ?? 120000,
      intervalMs: manifest.readiness?.intervalMs ?? 1000,
    },
  };
}

export class DisposableWorkbenchProcess {
  constructor(manifest) {
    this.manifest = manifest;
    this.child = undefined;
    this.stderr = "";
  }

  start() {
    if (this.child && this.child.exitCode === null) {
      throw new Error("Fixture Workbench is already running");
    }
    this.child = spawn(
      this.manifest.workbenchExecutable,
      [
        "-noThrow",
        "-profile",
        this.manifest.profileRoot,
        "-gproj",
        this.manifest.projectPath,
        "-addonsDir",
        this.manifest.addonsDir,
        "-wbModule",
        "WorldEditor",
        "-run",
        "-load",
        this.manifest.expected.worldResource,
      ],
      {
        cwd: this.manifest.workingDirectory,
        stdio: ["ignore", "ignore", "pipe"],
        windowsHide: true,
      },
    );
    this.child.stderr.setEncoding("utf8");
    this.child.stderr.on("data", (chunk) => {
      this.stderr += chunk;
    });
    return {
      processId: this.child.pid ?? null,
      arguments: [
        "-noThrow",
        "-profile",
        "<fixture-profile-root>",
        "-gproj",
        "<fixture-project>",
        "-addonsDir",
        "<fixture-addons>",
        "-wbModule",
        "WorldEditor",
        "-run",
        "-load",
        "<fixture-world-resource>",
      ],
    };
  }

  async stop(client) {
    if (!this.child || this.child.exitCode !== null) {
      return { outcome: "already-exited", processId: this.child?.pid ?? null };
    }
    const processId = this.child.pid;
    let graceful = false;
    let stopError;
    if (client && processId) {
      try {
        const response = await client.callTool("workbench_stop", { processId });
        graceful = response?.result?.isError !== true;
      } catch (error) {
        stopError = error instanceof Error ? error.message : String(error);
      }
    }
    try {
      await waitForCloseWithin(this.child, 20000);
    } catch {
      if (this.child.exitCode === null) {
        this.child.kill();
      }
      try {
        await waitForCloseWithin(this.child, 5000);
      } catch (error) {
        throw new Error(
          "Fixture Workbench did not exit after cleanup" +
            (stopError ? ": " + stopError : "") +
            "; " +
            (error instanceof Error ? error.message : String(error)),
        );
      }
    }
    return {
      outcome: graceful ? "graceful" : "forced-or-unverified",
      processId,
      ...(stopError ? { error: stopError } : {}),
    };
  }
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
          materialize(step.arguments ?? {}, context),
        );
        response = timed.response;
        const actualIsError = response?.result?.isError === true;
        const expectedIsError = step.expect?.isError ?? false;
        if (actualIsError !== expectedIsError) {
          reasons.push(
            "expected isError=" + expectedIsError + " but received " + actualIsError,
          );
        }
        for (const [pointer, expectedValue] of Object.entries(
          step.expect?.pointers ?? {},
        )) {
          const expected = materialize(expectedValue, context);
          const actual = readJsonPointer(response, pointer);
          if (!Object.is(actual, expected)) {
            reasons.push(
              "expected " + pointer + " to equal " + JSON.stringify(expected),
            );
          }
        }
      } catch (error) {
        reasons = [error instanceof Error ? error.message : String(error)];
      }
      if (iteration < warmupIterations) {
        if (reasons.length > 0) {
          throw new Error("Workbench scenario warmup failed: " + reasons.join("; "));
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
              ? "expected-error"
              : "success"
            : "failure",
        durationMs: timed?.timing.durationMs ?? performanceNow() - started,
        requestBytes: timed?.timing.requestBytes ?? null,
        responseBytes: timed?.timing.responseBytes ?? null,
      };
      if (totalIterations > 1) {
        observation.iteration = iteration - warmupIterations + 1;
      }
      if (reasons.length > 0) {
        observation.reasons = reasons;
        ok = false;
      }
      observations.push(observation);
      if (!ok) {
        break;
      }
    }
    if (!ok) {
      break;
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
    ? new DisposableWorkbenchProcess(loadFixtureManifest(fixturePath))
    : undefined;
  const client = new McpStdioClient({ serverPath, args });
  let fixtureLaunch;
  let fixtureCleanup;
  let clientInitialized = false;
  let cleanupError;
  let report;
  try {
    if (fixture) {
      fixtureLaunch = fixture.start();
    }
    const initialize = await client.initialize();
    clientInitialized = true;
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
      const stateResponse = await client.callTool("workbench_state", {});
      const state = stateResponse?.result?.structuredContent;
      if (
        stateResponse?.result?.isError === true ||
        state?.activeWorldPath !== fixture.manifest.expected.worldResource
      ) {
        throw new Error(
          "Fixture active world identity did not match expected.worldResource",
        );
      }
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
      const covered = new Set(
        runs.flatMap((run) => run.steps.map((step) => step.tool)),
      );
      report.contract.coverage = report.contract.coverage.map((entry) =>
        covered.has(entry.tool) ? { ...entry, liveEvidence: "scenario" } : entry,
      );
    } else {
      report.liveCoverage = {
        required: requireLiveCoverage,
        ...buildLiveCoverageReport(report.contract),
      };
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
