use crate::ast::AstSourceFile;
use crate::index::SymbolIndex;
use crate::model::{SourceFileMetadata, SymbolCatalog};
use crate::parser::parse_source;
use crate::scope::LexicalScopeModel;
use crate::syntax::{Parse, ParseDiagnostic};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;

use super::semantic_tokens::LspSemanticTokenProjection;
use super::LspDocumentSymbol;

pub(crate) struct OpenDocument {
    pub(crate) text: String,
    pub(crate) version: Option<i32>,
    pub(crate) revision: u64,
    pub(crate) analysis: FileIndexAnalysis,
    pub(crate) analysis_timings: FileIndexAnalysisTimings,
    document_symbols: Vec<LspDocumentSymbol>,
    document_symbols_ready: bool,
    pub(crate) semantic_tokens: SemanticTokenCache,
}

impl OpenDocument {
    pub(crate) fn new(text: String, version: Option<i32>, revision: u64) -> Self {
        let (analysis, analysis_timings) = file_index_for_source_with_timings(&text);
        Self {
            text,
            version,
            revision,
            analysis,
            analysis_timings,
            document_symbols: Vec::new(),
            document_symbols_ready: false,
            semantic_tokens: SemanticTokenCache::default(),
        }
    }

    pub(crate) fn replace(&mut self, text: String, version: Option<i32>) {
        self.text = text;
        self.version = version;
        self.revision += 1;
        (self.analysis, self.analysis_timings) = file_index_for_source_with_timings(&self.text);
        self.document_symbols.clear();
        self.document_symbols_ready = false;
        self.semantic_tokens.cancel_pending();
        self.semantic_tokens = SemanticTokenCache::default();
    }

    pub(crate) fn set_document_symbols(&mut self, symbols: Vec<LspDocumentSymbol>) {
        self.document_symbols = symbols;
        self.document_symbols_ready = true;
    }

    pub(crate) fn document_symbols(&self) -> &[LspDocumentSymbol] {
        &self.document_symbols
    }

    pub(crate) fn document_symbols_ready(&self) -> bool {
        self.document_symbols_ready
    }
}

#[derive(Default)]
pub(crate) struct SemanticTokenCache {
    rich_revision: Option<u64>,
    rich_external_generation: Option<u64>,
    rich_projection: Option<LspSemanticTokenProjection>,
    pending_revision: Option<u64>,
    pending_external_generation: Option<u64>,
    pending_cancel: Option<Arc<AtomicBool>>,
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
        self.pending_cancel = None;
    }

    pub(crate) fn pending_for_revision_and_external_generation(
        &self,
        revision: u64,
        external_generation: u64,
    ) -> bool {
        self.pending_revision == Some(revision)
            && self.pending_external_generation == Some(external_generation)
    }

    pub(crate) fn mark_pending(
        &mut self,
        revision: u64,
        external_generation: u64,
    ) -> Arc<AtomicBool> {
        self.cancel_pending();
        let cancel = Arc::new(AtomicBool::new(false));
        self.pending_revision = Some(revision);
        self.pending_external_generation = Some(external_generation);
        self.pending_cancel = Some(cancel.clone());
        cancel
    }

    pub(crate) fn cancel_pending(&mut self) {
        if let Some(cancel) = self.pending_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.pending_revision = None;
        self.pending_external_generation = None;
    }

    pub(crate) fn cancel_pending_for_other_external_generation(
        &mut self,
        external_generation: u64,
    ) {
        if self
            .pending_external_generation
            .is_some_and(|pending_generation| pending_generation != external_generation)
        {
            self.cancel_pending();
        }
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

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FileIndexAnalysisTimings {
    pub(crate) parse_ms: u128,
    pub(crate) catalog_ms: u128,
    pub(crate) index_ms: u128,
    pub(crate) scope_ms: u128,
    pub(crate) total_ms: u128,
}

pub fn file_index_for_source(source: &str) -> FileIndexAnalysis {
    file_index_for_source_with_timings(source).0
}

pub(crate) fn file_index_for_source_with_timings(
    source: &str,
) -> (FileIndexAnalysis, FileIndexAnalysisTimings) {
    let total_start = Instant::now();
    let parse_start = Instant::now();
    let parse = parse_source(source);
    let parse_ms = parse_start.elapsed().as_millis();
    let parse_diagnostics = parse.diagnostics.len();
    let diagnostics = parse.diagnostics.clone();
    let ast = AstSourceFile::new(source, &parse);
    let catalog_start = Instant::now();
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
    let catalog_ms = catalog_start.elapsed().as_millis();
    let index_start = Instant::now();
    let mut index = SymbolIndex::default();
    index.add_catalog(&catalog);
    let index_ms = index_start.elapsed().as_millis();
    let scope_start = Instant::now();
    let scope = LexicalScopeModel::from_parse_and_index(&parse, &index);
    let scope_ms = scope_start.elapsed().as_millis();
    (
        FileIndexAnalysis {
            parse,
            index,
            scope,
            parse_diagnostics,
            diagnostics,
        },
        FileIndexAnalysisTimings {
            parse_ms,
            catalog_ms,
            index_ms,
            scope_ms,
            total_ms: total_start.elapsed().as_millis(),
        },
    )
}
