## 1. Semantic-token request lifecycle

- [x] 1.1 Add revision-scoped deferred semantic-token request ownership and
  cancellation that is separate from ordinary deferred language-feature
  requests.
- [x] 1.2 Preserve the prior rendered semantic-token result by withholding an
  edited document's lexical-only full response until matching rich tokens are
  available.
- [x] 1.3 Keep first-open lexical fallback behavior and make overload,
  cancellation, close, and newer edits suppress stale token publication.

## 2. Rich token delivery

- [x] 2.1 Ensure current-revision rich analysis is scheduled immediately from
  the token request/analysis lifecycle with no fixed idle delay.
- [x] 2.2 Complete deferred token requests and semantic-token refreshes only
  from a matching revision and external-index generation rich projection.
- [x] 2.3 Add concise diagnostic fields for deferred, superseded, rich-ready,
  and unavailable token request outcomes.

## 3. Verification and documentation

- [x] 3.1 Add Rust regressions for stable external-type coloring during edits,
  rapid-edit supersession, first-open fallback, and unavailable rich work.
- [x] 3.2 Update semantic-token/runtime reference documentation with the
  stable-display and current-revision publication contract.
- [x] 3.3 Run focused Rust tests, full `cargo test`, extension compile/lint,
  and inspect fresh `GC_MarkerArea` logs for the absence of lexical-to-rich
  replacement during normal edits.
