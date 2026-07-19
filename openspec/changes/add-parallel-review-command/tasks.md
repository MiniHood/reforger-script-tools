## 1. Review-command design inputs

- [x] 1.1 Read the Codex skill-authoring guidance and existing repository skill conventions, then confirm the `/review` command location and invocation contract.
- [x] 1.2 Define the common read-only evidence package, severity/confidence taxonomy, scope-selection rules, and final synthesis format.

## 2. Independent reviewer personas

- [x] 2.1 Add the Architecture reviewer contract covering ownership boundaries, coupling, duplication, and maintainability.
- [x] 2.2 Add the Correctness reviewer contract covering behavioral defects, edge cases, state, cancellation, errors, and tests.
- [x] 2.3 Add the Performance & Reliability reviewer contract covering hot paths, large-input scaling, responsiveness, resource usage, observability, and resilience.
- [x] 2.4 Add the Developer Experience reviewer contract covering editor behavior, public interfaces, diagnostics, documentation, and workflow clarity.

## 3. Coordinator command

- [x] 3.1 Create the repository-local `/review` coordinator skill that prepares shared evidence and launches the four reviewer personas independently in parallel when capacity permits.
- [x] 3.2 Add capacity-limited scheduling, isolation, no-finding, disagreement, deduplication, and advisory/no-write safeguards to the coordinator instructions.

## 4. Documentation and verification

- [x] 4.1 Update the applicable agent-workflow documentation to describe `/review` as a read-only, multi-persona review workflow and distinguish it from `/debug` and `/fix`.
- [x] 4.2 Perform a dry-run instruction audit: validate the skill layout, persona isolation, parallel fan-out contract, paths, Markdown formatting, and repository diff; record any runtime limitation requiring future validation.
