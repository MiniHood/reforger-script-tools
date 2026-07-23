use crate::index::{CompletionMemberLookup, GlobalSymbolId, IndexedConditionalBranch, SymbolIndex};
use crate::lexer::TextSpan;
use crate::model::{CallableForm, SourceCategory, SourceKind, SymbolKind};
use crate::symbol_display::{SymbolDisplay, SymbolDisplayInfo};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct IndexQuery<'index> {
    index: &'index SymbolIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCompletionMembers {
    pub raw_candidates: Vec<GlobalSymbolId>,
    pub candidates: Vec<EditorCompletionCandidate>,
    pub shadowed_groups: Vec<EditorMemberShadowGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCompletionCandidate {
    pub id: GlobalSymbolId,
    pub name: Option<String>,
    pub kind: SymbolKind,
    pub detail: Option<String>,
    pub signature: Option<String>,
    pub constructor_signature: Option<String>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub source_kind: SourceKind,
    pub source_category: SourceCategory,
    pub source_priority: u16,
    pub relative_path: Option<PathBuf>,
    pub absolute_path: Option<PathBuf>,
    pub is_attribute_like: bool,
    pub origin: EditorCompletionOrigin,
    pub conditional_context: Vec<IndexedConditionalBranch>,
    pub callable_form: Option<CallableForm>,
    /// The declared arity of a class type. This is source-derived completion
    /// metadata, not a completion-side name allowlist.
    pub generic_type_parameter_count: usize,
    pub display: SymbolDisplayInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTopLevelCompletionMode {
    Type,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCompletionOrigin {
    Direct,
    Overlay,
    Inherited,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorMemberShadowGroup {
    pub key: String,
    pub kept: GlobalSymbolId,
    pub shadowed: Vec<GlobalSymbolId>,
}

impl<'index> IndexQuery<'index> {
    pub const fn new(index: &'index SymbolIndex) -> Self {
        Self { index }
    }

    pub fn preferred_class(&self, name: &str) -> Option<GlobalSymbolId> {
        self.index.preferred_classes_by_name(name).first().copied()
    }

    pub fn preferred_typedef(&self, name: &str) -> Option<GlobalSymbolId> {
        self.index.preferred_typedefs_by_name(name).first().copied()
    }

    pub fn preferred_function(&self, name: &str) -> Option<GlobalSymbolId> {
        self.index
            .preferred_functions_by_name(name)
            .first()
            .copied()
    }

    pub fn top_level_conflicts(&self, name: &str) -> Vec<GlobalSymbolId> {
        self.index.top_level_symbols_for_name(name).to_vec()
    }

    pub fn callable_signature(&self, id: GlobalSymbolId) -> Option<String> {
        self.index.callable_signature(id)
    }

    pub fn symbol_display(&self, id: GlobalSymbolId) -> Option<SymbolDisplayInfo> {
        SymbolDisplay::for_symbol(self.index, id)
    }

    pub fn completion_members_for_class(&self, name: &str) -> EditorCompletionMembers {
        let completion = self.index.completion_members_for_preferred_class(name);
        let members = self.editor_completion_members(name, completion);
        if !members.raw_candidates.is_empty() || !members.candidates.is_empty() {
            return members;
        }

        if let Some(target) = self.typedef_target_owner(name) {
            let completion = self.index.completion_members_for_preferred_class(&target);
            return self.editor_completion_members(&target, completion);
        }

        members
    }

    pub fn completion_static_members_for_type(&self, name: &str) -> Vec<EditorCompletionCandidate> {
        let mut candidates = self.enum_member_completion_candidates(name);
        if !candidates.is_empty() {
            return candidates;
        }

        candidates = self
            .completion_members_for_class(name)
            .candidates
            .into_iter()
            .filter(|candidate| {
                matches!(
                    candidate.kind,
                    SymbolKind::Field | SymbolKind::Method | SymbolKind::Constructor
                ) && candidate
                    .display
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "static")
            })
            .collect();

        if self.editor_class_owner_exists(name) {
            candidates.extend(self.engine_class_cast_completion_candidates());
            candidates = dedupe_completion_candidates(candidates);
        } else if let Some(target) = self.typedef_target_owner(name) {
            return self.completion_static_members_for_type(&target);
        }
        candidates
    }

    pub fn completion_top_level(
        &self,
        prefix: &str,
        mode: EditorTopLevelCompletionMode,
    ) -> Vec<EditorCompletionCandidate> {
        self.completion_top_level_limited(prefix, mode, usize::MAX)
    }

    /// Returns every indexed active Macro. Unlike ordinary editor completion,
    /// preprocessor operands intentionally include every source layer.
    pub fn completion_preprocessor_macros(&self, prefix: &str) -> Vec<EditorCompletionCandidate> {
        let mut candidates = self
            .index
            .symbols()
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::PreprocessorMacro)
            .filter(|symbol| {
                symbol
                    .name
                    .as_deref()
                    .is_some_and(|name| completion_name_match_rank(name, prefix).is_some())
            })
            .filter_map(|symbol| {
                self.editor_symbol_completion_candidate(symbol.id, EditorCompletionOrigin::Unknown)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            completion_name_match_rank(&left.display.label, prefix)
                .unwrap_or(u16::MAX)
                .cmp(&completion_name_match_rank(&right.display.label, prefix).unwrap_or(u16::MAX))
                .then_with(|| right.source_priority.cmp(&left.source_priority))
                .then_with(|| left.display.label.cmp(&right.display.label))
                .then_with(|| left.id.file_id.cmp(&right.id.file_id))
                .then_with(|| left.id.symbol_id.cmp(&right.id.symbol_id))
        });
        candidates
    }

    pub fn completion_top_level_limited(
        &self,
        prefix: &str,
        mode: EditorTopLevelCompletionMode,
        limit: usize,
    ) -> Vec<EditorCompletionCandidate> {
        if prefix.is_empty() && mode != EditorTopLevelCompletionMode::Type {
            return Vec::new();
        }
        if limit == 0 {
            return Vec::new();
        }

        let mut ids_by_key = BTreeMap::<String, Vec<GlobalSymbolId>>::new();
        let mut key_order = Vec::<String>::new();

        for (name, ids) in self.index.top_level_names() {
            if !prefix.is_empty() && completion_name_match_rank(name, prefix).is_none() {
                continue;
            }
            for id in ids {
                let Some(symbol) = self.index.symbol(*id) else {
                    continue;
                };
                if !self.is_editor_completion_source(symbol.id) {
                    continue;
                }
                if !top_level_completion_kind_allowed(symbol.kind, mode) {
                    continue;
                }
                let key = top_level_completion_key(self.index, symbol.id, symbol.kind, name);
                if !ids_by_key.contains_key(&key) {
                    key_order.push(key.clone());
                }
                ids_by_key.entry(key).or_default().push(symbol.id);
            }
        }

        let mut candidates = Vec::new();
        for key in key_order {
            let mut ids = ids_by_key.remove(&key).unwrap_or_default();
            ids.sort_by(|left, right| self.compare_symbol_preference(*left, *right));
            if let Some(candidate) = ids
                .first()
                .copied()
                .and_then(|id| self.editor_top_level_completion_candidate(id))
            {
                candidates.push(candidate);
            }
        }

        candidates.sort_by(|left, right| {
            completion_name_match_rank(&left.display.label, prefix)
                .unwrap_or(u16::MAX)
                .cmp(&completion_name_match_rank(&right.display.label, prefix).unwrap_or(u16::MAX))
                .then_with(|| left.display.label.cmp(&right.display.label))
                .then_with(|| {
                    completion_kind_rank(left.kind).cmp(&completion_kind_rank(right.kind))
                })
                .then_with(|| right.source_priority.cmp(&left.source_priority))
                .then_with(|| left.id.file_id.cmp(&right.id.file_id))
                .then_with(|| left.id.symbol_id.cmp(&right.id.symbol_id))
        });
        candidates.truncate(limit);
        candidates
    }

    pub fn completion_symbols(
        &self,
        ids: impl IntoIterator<Item = GlobalSymbolId>,
        origin: EditorCompletionOrigin,
    ) -> Vec<EditorCompletionCandidate> {
        ids.into_iter()
            .filter(|id| self.is_editor_completion_source(*id))
            .filter_map(|id| self.editor_symbol_completion_candidate(id, origin))
            .collect()
    }

    pub fn raw_symbols_for_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.index.symbols_for_name(name)
    }

    pub fn raw_top_level_symbols_for_name(&self, name: &str) -> &[GlobalSymbolId] {
        self.index.top_level_symbols_for_name(name)
    }

    pub fn raw_completion_members_for_owner_name(&self, owner: &str) -> CompletionMemberLookup {
        self.index.raw_completion_members_for_owner_name(owner)
    }

    fn editor_completion_members(
        &self,
        owner: &str,
        completion: CompletionMemberLookup,
    ) -> EditorCompletionMembers {
        let preferred_class = self.preferred_editor_class(owner);
        if preferred_class.is_none() {
            return EditorCompletionMembers {
                raw_candidates: completion.raw_candidates,
                candidates: Vec::new(),
                shadowed_groups: Vec::new(),
            };
        }

        let (candidates, shadowed_groups) =
            self.filtered_editor_completion_candidates(owner, preferred_class, &completion);

        EditorCompletionMembers {
            raw_candidates: completion.raw_candidates,
            candidates,
            shadowed_groups,
        }
    }

    fn preferred_editor_class(&self, name: &str) -> Option<GlobalSymbolId> {
        self.index
            .preferred_classes_by_name(name)
            .into_iter()
            .find(|id| self.is_editor_completion_source(*id))
    }

    fn filtered_editor_completion_candidates(
        &self,
        owner: &str,
        preferred_class: Option<GlobalSymbolId>,
        completion: &CompletionMemberLookup,
    ) -> (Vec<EditorCompletionCandidate>, Vec<EditorMemberShadowGroup>) {
        let mut ids_by_key = BTreeMap::<String, Vec<GlobalSymbolId>>::new();
        let mut key_order = Vec::<String>::new();

        for id in &completion.raw_candidates {
            if !self.is_editor_completion_source(*id) {
                continue;
            }
            let key = self.index.completion_member_key(*id);
            if !ids_by_key.contains_key(&key) {
                key_order.push(key.clone());
            }
            ids_by_key.entry(key).or_default().push(*id);
        }

        let mut candidates = Vec::new();
        let mut shadowed_groups = Vec::new();
        for key in key_order {
            let mut ids = ids_by_key.remove(&key).unwrap_or_default();
            ids.sort_by(|left, right| {
                self.compare_editor_completion_preference(owner, preferred_class, *left, *right)
            });
            let Some(kept) = ids.first().copied() else {
                continue;
            };
            if let Some(candidate) = self.editor_completion_candidate(owner, preferred_class, kept)
            {
                candidates.push(candidate);
            }
            let shadowed = ids.into_iter().filter(|id| *id != kept).collect::<Vec<_>>();
            if !shadowed.is_empty() {
                shadowed_groups.push(EditorMemberShadowGroup {
                    key,
                    kept,
                    shadowed,
                });
            }
        }

        (candidates, shadowed_groups)
    }

    fn is_editor_completion_source(&self, id: GlobalSymbolId) -> bool {
        self.index
            .file(id.file_id)
            .is_some_and(|file| file.metadata.category.is_editor_completion_default())
    }

    fn compare_editor_completion_preference(
        &self,
        owner: &str,
        preferred_class: Option<GlobalSymbolId>,
        left: GlobalSymbolId,
        right: GlobalSymbolId,
    ) -> std::cmp::Ordering {
        let left_origin = self.completion_origin(
            owner,
            preferred_class,
            self.index.symbol(left).and_then(|s| s.parent),
        );
        let right_origin = self.completion_origin(
            owner,
            preferred_class,
            self.index.symbol(right).and_then(|s| s.parent),
        );
        editor_origin_rank(left_origin)
            .cmp(&editor_origin_rank(right_origin))
            .then_with(|| {
                let left_priority = self
                    .index
                    .file(left.file_id)
                    .map(|file| file.metadata.priority)
                    .unwrap_or_default();
                let right_priority = self
                    .index
                    .file(right.file_id)
                    .map(|file| file.metadata.priority)
                    .unwrap_or_default();
                right_priority.cmp(&left_priority)
            })
            .then_with(|| {
                let left_form = self
                    .index
                    .symbol(left)
                    .and_then(|symbol| symbol.callable_form);
                let right_form = self
                    .index
                    .symbol(right)
                    .and_then(|symbol| symbol.callable_form);
                callable_form_rank(left_form).cmp(&callable_form_rank(right_form))
            })
            .then_with(|| left.file_id.cmp(&right.file_id))
            .then_with(|| left.symbol_id.cmp(&right.symbol_id))
    }

    fn compare_symbol_preference(
        &self,
        left: GlobalSymbolId,
        right: GlobalSymbolId,
    ) -> std::cmp::Ordering {
        let left_priority = self
            .index
            .file(left.file_id)
            .map(|file| file.metadata.priority)
            .unwrap_or_default();
        let right_priority = self
            .index
            .file(right.file_id)
            .map(|file| file.metadata.priority)
            .unwrap_or_default();
        right_priority
            .cmp(&left_priority)
            .then_with(|| left.file_id.cmp(&right.file_id))
            .then_with(|| left.symbol_id.cmp(&right.symbol_id))
    }

    fn editor_completion_candidate(
        &self,
        owner: &str,
        preferred_class: Option<GlobalSymbolId>,
        id: GlobalSymbolId,
    ) -> Option<EditorCompletionCandidate> {
        let symbol = self.index.symbol(id)?;
        let file = self.index.file(id.file_id)?;
        let origin = self.completion_origin(owner, preferred_class, symbol.parent);
        let display = self.symbol_display(id)?;
        let detail = display.detail.clone();
        let constructor_signature = self.class_constructor_signature(symbol.id, symbol.kind);
        let is_attribute_like = self.is_attribute_like_class(symbol.id, symbol.kind);

        Some(EditorCompletionCandidate {
            id,
            name: symbol.name.clone(),
            kind: symbol.kind,
            detail,
            signature: display.signature.clone(),
            constructor_signature,
            span: symbol.span,
            selection_span: symbol.selection_span,
            source_kind: file.metadata.kind,
            source_category: file.metadata.category,
            source_priority: file.metadata.priority,
            relative_path: file.metadata.relative_path.clone(),
            absolute_path: file.metadata.absolute_path.clone(),
            is_attribute_like,
            origin,
            conditional_context: symbol.conditional_context.clone(),
            callable_form: symbol.callable_form,
            generic_type_parameter_count: self.generic_type_parameter_count(symbol.id, symbol.kind),
            display,
        })
    }

    fn editor_top_level_completion_candidate(
        &self,
        id: GlobalSymbolId,
    ) -> Option<EditorCompletionCandidate> {
        self.editor_symbol_completion_candidate(id, EditorCompletionOrigin::Unknown)
    }

    fn enum_member_completion_candidates(&self, name: &str) -> Vec<EditorCompletionCandidate> {
        let enum_ids = self.preferred_editor_enums(name);
        if enum_ids.is_empty() {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        for enum_id in enum_ids {
            for child_id in self.index.children(enum_id) {
                let Some(symbol) = self.index.symbol(*child_id) else {
                    continue;
                };
                if symbol.kind != SymbolKind::EnumMember {
                    continue;
                }
                if !self.is_editor_completion_source(*child_id) {
                    continue;
                }
                if let Some(candidate) = self.editor_static_completion_candidate(*child_id) {
                    candidates.push(candidate);
                }
            }
        }
        candidates
    }

    fn preferred_editor_enums(&self, name: &str) -> Vec<GlobalSymbolId> {
        let mut ids = self
            .index
            .top_level_symbols_for_name(name)
            .iter()
            .copied()
            .filter(|id| {
                self.index
                    .symbol(*id)
                    .is_some_and(|symbol| symbol.kind == SymbolKind::Enum)
                    && self.is_editor_completion_source(*id)
            })
            .collect::<Vec<_>>();

        for typedef_id in self.index.preferred_typedefs_by_name(name) {
            if !self.is_editor_completion_source(typedef_id) {
                continue;
            }
            let Some(target) = self
                .index
                .symbol(typedef_id)
                .and_then(|symbol| symbol.detail.type_text.as_deref())
                .and_then(owner_type_from_type_text)
            else {
                continue;
            };
            ids.extend(
                self.index
                    .top_level_symbols_for_name(&target)
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.index
                            .symbol(*id)
                            .is_some_and(|symbol| symbol.kind == SymbolKind::Enum)
                            && self.is_editor_completion_source(*id)
                    }),
            );
        }

        self.index.preferred_from_symbols(&ids)
    }

    fn typedef_target_owner(&self, name: &str) -> Option<String> {
        self.index
            .preferred_typedefs_by_name(name)
            .into_iter()
            .find(|id| self.is_editor_completion_source(*id))
            .and_then(|id| {
                self.index
                    .symbol(id)
                    .and_then(|symbol| symbol.detail.type_text.as_deref())
                    .and_then(owner_type_from_type_text)
            })
            .filter(|target| target != name)
    }

    fn editor_class_owner_exists(&self, name: &str) -> bool {
        self.index
            .preferred_classes_by_name(name)
            .into_iter()
            .any(|id| self.is_editor_completion_source(id))
    }

    fn engine_class_cast_completion_candidates(&self) -> Vec<EditorCompletionCandidate> {
        self.completion_members_for_class("Class")
            .candidates
            .into_iter()
            .filter(|candidate| {
                candidate.kind == SymbolKind::Method && candidate.name.as_deref() == Some("Cast")
            })
            .collect()
    }

    fn editor_static_completion_candidate(
        &self,
        id: GlobalSymbolId,
    ) -> Option<EditorCompletionCandidate> {
        self.editor_symbol_completion_candidate(id, EditorCompletionOrigin::Unknown)
    }

    fn editor_symbol_completion_candidate(
        &self,
        id: GlobalSymbolId,
        origin: EditorCompletionOrigin,
    ) -> Option<EditorCompletionCandidate> {
        let symbol = self.index.symbol(id)?;
        let file = self.index.file(id.file_id)?;
        let display = self.symbol_display(id)?;
        let detail = display.detail.clone();
        let constructor_signature = self.class_constructor_signature(symbol.id, symbol.kind);
        let is_attribute_like = self.is_attribute_like_class(symbol.id, symbol.kind);

        Some(EditorCompletionCandidate {
            id,
            name: symbol.name.clone(),
            kind: symbol.kind,
            detail,
            signature: display.signature.clone(),
            constructor_signature,
            span: symbol.span,
            selection_span: symbol.selection_span,
            source_kind: file.metadata.kind,
            source_category: file.metadata.category,
            source_priority: file.metadata.priority,
            relative_path: file.metadata.relative_path.clone(),
            absolute_path: file.metadata.absolute_path.clone(),
            is_attribute_like,
            origin,
            conditional_context: symbol.conditional_context.clone(),
            callable_form: symbol.callable_form,
            generic_type_parameter_count: self.generic_type_parameter_count(symbol.id, symbol.kind),
            display,
        })
    }

    fn generic_type_parameter_count(&self, id: GlobalSymbolId, kind: SymbolKind) -> usize {
        (kind == SymbolKind::Class)
            .then(|| self.index.children(id))
            .into_iter()
            .flatten()
            .filter(|child| {
                self.index
                    .symbol(**child)
                    .is_some_and(|symbol| symbol.kind == SymbolKind::TypeParameter)
            })
            .count()
    }

    fn completion_origin(
        &self,
        owner: &str,
        preferred_class: Option<GlobalSymbolId>,
        parent: Option<GlobalSymbolId>,
    ) -> EditorCompletionOrigin {
        let Some(parent) = parent else {
            return EditorCompletionOrigin::Unknown;
        };
        let Some(parent_symbol) = self.index.symbol(parent) else {
            return EditorCompletionOrigin::Unknown;
        };
        let Some(parent_name) = parent_symbol.name.as_deref() else {
            return EditorCompletionOrigin::Unknown;
        };

        if parent_name != owner {
            return EditorCompletionOrigin::Inherited;
        }

        if Some(parent) == preferred_class {
            EditorCompletionOrigin::Direct
        } else {
            EditorCompletionOrigin::Overlay
        }
    }

    fn class_constructor_signature(&self, id: GlobalSymbolId, kind: SymbolKind) -> Option<String> {
        if kind != SymbolKind::Class {
            return None;
        }
        self.index.children(id).iter().find_map(|child_id| {
            self.index
                .symbol(*child_id)
                .filter(|symbol| symbol.kind == SymbolKind::Constructor)
                .and_then(|symbol| self.index.callable_signature(symbol.id))
        })
    }

    fn is_attribute_like_class(&self, id: GlobalSymbolId, kind: SymbolKind) -> bool {
        if kind != SymbolKind::Class {
            return false;
        }

        let mut current_base = self
            .index
            .symbol(id)
            .and_then(|symbol| symbol.detail.base_type.as_deref())
            .and_then(owner_type_from_type_text);
        let mut seen = BTreeSet::<String>::new();

        while let Some(base) = current_base {
            if base == "UniqueAttribute" {
                return true;
            }
            if !seen.insert(base.clone()) {
                return false;
            }

            let Some(base_id) = self.preferred_editor_class(&base) else {
                return base.ends_with("Attribute");
            };
            current_base = self
                .index
                .symbol(base_id)
                .and_then(|symbol| symbol.detail.base_type.as_deref())
                .and_then(owner_type_from_type_text);
        }

        false
    }
}

