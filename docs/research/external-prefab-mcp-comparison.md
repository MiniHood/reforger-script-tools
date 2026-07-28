# External prefab MCP comparison

## Scope and confidence

This is a source review of [`steffenbk/enfusion-mcp-BK` at commit
`3eecd8f`](https://github.com/steffenbk/enfusion-mcp-BK/tree/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739).
It is a comparative example, **not** authority for this project's Workbench
contract or for the current Reforger API. Every proposed capability still needs
the normal evidence sequence: current Workbench behavior, official API, then
verified game data.

## What the example exposes

| Example surface | Contract observed in source | What it actually changes |
| --- | --- | --- |
| [`prefab`](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/src/tools/prefab.ts#L84-L301) | `create` generates a typed `.et` text file from recipes; `inspect` parses local files and merges the ancestor chain. `create` declines to overwrite an existing target. | The Node host creates a directory and writes the text file directly. This is project-file scaffolding, not a live Workbench prefab edit. |
| [`wb_prefabs`](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/src/tools/wb-prefabs.ts#L6-L172) | One action-union tool: `createTemplate`, `save`, `getGuid`, `locate`, and `getAncestor`; the first two are edit-mode gated. | It delegates template creation/save/ancestor lookup to its custom NET API handler; GUID lookup and discovery use separate Workbench calls. |
| [`EMCP_WB_Prefabs`](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/mod/Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_Prefabs.c#L70-L279) | `createTemplate` finds a scene entity by name, resolves a destination, creates missing parent directories, then calls `CreateEntityTemplate`; `save` calls `SaveEntityTemplate`; `getAncestor` returns `GetAncestor().GetResourceName()`. | `createTemplate` opens a World Editor entity action. On failure it creates a temporary entity from the ancestor, saves it, then deletes it. `save` has no surrounding entity action in this source. |
| [`wb_entity_modify`](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/src/tools/wb-entities.ts#L232-L386) | Mutates a named **scene entity**: transform, name, parent, direct component properties, arrays, and object class; other actions read properties and transform. | Its handler wraps individual mutations in `BeginEntityAction`/`EndEntityAction`, including property writes through `SetVariableValue`. [Source](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/mod/Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_ModifyEntity.c#L201-L326) |
| [`wb_entity_duplicate`](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/src/tools/wb-entity-duplicate.ts#L20-L273) | Duplicates a scene entity into a destination `.et`, registers it, optionally places a replacement first, then deletes the original. | This is a migration workflow for locked/base-game instances, rather than mutation of their source prefab. |

The example therefore does **not** offer a single generic "modify prefab" API.
Its live workflow is: mutate a scene instance, then invoke a separate save
operation. Its other creation path is raw-file generation outside Workbench.

## Useful lessons, not adoption requirements

1. Keep prefab *inspection*, *instance editing*, *template creation*, and
   *template persistence* as separately named operations. They have different
   authority, failure modes, and recovery paths. A combined action-union tool
   obscures those differences.
2. A useful AI editing loop is inspect -> propose a small patch -> apply one
   bounded patch in one undo transaction -> re-inspect/verify -> explicitly
   save. The example provides pieces of that loop, but does not make the whole
   sequence atomic.
3. Entity name is not a durable mutation identity. The example's handler scans
   every editor entity and selects the first matching name
   ([source](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/mod/Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_Prefabs.c#L48-L60)).
   A future tool should use an authoritative, stable editor/container identity
   or an explicit uniquely-resolved selector, returning ambiguity rather than
   silently choosing an entity.
4. Raw text creation is valuable for scaffolding, but it must not become an
   alternate normal path for Workbench-owned resource state. Any future write
   tool should report canonical resource identity, affected files, registration
   or reload outcome, and verification result.
5. The temporary-ancestor fallback demonstrates why writes must have a
   deliberate recovery model. It also makes source copying and cleanup part of
   the operation's contract, which is too broad for an initial generic create
   command without live validation.

## Implication for future Reforger MCP research

Prioritize read-only prefab/resource inspection first: canonical resource path
and GUID, ancestor chain, effective components/properties with inheritance
provenance, and bounded child pagination. Then investigate exactly one narrow
write slice: saving an explicitly identified editable template after a
previewed patch, with a single verified Workbench undo transaction and
post-save re-read. Only after that should template creation or duplicate-to-mod
workflows be considered.

The example's direct-file generator and its NET API handlers are evidence of
its own design only. They do not establish that those calls, semantics, or
safety properties work in the target Workbench version.
