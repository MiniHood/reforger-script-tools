# server/src/lsp/signature_help.rs

## Purpose

Owns LSP `textDocument/signatureHelp` projection for callable argument lists.

## Architecture Role

This module sits inside the Rust LSP layer. It consumes cached open-document analysis, the shared callable helper, resolver, `IndexQuery`, and optional workspace/game-data indexes to build standard LSP signature-help responses.

## Current Behavior

Signature Help is available inside function, method, constructor, `new`, and attribute argument lists. It reports candidate signatures, active parameter index, optional/defaulted parameters, parameter modifiers such as `out` and `inout`, callable documentation previews, and Doxygen parameter/return text where available. It returns no result outside callable argument lists or when the callable target cannot be resolved.

Active parameters are computed independently for every candidate signature: a matching named label selects that candidate's parameter, otherwise the positional slot is clamped to the candidate's final parameter. Empty-parameter signatures report no active parameter. The top-level active parameter always comes from the selected signature, so heterogeneous candidate layouts cannot produce an out-of-range LSP index.

The server advertises `(`, `,`, `.`, `:`, `_`, letters, and digits as Signature Help trigger/retrigger characters. This is intentional: VS Code only asks for Signature Help on advertised triggers, and users need parameter help to appear while typing or editing argument values such as `Run(inp)`, not only immediately after `(` or `,`. The request path uses cached open-document analysis and returns `null` outside callable argument lists, so broad trigger characters do not add a second language system.

Completion and Signature Help share callable parsing through `server/src/lsp/callable.rs`. Callable and non-enum parameter-label completion items attach VS Code's standard parameter-hints command after insertion, so accepting a method/constructor/function/attribute completion can immediately show the active signature and each parameter while the user fills snippet placeholders. Enum-owner placeholders keep the completion-owned enum suggest command instead, because those need enum-member completion at the inserted `EnumOwner.` first.

The existing `reforger/debugCompletion` request appends a Signature Help section to the Ctrl+F2 debug report. This keeps callable editing troubleshooting in one user-facing command while normal `textDocument/signatureHelp` remains a concise standard LSP request.

## Dependencies and Boundaries

Depends on `lsp/callable.rs`, resolver, index query, symbol display documentation helpers, and protocol structs from `lsp.rs`. It must not insert text, trigger autocomplete, format code, evaluate overloads, validate with Workbench, or add TypeScript language analysis.

## Change Notes

Added as the first source-backed signature-help feature. It shares callable signature parsing with completion so optional/defaulted parameter behavior stays consistent.

Added the Signature Help Markdown section used by the existing Ctrl+F2 completion debug command.

Completion items now use Signature Help as the follow-up explanation path for ordinary callable snippets and named parameter labels. This keeps insertion in completion and parameter explanation in Signature Help rather than adding a second formatting or autocomplete system.

## Future Improvements

Add richer overload selection only after there is source-backed call-argument type matching. Generic type-argument help and inline optional-argument ghosting should be separate features, not mixed into this request.
