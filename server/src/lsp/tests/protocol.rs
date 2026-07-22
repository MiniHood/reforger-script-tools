#[test]
fn completion_formats_rpl_rpc_inside_an_existing_attribute_bracket() {
    let source = r#"class Example
{
	[RplR]
}
"#;
    let external = file_index_for_source(
        r#"enum RplChannel { Reliable }
enum RplRcver { Server }
class UniqueAttribute {}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver rcver);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "RplR"),
        Some(&external),
    );
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "RplRpc")
        .expect("expected RplRpc completion inside attribute brackets");
    assert_eq!(
        item.text_edit.new_text,
        "RplRpc(${1:RplChannel.Reliable}, ${2:RplRcver.Server})"
    );
    assert_eq!(item.insert_text_format, Some(2));
    assert_eq!(
        item.command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some("reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholder")
    );
}

#[test]
fn framed_lsp_smoke_test_handles_open_and_document_symbol() {
    let source = "class Smoke\n{\n\tvoid Run();\n}\n";
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c"
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: None,
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("\"documentSymbolProvider\":true"));
    assert!(output_text.contains("\"documentRangeFormattingProvider\":true"));
    assert!(output_text.contains("\"name\":\"Smoke\""));
    assert!(output_text.contains("\"name\":\"Run\""));
}

#[test]
fn framed_lsp_contains_invalid_request_params_and_continues() {
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null}),
    );
    write_test_message(
        &mut input,
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    );

    let mut output = Vec::new();
    run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("\"id\":1"));
    assert!(output.contains("\"code\":-32602"));
    assert!(output.contains("\"id\":2"));
    assert!(output.contains("\"serverInfo\""));
}

#[test]
fn framed_lsp_ignores_invalid_notification_params_and_continues() {
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null}),
    );
    write_test_message(
        &mut input,
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    );

    let mut output = Vec::new();
    run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(!output.contains("\"error\""));
    assert!(output.contains("\"id\":1"));
    assert!(output.contains("\"serverInfo\""));
}

#[test]
fn framed_lsp_rejects_requests_after_shutdown() {
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({"jsonrpc": "2.0", "id": 1, "method": "shutdown", "params": null}),
    );
    write_test_message(
        &mut input,
        json!({"jsonrpc": "2.0", "id": 2, "method": "initialize", "params": {}}),
    );
    write_test_message(
        &mut input,
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    );

    let mut output = Vec::new();
    run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("\"id\":2"));
    assert!(output.contains("\"code\":-32600"));
}

#[test]
fn framed_lsp_exit_before_shutdown_is_an_error() {
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({"jsonrpc": "2.0", "method": "exit", "params": null}),
    );

    let error = run(input.as_slice(), Vec::new(), LspServerOptions::default()).unwrap_err();

    assert!(error.contains("before shutdown"));
}

#[test]
fn framed_lsp_reuses_cached_document_symbols_for_repeated_requests() {
    let source = "class Smoke\n{\n\tvoid Run();\n}\n";
    let log_path = test_log_path("cached_document_symbols");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    for id in [2, 3] {
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c"
                    }
                }
            }),
        );
    }
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: Some(log_path.clone()),
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert_eq!(output_text.matches("\"name\":\"Smoke\"").count(), 2);
    assert_eq!(output_text.matches("\"name\":\"Run\"").count(), 2);

    let log = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(log.matches("notification didOpen").count(), 1);
    assert_eq!(log.matches("analysis_elapsed_ms=").count(), 1);
    assert_eq!(log.matches("request documentSymbol").count(), 2);
    assert_eq!(log.matches("document_symbols_cached=true").count(), 3);

    cleanup_log(&log_path);
}

#[test]
fn framed_lsp_did_change_defers_document_symbol_projection_until_requested() {
    let old_source = "class Old\n{\n\tvoid OldRun();\n}\n";
    let new_source = "class New\n{\n\tvoid NewRun();\n}\n";
    let log_path = test_log_path("lazy_document_symbols_after_change");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/LazySymbols.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": old_source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/LazySymbols.c",
                    "version": 2
                },
                "contentChanges": [
                    {
                        "text": new_source
                    }
                ]
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/LazySymbols.c"
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: Some(log_path.clone()),
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("\"name\":\"New\""));
    assert!(output_text.contains("\"name\":\"NewRun\""));
    assert!(!output_text.contains("\"name\":\"Old\""));
    assert!(!output_text.contains("\"name\":\"OldRun\""));

    let log = std::fs::read_to_string(&log_path).unwrap();
    assert!(log.contains("notification didChange"));
    assert!(log.contains("notification didChange uri=file:///Scripts/LazySymbols.c"));
    assert!(log.contains("document_symbols_cached=false symbols=pending"));
    assert!(log.contains("request documentSymbol uri=file:///Scripts/LazySymbols.c"));
    assert!(log.contains("document_symbols_cached=false document_symbol_ms="));

    cleanup_log(&log_path);
}

