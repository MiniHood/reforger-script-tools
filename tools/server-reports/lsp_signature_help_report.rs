use reforger_language_server::index::SymbolIndex;
use reforger_language_server::lsp::{
    signature_help_report_for_source_position, LspPosition, LspSignatureHelpReport,
};
use reforger_language_server::model::{
    source_category_for_path, SourceFileMetadata, SourceKind, SOURCE_PRIORITY_GAME_DATA,
};
use reforger_language_server::parser::parse_source;
use reforger_language_server::semantic_file::SemanticFile;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let mut out = PathBuf::from("tools/reports/lsp-signature-help-fixtures.report.md");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--out" {
            if let Some(value) = args.next() {
                out = PathBuf::from(value);
            }
        }
    }

    let source = r#"//! Sends notification to all players.
//! \param[in] notificationID ID of the notification.
//! \param[in] param1 Optional numeric parameter.
//! \return True if sent.
bool SendToEveryone(ENotification notificationID, int param1 = 0, string label = "ok");

enum ENotification
{
	PLAYER_JOINED
}
enum RplChannel
{
	Reliable
}
enum RplRcver
{
	Owner
}
class ParamEnumArray
{
	static ParamEnumArray FromEnum(typename e);
}
class UniqueAttribute {}
class Attribute : UniqueAttribute
{
	void Attribute(string defvalue = "", string uiwidget = "auto", string desc = "", string params = "", ParamEnumArray enums = NULL, string category = "", int precision = 3, typename enumType = void, bool prefabbed = false);
}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver receiver);
}
class CallableType
{
	void CallableType(string name = "", int value = 0);
	void Required(int value, out int foundCount);
	void Mixed(ENotification notificationID, int param1 = 0, string label = "ok");
}
class Example
{
	void Run()
	{
		CallableType callable = new CallableType(, );
		callable.Required(1, );
		callable.Mixed(ENotification.PLAYER_JOINED, );
		SendToEveryone(notificationID: ENotification.PLAYER_JOINED, );
		ParamEnumArray.FromEnum(ENotification);
	}

	[RplRpc(RplChannel.Reliable, )]
	[Attribute(defvalue: "0", uiwidget: UIWidgets.Flags, desc: )]
	int m_Value;
}"#;
    let _index = index_for_source(
        source,
        Path::new("tools/fixtures/signature-help/game-data"),
        Path::new("tools/fixtures/signature-help/game-data/Game/SignatureHelp.c"),
        SourceKind::GameData,
        SOURCE_PRIORITY_GAME_DATA,
    );

    let checks = vec![
        ("constructor first arg", check(source, "new CallableType(")),
        (
            "constructor second arg",
            check(source, "new CallableType(, "),
        ),
        ("method out arg", check(source, "callable.Required(1, ")),
        (
            "method optional arg",
            check(source, "callable.Mixed(ENotification.PLAYER_JOINED, "),
        ),
        (
            "function named arg",
            check(source, "notificationID: ENotification.PLAYER_JOINED, "),
        ),
        ("static call", check(source, "ParamEnumArray.FromEnum(")),
        (
            "rpc attribute second arg",
            check(source, "RplChannel.Reliable, "),
        ),
        ("attribute named arg", check(source, "desc: ")),
        (
            "non-call position",
            signature_help_report_for_source_position(
                source,
                LspPosition {
                    line: 0,
                    character: 2,
                },
            ),
        ),
    ];

    let mut report = String::new();
    report.push_str("# LSP Signature Help Fixture Report\n\n");
    report.push_str("Dev-only proof for source-backed `textDocument/signatureHelp` across functions, methods, constructors, attributes, named arguments, enum parameters, and nested call shapes.\n\n");
    report.push_str("| Check | Context | Active Parameter | Candidates | Selected | Failure | Signature | Params |\n");
    report.push_str("| --- | --- | ---: | ---: | --- | --- | --- | --- |\n");
    for (label, check) in checks {
        append_check(&mut report, label, &check);
    }

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&out, report).map_err(|error| error.to_string())?;
    println!("Wrote {}", out.display());
    Ok(())
}

fn check(source: &str, needle: &str) -> LspSignatureHelpReport {
    signature_help_report_for_source_position(source, position_after_needle(source, needle))
}

fn append_check(report: &mut String, label: &str, check: &LspSignatureHelpReport) {
    let (signature, params) = check
        .help
        .as_ref()
        .and_then(|help| help.signatures.first())
        .map(|signature| {
            (
                signature.label.as_str(),
                signature
                    .parameters
                    .iter()
                    .map(|parameter| parameter.label.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })
        .unwrap_or(("", String::new()));
    report.push_str(&format!(
        "| {} | `{}` | {} | {} | `{}` | `{}` | `{}` | `{}` |\n",
        label,
        check.context.as_deref().unwrap_or("<none>"),
        check
            .active_parameter
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        check.candidate_count,
        check.selected_label.as_deref().unwrap_or("<none>"),
        check.failure_reason.as_deref().unwrap_or("<none>"),
        signature.replace('|', "\\|"),
        params.replace('|', "\\|")
    ));
}

fn index_for_source(
    source: &str,
    root: &Path,
    file: &Path,
    kind: SourceKind,
    priority: u16,
) -> SymbolIndex {
    let parse = parse_source(source);
    let relative_path = file.strip_prefix(root).unwrap_or(file).to_path_buf();
    let semantic_file = SemanticFile::build(source, &parse);
    SymbolIndex::from_semantic_files([(
        &semantic_file,
        SourceFileMetadata {
            kind,
            category: source_category_for_path(kind, Some(&relative_path)),
            absolute_path: Some(file.to_path_buf()),
            virtual_source: None,
            root_path: Some(root.to_path_buf()),
            relative_path: Some(relative_path),
            priority,
        },
    )])
}

fn position_after_needle(source: &str, needle: &str) -> LspPosition {
    let offset = source
        .find(needle)
        .map(|start| start + needle.len())
        .expect("needle not found");
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let line_start = before.rfind('\n').map(|offset| offset + 1).unwrap_or(0);
    LspPosition {
        line,
        character: before[line_start..].encode_utf16().count() as u32,
    }
}
