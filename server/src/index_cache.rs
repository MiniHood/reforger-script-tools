use crate::ast::DocCommentKind;
use crate::index::{
    GlobalSymbolId, IndexedAttribute, IndexedConditionalBranch, IndexedDocComment, IndexedFile,
    IndexedSymbol, IndexedSymbolDetail, SourceFileId, SymbolIndex,
};
use crate::index_build::{build_index, IndexBuildConfig, IndexBuildResult, IndexSourceRoot};
use crate::lexer::TextSpan;
use crate::model::{
    CallableForm, PreprocessorBranchKind, SourceCategory, SourceFileMetadata, SourceKind, SymbolId,
    SymbolKind, SOURCE_PRIORITY_GAME_DATA,
};
use crate::semantic_file::{
    FileContribution, PublicSymbol, PublicSymbolDetail, PublicText, SemanticCallableForm,
    SemanticConditionalBranch, SemanticConditionalBranchKind, SemanticDeclarationId,
    SemanticDeclarationKind, SemanticDocComment, SemanticDocCommentKind, SemanticText,
    FILE_CONTRIBUTION_SCHEMA_VERSION, FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

const CACHE_FORMAT_VERSION: u32 = 11;
const CACHE_SCHEMA: &str = "reforger-symbol-index";
const CACHE_MAGIC: &[u8; 8] = b"RSTIDX11";
const CACHE_INDEX_SHAPE: &str =
    "runtime-pruned:no-local-variables:detail-spans-stripped:layered-external-v1:binary-v4:string-table-v1:canonical-public-facts-v1";
const LEGACY_CACHE_FORMAT_VERSION: u32 = 9;
const LEGACY_CACHE_MAGIC: &[u8; 8] = b"RSTIDX09";
const V10_CACHE_FORMAT_VERSION: u32 = 10;
const V10_CACHE_MAGIC: &[u8; 8] = b"RSTIDX10";
const V10_CACHE_INDEX_SHAPE: &str =
    "runtime-pruned:no-local-variables:detail-spans-stripped:layered-external-v1:binary-v3:string-table-v1:validated-file-contributions-v1";
const LEGACY_CACHE_INDEX_SHAPE: &str =
    "runtime-pruned:no-local-variables:detail-spans-stripped:layered-external-v1:binary-v2:string-table-v1";
const MAX_CACHE_STRING_TABLE_ENTRIES: usize = 1_000_000;
const MAX_CACHE_RAW_STRING_BYTES: usize = 16 * 1024 * 1024;
const MAX_CACHE_FILE_RECORDS: usize = 1_000_000;
const MAX_CACHE_SYMBOL_RECORDS: usize = 5_000_000;
const MAX_CACHE_SYMBOL_LIST_ITEMS: usize = 1_000_000;
/// v10 was intentionally larger than the canonical v11 payload because it
/// persisted both a query graph and JSON contributions. Read no more than a
/// plausible legacy game-data artifact before asking the allocator to hold it.
const MAX_LEGACY_CACHE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug)]
pub struct GameDataIndexCacheConfig {
    pub scripts_root: PathBuf,
    pub cache_path: PathBuf,
    pub metadata_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct GameDataIndexCacheResult {
    pub index: SymbolIndex,
    pub summary: RuntimeIndexSummary,
    pub cache_status: IndexCacheStatus,
    pub fingerprint: SourceFingerprint,
    pub timings: IndexCacheTimings,
    pub cache_file_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexCacheTimings {
    pub fingerprint: Duration,
    pub cache_file_read: Duration,
    pub cache_decode: Duration,
    pub cache_validate: Duration,
    pub map_rebuild: Duration,
    pub cache_read_deserialize_validate: Duration,
    pub rebuild: Duration,
    pub cache_write: Duration,
    pub total: Duration,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeIndexSummary {
    pub files: usize,
    pub bytes: usize,
    pub indexed_symbols: usize,
    pub parse_diagnostics: usize,
    pub lossy_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexCacheStatus {
    Loaded,
    Rebuilt { reason: String },
}

impl IndexCacheStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Loaded => "loaded",
            Self::Rebuilt { .. } => "rebuilt",
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Loaded => None,
            Self::Rebuilt { reason } => Some(reason.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SourceFingerprint {
    Downloaded {
        scripts_root: String,
        commit_sha: String,
    },
    Manual {
        scripts_root: String,
        file_count: usize,
        byte_count: u64,
        latest_modified_unix_ms: u128,
    },
}

#[derive(Debug, Clone)]
struct CachedGameDataIndex {
    schema: String,
    format_version: u32,
    index_shape: String,
    crate_version: String,
    fingerprint: SourceFingerprint,
    summary: CachedIndexSummary,
    files: Vec<CachedFileContribution>,
}

/// The cache's canonical payload.  This intentionally is neither a
/// `FileContribution` (which carries source-only spans/container text) nor a
/// serialized `SymbolIndex` (which duplicates all derived runtime records).
/// It contains exactly the source metadata and public facts needed to rebuild
/// a runtime contribution and its lookup maps.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedFileContribution {
    metadata: SourceFileMetadata,
    non_declaration_callable_fragments: usize,
    symbols: Vec<CachedPublicSymbol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedPublicSymbol {
    id: SemanticDeclarationId,
    parent: Option<SemanticDeclarationId>,
    kind: SemanticDeclarationKind,
    name: String,
    span: TextSpan,
    selection_span: TextSpan,
    detail: CachedPublicSymbolDetail,
    attributes: Vec<String>,
    modifiers: Vec<String>,
    doc_comments: Vec<CachedDocComment>,
    conditional_context: Vec<CachedConditionalBranch>,
    callable_form: Option<SemanticCallableForm>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CachedPublicSymbolDetail {
    type_text: Option<String>,
    return_type: Option<String>,
    base_type: Option<String>,
    default_value: Option<String>,
    enum_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedDocComment {
    kind: SemanticDocCommentKind,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedConditionalBranch {
    kind: SemanticConditionalBranchKind,
    condition: Option<String>,
}

/// The former v10 payload. It is decoded only to make a one-way v11 cache
/// replacement and is never published as a query representation.
#[derive(Debug)]
struct V10CachedGameDataIndex {
    schema: String,
    format_version: u32,
    index_shape: String,
    crate_version: String,
    fingerprint: SourceFingerprint,
    summary: CachedIndexSummary,
    index: V10CachedSymbolIndex,
}

#[derive(Debug)]
struct V10CachedSymbolIndex {
    files: Vec<IndexedFile>,
    symbols: Vec<IndexedSymbol>,
    contributions: Vec<FileContribution>,
}
/// A v9 cache had the same binary records as v10 up to its indexed symbols,
/// but no compiler-owned public contribution payload. It is read only as a
/// one-way migration input and is never an alternate runtime representation.
#[derive(Debug)]
struct LegacyCachedGameDataIndex {
    schema: String,
    format_version: u32,
    index_shape: String,
    crate_version: String,
    fingerprint: SourceFingerprint,
    summary: CachedIndexSummary,
    index: LegacyCachedSymbolIndex,
}

#[derive(Debug)]
struct LegacyCachedSymbolIndex {
    files: Vec<IndexedFile>,
    symbols: Vec<IndexedSymbol>,
}

impl CachedFileContribution {
    fn from_file_contribution(
        metadata: SourceFileMetadata,
        contribution: FileContribution,
    ) -> Result<Self, String> {
        let mut symbols = Vec::with_capacity(contribution.symbols.len());
        for symbol in contribution.symbols {
            let Some(name) = symbol.name else {
                return Err("v10 public contribution contains an unnamed symbol".to_string());
            };
            symbols.push(CachedPublicSymbol {
                id: symbol.id,
                parent: symbol.parent,
                kind: symbol.kind,
                name,
                span: symbol.span,
                selection_span: symbol.selection_span,
                detail: CachedPublicSymbolDetail {
                    type_text: symbol.detail.type_text.map(|text| text.text),
                    return_type: symbol.detail.return_type.map(|text| text.text),
                    base_type: symbol.detail.base_type.map(|text| text.text),
                    default_value: symbol.detail.default_value.map(|text| text.text),
                    enum_value: symbol.detail.enum_value.map(|text| text.text),
                },
                attributes: symbol
                    .attributes
                    .into_iter()
                    .map(|text| text.text)
                    .collect(),
                modifiers: symbol.modifiers.into_iter().map(|text| text.text).collect(),
                doc_comments: symbol
                    .doc_comments
                    .into_iter()
                    .map(|comment| CachedDocComment {
                        kind: comment.kind,
                        text: comment.text,
                    })
                    .collect(),
                conditional_context: symbol
                    .conditional_context
                    .into_iter()
                    .map(|branch| CachedConditionalBranch {
                        kind: branch.kind,
                        condition: branch.condition.map(|text| text.text),
                    })
                    .collect(),
                callable_form: symbol.callable_form,
            });
        }
        Ok(Self {
            metadata,
            non_declaration_callable_fragments: contribution.non_declaration_callable_fragments,
            symbols,
        }
        .with_contiguous_ids())
    }

    fn from_indexed_file(file: &IndexedFile, symbols: &[IndexedSymbol]) -> Self {
        let public_symbols = symbols
            .iter()
            .filter(|symbol| symbol.name.is_some())
            .filter_map(|symbol| {
                let kind = public_semantic_kind(symbol.kind)?;
                Some(CachedPublicSymbol {
                    id: SemanticDeclarationId(symbol.id.symbol_id.0 as u32),
                    parent: symbol
                        .parent
                        .map(|parent| SemanticDeclarationId(parent.symbol_id.0 as u32)),
                    kind,
                    name: symbol
                        .name
                        .clone()
                        .expect("named public symbols are filtered above"),
                    span: symbol.span,
                    selection_span: symbol.selection_span,
                    detail: CachedPublicSymbolDetail {
                        type_text: symbol.detail.type_text.clone(),
                        return_type: symbol.detail.return_type_text.clone(),
                        base_type: symbol.detail.base_type.clone(),
                        default_value: symbol.detail.default_text.clone(),
                        enum_value: symbol.detail.enum_value_text.clone(),
                    },
                    attributes: symbol
                        .attributes
                        .iter()
                        .map(|attribute| attribute.text.clone())
                        .collect(),
                    modifiers: symbol.modifiers.clone(),
                    doc_comments: symbol
                        .doc_comments
                        .iter()
                        .map(|comment| CachedDocComment {
                            kind: match comment.kind {
                                DocCommentKind::Line => SemanticDocCommentKind::Line,
                                DocCommentKind::Block => SemanticDocCommentKind::Block,
                            },
                            text: comment.text.clone(),
                        })
                        .collect(),
                    conditional_context: symbol
                        .conditional_context
                        .iter()
                        .map(|branch| CachedConditionalBranch {
                            kind: match branch.kind {
                                PreprocessorBranchKind::If => SemanticConditionalBranchKind::If,
                                PreprocessorBranchKind::Ifdef => {
                                    SemanticConditionalBranchKind::Ifdef
                                }
                                PreprocessorBranchKind::Ifndef => {
                                    SemanticConditionalBranchKind::Ifndef
                                }
                                PreprocessorBranchKind::Elif => SemanticConditionalBranchKind::Elif,
                                PreprocessorBranchKind::Else => SemanticConditionalBranchKind::Else,
                            },
                            condition: branch.condition.clone(),
                        })
                        .collect(),
                    callable_form: symbol.callable_form.map(|form| match form {
                        CallableForm::Implementation => SemanticCallableForm::Implementation,
                        CallableForm::Declaration => SemanticCallableForm::Declaration,
                        CallableForm::Prototype => SemanticCallableForm::Prototype,
                    }),
                })
            })
            .collect::<Vec<_>>();
        Self {
            metadata: file.metadata.clone(),
            non_declaration_callable_fragments: file.non_declaration_callable_fragments,
            symbols: public_symbols,
        }
        .with_contiguous_ids()
    }

    fn with_contiguous_ids(mut self) -> Self {
        let remapped: BTreeMap<_, _> = self
            .symbols
            .iter()
            .enumerate()
            .map(|(next, symbol)| (symbol.id, SemanticDeclarationId(next as u32)))
            .collect();
        for symbol in &mut self.symbols {
            symbol.parent = symbol.parent.map(|parent| remapped[&parent]);
            symbol.id = remapped[&symbol.id];
        }
        self
    }

    #[cfg(test)]
    fn to_file_contribution(&self) -> FileContribution {
        FileContribution {
            schema_version: FILE_CONTRIBUTION_SCHEMA_VERSION,
            source_manifest_version: FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION,
            non_declaration_callable_fragments: self.non_declaration_callable_fragments,
            symbols: self
                .symbols
                .iter()
                .map(|symbol| PublicSymbol {
                    id: symbol.id,
                    parent: symbol.parent,
                    kind: symbol.kind,
                    name: Some(symbol.name.clone()),
                    container: None,
                    detail: PublicSymbolDetail {
                        type_text: cached_public_text(None, symbol.detail.type_text.clone()),
                        return_type: cached_public_text(None, symbol.detail.return_type.clone()),
                        base_type: cached_public_text(None, symbol.detail.base_type.clone()),
                        default_value: cached_public_text(
                            None,
                            symbol.detail.default_value.clone(),
                        ),
                        enum_value: cached_public_text(None, symbol.detail.enum_value.clone()),
                    },
                    span: symbol.span,
                    selection_span: symbol.selection_span,
                    modifiers: symbol
                        .modifiers
                        .iter()
                        .cloned()
                        .map(|text| SemanticText {
                            span: symbol.span,
                            text,
                        })
                        .collect(),
                    attributes: symbol
                        .attributes
                        .iter()
                        .cloned()
                        .map(|text| SemanticText {
                            span: symbol.span,
                            text,
                        })
                        .collect(),
                    doc_comments: symbol
                        .doc_comments
                        .iter()
                        .map(|comment| SemanticDocComment {
                            span: symbol.span,
                            kind: comment.kind,
                            text: comment.text.clone(),
                        })
                        .collect(),
                    conditional_context: symbol
                        .conditional_context
                        .iter()
                        .map(|branch| SemanticConditionalBranch {
                            kind: branch.kind,
                            directive_span: symbol.span,
                            condition: branch.condition.clone().map(|text| SemanticText {
                                span: symbol.span,
                                text,
                            }),
                        })
                        .collect(),
                    callable_form: symbol.callable_form,
                })
                .collect(),
        }
    }

    fn into_file_contribution_with_metadata(self) -> (FileContribution, SourceFileMetadata) {
        let Self {
            metadata,
            non_declaration_callable_fragments,
            symbols,
        } = self;
        let contribution = FileContribution {
            schema_version: FILE_CONTRIBUTION_SCHEMA_VERSION,
            source_manifest_version: FILE_CONTRIBUTION_SOURCE_MANIFEST_VERSION,
            non_declaration_callable_fragments,
            symbols: symbols
                .into_iter()
                .map(|symbol| PublicSymbol {
                    id: symbol.id,
                    parent: symbol.parent,
                    kind: symbol.kind,
                    name: Some(symbol.name),
                    container: None,
                    detail: PublicSymbolDetail {
                        type_text: cached_public_text(None, symbol.detail.type_text),
                        return_type: cached_public_text(None, symbol.detail.return_type),
                        base_type: cached_public_text(None, symbol.detail.base_type),
                        default_value: cached_public_text(None, symbol.detail.default_value),
                        enum_value: cached_public_text(None, symbol.detail.enum_value),
                    },
                    span: symbol.span,
                    selection_span: symbol.selection_span,
                    modifiers: symbol
                        .modifiers
                        .into_iter()
                        .map(|text| SemanticText {
                            span: symbol.span,
                            text,
                        })
                        .collect(),
                    attributes: symbol
                        .attributes
                        .into_iter()
                        .map(|text| SemanticText {
                            span: symbol.span,
                            text,
                        })
                        .collect(),
                    doc_comments: symbol
                        .doc_comments
                        .into_iter()
                        .map(|comment| SemanticDocComment {
                            span: symbol.span,
                            kind: comment.kind,
                            text: comment.text,
                        })
                        .collect(),
                    conditional_context: symbol
                        .conditional_context
                        .into_iter()
                        .map(|branch| SemanticConditionalBranch {
                            kind: branch.kind,
                            directive_span: symbol.span,
                            condition: branch.condition.map(|text| SemanticText {
                                span: symbol.span,
                                text,
                            }),
                        })
                        .collect(),
                    callable_form: symbol.callable_form,
                })
                .collect(),
        };
        (contribution, metadata)
    }

    /// Validates the same identity contract as `FileContribution` without
    /// materializing a second, string-owning contribution during warm loads.
    fn validate(&self) -> Result<(), String> {
        for (expected, symbol) in self.symbols.iter().enumerate() {
            let expected = SemanticDeclarationId(expected as u32);
            if symbol.name.is_empty() {
                return Err(format!(
                    "invalid cached public file contribution: symbol {:?} has an empty name",
                    symbol.id
                ));
            }
            if symbol.id != expected {
                return Err(format!(
                    "invalid cached public file contribution: expected dense id {:?}, found {:?}",
                    expected, symbol.id
                ));
            }
            if let Some(parent) = symbol.parent {
                if parent.0 as usize >= self.symbols.len() {
                    return Err(format!(
                        "invalid cached public file contribution: symbol {:?} has missing parent {:?}",
                        symbol.id, parent
                    ));
                }
            }
        }
        Ok(())
    }
}

impl CachedGameDataIndex {
    fn from_index(
        index: &SymbolIndex,
        fingerprint: SourceFingerprint,
        summary: CachedIndexSummary,
    ) -> Self {
        Self {
            schema: CACHE_SCHEMA.to_string(),
            format_version: CACHE_FORMAT_VERSION,
            index_shape: CACHE_INDEX_SHAPE.to_string(),
            crate_version: env!("CARGO_PKG_VERSION").to_string(),
            fingerprint,
            summary,
            files: index
                .files()
                .iter()
                .map(|file| {
                    let end = file.symbol_start + file.symbol_count;
                    CachedFileContribution::from_indexed_file(
                        file,
                        &index.symbols()[file.symbol_start..end],
                    )
                })
                .collect(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        self.files
            .iter()
            .try_for_each(CachedFileContribution::validate)
    }

    fn into_index(self) -> SymbolIndex {
        let contributions = self
            .files
            .into_iter()
            .map(CachedFileContribution::into_file_contribution_with_metadata)
            .collect::<Vec<_>>();
        let mut index = SymbolIndex::default();
        index
            .add_owned_file_contributions(contributions)
            .expect("cached contributions were validated before index reconstruction");
        index
    }
}

impl LegacyCachedGameDataIndex {
    /// Converts a fully validated v9 snapshot into the current compiler-owned
    /// contribution contract. The v9 `SymbolIndex` records are source-derived
    /// facts; they are not used as a fallback query model after migration.
    fn into_current(self) -> CachedGameDataIndex {
        let index = SymbolIndex::from_indexed_parts(self.index.files, self.index.symbols);
        CachedGameDataIndex::from_index(&index, self.fingerprint, self.summary)
    }

    fn validates_for_migration(&self, expected_fingerprint: &SourceFingerprint) -> bool {
        self.schema == CACHE_SCHEMA
            && self.format_version == LEGACY_CACHE_FORMAT_VERSION
            && self.index_shape == LEGACY_CACHE_INDEX_SHAPE
            && self.crate_version == env!("CARGO_PKG_VERSION")
            && self.fingerprint == *expected_fingerprint
            && validate_legacy_index_records(&self.index.files, &self.index.symbols).is_ok()
    }
}

impl V10CachedGameDataIndex {
    fn into_current(self) -> Result<CachedGameDataIndex, String> {
        let V10CachedGameDataIndex {
            schema,
            crate_version,
            fingerprint,
            summary,
            index,
            ..
        } = self;
        let V10CachedSymbolIndex {
            files,
            symbols,
            contributions,
        } = index;
        // The indexed graph was used only to validate parity with the
        // canonical contribution facts. Release it before projecting v11 so a
        // migration never retains both legacy records and its replacement.
        drop(symbols);
        let files = files
            .into_iter()
            .zip(contributions)
            .map(|(file, contribution)| {
                CachedFileContribution::from_file_contribution(file.metadata, contribution)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CachedGameDataIndex {
            schema,
            format_version: CACHE_FORMAT_VERSION,
            index_shape: CACHE_INDEX_SHAPE.to_string(),
            crate_version,
            fingerprint,
            summary,
            files,
        })
    }

    fn validates_for_migration(&self, expected_fingerprint: &SourceFingerprint) -> bool {
        self.schema == CACHE_SCHEMA
            && self.format_version == V10_CACHE_FORMAT_VERSION
            && self.index_shape == V10_CACHE_INDEX_SHAPE
            && self.crate_version == env!("CARGO_PKG_VERSION")
            && self.fingerprint == *expected_fingerprint
            && self.index.files.len() == self.index.contributions.len()
            && self
                .index
                .contributions
                .iter()
                .all(|contribution| contribution.validate().is_ok())
            && validate_legacy_index_records(&self.index.files, &self.index.symbols).is_ok()
            && self.runtime_facts_match_contributions()
    }

    /// A v10 payload duplicated its query graph and its compiler contribution
    /// facts. Require those two representations to agree before trusting the
    /// contribution side as the one canonical v11 source of truth.
    fn runtime_facts_match_contributions(&self) -> bool {
        self.index
            .files
            .iter()
            .zip(&self.index.contributions)
            .all(|(file, contribution)| {
                let range_end = match file.symbol_start.checked_add(file.symbol_count) {
                    Some(end) => end,
                    None => return false,
                };
                let Some(symbols) = self.index.symbols.get(file.symbol_start..range_end) else {
                    return false;
                };
                let expected = CachedFileContribution::from_indexed_file(file, symbols);
                let actual = match CachedFileContribution::from_file_contribution(
                    file.metadata.clone(),
                    contribution.clone(),
                ) {
                    Ok(actual) => actual,
                    Err(_) => return false,
                };
                expected == actual
            })
    }
}

/// Establish the structural facts the old binary format did not serialize as
/// a versioned semantic contribution. Reject instead of attempting a partial
/// projection: a cache is disposable, while a bad external index is visible to
/// every language feature.
fn validate_legacy_index_records(
    files: &[IndexedFile],
    symbols: &[IndexedSymbol],
) -> Result<(), String> {
    let files_by_id: BTreeMap<_, _> = files.iter().map(|file| (file.id, file)).collect();
    if files_by_id.len() != files.len() {
        return Err("legacy cache contains duplicate file identifiers".to_string());
    }

    let symbols_by_id: BTreeMap<_, _> = symbols.iter().map(|symbol| (symbol.id, symbol)).collect();
    if symbols_by_id.len() != symbols.len() {
        return Err("legacy cache contains duplicate symbol identifiers".to_string());
    }

    for file in files {
        let range_end = file
            .symbol_start
            .checked_add(file.symbol_count)
            .ok_or_else(|| {
                format!(
                    "legacy cache file {:?} has an overflowing symbol range",
                    file.id
                )
            })?;
        let Some(range) = symbols.get(file.symbol_start..range_end) else {
            return Err(format!(
                "legacy cache file {:?} has an out-of-bounds symbol range",
                file.id
            ));
        };
        if range.iter().any(|symbol| symbol.id.file_id != file.id) {
            return Err(format!(
                "legacy cache file {:?} has a mixed-file symbol range",
                file.id
            ));
        }
        let actual_count = symbols
            .iter()
            .filter(|symbol| symbol.id.file_id == file.id)
            .count();
        if actual_count != file.symbol_count {
            return Err(format!(
                "legacy cache file {:?} declares {} symbols but contains {actual_count}",
                file.id, file.symbol_count
            ));
        }
    }

    let projected_ids: BTreeMap<_, _> = symbols
        .iter()
        .filter(|symbol| symbol.name.is_some() && public_semantic_kind(symbol.kind).is_some())
        .map(|symbol| (symbol.id, ()))
        .collect();

    for symbol in symbols {
        if !files_by_id.contains_key(&symbol.id.file_id) {
            return Err(format!(
                "legacy cache symbol {:?} references an unknown file",
                symbol.id
            ));
        }
        if let Some(parent) = symbol.parent {
            let Some(parent_symbol) = symbols_by_id.get(&parent) else {
                return Err(format!(
                    "legacy cache symbol {:?} references a missing parent {:?}",
                    symbol.id, parent
                ));
            };
            if parent.file_id != symbol.id.file_id {
                return Err(format!(
                    "legacy cache symbol {:?} references a parent in another file",
                    symbol.id
                ));
            }
            if public_semantic_kind(symbol.kind).is_some()
                && public_semantic_kind(parent_symbol.kind).is_none()
            {
                return Err(format!(
                    "legacy cache public symbol {:?} references a non-public parent",
                    symbol.id
                ));
            }
            if public_semantic_kind(symbol.kind).is_some()
                && symbol.name.is_some()
                && !projected_ids.contains_key(&parent)
            {
                return Err(format!(
                    "legacy cache public symbol {:?} references a parent omitted from the public projection",
                    symbol.id
                ));
            }
        }
    }
    Ok(())
}

fn cached_public_text(span: Option<TextSpan>, text: Option<String>) -> Option<PublicText> {
    text.map(|text| PublicText { span, text })
}

fn public_semantic_kind(kind: SymbolKind) -> Option<SemanticDeclarationKind> {
    Some(match kind {
        SymbolKind::Class => SemanticDeclarationKind::Class,
        SymbolKind::Enum => SemanticDeclarationKind::Enum,
        SymbolKind::EnumMember => SemanticDeclarationKind::EnumMember,
        SymbolKind::Typedef => SemanticDeclarationKind::Typedef,
        SymbolKind::Function => SemanticDeclarationKind::Function,
        SymbolKind::GlobalField => SemanticDeclarationKind::GlobalField,
        SymbolKind::Field => SemanticDeclarationKind::Field,
        SymbolKind::Method => SemanticDeclarationKind::Method,
        SymbolKind::Constructor => SemanticDeclarationKind::Constructor,
        SymbolKind::Destructor => SemanticDeclarationKind::Destructor,
        SymbolKind::PreprocessorMacro => SemanticDeclarationKind::PreprocessorMacro,
        SymbolKind::TypeParameter => SemanticDeclarationKind::TypeParameter,
        SymbolKind::Parameter => SemanticDeclarationKind::Parameter,
        SymbolKind::LocalVariable => return None,
    })
}

#[derive(Debug, Clone)]
struct CachedIndexSummary {
    files: usize,
    bytes: usize,
    indexed_symbols: usize,
    parse_diagnostics: usize,
    lossy_files: usize,
}

impl From<&RuntimeIndexSummary> for CachedIndexSummary {
    fn from(summary: &RuntimeIndexSummary) -> Self {
        Self {
            files: summary.files,
            bytes: summary.bytes,
            indexed_symbols: summary.indexed_symbols,
            parse_diagnostics: summary.parse_diagnostics,
            lossy_files: summary.lossy_files,
        }
    }
}

impl From<CachedIndexSummary> for RuntimeIndexSummary {
    fn from(summary: CachedIndexSummary) -> Self {
        Self {
            files: summary.files,
            bytes: summary.bytes,
            indexed_symbols: summary.indexed_symbols,
            parse_diagnostics: summary.parse_diagnostics,
            lossy_files: summary.lossy_files,
        }
    }
}

pub fn load_or_build_game_data_index(
    config: &GameDataIndexCacheConfig,
) -> Result<GameDataIndexCacheResult, String> {
    load_or_build_game_data_index_with_progress(config, |_| {})
}

pub fn load_or_build_game_data_index_with_progress(
    config: &GameDataIndexCacheConfig,
    mut progress: impl FnMut(&str),
) -> Result<GameDataIndexCacheResult, String> {
    let total_start = Instant::now();
    let mut timings = IndexCacheTimings::default();
    progress("validate-scripts-root-start");
    if !config.scripts_root.is_dir() {
        return Err(format!(
            "Game-data scripts folder does not exist: {}",
            config.scripts_root.display()
        ));
    }
    progress("validate-scripts-root-end");

    progress("fingerprint-start");
    let fingerprint_start = Instant::now();
    let fingerprint = source_fingerprint(&config.scripts_root, config.metadata_path.as_deref())?;
    timings.fingerprint = fingerprint_start.elapsed();
    progress("fingerprint-end");
    let initial_cache_file_bytes = cache_file_bytes(&config.cache_path);

    progress("cache-load-start");
    let cache_read_start = Instant::now();
    match load_cached_index(&config.cache_path, &fingerprint, &mut timings) {
        Ok(Some(CacheLoad::Current(cached))) => {
            timings.cache_read_deserialize_validate = cache_read_start.elapsed();
            progress("cache-load-hit");
            progress("map-rebuild-start");
            let map_rebuild_start = Instant::now();
            let summary: RuntimeIndexSummary = cached.summary.clone().into();
            let index = cached.into_index();
            timings.map_rebuild = map_rebuild_start.elapsed();
            progress("map-rebuild-end");
            timings.total = total_start.elapsed();
            return Ok(GameDataIndexCacheResult {
                index,
                summary,
                cache_status: IndexCacheStatus::Loaded,
                fingerprint,
                timings,
                cache_file_bytes: initial_cache_file_bytes,
            });
        }
        Ok(Some(CacheLoad::Migrated(cached))) => {
            timings.cache_read_deserialize_validate = cache_read_start.elapsed();
            progress("cache-load-hit");
            let summary: RuntimeIndexSummary = cached.summary.clone().into();

            // Legacy bytes have already passed source-identity and structural
            // validation. Replace them before reconstructing lookup maps so no
            // legacy graph survives publication (or coexists with the output).
            progress("cache-write-start");
            let cache_write_start = Instant::now();
            let migration_write = write_cached_payload(&config.cache_path, &cached);
            timings.cache_write = cache_write_start.elapsed();
            if migration_write.is_ok() {
                progress("cache-write-end");
                progress("map-rebuild-start");
                let map_rebuild_start = Instant::now();
                let index = cached.into_index();
                timings.map_rebuild = map_rebuild_start.elapsed();
                progress("map-rebuild-end");
                timings.total = total_start.elapsed();
                return Ok(GameDataIndexCacheResult {
                    index,
                    summary,
                    cache_status: IndexCacheStatus::Loaded,
                    fingerprint,
                    timings,
                    cache_file_bytes: cache_file_bytes(&config.cache_path),
                });
            }
            // A cache migration is never a reason to publish the old graph or
            // fail the language server. Discard it and take the normal source
            // rebuild path below; a later write may succeed after a transient
            // filesystem error is gone.
            progress("cache-write-failed");
        }
        Ok(None) | Err(_) => {
            timings.cache_read_deserialize_validate = cache_read_start.elapsed();
            progress("cache-load-miss");
        }
    }

    let rebuild_reason = cache_rebuild_reason(&config.cache_path, &fingerprint);
    progress("source-rebuild-start");
    let rebuild_start = Instant::now();
    let built = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &config.scripts_root,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })?;
    timings.rebuild = rebuild_start.elapsed();
    progress("source-rebuild-end");
    let cached_index = built.index.compact_for_runtime_cache();
    let summary = summary_from_build_with_cached_index(&built, &cached_index);

    progress("cache-write-start");
    let cache_write_start = Instant::now();
    write_cached_index(&config.cache_path, &fingerprint, &summary, &cached_index)?;
    timings.cache_write = cache_write_start.elapsed();
    progress("cache-write-end");
    timings.total = total_start.elapsed();
    let cache_file_bytes = cache_file_bytes(&config.cache_path);

    Ok(GameDataIndexCacheResult {
        index: cached_index,
        summary,
        cache_status: IndexCacheStatus::Rebuilt {
            reason: rebuild_reason,
        },
        fingerprint,
        timings,
        cache_file_bytes,
    })
}

fn cache_file_bytes(cache_path: &Path) -> Option<u64> {
    cache_path.metadata().ok().map(|metadata| metadata.len())
}

enum CacheLoad {
    Current(CachedGameDataIndex),
    Migrated(CachedGameDataIndex),
}

fn load_cached_index(
    cache_path: &Path,
    expected_fingerprint: &SourceFingerprint,
    timings: &mut IndexCacheTimings,
) -> Result<Option<CacheLoad>, String> {
    if !cache_path.is_file() {
        return Ok(None);
    }

    let read_start = Instant::now();
    let mut file = fs::File::open(cache_path).map_err(|error| {
        format!(
            "Failed to open index cache {}: {error}",
            cache_path.display()
        )
    })?;
    let file_bytes = file
        .metadata()
        .map_err(|error| {
            format!(
                "Failed to stat index cache {}: {error}",
                cache_path.display()
            )
        })?
        .len();
    let mut magic = [0_u8; CACHE_MAGIC.len()];
    file.read_exact(&mut magic).map_err(|error| {
        format!(
            "Failed to read index cache magic {}: {error}",
            cache_path.display()
        )
    })?;
    if (magic == *V10_CACHE_MAGIC || magic == *LEGACY_CACHE_MAGIC)
        && file_bytes > MAX_LEGACY_CACHE_BYTES
    {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&magic);
    file.read_to_end(&mut bytes).map_err(|error| {
        format!(
            "Failed to read index cache {}: {error}",
            cache_path.display()
        )
    })?;
    timings.cache_file_read = read_start.elapsed();
    let decode_start = Instant::now();
    let load = if magic == *CACHE_MAGIC {
        let cached = decode_cached_index(&bytes).map_err(|error| {
            format!(
                "Failed to decode index cache {}: {error}",
                cache_path.display()
            )
        })?;
        drop(bytes);
        CacheLoad::Current(cached)
    } else if magic == *V10_CACHE_MAGIC {
        let v10 = decode_v10_cached_index(&bytes).map_err(|error| {
            format!(
                "Failed to decode v10 index cache {}: {error}",
                cache_path.display()
            )
        })?;
        // The binary payload is no longer needed once its owned v10 graph is
        // decoded. Do not keep it while validating/projecting the v11 facts.
        drop(bytes);
        if !v10.validates_for_migration(expected_fingerprint) {
            timings.cache_decode = decode_start.elapsed();
            timings.cache_validate = timings.cache_decode;
            return Ok(None);
        }
        CacheLoad::Migrated(v10.into_current().map_err(|error| {
            format!(
                "Failed to project v10 index cache {} into canonical facts: {error}",
                cache_path.display()
            )
        })?)
    } else if magic == *LEGACY_CACHE_MAGIC {
        let legacy = decode_legacy_cached_index(&bytes).map_err(|error| {
            format!(
                "Failed to decode legacy index cache {}: {error}",
                cache_path.display()
            )
        })?;
        drop(bytes);
        if !legacy.validates_for_migration(expected_fingerprint) {
            timings.cache_decode = decode_start.elapsed();
            timings.cache_validate = timings.cache_decode;
            return Ok(None);
        }
        CacheLoad::Migrated(legacy.into_current())
    } else {
        return Err(format!(
            "Failed to decode index cache {}: binary cache magic mismatch",
            cache_path.display()
        ));
    };
    timings.cache_decode = decode_start.elapsed();

    let validate_start = Instant::now();
    let cached = match &load {
        CacheLoad::Current(cached) | CacheLoad::Migrated(cached) => cached,
    };
    if cached.schema != CACHE_SCHEMA
        || cached.format_version != CACHE_FORMAT_VERSION
        || cached.index_shape != CACHE_INDEX_SHAPE
        || cached.crate_version != env!("CARGO_PKG_VERSION")
        || cached.fingerprint != *expected_fingerprint
        || cached.validate().is_err()
    {
        timings.cache_validate = validate_start.elapsed();
        return Ok(None);
    }
    timings.cache_validate = validate_start.elapsed();

    Ok(Some(load))
}

fn write_cached_index(
    cache_path: &Path,
    fingerprint: &SourceFingerprint,
    summary: &RuntimeIndexSummary,
    index: &SymbolIndex,
) -> Result<(), String> {
    let cached = CachedGameDataIndex::from_index(
        index,
        fingerprint.clone(),
        CachedIndexSummary::from(summary),
    );
    write_cached_payload(cache_path, &cached)
}

fn write_cached_payload(cache_path: &Path, cached: &CachedGameDataIndex) -> Result<(), String> {
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create index cache folder {}: {error}",
                parent.display()
            )
        })?;
    }

    let temp_path = unique_cache_temp_path(cache_path);
    let result = (|| {
        let file = fs::File::create(&temp_path).map_err(|error| {
            format!(
                "Failed to create temporary index cache {}: {error}",
                temp_path.display()
            )
        })?;
        let bytes = encode_cached_index(cached)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&bytes).map_err(|error| {
            format!(
                "Failed to write index cache {}: {error}",
                temp_path.display()
            )
        })?;
        writer.flush().map_err(|error| {
            format!(
                "Failed to flush index cache {}: {error}",
                temp_path.display()
            )
        })?;
        // Windows `ReplaceFileW` requires the replacement handle to be closed.
        // Flushing alone leaves the `BufWriter` (and its file) open.
        drop(writer);
        replace_cache_atomically(&temp_path, cache_path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(not(windows))]
fn replace_cache_atomically(temp_path: &Path, cache_path: &Path) -> Result<(), String> {
    fs::rename(temp_path, cache_path).map_err(|error| {
        format!(
            "Failed to replace index cache {} with {}: {error}",
            cache_path.display(),
            temp_path.display()
        )
    })
}

#[cfg(windows)]
fn replace_cache_atomically(temp_path: &Path, cache_path: &Path) -> Result<(), String> {
    if !cache_path.exists() {
        return fs::rename(temp_path, cache_path).map_err(|error| {
            format!(
                "Failed to install index cache {} from {}: {error}",
                cache_path.display(),
                temp_path.display()
            )
        });
    }

    use std::os::windows::ffi::OsStrExt;
    let replaced: Vec<u16> = cache_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let replacement: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let replaced = unsafe {
        replace_file_w(
            replaced.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        return Err(format!(
            "Failed to atomically replace index cache {} with {}: {}",
            cache_path.display(),
            temp_path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(windows)]
unsafe fn replace_file_w(
    replaced: *const u16,
    replacement: *const u16,
    backup: *const u16,
    flags: u32,
    exclude: *mut std::ffi::c_void,
    reserved: *mut std::ffi::c_void,
) -> i32 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }
    unsafe { ReplaceFileW(replaced, replacement, backup, flags, exclude, reserved) }
}

fn unique_cache_temp_path(cache_path: &Path) -> PathBuf {
    let nonce = UNIX_EPOCH
        .elapsed()
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let file_name = cache_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "game-data-symbol-index".into());
    cache_path.with_file_name(format!("{file_name}.{}.{}.tmp", std::process::id(), nonce))
}

fn encode_cached_index(cached: &CachedGameDataIndex) -> Result<Vec<u8>, String> {
    let string_table = CacheStringTable::from_cached_index(cached)?;
    let mut writer = BinaryWriter::new(string_table);
    writer.write_bytes(CACHE_MAGIC);
    writer.write_string_table()?;
    writer.write_string(&cached.schema)?;
    writer.write_u32(cached.format_version);
    writer.write_string(&cached.index_shape)?;
    writer.write_string(&cached.crate_version)?;
    writer.write_fingerprint(&cached.fingerprint)?;
    writer.write_summary(&cached.summary);
    writer.write_vec_len(cached.files.len())?;
    for file in &cached.files {
        writer.write_cached_file_contribution(file)?;
    }
    Ok(writer.into_bytes())
}

#[cfg(test)]
fn encode_legacy_cached_index(cached: &LegacyCachedGameDataIndex) -> Result<Vec<u8>, String> {
    let runtime =
        SymbolIndex::from_indexed_parts(cached.index.files.clone(), cached.index.symbols.clone());
    let string_table = CacheStringTable::from_legacy(cached, &runtime)?;
    let mut writer = BinaryWriter::new(string_table);
    writer.write_bytes(LEGACY_CACHE_MAGIC);
    writer.write_string_table()?;
    writer.write_string(&cached.schema)?;
    writer.write_u32(cached.format_version);
    writer.write_string(&cached.index_shape)?;
    writer.write_string(&cached.crate_version)?;
    writer.write_fingerprint(&cached.fingerprint)?;
    writer.write_summary(&cached.summary);
    writer.write_vec_len(cached.index.files.len())?;
    for file in &cached.index.files {
        writer.write_indexed_file(file)?;
    }
    writer.write_vec_len(cached.index.symbols.len())?;
    for symbol in &cached.index.symbols {
        writer.write_indexed_symbol(symbol)?;
    }
    Ok(writer.into_bytes())
}

#[cfg(test)]
fn encode_v10_cached_index(cached: &V10CachedGameDataIndex) -> Result<Vec<u8>, String> {
    let runtime =
        SymbolIndex::from_indexed_parts(cached.index.files.clone(), cached.index.symbols.clone());
    let table_source = LegacyCachedGameDataIndex {
        schema: cached.schema.clone(),
        format_version: cached.format_version,
        index_shape: cached.index_shape.clone(),
        crate_version: cached.crate_version.clone(),
        fingerprint: cached.fingerprint.clone(),
        summary: cached.summary.clone(),
        index: LegacyCachedSymbolIndex {
            files: runtime.files().to_vec(),
            symbols: runtime.symbols().to_vec(),
        },
    };
    let string_table = CacheStringTable::from_legacy(&table_source, &runtime)?;
    let mut writer = BinaryWriter::new(string_table);
    writer.write_bytes(V10_CACHE_MAGIC);
    writer.write_string_table()?;
    writer.write_string(&cached.schema)?;
    writer.write_u32(cached.format_version);
    writer.write_string(&cached.index_shape)?;
    writer.write_string(&cached.crate_version)?;
    writer.write_fingerprint(&cached.fingerprint)?;
    writer.write_summary(&cached.summary);
    writer.write_vec_len(cached.index.files.len())?;
    for file in &cached.index.files {
        writer.write_indexed_file(file)?;
    }
    writer.write_vec_len(cached.index.symbols.len())?;
    for symbol in &cached.index.symbols {
        writer.write_indexed_symbol(symbol)?;
    }
    let contribution_bytes = serde_json::to_vec(&cached.index.contributions)
        .map_err(|error| format!("Failed to encode v10 file contributions: {error}"))?;
    writer.write_vec_len(contribution_bytes.len())?;
    writer.write_bytes(&contribution_bytes);
    Ok(writer.into_bytes())
}

fn decode_cached_index(bytes: &[u8]) -> Result<CachedGameDataIndex, String> {
    let mut reader = BinaryReader::new(bytes);
    let magic = reader.read_exact(CACHE_MAGIC.len())?;
    if magic != &CACHE_MAGIC[..] {
        return Err("binary cache magic mismatch".to_string());
    }
    reader.read_string_table()?;
    let schema = reader.read_string()?;
    let format_version = reader.read_u32()?;
    let index_shape = reader.read_string()?;
    let crate_version = reader.read_string()?;
    let fingerprint = reader.read_fingerprint()?;
    let summary = reader.read_summary()?;
    let file_count = reader.read_bounded_len("file records", MAX_CACHE_FILE_RECORDS)?;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        files.push(reader.read_cached_file_contribution()?);
    }
    reader.expect_eof()?;
    Ok(CachedGameDataIndex {
        schema,
        format_version,
        index_shape,
        crate_version,
        fingerprint,
        summary,
        files,
    })
}

fn decode_v10_cached_index(bytes: &[u8]) -> Result<V10CachedGameDataIndex, String> {
    let mut reader = BinaryReader::new(bytes);
    let magic = reader.read_exact(V10_CACHE_MAGIC.len())?;
    if magic != &V10_CACHE_MAGIC[..] {
        return Err("v10 binary cache magic mismatch".to_string());
    }
    reader.read_string_table()?;
    let schema = reader.read_string()?;
    let format_version = reader.read_u32()?;
    let index_shape = reader.read_string()?;
    let crate_version = reader.read_string()?;
    let fingerprint = reader.read_fingerprint()?;
    let summary = reader.read_summary()?;
    let file_count = reader.read_bounded_len("file records", MAX_CACHE_FILE_RECORDS)?;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        files.push(reader.read_indexed_file()?);
    }
    let symbol_count = reader.read_bounded_len("symbol records", MAX_CACHE_SYMBOL_RECORDS)?;
    let mut symbols = Vec::with_capacity(symbol_count);
    for _ in 0..symbol_count {
        symbols.push(reader.read_indexed_symbol()?);
    }
    let contribution_len = reader.read_bounded_len(
        "v10 public contribution bytes",
        usize::try_from(MAX_LEGACY_CACHE_BYTES)
            .map_err(|_| "legacy cache byte ceiling exceeds usize".to_string())?,
    )?;
    let contributions = serde_json::from_slice(reader.read_exact(contribution_len)?)
        .map_err(|error| format!("invalid v10 public file contributions: {error}"))?;
    reader.expect_eof()?;
    Ok(V10CachedGameDataIndex {
        schema,
        format_version,
        index_shape,
        crate_version,
        fingerprint,
        summary,
        index: V10CachedSymbolIndex {
            files,
            symbols,
            contributions,
        },
    })
}

fn decode_legacy_cached_index(bytes: &[u8]) -> Result<LegacyCachedGameDataIndex, String> {
    let mut reader = BinaryReader::new(bytes);
    let magic = reader.read_exact(LEGACY_CACHE_MAGIC.len())?;
    if magic != &LEGACY_CACHE_MAGIC[..] {
        return Err("legacy binary cache magic mismatch".to_string());
    }
    reader.read_string_table()?;
    let schema = reader.read_string()?;
    let format_version = reader.read_u32()?;
    let index_shape = reader.read_string()?;
    let crate_version = reader.read_string()?;
    let fingerprint = reader.read_fingerprint()?;
    let summary = reader.read_summary()?;
    let file_count = reader.read_bounded_len("file records", MAX_CACHE_FILE_RECORDS)?;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        files.push(reader.read_indexed_file()?);
    }
    let symbol_count = reader.read_bounded_len("symbol records", MAX_CACHE_SYMBOL_RECORDS)?;
    let mut symbols = Vec::with_capacity(symbol_count);
    for _ in 0..symbol_count {
        symbols.push(reader.read_indexed_symbol()?);
    }
    reader.expect_eof()?;
    Ok(LegacyCachedGameDataIndex {
        schema,
        format_version,
        index_shape,
        crate_version,
        fingerprint,
        summary,
        index: LegacyCachedSymbolIndex { files, symbols },
    })
}

struct CacheStringTable {
    ids: BTreeMap<String, u32>,
    values: Vec<String>,
}

impl CacheStringTable {
    fn from_cached_index(cached: &CachedGameDataIndex) -> Result<Self, String> {
        let mut table = Self {
            ids: BTreeMap::new(),
            values: Vec::new(),
        };
        table.insert(&cached.schema)?;
        table.insert(&cached.index_shape)?;
        table.insert(&cached.crate_version)?;
        table.insert_fingerprint(&cached.fingerprint)?;
        for file in &cached.files {
            table.insert_cached_file(file)?;
        }
        Ok(table)
    }

    #[cfg(test)]
    fn from_legacy(
        cached: &LegacyCachedGameDataIndex,
        runtime: &SymbolIndex,
    ) -> Result<Self, String> {
        let mut table = Self {
            ids: BTreeMap::new(),
            values: Vec::new(),
        };
        table.insert(&cached.schema)?;
        table.insert(&cached.index_shape)?;
        table.insert(&cached.crate_version)?;
        table.insert_fingerprint(&cached.fingerprint)?;
        for file in runtime.files() {
            table.insert_metadata(&file.metadata)?;
        }
        for symbol in runtime.symbols() {
            table.insert_symbol(symbol)?;
        }
        Ok(table)
    }

    fn insert(&mut self, value: &str) -> Result<(), String> {
        if self.ids.contains_key(value) {
            return Ok(());
        }
        let id = u32::try_from(self.values.len())
            .map_err(|_| "cache string table exceeds u32 entries".to_string())?;
        self.values.push(value.to_string());
        self.ids.insert(value.to_string(), id);
        Ok(())
    }

    fn insert_option(&mut self, value: Option<&str>) -> Result<(), String> {
        if let Some(value) = value {
            self.insert(value)?;
        }
        Ok(())
    }

    fn insert_path(&mut self, value: &Path) -> Result<(), String> {
        self.insert(&value.to_string_lossy())
    }

    fn insert_option_path(&mut self, value: Option<&Path>) -> Result<(), String> {
        if let Some(value) = value {
            self.insert_path(value)?;
        }
        Ok(())
    }

    fn insert_fingerprint(&mut self, fingerprint: &SourceFingerprint) -> Result<(), String> {
        match fingerprint {
            SourceFingerprint::Downloaded {
                scripts_root,
                commit_sha,
            } => {
                self.insert(scripts_root)?;
                self.insert(commit_sha)?;
            }
            SourceFingerprint::Manual { scripts_root, .. } => {
                self.insert(scripts_root)?;
            }
        }
        Ok(())
    }

    fn insert_metadata(&mut self, metadata: &SourceFileMetadata) -> Result<(), String> {
        self.insert_option_path(metadata.absolute_path.as_deref())?;
        self.insert_option_path(metadata.root_path.as_deref())?;
        self.insert_option_path(metadata.relative_path.as_deref())
    }

    #[cfg(test)]
    fn insert_symbol(&mut self, symbol: &IndexedSymbol) -> Result<(), String> {
        self.insert_option(symbol.name.as_deref())?;
        self.insert_detail(&symbol.detail)?;
        for attribute in &symbol.attributes {
            self.insert_option(attribute.name.as_deref())?;
            self.insert(&attribute.text)?;
        }
        for modifier in &symbol.modifiers {
            self.insert(modifier)?;
        }
        for doc_comment in &symbol.doc_comments {
            self.insert(&doc_comment.text)?;
        }
        for branch in &symbol.conditional_context {
            self.insert_option(branch.condition.as_deref())?;
        }
        Ok(())
    }

    fn insert_cached_file(&mut self, file: &CachedFileContribution) -> Result<(), String> {
        self.insert_metadata(&file.metadata)?;
        for symbol in &file.symbols {
            self.insert(&symbol.name)?;
            self.insert_option(symbol.detail.type_text.as_deref())?;
            self.insert_option(symbol.detail.return_type.as_deref())?;
            self.insert_option(symbol.detail.base_type.as_deref())?;
            self.insert_option(symbol.detail.default_value.as_deref())?;
            self.insert_option(symbol.detail.enum_value.as_deref())?;
            for attribute in &symbol.attributes {
                self.insert(attribute)?;
            }
            for modifier in &symbol.modifiers {
                self.insert(modifier)?;
            }
            for comment in &symbol.doc_comments {
                self.insert(&comment.text)?;
            }
            for branch in &symbol.conditional_context {
                self.insert_option(branch.condition.as_deref())?;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn insert_detail(&mut self, detail: &IndexedSymbolDetail) -> Result<(), String> {
        self.insert_option(detail.type_text.as_deref())?;
        self.insert_option(detail.return_type_text.as_deref())?;
        self.insert_option(detail.base_type.as_deref())?;
        self.insert_option(detail.default_text.as_deref())?;
        self.insert_option(detail.enum_value_text.as_deref())
    }

    fn id(&self, value: &str) -> Result<u32, String> {
        self.ids
            .get(value)
            .copied()
            .ok_or_else(|| format!("cache string was not interned before write: {value:?}"))
    }
}

struct BinaryWriter {
    bytes: Vec<u8>,
    string_table: CacheStringTable,
}

impl BinaryWriter {
    fn new(string_table: CacheStringTable) -> Self {
        Self {
            bytes: Vec::new(),
            string_table,
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_usize(&mut self, value: usize) -> Result<(), String> {
        let value = u64::try_from(value).map_err(|_| "usize value exceeds u64".to_string())?;
        self.write_u64(value);
        Ok(())
    }

    fn write_vec_len(&mut self, len: usize) -> Result<(), String> {
        self.write_usize(len)
    }

    fn write_string_table(&mut self) -> Result<(), String> {
        self.write_vec_len(self.string_table.values.len())?;
        // `values` is already the deterministic insertion order represented
        // by the table IDs. Append directly into the output buffer instead of
        // cloning every interned string for the write pass.
        let bytes = &mut self.bytes;
        for value in &self.string_table.values {
            let value = value.as_bytes();
            let len =
                u64::try_from(value.len()).map_err(|_| "usize value exceeds u64".to_string())?;
            bytes.extend_from_slice(&len.to_le_bytes());
            bytes.extend_from_slice(value);
        }
        Ok(())
    }

    fn write_string(&mut self, value: &str) -> Result<(), String> {
        let id = self.string_table.id(value)?;
        self.write_u32(id);
        Ok(())
    }

    fn write_path(&mut self, value: &Path) -> Result<(), String> {
        self.write_string(&value.to_string_lossy())
    }

    fn write_option_string(&mut self, value: Option<&str>) -> Result<(), String> {
        match value {
            Some(value) => {
                self.write_u8(1);
                self.write_string(value)?;
            }
            None => self.write_u8(0),
        }
        Ok(())
    }

    fn write_option_path(&mut self, value: Option<&Path>) -> Result<(), String> {
        match value {
            Some(value) => {
                self.write_u8(1);
                self.write_path(value)?;
            }
            None => self.write_u8(0),
        }
        Ok(())
    }

    #[cfg(test)]
    fn write_option_span(&mut self, value: Option<TextSpan>) -> Result<(), String> {
        match value {
            Some(value) => {
                self.write_u8(1);
                self.write_span(value)?;
            }
            None => self.write_u8(0),
        }
        Ok(())
    }

    fn write_span(&mut self, span: TextSpan) -> Result<(), String> {
        self.write_usize(span.start)?;
        self.write_usize(span.end)
    }

    #[cfg(test)]
    fn write_global_id(&mut self, id: GlobalSymbolId) -> Result<(), String> {
        self.write_usize(id.file_id.0)?;
        self.write_usize(id.symbol_id.0)
    }

    #[cfg(test)]
    fn write_option_global_id(&mut self, id: Option<GlobalSymbolId>) -> Result<(), String> {
        match id {
            Some(id) => {
                self.write_u8(1);
                self.write_global_id(id)?;
            }
            None => self.write_u8(0),
        }
        Ok(())
    }

    fn write_fingerprint(&mut self, fingerprint: &SourceFingerprint) -> Result<(), String> {
        match fingerprint {
            SourceFingerprint::Downloaded {
                scripts_root,
                commit_sha,
            } => {
                self.write_u8(0);
                self.write_string(scripts_root)?;
                self.write_string(commit_sha)?;
            }
            SourceFingerprint::Manual {
                scripts_root,
                file_count,
                byte_count,
                latest_modified_unix_ms,
            } => {
                self.write_u8(1);
                self.write_string(scripts_root)?;
                self.write_usize(*file_count)?;
                self.write_u64(*byte_count);
                self.write_u128(*latest_modified_unix_ms);
            }
        }
        Ok(())
    }

    fn write_summary(&mut self, summary: &CachedIndexSummary) {
        self.write_u64(summary.files as u64);
        self.write_u64(summary.bytes as u64);
        self.write_u64(summary.indexed_symbols as u64);
        self.write_u64(summary.parse_diagnostics as u64);
        self.write_u64(summary.lossy_files as u64);
    }

    fn write_metadata(&mut self, metadata: &SourceFileMetadata) -> Result<(), String> {
        self.write_u8(source_kind_tag(metadata.kind));
        self.write_u8(source_category_tag(metadata.category));
        self.write_option_path(metadata.absolute_path.as_deref())?;
        self.write_option_path(metadata.root_path.as_deref())?;
        self.write_option_path(metadata.relative_path.as_deref())?;
        self.write_u16(metadata.priority);
        Ok(())
    }

    #[cfg(test)]
    fn write_indexed_file(&mut self, file: &IndexedFile) -> Result<(), String> {
        self.write_usize(file.id.0)?;
        self.write_metadata(&file.metadata)?;
        self.write_usize(file.symbol_start)?;
        self.write_usize(file.symbol_count)?;
        self.write_usize(file.non_declaration_callable_fragments)
    }

    #[cfg(test)]
    fn write_detail(&mut self, detail: &IndexedSymbolDetail) -> Result<(), String> {
        self.write_option_string(detail.type_text.as_deref())?;
        self.write_option_span(detail.type_text_span)?;
        self.write_option_string(detail.return_type_text.as_deref())?;
        self.write_option_span(detail.return_type_text_span)?;
        self.write_option_string(detail.base_type.as_deref())?;
        self.write_option_span(detail.base_type_span)?;
        self.write_option_string(detail.default_text.as_deref())?;
        self.write_option_span(detail.default_text_span)?;
        self.write_option_string(detail.enum_value_text.as_deref())?;
        self.write_option_span(detail.enum_value_text_span)
    }

    #[cfg(test)]
    fn write_indexed_symbol(&mut self, symbol: &IndexedSymbol) -> Result<(), String> {
        self.write_global_id(symbol.id)?;
        self.write_option_global_id(symbol.parent)?;
        self.write_u8(symbol_kind_tag(symbol.kind));
        self.write_option_string(symbol.name.as_deref())?;
        self.write_span(symbol.span)?;
        self.write_span(symbol.selection_span)?;
        self.write_detail(&symbol.detail)?;
        self.write_vec_len(symbol.attributes.len())?;
        for attribute in &symbol.attributes {
            self.write_attribute(attribute)?;
        }
        self.write_vec_len(symbol.modifiers.len())?;
        for modifier in &symbol.modifiers {
            self.write_string(modifier)?;
        }
        self.write_vec_len(symbol.doc_comments.len())?;
        for doc_comment in &symbol.doc_comments {
            self.write_doc_comment(doc_comment)?;
        }
        self.write_vec_len(symbol.conditional_context.len())?;
        for branch in &symbol.conditional_context {
            self.write_conditional_branch(branch)?;
        }
        match symbol.callable_form {
            Some(form) => {
                self.write_u8(1);
                self.write_u8(callable_form_tag(form));
            }
            None => self.write_u8(0),
        }
        Ok(())
    }

    #[cfg(test)]
    fn write_attribute(&mut self, attribute: &IndexedAttribute) -> Result<(), String> {
        self.write_option_string(attribute.name.as_deref())?;
        self.write_string(&attribute.text)
    }

    #[cfg(test)]
    fn write_doc_comment(&mut self, comment: &IndexedDocComment) -> Result<(), String> {
        self.write_u8(doc_comment_kind_tag(comment.kind));
        self.write_string(&comment.text)
    }

    #[cfg(test)]
    fn write_conditional_branch(
        &mut self,
        branch: &IndexedConditionalBranch,
    ) -> Result<(), String> {
        self.write_u8(preprocessor_branch_kind_tag(branch.kind));
        self.write_option_string(branch.condition.as_deref())
    }

    fn write_cached_file_contribution(
        &mut self,
        file: &CachedFileContribution,
    ) -> Result<(), String> {
        self.write_metadata(&file.metadata)?;
        self.write_usize(file.non_declaration_callable_fragments)?;
        self.write_vec_len(file.symbols.len())?;
        for symbol in &file.symbols {
            self.write_cached_public_symbol(symbol)?;
        }
        Ok(())
    }

    fn write_cached_public_symbol(&mut self, symbol: &CachedPublicSymbol) -> Result<(), String> {
        self.write_u32(symbol.id.0);
        match symbol.parent {
            Some(parent) => {
                self.write_u8(1);
                self.write_u32(parent.0);
            }
            None => self.write_u8(0),
        }
        self.write_u8(semantic_declaration_kind_tag(symbol.kind));
        self.write_string(&symbol.name)?;
        self.write_span(symbol.span)?;
        self.write_span(symbol.selection_span)?;
        self.write_option_string(symbol.detail.type_text.as_deref())?;
        self.write_option_string(symbol.detail.return_type.as_deref())?;
        self.write_option_string(symbol.detail.base_type.as_deref())?;
        self.write_option_string(symbol.detail.default_value.as_deref())?;
        self.write_option_string(symbol.detail.enum_value.as_deref())?;
        self.write_vec_len(symbol.attributes.len())?;
        for attribute in &symbol.attributes {
            self.write_string(attribute)?;
        }
        self.write_vec_len(symbol.modifiers.len())?;
        for modifier in &symbol.modifiers {
            self.write_string(modifier)?;
        }
        self.write_vec_len(symbol.doc_comments.len())?;
        for comment in &symbol.doc_comments {
            self.write_u8(semantic_doc_comment_kind_tag(comment.kind));
            self.write_string(&comment.text)?;
        }
        self.write_vec_len(symbol.conditional_context.len())?;
        for branch in &symbol.conditional_context {
            self.write_u8(semantic_conditional_branch_kind_tag(branch.kind));
            self.write_option_string(branch.condition.as_deref())?;
        }
        match symbol.callable_form {
            Some(form) => {
                self.write_u8(1);
                self.write_u8(semantic_callable_form_tag(form));
            }
            None => self.write_u8(0),
        }
        Ok(())
    }
}

struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    string_table: Vec<String>,
}

impl<'a> BinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            string_table: Vec::new(),
        }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "cache offset overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("unexpected end of cache file".to_string());
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let bytes: [u8; 2] = self
            .read_exact(2)?
            .try_into()
            .map_err(|_| "invalid u16 bytes".to_string())?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes: [u8; 4] = self
            .read_exact(4)?
            .try_into()
            .map_err(|_| "invalid u32 bytes".to_string())?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let bytes: [u8; 8] = self
            .read_exact(8)?
            .try_into()
            .map_err(|_| "invalid u64 bytes".to_string())?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_u128(&mut self) -> Result<u128, String> {
        let bytes: [u8; 16] = self
            .read_exact(16)?
            .try_into()
            .map_err(|_| "invalid u128 bytes".to_string())?;
        Ok(u128::from_le_bytes(bytes))
    }

    fn read_usize(&mut self) -> Result<usize, String> {
        usize::try_from(self.read_u64()?).map_err(|_| "u64 value exceeds usize".to_string())
    }

    fn read_len(&mut self) -> Result<usize, String> {
        self.read_usize()
    }

    fn read_bounded_len(&mut self, label: &str, max: usize) -> Result<usize, String> {
        let len = self.read_len()?;
        if len > max {
            return Err(format!(
                "cache {label} length {len} exceeds safety limit {max}"
            ));
        }
        Ok(len)
    }

    fn read_raw_string(&mut self) -> Result<String, String> {
        let len = self.read_bounded_len("raw string byte", MAX_CACHE_RAW_STRING_BYTES)?;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|error| format!("invalid utf-8 string: {error}"))
    }

    fn read_string_table(&mut self) -> Result<(), String> {
        let len = self.read_bounded_len("string table entry", MAX_CACHE_STRING_TABLE_ENTRIES)?;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.read_raw_string()?);
        }
        self.string_table = values;
        Ok(())
    }

    fn read_string(&mut self) -> Result<String, String> {
        let id = usize::try_from(self.read_u32()?)
            .map_err(|_| "string table id exceeds usize".to_string())?;
        self.string_table
            .get(id)
            .cloned()
            .ok_or_else(|| format!("invalid string table id {id}"))
    }

    fn read_option_string(&mut self) -> Result<Option<String>, String> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_string().map(Some),
            tag => Err(format!("invalid option string tag {tag}")),
        }
    }

    fn read_option_path(&mut self) -> Result<Option<PathBuf>, String> {
        Ok(self.read_option_string()?.map(PathBuf::from))
    }

    fn read_span(&mut self) -> Result<TextSpan, String> {
        let start = self.read_usize()?;
        let end = self.read_usize()?;
        Ok(TextSpan { start, end })
    }

    fn read_option_span(&mut self) -> Result<Option<TextSpan>, String> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_span().map(Some),
            tag => Err(format!("invalid option span tag {tag}")),
        }
    }

