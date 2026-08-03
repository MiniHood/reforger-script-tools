use reforger_language_server::ast::Expression;
use reforger_language_server::expression_type::ExpressionTypeEnvironment;
use reforger_language_server::index::SymbolIndex;
use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::model::{SourceFileMetadata, SourceKind, SOURCE_PRIORITY_GAME_DATA};
use reforger_language_server::parser::parse_source;
use reforger_language_server::scope::LexicalScopeModel;
use reforger_language_server::semantic_file::SemanticFile;
use reforger_language_server::syntax::{SyntaxElement, SyntaxKind, SyntaxNode};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/expression-type-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_ROWS: usize = 50;
const MAX_SAMPLES: usize = 60;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    if !args.scripts_path.is_dir() {
        return Err(format!(
            "Scripts folder does not exist: {}",
            args.scripts_path.display()
        ));
    }

    let external_start = Instant::now();
    let external_index = if args.external_index {
        Some(
            build_index(&IndexBuildConfig {
                roots: vec![IndexSourceRoot::new(
                    &args.scripts_path,
                    SourceKind::GameData,
                    SOURCE_PRIORITY_GAME_DATA,
                )],
            })
            .map(|result| result.index)?,
        )
    } else {
        None
    };
    let external_elapsed = external_start.elapsed().as_millis();

    let scan_start = Instant::now();
    let report = render_report(&args, external_index.as_ref(), external_elapsed)?;
    let scan_elapsed = scan_start.elapsed().as_millis();

    if let Some(parent) = args.out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &args.out_path,
        report.replace("{{SCAN_MS}}", &scan_elapsed.to_string()),
    )
    .map_err(|error| format!("Failed to write {}: {error}", args.out_path.display()))?;

    println!("Wrote {}", args.out_path.display());
    Ok(())
}

#[derive(Debug)]
struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
    max_files: Option<usize>,
    external_index: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut scripts_path = None;
        let mut out_path = None;
        let mut max_files = None;
        let mut external_index = true;
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scripts" => {
                    scripts_path = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--scripts requires a path".to_string())?,
                    ));
                }
                "--out" => {
                    out_path = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--out requires a path".to_string())?,
                    ));
                }
                "--max-files" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--max-files requires a number".to_string())?;
                    max_files = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("Invalid --max-files value: {value}"))?,
                    );
                }
                "--no-external-index" => external_index = false,
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }

        Ok(Self {
            scripts_path: scripts_path.unwrap_or_else(default_scripts_path),
            out_path: resolve_repo_path(out_path, DEFAULT_REPORT_RELATIVE_PATH),
            max_files,
            external_index,
        })
    }
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    lossy_files: usize,
    parse_diagnostics: usize,
    expressions: usize,
    inferred: usize,
    unresolved: usize,
    actionable_unresolved: usize,
}

#[derive(Clone)]
struct TypeSample {
    path: String,
    line: usize,
    kind: String,
    role: String,
    parent: String,
    owner: String,
    category: String,
    reason: String,
    snippet: String,
    lookup_path: String,
}

#[derive(Clone)]
struct ExpressionContext {
    role: String,
    parent_kind: String,
    parent_expression_inferred: bool,
    declaration_context: Option<String>,
}

impl ExpressionContext {
    fn root() -> Self {
        Self {
            role: "root".to_string(),
            parent_kind: "<none>".to_string(),
            parent_expression_inferred: false,
            declaration_context: None,
        }
    }
}

