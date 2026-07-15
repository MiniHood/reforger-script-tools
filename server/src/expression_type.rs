use crate::ast::Expression;
use crate::index::{GlobalSymbolId, IndexedSymbol, SymbolIndex};
use crate::lexer::{lex, TextSpan, TokenKind};
use crate::model::SymbolKind;
use crate::scope::LexicalScopeModel;
use crate::syntax::{Parse, SyntaxElement, SyntaxKind, SyntaxNode};
use crate::type_facts::TypeFacts;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionType {
    pub owner_type: String,
    pub is_static: bool,
    pub raw_type_text: Option<String>,
}

impl ExpressionType {
    pub fn instance(owner_type: String) -> Self {
        Self {
            owner_type,
            is_static: false,
            raw_type_text: None,
        }
    }

    pub fn instance_with_raw(owner_type: String, raw_type_text: String) -> Self {
        Self {
            owner_type,
            is_static: false,
            raw_type_text: Some(raw_type_text),
        }
    }

    pub fn static_type(owner_type: String) -> Self {
        Self {
            owner_type,
            is_static: true,
            raw_type_text: None,
        }
    }

    pub fn static_type_with_raw(owner_type: String, raw_type_text: String) -> Self {
        Self {
            owner_type,
            is_static: true,
            raw_type_text: Some(raw_type_text),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExpressionTypeEnvironment<'source, 'index> {
    source: &'source str,
    file_index: &'index SymbolIndex,
    external_indexes: Vec<&'index SymbolIndex>,
    parse: &'index Parse,
    scope: &'index LexicalScopeModel,
}

impl<'source, 'index> ExpressionTypeEnvironment<'source, 'index> {
    pub fn new(
        source: &'source str,
        file_index: &'index SymbolIndex,
        parse: &'index Parse,
        scope: &'index LexicalScopeModel,
        external_index: Option<&'index SymbolIndex>,
    ) -> Self {
        Self::new_with_external_indexes(source, file_index, parse, scope, external_index)
    }

    pub fn new_with_external_indexes(
        source: &'source str,
        file_index: &'index SymbolIndex,
        parse: &'index Parse,
        scope: &'index LexicalScopeModel,
        external_indexes: impl IntoIterator<Item = &'index SymbolIndex>,
    ) -> Self {
        Self {
            source,
            file_index,
            external_indexes: external_indexes.into_iter().collect(),
            parse,
            scope,
        }
    }

    fn external_indexes(&self) -> impl Iterator<Item = &'index SymbolIndex> + '_ {
        self.external_indexes.iter().copied()
    }

    pub fn infer_expression_type(
        &self,
        expression: Expression<'source, '_>,
        offset: usize,
        lookup_path: &mut Vec<String>,
    ) -> Option<ExpressionType> {
        match expression {
            Expression::Call(node) => {
                let callee = node.callee()?;
                if let Some(member_access) = member_access_parts(callee) {
                    let receiver_type =
                        self.infer_expression_type(member_access.receiver, offset, lookup_path)?;
                    let member = member_access.member_name.text();
                    let callee_text = callee.source_text().trim();
                    if member == "Cast" && receiver_type.is_static {
                        let raw_type_text =
                            receiver_type.raw_type_text.clone().unwrap_or_else(|| {
                                member_access.receiver.source_text().trim().to_string()
                            });
                        lookup_path.push(format!(
                            "`{callee_text}` treated as cast returning `{}`",
                            receiver_type.owner_type
                        ));
                        return Some(ExpressionType::instance_with_raw(
                            receiver_type.owner_type,
                            raw_type_text,
                        ));
                    }
                    lookup_path.push(format!(
                        "`{callee_text}` call receiver inferred as `{}`",
                        receiver_type.owner_type
                    ));
                    return self.member_result_type_for_receiver(
                        &receiver_type,
                        member,
                        receiver_type.is_static,
                        lookup_path,
                    );
                }

                let callee_name = simple_callee_name(callee)?;
                lookup_path.push(format!("call `{callee_name}`"));
                self.callable_result_type(&callee_name, offset, lookup_path)
            }
            Expression::Index(node) => {
                let base = node.receiver()?;
                let base_type = self.infer_expression_type(base, offset, lookup_path)?;
                if let Some(raw_type_text) = base_type.raw_type_text.as_deref() {
                    if let Some(element_type) = collection_index_result_type(raw_type_text) {
                        lookup_path.push(format!(
                            "`{}` indexed receiver inferred as `{}`",
                            expression.source_text().trim(),
                            element_type.owner_type
                        ));
                        return Some(element_type);
                    }
                }
                lookup_path.push(format!(
                    "`{}` indexed receiver base inferred as `{}` without element type",
                    expression.source_text().trim(),
                    base_type.owner_type
                ));
                None
            }
            Expression::MemberAccess(_) => {
                let member_access = member_access_parts(expression)?;
                let receiver_type =
                    self.infer_expression_type(member_access.receiver, offset, lookup_path)?;
                let member = member_access.member_name.text();
                lookup_path.push(format!(
                    "`{}` member receiver inferred as `{}`",
                    expression.source_text().trim(),
                    receiver_type.owner_type
                ));
                self.member_result_type_for_receiver(
                    &receiver_type,
                    member,
                    receiver_type.is_static,
                    lookup_path,
                )
            }
            Expression::Unary(node)
            | Expression::Binary(node)
            | Expression::Parenthesized(node) => {
                if let Some(child) = first_expression_child(self.source, node.syntax_node()) {
                    if let Some(result) = self.infer_expression_type(child, offset, lookup_path) {
                        lookup_path.push(format!(
                            "`{}` inferred from nested `{}` expression",
                            expression.source_text().trim(),
                            format!("{:?}", expression.kind()).to_lowercase()
                        ));
                        return Some(result);
                    }
                }
                if let Some(primitive) =
                    primitive_type_from_expression_text(expression.source_text())
                {
                    lookup_path.push(format!(
                        "`{}` inferred as primitive `{primitive}`",
                        expression.source_text().trim()
                    ));
                    return Some(ExpressionType::instance_with_raw(
                        primitive.to_string(),
                        primitive.to_string(),
                    ));
                }
                None
            }
            Expression::Literal(_) => {
                let primitive = primitive_type_from_expression_text(expression.source_text())?;
                lookup_path.push(format!(
                    "`{}` inferred as primitive `{primitive}`",
                    expression.source_text().trim()
                ));
                Some(ExpressionType::instance_with_raw(
                    primitive.to_string(),
                    primitive.to_string(),
                ))
            }
            Expression::New(_) => {
                let text = expression.source_text().trim();
                let owner_type = owner_type_from_new_expression_text(text)?;
                lookup_path.push(format!("`{text}` inferred as new `{owner_type}`"));
                Some(ExpressionType::instance_with_raw(
                    owner_type,
                    text.to_string(),
                ))
            }
            Expression::Unknown(node) => first_expression_child(self.source, node.syntax_node())
                .and_then(|child| self.infer_expression_type(child, offset, lookup_path)),
            _ => {
                if let Some(primitive) =
                    primitive_type_from_expression_text(expression.source_text())
                {
                    lookup_path.push(format!(
                        "`{}` inferred as primitive `{primitive}`",
                        expression.source_text().trim()
                    ));
                    return Some(ExpressionType::instance_with_raw(
                        primitive.to_string(),
                        primitive.to_string(),
                    ));
                }

                if is_null_literal_text(expression.source_text()) {
                    lookup_path.push("`null` inferred as null literal".to_string());
                    return Some(ExpressionType::instance_with_raw(
                        "null".to_string(),
                        "null".to_string(),
                    ));
                }

                if let Some(static_type) = self.static_type_from_type_text_expression(expression) {
                    lookup_path.push(format!(
                        "`{}` inferred as static type `{}`",
                        expression.source_text().trim(),
                        static_type.owner_type
                    ));
                    return Some(static_type);
                }

                let name = expression.name_text()?.text();
                if name == "this" {
                    let class_name = self
                        .containing_class(offset)
                        .and_then(|id| self.file_index.symbol(id))
                        .and_then(|symbol| symbol.name.clone());
                    if let Some(class_name) = class_name {
                        lookup_path.push(format!("`this` inferred as `{class_name}`"));
                        return Some(ExpressionType::instance(class_name));
                    }
                }

                if name == "super" {
                    let base_type = self
                        .containing_class(offset)
                        .and_then(|id| base_owner_type_from_symbol(self.file_index, id));
                    if let Some(base_type) = base_type {
                        lookup_path.push(format!("`super` inferred as base `{base_type}`"));
                        return Some(ExpressionType::instance(base_type));
                    }
                }

                if self.class_type_parameter_exists(name, offset) {
                    lookup_path.push(format!("`{name}` inferred as class type parameter"));
                    return Some(ExpressionType::static_type(name.to_string()));
                }

                self.identifier_result_type(name, offset, lookup_path)
            }
        }
    }

    pub fn static_type_name_from_expression(
        &self,
        expression: Expression<'source, '_>,
        offset: usize,
    ) -> Option<String> {
        let name = simple_callee_name(expression)?;
        if self.class_type_parameter_exists(&name, offset) {
            return Some(name);
        }
        self.static_type_name_from_index(self.file_index, &name)
            .or_else(|| {
                self.external_indexes()
                    .find_map(|index| self.static_type_name_from_index(index, &name))
            })
    }

    pub fn type_from_file_symbol(&self, symbol: &IndexedSymbol) -> Option<ExpressionType> {
        let mut result = expression_type_from_index_symbol(self.file_index, symbol)?;
        if result.owner_type == "auto" && symbol.kind == SymbolKind::LocalVariable {
            if let Some(default_span) = symbol.detail.default_text_span {
                if let Some(expression) =
                    smallest_expression_containing_span(self.source, &self.parse.root, default_span)
                {
                    let mut lookup_path = Vec::new();
                    if let Some(default_result) =
                        self.infer_expression_type(expression, default_span.start, &mut lookup_path)
                    {
                        return Some(default_result);
                    }
                }
            }
            if let Some(default_text) = symbol.detail.default_text.as_deref() {
                if let Some(owner_type) = owner_type_from_new_expression_text(default_text) {
                    return Some(ExpressionType::instance_with_raw(
                        owner_type,
                        default_text.to_string(),
                    ));
                }
                if let Some(owner_type) = owner_type_from_cast_expression_text(default_text) {
                    return Some(ExpressionType::instance_with_raw(
                        owner_type,
                        default_text.to_string(),
                    ));
                }
            }
            if let Some(foreach_result) = self.foreach_auto_type_from_iterable(symbol) {
                return Some(foreach_result);
            }
        }
        if matches!(
            symbol.kind,
            SymbolKind::LocalVariable
                | SymbolKind::Parameter
                | SymbolKind::Field
                | SymbolKind::GlobalField
        ) {
            if let Some(type_text) = type_text_with_static_array_suffix(self.source, symbol) {
                result.raw_type_text = Some(type_text);
            }
        }
        Some(result)
    }

    fn class_type_parameter_exists(&self, name: &str, offset: usize) -> bool {
        let Some(class_id) = self.containing_class(offset) else {
            return false;
        };
        self.file_index.children(class_id).iter().any(|child| {
            self.file_index.symbol(*child).is_some_and(|symbol| {
                symbol.kind == SymbolKind::TypeParameter && symbol.name.as_deref() == Some(name)
            })
        })
    }

    fn static_type_name_from_index(&self, index: &SymbolIndex, name: &str) -> Option<String> {
        for id in index.top_level_symbols_for_name(name) {
            let Some(symbol) = index.symbol(*id) else {
                continue;
            };
            if matches!(
                symbol.kind,
                SymbolKind::Class | SymbolKind::Enum | SymbolKind::Typedef
            ) {
                return symbol
                    .name
                    .clone()
                    .or_else(|| owner_type_from_index_symbol(index, symbol));
            }
        }
        None
    }

    fn static_type_from_type_text_expression(
        &self,
        expression: Expression<'source, '_>,
    ) -> Option<ExpressionType> {
        let type_text = expression.source_text().trim();
        let owner = owner_type_from_type_text(type_text)?;
        if self
            .static_type_name_from_index(self.file_index, &owner)
            .is_some()
            || self
                .external_indexes()
                .any(|index| self.static_type_name_from_index(index, &owner).is_some())
        {
            return Some(ExpressionType::static_type_with_raw(
                owner,
                type_text.to_string(),
            ));
        }
        None
    }

    fn identifier_result_type(
        &self,
        name: &str,
        offset: usize,
        lookup_path: &mut Vec<String>,
    ) -> Option<ExpressionType> {
        for id in self
            .scope
            .visible_symbols_named(self.file_index, name, offset)
        {
            let Some(symbol) = self.file_index.symbol(id) else {
                continue;
            };
            let origin = match symbol.kind {
                SymbolKind::LocalVariable => "local",
                SymbolKind::Parameter => "parameter",
                _ => continue,
            };
            if let Some(result) = self.type_from_file_symbol(symbol) {
                lookup_path.push(format!(
                    "`{name}` inferred from `{origin}` `{}`",
                    result.owner_type
                ));
                return Some(result);
            }
        }

        if let Some(class) = self.containing_class(offset) {
            let class_name = self
                .file_index
                .symbol(class)
                .and_then(|symbol| symbol.name.as_deref());
            if let Some(class_name) = class_name {
                if let Some(result) =
                    self.member_type_from_owner_for_name(self.file_index, class_name, name)
                {
                    lookup_path.push(format!(
                        "`{name}` inferred from class member `{}`",
                        result.owner_type
                    ));
                    return Some(result);
                }
                for external_index in self.external_indexes() {
                    if let Some(result) =
                        self.member_type_from_owner_for_name(external_index, class_name, name)
                    {
                        lookup_path.push(format!(
                            "`{name}` inferred from external class member `{}`",
                            result.owner_type
                        ));
                        return Some(result);
                    }
                    if let Some(base_type) = base_owner_type_from_symbol(self.file_index, class) {
                        if let Some(result) =
                            self.member_type_from_owner_for_name(external_index, &base_type, name)
                        {
                            lookup_path.push(format!(
                                "`{name}` inferred from external base member `{}`",
                                result.owner_type
                            ));
                            return Some(result);
                        }
                    }
                }
            }
        }

        for id in self.file_index.top_level_symbols_for_name(name) {
            let Some(symbol) = self.file_index.symbol(*id) else {
                continue;
            };
            if let Some(result) = expression_type_from_top_level_symbol(self.file_index, symbol) {
                lookup_path.push(format!("`{name}` inferred from file-local top-level"));
                return Some(result);
            }
        }

        for external_index in self.external_indexes() {
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
                if let Some(result) = expression_type_from_top_level_symbol(external_index, symbol)
                {
                    lookup_path.push(format!("`{name}` inferred from external top-level"));
                    return Some(result);
                }
            }
        }

        None
    }

    fn callable_result_type(
        &self,
        name: &str,
        offset: usize,
        lookup_path: &mut Vec<String>,
    ) -> Option<ExpressionType> {
        for id in self
            .scope
            .visible_symbols_named(self.file_index, name, offset)
        {
            let Some(symbol) = self.file_index.symbol(id) else {
                continue;
            };
            if symbol.kind == SymbolKind::LocalVariable {
                if let Some(result) = self.type_from_file_symbol(symbol) {
                    lookup_path.push(format!("call `{name}` matched local callable-like value"));
                    return Some(result);
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
                if let Some(base_type) = base_owner_type_from_symbol(self.file_index, class) {
                    if let Some(result) =
                        self.member_result_type(&base_type, name, false, lookup_path)
                    {
                        lookup_path.push(format!("call `{name}` matched containing base member"));
                        return Some(result);
                    }
                }
            }
        }

        for id in self.file_index.functions_by_name(name) {
            let Some(symbol) = self.file_index.symbol(*id) else {
                continue;
            };
            if let Some(result) = expression_type_from_index_symbol(self.file_index, symbol) {
                lookup_path.push(format!("call `{name}` matched file-local function"));
                return Some(result);
            }
        }

        for external_index in self.external_indexes() {
            for id in external_index.preferred_functions_by_name(name) {
                let Some(symbol) = external_index.symbol(id) else {
                    continue;
                };
                if let Some(result) = expression_type_from_index_symbol(external_index, symbol) {
                    lookup_path.push(format!("call `{name}` matched external function"));
                    return Some(result);
                }
            }
        }

        if let Some(result) = constructor_call_result_type_from_index(self.file_index, name) {
            lookup_path.push(format!(
                "call `{name}` matched file-local constructor-style type call"
            ));
            return Some(result);
        }
        for external_index in self.external_indexes() {
            if let Some(result) = constructor_call_result_type_from_index(external_index, name) {
                lookup_path.push(format!(
                    "call `{name}` matched external constructor-style type call"
                ));
                return Some(result);
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
    ) -> Option<ExpressionType> {
        if let Some(result) =
            member_result_type_from_index(self.file_index, owner, member, static_only)
        {
            lookup_path.push(format!("member `{owner}.{member}` matched file-local"));
            return Some(result);
        }
        for external_index in self.external_indexes() {
            if let Some(result) =
                member_result_type_from_index(external_index, owner, member, static_only)
            {
                lookup_path.push(format!("member `{owner}.{member}` matched external"));
                return Some(result);
            }
        }
        None
    }

    fn member_result_type_for_receiver(
        &self,
        receiver: &ExpressionType,
        member: &str,
        static_only: bool,
        lookup_path: &mut Vec<String>,
    ) -> Option<ExpressionType> {
        if let Some(result) = member_result_type_for_receiver_from_index(
            self.file_index,
            receiver,
            member,
            static_only,
        ) {
            lookup_path.push(format!(
                "member `{}.{member}` matched file-local",
                receiver.owner_type
            ));
            return Some(result);
        }
        for external_index in self.external_indexes() {
            if let Some(result) = member_result_type_for_receiver_from_index(
                external_index,
                receiver,
                member,
                static_only,
            ) {
                lookup_path.push(format!(
                    "member `{}.{member}` matched external",
                    receiver.owner_type
                ));
                return Some(result);
            }
        }
        None
    }

    fn member_type_from_owner_for_name(
        &self,
        index: &SymbolIndex,
        owner: &str,
        name: &str,
    ) -> Option<ExpressionType> {
        for owner in member_lookup_owners(index, owner) {
            let lookup = index.completion_members_for_preferred_class(&owner);
            let matching = matching_members_from_ids(index, lookup.members.iter().copied(), name);
            for id in index.preferred_from_symbols(&matching) {
                let Some(symbol) = index.symbol(id) else {
                    continue;
                };
                let result = if std::ptr::eq(index, self.file_index) {
                    self.type_from_file_symbol(symbol)
                } else {
                    expression_type_from_index_symbol(index, symbol)
                };
                if let Some(result) = result {
                    return Some(result);
                }
            }
        }
        None
    }

    fn containing_class(&self, offset: usize) -> Option<GlobalSymbolId> {
        self.file_index
            .symbols()
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Class && span_contains(symbol.span, offset))
            .min_by_key(|symbol| symbol.span.len())
            .map(|symbol| symbol.id)
    }

    fn foreach_auto_type_from_iterable(&self, symbol: &IndexedSymbol) -> Option<ExpressionType> {
        let header = foreach_header_containing(&self.parse.root, symbol.selection_span)?;
        let variables = foreach_variables_in_header(header);
        let variable_index = variables
            .iter()
            .position(|variable| span_contains(variable.span, symbol.selection_span.start))?;
        let iterable = foreach_iterable_in_header(header)?;
        let expression = first_expression_child(self.source, iterable)?;
        let mut lookup_path = Vec::new();
        let iterable_type =
            self.infer_expression_type(expression, symbol.selection_span.start, &mut lookup_path)?;
        let raw_type_text = iterable_type.raw_type_text.as_deref()?;
        let element_type = collection_index_result_type(raw_type_text)?;
        if variable_index == 0
            && generic_owner_and_args(strip_all_type_prefixes(raw_type_text))
                .is_some_and(|(owner, args)| owner == "map" && args.len() >= 2)
        {
            return None;
        }
        Some(element_type)
    }
}

pub fn expression_type_from_index_symbol(
    index: &SymbolIndex,
    symbol: &IndexedSymbol,
) -> Option<ExpressionType> {
    let facts = TypeFacts::new(index);
    let raw_type_text = match symbol.kind {
        SymbolKind::LocalVariable
        | SymbolKind::Parameter
        | SymbolKind::Field
        | SymbolKind::GlobalField
        | SymbolKind::Typedef => facts.facts_for_symbol(symbol.id)?.type_text,
        SymbolKind::Function | SymbolKind::Method => facts.callable_return_type_text(symbol.id),
        SymbolKind::Constructor | SymbolKind::Class | SymbolKind::Enum => {
            return symbol.name.clone().map(ExpressionType::instance);
        }
        _ => None,
    }?;
    let owner_type = owner_type_from_type_text(raw_type_text)?;
    Some(ExpressionType::instance_with_raw(
        owner_type,
        raw_type_text.to_string(),
    ))
}

pub fn expression_type_from_index_symbol_with_receiver(
    index: &SymbolIndex,
    owner: &str,
    symbol: &IndexedSymbol,
    receiver_type_text: Option<&str>,
) -> Option<ExpressionType> {
    let result = expression_type_from_index_symbol(index, symbol)?;
    let Some(receiver_type_text) = receiver_type_text else {
        return Some(result);
    };
    let Some(raw_type_text) = result.raw_type_text.as_deref() else {
        return Some(result);
    };
    let Some(substituted) =
        substitute_generic_return_type(index, owner, receiver_type_text, raw_type_text)
    else {
        return Some(result);
    };
    let owner_type = owner_type_from_type_text(&substituted)?;
    Some(ExpressionType::instance_with_raw(owner_type, substituted))
}

pub fn expression_type_from_top_level_symbol(
    index: &SymbolIndex,
    symbol: &IndexedSymbol,
) -> Option<ExpressionType> {
    match symbol.kind {
        SymbolKind::Class | SymbolKind::Enum | SymbolKind::Typedef => {
            let owner_type = symbol
                .name
                .clone()
                .or_else(|| owner_type_from_index_symbol(index, symbol))?;
            Some(ExpressionType::static_type(owner_type))
        }
        SymbolKind::Function | SymbolKind::GlobalField => {
            expression_type_from_index_symbol(index, symbol)
        }
        _ => None,
    }
}

pub fn owner_type_from_index_symbol(index: &SymbolIndex, symbol: &IndexedSymbol) -> Option<String> {
    let facts = TypeFacts::new(index);
    match symbol.kind {
        SymbolKind::LocalVariable
        | SymbolKind::Parameter
        | SymbolKind::Field
        | SymbolKind::GlobalField
        | SymbolKind::Typedef => facts
            .facts_for_symbol(symbol.id)?
            .type_text
            .and_then(owner_type_from_type_text),
        SymbolKind::Function | SymbolKind::Method => facts
            .callable_return_type_text(symbol.id)
            .and_then(owner_type_from_type_text),
        SymbolKind::Constructor | SymbolKind::Class | SymbolKind::Enum => symbol.name.clone(),
        _ => None,
    }
}

pub fn member_lookup_owners(index: &SymbolIndex, owner: &str) -> Vec<String> {
    let mut owners = Vec::new();
    push_unique_string(&mut owners, owner.to_string());
    let facts = TypeFacts::new(index);

    for id in index.preferred_typedefs_by_name(owner) {
        let Some(type_text) = facts.typedef_target_text(id) else {
            continue;
        };
        if let Some(target_owner) = owner_type_from_type_text(type_text) {
            push_unique_string(&mut owners, target_owner);
        }
    }

    for id in index.preferred_classes_by_name(owner) {
        if let Some(target_owner) = base_owner_type_from_symbol(index, id) {
            push_unique_string(&mut owners, target_owner);
        }
    }

    owners
}

pub fn base_owner_type_from_symbol(index: &SymbolIndex, id: GlobalSymbolId) -> Option<String> {
    TypeFacts::new(index)
        .class_base_type(id)
        .and_then(owner_type_from_type_text)
}

pub fn collection_index_result_type(type_text: &str) -> Option<ExpressionType> {
    let text = strip_all_type_prefixes(type_text);

    if text == "string" {
        return Some(ExpressionType::instance_with_raw(
            "string".to_string(),
            "string".to_string(),
        ));
    }

    if matches!(text, "vector" | "quat") {
        return Some(ExpressionType::instance_with_raw(
            "float".to_string(),
            "float".to_string(),
        ));
    }

    if let Some(base) = static_array_base_type(text) {
        let owner_type = owner_type_from_type_text(base)?;
        return Some(ExpressionType::instance_with_raw(
            owner_type,
            base.trim().to_string(),
        ));
    }

    let (owner, args) = generic_owner_and_args(text)?;
    let target = match owner {
        "array" | "set" => args.first()?,
        "map" => args.get(1)?,
        _ => return None,
    };
    let owner_type = owner_type_from_type_text(target)?;
    Some(ExpressionType::instance_with_raw(
        owner_type,
        target.trim().to_string(),
    ))
}

pub fn primitive_type_from_expression_text(text: &str) -> Option<&'static str> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    if is_string_literal_text(text) {
        return Some("string");
    }

    if matches!(text, "true" | "false") {
        return Some("bool");
    }

    if looks_like_numeric_expression(text) {
        if text.contains('.') || text.contains('e') || text.contains('E') {
            return Some("float");
        }
        return Some("int");
    }

    None
}

pub fn is_null_literal_text(text: &str) -> bool {
    text.trim() == "null"
}

pub fn generic_owner_and_args(type_text: &str) -> Option<(&str, Vec<String>)> {
    let type_text = type_text.trim();
    let less = type_text.find('<')?;
    let owner = type_text[..less].trim();
    let mut depth = 0usize;
    let mut close = None;
    for (index, ch) in type_text.char_indices().skip(less) {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let args = split_top_level_commas(&type_text[less + 1..close]);
    (!owner.is_empty()).then_some((owner, args))
}

pub fn owner_type_from_type_text(type_text: &str) -> Option<String> {
    let text = strip_all_type_prefixes(type_text);

    if text.is_empty() {
        return None;
    }

    for collection in ["array", "set", "map"] {
        if text.starts_with(collection) && text[collection.len()..].trim_start().starts_with('<') {
            return Some(collection.to_string());
        }
    }

    let owner = text
        .split(|ch: char| {
            ch == '<' || ch == '[' || ch == '(' || ch.is_whitespace() || ch == '&' || ch == '*'
        })
        .next()
        .unwrap_or_default()
        .trim();

    (!owner.is_empty()).then(|| owner.to_string())
}

pub fn owner_type_from_new_expression_text(text: &str) -> Option<String> {
    let text = text.trim();
    let rest = text.strip_prefix("new")?.trim_start();
    owner_type_from_type_text(rest)
}

pub fn owner_type_from_cast_expression_text(text: &str) -> Option<String> {
    let text = text.trim();
    let cast_start = text.find(".Cast")?;
    let owner = text[..cast_start].trim_end();
    if owner.is_empty() {
        return None;
    }
    owner_type_from_type_text(owner)
}

pub fn strip_all_type_prefixes(type_text: &str) -> &str {
    let mut text = type_text.trim();
    loop {
        let stripped = strip_type_prefix(text).trim_start();
        if stripped == text {
            return text;
        }
        text = stripped;
    }
}

pub fn substitute_generic_return_type(
    index: &SymbolIndex,
    owner: &str,
    receiver_type_text: &str,
    return_type_text: &str,
) -> Option<String> {
    let return_owner = owner_type_from_type_text(return_type_text)?;
    let receiver_owner = owner_type_from_type_text(receiver_type_text)?;
    if receiver_owner != owner {
        return None;
    }
    let (_, args) = generic_owner_and_args(strip_all_type_prefixes(receiver_type_text))?;
    let parameter_names = class_type_parameter_names(index, owner);
    let position = parameter_names
        .iter()
        .position(|name| name == &return_owner)?;
    args.get(position).cloned()
}

fn member_access_parts<'source, 'tree>(
    expression: Expression<'source, 'tree>,
) -> Option<crate::ast::MemberAccessExpression<'source, 'tree>> {
    match expression {
        Expression::MemberAccess(_) => {
            let receiver = expression.receiver()?;
            let member_name = expression.member_name()?;
            Some(crate::ast::MemberAccessExpression {
                expression,
                receiver,
                member_name,
            })
        }
        _ => None,
    }
}

fn foreach_header_containing(node: &SyntaxNode, span: TextSpan) -> Option<&SyntaxNode> {
    if !span_contains(node.span, span.start)
        || !span_contains(node.span, span.end.saturating_sub(1))
    {
        return None;
    }
    if node.kind == SyntaxKind::ForeachHeader {
        return Some(node);
    }
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(child) => foreach_header_containing(child, span),
        SyntaxElement::Token(_) => None,
    })
}

fn foreach_variables_in_header(header: &SyntaxNode) -> Vec<&SyntaxNode> {
    let mut variables = Vec::new();
    collect_foreach_variables(header, &mut variables);
    variables
}

fn collect_foreach_variables<'tree>(
    node: &'tree SyntaxNode,
    variables: &mut Vec<&'tree SyntaxNode>,
) {
    if node.kind == SyntaxKind::ForeachVariable {
        variables.push(node);
        return;
    }
    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_foreach_variables(child, variables);
        }
    }
}