#[test]
fn enter_typing_assist_repairs_only_an_incomplete_if_header_with_a_body_selection() {
    let mut server = LspServer::new(Vec::new(), LspServerOptions::default());
    let uri = "file:///Scripts/IfHeaderEnter.c";
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "enforce",
                    "version": 1,
                    "text": "\tif (owner == GetOwner()\n\t"
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
                "position": { "line": 1, "character": 1 },
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
    assert!(
        output.contains("\"newText\":\"\\tif (owner == GetOwner())\\n\\t\\t\""),
        "{output}"
    );
    assert!(
        output.contains("\"selection\":{\"character\":2,\"line\":1}"),
        "{output}"
    );
}

#[test]
fn runtime_debug_hover_runs_off_the_lsp_message_loop() {
    let (sender, receiver) = mpsc::channel();
    let scheduler = RuntimeWorkExecutor::start(sender);
    let mut server = LspServer::new_with_runtime_senders(
        Vec::new(),
        LspServerOptions::default(),
        None,
        Some(scheduler),
        None,
    );
    let uri = "file:///Scripts/AsyncDebug.c";
    server
        .handle_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": {
                    "uri": uri,
                    "languageId": "enforce",
                    "version": 1,
                    "text": "class AsyncDebug { void Run() {} }"
                }}
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
                "id": 7,
                "method": "reforger/debugHover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": 0, "character": 24 }
                }
            }),
            None,
            0,
            0,
        )
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&server.writer).contains("\"id\":7"),
        "the main LSP loop must not wait for a debug capture"
    );

    let event = loop {
        let event = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("debug result");
        if matches!(event, ServerEvent::DebugRequestReady { .. }) {
            break event;
        }
        server.handle_internal_event(event).unwrap();
    };
    match &event {
        ServerEvent::DebugRequestReady { task, .. } => {
            assert_eq!(task.class(), TaskClass::Rich);
            assert_eq!(task.revision(), 1);
        }
        _ => panic!("expected runtime-admitted debug result"),
    }
    server.handle_internal_event(event).unwrap();
    assert!(String::from_utf8_lossy(&server.writer).contains("\"id\":7"));
}

#[test]
fn framed_lsp_smoke_test_handles_hover() {
    let source = "class Smoke\n{\n\tvoid Run(int value);\n}\n";
    let hover_position = position_for_needle(source, "Run(int", "Run");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c"
                },
                "position": {
                    "line": hover_position.line,
                    "character": hover_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: None,
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("\"hoverProvider\":true"));
    assert!(output_text.contains("\"signatureHelpProvider\""));
    assert!(output_text.contains("\"completionProvider\":{\"triggerCharacters\":[\".\",\"[\",\"#\"]}"));
    assert!(output_text.contains("void Run(int value)"));
    assert!(output_text.contains("\"kind\":\"markdown\""));
}

#[test]
fn framed_lsp_smoke_test_handles_definition() {
    let source = "class Smoke\n{\n\tvoid Run(int value)\n\t{\n\t\tPrint(value);\n\t}\n}\n";
    let definition_position = position_for_needle(source, "Print(value)", "value");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c"
                },
                "position": {
                    "line": definition_position.line,
                    "character": definition_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: None,
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("\"definitionProvider\":true"));
    assert!(output_text.contains("\"targetUri\":\"file:///Scripts/Smoke.c\""));
    assert!(output_text.contains("\"originSelectionRange\""));
    assert!(output_text.contains("\"targetRange\""));
    assert!(output_text.contains("\"targetSelectionRange\""));
    assert!(output_text.contains("\"line\":2"));
    assert!(output_text.contains("\"character\":14"));
}