fn render_report(
    args: &Args,
    external_index: Option<&SymbolIndex>,
    external_elapsed: u128,
) -> Result<String, String> {
    let mut files = Vec::new();
    collect_script_files(&args.scripts_path, &mut files)?;
    files.sort();
    if let Some(max_files) = args.max_files {
        files.truncate(max_files);
    }

    let mut totals = Totals::default();
    let mut expression_kind_counts = BTreeMap::<String, usize>::new();
    let mut inferred_kind_counts = BTreeMap::<String, usize>::new();
    let mut unresolved_kind_counts = BTreeMap::<String, usize>::new();
    let mut owner_counts = BTreeMap::<String, usize>::new();
    let mut role_counts = BTreeMap::<String, usize>::new();
    let mut unresolved_reason_counts = BTreeMap::<String, usize>::new();
    let mut unresolved_category_counts = BTreeMap::<String, usize>::new();
    let mut unresolved_review_counts = BTreeMap::<String, usize>::new();
    let mut unresolved_category_samples = BTreeMap::<String, Vec<TypeSample>>::new();
    let mut inferred_samples = Vec::<TypeSample>::new();
    let mut unresolved_samples = Vec::<TypeSample>::new();
    let mut chain_samples = Vec::<TypeSample>::new();
    let mut generic_index_cast_samples = Vec::<TypeSample>::new();

    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        totals.files += 1;
        totals.bytes += bytes.len();
        let source = String::from_utf8_lossy(&bytes);
        if matches!(source, Cow::Owned(_)) {
            totals.lossy_files += 1;
        }
        let source = source.into_owned();
        let parse = parse_source(&source);
        totals.parse_diagnostics += parse.diagnostics.len();
        let semantic_file = SemanticFile::build(&source, &parse);
        let index =
            SymbolIndex::from_semantic_files([(&semantic_file, SourceFileMetadata::unknown())]);
        let scope = LexicalScopeModel::from_parse_and_index(&parse, &index);
        let environment =
            ExpressionTypeEnvironment::new(&source, &index, &parse, &scope, external_index);
        let relative_path = relative_display(file, &args.scripts_path);

        collect_expression_types(
            &source,
            &parse.root,
            &relative_path,
            &environment,
            &mut totals,
            &mut expression_kind_counts,
            &mut inferred_kind_counts,
            &mut unresolved_kind_counts,
            &mut owner_counts,
            &mut role_counts,
            &mut unresolved_reason_counts,
            &mut unresolved_category_counts,
            &mut unresolved_review_counts,
            &mut unresolved_category_samples,
            &mut inferred_samples,
            &mut unresolved_samples,
            &mut chain_samples,
            &mut generic_index_cast_samples,
            ExpressionContext::root(),
        );
    }

    let mut report = String::new();
    report.push_str("# Expression Type Corpus Report\n\n");
    report.push_str("This report samples source-backed `ExpressionTypeEnvironment` inference across game-data expression syntax. It is review evidence for resolver/type-environment gaps, not semantic validation. Workbench remains compiler truth.\n\n");
    report.push_str("## Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Source path | `{}` |\n",
        args.scripts_path.display()
    ));
    report.push_str(&format!(
        "| Scan timestamp unix seconds | {} |\n",
        timestamp()
    ));
    report.push_str(&format!("| Files | {} |\n", totals.files));
    report.push_str(&format!("| Bytes | {} |\n", totals.bytes));
    report.push_str(&format!("| Lossy files | {} |\n", totals.lossy_files));
    report.push_str(&format!(
        "| Parse diagnostics | {} |\n",
        totals.parse_diagnostics
    ));
    report.push_str(&format!(
        "| Expressions sampled | {} |\n",
        totals.expressions
    ));
    report.push_str(&format!("| Inferred expressions | {} |\n", totals.inferred));
    report.push_str(&format!(
        "| Unresolved expressions | {} |\n",
        totals.unresolved
    ));
    report.push_str(&format!(
        "| Actionable unresolved expressions | {} |\n",
        totals.actionable_unresolved
    ));
    report.push_str(&format!(
        "| Inference coverage | {:.2}% |\n",
        percent(totals.inferred, totals.expressions)
    ));
    report.push_str(&format!(
        "| External index | `{}` |\n",
        if args.external_index {
            "enabled"
        } else {
            "disabled"
        }
    ));
    report.push_str(&format!(
        "| External index build ms | {external_elapsed} |\n"
    ));
    report.push_str("| Scan/render ms | {{SCAN_MS}} |\n\n");

    append_counts(
        &mut report,
        "Expression Kind Frequency",
        &expression_kind_counts,
    );
    append_counts(
        &mut report,
        "Inferred Expression Kind Frequency",
        &inferred_kind_counts,
    );
    append_counts(
        &mut report,
        "Unresolved Expression Kind Frequency",
        &unresolved_kind_counts,
    );
    append_counts(&mut report, "Top Inferred Owner Types", &owner_counts);
    append_counts(&mut report, "Expression Role Frequency", &role_counts);
    append_counts(
        &mut report,
        "Unresolved Reason Frequency",
        &unresolved_reason_counts,
    );
    append_counts(
        &mut report,
        "Unresolved Classification",
        &unresolved_category_counts,
    );
    append_counts(
        &mut report,
        "Unresolved Review Buckets",
        &unresolved_review_counts,
    );
    append_sample_groups(
        &mut report,
        "Actionable / Review Unresolved Samples By Classification",
        &unresolved_category_samples,
        &[
            "probable expression-type defect",
            "unresolved name/type fact",
            "unresolved receiver/member chain",
            "missing external/native API fact",
            "declaration/type syntax",
        ],
    );
    append_samples(&mut report, "Inferred Samples", &inferred_samples);
    append_samples(&mut report, "Unresolved Samples", &unresolved_samples);
    append_samples(
        &mut report,
        "Deep Member / Call / Index Chain Type Samples",
        &chain_samples,
    );
    append_samples(
        &mut report,
        "Generic / Index / Cast Type Samples",
        &generic_index_cast_samples,
    );

    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn collect_expression_types(
    source: &str,
    node: &SyntaxNode,
    path: &str,
    environment: &ExpressionTypeEnvironment<'_, '_>,
    totals: &mut Totals,
    expression_kind_counts: &mut BTreeMap<String, usize>,
    inferred_kind_counts: &mut BTreeMap<String, usize>,
    unresolved_kind_counts: &mut BTreeMap<String, usize>,
    owner_counts: &mut BTreeMap<String, usize>,
    role_counts: &mut BTreeMap<String, usize>,
    unresolved_reason_counts: &mut BTreeMap<String, usize>,
    unresolved_category_counts: &mut BTreeMap<String, usize>,
    unresolved_review_counts: &mut BTreeMap<String, usize>,
    unresolved_category_samples: &mut BTreeMap<String, Vec<TypeSample>>,
    inferred_samples: &mut Vec<TypeSample>,
    unresolved_samples: &mut Vec<TypeSample>,
    chain_samples: &mut Vec<TypeSample>,
    generic_index_cast_samples: &mut Vec<TypeSample>,
    context: ExpressionContext,
) {
    let mut current_expression_inferred = false;
    if let Some(expression) = Expression::from_node(source, node) {
        let kind = format!("{:?}", expression.kind());
        totals.expressions += 1;
        *expression_kind_counts.entry(kind.clone()).or_default() += 1;
        *role_counts.entry(context.role.clone()).or_default() += 1;
        let mut lookup_path = Vec::new();
        let offset = expression.span().end.saturating_sub(1);
        if let Some(inferred) =
            environment.infer_expression_type(expression, offset, &mut lookup_path)
        {
            current_expression_inferred = true;
            totals.inferred += 1;
            *inferred_kind_counts.entry(kind.clone()).or_default() += 1;
            *owner_counts.entry(inferred.owner_type.clone()).or_default() += 1;
            let sample = sample_for_expression(
                source,
                path,
                expression,
                &context,
                inferred.owner_type,
                "inferred".to_string(),
                "inferred".to_string(),
                lookup_path,
            );
            push_bounded(inferred_samples, sample.clone());
            if chain_depth(node) >= 3 {
                push_bounded(chain_samples, sample.clone());
            }
            if is_generic_index_or_cast_sample(expression) {
                push_bounded(generic_index_cast_samples, sample);
            }
        } else {
            totals.unresolved += 1;
            *unresolved_kind_counts.entry(kind.clone()).or_default() += 1;
            let reason = unresolved_reason(expression);
            let category = unresolved_category(expression, &reason, &context);
            *unresolved_reason_counts.entry(reason.clone()).or_default() += 1;
            *unresolved_category_counts
                .entry(category.clone())
                .or_default() += 1;
            *unresolved_review_counts
                .entry(unresolved_review_bucket(&category).to_string())
                .or_default() += 1;
            if is_actionable_unresolved_category(&category) {
                totals.actionable_unresolved += 1;
            }
            let sample = sample_for_expression(
                source,
                path,
                expression,
                &context,
                "<none>".to_string(),
                category.clone(),
                reason,
                lookup_path,
            );
            push_bounded(unresolved_samples, sample.clone());
            push_bounded(
                unresolved_category_samples.entry(category).or_default(),
                sample,
            );
        }
    }

    let parent_expression_inferred_for_children = current_expression_inferred;
    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            let child_context = context_for_child_expression(
                source,
                node,
                child,
                &context,
                parent_expression_inferred_for_children,
            );
            collect_expression_types(
                source,
                child,
                path,
                environment,
                totals,
                expression_kind_counts,
                inferred_kind_counts,
                unresolved_kind_counts,
                owner_counts,
                role_counts,
                unresolved_reason_counts,
                unresolved_category_counts,
                unresolved_review_counts,
                unresolved_category_samples,
                inferred_samples,
                unresolved_samples,
                chain_samples,
                generic_index_cast_samples,
                child_context,
            );
        }
    }
}

