# AGENTS.md

## Purpose

This repository is the VS Code extension for Reforger Script Tools. The goal is to build the most useful scripting and coding tool for Arma Reforger Enfusion Script, not the easiest extension to implement.

Agents and contributors must optimize for correctness, performance, full-fidelity language understanding, and strong editor features. Do not choose shortcuts that make a future complete AST, semantic model, indexer, or language server worse.

## Architecture

- The VS Code extension host is TypeScript. Keep it focused on activation, commands, configuration, UI glue, process management, and editor integration.
- The extension host is bundled with esbuild. Keep the JavaScript output minimal and avoid runtime dependency sprawl.
- The intended language engine is a Rust language server/parser/analyzer. It should own lexing, parsing, AST construction, semantic analysis, indexing, diagnostics, completion, go-to-definition, references, rename, formatting, and workspace intelligence.
- The extension should communicate with the language engine through clear LSP/server boundaries. Do not mix serious language intelligence directly into VS Code command glue.
- Workbench is the compiler truth and validation authority for Enfusion Script behavior. Workbench must not be treated as the LSP server.
- Official Reforger documentation, extracted APIs, samples, and Workbench/compiler behavior are the source of truth for language and engine behavior.

## Canonical Project Scope

- Treat the root project as canonical.
- Do not treat generated output, dependencies, or packaged artifacts as source truth.

## Marketplace Runtime Policy

Marketplace installs MUST be self-contained. End users should install the extension from the VS Code Marketplace and need nothing else.

- Do not require users to install Rust, Cargo, Node.js, npm packages, external LSP servers, CLI tools, or helper binaries.
- Any runtime binary, language server, library, grammar, schema, or tool required by the extension MUST be bundled in the packaged extension or acquired through extension-owned flows that do not require manual external setup.
- Development and build-time tools may use Rust, Cargo, npm, esbuild, or other local tooling, but Codex MUST design features so those tools are not user prerequisites.
- New runtime dependencies must be justified against package size, performance, security, update path, and offline marketplace-install behavior.
- Prefer no new third-party runtime dependencies. If a dependency is necessary, it must be bundled and invisible to the user at install/use time.
- Documentation, prompts, commands, and errors MUST NOT tell ordinary users to install developer tooling to make extension features work.

## Source Organization

Do not create placeholder folders for future systems. New folders MUST have a clear owner, current use, and matching documentation when non-trivial.

- `src/extension.ts`: activation only. Register top-level services and commands here. Do not put parser, model, indexing, Workbench, download, or feature logic here.
- `src/extensionConfig/`: centralized command IDs, setting keys, state keys, storage names, thresholds, repository constants, and extension-owned constants. Organize by subsystem, such as `gameData.ts`, future `logging.ts`, or future `languageClient.ts`.
- `src/gameData/`: TypeScript runtime behavior for acquiring and locating Reforger game script data. Own GitHub checks, manual-folder validation, global-storage updates, metadata, and game-data source resolution. Do not parse or semantically model Enfusion Script here.
- `src/languageClient/`: future TypeScript owner for starting, stopping, configuring, and communicating with the Rust LSP server. Own VS Code language-client glue, process lifecycle, server path resolution, and protocol wiring. Do not implement parser or analyzer logic here.
- `server/` or `crates/`: future Rust workspace for language tooling. Use `server/` only for a single focused Rust language-server project. Use `crates/` if the Rust side becomes multi-crate.
- `fixtures/`: future source examples for parser/analyzer tests only when needed. Organize by behavior, such as `fixtures/parser/`, `fixtures/model/`, `fixtures/index/`, and `fixtures/diagnostics/`.
- `tools/`: repo-only developer/Codex tooling that is not required by the runtime extension, future Rust LSP, tests, or packaged user features. Use this for game-data discovery scripts, corpus analysis, parser research scripts, one-off report generators, and investigation helpers.
- `documentation/tools/`: documentation for non-trivial tooling under `tools/`.

Dev-only tooling MUST NOT go under `src/`. Tooling may read extension/global-storage data as input, but it MUST NOT become a runtime dependency. Do not register tool commands in `package.json` unless they are intended as real user-facing extension commands. Tool-generated reports MUST be written to global storage or ignored output paths, not `src/`.

When Rust language tooling exists, organize by compiler-style responsibility:

