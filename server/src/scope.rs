use crate::index::{GlobalSymbolId, SourceFileId, SymbolIndex};
use crate::lexer::TextSpan;
use crate::model::SymbolKind;
use crate::semantic_file::{SemanticDeclarationKind, SemanticFile};
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
    ForLoop,
    ForeachLoop,
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
    declaration_scopes: Vec<(TextSpan, LexicalScopeId)>,
}

impl LexicalScopeModel {
    /// Builds lexical regions from parser CST while taking callable and local
    /// binding facts exclusively from the compiler semantic file. `SymbolIndex`
    /// supplies stable lookup identities for feature adapters only; it is not
    /// consulted to discover declarations.
    pub fn from_parse_and_semantics(
        parse: &Parse,
        semantic_file: &SemanticFile,
        index: &SymbolIndex,
        source_file_id: SourceFileId,
    ) -> Self {
        let mut model = Self {
            scopes: Vec::new(),
            symbol_scopes: BTreeMap::new(),
            declaration_scopes: Vec::new(),
        };
        let root = model.push_scope(None, LexicalScopeKind::Root, parse.root.span, None);
        let mut callable_scopes = BTreeMap::new();
        let mut callable_nodes = Vec::new();

        for declaration in semantic_file.declarations().iter().filter(|declaration| {
            matches!(
                declaration.kind,
                SemanticDeclarationKind::Function
                    | SemanticDeclarationKind::Method
                    | SemanticDeclarationKind::Constructor
                    | SemanticDeclarationKind::Destructor
            )
        }) {
            let id = GlobalSymbolId {
                file_id: source_file_id,
                symbol_id: crate::model::SymbolId(declaration.id.0 as usize),
            };
            // Fail closed if the derived index ever drops a semantic record;
            // do not recreate facts from the index.
            if index.symbol(id).is_none() {
                continue;
            }
            let callable_scope = model.push_scope(
                Some(root),
                LexicalScopeKind::Callable,
                declaration.span,
                Some(id),
            );
            callable_scopes.insert(declaration.id, callable_scope);
            callable_nodes.push((declaration.span, callable_scope));
        }

        collect_scopes_once(&parse.root, None, &callable_nodes, &mut model);

        for declaration in semantic_file.declarations() {
            let kind = match declaration.kind {
                SemanticDeclarationKind::Parameter => SymbolKind::Parameter,
                SemanticDeclarationKind::LocalVariable => SymbolKind::LocalVariable,
                _ => continue,
            };
            let Some(parent) = declaration.parent else {
                continue;
            };
            let Some(callable_scope) = callable_scopes.get(&parent).copied() else {
                continue;
            };
            let id = GlobalSymbolId {
                file_id: source_file_id,
                symbol_id: crate::model::SymbolId(declaration.id.0 as usize),
            };
            if index.symbol(id).is_none() {
                continue;
            }
            let scope = match kind {
                SymbolKind::Parameter => callable_scope,
                SymbolKind::LocalVariable => model
                    .declaration_scope_at(declaration.selection_span)
                    .or_else(|| {
                        model.innermost_scope_under_at(
                            callable_scope,
                            declaration.selection_span.start,
                        )
                    })
                    .unwrap_or(callable_scope),
                _ => unreachable!(),
            };
            model.attach_symbol(scope, id);
        }

        model
    }

