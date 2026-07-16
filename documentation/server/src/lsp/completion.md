# server/src/lsp/completion.rs

## Purpose

Owns LSP completion projection for the Rust language server.

## Architecture Role

This module sits inside the LSP layer and maps cached open-document analysis plus the optional external workspace/game-data overlay into LSP `CompletionList` responses and dev-report data. `server/src/lsp.rs` remains responsible for protocol dispatch and document/cache lifecycle.

## Current Behavior

Completion is resolver-owned. The module asks `ReferenceResolver` for member-completion context or top-level/type prefix context, then queries the file-local index, workspace external layer, and game-data external layer separately. It combines candidates without constructing a full merged index per request, preserving open-document candidates ahead of workspace candidates and workspace candidates ahead of game-data candidates. The server advertises `.` for member completion and `[` so attribute-list typing can request the same source-backed top-level/type completion path immediately.

Member completion supports instance and static owners. Instance members use `IndexQuery::completion_members_for_class`; static owners use `IndexQuery::completion_static_members_for_type`. Static-owner completion is the only editor-facing path that returns enum members, so values such as `DEBUG` or `STATIC` require their enum owner/container, for example `LogLevel.`. Explicit receiver member completion hides `private` and `protected` members when the receiver is an arbitrary object outside the owning class context, but keeps them for `this`/`super` and unqualified class-scope completion. Unqualified value completion uses the cached lexical scope for visible locals/parameters, current-class member completion for methods/fields available without a receiver, source-backed base-owner completion across the open-document/workspace/game-data layers for inherited unqualified calls, top-level value/callable completion for functions/globals/types, and language keyword items such as `return`. Prefix matching is case-insensitive for completion display, while inserted text remains the real source-backed symbol or keyword spelling. Keyword items are ranked before source-symbol matches for the same prefix. Statement/expression keywords such as `return`, `true`, `false`, `null`, and `new` are available in value contexts. Declaration/modifier keywords such as `static`, `protected`, `private`, `override`, `class`, and `typedef` are available only after clear declaration boundaries such as file start, `{`, `}`, or `;`, or after another declaration/modifier keyword. They are also available when the resolver classifies the prefix as a type-position completion but the token boundary is still declaration-shaped, so modifier prefixes such as `overr` inside a class can complete to `override`. This policy only affects keyword items; source-backed class/type/function/value candidates are still queried through resolver and index context so weak declaration-keyword context does not hide real type candidates. Type-position completion stays type-only for source symbols and primitive/type keywords such as `int` and `typename`. Completion returns at most 250 LSP items after source-aware ranking to keep one-letter prefixes such as `s` useful and bounded. Completion items use `SymbolDisplay`-derived candidate data for labels, signatures/details, documentation previews, label details, source-aware sort text, prefix-replacing text edits, and VS Code standard completion item kinds. Methods/destructors use Method, functions use Function, constructors use Constructor, fields use Field, parameters/locals/non-const globals use Variable, const fields/globals and preprocessor macros use Constant, classes use Class, enums use Enum, enum values use EnumMember, keywords use Keyword, and typedefs/type parameters use TypeParameter because VS Code has no dedicated typedef completion icon.

Inherited override completion is source-backed autocomplete, not auto formatting. At a class-body declaration boundary, completion walks the containing class's preferred base chain across the open document, workspace index, and game-data index, then offers overridable inherited methods matching the typed prefix. These items insert conservative snippet skeletons such as `override protected void OnPostInit(IEntity owner) { $0 }` so accepting the completion places the cursor inside the new method body. Override skeleton completion keeps matching declaration keywords, especially `override`, in the same result set so parent-method skeletons do not hide normal modifier completion while the user is still typing the modifier. When the user has already typed declaration modifiers before the method prefix, such as `override protected OnPostIn`, the skeleton omits those already-present modifiers from its inserted text so accepting completion does not duplicate them. Private, static, sealed, proto, external, and native methods are not offered as override skeletons. Formatting may later clean indentation or brace placement, but it must not independently choose the parent method.

