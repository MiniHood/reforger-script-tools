use crate::analysis_runtime::DocumentSnapshot;
use crate::ast::AstSourceFile;
use crate::index::SymbolIndex;
use crate::lexer::{lex, Token};
use crate::model::SourceFileMetadata;
use crate::parser::parse_source;
use crate::scope::LexicalScopeModel;
use crate::semantic_file::SemanticFile;
use crate::syntax::{Parse, ParseDiagnostic};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;

use super::semantic_tokens::LspSemanticTokenProjection;
use super::LspDocumentSymbol;

pub(crate) struct OpenDocument {
    /// The runtime-owned immutable source identity.  Analysis and caches below
    /// are derived state only and may never outlive this revision.
    pub(crate) snapshot: DocumentSnapshot,
    // Compatibility views share the snapshot's `Arc<str>` allocation.  They
    // exist only while feature adapters migrate to snapshot access; ownership
    // and admission remain in `AnalysisRuntime`.
    pub(crate) text: Arc<str>,
    pub(crate) version: i32,
    pub(crate) revision: u64,
    /// Current-revision parser output, retained independently from deferred
    /// semantic/index analysis so parser diagnostics never wait for it.
    syntax: Parse,
    analysis: Option<FileIndexAnalysis>,
    analysis_timings: Option<FileIndexAnalysisTimings>,
    analysis_rejected: bool,
    document_symbols: Vec<LspDocumentSymbol>,
    document_symbols_ready: bool,
    pub(crate) semantic_tokens: SemanticTokenCache,
}

impl OpenDocument {
    pub(crate) fn new(snapshot: DocumentSnapshot) -> Self {
        let revision = snapshot.revision();
        let mut document = Self::pending(snapshot);
        let (analysis, analysis_timings) =
            file_index_for_source_with_timings(document.snapshot.text());
        assert!(document.install_analysis(revision, analysis, analysis_timings));
        document
    }

    /// Creates a cache whose source snapshot is authoritative but whose
    /// compiler analysis has not run yet. Feature dispatch must therefore use
    /// only a foreground-safe projection until the worker installs this
    /// revision; no legacy empty-file analysis exists in this state.
    pub(crate) fn pending(snapshot: DocumentSnapshot) -> Self {
        let text = snapshot.text_arc();
        let version = snapshot.version();
        let revision = snapshot.revision();
        let syntax = parse_source(snapshot.text());
        Self {
            snapshot,
            text,
            version,
            revision,
            syntax,
            analysis: None,
            analysis_timings: None,
            analysis_rejected: false,
            document_symbols: Vec::new(),
            document_symbols_ready: false,
            semantic_tokens: SemanticTokenCache::default(),
        }
    }

    pub(crate) fn replace(&mut self, snapshot: DocumentSnapshot) {
        self.text = snapshot.text_arc();
        self.version = snapshot.version();
        self.revision = snapshot.revision();
        self.snapshot = snapshot;
        self.syntax = parse_source(self.snapshot.text());
        self.analysis = None;
        self.analysis_timings = None;
        self.analysis_rejected = false;
        self.document_symbols.clear();
        self.document_symbols_ready = false;
        self.semantic_tokens.cancel_pending();
        self.semantic_tokens = SemanticTokenCache::default();
    }

    pub(crate) fn analysis_ready(&self) -> bool {
        self.analysis.is_some()
    }

    pub(crate) fn syntax(&self) -> &Parse {
        &self.syntax
    }

    pub(crate) fn analysis(&self) -> &FileIndexAnalysis {
        self.analysis
            .as_ref()
            .expect("ready analysis is required by this feature path")
    }

    pub(crate) fn analysis_timings(&self) -> FileIndexAnalysisTimings {
        self.analysis_timings
            .expect("ready analysis timings are required by this feature path")
    }

    pub(crate) fn mark_analysis_pending(&mut self) {
        self.analysis_rejected = false;
    }

    /// Marks the matching revision unavailable after deterministic runtime
    /// overload. Request dispatch must respond rather than retaining an
    /// unbounded deferred request that can never be replayed.
    pub(crate) fn reject_pending_analysis(&mut self) {
        self.analysis_rejected = true;
    }

    pub(crate) fn analysis_rejected(&self) -> bool {
        self.analysis_rejected
    }

    pub(crate) fn install_analysis(
        &mut self,
        revision: u64,
        analysis: FileIndexAnalysis,
        analysis_timings: FileIndexAnalysisTimings,
    ) -> bool {
        if revision != self.snapshot.revision() {
            return false;
        }
        self.analysis = Some(analysis);
        self.analysis_timings = Some(analysis_timings);
        self.analysis_rejected = false;
        true
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
        cancel: Arc<AtomicBool>,
    ) {
        self.cancel_pending();
        self.pending_revision = Some(revision);
        self.pending_external_generation = Some(external_generation);
        self.pending_cancel = Some(cancel);
    }

    pub(crate) fn cancel_pending(&mut self) {
        if let Some(cancel) = self.pending_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.pending_revision = None;
        self.pending_external_generation = None;
    }

    pub(crate) fn cancel_pending_if_matches(&mut self, revision: u64, external_generation: u64) {
        if self.pending_revision == Some(revision)
            && self.pending_external_generation == Some(external_generation)
        {
            self.cancel_pending();
        }
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
    pub(crate) lexer_tokens: Vec<Token>,
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
    let lexer_tokens = lex(source);
    let parse_start = Instant::now();
    let parse = parse_source(source);
    let parse_ms = parse_start.elapsed().as_millis();
    let parse_diagnostics = parse.diagnostics.len();
    let diagnostics = parse.diagnostics.clone();
    let ast = AstSourceFile::new(source, &parse);
    let catalog_start = Instant::now();
    let semantic_file = SemanticFile::build(source, &ast);
    let catalog_ms = catalog_start.elapsed().as_millis();
    let index_start = Instant::now();
    let mut index = SymbolIndex::default();
    index.add_semantic_file(
        &semantic_file,
        SourceFileMetadata {
            kind: crate::model::SourceKind::Workspace,
            category: crate::model::SourceCategory::Workspace,
            absolute_path: None,
            root_path: None,
            relative_path: None,
            priority: crate::model::SOURCE_PRIORITY_WORKSPACE,
        },
    );
    let index_ms = index_start.elapsed().as_millis();
    let scope_start = Instant::now();
    let scope = LexicalScopeModel::from_parse_and_index(&parse, &index);
    let scope_ms = scope_start.elapsed().as_millis();
    (
        FileIndexAnalysis {
            parse,
            lexer_tokens,
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
