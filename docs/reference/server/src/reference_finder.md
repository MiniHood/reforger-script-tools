# server/src/reference_finder.rs

## Purpose

Provides resolver-backed file-local reference analysis for future references and rename support.

## Ownership

This module owns token scanning and exact selected-symbol grouping. Resolver owns selection rules; later LSP handlers own protocol projection and edits.

## Current Behavior

`find_file_local_references` scans identifier tokens and returns declaration/usage tokens only when resolver selection exactly equals the requested file-local `GlobalSymbolId`. `scan_file_local_references` performs one scan per file and groups selected references; the external-index variant additionally records unresolved and external selections for reports.

`analyze_file_local_rename_at_offset` resolves the symbol under an offset, requires a stable local selection, returns resolver-confirmed local references, and supplies same-name and declaration/usage safety metadata. It does not produce text edits or cross-file results.

## Dependencies and Boundaries

Depends on lexer, resolver, index, scope, and syntax. It does not text-match names, search workspaces, generate rename edits, validate in Workbench, or handle LSP.

## Verification

Tests cover exact selection, shadowing, member access, grouping, external classification, and rename-analysis safety metadata.

## Future Direction

Workspace references and rename edits must build on this resolver-backed path, not a separate text matcher.