fn sample_for_expression(
    source: &str,
    path: &str,
    expression: Expression<'_, '_>,
    context: &ExpressionContext,
    owner: String,
    category: String,
    reason: String,
    lookup_path: Vec<String>,
) -> TypeSample {
    TypeSample {
        path: path.to_string(),
        line: line_for_offset(source, expression.span().start),
        kind: format!("{:?}", expression.kind()),
        role: context.role.clone(),
        parent: context.parent_kind.clone(),
        owner,
        category,
        reason,
        snippet: snippet_for_span(source, expression.span()),
        lookup_path: lookup_path.join(" -> "),
    }
}

fn context_for_child_expression(
    source: &str,
    parent: &SyntaxNode,
    child: &SyntaxNode,
    parent_context: &ExpressionContext,
    parent_expression_inferred: bool,
) -> ExpressionContext {
    let role = expression_role_for_child(source, parent, child);
    let inherited_declaration_context = if parent_context.declaration_context.is_some()
        && matches!(
            parent.kind,
            SyntaxKind::Parameter
                | SyntaxKind::TypeRef
                | SyntaxKind::GenericArgList
                | SyntaxKind::TypedefDecl
        )
        || is_expression_syntax_kind(parent.kind)
    {
        parent_context.declaration_context.clone()
    } else {
        None
    };
    let declaration_context = if is_declaration_type_context(parent.kind) {
        Some(format!("{:?}", parent.kind))
    } else {
        inherited_declaration_context
    };
    ExpressionContext {
        role,
        parent_kind: format!("{:?}", parent.kind),
        parent_expression_inferred,
        declaration_context,
    }
}

