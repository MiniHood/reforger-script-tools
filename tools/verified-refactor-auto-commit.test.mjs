import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { mkdtempSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const toolPath = fileURLToPath(new URL("./verified-refactor-auto-commit.mjs", import.meta.url));

function run(command, argumentsList, cwd) {
  return spawnSync(command, argumentsList, { cwd, encoding: "utf8" });
}

function git(argumentsList, cwd) {
  const result = run("git", argumentsList, cwd);
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}

function createRepository() {
  const directory = mkdtempSync(join(tmpdir(), "verified-auto-commit-"));
  git(["init", "-b", "Refactor"], directory);
  git(["config", "user.email", "test@example.com"], directory);
  git(["config", "user.name", "Test User"], directory);
  writeFileSync(join(directory, "tracked.txt"), "before\n");
  writeFileSync(join(directory, "deleted.txt"), "remove me\n");
  git(["add", "-A"], directory);
  git(["commit", "-m", "initial"], directory);
  return directory;
}

function receiptPath(directory) {
  return join(directory, ".git", "reforger-verified-auto-commit.json");
}

function invoke(directory, ...argumentsList) {
  return run(process.execPath, [toolPath, ...argumentsList], directory);
}

function prepareChanges(directory) {
  writeFileSync(join(directory, "tracked.txt"), "after\n");
  writeFileSync(join(directory, "untracked.txt"), "new\n");
  rmSync(join(directory, "deleted.txt"));
}

test("verified receipt commits all working-tree changes on Refactor", () => {
  const directory = createRepository();
  prepareChanges(directory);

  const verify = invoke(directory, "verify", "--title", "update project files", "--", "git", "diff", "--check");
  assert.equal(verify.status, 0, verify.stderr);
  assert.ok(existsSync(receiptPath(directory)));

  const stop = invoke(directory, "stop");
  assert.equal(stop.status, 0, stop.stderr);
  assert.equal(git(["log", "-1", "--format=%s"], directory), "update project files");
  assert.equal(readFileSync(join(directory, "tracked.txt"), "utf8"), "after\n");
  assert.equal(readFileSync(join(directory, "untracked.txt"), "utf8"), "new\n");
  assert.equal(existsSync(join(directory, "deleted.txt")), false);
  assert.equal(git(["status", "--porcelain"], directory), "");
  assert.equal(existsSync(receiptPath(directory)), false);
});

test("failed verification does not arm or commit", () => {
  const directory = createRepository();
  prepareChanges(directory);

  const verify = invoke(directory, "verify", "--title", "failed check", "--", process.execPath, "-e", "process.exit(3)");
  assert.equal(verify.status, 3);
  assert.equal(existsSync(receiptPath(directory)), false);

  const stop = invoke(directory, "stop");
  assert.equal(stop.status, 0, stop.stderr);
  assert.equal(git(["log", "-1", "--format=%s"], directory), "initial");
});

test("receipt guards reject wrong branch, mismatched state, stale state, Git operations, and clean trees", () => {
  const directory = createRepository();
  prepareChanges(directory);
  assert.equal(invoke(directory, "verify", "--title", "guard test", "--", "git", "diff", "--check").status, 0);

  git(["checkout", "-b", "other"], directory);
  assert.equal(invoke(directory, "stop").status, 0);
  assert.equal(git(["log", "-1", "--format=%s"], directory), "initial");
  git(["checkout", "Refactor"], directory);

  const receipt = JSON.parse(readFileSync(receiptPath(directory), "utf8"));
  receipt.head = "0".repeat(40);
  writeFileSync(receiptPath(directory), `${JSON.stringify(receipt)}\n`);
  assert.equal(invoke(directory, "stop").status, 0);
  assert.equal(git(["log", "-1", "--format=%s"], directory), "initial");

  receipt.head = git(["rev-parse", "HEAD"], directory);
  receipt.verifiedAt = "2000-01-01T00:00:00.000Z";
  writeFileSync(receiptPath(directory), `${JSON.stringify(receipt)}\n`);
  assert.equal(invoke(directory, "stop").status, 0);
  assert.equal(git(["log", "-1", "--format=%s"], directory), "initial");

  receipt.verifiedAt = new Date().toISOString();
  writeFileSync(receiptPath(directory), `${JSON.stringify(receipt)}\n`);
  writeFileSync(join(directory, ".git", "MERGE_HEAD"), "pending\n");
  assert.equal(invoke(directory, "stop").status, 0);
  assert.equal(git(["log", "-1", "--format=%s"], directory), "initial");
  rmSync(join(directory, ".git", "MERGE_HEAD"));

  git(["restore", "."], directory);
  git(["clean", "-fd"], directory);
  assert.equal(invoke(directory, "stop").status, 0);
  assert.equal(git(["log", "-1", "--format=%s"], directory), "initial");
});

test("titles are bounded and commit failures retain the receipt", () => {
  const directory = createRepository();
  prepareChanges(directory);

  const invalidTitle = invoke(directory, "verify", "--title", "this title now has six words", "--", "git", "diff", "--check");
  assert.notEqual(invalidTitle.status, 0);
  assert.equal(existsSync(receiptPath(directory)), false);

  assert.equal(invoke(directory, "verify", "--title", "five words are allowed here", "--", "git", "diff", "--check").status, 0);
  mkdirSync(join(directory, ".git", "hooks"), { recursive: true });
  writeFileSync(join(directory, ".git", "hooks", "pre-commit"), "#!/bin/sh\nexit 1\n");
  const stop = invoke(directory, "stop");
  assert.notEqual(stop.status, 0);
  assert.equal(existsSync(receiptPath(directory)), true);
  assert.equal(git(["log", "-1", "--format=%s"], directory), "initial");
});
