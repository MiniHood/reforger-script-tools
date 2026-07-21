## Context

The existing Enter bridge receives a document-change event after VS Code has
inserted a line. It can prove an unbraced `if` relationship in Rust, but any
scope correction is necessarily a second edit and can visibly move the caret.
VS Code language configuration has synchronous paired `onEnterRules`: one
rule indents after a matched header and the other outdents after its immediate
physical body line. That matches the desired unbraced `if` interaction without
examining the controlled statement.

## Goals / Non-Goals

**Goals:**

- Preserve the Enfusion unbraced `if` style.
- Indent exactly the next line after a complete standalone `if (...)` header,
  including `else if (...)` and standalone `else` headers.
- Let VS Code return the following line to normal indentation without a
  language-client request, selection update, or deferred edit.
- Keep semicolon insertion independent from indentation.

**Non-Goals:**

- Do not insert braces, classify a body statement, or special-case `return`.
- Do not add regex-driven source formatting, an Enter command override, or
  TypeScript syntax logic.
- Do not enable unbraced `for`, `while`, `foreach`, `switch`, or `do` in this
  slice; each needs its own corpus and editor-journey evidence.

## Decisions

### Use native single-next-line indentation

`language-configuration.json` SHALL use paired narrow `onEnterRules` for
complete standalone `if`, `else if`, and `else` headers. VS Code evaluates
the rules as part of Enter, so the initial caret location is final and no
visual correction occurs. The first rule indents the body line; the second
outdents after the immediate non-comment body line.

The pattern is deliberately presentation-only. It must require a line that
starts with the control keyword, has a closing condition delimiter where
applicable, and has no trailing body, brace, semicolon, or comment. It is not
a parser and must fail closed when a line is uncertain.

### Remove deferred scope layout edits

Rust's Enter plan SHALL no longer emit a scope-exit whitespace edit. The
client SHALL no longer apply a deferred layout/caret edit for Enter. The
existing bounded semicolon classification remains independent because it does
not decide the next line's scope.

### Keep wider formatting explicit

Parser-backed document/range formatting and future structural code actions
remain Rust-owned. They can normalize completed source through an explicit
single editor transaction, but do not compete with native typing behavior.

## Risks / Trade-offs

- [A declarative regular expression cannot parse arbitrary Enfusion syntax]
  -> Restrict it to clear header shapes and test matched and rejected lines.
- [A one-line body can itself continue across physical lines] -> Let existing
  parenthesis/bracket behavior handle continuation; this change only owns the
  control header's first following line.
- [Future control forms differ from `if`] -> Add each form only after separate
  source/corpus evidence and native editor validation.
- [Users disable native VS Code auto indentation] -> Respect their editor
  setting; do not replace it with a custom asynchronous handler.
