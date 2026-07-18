# AGENTS.md

## Purpose

This repository is the VS Code extension for Reforger Script Tools.
The goal is to build the most useful scripting and coding tool for Arma Reforger Enfusion Script, not the easiest extension to implement.

Agents and contributors must optimize for correctness, performance, full-fidelity language understanding, and strong editor features.
Do not choose shortcuts that make a future complete AST, semantic model, indexer, or language server worse.

## Operating Contract

- Treat the root project as canonical.
- Do not treat generated output, dependencies, packaged artifacts, or downloaded data as source truth.
- Preserve current working behavior unless the task explicitly changes it.
- Prefer small verified slices over broad rewrites.
- Prefer one authoritative implementation path per feature. Temporary migration paths require a removal plan.
- Do not introduce managers, registries, wrappers, settings, or validation layers unless the current slice requires them.
- Do not infer Enfusion Script or Reforger behavior from C#, Unity, Unreal, Arma 3, SQF, or generic scripting-language behavior.

## Compound Engineering Workflow

Use Compound Engineering for non-trivial or ambiguous work when available:

- Use `ce-brainstorm` to settle what to build, scope boundaries, success criteria, and non-goals.
- Use `ce-plan` to turn settled scope into implementation-ready slices with verification.
- Use `ce-work`, `ce-debug`, `ce-code-review`, `ce-resolve-pr-feedback`, and commit/PR skills for execution and shipping when they match the task.
- Store CE plan artifacts under `docs/plans/`. These artifacts describe scoped work; they do not replace source-mirror documentation.
- Do not mutate CE plan progress during execution. Progress comes from git, verification results, and shipped commits.

If a contributor or agent does not have CE tooling, they must provide equivalent scoped requirements, an implementation plan, and verification notes before broad, ambiguous, or architectural changes.

For routing rationale, examples, and document ownership, read [docs/agent-workflow.md](docs/agent-workflow.md).

## Cost-Aware Agent Routing

Classify every task before delegation from observable scope and risk. Do not select a model by preference.

| Risk class | Exact classification |
|---|---|
| Bounded | One or two files, established local pattern, clear acceptance behavior, no public contract, no uncertain API, no semantic core, and focused verification available. |
| Normal | Behavior-bearing work with understood architecture, normal rollback, and adequate tests; may span several files but does not change a semantic or cross-subsystem contract. |
| Consequential | Parser, AST, semantic model, index, diagnostics semantics, formatting semantics, LSP contracts, process lifecycle, packaging/runtime acquisition, security, data-loss risk, difficult rollback, weak verification, or exact Reforger behavior encoded into tooling. |
| Critical | Explicitly identified before dispatch as unusually expensive to get wrong, or a consequential Sol High task that failed substantively after receiving exact evidence and one correction opportunity. |

Use this automatic route matrix. Explicit per-run user overrides remain authoritative.

| Route | Model and effort | Responsibility |
|---|---|---|
| Root/default, `explorer`, `worker`, `reforger-researcher` | GPT-5.6 Terra High | Normal orchestration, discovery, implementation, and Reforger evidence research. |
| `quick-implementer`, `doc-maintainer`, `commit-pusher` | GPT-5.6 Luna High | Strictly bounded implementation/docs and explicitly authorized git work. |
| `high-risk-implementer`, `code-reviewer`, `reforger-reviewer` | GPT-5.6 Sol High | Consequential implementation, the assigned project review, and Reforger truth review. |
| `recovery-implementer` | GPT-5.6 Sol XHigh | Work classified as critical before dispatch or one controlled recovery after substantive Sol High failure. |
| `code-validator` | GPT-5.4 Mini Low | Independent execution of specified checks only; no diagnosis or source edits. |

Sol Max is manual-only. Ultra is never automatic. Luna Low/Medium/XHigh/Max, Terra Low/Medium/XHigh/Max, Sol Low/Medium/Max, GPT-5.5, and full GPT-5.4 have no automatic project role. Delegation is limited to one level and four concurrent threads.

