# Refactor documentation to the owner standard

## Review result

Inventory found 107 tracked Markdown files. No tracked page is a high-confidence
delete candidate and no real relative-link breakage was found. The main problem
is structural: 82 current reference pages inherit chronological `Change Notes`
and broad `Future Improvements` sections, which duplicate plans and Git history
instead of helping a maintainer change the owning source.

## Standard

Current source-reference pages contain only: Purpose, Ownership, Current
Behavior, Dependencies and Boundaries, Verification, and genuine Future
Direction. The top-level architecture page owns cross-layer data flow; subsystem
pages own current runtime mechanics; child pages own feature contracts. Plans
remain historical decision artifacts and solutions remain reusable learnings.

## Implementation units

### U1 - Finalize governing entry points

- Make `docs/documentation.md` the authoritative procedure, including the
  per-non-trivial-source-file documentation contract.
- Reduce `AGENTS.md` to policy and discovery links.
- Rewrite `docs/agent-workflow.md` as rationale only; remove duplicate policy,
  verification, and Git instructions that conflict with `AGENTS.md`.

### U2 - Refactor top-level reference architecture

- Rewrite `architecture.md` to cross-layer ownership and data boundaries only.
- Rewrite `server.md` and TypeScript top-level pages as subsystem maps.
- Remove historical timelines; link to plans or solutions only when historical
  context is needed for a current constraint.

### U3 - Refactor Rust source-owner pages by subsystem

- Normalize `server/src/*.md` and `server/src/lsp/*.md` to the standard.
- Keep `lsp.md` as dispatch/runtime lifecycle context; move feature details to
  its child pages and remove duplicated projection prose.
- Preserve parser, resolver, index, and cache invariants that are needed to
  safely modify code.

### U4 - Refactor extension, tools, fixtures, and report-generator pages

- Normalize `docs/reference/src/**`, `tools/**`, and all `server/examples/**`
  pages without deleting source-owner documentation.
- Keep report-generator input/output/ownership; remove generated-output
  descriptions and chronological changelogs.

### U5 - Improve historical and learning discovery

- Add lightweight indexes for plans and solutions if navigation remains poor
  after the governing procedure links are in place.
- Label plans as historical; retain all existing plans unchanged.
- Link solutions to current owners without duplicating current behavior.

### U6 - Validate preservation

- Maintain a per-file classification and before/after ownership map.
- Check all Markdown links, `git diff --check`, and three journeys: new-agent
  orientation, source-file change, and known-problem lookup.
- Confirm every removed section is either obsolete history or preserved in its
  correct plan/solution/source owner.

## Execution notes

Apply the refactor in reviewable batches: governing docs, architecture/subsystem
maps, Rust source owners, extension/tool/example owners, then indexes and link
verification. Do not delete pages solely because their filename contains
`report`, `debug`, `baseline`, or `corpus`.
