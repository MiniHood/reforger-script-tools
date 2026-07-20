## Context

The specialized `RplRpc` snippet selects a complete default enum expression
and immediately opens VS Code Suggest. The Rust server renders that list with
an `InsertReplaceEdit` whose member-only insert range starts after the
full-expression replace range. VS Code requires both ranges to share a start,
so the protocol item is invalid at the editor boundary.

## Goals / Non-Goals

**Goals:**

- Emit only editor-valid completion edit ranges.
- Keep Rust authoritative for enum candidate selection, ranking, and edits.
- Keep the TypeScript bridge event-driven, one-dispatch-per-authored-placeholder, and free of
  language parsing or completion policy.
- Prove the wire invariant and the RplRpc user journey with focused coverage.

**Non-Goals:**

- Add a completion debounce, retry loop, or second completion provider.
- Generalize RplRpc defaults to arbitrary attributes.
- Offer keywords that cannot satisfy an enum argument.

## Decisions

### Use a single full-expression edit range for the selected enum field

The selected snippet field is the complete `RplChannel.Reliable` expression.
The specialized follow-up list SHALL replace that whole field with an ordinary
text edit (or an insert/replace pair whose ranges have the same start and meet
the VS Code prefix rule). This removes the invalid attempt to filter one
suffix while replacing an earlier-starting expression.

The standard LSP completion request does not know that VS Code selected a
snippet field. A new custom request or TypeScript completion implementation
would add parallel protocol/semantic paths without evidence that a valid LSP
edit cannot meet the required experience, so it is rejected for this change.

### Reuse the normal contextual value fallback beneath enum members

The selected first RplRpc argument is `RplChannel`. The shared static-enum
renderer SHALL place its qualified enum members first and retain the normal
contextual value candidates beneath them: visible locals and parameters,
containing-class and inherited members, top-level symbols, and keywords. The
member path SHALL reuse the same analyzed-snapshot candidate collector as
ordinary value completion rather than reconstructing a top-level-only list.
Every item uses the same complete `Owner.Member` edit range, so accepting a
fallback replaces the selected expression rather than producing an invalid
qualified expression. Typing replaces the selected field and resumes ordinary
completion.

The pending current-snapshot receiver query has no matching whole-file scope
model and SHALL continue to admit only bounded current or immutable external
facts. It must not merge locals or members from an older analysis merely to
fill this fallback list.

### Test and diagnose the editor boundary proportionally

Rust tests SHALL assert the VS Code insert/replace invariant wherever such an
edit is rendered. The Extension Development Host verification SHALL exercise
the visible suggestion journey. Bounded client diagnostics may record range
shapes and filter metadata for the first items, but must not log source text or
full completion payloads.

### Treat the bridge command as a cross-layer placeholder-sequence contract

The Rust-emitted command remains necessary for UI invocation, while TypeScript
continues to own registration and execution. For a multi-enum snippet it SHALL
carry the ordered selected placeholder defaults. TypeScript SHALL dispatch once
for each observed default and retain the transaction through the final field;
it must not parse document text or infer parameter types. Tests SHALL prevent
drift between the emitted command, extension configuration, and manifest
contribution.

## Risks / Trade-offs

- [A full-expression range can change native filtering behavior] → Validate
  the actual widget in a fresh Extension Development Host before release.
- [Value fallbacks are not all assignable to an enum parameter] → The editor
  presents normal completion choices as requested; accepting one replaces the
  entire field, and compiler validation remains the language authority.
- [An editor integration test can be more brittle than unit tests] → Keep it
  narrowly scoped to the exceptional RplRpc bridge and retain fast wire-level
  invariant tests.
