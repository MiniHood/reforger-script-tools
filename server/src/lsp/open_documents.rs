use crate::analysis_runtime::{DocumentSnapshot, PositionIndex};
use crate::ast::{AstSourceFile, ClassMember, Declaration, MethodDecl};
use crate::index::SymbolIndex;
use crate::lexer::{lex, Keyword, TextSpan, Token, TokenKind};
use crate::model::{SourceFileMetadata, SymbolKind};
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
    syntax: Option<Parse>,
    /// Foreground-only query facts for this exact revision. Semantic work is
    /// deliberately not allowed to manufacture or replace this state.
    foreground: Option<ForegroundQuerySnapshot>,
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
        // Deterministic in-process fixtures construct ready documents without
        // the production executor. The transport path never calls this: it
        // installs the same state through a `TaskClass::Foreground` worker.
        let positions = PositionIndex::new(document.snapshot.text());
        let lexer_tokens = lex(document.snapshot.text());
        let syntax = parse_source(document.snapshot.text());
        assert!(document.install_foreground(revision, positions, lexer_tokens, syntax));
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
        Self {
            snapshot,
            text,
            version,
            revision,
            syntax: None,
            foreground: None,
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
        self.syntax = None;
        self.foreground = None;
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

    pub(crate) fn syntax(&self) -> Option<&Parse> {
        self.syntax.as_ref()
    }

    pub(crate) fn parse_diagnostic_count(&self) -> usize {
        self.syntax().map_or(0, |parse| parse.diagnostics.len())
    }

    pub(crate) fn install_foreground(
        &mut self,
        revision: u64,
        positions: PositionIndex,
        lexer_tokens: Vec<Token>,
        syntax: Parse,
    ) -> bool {
        if revision != self.snapshot.revision() {
            return false;
        }
        // A duplicate foreground event is harmless only when the exact
        // revision already owns its immutable position table.
        if !self.snapshot.install_positions(positions) && self.snapshot.positions().is_none() {
            return false;
        }
        self.foreground = Some(ForegroundQuerySnapshot::build(
            self.snapshot.text(),
            lexer_tokens,
            &syntax,
        ));
        self.syntax = Some(syntax);
        true
    }

    pub(crate) fn foreground_ready(&self) -> bool {
        self.foreground.is_some() && self.syntax.is_some() && self.snapshot.positions().is_some()
    }

    pub(crate) fn foreground(&self) -> Option<&ForegroundQuerySnapshot> {
        self.foreground.as_ref()
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

/// The complete token state that may be published for one document revision.
///
/// The lexical baseline is the authoritative first response: it is derived
/// solely from the current snapshot text. A rich projection is an optional
/// replacement overlay, never an input to the lexical pass. The server only
/// supports full semantic-token responses today, so a changed overlay receives
/// a new opaque result id rather than attempting an unsafe delta against a
/// former result.
pub(crate) struct TokenSnapshot {
    revision: u64,
    lexical_baseline: LspSemanticTokenProjection,
    rich_overlay: Option<RichTokenOverlay>,
}

struct RichTokenOverlay {
    external_generation: u64,
    projection: LspSemanticTokenProjection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenResultDisposition {
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenProjectionKind {
    LexicalBaseline,
    RichOverlay,
}

pub(crate) struct TokenSelection<'a> {
    pub(crate) projection: &'a LspSemanticTokenProjection,
    pub(crate) kind: TokenProjectionKind,
    pub(crate) result_id: String,
    pub(crate) disposition: TokenResultDisposition,
}

impl TokenSnapshot {
    fn new(revision: u64, lexical_baseline: LspSemanticTokenProjection) -> Self {
        Self {
            revision,
            lexical_baseline,
            rich_overlay: None,
        }
    }

    fn select(&self, external_generation: u64) -> TokenSelection<'_> {
        if let Some(overlay) = self
            .rich_overlay
            .as_ref()
            .filter(|overlay| overlay.external_generation == external_generation)
        {
            return TokenSelection {
                projection: &overlay.projection,
                kind: TokenProjectionKind::RichOverlay,
                result_id: format!("reforger:{}:rich:{}", self.revision, external_generation),
                disposition: TokenResultDisposition::Full,
            };
        }
        TokenSelection {
            projection: &self.lexical_baseline,
            kind: TokenProjectionKind::LexicalBaseline,
            result_id: format!("reforger:{}:lexical", self.revision),
            disposition: TokenResultDisposition::Full,
        }
    }

    fn set_rich(&mut self, external_generation: u64, projection: LspSemanticTokenProjection) {
        self.rich_overlay = Some(RichTokenOverlay {
            external_generation,
            projection,
        });
    }
}

#[derive(Default)]
pub(crate) struct SemanticTokenCache {
    snapshot: Option<TokenSnapshot>,
    pending_revision: Option<u64>,
    pending_external_generation: Option<u64>,
    pending_cancel: Option<Arc<AtomicBool>>,
}

impl SemanticTokenCache {
    pub(crate) fn select_or_insert_lexical(
        &mut self,
        revision: u64,
        external_generation: u64,
        lexical_baseline: impl FnOnce() -> LspSemanticTokenProjection,
    ) -> TokenSelection<'_> {
        if self
            .snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.revision != revision)
        {
            self.snapshot = Some(TokenSnapshot::new(revision, lexical_baseline()));
        }
        self.discard_rich_for_other_external_generation(external_generation);
        self.snapshot
            .as_ref()
            .expect("lexical token snapshot was just installed")
            .select(external_generation)
    }

    #[cfg(test)]
    pub(crate) fn rich_for_revision_and_external_generation(
        &self,
        revision: u64,
        external_generation: u64,
    ) -> Option<&LspSemanticTokenProjection> {
        self.snapshot
            .as_ref()
            .filter(|snapshot| snapshot.revision == revision)
            .and_then(|snapshot| snapshot.rich_overlay.as_ref())
            .filter(|overlay| overlay.external_generation == external_generation)
            .map(|overlay| &overlay.projection)
    }

    pub(crate) fn set_rich(
        &mut self,
        revision: u64,
        external_generation: u64,
        projection: LspSemanticTokenProjection,
    ) {
        if let Some(snapshot) = self
            .snapshot
            .as_mut()
            .filter(|snapshot| snapshot.revision == revision)
        {
            snapshot.set_rich(external_generation, projection);
        }
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

    /// An external-index generation is part of a rich overlay's identity.
    /// Keeping an unmatched overlay would be harmless at selection time, but
    /// dropping it here makes the cache's retained state match its publishable
    /// state and prevents an old overlay from becoming eligible again.
    pub(crate) fn discard_rich_for_other_external_generation(&mut self, external_generation: u64) {
        if self.snapshot.as_ref().is_some_and(|snapshot| {
            snapshot
                .rich_overlay
                .as_ref()
                .is_some_and(|overlay| overlay.external_generation != external_generation)
        }) {
            self.snapshot
                .as_mut()
                .expect("snapshot was checked above")
                .rich_overlay = None;
        }
    }
}

/// Immutable, current-revision facts built exclusively by the foreground
/// worker. Pending request handlers may query these facts, but must not lex,
/// parse, or walk the CST/AST themselves.
#[derive(Clone)]
pub(crate) struct ForegroundQuerySnapshot {
    tokens: Vec<Token>,
    top_level_declarations: Vec<ForegroundTopLevelDeclaration>,
    callable_declarations: Vec<ForegroundCallableDeclaration>,
}

#[derive(Clone, Debug)]
pub(crate) struct ForegroundTopLevelDeclaration {
    pub(crate) name: String,
    pub(crate) name_span: TextSpan,
    pub(crate) kind: SymbolKind,
}

#[derive(Clone, Debug)]
pub(crate) struct ForegroundCallableDeclaration {
    pub(crate) name: String,
    pub(crate) signature: String,
}

impl ForegroundQuerySnapshot {
    pub(crate) fn build(source: &str, tokens: Vec<Token>, parse: &Parse) -> Self {
        Self {
            top_level_declarations: foreground_top_level_declarations(source, &tokens),
            callable_declarations: foreground_callable_declarations(source, parse),
            tokens,
        }
    }

    pub(crate) fn token_at_offset(&self, offset: usize) -> Option<Token> {
        self.tokens
            .binary_search_by(|token| {
                if token.span.end <= offset {
                    std::cmp::Ordering::Less
                } else if token.span.start > offset {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .ok()
            .and_then(|index| self.tokens.get(index).copied())
    }

    pub(crate) fn top_level_declaration_at_offset(
        &self,
        offset: usize,
    ) -> Option<&ForegroundTopLevelDeclaration> {
        self.top_level_declarations.iter().find(|declaration| {
            declaration.name_span.start <= offset && offset <= declaration.name_span.end
        })
    }

    pub(crate) fn callable_declarations_named<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a ForegroundCallableDeclaration> + 'a {
        self.callable_declarations
            .iter()
            .filter(move |candidate| candidate.name == name)
    }

    pub(crate) fn tokens(&self) -> &[Token] {
        &self.tokens
    }
}

fn foreground_top_level_declarations(
    source: &str,
    tokens: &[Token],
) -> Vec<ForegroundTopLevelDeclaration> {
    let mut declarations = Vec::new();
    let mut brace_depth = 0usize;
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index];
        match token.kind {
            TokenKind::LeftBrace => brace_depth += 1,
            TokenKind::RightBrace => brace_depth = brace_depth.saturating_sub(1),
            TokenKind::Keyword(Keyword::Class | Keyword::Enum | Keyword::Typedef)
                if brace_depth == 0 =>
            {
                let declaration_kind = token.kind;
                if let Some((name, name_span, next_index)) =
                    foreground_top_level_declaration(source, tokens, index, declaration_kind)
                {
                    let kind = match declaration_kind {
                        TokenKind::Keyword(Keyword::Class) => SymbolKind::Class,
                        TokenKind::Keyword(Keyword::Enum) => SymbolKind::Enum,
                        TokenKind::Keyword(Keyword::Typedef) => SymbolKind::Typedef,
                        _ => unreachable!("only declaration keywords reach this branch"),
                    };
                    declarations.push(ForegroundTopLevelDeclaration {
                        name,
                        name_span,
                        kind,
                    });
                    index = next_index;
                    continue;
                }
            }
            _ => {}
        }
        index += 1;
    }
    declarations
}

fn foreground_top_level_declaration(
    source: &str,
    tokens: &[Token],
    keyword_index: usize,
    declaration_kind: TokenKind,
) -> Option<(String, TextSpan, usize)> {
    let mut index = keyword_index + 1;
    let mut typedef_name = None;
    while let Some(token) = tokens.get(index).copied() {
        if token.kind.is_trivia() {
            index += 1;
            continue;
        }
        match declaration_kind {
            TokenKind::Keyword(Keyword::Class | Keyword::Enum) => {
                return (token.kind == TokenKind::Identifier).then(|| {
                    (
                        source[token.span.start..token.span.end].to_string(),
                        token.span,
                        index + 1,
                    )
                });
            }
            TokenKind::Keyword(Keyword::Typedef) => match token.kind {
                TokenKind::Identifier => typedef_name = Some(token),
                TokenKind::Semicolon | TokenKind::Eof => {
                    return typedef_name.map(|name| {
                        (
                            source[name.span.start..name.span.end].to_string(),
                            name.span,
                            index + 1,
                        )
                    });
                }
                TokenKind::LeftBrace | TokenKind::RightBrace => return None,
                _ => {}
            },
            _ => return None,
        }
        index += 1;
    }
    None
}

fn foreground_callable_declarations(
    source: &str,
    parse: &Parse,
) -> Vec<ForegroundCallableDeclaration> {
    let ast = AstSourceFile::new(source, parse);
    ast.declaration_iter()
        .flat_map(|declaration| match declaration {
            Declaration::Function(method) => vec![method],
            Declaration::Class(class) => class
                .members()
                .into_iter()
                .filter_map(|member| match member {
                    ClassMember::Method(method) => Some(method),
                    ClassMember::Field(_) | ClassMember::Empty(_) => None,
                })
                .collect(),
            Declaration::Enum(_) | Declaration::Typedef(_) | Declaration::Field(_) => Vec::new(),
        })
        .filter_map(foreground_callable_declaration)
        .collect()
}

fn foreground_callable_declaration(
    method: MethodDecl<'_, '_>,
) -> Option<ForegroundCallableDeclaration> {
    if method.is_destructor() || !method.parameter_fragments().is_empty() {
        return None;
    }
    let name = method.name()?.text().to_string();
    let return_type = method.return_type_text()?.text().trim().to_string();
    if return_type.is_empty() {
        return None;
    }
    let parameters = method
        .parameters()
        .into_iter()
        .map(|parameter| {
            parameter
                .text()
                .map(|text| text.text().trim().to_string())
                .filter(|text| !text.is_empty())
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ForegroundCallableDeclaration {
        signature: format!("{return_type} {name}({})", parameters.join(", ")),
        name,
    })
}

#[derive(Clone)]
pub struct FileIndexAnalysis {
    pub(crate) parse: Parse,
    pub(crate) lexer_tokens: Vec<Token>,
    /// Immutable compiler-owned declaration and local-binding facts. The
    /// index and lexical scope below are feature compatibility projections of
    /// this semantic authority, never independent declaration discovery.
    pub(crate) semantic: SemanticFile,
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
    let catalog_start = Instant::now();
    let semantic_file = SemanticFile::build(source, &parse);
    let catalog_ms = catalog_start.elapsed().as_millis();
    let index_start = Instant::now();
    let mut index = SymbolIndex::default();
    let local_file_id = index.add_semantic_file(
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
    let scope =
        LexicalScopeModel::from_parse_and_semantics(&parse, &semantic_file, &index, local_file_id);
    let scope_ms = scope_start.elapsed().as_millis();
    (
        FileIndexAnalysis {
            parse,
            lexer_tokens,
            semantic: semantic_file,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis_runtime::DocumentStore;
    use crate::lsp::definition::definition_report_for_pending_snapshot;
    use crate::lsp::hover::hover_report_for_pending_snapshot;
    use crate::lsp::signature_help::signature_help_report_for_pending_snapshot;
    use crate::lsp::LspPosition;

    #[test]
    fn pending_request_projections_do_not_relex_or_rewalk_a_large_snapshot() {
        let mut source = String::from(
            "// pending lexical hover\nclass Pending { void Current(string value) {} void Run() { Current( ); } }\n",
        );
        source.push_str(&"// unrelated foreground payload\n".repeat(40_000));

        let mut store = DocumentStore::new();
        assert_eq!(
            store.upsert("file:///Scripts/Pending.c", 1, source.as_str()),
            crate::analysis_runtime::UpsertOutcome::Accepted
        );
        let snapshot = store.latest("file:///Scripts/Pending.c").unwrap();
        assert!(snapshot.install_positions(PositionIndex::new(snapshot.text())));
        let parse = parse_source(snapshot.text());
        let foreground =
            ForegroundQuerySnapshot::build(snapshot.text(), lex(snapshot.text()), &parse);
        let lex_calls_before_requests = crate::lexer::test_lex_call_count();

        let comment = hover_report_for_pending_snapshot(
            &snapshot,
            &foreground,
            LspPosition {
                line: 0,
                character: 4,
            },
            parse.diagnostics.len(),
        );
        assert!(comment.is_hit());

        let definition = definition_report_for_pending_snapshot(
            &snapshot,
            &foreground,
            snapshot.uri(),
            LspPosition {
                line: 1,
                character: 7,
            },
            parse.diagnostics.len(),
        );
        assert!(definition.is_hit());

        let signature = signature_help_report_for_pending_snapshot(
            &snapshot,
            &foreground,
            parse.diagnostics.len(),
            LspPosition {
                line: 1,
                character: 67,
            },
        );
        assert!(signature.help.is_some());
        assert_eq!(
            crate::lexer::test_lex_call_count(),
            lex_calls_before_requests
        );
    }
}