#[test]
fn framed_lsp_if_completion_carries_rust_normalization_contract() {
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {
                    "uri": "file:///Scripts/IfCompletion.c", "languageId": "enforce", "version": 1, "text": "i"
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion", "params": {
                "textDocument": { "uri": "file:///Scripts/IfCompletion.c" },
                "position": { "line": 0, "character": 1 }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
    );

    let mut output = Vec::new();
    run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("\"label\":\"if\""));
    assert!(output.contains("\"expectedCommit\":\" \""));
    assert!(output.contains("\"deletion\":{\"end\":{\"character\":5,\"line\":0},\"start\":{\"character\":4,\"line\":0}}"));
    assert!(output.contains("\"trailingDeletion\":{\"end\":{\"character\":6,\"line\":0},\"start\":{\"character\":5,\"line\":0}}"));
    assert!(output.contains("\"caret\":{\"character\":4,\"line\":0}"));
}

#[test]
fn framed_lsp_preprocessor_completion_exposes_directives_and_active_macro_operands() {
    let mut input = Vec::new();
    write_test_message(&mut input, json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }));
    write_test_message(&mut input, json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": { "textDocument": {
            "uri": "file:///Scripts/PreprocessorCompletion.c", "languageId": "enforce", "version": 1,
            "text": "#define ACTIVE_FLAG\n//#define COMMENTED_FLAG\n\t#\n#ifndef "
        }}
    }));
    write_test_message(&mut input, json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion", "params": {
            "textDocument": { "uri": "file:///Scripts/PreprocessorCompletion.c" },
            "position": { "line": 2, "character": 2 }
        }
    }));
    write_test_message(&mut input, json!({
        "jsonrpc": "2.0", "id": 3, "method": "textDocument/completion", "params": {
            "textDocument": { "uri": "file:///Scripts/PreprocessorCompletion.c" },
            "position": { "line": 3, "character": 8 }
        }
    }));
    write_test_message(&mut input, json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown", "params": null }));
    write_test_message(&mut input, json!({ "jsonrpc": "2.0", "method": "exit", "params": null }));

    let mut output = Vec::new();
    run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();
    let output = String::from_utf8(output).unwrap();
    for directive in ["#define", "#ifdef", "#ifndef", "#else", "#endif"] {
        assert!(output.contains(&format!("\"label\":\"{directive}\"")), "{directive}");
    }
    assert!(output.contains("\"label\":\"ACTIVE_FLAG\""));
    assert!(output.contains("\"detail\":\"#define ACTIVE_FLAG (Workspace)\""));
    assert!(output.contains("\"newText\":\"ACTIVE_FLAG\""));
    assert!(!output.contains("COMMENTED_FLAG"));
}

#[test]
fn framed_lsp_smoke_test_handles_member_completion() {
    let source = "class Widget\n{\n\tvoid SetVisible(bool visible);\n}\nclass Smoke\n{\n\tvoid Run()\n\t{\n\t\tWidget widget;\n\t\twidget.\n\t}\n}\n";
    let completion_position = position_after_needle(source, "widget.");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c"
                },
                "position": {
                    "line": completion_position.line,
                    "character": completion_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("\"completionProvider\":{\"triggerCharacters\":[\".\",\"[\",\"#\"]}"));
    assert!(output_text.contains("\"isIncomplete\":false"));
    assert!(output_text.contains("\"label\":\"SetVisible\""));
    assert!(output_text.contains("\"newText\":\"SetVisible(${1:visible})\""));
}

#[test]
fn framed_lsp_smoke_test_handles_signature_help() {
    let source = "class Smoke\n{\n\tvoid Run(int value, string label = \"ok\");\n\tvoid Test(int input)\n\t{\n\t\tRun(1, );\n\t\tRun(inp);\n\t}\n}\n";
    let second_parameter_position = position_after_needle(source, "Run(1, ");
    let typed_argument_position = position_after_needle(source, "Run(inp");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/signatureHelp",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c"
                },
                "position": {
                    "line": second_parameter_position.line,
                    "character": second_parameter_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/signatureHelp",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c"
                },
                "position": {
                    "line": typed_argument_position.line,
                    "character": typed_argument_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("\"signatureHelpProvider\""));
    assert!(output_text.contains("\"triggerCharacters\":[\"(\",\",\",\".\",\":\",\"_\",\"a\""));
    assert!(output_text.contains("\"retriggerCharacters\":[\"(\",\",\",\".\",\":\",\"_\",\"a\""));
    assert!(output_text.contains("\"activeParameter\":1"));
    assert!(output_text.contains("\"activeParameter\":0"));
    assert!(output_text
        .contains("\"label\":\"Smoke.Run(int value, string label = \\\"ok\\\") -> void\""));
    assert!(output_text.contains("\"label\":\"int value\""));
    assert!(output_text.contains("\"label\":\"string label = \\\"ok\\\"\""));
}

