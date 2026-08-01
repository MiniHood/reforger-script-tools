import * as assert from 'assert';
import * as fs from 'fs/promises';
import * as os from 'os';
import * as path from 'path';
import { resolveBaseGameIndexCache } from '../gameData/baseGameIndexCache';

suite('Base Game index cache resolution', () => {
	test('follows the Workbench graph to the fingerprinted parser cache', async () => {
		const root = await fs.mkdtemp(path.join(os.tmpdir(), 'reforger-search-cache-'));
		try {
			const cacheDirectory = path.join(
				root,
				'addon-indexes',
				'58D0FB3206B6F859-fingerprint',
			);
			await fs.mkdir(cacheDirectory, { recursive: true });
			await fs.mkdir(path.join(root, 'addon-sources'), { recursive: true });
			await fs.writeFile(path.join(root, 'addon-sources', 'workbench-graph-v1.json'), JSON.stringify({
				addons: [{
					guid: '58D0FB3206B6F859',
					sourceRoot: 'C:/Game/addons/data',
				}],
			}));
			await fs.writeFile(path.join(cacheDirectory, 'manifest-header.json'), JSON.stringify({
				guid: '58D0FB3206B6F859',
				sourceRoot: '\\\\?\\C:\\Game\\addons\\data',
				indexFile: 'symbols.bin',
			}));
			await fs.writeFile(path.join(cacheDirectory, 'symbols.bin'), 'index');

			assert.strictEqual(
				await resolveBaseGameIndexCache(root),
				path.join(cacheDirectory, 'symbols.bin'),
			);
		} finally {
			await fs.rm(root, { recursive: true, force: true });
		}
	});
});