fn foreach_iterable_in_header(header: &SyntaxNode) -> Option<&SyntaxNode> {
    header.children.iter().find_map(|child| match child {
        SyntaxElement::Node(child) if child.kind == SyntaxKind::ForeachIterable => {
            Some(child.as_ref())
        }
        SyntaxElement::Node(child) => foreach_iterable_in_header(child),
        SyntaxElement::Token(_) => None,
    })
}

fn first_expression_child<'source, 'tree>(
    source: &'source str,
    node: &'tree SyntaxNode,
) -> Option<Expression<'source, 'tree>> {
    node.children.iter().find_map(|child| match child {
        SyntaxElement::Node(node) => Expression::from_node(source, node),
        SyntaxElement::Token(_) => None,
    })
}

fn smallest_expression_containing_span<'source, 'tree>(
    source: &'source str,
    root: &'tree SyntaxNode,
    span: TextSpan,
) -> Option<Expression<'source, 'tree>> {
    let mut best = None;
    collect_smallest_expression_containing_span(source, root, span, &mut best);
    best
}

fn collect_smallest_expression_containing_span<'source, 'tree>(
    source: &'source str,
    node: &'tree SyntaxNode,
    span: TextSpan,
    best: &mut Option<Expression<'source, 'tree>>,
) {
    if node.span.start > span.start || node.span.end < span.end {
        return;
    }

    if let Some(expression) = Expression::from_node(source, node) {
        let replace = best
            .as_ref()
            .map(|best| expression.span().len() < best.span().len())
            .unwrap_or(true);
        if replace {
            *best = Some(expression);
        }
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_smallest_expression_containing_span(source, child, span, best);
        }
    }
}

