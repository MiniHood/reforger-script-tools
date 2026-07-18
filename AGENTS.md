# AGENTS.md

## Purpose

Build Reforger Script Tools as a high-fidelity Enfusion Script toolchain. Optimize for correct language understanding, reliable editor behavior, and a durable path to a complete parser, semantic model, index, and language server. Do not choose shortcuts that make that target worse.

## Architecture Policy

The canonical runtime flow is documented in [docs/reference/architecture.md](docs/reference/architecture.md). Keep these ownership boundaries intact:

- VS Code TypeScript is the extension shell: activation, commands, settings, UI glue, process lifecycle, editor events, and LSP transport.
- Rust is the language engine: lexing, parsing, syntax/AST, semantic model, indexing, diagnostics, formatting, and LSP request handling.
- Workbench/compiler behavior is the final authority for Enfusion Script behavior. It is validation authority, not the extension's language server.
- Game-data acquisition and source resolution belong in `src/gameData/`; parsing or semantic modeling does not.
- Language-client process management and protocol bridging belong in `src/languageClient/`; language intelligence does not.
- Use one authoritative implementation path for each feature. Temporary migration paths require an explicit removal plan.
- Keep extension activation fast. Expensive analysis belongs in the Rust server or a background process.

Marketplace installs must be self-contained. Never require end users to install Rust, Cargo, Node.js, npm packages, an LSP server, or another helper tool. Runtime dependencies must be bundled or acquired through extension-owned flows without manual setup.

## Reforger Truth Policy

Before reasoning about or changing Arma Reforger, Workbench, Enfusion Script, game data, source-backed APIs, fixtures, parser/model/index behavior, diagnostics, formatting, or LSP language features, invoke the `reforger` skill.

Use this evidence order:

1. Workbench/compiler behavior when available.
2. Official Reforger documentation.
3. Extracted APIs and verified game-data records.
4. Source samples and fixtures, labeled by confidence.

Do not infer Enfusion behavior from C#, Unity, Unreal, Arma 3, SQF, or generic scripting-language conventions.

## Source And State Boundaries

- `src/extension.ts` wires top-level services only. Do not add parser, indexer, Workbench, downloader, or feature logic there.
- `src/extensionConfig/` owns extension-facing IDs, keys, names, defaults, and thresholds. Do not scatter magic strings.
- `server/` owns the Rust language engine. Organize it by compiler-style responsibility when introducing new layers.
- `tools/` is developer/Codex tooling only and must not become an extension runtime dependency.
- `tools/fixtures/` contains small language-tooling examples and states whether each is Workbench-confirmed, source-derived, or speculative.
- `context.globalState` holds small durable flags. `context.globalStorageUri` holds downloaded data, caches, metadata, and logs. Do not write runtime state into source files.
- Logs must be optional, centrally owned, concise, and outside the workspace source tree. Check `globalStorageUri/logs/hover-debug/latest.md` first for hover-selection investigations.

## Design And Implementation Policy

- Prefer full-fidelity parsing and precise semantic data over text matching.
- Prefer small verified vertical slices over broad rewrites.
- Preserve current working behavior unless the task explicitly changes it.
- Do not add managers, registries, wrappers, settings, or validation layers without a current concrete need.
- Keep TypeScript typed, thin, deterministic, and editor-facing.
- Use structured parsers and data models for language features; do not introduce temporary TypeScript language logic that competes with Rust.
- Treat user settings as intentional user controls. Keep internal consent and bookkeeping out of `package.json` settings unless users must configure them.

## Documentation Policy

`AGENTS.md` is strict policy. [docs/reference/architecture.md](docs/reference/architecture.md) is the architecture overview. [docs/agent-workflow.md](docs/agent-workflow.md) owns workflow and routing rationale. `docs/reference/` owns source and subsystem context; `docs/plans/` owns CE planning artifacts.

Before changing a non-trivial or architecture-sensitive source file, read its matching active reference page in full when it exists. Read related reference pages when they define a boundary involved in the change. Create or update the matching page when ownership, behavior, boundaries, or future direction changes.

Do not create reference pages for generated output, build artifacts, dependencies, or trivial metadata changes. Rewrite or remove harmful legacy documentation when current source, accepted policy, explicit user direction, or a current CE artifact supports the change; preserve useful replacement context in the correct owner.

## Workflow And Git Policy

Use Compound Engineering for non-trivial or ambiguous work when available: `ce-brainstorm` settles scope, `ce-plan` creates implementation-ready slices, and the matching CE execution, debug, review, or shipping skill handles the task. Do not mutate plan progress during execution.

Classify delegated work before dispatch: bounded, normal, consequential, or critical. `.codex/config.toml` is the executable source of truth for configured roles and model effort; [docs/agent-workflow.md](docs/agent-workflow.md) explains classification and escalation. Do not silently substitute unavailable configured models. Limit delegation to one level and four concurrent threads.

Git operations require active user authorization. The only exception is the verified auto-commit protocol in [tools/verified-refactor-auto-commit.mjs](tools/verified-refactor-auto-commit.mjs): after its final focused check passes, the trusted Codex `Stop` hook may commit all working-tree changes on the exact `Refactor` branch. It must never push, alter remotes, or create, switch, merge, rebase, reset, delete, or rewrite branches/history. Review and trust project hooks through Codex's normal flow; never bypass hook trust.

## Verification Policy

Before finalizing work:

1. Inspect the current repository state and read related source/reference documentation.
2. Confirm the smallest intended slice and avoid unrelated edits.
3. Run focused checks for changed behavior. Run `npm run check-types`, `npm run lint`, `npm run compile`, and `npm test` when their affected surface requires them.
4. Validate Reforger-language claims with Workbench/compiler behavior whenever available.
5. Update matching documentation where required and record verification plus remaining uncertainty.

For docs-only work, `git diff --check` plus manual link/path review is sufficient when no source, package, build, or runtime behavior changed.
