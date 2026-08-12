---
name: reforger-gauntlet
description: Run a user-supplied Arma Reforger goal through parallel implementation and independent evidence gates. Use when a demanding Reforger task needs persistent iteration, harsh review, live Workbench proof, or coordinated subagents.
---

# Reforger Gauntlet

Treat the invocation text as `GOAL`. If it is empty, ask for one sentence.

Model map: `loop` means a Codex goal; `subagent` means Luna at high reasoning; `ultracode` means `gpt-5.6-sol` at xhigh. If Luna is unavailable, use the available balanced model at high and report the substitution once.

1. **Set the bar.** Create a goal for `GOAL`, read the workspace instructions, and apply `$reforger`. Convert superlatives into observable acceptance checks backed by the strongest available Reforger evidence. Finish when every check has a proof method.
2. **Fan out.** Split independent work into small cards with one owner, one acceptance check, and non-overlapping files or explicit integration ownership. Dispatch cards in parallel up to the available slots; keep shared integration work with the coordinator. Finish when every card has an owner and evidence contract.
3. **Run the gauntlet.** Each worker loops: inspect evidence, make the smallest coherent change, verify it, and return raw artifacts. A separate harsh critic receives the card, acceptance check, and artifacts without the worker's conclusions. It returns `ACCEPT` only when the artifacts prove the check; otherwise it returns `REJECT` with the exact evidence gap. Route rejection back to the worker until accepted or genuinely blocked.
4. **Integrate.** Inspect the combined diff and run every repository-required build, test, documentation, live Workbench, commit, and push gate. Mark the goal complete only when every card and the integrated result are accepted.

For visual checks, inspect real captures through the supported Workbench route. Make side-by-side claims only when both lawful reference material and implementation captures exist; the critic's verdict must cite visible differences.