fn dedupe_completion_candidates(
    candidates: Vec<EditorCompletionCandidate>,
) -> Vec<EditorCompletionCandidate> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for candidate in candidates {
        let key = (
            candidate.kind,
            candidate.name.clone(),
            candidate.signature.clone(),
            candidate.detail.clone(),
        );
        if seen.insert(key) {
            deduped.push(candidate);
        }
    }
    deduped
}

fn owner_type_from_type_text(type_text: &str) -> Option<String> {
    let text = type_text.trim();
    if text.is_empty() {
        return None;
    }
    let text = text
        .strip_prefix("ref ")
        .or_else(|| text.strip_prefix("autoptr "))
        .or_else(|| text.strip_prefix("notnull "))
        .unwrap_or(text)
        .trim();
    let end = text
        .char_indices()
        .find_map(|(offset, ch)| {
            (!ch.is_ascii_alphanumeric() && ch != '_' && ch != ':').then_some(offset)
        })
        .unwrap_or(text.len());
    let name = text[..end].trim();
    (!name.is_empty()).then(|| name.to_string())
}

const fn editor_origin_rank(origin: EditorCompletionOrigin) -> u8 {
    match origin {
        EditorCompletionOrigin::Direct => 0,
        EditorCompletionOrigin::Overlay => 1,
        EditorCompletionOrigin::Inherited => 2,
        EditorCompletionOrigin::Unknown => 3,
    }
}

