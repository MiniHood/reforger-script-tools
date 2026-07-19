---
name: fix
description: Diagnose, design, implement, and verify the best durable fix for a Reforger Script Tools defect. Use when the user invokes /fix or asks Codex to solve an extension, language-server, parser, index, game-data, or editor issue comprehensively rather than applying a narrow workaround.
---

# Fix Reforger Script Tools

Deliver the smallest durable solution, not the quickest patch. Preserve the
TypeScript-shell/Rust-engine boundary and avoid new abstraction layers unless
the evidence proves they are needed.

## Establish the Problem

1. State the observed behavior, expected behavior, reproduction, affected
   layer, and whether it is a regression.
2. Read `AGENTS.md`, `git status`, the active reference page for each affected
   subsystem, relevant recent history, and the current implementation.
3. If the causal chain is not already proven, invoke `/debug` first. Inspect
   the smallest relevant logs, cursor reports, test, or reproduction.
4. Before making Enfusion Script, Workbench, or game API claims, invoke
   `reforger`, query the exact extracted API data, and inspect relevant game
   source examples. Do not infer language behavior from another engine.

## Choose the Best Design

1. Identify the failed invariant and the authoritative owner layer.
2. Inspect adjacent paths sharing the same abstraction, data source, lifecycle,
   parser rule, or protocol request. Search for analogous callers and existing
   guards before changing code.
3. Compare plausible fixes against correctness, revision safety, performance,
   memory, failure modes, package/runtime constraints, and architectural fit.
4. Select one implementation path. Do not retain a workaround or parallel path
   without a concrete removal condition.
5. Record the key invariant and rejected unsafe scope in the matching reference
   documentation when the behavior or ownership contract changes.

## Implement and Generalize

1. Add the narrowest failing regression test before changing behavior.
2. Implement in the owning layer. Keep UI/process glue thin and keep language
   understanding in Rust.
3. Review the complete affected family, not only the reported example:
   - first, intermediate, and terminal positions or lifecycle states;
   - valid, malformed, incomplete, and boundary-sized inputs;
   - inheritance/overloads/source precedence where applicable;
   - pending, cached, refreshed, and cancellation/stale-revision paths;
   - performance limits, allocations, request-thread work, and logging impact.
4. Add focused tests for meaningful siblings that the new invariant covers.
   Reject unsupported forms safely; do not guess semantic facts.
5. Update the relevant reference page and comments only where they clarify the
   durable contract.

## Verify and Hand Off

1. Run the focused regression test, then the smallest complete affected suite.
2. For Rust server, binary, or language-client changes, run `cargo test`, then
   run `npm run compile` to stop any active development server and replace the
   bundled development binary. After a successful build, force the active
   Extension Development Host to reload so it starts a fresh server process
   from that binary; use the available VS Code reload command rather than
   asking the user to perform a manual refresh. For extension-only changes,
   also reload the active Extension Development Host after typecheck, lint,
   and relevant extension tests. Do not report live verification until the
   refreshed host has received the rebuilt extension.
3. Run `git diff --check`; inspect the final diff for scope, duplicated paths,
   and unrelated user edits. Commit only attributable changes after coherent
   verification. Do not push or open a PR without explicit authorization.
4. State what was worked on, the selected design and why, verification, and
   remaining uncertainty or required Workbench/editor validation.
