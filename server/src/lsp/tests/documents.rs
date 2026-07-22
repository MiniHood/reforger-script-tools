#[test]
fn duplicate_did_open_rejects_old_rich_semantic_tokens() {
    let uri = "file:///Scripts/Reopened.c";
    let mut server = LspServer::new(Vec::new(), LspServerOptions::default());
    server
        .handle_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "enforce",
                        "version": 1,
                        "text": "class Old {}"
                    }
                }
            }),
            None,
            0,
            0,
        )
        .unwrap();

    let external_generation = server.external_index.status_summary().generation;
    let (task, old_revision, projection, cancel) = server
        .document_runtime
        .test_prepare_rich_event(uri, external_generation);

    server
        .handle_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "enforce",
                        "version": 2,
                        "text": "class New {}"
                    }
                }
            }),
            None,
            0,
            0,
        )
        .unwrap();

    let current_revision = server.document_runtime.test_document_state(uri).unwrap().revision;
    assert!(cancel.load(Ordering::Relaxed));
    assert_ne!(old_revision, current_revision);

    server
        .handle_internal_event(ServerEvent::RichSemanticTokensReady {
            task: task.identity().clone(),
            uri: uri.to_string(),
            revision: old_revision,
            external_generation,
            external_status: "missing",
            projection,
            elapsed_ms: 0,
        })
        .unwrap();

    assert!(!server.document_runtime.test_document_state(uri).unwrap().rich_semantic_tokens);
}

#[test]
fn document_symbols_include_nested_declarations() {
    let source = r#"class Example
{
	int m_Value;
	void Run(int value);
	void Local()
	{
		int localValue = 5;
	}
}
"#;

    let symbols = document_symbols_for_source(source);

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "Example");
    assert_eq!(symbols[0].kind, 5);
    assert!(symbols[0]
        .children
        .iter()
        .any(|child| child.name == "m_Value" && child.kind == 8));
    assert!(symbols[0]
        .children
        .iter()
        .any(|child| child.name == "Run" && child.kind == 6));
    assert!(!symbols[0]
        .children
        .iter()
        .any(|child| child.name == "localValue"));
    let run = symbols[0]
        .children
        .iter()
        .find(|child| child.name == "Run")
        .unwrap();
    assert!(run.children.is_empty());
}

#[test]
fn document_symbols_cover_declared_kinds_and_sane_ranges() {
    let source = r#"//! Global typedef docs
typedef string FactionKey;

Game g_Game;

[EnumBitFlag()]
enum ExampleFlags
{
	None = 0,
	Enabled = 1
}

class Example
{
	int m_Value;
	void Example(int value);
	void ~Example();
	void Run(string name);
}
"#;

    let report = document_symbol_report_for_source(source);

    assert_eq!(report.parse_diagnostics, 0);
    assert!(report
        .symbols
        .iter()
        .any(|symbol| symbol.name == "FactionKey" && symbol.kind == 26));
    assert!(report
        .symbols
        .iter()
        .any(|symbol| symbol.name == "g_Game" && symbol.kind == 13));
    let enum_symbol = report
        .symbols
        .iter()
        .find(|symbol| symbol.name == "ExampleFlags")
        .unwrap();
    assert_eq!(enum_symbol.kind, 10);
    assert!(enum_symbol
        .children
        .iter()
        .any(|child| child.name == "Enabled" && child.kind == 22));

    let class_symbol = report
        .symbols
        .iter()
        .find(|symbol| symbol.name == "Example")
        .unwrap();
    assert_eq!(class_symbol.kind, 5);
    assert!(class_symbol
        .children
        .iter()
        .any(|child| child.name == "m_Value" && child.kind == 8));
    assert!(class_symbol
        .children
        .iter()
        .any(|child| child.name == "Example" && child.kind == 9));
    assert!(class_symbol
        .children
        .iter()
        .any(|child| child.name == "Example" && child.kind == 6));
    assert!(class_symbol
        .children
        .iter()
        .any(|child| child.name == "Run" && child.kind == 6));

    assert_ranges_are_sane(&report.symbols);
}

