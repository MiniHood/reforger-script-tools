use crate::index::{GlobalSymbolId, IndexedFile, IndexedSymbol, SymbolIndex};
use crate::lexer::{lex, Keyword, Operator, TextSpan, Token, TokenKind};
use crate::model::{SourceCategory, SourceKind, SymbolKind};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct ReferenceResolver<'source, 'index> {
    source: &'source str,
    file_index: &'index SymbolIndex,
    external_index: Option<&'index SymbolIndex>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceResolution {
    pub token_text: String,
    pub token_span: TextSpan,
    pub identifier_context: IdentifierContext,
    pub candidates: Vec<ReferenceCandidate>,
    pub selected: Option<ReferenceCandidate>,
    pub reason: ResolutionReason,
    pub receiver: Option<ReceiverResolution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceCandidate {
    pub source: CandidateSource,
    pub id: GlobalSymbolId,
    pub reason: ResolutionReason,
    pub kind: SymbolKind,
    pub name: Option<String>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub source_kind: SourceKind,
    pub source_category: SourceCategory,
    pub source_priority: u16,
    pub relative_path: Option<PathBuf>,
    pub absolute_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiverResolution {
    pub receiver_text: String,
    pub receiver_span: TextSpan,
    pub owner_type: Option<String>,
    pub is_static: bool,
    pub lookup_path: Vec<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSource {
    FileLocal,
    External,
}

impl CandidateSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileLocal => "file-local",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionReason {
    DeclarationHit,
    LocalInCallable,
    ParameterInCallable,
    ClassMember,
    ReceiverMember,
    StaticMember,
    ReceiverUnresolved,
    TopLevel,
    ExternalPreferred,
    Unresolved,
}

impl ResolutionReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeclarationHit => "declaration-hit",
            Self::LocalInCallable => "local-in-callable",
            Self::ParameterInCallable => "parameter-in-callable",
            Self::ClassMember => "class-member",
            Self::ReceiverMember => "receiver-member",
            Self::StaticMember => "static-member",
            Self::ReceiverUnresolved => "receiver-unresolved",
            Self::TopLevel => "top-level",
            Self::ExternalPreferred => "external-preferred",
            Self::Unresolved => "unresolved",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierContext {
    DeclarationName,
    MemberAccess,
    TypePosition,
    ValueOrCallable,
}

impl IdentifierContext {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeclarationName => "declaration-name",
            Self::MemberAccess => "member-access",
            Self::TypePosition => "type-position",
            Self::ValueOrCallable => "value-or-callable",
        }
    }
}

impl<'source, 'index> ReferenceResolver<'source, 'index> {
    pub const fn new(
        source: &'source str,
        file_index: &'index SymbolIndex,
        external_index: Option<&'index SymbolIndex>,
    ) -> Self {
        Self {
            source,
            file_index,
            external_index,
        }
    }

    pub fn resolve_at_offset(&self, offset: usize) -> Option<ReferenceResolution> {
        let token = token_at_offset(self.source, offset)?;
        if token.kind != TokenKind::Identifier {
            return None;
        }

        let token_text = self.source[token.span.start..token.span.end].to_string();
        let member_access = self.member_access_context(token.span);
        let identifier_context = if member_access.is_some() {
            IdentifierContext::MemberAccess
        } else {
            self.identifier_context(token.span)
        };
        let mut candidates = Vec::new();
        let mut seen = BTreeSet::new();

        self.push_declaration_hits(&token_text, token.span, &mut candidates, &mut seen);

        let receiver = if let Some(member_access) = member_access {
            Some(self.push_receiver_member_candidates(
                &member_access,
                &token_text,
                offset,
                &mut candidates,
                &mut seen,
            ))
        } else if identifier_context == IdentifierContext::TypePosition {
            self.push_type_like_top_level(&token_text, &mut candidates, &mut seen);
            self.push_external_type_like(&token_text, &mut candidates, &mut seen);
            self.push_callable_locals_and_parameters(
                &token_text,
                offset,
                &mut candidates,
                &mut seen,
            );
            self.push_class_members(&token_text, offset, &mut candidates, &mut seen);
            self.push_top_level(&token_text, &mut candidates, &mut seen);
            None
        } else {
            self.push_callable_locals_and_parameters(
                &token_text,
                offset,
                &mut candidates,
                &mut seen,
            );
            self.push_class_members(&token_text, offset, &mut candidates, &mut seen);
            self.push_top_level(&token_text, &mut candidates, &mut seen);
            self.push_external(&token_text, &mut candidates, &mut seen);
            None
        };

        let selected = candidates.first().cloned();
        let reason = selected
            .as_ref()
            .map(|candidate| candidate.reason)
            .or_else(|| {
                receiver
                    .as_ref()
                    .map(|_| ResolutionReason::ReceiverUnresolved)
            })
            .unwrap_or(ResolutionReason::Unresolved);

        Some(ReferenceResolution {
            token_text,
            token_span: token.span,
            identifier_context,
            candidates,
            selected,
            reason,
            receiver,
        })
    }

    fn identifier_context(&self, token_span: TextSpan) -> IdentifierContext {
        if self
            .file_index
            .symbols()
            .iter()
            .any(|symbol| symbol.selection_span == token_span)
        {
            return IdentifierContext::DeclarationName;
        }

        if self.file_index.symbols().iter().any(|symbol| {
            [
                symbol.detail.type_text_span,
                symbol.detail.return_type_text_span,
                symbol.detail.base_type_span,
            ]
            .into_iter()
            .flatten()
            .any(|span| span_contains_span(span, token_span))
        }) {
            return IdentifierContext::TypePosition;
        }

        IdentifierContext::ValueOrCallable
    }

    fn push_declaration_hits(
        &self,
        token_text: &str,
        token_span: TextSpan,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        for id in self.file_index.symbols_for_name(token_text) {
            let Some(symbol) = self.file_index.symbol(*id) else {
                continue;
            };
            if symbol.selection_span == token_span {
                self.push_candidate(
                    candidates,
                    seen,
                    CandidateSource::FileLocal,
                    *id,
                    ResolutionReason::DeclarationHit,
                );
            }
        }
    }

    fn push_callable_locals_and_parameters(
        &self,
        token_text: &str,
        offset: usize,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        let Some(callable) = self.containing_callable(offset) else {
            return;
        };

        for kind in [SymbolKind::LocalVariable, SymbolKind::Parameter] {
            for child in self.file_index.children(callable) {
                let Some(symbol) = self.file_index.symbol(*child) else {
                    continue;
                };
                if symbol.kind != kind || symbol.name.as_deref() != Some(token_text) {
                    continue;
                }
                if kind == SymbolKind::LocalVariable && symbol.selection_span.start > offset {
                    continue;
                }
                let reason = match kind {
                    SymbolKind::LocalVariable => ResolutionReason::LocalInCallable,
                    SymbolKind::Parameter => ResolutionReason::ParameterInCallable,
                    _ => unreachable!(),
                };
                self.push_candidate(candidates, seen, CandidateSource::FileLocal, *child, reason);
            }
        }
    }

    fn push_class_members(
        &self,
        token_text: &str,
        offset: usize,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        let Some(class) = self.containing_class(offset) else {
            return;
        };
        let Some(class_name) = self
            .file_index
            .symbol(class)
            .and_then(|symbol| symbol.name.as_deref())
        else {
            return;
        };

        for member in self.file_index.members_by_owner(class_name) {
            let Some(symbol) = self.file_index.symbol(*member) else {
                continue;
            };
            if symbol.name.as_deref() == Some(token_text) {
                self.push_candidate(
                    candidates,
                    seen,
                    CandidateSource::FileLocal,
                    *member,
                    ResolutionReason::ClassMember,
                );
            }
        }
    }

    fn push_top_level(
        &self,
        token_text: &str,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        for id in self.file_index.top_level_symbols_for_name(token_text) {
            self.push_candidate(
                candidates,
                seen,
                CandidateSource::FileLocal,
                *id,
                ResolutionReason::TopLevel,
            );
        }
    }

    fn push_type_like_top_level(
        &self,
        token_text: &str,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        for id in self.file_index.top_level_symbols_for_name(token_text) {
            let Some(symbol) = self.file_index.symbol(*id) else {
                continue;
            };
            if is_type_like_kind(symbol.kind) {
                self.push_candidate(
                    candidates,
                    seen,
                    CandidateSource::FileLocal,
                    *id,
                    ResolutionReason::TopLevel,
                );
            }
        }
    }

    fn push_external(
        &self,
        token_text: &str,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        let Some(external_index) = self.external_index else {
            return;
        };

        let mut external = Vec::new();
        for id in external_index.preferred_classes_by_name(token_text) {
            push_unique_id(&mut external, id);
        }
        for id in external_index.preferred_typedefs_by_name(token_text) {
            push_unique_id(&mut external, id);
        }
        for id in external_index.preferred_functions_by_name(token_text) {
            push_unique_id(&mut external, id);
        }
        for id in external_index.preferred_top_level_symbols_for_name(token_text) {
            push_unique_id(&mut external, id);
        }

        for id in external {
            push_index_candidate(
                external_index,
                candidates,
                seen,
                CandidateSource::External,
                id,
                ResolutionReason::ExternalPreferred,
            );
        }
    }

    fn push_external_type_like(
        &self,
        token_text: &str,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        let Some(external_index) = self.external_index else {
            return;
        };

        let external = external_index
            .top_level_symbols_for_name(token_text)
            .iter()
            .copied()
            .filter(|id| {
                external_index
                    .symbol(*id)
                    .is_some_and(|symbol| is_type_like_kind(symbol.kind))
            })
            .collect::<Vec<_>>();

        for id in external_index.preferred_from_symbols(&external) {
            push_index_candidate(
                external_index,
                candidates,
                seen,
                CandidateSource::External,
                id,
                ResolutionReason::ExternalPreferred,
            );
        }
    }

    fn push_receiver_member_candidates(
        &self,
        member_access: &MemberAccessContext,
        member_name: &str,
        offset: usize,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) -> ReceiverResolution {
        let mut lookup_path = Vec::new();
        let inferred = self.infer_receiver_expression_type(
            &member_access.receiver_text,
            member_access.receiver_span,
            offset,
            &mut lookup_path,
        );
        let Some(inferred) = inferred else {
            return ReceiverResolution {
                receiver_text: member_access.receiver_text.clone(),
                receiver_span: member_access.receiver_span,
                owner_type: None,
                is_static: false,
                lookup_path,
                failure_reason: Some("receiver type was not inferred".to_string()),
            };
        };

        let reason = if inferred.is_static {
            ResolutionReason::StaticMember
        } else {
            ResolutionReason::ReceiverMember
        };
        let before = candidates.len();
        self.push_members_for_owner(
            self.file_index,
            CandidateSource::FileLocal,
            &inferred.owner_type,
            member_name,
            inferred.is_static,
            reason,
            candidates,
            seen,
        );
        if let Some(external_index) = self.external_index {
            self.push_members_for_owner(
                external_index,
                CandidateSource::External,
                &inferred.owner_type,
                member_name,
                inferred.is_static,
                reason,
                candidates,
                seen,
            );
        }

        if candidates.len() == before && inferred.is_static {
            self.push_static_fallback_candidates(member_name, reason, candidates, seen);
        }

        ReceiverResolution {
            receiver_text: member_access.receiver_text.clone(),
            receiver_span: member_access.receiver_span,
            owner_type: Some(inferred.owner_type),
            is_static: inferred.is_static,
            lookup_path,
            failure_reason: (candidates.len() == before)
                .then(|| format!("no member named `{member_name}` was found for inferred owner")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_members_for_owner(
        &self,
        index: &SymbolIndex,
        source: CandidateSource,
        owner: &str,
        member_name: &str,
        static_only: bool,
        reason: ResolutionReason,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        let lookup = index.completion_members_for_preferred_class(owner);
        let mut matching = lookup
            .members
            .iter()
            .copied()
            .filter(|id| {
                index.symbol(*id).is_some_and(|symbol| {
                    is_member_lookup_kind(symbol.kind)
                        && symbol.name.as_deref() == Some(member_name)
                })
            })
            .collect::<Vec<_>>();

        if static_only {
            let static_matching = matching
                .iter()
                .copied()
                .filter(|id| {
                    index
                        .symbol(*id)
                        .is_some_and(|symbol| has_modifier(symbol, "static"))
                })
                .collect::<Vec<_>>();
            if !static_matching.is_empty() {
                matching = static_matching;
            }
        }

        for id in matching {
            push_index_candidate(index, candidates, seen, source, id, reason);
        }

        self.push_enum_members_for_owner(
            index,
            source,
            owner,
            member_name,
            reason,
            candidates,
            seen,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_enum_members_for_owner(
        &self,
        index: &SymbolIndex,
        source: CandidateSource,
        owner: &str,
        member_name: &str,
        reason: ResolutionReason,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        for enum_id in index.top_level_symbols_for_name(owner) {
            let Some(enum_symbol) = index.symbol(*enum_id) else {
                continue;
            };
            if enum_symbol.kind != SymbolKind::Enum {
                continue;
            }
            for child in index.children(*enum_id) {
                let Some(member) = index.symbol(*child) else {
                    continue;
                };
                if member.kind == SymbolKind::EnumMember
                    && member.name.as_deref() == Some(member_name)
                {
                    push_index_candidate(index, candidates, seen, source, *child, reason);
                }
            }
        }
    }

    fn push_static_fallback_candidates(
        &self,
        member_name: &str,
        reason: ResolutionReason,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
    ) {
        if member_name != "Cast" {
            return;
        }

        self.push_members_for_owner(
            self.file_index,
            CandidateSource::FileLocal,
            "Class",
            member_name,
            true,
            reason,
            candidates,
            seen,
        );
        if let Some(external_index) = self.external_index {
            self.push_members_for_owner(
                external_index,
                CandidateSource::External,
                "Class",
                member_name,
                true,
                reason,
                candidates,
                seen,
            );
        }
    }

    fn infer_receiver_expression_type(
        &self,
        expression: &str,
        expression_span: TextSpan,
        offset: usize,
        lookup_path: &mut Vec<String>,
    ) -> Option<InferredReceiverType> {
        let expression = trim_expression(expression);
        if expression.is_empty() {
            return None;
        }

        if let Some((callee, _arguments)) = split_call_expression(expression) {
            let callee = trim_expression(callee);
            if let Some((receiver, member)) = split_last_member_access(callee) {
                let receiver_type = self.infer_receiver_expression_type(
                    receiver,
                    expression_span,
                    offset,
                    lookup_path,
                )?;
                if member == "Cast" && receiver_type.is_static {
                    lookup_path.push(format!(
                        "`{callee}` treated as cast returning `{}`",
                        receiver_type.owner_type
                    ));
                    return Some(InferredReceiverType {
                        owner_type: receiver_type.owner_type,
                        is_static: false,
                    });
                }
                lookup_path.push(format!(
                    "`{callee}` call receiver inferred as `{}`",
                    receiver_type.owner_type
                ));
                return self.member_result_type(
                    &receiver_type.owner_type,
                    member,
                    receiver_type.is_static,
                    lookup_path,
                );
            }

            lookup_path.push(format!("call `{callee}`"));
            return self.callable_result_type(callee, offset, lookup_path);
        }

        if let Some((receiver, member)) = split_last_member_access(expression) {
            let receiver_type = self.infer_receiver_expression_type(
                receiver,
                expression_span,
                offset,
                lookup_path,
            )?;
            lookup_path.push(format!(
                "`{expression}` member receiver inferred as `{}`",
                receiver_type.owner_type
            ));
            return self.member_result_type(
                &receiver_type.owner_type,
                member,
                receiver_type.is_static,
                lookup_path,
            );
        }

        if expression == "this" {
            let class_name = self
                .containing_class(offset)
                .and_then(|id| self.file_index.symbol(id))
                .and_then(|symbol| symbol.name.clone());
            if let Some(class_name) = class_name {
                lookup_path.push(format!("`this` inferred as `{class_name}`"));
                return Some(InferredReceiverType {
                    owner_type: class_name,
                    is_static: false,
                });
            }
        }

        if expression == "super" {
            let base_type = self
                .containing_class(offset)
                .and_then(|id| self.file_index.symbol(id))
                .and_then(|symbol| symbol.detail.base_type.as_deref())
                .and_then(owner_type_from_type_text);
            if let Some(base_type) = base_type {
                lookup_path.push(format!("`super` inferred as base `{base_type}`"));
                return Some(InferredReceiverType {
                    owner_type: base_type,
                    is_static: false,
                });
            }
        }

        self.identifier_result_type(expression, offset, lookup_path)
    }

    fn identifier_result_type(
        &self,
        name: &str,
        offset: usize,
        lookup_path: &mut Vec<String>,
    ) -> Option<InferredReceiverType> {
        if let Some(callable) = self.containing_callable(offset) {
            for kind in [SymbolKind::LocalVariable, SymbolKind::Parameter] {
                for child in self.file_index.children(callable) {
                    let Some(symbol) = self.file_index.symbol(*child) else {
                        continue;
                    };
                    if symbol.kind != kind || symbol.name.as_deref() != Some(name) {
                        continue;
                    }
                    if kind == SymbolKind::LocalVariable && symbol.selection_span.start > offset {
                        continue;
                    }
                    if let Some(owner_type) = owner_type_from_symbol(symbol) {
                        lookup_path.push(format!(
                            "`{name}` inferred from `{}` `{}`",
                            symbol_kind_label_for_path(kind),
                            owner_type
                        ));
                        return Some(InferredReceiverType {
                            owner_type,
                            is_static: false,
                        });
                    }
                }
            }
        }

        if let Some(class) = self.containing_class(offset) {
            let class_name = self
                .file_index
                .symbol(class)
                .and_then(|symbol| symbol.name.as_deref());
            if let Some(class_name) = class_name {
                for member in self.file_index.members_by_owner(class_name) {
                    let Some(symbol) = self.file_index.symbol(*member) else {
                        continue;
                    };
                    if symbol.name.as_deref() == Some(name) {
                        if let Some(owner_type) = owner_type_from_symbol(symbol) {
                            lookup_path.push(format!(
                                "`{name}` inferred from class member `{}`",
                                owner_type
                            ));
                            return Some(InferredReceiverType {
                                owner_type,
                                is_static: false,
                            });
                        }
                    }
                }
            }
        }

        for id in self.file_index.top_level_symbols_for_name(name) {
            let Some(symbol) = self.file_index.symbol(*id) else {
                continue;
            };
            if let Some(result) = inferred_type_from_top_level_symbol(symbol) {
                lookup_path.push(format!("`{name}` inferred from file-local top-level"));
                return Some(result);
            }
        }

        let Some(external_index) = self.external_index else {
            return None;
        };
        let mut external = Vec::new();
        for id in external_index.preferred_classes_by_name(name) {
            push_unique_id(&mut external, id);
        }
        for id in external_index.preferred_typedefs_by_name(name) {
            push_unique_id(&mut external, id);
        }
        for id in external_index.preferred_functions_by_name(name) {
            push_unique_id(&mut external, id);
        }
        for id in external_index.preferred_top_level_symbols_for_name(name) {
            push_unique_id(&mut external, id);
        }
        for id in external {
            let Some(symbol) = external_index.symbol(id) else {
                continue;
            };
            if let Some(result) = inferred_type_from_top_level_symbol(symbol) {
                lookup_path.push(format!("`{name}` inferred from external top-level"));
                return Some(result);
            }
        }

        None
    }

    fn callable_result_type(
        &self,
        name: &str,
        offset: usize,
        lookup_path: &mut Vec<String>,
    ) -> Option<InferredReceiverType> {
        if let Some(callable) = self.containing_callable(offset) {
            for child in self.file_index.children(callable) {
                let Some(symbol) = self.file_index.symbol(*child) else {
                    continue;
                };
                if symbol.kind == SymbolKind::LocalVariable && symbol.name.as_deref() == Some(name)
                {
                    if let Some(owner_type) = owner_type_from_symbol(symbol) {
                        lookup_path
                            .push(format!("call `{name}` matched local callable-like value"));
                        return Some(InferredReceiverType {
                            owner_type,
                            is_static: false,
                        });
                    }
                }
            }
        }

        if let Some(class) = self.containing_class(offset) {
            let class_name = self
                .file_index
                .symbol(class)
                .and_then(|symbol| symbol.name.as_deref());
            if let Some(class_name) = class_name {
                if let Some(result) = self.member_result_type(class_name, name, false, lookup_path)
                {
                    lookup_path.push(format!("call `{name}` matched containing class member"));
                    return Some(result);
                }
            }
        }

        for id in self.file_index.functions_by_name(name) {
            let Some(symbol) = self.file_index.symbol(*id) else {
                continue;
            };
            if let Some(owner_type) = symbol
                .detail
                .return_type_text
                .as_deref()
                .and_then(owner_type_from_type_text)
            {
                lookup_path.push(format!("call `{name}` matched file-local function"));
                return Some(InferredReceiverType {
                    owner_type,
                    is_static: false,
                });
            }
        }

        if let Some(external_index) = self.external_index {
            for id in external_index.preferred_functions_by_name(name) {
                let Some(symbol) = external_index.symbol(id) else {
                    continue;
                };
                if let Some(owner_type) = symbol
                    .detail
                    .return_type_text
                    .as_deref()
                    .and_then(owner_type_from_type_text)
                {
                    lookup_path.push(format!("call `{name}` matched external function"));
                    return Some(InferredReceiverType {
                        owner_type,
                        is_static: false,
                    });
                }
            }
        }

        None
    }

    fn member_result_type(
        &self,
        owner: &str,
        member: &str,
        static_only: bool,
        lookup_path: &mut Vec<String>,
    ) -> Option<InferredReceiverType> {
        if let Some(result) =
            member_result_type_from_index(self.file_index, owner, member, static_only)
        {
            lookup_path.push(format!("member `{owner}.{member}` matched file-local"));
            return Some(result);
        }
        if let Some(external_index) = self.external_index {
            if let Some(result) =
                member_result_type_from_index(external_index, owner, member, static_only)
            {
                lookup_path.push(format!("member `{owner}.{member}` matched external"));
                return Some(result);
            }
        }
        None
    }

    fn member_access_context(&self, token_span: TextSpan) -> Option<MemberAccessContext> {
        let tokens = syntax_tokens(self.source);
        let token_index = tokens.iter().position(|token| token.span == token_span)?;
        if token_index < 2 || tokens[token_index - 1].kind != TokenKind::Dot {
            return None;
        }

        let receiver_span = receiver_span_before_dot(&tokens, token_index - 1)?;
        let receiver_text = self.source[receiver_span.start..receiver_span.end]
            .trim()
            .to_string();
        (!receiver_text.is_empty()).then_some(MemberAccessContext {
            receiver_text,
            receiver_span,
        })
    }

    fn push_candidate(
        &self,
        candidates: &mut Vec<ReferenceCandidate>,
        seen: &mut BTreeSet<CandidateKey>,
        source: CandidateSource,
        id: GlobalSymbolId,
        reason: ResolutionReason,
    ) {
        push_index_candidate(self.file_index, candidates, seen, source, id, reason);
    }

    fn containing_callable(&self, offset: usize) -> Option<GlobalSymbolId> {
        self.file_index
            .symbols()
            .iter()
            .filter(|symbol| is_callable_kind(symbol.kind) && span_contains(symbol.span, offset))
            .min_by_key(|symbol| symbol.span.len())
            .map(|symbol| symbol.id)
    }

    fn containing_class(&self, offset: usize) -> Option<GlobalSymbolId> {
        self.file_index
            .symbols()
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Class && span_contains(symbol.span, offset))
            .min_by_key(|symbol| symbol.span.len())
            .map(|symbol| symbol.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemberAccessContext {
    receiver_text: String,
    receiver_span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InferredReceiverType {
    owner_type: String,
    is_static: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CandidateKey {
    source: CandidateSource,
    id: GlobalSymbolId,
}

impl PartialOrd for CandidateSource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CandidateSource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

fn push_index_candidate(
    index: &SymbolIndex,
    candidates: &mut Vec<ReferenceCandidate>,
    seen: &mut BTreeSet<CandidateKey>,
    source: CandidateSource,
    id: GlobalSymbolId,
    reason: ResolutionReason,
) {
    let key = CandidateKey { source, id };
    if !seen.insert(key) {
        return;
    }
    let Some(symbol) = index.symbol(id) else {
        return;
    };
    let Some(file) = index.file(id.file_id) else {
        return;
    };
    candidates.push(candidate_from_symbol(source, id, reason, symbol, file));
}

fn candidate_from_symbol(
    source: CandidateSource,
    id: GlobalSymbolId,
    reason: ResolutionReason,
    symbol: &IndexedSymbol,
    file: &IndexedFile,
) -> ReferenceCandidate {
    ReferenceCandidate {
        source,
        id,
        reason,
        kind: symbol.kind,
        name: symbol.name.clone(),
        span: symbol.span,
        selection_span: symbol.selection_span,
        source_kind: file.metadata.kind,
        source_category: file.metadata.category,
        source_priority: file.metadata.priority,
        relative_path: file.metadata.relative_path.clone(),
        absolute_path: file.metadata.absolute_path.clone(),
    }
}

fn token_at_offset(source: &str, offset: usize) -> Option<crate::lexer::Token> {
    lex(source)
        .into_iter()
        .find(|token| token.span.start <= offset && offset < token.span.end)
}

fn syntax_tokens(source: &str) -> Vec<Token> {
    lex(source)
        .into_iter()
        .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof)
        .collect()
}

fn receiver_span_before_dot(tokens: &[Token], dot_index: usize) -> Option<TextSpan> {
    if dot_index == 0 {
        return None;
    }

    let mut start = tokens[dot_index - 1].span.start;
    let end = tokens[dot_index].span.start;
    let mut index = dot_index;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    while index > 0 {
        let token = tokens[index - 1];
        if paren_depth == 0
            && bracket_depth == 0
            && index < dot_index
            && tokens[index].span.start > token.span.end
        {
            break;
        }

        match token.kind {
            TokenKind::RightParen => paren_depth += 1,
            TokenKind::LeftParen => {
                if paren_depth == 0 {
                    break;
                }
                paren_depth -= 1;
            }
            TokenKind::RightBracket => bracket_depth += 1,
            TokenKind::LeftBracket => {
                if bracket_depth == 0 {
                    break;
                }
                bracket_depth -= 1;
            }
            _ => {}
        }

        if paren_depth == 0 && bracket_depth == 0 && is_receiver_boundary(token.kind) {
            break;
        }

        start = token.span.start;
        index -= 1;
    }

    (start < end).then_some(TextSpan::new(start, end))
}

fn is_receiver_boundary(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Semicolon
            | TokenKind::Comma
            | TokenKind::LeftBrace
            | TokenKind::RightBrace
            | TokenKind::Colon
            | TokenKind::Question
            | TokenKind::Hash
            | TokenKind::Keyword(Keyword::If)
            | TokenKind::Keyword(Keyword::Else)
            | TokenKind::Keyword(Keyword::For)
            | TokenKind::Keyword(Keyword::Foreach)
            | TokenKind::Keyword(Keyword::While)
            | TokenKind::Keyword(Keyword::Do)
            | TokenKind::Keyword(Keyword::Switch)
            | TokenKind::Keyword(Keyword::Case)
            | TokenKind::Keyword(Keyword::Return)
            | TokenKind::Keyword(Keyword::New)
            | TokenKind::Operator(Operator::Equal)
            | TokenKind::Operator(Operator::Bang)
            | TokenKind::Operator(Operator::Plus)
            | TokenKind::Operator(Operator::Minus)
            | TokenKind::Operator(Operator::Star)
            | TokenKind::Operator(Operator::Slash)
            | TokenKind::Operator(Operator::Percent)
            | TokenKind::Operator(Operator::EqualEqual)
            | TokenKind::Operator(Operator::BangEqual)
            | TokenKind::Operator(Operator::Less)
            | TokenKind::Operator(Operator::LessEqual)
            | TokenKind::Operator(Operator::LessLess)
            | TokenKind::Operator(Operator::LessLessEqual)
            | TokenKind::Operator(Operator::Greater)
            | TokenKind::Operator(Operator::GreaterEqual)
            | TokenKind::Operator(Operator::GreaterGreater)
            | TokenKind::Operator(Operator::GreaterGreaterEqual)
            | TokenKind::Operator(Operator::Ampersand)
            | TokenKind::Operator(Operator::AmpersandAmpersand)
            | TokenKind::Operator(Operator::Pipe)
            | TokenKind::Operator(Operator::PipePipe)
            | TokenKind::Operator(Operator::Caret)
            | TokenKind::Operator(Operator::PlusEqual)
            | TokenKind::Operator(Operator::MinusEqual)
            | TokenKind::Operator(Operator::StarEqual)
            | TokenKind::Operator(Operator::SlashEqual)
            | TokenKind::Operator(Operator::PercentEqual)
            | TokenKind::Operator(Operator::PipeEqual)
            | TokenKind::Operator(Operator::AmpersandEqual)
            | TokenKind::Operator(Operator::CaretEqual)
    )
}

fn trim_expression(expression: &str) -> &str {
    expression.trim()
}

fn split_call_expression(expression: &str) -> Option<(&str, &str)> {
    let expression = expression.trim();
    if !expression.ends_with(')') {
        return None;
    }

    let mut depth = 0usize;
    for (index, ch) in expression.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let callee = expression[..index].trim();
                    let args = expression[index + 1..expression.len() - 1].trim();
                    return (!callee.is_empty()).then_some((callee, args));
                }
            }
            _ => {}
        }
    }

    None
}

fn split_last_member_access(expression: &str) -> Option<(&str, &str)> {
    let expression = expression.trim();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut angle_depth = 0usize;

    for (index, ch) in expression.char_indices().rev() {
        match ch {
            ')' => paren_depth += 1,
            '(' => paren_depth = paren_depth.saturating_sub(1),
            ']' => bracket_depth += 1,
            '[' => bracket_depth = bracket_depth.saturating_sub(1),
            '>' => angle_depth += 1,
            '<' => angle_depth = angle_depth.saturating_sub(1),
            '.' if paren_depth == 0 && bracket_depth == 0 && angle_depth == 0 => {
                let receiver = expression[..index].trim();
                let member = expression[index + 1..].trim();
                if !receiver.is_empty() && is_identifier_text(member) {
                    return Some((receiver, member));
                }
            }
            _ => {}
        }
    }

    None
}

fn is_identifier_text(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn owner_type_from_symbol(symbol: &IndexedSymbol) -> Option<String> {
    match symbol.kind {
        SymbolKind::LocalVariable
        | SymbolKind::Parameter
        | SymbolKind::Field
        | SymbolKind::GlobalField
        | SymbolKind::Typedef => symbol
            .detail
            .type_text
            .as_deref()
            .and_then(owner_type_from_type_text),
        SymbolKind::Function | SymbolKind::Method => symbol
            .detail
            .return_type_text
            .as_deref()
            .and_then(owner_type_from_type_text),
        SymbolKind::Constructor | SymbolKind::Class | SymbolKind::Enum => symbol.name.clone(),
        _ => None,
    }
}

fn inferred_type_from_top_level_symbol(symbol: &IndexedSymbol) -> Option<InferredReceiverType> {
    match symbol.kind {
        SymbolKind::Class | SymbolKind::Enum | SymbolKind::Typedef => {
            let owner_type = symbol
                .name
                .clone()
                .or_else(|| owner_type_from_symbol(symbol))?;
            Some(InferredReceiverType {
                owner_type,
                is_static: true,
            })
        }
        SymbolKind::Function | SymbolKind::GlobalField => {
            owner_type_from_symbol(symbol).map(|owner_type| InferredReceiverType {
                owner_type,
                is_static: false,
            })
        }
        _ => None,
    }
}

fn member_result_type_from_index(
    index: &SymbolIndex,
    owner: &str,
    member: &str,
    static_only: bool,
) -> Option<InferredReceiverType> {
    let lookup = index.completion_members_for_preferred_class(owner);
    let mut matching = lookup
        .members
        .iter()
        .copied()
        .filter(|id| {
            index.symbol(*id).is_some_and(|symbol| {
                is_member_lookup_kind(symbol.kind) && symbol.name.as_deref() == Some(member)
            })
        })
        .collect::<Vec<_>>();

    if static_only {
        let static_matching = matching
            .iter()
            .copied()
            .filter(|id| {
                index
                    .symbol(*id)
                    .is_some_and(|symbol| has_modifier(symbol, "static"))
            })
            .collect::<Vec<_>>();
        if !static_matching.is_empty() {
            matching = static_matching;
        }
    }

    for id in index.preferred_from_symbols(&matching) {
        let Some(symbol) = index.symbol(id) else {
            continue;
        };
        if let Some(owner_type) = owner_type_from_symbol(symbol) {
            return Some(InferredReceiverType {
                owner_type,
                is_static: false,
            });
        }
    }

    None
}

fn owner_type_from_type_text(type_text: &str) -> Option<String> {
    let mut text = type_text.trim();
    loop {
        let stripped = strip_type_prefix(text).trim_start();
        if stripped == text {
            break;
        }
        text = stripped;
    }

    if text.is_empty() {
        return None;
    }

    for collection in ["array", "set", "map"] {
        if text.starts_with(collection) && text[collection.len()..].trim_start().starts_with('<') {
            return Some(collection.to_string());
        }
    }

    let owner = text
        .split(|ch: char| ch == '<' || ch == '[' || ch.is_whitespace() || ch == '&' || ch == '*')
        .next()
        .unwrap_or_default()
        .trim();

    (!owner.is_empty()).then(|| owner.to_string())
}

fn strip_type_prefix(text: &str) -> &str {
    for prefix in [
        "ref", "notnull", "autoptr", "owned", "const", "out", "inout",
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            if rest
                .chars()
                .next()
                .is_some_and(|ch| ch.is_whitespace() || ch == '<')
            {
                return rest;
            }
        }
    }
    text
}

fn is_member_lookup_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Field | SymbolKind::Method | SymbolKind::Constructor | SymbolKind::Destructor
    )
}

fn has_modifier(symbol: &IndexedSymbol, modifier: &str) -> bool {
    symbol.modifiers.iter().any(|value| value == modifier)
}

fn symbol_kind_label_for_path(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::LocalVariable => "local",
        SymbolKind::Parameter => "parameter",
        SymbolKind::Field => "field",
        SymbolKind::GlobalField => "global",
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Class => "class",
        SymbolKind::Enum => "enum",
        SymbolKind::Typedef => "typedef",
        _ => "symbol",
    }
}

fn span_contains(span: TextSpan, offset: usize) -> bool {
    span.start <= offset && offset < span.end
}

fn span_contains_span(outer: TextSpan, inner: TextSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn is_callable_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Destructor
    )
}

