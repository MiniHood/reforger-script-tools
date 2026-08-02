use crate::game_data_inspection::{resolve_symbol_ref, GameDataInspectionError};
use crate::game_data_research::{
    GameDataRelationshipHit, GameDataRelationshipPage, GameDataRelationshipRequest,
    GameDataResearchError,
};
use crate::game_data_search::{
    compact_signature, encode_symbol_ref, kind_name, logical_path, owner_name, qualify,
    GameDataAddonMap, ReadSourceInput, SourceLineStarts, MAX_LIMIT,
};
use crate::index::{GlobalSymbolId, SourceFileId, SymbolIndex};
use crate::index_build::IndexBuildControl;
use crate::model::SymbolKind;
use crate::resolver::callable_override_key;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};

const DEFAULT_LIMIT: usize = 20;
const MAX_TRAVERSAL_RESULTS: usize = 5_000;
const RELATIONSHIP_KINDS: &[&str] = &[
    "direct",
    "directBase",
    "derivedType",
    "moddedExtension",
    "overriddenDeclaration",
    "override",
];

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum SourceAuthority {
    Workspace,
    GameData,
}

#[derive(Debug, Clone)]
pub struct SourceRelationshipSnapshot {
    pub authority: SourceAuthority,
    pub revision: String,
    pub index: Arc<SymbolIndex>,
    pub starts: Arc<BTreeMap<SourceFileId, SourceLineStarts>>,
    pub addon_map: Arc<GameDataAddonMap>,
    pub addon_order: Arc<Vec<String>>,
    pub addon_order_authoritative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRelationshipRequest {
    pub anchor_source: SourceAuthority,
    pub symbol_ref: String,
    pub include_workspace: bool,
    pub addon_guids: Vec<String>,
    pub relationship_kinds: Vec<String>,
    pub result_kinds: Vec<String>,
    pub depth: String,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceRelationshipPage {
    pub relationship_revision: String,
    pub anchor_source: SourceAuthority,
    pub target_symbol_ref: String,
    pub relationship_kinds: Vec<String>,
    pub depth: String,
    pub returned: usize,
    pub total: usize,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub warnings: Vec<String>,
    pub results: Vec<SourceRelationshipHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceRelationshipHit {
    pub source: SourceAuthority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon_label: Option<String>,
    pub symbol_ref: String,
    pub name: String,
    pub kind: String,
    pub qualified_name: String,
    pub signature: String,
    pub source_category: String,
    pub relative_path: String,
    pub relationship_kind: String,
    pub distance: usize,
    pub evidence: String,
    pub declaration_range: crate::game_data_search::SourceLineRange,
    pub selection_range: crate::game_data_search::SourceLineRange,
    pub read_source_input: crate::game_data_search::ReadSourceInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceRelationshipError {
    InvalidRequest(String),
    InvalidAnchor,
    StaleAnchor,
    InvalidCursor,
    StaleCursor,
    SourceUnavailable(SourceAuthority),
    Cancelled,
}

impl fmt::Display for SourceRelationshipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct NodeKey {
    source: SourceAuthority,
    symbol: GlobalSymbolId,
}

#[derive(Debug, Clone)]
struct ProjectionNode {
    key: NodeKey,
    name: Arc<str>,
    kind: SymbolKind,
    module: Arc<str>,
    owner_class: Option<usize>,
    base_type: Option<Arc<str>>,
    callable_key: Option<usize>,
    modded: bool,
    is_override: bool,
    load_order: usize,
}

#[derive(Debug, Default)]
struct RelationshipProjection {
    nodes: Vec<ProjectionNode>,
    node_ids: BTreeMap<NodeKey, usize>,
    class_bases: BTreeMap<usize, usize>,
    class_children: BTreeMap<usize, Vec<usize>>,
    modded_extensions: BTreeMap<usize, Vec<usize>>,
    method_bases: BTreeMap<usize, usize>,
    method_overrides: BTreeMap<usize, Vec<usize>>,
    methods_by_owner: BTreeMap<usize, Vec<usize>>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct CachedProjection {
    revision_key: String,
    projection: Arc<RelationshipProjection>,
}

#[derive(Debug, Clone, Copy)]
enum ProjectionShape {
    Class,
    Method,
    Direct(NodeKey),
}

#[derive(Debug, Default)]
pub struct SourceRelationshipQuery {
    cache: Mutex<Option<CachedProjection>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RelationshipCursor {
    version: u8,
    relationship_revision: String,
    offset: usize,
}

#[derive(Debug, Clone, Copy)]
struct RelatedNode {
    node: usize,
    kind: &'static str,
    distance: usize,
}

#[derive(Debug, Default)]
struct TraversalOutcome {
    related: Vec<RelatedNode>,
    truncated: bool,
    cycle_detected: bool,
}

impl SourceRelationshipQuery {
    pub fn query_restricted_legacy(
        &self,
        control: &IndexBuildControl,
        snapshot: SourceRelationshipSnapshot,
        request: GameDataRelationshipRequest,
    ) -> Result<Option<GameDataRelationshipPage>, GameDataResearchError> {
        let Some(relationship_kinds) = request.relationship_kinds.clone() else {
            return Ok(None);
        };
        if relationship_kinds.is_empty()
            || relationship_kinds.iter().any(|kind| {
                !matches!(
                    kind.as_str(),
                    "directBase" | "derivedType" | "override" | "overriddenDeclaration"
                )
            })
        {
            return Ok(None);
        }
        let authority = snapshot.authority;
        let revision = snapshot.revision.clone();
        let addon_guids = snapshot
            .addon_map
            .values()
            .map(|addon| addon.guid.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let (workspace, game_data) = match authority {
            SourceAuthority::Workspace => (Some(snapshot), None),
            SourceAuthority::GameData => (None, Some(snapshot)),
        };
        let page = self
            .query(
                control,
                workspace,
                game_data,
                SourceRelationshipRequest {
                    anchor_source: authority,
                    symbol_ref: request.symbol_ref.clone(),
                    include_workspace: authority == SourceAuthority::Workspace,
                    addon_guids,
                    relationship_kinds: relationship_kinds.clone(),
                    result_kinds: Vec::new(),
                    depth: "one".to_string(),
                    limit: request.limit,
                    cursor: request.cursor,
                },
            )
            .map_err(legacy_error)?;
        Ok(Some(GameDataRelationshipPage {
            source: "language-engine".to_string(),
            catalogue_revision: revision,
            target_symbol_ref: request.symbol_ref,
            relationship_kinds,
            returned: page.returned,
            total: page.total,
            next_cursor: page.next_cursor,
            results: page
                .results
                .into_iter()
                .map(|hit| GameDataRelationshipHit {
                    relationship_kind: hit.relationship_kind,
                    symbol_ref: Some(hit.symbol_ref),
                    name: Some(hit.name),
                    kind: Some(hit.kind),
                    qualified_name: Some(hit.qualified_name),
                    signature: Some(hit.signature),
                    relative_path: hit.relative_path,
                    range: hit.selection_range,
                    evidence: hit.evidence,
                    read_source_input: hit.read_source_input,
                })
                .collect(),
        }))
    }

    pub fn query(
        &self,
        control: &IndexBuildControl,
        workspace: Option<SourceRelationshipSnapshot>,
        game_data: Option<SourceRelationshipSnapshot>,
        mut request: SourceRelationshipRequest,
    ) -> Result<SourceRelationshipPage, SourceRelationshipError> {
        check(control)?;
        canonicalize_request(&mut request)?;
        let anchor_snapshot = match request.anchor_source {
            SourceAuthority::Workspace => workspace.as_ref(),
            SourceAuthority::GameData => game_data.as_ref(),
        }
        .ok_or(SourceRelationshipError::SourceUnavailable(
            request.anchor_source,
        ))?;
        let anchor = resolve_symbol_ref(
            &anchor_snapshot.index,
            &anchor_snapshot.addon_map,
            control,
            &anchor_snapshot.revision,
            &request.symbol_ref,
        )
        .map_err(map_anchor_error)?;
        let anchor_kind = anchor_snapshot
            .index
            .symbol(anchor)
            .map(|symbol| symbol.kind)
            .ok_or(SourceRelationshipError::InvalidAnchor)?;
        if !matches!(anchor_kind, SymbolKind::Class | SymbolKind::Method)
            && request
                .relationship_kinds
                .iter()
                .any(|kind| kind != "direct")
        {
            return Err(SourceRelationshipError::InvalidRequest(
                "this symbol kind supports only the direct relationship".to_string(),
            ));
        }
        let shape = match anchor_kind {
            SymbolKind::Class => ProjectionShape::Class,
            SymbolKind::Method => ProjectionShape::Method,
            _ => ProjectionShape::Direct(NodeKey {
                source: request.anchor_source,
                symbol: anchor,
            }),
        };
        let revision_key = format!(
            "{};shape={shape:?}",
            projection_revision_key(workspace.as_ref(), game_data.as_ref())
        );
        let projection = self.projection(
            control,
            &revision_key,
            shape,
            workspace.as_ref(),
            game_data.as_ref(),
        )?;
        let anchor_node = projection
            .node_ids
            .get(&NodeKey {
                source: request.anchor_source,
                symbol: anchor,
            })
            .copied()
            .ok_or(SourceRelationshipError::InvalidAnchor)?;
        let relationship_revision = relationship_revision(&revision_key, &request);
        let offset = request
            .cursor
            .as_deref()
            .map(decode_cursor)
            .transpose()?
            .map(|cursor| {
                (cursor.relationship_revision == relationship_revision)
                    .then_some(cursor.offset)
                    .ok_or(SourceRelationshipError::StaleCursor)
            })
            .transpose()?
            .unwrap_or(0);
        let mut traversal = collect_related(
            &projection,
            anchor_node,
            &request.relationship_kinds,
            &request.depth,
            control,
        )?;
        traversal.related.retain(|related| {
            let node = &projection.nodes[related.node];
            let kind_matches = request.result_kinds.is_empty()
                || request
                    .result_kinds
                    .binary_search(&kind_name(node.kind).to_string())
                    .is_ok();
            kind_matches
                && match node.key.source {
                    SourceAuthority::Workspace => request.include_workspace,
                    SourceAuthority::GameData => game_data
                        .as_ref()
                        .and_then(|snapshot| snapshot.addon_map.get(&node.key.symbol.file_id))
                        .is_some_and(|addon| {
                            request
                                .addon_guids
                                .binary_search_by(|guid| {
                                    guid.to_ascii_lowercase()
                                        .cmp(&addon.guid.to_ascii_lowercase())
                                })
                                .is_ok()
                        }),
                }
        });
        traversal.related.sort_by_key(|related| {
            let node = &projection.nodes[related.node];
            (
                relationship_kind_order(related.kind),
                related.distance,
                node.name.clone(),
                node.key,
            )
        });
        traversal
            .related
            .dedup_by_key(|related| (related.node, related.kind));
        if traversal.related.len() > MAX_TRAVERSAL_RESULTS {
            traversal.truncated = true;
            traversal.related.truncate(MAX_TRAVERSAL_RESULTS);
        }
        let total = traversal.related.len();
        if offset > total {
            return Err(SourceRelationshipError::StaleCursor);
        }
        let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let selected = traversal
            .related
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        let mut results = Vec::with_capacity(selected.len());
        for related in selected {
            check(control)?;
            results.push(project_hit(
                &projection.nodes[related.node],
                related.kind,
                related.distance,
                workspace.as_ref(),
                game_data.as_ref(),
            )?);
        }
        let returned = results.len();
        let next_cursor = (offset + returned < total).then(|| {
            encode_cursor(&RelationshipCursor {
                version: 1,
                relationship_revision: relationship_revision.clone(),
                offset: offset + returned,
            })
        });
        let mut warnings = projection.warnings.clone();
        if traversal.truncated {
            warnings.push(format!(
                "Relationship traversal was truncated at {MAX_TRAVERSAL_RESULTS} declarations."
            ));
        }
        if traversal.cycle_detected {
            warnings.push(
                "A semantic relationship cycle was detected and bounded during traversal."
                    .to_string(),
            );
        }
        Ok(SourceRelationshipPage {
            relationship_revision,
            anchor_source: request.anchor_source,
            target_symbol_ref: request.symbol_ref,
            relationship_kinds: request.relationship_kinds,
            depth: request.depth,
            returned,
            total,
            truncated: traversal.truncated,
            next_cursor,
            warnings,
            results,
        })
    }

    fn projection(
        &self,
        control: &IndexBuildControl,
        revision_key: &str,
        shape: ProjectionShape,
        workspace: Option<&SourceRelationshipSnapshot>,
        game_data: Option<&SourceRelationshipSnapshot>,
    ) -> Result<Arc<RelationshipProjection>, SourceRelationshipError> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(cached) = cache
            .as_ref()
            .filter(|cached| cached.revision_key == revision_key)
        {
            return Ok(cached.projection.clone());
        }
        let projection = Arc::new(build_projection(control, shape, workspace, game_data)?);
        *cache = Some(CachedProjection {
            revision_key: revision_key.to_string(),
            projection: projection.clone(),
        });
        Ok(projection)
    }
}

fn legacy_error(error: SourceRelationshipError) -> GameDataResearchError {
    match error {
        SourceRelationshipError::InvalidRequest(message) => {
            GameDataResearchError::InvalidRequest(message)
        }
        SourceRelationshipError::InvalidCursor => GameDataResearchError::InvalidCursor,
        SourceRelationshipError::StaleCursor => GameDataResearchError::StaleCursor,
        SourceRelationshipError::Cancelled => GameDataResearchError::Cancelled,
        SourceRelationshipError::InvalidAnchor => {
            GameDataResearchError::Inspection(GameDataInspectionError::InvalidSymbolRef)
        }
        SourceRelationshipError::StaleAnchor => {
            GameDataResearchError::Inspection(GameDataInspectionError::StaleSymbolRef)
        }
        SourceRelationshipError::SourceUnavailable(_) => GameDataResearchError::InvalidRequest(
            "selected relationship source is unavailable".to_string(),
        ),
    }
}

fn canonicalize_request(
    request: &mut SourceRelationshipRequest,
) -> Result<(), SourceRelationshipError> {
    if request.symbol_ref.is_empty() {
        return Err(SourceRelationshipError::InvalidAnchor);
    }
    if !matches!(request.depth.as_str(), "one" | "all") {
        return Err(SourceRelationshipError::InvalidRequest(
            "depth must be one or all".to_string(),
        ));
    }
    if request.relationship_kinds.is_empty()
        || request
            .relationship_kinds
            .iter()
            .any(|kind| !RELATIONSHIP_KINDS.contains(&kind.as_str()))
    {
        return Err(SourceRelationshipError::InvalidRequest(format!(
            "relationshipKinds must contain one or more of {}",
            RELATIONSHIP_KINDS.join(", ")
        )));
    }
    request
        .relationship_kinds
        .sort_by_key(|kind| relationship_kind_order(kind));
    request.relationship_kinds.dedup();
    request.result_kinds.sort();
    request.result_kinds.dedup();
    request
        .addon_guids
        .sort_by_key(|guid| guid.to_ascii_lowercase());
    request
        .addon_guids
        .dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    Ok(())
}

fn map_anchor_error(error: GameDataInspectionError) -> SourceRelationshipError {
    match error {
        GameDataInspectionError::Cancelled => SourceRelationshipError::Cancelled,
        GameDataInspectionError::InvalidSymbolRef => SourceRelationshipError::InvalidAnchor,
        GameDataInspectionError::StaleSymbolRef => SourceRelationshipError::StaleAnchor,
        _ => SourceRelationshipError::InvalidAnchor,
    }
}

fn projection_revision_key(
    workspace: Option<&SourceRelationshipSnapshot>,
    game_data: Option<&SourceRelationshipSnapshot>,
) -> String {
    format!(
        "workspace={};gameData={}",
        workspace
            .map(|snapshot| snapshot.revision.as_str())
            .unwrap_or("-"),
        game_data
            .map(|snapshot| snapshot.revision.as_str())
            .unwrap_or("-")
    )
}

fn relationship_revision(key: &str, request: &SourceRelationshipRequest) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hasher.update(format!("{:?}", request.anchor_source).as_bytes());
    hasher.update(request.symbol_ref.as_bytes());
    hasher.update(if request.include_workspace {
        b"1"
    } else {
        b"0"
    });
    for guid in &request.addon_guids {
        hasher.update(guid.as_bytes());
        hasher.update([0]);
    }
    for kind in &request.relationship_kinds {
        hasher.update(kind.as_bytes());
        hasher.update([0]);
    }
    for kind in &request.result_kinds {
        hasher.update(kind.as_bytes());
        hasher.update([0]);
    }
    hasher.update(request.depth.as_bytes());
    format!("rel1:{:x}", hasher.finalize())
}

fn build_projection(
    control: &IndexBuildControl,
    shape: ProjectionShape,
    workspace: Option<&SourceRelationshipSnapshot>,
    game_data: Option<&SourceRelationshipSnapshot>,
) -> Result<RelationshipProjection, SourceRelationshipError> {
    let mut projection = RelationshipProjection::default();
    let mut strings = BTreeSet::<Arc<str>>::new();
    let sources = [game_data, workspace]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if game_data.is_some_and(|snapshot| !snapshot.addon_order_authoritative) {
        projection.warnings.push(
            "The offline dependency scope proves matching modded extensions but does not claim exact Workbench overlay order."
                .to_string(),
        );
    }
    for snapshot in &sources {
        let symbol_ids: Box<dyn Iterator<Item = GlobalSymbolId>> = match shape {
            ProjectionShape::Class => Box::new(
                snapshot
                    .index
                    .symbols_for_kind(SymbolKind::Class)
                    .iter()
                    .copied(),
            ),
            ProjectionShape::Method => Box::new(
                snapshot
                    .index
                    .symbols_for_kind(SymbolKind::Class)
                    .iter()
                    .chain(snapshot.index.symbols_for_kind(SymbolKind::Method))
                    .copied(),
            ),
            ProjectionShape::Direct(anchor) if anchor.source == snapshot.authority => {
                Box::new(std::iter::once(anchor.symbol))
            }
            ProjectionShape::Direct(_) => Box::new(std::iter::empty()),
        };
        for symbol_id in symbol_ids {
            check(control)?;
            let Some(symbol) = snapshot.index.symbol(symbol_id) else {
                continue;
            };
            let key = NodeKey {
                source: snapshot.authority,
                symbol: symbol_id,
            };
            let Some(name) = symbol.name.as_deref() else {
                continue;
            };
            let file = snapshot
                .index
                .file(symbol_id.file_id)
                .expect("indexed symbol file");
            let node_id = projection.nodes.len();
            projection.node_ids.insert(key, node_id);
            projection.nodes.push(ProjectionNode {
                key,
                name: intern_value(&mut strings, name),
                kind: symbol.kind,
                module: intern_value(&mut strings, &script_module(logical_path(file).as_str())),
                owner_class: None,
                base_type: symbol
                    .detail
                    .base_type
                    .as_deref()
                    .map(|value| intern_value(&mut strings, value)),
                callable_key: None,
                modded: symbol.modifiers.iter().any(|modifier| modifier == "modded"),
                is_override: symbol
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "override"),
                load_order: source_load_order(snapshot, symbol_id.file_id),
            });
        }
    }
    for node_id in 0..projection.nodes.len() {
        if projection.nodes[node_id].kind != SymbolKind::Method {
            continue;
        }
        let node = &projection.nodes[node_id];
        let snapshot = snapshot_for(node.key.source, workspace, game_data)
            .ok_or(SourceRelationshipError::SourceUnavailable(node.key.source))?;
        projection.nodes[node_id].owner_class = snapshot
            .index
            .symbol(node.key.symbol)
            .and_then(|symbol| symbol.parent)
            .and_then(|parent| {
                projection
                    .node_ids
                    .get(&NodeKey {
                        source: node.key.source,
                        symbol: parent,
                    })
                    .copied()
            });
    }
    for node_id in 0..projection.nodes.len() {
        let node = &projection.nodes[node_id];
        if node.kind != SymbolKind::Method {
            continue;
        }
        if let Some(owner) = node.owner_class {
            projection
                .methods_by_owner
                .entry(owner)
                .or_default()
                .push(node_id);
        }
    }

    let mut class_ids = projection
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(node_id, node)| (node.kind == SymbolKind::Class).then_some(node_id))
        .collect::<Vec<_>>();
    class_ids.sort_by(|left, right| {
        let left = &projection.nodes[*left];
        let right = &projection.nodes[*right];
        (
            left.module.as_ref(),
            left.name.as_ref(),
            left.load_order,
            left.key,
        )
            .cmp(&(
                right.module.as_ref(),
                right.name.as_ref(),
                right.load_order,
                right.key,
            ))
    });
    let mut ambiguous_modded_edges = 0_usize;
    let mut ambiguous_base_edges = 0_usize;
    for node_id in 0..projection.nodes.len() {
        check(control)?;
        let node = &projection.nodes[node_id];
        if node.kind != SymbolKind::Class {
            continue;
        }
        let family = class_family(&projection, &class_ids, &node.module, &node.name);
        if node.modded {
            if let Some(original) = nearest_preceding_class(&projection, family, node_id, false) {
                projection
                    .modded_extensions
                    .entry(original)
                    .or_default()
                    .push(node_id);
            } else {
                ambiguous_modded_edges += 1;
            }
            continue;
        }
        let Some(base_name) = node.base_type.as_ref() else {
            continue;
        };
        let candidates = class_family(&projection, &class_ids, &node.module, base_name);
        if let Some(base) = effective_class(&projection, candidates) {
            projection.class_bases.insert(node_id, base);
            projection
                .class_children
                .entry(base)
                .or_default()
                .push(node_id);
        } else if !candidates.is_empty() {
            ambiguous_base_edges += 1;
        }
    }

    let method_nodes = projection
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(id, node)| {
            (node.kind == SymbolKind::Method && node.is_override).then_some(id)
        })
        .collect::<Vec<_>>();
    let mut callable_keys = BTreeMap::<String, usize>::new();
    let mut unproven_override_edges = 0_usize;
    for method in method_nodes {
        check(control)?;
        ensure_callable_key(
            &mut projection,
            method,
            workspace,
            game_data,
            &mut callable_keys,
        )?;
        if let Some(base) = nearest_overridden_method(
            &mut projection,
            &class_ids,
            method,
            workspace,
            game_data,
            &mut callable_keys,
        )? {
            projection.method_bases.insert(method, base);
            projection
                .method_overrides
                .entry(base)
                .or_default()
                .push(method);
        } else {
            unproven_override_edges += 1;
        }
    }
    if ambiguous_modded_edges > 0 {
        projection.warnings.push(format!(
            "Omitted {ambiguous_modded_edges} ambiguous or unproven modded class relationships."
        ));
    }
    if ambiguous_base_edges > 0 {
        projection.warnings.push(format!(
            "Omitted {ambiguous_base_edges} ambiguous class inheritance relationships."
        ));
    }
    if unproven_override_edges > 0 {
        projection.warnings.push(format!(
            "Omitted {unproven_override_edges} method overrides without one exact proven base behavior."
        ));
    }
    for values in projection.class_children.values_mut() {
        values.sort_unstable();
    }
    for values in projection.modded_extensions.values_mut() {
        values.sort_by_key(|node_id| projection.nodes[*node_id].load_order);
    }
    for values in projection.method_overrides.values_mut() {
        values.sort_by_key(|node_id| projection.nodes[*node_id].load_order);
    }
    Ok(projection)
}

fn snapshot_for<'a>(
    source: SourceAuthority,
    workspace: Option<&'a SourceRelationshipSnapshot>,
    game_data: Option<&'a SourceRelationshipSnapshot>,
) -> Option<&'a SourceRelationshipSnapshot> {
    match source {
        SourceAuthority::Workspace => workspace,
        SourceAuthority::GameData => game_data,
    }
}

