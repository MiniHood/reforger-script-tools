import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { unzipSync } from 'fflate';

const root = process.cwd();
const sandbox = mkdtempSync(join(tmpdir(), 'reforger-packaged-wiki-'));
const vsix = join(sandbox, 'reforger-script-tools.vsix');
const installed = join(sandbox, 'installed client é space');
const clientWorkingDirectory = join(sandbox, 'independent cwd Ω');
const gameDataScripts = join(sandbox, 'Game Data é space', 'scripts');
const gameDataCache = join(sandbox, 'Game Data é space', 'cache', 'index.bin');

try {
  run('npx', ['--no-install', 'vsce', 'package', '--no-dependencies', '--out', vsix]);
  for (const [path, contents] of Object.entries(unzipSync(readFileSync(vsix)))) {
    const output = join(installed, path);
    mkdirSync(dirname(output), { recursive: true });
    writeFileSync(output, contents);
  }
  mkdirSync(clientWorkingDirectory, { recursive: true });
  mkdirSync(gameDataScripts, { recursive: true });
  writeFileSync(join(gameDataScripts, 'PackagedFixture.c'), 'class PackagedFixture {}\n');

  const sourcePages = markdownFiles(join(root, 'resources', 'official-wiki'));
  const installedPages = markdownFiles(join(installed, 'extension', 'resources', 'official-wiki'));
  if (JSON.stringify(Object.keys(sourcePages)) !== JSON.stringify(Object.keys(installedPages)) || Object.keys(sourcePages).some(path => !sourcePages[path].equals(installedPages[path]))) {
    throw new Error('The VSIX Official Wiki Corpus differs from the authoritative source tree.');
  }
  if (!Object.hasOwn(installedPages, 'index.md')) throw new Error('The authoritative index.md is missing from the VSIX.');

  const executable = join(installed, 'extension', 'dist', 'server', `${process.platform}-${process.arch}`, process.platform === 'win32' ? 'reforger_language_server.exe' : 'reforger_language_server');
  const wikiSession = runMcp(executable, [], clientWorkingDirectory, [
    toolListRequest(2),
    toolCallRequest(3, 'official_wiki_status', {}),
  ]);
  const listed = response(wikiSession, 2).result.tools;
  if (listed.length !== 7 || listed.some(tool => tool.annotations?.readOnlyHint !== true || tool.annotations?.openWorldHint !== false)) {
    throw new Error(`Installed runtime did not advertise seven closed-world read-only tools: ${wikiSession.stdout}`);
  }
  const status = response(wikiSession, 3);
  if (status?.result?.structuredContent?.available !== true) throw new Error(`Installed corpus was unavailable: ${wikiSession.stdout}`);
  if (status.result.structuredContent.fileCount !== Object.keys(sourcePages).filter(path => path !== 'wiki-index.md').length) throw new Error('Installed corpus page count is incomplete.');
  const wikiSearchSession = runMcp(executable, [], clientWorkingDirectory, [
    toolCallRequest(2, 'search_official_wiki', { query: 'Reforger', limit: 1 }),
  ]);
  assertUnderFiveSeconds('cold Official Wiki search', wikiSearchSession);
  const readInput = response(wikiSearchSession, 2).result.structuredContent.results?.[0]?.readInput;
  if (!readInput) throw new Error(`Installed wiki search did not return a read handoff: ${wikiSearchSession.stdout}`);
  const wikiReadSession = runMcp(executable, [], clientWorkingDirectory, [
    toolCallRequest(2, 'read_official_wiki', readInput),
  ]);
  const wikiRead = response(wikiReadSession, 2).result.structuredContent;
  if (!wikiRead?.sourceUrl || !wikiRead.content || wikiRead.relativePath !== readInput.relativePath) {
    throw new Error(`Installed wiki read did not complete the search handoff: ${wikiReadSession.stdout}`);
  }
  const gameDataReadySession = runMcp(executable, [
    '--game-data-scripts', gameDataScripts,
    '--index-cache', gameDataCache,
  ], clientWorkingDirectory, [
    toolCallRequest(2, 'game_data_status', {}),
  ]);
  if (response(gameDataReadySession, 2).result.structuredContent?.available !== true) {
    throw new Error(`Installed Game Data catalogue did not become ready: ${gameDataReadySession.stdout}`);
  }
  const gameDataSession = runMcp(executable, [
    '--game-data-scripts', gameDataScripts,
    '--index-cache', gameDataCache,
  ], clientWorkingDirectory, [
    toolCallRequest(2, 'search_game_data_symbols', { query: 'PackagedFixture' }),
  ]);
  const gameDataSearch = response(gameDataSession, 2).result.structuredContent;
  assertUnderFiveSeconds('ready Game Data search', gameDataSession);
  const gameDataHit = gameDataSearch?.results?.[0];
  if (gameDataHit?.name !== 'PackagedFixture') {
    throw new Error(`Installed Game Data search did not complete: ${gameDataSession.stdout}`);
  }
  const gameDataInspectSession = runMcp(executable, [
    '--game-data-scripts', gameDataScripts,
    '--index-cache', gameDataCache,
  ], clientWorkingDirectory, [
    toolCallRequest(2, 'inspect_game_data_symbol', gameDataHit.inspectInput),
  ]);
  if (response(gameDataInspectSession, 2).result.structuredContent?.name !== 'PackagedFixture') {
    throw new Error(`Installed Game Data inspection did not complete the search handoff: ${gameDataInspectSession.stdout}`);
  }
  const gameDataReadSession = runMcp(executable, [
    '--game-data-scripts', gameDataScripts,
    '--index-cache', gameDataCache,
  ], clientWorkingDirectory, [
    toolCallRequest(2, 'read_game_data_source', gameDataHit.readSourceInput),
  ]);
  if (!response(gameDataReadSession, 2).result.structuredContent?.content.includes('class PackagedFixture')) {
    throw new Error(`Installed Game Data source read did not complete the search handoff: ${gameDataReadSession.stdout}`);
  }
  const physicalPaths = [sandbox, installed, clientWorkingDirectory, gameDataScripts]
    .map(path => path.replaceAll('\\', '/'));
  for (const session of [wikiSession, wikiSearchSession, wikiReadSession, gameDataSession, gameDataInspectSession, gameDataReadSession]) {
    const output = session.stdout.replaceAll('\\', '/');
    if (physicalPaths.some(path => output.includes(path))) {
      throw new Error(`Installed MCP output leaked a physical path: ${session.stdout}`);
    }
  }
  console.log(`Verified ${Object.keys(sourcePages).length} byte-identical packaged Markdown files, seven installed tools, and independent Game Data and Official Wiki workflows.`);
} finally {
  rmSync(sandbox, { recursive: true, force: true });
}

