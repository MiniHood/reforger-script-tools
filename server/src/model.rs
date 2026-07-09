use crate::ast::{
    AstSourceFile, ClassMember, Declaration, DocComment, DocCommentKind, MethodDecl, MethodKind,
    TextValue,
};
use crate::lexer::TextSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SymbolKind {
    Class,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolDetail {
    pub type_text: Option<TextSpan>,
    pub return_type_text: Option<TextSpan>,
    pub base_type: Option<TextSpan>,
    pub default_text: Option<TextSpan>,
    pub enum_value_text: Option<TextSpan>,
}

impl SymbolDetail {
    pub const fn empty() -> Self {
        Self {
            type_text: None,
            return_type_text: None,
            base_type: None,
            default_text: None,
            enum_value_text: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocCommentRecord {
    pub span: TextSpan,
    pub kind: DocCommentKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRecord {
    pub id: SymbolId,
    pub parent: Option<SymbolId>,
    pub kind: SymbolKind,
    pub name: Option<TextSpan>,
    pub span: TextSpan,
    pub selection_span: TextSpan,
    pub detail: SymbolDetail,
    pub attributes: Vec<TextSpan>,
    pub modifiers: Vec<TextSpan>,
    pub doc_comments: Vec<DocCommentRecord>,
}

pub struct SymbolCatalog<'source> {
    source: &'source str,
    records: Vec<SymbolRecord>,
    non_declaration_callable_fragments: usize,
}

impl<'source> SymbolCatalog<'source> {
    pub fn from_ast(source: &'source str, ast: &AstSourceFile<'source, '_>) -> Self {
        let mut builder = SymbolCatalogBuilder {
            source,
            records: Vec::new(),
            non_declaration_callable_fragments: 0,
        };
        builder.add_ast(ast);
        builder.finish()
    }

    pub const fn source(&self) -> &'source str {
        self.source
    }

    pub fn records(&self) -> &[SymbolRecord] {
        &self.records
    }

    pub fn record(&self, id: SymbolId) -> Option<&SymbolRecord> {
        self.records.get(id.0)
    }

    pub fn text(&self, span: TextSpan) -> &'source str {
        &self.source[span.start..span.end]
    }

    pub fn record_name(&self, record: &SymbolRecord) -> Option<&'source str> {
        record.name.map(|span| self.text(span))
    }

    pub fn attribute_name(&self, span: TextSpan) -> Option<&'source str> {
        attribute_name(self.text(span))
    }

    pub fn record_attribute_names(&self, record: &SymbolRecord) -> Vec<&'source str> {
        record
            .attributes
            .iter()
            .filter_map(|span| self.attribute_name(*span))
            .collect()
    }

    pub const fn non_declaration_callable_fragments(&self) -> usize {
        self.non_declaration_callable_fragments
    }
}

struct SymbolCatalogBuilder<'source> {
    source: &'source str,
    records: Vec<SymbolRecord>,
    non_declaration_callable_fragments: usize,
}

impl<'source> SymbolCatalogBuilder<'source> {
    fn add_ast(&mut self, ast: &AstSourceFile<'source, '_>) {
        for declaration in ast.declarations() {
            self.add_declaration(declaration);
        }
    }

