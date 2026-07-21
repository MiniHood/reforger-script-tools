# tools/fixtures/formatting/

## Purpose

Contains small, provenance-labelled Enfusion Script inputs for formatter and
documentation-assist research and tests.

## Ownership

These are development-only language-tooling fixtures. They are not extension
runtime dependencies and are not compiler truth unless their individual truth
status says `Workbench-confirmed`.

## Current Behavior

`comment_doxygen_matrix.c` is a pending-Workbench matrix for documentation
comment delimiters, tags, declaration kinds, trailing documentation,
attributes/directives, and unsupported-looking forms. It supports parser and
corpus-review work but does not authorize a formatter to rewrite any form.

## Dependencies and Boundaries

Keep the fixture source-faithful and label its evidence status. Do not make a
fixture's parser acceptance imply Workbench behavior, Doxygen attachment, or
formatter eligibility.

## Verification

Run `cargo test preserves_comment_matrix_delimiters_as_raw_trivia --manifest-path
server/Cargo.toml` to verify lexer preservation. Once a Workbench matrix run is
recorded, update the fixture's truth-status comment with the versioned evidence
rather than silently treating it as confirmed.