function markdownFiles(directory) {
  return Object.fromEntries(readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return Object.entries(markdownFiles(path)).map(([child, contents]) => [join(entry.name, child), contents]);
    return entry.isFile() && entry.name.endsWith('.md') ? [[entry.name, readFileSync(path)]] : [];
  }).map(([path, contents]) => [path.replaceAll('\\', '/'), contents]).sort(([left], [right]) => left.localeCompare(right)));
}

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', shell: process.platform === 'win32' });
  if (result.status !== 0) throw new Error(`${command} failed with ${result.status}`);
}

function runMcp(executable, args, cwd, requests) {
  const request = [
    { jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'package-test', version: '1' } } },
    { jsonrpc: '2.0', method: 'notifications/initialized' },
    ...requests,
  ].map(JSON.stringify).join('\n') + '\n';
  const startedAt = performance.now();
  const result = spawnSync(executable, ['mcp', ...args], { cwd, input: request, encoding: 'utf8', timeout: 15_000 });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Installed MCP runtime failed: ${result.stderr}`);
  return {
    stdout: result.stdout,
    elapsedMs: performance.now() - startedAt,
    responses: result.stdout.split(/\r?\n/).filter(Boolean).map(JSON.parse),
  };
}

function assertUnderFiveSeconds(operation, session) {
  if (session.elapsedMs >= 5_000) {
    throw new Error(`${operation} exceeded the five-second ceiling (${Math.round(session.elapsedMs)} ms).`);
  }
}

function response(session, id) {
  const result = session.responses.find(message => message.id === id);
  if (!result) throw new Error(`Installed MCP runtime did not respond to ${id}: ${session.stdout}`);
  return result;
}

function toolListRequest(id) {
  return { jsonrpc: '2.0', id, method: 'tools/list', params: {} };
}

function toolCallRequest(id, name, arguments_) {
  return { jsonrpc: '2.0', id, method: 'tools/call', params: { name, arguments: arguments_ } };
}
