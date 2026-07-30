import * as assert from 'assert';
import { promises as fs } from 'fs';
import { tmpdir } from 'os';
import * as path from 'path';
import {
	publishContentAddressedFile,
	resolveRootFrom,
} from '../gameData/localSourceInventory';

suite('local source inventory', () => {
	test('uses a valid explicit root without probing discovery candidates', async () => {
		const probed: string[] = [];
		const result = await resolveRootFrom(
			'base-game',
			'D:\\Games\\Arma Reforger\\addons',
			['C:\\standard'],
			async candidate => {
				probed.push(candidate);
				return candidate.startsWith('D:');
			},
		);

		assert.equal(result.status, 'ready');
		assert.equal(result.origin, 'configured');
		assert.deepEqual(probed, ['D:\\Games\\Arma Reforger\\addons']);
	});

	test('reports an invalid explicit root without silently falling back', async () => {
		const result = await resolveRootFrom(
			'workbench',
			'D:\\Missing\\addons',
			['C:\\standard'],
			async candidate => candidate === 'C:\\standard',
		);

		assert.equal(result.status, 'invalid');
		assert.equal(result.origin, 'configured');
		assert.match(result.diagnostic, /does not exist/);
	});

	test('selects the first existing standard candidate deterministically', async () => {
		const result = await resolveRootFrom(
			'user-addons',
			undefined,
			['C:\\first', 'D:\\second', 'E:\\third'],
			async candidate => candidate !== 'C:\\first',
		);

		assert.equal(result.status, 'ready');
		assert.equal(result.origin, 'discovered');
		assert.equal(result.path, 'D:\\second');
		assert.deepEqual(result.candidates, ['C:\\first', 'D:\\second', 'E:\\third']);
	});

	test('publishes one complete inventory under concurrent writers', async () => {
		const root = await fs.mkdtemp(path.join(tmpdir(), 'rst-inventory-'));
		const target = path.join(root, 'inventory-v1-digest.json');
		const contents = '{"schema":"test"}\n';
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

	test('rejects a corrupt existing content-addressed inventory', async () => {
		const root = await fs.mkdtemp(path.join(tmpdir(), 'rst-inventory-corrupt-'));
		const target = path.join(root, 'inventory-v1-digest.json');
		try {
			await fs.writeFile(target, 'truncated');
			await assert.rejects(
				publishContentAddressedFile(target, '{"schema":"test"}\n'),
				/corrupt/,
			);
		} finally {
			await fs.rm(root, { recursive: true, force: true });
		}
	});
});
