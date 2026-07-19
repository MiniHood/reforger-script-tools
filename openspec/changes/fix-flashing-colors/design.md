## Context

Semantic-token responses are full-document replacements. The current Rust LSP
returns a current-revision lexical baseline immediately after an edit, then
refreshes it with rich resolver-backed tokens. This makes externally resolved
symbols visibly lose their color while rich analysis runs. Rich work is already
latest-wins, bounded, cancellation-aware, and immediately runnable.

## Goals / Non-Goals

**Goals:**

- Keep the previous editor coloring visible during ordinary edits.
- Publish only matching-revision rich token data as the next replacement.
- Preserve foreground priority, cancellation, bounded admission, and privacy-safe
  operational logging.

**Non-Goals:**

- Reuse old semantic token ranges as a response for changed source.
- Add a user setting, TypeScript language logic, token deltas, or a fixed timer.
- Change lexical token classification or resolver accuracy.

## Decisions

### Defer current token responses until rich data is ready

When an open, previously colored document changes, retain its pending semantic
token request instead of replying with a lexical baseline. Start or join the
matching revision's rich job as soon as foreground/semantic analysis permits,
then answer the deferred request with that rich projection. VS Code retains the
last rendered token result while the request is outstanding, avoiding a visual
downgrade.

This is preferred over returning stale prior-revision token ranges because
edits can shift ranges or change semantic resolution. It is preferred over a
fixed delay because the wait is only for actual current-revision work.

### Bound and supersede deferred token requests by revision

Store pending semantic-token requests separately from deferred language-feature
requests, keyed by document URI, revision, and external generation. A newer
edit, close, or client cancellation removes older pending token requests without
sending stale data. A bounded overload or supersession returns the standard LSP
`ServerCancelled` error so VS Code can retrigger instead of retaining an
unbounded request. Reuse the existing bounded per-URI request capacity and
rich-task cancellation token.

### Explicit fallback policy

For an already colored document, overload, rejected analysis, or cancelled
current rich work does not trigger a lexical downgrade; the server sends a
bounded `ServerCancelled` response so the client can retrigger instead of
leaving an unbounded request outstanding.
For first-open documents with no prior semantic-token response, or a document
that has never established a rich display, the server may return the safe
lexical baseline so the editor is not blank.

### Refresh remains result-driven

Rich completion continues to issue the semantic-token refresh request. The
refresh obtains the matching rich projection; it does not introduce a second
coloring implementation or publish lexical tokens as an interim replacement.

## Risks / Trade-offs

- **Rich analysis can take longer on large files** → Existing colors remain
  visible instead of flashing; work remains capped and latest-wins.
- **An outstanding token request can be superseded while typing** → Discard it
  by revision and rely on VS Code's next request, never respond with old ranges.
- **First-open has no old display to preserve** → Use the current lexical safe
  baseline only for that case.
- **Overload can delay visual convergence** → Preserve existing display and log
  the unavailable current revision; do not trade correctness for a downgrade.
