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

pub const WORKBENCH_BRIDGE_VERSION: &str = "1.6.0";
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
    pub reload_verified: bool,
    pub log_path: PathBuf,
    pub verification_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchOpenWorldResult {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchResourceListPage {
    pub bridge_version: String,
    pub protocol_version: u32,
    pub project_revision: String,
    pub resources: Vec<String>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchSelectedEntity {
    pub entity_id: String,
    pub class_name: String,
    pub sub_scene: i32,
    pub layer_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchLogRead {
    pub source: String,
    pub path: Option<PathBuf>,
    pub lines: Vec<String>,
    pub truncated: bool,
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

    pub fn open_world(
        &self,
        world_path: &str,
    ) -> Result<WorkbenchOpenWorldResult, WorkbenchFailure> {
        let started = Instant::now();
        if world_path.trim().is_empty() {
            return Err(self.correlate_failure_details(
                "open_world",
                "world-path-required",
                failure(WorkbenchFailureCode::Protocol),
                json!({}),
            ));
        }
        let value = self
            .gateway
            .request(
                json!({"APIFunc": "RST_WorkbenchOpenWorld", "worldPath": world_path}),
                self.options.gateway.status_deadline,
            )
            .map_err(|failure| {
                self.correlate_failure_details(
                    "open_world",
                    failure_code(failure.code),
                    failure,
                    json!({"handler": "RST_WorkbenchOpenWorld"}),
                )
            })?;
        let result: WorkbenchOpenWorldResult = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "open_world",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchOpenWorld"}),
            )
        })?;
        self.log_event_timed(
            "open-world",
            &result.status,
            started,
            json!({"opened": result.opened}),
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
        let result: WorkbenchResourceInspection = serde_json::from_value(value).map_err(|_| {
            self.correlate_failure_details(
                "inspect_resource",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchInspectResource"}),
            )
        })?;
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
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<WorkbenchResourceListPage, WorkbenchFailure> {
        let limit = limit.clamp(1, 200);
        let query = query.unwrap_or("").trim();
        let kinds = kinds.join(";");
        let signature = sha256(format!("{kinds}\n{query}").as_bytes());
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
            json!({"APIFunc": "RST_WorkbenchListResources", "extensions": kinds, "query": query, "offset": offset, "limit": limit}),
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
        if raw.protocol_version != WORKBENCH_BRIDGE_PROTOCOL_VERSION || raw.resources.len() > limit
        {
            return Err(self.correlate_failure_details(
                "list_resources",
                "workbench_protocol_error",
                failure(WorkbenchFailureCode::Protocol),
                json!({"handler": "RST_WorkbenchListResources"}),
            ));
        }
        let project_revision = sha256(raw.loaded_addons.as_bytes());
        let next_cursor = raw
            .has_more
            .then(|| format!("wrl1:{signature}:{}", offset + raw.resources.len()));
        let result = WorkbenchResourceListPage {
            bridge_version: raw.bridge_version,
            protocol_version: raw.protocol_version,
            project_revision,
            resources: raw.resources,
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
            editor_available: raw.editor_available,
            status: raw.status,
            selected_count: raw.selected_count,
            selected_entities_truncated: raw.selected_entities_truncated
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

    /// Perform the one intentional keyboard automation supported by the Workbench bridge.
    ///
    /// This is deliberately not a general input facility: it requires exactly one current
    /// Workbench process, verifies that its main window owns foreground focus, and sends only
    /// Ctrl+Shift+R. The operation succeeds only after Workbench writes its full reload marker.
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
        focus_workbench_and_send_reload(*process).map_err(|outcome| {
            self.correlate_failure_details(
                "activate-scripts",
                outcome,
                failure(WorkbenchFailureCode::Unavailable),
                json!({"processId": process.id}),
            )
        })?;

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
                truncated: false,
            });
        };
        if !path.is_file() {
            return Ok(WorkbenchLogRead {
                source: source.to_string(),
                path: Some(path),
                lines: Vec::new(),
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
        let mut command = std::process::Command::new(executable);
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Some(project) = project {
            command.arg("-gproj").arg(project);
            let game_addons =
                base_game_addons_directory(paths.game.as_deref()).ok_or_else(|| {
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
            command.arg("-addonsDir").arg(game_addons);
        }
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
        let observed = self.observed_processes.lock().ok().and_then(|processes| {
            processes
                .iter()
                .find(|process| process.id == process_id)
                .copied()
        });
        let Some(observed) = observed.filter(|process| workbench_processes().contains(process))
        else {
            return Err(self.correlate_failure_details(
                "restart",
                "stale-or-unobserved-process",
                failure(WorkbenchFailureCode::Unavailable),
                json!({"processId": process_id}),
            ));
        };
        let paths = self.paths();
        let project = workbench_project_title(observed)
            .and_then(|title| resolve_project_gproj(&paths.workbench_root, &title))
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
        let stopped = self.stop(process_id)?;
        if !stopped.exited {
            return Ok(stopped);
        }
        self.launch_project(Some(&project))
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
        for (name, content) in bridge_payload() {
            fs::write(bridge_directory.join(name), content)?;
        }
        let files = bridge_payload()
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
                .map(|value| value.as_secs())
                .unwrap_or_default();
            let record = json!({
                "reference": reference,
                "timestamp": timestamp,
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
    editor_available: bool,
    status: String,
    #[serde(rename = "selectedCount")]
    selected_count: u32,
    #[serde(rename = "selectedEntities", default)]
    selected_entities: String,
    #[serde(rename = "selectedEntitiesTruncated", default)]
    selected_entities_truncated: bool,
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
    resources: Vec<String>,
    #[serde(rename = "hasMore", default)]
    has_more: bool,
}

fn workbench_bool(value: &Value) -> bool {
    matches!(value, Value::Bool(true)) || value.as_i64().is_some_and(|integer| integer != 0)
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
            if fields.next().is_some()
                || entity_id.len() > 128
                || class_name.len() > 256
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
            })
        })
        .collect()
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
    manifest.files.len() == bridge_payload().len()
        && bridge_payload().iter().all(|(name, content)| {
            let expected_hash = sha256(content.as_bytes());
            manifest
                .files
                .iter()
                .any(|file| file.name == *name && file.sha256 == expected_hash)
        })
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
        "SCRIPT        : Reloading game scripts",
        "Compiling GameLib scripts",
        "Compiling Game scripts",
        "Module: WorkbenchGame; loaded",
        "PROFILING: Reloading game scripts took",
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

fn focus_workbench_and_send_reload(process: ProcessIdentity) -> Result<(), &'static str> {
    let script = format!(
        r#"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class RSTWorkbenchWindow {{
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int maxCount);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool attach);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
}}
'@
$p = Get-Process -Id {process_id} -ErrorAction Stop
if ($p.ProcessName -ne 'ArmaReforgerWorkbenchSteamDiag' -or [uint64]$p.StartTime.ToUniversalTime().Ticks -ne [uint64]{start_ticks}) {{ exit 2 }}
$projectWindows = [System.Collections.Generic.List[System.IntPtr]]::new()
$callback = [RSTWorkbenchWindow+EnumWindowsProc] {{ param([IntPtr]$hWnd, [IntPtr]$unused)
	[uint32]$ownerProcess = 0
	[void][RSTWorkbenchWindow]::GetWindowThreadProcessId($hWnd, [ref]$ownerProcess)
	if ($ownerProcess -eq $p.Id -and [RSTWorkbenchWindow]::IsWindowVisible($hWnd)) {{
		$title = [System.Text.StringBuilder]::new(512)
		[void][RSTWorkbenchWindow]::GetWindowText($hWnd, $title, $title.Capacity)
		if ($title.ToString().StartsWith('Enfusion Workbench - ', [System.StringComparison]::Ordinal)) {{ $projectWindows.Add($hWnd) }}
	}}
	return $true
}}
[void][RSTWorkbenchWindow]::EnumWindows($callback, [IntPtr]::Zero)
if ($projectWindows.Count -ne 1) {{ exit 7 }}
$window = $projectWindows[0]
$foreground = [RSTWorkbenchWindow]::GetForegroundWindow()
[uint32]$ownerProcess = 0
$foregroundThread = [RSTWorkbenchWindow]::GetWindowThreadProcessId($foreground, [ref]$ownerProcess)
$targetThread = [RSTWorkbenchWindow]::GetWindowThreadProcessId($window, [ref]$ownerProcess)
$currentThread = [RSTWorkbenchWindow]::GetCurrentThreadId()
$attachedForeground = $false
$attachedTarget = $false
try {{
	if ($foregroundThread -ne $currentThread) {{ $attachedForeground = [RSTWorkbenchWindow]::AttachThreadInput($currentThread, $foregroundThread, $true) }}
	if ($targetThread -ne $currentThread) {{ $attachedTarget = [RSTWorkbenchWindow]::AttachThreadInput($currentThread, $targetThread, $true) }}
	[void][RSTWorkbenchWindow]::ShowWindowAsync($window, 9)
	[void][RSTWorkbenchWindow]::BringWindowToTop($window)
	[void][RSTWorkbenchWindow]::SetForegroundWindow($window)
	Start-Sleep -Milliseconds 300
	if ([RSTWorkbenchWindow]::GetForegroundWindow() -ne $window) {{ exit 3 }}
	$shell = New-Object -ComObject WScript.Shell
	$shell.SendKeys('^+r')
}} finally {{
	if ($attachedTarget) {{ [void][RSTWorkbenchWindow]::AttachThreadInput($currentThread, $targetThread, $false) }}
	if ($attachedForeground) {{ [void][RSTWorkbenchWindow]::AttachThreadInput($currentThread, $foregroundThread, $false) }}
}}
"#,
        process_id = process.id,
        start_ticks = process.start_ticks,
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
        .map_err(|_| "focus-reload-request-failed")?;
    match status.code() {
        Some(2) => Err("workbench-window-unavailable"),
        Some(3) => Err("workbench-focus-not-confirmed"),
        Some(7) => Err("workbench-project-window-ambiguous"),
        Some(0) => Ok(()),
        _ => Err("focus-reload-request-failed"),
    }
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
        ("RST_WorkbenchOpenWorld.c", BRIDGE_OPEN_WORLD_SOURCE),
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
        ("RST_WorkbenchListResources.c", BRIDGE_LIST_RESOURCES_SOURCE),
    ]
}

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
		response.bridgeVersion = "1.6.0";
	response.protocolVersion = 1;
	response.capabilities = "state;open-world;play-session;project-context;inspect-resource;world-selection;list-resources";
		return response;
	}
}
#endif
"#;

