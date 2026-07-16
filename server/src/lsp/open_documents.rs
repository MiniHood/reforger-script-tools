use crate::ast::AstSourceFile;
use crate::index::SymbolIndex;
use crate::model::{SourceFileMetadata, SymbolCatalog};
use crate::parser::parse_source;
use crate::scope::LexicalScopeModel;
use crate::syntax::{Parse, ParseDiagnostic};

use super::semantic_tokens::LspSemanticTokenProjection;
use super::LspDocumentSymbol;

pub(crate) struct OpenDocument {
    pub(crate) text: String,
    pub(crate) version: Option<i32>,
    pub(crate) revision: u64,
    pub(crate) analysis: FileIndexAnalysis,
    document_symbols: Vec<LspDocumentSymbol>,
    pub(crate) semantic_tokens: SemanticTokenCache,
}

impl OpenDocument {
    pub(crate) fn new(text: String, version: Option<i32>, revision: u64) -> Self {
        let analysis = file_index_for_source(&text);
        Self {
            text,
            version,
            revision,
            analysis,
            document_symbols: Vec::new(),
            semantic_tokens: SemanticTokenCache::default(),
        }
    }

    pub(crate) fn replace(&mut self, text: String, version: Option<i32>) {
        self.text = text;
        self.version = version;
        self.revision += 1;
        self.analysis = file_index_for_source(&self.text);
        self.document_symbols.clear();
        self.semantic_tokens = SemanticTokenCache::default();
    }

    pub(crate) fn set_document_symbols(&mut self, symbols: Vec<LspDocumentSymbol>) {
        self.document_symbols = symbols;
    }

    pub(crate) fn document_symbols(&self) -> &[LspDocumentSymbol] {
        &self.document_symbols
    }
}

#[derive(Default)]
pub(crate) struct SemanticTokenCache {
    rich_revision: Option<u64>,
    rich_external_generation: Option<u64>,
    rich_projection: Option<LspSemanticTokenProjection>,
    pending_revision: Option<u64>,
    pending_external_generation: Option<u64>,
}

impl SemanticTokenCache {
    pub(crate) fn rich_for_revision_and_external_generation(
        &self,
        revision: u64,
        external_generation: u64,
    ) -> Option<&LspSemanticTokenProjection> {
        (self.rich_revision == Some(revision)
            && self.rich_external_generation == Some(external_generation))
        .then_some(self.rich_projection.as_ref())
        .flatten()
    }

    pub(crate) fn set_rich(
        &mut self,
        revision: u64,
        external_generation: u64,
        projection: LspSemanticTokenProjection,
    ) {
        self.rich_revision = Some(revision);
        self.rich_external_generation = Some(external_generation);
        self.rich_projection = Some(projection);
        self.pending_revision = None;
        self.pending_external_generation = None;
    }

    pub(crate) fn pending_for_revision_and_external_generation(
        &self,
        revision: u64,
        external_generation: u64,
    ) -> bool {
        self.pending_revision == Some(revision)
            && self.pending_external_generation == Some(external_generation)
    }

    pub(crate) fn mark_pending(&mut self, revision: u64, external_generation: u64) {
        self.pending_revision = Some(revision);
        self.pending_external_generation = Some(external_generation);
    }
}

#[derive(Clone)]
pub struct FileIndexAnalysis {
    pub(crate) parse: Parse,
    pub(crate) index: SymbolIndex,
    pub(crate) scope: LexicalScopeModel,
    pub(crate) parse_diagnostics: usize,
    pub(crate) diagnostics: Vec<ParseDiagnostic>,
}

pub fn file_index_for_source(source: &str) -> FileIndexAnalysis {
    let parse = parse_source(source);
    let parse_diagnostics = parse.diagnostics.len();
    let diagnostics = parse.diagnostics.clone();
    let ast = AstSourceFile::new(source, &parse);
    let catalog = SymbolCatalog::from_ast_with_metadata(
        source,
        &ast,
        SourceFileMetadata {
            kind: crate::model::SourceKind::Workspace,
            category: crate::model::SourceCategory::Workspace,
            absolute_path: None,
            root_path: None,
            relative_path: None,
            priority: crate::model::SOURCE_PRIORITY_WORKSPACE,
        },
    );
    let mut index = SymbolIndex::default();
    index.add_catalog(&catalog);
    let scope = LexicalScopeModel::from_parse_and_index(&parse, &index);
    FileIndexAnalysis {
        parse,
        index,
        scope,
        parse_diagnostics,
        diagnostics,
    }
}