fn type_text_with_static_array_suffix(source: &str, symbol: &IndexedSymbol) -> Option<String> {
    let type_text = symbol.detail.type_text.as_deref()?;
    let suffix =
        static_array_suffix_after_selection(source, symbol.selection_span.end, symbol.span.end)?;
    Some(format!("{type_text}{suffix}"))
}

fn static_array_suffix_after_selection(source: &str, start: usize, end: usize) -> Option<String> {
    let text = source.get(start..end)?;
    let mut suffix = String::new();
    let mut chars = text.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        if ch != '[' {
            break;
        }

        let suffix_start = index;
        let mut depth = 1usize;
        let mut suffix_end = None;
        for (next_index, next_ch) in chars.by_ref() {
            match next_ch {
                '[' => depth += 1,
                ']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        suffix_end = Some(next_index + next_ch.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }

        let suffix_end = suffix_end?;
        suffix.push_str(&text[suffix_start..suffix_end]);
    }

    (!suffix.is_empty()).then_some(suffix)
}

fn member_result_type_from_index(
    index: &SymbolIndex,
    owner: &str,
    member: &str,
    static_only: bool,
) -> Option<ExpressionType> {
    if static_only && enum_member_exists(index, owner, member) {
        return Some(ExpressionType::instance(owner.to_string()));
    }

    for owner in member_lookup_owners(index, owner) {
        if let Some(result) = member_result_type_for_exact_owner(index, &owner, member, static_only)
        {
            return Some(result);
        }
    }

    if !static_only && is_pseudo_class_member_name(member) && owner != "Class" {
        return member_result_type_for_exact_owner(index, "Class", member, false);
    }

    None
}

fn constructor_call_result_type_from_index(
    index: &SymbolIndex,
    name: &str,
) -> Option<ExpressionType> {
    for id in index.preferred_classes_by_name(name) {
        let Some(symbol) = index.symbol(id) else {
            continue;
        };
        let owner_type = symbol.name.clone().unwrap_or_else(|| name.to_string());
        return Some(ExpressionType::instance_with_raw(
            owner_type,
            name.to_string(),
        ));
    }

    for id in index.preferred_typedefs_by_name(name) {
        let Some(symbol) = index.symbol(id) else {
            continue;
        };
        let owner_type = symbol.name.clone().unwrap_or_else(|| name.to_string());
        return Some(ExpressionType::instance_with_raw(
            owner_type,
            name.to_string(),
        ));
    }

    None
}

fn member_result_type_for_receiver_from_index(
    index: &SymbolIndex,
    receiver: &ExpressionType,
    member: &str,
    static_only: bool,
) -> Option<ExpressionType> {
    if static_only && enum_member_exists(index, &receiver.owner_type, member) {
        return Some(ExpressionType::instance(receiver.owner_type.clone()));
    }

    for owner in member_lookup_owners(index, &receiver.owner_type) {
        if let Some(result) = member_result_type_for_exact_owner_with_receiver(
            index,
            &owner,
            member,
            static_only,
            receiver.raw_type_text.as_deref(),
        ) {
            return Some(result);
        }
    }

    if !static_only && is_pseudo_class_member_name(member) && receiver.owner_type != "Class" {
        return member_result_type_for_exact_owner_with_receiver(
            index,
            "Class",
            member,
            false,
            receiver.raw_type_text.as_deref(),
        );
    }

    None
}

fn enum_member_exists(index: &SymbolIndex, owner: &str, member: &str) -> bool {
    !enum_member_ids_for_owner(index, owner, member).is_empty()
}

fn enum_member_ids_for_owner(
    index: &SymbolIndex,
    owner: &str,
    member: &str,
) -> Vec<GlobalSymbolId> {
    let mut ids = Vec::new();
    collect_enum_member_ids_for_owner(
        index,
        owner,
        member,
        &mut std::collections::BTreeSet::new(),
        &mut ids,
    );
    ids
}

fn collect_enum_member_ids_for_owner(
    index: &SymbolIndex,
    owner: &str,
    member: &str,
    visited: &mut std::collections::BTreeSet<String>,
    ids: &mut Vec<GlobalSymbolId>,
) {
    if !visited.insert(owner.to_string()) {
        return;
    }

    for expanded_owner in member_lookup_owners(index, owner) {
        if expanded_owner == owner {
            continue;
        }
        collect_enum_member_ids_for_owner(index, &expanded_owner, member, visited, ids);
    }

    for enum_id in index.top_level_symbols_for_name(owner) {
        let Some(enum_symbol) = index.symbol(*enum_id) else {
            continue;
        };
        if enum_symbol.kind != SymbolKind::Enum {
            continue;
        }
        for child in index.children(*enum_id) {
            let Some(symbol) = index.symbol(*child) else {
                continue;
            };
            if symbol.kind == SymbolKind::EnumMember && symbol.name.as_deref() == Some(member) {
                push_unique_id(ids, *child);
            }
        }
        if let Some(base_type) = enum_symbol
            .detail
            .base_type
            .as_deref()
            .and_then(owner_type_from_type_text)
        {
            collect_enum_member_ids_for_owner(index, &base_type, member, visited, ids);
        }
    }
}

fn member_result_type_for_exact_owner(
    index: &SymbolIndex,
    owner: &str,
    member: &str,
    static_only: bool,
) -> Option<ExpressionType> {
    let mut matching = matching_members_for_exact_owner(index, owner, member);

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
        if let Some(result) = expression_type_from_index_symbol(index, symbol) {
            return Some(result);
        }
        if symbol.kind == SymbolKind::EnumMember {
            return Some(ExpressionType::instance(owner.to_string()));
        }
    }

    None
}

