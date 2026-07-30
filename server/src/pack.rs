use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use flate2::read::ZlibDecoder;

const MAX_FILE_TABLE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 200_000;
const MAX_PATH_BYTES: usize = 1024;
const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_NESTING: usize = 128;
const MAX_SELECTED_ENTRIES: usize = 10_000;
const MAX_EXPANSION_RATIO: u64 = 128;
const DEFLATE_ZLIB_COMPRESSION: u32 = 0x106;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PakEntry {
    logical_path: String,
    offset: u64,
    compressed_length: u64,
    original_length: u64,
    compression: u32,
    archive_path: PathBuf,
}
impl PakEntry {
    pub fn logical_path(&self) -> &str {
        &self.logical_path
    }

    pub const fn offset(&self) -> u64 {
        self.offset
    }

    pub const fn compressed_length(&self) -> u64 {
        self.compressed_length
    }

    pub const fn original_length(&self) -> u64 {
        self.original_length
    }

    pub const fn compression(&self) -> u32 {
        self.compression
    }

    pub fn archive_path(&self) -> &Path {
        &self.archive_path
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PakSelection<'a> {
    Scripts,
    ExactPaths(&'a [&'a str]),
}
impl<'a> PakSelection<'a> {
    pub const fn scripts() -> Self {
        Self::Scripts
    }

    pub const fn exact_paths(paths: &'a [&'a str]) -> Self {
        Self::ExactPaths(paths)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    Io(String),
    Invalid(String),
    Limit(String),
    MissingPath(String),
    ArchiveMismatch { path: String },
    Cancelled,
    UnsupportedCompression { path: String, compression: u32 },
}
impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(v) | Self::Invalid(v) | Self::Limit(v) | Self::MissingPath(v) => {
                f.write_str(v)
            }
            Self::ArchiveMismatch { path } => {
                write!(f, "PAC1 entry does not belong to this archive: {path}")
            }
            Self::Cancelled => f.write_str("PAC1 extraction cancelled"),
            Self::UnsupportedCompression { path, compression } => {
                write!(f, "unsupported PAC1 compression {compression} for {path}")
            }
        }
    }
}
impl std::error::Error for PackError {}

#[derive(Debug, Clone)]
pub struct PakArchive {
    path: PathBuf,
    file_length: u64,
    entries: Vec<PakEntry>,
}
impl PakArchive {
    pub fn inspect(path: impl AsRef<Path>) -> Result<Self, PackError> {
        Self::inspect_with_cancel(path, || false)
    }

