#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const release = process.argv.includes("--release");
const cargo = resolveCargoCommand();
const platformArch = `${process.platform}-${process.arch}`;
const executableName = process.platform === "win32" ? "reforger_language_server.exe" : "reforger_language_server";
const profile = release ? "release" : "debug";
const sourceBinary = resolve(repoRoot, "server", "target", profile, executableName);
const targetFolder = resolve(repoRoot, "dist", "server", platformArch);
const targetBinary = resolve(targetFolder, executableName);

const cargoArgs = [
  "build",
  ...(release ? ["--release"] : []),
  "--manifest-path",
  "server/Cargo.toml",
  "--bin",
  "reforger_language_server",
];

const result = spawnSync(cargo, cargoArgs, {
  cwd: repoRoot,
  stdio: "inherit",
});

if (result.error) {
  console.error(`Failed to build language server: ${result.error.message}`);
  process.exit(1);
}

if ((result.status ?? 1) !== 0) {
  process.exit(result.status ?? 1);
}

mkdirSync(targetFolder, { recursive: true });
copyFileSync(sourceBinary, targetBinary);
if (process.platform !== "win32") {
  chmodSync(targetBinary, 0o755);
}

console.log(`Copied language server binary: ${targetBinary}`);

function resolveCargoCommand() {
  if (process.platform === "win32" && process.env.USERPROFILE) {
    const userCargo = resolve(process.env.USERPROFILE, ".cargo", "bin", "cargo.exe");
    if (existsSync(userCargo)) {
      return userCargo;
    }
  }

  return "cargo";
}