- `lexer`: tokenization only. Own trivia, comments, strings, identifiers, keywords, operators, and source spans. Do not build AST nodes or perform semantic checks.
- `parser`: syntax parsing only. Consume tokens and produce syntax tree or AST structures. Own error recovery and parse diagnostics. Do not resolve symbols, types, inheritance, or engine APIs.
- `syntax`: shared syntax node/kind definitions if needed. Own tree shape, node IDs, source ranges, and syntax utilities. Stay independent of VS Code and Workbench.
- `ast`: typed AST wrappers over syntax tree nodes if using a green-tree/full-fidelity design. Own ergonomic accessors for declarations, classes, methods, attributes, and params. Do not perform workspace-wide semantic resolution.
- `model`: semantic model for declarations, scopes, symbols, type facts, inheritance, modifiers, attributes, and script-level relationships. Own compiler-like understanding derived from parsed source. Do not depend on VS Code APIs.
- `index`: workspace and game-data symbol indexes. Own lookup structures for definitions, references, completions, and cross-file queries. Keep user workspace data separate from downloaded game-data scripts.
- `diagnostics`: extension-generated diagnostics from parser, model, and index behavior. Distinguish extension diagnostics from Workbench/compiler truth.
- `lsp`: Language Server Protocol handlers. Own request/response mapping for completion, hover, definition, references, rename, diagnostics, and formatting. Call parser/model/index layers instead of embedding language logic.
- `formatting`: formatting rules only. Use parser/AST structures where possible, not raw string manipulation.

## Philosophy

- Build for the best final tool, not the fastest demo.
- Prefer full-fidelity parsing and precise semantic data over approximate text matching.
- Prefer explicit, durable architecture over clever local shortcuts.
- Prefer small, verified slices over broad rewrites.
- Preserve current working behavior unless the task explicitly changes it.
- Avoid abstractions until they remove real complexity or establish a necessary boundary.
- Do not assume C#, Unity, Unreal, Arma 3, or generic scripting-language behavior applies to Enfusion Script.

## Coding Practices

- Keep TypeScript thin, typed, and editor-facing.
- Keep language analysis isolated behind stable server, protocol, or service boundaries.
- Use structured parsers and data models instead of ad hoc string manipulation when working on language features.
- Keep extension activation fast. Expensive analysis belongs in the language server or a background process.
- Do not introduce broad managers, registries, wrappers, settings, or validation layers unless a concrete feature requires them.
- Follow existing repo patterns for scripts, formatting, build output, and VS Code extension conventions.
- Use ASCII in source and documentation unless a file already requires another character set.
- Keep user-facing behavior deterministic. Avoid hidden magic and implicit global state.

## Settings and State Organization

- VS Code settings are only for end-user-facing configuration that users should intentionally inspect or change.
- `context.globalState` is for small internal durable flags such as one-time approvals, dismissed warnings, and extension-owned state.
- `context.globalStorageUri` is for downloaded or cached files such as game data, logs, metadata, and future indexes.
- Source `.ts` files may define typed constants, defaults, and helpers, but runtime state MUST NOT be written into source files.
- New settings, state keys, command IDs, storage names, and extension-owned constants MUST be centralized by subsystem under `src/extensionConfig/` instead of scattered as magic strings.
- Runtime feature folders MUST import extension-facing names from `src/extensionConfig/`; do not create local constants files for settings/state/storage unless there is a concrete reason.
- Before adding a setting to `package.json`, Codex MUST justify that it is a real user-facing control. Internal consent and bookkeeping MUST NOT become visible settings unless users need to configure them directly.

## Language Tooling Boundary

- TypeScript must stay limited to VS Code integration.
- Parser, AST, semantic analysis, indexing, and serious language intelligence belong behind the future Rust/LSP boundary.
- Do not add temporary TypeScript language logic that would become competing architecture.

## Debug and Logging Philosophy

Debugging and logging are development and investigation aids, not permanent hot-path costs. They must be optional and centrally controlled. During active development, logging should be enabled by default, with a clear future path to disable, reduce, or scope it for release builds.

Runtime logs belong in VS Code global storage, not in workspace source files or packaged extension files. Use `ExtensionContext.globalStorageUri` and write logs under a dedicated `logs/` child folder, treated as `globalStorageUri/logs/`.

Debug output should be useful to Codex and human maintainers. Prefer human-reviewable input vs output records:

- What request, command, document, or action entered a subsystem.
- What decision, diagnostic, transformation, or result came out.
- What files, symbols, parser states, protocol messages, or Workbench-validation results were involved.

Avoid scattered ad hoc `console.log` calls. Logging and debugging should generally be implemented through dedicated scripts, helpers, debug commands, or debug utilities with clear ownership.

Logging must not create meaningful performance overhead:

- Check whether logging is enabled before doing expensive formatting or serialization.
- Avoid logging inside tight loops unless it is explicitly sampled, throttled, or scoped to a targeted debug command.
- Do not serialize full ASTs, indexes, documents, or Workbench output unless targeted debug mode is enabled.
- Prefer concise structured records over large unbounded text dumps.

Logs should include timestamps, subsystem names, operation names, relevant source paths, and summarized input/output. Avoid secrets, machine-specific noise, and irrelevant bulk data.

Logging ownership must stay clear:

