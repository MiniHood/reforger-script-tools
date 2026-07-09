use reforger_language_server::ast::{
    AstSourceFile, ClassMember, Declaration, DocComment, DocCommentKind, MethodKind,
};
use reforger_language_server::lexer::TextSpan;
use reforger_language_server::parser::parse_source;
use reforger_language_server::syntax::{ParseDiagnostic, SyntaxElement, SyntaxKind, SyntaxNode};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REPORT_RELATIVE_PATH: &str = "tools/reports/ast-corpus.report.md";
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";
const MAX_UNKNOWN_FILES: usize = 100;
const MAX_SNIPPET_FILES: usize = 25;
const MAX_UNKNOWNS_PER_SNIPPET_FILE: usize = 3;
const SNIPPET_CONTEXT_LINES: usize = 2;

struct Args {
    scripts_path: PathBuf,
    out_path: PathBuf,
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    lossy_files: usize,
    parse_diagnostics: usize,
    files_with_parse_diagnostics: usize,
    top_level_declarations: usize,
    classes: usize,
    enums: usize,
    enum_members: usize,
    enum_members_with_explicit_values: usize,
    typedefs: usize,
    functions: usize,
    fields: usize,
    global_fields: usize,
    class_fields: usize,
    methods: usize,
    regular_methods: usize,
    constructors: usize,
    destructors: usize,
    parameters: usize,
    parameters_with_defaults: usize,
    non_declaration_callable_fragments: usize,
    declarations_or_members_with_doc_comments: usize,
    attached_doc_comments: usize,
    attributes: usize,
    empty_declarations: usize,
}

#[derive(Default)]
struct Quality {
    unknown_class_names: usize,
    unknown_enum_names: usize,
    unknown_typedef_names: usize,
    unknown_function_names: usize,
    unknown_function_return_types: usize,
    unknown_field_names: usize,
    unknown_field_types: usize,
    unknown_method_names: usize,
    unknown_method_return_types: usize,
    unknown_parameter_names: usize,
    unknown_parameter_types: usize,
    unknown_enum_member_names: usize,
}

#[derive(Default)]
struct Frequencies {
    class_base_types: BTreeMap<String, usize>,
    field_types: BTreeMap<String, usize>,
    method_return_types: BTreeMap<String, usize>,
    modifiers: BTreeMap<String, usize>,
    attribute_names: BTreeMap<String, usize>,
    doc_comment_kinds: BTreeMap<String, usize>,
    enum_member_values: BTreeMap<String, usize>,
    parameter_counts: BTreeMap<String, usize>,
    parameter_types: BTreeMap<String, usize>,
    parameter_modifiers: BTreeMap<String, usize>,
}

struct FileUnknowns {
    path: PathBuf,
    unknowns: Vec<UnknownExtraction>,
    source: String,
}

struct FileCallableFragments {
    path: PathBuf,
    fragments: Vec<UnknownExtraction>,
    source: String,
}

struct UnknownExtraction {
    kind: &'static str,
    span: TextSpan,
}

struct LossyFile {
    path: PathBuf,
}

#[derive(Default)]
struct AttributeCoverage {
    parser_attributes: usize,
    ast_attributes: usize,
    matched_attributes: usize,
    unmatched_attributes: usize,
}

