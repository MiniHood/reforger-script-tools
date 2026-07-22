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
| Typing `{`, `(`, `)`, indentation | Language configuration | Bracket pairing, ordinary indentation, and other regex-local rules | Syntax-aware scaffolding |
| Completion acceptance | Rust completion result plus VS Code snippet support | Keyword skeletons and editable defaults | Reimplementing Tab navigation |
| Future operation, e.g. `insertText` | Operation entry command -> TypeScript bridge -> Rust | A separately proven action such as atomic block-comment expansion | A free-form relay for every printable key |

An operation vocabulary entry has a canonical Native Fallback and tests. Rust
cannot request an arbitrary VS Code command name. VS Code keybinding
precedence remains authoritative: user remappings, platform-reserved input,
and IME composition may bypass an operation command by design.

## Performance and Correctness Contract

1. Do not bind every printable key or maintain a semantic `when` context on
   every cursor/edit event. Leave non-candidates to VS Code's default bindings.
2. A TypeScript candidate gate may reject impossible cases using editor state
   and local shape, but must not duplicate Enfusion parsing or ownership
   decisions. Rust selects the one Input Feature Owner through explicit,
   tested priority; features never compose.
3. The initial eligibility boundary is one empty caret. The request represents
   all selections, but non-empty selections and multiple cursors fall through
   natively until a feature defines correct semantics for them.
4. A plausible candidate sends one versioned request and waits for its Rust
   decision without an arbitrary timeout. On request failure or server
   unavailability it immediately uses Native Fallback and ignores a late
   result.
5. Apply a handled result as one source edit and final selection state. It may
   declaratively request that suggestion UI opens afterwards, but it cannot
   run arbitrary VS Code commands, insert a follow-up snippet, or perform a
   second source mutation.
6. A decision is valid only for the exact document version and selection it
   inspected. A stale response is discarded and Native Fallback runs at the
   editor's then-current state.
7. Completion lists, active snippet placeholders, IME/composition input, and
   read-only editors retain native behavior unless a future feature explicitly
   defines compatible semantics.
8. Once a behavior is migrated, no `onDidChangeTextDocument` listener may
   mutate the document to reproduce it. Observers may only trace or clean up
   state.

Optional centralized route traces record operation kind, eligibility or
decline reason, selected feature, version match, result, and elapsed time. They
never record source text or identifiers and are disabled in ordinary use. They
are available in extension development diagnostics.

## Implication for Reforger Script Tools

The first Input Feature Owner is control-header `insertNewline`: `for`,
`foreach`, `while`, and `switch` create their braced bodies before native
Enter. The `switch` result embeds `default:` in its one primary edit, selects
`default`, and may request suggestion UI; it must not insert a second snippet.
`if` and `else` retain native body behavior.

The prior post-native Enter typing assist is removed with this migration,
including its duplicate control-block logic, incomplete-`if` repair, and
automatic semicolon insertion. Block-comment expansion is a future
`insertText` feature: it must become pre-native before it is migrated. Existing
completion and snippet transactions and language configuration remain outside
this module.

Required evidence for a delivery is Rust decision tests for supported and
declined source shapes; TypeScript tests for fallback, failure, stale state,
special modes, selections, and presentation; an extension-host test that the
legacy post-edit path does not run; and a manual VS Code check for no visible
correction or cursor flash.