    fn finish(self) -> SymbolCatalog<'source> {
        SymbolCatalog {
            source: self.source,
            records: self.records,
            non_declaration_callable_fragments: self.non_declaration_callable_fragments,
        }
    }

    fn add_declaration(&mut self, declaration: Declaration<'source, '_>) {
        match declaration {
            Declaration::Class(class) => {
                let class_id = self.push_record(NewSymbol {
                    parent: None,
                    kind: SymbolKind::Class,
                    name: class.name(),
                    span: class.span(),
                    detail: SymbolDetail {
                        base_type: class.base_type().map(|value| value.span),
                        ..SymbolDetail::empty()
                    },
                    attributes: spans(class.attributes()),
                    modifiers: text_spans(class.modifiers()),
                    doc_comments: doc_comment_records(class.doc_comments()),
                });

                for member in class.members() {
                    match member {
                        ClassMember::Field(field) => {
                            self.push_record(NewSymbol {
                                parent: Some(class_id),
                                kind: SymbolKind::Field,
                                name: field.name(),
                                span: field.span(),
                                detail: SymbolDetail {
                                    type_text: field.type_text().map(|value| value.span),
                                    ..SymbolDetail::empty()
                                },
                                attributes: spans(field.attributes()),
                                modifiers: text_spans(field.modifiers()),
                                doc_comments: doc_comment_records(field.doc_comments()),
                            });
                        }
                        ClassMember::Method(method) => {
                            let method_kind = match class.classify_method(method) {
                                MethodKind::Method => SymbolKind::Method,
                                MethodKind::Constructor => SymbolKind::Constructor,
                                MethodKind::Destructor => SymbolKind::Destructor,
                            };
                            self.add_callable(Some(class_id), method_kind, method);
                        }
                        ClassMember::Empty(_) => {}
                    }
                }
            }
            Declaration::Enum(enum_decl) => {
                let enum_id = self.push_record(NewSymbol {
                    parent: None,
                    kind: SymbolKind::Enum,
                    name: enum_decl.name(),
                    span: enum_decl.span(),
                    detail: SymbolDetail::empty(),
                    attributes: spans(enum_decl.attributes()),
                    modifiers: Vec::new(),
                    doc_comments: doc_comment_records(enum_decl.doc_comments()),
                });

                for member in enum_decl.members() {
                    self.push_record(NewSymbol {
                        parent: Some(enum_id),
                        kind: SymbolKind::EnumMember,
                        name: member.name(),
                        span: member.span(),
                        detail: SymbolDetail {
                            enum_value_text: member.value_text().map(|value| value.span),
                            ..SymbolDetail::empty()
                        },
                        attributes: Vec::new(),
                        modifiers: Vec::new(),
                        doc_comments: Vec::new(),
                    });
                }
            }
            Declaration::Typedef(typedef_decl) => {
                self.push_record(NewSymbol {
                    parent: None,
                    kind: SymbolKind::Typedef,
                    name: typedef_decl.name(),
                    span: typedef_decl.text_span(),
                    detail: SymbolDetail {
                        type_text: typedef_decl.type_text().map(|value| value.span),
                        ..SymbolDetail::empty()
                    },
                    attributes: Vec::new(),
                    modifiers: Vec::new(),
                    doc_comments: doc_comment_records(typedef_decl.doc_comments()),
                });
            }
            Declaration::Function(function) => {
                self.add_callable(None, SymbolKind::Function, function);
            }
            Declaration::Field(field) => {
                self.push_record(NewSymbol {
                    parent: None,
                    kind: SymbolKind::GlobalField,
                    name: field.name(),
                    span: field.span(),
                    detail: SymbolDetail {
                        type_text: field.type_text().map(|value| value.span),
                        ..SymbolDetail::empty()
                    },
                    attributes: spans(field.attributes()),
                    modifiers: text_spans(field.modifiers()),
                    doc_comments: doc_comment_records(field.doc_comments()),
                });
            }
        }
    }

    fn add_callable(
        &mut self,
        parent: Option<SymbolId>,
        kind: SymbolKind,
        method: MethodDecl<'source, '_>,
    ) {
        let callable_id = self.push_record(NewSymbol {
            parent,
            kind,
            name: method.name(),
            span: method.span(),
            detail: SymbolDetail {
                return_type_text: method.return_type_text().map(|value| value.span),
                ..SymbolDetail::empty()
            },
            attributes: spans(method.attributes()),
            modifiers: text_spans(method.modifiers()),
            doc_comments: doc_comment_records(method.doc_comments()),
        });

        for parameter in method.parameters() {
            self.push_record(NewSymbol {
                parent: Some(callable_id),
                kind: SymbolKind::Parameter,
                name: parameter.name(),
                span: parameter.span(),
                detail: SymbolDetail {
                    type_text: parameter.type_text().map(|value| value.span),
                    default_text: parameter.default_text().map(|value| value.span),
                    ..SymbolDetail::empty()
                },
                attributes: Vec::new(),
                modifiers: text_spans(parameter.modifiers()),
                doc_comments: Vec::new(),
            });
        }

        self.non_declaration_callable_fragments += method.parameter_fragments().len();
    }

    fn push_record(&mut self, symbol: NewSymbol<'source>) -> SymbolId {
        let id = SymbolId(self.records.len());
        let name = symbol.name.map(|value| value.span);
        let selection_span = name.unwrap_or(symbol.span);
        self.records.push(SymbolRecord {
            id,
            parent: symbol.parent,
            kind: symbol.kind,
            name,
            span: symbol.span,
            selection_span,
            detail: symbol.detail,
            attributes: symbol.attributes,
            modifiers: symbol.modifiers,
            doc_comments: symbol.doc_comments,
        });
        id
    }
}

struct NewSymbol<'source> {
    parent: Option<SymbolId>,
    kind: SymbolKind,
    name: Option<TextValue<'source>>,
    span: TextSpan,
    detail: SymbolDetail,
    attributes: Vec<TextSpan>,
    modifiers: Vec<TextSpan>,
    doc_comments: Vec<DocCommentRecord>,
}