fn source_load_order(snapshot: &SourceRelationshipSnapshot, file: SourceFileId) -> usize {
    match snapshot.authority {
        SourceAuthority::Workspace => usize::MAX - 1,
        SourceAuthority::GameData => snapshot
            .addon_map
            .get(&file)
            .and_then(|addon| {
                snapshot
                    .addon_order
                    .iter()
                    .position(|guid| guid.eq_ignore_ascii_case(&addon.guid))
            })
            .unwrap_or(0),
    }
}

fn script_module(path: &str) -> String {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let path = normalized.strip_prefix("scripts/").unwrap_or(&normalized);
    path.split('/').next().unwrap_or("unknown").to_string()
}

fn intern_value(values: &mut BTreeSet<Arc<str>>, value: &str) -> Arc<str> {
    if let Some(existing) = values.get(value) {
        return existing.clone();
    }
    let value = Arc::<str>::from(value);
    values.insert(value.clone());
    value
}

fn effective_class(projection: &RelationshipProjection, candidates: &[usize]) -> Option<usize> {
    let ordinary = candidates
        .iter()
        .copied()
        .filter(|node| !projection.nodes[*node].modded)
        .collect::<Vec<_>>();
    if ordinary.len() == 1 {
        Some(ordinary[0])
    } else {
        None
    }
}

