#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cargo = resolveCargoCommand();
const parsedArgs = parseWrapperArgs(process.argv.slice(2));

const result = spawnSync(
  cargo,
  [
    "run",
    ...(parsedArgs.release ? ["--release"] : []),
    "--manifest-path",
    "server/Cargo.toml",
    "--example",
    "lsp_hover_corpus_report",
    "--",
    "--profile-label",
    parsedArgs.release ? "release" : parsedArgs.profileLabel,
    ...parsedArgs.passthroughArgs,
  ],
  {
    cwd: repoRoot,
    stdio: "inherit",
    shell: process.platform === "win32",
  },
);

if (result.error) {
  console.error(`Failed to run LSP hover corpus report: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 1);

function parseWrapperArgs(args) {
  const passthroughArgs = [];
  let release = false;
  let profileLabel = "debug";

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--release") {
      release = true;
      continue;
    }

    if (arg === "--profile-label") {
      const value = args[index + 1];
      if (!value) {
        console.error("--profile-label requires a value");
        process.exit(1);
      }
      profileLabel = value;
      index += 1;
      continue;
    }

    passthroughArgs.push(arg);
  }

  return { release, profileLabel, passthroughArgs };
}

function resolveCargoCommand() {
  if (process.platform === "win32" && process.env.USERPROFILE) {
    const userCargo = resolve(process.env.USERPROFILE, ".cargo", "bin", "cargo.exe");
    if (existsSync(userCargo)) {
      return userCargo;
    }
  }

  return "cargo";
}