const BRIDGE_STATE_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchStateRequest : JsonApiStruct
{
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
		RST_WorkbenchStateResponse response = new RST_WorkbenchStateResponse();
	response.bridgeVersion = "1.6.0";
		response.protocolVersion = 1;
		response.mode = "workbench";
		response.playSession = "unavailable";
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
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

const BRIDGE_OPEN_WORLD_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchOpenWorldRequest : JsonApiStruct
{
	string worldPath;

	void RST_WorkbenchOpenWorldRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenWorldResponse : JsonApiStruct
{
	bool opened;
	string status;

	void RST_WorkbenchOpenWorldResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenWorld : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchOpenWorldRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchOpenWorldRequest typedRequest = RST_WorkbenchOpenWorldRequest.Cast(request);
		RST_WorkbenchOpenWorldResponse response = new RST_WorkbenchOpenWorldResponse();
		if (typedRequest.worldPath == string.Empty)
		{
			response.status = "world-path-required";
			return response;
		}

		Workbench.OpenModule(WorldEditor);
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}

		response.opened = worldEditor.SetOpenedResource(typedRequest.worldPath);
		if (response.opened)
			response.status = "opened";
		else
			response.status = "open-failed";
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
		response.bridgeVersion = "1.6.0";
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

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchWorldSelectionResponse response = new RST_WorkbenchWorldSelectionResponse();
		response.bridgeVersion = "1.6.0";
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
			if (response.selectedEntities != string.Empty)
				response.selectedEntities += ";";
			response.selectedEntities += string.Format("%1|%2|%3|%4", entity.GetID().ToString(), entity.GetClassName(), entity.GetSubScene(), entity.GetLayerID());
		}
		return response;
	}
}
#endif
"#;

