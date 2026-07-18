# Agent Workflow

## Purpose

This document explains how agent policy, Compound Engineering artifacts, source-mirror documentation, and Reforger evidence fit together in this repository.
[AGENTS.md](../AGENTS.md) is the active policy contract; this file is rationale and orientation for maintainers and future agents.

## Document Ownership

`AGENTS.md` owns the rules that must be obeyed during repository work.
It should stay short, direct, and enforceable.
When a rule needs background, tradeoffs, or examples, put that explanation here or in the matching subsystem documentation instead of expanding `AGENTS.md`.

[Architecture overview](reference/architecture.md) owns the canonical runtime data flow and layer boundaries. It is the starting point for cross-subsystem architecture work; matching `docs/reference/<source path>.md` pages remain the authority for file-level behavior.

`docs/reference/` owns durable source and subsystem context.
Its job is to explain why files exist, what they own, what behavior must be preserved, and what boundaries future work must respect.
It mirrors the source tree so agents can find relevant context before editing.

`tools/reports/` owns ignored generated report output.
It is a developer-output path, not a documentation tier and not a source-mirror home.
Report-like names do not change ownership: `docs/reference/server/examples/parser_report.md` documents the `server/examples/parser_report.rs` generator and remains active source context; `tools/reports/parser-fixtures.report.md` is generated output for investigation.

`docs/plans/` owns Compound Engineering plan artifacts.
Plans capture scoped work: Product Contract, Planning Contract, implementation units, verification contract, and definition of done.
Plans are not source ownership docs and should not be copied into `docs/reference/`.

`CONCEPTS.md` may be introduced later by CE compound workflows if the project develops stable vocabulary that needs glossary-level capture.
It should define project-specific terms, not replace plans or source-mirror docs.

## Documentation Routing

Use active reference docs deliberately rather than treating every Markdown file as mandatory context.

| Situation | Read or create documentation? |
|---|---|
| Non-trivial or architecture-sensitive source change | Read the matching active reference page in full when it exists. Create or update it when the change affects ownership, behavior, boundaries, or future direction. |
| Change that crosses a subsystem boundary | Read the matching page and the active subsystem/folder context needed to understand the boundary. |
| Trivial metadata, generated output, dependency artifact, or formatting-only mechanical edit | No mirror page is required solely because a file changed. Consult active context only when it answers a real ownership or behavior question. |
| Investigation of report output | Read the source-owner page under `docs/reference/`; inspect ignored `tools/reports/` output only when the investigation needs evidence from that run. |
| Page that is missing, stale, duplicative, or harmful | Use the evidence gate before creating, rewriting, moving, or deleting it. |

The current documentation inventory confirms that pages under `docs/reference/server/examples/` document real report-generator source files. They are active reference pages, not generated report output, so they stay in the source-reference tree.

For consistent routing, a source change is non-trivial when it changes executed logic, a command, a setting, a data contract, parser/LSP behavior, a process/tool flow, or a subsystem boundary.
Architecture-sensitive work includes `server/`, `src/languageClient/`, `src/gameData/`, `src/extensionConfig/`, `tools/fixtures/`, and new non-trivial source folders.

## Compound Engineering Lifecycle

Use CE to keep broad work from collapsing into ungrounded edits.

`ce-brainstorm` settles what to build.
It is useful when the problem, user outcome, scope boundary, or success criteria are still ambiguous.
Its output belongs in `docs/plans/` as a requirements-only unified plan when a durable artifact is warranted.

`ce-plan` settles how to build.
It enriches requirements into implementation units, decisions, file scope, test scenarios, and verification gates.
Its output also belongs in `docs/plans/`.

`ce-work` executes implementation-ready plans.
It treats the plan as authority, works the units, verifies the result, and leaves progress in git and verification output rather than mutating the plan.

Use CE execution and review skills when they match the task:

- `ce-debug` for bugs, failing tests, regressions, or stack traces.
- `ce-code-review` for structured review of code changes.
- `ce-resolve-pr-feedback` or GitHub PR feedback skills for review comments.
- CE commit and PR skills when the user asks to commit, push, or open a PR.

