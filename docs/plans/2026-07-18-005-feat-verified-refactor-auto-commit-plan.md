---
title: Verified Refactor Auto-Commit - Plan
type: feat
date: 2026-07-18
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# Verified Refactor Auto-Commit - Plan

## Goal Capsule

| Field | Plan |
| --- | --- |
| Objective | Automatically commit all working-tree changes on `Refactor` after the task's focused verification succeeds. |
| Product authority | User-directed: use only `Refactor`, include all changes, generate a short surface-level title, and commit only after focused verification. Existing policy keeps pushes explicit-only. |
| Execution profile | Project-local Codex hook, Node dev-only helper, focused tests, and policy/documentation updates. No extension runtime behavior. |
| Stop conditions | Stop instead of committing when the branch is not `Refactor`, verification fails, no changes exist, Git is mid-operation, the verification receipt is stale, or the commit itself fails. |

## Product Contract

### Summary

Add a verified auto-commit protocol for this repository. A task owner runs one focused verification command through a project-owned helper. Only a successful result arms a short-lived receipt. A trusted Codex `Stop` hook consumes that receipt, stages all working-tree changes, and commits them on `Refactor` with a short title.

The hook must not infer verification from a transcript or from arbitrary shell output. Codex documents transcript structure as unstable, so the helper is the explicit evidence boundary.

### Requirements

- R1. The protocol must commit only when the current branch is exactly `Refactor`; it must never create, switch, merge, rebase, or delete branches.
- R2. A successful verified task commits every current working-tree change, including tracked, untracked, modified, and deleted files.
- R3. The helper must execute the designated focused verification command itself and arm a receipt only when that command exits successfully.
- R4. The `Stop` hook must consume only a fresh receipt for the current `Refactor` HEAD. It must skip visibly when the receipt is missing, stale, mismatched, or invalid.
- R5. The generated commit title must be a surface-level description of no more than five words. It must not include task prompts, source snippets, paths, or verbose bodies.
- R6. The protocol must never push, tag, change remotes, rewrite history, or bypass Git's normal commit failures.
- R7. If focused verification fails, no commit may be attempted. If a commit fails, the failure and uncommitted state must remain visible for the next user action.
- R8. Existing explicit-request-only push policy remains unchanged. The user has explicitly authorized this narrow automatic commit protocol for `Refactor` only.

### Scope Boundaries

- In scope: project-local trusted Codex hook configuration, a Node helper, focused helper tests, policy/agent instructions, and matching reference documentation.
- Out of scope: Git hooks that run outside Codex, automatic pushes, commits outside `Refactor`, branch orchestration, commit-message bodies, remote operations, and extension runtime changes.
- Out of scope: treating arbitrary successful commands or transcript text as proof that a task's required verification passed.

### Acceptance Examples

- AE1. A docs-only task runs the helper with `git diff --check`; on success, stopping the task commits all changes on `Refactor` with a title such as `fix routing rule`.
- AE2. A focused verification command fails; no receipt is armed and the stop hook makes no commit.
- AE3. The same receipt is used after switching away from `Refactor`; the hook skips and leaves the tree untouched.
- AE4. A fresh receipt exists but the tree has no changes; the hook skips without creating an empty commit.
- AE5. A task has manual edits and agent edits; a valid receipt stages and commits both because the user selected all working-tree changes.
- AE6. A successful commit does not push or create any branch.

## Planning Contract

### Key Technical Decisions

- KTD1. Use a project-local Codex `Stop` hook plus a project-owned verification helper. The hook cannot safely infer arbitrary verification from transcripts, so the helper provides explicit evidence.
- KTD2. Guard the protocol to the exact branch name `Refactor`.
- KTD3. Stage all changes with `git add -A`. (session-settled: user-directed - chosen over agent-only changes: include all working-tree changes.)
- KTD4. Use a short generated title of at most five words and no commit body. (session-settled: user-directed - chosen over user-supplied messages: surface-level title only.)
- KTD5. Keep pushing explicit-request-only. The requested automation is restricted to commits; this also preserves the repository's existing remote-operation policy.

### Receipt Lifecycle

```text
task owner -> helper executes focused verification
  success -> receipt: branch, HEAD, title, timestamp
  failure -> no receipt, no commit

Codex Stop hook -> validate receipt and repository state
  valid + dirty Refactor -> git add -A -> git commit -> remove receipt
  otherwise -> visible skip or failure; no branch or remote action
```

### Risks And Mitigations

- All changes are intentionally broad: the user chose to include manual and agent edits. Mitigation: require a valid fresh receipt and exact `Refactor` branch before staging.
- A stale receipt could commit later unrelated work. Mitigation: bind it to branch and HEAD, impose a short expiry, and consume it only after a successful commit.
- Codex hooks require review and trust. Mitigation: document the one-time hook-trust step and verify that Codex lists the hook before relying on it.
- A verification command can be too narrow for the task. Mitigation: the task owner selects the focused command from the task's verification contract; the helper only proves that selected command passed.
- Git identity, locks, hooks, or conflicts can reject commits. Mitigation: do not hide failures or delete the receipt before a successful commit.

