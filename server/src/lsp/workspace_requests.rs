//! Workspace-index request execution.
//!
//! The external index remains owned by the composition root, while this
//! boundary owns the workspace request shape and translates index outcomes
//! into transport-neutral runtime effects.
use super::{ExternalIndexHandle, RuntimeEffect};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkspaceFileChangedParams {
    path: String,
    text: String,
    sequence: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkspaceFileDeletedParams {
    path: String,
    sequence: u64,
}

pub(super) fn update_workspace_file(
    external_index: &mut ExternalIndexHandle,
    params: Option<Value>,
) -> Result<Vec<RuntimeEffect>, String> {
    let Some(params) = params else {
        return Ok(Vec::new());
    };
    let params = serde_json::from_value::<WorkspaceFileChangedParams>(params)
        .map_err(|error| format!("Invalid workspaceFileChanged params: {error}"))?;
    let start = Instant::now();
    let path = PathBuf::from(params.path);
    let bytes = params.text.len();
    let result = external_index.update_workspace_file(path.clone(), params.text, params.sequence);
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
    Ok(vec![RuntimeEffect::Log(message)])
}

pub(super) fn delete_workspace_file(
    external_index: &mut ExternalIndexHandle,
    params: Option<Value>,
) -> Result<Vec<RuntimeEffect>, String> {
    let Some(params) = params else {
        return Ok(Vec::new());
    };
    let params = serde_json::from_value::<WorkspaceFileDeletedParams>(params)
        .map_err(|error| format!("Invalid workspaceFileDeleted params: {error}"))?;
    let start = Instant::now();
    let path = PathBuf::from(params.path);
    let removed = external_index.delete_workspace_file(&path, params.sequence);
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
    Ok(vec![RuntimeEffect::Log(message)])
}
