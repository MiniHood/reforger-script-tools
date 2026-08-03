#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cargo = resolveCargoCommand();
const targetDirectory = resolve(repoRoot, ".cache", "cargo", "server-tests");
const result = spawnSync(
  cargo,
  [
    "test",
    "--manifest-path",
    "server/Cargo.toml",
    "--features",
    "test-hooks",
    "--lib",
    "--bins",
    "--tests",
  ],
  {
    cwd: repoRoot,
    stdio: "inherit",
    env: { ...process.env, CARGO_TARGET_DIR: targetDirectory },
  },
);

if (result.error) {
  console.error(`Failed to run Rust tests: ${result.error.message}`);
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
