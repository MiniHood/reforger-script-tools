---
name: implement
description: "Implement a piece of work based on a spec or set of tickets."
disable-model-invocation: true
---

Implement the work described by the user in the spec or tickets.

For ticket work, read and follow `/tdd` before implementation. Create a slash
goal for completing the entire ticket, keep it active through every acceptance
criterion, and do not end the run after a partial implementation, review, or
commit. Complete required public-behavior tests, verification, documentation,
commit/push, and the tracker final state before marking the goal complete; use
the goal's blocked state only for a genuine required external-state or user-input
blocker.

Use TDD at the ticket's pre-agreed public seams.

Run typechecking regularly, single test files regularly, and the full test suite once at the end.

Once done, use /code-review to review the work.

Commit your work to the current branch.
