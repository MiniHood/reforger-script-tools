# Relation search research

Research date: 2026-07-29.

This note scopes AI-facing relation search for authored World Editor entities
and Reforger resources. It is evidence, not a promise of a complete live
Workbench resource-enumeration route.

## Conclusion

Search must use typed relations, not one generic `parent` or `override`.
Entity containment, component ownership/nesting, prefab inheritance,
scene-instance overrides, and addon resource overrides have different identity
and propagation semantics.

The most useful first queries are:

1. World entities with direct/descendant entities or direct components of a
   supplied class, constrained by parent/ancestor class, layer, or subscene.
2. `.et` entity-prefab templates with member hierarchy and direct component
   predicates, including nested prefab members.
3. Prefab inheritance chains, then direct known inherited children.
4. Component owner, sibling-through-owner, and child-component relations.
5. Addon provenance plus same-GUID override versus unique-GUID inherit/duplicate
   relations.

Property search should be an opt-in typed predicate that returns the matched
path and direct/inherited provenance, not a raw container dump.

## Relation vocabulary

| Relation | Meaning |
| --- | --- |
| `entityChildOf` | Authored or runtime entity containment; direct and bounded-transitive forms. |
| `componentOwnedBy` | Component to its entity owner; distinct from an arbitrary reference. |
| `componentChildOf` | Nested component containment; distinct from the owner entity. |
| `prefabMemberOf` | Template entity membership in an `.et` prefab. |
| `prefabInheritsFrom` | Unique-ID resource derivation. Not entity containment. |
| `sceneInstanceOf` | A world entity materialised from a prefab; return `unknown` when source data does not prove it. |
| `propertyOverrides` | Local instance/prefab value replacing inherited/default data. |
| `resourceOverridesSameGuid` | Addon resource modification with precedence/provenance semantics. |
| `resourceDuplicateOf` | New-GUID copy, independent of later source changes. |

Script `modded` classes and method overrides are useful code-semantic relations,
but should remain separate from the authored prefab/container graph initially.

## Primary evidence

[Prefabs Basics](https://community.bistudio.com/wiki/Arma_Reforger:Prefabs_Basics)
defines `.et` entity prefabs as entities with hierarchy, components, and config
objects, and permits recursive nesting. It distinguishes `.ct` component
prefabs and `.conf` config prefabs. It also states that a prefab can inherit a
prefab, inherited parameters can be overridden, and final scene instances can
override parameters.

That guide defines a prefab instance as an exact mirror of its prefab: it
cannot have a different member count or lose one individual member. In contrast,
adding an entity to a prefab propagates to all its instances and inherited
prefabs. It documents components attached to entities and components that need
child components (for example, `WeaponComponent`). These facts require separate
structural, inheritance, owner, and child-component predicates.

The same source says a bold property in an inherited prefab overrides the parent
value; in a base prefab it overrides a class default; a scene value can be
restored to the prefab value. Results must report scope/provenance rather than
only a resolved value.

[Data Modding Basics](https://community.bistudio.com/wiki/Arma_Reforger:Data_Modding_Basics)
states that an override uses the same GUID and selectively replaces/adds data;
an inherited resource has a unique ID and receives parent data; and a duplicate
has a new GUID, copies source data including inheritance, and does not modify
the source. Thus a query for derived resources must name `override`, `inherit`,
or `duplicate`; they are not interchangeable “children.”

## Extracted API and example evidence

The extracted API provides runtime entity traversal via `IEntity.GetParent`,
`GetRootParent`, `GetChildren`, and `GetSibling`
([`IEntity.c`](../../raw/game-data/scripts/Core/generated/Entities/IEntity.c)).
`GetChildren` yields the first child and `GetSibling` advances that level, so a
bridge must traverse both. `FindComponent`/`FindComponents` provide component
lookup.

For saved authored data, `IEntitySource` extends `BaseContainer` and exposes
editor ID, layer, subscene, component count, and indexed component source
([`IEntitySource.c`](../../raw/game-data/scripts/Core/generated/Containers/IEntitySource.c)).
`BaseContainer` supplies class/name, parent/ancestor, direct child indexing,
direct-variable flags, and source addons
([`BaseContainer.c`](../../raw/game-data/scripts/Core/generated/Containers/BaseContainer.c)).
Use this source-container path as the authority for authored world search.

`ScriptComponent.GetOwner`, `OnAddedToParent`, and `OnChildAdded` establish
component ownership and hierarchy-event direction
([`ScriptComponent.c`](../../raw/game-data/scripts/GameLib/generated/Components/ScriptComponent.c)).
`GenericComponent.GetComponentData` and `GetComponentSource` support targeted
component source inspection. The game-source example
`SCR_ProjectileWindageDataGeneratorPlugin` loads a prefab to an entity source,
iterates component sources, checks class names, and reads a named property;
this proves prefab component/property inspection is structurally feasible.

## Recommended minimal surface

1. `search_world_entities`: name/class, direct component, parent/ancestor,
   child/descendant (explicit `maxDepth`), layer/subscene, and source addon.
2. `inspect_world_entity`: compact ancestry, children, components, and opt-in
   property values with provenance.
3. `inspect_prefab`: `.et` member hierarchy, direct components, inherited
   parent(s), direct overrides, and source addons. It must not claim world
   presence or spawn anything.
4. `find_related`: direct typed relation traversal first; all transitive forms
   require explicit depth/result limits, cursor, deterministic order, and
   truncation facts.

This extends the existing search/inspect split in
[world-resource-search-research.md](world-resource-search-research.md).

## Delivery order and gaps

Start with authored entity-source parent/child, direct components, owner,
class, layer/subscene, and source addon; live-validate an inline hierarchy and
a placed prefab. Next add `.et` inspection, then prefab inheritance and direct
override-property provenance. Add same-GUID override/duplicate search only
after proving the native Resource Manager route across loaded addons.

The extracted APIs prove traversal and container inspection, not a complete
global Resource Manager query, universal prefab-origin mapping, property scope,
or reverse dependency traversal. Preserve `unknown` rather than infer those
facts. Never recursively traverse unbounded entity, prefab, or resource graphs.