- Reclassify a bounded task before continuing when its scope expands. Normal work moves to Terra; consequential work moves directly to Sol High.
- After exact evidence and one correction opportunity, a substantive Sol High failure permits one Sol XHigh recovery. Stop automatic escalation after XHigh failure.
- Tool, environment, dependency, flaky-test, credential, and model-availability failures never raise the reasoning tier. An unavailable configured model must fail visibly and be recorded; required checks may run inline, but no agent model may be silently substituted.
- Writers perform their own focused verification inline. Independent Mini validation is automatic after normal and consequential implementation and optional for bounded work; validation is not a second review.
- Give each review question exactly one owner: either the project `code-reviewer` or the applicable CE review workflow. Never dispatch both automatically. CE-owned generic reviewers and semantic model tiers remain CE-managed; use named project roles only where the workflow supports them.
- Reforger evidence work must invoke the `reforger` skill. Consequential language, API, replication, or Workbench conclusions also require independent Sol High `reforger-reviewer` review; Workbench/compiler remains final authority.
- Commit and push actions require explicit user authorization. A plan, workflow, route, or model assignment is not authorization.

The parent creates a content-free attempt observation at dispatch and finalizes it for every terminal state. It assigns one work ID, a unique attempt ID, and a positive sequence number; retries and recovery keep the work ID and increment the sequence. It records outcomes serially under ignored `tools/reports/`; workers never write shared observation files. Usage, latency, token, and reported-cost values are optional and must never be estimated.

Hold the first routing review after at least 30 completed delegations and at least five samples for every high-volume role, defined as at least 10% of completed delegations at review time. Record its completed-delegation checkpoint and review again every 50 additional completed delegations or after three similar failures within ten comparable tasks. Every routing review must include a blinded, stratified classification audit and inspect every critical/recovery record regardless of sample size. Reports without trustworthy usage or cost data are quality-only and cannot support cost-efficiency routing changes. Use `ce-optimize` for ambiguous comparisons and `ce-compound` only for accepted durable lessons.

## Mandatory Reforger Grounding

Use the `reforger` skill before reasoning about or changing any of the following:

- Arma Reforger or Workbench behavior
- Enfusion Script syntax, semantics, APIs, attributes, callbacks, lifecycle, replication, resources, prefabs, configs, or UI
- game data, extracted APIs, official samples, or source-backed examples
- lexer, parser, AST, model, resolver, index, diagnostics, formatting, hover, completion, definition, references, rename, semantic tokens, or LSP behavior for Enfusion Script
- fixtures, corpus reports, or validation data for Reforger language behavior

Workbench/compiler behavior is the final authority.
Official Reforger documentation, extracted APIs, verified game-data records, source examples, and Workbench/compiler behavior are primary evidence.
Examples guide idioms but do not override verified symbols, signatures, or compiler behavior.

## Architecture Boundaries

- The VS Code extension host is TypeScript. Keep it focused on activation, commands, configuration, UI glue, process management, and editor integration.
- The extension host is bundled with esbuild. Keep JavaScript output minimal and avoid runtime dependency sprawl.
- Real language intelligence belongs in the Rust language server/parser/analyzer, not in VS Code command glue.
- The Rust side owns lexing, parsing, AST construction, semantic analysis, indexing, diagnostics, completion, go-to-definition, references, rename, formatting, and workspace intelligence.
- The extension communicates with the language engine through clear LSP/server boundaries.
- Workbench is the compiler truth and validation authority. Workbench must not be treated as the LSP server.

## Marketplace Runtime Policy

Marketplace installs must be self-contained.
End users should install the extension from the VS Code Marketplace and need nothing else.

- Do not require users to install Rust, Cargo, Node.js, npm packages, external LSP servers, CLI tools, helper binaries, or developer tools.
- Any runtime binary, language server, library, grammar, schema, or tool required by the extension must be bundled in the packaged extension or acquired through extension-owned flows that do not require manual external setup.
- Development and build-time tools may use Rust, Cargo, npm, esbuild, or other local tooling, but those tools must not become user prerequisites.
- New runtime dependencies must be justified against package size, performance, security, update path, and offline marketplace-install behavior.
- Prefer no new third-party runtime dependencies.
- Documentation, prompts, commands, and errors must not tell ordinary users to install developer tooling to make extension features work.

