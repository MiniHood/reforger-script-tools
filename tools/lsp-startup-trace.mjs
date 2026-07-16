#!/usr/bin/env node

import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { homedir } from "node:os";

const args = parseArgs(process.argv.slice(2));
const globalStorage = args.globalStorage ?? defaultGlobalStorage();
const out = args.out ?? join("tools", "reports", "lsp-startup-trace.report.md");

const serverLog = join(globalStorage, "logs", "language-server.log");
const clientLog = join(globalStorage, "logs", "language-client-startup.log");
const vscodeOutputLog = findLatestVsCodeOutputLog();

const serverLines = readLines(serverLog);
const clientEvents = readJsonLines(clientLog);
const outputLines = readLines(vscodeOutputLog);

const report = renderReport({
  globalStorage,
  serverLog,
  clientLog,
  vscodeOutputLog,
  serverLines,
  clientEvents,
  outputLines,
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
    } else if (arg === "--out") {
      parsed.out = rawArgs[++index];
    } else if (arg === "--help" || arg === "-h") {
      console.log(`Usage: node tools/lsp-startup-trace.mjs [--global-storage <path>] [--out <path>]`);
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

function readLines(path) {
  if (!path || !existsSync(path)) {
    return [];
  }
  return readFileSync(path, "utf8").split(/\r?\n/).filter((line) => line.length > 0);
}

function readJsonLines(path) {
  return readLines(path)
    .map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return { event: "unparsed", raw: line };
      }
    });
}

function findLatestVsCodeOutputLog() {
  const logsRoot = process.platform === "win32"
    ? join(process.env.APPDATA ?? join(homedir(), "AppData", "Roaming"), "Code", "logs")
    : join(homedir(), ".config", "Code", "logs");
  if (!existsSync(logsRoot)) {
    return undefined;
  }

  const matches = [];
  walk(logsRoot, (path) => {
    const lower = path.toLowerCase();
    if (lower.includes("reforger-sript-tools") || lower.endsWith("reforger script tools language server.log")) {
      matches.push(path);
    }
  });
  matches.sort((left, right) => statSync(right).mtimeMs - statSync(left).mtimeMs);
  return matches[0];
}

function walk(root, onFile) {
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      walk(path, onFile);
    } else if (entry.isFile()) {
      onFile(path);
    }
  }
}

function renderReport(input) {
  const latestSession = latestClientSession(input.clientEvents);
  const latestServerLines = latestServerSession(input.serverLines);
  const latestOutputLines = input.outputLines.slice(-120);
  const serverStatus = classifyServerStatus(latestServerLines, latestOutputLines);
  const clientStatus = classifyClientStatus(latestSession);

  const lines = [];
  lines.push("# LSP Startup Trace");
  lines.push("");
  lines.push("This report merges the TypeScript startup log, Rust language-server log, and VS Code language-client output so startup stalls can be diagnosed from one file.");
  lines.push("");
  lines.push("## Summary");
  lines.push("");
  lines.push(`- Generated: ${new Date().toISOString()}`);
  lines.push(`- Global storage: \`${input.globalStorage}\``);
  lines.push(`- Client status: ${clientStatus}`);
  lines.push(`- Server status: ${serverStatus}`);
  lines.push(`- Latest client session: ${latestSession?.session ?? "None"}`);
  lines.push(`- Latest server startup lines: ${latestServerLines.length}`);
  lines.push(`- VS Code output lines: ${latestOutputLines.length}`);
  lines.push("");
  lines.push("## Log Files");
  lines.push("");
  lines.push(`- TypeScript startup: \`${input.clientLog}\`${existsSync(input.clientLog) ? "" : " (missing)"}`);
  lines.push(`- Rust server: \`${input.serverLog}\`${existsSync(input.serverLog) ? "" : " (missing)"}`);
  lines.push(`- VS Code output: \`${input.vscodeOutputLog ?? "not found"}\``);
  lines.push("");

  lines.push("## Latest Client Timeline");
  lines.push("");
  if (!latestSession) {
    lines.push("None.");
  } else {
    lines.push("| Elapsed ms | Event | Detail |");
    lines.push("| ---: | --- | --- |");
    for (const event of latestSession.events.slice(-80)) {
      lines.push(`| ${formatValue(event.elapsedMs)} | ${escapeMd(event.event ?? "unknown")} | ${escapeMd(clientEventDetail(event))} |`);
    }
  }
  lines.push("");

  lines.push("## Latest Rust Server Timeline");
  lines.push("");
  if (latestServerLines.length === 0) {
    lines.push("None.");
  } else {
    lines.push("```text");
    lines.push(...latestServerLines.slice(-120));
    lines.push("```");
  }
  lines.push("");

  lines.push("## Latest VS Code Output");
  lines.push("");
  if (latestOutputLines.length === 0) {
    lines.push("None.");
  } else {
    lines.push("```text");
    lines.push(...latestOutputLines);
    lines.push("```");
  }
  lines.push("");

  lines.push("## Interpretation");
  lines.push("");
  lines.push(...interpret(latestSession, latestServerLines, latestOutputLines).map((line) => `- ${line}`));
  lines.push("");
  return `${lines.join("\n")}\n`;
}

