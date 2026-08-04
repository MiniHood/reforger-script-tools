use crate::index::{GlobalSymbolId, IndexedFile, IndexedSymbol, SymbolIndex};
use crate::model::SymbolKind;

#[derive(Debug, Clone, Copy)]
pub struct TypeFacts<'index> {
    index: &'index SymbolIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolTypeFacts<'index> {
    pub id: GlobalSymbolId,
    pub kind: SymbolKind,
    pub name: Option<&'index str>,
    pub containing_type_name: Option<&'index str>,
    pub type_text: Option<&'index str>,
    pub return_type_text: Option<&'index str>,
    pub base_type: Option<&'index str>,
    pub default_text: Option<&'index str>,
    pub enum_value_text: Option<&'index str>,
}

impl<'index> TypeFacts<'index> {
    pub const fn new(index: &'index SymbolIndex) -> Self {
        Self { index }
    }

    pub fn facts_for_symbol(&self, id: GlobalSymbolId) -> Option<SymbolTypeFacts<'index>> {
        let symbol = self.index.symbol(id)?;
        Some(SymbolTypeFacts {
            id,
            kind: symbol.kind,
            name: symbol.name.as_deref(),
            containing_type_name: self.containing_type_name(id),
            type_text: symbol.detail.type_text.as_deref(),
            return_type_text: symbol.detail.return_type_text.as_deref(),
            base_type: symbol.detail.base_type.as_deref(),
            default_text: symbol.detail.default_text.as_deref(),
            enum_value_text: symbol.detail.enum_value_text.as_deref(),
        })
    }

    pub fn file_for_symbol(&self, id: GlobalSymbolId) -> Option<&'index IndexedFile> {
        self.index.file(id.file_id)
    }

    pub fn symbol(&self, id: GlobalSymbolId) -> Option<&'index IndexedSymbol> {
        self.index.symbol(id)
    }

    pub fn value_type_text(&self, id: GlobalSymbolId) -> Option<&'index str> {
        let symbol = self.index.symbol(id)?;
        if matches!(
            symbol.kind,
            SymbolKind::Field
                | SymbolKind::GlobalField
                | SymbolKind::Parameter
                | SymbolKind::LocalVariable
        ) {
            symbol.detail.type_text.as_deref()
        } else {
            None
        }
    }

    pub fn typedef_target_text(&self, id: GlobalSymbolId) -> Option<&'index str> {
        let symbol = self.index.symbol(id)?;
        if symbol.kind == SymbolKind::Typedef {
            symbol.detail.type_text.as_deref()
        } else {
            None
        }
    }

    pub fn callable_return_type_text(&self, id: GlobalSymbolId) -> Option<&'index str> {
        let symbol = self.index.symbol(id)?;
        if matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
            symbol.detail.return_type_text.as_deref()
        } else {
            None
        }
    }

    pub fn class_base_type(&self, id: GlobalSymbolId) -> Option<&'index str> {
        let symbol = self.index.symbol(id)?;
        if matches!(symbol.kind, SymbolKind::Class | SymbolKind::Enum) {
            symbol.detail.base_type.as_deref()
        } else {
            None
        }
    }

    /// Returns the class owner targeted by `super`. A normal class targets its
    /// declared base, while an implicit `modded class` layer targets the
    /// original same-named class that it replaces.
    pub fn class_super_type(&self, id: GlobalSymbolId) -> Option<&'index str> {
        let symbol = self.index.symbol(id)?;
        if symbol.kind != SymbolKind::Class {
            return None;
        }
        symbol.detail.base_type.as_deref().or_else(|| {
            self.is_implicit_modded_class(id)
                .then_some(symbol.name.as_deref())
                .flatten()
        })
    }

    pub fn is_implicit_modded_class(&self, id: GlobalSymbolId) -> bool {
        self.index.symbol(id).is_some_and(|symbol| {
            symbol.kind == SymbolKind::Class
                && symbol.detail.base_type.is_none()
                && symbol.modifiers.iter().any(|modifier| modifier == "modded")
                && symbol.name.as_deref().is_some_and(|name| !name.is_empty())
        })
    }

    pub fn enum_member_value_text(&self, id: GlobalSymbolId) -> Option<&'index str> {
        let symbol = self.index.symbol(id)?;
        if symbol.kind == SymbolKind::EnumMember {
            symbol.detail.enum_value_text.as_deref()
        } else {
            None
        }
    }

    pub fn containing_type_name(&self, id: GlobalSymbolId) -> Option<&'index str> {
        let mut current = self.index.symbol(id)?.parent;
        while let Some(parent_id) = current {
            let parent = self.index.symbol(parent_id)?;
            if parent.kind == SymbolKind::Class {
                return parent.name.as_deref();
            }
            current = parent.parent;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SourceFileMetadata;
    use crate::parser::parse_source;
    use crate::semantic_file::SemanticFile;

    fn index_for_source(source: &str) -> SymbolIndex {
        let parse = parse_source(source);
        let semantic_file = SemanticFile::build(source, &parse);
        SymbolIndex::from_semantic_files([(&semantic_file, SourceFileMetadata::unknown())])
    }

    fn only_named(index: &SymbolIndex, name: &str) -> GlobalSymbolId {
        let ids = index.symbols_for_name(name);
        assert_eq!(ids.len(), 1, "expected exactly one symbol named {name}");
        ids[0]
    }

    fn named_kind(index: &SymbolIndex, name: &str, kind: SymbolKind) -> GlobalSymbolId {
        *index
            .symbols_for_name(name)
            .iter()
            .find(|id| index.symbol(**id).is_some_and(|symbol| symbol.kind == kind))
            .unwrap_or_else(|| panic!("expected {kind:?} named {name}"))
    }

    #[test]
    fn exposes_source_backed_type_facts_for_declarations() {
        let source = r#"
typedef string FactionKey;
enum EExample : int
{
    One = 1
}

int s_GlobalCount;
void GlobalFn(out int value = 4) {}

class Example : Base
{
    protected ref Widget m_Widget;
    void Run(string name, int count = 2)
    {
        vector localValue = "0 0 0";
    }
}
"#;
        let index = index_for_source(source);
        let facts = TypeFacts::new(&index);

        let typedef = only_named(&index, "FactionKey");
        assert_eq!(facts.typedef_target_text(typedef), Some("string"));
        assert_eq!(
            facts.facts_for_symbol(typedef).unwrap().type_text,
            Some("string")
        );

        let class = only_named(&index, "Example");
        assert_eq!(facts.class_base_type(class), Some("Base"));
        assert_eq!(facts.class_super_type(class), Some("Base"));

        let enum_member = only_named(&index, "One");
        assert_eq!(facts.enum_member_value_text(enum_member), Some("1"));

        let global = only_named(&index, "s_GlobalCount");
        assert_eq!(facts.value_type_text(global), Some("int"));

        let function = only_named(&index, "GlobalFn");
        assert_eq!(facts.callable_return_type_text(function), Some("void"));

        let field = only_named(&index, "m_Widget");
        assert_eq!(facts.value_type_text(field), Some("ref Widget"));
        assert_eq!(facts.containing_type_name(field), Some("Example"));

        let method = only_named(&index, "Run");
        assert_eq!(facts.callable_return_type_text(method), Some("void"));
        assert_eq!(facts.containing_type_name(method), Some("Example"));

        let parameter = named_kind(&index, "count", SymbolKind::Parameter);
        let parameter_facts = facts.facts_for_symbol(parameter).unwrap();
        assert_eq!(parameter_facts.type_text, Some("int"));
        assert_eq!(parameter_facts.default_text, Some("2"));
        assert_eq!(parameter_facts.containing_type_name, Some("Example"));

        let local = only_named(&index, "localValue");
        assert_eq!(facts.value_type_text(local), Some("vector"));
        assert_eq!(facts.containing_type_name(local), Some("Example"));
    }

    #[test]
    fn ignores_inapplicable_type_fact_queries() {
        let index = index_for_source("class Example { void Run() {} }");
        let facts = TypeFacts::new(&index);
        let class = only_named(&index, "Example");
        let method = only_named(&index, "Run");

        assert_eq!(facts.value_type_text(class), None);
        assert_eq!(facts.typedef_target_text(class), None);
        assert_eq!(facts.enum_member_value_text(method), None);
        assert_eq!(facts.containing_type_name(class), None);
    }

    #[test]
    fn exposes_the_original_same_named_class_as_an_implicit_modded_super_type() {
        let index = index_for_source("modded class Example {}");
        let class = only_named(&index, "Example");
        let facts = TypeFacts::new(&index);

        assert_eq!(facts.class_base_type(class), None);
        assert!(facts.is_implicit_modded_class(class));
        assert_eq!(facts.class_super_type(class), Some("Example"));
    }
}
