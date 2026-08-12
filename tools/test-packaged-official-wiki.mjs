import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { unzipSync } from 'fflate';

import { expectedAgentSkillContributions, expectedAgentSkillFiles } from './agent-skills.mjs';

const root = process.cwd();
const sandbox = mkdtempSync(join(tmpdir(), 'reforger-packaged-wiki-'));
const vsix = join(sandbox, 'reforger-script-tools.vsix');
const installed = join(sandbox, 'installed client é space');
const clientWorkingDirectory = join(sandbox, 'independent cwd Ω');
const gameDataScripts = join(sandbox, 'Game Data é space', 'scripts');

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

  const sourcePages = markdownFiles(join(root, 'data', 'official-wiki'));
  const installedPages = markdownFiles(join(installed, 'extension', 'data', 'official-wiki'));
  if (JSON.stringify(Object.keys(sourcePages)) !== JSON.stringify(Object.keys(installedPages)) || Object.keys(sourcePages).some(path => !sourcePages[path].equals(installedPages[path]))) {
    throw new Error('The VSIX Official Wiki Corpus differs from the authoritative source tree.');
  }
  if (!Object.hasOwn(installedPages, 'index.md')) throw new Error('The authoritative index.md is missing from the VSIX.');
  assertInstalledAgentSkills(join(installed, 'extension'));

  const executable = join(installed, 'extension', 'dist', 'server', `${process.platform}-${process.arch}`, process.platform === 'win32' ? 'reforger_language_server.exe' : 'reforger_language_server');
  const wikiSession = runMcp(executable, [], clientWorkingDirectory, [
    toolListRequest(2),
    toolCallRequest(3, 'official_wiki_status', {}),
  ]);
  const listed = response(wikiSession, 2).result.tools;
  const discoveryTools = listed.filter(tool => /search|research/.test(tool.name));
  if (listed.length !== 20
    || discoveryTools.length !== 1
    || discoveryTools[0]?.name !== 'search_reforger'
    || listed.some(tool => (tool.description?.length ?? 0) > 240)
    || listed.some(tool => tool.name === 'workbench_create_entity')) {
    throw new Error(`Installed runtime did not advertise the compact authoring profile: ${wikiSession.stdout}`);
  }
  const status = response(wikiSession, 3);
  if (status?.result?.structuredContent?.available !== true) throw new Error(`Installed corpus was unavailable: ${wikiSession.stdout}`);
  if (status.result.structuredContent.fileCount !== Object.keys(sourcePages).filter(path => path !== 'wiki-index.md').length) throw new Error('Installed corpus page count is incomplete.');
  const wikiSearchSession = runMcp(executable, [], clientWorkingDirectory, [
    toolCallRequest(2, 'search_reforger', { query: 'Reforger', sources: ['officialWiki'] }),
  ]);
  assertUnderFiveSeconds('cold Official Wiki search', wikiSearchSession);
  const readHandoff = response(wikiSearchSession, 2).result.structuredContent.results?.[0]?.read;
  if (readHandoff?.tool !== 'read_official_wiki') throw new Error(`Installed wiki search did not return a read handoff: ${wikiSearchSession.stdout}`);
  const wikiReadSession = runMcp(executable, [], clientWorkingDirectory, [
    toolCallRequest(2, readHandoff.tool, readHandoff.arguments),
  ]);
  const wikiRead = response(wikiReadSession, 2).result.structuredContent;
  if (!wikiRead?.sourceUrl || !wikiRead.content || wikiRead.relativePath !== readHandoff.arguments.relativePath) {
    throw new Error(`Installed wiki read did not complete the search handoff: ${wikiReadSession.stdout}`);
  }
  const gameDataReadySession = runMcp(executable, [
    '--workspace-scripts', gameDataScripts,
  ], clientWorkingDirectory, [
    toolCallRequest(2, 'search_reforger', { query: 'PackagedFixture', sources: ['workspace'] }),
  ]);
  if (response(gameDataReadySession, 2).result.structuredContent?.results?.[0]?.title !== 'PackagedFixture') {
    throw new Error(`Installed workspace catalogue did not become ready: ${gameDataReadySession.stdout}`);
  }
  const gameDataSession = runMcp(executable, [
    '--workspace-scripts', gameDataScripts,
  ], clientWorkingDirectory, [
    toolCallRequest(2, 'search_reforger', { query: 'PackagedFixture', sources: ['workspace'] }),
  ]);
  const gameDataSearch = response(gameDataSession, 2).result.structuredContent;
  assertUnderFiveSeconds('ready workspace search', gameDataSession);
  const gameDataHit = gameDataSearch?.results?.[0];
  if (gameDataHit?.title !== 'PackagedFixture') {
    throw new Error(`Installed workspace search did not complete: ${gameDataSession.stdout}`);
  }
  const gameDataInspectSession = runMcp(executable, [
    '--workspace-scripts', gameDataScripts,
  ], clientWorkingDirectory, [
    toolCallRequest(2, gameDataHit.inspect.tool, gameDataHit.inspect.arguments),
  ]);
  if (response(gameDataInspectSession, 2).result.structuredContent?.qualifiedName !== 'PackagedFixture') {
    throw new Error(`Installed workspace inspection did not complete the search handoff: ${gameDataInspectSession.stdout}`);
  }
  const gameDataReadSession = runMcp(executable, [
    '--workspace-scripts', gameDataScripts,
  ], clientWorkingDirectory, [
    toolCallRequest(2, gameDataHit.read.tool, gameDataHit.read.arguments),
  ]);
  if (!response(gameDataReadSession, 2).result.structuredContent?.content.includes('class PackagedFixture')) {
    throw new Error(`Installed workspace source read did not complete the search handoff: ${gameDataReadSession.stdout}`);
  }
  const physicalPaths = [installed, clientWorkingDirectory]
    .map(path => path.replaceAll('\\', '/'));
  for (const session of [wikiSession, wikiSearchSession, wikiReadSession, gameDataSession, gameDataInspectSession, gameDataReadSession]) {
    const output = session.stdout.replaceAll('\\', '/');
    if (physicalPaths.some(path => output.includes(path))) {
      throw new Error(`Installed MCP output leaked a physical path: ${session.stdout}`);
    }
  }
  console.log(`Verified ${Object.keys(sourcePages).length} byte-identical packaged Wiki files, ${expectedAgentSkillFiles.length} reachable Agent Skill files, 20 concise authoring tools, and independent workspace and Official Wiki workflows.`);
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

