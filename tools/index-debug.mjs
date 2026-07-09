#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cargo = resolveCargoCommand();

const result = spawnSync(
  cargo,
  [
    "run",
    "--manifest-path",
    "server/Cargo.toml",
    "--example",
    "index_debug",
    "--",
    ...process.argv.slice(2),
  ],
  {
    cwd: repoRoot,
    stdio: "inherit",
    shell: process.platform === "win32",
  },
);

if (result.error) {
  console.error(`Failed to run index debug: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 1);

function resolveCargoCommand() {
  if (process.platform === "win32" && process.env.USERPROFILE) {
    const userCargo = resolve(process.env.USERPROFILE, ".cargo", "bin", "cargo.exe");
    if (existsSync(userCargo)) {
      return userCargo;
    }
  }

  return "cargo";
}
