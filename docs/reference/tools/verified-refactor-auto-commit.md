# tools/verified-refactor-auto-commit.mjs

## Purpose

Provides the project-local, verification-gated automatic commit protocol for Codex tasks on the `Refactor` branch. It is developer tooling and is never part of the VS Code extension runtime.

## Ownership

The helper is the evidence boundary between a task's focused verification and the trusted Codex `Stop` hook. A hook cannot safely infer whether arbitrary transcript or shell output proves the correct check passed, so `verify` executes the selected command itself before creating a Git-owned receipt. The project-local hook then invokes `stop`.

## Current Behavior

Run the final focused check through the helper:

```powershell
node tools/verified-refactor-auto-commit.mjs verify --title "update tool policy" -- git diff --check
```

The title must contain one to five plain words. Verification succeeds only on the exact `Refactor` branch and writes a receipt containing the branch, current HEAD, title, and timestamp beneath Git state. Receipts expire after ten minutes.

At task stop, the trusted hook runs:

```powershell
node tools/verified-refactor-auto-commit.mjs stop
```

`stop` commits only when the receipt is current, the branch and HEAD match, no Git operation is active, and the working tree is dirty. It stages all changes with `git add -A`, commits with the receipt title, and removes the receipt after success. It never pushes, tags, changes remotes, creates or switches branches, merges, rebases, resets, or rewrites history.

Missing, invalid, stale, mismatched, wrong-branch, clean-tree, and in-progress-operation states skip without a commit. A failed commit retains the receipt and reports the failure.

## Dependencies and Boundaries

Uses only Node built-ins and the local `git` executable. The Codex configuration in `.codex/config.toml` owns hook registration; `AGENTS.md` owns the enforcement policy. Hook trust must be reviewed and accepted through Codex's normal interactive hook flow. Do not use a trust-bypass flag.

This tool does not decide which verification a task needs. The task owner selects the final focused check using the task's verification contract.

## Verification

Run the helper's isolated-repository tests and manually verify that an invalid, stale, or wrong-branch receipt cannot create a commit.
