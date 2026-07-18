# server/src/scope.rs

## Purpose

`scope.rs` owns the first lexical scope model for parsed Enfusion source. It turns source-backed indexed callable parameters and local variables into a block-aware scope tree that later resolver, hover, definition, completion, and diagnostics work can query.

## Architecture Role

The module sits above parser, AST, model, and index. Parser syntax provides callable/block structure, and `SymbolIndex` provides source-backed parameter/local symbols. The scope model does not parse source text itself and does not resolve cross-file symbols.

## Current Behavior

`LexicalScopeModel::from_parse_and_index` builds a root scope, callable scopes, nested block scopes, and parser-owned loop scopes from the syntax tree. Parameters attach to callable scopes. Ordinary local variables attach to the innermost block scope containing their declaration. Declaration-form `for` locals attach to a `ForLoop` scope spanning the header and body, so they are visible in the condition, increment, and body but not after the loop. `foreach` variables attach to a `ForeachLoop` scope spanning only the body, so they cannot shadow the iterable expression or leak afterward. `visible_symbols_named` walks from the innermost scope outward and returns visible local/parameter candidates in shadowing order, with locals declared before the cursor beating later or outer declarations. `visible_symbols_with_prefix` uses the same lexical ordering and case-insensitive prefix matching for unqualified value completion so locals and parameters are returned as real LSP completion items instead of relying on VS Code word suggestions.

## Dependencies and Boundaries

The module depends on syntax tree spans and indexed symbol records. It must not duplicate parser local-discovery logic, perform semantic type inference, evaluate control flow, resolve inheritance, or query workspace/game-data indexes.

## Change Notes

This started as a scaffold for block-accurate local lookup. Resolver now uses it as the authoritative path for local and parameter visibility in hover, definition, completion receiver inference, and semantic-token reference classification. It intentionally preserves existing model/index symbol facts and adds a queryable scope structure instead of changing local extraction behavior.

Callable block collection only creates block scopes for `Block` nodes contained by the callable declaration span. Containing class/file blocks are traversed but not inserted as callable-local scopes; this avoids repeated class-block nesting across methods and keeps corpus scope depth aligned with real callable-body nesting.

Added prefix-based visible-symbol lookup for unqualified value completion. It shares the same declaration-before-use and shadowing behavior as exact-name lookup, while matching prefixes case-insensitively for completion usability.

Loop visibility is CST-derived rather than inferred from indexed symbol spans: `ForInitializer` declarations map to their enclosing `ForStatement`, while `ForeachVariableList` declarations map only to the following statement body. This keeps resolver, completion, and semantic-token local lookup aligned with loop-header ordering.

## Future Improvements

Later slices can add richer statement scopes, branch/control-flow metadata, and semantic type facts. The scope model should remain lexical/source-backed and should not grow semantic type checking or cross-file lookup responsibilities.