When CE tooling is unavailable, contributors should still provide equivalent structure for broad or architectural work: scoped requirements, implementation plan, verification evidence, and a clear account of decisions.
The repository should not depend on a specific agent product to preserve engineering rigor.

## Routing Rationale

The project uses a balanced-premium fleet because completed-work quality matters more than nominal per-attempt price.
DeepSWE v1.1 is a useful prior for long, underspecified implementation: it favored Terra High over cheaper Luna routes and found Sol High to be the practical premium quality/cost point.
It is not a universal model ranking because its model-neutral harness under-represents bug localization, refactoring, native Codex behavior, and this repository's smaller tasks.
Local routing outcomes are the repository-specific authority; benchmark or pricing changes trigger review but never rewrite routes directly.

## Risk Classification

The parent classifies every task before delegation from observable facts, records the reason, and reclassifies when those facts change.
Task type and risk class are separate: documentation can be consequential, and code can be bounded.

| Risk class | Required facts |
|---|---|
| Bounded | One or two files, established local pattern, clear acceptance behavior, no public contract, no uncertain API, no semantic core, and focused verification available. Every condition must hold. |
| Normal | Behavior-bearing work with understood architecture, normal rollback, and adequate tests; may span several files but does not change a semantic or cross-subsystem contract. |
| Consequential | Parser, AST, semantic model, index, diagnostics semantics, formatting semantics, LSP contracts, process lifecycle, packaging/runtime acquisition, security, data-loss risk, difficult rollback, weak verification, or exact Reforger behavior encoded into tooling. Any listed trigger is sufficient. |
| Critical | Explicitly identified before dispatch as unusually expensive to get wrong, or a consequential Sol High task that failed substantively after receiving exact evidence and one correction opportunity. |

Representative classifications:

| Task | Route | Reason |
|---|---|---|
| Update a one-file contributor note using an established pattern and verify links | Luna High `doc-maintainer` | Bounded when every bounded condition holds. |
| Make a clear one- or two-file local implementation change with focused tests | Luna High `quick-implementer` | Bounded only while scope and contract remain fixed. |
| Implement ordinary TypeScript or Rust behavior within understood architecture | Terra High `worker` | Normal behavior-bearing work. |
| Explore the repository before a brainstorm, plan, implementation, or debug decision | Terra High `explorer` | Discovery uses the normal read-only route. |
| Change parser, model, index, diagnostics meaning, formatting meaning, or an LSP contract | Sol High `high-risk-implementer` | Semantic-core and protocol work is consequential even when the diff is small. |
| Encode an exact Reforger behavior in language tooling | Terra High `reforger-researcher`, then Sol High `reforger-reviewer` | Evidence collection is normal; the consequential conclusion receives independent truth review. |
| Perform work explicitly classified as unusually costly to get wrong before dispatch | Sol XHigh `recovery-implementer` | Explicit critical route. |

If a bounded task needs a third file, changes a public contract, encounters an uncertain API, enters the semantic core, or loses focused verification, its writer stops and returns it for reclassification before continuing.
A test runner failure, missing dependency, unavailable credential, flaky test, or tool failure is an environment problem, not evidence for a stronger reasoning model.

## Model Matrix

Explicit per-run user overrides win over project defaults.
Automatic project routing is otherwise exact:

| Model and effort | Named routes | Automatic responsibility |
|---|---|---|
| GPT-5.6 Luna High | `quick-implementer`, `doc-maintainer`, `commit-pusher` | Strictly bounded implementation/docs and explicitly authorized git operations. |
| GPT-5.6 Terra High | Root/default, `explorer`, `worker`, `reforger-researcher` | Everyday orchestration, discovery, normal implementation, and source-backed Reforger research. |
| GPT-5.6 Sol High | `high-risk-implementer`, `code-reviewer`, `reforger-reviewer` | Consequential implementation, an assigned project review, and consequential Reforger truth review. |
| GPT-5.6 Sol XHigh | `recovery-implementer` | Work classified as critical before dispatch or one controlled recovery after substantive Sol High failure. |
| GPT-5.4 Mini Low | `code-validator` | Independent execution and exact reporting of specified tests, builds, lint, and type checks without diagnosis or source edits. |