fn expression_role_for_child(source: &str, parent: &SyntaxNode, child: &SyntaxNode) -> String {
    if Expression::from_node(source, child).is_none() {
        return "non-expression-child".to_string();
    }
    match parent.kind {
        SyntaxKind::ExpressionStatement | SyntaxKind::ReturnStatement => "standalone value",
        SyntaxKind::LocalDeclStatement | SyntaxKind::ForInitializer => "declaration/default value",
        SyntaxKind::NamedArgument => {
            if first_expression_child_node(parent).is_some_and(|first| std::ptr::eq(first, child)) {
                "named argument label"
            } else {
                "named argument value"
            }
        }
        SyntaxKind::ArgumentList => "call argument",
        SyntaxKind::CallExpression => {
            if first_expression_child_node(parent).is_some_and(|first| std::ptr::eq(first, child)) {
                "callee"
            } else {
                "call child"
            }
        }
        SyntaxKind::MemberAccessExpression => {
            if first_expression_child_node(parent).is_some_and(|first| std::ptr::eq(first, child)) {
                "member receiver"
            } else if child.kind == SyntaxKind::NameExpression {
                "member name"
            } else {
                "member child"
            }
        }
        SyntaxKind::IndexExpression => {
            if first_expression_child_node(parent).is_some_and(|first| std::ptr::eq(first, child)) {
                "index receiver"
            } else {
                "index argument"
            }
        }
        SyntaxKind::ParenthesizedExpression => "parenthesized child",
        SyntaxKind::UnaryExpression | SyntaxKind::PostfixExpression => "unary operand",
        SyntaxKind::BinaryExpression => "binary operand",
        SyntaxKind::AssignmentExpression => {
            if first_expression_child_node(parent).is_some_and(|first| std::ptr::eq(first, child)) {
                "assignment target"
            } else {
                "assignment value"
            }
        }
        SyntaxKind::TernaryExpression => "ternary child",
        SyntaxKind::InitializerExpression => "initializer element",
        _ => "nested expression",
    }
    .to_string()
}

