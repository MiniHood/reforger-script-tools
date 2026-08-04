import { defineConfig } from '@vscode/test-cli';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig({
	files: 'out/test/mcpCleanWindowAcceptance.acceptance.js',
	launchArgs: [
		'--disable-workspace-trust',
		`--user-data-dir=${path.join(repositoryRoot, '.vscode-test', 'mcp-user-data')}`,
	],
	mocha: { timeout: 15_000 },
});
