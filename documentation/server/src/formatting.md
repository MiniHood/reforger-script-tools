# server/src/formatting.rs

## Purpose

Temporary architecture notes for the future Rust-side formatter and typing-assist system.

The formatter should make Reforger Script editing faster and more consistent without becoming a loose collection of unrelated text rewrites. It should be source-backed, syntax-aware, and split into clear feature owners so document formatting, on-type formatting, snippets, comments, and attributes do not blur together.

## Architecture Role

Formatting belongs in the Rust language engine, not the TypeScript VS Code shell. TypeScript should only register LSP capabilities, commands, and keybindings. The Rust side should use lexer/parser/syntax/AST facts to decide whether a format or typing assist is safe.

The preferred shape is one shared formatting/context core with separate feature surfaces:

- Document formatting: explicit full-document or range formatting.
- On-type formatting: small edits after typed characters such as Enter, `}`, `;`, `,`, or `:`.
- Typing assists: smart semicolon insertion, bracket handling, and line continuation.
- Completion snippets: source-backed structure insertion for methods, constructors, control flow, and common Reforger idioms.
- Type assist: source-backed type snippets and generic container helpers such as `array<T>`, `map<TKey, TValue>`, and `set<T>`.
- Attribute assist: attribute templates, argument formatting, and type-aware defaults.
- Comment formatting: `//`, `//!`, `/* */`, and `/** */` / Doxygen block formatting.
- Doxygen assist: param/return/warning/note scaffolding and maintenance.

The formatter must not become a second parser. It should consume syntax and semantic facts from existing compiler-style layers.

## Current Behavior

No formatter implementation exists yet. Current notes are planning-only.

The current parser already preserves enough structure for a conservative first formatter:

- declarations, classes, enums, typedefs, fields, methods, constructors, and destructors;
- callable body statements and expressions;
- attributes and attribute argument expressions;
- `for`, `foreach`, and switch sections;
- comments, trivia, and token spans.

Game-code review shows the formatter must handle mixed real-world source:

- Allman-style braces and tab indentation are common and match official conventions.
- Attributes are frequent and can be positional, named, one-line, multi-line, and mixed with `RplProp`.
- Doxygen-style docs use `//!`, `/*! */`, `/** */`, `\param[in]`, `\param[out]`, `\return`, `\warning`, and `\note`.
- Generated, proto, native, Workbench, and docs/example files have different risk profiles.
- Single-line control bodies exist, but official autocomplete templates prefer braces.
- Long method signatures and attribute argument lists are common.

## Dependencies and Boundaries

The future formatter should depend on:

- lexer token/trivia spans;
- parser syntax nodes;
- AST/model facts where needed for declarations, parameters, attributes, and comments;
- resolver/type facts only for targeted assists that need symbol meaning.

The formatter should not:

- use regex-only rewriting for language structure;
- implement serious language analysis in TypeScript;
- reorder declarations or change semantics;
- evaluate macros or Workbench-only behavior;
- format generated or read-only source by default without an explicit policy;
- apply broad edits while the user is typing unless there is no realistic alternate intent.

The core safety rule is:

Only auto-apply an edit when the context leaves no realistic alternate user action. If multiple reasonable actions exist, prefer a completion snippet, explicit command, or no automatic edit.

## Comment and Doxygen Assist

Comment generation should produce documentation that the hover renderer can display cleanly. The formatter must preserve raw user comments, but generated or maintained comments should follow a stable Doxygen-compatible shape.

Canonical compact declaration comments:

```c
//! Brief summary.
//! \param[in] value Description.
//! \param[out] result Description.
//! \return Description.
bool TryGetValue(int value, out string result);
```

Canonical block comments for larger docs:

```c
/*!
	\brief Brief summary.
	\param[in] value Description.
	\param[in,out] state Description.
	\return Description.
	\note Additional note.
*/
bool UpdateState(int value, inout SCR_State state);
```

Class comments should describe the class purpose and stay directly attached to the class:

```c
//! Handles game mode state transitions.
class SCR_MyStateComponent : ScriptComponent
{
}
```

Field comments should describe the field's purpose, not repeat its type:

```c
//! Cached player controller.
protected SCR_PlayerController m_PlayerController;
```

Generation rules:

