use std::path::{Component, Path, PathBuf};
use url::Url;

pub(super) fn file_path_identity(path: &Path) -> Option<String> {
    if !path.is_absolute() {
        return None;
    }
    let normalized = path
        .canonicalize()
        .unwrap_or_else(|_| lexically_normalized(path));
    let identity = normalized.to_string_lossy().replace('\\', "/");
    Some(if cfg!(windows) {
        identity.to_ascii_lowercase()
    } else {
        identity
    })
}

pub(super) fn file_uri_path_identity(uri: &str) -> Option<String> {
    let path = Url::parse(uri).ok()?.to_file_path().ok()?;
    file_path_identity(&path)
}

fn lexically_normalized(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vscode_and_server_windows_file_uris_share_one_path_identity() {
        if cfg!(windows) {
            assert_eq!(
                file_uri_path_identity("file:///C:/Game%20Data/Scripts/File.c"),
                file_uri_path_identity("file:///c%3A/Game%20Data/Scripts/File.c")
            );
        }
    }

    #[test]
    fn file_uri_identity_normalizes_dot_segments_and_percent_encoding() {
        let root = std::env::temp_dir().join("reforger file identity");
        let path = root.join("Scripts").join("File.c");
        let uri = Url::from_file_path(&path).unwrap();
        let dotted_uri = uri
            .as_str()
            .replace("/Scripts/File.c", "/nested/../Scripts/File.c");

        assert_eq!(
            file_uri_path_identity(&dotted_uri),
            file_path_identity(&path)
        );
    }
}
