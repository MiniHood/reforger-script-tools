use reforger_language_server::ast::Expression;
use reforger_language_server::lexer::TextSpan;
use reforger_language_server::parser::parse_source;
use reforger_language_server::syntax::{SyntaxElement, SyntaxKind, SyntaxNode};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/expression-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_ROWS: usize = 50;

struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
}

#[derive(Default)]
struct FileStats {
    path: PathBuf,
    diagnostics: usize,
    error_nodes: usize,
    expected_error_nodes: usize,
    expression_depth: usize,
    expression_depth_snippet: Option<String>,
    chain_depth: usize,
    chain_depth_snippet: Option<String>,
    named_arguments: usize,
    initializer_expressions: usize,
    statement_nodes: usize,
    expression_nodes: usize,
    ast_expression_wrappers: usize,
    ast_expression_unknown_wrappers: usize,
    ast_expression_actionable_unknown_wrappers: usize,
    ast_expression_unknown_snippet: Option<String>,
    for_initializers: usize,
    for_decl_initializers: usize,
    for_expression_initializers: usize,
    foreach_headers: usize,
    foreach_variable_lists: usize,
    foreach_variables: usize,
    foreach_iterables: usize,
    switch_statements: usize,
    switch_sections: usize,
    case_clauses: usize,
    default_clauses: usize,
}

fn main() -> Result<(), String> {
    let args = parse_args()?;
    let report = render_report(&args.scripts_path)?;

    if let Some(parent) = args.out_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create report folder {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(&args.out_path, report).map_err(|error| {
        format!(
            "Failed to write expression corpus report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!(
        "Wrote expression corpus report: {}",
        args.out_path.display()
    );
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut scripts = None;
    let mut out = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scripts" => {
                let Some(value) = args.next() else {
                    return Err("--scripts requires a path".to_string());
                };
                scripts = Some(PathBuf::from(value));
            }
            "--out" => {
                let Some(value) = args.next() else {
                    return Err("--out requires a path".to_string());
                };
                out = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                println!("Usage: cargo run --manifest-path server/Cargo.toml --example expression_corpus_report -- [--scripts <path>] [--out <path>]");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }

    Ok(Args {
        scripts_path: scripts.unwrap_or_else(default_scripts_path),
        out_path: resolve_repo_path(out, DEFAULT_REPORT_RELATIVE_PATH),
    })
}

fn render_report(scripts_path: &Path) -> Result<String, String> {
    if !scripts_path.is_dir() {
        return Err(format!(
            "Scripts folder does not exist or is not a folder: {}",
            scripts_path.display()
        ));
    }

    let mut files = Vec::new();
    collect_script_files(scripts_path, &mut files)?;
    files.sort();

    let mut total_bytes = 0usize;
    let mut total_tokens = 0usize;
    let mut total_diagnostics = 0usize;
    let mut lossy_files = 0usize;
    let mut kind_counts = BTreeMap::<String, usize>::new();
    let mut wrapper_kind_counts = BTreeMap::<String, usize>::new();
    let mut unknown_wrapper_syntax_counts = BTreeMap::<String, usize>::new();
    let mut named_argument_labels = BTreeMap::<String, usize>::new();
    let mut file_stats = Vec::<FileStats>::new();

    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        total_bytes += bytes.len();
        let source = String::from_utf8_lossy(&bytes);
        if matches!(source, Cow::Owned(_)) {
            lossy_files += 1;
        }
        let source = source.into_owned();
        let parse = parse_source(&source);

        let (expression_depth, expression_depth_span) = max_expression_depth_with_span(&parse.root);
        let (chain_depth, chain_depth_span) = max_member_call_index_chain_with_span(&parse.root);
        let mut stats = FileStats {
            path: file.clone(),
            diagnostics: parse.diagnostics.len(),
            expected_error_nodes: expected_recovery_node_count(&parse.root, &source, file),
            expression_depth,
            expression_depth_snippet: expression_depth_span
                .map(|span| snippet_for_span(&source, span, 1)),
            chain_depth,
            chain_depth_snippet: chain_depth_span.map(|span| snippet_for_span(&source, span, 1)),
            ..FileStats::default()
        };
        total_tokens += parse.root.token_count();
        total_diagnostics += parse.diagnostics.len();
        collect_stats(
            &source,
            &parse.root,
            &mut kind_counts,
            &mut wrapper_kind_counts,
            &mut unknown_wrapper_syntax_counts,
            &mut named_argument_labels,
            &mut stats,
        );
        file_stats.push(stats);
    }

    let mut report = String::new();
    report.push_str("# Expression Corpus Report\n\n");
    report.push_str(
        "> Human-review output generated by `node tools/expression-corpus-report.mjs`.\n\n",
    );
    report.push_str("This report summarizes statement/expression parser coverage across game-data scripts. It is review data only; Workbench remains compiler truth.\n\n");

    report.push_str("## Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Source path | `{}` |\n", scripts_path.display()));
    report.push_str(&format!(
        "| Scan timestamp unix seconds | {} |\n",
        timestamp()
    ));
    report.push_str(&format!("| `.c` files | {} |\n", files.len()));
    report.push_str(&format!("| Bytes | {} |\n", total_bytes));
    report.push_str(&format!("| Tokens preserved | {} |\n", total_tokens));
    report.push_str(&format!("| Diagnostics | {} |\n", total_diagnostics));
    report.push_str(&format!("| Files decoded lossily | {} |\n\n", lossy_files));

    append_counts(
        &mut report,
        "Statement / Expression Kind Frequency",
        &kind_counts,
    );
    append_expression_wrapper_coverage(
        &mut report,
        scripts_path,
        &file_stats,
        &wrapper_kind_counts,
        &unknown_wrapper_syntax_counts,
    );
    append_for_initializer_coverage(&mut report, &file_stats);
    append_foreach_header_coverage(&mut report, &file_stats);
    append_switch_section_coverage(&mut report, &file_stats);
    append_expected_recovery_nodes(&mut report, scripts_path, &file_stats);
    append_expression_depth_samples(&mut report, scripts_path, &file_stats);
    append_chain_depth_samples(&mut report, scripts_path, &file_stats);
    append_counts(
        &mut report,
        "Named Argument Label Frequency",
        &named_argument_labels,
    );
    append_top_files(
        &mut report,
        scripts_path,
        "Top Files By Expression Depth",
        &file_stats,
        |stats| stats.expression_depth,
    );
    append_top_files(
        &mut report,
        scripts_path,
        "Top Files By Named Arguments",
        &file_stats,
        |stats| stats.named_arguments,
    );
    append_top_files(
        &mut report,
        scripts_path,
        "Top Files By Initializer Expressions",
        &file_stats,
        |stats| stats.initializer_expressions,
    );
    append_top_files(
        &mut report,
        scripts_path,
        "Top Files By Recovery/Error Nodes",
        &file_stats,
        |stats| stats.error_nodes,
    );

    Ok(report)
}

