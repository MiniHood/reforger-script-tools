---
name: tdd
description: Test-driven development. Use when the user wants to build features or fix bugs test-first, mentions "red-green-refactor", or wants integration tests.
---

# Test-Driven Development

Use the red-green loop to produce durable public-behavior tests. Consult these
rules before and during every cycle. When exploring a codebase, read
`CONTEXT.md` if it exists and respect applicable ADRs.

## Goal gate

For ticket work, create a slash goal before acting. Keep it active until every
stated requirement is implemented and checked, relevant public-behavior tests
and required verification pass, the change is reviewed, required documentation
is updated, the coherent result is committed, and the ticket tracker reaches
its required final state.

Continue while the goal is active. A response may end the run only when the
goal is complete or genuinely blocked by required user input or external state.
Use commentary for progress. A partial implementation, check, review, or
commit is not a terminal state.

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
