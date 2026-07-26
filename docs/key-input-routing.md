# Key Input Routing

## Decision

Use a versioned, closed vocabulary of semantic editor operations for actions
that must replace native input atomically. An operation has one extension entry
command; its keybinding, a completion flow, or a future extension action may
invoke that command. Keep generic structural editing in
`language-configuration.json`, and keep completion/snippet presentation in
its existing ownership boundary. Do not attempt a global keyboard relay.

Every operation remains native unless an input feature claims its exact editor
state. TypeScript owns editor transport, conservative candidate screening, and
one atomic application. Rust is the sole authority for feature eligibility,
priority, and the result. It either declines or returns one atomic text-edit
result with final selections. On decline or a request failure, the bridge calls
the operation's known Native Fallback directly; it never re-dispatches the
original keybinding.

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
| `insertNewline` | Operation entry command -> TypeScript bridge -> Rust | A proven control-header block edit applied before native Enter | General formatting or situations where Rust declines |
| Typing `{`, `(`, `)` | Language configuration | Bracket pairing and other regex-local rules | Syntax-aware scaffolding |
| `indent` | Operation entry command -> TypeScript bridge -> Rust | A proven blank line following a complete unbraced `if` body | General Tab behavior, snippet navigation, or uncertain control scope |
| `insertSpace` | Operation entry command -> TypeScript bridge -> Rust | Opening a declaration-tail choice list after a complete, uninitialized `array`, `set`, or `map` field/local | General Space behavior or declarations whose shape is not proven |
| Completion acceptance | Rust completion result plus VS Code snippet support | Keyword skeletons and editable defaults | Reimplementing Tab navigation |
| Future operation, e.g. `insertText` | Operation entry command -> TypeScript bridge -> Rust | A separately proven action such as atomic block-comment expansion | A free-form relay for every printable key |

An operation vocabulary entry has a canonical Native Fallback and tests. Rust
cannot request an arbitrary VS Code command name. VS Code keybinding
precedence remains authoritative: user remappings, platform-reserved input,
and IME composition may bypass an operation command by design.

## Correctness Contract

- Do not bind every printable key or maintain semantic state for ordinary
  typing. Native VS Code input remains the default.
- TypeScript may reject impossible editor states, but Rust alone decides
  eligibility and the resulting structural edit.
- A handled operation is one versioned edit with its final selections. Stale,
  failed, unsupported, composition, snippet, read-only, or ambiguous cases
  use the operation's Native Fallback.
- A migrated behaviour must be pre-native and atomic. Do not recreate it from
  a post-edit document-change listener.
- New operations need Rust decision tests, TypeScript fallback/application
  tests, and an extension-host check that native input has not visibly been
  corrected after the fact.

The implemented operations are intentionally narrow examples of this policy;
their eligible source shapes, edits, and limits are maintained in code and
tests. Add another operation only when it has a distinct semantic owner and a
known native fallback.