    fn read_global_id(&mut self) -> Result<GlobalSymbolId, String> {
        Ok(GlobalSymbolId {
            file_id: SourceFileId(self.read_usize()?),
            symbol_id: SymbolId(self.read_usize()?),
        })
    }

    fn read_option_global_id(&mut self) -> Result<Option<GlobalSymbolId>, String> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_global_id().map(Some),
            tag => Err(format!("invalid option global id tag {tag}")),
        }
    }

    fn read_fingerprint(&mut self) -> Result<SourceFingerprint, String> {
        match self.read_u8()? {
            0 => Ok(SourceFingerprint::Downloaded {
                scripts_root: self.read_string()?,
                commit_sha: self.read_string()?,
            }),
            1 => Ok(SourceFingerprint::Manual {
                scripts_root: self.read_string()?,
                file_count: self.read_usize()?,
                byte_count: self.read_u64()?,
                latest_modified_unix_ms: self.read_u128()?,
            }),
            tag => Err(format!("invalid fingerprint tag {tag}")),
        }
    }

    fn read_summary(&mut self) -> Result<CachedIndexSummary, String> {
        Ok(CachedIndexSummary {
            files: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary files exceeds usize".to_string())?,
            bytes: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary bytes exceeds usize".to_string())?,
            indexed_symbols: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary symbols exceeds usize".to_string())?,
            parse_diagnostics: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary diagnostics exceeds usize".to_string())?,
            lossy_files: usize::try_from(self.read_u64()?)
                .map_err(|_| "summary lossy files exceeds usize".to_string())?,
        })
    }

    fn read_metadata(&mut self) -> Result<SourceFileMetadata, String> {
        Ok(SourceFileMetadata {
            kind: source_kind_from_tag(self.read_u8()?)?,
            category: source_category_from_tag(self.read_u8()?)?,
            absolute_path: self.read_option_path()?,
            root_path: self.read_option_path()?,
            relative_path: self.read_option_path()?,
            priority: self.read_u16()?,
        })
    }

    fn read_indexed_file(&mut self) -> Result<IndexedFile, String> {
        Ok(IndexedFile {
            id: SourceFileId(self.read_usize()?),
            metadata: self.read_metadata()?,
            symbol_start: self.read_usize()?,
            symbol_count: self.read_usize()?,
            non_declaration_callable_fragments: self.read_usize()?,
        })
    }

    fn read_detail(&mut self) -> Result<IndexedSymbolDetail, String> {
        Ok(IndexedSymbolDetail {
            type_text: self.read_option_string()?,
            type_text_span: self.read_option_span()?,
            return_type_text: self.read_option_string()?,
            return_type_text_span: self.read_option_span()?,
            base_type: self.read_option_string()?,
            base_type_span: self.read_option_span()?,
            default_text: self.read_option_string()?,
            default_text_span: self.read_option_span()?,
            enum_value_text: self.read_option_string()?,
            enum_value_text_span: self.read_option_span()?,
        })
    }

    fn read_indexed_symbol(&mut self) -> Result<IndexedSymbol, String> {
        let id = self.read_global_id()?;
        let parent = self.read_option_global_id()?;
        let kind = symbol_kind_from_tag(self.read_u8()?)?;
        let name = self.read_option_string()?;
        let span = self.read_span()?;
        let selection_span = self.read_span()?;
        let detail = self.read_detail()?;
        let attributes = self.read_list(Self::read_attribute)?;
        let modifiers = self.read_list(Self::read_string)?;
        let doc_comments = self.read_list(Self::read_doc_comment)?;
        let conditional_context = self.read_list(Self::read_conditional_branch)?;
        let callable_form = match self.read_u8()? {
            0 => None,
            1 => Some(callable_form_from_tag(self.read_u8()?)?),
            tag => return Err(format!("invalid callable form option tag {tag}")),
        };
        Ok(IndexedSymbol {
            id,
            parent,
            kind,
            name,
            span,
            selection_span,
            detail,
            attributes,
            modifiers,
            doc_comments,
            conditional_context,
            callable_form,
        })
    }

    fn read_list<T>(
        &mut self,
        mut read_item: impl FnMut(&mut Self) -> Result<T, String>,
    ) -> Result<Vec<T>, String> {
        let len = self.read_bounded_len("symbol child list", MAX_CACHE_SYMBOL_LIST_ITEMS)?;
        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            items.push(read_item(self)?);
        }
        Ok(items)
    }

    fn read_attribute(&mut self) -> Result<IndexedAttribute, String> {
        Ok(IndexedAttribute {
            name: self.read_option_string()?,
            text: self.read_string()?,
        })
    }

    fn read_doc_comment(&mut self) -> Result<IndexedDocComment, String> {
        Ok(IndexedDocComment {
            kind: doc_comment_kind_from_tag(self.read_u8()?)?,
            text: self.read_string()?,
        })
    }

    fn read_conditional_branch(&mut self) -> Result<IndexedConditionalBranch, String> {
        Ok(IndexedConditionalBranch {
            kind: preprocessor_branch_kind_from_tag(self.read_u8()?)?,
            condition: self.read_option_string()?,
        })
    }

    fn read_cached_file_contribution(&mut self) -> Result<CachedFileContribution, String> {
        let metadata = self.read_metadata()?;
        let non_declaration_callable_fragments = self.read_usize()?;
        let symbol_count =
            self.read_bounded_len("cached public symbols", MAX_CACHE_SYMBOL_RECORDS)?;
        let mut symbols = Vec::with_capacity(symbol_count);
        for _ in 0..symbol_count {
            symbols.push(self.read_cached_public_symbol()?);
        }
        Ok(CachedFileContribution {
            metadata,
            non_declaration_callable_fragments,
            symbols,
        })
    }

    fn read_cached_public_symbol(&mut self) -> Result<CachedPublicSymbol, String> {
        let id = SemanticDeclarationId(self.read_u32()?);
        let parent = match self.read_u8()? {
            0 => None,
            1 => Some(SemanticDeclarationId(self.read_u32()?)),
            tag => return Err(format!("invalid cached parent option tag {tag}")),
        };
        let kind = semantic_declaration_kind_from_tag(self.read_u8()?)?;
        let name = self.read_string()?;
        let span = self.read_span()?;
        let selection_span = self.read_span()?;
        let detail = CachedPublicSymbolDetail {
            type_text: self.read_option_string()?,
            return_type: self.read_option_string()?,
            base_type: self.read_option_string()?,
            default_value: self.read_option_string()?,
            enum_value: self.read_option_string()?,
        };
        let attributes = self.read_list(Self::read_string)?;
        let modifiers = self.read_list(Self::read_string)?;
        let doc_comments = self.read_list(|reader| {
            Ok(CachedDocComment {
                kind: semantic_doc_comment_kind_from_tag(reader.read_u8()?)?,
                text: reader.read_string()?,
            })
        })?;
        let conditional_context = self.read_list(|reader| {
            Ok(CachedConditionalBranch {
                kind: semantic_conditional_branch_kind_from_tag(reader.read_u8()?)?,
                condition: reader.read_option_string()?,
            })
        })?;
        let callable_form = match self.read_u8()? {
            0 => None,
            1 => Some(semantic_callable_form_from_tag(self.read_u8()?)?),
            tag => return Err(format!("invalid cached callable form option tag {tag}")),
        };
        Ok(CachedPublicSymbol {
            id,
            parent,
            kind,
            name,
            span,
            selection_span,
            detail,
            attributes,
            modifiers,
            doc_comments,
            conditional_context,
            callable_form,
        })
    }

    fn expect_eof(&self) -> Result<(), String> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(format!(
                "cache has {} trailing bytes",
                self.bytes.len().saturating_sub(self.offset)
            ))
        }
    }
}