function latestClientSession(events) {
  const sessions = new Map();
  for (const event of events) {
    const session = event.session ?? "unknown";
    if (!sessions.has(session)) {
      sessions.set(session, []);
    }
    sessions.get(session).push(event);
  }
  let latest;
  for (const [session, sessionEvents] of sessions) {
    const last = sessionEvents.at(-1);
    if (!latest || (last?.timestamp ?? "") > (latest.last?.timestamp ?? "")) {
      latest = { session, events: sessionEvents, last };
    }
  }
  return latest;
}

function latestServerSession(lines) {
  let start = -1;
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    if (lines[index].includes(" startup server=")) {
      start = index;
      break;
    }
  }
  return start >= 0 ? lines.slice(start) : lines.slice(-120);
}

function classifyClientStatus(session) {
  if (!session) {
    return "No TypeScript startup session found.";
  }
  const events = session.events.map((event) => event.event);
  if (!events.includes("languageServerProcessSpawnRequested")) {
    return "Client activated but did not request language-server spawn.";
  }
  if (!events.includes("languageServerInitializeResponse")) {
    return "Server spawn requested, but initialize response was not logged.";
  }
  if (!events.includes("firstDocumentOpened")) {
    return "Server initialized, but no Enforce document-open event was logged.";
  }
  if (!events.includes("firstSemanticTokenResponse")) {
    return "Server initialized and saw an open document, but no semantic-token response completed.";
  }
  const firstSemantic = session.events.find((event) => event.event === "firstSemanticTokenResponse");
  return `First semantic-token response completed in ${formatValue(firstSemantic.elapsedMsForRequest)} ms.`;
}

function classifyServerStatus(serverLines, outputLines) {
  const text = `${serverLines.join("\n")}\n${outputLines.join("\n")}`.toLowerCase();
  if (text.includes("allocation") || text.includes("paging file") || text.includes("out of memory")) {
    return "Crash or stall likely involved memory allocation pressure.";
  }
  if (text.includes("timed out")) {
    return "VS Code/client reported a stop or restart timeout.";
  }
  if (serverLines.some((line) => line.includes("externalIndex gameData ready"))) {
    return "Game-data external index reached ready state.";
  }
  if (serverLines.some((line) => line.includes("phase=map-rebuild-end"))) {
    return "Game-data cache decoded and lookup maps rebuilt, but ready state was not logged afterward.";
  }
  if (serverLines.some((line) => line.includes("phase=cache-load-start"))) {
    return "Game-data cache load started but did not reach map rebuild or ready state.";
  }
  if (serverLines.some((line) => line.includes("externalIndex gameData start"))) {
    return "Game-data external index started but no cache phase progress was logged.";
  }
  return "No Rust server startup progress found.";
}

function interpret(session, serverLines, outputLines) {
  const notes = [];
  const clientEvents = new Set(session?.events.map((event) => event.event) ?? []);
  const serverText = serverLines.join("\n");
  const outputText = outputLines.join("\n");

  if (!clientEvents.has("languageServerInitializeResponse")) {
    notes.push("Focus first on process launch or JSON-RPC initialize. The client never logged initialize completion.");
  }
  if (clientEvents.has("languageServerInitializeResponse") && !clientEvents.has("firstSemanticTokenResponse")) {
    notes.push("The client initialized successfully but did not complete the first semantic-token response in this session.");
  }
  if (serverText.includes("phase=map-rebuild-end") && !serverText.includes("externalIndex gameData ready")) {
    notes.push("The Rust server got through binary cache decode and lookup-map rebuild, then failed or restarted before publishing the ready index.");
  }
  if (/allocation|paging file|out of memory/i.test(outputText)) {
    notes.push("The VS Code output contains memory pressure evidence. Treat cache decoder bounds and semantic-token caps as the first verification points.");
  }
  if (/Restarting language server|development language-server binary changed/i.test(outputText)) {
    notes.push("The language client detected a development binary change and restarted the server. Ignore startup timings across that boundary.");
  }
  if (/Stopping server timed out/i.test(outputText)) {
    notes.push("Server shutdown did not complete quickly. Check whether a long request or background worker blocked process exit.");
  }
  if (notes.length === 0) {
    notes.push("No obvious failure signature was detected. Reproduce once, then rerun this report immediately.");
  }
  return notes;
}

function clientEventDetail(event) {
  const details = [];
  for (const [key, value] of Object.entries(event)) {
    if (["timestamp", "session", "elapsedMs", "event"].includes(key)) {
      continue;
    }
    details.push(`${key}=${formatValue(value)}`);
  }
  return details.join(" ");
}

function formatValue(value) {
  if (value === undefined || value === null) {
    return "";
  }
  if (typeof value === "string") {
    return value;
  }
  return String(value);
}

function escapeMd(value) {
  return String(value).replaceAll("|", "\\|").replaceAll("\n", " ");
}
