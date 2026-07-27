# Workbench water-surface sampling research

Research date: 2026-07-27. This is primary-evidence research for a read-only
water extension to `workbench_sample_terrain`, not an implementation.

## Decision

Use `ChimeraWorldUtils.TryGetWaterSurface` at each already-selected terrain
lattice point for water registered with the running engine. It is the only
direct API located that returns water presence, water surface height, and
engine water kind. Do not infer water from entity proximity, terrain height
versus ocean level, or physics traces: entity search misses the engine ocean,
the ocean comparison misses placed water, and tracing belongs to the separately
planned tracing slice.

Live Workbench acceptance found that the authored `LakeGeneratorEntity` pond at
the supplied fixture position is not reported by this API and is not exposed as
a physics water-surface hit. The public generated API exposes no point query
for that editor-authored lake. Therefore this tool must report that fixture as
`none`, rather than fabricate an ocean or pond result. Supporting those authored
water bodies requires a later explicit design choice: inspect generator geometry
or add a dedicated trace/runtime integration that can observe them.

Water is a suitable optional field on terrain sampling because both are bounded
X/Z fields on the same native lattice and their relationship derives depth.
Entity search should remain a separate 3D sphere query.

## Exact API evidence

| API | Exact signature | Relevance |
| --- | --- | --- |
| `WorldEditorAPI.GetWorld` | `proto external BaseWorld GetWorld()` | Gets the live editor world. |
| `ChimeraWorldUtils.TryGetWaterSurface` | `static proto bool TryGetWaterSurface(BaseWorld world, vector inPoint, out vector outWaterSurfacePoint, out EWaterSurfaceType outType, out vector transformWS[4], out vector obbExtents)` | Direct water point query with kind and surface height. |
| `ChimeraWorldUtils.TryGetWaterSurfaceSimple` | `static proto bool TryGetWaterSurfaceSimple(BaseWorld world, vector inPoint)` | Presence-only; insufficient for the MCP result. |
| `BaseWorld.IsOcean` | `proto external bool IsOcean()` | World availability only, not a point query. |
| `BaseWorld.GetOceanHeight` | `proto external float GetOceanHeight(float worldX, float worldZ)` | Ocean-only height; incomplete for placed water. |
| `BaseWorld.GetOceanBaseHeight` | `proto external float GetOceanBaseHeight()` | Ocean baseline; incomplete for placed water. |
| `WorldEditorAPI.TryGetTerrainSurfaceY` | `proto external bool TryGetTerrainSurfaceY(float x, float z, out float y)` | Existing valid-terrain/ground-height source. |

Sources: extracted `scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c:208,313`,
`scripts/Game/generated/ChimeraWorldUtils.c:13-14`, and
`scripts/Core/generated/World/BaseWorld.c:120-136`.

`EWaterSurfaceType` is exactly `WST_NONE`, `WST_OCEAN`, `WST_POND`, and
`WST_RIVER` (extracted `scripts/Game/generated/EWaterSurfaceType.c:7-13`).
There is no `LAKE` engine value. Preserve the exact wire value `pond`; a client
may describe it as "pond/lake," but must not claim a reliable lake distinction.

## Game-source examples

`SCR_WorldTools.IsObjectUnderwater` calls `TryGetWaterSurface`, receives the
surface point/type/transform/extents, and derives depth from the surface point
(`scripts/Game/Global/SCR_WorldTools.c:217-238`). `SCR_WorldTools.GetWaterSurfaceY`
and the ambient sound equivalent return the Y component and derive an approximate
footprint from `obbExtents[0] * obbExtents[2]`
(`scripts/Game/Global/SCR_WorldTools.c:242-252`; `scripts/Game/Components/AmbientSoundsComponent/SCR_SoundGroup.c:29-39`).

The stock Workbench statistics plugin compares terrain elevation with
`GetOceanBaseHeight()` to calculate land above water
(`scripts/WorkbenchGame/WorldEditor/SCR_WorldEntitiesStatisticsPlugin.c:418-480`).
That proves ocean/terrain are separate facts, but it cannot detect ponds or
rivers and is not appropriate here.

## Recommended MCP contract

Add `includeWater: boolean = false` to `workbench_sample_terrain`. When true,
for each valid terrain cell call:

```text
inPoint = Vector(sampleX, terrainHeight, sampleZ)
TryGetWaterSurface(world, inPoint, surfacePoint, kind, transform, extents)
```

Return an optional parallel `water` object with row-major arrays aligned to
`grid.heights`:

```json
{
  "types": ["none", "ocean", "pond", "river", null],
  "surfaceHeights": [null, 4.2, 15.1, 12.5, null],
  "depthsAboveTerrain": [null, 1.3, 0.6, 2.1, null],
  "summary": {
    "wetSampleCount": 3,
    "oceanSampleCount": 1,
    "pondSampleCount": 1,
    "riverSampleCount": 1,
    "maximumDepthAboveTerrain": 2.1
  }
}
```

`null` type means terrain was absent and water was not queried; `none` means a
valid terrain cell had no water surface. `surfaceHeights` is the returned water
Y, and `depthsAboveTerrain` is derived as `surfaceHeight - terrainHeight` for
wet cells. Keep the existing 4,096-cell cap. Do not add tracing, entity scans,
mutations, material inspection, or water-body resource scans.

Do not add approximate water-body area in the first contract. The source uses
OBB extents for a local area estimate, but neither units nor a robust lake-size
meaning are documented.

## Required live acceptance and uncertainty

The generated data proves the symbols. Workbench acceptance observed:

1. Compile a minimal `WorkbenchGame` NET handler using
   `ChimeraWorldUtils.TryGetWaterSurface`, confirming the Game utility is
   available from the handler module.
2. A known ocean is returned with ocean surface heights and positive depth.
3. The supplied nearby authored pond coordinate X=814.879/Z=2367.746 returns
   no engine water surface. Lowering the point below terrain falsely classifies
   the inland area as ocean and is rejected.

The boolean's precise above/below-surface semantics are inferred from the stock
`IsObjectUnderwater` use. The tool passes terrain Y and does not use an
ocean-level heuristic.

Official terrain guidance also treats ocean as environment setup while water
bodies/rivers/shorelines are separate terrain population concerns:
[New Terrain Setup](https://community.bistudio.com/wiki/Arma_Reforger:New_Terrain_Setup)
and [Terrain Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Terrain_Tutorial).
