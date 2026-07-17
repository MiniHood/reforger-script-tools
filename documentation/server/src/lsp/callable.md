# server/src/lsp/callable.rs

## Purpose

Owns shared LSP-side callable helpers for source-backed signatures and callable argument context.

## Architecture Role

This module sits below LSP feature projection modules such as completion and signature help. It parses copied callable signature strings into parameter facts and walks parser syntax nodes to find the callable argument list at a cursor offset.

## Current Behavior

The helper exposes callable signature parts, parameter names/types/defaults, required/optional classification, and callable argument context for attributes, call expressions, and `new` expressions. It also identifies the active argument index, active named argument label, and already supplied named labels.

## Dependencies and Boundaries

Depends on lexer, syntax, and AST expression views. It must not own completion ranking, signature-help rendering, request dispatch, resolver policy, external index state, or formatting edits.

## Change Notes

Extracted from completion so completion and signature help use the same source-backed signature parser and argument-list context path.

## Future Improvements

Add generic type-argument context only if a separate type-argument help feature is implemented. Keep callable argument context syntax-backed rather than string-scanned.