## Implementation Units

### U1. Build Verified Commit Helper

- **Goal:** Create a deterministic Node helper that runs focused verification, writes a bounded receipt, validates it at stop time, and commits only when every guard passes.
- **Requirements:** R1-R7; AE1-AE6; KTD2-KTD4
- **Dependencies:** None
- **Files:** `tools/verified-refactor-auto-commit.mjs`, `tools/verified-refactor-auto-commit.test.mjs`
- **Approach:** Support a verification mode that accepts a short title and executes one command as the final focused check. On success, store a receipt under Git-owned state with branch, HEAD, timestamp, and title. Support a stop mode that validates the receipt, exact branch, repository operation state, freshness, and dirty tree before `git add -A` and `git commit`. Never call push or branch-changing commands.
- **Test scenarios:** Successful docs verification commits tracked/untracked/deleted changes; failed verification creates no receipt; wrong branch, stale/mismatched receipt, clean tree, and simulated commit failure do not commit; valid title boundaries accept five words and reject longer/content-bearing titles; successful commit consumes the receipt.
- **Verification:** Run the focused Node test file in temporary Git repositories and inspect command history/state to prove no push or branch-changing command is issued.

### U2. Register Trusted Stop Hook And Protocol Policy

- **Goal:** Make the verified helper the only automatic-commit path for trusted Codex sessions on `Refactor`.
- **Requirements:** R1, R3-R8; AE1-AE6; KTD1, KTD5
- **Dependencies:** U1
- **Files:** `.codex/config.toml`, `.codex/agents/commit-pusher.toml`, `AGENTS.md`, `docs/agent-workflow.md`
- **Approach:** Register one project-local `Stop` command hook that invokes the helper's stop mode from the Git root. Update policy so task owners arm the receipt only after their focused verification passes; allow the resulting commit despite the usual explicit-request-only rule because this user-approved protocol is narrower and mechanically guarded. Preserve the absolute prohibition on automatic push and branch changes. Keep manual hook trust as a one-time setup requirement rather than bypassing trust.
- **Test scenarios:** Strict Codex configuration accepts the hook declaration; the hook is visible for review/trust; policy assigns no automatic commit outside `Refactor`; direct `commit-pusher` behavior remains explicit-request-only for pushes and other Git operations.
- **Verification:** Run `codex --strict-config exec --help`, inspect the hook configuration in Codex, and manually trigger one valid stop receipt in an isolated test repository before relying on it in this workspace.

### U3. Document The User-Facing Workflow

- **Goal:** Preserve the exact protocol and failure behavior for future agents and maintainers.
- **Requirements:** R3-R8; AE1-AE6
- **Dependencies:** U1, U2
- **Files:** `docs/reference/tools/verified-refactor-auto-commit.md`, `docs/solutions/workflow-issues/avoid-disproportionate-agent-workflows.md`
- **Approach:** Document how a task chooses and executes its focused verification, supplies a five-word-or-fewer title, arms the receipt, and receives an automatic commit only at task stop. Extend the proportional-workflow learning with this narrow automation as an example of a valid mechanical post-verification action, not a reason to expand review scope.
- **Test scenarios:** Documentation distinguishes commit authorization from push authorization, names every skip/failure condition, and uses repo-relative links.
- **Verification:** Validate links, run `git diff --check`, and confirm the helper reference matches the tested CLI.

## Verification Contract

| Check | Applies To | Done Signal |
| --- | --- | --- |
| Focused helper tests | U1 | Receipt and commit guards pass in temporary Git repositories. |
| Strict Codex config validation | U2 | Project hook configuration is accepted. |
| Hook trust inspection | U2 | The stop hook is visible and explicitly trusted, never bypassed. |
| Isolated end-to-end hook smoke | U1-U2 | A passing focused command produces one `Refactor` commit with no push or branch change. |
| Documentation and path review | U3 | Policy and tool reference agree on branch, verification, title, and no-push boundaries. |
| `git diff --check` | U1-U3 | No whitespace or patch-format errors. |

## Definition Of Done

- A passing focused task on `Refactor` commits all current working-tree changes exactly once with a title of five words or fewer.
- Failed, missing, stale, mismatched, or wrong-branch receipts never create a commit.
- No automatic path pushes, switches branches, creates branches, tags, rebases, merges, or rewrites history.
- The hook is trusted through Codex's normal review flow and is not run through a trust bypass.
- Tests cover the receipt lifecycle and Git safety guards.
- Policy and source-mirror documentation explain the protocol and its limitations.
