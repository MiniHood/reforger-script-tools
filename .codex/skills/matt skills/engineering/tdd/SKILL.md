---
name: tdd
description: Test-driven development. Use when the user wants to build features or fix bugs test-first, mentions "red-green-refactor", or wants integration tests.
---

# Test-Driven Development

Use the red-green loop to produce durable public-behavior tests. Consult these
rules before and during every cycle. When exploring a codebase, read
`CONTEXT.md` if it exists and respect applicable ADRs.

## Goal gate

For ticket work, create a slash goal before acting and keep an explicit
checklist of every acceptance criterion, test, verification step,
documentation obligation, commit/push requirement, and tracker transition.
Pursue the goal until every item is evidenced and the tracker reaches its
required final state (normally closed); a partial implementation, passing
subset of checks, review, commit, push, or report is never terminal. End only
when complete or genuinely blocked by required user input or unavailable
external state. Difficulty, uncertainty, an unfinished design, a missing test,
or an implementation path that still needs to be worked out are not blockers:
continue investigating and implementing them. Before marking a goal blocked,
identify the exact external dependency or user decision that makes further
progress impossible, confirm that safe in-scope alternatives are exhausted,
and keep the goal active unless the platform's blocked-state threshold has
actually been met. Use commentary for progress and re-check the checklist
before declaring completion.

## Public seams

Test behavior through public interfaces, never implementation details. A test
should read like a specification, survive internal moves, and use an
independent expected value.

Treat existing public tests and documented contracts as pre-agreed seams. Infer
new seams from the ticket, existing tests, and documentation; ask only when a
material product decision cannot be determined safely from available context.

See [tests.md](tests.md) for test examples and [mocking.md](mocking.md) for
boundary-mocking guidance.

## Anti-patterns

- Do not test private helpers, internal calls, or module structure.
- Do not derive an expected value by repeating the implementation.
- Do not write a horizontal batch of imagined tests. Work in vertical slices:
  one seam, one test, one minimal implementation.

## Loop

- Write a failing public-behavior test before adding new observable behavior.
- Implement only enough to make that test pass, then run it.
- For a behavior-preserving refactor, use the existing green public-seam tests
  as the contract. Make one cohesive module move and run the smallest relevant
  public-behavior verification. Do not defer the refactor to review.
- Add a new red test during a refactor only when it introduces observable
  behavior not covered by the agreed seams.
