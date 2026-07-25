# Documentation

This directory records durable context that code alone cannot communicate:
architecture, module boundaries, workflow, and consequential decisions. Code
and tests are the source of truth for implementation.

Core contracts stay at this directory's root. Investigation journals and
supporting evidence belong under `research/`; accepted architectural decisions
belong under `adr/`; agent workflow contracts belong under `agents/`. Do not
recreate path-mirrored, per-file current-state documentation. Add a document
only when it explains a stable module boundary, a consequential decision, or a
reusable evidence contract. Update an existing document when it owns the
subject.

## Documentation Lifecycle

This is the repository's documentation policy. Read it before creating or
updating any documentation.

At task completion, decide whether the completed change altered a documented
contract, architecture boundary, workflow, design decision, or evidence
format. Update the document that owns that context when it did. Do not add a
documentation change merely to restate implementation details already clear in
code and tests.

For ticketed work, completion means every ticket requirement has been checked
against the final code and verification evidence. An independently verified
implementation slice is progress, not a completed ticket.

## Core Documents

- [System overview](overview.md): product purpose and sources of truth.
- [Architecture](architecture.md): module boundaries and runtime invariants.
- [Language engine](language-engine.md): Rust analysis and LSP contract.
- [Development](development.md): build, test, and local development workflow.
- [MCP API Reference](mcp-api.md): generated public tool descriptors, schemas,
  workflows, limits, and stable failures.
- [Key input routing](key-input-routing.md): VS Code key-routing boundary and
  the ownership policy for atomic typing assists.

## Research

- [Architecture review journal](research/architecture-review-journal.md):
  module-boundary review notes and follow-up opportunities.
- [MCP server exploration journal](research/mcp-server-research.md): proposed capability
  boundary and feature catalogue for a local Reforger MCP server.
- [Base-game source search research](research/base-game-search-research.md): language
  index ownership, VS Code search surfaces, and MCP adapter guidance.
- [Workbench NET API exploration journal](research/workbench-net-api-research.md):
  extracted protocol evidence, adapter boundary, and validation backlog.
- [Workbench compiler-validation research](research/workbench-compiler-validation-research.md):
  `ValidateScripts` contract, continuous-validation design, diagnostics, and
  live-session acceptance experiments.
- [Enfusion structural-formatting research](research/enfusion-structural-formatting-research.md):
  evidence and design notes for automated editing behavior.
- [Enfusion `new` editing research](research/enfusion-new-formatting-research.md):
  primary-source evidence and safe completion/formatting boundary for `new`.
- [VS Code auto-indent research](research/vscode-auto-indent.md): editor
  indentation behavior and integration evidence.
- [Third-party add-on symbol metadata research](research/third-party-addon-symbol-metadata.md):
  licensing and technical boundary for dependency API assistance without source
  extraction.
- [s&box MCP server review](research/sbox-mcp-research.md): first-party MCP-host and
  tool-authoring patterns relevant to the Reforger MCP design.
