# Warm manifest and locator overhead research

**Date:** 2026-07-31  
**Scope:** warm startup of the Rust external-index path. The measurements below
were the design input for the sectioned binary cache implementation.

## Recommendation

Keep JSON for the human-readable recovery/debug manifest, but make the warm
path depend only on a small, mandatory cache header and an immutable locator
table. The highest-value locator change is not merely “use binary instead of
JSON”; it is to stop rebuilding a `BTreeMap<String, PackedSourceEntry>` for
every cached script.

Recommended order:

1. Make `manifest-header.json` authoritative for warm validation and cache
   selection. Always publish it with `symbols.bin`; a missing header should
   be a repair/cold-path condition, not permission to parse the full locator
   manifest on warm startup.
2. Keep the locator-rich JSON manifest only for repair, inspection, and a
   compatibility fallback. Store a versioned locator section beside the
   semantic index inside the same `symbols.bin` container. Use a sorted table
   keyed by logical path, an interned pack-path table, and binary digest bytes.
   Derive the URI instead of storing one per script.
3. Restore a locator table as an immutable sorted vector (or an on-disk
   indexed table), and binary-search it on `read_virtual_source`. Construct a
   `PakEntry` only for the requested source. Do not materialize a URI-keyed
   `BTreeMap` during cache hydration.
4. Benchmark this separately from semantic-index loading. Only investigate
   memory mapping after the binary table has demonstrated that copying the
   locator bytes is measurable.

Do not add `bincode`: its current official crate documentation marks the crate
unmaintained. If a general serializer is still preferred after a prototype,
`rkyv` is the relevant zero-copy comparison, but a small repository-owned
format is a better fit for this fixed table and keeps format/version control
inside the existing cache reader.

## What the current code actually does