fn member_result_type_for_exact_owner_with_receiver(
    index: &SymbolIndex,
    owner: &str,
    member: &str,
    static_only: bool,
    receiver_type_text: Option<&str>,
) -> Option<ExpressionType> {
    let mut matching = matching_members_for_exact_owner(index, owner, member);

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
        if let Some(result) = expression_type_from_index_symbol_with_receiver(
            index,
            owner,
            symbol,
            receiver_type_text,
        ) {
            return Some(result);
        }
        if symbol.kind == SymbolKind::EnumMember {
            return Some(ExpressionType::instance(owner.to_string()));
        }
    }

    None
}

fn matching_members_for_exact_owner(
    index: &SymbolIndex,
    owner: &str,
    name: &str,
) -> Vec<GlobalSymbolId> {
    let lookup = index.completion_members_for_preferred_class(owner);
    matching_members_from_ids(index, lookup.members.iter().copied(), name)
}

fn matching_members_from_ids(
    index: &SymbolIndex,
    ids: impl Iterator<Item = GlobalSymbolId>,
    name: &str,
) -> Vec<GlobalSymbolId> {
    ids.filter(|id| {
        index.symbol(*id).is_some_and(|symbol| {
            is_member_lookup_kind(symbol.kind) && symbol.name.as_deref() == Some(name)
        })
    })
    .collect()
}

