import * as assert from 'assert';
import { promises as fs } from 'fs';
import { tmpdir } from 'os';
import * as path from 'path';
import { publishContentAddressedFile } from '../gameData/localSourceInventory';

suite('Workbench loaded add-on inventory', () => {
	test('publishes one complete graph under concurrent writers', async () => {
		const root = await fs.mkdtemp(path.join(tmpdir(), 'rst-workbench-graph-'));
		const target = path.join(root, 'workbench-graph-v1-digest.json');
		const contents = '{"schema":"reforger-workbench-loaded-addon-graph-v1"}\n';
		try {
			await Promise.all(
				Array.from({ length: 8 }, () => publishContentAddressedFile(target, contents)),
			);
			assert.equal(await fs.readFile(target, 'utf8'), contents);
			assert.deepEqual(
				(await fs.readdir(root)).filter(file => file.endsWith('.tmp')),
				[],
			);
		} finally {
			await fs.rm(root, { recursive: true, force: true });
		}
	});
});