function assertInstalledAgentSkills(installedExtension) {
  const packageJson = JSON.parse(readFileSync(join(installedExtension, 'package.json'), 'utf8'));
  const contributions = packageJson.contributes?.chatSkills ?? [];
  if (JSON.stringify(contributions) !== JSON.stringify(expectedAgentSkillContributions)) {
    throw new Error('The installed extension does not contribute the exact Agent Skill entry points.');
  }

  const installedSkillsRoot = resolve(installedExtension, 'skills');
  const reachable = new Set();
  const pending = contributions.map(contribution => contribution.path.replace(/^\.\//, ''));
  while (pending.length > 0) {
    const source = pending.shift();
    if (reachable.has(source)) continue;
    const sourcePath = resolve(installedExtension, source);
    const relativeToSkills = relative(installedSkillsRoot, sourcePath);
    if (relativeToSkills === '..' || relativeToSkills.startsWith(`..${sep}`)) {
      throw new Error(`Installed Agent Skill entry escapes the library: ${source}`);
    }
    const contents = readFileSync(sourcePath);
    const sourceContents = readFileSync(resolve(root, source));
    if (!contents.equals(sourceContents)) {
      throw new Error(`Installed Agent Skill differs from its repository source: ${source}`);
    }
    reachable.add(source.replaceAll('\\', '/'));
    const text = contents.toString('utf8');
    for (const match of text.matchAll(/\[[^\]]*\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g)) {
      const reference = match[1];
      if (reference.startsWith('#') || /^[a-z][a-z0-9+.-]*:/i.test(reference)) continue;
      const targetPath = resolve(dirname(sourcePath), decodeURIComponent(reference.split('#', 1)[0]));
      const targetRelativeToSkills = relative(installedSkillsRoot, targetPath);
      if (targetRelativeToSkills === '..' || targetRelativeToSkills.startsWith(`..${sep}`)) {
        throw new Error(`Installed Agent Skill reference escapes the library: ${source} -> ${reference}`);
      }
      pending.push(relative(installedExtension, targetPath).replaceAll('\\', '/'));
    }
  }

  if (JSON.stringify([...reachable].sort()) !== JSON.stringify([...expectedAgentSkillFiles].sort())) {
    throw new Error(`Installed Agent Skill reference graph is incomplete: ${[...reachable].sort().join(', ')}`);
  }
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