fn collect_script_files(folder: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Failed to read folder {}: {error}", folder.display()))?
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

fn collect_stats(
    source: &str,
    node: &SyntaxNode,
    kind_counts: &mut BTreeMap<String, usize>,
    wrapper_kind_counts: &mut BTreeMap<String, usize>,
    unknown_wrapper_syntax_counts: &mut BTreeMap<String, usize>,
    named_argument_labels: &mut BTreeMap<String, usize>,
    stats: &mut FileStats,
) {
    if is_statement_or_expression_kind(node.kind) {
        *kind_counts.entry(format!("{:?}", node.kind)).or_default() += 1;
    }
    if is_statement_kind(node.kind) {
        stats.statement_nodes += 1;
    }
    if is_expression_kind(node.kind) {
        stats.expression_nodes += 1;
        if let Some(expression) = Expression::from_node(source, node) {
            stats.ast_expression_wrappers += 1;
            *wrapper_kind_counts
                .entry(format!("{:?}", expression.kind()))
                .or_default() += 1;
            if matches!(
                expression.kind(),
                reforger_language_server::ast::ExpressionKind::Unknown
            ) {
                stats.ast_expression_unknown_wrappers += 1;
                *unknown_wrapper_syntax_counts
                    .entry(format!("{:?}", node.kind))
                    .or_default() += 1;
                if !is_expected_unknown_wrapper_syntax_kind(node.kind) {
                    stats.ast_expression_actionable_unknown_wrappers += 1;
                    stats
                        .ast_expression_unknown_snippet
                        .get_or_insert_with(|| snippet_for_span(source, node.span, 1));
                }
            }
        }
    }
    match node.kind {
        SyntaxKind::Error => stats.error_nodes += 1,
        SyntaxKind::NamedArgument => {
            stats.named_arguments += 1;
            if let Some(label) = named_argument_label(source, node) {
                *named_argument_labels.entry(label).or_default() += 1;
            }
        }
        SyntaxKind::InitializerExpression => stats.initializer_expressions += 1,
        SyntaxKind::ForInitializer => {
            stats.for_initializers += 1;
            if direct_child_node_count(node, SyntaxKind::LocalDeclStatement) > 0 {
                stats.for_decl_initializers += 1;
            } else {
                stats.for_expression_initializers += 1;
            }
        }
        SyntaxKind::ForeachHeader => stats.foreach_headers += 1,
        SyntaxKind::ForeachVariableList => stats.foreach_variable_lists += 1,
        SyntaxKind::ForeachVariable => stats.foreach_variables += 1,
        SyntaxKind::ForeachIterable => stats.foreach_iterables += 1,
        SyntaxKind::SwitchStatement => stats.switch_statements += 1,
        SyntaxKind::SwitchSection => stats.switch_sections += 1,
        SyntaxKind::CaseClause => stats.case_clauses += 1,
        SyntaxKind::DefaultClause => stats.default_clauses += 1,
        _ => {}
    }
    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            collect_stats(
                source,
                child,
                kind_counts,
                wrapper_kind_counts,
                unknown_wrapper_syntax_counts,
                named_argument_labels,
                stats,
            );
        }
    }
}

