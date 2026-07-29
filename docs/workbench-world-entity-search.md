# Workbench World-Entity Relation Search

`workbench_search_world_entities` is the discovery tool for questions about
the live World Editor graph: which authored entities exist, which direct
components they have, and whether a bounded hierarchy relation has a matching
entity. It is read-only. The generated [MCP API Reference](mcp-api.md) remains
the authoritative schema; this guide explains how an AI should use the search
and its follow-up inspection calls.

## When to Use It

Use the search to narrow an open world before asking for detailed facts. It is
especially useful for questions such as:

- Which entities use a known class, prefab resource, or direct component?
- Which authored entities contain a matching child or descendant?
- Which entities sit below a particular parent or ancestor?
- Which candidate entities should be inspected for their actual hierarchy,
  properties, prefab provenance, or overrides?

It does not prove arbitrary graph facts. A result proves only the supplied
filters and one returned `relationMatch`; it is not a complete hierarchy dump,
component-property inspection, or prefab-override report.

## Call Contract

All top-level filters are optional, and supplied filters are combined with
logical AND. An empty request is allowed but normally too broad to be useful.

| Parameter | Required | Meaning |
| --- | --- | --- |
| `query` | No | Bounded text discovery filter (maximum 128 characters). Use only when class, resource, component, layer, or subscene facts are not known. |
| `className` | No | Exact candidate entity class (maximum 128 characters). |
| `resourceQuery` | No | Candidate prefab-resource text (maximum 512 characters). |
| `componentClasses` | No | Up to 32 exact classes that must all be direct components of the candidate entity. |
| `subScene` | No | Exact non-negative subscene identifier. Use it early to constrain a large world. |
| `layerId` | No | Exact non-negative layer identifier. |
| `limit` | No | Returned hits per page, from 1 through 100. Start small when exploring. |
| `cursor` | No | Opaque continuation from the previous response. Reuse every other filter unchanged. |
| `relation` | No | One exact-class/component containment predicate, described below. |

`relation` is optional, but when present it must contain `direction` and
`maxDepth`, plus at least one of `className` or `componentClasses`:

| Relation parameter | Required | Meaning |
| --- | --- | --- |
| `direction` | Yes | `parent`, `ancestor`, `child`, or `descendant`. |
| `className` | One predicate required | Exact class of the related entity. |
| `componentClasses` | One predicate required | Up to 32 exact classes that must all be direct components of the related entity. |
| `maxDepth` | Yes | 1 through 8. `parent` and `child` require exactly 1; use `ancestor` or `descendant` for a transitive search. |

The class and component predicates inside `relation` are also ANDed. For
example, a descendant relation with `className: "GenericEntity"` and
`componentClasses: ["SCR_ArsenalComponent"]` finds candidates that have a
descendant matching both facts.

## Useful Query Patterns

Find a known parent composition. This asks for entities with an immediate
child that is both `GenericEntity` and directly owns the Arsenal component:

```json
{
  "className": "GenericEntity",
  "subScene": 2,
  "limit": 20,
  "relation": {
    "direction": "child",
    "className": "GenericEntity",
    "componentClasses": ["SCR_ArsenalComponent"],
    "maxDepth": 1
  }
}
```

Find entities nested below a known type. Keep the scope narrow with a known
layer or subscene whenever possible:

```json
{
  "subScene": 2,
  "layerId": 7,
  "relation": {
    "direction": "ancestor",
    "className": "SCR_BaseContainerEntity",
    "maxDepth": 4
  }
}
```

Find direct component composition without a hierarchy condition:

```json
{
  "resourceQuery": "Arsenal",
  "componentClasses": ["SCR_ArsenalComponent"],
  "limit": 10
}
```

## Interpreting a Response

Each result contains a stable-for-this-editor-context `entity.entityId`, its
direct `componentClasses`, and `matchedFields`. If a relation was requested,
`relationMatch` contains the first matching related entity found: its stable
ID, exact class, subscene, layer, matching direct component classes, requested
direction, and depth. It is evidence for one match, not a list of every
matching relative.

`truncated: true` means that more matching results are available. Continue by
passing `nextCursor` with exactly the same filters. The summary counts are
exact only when `truncated` is false; while a page is truncated they are counts
observed through that page boundary, not totals for the whole world.

`relationTraversalTruncated: true` has a different meaning: a per-candidate
relation walk reached its fixed 1,024-node bound, or the request reached its
fixed relation-candidate bound. Affected candidates are omitted. A cursor
cannot recover those omitted relation walks. Narrow the query with `subScene`,
`layerId`, candidate class/resource/component filters, or inspect a known
entity directly.

Entity IDs are targets, not durable project identities. Use them only against
the same observed live World Editor context; reacquire them after changing or
reloading the world.

## Follow-Up Calls

Use the smallest call that answers the next question:

| Need | MCP call | Required input | What it adds |
| --- | --- | --- | --- |
| Live hierarchy, root facts, source resource, or component summaries for a returned entity | `workbench_inspect_entity` | `entityId` from a search hit or `relationMatch` | Exact entity inspection without changing selection. |
| Parent chain and direct children of something already selected in World Editor | `workbench_selected_entity_hierarchy` | `selectionIndex` from 0 to 31 | Bounded selected-entity hierarchy. It does not select an entity itself. |
| Prefab ancestry, member facts, direct-child prefab summaries, and override origins | `workbench_inspect_prefab_context` | Exactly one of a live `entityId` or canonical `resourceName`; optional direct-child `memberId` | Distinguishes prefab inheritance/overrides from scene hierarchy. |
| Complete properties for one prefab component | `workbench_inspect_prefab_component` | The context-derived target and component identity | Typed effective property values and direct/inherited/default origins. |

Do not infer prefab overrides from `relationMatch`, `parentClassName`, or a
scene child relationship. Scene hierarchy and prefab ancestry are separate
facts; use `workbench_inspect_prefab_context` for the latter.

## AI Operating Guidance

Start with the most discriminating known fact, then inspect exact returned IDs
before proposing an edit. Prefer `subScene` and `layerId` to a broad text query
when they are known. Request a small `limit`, page deliberately, and report
both truncation flags whenever they affect a conclusion. For a question about
all matching descendants, say that the result is bounded and identify any
relation-traversal omission rather than claiming exhaustive coverage.