fn simple_callee_name(expression: Expression<'_, '_>) -> Option<String> {
    if let Some(name) = expression.name_text() {
        return Some(name.text().to_string());
    }

    let text = expression.source_text().trim();
    let mut tokens = lex(text)
        .into_iter()
        .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof);
    let token = tokens.next()?;
    if tokens.next().is_some() || token.kind != TokenKind::Identifier {
        return None;
    }
    Some(text[token.span.start..token.span.end].to_string())
}

fn is_member_lookup_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Field | SymbolKind::Method | SymbolKind::Constructor | SymbolKind::Destructor
    )
}

fn is_pseudo_class_member_name(name: &str) -> bool {
    matches!(name, "ClassName" | "IsInherited" | "ToString" | "Type")
}

fn has_modifier(symbol: &IndexedSymbol, modifier: &str) -> bool {
    symbol.modifiers.iter().any(|value| value == modifier)
}

fn span_contains(span: TextSpan, offset: usize) -> bool {
    span.start <= offset && offset < span.end
}

fn class_type_parameter_names(index: &SymbolIndex, owner: &str) -> Vec<String> {
    for id in index.preferred_classes_by_name(owner) {
        let mut names = Vec::new();
        for child in index.children(id) {
            let Some(symbol) = index.symbol(*child) else {
                continue;
            };
            if symbol.kind == SymbolKind::TypeParameter {
                if let Some(name) = &symbol.name {
                    names.push(name.clone());
                }
            }
        }
        if !names.is_empty() {
            return names;
        }
    }
    Vec::new()
}

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn push_unique_id(ids: &mut Vec<GlobalSymbolId>, id: GlobalSymbolId) {
    if !ids.contains(&id) {
        ids.push(id);
    }
}

