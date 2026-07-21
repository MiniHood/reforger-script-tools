---
name: tdd
description: Test-driven development. Use when the user wants to build features or fix bugs test-first, mentions "red-green-refactor", or wants integration tests.
---

# Test-Driven Development

Use the red-green loop to produce durable public-behavior tests. Consult these
rules before and during every cycle. When exploring a codebase, read
`CONTEXT.md` if it exists and respect applicable ADRs.

## Ticket goal

When this skill is invoked for a ticket, first create a slash goal for
completing that ticket. The goal is complete only after every stated
requirement has been implemented and checked against the ticket, relevant
public-behavior tests have passed, the full required verification has passed,
the change has been reviewed, required documentation has been updated, the
coherent result has been committed, and the ticket tracker has been brought to
its required final state. Keep working through those steps without treating an
individual red-green cycle, a partial build, or an intermediate commit as goal
completion. Do not pause for confirmation when the next step is determined by
the ticket and repository workflow; report progress in commentary instead.

## Public seams

Test behavior through public interfaces, never implementation details. A test
should read like a specification, survive internal moves, and use an
independent expected value.

Treat existing public tests and documented contracts as pre-agreed seams. Write
them down before adding a test. Ask the user to confirm a seam only when a task
introduces new observable behavior and the relevant public boundary cannot be
determined from available context.

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
