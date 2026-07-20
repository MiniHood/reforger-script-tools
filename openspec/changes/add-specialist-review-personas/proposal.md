## Why

The existing review roster covers broad engineering risks but does not give
Enfusion language truth or test evidence their own independent challenge.
Those concerns recur in language-server work and need focused reviewers that
remain optional for unrelated changes.

## What Changes

- Add Language Fidelity and Verification & Observability persona contracts to
  the `/review` catalog.
- Define risk-based selection rules for both personas while retaining the
  four-reviewer execution cap.
- Document the expanded catalog and its intended use.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `parallel-code-review`: Expand the selectable persona catalog and define
  specialist selection behavior.

## Impact

Updates the repository-local `/review` skill, its persona contracts, and the
agent workflow reference. No extension runtime behavior changes.
