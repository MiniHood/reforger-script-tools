use crate::index::{GlobalSymbolId, SymbolIndex};
use crate::lexer::{lex, TextSpan, TokenKind};
use crate::model::SymbolKind;
use crate::resolver::{CandidateSource, IdentifierContext, ReferenceResolver, ResolutionReason};
use crate::scope::LexicalScopeModel;
use crate::syntax::Parse;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSearchResult {
    pub target: GlobalSymbolId,
    pub references: Vec<SymbolReference>,
    pub identifiers_scanned: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolReference {
    pub token_text: String,
    pub span: TextSpan,
    pub reason: ResolutionReason,
    pub candidate_count: usize,
    pub is_declaration: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReferenceScan {
    pub references_by_target: BTreeMap<GlobalSymbolId, Vec<SymbolReference>>,
    pub unresolved: Vec<UnresolvedReferenceToken>,
    pub external_references: usize,
    pub identifiers_scanned: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedReferenceToken {
    pub token_text: String,
    pub span: TextSpan,
    pub reason: ResolutionReason,
    pub identifier_context: IdentifierContext,
    pub candidate_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLocalRenameAnalysis {
    pub target: Option<RenameTarget>,
    pub references: Vec<SymbolReference>,
    pub safety: RenameSafety,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameTarget {
    pub id: GlobalSymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub span: TextSpan,
    pub selection_span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameSafety {
    pub can_rename: bool,
    pub reasons: Vec<String>,
    pub same_name_symbol_count: usize,
    pub declaration_reference_count: usize,
    pub usage_reference_count: usize,
}

pub fn find_file_local_references(
    source: &str,
    index: &SymbolIndex,
    parse: &Parse,
    scope: &LexicalScopeModel,
    target: GlobalSymbolId,
) -> ReferenceSearchResult {
    let resolver = ReferenceResolver::new_with_parse_and_scope(source, index, parse, scope, None);
    let scan = scan_file_local_references_with_resolver(source, &resolver);

    ReferenceSearchResult {
        target,
        references: scan
            .references_by_target
            .get(&target)
            .cloned()
            .unwrap_or_default(),
        identifiers_scanned: scan.identifiers_scanned,
    }
}

pub fn scan_file_local_references(
    source: &str,
    index: &SymbolIndex,
    parse: &Parse,
    scope: &LexicalScopeModel,
) -> FileReferenceScan {
    scan_file_local_references_with_external(source, index, parse, scope, None)
}

pub fn scan_file_local_references_with_external(
    source: &str,
    index: &SymbolIndex,
    parse: &Parse,
    scope: &LexicalScopeModel,
    external_index: Option<&SymbolIndex>,
) -> FileReferenceScan {
    let resolver =
        ReferenceResolver::new_with_parse_and_scope(source, index, parse, scope, external_index);
    scan_file_local_references_with_resolver(source, &resolver)
}

fn scan_file_local_references_with_resolver(
    source: &str,
    resolver: &ReferenceResolver<'_, '_>,
) -> FileReferenceScan {
    let mut references_by_target = BTreeMap::<GlobalSymbolId, Vec<SymbolReference>>::new();
    let mut unresolved = Vec::new();
    let mut external_references = 0;
    let mut identifiers_scanned = 0;

    for token in lex(source) {
        if token.kind != TokenKind::Identifier {
            continue;
        }
        identifiers_scanned += 1;
        let Some(resolution) = resolver.resolve_at_offset(token.span.start) else {
            continue;
        };
        let candidate_count = resolution.candidates.len();
        let Some(selected) = resolution.selected.as_ref() else {
            unresolved.push(UnresolvedReferenceToken {
                token_text: resolution.token_text,
                span: resolution.token_span,
                reason: resolution.reason,
                identifier_context: resolution.identifier_context,
                candidate_count,
            });
            continue;
        };
        match selected.source {
            CandidateSource::FileLocal => {
                references_by_target
                    .entry(selected.id)
                    .or_default()
                    .push(SymbolReference {
                        token_text: resolution.token_text,
                        span: resolution.token_span,
                        reason: resolution.reason,
                        candidate_count,
                        is_declaration: resolution.reason == ResolutionReason::DeclarationHit,
                    });
            }
            CandidateSource::External => {
                external_references += 1;
            }
        }
    }

    FileReferenceScan {
        references_by_target,
        unresolved,
        external_references,
        identifiers_scanned,
    }
}

pub fn analyze_file_local_rename_at_offset(
    source: &str,
    index: &SymbolIndex,
    parse: &Parse,
    scope: &LexicalScopeModel,
    offset: usize,
) -> FileLocalRenameAnalysis {
    let resolver = ReferenceResolver::new_with_parse_and_scope(source, index, parse, scope, None);
    let Some(resolution) = resolver.resolve_at_offset(offset) else {
        return rename_analysis_without_target("offset does not resolve to a symbol");
    };
    let Some(selected) = resolution.selected else {
        return rename_analysis_without_target(format!(
            "unresolved identifier: {}",
            resolution.reason.as_str()
        ));
    };
    if selected.source != CandidateSource::FileLocal {
        return rename_analysis_without_target("selected symbol is external");
    }
    let Some(symbol) = index.symbol(selected.id) else {
        return rename_analysis_without_target("selected symbol is missing from file index");
    };
    let Some(name) = symbol.name.clone() else {
        return rename_analysis_without_target("selected symbol has no stable name");
    };

    let scan = scan_file_local_references_with_resolver(source, &resolver);
    let references = scan
        .references_by_target
        .get(&selected.id)
        .cloned()
        .unwrap_or_default();
    let declaration_reference_count = references
        .iter()
        .filter(|reference| reference.is_declaration)
        .count();
    let usage_reference_count = references.len().saturating_sub(declaration_reference_count);
    let same_name_symbol_count = index
        .symbols()
        .iter()
        .filter(|candidate| candidate.name.as_deref() == Some(&name))
        .count();
    let mut reasons = Vec::new();
    if declaration_reference_count == 0 {
        reasons.push("target declaration was not found in reference scan".to_string());
    }
    if same_name_symbol_count > 1 {
        reasons.push(format!(
            "{same_name_symbol_count} file-local symbols share the same name"
        ));
    }

    FileLocalRenameAnalysis {
        target: Some(RenameTarget {
            id: selected.id,
            name,
            kind: symbol.kind,
            span: symbol.span,
            selection_span: symbol.selection_span,
        }),
        references,
        safety: RenameSafety {
            can_rename: reasons.is_empty(),
            reasons,
            same_name_symbol_count,
            declaration_reference_count,
            usage_reference_count,
        },
    }
}

fn rename_analysis_without_target(reason: impl Into<String>) -> FileLocalRenameAnalysis {
    FileLocalRenameAnalysis {
        target: None,
        references: Vec::new(),
        safety: RenameSafety {
            can_rename: false,
            reasons: vec![reason.into()],
            same_name_symbol_count: 0,
            declaration_reference_count: 0,
            usage_reference_count: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::SymbolIndex;
    use crate::model::{SourceFileMetadata, SymbolKind};
    use crate::parser::parse_source;
    use crate::semantic_file::SemanticFile;

    fn analysis(source: &str) -> (Parse, SymbolIndex, LexicalScopeModel) {
        let parse = parse_source(source);
        let semantic_file = SemanticFile::build(source, &parse);
        let index =
            SymbolIndex::from_semantic_files([(&semantic_file, SourceFileMetadata::unknown())]);
        let scope = LexicalScopeModel::from_parse_and_index(&parse, &index);
        (parse, index, scope)
    }

    fn symbol_named(index: &SymbolIndex, name: &str, kind: SymbolKind) -> GlobalSymbolId {
        index
            .symbols()
            .iter()
            .find(|symbol| symbol.name.as_deref() == Some(name) && symbol.kind == kind)
            .map(|symbol| symbol.id)
            .expect("symbol should exist")
    }

    #[test]
    fn finds_file_local_parameter_references_through_resolver() {
        let source = r#"
class Example {
    void Run(int value)
    {
        int local = value;
        local = value;
        Print(local);
    }
}
"#;
        let (parse, index, scope) = analysis(source);
        let target = symbol_named(&index, "value", SymbolKind::Parameter);
        let result = find_file_local_references(source, &index, &parse, &scope, target);
        assert_eq!(result.references.len(), 3);
        assert_eq!(
            result
                .references
                .iter()
                .filter(|reference| reference.is_declaration)
                .count(),
            1
        );
    }

    #[test]
    fn finds_field_references_without_matching_shadowing_locals() {
        let source = r#"
class Example {
    int value;
    void Run(int value)
    {
        this.value = value;
    }
}
"#;
        let (parse, index, scope) = analysis(source);
        let target = symbol_named(&index, "value", SymbolKind::Field);
        let result = find_file_local_references(source, &index, &parse, &scope, target);
        assert_eq!(result.references.len(), 2);
        assert!(result
            .references
            .iter()
            .any(|reference| reference.is_declaration));
        assert!(result
            .references
            .iter()
            .any(|reference| reference.reason == ResolutionReason::ReceiverMember));
    }

    #[test]
    fn scans_all_file_local_references_in_one_pass() {
        let source = r#"
class Example {
    int value;
    void Run()
    {
        value = value + 1;
    }
}
"#;
        let (parse, index, scope) = analysis(source);
        let target = symbol_named(&index, "value", SymbolKind::Field);
        let scan = scan_file_local_references(source, &index, &parse, &scope);
        let references = scan
            .references_by_target
            .get(&target)
            .expect("field references should be collected");
        assert_eq!(references.len(), 3);
        assert!(references.iter().any(|reference| reference.is_declaration));
    }

    #[test]
    fn rename_analysis_selects_target_and_reports_safety_metadata() {
        let source = r#"
class Example {
    void Run(int value)
    {
        int local = value;
        local = value;
    }
}
"#;
        let (parse, index, scope) = analysis(source);
        let offset = source.find("local = value").unwrap();
        let analysis = analyze_file_local_rename_at_offset(source, &index, &parse, &scope, offset);

        assert_eq!(analysis.target.as_ref().unwrap().name, "local");
        assert_eq!(
            analysis.target.as_ref().unwrap().kind,
            SymbolKind::LocalVariable
        );
        assert_eq!(analysis.references.len(), 2);
        assert!(analysis.safety.can_rename);
        assert_eq!(analysis.safety.declaration_reference_count, 1);
        assert_eq!(analysis.safety.usage_reference_count, 1);
    }
}
