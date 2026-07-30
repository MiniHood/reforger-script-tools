import * as fs from "fs/promises";
import * as path from "path";
import { randomUUID } from "crypto";
import * as vscode from "vscode";
import type { WorkbenchLoadedAddonGraph } from "../workbenchNetApi/gateway/workbenchGateway";
import { gameDataStorage } from "../extensionConfig/gameData";

export interface LoadedAddonSourceInventory {
  schema: "reforger-workbench-loaded-addon-graph-v1";
  bridgeVersion: string;
  protocolVersion: 1;
  addons: WorkbenchLoadedAddonGraph["addons"];
}

export interface PublishedLoadedAddonSourceInventory {
  path: string;
  timingsMs: {
    serializeAndHash: number;
    publish: number;
    total: number;
  };
  bytes: number;
}

/**
 * Publishes the exact graph which Workbench reports for this process. The
 * graph is the complete Workbench-owned source of add-on identity and roots.
 */
export async function writeLoadedAddonSourceInventory(
  context: vscode.ExtensionContext,
  graph: WorkbenchLoadedAddonGraph,
): Promise<PublishedLoadedAddonSourceInventory> {
  const startedAt = Date.now();
  const serializeStartedAt = Date.now();
  const inventory: LoadedAddonSourceInventory = {
    schema: "reforger-workbench-loaded-addon-graph-v1",
    bridgeVersion: graph.bridgeVersion,
    protocolVersion: graph.protocolVersion,
    addons: graph.addons,
  };
  const contents = `${JSON.stringify(inventory, null, 2)}\n`;
  const inventoryPath = path.join(
    context.globalStorageUri.fsPath,
    gameDataStorage.rootFolder,
    gameDataStorage.inventoryFile,
  );
  const serializeAndHash = Date.now() - serializeStartedAt;
  const publishStartedAt = Date.now();
  await publishAtomicFile(inventoryPath, contents);
  await pruneRetiredGraphFiles(path.dirname(inventoryPath));
  return {
    path: inventoryPath,
    timingsMs: {
      serializeAndHash,
      publish: Date.now() - publishStartedAt,
      total: Date.now() - startedAt,
    },
    bytes: Buffer.byteLength(contents, "utf8"),
  };
}

export async function publishAtomicFile(
  targetPath: string,
  contents: string,
): Promise<void> {
  await fs.mkdir(path.dirname(targetPath), { recursive: true });
  const temporaryPath = path.join(
    path.dirname(targetPath),
    `.${path.basename(targetPath)}.${process.pid}.${randomUUID()}.tmp`,
  );
  const handle = await fs.open(temporaryPath, "wx");
  try {
    await handle.writeFile(contents, { encoding: "utf8" });
    await handle.sync();
  } finally {
    await handle.close();
  }
  await fs.rename(temporaryPath, targetPath);
}

async function pruneRetiredGraphFiles(directory: string): Promise<void> {
  const current = gameDataStorage.inventoryFile;
  const entries = await fs.readdir(directory);
  await Promise.all(
    entries
      .filter(
        (entry) =>
          entry !== current &&
          entry.startsWith("workbench-graph-") &&
          entry.endsWith(".json"),
      )
      .map((entry) => fs.rm(path.join(directory, entry), { force: true })),
  );
}