- Generate `\param[in]` for normal parameters.
- Generate `\param[out]` for `out` parameters.
- Generate `\param[in,out]` for `inout` parameters.
- Generate `\return` only for non-`void` functions and methods.
- Constructors may get a summary but no return tag.
- Destructors should not receive generated docs by default unless explicitly requested.
- Classes, enums, fields, typedefs, functions, methods, constructors, enum values, and globals can receive summaries.
- Preserve existing user-written summary text and only add missing structured tags when a dedicated assist asks for it.
- Keep comments directly attached to the declaration they document; do not insert blank lines between generated docs and the declaration.
- Do not rewrite prose content during normal formatting.

Hover compatibility rules:

- Generated comments should use tags the hover renderer already understands or can support predictably: `\brief`, `\param[in]`, `\param[out]`, `\param[in,out]`, `\return`, `\warning`, and `\note`.
- Parameter names in generated docs must match the parsed declaration exactly.
- `\param` direction should come from parameter modifiers, not naming guesses.
- Generated summaries should be plain text so hover can show them without exposing Doxygen noise.
- Unknown tags should be preserved, but not introduced by automated assists.

## Type Assist and Generic Containers

Generic container helpers such as `array<>`, `map<,>`, `set<>`, and `ScriptInvokerBase<>` are not basic bracket auto-closing. They should be treated as type-position snippets or type assists.

Do not globally auto-close `<` to `<>`. In Enforce source, `<` is also a comparison operator:

```c
if (count < max)
{
}
```

Auto-closing `<` in expression contexts would create incorrect code and fight normal typing. Generic angle brackets should only be inserted when the language context is clearly type-like, usually through completion/snippet selection rather than raw character typing.

Preferred future snippets:

```c
array<T>
ref array<T>
set<T>
ref set<T>
map<TKey, TValue>
ref map<TKey, TValue>
ScriptInvokerBase<T>
```

Rules:

- Offer generic container snippets only in type/declaration contexts.
- Do not trigger generic snippets inside comments, strings, preprocessor directives, or ordinary expression comparison contexts.
- Let normal type completion work inside generic placeholders.
- Avoid duplicate `>` when the user is already inside a generic type.
- Keep `ref` variants as snippets, not automatic rewrites, because `ref` is ownership/signature intent.
- Keep generic type normalization out of formatting; formatting may fix spaces around commas later, but it must not change type meaning.

## Change Notes

Initial planning captured before implementation. This document intentionally separates formatter architecture from autocomplete and hover work so future slices do not add scattered formatting behavior in unrelated modules.

## Future Improvements

Recommended implementation slices:

1. Formatting corpus report:
   - indentation and brace style counts;
   - semicolon-missing candidates;
   - attribute shape samples;
   - method/constructor parameter shape samples;
   - comment and Doxygen block shape samples;
   - single-line control-body samples;
   - generated/proto/native/Workbench file classification.

2. Safe whitespace-only document formatter:
   - tabs for indentation;
   - trim trailing whitespace;
   - stable final newline;
   - preserve comments and blank-line intent;
   - no expression rewriting.

3. On-type semicolon assist:
   - pressing Enter after a complete statement/declaration may insert `;`;
   - never inside comments, strings, preprocessor lines, incomplete argument lists, attributes, or declarations that do not take semicolons.

4. Bracket assist:
   - avoid duplicate `{}` / `()` / `[]`;
   - prefer snippets for larger structures;
   - do not force braces for ambiguous one-line control statements.

5. Type assist and generic container snippets:
   - offer `array<T>`, `map<TKey, TValue>`, `set<T>`, and useful `ref` variants only in source-backed type contexts;
   - do not globally auto-close `<` / `>`;
   - avoid duplicate `>` when completing inside existing generic type text.

6. Attribute assist:
   - type-aware attribute templates;
   - spacing around commas and named-argument colons;
   - stable multiline attributes only through explicit formatting or snippet actions.

7. Comment and Doxygen formatting:
   - preserve raw docs;
   - normalize indentation for `/** */`, `/*! */`, and `//!`;
   - maintain `\param`, `\return`, `\warning`, and `\note` blocks;
   - generate hover-friendly docs for methods, classes, fields, enums, typedefs, constructors, and globals when explicitly requested;
   - avoid rewriting prose content.

8. LSP integration:
   - `textDocument/formatting`;
   - `textDocument/rangeFormatting`;
   - `textDocument/onTypeFormatting`;
   - explicit debug/report tooling before default enablement.
