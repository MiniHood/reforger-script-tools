import { spawnSync } from 'node:child_process';
import {
	existsSync,
	mkdirSync,
	readdirSync,
	readFileSync,
	unlinkSync,
	writeFileSync,
} from 'node:fs';
import { resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dirname, '..');
const referencePath = resolve(repositoryRoot, 'docs', 'mcp-api.md');
const contractsPath = resolve(repositoryRoot, 'docs', 'mcp-api', 'tools');
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
		'mcp-api-bundle',
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

let bundle;
try {
	bundle = JSON.parse(result.stdout);
} catch {
	process.stderr.write('MCP API generator returned an invalid document bundle.\n');
	process.exit(1);
}
if (
	typeof bundle?.reference !== 'string' ||
	bundle?.contracts === null ||
	typeof bundle?.contracts !== 'object' ||
	Array.isArray(bundle?.contracts)
) {
	process.stderr.write('MCP API generator returned an incomplete document bundle.\n');
	process.exit(1);
}

const generatedReference = bundle.reference.replace(/\r\n/g, '\n');
const generatedContracts = new Map();
for (const [name, contract] of Object.entries(bundle.contracts)) {
	if (!/^[a-z0-9_]+$/.test(name) || typeof contract !== 'string') {
		process.stderr.write(`MCP API generator returned an invalid contract: ${name}.\n`);
		process.exit(1);
	}
	generatedContracts.set(`${name}.md`, contract.replace(/\r\n/g, '\n'));
}

function contractEntries() {
	if (!existsSync(contractsPath)) {
		return [];
	}
	return readdirSync(contractsPath, { withFileTypes: true });
}

if (check) {
	let committed = '';
	try {
		committed = readFileSync(referencePath, 'utf8').replace(/\r\n/g, '\n');
	} catch {
		process.stderr.write('docs/mcp-api.md is missing; run npm run mcp-api:generate.\n');
		process.exit(1);
	}
	if (committed !== generatedReference) {
		process.stderr.write('docs/mcp-api.md has drifted; run npm run mcp-api:generate.\n');
		process.exit(1);
	}
	const entries = contractEntries();
	const actualNames = new Set(entries.map((entry) => entry.name));
	for (const entry of entries) {
		if (!entry.isFile() || !generatedContracts.has(entry.name)) {
			process.stderr.write(
				`Unexpected MCP API contract entry: docs/mcp-api/tools/${entry.name}.\n`,
			);
			process.exit(1);
		}
	}
	for (const [fileName, generated] of generatedContracts) {
		if (!actualNames.has(fileName)) {
			process.stderr.write(
				`Missing MCP API contract: docs/mcp-api/tools/${fileName}.\n`,
			);
			process.exit(1);
		}
		const committedContract = readFileSync(
			resolve(contractsPath, fileName),
			'utf8',
		).replace(/\r\n/g, '\n');
		if (committedContract !== generated) {
			process.stderr.write(
				`MCP API contract has drifted: docs/mcp-api/tools/${fileName}.\n`,
			);
			process.exit(1);
		}
	}
	process.stdout.write(
		`MCP API reference and ${generatedContracts.size} tool contracts are current.\n`,
	);
} else {
	mkdirSync(contractsPath, { recursive: true });
	for (const entry of contractEntries()) {
		if (!entry.isFile() || !entry.name.endsWith('.md')) {
			process.stderr.write(
				`Refusing to replace unexpected MCP API contract entry: docs/mcp-api/tools/${entry.name}.\n`,
			);
			process.exit(1);
		}
		if (!generatedContracts.has(entry.name)) {
			unlinkSync(resolve(contractsPath, entry.name));
		}
	}
	writeFileSync(referencePath, generatedReference, 'utf8');
	for (const [fileName, generated] of generatedContracts) {
		writeFileSync(resolve(contractsPath, fileName), generated, 'utf8');
	}
	process.stdout.write(
		`Generated docs/mcp-api.md and ${generatedContracts.size} tool contracts.\n`,
	);
}