Sol Max is manual-only, and Ultra is never selected automatically.
Luna Low/Medium/XHigh/Max, Terra Low/Medium/XHigh/Max, Sol Low/Medium/Max, GPT-5.5, and full GPT-5.4 have no automatic role until local evidence establishes a distinct responsibility.
The initial concurrency limits are four threads and one delegation level; only a measured routing review may propose changing them.

## Escalation And Verification

Routing follows a state machine, not a cascade:

1. Classify before dispatch.
2. Route bounded work to Luna High, normal/discovery work to Terra High, consequential work to Sol High, and explicitly critical work to Sol XHigh.
3. Reclassify Luna work to Terra when bounded conditions cease to hold, or directly to Sol High when a consequential trigger appears.
4. Reclassify Terra work directly to Sol High when a consequential trigger appears.
5. Give a Sol High writer exact evidence and one correction opportunity before treating a substantive implementation failure as recovery-eligible.
6. Permit one Sol XHigh recovery attempt. If it fails, stop automatic escalation; Sol Max remains a manual user choice.

Tool, environment, dependency, flaky-test, credential, and availability failures stay at the same reasoning tier while the underlying problem is fixed.
If a configured model is unavailable, the dispatch fails visibly and the parent records `unavailable`; it never substitutes another model or guesses the actual route.
Required checks may run inline so verification can continue, but an inline check is not a replacement agent route.

Every writer performs its own focused verification inline and reports the evidence.
Dispatch the Mini `code-validator` only when independent execution answers a distinct question or validates evidence the writer cannot directly establish. Do not run it merely to repeat an already-passing command.
The validator reports exact command outcomes, does not diagnose failures, and does not edit source.
Independent validation is evidence, not a second review owner.

## Evidence Discipline

Rigor comes from distinct evidence, not from repeating tools, reviewers, or the same conclusion at a higher reasoning tier. Before editing, state the smallest intended slice, the exact behavior or invariant to prove, the verification set, and the stop condition. Re-open scope only when a check fails, the implementation exposes a new uncertainty, or the user changes the task.

Every command must answer a question not already answered by earlier evidence. Select the smallest non-overlapping project commands:

- `npm test` is the final extension workflow when extension behavior requires it; its pretest path already runs type checking, linting, and compilation, so those commands must not be run separately first.
- Rust behavior is verified from `server/` with the relevant `cargo test` invocation. Do not invoke Cargo from the repository root.
- Docs-only work uses `git diff --check` plus manual link/path review.
- The verified auto-commit helper owns one final selected check. Arm it with that check instead of manually running the same command and then running it again through the helper.

Each review question has one owner. Use a CE review or the project reviewer, not both. When a review produces an exact finding and the corrective scope is understood, inspect the changed lines and the regression test after the fix; restart a broad review only when the correction changes architecture, expands the contract, fails focused verification, or introduces a separate concern.

Subagents answer independent questions: unfamiliar evidence collection, competing architecture options, broad review, or independent validation. Do not delegate an already-scoped mechanical correction solely for ceremony. Process lifecycle work is also single-purpose: the final extension build replaces any repo-owned language-server binary, and the active extension/development host reload happens once after that final build.

Stop when the declared invariant and final verification set pass. A passing command, completed review, or successful reload does not justify another confidence pass without a new signal. Final reports record the verification set, retries, and any uncertainty; elapsed time, token usage, and cost are recorded only when the active surface exposes trustworthy values.

## CE Integration Boundaries

Project routing controls the root default, named agents, and built-in `explorer` and `worker` overrides where Codex supports them.
It does not modify the installed Compound Engineering plugin, force models inside CE-owned dispatch, or claim visibility that the active surface does not provide.

