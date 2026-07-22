# VS Code Auto-Indent Research

Research date: 2026-07-22. This note uses only VS Code's official documentation
and the current `microsoft/vscode` source tree.

## Result

VS Code's current default for `editor.autoIndent` is `"full"`. The editor
source registers `full` as the default and exposes these values:

| Value | Behavior |
| --- | --- |
| `none` | Insert no indentation automatically. |
| `keep` | Keep the current line's indentation. |
| `brackets` | `keep`, plus language-defined bracket handling. |
| `advanced` | `brackets`, plus language `onEnterRules`. |
| `full` | `advanced`, plus language `indentationRules`. |

The setting affects indentation while typing, pasting, moving, and indenting
lines. The current implementation is the authoritative default/value record:
[editorOptions.ts](https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/config/editorOptions.ts).

## Language-configuration mechanisms

A language extension contributes a language configuration declaratively through
`contributes.languages` in `package.json`; its `configuration` path points to
the language-configuration JSON. See VS Code's
[contribution-point reference](https://code.visualstudio.com/api/references/contribution-points#contributes.languages)
and [Language Configuration Guide](https://code.visualstudio.com/api/language-extensions/language-configuration-guide).

`indentationRules` supplies regular expressions for the editor's general
indentation behavior:

- `increaseIndentPattern` and `decreaseIndentPattern`
- optional `indentNextLinePattern`, to indent only the following line
- optional `unIndentedLinePattern`, to leave a matching line unchanged and
  exclude it from the other rule checks

They govern current/next-line adjustment while typing, pasting, and moving
lines. With no `indentationRules`, VS Code falls back to the language's
declared `brackets`: it indents after an opening bracket and outdents when a
closing bracket is typed. This is distinct from `editor.formatOnPaste`, which
is driven by a document range formatting provider rather than auto-indent.

`onEnterRules` handles Enter-only exceptions. Rules are evaluated in order; the
first complete match wins. Each rule requires `beforeText` and may add
`afterText` and `previousLineText`. Its action has one of `none`, `indent`,
`outdent`, or `indentOutdent`, and may also use `appendText` or `removeText`.
The complete contract and examples are in the
[official guide](https://code.visualstudio.com/api/language-extensions/language-configuration-guide#on-enter-rules).

## Built-in custom-language pattern

VS Code's built-in JavaScript configuration uses a declarative
`language-configuration.json`: bracket definitions, broad indentation rules,
and narrowly scoped Enter rules for JSDoc continuation, `case`/`default`,
single-line control flow, and pressing Enter between matching delimiters.
It does not implement a Tab command or edit listener to compensate for
indentation. See the first-party
[JavaScript language configuration](https://raw.githubusercontent.com/microsoft/vscode/main/extensions/javascript/javascript-language-configuration.json).

For a custom language, the least-invasive progression is therefore:

1. Declare accurate `brackets` and rely on the user's native `editor.autoIndent`
   setting.
2. Add `indentationRules` only for stable, language-wide indentation facts.
3. Add targeted `onEnterRules` only for Enter-specific syntax that generic rules
   cannot express.

Do not bind Tab or apply editor edits merely to correct auto-indentation. That
would compete with VS Code's indentation engine and its user-controlled
setting.

## Repository observation

This extension now leaves `editor.autoIndent` unset for Enforce. VS Code and
the user's normal editor configuration therefore own the policy; current VS
Code defaults to `"full"`.

## Current implementation audit

| Area | Current implementation | Assessment | Follow-up |
| --- | --- | --- | --- |
| Native setting ownership | `package.json` does not override `[enforce].editor.autoIndent`. | Aligned: the extension inherits VS Code's default and the user's editor policy. | Keep it unset. |
| Declarative Enter behavior | `language-configuration.json` has two narrow `onEnterRules` for `if`/`else` headers and their immediate body line. | Aligned: these are synchronous, editor-native rules with bounded textual scope. | Keep and validate them against Workbench/compiler-supported syntax as the language surface expands. |
| Braces and pairs | The language configuration declares `{}`, `[]`, and `()` pairs. It has no `indentationRules`. | Aligned for now: native bracket behavior remains available; no broad regex has been asserted as Enfusion truth. | Add declarative rules only when primary evidence establishes a stable language-wide rule. |
| Post-edit control-body correction | No typing assist observes Tab or corrects unbraced control-body indentation. | Aligned: VS Code owns indentation synchronously, without a second post-edit caret move. | Keep Tab outside the extension's typing-assist path. |
| Other typing assists | The same bridge also handles incomplete `if` headers, semicolon insertion, and empty block-comment expansion. | Out of scope for the Tab/auto-indent decision; they are separate, explicit typing assists. | Review independently before changing. |
| Explicit formatting | The current LSP range formatter is limited to comment regions; it is not a general Enfusion formatter. | Expected current state, not a native-auto-indent substitute. | When the parser/formatter is ready, offer explicit parser-backed formatting rather than a Tab correction. |