## Source Organization

Do not create placeholder folders for future systems.
New folders must have a clear owner, current use, and matching documentation when non-trivial.

- `src/extension.ts`: activation only. Register top-level services and commands here.
- `src/extensionConfig/`: centralized command IDs, setting keys, state keys, storage names, thresholds, repository constants, and extension-owned constants.
- `src/gameData/`: TypeScript runtime behavior for acquiring and locating Reforger game script data. Do not parse or semantically model Enfusion Script here.
- `src/languageClient/`: TypeScript owner for starting, stopping, configuring, and communicating with the Rust LSP server.
- `server/`: Rust language server/parser/analyzer workspace unless the Rust side grows into a multi-crate `crates/` layout.
- `tools/fixtures/`: repo-only source examples for lexer/parser/analyzer/model/index/diagnostics tests and language-tooling research.
- `tools/`: repo-only developer/Codex tooling. Tooling must not become a runtime dependency.
- `docs/reference/`: internal contributor and agent context mirroring source ownership and behavior.
- `tools/reports/`: ignored generated output from developer report tools. It is not active reference context and must not be committed as a source-mirror page.
- `docs/plans/`: CE planning artifacts only.

Dev-only tooling must not go under `src/`.
Tool-generated reports must be written to global storage or ignored output paths, not `src/`.

When Rust language tooling is organized by compiler responsibility:

- `lexer` tokenizes only.
- `parser` parses syntax only.
- `syntax` owns shared node/kind/range definitions.
- `ast` owns typed AST wrappers.
- `model` owns semantic declarations, scopes, symbols, type facts, inheritance, modifiers, attributes, and relationships.
- `index` owns workspace and game-data symbol lookup structures.
- `diagnostics` owns extension-generated parser/model/index diagnostics.
- `lsp` owns protocol handlers and maps requests to parser/model/index layers.
- `formatting` owns formatting rules using parser/AST structures where possible.

## Documentation Policy

Use `docs/reference/` as the canonical folder for durable contributor and agent context: why a source file exists, what it owns, how it fits the architecture, what behavior must be preserved, and what future work is expected.
The name of a source file does not determine its documentation tier: a page for `server/examples/*_report.rs` remains active reference context because it documents the report generator, while the generator's output belongs under ignored `tools/reports/`.

Before planning, reviewing, or editing a non-trivial or architecture-sensitive source file, read its matching active reference page in full when one exists.
Read related active reference pages when they establish an ownership boundary needed by the change.
Searching snippets is not a substitute for a required full read.

For this policy, a source change is non-trivial when it changes executed logic, a command, a setting, a data contract, parser/LSP behavior, a process/tool flow, or a subsystem boundary.
It is architecture-sensitive when it touches `server/`, `src/languageClient/`, `src/gameData/`, `src/extensionConfig/`, `tools/fixtures/`, or creates a non-trivial source folder.

Use this routing rule:

| Work | Required documentation action |
|---|---|
| Non-trivial or architecture-sensitive source change | Read the matching active page under `docs/reference/` when it exists; create or update it when ownership, behavior, boundaries, or future direction changes. |
| Change crossing a subsystem boundary | Read the matching page plus the active subsystem/folder pages needed to understand that boundary. |
| Trivial metadata-only edit, generated output, dependency artifact, or formatting-only mechanical edit | Do not create or read a mirror page solely by ritual. Read existing active context only when deciding the source owner, a preserved behavior, or an explicit boundary. |
| Investigating generated report output | Read the relevant tool/source-owner page under `docs/reference/`; inspect ignored `tools/reports/` output only when the investigation needs it. |
| Missing or stale reference page | Create, rewrite, move, or delete it only through the evidence gate below. Do not preserve misleading architecture merely for legacy continuity. |