#[test]
fn framed_lsp_smoke_test_handles_semantic_tokens() {
    let source = "class Smoke\n{\n\tvoid Run(int value)\n\t{\n\t\tstring name = \"x\";\n\t}\n}\n";
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/semanticTokens/full",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c"
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": "server-1",
            "result": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/semanticTokens/full",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c"
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("\"semanticTokensProvider\""));
    assert!(output_text.contains("\"tokenTypes\":[\"class\",\"enum\",\"type\""));
    assert!(output_text.contains("\"id\":2"));
    assert!(output_text.contains("\"method\":\"workspace/semanticTokens/refresh\""));
    assert!(output_text.contains("\"id\":4"));
    assert!(output_text.contains("\"data\":["));
}

#[test]
fn framed_lsp_workspace_overlay_updates_hover_and_definition() {
    let root = temp_test_dir("workspace_overlay");
    let scripts = root.join("Scripts");
    std::fs::create_dir_all(&scripts).unwrap();
    let workspace_file = scripts.join("WorkspaceThing.c");
    let user_file = scripts.join("User.c");
    let workspace_source = "class WorkspaceThing\n{\n\tvoid WorkspaceMethod();\n}\n";
    std::fs::write(&workspace_file, workspace_source).unwrap();

    let user_source = "class User\n{\n\tvoid Run()\n\t{\n\t\tWorkspaceThing thing;\n\t\tthing.WorkspaceMethod();\n\t}\n}\n";
    let hover_position =
        position_for_needle(user_source, "thing.WorkspaceMethod", "WorkspaceMethod");
    let completion_position = position_after_needle(user_source, "thing.");
    let definition_position =
        position_for_needle(user_source, "WorkspaceThing thing", "WorkspaceThing");
    let user_uri = file_uri_for_path(&user_file).unwrap();
    let target_uri = file_uri_for_path(&workspace_file).unwrap();
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": user_uri,
                    "languageId": "enforce",
                    "version": 1,
                    "text": user_source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": WORKSPACE_FILE_CHANGED_METHOD,
            "params": {
                "path": workspace_file.display().to_string(),
                "text": workspace_source,
                "sequence": 1
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": user_uri
                },
                "position": {
                    "line": hover_position.line,
                    "character": hover_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": user_uri
                },
                "position": {
                    "line": completion_position.line,
                    "character": completion_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {
                    "uri": user_uri
                },
                "position": {
                    "line": definition_position.line,
                    "character": definition_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": WORKSPACE_FILE_DELETED_METHOD,
            "params": {
                "path": workspace_file.display().to_string(),
                "sequence": 2
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": WORKSPACE_FILE_CHANGED_METHOD,
            "params": {
                "path": workspace_file.display().to_string(),
                "text": workspace_source,
                "sequence": 1
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": user_uri
                },
                "position": {
                    "line": completion_position.line,
                    "character": completion_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": user_uri
                },
                "position": {
                    "line": hover_position.line,
                    "character": hover_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(input.as_slice(), &mut output, LspServerOptions::default()).unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("WorkspaceThing.WorkspaceMethod() -> void"));
    assert!(output_text.contains("\"label\":\"WorkspaceMethod\""));
    assert!(output_text.contains(&target_uri));
    assert!(output_text.contains(
        "{\"id\":5,\"jsonrpc\":\"2.0\",\"result\":{\"isIncomplete\":true,\"items\":[]}}"
    ));
    assert!(output_text.contains("{\"id\":6,\"jsonrpc\":\"2.0\",\"result\":null}"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn framed_lsp_uses_cached_analysis_for_repeated_hover() {
    let source = "class Smoke\n{\n\tvoid Run(int value);\n}\n";
    let hover_position = position_for_needle(source, "Run(int", "Run");
    let log_path = test_log_path("cached_hover");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    for id in [2, 3] {
        write_test_message(
            &mut input,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": {
                        "uri": "file:///Scripts/Smoke.c"
                    },
                    "position": {
                        "line": hover_position.line,
                        "character": hover_position.character
                    }
                }
            }),
        );
    }
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: Some(log_path.clone()),
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert_eq!(output_text.matches("void Run(int value)").count(), 2);

    let log = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(log.matches("notification didOpen").count(), 1);
    assert_eq!(log.matches("analysis_elapsed_ms=").count(), 1);
    assert_eq!(log.matches("request hover").count(), 2);
    assert_eq!(
        log.matches("request hover").count(),
        log.matches("cached_analysis=true").count() - 1
    );

    cleanup_log(&log_path);
}

#[test]
fn framed_lsp_did_change_replaces_cached_analysis() {
    let old_source = "class Old\n{\n\tvoid OldRun();\n}\n";
    let new_source = "class New\n{\n\tvoid NewRun();\n}\n";
    let hover_position = position_for_needle(new_source, "NewRun", "NewRun");
    let definition_position = position_for_needle(new_source, "NewRun", "NewRun");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Changed.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": old_source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Changed.c",
                    "version": 2
                },
                "contentChanges": [
                    {
                        "text": new_source
                    }
                ]
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Changed.c"
                },
                "position": {
                    "line": hover_position.line,
                    "character": hover_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Changed.c"
                },
                "position": {
                    "line": definition_position.line,
                    "character": definition_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Changed.c"
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: None,
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("New.NewRun() -> void"));
    assert!(output_text.contains("\"name\":\"New\""));
    assert!(output_text.contains("\"name\":\"NewRun\""));
    assert!(output_text.contains("\"uri\":\"file:///Scripts/Changed.c\""));
    assert!(!output_text.contains("\"name\":\"Old\""));
    assert!(!output_text.contains("\"name\":\"OldRun\""));
}

