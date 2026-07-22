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

At the time of this research, this extension's `configurationDefaults` sets
`[enforce].editor.autoIndent` to `"full"`. That matches VS Code's native
default, so it is redundant unless the extension deliberately intends to set a
language default. Removing that contribution is the appropriate follow-up if
the intended policy is to leave the editor setting wholly to VS Code and the
user; this research note makes no code change.