Documentation should mirror source paths:

- `src/extension.ts` maps to `docs/reference/src/extension.md`
- `src/test/extension.test.ts` maps to `docs/reference/src/test/extension.test.md`
- source folders map to matching folders under `docs/reference/`

If a non-trivial or architecture-sensitive source change has no matching documentation file, create it in the same slice when a durable ownership page would remain useful.
Update matching documentation whenever a file's purpose, architecture role, public behavior, boundaries, or future direction changes.
Do not create documentation for generated output, build artifacts, `node_modules`, or trivial metadata-only edits.

Documentation may be rewritten or deleted when it preserves harmful legacy architecture, but only when the change is grounded in current source behavior, current accepted policy, explicit user direction, or an applicable CE artifact that is current, names the affected path or subsystem, and does not contradict current source behavior, accepted policy, or source-mirror ownership docs.
A CE artifact is historical only when a newer artifact supersedes its scope or current source/policy contradicts it; a plan never overrides source truth merely because it exists.
Preserve replacement context in the correct owner, or explicitly record why no replacement context is needed.

For a documentation-root migration, produce a per-file old-to-new mapping before moving files and verify every deleted source path has exactly one expected replacement before staging or committing.
Do not commit a migration that leaves old documentation deleted while its replacement pages are merely untracked.

Per-file documentation pages should use this structure unless a file has a better established local pattern:

```markdown
# <source path>

## Purpose

## Architecture Role

## Current Behavior

## Dependencies and Boundaries

## Change Notes

## Future Improvements
```

## Settings, State, and Logging

- VS Code settings are only for end-user-facing configuration that users should intentionally inspect or change.
- `context.globalState` is for small internal durable flags.
- `context.globalStorageUri` is for downloaded or cached files such as game data, logs, metadata, and future indexes.
- New settings, state keys, command IDs, storage names, and extension-owned constants belong under `src/extensionConfig/`.
- Runtime feature folders must import extension-facing names from `src/extensionConfig/`.
- Runtime logs belong under `globalStorageUri/logs/`, not workspace source files or packaged extension files.
- Avoid scattered ad hoc `console.log` calls.
- Check `globalStorageUri/logs/hover-debug/latest.md` first when investigating hover selection, cursor position, lexer/parser/AST/model/index output, or symbol display issues.

## Fixtures and Truth

- Parser and analyzer behavior should be backed by small Enfusion Script fixtures.
- Fixtures belong under `tools/fixtures/` unless a packaged runtime feature explicitly needs them elsewhere.
- Fixtures must state whether they are Workbench-confirmed, official-sample-derived, or speculative.
- Speculative behavior must not be treated as compiler truth.

## Verification Loop

Before finalizing repository work:

1. Inspect the current repo state.
2. Read every related source file in full before changing it.
3. Follow the Documentation Policy: read every required, relevant active reference page in full before reasoning about or changing the related source file.
4. Confirm the intended slice and avoid unrelated edits.
5. Implement one small slice.
6. Update matching documentation when source purpose, architecture role, behavior, boundaries, or future direction changes.
7. Run relevant checks.
8. For Reforger-language behavior, validate assumptions against Workbench/compiler behavior whenever available.
9. Re-check uncertain Reforger or Enfusion APIs against official docs, extracted APIs, or samples.
10. Record what changed, what was verified, and what remains uncertain.

Docs-only changes may use `git diff --check` plus manual link/path review when no source, package, build, or runtime behavior changes.
This docs-only exception does not weaken the verification loop for code or behavior-bearing changes.

## Current Project Commands

- `npm run check-types` validates TypeScript types.
- `npm run lint` runs ESLint against `src`.
- `npm run compile` runs type checks, linting, and esbuild.
- `npm test` runs the VS Code extension test flow.

Run `npm test` when tests or extension behavior are affected.
Use the VS Code Extension Development Host for targeted manual validation when extension activation, commands, editor integration, or debug behavior changes.
