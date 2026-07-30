//! Versioned Workbench handler sources.
//!
//! These checked-in Enforce Script files are the authoritative managed handler
//! package. The Workbench controller embeds and installs their exact bytes.

pub const BRIDGE_CAPABILITIES_SOURCE: &str = include_str!("../bridge/RST_WorkbenchCapabilities.c");
pub const BRIDGE_STATE_SOURCE: &str = include_str!("../bridge/RST_WorkbenchState.c");
pub const BRIDGE_LIST_EDITORS_SOURCE: &str = include_str!("../bridge/RST_WorkbenchListEditors.c");
pub const BRIDGE_OPEN_EDITOR_SOURCE: &str = include_str!("../bridge/RST_WorkbenchOpenEditor.c");
pub const BRIDGE_OPEN_RESOURCE_SOURCE: &str = include_str!("../bridge/RST_WorkbenchOpenResource.c");
pub const BRIDGE_PLAY_SESSION_SOURCE: &str = include_str!("../bridge/RST_WorkbenchPlaySession.c");
pub const BRIDGE_PROJECT_CONTEXT_SOURCE: &str = include_str!("../bridge/RST_WorkbenchProjectContext.c");
pub const BRIDGE_LOADED_ADDON_GRAPH_SOURCE: &str =
    include_str!("../bridge/RST_WorkbenchLoadedAddonGraph.c");
pub const BRIDGE_INSPECT_RESOURCE_SOURCE: &str = include_str!("../bridge/RST_WorkbenchInspectResource.c");
pub const BRIDGE_WORLD_SELECTION_SOURCE: &str = include_str!("../bridge/RST_WorkbenchWorldSelection.c");
pub const BRIDGE_SELECTED_ENTITY_HIERARCHY_SOURCE: &str = include_str!("../bridge/RST_WorkbenchSelectedEntityHierarchy.c");
pub const BRIDGE_ENTITY_LIST_SOURCE: &str = include_str!("../bridge/RST_WorkbenchListEntities.c");
pub const BRIDGE_ENTITY_SEARCH_SOURCE: &str = include_str!("../bridge/RST_WorkbenchSearchEntities.c");
pub const BRIDGE_LAYER_STATE_SOURCE: &str = include_str!("../bridge/RST_WorkbenchLayerState.c");
pub const BRIDGE_ENTITY_INSPECT_SOURCE: &str = include_str!("../bridge/RST_WorkbenchInspectEntity.c");
pub const BRIDGE_SET_SELECTION_SOURCE: &str = include_str!("../bridge/RST_WorkbenchSetSelection.c");
pub const BRIDGE_ENTITY_RADIUS_QUERY_SOURCE: &str = include_str!("../bridge/RST_WorkbenchFindEntitiesByRadius.c");
pub const BRIDGE_TERRAIN_SAMPLE_SOURCE: &str = include_str!("../bridge/RST_WorkbenchSampleTerrain.c");
pub const BRIDGE_VIEWPORT_CONTEXT_SOURCE: &str = include_str!("../bridge/RST_WorkbenchViewportContext.c");
pub const BRIDGE_TRACE_SOURCE: &str = include_str!("../bridge/RST_WorkbenchTrace.c");
pub const BRIDGE_CLEAR_SELECTION_SOURCE: &str = include_str!("../bridge/RST_WorkbenchClearSelection.c");
pub const BRIDGE_ENTITY_MUTATION_SOURCE: &str = include_str!("../bridge/RST_WorkbenchEntityMutation.c");
pub const BRIDGE_SHAPE_POINTS_SOURCE: &str = include_str!("../bridge/RST_WorkbenchShapePoints.c");
pub const BRIDGE_SHAPE_GEOMETRY_SOURCE: &str = include_str!("../bridge/RST_WorkbenchShapeGeometry.c");
pub const BRIDGE_COMPONENTS_SOURCE: &str = include_str!("../bridge/RST_WorkbenchComponents.c");
pub const BRIDGE_PROPERTIES_SOURCE: &str = include_str!("../bridge/RST_WorkbenchProperties.c");
pub const BRIDGE_PREFAB_SOURCE: &str = include_str!("../bridge/RST_WorkbenchPrefab.c");
pub const BRIDGE_LIST_RESOURCES_SOURCE: &str = include_str!("../bridge/RST_WorkbenchListResources.c");
