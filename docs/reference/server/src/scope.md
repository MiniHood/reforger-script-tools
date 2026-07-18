# server/src/scope.rs

## Purpose

Builds the lexical scope model for parsed Enfusion source.

## Ownership

`LexicalScopeModel` turns parser block structure and indexed callable/local facts into queryable file-local visibility. It owns lexical ordering and shadowing, not declaration extraction or semantic type analysis.

## Current Behavior

`from_parse_and_index` builds root, callable, nested block, `ForLoop`, and `ForeachLoop` scopes. Parameters attach to callables; locals attach to their innermost containing block. `for` initializer locals span the header and body; `foreach` variables span only the body. Exact-name and prefix queries walk outward in declaration-before-use and shadowing order, with case-insensitive prefix matching for completion.

Only blocks inside the callable declaration become callable-local scopes, avoiding enclosing class/file nesting. Loop visibility comes from CST headers rather than inferred indexed spans.

## Dependencies and Boundaries

Depends on syntax spans and indexed records. It does not rediscover locals, infer types, evaluate control flow, resolve inheritance, or query external indexes.

## Verification

Scope tests cover nested visibility, declaration ordering, shadowing, prefix completion, and `for`/`foreach` extent.

## Future Direction

Richer statement/branch facts remain possible, but this layer stays lexical and source-backed.
