import * as assert from "assert";
import { promises as fs } from "fs";
import { tmpdir } from "os";
import * as path from "path";
import type * as vscode from "vscode";
import {
  clearConfirmedLoadedAddonSourceInventory,
  confirmLoadedAddonSourceInventory,
  loadedAddonSourceInventoryIsConfirmed,
  onDidConfirmLoadedAddonSourceInventory,
  publishAtomicFile,
  writeLoadedAddonSourceInventory,
} from "../gameData/localSourceInventory";

suite("Workbench loaded add-on inventory", () => {
  test("publishes confirmation only after the language server accepts the graph", () => {
    const inventoryPath = path.join(tmpdir(), "confirmed-workbench-graph.json");
    let confirmedPath: string | undefined;
    const subscription = onDidConfirmLoadedAddonSourceInventory((value) => {
      confirmedPath = value;
    });
    try {
      assert.equal(loadedAddonSourceInventoryIsConfirmed(inventoryPath), false);
      confirmLoadedAddonSourceInventory(inventoryPath);
      assert.equal(loadedAddonSourceInventoryIsConfirmed(inventoryPath), true);
      assert.equal(confirmedPath, path.resolve(inventoryPath));
      clearConfirmedLoadedAddonSourceInventory();
      assert.equal(loadedAddonSourceInventoryIsConfirmed(inventoryPath), false);
      assert.equal(confirmedPath, undefined);
    } finally {
      subscription.dispose();
    }
  });

  test("publishes one complete graph under concurrent writers", async () => {
    const root = await fs.mkdtemp(path.join(tmpdir(), "rst-workbench-graph-"));
    const target = path.join(root, "workbench-graph-v1.json");
    const contents = '{"schema":"reforger-workbench-loaded-addon-graph-v1"}\n';
    try {
      await Promise.all(
        Array.from({ length: 8 }, () =>
          publishAtomicFile(target, contents),
        ),
      );
      assert.equal(await fs.readFile(target, "utf8"), contents);
      assert.deepEqual(
        (await fs.readdir(root)).filter((file) => file.endsWith(".tmp")),
        [],
      );
    } finally {
      await fs.rm(root, { recursive: true, force: true });
    }
  });

  test("publishes the Workbench graph without touching retired storage", async () => {
    const root = await fs.mkdtemp(path.join(tmpdir(), "rst-workbench-graph-"));
    const retiredFiles = [
      path.join(root, "game-data", "legacy-index.bin"),
      path.join(root, "index-cache", "legacy-index.bin"),
      path.join(root, "addon-sources", "inventory-v1.json"),
    ];
    try {
      await Promise.all(
        retiredFiles.map(async (retiredFile) => {
          await fs.mkdir(path.dirname(retiredFile), { recursive: true });
          await fs.writeFile(retiredFile, "retired");
        }),
      );
      await fs.writeFile(
        path.join(root, "ArmaReforger.gproj"),
        'GameProject { GUID "58D0FB3206B6F859" }',
      );
      const published = await writeLoadedAddonSourceInventory(
        {
          globalStorageUri: { fsPath: root },
        } as unknown as vscode.ExtensionContext,
        {
          bridgeVersion: "1.52.0",
          protocolVersion: 1,
          addons: [
            {
              guid: "58D0FB3206B6F859",
              id: "ArmaReforger",
              title: "Arma Reforger",
              sourceRoot: "C:\\addons\\data",
            },
          ],
        },
      );
      await Promise.all(
        retiredFiles.map(async (retiredFile) => {
          assert.equal(await fs.readFile(retiredFile, "utf8"), "retired");
        }),
      );
      assert.ok((await fs.stat(published.path)).isFile());
    } finally {
      await fs.rm(root, { recursive: true, force: true });
    }
  });

  test("publishes the exact roots returned by the completed Workbench graph", async () => {
    const root = await fs.mkdtemp(path.join(tmpdir(), "rst-workbench-graph-"));
    try {
      const sourceRoot = path.join(root, "GC-Suppression");
      await fs.mkdir(sourceRoot);
      const published = await writeLoadedAddonSourceInventory(
        { globalStorageUri: { fsPath: root } } as unknown as vscode.ExtensionContext,
        {
          bridgeVersion: "1.52.0",
          protocolVersion: 1,
          addons: [{ guid: "684CE8AA3B1D6573", id: "GCSuppression", title: "GC Suppression", sourceRoot }],
        },
      );
      const inventory = JSON.parse(await fs.readFile(published.path, "utf8"));
      assert.equal(inventory.addons[0].sourceRoot, sourceRoot);
    } finally {
      await fs.rm(root, { recursive: true, force: true });
    }
  });

});
