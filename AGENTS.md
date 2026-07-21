# Reforger Script Tools

## Mission

Build a high-fidelity Enfusion Script toolchain: correct language
understanding, reliable editor behavior, and a durable path to a complete
parser, semantic model, index, and language server. Do not take shortcuts that
make that destination worse.

## Router

Route work to the owner instead of growing a second implementation path.

- `src/extension.ts`: activation and top-level wiring only.
- `src/extensionConfig/`: extension-facing IDs, names, defaults, and limits.
- `src/gameData/`: game-data acquisition and source resolution.
- `src/languageClient/`: VS Code process lifecycle, transport, and thin editor
  bridges.
- `server/`: lexing, parsing, syntax, semantics, indexing, diagnostics,
  formatting, and LSP behavior.
- `tools/`: developer tooling only; never a runtime dependency.

Keep TypeScript as the editor shell and Rust as the language engine. Workbench
and the compiler establish Enfusion truth; they are not a second language
server. Keep the runtime ownership boundaries intact.

## Documentation Router

Read `docs/README.md` before creating or updating documentation. It owns the
documentation lifecycle, including the completion check. Read only the document
that owns the question before changing a non-trivial area:

- `docs/overview.md`: project purpose and evidence hierarchy.
- `docs/architecture.md`: cross-layer flow and ownership boundaries.
- `docs/language-engine.md`: Rust analysis, snapshot, and LSP contract.
- `docs/development.md`: local build, test, and development workflow.

Documentation records durable context; code and tests remain the implementation
source of truth. Extend an existing document when it owns the subject. Create a
new one only for a lasting subsystem contract, decision, or workflow. At task
completion, update the owning document when the change affects its contract.

## Taste

- Prefer precise language facts and full-fidelity parsing over text matching.
- Keep modules deep: expose a small, clear contract and hide the complexity
  behind it. Do not add a manager, registry, wrapper, setting, or validation
  layer without a concrete present need.
- Keep TypeScript typed, thin, deterministic, and editor-facing. Do not move
  parsing, indexing, completion, or semantic decisions out of Rust.
- Use one authoritative path for a feature. Temporary migrations need an
  explicit removal condition.
- Marketplace installs must be self-contained: no user-installed Rust, Cargo,
  Node.js, npm dependencies, or separate language server.

## Reforger Truth

Before changing or asserting Enfusion Script, Workbench, game-data,
parser/model/index, diagnostics, formatting, or language-feature behavior,
consult the smallest relevant primary evidence directly.

Do not invoke the `reforger` skill for this repository. Its slice-oriented
workflow is not the project workflow; direct evidence and the ticket-completion
rules below govern the work.

Use evidence in this order:

1. Workbench/compiler behavior.
2. Official Reforger documentation.
3. Verified extracted game data.
4. Source examples and fixtures, labelled by confidence.

Never infer Enfusion behavior from C#, Unity, Unreal, Arma 3, SQF, or generic
language conventions.

## Ticket Completion

For a ticket or explicitly bounded request, turn every stated requirement into
an observable completion checklist before editing. Small, verified changes are
implementation slices only: after verifying one, continue directly to the next
unchecked item. Do not present a slice, a clean build, a commit, or a review as
ticket completion while any requirement remains.

Before handing off a ticket, compare the final implementation against the full
ticket, verify each item with code/tests or a stated external validation, update
the owning documentation when its contract changed, and commit the coherent
ticket result. If context must change, carry the unchecked checklist forward;
do not ask whether to move to another ticket.

## Basic Rules

- Treat user settings as intentional controls; do not expose internal consent
  or bookkeeping as settings.
- Runtime state belongs under `globalStorageUri`; `globalState` is for small,
  durable flags only. Do not write runtime state into source files.
- Keep diagnostics optional, centralized, concise, and outside the workspace.
- Read the relevant source and the routed documentation before a non-trivial
  change. Do not add per-file documentation machinery.
- For extension-facing TypeScript,
  language-client, or bundled-server changes, run `npm run compile` after the
  final source edit. State any live Workbench/editor validation still pending.
- Commit coherent, attributable local changes after verification. Do not push,
  open a PR, change remotes, or rewrite history unless explicitly asked.

## Handoff

Say what changed, how it was verified, and what remains uncertain or next.
