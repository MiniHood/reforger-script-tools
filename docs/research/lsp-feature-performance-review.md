# LSP Feature Performance Review

## Scope

This review measured the complete available LSP runtime log first, then used
controlled release-profile workloads for the two paths selected for change:
rich semantic coloring and explicit comment formatting. The goal was to keep
only simple local changes with stable output fingerprints and to check that a
feature-specific gain did not merely shift time into startup or another LSP
operation.

Measurements were collected on August 2, 2026. The host was busy during part
of the run, reaching 50–98% total CPU utilization. Controlled semantic and
startup comparisons therefore used baseline and candidate binaries in
alternating pairs. Results below report medians across each repeated run; they
are local evidence, not portable budgets.

## Whole-Runtime Baseline

The existing runtime-performance report parsed 61,062 records from the local
language-server log. This historical capture establishes priority and feature
coverage; it is not a controlled before/after workload.

| Operation | Samples | Average | P95 | Maximum |
| --- | ---: | ---: | ---: | ---: |
| Rich semantic coloring | 6,224 | 17 ms | 72 ms | 158 ms |
| Foreground semantic-token return | 5,062 | below 1 ms | below 1 ms | 16 ms |
| Completion | 11 | 18 ms | 55 ms | 55 ms |
| Definition | 34 | 2 ms | 16 ms | 24 ms |
| Document symbols | 451 | below 1 ms | 1 ms | 3 ms |
| Hover | 54 | below 1 ms | 1 ms | 2 ms |
| Active scope delimiters | 773 | below 1 ms | below 1 ms | 1 ms |

The foreground token response is already cheap because it returns the settled
projection. Background rich coloring is the meaningful semantic cost. Live
records split that work further: member resolution is the largest consistent
resolver subphase, while external lookup, declaration overlay, delimiter
overlay, and encoding are usually below the millisecond log resolution.

Formatting had no live runtime samples, so it was measured with the retained
`formatting_benchmark` developer workload instead of inferred from absence.

## Changes Selected

### Semantic preprocessor-line classification

Rich and lexical semantic-token projection previously scanned backward through
the source for every non-whitespace token to rediscover whether the token's
line began with `#`. That made this classification quadratic in document size.
The replacement performs one linear pass to collect preprocessor-line spans,
then advances through those spans monotonically while visiting the already
ordered lexer tokens.

### Comment-only formatting validation

Comment formatting previously searched the full comment-token list for every
non-whitespace character in every selected line. The replacement starts at the
first comment that can intersect a line and advances through comment spans as
the characters advance. It preserves lines containing multiple block-comment
tokens and keeps the existing partial-comment rejection behavior.

No resolver cache, manager, scheduler, protocol, or setting was added. Member
resolution was left unchanged because the review did not establish a similarly
small improvement with an isolated correctness boundary.

## Controlled Before and After

Every semantic run used the same source, external index, token count, resolver
call count, and encoded-token fingerprint. Every formatting run used the same
edit fingerprint.

| Workload | Baseline median | Candidate median | Change | Output gate |
| --- | ---: | ---: | ---: | --- |
| Semantic coloring, 32 KB / 5,221 tokens | 13 ms | 12 ms | 7.7% faster | identical `16db9f702e6a198d` fingerprint |
| Semantic coloring, 85 KB / 11,105 tokens | 31 ms | 30 ms | 3.2% faster | identical `119e8e34d31f6092` fingerprint |
| Semantic coloring, 85 KB P95 | 39 ms | 35 ms | 10.3% faster | same runs as above |
| Comment formatting, 22.6 KB / 500 lines | 1,666 us | 165 us | 90.1% faster (10.1×) | identical `f0ffe871a8974d9c` fingerprint |

The 32 KB semantic P95 moved from 16 ms to 17 ms while the host was saturated,
so that individual tail result is noise rather than evidence of improvement.
The larger source shows the expected size-dependent benefit. The semantic
change is retained because it removes quadratic work, improves the larger-file
tail, and does not change classification output; the formatter gain is large
and decisive.

## Whole-Path Check

Release server startup was measured in three alternating seven-start pairs.
The median of the per-run medians was 17.49 ms for the baseline and 18.20 ms
for the candidate. The 0.71 ms difference occurred while total host CPU ranged
up to 98%, and neither changed function executes during initialization. This
does not establish a startup regression, but it also is not claimed as a gain.