    pub fn inspect_with_cancel(
        path: impl AsRef<Path>,
        mut is_cancelled: impl FnMut() -> bool,
    ) -> Result<Self, PackError> {
        ensure_not_cancelled(&mut is_cancelled)?;
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path).map_err(io)?;
        let file_length = file.metadata().map_err(io)?.len();
        let mut header = [0; 12];
        read_exact(&mut file, &mut header)?;
        if &header[..4] != b"FORM" || &header[8..] != b"PAC1" {
            return Err(PackError::Invalid("not a PAC1 FORM archive".into()));
        }
        let form_length = u32::from_be_bytes(header[4..8].try_into().unwrap()) as u64;
        let end = 8_u64
            .checked_add(form_length)
            .filter(|v| *v <= file_length)
            .ok_or_else(|| PackError::Invalid("PAC1 FORM length exceeds archive".into()))?;
        let mut entries = Vec::new();
        let mut nodes = 0;
        while file.stream_position().map_err(io)? < end {
            ensure_not_cancelled(&mut is_cancelled)?;
            let mut chunk = [0; 8];
            read_exact(&mut file, &mut chunk)?;
            let length = u32::from_be_bytes(chunk[4..].try_into().unwrap()) as u64;
            let body = file.stream_position().map_err(io)?;
            let next = body
                .checked_add(length)
                .filter(|v| *v <= end)
                .ok_or_else(|| PackError::Invalid("PAC1 chunk exceeds FORM".into()))?;
            if &chunk[..4] == b"FILE" {
                if length > MAX_FILE_TABLE_BYTES {
                    return Err(PackError::Limit("PAC1 file table exceeds limit".into()));
                }
                let mut bytes = vec![0; length as usize];
                read_exact(&mut file, &mut bytes)?;
                let mut cursor = 0;
                parse_entry(
                    &bytes,
                    &mut cursor,
                    "",
                    &mut entries,
                    &path,
                    file_length,
                    0,
                    &mut nodes,
                    &mut is_cancelled,
                )?;
                if cursor != bytes.len() {
                    return Err(PackError::Invalid("trailing PAC1 file-table data".into()));
                }
            } else {
                file.seek(SeekFrom::Start(next)).map_err(io)?;
            }
        }
        if entries.is_empty() {
            return Err(PackError::Invalid(
                "PAC1 archive has no FILE catalogue".into(),
            ));
        }
        Ok(Self {
            path,
            file_length,
            entries,
        })
    }
    pub fn entries(&self) -> &[PakEntry] {
        &self.entries
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn select(&self, selection: PakSelection<'_>) -> Result<Vec<PakEntry>, PackError> {
        match selection {
            PakSelection::Scripts => self.collect_selected(
                self.entries
                    .iter()
                    .filter(|entry| has_extension(entry, "c")),
            ),
            PakSelection::ExactPaths(paths) => self.select_paths(paths.iter().copied()),
        }
    }

    pub fn select_paths<'a>(
        &self,
        paths: impl IntoIterator<Item = &'a str>,
    ) -> Result<Vec<PakEntry>, PackError> {
        let mut selected = Vec::new();
        for path in paths {
            validate_logical_path(path)?;
            if selected.len() == MAX_SELECTED_ENTRIES {
                return Err(PackError::Limit(
                    "PAC1 selected entry count exceeds limit".into(),
                ));
            }
            if selected
                .iter()
                .any(|entry: &PakEntry| entry.logical_path == path)
            {
                return Err(PackError::Invalid(format!(
                    "duplicate PAC1 selected path: {path}"
                )));
            }
            let entry = self
                .entries
                .iter()
                .find(|entry| entry.logical_path == path)
                .ok_or_else(|| PackError::MissingPath(format!("PAC1 path not found: {path}")))?;
            selected.push(entry.clone());
        }
        Ok(selected)
    }

    pub fn select_where(
        &self,
        mut predicate: impl FnMut(&PakEntry) -> bool,
    ) -> Result<Vec<PakEntry>, PackError> {
        self.collect_selected(self.entries.iter().filter(|entry| predicate(entry)))
    }

    fn collect_selected<'a>(
        &self,
        entries: impl Iterator<Item = &'a PakEntry>,
    ) -> Result<Vec<PakEntry>, PackError> {
        let mut selected = Vec::new();
        for entry in entries {
            if selected.len() == MAX_SELECTED_ENTRIES {
                return Err(PackError::Limit(
                    "PAC1 selected entry count exceeds limit".into(),
                ));
            }
            selected.push(entry.clone());
        }
        Ok(selected)
    }
    pub fn read_to(&self, entry: &PakEntry, output: &mut impl Write) -> Result<u64, PackError> {
        self.read_to_with_cancel(entry, output, || false)
    }

    /// Streams one selected entry. On error, the output may contain a prefix;
    /// callers that require an all-or-nothing result should use [`Self::read`].
    pub fn read_to_with_cancel(
        &self,
        entry: &PakEntry,
        output: &mut impl Write,
        is_cancelled: impl FnMut() -> bool,
    ) -> Result<u64, PackError> {
        if entry.archive_path != self.path {
            return Err(PackError::ArchiveMismatch {
                path: entry.logical_path.clone(),
            });
        }
        if entry.original_length > MAX_ENTRY_BYTES || entry.compressed_length > MAX_ENTRY_BYTES {
            return Err(PackError::Limit(format!(
                "PAC1 entry exceeds extraction limit: {}",
                entry.logical_path
            )));
        }
        if (entry.compressed_length == 0 && entry.original_length != 0)
            || entry.original_length > entry.compressed_length.saturating_mul(MAX_EXPANSION_RATIO)
        {
            return Err(PackError::Limit(format!(
                "PAC1 entry expansion ratio exceeds limit: {}",
                entry.logical_path
            )));
        }
        let _end = entry
            .offset
            .checked_add(entry.compressed_length)
            .filter(|v| *v <= self.file_length)
            .ok_or_else(|| {
                PackError::Invalid(format!(
                    "PAC1 entry is outside archive: {}",
                    entry.logical_path
                ))
            })?;
        let mut file = File::open(&self.path).map_err(io)?;
        file.seek(SeekFrom::Start(entry.offset)).map_err(io)?;
        match entry.compression {
            0 => {
                if entry.compressed_length != entry.original_length {
                    return Err(PackError::Invalid(format!(
                        "uncompressed PAC1 entry has mismatched lengths: {}",
                        entry.logical_path
                    )));
                }
                copy_exact(
                    &mut file.take(entry.compressed_length),
                    output,
                    entry.original_length,
                    is_cancelled,
                )
            }
            DEFLATE_ZLIB_COMPRESSION => {
                let mut decoder = ZlibDecoder::new(file.take(entry.compressed_length));
                copy_exact(&mut decoder, output, entry.original_length, is_cancelled)
            }
            compression => Err(PackError::UnsupportedCompression {
                path: entry.logical_path.clone(),
                compression,
            }),
        }
    }

    pub fn read(&self, entry: &PakEntry) -> Result<Vec<u8>, PackError> {
        let mut bytes = Vec::with_capacity(entry.original_length as usize);
        self.read_to(entry, &mut bytes)?;
        Ok(bytes)
    }
}

