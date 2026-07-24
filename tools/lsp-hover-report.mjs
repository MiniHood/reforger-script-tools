#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

const cargo = resolveCargo();
const args = process.argv.slice(2);
const cargoArgs = [
  "run",
  "--manifest-path",
  "server/Cargo.toml",
  "--example",
  "lsp_hover_report",
  "--",
  ...args,
];

const result = spawnSync(cargo, cargoArgs, {
  stdio: "inherit",
  shell: false,
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);

function resolveCargo() {
  const windowsCargo = join(homedir(), ".cargo", "bin", "cargo.exe");
  if (process.platform === "win32" && existsSync(windowsCargo)) {
    return windowsCargo;
  }
  return "cargo";
}
