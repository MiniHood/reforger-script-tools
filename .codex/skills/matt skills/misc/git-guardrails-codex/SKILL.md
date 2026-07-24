---
name: git-guardrails-codex
description: Set up Codex git guardrails for dangerous commands such as push, reset --hard, clean, and branch -D. Use when the user wants Git safety rules in a Codex workspace.
---

# Set Up Codex Git Guardrails

Add explicit git-safety rules to the repository's `AGENTS.md`. Codex follows these instructions while working in the repository; this workflow does not claim to install an automatic command hook.

## What to protect

- `git push`, especially force push
- `git reset --hard`
- `git clean -f` and `git clean -fd`
- `git branch -D`
- `git checkout .` and `git restore .`

## Steps

1. Read the existing `AGENTS.md`; preserve all user-authored instructions.
2. Ask whether the rules apply to this repository only or should also be copied to the user's global Codex instructions.
3. Add or update a concise `## Git safety` section. Require explicit user authorization before the protected commands and prohibit history rewrites unless the user requests them.
4. If a deterministic manual check is useful, copy [scripts/block-dangerous-git.sh](scripts/block-dangerous-git.sh) to `.codex/scripts/` and document how to run it before a git command. Do not present it as an enforcement hook.
5. Ask whether the user wants to add or remove protected patterns.
6. Verify that the instructions retain every existing rule and that the optional script blocks a representative protected command.
