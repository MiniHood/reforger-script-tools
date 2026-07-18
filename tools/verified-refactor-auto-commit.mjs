#!/usr/bin/env node

import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const REQUIRED_BRANCH = "Refactor";
const RECEIPT_NAME = "reforger-verified-auto-commit.json";
const RECEIPT_MAX_AGE_MS = 10 * 60 * 1000;
const TITLE_WORD_PATTERN = /^[A-Za-z0-9][A-Za-z0-9'-]*$/;

function fail(message) {
  process.stderr.write(`auto-commit: ${message}\n`);
  process.exitCode = 1;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd,
    encoding: "utf8",
    stdio: options.capture === false ? "inherit" : "pipe",
  });

  if (result.error) {
    throw result.error;
  }

  return result;
}

function git(args, options = {}) {
  return run("git", args, options);
}

function gitOutput(args) {
  const result = git(args);
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || `git ${args.join(" ")} failed`);
  }

  return result.stdout.trim();
}

function repositoryState() {
  const root = gitOutput(["rev-parse", "--show-toplevel"]);
  return {
    root,
    branch: gitOutput(["branch", "--show-current"]),
    head: gitOutput(["rev-parse", "HEAD"]),
    receiptPath: resolve(root, gitOutput(["rev-parse", "--git-path", RECEIPT_NAME])),
  };
}

function validateTitle(title) {
  const words = title.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0 || words.length > 5 || !words.every((word) => TITLE_WORD_PATTERN.test(word))) {
    throw new Error("title must contain one to five plain words");
  }

  return words.join(" ");
}

function writeReceipt(state, title) {
  writeFileSync(state.receiptPath, `${JSON.stringify({
    version: 1,
    branch: state.branch,
    head: state.head,
    title,
    verifiedAt: new Date().toISOString(),
  }, null, 2)}\n`, "utf8");
}

function readReceipt(receiptPath) {
  try {
    return JSON.parse(readFileSync(receiptPath, "utf8"));
  } catch {
    return null;
  }
}

function isGitOperationInProgress(root) {
  const gitDirectory = dirname(gitOutput(["rev-parse", "--git-path", "HEAD"]));
  const operationFiles = ["MERGE_HEAD", "REBASE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD", "index.lock", "rebase-apply", "rebase-merge"];
  return operationFiles.some((name) => existsSync(resolve(root, gitDirectory, name)));
}

function skip(message) {
  process.stdout.write(`auto-commit: skipped - ${message}\n`);
}

function verify(argumentsAfterMode) {
  if (argumentsAfterMode[0] !== "--title") {
    throw new Error("verify requires --title <title> -- <command>");
  }

  const separatorIndex = argumentsAfterMode.indexOf("--");
  if (separatorIndex !== 2 || separatorIndex === argumentsAfterMode.length - 1) {
    throw new Error("verify requires --title <title> -- <command>");
  }

  const title = validateTitle(argumentsAfterMode[1]);
  const command = argumentsAfterMode.slice(separatorIndex + 1);
  const state = repositoryState();
  if (state.branch !== REQUIRED_BRANCH) {
    throw new Error(`verification must run on ${REQUIRED_BRANCH}`);
  }

  const result = run(command[0], command.slice(1), { cwd: state.root, capture: false });
  if (result.status !== 0) {
    process.exitCode = result.status || 1;
    return;
  }

  const currentState = repositoryState();
  if (currentState.branch !== state.branch || currentState.head !== state.head) {
    throw new Error("repository changed while verification was running");
  }

  writeReceipt(currentState, title);
  process.stdout.write(`auto-commit: armed for ${title}\n`);
}

function stop() {
  const state = repositoryState();
  if (state.branch !== REQUIRED_BRANCH) {
    skip(`branch is ${state.branch || "detached"}`);
    return;
  }

  if (!existsSync(state.receiptPath)) {
    skip("no verified receipt");
    return;
  }

  const receipt = readReceipt(state.receiptPath);
  if (!receipt || receipt.version !== 1) {
    skip("receipt is invalid");
    return;
  }

  let title;
  try {
    title = validateTitle(receipt.title);
  } catch {
    skip("receipt title is invalid");
    return;
  }

  const verifiedAt = Date.parse(receipt.verifiedAt);
  if (!Number.isFinite(verifiedAt) || verifiedAt > Date.now() || Date.now() - verifiedAt > RECEIPT_MAX_AGE_MS) {
    skip("receipt is stale");
    return;
  }

  if (receipt.branch !== state.branch || receipt.head !== state.head) {
    skip("receipt does not match HEAD");
    return;
  }

  if (isGitOperationInProgress(state.root)) {
    skip("Git operation is in progress");
    return;
  }

  if (!gitOutput(["status", "--porcelain"])) {
    skip("working tree is clean");
    return;
  }

  const addResult = git(["add", "-A"], { cwd: state.root, capture: false });
  if (addResult.status !== 0) {
    fail("could not stage working-tree changes");
    return;
  }

  const commitResult = git(["commit", "-m", title], { cwd: state.root, capture: false });
  if (commitResult.status !== 0) {
    fail("commit failed; receipt retained");
    return;
  }

  rmSync(state.receiptPath, { force: true });
  process.stdout.write(`auto-commit: committed ${title}\n`);
}

function main() {
  const [mode, ...argumentsAfterMode] = process.argv.slice(2);
  try {
    if (mode === "verify") {
      verify(argumentsAfterMode);
    } else if (mode === "stop") {
      stop();
    } else {
      throw new Error("usage: verified-refactor-auto-commit.mjs <verify|stop> ...");
    }
  } catch (error) {
    fail(error instanceof Error ? error.message : String(error));
  }
}

main();
