//! Workspace-index request execution.
//!
//! The external index remains owned by the composition root, while this
//! boundary owns the workspace request shape and translates index outcomes
//! into transport-neutral runtime effects.
use super::{ExternalIndexHandle, RuntimeEffect};
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkspaceFileChangedParams {
    pub(super) path: String,
    pub(super) text: String,
    pub(super) sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkspaceFileDeletedParams {
    pub(super) path: String,
    pub(super) sequence: u64,
}

pub(super) fn update_workspace_file(
    external_index: &mut ExternalIndexHandle,
    params: Option<WorkspaceFileChangedParams>,
    operational_logging: bool,
) -> Vec<RuntimeEffect> {
    let Some(params) = params else {
        return Vec::new();
    };
    let start = Instant::now();
    let path = PathBuf::from(params.path);
    let bytes = params.text.len();
    let result = external_index.update_workspace_file(path.clone(), params.text, params.sequence);
    if !operational_logging {
        return Vec::new();
    }
    let message = match result {
        Ok(Some((symbols, parse_diagnostics))) => {
            let status = external_index.status_summary();
            format!(
                "notification workspaceFileChanged path={} sequence={} bytes={} symbols={} parse_diagnostics={} overlay_status={} overlay_generation={} overlay_files={} overlay_symbols={} elapsed_ms={}",
                path.display(), params.sequence, bytes, symbols, parse_diagnostics, status.status,
                status.generation, status.files, status.symbols, start.elapsed().as_millis()
            )
        }
        Ok(None) => {
            format!(
            "notification workspaceFileChanged ignored path={} sequence={} bytes={} elapsed_ms={}",
            path.display(), params.sequence, bytes, start.elapsed().as_millis()
        )
        }
        Err(error) => {
            format!(
            "notification workspaceFileChanged path={} sequence={} bytes={} error={} elapsed_ms={}",
            path.display(), params.sequence, bytes, error, start.elapsed().as_millis()
        )
        }
    };
    vec![RuntimeEffect::Log(message)]
}

pub(super) fn delete_workspace_file(
    external_index: &mut ExternalIndexHandle,
    params: Option<WorkspaceFileDeletedParams>,
    operational_logging: bool,
) -> Vec<RuntimeEffect> {
    let Some(params) = params else {
        return Vec::new();
    };
    let start = Instant::now();
    let path = PathBuf::from(params.path);
    let removed = external_index.delete_workspace_file(&path, params.sequence);
    if !operational_logging {
        return Vec::new();
    }
    let status = external_index.status_summary();
    let message = match removed {
        Some(removed) => format!(
            "notification workspaceFileDeleted path={} sequence={} removed={} overlay_status={} overlay_generation={} overlay_files={} overlay_symbols={} elapsed_ms={}",
            path.display(), params.sequence, removed, status.status, status.generation,
            status.files, status.symbols, start.elapsed().as_millis()
        ),
        None => format!(
            "notification workspaceFileDeleted ignored path={} sequence={} elapsed_ms={}",
            path.display(), params.sequence, start.elapsed().as_millis()
        ),
    };
    vec![RuntimeEffect::Log(message)]
}
