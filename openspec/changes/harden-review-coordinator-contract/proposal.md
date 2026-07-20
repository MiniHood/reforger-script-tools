## Why

The review workflow can produce inconsistent rosters or wait forever when
inputs conflict or a reviewer never returns. Its acceptance matrix also misses
these and other isolation/safety invariants.

## What Changes

- Define deterministic request grammar, precedence, invalid-input behavior,
  and an explicit persona-only form.
- Define specialist tie-breaking and bounded reviewer liveness.
- Expand acceptance scenarios for selection, liveness, isolation, and active
  review safety.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `parallel-code-review`: Make roster resolution and reviewer terminal states
  deterministic and acceptance-tested.

## Impact

Updates only repository-local `/review` contracts and documentation.
