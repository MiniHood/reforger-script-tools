use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct WorkbenchGatewayOptions {
    pub host: String,
    pub port: u16,
    pub status_deadline: Duration,
    pub validation_deadline: Duration,
}

impl Default for WorkbenchGatewayOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 5775,
            status_deadline: Duration::from_millis(1_500),
            validation_deadline: Duration::from_secs(120),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchStatus {
    pub is_running: bool,
    pub scripts_compiled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchDiagnosticLocation {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_abs: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCompilerDiagnostic {
    pub severity: WorkbenchDiagnosticSeverity,
    pub message: String,
    pub location: WorkbenchDiagnosticLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkbenchDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchValidation {
    pub profile: String,
    pub success: bool,
    pub diagnostics: Vec<WorkbenchCompilerDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchValidationPage {
    pub profile: String,
    pub success: bool,
    pub total_diagnostics: usize,
    pub diagnostics: Vec<WorkbenchCompilerDiagnostic>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchFailureCode {
    ConsentRequired,
    Unavailable,
    Timeout,
    Protocol,
    WorkbenchError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchFailure {
    pub code: WorkbenchFailureCode,
    pub log_reference: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkbenchGateway {
    options: WorkbenchGatewayOptions,
    request_lock: Arc<Mutex<()>>,
}

pub const WORKBENCH_BRIDGE_VERSION: &str = "1.51.0";
pub const WORKBENCH_BRIDGE_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchInstallAuthorization {
    ExistingConsent,
    UserApprovedFirstInstall,
}

#[derive(Debug, Clone)]
pub struct WorkbenchControllerOptions {
    pub gateway: WorkbenchGatewayOptions,
    pub user_directory: Option<PathBuf>,
    pub game_directory: Option<PathBuf>,
    pub tools_directory: Option<PathBuf>,
    pub executable: Option<PathBuf>,
}

impl Default for WorkbenchControllerOptions {
    fn default() -> Self {
        Self {
            gateway: WorkbenchGatewayOptions::default(),
            user_directory: None,
            game_directory: None,
            tools_directory: None,
            executable: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPathStatus {
    pub path: Option<PathBuf>,
    pub exists: bool,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManagedBridgeStatus {
    pub installed: bool,
    pub installation_available: bool,
    pub installed_version: Option<String>,
    pub active_version: Option<String>,
    pub protocol_version: Option<u32>,
    pub compatible: bool,
    pub activation_required: bool,
    pub capabilities: Vec<String>,
    pub capabilities_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchOverview {
    pub game: WorkbenchPathStatus,
    pub tools: WorkbenchPathStatus,
    pub executable: WorkbenchPathStatus,
    pub profile: WorkbenchPathStatus,
    pub bridge_directory: PathBuf,
    pub process_ids: Vec<u32>,
    pub native: Option<WorkbenchStatus>,
    pub native_failure: Option<String>,
    pub bridge: ManagedBridgeStatus,
    pub support_log: WorkbenchPathStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchBridgeInstallResult {
    pub installed_version: String,
    pub active_version: Option<String>,
    pub protocol_version: Option<u32>,
    pub activated: bool,
    pub managed_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchLiveState {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub mode: String,
    pub world_editor_active: bool,
    pub world_editor_module_present: bool,
    pub world_editor_api_available: bool,
    pub play_session: WorkbenchPlaySession,
    pub loaded_addons: Vec<String>,
    pub loaded_addons_truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_sub_scene: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_entity_layer_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_subscene_layer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkbenchPlaySession {
    Unavailable,
    Unknown,
    LikelyRunning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchScriptActivationResult {
    pub process_id: u32,
    pub workbench_was_minimized: bool,
    pub world_saved_before_reload: bool,
    pub world_save_status: String,
    pub reload_verified: bool,
    pub log_path: PathBuf,
    pub verification_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchSaveAllResult {
    pub process_id: u32,
    pub workbench_was_minimized: bool,
    pub save_all_accepted: bool,
    pub world_save_accepted: bool,
    pub world_save_status: String,
    pub action_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchSaveWorldResult {
    pub process_id: u32,
    pub workbench_was_minimized: bool,
    pub world_save_accepted: bool,
    pub world_save_status: String,
    pub action_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEditor {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEditorList {
    pub editors: Vec<WorkbenchEditor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchOpenEditorResult {
    pub editor_id: String,
    pub opened: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchOpenResourceResult {
    pub resource_path: String,
    pub opened: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPlaySessionResult {
    pub accepted: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchProjectContext {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub loaded_addons: Vec<String>,
    pub loaded_addons_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchResourceInspection {
    pub found: bool,
    pub status: String,
    pub resource_name: Option<String>,
    pub class_name: Option<String>,
    #[serde(default)]
    pub source_addons: Vec<String>,
    #[serde(default)]
    pub source_addons_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchResourceListPage {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub project_revision: String,
    pub limit: usize,
    pub resources: Vec<String>,
    #[serde(skip_serializing, skip_deserializing)]
    #[schemars(skip)]
    pub(crate) resource_details: Vec<String>,
    pub truncated: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchResourceSearchHit {
    pub resource_name: String,
    pub addon_guid: String,
    pub addon_id: Option<String>,
    pub logical_path: String,
    pub name: String,
    pub extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchResourceSearchPage {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub project_revision: String,
    pub limit: usize,
    pub results: Vec<WorkbenchResourceSearchHit>,
    pub truncated: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntityPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchShapePoints {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_class: Option<String>,
    pub closed: bool,
    pub points: Vec<WorkbenchEntityPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchShapePointEdit {
    Set,
    Insert,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchShapePointSpace {
    Local,
    World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchShapeTransformOperation {
    Translate,
    RotateXz,
    Scale,
    Mirror,
    Reverse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchShapePointConversion {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_class: Option<String>,
    pub from_space: String,
    pub to_space: String,
    pub points: Vec<WorkbenchEntityPosition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPolylineResample {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_class: Option<String>,
    pub closed: bool,
    pub points: Vec<WorkbenchEntityPosition>,
    pub spacing_meters: f32,
    pub original_point_count: usize,
    pub result_point_count: usize,
    pub path_length: f32,
    pub skipped_zero_length_segments: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchSelectedEntity {
    pub entity_id: String,
    pub class_name: String,
    pub sub_scene: i32,
    pub layer_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_scene_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<WorkbenchEntityPosition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchWorldSelectionSummary {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub editor_available: bool,
    pub status: String,
    pub selected_count: u32,
    pub selected_entities: Vec<WorkbenchSelectedEntity>,
    pub selected_entities_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchSelectedEntityHierarchy {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub editor_available: bool,
    pub status: String,
    pub selection_index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
    pub ancestors: Vec<WorkbenchSelectedEntity>,
    pub ancestors_truncated: bool,
    pub children: Vec<WorkbenchSelectedEntity>,
    pub children_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntityListPage {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub world_revision: String,
    pub limit: usize,
    pub entities: Vec<WorkbenchSelectedEntity>,
    pub truncated: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntitySearchHit {
    pub entity: WorkbenchSelectedEntity,
    pub component_classes: Vec<String>,
    /// The requested direct component classes that this entity satisfied.
    pub matched_component_classes: Vec<String>,
    pub matched_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_class_name: Option<String>,
    pub child_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_match: Option<WorkbenchEntityRelationMatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum WorkbenchEntityRelationDirection {
    Parent,
    Ancestor,
    Child,
    Descendant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchEntityRelationFilter {
    pub direction: WorkbenchEntityRelationDirection,
    pub class_name: Option<String>,
    pub component_classes: Vec<String>,
    pub max_depth: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntityRelationMatch {
    pub direction: WorkbenchEntityRelationDirection,
    pub depth: i32,
    pub entity_id: String,
    pub class_name: String,
    pub sub_scene: i32,
    pub layer_id: i32,
    pub matched_component_classes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntitySearchSummary {
    /// Matches observed through the current page boundary. Exact only when `truncated` is false.
    pub total_matches: u32,
    /// Matches with an authored entity name. The remainder are anonymous authored entities.
    pub named_matches: u32,
    pub anonymous_matches: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntitySearchPage {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub world_revision: String,
    pub status: String,
    pub limit: usize,
    pub summary: WorkbenchEntitySearchSummary,
    pub results: Vec<WorkbenchEntitySearchHit>,
    pub truncated: bool,
    /// A relation traversal hit its fixed per-candidate visit limit; affected candidates are
    /// omitted rather than searched without a bound.
    pub relation_traversal_truncated: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchLayerState {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    pub sub_scene: i32,
    pub layer_id: i32,
    pub layer_path: String,
    pub visible: bool,
    pub explicitly_locked: bool,
    pub locked_in_hierarchy: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntityInspection {
    #[serde(skip_serializing)]
    pub bridge_version: String,
    #[serde(skip_serializing)]
    pub protocol_version: u32,
    #[serde(skip_serializing)]
    pub editor_available: bool,
    #[serde(skip_serializing_if = "is_available_status")]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_name: Option<String>,
    pub resource_reference_kind: String,
    pub contributor_addons: Vec<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub contributor_addons_truncated: bool,
    pub ancestors: Vec<WorkbenchSelectedEntity>,
    #[serde(skip_serializing_if = "is_false")]
    pub ancestors_truncated: bool,
    pub children: Vec<WorkbenchSelectedEntity>,
    #[serde(skip_serializing_if = "is_false")]
    pub children_truncated: bool,
    pub components: Vec<WorkbenchComponent>,
    #[serde(skip_serializing_if = "is_false")]
    pub component_properties_truncated: bool,
    pub properties: Vec<WorkbenchPrefabProperty>,
    #[serde(skip_serializing_if = "is_false")]
    pub properties_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntitySelectionResult {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntityRadiusQuery {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    pub center: WorkbenchEntityPosition,
    pub radius_meters: f32,
    pub query_scope: String,
    pub require_object: bool,
    pub exclude_proxies: bool,
    pub entities: Vec<WorkbenchSelectedEntity>,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct WorkbenchEntityRadiusQueryOptions {
    pub center: WorkbenchEntityPosition,
    pub radius_meters: f32,
    pub query_scope: String,
    pub require_object: bool,
    pub exclude_proxies: bool,
    pub class_name: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct WorkbenchTerrainSampleOptions {
    pub center_x: f32,
    pub center_z: f32,
    pub half_extent_meters: f32,
    pub spacing_meters: Option<f32>,
    pub include_water: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTerrainCoordinate {
    pub x: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTerrainBounds {
    pub min: WorkbenchEntityPosition,
    pub max: WorkbenchEntityPosition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTerrainMetadata {
    pub bounds: WorkbenchTerrainBounds,
    pub heightmap_resolution_x: u32,
    pub heightmap_resolution_z: u32,
    pub native_spacing_meters: f32,
    pub tile_count_x: u32,
    pub tile_count_z: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTerrainGrid {
    pub origin: WorkbenchTerrainCoordinate,
    pub requested_half_extent_meters: f32,
    pub requested_spacing_meters: Option<f32>,
    pub effective_spacing_meters: f32,
    pub spacing_clamped: bool,
    pub width: u32,
    pub height: u32,
    pub heights: Vec<Option<f32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkbenchTerrainWaterType {
    None,
    Ocean,
    Pond,
    River,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTerrainWaterGrid {
    /// `null` marks an absent terrain cell; `none` marks dry valid terrain.
    pub types: Vec<Option<WorkbenchTerrainWaterType>>,
    pub surface_heights: Vec<Option<f32>>,
    pub depths_above_terrain: Vec<Option<f32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTerrainWaterSummary {
    pub wet_sample_count: u32,
    pub ocean_sample_count: u32,
    pub pond_sample_count: u32,
    pub river_sample_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_depth_above_terrain: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTerrainSummary {
    pub valid_sample_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_height: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_height: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean_height: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevation_range: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steepest_adjacent_slope_degrees: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steepest_adjacent_slope_position: Option<WorkbenchTerrainCoordinate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTerrainSample {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain: Option<WorkbenchTerrainMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid: Option<WorkbenchTerrainGrid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<WorkbenchTerrainSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub water: Option<WorkbenchTerrainWaterGrid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub water_summary: Option<WorkbenchTerrainWaterSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchViewportContext {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mouse_x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mouse_y: Option<i32>,
    pub mouse_inside: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_position: Option<WorkbenchEntityPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_direction: Option<WorkbenchEntityPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mouse_world_position: Option<WorkbenchEntityPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ray_start: Option<WorkbenchEntityPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ray_end: Option<WorkbenchEntityPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ray_direction: Option<WorkbenchEntityPosition>,
}

#[derive(Debug, Clone)]
pub struct WorkbenchViewportContextOptions {
    pub include_ray: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkbenchTraceShape {
    Line,
    Sphere,
    Box,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkbenchTraceHitKind {
    Entity,
    Terrain,
    Ocean,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTraceOptions {
    pub start: WorkbenchEntityPosition,
    pub end: WorkbenchEntityPosition,
    pub shape: WorkbenchTraceShape,
    pub radius: Option<f32>,
    pub box_mins: Option<WorkbenchEntityPosition>,
    pub box_maxs: Option<WorkbenchEntityPosition>,
    pub entities: bool,
    pub terrain: bool,
    pub ocean: bool,
    pub target_layers: Option<i32>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchTraceResult {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    pub hit: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fraction: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<WorkbenchEntityPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal: Option<WorkbenchEntityPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<WorkbenchTraceHitKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collider_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
}

const MAX_WORKBENCH_TRACE_LENGTH_METERS: f32 = 10_000.0;
const MAX_WORKBENCH_TRACE_DIMENSION_METERS: f32 = 1_000.0;
const MAX_WORKBENCH_TARGET_LAYER_MASK: i32 = 0x7fff_ffff;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntityMutationResult {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_layer_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_saved: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inspection: Option<WorkbenchPrefabContext>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPrefabResourceMutationResult {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    pub resource_name: String,
    pub persistence_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_class: Option<String>,
    pub template_saved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inspection: Option<WorkbenchPrefabContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_inspection: Option<WorkbenchPrefabComponentInspection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchComponent {
    pub component_id: String,
    pub class_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_override_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchComponentResult {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
    pub components: Vec<WorkbenchComponent>,
    pub properties: Vec<WorkbenchDirectProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchDirectProperty {
    pub name: String,
    pub data_type: String,
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directly_overridden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_origin: Option<WorkbenchPrefabPropertyOrigin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_descriptor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPropertyList {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    pub properties: Vec<WorkbenchDirectProperty>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum WorkbenchPrefabPropertyOrigin {
    Direct,
    Inherited,
    Default,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPrefabProperty {
    pub path: String,
    pub data_type: String,
    pub value: Value,
    pub directly_overridden: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_origin: Option<WorkbenchPrefabPropertyOrigin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_descriptor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPrefabComponent {
    pub component_id: String,
    pub class_name: String,
    pub properties: Vec<WorkbenchPrefabProperty>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPrefabComponentInspection {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    pub resource_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<WorkbenchPrefabComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPrefabMember {
    /// Stable only within this one inspected prefab context, not a live entity identity.
    pub member_id: String,
    pub class_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchPrefabContext {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<WorkbenchSelectedEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_reference_kind: Option<String>,
    pub contributor_addons: Vec<String>,
    pub ancestor_resources: Vec<String>,
    pub ancestor_resources_truncated: bool,
    pub prefab_edit_mode: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
    pub components: Vec<WorkbenchComponent>,
    /// Direct stored prefab members only; these are not live scene entity identities.
    pub children: Vec<WorkbenchPrefabMember>,
    pub children_truncated: bool,
    pub properties: Vec<WorkbenchPrefabProperty>,
    pub properties_truncated: bool,
    pub child_count: u32,
}

#[derive(Debug, Clone)]
struct PropertyWriteDescriptor {
    entity_id: String,
    component_id: Option<String>,
    property_name: String,
    data_type: String,
    observed_value: String,
    issued: Instant,
}

#[derive(Debug, Clone)]
pub struct WorkbenchCreateEntityOptions {
    pub target: String,
    pub target_is_resource: bool,
    pub sub_scene: i32,
    pub position: WorkbenchEntityPosition,
    pub angles: WorkbenchEntityPosition,
    pub layer_id: i32,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchLogRead {
    pub source: String,
    pub path: Option<PathBuf>,
    pub lines: Vec<String>,
    pub markers: Vec<WorkbenchLogMarker>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchLogMarker {
    pub kind: String,
    pub line_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchProcessResult {
    pub process_id: Option<u32>,
    pub already_running: bool,
    pub net_api_connected: bool,
    pub exited: bool,
    pub user_interaction_required: bool,
}

#[derive(Debug, Clone)]
pub struct WorkbenchController {
    options: WorkbenchControllerOptions,
    gateway: WorkbenchGateway,
    observed_processes: Arc<Mutex<HashSet<ProcessIdentity>>>,
    validation_snapshot: Arc<Mutex<Option<(String, WorkbenchValidation)>>>,
    maintenance_lock: Arc<Mutex<()>>,
    delete_confirmations: Arc<Mutex<HashMap<String, (String, Instant)>>>,
    property_write_descriptors: Arc<Mutex<HashMap<String, PropertyWriteDescriptor>>>,
}

impl WorkbenchController {
    pub fn new(options: WorkbenchControllerOptions) -> Self {
        let gateway = WorkbenchGateway::new(options.gateway.clone());
        Self {
            options,
            gateway,
            observed_processes: Arc::new(Mutex::new(HashSet::new())),
            validation_snapshot: Arc::new(Mutex::new(None)),
            maintenance_lock: Arc::new(Mutex::new(())),
            delete_confirmations: Arc::new(Mutex::new(HashMap::new())),
            property_write_descriptors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn correlate_failure(
        &self,
        operation: &str,
        failure: WorkbenchFailure,
    ) -> WorkbenchFailure {
        if failure.log_reference.is_some() {
            return failure;
        }
        let code = failure_code(failure.code);
        self.correlate_failure_details(
            operation,
            code,
            failure,
            json!({
                "failureCode": code,
            }),
        )
    }

    pub fn overview(&self) -> WorkbenchOverview {
        let started = Instant::now();
        let paths = self.paths();
        let native_result = self.gateway.status();
        let native_failure = native_result
            .as_ref()
            .err()
            .map(|failure| failure_code(failure.code).to_string());
        let native = native_result.ok();
        let mut bridge = self.bridge_disk_status(&paths.bridge_directory);
        if native.is_some() {
            if !bridge.installed
                && self
                    .migrate_legacy_bridge(&paths.legacy_bridge_directory, &paths.bridge_directory)
                    .unwrap_or(false)
            {
                bridge = self.bridge_disk_status(&paths.bridge_directory);
            }
            if bridge.installed {
                bridge = self.maintain_existing_bridge(&paths.bridge_directory);
            } else {
                bridge.installation_available = paths.profile.is_dir();
            }
        }
        let processes = workbench_processes();
        self.observe_processes(&processes);
        let mut overview = WorkbenchOverview {
            game: path_status(paths.game, &paths.game_source),
            tools: path_status(paths.tools, &paths.tools_source),
            executable: path_status(paths.executable, &paths.executable_source),
            profile: path_status(Some(paths.profile), "windows-user"),
            bridge_directory: paths.bridge_directory,
            process_ids: processes.iter().map(|process| process.id).collect(),
            native,
            native_failure,
            bridge,
            support_log: path_status(Some(self.integration_log_path()), "local-app-data"),
        };
        self.log_event_timed(
            "status",
            if overview.native.is_some() {
                "connected"
            } else {
                "unavailable"
            },
            started,
            json!({
                "processCount": overview.process_ids.len(),
                "gameFound": overview.game.exists,
                "toolsFound": overview.tools.exists,
                "executableFound": overview.executable.exists,
                "profileFound": overview.profile.exists,
                "bridgeInstalled": overview.bridge.installed,
                "bridgeVersion": overview.bridge.installed_version.clone(),
                "protocolVersion": overview.bridge.protocol_version,
            }),
        );
        overview.support_log.exists = overview
            .support_log
            .path
            .as_ref()
            .is_some_and(|path| path.is_file());
        overview
    }

    pub fn native_status(&self) -> Result<WorkbenchStatus, WorkbenchFailure> {
        let started = Instant::now();
        let result = self.gateway.status();
        self.log_event_timed(
            "native-status",
            match &result {
                Ok(_) => "connected",
                Err(failure) => failure_code(failure.code),
            },
            started,
            json!({
                "isRunning": result.as_ref().map(|status| status.is_running).ok(),
                "scriptsCompiled": result.as_ref().map(|status| status.scripts_compiled).ok(),
            }),
        );
        result
    }

    pub fn install_bridge(
        &self,
        authorization: WorkbenchInstallAuthorization,
    ) -> Result<WorkbenchBridgeInstallResult, WorkbenchFailure> {
        let _maintenance = self
            .maintenance_lock
            .lock()
            .map_err(|_| failure(WorkbenchFailureCode::Unavailable))?;
        let started = Instant::now();
        if let Err(failure) = self.gateway.status() {
            return Err(self.correlate_failure_details(
                "install",
                "native-unavailable",
                failure,
                json!({"nativeConnected": false}),
            ));
        }
        let paths = self.paths();
        if !paths.profile.is_dir() {
            return Err(self.correlate_failure_details(
                "install",
                "profile-missing",
                failure(WorkbenchFailureCode::Unavailable),
                json!({
                    "profileFound": false,
                    "managedDirectoryCreated": false,
                }),
            ));
        }
        let mut existing_manifest = fs::read(
            paths
                .bridge_directory
                .join("reforger-script-tools.manifest.json"),
        )
        .ok()
        .and_then(|bytes| serde_json::from_slice::<BridgeManifest>(&bytes).ok());
        if existing_manifest.is_none()
            && self
                .migrate_legacy_bridge(&paths.legacy_bridge_directory, &paths.bridge_directory)
                .unwrap_or(false)
        {
            existing_manifest = fs::read(
                paths
                    .bridge_directory
                    .join("reforger-script-tools.manifest.json"),
            )
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BridgeManifest>(&bytes).ok());
        }
        if existing_manifest.is_none()
            && authorization != WorkbenchInstallAuthorization::UserApprovedFirstInstall
        {
            return Err(self.correlate_failure_details(
                "install",
                "consent-required",
                failure(WorkbenchFailureCode::ConsentRequired),
                json!({
                    "managedDirectoryCreated": false,
                    "manifestFound": false,
                }),
            ));
        }
        if let Some(manifest) = existing_manifest.as_ref().filter(|manifest| {
            version_order(&manifest.bridge_version, WORKBENCH_BRIDGE_VERSION).is_gt()
        }) {
            let active = self.active_bridge_status(&paths.bridge_directory, true);
            let result = WorkbenchBridgeInstallResult {
                installed_version: manifest.bridge_version.clone(),
                active_version: active.active_version,
                protocol_version: active.protocol_version,
                activated: active.compatible && !active.activation_required,
                managed_files: manifest.files.len(),
            };
            self.log_event_timed(
                "install",
                "newer-preserved",
                started,
                json!({
                    "bridgeVersion": result.installed_version.clone(),
                    "activeVersion": result.active_version.clone(),
                    "protocolVersion": result.protocol_version,
                    "managedFileCount": result.managed_files,
                    "managedFiles": manifest
                        .files
                        .iter()
                        .map(|file| file.name.as_str())
                        .collect::<Vec<_>>(),
                    "activated": result.activated,
                }),
            );
            return Ok(result);
        }
        if let Err(error) = self.write_managed_files(&paths.bridge_directory) {
            return Err(self.correlate_failure_details(
                "install",
                "write-failed",
                failure(WorkbenchFailureCode::Unavailable),
                json!({
                    "errorKind": format!("{:?}", error.kind()),
                    "managedFileCount": bridge_payload().len(),
                    "managedFiles": bridge_payload()
                        .iter()
                        .map(|(name, _)| *name)
                        .collect::<Vec<_>>(),
                }),
            ));
        }
        let validation = self.gateway.validate_scripts();
        let result = WorkbenchBridgeInstallResult {
            installed_version: WORKBENCH_BRIDGE_VERSION.to_string(),
            // Profile NetApiHandler discovery happens when Workbench reloads its
            // script runtime. Do not call the newly written handler here: until
            // that reload, it cannot exist and the probe only adds a misleading
            // NET API error to the Workbench log.
            active_version: None,
            protocol_version: None,
            activated: false,
            managed_files: bridge_payload().len(),
        };
        self.log_event_timed(
            "install",
            "installed",
            started,
            json!({
                "bridgeVersion": result.installed_version.clone(),
                "activeVersion": result.active_version.clone(),
                "protocolVersion": result.protocol_version,
                "managedFileCount": result.managed_files,
                "managedFiles": bridge_payload()
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>(),
                "activated": result.activated,
                "validationSuccess": validation.as_ref().ok().map(|value| value.success),
            }),
        );
        Ok(result)
    }

    pub fn validate_scripts(&self) -> Result<WorkbenchValidation, WorkbenchFailure> {
        let paths = self.paths();
        if self.bridge_disk_status(&paths.bridge_directory).installed
            && self.gateway.status().is_ok()
        {
            let _ = self.maintain_existing_bridge(&paths.bridge_directory);
        }
        self.native_validate_scripts()
    }

    pub fn native_validate_scripts(&self) -> Result<WorkbenchValidation, WorkbenchFailure> {
        let started = Instant::now();
        let result = self.gateway.validate_scripts();
        self.log_event_timed(
            "validate",
            match &result {
                Ok(validation) if validation.success => "success",
                Ok(_) => "compiler-findings",
                Err(_) => "failed",
            },
            started,
            json!({
                "diagnosticCount": result.as_ref().map(|value| value.diagnostics.len()).ok(),
                "success": result.as_ref().map(|value| value.success).ok(),
            }),
        );
        result
    }

    pub fn validate_scripts_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<WorkbenchValidationPage, WorkbenchFailure> {
        let limit = limit.clamp(1, 200);
        let (token, validation, offset) = if let Some(cursor) = cursor {
            let (token, offset) = parse_validation_cursor(cursor)
                .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
            let snapshot = self
                .validation_snapshot
                .lock()
                .ok()
                .and_then(|snapshot| snapshot.clone())
                .filter(|(snapshot_token, _)| snapshot_token == &token)
                .ok_or_else(|| failure(WorkbenchFailureCode::Unavailable))?;
            (snapshot.0, snapshot.1, offset)
        } else {
            let validation = self.validate_scripts()?;
            let token = sha256(
                &serde_json::to_vec(&validation)
                    .map_err(|_| failure(WorkbenchFailureCode::Protocol))?,
            );
            if let Ok(mut snapshot) = self.validation_snapshot.lock() {
                *snapshot = Some((token.clone(), validation.clone()));
            }
            (token, validation, 0)
        };
        if offset > validation.diagnostics.len() {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let end = (offset + limit).min(validation.diagnostics.len());
        let next_cursor =
            (end < validation.diagnostics.len()).then(|| format!("wv1:{token}:{end}"));
        Ok(WorkbenchValidationPage {
            profile: validation.profile,
            success: validation.success,
            total_diagnostics: validation.diagnostics.len(),
            diagnostics: validation.diagnostics[offset..end].to_vec(),
            next_cursor,
        })
    }

    pub fn state(&self) -> Result<WorkbenchLiveState, WorkbenchFailure> {
        let started = Instant::now();
        let paths = self.paths();
        if self.gateway.status().is_ok()
            && self.bridge_disk_status(&paths.bridge_directory).installed
        {
            let _ = self.maintain_existing_bridge(&paths.bridge_directory);
        }
        let value = self
            .gateway
            .request(
                json!({"APIFunc": "RST_WorkbenchState"}),
                self.options.gateway.status_deadline,
            )
            .map_err(|failure| {
                self.correlate_failure_details(
                    "state",
                    failure_code(failure.code),
                    failure,
                    json!({"handler": "RST_WorkbenchState"}),
                )
            })?;
        let raw: RawBridgeState = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "state",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchState"}),
            )
        })?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(self.correlate_failure_details(
                "state",
                "incompatible-handler",
                failure(WorkbenchFailureCode::Protocol),
                json!({
                    "handler": "RST_WorkbenchState",
                    "expectedProtocolVersion": WORKBENCH_BRIDGE_PROTOCOL_VERSION,
                    "activeProtocolVersion": raw.protocol_version,
                    "activeBridgeVersion": raw.bridge_version,
                }),
            ));
        }
        let (loaded_addons, loaded_addons_truncated) =
            split_bounded_list(&raw.loaded_addons, 256, 256);
        let world_editor_module_present = workbench_bool(&raw.world_editor_module_present);
        let world_editor_api_available = workbench_bool(&raw.world_editor_api_available)
            || workbench_bool(&raw.world_editor_active)
            || raw.mode == "world-editor";
        let state = WorkbenchLiveState {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            world_editor_active: world_editor_api_available,
            world_editor_module_present,
            world_editor_api_available,
            play_session: play_session(
                &raw.play_session,
                world_editor_module_present,
                world_editor_api_available,
            )
            .ok_or_else(|| {
                self.correlate_failure_details(
                    "state",
                    "workbench_protocol_error",
                    failure(WorkbenchFailureCode::Protocol),
                    json!({"handler": "RST_WorkbenchState"}),
                )
            })?,
            mode: raw.mode,
            loaded_addons,
            loaded_addons_truncated,
            current_sub_scene: raw.current_sub_scene,
            current_entity_layer_id: raw.current_entity_layer_id,
            active_subscene_layer: raw.active_subscene_layer.filter(|layer| !layer.is_empty()),
        };
        self.log_event_timed(
            "state",
            "success",
            started,
            json!({
                "activeBridgeVersion": state.bridge_version.clone(),
                "protocolVersion": state.protocol_version,
                "mode": state.mode.clone(),
                "worldEditorActive": state.world_editor_active,
                "worldEditorModulePresent": state.world_editor_module_present,
                "worldEditorApiAvailable": state.world_editor_api_available,
                "playSession": state.play_session,
                "loadedAddonCount": state.loaded_addons.len(),
                "loadedAddonsTruncated": state.loaded_addons_truncated,
            }),
        );
        Ok(state)
    }

    pub fn list_editors(&self) -> Result<WorkbenchEditorList, WorkbenchFailure> {
        let started = Instant::now();
        let value = self
            .gateway
            .request(
                json!({"APIFunc": "RST_WorkbenchListEditors"}),
                self.options.gateway.status_deadline,
            )
            .map_err(|failure| {
                self.correlate_failure_details(
                    "list_editors",
                    failure_code(failure.code),
                    failure,
                    json!({"handler": "RST_WorkbenchListEditors"}),
                )
            })?;
        let raw: RawWorkbenchEditorList = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "list_editors",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchListEditors"}),
            )
        })?;
        let editors = raw
            .editors
            .split(';')
            .filter_map(|entry| {
                let (id, display_name) = entry.split_once('|')?;
                (!id.is_empty() && !display_name.is_empty()).then(|| WorkbenchEditor {
                    id: id.to_string(),
                    display_name: display_name.to_string(),
                })
            })
            .collect::<Vec<_>>();
        if editors.is_empty() {
            return Err(self.correlate_failure_details(
                "list_editors",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchListEditors"}),
            ));
        }
        let result = WorkbenchEditorList { editors };
        self.log_event_timed(
            "list-editors",
            "success",
            started,
            json!({"editorCount": result.editors.len()}),
        );
        Ok(result)
    }

    pub fn open_editor(
        &self,
        editor_id: &str,
    ) -> Result<WorkbenchOpenEditorResult, WorkbenchFailure> {
        if editor_id.trim().is_empty() {
            return Err(self.correlate_failure_details(
                "open_editor",
                "editor-id-required",
                failure(WorkbenchFailureCode::Protocol),
                json!({}),
            ));
        }
        let started = Instant::now();
        let value = self
            .gateway
            .request(
                json!({"APIFunc": "RST_WorkbenchOpenEditor", "editorId": editor_id}),
                self.options.gateway.status_deadline,
            )
            .map_err(|failure| {
                self.correlate_failure_details(
                    "open_editor",
                    failure_code(failure.code),
                    failure,
                    json!({"handler": "RST_WorkbenchOpenEditor"}),
                )
            })?;
        let raw: RawWorkbenchOpenEditor = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "open_editor",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchOpenEditor"}),
            )
        })?;
        let result = WorkbenchOpenEditorResult {
            editor_id: raw.editor_id,
            opened: workbench_bool(&raw.opened),
            status: raw.status,
        };
        self.log_event_timed(
            "open-editor",
            &result.status,
            started,
            json!({"editorId": editor_id, "opened": result.opened}),
        );
        Ok(result)
    }

    pub fn open_resource(
        &self,
        resource_path: &str,
    ) -> Result<WorkbenchOpenResourceResult, WorkbenchFailure> {
        const OPEN_RESOURCE_DEADLINE: Duration = Duration::from_secs(15);

        if resource_path.trim().is_empty() {
            return Err(self.correlate_failure_details(
                "open_resource",
                "resource-path-required",
                failure(WorkbenchFailureCode::Protocol),
                json!({}),
            ));
        }
        let started = Instant::now();
        let value = self
            .gateway
            .request(
                json!({"APIFunc": "RST_WorkbenchOpenResource", "resourcePath": resource_path}),
                OPEN_RESOURCE_DEADLINE,
            )
            .map_err(|failure| {
                self.correlate_failure_details(
                    "open_resource",
                    failure_code(failure.code),
                    failure,
                    json!({"handler": "RST_WorkbenchOpenResource"}),
                )
            })?;
        let raw: RawWorkbenchOpenResource = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "open_resource",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchOpenResource"}),
            )
        })?;
        let result = WorkbenchOpenResourceResult {
            resource_path: raw.resource_path,
            opened: workbench_bool(&raw.opened),
            status: raw.status,
        };
        self.log_event_timed(
            "open-resource",
            &result.status,
            started,
            json!({"resourcePath": resource_path, "opened": result.opened}),
        );
        Ok(result)
    }

    pub fn project_context(&self) -> Result<WorkbenchProjectContext, WorkbenchFailure> {
        let started = Instant::now();
        let value = self
            .gateway
            .request(
                json!({"APIFunc": "RST_WorkbenchProjectContext"}),
                self.options.gateway.status_deadline,
            )
            .map_err(|failure| {
                self.correlate_failure_details(
                    "project_context",
                    failure_code(failure.code),
                    failure,
                    json!({"handler": "RST_WorkbenchProjectContext"}),
                )
            })?;
        let raw: RawBridgeProjectContext = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "project_context",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchProjectContext"}),
            )
        })?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(self.correlate_failure_details(
                "project_context", "incompatible-handler", failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchProjectContext", "activeBridgeVersion": raw.bridge_version, "activeProtocolVersion": raw.protocol_version}),
            ));
        }
        let (loaded_addons, loaded_addons_truncated) =
            split_bounded_list(&raw.loaded_addons, 256, 256);
        let result = WorkbenchProjectContext {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            loaded_addons,
            loaded_addons_truncated,
        };
        self.log_event_timed("project-context", "success", started, json!({"loadedAddonCount": result.loaded_addons.len(), "loadedAddonsTruncated": result.loaded_addons_truncated}));
        Ok(result)
    }

    pub fn inspect_resource(
        &self,
        resource_name: &str,
    ) -> Result<WorkbenchResourceInspection, WorkbenchFailure> {
        let started = Instant::now();
        if !canonical_resource_name(resource_name) {
            return Err(self.correlate_failure_details(
                "inspect_resource",
                "invalid-resource-name",
                failure(WorkbenchFailureCode::Protocol),
                json!({}),
            ));
        }
        let value = self
            .gateway
            .request(
                json!({"APIFunc": "RST_WorkbenchInspectResource", "resourceName": resource_name}),
                self.options.gateway.status_deadline,
            )
            .map_err(|failure| {
                self.correlate_failure_details(
                    "inspect_resource",
                    failure_code(failure.code),
                    failure,
                    json!({"handler": "RST_WorkbenchInspectResource"}),
                )
            })?;
        let raw: RawBridgeResourceInspection = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "inspect_resource",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchInspectResource"}),
            )
        })?;
        let (source_addons, source_addons_truncated) =
            split_bounded_list(&raw.source_addons, 64, 4 * 1024);
        let result = WorkbenchResourceInspection {
            found: raw.found,
            status: raw.status,
            resource_name: raw.resource_name,
            class_name: raw.class_name,
            source_addons,
            source_addons_truncated: raw.source_addons_truncated || source_addons_truncated,
        };
        self.log_event_timed(
            "inspect-resource",
            &result.status,
            started,
            json!({"found": result.found}),
        );
        Ok(result)
    }

    pub fn list_resources(
        &self,
        kinds: &[&str],
        query: Option<&str>,
        root_path: Option<&str>,
        addon_guid: Option<&str>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<WorkbenchResourceListPage, WorkbenchFailure> {
        let limit = limit.clamp(1, 200);
        let query = query.unwrap_or("").trim();
        let root_path = root_path.unwrap_or("").trim();
        let addon_guid = addon_guid.unwrap_or("").trim();
        if !valid_resource_root_path(root_path) {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        if !addon_guid.is_empty()
            && (addon_guid.len() != 16 || !addon_guid.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let kinds = kinds.join(";");
        let signature = sha256(format!("{kinds}\n{query}\n{root_path}\n{addon_guid}").as_bytes());
        let offset = if let Some(cursor) = cursor {
            let (cursor_signature, offset) = parse_resource_list_cursor(cursor)
                .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
            if cursor_signature != signature {
                return Err(failure(WorkbenchFailureCode::Protocol));
            }
            offset
        } else {
            0
        };
        let started = Instant::now();
        let value = self.gateway.request(
            json!({"APIFunc": "RST_WorkbenchListResources", "extensions": kinds, "query": query, "rootPath": root_path, "addonGuid": addon_guid, "offset": offset, "limit": limit}),
            self.options.gateway.status_deadline,
        ).map_err(|failure| self.correlate_failure_details(
            "list_resources", failure_code(failure.code), failure, json!({"handler": "RST_WorkbenchListResources"}),
        ))?;
        let raw: RawBridgeResourceList = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "list_resources",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchListResources"}),
            )
        })?;
        let resources = split_bounded_list(&raw.resources, limit, 256 * 1024).0;
        let resource_details = split_bounded_list(&raw.resource_details, limit, 256 * 1024).0;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
            || resources.len() > limit
            || (!resource_details.is_empty() && resource_details.len() != resources.len())
        {
            return Err(self.correlate_failure_details(
                "list_resources",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchListResources"}),
            ));
        }
        let project_revision = sha256(raw.loaded_addons.as_bytes());
        let has_more = workbench_bool(&raw.has_more);
        let next_cursor =
            has_more.then(|| format!("wrl1:{signature}:{}", offset + resources.len()));
        let result = WorkbenchResourceListPage {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            project_revision,
            limit,
            resources,
            resource_details,
            truncated: has_more,
            next_cursor,
        };
        self.log_event_timed(
            "list-resources",
            "success",
            started,
            json!({"returned": result.resources.len(), "hasMore": result.next_cursor.is_some()}),
        );
        Ok(result)
    }

    pub fn search_resources(
        &self,
        kinds: &[&str],
        query: Option<&str>,
        root_path: Option<&str>,
        addon_guid: Option<&str>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<WorkbenchResourceSearchPage, WorkbenchFailure> {
        let page = self.list_resources(kinds, query, root_path, addon_guid, cursor, limit)?;
        if page.resource_details.len() != page.resources.len() {
            return Err(self.correlate_failure_details(
                "search_resources",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchListResources"}),
            ));
        }
        let results = page
            .resource_details
            .iter()
            .map(|resource_name| parse_resource_search_hit(resource_name))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WorkbenchResourceSearchPage {
            bridge_version: page.bridge_version,
            protocol_version: page.protocol_version,
            project_revision: page.project_revision,
            limit: page.limit,
            results,
            truncated: page.truncated,
            next_cursor: page.next_cursor,
        })
    }

    pub fn world_selection_summary(
        &self,
    ) -> Result<WorkbenchWorldSelectionSummary, WorkbenchFailure> {
        let started = Instant::now();
        let value = self
            .gateway
            .request(
                json!({"APIFunc": "RST_WorkbenchWorldSelection"}),
                self.options.gateway.status_deadline,
            )
            .map_err(|failure| {
                self.correlate_failure_details(
                    "world_selection_summary",
                    failure_code(failure.code),
                    failure,
                    json!({"handler": "RST_WorkbenchWorldSelection"}),
                )
            })?;
        let raw: RawBridgeWorldSelection = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "world_selection_summary",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchWorldSelection"}),
            )
        })?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(self.correlate_failure_details(
                "world_selection_summary",
                "incompatible-handler",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchWorldSelection", "activeBridgeVersion": raw.bridge_version, "activeProtocolVersion": raw.protocol_version}),
            ));
        }
        let selected_entities =
            parse_world_selection_records(&raw.selected_entities).map_err(|_| {
                self.correlate_failure_details(
                    "world_selection_summary",
                    "workbench_protocol_error",
                    failure(WorkbenchFailureCode::Protocol),
                    json!({"handler": "RST_WorkbenchWorldSelection"}),
                )
            })?;
        if selected_entities.len() > 32 || selected_entities.len() > raw.selected_count as usize {
            return Err(self.correlate_failure_details(
                "world_selection_summary",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchWorldSelection"}),
            ));
        }
        let result = WorkbenchWorldSelectionSummary {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            editor_available: workbench_bool(&raw.editor_available),
            status: raw.status,
            selected_count: raw.selected_count,
            selected_entities_truncated: workbench_bool(&raw.selected_entities_truncated)
                || selected_entities.len() < raw.selected_count as usize,
            selected_entities,
        };
        self.log_event_timed(
            "world-selection-summary",
            &result.status,
            started,
            json!({"editorAvailable": result.editor_available, "selectedCount": result.selected_count, "selectedEntitiesReturned": result.selected_entities.len(), "selectedEntitiesTruncated": result.selected_entities_truncated}),
        );
        Ok(result)
    }

    pub fn set_play_session(
        &self,
        start: bool,
        debug_mode: bool,
        full_screen: bool,
    ) -> Result<WorkbenchPlaySessionResult, WorkbenchFailure> {
        let started = Instant::now();
        let operation = if start {
            "start_play_session"
        } else {
            "stop_play_session"
        };
        let value = self.gateway.request(
            json!({"APIFunc": "RST_WorkbenchPlaySession", "start": start, "debugMode": debug_mode, "fullScreen": full_screen}),
            self.options.gateway.status_deadline,
        ).map_err(|failure| self.correlate_failure_details(
            operation, failure_code(failure.code), failure, json!({"handler": "RST_WorkbenchPlaySession"}),
        ))?;
        let result: WorkbenchPlaySessionResult = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                operation,
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchPlaySession"}),
            )
        })?;
        self.log_event_timed(
            operation,
            &result.status,
            started,
            json!({"accepted": result.accepted}),
        );
        Ok(result)
    }

    pub fn selected_entity_hierarchy(
        &self,
        selection_index: u32,
    ) -> Result<WorkbenchSelectedEntityHierarchy, WorkbenchFailure> {
        if selection_index > 31 {
            return Err(self.correlate_failure_details(
                "selected_entity_hierarchy",
                "selection-index-out-of-range",
                failure(WorkbenchFailureCode::Protocol),
                json!({"selectionIndex": selection_index}),
            ));
        }
        let started = Instant::now();
        let value = self.gateway.request(
            json!({"APIFunc": "RST_WorkbenchSelectedEntityHierarchy", "selectionIndex": selection_index}),
            self.options.gateway.status_deadline,
        ).map_err(|failure| self.correlate_failure_details(
            "selected_entity_hierarchy", failure_code(failure.code), failure,
            json!({"handler": "RST_WorkbenchSelectedEntityHierarchy"}),
        ))?;
        let raw: RawBridgeSelectedEntityHierarchy =
            serde_json::from_value(value).map_err(|_| {
                self.correlate_failure_details(
                    "selected_entity_hierarchy",
                    "workbench_protocol_error",
                    failure(WorkbenchFailureCode::Protocol),
                    json!({"handler": "RST_WorkbenchSelectedEntityHierarchy"}),
                )
            })?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(self.correlate_failure_details(
                "selected_entity_hierarchy", "incompatible-handler", failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchSelectedEntityHierarchy", "activeBridgeVersion": raw.bridge_version, "activeProtocolVersion": raw.protocol_version}),
            ));
        }
        let parse_error = || {
            self.correlate_failure_details(
                "selected_entity_hierarchy",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchSelectedEntityHierarchy"}),
            )
        };
        let entity =
            parse_optional_world_selection_record(&raw.entity).map_err(|_| parse_error())?;
        let ancestors = parse_world_selection_records(&raw.ancestors).map_err(|_| parse_error())?;
        let children = parse_world_selection_records(&raw.children).map_err(|_| parse_error())?;
        if ancestors.len() > 32 || children.len() > 64 {
            return Err(self.correlate_failure_details(
                "selected_entity_hierarchy",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchSelectedEntityHierarchy"}),
            ));
        }
        let result = WorkbenchSelectedEntityHierarchy {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            editor_available: workbench_bool(&raw.editor_available),
            status: raw.status,
            selection_index,
            entity,
            ancestors,
            ancestors_truncated: workbench_bool(&raw.ancestors_truncated),
            children,
            children_truncated: workbench_bool(&raw.children_truncated),
        };
        self.log_event_timed(
            "selected-entity-hierarchy", &result.status, started,
            json!({"selectionIndex": selection_index, "ancestorCount": result.ancestors.len(), "ancestorsTruncated": result.ancestors_truncated, "childCount": result.children.len(), "childrenTruncated": result.children_truncated}),
        );
        Ok(result)
    }

    pub fn list_entities(
        &self,
        query: Option<&str>,
        class_name: Option<&str>,
        sub_scene: Option<i32>,
        layer_id: Option<i32>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<WorkbenchEntityListPage, WorkbenchFailure> {
        let signature = sha256(
            format!(
                "{}\n{}\n{}\n{}",
                query.unwrap_or_default(),
                class_name.unwrap_or_default(),
                sub_scene.map_or(String::new(), |value| value.to_string()),
                layer_id.map_or(String::new(), |value| value.to_string())
            )
            .as_bytes(),
        );
        let offset = match cursor {
            Some(cursor) => {
                let (found, offset) = parse_entity_list_cursor(cursor)
                    .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
                (found == signature)
                    .then_some(offset)
                    .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?
            }
            None => 0,
        };
        let value = self.gateway.request(json!({"APIFunc":"RST_WorkbenchListEntities","query":query.unwrap_or_default(),"className":class_name.unwrap_or_default(),"subScene":sub_scene.unwrap_or(-1),"layerId":layer_id.unwrap_or(-1),"offset":offset,"limit":limit}), self.options.gateway.status_deadline)?;
        let raw: RawBridgeEntityList =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let entities = parse_world_selection_records(&raw.entities)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
            || entities.len() > limit
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        Ok(WorkbenchEntityListPage {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            world_revision: sha256(raw.world_path.as_bytes()),
            limit,
            next_cursor: workbench_bool(&raw.has_more)
                .then(|| format!("wel1:{signature}:{}", offset + entities.len())),
            truncated: workbench_bool(&raw.has_more),
            entities,
        })
    }

    pub fn search_entities(
        &self,
        query: Option<&str>,
        class_name: Option<&str>,
        resource_query: Option<&str>,
        component_classes: &[&str],
        relation: Option<&WorkbenchEntityRelationFilter>,
        sub_scene: Option<i32>,
        layer_id: Option<i32>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<WorkbenchEntitySearchPage, WorkbenchFailure> {
        let limit = limit.clamp(1, 100);
        if component_classes
            .iter()
            .any(|class_name| !valid_component_class_name(class_name))
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        if let Some(relation) = relation {
            if relation
                .class_name
                .as_deref()
                .is_some_and(|class_name| !valid_component_class_name(class_name))
                || relation.component_classes.len() > 32
                || relation
                    .component_classes
                    .iter()
                    .any(|class_name| !valid_component_class_name(class_name))
                || (relation.class_name.is_none() && relation.component_classes.is_empty())
                || relation.max_depth == 0
                || relation.max_depth > 8
                || matches!(
                    relation.direction,
                    WorkbenchEntityRelationDirection::Parent
                        | WorkbenchEntityRelationDirection::Child
                ) && relation.max_depth != 1
            {
                return Err(failure(WorkbenchFailureCode::Protocol));
            }
        }
        let components = component_classes.join(";");
        let (
            relation_direction,
            relation_class_name,
            relation_component_classes,
            relation_max_depth,
        ) = relation
            .map(|relation| {
                (
                    match relation.direction {
                        WorkbenchEntityRelationDirection::Parent => "parent",
                        WorkbenchEntityRelationDirection::Ancestor => "ancestor",
                        WorkbenchEntityRelationDirection::Child => "child",
                        WorkbenchEntityRelationDirection::Descendant => "descendant",
                    },
                    relation.class_name.as_deref().unwrap_or_default(),
                    relation.component_classes.join(";"),
                    relation.max_depth,
                )
            })
            .unwrap_or(("", "", String::new(), 0));
        let signature = sha256(
            format!(
                "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                query.unwrap_or_default(),
                class_name.unwrap_or_default(),
                resource_query.unwrap_or_default(),
                components,
                relation_direction,
                relation_class_name,
                relation_component_classes,
                relation_max_depth,
                sub_scene.map_or(String::new(), |v| v.to_string()),
                layer_id.map_or(String::new(), |v| v.to_string())
            )
            .as_bytes(),
        );
        let offset = match cursor {
            Some(cursor) => {
                let (found, offset) = parse_entity_list_cursor(cursor)
                    .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
                (found == signature)
                    .then_some(offset)
                    .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?
            }
            None => 0,
        };
        let value = self.gateway.request(json!({"APIFunc":"RST_WorkbenchSearchEntities","query":query.unwrap_or_default(),"className":class_name.unwrap_or_default(),"resourceQuery":resource_query.unwrap_or_default(),"componentClasses":components,"relationDirection":relation_direction,"relationClassName":relation_class_name,"relationComponentClasses":relation_component_classes,"relationMaxDepth":relation_max_depth,"subScene":sub_scene.unwrap_or(-1),"layerId":layer_id.unwrap_or(-1),"offset":offset,"limit":limit}), self.options.gateway.status_deadline)?;
        let raw: RawBridgeEntitySearch =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let results = parse_entity_search_records(&raw.results)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let available = raw.status == "available";
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
            || results.len() > limit
            || (available
                && (raw.named_matches > raw.total_matches
                    || raw.total_matches < results.len() as u32))
            || (!available
                && (!results.is_empty()
                    || raw.total_matches != 0
                    || raw.named_matches != 0
                    || workbench_bool(&raw.has_more)))
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        Ok(WorkbenchEntitySearchPage {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            world_revision: sha256(raw.world_path.as_bytes()),
            status: raw.status,
            limit,
            summary: WorkbenchEntitySearchSummary {
                total_matches: raw.total_matches,
                named_matches: raw.named_matches,
                anonymous_matches: raw.total_matches.saturating_sub(raw.named_matches),
            },
            next_cursor: (available && workbench_bool(&raw.has_more))
                .then(|| format!("wel1:{signature}:{}", offset + results.len())),
            truncated: available && workbench_bool(&raw.has_more),
            relation_traversal_truncated: workbench_bool(&raw.relation_traversal_truncated),
            results,
        })
    }

    pub fn layer_state(
        &self,
        sub_scene: i32,
        layer_id: i32,
    ) -> Result<WorkbenchLayerState, WorkbenchFailure> {
        let value = self.gateway.request(
            json!({"APIFunc":"RST_WorkbenchLayerState","subScene":sub_scene,"layerId":layer_id}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgeLayerState =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
            || raw.sub_scene != sub_scene
            || raw.layer_id != layer_id
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        Ok(WorkbenchLayerState {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            sub_scene: raw.sub_scene,
            layer_id: raw.layer_id,
            layer_path: raw.layer_path,
            visible: workbench_bool(&raw.visible),
            explicitly_locked: workbench_bool(&raw.explicitly_locked),
            locked_in_hierarchy: workbench_bool(&raw.locked_in_hierarchy),
        })
    }

    pub fn inspect_entity(
        &self,
        entity_id: &str,
    ) -> Result<WorkbenchEntityInspection, WorkbenchFailure> {
        let value = self.gateway.request(
            json!({"APIFunc":"RST_WorkbenchInspectEntity","entityId":entity_id}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgeSelectedEntityHierarchy =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let entity = parse_optional_world_selection_record(&raw.entity)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let ancestors = parse_world_selection_records(&raw.ancestors)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let children = parse_world_selection_records(&raw.children)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if ancestors.len() > 32 || children.len() > 64 {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let components = parse_component_summaries(&raw.components, &raw.component_properties)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let mut properties = parse_prefab_properties(&raw.properties)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        properties.retain(|property| {
            property.directly_overridden
                || matches!(property.path.as_str(), "coords" | "angles" | "scale")
        });
        let (contributor_addons, contributor_addons_truncated) =
            split_bounded_list(&raw.contributor_addons, 64, 4 * 1024);
        Ok(WorkbenchEntityInspection {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            editor_available: workbench_bool(&raw.editor_available),
            status: raw.status,
            entity,
            resource_name: raw.resource_name,
            resource_reference_kind: raw.resource_reference_kind,
            contributor_addons,
            contributor_addons_truncated: workbench_bool(&raw.contributor_addons_truncated)
                || contributor_addons_truncated,
            ancestors,
            ancestors_truncated: workbench_bool(&raw.ancestors_truncated),
            children,
            children_truncated: workbench_bool(&raw.children_truncated),
            components,
            component_properties_truncated: workbench_bool(&raw.component_properties_truncated),
            properties,
            properties_truncated: workbench_bool(&raw.properties_truncated),
        })
    }

    pub fn find_entities_by_radius(
        &self,
        options: WorkbenchEntityRadiusQueryOptions,
    ) -> Result<WorkbenchEntityRadiusQuery, WorkbenchFailure> {
        if !(0.01..=50_000.0).contains(&options.radius_meters)
            || !(1..=100).contains(&options.limit)
            || !matches!(
                options.query_scope.as_str(),
                "all" | "static" | "dynamic" | "features"
            )
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let value = self.gateway.request(
            json!({"APIFunc":"RST_WorkbenchFindEntitiesByRadius","centerX":options.center.x,"centerY":options.center.y,"centerZ":options.center.z,"radiusMeters":options.radius_meters,"queryScope":options.query_scope,"requireObject":options.require_object,"excludeProxies":options.exclude_proxies,"className":options.class_name.unwrap_or_default(),"limit":options.limit}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgeEntityRadiusQuery =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let entities = parse_world_selection_records(&raw.entities)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
            || entities.len() > options.limit
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        Ok(WorkbenchEntityRadiusQuery {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            center: WorkbenchEntityPosition {
                x: raw.center_x,
                y: raw.center_y,
                z: raw.center_z,
            },
            radius_meters: raw.radius_meters,
            query_scope: raw.query_scope,
            require_object: workbench_bool(&raw.require_object),
            exclude_proxies: workbench_bool(&raw.exclude_proxies),
            entities,
            truncated: workbench_bool(&raw.truncated),
        })
    }

    pub fn sample_terrain(
        &self,
        options: WorkbenchTerrainSampleOptions,
    ) -> Result<WorkbenchTerrainSample, WorkbenchFailure> {
        const MAX_HALF_EXTENT_METERS: f32 = 500.0;
        const MAX_SPACING_METERS: f32 = 500.0;
        const MAX_SAMPLES: usize = 4_096;
        if !options.center_x.is_finite()
            || !options.center_z.is_finite()
            || !(0.01..=MAX_HALF_EXTENT_METERS).contains(&options.half_extent_meters)
            || options.spacing_meters.is_some_and(|spacing| {
                !spacing.is_finite() || !(0.01..=MAX_SPACING_METERS).contains(&spacing)
            })
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let requested_spacing = options.spacing_meters.unwrap_or(0.0);
        let value = self.gateway.request(
            json!({"APIFunc":"RST_WorkbenchSampleTerrain","centerX":options.center_x,"centerZ":options.center_z,"halfExtentMeters":options.half_extent_meters,"spacingMeters":requested_spacing,"includeWater":options.include_water}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgeTerrainSample =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        if raw.status != "available" {
            return Ok(WorkbenchTerrainSample {
                bridge_version: raw.bridge_version,
                protocol_version: raw.protocol_version,
                status: raw.status,
                terrain: None,
                grid: None,
                summary: None,
                water: None,
                water_summary: None,
            });
        }
        let terrain =
            parse_terrain_metadata(&raw).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let grid = parse_terrain_grid(&raw, options.spacing_meters, MAX_SAMPLES)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let summary = summarize_terrain_grid(&grid);
        let water = options
            .include_water
            .then(|| parse_terrain_water_grid(&raw, grid.heights.len()))
            .transpose()
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let water_summary = water.as_ref().map(summarize_terrain_water_grid);
        Ok(WorkbenchTerrainSample {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            terrain: Some(terrain),
            grid: Some(grid),
            summary: Some(summary),
            water,
            water_summary,
        })
    }

    pub fn viewport_context(
        &self,
        options: WorkbenchViewportContextOptions,
    ) -> Result<WorkbenchViewportContext, WorkbenchFailure> {
        let value = self.gateway.request(
            json!({"APIFunc":"RST_WorkbenchViewportContext"}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgeViewportContext =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        if raw.status != "available" {
            return Ok(WorkbenchViewportContext {
                bridge_version: raw.bridge_version,
                protocol_version: raw.protocol_version,
                status: raw.status,
                width: options.include_ray.then_some(raw.width),
                height: options.include_ray.then_some(raw.height),
                mouse_x: options.include_ray.then_some(raw.mouse_x),
                mouse_y: options.include_ray.then_some(raw.mouse_y),
                mouse_inside: workbench_bool(&raw.mouse_inside),
                camera_position: None,
                camera_direction: None,
                mouse_world_position: None,
                ray_start: None,
                ray_end: None,
                ray_direction: None,
            });
        }
        let position =
            |x: f32, y: f32, z: f32| {
                (x.is_finite() && y.is_finite() && z.is_finite())
                    .then_some(WorkbenchEntityPosition { x, y, z })
            };
        let ray_start = position(raw.start_x, raw.start_y, raw.start_z);
        Ok(WorkbenchViewportContext {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            width: options.include_ray.then_some(raw.width),
            height: options.include_ray.then_some(raw.height),
            mouse_x: options.include_ray.then_some(raw.mouse_x),
            mouse_y: options.include_ray.then_some(raw.mouse_y),
            mouse_inside: workbench_bool(&raw.mouse_inside),
            camera_position: position(raw.camera_x, raw.camera_y, raw.camera_z),
            camera_direction: options
                .include_ray
                .then(|| {
                    position(
                        raw.camera_direction_x,
                        raw.camera_direction_y,
                        raw.camera_direction_z,
                    )
                })
                .flatten(),
            mouse_world_position: position(raw.end_x, raw.end_y, raw.end_z),
            ray_start: options.include_ray.then_some(ray_start).flatten(),
            ray_end: options
                .include_ray
                .then(|| position(raw.end_x, raw.end_y, raw.end_z))
                .flatten(),
            ray_direction: options
                .include_ray
                .then(|| position(raw.direction_x, raw.direction_y, raw.direction_z))
                .flatten(),
        })
    }

    pub fn trace(
        &self,
        options: WorkbenchTraceOptions,
    ) -> Result<WorkbenchTraceResult, WorkbenchFailure> {
        let valid =
            |p: &WorkbenchEntityPosition| p.x.is_finite() && p.y.is_finite() && p.z.is_finite();
        let trace_length = ((options.end.x - options.start.x).powi(2)
            + (options.end.y - options.start.y).powi(2)
            + (options.end.z - options.start.z).powi(2))
        .sqrt();
        if !valid(&options.start)
            || !valid(&options.end)
            || !trace_length.is_finite()
            || !(0.0..=MAX_WORKBENCH_TRACE_LENGTH_METERS).contains(&trace_length)
            || (!options.entities && !options.terrain && !options.ocean)
            || options
                .radius
                .is_some_and(|v| !v.is_finite() || !(0.001..=1000.0).contains(&v))
            || options.box_mins.as_ref().is_some_and(|p| !valid(p))
            || options.box_maxs.as_ref().is_some_and(|p| !valid(p))
            || options.target_layers.is_some_and(|layers| {
                !(1..=MAX_WORKBENCH_TARGET_LAYER_MASK).contains(&layers) || !options.entities
            })
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        if let (Some(mins), Some(maxs)) = (&options.box_mins, &options.box_maxs) {
            if mins.x < 0.0
                || mins.y < 0.0
                || mins.z < 0.0
                || maxs.x < mins.x
                || maxs.y < mins.y
                || maxs.z < mins.z
                || maxs.x - mins.x > MAX_WORKBENCH_TRACE_DIMENSION_METERS
                || maxs.y - mins.y > MAX_WORKBENCH_TRACE_DIMENSION_METERS
                || maxs.z - mins.z > MAX_WORKBENCH_TRACE_DIMENSION_METERS
                || (maxs.x == mins.x && maxs.y == mins.y && maxs.z == mins.z)
            {
                return Err(failure(WorkbenchFailureCode::Protocol));
            }
        }
        let shape = match options.shape {
            WorkbenchTraceShape::Line => "line",
            WorkbenchTraceShape::Sphere => "sphere",
            WorkbenchTraceShape::Box => "box",
        };
        if matches!(options.shape, WorkbenchTraceShape::Sphere) && options.radius.is_none()
            || matches!(options.shape, WorkbenchTraceShape::Box)
                && (options.box_mins.is_none() || options.box_maxs.is_none())
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let value = self.gateway.request(json!({"APIFunc":"RST_WorkbenchTrace","startX":options.start.x,"startY":options.start.y,"startZ":options.start.z,"endX":options.end.x,"endY":options.end.y,"endZ":options.end.z,"shape":shape,"radius":options.radius.unwrap_or(0.0),"minsX":options.box_mins.as_ref().map_or(0.0,|p|p.x),"minsY":options.box_mins.as_ref().map_or(0.0,|p|p.y),"minsZ":options.box_mins.as_ref().map_or(0.0,|p|p.z),"maxsX":options.box_maxs.as_ref().map_or(0.0,|p|p.x),"maxsY":options.box_maxs.as_ref().map_or(0.0,|p|p.y),"maxsZ":options.box_maxs.as_ref().map_or(0.0,|p|p.z),"entities":options.entities,"terrain":options.terrain,"ocean":options.ocean,"targetLayers":options.target_layers.unwrap_or(0)}), self.options.gateway.status_deadline)?;
        let raw: RawBridgeTrace =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let hit = workbench_bool(&raw.hit);
        let position = hit.then_some(WorkbenchEntityPosition {
            x: raw.hit_x,
            y: raw.hit_y,
            z: raw.hit_z,
        });
        let normal = hit.then_some(WorkbenchEntityPosition {
            x: raw.normal_x,
            y: raw.normal_y,
            z: raw.normal_z,
        });
        Ok(WorkbenchTraceResult {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: (!raw.status.is_empty())
                .then_some(raw.status)
                .unwrap_or_else(|| "available".to_string()),
            hit,
            fraction: hit.then_some(raw.fraction),
            distance: hit.then_some(raw.distance),
            position,
            normal,
            kind: match raw.kind.as_str() {
                "entity" => Some(WorkbenchTraceHitKind::Entity),
                "terrain" => Some(WorkbenchTraceHitKind::Terrain),
                "ocean" => Some(WorkbenchTraceHitKind::Ocean),
                _ => None,
            },
            entity: parse_optional_world_selection_record(&raw.entity)
                .map_err(|_| failure(WorkbenchFailureCode::Protocol))?,
            collider_name: (!raw.collider_name.is_empty()).then_some(raw.collider_name),
            material: (!raw.material.is_empty()).then_some(raw.material),
        })
    }

    pub fn clear_selection(&self) -> Result<WorkbenchWorldSelectionSummary, WorkbenchFailure> {
        self.selection_mutation("RST_WorkbenchClearSelection", json!({}))
    }

    pub fn shape_points(&self, entity_id: &str) -> Result<WorkbenchShapePoints, WorkbenchFailure> {
        self.shape_points_request("RST_WorkbenchShapePoints", json!({"entityId": entity_id}))
    }

    pub fn edit_shape_points(
        &self,
        entity_id: &str,
        edit: WorkbenchShapePointEdit,
        index: Option<usize>,
        count: Option<usize>,
        points: &[WorkbenchEntityPosition],
    ) -> Result<WorkbenchShapePoints, WorkbenchFailure> {
        let operation = match edit {
            WorkbenchShapePointEdit::Set => "set",
            WorkbenchShapePointEdit::Insert => "insert",
            WorkbenchShapePointEdit::Delete => "delete",
        };
        let encoded_points = points
            .iter()
            .map(|point| format!("{},{},{}", point.x, point.y, point.z))
            .collect::<Vec<_>>()
            .join(";");
        self.shape_points_request(
            "RST_WorkbenchEditShapePoints",
            json!({
                "entityId": entity_id,
                "operation": operation,
                "index": index.unwrap_or(0),
                "count": count.unwrap_or(1),
                "points": encoded_points,
            }),
        )
    }

    pub fn convert_shape_points(
        &self,
        entity_id: &str,
        from_space: WorkbenchShapePointSpace,
        to_space: WorkbenchShapePointSpace,
        points: &[WorkbenchEntityPosition],
    ) -> Result<WorkbenchShapePointConversion, WorkbenchFailure> {
        self.shape_geometry_request(
            "RST_WorkbenchShapeGeometry",
            json!({
                "entityId": entity_id,
                "operation": "convert",
                "fromSpace": shape_point_space_name(from_space),
                "toSpace": shape_point_space_name(to_space),
                "points": encode_shape_points(points),
            }),
        )
        .map(RawBridgeShapeGeometry::into_conversion)
    }

    pub fn transform_shape_points(
        &self,
        entity_id: &str,
        space: WorkbenchShapePointSpace,
        operation: WorkbenchShapeTransformOperation,
        offset: WorkbenchEntityPosition,
        pivot: WorkbenchEntityPosition,
        degrees: f32,
        scale: WorkbenchEntityPosition,
        mirror_axis: &str,
    ) -> Result<WorkbenchShapePoints, WorkbenchFailure> {
        let raw = self.shape_geometry_request(
            "RST_WorkbenchShapeGeometry",
            json!({
                "entityId": entity_id, "operation": "transform", "space": shape_point_space_name(space),
                "transformOperation": shape_transform_operation_name(operation),
                "offsetX": offset.x, "offsetY": offset.y, "offsetZ": offset.z,
                "pivotX": pivot.x, "pivotY": pivot.y, "pivotZ": pivot.z,
                "degrees": degrees, "scaleX": scale.x, "scaleY": scale.y, "scaleZ": scale.z,
                "mirrorAxis": mirror_axis,
            }),
        )?;
        Ok(raw.into_shape_points())
    }

    pub fn resample_polyline(
        &self,
        entity_id: &str,
        space: WorkbenchShapePointSpace,
        spacing_meters: f32,
    ) -> Result<WorkbenchPolylineResample, WorkbenchFailure> {
        let raw = self.shape_geometry_request(
            "RST_WorkbenchShapeGeometry",
            json!({"entityId": entity_id, "operation": "resample", "space": shape_point_space_name(space), "spacingMeters": spacing_meters}),
        )?;
        Ok(raw.into_resample())
    }

    pub fn set_selection(
        &self,
        entity_id: &str,
    ) -> Result<WorkbenchEntitySelectionResult, WorkbenchFailure> {
        let value = self.gateway.request(
            json!({"APIFunc":"RST_WorkbenchSetSelection","entityId":entity_id}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgeEntitySelection =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let entity = parse_optional_world_selection_record(&raw.entity)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        Ok(WorkbenchEntitySelectionResult {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            entity,
        })
    }

    fn shape_points_request(
        &self,
        api_func: &str,
        request: Value,
    ) -> Result<WorkbenchShapePoints, WorkbenchFailure> {
        let mut payload = request
            .as_object()
            .cloned()
            .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
        payload.insert("APIFunc".to_string(), Value::String(api_func.to_string()));
        let value = self
            .gateway
            .request(Value::Object(payload), self.options.gateway.status_deadline)?;
        let raw: RawBridgeShapePoints =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let entity = parse_optional_world_selection_record(&raw.entity)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let points = parse_shape_points(&raw.points)
            .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
        Ok(WorkbenchShapePoints {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            entity,
            shape_class: (!raw.shape_class.is_empty()).then_some(raw.shape_class),
            closed: raw.closed,
            points,
        })
    }

    fn shape_geometry_request(
        &self,
        api_func: &str,
        request: Value,
    ) -> Result<RawBridgeShapeGeometry, WorkbenchFailure> {
        let mut payload = request
            .as_object()
            .cloned()
            .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
        payload.insert("APIFunc".to_string(), Value::String(api_func.to_string()));
        let raw: RawBridgeShapeGeometry = serde_json::from_value(
            self.gateway
                .request(Value::Object(payload), self.options.gateway.status_deadline)?,
        )
        .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        raw.validate()
    }

    pub fn create_entity(
        &self,
        options: WorkbenchCreateEntityOptions,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        self.entity_mutation("RST_WorkbenchCreateEntity", json!({"resourceName":options.target,"targetIsResource":options.target_is_resource,"subScene":options.sub_scene,"x":options.position.x,"y":options.position.y,"z":options.position.z,"pitch":options.angles.x,"yaw":options.angles.y,"roll":options.angles.z,"layerId":options.layer_id,"name":options.name.unwrap_or_default()}))
    }

    pub fn rename_entity(
        &self,
        entity_id: &str,
        name: &str,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        self.entity_mutation(
            "RST_WorkbenchRenameEntity",
            json!({"entityId":entity_id,"name":name}),
        )
    }

    pub fn move_entity(
        &self,
        entity_id: &str,
        position: WorkbenchEntityPosition,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        self.entity_mutation(
            "RST_WorkbenchMoveEntity",
            json!({"entityId":entity_id,"x":position.x,"y":position.y,"z":position.z}),
        )
    }

    pub fn rotate_entity(
        &self,
        entity_id: &str,
        angles: WorkbenchEntityPosition,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        self.entity_mutation(
            "RST_WorkbenchRotateEntity",
            json!({"entityId":entity_id,"pitch":angles.x,"yaw":angles.y,"roll":angles.z}),
        )
    }

    pub fn reparent_entity(
        &self,
        entity_id: &str,
        parent_entity_id: &str,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        self.entity_mutation(
            "RST_WorkbenchReparentEntity",
            json!({"entityId":entity_id,"parentEntityId":parent_entity_id}),
        )
    }

    pub fn duplicate_entity(
        &self,
        entity_id: &str,
        position: WorkbenchEntityPosition,
        name: Option<&str>,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        self.entity_mutation(
            "RST_WorkbenchDuplicateEntity",
            json!({"entityId":entity_id,"x":position.x,"y":position.y,"z":position.z,"name":name.unwrap_or_default()}),
        )
    }

    pub fn delete_entity(
        &self,
        entity_id: &str,
        confirmation_token: Option<&str>,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        if let Some(token) = confirmation_token {
            let valid = self
                .delete_confirmations
                .lock()
                .unwrap()
                .remove(token)
                .is_some_and(|(bound_entity, issued)| {
                    bound_entity == entity_id && issued.elapsed() <= Duration::from_secs(30)
                });
            if !valid {
                return Ok(WorkbenchEntityMutationResult {
                    bridge_version: WORKBENCH_BRIDGE_VERSION.to_string(),
                    protocol_version: WORKBENCH_BRIDGE_PROTOCOL_VERSION,
                    status: "invalid-confirmation".to_string(),
                    active_layer_id: None,
                    entity: None,
                    confirmation_token: None,
                    destination: None,
                    destination_exists: None,
                    resource_name: None,
                    persistence_path: None,
                    template_saved: None,
                    inspection: None,
                });
            }
            return self.entity_mutation(
                "RST_WorkbenchDeleteEntity",
                json!({"entityId":entity_id,"confirm":true}),
            );
        }
        let mut preview = self.entity_mutation(
            "RST_WorkbenchDeleteEntity",
            json!({"entityId":entity_id,"confirm":false}),
        )?;
        if preview.status == "confirmation-required" {
            let sequence = DELETE_CONFIRMATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let token = format!(
                "del1:{}",
                sha256(format!("{entity_id}:{sequence}").as_bytes())
            );
            self.delete_confirmations
                .lock()
                .unwrap()
                .insert(token.clone(), (entity_id.to_string(), Instant::now()));
            preview.confirmation_token = Some(token);
        }
        Ok(preview)
    }

    fn entity_mutation(
        &self,
        api_func: &str,
        mut request: Value,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        let started = Instant::now();
        request["APIFunc"] = Value::String(api_func.to_string());
        let audit_request = request.clone();
        let value = self
            .gateway
            .request(request, self.options.gateway.status_deadline)?;
        let raw: RawBridgeEntitySelection =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let result = WorkbenchEntityMutationResult {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            active_layer_id: raw.active_layer_id,
            entity: parse_optional_world_selection_record(&raw.entity)
                .map_err(|_| failure(WorkbenchFailureCode::Protocol))?,
            confirmation_token: None,
            destination: (!raw.destination.is_empty()).then_some(raw.destination),
            destination_exists: raw.destination_exists,
            resource_name: None,
            persistence_path: None,
            template_saved: None,
            inspection: None,
        };
        self.log_event_timed(
            entity_mutation_operation(api_func),
            &result.status,
            started,
            entity_mutation_audit_details(&audit_request, &result),
        );
        Ok(result)
    }

    pub fn list_components(
        &self,
        entity_id: &str,
    ) -> Result<WorkbenchComponentResult, WorkbenchFailure> {
        self.component_operation(
            "RST_WorkbenchListComponents",
            json!({"entityId":entity_id}),
            None,
        )
    }
    pub fn inspect_component(
        &self,
        entity_id: &str,
        component_id: &str,
    ) -> Result<WorkbenchComponentResult, WorkbenchFailure> {
        let Some(native_component_id) = self.component_descriptor(entity_id, component_id)? else {
            return Ok(invalid_component_descriptor_result());
        };
        self.component_operation(
            "RST_WorkbenchInspectComponent",
            json!({"entityId":entity_id,"componentId":native_component_id}),
            Some(component_id),
        )
    }
    pub fn add_component(
        &self,
        entity_id: &str,
        class_name: &str,
    ) -> Result<WorkbenchComponentResult, WorkbenchFailure> {
        self.component_operation(
            "RST_WorkbenchAddComponent",
            json!({"entityId":entity_id,"className":class_name}),
            None,
        )
    }
    pub fn remove_component(
        &self,
        entity_id: &str,
        component_id: &str,
        confirmation_token: Option<&str>,
    ) -> Result<WorkbenchComponentResult, WorkbenchFailure> {
        let Some(native_component_id) = self.component_descriptor(entity_id, component_id)? else {
            return Ok(invalid_component_descriptor_result());
        };
        let bound = format!("{entity_id}|{component_id}|{native_component_id}");
        if let Some(token) = confirmation_token {
            let valid = self
                .delete_confirmations
                .lock()
                .unwrap()
                .remove(token)
                .is_some_and(|(value, issued)| {
                    value == bound && issued.elapsed() <= Duration::from_secs(30)
                });
            if !valid {
                return Ok(WorkbenchComponentResult {
                    bridge_version: WORKBENCH_BRIDGE_VERSION.to_string(),
                    protocol_version: WORKBENCH_BRIDGE_PROTOCOL_VERSION,
                    status: "invalid-confirmation".to_string(),
                    entity: None,
                    components: Vec::new(),
                    properties: Vec::new(),
                    confirmation_token: None,
                });
            }
            return self.component_operation(
                "RST_WorkbenchRemoveComponent",
                json!({"entityId":entity_id,"componentId":native_component_id,"confirm":true}),
                None,
            );
        }
        let mut preview = self.component_operation(
            "RST_WorkbenchRemoveComponent",
            json!({"entityId":entity_id,"componentId":native_component_id,"confirm":false}),
            None,
        )?;
        if preview.status == "confirmation-required" {
            let token = format!(
                "cmpdel1:{}",
                sha256(
                    format!(
                        "{bound}:{}",
                        DELETE_CONFIRMATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                    )
                    .as_bytes()
                )
            );
            self.delete_confirmations
                .lock()
                .unwrap()
                .insert(token.clone(), (bound, Instant::now()));
            preview.confirmation_token = Some(token);
        }
        Ok(preview)
    }
    pub fn set_component_property(
        &self,
        entity_id: &str,
        component_id: &str,
        write_descriptor: &str,
        value: Value,
    ) -> Result<WorkbenchComponentResult, WorkbenchFailure> {
        let Some(descriptor) =
            self.take_property_descriptor(entity_id, Some(component_id), write_descriptor)
        else {
            return Ok(invalid_component_property_descriptor_result());
        };
        let Some(value) = property_value_wire_format(&descriptor.data_type, &value) else {
            return Ok(invalid_component_property_descriptor_result());
        };
        let Some(native_component_id) = self.component_descriptor(entity_id, component_id)? else {
            return Ok(invalid_component_descriptor_result());
        };
        self.component_operation("RST_WorkbenchSetComponentProperty", json!({"entityId":entity_id,"componentId":native_component_id,"propertyName":descriptor.property_name,"expectedValue":descriptor.observed_value,"value":value}), Some(component_id))
    }
    fn component_operation(
        &self,
        api_func: &str,
        mut request: Value,
        inspected_component_id: Option<&str>,
    ) -> Result<WorkbenchComponentResult, WorkbenchFailure> {
        let started = Instant::now();
        let entity_id = request["entityId"].as_str().unwrap_or_default().to_string();
        request["APIFunc"] = Value::String(api_func.to_string());
        let audit_request = request.clone();
        let raw: RawBridgeComponentResult = serde_json::from_value(
            self.gateway
                .request(request, self.options.gateway.status_deadline)?,
        )
        .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let components = parse_components(&raw.components)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let mut properties = parse_properties(&raw.properties)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if let Some(component_id) = inspected_component_id {
            self.issue_property_descriptors(
                &entity_id,
                Some(component_id),
                &raw.properties,
                &mut properties,
            );
        }
        let result = WorkbenchComponentResult {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            entity: parse_optional_world_selection_record(&raw.entity)
                .map_err(|_| failure(WorkbenchFailureCode::Protocol))?,
            components,
            properties,
            confirmation_token: None,
        };
        if let Some(operation) = component_mutation_operation(api_func) {
            self.log_event_timed(
                operation,
                &result.status,
                started,
                component_mutation_audit_details(&audit_request, &result),
            );
        }
        Ok(result)
    }

    fn component_descriptor(
        &self,
        _entity_id: &str,
        descriptor_id: &str,
    ) -> Result<Option<String>, WorkbenchFailure> {
        Ok(is_native_component_descriptor(descriptor_id).then(|| descriptor_id.to_string()))
    }

    pub fn list_entity_properties(
        &self,
        entity_id: &str,
    ) -> Result<WorkbenchPropertyList, WorkbenchFailure> {
        let value = self.gateway.request(
            json!({"APIFunc":"RST_WorkbenchListEntityProperties","entityId":entity_id}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgePropertyList =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let mut properties = parse_properties(&raw.properties)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        self.issue_property_descriptors(entity_id, None, &raw.properties, &mut properties);
        Ok(WorkbenchPropertyList {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            properties,
        })
    }

    pub fn inspect_prefab_context(
        &self,
        entity_id: Option<&str>,
        resource_name: Option<&str>,
        member_id: Option<&str>,
    ) -> Result<WorkbenchPrefabContext, WorkbenchFailure> {
        if entity_id.is_some() == resource_name.is_some()
            || (member_id.is_some() && resource_name.is_none())
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let value = self.gateway.request(
            json!({"APIFunc":"RST_WorkbenchInspectPrefab","entityId":entity_id.unwrap_or_default(),"resourceName":resource_name.unwrap_or_default(),"memberId":member_id.unwrap_or_default()}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgePrefabContext =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let raw_properties = raw.properties.clone();
        Ok(WorkbenchPrefabContext {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            entity: parse_optional_world_selection_record(&raw.entity)
                .map_err(|_| failure(WorkbenchFailureCode::Protocol))?,
            resource_name: (!raw.resource_name.is_empty()).then_some(raw.resource_name),
            resource_reference_kind: (!raw.resource_reference_kind.is_empty())
                .then_some(raw.resource_reference_kind),
            contributor_addons: split_bounded_records(&raw.contributor_addons),
            ancestor_resources: split_bounded_records(&raw.ancestor_resources),
            ancestor_resources_truncated: workbench_bool(&raw.ancestor_resources_truncated),
            prefab_edit_mode: workbench_bool(&raw.prefab_edit_mode),
            member_id: (!raw.member_id.is_empty()).then_some(raw.member_id.clone()),
            components: parse_component_summaries(&raw.components, &raw.component_properties)
                .map_err(|_| failure(WorkbenchFailureCode::Protocol))?,
            children: parse_prefab_members(&raw.children, &raw.member_id)
                .map_err(|_| failure(WorkbenchFailureCode::Protocol))?,
            children_truncated: workbench_bool(&raw.children_truncated),
            properties: parse_prefab_properties(&raw.properties)
                .map_err(|_| failure(WorkbenchFailureCode::Protocol))?,
            properties_truncated: workbench_bool(&raw.properties_truncated),
            child_count: raw.child_count,
        })
        .map(|mut context| {
            if let Some(target_id) = context
                .entity
                .as_ref()
                .map(|entity| entity.entity_id.as_str())
                .or(context.resource_name.as_deref())
            {
                self.issue_prefab_property_descriptors(
                    target_id,
                    &raw_properties,
                    &mut context.properties,
                );
            }
            context
        })
    }

    pub fn inspect_prefab_component(
        &self,
        entity_id: Option<&str>,
        resource_name: Option<&str>,
        component_id: &str,
        member_id: Option<&str>,
    ) -> Result<WorkbenchPrefabComponentInspection, WorkbenchFailure> {
        if entity_id.is_some() == resource_name.is_some()
            || (member_id.is_some() && resource_name.is_none())
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let value = self.gateway.request(
            json!({"APIFunc":"RST_WorkbenchInspectPrefab","entityId":entity_id.unwrap_or_default(),"resourceName":resource_name.unwrap_or_default(),"memberId":member_id.unwrap_or_default()}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgePrefabContext =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let mut component = parse_prefab_components(&raw.components, &raw.component_properties)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?
            .into_iter()
            .find(|component| component.component_id == component_id);
        if let (Some(target_id), Some(component)) =
            (entity_id.or(resource_name), component.as_mut())
        {
            self.issue_prefab_component_property_descriptors(
                target_id,
                component_id,
                &raw.component_properties,
                &mut component.properties,
            );
        }
        Ok(WorkbenchPrefabComponentInspection {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: if component.is_some() {
                raw.status
            } else if raw.status == "available" {
                "component-not-found".to_string()
            } else {
                raw.status
            },
            resource_name: raw.resource_name,
            member_id: (!raw.member_id.is_empty()).then_some(raw.member_id),
            component,
        })
    }

    fn issue_prefab_property_descriptors(
        &self,
        entity_id: &str,
        raw: &str,
        properties: &mut [WorkbenchPrefabProperty],
    ) {
        let mut direct = parse_properties(raw).unwrap_or_default();
        self.issue_property_descriptors(entity_id, None, raw, &mut direct);
        for property in properties {
            property.write_descriptor = direct
                .iter()
                .find(|value| value.name == property.path)
                .and_then(|value| value.write_descriptor.clone());
        }
    }

    fn issue_prefab_component_property_descriptors(
        &self,
        entity_id: &str,
        component_id: &str,
        raw_component_properties: &str,
        properties: &mut [WorkbenchPrefabProperty],
    ) {
        let Some(component_index) = component_id
            .splitn(3, ':')
            .nth(1)
            .and_then(|index| index.parse::<u32>().ok())
        else {
            return;
        };
        let raw_properties = raw_component_properties
            .split(';')
            .filter_map(|record| {
                let (index, property) = record.split_once('|')?;
                (index.parse::<u32>().ok() == Some(component_index)).then_some(property)
            })
            .collect::<Vec<_>>()
            .join(";");
        let mut direct = parse_properties(&raw_properties).unwrap_or_default();
        self.issue_property_descriptors(
            entity_id,
            Some(component_id),
            &raw_properties,
            &mut direct,
        );
        for property in properties {
            property.write_descriptor = direct
                .iter()
                .find(|value| value.name == property.path)
                .and_then(|value| value.write_descriptor.clone());
        }
    }

    pub fn create_prefab(
        &self,
        entity_id: &str,
        destination: &str,
        confirmation_token: Option<&str>,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        let bound = format!("prefab-create|{entity_id}|{destination}");
        if let Some(token) = confirmation_token {
            let valid = self
                .delete_confirmations
                .lock()
                .unwrap()
                .remove(token)
                .is_some_and(|(value, issued)| {
                    value == bound && issued.elapsed() <= Duration::from_secs(30)
                });
            if !valid {
                return Ok(invalid_confirmation_result());
            }
            return self.entity_mutation(
                "RST_WorkbenchCreatePrefab",
                json!({"entityId":entity_id,"name":destination,"confirm":true}),
            );
        }
        let mut preview = self.entity_mutation(
            "RST_WorkbenchCreatePrefab",
            json!({"entityId":entity_id,"name":destination,"confirm":false}),
        )?;
        if preview.status == "confirmation-required" {
            let token = format!(
                "prefabcreate1:{}",
                sha256(
                    format!(
                        "{bound}:{}",
                        DELETE_CONFIRMATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                    )
                    .as_bytes()
                )
            );
            self.delete_confirmations
                .lock()
                .unwrap()
                .insert(token.clone(), (bound, Instant::now()));
            preview.confirmation_token = Some(token);
        }
        Ok(preview)
    }

    pub fn create_generic_prefab(
        &self,
        destination: &str,
        confirmation_token: Option<&str>,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        let bound = format!("generic-prefab-create|{destination}");
        if let Some(token) = confirmation_token {
            let valid = self
                .delete_confirmations
                .lock()
                .unwrap()
                .remove(token)
                .is_some_and(|(value, issued)| {
                    value == bound && issued.elapsed() <= Duration::from_secs(30)
                });
            if !valid {
                return Ok(invalid_confirmation_result());
            }
            return self.entity_mutation(
                "RST_WorkbenchCreateGenericPrefab",
                json!({"name":destination,"confirm":true}),
            );
        }
        let mut preview = self.entity_mutation(
            "RST_WorkbenchCreateGenericPrefab",
            json!({"name":destination,"confirm":false}),
        )?;
        if preview.status == "confirmation-required" {
            let token = format!(
                "genericprefabcreate1:{}",
                sha256(
                    format!(
                        "{bound}:{}",
                        DELETE_CONFIRMATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                    )
                    .as_bytes()
                )
            );
            self.delete_confirmations
                .lock()
                .unwrap()
                .insert(token.clone(), (bound, Instant::now()));
            preview.confirmation_token = Some(token);
        }
        Ok(preview)
    }

    pub fn save_prefab(
        &self,
        entity_id: Option<&str>,
        resource_name: Option<&str>,
        confirmation_token: Option<&str>,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        if entity_id.is_some() == resource_name.is_some()
            || resource_name.is_some_and(|name| !canonical_resource_name(name))
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let target = entity_id
            .or(resource_name)
            .expect("one prefab target is required");
        let bound = format!("prefab-save|{target}");
        if let Some(token) = confirmation_token {
            let valid = self
                .delete_confirmations
                .lock()
                .unwrap()
                .remove(token)
                .is_some_and(|(value, issued)| {
                    value == bound && issued.elapsed() <= Duration::from_secs(30)
                });
            if !valid {
                return Ok(invalid_confirmation_result());
            }
            let mut result = self.entity_mutation(
                "RST_WorkbenchSavePrefab",
                json!({"entityId":entity_id.unwrap_or_default(),"resourceName":resource_name.unwrap_or_default(),"confirm":true}),
            )?;
            if let Some(resource_name) = resource_name.filter(|_| result.status == "prefab-saved") {
                result.resource_name = Some(resource_name.to_string());
                result.persistence_path = Some("workbench-resource".to_string());
                result.template_saved = Some(true);
                result.inspection =
                    Some(self.inspect_prefab_context(None, Some(resource_name), None)?);
            }
            return Ok(result);
        }
        let mut preview = self.entity_mutation(
            "RST_WorkbenchSavePrefab",
            json!({"entityId":entity_id.unwrap_or_default(),"resourceName":resource_name.unwrap_or_default(),"confirm":false}),
        )?;
        if preview.status == "confirmation-required" {
            let token = format!(
                "prefabsave1:{}",
                sha256(
                    format!(
                        "{bound}:{}",
                        DELETE_CONFIRMATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                    )
                    .as_bytes()
                )
            );
            self.delete_confirmations
                .lock()
                .unwrap()
                .insert(token.clone(), (bound, Instant::now()));
            preview.confirmation_token = Some(token);
        }
        Ok(preview)
    }

    pub fn add_prefab_resource_component(
        &self,
        resource_name: &str,
        class_name: &str,
        confirmation_token: Option<&str>,
    ) -> Result<WorkbenchPrefabResourceMutationResult, WorkbenchFailure> {
        if !canonical_resource_name(resource_name) || !canonical_component_class_name(class_name) {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }

        let bound = format!("prefab-resource-component-add|{resource_name}|{class_name}");
        let confirm = if let Some(token) = confirmation_token {
            let valid = self
                .delete_confirmations
                .lock()
                .unwrap()
                .remove(token)
                .is_some_and(|(value, issued)| {
                    value == bound && issued.elapsed() <= Duration::from_secs(30)
                });
            if !valid {
                return Ok(invalid_prefab_resource_mutation_result(resource_name));
            }
            true
        } else {
            false
        };

        let raw: RawBridgePrefabResourceComponentMutation =
            serde_json::from_value(self.gateway.request(
                json!({
                    "APIFunc": "RST_WorkbenchAddPrefabResourceComponent",
                    "resourceName": resource_name,
                    "className": class_name,
                    "confirm": confirm,
                }),
                self.options.gateway.status_deadline,
            )?)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }

        let mut result = WorkbenchPrefabResourceMutationResult {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            resource_name: raw.resource_name,
            persistence_path: "workbench-resource".to_string(),
            component_id: (raw.component_index >= 0 && !raw.component_class.is_empty())
                .then(|| format!("cmp1:{}:{}", raw.component_index, raw.component_class)),
            component_class: (!raw.component_class.is_empty()).then_some(raw.component_class),
            template_saved: raw.template_saved,
            inspection: None,
            component_inspection: None,
            confirmation_token: None,
        };
        if result.status == "confirmation-required" {
            let token = format!(
                "prefabresourceadd1:{}",
                sha256(
                    format!(
                        "{bound}:{}",
                        DELETE_CONFIRMATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                    )
                    .as_bytes()
                )
            );
            self.delete_confirmations
                .lock()
                .unwrap()
                .insert(token.clone(), (bound, Instant::now()));
            result.confirmation_token = Some(token);
        }
        if result.status == "prefab-component-added" && result.template_saved {
            result.inspection =
                Some(self.inspect_prefab_context(None, Some(resource_name), None)?);
        }
        Ok(result)
    }

    pub fn remove_prefab_resource_component(
        &self,
        resource_name: &str,
        component_id: &str,
        confirmation_token: Option<&str>,
    ) -> Result<WorkbenchPrefabResourceMutationResult, WorkbenchFailure> {
        if !canonical_resource_name(resource_name) {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let Some((component_index, class_name)) = parse_prefab_component_id(component_id) else {
            return Err(failure(WorkbenchFailureCode::Protocol));
        };

        let bound = format!("prefab-resource-component-remove|{resource_name}|{component_id}");
        let confirm = if let Some(token) = confirmation_token {
            let valid = self
                .delete_confirmations
                .lock()
                .unwrap()
                .remove(token)
                .is_some_and(|(value, issued)| {
                    value == bound && issued.elapsed() <= Duration::from_secs(30)
                });
            if !valid {
                return Ok(invalid_prefab_resource_mutation_result(resource_name));
            }
            true
        } else {
            false
        };

        let raw: RawBridgePrefabResourceComponentMutation =
            serde_json::from_value(self.gateway.request(
                json!({
                    "APIFunc": "RST_WorkbenchRemovePrefabResourceComponent",
                    "resourceName": resource_name,
                    "className": class_name,
                    "componentIndex": component_index,
                    "confirm": confirm,
                }),
                self.options.gateway.status_deadline,
            )?)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }

        let mut result = WorkbenchPrefabResourceMutationResult {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            resource_name: raw.resource_name,
            persistence_path: "workbench-resource".to_string(),
            component_id: Some(component_id.to_string()),
            component_class: (!raw.component_class.is_empty()).then_some(raw.component_class),
            template_saved: raw.template_saved,
            inspection: None,
            component_inspection: None,
            confirmation_token: None,
        };
        if result.status == "confirmation-required" {
            let token = format!(
                "prefabresourceremove1:{}",
                sha256(
                    format!(
                        "{bound}:{}",
                        DELETE_CONFIRMATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                    )
                    .as_bytes()
                )
            );
            self.delete_confirmations
                .lock()
                .unwrap()
                .insert(token.clone(), (bound, Instant::now()));
            result.confirmation_token = Some(token);
        }
        if result.status == "prefab-component-removed" && result.template_saved {
            result.inspection =
                Some(self.inspect_prefab_context(None, Some(resource_name), None)?);
        }
        Ok(result)
    }

    pub fn set_prefab_resource_property(
        &self,
        resource_name: &str,
        component_id: Option<&str>,
        write_descriptor: &str,
        value: Value,
        confirmation_token: Option<&str>,
    ) -> Result<WorkbenchPrefabResourceMutationResult, WorkbenchFailure> {
        if !canonical_resource_name(resource_name) {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let component = match component_id {
            Some(component_id) => Some(
                parse_prefab_component_id(component_id)
                    .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?,
            ),
            None => None,
        };
        let component_class = component
            .as_ref()
            .map(|(_, class_name)| class_name.as_str());
        let component_index = component.as_ref().map_or(-1, |(index, _)| *index);
        let component_id = component_id.unwrap_or_default();
        let bound_target = format!("{resource_name}|{component_id}|{write_descriptor}");

        let confirm = confirmation_token.is_some();
        let descriptor = self.peek_property_descriptor(
            resource_name,
            (!component_id.is_empty()).then_some(component_id),
            write_descriptor,
        );
        let Some(mut descriptor) = descriptor else {
            return Ok(invalid_prefab_resource_mutation_result(resource_name));
        };
        let Some(value) = property_value_wire_format(&descriptor.data_type, &value) else {
            return Ok(invalid_prefab_resource_mutation_result(resource_name));
        };
        let bound = format!(
            "prefab-resource-property|{bound_target}|{}",
            sha256(value.as_bytes())
        );

        if let Some(token) = confirmation_token {
            let valid = self
                .delete_confirmations
                .lock()
                .unwrap()
                .remove(token)
                .is_some_and(|(stored, issued)| {
                    stored == bound && issued.elapsed() <= Duration::from_secs(30)
                });
            if !valid {
                return Ok(invalid_prefab_resource_mutation_result(resource_name));
            }
            let Some(taken_descriptor) = self.take_property_descriptor(
                resource_name,
                (!component_id.is_empty()).then_some(component_id),
                write_descriptor,
            ) else {
                return Ok(invalid_prefab_resource_mutation_result(resource_name));
            };
            descriptor = taken_descriptor;
        }

        let raw: RawBridgePrefabResourcePropertyMutation =
            serde_json::from_value(self.gateway.request(
                json!({
                    "APIFunc": "RST_WorkbenchSetPrefabResourceProperty",
                    "resourceName": resource_name,
                    "componentIndex": component_index,
                    "componentClass": component_class.unwrap_or_default(),
                    "propertyName": descriptor.property_name,
                    "expectedValue": descriptor.observed_value,
                    "value": value,
                    "confirm": confirm,
                }),
                self.options.gateway.status_deadline,
            )?)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.bridge_version != WORKBENCH_BRIDGE_VERSION
            || raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }

        let mut result = WorkbenchPrefabResourceMutationResult {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            status: raw.status,
            resource_name: raw.resource_name,
            persistence_path: "workbench-resource".to_string(),
            component_id: (!component_id.is_empty()).then_some(component_id.to_string()),
            component_class: component_class.map(str::to_string),
            template_saved: raw.template_saved,
            inspection: None,
            component_inspection: None,
            confirmation_token: None,
        };
        if result.status == "confirmation-required" {
            let token = format!(
                "prefabresourceproperty1:{}",
                sha256(
                    format!(
                        "{bound}:{}",
                        DELETE_CONFIRMATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                    )
                    .as_bytes()
                )
            );
            self.delete_confirmations
                .lock()
                .unwrap()
                .insert(token.clone(), (bound, Instant::now()));
            result.confirmation_token = Some(token);
        }
        if result.status == "prefab-property-set" && result.template_saved {
            if component_id.is_empty() {
                result.inspection =
                    Some(self.inspect_prefab_context(None, Some(resource_name), None)?);
            } else {
                result.component_inspection = Some(self.inspect_prefab_component(
                    None,
                    Some(resource_name),
                    component_id,
                    None,
                )?);
            }
        }
        Ok(result)
    }

    pub fn set_prefab_property(
        &self,
        entity_id: &str,
        write_descriptor: &str,
        value: Value,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        let Some(descriptor) = self.take_property_descriptor(entity_id, None, write_descriptor)
        else {
            return Ok(invalid_property_descriptor_result());
        };
        let Some(value) = property_value_wire_format(&descriptor.data_type, &value) else {
            return Ok(invalid_property_descriptor_result());
        };
        self.entity_mutation("RST_WorkbenchSetPrefabProperty", json!({"entityId":entity_id,"propertyName":descriptor.property_name,"expectedValue":descriptor.observed_value,"value":value}))
    }
    pub fn set_prefab_component_property(
        &self,
        entity_id: &str,
        component_id: &str,
        write_descriptor: &str,
        value: Value,
    ) -> Result<WorkbenchComponentResult, WorkbenchFailure> {
        let Some(descriptor) =
            self.take_property_descriptor(entity_id, Some(component_id), write_descriptor)
        else {
            return Ok(invalid_component_property_descriptor_result());
        };
        let Some(value) = property_value_wire_format(&descriptor.data_type, &value) else {
            return Ok(invalid_component_property_descriptor_result());
        };
        let Some(native_component_id) = self.component_descriptor(entity_id, component_id)? else {
            return Ok(invalid_component_descriptor_result());
        };
        self.component_operation("RST_WorkbenchSetPrefabComponentProperty", json!({"entityId":entity_id,"componentId":native_component_id,"propertyName":descriptor.property_name,"expectedValue":descriptor.observed_value,"value":value}), Some(component_id))
    }
    pub fn set_entity_property(
        &self,
        entity_id: &str,
        write_descriptor: &str,
        value: Value,
    ) -> Result<WorkbenchEntityMutationResult, WorkbenchFailure> {
        let Some(descriptor) = self.take_property_descriptor(entity_id, None, write_descriptor)
        else {
            return Ok(invalid_property_descriptor_result());
        };
        let Some(value) = property_value_wire_format(&descriptor.data_type, &value) else {
            return Ok(invalid_property_descriptor_result());
        };
        self.entity_mutation("RST_WorkbenchSetEntityProperty", json!({"entityId":entity_id,"propertyName":descriptor.property_name,"expectedValue":descriptor.observed_value,"value":value}))
    }

    fn issue_property_descriptors(
        &self,
        entity_id: &str,
        component_id: Option<&str>,
        raw_properties: &str,
        properties: &mut [WorkbenchDirectProperty],
    ) {
        let raw_values = raw_properties
            .split(';')
            .filter_map(|record| {
                let mut fields = record.split('|');
                let name = fields.next()?;
                let data_type = fields.next()?;
                let value = fields.next()?;
                fields.next();
                Some((name, (data_type, value)))
            })
            .collect::<HashMap<_, _>>();
        let Ok(mut descriptors) = self.property_write_descriptors.lock() else {
            return;
        };
        descriptors.retain(|_, descriptor| descriptor.issued.elapsed() <= Duration::from_secs(30));
        for property in properties {
            let Some((data_type, observed_value)) = raw_values.get(property.name.as_str()).copied()
            else {
                continue;
            };
            if !supported_property_type(data_type) {
                continue;
            }
            let sequence = PROPERTY_DESCRIPTOR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let descriptor_id = format!(
                "prop2:{}",
                sha256(
                    format!(
                        "{entity_id}|{}|{}|{data_type}|{observed_value}|{sequence}",
                        component_id.unwrap_or_default(),
                        property.name
                    )
                    .as_bytes()
                )
            );
            descriptors.insert(
                descriptor_id.clone(),
                PropertyWriteDescriptor {
                    entity_id: entity_id.to_string(),
                    component_id: component_id.map(str::to_string),
                    property_name: property.name.clone(),
                    data_type: data_type.to_string(),
                    observed_value: observed_value.to_string(),
                    issued: Instant::now(),
                },
            );
            property.write_descriptor = Some(descriptor_id);
        }
    }

    fn take_property_descriptor(
        &self,
        entity_id: &str,
        component_id: Option<&str>,
        descriptor_id: &str,
    ) -> Option<PropertyWriteDescriptor> {
        let mut descriptors = self.property_write_descriptors.lock().ok()?;
        let descriptor = descriptors.remove(descriptor_id)?;
        (descriptor.issued.elapsed() <= Duration::from_secs(30)
            && descriptor.entity_id == entity_id
            && descriptor.component_id.as_deref() == component_id)
            .then_some(descriptor)
    }

    fn peek_property_descriptor(
        &self,
        entity_id: &str,
        component_id: Option<&str>,
        descriptor_id: &str,
    ) -> Option<PropertyWriteDescriptor> {
        let mut descriptors = self.property_write_descriptors.lock().ok()?;
        descriptors.retain(|_, descriptor| descriptor.issued.elapsed() <= Duration::from_secs(30));
        descriptors
            .get(descriptor_id)
            .cloned()
            .filter(|descriptor| {
                descriptor.entity_id == entity_id
                    && descriptor.component_id.as_deref() == component_id
            })
    }

    fn selection_mutation(
        &self,
        api_func: &str,
        mut request: Value,
    ) -> Result<WorkbenchWorldSelectionSummary, WorkbenchFailure> {
        request["APIFunc"] = Value::String(api_func.to_string());
        let value = self
            .gateway
            .request(request, self.options.gateway.status_deadline)?;
        let raw: RawBridgeWorldSelection =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        let selected_entities = parse_world_selection_records(&raw.selected_entities)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if selected_entities.len() > 32 || selected_entities.len() > raw.selected_count as usize {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        Ok(WorkbenchWorldSelectionSummary {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            editor_available: workbench_bool(&raw.editor_available),
            status: raw.status,
            selected_count: raw.selected_count,
            selected_entities_truncated: workbench_bool(&raw.selected_entities_truncated)
                || selected_entities.len() < raw.selected_count as usize,
            selected_entities,
        })
    }

    fn dispatch_background_reload_action(&self) -> Result<RawBridgeReloadAction, WorkbenchFailure> {
        let value = self.gateway.request(
            json!({"APIFunc": "RST_WorkbenchState", "executeReloadAction": true}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgeReloadAction =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        // Reload is the one recovery action that must remain callable from the
        // immediately previous compatible handler after its managed package is upgraded.
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
            || raw.action_path != "Plugins/Settings/Reload WB Scripts"
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        Ok(raw)
    }

    fn dispatch_background_save_all_action(
        &self,
    ) -> Result<RawBridgeSaveAllAction, WorkbenchFailure> {
        let value = self.gateway.request(
            json!({"APIFunc": "RST_WorkbenchState", "executeSaveAllAction": true}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgeSaveAllAction =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
            || raw.action_path != "File/Save All"
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        Ok(raw)
    }

    fn dispatch_background_save_world_action(
        &self,
    ) -> Result<RawBridgeSaveWorldAction, WorkbenchFailure> {
        let value = self.gateway.request(
            json!({"APIFunc": "RST_WorkbenchState", "executeSaveWorldAction": true}),
            self.options.gateway.status_deadline,
        )?;
        let raw: RawBridgeSaveWorldAction =
            serde_json::from_value(value).map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
            || raw.action_path != "WorldEditor.Save"
        {
            return Err(failure(WorkbenchFailureCode::Protocol));
        }
        Ok(raw)
    }

    /// Dispatch the fixed Workbench Save All action in-process and wait briefly for it to settle.
    pub fn save_all(&self) -> Result<WorkbenchSaveAllResult, WorkbenchFailure> {
        const POST_SAVE_ACTION_DELAY: Duration = Duration::from_millis(750);

        let started = Instant::now();
        let processes = workbench_processes();
        let [process] = processes.as_slice() else {
            return Err(self.correlate_failure_details(
                "save-all",
                "ambiguous-workbench-process",
                failure(WorkbenchFailureCode::Unavailable),
                json!({"processCount": processes.len()}),
            ));
        };
        let workbench_was_minimized =
            workbench_has_minimized_window(process.id).map_err(|outcome| {
                self.correlate_failure_details(
                    "save-all",
                    outcome,
                    failure(WorkbenchFailureCode::Unavailable),
                    json!({"processId": process.id}),
                )
            })?;
        let action = self
            .dispatch_background_save_all_action()
            .map_err(|dispatch_failure| {
                self.correlate_failure_details(
                    "save-all",
                    "workbench-save-all-action-unavailable",
                    dispatch_failure,
                    json!({"processId": process.id}),
                )
            })?;
        if !workbench_bool(&action.accepted)
            || (action.world_save_status == "saved" && !workbench_bool(&action.world_save_accepted))
            || !matches!(
                action.world_save_status.as_str(),
                "saved" | "skipped-no-open-world"
            )
        {
            return Err(self.correlate_failure_details(
                "save-all",
                "workbench-save-all-or-world-save-rejected",
                failure(WorkbenchFailureCode::Unavailable),
                json!({
                    "processId": process.id,
                    "actionPath": action.action_path,
                    "saveAllAccepted": workbench_bool(&action.accepted),
                    "worldSaveAccepted": workbench_bool(&action.world_save_accepted),
                    "worldSaveStatus": action.world_save_status,
                }),
            ));
        }
        std::thread::sleep(POST_SAVE_ACTION_DELAY);
        let result = WorkbenchSaveAllResult {
            process_id: process.id,
            workbench_was_minimized,
            save_all_accepted: true,
            world_save_accepted: workbench_bool(&action.world_save_accepted),
            world_save_status: action.world_save_status,
            action_path: action.action_path,
        };
        self.log_event_timed(
            "save-all",
            "accepted",
            started,
            json!({
                "processId": result.process_id,
                "workbenchWasMinimized": result.workbench_was_minimized,
                "actionPath": result.action_path,
                "worldSaveAccepted": result.world_save_accepted,
                "worldSaveStatus": result.world_save_status,
            }),
        );
        Ok(result)
    }

    /// Save the active World Editor document through its own Workbench module. This intentionally
    /// remains separate from Resource Manager Save All.
    pub fn save_world(&self) -> Result<WorkbenchSaveWorldResult, WorkbenchFailure> {
        const POST_SAVE_ACTION_DELAY: Duration = Duration::from_millis(750);

        let started = Instant::now();
        let processes = workbench_processes();
        let [process] = processes.as_slice() else {
            return Err(self.correlate_failure_details(
                "save-world",
                "ambiguous-workbench-process",
                failure(WorkbenchFailureCode::Unavailable),
                json!({"processCount": processes.len()}),
            ));
        };
        let workbench_was_minimized =
            workbench_has_minimized_window(process.id).map_err(|outcome| {
                self.correlate_failure_details(
                    "save-world",
                    outcome,
                    failure(WorkbenchFailureCode::Unavailable),
                    json!({"processId": process.id}),
                )
            })?;
        let action = self
            .dispatch_background_save_world_action()
            .map_err(|dispatch_failure| {
                self.correlate_failure_details(
                    "save-world",
                    "world-editor-save-unavailable",
                    dispatch_failure,
                    json!({"processId": process.id}),
                )
            })?;
        if !workbench_bool(&action.accepted)
            || !matches!(
                action.world_save_status.as_str(),
                "saved" | "skipped-no-open-world"
            )
        {
            return Err(self.correlate_failure_details(
                "save-world",
                "world-editor-save-unavailable",
                failure(WorkbenchFailureCode::Unavailable),
                json!({
                    "processId": process.id,
                    "actionPath": action.action_path,
                    "worldSaveStatus": action.world_save_status,
                }),
            ));
        }
        std::thread::sleep(POST_SAVE_ACTION_DELAY);
        let result = WorkbenchSaveWorldResult {
            process_id: process.id,
            workbench_was_minimized,
            world_save_accepted: workbench_bool(&action.accepted),
            world_save_status: action.world_save_status,
            action_path: action.action_path,
        };
        self.log_event_timed(
            "save-world",
            "accepted",
            started,
            json!({
                "processId": result.process_id,
                "workbenchWasMinimized": result.workbench_was_minimized,
                "actionPath": result.action_path,
                "worldSaveStatus": result.world_save_status,
            }),
        );
        Ok(result)
    }

    /// Dispatch the fixed Workbench reload action in-process after a confirmed Save All action.
    ///
    /// The dispatcher response only records whether Workbench reported accepting the action.
    /// Success is established solely by a complete fresh reload marker sequence in the Workbench
    /// log.
    pub fn activate_scripts(&self) -> Result<WorkbenchScriptActivationResult, WorkbenchFailure> {
        const RELOAD_VERIFICATION_DEADLINE: Duration = Duration::from_secs(60);
        const RELOAD_VERIFICATION_POLL: Duration = Duration::from_millis(500);

        let started = Instant::now();
        let processes = workbench_processes();
        let [process] = processes.as_slice() else {
            return Err(self.correlate_failure_details(
                "activate-scripts",
                "ambiguous-workbench-process",
                failure(WorkbenchFailureCode::Unavailable),
                json!({"processCount": processes.len()}),
            ));
        };
        let save_result = self.save_all().map_err(|save_failure| {
            self.correlate_failure_details(
                "activate-scripts",
                "workbench-save-all-before-reload-failed",
                save_failure,
                json!({"processId": process.id}),
            )
        })?;
        let workbench_was_minimized = save_result.workbench_was_minimized;
        let world_saved_before_reload = save_result.world_save_accepted;
        let world_save_status = save_result.world_save_status;
        let log_before = latest_workbench_log(&self.paths().workbench_root)
            .and_then(|path| log_cursor(&path).ok())
            .ok_or_else(|| {
                self.correlate_failure_details(
                    "activate-scripts",
                    "workbench-log-unavailable",
                    failure(WorkbenchFailureCode::Unavailable),
                    json!({"processId": process.id}),
                )
            })?;
        reload_action_path(self.dispatch_background_reload_action()).map_err(
            |dispatch_failure| {
                self.correlate_failure_details(
                    "activate-scripts",
                    "workbench-reload-action-unavailable",
                    dispatch_failure,
                    json!({"processId": process.id}),
                )
            },
        )?;
        while started.elapsed() < RELOAD_VERIFICATION_DEADLINE {
            if let Some(verification) = latest_workbench_log(&self.paths().workbench_root)
                .and_then(|path| reload_verification_since(&path, Some(&log_before)).ok())
                .flatten()
            {
                if !workbench_processes().contains(process) {
                    return Err(self.correlate_failure_details(
                        "activate-scripts",
                        "workbench-process-changed",
                        failure(WorkbenchFailureCode::Unavailable),
                        json!({"processId": process.id}),
                    ));
                }
                let result = WorkbenchScriptActivationResult {
                    process_id: process.id,
                    workbench_was_minimized,
                    world_saved_before_reload,
                    world_save_status,
                    reload_verified: true,
                    log_path: verification.path,
                    verification_lines: verification.lines,
                };
                self.log_event_timed(
                    "activate-scripts",
                    "verified",
                    started,
                    json!({
                        "processId": result.process_id,
                        "workbenchWasMinimized": result.workbench_was_minimized,
                        "worldSavedBeforeReload": result.world_saved_before_reload,
                        "worldSaveStatus": result.world_save_status,
                        "logPath": result.log_path,
                        "verificationLineCount": result.verification_lines.len(),
                    }),
                );
                return Ok(result);
            }
            std::thread::sleep(RELOAD_VERIFICATION_POLL);
        }
        Err(self.correlate_failure_details(
            "activate-scripts",
            "reload-not-verified",
            failure(WorkbenchFailureCode::Timeout),
            json!({
                "processId": process.id,
                "verificationDeadlineMs": RELOAD_VERIFICATION_DEADLINE.as_millis(),
            }),
        ))
    }

    pub fn read_logs(
        &self,
        source: &str,
        line_count: usize,
    ) -> Result<WorkbenchLogRead, WorkbenchFailure> {
        let line_count = line_count.clamp(1, 500);
        let path = match source {
            "integration" => Some(self.integration_log_path()),
            "workbench" => latest_workbench_log(&self.paths().workbench_root),
            _ => return Err(failure(WorkbenchFailureCode::Protocol)),
        };
        let Some(path) = path else {
            return Ok(WorkbenchLogRead {
                source: source.to_string(),
                path: None,
                lines: Vec::new(),
                markers: Vec::new(),
                truncated: false,
            });
        };
        if !path.is_file() {
            return Ok(WorkbenchLogRead {
                source: source.to_string(),
                path: Some(path),
                lines: Vec::new(),
                markers: Vec::new(),
                truncated: false,
            });
        }
        let (lines, truncated) = bounded_log_tail(&path, line_count).map_err(|error| {
            self.correlate_failure_details(
                "read_logs",
                "read-failed",
                failure(WorkbenchFailureCode::Unavailable),
                json!({
                    "source": source,
                    "errorKind": format!("{:?}", error.kind()),
                }),
            )
        })?;
        let result = WorkbenchLogRead {
            source: source.to_string(),
            path: Some(path),
            markers: workbench_log_markers(source, &lines),
            lines,
            truncated,
        };
        self.log_event_timed(
            "read-logs",
            "success",
            Instant::now(),
            json!({
                "source": result.source.clone(),
                "lineCount": result.lines.len(),
                "markerCount": result.markers.len(),
                "truncated": result.truncated,
            }),
        );
        Ok(result)
    }

    pub fn launch(&self) -> Result<WorkbenchProcessResult, WorkbenchFailure> {
        self.launch_project(None)
    }

    fn launch_project(
        &self,
        project: Option<&std::path::Path>,
    ) -> Result<WorkbenchProcessResult, WorkbenchFailure> {
        let started = Instant::now();
        let existing = workbench_processes();
        self.observe_processes(&existing);
        if let Some(process) = existing.first() {
            let net_api_connected =
                self.native_status().is_ok() || self.wait_for_net_api(Duration::from_secs(90));
            if !net_api_connected {
                return Err(self.correlate_failure_details(
                    "launch",
                    "net-api-timeout",
                    failure(WorkbenchFailureCode::Timeout),
                    json!({
                        "processId": process.id,
                        "alreadyRunning": true,
                    }),
                ));
            }
            let result = WorkbenchProcessResult {
                process_id: Some(process.id),
                already_running: true,
                net_api_connected,
                exited: false,
                user_interaction_required: false,
            };
            self.log_event_timed(
                "launch",
                "already-running",
                started,
                json!({"processId": process.id, "netApiConnected": true}),
            );
            return Ok(result);
        }
        let paths = self.paths();
        let executable = paths
            .executable
            .as_ref()
            .filter(|path| is_workbench_executable(path))
            .cloned()
            .ok_or_else(|| {
                self.correlate_failure_details(
                    "launch",
                    "executable-unavailable",
                    failure(WorkbenchFailureCode::Unavailable),
                    json!({"executableValidated": false}),
                )
            })?;
        let working_directory = executable.parent().map(std::path::Path::to_path_buf);
        let arguments =
            workbench_launch_arguments(project, paths.game.as_deref()).ok_or_else(|| {
                self.correlate_failure_details(
                    "launch",
                    "base-game-addon-directory-unavailable",
                    failure(WorkbenchFailureCode::Unavailable),
                    json!({
                        "project": project,
                        "gameDirectoryDiscovered": paths.game.is_some(),
                    }),
                )
            })?;
        let mut command = std::process::Command::new(executable);
        command
            .args(arguments)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Some(working_directory) = working_directory {
            command.current_dir(working_directory);
        }
        let child = command.spawn().map_err(|error| {
            self.correlate_failure_details(
                "launch",
                "process-start-failed",
                failure(WorkbenchFailureCode::Unavailable),
                json!({"errorKind": format!("{:?}", error.kind())}),
            )
        })?;
        if let Some(process) = workbench_processes()
            .into_iter()
            .find(|process| process.id == child.id())
        {
            self.observe_processes(&[process]);
        }
        let net_api_connected = self.wait_for_net_api(Duration::from_secs(90));
        if !net_api_connected {
            return Err(self.correlate_failure_details(
                "launch",
                "net-api-timeout",
                failure(WorkbenchFailureCode::Timeout),
                json!({
                    "processId": child.id(),
                    "alreadyRunning": false,
                    "processStillRunning": workbench_process_ids().contains(&child.id()),
                }),
            ));
        }
        let result = WorkbenchProcessResult {
            process_id: Some(child.id()),
            already_running: false,
            net_api_connected,
            exited: false,
            user_interaction_required: false,
        };
        self.log_event_timed(
            "launch",
            if net_api_connected {
                "connected"
            } else {
                "net-api-timeout"
            },
            started,
            json!({
                "processId": result.process_id,
                "alreadyRunning": result.already_running,
                "netApiConnected": result.net_api_connected,
            }),
        );
        Ok(result)
    }

    pub fn stop(&self, process_id: u32) -> Result<WorkbenchProcessResult, WorkbenchFailure> {
        let started = Instant::now();
        let current = workbench_processes();
        let observed = self.observed_processes.lock().ok().and_then(|processes| {
            processes
                .iter()
                .find(|process| process.id == process_id)
                .copied()
        });
        if observed.is_none() || !current.iter().any(|process| Some(*process) == observed) {
            return Err(self.correlate_failure_details(
                "stop",
                "stale-or-unobserved-process",
                failure(WorkbenchFailureCode::Unavailable),
                json!({
                    "processId": process_id,
                    "observedBySession": observed.is_some(),
                    "currentIdentityMatched": false,
                }),
            ));
        }
        let observed = observed.expect("checked observed process identity");
        let script = format!(
            "$p=Get-Process -Id {process_id} -ErrorAction Stop; \
             if ($p.ProcessName -ne 'ArmaReforgerWorkbenchSteamDiag' -or \
                 [uint64]$p.StartTime.ToUniversalTime().Ticks -ne [uint64]{}) {{ exit 2 }}; \
             [void]$p.CloseMainWindow()",
            observed.start_ticks
        );
        let status = std::process::Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &script,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|error| {
                self.correlate_failure_details(
                    "stop",
                    "graceful-close-request-failed",
                    failure(WorkbenchFailureCode::Unavailable),
                    json!({
                        "processId": process_id,
                        "errorKind": format!("{:?}", error.kind()),
                    }),
                )
            })?;
        if !status.success() {
            return Err(self.correlate_failure_details(
                "stop",
                "graceful-close-failed",
                failure(WorkbenchFailureCode::Unavailable),
                json!({
                    "processId": process_id,
                    "exitCode": status.code(),
                }),
            ));
        }
        for _ in 0..20 {
            if !workbench_process_ids().contains(&process_id) {
                self.log_event_timed("stop", "exited", started, json!({"processId": process_id}));
                return Ok(WorkbenchProcessResult {
                    process_id: Some(process_id),
                    already_running: false,
                    net_api_connected: false,
                    exited: true,
                    user_interaction_required: false,
                });
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        let result = WorkbenchProcessResult {
            process_id: Some(process_id),
            already_running: false,
            net_api_connected: self.native_status().is_ok(),
            exited: false,
            user_interaction_required: true,
        };
        self.log_event_timed(
            "stop",
            "user-interaction-required",
            started,
            json!({
                "processId": process_id,
                "netApiConnected": result.net_api_connected,
            }),
        );
        Ok(result)
    }

    pub fn restart(&self, process_id: u32) -> Result<WorkbenchProcessResult, WorkbenchFailure> {
        let current = workbench_processes();
        let Some(process) = current
            .iter()
            .find(|process| process.id == process_id)
            .copied()
        else {
            return Err(self.correlate_failure_details(
                "restart",
                "workbench-process-not-running",
                failure(WorkbenchFailureCode::Unavailable),
                json!({"processId": process_id}),
            ));
        };
        let paths = self.paths();
        let project = workbench_project_gproj(process)
            .or_else(|| {
                workbench_project_title(process)
                    .and_then(|title| resolve_project_gproj(&paths.workbench_root, &title))
            })
            .ok_or_else(|| {
                self.correlate_failure_details(
                    "restart",
                    "project-not-resolved",
                    failure(WorkbenchFailureCode::Unavailable),
                    json!({"processId": process_id}),
                )
            })?;
        if base_game_addons_directory(paths.game.as_deref()).is_none() {
            return Err(self.correlate_failure_details(
                "restart",
                "base-game-addon-directory-unavailable",
                failure(WorkbenchFailureCode::Unavailable),
                json!({
                    "processId": process_id,
                    "gameDirectoryDiscovered": paths.game.is_some(),
                }),
            ));
        }
        self.save_all().map_err(|save_failure| {
            self.correlate_failure_details(
                "restart",
                "workbench-save-all-before-force-close-failed",
                save_failure,
                json!({"processId": process_id}),
            )
        })?;
        let script = force_stop_workbench_script(process);
        let status = std::process::Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &script,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|error| {
                self.correlate_failure_details(
                    "restart",
                    "force-close-request-failed",
                    failure(WorkbenchFailureCode::Unavailable),
                    json!({
                        "processId": process_id,
                        "errorKind": format!("{:?}", error.kind()),
                    }),
                )
            })?;
        if !status.success() {
            return Err(self.correlate_failure_details(
                "restart",
                "force-close-failed",
                failure(WorkbenchFailureCode::Unavailable),
                json!({"processId": process_id, "exitCode": status.code()}),
            ));
        }
        for _ in 0..20 {
            if !workbench_processes().contains(&process) {
                return self.launch_project(Some(&project));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Err(self.correlate_failure_details(
            "restart",
            "force-close-not-observed",
            failure(WorkbenchFailureCode::Unavailable),
            json!({"processId": process_id}),
        ))
    }

    fn active_bridge_status(
        &self,
        bridge_directory: &std::path::Path,
        retry_activation: bool,
    ) -> ManagedBridgeStatus {
        let disk = self.bridge_disk_status(bridge_directory);
        let expected_protocol =
            fs::read(bridge_directory.join("reforger-script-tools.manifest.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<BridgeManifest>(&bytes).ok())
                .map(|manifest| manifest.protocol_version);
        let call = || {
            self.gateway.request(
                json!({"APIFunc": "RST_WorkbenchCapabilities"}),
                self.options.gateway.status_deadline,
            )
        };
        let decode = |value: Result<Value, WorkbenchFailure>| {
            value
                .ok()
                .and_then(|value| serde_json::from_value::<RawBridgeCapabilities>(value).ok())
        };
        let mut raw = decode(call());
        let handshake_matches = |raw: &RawBridgeCapabilities| {
            disk.installed_version
                .as_deref()
                .is_some_and(|version| version == raw.bridge_version)
                && expected_protocol == Some(raw.protocol_version)
        };
        if retry_activation && raw.as_ref().is_none_or(|raw| !handshake_matches(raw)) {
            let _ = self.gateway.validate_scripts();
            raw = decode(call());
        }
        let Some(raw) = raw else {
            return ManagedBridgeStatus {
                activation_required: disk.installed,
                ..disk
            };
        };
        let (capabilities, capabilities_truncated) = split_bounded_list(&raw.capabilities, 32, 64);
        let compatible = raw.protocol_version == WORKBENCH_BRIDGE_PROTOCOL_VERSION;
        let activation_required = disk.installed && !handshake_matches(&raw);
        ManagedBridgeStatus {
            installed: disk.installed,
            installation_available: false,
            installed_version: disk.installed_version,
            active_version: Some(raw.bridge_version),
            protocol_version: Some(raw.protocol_version),
            compatible,
            activation_required,
            capabilities: if compatible { capabilities } else { Vec::new() },
            capabilities_truncated: compatible && capabilities_truncated,
        }
    }

    fn bridge_disk_status(&self, bridge_directory: &std::path::Path) -> ManagedBridgeStatus {
        let manifest = fs::read(bridge_directory.join("reforger-script-tools.manifest.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BridgeManifest>(&bytes).ok());
        let installed = manifest.is_some();
        ManagedBridgeStatus {
            installed,
            installation_available: false,
            installed_version: manifest.map(|value| value.bridge_version),
            active_version: None,
            protocol_version: None,
            compatible: false,
            activation_required: installed,
            capabilities: Vec::new(),
            capabilities_truncated: false,
        }
    }

    fn maintain_existing_bridge(&self, bridge_directory: &std::path::Path) -> ManagedBridgeStatus {
        let started = Instant::now();
        let Ok(_maintenance) = self.maintenance_lock.lock() else {
            return self.bridge_disk_status(bridge_directory);
        };
        let repaired = match self.repair_managed_files(bridge_directory) {
            Ok(repaired) => repaired,
            Err(error) => {
                self.log_event_timed(
                    "maintenance",
                    "repair-failed",
                    started,
                    json!({
                        "errorKind": format!("{:?}", error.kind()),
                        "managedFiles": bridge_payload()
                            .iter()
                            .map(|(name, _)| *name)
                            .collect::<Vec<_>>(),
                    }),
                );
                return self.bridge_disk_status(bridge_directory);
            }
        };
        if repaired {
            let _ = self.gateway.validate_scripts();
        }
        // A missing custom NET API function is logged by Workbench as an error.
        // Maintenance and diagnosis must therefore never probe an unregistered
        // handler. Explicit custom operations remain responsible for their own
        // availability result.
        let status = self.bridge_disk_status(bridge_directory);
        self.log_event_timed(
            "maintenance",
            if repaired {
                "updated-reload-required"
            } else {
                "activation-pending"
            },
            started,
            json!({
                "repaired": repaired,
                "installedVersion": status.installed_version.clone(),
                "activeVersion": status.active_version.clone(),
                "protocolVersion": status.protocol_version,
                "managedFileCount": bridge_payload().len(),
                "managedFiles": bridge_payload()
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>(),
            }),
        );
        status
    }

    fn repair_managed_files(&self, bridge_directory: &std::path::Path) -> std::io::Result<bool> {
        let manifest_path = bridge_directory.join("reforger-script-tools.manifest.json");
        let manifest = fs::read(&manifest_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BridgeManifest>(&bytes).ok());
        let needs_repair = manifest.as_ref().is_none_or(|manifest| {
            version_order(&manifest.bridge_version, WORKBENCH_BRIDGE_VERSION).is_lt()
                || (manifest.bridge_version == WORKBENCH_BRIDGE_VERSION
                    && (manifest.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION
                        || !manifest_matches_payload(manifest)
                        || bridge_payload().iter().any(|(name, content)| {
                            fs::read(bridge_directory.join(name))
                                .ok()
                                .is_none_or(|bytes| sha256(&bytes) != sha256(content.as_bytes()))
                        })))
        });
        if needs_repair {
            self.write_managed_files(bridge_directory)?;
        }
        Ok(needs_repair)
    }

    fn migrate_legacy_bridge(
        &self,
        legacy_directory: &std::path::Path,
        bridge_directory: &std::path::Path,
    ) -> std::io::Result<bool> {
        let legacy_manifest_path = legacy_directory.join("reforger-script-tools.manifest.json");
        let Some(manifest) = fs::read(&legacy_manifest_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BridgeManifest>(&bytes).ok())
        else {
            return Ok(false);
        };
        if !manifest_matches_payload(&manifest) {
            return Ok(false);
        }
        fs::create_dir_all(bridge_directory)?;
        for file in &manifest.files {
            let source = legacy_directory.join(&file.name);
            let destination = bridge_directory.join(&file.name);
            if destination.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "managed bridge migration destination already exists",
                ));
            }
            fs::rename(source, destination)?;
        }
        fs::rename(
            legacy_manifest_path,
            bridge_directory.join("reforger-script-tools.manifest.json"),
        )?;
        Ok(true)
    }

    fn write_managed_files(&self, bridge_directory: &std::path::Path) -> std::io::Result<()> {
        self.write_managed_payload(bridge_directory, bridge_payload())
    }

    fn write_managed_payload(
        &self,
        bridge_directory: &std::path::Path,
        payload: &[(&str, &str)],
    ) -> std::io::Result<()> {
        if fs::symlink_metadata(bridge_directory)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "managed bridge directory cannot be a symbolic link",
            ));
        }
        fs::create_dir_all(bridge_directory)?;
        let previous = fs::read(bridge_directory.join("reforger-script-tools.manifest.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BridgeManifest>(&bytes).ok());
        for (name, content) in payload {
            fs::write(bridge_directory.join(name), content)?;
        }
        let files = payload
            .iter()
            .map(|(name, content)| BridgeManifestFile {
                name: (*name).to_string(),
                sha256: sha256(content.as_bytes()),
            })
            .collect::<Vec<_>>();
        if let Some(previous) = previous {
            for file in previous.files {
                if is_managed_file_name(&file.name)
                    && !files.iter().any(|current| current.name == file.name)
                {
                    let _ = fs::remove_file(bridge_directory.join(file.name));
                }
            }
        }
        let manifest = BridgeManifest {
            bridge_version: WORKBENCH_BRIDGE_VERSION.to_string(),
            protocol_version: WORKBENCH_BRIDGE_PROTOCOL_VERSION,
            files,
        };
        fs::write(
            bridge_directory.join("reforger-script-tools.manifest.json"),
            serde_json::to_vec_pretty(&manifest).expect("bridge manifest serializes"),
        )
    }

    fn paths(&self) -> ResolvedWorkbenchPaths {
        let user = self
            .options
            .user_directory
            .clone()
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
            .unwrap_or_default();
        let workbench_root = user
            .join("Documents")
            .join("My Games")
            .join("ArmaReforgerWorkbench");
        let profile = workbench_root.join("profile");
        let scripts_directory = profile.join("scripts");
        let bridge_directory = scripts_directory
            .join("WorkbenchGame")
            .join("reforger-script-tools");
        let legacy_bridge_directory = scripts_directory.join("reforger-script-tools");
        let (game, game_source) = if let Some(game) = self.options.game_directory.clone() {
            (Some(game), "explicit".to_string())
        } else {
            (
                discover_steam_app("1874880", "Arma Reforger"),
                "steam-discovery".to_string(),
            )
        };
        let (tools, tools_source) = if let Some(tools) = self.options.tools_directory.clone() {
            (Some(tools), "explicit".to_string())
        } else {
            (
                discover_steam_app("1874910", "Arma Reforger Tools"),
                "steam-discovery".to_string(),
            )
        };
        let (executable, executable_source) =
            if let Some(executable) = self.options.executable.clone() {
                (Some(executable), "explicit".to_string())
            } else {
                (
                    tools.as_ref().map(|tools| {
                        tools
                            .join("Workbench")
                            .join("ArmaReforgerWorkbenchSteamDiag.exe")
                    }),
                    "tools-installation".to_string(),
                )
            };
        ResolvedWorkbenchPaths {
            workbench_root,
            profile,
            bridge_directory,
            legacy_bridge_directory,
            game,
            game_source,
            tools,
            tools_source,
            executable,
            executable_source,
        }
    }

    fn integration_log_path(&self) -> PathBuf {
        let user = self
            .options
            .user_directory
            .clone()
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
            .unwrap_or_default();
        user.join("AppData")
            .join("Local")
            .join("ReforgerScriptTools")
            .join("logs")
            .join("workbench.log")
    }

    fn log_event_timed(
        &self,
        operation: &str,
        outcome: &str,
        started: Instant,
        details: Value,
    ) -> String {
        use std::io::Write;
        let reference = next_log_reference();
        let path = self.integration_log_path();
        let Some(parent) = path.parent() else {
            return reference;
        };
        if fs::create_dir_all(parent).is_err() {
            return reference;
        }
        if fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 1024 * 1024) {
            let rotated = path.with_extension("log.1");
            let _ = fs::remove_file(&rotated);
            let _ = fs::rename(&path, rotated);
        }
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_millis())
                .unwrap_or_default();
            let record = json!({
                "reference": reference,
                "timestampMs": timestamp,
                "operation": operation,
                "outcome": outcome,
                "durationMs": started.elapsed().as_millis(),
                "details": details,
            });
            let _ = writeln!(file, "{record}");
        }
        reference
    }

    fn correlate_failure_details(
        &self,
        operation: &str,
        outcome: &str,
        mut failure: WorkbenchFailure,
        details: Value,
    ) -> WorkbenchFailure {
        if failure.log_reference.is_none() {
            failure.log_reference =
                Some(self.log_event_timed(operation, outcome, Instant::now(), details));
        }
        failure
    }

    fn wait_for_net_api(&self, deadline: Duration) -> bool {
        let started = std::time::Instant::now();
        while started.elapsed() < deadline {
            if self.gateway.status().is_ok() {
                let paths = self.paths();
                if self.bridge_disk_status(&paths.bridge_directory).installed {
                    let _ = self.maintain_existing_bridge(&paths.bridge_directory);
                }
                return true;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        false
    }

    fn observe_processes(&self, processes: &[ProcessIdentity]) {
        if let Ok(mut observed) = self.observed_processes.lock() {
            observed.extend(processes.iter().copied());
        }
    }
}

fn entity_mutation_operation(api_func: &str) -> &str {
    match api_func {
        "RST_WorkbenchCreateEntity" => "create-entity",
        "RST_WorkbenchRenameEntity" => "rename-entity",
        "RST_WorkbenchMoveEntity" => "move-entity",
        "RST_WorkbenchRotateEntity" => "rotate-entity",
        "RST_WorkbenchReparentEntity" => "reparent-entity",
        "RST_WorkbenchDuplicateEntity" => "duplicate-entity",
        "RST_WorkbenchDeleteEntity" => "delete-entity",
        "RST_WorkbenchSetEntityProperty" => "set-entity-property",
        "RST_WorkbenchCreatePrefab" => "create-prefab",
        "RST_WorkbenchSavePrefab" => "save-prefab",
        "RST_WorkbenchSetPrefabProperty" => "set-prefab-property",
        _ => "entity-mutation",
    }
}

fn entity_mutation_audit_details(request: &Value, result: &WorkbenchEntityMutationResult) -> Value {
    let entity = result.entity.as_ref();
    json!({
        "entityId": request.get("entityId").and_then(Value::as_str),
        "parentEntityId": request.get("parentEntityId").and_then(Value::as_str),
        "propertyName": request.get("propertyName").and_then(Value::as_str),
        "resultEntityId": entity.map(|entity| entity.entity_id.as_str()),
        "resultClass": entity.map(|entity| entity.class_name.as_str()),
        "activeLayerId": result.active_layer_id,
    })
}

fn component_mutation_operation(api_func: &str) -> Option<&str> {
    match api_func {
        "RST_WorkbenchAddComponent" => Some("add-component"),
        "RST_WorkbenchRemoveComponent" => Some("remove-component"),
        "RST_WorkbenchSetComponentProperty" => Some("set-component-property"),
        "RST_WorkbenchSetPrefabComponentProperty" => Some("set-prefab-component-property"),
        _ => None,
    }
}

fn component_mutation_audit_details(request: &Value, result: &WorkbenchComponentResult) -> Value {
    let entity = result.entity.as_ref();
    json!({
        "entityId": request.get("entityId").and_then(Value::as_str),
        "componentClass": request.get("className").and_then(Value::as_str),
        "propertyName": request.get("propertyName").and_then(Value::as_str),
        "resultEntityId": entity.map(|entity| entity.entity_id.as_str()),
        "resultClass": entity.map(|entity| entity.class_name.as_str()),
        "componentCount": result.components.len(),
    })
}

impl WorkbenchGateway {
    pub fn new(options: WorkbenchGatewayOptions) -> Self {
        Self {
            options,
            request_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn status(&self) -> Result<WorkbenchStatus, WorkbenchFailure> {
        let value = self.request(
            json!({"APIFunc": "IsWorkbenchRunning"}),
            self.options.status_deadline,
        )?;
        serde_json::from_value::<RawWorkbenchStatus>(value)
            .map(|value| WorkbenchStatus {
                is_running: value.is_running,
                scripts_compiled: value.scripts_compiled,
            })
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))
    }

    pub fn validate_scripts(&self) -> Result<WorkbenchValidation, WorkbenchFailure> {
        let value = self.request(
            json!({"APIFunc": "ValidateScripts", "Configuration": "WORKBENCH"}),
            self.options.validation_deadline,
        )?;
        let value = serde_json::from_value::<RawValidation>(value)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        let mut diagnostics: Vec<WorkbenchCompilerDiagnostic> = Vec::new();
        let mut identities = HashMap::<DiagnosticIdentity, usize>::new();
        for (severity, raw) in value
            .errors
            .into_iter()
            .map(|value| (WorkbenchDiagnosticSeverity::Error, value))
            .chain(
                value
                    .warnings
                    .into_iter()
                    .map(|value| (WorkbenchDiagnosticSeverity::Warning, value)),
            )
        {
            let diagnostic = WorkbenchCompilerDiagnostic {
                severity,
                message: raw.error,
                location: WorkbenchDiagnosticLocation {
                    file: raw.file,
                    file_abs: raw.file_abs.map(PathBuf::from),
                    addon: raw.addon,
                    line: raw.line,
                },
            };
            let identity = DiagnosticIdentity::from(&diagnostic);
            if let Some(index) = identities.get(&identity).copied() {
                if diagnostics[index].severity == WorkbenchDiagnosticSeverity::Warning
                    && severity == WorkbenchDiagnosticSeverity::Error
                {
                    diagnostics[index] = diagnostic;
                }
            } else {
                identities.insert(identity, diagnostics.len());
                diagnostics.push(diagnostic);
            }
        }
        Ok(WorkbenchValidation {
            profile: "WORKBENCH".to_string(),
            success: value.success,
            diagnostics,
        })
    }

    fn request(&self, payload: Value, deadline: Duration) -> Result<Value, WorkbenchFailure> {
        let _request = self
            .request_lock
            .lock()
            .map_err(|_| failure(WorkbenchFailureCode::Unavailable))?;
        let started = Instant::now();
        let ip = self
            .options
            .host
            .parse::<IpAddr>()
            .map_err(|_| failure(WorkbenchFailureCode::Unavailable))?;
        if !ip.is_loopback() {
            return Err(failure(WorkbenchFailureCode::Unavailable));
        }
        let address = SocketAddr::new(ip, self.options.port);
        let mut stream = TcpStream::connect_timeout(&address, deadline)
            .map_err(|_| failure(WorkbenchFailureCode::Unavailable))?;
        stream
            .set_read_timeout(Some(deadline))
            .map_err(|_| failure(WorkbenchFailureCode::Unavailable))?;
        stream
            .set_write_timeout(Some(deadline))
            .map_err(|_| failure(WorkbenchFailureCode::Unavailable))?;
        stream
            .write_all(&1_i32.to_le_bytes())
            .and_then(|_| write_string(&mut stream, "ReforgerScriptTools"))
            .and_then(|_| write_string(&mut stream, "JsonRPC"))
            .and_then(|_| write_string(&mut stream, &payload.to_string()))
            .map_err(map_io_failure)?;
        stream.shutdown(Shutdown::Write).map_err(map_io_failure)?;
        let error_code = read_string(&mut stream).map_err(|error| {
            if started.elapsed() >= deadline {
                failure(WorkbenchFailureCode::Timeout)
            } else {
                map_io_failure(error)
            }
        })?;
        if started.elapsed() >= deadline {
            return Err(failure(WorkbenchFailureCode::Timeout));
        }
        let payload = read_string(&mut stream).map_err(|error| {
            if started.elapsed() >= deadline {
                failure(WorkbenchFailureCode::Timeout)
            } else {
                map_io_failure(error)
            }
        })?;
        if started.elapsed() >= deadline {
            return Err(failure(WorkbenchFailureCode::Timeout));
        }
        if error_code != "Ok" {
            return Err(failure(WorkbenchFailureCode::WorkbenchError));
        }
        serde_json::from_str(&payload).map_err(|_| failure(WorkbenchFailureCode::Protocol))
    }
}

const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Deserialize)]
struct RawWorkbenchStatus {
    #[serde(rename = "IsRunning")]
    is_running: bool,
    #[serde(rename = "ScriptsCompiled")]
    scripts_compiled: bool,
}

#[derive(Deserialize)]
struct RawValidation {
    #[serde(rename = "Success")]
    success: bool,
    #[serde(rename = "Errors")]
    errors: Vec<RawDiagnostic>,
    #[serde(rename = "Warnings")]
    warnings: Vec<RawDiagnostic>,
}

#[derive(Deserialize)]
struct RawDiagnostic {
    error: String,
    file: String,
    #[serde(rename = "fileAbs")]
    file_abs: Option<String>,
    addon: Option<String>,
    line: usize,
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct DiagnosticIdentity {
    message: String,
    file: String,
    file_abs: Option<PathBuf>,
    addon: Option<String>,
    line: usize,
}

impl From<&WorkbenchCompilerDiagnostic> for DiagnosticIdentity {
    fn from(value: &WorkbenchCompilerDiagnostic) -> Self {
        Self {
            message: value.message.clone(),
            file: value.location.file.clone(),
            file_abs: value.location.file_abs.clone(),
            addon: value.location.addon.clone(),
            line: value.location.line,
        }
    }
}

fn write_string(stream: &mut impl Write, value: &str) -> std::io::Result<()> {
    let bytes = value.as_bytes();
    let length = i32::try_from(bytes.len())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "string too large"))?;
    stream.write_all(&length.to_le_bytes())?;
    stream.write_all(bytes)
}

fn read_string(stream: &mut impl Read) -> std::io::Result<String> {
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length)?;
    let length = i32::from_le_bytes(length);
    if length < 0 || length as usize > MAX_RESPONSE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid NET API string length",
        ));
    }
    let mut bytes = vec![0_u8; length as usize];
    stream.read_exact(&mut bytes)?;
    String::from_utf8(bytes)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid UTF-8"))
}

fn map_io_failure(error: std::io::Error) -> WorkbenchFailure {
    if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        failure(WorkbenchFailureCode::Timeout)
    } else {
        failure(WorkbenchFailureCode::Protocol)
    }
}

fn failure(code: WorkbenchFailureCode) -> WorkbenchFailure {
    WorkbenchFailure {
        code,
        log_reference: None,
    }
}

static LOG_REFERENCE_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static DELETE_CONFIRMATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static PROPERTY_DESCRIPTOR_SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn next_log_reference() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default();
    let sequence = LOG_REFERENCE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("wb-{}-{timestamp}-{sequence}", std::process::id())
}

fn failure_code(code: WorkbenchFailureCode) -> &'static str {
    match code {
        WorkbenchFailureCode::ConsentRequired => "workbench_installation_consent_required",
        WorkbenchFailureCode::Unavailable => "workbench_unavailable",
        WorkbenchFailureCode::Timeout => "workbench_timeout",
        WorkbenchFailureCode::Protocol => "workbench_protocol_error",
        WorkbenchFailureCode::WorkbenchError => "workbench_error",
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeManifest {
    bridge_version: String,
    protocol_version: u32,
    files: Vec<BridgeManifestFile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeManifestFile {
    name: String,
    sha256: String,
}

#[derive(Deserialize)]
struct RawBridgeCapabilities {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    capabilities: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWorkbenchEditorList {
    editors: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWorkbenchOpenEditor {
    editor_id: String,
    opened: Value,
    status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWorkbenchOpenResource {
    resource_path: String,
    opened: Value,
    status: String,
}

#[derive(Deserialize)]
struct RawBridgeReloadAction {
    #[serde(rename = "bridgeVersion")]
    _bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    #[serde(rename = "reloadActionAccepted")]
    _accepted: Value,
    #[serde(rename = "reloadActionPath")]
    action_path: String,
}

fn reload_action_path(
    dispatch: Result<RawBridgeReloadAction, WorkbenchFailure>,
) -> Result<String, WorkbenchFailure> {
    match dispatch {
        // A false dispatcher acknowledgement is not a reliable failure signal: Workbench can
        // begin reloading while its handler still reports false. The caller's log verification
        // decides whether the reload actually happened.
        Ok(action) => Ok(action.action_path),
        // Reloading can tear down the in-flight script handler before it returns a response.
        // The fresh console marker sequence is likewise authoritative in that case.
        Err(dispatch_failure) if dispatch_failure.code == WorkbenchFailureCode::Timeout => {
            Ok("Plugins/Settings/Reload WB Scripts".to_string())
        }
        Err(dispatch_failure) => Err(dispatch_failure),
    }
}

#[derive(Deserialize)]
struct RawBridgeSaveAllAction {
    #[serde(rename = "bridgeVersion")]
    _bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    #[serde(rename = "saveAllActionAccepted")]
    accepted: Value,
    #[serde(rename = "worldSaveActionAccepted")]
    world_save_accepted: Value,
    #[serde(rename = "worldSaveStatus")]
    world_save_status: String,
    #[serde(rename = "saveAllActionPath")]
    action_path: String,
}

#[derive(Deserialize)]
struct RawBridgeSaveWorldAction {
    #[serde(rename = "bridgeVersion")]
    _bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    #[serde(rename = "worldSaveActionAccepted")]
    accepted: Value,
    #[serde(rename = "worldSaveStatus")]
    world_save_status: String,
    #[serde(rename = "worldSaveActionPath")]
    action_path: String,
}

#[derive(Deserialize)]
struct RawBridgeState {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    mode: String,
    #[serde(rename = "worldEditorActive", default)]
    world_editor_active: Value,
    #[serde(rename = "worldEditorModulePresent", default)]
    world_editor_module_present: Value,
    #[serde(rename = "worldEditorApiAvailable", default)]
    world_editor_api_available: Value,
    #[serde(rename = "playSession")]
    play_session: Option<String>,
    #[serde(rename = "loadedAddons")]
    loaded_addons: String,
    #[serde(rename = "currentSubScene")]
    current_sub_scene: Option<i32>,
    #[serde(rename = "currentEntityLayerId")]
    current_entity_layer_id: Option<i32>,
    #[serde(rename = "activeSubsceneLayer")]
    active_subscene_layer: Option<String>,
}

#[derive(Deserialize)]
struct RawBridgeProjectContext {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    #[serde(rename = "loadedAddons", default)]
    loaded_addons: String,
}

#[derive(Deserialize)]
struct RawBridgeWorldSelection {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    #[serde(rename = "editorAvailable")]
    editor_available: Value,
    status: String,
    #[serde(rename = "selectedCount")]
    selected_count: u32,
    #[serde(rename = "selectedEntities", default)]
    selected_entities: String,
    #[serde(rename = "selectedEntitiesTruncated", default)]
    selected_entities_truncated: Value,
}

#[derive(Deserialize)]
struct RawBridgeResourceList {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    #[serde(rename = "loadedAddons", default)]
    loaded_addons: String,
    #[serde(rename = "resources", default)]
    resources: String,
    #[serde(rename = "resourceDetails", default)]
    resource_details: String,
    #[serde(rename = "hasMore", default)]
    has_more: Value,
}

#[derive(Deserialize)]
struct RawBridgeResourceInspection {
    found: bool,
    status: String,
    #[serde(rename = "resourceName")]
    resource_name: Option<String>,
    #[serde(rename = "className")]
    class_name: Option<String>,
    #[serde(rename = "sourceAddons", default)]
    source_addons: String,
    #[serde(rename = "sourceAddonsTruncated", default)]
    source_addons_truncated: bool,
}

#[derive(Deserialize)]
struct RawBridgeSelectedEntityHierarchy {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    #[serde(rename = "editorAvailable")]
    editor_available: Value,
    status: String,
    #[serde(default)]
    entity: String,
    #[serde(rename = "resourceName")]
    resource_name: Option<String>,
    #[serde(rename = "resourceReferenceKind", default)]
    resource_reference_kind: String,
    #[serde(rename = "contributorAddons", default)]
    contributor_addons: String,
    #[serde(rename = "contributorAddonsTruncated", default)]
    contributor_addons_truncated: Value,
    #[serde(default)]
    ancestors: String,
    #[serde(rename = "ancestorsTruncated", default)]
    ancestors_truncated: Value,
    #[serde(default)]
    children: String,
    #[serde(rename = "childrenTruncated", default)]
    children_truncated: Value,
    #[serde(default)]
    components: String,
    #[serde(rename = "componentProperties", default)]
    component_properties: String,
    #[serde(rename = "componentPropertiesTruncated", default)]
    component_properties_truncated: Value,
    #[serde(default)]
    properties: String,
    #[serde(rename = "propertiesTruncated", default)]
    properties_truncated: Value,
}

#[derive(Deserialize)]
struct RawBridgeEntityList {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    #[serde(rename = "worldPath", default)]
    world_path: String,
    #[serde(default)]
    entities: String,
    #[serde(rename = "hasMore", default)]
    has_more: Value,
}

#[derive(Deserialize)]
struct RawBridgeEntitySearch {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    #[serde(rename = "worldPath", default)]
    world_path: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    results: String,
    #[serde(rename = "totalMatches", default)]
    total_matches: u32,
    #[serde(rename = "namedMatches", default)]
    named_matches: u32,
    #[serde(rename = "hasMore", default)]
    has_more: Value,
    #[serde(rename = "relationTraversalTruncated", default)]
    relation_traversal_truncated: Value,
}

#[derive(Deserialize)]
struct RawBridgeLayerState {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(rename = "subScene")]
    sub_scene: i32,
    #[serde(rename = "layerId")]
    layer_id: i32,
    #[serde(rename = "layerPath", default)]
    layer_path: String,
    #[serde(default)]
    visible: Value,
    #[serde(rename = "explicitlyLocked", default)]
    explicitly_locked: Value,
    #[serde(rename = "lockedInHierarchy", default)]
    locked_in_hierarchy: Value,
}

#[derive(Deserialize)]
struct RawBridgeEntitySelection {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(rename = "activeLayerId", default)]
    active_layer_id: Option<i32>,
    #[serde(default)]
    entity: String,
    #[serde(default)]
    destination: String,
    #[serde(
        rename = "destinationExists",
        default,
        deserialize_with = "deserialize_optional_boolish"
    )]
    destination_exists: Option<bool>,
}

#[derive(Deserialize)]
struct RawBridgeShapePoints {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(default)]
    entity: String,
    #[serde(rename = "shapeClass", default)]
    shape_class: String,
    #[serde(default, deserialize_with = "deserialize_boolish")]
    closed: bool,
    #[serde(default)]
    points: String,
}

#[derive(Deserialize)]
struct RawBridgeShapeGeometry {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(default)]
    entity: String,
    #[serde(rename = "shapeClass", default)]
    shape_class: String,
    #[serde(default, deserialize_with = "deserialize_boolish")]
    closed: bool,
    #[serde(default)]
    points: String,
    #[serde(rename = "fromSpace", default)]
    from_space: String,
    #[serde(rename = "toSpace", default)]
    to_space: String,
    #[serde(rename = "spacingMeters", default)]
    spacing_meters: f32,
    #[serde(rename = "originalPointCount", default)]
    original_point_count: usize,
    #[serde(rename = "resultPointCount", default)]
    result_point_count: usize,
    #[serde(rename = "pathLength", default)]
    path_length: f32,
    #[serde(rename = "skippedZeroLengthSegments", default)]
    skipped_zero_length_segments: usize,
}

impl RawBridgeShapeGeometry {
    fn validate(self) -> Result<Self, WorkbenchFailure> {
        parse_optional_world_selection_record(&self.entity)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))?;
        parse_shape_points(&self.points).ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
        Ok(self)
    }
    fn entity(&self) -> Result<Option<WorkbenchSelectedEntity>, WorkbenchFailure> {
        parse_optional_world_selection_record(&self.entity)
            .map_err(|_| failure(WorkbenchFailureCode::Protocol))
    }
    fn points(&self) -> Result<Vec<WorkbenchEntityPosition>, WorkbenchFailure> {
        parse_shape_points(&self.points).ok_or_else(|| failure(WorkbenchFailureCode::Protocol))
    }
    fn into_shape_points(self) -> WorkbenchShapePoints {
        let entity = self.entity().expect("validated shape geometry entity");
        let points = self.points().expect("validated shape geometry points");
        WorkbenchShapePoints {
            bridge_version: self.bridge_version,
            protocol_version: self.protocol_version,
            status: self.status,
            entity,
            shape_class: (!self.shape_class.is_empty()).then_some(self.shape_class),
            closed: self.closed,
            points,
        }
    }
    fn into_conversion(self) -> WorkbenchShapePointConversion {
        let entity = self.entity().expect("validated shape geometry entity");
        let points = self.points().expect("validated shape geometry points");
        WorkbenchShapePointConversion {
            bridge_version: self.bridge_version,
            protocol_version: self.protocol_version,
            status: self.status,
            entity,
            shape_class: (!self.shape_class.is_empty()).then_some(self.shape_class),
            from_space: self.from_space,
            to_space: self.to_space,
            points,
        }
    }
    fn into_resample(self) -> WorkbenchPolylineResample {
        let entity = self.entity().expect("validated shape geometry entity");
        let points = self.points().expect("validated shape geometry points");
        WorkbenchPolylineResample {
            bridge_version: self.bridge_version,
            protocol_version: self.protocol_version,
            status: self.status,
            entity,
            shape_class: (!self.shape_class.is_empty()).then_some(self.shape_class),
            closed: self.closed,
            points,
            spacing_meters: self.spacing_meters,
            original_point_count: self.original_point_count,
            result_point_count: self.result_point_count,
            path_length: self.path_length,
            skipped_zero_length_segments: self.skipped_zero_length_segments,
        }
    }
}

#[derive(Deserialize)]
struct RawBridgePrefabResourceComponentMutation {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(rename = "resourceName")]
    resource_name: String,
    #[serde(rename = "componentIndex", default)]
    component_index: i32,
    #[serde(rename = "componentClass", default)]
    component_class: String,
    #[serde(
        rename = "templateSaved",
        default,
        deserialize_with = "deserialize_boolish"
    )]
    template_saved: bool,
}

#[derive(Deserialize)]
struct RawBridgePrefabResourcePropertyMutation {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(rename = "resourceName")]
    resource_name: String,
    #[serde(
        rename = "templateSaved",
        default,
        deserialize_with = "deserialize_boolish"
    )]
    template_saved: bool,
}

fn deserialize_boolish<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_optional_boolish(deserializer)?
        .ok_or_else(|| serde::de::Error::custom("expected templateSaved to be a boolean or 0/1"))
}

fn deserialize_optional_boolish<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(value)),
        Some(Value::Number(value)) => match value.as_i64() {
            Some(0) => Ok(Some(false)),
            Some(1) => Ok(Some(true)),
            _ => Err(serde::de::Error::custom(
                "expected destinationExists to be a boolean or 0/1",
            )),
        },
        Some(Value::String(value)) => match value.as_str() {
            "0" | "false" => Ok(Some(false)),
            "1" | "true" => Ok(Some(true)),
            _ => Err(serde::de::Error::custom(
                "expected destinationExists to be a boolean or 0/1",
            )),
        },
        Some(_) => Err(serde::de::Error::custom(
            "expected destinationExists to be a boolean or 0/1",
        )),
    }
}

#[derive(Deserialize)]
struct RawBridgeEntityRadiusQuery {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(rename = "centerX")]
    center_x: f32,
    #[serde(rename = "centerY")]
    center_y: f32,
    #[serde(rename = "centerZ")]
    center_z: f32,
    #[serde(rename = "radiusMeters")]
    radius_meters: f32,
    #[serde(rename = "queryScope")]
    query_scope: String,
    #[serde(rename = "requireObject")]
    require_object: Value,
    #[serde(rename = "excludeProxies")]
    exclude_proxies: Value,
    #[serde(default)]
    entities: String,
    #[serde(default)]
    truncated: Value,
}

#[derive(Deserialize)]
struct RawBridgeTerrainSample {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(rename = "halfExtentMeters", default)]
    half_extent_meters: f32,
    #[serde(rename = "requestedSpacingMeters", default)]
    requested_spacing_meters: f32,
    #[serde(rename = "effectiveSpacingMeters", default)]
    effective_spacing_meters: f32,
    #[serde(rename = "spacingClamped", default)]
    spacing_clamped: Value,
    #[serde(rename = "gridOriginX", default)]
    grid_origin_x: f32,
    #[serde(rename = "gridOriginZ", default)]
    grid_origin_z: f32,
    #[serde(rename = "gridWidth", default)]
    grid_width: u32,
    #[serde(rename = "gridHeight", default)]
    grid_height: u32,
    #[serde(default)]
    heights: String,
    #[serde(rename = "waterTypes", default)]
    water_types: String,
    #[serde(rename = "waterSurfaceHeights", default)]
    water_surface_heights: String,
    #[serde(rename = "waterDepthsAboveTerrain", default)]
    water_depths_above_terrain: String,
    #[serde(rename = "boundsMinX", default)]
    bounds_min_x: f32,
    #[serde(rename = "boundsMinY", default)]
    bounds_min_y: f32,
    #[serde(rename = "boundsMinZ", default)]
    bounds_min_z: f32,
    #[serde(rename = "boundsMaxX", default)]
    bounds_max_x: f32,
    #[serde(rename = "boundsMaxY", default)]
    bounds_max_y: f32,
    #[serde(rename = "boundsMaxZ", default)]
    bounds_max_z: f32,
    #[serde(rename = "heightmapResolutionX", default)]
    heightmap_resolution_x: u32,
    #[serde(rename = "heightmapResolutionZ", default)]
    heightmap_resolution_z: u32,
    #[serde(rename = "nativeSpacingMeters", default)]
    native_spacing_meters: f32,
    #[serde(rename = "tileCountX", default)]
    tile_count_x: u32,
    #[serde(rename = "tileCountZ", default)]
    tile_count_z: u32,
}

#[derive(Deserialize)]
struct RawBridgeViewportContext {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(default)]
    width: i32,
    #[serde(default)]
    height: i32,
    #[serde(rename = "mouseX", default)]
    mouse_x: i32,
    #[serde(rename = "mouseY", default)]
    mouse_y: i32,
    #[serde(rename = "mouseInside", default)]
    mouse_inside: Value,
    #[serde(rename = "cameraX", default)]
    camera_x: f32,
    #[serde(rename = "cameraY", default)]
    camera_y: f32,
    #[serde(rename = "cameraZ", default)]
    camera_z: f32,
    #[serde(rename = "cameraDirectionX", default)]
    camera_direction_x: f32,
    #[serde(rename = "cameraDirectionY", default)]
    camera_direction_y: f32,
    #[serde(rename = "cameraDirectionZ", default)]
    camera_direction_z: f32,
    #[serde(rename = "startX", default)]
    start_x: f32,
    #[serde(rename = "startY", default)]
    start_y: f32,
    #[serde(rename = "startZ", default)]
    start_z: f32,
    #[serde(rename = "endX", default)]
    end_x: f32,
    #[serde(rename = "endY", default)]
    end_y: f32,
    #[serde(rename = "endZ", default)]
    end_z: f32,
    #[serde(rename = "directionX", default)]
    direction_x: f32,
    #[serde(rename = "directionY", default)]
    direction_y: f32,
    #[serde(rename = "directionZ", default)]
    direction_z: f32,
}
#[derive(Deserialize)]
struct RawBridgeTrace {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(default)]
    hit: Value,
    #[serde(default)]
    fraction: f32,
    #[serde(default)]
    distance: f32,
    #[serde(rename = "hitX", default)]
    hit_x: f32,
    #[serde(rename = "hitY", default)]
    hit_y: f32,
    #[serde(rename = "hitZ", default)]
    hit_z: f32,
    #[serde(rename = "normalX", default)]
    normal_x: f32,
    #[serde(rename = "normalY", default)]
    normal_y: f32,
    #[serde(rename = "normalZ", default)]
    normal_z: f32,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    entity: String,
    #[serde(rename = "colliderName", default)]
    collider_name: String,
    #[serde(default)]
    material: String,
}

#[derive(Deserialize)]
struct RawBridgeComponentResult {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(default)]
    entity: String,
    #[serde(default)]
    components: String,
    #[serde(default)]
    properties: String,
}
#[derive(Deserialize)]
struct RawBridgePropertyList {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(default)]
    properties: String,
}

#[derive(Deserialize)]
struct RawBridgePrefabContext {
    #[serde(rename = "bridgeVersion")]
    bridge_version: String,
    #[serde(rename = "protocolVersion")]
    protocol_version: u32,
    status: String,
    #[serde(default)]
    entity: String,
    #[serde(rename = "memberId", default)]
    member_id: String,
    #[serde(rename = "resourceName", default)]
    resource_name: String,
    #[serde(rename = "resourceReferenceKind", default)]
    resource_reference_kind: String,
    #[serde(rename = "contributorAddons", default)]
    contributor_addons: String,
    #[serde(rename = "ancestorResources", default)]
    ancestor_resources: String,
    #[serde(rename = "ancestorResourcesTruncated", default)]
    ancestor_resources_truncated: Value,
    #[serde(rename = "prefabEditMode", default)]
    prefab_edit_mode: Value,
    #[serde(default)]
    components: String,
    #[serde(rename = "componentProperties", default)]
    component_properties: String,
    #[serde(default)]
    children: String,
    #[serde(rename = "childrenTruncated", default)]
    children_truncated: Value,
    #[serde(default)]
    properties: String,
    #[serde(rename = "propertiesTruncated", default)]
    properties_truncated: Value,
    #[serde(rename = "childCount", default)]
    child_count: u32,
}

fn workbench_bool(value: &Value) -> bool {
    matches!(value, Value::Bool(true)) || value.as_i64().is_some_and(|integer| integer != 0)
}

fn parse_terrain_metadata(raw: &RawBridgeTerrainSample) -> Result<WorkbenchTerrainMetadata, ()> {
    if !raw.bounds_min_x.is_finite()
        || !raw.bounds_min_y.is_finite()
        || !raw.bounds_min_z.is_finite()
        || !raw.bounds_max_x.is_finite()
        || !raw.bounds_max_y.is_finite()
        || !raw.bounds_max_z.is_finite()
        || raw.bounds_min_x > raw.bounds_max_x
        || raw.bounds_min_y > raw.bounds_max_y
        || raw.bounds_min_z > raw.bounds_max_z
        || raw.heightmap_resolution_x == 0
        || raw.heightmap_resolution_z == 0
        || !raw.native_spacing_meters.is_finite()
        || raw.native_spacing_meters <= 0.0
        || raw.tile_count_x == 0
        || raw.tile_count_z == 0
    {
        return Err(());
    }
    Ok(WorkbenchTerrainMetadata {
        bounds: WorkbenchTerrainBounds {
            min: WorkbenchEntityPosition {
                x: raw.bounds_min_x,
                y: raw.bounds_min_y,
                z: raw.bounds_min_z,
            },
            max: WorkbenchEntityPosition {
                x: raw.bounds_max_x,
                y: raw.bounds_max_y,
                z: raw.bounds_max_z,
            },
        },
        heightmap_resolution_x: raw.heightmap_resolution_x,
        heightmap_resolution_z: raw.heightmap_resolution_z,
        native_spacing_meters: raw.native_spacing_meters,
        tile_count_x: raw.tile_count_x,
        tile_count_z: raw.tile_count_z,
    })
}

fn parse_terrain_grid(
    raw: &RawBridgeTerrainSample,
    requested_spacing_meters: Option<f32>,
    max_samples: usize,
) -> Result<WorkbenchTerrainGrid, ()> {
    let sample_count = (raw.grid_width as usize).saturating_mul(raw.grid_height as usize);
    if raw.grid_width == 0
        || raw.grid_height == 0
        || sample_count > max_samples
        || !raw.half_extent_meters.is_finite()
        || raw.half_extent_meters <= 0.0
        || !raw.requested_spacing_meters.is_finite()
        || !raw.effective_spacing_meters.is_finite()
        || raw.effective_spacing_meters <= 0.0
        || !raw.grid_origin_x.is_finite()
        || !raw.grid_origin_z.is_finite()
    {
        return Err(());
    }
    let heights = raw
        .heights
        .split(';')
        .map(|value| {
            if value == "~" {
                Ok(None)
            } else {
                value
                    .parse::<f32>()
                    .ok()
                    .filter(|height| height.is_finite())
                    .map(Some)
                    .ok_or(())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if heights.len() != sample_count {
        return Err(());
    }
    Ok(WorkbenchTerrainGrid {
        origin: WorkbenchTerrainCoordinate {
            x: raw.grid_origin_x,
            z: raw.grid_origin_z,
        },
        requested_half_extent_meters: raw.half_extent_meters,
        requested_spacing_meters,
        effective_spacing_meters: raw.effective_spacing_meters,
        spacing_clamped: workbench_bool(&raw.spacing_clamped),
        width: raw.grid_width,
        height: raw.grid_height,
        heights,
    })
}

fn summarize_terrain_grid(grid: &WorkbenchTerrainGrid) -> WorkbenchTerrainSummary {
    let valid_heights = grid.heights.iter().flatten().copied().collect::<Vec<_>>();
    let valid_sample_count = valid_heights.len() as u32;
    let minimum_height = valid_heights.iter().copied().reduce(f32::min);
    let maximum_height = valid_heights.iter().copied().reduce(f32::max);
    let mean_height = (!valid_heights.is_empty())
        .then(|| valid_heights.iter().sum::<f32>() / valid_heights.len() as f32);
    let elevation_range = minimum_height
        .zip(maximum_height)
        .map(|(min, max)| max - min);
    let mut steepest: Option<(f32, WorkbenchTerrainCoordinate)> = None;
    for z in 0..grid.height as usize {
        for x in 0..grid.width as usize {
            let index = z * grid.width as usize + x;
            let Some(height) = grid.heights[index] else {
                continue;
            };
            for (adjacent_x, adjacent_z) in [(x + 1, z), (x, z + 1)] {
                if adjacent_x >= grid.width as usize || adjacent_z >= grid.height as usize {
                    continue;
                }
                let adjacent_index = adjacent_z * grid.width as usize + adjacent_x;
                let Some(adjacent_height) = grid.heights[adjacent_index] else {
                    continue;
                };
                let slope_degrees = ((adjacent_height - height).abs()
                    / grid.effective_spacing_meters)
                    .atan()
                    .to_degrees();
                if steepest
                    .as_ref()
                    .is_none_or(|(current, _)| slope_degrees > *current)
                {
                    steepest = Some((
                        slope_degrees,
                        WorkbenchTerrainCoordinate {
                            x: grid.origin.x + adjacent_x as f32 * grid.effective_spacing_meters,
                            z: grid.origin.z + adjacent_z as f32 * grid.effective_spacing_meters,
                        },
                    ));
                }
            }
        }
    }
    WorkbenchTerrainSummary {
        valid_sample_count,
        minimum_height,
        maximum_height,
        mean_height,
        elevation_range,
        steepest_adjacent_slope_degrees: steepest.as_ref().map(|(slope, _)| *slope),
        steepest_adjacent_slope_position: steepest.map(|(_, position)| position),
    }
}

fn parse_terrain_water_grid(
    raw: &RawBridgeTerrainSample,
    sample_count: usize,
) -> Result<WorkbenchTerrainWaterGrid, ()> {
    let types = raw
        .water_types
        .split(';')
        .map(|value| match value {
            "~" => Ok(None),
            "n" => Ok(Some(WorkbenchTerrainWaterType::None)),
            "o" => Ok(Some(WorkbenchTerrainWaterType::Ocean)),
            "p" => Ok(Some(WorkbenchTerrainWaterType::Pond)),
            "r" => Ok(Some(WorkbenchTerrainWaterType::River)),
            _ => Err(()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let values = |encoded: &str| {
        encoded
            .split(';')
            .map(|value| {
                if value == "~" {
                    Ok(None)
                } else {
                    value
                        .parse::<f32>()
                        .ok()
                        .filter(|value| value.is_finite())
                        .map(Some)
                        .ok_or(())
                }
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let surface_heights = values(&raw.water_surface_heights)?;
    let depths_above_terrain = values(&raw.water_depths_above_terrain)?;
    if types.len() != sample_count
        || surface_heights.len() != sample_count
        || depths_above_terrain.len() != sample_count
    {
        return Err(());
    }
    Ok(WorkbenchTerrainWaterGrid {
        types,
        surface_heights,
        depths_above_terrain,
    })
}

fn summarize_terrain_water_grid(grid: &WorkbenchTerrainWaterGrid) -> WorkbenchTerrainWaterSummary {
    let mut summary = WorkbenchTerrainWaterSummary {
        wet_sample_count: 0,
        ocean_sample_count: 0,
        pond_sample_count: 0,
        river_sample_count: 0,
        maximum_depth_above_terrain: None,
    };
    for (kind, depth) in grid.types.iter().zip(&grid.depths_above_terrain) {
        match kind {
            Some(WorkbenchTerrainWaterType::Ocean) => summary.ocean_sample_count += 1,
            Some(WorkbenchTerrainWaterType::Pond) => summary.pond_sample_count += 1,
            Some(WorkbenchTerrainWaterType::River) => summary.river_sample_count += 1,
            _ => continue,
        }
        summary.wet_sample_count += 1;
        if let Some(depth) = depth {
            summary.maximum_depth_above_terrain = Some(
                summary
                    .maximum_depth_above_terrain
                    .map_or(*depth, |maximum| maximum.max(*depth)),
            );
        }
    }
    summary
}

fn play_session(
    reported: &Option<String>,
    world_editor_module_present: bool,
    world_editor_api_available: bool,
) -> Option<WorkbenchPlaySession> {
    match reported.as_deref() {
        Some("unavailable") => Some(WorkbenchPlaySession::Unavailable),
        Some("editing") | Some("unknown") => Some(WorkbenchPlaySession::Unknown),
        Some("likely-running") => Some(WorkbenchPlaySession::LikelyRunning),
        Some(_) => None,
        None if world_editor_api_available => Some(WorkbenchPlaySession::Unknown),
        None if world_editor_module_present => Some(WorkbenchPlaySession::LikelyRunning),
        None => Some(WorkbenchPlaySession::Unavailable),
    }
}

fn canonical_resource_name(value: &str) -> bool {
    let Some((guid, path)) = value
        .strip_prefix('{')
        .and_then(|value| value.split_once('}'))
    else {
        return false;
    };
    guid.len() == 16
        && guid.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !path.is_empty()
        && !path.contains("..")
        && !path.contains('\\')
        && !path.contains(':')
}

fn canonical_component_class_name(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(character) if character.is_ascii_alphabetic() || character == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
        && value.len() <= 128
}

fn parse_prefab_component_id(value: &str) -> Option<(i32, String)> {
    let mut fields = value.splitn(3, ':');
    (fields.next()? == "cmp1").then_some(())?;
    let index = fields.next()?.parse::<i32>().ok()?;
    let class_name = fields.next()?;
    (index >= 0 && canonical_component_class_name(class_name))
        .then(|| (index, class_name.to_string()))
}

fn parse_world_selection_records(value: &str) -> Result<Vec<WorkbenchSelectedEntity>, ()> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(';')
        .map(|record| {
            let mut fields = record.split('|');
            let entity_id = fields.next().filter(|value| !value.is_empty()).ok_or(())?;
            let class_name = fields.next().filter(|value| !value.is_empty()).ok_or(())?;
            let sub_scene = fields.next().ok_or(())?.parse::<i32>().map_err(|_| ())?;
            let layer_id = fields.next().ok_or(())?.parse::<i32>().map_err(|_| ())?;
            let position = match fields.next() {
                None => None,
                Some(x) => {
                    let y = fields.next().ok_or(())?;
                    let z = fields.next().ok_or(())?;
                    let x = x.parse::<f32>().map_err(|_| ())?;
                    let y = y.parse::<f32>().map_err(|_| ())?;
                    let z = z.parse::<f32>().map_err(|_| ())?;
                    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                        return Err(());
                    }
                    Some(WorkbenchEntityPosition {
                        x: round_world_coordinate(x),
                        y: round_world_coordinate(y),
                        z: round_world_coordinate(z),
                    })
                }
            };
            let resource_name = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let name = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let sub_scene_name = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let layer_name = fields
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            if fields.next().is_some()
                || entity_id.len() > 128
                || class_name.len() > 256
                || [
                    resource_name.as_deref(),
                    name.as_deref(),
                    sub_scene_name.as_deref(),
                    layer_name.as_deref(),
                ]
                .into_iter()
                .flatten()
                .any(|value| {
                    value.len() > 1024 || value.contains(|character: char| character.is_control())
                })
                || entity_id.contains(|character: char| character.is_control())
                || class_name.contains(|character: char| character.is_control())
            {
                return Err(());
            }
            Ok(WorkbenchSelectedEntity {
                entity_id: entity_id.to_string(),
                class_name: class_name.to_string(),
                sub_scene,
                layer_id,
                resource_name,
                name,
                sub_scene_name,
                layer_name,
                position,
            })
        })
        .collect()
}

fn parse_entity_search_records(value: &str) -> Result<Vec<WorkbenchEntitySearchHit>, ()> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(';')
        .map(|record| {
            let fields: Vec<_> = record.split('|').collect();
            if fields.len() != 18 || fields[..4].iter().any(|field| field.is_empty()) {
                return Err(());
            }
            let component_classes = if fields[6].is_empty() {
                Vec::new()
            } else {
                fields[6].split(',').map(str::to_owned).collect()
            };
            let matched_fields = if fields[7].is_empty() {
                Vec::new()
            } else {
                fields[7].split(',').map(str::to_owned).collect()
            };
            let matched_component_classes = if fields[8].is_empty() {
                Vec::new()
            } else {
                fields[8].split(',').map(str::to_owned).collect()
            };
            let relation_match = match (
                fields[11], fields[12], fields[13], fields[14], fields[15], fields[16], fields[17],
            ) {
                ("", "", "", "", "", "", "") => None,
                (direction, depth, entity_id, class_name, sub_scene, layer_id, components) => {
                    let direction = match direction {
                        "parent" => WorkbenchEntityRelationDirection::Parent,
                        "ancestor" => WorkbenchEntityRelationDirection::Ancestor,
                        "child" => WorkbenchEntityRelationDirection::Child,
                        "descendant" => WorkbenchEntityRelationDirection::Descendant,
                        _ => return Err(()),
                    };
                    let depth = depth.parse::<i32>().map_err(|_| ())?;
                    if !(1..=8).contains(&depth)
                        || entity_id.is_empty()
                        || entity_id.len() > 256
                        || entity_id
                            .bytes()
                            .any(|byte| matches!(byte, b'|' | b';' | b',' | b'\r' | b'\n'))
                        || !valid_component_class_name(class_name)
                    {
                        return Err(());
                    }
                    let matched_component_classes = if components.is_empty() {
                        Vec::new()
                    } else {
                        components.split(',').map(str::to_owned).collect()
                    };
                    if matched_component_classes.len() > 32
                        || matched_component_classes
                            .iter()
                            .any(|class_name| !valid_component_class_name(class_name))
                    {
                        return Err(());
                    }
                    Some(WorkbenchEntityRelationMatch {
                        direction,
                        depth,
                        entity_id: entity_id.to_owned(),
                        class_name: class_name.to_owned(),
                        sub_scene: sub_scene.parse().map_err(|_| ())?,
                        layer_id: layer_id.parse().map_err(|_| ())?,
                        matched_component_classes,
                    })
                }
            };
            Ok(WorkbenchEntitySearchHit {
                entity: WorkbenchSelectedEntity {
                    entity_id: fields[0].to_owned(),
                    class_name: fields[1].to_owned(),
                    sub_scene: fields[2].parse().map_err(|_| ())?,
                    layer_id: fields[3].parse().map_err(|_| ())?,
                    resource_name: (!fields[4].is_empty()).then(|| fields[4].to_owned()),
                    name: (!fields[5].is_empty()).then(|| fields[5].to_owned()),
                    sub_scene_name: None,
                    layer_name: None,
                    position: None,
                },
                component_classes,
                matched_component_classes,
                matched_fields,
                parent_class_name: (!fields[9].is_empty()).then(|| fields[9].to_owned()),
                child_count: fields[10].parse().map_err(|_| ())?,
                relation_match,
            })
        })
        .collect()
}

fn valid_component_class_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn round_world_coordinate(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn is_false(value: &bool) -> bool {
    !value
}

fn is_available_status(value: &String) -> bool {
    value == "available"
}

fn parse_components(value: &str) -> Result<Vec<WorkbenchComponent>, ()> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(';')
        .map(|record| {
            let mut fields = record.split('|');
            let index = fields.next().ok_or(())?.parse::<u32>().map_err(|_| ())?;
            let class_name = fields
                .next()
                .filter(|value| !value.is_empty())
                .ok_or(())?
                .to_string();
            if fields.next().is_some() {
                return Err(());
            }
            Ok(WorkbenchComponent {
                component_id: format!("cmp1:{index}:{class_name}"),
                class_name,
                property_count: None,
                direct_override_count: None,
            })
        })
        .collect()
}

fn parse_prefab_components(
    components: &str,
    component_properties: &str,
) -> Result<Vec<WorkbenchPrefabComponent>, ()> {
    let mut properties_by_component =
        std::collections::BTreeMap::<u32, Vec<WorkbenchPrefabProperty>>::new();
    for record in component_properties
        .split(';')
        .filter(|record| !record.is_empty())
    {
        let mut fields = record.split('|');
        let index = fields.next().ok_or(())?.parse::<u32>().map_err(|_| ())?;
        let path = fields.next().filter(|value| !value.is_empty()).ok_or(())?;
        let data_type = fields.next().ok_or(())?;
        let value = fields.next().ok_or(())?;
        let directly_overridden = fields.next().ok_or(())? == "1";
        let value_origin = fields
            .next()
            .map(parse_prefab_property_origin)
            .transpose()?;
        if fields.next().is_some() {
            return Err(());
        }
        properties_by_component
            .entry(index)
            .or_default()
            .push(WorkbenchPrefabProperty {
                path: path.to_string(),
                data_type: data_type.to_string(),
                value: normalized_property_value(data_type, value),
                directly_overridden,
                value_origin,
                write_descriptor: None,
            });
    }

    if components.is_empty() {
        return Ok(Vec::new());
    }
    components
        .split(';')
        .map(|record| {
            let mut fields = record.split('|');
            let index = fields.next().ok_or(())?.parse::<u32>().map_err(|_| ())?;
            let class_name = fields
                .next()
                .filter(|value| !value.is_empty())
                .ok_or(())?
                .to_string();
            if fields.next().is_some() {
                return Err(());
            }
            Ok(WorkbenchPrefabComponent {
                component_id: format!("cmp1:{index}:{class_name}"),
                class_name,
                properties: properties_by_component.remove(&index).unwrap_or_default(),
            })
        })
        .collect()
}

fn parse_prefab_property_origin(value: &str) -> Result<WorkbenchPrefabPropertyOrigin, ()> {
    match value {
        "direct" => Ok(WorkbenchPrefabPropertyOrigin::Direct),
        "inherited" => Ok(WorkbenchPrefabPropertyOrigin::Inherited),
        "default" => Ok(WorkbenchPrefabPropertyOrigin::Default),
        _ => Err(()),
    }
}

fn parse_component_summaries(
    components: &str,
    component_properties: &str,
) -> Result<Vec<WorkbenchComponent>, ()> {
    parse_prefab_components(components, component_properties).map(|components| {
        components
            .into_iter()
            .map(|component| WorkbenchComponent {
                component_id: component.component_id,
                class_name: component.class_name,
                property_count: Some(component.properties.len() as u32),
                direct_override_count: Some(
                    component
                        .properties
                        .iter()
                        .filter(|property| property.directly_overridden)
                        .count() as u32,
                ),
            })
            .collect()
    })
}

fn is_native_component_descriptor(value: &str) -> bool {
    let mut fields = value.splitn(3, ':');
    matches!(fields.next(), Some("cmp1"))
        && fields
            .next()
            .is_some_and(|index| index.parse::<u32>().is_ok())
        && fields
            .next()
            .is_some_and(|class_name| !class_name.is_empty())
}

fn parse_properties(value: &str) -> Result<Vec<WorkbenchDirectProperty>, ()> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(';')
        .map(|record| {
            let mut fields = record.split('|');
            let name = fields
                .next()
                .filter(|value| !value.is_empty())
                .ok_or(())?
                .to_string();
            let data_type = fields.next().ok_or(())?.to_string();
            let raw_value = fields.next().ok_or(())?.to_string();
            let directly_overridden = fields
                .next()
                .map(|value| match value {
                    "0" => Ok(false),
                    "1" => Ok(true),
                    _ => Err(()),
                })
                .transpose()?;
            let value_origin = fields
                .next()
                .map(parse_prefab_property_origin)
                .transpose()?;
            if fields.next().is_some() {
                return Err(());
            }
            Ok(WorkbenchDirectProperty {
                name,
                value: normalized_property_value(&data_type, &raw_value),
                data_type,
                directly_overridden,
                value_origin,
                write_descriptor: None,
            })
        })
        .collect()
}

fn split_bounded_records(value: &str) -> Vec<String> {
    value
        .split(';')
        .filter(|record| !record.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_prefab_members(
    value: &str,
    parent_member_id: &str,
) -> Result<Vec<WorkbenchPrefabMember>, ()> {
    value
        .split(';')
        .filter(|record| !record.is_empty())
        .map(|record| {
            let mut fields = record.split('|');
            let index = fields.next().ok_or(())?.parse::<u32>().map_err(|_| ())?;
            let class_name = fields.next().filter(|value| !value.is_empty()).ok_or(())?;
            let name = fields.next().ok_or(())?;
            if fields.next().is_some()
                || class_name.len() > 256
                || name.len() > 1024
                || class_name.contains(|character: char| character.is_control())
                || name.contains(|character: char| character.is_control())
            {
                return Err(());
            }
            let member_id = format!("member:{index}");
            Ok(WorkbenchPrefabMember {
                member_id: if parent_member_id.is_empty() {
                    member_id
                } else {
                    format!("{parent_member_id}/{member_id}")
                },
                class_name: class_name.to_string(),
                name: (!name.is_empty()).then_some(name.to_string()),
            })
        })
        .collect()
}

fn parse_prefab_properties(value: &str) -> Result<Vec<WorkbenchPrefabProperty>, ()> {
    let mut properties = value
        .split(';')
        .filter(|record| !record.is_empty())
        .map(|record| {
            let mut fields = record.split('|');
            let path = fields.next().ok_or(())?;
            let data_type = fields.next().ok_or(())?;
            let value = fields.next().ok_or(())?;
            let directly_overridden = fields.next().ok_or(())? == "1";
            let value_origin = fields
                .next()
                .map(parse_prefab_property_origin)
                .transpose()?;
            if fields.next().is_some() {
                return Err(());
            }
            Ok(WorkbenchPrefabProperty {
                path: path.to_string(),
                data_type: data_type.to_string(),
                value: normalized_property_value(data_type, value),
                directly_overridden,
                value_origin,
                write_descriptor: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    properties.retain(|property| !is_engine_owned_prefab_property(&property.path));
    Ok(properties)
}

fn is_engine_owned_prefab_property(path: &str) -> bool {
    matches!(path, "userScript" | "constructor" | "destructor")
}

#[derive(Clone, Copy)]
enum PropertyValueKind {
    Bool,
    Integer,
    Float,
    Vector,
    String,
}

fn property_value_kind(data_type: &str) -> Option<PropertyValueKind> {
    match data_type.trim().to_ascii_lowercase().as_str() {
        "bool" => Some(PropertyValueKind::Bool),
        "int" | "integer" | "enum" => Some(PropertyValueKind::Integer),
        "float" => Some(PropertyValueKind::Float),
        "vector" => Some(PropertyValueKind::Vector),
        "string" | "resource" | "entity" => Some(PropertyValueKind::String),
        _ => None,
    }
}

fn supported_property_type(data_type: &str) -> bool {
    property_value_kind(data_type).is_some()
}

fn normalized_property_value(data_type: &str, raw: &str) -> Value {
    if matches!(
        property_value_kind(data_type),
        Some(PropertyValueKind::Bool)
    ) {
        return Value::Bool(matches!(raw, "1" | "true" | "True"));
    }
    if matches!(
        property_value_kind(data_type),
        Some(PropertyValueKind::Integer)
    ) {
        if let Ok(value) = raw.parse::<i64>() {
            return Value::Number(value.into());
        }
    }
    if matches!(
        property_value_kind(data_type),
        Some(PropertyValueKind::Float)
    ) {
        if let Ok(value) = raw.parse::<f64>() {
            if let Some(value) = serde_json::Number::from_f64(value) {
                return Value::Number(value);
            }
        }
    }
    if matches!(
        property_value_kind(data_type),
        Some(PropertyValueKind::Vector)
    ) {
        let values = raw
            .split_whitespace()
            .filter_map(|part| part.parse::<f64>().ok())
            .collect::<Vec<_>>();
        if values.len() == 3 && values.iter().all(|value| value.is_finite()) {
            return json!({"x": values[0], "y": values[1], "z": values[2]});
        }
    }
    Value::String(raw.to_string())
}

fn property_value_wire_format(data_type: &str, value: &Value) -> Option<String> {
    if matches!(
        property_value_kind(data_type),
        Some(PropertyValueKind::Bool)
    ) {
        return value.as_bool().map(|value| {
            if value {
                "1".to_string()
            } else {
                "0".to_string()
            }
        });
    }
    if matches!(
        property_value_kind(data_type),
        Some(PropertyValueKind::Integer)
    ) {
        return value.as_i64().map(|value| value.to_string());
    }
    if matches!(
        property_value_kind(data_type),
        Some(PropertyValueKind::Float)
    ) {
        return value
            .as_f64()
            .filter(|value| value.is_finite())
            .map(|value| value.to_string());
    }
    if matches!(
        property_value_kind(data_type),
        Some(PropertyValueKind::Vector)
    ) {
        let object = value.as_object()?;
        let component = |name| {
            object
                .get(name)
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite())
        };
        let x = component("x")?;
        let y = component("y")?;
        let z = component("z")?;
        return Some(format!("{x} {y} {z}"));
    }
    if matches!(
        property_value_kind(data_type),
        Some(PropertyValueKind::String)
    ) {
        return value
            .as_str()
            .filter(|value| value.len() <= 1024 && !value.contains(['|', ';', '\0']))
            .map(str::to_string);
    }
    None
}

fn invalid_property_descriptor_result() -> WorkbenchEntityMutationResult {
    WorkbenchEntityMutationResult {
        bridge_version: WORKBENCH_BRIDGE_VERSION.to_string(),
        protocol_version: WORKBENCH_BRIDGE_PROTOCOL_VERSION,
        status: "invalid-property-descriptor".to_string(),
        active_layer_id: None,
        entity: None,
        confirmation_token: None,
        destination: None,
        destination_exists: None,
        resource_name: None,
        persistence_path: None,
        template_saved: None,
        inspection: None,
    }
}

fn invalid_confirmation_result() -> WorkbenchEntityMutationResult {
    WorkbenchEntityMutationResult {
        bridge_version: WORKBENCH_BRIDGE_VERSION.to_string(),
        protocol_version: WORKBENCH_BRIDGE_PROTOCOL_VERSION,
        status: "invalid-confirmation".to_string(),
        active_layer_id: None,
        entity: None,
        confirmation_token: None,
        destination: None,
        destination_exists: None,
        resource_name: None,
        persistence_path: None,
        template_saved: None,
        inspection: None,
    }
}

fn invalid_prefab_resource_mutation_result(
    resource_name: &str,
) -> WorkbenchPrefabResourceMutationResult {
    WorkbenchPrefabResourceMutationResult {
        bridge_version: WORKBENCH_BRIDGE_VERSION.to_string(),
        protocol_version: WORKBENCH_BRIDGE_PROTOCOL_VERSION,
        status: "invalid-confirmation".to_string(),
        resource_name: resource_name.to_string(),
        persistence_path: "workbench-resource".to_string(),
        component_id: None,
        component_class: None,
        template_saved: false,
        inspection: None,
        component_inspection: None,
        confirmation_token: None,
    }
}

fn invalid_component_property_descriptor_result() -> WorkbenchComponentResult {
    WorkbenchComponentResult {
        bridge_version: WORKBENCH_BRIDGE_VERSION.to_string(),
        protocol_version: WORKBENCH_BRIDGE_PROTOCOL_VERSION,
        status: "invalid-property-descriptor".to_string(),
        entity: None,
        components: Vec::new(),
        properties: Vec::new(),
        confirmation_token: None,
    }
}

fn invalid_component_descriptor_result() -> WorkbenchComponentResult {
    WorkbenchComponentResult {
        bridge_version: WORKBENCH_BRIDGE_VERSION.to_string(),
        protocol_version: WORKBENCH_BRIDGE_PROTOCOL_VERSION,
        status: "invalid-component-descriptor".to_string(),
        entity: None,
        components: Vec::new(),
        properties: Vec::new(),
        confirmation_token: None,
    }
}

fn parse_optional_world_selection_record(
    value: &str,
) -> Result<Option<WorkbenchSelectedEntity>, ()> {
    if value.is_empty() {
        return Ok(None);
    }
    let mut records = parse_world_selection_records(value)?;
    if records.len() != 1 {
        return Err(());
    }
    Ok(records.pop())
}

fn parse_shape_points(value: &str) -> Option<Vec<WorkbenchEntityPosition>> {
    if value.is_empty() {
        return Some(Vec::new());
    }
    value
        .split(';')
        .map(|record| {
            let fields = record.split('|').collect::<Vec<_>>();
            if fields.len() != 3 {
                return None;
            }
            let x = fields[0].parse::<f32>().ok()?;
            let y = fields[1].parse::<f32>().ok()?;
            let z = fields[2].parse::<f32>().ok()?;
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                return None;
            }
            Some(WorkbenchEntityPosition { x, y, z })
        })
        .collect()
}

fn encode_shape_points(points: &[WorkbenchEntityPosition]) -> String {
    points
        .iter()
        .map(|point| format!("{},{},{}", point.x, point.y, point.z))
        .collect::<Vec<_>>()
        .join(";")
}

fn shape_point_space_name(space: WorkbenchShapePointSpace) -> &'static str {
    match space {
        WorkbenchShapePointSpace::Local => "local",
        WorkbenchShapePointSpace::World => "world",
    }
}

fn shape_transform_operation_name(operation: WorkbenchShapeTransformOperation) -> &'static str {
    match operation {
        WorkbenchShapeTransformOperation::Translate => "translate",
        WorkbenchShapeTransformOperation::RotateXz => "rotateXZ",
        WorkbenchShapeTransformOperation::Scale => "scale",
        WorkbenchShapeTransformOperation::Mirror => "mirror",
        WorkbenchShapeTransformOperation::Reverse => "reverse",
    }
}

struct ResolvedWorkbenchPaths {
    workbench_root: PathBuf,
    profile: PathBuf,
    bridge_directory: PathBuf,
    legacy_bridge_directory: PathBuf,
    game: Option<PathBuf>,
    game_source: String,
    tools: Option<PathBuf>,
    tools_source: String,
    executable: Option<PathBuf>,
    executable_source: String,
}

fn path_status(path: Option<PathBuf>, source: &str) -> WorkbenchPathStatus {
    let exists = path.as_ref().is_some_and(|value| value.exists());
    WorkbenchPathStatus {
        path,
        exists,
        source: source.to_string(),
    }
}

fn is_workbench_executable(path: &std::path::Path) -> bool {
    path.is_file()
        && path.file_name().is_some_and(|name| {
            name.to_string_lossy()
                .eq_ignore_ascii_case("ArmaReforgerWorkbenchSteamDiag.exe")
        })
}

fn manifest_matches_payload(manifest: &BridgeManifest) -> bool {
    manifest_matches_payload_for(manifest, bridge_payload())
}

fn manifest_matches_payload_for(manifest: &BridgeManifest, payload: &[(&str, &str)]) -> bool {
    manifest.files.len() == payload.len()
        && payload.iter().all(|(name, content)| {
            let expected_hash = sha256(content.as_bytes());
            manifest
                .files
                .iter()
                .any(|file| file.name == *name && file.sha256 == expected_hash)
        })
}

/// Formats generated Enforce handlers before they are installed.  Templates may
/// be compact, but the files a Workbench user debugs must be human-readable.
fn format_bridge_source(source: &str) -> String {
    let mut output = String::with_capacity(source.len() * 2);
    let mut indent = 0usize;
    let mut parens = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut at_line_start = true;
    let mut preprocessor = false;
    let write = |text: &str, output: &mut String, at_line_start: &mut bool, indent: usize| {
        if *at_line_start && !text.trim().is_empty() {
            output.push_str(&"\t".repeat(indent));
            *at_line_start = false;
        }
        output.push_str(text);
    };
    for character in source.chars() {
        if preprocessor {
            if character == '\n' || character == '\r' {
                output.push('\n');
                at_line_start = true;
                preprocessor = false;
            } else {
                write(
                    &character.to_string(),
                    &mut output,
                    &mut at_line_start,
                    indent,
                );
            }
            continue;
        }
        if quoted {
            write(
                &character.to_string(),
                &mut output,
                &mut at_line_start,
                indent,
            );
            if character == '"' && !escaped {
                quoted = false;
            }
            escaped = character == '\\' && !escaped;
            continue;
        }
        match character {
            '#' if at_line_start => {
                preprocessor = true;
                write("#", &mut output, &mut at_line_start, indent);
            }
            '"' => {
                quoted = true;
                write("\"", &mut output, &mut at_line_start, indent);
            }
            '(' => {
                parens += 1;
                write("(", &mut output, &mut at_line_start, indent);
            }
            ')' => {
                parens = parens.saturating_sub(1);
                write(")", &mut output, &mut at_line_start, indent);
            }
            '{' => {
                if !at_line_start {
                    output.push('\n');
                }
                output.push_str(&"\t".repeat(indent));
                output.push('{');
                output.push('\n');
                indent += 1;
                at_line_start = true;
            }
            '}' => {
                if !at_line_start {
                    output.push('\n');
                }
                indent = indent.saturating_sub(1);
                output.push_str(&"\t".repeat(indent));
                output.push('}');
                output.push('\n');
                at_line_start = true;
            }
            ';' if parens == 0 => {
                write(";", &mut output, &mut at_line_start, indent);
                output.push('\n');
                at_line_start = true;
            }
            '\n' | '\r' | '\t' => {
                if !at_line_start {
                    output.push(' ');
                }
            }
            ' ' if at_line_start || output.ends_with(' ') => {}
            _ => write(
                &character.to_string(),
                &mut output,
                &mut at_line_start,
                indent,
            ),
        }
    }
    output
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn is_managed_file_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && std::path::Path::new(name)
            .file_name()
            .is_some_and(|file| file == name)
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn split_bounded_list(value: &str, max_items: usize, max_item_bytes: usize) -> (Vec<String>, bool) {
    let all = value
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let truncated = all.len() > max_items
        || all
            .iter()
            .take(max_items)
            .any(|value| value.len() > max_item_bytes);
    let items = all
        .into_iter()
        .take(max_items)
        .map(|value| {
            if value.len() <= max_item_bytes {
                value.to_string()
            } else {
                let mut end = max_item_bytes;
                while !value.is_char_boundary(end) {
                    end -= 1;
                }
                value[..end].to_string()
            }
        })
        .collect();
    (items, truncated)
}

fn version_order(left: &str, right: &str) -> std::cmp::Ordering {
    match (Version::parse(left), Version::parse(right)) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ if left == right => std::cmp::Ordering::Equal,
        // An unrecognized installed version is never safe to overwrite
        // automatically because its precedence cannot be proven.
        _ => std::cmp::Ordering::Greater,
    }
}

fn parse_validation_cursor(cursor: &str) -> Option<(String, usize)> {
    let mut parts = cursor.split(':');
    (parts.next()? == "wv1").then_some(())?;
    let token = parts.next()?.to_string();
    let offset = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((token, offset))
}

fn parse_resource_list_cursor(cursor: &str) -> Option<(String, usize)> {
    let mut parts = cursor.split(':');
    (parts.next()? == "wrl1").then_some(())?;
    let signature = parts.next()?.to_string();
    let offset = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((signature, offset))
}

fn valid_resource_root_path(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    let Some((addon, logical_path)) = value
        .strip_prefix('$')
        .and_then(|value| value.split_once(':'))
    else {
        return false;
    };
    !addon.is_empty()
        && !addon.contains([':', ';', '|', '/', '\\'])
        && !logical_path.starts_with('/')
        && !logical_path.contains("..")
        && !logical_path.contains([';', '|', '\\'])
}

fn parse_resource_search_hit(value: &str) -> Result<WorkbenchResourceSearchHit, WorkbenchFailure> {
    let mut fields = value.split('|');
    let resource_name = fields.next().unwrap_or_default();
    let addon_guid = fields.next().unwrap_or_default();
    let addon_id = fields.next().unwrap_or_default();
    let logical_path = fields.next().unwrap_or_default();
    let extension = fields.next().unwrap_or_default();
    if fields.next().is_some()
        || resource_name.is_empty()
        || addon_guid.is_empty()
        || logical_path.is_empty()
        || extension.is_empty()
        || !resource_name.starts_with('{')
        || !resource_name.contains('}')
        || logical_path.contains("..")
        || logical_path.contains([';', '|', '\\'])
        || logical_path.starts_with('/')
    {
        return Err(failure(WorkbenchFailureCode::Protocol));
    }
    let (stem, observed_extension) = logical_path
        .rsplit_once('.')
        .filter(|(_, observed_extension)| !observed_extension.is_empty())
        .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
    if observed_extension != extension {
        return Err(failure(WorkbenchFailureCode::Protocol));
    }
    let name = stem
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| failure(WorkbenchFailureCode::Protocol))?;
    Ok(WorkbenchResourceSearchHit {
        resource_name: resource_name.to_owned(),
        addon_guid: addon_guid.to_owned(),
        addon_id: (!addon_id.is_empty()).then(|| addon_id.to_owned()),
        logical_path: logical_path.to_owned(),
        name: name.to_owned(),
        extension: extension.to_owned(),
    })
}

fn parse_entity_list_cursor(cursor: &str) -> Option<(String, usize)> {
    let mut fields = cursor.split(':');
    (fields.next()? == "wel1")
        .then_some(())
        .and_then(|_| Some((fields.next()?.to_string(), fields.next()?.parse().ok()?)))
        .filter(|_| fields.next().is_none())
}

fn latest_workbench_log(workbench_root: &std::path::Path) -> Option<PathBuf> {
    let logs = workbench_root.join("logs");
    let mut directories = fs::read_dir(logs)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .collect::<Vec<_>>();
    directories.sort_by_key(|entry| entry.file_name());
    directories
        .into_iter()
        .rev()
        .map(|entry| entry.path().join("console.log"))
        .find(|path| path.is_file())
}

fn workbench_log_markers(source: &str, lines: &[String]) -> Vec<WorkbenchLogMarker> {
    if source != "workbench" {
        return Vec::new();
    }
    const MARKERS: [(&str, &str); 5] = [
        ("reload-started", "Reloading game scripts"),
        ("script-validation", "Script validation"),
        ("gamelib-compilation", "Compiling GameLib scripts"),
        ("game-compilation", "Compiling Game scripts"),
        (
            "workbench-game-module-loaded",
            "Module: WorkbenchGame; loaded",
        ),
    ];
    lines
        .iter()
        .enumerate()
        .flat_map(|(line_index, line)| {
            MARKERS
                .iter()
                .filter(move |(_, marker)| line.contains(marker))
                .map(move |(kind, _)| WorkbenchLogMarker {
                    kind: (*kind).to_string(),
                    line_index,
                })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkbenchLogCursor {
    path: PathBuf,
    length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReloadLogVerification {
    path: PathBuf,
    lines: Vec<String>,
}

fn log_cursor(path: &std::path::Path) -> std::io::Result<WorkbenchLogCursor> {
    Ok(WorkbenchLogCursor {
        path: path.to_path_buf(),
        length: fs::metadata(path)?.len(),
    })
}

fn reload_verification_since(
    path: &std::path::Path,
    before: Option<&WorkbenchLogCursor>,
) -> std::io::Result<Option<ReloadLogVerification>> {
    let Some(before) = before.filter(|cursor| cursor.path == path) else {
        return Ok(None);
    };
    let offset = before.length;
    let mut file = fs::File::open(path)?;
    let length = file.metadata()?.len();
    if length <= offset || length - offset > MAX_LOG_READ_BYTES {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    let lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    let required_markers = [
        "Reloading game scripts",
        "Script validation",
        "Compiling GameLib scripts",
        "Compiling Game scripts",
        "Module: WorkbenchGame; loaded",
    ];
    let mut next_marker = 0;
    let mut matched = Vec::new();
    for line in &lines {
        if line.contains(required_markers[next_marker]) {
            matched.push(line.clone());
            next_marker += 1;
            if next_marker == required_markers.len() {
                return Ok(Some(ReloadLogVerification {
                    path: path.to_path_buf(),
                    lines: matched,
                }));
            }
        }
    }
    Ok(None)
}

fn workbench_has_minimized_window(process_id: u32) -> Result<bool, &'static str> {
    let script = format!(
        r#"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class RSTWorkbenchWindowState {{
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
}}
'@
$process = Get-Process -Id {process_id} -ErrorAction Stop
[pscustomobject]@{{minimized = [RSTWorkbenchWindowState]::IsIconic($process.MainWindowHandle)}} | ConvertTo-Json -Compress
"#
    );
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|_| "workbench-window-state-unavailable")?;
    if !output.status.success() {
        return Err("workbench-window-state-unavailable");
    }
    #[derive(Deserialize)]
    struct WindowState {
        minimized: bool,
    }
    serde_json::from_slice::<WindowState>(&output.stdout)
        .map(|state| state.minimized)
        .map_err(|_| "workbench-window-state-unavailable")
}

const MAX_LOG_READ_BYTES: u64 = 512 * 1024;

fn bounded_log_tail(
    path: &std::path::Path,
    line_count: usize,
) -> std::io::Result<(Vec<String>, bool)> {
    let mut file = fs::File::open(path)?;
    let length = file.metadata()?.len();
    let offset = length.saturating_sub(MAX_LOG_READ_BYTES);
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::with_capacity((length - offset) as usize);
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut all = text.lines();
    if offset > 0 {
        all.next();
    }
    let all = all.map(str::to_string).collect::<Vec<_>>();
    let truncated = offset > 0 || all.len() > line_count;
    let lines = all
        .into_iter()
        .rev()
        .take(line_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    Ok((lines, truncated))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProcessIdentity {
    id: u32,
    start_ticks: u64,
}

fn workbench_processes() -> Vec<ProcessIdentity> {
    let script = "$items=@(Get-Process -Name ArmaReforgerWorkbenchSteamDiag -ErrorAction SilentlyContinue | ForEach-Object { [pscustomobject]@{ id=[uint32]$_.Id; startTicks=[uint64]$_.StartTime.ToUniversalTime().Ticks } }); ConvertTo-Json -Compress -InputObject $items";
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    parse_process_identities(&output.stdout)
}

fn force_stop_workbench_script(process: ProcessIdentity) -> String {
    format!(
        "$p=Get-Process -Id {} -ErrorAction Stop; \
         if ($p.ProcessName -ne 'ArmaReforgerWorkbenchSteamDiag' -or \
             [uint64]$p.StartTime.ToUniversalTime().Ticks -ne [uint64]{}) {{ exit 2 }}; \
         Stop-Process -Id $p.Id -Force",
        process.id, process.start_ticks
    )
}

fn parse_process_identities(bytes: &[u8]) -> Vec<ProcessIdentity> {
    serde_json::from_slice(bytes).unwrap_or_else(|_| {
        serde_json::from_slice::<ProcessIdentity>(bytes)
            .map(|process| vec![process])
            .unwrap_or_default()
    })
}

fn workbench_process_ids() -> Vec<u32> {
    workbench_processes()
        .into_iter()
        .map(|process| process.id)
        .collect()
}

fn workbench_project_title(process: ProcessIdentity) -> Option<String> {
    let script = format!(
        r#"
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class RSTRestartProject {{
 public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);
}}
'@
$p = Get-Process -Id {process_id} -ErrorAction Stop
if ($p.ProcessName -ne 'ArmaReforgerWorkbenchSteamDiag' -or [uint64]$p.StartTime.ToUniversalTime().Ticks -ne [uint64]{start_ticks}) {{ exit 2 }}
$titles = [System.Collections.Generic.List[string]]::new()
$callback = [RSTRestartProject+EnumWindowsProc] {{ param([IntPtr]$hWnd, [IntPtr]$unused)
 [uint32]$owner = 0; [void][RSTRestartProject]::GetWindowThreadProcessId($hWnd, [ref]$owner)
 if ($owner -eq $p.Id -and [RSTRestartProject]::IsWindowVisible($hWnd)) {{
  $title = [System.Text.StringBuilder]::new(512); [void][RSTRestartProject]::GetWindowText($hWnd, $title, $title.Capacity)
  $value = $title.ToString(); if ($value.StartsWith('Enfusion Workbench - ', [System.StringComparison]::Ordinal)) {{ $titles.Add($value.Substring('Enfusion Workbench - '.Length)) }}
 }}
 return $true
}}
[void][RSTRestartProject]::EnumWindows($callback, [IntPtr]::Zero)
if ($titles.Count -ne 1) {{ exit 3 }}
$titles[0]
"#,
        process_id = process.id,
        start_ticks = process.start_ticks,
    );
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .output()
        .ok()?;
    output.status.success().then_some(())?;
    let title = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!title.is_empty()).then_some(title)
}

/// Returns the exact `.gproj` passed to the already-running Workbench process.
///
/// The command line is more authoritative than the window title: user addons commonly live
/// outside the Tools installation's `addons` directory and therefore cannot be rediscovered by
/// title alone.
fn workbench_project_gproj(process: ProcessIdentity) -> Option<PathBuf> {
    let script = format!(
        r#"$commandLine = (Get-CimInstance Win32_Process -Filter 'ProcessId = {}' -ErrorAction Stop).CommandLine;
if ($commandLine -match '(?i)(?:^|\s)-gproj\s+(?:"([^"]+)"|(\S+))') {{
    if ($Matches[1]) {{ $Matches[1] }} else {{ $Matches[2] }}
}}"#,
        process.id
    );
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    output.status.success().then_some(())?;
    let path = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
    (path.is_absolute()
        && path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("gproj"))
        && path.is_file())
    .then_some(path)
}

fn resolve_project_gproj(workbench_root: &std::path::Path, title: &str) -> Option<PathBuf> {
    let mut directories = vec![workbench_root.join("addons")];
    let mut matches = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).ok()?.flatten() {
            let path = entry.path();
            let kind = entry.file_type().ok()?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                directories.push(path);
            } else if kind.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("gproj"))
                && fs::read_to_string(&path)
                    .ok()
                    .is_some_and(|text| project_title(&text) == Some(title))
            {
                matches.push(path);
            }
        }
        if matches.len() > 1 || directories.len() > 1_024 {
            return None;
        }
    }
    (matches.len() == 1).then(|| matches.remove(0))
}

fn project_title(content: &str) -> Option<&str> {
    content.lines().find_map(|line| {
        let line = line.trim();
        let value = line.strip_prefix("TITLE ")?.trim();
        value.strip_prefix('"')?.strip_suffix('"')
    })
}

fn base_game_addons_directory(game_directory: Option<&std::path::Path>) -> Option<PathBuf> {
    let addons = game_directory?.join("addons");
    addons
        .join("data")
        .join("ArmaReforger.gproj")
        .is_file()
        .then_some(addons)
}

fn workbench_launch_arguments(
    project: Option<&std::path::Path>,
    game_directory: Option<&std::path::Path>,
) -> Option<Vec<std::ffi::OsString>> {
    let mut arguments = vec![std::ffi::OsString::from("-noThrow")];
    let project = match project {
        Some(project) => project,
        None => return Some(arguments),
    };
    let game_addons = base_game_addons_directory(game_directory)?;
    arguments.extend([
        std::ffi::OsString::from("-gproj"),
        project.as_os_str().to_os_string(),
        std::ffi::OsString::from("-addonsDir"),
        game_addons.into_os_string(),
    ]);
    Some(arguments)
}

fn discover_steam_app(app_id: &str, default_folder: &str) -> Option<PathBuf> {
    let steam_root = std::env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"))
        .join("Steam");
    discover_steam_app_from_root(&steam_root, app_id, default_folder)
}

fn discover_steam_app_from_root(
    steam_root: &std::path::Path,
    app_id: &str,
    default_folder: &str,
) -> Option<PathBuf> {
    let mut libraries = vec![steam_root.to_path_buf()];
    if let Ok(vdf) = fs::read_to_string(steam_root.join("steamapps").join("libraryfolders.vdf")) {
        for line in vdf.lines() {
            let values = line
                .split('"')
                .enumerate()
                .filter_map(|(index, value)| (index % 2 == 1).then_some(value))
                .collect::<Vec<_>>();
            if values.first().is_some_and(|value| *value == "path") {
                if let Some(value) = values.get(1) {
                    libraries.push(PathBuf::from(value.replace("\\\\", "\\")));
                }
            }
        }
    }
    let mut candidates = libraries
        .iter()
        .filter_map(|library| {
            let steamapps = library.join("steamapps");
            let manifest = steamapps.join(format!("appmanifest_{app_id}.acf"));
            let content = fs::read_to_string(manifest).ok()?;
            let install_dir =
                acf_string(&content, "installdir").unwrap_or_else(|| default_folder.to_string());
            let candidate = steamapps.join("common").join(install_dir);
            candidate.is_dir().then_some(candidate)
        })
        .collect::<Vec<_>>();
    let canonical = steam_root
        .join("steamapps")
        .join("common")
        .join(default_folder);
    candidates.sort();
    candidates.dedup();
    match candidates.len() {
        0 => canonical.is_dir().then_some(canonical),
        1 => Some(candidates.remove(0)),
        _ => None,
    }
}

fn acf_string(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let values = line
            .split('"')
            .enumerate()
            .filter_map(|(index, value)| (index % 2 == 1).then_some(value))
            .collect::<Vec<_>>();
        (values.first().is_some_and(|value| *value == key))
            .then(|| values.get(1).map(|value| (*value).to_string()))
            .flatten()
    })
}

fn bridge_payload() -> &'static [(&'static str, &'static str)] {
    &[
        ("RST_WorkbenchCapabilities.c", BRIDGE_CAPABILITIES_SOURCE),
        ("RST_WorkbenchState.c", BRIDGE_STATE_SOURCE),
        ("RST_WorkbenchListEditors.c", BRIDGE_LIST_EDITORS_SOURCE),
        ("RST_WorkbenchOpenEditor.c", BRIDGE_OPEN_EDITOR_SOURCE),
        ("RST_WorkbenchOpenResource.c", BRIDGE_OPEN_RESOURCE_SOURCE),
        ("RST_WorkbenchPlaySession.c", BRIDGE_PLAY_SESSION_SOURCE),
        (
            "RST_WorkbenchProjectContext.c",
            BRIDGE_PROJECT_CONTEXT_SOURCE,
        ),
        (
            "RST_WorkbenchInspectResource.c",
            BRIDGE_INSPECT_RESOURCE_SOURCE,
        ),
        (
            "RST_WorkbenchWorldSelection.c",
            BRIDGE_WORLD_SELECTION_SOURCE,
        ),
        (
            "RST_WorkbenchSelectedEntityHierarchy.c",
            BRIDGE_SELECTED_ENTITY_HIERARCHY_SOURCE,
        ),
        ("RST_WorkbenchListEntities.c", BRIDGE_ENTITY_LIST_SOURCE),
        ("RST_WorkbenchSearchEntities.c", BRIDGE_ENTITY_SEARCH_SOURCE),
        ("RST_WorkbenchLayerState.c", BRIDGE_LAYER_STATE_SOURCE),
        ("RST_WorkbenchInspectEntity.c", BRIDGE_ENTITY_INSPECT_SOURCE),
        ("RST_WorkbenchSetSelection.c", BRIDGE_SET_SELECTION_SOURCE),
        (
            "RST_WorkbenchFindEntitiesByRadius.c",
            BRIDGE_ENTITY_RADIUS_QUERY_SOURCE,
        ),
        ("RST_WorkbenchSampleTerrain.c", BRIDGE_TERRAIN_SAMPLE_SOURCE),
        (
            "RST_WorkbenchViewportContext.c",
            BRIDGE_VIEWPORT_CONTEXT_SOURCE,
        ),
        ("RST_WorkbenchTrace.c", BRIDGE_TRACE_SOURCE),
        (
            "RST_WorkbenchClearSelection.c",
            BRIDGE_CLEAR_SELECTION_SOURCE,
        ),
        (
            "RST_WorkbenchEntityMutation.c",
            BRIDGE_ENTITY_MUTATION_SOURCE,
        ),
        ("RST_WorkbenchShapePoints.c", BRIDGE_SHAPE_POINTS_SOURCE),
        ("RST_WorkbenchShapeGeometry.c", BRIDGE_SHAPE_GEOMETRY_SOURCE),
        ("RST_WorkbenchComponents.c", BRIDGE_COMPONENTS_SOURCE),
        ("RST_WorkbenchProperties.c", BRIDGE_PROPERTIES_SOURCE),
        ("RST_WorkbenchPrefab.c", BRIDGE_PREFAB_SOURCE),
        ("RST_WorkbenchListResources.c", BRIDGE_LIST_RESOURCES_SOURCE),
    ]
}

/* Retired UI prototypes. Kept out of the compiled bridge pending deletion in
the next focused Workbench-UI branch. */
/*
// UI-only prototype: validate the unified World Editor entry point before
// implementing source resolution or launching VS Code.
const BRIDGE_OPEN_DEFINITION_PROTOTYPE_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_OpenDefinitionPrototypeDialog
{
    protected static ref ParamEnumArray s_Targets = new ParamEnumArray();

    [Attribute("", UIWidgets.ComboBox, "Open definition", "Choose the entity class, a component class, or one of the selected entity's component properties.", enums: GetTargets())]
    protected string m_sTarget;

    [ButtonAttribute("Preview target", true)]
    protected int ButtonPreviewTarget()
    {
        return 1;
    }

    [ButtonAttribute("Cancel")]
    protected int ButtonCancel()
    {
        return 0;
    }

    static ParamEnumArray GetTargets()
    {
        return s_Targets;
    }

    static void PopulateTargets(IEntitySource entity)
    {
        s_Targets.Clear();
        s_Targets.Insert(new ParamEnum(string.Format("[Entity] %1", entity.GetClassName()), string.Format("entity:%1", entity.GetClassName())));

        for (int componentIndex, componentCount = entity.GetComponentCount(); componentIndex < componentCount; componentIndex++)
        {
            IEntityComponentSource component = entity.GetComponent(componentIndex);
            if (!component)
                continue;

            string componentClass = component.GetClassName();
            string componentTarget = string.Format("component:%1:%2", componentIndex, componentClass);
            s_Targets.Insert(new ParamEnum(string.Format("  [Component] %1", componentClass), componentTarget));

            for (int propertyIndex, propertyCount = component.GetNumVars(); propertyIndex < propertyCount; propertyIndex++)
            {
                string propertyName = component.GetVarName(propertyIndex);
                if (propertyName.IsEmpty())
                    continue;

                s_Targets.Insert(new ParamEnum(string.Format("    [Property] %1.%2", componentClass, propertyName), string.Format("property:%1:%2:%3", componentIndex, componentClass, propertyName)));
            }
        }
    }

    string TargetLabel()
    {
        foreach (ParamEnum target : s_Targets)
        {
            if (target.m_Value == m_sTarget)
                return target.m_Key;
        }

        return m_sTarget;
    }
}

[WorkbenchPluginAttribute(name: "[Prototype] Open Definition…", description: "UI-only prototype for a unified entity, component, and property definition picker.", category: "Reforger Script Tools", wbModules: { "WorldEditor" }, shortcut: "Ctrl+Alt+O")]
class RST_OpenDefinitionPrototypePlugin : WorkbenchPlugin
{
    override void Run()
    {
        WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
        if (!worldEditor || !worldEditor.GetApi())
        {
            Workbench.Dialog("Open Definition (prototype)", "The World Editor API is not available.");
            return;
        }

        WorldEditorAPI api = worldEditor.GetApi();
        if (api.GetSelectedEntitiesCount() < 1)
        {
            Workbench.Dialog("Open Definition (prototype)", "Select an entity first, then press Ctrl+Alt+O.");
            return;
        }

        IEntitySource selected = api.GetSelectedEntity(0);
        if (!selected)
        {
            Workbench.Dialog("Open Definition (prototype)", "The selected entity is no longer available.");
            return;
        }

        RST_OpenDefinitionPrototypeDialog dialog = new RST_OpenDefinitionPrototypeDialog();
        RST_OpenDefinitionPrototypeDialog.PopulateTargets(selected);
        string message = string.Format("Selected entity: %1\n\nPick a concrete definition from the list. Components and their properties are included directly under the entity. This prototype intentionally does not open VS Code or Workbench's script editor.", selected.GetClassName());
        if (!Workbench.ScriptDialog("Open Definition in VS Code (prototype)", message, dialog))
            return;

        string target = dialog.TargetLabel();
        PrintFormat("RST open-definition prototype: selected=%1 target=%2", selected.GetClassName(), target);
        Workbench.Dialog("Open Definition (prototype)", string.Format("Would open the %1 for %2 in VS Code.\n\nNo editor was launched: this is the UX-only prototype.", target, selected.GetClassName()));
    }
}
#endif
"#;

// UI-only prototype: test the native Custom section before adding VS Code
// launching or generalized context-menu coverage.
const BRIDGE_OPEN_DEFINITION_CONTEXT_PROTOTYPE_SOURCE: &str = r#"#ifdef WORKBENCH
// Workbench only permits an addon to mod classes supplied by that addon. This
// prototype deliberately targets the Test Bullshit entity to prove the native
// Custom-menu callback before we introduce project-owned registration.
modded class GRAY_ENT
{
    protected static const int RST_OPEN_DEFINITION_CONTEXT_ID = 17201;

    override array<ref WB_UIMenuItem> _WB_GetContextMenuItems()
    {
        return { new WB_UIMenuItem("Open entity definition in VS Code (prototype)", RST_OPEN_DEFINITION_CONTEXT_ID) };
    }

    override void _WB_OnContextMenu(int id)
    {
        if (id != RST_OPEN_DEFINITION_CONTEXT_ID)
            return;

        WorldEditorAPI api = _WB_GetEditorAPI();
        if (!api)
        {
            Workbench.Dialog("Open Definition (prototype)", "The World Editor API is not available.");
            return;
        }

        IEntitySource source = api.EntityToSource(this);
        if (!source)
        {
            Workbench.Dialog("Open Definition (prototype)", "The selected entity source is not available.");
            return;
        }

        string className = source.GetClassName();
        PrintFormat("RST open-definition context prototype: entity=%1", className);
        Workbench.Dialog("Open Definition (prototype)", string.Format("Would open the entity definition %1 in VS Code.\n\nNo editor was launched: this is the native-context-menu prototype.", className));
    }
}

// The component callback appears in the component's own Custom menu.
modded class GRAY_TEST
{
    protected static const int RST_OPEN_COMPONENT_DEFINITION_CONTEXT_ID = 17202;

    override array<ref WB_UIMenuItem> _WB_GetContextMenuItems(IEntity owner)
    {
        return { new WB_UIMenuItem("Open component definition in VS Code (prototype)", RST_OPEN_COMPONENT_DEFINITION_CONTEXT_ID) };
    }

    override void _WB_OnContextMenu(IEntity owner, int id)
    {
        if (id != RST_OPEN_COMPONENT_DEFINITION_CONTEXT_ID)
            return;

        WorldEditorAPI api = GenericEntity.Cast(owner)._WB_GetEditorAPI();
        IEntitySource source = api.EntityToSource(owner);
        string ownerClass = "unknown";
        if (source)
            ownerClass = source.GetClassName();

        PrintFormat("RST open-definition context prototype: component=GRAY_TEST owner=%1", ownerClass);
        Workbench.Dialog("Open Definition (prototype)", string.Format("Would open the component definition GRAY_TEST on %1 in VS Code.\n\nNo editor was launched: this is the native-context-menu prototype.", ownerClass));
    }
}
#endif
"#;

*/

const BRIDGE_CAPABILITIES_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchCapabilitiesRequest : JsonApiStruct
{
	void RST_WorkbenchCapabilitiesRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchCapabilitiesResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string capabilities;

	void RST_WorkbenchCapabilitiesResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchCapabilities : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchCapabilitiesRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchCapabilitiesResponse response = new RST_WorkbenchCapabilitiesResponse();
	response.bridgeVersion = "1.51.0";
	response.protocolVersion = 1;
	response.capabilities = "state;editors;open-resource;play-session;project-context;inspect-resource;world-selection;entity-hierarchy;list-resources;list-entities;layer-state;inspect-entity;set-selection;clear-selection;entity-position;entity-details;create-entity;rename-entity;delete-entity;move-entity;rotate-entity;reparent-entity;duplicate-entity;entity-properties;components;component-properties;reload-action";
		return response;
	}
}
#endif
"#;

const BRIDGE_STATE_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchStateRequest : JsonApiStruct
{
	bool executeReloadAction;
	bool executeSaveAllAction;
	bool executeSaveWorldAction;

	void RST_WorkbenchStateRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchStateResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string mode;
	bool worldEditorActive;
	bool worldEditorModulePresent;
	bool worldEditorApiAvailable;
	string playSession;
	string loadedAddons;
	int currentSubScene;
	int currentEntityLayerId;
	string activeSubsceneLayer;
	bool reloadActionAccepted;
	string reloadActionPath;
	bool saveAllActionAccepted;
	string saveAllActionPath;
	bool worldSaveActionAccepted;
	string worldSaveActionPath;
	string worldSaveStatus;

	void RST_WorkbenchStateResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchState : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchStateRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchStateRequest typedRequest = RST_WorkbenchStateRequest.Cast(request);
		RST_WorkbenchStateResponse response = new RST_WorkbenchStateResponse();
	response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		response.mode = "workbench";
		response.playSession = "unavailable";
		response.reloadActionPath = "Plugins/Settings/Reload WB Scripts";
		response.saveAllActionPath = "File/Save All";
		response.worldSaveActionPath = "WorldEditor.Save";
		response.worldSaveStatus = "skipped-no-open-world";
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (typedRequest && typedRequest.executeReloadAction)
		{
			ResourceManager resourceManager = Workbench.GetModule(ResourceManager);
			if (resourceManager)
			{
				array<string> menuPath = {"Plugins", "Settings", "Reload WB Scripts"};
				response.reloadActionAccepted = resourceManager.ExecuteAction(menuPath, true);
			}
		}
		if (typedRequest && typedRequest.executeSaveAllAction)
		{
			ResourceManager resourceManager = Workbench.GetModule(ResourceManager);
			if (resourceManager)
			{
				array<string> menuPath = {"File", "Save All"};
				response.saveAllActionAccepted = resourceManager.ExecuteAction(menuPath, true);
			}
			if (worldEditor)
			{
				WorldEditorAPI worldEditorApi = worldEditor.GetApi();
				string worldPath;
				if (worldEditorApi)
					worldEditorApi.GetWorldPath(worldPath);
				if (!worldPath.IsEmpty())
				{
					response.worldSaveActionAccepted = worldEditor.Save();
					if (response.worldSaveActionAccepted)
						response.worldSaveStatus = "saved";
					else
						response.worldSaveStatus = "rejected";
				}
			}
		}
		if (typedRequest && typedRequest.executeSaveWorldAction && worldEditor)
		{
			WorldEditorAPI worldEditorApi = worldEditor.GetApi();
			string worldPath;
			if (worldEditorApi)
				worldEditorApi.GetWorldPath(worldPath);
			if (!worldPath.IsEmpty())
			{
				response.worldSaveActionAccepted = worldEditor.Save();
				if (response.worldSaveActionAccepted)
					response.worldSaveStatus = "saved";
				else
					response.worldSaveStatus = "rejected";
			}
		}
		if (worldEditor)
		{
			response.worldEditorModulePresent = true;
			WorldEditorAPI worldEditorApi = worldEditor.GetApi();
			if (worldEditorApi)
			{
				response.mode = "world-editor";
				response.worldEditorActive = true;
				response.worldEditorApiAvailable = true;
				response.playSession = "unknown";
				response.currentSubScene = worldEditorApi.GetCurrentSubScene();
				response.currentEntityLayerId = worldEditorApi.GetCurrentEntityLayerId();
				response.activeSubsceneLayer = worldEditorApi.GetActiveSubsceneLayer(response.currentSubScene);
			}
			else
			{
				response.playSession = "likely-running";
			}
		}
		array<string> addonGuids = {};
		GameProject.GetLoadedAddons(addonGuids);
		for (int index = 0; index < addonGuids.Count(); index++)
		{
			if (index > 0)
				response.loadedAddons += ";";
			response.loadedAddons += GameProject.GetAddonID(addonGuids[index]);
		}
		return response;
	}
}
#endif
"#;

const BRIDGE_LIST_EDITORS_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchListEditorsResponse : JsonApiStruct
{
	string editors;

	void RST_WorkbenchListEditorsResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchListEditors : NetApiHandler
{
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchListEditorsResponse response = new RST_WorkbenchListEditorsResponse();
		response.editors = "world|World Editor;animation|Animation Editor;audio|Audio Editor;behavior|Behavior Editor;localization|String Editor;particle|Particle Editor;procedural-animation|Procedural Animation Editor;script|Script Editor";
		PrintFormat("RST editor list: %1", response.editors);
		return response;
	}
}
#endif
"#;

const BRIDGE_OPEN_EDITOR_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchOpenEditorRequest : JsonApiStruct
{
	string editorId;

	void RST_WorkbenchOpenEditorRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenEditorResponse : JsonApiStruct
{
	string editorId;
	bool opened;
	string status;

	void RST_WorkbenchOpenEditorResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenEditor : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchOpenEditorRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchOpenEditorRequest typedRequest = RST_WorkbenchOpenEditorRequest.Cast(request);
		RST_WorkbenchOpenEditorResponse response = new RST_WorkbenchOpenEditorResponse();
		if (!typedRequest || typedRequest.editorId == string.Empty)
		{
			response.status = "editor-id-required";
			return response;
		}

		response.editorId = typedRequest.editorId;
		switch (typedRequest.editorId)
		{
			case "world":
				response.opened = Workbench.OpenModule(WorldEditor);
				break;
			case "animation":
				response.opened = Workbench.OpenModule(AnimEditor);
				break;
			case "audio":
				response.opened = Workbench.OpenModule(AudioEditor);
				break;
			case "behavior":
				response.opened = Workbench.OpenModule(BehaviorEditor);
				break;
			case "localization":
				response.opened = Workbench.OpenModule(LocalizationEditor);
				break;
			case "particle":
				response.opened = Workbench.OpenModule(ParticleEditor);
				break;
			case "procedural-animation":
				response.opened = Workbench.OpenModule(ProcAnimEditor);
				break;
			case "script":
				response.opened = Workbench.OpenModule(ScriptEditor);
				break;
			default:
				response.status = "unknown-editor";
				PrintFormat("RST open editor rejected: %1", typedRequest.editorId);
				return response;
		}

		if (response.opened)
			response.status = "opened";
		else
			response.status = "open-failed";
		PrintFormat("RST open editor: id=%1 opened=%2 status=%3", response.editorId, response.opened, response.status);
		return response;
	}
}
#endif
"#;

const BRIDGE_OPEN_RESOURCE_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchOpenResourceRequest : JsonApiStruct
{
	string resourcePath;

	void RST_WorkbenchOpenResourceRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenResourceResponse : JsonApiStruct
{
	string resourcePath;
	bool opened;
	string status;

	void RST_WorkbenchOpenResourceResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenResource : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchOpenResourceRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchOpenResourceRequest typedRequest = RST_WorkbenchOpenResourceRequest.Cast(request);
		RST_WorkbenchOpenResourceResponse response = new RST_WorkbenchOpenResourceResponse();
		if (!typedRequest || typedRequest.resourcePath == string.Empty)
		{
			response.status = "resource-path-required";
			return response;
		}

		response.resourcePath = typedRequest.resourcePath;
		string workbenchPath = typedRequest.resourcePath;
		if (workbenchPath.IndexOf("{") == 0)
		{
			int guidEnd = workbenchPath.IndexOf("}");
			if (guidEnd >= 0)
				workbenchPath = workbenchPath.Substring(guidEnd + 1, workbenchPath.Length() - guidEnd - 1);
		}
		if (workbenchPath.Length() > 3 && workbenchPath.Substring(workbenchPath.Length() - 3, 3) == ".st")
		{
			Workbench.OpenModule(LocalizationEditor);
			LocalizationEditor stringEditor = Workbench.GetModule(LocalizationEditor);
			if (stringEditor)
				response.opened = stringEditor.SetOpenedResource(typedRequest.resourcePath) && stringEditor.GetTable() != null;
		}
		else
		{
			response.opened = Workbench.OpenResource(workbenchPath);
		}
		if (response.opened)
			response.status = "opened";
		else
			response.status = "open-failed";
		PrintFormat("RST open resource: path=%1 opened=%2 status=%3", response.resourcePath, response.opened, response.status);
		return response;
	}
}
#endif
"#;

const BRIDGE_PLAY_SESSION_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchPlaySessionRequest : JsonApiStruct
{
	bool start;
	bool debugMode;
	bool fullScreen;

	void RST_WorkbenchPlaySessionRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchPlaySessionResponse : JsonApiStruct
{
	bool accepted;
	string status;

	void RST_WorkbenchPlaySessionResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchPlaySession : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchPlaySessionRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPlaySessionRequest typedRequest = RST_WorkbenchPlaySessionRequest.Cast(request);
		RST_WorkbenchPlaySessionResponse response = new RST_WorkbenchPlaySessionResponse();
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		if (typedRequest.start)
		{
			if (!worldEditor.GetApi())
			{
				response.status = "play-session-already-running";
				return response;
			}
			worldEditor.SwitchToGameMode(typedRequest.debugMode, typedRequest.fullScreen);
			response.status = "play-started";
		}
		else
		{
			worldEditor.SwitchToEditMode();
			response.status = "play-stopped";
		}
		response.accepted = true;
		return response;
	}
}
#endif
"#;

const BRIDGE_PROJECT_CONTEXT_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchProjectContextRequest : JsonApiStruct
{
	void RST_WorkbenchProjectContextRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchProjectContextResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string loadedAddons;

	void RST_WorkbenchProjectContextResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchProjectContext : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchProjectContextRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchProjectContextResponse response = new RST_WorkbenchProjectContextResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		array<string> addonGuids = new array<string>();
		GameProject.GetLoadedAddons(addonGuids);
		foreach (string addonGuid : addonGuids)
		{
			string addonId = GameProject.GetAddonID(addonGuid);
			if (addonId != string.Empty)
			{
				if (response.loadedAddons != string.Empty)
					response.loadedAddons += ";";
				response.loadedAddons += addonId;
			}
		}
		return response;
	}
}
#endif
"#;

const BRIDGE_INSPECT_RESOURCE_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchInspectResourceRequest : JsonApiStruct
{
	string resourceName;
	void RST_WorkbenchInspectResourceRequest() { RegAll(); }
}
class RST_WorkbenchInspectResourceResponse : JsonApiStruct
{
	bool found;
	string status;
	string resourceName;
	string className;
	string sourceAddons;
	bool sourceAddonsTruncated;
	void RST_WorkbenchInspectResourceResponse() { RegAll(); }
}
class RST_WorkbenchInspectResource : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchInspectResourceRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchInspectResourceRequest typedRequest = RST_WorkbenchInspectResourceRequest.Cast(request);
		RST_WorkbenchInspectResourceResponse response = new RST_WorkbenchInspectResourceResponse();
		ResourceManager resourceManager = Workbench.GetModule(ResourceManager);
		if (!resourceManager) { response.status = "resource-manager-unavailable"; return response; }
		ResourceName resourceName = typedRequest.resourceName;
		MetaFile meta = resourceManager.GetMetaFile(resourceName.GetPath());
		if (!meta) { response.status = "resource-not-found"; return response; }
		BaseContainer configuration = meta.GetObjectArray("Configurations")[0];
		if (!configuration) { response.status = "resource-configuration-unavailable"; return response; }
		response.found = true;
		response.status = "found";
		response.resourceName = meta.GetResourceID();
		response.className = configuration.GetClassName();
		array<string> sourceAddons = new array<string>();
		meta.GetSourceAddons(sourceAddons);
		int sourceAddonCount = 0;
		foreach (string sourceAddon : sourceAddons)
		{
			if (sourceAddonCount >= 64)
			{
				response.sourceAddonsTruncated = true;
				break;
			}
			if (response.sourceAddons != string.Empty)
				response.sourceAddons += ";";
			response.sourceAddons += sourceAddon;
			sourceAddonCount++;
			if (response.sourceAddons.Length() >= 4096)
			{
				response.sourceAddonsTruncated = true;
				break;
			}
		}
		return response;
	}
}
#endif
"#;

const BRIDGE_WORLD_SELECTION_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchWorldSelectionRequest : JsonApiStruct
{
	void RST_WorkbenchWorldSelectionRequest() { RegAll(); }
}

class RST_WorkbenchWorldSelectionResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	bool editorAvailable;
	string status;
	int selectedCount;
	string selectedEntities;
	bool selectedEntitiesTruncated;

	void RST_WorkbenchWorldSelectionResponse() { RegAll(); }
}

class RST_WorkbenchWorldSelection : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchWorldSelectionRequest(); }
	protected void AppendEntity(out string records, WorldEditorAPI api, IEntitySource entity)
	{
		if (!entity) return;
		if (!records.IsEmpty()) records += ";";
		IEntity runtimeEntity = api.SourceToEntity(entity);
		if (!runtimeEntity) { records += string.Format("%1|%2|%3|%4", entity.GetID().ToString(), entity.GetClassName(), entity.GetSubScene(), entity.GetLayerID()); return; }
		vector transform[4]; runtimeEntity.GetTransform(transform);
		string resourceName = string.Format("%1", entity.GetResourceName()); string name = entity.GetName(); string subSceneName = runtimeEntity.GetWorld().GetSubSceneName(entity.GetSubScene()); string layerName = api.GetEntitySubsceneLayer(entity.GetSubScene(), entity);
		if (name == resourceName) name = string.Empty;
		resourceName.Replace("|", "/"); resourceName.Replace(";", "/"); name.Replace("|", "/"); name.Replace(";", "/"); subSceneName.Replace("|", "/"); subSceneName.Replace(";", "/"); layerName.Replace("|", "/"); layerName.Replace(";", "/");
		records += string.Format("%1|%2|%3|%4|%5|%6|%7", entity.GetID().ToString(), entity.GetClassName(), entity.GetSubScene(), entity.GetLayerID(), transform[3][0], transform[3][1], transform[3][2]) + "|" + resourceName + "|" + name + "|" + subSceneName + "|" + layerName;
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchWorldSelectionResponse response = new RST_WorkbenchWorldSelectionResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}
		WorldEditorAPI worldEditorApi = worldEditor.GetApi();
		if (!worldEditorApi)
		{
			response.status = "world-editor-api-unavailable";
			return response;
		}
		response.editorAvailable = true;
		response.status = "available";
		response.selectedCount = worldEditorApi.GetSelectedEntitiesCount();
		int boundedCount = response.selectedCount;
		if (boundedCount > 32)
		{
			boundedCount = 32;
			response.selectedEntitiesTruncated = true;
		}
		for (int index = 0; index < boundedCount; index++)
		{
			IEntitySource entity = worldEditorApi.GetSelectedEntity(index);
			if (!entity)
				continue;
			AppendEntity(response.selectedEntities, worldEditorApi, entity);
		}
		return response;
	}
}
#endif
"#;

const BRIDGE_SELECTED_ENTITY_HIERARCHY_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchSelectedEntityHierarchyRequest : JsonApiStruct
{
	int selectionIndex;
	void RST_WorkbenchSelectedEntityHierarchyRequest() { RegAll(); }
}
class RST_WorkbenchSelectedEntityHierarchyResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	bool editorAvailable;
	string status;
	string entity;
	string ancestors;
	bool ancestorsTruncated;
	string children;
	bool childrenTruncated;
	void RST_WorkbenchSelectedEntityHierarchyResponse() { RegAll(); }
}
class RST_WorkbenchSelectedEntityHierarchy : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchSelectedEntityHierarchyRequest(); }

	protected void AppendEntity(out string records, WorldEditorAPI api, IEntitySource entity)
	{
		if (!entity)
			return;
		if (records != string.Empty)
			records += ";";
		IEntity runtimeEntity = api.SourceToEntity(entity);
		if (!runtimeEntity) { records += string.Format("%1|%2|%3|%4", entity.GetID().ToString(), entity.GetClassName(), entity.GetSubScene(), entity.GetLayerID()); return; }
		vector transform[4]; runtimeEntity.GetTransform(transform);
		string resourceName = string.Format("%1", entity.GetResourceName()); string name = entity.GetName(); string subSceneName = runtimeEntity.GetWorld().GetSubSceneName(entity.GetSubScene()); string layerName = api.GetEntitySubsceneLayer(entity.GetSubScene(), entity);
		if (name == resourceName) name = string.Empty;
		resourceName.Replace("|", "/"); resourceName.Replace(";", "/"); name.Replace("|", "/"); name.Replace(";", "/"); subSceneName.Replace("|", "/"); subSceneName.Replace(";", "/"); layerName.Replace("|", "/"); layerName.Replace(";", "/");
		records += string.Format("%1|%2|%3|%4|%5|%6|%7", entity.GetID().ToString(), entity.GetClassName(), entity.GetSubScene(), entity.GetLayerID(), transform[3][0], transform[3][1], transform[3][2]) + "|" + resourceName + "|" + name + "|" + subSceneName + "|" + layerName;
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchSelectedEntityHierarchyRequest typedRequest = RST_WorkbenchSelectedEntityHierarchyRequest.Cast(request);
		RST_WorkbenchSelectedEntityHierarchyResponse response = new RST_WorkbenchSelectedEntityHierarchyResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}
		WorldEditorAPI worldEditorApi = worldEditor.GetApi();
		if (!worldEditorApi)
		{
			response.status = "world-editor-api-unavailable";
			return response;
		}
		response.editorAvailable = true;
		if (typedRequest.selectionIndex < 0 || typedRequest.selectionIndex >= worldEditorApi.GetSelectedEntitiesCount())
		{
			response.status = "selection-index-out-of-range";
			return response;
		}
		IEntitySource entity = worldEditorApi.GetSelectedEntity(typedRequest.selectionIndex);
		if (!entity)
		{
			response.status = "selected-entity-unavailable";
			return response;
		}
		response.status = "available";
		AppendEntity(response.entity, worldEditorApi, entity);
		BaseContainer parent = entity.GetParent();
		for (int index = 0; parent && index < 32; index++)
		{
			IEntitySource parentEntity = IEntitySource.Cast(parent);
			if (parentEntity)
				AppendEntity(response.ancestors, worldEditorApi, parentEntity);
			parent = parent.GetParent();
		}
		response.ancestorsTruncated = parent != null;
		int childCount = entity.GetNumChildren();
		int returnedCount = 0;
		for (int index = 0; index < childCount; index++)
		{
			IEntitySource childEntity = IEntitySource.Cast(entity.GetChild(index));
			if (!childEntity)
				continue;
			if (returnedCount >= 64)
			{
				response.childrenTruncated = true;
				break;
			}
			AppendEntity(response.children, worldEditorApi, childEntity);
			returnedCount++;
		}
		return response;
	}
}
#endif
"#;

const BRIDGE_ENTITY_LIST_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchListEntitiesRequest : JsonApiStruct
{
	string query;
	string className;
	int subScene;
	int layerId;
	int offset;
	int limit;
	void RST_WorkbenchListEntitiesRequest() { RegAll(); subScene = -1; layerId = -1; }
}
class RST_WorkbenchListEntitiesResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string worldPath;
	string entities;
	bool hasMore;
	void RST_WorkbenchListEntitiesResponse() { RegAll(); }
}
class RST_WorkbenchListEntities : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchListEntitiesRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchListEntitiesRequest typedRequest = RST_WorkbenchListEntitiesRequest.Cast(request);
		RST_WorkbenchListEntitiesResponse response = new RST_WorkbenchListEntitiesResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor) return response;
		WorldEditorAPI api = worldEditor.GetApi();
		if (!api || typedRequest.offset < 0 || typedRequest.limit < 1 || typedRequest.limit > 100) return response;
		api.GetWorldPath(response.worldPath);
		int matched = 0;
		int returned = 0;
		for (int index = 0, count = api.GetEditorEntityCount(); index < count; index++)
		{
			IEntitySource entity = api.GetEditorEntity(index);
			if (!entity) continue;
			if (typedRequest.subScene >= 0 && entity.GetSubScene() != typedRequest.subScene) continue;
			if (typedRequest.layerId >= 0 && entity.GetLayerID() != typedRequest.layerId) continue;
			string name = api.GetEntityNiceName(entity);
			if (!typedRequest.className.IsEmpty() && entity.GetClassName().IndexOf(typedRequest.className) == -1) continue;
			if (!typedRequest.query.IsEmpty() && name.IndexOf(typedRequest.query) == -1 && entity.GetClassName().IndexOf(typedRequest.query) == -1) continue;
			if (matched++ < typedRequest.offset) continue;
			if (returned >= typedRequest.limit) { response.hasMore = true; break; }
			if (!response.entities.IsEmpty()) response.entities += ";";
			IEntity runtimeEntity = api.SourceToEntity(entity);
			if (!runtimeEntity) response.entities += string.Format("%1|%2|%3|%4", entity.GetID().ToString(), entity.GetClassName(), entity.GetSubScene(), entity.GetLayerID());
			else { vector transform[4]; runtimeEntity.GetTransform(transform); string resourceName = string.Format("%1", entity.GetResourceName()); string authoredName = entity.GetName(); string subSceneName = runtimeEntity.GetWorld().GetSubSceneName(entity.GetSubScene()); string layerName = api.GetEntitySubsceneLayer(entity.GetSubScene(), entity); if (authoredName == resourceName) authoredName = string.Empty; resourceName.Replace("|", "/"); resourceName.Replace(";", "/"); authoredName.Replace("|", "/"); authoredName.Replace(";", "/"); subSceneName.Replace("|", "/"); subSceneName.Replace(";", "/"); layerName.Replace("|", "/"); layerName.Replace(";", "/"); response.entities += string.Format("%1|%2|%3|%4|%5|%6|%7", entity.GetID().ToString(), entity.GetClassName(), entity.GetSubScene(), entity.GetLayerID(), transform[3][0], transform[3][1], transform[3][2]) + "|" + resourceName + "|" + authoredName + "|" + subSceneName + "|" + layerName; }
			returned++;
		}
		return response;
	}
}
#endif
"#;

const BRIDGE_ENTITY_SEARCH_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchSearchEntitiesRequest : JsonApiStruct { string query; string className; string resourceQuery; string componentClasses; string relationDirection; string relationClassName; string relationComponentClasses; int relationMaxDepth; int subScene; int layerId; int offset; int limit; void RST_WorkbenchSearchEntitiesRequest() { RegAll(); subScene = -1; layerId = -1; } }
class RST_WorkbenchSearchEntitiesResponse : JsonApiStruct { string bridgeVersion; int protocolVersion; string status; string worldPath; string results; int totalMatches; int namedMatches; bool hasMore; bool relationTraversalTruncated; void RST_WorkbenchSearchEntitiesResponse() { RegAll(); } }
class RST_WorkbenchSearchEntities : NetApiHandler
{
	static const int MAX_RELATION_CANDIDATES = 4096;
	static const int MAX_RESULT_CHARACTERS = 262144;
	static const int MAX_RESULT_FIELD_CHARACTERS = 4096;
	override JsonApiStruct GetRequest() { return new RST_WorkbenchSearchEntitiesRequest(); }
	protected string BoundResultField(string value)
	{
		if (value.Length() > MAX_RESULT_FIELD_CHARACTERS) return value.Substring(0, MAX_RESULT_FIELD_CHARACTERS);
		return value;
	}
	int ComponentCount(IEntitySource entity)
	{
		int count = entity.GetComponentCount();
		if (count == 0)
		{
			ref BaseContainerList components = entity.GetObjectArray("components");
			if (components) count = components.Count();
		}
		return count;
	}
	IEntityComponentSource ComponentAt(IEntitySource entity, int index)
	{
		if (entity.GetComponentCount() > 0) return entity.GetComponent(index);
		ref BaseContainerList components = entity.GetObjectArray("components");
		if (components) return IEntityComponentSource.Cast(components.Get(index));
		return null;
	}
	protected bool HasComponent(IEntitySource entity, string expected)
	{
		for (int index, count = ComponentCount(entity); index < count; index++) { IEntityComponentSource component = ComponentAt(entity, index); if (component && component.GetClassName() == expected) return true; }
		return false;
	}
	protected bool HasRequiredComponents(IEntitySource entity, array<string> required)
	{
		foreach (string expected : required) if (!HasComponent(entity, expected)) return false;
		return true;
	}
	protected bool MatchesCandidate(IEntitySource entity, RST_WorkbenchSearchEntitiesRequest request, array<string> required)
	{
		if (!entity) return false;
		if (request.subScene >= 0 && entity.GetSubScene() != request.subScene) return false;
		if (request.layerId >= 0 && entity.GetLayerID() != request.layerId) return false;
		string name = entity.GetName(); string resource = string.Format("%1", entity.GetResourceName()); string className = entity.GetClassName();
		if (!request.query.IsEmpty() && !name.Contains(request.query) && !className.Contains(request.query) && !resource.Contains(request.query)) return false;
		if (!request.className.IsEmpty() && className != request.className) return false;
		if (!request.resourceQuery.IsEmpty() && !resource.Contains(request.resourceQuery)) return false;
		return HasRequiredComponents(entity, required);
	}
	protected bool MatchesRelationTarget(IEntitySource entity, RST_WorkbenchSearchEntitiesRequest request, array<string> required)
	{
		return entity && (request.relationClassName.IsEmpty() || entity.GetClassName() == request.relationClassName) && HasRequiredComponents(entity, required);
	}
	protected bool FindRelation(IEntitySource entity, RST_WorkbenchSearchEntitiesRequest request, array<string> required, out IEntitySource related, out int depth, out bool truncated)
	{
		if (request.relationDirection == "parent" || request.relationDirection == "ancestor")
		{
			BaseContainer parent = entity.GetParent();
			for (depth = 1; parent && depth <= request.relationMaxDepth; depth++)
			{
				IEntitySource candidate = IEntitySource.Cast(parent);
				if (MatchesRelationTarget(candidate, request, required)) { related = candidate; return true; }
				parent = parent.GetParent();
			}
			return false;
		}
		int visited = 0; array<IEntitySource> current = {entity};
		for (depth = 1; depth <= request.relationMaxDepth; depth++)
		{
			array<IEntitySource> next = {};
			foreach (IEntitySource parent : current)
			{
				for (int index, count = parent.GetNumChildren(); index < count; index++)
				{
					IEntitySource candidate = IEntitySource.Cast(parent.GetChild(index));
					if (!candidate) continue;
					visited++; if (visited > 1024) { truncated = true; return false; }
					if (MatchesRelationTarget(candidate, request, required)) { related = candidate; return true; }
					next.Insert(candidate);
				}
			}
			current = next;
		}
		return false;
	}
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchSearchEntitiesRequest req = RST_WorkbenchSearchEntitiesRequest.Cast(request); RST_WorkbenchSearchEntitiesResponse response = new RST_WorkbenchSearchEntitiesResponse(); response.bridgeVersion = "1.51.0"; response.protocolVersion = 1;
		PrintFormat("RST entity search: begin limit=%1 offset=%2 relation=%3", req.limit, req.offset, req.relationDirection);
		WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor || !editor.GetApi()) { response.status = "world-editor-unavailable"; return response; }
		bool hasRelation = !req.relationDirection.IsEmpty();
		bool validDirection = req.relationDirection == "parent" || req.relationDirection == "ancestor" || req.relationDirection == "child" || req.relationDirection == "descendant";
		bool invalidRelation = false;
		if (!hasRelation)
			invalidRelation = !req.relationClassName.IsEmpty() || !req.relationComponentClasses.IsEmpty() || req.relationMaxDepth != 0;
		else if (!validDirection || (req.relationClassName.IsEmpty() && req.relationComponentClasses.IsEmpty()) || req.relationMaxDepth < 1 || req.relationMaxDepth > 8)
			invalidRelation = true;
		else if ((req.relationDirection == "parent" || req.relationDirection == "child") && req.relationMaxDepth != 1)
			invalidRelation = true;
		if (req.offset < 0 || req.limit < 1 || req.limit > 100 || invalidRelation) { response.status = "invalid-request"; return response; }
		WorldEditorAPI api = editor.GetApi(); response.status = "available"; api.GetWorldPath(response.worldPath); array<string> required = new array<string>(); if (!req.componentClasses.IsEmpty()) req.componentClasses.Split(";", required, true); array<string> relationRequired = new array<string>(); if (!req.relationComponentClasses.IsEmpty()) req.relationComponentClasses.Split(";", relationRequired, true); int matched = 0; int named = 0; int returned = 0; int relationCandidates = 0; int entityCount = api.GetEditorEntityCount();
		PrintFormat("RST entity search: world=%1 entities=%2 candidateComponents=%3 relationComponents=%4", response.worldPath, entityCount, required.Count(), relationRequired.Count());
		for (int index; index < entityCount; index++)
		{
			if (index > 0 && index % 1000 == 0) PrintFormat("RST entity search: scanned=%1 matched=%2 returned=%3", index, matched, returned);
			IEntitySource entity = api.GetEditorEntity(index);
			if (MatchesCandidate(entity, req, required))
			{
			string name = entity.GetName(); string resource = string.Format("%1", entity.GetResourceName()); string className = entity.GetClassName();
			bool nameMatch = !req.query.IsEmpty() && name.Contains(req.query); bool classMatch = !req.query.IsEmpty() && className.Contains(req.query); bool resourceTextMatch = !req.query.IsEmpty() && resource.Contains(req.query);
			IEntitySource related; int relationDepth; bool relationTruncated = false;
			if (hasRelation && relationCandidates >= MAX_RELATION_CANDIDATES) { response.relationTraversalTruncated = true; response.totalMatches = matched; response.namedMatches = named; PrintFormat("RST entity search: relation candidate cap=%1", MAX_RELATION_CANDIDATES); return response; }
			if (hasRelation) relationCandidates++;
			bool relationMatches = !hasRelation || FindRelation(entity, req, relationRequired, related, relationDepth, relationTruncated);
			if (relationTruncated) { response.relationTraversalTruncated = true; PrintFormat("RST entity search: relation traversal capped entity=%1", entity.GetID().ToString()); }
			if (relationMatches)
			{
			matched = matched + 1; if (!name.IsEmpty()) named = named + 1;
			if (matched > req.offset)
			{
			if (matched > req.offset + req.limit) { response.hasMore = true; response.totalMatches = matched; response.namedMatches = named; PrintFormat("RST entity search: page boundary matched=%1 returned=%2", matched, returned); return response; }
			string components; int componentCount = ComponentCount(entity); PrintFormat("RST entity search: record entity=%1 components=%2", entity.GetID().ToString(), componentCount); for (int componentIndex; componentIndex < componentCount; componentIndex++) { IEntityComponentSource component = ComponentAt(entity, componentIndex); if (!component) continue; if (!components.IsEmpty()) components += ","; components += string.Format("%1", component.GetClassName()); }
			string matchedComponents; foreach (string expected : required) { if (!matchedComponents.IsEmpty()) matchedComponents += ","; matchedComponents += expected; }
			string matches; if (nameMatch) matches = "name"; if (classMatch || !req.className.IsEmpty()) { if (!matches.IsEmpty()) matches += ","; matches += "class"; } if (resourceTextMatch || !req.resourceQuery.IsEmpty()) { if (!matches.IsEmpty()) matches += ","; matches += "resource"; } if (!required.IsEmpty()) { if (!matches.IsEmpty()) matches += ","; matches += "components"; } if (hasRelation) { if (!matches.IsEmpty()) matches += ","; matches += "relation"; }
			IEntitySource parent = IEntitySource.Cast(entity.GetParent()); string parentClass; if (parent) parentClass = parent.GetClassName(); string relationDirection; string relationDepthText; string relationId; string relationClass; string relationSubScene; string relationLayer; string relationComponents;
			if (hasRelation) { relationDirection = req.relationDirection; relationDepthText = relationDepth.ToString(); relationId = related.GetID().ToString(); relationClass = related.GetClassName(); relationSubScene = related.GetSubScene().ToString(); relationLayer = related.GetLayerID().ToString(); foreach (string expected : relationRequired) { if (!relationComponents.IsEmpty()) relationComponents += ","; relationComponents += expected; } }
			className = BoundResultField(className); resource = BoundResultField(resource); name = BoundResultField(name); components = BoundResultField(components); matchedComponents = BoundResultField(matchedComponents); parentClass = BoundResultField(parentClass); relationId = BoundResultField(relationId); relationClass = BoundResultField(relationClass); relationComponents = BoundResultField(relationComponents);
			resource.Replace("|", "/"); resource.Replace(";", "/"); name.Replace("|", "/"); name.Replace(";", "/"); parentClass.Replace("|", "/"); parentClass.Replace(";", "/"); relationId.Replace("|", "/"); relationId.Replace(";", "/"); relationClass.Replace("|", "/"); relationClass.Replace(";", "/");
			string record = string.Format("%1|%2|%3|%4|%5|%6|%7", entity.GetID().ToString(), className, entity.GetSubScene(), entity.GetLayerID(), resource, name, components);
			record += "|" + matches; record += "|" + matchedComponents; record += "|" + parentClass; record += "|" + entity.GetNumChildren();
			record += "|" + relationDirection; record += "|" + relationDepthText; record += "|" + relationId; record += "|" + relationClass;
			record += "|" + relationSubScene; record += "|" + relationLayer; record += "|" + relationComponents;
			if (response.results.Length() + record.Length() + 1 > MAX_RESULT_CHARACTERS) { response.hasMore = true; response.totalMatches = matched; response.namedMatches = named; PrintFormat("RST entity search: response cap=%1 returned=%2", MAX_RESULT_CHARACTERS, returned); return response; }
			if (!response.results.IsEmpty()) response.results += ";"; response.results += record; returned = returned + 1;
			}
			}
			}
		}
		response.totalMatches = matched; response.namedMatches = named; PrintFormat("RST entity search: complete matched=%1 returned=%2 hasMore=%3 relationCapped=%4", matched, returned, response.hasMore, response.relationTraversalTruncated); return response;
	}
}
#endif
"#;

const BRIDGE_LAYER_STATE_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchLayerStateRequest : JsonApiStruct
{
	int subScene;
	int layerId;
	void RST_WorkbenchLayerStateRequest() { RegAll(); }
}
class RST_WorkbenchLayerStateResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string status;
	int subScene;
	int layerId;
	string layerPath;
	bool visible;
	bool explicitlyLocked;
	bool lockedInHierarchy;
	void RST_WorkbenchLayerStateResponse() { RegAll(); }
}
class RST_WorkbenchLayerState : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchLayerStateRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchLayerStateRequest typedRequest = RST_WorkbenchLayerStateRequest.Cast(request);
		RST_WorkbenchLayerStateResponse response = new RST_WorkbenchLayerStateResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		response.subScene = typedRequest.subScene;
		response.layerId = typedRequest.layerId;
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi()) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi();
		if (typedRequest.subScene < 0 || typedRequest.subScene >= api.GetNumSubScenes() || typedRequest.layerId < 0) { response.status = "invalid-layer"; return response; }
		response.layerPath = api.GetSubsceneLayerPath(typedRequest.subScene, typedRequest.layerId);
		if (response.layerPath.IsEmpty()) { response.status = "layer-not-found"; return response; }
		response.visible = api.IsEntityLayerVisible(typedRequest.subScene, typedRequest.layerId);
		response.explicitlyLocked = api.IsEntityLayerLocked(typedRequest.subScene, typedRequest.layerId);
		response.lockedInHierarchy = api.IsEntityLayerLockedHierarchy(typedRequest.subScene, typedRequest.layerId);
		response.status = "available";
		return response;
	}
}
#endif
"#;

const BRIDGE_ENTITY_INSPECT_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchInspectEntityRequest : JsonApiStruct
{
	string entityId;
	void RST_WorkbenchInspectEntityRequest() { RegAll(); }
}
class RST_WorkbenchInspectEntityResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	bool editorAvailable;
	string status;
	string entity;
	string resourceName;
	string resourceReferenceKind;
	string contributorAddons;
	bool contributorAddonsTruncated;
	string ancestors;
	bool ancestorsTruncated;
	string children;
	bool childrenTruncated;
	string components;
	string componentProperties;
	bool componentPropertiesTruncated;
	string properties;
	bool propertiesTruncated;
	void RST_WorkbenchInspectEntityResponse() { RegAll(); }
}
class RST_WorkbenchInspectEntity : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchInspectEntityRequest(); }
	protected void AppendEntity(out string records, WorldEditorAPI api, IEntitySource entity)
	{
		if (!entity) return;
		if (!records.IsEmpty()) records += ";";
		IEntity runtimeEntity = api.SourceToEntity(entity);
		if (!runtimeEntity) { records += string.Format("%1|%2|%3|%4", entity.GetID().ToString(), entity.GetClassName(), entity.GetSubScene(), entity.GetLayerID()); return; }
		vector transform[4]; runtimeEntity.GetTransform(transform);
		string resourceName = string.Format("%1", entity.GetResourceName()); string name = entity.GetName(); string subSceneName = runtimeEntity.GetWorld().GetSubSceneName(entity.GetSubScene()); string layerName = api.GetEntitySubsceneLayer(entity.GetSubScene(), entity);
		if (name == resourceName) name = string.Empty;
		resourceName.Replace("|", "/"); resourceName.Replace(";", "/"); name.Replace("|", "/"); name.Replace(";", "/"); subSceneName.Replace("|", "/"); subSceneName.Replace(";", "/"); layerName.Replace("|", "/"); layerName.Replace(";", "/");
		records += string.Format("%1|%2|%3|%4|%5|%6|%7", entity.GetID().ToString(), entity.GetClassName(), entity.GetSubScene(), entity.GetLayerID(), transform[3][0], transform[3][1], transform[3][2]) + "|" + resourceName + "|" + name + "|" + subSceneName + "|" + layerName;
	}
	// Workbench exposes authored components through either its direct component
	// list or the `components` container, depending on the editor context.
	int ComponentCount(IEntitySource entity)
	{
		int count = entity.GetComponentCount();
		if (count == 0)
		{
			ref BaseContainerList components = entity.GetObjectArray("components");
			if (components) count = components.Count();
		}
		return count;
	}
	IEntityComponentSource ComponentAt(IEntitySource entity, int index)
	{
		IEntityComponentSource component;
		int count = entity.GetComponentCount();
		if (count > 0) component = entity.GetComponent(index);
		else
		{
			ref BaseContainerList components = entity.GetObjectArray("components");
			if (components) component = IEntityComponentSource.Cast(components.Get(index));
		}
		return component;
	}
	protected bool IsEngineCallback(string name)
	{
		return name.StartsWith("EOn") || name.StartsWith("_WB_") || name == "RplLoad"
			|| name == "RplSave" || name == "Preload" || name == "OnTransformResetImpl"
			|| name == "userScript" || name == "constructor" || name == "destructor";
	}
	protected string PropertyTypeName(DataVarType dataType)
	{
		switch (dataType)
		{
			case DataVarType.BOOLEAN:
				return "bool";
			case DataVarType.INTEGER:
				return "integer";
			case DataVarType.SCALAR:
				return "float";
			case DataVarType.VECTOR3:
				return "vector";
			case DataVarType.STRING:
				return "string";
			case DataVarType.RESOURCE_NAME:
				return "resource";
		}
		return string.Empty;
	}
	protected string PropertyOrigin(BaseContainer container, string name)
	{
		if (container.IsVariableSetDirectly(name))
			return "direct";
		for (BaseContainer ancestor = container.GetAncestor(); ancestor; ancestor = ancestor.GetAncestor())
		{
			if (ancestor.IsVariableSetDirectly(name))
				return "inherited";
		}
		return "default";
	}
	protected string PropertyRecord(int componentIndex, BaseContainer container, string name, string typeName, string value)
	{
		name.Replace("|", "/");
		name.Replace(";", "/");
		value.Replace("|", "/");
		value.Replace(";", "/");
		string origin = PropertyOrigin(container, name);
		if (componentIndex >= 0)
			return string.Format("%1|%2|%3|%4|%5|%6", componentIndex, name, typeName, value, container.IsVariableSetDirectly(name), origin);
		return string.Format("%1|%2|%3|%4|%5", name, typeName, value, container.IsVariableSetDirectly(name), origin);
	}
	protected bool ReadPropertyRecord(int componentIndex, BaseContainer container, string name, DataVarType dataType, out string record)
	{
		bool boolValue;
		int integerValue;
		float floatValue;
		vector vectorValue;
		string stringValue;
		ResourceName resourceValue;
		string typeName = PropertyTypeName(dataType);
		if (typeName.IsEmpty())
			return false;

		switch (dataType)
		{
			case DataVarType.BOOLEAN:
				if (!container.Get(name, boolValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, boolValue.ToString());
				return true;
			case DataVarType.INTEGER:
				if (!container.Get(name, integerValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, integerValue.ToString());
				return true;
			case DataVarType.SCALAR:
				if (!container.Get(name, floatValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, floatValue.ToString());
				return true;
			case DataVarType.VECTOR3:
				if (!container.Get(name, vectorValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, vectorValue.ToString(false));
				return true;
			case DataVarType.STRING:
				if (!container.Get(name, stringValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, stringValue);
				return true;
			case DataVarType.RESOURCE_NAME:
				if (!container.Get(name, resourceValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, resourceValue);
				return true;
		}
		return false;
	}
	protected void AppendPropertyRecords(int componentIndex, BaseContainer definitionSource, BaseContainer valueSource, out string records, out bool truncated)
	{
		array<string> seenNames = new array<string>();
		BaseContainer container = definitionSource;
		int returned;
		for (int depth; container && depth < 16; depth++)
		{
			for (int index, count = container.GetNumVars(); index < count; index++)
			{
				string name = container.GetVarName(index);
				DataVarType dataType = container.GetDataVarType(index);
				string record;
				if (seenNames.Find(name) >= 0 || IsEngineCallback(name))
					continue;
				seenNames.Insert(name);
				if (returned >= 128)
				{
					truncated = true;
					return;
				}
				if (!ReadPropertyRecord(componentIndex, valueSource, name, dataType, record))
				{
					if (!ReadPropertyRecord(componentIndex, container, name, dataType, record))
						continue;
				}
				if (!records.IsEmpty())
					records += ";";
				records += record;
				returned++;
			}
			container = container.GetAncestor();
		}
	}
	protected void ResolveOrigin(RST_WorkbenchInspectEntityResponse response, IEntitySource entity, IEntity runtimeEntity)
	{
		response.resourceReferenceKind = "unresolved";
		if (!runtimeEntity) return;
		ResourceManager resourceManager = Workbench.GetModule(ResourceManager);
		if (!resourceManager) { response.resourceReferenceKind = "resource-manager-unavailable"; return; }

		string entityResourceName = string.Format("%1", entity.GetResourceName());
		string subsceneResourceName = runtimeEntity.GetWorld().GetSubSceneName(entity.GetSubScene());
		MetaFile meta;
		if (!entityResourceName.IsEmpty())
		{
			ResourceName entityResource = entityResourceName;
			meta = resourceManager.GetMetaFile(entityResource.GetPath());
			if (meta) response.resourceReferenceKind = "prefab-resource";
		}
		if (!meta && !subsceneResourceName.IsEmpty())
		{
			ResourceName subsceneResource = subsceneResourceName;
			meta = resourceManager.GetMetaFile(subsceneResource.GetPath());
			if (meta) response.resourceReferenceKind = "world-subscene";
		}
		if (!meta) return;

		response.resourceName = meta.GetResourceID();
		array<string> sourceAddons = new array<string>();
		meta.GetSourceAddons(sourceAddons);
		for (int index = 0; index < sourceAddons.Count(); index++)
		{
			if (index >= 64 || response.contributorAddons.Length() >= 4096) { response.contributorAddonsTruncated = true; break; }
			string sourceAddon = sourceAddons[index];
			if (!response.contributorAddons.IsEmpty()) response.contributorAddons += ";";
			response.contributorAddons += sourceAddon;
		}
	}
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchInspectEntityRequest typedRequest = RST_WorkbenchInspectEntityRequest.Cast(request);
		RST_WorkbenchInspectEntityResponse response = new RST_WorkbenchInspectEntityResponse(); response.bridgeVersion = "1.51.0"; response.protocolVersion = 1;
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor); if (!worldEditor) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = worldEditor.GetApi(); if (!api) { response.status = "world-editor-api-unavailable"; return response; }
		response.editorAvailable = true;
		IEntitySource target;
		for (int index = 0, count = api.GetSelectedEntitiesCount(); index < count; index++)
		{
			IEntitySource candidate = api.GetSelectedEntity(index);
			if (candidate && candidate.GetID().ToString() == typedRequest.entityId)
			{
				target = candidate;
				break;
			}
		}
		if (!target)
		{
			for (int index = 0, count = api.GetEditorEntityCount(); index < count; index++)
			{
				IEntitySource candidate = api.GetEditorEntity(index);
				if (candidate && candidate.GetID().ToString() == typedRequest.entityId)
				{
					target = candidate;
					break;
				}
			}
		}
		if (!target) { response.status = "entity-not-found"; return response; }
		response.status = "available"; AppendEntity(response.entity, api, target); ResolveOrigin(response, target, api.SourceToEntity(target));
		int componentCount = ComponentCount(target);
		for (int index = 0; index < componentCount && index < 64; index++)
		{
			IEntityComponentSource component = ComponentAt(target, index);
			if (!component)
				continue;
			if (!response.components.IsEmpty())
				response.components += ";";
			response.components += string.Format("%1|%2", index, component.GetClassName());
			IEntity runtimeTarget = api.SourceToEntity(target);
			GenericComponent runtimeComponent;
			BaseContainer propertySource = component;
			if (runtimeTarget)
			{
				typename componentType = component.GetClassName().ToType();
				if (componentType)
				{
					runtimeComponent = GenericComponent.Cast(runtimeTarget.FindComponent(componentType));
					if (runtimeComponent)
					{
						BaseContainer runtimeSource = runtimeComponent.GetComponentSource(runtimeTarget);
						if (runtimeSource)
						{
							propertySource = runtimeSource;
						}
					}
				}
			}
			AppendPropertyRecords(index, propertySource, component, response.componentProperties, response.componentPropertiesTruncated);
		}
		BaseContainer parent = target.GetParent(); for (int index = 0; parent && index < 32; index++) { IEntitySource parentEntity = IEntitySource.Cast(parent); if (parentEntity) AppendEntity(response.ancestors, api, parentEntity); parent = parent.GetParent(); } response.ancestorsTruncated = parent != null;
		// Stored prefab members can be one-indexed even though GetNumChildren reports
		// their count. Match the prefab inspector's resolved-child indexing here.
		int firstChildIndex;
		if (target.GetNumChildren() > 0 && !target.GetChild(0) && target.GetChild(1))
			firstChildIndex = 1;
		for (int index = 0, count = target.GetNumChildren(), returned = 0; index < count; index++)
		{
			IEntitySource child = IEntitySource.Cast(target.GetChild(index + firstChildIndex));
			if (!child)
				continue;
			if (returned >= 64)
			{
				response.childrenTruncated = true;
				break;
			}
			AppendEntity(response.children, api, child);
			returned++;
		}
		BaseContainer rootDefinition = target;
		ResourceName resourceName = target.GetResourceName();
		if (!resourceName.IsEmpty())
		{
			Resource resource = Resource.Load(resourceName);
			if (resource && resource.IsValid())
			{
				IEntitySource prefabSource = resource.GetResource().ToEntitySource();
				if (prefabSource)
					rootDefinition = prefabSource;
			}
		}
		AppendPropertyRecords(-1, rootDefinition, target, response.properties, response.propertiesTruncated);
		return response;
	}
}
#endif
"#;

const BRIDGE_SET_SELECTION_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchSetSelectionRequest : JsonApiStruct
{
	string entityId;
	void RST_WorkbenchSetSelectionRequest() { RegAll(); }
}
class RST_WorkbenchSetSelectionResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string status;
	string entity;
	void RST_WorkbenchSetSelectionResponse() { RegAll(); }
}
class RST_WorkbenchSetSelection : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchSetSelectionRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchSetSelectionRequest typedRequest = RST_WorkbenchSetSelectionRequest.Cast(request);
		RST_WorkbenchSetSelectionResponse response = new RST_WorkbenchSetSelectionResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi();
		if (!api) { response.status = "world-editor-api-unavailable"; return response; }
		IEntitySource target;
		for (int index = 0, count = api.GetEditorEntityCount(); index < count; index++)
		{
			IEntitySource candidate = api.GetEditorEntity(index);
			if (candidate && candidate.GetID().ToString() == typedRequest.entityId) { target = candidate; break; }
		}
		if (!target) { response.status = "entity-not-found"; return response; }
		api.SetEntitySelection(target);
		response.status = "selected";
		IEntity runtimeEntity = api.SourceToEntity(target);
		if (!runtimeEntity) response.entity = string.Format("%1|%2|%3|%4", target.GetID().ToString(), target.GetClassName(), target.GetSubScene(), target.GetLayerID());
		else { vector transform[4]; runtimeEntity.GetTransform(transform); string resourceName = string.Format("%1", target.GetResourceName()); string name = target.GetName(); string subSceneName = runtimeEntity.GetWorld().GetSubSceneName(target.GetSubScene()); string layerName = api.GetEntitySubsceneLayer(target.GetSubScene(), target); if (name == resourceName) name = string.Empty; resourceName.Replace("|", "/"); resourceName.Replace(";", "/"); name.Replace("|", "/"); name.Replace(";", "/"); subSceneName.Replace("|", "/"); subSceneName.Replace(";", "/"); layerName.Replace("|", "/"); layerName.Replace(";", "/"); response.entity = string.Format("%1|%2|%3|%4|%5|%6|%7", target.GetID().ToString(), target.GetClassName(), target.GetSubScene(), target.GetLayerID(), transform[3][0], transform[3][1], transform[3][2]) + "|" + resourceName + "|" + name + "|" + subSceneName + "|" + layerName; }
		return response;
	}
}
#endif
"#;

const BRIDGE_ENTITY_RADIUS_QUERY_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchFindEntitiesByRadiusRequest : JsonApiStruct
{
	float centerX; float centerY; float centerZ; float radiusMeters; string queryScope; bool requireObject; bool excludeProxies; string className; int limit;
	void RST_WorkbenchFindEntitiesByRadiusRequest() { RegAll(); }
}
class RST_WorkbenchFindEntitiesByRadiusResponse : JsonApiStruct
{
	string bridgeVersion; int protocolVersion; string status; float centerX; float centerY; float centerZ; float radiusMeters; string queryScope; bool requireObject; bool excludeProxies; string entities; bool truncated;
	void RST_WorkbenchFindEntitiesByRadiusResponse() { RegAll(); }
}
class RST_WorkbenchRadiusCollector
{
	WorldEditorAPI m_Api; RST_WorkbenchFindEntitiesByRadiusRequest m_Request; RST_WorkbenchFindEntitiesByRadiusResponse m_Response; int m_Returned;
	void RST_WorkbenchRadiusCollector(WorldEditorAPI api, RST_WorkbenchFindEntitiesByRadiusRequest request, RST_WorkbenchFindEntitiesByRadiusResponse response) { m_Api = api; m_Request = request; m_Response = response; }
	bool AddEntity(IEntity entity)
	{
		IEntitySource source = m_Api.EntityToSource(entity); if (!source) return true;
		if (!m_Request.className.IsEmpty() && source.GetClassName().IndexOf(m_Request.className) == -1) return true;
		if (m_Returned >= m_Request.limit) { m_Response.truncated = true; return false; }
		if (!m_Response.entities.IsEmpty()) m_Response.entities += ";";
		vector position = entity.GetOrigin();
		string resourceName = string.Format("%1", source.GetResourceName()); string name = source.GetName(); string subSceneName = entity.GetWorld().GetSubSceneName(source.GetSubScene()); string layerName = m_Api.GetEntitySubsceneLayer(source.GetSubScene(), source);
		if (name == resourceName) name = string.Empty;
		resourceName.Replace("|", "/"); resourceName.Replace(";", "/"); name.Replace("|", "/"); name.Replace(";", "/"); subSceneName.Replace("|", "/"); subSceneName.Replace(";", "/"); layerName.Replace("|", "/"); layerName.Replace(";", "/");
		m_Response.entities += string.Format("%1|%2|%3|%4|%5|%6|%7", source.GetID().ToString(), source.GetClassName(), source.GetSubScene(), source.GetLayerID(), position[0], position[1], position[2]) + "|" + resourceName + "|" + name + "|" + subSceneName + "|" + layerName;
		m_Returned++; return true;
	}
}
class RST_WorkbenchFindEntitiesByRadius : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchFindEntitiesByRadiusRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchFindEntitiesByRadiusRequest typedRequest = RST_WorkbenchFindEntitiesByRadiusRequest.Cast(request);
		RST_WorkbenchFindEntitiesByRadiusResponse response = new RST_WorkbenchFindEntitiesByRadiusResponse(); response.bridgeVersion = "1.51.0"; response.protocolVersion = 1;
		response.centerX = typedRequest.centerX; response.centerY = typedRequest.centerY; response.centerZ = typedRequest.centerZ; response.radiusMeters = typedRequest.radiusMeters; response.queryScope = typedRequest.queryScope; response.requireObject = typedRequest.requireObject; response.excludeProxies = typedRequest.excludeProxies;
		if (typedRequest.radiusMeters < 0.01 || typedRequest.radiusMeters > 50000 || typedRequest.limit < 1 || typedRequest.limit > 100) { response.status = "invalid-query"; return response; }
		WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi(); if (!api || api.GetEditorEntityCount() < 1) { response.status = "world-editor-api-unavailable"; return response; }
		IEntity root = api.SourceToEntity(api.GetEditorEntity(0)); if (!root) { response.status = "world-unavailable"; return response; }
		EQueryEntitiesFlags flags = EQueryEntitiesFlags.ALL;
		if (typedRequest.queryScope == "static") flags = EQueryEntitiesFlags.STATIC;
		else if (typedRequest.queryScope == "dynamic") flags = EQueryEntitiesFlags.DYNAMIC;
		else if (typedRequest.queryScope == "features") flags = EQueryEntitiesFlags.FEATURES;
		else if (typedRequest.queryScope != "all") { response.status = "invalid-query-scope"; return response; }
		if (typedRequest.requireObject) flags |= EQueryEntitiesFlags.WITH_OBJECT;
		if (typedRequest.excludeProxies) flags |= EQueryEntitiesFlags.NO_PROXIES;
		RST_WorkbenchRadiusCollector collector = new RST_WorkbenchRadiusCollector(api, typedRequest, response);
		root.GetWorld().QueryEntitiesBySphere(Vector(typedRequest.centerX, typedRequest.centerY, typedRequest.centerZ), typedRequest.radiusMeters, collector.AddEntity, null, flags);
		response.status = "available"; return response;
	}
}
#endif
"#;

const BRIDGE_TERRAIN_SAMPLE_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchSampleTerrainRequest : JsonApiStruct
{
	float centerX; float centerZ; float halfExtentMeters; float spacingMeters; bool includeWater;
	void RST_WorkbenchSampleTerrainRequest() { RegAll(); }
}
class RST_WorkbenchSampleTerrainResponse : JsonApiStruct
{
	string bridgeVersion; int protocolVersion; string status;
	float centerX; float centerZ; float halfExtentMeters; float requestedSpacingMeters; float effectiveSpacingMeters; bool spacingClamped;
	float gridOriginX; float gridOriginZ; int gridWidth; int gridHeight; string heights;
	string waterTypes; string waterSurfaceHeights; string waterDepthsAboveTerrain;
	float boundsMinX; float boundsMinY; float boundsMinZ; float boundsMaxX; float boundsMaxY; float boundsMaxZ;
	int heightmapResolutionX; int heightmapResolutionZ; float nativeSpacingMeters; int tileCountX; int tileCountZ;
	void RST_WorkbenchSampleTerrainResponse() { RegAll(); }
}
class RST_WorkbenchSampleTerrain : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchSampleTerrainRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchSampleTerrainRequest typedRequest = RST_WorkbenchSampleTerrainRequest.Cast(request);
		RST_WorkbenchSampleTerrainResponse response = new RST_WorkbenchSampleTerrainResponse();
		response.bridgeVersion = "1.51.0"; response.protocolVersion = 1;
		response.centerX = typedRequest.centerX; response.centerZ = typedRequest.centerZ; response.halfExtentMeters = typedRequest.halfExtentMeters; response.requestedSpacingMeters = typedRequest.spacingMeters;
		if (typedRequest.halfExtentMeters < 0.01 || typedRequest.halfExtentMeters > 500 || typedRequest.spacingMeters < 0 || typedRequest.spacingMeters > 500) { response.status = "invalid-query"; return response; }
		WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi(); if (!api) { response.status = "world-editor-api-unavailable"; return response; }
		BaseWorld world = api.GetWorld(); if (typedRequest.includeWater && !world) { response.status = "water-world-unavailable"; return response; }
		vector boundsMin, boundsMax;
		if (!editor.GetTerrainBounds(boundsMin, boundsMax)) { response.status = "terrain-unavailable"; return response; }
		response.boundsMinX = boundsMin[0]; response.boundsMinY = boundsMin[1]; response.boundsMinZ = boundsMin[2]; response.boundsMaxX = boundsMax[0]; response.boundsMaxY = boundsMax[1]; response.boundsMaxZ = boundsMax[2];
		response.heightmapResolutionX = api.GetTerrainResolutionX(); response.heightmapResolutionZ = api.GetTerrainResolutionY(); response.nativeSpacingMeters = api.GetTerrainUnitScale(); response.tileCountX = api.GetTerrainTilesX(); response.tileCountZ = api.GetTerrainTilesY();
		if (response.heightmapResolutionX < 1 || response.heightmapResolutionZ < 1 || response.nativeSpacingMeters <= 0 || response.tileCountX < 1 || response.tileCountZ < 1) { response.status = "terrain-metadata-unavailable"; return response; }
		response.effectiveSpacingMeters = response.nativeSpacingMeters;
		if (typedRequest.spacingMeters > 0) response.effectiveSpacingMeters = Math.Ceil(typedRequest.spacingMeters / response.nativeSpacingMeters) * response.nativeSpacingMeters;
		response.spacingClamped = typedRequest.spacingMeters > 0 && typedRequest.spacingMeters != response.effectiveSpacingMeters;
		int firstX = Math.Ceil((typedRequest.centerX - typedRequest.halfExtentMeters - boundsMin[0]) / response.effectiveSpacingMeters);
		int lastX = Math.Floor((typedRequest.centerX + typedRequest.halfExtentMeters - boundsMin[0]) / response.effectiveSpacingMeters);
		int firstZ = Math.Ceil((typedRequest.centerZ - typedRequest.halfExtentMeters - boundsMin[2]) / response.effectiveSpacingMeters);
		int lastZ = Math.Floor((typedRequest.centerZ + typedRequest.halfExtentMeters - boundsMin[2]) / response.effectiveSpacingMeters);
		response.gridWidth = lastX - firstX + 1; response.gridHeight = lastZ - firstZ + 1;
		if (response.gridWidth < 1 || response.gridHeight < 1 || response.gridWidth * response.gridHeight > 4096) { response.status = "invalid-sample-grid"; return response; }
		response.gridOriginX = boundsMin[0] + firstX * response.effectiveSpacingMeters; response.gridOriginZ = boundsMin[2] + firstZ * response.effectiveSpacingMeters;
		for (int z = 0; z < response.gridHeight; z++)
		{
			for (int x = 0; x < response.gridWidth; x++)
			{
				if (!response.heights.IsEmpty()) response.heights += ";";
				if (typedRequest.includeWater && !response.waterTypes.IsEmpty()) { response.waterTypes += ";"; response.waterSurfaceHeights += ";"; response.waterDepthsAboveTerrain += ";"; }
				float height; float sampleX = response.gridOriginX + x * response.effectiveSpacingMeters; float sampleZ = response.gridOriginZ + z * response.effectiveSpacingMeters;
				if (!api.TryGetTerrainSurfaceY(sampleX, sampleZ, height)) { response.heights += "~"; if (typedRequest.includeWater) { response.waterTypes += "~"; response.waterSurfaceHeights += "~"; response.waterDepthsAboveTerrain += "~"; } continue; }
				response.heights += height.ToString();
				if (typedRequest.includeWater)
				{
					vector waterSurface; EWaterSurfaceType waterType; vector transformWS[4]; vector obbExtents; float surfaceY;
					if (ChimeraWorldUtils.TryGetWaterSurface(world, Vector(sampleX, height, sampleZ), waterSurface, waterType, transformWS, obbExtents))
					{
						if (waterType == EWaterSurfaceType.WST_OCEAN) response.waterTypes += "o"; else if (waterType == EWaterSurfaceType.WST_POND) response.waterTypes += "p"; else if (waterType == EWaterSurfaceType.WST_RIVER) response.waterTypes += "r"; else response.waterTypes += "n";
						surfaceY = waterSurface[1];
					}
					else
					{
						TraceParam waterTrace = new TraceParam();
						waterTrace.Start = Vector(sampleX, boundsMax[1] + 1, sampleZ);
						waterTrace.End = Vector(sampleX, boundsMin[1] - 1, sampleZ);
						waterTrace.Flags = TraceFlags.ENTS;
						waterTrace.TargetLayers = EPhysicsLayerDefs.Water;
						float traceFraction = world.TraceMove(waterTrace);
						if (traceFraction >= 1)
						{
							response.waterTypes += "n";
							response.waterSurfaceHeights += "~";
							response.waterDepthsAboveTerrain += "~";
							continue;
						}
						surfaceY = waterTrace.Start[1] + (waterTrace.End[1] - waterTrace.Start[1]) * traceFraction;
						if (surfaceY <= height)
						{
							response.waterTypes += "n";
							response.waterSurfaceHeights += "~";
							response.waterDepthsAboveTerrain += "~";
							continue;
						}
						if (waterTrace.TraceEnt && waterTrace.TraceEnt.Type().IsInherited(RiverPartEntity)) response.waterTypes += "r"; else response.waterTypes += "p";
					}
					response.waterSurfaceHeights += surfaceY.ToString(); response.waterDepthsAboveTerrain += (surfaceY - height).ToString();
				}
			}
		}
		response.status = "available"; return response;
	}
}
#endif
"#;

const BRIDGE_VIEWPORT_CONTEXT_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchViewportContextRequest : JsonApiStruct
{
	void RST_WorkbenchViewportContextRequest() { RegAll(); }
}
class RST_WorkbenchViewportContextResponse : JsonApiStruct
{
	string bridgeVersion; int protocolVersion; string status;
	int width; int height; int mouseX; int mouseY; bool mouseInside;
	float cameraX; float cameraY; float cameraZ;
	float cameraDirectionX; float cameraDirectionY; float cameraDirectionZ;
	float startX; float startY; float startZ; float endX; float endY; float endZ;
	float directionX; float directionY; float directionZ;
	void RST_WorkbenchViewportContextResponse() { RegAll(); }
}
class RST_WorkbenchViewportContext : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchViewportContextRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchViewportContextResponse response = new RST_WorkbenchViewportContextResponse();
		response.bridgeVersion = "1.51.0"; response.protocolVersion = 1;
		WorldEditor e = Workbench.GetModule(WorldEditor);
		if (!e || !e.GetApi() || !e.GetApi().GetWorld()) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI a = e.GetApi(); BaseWorld world = a.GetWorld(); vector cameraTransform[4];
		world.GetCurrentCamera(cameraTransform);
		response.cameraX = cameraTransform[3][0]; response.cameraY = cameraTransform[3][1]; response.cameraZ = cameraTransform[3][2];
		response.cameraDirectionX = cameraTransform[2][0]; response.cameraDirectionY = cameraTransform[2][1]; response.cameraDirectionZ = cameraTransform[2][2];
		response.width = a.GetScreenWidth(); response.height = a.GetScreenHeight(); response.mouseX = a.GetMousePosX(false); response.mouseY = a.GetMousePosY(false);
		response.mouseInside = response.mouseX >= 0 && response.mouseY >= 0 && response.mouseX < response.width && response.mouseY < response.height;
		if (!response.mouseInside) { response.status = "mouse-outside-viewport"; return response; }
		vector start, end, direction;
		if (!a.TraceWorldPos(response.mouseX, response.mouseY, TraceFlags.WORLD, start, end, direction)) { response.status = "mouse-world-position-unavailable"; return response; }
		response.startX = start[0]; response.startY = start[1]; response.startZ = start[2]; response.endX = end[0]; response.endY = end[1]; response.endZ = end[2];
		response.directionX = direction[0]; response.directionY = direction[1]; response.directionZ = direction[2]; response.status = "available"; return response;
	}
}
#endif
"#;

const BRIDGE_TRACE_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchTraceRequest : JsonApiStruct
{
	float startX; float startY; float startZ; float endX; float endY; float endZ; string shape; float radius;
	float minsX; float minsY; float minsZ; float maxsX; float maxsY; float maxsZ;
	bool entities; bool terrain; bool ocean; int targetLayers;
	void RST_WorkbenchTraceRequest() { RegAll(); }
}
class RST_WorkbenchTraceResponse : JsonApiStruct
{
	string bridgeVersion; int protocolVersion; string status; bool hit; float fraction; float distance;
	float hitX; float hitY; float hitZ; float normalX; float normalY; float normalZ;
	string kind; string entity; string colliderName; string material;
	void RST_WorkbenchTraceResponse() { RegAll(); }
}
class RST_WorkbenchTrace : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchTraceRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchTraceRequest q = RST_WorkbenchTraceRequest.Cast(request); RST_WorkbenchTraceResponse response = new RST_WorkbenchTraceResponse();
		response.bridgeVersion = "1.51.0"; response.protocolVersion = 1;
		WorldEditor e = Workbench.GetModule(WorldEditor);
		if (!e || !e.GetApi() || !e.GetApi().GetWorld()) { response.status = "world-editor-unavailable"; return response; }
		TraceParam p;
		if (q.shape == "sphere") { TraceSphere s = new TraceSphere(); s.Radius = q.radius; p = s; }
		else if (q.shape == "box") { TraceBox b = new TraceBox(); b.Mins = Vector(q.minsX, q.minsY, q.minsZ); b.Maxs = Vector(q.maxsX, q.maxsY, q.maxsZ); p = b; }
		else if (q.shape == "line") p = new TraceParam();
		else { response.status = "invalid-query"; return response; }
		p.Start = Vector(q.startX, q.startY, q.startZ); p.End = Vector(q.endX, q.endY, q.endZ);
		if (q.entities) p.Flags |= TraceFlags.ENTS; if (q.terrain) p.Flags |= TraceFlags.WORLD; if (q.ocean) p.Flags |= TraceFlags.OCEAN;
		if (q.targetLayers != 0) p.TargetLayers = q.targetLayers;
		response.fraction = e.GetApi().GetWorld().TraceMove(p);
		if (response.fraction >= 1) { response.status = "available"; return response; }
		vector h = p.Start + (p.End - p.Start) * response.fraction; vector travelled = h - p.Start;
		response.hit = true; response.hitX = h[0]; response.hitY = h[1]; response.hitZ = h[2]; response.distance = Math.Sqrt(travelled[0] * travelled[0] + travelled[1] * travelled[1] + travelled[2] * travelled[2]);
		response.normalX = p.TraceNorm[0]; response.normalY = p.TraceNorm[1]; response.normalZ = p.TraceNorm[2]; response.colliderName = p.ColliderName; response.material = p.TraceMaterial;
		if (p.TraceEnt && !p.TraceEnt.Type().IsInherited(GenericTerrainEntity))
		{
			response.kind = "entity"; IEntitySource source = e.GetApi().EntityToSource(p.TraceEnt);
			if (source) response.entity = string.Format("%1|%2|%3|%4", source.GetID().ToString(), source.GetClassName(), source.GetSubScene(), source.GetLayerID());
		}
		else if (p.TraceEnt) response.kind = "terrain";
		else if (q.ocean) response.kind = "ocean";
		else response.kind = "terrain";
		response.status = "available"; return response;
	}
}
#endif
"#;

const BRIDGE_CLEAR_SELECTION_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchClearSelectionRequest : JsonApiStruct
{
	void RST_WorkbenchClearSelectionRequest() { RegAll(); }
}
class RST_WorkbenchClearSelectionResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	bool editorAvailable;
	string status;
	int selectedCount;
	string selectedEntities;
	bool selectedEntitiesTruncated;
	void RST_WorkbenchClearSelectionResponse() { RegAll(); }
}
class RST_WorkbenchClearSelection : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchClearSelectionRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchClearSelectionResponse response = new RST_WorkbenchClearSelectionResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}
		WorldEditorAPI api = editor.GetApi();
		if (!api)
		{
			response.status = "world-editor-api-unavailable";
			return response;
		}
		response.editorAvailable = true;
		api.ClearEntitySelection();
		api.UpdateSelectionGui();
		response.selectedCount = api.GetSelectedEntitiesCount();
		if (response.selectedCount == 0)
			response.status = "available";
		else
			response.status = "selection-not-observed";
		return response;
	}
}
#endif
"#;

const BRIDGE_ENTITY_MUTATION_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchEntityMutationRequest : JsonApiStruct
{
	string entityId;
	string parentEntityId;
	string resourceName;
	string name;
	int subScene;
	float x;
	float y;
	float z;
	float pitch;
	float yaw;
	float roll;
	int layerId;
	bool targetIsResource;
	bool confirm;

	void RST_WorkbenchEntityMutationRequest()
	{
		RegV("entityId");
		RegV("parentEntityId");
		RegV("resourceName");
		RegV("name");
		RegV("subScene");
		RegV("x");
		RegV("y");
		RegV("z");
		RegV("pitch");
		RegV("yaw");
		RegV("roll");
		RegV("layerId");
		RegV("targetIsResource");
		RegV("confirm");
	}
}
class RST_WorkbenchEntityMutationResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string status;
	int activeLayerId;
	string entity;
	string destination;
	bool destinationExists;

	void RST_WorkbenchEntityMutationResponse()
	{
		RegAll();
	}
}
class RST_WorkbenchEntityMutationBase : NetApiHandler
{
	IEntitySource Find(WorldEditorAPI api, string entityId)
	{
		IEntitySource candidate;
		int selectedCount = api.GetSelectedEntitiesCount();
		int editorCount = api.GetEditorEntityCount();

		for (int i; i < selectedCount; i++)
		{
			candidate = api.GetSelectedEntity(i);
			if (candidate && candidate.GetID().ToString() == entityId)
			{
				return candidate;
			}
		}

		for (int i; i < editorCount; i++)
		{
			candidate = api.GetEditorEntity(i);
			if (candidate && candidate.GetID().ToString() == entityId)
			{
				return candidate;
			}
		}

		return null;
	}
	bool IsAncestor(IEntitySource entity, IEntitySource candidateParent)
	{
		IEntitySource current = candidateParent;
		while (current)
		{
			if (current == entity) return true;
			current = current.GetParent();
		}

		return false;
	}

	bool Setup(WorldEditorAPI api, RST_WorkbenchEntityMutationResponse response)
	{
		if (!api)
		{
			response.status = "world-editor-api-unavailable";
			return false;
		}
		if (!api.GetWorld())
		{
			response.status = "world-unavailable";
			return false;
		}
		if (api.IsPrefabEditMode())
		{
			response.status = "prefab-edit-mode";
			return false;
		}
		if (api.IsDoingEditAction())
		{
			response.status = "editor-action-active";
			return false;
		}

		return true;
	}

	void Record(WorldEditorAPI api, RST_WorkbenchEntityMutationResponse response, IEntitySource entity)
	{
		IEntity runtimeEntity;
		vector p;
		string resourceName;
		string name;
		string subSceneName;
		string layerName;

		if (!entity) return;

		runtimeEntity = api.SourceToEntity(entity);
		if (runtimeEntity)
			p = runtimeEntity.GetOrigin();
		else
		{
			p = vector.Zero;
			entity.Get("coords", p);
		}

		resourceName = string.Format("%1", entity.GetResourceName());
		name = entity.GetName();
		subSceneName = api.GetWorld().GetSubSceneName(entity.GetSubScene());
		layerName = api.GetEntitySubsceneLayer(entity.GetSubScene(), entity);
		if (name == resourceName) name = string.Empty;

		resourceName.Replace("|", "/");
		resourceName.Replace(";", "/");
		name.Replace("|", "/");
		name.Replace(";", "/");
		subSceneName.Replace("|", "/");
		subSceneName.Replace(";", "/");
		layerName.Replace("|", "/");
		layerName.Replace(";", "/");

		response.entity = string.Format(
			"%1|%2|%3|%4|%5|%6|%7",
			entity.GetID().ToString(),
			entity.GetClassName(),
			entity.GetSubScene(),
			entity.GetLayerID(),
			p[0],
			p[1],
			p[2]) + "|" + resourceName + "|" + name + "|" + subSceneName + "|" + layerName;
	}

	string PropertyTypeName(DataVarType dataType)
	{
		switch (dataType)
		{
			case DataVarType.BOOLEAN: return "bool";
			case DataVarType.INTEGER: return "integer";
			case DataVarType.SCALAR: return "float";
			case DataVarType.VECTOR3: return "vector";
			case DataVarType.STRING: return "string";
			case DataVarType.RESOURCE_NAME: return "resource";
		}

		return string.Empty;
	}

	bool ReadPropertyValue(BaseContainer container, string name, DataVarType dataType, out string value)
	{
		bool boolValue;
		int integerValue;
		float floatValue;
		vector vectorValue;
		string stringValue;

		switch (dataType)
		{
			case DataVarType.BOOLEAN:
				if (!container.Get(name, boolValue)) return false;
				if (boolValue) value = "1";
				else value = "0";
				return true;
			case DataVarType.INTEGER:
				if (!container.Get(name, integerValue)) return false;
				value = integerValue.ToString();
				return true;
			case DataVarType.SCALAR:
				if (!container.Get(name, floatValue)) return false;
				value = floatValue.ToString();
				return true;
			case DataVarType.VECTOR3:
				if (!container.Get(name, vectorValue)) return false;
				value = string.Format("%1 %2 %3", vectorValue[0], vectorValue[1], vectorValue[2]);
				return true;
			case DataVarType.STRING:
			case DataVarType.RESOURCE_NAME:
				if (!container.Get(name, stringValue)) return false;
				value = stringValue;
				return true;
		}

		return false;
	}

	RST_WorkbenchEntityMutationResponse Response()
	{
		RST_WorkbenchEntityMutationResponse response = new RST_WorkbenchEntityMutationResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		response.activeLayerId = -1;
		return response;
	}
}

class RST_WorkbenchCreateEntity : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchEntityMutationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!Setup(api, response)) return response;

		response.activeLayerId = api.GetCurrentEntityLayerId();
		if (r.resourceName.IsEmpty() || r.subScene < 0 || r.layerId < 0
			|| api.IsEntityLayerLockedHierarchy(api.GetCurrentSubScene(), r.layerId))
		{
			response.status = "invalid-create-target";
			return response;
		}

		if (r.targetIsResource)
		{
			ResourceName prefab = r.resourceName;
			Resource resource = Resource.Load(prefab);
			if (!resource || !resource.IsValid())
			{
				response.status = "resource-load-failed";
				return response;
			}
		}

		if (!api.BeginEntityAction("Reforger Script Tools: create entity"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		IEntitySource entity = api.CreateEntity(
			r.resourceName,
			r.name,
			r.layerId,
			null,
			Vector(r.x, r.y, r.z),
			Vector(r.pitch, r.yaw, r.roll));
		api.EndEntityAction("Reforger Script Tools: create entity");
		if (!entity)
		{
			response.status = "create-rejected";
			return response;
		}

		Record(api, response, entity);
		response.activeLayerId = entity.GetLayerID();
		if (entity.GetSubScene() != r.subScene || entity.GetLayerID() != r.layerId)
			response.status = "target-mismatch";
		else
			response.status = "created";
		return response;
	}
}
class RST_WorkbenchRenameEntity : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchEntityMutationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!Setup(api, response)) return response;

		IEntitySource entity = Find(api, r.entityId);
		if (!entity)
		{
			response.status = "entity-not-found";
			return response;
		}

		if (api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID())
			|| !api.BeginEntityAction("Reforger Script Tools: rename entity"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		bool changed = api.RenameEntity(entity, r.name);
		api.EndEntityAction("Reforger Script Tools: rename entity");
		if (!changed)
		{
			response.status = "mutation-rejected";
			return response;
		}

		Record(api, response, entity);
		response.status = "renamed";
		return response;
	}
}
class RST_WorkbenchMoveEntity : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchEntityMutationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!Setup(api, response)) return response;

		IEntitySource entity = Find(api, r.entityId);
		if (!entity)
		{
			response.status = "entity-not-found";
			return response;
		}

		if (api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID())
			|| !api.BeginEntityAction("Reforger Script Tools: move entity"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		if (!api.SetVariableValue(entity, null, "coords", string.Format("%1 %2 %3", r.x, r.y, r.z)))
		{
			api.EndEntityAction("Reforger Script Tools: move entity");
			response.status = "mutation-rejected";
			return response;
		}

		api.EndEntityAction("Reforger Script Tools: move entity");
		Record(api, response, entity);
		response.status = "moved";
		return response;
	}
}
class RST_WorkbenchRotateEntity : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchEntityMutationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!Setup(api, response)) return response;

		IEntitySource entity = Find(api, r.entityId);
		if (!entity)
		{
			response.status = "entity-not-found";
			return response;
		}

		if (api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID()))
		{
			response.status = "mutation-rejected";
			return response;
		}
		if (!api.BeginEntityAction("Reforger Script Tools: rotate entity"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		bool changed = api.SetVariableValue(
			entity,
			null,
			"angles",
			string.Format("%1 %2 %3", r.pitch, r.yaw, r.roll));
		api.EndEntityAction("Reforger Script Tools: rotate entity");
		if (!changed)
		{
			response.status = "mutation-rejected";
			return response;
		}

		Record(api, response, entity);
		response.status = "rotated";
		return response;
	}
}
class RST_WorkbenchReparentEntity : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchEntityMutationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!Setup(api, response)) return response;

		IEntitySource entity = Find(api, r.entityId);
		if (!entity)
		{
			response.status = "entity-not-found";
			return response;
		}

		IEntitySource parent = Find(api, r.parentEntityId);
		if (!parent)
		{
			response.status = "parent-entity-not-found";
			return response;
		}

		if (entity == parent || entity.GetSubScene() != parent.GetSubScene()
			|| IsAncestor(entity, parent)
			|| api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID())
			|| api.IsEntityLayerLockedHierarchy(parent.GetSubScene(), parent.GetLayerID())
			|| !api.BeginEntityAction("Reforger Script Tools: reparent entity"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		bool changed = api.ParentEntity(parent, entity, true);
		api.EndEntityAction("Reforger Script Tools: reparent entity");
		if (!changed)
		{
			response.status = "mutation-rejected";
			return response;
		}

		Record(api, response, entity);
		response.status = "reparented";
		return response;
	}
}
class RST_WorkbenchDuplicateEntity : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchEntityMutationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!Setup(api, response)) return response;

		IEntitySource entity = Find(api, r.entityId);
		if (!entity)
		{
			response.status = "entity-not-found";
			return response;
		}

		if (api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID())
			|| !api.BeginEntityAction("Reforger Script Tools: duplicate entity"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		IEntitySource clone = api.CreateClonedEntity(entity, r.name, null, false);
		bool moved = clone && api.SetVariableValue(
			clone,
			null,
			"coords",
			string.Format("%1 %2 %3", r.x, r.y, r.z));
		api.EndEntityAction("Reforger Script Tools: duplicate entity");
		if (!clone || !moved)
		{
			response.status = "mutation-rejected";
			return response;
		}

		Record(api, response, clone);
		response.status = "duplicated";
		return response;
	}
}
class RST_WorkbenchDeleteEntity : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchEntityMutationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!Setup(api, response)) return response;

		IEntitySource entity = Find(api, r.entityId);
		if (!entity)
		{
			response.status = "entity-not-found";
			return response;
		}

		Record(api, response, entity);
		if (!r.confirm)
		{
			response.status = "confirmation-required";
			return response;
		}

		if (api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID())
			|| !api.BeginEntityAction("Reforger Script Tools: delete entity"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		bool deleted = api.DeleteEntity(entity);
		api.EndEntityAction("Reforger Script Tools: delete entity");
		response.entity = string.Empty;
		if (deleted && !Find(api, r.entityId))
			response.status = "deleted";
		else
			response.status = "mutation-rejected";
		return response;
	}
}
#endif
"#;

const BRIDGE_SHAPE_POINTS_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchShapePointsRequest : JsonApiStruct
{
	string entityId;
	string operation;
	int index;
	int count;
	string points;

	void RST_WorkbenchShapePointsRequest() { RegAll(); }
}
class RST_WorkbenchShapePointsResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string status;
	string entity;
	string shapeClass;
	bool closed;
	string points;

	void RST_WorkbenchShapePointsResponse() { RegAll(); }
}
class RST_WorkbenchShapePointsBase : NetApiHandler
{
	IEntitySource Find(WorldEditorAPI api, string entityId)
	{
		for (int i, count = api.GetEditorEntityCount(); i < count; i++)
		{
			IEntitySource candidate = api.GetEditorEntity(i);
			if (candidate && candidate.GetID().ToString() == entityId) return candidate;
		}
		return null;
	}
	RST_WorkbenchShapePointsResponse Response()
	{
		RST_WorkbenchShapePointsResponse response = new RST_WorkbenchShapePointsResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		return response;
	}
	bool Setup(WorldEditorAPI api, RST_WorkbenchShapePointsResponse response)
	{
		if (!api) { response.status = "world-editor-api-unavailable"; return false; }
		if (!api.GetWorld()) { response.status = "world-unavailable"; return false; }
		if (api.IsPrefabEditMode()) { response.status = "prefab-edit-mode"; return false; }
		if (api.IsDoingEditAction()) { response.status = "editor-action-active"; return false; }
		return true;
	}
	bool ResolveShape(WorldEditorAPI api, string entityId, RST_WorkbenchShapePointsResponse response, out IEntitySource source, out ShapeEntity shape)
	{
		source = Find(api, entityId);
		if (!source) { response.status = "entity-not-found"; return false; }
		shape = ShapeEntity.Cast(api.SourceToEntity(source));
		if (!shape) { response.status = "entity-not-shape"; return false; }
		return true;
	}
	void Record(WorldEditorAPI api, IEntitySource source, ShapeEntity shape, RST_WorkbenchShapePointsResponse response)
	{
		vector origin = shape.GetOrigin();
		string resourceName = string.Format("%1", source.GetResourceName());
		string name = source.GetName();
		string subSceneName = api.GetWorld().GetSubSceneName(source.GetSubScene());
		string layerName = api.GetEntitySubsceneLayer(source.GetSubScene(), source);
		if (name == resourceName) name = string.Empty;
		resourceName.Replace("|", "/"); resourceName.Replace(";", "/");
		name.Replace("|", "/"); name.Replace(";", "/");
		subSceneName.Replace("|", "/"); subSceneName.Replace(";", "/");
		layerName.Replace("|", "/"); layerName.Replace(";", "/");
		response.entity = string.Format("%1|%2|%3|%4|%5|%6|%7", source.GetID().ToString(), source.GetClassName(), source.GetSubScene(), source.GetLayerID(), origin[0], origin[1], origin[2]) + "|" + resourceName + "|" + name + "|" + subSceneName + "|" + layerName;
		response.shapeClass = source.GetClassName();
		response.closed = shape.IsClosed();
		array<vector> positions = {};
		shape.GetPointsPositions(positions);
		foreach (vector point : positions)
		{
			if (!response.points.IsEmpty()) response.points += ";";
			response.points += string.Format("%1|%2|%3", point[0], point[1], point[2]);
		}
	}
	bool DecodePoints(string encoded, out array<vector> decoded)
	{
		if (encoded.IsEmpty()) return true;
		array<string> records = {};
		encoded.Split(";", records, true);
		foreach (string record : records)
		{
			array<string> fields = {};
			record.Split(",", fields, false);
			if (fields.Count() != 3) return false;
			decoded.Insert(Vector(fields[0].ToFloat(), fields[1].ToFloat(), fields[2].ToFloat()));
		}
		return true;
	}
}
class RST_WorkbenchShapePoints : RST_WorkbenchShapePointsBase
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchShapePointsRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchShapePointsRequest r = RST_WorkbenchShapePointsRequest.Cast(request); RST_WorkbenchShapePointsResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi(); if (!Setup(api, response)) return response;
		IEntitySource source; ShapeEntity shape; if (!ResolveShape(api, r.entityId, response, source, shape)) return response;
		Record(api, source, shape, response); response.status = "available"; return response;
	}
}
class RST_WorkbenchEditShapePoints : RST_WorkbenchShapePointsBase
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchShapePointsRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchShapePointsRequest r = RST_WorkbenchShapePointsRequest.Cast(request); RST_WorkbenchShapePointsResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi(); if (!Setup(api, response)) return response;
		IEntitySource source; ShapeEntity shape; if (!ResolveShape(api, r.entityId, response, source, shape)) return response;
		if (api.IsEntityLayerLockedHierarchy(source.GetSubScene(), source.GetLayerID())) { response.status = "mutation-rejected"; return response; }
		array<vector> current = {}; shape.GetPointsPositions(current);
		array<vector> supplied = {}; if (!DecodePoints(r.points, supplied)) { response.status = "invalid-points"; return response; }
		if (r.operation == "set") current = supplied;
		else if (r.operation == "insert")
		{
			if (supplied.IsEmpty() || r.index < 0 || r.index > current.Count()) { response.status = "invalid-point-edit"; return response; }
			foreach (vector point : supplied) { if (r.index == current.Count()) current.Insert(point); else current.InsertAt(point, r.index); r.index++; }
		}
		else if (r.operation == "delete")
		{
			if (r.count < 1 || r.index < 0 || r.index >= current.Count() || r.count > current.Count() - r.index) { response.status = "invalid-point-edit"; return response; }
			for (int i; i < r.count; i++) current.RemoveOrdered(r.index);
		}
		else { response.status = "invalid-point-edit"; return response; }
		if (!api.BeginEntityAction("Reforger Script Tools: edit shape points")) { response.status = "mutation-rejected"; return response; }
		shape.SetPoints(current, source);
		api.EndEntityAction("Reforger Script Tools: edit shape points");
		if (!ResolveShape(api, r.entityId, response, source, shape)) return response;
		Record(api, source, shape, response); response.status = "points-updated"; return response;
	}
}
#endif
"#;

const BRIDGE_SHAPE_GEOMETRY_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchShapeGeometryRequest : JsonApiStruct
{
	string entityId; string operation; string fromSpace; string toSpace; string space; string points;
	string transformOperation; float offsetX; float offsetY; float offsetZ; float pivotX; float pivotY; float pivotZ;
	float degrees; float scaleX; float scaleY; float scaleZ; string mirrorAxis; float spacingMeters;
	void RST_WorkbenchShapeGeometryRequest() { RegAll(); }
}
class RST_WorkbenchShapeGeometryResponse : JsonApiStruct
{
	string bridgeVersion; int protocolVersion; string status; string entity; string shapeClass; bool closed; string points;
	string fromSpace; string toSpace; float spacingMeters; int originalPointCount; int resultPointCount; float pathLength; int skippedZeroLengthSegments;
	void RST_WorkbenchShapeGeometryResponse() { RegAll(); }
}
class RST_WorkbenchShapeGeometry : NetApiHandler
{
	IEntitySource Find(WorldEditorAPI api, string entityId)
	{
		for (int i, count = api.GetEditorEntityCount(); i < count; i++) { IEntitySource candidate = api.GetEditorEntity(i); if (candidate && candidate.GetID().ToString() == entityId) return candidate; }
		return null;
	}
	RST_WorkbenchShapeGeometryResponse Response() { RST_WorkbenchShapeGeometryResponse response = new RST_WorkbenchShapeGeometryResponse(); response.bridgeVersion = "1.51.0"; response.protocolVersion = 1; return response; }
	bool Setup(WorldEditorAPI api, RST_WorkbenchShapeGeometryResponse response)
	{
		if (!api) { response.status = "world-editor-api-unavailable"; return false; }
		if (!api.GetWorld()) { response.status = "world-unavailable"; return false; }
		if (api.IsPrefabEditMode()) { response.status = "prefab-edit-mode"; return false; }
		if (api.IsDoingEditAction()) { response.status = "editor-action-active"; return false; }
		return true;
	}
	bool Resolve(WorldEditorAPI api, string entityId, RST_WorkbenchShapeGeometryResponse response, out IEntitySource source, out ShapeEntity shape)
	{
		source = Find(api, entityId); if (!source) { response.status = "entity-not-found"; return false; }
		shape = ShapeEntity.Cast(api.SourceToEntity(source)); if (!shape) { response.status = "entity-not-shape"; return false; }
		if (source.GetClassName() != "PolylineShapeEntity" && source.GetClassName() != "SplineShapeEntity") { response.status = "unsupported-shape-class"; return false; }
		return true;
	}
	void Record(WorldEditorAPI api, IEntitySource source, ShapeEntity shape, RST_WorkbenchShapeGeometryResponse response)
	{
		vector origin = shape.GetOrigin(); string resourceName = string.Format("%1", source.GetResourceName()); string name = source.GetName(); string subSceneName = api.GetWorld().GetSubSceneName(source.GetSubScene()); string layerName = api.GetEntitySubsceneLayer(source.GetSubScene(), source);
		if (name == resourceName) name = string.Empty; resourceName.Replace("|", "/"); resourceName.Replace(";", "/"); name.Replace("|", "/"); name.Replace(";", "/"); subSceneName.Replace("|", "/"); subSceneName.Replace(";", "/"); layerName.Replace("|", "/"); layerName.Replace(";", "/");
		response.entity = string.Format("%1|%2|%3|%4|%5|%6|%7", source.GetID().ToString(), source.GetClassName(), source.GetSubScene(), source.GetLayerID(), origin[0], origin[1], origin[2]) + "|" + resourceName + "|" + name + "|" + subSceneName + "|" + layerName; response.shapeClass = source.GetClassName(); response.closed = shape.IsClosed();
		array<vector> positions = {}; shape.GetPointsPositions(positions); Encode(positions, response.points);
	}
	void Encode(array<vector> values, out string encoded) { encoded = string.Empty; foreach (vector point : values) { if (!encoded.IsEmpty()) encoded += ";"; encoded += string.Format("%1|%2|%3", point[0], point[1], point[2]); } }
	bool Decode(string encoded, out array<vector> decoded)
	{
		if (encoded.IsEmpty()) return true; array<string> records = {}; encoded.Split(";", records, true); if (records.Count() > 4096) return false;
		foreach (string record : records) { array<string> fields = {}; record.Split(",", fields, false); if (fields.Count() != 3) return false; decoded.Insert(Vector(fields[0].ToFloat(), fields[1].ToFloat(), fields[2].ToFloat())); }
		return true;
	}
	float Distance(vector a, vector b) { float x = b[0] - a[0]; float y = b[1] - a[1]; float z = b[2] - a[2]; return Math.Sqrt(x * x + y * y + z * z); }
	vector Interpolate(vector a, vector b, float t) { return Vector(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t); }
	void ToSpace(ShapeEntity shape, array<vector> values, string fromSpace, string toSpace)
	{
		if (fromSpace == toSpace) return;
		for (int i; i < values.Count(); i++) { if (fromSpace == "local") values[i] = shape.CoordToParent(values[i]); else values[i] = shape.CoordToLocal(values[i]); }
	}
	bool Commit(WorldEditorAPI api, string entityId, IEntitySource source, ShapeEntity shape, array<vector> points, RST_WorkbenchShapeGeometryResponse response, string label)
	{
		if (api.IsEntityLayerLockedHierarchy(source.GetSubScene(), source.GetLayerID())) { response.status = "mutation-rejected"; return false; }
		if (!api.BeginEntityAction(label)) { response.status = "mutation-rejected"; return false; } shape.SetPoints(points, source); api.EndEntityAction(label);
		if (!Resolve(api, entityId, response, source, shape)) return false; Record(api, source, shape, response); return true;
	}
	override JsonApiStruct GetRequest() { return new RST_WorkbenchShapeGeometryRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchShapeGeometryRequest r = RST_WorkbenchShapeGeometryRequest.Cast(request); RST_WorkbenchShapeGeometryResponse response = Response(); WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor) { response.status = "world-editor-unavailable"; return response; } WorldEditorAPI api = editor.GetApi(); if (!Setup(api, response)) return response;
		IEntitySource source; ShapeEntity shape; if (!Resolve(api, r.entityId, response, source, shape)) return response;
		if (r.operation == "convert")
		{
			if ((r.fromSpace != "local" && r.fromSpace != "world") || (r.toSpace != "local" && r.toSpace != "world")) { response.status = "invalid-input"; return response; }
			array<vector> responsePoints = {}; if (!Decode(r.points, responsePoints)) { response.status = "invalid-input"; return response; } ToSpace(shape, responsePoints, r.fromSpace, r.toSpace); Encode(responsePoints, response.points); Record(api, source, shape, response); Encode(responsePoints, response.points); response.fromSpace = r.fromSpace; response.toSpace = r.toSpace; response.status = "converted"; return response;
		}
		array<vector> points = {}; shape.GetPointsPositions(points);
		if (r.operation == "transform")
		{
			if (r.space != "local" && r.space != "world") { response.status = "invalid-input"; return response; }
			if (r.transformOperation == "reverse") { array<vector> reversed = {}; for (int i = points.Count() - 1; i >= 0; i--) reversed.Insert(points[i]); points = reversed; }
			else
			{
				ToSpace(shape, points, "local", r.space); float radians = r.degrees * Math.PI / 180.0; float sine = Math.Sin(radians); float cosine = Math.Cos(radians);
				if (r.transformOperation == "scale" && (r.scaleX == 0 || r.scaleY == 0 || r.scaleZ == 0)) { response.status = "invalid-input"; return response; }
				if (r.transformOperation == "mirror" && r.mirrorAxis != "x" && r.mirrorAxis != "y" && r.mirrorAxis != "z") { response.status = "invalid-input"; return response; }
				if (r.transformOperation != "translate" && r.transformOperation != "rotateXZ" && r.transformOperation != "scale" && r.transformOperation != "mirror") { response.status = "invalid-input"; return response; }
				for (int i; i < points.Count(); i++) { vector p = points[i]; if (r.transformOperation == "translate") p = p + Vector(r.offsetX, r.offsetY, r.offsetZ); else { p = p - Vector(r.pivotX, r.pivotY, r.pivotZ); if (r.transformOperation == "rotateXZ") p = Vector(p[0] * cosine - p[2] * sine, p[1], p[0] * sine + p[2] * cosine); else if (r.transformOperation == "scale") p = Vector(p[0] * r.scaleX, p[1] * r.scaleY, p[2] * r.scaleZ); else if (r.mirrorAxis == "x") p[0] = -p[0]; else if (r.mirrorAxis == "y") p[1] = -p[1]; else p[2] = -p[2]; p = p + Vector(r.pivotX, r.pivotY, r.pivotZ); } points[i] = p; }
				ToSpace(shape, points, r.space, "local");
			}
			if (!Commit(api, r.entityId, source, shape, points, response, "Reforger Script Tools: transform shape points")) return response; response.status = "points-transformed"; return response;
		}
		if (r.operation != "resample") { response.status = "invalid-input"; return response; }
		if (source.GetClassName() != "PolylineShapeEntity") { response.status = "entity-not-polyline"; return response; }
		if (r.space != "local" && r.space != "world" || r.spacingMeters <= 0) { response.status = "invalid-input"; return response; }
		int originalCount = points.Count(); if (originalCount < 2) { response.status = "resample-rejected"; return response; } ToSpace(shape, points, "local", r.space); array<vector> sampled = {}; sampled.Insert(points[0]); float total = 0; int skipped = 0; int segments = points.Count() - 1; if (shape.IsClosed()) segments++;
		for (int i; i < segments; i++) { vector a = points[i]; vector b = points[(i + 1) % points.Count()]; float length = Distance(a, b); if (length <= 0.00001) { skipped++; continue; } total += length; }
		if (total <= 0.00001) { response.status = "resample-rejected"; return response; } float next = r.spacingMeters; float travelled = 0;
		for (int i; i < segments; i++) { vector a = points[i]; vector b = points[(i + 1) % points.Count()]; float length = Distance(a, b); if (length <= 0.00001) continue; while (next < travelled + length) { if (sampled.Count() >= 4096) { response.status = "resample-too-many-points"; return response; } sampled.Insert(Interpolate(a, b, (next - travelled) / length)); next += r.spacingMeters; } travelled += length; }
		if (!shape.IsClosed()) { if (sampled.Count() >= 4096) { response.status = "resample-too-many-points"; return response; } sampled.Insert(points[points.Count() - 1]); }
		ToSpace(shape, sampled, r.space, "local"); if (!Commit(api, r.entityId, source, shape, sampled, response, "Reforger Script Tools: resample polyline")) return response; response.spacingMeters = r.spacingMeters; response.originalPointCount = originalCount; response.resultPointCount = sampled.Count(); response.pathLength = total; response.skippedZeroLengthSegments = skipped; response.status = "polyline-resampled"; return response;
	}
}
#endif
"#;

const BRIDGE_COMPONENTS_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchComponentsRequest : JsonApiStruct { string entityId; string componentId; string className; string propertyName; string expectedValue; string value; bool confirm; void RST_WorkbenchComponentsRequest() { RegAll(); } }
class RST_WorkbenchComponentsResponse : JsonApiStruct { string bridgeVersion; int protocolVersion; string status; string entity; string components; string properties; void RST_WorkbenchComponentsResponse() { RegAll(); } }
class RST_WorkbenchComponentsBase : NetApiHandler
{
	IEntitySource Find(WorldEditorAPI api, string id) { for (int i = 0, count = api.GetEditorEntityCount(); i < count; i++) { IEntitySource candidate = api.GetEditorEntity(i); if (candidate && candidate.GetID().ToString() == id) return candidate; } return null; }
	RST_WorkbenchComponentsResponse Response() { RST_WorkbenchComponentsResponse response = new RST_WorkbenchComponentsResponse(); response.bridgeVersion = "1.51.0"; response.protocolVersion = 1; return response; }
	// Workbench exposes authored direct components through the `components` container in
	// some editor contexts, even when IEntitySource.GetComponentCount() is zero.
	int ComponentCount(IEntitySource entity) { int count = entity.GetComponentCount(); if (count == 0) { ref BaseContainerList components = entity.GetObjectArray("components"); if (components) count = components.Count(); } return count; }
	IEntityComponentSource ComponentAt(IEntitySource entity, int index) { IEntityComponentSource component; int count = entity.GetComponentCount(); if (count > 0) component = entity.GetComponent(index); else { ref BaseContainerList components = entity.GetObjectArray("components"); if (components) component = IEntityComponentSource.Cast(components.Get(index)); } return component; }
	void List(IEntitySource entity, RST_WorkbenchComponentsResponse response)
	{
		int count = ComponentCount(entity);
		for (int i; i < count; i++)
		{
			IEntityComponentSource component = ComponentAt(entity, i);
			if (!component) continue;
			if (!response.components.IsEmpty()) response.components += ";";
			response.components += string.Format("%1|%2", i, component.GetClassName());
		}
	}
	string SupportedPropertyType(DataVarType dataType)
	{
		switch (dataType)
		{
			case DataVarType.BOOLEAN: return "bool";
			case DataVarType.INTEGER: return "integer";
			case DataVarType.SCALAR: return "float";
			case DataVarType.VECTOR3: return "vector";
			case DataVarType.STRING: return "string";
			case DataVarType.RESOURCE_NAME: return "resource";
		}

		return string.Empty;
	}

	string PropertyOrigin(IEntityComponentSource component, string name)
	{
		if (component.IsVariableSetDirectly(name))
			return "direct";
		for (BaseContainer ancestor = component.GetAncestor(); ancestor; ancestor = ancestor.GetAncestor())
		{
			if (ancestor.IsVariableSetDirectly(name))
				return "inherited";
		}
		return "default";
	}

	bool ReadPropertyValue(IEntityComponentSource component, string name, DataVarType dataType, out string value)
	{
		bool boolValue;
		int integerValue;
		float floatValue;
		vector vectorValue;
		string stringValue;

		switch (dataType)
		{
			case DataVarType.BOOLEAN:
				if (!component.Get(name, boolValue)) return false;
				if (boolValue)
					value = "1";
				else
					value = "0";
				return true;
			case DataVarType.INTEGER:
				if (!component.Get(name, integerValue)) return false;
				value = integerValue.ToString();
				return true;
			case DataVarType.SCALAR:
				if (!component.Get(name, floatValue)) return false;
				value = floatValue.ToString();
				return true;
			case DataVarType.VECTOR3:
				if (!component.Get(name, vectorValue)) return false;
				value = string.Format("%1 %2 %3", vectorValue[0], vectorValue[1], vectorValue[2]);
				return true;
			case DataVarType.STRING:
			case DataVarType.RESOURCE_NAME:
				if (!component.Get(name, stringValue)) return false;
				value = stringValue;
				return true;
		}

		return false;
	}

	void ListProperties(IEntityComponentSource component, RST_WorkbenchComponentsResponse response)
	{
		for (int i = 0, count = component.GetNumVars(); i < count; i++)
		{
			string name = component.GetVarName(i);
			DataVarType dataType = component.GetDataVarType(i);
			string typeName = SupportedPropertyType(dataType);
			string value;
			string directlySet;
			if (typeName.IsEmpty() || !ReadPropertyValue(component, name, dataType, value)) continue;
			if (component.IsVariableSetDirectly(name))
				directlySet = "1";
			else
				directlySet = "0";

			value.Replace("|", "/");
			value.Replace(";", "/");
			if (!response.properties.IsEmpty()) response.properties += ";";
			response.properties += string.Format(
				"%1|%2|%3|%4|%5",
				name,
				typeName,
				value,
				directlySet,
				PropertyOrigin(component, name));
		}
	}
	IEntityComponentSource FindComponent(IEntitySource entity, string componentId) { for (int i = 0, count = ComponentCount(entity); i < count; i++) { IEntityComponentSource component = ComponentAt(entity, i); if (component && componentId == string.Format("cmp1:%1:%2", i, component.GetClassName())) return component; } return null; }
}
class RST_WorkbenchListComponents : RST_WorkbenchComponentsBase
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchComponentsRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request) { RST_WorkbenchComponentsRequest r = RST_WorkbenchComponentsRequest.Cast(request); RST_WorkbenchComponentsResponse response = Response(); WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor || !editor.GetApi()) { response.status = "world-editor-unavailable"; return response; } IEntitySource entity = Find(editor.GetApi(), r.entityId); if (!entity) { response.status = "entity-not-found"; return response; } List(entity, response); response.status = "available"; return response; }
}
class RST_WorkbenchInspectComponent : RST_WorkbenchComponentsBase
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchComponentsRequest(); }
override JsonApiStruct GetResponse(JsonApiStruct request) { RST_WorkbenchComponentsRequest r = RST_WorkbenchComponentsRequest.Cast(request); RST_WorkbenchComponentsResponse response = Response(); WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor || !editor.GetApi()) { response.status = "world-editor-unavailable"; return response; } IEntitySource entity = Find(editor.GetApi(), r.entityId); if (!entity) { response.status = "entity-not-found"; return response; } List(entity, response); for (int i = 0, count = ComponentCount(entity); i < count; i++) { IEntityComponentSource component = ComponentAt(entity, i); if (component && r.componentId == string.Format("cmp1:%1:%2", i, component.GetClassName())) { ListProperties(component, response); response.status = "available"; return response; } } response.status = "component-not-found"; return response; }
}
class RST_WorkbenchAddComponent : RST_WorkbenchComponentsBase
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchComponentsRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request) { RST_WorkbenchComponentsRequest r = RST_WorkbenchComponentsRequest.Cast(request); RST_WorkbenchComponentsResponse response = Response(); WorldEditor editor = Workbench.GetModule(WorldEditor); WorldEditorAPI api; IEntitySource entity; typename componentType; if (!editor) { response.status = "world-editor-unavailable"; return response; } api = editor.GetApi(); if (!api || api.IsPrefabEditMode() || api.IsDoingEditAction()) { response.status = "world-editor-unavailable"; return response; } entity = Find(api, r.entityId); componentType = r.className.ToType(); if (!entity || r.className.IsEmpty() || !componentType || !componentType.IsInherited(ScriptComponent) || api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID()) || !api.BeginEntityAction("Reforger Script Tools: add component")) { response.status = "mutation-rejected"; return response; } if (!api.CreateComponent(entity, r.className)) { api.EndEntityAction("Reforger Script Tools: add component"); response.status = "mutation-rejected"; return response; } api.EndEntityAction("Reforger Script Tools: add component"); List(entity, response); response.status = "added"; return response; }
}
class RST_WorkbenchRemoveComponent : RST_WorkbenchComponentsBase
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchComponentsRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request) { RST_WorkbenchComponentsRequest r = RST_WorkbenchComponentsRequest.Cast(request); RST_WorkbenchComponentsResponse response = Response(); WorldEditor editor = Workbench.GetModule(WorldEditor); WorldEditorAPI api; IEntitySource entity; IEntityComponentSource component; if (!editor) { response.status = "world-editor-unavailable"; return response; } api = editor.GetApi(); if (!api || api.IsPrefabEditMode() || api.IsDoingEditAction()) { response.status = "world-editor-unavailable"; return response; } entity = Find(api, r.entityId); if (!entity) { response.status = "component-not-found"; return response; } component = FindComponent(entity, r.componentId); if (!component) { response.status = "component-not-found"; return response; } if (!r.confirm) { List(entity, response); response.status = "confirmation-required"; return response; } if (api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID()) || !api.BeginEntityAction("Reforger Script Tools: remove component")) { response.status = "mutation-rejected"; return response; } if (!api.DeleteComponent(entity, component)) { api.EndEntityAction("Reforger Script Tools: remove component"); response.status = "mutation-rejected"; return response; } api.EndEntityAction("Reforger Script Tools: remove component"); List(entity, response); response.status = "removed"; return response; }
}
class RST_WorkbenchSetComponentProperty : RST_WorkbenchComponentsBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchComponentsRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchComponentsRequest r = RST_WorkbenchComponentsRequest.Cast(request);
		RST_WorkbenchComponentsResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!api || api.IsPrefabEditMode() || api.IsDoingEditAction())
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		IEntitySource entity = Find(api, r.entityId);
		if (!entity || api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID()))
		{
			response.status = "mutation-rejected";
			return response;
		}

		IEntityComponentSource component = FindComponent(entity, r.componentId);
		if (!component)
		{
			response.status = "property-not-found";
			return response;
		}

		int propertyIndex = component.GetVarIndex(r.propertyName);
		if (r.propertyName.IsEmpty() || propertyIndex < 0)
		{
			response.status = "property-not-found";
			return response;
		}

		string observedValue;
		DataVarType propertyDataType = component.GetDataVarType(propertyIndex);
		if (!ReadPropertyValue(component, r.propertyName, propertyDataType, observedValue)
			|| observedValue != r.expectedValue)
		{
			response.status = "stale-property-observation";
			return response;
		}

		int componentIndex = -1;
		for (int i = 0, count = ComponentCount(entity); i < count; i++)
		{
			if (ComponentAt(entity, i) == component)
			{
				componentIndex = i;
				break;
			}
		}
		if (componentIndex < 0 || !api.BeginEntityAction("Reforger Script Tools: set component property"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		if (!api.SetVariableValue(
			entity,
			{ new ContainerIdPathEntry("components", componentIndex) },
			r.propertyName,
			r.value))
		{
			api.EndEntityAction("Reforger Script Tools: set component property");
			response.status = "mutation-rejected";
			return response;
		}

		api.EndEntityAction("Reforger Script Tools: set component property");
		List(entity, response);
		ListProperties(component, response);
		response.status = "property-set";
		return response;
	}
}
class RST_WorkbenchSetPrefabComponentProperty : RST_WorkbenchComponentsBase
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchComponentsRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchComponentsRequest r = RST_WorkbenchComponentsRequest.Cast(request); RST_WorkbenchComponentsResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor || !editor.GetApi()) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi(); if (!api.IsPrefabEditMode() || api.IsDoingEditAction()) { response.status = "prefab-edit-unavailable"; return response; }
		IEntitySource entity = Find(api, r.entityId); IEntityComponentSource component;
		if (entity) component = FindComponent(entity, r.componentId);
		int index = -1; if (component) index = component.GetVarIndex(r.propertyName); string observed;
		if (!component || index < 0 || !ReadPropertyValue(component, r.propertyName, component.GetDataVarType(index), observed) || observed != r.expectedValue) { response.status = "stale-property-observation"; return response; }
		int componentIndex = -1; for (int i, count = ComponentCount(entity); i < count; i++) if (ComponentAt(entity, i) == component) { componentIndex = i; break; }
		if (componentIndex < 0 || !api.BeginEntityAction("Reforger Script Tools: set prefab component property") || !api.SetVariableValue(entity, { new ContainerIdPathEntry("components", componentIndex) }, r.propertyName, r.value)) { api.EndEntityAction("Reforger Script Tools: set prefab component property"); response.status = "mutation-rejected"; return response; }
		api.EndEntityAction("Reforger Script Tools: set prefab component property"); List(entity, response); ListProperties(component, response); response.status = "prefab-component-property-set"; return response;
	}
}
#endif
"#;

const BRIDGE_PROPERTIES_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchPropertiesRequest : JsonApiStruct
{
	string entityId;
	string propertyName;
	string expectedValue;
	string value;

	void RST_WorkbenchPropertiesRequest()
	{
		RegAll();
	}
}
class RST_WorkbenchPropertiesResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string status;
	string properties;

	void RST_WorkbenchPropertiesResponse()
	{
		RegAll();
	}
}
class RST_WorkbenchListEntityProperties : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchPropertiesRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPropertiesRequest r = RST_WorkbenchPropertiesRequest.Cast(request);
		RST_WorkbenchPropertiesResponse response = new RST_WorkbenchPropertiesResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;

		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi())
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		IEntitySource entity = Find(editor.GetApi(), r.entityId);
		if (!entity)
		{
			response.status = "entity-not-found";
			return response;
		}

		for (int i, count = entity.GetNumVars(); i < count; i++)
		{
			string name = entity.GetVarName(i);
			DataVarType dataType = entity.GetDataVarType(i);
			string typeName = PropertyTypeName(dataType);
			string value;
			if (typeName.IsEmpty() || !ReadPropertyValue(entity, name, dataType, value)) continue;

			value.Replace("|", "/");
			value.Replace(";", "/");
			if (!response.properties.IsEmpty()) response.properties += ";";
			response.properties += string.Format("%1|%2|%3|1", name, typeName, value);
		}

		response.status = "available";
		return response;
	}
}
class RST_WorkbenchSetEntityProperty : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchPropertiesRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPropertiesRequest r = RST_WorkbenchPropertiesRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!Setup(api, response)) return response;

		IEntitySource entity = Find(api, r.entityId);
		if (!entity || r.propertyName.IsEmpty())
		{
			response.status = "property-not-found";
			return response;
		}

		int propertyIndex = entity.GetVarIndex(r.propertyName);
		if (propertyIndex < 0)
		{
			response.status = "property-not-found";
			return response;
		}

		string observedValue;
		if (!ReadPropertyValue(entity, r.propertyName, entity.GetDataVarType(propertyIndex), observedValue)
			|| observedValue != r.expectedValue)
		{
			response.status = "stale-property-observation";
			return response;
		}

		if (api.IsEntityLayerLockedHierarchy(entity.GetSubScene(), entity.GetLayerID())
			|| !api.BeginEntityAction("Reforger Script Tools: set entity property"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		if (!api.SetVariableValue(entity, null, r.propertyName, r.value))
		{
			api.EndEntityAction("Reforger Script Tools: set entity property");
			response.status = "mutation-rejected";
			return response;
		}

		api.EndEntityAction("Reforger Script Tools: set entity property");
		Record(api, response, entity);
		response.status = "property-set";
		return response;
	}
}
#endif
"#;

const BRIDGE_PREFAB_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchPrefabRequest : JsonApiStruct
{
	string entityId;
	string resourceName;
	string memberId;

	void RST_WorkbenchPrefabRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchPrefabResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string status;
	string entity;
	string memberId;
	string resourceName;
	string resourceReferenceKind;
	string contributorAddons;
	string ancestorResources;
	bool ancestorResourcesTruncated;
	bool prefabEditMode;
	string components;
	string componentProperties;
	string children;
	bool childrenTruncated;
	string properties;
	bool propertiesTruncated;
	int childCount;

	void RST_WorkbenchPrefabResponse()
	{
		RegAll();
	}
}
class RST_WorkbenchInspectPrefab : NetApiHandler
{
	IEntitySource Find(WorldEditorAPI api, string id)
	{
		for (int i, count = api.GetEditorEntityCount(); i < count; i++)
		{
			IEntitySource candidate = api.GetEditorEntity(i);
			if (candidate && candidate.GetID().ToString() == id)
				return candidate;
		}
		return null;
	}
	// Keep resource and entity inspection on the same authored-component path.
	int ComponentCount(BaseContainer source)
	{
		IEntitySource entity = IEntitySource.Cast(source);
		int count;
		ref BaseContainerList components;
		if (entity)
		{
			count = entity.GetComponentCount();
			if (count != 0)
				return count;
		}
		components = source.GetObjectArray("components");
		if (components) return components.Count();
		return 0;
	}
	IEntityComponentSource ComponentAt(BaseContainer source, int index)
	{
		IEntitySource entity = IEntitySource.Cast(source);
		ref BaseContainerList components;
		if (entity && entity.GetComponentCount() > 0)
			return entity.GetComponent(index);
		components = source.GetObjectArray("components");
		if (components) return IEntityComponentSource.Cast(components.Get(index));
		return null;
	}
	BaseContainer FindMember(BaseContainer parent, string memberId, string prefix, int depth)
	{
		if (depth >= 16)
			return null;
		int firstChildIndex;
		if (parent.GetNumChildren() > 0 && !parent.GetChild(0) && parent.GetChild(1))
			firstChildIndex = 1;
		for (int index, count = parent.GetNumChildren(); index < count; index++)
		{
			BaseContainer child = parent.GetChild(index + firstChildIndex);
			if (!child)
				continue;
			string childId = string.Format("member:%1", index);
			if (!prefix.IsEmpty())
				childId = string.Format("%1/%2", prefix, childId);
			if (memberId == childId)
				return child;
			BaseContainer result = FindMember(child, memberId, childId, depth + 1);
			if (result)
				return result;
		}
		return null;
	}
	BaseContainer ResolveMember(IEntitySource root, string memberId)
	{
		if (memberId.IsEmpty())
			return root;
		return FindMember(root, memberId, string.Empty, 0);
	}
	bool IsEngineCallback(string name)
	{
		return name.StartsWith("EOn") || name.StartsWith("_WB_") || name == "RplLoad"
			|| name == "RplSave" || name == "Preload" || name == "OnTransformResetImpl"
			|| name == "userScript" || name == "constructor" || name == "destructor";
	}
	string PropertyOrigin(BaseContainer container, string name)
	{
		if (container.IsVariableSetDirectly(name))
			return "direct";
		for (BaseContainer ancestor = container.GetAncestor(); ancestor; ancestor = ancestor.GetAncestor())
		{
			if (ancestor.IsVariableSetDirectly(name))
				return "inherited";
		}
		return "default";
	}
	string PropertyTypeName(DataVarType dataType) { switch (dataType) { case DataVarType.BOOLEAN: return "bool"; case DataVarType.INTEGER: return "integer"; case DataVarType.SCALAR: return "float"; case DataVarType.VECTOR3: return "vector"; case DataVarType.STRING: return "string"; case DataVarType.RESOURCE_NAME: return "resource"; } return string.Empty; }
	string PropertyRecord(int componentIndex, BaseContainer container, string name, string typeName, string value)
	{
		name.Replace("|", "/");
		name.Replace(";", "/");
		value.Replace("|", "/");
		value.Replace(";", "/");
		string origin = PropertyOrigin(container, name);
		if (componentIndex >= 0)
			return string.Format("%1|%2|%3|%4|%5|%6", componentIndex, name, typeName, value, container.IsVariableSetDirectly(name), origin);
		return string.Format("%1|%2|%3|%4|%5", name, typeName, value, container.IsVariableSetDirectly(name), origin);
	}
	bool ReadPropertyRecord(int componentIndex, BaseContainer container, string name, DataVarType dataType, out string record)
	{
		bool boolValue;
		int integerValue;
		float floatValue;
		vector vectorValue;
		string stringValue;
		ResourceName resourceValue;
		string typeName = PropertyTypeName(dataType);
		if (typeName.IsEmpty())
			return false;

		switch (dataType)
		{
			case DataVarType.BOOLEAN:
				if (!container.Get(name, boolValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, boolValue.ToString());
				return true;
			case DataVarType.INTEGER:
				if (!container.Get(name, integerValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, integerValue.ToString());
				return true;
			case DataVarType.SCALAR:
				if (!container.Get(name, floatValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, floatValue.ToString());
				return true;
			case DataVarType.VECTOR3:
				if (!container.Get(name, vectorValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, vectorValue.ToString(false));
				return true;
			case DataVarType.STRING:
				if (!container.Get(name, stringValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, stringValue);
				return true;
			case DataVarType.RESOURCE_NAME:
				if (!container.Get(name, resourceValue))
					return false;
				record = PropertyRecord(componentIndex, container, name, typeName, resourceValue);
				return true;
		}
		return false;
	}
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchPrefabRequest();
	}
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPrefabRequest r = RST_WorkbenchPrefabRequest.Cast(request); RST_WorkbenchPrefabResponse response = new RST_WorkbenchPrefabResponse(); response.bridgeVersion = "1.51.0"; response.protocolVersion = 1;
		WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor || !editor.GetApi()) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi();
		// Keep the resource handle alive while any source/container derived from it is read.
		ref Resource resource;
		IEntitySource source;
		Print(string.Format("RST prefab inspect: begin entity=%1 resource=%2", r.entityId, r.resourceName));
		if (!r.entityId.IsEmpty())
		{
			source = Find(api, r.entityId);
			Print("RST prefab inspect: resolved live entity source");
		}
		else if (!r.resourceName.IsEmpty())
		{
			ResourceName prefabName = r.resourceName;
			Print("RST prefab inspect: loading resource");
			resource = Resource.Load(prefabName);
			Print(string.Format("RST prefab inspect: resource valid=%1", resource && resource.IsValid()));
			if (resource && resource.IsValid())
			{
				source = resource.GetResource().ToEntitySource();
				Print(string.Format("RST prefab inspect: source resolved=%1", source != null));
			}
		}
		if (!source) { response.status = "prefab-not-found"; return response; }
		Print("RST prefab inspect: source available");
		BaseContainer target = ResolveMember(source, r.memberId);
		if (!target) { response.status = "member-not-found"; return response; }
		Print("RST prefab inspect: member resolved");
		response.memberId = r.memberId;
		if (!r.entityId.IsEmpty()) response.entity = string.Format("%1|%2|%3|%4", source.GetID().ToString(), source.GetClassName(), source.GetSubScene(), source.GetLayerID());
		Print("RST prefab inspect: entity metadata complete");
		ResourceName resourceName = source.GetResourceName(); response.resourceName = resourceName; if (resourceName.IsExternal()) response.resourceReferenceKind = "external"; else if (resourceName.IsInternal()) response.resourceReferenceKind = "internal"; else response.resourceReferenceKind = "path";
		Print("RST prefab inspect: resource metadata complete");
		array<string> addons = {}; source.GetSourceAddons(addons); foreach (string addon : addons) { addon.Replace(";", "/"); if (!response.contributorAddons.IsEmpty()) response.contributorAddons += ";"; response.contributorAddons += addon; }
		Print("RST prefab inspect: addon metadata complete");
		BaseContainer ancestor = source.GetAncestor();
		for (int depth = 0; ancestor && depth < 16; depth++)
		{
			ResourceName ancestorName = ancestor.GetResourceName();
			string ancestorResource = string.Format("%1", ancestorName);
			ancestorResource.Replace(";", "/");
			if (!response.ancestorResources.IsEmpty())
				response.ancestorResources += ";";
			response.ancestorResources += ancestorResource;
			ancestor = ancestor.GetAncestor();
		}
		if (ancestor)
			response.ancestorResourcesTruncated = true;
		Print("RST prefab inspect: ancestors complete");
		response.prefabEditMode = api.IsPrefabEditMode(); response.childCount = target.GetNumChildren();
		for (int i, count = ComponentCount(target); i < count && i < 64; i++)
		{
			IEntityComponentSource component = ComponentAt(target, i);
			if (!component) continue;
			if (!response.components.IsEmpty()) response.components += ";";
			response.components += string.Format("%1|%2", i, component.GetClassName());

			for (int propertyIndex, propertyCount = component.GetNumVars(); propertyIndex < propertyCount; propertyIndex++)
			{
				string name = component.GetVarName(propertyIndex);
				DataVarType dataType = component.GetDataVarType(propertyIndex);
				string record;
				if (IsEngineCallback(name))
					continue;
				if (!ReadPropertyRecord(i, component, name, dataType, record))
					continue;
				if (!response.componentProperties.IsEmpty())
					response.componentProperties += ";";
				response.componentProperties += record;
			}
		}
		int firstChildIndex;
		if (target.GetNumChildren() > 0 && !target.GetChild(0) && target.GetChild(1))
			firstChildIndex = 1;
		for (int i, count = target.GetNumChildren(), returned; i < count; i++)
		{
			BaseContainer child = target.GetChild(i + firstChildIndex);
			if (!child)
				continue;
			if (returned >= 64)
			{
				response.childrenTruncated = true;
				break;
			}
			string className = child.GetClassName();
			string name = child.GetName();
			className.Replace("|", "/");
			className.Replace(";", "/");
			name.Replace("|", "/");
			name.Replace(";", "/");
			if (!response.children.IsEmpty())
				response.children += ";";
			response.children += string.Format("%1|%2|%3", i, className, name);
			returned++;
		}
		for (int i, count = target.GetNumVars(), returned; i < count; i++)
		{
			string name = target.GetVarName(i);
			DataVarType dataType;
			string record;
			if (IsEngineCallback(name))
				continue;
			if (!target.IsVariableSetDirectly(name) && name != "coords" && name != "angles" && name != "scale")
				continue;
			if (returned >= 128)
			{
				response.propertiesTruncated = true;
				break;
			}
			dataType = target.GetDataVarType(i);
			name.Replace("|", "/");
			name.Replace(";", "/");
			if (!ReadPropertyRecord(-1, target, name, dataType, record))
				continue;
			if (!response.properties.IsEmpty())
				response.properties += ";";
			response.properties += record;
			returned++;
		}
		response.status = "available";
		return response;
	}
}

class RST_WorkbenchCreatePrefab : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchEntityMutationRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request); RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor || !editor.GetApi()) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi(); IEntitySource entity = Find(api, r.entityId);
		if (!entity || r.name.IsEmpty() || !r.name.EndsWith(".et") || r.name.Contains("..") || r.name.Contains(":") || r.name.StartsWith("/") || r.name.StartsWith("\\")) { response.status = "invalid-prefab-destination"; return response; }
		string absolutePath; if (!Workbench.GetAbsolutePath(r.name, absolutePath, false)) { response.status = "invalid-prefab-destination"; return response; }
		response.destination = r.name; response.destinationExists = FileIO.FileExists(absolutePath);
		if (response.destinationExists) { response.status = "prefab-destination-exists"; return response; }
		if (!r.confirm) { Record(api, response, entity); response.status = "confirmation-required"; return response; }
		if (!api.CreateEntityTemplate(entity, absolutePath)) { response.status = "prefab-create-rejected"; return response; }
		Record(api, response, entity); response.status = "prefab-created"; return response;
	}
}

class RST_WorkbenchCreateGenericPrefab : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchEntityMutationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi())
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		int subScene = api.GetCurrentSubScene();
		int layerId = api.GetCurrentEntityLayerId();
		response.activeLayerId = layerId;
		if (r.name.IsEmpty() || !r.name.EndsWith(".et") || r.name.Contains("..")
			|| r.name.Contains(":") || r.name.StartsWith("/") || r.name.StartsWith("\\"))
		{
			response.status = "invalid-prefab-destination";
			return response;
		}

		string absolutePath;
		if (!Workbench.GetAbsolutePath(r.name, absolutePath, false))
		{
			response.status = "invalid-prefab-destination";
			return response;
		}

		response.destination = r.name;
		response.destinationExists = FileIO.FileExists(absolutePath);
		if (response.destinationExists)
		{
			response.status = "prefab-destination-exists";
			return response;
		}

		if (subScene < 0 || layerId < 0 || api.IsEntityLayerLockedHierarchy(subScene, layerId))
		{
			response.status = "invalid-create-target";
			return response;
		}

		if (!r.confirm)
		{
			response.status = "confirmation-required";
			return response;
		}

		if (!api.BeginEntityAction("Reforger Script Tools: create GenericEntity prefab"))
		{
			response.status = "mutation-rejected";
			return response;
		}

		IEntitySource temporary = api.CreateEntity(
			"GenericEntity",
			"RST_PrefabTemplateSource",
			layerId,
			null,
			Vector(0, 0, 0),
			Vector(0, 0, 0));
		if (!temporary)
		{
			api.EndEntityAction("Reforger Script Tools: create GenericEntity prefab");
			response.status = "generic-entity-create-rejected";
			return response;
		}

		bool templateCreated = api.CreateEntityTemplate(temporary, absolutePath);
		bool temporaryDeleted = api.DeleteEntity(temporary);
		api.EndEntityAction("Reforger Script Tools: create GenericEntity prefab");
		if (!templateCreated)
		{
			response.status = "prefab-create-rejected";
			return response;
		}
		if (!temporaryDeleted)
		{
			response.status = "temporary-entity-cleanup-failed";
			return response;
		}

		response.status = "generic-prefab-created";
		return response;
	}
}

class RST_WorkbenchSavePrefab : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchEntityMutationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchEntityMutationRequest r = RST_WorkbenchEntityMutationRequest.Cast(request);
		RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi())
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		IEntitySource entity = Find(api, r.entityId);
		ref Resource resource;
		if (r.entityId.IsEmpty())
		{
			if (r.resourceName.IsEmpty())
			{
				response.status = "invalid-prefab-target";
				return response;
			}

			ResourceName resourceName = r.resourceName;
			resource = Resource.Load(resourceName);
			if (!resource || !resource.IsValid() || !resource.GetResource())
			{
				response.status = "prefab-not-found";
				return response;
			}

			entity = IEntitySource.Cast(resource.GetResource().ToBaseContainer());
			if (!entity)
			{
				response.status = "prefab-source-unavailable";
				return response;
			}
		}
		else if (!entity || !api.IsPrefabEditMode())
		{
			response.status = "prefab-edit-unavailable";
			return response;
		}

		if (!r.confirm)
		{
			if (!r.entityId.IsEmpty())
				Record(api, response, entity);
			response.status = "confirmation-required";
			return response;
		}

		if (!api.SaveEntityTemplate(entity))
		{
			response.status = "prefab-save-rejected";
			return response;
		}

		if (!r.entityId.IsEmpty())
			Record(api, response, entity);
		response.status = "prefab-saved";
		return response;
	}
}

// This is a bounded acceptance gate, not a general-purpose mutation endpoint.
// It proves that Workbench can mutate and save a resource-loaded IEntitySource
// without using a scene instance or a resource-file writer. The component is
// created and removed in the same native editor action, so a successful save
// leaves the resource's effective component count unchanged.
class RST_WorkbenchPrefabResourceProofRequest : JsonApiStruct
{
	string resourceName;
	bool confirm;

	void RST_WorkbenchPrefabResourceProofRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchPrefabResourceProofResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string status;
	string resourceName;
	bool resourceLoaded;
	bool sourceResolved;
	bool actionBegun;
	bool componentCreated;
	bool componentDeleted;
	bool templateSaved;
	int componentCountBefore;
	int componentCountAfterReload;

	void RST_WorkbenchPrefabResourceProofResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchPrefabResourceProof : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchPrefabResourceProofRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPrefabResourceProofRequest r = RST_WorkbenchPrefabResourceProofRequest.Cast(request);
		RST_WorkbenchPrefabResourceProofResponse response = new RST_WorkbenchPrefabResourceProofResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		response.resourceName = r.resourceName;

		if (r.resourceName.IsEmpty())
		{
			response.status = "invalid-prefab-target";
			return response;
		}

		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi())
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		ResourceName prefabName = r.resourceName;
		ref Resource resource = Resource.Load(prefabName);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			response.status = "resource-load-failed";
			return response;
		}
		response.resourceLoaded = true;

		IEntitySource source = IEntitySource.Cast(resource.GetResource().ToBaseContainer());
		if (!source)
		{
			response.status = "prefab-source-unavailable";
			return response;
		}
		response.sourceResolved = true;
		response.componentCountBefore = source.GetComponentCount();

		if (!r.confirm)
		{
			response.status = "confirmation-required";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!api.BeginEntityAction("Reforger Script Tools: resource prefab proof"))
		{
			response.status = "action-rejected";
			return response;
		}
		response.actionBegun = true;

		IEntityComponentSource component = api.CreateComponent(source, "ScriptComponent");
		if (!component)
		{
			api.EndEntityAction("Reforger Script Tools: resource prefab proof");
			response.status = "component-create-rejected";
			return response;
		}
		response.componentCreated = true;

		if (!api.DeleteComponent(source, component))
		{
			api.EndEntityAction("Reforger Script Tools: resource prefab proof");
			response.status = "component-delete-rejected";
			return response;
		}
		response.componentDeleted = true;
		api.EndEntityAction("Reforger Script Tools: resource prefab proof");

		if (!api.SaveEntityTemplate(source))
		{
			response.status = "prefab-save-rejected";
			return response;
		}
		response.templateSaved = true;

		resource = null;
		resource = Resource.Load(prefabName);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			response.status = "reload-failed";
			return response;
		}

		IEntitySource reloadedSource = IEntitySource.Cast(resource.GetResource().ToBaseContainer());
		if (!reloadedSource)
		{
			response.status = "reload-source-unavailable";
			return response;
		}
		response.componentCountAfterReload = reloadedSource.GetComponentCount();
		if (response.componentCountAfterReload != response.componentCountBefore)
		{
			response.status = "reload-mismatch";
			return response;
		}

		response.status = "proof-succeeded";
		return response;
	}
}
class RST_WorkbenchPrefabResourceComponentRequest : JsonApiStruct
{
	string resourceName;
	string className;
	bool confirm;

	void RST_WorkbenchPrefabResourceComponentRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchPrefabResourceComponentResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string status;
	string resourceName;
	int componentIndex;
	string componentClass;
	bool templateSaved;

	void RST_WorkbenchPrefabResourceComponentResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchAddPrefabResourceComponent : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchPrefabResourceComponentRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPrefabResourceComponentRequest r = RST_WorkbenchPrefabResourceComponentRequest.Cast(request);
		RST_WorkbenchPrefabResourceComponentResponse response = new RST_WorkbenchPrefabResourceComponentResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		response.resourceName = r.resourceName;
		response.componentIndex = -1;

		if (r.resourceName.IsEmpty() || r.className.IsEmpty())
		{
			response.status = "invalid-component-request";
			return response;
		}

		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi())
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		ResourceName prefabName = r.resourceName;
		ref Resource resource = Resource.Load(prefabName);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			response.status = "resource-load-failed";
			return response;
		}

		IEntitySource source = IEntitySource.Cast(resource.GetResource().ToBaseContainer());
		if (!source)
		{
			response.status = "prefab-source-unavailable";
			return response;
		}

		if (!r.confirm)
		{
			response.status = "confirmation-required";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!api.BeginEntityAction("Reforger Script Tools: add prefab resource component"))
		{
			response.status = "action-rejected";
			return response;
		}

		IEntityComponentSource component = api.CreateComponent(source, r.className);
		if (!component)
		{
			api.EndEntityAction("Reforger Script Tools: add prefab resource component");
			response.status = "component-create-rejected";
			return response;
		}

		response.componentClass = component.GetClassName();
		for (int index, count = source.GetComponentCount(); index < count; index++)
		{
			if (source.GetComponent(index) == component)
			{
				response.componentIndex = index;
				break;
			}
		}
		api.EndEntityAction("Reforger Script Tools: add prefab resource component");

		if (!api.SaveEntityTemplate(source))
		{
			response.status = "prefab-save-rejected";
			return response;
		}

		response.templateSaved = true;
		response.status = "prefab-component-added";
		return response;
	}
}

class RST_WorkbenchPrefabResourceComponentRemovalRequest : JsonApiStruct
{
	string resourceName;
	string className;
	int componentIndex;
	bool confirm;

	void RST_WorkbenchPrefabResourceComponentRemovalRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchRemovePrefabResourceComponent : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchPrefabResourceComponentRemovalRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPrefabResourceComponentRemovalRequest r = RST_WorkbenchPrefabResourceComponentRemovalRequest.Cast(request);
		RST_WorkbenchPrefabResourceComponentResponse response = new RST_WorkbenchPrefabResourceComponentResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		response.resourceName = r.resourceName;
		response.componentIndex = r.componentIndex;
		response.componentClass = r.className;

		if (r.resourceName.IsEmpty() || r.className.IsEmpty() || r.componentIndex < 0)
		{
			response.status = "invalid-component-request";
			return response;
		}

		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi())
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		ResourceName prefabName = r.resourceName;
		ref Resource resource = Resource.Load(prefabName);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			response.status = "resource-load-failed";
			return response;
		}

		IEntitySource source = IEntitySource.Cast(resource.GetResource().ToBaseContainer());
		if (!source)
		{
			response.status = "prefab-source-unavailable";
			return response;
		}

		if (r.componentIndex >= source.GetComponentCount())
		{
			response.status = "stale-component-observation";
			return response;
		}

		IEntityComponentSource component = source.GetComponent(r.componentIndex);
		if (!component || component.GetClassName() != r.className)
		{
			response.status = "stale-component-observation";
			return response;
		}

		if (!r.confirm)
		{
			response.status = "confirmation-required";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!api.BeginEntityAction("Reforger Script Tools: remove prefab resource component"))
		{
			response.status = "action-rejected";
			return response;
		}

		if (!api.DeleteComponent(source, component))
		{
			api.EndEntityAction("Reforger Script Tools: remove prefab resource component");
			response.status = "component-delete-rejected";
			return response;
		}
		api.EndEntityAction("Reforger Script Tools: remove prefab resource component");

		if (!api.SaveEntityTemplate(source))
		{
			response.status = "prefab-save-rejected";
			return response;
		}

		response.templateSaved = true;
		response.status = "prefab-component-removed";
		return response;
	}
}

class RST_WorkbenchPrefabResourcePropertyRequest : JsonApiStruct
{
	string resourceName;
	int componentIndex;
	string componentClass;
	string propertyName;
	string expectedValue;
	string value;
	bool confirm;

	void RST_WorkbenchPrefabResourcePropertyRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchPrefabResourcePropertyResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string status;
	string resourceName;
	bool templateSaved;

	void RST_WorkbenchPrefabResourcePropertyResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchSetPrefabResourceProperty : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchPrefabResourcePropertyRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPrefabResourcePropertyRequest r = RST_WorkbenchPrefabResourcePropertyRequest.Cast(request);
		RST_WorkbenchPrefabResourcePropertyResponse response = new RST_WorkbenchPrefabResourcePropertyResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		response.resourceName = r.resourceName;

		if (r.resourceName.IsEmpty() || r.propertyName.IsEmpty())
		{
			response.status = "invalid-property-request";
			return response;
		}

		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi())
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		ResourceName prefabName = r.resourceName;
		ref Resource resource = Resource.Load(prefabName);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			response.status = "resource-load-failed";
			return response;
		}

		IEntitySource source = IEntitySource.Cast(resource.GetResource().ToBaseContainer());
		if (!source)
		{
			response.status = "prefab-source-unavailable";
			return response;
		}

		BaseContainer target = source;
		if (r.componentIndex >= 0)
		{
			if (r.componentIndex >= source.GetComponentCount())
			{
				response.status = "stale-component-observation";
				return response;
			}

			IEntityComponentSource component = source.GetComponent(r.componentIndex);
			if (!component || component.GetClassName() != r.componentClass)
			{
				response.status = "stale-component-observation";
				return response;
			}
			target = component;
		}

		int propertyIndex = target.GetVarIndex(r.propertyName);
		string observedValue;
		if (propertyIndex < 0
			|| !ReadPropertyValue(target, r.propertyName, target.GetDataVarType(propertyIndex), observedValue)
			|| observedValue != r.expectedValue)
		{
			response.status = "stale-property-observation";
			return response;
		}

		if (!r.confirm)
		{
			response.status = "confirmation-required";
			return response;
		}

		WorldEditorAPI api = editor.GetApi();
		if (!api.BeginEntityAction("Reforger Script Tools: set prefab resource property"))
		{
			response.status = "action-rejected";
			return response;
		}

		array<ref ContainerIdPathEntry> path;
		if (r.componentIndex >= 0)
			path = {new ContainerIdPathEntry("components", r.componentIndex)};

		if (!api.SetVariableValue(source, path, r.propertyName, r.value))
		{
			api.EndEntityAction("Reforger Script Tools: set prefab resource property");
			response.status = "mutation-rejected";
			return response;
		}
		api.EndEntityAction("Reforger Script Tools: set prefab resource property");

		if (!api.SaveEntityTemplate(source))
		{
			response.status = "prefab-save-rejected";
			return response;
		}

		response.templateSaved = true;
		response.status = "prefab-property-set";
		return response;
	}
}

class RST_WorkbenchPrefabPropertyRequest : JsonApiStruct
{
	string entityId;
	string propertyName;
	string expectedValue;
	string value;

	void RST_WorkbenchPrefabPropertyRequest()
	{
		RegAll();
	}
}
class RST_WorkbenchSetPrefabProperty : RST_WorkbenchEntityMutationBase
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchPrefabPropertyRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPrefabPropertyRequest r = RST_WorkbenchPrefabPropertyRequest.Cast(request); RST_WorkbenchEntityMutationResponse response = Response();
		WorldEditor editor = Workbench.GetModule(WorldEditor); if (!editor || !editor.GetApi()) { response.status = "world-editor-unavailable"; return response; }
		WorldEditorAPI api = editor.GetApi(); IEntitySource entity = Find(api, r.entityId);
		if (!entity || !api.IsPrefabEditMode() || r.propertyName.IsEmpty()) { response.status = "prefab-edit-unavailable"; return response; }
		int index = entity.GetVarIndex(r.propertyName); string observed; if (index < 0 || !ReadPropertyValue(entity, r.propertyName, entity.GetDataVarType(index), observed) || observed != r.expectedValue) { response.status = "stale-property-observation"; return response; }
		if (!api.BeginEntityAction("Reforger Script Tools: set prefab property") || !api.SetVariableValue(entity, null, r.propertyName, r.value)) { api.EndEntityAction("Reforger Script Tools: set prefab property"); response.status = "mutation-rejected"; return response; }
		api.EndEntityAction("Reforger Script Tools: set prefab property"); Record(api, response, entity); response.status = "prefab-property-set"; return response;
	}
}
#endif
"#;

const BRIDGE_LIST_RESOURCES_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchListResourcesRequest : JsonApiStruct
{
	string extensions;
	string query;
	string rootPath;
	string addonGuid;
	int offset;
	int limit;
	void RST_WorkbenchListResourcesRequest() { RegAll(); }
}
class RST_WorkbenchListResourcesResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string loadedAddons;
	string resources;
	string resourceDetails;
	bool hasMore;
	void RST_WorkbenchListResourcesResponse() { RegAll(); }
}
class RST_WorkbenchListResources : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new RST_WorkbenchListResourcesRequest(); }
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchListResourcesRequest typedRequest = RST_WorkbenchListResourcesRequest.Cast(request);
		RST_WorkbenchListResourcesResponse response = new RST_WorkbenchListResourcesResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		array<string> addonGuids = new array<string>();
		GameProject.GetLoadedAddons(addonGuids);
		foreach (string addonGuid : addonGuids)
		{
			string addonId = GameProject.GetAddonID(addonGuid);
			if (addonId == string.Empty)
				continue;
			if (response.loadedAddons != string.Empty)
				response.loadedAddons += ";";
			response.loadedAddons += addonId;
		}
		array<string> extensions = new array<string>();
		if (typedRequest.extensions != string.Empty)
			typedRequest.extensions.Split(";", extensions, true);
		array<string> searchStrings = new array<string>();
		if (typedRequest.query != string.Empty)
			searchStrings.Insert(typedRequest.query);
		SearchResourcesFilter filter = new SearchResourcesFilter();
		filter.fileExtensions = extensions;
		filter.searchStr = searchStrings;
		filter.rootPath = typedRequest.rootPath;
		array<ResourceName> allResources = new array<ResourceName>();
		ResourceDatabase.SearchResources(filter, allResources.Insert);
		if (!typedRequest.addonGuid.IsEmpty())
		{
			array<ResourceName> matchingResources = new array<ResourceName>();
			foreach (ResourceName resource : allResources)
			{
				string resourceName = string.Format("%1", resource);
				if (resourceName.Contains("{" + typedRequest.addonGuid + "}"))
					matchingResources.Insert(resource);
			}
			allResources = matchingResources;
		}
		allResources.Sort();
		int start = typedRequest.offset;
		if (start < 0)
			start = 0;
		int limit = typedRequest.limit;
		if (limit < 1)
			limit = 1;
		if (limit > 200)
			limit = 200;
		int returnedCount = 0;
		for (int index = start; index < allResources.Count() && returnedCount < limit; index++)
		{
			ResourceName resource = allResources[index];
			string resourceName = string.Format("%1", resource);
			resourceName.Replace(";", "/");
			int addonGuidEnd = resourceName.IndexOf("}");
			if (!resourceName.StartsWith("{") || addonGuidEnd <= 1)
				continue;
			string addonGuid = resourceName.Substring(1, addonGuidEnd - 1);
			string addonId = GameProject.GetAddonID(addonGuid);
			if (addonId.IsEmpty())
				addonId = GameProject.GetAddonID(resourceName.Substring(0, addonGuidEnd + 1));
			string logicalPath = resource.GetPath();
			string extension;
			FilePath.StripExtension(logicalPath, extension);
			if (logicalPath.IsEmpty() || extension.IsEmpty())
				continue;
			if (!response.resources.IsEmpty())
			{
				response.resources += ";";
				response.resourceDetails += ";";
			}
			response.resources += resourceName;
			response.resourceDetails += resourceName + "|" + addonGuid + "|" + addonId + "|" + logicalPath + "|" + extension;
			returnedCount++;
		}
		response.hasMore = start + returnedCount < allResources.Count();
		return response;
	}
}
#endif
"#;

#[cfg(test)]
mod tests {
    use super::{
        WorkbenchDiagnosticLocation, WorkbenchDiagnosticSeverity, WorkbenchFailureCode,
        WorkbenchGateway, WorkbenchGatewayOptions, WorkbenchStatus,
    };
    use serde_json::{json, Value};
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn workbench_bool_accepts_json_and_enfusion_representations() {
        assert!(super::workbench_bool(&json!(true)));
        assert!(super::workbench_bool(&json!(1)));
        assert!(!super::workbench_bool(&json!(false)));
        assert!(!super::workbench_bool(&json!(0)));
        assert!(!super::workbench_bool(&Value::Null));
    }

    #[test]
    fn world_selection_records_are_bounded_and_require_stable_identity_fields() {
        let records = super::parse_world_selection_records(
            "0x0000000000000001|TestEntity|0|12|12.5|3|-4.25|PrefabName|Authored name|Main scene|Gameplay;0x0000000000000002|LightEntity|2|4",
        )
        .unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].entity_id, "0x0000000000000001");
        assert_eq!(records[0].class_name, "TestEntity");
        assert_eq!(records[0].sub_scene, 0);
        assert_eq!(records[0].layer_id, 12);
        assert_eq!(
            records[0].position,
            Some(super::WorkbenchEntityPosition {
                x: 12.5,
                y: 3.0,
                z: -4.25,
            })
        );
        assert_eq!(records[0].resource_name.as_deref(), Some("PrefabName"));
        assert_eq!(records[0].name.as_deref(), Some("Authored name"));
        assert_eq!(records[0].sub_scene_name.as_deref(), Some("Main scene"));
        assert_eq!(records[0].layer_name.as_deref(), Some("Gameplay"));
        assert!(records[1].position.is_none());
        assert!(super::parse_world_selection_records("missing-fields").is_err());
        assert!(super::parse_world_selection_records("id|class|nope|0").is_err());
        assert!(super::parse_world_selection_records("id|class|0|0|NaN|0|0").is_err());
    }

    #[test]
    fn selected_entity_hierarchy_records_require_one_stable_target() {
        let entity =
            super::parse_optional_world_selection_record("0x0000000000000001|TestEntity|0|12")
                .unwrap()
                .expect("entity");
        assert_eq!(entity.entity_id, "0x0000000000000001");
        assert!(super::parse_optional_world_selection_record("")
            .unwrap()
            .is_none());
        assert!(super::parse_optional_world_selection_record(
            "0x0000000000000001|One|0|1;0x0000000000000002|Two|0|2"
        )
        .is_err());
    }

    #[test]
    fn versioned_bridge_handlers_report_the_manifest_version() {
        let expected = format!(
            "response.bridgeVersion = \"{}\"",
            super::WORKBENCH_BRIDGE_VERSION
        );
        for (name, source) in super::bridge_payload() {
            if source.contains("bridgeVersion") {
                assert!(
                    source.contains(&expected),
                    "{name} must report the manifest version"
                );
            }
        }
    }

    #[test]
    fn workbench_log_markers_classify_only_observed_reload_milestones() {
        let lines = vec![
            "SCRIPT: Reloading game scripts".to_string(),
            "SCRIPT: Script validation".to_string(),
            "SCRIPT: Compiling GameLib scripts".to_string(),
            "SCRIPT: Compiling Game scripts".to_string(),
            "SCRIPT: Module: WorkbenchGame; loaded".to_string(),
        ];
        let markers = super::workbench_log_markers("workbench", &lines);
        assert_eq!(
            markers
                .iter()
                .map(|marker| marker.kind.as_str())
                .collect::<Vec<_>>(),
            vec![
                "reload-started",
                "script-validation",
                "gamelib-compilation",
                "game-compilation",
                "workbench-game-module-loaded",
            ]
        );
        assert!(super::workbench_log_markers("integration", &lines).is_empty());
    }

    #[test]
    fn reload_action_dispatch_acknowledgement_does_not_override_log_verification() {
        let path = super::reload_action_path(Ok(super::RawBridgeReloadAction {
            _bridge_version: "1.51.0".to_string(),
            protocol_version: 1,
            _accepted: json!(false),
            action_path: "Plugins/Settings/Reload WB Scripts".to_string(),
        }))
        .unwrap();

        assert_eq!(path, "Plugins/Settings/Reload WB Scripts");
    }

    #[test]
    fn reload_action_timeout_defers_to_log_verification() {
        let path =
            super::reload_action_path(Err(super::failure(super::WorkbenchFailureCode::Timeout)))
                .unwrap();

        assert_eq!(path, "Plugins/Settings/Reload WB Scripts");
    }

    #[test]
    fn world_selection_summary_returns_bounded_stable_editor_facts() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request, json!({"APIFunc": "RST_WorkbenchWorldSelection"}));
            json!({
                "bridgeVersion": "1.5.0",
                "protocolVersion": 1,
                "editorAvailable": true,
                "status": "available",
                "selectedCount": 2,
                "selectedEntities": "0x0000000000000001|TestEntity|0|12;0x0000000000000002|LightEntity|2|4",
                "selectedEntitiesTruncated": false
            })
        });
        let root = test_root("world-selection-summary");
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let result = controller.world_selection_summary().unwrap();

        assert!(result.editor_available);
        assert_eq!(result.selected_count, 2);
        assert_eq!(result.selected_entities.len(), 2);
        assert!(!result.selected_entities_truncated);
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_listing_binds_opaque_cursors_to_the_same_filter() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["APIFunc"], "RST_WorkbenchListResources");
            assert_eq!(request["extensions"], "ent");
            assert_eq!(request["query"], "test");
            assert_eq!(request["rootPath"], "");
            assert_eq!(request["addonGuid"], "");
            assert_eq!(request["offset"], 0);
            assert_eq!(request["limit"], 2);
            json!({
                "bridgeVersion": "1.6.0",
                "protocolVersion": 1,
                "loadedAddons": "ArmaReforger;TestBullshit",
                "resources": "{DD49A6CE18710A05}worlds/test/empty_test.ent",
                "hasMore": true
            })
        });
        let root = test_root("resource-listing");
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let page = controller
            .list_resources(&["ent"], Some("test"), None, None, None, 2)
            .unwrap();

        assert_eq!(page.resources.len(), 1);
        assert_eq!(page.limit, 2);
        assert!(page.truncated);
        assert!(page.next_cursor.is_some());
        assert!(super::parse_resource_list_cursor(page.next_cursor.as_deref().unwrap()).is_some());
        assert_ne!(page.project_revision, "ArmaReforger;TestBullshit");
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_search_projects_only_canonical_resource_facts() {
        let hit = super::parse_resource_search_hit(
            "{00B6CAF6E4A5BAB4}Prefabs/Props/Test.et|00B6CAF6E4A5BAB4|TestBullshit|Prefabs/Props/Test.et|et",
        )
        .unwrap();
        assert_eq!(hit.addon_guid, "00B6CAF6E4A5BAB4");
        assert_eq!(hit.addon_id.as_deref(), Some("TestBullshit"));
        assert_eq!(hit.logical_path, "Prefabs/Props/Test.et");
        assert_eq!(hit.name, "Test");
        assert_eq!(hit.extension, "et");
        assert!(super::parse_resource_search_hit("C:/absolute/Test.et").is_err());
        assert!(super::parse_resource_search_hit("{GUID}../Test.et|GUID||../Test.et|et").is_err());
        assert!(super::valid_resource_root_path(
            "$TestBullshit:Prefabs/Props"
        ));
        assert!(super::valid_resource_root_path("$TestBullshit:"));
        assert!(!super::valid_resource_root_path("C:/absolute"));
        assert!(!super::valid_resource_root_path("$TestBullshit:../Prefabs"));
    }

    #[test]
    fn resource_search_binds_root_and_projects_bridge_facts() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["APIFunc"], "RST_WorkbenchListResources");
            assert_eq!(request["extensions"], "et");
            assert_eq!(request["query"], "test");
            assert_eq!(request["rootPath"], "$TestBullshit:Prefabs");
            assert_eq!(request["addonGuid"], "00B6CAF6E4A5BAB4");
            assert_eq!(request["offset"], 0);
            assert_eq!(request["limit"], 3);
            json!({
                "bridgeVersion": "1.51.0",
                "protocolVersion": 1,
                "loadedAddons": "ArmaReforger;TestBullshit",
                "resources": "{00B6CAF6E4A5BAB4}Prefabs/Props/Test.et",
                "resourceDetails": "{00B6CAF6E4A5BAB4}Prefabs/Props/Test.et|00B6CAF6E4A5BAB4|TestBullshit|Prefabs/Props/Test.et|et",
                "hasMore": false
            })
        });
        let root = test_root("resource-search");
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let page = controller
            .search_resources(
                &["et"],
                Some("test"),
                Some("$TestBullshit:Prefabs"),
                Some("00B6CAF6E4A5BAB4"),
                None,
                3,
            )
            .unwrap();

        assert_eq!(page.results.len(), 1);
        assert_eq!(page.results[0].name, "Test");
        assert_eq!(page.results[0].addon_id.as_deref(), Some("TestBullshit"));
        assert!(!page.truncated);
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn entity_listing_accepts_workbenchs_numeric_has_more_flag() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["APIFunc"], "RST_WorkbenchListEntities");
            assert_eq!(request["limit"], 30);
            assert_eq!(request["subScene"], 2);
            assert_eq!(request["layerId"], 7);
            json!({
                "bridgeVersion": "1.51.0",
                "protocolVersion": 1,
                "worldPath": "$TestBullshit:worlds/test/arland_test.ent",
                "entities": "0x0000000000000001 {}|GenericWorldEntity|0|0",
                "hasMore": 1
            })
        });
        let root = test_root("entity-listing-numeric-has-more");
        fs::create_dir_all(&root).unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let page = controller
            .list_entities(None, None, Some(2), Some(7), None, 30)
            .unwrap();

        assert_eq!(page.entities.len(), 1);
        assert_eq!(page.entities[0].entity_id, "0x0000000000000001 {}");
        assert!(page.truncated);
        assert!(page.next_cursor.is_some());
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn entity_listing_rejects_a_cursor_reused_with_a_different_layer_filter() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["APIFunc"], "RST_WorkbenchListEntities");
            json!({
                "bridgeVersion": "1.51.0",
                "protocolVersion": 1,
                "worldPath": "$Test:worlds/test.ent",
                "entities": "0x0000000000000001 {}|TestEntity|2|7",
                "hasMore": true
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });

        let first = controller
            .list_entities(None, None, Some(2), Some(7), None, 1)
            .unwrap();
        let failure = controller
            .list_entities(
                None,
                None,
                Some(2),
                Some(8),
                first.next_cursor.as_deref(),
                1,
            )
            .unwrap_err();

        assert_eq!(failure.code, super::WorkbenchFailureCode::Protocol);
        peer.join().unwrap();
    }

    #[test]
    fn entity_listing_sends_the_handler_sentinel_for_omitted_layer_filters() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["subScene"], -1);
            assert_eq!(request["layerId"], -1);
            json!({
                "bridgeVersion": "1.51.0",
                "protocolVersion": 1,
                "worldPath": "$Test:worlds/test.ent",
                "entities": "",
                "hasMore": false
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });

        let page = controller
            .list_entities(None, None, None, None, None, 1)
            .unwrap();

        assert!(page.entities.is_empty());
        peer.join().unwrap();
    }

    #[test]
    fn entity_search_returns_component_and_match_facts_from_the_handler() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["APIFunc"], "RST_WorkbenchSearchEntities");
            assert_eq!(request["componentClasses"], "SCR_TriggerEntity");
            assert_eq!(request["resourceQuery"], "Checkpoints");
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"available","worldPath":"$Test:worlds/test.ent","results":"0x01|GenericEntity|0|7|{GUID}Prefabs/Checkpoints/West.et|West checkpoint|SCR_TriggerEntity,SCR_BaseGameModeComponent|name,resource,components|SCR_TriggerEntity|GenericEntity|3|||||||","totalMatches":2,"namedMatches":1,"hasMore":false})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });
        let page = controller
            .search_entities(
                Some("checkpoint"),
                None,
                Some("Checkpoints"),
                &["SCR_TriggerEntity"],
                None,
                None,
                None,
                None,
                20,
            )
            .unwrap();
        assert_eq!(page.results.len(), 1);
        assert_eq!(
            page.results[0].entity.name.as_deref(),
            Some("West checkpoint")
        );
        assert_eq!(
            page.results[0].component_classes,
            ["SCR_TriggerEntity", "SCR_BaseGameModeComponent"]
        );
        assert_eq!(
            page.results[0].matched_component_classes,
            ["SCR_TriggerEntity"]
        );
        assert_eq!(
            page.results[0].matched_fields,
            ["name", "resource", "components"]
        );
        assert_eq!(page.summary.total_matches, 2);
        assert_eq!(page.summary.named_matches, 1);
        assert_eq!(page.summary.anonymous_matches, 1);
        peer.join().unwrap();
    }

    #[test]
    fn entity_search_returns_bounded_descendant_relation_evidence() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["relationDirection"], "descendant");
            assert_eq!(request["relationClassName"], "SCR_TriggerEntity");
            assert_eq!(
                request["relationComponentClasses"],
                "SCR_BaseGameModeComponent"
            );
            assert_eq!(request["relationMaxDepth"], 3);
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"available","worldPath":"$Test:worlds/test.ent","results":"0x01|GenericEntity|0|7|{GUID}Prefabs/Checkpoints/West.et|West checkpoint|SCR_TriggerEntity|relation||GenericEntity|3|descendant|2|0x02|SCR_TriggerEntity|0|7|SCR_BaseGameModeComponent","totalMatches":1,"namedMatches":1,"hasMore":false,"relationTraversalTruncated":true})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });
        let relation = super::WorkbenchEntityRelationFilter {
            direction: super::WorkbenchEntityRelationDirection::Descendant,
            class_name: Some("SCR_TriggerEntity".to_string()),
            component_classes: vec!["SCR_BaseGameModeComponent".to_string()],
            max_depth: 3,
        };
        let page = controller
            .search_entities(None, None, None, &[], Some(&relation), None, None, None, 20)
            .unwrap();
        let matched = page.results[0].relation_match.as_ref().unwrap();
        assert_eq!(
            matched.direction,
            super::WorkbenchEntityRelationDirection::Descendant
        );
        assert_eq!(matched.depth, 2);
        assert_eq!(matched.entity_id, "0x02");
        assert_eq!(matched.class_name, "SCR_TriggerEntity");
        assert!(page.relation_traversal_truncated);
        assert_eq!(
            matched.matched_component_classes,
            ["SCR_BaseGameModeComponent"]
        );
        peer.join().unwrap();
    }

    #[test]
    fn entity_search_uses_the_authored_component_container_fallback() {
        assert!(
            super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("entity.GetObjectArray(\"components\")")
        );
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("entity.GetNumChildren()"));
        assert!(
            !super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("else count = entity.GetNumChildren()")
        );
        assert!(!super::BRIDGE_ENTITY_SEARCH_SOURCE
            .contains("IEntityComponentSource.Cast(entity.GetChild(index))"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("visited > 1024"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("MAX_RELATION_CANDIDATES = 4096"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("MAX_RESULT_CHARACTERS = 262144"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("bool relationTruncated = false"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("relationTraversalTruncated"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("parent.GetChild(index)"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("ComponentAt(entity, componentIndex)"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE
            .contains("int componentCount = ComponentCount(entity);"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains(
            "for (int componentIndex; componentIndex < componentCount; componentIndex++)"
        ));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE
            .contains("components += string.Format(\"%1\", component.GetClassName())"));
    }

    #[test]
    fn entity_search_rejects_component_class_transport_delimiters() {
        let controller = super::WorkbenchController::new(Default::default());

        let failure = controller
            .search_entities(
                None,
                None,
                None,
                &["SCR_A;SCR_B"],
                None,
                None,
                None,
                None,
                1,
            )
            .unwrap_err();

        assert_eq!(failure.code, super::WorkbenchFailureCode::Protocol);
        assert!(super::valid_component_class_name(
            "SCR_MapDescriptorComponent"
        ));
        assert!(!super::valid_component_class_name("SCR_A;SCR_B"));
    }

    #[test]
    fn entity_search_rejects_unbounded_or_empty_relation_filters() {
        let controller = super::WorkbenchController::new(Default::default());
        let invalid_parent_depth = super::WorkbenchEntityRelationFilter {
            direction: super::WorkbenchEntityRelationDirection::Parent,
            class_name: Some("GenericEntity".to_string()),
            component_classes: Vec::new(),
            max_depth: 2,
        };
        let empty_descendant = super::WorkbenchEntityRelationFilter {
            direction: super::WorkbenchEntityRelationDirection::Descendant,
            class_name: None,
            component_classes: Vec::new(),
            max_depth: 3,
        };

        for relation in [&invalid_parent_depth, &empty_descendant] {
            let failure = controller
                .search_entities(None, None, None, &[], Some(relation), None, None, None, 1)
                .unwrap_err();
            assert_eq!(failure.code, super::WorkbenchFailureCode::Protocol);
        }
    }

    #[test]
    fn entity_search_cursor_is_bound_to_the_relation_filter() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["relationDirection"], "descendant");
            assert_eq!(request["relationClassName"], "SCR_TriggerEntity");
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"available","worldPath":"$Test:worlds/test.ent","results":"0x01|GenericEntity|0|7||||||GenericEntity|1|descendant|1|0x02|SCR_TriggerEntity|0|7|","totalMatches":2,"namedMatches":0,"hasMore":true})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });
        let trigger_relation = super::WorkbenchEntityRelationFilter {
            direction: super::WorkbenchEntityRelationDirection::Descendant,
            class_name: Some("SCR_TriggerEntity".to_string()),
            component_classes: Vec::new(),
            max_depth: 2,
        };
        let checkpoint_relation = super::WorkbenchEntityRelationFilter {
            class_name: Some("SCR_CheckpointEntity".to_string()),
            ..trigger_relation.clone()
        };

        let page = controller
            .search_entities(
                None,
                None,
                None,
                &[],
                Some(&trigger_relation),
                None,
                None,
                None,
                1,
            )
            .unwrap();
        let failure = controller
            .search_entities(
                None,
                None,
                None,
                &[],
                Some(&checkpoint_relation),
                None,
                None,
                page.next_cursor.as_deref(),
                1,
            )
            .unwrap_err();

        assert_eq!(failure.code, super::WorkbenchFailureCode::Protocol);
        peer.join().unwrap();
    }

    #[test]
    fn entity_search_returns_a_structured_unavailable_world_status() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["APIFunc"], "RST_WorkbenchSearchEntities");
            json!({
                "bridgeVersion": "1.51.0",
                "protocolVersion": 1,
                "status": "world-editor-unavailable",
                "worldPath": "",
                "results": "",
                "hasMore": false
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });

        let page = controller
            .search_entities(None, None, None, &[], None, None, None, None, 5)
            .unwrap();

        assert_eq!(page.status, "world-editor-unavailable");
        assert!(page.results.is_empty());
        assert!(!page.truncated);
        assert!(page.next_cursor.is_none());
        peer.join().unwrap();
    }

    #[test]
    fn entity_search_handler_initializes_its_paging_counters() {
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE
            .contains("int matched = 0; int named = 0; int returned = 0;"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE
            .contains("matched = matched + 1; if (!name.IsEmpty()) named = named + 1;"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE
            .contains("if (matched > req.offset + req.limit) { response.hasMore = true;"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE
            .contains("response.totalMatches = matched; response.namedMatches = named;"));
        assert!(super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("returned = returned + 1;"));
        assert!(!super::BRIDGE_ENTITY_SEARCH_SOURCE.contains("matched++"));
    }

    #[test]
    fn layer_state_reports_explicit_and_hierarchical_lock_facts() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchLayerState","subScene":2,"layerId":7})
            );
            json!({
                "bridgeVersion":"1.51.0",
                "protocolVersion":1,
                "status":"available",
                "subScene":2,
                "layerId":7,
                "layerPath":"Gameplay/LockedParent/Child",
                "visible":true,
                "explicitlyLocked":false,
                "lockedInHierarchy":true
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });

        let state = controller.layer_state(2, 7).unwrap();

        assert_eq!(state.status, "available");
        assert_eq!(state.layer_path, "Gameplay/LockedParent/Child");
        assert!(state.visible);
        assert!(!state.explicitly_locked);
        assert!(state.locked_in_hierarchy);
        peer.join().unwrap();
    }

    #[test]
    fn prefab_context_normalizes_provenance_ancestors_components_and_direct_overrides() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchInspectPrefab","entityId":"0x01","resourceName":"","memberId":""})
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"available","entity":"0x01|TestEntity|0|1","resourceName":"{GUID}Prefabs/Test.et","resourceReferenceKind":"external","contributorAddons":"BaseGame;MyAddon","ancestorResources":"{BASE_GUID}Prefabs/Base.et","prefabEditMode":true,"components":"0|MeshObject","componentProperties":"0|Enabled|bool|1|0|default;0|VisibleDistance|integer|250|1|direct;0|Offset|vector|1 2 3|1|direct;0|Scale|float|1.5|0|inherited;0|Label|string|Test|0|default","children":"0|Wheel|front-left","properties":"Mass|float|2000|1|direct;userScript|string||0|default;constructor|string||0|default;destructor|string||0|default;Name|string|Jeep|0|inherited","childCount":2})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });
        let context = controller
            .inspect_prefab_context(Some("0x01"), None, None)
            .unwrap();
        assert_eq!(context.contributor_addons, vec!["BaseGame", "MyAddon"]);
        assert_eq!(
            context.ancestor_resources,
            vec!["{BASE_GUID}Prefabs/Base.et"]
        );
        assert!(context.prefab_edit_mode);
        assert_eq!(context.components[0].class_name, "MeshObject");
        assert_eq!(context.components[0].property_count, Some(5));
        assert_eq!(context.components[0].direct_override_count, Some(2));
        assert_eq!(context.children[0].class_name, "Wheel");
        assert_eq!(context.children[0].member_id, "member:0");
        assert_eq!(context.children[0].name.as_deref(), Some("front-left"));
        assert!(context.properties[0].directly_overridden);
        assert_eq!(
            context.properties[0].value_origin,
            Some(super::WorkbenchPrefabPropertyOrigin::Direct)
        );
        assert_eq!(context.properties.len(), 2);
        assert_eq!(context.properties[1].path, "Name");
        assert_eq!(
            context.properties[1].value_origin,
            Some(super::WorkbenchPrefabPropertyOrigin::Inherited)
        );
        assert!(!context.properties[1].directly_overridden);
        peer.join().unwrap();
    }

    #[test]
    fn prefab_bridge_reports_stored_members_without_requiring_live_entity_sources() {
        let children = &super::BRIDGE_PREFAB_SOURCE[super::BRIDGE_PREFAB_SOURCE
            .find("response.childCount = target.GetNumChildren()")
            .unwrap()..];
        assert!(children.contains("BaseContainer child = target.GetChild(i + firstChildIndex);"));
        assert!(children.contains("child.GetClassName()"));
        assert!(!children.contains("IEntitySource child = IEntitySource.Cast(target.GetChild(i))"));
    }

    #[test]
    fn prefab_component_inspection_returns_the_requested_components_full_properties() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchInspectPrefab","entityId":"","resourceName":"{GUID}Prefabs/Test.et","memberId":""})
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"available","resourceName":"{GUID}Prefabs/Test.et","components":"0|MeshObject;1|Light","componentProperties":"0|Enabled|bool|1|0;0|Offset|vector|1 2 3|1;1|Intensity|float|500.25|0"})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });

        let result = controller
            .inspect_prefab_component(
                None,
                Some("{GUID}Prefabs/Test.et"),
                "cmp1:0:MeshObject",
                None,
            )
            .unwrap();

        assert_eq!(result.status, "available");
        assert_eq!(result.resource_name, "{GUID}Prefabs/Test.et");
        let component = result.component.unwrap();
        assert_eq!(component.class_name, "MeshObject");
        assert_eq!(component.properties.len(), 2);
        assert_eq!(component.properties[0].path, "Enabled");
        assert_eq!(component.properties[0].value, json!(true));
        assert_eq!(
            component.properties[1].value,
            json!({"x":1.0,"y":2.0,"z":3.0})
        );
        assert!(component.properties[1].directly_overridden);
        peer.join().unwrap();
    }

    #[test]
    fn prefab_component_inspection_reports_a_missing_component_without_losing_context() {
        let (port, peer) = start_peer(
            |_| json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"available","resourceName":"{GUID}Prefabs/Test.et","components":"0|MeshObject"}),
        );
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });

        let result = controller
            .inspect_prefab_component(None, Some("{GUID}Prefabs/Test.et"), "cmp1:9:Missing", None)
            .unwrap();

        assert_eq!(result.status, "component-not-found");
        assert!(result.component.is_none());
        peer.join().unwrap();
    }

    #[test]
    fn prefab_edit_component_inspection_issues_component_bound_property_descriptors() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchInspectPrefab","entityId":"0x01 {}","resourceName":"","memberId":""})
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"available","resourceName":"{GUID}Prefabs/Test.et","components":"0|MeshObject","componentProperties":"0|Enabled|bool|1|0|default"})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });

        let result = controller
            .inspect_prefab_component(Some("0x01 {}"), None, "cmp1:0:MeshObject", None)
            .unwrap();
        let component = result.component.unwrap();
        assert!(component.properties[0]
            .write_descriptor
            .as_deref()
            .is_some_and(|value| value.starts_with("prop2:")));

        peer.join().unwrap();
    }

    #[test]
    fn prefab_context_and_component_inspection_scope_to_a_returned_member() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchInspectPrefab","entityId":"","resourceName":"{GUID}Prefabs/Test.et","memberId":"member:0"})
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"available","resourceName":"{GUID}Prefabs/Test.et","memberId":"member:0","components":"0|MeshObject","componentProperties":"0|Enabled|bool|1|0|default","children":"0|Wheel|","properties":"coords|vector|1 2 3|1|direct","childCount":1})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });

        let context = controller
            .inspect_prefab_context(None, Some("{GUID}Prefabs/Test.et"), Some("member:0"))
            .unwrap();

        assert_eq!(context.member_id.as_deref(), Some("member:0"));
        assert_eq!(context.children[0].class_name, "Wheel");
        assert_eq!(
            context.properties[0].value_origin,
            Some(super::WorkbenchPrefabPropertyOrigin::Direct)
        );
        peer.join().unwrap();
    }

    #[test]
    fn prefab_create_preview_issues_one_use_confirmation_bound_to_entity_and_destination() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchCreatePrefab","entityId":"0x01","name":"Prefabs/New.et","confirm":false})
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"confirmation-required","entity":"0x01|TestEntity|0|1","destination":"Prefabs/New.et","destinationExists":false})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });
        let preview = controller
            .create_prefab("0x01", "Prefabs/New.et", None)
            .unwrap();
        assert_eq!(preview.status, "confirmation-required");
        assert_eq!(preview.destination.as_deref(), Some("Prefabs/New.et"));
        assert_eq!(preview.destination_exists, Some(false));
        assert!(preview.confirmation_token.is_some());
        peer.join().unwrap();
    }

    #[test]
    fn generic_prefab_create_preview_uses_the_native_workbench_path() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc":"RST_WorkbenchCreateGenericPrefab",
                    "name":"Prefabs/Generated/RST_Test.et",
                    "confirm":false
                })
            );
            json!({
                "bridgeVersion":"1.51.0",
                "protocolVersion":1,
                "status":"confirmation-required",
                "destination":"Prefabs/Generated/RST_Test.et",
                "destinationExists":false
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });

        let preview = controller
            .create_generic_prefab("Prefabs/Generated/RST_Test.et", None)
            .unwrap();

        assert_eq!(preview.status, "confirmation-required");
        assert_eq!(
            preview.destination.as_deref(),
            Some("Prefabs/Generated/RST_Test.et")
        );
        assert!(preview.confirmation_token.is_some());
        peer.join().unwrap();
    }

    #[test]
    fn prefab_resource_save_preview_is_bound_to_the_exact_resource() {
        let resource_name = "{1234567890ABCDEF}Prefabs/Test_GenericEntity.et";
        let (port, peer) = start_peer(move |request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc":"RST_WorkbenchSavePrefab",
                    "entityId":"",
                    "resourceName":resource_name,
                    "confirm":false
                })
            );
            json!({
                "bridgeVersion":"1.51.0",
                "protocolVersion":1,
                "status":"confirmation-required",
                "destinationExists":0
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });

        let preview = controller
            .save_prefab(None, Some(resource_name), None)
            .unwrap();

        assert_eq!(preview.status, "confirmation-required");
        assert_eq!(preview.destination_exists, Some(false));
        assert!(preview.confirmation_token.is_some());
        peer.join().unwrap();
    }

    #[test]
    fn prefab_resource_component_add_preview_is_bound_to_resource_and_class() {
        let resource_name = "{1234567890ABCDEF}Prefabs/Test_GenericEntity.et";
        let (port, peer) = start_peer(move |request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc":"RST_WorkbenchAddPrefabResourceComponent",
                    "resourceName":resource_name,
                    "className":"ScriptComponent",
                    "confirm":false
                })
            );
            json!({
                "bridgeVersion":"1.51.0",
                "protocolVersion":1,
                "status":"confirmation-required",
                "resourceName":resource_name,
                "componentIndex":-1,
                "templateSaved":0
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });

        let preview = controller
            .add_prefab_resource_component(resource_name, "ScriptComponent", None)
            .unwrap();

        assert_eq!(preview.status, "confirmation-required");
        assert_eq!(preview.persistence_path, "workbench-resource");
        assert!(!preview.template_saved);
        assert!(preview.confirmation_token.is_some());
        peer.join().unwrap();
    }

    #[test]
    fn prefab_resource_component_remove_preview_is_bound_to_inspected_identity() {
        let resource_name = "{1234567890ABCDEF}Prefabs/Test_GenericEntity.et";
        let (port, peer) = start_peer(move |request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc":"RST_WorkbenchRemovePrefabResourceComponent",
                    "resourceName":resource_name,
                    "className":"ScriptComponent",
                    "componentIndex":0,
                    "confirm":false
                })
            );
            json!({
                "bridgeVersion":"1.51.0",
                "protocolVersion":1,
                "status":"confirmation-required",
                "resourceName":resource_name,
                "componentIndex":0,
                "componentClass":"ScriptComponent",
                "templateSaved":0
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });

        let preview = controller
            .remove_prefab_resource_component(resource_name, "cmp1:0:ScriptComponent", None)
            .unwrap();

        assert_eq!(preview.status, "confirmation-required");
        assert_eq!(
            preview.component_id.as_deref(),
            Some("cmp1:0:ScriptComponent")
        );
        assert!(preview.confirmation_token.is_some());
        peer.join().unwrap();
    }

    #[test]
    fn prefab_resource_property_preview_is_bound_to_resource_descriptor_and_value() {
        let resource_name = "{1234567890ABCDEF}Prefabs/Test_GenericEntity.et";
        let (port, peer) = start_peer(move |request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc":"RST_WorkbenchSetPrefabResourceProperty",
                    "resourceName":resource_name,
                    "componentIndex":-1,
                    "componentClass":"",
                    "propertyName":"scale",
                    "expectedValue":"1",
                    "value":"2",
                    "confirm":false
                })
            );
            json!({
                "bridgeVersion":"1.51.0",
                "protocolVersion":1,
                "status":"confirmation-required",
                "resourceName":resource_name,
                "templateSaved":0
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });
        controller
            .property_write_descriptors
            .lock()
            .unwrap()
            .insert(
                "prop2:resource".to_string(),
                super::PropertyWriteDescriptor {
                    entity_id: resource_name.to_string(),
                    component_id: None,
                    property_name: "scale".to_string(),
                    data_type: "float".to_string(),
                    observed_value: "1".to_string(),
                    issued: Instant::now(),
                },
            );

        let preview = controller
            .set_prefab_resource_property(resource_name, None, "prop2:resource", json!(2.0), None)
            .unwrap();

        assert_eq!(preview.status, "confirmation-required");
        assert!(!preview.template_saved);
        assert!(preview.confirmation_token.is_some());
        peer.join().unwrap();
    }

    #[test]
    fn viewport_context_keeps_compact_cursor_result_separate_from_optional_ray_diagnostics() {
        let response = || {
            json!({
                "bridgeVersion":"1.51.0", "protocolVersion":1, "status":"available",
                "width":1920, "height":1080, "mouseX":960, "mouseY":540, "mouseInside":true,
                "cameraX":10.0, "cameraY":20.0, "cameraZ":30.0,
                "cameraDirectionX":0.0, "cameraDirectionY":0.0, "cameraDirectionZ":1.0,
                "startX":10.0, "startY":20.0, "startZ":30.0,
                "endX":100.0, "endY":40.0, "endZ":300.0,
                "directionX":0.3, "directionY":-0.1, "directionZ":0.9
            })
        };
        let (port, peer) = start_peer(move |request| {
            assert_eq!(request, json!({"APIFunc":"RST_WorkbenchViewportContext"}));
            response()
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });
        let compact = controller
            .viewport_context(super::WorkbenchViewportContextOptions { include_ray: false })
            .unwrap();
        assert_eq!(compact.mouse_world_position.unwrap().x, 100.0);
        assert!(compact.width.is_none());
        assert!(compact.ray_start.is_none());
        peer.join().unwrap();

        let (port, peer) = start_peer(move |_| response());
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });
        let detailed = controller
            .viewport_context(super::WorkbenchViewportContextOptions { include_ray: true })
            .unwrap();
        assert_eq!(detailed.width, Some(1920));
        assert_eq!(detailed.camera_direction.unwrap().z, 1.0);
        assert_eq!(detailed.ray_end.unwrap().z, 300.0);
        peer.join().unwrap();
    }

    #[test]
    fn trace_serializes_bounded_policy_and_normalizes_distance_and_ocean_hit() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc":"RST_WorkbenchTrace", "startX":0.0, "startY":10.0, "startZ":0.0,
                    "endX":0.0, "endY":-10.0, "endZ":0.0, "shape":"sphere", "radius":2.0,
                    "minsX":0.0, "minsY":0.0, "minsZ":0.0, "maxsX":0.0, "maxsY":0.0, "maxsZ":0.0,
                    "entities":false, "terrain":false, "ocean":true, "targetLayers":0
                })
            );
            json!({
                "bridgeVersion":"1.51.0", "protocolVersion":1, "status":"available", "hit":true,
                "fraction":0.5, "distance":10.0, "hitX":0.0, "hitY":0.0, "hitZ":0.0,
                "normalX":0.0, "normalY":1.0, "normalZ":0.0, "kind":"ocean"
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..Default::default()
            },
            ..Default::default()
        });
        let hit = controller
            .trace(super::WorkbenchTraceOptions {
                start: super::WorkbenchEntityPosition {
                    x: 0.0,
                    y: 10.0,
                    z: 0.0,
                },
                end: super::WorkbenchEntityPosition {
                    x: 0.0,
                    y: -10.0,
                    z: 0.0,
                },
                shape: super::WorkbenchTraceShape::Sphere,
                radius: Some(2.0),
                box_mins: None,
                box_maxs: None,
                entities: false,
                terrain: false,
                ocean: true,
                target_layers: None,
            })
            .unwrap();
        assert_eq!(hit.distance, Some(10.0));
        assert_eq!(hit.kind, Some(super::WorkbenchTraceHitKind::Ocean));
        peer.join().unwrap();
    }

    #[test]
    fn trace_rejects_unbounded_or_invalid_sweep_requests_before_gateway_dispatch() {
        let controller = super::WorkbenchController::new(Default::default());
        let valid_position = super::WorkbenchEntityPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        for options in [
            super::WorkbenchTraceOptions {
                start: valid_position.clone(),
                end: super::WorkbenchEntityPosition {
                    x: 10_001.0,
                    y: 0.0,
                    z: 0.0,
                },
                shape: super::WorkbenchTraceShape::Line,
                radius: None,
                box_mins: None,
                box_maxs: None,
                entities: true,
                terrain: false,
                ocean: false,
                target_layers: None,
            },
            super::WorkbenchTraceOptions {
                start: valid_position.clone(),
                end: valid_position.clone(),
                shape: super::WorkbenchTraceShape::Box,
                radius: None,
                box_mins: Some(valid_position.clone()),
                box_maxs: Some(valid_position.clone()),
                entities: true,
                terrain: false,
                ocean: false,
                target_layers: None,
            },
            super::WorkbenchTraceOptions {
                start: valid_position.clone(),
                end: valid_position.clone(),
                shape: super::WorkbenchTraceShape::Line,
                radius: None,
                box_mins: None,
                box_maxs: None,
                entities: false,
                terrain: true,
                ocean: false,
                target_layers: Some(1),
            },
        ] {
            assert_eq!(
                controller.trace(options).unwrap_err().code,
                super::WorkbenchFailureCode::Protocol
            );
        }
    }

    #[test]
    fn terrain_sampling_returns_bounded_grid_metadata_and_derived_summary() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc": "RST_WorkbenchSampleTerrain",
                    "centerX": 100.0,
                    "centerZ": 200.0,
                    "halfExtentMeters": 30.0,
                    "spacingMeters": 1.0,
                    "includeWater": true
                })
            );
            json!({
                "bridgeVersion": "1.51.0",
                "protocolVersion": 1,
                "status": "available",
                "centerX": 100.0,
                "centerZ": 200.0,
                "halfExtentMeters": 30.0,
                "requestedSpacingMeters": 1.0,
                "effectiveSpacingMeters": 10.0,
                "spacingClamped": true,
                "gridOriginX": 70.0,
                "gridOriginZ": 170.0,
                "gridWidth": 2,
                "gridHeight": 2,
                "heights": "10;20;~;30",
                "waterTypes": "n;p;~;r",
                "waterSurfaceHeights": "~;22;~;35",
                "waterDepthsAboveTerrain": "~;2;~;5",
                "boundsMinX": 0.0,
                "boundsMinY": -5.0,
                "boundsMinZ": 0.0,
                "boundsMaxX": 10240.0,
                "boundsMaxY": 500.0,
                "boundsMaxZ": 10240.0,
                "heightmapResolutionX": 1024,
                "heightmapResolutionZ": 1024,
                "nativeSpacingMeters": 10.0,
                "tileCountX": 8,
                "tileCountZ": 8
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });

        let sample = controller
            .sample_terrain(super::WorkbenchTerrainSampleOptions {
                center_x: 100.0,
                center_z: 200.0,
                half_extent_meters: 30.0,
                spacing_meters: Some(1.0),
                include_water: true,
            })
            .unwrap();

        assert_eq!(sample.status, "available");
        let terrain = sample.terrain.expect("terrain metadata");
        assert_eq!(terrain.native_spacing_meters, 10.0);
        assert_eq!(terrain.heightmap_resolution_x, 1024);
        let grid = sample.grid.expect("terrain grid");
        assert!(grid.spacing_clamped);
        assert_eq!(grid.effective_spacing_meters, 10.0);
        assert_eq!(grid.heights, vec![Some(10.0), Some(20.0), None, Some(30.0)]);
        let summary = sample.summary.expect("terrain summary");
        assert_eq!(summary.valid_sample_count, 3);
        assert_eq!(summary.minimum_height, Some(10.0));
        assert_eq!(summary.maximum_height, Some(30.0));
        assert_eq!(summary.mean_height, Some(20.0));
        assert_eq!(summary.elevation_range, Some(20.0));
        assert_eq!(summary.steepest_adjacent_slope_degrees, Some(45.0));
        let water = sample.water.expect("water grid");
        assert_eq!(
            water.types,
            vec![
                Some(super::WorkbenchTerrainWaterType::None),
                Some(super::WorkbenchTerrainWaterType::Pond),
                None,
                Some(super::WorkbenchTerrainWaterType::River),
            ]
        );
        let water_summary = sample.water_summary.expect("water summary");
        assert_eq!(water_summary.wet_sample_count, 2);
        assert_eq!(water_summary.pond_sample_count, 1);
        assert_eq!(water_summary.river_sample_count, 1);
        assert_eq!(water_summary.maximum_depth_above_terrain, Some(5.0));
        peer.join().unwrap();
    }

    #[test]
    fn terrain_sampling_rejects_invalid_parameter_values_before_gateway_dispatch() {
        let controller = super::WorkbenchController::new(Default::default());
        for options in [
            super::WorkbenchTerrainSampleOptions {
                center_x: f32::NAN,
                center_z: 0.0,
                half_extent_meters: 30.0,
                spacing_meters: None,
                include_water: false,
            },
            super::WorkbenchTerrainSampleOptions {
                center_x: 0.0,
                center_z: 0.0,
                half_extent_meters: 0.0,
                spacing_meters: None,
                include_water: false,
            },
            super::WorkbenchTerrainSampleOptions {
                center_x: 0.0,
                center_z: 0.0,
                half_extent_meters: 30.0,
                spacing_meters: Some(501.0),
                include_water: false,
            },
        ] {
            assert_eq!(
                controller.sample_terrain(options).unwrap_err().code,
                super::WorkbenchFailureCode::Protocol
            );
        }
    }

    #[test]
    fn terrain_sampling_requires_the_matching_managed_handler_version() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc": "RST_WorkbenchSampleTerrain",
                    "centerX": 0.0,
                    "centerZ": 0.0,
                    "halfExtentMeters": 30.0,
                    "spacingMeters": 0.0,
                    "includeWater": false
                })
            );
            json!({"bridgeVersion":"1.19.0","protocolVersion":1,"status":"terrain-unavailable"})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });

        assert_eq!(
            controller
                .sample_terrain(super::WorkbenchTerrainSampleOptions {
                    center_x: 0.0,
                    center_z: 0.0,
                    half_extent_meters: 30.0,
                    spacing_meters: None,
                    include_water: false,
                })
                .unwrap_err()
                .code,
            super::WorkbenchFailureCode::Protocol
        );
        peer.join().unwrap();
    }

    #[test]
    fn terrain_handler_aligns_coarser_spacing_to_native_lattice_and_enforces_limits() {
        assert!(super::BRIDGE_TERRAIN_SAMPLE_SOURCE.contains("typedRequest.spacingMeters > 500"));
        assert!(super::BRIDGE_TERRAIN_SAMPLE_SOURCE.contains(
            "Math.Ceil(typedRequest.spacingMeters / response.nativeSpacingMeters) * response.nativeSpacingMeters"
        ));
    }

    #[test]
    fn clear_selection_accepts_workbenchs_numeric_boolean_fields() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request, json!({"APIFunc": "RST_WorkbenchClearSelection"}));
            json!({
                "bridgeVersion": "1.10.0",
                "protocolVersion": 1,
                "editorAvailable": 1,
                "status": "available",
                "selectedCount": 0,
                "selectedEntities": "",
                "selectedEntitiesTruncated": 0
            })
        });
        let root = test_root("clear-selection-numeric-booleans");
        fs::create_dir_all(&root).unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let selection = controller.clear_selection().unwrap();

        assert!(selection.editor_available);
        assert_eq!(selection.selected_count, 0);
        assert!(selection.selected_entities.is_empty());
        assert!(!selection.selected_entities_truncated);
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn set_selection_uses_one_exact_entity_identity() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc": "RST_WorkbenchSetSelection",
                    "entityId": "0x0000000000000003 {}"
                })
            );
            json!({
                "bridgeVersion": "1.11.0",
                "protocolVersion": 1,
                "status": "selected",
                "entity": "0x0000000000000003 {}|GenericTerrainEntity|0|0|10|20|30"
            })
        });
        let root = test_root("set-selection-single-entity");
        fs::create_dir_all(&root).unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let result = controller.set_selection("0x0000000000000003 {}").unwrap();

        assert_eq!(result.status, "selected");
        assert_eq!(result.entity.unwrap().class_name, "GenericTerrainEntity");
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn create_entity_uses_explicit_resource_position_angles_and_layer() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchCreateEntity","resourceName":"{GUID}Prefabs/Test.et","targetIsResource":true,"subScene":2,"x":1.0,"y":2.0,"z":3.0,"pitch":4.0,"yaw":5.0,"roll":6.0,"layerId":7,"name":"Created"})
            );
            json!({"bridgeVersion":"1.17.0","protocolVersion":1,"status":"created","entity":"0x01 {}|TestEntity|0|7|1|2|3"})
        });
        let root = test_root("create-entity");
        fs::create_dir_all(&root).unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });
        let result = controller
            .create_entity(super::WorkbenchCreateEntityOptions {
                target: "{GUID}Prefabs/Test.et".to_string(),
                target_is_resource: true,
                sub_scene: 2,
                position: super::WorkbenchEntityPosition {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                angles: super::WorkbenchEntityPosition {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                },
                layer_id: 7,
                name: Some("Created".to_string()),
            })
            .unwrap();
        assert_eq!(result.status, "created");
        assert_eq!(result.entity.unwrap().layer_id, 7);
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn create_entity_bridge_distinguishes_resource_loading_from_editor_rejection() {
        assert!(super::BRIDGE_ENTITY_MUTATION_SOURCE
            .contains("response.status = \"resource-load-failed\""));
        let missing_entity = super::BRIDGE_ENTITY_MUTATION_SOURCE
            .find("if (!entity)")
            .unwrap();
        let rejected = super::BRIDGE_ENTITY_MUTATION_SOURCE
            .find("response.status = \"create-rejected\"")
            .unwrap();
        assert!(missing_entity < rejected);
    }

    #[test]
    fn entity_mutations_resolve_the_exact_selected_entity_before_the_general_enumeration() {
        assert!(super::BRIDGE_ENTITY_MUTATION_SOURCE.contains("api.GetSelectedEntitiesCount()"));
        assert!(super::BRIDGE_ENTITY_MUTATION_SOURCE.contains("api.GetSelectedEntity(i)"));
        assert!(super::BRIDGE_ENTITY_MUTATION_SOURCE.contains("api.GetEditorEntityCount()"));
        assert!(super::BRIDGE_ENTITY_MUTATION_SOURCE.contains("RegV(\"entityId\")"));
    }

    #[test]
    fn entity_transform_and_parenting_use_exact_id_requests() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchMoveEntity","entityId":"0x01 {}","x":10.0,"y":20.0,"z":30.0})
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"moved","entity":"0x01 {}|TestEntity|0|7|10|20|30"})
        });
        let root = test_root("move-entity");
        fs::create_dir_all(&root).unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });
        let result = controller
            .move_entity(
                "0x01 {}",
                super::WorkbenchEntityPosition {
                    x: 10.0,
                    y: 20.0,
                    z: 30.0,
                },
            )
            .unwrap();
        assert_eq!(result.status, "moved");
        assert_eq!(result.entity.unwrap().position.unwrap().x, 10.0);
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_entity_uses_native_clone_with_an_explicit_destination() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchDuplicateEntity","entityId":"0x01 {}","x":11.0,"y":22.0,"z":33.0,"name":"Copy"})
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"duplicated","entity":"0x02 {}|TestEntity|0|7|11|22|33"})
        });
        let root = test_root("duplicate-entity");
        fs::create_dir_all(&root).unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });
        let result = controller
            .duplicate_entity(
                "0x01 {}",
                super::WorkbenchEntityPosition {
                    x: 11.0,
                    y: 22.0,
                    z: 33.0,
                },
                Some("Copy"),
            )
            .unwrap();
        assert_eq!(result.entity.unwrap().entity_id, "0x02 {}");
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_entity_preview_returns_a_one_use_confirmation_token() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchDeleteEntity","entityId":"0x01 {}","confirm":false})
            );
            json!({"bridgeVersion":"1.17.0","protocolVersion":1,"status":"confirmation-required","entity":"0x01 {}|TestEntity|0|7|1|2|3"})
        });
        let root = test_root("delete-entity-preview");
        fs::create_dir_all(&root).unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });
        let result = controller.delete_entity("0x01 {}", None).unwrap();
        assert_eq!(result.status, "confirmation-required");
        assert!(result.confirmation_token.is_some());
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn entity_inspection_accepts_workbenchs_numeric_hierarchy_flags() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request["APIFunc"], "RST_WorkbenchInspectEntity");
            json!({
                "bridgeVersion": "1.11.0",
                "protocolVersion": 1,
                "editorAvailable": 1,
                "status": "available",
                "entity": "0x0000000000000003 {}|GenericTerrainEntity|0|0|1|2|3",
                "resourceReferenceKind": "world-subscene",
                "resourceName": "$Example:worlds/Test.ent",
                "contributorAddons": "Example",
                "contributorAddonsTruncated": 0,
                "ancestors": "",
                "ancestorsTruncated": 0,
                "children": "",
                "childrenTruncated": 0,
                "components": "0|GRAY_TEST",
                "componentProperties": "0|Enabled|bool|1|0;0|m_iNumTest|integer|99|1",
                "properties": "coords|vector|1 2 3|1;scale|float|1|0;m_bTestBool|bool|1|0;m_iNumTest|integer|101|1",
                "propertiesTruncated": 0
            })
        });
        let root = test_root("entity-inspection-numeric-hierarchy-flags");
        fs::create_dir_all(&root).unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let inspection = controller.inspect_entity("0x0000000000000003 {}").unwrap();

        assert!(inspection.editor_available);
        let entity = inspection.entity.unwrap();
        assert_eq!(entity.class_name, "GenericTerrainEntity");
        assert_eq!(
            entity.position,
            Some(super::WorkbenchEntityPosition {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );
        assert!(!inspection.ancestors_truncated);
        assert!(!inspection.children_truncated);
        assert_eq!(inspection.resource_reference_kind, "world-subscene");
        assert_eq!(
            inspection.resource_name.as_deref(),
            Some("$Example:worlds/Test.ent")
        );
        assert_eq!(inspection.contributor_addons, vec!["Example"]);
        assert!(!inspection.contributor_addons_truncated);
        assert_eq!(inspection.components.len(), 1);
        assert_eq!(inspection.components[0].class_name, "GRAY_TEST");
        assert_eq!(inspection.components[0].property_count, Some(2));
        assert_eq!(inspection.components[0].direct_override_count, Some(1));
        assert_eq!(
            inspection.properties[0].value,
            json!({"x": 1.0, "y": 2.0, "z": 3.0})
        );
        assert_eq!(inspection.properties[1].value, json!(1.0));
        assert_eq!(inspection.properties[2].value, json!(101));
        assert!(inspection.properties[2].directly_overridden);
        assert!(inspection
            .properties
            .iter()
            .all(|property| property.path != "m_bTestBool"));
        assert!(!inspection.properties_truncated);
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn entity_inspection_bridge_uses_the_component_container_fallback() {
        assert!(super::BRIDGE_ENTITY_INSPECT_SOURCE
            .contains("int componentCount = ComponentCount(target);"));
        assert!(
            super::BRIDGE_ENTITY_INSPECT_SOURCE.contains("entity.GetObjectArray(\"components\")")
        );
    }

    #[test]
    fn play_session_preserves_direct_observations_without_claiming_runtime_certainty() {
        assert_eq!(
            super::play_session(&Some("editing".to_string()), true, true),
            Some(super::WorkbenchPlaySession::Unknown)
        );
        assert_eq!(
            super::play_session(&None, true, true),
            Some(super::WorkbenchPlaySession::Unknown)
        );
        assert_eq!(
            super::play_session(&Some("likely-running".to_string()), true, false),
            Some(super::WorkbenchPlaySession::LikelyRunning)
        );
        assert_eq!(
            super::play_session(&None, false, false),
            Some(super::WorkbenchPlaySession::Unavailable)
        );
        assert_eq!(
            super::play_session(&Some("running".to_string()), true, false),
            None
        );
    }

    #[test]
    fn native_status_uses_the_documented_net_api_transaction() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(request, json!({"APIFunc": "IsWorkbenchRunning"}));
            json!({"IsRunning": true, "ScriptsCompiled": false})
        });
        let gateway = test_gateway(port);

        assert_eq!(
            gateway.status().unwrap(),
            WorkbenchStatus {
                is_running: true,
                scripts_compiled: false,
            }
        );
        peer.join().unwrap();
    }

    #[test]
    fn native_validation_uses_the_fixed_workbench_profile_and_normalizes_duplicates() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc": "ValidateScripts", "Configuration": "WORKBENCH"})
            );
            json!({
                "Success": false,
                "Errors": [
                    {"error": "broken", "file": "scripts/A.c", "fileAbs": "C:\\Addon\\scripts\\A.c", "addon": "Addon", "line": 7},
                    {"error": "broken", "file": "scripts/A.c", "fileAbs": "C:\\Addon\\scripts\\A.c", "addon": "Addon", "line": 7}
                ],
                "Warnings": [
                    {"error": "broken", "file": "scripts/A.c", "fileAbs": "C:\\Addon\\scripts\\A.c", "addon": "Addon", "line": 7},
                    {"error": "unused", "file": "scripts/B.c", "line": 4}
                ]
            })
        });
        let gateway = test_gateway(port);

        let result = gateway.validate_scripts().unwrap();

        assert!(!result.success);
        assert_eq!(result.profile, "WORKBENCH");
        assert_eq!(result.diagnostics.len(), 2);
        assert_eq!(
            result.diagnostics[0].severity,
            WorkbenchDiagnosticSeverity::Error
        );
        assert_eq!(
            result.diagnostics[0].location,
            WorkbenchDiagnosticLocation {
                file: "scripts/A.c".to_string(),
                file_abs: Some("C:\\Addon\\scripts\\A.c".into()),
                addon: Some("Addon".to_string()),
                line: 7,
            }
        );
        assert_eq!(
            result.diagnostics[1].severity,
            WorkbenchDiagnosticSeverity::Warning
        );
        peer.join().unwrap();
    }

    #[test]
    fn malformed_native_status_is_a_protocol_failure() {
        let (port, peer) = start_peer(|_| json!({"IsRunning": "yes"}));
        let gateway = test_gateway(port);

        assert_eq!(
            gateway.status().unwrap_err().code,
            WorkbenchFailureCode::Protocol
        );
        peer.join().unwrap();
    }

    #[test]
    fn native_gateway_accepts_fragmented_responses() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut version = [0_u8; 4];
            stream.read_exact(&mut version).unwrap();
            let _client = read_string(&mut stream);
            let _content_type = read_string(&mut stream);
            let _payload = read_string(&mut stream);
            let response = [
                &("Ok".len() as i32).to_le_bytes()[..],
                b"Ok",
                &(r#"{"IsRunning":true,"ScriptsCompiled":true}"#.len() as i32).to_le_bytes()[..],
                br#"{"IsRunning":true,"ScriptsCompiled":true}"#,
            ]
            .concat();
            for byte in response {
                stream.write_all(&[byte]).unwrap();
                stream.flush().unwrap();
            }
        });
        let gateway = test_gateway(port);

        assert_eq!(
            gateway.status().unwrap(),
            WorkbenchStatus {
                is_running: true,
                scripts_compiled: true,
            }
        );
        peer.join().unwrap();
    }

    #[test]
    fn premature_native_close_is_a_protocol_failure() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut version = [0_u8; 4];
            stream.read_exact(&mut version).unwrap();
            let _client = read_string(&mut stream);
            let _content_type = read_string(&mut stream);
            let _payload = read_string(&mut stream);
            stream.write_all(&4_i32.to_le_bytes()).unwrap();
            stream.write_all(b"Ok").unwrap();
        });
        let gateway = test_gateway(port);

        assert_eq!(
            gateway.status().unwrap_err().code,
            WorkbenchFailureCode::Protocol
        );
        peer.join().unwrap();
    }

    #[test]
    fn native_workbench_error_is_distinct_from_protocol_failure() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut version = [0_u8; 4];
            stream.read_exact(&mut version).unwrap();
            let _client = read_string(&mut stream);
            let _content_type = read_string(&mut stream);
            let _payload = read_string(&mut stream);
            write_string(&mut stream, "WorkbenchError");
            write_string(&mut stream, "{}");
        });
        let gateway = test_gateway(port);

        assert_eq!(
            gateway.status().unwrap_err().code,
            WorkbenchFailureCode::WorkbenchError
        );
        peer.join().unwrap();
    }

    #[test]
    fn refused_native_endpoint_is_unavailable() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let gateway = test_gateway(port);

        assert_eq!(
            gateway.status().unwrap_err().code,
            WorkbenchFailureCode::Unavailable
        );
    }

    #[test]
    fn oversized_native_response_is_rejected_before_allocation() {
        use std::net::TcpListener;

        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut version = [0_u8; 4];
            stream.read_exact(&mut version).unwrap();
            let _client = read_string(&mut stream);
            let _content_type = read_string(&mut stream);
            let _payload = read_string(&mut stream);
            write_string(&mut stream, "Ok");
            stream
                .write_all(&((super::MAX_RESPONSE_BYTES as i32) + 1).to_le_bytes())
                .unwrap();
        });
        let gateway = test_gateway(port);

        assert_eq!(
            gateway.status().unwrap_err().code,
            WorkbenchFailureCode::Protocol
        );
        peer.join().unwrap();
    }

    #[test]
    fn managed_installation_preserves_unknown_profile_scripts() {
        let root = std::env::temp_dir().join(format!(
            "rst-workbench-install-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let profile = root
            .join("Documents")
            .join("My Games")
            .join("ArmaReforgerWorkbench")
            .join("profile");
        let bridge = profile
            .join("scripts")
            .join("WorkbenchGame")
            .join("reforger-script-tools");
        fs::create_dir_all(&bridge).unwrap();
        fs::write(bridge.join("user-script.c"), "keep me").unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        controller.write_managed_files(&bridge).unwrap();

        assert_eq!(
            fs::read_to_string(bridge.join("user-script.c")).unwrap(),
            "keep me"
        );
        assert!(bridge.join("reforger-script-tools.manifest.json").is_file());
        assert!(controller.bridge_disk_status(&bridge).installed);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migrates_the_known_flat_profile_package_into_workbench_game() {
        let root = test_root("migrate-flat-profile-package");
        let scripts = root.join("scripts");
        let legacy = scripts.join("reforger-script-tools");
        let destination = scripts.join("WorkbenchGame").join("reforger-script-tools");
        let controller =
            super::WorkbenchController::new(super::WorkbenchControllerOptions::default());
        controller.write_managed_files(&legacy).unwrap();
        fs::write(legacy.join("user-script.c"), "preserve me").unwrap();

        assert!(controller
            .migrate_legacy_bridge(&legacy, &destination)
            .unwrap());
        assert!(destination
            .join("reforger-script-tools.manifest.json")
            .is_file());
        for (name, _) in super::bridge_payload() {
            assert!(destination.join(name).is_file());
            assert!(!legacy.join(name).exists());
        }
        assert_eq!(
            fs::read_to_string(legacy.join("user-script.c")).unwrap(),
            "preserve me"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_install_validates_scripts_without_probing_the_new_handler() {
        let root = test_root("first-install-reload-required");
        let profile = root
            .join("Documents")
            .join("My Games")
            .join("ArmaReforgerWorkbench")
            .join("profile");
        fs::create_dir_all(&profile).unwrap();
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            for (expected, response) in [
                (
                    json!({"APIFunc":"IsWorkbenchRunning"}),
                    json!({"IsRunning":true,"ScriptsCompiled":true}),
                ),
                (
                    json!({"APIFunc":"ValidateScripts","Configuration":"WORKBENCH"}),
                    json!({"Success":true,"Errors":[],"Warnings":[]}),
                ),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut version = [0_u8; 4];
                stream.read_exact(&mut version).unwrap();
                let _client = read_string(&mut stream);
                let _content_type = read_string(&mut stream);
                let request: Value = serde_json::from_str(&read_string(&mut stream)).unwrap();
                assert_eq!(request, expected);
                write_string(&mut stream, "Ok");
                write_string(&mut stream, &response.to_string());
            }
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                validation_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let result = controller
            .install_bridge(super::WorkbenchInstallAuthorization::UserApprovedFirstInstall)
            .unwrap();

        assert_eq!(result.installed_version, super::WORKBENCH_BRIDGE_VERSION);
        assert!(!result.activated);
        assert_eq!(result.active_version, None);
        assert_eq!(result.protocol_version, None);
        assert_eq!(result.managed_files, super::bridge_payload().len());
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_consent_install_does_not_create_a_first_installation() {
        let root = test_root("install-consent-required");
        let profile = root
            .join("Documents")
            .join("My Games")
            .join("ArmaReforgerWorkbench")
            .join("profile");
        let bridge = profile.join("scripts").join("reforger-script-tools");
        fs::create_dir_all(&profile).unwrap();
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut version = [0_u8; 4];
            stream.read_exact(&mut version).unwrap();
            let _client = read_string(&mut stream);
            let _content_type = read_string(&mut stream);
            let request: Value = serde_json::from_str(&read_string(&mut stream)).unwrap();
            assert_eq!(request, json!({"APIFunc":"IsWorkbenchRunning"}));
            write_string(&mut stream, "Ok");
            write_string(
                &mut stream,
                &json!({"IsRunning":true,"ScriptsCompiled":true}).to_string(),
            );
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                validation_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let failure = controller
            .install_bridge(super::WorkbenchInstallAuthorization::ExistingConsent)
            .unwrap_err();

        assert_eq!(failure.code, super::WorkbenchFailureCode::ConsentRequired);
        assert!(!bridge.exists());
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_status_probe_does_not_run_managed_maintenance() {
        let root = test_root("native-status-no-maintenance");
        let bridge = root
            .join("Documents")
            .join("My Games")
            .join("ArmaReforgerWorkbench")
            .join("profile")
            .join("scripts")
            .join("reforger-script-tools");
        fs::create_dir_all(&bridge).unwrap();
        fs::write(
            bridge.join("reforger-script-tools.manifest.json"),
            serde_json::to_vec_pretty(&super::BridgeManifest {
                bridge_version: super::WORKBENCH_BRIDGE_VERSION.to_string(),
                protocol_version: super::WORKBENCH_BRIDGE_PROTOCOL_VERSION,
                files: Vec::new(),
            })
            .unwrap(),
        )
        .unwrap();
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut version = [0_u8; 4];
            stream.read_exact(&mut version).unwrap();
            let _client = read_string(&mut stream);
            let _content_type = read_string(&mut stream);
            let request: Value = serde_json::from_str(&read_string(&mut stream)).unwrap();
            assert_eq!(request, json!({"APIFunc":"IsWorkbenchRunning"}));
            write_string(&mut stream, "Ok");
            write_string(
                &mut stream,
                &json!({"IsRunning":true,"ScriptsCompiled":false}).to_string(),
            );
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                validation_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        controller.native_status().unwrap();

        assert!(
            !bridge.join("RST_WorkbenchCapabilities.c").exists(),
            "heartbeat status must not repair or retry the managed package"
        );
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn automatic_maintenance_never_downgrades_a_newer_managed_package() {
        let root = std::env::temp_dir().join(format!(
            "rst-workbench-newer-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bridge = root.join("bridge");
        fs::create_dir_all(&bridge).unwrap();
        fs::write(bridge.join("future.c"), "future package").unwrap();
        fs::write(
            bridge.join("reforger-script-tools.manifest.json"),
            serde_json::to_vec_pretty(&super::BridgeManifest {
                bridge_version: "9.0.0".to_string(),
                protocol_version: 9,
                files: vec![super::BridgeManifestFile {
                    name: "future.c".to_string(),
                    sha256: super::sha256(b"future package"),
                }],
            })
            .unwrap(),
        )
        .unwrap();
        let controller =
            super::WorkbenchController::new(super::WorkbenchControllerOptions::default());

        controller.repair_managed_files(&bridge).unwrap();

        assert_eq!(
            fs::read_to_string(bridge.join("future.c")).unwrap(),
            "future package"
        );
        assert!(!bridge.join("RST_WorkbenchCapabilities.c").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_install_is_idempotent_and_does_not_downgrade_a_newer_package() {
        let root = test_root("install-newer");
        let profile = root
            .join("Documents")
            .join("My Games")
            .join("ArmaReforgerWorkbench")
            .join("profile");
        let bridge = profile
            .join("scripts")
            .join("WorkbenchGame")
            .join("reforger-script-tools");
        fs::create_dir_all(&bridge).unwrap();
        fs::write(bridge.join("future.c"), "future package").unwrap();
        fs::write(
            bridge.join("reforger-script-tools.manifest.json"),
            serde_json::to_vec_pretty(&super::BridgeManifest {
                bridge_version: "9.0.0".to_string(),
                protocol_version: 9,
                files: vec![super::BridgeManifestFile {
                    name: "future.c".to_string(),
                    sha256: super::sha256(b"future package"),
                }],
            })
            .unwrap(),
        )
        .unwrap();
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            for (expected, response) in [
                (
                    json!({"APIFunc":"IsWorkbenchRunning"}),
                    json!({"IsRunning":true,"ScriptsCompiled":true}),
                ),
                (
                    json!({"APIFunc":"RST_WorkbenchCapabilities"}),
                    json!({"bridgeVersion":"9.0.0","protocolVersion":9,"capabilities":"future"}),
                ),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut version = [0_u8; 4];
                stream.read_exact(&mut version).unwrap();
                let _client = read_string(&mut stream);
                let _content_type = read_string(&mut stream);
                let request: Value = serde_json::from_str(&read_string(&mut stream)).unwrap();
                assert_eq!(request, expected);
                write_string(&mut stream, "Ok");
                write_string(&mut stream, &response.to_string());
            }
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                validation_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let result = controller
            .install_bridge(super::WorkbenchInstallAuthorization::ExistingConsent)
            .unwrap();

        assert_eq!(result.installed_version, "9.0.0");
        assert!(!result.activated);
        assert_eq!(
            fs::read_to_string(bridge.join("future.c")).unwrap(),
            "future package"
        );
        assert!(!bridge.join("RST_WorkbenchCapabilities.c").exists());
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn managed_version_order_uses_semver_and_preserves_unknown_installed_versions() {
        use std::cmp::Ordering;

        assert_eq!(
            super::version_order("1.0.1-beta.1", "1.0.0"),
            Ordering::Greater
        );
        assert_eq!(
            super::version_order("1.0.0-beta.1", "1.0.0"),
            Ordering::Less
        );
        assert_eq!(super::version_order("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(
            super::version_order("future-channel", "1.0.0"),
            Ordering::Greater
        );
    }

    #[test]
    fn integration_log_rotates_and_emits_bounded_support_records() {
        let root = test_root("integration-log");
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            user_directory: Some(root.clone()),
            ..super::WorkbenchControllerOptions::default()
        });
        let path = controller.integration_log_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, vec![b'x'; 1024 * 1024 + 1]).unwrap();

        let reference = controller.log_event_timed(
            "test-operation",
            "test-outcome",
            std::time::Instant::now(),
            json!({"phase":"fixture"}),
        );

        assert!(path.with_extension("log.1").is_file());
        let record: Value =
            serde_json::from_str(fs::read_to_string(&path).unwrap().trim()).unwrap();
        assert_eq!(record.get("reference"), Some(&json!(reference)));
        assert_eq!(record.get("operation"), Some(&json!("test-operation")));
        assert_eq!(record.get("outcome"), Some(&json!("test-outcome")));
        assert_eq!(record.pointer("/details/phase"), Some(&json!("fixture")));
        assert!(record.get("sourceText").is_none());
        assert!(record.get("netApiPayload").is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mutation_audit_records_identify_the_action_without_recording_values() {
        let entity_result = super::WorkbenchEntityMutationResult {
            bridge_version: "1.51.0".to_string(),
            protocol_version: 1,
            status: "available".to_string(),
            active_layer_id: Some(3),
            entity: None,
            confirmation_token: None,
            destination: None,
            destination_exists: None,
            resource_name: None,
            persistence_path: None,
            template_saved: None,
            inspection: None,
        };
        let entity_details = super::entity_mutation_audit_details(
            &json!({
                "entityId": "0x10",
                "propertyName": "m_iNumTest",
                "value": "must-not-be-logged",
            }),
            &entity_result,
        );

        assert_eq!(
            super::entity_mutation_operation("RST_WorkbenchSetEntityProperty"),
            "set-entity-property"
        );
        assert_eq!(
            super::entity_mutation_operation("RST_WorkbenchCreatePrefab"),
            "create-prefab"
        );
        assert_eq!(
            super::entity_mutation_operation("RST_WorkbenchSavePrefab"),
            "save-prefab"
        );
        assert_eq!(
            super::entity_mutation_operation("RST_WorkbenchSetPrefabProperty"),
            "set-prefab-property"
        );
        assert_eq!(entity_details.get("entityId"), Some(&json!("0x10")));
        assert_eq!(
            entity_details.get("propertyName"),
            Some(&json!("m_iNumTest"))
        );
        assert!(entity_details.get("value").is_none());

        let component_details = super::component_mutation_audit_details(
            &json!({
                "entityId": "0x10",
                "className": "GRAY_TEST",
                "propertyName": "m_iNumTest",
                "value": 40,
            }),
            &super::WorkbenchComponentResult {
                bridge_version: "1.51.0".to_string(),
                protocol_version: 1,
                status: "available".to_string(),
                entity: None,
                components: Vec::new(),
                properties: Vec::new(),
                confirmation_token: None,
            },
        );
        assert_eq!(
            super::component_mutation_operation("RST_WorkbenchSetComponentProperty"),
            Some("set-component-property")
        );
        assert_eq!(
            super::component_mutation_operation("RST_WorkbenchSetPrefabComponentProperty"),
            Some("set-prefab-component-property")
        );
        assert_eq!(
            component_details.get("componentClass"),
            Some(&json!("GRAY_TEST"))
        );
        assert!(component_details.get("value").is_none());
    }

    #[test]
    fn known_log_tail_is_bounded_and_reports_truncation() {
        let root = test_root("log-tail");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("console.log");
        fs::write(
            &path,
            (1..=600)
                .map(|line| format!("line {line}\n"))
                .collect::<String>(),
        )
        .unwrap();

        let (lines, truncated) = super::bounded_log_tail(&path, 500).unwrap();

        assert!(truncated);
        assert_eq!(lines.len(), 500);
        assert_eq!(lines.first().map(String::as_str), Some("line 101"));
        assert_eq!(lines.last().map(String::as_str), Some("line 600"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restart_project_resolution_requires_one_exact_gproj_title() {
        let root = test_root("restart-project");
        let addons = root.join("addons");
        let matching = addons.join("Test Bullshit").join("addon.gproj");
        fs::create_dir_all(matching.parent().unwrap()).unwrap();
        fs::write(&matching, "ID \"TestBullshit\"\nTITLE \"Test Bullshit\"\n").unwrap();

        assert_eq!(
            super::resolve_project_gproj(&root, "Test Bullshit"),
            Some(matching.clone())
        );
        fs::write(
            addons.join("duplicate.gproj"),
            "ID \"Duplicate\"\nTITLE \"Test Bullshit\"\n",
        )
        .unwrap();
        assert_eq!(super::resolve_project_gproj(&root, "Test Bullshit"), None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn base_game_addon_directory_requires_the_reforger_project_descriptor() {
        let root = test_root("base-game-addons");
        let addons = root.join("addons");
        fs::create_dir_all(addons.join("data")).unwrap();

        assert_eq!(super::base_game_addons_directory(Some(&root)), None);

        fs::write(
            addons.join("data").join("ArmaReforger.gproj"),
            "GameProject {}",
        )
        .unwrap();
        assert_eq!(super::base_game_addons_directory(Some(&root)), Some(addons));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workbench_launch_arguments_always_disable_error_dialogs() {
        let root = test_root("launch-arguments");
        let game = root.join("game");
        let project = root.join("project").join("Example.gproj");
        fs::create_dir_all(game.join("addons").join("data")).unwrap();
        fs::create_dir_all(project.parent().unwrap()).unwrap();
        fs::write(
            game.join("addons").join("data").join("ArmaReforger.gproj"),
            "{}",
        )
        .unwrap();

        assert_eq!(
            super::workbench_launch_arguments(None, None),
            Some(vec![std::ffi::OsString::from("-noThrow")]),
        );
        assert_eq!(
            super::workbench_launch_arguments(Some(&project), Some(&game)),
            Some(vec![
                std::ffi::OsString::from("-noThrow"),
                std::ffi::OsString::from("-gproj"),
                project.into_os_string(),
                std::ffi::OsString::from("-addonsDir"),
                game.join("addons").into_os_string(),
            ]),
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reload_verification_requires_the_complete_ordered_reload_sequence_after_baseline() {
        let root = test_root("reload-log-verification");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("console.log");
        fs::write(
            &path,
            "SCRIPT        : Reloading game scripts\nSCRIPT        : Script validation\nSCRIPT        : Compiling GameLib scripts\nSCRIPT        : Compiling Game scripts\nModule: WorkbenchGame; loaded 171x files\n",
        )
        .unwrap();
        let cursor = super::log_cursor(&path).unwrap();

        fs::write(
            &path,
            format!(
                "{}Game destroyed.\nSCRIPT        : Reloading game scripts\nSCRIPT        : Script validation\nSCRIPT        : Compiling GameLib scripts\nSCRIPT        : Compiling Game scripts\nModule: WorkbenchGame; loaded 171x files\n",
                fs::read_to_string(&path).unwrap()
            ),
        )
        .unwrap();
        let verification = super::reload_verification_since(&path, Some(&cursor))
            .unwrap()
            .expect("complete new reload sequence");

        assert_eq!(verification.path, path);
        assert_eq!(verification.lines.len(), 5);
        assert!(verification.lines[0].contains("Reloading game scripts"));
        assert!(verification.lines[4].contains("Module: WorkbenchGame; loaded"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reload_verification_rejects_incomplete_or_preexisting_reload_lines() {
        let root = test_root("reload-log-incomplete");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("console.log");
        fs::write(
            &path,
            "SCRIPT        : Reloading game scripts\nSCRIPT        : Script validation\nSCRIPT        : Compiling GameLib scripts\nSCRIPT        : Compiling Game scripts\nModule: WorkbenchGame; loaded 171x files\n",
        )
        .unwrap();
        let cursor = super::log_cursor(&path).unwrap();
        fs::write(
            &path,
            format!(
                "{}SCRIPT        : Reloading game scripts\nSCRIPT        : Script validation\n",
                fs::read_to_string(&path).unwrap()
            ),
        )
        .unwrap();

        assert!(super::reload_verification_since(&path, Some(&cursor))
            .unwrap()
            .is_none());
        let rotated = root.join("rotated-console.log");
        fs::rename(&path, &rotated).unwrap();
        fs::write(
            &path,
            "SCRIPT        : Reloading game scripts\nSCRIPT        : Script validation\nSCRIPT        : Compiling GameLib scripts\nSCRIPT        : Compiling Game scripts\nModule: WorkbenchGame; loaded 171x files\n",
        )
        .unwrap();
        assert!(super::reload_verification_since(&path, Some(&cursor))
            .unwrap()
            .is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn automatic_maintenance_repairs_missing_modified_old_and_inconsistent_files() {
        let root = test_root("managed-repair");
        let bridge = root.join("bridge");
        let controller =
            super::WorkbenchController::new(super::WorkbenchControllerOptions::default());
        controller.write_managed_files(&bridge).unwrap();
        fs::write(
            bridge.join("RST_WorkbenchCapabilities.c"),
            "modified managed file",
        )
        .unwrap();
        fs::remove_file(bridge.join("RST_WorkbenchState.c")).unwrap();

        assert!(controller.repair_managed_files(&bridge).unwrap());
        for (name, content) in super::bridge_payload() {
            assert_eq!(fs::read(bridge.join(name)).unwrap(), content.as_bytes());
        }

        let manifest_path = bridge.join("reforger-script-tools.manifest.json");
        let mut tampered: super::BridgeManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        tampered.files[0].sha256 = "incorrect-hash".to_string();
        tampered.files.push(super::BridgeManifestFile {
            name: "RST_ObsoleteCurrent.c".to_string(),
            sha256: super::sha256(b"obsolete current"),
        });
        fs::write(bridge.join("RST_ObsoleteCurrent.c"), "obsolete current").unwrap();
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&tampered).unwrap(),
        )
        .unwrap();

        assert!(controller.repair_managed_files(&bridge).unwrap());
        assert!(!bridge.join("RST_ObsoleteCurrent.c").exists());
        let corrected: super::BridgeManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        assert!(super::manifest_matches_payload(&corrected));

        let mut manifest: super::BridgeManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest.bridge_version = "0.9.0".to_string();
        manifest.protocol_version = 9;
        manifest.files.push(super::BridgeManifestFile {
            name: "RST_Obsolete.c".to_string(),
            sha256: super::sha256(b"obsolete"),
        });
        fs::write(bridge.join("RST_Obsolete.c"), "obsolete").unwrap();
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        assert!(controller.repair_managed_files(&bridge).unwrap());
        assert!(!bridge.join("RST_Obsolete.c").exists());
        let repaired: super::BridgeManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        assert_eq!(repaired.bridge_version, super::WORKBENCH_BRIDGE_VERSION);
        assert_eq!(
            repaired.protocol_version,
            super::WORKBENCH_BRIDGE_PROTOCOL_VERSION
        );

        repaired.files.iter().for_each(|file| {
            assert_eq!(
                file.sha256,
                super::sha256(&fs::read(bridge.join(&file.name)).unwrap())
            )
        });

        let mut inconsistent = repaired;
        inconsistent.protocol_version = 9;
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&inconsistent).unwrap(),
        )
        .unwrap();
        assert!(controller.repair_managed_files(&bridge).unwrap());
        let repaired_protocol: super::BridgeManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        assert_eq!(
            repaired_protocol.protocol_version,
            super::WORKBENCH_BRIDGE_PROTOCOL_VERSION
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn maintenance_does_not_probe_an_unregistered_handler() {
        let root = test_root("activation-retry");
        let bridge = root.join("bridge");
        let controller =
            super::WorkbenchController::new(super::WorkbenchControllerOptions::default());
        controller.write_managed_files(&bridge).unwrap();

        let status = controller.maintain_existing_bridge(&bridge);

        assert_eq!(status.active_version, None);
        assert_eq!(status.protocol_version, None);
        assert!(!status.compatible);
        assert!(status.activation_required);
        assert!(status.capabilities.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn incompatible_handler_protocol_exposes_no_dependent_capabilities() {
        let root = test_root("incompatible-handler");
        let bridge = root.join("bridge");
        let (port, peer) = start_peer(|request| {
            assert_eq!(request, json!({"APIFunc":"RST_WorkbenchCapabilities"}));
            json!({
                "bridgeVersion":"1.0.0",
                "protocolVersion":9,
                "capabilities":"state;reload"
            })
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                validation_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });
        controller.write_managed_files(&bridge).unwrap();

        let status = controller.active_bridge_status(&bridge, false);

        assert!(!status.compatible);
        assert!(status.activation_required);
        assert!(status.capabilities.is_empty());
        assert!(!status.capabilities_truncated);
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn obsolete_manifest_entries_cannot_escape_the_managed_directory() {
        let root = std::env::temp_dir().join(format!(
            "rst-workbench-containment-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bridge = root.join("bridge");
        fs::create_dir_all(&bridge).unwrap();
        fs::write(root.join("outside.c"), "keep").unwrap();
        fs::write(bridge.join("obsolete.c"), "remove").unwrap();
        fs::write(
            bridge.join("reforger-script-tools.manifest.json"),
            serde_json::to_vec_pretty(&super::BridgeManifest {
                bridge_version: "0.9.0".to_string(),
                protocol_version: 1,
                files: vec![
                    super::BridgeManifestFile {
                        name: "obsolete.c".to_string(),
                        sha256: super::sha256(b"remove"),
                    },
                    super::BridgeManifestFile {
                        name: "../outside.c".to_string(),
                        sha256: super::sha256(b"keep"),
                    },
                ],
            })
            .unwrap(),
        )
        .unwrap();
        let controller =
            super::WorkbenchController::new(super::WorkbenchControllerOptions::default());

        controller.write_managed_files(&bridge).unwrap();

        assert!(!bridge.join("obsolete.c").exists());
        assert_eq!(fs::read_to_string(root.join("outside.c")).unwrap(), "keep");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_workbench_paths_override_discovery_and_keep_the_profile_user_relative() {
        let root = test_root("explicit-paths");
        let game = root.join("game");
        let tools = root.join("tools");
        let executable = tools
            .join("Workbench")
            .join("ArmaReforgerWorkbenchSteamDiag.exe");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"fixture").unwrap();
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            user_directory: Some(root.clone()),
            game_directory: Some(game.clone()),
            tools_directory: Some(tools.clone()),
            executable: Some(executable.clone()),
            ..super::WorkbenchControllerOptions::default()
        });

        let paths = controller.paths();

        assert_eq!(paths.game, Some(game));
        assert_eq!(paths.game_source, "explicit");
        assert_eq!(paths.tools, Some(tools));
        assert_eq!(paths.tools_source, "explicit");
        assert_eq!(paths.executable, Some(executable));
        assert_eq!(paths.executable_source, "explicit");
        assert_eq!(
            paths.profile,
            root.join("Documents")
                .join("My Games")
                .join("ArmaReforgerWorkbench")
                .join("profile")
        );
        assert_eq!(
            paths.bridge_directory,
            paths
                .profile
                .join("scripts")
                .join("WorkbenchGame")
                .join("reforger-script-tools")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn steam_discovery_resolves_game_and_tools_from_independent_libraries() {
        let root = test_root("steam-independent-libraries");
        let steam = root.join("Steam");
        let game_library = root.join("GameLibrary");
        let tools_library = root.join("ToolsLibrary");
        write_library_folders(&steam, &[&game_library, &tools_library]);
        write_steam_app(&game_library, "1874880", "Arma Reforger", "Arma Reforger");
        write_steam_app(
            &tools_library,
            "1874910",
            "Arma Reforger Tools",
            "Arma Reforger Tools",
        );

        assert_eq!(
            super::discover_steam_app_from_root(&steam, "1874880", "Arma Reforger"),
            Some(
                game_library
                    .join("steamapps")
                    .join("common")
                    .join("Arma Reforger")
            )
        );
        assert_eq!(
            super::discover_steam_app_from_root(&steam, "1874910", "Arma Reforger Tools"),
            Some(
                tools_library
                    .join("steamapps")
                    .join("common")
                    .join("Arma Reforger Tools")
            )
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn steam_discovery_uses_canonical_only_as_fallback_and_rejects_metadata_ambiguity() {
        let root = test_root("steam-ambiguity");
        let steam = root.join("Steam");
        let canonical = steam
            .join("steamapps")
            .join("common")
            .join("Arma Reforger Tools");
        fs::create_dir_all(&canonical).unwrap();

        assert_eq!(
            super::discover_steam_app_from_root(&steam, "1874910", "Arma Reforger Tools"),
            Some(canonical)
        );

        let other = root.join("OtherLibrary");
        write_library_folders(&steam, &[&other]);
        write_steam_app(
            &other,
            "1874910",
            "Arma Reforger Tools",
            "Arma Reforger Tools",
        );
        assert_eq!(
            super::discover_steam_app_from_root(&steam, "1874910", "Arma Reforger Tools"),
            Some(
                other
                    .join("steamapps")
                    .join("common")
                    .join("Arma Reforger Tools")
            )
        );

        let second = root.join("SecondLibrary");
        write_library_folders(&steam, &[&other, &second]);
        write_steam_app(
            &second,
            "1874910",
            "Arma Reforger Tools",
            "Arma Reforger Tools",
        );
        assert_eq!(
            super::discover_steam_app_from_root(&steam, "1874910", "Arma Reforger Tools"),
            None
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workbench_executable_shape_is_exact() {
        let root = test_root("executable-shape");
        let directory = root.join("Tools").join("Workbench");
        fs::create_dir_all(&directory).unwrap();
        let expected = directory.join("ArmaReforgerWorkbenchSteamDiag.exe");
        let wrong_name = directory.join("ArmaReforgerWorkbench.exe");
        fs::write(&expected, b"fixture").unwrap();
        fs::write(&wrong_name, b"fixture").unwrap();

        assert!(super::is_workbench_executable(&expected));
        assert!(!super::is_workbench_executable(&wrong_name));
        assert!(!super::is_workbench_executable(
            &directory
                .join("missing")
                .join("ArmaReforgerWorkbenchSteamDiag.exe")
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn process_identity_decoder_preserves_single_and_multiple_exact_identities() {
        assert_eq!(
            super::parse_process_identities(br#"{"id":7,"startTicks":11}"#),
            vec![super::ProcessIdentity {
                id: 7,
                start_ticks: 11,
            }]
        );
        assert_eq!(
            super::parse_process_identities(
                br#"[{"id":7,"startTicks":11},{"id":8,"startTicks":12}]"#
            ),
            vec![
                super::ProcessIdentity {
                    id: 7,
                    start_ticks: 11,
                },
                super::ProcessIdentity {
                    id: 8,
                    start_ticks: 12,
                },
            ]
        );
        assert!(super::parse_process_identities(b"not-json").is_empty());
    }

    #[test]
    fn force_close_script_requires_the_exact_running_workbench_identity() {
        let script = super::force_stop_workbench_script(super::ProcessIdentity {
            id: 7,
            start_ticks: 11,
        });

        assert!(script.contains("Get-Process -Id 7"));
        assert!(script.contains("ArmaReforgerWorkbenchSteamDiag"));
        assert!(script.contains("Ticks -ne [uint64]11"));
        assert!(script.contains("Stop-Process -Id $p.Id -Force"));
    }

    #[test]
    fn validation_cursor_pages_one_immutable_compiler_snapshot() {
        let controller =
            super::WorkbenchController::new(super::WorkbenchControllerOptions::default());
        let diagnostics = (1..=3)
            .map(|line| super::WorkbenchCompilerDiagnostic {
                severity: super::WorkbenchDiagnosticSeverity::Error,
                message: format!("finding {line}"),
                location: super::WorkbenchDiagnosticLocation {
                    file: "scripts/Test.c".to_string(),
                    file_abs: None,
                    addon: Some("Test".to_string()),
                    line,
                },
            })
            .collect();
        *controller.validation_snapshot.lock().unwrap() = Some((
            "snapshot".to_string(),
            super::WorkbenchValidation {
                profile: "WORKBENCH".to_string(),
                success: false,
                diagnostics,
            },
        ));

        let page = controller
            .validate_scripts_page(Some("wv1:snapshot:1"), 1)
            .unwrap();

        assert_eq!(page.total_diagnostics, 3);
        assert_eq!(page.diagnostics[0].message, "finding 2");
        assert_eq!(page.next_cursor.as_deref(), Some("wv1:snapshot:2"));
    }

    fn test_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "rst-workbench-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write_library_folders(steam: &std::path::Path, libraries: &[&std::path::Path]) {
        let steamapps = steam.join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        let entries = libraries
            .iter()
            .enumerate()
            .map(|(index, library)| {
                format!(
                    "\"{}\"\n{{\n\"path\"\t\"{}\"\n}}\n",
                    index + 1,
                    library.display()
                )
            })
            .collect::<String>();
        fs::write(
            steamapps.join("libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n{entries}}}\n"),
        )
        .unwrap();
    }

    fn write_steam_app(library: &std::path::Path, app_id: &str, install_dir: &str, folder: &str) {
        let steamapps = library.join("steamapps");
        fs::create_dir_all(steamapps.join("common").join(folder)).unwrap();
        fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            format!("\"AppState\"\n{{\n\"installdir\"\t\"{install_dir}\"\n}}\n"),
        )
        .unwrap();
    }

    #[test]
    fn typed_property_values_are_normalized_and_reject_mismatched_wire_values() {
        assert_eq!(super::normalized_property_value("float", "2.5"), json!(2.5));
        assert_eq!(
            super::normalized_property_value("vector", "1 2 3"),
            json!({"x": 1.0, "y": 2.0, "z": 3.0})
        );
        assert_eq!(
            super::property_value_wire_format("bool", &json!(true)).as_deref(),
            Some("1")
        );
        assert_eq!(
            super::property_value_wire_format("float", &json!("not-a-number")),
            None
        );
        assert_eq!(
            super::property_value_wire_format("vector", &json!({"x": 1.0, "y": 2.0})),
            None
        );
    }

    #[test]
    fn shape_point_wire_records_require_three_finite_coordinates() {
        assert_eq!(
            super::parse_shape_points("1|2|3;-4.5|0|7"),
            Some(vec![
                super::WorkbenchEntityPosition {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                super::WorkbenchEntityPosition {
                    x: -4.5,
                    y: 0.0,
                    z: 7.0,
                },
            ])
        );
        assert_eq!(super::parse_shape_points("1|2"), None);
        assert_eq!(super::parse_shape_points("1|2|NaN"), None);
    }

    #[test]
    fn shape_point_edit_uses_the_typed_native_handler() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc": "RST_WorkbenchEditShapePoints",
                    "entityId": "0x01 {}",
                    "operation": "insert",
                    "index": 1,
                    "count": 1,
                    "points": "1,2,3;4,5,6",
                })
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"points-updated","entity":"0x01 {}|PolylineShapeEntity|0|1|10|20|30||||","shapeClass":"PolylineShapeEntity","closed":false,"points":"1|2|3;4|5|6"})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });
        let result = controller
            .edit_shape_points(
                "0x01 {}",
                super::WorkbenchShapePointEdit::Insert,
                Some(1),
                None,
                &[
                    super::WorkbenchEntityPosition {
                        x: 1.0,
                        y: 2.0,
                        z: 3.0,
                    },
                    super::WorkbenchEntityPosition {
                        x: 4.0,
                        y: 5.0,
                        z: 6.0,
                    },
                ],
            )
            .unwrap();
        assert_eq!(result.status, "points-updated");
        assert_eq!(result.points.len(), 2);
        assert_eq!(result.shape_class.as_deref(), Some("PolylineShapeEntity"));
        peer.join().unwrap();
    }

    #[test]
    fn shape_geometry_conversion_uses_explicit_spaces_and_typed_handler() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc":"RST_WorkbenchShapeGeometry", "entityId":"0x01 {}", "operation":"convert",
                    "fromSpace":"local", "toSpace":"world", "points":"1,2,3"
                })
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"converted","entity":"0x01 {}|PolylineShapeEntity|0|1|10|20|30||||","shapeClass":"PolylineShapeEntity","fromSpace":"local","toSpace":"world","points":"11|22|33"})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });
        let result = controller
            .convert_shape_points(
                "0x01 {}",
                super::WorkbenchShapePointSpace::Local,
                super::WorkbenchShapePointSpace::World,
                &[super::WorkbenchEntityPosition {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                }],
            )
            .unwrap();
        assert_eq!(result.status, "converted");
        assert_eq!(result.points[0].x, 11.0);
        assert_eq!(result.from_space, "local");
        peer.join().unwrap();
    }

    #[test]
    fn shape_geometry_bridge_uses_parent_aware_conversion_and_one_action_mutations() {
        assert!(super::BRIDGE_SHAPE_GEOMETRY_SOURCE.contains("shape.CoordToParent(values[i])"));
        assert!(super::BRIDGE_SHAPE_GEOMETRY_SOURCE.contains("shape.CoordToLocal(values[i])"));
        assert!(super::BRIDGE_SHAPE_GEOMETRY_SOURCE
            .contains("Reforger Script Tools: transform shape points"));
        assert!(super::BRIDGE_SHAPE_GEOMETRY_SOURCE
            .contains("Reforger Script Tools: resample polyline"));
        assert!(super::BRIDGE_SHAPE_GEOMETRY_SOURCE
            .contains("source.GetClassName() != \"PolylineShapeEntity\""));
        assert!(super::BRIDGE_SHAPE_GEOMETRY_SOURCE
            .contains("source.GetClassName() != \"SplineShapeEntity\""));
    }

    #[test]
    fn reparent_handler_rejects_a_descendant_parent_before_starting_an_action() {
        assert!(super::BRIDGE_ENTITY_MUTATION_SOURCE
            .contains("bool IsAncestor(IEntitySource entity, IEntitySource candidateParent)",));
        let descendant_guard = super::BRIDGE_ENTITY_MUTATION_SOURCE
            .find("IsAncestor(entity, parent)")
            .unwrap();
        let reparent_guard = &super::BRIDGE_ENTITY_MUTATION_SOURCE[descendant_guard..];
        assert!(reparent_guard.contains("api.IsEntityLayerLockedHierarchy"));
    }

    #[test]
    fn component_addition_accepts_only_a_loaded_script_component_class() {
        assert!(
            super::BRIDGE_COMPONENTS_SOURCE.contains("r.className.ToType()")
                && super::BRIDGE_COMPONENTS_SOURCE.contains("IsInherited(ScriptComponent)")
        );
    }

    #[test]
    fn entity_property_mutation_records_the_post_action_entity_state() {
        assert!(super::BRIDGE_PROPERTIES_SOURCE
            .contains("class RST_WorkbenchSetEntityProperty : RST_WorkbenchEntityMutationBase"));
        let record = super::BRIDGE_PROPERTIES_SOURCE
            .find("Record(api, response, entity);")
            .unwrap();
        let property_set = super::BRIDGE_PROPERTIES_SOURCE
            .find("response.status = \"property-set\"")
            .unwrap();
        assert!(record < property_set);
    }

    #[test]
    fn entity_property_write_uses_only_the_inspection_descriptor_binding() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({
                    "APIFunc": "RST_WorkbenchSetEntityProperty",
                    "entityId": "0x01 {}",
                    "propertyName": "m_fRadius",
                    "expectedValue": "2.5",
                    "value": "3.75",
                })
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"property-set","activeLayerId":7,"entity":""})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });
        controller
            .property_write_descriptors
            .lock()
            .unwrap()
            .insert(
                "prop2:test".to_string(),
                super::PropertyWriteDescriptor {
                    entity_id: "0x01 {}".to_string(),
                    component_id: None,
                    property_name: "m_fRadius".to_string(),
                    data_type: "float".to_string(),
                    observed_value: "2.5".to_string(),
                    issued: Instant::now(),
                },
            );
        let result = controller
            .set_entity_property("0x01 {}", "prop2:test", json!(3.75))
            .unwrap();
        assert_eq!(result.status, "property-set");
        peer.join().unwrap();
    }

    #[test]
    fn component_inspection_issues_a_component_bound_property_descriptor() {
        let (port, peer) = start_peer(|request| {
            assert_eq!(
                request,
                json!({"APIFunc":"RST_WorkbenchInspectComponent","entityId":"0x01 {}","componentId":"cmp1:0:TestComponent"})
            );
            json!({"bridgeVersion":"1.51.0","protocolVersion":1,"status":"available","entity":"","components":"0|TestComponent","properties":"m_fRadius|float|2.5|1"})
        });
        let controller = super::WorkbenchController::new(super::WorkbenchControllerOptions {
            gateway: super::WorkbenchGatewayOptions {
                port,
                status_deadline: Duration::from_secs(1),
                ..super::WorkbenchGatewayOptions::default()
            },
            ..super::WorkbenchControllerOptions::default()
        });
        let result = controller
            .inspect_component("0x01 {}", "cmp1:0:TestComponent")
            .unwrap();
        assert_eq!(result.components.len(), 1);
        assert_eq!(result.properties.len(), 1);
        let property = serde_json::to_value(&result.properties[0]).unwrap();
        assert!(property.get("directlySet").is_none());
        assert!(property.get("writable").is_none());
        assert!(result.properties[0]
            .write_descriptor
            .as_deref()
            .is_some_and(|value| value.starts_with("prop2:")));
        peer.join().unwrap();
    }

    #[test]
    fn component_operations_reject_an_unissued_component_descriptor_before_gateway_dispatch() {
        let controller =
            super::WorkbenchController::new(super::WorkbenchControllerOptions::default());

        let result = controller
            .inspect_component("0x01 {}", "forged-component")
            .unwrap();

        assert_eq!(result.status, "invalid-component-descriptor");
    }

    fn test_gateway(port: u16) -> WorkbenchGateway {
        WorkbenchGateway::new(WorkbenchGatewayOptions {
            port,
            status_deadline: Duration::from_secs(1),
            validation_deadline: Duration::from_secs(1),
            ..WorkbenchGatewayOptions::default()
        })
    }

    fn start_peer(
        response: impl FnOnce(Value) -> Value + Send + 'static,
    ) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut version = [0_u8; 4];
            stream.read_exact(&mut version).unwrap();
            assert_eq!(i32::from_le_bytes(version), 1);
            assert_eq!(read_string(&mut stream), "ReforgerScriptTools");
            assert_eq!(read_string(&mut stream), "JsonRPC");
            let payload: Value = serde_json::from_str(&read_string(&mut stream)).unwrap();
            let payload = response(payload);
            write_string(&mut stream, "Ok");
            write_string(&mut stream, &payload.to_string());
        });
        (port, peer)
    }

    fn read_string(stream: &mut impl Read) -> String {
        let mut length = [0_u8; 4];
        stream.read_exact(&mut length).unwrap();
        let mut bytes = vec![0; i32::from_le_bytes(length) as usize];
        stream.read_exact(&mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    fn write_string(stream: &mut impl Write, value: &str) {
        stream
            .write_all(&(value.len() as i32).to_le_bytes())
            .unwrap();
        stream.write_all(value.as_bytes()).unwrap();
    }
}
