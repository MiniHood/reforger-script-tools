use crate::index::SymbolIndex;
use crate::model::SourceKind;
use crate::resolver::ReferenceCandidate;

#[derive(Clone, Copy)]
pub(crate) struct ExternalIndexes<'a> {
    workspace: Option<&'a SymbolIndex>,
    game_data: Option<&'a SymbolIndex>,
}

impl<'a> ExternalIndexes<'a> {
    pub(crate) const fn new(
        workspace: Option<&'a SymbolIndex>,
        game_data: Option<&'a SymbolIndex>,
    ) -> Self {
        Self {
            workspace,
            game_data,
        }
    }

    pub(crate) fn ordered(self) -> Vec<&'a SymbolIndex> {
        self.workspace.into_iter().chain(self.game_data).collect()
    }

    pub(crate) fn for_candidate(self, candidate: &ReferenceCandidate) -> Option<&'a SymbolIndex> {
        match candidate.source_kind {
            SourceKind::Workspace => self.workspace,
            SourceKind::GameData => self.game_data,
            SourceKind::Unknown | SourceKind::Fixture => None,
        }
        .or(self.workspace)
        .or(self.game_data)
    }
}

#[cfg(test)]
mod tests {
    use super::ExternalIndexes;
    use crate::index::SymbolIndex;

    #[test]
    fn orders_workspace_ahead_of_game_data_for_same_symbol_lookup() {
        let workspace = SymbolIndex::default();
        let game_data = SymbolIndex::default();

        let ordered = ExternalIndexes::new(Some(&workspace), Some(&game_data)).ordered();

        assert_eq!(ordered.len(), 2);
        assert!(std::ptr::eq(ordered[0], &workspace));
        assert!(std::ptr::eq(ordered[1], &game_data));
    }
}
