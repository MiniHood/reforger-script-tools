use crate::lexer::TextSpan;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SOURCE_PRIORITY_UNKNOWN: u16 = 0;
pub const SOURCE_PRIORITY_FIXTURE: u16 = 50;
pub const SOURCE_PRIORITY_GAME_DATA: u16 = 100;
pub const SOURCE_PRIORITY_WORKSPACE: u16 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SourceKind {
    Unknown,
    GameData,
    Workspace,
    Fixture,
}

impl SourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::GameData => "GameData",
            Self::Workspace => "Workspace",
            Self::Fixture => "Fixture",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SourceCategory {
    Workspace,
    Game,
    GameCode,
    GameLib,
    Core,
    Generated,
    Workbench,
    DocsDoxygen,
    TestAutotest,
    Unknown,
}

impl SourceCategory {
    /// Categories that can occur in the immutable Game Data Catalogue.
    /// Workspace is intentionally excluded because it is an overlay, not
    /// extracted Game Data.
    pub const GAME_DATA_FILTER_VALUES: &'static [&'static str] = &[
        "core",
        "docs/doxygen",
        "game",
        "gamecode",
        "gamelib",
        "generated",
        "test/autotest",
        "unknown",
        "workbench",
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Game => "game",
            Self::GameCode => "gamecode",
            Self::GameLib => "gamelib",
            Self::Core => "core",
            Self::Generated => "generated",
            Self::Workbench => "workbench",
            Self::DocsDoxygen => "docs/doxygen",
            Self::TestAutotest => "test/autotest",
            Self::Unknown => "unknown",
        }
    }

    pub const fn is_editor_completion_default(self) -> bool {
        matches!(
            self,
            Self::Workspace
                | Self::Game
                | Self::GameCode
                | Self::GameLib
                | Self::Core
                | Self::Generated
        )
    }

    pub const fn is_generated(self) -> bool {
        matches!(self, Self::Generated)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualSourceIdentity {
    pub uri: String,
    pub addon_guid: String,
    pub revision: String,
    pub logical_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFileMetadata {
    pub kind: SourceKind,
    pub category: SourceCategory,
    pub absolute_path: Option<PathBuf>,
    #[serde(default)]
    pub virtual_source: Option<VirtualSourceIdentity>,
    pub root_path: Option<PathBuf>,
    pub relative_path: Option<PathBuf>,
    pub priority: u16,
}

impl SourceFileMetadata {
    pub const fn unknown() -> Self {
        Self {
            kind: SourceKind::Unknown,
            category: SourceCategory::Unknown,
            absolute_path: None,
            virtual_source: None,
            root_path: None,
            relative_path: None,
            priority: SOURCE_PRIORITY_UNKNOWN,
        }
    }
}

pub fn source_category_for_path(kind: SourceKind, path: Option<&Path>) -> SourceCategory {
    if kind == SourceKind::Workspace {
        return SourceCategory::Workspace;
    }

    let path = path
        .map(|path| path.to_string_lossy().replace('\\', "/").to_lowercase())
        .unwrap_or_default();
    // Workbench reports loose scripts relative to the add-on root, whereas
    // packed entries can already be relative to the script catalogue. Both
    // forms name the same engine source families.
    let path = path.strip_prefix("scripts/").unwrap_or(&path);

    if path.contains("/generated/") || path.starts_with("generated/") {
        SourceCategory::Generated
    } else if path.contains("docs") || path.contains("doxygen") {
        SourceCategory::DocsDoxygen
    } else if path.starts_with("autotest/")
        || path.contains("/autotest/")
        || path.contains("/tests/")
    {
        SourceCategory::TestAutotest
    } else if path.starts_with("workbench") {
        SourceCategory::Workbench
    } else if path.starts_with("gamecode/") {
        SourceCategory::GameCode
    } else if path.starts_with("gamelib/") {
        SourceCategory::GameLib
    } else if path.starts_with("game/") {
        SourceCategory::Game
    } else if path.starts_with("core/") {
        SourceCategory::Core
    } else {
        SourceCategory::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Class,
    TypeParameter,
    Enum,
    EnumMember,
    Typedef,
    Function,
    GlobalField,
    Field,
    Method,
    Constructor,
    Destructor,
    Parameter,
    LocalVariable,
    PreprocessorMacro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreprocessorBranchKind {
    If,
    Ifdef,
    Ifndef,
    Elif,
    Else,
}

impl PreprocessorBranchKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::If => "#if",
            Self::Ifdef => "#ifdef",
            Self::Ifndef => "#ifndef",
            Self::Elif => "#elif",
            Self::Else => "#else",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CallableForm {
    Implementation,
    Declaration,
    Prototype,
}

impl CallableForm {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implementation => "implementation",
            Self::Declaration => "declaration",
            Self::Prototype => "prototype",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeShape<'source> {
    source: &'source str,
    span: TextSpan,
    qualifiers: Vec<TextSpan>,
    base_name: Option<TextSpan>,
    generic_args: Vec<TypeShape<'source>>,
    array_suffixes: Vec<TextSpan>,
}

impl<'source> TypeShape<'source> {
    pub const fn span(&self) -> TextSpan {
        self.span
    }

    pub fn text(&self) -> &'source str {
        &self.source[self.span.start..self.span.end]
    }

    pub fn qualifier_spans(&self) -> &[TextSpan] {
        &self.qualifiers
    }

    pub fn qualifier_texts(&self) -> Vec<&'source str> {
        self.qualifiers
            .iter()
            .map(|span| &self.source[span.start..span.end])
            .collect()
    }

    pub const fn base_name_span(&self) -> Option<TextSpan> {
        self.base_name
    }

    pub fn base_name_text(&self) -> Option<&'source str> {
        self.base_name
            .map(|span| &self.source[span.start..span.end])
    }

    pub fn generic_args(&self) -> &[TypeShape<'source>] {
        &self.generic_args
    }

    pub fn array_suffix_spans(&self) -> &[TextSpan] {
        &self.array_suffixes
    }

    pub fn array_suffix_texts(&self) -> Vec<&'source str> {
        self.array_suffixes
            .iter()
            .map(|span| &self.source[span.start..span.end])
            .collect()
    }
}