fn nearest_preceding_class(
    projection: &RelationshipProjection,
    family: &[usize],
    node: usize,
    include_modded: bool,
) -> Option<usize> {
    let order = projection.nodes[node].load_order;
    let candidates = family
        .iter()
        .copied()
        .filter(|candidate| {
            *candidate != node
                && projection.nodes[*candidate].load_order < order
                && (include_modded || !projection.nodes[*candidate].modded)
        })
        .collect::<Vec<_>>();
    let nearest_order = candidates
        .iter()
        .map(|candidate| projection.nodes[*candidate].load_order)
        .max()?;
    let nearest = candidates
        .into_iter()
        .filter(|candidate| projection.nodes[*candidate].load_order == nearest_order)
        .collect::<Vec<_>>();
    (nearest.len() == 1).then_some(nearest[0])
}

fn class_family<'a>(
    projection: &RelationshipProjection,
    class_ids: &'a [usize],
    module: &str,
    name: &str,
) -> &'a [usize] {
    let start = class_ids.partition_point(|node_id| {
        let node = &projection.nodes[*node_id];
        node.module.as_ref() < module
            || (node.module.as_ref() == module && node.name.as_ref() < name)
    });
    let end = start
        + class_ids[start..].partition_point(|node_id| {
            let node = &projection.nodes[*node_id];
            node.module.as_ref() == module && node.name.as_ref() == name
        });
    &class_ids[start..end]
}