The locator-rich manifest stores one `ScriptLocator` per script: URI, logical
path, archive path, offsets and lengths, compression, and a hex SHA-256 string
([`server/src/addon_sources.rs:239-250`](../../server/src/addon_sources.rs#L239-L250)).
The full `AddonIndexManifest` repeats cache identity and adds the entire
locator vector ([`server/src/addon_sources.rs:252-271`](../../server/src/addon_sources.rs#L252-L271)).

The separate header already has the right broad idea: it omits `scripts`, but
still carries all validation metadata and pack-artifact records
([`server/src/addon_sources.rs:273-297`](../../server/src/addon_sources.rs#L273-L297)).
The current working tree also has a catalogue, but its entries are full header
objects ([`server/src/addon_sources.rs:299-304`](../../server/src/addon_sources.rs#L299-L304)).

There are three materially different paths:

- **Ordinary cache hydration:** the cache workers load `symbols.bin` and
  register only `(GUID, revision) -> cache root`
  ([`server/src/addon_sources.rs:1150-1178`](../../server/src/addon_sources.rs#L1150-L1178)).
  This is the correct warm-first shape: no locator JSON is needed until a
  virtual source is requested.
- **First virtual source read:** `read_virtual_source` calls
  `load_cached_source_revision` when the revision is not already materialized
  ([`server/src/addon_sources.rs:1794-1844`](../../server/src/addon_sources.rs#L1794-L1844)).
  The loader reads the header, then `register_cached_source_revision` reads
  and deserializes the full `manifest.json`, creates one `PakEntry` per script,
  derives a URI, and inserts every entry into a `BTreeMap`
  ([`server/src/addon_sources.rs:1315-1367`](../../server/src/addon_sources.rs#L1315-L1367)).
- **Authoritative source validation:** if the compact header is absent, the
  validator falls back to deserializing the full manifest
  ([`server/src/addon_sources.rs:1561-1610`](../../server/src/addon_sources.rs#L1561-L1610)).
  The same fallback exists in lazy locator restoration
  ([`server/src/addon_sources.rs:2363-2393`](../../server/src/addon_sources.rs#L2363-L2393)).

This explains why the manifest can look expensive in traces even though it
should not be on the first usable warm-index path: locator restoration and
source validation are separate consumers of the same JSON artifact.

## Local measurements

The existing PAC acceptance report measured unchanged external-index readiness
at **198 ms median** over five runs. Its representative warm breakdown was:

| Phase | Time |
| --- | ---: |
| PAC identity | 36 ms |
| `symbols.bin` read | 4 ms |
| Binary semantic-index decode | 50 ms |
| Semantic lookup-map reconstruction | 49 ms |

Source: [`docs/research/current-indexing-performance-baseline.md:118-139`](current-indexing-performance-baseline.md#L118-L139).

The important conclusion is that the manifest is **not currently the largest
critical-path item** when lazy locator registration is active. The 49 ms map
rebuild is the larger measured warm cost. Locator work becomes a startup cost
when the first source/definition request forces restoration, or when an older
cache lacks the header and the background validator takes the full-manifest
fallback.

The installed local cache used for this review contained:

| Artifact | Bytes | Script locators |
| --- | ---: | ---: |
| Base-game `manifest.json` | 3,119,057 | 5,776 |
| Add-on `manifest.json` | 362,006 | 719 |
| Both full manifests | 3,481,063 | 6,495 |
| `cache-catalogue.json` | 6,617 | 2 entries |

No `manifest-header.json` files were present in that cache snapshot, so the
full-manifest fallback is not theoretical for existing installations.

Using the exact header field set from the Rust type, a local JSON-size proxy
produced headers of 3,685 bytes and 1,277 bytes for the two manifests. That is
approximately **99.86% less metadata input** than the two full manifests, and
it avoids deserializing 6,495 locator records. A Node `JSON.parse` proxy on the
3.1 MB base manifest measured 4.48 ms median / 5.16 ms p95 versus 0.007 / 0.029
ms for the 3.7 KB header. This is a size/parser proxy, not a Rust acceptance
benchmark; it does not include Rust allocation, URI construction, map inserts,
or file I/O.

The existing semantic cache already demonstrates the useful binary pattern:
it reads a self-described binary payload, validates a magic/schema/shape, and
then builds runtime structures ([`server/src/index_cache.rs:1007-1060`](../../server/src/index_cache.rs#L1007-L1060);
[`server/src/index_cache.rs:1736-1790`](../../server/src/index_cache.rs#L1736-L1790)).
The repository has timing fields for decode and lookup-map rebuild, and the
baseline report already separates them ([`tools/server-reports/index_cache_baseline.rs:262-297`](../../tools/server-reports/index_cache_baseline.rs#L262-L297)).

## Option comparison

| Option | Warm benefit | Complexity | Assessment |
| --- | --- | --- | --- |
| Lazy JSON parsing, current | Near-zero startup cost when no virtual source is read; preserves readable recovery data | Low | Keep as the safety fallback, but do not allow it on warm validation when a header is missing. |
| Smaller JSON/header | Removes roughly 3.48 MB and 6,495 locator deserializations in the local cache shape | Low | Best immediate change. The current header is close, but it must be written reliably and used as the only warm metadata input. |
| Binary locator table | Lower read/parse/allocation cost; likely reduces a 3.1 MB base locator payload to roughly 0.4–0.7 MB with shared strings and 32-byte digests | Medium | Best locator-specific follow-up. Estimate is from the current fields, not a benchmark. |
| Memory-mapped binary table | Can avoid copying the table and support direct indexed reads | High | Not justified yet. Mapping bytes does not remove validation, lookup design, or archive-path handling. |
| Persist/reuse URI maps | Eliminates the O(6,495) URI construction and `BTreeMap` insertion work | Medium | Highest benefit if locator hydration is on the critical path. Prefer a sorted logical-path table over persisting Rust map internals. |

### Lazy JSON parsing

This is already the correct default for warm startup. `serde_json::from_slice`
deserializes a typed value from a byte slice, and the current manifest types use
owned `String`, `PathBuf`, and `Vec` fields. That means the lazy boundary moves
the allocations to first source navigation; it does not make the full manifest
cheap when restored. The official API is documented at
[`serde_json::from_slice`](https://docs.rs/serde_json/latest/serde_json/de/fn.from_slice.html).

### Compact JSON/header

The current header contains more than the warm selector strictly needs: pack
artifact records, display metadata, source-root identity, revision, and cache
shape. It is still tiny compared with the locator vector. A catalogue entry can
be smaller still: GUID, source root, instance key, revision, cache format,
index byte length, and perhaps source precedence. Pack artifact detail belongs
in the authoritative validation header, not in every dependency-selection
record.

The current catalogue is deserialized in full and filtered afterward
([`server/src/addon_sources.rs:2600-2639`](../../server/src/addon_sources.rs#L2600-L2639)).
For the observed two-entry catalogue this is irrelevant; for hundreds of cache
entries it becomes a second, smaller version of the same scaling problem. A
compact GUID-keyed catalogue is worthwhile, but it is lower priority than
eliminating locator restoration and semantic lookup-map reconstruction.

### Binary locator table

A suitable table should contain:

```text
magic + format version + GUID/revision + entry count
string table: logical paths and pack-relative paths
sorted records: logical-string-id, pack-string-id, offset, compressed length,
                original length, compression, 32-byte compressed-payload digest
```

The URI should not be stored: the code already has the GUID, revision, and
logical path needed to derive it. The digest should be stored as 32 raw bytes,
not a 64-character hexadecimal string. A sorted record vector allows lookup by
logical path without allocating a URI-keyed tree. A small optional hash/index
table can be added only if binary search is demonstrated to matter.

The repository should extend its existing bounded custom reader rather than
introduce a generic serializer solely for this table. `bincode` is a poor new
dependency: its official documentation now says development has stopped and
points users toward alternatives ([bincode crate documentation](https://docs.rs/crate/bincode/latest)).
`postcard` has a documented stable wire format and varint encoding, but it is
aimed at compact Serde messages rather than a directly indexable file table
([postcard documentation](https://docs.rs/postcard/latest/postcard/)).

### Memory mapping

`memmap2` maps a file to a byte slice, which is useful for a large immutable
table ([official crate documentation](https://docs.rs/memmap2/latest/memmap2/)).
It does not automatically provide a schema, bounds checks, string decoding,
logical-path search, or a lifetime-safe `PakEntry` view. For a 0.4–0.7 MB
locator table, a normal read may be faster and simpler than introducing page
faults and an unsafe mapping boundary. Measure first.

`rkyv` is a credible later experiment: its official documentation supports
zero-copy access, validation, and archived maps
([rkyv documentation](https://docs.rs/rkyv/latest/rkyv/index.html)). It also
warns that format-control choices can make old serialized data unreadable, so
the cache would still need an explicit repository format/version contract.
The current table is small and has simple fixed-width records; custom binary is
more transparent and avoids coupling `PakEntry` lifetime and archive paths to a
third-party archived object graph.

## Implemented decision and remaining work

The implementation now wraps the existing semantic payload and the optional
locator table in one `RSTCNT17` container. Newly rebuilt add-on caches put the
locator section in that file; warm cache hits do not encode or read it. The
locator section uses an interned pack-path table, sorted logical paths, fixed
width numeric fields, and raw 32-byte digests. `manifest-header.json` remains
the compact validation record, while `manifest.json` remains the JSON repair
and debug fallback. Raw pre-container semantic caches remain readable.

Remaining measurement work is to benchmark first virtual-source lookup against
the JSON fallback and to decide whether a sorted in-memory locator vector can
replace the current URI-keyed map. Memory mapping remains deferred until those
measurements show that copying the locator section is material.

## Bottom line

For the current measured warm startup, manifest/locator work is not the main
198 ms bottleneck: semantic lookup-map reconstruction is 49 ms, versus a
manifest path that should now be lazy. The best manifest improvement is to make
the compact header real and mandatory. The best locator improvement is to
replace eager URI-map reconstruction with a versioned, binary, logical-path
table. Memory mapping is a later optimization, not the first move.