#[test]
fn offset_conversion_uses_utf16_positions() {
    let source = "class Sm😀ke {}\n";
    let offset = source.find("ke").unwrap();

    let position = position_for_offset(source, offset);

    assert_eq!(
        position,
        LspPosition {
            line: 0,
            character: 10
        }
    );
    assert_eq!(offset_for_position(source, position), Some(offset));
    assert_eq!(
        offset_for_position(
            source,
            LspPosition {
                line: 0,
                character: 9
            }
        ),
        Some(source.find('😀').unwrap())
    );
}

#[test]
fn offset_conversion_treats_cr_and_crlf_as_single_line_endings() {
    for source in ["class A {}\rclass B {}", "class A {}\r\nclass B {}"] {
        let offset = source.find("class B").expect("second class");
        let position = position_for_offset(source, offset);
        assert_eq!(
            position,
            LspPosition {
                line: 1,
                character: 0
            }
        );
        assert_eq!(offset_for_position(source, position), Some(offset));
    }
}

#[test]
fn document_symbol_projection_builds_one_position_index() {
    POSITION_INDEX_BUILD_COUNT.with(|count| count.set(0));

    let report = document_symbol_report_for_source(
        "class First { void Run() {} }\nclass Second { int value; }\n",
    );

    assert_eq!(report.symbols.len(), 2);
    assert_eq!(POSITION_INDEX_BUILD_COUNT.with(|count| count.get()), 1);
}

#[test]
fn enter_typing_assist_returns_one_semicolon_edit_only_for_the_current_version() {
    let mut server = LspServer::new(Vec::new(), LspServerOptions::default());
    let uri = "file:///Scripts/OnTypeFormatting.c";
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "enforce",
                    "version": 1,
                    "text": "void Run() {\n\tGetGame()\n}"
                }
            }}),
            None,
            0,
            0,
        )
        .unwrap();
    server.writer.clear();
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "id": 1, "method": ENTER_TYPING_ASSIST_METHOD, "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 0 },
                "ch": "\n",
                "version": 1,
                "options": { "tabSize": 4, "insertSpaces": false }
            }}),
            None,
            0,
            0,
        )
        .unwrap();
    let output = String::from_utf8_lossy(&server.writer);
    assert!(output.contains("\"newText\":\";\""), "{output}");
    assert!(output.contains("\"character\":10,\"line\":1"), "{output}");

    server.writer.clear();
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "id": 2, "method": ENTER_TYPING_ASSIST_METHOD, "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 2, "character": 0 },
                "ch": "\n",
                "version": 2,
                "options": { "tabSize": 4, "insertSpaces": false }
            }}),
            None,
            0,
            0,
        )
        .unwrap();
    assert!(String::from_utf8_lossy(&server.writer).contains("\"edits\":[]"));
}

#[test]
fn document_open_and_change_require_versions() {
    let uri = "file:///Scripts/RequiredVersions.c";
    let mut server = LspServer::new(Vec::new(), LspServerOptions::default());

    server
        .handle_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri, "version": 1 },
                    "contentChanges": [{ "text": "class ChangedBeforeOpen {}" }]
                }
            }),
            None,
            0,
            0,
        )
        .unwrap();
    assert!(server.document_runtime.test_document_state(uri).is_none());

    server
        .handle_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "enforce",
                        "text": "class MissingOpenVersion {}"
                    }
                }
            }),
            None,
            0,
            0,
        )
        .unwrap();
    assert!(server.document_runtime.test_document_state(uri).is_none());

    server
        .handle_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "enforce",
                        "version": 1,
                        "text": "class Current {}"
                    }
                }
            }),
            None,
            0,
            0,
        )
        .unwrap();
    server
        .handle_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri },
                    "contentChanges": [{ "text": "class MissingChangeVersion {}" }]
                }
            }),
            None,
            0,
            0,
        )
        .unwrap();

    let document = server.document_runtime.test_document_state(uri).unwrap();
    assert_eq!(document.version, 1);
    assert_eq!(document.text, "class Current {}");

    server
        .handle_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri, "version": 1 },
                    "contentChanges": [{ "text": "class SameVersionReplay {}" }]
                }
            }),
            None,
            0,
            0,
        )
        .unwrap();

    let document = server.document_runtime.test_document_state(uri).unwrap();
    assert_eq!(document.version, 1);
    assert_eq!(document.text, "class Current {}");
}
