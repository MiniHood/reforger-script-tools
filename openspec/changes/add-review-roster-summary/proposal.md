## Why

A fixed four-person review is unnecessarily slow for narrow changes, while the final report does not yet provide one prioritized decision surface. Review should select the smallest relevant roster and summarize completed evidence in a clear actionable table.

## What Changes

- Add a risk-based persona catalog and `depth:auto`, `depth:full`, and explicit persona selection.
- Keep Correctness and Architecture as core reviewers; choose relevant specialists with a normal cap of four.
- Add a deduplicated prioritized findings table with evidence, impact, next step, confidence, and contributing reviewers.
- Require partial reviews to disclose unavailable personas and incomplete coverage.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `parallel-code-review`: Select a relevant bounded reviewer roster and render a prioritized synthesis table.

## Impact

- Updates the repository-local `/review` skill and its review contracts only.