| Workflow | Project routing behavior |
|---|---|
| `ce-brainstorm`, `ce-plan` | Terra High remains the root; compatible repository discovery uses `explorer`. CE-owned internal tiers remain CE-managed. |
| `ce-work` | A unit satisfying every bounded condition may use `quick-implementer`; normal implementation uses `worker`; consequential implementation uses `high-risk-implementer`; critical/recovery work follows the escalation state machine. |
| `ce-debug` | Classify the behavior-bearing fix by the same rules. Environment and tool failures do not justify model escalation. |
| `ce-code-review` | The CE review workflow owns its review questions and generic reviewer personas. Record internal routes as CE-managed or unverified and do not add an automatic project `code-reviewer`. |
| Direct project review | Sol High `code-reviewer` owns the specified review question when no CE review workflow owns it. |
| `ce-resolve-pr-feedback` | CE owns review interpretation; compatible fix implementation is routed by risk without adding a duplicate reviewer. |
| `ce-optimize` | Owns controlled comparisons when routing evidence is ambiguous. It may propose but cannot apply policy changes automatically. |
| `ce-compound` | Captures only a human-accepted durable routing lesson, never raw observations or a provisional benchmark inference. |
| CE commit/PR skills | May use `commit-pusher` only after the user explicitly asks for the commit or push operation. Workflow selection does not grant git authorization. |

Each review question has one owner: either the targeted project `code-reviewer` or the applicable CE review workflow.
Do not automatically ask both to answer the same question.
When the surface exposes the actual route, record it; otherwise use route source `inherited`, `CE-managed`, or `unverified` as applicable rather than inferring a model.

Commit and push are explicit-request-only operations, with one repository-local exception: the trusted verified auto-commit protocol. A task owner may arm `tools/verified-refactor-auto-commit.mjs verify --title <one-to-five-word-title> -- <focused-check>` only after its final focused verification is ready to run. A successful check records a fresh receipt; the project-local Codex `Stop` hook may then commit all current working-tree changes on the exact `Refactor` branch.

This protocol never pushes, tags, changes remotes, switches or creates branches, merges, rebases, resets, or rewrites history. It skips on a missing, stale, mismatched, or invalid receipt; a non-`Refactor` branch; an active Git operation; or a clean tree. A failed commit remains visible with its receipt intact. The hook must be reviewed and trusted through Codex's normal hook flow. Direct `commit-pusher` operations, pushes, and every Git operation outside this narrow protocol still require the active user's explicit authorization.

## Routing Observations

The parent owns observation lifecycle and serialization.
At dispatch it creates a content-free attempt observation; at success, correction, failure, timeout, cancellation, unavailable route, or missing return it finalizes that same attempt.
This prevents failed or unavailable attempts from disappearing from the evidence set.
Subagents return outcomes to the parent and never append shared JSONL themselves.

The parent owns work ID, attempt ID, positive attempt sequence, workflow, role, route source, task/risk classes, requested route, and selection/escalation reasons. Every retry or recovery keeps the same work ID and increments the attempt sequence. The subagent returns only terminal state, actual route when exposed, verification counts, correction count, failure tags, and optional trustworthy usage; the parent merges those facts before invoking the recorder.
It never records prompts, source content, tool output, or absolute paths.
Usage, latency, token, and reported-cost fields are optional and are recorded only when the active Codex surface exposes trustworthy values; missing values remain missing and are never estimated.
Raw observations and generated reports stay under ignored `tools/reports/`.

The first routing review occurs only after at least 30 completed delegations and at least five samples for every high-volume role, defined as a role with at least 10% of completed delegations at review time.
Record that review's completed-delegation count and pass it to later reports with `--last-reviewed-completed`; later reviews occur every 50 additional completed delegations or after three similar failures within ten comparable tasks.
Every review:

- compares first-pass quality, correction burden, escalation, and completed-work cost before per-attempt price
- includes a classification audit stratified across recorded risk classes and high-volume workflows/roles
- blinds the auditor to the recorded class, requested/actual model, selection reason, outcome, corrections, escalation, and usage while the auditor independently classifies from task facts and risk triggers
- prepares any task-context audit packet ephemerally and does not add prompt or source content to observations or reports
- inspects every critical and recovery record, even when its stratum is too small for statistical comparison
- flags insufficient samples and never changes routing configuration automatically