The module also owns the bounded Markdown formatter for the custom `reforger/debugCompletion` request. That request runs the same cached-analysis completion projection as `textDocument/completion`, then reports request context, receiver/owner inference, prefix, candidate counts, source/origin counts, phase timings, and the first completion items. It is command-triggered only and must not add background or per-completion logging cost.

## Dependencies and Boundaries

Depends on `ReferenceResolver`, `IndexQuery`, `SymbolDisplay`-backed completion candidates, `SymbolIndex`, and basic LSP range/position helpers from `server/src/lsp.rs`.

This module must not own request dispatch, open-document storage, workspace file watching, game-data cache lifecycle, TypeScript client behavior, diagnostics, hover, definition, or semantic-token projection. It must not add a second completion path through raw index APIs when `IndexQuery` has an editor-facing API for the same result.

## Change Notes

Extracted from the monolithic `server/src/lsp.rs` without behavior changes. This keeps completion candidate lookup, ranking, item rendering, and timing in one owner while leaving protocol dispatch in `lsp.rs`.

Added the `reforger/debugCompletion` report formatter for Ctrl+F2 completion troubleshooting without adding normal request hot-path overhead.

Aligned completion item kind projection with the VS Code IntelliSense icon table, including Constant for const source facts and macros.

Added unqualified value-prefix completion for visible locals/parameters and current-class members so symbols such as `owner` and `GetOwner` come from the LSP completion list with Variable/Method icons instead of falling through to VS Code word suggestions.

Made completion prefix matching case-insensitive so typing `get` can offer the source-backed `GetOwner()` item. Comment/string autocomplete startup is controlled by Enforce language defaults in `package.json`; the LSP still returns no candidates when the resolver reports no completion context.

Added LSP-owned keyword completions so disabling VS Code word suggestions does not remove language words such as `return`.

Expanded keyword completion to include declaration/modifier words such as `static`, `protected`, and `private`, and verified keyword results rank before matching source symbols.

Removed standalone enum-member results from unqualified top-level completion. Enum members remain available from static-owner completion when their enum owner is present.

Bounded returned completion lists to 250 items and split keyword completion into statement/expression keywords versus declaration/modifier keywords. Declaration keywords now require a clear source-backed declaration boundary, while source-backed symbol/type completion remains unchanged.

Allowed declaration/modifier keywords to appear at declaration-shaped boundaries even when the resolver reports a type completion context. This fixes prefixes such as `overr` inside a class body without allowing declaration keywords in expression contexts.

Unqualified value completion now walks the containing class's source-backed base owner chain across the file-local, workspace, and game-data layers. This lets inherited no-receiver calls such as `getow` offer `GetOwner()` even when the open document contains only `class Example : ScriptComponent` and `ScriptComponent` lives in the external index.

Added receiver visibility filtering for member completion. `private` and `protected` members are hidden for external object receivers such as `GRAY_TEST2 test33; test33.pro`, while `this.pro` and unqualified in-class prefixes still expose restricted source-backed members.

Added `[` as a completion trigger for attribute-list starts. The server-side completion path already resolves `[Attribu]` to the generated `Attribute` class through normal top-level completion; the trigger makes VS Code ask for completions in that context without manual invocation.

Added inherited override method completion in class-body declaration contexts. Typing a parent method prefix such as `OnPostIn` can now offer a source-backed override skeleton while keeping method-body expression completion unchanged. Override skeleton results retain declaration keyword items so prefixes such as `o` and `overr` can still complete to the `override` keyword instead of being hidden by inherited method suggestions. Override skeleton insertion subtracts already typed modifiers from the current declaration fragment so accepting a skeleton after `override protected` does not produce `override protected override protected`.

## Future Improvements

Add `completionItem/resolve`, richer overload presentation, and snippet placeholders in separate verified slices. Keep future completion behavior routed through resolver context and `IndexQuery` rather than direct raw aggregate index access or merged external overlay construction.
