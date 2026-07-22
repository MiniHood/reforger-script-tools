# Key Input Routing

## Decision

Use VS Code commands and narrowly-scoped keybindings for an editor action that
must replace a native keystroke atomically. Keep generic structural editing in
`language-configuration.json`, and use completions/snippets for keyword
expansion and editable scaffolds. Do not attempt a global keyboard relay.

The TypeScript bridge may perform only cheap, conservative admission checks
(editor language, one empty selection, visible suggestion/snippet state, and a
local candidate shape). Rust is the sole authority that decides whether an
Enfusion action is valid and returns its edit. If Rust declines, the bridge
executes the known native command without changing the document first.

## Evidence

- A contributed `keybindings` rule maps a key combination, optionally limited
  by a `when` clause, to a command. VS Code runs the first matching rule, so a
  narrowly scoped rule can own Enter without affecting ordinary editor use.
  User keybindings and other extensions can still override it; the Keyboard
  Shortcuts troubleshooting command shows the winning rule.
  [Keybindings contribution](https://code.visualstudio.com/api/references/contribution-points#contributes.keybindings),
  [keybinding resolution and troubleshooting](https://code.visualstudio.com/docs/configure/keybindings).
- Commands are the supported extension boundary for keyboard gestures.
  `registerTextEditorCommand` gives the active editor and a temporary edit
  builder; ordinary command handlers receive command arguments, not raw DOM
  keyboard events. Therefore extensions can control keys that VS Code routes
  through its keybinding service, but cannot create a universal low-level key
  interceptor for IME composition, platform-reserved shortcuts, or every text
  input path. [Commands API](https://code.visualstudio.com/api/references/vscode-api#commands).
- `when` clauses can use documented editor contexts and extension-owned values
  created with `setContext`. Avoid relying on undocumented VS Code internal
  keys, which the API warns may change. [When-clause contexts](https://code.visualstudio.com/api/references/when-clause-contexts).
- Language configuration owns generic bracket behavior and indentation.
  `onEnterRules` only inspect the current line before/after the cursor and one
  previous line; their effects are limited to indent/outdent and optional text
  adjustments, and the first matching rule wins. They cannot make a
  parser-aware decision about a control header or distinguish a `do ... while`
  tail. [Language configuration guide](https://code.visualstudio.com/api/language-extensions/language-configuration-guide#on-enter-rules),
  [VS Code language-configuration source](https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/languages/languageConfiguration.ts).
- A completion can insert a `SnippetString`; snippets supply placeholders and
  tab stops. Completion commit characters accept the completion before typing
  one character, and a completion command runs only after insertion.
  [CompletionItem API](https://code.visualstudio.com/api/references/vscode-api#CompletionItem),
  [SnippetString API](https://code.visualstudio.com/api/references/vscode-api#SnippetString).

## Routing Policy

| Input | Preferred owner | Use it for | Do not use it for |
| --- | --- | --- | --- |
| Enter | Narrow keybinding -> bridge -> Rust | A proven, single control-header block edit applied before native Enter | General formatting or situations where Rust declines |
| Typing `{`, `(`, `)`, indentation | Language configuration | Bracket pairing, ordinary indentation, and other regex-local rules | Syntax-aware scaffolding |
| Completion acceptance | Rust completion result plus VS Code snippet support | Keyword skeletons and editable defaults | Reimplementing Tab navigation |
| Space | Completion commit character only | An unambiguous completion that conventionally commits on Space | A global Space relay |
| Tab | VS Code default bindings/snippet mode | Suggestion acceptance, indentation, and placeholder navigation | An extension override except a separately proven, non-overlapping context |

## Performance and Correctness Contract

1. Do not bind every printable key or maintain a semantic `when` context on
   every cursor/edit event. Leave non-candidates to VS Code's default bindings.
2. In a key-owning command, reject immediately without an LSP request unless
   the editor, language, selection, and cheap local shape can possibly qualify.
   This screening is routing only; it must not duplicate Enfusion parsing.
3. Send at most one versioned request for a candidate keystroke. Apply the
   Rust-produced edit as one editor operation, including the final selection or
   snippet, so native input is never first inserted and then repaired.
4. Before falling back, verify the document version and caret still match the
   command's starting state. If they do not, do nothing rather than inserting a
   late native character into changed text.
5. Keep `suggestWidgetVisible` and snippet mode out of the custom Enter/Tab
   binding. VS Code's completion and snippet contract owns those interactions.

## Implication for Reforger Script Tools

The existing control-header Enter command is the correct mechanism for atomic
loop/switch body creation. Its `mayBeControlHeader` check should remain a
conservative transport gate; `server/` must continue to decide whether the
header is complete, whether a body is appropriate, and the exact edit. Future
keyword expansion should prefer server-provided snippets/completions over
custom Space or Tab commands. Add a custom keybinding only after there is one
unambiguous Enfusion action and a test proves that it does not overlap native
suggestion, snippet, multi-cursor, read-only, or selection behavior.