#[test]
fn framed_lsp_did_close_removes_cached_document() {
    let source = "class Closed {}\n";
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Closed.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didClose",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Closed.c"
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Closed.c"
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: None,
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("\"result\":null"));
    assert!(!output_text.contains("\"name\":\"Closed\""));
}

#[test]
fn framed_lsp_publishes_and_clears_parser_diagnostics() {
    let broken_source = "class Broken\n{\n\tvoid Run(\n}\n";
    let fixed_source = "class Fixed\n{\n\tvoid Run();\n}\n";
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Diagnostics.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": broken_source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Diagnostics.c",
                    "version": 2
                },
                "contentChanges": [
                    {
                        "text": fixed_source
                    }
                ]
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didClose",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Diagnostics.c"
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: None,
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert_eq!(
        output_text
            .matches("textDocument/publishDiagnostics")
            .count(),
        3
    );
    assert!(output_text.contains("Reforger Script Tools parser"));
    assert!(output_text.contains("reforger.parser.syntax"));
    assert!(output_text.contains("\"severity\":1"));
    assert!(output_text.contains("\"version\":1"));
    assert!(output_text.contains("\"version\":2"));
    assert!(
        clear_diagnostics_message("file:///Scripts/Diagnostics.c")["params"]
            .get("version")
            .is_none()
    );
    assert!(output_text.contains("\"diagnostics\":[]"));
}

#[test]
fn framed_lsp_ignores_stale_changes_without_regressing_diagnostics_or_symbols() {
    let initial_source = "class Initial\n{\n\tvoid InitialRun(\n}\n";
    let current_source = "class Current\n{\n\tvoid CurrentRun();\n}\n";
    let stale_source = "class Stale\n{\n\tvoid StaleRun(\n}\n";
    let uri = "file:///Scripts/VersionedDiagnostics.c";
    let log_path = test_log_path("stale_diagnostic_change");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "enforce",
                    "version": 1,
                    "text": initial_source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "text": current_source }]
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": stale_source }]
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentSymbol",
            "params": { "textDocument": { "uri": uri } }
        }),
    );
    write_test_message(
        &mut input,
        json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": null }),
    );
    write_test_message(
        &mut input,
        json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: Some(log_path.clone()),
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert_eq!(
        output_text
            .matches("textDocument/publishDiagnostics")
            .count(),
        2
    );
    assert!(output_text.contains("\"version\":1"));
    assert!(output_text.contains("\"version\":3"));
    assert!(!output_text.contains("\"version\":2"));
    assert!(output_text.contains("\"name\":\"Current\""));
    assert!(output_text.contains("\"name\":\"CurrentRun\""));
    assert!(!output_text.contains("\"name\":\"Stale\""));
    assert!(!output_text.contains("\"name\":\"StaleRun\""));

    let log = std::fs::read_to_string(&log_path).unwrap();
    assert!(log.contains(
        "notification didChange ignored uri=file:///Scripts/VersionedDiagnostics.c version=2 current_version=3 reason=stale"
    ));
    cleanup_log(&log_path);
}

