use super::external_overlay::ExternalIndexSnapshot;
use super::open_documents::{FileIndexAnalysis, ForegroundQuerySnapshot, OpenDocument};
use crate::analysis_runtime::QueryQuality;

/// A request-local view of one immutable document snapshot and one captured
/// external-index snapshot. Request routing admits this before projecting any
/// source-backed LSP result.
pub(super) struct DocumentQuery<'a> {
    pub document: &'a OpenDocument,
    pub external_indexes: ExternalIndexSnapshot,
}

pub(super) enum DocumentQueryState<'a> {
    Cached(&'a FileIndexAnalysis),
    Foreground(&'a ForegroundQuerySnapshot),
    Pending,
}

impl<'a> DocumentQuery<'a> {
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
        match Self::state_for(self.document) {
            DocumentQueryState::Cached(_) => QueryQuality::Exact,
            DocumentQueryState::Foreground(_) | DocumentQueryState::Pending => {
                QueryQuality::Unavailable
            }
        }
    }
}