fn source_kind_tag(kind: SourceKind) -> u8 {
    match kind {
        SourceKind::Unknown => 0,
        SourceKind::GameData => 1,
        SourceKind::Workspace => 2,
        SourceKind::Fixture => 3,
    }
}

fn source_kind_from_tag(tag: u8) -> Result<SourceKind, String> {
    match tag {
        0 => Ok(SourceKind::Unknown),
        1 => Ok(SourceKind::GameData),
        2 => Ok(SourceKind::Workspace),
        3 => Ok(SourceKind::Fixture),
        _ => Err(format!("invalid source kind tag {tag}")),
    }
}

fn source_category_tag(category: SourceCategory) -> u8 {
    match category {
        SourceCategory::Workspace => 0,
        SourceCategory::Game => 1,
        SourceCategory::GameCode => 2,
        SourceCategory::GameLib => 3,
        SourceCategory::Core => 4,
        SourceCategory::Generated => 5,
        SourceCategory::Workbench => 6,
        SourceCategory::DocsDoxygen => 7,
        SourceCategory::TestAutotest => 8,
        SourceCategory::Unknown => 9,
    }
}

fn source_category_from_tag(tag: u8) -> Result<SourceCategory, String> {
    match tag {
        0 => Ok(SourceCategory::Workspace),
        1 => Ok(SourceCategory::Game),
        2 => Ok(SourceCategory::GameCode),
        3 => Ok(SourceCategory::GameLib),
        4 => Ok(SourceCategory::Core),
        5 => Ok(SourceCategory::Generated),
        6 => Ok(SourceCategory::Workbench),
        7 => Ok(SourceCategory::DocsDoxygen),
        8 => Ok(SourceCategory::TestAutotest),
        9 => Ok(SourceCategory::Unknown),
        _ => Err(format!("invalid source category tag {tag}")),
    }
}

