# Current physical game-data indexing performance baseline

**Purpose.** This is the pre-PAC-backed-index baseline for deciding whether a
new source representation improves real user-facing readiness. It records the
current downloaded Game Data path, not a hypothetical virtual-source result.

## Scope and method

Measurements were taken on 2026-07-30 from the installed global-storage Game
Data cache:

| Input | Value |
| --- | ---: |
| Script files | 6,495 |
| Script bytes | 27,640,746 |
| Persisted semantic cache | 26,526,577 bytes (25.30 MiB) |
| Persisted public symbols | 143,144 |
| Game-data revision | `2735631ce1400eaf9f1761c66cdee10c46921d37` |

The repeatable commands were:

```powershell
$env:CARGO_TARGET_DIR = ".cache/cargo/index-cache-baseline"
node tools/index-cache-baseline.mjs --out .cache/research/current-index-cache-baseline.md --scripts <global-storage>\game-data\scripts --metadata <global-storage>\game-data\metadata.json --cache <global-storage>\index-cache\game-data-symbol-index.v12.bin
node tools/lsp-startup-baseline.mjs server/target/release/reforger_language_server.exe 9
```

The cache timing used the repository's repeatable
`tools/index-cache-baseline.mjs` benchmark against the live v12 cache. It
measures the server's index-load/build code only; Cargo compilation was kept
out of the results with an isolated Cargo target directory. The LSP timing
used `tools/lsp-startup-baseline.mjs` with the release executable and no
external source arguments, so it measures process-to-`initialize`, not Game
Data readiness or VS Code activation.

## Current results

### Release index path

| Measurement | Total | Important components |
| --- | ---: | --- |
| Existing persisted-cache hit | **454 ms** | source fingerprint/digest 343 ms; cache read/decode/validate 60 ms; lookup-map rebuild 50 ms |
| Forced cache miss: parse and write cache | **1,979 ms** | source fingerprint/digest 223 ms; direct rebuild 1,387 ms; serialize/write 198 ms |
| Direct rebuild, no persistence | **1,398 ms** | includes walking and parsing all 6,495 source files |

The cache hit contains the same 6,495 files and 143,144 persisted symbols as
the direct build. It is 67.5% faster than the direct rebuild in this run. The
cache therefore remains required for fast warm starts.

The current validator computes a content digest by walking every `.c` file and
reading its bytes before it trusts a cache hit. This accounts for 343 ms, or
about 76% of the observed 454 ms warm-cache load. A PAC design must not replace
this with a complete multi-gigabyte PAK hash at every startup. Its comparison
baseline is a small add-on manifest and pack-set fingerprint.

### Server initialization without external data

Nine release-process measurements from process spawn to the LSP `initialize`
response were:

`10.56, 10.36, 10.38, 10.49, 9.82, 9.86, 9.92, 10.57, 9.95 ms`.

| Minimum | Median | Maximum |
| ---: | ---: | ---: |
| 9.82 ms | **10.36 ms** | 10.57 ms |

The external index is deliberately built on a background thread, so this is
not the time at which Game Data features become available. The relevant
readiness baseline for unchanged downloaded Game Data is the 454 ms cache hit
above.

### Physical PAC extraction context

The separate PAC-reader experiments measured full base-game script
materialization (5,776 scripts from `data007.pak` plus 719 from `core/data.pak`)
at **3.610 s** for the prior single-worker path. The best experimental
physical-write variant was **3.202 s** using offset ordering and four workers.
Those measurements include creating 6,495 physical output files, and are not
part of the 454 ms already-extracted-cache load above.

## Interpretation for the PAC-backed design

The next implementation must be measured against three separate baselines;
they answer different questions and must not be added together:

| PAC-backed measurement | Current baseline | Required outcome |
| --- | ---: | --- |
| First add-on index, including parse/decode | 1,398 ms direct rebuild, plus no physical extraction | Preserve all semantic counts and remove loose-file materialization |
| Unchanged add-on validation and index readiness | 454 ms | Use a pack-set manifest; avoid per-script walks and full PAK hashing |
| Bare LSP initialization | 10.36 ms median | Do not move add-on indexing onto the request-handling startup path |

Keep one persisted semantic index per add-on. The source bytes remain in PAK
files, while the index persists only the derived semantic facts and a stable
source locator. Startup should validate each add-on manifest, load valid
indexes, and rebuild only changed add-ons. A virtual document read for an
individual definition target is a separate on-demand measurement.

## Caveats

- These are local wall-clock observations, not portable performance budgets.
- The debug cache-hit run was 33.439 s because its fingerprint phase was an
  anomalous 33.296 s; debug timing is intentionally excluded from the release
  decision.
- The LSP test has no VS Code client, extension activation, workspace index,
  or Game Data arguments.
- A PAC-backed implementation has not been benchmarked yet. This document is
  the acceptance baseline, not evidence that virtual sources are faster.