fn ensure_callable_key(
    projection: &mut RelationshipProjection,
    node: usize,
    workspace: Option<&SourceRelationshipSnapshot>,
    game_data: Option<&SourceRelationshipSnapshot>,
    callable_keys: &mut BTreeMap<String, usize>,
) -> Result<(), SourceRelationshipError> {
    if projection.nodes[node].callable_key.is_some() {
        return Ok(());
    }
    let key = projection.nodes[node].key;
    let snapshot = snapshot_for(key.source, workspace, game_data)
        .ok_or(SourceRelationshipError::SourceUnavailable(key.source))?;
    if let Some(callable_key) = callable_override_key(&snapshot.index, key.symbol) {
        let next_key = callable_keys.len();
        let key_id = *callable_keys.entry(callable_key).or_insert(next_key);
        projection.nodes[node].callable_key = Some(key_id);
    }
    Ok(())
}

fn method_in_class(
    projection: &mut RelationshipProjection,
    class: usize,
    method: usize,
) -> Vec<usize> {
    projection
        .methods_by_owner
        .get(&class)
        .cloned()
        .unwrap_or_default()
        .iter()
        .copied()
        .filter(|candidate| projection.nodes[*candidate].name == projection.nodes[method].name)
        .collect()
}

fn nearest_overridden_method(
    projection: &mut RelationshipProjection,
    class_ids: &[usize],
    method: usize,
    workspace: Option<&SourceRelationshipSnapshot>,
    game_data: Option<&SourceRelationshipSnapshot>,
    callable_keys: &mut BTreeMap<String, usize>,
) -> Result<Option<usize>, SourceRelationshipError> {
    let Some(owner) = projection.nodes[method].owner_class else {
        return Ok(None);
    };
    if projection.nodes[owner].modded {
        let family = class_family(
            projection,
            class_ids,
            &projection.nodes[owner].module,
            &projection.nodes[owner].name,
        );
        let mut previous = nearest_preceding_class(projection, family, owner, true);
        while let Some(class) = previous {
            for candidate in method_in_class(projection, class, method) {
                ensure_callable_key(projection, candidate, workspace, game_data, callable_keys)?;
                if projection.nodes[candidate].callable_key == projection.nodes[method].callable_key
                {
                    return Ok(Some(candidate));
                }
            }
            previous = nearest_preceding_class(projection, family, class, true);
        }
    }
    let mut class = projection.class_bases.get(&owner).copied();
    while let Some(base_class) = class {
        for candidate in method_in_class(projection, base_class, method) {
            ensure_callable_key(projection, candidate, workspace, game_data, callable_keys)?;
            if projection.nodes[candidate].callable_key == projection.nodes[method].callable_key {
                return Ok(Some(candidate));
            }
        }
        class = projection.class_bases.get(&base_class).copied();
    }
    Ok(None)
}

