import * as assert from 'assert';
import {
	buildMcpLaunchConfiguration,
	renderCodexMcpConfiguration,
	renderGenericMcpConfiguration,
} from '../mcp/mcpConfiguration';

suite('MCP configuration', () => {
	test('builds a stable launch independent of a running VS Code process', () => {
		const launch = buildMcpLaunchConfiguration({
			serverPath: 'C:\\Extensions\\reforger_language_server.exe',
			gameDataScripts: 'D:\\Reforger\\scripts',
			gameDataMetadata: 'C:\\Storage\\metadata.json',
			indexCache: 'C:\\Storage\\index-cache\\game-data-symbol-index.v12.bin',
		});

		assert.deepStrictEqual(launch, {
			command: 'C:\\Extensions\\reforger_language_server.exe',
			args: [
				'mcp',
				'--game-data-scripts',
				'D:\\Reforger\\scripts',
				'--game-data-metadata',
				'C:\\Storage\\metadata.json',
				'--index-cache',
				'C:\\Storage\\index-cache\\game-data-symbol-index.v12.bin',
			],
		});
	});

	test('omits unavailable optional Game Data inputs without inventing paths', () => {
		const launch = buildMcpLaunchConfiguration({
			serverPath: '/extension/reforger_language_server',
			gameDataScripts: undefined,
			gameDataMetadata: undefined,
			indexCache: '/storage/index-cache/game-data-symbol-index.v12.bin',
		});

		assert.deepStrictEqual(launch.args, [
			'mcp',
			'--index-cache',
			'/storage/index-cache/game-data-symbol-index.v12.bin',
		]);
	});

	test('renders generic JSON and Codex TOML from the same launch', () => {
		const launch = {
			command: 'C:\\Extension\\reforger_language_server.exe',
			args: ['mcp', '--index-cache', 'C:\\Storage\\cache.bin'],
		};

		const generic = JSON.parse(renderGenericMcpConfiguration(launch));
		assert.deepStrictEqual(generic, {
			mcpServers: {
				'reforger-script-tools': launch,
			},
		});

		const codex = renderCodexMcpConfiguration(launch);
		assert.match(codex, /^\[mcp_servers\.reforger-script-tools\]/m);
		assert.match(codex, /command = "C:\\\\Extension\\\\reforger_language_server\.exe"/);
		assert.match(codex, /args = \["mcp", "--index-cache", "C:\\\\Storage\\\\cache\.bin"\]/);
		assert.match(codex, /startup_timeout_sec = 120\.0/);
		assert.match(codex, /tool_timeout_sec = 130\.0/);
	});

	test('regenerates configuration with the current packaged runtime after an extension upgrade', () => {
		const previous = renderCodexMcpConfiguration(buildMcpLaunchConfiguration({
			serverPath: 'C:\\Users\\Gray\\.vscode\\extensions\\burn0ut7.reforger-script-tools-1.0.1\\dist\\server\\win32-x64\\reforger_language_server.exe',
			gameDataScripts: undefined,
			gameDataMetadata: undefined,
			indexCache: 'C:\\Storage\\index-cache.bin',
		}));
		const current = renderCodexMcpConfiguration(buildMcpLaunchConfiguration({
			serverPath: 'C:\\Users\\Gray\\.vscode\\extensions\\burn0ut7.reforger-script-tools-1.0.2\\dist\\server\\win32-x64\\reforger_language_server.exe',
			gameDataScripts: undefined,
			gameDataMetadata: undefined,
			indexCache: 'C:\\Storage\\index-cache.bin',
		}));

		assert.doesNotMatch(current, /1\.0\.1/);
		assert.match(current, /1\.0\.2/);
		assert.notStrictEqual(current, previous);
	});
});