const fn callable_form_rank(form: Option<CallableForm>) -> u8 {
    match form {
        Some(CallableForm::Implementation) => 0,
        Some(CallableForm::Declaration) => 1,
        Some(CallableForm::Prototype) => 2,
        None => 3,
    }
}

const fn completion_kind_rank(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Class => 0,
        SymbolKind::Enum => 1,
        SymbolKind::Typedef => 2,
        SymbolKind::Function => 3,
        SymbolKind::GlobalField => 4,
        SymbolKind::EnumMember => 5,
        _ => 9,
    }
}

const fn top_level_completion_kind_allowed(
    kind: SymbolKind,
    mode: EditorTopLevelCompletionMode,
) -> bool {
    match mode {
        EditorTopLevelCompletionMode::Type => {
            matches!(
                kind,
                SymbolKind::Class | SymbolKind::Enum | SymbolKind::Typedef
            )
        }
        EditorTopLevelCompletionMode::Value => {
            matches!(
                kind,
                SymbolKind::Class
                    | SymbolKind::Enum
                    | SymbolKind::Typedef
                    | SymbolKind::Function
                    | SymbolKind::GlobalField
            )
        }
    }
}

fn top_level_completion_key(
    index: &SymbolIndex,
    id: GlobalSymbolId,
    kind: SymbolKind,
    name: &str,
) -> String {
    let signature = index.callable_signature(id).unwrap_or_default();
    format!("{kind:?}:{name}:{signature}")
}