fn collect_related(
    projection: &RelationshipProjection,
    anchor: usize,
    kinds: &[String],
    depth: &str,
    control: &IndexBuildControl,
) -> Result<TraversalOutcome, SourceRelationshipError> {
    let mut outcome = TraversalOutcome::default();
    for kind in kinds {
        check(control)?;
        match kind.as_str() {
            "direct" => outcome.related.push(RelatedNode {
                node: anchor,
                kind: "direct",
                distance: 0,
            }),
            "directBase" => traverse_single(
                projection,
                anchor,
                "directBase",
                depth,
                |projection, node| {
                    projection
                        .class_bases
                        .get(&node)
                        .copied()
                        .into_iter()
                        .collect()
                },
                control,
                &mut outcome,
            )?,
            "derivedType" => traverse_single(
                projection,
                anchor,
                "derivedType",
                depth,
                |projection, node| {
                    projection
                        .class_children
                        .get(&node)
                        .cloned()
                        .unwrap_or_default()
                },
                control,
                &mut outcome,
            )?,
            "moddedExtension" => {
                let family_root = if projection.nodes[anchor].modded {
                    projection
                        .modded_extensions
                        .iter()
                        .find_map(|(root, members)| members.contains(&anchor).then_some(*root))
                        .unwrap_or(anchor)
                } else {
                    anchor
                };
                for node in projection
                    .modded_extensions
                    .get(&family_root)
                    .into_iter()
                    .flatten()
                {
                    outcome.related.push(RelatedNode {
                        node: *node,
                        kind: "moddedExtension",
                        distance: 1,
                    });
                }
            }
            "overriddenDeclaration" => traverse_single(
                projection,
                anchor,
                "overriddenDeclaration",
                depth,
                |projection, node| {
                    projection
                        .method_bases
                        .get(&node)
                        .copied()
                        .into_iter()
                        .collect()
                },
                control,
                &mut outcome,
            )?,
            "override" => traverse_single(
                projection,
                anchor,
                "override",
                depth,
                |projection, node| {
                    projection
                        .method_overrides
                        .get(&node)
                        .cloned()
                        .unwrap_or_default()
                },
                control,
                &mut outcome,
            )?,
            _ => unreachable!("request was canonicalized"),
        }
    }
    Ok(outcome)
}