const BRIDGE_LIST_RESOURCES_SOURCE: &str = r#"#ifdef WORKBENCH
class RST_WorkbenchListResourcesRequest : JsonApiStruct
{
	string extensions;
	string query;
	int offset;
	int limit;
	void RST_WorkbenchListResourcesRequest() { RegAll(); }
}
class RST_WorkbenchListResourcesResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string loadedAddons;
	ref array<ResourceName> resources = new array<ResourceName>();
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
		response.bridgeVersion = "1.6.0";
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
		array<ResourceName> allResources = new array<ResourceName>();
		ResourceDatabase.SearchResources(filter, allResources.Insert);
		allResources.Sort();
		int start = typedRequest.offset;
		if (start < 0)
			start = 0;
		int limit = typedRequest.limit;
		if (limit < 1)
			limit = 1;
		if (limit > 200)
			limit = 200;
		for (int index = start; index < allResources.Count() && response.resources.Count() < limit; index++)
			response.resources.Insert(allResources[index]);
		response.hasMore = start + response.resources.Count() < allResources.Count();
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
    use std::time::Duration;

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
            "0x0000000000000001|TestEntity|0|12;0x0000000000000002|LightEntity|2|4",
        )
        .unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].entity_id, "0x0000000000000001");
        assert_eq!(records[0].class_name, "TestEntity");
        assert_eq!(records[0].sub_scene, 0);
        assert_eq!(records[0].layer_id, 12);
        assert!(super::parse_world_selection_records("missing-fields").is_err());
        assert!(super::parse_world_selection_records("id|class|nope|0").is_err());
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
            assert_eq!(request["offset"], 0);
            assert_eq!(request["limit"], 2);
            json!({
                "bridgeVersion": "1.6.0",
                "protocolVersion": 1,
                "loadedAddons": "ArmaReforger;TestBullshit",
                "resources": ["{DD49A6CE18710A05}worlds/test/empty_test.ent"],
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
            .list_resources(&["ent"], Some("test"), None, 2)
            .unwrap();

        assert_eq!(page.resources.len(), 1);
        assert!(page.next_cursor.is_some());
        assert!(super::parse_resource_list_cursor(page.next_cursor.as_deref().unwrap()).is_some());
        assert_ne!(page.project_revision, "ArmaReforger;TestBullshit");
        peer.join().unwrap();
        fs::remove_dir_all(root).unwrap();
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
    fn reload_verification_requires_the_complete_ordered_reload_sequence_after_baseline() {
        let root = test_root("reload-log-verification");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("console.log");
        fs::write(
            &path,
            "SCRIPT        : Reloading game scripts\nCompiling GameLib scripts\nCompiling Game scripts\nModule: WorkbenchGame; loaded 171x files\nPROFILING: Reloading game scripts took: 12 ms\n",
        )
        .unwrap();
        let cursor = super::log_cursor(&path).unwrap();

        fs::write(
            &path,
            format!(
                "{}Game destroyed.\nSCRIPT        : Reloading game scripts\nCompiling GameLib scripts\nCompiling Game scripts\nModule: WorkbenchGame; loaded 171x files\nPROFILING: Reloading game scripts took: 12000 ms\n",
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
        assert!(verification.lines[4].contains("took: 12000 ms"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reload_verification_rejects_incomplete_or_preexisting_reload_lines() {
        let root = test_root("reload-log-incomplete");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("console.log");
        fs::write(
            &path,
            "SCRIPT        : Reloading game scripts\nCompiling GameLib scripts\nCompiling Game scripts\nModule: WorkbenchGame; loaded 171x files\nPROFILING: Reloading game scripts took: 12 ms\n",
        )
        .unwrap();
        let cursor = super::log_cursor(&path).unwrap();
        fs::write(
            &path,
            format!(
                "{}SCRIPT        : Reloading game scripts\nCompiling GameLib scripts\n",
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
            "SCRIPT        : Reloading game scripts\nCompiling GameLib scripts\nCompiling Game scripts\nModule: WorkbenchGame; loaded 171x files\nPROFILING: Reloading game scripts took: 12 ms\n",
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
