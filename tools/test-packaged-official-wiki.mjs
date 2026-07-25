import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { unzipSync } from 'fflate';

const root = process.cwd();
const sandbox = mkdtempSync(join(tmpdir(), 'reforger-packaged-wiki-'));
const vsix = join(sandbox, 'reforger-script-tools.vsix');
const installed = join(sandbox, 'installed');

try {
  run('npx', ['--no-install', 'vsce', 'package', '--no-dependencies', '--out', vsix]);
  for (const [path, contents] of Object.entries(unzipSync(readFileSync(vsix)))) {
    const output = join(installed, path);
    mkdirSync(dirname(output), { recursive: true });
    writeFileSync(output, contents);
  }

  const sourcePages = markdownFiles(join(root, 'resources', 'official-wiki'));
  const installedPages = markdownFiles(join(installed, 'extension', 'resources', 'official-wiki'));
  if (JSON.stringify(Object.keys(sourcePages)) !== JSON.stringify(Object.keys(installedPages)) || Object.keys(sourcePages).some(path => !sourcePages[path].equals(installedPages[path]))) {
    throw new Error('The VSIX Official Wiki Corpus differs from the authoritative source tree.');
  }
  if (!Object.hasOwn(installedPages, 'index.md')) throw new Error('The authoritative index.md is missing from the VSIX.');

  const executable = join(installed, 'extension', 'dist', 'server', `${process.platform}-${process.arch}`, process.platform === 'win32' ? 'reforger_language_server.exe' : 'reforger_language_server');
  const request = [
    { jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'package-test', version: '1' } } },
    { jsonrpc: '2.0', method: 'notifications/initialized' },
    { jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name: 'official_wiki_status', arguments: {} } },
  ].map(JSON.stringify).join('\n') + '\n';
  const result = spawnSync(executable, ['mcp'], { cwd: sandbox, input: request, encoding: 'utf8', timeout: 15_000 });
  if (result.error) throw result.error;
  const status = result.stdout.split(/\r?\n/).filter(Boolean).map(JSON.parse).find(message => message.id === 2);
  if (status?.result?.structuredContent?.available !== true) throw new Error(`Installed corpus was unavailable: ${result.stdout}${result.stderr}`);
  if (status.result.structuredContent.fileCount !== Object.keys(sourcePages).filter(path => path !== 'wiki-index.md').length) throw new Error('Installed corpus page count is incomplete.');
  console.log(`Verified ${Object.keys(sourcePages).length} byte-identical packaged Markdown files and installed MCP corpus resolution.`);
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
