# server/src/parser.rs

## Purpose

Parses Enfusion Script tokens into a full-fidelity syntax tree and recoverable diagnostics.

## Ownership

The parser owns syntactic structure and recovery below AST/model/index/resolver/formatting/LSP layers. It is the sole owner of declaration, statement, expression, and declarator CST boundaries.

## Current Behavior

It structures declarations, classes, attributes/modifiers, generic parameters, inheritance, enums, typedefs, fields, callables, parameters, preprocessor directives, initializer expressions, and preserved empty declarations. Callable bodies and attribute arguments use statement/expression syntax for control flow, loops, switch sections, locals, calls, member/index access, casts, operators, named arguments, `new`, `thread`, `delete`, and initializer expressions.

`TypeRef`, `DeclaratorList`, and `Declarator` define shared type/modifier, comma-separated declarator, array suffix, and default-expression boundaries for fields, locals, and declaration-form `for` initializers. `ForeachVariable` retains its distinct header shape. `ForHeader`, `ForeachHeader`, and `SwitchSection` preserve loop/header and label grouping without semantic control-flow interpretation.

Recovery forwards lexer errors, creates bounded `Error` nodes, uses semicolon/braces/declaration starts as synchronization, and preserves later valid declarations. Recursion has a shared depth budget; deep input yields a diagnostic and iterative nested-region consumption. Physical CR, LF, and CRLF terminate directive recovery. Progress assertions prevent non-consuming recovery loops.

## Dependencies and Boundaries

Depends only on lexer and syntax modules. It does not resolve symbols/types/overloads, evaluate expressions/macros/preprocessor branches, index files, call Workbench, or handle LSP. Recovery tolerance is editor availability behavior, not compiler-language proof.

## Verification

Parser tests cover declarations, expressions, declarators, loops, attributes, recovery, deep nesting, line endings, diagnostics, and committed fixtures. Corpus reports provide larger-source recovery evidence.

## Future Direction

Syntax kinds may expand with verified needs. Semantic interpretation stays in later AST/model/resolver layers.