fn split_top_level_commas(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    for (index, ch) in text.char_indices() {
        match ch {
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                args.push(text[start..index].trim().to_string());
                start = index + 1;
            }
            _ => {}
        }
    }
    args.push(text[start..].trim().to_string());
    args.retain(|arg| !arg.is_empty());
    args
}

fn static_array_base_type(type_text: &str) -> Option<&str> {
    let bracket = type_text.find('[')?;
    let base = type_text[..bracket].trim();
    (!base.is_empty()).then_some(base)
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

fn is_string_literal_text(text: &str) -> bool {
    text.len() >= 2 && text.starts_with('"') && text.ends_with('"')
}

fn looks_like_numeric_expression(text: &str) -> bool {
    let trimmed = text.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return !hex.is_empty() && hex.chars().all(|ch| ch.is_ascii_hexdigit());
    }

    let mut saw_digit = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        if ch.is_ascii_whitespace()
            || matches!(
                ch,
                '+' | '-' | '*' | '/' | '%' | '.' | '(' | ')' | '<' | '>' | '=' | '!' | '&' | '|'
            )
            || matches!(ch, 'e' | 'E')
        {
            continue;
        }
        return false;
    }
    saw_digit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstSourceFile;
    use crate::model::{SourceFileMetadata, SymbolCatalog};
    use crate::parser::parse_source;
    use crate::scope::LexicalScopeModel;

    fn index_for_source(source: &str) -> SymbolIndex {
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let catalog =
            SymbolCatalog::from_ast_with_metadata(source, &ast, SourceFileMetadata::unknown());
        SymbolIndex::from_catalogs([&catalog])
    }

    fn analysis_for_source(source: &str) -> (Parse, SymbolIndex, LexicalScopeModel) {
        let parse = parse_source(source);
        let ast = AstSourceFile::new(source, &parse);
        let catalog =
            SymbolCatalog::from_ast_with_metadata(source, &ast, SourceFileMetadata::unknown());
        let index = SymbolIndex::from_catalogs([&catalog]);
        let scope = LexicalScopeModel::from_parse_and_index(&parse, &index);
        (parse, index, scope)
    }

    #[test]
    fn extracts_owner_names_from_raw_type_text() {
        assert_eq!(
            owner_type_from_type_text("ref Widget").as_deref(),
            Some("Widget")
        );
        assert_eq!(
            owner_type_from_type_text("notnull array<IEntity>").as_deref(),
            Some("array")
        );
        assert_eq!(
            owner_type_from_type_text("map<string, ref Widget>").as_deref(),
            Some("map")
        );
    }

    #[test]
    fn collection_index_results_preserve_element_owner_and_raw_type() {
        let array = collection_index_result_type("array<IEntity>").unwrap();
        assert_eq!(array.owner_type, "IEntity");
        assert_eq!(array.raw_type_text.as_deref(), Some("IEntity"));

        let map = collection_index_result_type("map<string, ref Widget>").unwrap();
        assert_eq!(map.owner_type, "Widget");
        assert_eq!(map.raw_type_text.as_deref(), Some("ref Widget"));
    }

    #[test]
    fn substitutes_generic_return_types_from_receiver_text() {
        let index = index_for_source(
            r#"
class MapLike<TKey, TValue>
{
    TValue Get(TKey key) {}
}
"#,
        );
        assert_eq!(
            substitute_generic_return_type(&index, "MapLike", "MapLike<string, Widget>", "TValue",)
                .as_deref(),
            Some("Widget")
        );
    }

    #[test]
    fn environment_infers_names_calls_members_indexes_new_and_casts() {
        let source = r#"
class Class
{
	static Managed Cast(Managed value);
}

class Widget
{
	void SetVisible(bool visible);
}

class array<Class T>
{
	T Get(int index);
}

Widget MakeWidget();

class Example
{
	array<Widget> m_Widgets;
	void Run()
	{
		auto fromCall = MakeWidget();
		Widget fromNew = new Widget();
		auto fromCast = Widget.Cast(fromNew);
		m_Widgets[0].SetVisible(true);
		fromCall.SetVisible(true);
		fromNew.SetVisible(true);
		fromCast.SetVisible(true);
	}
}
"#;
        let (parse, index, scope) = analysis_for_source(source);
        let environment = ExpressionTypeEnvironment::new(source, &index, &parse, &scope, None);

        for (needle, expected) in [
            ("fromCall.SetVisible", "Widget"),
            ("fromNew.SetVisible", "Widget"),
            ("fromCast.SetVisible", "Widget"),
            ("m_Widgets[0].SetVisible", "Widget"),
        ] {
            let offset = source.find(needle).unwrap() + needle.find('.').unwrap() - 1;
            let expression =
                crate::ast::smallest_expression_at_offset(source, &parse.root, offset).unwrap();
            let mut path = Vec::new();
            let inferred = environment
                .infer_expression_type(expression, offset, &mut path)
                .unwrap();
            assert_eq!(inferred.owner_type, expected, "{needle}: {path:?}");
        }
    }

    #[test]
    fn environment_infers_bool_null_and_hex_literals() {
        let source = r#"
class Example
{
	void Run()
	{
		bool enabled = true;
		bool disabled = false;
		Managed value = null;
		int color = 0xFFF22613;
	}
}
"#;
        let (parse, index, scope) = analysis_for_source(source);
        let environment = ExpressionTypeEnvironment::new(source, &index, &parse, &scope, None);

        for (needle, expected) in [
            ("true", "bool"),
            ("false", "bool"),
            ("null", "null"),
            ("0xFFF22613", "int"),
        ] {
            let offset = source.find(needle).unwrap();
            let expression =
                crate::ast::smallest_expression_at_offset(source, &parse.root, offset).unwrap();
            let mut path = Vec::new();
            let inferred = environment
                .infer_expression_type(expression, offset, &mut path)
                .unwrap();
            assert_eq!(inferred.owner_type, expected, "{needle}: {path:?}");
        }
    }

    #[test]
    fn class_static_array_field_receiver_uses_element_type() {
        let source = r#"
class Sector
{
	vector m_vEstimatedPos;
}

class Example
{
	protected ref Sector m_aSectors[2];

	vector Run(int index)
	{
		return m_aSectors[index].m_vEstimatedPos;
	}
}
"#;
        let (parse, index, scope) = analysis_for_source(source);
        let environment = ExpressionTypeEnvironment::new(source, &index, &parse, &scope, None);
        let member = "m_vEstimatedPos";
        let offset = source.rfind(member).unwrap();
        let expression = crate::ast::member_access_for_member_name_at_offset(
            source,
            &parse.root,
            TextSpan::new(offset, offset + member.len()),
        )
        .unwrap()
        .expression;
        let mut path = Vec::new();
        let inferred = environment
            .infer_expression_type(expression, offset, &mut path)
            .unwrap();
        assert_eq!(inferred.owner_type, "vector", "{path:?}");
    }

    #[test]
    fn unterminated_static_array_field_receiver_uses_element_type() {
        let source = r#"class Example
{
	protected vector m_Target[4]

	//-------------------------------------------------------------------------
	//! Calculates the position.
	protected void Run()
	{
		m_Target[0].ToString();
	}
}
"#;
        let (parse, index, scope) = analysis_for_source(source);
        let environment = ExpressionTypeEnvironment::new(source, &index, &parse, &scope, None);
        let member = "ToString";
        let offset = source.rfind(member).unwrap();
        let expression = crate::ast::member_access_for_member_name_at_offset(
            source,
            &parse.root,
            TextSpan::new(offset, offset + member.len()),
        )
        .unwrap()
        .receiver;
        let mut path = Vec::new();
        let inferred = environment
            .infer_expression_type(expression, offset, &mut path)
            .unwrap();
        assert_eq!(inferred.owner_type, "vector", "{path:?}");
    }

    #[test]
    fn constructor_style_type_call_infers_instance_receiver() {
        let source = r#"
class TStringArray
{
}

class SCR_AIAction
{
	TStringArray GetPortNames();
}

class SCR_AIResupplyActivity : SCR_AIAction
{
}

class Example
{
	void Run()
	{
		SCR_AIResupplyActivity(null, null, null, Class).GetPortNames();
	}
}
"#;
        let (parse, index, scope) = analysis_for_source(source);
        let environment = ExpressionTypeEnvironment::new(source, &index, &parse, &scope, None);
        let member = "GetPortNames";
        let offset = source.rfind(member).unwrap();
        let expression = crate::ast::member_access_for_member_name_at_offset(
            source,
            &parse.root,
            TextSpan::new(offset, offset + member.len()),
        )
        .unwrap()
        .expression;
        let mut path = Vec::new();
        let inferred = environment
            .infer_expression_type(expression, offset, &mut path)
            .unwrap();
        assert_eq!(inferred.owner_type, "TStringArray", "{path:?}");
        assert!(
            path.iter()
                .any(|entry| entry.contains("constructor-style type call")),
            "{path:?}"
        );
    }

    #[test]
    fn auto_local_member_access_default_uses_full_default_expression() {
        let source = r#"
class InfoComponent
{
	int GetAIState();
}

class Agent
{
	InfoComponent m_InfoComponent;
}

class Example
{
	void Run(Agent agent)
	{
		auto infoComponent = agent.m_InfoComponent;
		infoComponent.GetAIState();
	}
}
"#;
        let (parse, index, scope) = analysis_for_source(source);
        let environment = ExpressionTypeEnvironment::new(source, &index, &parse, &scope, None);
        let member = "GetAIState";
        let offset = source.rfind(member).unwrap();
        let expression = crate::ast::member_access_for_member_name_at_offset(
            source,
            &parse.root,
            TextSpan::new(offset, offset + member.len()),
        )
        .unwrap()
        .expression;
        let mut path = Vec::new();
        let inferred = environment
            .infer_expression_type(expression, offset, &mut path)
            .unwrap();
        assert_eq!(inferred.owner_type, "int", "{path:?}");
    }
}