fn append_expression_wrapper_coverage(
    report: &mut String,
    root: &Path,
    file_stats: &[FileStats],
    wrapper_kind_counts: &BTreeMap<String, usize>,
    unknown_wrapper_syntax_counts: &BTreeMap<String, usize>,
) {
    let parser_expression_nodes = file_stats
        .iter()
        .map(|stats| stats.expression_nodes)
        .sum::<usize>();
    let ast_wrappers = file_stats
        .iter()
        .map(|stats| stats.ast_expression_wrappers)
        .sum::<usize>();
    let unknown_wrappers = file_stats
        .iter()
        .map(|stats| stats.ast_expression_unknown_wrappers)
        .sum::<usize>();
    let actionable_unknown_wrappers = file_stats
        .iter()
        .map(|stats| stats.ast_expression_actionable_unknown_wrappers)
        .sum::<usize>();
    let non_wrapper_nodes = parser_expression_nodes.saturating_sub(ast_wrappers);
    let expected_unknown_wrappers = unknown_wrappers.saturating_sub(actionable_unknown_wrappers);

    report.push_str("## Expression AST Wrapper Coverage\n\n");
    report.push_str("This section compares parser expression syntax nodes with source-backed AST `Expression` wrappers. Non-wrapper nodes are parser containers such as `ArgumentList`. Expected `Unknown` wrappers are generic expression/named-argument containers; actionable unknown wrappers would indicate syntax accepted by the wrapper API but not mapped to a specific expression variant.\n\n");
    report.push_str("| Metric | Count |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Parser expression syntax nodes | {parser_expression_nodes} |\n"
    ));
    report.push_str(&format!("| AST expression wrappers | {ast_wrappers} |\n"));
    report.push_str(&format!(
        "| Parser expression nodes without wrappers | {non_wrapper_nodes} |\n"
    ));
    report.push_str(&format!(
        "| `Unknown` AST wrappers | {unknown_wrappers} |\n\n"
    ));
    report.push_str(&format!(
        "| Expected container `Unknown` wrappers | {expected_unknown_wrappers} |\n"
    ));
    report.push_str(&format!(
        "| Actionable `Unknown` wrapper gaps | {actionable_unknown_wrappers} |\n\n"
    ));

    append_counts(
        report,
        "Expression AST Wrapper Variant Frequency",
        wrapper_kind_counts,
    );
    append_counts(
        report,
        "Unknown Expression Wrapper Syntax Kinds",
        unknown_wrapper_syntax_counts,
    );

    let mut rows = file_stats
        .iter()
        .filter(|stats| stats.ast_expression_actionable_unknown_wrappers > 0)
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .ast_expression_actionable_unknown_wrappers
            .cmp(&left.ast_expression_actionable_unknown_wrappers)
            .then_with(|| left.path.cmp(&right.path))
    });

    report.push_str("## Actionable Unknown Expression Wrapper Samples\n\n");
    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Unknown wrappers | Snippet |\n");
    report.push_str("| --- | ---: | --- |\n");
    for stats in rows.into_iter().take(25) {
        report.push_str(&format!(
            "| `{}` | {} | `{}` |\n",
            relative_path(root, &stats.path),
            stats.ast_expression_actionable_unknown_wrappers,
            escape_table(
                &stats
                    .ast_expression_unknown_snippet
                    .as_deref()
                    .unwrap_or("")
                    .replace('`', "\\`")
            )
        ));
    }
    report.push('\n');
}

