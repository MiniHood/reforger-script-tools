use crate::index::{GlobalSymbolId, SymbolIndex};
use crate::lexer::TextSpan;
use crate::model::SymbolKind;
use crate::syntax::{Parse, SyntaxElement, SyntaxKind, SyntaxNode};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LexicalScopeId(usize);

impl LexicalScopeId {
    pub const fn raw(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexicalScopeKind {
    Root,
    Callable,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalScope {
    pub id: LexicalScopeId,
    pub parent: Option<LexicalScopeId>,
    pub kind: LexicalScopeKind,
    pub span: TextSpan,
    pub owner: Option<GlobalSymbolId>,
    pub symbols: Vec<GlobalSymbolId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalScopeModel {
    scopes: Vec<LexicalScope>,
    symbol_scopes: BTreeMap<GlobalSymbolId, LexicalScopeId>,
}

impl LexicalScopeModel {
    pub fn from_parse_and_index(parse: &Parse, index: &SymbolIndex) -> Self {
        let mut model = Self {
            scopes: Vec::new(),
            symbol_scopes: BTreeMap::new(),
        };
        let root = model.push_scope(None, LexicalScopeKind::Root, parse.root.span, None);

        for callable in index.symbols().iter().filter_map(|symbol| {
            is_callable_symbol(symbol.kind).then_some((symbol.id, symbol.span))
        }) {
            let callable_scope = model.push_scope(
                Some(root),
                LexicalScopeKind::Callable,
                callable.1,
                Some(callable.0),
            );
            collect_block_scopes_for_callable(&parse.root, callable.1, callable_scope, &mut model);
        }

        for symbol in index.symbols() {
            match symbol.kind {
                SymbolKind::Parameter => {
                    if let Some(callable_scope) =
                        model.scope_for_owner(symbol.parent, LexicalScopeKind::Callable)
                    {
                        model.attach_symbol(callable_scope, symbol.id);
                    }
                }
                SymbolKind::LocalVariable => {
                    if let Some(callable_scope) =
                        model.scope_for_owner(symbol.parent, LexicalScopeKind::Callable)
                    {
                        let scope = model
                            .innermost_scope_under_at(callable_scope, symbol.selection_span.start)
                            .unwrap_or(callable_scope);
                        model.attach_symbol(scope, symbol.id);
                    }
                }
                _ => {}
            }
        }

        model
    }

    pub fn scope(&self, id: LexicalScopeId) -> Option<&LexicalScope> {
        self.scopes.get(id.0)
    }

    pub fn scopes(&self) -> &[LexicalScope] {
        &self.scopes
    }

    pub fn scope_for_symbol(&self, id: GlobalSymbolId) -> Option<LexicalScopeId> {
        self.symbol_scopes.get(&id).copied()
    }

    pub fn innermost_scope_at(&self, offset: usize) -> Option<LexicalScopeId> {
        self.scopes
            .iter()
            .filter(|scope| contains_offset(scope.span, offset))
            .min_by_key(|scope| (scope.span.len(), std::cmp::Reverse(scope.id.0)))
            .map(|scope| scope.id)
    }

    pub fn visible_symbols_named(
        &self,
        index: &SymbolIndex,
        name: &str,
        offset: usize,
    ) -> Vec<GlobalSymbolId> {
        let mut result = Vec::new();
        let mut current = self.innermost_scope_at(offset);
        while let Some(scope_id) = current {
            let Some(scope) = self.scope(scope_id) else {
                break;
            };
            let mut scoped = scope
                .symbols
                .iter()
                .filter_map(|id| {
                    let symbol = index.symbol(*id)?;
                    (symbol.name.as_deref() == Some(name)
                        && is_visible_at_offset(symbol.kind, symbol.selection_span, offset))
                    .then_some((*id, symbol.kind, symbol.selection_span.start))
                })
                .collect::<Vec<_>>();
            scoped.sort_by(|left, right| {
                scope_symbol_rank(left.1)
                    .cmp(&scope_symbol_rank(right.1))
                    .then_with(|| right.2.cmp(&left.2))
            });
            result.extend(scoped.into_iter().map(|(id, _, _)| id));
            current = scope.parent;
        }
        result
    }

    fn push_scope(
        &mut self,
        parent: Option<LexicalScopeId>,
        kind: LexicalScopeKind,
        span: TextSpan,
        owner: Option<GlobalSymbolId>,
    ) -> LexicalScopeId {
        let id = LexicalScopeId(self.scopes.len());
        self.scopes.push(LexicalScope {
            id,
            parent,
            kind,
            span,
            owner,
            symbols: Vec::new(),
        });
        id
    }

    fn attach_symbol(&mut self, scope: LexicalScopeId, symbol: GlobalSymbolId) {
        if let Some(scope) = self.scopes.get_mut(scope.0) {
            scope.symbols.push(symbol);
            self.symbol_scopes.insert(symbol, scope.id);
        }
    }

    fn scope_for_owner(
        &self,
        owner: Option<GlobalSymbolId>,
        kind: LexicalScopeKind,
    ) -> Option<LexicalScopeId> {
        self.scopes
            .iter()
            .find(|scope| scope.kind == kind && scope.owner == owner)
            .map(|scope| scope.id)
    }

    fn innermost_scope_under_at(
        &self,
        root: LexicalScopeId,
        offset: usize,
    ) -> Option<LexicalScopeId> {
        let root_span = self.scope(root)?.span;
        self.scopes
            .iter()
            .filter(|scope| {
                contains_offset(scope.span, offset) && span_contains(root_span, scope.span)
            })
            .min_by_key(|scope| (scope.span.len(), std::cmp::Reverse(scope.id.0)))
            .map(|scope| scope.id)
    }
}

fn collect_block_scopes_for_callable(
    node: &SyntaxNode,
    callable_span: TextSpan,
    callable_scope: LexicalScopeId,
    model: &mut LexicalScopeModel,
) {
    if !spans_overlap(callable_span, node.span) {
        return;
    }
    if node.kind == SyntaxKind::Block && span_contains(callable_span, node.span) {
        let parent = model
            .scopes
            .iter()
            .filter(|scope| {
                matches!(
                    scope.kind,
                    LexicalScopeKind::Callable | LexicalScopeKind::Block
                ) && span_contains(scope.span, node.span)
            })
            .min_by_key(|scope| (scope.span.len(), std::cmp::Reverse(scope.id.0)))
            .map(|scope| scope.id)
            .unwrap_or(callable_scope);
        model.push_scope(Some(parent), LexicalScopeKind::Block, node.span, None);
    }
    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_block_scopes_for_callable(child, callable_span, callable_scope, model);
        }
    }
}

fn is_callable_symbol(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Destructor
    )
}

fn is_visible_at_offset(kind: SymbolKind, span: TextSpan, offset: usize) -> bool {
    match kind {
        SymbolKind::LocalVariable => span.start <= offset,
        SymbolKind::Parameter => true,
        _ => false,
    }
}

fn scope_symbol_rank(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::LocalVariable => 0,
        SymbolKind::Parameter => 1,
        _ => 9,
    }
}

