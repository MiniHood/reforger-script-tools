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