fn is_expected_unknown_wrapper_syntax_kind(kind: SyntaxKind) -> bool {
    matches!(kind, SyntaxKind::Expression | SyntaxKind::NamedArgument)
}

fn append_for_initializer_coverage(report: &mut String, file_stats: &[FileStats]) {
    let total = file_stats
        .iter()
        .map(|stats| stats.for_initializers)
        .sum::<usize>();
    let declarations = file_stats
        .iter()
        .map(|stats| stats.for_decl_initializers)
        .sum::<usize>();
    let expressions = file_stats
        .iter()
        .map(|stats| stats.for_expression_initializers)
        .sum::<usize>();

    report.push_str("## For Initializer Shape Coverage\n\n");
    report.push_str("| Shape | Count |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Total `ForInitializer` nodes | {total} |\n"));
    report.push_str(&format!(
        "| Declaration-shaped with nested `LocalDeclStatement` | {declarations} |\n"
    ));
    report.push_str(&format!(
        "| Expression-shaped initializer lists | {expressions} |\n\n"
    ));
}

fn append_foreach_header_coverage(report: &mut String, file_stats: &[FileStats]) {
    let headers = file_stats
        .iter()
        .map(|stats| stats.foreach_headers)
        .sum::<usize>();
    let lists = file_stats
        .iter()
        .map(|stats| stats.foreach_variable_lists)
        .sum::<usize>();
    let variables = file_stats
        .iter()
        .map(|stats| stats.foreach_variables)
        .sum::<usize>();
    let iterables = file_stats
        .iter()
        .map(|stats| stats.foreach_iterables)
        .sum::<usize>();

    report.push_str("## Foreach Header Shape Coverage\n\n");
    report.push_str("| Shape | Count |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| `ForeachHeader` nodes | {headers} |\n"));
    report.push_str(&format!("| `ForeachVariableList` nodes | {lists} |\n"));
    report.push_str(&format!("| `ForeachVariable` nodes | {variables} |\n"));
    report.push_str(&format!("| `ForeachIterable` nodes | {iterables} |\n"));
    report.push_str(&format!(
        "| Headers without iterable node | {} |\n\n",
        headers.saturating_sub(iterables)
    ));
}

fn append_switch_section_coverage(report: &mut String, file_stats: &[FileStats]) {
    let switches = file_stats
        .iter()
        .map(|stats| stats.switch_statements)
        .sum::<usize>();
    let sections = file_stats
        .iter()
        .map(|stats| stats.switch_sections)
        .sum::<usize>();
    let cases = file_stats
        .iter()
        .map(|stats| stats.case_clauses)
        .sum::<usize>();
    let defaults = file_stats
        .iter()
        .map(|stats| stats.default_clauses)
        .sum::<usize>();

    report.push_str("## Switch Section Coverage\n\n");
    report.push_str("| Shape | Count |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| `SwitchStatement` nodes | {switches} |\n"));
    report.push_str(&format!("| `SwitchSection` nodes | {sections} |\n"));
    report.push_str(&format!("| `CaseClause` labels | {cases} |\n"));
    report.push_str(&format!("| `DefaultClause` labels | {defaults} |\n\n"));
}

fn append_expected_recovery_nodes(report: &mut String, root: &Path, file_stats: &[FileStats]) {
    let expected = file_stats
        .iter()
        .map(|stats| stats.expected_error_nodes)
        .sum::<usize>();
    let total = file_stats
        .iter()
        .map(|stats| stats.error_nodes)
        .sum::<usize>();

    report.push_str("## Expected Recovery Nodes\n\n");
    report.push_str("| Metric | Count |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Error nodes | {total} |\n"));
    report.push_str(&format!(
        "| Expected preprocessor-test recovery | {expected} |\n"
    ));
    report.push_str(&format!(
        "| Unexplained recovery nodes | {} |\n\n",
        total.saturating_sub(expected)
    ));

    let mut rows = file_stats
        .iter()
        .filter(|stats| stats.error_nodes > 0)
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Error nodes | Classification |\n");
    report.push_str("| --- | ---: | --- |\n");
    for stats in rows.into_iter().take(MAX_ROWS) {
        let classification = if stats.expected_error_nodes == stats.error_nodes {
            "expected `#ifdef BREAK_COMPILATION` preprocessor-test text"
        } else if stats.expected_error_nodes > 0 {
            "mixed expected and unexplained recovery"
        } else {
            "unexplained recovery"
        };
        report.push_str(&format!(
            "| `{}` | {} | {classification} |\n",
            relative_path(root, &stats.path),
            stats.error_nodes
        ));
    }
    report.push('\n');
}

