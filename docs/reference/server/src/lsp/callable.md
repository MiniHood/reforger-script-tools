# server/src/lsp/callable.rs

## Purpose

Owns shared LSP-side callable helpers for source-backed signatures and callable argument context.

## Architecture Role

This module sits below LSP feature projection modules such as completion and signature help. It parses copied callable signature strings into parameter facts and walks parser syntax nodes to find the callable argument list at a cursor offset.

## Current Behavior

The helper exposes callable signature parts, parameter names/types/defaults, required/optional classification, and callable argument context for attributes, call expressions, and `new` expressions. Nested calls select the innermost enclosing argument list. Argument counting is lexer-backed, ignores quoted literals and nested expression delimiters, and recognizes generic angle brackets only when they are syntactically type-like rather than treating relational comparisons as generic nesting. Signature splitting likewise preserves commas, closing parentheses, and escaped quotes inside default literals.

It identifies the active argument index, active named argument label, and already supplied named labels. Supplied-label keys are normalized to ASCII lowercase because parameter labels are matched case-insensitively throughout callable completion and signature help.

## Dependencies and Boundaries

Depends on lexer, syntax, and AST expression views. It must not own completion ranking, signature-help rendering, request dispatch, resolver policy, external index state, or formatting edits.

## Change Notes

Extracted from completion so completion and signature help use the same source-backed signature parser and argument-list context path.

## Future Improvements

Add generic type-argument context only if a separate type-argument help feature is implemented. Keep callable argument context syntax-backed rather than string-scanned.