fn parse_entry(
    bytes: &[u8],
    cursor: &mut usize,
    parent: &str,
    entries: &mut Vec<PakEntry>,
    archive_path: &Path,
    file_length: u64,
    depth: usize,
    nodes: &mut usize,
    is_cancelled: &mut impl FnMut() -> bool,
) -> Result<(), PackError> {
    ensure_not_cancelled(is_cancelled)?;
    *nodes = nodes
        .checked_add(1)
        .ok_or_else(|| PackError::Limit("PAC1 entry count exceeds limit".into()))?;
    if *nodes > MAX_ENTRIES {
        return Err(PackError::Limit("PAC1 entry count exceeds limit".into()));
    }
    if depth > MAX_NESTING {
        return Err(PackError::Limit("PAC1 folder nesting exceeds limit".into()));
    }
    let kind = take(bytes, cursor, 1)?[0];
    let name_length = take(bytes, cursor, 1)?[0] as usize;
    if name_length > MAX_PATH_BYTES {
        return Err(PackError::Invalid("invalid PAC1 entry name length".into()));
    }
    let name = std::str::from_utf8(take(bytes, cursor, name_length)?)
        .map_err(|_| PackError::Invalid("PAC1 entry name is not UTF-8".into()))?;
    if name.is_empty() {
        if depth != 0 || kind != 0 {
            return Err(PackError::Invalid(
                "invalid empty PAC1 path component".into(),
            ));
        }
    } else {
        validate_component(name)?;
    }
    let path = if name.is_empty() {
        parent.to_owned()
    } else if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    };
    if path.len() > MAX_PATH_BYTES {
        return Err(PackError::Limit("PAC1 logical path exceeds limit".into()));
    }
    match kind {
        0 => {
            let count = u32::from_le_bytes(take(bytes, cursor, 4)?.try_into().unwrap()) as usize;
            if count > MAX_ENTRIES || entries.len().saturating_add(count) > MAX_ENTRIES {
                return Err(PackError::Limit("PAC1 entry count exceeds limit".into()));
            }
            for _ in 0..count {
                parse_entry(
                    bytes,
                    cursor,
                    &path,
                    entries,
                    archive_path,
                    file_length,
                    depth + 1,
                    nodes,
                    is_cancelled,
                )?;
            }
        }
        1 => {
            let offset = u32::from_le_bytes(take(bytes, cursor, 4)?.try_into().unwrap()) as u64;
            let compressed_length =
                u32::from_le_bytes(take(bytes, cursor, 4)?.try_into().unwrap()) as u64;
            let original_length =
                u32::from_le_bytes(take(bytes, cursor, 4)?.try_into().unwrap()) as u64;
            take(bytes, cursor, 4)?;
            let compression = u32::from_be_bytes(take(bytes, cursor, 4)?.try_into().unwrap());
            take(bytes, cursor, 4)?;
            if offset
                .checked_add(compressed_length)
                .is_none_or(|end| end > file_length)
            {
                return Err(PackError::Invalid(format!(
                    "PAC1 entry is outside archive: {path}"
                )));
            }
            if entries.iter().any(|entry| entry.logical_path == path) {
                return Err(PackError::Invalid(format!(
                    "duplicate PAC1 logical path: {path}"
                )));
            }
            entries.push(PakEntry {
                logical_path: path,
                offset,
                compressed_length,
                original_length,
                compression,
                archive_path: archive_path.to_path_buf(),
            });
        }
        _ => return Err(PackError::Invalid("unknown PAC1 entry kind".into())),
    }
    Ok(())
}

