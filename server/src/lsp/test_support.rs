use std::path::PathBuf;

pub(super) fn timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub(super) fn test_log_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "reforger_lsp_{name}_{}_{}.log",
        std::process::id(),
        timestamp_millis()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

pub(super) fn cleanup_log(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}

pub(super) fn temp_test_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "reforger_lsp_{name}_{}_{}",
        std::process::id(),
        timestamp_millis()
    ));
    let _ = std::fs::remove_dir_all(&path);
    path
}
