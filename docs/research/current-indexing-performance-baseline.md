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
- The pre-change measurements remain the comparison baseline. The implemented
  PAC-backed acceptance results are recorded below.

## PAC-backed acceptance snapshot

The issue #40 implementation was measured on 2026-07-30 against the installed
base-game `data/data007.pak` and `core/data.pak` using the development server
and the real LSP publication path.

| Measurement | Process to external index ready | Files |
| --- | ---: | ---: |
| First PAC-backed build and cache publication | **1,770 ms** | 6,495 |
| Unchanged PAC revision validation and cache load | **198 ms median** (189-199 ms, five runs) | 6,495 |

The first run includes PAC catalogue inspection, selective decode and parse,
semantic-cache write, immutable external-index publication, and LSP
notifications. The warm run includes process launch, strong selected-payload
identity, cache validation/deserialization, lookup-map reconstruction, and
publication. Neither run creates physical source files.

The final warm path intentionally does more correctness work than the earlier
146 ms prototype: it hashes only the 27.6 MB of selected compressed script
payloads so same-size changes cannot reuse a stale revision. Compared with the
earlier loose-file measurement, unchanged readiness still fell from 454 ms to
a 198 ms median, while the cold path avoided the separate 3.2-3.6 second
physical extraction step.

The representative final cold log attributed 39 ms to PAC catalogue plus
selected-payload identity, 1,065 ms to verified selective decode/parse/index
build, and 174 ms to the 26.7 MB semantic-cache write; external publication was
ready at 1,532 ms inside the process. The 517-add-on inventory-manifest worker
ran independently from that base layer. A representative warm log
attributed 36 ms to PAC identity, 4 ms to cache file read, 50 ms to binary
decode, and 49 ms to lookup map reconstruction; external publication was ready
at 191 ms inside the process. An earlier extension-side size/mtime inventory
pass for all 517 discovered add-ons measured 15 ms, but that weak signal is no
longer trusted for reuse: the final implementation validates non-base projects
and PAC identity in Rust on an independent worker. It uses an available
Reforger manifest hash and falls back to selected-script content identity,
never a full multi-gigabyte payload scan. The base readiness table includes
process launch and client-observed notification latency; it does not treat
completion of deferred, non-semantic add-on manifests as part of base API
readiness.

These are local wall-clock observations rather than portable budgets. Fixture
acceptance tests separately verify cache reuse, immutable revision and
current-pointer publication, cancellation safety, same-size source-change
detection, on-demand virtual-source reading, GUID-keyed inventory manifests,
repair of missing/corrupt GUID manifests, duplicate-GUID rejection,
byte-identical Workbench-core exclusion, and absence of an extracted script
tree.

## Cached-locator pack-read optimization

The Game Data full-text path was measured again on 2026-08-02 with the
development server, the installed add-on cache, and the broad literal query
`SCR_`. Each process considered 8,626 sources and stopped after the bounded
10,000-result ceiling. The comparison used one uncounted warm-up for each
binary followed by seven alternating baseline/candidate runs, preventing one
binary from consistently receiving the warmer filesystem state.

| Measurement | Reinspect PAC catalogue | Open from validated locator | Change |
| --- | ---: | ---: | ---: |
| Source-read median | 267 ms (253-358 ms) | **207 ms** (193-222 ms) | **-22.5%** |
| End-to-end search median | 301.42 ms | **240.39 ms** | **-20.2%** |
| Text scan median | 15 ms | 14 ms | -1 ms |

The candidate removes PAC catalogue reparsing only after an immutable locator
revision has been loaded and its archive artifact stamps have been validated.
The pack reader still checks the locator's archive identity and byte bounds,
enforces extraction and expansion limits, and verifies the captured compressed
payload SHA-256 before returning content. The scan time did not increase, so
the measured source-read reduction was removed work rather than deferred work.

A separate first observation from a newly copied baseline executable reported
3,012 ms of source-read time; subsequent warm runs were 280-317 ms. That cold
observation combines process/executable and filesystem cold-start effects and
is recorded as an outlier, not used in the paired optimization percentage.

## Repeated pack-read experiments

The same installed scope was measured on 2026-08-02 in one long-lived MCP
process to distinguish first-read cost from repeated work. The scope contained
8,391 packed scripts whose locators referenced 9,771,262 compressed bytes and
37,046,590 decoded bytes. Including source identity metadata, retaining that
installed packed corpus is estimated at 38.15 MiB; the accepted cache refuses
corpora above 64 MiB and never retains the multi-gigabyte PAC files themselves.

Seven measured pairs used one binary with a temporary cache-disable switch,
alternating the switch order after one warm-up per condition. This isolated the
cache from concurrent shared-worktree changes. Each run read
`SCR_BaseGameMode.c` nine times, then issued the distinct queries `SCR_`,
`override`, `typename`, and `Replication` against 8,626 total packed and loose
sources. The temporary switch was removed after measurement.

| Measurement | No decoded reuse | Bounded corpus reuse | Change |
| --- | ---: | ---: | ---: |
| First text search | 160.84 ms | 153.26 ms | paired median -4.13 ms |
| Later distinct text search | 153.58 ms | **17.72 ms** | **-88.5%** |
| Later source acquisition | 129 ms | **2 ms** | **-98.4%** |

A rejected per-entry decoded cache reduced later searches to 45.75 ms but
increased the first search from 182.12 ms to 192.42 ms and required one cache
cell on every locator. It changed repeated single-file reads only from 4.88 ms
to 4.45 ms. The accepted catalogue-owned corpus has one revision-and-scope
slot, shares source contents through `Arc<str>`, has no LRU or background fill,
and moves the already-scanned corpus into that slot without cloning it.

Six reads of the same packed source spaced one second apart measured 3.94 ms
median before and 3.82 ms with the candidate. This falsified retained archive
handles and scratch-buffer pools as worthwhile next steps. A controlled attempt
to reuse the corpus for bounded source reads improved their paired median by
only 0.16 ms, so that branch was removed rather than expanding the cache's
responsibility.

### First corpus materialization

Fresh MCP processes were measured against the same 8,626-source loaded scope
to isolate the first full-text query. Seven paired runs alternated executable
order. Valid UTF-8 now takes ownership of its decompression buffer instead of
copying the decoded source into a second allocation. Independent add-on source
batches are read by at most four scoped workers; each worker opens one archive
at a time and retains only selected decoded scripts, so the potentially
multi-gigabyte PAC files are never loaded into memory.

| Measurement | Original | Accepted | Change |
| --- | ---: | ---: | ---: |
| First text search | 172.82 ms | **117.00 ms** | **-32.3%** |
| First source acquisition | 157 ms | **100 ms** | **-36.3%** |
| Text scan | 11-13 ms | 11-13 ms | unchanged |

Isolating only the zero-copy UTF-8 conversion reduced the first query from
172.82 ms to 157.81 ms and source acquisition from 157 ms to 141 ms. Four
workers then reduced a 159.65 ms single-worker query to 118.67 ms. A direct
two-versus-four comparison measured 128.23 ms and 117.00 ms respectively, so
the existing four-worker ceiling earned its bounded concurrency.

A diagnostic build that skipped compressed-payload SHA-256 verification saved
about 11 ms. That prototype was removed: the digest check protects the
revision-bound source contract, and eliminating it would trade correctness for
latency. No persistent archive handles, buffer pool, background preload, or
whole-pack cache was added.