A report with no trustworthy usage or reported-cost data is quality-only.
It may support quality or safety findings, but it cannot support a cost-efficiency route change.
Ambiguous comparisons go to `ce-optimize`; only accepted durable lessons go to `ce-compound`.

## Reforger Evidence Model

Reforger language and engine behavior must be source-backed.
Workbench/compiler behavior is final authority, and the `reforger` skill is the first grounding layer for Reforger facts.

Use the skill before reasoning about Enfusion Script syntax, APIs, attributes, callbacks, lifecycle, replication, resources, prefabs, configs, game-data, extracted APIs, examples, fixtures, parser/model/index behavior, diagnostics, formatting, hover, completion, definition, references, rename, semantic tokens, or LSP behavior.

Terra High `reforger-researcher` owns source-backed evidence collection and must invoke the skill.
When the conclusion is consequential for language behavior, API use, replication, Workbench behavior, or tooling semantics, Sol High `reforger-reviewer` independently checks the evidence and conclusion.
That review does not displace Workbench/compiler authority.

The evidence hierarchy is:

1. Workbench/compiler behavior when available.
2. Official Reforger documentation.
3. Extracted APIs and generated game-data records.
4. Source examples and samples as idiom guidance.
5. Repo fixtures labeled by confirmation level.

Examples are useful, but they are not compiler truth.
Do not import assumptions from C#, Unity, Unreal, Arma 3, SQF, or generic scripting-language habits.

## Documentation Preservation

Documentation should preserve useful project memory, not obsolete architecture.
When docs match current ownership, behavior, or constraints, keep them current.
When docs encode harmful legacy direction, rewrite or delete that content instead of preserving it as a burden.

Deletion or major rewrite needs an evidence gate.
Ground the change in at least one of:

- current source behavior
- current accepted policy
- explicit user direction
- an applicable CE artifact that is current, names the affected path or subsystem, and does not contradict current source behavior, accepted policy, or source-mirror ownership docs
- Workbench/compiler or Reforger source-backed evidence for language behavior

Preserve replacement context in the correct owner whenever any part of the old documentation remains useful.
If no replacement context is needed, say why in the change notes, commit message, or plan result.

CE plans are decision artifacts, not permanent proof.
Treat a plan as historical when a newer artifact supersedes its scope or current source/policy contradicts it.
An applicable plan supports a documentation decision only when it is current, names the affected path or subsystem, and remains consistent with source truth.

## Migration Preservation

For a documentation-root migration, create a per-file old-to-new mapping before moving files.
Before staging or committing, verify that every deleted legacy path has exactly one expected replacement and that no replacement remains merely untracked while its old path is deleted.
This checkpoint prevents a partial commit from dropping durable project context.

## Verification Philosophy

The verification loop in `AGENTS.md` is the default for repository work.
Code and behavior-bearing changes should run the relevant project commands and targeted manual validation needed for the touched subsystem, once each and only when they add evidence not supplied by another selected command.

Docs-only changes can be verified with `git diff --check` and manual link/path review when no source, package, build, or runtime behavior changed.
That exception is narrow.
It does not justify skipping tests for code, language behavior, extension activation, packaging, or user-visible workflows.

For Reforger-language behavior, local tests and repo checks are not enough when the claim depends on compiler or engine truth.
Use Workbench/compiler validation whenever available and record what remains unverified when it is not.

## Practical Routing Examples

For a vague architecture change, start with `ce-brainstorm` or `ce-plan` before editing.
Write or update a CE plan under `docs/plans/` when decisions need durable handoff.

For a parser, model, index, or LSP feature, use the `reforger` skill first, read matching `docs/reference/server/...` files, inspect the relevant Rust source, then work through a CE plan when the change is non-trivial.

For a docs-only cleanup, read the affected docs in full, identify whether the content is current context or harmful legacy direction, apply the evidence gate for deletion, and verify with markdown/path checks.

For a small mechanical source change, read matching docs if they exist, make the smallest change, run the targeted project checks, and update docs only if ownership, behavior, boundaries, or future direction changed.