struct FileUnmatchedAttributes {
    path: PathBuf,
    spans: Vec<TextSpan>,
    source: String,
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
            "Failed to write AST corpus report {}: {error}",
            args.out_path.display()
        )
    })?;

    println!("Wrote AST corpus report: {}", args.out_path.display());
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut scripts: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;

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
                println!(
                    "Usage: node tools/ast-corpus-report.mjs [--scripts <path>] [--out <path>]"
                );
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

    let mut totals = Totals::default();
    let mut quality = Quality::default();
    let mut frequencies = Frequencies::default();
    let mut files_with_unknowns = Vec::<FileUnknowns>::new();
    let mut files_with_callable_fragments = Vec::<FileCallableFragments>::new();
    let mut attribute_coverage = AttributeCoverage::default();
    let mut files_with_unmatched_attributes = Vec::<FileUnmatchedAttributes>::new();
    let mut lossy_files = Vec::<LossyFile>::new();

    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| format!("Failed to read {}: {error}", file.display()))?;
        totals.files += 1;
        totals.bytes += bytes.len();

        let source = String::from_utf8_lossy(&bytes);
        if matches!(source, Cow::Owned(_)) {
            totals.lossy_files += 1;
            lossy_files.push(LossyFile { path: file.clone() });
        }
        let source = source.into_owned();

        let parse = parse_source(&source);
        totals.parse_diagnostics += parse.diagnostics.len();
        if !parse.diagnostics.is_empty() {
            totals.files_with_parse_diagnostics += 1;
        }

        let ast = AstSourceFile::new(&source, &parse);
        let mut file_unknowns = Vec::<UnknownExtraction>::new();
        let mut file_callable_fragments = Vec::<UnknownExtraction>::new();
        let mut ast_attribute_spans = Vec::<TextSpan>::new();
        scan_ast(
            &ast,
            &mut totals,
            &mut quality,
            &mut frequencies,
            &mut file_unknowns,
            &mut file_callable_fragments,
            &mut ast_attribute_spans,
        );
        let parser_attribute_spans = parser_attribute_spans(&parse.root);
        record_attribute_coverage(
            &parser_attribute_spans,
            &ast_attribute_spans,
            &mut attribute_coverage,
        );
        let unmatched_attribute_spans =
            unmatched_attribute_spans(&parser_attribute_spans, &ast_attribute_spans);
        add_parse_diagnostic_unknowns(&parse.diagnostics, &mut file_unknowns);

        if !file_unknowns.is_empty() {
            files_with_unknowns.push(FileUnknowns {
                path: file.clone(),
                unknowns: file_unknowns,
                source: source.clone(),
            });
        }

        if !file_callable_fragments.is_empty() {
            files_with_callable_fragments.push(FileCallableFragments {
                path: file.clone(),
                fragments: file_callable_fragments,
                source: source.clone(),
            });
        }

        if !unmatched_attribute_spans.is_empty() {
            files_with_unmatched_attributes.push(FileUnmatchedAttributes {
                path: file.clone(),
                spans: unmatched_attribute_spans,
                source,
            });
        }
    }

    files_with_unknowns.sort_by(|left, right| {
        right
            .unknowns
            .len()
            .cmp(&left.unknowns.len())
            .then_with(|| left.path.cmp(&right.path))
    });

    files_with_unmatched_attributes.sort_by(|left, right| {
        right
            .spans
            .len()
            .cmp(&left.spans.len())
            .then_with(|| left.path.cmp(&right.path))
    });

    files_with_callable_fragments.sort_by(|left, right| {
        right
            .fragments
            .len()
            .cmp(&left.fragments.len())
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut report = String::new();
    report.push_str("# AST Corpus Report\n\n");
    report.push_str("> Human-review output generated by `node tools/ast-corpus-report.mjs`.\n\n");
    report.push_str("This report summarizes source-backed AST declaration extraction across real game-data scripts. It is review data only; Workbench remains compiler truth.\n\n");

    append_summary(&mut report, scripts_path, &totals);
    append_quality(&mut report, &quality);
    append_attribute_coverage(&mut report, &attribute_coverage);
    append_counts(
        &mut report,
        "Class Base Type Frequency",
        &frequencies.class_base_types,
        80,
    );
    append_counts(
        &mut report,
        "Field Type Frequency",
        &frequencies.field_types,
        80,
    );
    append_counts(
        &mut report,
        "Regular Method And Function Return Type Frequency",
        &frequencies.method_return_types,
        80,
    );
    append_counts(
        &mut report,
        "Modifier Frequency",
        &frequencies.modifiers,
        80,
    );
    append_counts(
        &mut report,
        "Attribute Name Frequency",
        &frequencies.attribute_names,
        80,
    );
    append_counts(
        &mut report,
        "Doc Comment Kind Frequency",
        &frequencies.doc_comment_kinds,
        80,
    );
    append_counts(
        &mut report,
        "Enum Member Value Frequency",
        &frequencies.enum_member_values,
        80,
    );
    append_counts(
        &mut report,
        "Parameter Count Frequency",
        &frequencies.parameter_counts,
        80,
    );
    append_counts(
        &mut report,
        "Parameter Type Frequency",
        &frequencies.parameter_types,
        80,
    );
    append_counts(
        &mut report,
        "Parameter Modifier Frequency",
        &frequencies.parameter_modifiers,
        80,
    );
    append_unknown_files(&mut report, scripts_path, &files_with_unknowns);
    append_unknown_snippets(&mut report, scripts_path, &files_with_unknowns);
    append_callable_fragment_files(&mut report, scripts_path, &files_with_callable_fragments);
    append_callable_fragment_snippets(&mut report, scripts_path, &files_with_callable_fragments);
    append_unmatched_attribute_files(&mut report, scripts_path, &files_with_unmatched_attributes);
    append_unmatched_attribute_snippets(
        &mut report,
        scripts_path,
        &files_with_unmatched_attributes,
    );
    append_lossy_files(&mut report, scripts_path, &lossy_files);

    Ok(report)
}

fn scan_ast(
    ast: &AstSourceFile<'_, '_>,
    totals: &mut Totals,
    quality: &mut Quality,
    frequencies: &mut Frequencies,
    unknowns: &mut Vec<UnknownExtraction>,
    callable_fragments: &mut Vec<UnknownExtraction>,
    ast_attribute_spans: &mut Vec<TextSpan>,
) {
    for declaration in ast.declarations() {
        totals.top_level_declarations += 1;
        match declaration {
            Declaration::Class(class) => {
                totals.classes += 1;
                if class.name().is_none() {
                    quality.unknown_class_names += 1;
                    unknowns.push(UnknownExtraction {
                        kind: "unknown class name",
                        span: class.span(),
                    });
                }
                if let Some(base_type) = class.base_type() {
                    count(&mut frequencies.class_base_types, base_type.text());
                }
                record_doc_comments(class.doc_comments(), totals, frequencies);
                record_attributes(class.attributes(), totals, frequencies, ast_attribute_spans);
                record_modifiers(class.modifiers(), frequencies);

                for member in class.members() {
                    match member {
                        ClassMember::Field(field) => {
                            totals.fields += 1;
                            totals.class_fields += 1;
                            scan_field(
                                field,
                                totals,
                                quality,
                                frequencies,
                                unknowns,
                                ast_attribute_spans,
                            );
                        }
                        ClassMember::Method(method) => {
                            totals.methods += 1;
                            let method_kind = class.classify_method(method);
                            match method_kind {
                                MethodKind::Method => totals.regular_methods += 1,
                                MethodKind::Constructor => totals.constructors += 1,
                                MethodKind::Destructor => totals.destructors += 1,
                            }
                            if method.name().is_none() {
                                quality.unknown_method_names += 1;
                                unknowns.push(UnknownExtraction {
                                    kind: "unknown method name",
                                    span: method.span(),
                                });
                            }
                            if let Some(return_type) = method.return_type_text() {
                                if method_kind == MethodKind::Method {
                                    count(&mut frequencies.method_return_types, return_type.text());
                                }
                            } else {
                                quality.unknown_method_return_types += 1;
                                unknowns.push(UnknownExtraction {
                                    kind: "unknown method return type",
                                    span: method.span(),
                                });
                            }
                            let parameters = method.parameters();
                            count(
                                &mut frequencies.parameter_counts,
                                &parameters.len().to_string(),
                            );
                            scan_parameters(parameters, totals, quality, frequencies, unknowns);
                            scan_callable_fragments(
                                method.parameter_fragments(),
                                totals,
                                callable_fragments,
                            );
                            record_doc_comments(method.doc_comments(), totals, frequencies);
                            record_attributes(
                                method.attributes(),
                                totals,
                                frequencies,
                                ast_attribute_spans,
                            );
                            record_modifiers(method.modifiers(), frequencies);
                        }
                        ClassMember::Empty(_) => totals.empty_declarations += 1,
                    }
                }
            }
            Declaration::Enum(enum_decl) => {
                totals.enums += 1;
                if enum_decl.name().is_none() {
                    quality.unknown_enum_names += 1;
                    unknowns.push(UnknownExtraction {
                        kind: "unknown enum name",
                        span: enum_decl.span(),
                    });
                }
                for member in enum_decl.members() {
                    totals.enum_members += 1;
                    if member.name().is_none() {
                        quality.unknown_enum_member_names += 1;
                        unknowns.push(UnknownExtraction {
                            kind: "unknown enum member name",
                            span: member.span(),
                        });
                    }
                    if let Some(value) = member.value_text() {
                        totals.enum_members_with_explicit_values += 1;
                        count(&mut frequencies.enum_member_values, value.text());
                    }
                }
                record_attributes(
                    enum_decl.attributes(),
                    totals,
                    frequencies,
                    ast_attribute_spans,
                );
                record_doc_comments(enum_decl.doc_comments(), totals, frequencies);
            }
            Declaration::Typedef(typedef_decl) => {
                totals.typedefs += 1;
                if typedef_decl.name().is_none() {
                    quality.unknown_typedef_names += 1;
                    unknowns.push(UnknownExtraction {
                        kind: "unknown typedef name",
                        span: typedef_decl.text_span(),
                    });
                }
                record_doc_comments(typedef_decl.doc_comments(), totals, frequencies);
            }
            Declaration::Function(function) => {
                totals.functions += 1;
                if function.name().is_none() {
                    quality.unknown_function_names += 1;
                    unknowns.push(UnknownExtraction {
                        kind: "unknown function name",
                        span: function.span(),
                    });
                }
                if let Some(return_type) = function.return_type_text() {
                    count(&mut frequencies.method_return_types, return_type.text());
                } else {
                    quality.unknown_function_return_types += 1;
                    unknowns.push(UnknownExtraction {
                        kind: "unknown function return type",
                        span: function.span(),
                    });
                }
                let parameters = function.parameters();
                count(
                    &mut frequencies.parameter_counts,
                    &parameters.len().to_string(),
                );
                scan_parameters(parameters, totals, quality, frequencies, unknowns);
                scan_callable_fragments(function.parameter_fragments(), totals, callable_fragments);
                record_doc_comments(function.doc_comments(), totals, frequencies);
                record_attributes(
                    function.attributes(),
                    totals,
                    frequencies,
                    ast_attribute_spans,
                );
                record_modifiers(function.modifiers(), frequencies);
            }
            Declaration::Field(field) => {
                totals.fields += 1;
                totals.global_fields += 1;
                scan_field(
                    field,
                    totals,
                    quality,
                    frequencies,
                    unknowns,
                    ast_attribute_spans,
                );
            }
        }
    }
}

fn scan_field(
    field: reforger_language_server::ast::FieldDecl<'_, '_>,
    totals: &mut Totals,
    quality: &mut Quality,
    frequencies: &mut Frequencies,
    unknowns: &mut Vec<UnknownExtraction>,
    ast_attribute_spans: &mut Vec<TextSpan>,
) {
    if field.name().is_none() {
        quality.unknown_field_names += 1;
        unknowns.push(UnknownExtraction {
            kind: "unknown field name",
            span: field.span(),
        });
    }
    if let Some(field_type) = field.type_text() {
        count(&mut frequencies.field_types, field_type.text());
    } else {
        quality.unknown_field_types += 1;
        unknowns.push(UnknownExtraction {
            kind: "unknown field type",
            span: field.span(),
        });
    }
    record_attributes(field.attributes(), totals, frequencies, ast_attribute_spans);
    record_doc_comments(field.doc_comments(), totals, frequencies);
    record_modifiers(field.modifiers(), frequencies);
}

fn scan_parameters(
    parameters: Vec<reforger_language_server::ast::Parameter<'_, '_>>,
    totals: &mut Totals,
    quality: &mut Quality,
    frequencies: &mut Frequencies,
    unknowns: &mut Vec<UnknownExtraction>,
) {
    totals.parameters += parameters.len();
    for parameter in parameters {
        if parameter.name().is_none() {
            quality.unknown_parameter_names += 1;
            unknowns.push(UnknownExtraction {
                kind: "unknown parameter name",
                span: parameter.span(),
            });
        }
        if let Some(parameter_type) = parameter.type_text() {
            count(&mut frequencies.parameter_types, parameter_type.text());
        } else {
            quality.unknown_parameter_types += 1;
            unknowns.push(UnknownExtraction {
                kind: "unknown parameter type",
                span: parameter.span(),
            });
        }
        if parameter.default_text().is_some() {
            totals.parameters_with_defaults += 1;
        }
        for modifier in parameter.modifiers() {
            count(&mut frequencies.parameter_modifiers, modifier.text());
        }
    }
}

fn scan_callable_fragments(
    fragments: Vec<reforger_language_server::ast::Parameter<'_, '_>>,
    totals: &mut Totals,
    callable_fragments: &mut Vec<UnknownExtraction>,
) {
    totals.non_declaration_callable_fragments += fragments.len();
    for fragment in fragments {
        callable_fragments.push(UnknownExtraction {
            kind: "non-declaration callable fragment",
            span: fragment.span(),
        });
    }
}

fn parser_attribute_spans(root: &SyntaxNode) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    collect_parser_attribute_spans(root, &mut spans);
    spans
}