- The TypeScript VS Code layer logs activation, commands, settings, process startup, and protocol boundaries.
- The future Rust/LSP layer logs parser, analyzer, indexer, and workspace operations through its own controlled debug path.
- Workbench validation logs record command/input, summarized output, exit status, and relevant paths.

## Piece-Meal Implementation Strategy

Every implementation plan must be broken into small slices. Each slice must state:

- Goal
- Files or subsystem touched
- Expected behavior
- Verification command or manual validation step

Prefer vertical slices that create usable behavior. Avoid large infrastructure-only rewrites unless the infrastructure is the slice's explicit deliverable and has its own verification.

Incomplete future systems must stay behind explicit boundaries. Do not blend placeholder parser, analyzer, or indexing logic into production paths as if it were complete.

## Documentation Context

Use `documentation/` as the canonical folder for internal contributor and agent context. This documentation exists to preserve durable context for Codex and human maintainers: why a file exists, what it owns, how it fits the architecture, what behavior must be preserved, and what future work is expected.

Documentation should mirror the source tree. Examples:

- `src/extension.ts` maps to `documentation/src/extension.md`
- `src/test/extension.test.ts` maps to `documentation/src/test/extension.test.md`
- Future source folders map to matching folders under `documentation/`

Before thinking through, planning, reviewing, or editing a source file, Codex MUST read the matching documentation file in full if it exists. This is mandatory context, not optional background reading. Do not rely on search snippets, partial reads, summaries, or memory for related documentation. If multiple related source files are involved, Codex MUST read each matching documentation file in full before reasoning about the change.

If a non-trivial source change has no matching documentation file, Codex MUST create it as part of the same slice. Codex MUST update the matching documentation whenever a file's purpose, architecture role, public behavior, boundaries, or future direction changes.

Do not create documentation for generated output, build artifacts, `node_modules`, or trivial metadata-only edits. Keep docs concise and useful as context, not duplicated code commentary. Human oversight is expected: documentation must be readable, reviewable, and structured for maintainers.

Every non-trivial new source folder MUST have matching folder documentation under `documentation/` explaining ownership, boundaries, and what must not be placed there. Per-file docs still mirror source files.

Each per-file documentation page should use this structure:

```markdown
# <source path>

## Purpose

What the file owns and why it exists.

## Architecture Role

How it fits into the VS Code shell, future Rust/LSP engine, Workbench validation flow, or test/build system.

## Current Behavior

The important behavior a future agent must preserve.

## Dependencies and Boundaries

Key imports, external systems, ownership limits, and what must not leak into the file.

## Change Notes

Human-readable notes for important changes, decisions, or pitfalls.

## Future Improvements

Planned features, known gaps, or follow-up ideas that should not be implemented accidentally.
```

## Verification Loop

Agents MUST follow this loop before finalizing work:

1. Inspect the current repo state before changing behavior.
2. Read every related source file in full before changing it.
3. Read every matching related documentation file in full before thinking through, planning, reviewing, or changing the related source file. Searching documentation is not a substitute for reading the full file.
4. Confirm the intended slice and avoid unrelated edits.
5. Implement one small slice.
6. Update matching documentation in the same slice when source purpose, architecture role, behavior, boundaries, or future direction changes.
7. Run the relevant checks:
   - `npm run check-types`
   - `npm run lint`
   - `npm run compile`
   - `npm test` when tests or extension behavior are affected
8. For Reforger-language behavior, validate assumptions against Workbench/compiler behavior whenever available.
9. Re-check any uncertain Reforger or Enfusion APIs against official docs, extracted APIs, or samples.
10. Record what changed, what was verified, and what remains uncertain.

If a verification step cannot be run, state why and describe the remaining risk.

## Reforger Language Truth

Workbench is the final compiler authority. The extension may provide faster diagnostics, richer navigation, and better editing support, but it must not knowingly contradict Workbench behavior.

When implementing language features:

- Treat Workbench/compiler behavior as the final answer.
- Treat official docs and extracted APIs as primary references.
- Treat samples as examples, not rule authority.
- Verify uncertain syntax, lifecycle, replication, resource, prefab, and API behavior before encoding it.
- Do not infer engine behavior from other game engines or other Arma versions.

## Fixtures and Truth

- Future parser and analyzer behavior should be backed by small Enfusion Script fixtures.
- Future `fixtures/` should exist only when parser/analyzer tests need source examples.
- Fixtures must state whether they are Workbench-confirmed, official-sample-derived, or speculative.
- Speculative behavior must not be treated as compiler truth.

## Current Project Commands

- `npm run check-types` validates TypeScript types.
- `npm run lint` runs ESLint against `src`.
- `npm run compile` runs type checks, linting, and esbuild.
- `npm test` runs the VS Code extension test flow.

Use the VS Code Extension Development Host for targeted manual validation when extension activation, commands, editor integration, or debug behavior changes.