pub fn declaration_type_shape(
    source: &str,
    type_span: TextSpan,
    declaration_span: TextSpan,
    name_span: Option<TextSpan>,
) -> TypeShape<'_> {
    let mut shape = parse_type_shape(source, trim_span(source, type_span));
    if let Some(name_span) = name_span {
        shape.array_suffixes.extend(array_suffixes_after_name(
            source,
            declaration_span,
            name_span,
        ));
    }
    shape
}

fn parse_type_shape(source: &str, span: TextSpan) -> TypeShape<'_> {
    let span = trim_span(source, span);
    let mut position = span.start;
    let mut qualifiers = Vec::new();

    loop {
        position = skip_whitespace(source, position, span.end);
        let Some(identifier) = identifier_span_at(source, position, span.end) else {
            break;
        };
        if !is_type_qualifier(&source[identifier.start..identifier.end]) {
            break;
        }
        qualifiers.push(identifier);
        position = identifier.end;
    }

    position = skip_whitespace(source, position, span.end);
    let base_name = identifier_span_at(source, position, span.end);
    if let Some(base) = base_name {
        position = base.end;
    }

    let mut generic_args = Vec::new();
    position = skip_whitespace(source, position, span.end);
    if position < span.end && source.as_bytes()[position] == b'<' {
        if let Some(generic_end) = matching_generic_end(source, position, span.end) {
            generic_args = parse_generic_args(source, position + 1, generic_end);
            position = generic_end + 1;
        }
    }

    let array_suffixes = array_suffixes_after_offset(source, position, span.end);

    TypeShape {
        source,
        span,
        qualifiers,
        base_name,
        generic_args,
        array_suffixes,
    }
}