#[test]
fn framed_lsp_smoke_test_handles_debug_hover_request() {
    let source = "class Smoke\n{\n\tvoid Run(int value);\n}\n";
    let hover_position = position_for_needle(source, "Run(int", "Run");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "reforger/debugHover",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/Smoke.c"
                },
                "position": {
                    "line": hover_position.line,
                    "character": hover_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: None,
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("# Reforger Hover Debug"));
    assert!(output_text.contains("Smoke.Run(int value) -> void"));
    assert!(output_text.contains("Candidate Symbols At Cursor"));
}

#[test]
fn framed_lsp_smoke_test_handles_debug_completion_request() {
    let source = "class Smoke\n{\n\tvoid Run()\n\t{\n\t\tSmoke value;\n\t\tvalue.\n\t}\n}\n";
    let completion_position = position_after_needle(source, "value.");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/SmokeCompletion.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "reforger/debugCompletion",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/SmokeCompletion.c"
                },
                "position": {
                    "line": completion_position.line,
                    "character": completion_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: None,
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("# Reforger Completion Debug"));
    assert!(output_text.contains("## Completion Context"));
    assert!(output_text.contains("## Signature Help Context"));
    assert!(output_text.contains("not in callable argument list"));
    assert!(output_text.contains("value"));
    assert!(output_text.contains("Run"));
    assert!(!output_text.contains("Method not found"));
}

#[test]
fn framed_lsp_debug_completion_includes_signature_help_when_inside_call() {
    let source = "class Smoke\n{\n\tvoid Run(int value, string label = \"ok\");\n\tvoid Test()\n\t{\n\t\tRun(1, );\n\t}\n}\n";
    let completion_position = position_after_needle(source, "Run(1, ");
    let mut input = Vec::new();
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/SmokeSignatureDebug.c",
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "reforger/debugCompletion",
            "params": {
                "textDocument": {
                    "uri": "file:///Scripts/SmokeSignatureDebug.c"
                },
                "position": {
                    "line": completion_position.line,
                    "character": completion_position.character
                }
            }
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    );
    write_test_message(
        &mut input,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let mut output = Vec::new();
    run(
        input.as_slice(),
        &mut output,
        LspServerOptions {
            log_path: None,
            diagnostic_log_path: None,
            game_data_scripts: None,
            game_data_metadata: None,
            index_cache: None,
            workspace_scripts: Vec::new(),
        },
    )
    .unwrap();

    let output_text = String::from_utf8(output).unwrap();
    assert!(output_text.contains("# Reforger Completion Debug"));
    assert!(output_text.contains("## Signature Help Context"));
    assert!(output_text.contains("- Active Parameter: `1`"));
    assert!(output_text.contains("Smoke.Run(int value, string label = \\\"ok\\\") -> void"));
    assert!(output_text.contains("string label = \\\"ok\\\""));
}

fn assert_ranges_are_sane(symbols: &[LspDocumentSymbol]) {
    for symbol in symbols {
        assert!(
            range_contains(symbol.range, symbol.selection_range),
            "selection range must be inside declaration range for {}",
            symbol.name
        );
        assert_ranges_are_sane(&symbol.children);
    }
}

fn range_contains(outer: LspRange, inner: LspRange) -> bool {
    position_le(outer.start, inner.start) && position_le(inner.end, outer.end)
}

fn position_le(left: LspPosition, right: LspPosition) -> bool {
    (left.line, left.character) <= (right.line, right.character)
}