pub(crate) fn completion_name_match_rank(value: &str, prefix: &str) -> Option<u16> {
    if prefix.is_empty() {
        return Some(0);
    }
    if value == prefix {
        return Some(0);
    }
    if value.eq_ignore_ascii_case(prefix) {
        return Some(1);
    }
    if starts_with_ignore_ascii_case(value, prefix) {
        return Some(10 + completion_match_tiebreak_score(value, prefix));
    }
    if prefix.chars().count() >= 2 {
        if let Some(score) = boundary_abbreviation_match_score(value, prefix) {
            return Some(100 + score.min(99));
        }
    }
    if prefix.chars().count() >= 2 {
        if let Some(score) = subsequence_match_score(value, prefix) {
            return Some(200 + score.min(u16::MAX - 200));
        }
    }
    None
}

fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn length_delta_score(value: &str, prefix: &str) -> u16 {
    value
        .chars()
        .count()
        .saturating_sub(prefix.chars().count())
        .min(u16::MAX as usize) as u16
}

fn completion_match_tiebreak_score(value: &str, prefix: &str) -> u16 {
    length_delta_score(value, prefix).min(89)
}

fn boundary_abbreviation_match_score(value: &str, prefix: &str) -> Option<u16> {
    let boundaries = completion_word_boundaries(value);
    let mut boundary_index = 0usize;
    let mut score = 0u16;

    for prefix_char in prefix.chars() {
        let target = prefix_char.to_ascii_lowercase();
        let mut matched = None;
        while boundary_index < boundaries.len() {
            let (index, ch) = boundaries[boundary_index];
            boundary_index += 1;
            if ch.to_ascii_lowercase() == target {
                matched = Some(index);
                break;
            }
        }
        score = score.saturating_add(matched?.min(u16::MAX as usize) as u16);
    }

    Some(score.saturating_add(length_delta_score(value, prefix)))
}

