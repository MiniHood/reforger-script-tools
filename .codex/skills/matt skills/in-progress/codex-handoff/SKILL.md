---
name: codex-handoff
description: Hand the current conversation off to a fresh Codex background agent that picks up the work immediately.
---

Write a handoff summary of the current conversation so a fresh Codex agent can continue the work. Launch a background agent seeded with the summary as its prompt using the available collaboration tool. It starts with the current working-directory context and returns immediately.

Always give the background agent a descriptive name, for example `fix-login-bug`.

Include a "suggested skills" section in the summary, which suggests skills that the agent should invoke.

Do not duplicate content already captured in other artifacts (PRDs, plans, ADRs, issues, commits, diffs). Reference them by path or URL instead.

Redact sensitive information, such as API keys, passwords, or personally identifiable information; the summary becomes the agent's prompt.

If the user passed arguments, treat them as a description of what the next session will focus on and tailor the summary accordingly.