#[cfg(test)]
fn symbol_kind_tag(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Class => 0,
        SymbolKind::TypeParameter => 1,
        SymbolKind::Enum => 2,
        SymbolKind::EnumMember => 3,
        SymbolKind::Typedef => 4,
        SymbolKind::Function => 5,
        SymbolKind::GlobalField => 6,
        SymbolKind::Field => 7,
        SymbolKind::Method => 8,
        SymbolKind::Constructor => 9,
        SymbolKind::Destructor => 10,
        SymbolKind::Parameter => 11,
        SymbolKind::LocalVariable => 12,
        SymbolKind::PreprocessorMacro => 13,
    }
}

fn symbol_kind_from_tag(tag: u8) -> Result<SymbolKind, String> {
    match tag {
        0 => Ok(SymbolKind::Class),
        1 => Ok(SymbolKind::TypeParameter),
        2 => Ok(SymbolKind::Enum),
        3 => Ok(SymbolKind::EnumMember),
        4 => Ok(SymbolKind::Typedef),
        5 => Ok(SymbolKind::Function),
        6 => Ok(SymbolKind::GlobalField),
        7 => Ok(SymbolKind::Field),
        8 => Ok(SymbolKind::Method),
        9 => Ok(SymbolKind::Constructor),
        10 => Ok(SymbolKind::Destructor),
        11 => Ok(SymbolKind::Parameter),
        12 => Ok(SymbolKind::LocalVariable),
        13 => Ok(SymbolKind::PreprocessorMacro),
        _ => Err(format!("invalid symbol kind tag {tag}")),
    }
}

