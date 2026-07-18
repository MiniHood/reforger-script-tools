# server/src/lsp/diagnostics.rs

## Purpose

Owns LSP projection for parser diagnostics.

## Architecture Role

This file is a focused submodule of the Rust LSP layer. It converts parser `ParseDiagnostic` records into protocol-shaped diagnostic notifications and reusable diagnostic records for reports.

## Current Behavior

Diagnostics use source `Reforger Script Tools parser`, code `reforger.parser.syntax`, and severity `1` for parser/lexer errors. The projection keeps parser messages unchanged and expands zero-width parser spans to a nearby visible source range where possible. `publish_diagnostics_message` builds the standard `textDocument/publishDiagnostics` notification with the matching document version, while `clear_diagnostics_message` retains the versionless close-notification shape. `parser_diagnostics_for_source` is shared by tests and diagnostics reports.

## Dependencies and Boundaries

This module depends on lexer spans, syntax diagnostics, and shared LSP range helpers. It must not parse source, evaluate semantics, call Workbench, inspect indexes, decide whether diagnostics are compiler truth, or handle LSP request dispatch.

## Change Notes

- Extracted parser-diagnostic projection out of the monolithic `server/src/lsp.rs`.

## Future Improvements

- Add semantic diagnostic projection only as a separate diagnostics layer, not by expanding parser diagnostics in place.
- Add richer diagnostic codes only when parser diagnostics become structured enough to support them.
