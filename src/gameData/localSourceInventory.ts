import * as fs from "fs/promises";
import * as path from "path";
import { createHash, randomUUID } from "crypto";
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
  const digest = createHash("sha256").update(contents).digest("hex");
  const inventoryPath = path.join(
    context.globalStorageUri.fsPath,
    gameDataStorage.rootFolder,
    `${gameDataStorage.inventoryPrefix}${digest}.json`,
  );
  const serializeAndHash = Date.now() - serializeStartedAt;
  const publishStartedAt = Date.now();
  await publishContentAddressedFile(inventoryPath, contents);
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

export async function publishContentAddressedFile(
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
  try {
    await fs.link(temporaryPath, targetPath);
  } catch (error) {
    if (!isAlreadyExists(error)) {
      throw error;
    }
    const existing = await fs.readFile(targetPath, "utf8");
    if (existing !== contents) {
      throw new Error(
        `Content-addressed Workbench add-on graph is corrupt: ${targetPath}`,
      );
    }
  } finally {
    await fs.rm(temporaryPath, { force: true });
  }
}

function isAlreadyExists(error: unknown): boolean {
  return error instanceof Error && "code" in error && error.code === "EEXIST";
}