fn contains_offset(span: TextSpan, offset: usize) -> bool {
    if span.is_empty() {
        span.start == offset
    } else {
        span.start <= offset && offset < span.end
    }
}

fn span_contains(outer: TextSpan, inner: TextSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn spans_overlap(left: TextSpan, right: TextSpan) -> bool {
    left.start < right.end && right.start < left.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::model::{SourceFileMetadata, SourceKind, SymbolCatalog};
    use crate::parser::parse_source;

    fn index_for(source: &str) -> (Parse, SymbolIndex) {
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let catalog = SymbolCatalog::from_ast_with_metadata(
            source,
            &ast,
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                category: crate::model::SourceCategory::Workspace,
                absolute_path: None,
                root_path: None,
                relative_path: None,
                priority: crate::model::SOURCE_PRIORITY_WORKSPACE,
            },
        );
        let mut index = SymbolIndex::default();
        index.add_catalog(&catalog);
        (parse, index)
    }

    #[test]
    fn builds_callable_and_block_scopes_with_locals_and_parameters() {
        let source = r#"class Example
{
	void Run(int value)
	{
		int outerValue;
		if (value)
		{
			int innerValue;
			innerValue;
		}
	}
}
"#;
        let (parse, index) = index_for(source);
        let scopes = LexicalScopeModel::from_parse_and_index(&parse, &index);

        assert!(scopes
            .scopes()
            .iter()
            .any(|scope| scope.kind == LexicalScopeKind::Callable));
        assert!(
            scopes
                .scopes()
                .iter()
                .filter(|scope| scope.kind == LexicalScopeKind::Block)
                .count()
                >= 2
        );

        let inner_use = source.rfind("innerValue;").unwrap();
        let visible = scopes.visible_symbols_named(&index, "innerValue", inner_use);
        assert_eq!(visible.len(), 1);
        assert_eq!(
            index.symbol(visible[0]).unwrap().kind,
            SymbolKind::LocalVariable
        );
    }

    #[test]
    fn inner_scope_local_shadows_outer_local_before_parameters() {
        let source = r#"class Example
{
	void Run(int value)
	{
		int value;
		{
			string value;
			value;
		}
	}
}
"#;
        let (parse, index) = index_for(source);
        let scopes = LexicalScopeModel::from_parse_and_index(&parse, &index);
        let use_offset = source.rfind("value;").unwrap();
        let visible = scopes.visible_symbols_named(&index, "value", use_offset);

        assert!(visible.len() >= 3);
        let first = index.symbol(visible[0]).unwrap();
        assert_eq!(first.kind, SymbolKind::LocalVariable);
        assert_eq!(first.detail.type_text.as_deref(), Some("string"));
    }
}