    pub fn from_parse_and_index(parse: &Parse, index: &SymbolIndex) -> Self {
        let mut model = Self {
            scopes: Vec::new(),
            symbol_scopes: BTreeMap::new(),
            declaration_scopes: Vec::new(),
        };
        let root = model.push_scope(None, LexicalScopeKind::Root, parse.root.span, None);
        let mut callable_scopes = BTreeMap::new();
        let mut callable_nodes = Vec::new();

        for callable in index
            .symbols()
            .iter()
            .filter(|symbol| is_callable_symbol(symbol.kind))
        {
            let callable_scope = model.push_scope(
                Some(root),
                LexicalScopeKind::Callable,
                callable.span,
                Some(callable.id),
            );
            callable_scopes.insert(callable.id, callable_scope);
            callable_nodes.push((callable.span, callable_scope));
        }

        collect_scopes_once(&parse.root, None, &callable_nodes, &mut model);

        for symbol in index.symbols() {
            match symbol.kind {
                SymbolKind::Parameter => {
                    if let Some(callable_scope) = symbol
                        .parent
                        .and_then(|parent| callable_scopes.get(&parent).copied())
                    {
                        model.attach_symbol(callable_scope, symbol.id);
                    }
                }
                SymbolKind::LocalVariable => {
                    if let Some(callable_scope) = symbol
                        .parent
                        .and_then(|parent| callable_scopes.get(&parent).copied())
                    {
                        let scope = model
                            .declaration_scope_at(symbol.selection_span)
                            .or_else(|| {
                                model.innermost_scope_under_at(
                                    callable_scope,
                                    symbol.selection_span.start,
                                )
                            })
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

    /// True when `offset` is inside a callable's lexical region.  Foreground
    /// local completion uses this to avoid treating file/class scope as a
    /// local-scope query.
    pub fn has_callable_scope_at(&self, offset: usize) -> bool {
        let mut current = self.innermost_scope_at(offset);
        while let Some(scope_id) = current {
            let Some(scope) = self.scope(scope_id) else {
                return false;
            };
            if scope.kind == LexicalScopeKind::Callable {
                return true;
            }
            current = scope.parent;
        }
        false
    }

    pub fn visible_symbols_named(
        &self,
        index: &SymbolIndex,
        name: &str,
        offset: usize,
    ) -> Vec<GlobalSymbolId> {
        self.visible_symbols_matching(index, offset, |symbol_name| symbol_name == name)
    }

    pub fn visible_symbols_with_prefix(
        &self,
        index: &SymbolIndex,
        prefix: &str,
        offset: usize,
    ) -> Vec<GlobalSymbolId> {
        self.visible_symbols_matching(index, offset, |symbol_name| {
            starts_with_ignore_ascii_case(symbol_name, prefix)
        })
    }

    fn visible_symbols_matching(
        &self,
        index: &SymbolIndex,
        offset: usize,
        mut matches_name: impl FnMut(&str) -> bool,
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
                    let symbol_name = symbol.name.as_deref()?;
                    (matches_name(symbol_name)
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

    fn declaration_scope_at(&self, span: TextSpan) -> Option<LexicalScopeId> {
        self.declaration_scopes
            .iter()
            .filter(|(declaration_span, _)| span_contains(*declaration_span, span))
            .min_by_key(|(declaration_span, _)| declaration_span.len())
            .map(|(_, scope)| *scope)
    }
}

fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn collect_scopes_once(
    node: &SyntaxNode,
    current_scope: Option<LexicalScopeId>,
    callable_nodes: &[(TextSpan, LexicalScopeId)],
    model: &mut LexicalScopeModel,
) {
    let current_scope = match node.kind {
        SyntaxKind::FunctionDecl | SyntaxKind::MethodDecl => callable_nodes
            .iter()
            .find_map(|(span, scope)| (*span == node.span).then_some(*scope)),
        _ => current_scope,
    };

    let Some(current_scope) = current_scope else {
        for child in &node.children {
            if let SyntaxElement::Node(child) = child {
                collect_scopes_once(child, None, callable_nodes, model);
            }
        }
        return;
    };

    match node.kind {
        SyntaxKind::ForStatement => {
            if let Some(initializer) = direct_child(node, SyntaxKind::ForHeader)
                .and_then(|header| direct_child(header, SyntaxKind::ForInitializer))
                .filter(|initializer| has_direct_child(initializer, SyntaxKind::LocalDeclStatement))
            {
                let scope = model.push_scope(
                    Some(current_scope),
                    LexicalScopeKind::ForLoop,
                    node.span,
                    None,
                );
                model.declaration_scopes.push((initializer.span, scope));
                for child in &node.children {
                    if let SyntaxElement::Node(child) = child {
                        collect_scopes_once(child, Some(scope), callable_nodes, model);
                    }
                }
                return;
            }
        }
        SyntaxKind::ForeachStatement => {
            if let (Some(variables), Some(body)) = (
                direct_child(node, SyntaxKind::ForeachHeader)
                    .and_then(|header| direct_child(header, SyntaxKind::ForeachVariableList)),
                statement_body(node, SyntaxKind::ForeachHeader),
            ) {
                let scope = model.push_scope(
                    Some(current_scope),
                    LexicalScopeKind::ForeachLoop,
                    body.span,
                    None,
                );
                model.declaration_scopes.push((variables.span, scope));
                for child in &node.children {
                    if let SyntaxElement::Node(child) = child {
                        let child_scope = (child.span == body.span).then_some(scope);
                        collect_scopes_once(
                            child,
                            child_scope.or(Some(current_scope)),
                            callable_nodes,
                            model,
                        );
                    }
                }
                return;
            }
        }
        SyntaxKind::Block => {
            let scope = model.push_scope(
                Some(current_scope),
                LexicalScopeKind::Block,
                node.span,
                None,
            );
            for child in &node.children {
                if let SyntaxElement::Node(child) = child {
                    collect_scopes_once(child, Some(scope), callable_nodes, model);
                }
            }
            return;
        }
        _ => {}
    }
    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_scopes_once(child, Some(current_scope), callable_nodes, model);
        }
    }
}

fn direct_child(node: &SyntaxNode, kind: SyntaxKind) -> Option<&SyntaxNode> {
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(child) if child.kind == kind => Some(child.as_ref()),
        _ => None,
    })
}

fn has_direct_child(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    direct_child(node, kind).is_some()
}

fn statement_body<'a>(node: &'a SyntaxNode, header_kind: SyntaxKind) -> Option<&'a SyntaxNode> {
    let mut after_header = false;
    for child in &node.children {
        let SyntaxElement::Node(child) = child else {
            continue;
        };
        if after_header {
            return Some(child);
        }
        after_header = child.kind == header_kind;
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SourceFileMetadata, SourceKind};
    use crate::parser::parse_source;
    use crate::semantic_file::SemanticFile;

    fn index_for(source: &str) -> (Parse, SymbolIndex) {
        let parse = parse_source(source);
        let semantic = SemanticFile::build(source, &parse);
        let index = SymbolIndex::from_semantic_files([(
            &semantic,
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                category: crate::model::SourceCategory::Workspace,
                absolute_path: None,
                virtual_source: None,
                root_path: None,
                relative_path: None,
                priority: crate::model::SOURCE_PRIORITY_WORKSPACE,
            },
        )]);
        (parse, index)
    }