fn trim_span(source: &str, span: TextSpan) -> TextSpan {
    let mut start = span.start;
    let mut end = span.end;

    while start < end {
        let Some(value) = source[start..end].chars().next() else {
            break;
        };
        if !value.is_whitespace() {
            break;
        }
        start += value.len_utf8();
    }

    while start < end {
        let Some(value) = source[start..end].chars().next_back() else {
            break;
        };
        if !value.is_whitespace() {
            break;
        }
        end -= value.len_utf8();
    }

    TextSpan::new(start, end)
}

fn skip_whitespace(source: &str, mut position: usize, end: usize) -> usize {
    while position < end {
        let Some(value) = source[position..end].chars().next() else {
            break;
        };
        if !value.is_whitespace() {
            break;
        }
        position += value.len_utf8();
    }
    position
}

fn identifier_span_at(source: &str, position: usize, end: usize) -> Option<TextSpan> {
    let mut chars = source[position..end].char_indices();
    let (_, first) = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }

    let mut identifier_end = position + first.len_utf8();
    for (index, value) in chars {
        if value.is_ascii_alphanumeric() || value == '_' {
            identifier_end = position + index + value.len_utf8();
        } else {
            break;
        }
    }

    Some(TextSpan::new(position, identifier_end))
}

fn is_type_qualifier(text: &str) -> bool {
    matches!(text, "ref" | "notnull" | "autoptr" | "owned")
}

fn matching_generic_end(source: &str, start: usize, end: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, value) in source[start..end].char_indices() {
        let offset = start + index;
        match value {
            '<' => depth += 1,
            '>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_generic_args(source: &str, start: usize, end: usize) -> Vec<TypeShape<'_>> {
    let mut args = Vec::new();
    let mut arg_start = start;
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    for (index, value) in source[start..end].char_indices() {
        let offset = start + index;
        match value {
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                let span = trim_span(source, TextSpan::new(arg_start, offset));
                if !span.is_empty() {
                    args.push(parse_type_shape(source, span));
                }
                arg_start = offset + value.len_utf8();
            }
            _ => {}
        }
    }

    let span = trim_span(source, TextSpan::new(arg_start, end));
    if !span.is_empty() {
        args.push(parse_type_shape(source, span));
    }

    args
}

fn array_suffixes_after_name(
    source: &str,
    record_span: TextSpan,
    name_span: TextSpan,
) -> Vec<TextSpan> {
    array_suffixes_after_offset(source, name_span.end, record_span.end)
}

fn array_suffixes_after_offset(source: &str, mut position: usize, end: usize) -> Vec<TextSpan> {
    let mut suffixes = Vec::new();

    loop {
        position = skip_whitespace(source, position, end);
        if position >= end || source.as_bytes()[position] != b'[' {
            break;
        }

        let suffix_start = position;
        position += 1;
        while position < end {
            let Some(value) = source[position..end].chars().next() else {
                break;
            };
            position += value.len_utf8();
            if value == ']' {
                suffixes.push(TextSpan::new(suffix_start, position));
                break;
            }
        }
    }

    suffixes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_data_filter_categories_are_canonical_and_exclude_workspace() {
        assert_eq!(
            SourceCategory::GAME_DATA_FILTER_VALUES,
            &[
                "core",
                "docs/doxygen",
                "game",
                "gamecode",
                "gamelib",
                "generated",
                "test/autotest",
                "unknown",
                "workbench",
            ]
        );
        assert!(SourceCategory::Generated.is_generated());
        assert!(!SourceCategory::Game.is_generated());
        assert!(!SourceCategory::Workspace.is_generated());
    }

    #[test]
    fn classifies_workbench_loose_script_paths_by_their_engine_family() {
        for (path, category) in [
            ("scripts/Game/game.c", SourceCategory::Game),
            ("scripts/GameCode/World.c", SourceCategory::GameCode),
            ("scripts/GameLib/Ui.c", SourceCategory::GameLib),
            ("scripts/Core/Math.c", SourceCategory::Core),
        ] {
            assert_eq!(
                source_category_for_path(SourceKind::GameData, Some(Path::new(path))),
                category,
                "{path}"
            );
        }
    }
}