fn append_expression_depth_samples(report: &mut String, root: &Path, file_stats: &[FileStats]) {
    let mut rows = file_stats
        .iter()
        .filter(|stats| stats.expression_depth > 0)
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .expression_depth
            .cmp(&left.expression_depth)
            .then_with(|| left.path.cmp(&right.path))
    });

    report.push_str("## Expression Depth Samples With Snippets\n\n");
    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Depth | Snippet |\n");
    report.push_str("| --- | ---: | --- |\n");
    for stats in rows.into_iter().take(25) {
        report.push_str(&format!(
            "| `{}` | {} | `{}` |\n",
            relative_path(root, &stats.path),
            stats.expression_depth,
            escape_table(
                &stats
                    .expression_depth_snippet
                    .as_deref()
                    .unwrap_or("")
                    .replace('`', "\\`")
            )
        ));
    }
    report.push('\n');
}

fn append_chain_depth_samples(report: &mut String, root: &Path, file_stats: &[FileStats]) {
    let mut rows = file_stats
        .iter()
        .filter(|stats| stats.chain_depth > 0)
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .chain_depth
            .cmp(&left.chain_depth)
            .then_with(|| left.path.cmp(&right.path))
    });

    report.push_str("## Member / Call / Index Chain Samples With Snippets\n\n");
    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Depth | Snippet |\n");
    report.push_str("| --- | ---: | --- |\n");
    for stats in rows.into_iter().take(25) {
        report.push_str(&format!(
            "| `{}` | {} | `{}` |\n",
            relative_path(root, &stats.path),
            stats.chain_depth,
            escape_table(
                &stats
                    .chain_depth_snippet
                    .as_deref()
                    .unwrap_or("")
                    .replace('`', "\\`")
            )
        ));
    }
    report.push('\n');
}

fn append_counts(report: &mut String, title: &str, counts: &BTreeMap<String, usize>) {
    report.push_str(&format!("## {title}\n\n"));
    report.push_str("| Syntax kind | Count |\n");
    report.push_str("| --- | ---: |\n");
    for (kind, count) in counts.iter().take(MAX_ROWS) {
        report.push_str(&format!("| `{kind}` | {count} |\n"));
    }
    report.push('\n');
}

fn append_top_files<F>(
    report: &mut String,
    root: &Path,
    title: &str,
    file_stats: &[FileStats],
    metric: F,
) where
    F: Fn(&FileStats) -> usize,
{
    let mut rows = file_stats
        .iter()
        .filter(|stats| metric(stats) > 0)
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        metric(right)
            .cmp(&metric(left))
            .then_with(|| left.path.cmp(&right.path))
    });

    report.push_str(&format!("## {title}\n\n"));
    if rows.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Value | Statements | Expressions | Diagnostics |\n");
    report.push_str("| --- | ---: | ---: | ---: | ---: |\n");
    for stats in rows.into_iter().take(25) {
        report.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            relative_path(root, &stats.path),
            metric(stats),
            stats.statement_nodes,
            stats.expression_nodes,
            stats.diagnostics
        ));
    }
    report.push('\n');
}

fn max_expression_depth_with_span(node: &SyntaxNode) -> (usize, Option<TextSpan>) {
    let mut best = (0usize, None);
    max_expression_depth(node, 0, &mut best);
    best
}

fn max_expression_depth(node: &SyntaxNode, current: usize, best: &mut (usize, Option<TextSpan>)) {
    let next = if is_expression_kind(node.kind) {
        current + 1
    } else {
        current
    };
    if is_expression_kind(node.kind) && next > best.0 {
        *best = (next, Some(node.span));
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            max_expression_depth(child, next, best);
        }
    }
}

fn max_member_call_index_chain_with_span(node: &SyntaxNode) -> (usize, Option<TextSpan>) {
    let mut best = (0usize, None);
    max_member_call_index_chain(node, 0, &mut best);
    best
}