fn first_expression_child_node(parent: &SyntaxNode) -> Option<&SyntaxNode> {
    parent.children.iter().find_map(|child| match child {
        SyntaxElement::Node(node) if is_expression_syntax_kind(node.kind) => Some(node.as_ref()),
        SyntaxElement::Node(_) | SyntaxElement::Token(_) => None,
    })
}

fn is_expression_syntax_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Expression
            | SyntaxKind::NameExpression
            | SyntaxKind::LiteralExpression
            | SyntaxKind::ParenthesizedExpression
            | SyntaxKind::UnaryExpression
            | SyntaxKind::BinaryExpression
            | SyntaxKind::AssignmentExpression
            | SyntaxKind::TernaryExpression
            | SyntaxKind::CallExpression
            | SyntaxKind::NamedArgument
            | SyntaxKind::MemberAccessExpression
            | SyntaxKind::IndexExpression
            | SyntaxKind::CastExpression
            | SyntaxKind::PostfixExpression
            | SyntaxKind::NewExpression
            | SyntaxKind::InitializerExpression
    )
}

fn is_declaration_type_context(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Parameter
            | SyntaxKind::TypeRef
            | SyntaxKind::GenericArgList
            | SyntaxKind::TypedefDecl
    )
}

fn unresolved_reason(expression: Expression<'_, '_>) -> String {
    match expression {
        Expression::Name(_) => "name has no visible type fact".to_string(),
        Expression::MemberAccess(_) => {
            "member receiver or member return type unresolved".to_string()
        }
        Expression::Call(_) => "call return type unresolved".to_string(),
        Expression::Index(_) => "indexed receiver element type unresolved".to_string(),
        Expression::Initializer(_) => "initializer expression has no owner type".to_string(),
        Expression::Ternary(_) => "ternary branch type not inferred".to_string(),
        Expression::Assignment(_) => "assignment result type not inferred".to_string(),
        Expression::Unknown(_) => "unknown/container expression".to_string(),
        _ => "expression type not inferred".to_string(),
    }
}