fn semantic_declaration_kind_tag(kind: SemanticDeclarationKind) -> u8 {
    match kind {
        SemanticDeclarationKind::Class => 0,
        SemanticDeclarationKind::TypeParameter => 1,
        SemanticDeclarationKind::Enum => 2,
        SemanticDeclarationKind::EnumMember => 3,
        SemanticDeclarationKind::Typedef => 4,
        SemanticDeclarationKind::Function => 5,
        SemanticDeclarationKind::GlobalField => 6,
        SemanticDeclarationKind::Field => 7,
        SemanticDeclarationKind::Method => 8,
        SemanticDeclarationKind::Constructor => 9,
        SemanticDeclarationKind::Destructor => 10,
        SemanticDeclarationKind::Parameter => 11,
        SemanticDeclarationKind::LocalVariable => 12,
        SemanticDeclarationKind::PreprocessorMacro => 13,
    }
}

fn semantic_declaration_kind_from_tag(tag: u8) -> Result<SemanticDeclarationKind, String> {
    Ok(match tag {
        0 => SemanticDeclarationKind::Class,
        1 => SemanticDeclarationKind::TypeParameter,
        2 => SemanticDeclarationKind::Enum,
        3 => SemanticDeclarationKind::EnumMember,
        4 => SemanticDeclarationKind::Typedef,
        5 => SemanticDeclarationKind::Function,
        6 => SemanticDeclarationKind::GlobalField,
        7 => SemanticDeclarationKind::Field,
        8 => SemanticDeclarationKind::Method,
        9 => SemanticDeclarationKind::Constructor,
        10 => SemanticDeclarationKind::Destructor,
        11 => SemanticDeclarationKind::Parameter,
        12 => SemanticDeclarationKind::LocalVariable,
        13 => SemanticDeclarationKind::PreprocessorMacro,
        _ => return Err(format!("invalid semantic declaration kind tag {tag}")),
    })
}