fn is_type_like_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class | SymbolKind::Enum | SymbolKind::Typedef
    )
}

fn push_unique_id(ids: &mut Vec<GlobalSymbolId>, id: GlobalSymbolId) {
    if !ids.contains(&id) {
        ids.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::model::{
        source_category_for_path, SourceFileMetadata, SymbolCatalog, SOURCE_PRIORITY_GAME_DATA,
        SOURCE_PRIORITY_WORKSPACE,
    };
    use crate::parser::parse_source;

    #[test]
    fn declaration_identifier_resolves_to_itself() {
        let source = "class Example {}";
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve(&index, source, "Example");

        assert_eq!(resolution.token_text, "Example");
        assert_eq!(resolution.reason, ResolutionReason::DeclarationHit);
        assert_eq!(resolution.selected.unwrap().kind, SymbolKind::Class);
    }

    #[test]
    fn local_variable_use_resolves_before_parameter_member_and_top_level() {
        let source = r#"int value;
class Example
{
	int value;
	void Run(int value)
	{
		int value = 4;
		value = value + 1;
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "value = value + 1", "value");

        assert_eq!(resolution.reason, ResolutionReason::LocalInCallable);
        let candidates = candidate_kinds_reasons(&resolution);
        assert_eq!(
            candidates[..4],
            [
                (SymbolKind::LocalVariable, ResolutionReason::LocalInCallable),
                (SymbolKind::Parameter, ResolutionReason::ParameterInCallable),
                (SymbolKind::Field, ResolutionReason::ClassMember),
                (SymbolKind::GlobalField, ResolutionReason::TopLevel),
            ]
        );
    }

    #[test]
    fn parameter_use_resolves_before_member_and_top_level_when_no_local_exists() {
        let source = r#"int value;
class Example
{
	int value;
	void Run(int value)
	{
		Print(value);
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "Print(value)", "value");

        assert_eq!(resolution.reason, ResolutionReason::ParameterInCallable);
        assert_eq!(resolution.selected.unwrap().kind, SymbolKind::Parameter);
    }

    #[test]
    fn class_member_use_resolves_from_containing_class() {
        let source = r#"class Example
{
	int m_Value;
	void Run()
	{
		m_Value = 4;
		Run();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let field = resolve_at_needle(&index, source, "m_Value = 4", "m_Value");
        let method = resolve_at_needle(&index, source, "Run();", "Run");

        assert_eq!(field.reason, ResolutionReason::ClassMember);
        assert_eq!(field.selected.unwrap().kind, SymbolKind::Field);
        assert_eq!(method.reason, ResolutionReason::ClassMember);
        assert_eq!(method.selected.unwrap().kind, SymbolKind::Method);
    }

    #[test]
    fn file_local_top_level_declarations_resolve_by_name() {
        let source = r#"Game g_Game;
typedef string FactionKey;
enum EExample { One = 1 }
void GlobalFn();
class Example
{
	void Run()
	{
		Example local;
		FactionKey key;
		EExample flag;
		GlobalFn();
		g_Game = null;
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        assert_eq!(
            resolve_at_needle(&index, source, "Example local", "Example").reason,
            ResolutionReason::TopLevel
        );
        assert_eq!(
            resolve_at_needle(&index, source, "FactionKey key", "FactionKey")
                .selected
                .unwrap()
                .kind,
            SymbolKind::Typedef
        );
        assert_eq!(
            resolve_at_needle(&index, source, "EExample flag", "EExample")
                .selected
                .unwrap()
                .kind,
            SymbolKind::Enum
        );
        assert_eq!(
            resolve_at_needle(&index, source, "GlobalFn();", "GlobalFn")
                .selected
                .unwrap()
                .kind,
            SymbolKind::Function
        );
        assert_eq!(
            resolve_at_needle(&index, source, "g_Game = null", "g_Game")
                .selected
                .unwrap()
                .kind,
            SymbolKind::GlobalField
        );
    }

    #[test]
    fn type_position_prefers_type_declarations_over_constructor_members() {
        let source = r#"class Example
{
	void Example();
	static Example Make()
	{
		Example value;
		array<Example> values;
		Example constructed = new Example();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let return_type = resolve_at_needle(&index, source, "static Example Make", "Example");
        let local_type = resolve_at_needle(&index, source, "Example value", "Example");
        let generic_arg = resolve_at_needle(&index, source, "array<Example> values", "Example");
        let constructor_call = resolve_at_needle(&index, source, "new Example()", "Example");

        assert_eq!(
            return_type.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(return_type.reason, ResolutionReason::TopLevel);
        assert_eq!(return_type.selected.unwrap().kind, SymbolKind::Class);

        assert_eq!(
            local_type.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(local_type.selected.unwrap().kind, SymbolKind::Class);

        assert_eq!(
            generic_arg.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(generic_arg.selected.unwrap().kind, SymbolKind::Class);

        assert_eq!(
            constructor_call.identifier_context,
            IdentifierContext::ValueOrCallable
        );
        assert_eq!(constructor_call.reason, ResolutionReason::ClassMember);
        assert_eq!(
            constructor_call.selected.unwrap().kind,
            SymbolKind::Constructor
        );
    }

    #[test]
    fn base_type_position_prefers_file_local_class() {
        let source = r#"class Base {}
class Child : Base {}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "class Child : Base", "Base");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(resolution.reason, ResolutionReason::TopLevel);
        assert_eq!(resolution.selected.unwrap().kind, SymbolKind::Class);
    }

    #[test]
    fn external_index_resolves_missing_type_name() {
        let source = r#"class Example
{
	void Run()
	{
		ExternalType value;
	}
}
"#;
        let file_index = index_for_source(source, workspace_metadata("Example.c"));
        let external_source = "class ExternalType {}";
        let external_index =
            index_for_source(external_source, game_metadata("Game/ExternalType.c"));

        let offset = offset_for_needle(source, "ExternalType value", "ExternalType");
        let resolution = ReferenceResolver::new(source, &file_index, Some(&external_index))
            .resolve_at_offset(offset)
            .unwrap();

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::TypePosition
        );
        assert_eq!(resolution.reason, ResolutionReason::ExternalPreferred);
        let selected = resolution.selected.unwrap();
        assert_eq!(selected.source, CandidateSource::External);
        assert_eq!(selected.kind, SymbolKind::Class);
        assert_eq!(selected.source_kind, SourceKind::GameData);
    }

    #[test]
    fn local_variable_receiver_resolves_external_member() {
        let source = r#"class Example
{
	void Run()
	{
		IEntity ent;
		ent.GetOrigin();
	}
}
"#;
        let file_index = index_for_source(source, workspace_metadata("Example.c"));
        let external_index = index_for_source(
            "class IEntity { vector GetOrigin(); }",
            game_metadata("Game/IEntity.c"),
        );

        let resolution = resolve_at_needle_with_external(
            &file_index,
            &external_index,
            source,
            "ent.GetOrigin",
            "GetOrigin",
        );

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::ReceiverMember);
        assert_eq!(
            resolution.selected.as_ref().unwrap().source,
            CandidateSource::External
        );
        assert_eq!(
            resolution.selected.as_ref().unwrap().kind,
            SymbolKind::Method
        );
        assert_eq!(
            resolution.receiver.as_ref().unwrap().owner_type.as_deref(),
            Some("IEntity")
        );
    }

    #[test]
    fn parameter_receiver_resolves_file_local_member() {
        let source = r#"class Component
{
	int GetResourceValue();
}

class Example
{
	void Run(Component comp)
	{
		comp.GetResourceValue();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution =
            resolve_at_needle(&index, source, "comp.GetResourceValue", "GetResourceValue");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::ReceiverMember);
        assert_eq!(
            resolution.selected.as_ref().unwrap().source,
            CandidateSource::FileLocal
        );
        assert_eq!(
            resolution.selected.as_ref().unwrap().kind,
            SymbolKind::Method
        );
    }

    #[test]
    fn field_receiver_strips_collection_type_to_owner() {
        let source = r#"class Example
{
	ref array<int> m_Values;
	void Run()
	{
		m_Values.Insert(4);
	}
}
"#;
        let file_index = index_for_source(source, workspace_metadata("Example.c"));
        let external_index = index_for_source(
            "class array { void Insert(int value); }",
            game_metadata("Core/proto/Types.c"),
        );

        let resolution = resolve_at_needle_with_external(
            &file_index,
            &external_index,
            source,
            "m_Values.Insert",
            "Insert",
        );

        assert_eq!(resolution.reason, ResolutionReason::ReceiverMember);
        assert_eq!(
            resolution.receiver.as_ref().unwrap().owner_type.as_deref(),
            Some("array")
        );
        assert_eq!(
            resolution.selected.as_ref().unwrap().source,
            CandidateSource::External
        );
    }

    #[test]
    fn this_receiver_resolves_containing_class_members() {
        let source = r#"class Example
{
	void DoThing();
	void Run()
	{
		this.DoThing();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "this.DoThing", "DoThing");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::ReceiverMember);
        assert_eq!(
            resolution.selected.as_ref().unwrap().kind,
            SymbolKind::Method
        );
    }

    #[test]
    fn super_receiver_resolves_base_class_members() {
        let source = r#"class Base
{
	void OnInit();
}

class Example : Base
{
	void Run()
	{
		super.OnInit();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "super.OnInit", "OnInit");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::ReceiverMember);
        assert_eq!(
            resolution.receiver.as_ref().unwrap().owner_type.as_deref(),
            Some("Base")
        );
        assert_eq!(
            resolution.selected.as_ref().unwrap().kind,
            SymbolKind::Method
        );
    }

    #[test]
    fn static_receiver_prefers_static_member_when_available() {
        let source = r#"class IEntity {}
class SCR_BaseGameMode
{
	static SCR_BaseGameMode Cast(IEntity entity);
	void Cast();
}

class Example
{
	void Run(IEntity entity)
	{
		SCR_BaseGameMode.Cast(entity);
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "SCR_BaseGameMode.Cast", "Cast");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::StaticMember);
        assert_eq!(
            resolution.selected.as_ref().unwrap().kind,
            SymbolKind::Method
        );
        assert_eq!(
            resolution.receiver.as_ref().unwrap().owner_type.as_deref(),
            Some("SCR_BaseGameMode")
        );
    }

    #[test]
    fn static_enum_member_resolves_to_enum_member_child() {
        let source = r#"enum RplChannel
{
	Reliable = 0,
	Unreliable = 1
}

class Example
{
	void Run()
	{
		RplChannel.Reliable;
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "RplChannel.Reliable", "Reliable");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::StaticMember);
        let selected = resolution.selected.as_ref().unwrap();
        assert_eq!(selected.kind, SymbolKind::EnumMember);
        assert_eq!(selected.name.as_deref(), Some("Reliable"));
        assert_eq!(
            resolution.receiver.as_ref().unwrap().owner_type.as_deref(),
            Some("RplChannel")
        );
    }

    #[test]
    fn type_cast_falls_back_to_generic_class_cast_member() {
        let source = r#"class IEntity {}
class Class
{
	static Class Cast(Class from);
}
class SCR_Foo {}

class Example
{
	void Run(IEntity entity)
	{
		SCR_Foo.Cast(entity);
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "SCR_Foo.Cast", "Cast");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::StaticMember);
        let selected = resolution.selected.as_ref().unwrap();
        assert_eq!(selected.kind, SymbolKind::Method);
        assert_eq!(selected.name.as_deref(), Some("Cast"));
        assert_eq!(resolution.receiver.as_ref().unwrap().failure_reason, None);
    }

    #[test]
    fn cast_receiver_infers_cast_target_for_next_member() {
        let source = r#"class IEntity {}
class SCR_Foo
{
	static SCR_Foo Cast(IEntity entity);
	void Run();
}

class Example
{
	void Test(IEntity entity)
	{
		SCR_Foo.Cast(entity).Run();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "SCR_Foo.Cast(entity).Run", "Run");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::ReceiverMember);
        assert_eq!(
            resolution.receiver.as_ref().unwrap().owner_type.as_deref(),
            Some("SCR_Foo")
        );
        assert_eq!(
            resolution.selected.as_ref().unwrap().kind,
            SymbolKind::Method
        );
    }

    #[test]
    fn function_call_receiver_uses_return_type() {
        let source = r#"class World {}
class Game
{
	World GetWorld();
}
Game GetGame();

class Example
{
	void Run()
	{
		GetGame().GetWorld();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "GetGame().GetWorld", "GetWorld");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::ReceiverMember);
        assert_eq!(
            resolution.receiver.as_ref().unwrap().owner_type.as_deref(),
            Some("Game")
        );
        assert_eq!(
            resolution.selected.as_ref().unwrap().kind,
            SymbolKind::Method
        );
    }

    #[test]
    fn receiver_span_stops_at_control_statement_condition() {
        let source = r#"class Widget
{
	void RemoveFromHierarchy();
}

class Example
{
	Widget m_wHint;
	void Run()
	{
		if (m_wHint)
			m_wHint.RemoveFromHierarchy();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(
            &index,
            source,
            "m_wHint.RemoveFromHierarchy",
            "RemoveFromHierarchy",
        );

        assert_eq!(resolution.reason, ResolutionReason::ReceiverMember);
        let receiver = resolution.receiver.as_ref().unwrap();
        assert_eq!(receiver.receiver_text, "m_wHint");
        assert_eq!(receiver.owner_type.as_deref(), Some("Widget"));
        assert_eq!(receiver.failure_reason, None);
        assert_eq!(
            resolution.selected.as_ref().unwrap().kind,
            SymbolKind::Method
        );
    }

    #[test]
    fn receiver_span_stops_at_return_keyword_for_static_enum_members() {
        let source = r#"enum ENodeResult
{
	FAIL,
	SUCCESS,
}

class Example
{
	ENodeResult Run()
	{
		return ENodeResult.FAIL;
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "return ENodeResult.FAIL", "FAIL");

        assert_eq!(resolution.reason, ResolutionReason::StaticMember);
        let receiver = resolution.receiver.as_ref().unwrap();
        assert_eq!(receiver.receiver_text, "ENodeResult");
        assert_eq!(receiver.owner_type.as_deref(), Some("ENodeResult"));
        assert_eq!(receiver.failure_reason, None);
        assert_eq!(
            resolution.selected.as_ref().unwrap().kind,
            SymbolKind::EnumMember
        );
    }

    #[test]
    fn receiver_span_stops_at_binary_operators() {
        let source = r#"enum TraceFlags
{
	WORLD,
	ENTS,
}

class Example
{
	void Run()
	{
		int flags = TraceFlags.WORLD | TraceFlags.ENTS;
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let world = resolve_at_needle(&index, source, "TraceFlags.WORLD", "WORLD");
        let ents = resolve_at_needle(&index, source, "TraceFlags.ENTS", "ENTS");

        assert_eq!(world.reason, ResolutionReason::StaticMember);
        assert_eq!(world.receiver.as_ref().unwrap().receiver_text, "TraceFlags");
        assert_eq!(
            world.selected.as_ref().unwrap().kind,
            SymbolKind::EnumMember
        );
        assert_eq!(ents.reason, ResolutionReason::StaticMember);
        assert_eq!(ents.receiver.as_ref().unwrap().receiver_text, "TraceFlags");
        assert_eq!(ents.selected.as_ref().unwrap().kind, SymbolKind::EnumMember);
    }

    #[test]
    fn receiver_span_stops_at_call_argument_boundaries() {
        let source = r#"class Math
{
	static float RandomFloatInclusive(float min, float max);
	static float Max(float first, float second);
}

class Example
{
	void Run(float value)
	{
		float random = Math.RandomFloatInclusive(Math.Max(0, value), value);
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(
            &index,
            source,
            "Math.RandomFloatInclusive",
            "RandomFloatInclusive",
        );

        assert_eq!(resolution.reason, ResolutionReason::StaticMember);
        let receiver = resolution.receiver.as_ref().unwrap();
        assert_eq!(receiver.receiver_text, "Math");
        assert_eq!(receiver.owner_type.as_deref(), Some("Math"));
        assert_eq!(receiver.failure_reason, None);
    }

    #[test]
    fn receiver_span_preserves_valid_call_chains() {
        let source = r#"class World {}
class Game
{
	World GetWorld();
}
Game GetGame();

class Example
{
	void Run()
	{
		GetGame().GetWorld();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "GetGame().GetWorld", "GetWorld");

        assert_eq!(resolution.reason, ResolutionReason::ReceiverMember);
        let receiver = resolution.receiver.as_ref().unwrap();
        assert_eq!(receiver.receiver_text, "GetGame()");
        assert_eq!(receiver.owner_type.as_deref(), Some("Game"));
        assert_eq!(receiver.failure_reason, None);
    }

    #[test]
    fn unresolved_receiver_does_not_fall_back_to_containing_symbol() {
        let source = r#"class Example
{
	void Run()
	{
		missing.GetWorld();
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        let resolution = resolve_at_needle(&index, source, "missing.GetWorld", "GetWorld");

        assert_eq!(
            resolution.identifier_context,
            IdentifierContext::MemberAccess
        );
        assert_eq!(resolution.reason, ResolutionReason::ReceiverUnresolved);
        assert!(resolution.selected.is_none());
        assert!(resolution.candidates.is_empty());
        assert!(resolution
            .receiver
            .as_ref()
            .unwrap()
            .failure_reason
            .is_some());
    }

    #[test]
    fn non_identifier_tokens_do_not_resolve() {
        let source = r#"class Example
{
	void Run()
	{
		// comment value
		string text = "value";
		if (true) {}
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Example.c"));

        assert_no_resolution(&index, source, "class Example", "class");
        assert_no_resolution(&index, source, "// comment value", "value");
        assert_no_resolution(&index, source, "\"value\"", "value");
        assert_no_resolution(&index, source, "(true)", "(");
    }

    #[test]
    fn ambiguous_same_name_candidates_are_preserved_with_selected_first() {
        let source = r#"class Shared {}
typedef string Shared;
void Shared();
class Example
{
	void Run()
	{
		Shared value;
	}
}
"#;
        let index = index_for_source(source, workspace_metadata("Shared.c"));

        let resolution = resolve_at_needle(&index, source, "Shared value", "Shared");

        assert_eq!(resolution.reason, ResolutionReason::TopLevel);
        assert_eq!(resolution.candidates.len(), 3);
        assert_eq!(resolution.selected.unwrap().kind, SymbolKind::Class);
        assert!(resolution
            .candidates
            .iter()
            .any(|candidate| candidate.kind == SymbolKind::Typedef));
        assert!(resolution
            .candidates
            .iter()
            .any(|candidate| candidate.kind == SymbolKind::Function));
    }

    fn resolve(index: &SymbolIndex, source: &str, needle: &str) -> ReferenceResolution {
        let offset = source.find(needle).unwrap();
        ReferenceResolver::new(source, index, None)
            .resolve_at_offset(offset)
            .unwrap()
    }

    fn resolve_at_needle(
        index: &SymbolIndex,
        source: &str,
        needle: &str,
        cursor: &str,
    ) -> ReferenceResolution {
        let offset = offset_for_needle(source, needle, cursor);
        ReferenceResolver::new(source, index, None)
            .resolve_at_offset(offset)
            .unwrap()
    }

    fn resolve_at_needle_with_external(
        index: &SymbolIndex,
        external_index: &SymbolIndex,
        source: &str,
        needle: &str,
        cursor: &str,
    ) -> ReferenceResolution {
        let offset = offset_for_needle(source, needle, cursor);
        ReferenceResolver::new(source, index, Some(external_index))
            .resolve_at_offset(offset)
            .unwrap()
    }

    fn assert_no_resolution(index: &SymbolIndex, source: &str, needle: &str, cursor: &str) {
        let offset = offset_for_needle(source, needle, cursor);
        assert_eq!(
            ReferenceResolver::new(source, index, None).resolve_at_offset(offset),
            None
        );
    }

    fn candidate_kinds_reasons(
        resolution: &ReferenceResolution,
    ) -> Vec<(SymbolKind, ResolutionReason)> {
        resolution
            .candidates
            .iter()
            .map(|candidate| (candidate.kind, candidate.reason))
            .collect()
    }

    fn offset_for_needle(source: &str, needle: &str, cursor: &str) -> usize {
        let start = source.find(needle).unwrap();
        let cursor_start = needle.find(cursor).unwrap();
        start + cursor_start
    }

    fn index_for_source(source: &str, metadata: SourceFileMetadata) -> SymbolIndex {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        let catalog = SymbolCatalog::from_ast_with_metadata(source, &ast, metadata);
        SymbolIndex::from_catalogs([&catalog])
    }

    fn workspace_metadata(path: &str) -> SourceFileMetadata {
        SourceFileMetadata {
            kind: SourceKind::Workspace,
            category: SourceCategory::Workspace,
            absolute_path: Some(PathBuf::from("C:/workspace").join(path)),
            root_path: Some(PathBuf::from("C:/workspace")),
            relative_path: Some(PathBuf::from(path)),
            priority: SOURCE_PRIORITY_WORKSPACE,
        }
    }

    fn game_metadata(path: &str) -> SourceFileMetadata {
        let relative_path = PathBuf::from(path);
        SourceFileMetadata {
            kind: SourceKind::GameData,
            category: source_category_for_path(SourceKind::GameData, Some(&relative_path)),
            absolute_path: Some(PathBuf::from("C:/game").join(path)),
            root_path: Some(PathBuf::from("C:/game")),
            relative_path: Some(relative_path),
            priority: SOURCE_PRIORITY_GAME_DATA,
        }
    }
}
