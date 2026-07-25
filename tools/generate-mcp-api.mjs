import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dirname, '..');
const referencePath = resolve(repositoryRoot, 'docs', 'mcp-api.md');
const check = process.argv.includes('--check');
const result = spawnSync(
	'cargo',
	[
		'run',
		'--quiet',
		'--manifest-path',
		resolve(repositoryRoot, 'server', 'Cargo.toml'),
		'--bin',
		'reforger_language_server',
		'--',
		'mcp-api',
	],
	{
		cwd: repositoryRoot,
		encoding: 'utf8',
	},
);

if (result.status !== 0) {
	process.stderr.write(result.stderr || 'MCP API generator failed.\n');
	process.exit(result.status ?? 1);
}
if (result.stderr) {
	process.stderr.write(result.stderr);
}

const generated = result.stdout.replace(/\r\n/g, '\n');
if (check) {
	let committed = '';
	try {
		committed = readFileSync(referencePath, 'utf8').replace(/\r\n/g, '\n');
	} catch {
		process.stderr.write('docs/mcp-api.md is missing; run npm run mcp-api:generate.\n');
		process.exit(1);
	}
	if (committed !== generated) {
		process.stderr.write('docs/mcp-api.md has drifted; run npm run mcp-api:generate.\n');
		process.exit(1);
	}
	process.stdout.write('MCP API reference is current.\n');
} else {
	writeFileSync(referencePath, generated, 'utf8');
	process.stdout.write('Generated docs/mcp-api.md.\n');
}