fn semantic_doc_comment_kind_tag(kind: SemanticDocCommentKind) -> u8 {
    match kind {
        SemanticDocCommentKind::Line => 0,
        SemanticDocCommentKind::Block => 1,
    }
}

fn semantic_doc_comment_kind_from_tag(tag: u8) -> Result<SemanticDocCommentKind, String> {
    match tag {
        0 => Ok(SemanticDocCommentKind::Line),
        1 => Ok(SemanticDocCommentKind::Block),
        _ => Err(format!("invalid semantic doc comment kind tag {tag}")),
    }
}

fn semantic_conditional_branch_kind_tag(kind: SemanticConditionalBranchKind) -> u8 {
    match kind {
        SemanticConditionalBranchKind::If => 0,
        SemanticConditionalBranchKind::Ifdef => 1,
        SemanticConditionalBranchKind::Ifndef => 2,
        SemanticConditionalBranchKind::Elif => 3,
        SemanticConditionalBranchKind::Else => 4,
    }
}

fn semantic_conditional_branch_kind_from_tag(
    tag: u8,
) -> Result<SemanticConditionalBranchKind, String> {
    match tag {
        0 => Ok(SemanticConditionalBranchKind::If),
        1 => Ok(SemanticConditionalBranchKind::Ifdef),
        2 => Ok(SemanticConditionalBranchKind::Ifndef),
        3 => Ok(SemanticConditionalBranchKind::Elif),
        4 => Ok(SemanticConditionalBranchKind::Else),
        _ => Err(format!(
            "invalid semantic conditional branch kind tag {tag}"
        )),
    }
}

fn semantic_callable_form_tag(form: SemanticCallableForm) -> u8 {
    match form {
        SemanticCallableForm::Implementation => 0,
        SemanticCallableForm::Declaration => 1,
        SemanticCallableForm::Prototype => 2,
    }
}

fn semantic_callable_form_from_tag(tag: u8) -> Result<SemanticCallableForm, String> {
    match tag {
        0 => Ok(SemanticCallableForm::Implementation),
        1 => Ok(SemanticCallableForm::Declaration),
        2 => Ok(SemanticCallableForm::Prototype),
        _ => Err(format!("invalid semantic callable form tag {tag}")),
    }
}

#[cfg(test)]
fn doc_comment_kind_tag(kind: DocCommentKind) -> u8 {
    match kind {
        DocCommentKind::Line => 0,
        DocCommentKind::Block => 1,
    }
}

fn doc_comment_kind_from_tag(tag: u8) -> Result<DocCommentKind, String> {
    match tag {
        0 => Ok(DocCommentKind::Line),
        1 => Ok(DocCommentKind::Block),
        _ => Err(format!("invalid doc comment kind tag {tag}")),
    }
}

#[cfg(test)]
fn preprocessor_branch_kind_tag(kind: PreprocessorBranchKind) -> u8 {
    match kind {
        PreprocessorBranchKind::If => 0,
        PreprocessorBranchKind::Ifdef => 1,
        PreprocessorBranchKind::Ifndef => 2,
        PreprocessorBranchKind::Elif => 3,
        PreprocessorBranchKind::Else => 4,
    }
}

fn preprocessor_branch_kind_from_tag(tag: u8) -> Result<PreprocessorBranchKind, String> {
    match tag {
        0 => Ok(PreprocessorBranchKind::If),
        1 => Ok(PreprocessorBranchKind::Ifdef),
        2 => Ok(PreprocessorBranchKind::Ifndef),
        3 => Ok(PreprocessorBranchKind::Elif),
        4 => Ok(PreprocessorBranchKind::Else),
        _ => Err(format!("invalid preprocessor branch kind tag {tag}")),
    }
}

#[cfg(test)]
fn callable_form_tag(form: CallableForm) -> u8 {
    match form {
        CallableForm::Implementation => 0,
        CallableForm::Declaration => 1,
        CallableForm::Prototype => 2,
    }
}

fn callable_form_from_tag(tag: u8) -> Result<CallableForm, String> {
    match tag {
        0 => Ok(CallableForm::Implementation),
        1 => Ok(CallableForm::Declaration),
        2 => Ok(CallableForm::Prototype),
        _ => Err(format!("invalid callable form tag {tag}")),
    }
}

fn cache_rebuild_reason(cache_path: &Path, fingerprint: &SourceFingerprint) -> String {
    if !cache_path.is_file() {
        return "cache-missing".to_string();
    }
    format!(
        "cache-stale-or-incompatible fingerprint={}",
        fingerprint.summary()
    )
}

fn source_fingerprint(
    scripts_root: &Path,
    metadata_path: Option<&Path>,
) -> Result<SourceFingerprint, String> {
    let scripts_root = scripts_root
        .canonicalize()
        .unwrap_or_else(|_| scripts_root.to_path_buf())
        .to_string_lossy()
        .to_string();

    if let Some(metadata_path) = metadata_path {
        if let Some(commit_sha) = read_commit_sha(metadata_path)? {
            return Ok(SourceFingerprint::Downloaded {
                scripts_root,
                commit_sha,
            });
        }
    }

    let manual = manual_folder_fingerprint(Path::new(&scripts_root))?;
    Ok(SourceFingerprint::Manual {
        scripts_root,
        file_count: manual.file_count,
        byte_count: manual.byte_count,
        latest_modified_unix_ms: manual.latest_modified_unix_ms,
    })
}