fn traverse_single(
    projection: &RelationshipProjection,
    anchor: usize,
    kind: &'static str,
    depth: &str,
    neighbors: impl Fn(&RelationshipProjection, usize) -> Vec<usize>,
    control: &IndexBuildControl,
    outcome: &mut TraversalOutcome,
) -> Result<(), SourceRelationshipError> {
    let mut queue = VecDeque::from([(anchor, 0_usize)]);
    let mut visited = BTreeSet::from([anchor]);
    while let Some((node, distance)) = queue.pop_front() {
        check(control)?;
        if outcome.related.len() >= MAX_TRAVERSAL_RESULTS {
            outcome.truncated = true;
            break;
        }
        if distance > 0 {
            outcome.related.push(RelatedNode {
                node,
                kind,
                distance,
            });
        }
        if depth == "one" && distance >= 1 {
            continue;
        }
        for next in neighbors(projection, node) {
            if visited.insert(next) {
                queue.push_back((next, distance + 1));
            } else if next != anchor || distance > 0 {
                outcome.cycle_detected = true;
            }
        }
    }
    Ok(())
}

fn project_hit(
    node: &ProjectionNode,
    relationship_kind: &str,
    distance: usize,
    workspace: Option<&SourceRelationshipSnapshot>,
    game_data: Option<&SourceRelationshipSnapshot>,
) -> Result<SourceRelationshipHit, SourceRelationshipError> {
    let snapshot = snapshot_for(node.key.source, workspace, game_data)
        .ok_or(SourceRelationshipError::SourceUnavailable(node.key.source))?;
    let symbol = snapshot
        .index
        .symbol(node.key.symbol)
        .ok_or(SourceRelationshipError::StaleAnchor)?;
    let file = snapshot
        .index
        .file(node.key.symbol.file_id)
        .ok_or(SourceRelationshipError::StaleAnchor)?;
    let addon = snapshot.addon_map.get(&node.key.symbol.file_id);
    let qualified_name = qualify(
        owner_name(&snapshot.index, symbol).as_deref(),
        node.name.as_ref(),
    );
    let path = logical_path(file);
    let starts = snapshot
        .starts
        .get(&node.key.symbol.file_id)
        .cloned()
        .unwrap_or_default();
    let declaration_range = starts.range(symbol.span.start, symbol.span.end);
    let selection_range = starts.range(symbol.selection_span.start, symbol.selection_span.end);
    Ok(SourceRelationshipHit {
        source: node.key.source,
        addon_guid: addon.map(|addon| addon.guid.clone()),
        addon_label: addon.map(|addon| addon.label.clone()),
        symbol_ref: encode_symbol_ref(
            &snapshot.revision,
            addon.map(|addon| addon.guid.as_str()),
            &path,
            kind_name(symbol.kind),
            &qualified_name,
            symbol.selection_span.start,
        ),
        name: node.name.to_string(),
        kind: kind_name(symbol.kind).to_string(),
        qualified_name: qualified_name.clone(),
        signature: snapshot
            .index
            .callable_signature(node.key.symbol)
            .unwrap_or_else(|| compact_signature(symbol, &qualified_name)),
        source_category: file.metadata.category.as_str().to_string(),
        relative_path: path.clone(),
        relationship_kind: relationship_kind.to_string(),
        distance,
        evidence: relationship_evidence(relationship_kind).to_string(),
        declaration_range,
        selection_range: selection_range.clone(),
        read_source_input: ReadSourceInput {
            catalogue_revision: snapshot.revision.clone(),
            addon_guid: addon.map(|addon| addon.guid.clone()),
            relative_path: path,
            start_line: selection_range.start_line,
        },
    })
}

fn relationship_evidence(kind: &str) -> &'static str {
    match kind {
        "direct" => "exact revision-bound symbol reference",
        "directBase" | "derivedType" => "indexed class base type and exact script module",
        "moddedExtension" => "modded modifier, same class name, and exact script module",
        "overriddenDeclaration" | "override" => {
            "override modifier, exact callable shape, and proven owner relationship"
        }
        _ => "indexed semantic relationship",
    }
}

fn relationship_kind_order(kind: &str) -> usize {
    RELATIONSHIP_KINDS
        .iter()
        .position(|candidate| *candidate == kind)
        .unwrap_or(RELATIONSHIP_KINDS.len())
}

fn check(control: &IndexBuildControl) -> Result<(), SourceRelationshipError> {
    control
        .check()
        .map_err(|_| SourceRelationshipError::Cancelled)
}