fn has_extension(entry: &PakEntry, extension: &str) -> bool {
    entry
        .logical_path
        .rsplit('.')
        .next()
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(extension))
}

fn validate_component(component: &str) -> Result<(), PackError> {
    if component.is_empty()
        || component.contains(['/', '\\'])
        || component == "."
        || component == ".."
    {
        return Err(PackError::Invalid(
            "invalid PAC1 logical path component".into(),
        ));
    }
    Ok(())
}

fn validate_logical_path(path: &str) -> Result<(), PackError> {
    if path.starts_with('/') || path.starts_with('\\') || path.contains(['\\', ':']) {
        return Err(PackError::Invalid("invalid PAC1 logical path".into()));
    }
    for component in path.split('/') {
        validate_component(component)?;
    }
    Ok(())
}

fn ensure_not_cancelled(is_cancelled: &mut impl FnMut() -> bool) -> Result<(), PackError> {
    if is_cancelled() {
        Err(PackError::Cancelled)
    } else {
        Ok(())
    }
}

fn copy_exact(
    input: &mut impl Read,
    output: &mut impl Write,
    expected_length: u64,
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<u64, PackError> {
    let mut copied = 0_u64;
    let mut buffer = [0; 32 * 1024];
    loop {
        if is_cancelled() {
            return Err(PackError::Cancelled);
        }
        let read = input.read(&mut buffer).map_err(io)?;
        if read == 0 {
            break;
        }
        copied = copied.checked_add(read as u64).ok_or_else(|| {
            PackError::Limit("PAC1 entry length overflow during extraction".into())
        })?;
        if copied > expected_length {
            return Err(PackError::Limit(format!(
                "PAC1 entry exceeds declared uncompressed length: expected {expected_length}"
            )));
        }
        output.write_all(&buffer[..read]).map_err(io)?;
    }
    if copied != expected_length {
        return Err(PackError::Invalid(format!(
            "PAC1 entry length differs from catalogue: expected {expected_length}, read {copied}"
        )));
    }
    Ok(copied)
}
fn take<'a>(bytes: &'a [u8], cursor: &mut usize, count: usize) -> Result<&'a [u8], PackError> {
    let end = cursor
        .checked_add(count)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| PackError::Invalid("truncated PAC1 file table".into()))?;
    let result = &bytes[*cursor..end];
    *cursor = end;
    Ok(result)
}
fn read_exact(file: &mut File, bytes: &mut [u8]) -> Result<(), PackError> {
    file.read_exact(bytes).map_err(io)
}
fn io(error: std::io::Error) -> PackError {
    PackError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::{write::ZlibEncoder, Compression};

    use super::{PackError, PakArchive, PakSelection, DEFLATE_ZLIB_COMPRESSION};

    #[test]
    fn selects_and_reads_only_script_entries() {
        let path = fixture_pak(&[
            (
                "Feature.c",
                b"class Feature {}",
                0,
                b"class Feature {}".len(),
            ),
            ("Icon.edds", b"not source", 0, b"not source".len()),
        ]);
        let archive = PakArchive::inspect(&path).unwrap();
        let scripts = archive.select(PakSelection::scripts()).unwrap();

        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].logical_path(), "Root/Feature.c");
        assert_eq!(archive.read(&scripts[0]).unwrap(), b"class Feature {}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn selects_exact_logical_paths_without_reading_other_entries() {
        let path = fixture_pak(&[
            (
                "Feature.c",
                b"class Feature {}",
                0,
                b"class Feature {}".len(),
            ),
            ("Other.c", b"class Other {}", 0, b"class Other {}".len()),
        ]);
        let archive = PakArchive::inspect(&path).unwrap();
        let selected = archive
            .select(PakSelection::exact_paths(&["Root/Other.c"]))
            .unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].logical_path(), "Root/Other.c");
        assert_eq!(archive.read(&selected[0]).unwrap(), b"class Other {}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn inflates_deflate_entries_without_materializing_the_archive() {
        let source = b"class CompressedFeature {}";
        let mut zlib_deflate = ZlibEncoder::new(Vec::new(), Compression::default());
        zlib_deflate.write_all(source).unwrap();
        let zlib_deflate = zlib_deflate.finish().unwrap();
        let path = fixture_pak(&[(
            "Compressed.c",
            &zlib_deflate,
            DEFLATE_ZLIB_COMPRESSION,
            source.len(),
        )]);
        let archive = PakArchive::inspect(&path).unwrap();
        let entry = &archive.entries()[0];

        let mut output = Vec::new();
        assert_eq!(
            archive.read_to(entry, &mut output).unwrap(),
            source.len() as u64
        );
        assert_eq!(output, source);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_an_entry_from_a_different_archive() {
        let first = fixture_pak(&[(
            "Feature.c",
            b"class Feature {}",
            0,
            b"class Feature {}".len(),
        )]);
        let second = fixture_pak(&[("Other.c", b"class Other {}", 0, b"class Other {}".len())]);
        let first_archive = PakArchive::inspect(&first).unwrap();
        let second_archive = PakArchive::inspect(&second).unwrap();

        assert!(matches!(
            second_archive.read(&first_archive.entries()[0]),
            Err(PackError::ArchiveMismatch { .. })
        ));
        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn reports_unsupported_compression_without_reading_a_different_entry() {
        let path = fixture_pak(&[("Feature.c", b"opaque", 7, b"opaque".len())]);
        let archive = PakArchive::inspect(&path).unwrap();

        assert!(matches!(
            archive.read(&archive.entries()[0]),
            Err(PackError::UnsupportedCompression { compression: 7, .. })
        ));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn stops_streaming_when_cancelled() {
        let path = fixture_pak(&[(
            "Feature.c",
            b"class Feature {}",
            0,
            b"class Feature {}".len(),
        )]);
        let archive = PakArchive::inspect(&path).unwrap();
        let mut output = Vec::new();

        assert!(matches!(
            archive.read_to_with_cancel(&archive.entries()[0], &mut output, || true),
            Err(PackError::Cancelled)
        ));
        assert!(output.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn stops_inspection_when_cancelled() {
        let path = fixture_pak(&[(
            "Feature.c",
            b"class Feature {}",
            0,
            b"class Feature {}".len(),
        )]);

        assert!(matches!(
            PakArchive::inspect_with_cancel(&path, || true),
            Err(PackError::Cancelled)
        ));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reads_an_empty_uncompressed_script() {
        let path = fixture_pak(&[("Empty.c", b"", 0, 0)]);
        let archive = PakArchive::inspect(&path).unwrap();

        assert_eq!(archive.read(&archive.entries()[0]).unwrap(), b"");
        let _ = std::fs::remove_file(path);
    }

    fn fixture_pak(entries: &[(&str, &[u8], u32, usize)]) -> std::path::PathBuf {
        let mut table = Vec::new();
        table.push(0);
        table.push(4);
        table.extend_from_slice(b"Root");
        table.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        let table_length = 1
            + 1
            + 4
            + 4
            + entries
                .iter()
                .map(|(path, _, _, _)| 1 + 1 + path.len() + 24)
                .sum::<usize>();
        let mut offset = 12 + 8 + 28 + 8 + table_length + 8;
        for (path, content, compression, original_length) in entries {
            table.push(1);
            table.push(path.len() as u8);
            table.extend_from_slice(path.as_bytes());
            table.extend_from_slice(&(offset as u32).to_le_bytes());
            table.extend_from_slice(&(content.len() as u32).to_le_bytes());
            table.extend_from_slice(&(*original_length as u32).to_le_bytes());
            table.extend_from_slice(&[0; 4]);
            table.extend_from_slice(&compression.to_be_bytes());
            table.extend_from_slice(&[0; 4]);
            offset += content.len();
        }
        let mut bytes = b"FORM".to_vec();
        let form_length = (4
            + 8
            + 28
            + 8
            + table.len()
            + 8
            + entries
                .iter()
                .map(|(_, content, _, _)| content.len())
                .sum::<usize>()) as u32;
        bytes.extend_from_slice(&form_length.to_be_bytes());
        bytes.extend_from_slice(b"PAC1");
        bytes.extend_from_slice(b"HEAD");
        bytes.extend_from_slice(&28_u32.to_be_bytes());
        bytes.extend_from_slice(&[0; 28]);
        bytes.extend_from_slice(b"FILE");
        bytes.extend_from_slice(&(table.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&table);
        bytes.extend_from_slice(b"DATA");
        bytes.extend_from_slice(
            &(entries
                .iter()
                .map(|(_, content, _, _)| content.len())
                .sum::<usize>() as u32)
                .to_be_bytes(),
        );
        for (_, content, _, _) in entries {
            bytes.extend_from_slice(content);
        }
        let path = std::env::temp_dir().join(format!(
            "rst_pack_{}_{}.pak",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, bytes).unwrap();
        path
    }
}