fn collect_parser_attribute_spans(node: &SyntaxNode, spans: &mut Vec<TextSpan>) {
    if node.kind == SyntaxKind::Attribute {
        spans.push(node.span);
    }

    for child in &node.children {
        if let SyntaxElement::Node(child_node) = child {
            collect_parser_attribute_spans(child_node, spans);
        }
    }
}

fn record_attribute_coverage(
    parser_attribute_spans: &[TextSpan],
    ast_attribute_spans: &[TextSpan],
    coverage: &mut AttributeCoverage,
) {
    let unmatched = unmatched_attribute_spans(parser_attribute_spans, ast_attribute_spans);

    coverage.parser_attributes += parser_attribute_spans.len();
    coverage.ast_attributes += ast_attribute_spans.len();
    coverage.matched_attributes += parser_attribute_spans.len().saturating_sub(unmatched.len());
    coverage.unmatched_attributes += unmatched.len();
}

fn unmatched_attribute_spans(
    parser_attribute_spans: &[TextSpan],
    ast_attribute_spans: &[TextSpan],
) -> Vec<TextSpan> {
    let ast_spans: BTreeSet<(usize, usize)> = ast_attribute_spans
        .iter()
        .map(|span| (span.start, span.end))
        .collect();

    parser_attribute_spans
        .iter()
        .copied()
        .filter(|span| !ast_spans.contains(&(span.start, span.end)))
        .collect()
}