fn unresolved_category(
    expression: Expression<'_, '_>,
    reason: &str,
    context: &ExpressionContext,
) -> String {
    let text = expression.source_text().trim();
    if is_named_argument_label_expression(text) {
        return "named argument label/source-noise".to_string();
    }
    if context.role == "named argument label" {
        return "named argument label/source-noise".to_string();
    }
    if is_attribute_like_expression(text) {
        return "attribute argument/source-noise".to_string();
    }
    if context.declaration_context.is_some()
        && matches!(
            context.role.as_str(),
            "nested expression" | "declaration/default value"
        )
    {
        return "declaration/type syntax".to_string();
    }
    if context.parent_expression_inferred
        && matches!(
            context.role.as_str(),
            "callee" | "member receiver" | "member name" | "index receiver" | "call child"
        )
    {
        return "typed by parent expression".to_string();
    }
    match expression {
        Expression::Unknown(_) | Expression::Initializer(_) | Expression::Assignment(_) => {
            "expected non-value/container expression".to_string()
        }
        Expression::Ternary(_) => "unsupported expression result type".to_string(),
        Expression::MemberAccess(_) | Expression::Call(_) | Expression::Index(_) => {
            if reason.contains("member receiver") || reason.contains("receiver") {
                "unresolved receiver/member chain".to_string()
            } else if looks_like_external_or_native_gap(text) {
                "missing external/native API fact".to_string()
            } else {
                "probable expression-type defect".to_string()
            }
        }
        Expression::Name(_) => {
            if looks_like_external_or_native_gap(text) {
                "missing external/native API fact".to_string()
            } else {
                "unresolved name/type fact".to_string()
            }
        }
        _ => "probable expression-type defect".to_string(),
    }
}

fn unresolved_review_bucket(category: &str) -> &'static str {
    match category {
        "expected non-value/container expression"
        | "named argument label/source-noise"
        | "attribute argument/source-noise"
        | "typed by parent expression"
        | "declaration/type syntax" => "expected/noise",
        "unresolved receiver/member chain" => "actionable receiver/member typing",
        "missing external/native API fact" => "source/API unavailable",
        "unresolved name/type fact" | "unsupported expression result type" => {
            "needs type-environment review"
        }
        _ => "probable type-environment defect",
    }
}

fn is_actionable_unresolved_category(category: &str) -> bool {
    matches!(
        unresolved_review_bucket(category),
        "actionable receiver/member typing"
            | "needs type-environment review"
            | "probable type-environment defect"
    )
}

