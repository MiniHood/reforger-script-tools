# Add-on PAK Extraction and Index Identity Research

**Status:** investigation only; no runtime contract is adopted by this note.

## Question

Replace the single GitHub-downloaded base-game source corpus with independently
maintained indexes for every add-on loaded by the current Reforger project,
including `ArmaReforger`. Establish which facts should identify an add-on and
what source/extraction boundary is safe and performant.

## Evidence and confidence

### Official Reforger documentation

- A `.pak` is a Workbench game/mod data archive, while a `.c` file is Enforce
  Script source. [File Types: `.pak` and `.c`](https://community.bohemia.net/wiki/Arma_Reforger%3AFile_Types)
  (high confidence).
- Reforger loads add-ons by IDs from their `.gproj`; GUID is the preferred ID,
  with project ID and directory name documented as lower-precedence choices.
  It searches the profile, executable `addons`, and configured `-addonsDir`
  locations. [Startup Parameters: `-addons` and `-addonsDir`](https://community.bohemia.net/wiki/Arma_Reforger%3AStartup_Parameters?useskin=vector)
  (high confidence).
- Workshop downloads are stored in an `addons` subdirectory of the configured
  `-addonDownloadDir`; the same documentation says it is unnecessary to pair
  this option with `-addonsDir`. [Startup Parameters: `-addonDownloadDir`](https://community.bohemia.net/wiki/Arma_Reforger%3AStartup_Parameters?useskin=vector)
  (high confidence).
- A Workbench `-gproj` selects the add-on project to load. Workbench packs an
  add-on via `-packAddon`, can direct output with `-packAddonDir`, and publishes
  a version via `-publishAddonVersion`; when omitted, the backend's newest
  version is used with its last digit incremented. [Startup Parameters:
  Workbench packaging and publishing](https://community.bohemia.net/wiki/Arma_Reforger%3AStartup_Parameters?useskin=vector)
  (high confidence). The documentation establishes a *published* version but
  does not establish that an installed `.pak` exposes a canonical version field
  suitable as a cache key.
- A Reforger add-on requires `Arma Reforger` as a dependency, selected
  dependencies load after a Workbench restart, and a dependency GUID can be
  obtained from its `.gproj`. [Resource Manager Options: dependencies](https://community.bohemia.net/wiki/Arma_Reforger%3AResource_Manager%3A_Options)
  (high confidence).
- The Resource Manager browses resources from the base game and loaded mods.
  [Resource Manager](https://community.bohemia.net/wiki/Arma_Reforger%3AResource_Manager)
  (high confidence). This makes the loaded Workbench project the appropriate
  authority for the effective layer set, rather than a blind scan of all local
  Workshop downloads.

### Verified repository/bridge evidence

The checked-in Workbench bridge already compiles against and uses the following
Enfusion APIs; live Workbench validation is still required before making them a
new LSP acquisition contract.

- `GameProject.GetLoadedAddons` supplies loaded GUIDs and
  `GameProject.GetAddonID` resolves them in
  [`server/bridge/RST_WorkbenchProjectContext.c`](../../server/bridge/RST_WorkbenchProjectContext.c)
  and [`server/bridge/RST_WorkbenchListResources.c`](../../server/bridge/RST_WorkbenchListResources.c).
- `ResourceDatabase.SearchResources` discovers resources in the loaded project;
  the bridge derives each resource GUID, resolved add-on ID, path, and extension
  and has an explicit `addonGuid` filter in
  [`RST_WorkbenchListResources.c`](../../server/bridge/RST_WorkbenchListResources.c).
  This is direct local evidence that the effective resource database can be
  partitioned by add-on GUID.
- The current LSP starts the Rust process with one `--game-data-scripts` root
  and one `--index-cache` file in
  [`src/languageClient/languageClient.ts`](../../src/languageClient/languageClient.ts).
  The current Rust cache config likewise has one `scripts_root`, `cache_path`,
  and optional downloaded metadata file in
  [`server/src/index_cache.rs`](../../server/src/index_cache.rs).
- The current cache already protects a good foundation: it validates source
  identity/content before reuse and writes through a uniquely named temporary
  file followed by an atomic replacement. See `source_fingerprint*`,
  `load_or_build_game_data_index*`, and `write_cached_payload` in
  [`index_cache.rs`](../../server/src/index_cache.rs).
- The language-engine contract requires external indexes to be immutable,
  revisioned layers so an in-flight request has stable meaning.
  [`docs/language-engine.md`](../language-engine.md) is therefore a hard
  constraint on any multi-add-on merge.

### PakInspector (secondary, not format authority)

[rvost/PakInspector](https://github.com/rvost/PakInspector) describes itself as
a proof-of-concept viewer/extractor for Reforger `PAC1` PAK files. Its checked
out `formats/pak.ksy` models a `FORM` / `PAC1` container, a file tree, and per
file offset, compressed/original length, and compression type. Its README
explicitly warns that its Kaitai implementation uses RAM comparable to the PAK
size. It supports listing and extracting selected paths, but it does not
provide add-on identity/version semantics. Treat it as a useful compatibility
fixture/reference, not as an LSP runtime dependency or authoritative format
specification.

### Installed RHS Content Pack 01 case study (local primary evidence)

The user-provided installed add-on directory,
`C:\\Users\\Gray\\Documents\\My Games\\ArmaReforger\\addons\\RHS-ContentPack01_1337C0DE5DABBEEF`,
was inspected read-only on 2026-07-30. This is a concrete Workshop-installed
package shape, not a claim that every add-on has every one of these files.

`addon.gproj` supplies the project-local identity facts:

```text
ID            RHS_Content_01
GUID          1337C0DE5DABBEEF
TITLE         RHS: Status Quo - Content Pack 01
Dependencies  58D0FB3206B6F859
```

The directory name repeats the GUID but is not used as evidence for identity.
The `GUID` field agrees with the separate installed metadata files: `meta` has
`meta.id = 1337C0DE5DABBEEF` and `ServerData.json` has the same `id`. The
stable cache key should consequently remain the `.gproj`/Workbench GUID, with
the project ID and directory name kept only as display/locator context.

`meta` selects revision `0` and records version `0.15.5089`, creation time
`2025-04-25T13:01:00.0Z`, and update time `2026-07-22T09:46:03.0Z`.
`ServerData.json` independently reports revision version `0.15.5089`,
`corrupted = false`, and an empty `gameVersion`. That establishes an installed
published-version hint, but **not** a sufficient source-content identity.
Notably, `ServerData.json` reports no dependencies while `addon.gproj` reports
one GUID. This disagreement means neither sidecar metadata nor a local project
file can replace Workbench's loaded graph as the authority for dependencies or
effective order.

The selected `meta` package lists seven payload artifacts in this observed
order: `data.pak`, `data002.pak`, `data001.pak`, `data003.pak`, `addon.gproj`,
`thumbnail.png`, and `resourceDatabase.rdb`. Four are PAKs, not one:

| PAK | Actual and manifest-declared bytes | Local PAK header | Companion manifest |
| --- | ---: | --- | --- |
| `data.pak` | 1,992,553,453 | `FORM` … `PAC1HEAD` | `data.pak_0.15.5089_manifest.json` |
| `data001.pak` | 1,995,096,661 | `FORM` … `PAC1HEAD` | `data001.pak_0.15.5089_manifest.json` |
| `data002.pak` | 1,992,529,274 | `FORM` … `PAC1HEAD` | `data002.pak_0.15.5089_manifest.json` |
| `data003.pak` | 358,932,853 | `FORM` … `PAC1HEAD` | `data003.pak_0.15.5089_manifest.json` |

The four PAK byte counts sum to 6,339,112,241; `meta` declares package
`totalSize = 6,339,694,012`, which is exactly that PAK total plus the other
three listed artifacts. The approximately 2-GB first three archives and a
smaller fourth are evidence that an inspector must enumerate a **set** of PAKs
per add-on. They do not establish a universal split-size rule or a lexical
archive precedence rule. Preserve the package/Workbench ordering as observed;
do not assume `data`, `data001`, … order is the effective resource overlay
order until the live Workbench experiment proves it.

Every listed payload artifact has a same-version companion
`<artifact>_0.15.5089_manifest.json`. Each observed manifest has schema
`version = 1`, exact byte `size`, a full-file `sha512`, and fragment records;
the four PAK manifests contain 1,300, 1,935, 1,867, and 533 fragments
respectively. Thus this installation supplies an unusually cheap strong
artifact signature without reading 6.34 GB of archives: canonicalize the
selected revision plus the package artifact sequence and, for each PAK, record
its name, declared size, full SHA-512, and the SHA-512 of its manifest bytes.
Before trusting that fast path, require that the manifest filename/version,
payload filename, and payload size agree with the selected `meta` entry and
with the local file metadata. If any sidecar is missing, malformed,
inconsistent, or unavailable for another install source, fall back to the
bounded PAK inspection/content-digest path already proposed above. Never use
only the visible version or mtime as a cache correctness key.

The header reads establish that each of the four payloads is a PAC1 container;
they do not enumerate its files or prove which archive contains `.c` sources.
This case study deliberately did not run PakInspector against the 6.34-GB set:
its documented whole-archive memory behavior is inappropriate as the evidence
tool for a performance-sensitive runtime design. The first extractor fixture
experiment must instead list the file tables of all four PAKs with bounded I/O,
then record script count, logical paths, duplicate paths, compression modes,
and the contributing PAK for every `.c` entry.

The following read-only PowerShell probe reproduces the identity, package, and
manifest observations without hashing or extracting the PAK payloads:

```powershell
$addon = 'C:\\Users\\Gray\\Documents\\My Games\\ArmaReforger\\addons\\RHS-ContentPack01_1337C0DE5DABBEEF'
$meta = Get-Content -Raw -LiteralPath "$addon\\meta" | ConvertFrom-Json
$project = Get-Content -Raw -LiteralPath "$addon\\addon.gproj"
$meta.meta.versions[$meta.meta.selectedRev].package.files |
  Select-Object name, size, updatedAt, filePath
Get-ChildItem -LiteralPath $addon -Filter '*_manifest.json' |
  Sort-Object Name | ForEach-Object {
    $manifest = Get-Content -Raw -LiteralPath $_.FullName | ConvertFrom-Json
    [pscustomobject]@{ manifest = $_.Name; schema = $manifest.version
      bytes = $manifest.size; sha512 = $manifest.sha512
      fragments = @($manifest.fragments).Count }
  }
```

## Design consequences

1. **Discover the loaded graph before inspecting archives.** The authoritative
   discovery result should be a deterministic list of `{ guid, addonId }` from
   Workbench, with `ArmaReforger` simply one ordinary required layer. Do not
   scan every add-on physically present on disk; presence is not load state.

2. **Use GUID as stable add-on identity.** Keep the resolved add-on/project ID
   only for display, diagnostics, and a human-readable cache directory name.
   Never use a directory name as the cache identity: official documentation
   names it only as a fallback loader selector.

3. **Version is an optimization hint, content identity is the correctness
   key.** A published Workshop version is not enough to prove exact installed
   bytes or source contents. Each add-on cache manifest should contain:

   ```text
   schema + extractor version + parser/index shape
   addon GUID + display ID
   source artifact identity (resolved PAK set/path metadata or source root)
   deterministic digest of selected `.c` logical paths and uncompressed bytes
   ```

   Reuse only when all required manifest fields match. A cheap artifact
   signature (file name, size, mtime, and ideally PAK header digest) may avoid
   extraction; it must fall through to the content digest whenever uncertain.

4. **Separate inspection from extraction.** A pack inspector should first read
   bounded container/file-table metadata, select only `.c` paths, validate each
   path against traversal/duplicate/size limits, then stream/decompress only
   those files to a staging directory or directly into the Rust indexing input.
   Do not extract an entire add-on merely to index scripts, and do not adopt the
   POC's whole-file RAM behaviour.

5. **One add-on, one durable cache artifact.** Suggested layout under VS Code
   global storage:

   ```text
   addon-indexes/v1/<guid>/<content-digest>/manifest.json
   addon-indexes/v1/<guid>/<content-digest>/symbols.bin
   addon-indexes/v1/<guid>/<content-digest>/sources/   (only if definitions need source text)
   ```

   Publish a completed revision atomically, then atomically replace a small
   per-GUID `current` pointer/manifest. A failed or cancelled rebuild must leave
   the preceding valid revision readable. Garbage collection belongs outside
   startup; retain the current revision plus a bounded recent set.

6. **Load, do not rebuild, at LSP startup.** Build/update indexes in a
   dedicated acquisition phase. At server initialization, resolve the manifest
   set for the discovered add-on GUIDs and publish a single immutable external
   snapshot assembled from their individual indexes. On a cold/missing add-on,
   preserve the prior complete snapshot until a new complete generation is
   ready; report per-add-on availability rather than silently changing query
   scope mid-request.

7. **Define overlay ordering explicitly before implementation.** Add-ons may
   replace base-game resources. The discovery manifest must preserve Workbench's
   effective loaded order and source provenance. The merger cannot use lexical
   GUID ordering or collapse same-named symbols without a verified precedence
   rule. This is the most important outstanding Workbench acceptance experiment.

## Required implementation experiments

Before committing to the extractor API, validate against a live Workbench
project containing `ArmaReforger` plus at least two dependencies:

1. Compare `GameProject.GetLoadedAddons` and `GetAddonID` output with the
   visible dependency/load order, including base game.
2. For every loaded GUID, query `ResourceDatabase.SearchResources` with `.c`
   filtering and prove whether returned paths cover scripts in packed and local
   source add-ons; record identity/path shapes and ordering.
3. Locate the corresponding on-disk PAK(s) from each resource/add-on identity
   without guessing installation directories. If Workbench exposes no stable
   source-file API, make a narrowly scoped, user-configured locator a separate
   design decision rather than inventing a directory heuristic.
4. Compare script bytes emitted by the prospective streaming extractor with
   Workbench-visible script resources for uncompressed, zlib-compressed, and
   unsupported-compression samples. Unsupported compression must make only that
   add-on unavailable with an actionable diagnostic.
5. Build an add-on index twice unchanged (cache hit), update only one add-on,
   and prove that exactly that GUID is extracted/reindexed while all other cache
   artifacts are reused. Measure discovery, inspection, extraction, parse,
   deserialize, merge, peak memory, and total startup time independently.

## Non-goals and risks

- Do not make GitHub source download another normal indexing path; it conflicts
  with effective installed/loaded add-on truth. It may remain a separately
  labelled developer fixture only if needed.
- Do not treat unverified PAC1 reverse engineering as proof of every PAK
  variant or compression mode. Parser limits and compatibility need explicit
  telemetry and fixtures.
- Do not make the TypeScript extension parse PAKs or language source. It may
  own process lifecycle and user-facing progress; Rust should own extractor
  validation, source manifests, cache identity, indexing, and immutable merged
  snapshots.
