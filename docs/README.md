# Documentation

This directory records durable context that code alone cannot communicate:
architecture, module boundaries, workflow, and consequential decisions. Code
and tests are the source of truth for implementation.

Keep it flat until a real group of documents needs a folder. Do not recreate
path-mirrored, per-file current-state documentation. Add a document only when
it explains a stable module boundary, a consequential decision, or a reusable
evidence contract. Update an existing document when it owns the subject.

## Documentation Lifecycle

This is the repository's documentation policy. Read it before creating or
updating any documentation.

At task completion, decide whether the completed change altered a documented
contract, architecture boundary, workflow, design decision, or evidence
format. Update the document that owns that context when it did. Do not add a
documentation change merely to restate implementation details already clear in
code and tests.

## Current Documents

- [System overview](overview.md): product purpose and sources of truth.
- [Architecture](architecture.md): module boundaries and runtime invariants.
- [Language engine](language-engine.md): Rust analysis and LSP contract.
- [Development](development.md): build, test, and local development workflow.
