# Enfusion editing-automation research

Research date: 2026-07-22. This is a discovery report, not an implementation
plan. It expands the earlier narrow question ("what should happen on Enter?")
into the broader editing surface already suggested by Reforger game code and
Workbench: structural typing assists, parser-backed formatting, code actions,
snippets, documentation tooling, and deliberately non-automatic advice.

## Evidence, scope, and decision rule

The primary evidence is the extracted Reforger 1.7.0.54 game source queried
through the Reforger data tool, with selected bounded source inspections. The
official conventions establish Allman braces, tabs, public Doxygen, creator
tags, and naming expectations; the Script Editor plugins show what Workbench
actually automates. See [Scripting conventions](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Conventions), [Script Editor](https://community.bistudio.com/wiki/Arma_Reforger:Script_Editor), and the [Basic Code Formatter
plugin](https://community.bistudio.com/wiki/Arma_Reforger:Script_Editor:_Basic_Code_Formatter_Plugin).

The source survey deliberately spans Game, Core, WorkbenchGame, and Autotest
examples: game code has conditional compilation and inline comments; Core has
generic/nested declarations and attribute support; WorkbenchGame has formatter,
Doxygen, and template plugins; Autotest and editor code show declaration-heavy
styles. It is evidence of existing practice, not a claim that every observed
style is mandatory.

Use the following gate before creating an automatic edit:

| Mode | Required certainty | User control |
| --- | --- | --- |
| Type/Enter action | One parser-proven legal continuation, with no meaningful style or semantic choice. | Immediate, atomic, undoable; decline on ambiguity. |
| Document/range formatter | A semantics-preserving token/CST rewrite with a specified result. | Explicit formatting command (or a separately enabled format-on-save). |
| Completion/snippet/code action | Several valid outcomes, or an action adds user-authored intent. | User selects/accepts it. |
| Diagnostic/advisory | The observation is useful, but any rewrite could change intent or encode a project policy. | Never mutate; offer a reviewable fix only when safe. |

This matters particularly for Enfusion because source uses intentionally
unbraced branches, conditional compilation, native/proto declarations,
attributes, generated files, comments, and differing declaration layouts.
The language engine should provide parser and semantic facts; the VS Code
client must only apply the versioned edit as described in
[Language engine](../language-engine.md) and [Key input routing](../key-input-routing.md).

## A. Deterministic type and Enter actions

These are the deliberately small actions that meet the one-outcome rule.
All must be snapshot-checked, single-caret edits outside strings/comments and
preprocessor directives; they must decline in recovery/error regions, with
existing syntax, active snippets, or multiple selections.

| Candidate | Parser proof and atomic result | Why it is safe enough |
| --- | --- | --- |
| Complete class header + Enter | A parsed class header with no opening brace becomes an Allman body with an indented caret. | A class body is required. The official formatter removes a historical `};` after a top-level class, so this action must close with `}`, not `};` ([formatter](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c:1029)). |
| Complete enum header + Enter | Same Allman body, but insert neither member nor comma. | A body is required, but enum-member comma choice is not. |
| `break`, `continue`, `return`, parsed local declaration, complete call, or assignment + Enter | Append the required statement semicolon before the native newline. | This extends the existing narrow server behaviour only through statement-node kind, never a text whitelist. |
| `#ifdef NAME` / `#ifndef NAME` completion acceptance | A snippet can produce a paired `#endif // NAME` and place the caret inside. | Game source consistently uses matching directive pairs and often names the closing directive ([Doxygen plugin example](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_DoxygenFillerPluginExample.c:1)); it is a snippet rather than an Enter repair because whether a region is needed is user intent. |
| `//!` at the start of a doc line | Continue the exact indentation plus `//! ` on Enter. | It continues an existing Doxygen line, not ordinary commentary; official examples use contiguous `//!` blocks ([Doxygen example](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/Game/Examples/SCR_ExampleDoxygen.c:54)). |

The existing control-header body behavior (`for`, `foreach`, `while`, and
`switch`) stays in this class. `switch` is the important exception: the body is
structurally required, while a `case`, a `default`, fallthrough policy, and a
`break` are authored policy. A selected `default` snippet is appropriate; an
unrequested case or break insertion is not.

## B. Explicit parser-backed document and range formatting

This is the largest safe feature area. It must be a real formatter operating
over tokens/CST—not ported string replacement—and preserve comments, strings,
directives, source ranges, and user-selected range boundaries. Its edits should
be idempotent.

### Baseline layout and token rules

- Normalize leading indentation to the configured canonical tab policy,
  remove trailing whitespace, and ensure one final newline. The Workbench
  formatter exposes exactly these as independent options
  ([formatter configuration](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c:30)).
- Apply Allman brace placement and indentation consistently only where the
  syntax tree makes the brace's owner clear. This is an explicit format choice,
  not an on-type action.
- Normalize control-header spacing (`if (` / `else if (` / `for (` / `foreach
  (` / `while (` / `switch (`), commas, semicolons, parentheses, generic
  brackets, empty `{}`, and duplicate spaces in code tokens. Workbench's
  formatter lists these exact families ([rule table](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c:1872)).
- Normalize `NULL` to `null` only when it is an identifier token in code, never
  a string/comment/macro argument. Workbench performs this narrow spelling
  normalization ([rule table](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c:1881)).
- Remove an extraneous `;` after a complete attribute list, and remove the
  historical semicolon after a top-level class. Both are supported by the
  formatter/example ([example](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPluginExample.c:8)).
- Canonicalize a method separator only when a comment token is unambiguously a
  separator, and retain its indentation. Workbench’s separator normalizer is
  intentionally separate from general formatting
  ([implementation](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c:555)).

### Declarations, attributes, and lists

- Format modifiers, return type, name, parameters, inheritance colon, generic
  arguments, and attribute arguments from syntax nodes. This provides useful
  cleanup for the very dense declarations seen in Core and Workbench without
  changing what is declared.
- Preserve one declaration per line unless a user has explicitly chosen a
  wrapping convention. There is no source-wide single line-width or wrapping
  rule strong enough to infer automatically.
- Attribute lists are high-value formatting targets: normalize internal
  comma/colon spacing, do not insert/remove values, and do not reorder named
  arguments. Attribute metadata can encode editor widgets, default values,
  categories and resources, as shown in Core and ScriptTemplate source
  ([Core attributes](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/Core/attributes.c:38), [template attributes](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_ScriptTemplatePlugin.c:9)).
- Format array/map literals and argument lists conservatively: spacing and
  indentation are safe; trailing comma introduction/removal and line wrapping
  are not until the grammar and formatter contract explicitly choose a style.

### Preprocessor, comments, and literals

- Preserve directive lines verbatim for the first formatter release, except
  optional trailing-space cleanup. Directive nesting and macro contents must
  never be interpreted as normal code. Game/Workbench source relies heavily on
  `#ifdef WORKBENCH`, `#ifdef DEBUG`, and matching `#endif` pairs.
- Treat Doxygen `//!` and block-doc comments as structured comment regions:
  preserve tags, parameter direction (`[in]`, `[out]`, `[in,out]`), and ASCII
  art; do not reflow prose by default. The official Doxygen example has all of
  those forms ([example](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/Game/Examples/SCR_ExampleDoxygen.c:1)).
- Do not rewrite ordinary comments, string literals, resource paths, format
  strings, or debug message text. Spelling and grammar suggestions belong in a
  diagnostics layer, not formatting.

## C. Completion, snippet, and code-action opportunities

These create more value than aggressive typing repair because they expose
choice and make the resulting intent reviewable.

| Opportunity | Interaction | Evidence and guardrails |
| --- | --- | --- |
| Class family templates | Offer `class`, `component-class pair`, `modded class`, `WorkbenchPlugin`, `ScriptedUserAction`, and config/container skeletons as named snippets. | Workbench’s template plugin resolves class name, default parent and type-specific suffix from configuration before insertion ([template plugin](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_ScriptTemplatePlugin.c:61)). Keep this as selectable templates, not generated guesses. |
| Doxygen for declared API | Code action on a parsed method, field, class, enum value, or override: add `//!` summary, typed `\param` entries and a return section; select description placeholders. | Workbench's Doxygen Filler is explicit (Ctrl+Alt+Shift+D), configurable by visibility/override/static/obsolete status, and distinguishes partial `Get`/`Set`/`On`/`Is` docs ([plugin](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_DoxygenFillerPlugin.c:2)). Never overwrite authored docs; offer update/insert separately. |
| Method separator | “Insert canonical separator above declaration” command or Doxygen action option. | Workbench treats missing separators as an opt-in documentation/style operation, not a typing action ([Doxygen plugin](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_DoxygenFillerPlugin.c:44)). |
| Attribute completion | Once `[` opens in valid declaration context, offer `Attribute`, `Obsolete`, `WorkbenchPluginAttribute`, `ComponentEditorProps`, etc., with parameter-name snippets sourced from symbols. | Attributes are editor/data contracts; resolve exact current signatures and do not invent defaults. |
| `new` completion | When completion has selected a constructible type, insert `new Type($0)`; provide collection literal snippets separately. | Workbench’s formatter detects malformed constructor punctuation, but type resolution belongs to completion, not Enter. |
| Logging/debug snippets | Offer `Print`, `PrintFormat`, guarded debug-region, and TODO/FIXME comment snippets with placeholders. | Game sources use explicit debug macro fences and logging patterns; the user must choose level, category and message. Do not auto-add logging. |
| Conditional-compilation region | `#ifdef`, `#ifndef`, and `#if` snippets with paired closing directive and optional name echo. | Existing source demonstrates the pairing; user chooses symbol and scope. |
| “Generate override” | From a resolved base member, insert the exact override declaration/body and optional `super` placeholder. | This is semantic refactoring/completion work, not formatting; it needs exact game-data signature evidence. |
| Naming fixes | Diagnostics plus one-click rename proposals for creator tag/class-file alignment and known member prefixes. | Workbench formatter reports non-prefixed classes and bad variable naming rather than blindly renaming ([formatter findings](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c:935)). Cross-file rename makes this a refactor with preview. |

## D. High-value diagnostics that must not silently format

The formatter source is useful here precisely because it separates mechanical
edits from findings. Candidates include duplicate empty-line groups, one-line
control branches, suspect method separators, creator-tag/class naming, member
prefix/type mismatch, forbidden project terminology, obvious constant division,
and generated-file protection. Workbench reports these rather than making a
unreviewed semantic change ([formatter reporting](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c:803)).

Offer configuration only for real project policy (for example, a team tag or
approved vocabulary). Avoid copying every Workbench plugin toggle into extension
settings; parser-backed diagnostics and `workspace/configuration` should be
introduced only with a defined owner and a stable need.

## External practice: concrete editing interactions worth borrowing

This section compares the preceding ideas with primary VS Code documentation,
the clangd extension, and formatter issue reports from another language
extension. It is deliberately about exact interactions rather than a generic
claim that “other extensions format code.”

### Use the editor's declarative layer before inventing a typing feature

VS Code language configuration already owns brackets, surrounding pairs,
auto-closing pairs, indentation rules, and `onEnterRules`. Indentation rules
run for typing, paste, and moving lines; `onEnterRules` may indent, outdent, or
perform an `indentOutdent` pair ([Language Configuration Guide](https://code.visualstudio.com/api/language-extensions/language-configuration-guide#indentation-rules)).
The built-in C++ configuration demonstrates context exclusions for quote and
comment-pair closing (`notIn: ["string", "comment"]`)
([source](https://github.com/microsoft/vscode/blob/main/extensions/cpp/language-configuration.json)).

That gives Enfusion several very specific, non-server assists:

| Trigger | Exact edit / behavior | Eligibility guards | Interaction model |
| --- | --- | --- | --- |
| Type `{` in ordinary script code | VS Code inserts `}` and places the caret between the pair. | Configure only the language's ordinary brace pair; never have the extension synthesize a second close. The editor suppresses it where its tokenization says it is not appropriate. | Native auto-close; one undo step. |
| Press Enter between an otherwise empty `{|}` | Produce an indented blank line and an outdented closing brace: `{\n\t|\n}`. | Language configuration's `indentOutdent`, only for the exact empty pair. It must not insert a body member, case, or statement. | Native Enter behavior, not an LSP edit. |
| Type `}` at the start of an indented line | Dedent the `}` to the matching brace's indentation. | Declarative decrease-indent rule; must ignore strings/comments and must not try to match preprocessor structure. | Native auto-outdent. |
| Select an expression then type `(`, `[`, `{`, `'`, or `"` | Surround the selection with the paired delimiters. | Declare only pairs Enfusion actually supports. Do not add fake pairs such as angle brackets: they can be comparisons or generic syntax. | Native surrounding-pair behavior; selection is explicit user intent. |
| Type `/*` in script code | Insert the matching `*/`; a separate doc-comment snippet can produce `//! ${1:summary}` rather than guessing block-doc layout. | Do not auto-close within strings/comments; do not change existing `//!` convention into `/** */`. | Native pair for the former; explicit snippet for the latter. |

These are high-leverage because no Rust implementation is needed, and because
they provide familiar behavior without making a syntactic or semantic guess.
They should be tested against the actual Enfusion TextMate grammar, since that
grammar supplies the string/comment context used by the editor.

### Formatting triggers should have visibly different scopes

VS Code distinguishes whole-document, range, and on-type formatting in the
language-server contract ([Language Server Extension Guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide)).
It also routes Format on Paste through the *range* formatter, not the
declarative indentation rules ([Language Configuration Guide](https://code.visualstudio.com/api/language-extensions/language-configuration-guide#indentation-rules)).
clangd makes the on-type scope intentionally modest: Enter formats the old line
and semantically reindents, while full-file and selection formatting remain
available ([clangd VS Code extension](https://github.com/clangd/vscode-clangd#formatting)).

Apply that pattern to Enfusion as follows:

| Trigger | Exact edit | Eligibility guards | Interaction model |
| --- | --- | --- | --- |
| **Format Selection** on any span | Expand the requested range to whole, parser-valid syntax nodes that intersect it (for example: an attribute list, parameter list, declaration, statement list, or one complete class body); format only their trivia. | Decline if expansion would cross a directive boundary, enter an error/recovery node, or include an unterminated string/comment. Never format an arbitrary partial token. | Explicit command; show no prompt. The returned edits are a single undo operation. |
| Paste a block of Enfusion code | Run the same range formatter over the pasted syntactic units: correct indentation, spaces around punctuation, and brace layout inside the pasted block. | Only when VS Code's `editor.formatOnPaste` is enabled; do not touch adjacent existing declarations merely to make a blank-line rule “look right.” | User-configured convenience, based on the range provider. |
| Press Enter after a syntactically complete line | Reformat only the old line's safe trivia plus the new line indentation. Example: `if(x) {` becomes `if (x) {`, then the new line is indented. | The parse snapshot must prove the line is not in a directive/string/comment/recovery region. No insertion of braces, semicolons, cases, or code that was not already implied. | Opt-in format-on-type, analogous to clangd; must be fast and must return no edit on doubt. |
| Format Document | Canonicalize all formatter-owned trivia, while preserving directive lines, comments, and literals as defined above. | Idempotence test is mandatory; skip generated files by default. | Explicit command / separately enabled format-on-save. |

The range boundary rule is particularly important. A real formatter sometimes
needs to inspect surrounding syntax in order to produce a correct local edit,
but it should make no unrelated edits outside the selected construct. A VS Code
format-on-save bug report documents the failure mode when modified-range
formatting and a full formatter disagree ([microsoft/vscode#146910](https://github.com/microsoft/vscode/issues/146910)).

### Treat template-shaped edits as completions or snippets, not formatting

VS Code has a first-class `contributes.snippets` mechanism, so common
language-shaped boilerplate need not be simulated with character hooks
([contribution-point reference](https://code.visualstudio.com/api/references/contribution-points#contributes.snippets)).
Current LSP support also allows a workspace/code-action edit to carry snippet
placeholders when the client advertises support
([vscode-languageserver-node 3.18 release notes](https://github.com/microsoft/vscode-languageserver-node#3180-protocol-900-json-rpc-1000-client-and-1000-server)).

| User-visible proposal | Trigger | Exact inserted shape | Eligibility guards | Interaction model |
| --- | --- | --- | --- | --- |
| `if` / `if else` / `for` / `foreach` / `while` skeleton | Select the completion/snippet, not Enter after a keyword. | `if (${1:condition})\n{\n\t$0\n}`; the two-branch form additionally contains `else`. | Fixed syntax template only; it must not choose a condition, variable, collection, or branch behavior. | Tab-stop snippet. This is the safe answer to “always use braces” without rewriting existing source. |
| `switch` skeleton | Select a snippet. | `switch (${1:value})\n{\n\tcase ${2:value}:\n\t\t$0\n\t\tbreak;\n}` plus a separate `default` snippet. | Never make `break` part of the generic body assist: direct return/fallthrough are valid. The selected snippet explicitly represents the common break-based intent. | Named alternatives, not a one-size-fits-all hook. |
| Resource/null guard | Invoke completion after a resolved nullable expression, or choose a lightbulb action on it. | `if (!${1:reference})\n\treturn;\n\n$0` or `if (${1:reference})\n{\n\t$0\n}`. | Requires resolved expression/type and a user-selected guard *form*; no automatic early-return choice. | Semantic completion/code action with previewable edit. |
| Generate override | Lightbulb on a class or inherited member. | Insert the exact resolved signature, `override`, Allman body, and a `$0` body placeholder; offer “with `super` call” as a distinct action. | Base member must be uniquely resolved; reject native/proto/declaration-only forms; never infer whether a `super` call is correct. | Explicit refactor/code action. |
| Insert/update Doxygen | Lightbulb on an eligible declaration. | Create or update only the machine-owned `//!` tag lines for the current signature, leaving summary prose selected or untouched. | Do not overwrite authored prose; show a diff/preview for updates; skip generated files. | Source action or declaration-local code action. |
| Make conditional region | Select `ifdef` / `ifndef` snippet. | `#ifdef ${1:SYMBOL}\n$0\n#endif // ${1:SYMBOL}`. | Do not auto-wrap a selected block because choosing the macro and scope is intent. | Snippet with linked placeholder. |

This is also where a code action can be more valuable than a formatter: clangd,
for example, exposes extract-variable/function and type-expansion operations as
lightbulb refactorings rather than formatting ([clangd VS Code extension](https://github.com/clangd/vscode-clangd#refactoring)). Enfusion should follow
that separation for any action whose result is more than trivia.

### What feature requests elsewhere warn us not to do

The YAML extension has a long-lived formatting request where changing the
number of spaces before inline comments caused format-on-save output to fail a
widely used linter ([redhat-developer/vscode-yaml#433](https://github.com/redhat-developer/vscode-yaml/issues/433)). The lesson is concrete: do not silently
adopt a “nice” comment-spacing or wrapping policy from game examples. Either
make it part of a documented Enfusion formatter contract, leave it unchanged,
or expose it as a genuine workspace convention with a matching validation
story.

Similarly, a YAML formatter registration problem occurred when related
extensions changed the document language ID; the formatter had only registered
for `yaml`, not the specialized language IDs
([redhat-developer/vscode-yaml#812](https://github.com/redhat-developer/vscode-yaml/issues/812)). For Enfusion, keep a single canonical language ID and test
formatter registration through the packaged extension. If specialised script
flavours are introduced later, decide explicitly whether they share the
formatter rather than accidentally losing formatting for them.

### A sharper shortlist from the external comparison

If the goal is additional *ideas that feel immediately useful*, the best next
experiments are: declarative empty-brace Enter/outdent/surround behavior;
Format Selection that expands only to safe CST nodes; format-on-paste; small
on-Enter trivia cleanup; selected control-flow snippets; and semantic
lightbulbs for generate-override, Doxygen, and guard forms. They cover four
different user moments—typing, pasting, deliberate formatting, and deliberate
generation—without pretending that they are all auto-formatting.

## E. Explicit anti-patterns and ambiguous transforms

- Do not add braces to every `if`/`else`, expand one-liners, or add blank lines
  around branches. These are code-style choices and game code uses intentional
  unbraced branches.
- Do not insert `break` after each `case`, comma after every enum/list member,
  `;` after declarations/attributes/class endings, or a `do` body. Each has
  legal alternatives (fallthrough, direct return, last-item syntax, optional
  forms, or a required trailing `while`).
- Do not fabricate method bodies: `proto`, `external`, interface-like, native,
  and declaration-only methods legitimately end with `;`. Do not infer `super`
  calls in an override.
- Do not sort attributes, parameters, enum members, imports/includes, methods,
  fields, cases, or configuration entries. Ordering can be API/data/behavior.
- Do not normalize numeric literal spelling, vector representations, colors,
  resource names, localization text, or string concatenation. Equivalent-looking
  forms can carry intent, precision, serialization, or presentation meaning.
- Do not alter preprocessor spacing/structure, comment wording, Doxygen prose,
  TODO ownership, logging level, or debug macro use automatically.
- Do not auto-fix names with a textual replacement. Renaming has workspace and
  asset/config references; it needs symbol-aware previewable refactoring.
- Do not format generated paths/files by default. Workbench explicitly excludes
  configured directories and the Doxygen plugin skips generated scripts
  ([formatter option](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c:86), [Doxygen guard](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_DoxygenFillerPlugin.c:134)).

## Suggested delivery order

1. Build the explicit document/range formatter foundation: trivia ownership,
   safe token spacing, final newline, trailing whitespace, indentation, and
   comments/directives/literals regression corpus.
2. Add mechanical attribute/declaration/list formatting and canonical
   separators only after parse coverage makes them idempotent.
3. Add explicit Doxygen and template/region snippets, then semantic code
   actions such as generate-override and previewable naming fixes.
4. Extend on-type only for independently tested one-outcome actions; class,
   enum, and doc-line continuation are more valuable next than broad punctuation
   heuristics.
5. Add advisory diagnostics last, with a clear distinction between official
   convention, Workbench preference, and configurable project policy.

For every eventual implementation, add accepted/declined tests for comments,
strings, preprocessor regions, generated-file markers, malformed/recovery
trees, CRLF, UTF-16 positions, snippets, multi-cursor edits, stale snapshots,
and idempotence. For a `server/` change, run `npm run compile` to replace the
bundled server and reload/relaunch the language server before editor validation.
