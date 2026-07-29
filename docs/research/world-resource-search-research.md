# World and resource search research

Research date: 2026-07-29.

This note scopes an AI-oriented, read-only search and inspection surface for a
loaded World Editor world and the Resource Manager database. It is evidence,
not a commitment to a particular MCP schema or a substitute for live
Workbench validation.

## Conclusion

Search should have two first-class domains with a small result projection and
separate, bounded inspection:

| Domain | Search facts worth indexing/filtering | Inspect only on demand |
| --- | --- | --- |
| World entity | entity name, root class, entity ID, layer/subscene, prefab/resource identity, direct component-class set, parent/child facts | transform, flags, scripts, all component/property values, prefab definition/inheritance |
| Resource | name/text, logical path/directory, extension/category, GUID, tags, registered/runtime state, addon/source, nested-child facts | metadata, resolved container, prefab component/child templates, links/dependencies |

Component predicates and hierarchy are high-value default entity facets.
Property matching is also useful, but must be an explicit, typed and bounded
inspection/search option (`propertyPath`, exact/contains value); it should not
be eagerly indexed or returned for every entity. World Editor exposes a large,
scope-sensitive property surface, and container properties can nest or be
arrays. A separate property query can report the matched path and a compact
typed preview, then hand the entity/resource to inspection.

## Official Workbench evidence

The [Resource Manager](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager)
Resource Browser combines local and packed game data. Its search accepts
whitespace-tokenized filename/keyword terms, GUID lookup, `#tag`, and `!term`
exclusions; it filters runtime-only, unregistered-only and file-type category,
and sorts by name, size, type or date. This establishes name/text, GUID, tag,
state, type, path/directory and pagination/sorting as useful resource-search
facts. Resource context also exposes names/GUIDs, and its browser spans data
types including configs, textures and prefabs.

The [Find Linked Resources plugin](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Find_Linked_Resources_Plugin)
finds resources referenced by selected resources and can constrain results by
extensions such as `et`, `edds`, and `xob`; it warns that large selections add
processing cost. Resource graph traversal is therefore valuable, but should
be direct-only initially, extension-filterable, paginated, and depth/total
bounded. A reverse-link query is desirable only after its native data source
is separately established.

The [World Editor](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor)
Hierarchy displays world entities grouped by layers, distinguishes entities
from prefabs, and supports parent-with-children selection. Object Properties
shows a root type, script-reference name and component list, then transform,
flags, script slots and other settings. The same guide says changes may target
an instance, definition, modded entity or parent prefab. This supports
searching entity name, class, component classes, layer, ancestry/descendants
and prefab/resource identity, while keeping definition/instance property
scope visible in detailed results rather than collapsing it into one value.

The World Editor's World Resource Browser shows project directory structure;
Prefab Library categorizes available prefabs. The
[Prefab Management Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Prefab_Management_Tool)
also treats the addon as an explicit target filesystem and preserves relative
directory hierarchy when importing XOB selections. Include addon/source and
directory as resource facets, and represent nested children for both prefab
templates and world entities.

[WorldEditorAPI Usage](https://community.bistudio.com/wiki/Arma_Reforger:WorldEditorAPI_Usage)
identifies `.et` files as entity-template prefabs and says prefab operations
primarily use `BaseContainer` references. Inspection should consequently load
and describe prefab/container data separately from a live world instance;
never imply that a prefab query proves an instance is present in the world.

## Extracted API and game-source evidence

The current extracted APIs provide an implementation path without name-based
guessing:

- `WorldEditorAPI.GetEditorEntityCount()` and `GetEditorEntity(index)` enumerate
  editor entity sources (generated `scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c`,
  lines 40--42). `scripts/Core/worldEditor.c`, lines 37--61, supplies
  `EditorEntityIterator`, whose comment and implementation establish that it
  skips non-top-level entries by testing `GetParent()`.
- An `IEntitySource` is a `BaseContainer` and exposes ID, layer, subscene,
  component count and indexed components (generated
  `scripts/Core/generated/Containers/IEntitySource.c`, lines 14--18).
  `BaseContainer` exposes class name, child/parent/ancestor containers and
  source addons (generated `scripts/Core/generated/Containers/BaseContainer.c`,
  lines 18--21 and 70--73). That supports class/component, hierarchy, and
  provenance facets from source data rather than live-object heuristics.
- `Resource.Load(ResourceName)` loads or obtains a cached resource
  (generated `scripts/Core/generated/Resources/Resource.c`, line 31).
  `Resource.GetResource().ToEntitySource()` is used by the World Editor
  `SCR_ProjectileWindageDataGeneratorPlugin` to obtain an entity source from a
  prefab before spawning it (`scripts/WorkbenchGame/WorldEditor/SCR_ProjectileWindageDataGeneratorPlugin.c`,
  lines 44--55). The same source iterates component sources, tests the
  component class with `GetClassName()`, and reads a named property (`lines
  62--76`), proving prefab component/property inspection is useful and
  structurally feasible.
- The plugin's attributes constrain projectiles to `et` and output configs to
  `conf class=SCR_ProjectileWindTable` (`lines 5--9`), a concrete game example
  for extension/type/class-constrained resource selection.
- At runtime, `IEntity.GetName()`, `GetChildren()`, `GetSibling()`, and
  `FindComponent(typename)` exist (generated
  `scripts/Core/generated/Entities/IEntity.c`, lines 171, 389, 524 and its
  `GetSibling` record). They are useful for a live-entity inspection path, but
  World Editor source/container facts should remain the authoritative path for
  searching authored world data.

## Recommended public shape

Start with four read-only operations, all with explicit `limit`, deterministic
sort and continuation cursor/result truncation facts:

1. `search_world_entities(filters, projection)` -- filters for text/name,
   root class (exact or inherited only if inheritance evidence is supplied),
   prefab/resource path, direct component class(es), layer/subscene, and
   ancestor/descendant relationship; projects stable ID, name, class,
   layer/subscene, prefab hint, component names, and parent/child counts.
2. `inspect_world_entity(entity_id, include)` -- returns compact hierarchy,
   component descriptors, transform/flags, and opt-in properties with their
   definition-versus-instance provenance where native data supplies it.
3. `search_resources(filters, scope)` -- filters name/text, path/directory,
   extension/category, GUID, tags, addon/source, registered/runtime state and
   `hasChildren`; scopes base/project/addon resources explicitly.
4. `inspect_resource(resource_id_or_path, include)` -- returns identity and
   metadata, container children/properties, and for `.et`, an entity-template
   class/components/children summary. Loading must not spawn or mutate.

Add `find_resource_links(resources, extension_filter, direction)` only after
proving a native direct-link source and retain strict resource/depth/result
limits. Do not create an arbitrary property-expression language, generic file
search, raw container dump, or unbounded recursive graph traversal.

## Gaps and required validation

- The extracted `ResourceManager` API confirms selected Resource Browser paths
  and metadata/rebuild operations, but this research did not find a generated
  full-database query/enumeration API. The bridge must establish its actual
  authoritative Resource Database query route before promising global resource
  search or reverse dependencies.
- The inspected source proves prefab conversion and component/property reads;
  it does not establish a general, stable mapping from every world entity
  source to its originating prefab path or every property's definition scope.
  Return `unknown` rather than infer either fact, and add live Workbench
  acceptance fixtures covering placed prefab, inline entity, inherited prefab,
  nested child and overridden component property.
- Verify result identity, ordering, pagination, inactive-layer behavior and
  addon provenance in a live Workbench before publishing tools. Large worlds,
  property scans and linked-resource traversal require cancellation and hard
  budgets.
