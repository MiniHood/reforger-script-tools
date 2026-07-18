# server/src/symbol_display.rs

## Purpose

Converts indexed symbol facts into shared editor-ready display records.

## Ownership

`SymbolDisplay` owns source-backed labels, details, signatures, documentation presentation, and provenance formatting below LSP features. Index owns stored facts; feature handlers own protocol-specific conversion.

## Current Behavior

`for_symbol()` returns `SymbolDisplayInfo` with label, kind, owner, detail, callable signature, raw `doc_comments`, bounded preview, modifiers, attributes, provenance, spans, conditional context, and callable form. Callable signatures preserve parameter modifiers; other symbols use copied type/base/value/default facts. `documentation_display()` derives brief, parameter, return, warning, and note sections while raw comments remain unmodified.

Child symbols carry immediate indexed container names for display without re-resolution.

## Dependencies and Boundaries

Depends on index and model/source metadata. It does not parse, resolve, evaluate types/values, perform complete Doxygen parsing, persist caches, call Workbench, or handle LSP.

## Verification

Display tests cover symbol kinds, signatures, details, previews, raw comments, structured documentation sections, and provenance.

## Future Direction

LSP conversion stays in LSP modules. More complete documentation parsing needs an intentional format contract.