fn add_parse_diagnostic_unknowns(
    diagnostics: &[ParseDiagnostic],
    unknowns: &mut Vec<UnknownExtraction>,
) {
    for diagnostic in diagnostics {
        unknowns.push(UnknownExtraction {
            kind: "parse diagnostic",
            span: diagnostic.span,
        });
    }
}

fn record_attributes(
    attributes: Vec<reforger_language_server::ast::Attribute<'_, '_>>,
    totals: &mut Totals,
    frequencies: &mut Frequencies,
    ast_attribute_spans: &mut Vec<TextSpan>,
) {
    totals.attributes += attributes.len();
    for attribute in attributes {
        ast_attribute_spans.push(attribute.span());
        if let Some(name) = attribute.name() {
            count(&mut frequencies.attribute_names, name.text());
        }
    }
}

fn record_doc_comments(
    comments: Vec<DocComment<'_>>,
    totals: &mut Totals,
    frequencies: &mut Frequencies,
) {
    if comments.is_empty() {
        return;
    }

    totals.declarations_or_members_with_doc_comments += 1;
    totals.attached_doc_comments += comments.len();

    for comment in comments {
        let kind = match comment.kind() {
            DocCommentKind::Line => "line",
            DocCommentKind::Block => "block",
        };
        count(&mut frequencies.doc_comment_kinds, kind);
    }
}

