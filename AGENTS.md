# AGENTS.md

## Mission

Build Reforger Script Tools as a high-fidelity Enfusion Script toolchain. Favor
correct language understanding, reliable editor behavior, and a durable path to
a complete parser, semantic model, index, and language server. Do not take a
shortcut that makes that destination worse.

## Architecture

The canonical runtime flow is in [docs/reference/architecture.md](docs/reference/architecture.md).
Keep these boundaries intact.

- VS Code TypeScript is the extension shell: activation, commands, settings,
  UI glue, process lifecycle, editor events, and LSP transport.
- Rust is the language engine: lexing, parsing, syntax/AST, semantic model,
  indexing, diagnostics, formatting, and LSP request handling.
- Workbench/compiler behavior is the authority for Enfusion Script behavior;
  it validates language truth but is not the extension's language server.
- Game-data acquisition and source resolution belong in `src/gameData/`.
- Language-client process management and protocol bridging belong in
  `src/languageClient/`.
- `src/extension.ts` wires top-level services only. It must not gain language
  intelligence, parser, indexer, downloader, or Workbench logic.
- `src/extensionConfig/` owns extension-facing IDs, keys, names, defaults, and
  thresholds. Do not scatter magic strings.
- `server/` owns the Rust language engine. Keep compiler-style responsibilities
  separate as the engine gains layers.
- `tools/` is developer/Codex tooling only, never an extension runtime
  dependency.
- Use one authoritative implementation path for each feature. A temporary
  migration path requires a removal condition and must not become permanent.
- Keep extension activation fast; expensive analysis belongs in Rust or a
  background process.

Marketplace installs must be self-contained. End users must not need Rust,
Cargo, Node.js, npm packages, a separately installed LSP server, or another
manual helper dependency.

## Reforger Truth

Before reasoning about or changing Enfusion Script behavior, Workbench, game
data, fixtures, parser/model/index behavior, diagnostics, formatting, or LSP
language features, invoke the `reforger` skill.

Use evidence in this order:

1. Workbench/compiler behavior when available.
2. Official Reforger documentation.
3. Extracted APIs and verified game-data records.
4. Source samples and fixtures, labelled by confidence.

Never infer Enfusion behavior from C#, Unity, Unreal, Arma 3, SQF, or generic
scripting-language conventions.

## Design Rules

- Prefer full-fidelity parsing and precise semantic data over text matching.
- Prefer small, verified vertical slices over broad speculative rewrites.
- Preserve working behavior unless the task intentionally changes it.
- Do not add managers, registries, wrappers, settings, or validation layers
  without a current, concrete need.
- Keep TypeScript typed, thin, deterministic, and editor-facing.
- Do not introduce TypeScript language logic that competes with Rust.
- Treat user settings as intentional controls; do not expose internal consent
  or bookkeeping as settings.
- Runtime state belongs under `globalStorageUri`; `globalState` contains only
  small durable flags. Never write runtime state into source files.
- Logs are optional, centrally owned, concise, and outside the workspace source
  tree. For hover-selection work, inspect
  `globalStorageUri/logs/hover-debug/latest.md` first.

## Documentation

`AGENTS.md` is strict policy. Follow [docs/documentation.md](docs/documentation.md)
for document lifecycle and verification. [docs/reference/architecture.md](docs/reference/architecture.md)
owns cross-layer architecture. [docs/agent-workflow.md](docs/agent-workflow.md)
owns workflow rationale. `docs/reference/` owns current subsystem context;
`docs/plans/` owns CE planning artifacts; `docs/solutions/` owns reusable
resolved-problem learnings.

Before changing a non-trivial or architecture-sensitive source file, read its
matching active reference page in full when one exists, plus any related page
that defines a boundary involved in the change. Update or create the matching
reference page when ownership, behavior, boundaries, or future direction
changes. Do not create reference pages for generated output, dependencies, or
trivial metadata. Replace harmful legacy documentation when current source,
accepted policy, user direction, or a CE artifact supports the replacement.

## Continuous Plan Execution

An approved plan is standing authorization to implement its entire stated
scope. When the user asks to execute, complete, finish, or work through a plan,
work every implementation unit, required verification, documentation update,
review/fix cycle, and task-scoped commit until the plan is finished.

Do not stop at checkpoints. Discovery, a partial implementation, a test pass,
a review finding, an agent result, a commit, a documentation update, or a
milestone is not a completion condition. After each one, reconcile the result
against the plan and immediately execute the next unblocked step.

A later question about status, review results, design, or progress does not
replace, pause, narrow, or complete the active plan. Answer it concisely, then
continue the plan in the same turn. A report-only CE skill limits edits only
while that review runs; it does not revoke the authorization to resume the
existing work afterward.

Ask the user only when continuing needs information or authority not supplied
by the plan: a material scope/product/design decision, destructive or external
action outside scope, unavailable credentials/access, conflicting working-tree
ownership, or evidence that invalidates a plan decision. State the concrete
evidence and the smallest decision needed. Large scope, uncertainty that can be
investigated locally, failures that can be repaired, and intermediate design
questions are not reasons to stop.

Before ending plan work, perform this terminal check: every unit is implemented,
verified, documented, reviewed, and committed; or the user explicitly
deferred it; or a genuine blocker was reported with evidence. If any unit
remains, continue work. Never present unfinished plan work as completed.

## Workflow and Git

Use Compound Engineering for non-trivial or ambiguous work when available:
`ce-brainstorm` resolves scope, `ce-plan` creates implementation-ready slices,
`ce-work` executes, `ce-code-review` reviews changed behavior, and
`ce-compound` records durable learnings. Do not mutate CE plan progress during
execution; derive progress from the working tree and verification evidence.

Use parallel agents only for independent bounded work with disjoint file
ownership or read-only review/research. Keep dependent edits, integration,
verification, and commits under one owner.

After an implementation task is fully verified and the terminal check passes,
commit its task-scoped changes to the current branch without a separate Git
instruction. Stage only attributable changes; do not absorb unrelated
pre-existing work. Use a concise, value-focused title. Do not push, open a PR,
modify remotes, or alter branches or history unless the user explicitly asks
for that operation. If the branch is unclear, verification fails, or changes
cannot be separated safely, report that blocker.

## Verification

Before completing a source-changing task:

1. Inspect repository state and relevant source/reference documentation.
2. Confirm the smallest intended slice and avoid unrelated edits.
3. Define the smallest non-overlapping verification set before editing; run it
   after the final change. Do not duplicate checks covered by a selected final
   command.
4. Validate Reforger language claims with Workbench/compiler behavior whenever
   available.
5. For Rust server, server-binary, or language-client lifecycle changes, force
   a fresh language-server process when necessary. Reload the active extension
   host after completed extension work so it uses the packaged build.
6. Update required documentation and record verification plus remaining
   uncertainty.

For documentation-only work, `git diff --check` plus manual link/path review
is sufficient when no source, package, build, or runtime behavior changed.
