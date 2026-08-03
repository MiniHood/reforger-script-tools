use super::external_overlay::ExternalIndexSnapshot;
use super::open_documents::{FileIndexAnalysis, ForegroundQuerySnapshot, OpenDocument};
use super::{
    document_symbols_from_cached_analysis, lexical_document_symbols_for_snapshot, LspDocumentSymbol,
};
use crate::analysis_runtime::QueryQuality;
use std::time::Instant;

/// A request-local view of one immutable document snapshot and one captured
/// external-index snapshot. Request routing admits this before projecting any
/// source-backed LSP result.
pub(super) struct DocumentQuery<'a> {
    pub document: &'a OpenDocument,
    pub external_indexes: ExternalIndexSnapshot,
    pub(super) state: DocumentQueryState<'a>,
}

#[derive(Clone, Copy)]
pub(super) enum DocumentQueryState<'a> {
    Cached(&'a FileIndexAnalysis),
    Foreground(&'a ForegroundQuerySnapshot),
    Pending,
}

/// Immutable outline projection captured from one `DocumentQuery`.
pub(super) struct DocumentSymbolProjection {
    pub(super) symbols: Vec<LspDocumentSymbol>,
    pub(super) bytes: usize,
    pub(super) revision: u64,
    pub(super) parse_diagnostics: usize,
    pub(super) cached: bool,
    pub(super) quality: &'static str,
    pub(super) projection_ms: u128,
}

impl<'a> DocumentQuery<'a> {
    pub fn new(document: &'a OpenDocument, external_indexes: ExternalIndexSnapshot) -> Self {
        let state = Self::state_for(document);
        Self {
            document,
            external_indexes,
            state,
        }
    }

    pub fn state_for(document: &'a OpenDocument) -> DocumentQueryState<'a> {
        if document.analysis_ready() {
            DocumentQueryState::Cached(document.analysis())
        } else if let Some(foreground) = document.foreground() {
            DocumentQueryState::Foreground(foreground)
        } else {
            DocumentQueryState::Pending
        }
    }

    pub fn quality(&self) -> QueryQuality {
        match self.state() {
            DocumentQueryState::Cached(_) => QueryQuality::Exact,
            DocumentQueryState::Foreground(_) | DocumentQueryState::Pending => {
                QueryQuality::Unavailable
            }
        }
    }

    pub fn state(&self) -> DocumentQueryState<'a> {
        self.state
    }

    pub(super) fn document_symbols(&self) -> DocumentSymbolProjection {
        let document = self.document;
        let projection_start = Instant::now();
        if let DocumentQueryState::Cached(analysis) = self.state() {
            let cached = document.document_symbols_ready();
            let symbols = if cached {
                document.document_symbols().to_vec()
            } else {
                document_symbols_from_cached_analysis(&document.snapshot.text(), analysis)
            };
            DocumentSymbolProjection {
                symbols,
                bytes: document.snapshot.text().len(),
                revision: document.snapshot.revision(),
                parse_diagnostics: analysis.parse_diagnostics,
                cached,
                quality: "Exact",
                projection_ms: projection_start.elapsed().as_millis(),
            }
        } else {
            DocumentSymbolProjection {
                symbols: lexical_document_symbols_for_snapshot(&document.snapshot),
                bytes: document.snapshot.text().len(),
                revision: document.snapshot.revision(),
                parse_diagnostics: 0,
                cached: false,
                quality: "Unavailable",
                projection_ms: projection_start.elapsed().as_millis(),
            }
        }
    }
}
