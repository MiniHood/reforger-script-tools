#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..');
const cargo = process.env.CARGO
	?? (process.platform === 'win32' && process.env.USERPROFILE
		? path.join(process.env.USERPROFILE, '.cargo', 'bin', 'cargo.exe')
		: 'cargo');

const args = process.argv.slice(2);
const cargoArgs = [
	'run',
	'--manifest-path',
	path.join(repoRoot, 'server', 'Cargo.toml'),
	'--example',
	'lsp_workspace_overlay_report',
	'--',
	...args,
];

const result = spawnSync(cargo, cargoArgs, {
	cwd: repoRoot,
	stdio: 'inherit',
	shell: process.platform === 'win32',
});

process.exit(result.status ?? 1);