fn spans(attributes: Vec<crate::ast::Attribute<'_, '_>>) -> Vec<TextSpan> {
    attributes
        .into_iter()
        .map(|attribute| attribute.span())
        .collect()
}

fn text_spans(values: Vec<TextValue<'_>>) -> Vec<TextSpan> {
    values.into_iter().map(|value| value.span).collect()
}

fn doc_comment_records(comments: Vec<DocComment<'_>>) -> Vec<DocCommentRecord> {
    comments
        .into_iter()
        .map(|comment| DocCommentRecord {
            span: comment.span(),
            kind: comment.kind(),
        })
        .collect()
}

fn attribute_name(text: &str) -> Option<&str> {
    let trimmed = text.trim_start();
    let trimmed = trimmed.strip_prefix('[').unwrap_or(trimmed).trim_start();
    let mut end = 0usize;
    for (index, value) in trimmed.char_indices() {
        if value.is_ascii_alphanumeric() || value == '_' {
            end = index + value.len_utf8();
        } else {
            break;
        }
    }

    if end == 0 {
        None
    } else {
        Some(&trimmed[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;
    use std::collections::BTreeMap;

    #[test]
    fn catalogs_top_level_declarations() {
        let source = r#"//! Global docs
Game g_Game;

[EnumBitFlag()]
enum EFlags
{
	None,
	One = 1,
}

typedef int ExampleAlias;

void GlobalFn(int value = 4);

class Example : Base
{
}
"#;
        let catalog = catalog(source);

        assert_eq!(count_kind(&catalog, SymbolKind::GlobalField), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::Enum), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::EnumMember), 2);
        assert_eq!(count_kind(&catalog, SymbolKind::Typedef), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::Function), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::Class), 1);

        let global = find(&catalog, SymbolKind::GlobalField, "g_Game");
        assert_eq!(catalog.text(global.detail.type_text.unwrap()), "Game");
        assert_eq!(global.doc_comments.len(), 1);

        let enum_symbol = find(&catalog, SymbolKind::Enum, "EFlags");
        assert_eq!(enum_symbol.attributes.len(), 1);
        assert_eq!(
            catalog.attribute_name(enum_symbol.attributes[0]),
            Some("EnumBitFlag")
        );
        assert_eq!(
            catalog.record_attribute_names(enum_symbol),
            vec!["EnumBitFlag"]
        );

        let enum_member = find(&catalog, SymbolKind::EnumMember, "One");
        assert_eq!(enum_member.parent, Some(enum_symbol.id));
        assert_eq!(
            catalog.text(enum_member.detail.enum_value_text.unwrap()),
            "1"
        );

        let typedef = find(&catalog, SymbolKind::Typedef, "ExampleAlias");
        assert_eq!(catalog.text(typedef.detail.type_text.unwrap()), "int");

        let class = find(&catalog, SymbolKind::Class, "Example");
        assert_eq!(catalog.text(class.detail.base_type.unwrap()), "Base");

        let function = find(&catalog, SymbolKind::Function, "GlobalFn");
        assert_eq!(
            catalog.text(function.detail.return_type_text.unwrap()),
            "void"
        );
        assert_eq!(child_count(&catalog, function.id, SymbolKind::Parameter), 1);
    }

    #[test]
    fn catalogs_class_members_and_parameters_with_parent_links() {
        let source = r#"class Example
{
	protected ref array<int> m_aValues;
	void Example(int value) {}
	void ~Example() {}
	static bool Run(out notnull array<ref SCR_Value> values);
}
"#;
        let catalog = catalog(source);
        let class = find(&catalog, SymbolKind::Class, "Example");
        let field = find(&catalog, SymbolKind::Field, "m_aValues");
        let constructor = find(&catalog, SymbolKind::Constructor, "Example");
        let destructor = find(&catalog, SymbolKind::Destructor, "Example");
        let method = find(&catalog, SymbolKind::Method, "Run");
        let parameter = find(&catalog, SymbolKind::Parameter, "values");

        assert_eq!(field.parent, Some(class.id));
        assert_eq!(constructor.parent, Some(class.id));
        assert_eq!(destructor.parent, Some(class.id));
        assert_eq!(method.parent, Some(class.id));
        assert_eq!(parameter.parent, Some(method.id));

        assert_eq!(
            catalog.text(field.detail.type_text.unwrap()),
            "ref array<int>"
        );
        assert_eq!(
            catalog.text(method.detail.return_type_text.unwrap()),
            "bool"
        );
        assert_eq!(
            catalog.text(parameter.detail.type_text.unwrap()),
            "array<ref SCR_Value>"
        );
        assert_eq!(
            parameter
                .modifiers
                .iter()
                .map(|span| catalog.text(*span))
                .collect::<Vec<_>>(),
            vec!["out", "notnull"]
        );
    }

    #[test]
    fn excludes_non_declaration_callable_fragments() {
        let source = r#"class Example
{
	void FilterOutStorages(false);
	void Real(bool enabled = true);
}
"#;
        let catalog = catalog(source);

        assert_eq!(catalog.non_declaration_callable_fragments(), 1);
        assert_eq!(count_kind(&catalog, SymbolKind::Parameter), 1);
        let parameter = find(&catalog, SymbolKind::Parameter, "enabled");
        assert_eq!(catalog.text(parameter.detail.type_text.unwrap()), "bool");
        assert_eq!(catalog.text(parameter.detail.default_text.unwrap()), "true");
    }

    #[test]
    fn catalogs_typedef_aliased_type_text() {
        let source = r#"typedef string FactionKey;
typedef func Callback;
typedef map<ref Managed, ref Managed> TManagedMap;
"#;
        let catalog = catalog(source);

        let faction_key = find(&catalog, SymbolKind::Typedef, "FactionKey");
        assert_eq!(
            catalog.text(faction_key.detail.type_text.unwrap()),
            "string"
        );

        let callback = find(&catalog, SymbolKind::Typedef, "Callback");
        assert_eq!(catalog.text(callback.detail.type_text.unwrap()), "func");

        let map = find(&catalog, SymbolKind::Typedef, "TManagedMap");
        assert_eq!(
            catalog.text(map.detail.type_text.unwrap()),
            "map<ref Managed, ref Managed>"
        );
    }

    #[test]
    fn committed_parser_fixtures_catalog_without_missing_names() {
        let fixtures = [
            include_str!("../../tools/fixtures/parser/core_types_declarations.c"),
            include_str!("../../tools/fixtures/parser/attributes_rpc_workbench.c"),
            include_str!("../../tools/fixtures/parser/modded_game_mode_members.c"),
            include_str!("../../tools/fixtures/parser/preprocessor_directives.c"),
            include_str!("../../tools/fixtures/parser/game_building_network_component.c"),
            include_str!("../../tools/fixtures/parser/game_building_provider_excerpt.c"),
            include_str!("../../tools/fixtures/parser/game_editable_group_excerpt.c"),
            include_str!("../../tools/fixtures/parser/game_editor_preview_params.c"),
            include_str!("../../tools/fixtures/parser/workbench_basic_code_formatter_excerpt.c"),
            include_str!("../../tools/fixtures/parser/game_optional_semicolon_shapes.c"),
            include_str!("../../tools/fixtures/parser/game_field_initializer_call_shapes.c"),
        ];

        for fixture in fixtures {
            let catalog = catalog(fixture);
            assert!(!catalog.records().is_empty());
            assert!(
                catalog.records().iter().all(|record| record.name.is_some()),
                "fixture had missing symbol names"
            );
        }
    }

    fn catalog(source: &str) -> SymbolCatalog<'_> {
        let parse = parse_source(source);
        assert!(parse.diagnostics.is_empty(), "{:?}", parse.diagnostics);
        let ast = AstSourceFile::new(source, &parse);
        SymbolCatalog::from_ast(source, &ast)
    }

    fn count_kind(catalog: &SymbolCatalog<'_>, kind: SymbolKind) -> usize {
        catalog
            .records()
            .iter()
            .filter(|record| record.kind == kind)
            .count()
    }

    fn child_count(catalog: &SymbolCatalog<'_>, parent: SymbolId, kind: SymbolKind) -> usize {
        catalog
            .records()
            .iter()
            .filter(|record| record.parent == Some(parent) && record.kind == kind)
            .count()
    }

    fn find<'a>(catalog: &'a SymbolCatalog<'_>, kind: SymbolKind, name: &str) -> &'a SymbolRecord {
        catalog
            .records()
            .iter()
            .find(|record| {
                record.kind == kind && record.name.is_some_and(|span| catalog.text(span) == name)
            })
            .unwrap_or_else(|| panic!("missing {kind:?} {name}; got {:?}", names(catalog)))
    }

    fn names(catalog: &SymbolCatalog<'_>) -> BTreeMap<String, SymbolKind> {
        catalog
            .records()
            .iter()
            .filter_map(|record| {
                record
                    .name
                    .map(|span| (catalog.text(span).to_string(), record.kind))
            })
            .collect()
    }
}