fn read_commit_sha(metadata_path: &Path) -> Result<Option<String>, String> {
    if !metadata_path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(metadata_path).map_err(|error| {
        format!(
            "Failed to read game-data metadata {}: {error}",
            metadata_path.display()
        )
    })?;
    let json = serde_json::from_str::<Value>(&raw).map_err(|error| {
        format!(
            "Failed to parse game-data metadata {}: {error}",
            metadata_path.display()
        )
    })?;
    Ok(json
        .get("commitSha")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManualFolderFingerprint {
    file_count: usize,
    byte_count: u64,
    latest_modified_unix_ms: u128,
}

fn manual_folder_fingerprint(root: &Path) -> Result<ManualFolderFingerprint, String> {
    let mut fingerprint = ManualFolderFingerprint {
        file_count: 0,
        byte_count: 0,
        latest_modified_unix_ms: 0,
    };
    collect_manual_fingerprint(root, &mut fingerprint)?;
    Ok(fingerprint)
}

fn collect_manual_fingerprint(
    folder: &Path,
    fingerprint: &mut ManualFolderFingerprint,
) -> Result<(), String> {
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Failed to read folder {}: {error}", folder.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to read entry in {}: {error}", folder.display()))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Failed to read metadata for {}: {error}", path.display()))?;
        if metadata.is_dir() {
            collect_manual_fingerprint(&path, fingerprint)?;
        } else if metadata.is_file() && path.extension().is_some_and(|extension| extension == "c") {
            fingerprint.file_count += 1;
            fingerprint.byte_count += metadata.len();
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis())
                .unwrap_or(0);
            fingerprint.latest_modified_unix_ms = fingerprint.latest_modified_unix_ms.max(modified);
        }
    }
    Ok(())
}

fn summary_from_build(result: &IndexBuildResult) -> RuntimeIndexSummary {
    RuntimeIndexSummary {
        files: result.summary.totals.files,
        bytes: result.summary.totals.bytes,
        indexed_symbols: result.summary.totals.indexed_symbols,
        parse_diagnostics: result.summary.totals.parse_diagnostics,
        lossy_files: result.summary.totals.lossy_files,
    }
}

fn summary_from_build_with_cached_index(
    result: &IndexBuildResult,
    cached_index: &SymbolIndex,
) -> RuntimeIndexSummary {
    RuntimeIndexSummary {
        indexed_symbols: cached_index.symbols().len(),
        ..summary_from_build(result)
    }
}

impl SourceFingerprint {
    pub fn summary(&self) -> String {
        match self {
            Self::Downloaded { commit_sha, .. } => format!("downloaded:{commit_sha}"),
            Self::Manual {
                file_count,
                byte_count,
                latest_modified_unix_ms,
                ..
            } => format!(
                "manual:files={file_count}:bytes={byte_count}:modified={latest_modified_unix_ms}"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SymbolKind;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cache_loads_when_fingerprint_matches() {
        let root = test_root("loads");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        write_file(
            &scripts.join("Game/Example.c"),
            "class Example { int m_Value; void Run(int value); }",
        );
        write_file(
            &root.join("metadata.json"),
            r#"{"commitSha":"abc123","fileCount":1}"#,
        );

        let first = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(root.join("metadata.json")),
        })
        .unwrap();
        assert!(matches!(
            first.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        let second = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: Some(root.join("metadata.json")),
        })
        .unwrap();
        assert_eq!(second.cache_status, IndexCacheStatus::Loaded);
        assert_eq!(second.summary.indexed_symbols, 4);
        assert_eq!(second.index.classes_by_name("Example").len(), 1);
        assert_eq!(
            second.index.methods_by_owner_name("Example", "Run").len(),
            1
        );
        assert!(second.timings.cache_read_deserialize_validate > std::time::Duration::ZERO);
        assert!(second.timings.total > std::time::Duration::ZERO);
        assert!(second.cache_file_bytes.unwrap_or_default() > 0);
        let cache_bytes = fs::read(&cache).unwrap();
        assert!(cache_bytes.starts_with(CACHE_MAGIC));
        let decoded = decode_cached_index(&cache_bytes).unwrap();
        assert_eq!(decoded.format_version, CACHE_FORMAT_VERSION);
        assert_eq!(decoded.index_shape, CACHE_INDEX_SHAPE);
        assert_eq!(decoded.files.len(), 1);
        decoded.validate().unwrap();

        cleanup(&root);
    }

    #[test]
    fn canonical_codec_is_materially_smaller_than_v10_without_json_or_duplicate_strings() {
        let root = test_root("canonical_codec_size");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        let repeated_type = "CacheSizeSentinel";
        write_file(
            &scripts.join("Game/First.c"),
            "class CacheSizeSentinel {}\nclass First { CacheSizeSentinel Make(CacheSizeSentinel value); }",
        );
        write_file(
            &scripts.join("Game/Second.c"),
            "class Second { CacheSizeSentinel Make(CacheSizeSentinel value); }",
        );
        write_file(
            &scripts.join("Game/Third.c"),
            "class Third { CacheSizeSentinel Make(CacheSizeSentinel value); }",
        );
        write_file(&metadata, r#"{"commitSha":"canonical-codec-size"}"#);

        load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: Some(metadata),
        })
        .unwrap();
        let v11_bytes = fs::read(&cache).unwrap();
        let current = decode_cached_index(&v11_bytes).unwrap();
        let runtime = current.clone().into_index();
        let v10 = V10CachedGameDataIndex {
            schema: CACHE_SCHEMA.to_string(),
            format_version: V10_CACHE_FORMAT_VERSION,
            index_shape: V10_CACHE_INDEX_SHAPE.to_string(),
            crate_version: current.crate_version.clone(),
            fingerprint: current.fingerprint.clone(),
            summary: current.summary.clone(),
            index: V10CachedSymbolIndex {
                files: runtime.files().to_vec(),
                symbols: runtime.symbols().to_vec(),
                contributions: current
                    .files
                    .iter()
                    .map(CachedFileContribution::to_file_contribution)
                    .collect(),
            },
        };
        let v10_bytes = encode_v10_cached_index(&v10).unwrap();

        assert!(
            v11_bytes.len() * 4 <= v10_bytes.len() * 3,
            "canonical v11 cache ({}) must be at least 25% smaller than equivalent v10 ({})",
            v11_bytes.len(),
            v10_bytes.len()
        );
        assert_eq!(count_subslice(&v11_bytes, repeated_type.as_bytes()), 1);
        assert_eq!(count_subslice(&v11_bytes, b"\"schema_version\""), 0);
        assert!(count_subslice(&v10_bytes, b"\"schema_version\"") > 0);

        cleanup(&root);
    }