fn write_test_message(output: &mut Vec<u8>, value: Value) {
    let body = serde_json::to_vec(&value).unwrap();
    write!(output, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
    output.extend_from_slice(&body);
}

#[test]
fn read_message_rejects_an_oversized_header_before_parsing() {
    let input = format!("X-Long: {}\r\n\r\n", "x".repeat(16 * 1024));
    let error = read_message(&mut BufReader::new(input.as_bytes())).unwrap_err();

    assert_eq!(error, "LSP header line exceeds the configured limit");
}

fn assert_hover(
    source: &str,
    needle: &str,
    cursor: &str,
    expected_kind: SymbolKind,
    expected_label: &str,
) {
    let report = hover_at(source, needle, cursor);

    assert_eq!(report.parse_diagnostics, 0);
    assert_eq!(
        report.selected_kind,
        Some(expected_kind),
        "hover kind mismatch for needle `{needle}` cursor `{cursor}`"
    );
    assert_eq!(
        report.selected_label.as_deref(),
        Some(expected_label),
        "hover label mismatch for needle `{needle}` cursor `{cursor}`"
    );
    assert!(report.hover.is_some());
    assert_eq!(
        report.selection_source,
        HoverSelectionSource::ResolverIdentifier
    );
    assert!(report.resolver_candidate_count > 0);
}

fn hover_at(source: &str, needle: &str, cursor: &str) -> LspHoverReport {
    hover_report_for_source_position(source, position_for_needle(source, needle, cursor))
}

fn assert_definition(
    source: &str,
    uri: &str,
    needle: &str,
    cursor: &str,
    expected_kind: SymbolKind,
    expected_label: &str,
    expected_uri: &str,
) {
    let report = definition_report_for_source_position(
        source,
        uri,
        position_for_needle(source, needle, cursor),
    );
    assert!(
        report.is_hit(),
        "definition miss for needle `{needle}` cursor `{cursor}`"
    );
    assert_eq!(report.parse_diagnostics, 0);
    assert_eq!(report.selected_kind, Some(expected_kind));
    assert_eq!(report.selected_label.as_deref(), Some(expected_label));
    assert_eq!(report.locations.len(), 1);
    assert_eq!(report.locations[0].uri, expected_uri);
}

fn definition_at(source: &str, needle: &str, cursor: &str) -> LspDefinitionReport {
    definition_report_for_source_position(
        source,
        "file:///Scripts/Definition.c",
        position_for_needle(source, needle, cursor),
    )
}

fn position_for_needle(source: &str, needle: &str, cursor: &str) -> LspPosition {
    let start = source
        .find(needle)
        .unwrap_or_else(|| panic!("missing needle {needle}"));
    let cursor_start = needle
        .find(cursor)
        .unwrap_or_else(|| panic!("missing cursor {cursor} in {needle}"));
    position_for_offset(source, start + cursor_start)
}

fn position_after_needle(source: &str, needle: &str) -> LspPosition {
    let start = source
        .find(needle)
        .unwrap_or_else(|| panic!("missing needle {needle}"));
    position_for_offset(source, start + needle.len())
}

fn assert_semantic_token(
    report: &LspSemanticTokenReport,
    text: &str,
    token_type: &str,
    color: Option<&str>,
) {
    assert!(
        report.decoded.iter().any(|token| {
            token.text == text
                && token.token_type == token_type
                && color.is_none_or(|color| token.color == color)
        }),
        "missing semantic token text={text:?} type={token_type:?} color={color:?}: {:?}",
        report.decoded
    );
}

fn assert_semantic_token_count_at_least(
    report: &LspSemanticTokenReport,
    text: &str,
    token_type: &str,
    expected: usize,
) {
    let actual = report
        .decoded
        .iter()
        .filter(|token| token.text == text && token.token_type == token_type)
        .count();
    assert!(
        actual >= expected,
        "expected at least {expected} semantic tokens text={text:?} type={token_type:?}, found {actual}: {:?}",
        report.decoded
    );
}

fn assert_semantic_type_family_token_count_at_least(
    report: &LspSemanticTokenReport,
    text: &str,
    expected: usize,
) {
    let actual = report
        .decoded
        .iter()
        .filter(|token| {
            token.text == text
                && ((matches!(token.token_type, "class" | "type" | "typeParameter")
                    && token.color == "#40b5ac")
                    || (token.token_type == "enum" && token.color == "#40b5ac"))
        })
        .count();
    assert!(
        actual >= expected,
        "expected at least {expected} type-family semantic tokens text={text:?}, found {actual}: {:?}",
        report.decoded
    );
}