fn completion_word_boundaries(value: &str) -> Vec<(usize, char)> {
    let mut boundaries = Vec::new();
    let mut previous = None;
    for (index, ch) in value.char_indices() {
        let is_boundary = index == 0
            || previous.is_some_and(|prev: char| prev == '_' || !prev.is_ascii_alphanumeric())
            || (ch.is_ascii_uppercase()
                && previous
                    .is_some_and(|prev: char| prev.is_ascii_lowercase() || prev.is_ascii_digit()));
        if is_boundary && ch.is_ascii_alphanumeric() {
            boundaries.push((index, ch));
        }
        previous = Some(ch);
    }
    boundaries
}

fn subsequence_match_score(value: &str, prefix: &str) -> Option<u16> {
    let mut value_chars = value.char_indices();
    let mut last_index = 0usize;
    let mut score = 0u16;

    for prefix_char in prefix.chars() {
        let target = prefix_char.to_ascii_lowercase();
        let mut matched = None;
        for (index, ch) in value_chars.by_ref() {
            if ch.to_ascii_lowercase() == target {
                matched = Some(index);
                break;
            }
        }
        let index = matched?;
        score =
            score.saturating_add(index.saturating_sub(last_index).min(u16::MAX as usize) as u16);
        last_index = index;
    }

    Some(score.saturating_add(length_delta_score(value, prefix)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::index::SymbolIndex;
    use crate::model::{
        source_category_for_path, SourceCategory, SourceFileMetadata, SymbolCatalog,
        SOURCE_PRIORITY_GAME_DATA, SOURCE_PRIORITY_WORKSPACE,
    };
    use crate::parser::parse_source;
    use std::path::PathBuf;

    #[test]
    fn preferred_kind_lookup_returns_workspace_symbols_first() {
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
        let query = IndexQuery::new(&index);

        for id in [
            query.preferred_class("Example").unwrap(),
            query.preferred_typedef("ExampleAlias").unwrap(),
            query.preferred_function("ExampleFn").unwrap(),
        ] {
            assert_eq!(
                index.file(id.file_id).unwrap().metadata.kind,
                SourceKind::Workspace
            );
        }
    }

    #[test]
    fn completion_name_match_rank_prefers_prefixes_over_fuzzy_matches() {
        let prefix_rank = completion_name_match_rank("OnPostInit", "on")
            .expect("expected a case-insensitive prefix match");
        let boundary_rank = completion_name_match_rank("SCR_OrientToSeaNormalContextAction", "on")
            .expect("expected a boundary abbreviation match");
        let subsequence_rank =
            completion_name_match_rank("Ocean", "on").expect("expected a subsequence match");

        assert!(prefix_rank < boundary_rank);
        assert!(boundary_rank < subsequence_rank);
    }

    #[test]
    fn top_level_conflicts_returns_mixed_kinds_without_authoritative_preference() {
        let catalog = catalog(
            r#"typedef string FactionKey;
class FactionKey : string {}
void FactionKey(int value);
"#,
            workspace_metadata("FactionKey.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let conflicts = query.top_level_conflicts("FactionKey");

        assert_eq!(conflicts.len(), 3);
        assert!(conflicts
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Class));
        assert!(conflicts
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Typedef));
        assert!(conflicts
            .iter()
            .any(|id| index.symbol(*id).unwrap().kind == SymbolKind::Function));
    }

    #[test]
    fn top_level_type_completion_returns_only_type_like_symbols() {
        let catalog = catalog(
            r#"class SCR_Type {}
enum SCR_Mode
{
	SCR_Value
}
typedef int SCR_Alias;
void SCR_Function();
int SCR_Global;
"#,
            workspace_metadata("Types.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_top_level("SCR_", EditorTopLevelCompletionMode::Type);

        assert_eq!(
            completion
                .iter()
                .map(|candidate| (candidate.name.as_deref().unwrap(), candidate.kind))
                .collect::<Vec<_>>(),
            vec![
                ("SCR_Mode", SymbolKind::Enum),
                ("SCR_Type", SymbolKind::Class),
                ("SCR_Alias", SymbolKind::Typedef)
            ]
        );
    }

    #[test]
    fn top_level_completion_uses_match_quality_before_limit() {
        let mut source = String::new();
        for index in 0..400 {
            source.push_str(&format!("class RplGenerated{index} {{}}\n"));
        }
        source.push_str("class RplProp {}\n");
        let catalog = catalog(&source, game_metadata("Game.c"));
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion =
            query.completion_top_level_limited("rp", EditorTopLevelCompletionMode::Type, 250);

        assert_eq!(completion.len(), 250);
        assert_eq!(completion.first().unwrap().name.as_deref(), Some("RplProp"));
        assert!(completion
            .iter()
            .any(|candidate| candidate.name.as_deref() == Some("RplProp")));
    }

    #[test]
    fn top_level_completion_marks_indirect_unique_attribute_classes() {
        let catalog = catalog(
            r#"class UniqueAttribute {}
class SharedAttributeBase : UniqueAttribute {}
class CustomGameplayFlag : SharedAttributeBase {}
class NotAttributeBase {}
class LooksLikeAttribute : NotAttributeBase {}
"#,
            game_metadata("Game/Attributes.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_top_level("Custom", EditorTopLevelCompletionMode::Type);
        let custom = completion
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("CustomGameplayFlag"))
            .expect("expected custom attribute candidate");
        assert!(custom.is_attribute_like);

        let completion = query.completion_top_level("Looks", EditorTopLevelCompletionMode::Type);
        let looks_like = completion
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("LooksLikeAttribute"))
            .expect("expected suffix-only non-attribute candidate");
        assert!(!looks_like.is_attribute_like);
    }

    #[test]
    fn top_level_value_completion_includes_runtime_values_and_prefers_workspace() {
        let game = catalog(
            r#"class SCR_Shared {}
void SCR_Function();
int SCR_Global;
enum SCR_Mode
{
	SCR_Value
}
"#,
            game_metadata("Game.c"),
        );
        let workspace = catalog(
            r#"class SCR_Shared {}
void SCR_WorkspaceOnly();
"#,
            workspace_metadata("Workspace.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_top_level("SCR_", EditorTopLevelCompletionMode::Value);

        assert!(completion.iter().any(|candidate| candidate.name.as_deref()
            == Some("SCR_Function")
            && candidate.kind == SymbolKind::Function));
        assert!(completion
            .iter()
            .any(|candidate| candidate.name.as_deref() == Some("SCR_Global")
                && candidate.kind == SymbolKind::GlobalField));
        assert!(!completion
            .iter()
            .any(|candidate| candidate.name.as_deref() == Some("SCR_Value")));
        let shared = completion
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("SCR_Shared"))
            .unwrap();
        assert_eq!(shared.source_kind, SourceKind::Workspace);
    }

    #[test]
    fn editor_completion_uses_preferred_class_overlay_path() {
        let game = catalog(
            r#"class BaseMode
{
	void BaseOnly();
	void OnGameStart();
}

class SCR_BaseGameMode : BaseMode
{
	void OnGameStart();
	void GameOnly();
}
"#,
            game_metadata("Game.c"),
        );
        let workspace = catalog(
            r#"modded class SCR_BaseGameMode
{
	override void OnGameStart();
	void WorkspaceOnly();
}
"#,
            workspace_metadata("Workspace.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_members_for_class("SCR_BaseGameMode");

        assert_eq!(
            completion
                .candidates
                .iter()
                .map(|candidate| (
                    candidate.name.as_deref().unwrap_or("<missing>"),
                    candidate.origin,
                    candidate.source_kind
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "OnGameStart",
                    EditorCompletionOrigin::Direct,
                    SourceKind::Workspace
                ),
                (
                    "WorkspaceOnly",
                    EditorCompletionOrigin::Direct,
                    SourceKind::Workspace
                ),
                (
                    "GameOnly",
                    EditorCompletionOrigin::Overlay,
                    SourceKind::GameData
                ),
                (
                    "BaseOnly",
                    EditorCompletionOrigin::Inherited,
                    SourceKind::GameData
                )
            ]
        );

        let on_game_start = completion
            .candidates
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("OnGameStart"))
            .unwrap();
        assert_eq!(on_game_start.source_kind, SourceKind::Workspace);
        assert_eq!(
            on_game_start.signature.as_deref(),
            Some("SCR_BaseGameMode.OnGameStart() -> void")
        );

        let shadow_group = completion
            .shadowed_groups
            .iter()
            .find(|group| group.key == "Method OnGameStart() -> void")
            .unwrap();
        assert_eq!(shadow_group.kept, on_game_start.id);
        assert!(shadow_group.shadowed.iter().any(|id| index
            .file(id.file_id)
            .is_some_and(|file| file.metadata.kind == SourceKind::GameData)));
    }

    #[test]
    fn raw_owner_name_completion_stays_separate_from_editor_completion() {
        let game = catalog(
            r#"class SCR_BaseGameMode
{
	void GameOnly();
}
"#,
            game_metadata("Game.c"),
        );
        let workspace = catalog(
            r#"modded class SCR_BaseGameMode
{
	void WorkspaceOnly();
}
"#,
            workspace_metadata("Workspace.c"),
        );
        let index = SymbolIndex::from_catalogs([&game, &workspace]);
        let query = IndexQuery::new(&index);

        let raw = query.raw_completion_members_for_owner_name("SCR_BaseGameMode");
        let editor = query.completion_members_for_class("SCR_BaseGameMode");

        assert_eq!(raw.members.len(), 2);
        assert_eq!(editor.candidates.len(), 2);
        assert!(raw.members.iter().all(|id| editor
            .candidates
            .iter()
            .any(|candidate| candidate.id == *id)));
    }

    #[test]
    fn editor_completion_excludes_non_runtime_source_categories_by_default() {
        let docs = catalog(
            r#"class Example
{
	void DocsOnly();
}
"#,
            game_metadata("GameLib/WorldSystemsDocs.c"),
        );
        let workbench = catalog(
            r#"class Example
{
	void WorkbenchOnly();
}
"#,
            game_metadata("WorkbenchGame/Example.c"),
        );
        let runtime = catalog(
            r#"class Example
{
	void RuntimeOnly();
}
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&docs, &workbench, &runtime]);
        let query = IndexQuery::new(&index);

        let raw = query.raw_completion_members_for_owner_name("Example");
        let editor = query.completion_members_for_class("Example");

        assert_eq!(raw.members.len(), 3);
        assert_eq!(
            editor
                .candidates
                .iter()
                .map(|candidate| candidate.name.as_deref().unwrap())
                .collect::<Vec<_>>(),
            vec!["RuntimeOnly"]
        );
        assert_eq!(editor.candidates[0].source_category, SourceCategory::Game);
    }

    #[test]
    fn editor_completion_returns_no_candidates_when_class_is_docs_only() {
        let docs = catalog(
            r#"class HelloWorldSystem : WorldSystem
{
	void DocsOnly();
}

class WorldSystem
{
	void GeneratedBase();
}
"#,
            game_metadata("GameLib/WorldSystemsDocs.c"),
        );
        let index = SymbolIndex::from_catalogs([&docs]);
        let query = IndexQuery::new(&index);

        let editor = query.completion_members_for_class("HelloWorldSystem");

        assert!(!editor.raw_candidates.is_empty());
        assert!(editor.candidates.is_empty());
        assert!(editor.shadowed_groups.is_empty());
    }

    #[test]
    fn editor_completion_uses_lower_priority_runtime_class_when_higher_priority_class_is_excluded()
    {
        let docs = catalog(
            r#"class Example
{
	void DocsOnly();
}
"#,
            game_metadata("GameLib/WorldSystemsDocs.c"),
        );
        let runtime = catalog(
            r#"class Example
{
	void RuntimeOnly();
}
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&docs, &runtime]);
        let query = IndexQuery::new(&index);

        let editor = query.completion_members_for_class("Example");

        assert_eq!(
            editor
                .candidates
                .iter()
                .map(|candidate| (candidate.name.as_deref().unwrap(), candidate.origin))
                .collect::<Vec<_>>(),
            vec![("RuntimeOnly", EditorCompletionOrigin::Direct)]
        );
    }

    #[test]
    fn editor_completion_exposes_conditional_context_and_callable_form() {
        let catalog = catalog(
            r#"#ifndef DISABLE_INVENTORY
class Example
{
	void Run() {}
}
#endif
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_members_for_class("Example");
        let run = completion
            .candidates
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("Run"))
            .unwrap();

        assert_eq!(run.callable_form, Some(CallableForm::Implementation));
        assert_eq!(run.conditional_context.len(), 1);
        assert_eq!(run.conditional_context[0].kind.as_str(), "#ifndef");
        assert_eq!(
            run.conditional_context[0].condition.as_deref(),
            Some("DISABLE_INVENTORY")
        );
    }

    #[test]
    fn editor_completion_prefers_implementation_over_declaration_for_same_key() {
        let catalog = catalog(
            r#"class Example
{
	void Run();
	void Run() {}
}
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_members_for_class("Example");
        let run = completion
            .candidates
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("Run"))
            .unwrap();

        assert_eq!(run.callable_form, Some(CallableForm::Implementation));
        let shadow_group = completion
            .shadowed_groups
            .iter()
            .find(|group| group.key == "Method Run() -> void")
            .unwrap();
        assert_eq!(shadow_group.kept, run.id);
        assert_eq!(shadow_group.shadowed.len(), 1);
    }

    #[test]
    fn static_completion_returns_enum_members() {
        let catalog = catalog(
            r#"enum LogLevel
{
	DEBUG,
	NORMAL
}

typedef LogLevel ELogLevel;
"#,
            game_metadata("Game/LogLevel.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let direct = query.completion_static_members_for_type("LogLevel");
        assert_eq!(
            direct
                .iter()
                .map(|candidate| (candidate.name.as_deref().unwrap(), candidate.kind))
                .collect::<Vec<_>>(),
            vec![
                ("DEBUG", SymbolKind::EnumMember),
                ("NORMAL", SymbolKind::EnumMember)
            ]
        );

        let alias = query.completion_static_members_for_type("ELogLevel");
        assert_eq!(
            alias
                .iter()
                .map(|candidate| candidate.name.as_deref().unwrap())
                .collect::<Vec<_>>(),
            vec!["DEBUG", "NORMAL"]
        );
    }

    #[test]
    fn static_completion_returns_static_class_members_only() {
        let catalog = catalog(
            r#"class Example
{
	static int s_Value;
	static void StaticRun();
	void InstanceRun();
	int m_Value;
}
"#,
            game_metadata("Game/Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_static_members_for_type("Example");

        assert_eq!(
            completion
                .iter()
                .map(|candidate| (candidate.name.as_deref().unwrap(), candidate.kind))
                .collect::<Vec<_>>(),
            vec![
                ("s_Value", SymbolKind::Field),
                ("StaticRun", SymbolKind::Method)
            ]
        );
    }

    #[test]
    fn static_completion_includes_source_backed_engine_class_cast_rule() {
        let catalog = catalog(
            r#"class Class
{
	static Class Cast(Class from);
}

class Example
{
}
"#,
            game_metadata("Game/Class.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_static_members_for_type("Example");

        assert_eq!(
            completion
                .iter()
                .map(|candidate| (candidate.name.as_deref().unwrap(), candidate.kind))
                .collect::<Vec<_>>(),
            vec![("Cast", SymbolKind::Method)]
        );
    }

    #[test]
    fn member_completion_expands_typedef_owner_to_target_members() {
        let catalog = catalog(
            r#"class array<Class T>
{
	void Insert(T value);
	void Remove(T value);
}

typedef array<int> TIntArray;
"#,
            game_metadata("Game/Arrays.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_members_for_class("TIntArray");

        assert_eq!(
            completion
                .candidates
                .iter()
                .map(|candidate| candidate.name.as_deref().unwrap())
                .collect::<Vec<_>>(),
            vec!["Insert", "Remove"]
        );
    }

    #[test]
    fn callable_signature_delegates_to_index_for_all_callable_kinds() {
        let catalog = catalog(
            r#"void GlobalFn(int value = 4);

class Example
{
	void Run(string name);
	void Example(int value);
	void ~Example();
	int m_Value;
}
"#,
            workspace_metadata("Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        assert_eq!(
            query
                .callable_signature(find(&index, SymbolKind::Function, "GlobalFn"))
                .as_deref(),
            Some("GlobalFn(int value = 4) -> void")
        );
        assert_eq!(
            query
                .callable_signature(find(&index, SymbolKind::Method, "Run"))
                .as_deref(),
            Some("Example.Run(string name) -> void")
        );
        assert_eq!(
            query
                .callable_signature(find(&index, SymbolKind::Constructor, "Example"))
                .as_deref(),
            Some("Example(int value)")
        );
        assert_eq!(
            query
                .callable_signature(find(&index, SymbolKind::Destructor, "Example"))
                .as_deref(),
            Some("~Example()")
        );
        assert_eq!(
            query.callable_signature(find(&index, SymbolKind::Field, "m_Value")),
            None
        );
    }

    #[test]
    fn symbol_display_returns_editor_ready_metadata() {
        let catalog = catalog(
            r#"//! Run documentation.
class Example
{
	[Attribute()]
	protected void Run(int value = 4);
}
"#,
            workspace_metadata("Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);
        let method = find(&index, SymbolKind::Method, "Run");

        let display = query.symbol_display(method).unwrap();

        assert_eq!(display.label, "Run");
        assert_eq!(display.kind, SymbolKind::Method);
        assert_eq!(
            display.signature.as_deref(),
            Some("Example.Run(int value = 4) -> void")
        );
        assert_eq!(
            display.detail.as_deref(),
            Some("Example.Run(int value = 4) -> void")
        );
        assert_eq!(display.modifiers, vec!["protected"]);
        assert_eq!(display.attributes[0].name.as_deref(), Some("Attribute"));
        assert_eq!(display.source_kind, SourceKind::Workspace);
    }

    #[test]
    fn completion_candidates_include_shared_symbol_display() {
        let catalog = catalog(
            r#"class Example
{
	//! Value docs.
	int m_Value;
}
"#,
            workspace_metadata("Example.c"),
        );
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        let completion = query.completion_members_for_class("Example");
        let candidate = completion
            .candidates
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some("m_Value"))
            .unwrap();

        assert_eq!(candidate.detail.as_deref(), Some("type int"));
        assert_eq!(candidate.display.label, "m_Value");
        assert_eq!(candidate.display.detail.as_deref(), Some("type int"));
        assert_eq!(
            candidate.display.documentation_preview.as_deref(),
            Some("Value docs.")
        );
    }

    #[test]
    fn missing_names_are_empty_and_do_not_panic() {
        let catalog = catalog("class Example {}", workspace_metadata("Example.c"));
        let index = SymbolIndex::from_catalogs([&catalog]);
        let query = IndexQuery::new(&index);

        assert_eq!(query.preferred_class("Missing"), None);
        assert_eq!(query.preferred_typedef("Missing"), None);
        assert_eq!(query.preferred_function("Missing"), None);
        assert!(query.top_level_conflicts("Missing").is_empty());
        assert!(query.raw_symbols_for_name("Missing").is_empty());
        assert!(query.raw_top_level_symbols_for_name("Missing").is_empty());

        let completion = query.completion_members_for_class("Missing");
        assert!(completion.raw_candidates.is_empty());
        assert!(completion.candidates.is_empty());
        assert!(completion.shadowed_groups.is_empty());
    }

    fn find(index: &SymbolIndex, kind: SymbolKind, name: &str) -> GlobalSymbolId {
        index
            .symbols()
            .iter()
            .find(|symbol| symbol.kind == kind && symbol.name.as_deref() == Some(name))
            .map(|symbol| symbol.id)
            .unwrap_or_else(|| panic!("missing {kind:?} {name}"))
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
}