The final verification gate covers the complete Rust server test suite and the
repository compile path. Runtime code remains inside the existing semantic and
formatting owners, and foreground request scheduling, external indexing,
completion, hover, definition, symbols, diagnostics, and transport are
unchanged.

## Reproduction

Use an ignored or external Cargo target directory for all commands.

```powershell
cargo run --release --manifest-path server/Cargo.toml `
  --example lsp_semantic_tokens_benchmark -- `
  --scripts <external-scripts-root> --file <large-script> --iterations 31

cargo run --release --manifest-path server/Cargo.toml `
  --example formatting_benchmark -- --lines 500 --iterations 11

node tools/lsp-runtime-performance-report.mjs `
  --global-storage <extension-global-storage> --out <report-path>

node tools/lsp-startup-baseline.mjs <release-server-path> 7
```

The semantic benchmark rejects token-count, resolver-call, or encoded-token
fingerprint drift between iterations. The formatting benchmark rejects edit
fingerprint drift.

## Completion Follow-Up

A deeper completion review on the same date parsed the expanded 64,154-record
runtime log and then exercised 1,594 real completion positions sampled from
2,000 extracted game-data files. The live capture still contains only 11
completion requests, so it establishes user-path shape but not a controlled
before/after result.

| Live operation | Samples | Average | P95 | Maximum |
| --- | ---: | ---: | ---: | ---: |
| Rich semantic coloring | 6,563 | 18 ms | 75 ms | 204 ms |
| Document analysis | 6,324 | 3 ms | 10 ms | 61 ms |
| Foreground analysis publication | 6,326 | 1 ms | 3 ms | 20 ms |
| Completion | 11 | 18 ms | 55 ms | 55 ms |
| Foreground semantic-token return | 5,274 | below 1 ms | 1 ms | 16 ms |

Candidate lookup accounted for 155 of the 203 completion milliseconds in that
capture. The controlled corpus made the same boundary much clearer: lookup
accounted for 22,310 of 23,348 reported completion milliseconds in the first
release run. Context detection, receiver inference, and item rendering were
not competitive hotspots.

Top-level completion previously collected fuzzy matches into an ordered map,
fully sorted every surviving match, and only then retained the editor's first
251 candidates. The replacement uses the existing fast hash map for temporary
grouping, partitions to the bounded candidate set, and sorts only that set.
The final comparator and candidate rendering are unchanged. A direct repeated
query produced the same SHA-256 completion-list fingerprint before and after.

Three alternating full-corpus pairs separated baseline and candidate
executables. The table reports the median of each run's aggregate time over the
same 1,594 requests.

| Completion corpus phase | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| Projection wall time | 12,360 ms | 8,966 ms | 27.5% faster |
| Candidate lookup | 10,851 ms | 7,447 ms | 31.4% faster |
| Reported completion total | 11,462 ms | 8,046 ms | 29.8% faster |

The retained repeated query returned 250 items with fingerprint
`9e4eb0af5929687152489f67a68a977f3ffa4a005f93e0277698ea99bcd5b540`.
Its wall median moved from 13,208 us to 8,874 us (32.8%) and P95 from
17,372 us to 13,306 us (23.4%).

No cost moved into the other measured projections. Current-candidate checks
reported 9 ms semantic-coloring median for a 32 KB source, 24 ms for an 85 KB
source, 148 us formatting median for the retained 500-line workload, and
12.43 ms median over seven server starts. The completion code does not execute
on those paths, so these checks are regression evidence rather than claimed
completion-derived gains.

Background rich semantic coloring remains the largest recurring runtime cost.
Member resolution is still its largest measured resolver subphase; on the
current 85 KB workload, resolver work used 15 of 24 median milliseconds and
member lookup used 5 milliseconds. Completion candidate lookup remains the
largest completion subphase even after this change. Further work should start
at those two boundaries, but the cached-analysis re-lexing helpers are not a
current priority: completion context detection was only 251 milliseconds over
the full candidate corpus.

For a repeated completion measurement with output and latency gates, run:

```powershell
cargo run --release --manifest-path server/Cargo.toml `
  --example lsp_completion_benchmark -- `
  --scripts <external-scripts-root> `
  --file <source-file> --line <one-based-line> `
  --character <zero-based-character> --iterations 31 `
  --expect-fingerprint <sha256> --max-median-us <local-budget>
```
