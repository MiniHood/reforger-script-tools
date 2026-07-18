# Fix semantic-token runtime plan

## Goal

Keep semantic-token work and client refresh traffic bounded while preserving
revision-safe rich projections.

## Findings

1. Multiline token splitting can expand a capped raw-token list beyond the
   configured output cap.
2. `workspace/semanticTokens/refresh` requests are emitted independently by
   workspace updates, external-generation changes, and rich-token completion,
   allowing duplicate and burst traffic.

## Implementation units

### U1 - Bound the final semantic-token stream

- Enforce the token cap after multiline splitting and defensively during
  encoding.
- Add a regression with an unterminated multiline comment larger than the cap;
  assert the encoded stream remains capped and preserves valid token records.

### U2 - Coalesce semantic-token refresh requests

- Centralize refresh issuance behind one stateful queue with at most one
  in-flight request and one dirty follow-up.
- Route workspace/external-generation and rich-projection triggers through the
  queue and handle matching JSON-RPC responses to release or issue the pending
  refresh.
- Add framed LSP regressions for duplicate workspace generation signals and a
  refresh burst followed by a client response.

## Verification

- Focused semantic-token and framed LSP regressions.
- `cargo fmt --check`
- `cargo test` from `server/`
- `npm test` because the packaged server build is part of the extension test
  command.