    #[test]
    fn cache_rebuilds_when_commit_changes() {
        let root = test_root("commit_changes");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(&scripts.join("Example.c"), "class Example {}");
        write_file(&metadata, r#"{"commitSha":"one"}"#);

        let _ = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(metadata.clone()),
        })
        .unwrap();
        write_file(&metadata, r#"{"commitSha":"two"}"#);

        let rebuilt = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: Some(metadata),
        })
        .unwrap();

        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        cleanup(&root);
    }

    #[test]
    fn cache_rebuild_writes_runtime_pruned_index() {
        let root = test_root("pruned");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(
            &scripts.join("Game/Example.c"),
            r#"class Example
{
	ref array<int> m_Values;

	int Run(string name = "ok")
	{
		int localValue = 1;
		return localValue;
	}
}
"#,
        );
        write_file(&metadata, r#"{"commitSha":"abc123"}"#);

        let result = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(metadata.clone()),
        })
        .unwrap();

        assert!(matches!(
            result.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert!(result
            .index
            .symbols_for_kind(SymbolKind::LocalVariable)
            .is_empty());
        assert_eq!(
            result.index.symbols_for_kind(SymbolKind::Parameter).len(),
            1
        );
        assert_eq!(result.index.symbols_for_name("localValue").len(), 0);
        let field = result
            .index
            .symbol(result.index.fields_by_owner_name("Example", "m_Values")[0])
            .unwrap();
        assert_eq!(field.detail.type_text.as_deref(), Some("ref array<int>"));
        assert!(field.detail.type_text_span.is_none());
        assert_eq!(
            result
                .index
                .callable_signature(result.index.methods_by_owner_name("Example", "Run")[0])
                .as_deref(),
            Some("Example.Run(string name = \"ok\") -> int")
        );
        let parameter = result.index.symbols_for_name("name")[0];
        let parameter = result.index.symbol(parameter).unwrap();
        assert_eq!(parameter.detail.type_text.as_deref(), Some("string"));
        assert_eq!(parameter.detail.default_text.as_deref(), Some("\"ok\""));
        assert!(parameter.detail.type_text_span.is_none());
        assert!(parameter.detail.default_text_span.is_none());

        let loaded = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: Some(metadata),
        })
        .unwrap();
        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert!(loaded
            .index
            .symbols_for_kind(SymbolKind::LocalVariable)
            .is_empty());
        assert_eq!(
            loaded.index.symbols_for_kind(SymbolKind::Parameter).len(),
            1
        );
        assert_eq!(loaded.index.classes_by_name("Example").len(), 1);
        assert_eq!(
            loaded
                .index
                .fields_by_owner_name("Example", "m_Values")
                .len(),
            1
        );
        assert_eq!(
            loaded.index.methods_by_owner_name("Example", "Run").len(),
            1
        );
        assert_eq!(
            loaded
                .index
                .callable_signature(loaded.index.methods_by_owner_name("Example", "Run")[0])
                .as_deref(),
            Some("Example.Run(string name = \"ok\") -> int")
        );

        cleanup(&root);
    }

    #[test]
    fn cache_preserves_public_symbols_after_pruned_local_ids() {
        const SOURCE: &str =
            include_str!("../../tools/fixtures/index/contribution_public_ids_after_local.c");
        let root = test_root("public-ids-after-local");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Game/Contribution.c"), SOURCE);

        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: None,
        };
        let rebuilt = load_or_build_game_data_index(&config).unwrap();
        let later = rebuilt
            .index
            .classes_by_name("ContributionIdsAfterPublicFixture")[0];
        assert_eq!(later.symbol_id.0, 2);
        assert_eq!(
            rebuilt
                .index
                .symbol(later)
                .and_then(|symbol| symbol.name.as_deref()),
            Some("ContributionIdsAfterPublicFixture")
        );

        let loaded = load_or_build_game_data_index(&config).unwrap();
        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        let later = loaded
            .index
            .classes_by_name("ContributionIdsAfterPublicFixture")[0];
        assert_eq!(later.symbol_id.0, 2);
        assert_eq!(
            loaded
                .index
                .symbol(later)
                .and_then(|symbol| symbol.name.as_deref()),
            Some("ContributionIdsAfterPublicFixture")
        );

        cleanup(&root);
    }

    #[test]
    fn cache_rebuilds_when_a_contribution_has_sparse_public_ids() {
        const SOURCE: &str =
            include_str!("../../tools/fixtures/index/contribution_public_ids_after_local.c");
        let root = test_root("sparse-public-ids");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Game/Contribution.c"), SOURCE);

        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: None,
        };
        load_or_build_game_data_index(&config).unwrap();

        let mut decoded = decode_cached_index(&fs::read(&cache).unwrap()).unwrap();
        decoded.files[0].symbols[2].id = SemanticDeclarationId(3);
        fs::write(&cache, encode_cached_index(&decoded).unwrap()).unwrap();

        let rebuilt = load_or_build_game_data_index(&config).unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        cleanup(&root);
    }

    #[test]
    fn v9_cache_migrates_to_validated_v11_contributions() {
        let root = test_root("v9_migration");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(
            &scripts.join("Game/Example.c"),
            "class Example { int m_Value; void Run(int value); }",
        );
        write_file(&metadata, r#"{"commitSha":"v9-migration"}"#);
        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: Some(metadata),
        };
        load_or_build_game_data_index(&config).unwrap();
        rewrite_cache_as_v9(&cache, |legacy| legacy).unwrap();
        assert!(fs::read(&cache).unwrap().starts_with(LEGACY_CACHE_MAGIC));

        let migrated = load_or_build_game_data_index(&config).unwrap();
        assert_eq!(migrated.cache_status, IndexCacheStatus::Loaded);
        assert_eq!(migrated.index.classes_by_name("Example").len(), 1);
        assert_eq!(
            migrated.index.methods_by_owner_name("Example", "Run").len(),
            1
        );
        let migrated_bytes = fs::read(&cache).unwrap();
        assert!(migrated_bytes.starts_with(CACHE_MAGIC));
        let migrated_cache = decode_cached_index(&migrated_bytes).unwrap();
        migrated_cache.validate().unwrap();

        cleanup(&root);
    }

    #[test]
    fn v10_cache_migrates_at_the_same_path_without_source_parsing() {
        let root = test_root("v10_migration");
        let cache = root.join("game-data-symbol-index.v9.bin");
        let scripts = root.join("scripts");
        let source = scripts.join("Game/Example.c");
        let metadata = root.join("metadata.json");
        write_file(
            &source,
            "class Example { int m_Value; void Run(int value); }",
        );
        write_file(&metadata, r#"{"commitSha":"v10-migration"}"#);
        let config = GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(metadata),
        };
        let cold = load_or_build_game_data_index(&config).unwrap();
        let expected_signature = cold
            .index
            .callable_signature(cold.index.methods_by_owner_name("Example", "Run")[0]);
        rewrite_cache_as_v10(&cache, |cached| cached).unwrap();
        assert!(fs::read(&cache).unwrap().starts_with(V10_CACHE_MAGIC));

        // A downloaded fingerprint is metadata-backed. Removing the source
        // proves this result came from the v10 migration, not source parsing.
        fs::remove_file(source).unwrap();
        let migrated = load_or_build_game_data_index(&config).unwrap();
        assert_eq!(migrated.cache_status, IndexCacheStatus::Loaded);
        assert_eq!(migrated.index.classes_by_name("Example").len(), 1);
        assert_eq!(
            migrated
                .index
                .callable_signature(migrated.index.methods_by_owner_name("Example", "Run")[0]),
            expected_signature
        );
        let migrated_bytes = fs::read(&cache).unwrap();
        assert!(migrated_bytes.starts_with(CACHE_MAGIC));
        assert!(fs::read_dir(&root).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp")));

        cleanup(&root);
    }

    #[test]
    fn divergent_v10_query_graph_rebuilds_instead_of_migrating() {
        let root = test_root("v10_divergent_query_graph");
        let cache = root.join("game-data-symbol-index.v9.bin");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(
            &scripts.join("Game/Example.c"),
            "class Example { void Run(); }",
        );
        write_file(&metadata, r#"{"commitSha":"v10-divergent-query-graph"}"#);
        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: Some(metadata),
        };
        load_or_build_game_data_index(&config).unwrap();
        rewrite_cache_as_v10(&cache, |mut cached| {
            cached.index.symbols[0].name = Some("NotExample".to_string());
            cached
        })
        .unwrap();

        let rebuilt = load_or_build_game_data_index(&config).unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert_eq!(rebuilt.index.classes_by_name("Example").len(), 1);
        assert!(fs::read(&cache).unwrap().starts_with(CACHE_MAGIC));

        cleanup(&root);
    }

    #[test]
    fn invalid_or_oversized_v10_cache_rebuilds_safely() {
        let root = test_root("v10_invalid");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(&scripts.join("Example.c"), "class Example {}");
        write_file(&metadata, r#"{"commitSha":"v10-invalid"}"#);
        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: Some(metadata),
        };
        load_or_build_game_data_index(&config).unwrap();

        rewrite_cache_as_v10(&cache, |mut cached| {
            cached.fingerprint = SourceFingerprint::Downloaded {
                scripts_root: "wrong-root".to_string(),
                commit_sha: "stale".to_string(),
            };
            cached
        })
        .unwrap();
        assert!(matches!(
            load_or_build_game_data_index(&config).unwrap().cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        rewrite_cache_as_v10(&cache, |cached| cached).unwrap();
        let mut corrupt = fs::read(&cache).unwrap();
        corrupt.truncate(V10_CACHE_MAGIC.len() + 1);
        fs::write(&cache, corrupt).unwrap();
        assert!(matches!(
            load_or_build_game_data_index(&config).unwrap().cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        fs::write(&cache, V10_CACHE_MAGIC).unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(&cache)
            .unwrap()
            .set_len(MAX_LEGACY_CACHE_BYTES + 1)
            .unwrap();
        assert!(matches!(
            load_or_build_game_data_index(&config).unwrap().cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert!(fs::read(&cache).unwrap().starts_with(CACHE_MAGIC));

        cleanup(&root);
    }

    #[test]
    fn v9_cache_with_wrong_fingerprint_rebuilds_instead_of_migrating() {
        let root = test_root("v9_wrong_fingerprint");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(&scripts.join("Example.c"), "class Example {}");
        write_file(&metadata, r#"{"commitSha":"current"}"#);
        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: Some(metadata),
        };
        load_or_build_game_data_index(&config).unwrap();
        rewrite_cache_as_v9(&cache, |mut legacy| {
            legacy.fingerprint = SourceFingerprint::Downloaded {
                scripts_root: "wrong-root".to_string(),
                commit_sha: "stale".to_string(),
            };
            legacy
        })
        .unwrap();

        let rebuilt = load_or_build_game_data_index(&config).unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert!(fs::read(&cache).unwrap().starts_with(CACHE_MAGIC));
        cleanup(&root);
    }

    #[test]
    fn v9_cache_with_wrong_version_or_malformed_records_rebuilds() {
        let root = test_root("v9_invalid");
        let cache = root.join("cache.bin");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Example.c"), "class Example { int m_Value; }");
        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: None,
        };

        load_or_build_game_data_index(&config).unwrap();
        rewrite_cache_as_v9(&cache, |mut legacy| {
            legacy.format_version = LEGACY_CACHE_FORMAT_VERSION - 1;
            legacy
        })
        .unwrap();
        assert!(matches!(
            load_or_build_game_data_index(&config).unwrap().cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        rewrite_cache_as_v9(&cache, |mut legacy| {
            legacy.index.symbols[0].parent = Some(GlobalSymbolId {
                file_id: SourceFileId(999),
                symbol_id: SymbolId(999),
            });
            legacy
        })
        .unwrap();
        assert!(matches!(
            load_or_build_game_data_index(&config).unwrap().cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        cleanup(&root);
    }

    #[test]
    fn v10_binary_cache_load_rebuilds_lookup_maps_from_files_and_symbols() {
        let root = test_root("rebuild_maps");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(
            &scripts.join("Game/Example.c"),
            r#"typedef string FactionKey;

void GlobalFn(int value);

class Example : BaseExample
{
	int m_Value;
	void Run(string name);
}
"#,
        );
        write_file(&metadata, r#"{"commitSha":"abc123"}"#);

        let _ = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(metadata.clone()),
        })
        .unwrap();

        let loaded = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: Some(metadata),
        })
        .unwrap();

        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert_eq!(loaded.index.classes_by_name("Example").len(), 1);
        assert_eq!(loaded.index.typedefs_by_name("FactionKey").len(), 1);
        assert_eq!(loaded.index.functions_by_name("GlobalFn").len(), 1);
        assert_eq!(loaded.index.symbols_for_kind(SymbolKind::Class).len(), 1);
        assert_eq!(loaded.index.symbols_for_name("m_Value").len(), 1);
        assert_eq!(
            loaded
                .index
                .fields_by_owner_name("Example", "m_Value")
                .len(),
            1
        );
        assert_eq!(
            loaded.index.methods_by_owner_name("Example", "Run").len(),
            1
        );
        assert!(loaded
            .index
            .members_by_owner("Example")
            .iter()
            .any(|id| loaded
                .index
                .symbol(*id)
                .is_some_and(|symbol| symbol.name.as_deref() == Some("m_Value"))));

        let class = loaded.index.classes_by_name("Example")[0];
        assert!(!loaded.index.children(class).is_empty());
        assert_eq!(
            loaded.index.preferred_classes_by_name("Example"),
            vec![class]
        );
        assert_eq!(
            loaded
                .index
                .callable_signature(loaded.index.methods_by_owner_name("Example", "Run")[0])
                .as_deref(),
            Some("Example.Run(string name) -> void")
        );

        cleanup(&root);
    }

    #[test]
    fn cache_load_preserves_multi_file_symbol_ranges() {
        let root = test_root("multi_file_ranges");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        let metadata = root.join("metadata.json");
        write_file(
            &scripts.join("Game/First.c"),
            r#"class First
{
	void Run()
	{
		int localValue;
	}
}
"#,
        );
        write_file(
            &scripts.join("Game/generated/GameMode/BaseGameMode.c"),
            r#"class GenericEntityClass {}

class BaseGameModeClass : GenericEntityClass
{
}
"#,
        );
        write_file(&metadata, r#"{"commitSha":"abc123"}"#);

        let _ = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: Some(metadata.clone()),
        })
        .unwrap();

        let loaded = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: Some(metadata),
        })
        .unwrap();

        assert_eq!(loaded.cache_status, IndexCacheStatus::Loaded);
        assert!(loaded
            .index
            .symbols_for_kind(SymbolKind::LocalVariable)
            .is_empty());

        let base_class_ids = loaded.index.classes_by_name("BaseGameModeClass");
        assert_eq!(base_class_ids.len(), 1);
        let base_class = loaded.index.symbol(base_class_ids[0]).unwrap();
        assert_eq!(base_class.name.as_deref(), Some("BaseGameModeClass"));
        assert_eq!(base_class.kind, SymbolKind::Class);
        assert_eq!(
            base_class.detail.base_type.as_deref(),
            Some("GenericEntityClass")
        );

        let generic_class_ids = loaded.index.classes_by_name("GenericEntityClass");
        assert_eq!(generic_class_ids.len(), 1);
        let generic_class = loaded.index.symbol(generic_class_ids[0]).unwrap();
        assert_eq!(generic_class.name.as_deref(), Some("GenericEntityClass"));

        cleanup(&root);
    }

    #[test]
    fn cache_rebuilds_when_format_version_is_stale() {
        let root = test_root("format_version");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Example.c"), "class Example {}");

        let _ = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: None,
        })
        .unwrap();

        let mut decoded = decode_cached_index(&fs::read(&cache).unwrap()).unwrap();
        decoded.format_version = CACHE_FORMAT_VERSION - 1;
        fs::write(&cache, encode_cached_index(&decoded).unwrap()).unwrap();

        let rebuilt = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: None,
        })
        .unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        cleanup(&root);
    }

    #[test]
    fn cache_rebuilds_when_a_public_contribution_is_not_current() {
        let root = test_root("stale-contribution");
        let scripts = root.join("scripts");
        let cache = root.join("index.bin");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(scripts.join("Example.c"), "class Example {}\n").unwrap();

        let config = GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache.clone(),
            metadata_path: None,
        };
        load_or_build_game_data_index(&config).unwrap();

        let mut decoded = decode_cached_index(&fs::read(&cache).unwrap()).unwrap();
        decoded.files[0].symbols[0].id = SemanticDeclarationId(7);
        fs::write(&cache, encode_cached_index(&decoded).unwrap()).unwrap();

        let rebuilt = load_or_build_game_data_index(&config).unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        cleanup(&root);
    }

    #[test]
    fn cache_rebuilds_when_index_shape_is_stale() {
        let root = test_root("index_shape");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Example.c"), "class Example {}");

        let _ = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts.clone(),
            cache_path: cache.clone(),
            metadata_path: None,
        })
        .unwrap();

        let mut decoded = decode_cached_index(&fs::read(&cache).unwrap()).unwrap();
        decoded.index_shape = "old-index-shape".to_string();
        fs::write(&cache, encode_cached_index(&decoded).unwrap()).unwrap();

        let rebuilt = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: None,
        })
        .unwrap();
        assert!(matches!(
            rebuilt.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));

        cleanup(&root);
    }

    #[test]
    fn corrupt_cache_rebuilds_without_failing() {
        let root = test_root("corrupt");
        let cache = root.join("cache.json");
        let scripts = root.join("scripts");
        write_file(&scripts.join("Example.c"), "class Example {}");
        fs::write(&cache, b"bad binary cache").unwrap();

        let result = load_or_build_game_data_index(&GameDataIndexCacheConfig {
            scripts_root: scripts,
            cache_path: cache,
            metadata_path: None,
        })
        .unwrap();

        assert!(matches!(
            result.cache_status,
            IndexCacheStatus::Rebuilt { .. }
        ));
        assert_eq!(result.index.classes_by_name("Example").len(), 1);
        assert!(result.timings.rebuild > std::time::Duration::ZERO);
        assert!(result.timings.cache_write > std::time::Duration::ZERO);
        assert!(result.timings.total > std::time::Duration::ZERO);

        cleanup(&root);
    }

    #[test]
    fn corrupt_binary_cache_lengths_do_not_allocate_unbounded_memory() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(CACHE_MAGIC);
        bytes.extend_from_slice(&(MAX_CACHE_STRING_TABLE_ENTRIES as u64 + 1).to_le_bytes());

        let error = decode_cached_index(&bytes).unwrap_err();
        assert!(error.contains("string table entry length"));
        assert!(error.contains("exceeds safety limit"));
    }

    #[test]
    fn manual_fingerprint_changes_when_files_change() {
        let root = test_root("manual_fingerprint");
        let scripts = root.join("scripts");
        write_file(&scripts.join("A.c"), "class A {}");
        let first = source_fingerprint(&scripts, None).unwrap();
        write_file(&scripts.join("B.c"), "class B {}");
        let second = source_fingerprint(&scripts, None).unwrap();

        assert_ne!(first, second);

        cleanup(&root);
    }

    fn test_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "reforger_index_cache_{name}_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn rewrite_cache_as_v9(
        cache_path: &Path,
        transform: impl FnOnce(LegacyCachedGameDataIndex) -> LegacyCachedGameDataIndex,
    ) -> Result<(), String> {
        let current =
            decode_cached_index(&fs::read(cache_path).map_err(|error| error.to_string())?)?;
        let runtime = current.clone().into_index();
        let legacy = transform(LegacyCachedGameDataIndex {
            schema: CACHE_SCHEMA.to_string(),
            format_version: LEGACY_CACHE_FORMAT_VERSION,
            index_shape: LEGACY_CACHE_INDEX_SHAPE.to_string(),
            crate_version: current.crate_version,
            fingerprint: current.fingerprint,
            summary: current.summary,
            index: LegacyCachedSymbolIndex {
                files: runtime.files().to_vec(),
                symbols: runtime.symbols().to_vec(),
            },
        });
        fs::write(cache_path, encode_legacy_cached_index(&legacy)?)
            .map_err(|error| error.to_string())
    }

    fn rewrite_cache_as_v10(
        cache_path: &Path,
        transform: impl FnOnce(V10CachedGameDataIndex) -> V10CachedGameDataIndex,
    ) -> Result<(), String> {
        let current =
            decode_cached_index(&fs::read(cache_path).map_err(|error| error.to_string())?)?;
        let runtime = current.clone().into_index();
        let cached = transform(V10CachedGameDataIndex {
            schema: CACHE_SCHEMA.to_string(),
            format_version: V10_CACHE_FORMAT_VERSION,
            index_shape: V10_CACHE_INDEX_SHAPE.to_string(),
            crate_version: current.crate_version,
            fingerprint: current.fingerprint,
            summary: current.summary,
            index: V10CachedSymbolIndex {
                files: runtime.files().to_vec(),
                symbols: runtime.symbols().to_vec(),
                contributions: current
                    .files
                    .iter()
                    .map(CachedFileContribution::to_file_contribution)
                    .collect(),
            },
        });
        fs::write(cache_path, encode_v10_cached_index(&cached)?).map_err(|error| error.to_string())
    }

    fn write_file(path: &Path, source: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, source).unwrap();
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn count_subslice(bytes: &[u8], needle: &[u8]) -> usize {
        bytes
            .windows(needle.len())
            .filter(|window| *window == needle)
            .count()
    }
}