fn record_modifiers(
    modifiers: Vec<reforger_language_server::ast::TextValue<'_>>,
    frequencies: &mut Frequencies,
) {
    for modifier in modifiers {
        count(&mut frequencies.modifiers, modifier.text());
    }
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

fn append_summary(report: &mut String, scripts_path: &Path, totals: &Totals) {
    report.push_str("## Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!("| Source path | `{}` |\n", scripts_path.display()));
    report.push_str(&format!(
        "| Scan timestamp unix seconds | {} |\n",
        timestamp()
    ));
    report.push_str(&format!("| `.c` files | {} |\n", totals.files));
    report.push_str(&format!("| Bytes | {} |\n", totals.bytes));
    report.push_str(&format!(
        "| Files decoded lossily | {} |\n",
        totals.lossy_files
    ));
    report.push_str(&format!(
        "| Parse diagnostics | {} |\n",
        totals.parse_diagnostics
    ));
    report.push_str(&format!(
        "| Files with parse diagnostics | {} |\n",
        totals.files_with_parse_diagnostics
    ));
    report.push_str(&format!(
        "| Top-level declarations | {} |\n",
        totals.top_level_declarations
    ));
    report.push_str(&format!("| Classes | {} |\n", totals.classes));
    report.push_str(&format!("| Enums | {} |\n", totals.enums));
    report.push_str(&format!("| Enum members | {} |\n", totals.enum_members));
    report.push_str(&format!(
        "| Enum members with explicit values | {} |\n",
        totals.enum_members_with_explicit_values
    ));
    report.push_str(&format!("| Typedefs | {} |\n", totals.typedefs));
    report.push_str(&format!("| Functions | {} |\n", totals.functions));
    report.push_str(&format!("| Fields | {} |\n", totals.fields));
    report.push_str(&format!("| Global fields | {} |\n", totals.global_fields));
    report.push_str(&format!("| Class fields | {} |\n", totals.class_fields));
    report.push_str(&format!("| Methods | {} |\n", totals.methods));
    report.push_str(&format!(
        "| Regular methods | {} |\n",
        totals.regular_methods
    ));
    report.push_str(&format!("| Constructors | {} |\n", totals.constructors));
    report.push_str(&format!("| Destructors | {} |\n", totals.destructors));
    report.push_str(&format!("| Parameters | {} |\n", totals.parameters));
    report.push_str(&format!(
        "| Parameters with defaults | {} |\n",
        totals.parameters_with_defaults
    ));
    report.push_str(&format!(
        "| Non-declaration callable fragments | {} |\n",
        totals.non_declaration_callable_fragments
    ));
    report.push_str(&format!(
        "| Declarations/members with doc comments | {} |\n",
        totals.declarations_or_members_with_doc_comments
    ));
    report.push_str(&format!(
        "| Attached doc comments | {} |\n",
        totals.attached_doc_comments
    ));
    report.push_str(&format!("| Attributes | {} |\n", totals.attributes));
    report.push_str(&format!(
        "| Empty declarations | {} |\n\n",
        totals.empty_declarations
    ));
}

fn append_quality(report: &mut String, quality: &Quality) {
    report.push_str("## Extraction Quality\n\n");
    report.push_str("| Item | Count |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Unknown class names | {} |\n",
        quality.unknown_class_names
    ));
    report.push_str(&format!(
        "| Unknown enum names | {} |\n",
        quality.unknown_enum_names
    ));
    report.push_str(&format!(
        "| Unknown typedef names | {} |\n",
        quality.unknown_typedef_names
    ));
    report.push_str(&format!(
        "| Unknown function names | {} |\n",
        quality.unknown_function_names
    ));
    report.push_str(&format!(
        "| Unknown function return types | {} |\n",
        quality.unknown_function_return_types
    ));
    report.push_str(&format!(
        "| Unknown field names | {} |\n",
        quality.unknown_field_names
    ));
    report.push_str(&format!(
        "| Unknown field types | {} |\n",
        quality.unknown_field_types
    ));
    report.push_str(&format!(
        "| Unknown method names | {} |\n",
        quality.unknown_method_names
    ));
    report.push_str(&format!(
        "| Unknown method return types | {} |\n",
        quality.unknown_method_return_types
    ));
    report.push_str(&format!(
        "| Unknown parameter names | {} |\n",
        quality.unknown_parameter_names
    ));
    report.push_str(&format!(
        "| Unknown parameter types | {} |\n",
        quality.unknown_parameter_types
    ));
    report.push_str(&format!(
        "| Unknown enum member names | {} |\n\n",
        quality.unknown_enum_member_names
    ));
}

fn append_attribute_coverage(report: &mut String, coverage: &AttributeCoverage) {
    report.push_str("## Attribute Coverage Summary\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("| --- | ---: |\n");
    report.push_str(&format!(
        "| Parser attributes | {} |\n",
        coverage.parser_attributes
    ));
    report.push_str(&format!(
        "| AST attributes | {} |\n",
        coverage.ast_attributes
    ));
    report.push_str(&format!(
        "| Matched attributes | {} |\n",
        coverage.matched_attributes
    ));
    report.push_str(&format!(
        "| Unmatched attributes | {} |\n",
        coverage.unmatched_attributes
    ));
    report.push_str(&format!(
        "| Attribute coverage delta | {} |\n\n",
        coverage.parser_attributes as isize - coverage.ast_attributes as isize
    ));
}

fn append_counts(
    report: &mut String,
    heading: &str,
    counts: &BTreeMap<String, usize>,
    limit: usize,
) {
    report.push_str(&format!("## {heading}\n\n"));

    if counts.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| Item | Count |\n");
    report.push_str("| --- | ---: |\n");
    for (item, count) in sorted_counts(counts).into_iter().take(limit) {
        report.push_str(&format!("| `{}` | {} |\n", escape_table(&item), count));
    }
    report.push('\n');
}

fn append_unknown_files(report: &mut String, scripts_path: &Path, files: &[FileUnknowns]) {
    report.push_str("## Top Files With Unknown Extraction\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Unknowns |\n");
    report.push_str("| --- | ---: |\n");
    for file in files.iter().take(MAX_UNKNOWN_FILES) {
        report.push_str(&format!(
            "| `{}` | {} |\n",
            relative_path(scripts_path, &file.path),
            file.unknowns.len()
        ));
    }
    report.push('\n');
}

fn append_unknown_snippets(report: &mut String, scripts_path: &Path, files: &[FileUnknowns]) {
    report.push_str("## Unknown Extraction Snippets\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    for file in files.iter().take(MAX_SNIPPET_FILES) {
        report.push_str(&format!(
            "### `{}`\n\n",
            relative_path(scripts_path, &file.path)
        ));
        for unknown in file.unknowns.iter().take(MAX_UNKNOWNS_PER_SNIPPET_FILE) {
            let (line, column) = line_column(&file.source, unknown.span.start);
            report.push_str(&format!(
                "- `{}` at {}:{} span {}..{}\n\n",
                unknown.kind, line, column, unknown.span.start, unknown.span.end
            ));
            append_source_snippet(report, &file.source, line);
        }
    }
}

fn append_callable_fragment_files(
    report: &mut String,
    scripts_path: &Path,
    files: &[FileCallableFragments],
) {
    report.push_str("## Top Files With Non-Declaration Callable Fragments\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Fragments |\n");
    report.push_str("| --- | ---: |\n");
    for file in files.iter().take(MAX_UNKNOWN_FILES) {
        report.push_str(&format!(
            "| `{}` | {} |\n",
            relative_path(scripts_path, &file.path),
            file.fragments.len()
        ));
    }
    report.push('\n');
}

fn append_callable_fragment_snippets(
    report: &mut String,
    scripts_path: &Path,
    files: &[FileCallableFragments],
) {
    report.push_str("## Non-Declaration Callable Fragment Snippets\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    for file in files.iter().take(MAX_SNIPPET_FILES) {
        report.push_str(&format!(
            "### `{}`\n\n",
            relative_path(scripts_path, &file.path)
        ));
        for fragment in file.fragments.iter().take(MAX_UNKNOWNS_PER_SNIPPET_FILE) {
            let (line, column) = line_column(&file.source, fragment.span.start);
            report.push_str(&format!(
                "- `{}` at {}:{} span {}..{}\n\n",
                fragment.kind, line, column, fragment.span.start, fragment.span.end
            ));
            append_source_snippet(report, &file.source, line);
        }
    }
}

fn append_unmatched_attribute_files(
    report: &mut String,
    scripts_path: &Path,
    files: &[FileUnmatchedAttributes],
) {
    report.push_str("## Top Files With Unmatched Attributes\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File | Unmatched attributes |\n");
    report.push_str("| --- | ---: |\n");
    for file in files.iter().take(MAX_UNKNOWN_FILES) {
        report.push_str(&format!(
            "| `{}` | {} |\n",
            relative_path(scripts_path, &file.path),
            file.spans.len()
        ));
    }
    report.push('\n');
}

fn append_unmatched_attribute_snippets(
    report: &mut String,
    scripts_path: &Path,
    files: &[FileUnmatchedAttributes],
) {
    report.push_str("## Unmatched Attribute Snippets\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    for file in files.iter().take(MAX_SNIPPET_FILES) {
        report.push_str(&format!(
            "### `{}`\n\n",
            relative_path(scripts_path, &file.path)
        ));
        for span in file.spans.iter().take(MAX_UNKNOWNS_PER_SNIPPET_FILE) {
            let (line, column) = line_column(&file.source, span.start);
            report.push_str(&format!(
                "- `unmatched parser attribute` at {}:{} span {}..{}\n\n",
                line, column, span.start, span.end
            ));
            append_source_snippet(report, &file.source, line);
        }
    }
}

fn append_source_snippet(report: &mut String, source: &str, line: usize) {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        report.push_str("````text\n<empty file>\n````\n\n");
        return;
    }

    let start = line.saturating_sub(SNIPPET_CONTEXT_LINES + 1);
    let end = (line + SNIPPET_CONTEXT_LINES).min(lines.len());

    report.push_str("````enforce\n");
    for index in start..end {
        let marker = if index + 1 == line { ">" } else { " " };
        report.push_str(&format!(
            "{marker} {:>5} | {}\n",
            index + 1,
            lines[index].replace('\t', "    ")
        ));
    }
    report.push_str("````\n\n");
}

fn append_lossy_files(report: &mut String, scripts_path: &Path, files: &[LossyFile]) {
    report.push_str("## Files Decoded Lossily\n\n");

    if files.is_empty() {
        report.push_str("None.\n\n");
        return;
    }

    report.push_str("| File |\n");
    report.push_str("| --- |\n");
    for file in files.iter().take(100) {
        report.push_str(&format!(
            "| `{}` |\n",
            relative_path(scripts_path, &file.path)
        ));
    }
    report.push('\n');
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;

    for (index, value) in source.char_indices() {
        if index >= offset {
            break;
        }

        if value == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut values: Vec<_> = counts
        .iter()
        .map(|(item, count)| (item.clone(), *count))
        .collect();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
}

fn count(counts: &mut BTreeMap<String, usize>, value: &str) {
    *counts.entry(value.to_string()).or_default() += 1;
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|")
}
