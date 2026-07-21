# System Overview

Reforger Script Tools is a VS Code extension and bundled Rust language server
for Enfusion Script. Its purpose is high-fidelity language understanding and
reliable editor behavior without making users install a separate toolchain.

Read [the architecture](architecture.md) for runtime flow, ownership, and
invariants.

## Sources of Truth

For Enfusion Script behavior, evidence is ordered as follows:

1. Workbench/compiler behavior.
2. Official Reforger documentation.
3. Verified extracted game data.
4. Source examples and fixtures, labelled by confidence.

Source code is authoritative for implementation. Tests prove covered behavior.
Generated reports and investigations are supporting evidence, never the
architecture or language authority.

## Documentation Rule

This overview records cross-module facts. Future documentation should explain
only a stable module contract, a consequential decision, or a reusable evidence
format. It should not restate volatile implementation details or mirror every
source file.