    fn semantic_index_for(source: &str) -> (Parse, SemanticFile, SymbolIndex, SourceFileId) {
        let parse = parse_source(source);
        let semantic = SemanticFile::build(source, &parse);
        let mut index = SymbolIndex::default();
        let file_id = index.add_semantic_file(
            &semantic,
            SourceFileMetadata {
                kind: SourceKind::Workspace,
                category: crate::model::SourceCategory::Workspace,
                absolute_path: None,
                virtual_source: None,
                root_path: None,
                relative_path: None,
                priority: crate::model::SOURCE_PRIORITY_WORKSPACE,
            },
        );
        (parse, semantic, index, file_id)
    }

    #[test]
    fn semantic_facts_authoritatively_attach_local_bindings_to_cst_scopes() {
        let source = "class Example { void Run(int parameter) { int local; local; } }";
        let (parse, semantic, index, file_id) = semantic_index_for(source);
        let scopes =
            LexicalScopeModel::from_parse_and_semantics(&parse, &semantic, &index, file_id);
        let local_use = source.rfind("local;").expect("local use");
        let visible = scopes.visible_symbols_named(&index, "local", local_use);

        assert_eq!(visible.len(), 1);
        assert_eq!(
            index.symbol(visible[0]).map(|symbol| symbol.kind),
            Some(SymbolKind::LocalVariable)
        );
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

    #[test]
    fn keeps_block_scopes_and_locals_with_their_own_callable() {
        let source = r#"class Example
{
	void First()
	{
		int first;
		first;
	}

	void Second()
	{
		int second;
		second;
	}
}
"#;
        let (parse, index) = index_for(source);
        let scopes = LexicalScopeModel::from_parse_and_index(&parse, &index);

        let first_use = source.rfind("first;").expect("first use");
        let second_use = source.rfind("second;").expect("second use");
        assert_eq!(
            scopes
                .visible_symbols_named(&index, "first", first_use)
                .len(),
            1
        );
        assert!(scopes
            .visible_symbols_named(&index, "first", second_use)
            .is_empty());
        assert_eq!(
            scopes
                .visible_symbols_named(&index, "second", second_use)
                .len(),
            1
        );
    }

    #[test]
    fn loop_locals_are_visible_only_in_their_loop_regions() {
        let source = r#"class Example
{
	void Run(array<int> items)
	{
		for (int index = 0; index < items.Count(); index++)
		{
			index;
		}
		index;
		foreach (int item : items)
		{
			item;
		}
		item;
		foreach (int items : items)
		{
			items;
		}
	}
}
"#;
        let (parse, index) = index_for(source);
        let scopes = LexicalScopeModel::from_parse_and_index(&parse, &index);

        let for_condition = source.find("index < items").expect("for condition");
        assert_eq!(
            index
                .symbol(scopes.visible_symbols_named(&index, "index", for_condition)[0])
                .unwrap()
                .kind,
            SymbolKind::LocalVariable
        );

        let after_for = source.rfind("\n\t\tindex;").expect("use after for") + 3;
        assert!(scopes
            .visible_symbols_named(&index, "index", after_for)
            .is_empty());

        let foreach_body = source.find("\n\t\t\titem;").expect("foreach body") + 4;
        assert_eq!(
            index
                .symbol(scopes.visible_symbols_named(&index, "item", foreach_body)[0])
                .unwrap()
                .kind,
            SymbolKind::LocalVariable
        );

        let after_foreach = source.rfind("\n\t\titem;").expect("use after foreach") + 3;
        assert!(scopes
            .visible_symbols_named(&index, "item", after_foreach)
            .is_empty());

        let iterable = source.rfind(": items)").expect("foreach iterable") + 2;
        let visible = scopes.visible_symbols_named(&index, "items", iterable);
        assert_eq!(visible.len(), 1);
        assert_eq!(
            index.symbol(visible[0]).unwrap().kind,
            SymbolKind::Parameter
        );
    }

    #[test]
    fn identifies_callable_regions_without_treating_root_as_local_scope() {
        let source = "class Example { void Run() { int localValue; localValue; } }";
        let (parse, index) = index_for(source);
        let scopes = LexicalScopeModel::from_parse_and_index(&parse, &index);

        assert!(scopes.has_callable_scope_at(source.find("localValue;").unwrap()));
        assert!(!scopes.has_callable_scope_at(0));
    }
}