fn encode_cursor(cursor: &RelationshipCursor) -> String {
    serde_json::to_vec(cursor)
        .expect("relationship cursor serializes")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_cursor(value: &str) -> Result<RelationshipCursor, SourceRelationshipError> {
    if value.len() > 4096 || value.len() % 2 != 0 {
        return Err(SourceRelationshipError::InvalidCursor);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| SourceRelationshipError::InvalidCursor)?;
    let cursor: RelationshipCursor =
        serde_json::from_slice(&bytes).map_err(|_| SourceRelationshipError::InvalidCursor)?;
    (cursor.version == 1)
        .then_some(cursor)
        .ok_or(SourceRelationshipError::InvalidCursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_data_search::{
        encode_symbol_ref, kind_name, logical_path, owner_name, qualify, GameDataAddonIdentity,
    };
    use crate::model::{
        source_category_for_path, SourceCategory, SourceFileMetadata, SourceKind, SymbolKind,
        SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
    };
    use crate::parser::parse_source;
    use crate::semantic_file::SemanticFile;
    use std::path::PathBuf;

    struct TestFile {
        semantic: SemanticFile,
        metadata: SourceFileMetadata,
        source: String,
    }

    fn test_file(source: &str, kind: SourceKind, path: &str) -> TestFile {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let relative_path = PathBuf::from(path);
        let category = if kind == SourceKind::Workspace {
            SourceCategory::Workspace
        } else {
            source_category_for_path(kind, Some(&relative_path))
        };
        TestFile {
            semantic: SemanticFile::build(source, &parse),
            metadata: SourceFileMetadata {
                kind,
                category,
                absolute_path: None,
                virtual_source: None,
                root_path: None,
                relative_path: Some(relative_path),
                priority: if kind == SourceKind::Workspace {
                    SOURCE_PRIORITY_WORKSPACE
                } else {
                    SOURCE_PRIORITY_GAME_DATA
                },
            },
            source: source.to_string(),
        }
    }

    fn snapshot(
        authority: SourceAuthority,
        revision: &str,
        files: &[TestFile],
    ) -> SourceRelationshipSnapshot {
        let index = Arc::new(SymbolIndex::from_semantic_files(
            files
                .iter()
                .map(|file| (&file.semantic, file.metadata.clone())),
        ));
        let starts = index
            .files()
            .iter()
            .zip(files)
            .map(|(indexed, file)| (indexed.id, SourceLineStarts::from_source(&file.source)))
            .collect();
        let addon_map = if authority == SourceAuthority::GameData {
            index
                .files()
                .iter()
                .map(|file| {
                    (
                        file.id,
                        GameDataAddonIdentity {
                            guid: "game-guid".to_string(),
                            label: "Game".to_string(),
                        },
                    )
                })
                .collect()
        } else {
            BTreeMap::new()
        };
        SourceRelationshipSnapshot {
            authority,
            revision: revision.to_string(),
            index,
            starts: Arc::new(starts),
            addon_map: Arc::new(addon_map),
            addon_order: Arc::new(vec!["game-guid".to_string()]),
            addon_order_authoritative: true,
        }
    }

    fn symbol_ref(
        snapshot: &SourceRelationshipSnapshot,
        kind: SymbolKind,
        name: &str,
        owner: Option<&str>,
    ) -> String {
        let symbol = snapshot
            .index
            .symbol_iter()
            .find(|symbol| {
                symbol.kind == kind
                    && symbol.name.as_deref() == Some(name)
                    && owner_name(&snapshot.index, symbol).as_deref() == owner
            })
            .expect("fixture symbol");
        let file = snapshot.index.file(symbol.id.file_id).unwrap();
        let qualified_name = qualify(owner, name);
        encode_symbol_ref(
            &snapshot.revision,
            snapshot
                .addon_map
                .get(&symbol.id.file_id)
                .map(|addon| addon.guid.as_str()),
            &logical_path(file),
            kind_name(kind),
            &qualified_name,
            symbol.selection_span.start,
        )
    }

    #[test]
    fn relates_game_data_classes_and_methods_to_workspace_without_mixing_overloads() {
        let game = snapshot(
            SourceAuthority::GameData,
            "game-r1",
            &[test_file(
                "class Vehicle { void Move(int speed); }",
                SourceKind::GameData,
                "Game/Vehicles/Vehicle.c",
            )],
        );
        let workspace = snapshot(SourceAuthority::Workspace, "ws-r1", &[
            test_file(
                "class Car : Vehicle { override void Move(int speed) {} override void Move(string mode) {} }",
                SourceKind::Workspace,
                "Game/Vehicles/Car.c",
            ),
            test_file(
                "modded class Vehicle { override void Move(int speed) {} }",
                SourceKind::Workspace,
                "Game/Vehicles/VehicleMod.c",
            ),
        ]);
        let class_anchor = symbol_ref(&game, SymbolKind::Class, "Vehicle", None);
        let query = SourceRelationshipQuery::default();
        let page = query
            .query(
                &IndexBuildControl::default(),
                Some(workspace.clone()),
                Some(game.clone()),
                SourceRelationshipRequest {
                    anchor_source: SourceAuthority::GameData,
                    symbol_ref: class_anchor,
                    include_workspace: true,
                    addon_guids: vec!["game-guid".to_string()],
                    relationship_kinds: vec![
                        "derivedType".to_string(),
                        "moddedExtension".to_string(),
                    ],
                    result_kinds: Vec::new(),
                    depth: "all".to_string(),
                    limit: Some(20),
                    cursor: None,
                },
            )
            .unwrap();
        assert_eq!(
            page.results
                .iter()
                .map(|hit| (hit.relationship_kind.as_str(), hit.qualified_name.as_str()))
                .collect::<Vec<_>>(),
            vec![("derivedType", "Car"), ("moddedExtension", "Vehicle")]
        );
        assert!(page
            .results
            .iter()
            .all(|hit| hit.source == SourceAuthority::Workspace));

        let method_anchor = symbol_ref(&game, SymbolKind::Method, "Move", Some("Vehicle"));
        let page = query
            .query(
                &IndexBuildControl::default(),
                Some(workspace),
                Some(game),
                SourceRelationshipRequest {
                    anchor_source: SourceAuthority::GameData,
                    symbol_ref: method_anchor,
                    include_workspace: true,
                    addon_guids: vec!["game-guid".to_string()],
                    relationship_kinds: vec!["override".to_string()],
                    result_kinds: Vec::new(),
                    depth: "all".to_string(),
                    limit: Some(20),
                    cursor: None,
                },
            )
            .unwrap();
        assert_eq!(page.results.len(), 2);
        assert!(page
            .results
            .iter()
            .all(|hit| hit.signature.contains("int speed")));
    }

    #[test]
    fn resolves_all_levels_before_filtering_hidden_workspace_intermediates() {
        let game = snapshot(
            SourceAuthority::GameData,
            "game-r1",
            &[
                test_file(
                    "class Vehicle {}",
                    SourceKind::GameData,
                    "Game/Vehicles/Vehicle.c",
                ),
                test_file(
                    "class Leaf : Middle {}",
                    SourceKind::GameData,
                    "Game/Vehicles/Leaf.c",
                ),
            ],
        );
        let workspace = snapshot(
            SourceAuthority::Workspace,
            "ws-r1",
            &[test_file(
                "class Middle : Vehicle {}",
                SourceKind::Workspace,
                "Game/Vehicles/Middle.c",
            )],
        );
        let anchor = symbol_ref(&game, SymbolKind::Class, "Vehicle", None);
        let page = SourceRelationshipQuery::default()
            .query(
                &IndexBuildControl::default(),
                Some(workspace),
                Some(game),
                SourceRelationshipRequest {
                    anchor_source: SourceAuthority::GameData,
                    symbol_ref: anchor,
                    include_workspace: false,
                    addon_guids: vec!["game-guid".to_string()],
                    relationship_kinds: vec!["derivedType".to_string()],
                    result_kinds: Vec::new(),
                    depth: "all".to_string(),
                    limit: Some(20),
                    cursor: None,
                },
            )
            .unwrap();

        assert_eq!(page.results.len(), 1);
        assert_eq!(page.results[0].qualified_name, "Leaf");
        assert_eq!(page.results[0].distance, 2);
    }

    #[test]
    fn binds_paging_cursor_to_scope_kinds_and_depth() {
        let game = snapshot(
            SourceAuthority::GameData,
            "game-r1",
            &[test_file(
                "class Vehicle {}",
                SourceKind::GameData,
                "Game/Vehicles/Vehicle.c",
            )],
        );
        let workspace = snapshot(
            SourceAuthority::Workspace,
            "ws-r1",
            &[
                test_file(
                    "class Car : Vehicle {}",
                    SourceKind::Workspace,
                    "Game/Vehicles/Car.c",
                ),
                test_file(
                    "modded class Vehicle {}",
                    SourceKind::Workspace,
                    "Game/Vehicles/VehicleMod.c",
                ),
            ],
        );
        let anchor = symbol_ref(&game, SymbolKind::Class, "Vehicle", None);
        let query = SourceRelationshipQuery::default();
        let first = query
            .query(
                &IndexBuildControl::default(),
                Some(workspace.clone()),
                Some(game.clone()),
                SourceRelationshipRequest {
                    anchor_source: SourceAuthority::GameData,
                    symbol_ref: anchor.clone(),
                    include_workspace: true,
                    addon_guids: vec!["game-guid".to_string()],
                    relationship_kinds: vec![
                        "derivedType".to_string(),
                        "moddedExtension".to_string(),
                    ],
                    result_kinds: Vec::new(),
                    depth: "all".to_string(),
                    limit: Some(1),
                    cursor: None,
                },
            )
            .unwrap();
        let cursor = first.next_cursor.expect("second page cursor");
        let error = query
            .query(
                &IndexBuildControl::default(),
                Some(workspace),
                Some(game),
                SourceRelationshipRequest {
                    anchor_source: SourceAuthority::GameData,
                    symbol_ref: anchor,
                    include_workspace: true,
                    addon_guids: vec!["game-guid".to_string()],
                    relationship_kinds: vec!["derivedType".to_string()],
                    result_kinds: Vec::new(),
                    depth: "all".to_string(),
                    limit: Some(1),
                    cursor: Some(cursor),
                },
            )
            .unwrap_err();

        assert_eq!(error, SourceRelationshipError::StaleCursor);
    }

    #[test]
    fn honours_cancellation_before_projection_or_traversal() {
        let game = snapshot(
            SourceAuthority::GameData,
            "game-r1",
            &[test_file(
                "class Vehicle {}",
                SourceKind::GameData,
                "Game/Vehicles/Vehicle.c",
            )],
        );
        let control = IndexBuildControl::default();
        control.cancel();
        let error = SourceRelationshipQuery::default()
            .query(
                &control,
                None,
                Some(game.clone()),
                SourceRelationshipRequest {
                    anchor_source: SourceAuthority::GameData,
                    symbol_ref: symbol_ref(&game, SymbolKind::Class, "Vehicle", None),
                    include_workspace: false,
                    addon_guids: vec!["game-guid".to_string()],
                    relationship_kinds: vec!["direct".to_string()],
                    result_kinds: Vec::new(),
                    depth: "one".to_string(),
                    limit: Some(20),
                    cursor: None,
                },
            )
            .unwrap_err();

        assert_eq!(error, SourceRelationshipError::Cancelled);
    }

    #[test]
    fn unsupported_symbol_kinds_still_support_exact_direct_results() {
        let workspace = snapshot(
            SourceAuthority::Workspace,
            "ws-r1",
            &[test_file(
                "class Vehicle { int Speed; }",
                SourceKind::Workspace,
                "Game/Vehicles/Vehicle.c",
            )],
        );
        let page = SourceRelationshipQuery::default()
            .query(
                &IndexBuildControl::default(),
                Some(workspace.clone()),
                None,
                SourceRelationshipRequest {
                    anchor_source: SourceAuthority::Workspace,
                    symbol_ref: symbol_ref(&workspace, SymbolKind::Field, "Speed", Some("Vehicle")),
                    include_workspace: true,
                    addon_guids: Vec::new(),
                    relationship_kinds: vec!["direct".to_string()],
                    result_kinds: Vec::new(),
                    depth: "one".to_string(),
                    limit: Some(20),
                    cursor: None,
                },
            )
            .unwrap();

        assert_eq!(page.total, 1);
        assert_eq!(page.results[0].kind, "field");
        assert_eq!(page.results[0].qualified_name, "Vehicle.Speed");
    }
}
