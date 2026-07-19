use crate::ast::DocCommentKind;
use crate::lexer::TextSpan;
use crate::model::{
    CallableForm, ConditionalBranch, PreprocessorBranchKind, SourceFileMetadata, SourceKind,
    SymbolCatalog, SymbolId, SymbolKind,
};
use crate::semantic_file::{
    FileContribution, FileContributionValidationError, PublicSymbol, PublicText,
    SemanticCallableForm, SemanticConditionalBranchKind, SemanticDeclarationKind,
    SemanticDocCommentKind, SemanticFile,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceFileId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalSymbolId {
    pub file_id: SourceFileId,
    pub symbol_id: SymbolId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedFile {
    pub id: SourceFileId,
    pub metadata: SourceFileMetadata,
    pub symbol_start: usize,
    pub symbol_count: usize,
    pub non_declaration_callable_fragments: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedSymbolDetail {
    pub type_text: Option<String>,
    pub type_text_span: Option<TextSpan>,
    pub return_type_text: Option<String>,
    pub return_type_text_span: Option<TextSpan>,
    pub base_type: Option<String>,
    pub base_type_span: Option<TextSpan>,
    pub default_text: Option<String>,
    pub default_text_span: Option<TextSpan>,
    pub enum_value_text: Option<String>,
    pub enum_value_text_span: Option<TextSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedSymbol {
    pub id: GlobalSymbolId,
    pub parent: Option<GlobalSymbolId>,
    pub kind: SymbolKind,
    pub name: Option<String>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub detail: IndexedSymbolDetail,
    pub attributes: Vec<IndexedAttribute>,
    pub modifiers: Vec<String>,
    pub doc_comments: Vec<IndexedDocComment>,
    pub conditional_context: Vec<IndexedConditionalBranch>,
    pub callable_form: Option<CallableForm>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedAttribute {
    pub name: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedDocComment {
    pub kind: DocCommentKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedConditionalBranch {
    pub kind: PreprocessorBranchKind,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionMemberLookup {
    pub raw_candidates: Vec<GlobalSymbolId>,
    pub members: Vec<GlobalSymbolId>,
    pub shadowed_groups: Vec<MemberShadowGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberShadowGroup {
    pub key: String,
    pub kept: GlobalSymbolId,
    pub shadowed: Vec<GlobalSymbolId>,
}

#[derive(Debug, Clone, Default)]
pub struct SymbolIndex {
    files: Vec<IndexedFile>,
    symbols: Vec<IndexedSymbol>,
    by_name: BTreeMap<String, Vec<GlobalSymbolId>>,
    top_level_by_name: BTreeMap<String, Vec<GlobalSymbolId>>,
    by_kind: BTreeMap<SymbolKind, Vec<GlobalSymbolId>>,
    children: BTreeMap<GlobalSymbolId, Vec<GlobalSymbolId>>,
    classes_by_name: BTreeMap<String, Vec<GlobalSymbolId>>,
    typedefs_by_name: BTreeMap<String, Vec<GlobalSymbolId>>,
    functions_by_name: BTreeMap<String, Vec<GlobalSymbolId>>,
    methods_by_owner_name: BTreeMap<(String, String), Vec<GlobalSymbolId>>,
    fields_by_owner_name: BTreeMap<(String, String), Vec<GlobalSymbolId>>,
    members_by_owner: BTreeMap<String, Vec<GlobalSymbolId>>,
    #[cfg(test)]
    lookup_map_rebuild_count: usize,
}

#[derive(Serialize, Deserialize)]
struct SymbolIndexSnapshot {
    files: Vec<IndexedFile>,
    symbols: Vec<IndexedSymbol>,
    by_name: Vec<(String, Vec<GlobalSymbolId>)>,
    top_level_by_name: Vec<(String, Vec<GlobalSymbolId>)>,
    by_kind: Vec<(SymbolKind, Vec<GlobalSymbolId>)>,
    children: Vec<(GlobalSymbolId, Vec<GlobalSymbolId>)>,
    classes_by_name: Vec<(String, Vec<GlobalSymbolId>)>,
    typedefs_by_name: Vec<(String, Vec<GlobalSymbolId>)>,
    functions_by_name: Vec<(String, Vec<GlobalSymbolId>)>,
    methods_by_owner_name: Vec<((String, String), Vec<GlobalSymbolId>)>,
    fields_by_owner_name: Vec<((String, String), Vec<GlobalSymbolId>)>,
    members_by_owner: Vec<(String, Vec<GlobalSymbolId>)>,
}

impl From<&SymbolIndex> for SymbolIndexSnapshot {
    fn from(index: &SymbolIndex) -> Self {
        Self {
            files: index.files.clone(),
            symbols: index.symbols.clone(),
            by_name: index
                .by_name
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            top_level_by_name: index
                .top_level_by_name
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            by_kind: index
                .by_kind
                .iter()
                .map(|(key, value)| (*key, value.clone()))
                .collect(),
            children: index
                .children
                .iter()
                .map(|(key, value)| (*key, value.clone()))
                .collect(),
            classes_by_name: index
                .classes_by_name
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            typedefs_by_name: index
                .typedefs_by_name
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            functions_by_name: index
                .functions_by_name
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            methods_by_owner_name: index
                .methods_by_owner_name
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            fields_by_owner_name: index
                .fields_by_owner_name
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            members_by_owner: index
                .members_by_owner
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        }
    }
}

impl From<SymbolIndexSnapshot> for SymbolIndex {
    fn from(snapshot: SymbolIndexSnapshot) -> Self {
        Self {
            files: snapshot.files,
            symbols: snapshot.symbols,
            by_name: snapshot.by_name.into_iter().collect(),
            top_level_by_name: snapshot.top_level_by_name.into_iter().collect(),
            by_kind: snapshot.by_kind.into_iter().collect(),
            children: snapshot.children.into_iter().collect(),
            classes_by_name: snapshot.classes_by_name.into_iter().collect(),
            typedefs_by_name: snapshot.typedefs_by_name.into_iter().collect(),
            functions_by_name: snapshot.functions_by_name.into_iter().collect(),
            methods_by_owner_name: snapshot.methods_by_owner_name.into_iter().collect(),
            fields_by_owner_name: snapshot.fields_by_owner_name.into_iter().collect(),
            members_by_owner: snapshot.members_by_owner.into_iter().collect(),
            #[cfg(test)]
            lookup_map_rebuild_count: 0,
        }
    }
}

impl Serialize for SymbolIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SymbolIndexSnapshot::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SymbolIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        SymbolIndexSnapshot::deserialize(deserializer).map(Self::from)
    }
}

impl SymbolIndex {
    pub fn from_catalogs<'source>(
        catalogs: impl IntoIterator<Item = &'source SymbolCatalog<'source>>,
    ) -> Self {
        let mut index = Self::default();
        for catalog in catalogs {
            index.add_catalog(catalog);
        }
        index
    }

    pub fn from_indexed_parts(files: Vec<IndexedFile>, symbols: Vec<IndexedSymbol>) -> Self {
        let mut index = Self {
            files,
            symbols,
            ..Self::default()
        };
        index.rebuild_lookup_maps();
        index
    }

    pub fn merged<'a>(indexes: impl IntoIterator<Item = &'a SymbolIndex>) -> Self {
        let mut merged = Self::default();
        for index in indexes {
            for file in &index.files {
                let mut remapped_ids = BTreeMap::<GlobalSymbolId, GlobalSymbolId>::new();
                let new_file_id = SourceFileId(merged.files.len());
                let symbol_start = merged.symbols.len();

                for symbol in index.symbols_for_file(file) {
                    remapped_ids.insert(
                        symbol.id,
                        GlobalSymbolId {
                            file_id: new_file_id,
                            symbol_id: symbol.id.symbol_id,
                        },
                    );
                }

                merged.files.push(IndexedFile {
                    id: new_file_id,
                    metadata: file.metadata.clone(),
                    symbol_start,
                    symbol_count: file.symbol_count,
                    non_declaration_callable_fragments: file.non_declaration_callable_fragments,
                });

                for symbol in index.symbols_for_file(file) {
                    let mut remapped = symbol.clone();
                    remapped.id = remapped_ids
                        .get(&symbol.id)
                        .copied()
                        .expect("merged symbol id should be remapped");
                    remapped.parent = symbol
                        .parent
                        .and_then(|parent| remapped_ids.get(&parent).copied());
                    merged.symbols.push(remapped);
                }
            }
        }

        merged.rebuild_lookup_maps();
        merged
    }

    pub fn add_catalog<'source>(&mut self, catalog: &SymbolCatalog<'source>) -> SourceFileId {
        let file_id = SourceFileId(self.files.len());
        let symbol_start = self.symbols.len();

        self.files.push(IndexedFile {
            id: file_id,
            metadata: catalog.metadata().clone(),
            symbol_start,
            symbol_count: catalog.records().len(),
            non_declaration_callable_fragments: catalog.non_declaration_callable_fragments(),
        });

        for record in catalog.records() {
            let id = GlobalSymbolId {
                file_id,
                symbol_id: record.id,
            };
            let parent = record
                .parent
                .map(|symbol_id| GlobalSymbolId { file_id, symbol_id });
            let name = catalog.record_name(record).map(str::to_string);
            let symbol = IndexedSymbol {
                id,
                parent,
                kind: record.kind,
                name,
                span: record.span,
                selection_span: record.selection_span,
                detail: IndexedSymbolDetail {
                    type_text: record
                        .detail
                        .type_text
                        .map(|span| catalog.text(span).to_string()),
                    type_text_span: record.detail.type_text,
                    return_type_text: record
                        .detail
                        .return_type_text
                        .map(|span| catalog.text(span).to_string()),
                    return_type_text_span: record.detail.return_type_text,
                    base_type: record
                        .detail
                        .base_type
                        .map(|span| catalog.text(span).to_string()),
                    base_type_span: record.detail.base_type,
                    default_text: record
                        .detail
                        .default_text
                        .map(|span| catalog.text(span).to_string()),
                    default_text_span: record.detail.default_text,
                    enum_value_text: record
                        .detail
                        .enum_value_text
                        .map(|span| catalog.text(span).to_string()),
                    enum_value_text_span: record.detail.enum_value_text,
                },
                attributes: indexed_attributes(catalog, &record.attributes),
                modifiers: record
                    .modifiers
                    .iter()
                    .map(|span| catalog.text(*span).to_string())
                    .collect(),
                doc_comments: record
                    .doc_comments
                    .iter()
                    .map(|comment| IndexedDocComment {
                        kind: comment.kind,
                        text: catalog.text(comment.span).to_string(),
                    })
                    .collect(),
                conditional_context: indexed_conditional_context(
                    catalog,
                    &record.conditional_context,
                ),
                callable_form: record.callable_form,
            };

            self.index_symbol(catalog, &symbol);
            self.symbols.push(symbol);
        }

        file_id
    }

    /// Adds compiler-owned declaration facts without constructing the legacy
    /// `SymbolCatalog`.  This is intentionally an ingestion seam only: callers
    /// continue to choose when a file is parsed and when its immutable facts
    /// are published into an index.
    pub fn add_semantic_file(
        &mut self,
        semantic_file: &SemanticFile,
        metadata: SourceFileMetadata,
    ) -> SourceFileId {
        let file_id = SourceFileId(self.files.len());
        let symbol_start = self.symbols.len();

        self.files.push(IndexedFile {
            id: file_id,
            metadata,
            symbol_start,
            symbol_count: semantic_file.declarations().len(),
            non_declaration_callable_fragments: semantic_file.non_declaration_callable_fragments(),
        });

        for declaration in semantic_file.declarations() {
            let id = GlobalSymbolId {
                file_id,
                symbol_id: SymbolId(declaration.id.0 as usize),
            };
            let parent = declaration.parent.map(|parent| GlobalSymbolId {
                file_id,
                symbol_id: SymbolId(parent.0 as usize),
            });
            self.symbols.push(IndexedSymbol {
                id,
                parent,
                kind: indexed_symbol_kind(declaration.kind),
                name: declaration.name.as_ref().map(|value| value.text.clone()),
                span: declaration.span,
                selection_span: declaration.selection_span,
                detail: IndexedSymbolDetail {
                    type_text: declaration
                        .detail
                        .type_text
                        .as_ref()
                        .map(|value| value.text.clone()),
                    type_text_span: declaration
                        .detail
                        .type_text
                        .as_ref()
                        .map(|value| value.span),
                    return_type_text: declaration
                        .detail
                        .return_type
                        .as_ref()
                        .map(|value| value.text.clone()),
                    return_type_text_span: declaration
                        .detail
                        .return_type
                        .as_ref()
                        .map(|value| value.span),
                    base_type: declaration
                        .detail
                        .base_type
                        .as_ref()
                        .map(|value| value.text.clone()),
                    base_type_span: declaration
                        .detail
                        .base_type
                        .as_ref()
                        .map(|value| value.span),
                    default_text: declaration
                        .detail
                        .default_value
                        .as_ref()
                        .map(|value| value.text.clone()),
                    default_text_span: declaration
                        .detail
                        .default_value
                        .as_ref()
                        .map(|value| value.span),
                    enum_value_text: declaration
                        .detail
                        .enum_value
                        .as_ref()
                        .map(|value| value.text.clone()),
                    enum_value_text_span: declaration
                        .detail
                        .enum_value
                        .as_ref()
                        .map(|value| value.span),
                },
                attributes: declaration
                    .attributes
                    .iter()
                    .map(|attribute| IndexedAttribute {
                        name: semantic_attribute_name(&attribute.text).map(str::to_owned),
                        text: attribute.text.clone(),
                    })
                    .collect(),
                modifiers: declaration
                    .modifiers
                    .iter()
                    .map(|modifier| modifier.text.clone())
                    .collect(),
                doc_comments: declaration
                    .doc_comments
                    .iter()
                    .map(|comment| IndexedDocComment {
                        kind: match comment.kind {
                            SemanticDocCommentKind::Line => DocCommentKind::Line,
                            SemanticDocCommentKind::Block => DocCommentKind::Block,
                        },
                        text: comment.text.clone(),
                    })
                    .collect(),
                conditional_context: semantic_file
                    .conditional_context(declaration.conditional_context)
                    .iter()
                    .map(|branch| IndexedConditionalBranch {
                        kind: indexed_conditional_kind(branch.kind),
                        condition: branch
                            .condition
                            .as_ref()
                            .map(|condition| condition.text.clone()),
                    })
                    .collect(),
                callable_form: declaration.callable_form.map(indexed_callable_form),
            });
        }

        self.rebuild_lookup_maps();
        file_id
    }

    /// Adds a validated, serialized compiler contribution. Production
    /// workspace and game-data ingestion uses this boundary so the index never
    /// reconstructs facts from the legacy catalog or source text.
    pub fn add_file_contribution(
        &mut self,
        contribution: &FileContribution,
        metadata: SourceFileMetadata,
    ) -> Result<SourceFileId, FileContributionValidationError> {
        let mut file_ids =
            self.add_file_contributions(std::iter::once((contribution, metadata)))?;
        Ok(file_ids
            .pop()
            .expect("one contribution must produce one source file id"))
    }

    /// Adds a complete group of validated compiler contributions and rebuilds
    /// global lookup maps once after the group is visible. This is the bulk
    /// construction boundary for a cold index build; per-file updates should
    /// continue to use [`Self::add_file_contribution`].
    pub fn add_file_contributions<'contribution>(
        &mut self,
        contributions: impl IntoIterator<Item = (&'contribution FileContribution, SourceFileMetadata)>,
    ) -> Result<Vec<SourceFileId>, FileContributionValidationError> {
        let contributions = contributions.into_iter().collect::<Vec<_>>();
        for (contribution, _) in &contributions {
            contribution.validate()?;
        }

        self.files.reserve(contributions.len());
        self.symbols.reserve(
            contributions
                .iter()
                .map(|(contribution, _)| contribution.symbols.len())
                .sum(),
        );

        let mut file_ids = Vec::with_capacity(contributions.len());
        for (contribution, metadata) in contributions {
            file_ids.push(self.append_file_contribution(contribution, metadata));
        }
        if !file_ids.is_empty() {
            self.rebuild_lookup_maps();
        }
        Ok(file_ids)
    }

    /// Consumes a complete group of already-validated compiler contributions.
    ///
    /// Cache loading owns its decoded canonical records, so this avoids cloning
    /// every public string into a short-lived `FileContribution` and then into
    /// the runtime index.  Keep the borrowed API above for live semantic files
    /// that remain owned by their caller.
    pub fn add_owned_file_contributions(
        &mut self,
        contributions: impl IntoIterator<Item = (FileContribution, SourceFileMetadata)>,
    ) -> Result<Vec<SourceFileId>, FileContributionValidationError> {
        let contributions = contributions.into_iter().collect::<Vec<_>>();
        for (contribution, _) in &contributions {
            contribution.validate()?;
        }

        self.files.reserve(contributions.len());
        self.symbols.reserve(
            contributions
                .iter()
                .map(|(contribution, _)| contribution.symbols.len())
                .sum(),
        );

        let mut file_ids = Vec::with_capacity(contributions.len());
        for (contribution, metadata) in contributions {
            file_ids.push(self.append_owned_file_contribution(contribution, metadata));
        }
        if !file_ids.is_empty() {
            self.rebuild_lookup_maps();
        }
        Ok(file_ids)
    }

    fn append_file_contribution(
        &mut self,
        contribution: &FileContribution,
        metadata: SourceFileMetadata,
    ) -> SourceFileId {
        let file_id = SourceFileId(self.files.len());
        let symbol_start = self.symbols.len();
        self.files.push(IndexedFile {
            id: file_id,
            metadata,
            symbol_start,
            symbol_count: contribution.symbols.len(),
            non_declaration_callable_fragments: contribution.non_declaration_callable_fragments,
        });

        for declaration in &contribution.symbols {
            let id = GlobalSymbolId {
                file_id,
                symbol_id: SymbolId(declaration.id.0 as usize),
            };
            let parent = declaration.parent.map(|parent| GlobalSymbolId {
                file_id,
                symbol_id: SymbolId(parent.0 as usize),
            });
            self.symbols.push(IndexedSymbol {
                id,
                parent,
                kind: indexed_symbol_kind(declaration.kind),
                name: declaration.name.clone(),
                span: declaration.span,
                selection_span: declaration.selection_span,
                detail: IndexedSymbolDetail {
                    type_text: declaration
                        .detail
                        .type_text
                        .as_ref()
                        .map(|value| value.text.clone()),
                    type_text_span: declaration
                        .detail
                        .type_text
                        .as_ref()
                        .and_then(|value| value.span),
                    return_type_text: declaration
                        .detail
                        .return_type
                        .as_ref()
                        .map(|value| value.text.clone()),
                    return_type_text_span: declaration
                        .detail
                        .return_type
                        .as_ref()
                        .and_then(|value| value.span),
                    base_type: declaration
                        .detail
                        .base_type
                        .as_ref()
                        .map(|value| value.text.clone()),
                    base_type_span: declaration
                        .detail
                        .base_type
                        .as_ref()
                        .and_then(|value| value.span),
                    default_text: declaration
                        .detail
                        .default_value
                        .as_ref()
                        .map(|value| value.text.clone()),
                    default_text_span: declaration
                        .detail
                        .default_value
                        .as_ref()
                        .and_then(|value| value.span),
                    enum_value_text: declaration
                        .detail
                        .enum_value
                        .as_ref()
                        .map(|value| value.text.clone()),
                    enum_value_text_span: declaration
                        .detail
                        .enum_value
                        .as_ref()
                        .and_then(|value| value.span),
                },
                attributes: declaration
                    .attributes
                    .iter()
                    .map(|attribute| IndexedAttribute {
                        name: semantic_attribute_name(&attribute.text).map(str::to_owned),
                        text: attribute.text.clone(),
                    })
                    .collect(),
                modifiers: declaration
                    .modifiers
                    .iter()
                    .map(|value| value.text.clone())
                    .collect(),
                doc_comments: declaration
                    .doc_comments
                    .iter()
                    .map(|comment| IndexedDocComment {
                        kind: match comment.kind {
                            SemanticDocCommentKind::Line => DocCommentKind::Line,
                            SemanticDocCommentKind::Block => DocCommentKind::Block,
                        },
                        text: comment.text.clone(),
                    })
                    .collect(),
                conditional_context: declaration
                    .conditional_context
                    .iter()
                    .map(|branch| IndexedConditionalBranch {
                        kind: indexed_conditional_kind(branch.kind),
                        condition: branch.condition.as_ref().map(|value| value.text.clone()),
                    })
                    .collect(),
                callable_form: declaration.callable_form.map(indexed_callable_form),
            });
        }

        file_id
    }

    fn append_owned_file_contribution(
        &mut self,
        contribution: FileContribution,
        metadata: SourceFileMetadata,
    ) -> SourceFileId {
        let file_id = SourceFileId(self.files.len());
        let symbol_start = self.symbols.len();
        let FileContribution {
            non_declaration_callable_fragments,
            symbols,
            ..
        } = contribution;
        self.files.push(IndexedFile {
            id: file_id,
            metadata,
            symbol_start,
            symbol_count: symbols.len(),
            non_declaration_callable_fragments,
        });

        for declaration in symbols {
            let PublicSymbol {
                id,
                parent,
                kind,
                name,
                detail,
                span,
                selection_span,
                modifiers,
                attributes,
                doc_comments,
                conditional_context,
                callable_form,
                ..
            } = declaration;
            let id = GlobalSymbolId {
                file_id,
                symbol_id: SymbolId(id.0 as usize),
            };
            let parent = parent.map(|parent| GlobalSymbolId {
                file_id,
                symbol_id: SymbolId(parent.0 as usize),
            });
            let crate::semantic_file::PublicSymbolDetail {
                type_text,
                return_type,
                base_type,
                default_value,
                enum_value,
            } = detail;
            let (type_text, type_text_span) = owned_public_text(type_text);
            let (return_type_text, return_type_text_span) = owned_public_text(return_type);
            let (base_type, base_type_span) = owned_public_text(base_type);
            let (default_text, default_text_span) = owned_public_text(default_value);
            let (enum_value_text, enum_value_text_span) = owned_public_text(enum_value);
            self.symbols.push(IndexedSymbol {
                id,
                parent,
                kind: indexed_symbol_kind(kind),
                name,
                span,
                selection_span,
                detail: IndexedSymbolDetail {
                    type_text,
                    type_text_span,
                    return_type_text,
                    return_type_text_span,
                    base_type,
                    base_type_span,
                    default_text,
                    default_text_span,
                    enum_value_text,
                    enum_value_text_span,
                },
                attributes: attributes
                    .into_iter()
                    .map(|attribute| IndexedAttribute {
                        name: semantic_attribute_name(&attribute.text).map(str::to_owned),
                        text: attribute.text,
                    })
                    .collect(),
                modifiers: modifiers.into_iter().map(|value| value.text).collect(),
                doc_comments: doc_comments
                    .into_iter()
                    .map(|comment| IndexedDocComment {
                        kind: match comment.kind {
                            SemanticDocCommentKind::Line => DocCommentKind::Line,
                            SemanticDocCommentKind::Block => DocCommentKind::Block,
                        },
                        text: comment.text,
                    })
                    .collect(),
                conditional_context: conditional_context
                    .into_iter()
                    .map(|branch| IndexedConditionalBranch {
                        kind: indexed_conditional_kind(branch.kind),
                        condition: branch.condition.map(|value| value.text),
                    })
                    .collect(),
                callable_form: callable_form.map(indexed_callable_form),
            });
        }

        file_id
    }

    #[cfg(test)]
    pub fn lookup_map_rebuild_count(&self) -> usize {
        self.lookup_map_rebuild_count
    }

    pub fn files(&self) -> &[IndexedFile] {
        &self.files
    }

    pub fn symbols(&self) -> &[IndexedSymbol] {
        &self.symbols
    }

    pub fn without_local_variables(&self) -> Self {
        self.without_symbol_kind(SymbolKind::LocalVariable)
    }

    pub fn compact_for_runtime_cache(&self) -> Self {
        let mut compact = self.without_local_variables();
        compact.strip_detail_spans();
        compact
    }

    fn strip_detail_spans(&mut self) {
        for symbol in &mut self.symbols {
            symbol.detail.type_text_span = None;
            symbol.detail.return_type_text_span = None;
            symbol.detail.base_type_span = None;
            symbol.detail.default_text_span = None;
            symbol.detail.enum_value_text_span = None;
        }
    }

    fn without_symbol_kind(&self, excluded_kind: SymbolKind) -> Self {
        let mut filtered = Self::default();
        let mut remapped_ids = BTreeMap::<GlobalSymbolId, GlobalSymbolId>::new();
        let mut next_symbol_start = 0;

        for file in &self.files {
            let new_file_id = SourceFileId(filtered.files.len());
            let symbol_start = next_symbol_start;
            let mut symbol_count = 0;

            for symbol in self.symbols_for_file(file) {
                if symbol.kind == excluded_kind {
                    continue;
                }

                let new_id = GlobalSymbolId {
                    file_id: new_file_id,
                    symbol_id: SymbolId(symbol_count),
                };
                remapped_ids.insert(symbol.id, new_id);
                symbol_count += 1;
            }
            next_symbol_start += symbol_count;

            filtered.files.push(IndexedFile {
                id: new_file_id,
                metadata: file.metadata.clone(),
                symbol_start,
                symbol_count,
                non_declaration_callable_fragments: file.non_declaration_callable_fragments,
            });
        }

        for file in &self.files {
            for symbol in self.symbols_for_file(file) {
                if symbol.kind == excluded_kind {
                    continue;
                }

                let Some(new_id) = remapped_ids.get(&symbol.id).copied() else {
                    continue;
                };
                let mut remapped = symbol.clone();
                remapped.id = new_id;
                remapped.parent = symbol
                    .parent
                    .and_then(|parent| remapped_ids.get(&parent).copied());
                filtered.symbols.push(remapped);
            }
        }

        filtered.rebuild_lookup_maps();
        filtered
    }

    pub fn file(&self, id: SourceFileId) -> Option<&IndexedFile> {
        self.files.get(id.0)
    }

    pub fn symbol(&self, id: GlobalSymbolId) -> Option<&IndexedSymbol> {
        let file = self.file(id.file_id)?;
        let local_index = id.symbol_id.0;
        if local_index >= file.symbol_count {
            return None;
        }
        self.symbols.get(file.symbol_start + local_index)
    }

    pub fn symbols_for_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.by_name.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn top_level_symbols_for_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.top_level_by_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn preferred_symbols_for_name(&self, name: &str) -> Vec<GlobalSymbolId> {
        self.preferred_from_symbols(self.symbols_for_name(name))
    }

    pub fn preferred_top_level_symbols_for_name(&self, name: &str) -> Vec<GlobalSymbolId> {
        self.preferred_from_symbols(self.top_level_symbols_for_name(name))
    }

    pub fn preferred_classes_by_name(&self, name: &str) -> Vec<GlobalSymbolId> {
        self.preferred_from_symbols(self.classes_by_name(name))
    }

    pub fn preferred_typedefs_by_name(&self, name: &str) -> Vec<GlobalSymbolId> {
        self.preferred_from_symbols(self.typedefs_by_name(name))
    }

    pub fn preferred_functions_by_name(&self, name: &str) -> Vec<GlobalSymbolId> {
        self.preferred_from_symbols(self.functions_by_name(name))
    }

    pub fn preferred_from_symbols(&self, symbols: &[GlobalSymbolId]) -> Vec<GlobalSymbolId> {
        let mut symbols = symbols.to_vec();
        symbols.sort_by(|left, right| self.compare_symbol_preference(*left, *right));
        symbols
    }

    pub fn symbols_for_kind(&self, kind: SymbolKind) -> &[GlobalSymbolId] {
        self.by_kind.get(&kind).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn children(&self, parent: GlobalSymbolId) -> &[GlobalSymbolId] {
        self.children.get(&parent).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn classes_by_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.classes_by_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn typedefs_by_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.typedefs_by_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn functions_by_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.functions_by_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn methods_by_owner_name(&self, owner: &str, name: &str) -> &[GlobalSymbolId] {
        self.methods_by_owner_name
            .get(&(owner.to_string(), name.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn method_owner_name_groups(&self) -> &BTreeMap<(String, String), Vec<GlobalSymbolId>> {
        &self.methods_by_owner_name
    }

    pub fn fields_by_owner_name(&self, owner: &str, name: &str) -> &[GlobalSymbolId] {
        self.fields_by_owner_name
            .get(&(owner.to_string(), name.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn members_by_owner(&self, owner: &str) -> &[GlobalSymbolId] {
        self.members_by_owner
            .get(owner)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn direct_members_by_owner(&self, owner: &str) -> &[GlobalSymbolId] {
        self.members_by_owner(owner)
    }

    pub fn raw_members_for_class_including_bases(&self, owner: &str) -> Vec<GlobalSymbolId> {
        self.member_segments_for_class_including_bases(owner)
            .into_iter()
            .flatten()
            .collect()
    }

    pub fn raw_completion_members_for_owner_name(&self, owner: &str) -> CompletionMemberLookup {
        let member_segments = self.member_segments_for_class_including_bases(owner);
        self.completion_from_member_segments(member_segments)
    }

    pub fn completion_members_for_preferred_class(&self, owner: &str) -> CompletionMemberLookup {
        let member_segments = self.preferred_class_member_segments_including_bases(owner);
        self.completion_from_member_segments(member_segments)
    }

    fn completion_from_member_segments(
        &self,
        member_segments: Vec<Vec<GlobalSymbolId>>,
    ) -> CompletionMemberLookup {
        let raw_candidates = member_segments
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        let mut members = Vec::new();
        let mut kept_by_key = BTreeMap::<String, GlobalSymbolId>::new();
        let mut shadow_group_by_key = BTreeMap::<String, usize>::new();
        let mut shadowed_groups = Vec::new();

        for segment in member_segments {
            let mut ids_by_key = BTreeMap::<String, Vec<GlobalSymbolId>>::new();
            let mut key_order = Vec::<String>::new();
            for id in segment {
                let key = self.completion_member_key(id);
                if !ids_by_key.contains_key(&key) {
                    key_order.push(key.clone());
                }
                ids_by_key.entry(key).or_default().push(id);
            }

            for key in key_order {
                let ids = ids_by_key.remove(&key).unwrap_or_default();
                let Some(preferred) = self.preferred_from_symbols(&ids).first().copied() else {
                    continue;
                };

                if let Some(kept) = kept_by_key.get(&key).copied() {
                    self.push_shadowed_members(
                        &mut shadowed_groups,
                        &mut shadow_group_by_key,
                        key,
                        kept,
                        ids,
                    );
                } else {
                    kept_by_key.insert(key.clone(), preferred);
                    members.push(preferred);
                    self.push_shadowed_members(
                        &mut shadowed_groups,
                        &mut shadow_group_by_key,
                        key,
                        preferred,
                        ids.into_iter().filter(|id| *id != preferred).collect(),
                    );
                }
            }
        }

        CompletionMemberLookup {
            raw_candidates,
            members,
            shadowed_groups,
        }
    }

    fn push_shadowed_members(
        &self,
        shadowed_groups: &mut Vec<MemberShadowGroup>,
        shadow_group_by_key: &mut BTreeMap<String, usize>,
        key: String,
        kept: GlobalSymbolId,
        shadowed: Vec<GlobalSymbolId>,
    ) {
        if shadowed.is_empty() {
            return;
        }

        let group_index = *shadow_group_by_key.entry(key.clone()).or_insert_with(|| {
            shadowed_groups.push(MemberShadowGroup {
                key,
                kept,
                shadowed: Vec::new(),
            });
            shadowed_groups.len() - 1
        });
        shadowed_groups[group_index].shadowed.extend(shadowed);
    }

    pub fn callable_signature(&self, id: GlobalSymbolId) -> Option<String> {
        let symbol = self.symbol(id)?;
        let name = symbol.name.as_deref()?;
        let parameters = self.callable_parameter_text(id);

        match symbol.kind {
            SymbolKind::Function => {
                let return_type = symbol
                    .detail
                    .return_type_text
                    .as_deref()
                    .unwrap_or("<unknown>");
                Some(format!("{name}({parameters}) -> {return_type}"))
            }
            SymbolKind::Method => {
                let owner = self.callable_owner_name(symbol)?;
                let return_type = symbol
                    .detail
                    .return_type_text
                    .as_deref()
                    .unwrap_or("<unknown>");
                Some(format!("{owner}.{name}({parameters}) -> {return_type}"))
            }
            SymbolKind::Constructor => {
                let owner = self.callable_owner_name(symbol)?;
                Some(format!("{owner}({parameters})"))
            }
            SymbolKind::Destructor => {
                let owner = self.callable_owner_name(symbol)?;
                Some(format!("~{owner}({parameters})"))
            }
            _ => None,
        }
    }

    fn callable_parameter_text(&self, id: GlobalSymbolId) -> String {
        self.children(id)
            .iter()
            .filter_map(|child_id| self.symbol(*child_id))
            .filter(|child| child.kind == SymbolKind::Parameter)
            .map(parameter_signature_text)
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn callable_owner_name<'a>(&'a self, symbol: &'a IndexedSymbol) -> Option<&'a str> {
        symbol
            .parent
            .and_then(|parent| self.symbol(parent))
            .and_then(|parent| parent.name.as_deref())
    }

    pub fn names(&self) -> &BTreeMap<String, Vec<GlobalSymbolId>> {
        &self.by_name
    }

    pub fn top_level_names(&self) -> &BTreeMap<String, Vec<GlobalSymbolId>> {
        &self.top_level_by_name
    }

    pub fn duplicate_names(&self) -> Vec<(&str, &[GlobalSymbolId])> {
        self.by_name
            .iter()
            .filter(|(_, symbols)| symbols.len() > 1)
            .map(|(name, symbols)| (name.as_str(), symbols.as_slice()))
            .collect()
    }

    pub fn duplicate_top_level_names(&self) -> Vec<(&str, &[GlobalSymbolId])> {
        self.top_level_by_name
            .iter()
            .filter(|(_, symbols)| symbols.len() > 1)
            .map(|(name, symbols)| (name.as_str(), symbols.as_slice()))
            .collect()
    }

    pub fn map_counts(&self) -> IndexMapCounts {
        IndexMapCounts {
            names: self.by_name.len(),
            name_entries: map_entry_count(&self.by_name),
            top_level_names: self.top_level_by_name.len(),
            top_level_name_entries: map_entry_count(&self.top_level_by_name),
            kinds: self.by_kind.len(),
            kind_entries: map_entry_count(&self.by_kind),
            class_names: self.classes_by_name.len(),
            class_name_entries: map_entry_count(&self.classes_by_name),
            typedef_names: self.typedefs_by_name.len(),
            typedef_name_entries: map_entry_count(&self.typedefs_by_name),
            function_names: self.functions_by_name.len(),
            function_name_entries: map_entry_count(&self.functions_by_name),
            method_owner_names: self.methods_by_owner_name.len(),
            method_owner_name_entries: map_entry_count(&self.methods_by_owner_name),
            field_owner_names: self.fields_by_owner_name.len(),
            field_owner_name_entries: map_entry_count(&self.fields_by_owner_name),
            member_owners: self.members_by_owner.len(),
            member_owner_entries: map_entry_count(&self.members_by_owner),
            parent_symbols: self.children.len(),
            child_entries: map_entry_count(&self.children),
        }
    }

    pub fn source_kind_counts(&self) -> BTreeMap<SourceKind, usize> {
        let mut counts = BTreeMap::new();
        for file in &self.files {
            *counts.entry(file.metadata.kind).or_default() += 1;
        }
        counts
    }

    fn symbols_for_file(&self, file: &IndexedFile) -> &[IndexedSymbol] {
        &self.symbols[file.symbol_start..file.symbol_start + file.symbol_count]
    }

    fn rebuild_lookup_maps(&mut self) {
        #[cfg(test)]
        {
            self.lookup_map_rebuild_count += 1;
        }
        let mut by_name = BTreeMap::<String, Vec<GlobalSymbolId>>::new();
        let mut top_level_by_name = BTreeMap::<String, Vec<GlobalSymbolId>>::new();
        let mut by_kind = BTreeMap::<SymbolKind, Vec<GlobalSymbolId>>::new();
        let mut children = BTreeMap::<GlobalSymbolId, Vec<GlobalSymbolId>>::new();
        let mut classes_by_name = BTreeMap::<String, Vec<GlobalSymbolId>>::new();
        let mut typedefs_by_name = BTreeMap::<String, Vec<GlobalSymbolId>>::new();
        let mut functions_by_name = BTreeMap::<String, Vec<GlobalSymbolId>>::new();
        let mut methods_by_owner_name = BTreeMap::<(String, String), Vec<GlobalSymbolId>>::new();
        let mut fields_by_owner_name = BTreeMap::<(String, String), Vec<GlobalSymbolId>>::new();
        let mut members_by_owner = BTreeMap::<String, Vec<GlobalSymbolId>>::new();

        for symbol in &self.symbols {
            by_kind.entry(symbol.kind).or_default().push(symbol.id);

            if let Some(parent) = symbol.parent {
                children.entry(parent).or_default().push(symbol.id);
            }

            let Some(name) = &symbol.name else {
                continue;
            };

            by_name.entry(name.clone()).or_default().push(symbol.id);

            if symbol.parent.is_none() {
                top_level_by_name
                    .entry(name.clone())
                    .or_default()
                    .push(symbol.id);
            }

            match symbol.kind {
                SymbolKind::Class => {
                    classes_by_name
                        .entry(name.clone())
                        .or_default()
                        .push(symbol.id);
                }
                SymbolKind::Typedef => {
                    typedefs_by_name
                        .entry(name.clone())
                        .or_default()
                        .push(symbol.id);
                }
                SymbolKind::Function => {
                    functions_by_name
                        .entry(name.clone())
                        .or_default()
                        .push(symbol.id);
                }
                SymbolKind::Method => {
                    if let Some(owner) = self.parent_class_name(symbol).map(str::to_string) {
                        methods_by_owner_name
                            .entry((owner, name.clone()))
                            .or_default()
                            .push(symbol.id);
                    }
                }
                _ => {}
            }

            if is_class_member_kind(symbol.kind) {
                if let Some(owner) = self.parent_class_name(symbol).map(str::to_string) {
                    members_by_owner
                        .entry(owner.clone())
                        .or_default()
                        .push(symbol.id);

                    if symbol.kind == SymbolKind::Field {
                        fields_by_owner_name
                            .entry((owner, name.clone()))
                            .or_default()
                            .push(symbol.id);
                    }
                }
            }
        }

        self.by_name = by_name;
        self.top_level_by_name = top_level_by_name;
        self.by_kind = by_kind;
        self.children = children;
        self.classes_by_name = classes_by_name;
        self.typedefs_by_name = typedefs_by_name;
        self.functions_by_name = functions_by_name;
        self.methods_by_owner_name = methods_by_owner_name;
        self.fields_by_owner_name = fields_by_owner_name;
        self.members_by_owner = members_by_owner;
    }

    fn parent_class_name<'a>(&'a self, symbol: &'a IndexedSymbol) -> Option<&'a str> {
        let parent = symbol.parent?;
        let parent_symbol = self.symbol(parent)?;
        if parent_symbol.kind != SymbolKind::Class {
            return None;
        }
        parent_symbol.name.as_deref()
    }

    fn index_symbol<'source>(&mut self, catalog: &SymbolCatalog<'source>, symbol: &IndexedSymbol) {
        self.by_kind.entry(symbol.kind).or_default().push(symbol.id);

        if let Some(parent) = symbol.parent {
            self.children.entry(parent).or_default().push(symbol.id);
        }

        let Some(name) = &symbol.name else {
            return;
        };

        self.by_name
            .entry(name.clone())
            .or_default()
            .push(symbol.id);

        if symbol.parent.is_none() {
            self.top_level_by_name
                .entry(name.clone())
                .or_default()
                .push(symbol.id);
        }

        match symbol.kind {
            SymbolKind::Class => {
                self.classes_by_name
                    .entry(name.clone())
                    .or_default()
                    .push(symbol.id);
            }
            SymbolKind::Typedef => {
                self.typedefs_by_name
                    .entry(name.clone())
                    .or_default()
                    .push(symbol.id);
            }
            SymbolKind::Function => {
                self.functions_by_name
                    .entry(name.clone())
                    .or_default()
                    .push(symbol.id);
            }
            SymbolKind::Method => {
                if let Some(owner) = symbol
                    .parent
                    .and_then(|parent| catalog.record(parent.symbol_id))
                    .and_then(|parent| catalog.record_name(parent))
                {
                    self.methods_by_owner_name
                        .entry((owner.to_string(), name.clone()))
                        .or_default()
                        .push(symbol.id);
                }
            }
            _ => {}
        }

        if is_class_member_kind(symbol.kind) {
            if let Some(owner) = owner_class_name(catalog, symbol) {
                self.members_by_owner
                    .entry(owner.to_string())
                    .or_default()
                    .push(symbol.id);

                if symbol.kind == SymbolKind::Field {
                    self.fields_by_owner_name
                        .entry((owner.to_string(), name.clone()))
                        .or_default()
                        .push(symbol.id);
                }
            }
        }
    }

    fn compare_symbol_preference(
        &self,
        left: GlobalSymbolId,
        right: GlobalSymbolId,
    ) -> std::cmp::Ordering {
        let left_file = self.file(left.file_id);
        let right_file = self.file(right.file_id);
        let left_priority = left_file
            .map(|file| file.metadata.priority)
            .unwrap_or_default();
        let right_priority = right_file
            .map(|file| file.metadata.priority)
            .unwrap_or_default();

        right_priority
            .cmp(&left_priority)
            .then_with(|| left.file_id.cmp(&right.file_id))
            .then_with(|| left.symbol_id.cmp(&right.symbol_id))
    }

    fn member_segments_for_class_including_bases(&self, owner: &str) -> Vec<Vec<GlobalSymbolId>> {
        let mut segments = Vec::new();
        let mut visited = BTreeSet::new();
        self.add_member_segments_for_class_including_bases(owner, &mut visited, &mut segments);
        segments
    }

    fn preferred_class_member_segments_including_bases(
        &self,
        owner: &str,
    ) -> Vec<Vec<GlobalSymbolId>> {
        let mut segments = Vec::new();
        let mut visited = BTreeSet::new();
        self.add_preferred_class_member_segments_including_bases(
            owner,
            &mut visited,
            &mut segments,
        );
        segments
    }

    fn add_member_segments_for_class_including_bases(
        &self,
        owner: &str,
        visited: &mut BTreeSet<String>,
        segments: &mut Vec<Vec<GlobalSymbolId>>,
    ) {
        if !visited.insert(owner.to_string()) {
            return;
        }

        segments.push(self.members_by_owner(owner).to_vec());

        let Some(base_name) = self.preferred_class_base_name(owner) else {
            return;
        };
        self.add_member_segments_for_class_including_bases(&base_name, visited, segments);
    }

    fn add_preferred_class_member_segments_including_bases(
        &self,
        owner: &str,
        visited: &mut BTreeSet<String>,
        segments: &mut Vec<Vec<GlobalSymbolId>>,
    ) {
        if !visited.insert(owner.to_string()) {
            return;
        }

        let classes = self.preferred_classes_by_name(owner);
        for class_id in &classes {
            segments.push(self.direct_members_for_class_declaration(*class_id));
        }

        let Some(base_name) = self.first_class_base_name(&classes) else {
            return;
        };
        self.add_preferred_class_member_segments_including_bases(&base_name, visited, segments);
    }

    fn preferred_class_base_name(&self, owner: &str) -> Option<String> {
        let class_id = self
            .preferred_from_symbols(self.classes_by_name(owner))
            .first()
            .copied()?;
        self.class_base_name(class_id)
    }

    fn first_class_base_name(&self, class_ids: &[GlobalSymbolId]) -> Option<String> {
        class_ids
            .iter()
            .find_map(|class_id| self.class_base_name(*class_id))
    }

    fn class_base_name(&self, class_id: GlobalSymbolId) -> Option<String> {
        let class = self.symbol(class_id)?;
        let base = class.detail.base_type.as_deref()?.trim();
        if base.is_empty() {
            None
        } else {
            Some(base.to_string())
        }
    }

    fn direct_members_for_class_declaration(
        &self,
        class_id: GlobalSymbolId,
    ) -> Vec<GlobalSymbolId> {
        self.children(class_id)
            .iter()
            .copied()
            .filter(|child_id| {
                self.symbol(*child_id)
                    .is_some_and(|symbol| is_class_member_kind(symbol.kind))
            })
            .collect()
    }

    pub fn completion_member_key(&self, id: GlobalSymbolId) -> String {
        let Some(symbol) = self.symbol(id) else {
            return format!("Missing:{}:{}", id.file_id.0, id.symbol_id.0);
        };
        let name = symbol.name.as_deref().unwrap_or("<unknown>");

        match symbol.kind {
            SymbolKind::Field => format!("Field {name}"),
            SymbolKind::Method => format!(
                "Method {name}({}) -> {}",
                self.parameter_type_shape(id),
                symbol
                    .detail
                    .return_type_text
                    .as_deref()
                    .unwrap_or("<unknown>")
            ),
            SymbolKind::Constructor => {
                format!("Constructor {name}({})", self.parameter_type_shape(id))
            }
            SymbolKind::Destructor => {
                format!("Destructor {name}({})", self.parameter_type_shape(id))
            }
            _ => format!(
                "{} {name} #{}:{}",
                symbol_kind_key(symbol.kind),
                id.file_id.0,
                id.symbol_id.0
            ),
        }
    }

    fn parameter_type_shape(&self, id: GlobalSymbolId) -> String {
        self.children(id)
            .iter()
            .filter_map(|child_id| self.symbol(*child_id))
            .filter(|child| child.kind == SymbolKind::Parameter)
            .map(|parameter| {
                parameter
                    .detail
                    .type_text
                    .as_deref()
                    .unwrap_or("<unknown>")
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn map_entry_count<K>(map: &BTreeMap<K, Vec<GlobalSymbolId>>) -> usize {
    map.values().map(Vec::len).sum()
}

fn owner_class_name<'source>(
    catalog: &'source SymbolCatalog<'source>,
    symbol: &IndexedSymbol,
) -> Option<&'source str> {
    let parent = symbol.parent?;
    let parent_record = catalog.record(parent.symbol_id)?;
    if parent_record.kind != SymbolKind::Class {
        return None;
    }
    catalog.record_name(parent_record)
}

fn indexed_attributes<'source>(
    catalog: &SymbolCatalog<'source>,
    attributes: &[TextSpan],
) -> Vec<IndexedAttribute> {
    attributes
        .iter()
        .map(|span| IndexedAttribute {
            name: catalog.attribute_name(*span).map(str::to_string),
            text: indexed_attribute_text(catalog, *span),
        })
        .collect()
}

fn indexed_attribute_text<'source>(catalog: &SymbolCatalog<'source>, span: TextSpan) -> String {
    let source = catalog.source();
    let bytes = source.as_bytes();
    let start = if span.start > 0 && bytes[span.start - 1] == b'[' {
        span.start - 1
    } else {
        span.start
    };
    let end = if span.end < bytes.len() && bytes[span.end] == b']' {
        span.end + 1
    } else {
        span.end
    };
    source[start..end].to_string()
}

fn indexed_conditional_context<'source>(
    catalog: &SymbolCatalog<'source>,
    context: &[ConditionalBranch],
) -> Vec<IndexedConditionalBranch> {
    context
        .iter()
        .map(|branch| IndexedConditionalBranch {
            kind: branch.kind,
            condition: branch.condition.map(|span| catalog.text(span).to_string()),
        })
        .collect()
}

fn indexed_symbol_kind(kind: SemanticDeclarationKind) -> SymbolKind {
    match kind {
        SemanticDeclarationKind::Class => SymbolKind::Class,
        SemanticDeclarationKind::TypeParameter => SymbolKind::TypeParameter,
        SemanticDeclarationKind::Enum => SymbolKind::Enum,
        SemanticDeclarationKind::EnumMember => SymbolKind::EnumMember,
        SemanticDeclarationKind::Typedef => SymbolKind::Typedef,
        SemanticDeclarationKind::Function => SymbolKind::Function,
        SemanticDeclarationKind::GlobalField => SymbolKind::GlobalField,
        SemanticDeclarationKind::Field => SymbolKind::Field,
        SemanticDeclarationKind::Method => SymbolKind::Method,
        SemanticDeclarationKind::Constructor => SymbolKind::Constructor,
        SemanticDeclarationKind::Destructor => SymbolKind::Destructor,
        SemanticDeclarationKind::Parameter => SymbolKind::Parameter,
        SemanticDeclarationKind::LocalVariable => SymbolKind::LocalVariable,
        SemanticDeclarationKind::PreprocessorMacro => SymbolKind::PreprocessorMacro,
    }
}

fn indexed_callable_form(form: SemanticCallableForm) -> CallableForm {
    match form {
        SemanticCallableForm::Implementation => CallableForm::Implementation,
        SemanticCallableForm::Declaration => CallableForm::Declaration,
        SemanticCallableForm::Prototype => CallableForm::Prototype,
    }
}

fn indexed_conditional_kind(kind: SemanticConditionalBranchKind) -> PreprocessorBranchKind {
    match kind {
        SemanticConditionalBranchKind::If => PreprocessorBranchKind::If,
        SemanticConditionalBranchKind::Ifdef => PreprocessorBranchKind::Ifdef,
        SemanticConditionalBranchKind::Ifndef => PreprocessorBranchKind::Ifndef,
        SemanticConditionalBranchKind::Elif => PreprocessorBranchKind::Elif,
        SemanticConditionalBranchKind::Else => PreprocessorBranchKind::Else,
    }
}

fn semantic_attribute_name(text: &str) -> Option<&str> {
    let trimmed = text.trim_start();
    let trimmed = trimmed.strip_prefix('[').unwrap_or(trimmed).trim_start();
    let end = trimmed
        .char_indices()
        .take_while(|(_, value)| value.is_ascii_alphanumeric() || *value == '_')
        .map(|(index, value)| index + value.len_utf8())
        .last()?;
    Some(&trimmed[..end])
}

fn owned_public_text(value: Option<PublicText>) -> (Option<String>, Option<TextSpan>) {
    match value {
        Some(PublicText { span, text }) => (Some(text), span),
        None => (None, None),
    }
}

fn is_class_member_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Field | SymbolKind::Method | SymbolKind::Constructor | SymbolKind::Destructor
    )
}

fn parameter_signature_text(symbol: &IndexedSymbol) -> String {
    let mut value = String::new();
    if !symbol.modifiers.is_empty() {
        value.push_str(&symbol.modifiers.join(" "));
    }
    if let Some(type_text) = &symbol.detail.type_text {
        if !value.is_empty() {
            value.push(' ');
        }
        value.push_str(type_text);
    }
    if let Some(name) = &symbol.name {
        if !value.is_empty() {
            value.push(' ');
        }
        value.push_str(name);
    }
    if value.is_empty() {
        value.push_str("<unknown>");
    }
    if let Some(default_text) = &symbol.detail.default_text {
        value.push_str(" = ");
        value.push_str(default_text);
    }
    value
}

fn symbol_kind_key(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "Class",
        SymbolKind::TypeParameter => "TypeParameter",
        SymbolKind::Enum => "Enum",
        SymbolKind::EnumMember => "EnumMember",
        SymbolKind::Typedef => "Typedef",
        SymbolKind::Function => "Function",
        SymbolKind::GlobalField => "GlobalField",
        SymbolKind::Field => "Field",
        SymbolKind::Method => "Method",
        SymbolKind::Constructor => "Constructor",
        SymbolKind::Destructor => "Destructor",
        SymbolKind::Parameter => "Parameter",
        SymbolKind::LocalVariable => "LocalVariable",
        SymbolKind::PreprocessorMacro => "PreprocessorMacro",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexMapCounts {
    pub names: usize,
    pub name_entries: usize,
    pub top_level_names: usize,
    pub top_level_name_entries: usize,
    pub kinds: usize,
    pub kind_entries: usize,
    pub class_names: usize,
    pub class_name_entries: usize,
    pub typedef_names: usize,
    pub typedef_name_entries: usize,
    pub function_names: usize,
    pub function_name_entries: usize,
    pub method_owner_names: usize,
    pub method_owner_name_entries: usize,
    pub field_owner_names: usize,
    pub field_owner_name_entries: usize,
    pub member_owners: usize,
    pub member_owner_entries: usize,
    pub parent_symbols: usize,
    pub child_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::model::{
        source_category_for_path, SourceCategory, SourceFileMetadata, SOURCE_PRIORITY_GAME_DATA,
        SOURCE_PRIORITY_WORKSPACE,
    };
    use crate::parser::parse_source;
    use crate::semantic_file::SemanticFile;
    use std::path::PathBuf;

    #[test]
    fn semantic_file_ingestion_matches_legacy_declaration_indexing() {
        let source = r#"typedef int Count;
class Example : Base
{
    int m_Value;
    void Run(string label = "x");
}
void Start();
"#;
        let metadata = SourceFileMetadata::unknown();
        let legacy_catalog = catalog(source, metadata.clone());
        let legacy = SymbolIndex::from_catalogs([&legacy_catalog]);

        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let semantic_file = SemanticFile::build(source, &parse);
        let mut semantic = SymbolIndex::default();
        semantic.add_semantic_file(&semantic_file, metadata);

        assert_eq!(semantic.files(), legacy.files());
        assert_eq!(semantic.symbols().len(), legacy.symbols().len());
        for (actual, expected) in semantic.symbols().iter().zip(legacy.symbols()) {
            assert_eq!(actual.id, expected.id);
            assert_eq!(actual.parent, expected.parent);
            assert_eq!(actual.kind, expected.kind);
            assert_eq!(actual.name, expected.name);
            assert_eq!(actual.span, expected.span);
            assert_eq!(actual.selection_span, expected.selection_span);
            assert_eq!(actual.detail, expected.detail);
            assert_eq!(actual.attributes, expected.attributes);
            assert_eq!(actual.modifiers, expected.modifiers);
            assert_eq!(actual.doc_comments, expected.doc_comments);
        }
        assert_eq!(
            semantic.methods_by_owner_name("Example", "Run"),
            legacy.methods_by_owner_name("Example", "Run")
        );
        assert_eq!(
            semantic.fields_by_owner_name("Example", "m_Value"),
            legacy.fields_by_owner_name("Example", "m_Value")
        );
        assert_eq!(
            semantic.functions_by_name("Start"),
            legacy.functions_by_name("Start")
        );
    }

    #[test]
    fn validated_contribution_ingestion_matches_semantic_file_indexing() {
        let source = r#"typedef int Count;
class Example : Base
{
    int m_Value;
    void Run(string label = "x");
}
void Start();
"#;
        let metadata = SourceFileMetadata::unknown();
        let parse = parse_source(source);
        let semantic_file = SemanticFile::build(source, &parse);

        let mut from_semantic_file = SymbolIndex::default();
        from_semantic_file.add_semantic_file(&semantic_file, metadata.clone());

        let contribution = semantic_file.contribution();
        let mut from_contribution = SymbolIndex::default();
        from_contribution
            .add_file_contribution(&contribution, metadata)
            .unwrap();

        assert_eq!(from_contribution.files(), from_semantic_file.files());
        assert_eq!(from_contribution.symbols(), from_semantic_file.symbols());
        assert_eq!(
            from_contribution.methods_by_owner_name("Example", "Run"),
            from_semantic_file.methods_by_owner_name("Example", "Run")
        );
    }

    #[test]
    fn contribution_ingestion_preserves_public_symbols_after_local_ids() {
        let source =
            include_str!("../../tools/fixtures/index/contribution_public_ids_after_local.c");
        let parse = parse_source(source);
        let semantic_file = SemanticFile::build(source, &parse);
        let contribution = semantic_file.contribution();
        let mut index = SymbolIndex::default();
        index
            .add_file_contribution(&contribution, SourceFileMetadata::unknown())
            .unwrap();

        let later = index.classes_by_name("ContributionIdsAfterPublicFixture")[0];
        assert_eq!(later.symbol_id, SymbolId(2));
        assert_eq!(
            index
                .symbol(later)
                .and_then(|symbol| symbol.name.as_deref()),
            Some("ContributionIdsAfterPublicFixture")
        );
    }

    #[test]
    fn indexes_names_kinds_children_classes_typedefs_and_methods() {
        let source = r#"typedef string FactionKey;

class Example : Base
{
	int m_Value;
	void Run(int value);
}
"#;
        let catalog = catalog(
            source,
            SourceFileMetadata {
                kind: SourceKind::GameData,
                category: SourceCategory::Unknown,
                absolute_path: Some(PathBuf::from("C:/game/Example.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        assert_eq!(index.files().len(), 1);
        assert_eq!(index.symbols().len(), 5);
        assert_eq!(index.symbols_for_name("Example").len(), 1);
        assert_eq!(index.top_level_symbols_for_name("Example").len(), 1);
        assert_eq!(index.classes_by_name("Example").len(), 1);
        assert_eq!(index.typedefs_by_name("FactionKey").len(), 1);
        assert!(index.functions_by_name("FactionKey").is_empty());
        assert_eq!(index.methods_by_owner_name("Example", "Run").len(), 1);
        assert_eq!(index.fields_by_owner_name("Example", "m_Value").len(), 1);
        assert_eq!(index.members_by_owner("Example").len(), 2);
        assert_eq!(index.symbols_for_kind(SymbolKind::Parameter).len(), 1);

        let class_id = index.classes_by_name("Example")[0];
        let children = index.children(class_id);
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|id| index
            .symbol(*id)
            .is_some_and(|symbol| symbol.name.as_deref() == Some("Run"))));
    }

    #[test]
    fn global_ids_keep_file_id_and_file_local_symbol_id() {
        let game = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::GameData,
                category: SourceCategory::Unknown,
                absolute_path: Some(PathBuf::from("C:/game/Example.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let workspace = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                category: SourceCategory::Workspace,
                absolute_path: Some(PathBuf::from("C:/workspace/Example.c")),
                root_path: Some(PathBuf::from("C:/workspace")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_WORKSPACE,
            },
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);

        let symbols = index.symbols_for_name("Example");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].file_id, SourceFileId(0));
        assert_eq!(symbols[0].symbol_id, SymbolId(0));
        assert_eq!(symbols[1].file_id, SourceFileId(1));
        assert_eq!(symbols[1].symbol_id, SymbolId(0));

        let preferred = index.preferred_symbols_for_name("Example");
        assert_eq!(preferred[0].file_id, SourceFileId(1));
        assert_eq!(
            index.file(preferred[0].file_id).unwrap().metadata.kind,
            SourceKind::Workspace
        );
        assert_eq!(index.duplicate_top_level_names().len(), 1);
    }

    #[test]
    fn preferred_top_level_lookup_excludes_non_top_level_symbols() {
        let game = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::GameData,
                category: SourceCategory::Unknown,
                absolute_path: Some(PathBuf::from("C:/game/Example.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let workspace = catalog(
            r#"class Example
{
	void Run(int Example);
}
"#,
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                category: SourceCategory::Workspace,
                absolute_path: Some(PathBuf::from("C:/workspace/Example.c")),
                root_path: Some(PathBuf::from("C:/workspace")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_WORKSPACE,
            },
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);

        let all = index.symbols_for_name("Example");
        assert_eq!(all.len(), 3);
        assert!(all.iter().any(|id| index
            .symbol(*id)
            .is_some_and(|symbol| symbol.kind == SymbolKind::Parameter)));

        let top_level = index.top_level_symbols_for_name("Example");
        assert_eq!(top_level.len(), 2);
        assert!(top_level.iter().all(|id| index
            .symbol(*id)
            .is_some_and(|symbol| symbol.parent.is_none())));

        let preferred_all = index.preferred_symbols_for_name("Example");
        assert_eq!(preferred_all.len(), 3);

        let preferred_top_level = index.preferred_top_level_symbols_for_name("Example");
        assert_eq!(preferred_top_level.len(), 2);
        assert_eq!(preferred_top_level[0].file_id, SourceFileId(1));
        assert_eq!(
            index.symbol(preferred_top_level[0]).unwrap().kind,
            SymbolKind::Class
        );
        assert_eq!(
            index
                .file(preferred_top_level[0].file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::Workspace
        );
    }

    #[test]
    fn stores_copied_lookup_details_without_requiring_source_text() {
        let catalog = catalog(
            r#"enum E
{
	One = 1,
}

class Example : Base
{
	void Run(int value = 4);
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let class = index.symbol(index.classes_by_name("Example")[0]).unwrap();
        assert_eq!(class.detail.base_type.as_deref(), Some("Base"));

        let method = index
            .symbol(index.methods_by_owner_name("Example", "Run")[0])
            .unwrap();
        assert_eq!(method.detail.return_type_text.as_deref(), Some("void"));

        let parameter = index.symbols_for_kind(SymbolKind::Parameter)[0];
        let parameter = index.symbol(parameter).unwrap();
        assert_eq!(parameter.detail.type_text.as_deref(), Some("int"));
        assert_eq!(parameter.detail.default_text.as_deref(), Some("4"));

        let enum_member = index.symbols_for_name("One")[0];
        let enum_member = index.symbol(enum_member).unwrap();
        assert_eq!(enum_member.detail.enum_value_text.as_deref(), Some("1"));
    }

    #[test]
    fn stores_copied_presentation_metadata_without_requiring_source_text() {
        let catalog = catalog(
            r#"//! Class docs
[BaseContainerProps()]
modded class Example
{
	/*! Field docs */
	protected int m_Value;

#ifdef ENABLE_RUN
	override void Run() {}
#endif
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let class = index.symbol(index.classes_by_name("Example")[0]).unwrap();
        assert_eq!(class.modifiers, vec!["modded"]);
        assert_eq!(class.attributes.len(), 1);
        assert_eq!(
            class.attributes[0].name.as_deref(),
            Some("BaseContainerProps")
        );
        assert_eq!(class.attributes[0].text, "[BaseContainerProps()]");
        assert_eq!(class.doc_comments.len(), 1);
        assert_eq!(class.doc_comments[0].kind, DocCommentKind::Line);
        assert_eq!(class.doc_comments[0].text, "//! Class docs");

        let field = index
            .symbol(index.fields_by_owner_name("Example", "m_Value")[0])
            .unwrap();
        assert_eq!(field.modifiers, vec!["protected"]);
        assert_eq!(field.doc_comments.len(), 1);
        assert_eq!(field.doc_comments[0].kind, DocCommentKind::Block);
        assert_eq!(field.doc_comments[0].text, "/*! Field docs */");

        let method = index
            .symbol(index.methods_by_owner_name("Example", "Run")[0])
            .unwrap();
        assert_eq!(method.modifiers, vec!["override"]);
        assert_eq!(method.callable_form, Some(CallableForm::Implementation));
        assert_eq!(method.conditional_context.len(), 1);
        assert_eq!(
            method.conditional_context[0].condition.as_deref(),
            Some("ENABLE_RUN")
        );
    }

    #[test]
    fn exposes_method_owner_name_groups_for_overload_review() {
        let catalog = catalog(
            r#"class SCR_AutotestHarness
{
	void Begin();
	void Begin(int value);
	int Count();
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        assert_eq!(
            index
                .methods_by_owner_name("SCR_AutotestHarness", "Begin")
                .len(),
            2
        );
        assert_eq!(
            index
                .methods_by_owner_name("SCR_AutotestHarness", "Count")
                .len(),
            1
        );

        let begin_key = ("SCR_AutotestHarness".to_string(), "Begin".to_string());
        let count_key = ("SCR_AutotestHarness".to_string(), "Count".to_string());
        assert_eq!(index.method_owner_name_groups()[&begin_key].len(), 2);
        assert_eq!(index.method_owner_name_groups()[&count_key].len(), 1);
    }

    #[test]
    fn formats_regular_method_signatures_from_indexed_parameter_children() {
        let catalog = catalog(
            r#"class SCR_BaseGameMode
{
	void OnGameStart();
	void Begin(string suite, string test);
	void Run(int value = 4);
	array<SCR_BaseGameModeComponent> GetComponentsByType(typename componentType, out int foundCount);
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let on_game_start = index.methods_by_owner_name("SCR_BaseGameMode", "OnGameStart")[0];
        let begin = index.methods_by_owner_name("SCR_BaseGameMode", "Begin")[0];
        let run = index.methods_by_owner_name("SCR_BaseGameMode", "Run")[0];
        let get_components =
            index.methods_by_owner_name("SCR_BaseGameMode", "GetComponentsByType")[0];

        assert_eq!(
            index.callable_signature(on_game_start).as_deref(),
            Some("SCR_BaseGameMode.OnGameStart() -> void")
        );
        assert_eq!(
            index.callable_signature(begin).as_deref(),
            Some("SCR_BaseGameMode.Begin(string suite, string test) -> void")
        );
        assert_eq!(
            index.callable_signature(run).as_deref(),
            Some("SCR_BaseGameMode.Run(int value = 4) -> void")
        );
        assert_eq!(
            index.callable_signature(get_components).as_deref(),
            Some("SCR_BaseGameMode.GetComponentsByType(typename componentType, out int foundCount) -> array<SCR_BaseGameModeComponent>")
        );
    }

    #[test]
    fn formats_general_callable_signatures() {
        let catalog = catalog(
            r#"void GlobalFn(int value = 4);

class Example
{
	void Example(int value);
	void ~Example();
	void Run(notnull string name, inout int count);
	int Count();
	int m_Value;
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let global_fn = index.symbols_for_name("GlobalFn")[0];
        let run = index.methods_by_owner_name("Example", "Run")[0];
        let constructor = index
            .members_by_owner("Example")
            .iter()
            .copied()
            .find(|id| index.symbol(*id).unwrap().kind == SymbolKind::Constructor)
            .unwrap();
        let destructor = index
            .members_by_owner("Example")
            .iter()
            .copied()
            .find(|id| index.symbol(*id).unwrap().kind == SymbolKind::Destructor)
            .unwrap();
        let field = index.fields_by_owner_name("Example", "m_Value")[0];

        assert_eq!(
            index.callable_signature(global_fn).as_deref(),
            Some("GlobalFn(int value = 4) -> void")
        );
        assert_eq!(
            index.callable_signature(run).as_deref(),
            Some("Example.Run(notnull string name, inout int count) -> void")
        );
        assert_eq!(
            index.callable_signature(constructor).as_deref(),
            Some("Example(int value)")
        );
        assert_eq!(
            index.callable_signature(destructor).as_deref(),
            Some("~Example()")
        );
        assert_eq!(index.callable_signature(field), None);
    }

    #[test]
    fn indexes_direct_class_fields_and_members_by_owner() {
        let catalog = catalog(
            r#"int m_Value;

class Example
{
	int m_Value;
	void Example();
	void ~Example();
	void Run(int value);
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let fields = index.fields_by_owner_name("Example", "m_Value");
        assert_eq!(fields.len(), 1);
        assert_eq!(index.symbol(fields[0]).unwrap().kind, SymbolKind::Field);

        let members = index.members_by_owner("Example");
        assert_eq!(members.len(), 4);
        assert!(members
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Field));
        assert!(members
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Method));
        assert!(members
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Constructor));
        assert!(members
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Destructor));
        assert!(members
            .iter()
            .all(|id| index.symbol(*id).unwrap().kind != SymbolKind::Parameter));
    }

    #[test]
    fn walks_direct_members_then_exact_name_base_class_members() {
        let catalog = catalog(
            r#"class Base
{
	int m_Base;
	void Run();
}

class Child : Base
{
	int m_Child;
	void Run(int value);
}

class GrandChild : Child
{
	int m_GrandChild;
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let members = index.raw_members_for_class_including_bases("GrandChild");
        let member_names = member_names(&index, &members);

        assert_eq!(
            member_names,
            vec!["m_GrandChild", "m_Child", "Run", "m_Base", "Run"]
        );
        assert_eq!(index.direct_members_by_owner("GrandChild").len(), 1);
        assert_eq!(index.members_by_owner("GrandChild").len(), 1);
    }

    #[test]
    fn inherited_member_lookup_keeps_direct_members_when_base_is_missing() {
        let catalog = catalog(
            r#"class Child : MissingBase
{
	int m_Child;
	void Run();
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let members = index.raw_members_for_class_including_bases("Child");

        assert_eq!(member_names(&index, &members), vec!["m_Child", "Run"]);
    }

    #[test]
    fn inherited_member_lookup_stops_on_cycles() {
        let catalog = catalog(
            r#"class A : B
{
	int m_A;
}

class B : A
{
	int m_B;
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let members = index.raw_members_for_class_including_bases("A");

        assert_eq!(member_names(&index, &members), vec!["m_A", "m_B"]);
    }

    #[test]
    fn completion_members_are_direct_first_and_hide_matching_base_members() {
        let catalog = catalog(
            r#"class Base
{
	int m_Value;
	int m_BaseOnly;
	void Run(int value);
	void Run(string value);
	void BaseOnly();
}

class Child : Base
{
	int m_Value;
	void Run(int other);
	void ChildOnly();
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let raw = index.raw_members_for_class_including_bases("Child");
        assert_eq!(
            member_names(&index, &raw),
            vec![
                "m_Value",
                "Run",
                "ChildOnly",
                "m_Value",
                "m_BaseOnly",
                "Run",
                "Run",
                "BaseOnly"
            ]
        );

        let completion = index.raw_completion_members_for_owner_name("Child");
        assert_eq!(completion.raw_candidates, raw);
        assert_eq!(
            member_names(&index, &completion.members),
            vec![
                "m_Value",
                "Run",
                "ChildOnly",
                "m_BaseOnly",
                "Run",
                "BaseOnly"
            ]
        );
        assert_eq!(completion.shadowed_groups.len(), 2);
        assert!(completion
            .shadowed_groups
            .iter()
            .any(|group| group.key == "Field m_Value" && group.shadowed.len() == 1));
        assert!(completion
            .shadowed_groups
            .iter()
            .any(|group| group.key == "Method Run(int) -> void" && group.shadowed.len() == 1));
    }

    #[test]
    fn completion_members_do_not_shadow_static_array_fields_by_bound_name() {
        let catalog = catalog(
            r#"class Example
{
	static const int COUNT = 4;
	static const string TAGS[COUNT] = {};
	LocalizedString NAMES[COUNT];
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        assert_eq!(index.fields_by_owner_name("Example", "COUNT").len(), 1);
        assert_eq!(index.fields_by_owner_name("Example", "TAGS").len(), 1);
        assert_eq!(index.fields_by_owner_name("Example", "NAMES").len(), 1);
        assert!(index
            .fields_by_owner_name("Example", "COUNT")
            .iter()
            .all(|id| index.symbol(*id).unwrap().detail.type_text.as_deref() == Some("int")));

        let completion = index.raw_completion_members_for_owner_name("Example");

        assert_eq!(
            member_names(&index, &completion.members),
            vec!["COUNT", "TAGS", "NAMES"]
        );
        assert!(completion.shadowed_groups.is_empty());
    }

    #[test]
    fn indexes_comma_separated_field_declarators_for_lookup_and_completion() {
        let catalog = catalog(
            r#"class Example
{
	protected Widget m_ContentWidget, m_ButtonPrevWidget, m_ButtonNextWidget;
	protected int count, values[COUNT], other = 4;
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        for name in [
            "m_ContentWidget",
            "m_ButtonPrevWidget",
            "m_ButtonNextWidget",
            "count",
            "values",
            "other",
        ] {
            let fields = index.fields_by_owner_name("Example", name);
            assert_eq!(fields.len(), 1, "missing field {name}");
        }

        let content = index.symbol(index.fields_by_owner_name("Example", "m_ContentWidget")[0]);
        assert_eq!(content.unwrap().detail.type_text.as_deref(), Some("Widget"));
        let button_next =
            index.symbol(index.fields_by_owner_name("Example", "m_ButtonNextWidget")[0]);
        assert_eq!(
            button_next.unwrap().detail.type_text.as_deref(),
            Some("Widget")
        );
        let values = index.symbol(index.fields_by_owner_name("Example", "values")[0]);
        assert_eq!(values.unwrap().detail.type_text.as_deref(), Some("int"));

        let completion = index.raw_completion_members_for_owner_name("Example");
        assert_eq!(
            member_names(&index, &completion.members),
            vec![
                "m_ContentWidget",
                "m_ButtonPrevWidget",
                "m_ButtonNextWidget",
                "count",
                "values",
                "other"
            ]
        );
        assert!(completion.shadowed_groups.is_empty());
    }

    #[test]
    fn completion_members_prefer_workspace_within_same_owner_depth() {
        let game = catalog(
            r#"class SCR_BaseGameMode
{
	void OnGameStart();
	void GameOnly();
}
"#,
            game_metadata("SCR_BaseGameMode.c"),
        );
        let workspace = catalog(
            r#"modded class SCR_BaseGameMode
{
	override void OnGameStart();
	void WorkspaceOnly();
}
"#,
            workspace_metadata("SCR_BaseGameMode.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);

        let raw = index.raw_members_for_class_including_bases("SCR_BaseGameMode");
        assert_eq!(
            member_names(&index, &raw),
            vec!["OnGameStart", "GameOnly", "OnGameStart", "WorkspaceOnly"]
        );

        let completion = index.raw_completion_members_for_owner_name("SCR_BaseGameMode");
        assert_eq!(completion.raw_candidates, raw);
        assert_eq!(
            member_names(&index, &completion.members),
            vec!["OnGameStart", "GameOnly", "WorkspaceOnly"]
        );

        let kept_on_game_start = completion
            .members
            .iter()
            .copied()
            .find(|id| index.symbol(*id).unwrap().name.as_deref() == Some("OnGameStart"))
            .unwrap();
        assert_eq!(
            index
                .file(kept_on_game_start.file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::Workspace
        );

        let shadow_group = completion
            .shadowed_groups
            .iter()
            .find(|group| group.key == "Method OnGameStart() -> void")
            .unwrap();
        assert_eq!(shadow_group.kept, kept_on_game_start);
        assert_eq!(shadow_group.shadowed.len(), 1);
        assert_eq!(
            index
                .file(shadow_group.shadowed[0].file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::GameData
        );
    }

    #[test]
    fn preferred_class_completion_uses_preferred_declaration_then_overlay_then_bases() {
        let base = catalog(
            r#"class BaseGameMode
{
	void BaseOnly();
}
"#,
            game_metadata("BaseGameMode.c"),
        );
        let game = catalog(
            r#"class SCR_BaseGameMode : BaseGameMode
{
	void OnGameStart();
	void GameOnly();
}
"#,
            game_metadata("SCR_BaseGameMode.c"),
        );
        let workspace = catalog(
            r#"modded class SCR_BaseGameMode
{
	override void OnGameStart();
	void WorkspaceOnly();
}
"#,
            workspace_metadata("SCR_BaseGameMode.c"),
        );
        let index = SymbolIndex::from_catalogs([&base, &game, &workspace]);

        let raw = index.raw_completion_members_for_owner_name("SCR_BaseGameMode");
        assert_eq!(
            member_names(&index, &raw.members),
            vec!["OnGameStart", "GameOnly", "WorkspaceOnly"]
        );

        let completion = index.completion_members_for_preferred_class("SCR_BaseGameMode");
        assert_eq!(
            member_names(&index, &completion.raw_candidates),
            vec![
                "OnGameStart",
                "WorkspaceOnly",
                "OnGameStart",
                "GameOnly",
                "BaseOnly"
            ]
        );
        assert_eq!(
            member_names(&index, &completion.members),
            vec!["OnGameStart", "WorkspaceOnly", "GameOnly", "BaseOnly"]
        );

        let kept_on_game_start = completion
            .members
            .iter()
            .copied()
            .find(|id| index.symbol(*id).unwrap().name.as_deref() == Some("OnGameStart"))
            .unwrap();
        assert_eq!(
            index
                .file(kept_on_game_start.file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::Workspace
        );

        let shadow_group = completion
            .shadowed_groups
            .iter()
            .find(|group| group.key == "Method OnGameStart() -> void")
            .unwrap();
        assert_eq!(shadow_group.kept, kept_on_game_start);
        assert_eq!(shadow_group.shadowed.len(), 1);
        assert_eq!(
            index
                .file(shadow_group.shadowed[0].file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::GameData
        );
    }

    #[test]
    fn preferred_class_completion_keeps_direct_overlay_before_higher_priority_base_members() {
        let base = catalog(
            r#"class Base
{
	void Run();
}
"#,
            workspace_metadata("Base.c"),
        );
        let child = catalog(
            r#"class Child : Base
{
	void Run();
}
"#,
            game_metadata("Child.c"),
        );
        let index = SymbolIndex::from_catalogs([&base, &child]);

        let completion = index.completion_members_for_preferred_class("Child");
        let run = completion.members[0];

        assert_eq!(index.symbol(run).unwrap().name.as_deref(), Some("Run"));
        assert_eq!(
            index.file(run.file_id).unwrap().metadata.kind,
            SourceKind::GameData
        );
        assert_eq!(completion.shadowed_groups.len(), 1);
        assert_eq!(completion.shadowed_groups[0].kept, run);
        assert_eq!(
            index
                .file(completion.shadowed_groups[0].shadowed[0].file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::Workspace
        );
    }

    #[test]
    fn preferred_class_completion_keeps_direct_members_when_base_is_missing() {
        let catalog = catalog(
            r#"class Child : MissingBase
{
	int m_Child;
	void Run();
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let completion = index.completion_members_for_preferred_class("Child");

        assert_eq!(
            member_names(&index, &completion.members),
            vec!["m_Child", "Run"]
        );
        assert!(completion.shadowed_groups.is_empty());
    }

    #[test]
    fn preferred_class_completion_stops_on_cycles() {
        let catalog = catalog(
            r#"class A : B
{
	int m_A;
	void Run();
}

class B : A
{
	int m_B;
	void Run();
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let completion = index.completion_members_for_preferred_class("A");

        assert_eq!(
            member_names(&index, &completion.members),
            vec!["m_A", "Run", "m_B"]
        );
        assert_eq!(completion.shadowed_groups.len(), 1);
        assert_eq!(completion.shadowed_groups[0].key, "Method Run() -> void");
    }

    #[test]
    fn completion_members_keep_direct_depth_before_higher_priority_base_members() {
        let game_base = catalog(
            r#"class Base
{
	void Run();
}
"#,
            workspace_metadata("Base.c"),
        );
        let game_child = catalog(
            r#"class Child : Base
{
	void Run();
}
"#,
            game_metadata("Child.c"),
        );
        let index = SymbolIndex::from_catalogs([&game_base, &game_child]);

        let completion = index.raw_completion_members_for_owner_name("Child");
        let run = completion.members[0];

        assert_eq!(index.symbol(run).unwrap().name.as_deref(), Some("Run"));
        assert_eq!(
            index.file(run.file_id).unwrap().metadata.kind,
            SourceKind::GameData
        );
        assert_eq!(completion.shadowed_groups.len(), 1);
        assert_eq!(completion.shadowed_groups[0].kept, run);
        assert_eq!(
            index
                .file(completion.shadowed_groups[0].shadowed[0].file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::Workspace
        );
    }

    #[test]
    fn completion_members_keep_direct_members_when_base_is_missing() {
        let catalog = catalog(
            r#"class Child : MissingBase
{
	int m_Child;
	void Run();
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let completion = index.raw_completion_members_for_owner_name("Child");

        assert_eq!(
            member_names(&index, &completion.members),
            vec!["m_Child", "Run"]
        );
        assert!(completion.shadowed_groups.is_empty());
    }

    #[test]
    fn completion_member_lookup_stops_on_cycles() {
        let catalog = catalog(
            r#"class A : B
{
	int m_A;
	void Run();
}

class B : A
{
	int m_B;
	void Run();
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let completion = index.raw_completion_members_for_owner_name("A");

        assert_eq!(
            member_names(&index, &completion.members),
            vec!["m_A", "Run", "m_B"]
        );
        assert_eq!(completion.shadowed_groups.len(), 1);
        assert_eq!(completion.shadowed_groups[0].key, "Method Run() -> void");
    }

    #[test]
    fn completion_member_lookup_deduplicates_constructor_and_destructor_shapes() {
        let catalog = catalog(
            r#"class Base
{
	void Base(int value);
	void ~Base();
}

class Child : Base
{
	void Child(int value);
	void ~Child();
}
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let completion = index.raw_completion_members_for_owner_name("Child");

        assert_eq!(
            member_names(&index, &completion.members),
            vec!["Child", "Child", "Base", "Base"]
        );
        assert!(completion.shadowed_groups.is_empty());
    }

    #[test]
    fn preferred_from_symbols_sorts_by_priority_then_stable_ids() {
        let first_game = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::GameData,
                category: SourceCategory::Unknown,
                absolute_path: Some(PathBuf::from("C:/game/First.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("First.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let workspace = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                category: SourceCategory::Workspace,
                absolute_path: Some(PathBuf::from("C:/workspace/Example.c")),
                root_path: Some(PathBuf::from("C:/workspace")),
                relative_path: Some(PathBuf::from("Example.c")),
                priority: SOURCE_PRIORITY_WORKSPACE,
            },
        );
        let second_game = catalog(
            "class Example {}",
            SourceFileMetadata {
                kind: SourceKind::GameData,
                category: SourceCategory::Unknown,
                absolute_path: Some(PathBuf::from("C:/game/Second.c")),
                root_path: Some(PathBuf::from("C:/game")),
                relative_path: Some(PathBuf::from("Second.c")),
                priority: SOURCE_PRIORITY_GAME_DATA,
            },
        );
        let index = SymbolIndex::from_catalogs([&first_game, &workspace, &second_game]);
        let unsorted = [
            GlobalSymbolId {
                file_id: SourceFileId(2),
                symbol_id: SymbolId(0),
            },
            GlobalSymbolId {
                file_id: SourceFileId(0),
                symbol_id: SymbolId(0),
            },
            GlobalSymbolId {
                file_id: SourceFileId(1),
                symbol_id: SymbolId(0),
            },
        ];

        let preferred = index.preferred_from_symbols(&unsorted);

        assert_eq!(preferred[0].file_id, SourceFileId(1));
        assert_eq!(preferred[1].file_id, SourceFileId(0));
        assert_eq!(preferred[2].file_id, SourceFileId(2));
    }

    #[test]
    fn workspace_modded_class_is_preferred_over_game_data_class() {
        let game = catalog(
            "class SCR_BaseGameMode {}",
            game_metadata("SCR_BaseGameMode.c"),
        );
        let workspace = catalog(
            "modded class SCR_BaseGameMode {}",
            workspace_metadata("SCR_BaseGameMode.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);

        let classes = index.classes_by_name("SCR_BaseGameMode");
        assert_eq!(classes.len(), 2);

        let preferred = index.preferred_from_symbols(classes);
        let preferred_symbol = index.symbol(preferred[0]).unwrap();
        let preferred_file = index.file(preferred[0].file_id).unwrap();

        assert_eq!(preferred_symbol.kind, SymbolKind::Class);
        assert_eq!(preferred_symbol.name.as_deref(), Some("SCR_BaseGameMode"));
        assert_eq!(preferred_file.metadata.kind, SourceKind::Workspace);
        assert_eq!(preferred_file.metadata.priority, SOURCE_PRIORITY_WORKSPACE);
    }

    #[test]
    fn top_level_lookup_ignores_fields_and_parameters_with_same_name() {
        let catalog = catalog(
            r#"class SharedName
{
	int SharedName;
	void Run(int SharedName);
}
"#,
            workspace_metadata("SharedName.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let all = index.symbols_for_name("SharedName");
        assert_eq!(all.len(), 3);
        assert!(all
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Class));
        assert!(all
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Field));
        assert!(all
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Parameter));

        let top_level = index.top_level_symbols_for_name("SharedName");
        assert_eq!(top_level.len(), 1);
        assert_eq!(index.symbol(top_level[0]).unwrap().kind, SymbolKind::Class);

        let preferred = index.preferred_top_level_symbols_for_name("SharedName");
        assert_eq!(preferred, top_level);
    }

    #[test]
    fn local_variables_are_indexed_for_name_lookup_but_not_member_completion() {
        let catalog = catalog(
            r#"class Example
{
	int value;
	void Run(int value)
	{
		int value = 4;
	}
}
"#,
            workspace_metadata("Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let all = index.symbols_for_name("value");
        assert_eq!(all.len(), 3);
        assert!(all
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Field));
        assert!(all
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Parameter));
        assert!(all
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::LocalVariable));

        assert!(index.top_level_symbols_for_name("value").is_empty());
        assert_eq!(index.fields_by_owner_name("Example", "value").len(), 1);
        assert!(index
            .members_by_owner("Example")
            .iter()
            .all(|id| index.symbol(*id).unwrap().kind != SymbolKind::LocalVariable));
        assert!(index
            .raw_completion_members_for_owner_name("Example")
            .members
            .iter()
            .all(|id| index.symbol(*id).unwrap().kind != SymbolKind::LocalVariable));
    }

    #[test]
    fn method_owner_lookup_aggregates_game_data_and_workspace_methods() {
        let game = catalog(
            r#"class SCR_BaseGameMode
{
	void OnGameStart();
}
"#,
            game_metadata("SCR_BaseGameMode.c"),
        );
        let workspace = catalog(
            r#"modded class SCR_BaseGameMode
{
	override void OnGameStart();
}
"#,
            workspace_metadata("SCR_BaseGameMode.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);

        let methods = index.methods_by_owner_name("SCR_BaseGameMode", "OnGameStart");
        assert_eq!(methods.len(), 2);

        let preferred = index.preferred_from_symbols(methods);
        let preferred_symbol = index.symbol(preferred[0]).unwrap();
        let preferred_file = index.file(preferred[0].file_id).unwrap();

        assert_eq!(preferred_symbol.kind, SymbolKind::Method);
        assert_eq!(preferred_symbol.name.as_deref(), Some("OnGameStart"));
        assert_eq!(preferred_file.metadata.kind, SourceKind::Workspace);
        assert_eq!(preferred_file.metadata.priority, SOURCE_PRIORITY_WORKSPACE);
    }

    #[test]
    fn duplicate_top_level_conflict_records_include_review_metadata() {
        let catalog = catalog(
            r#"typedef string FactionKey;
class FactionKey : string {}
"#,
            game_metadata("GameCode/Faction/FactionKey.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let duplicates = index.duplicate_top_level_names();
        let faction_key = duplicates
            .iter()
            .find(|(name, _)| *name == "FactionKey")
            .expect("FactionKey should be a duplicate top-level name");
        assert_eq!(faction_key.1.len(), 2);

        let kinds = faction_key
            .1
            .iter()
            .map(|id| index.symbol(*id).unwrap().kind)
            .collect::<Vec<_>>();
        assert!(kinds.contains(&SymbolKind::Typedef));
        assert!(kinds.contains(&SymbolKind::Class));

        for id in faction_key.1 {
            let file = index.file(id.file_id).unwrap();
            assert_eq!(file.metadata.kind, SourceKind::GameData);
            assert_eq!(file.metadata.priority, SOURCE_PRIORITY_GAME_DATA);
            assert_eq!(
                file.metadata.relative_path.as_deref(),
                Some(std::path::Path::new("GameCode/Faction/FactionKey.c"))
            );
        }
    }

    #[test]
    fn preferred_kind_specific_top_level_lookup_separates_conflict_kinds() {
        let catalog = catalog(
            r#"typedef string FactionKey;
class FactionKey : string {}
void FactionKey(int value);
"#,
            workspace_metadata("GameCode/Faction/FactionKey.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        let generic = index.preferred_top_level_symbols_for_name("FactionKey");
        assert_eq!(generic.len(), 3);
        assert!(generic.iter().any(|id| index
            .symbol(*id)
            .is_some_and(|symbol| symbol.kind == SymbolKind::Class)));
        assert!(generic.iter().any(|id| index
            .symbol(*id)
            .is_some_and(|symbol| symbol.kind == SymbolKind::Typedef)));
        assert!(generic.iter().any(|id| index
            .symbol(*id)
            .is_some_and(|symbol| symbol.kind == SymbolKind::Function)));

        let preferred_class = index.preferred_classes_by_name("FactionKey");
        let preferred_typedef = index.preferred_typedefs_by_name("FactionKey");
        let preferred_function = index.preferred_functions_by_name("FactionKey");

        assert_eq!(preferred_class.len(), 1);
        assert_eq!(preferred_typedef.len(), 1);
        assert_eq!(preferred_function.len(), 1);
        assert_eq!(
            index.symbol(preferred_class[0]).unwrap().kind,
            SymbolKind::Class
        );
        assert_eq!(
            index.symbol(preferred_typedef[0]).unwrap().kind,
            SymbolKind::Typedef
        );
        assert_eq!(
            index.symbol(preferred_function[0]).unwrap().kind,
            SymbolKind::Function
        );
    }

    #[test]
    fn preferred_kind_specific_lookup_uses_workspace_priority_within_kind() {
        let game = catalog(
            r#"class Example {}
typedef int ExampleAlias;
void ExampleFn();
"#,
            game_metadata("Game.c"),
        );
        let workspace = catalog(
            r#"modded class Example {}
typedef float ExampleAlias;
void ExampleFn(int value);
"#,
            workspace_metadata("Workspace.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);

        let preferred_class = index.preferred_classes_by_name("Example");
        let preferred_typedef = index.preferred_typedefs_by_name("ExampleAlias");
        let preferred_function = index.preferred_functions_by_name("ExampleFn");

        for id in [
            preferred_class[0],
            preferred_typedef[0],
            preferred_function[0],
        ] {
            assert_eq!(
                index.file(id.file_id).unwrap().metadata.kind,
                SourceKind::Workspace
            );
        }
        assert_eq!(
            index.symbol(preferred_class[0]).unwrap().kind,
            SymbolKind::Class
        );
        assert_eq!(
            index.symbol(preferred_typedef[0]).unwrap().kind,
            SymbolKind::Typedef
        );
        assert_eq!(
            index.symbol(preferred_function[0]).unwrap().kind,
            SymbolKind::Function
        );
    }

    #[test]
    fn function_lookup_excludes_top_level_classes_and_typedefs_with_same_name() {
        let catalog = catalog(
            r#"typedef string Shared;
class Shared {}
void Shared();
"#,
            SourceFileMetadata::unknown(),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);

        assert_eq!(index.top_level_symbols_for_name("Shared").len(), 3);
        assert_eq!(index.functions_by_name("Shared").len(), 1);
        assert_eq!(
            index
                .symbol(index.functions_by_name("Shared")[0])
                .unwrap()
                .kind,
            SymbolKind::Function
        );
    }

    #[test]
    fn overlay_index_prefers_workspace_symbols_over_game_data() {
        let game = catalog(
            r#"class SCR_BaseGameMode
{
	void OnGameStart();
}

typedef string FactionKey;
"#,
            game_metadata("Game.c"),
        );
        let workspace = catalog(
            r#"modded class SCR_BaseGameMode
{
	override void OnGameStart();
}

class FactionKey : string
{
}
"#,
            workspace_metadata("Workspace.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);

        let source_counts = index.source_kind_counts();
        assert_eq!(source_counts.get(&SourceKind::GameData), Some(&1));
        assert_eq!(source_counts.get(&SourceKind::Workspace), Some(&1));

        let preferred_class =
            index.preferred_from_symbols(index.classes_by_name("SCR_BaseGameMode"));
        assert_eq!(
            index
                .file(preferred_class[0].file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::Workspace
        );

        let preferred_top_level = index.preferred_top_level_symbols_for_name("FactionKey");
        assert_eq!(
            index
                .file(preferred_top_level[0].file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::Workspace
        );
        assert_eq!(
            index.symbol(preferred_top_level[0]).unwrap().kind,
            SymbolKind::Class
        );

        let preferred_method = index
            .preferred_from_symbols(index.methods_by_owner_name("SCR_BaseGameMode", "OnGameStart"));
        assert_eq!(
            index
                .file(preferred_method[0].file_id)
                .unwrap()
                .metadata
                .kind,
            SourceKind::Workspace
        );
        assert_eq!(
            index.symbol(preferred_method[0]).unwrap().kind,
            SymbolKind::Method
        );
    }

    #[test]
    fn merged_indexes_preserve_file_symbol_ranges_and_parent_links() {
        let first = SymbolIndex::from_catalogs([&catalog(
            "class First { void FirstMethod(); }",
            game_metadata("Game/First.c"),
        )]);
        let second = SymbolIndex::from_catalogs([&catalog(
            "class Second { void SecondMethod(); }",
            workspace_metadata("Scripts/Second.c"),
        )]);

        let merged = SymbolIndex::merged([&first, &second]);

        assert_eq!(
            member_names(
                &merged,
                &merged
                    .completion_members_for_preferred_class("First")
                    .members
            ),
            vec!["FirstMethod"]
        );
        assert_eq!(
            member_names(
                &merged,
                &merged
                    .completion_members_for_preferred_class("Second")
                    .members
            ),
            vec!["SecondMethod"]
        );
        assert_no_dangling_symbol_references(&merged);
    }

    #[test]
    fn pruned_index_removes_local_variables_and_preserves_parameters() {
        let catalog = catalog(
            r#"class Example
{
	int m_Value;
	void Run(int value)
	{
		int localValue = value;
		string localName = "ok";
	}
}
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let pruned = index.without_local_variables();

        assert_eq!(index.symbols_for_kind(SymbolKind::LocalVariable).len(), 2);
        assert!(pruned
            .symbols_for_kind(SymbolKind::LocalVariable)
            .is_empty());
        assert_eq!(pruned.symbols_for_kind(SymbolKind::Parameter).len(), 1);
        assert_eq!(pruned.symbols_for_name("localValue").len(), 0);
        assert_eq!(pruned.symbols_for_name("localName").len(), 0);
        assert_eq!(pruned.symbols_for_name("value").len(), 1);
        assert_eq!(pruned.classes_by_name("Example").len(), 1);
        assert_eq!(pruned.fields_by_owner_name("Example", "m_Value").len(), 1);
        assert_eq!(pruned.methods_by_owner_name("Example", "Run").len(), 1);
        assert_eq!(
            pruned
                .callable_signature(pruned.methods_by_owner_name("Example", "Run")[0])
                .as_deref(),
            Some("Example.Run(int value) -> void")
        );
        assert_no_dangling_symbol_references(&pruned);
    }

    #[test]
    fn runtime_cache_compaction_removes_locals_and_detail_spans_only() {
        let catalog = catalog(
            r#"class Example : BaseExample
{
	ref array<int> m_Values;

	int Run(string name = "ok")
	{
		int localValue;
		return 0;
	}
}
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let compact = index.compact_for_runtime_cache();

        assert_eq!(index.symbols_for_kind(SymbolKind::LocalVariable).len(), 1);
        assert!(compact
            .symbols_for_kind(SymbolKind::LocalVariable)
            .is_empty());
        assert_eq!(compact.symbols_for_kind(SymbolKind::Parameter).len(), 1);

        let class = compact
            .symbol(compact.classes_by_name("Example")[0])
            .unwrap();
        assert_eq!(class.detail.base_type.as_deref(), Some("BaseExample"));
        assert!(class.detail.base_type_span.is_none());

        let field = compact
            .symbol(compact.fields_by_owner_name("Example", "m_Values")[0])
            .unwrap();
        assert_eq!(field.detail.type_text.as_deref(), Some("ref array<int>"));
        assert!(field.detail.type_text_span.is_none());

        let method = compact
            .symbol(compact.methods_by_owner_name("Example", "Run")[0])
            .unwrap();
        assert_eq!(method.detail.return_type_text.as_deref(), Some("int"));
        assert!(method.detail.return_type_text_span.is_none());
        assert_eq!(
            compact.callable_signature(method.id).as_deref(),
            Some("Example.Run(string name = \"ok\") -> int")
        );

        let parameter = compact.symbols_for_name("name")[0];
        let parameter = compact.symbol(parameter).unwrap();
        assert_eq!(parameter.detail.type_text.as_deref(), Some("string"));
        assert_eq!(parameter.detail.default_text.as_deref(), Some("\"ok\""));
        assert!(parameter.detail.type_text_span.is_none());
        assert!(parameter.detail.default_text_span.is_none());

        assert_no_dangling_symbol_references(&compact);
    }

    #[test]
    fn runtime_cache_compaction_preserves_multi_file_symbol_ranges() {
        let first = catalog(
            r#"class First
{
	void Run()
	{
		int localValue;
	}
}
"#,
            game_metadata("Game/First.c"),
        );
        let second = catalog(
            r#"class SecondBase {}
class Second : SecondBase
{
	int m_Value;
}
"#,
            game_metadata("Game/Second.c"),
        );
        let compact = SymbolIndex::from_catalogs([&first, &second]).compact_for_runtime_cache();

        assert!(compact
            .symbols_for_kind(SymbolKind::LocalVariable)
            .is_empty());

        let second_id = compact.classes_by_name("Second")[0];
        let second_symbol = compact.symbol(second_id).unwrap();
        assert_eq!(second_symbol.name.as_deref(), Some("Second"));
        assert_eq!(second_symbol.kind, SymbolKind::Class);
        assert_eq!(
            second_symbol.detail.base_type.as_deref(),
            Some("SecondBase")
        );

        let field_id = compact.fields_by_owner_name("Second", "m_Value")[0];
        let field = compact.symbol(field_id).unwrap();
        assert_eq!(field.name.as_deref(), Some("m_Value"));
        assert_eq!(field.kind, SymbolKind::Field);

        let first_file = compact.file(SourceFileId(0)).unwrap();
        let second_file = compact.file(SourceFileId(1)).unwrap();
        assert_eq!(
            first_file.symbol_start + first_file.symbol_count,
            second_file.symbol_start
        );
        assert_no_dangling_symbol_references(&compact);
    }

    #[test]
    fn pruned_index_remaps_file_local_symbol_ids() {
        let catalog = catalog(
            r#"class Example
{
	void First()
	{
		int localValue;
	}

	void Second(string name);
}
"#,
            game_metadata("Game/Example.c"),
        );
        let pruned = SymbolIndex::from_catalogs([&catalog]).without_local_variables();

        let first = pruned.methods_by_owner_name("Example", "First")[0];
        let second = pruned.methods_by_owner_name("Example", "Second")[0];
        assert_ne!(first.symbol_id, second.symbol_id);
        assert_eq!(
            pruned.callable_signature(second).as_deref(),
            Some("Example.Second(string name) -> void")
        );
        for file in pruned.files() {
            for local_id in 0..file.symbol_count {
                let id = GlobalSymbolId {
                    file_id: file.id,
                    symbol_id: SymbolId(local_id),
                };
                assert!(pruned.symbol(id).is_some());
            }
        }
        assert_no_dangling_symbol_references(&pruned);
    }

    fn catalog(source: &str, metadata: SourceFileMetadata) -> SymbolCatalog<'_> {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        SymbolCatalog::from_ast_with_metadata(source, &ast, metadata)
    }

    fn game_metadata(path: &str) -> SourceFileMetadata {
        let relative_path = PathBuf::from(path);
        let mut category = source_category_for_path(SourceKind::GameData, Some(&relative_path));
        if category == SourceCategory::Unknown {
            category = SourceCategory::Game;
        }
        SourceFileMetadata {
            kind: SourceKind::GameData,
            category,
            absolute_path: Some(PathBuf::from("C:/game").join(path)),
            root_path: Some(PathBuf::from("C:/game")),
            relative_path: Some(relative_path),
            priority: SOURCE_PRIORITY_GAME_DATA,
        }
    }

    fn workspace_metadata(path: &str) -> SourceFileMetadata {
        let relative_path = PathBuf::from(path);
        SourceFileMetadata {
            kind: SourceKind::Workspace,
            category: SourceCategory::Workspace,
            absolute_path: Some(PathBuf::from("C:/workspace").join(path)),
            root_path: Some(PathBuf::from("C:/workspace")),
            relative_path: Some(relative_path),
            priority: SOURCE_PRIORITY_WORKSPACE,
        }
    }

    fn member_names(index: &SymbolIndex, members: &[GlobalSymbolId]) -> Vec<String> {
        members
            .iter()
            .filter_map(|id| index.symbol(*id))
            .filter_map(|symbol| symbol.name.clone())
            .collect()
    }

    fn assert_no_dangling_symbol_references(index: &SymbolIndex) {
        for symbol in index.symbols() {
            assert_eq!(
                index.symbol(symbol.id).map(|found| found.id),
                Some(symbol.id)
            );
            if let Some(parent) = symbol.parent {
                assert!(
                    index.symbol(parent).is_some(),
                    "dangling parent {:?} for {:?}",
                    parent,
                    symbol.id
                );
            }
            for child in index.children(symbol.id) {
                assert!(
                    index.symbol(*child).is_some(),
                    "dangling child {:?} for {:?}",
                    child,
                    symbol.id
                );
                assert_eq!(index.symbol(*child).unwrap().parent, Some(symbol.id));
            }
        }
    }
}