fn max_member_call_index_chain(
    node: &SyntaxNode,
    current: usize,
    best: &mut (usize, Option<TextSpan>),
) {
    let next = if is_chain_kind(node.kind) {
        current + 1
    } else {
        current
    };
    if is_chain_kind(node.kind) && next > best.0 {
        *best = (next, Some(node.span));
    }

    for child in &node.children {
        if let SyntaxElement::Node(child) = child {
            max_member_call_index_chain(child, next, best);
        }
    }
}

fn is_chain_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::MemberAccessExpression
            | SyntaxKind::CallExpression
            | SyntaxKind::IndexExpression
    )
}

fn is_statement_or_expression_kind(kind: SyntaxKind) -> bool {
    is_statement_kind(kind) || is_expression_kind(kind)
}

fn is_statement_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::IfStatement
            | SyntaxKind::ForStatement
            | SyntaxKind::ForeachStatement
            | SyntaxKind::WhileStatement
            | SyntaxKind::DoWhileStatement
            | SyntaxKind::SwitchStatement
            | SyntaxKind::SwitchSection
            | SyntaxKind::CaseClause
            | SyntaxKind::DefaultClause
            | SyntaxKind::ReturnStatement
            | SyntaxKind::BreakStatement
            | SyntaxKind::ContinueStatement
            | SyntaxKind::DeleteStatement
            | SyntaxKind::ThreadStatement
            | SyntaxKind::EmptyStatement
            | SyntaxKind::LocalDeclStatement
            | SyntaxKind::ExpressionStatement
            | SyntaxKind::ForHeader
            | SyntaxKind::ForInitializer
            | SyntaxKind::ForCondition
            | SyntaxKind::ForIncrement
            | SyntaxKind::ForeachHeader
            | SyntaxKind::ForeachVariableList
            | SyntaxKind::ForeachVariable
            | SyntaxKind::ForeachIterable
    )
}

fn direct_child_node_count(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    node.children
        .iter()
        .filter(|child| matches!(child, SyntaxElement::Node(node) if node.kind == kind))
        .count()
}

fn named_argument_label(source: &str, node: &SyntaxNode) -> Option<String> {
    if node.kind != SyntaxKind::NamedArgument {
        return None;
    }

    for child in &node.children {
        match child {
            SyntaxElement::Token(token)
                if token.kind == reforger_language_server::lexer::TokenKind::Colon =>
            {
                return None;
            }
            SyntaxElement::Token(token) if !token.kind.is_trivia() => {
                return Some(source[token.span.start..token.span.end].to_string());
            }
            SyntaxElement::Node(child) if child.kind == SyntaxKind::NameExpression => {
                return Some(source[child.span.start..child.span.end].to_string());
            }
            SyntaxElement::Node(_) => {}
            _ => {}
        }
    }

    None
}

fn expected_recovery_node_count(node: &SyntaxNode, source: &str, path: &Path) -> usize {
    let relative = path.display().to_string().replace('/', "\\");
    if !relative.ends_with("Game\\game.c") || !source.contains("#ifdef BREAK_COMPILATION") {
        return 0;
    }

    count_kind(node, SyntaxKind::Error)
}

fn count_kind(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    let own = usize::from(node.kind == kind);
    own + node
        .children
        .iter()
        .map(|child| match child {
            SyntaxElement::Node(child) => count_kind(child, kind),
            SyntaxElement::Token(_) => 0,
        })
        .sum::<usize>()
}

fn snippet_for_span(source: &str, span: TextSpan, context_lines: usize) -> String {
    let mut line_start_offsets = vec![0usize];
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            line_start_offsets.push(index + 1);
        }
    }

    let line_index = line_start_offsets
        .partition_point(|offset| *offset <= span.start)
        .saturating_sub(1);
    let start_line = line_index.saturating_sub(context_lines);
    let end_line = (line_index + context_lines + 1).min(line_start_offsets.len());
    let mut lines = Vec::new();
    for current in start_line..end_line {
        let start = line_start_offsets[current];
        let end = line_start_offsets
            .get(current + 1)
            .copied()
            .unwrap_or(source.len());
        lines.push(source[start..end].trim_end().replace('\t', "\\t"));
    }
    lines.join(" / ")
}

fn is_expression_kind(kind: SyntaxKind) -> bool {
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
            | SyntaxKind::ArgumentList
            | SyntaxKind::NamedArgument
            | SyntaxKind::MemberAccessExpression
            | SyntaxKind::IndexExpression
            | SyntaxKind::CastExpression
            | SyntaxKind::PostfixExpression
            | SyntaxKind::NewExpression
            | SyntaxKind::InitializerExpression
    )
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
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