fn is_named_argument_label_expression(text: &str) -> bool {
    let Some((left, right)) = text.split_once(':') else {
        return false;
    };
    !left.trim().is_empty()
        && !right.trim().is_empty()
        && left
            .trim()
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_attribute_like_expression(text: &str) -> bool {
    matches!(
        text,
        "desc" | "defvalue" | "uiwidget" | "params" | "category" | "configRoot"
    )
}

fn looks_like_external_or_native_gap(text: &str) -> bool {
    text.starts_with("super.")
        || text.starts_with("proto ")
        || text.contains("native")
        || text.contains("external")
        || text
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn is_generic_index_or_cast_sample(expression: Expression<'_, '_>) -> bool {
    let text = expression.source_text();
    matches!(expression, Expression::Index(_) | Expression::Cast(_))
        || text.contains('<')
        || text.contains(".Cast")
}

fn chain_depth(node: &SyntaxNode) -> usize {
    let own = usize::from(matches!(
        node.kind,
        SyntaxKind::MemberAccessExpression
            | SyntaxKind::CallExpression
            | SyntaxKind::IndexExpression
    ));
    own + node
        .children
        .iter()
        .filter_map(|child| match child {
            SyntaxElement::Node(child) => Some(chain_depth(child)),
            SyntaxElement::Token(_) => None,
        })
        .max()
        .unwrap_or(0)
}

fn append_counts(report: &mut String, title: &str, counts: &BTreeMap<String, usize>) {
    report.push_str(&format!("## {title}\n\n"));
    if counts.is_empty() {
        report.push_str("None.\n\n");
        return;
    }
    let mut rows = counts.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    report.push_str("| Value | Count |\n");
    report.push_str("| --- | ---: |\n");
    for (value, count) in rows.into_iter().take(MAX_ROWS) {
        report.push_str(&format!("| `{}` | {} |\n", escape_table(value), count));
    }
    report.push('\n');
}

fn append_samples(report: &mut String, title: &str, samples: &[TypeSample]) {
    report.push_str(&format!("## {title}\n\n"));
    if samples.is_empty() {
        report.push_str("None.\n\n");
        return;
    }
    report.push_str(
        "| Path | Line | Kind | Role | Parent | Owner | Category | Reason | Snippet | Lookup path |\n",
    );
    report.push_str("| --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for sample in samples.iter().take(MAX_ROWS) {
        report.push_str(&format!(
            "| `{}` | {} | `{}` | `{}` | `{}` | `{}` | `{}` | {} | `{}` | `{}` |\n",
            escape_table(&sample.path),
            sample.line,
            escape_table(&sample.kind),
            escape_table(&sample.role),
            escape_table(&sample.parent),
            escape_table(&sample.owner),
            escape_table(&sample.category),
            escape_table(&sample.reason),
            escape_table(&sample.snippet.replace('`', "\\`")),
            escape_table(&sample.lookup_path.replace('`', "\\`")),
        ));
    }
    report.push('\n');
}

fn append_sample_groups(
    report: &mut String,
    title: &str,
    samples: &BTreeMap<String, Vec<TypeSample>>,
    categories: &[&str],
) {
    report.push_str(&format!("## {title}\n\n"));
    let mut wrote_any = false;
    for category in categories {
        let Some(samples) = samples.get(*category) else {
            continue;
        };
        if samples.is_empty() {
            continue;
        }
        wrote_any = true;
        report.push_str(&format!("### `{}`\n\n", escape_table(category)));
        append_sample_table(report, samples);
    }
    if !wrote_any {
        report.push_str("None.\n\n");
    }
}

fn append_sample_table(report: &mut String, samples: &[TypeSample]) {
    report.push_str(
        "| Path | Line | Kind | Role | Parent | Owner | Category | Reason | Snippet | Lookup path |\n",
    );
    report.push_str("| --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for sample in samples.iter().take(MAX_ROWS) {
        report.push_str(&format!(
            "| `{}` | {} | `{}` | `{}` | `{}` | `{}` | `{}` | {} | `{}` | `{}` |\n",
            escape_table(&sample.path),
            sample.line,
            escape_table(&sample.kind),
            escape_table(&sample.role),
            escape_table(&sample.parent),
            escape_table(&sample.owner),
            escape_table(&sample.category),
            escape_table(&sample.reason),
            escape_table(&sample.snippet.replace('`', "\\`")),
            escape_table(&sample.lookup_path.replace('`', "\\`")),
        ));
    }
    report.push('\n');
}

fn push_bounded(samples: &mut Vec<TypeSample>, sample: TypeSample) {
    if samples.len() < MAX_SAMPLES {
        samples.push(sample);
    }
}

fn collect_script_files(folder: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Failed to read {}: {error}", folder.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to read entry in {}: {error}", folder.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_script_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "c") {
            files.push(path);
        }
    }
    Ok(())
}

fn snippet_for_span(source: &str, span: reforger_language_server::lexer::TextSpan) -> String {
    let start = source[..span.start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let end = source[span.start..]
        .find('\n')
        .map(|index| span.start + index)
        .unwrap_or(source.len());
    source[start..end].trim().replace('\t', "\\t")
}

fn line_for_offset(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn relative_display(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn percent(part: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / total as f64
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|")
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn default_scripts_path() -> PathBuf {
    if let Some(app_data) = env::var_os("APPDATA") {
        PathBuf::from(app_data).join(DEFAULT_STORAGE_RELATIVE_PATH)
    } else {
        PathBuf::from(DEFAULT_STORAGE_RELATIVE_PATH)
    }
}

fn resolve_repo_path(path: Option<PathBuf>, default_relative_path: &str) -> PathBuf {
    match path {
        Some(path) if path.is_absolute() => path,
        Some(path) => repo_root().join(path),
        None => repo_root().join(default_relative_path),
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("server crate should be inside the repository root")
        .to_path_buf()
}

fn print_help() {
    println!("Usage: cargo run --manifest-path server/Cargo.toml --example expression_type_corpus_report -- [--scripts <path>] [--out <path>] [--max-files <n>] [--no-external-index]");
}
