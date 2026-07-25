use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn executable_rejects_unknown_modes_without_writing_protocol_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_reforger_language_server"))
        .arg("unknown-mode")
        .stdin(Stdio::null())
        .output()
        .expect("run language server");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown mode"),
        "stderr should explain the rejected mode: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn mcp_stdio_initializes_lists_and_reports_game_data_status() {
    let fixture = TempFixture::new("mcp_status");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(scripts_root.join("Game")).expect("create scripts fixture");
    fs::write(
        scripts_root.join("Game").join("McpFixture.c"),
        "class McpFixture\n{\n\tvoid Run()\n\t{\n\t}\n}\n",
    )
    .expect("write game-data fixture");
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");

    let mut client = McpClient::spawn(&[
        "mcp",
        "--game-data-scripts",
        scripts_root.to_str().expect("utf-8 scripts path"),
        "--index-cache",
        cache_path.to_str().expect("utf-8 cache path"),
    ]);

    let initialize = client.initialize(1);
    assert_eq!(
        initialize.pointer("/result/protocolVersion"),
        Some(&json!("2025-11-25"))
    );
    assert_eq!(
        initialize.pointer("/result/capabilities/tools/listChanged"),
        Some(&json!(false))
    );
    assert_eq!(
        initialize.pointer("/result/serverInfo/name"),
        Some(&json!("reforger-script-tools"))
    );

    client.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }));
    let tools = client.response(2);
    let listed = tools
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .expect("tools/list result");
    assert_eq!(listed.len(), 7);
    assert_eq!(listed[0].get("name"), Some(&json!("game_data_status")));
    assert_eq!(
        listed[0].pointer("/annotations/readOnlyHint"),
        Some(&json!(true))
    );
    assert_eq!(
        listed[0].pointer("/annotations/openWorldHint"),
        Some(&json!(false))
    );
    assert_eq!(
        listed[0].pointer("/inputSchema/additionalProperties"),
        Some(&json!(false))
    );
    assert!(listed[0].get("outputSchema").is_some());
    assert_eq!(
        listed[1].get("name"),
        Some(&json!("search_game_data_symbols"))
    );
    assert_eq!(
        listed[1].pointer("/annotations/readOnlyHint"),
        Some(&json!(true))
    );
    assert_eq!(
        listed[1].pointer("/annotations/openWorldHint"),
        Some(&json!(false))
    );
    assert_eq!(
        listed[1].pointer("/inputSchema/additionalProperties"),
        Some(&json!(false))
    );
    assert_eq!(
        listed[1].pointer("/inputSchema/required/0"),
        Some(&json!("query"))
    );
    assert_eq!(
        listed[2].get("name"),
        Some(&json!("inspect_game_data_symbol"))
    );
    assert_eq!(listed[3].get("name"), Some(&json!("read_game_data_source")));
    assert_eq!(listed[4].get("name"), Some(&json!("official_wiki_status")));
    assert_eq!(
        listed[4].pointer("/annotations/readOnlyHint"),
        Some(&json!(true))
    );
    assert_eq!(
        listed[4].pointer("/inputSchema/additionalProperties"),
        Some(&json!(false))
    );
    assert_eq!(listed[5].get("name"), Some(&json!("search_official_wiki")));
    assert_eq!(listed[6].get("name"), Some(&json!("read_official_wiki")));

    client.send(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "game_data_status",
            "arguments": {}
        }
    }));
    let call = client.response(3);
    assert_eq!(call.pointer("/result/isError"), Some(&json!(false)));
    let structured = call
        .pointer("/result/structuredContent")
        .cloned()
        .expect("structured status result");
    assert_eq!(structured.get("available"), Some(&json!(true)));
    assert_eq!(
        structured.pointer("/source/acquisition"),
        Some(&json!("manual"))
    );
    assert_eq!(
        structured.pointer("/authorities/sourceEvidence"),
        Some(&json!("evidence-catalogue"))
    );
    assert_eq!(
        structured.pointer("/authorities/sourceMetadata"),
        Some(&json!("filesystem"))
    );
    assert_eq!(
        structured.pointer("/authorities/semanticCatalogue"),
        Some(&json!("language-engine"))
    );
    assert_eq!(
        structured.pointer("/cache/outcome"),
        Some(&json!("rebuilt"))
    );
    assert_eq!(structured.pointer("/coverage/files"), Some(&json!(1)));
    assert!(structured
        .get("catalogueRevision")
        .and_then(Value::as_str)
        .is_some_and(|revision| revision.starts_with("gd1:")));
    assert!(structured.get("limits").is_some());
    assert!(structured.get("warnings").is_some());
    assert!(structured.get("recovery").is_some());
    assert!(
        !structured
            .to_string()
            .contains(fixture.path().to_str().expect("utf-8 fixture path")),
        "status must not disclose physical paths"
    );

    let compatibility_text = call
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .expect("compatibility JSON text");
    assert_eq!(
        serde_json::from_str::<Value>(compatibility_text).expect("valid compatibility JSON"),
        structured
    );

    let search_started = std::time::Instant::now();
    client.send(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": { "name": "search_game_data_symbols", "arguments": { "query": "McpFixture" } }
    }));
    let search = client.response(4);
    assert!(
        search_started.elapsed() < Duration::from_secs(5),
        "ready-catalogue search exceeded the five-second ceiling"
    );
    assert_eq!(search.pointer("/result/isError"), Some(&json!(false)));
    let results = search
        .pointer("/result/structuredContent/results")
        .and_then(Value::as_array)
        .expect("search results");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].get("name"), Some(&json!("McpFixture")));
    assert!(results[0].get("symbolRef").is_some());
    assert_eq!(
        results[0].pointer("/inspectInput/symbolRef"),
        results[0].get("symbolRef")
    );
    assert_eq!(
        results[0].pointer("/readSourceInput/relativePath"),
        Some(&json!("Game/McpFixture.c"))
    );

    client.send(json!({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call",
        "params": { "name": "inspect_game_data_symbol", "arguments": results[0]["inspectInput"] }
    }));
    let inspection = client.response(5);
    assert_eq!(inspection.pointer("/result/isError"), Some(&json!(false)));
    let inspected = inspection
        .pointer("/result/structuredContent")
        .expect("inspection result");
    assert_eq!(
        inspected.pointer("/qualifiedName"),
        Some(&json!("McpFixture"))
    );
    assert_eq!(
        inspected.pointer("/relativePath"),
        Some(&json!("Game/McpFixture.c"))
    );

    client.send(json!({
        "jsonrpc": "2.0", "id": 6, "method": "tools/call",
        "params": { "name": "read_game_data_source", "arguments": results[0]["readSourceInput"] }
    }));
    let source = client.response(6);
    assert_eq!(source.pointer("/result/isError"), Some(&json!(false)));
    assert!(source
        .pointer("/result/structuredContent/content")
        .and_then(Value::as_str)
        .is_some_and(|text| text.contains("McpFixture")));

    client.close_stdin();
    assert!(
        client.wait_for_exit(Duration::from_secs(3)),
        "MCP process should exit promptly after stdin EOF"
    );
}

#[test]
fn mcp_inspection_and_source_read_reject_stale_and_changed_handoffs() {
    let fixture = TempFixture::new("mcp_inspection_contract");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(scripts_root.join("Game")).expect("create scripts fixture");
    let source_path = scripts_root.join("Game").join("Inspectable.c");
    fs::write(
        &source_path,
        "//! \\brief Inspectable summary.\nclass Inspectable\n{\n\t/*! \\param[in] value input value.\n\t * \\return true when accepted.\n\t * \\warning requires setup.\n\t * \\note fixture note.\n\t */\n\tbool Run(int value);\n}\n",
    )
    .expect("write game-data fixture");
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");
    let mut client = McpClient::spawn(&[
        "mcp",
        "--game-data-scripts",
        scripts_root.to_str().expect("utf-8 scripts path"),
        "--index-cache",
        cache_path.to_str().expect("utf-8 cache path"),
    ]);
    client.initialize(1);
    client.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_game_data_symbols","arguments":{"query":"Run"}}}));
    let search = client.response(2);
    let result = search
        .pointer("/result/structuredContent/results/0")
        .expect("search hit");
    let symbol_ref = result.get("symbolRef").cloned().expect("symbol reference");
    let revision = search
        .pointer("/result/structuredContent/catalogueRevision")
        .cloned()
        .expect("revision");

    client.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"inspect_game_data_symbol","arguments":{"symbolRef":symbol_ref}}}));
    let inspected = client.response(3);
    assert_eq!(inspected.pointer("/result/isError"), Some(&json!(false)));
    assert_eq!(
        inspected.pointer("/result/structuredContent/documentation/parameters/0/name"),
        Some(&json!("value"))
    );
    assert_eq!(
        inspected.pointer("/result/structuredContent/documentation/parameters/0/direction"),
        Some(&json!("in"))
    );
    assert_eq!(
        inspected.pointer("/result/structuredContent/documentation/returns"),
        Some(&json!("true when accepted."))
    );
    assert_eq!(
        inspected.pointer("/result/structuredContent/documentation/warnings/0"),
        Some(&json!("requires setup."))
    );
    assert_eq!(
        inspected.pointer("/result/structuredContent/documentation/notes/0"),
        Some(&json!("fixture note."))
    );

    client.send(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"inspect_game_data_symbol","arguments":{"symbolRef":"sr1:not-a-reference"}}}));
    let invalid = client.response(4);
    assert_eq!(invalid.pointer("/result/isError"), Some(&json!(true)));
    assert!(invalid
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("invalid_symbol_ref:")));

    client.send(json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"read_game_data_source","arguments":{"catalogueRevision":revision,"relativePath":"../Inspectable.c"}}}));
    let invalid_path = client.response(5);
    assert_eq!(invalid_path.pointer("/result/isError"), Some(&json!(true)));
    assert!(invalid_path
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("invalid_arguments:")));

    fs::write(&source_path, "class Inspectable { int changed; }\n").expect("change backing data");
    client.send(json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"read_game_data_source","arguments":{"catalogueRevision":revision,"relativePath":"Game/Inspectable.c"}}}));
    let changed = client.response(6);
    assert_eq!(changed.pointer("/result/isError"), Some(&json!(true)));
    assert!(changed
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("game_data_changed:")));
}

#[test]
fn mcp_progressive_retrieval_enforces_member_documentation_and_source_bounds() {
    let fixture = TempFixture::new("mcp_progressive_bounds");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(scripts_root.join("Game")).expect("create scripts fixture");
    let source_path = scripts_root.join("Game").join("Bounds.c");
    let mut source = "/*! \\brief Bounds summary.\n".repeat(1);
    for _ in 0..3_000 {
        source.push_str(" * documentation payload keeps the raw comment bounded.\n");
    }
    source.push_str(" */\nclass Bounds\n{\n");
    for index in 0..55 {
        source.push_str(&format!("\tint Member{index:02};\n"));
    }
    source.push_str("}\n");
    for index in 0..600 {
        source.push_str(&format!("int TopLevel{index:03};\n"));
    }
    fs::write(&source_path, source).expect("write bounds fixture");
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");
    let mut client = McpClient::spawn(&[
        "mcp",
        "--game-data-scripts",
        scripts_root.to_str().expect("utf-8 scripts path"),
        "--index-cache",
        cache_path.to_str().expect("utf-8 cache path"),
    ]);
    client.initialize(1);
    client.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_game_data_symbols","arguments":{"query":"Bounds"}}}));
    let search = client.response(2);
    let hit = search
        .pointer("/result/structuredContent/results/0")
        .expect("Bounds hit");
    let symbol_ref = hit.get("symbolRef").cloned().expect("symbol reference");
    let revision = search
        .pointer("/result/structuredContent/catalogueRevision")
        .cloned()
        .expect("catalogue revision");

    client.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"inspect_game_data_symbol","arguments":{"symbolRef":symbol_ref}}}));
    let inspected = client.response(3);
    assert_eq!(inspected.pointer("/result/isError"), Some(&json!(false)));
    let inspection = inspected
        .pointer("/result/structuredContent")
        .expect("inspection");
    assert_eq!(inspection.pointer("/rawTruncated"), Some(&json!(true)));
    assert!(inspection
        .pointer("/rawDocumentation")
        .and_then(Value::as_str)
        .is_some_and(|text| text.len() <= 16 * 1024));
    assert_eq!(inspection.pointer("/membersReturned"), Some(&json!(50)));
    assert_eq!(inspection.pointer("/membersTotal"), Some(&json!(55)));
    assert_eq!(inspection.pointer("/membersTruncated"), Some(&json!(true)));
    assert_eq!(
        inspection.pointer("/members/0/name"),
        Some(&json!("Member00"))
    );
    assert_eq!(
        inspection.pointer("/members/49/name"),
        Some(&json!("Member49"))
    );
    assert!(inspection
        .pointer("/membersTruncationGuidance")
        .and_then(Value::as_str)
        .is_some_and(|text| text.contains("search_game_data_symbols")));

    client.send(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"read_game_data_source","arguments":{"catalogueRevision":revision,"relativePath":"Game/Missing.c"}}}));
    assert!(client
        .response(4)
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("invalid_arguments:")));
    client.send(json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"read_game_data_source","arguments":{"catalogueRevision":"gd1:stale","relativePath":"Game/Bounds.c"}}}));
    assert!(client
        .response(5)
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("stale_symbol_ref:")));
    client.send(json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"read_game_data_source","arguments":{"catalogueRevision":revision,"relativePath":"Game/Bounds.c","startLine":0}}}));
    assert!(client
        .response(6)
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("invalid_arguments:")));
    client.send(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"read_game_data_source","arguments":{"catalogueRevision":revision,"relativePath":"Game/Bounds.c","startLine":1,"lineCount":999}}}));
    let read = client.response(7);
    assert_eq!(read.pointer("/result/isError"), Some(&json!(false)));
    assert_eq!(
        read.pointer("/result/structuredContent/startLine"),
        Some(&json!(1))
    );
    assert_eq!(
        read.pointer("/result/structuredContent/endLine"),
        Some(&json!(500))
    );
    assert_eq!(
        read.pointer("/result/structuredContent/truncated"),
        Some(&json!(true))
    );
    assert_eq!(
        read.pointer("/result/structuredContent/nextStartLine"),
        Some(&json!(501))
    );
}

#[test]
fn game_data_revision_is_immutable_per_process_and_shared_cache_loads_warm() {
    let fixture = TempFixture::new("mcp_immutable_revision");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(&scripts_root).expect("create scripts fixture");
    fs::write(
        scripts_root.join("RevisionFixture.c"),
        "class RevisionFixture {}",
    )
    .expect("write game-data fixture");
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");
    let arguments = [
        "mcp",
        "--game-data-scripts",
        scripts_root.to_str().expect("utf-8 scripts path"),
        "--index-cache",
        cache_path.to_str().expect("utf-8 cache path"),
    ];

    let mut first = McpClient::spawn(&arguments);
    first.initialize(1);
    let cold = first.call_status(2);
    let repeated = first.call_status(3);
    assert_eq!(cold, repeated, "one process retains one immutable snapshot");
    assert_eq!(cold.pointer("/cache/outcome"), Some(&json!("rebuilt")));
    let revision = cold
        .get("catalogueRevision")
        .cloned()
        .expect("cold catalogue revision");
    first.close_stdin();
    assert!(first.wait_for_exit(Duration::from_secs(3)));

    let mut second = McpClient::spawn(&arguments);
    second.initialize(4);
    let warm = second.call_status(5);
    assert_eq!(warm.pointer("/cache/outcome"), Some(&json!("loaded")));
    assert_eq!(warm.get("catalogueRevision"), Some(&revision));
    second.close_stdin();
    assert!(second.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn unavailable_status_and_malformed_calls_are_sanitized_and_process_isolated() {
    let mut client = McpClient::spawn(&["mcp"]);
    client.initialize(1);

    let unavailable = client.call_status(2);
    assert_eq!(unavailable.get("available"), Some(&json!(false)));
    assert_eq!(
        unavailable.pointer("/warnings/0/code"),
        Some(&json!("game_data_not_configured"))
    );
    assert_eq!(
        unavailable.pointer("/limits/initializationDeadlineMs"),
        Some(&json!(120_000))
    );
    assert!(
        !unavailable.to_string().contains(":\\"),
        "sanitized status must not contain a Windows physical path"
    );

    client.send(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "game_data_status",
            "arguments": {"unexpected": true}
        }
    }));
    let invalid = client.response(3);
    assert_eq!(invalid.pointer("/error/code"), Some(&json!(-32602)));
    assert!(invalid
        .pointer("/error/message")
        .and_then(Value::as_str)
        .is_some_and(|message| message.contains("empty object")));

    client.send(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "not_a_tool",
            "arguments": {}
        }
    }));
    assert_eq!(
        client.response(4).pointer("/error/code"),
        Some(&json!(-32602))
    );

    client.send(json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "search_game_data_symbols",
            "arguments": {"query": "", "unexpected": true}
        }
    }));
    assert_eq!(
        client.response(5).pointer("/error/code"),
        Some(&json!(-32602))
    );

    client.send(json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "ping",
        "params": {}
    }));
    assert_eq!(client.response(6).get("result"), Some(&json!({})));

    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn search_official_wiki_projects_validated_sections_and_keeps_the_session_healthy() {
    let fixture = TempFixture::new("official_wiki_search");
    let wiki_root = fixture.path().join("official-wiki");
    fs::create_dir_all(wiki_root.join("Guides")).expect("create wiki fixture");
    fs::write(
        wiki_root.join("wiki-index.md"),
        "# Wiki Markdown Index\nneedle ignored\n",
    )
    .expect("write index");
    fs::write(wiki_root.join("Guides").join("Guide.md"), "# [Guide](https://community.bistudio.com/wiki/Arma_Reforger:Guide)\n\n## Needle\nneedle prose\n\n## Another\nneedle prose\n").expect("write page");
    let mut client = McpClient::spawn(&[
        "mcp",
        "--official-wiki-root",
        wiki_root.to_str().expect("utf-8 wiki root"),
    ]);
    client.initialize(1);
    client.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_official_wiki","arguments":{"query":"needle","pathPrefix":"Guides","limit":1}}}));
    let response = client.response(2);
    let page = response
        .pointer("/result/structuredContent")
        .expect("structured search page");
    assert_eq!(page.get("returned"), Some(&json!(1)));
    assert_eq!(page.get("total"), Some(&json!(2)));
    assert_eq!(page.pointer("/results/0/heading"), Some(&json!("Needle")));
    assert_eq!(
        page.pointer("/results/0/readInput/relativePath"),
        Some(&json!("Guides/Guide.md"))
    );
    assert_eq!(
        response.pointer("/result/content/0/text"),
        Some(&Value::String(page.to_string()))
    );
    let cursor = page.get("nextCursor").cloned().expect("next cursor");
    client.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search_official_wiki","arguments":{"query":"needle","pathPrefix":"Guides/","cursor":cursor}}}));
    assert_eq!(
        client
            .response(3)
            .pointer("/result/structuredContent/results/0/heading"),
        Some(&json!("Another"))
    );
    client.send(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"search_official_wiki","arguments":{"query":"","unexpected":true}}}));
    assert_eq!(
        client.response(4).pointer("/error/code"),
        Some(&json!(-32602))
    );
    client.send(json!({"jsonrpc":"2.0","id":5,"method":"ping","params":{}}));
    assert_eq!(client.response(5).get("result"), Some(&json!({})));
    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn read_official_wiki_follows_search_handoffs_with_bounded_continuations() {
    let fixture = TempFixture::new("official_wiki_read");
    let wiki_root = fixture.path().join("official-wiki");
    fs::create_dir_all(wiki_root.join("Guides")).expect("create wiki fixture");
    fs::write(
        wiki_root.join("Guides").join("Unicode.md"),
        "# [Unicode guide](https://community.bistudio.com/wiki/Arma_Reforger:Unicode)\n\n## Needle\nfirst caf\u{e9}\nsecond line\nthird line\n",
    )
    .expect("write page");
    let mut client = McpClient::spawn(&[
        "mcp",
        "--official-wiki-root",
        wiki_root.to_str().expect("utf-8 wiki root"),
    ]);
    client.initialize(1);
    client.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_official_wiki","arguments":{"query":"needle"}}}));
    let input = client
        .response(2)
        .pointer("/result/structuredContent/results/0/readInput")
        .cloned()
        .expect("copy-ready read input");
    client.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_official_wiki","arguments":input}}));
    let page = client
        .response(3)
        .pointer("/result/structuredContent")
        .cloned()
        .expect("structured wiki read");
    assert_eq!(page.get("relativePath"), Some(&json!("Guides/Unicode.md")));
    assert_eq!(page.get("sourceUrl"), Some(&json!("https://community.bistudio.com/wiki/Arma_Reforger:Unicode")));
    assert_eq!(page.get("startLine"), Some(&json!(3)));
    assert_eq!(page.get("endLine"), Some(&json!(6)));
    assert_eq!(page.get("content"), Some(&json!("## Needle\nfirst caf\u{e9}\nsecond line\nthird line\n")));
    assert_eq!(page.get("truncated"), Some(&json!(false)));
    assert_eq!(page.get("continuation"), Some(&Value::Null));
    client.send(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision": input["corpusRevision"], "relativePath":input["relativePath"], "startLine":3, "lineCount":2}}}));
    let bounded = client.response(4);
    assert_eq!(bounded.pointer("/result/structuredContent/content"), Some(&json!("## Needle\nfirst caf\u{e9}\n")));
    assert_eq!(bounded.pointer("/result/structuredContent/truncated"), Some(&json!(true)));
    assert_eq!(bounded.pointer("/result/structuredContent/continuation/startLine"), Some(&json!(5)));
    client.send(json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision": input["corpusRevision"], "relativePath":"../Unicode.md"}}}));
    assert_eq!(client.response(5).pointer("/result/content/0/text"), Some(&Value::String("invalid_path: relativePath must be an exact logical Official Wiki Markdown path. Recovery: Use a relative logical Markdown path returned by Official Wiki search.".to_string())));
    client.send(json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision": input["corpusRevision"], "relativePath":"Guides/Unicode.md", "unexpected":true}}}));
    assert_eq!(client.response(6).pointer("/error/code"), Some(&json!(-32602)));
    client.send(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision":"ow1:stale", "relativePath":"Guides/Unicode.md"}}}));
    assert!(client
        .response(7)
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("stale_corpus_revision:")));
    client.send(json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision": input["corpusRevision"], "relativePath":"Guides/Missing.md"}}}));
    assert!(client
        .response(8)
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("invalid_path:")));
    client.send(json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision": input["corpusRevision"], "relativePath":"Guides/Unicode.md", "startLine":0}}}));
    assert!(client
        .response(9)
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("invalid_range:")));
    client.send(json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision": input["corpusRevision"], "relativePath":"Guides/Unicode.md", "startLine":7}}}));
    assert!(client
        .response(10)
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("invalid_range:")));
    fs::write(
        wiki_root.join("Guides").join("Unicode.md"),
        "# [Unicode guide](https://community.bistudio.com/wiki/Arma_Reforger:Unicode)\nchanged\n",
    )
    .expect("change validated page");
    client.send(json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"read_official_wiki","arguments":input}}));
    assert!(client
        .response(11)
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("official_wiki_changed:")));
    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn read_official_wiki_rejects_a_reparse_escape_after_validation() {
    let fixture = TempFixture::new("official_wiki_reparse_escape");
    let wiki_root = fixture.path().join("official-wiki");
    let outside_root = fixture.path().join("outside");
    fs::create_dir_all(wiki_root.join("Escaped")).expect("create wiki fixture");
    fs::create_dir_all(&outside_root).expect("create outside fixture");
    fs::write(
        wiki_root.join("Escaped").join("Inside.md"),
        "# [Escaped](https://community.bistudio.com/wiki/Arma_Reforger:Escaped)\ntrusted\n",
    )
    .expect("write validated page");
    fs::write(
        outside_root.join("Inside.md"),
        "# [Outside](https://community.bistudio.com/wiki/Arma_Reforger:Outside)\nuntrusted\n",
    )
    .expect("write outside page");
    let mut client = McpClient::spawn(&[
        "mcp",
        "--official-wiki-root",
        wiki_root.to_str().expect("utf-8 wiki root"),
    ]);
    client.initialize(1);
    client.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_official_wiki","arguments":{"query":"escaped"}}}));
    let input = client
        .response(2)
        .pointer("/result/structuredContent/results/0/readInput")
        .cloned()
        .expect("validated read input");
    let escaped = wiki_root.join("Escaped");
    fs::remove_dir_all(&escaped).expect("remove validated page directory");
    create_directory_reparse_escape(&outside_root, &escaped);
    client.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_official_wiki","arguments":input}}));
    assert!(client
        .response(3)
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("official_wiki_changed:")));
    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

#[cfg(target_os = "windows")]
fn create_directory_reparse_escape(target: &Path, link: &Path) {
    assert!(
        Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .expect("create junction")
            .success(),
        "create reparse escape"
    );
}

#[cfg(unix)]
fn create_directory_reparse_escape(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).expect("create symlink escape");
}

#[test]
fn read_official_wiki_defaults_and_clamps_its_line_window() {
    let fixture = TempFixture::new("official_wiki_read_bounds");
    let wiki_root = fixture.path().join("official-wiki");
    fs::create_dir_all(&wiki_root).expect("create wiki fixture");
    let page = format!(
        "# [Bounds](https://community.bistudio.com/wiki/Arma_Reforger:Bounds)\n{}",
        (1..=600)
            .map(|line| format!("line {line}\n"))
            .collect::<String>()
    );
    fs::write(wiki_root.join("Bounds.md"), page).expect("write page");
    fs::write(
        wiki_root.join("Large.md"),
        format!(
            "# [Large](https://community.bistudio.com/wiki/Arma_Reforger:Large)\n{}\nfinal line\n",
            "x".repeat(128 * 1024 + 1),
        ),
    )
    .expect("write bounded page");
    let mut client = McpClient::spawn(&[
        "mcp",
        "--official-wiki-root",
        wiki_root.to_str().expect("utf-8 wiki root"),
    ]);
    client.initialize(1);
    client.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"official_wiki_status","arguments":{}}}));
    let revision = client
        .response(2)
        .pointer("/result/structuredContent/corpusRevision")
        .cloned()
        .expect("corpus revision");
    client.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision":revision,"relativePath":"Bounds.md","lineCount":1000}}}));
    let capped = client.response(3);
    assert_eq!(capped.pointer("/result/structuredContent/startLine"), Some(&json!(1)));
    assert_eq!(capped.pointer("/result/structuredContent/endLine"), Some(&json!(500)));
    assert_eq!(capped.pointer("/result/structuredContent/continuation/startLine"), Some(&json!(501)));
    client.send(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision":revision,"relativePath":"Bounds.md","startLine":501}}}));
    assert_eq!(client.response(4).pointer("/result/structuredContent/endLine"), Some(&json!(601)));
    client.send(json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision":revision,"relativePath":"Large.md"}}}));
    let bounded = client.response(5);
    let bounded_content = bounded
        .pointer("/result/structuredContent/content")
        .and_then(Value::as_str)
        .expect("bounded content");
    assert!(bounded_content.len() <= 128 * 1024);
    assert!(bounded_content.ends_with('\n'));
    assert_eq!(bounded.pointer("/result/structuredContent/endLine"), Some(&json!(1)));
    assert_eq!(bounded.pointer("/result/structuredContent/continuation/startLine"), Some(&json!(2)));
    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn read_official_wiki_cancellation_and_deadline_do_not_block_the_next_request() {
    let fixture = TempFixture::new("official_wiki_read_cancellation");
    let wiki_root = fixture.path().join("official-wiki");
    fs::create_dir_all(&wiki_root).expect("create wiki fixture");
    fs::write(
        wiki_root.join("Page.md"),
        "# [Page](https://community.bistudio.com/wiki/Arma_Reforger:Page)\ncontent\n",
    )
    .expect("write page");
    let mut client = McpClient::spawn_with_env(
        &[
            "mcp",
            "--official-wiki-root",
            wiki_root.to_str().expect("utf-8 wiki root"),
        ],
        &[("REFORGER_MCP_TEST_OFFICIAL_WIKI_READ_DELAY_MS", "5000")],
    );
    client.initialize(1);
    client.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"official_wiki_status","arguments":{}}}));
    let revision = client
        .response(2)
        .pointer("/result/structuredContent/corpusRevision")
        .cloned()
        .expect("corpus revision");
    client.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision":revision,"relativePath":"Page.md"}}}));
    client.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":3,"reason":"test cancellation"}}));
    client.send(json!({"jsonrpc":"2.0","id":4,"method":"ping","params":{}}));
    let responses = client.responses_until(4);
    assert!(responses.iter().all(|response| response.get("id") != Some(&json!(3))));
    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));

    let mut deadline_client = McpClient::spawn_with_env(
        &[
            "mcp",
            "--official-wiki-root",
            wiki_root.to_str().expect("utf-8 wiki root"),
        ],
        &[
            ("REFORGER_MCP_TEST_OFFICIAL_WIKI_READ_DELAY_MS", "5000"),
            ("REFORGER_MCP_TEST_OFFICIAL_WIKI_DEADLINE_MS", "50"),
        ],
    );
    deadline_client.initialize(1);
    deadline_client.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"official_wiki_status","arguments":{}}}));
    let deadline_revision = deadline_client
        .response(2)
        .pointer("/result/structuredContent/corpusRevision")
        .cloned()
        .expect("corpus revision");
    let started = std::time::Instant::now();
    deadline_client.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_official_wiki","arguments":{"corpusRevision":deadline_revision,"relativePath":"Page.md"}}}));
    let deadline = deadline_client.response(3);
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(deadline
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("deadline_exceeded:")));
    deadline_client.send(json!({"jsonrpc":"2.0","id":4,"method":"ping","params":{}}));
    assert_eq!(deadline_client.response(4).get("result"), Some(&json!({})));
    deadline_client.close_stdin();
    assert!(deadline_client.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn official_wiki_status_reports_a_validated_packaged_style_corpus() {
    let fixture = TempFixture::new("official_wiki_status");
    let wiki_root = fixture.path().join("official-wiki");
    fs::create_dir_all(wiki_root.join("Guides")).expect("create wiki fixture");
    fs::write(
        wiki_root.join("index.md"),
        "# [Official index](https://community.bistudio.com/wiki/Category:Arma_Reforger)\n\nAuthoritative index.\n",
    )
    .expect("write index");
    fs::write(
        wiki_root.join("Guides").join("Getting Started.md"),
        "# [Getting Started](https://community.bistudio.com/wiki/Arma_Reforger:Getting_Started)\n\nStart here.\n",
    )
    .expect("write page");
    fs::write(wiki_root.join("wiki-index.md"), "# Wiki Markdown Index\n")
        .expect("write excluded index");
    fs::write(
        wiki_root.join("Guides").join("untrusted.md"),
        "# [Untrusted](https://example.com/wiki/not-authoritative)\n",
    )
    .expect("write malformed page");

    let mut client = McpClient::spawn(&[
        "mcp",
        "--official-wiki-root",
        wiki_root.to_str().expect("utf-8 wiki root"),
    ]);
    client.initialize(1);
    client.send(json!({
        "jsonrpc":"2.0", "id":2, "method":"tools/call",
        "params":{"name":"official_wiki_status", "arguments":{}}
    }));
    let response = client.response(2);
    let status = response
        .pointer("/result/structuredContent")
        .expect("structured official wiki status");
    assert_eq!(status.get("available"), Some(&json!(true)));
    assert_eq!(status.get("source"), Some(&json!("evidence-catalogue")));
    assert_eq!(status.get("fileCount"), Some(&json!(2)));
    assert_eq!(status.get("invalidFileCount"), Some(&json!(1)));
    assert_eq!(
        status.get("invalidFiles"),
        Some(&json!(["Guides/untrusted.md"]))
    );
    assert_eq!(status.get("excludedFiles"), Some(&json!(["wiki-index.md"])));
    assert!(status
        .get("corpusRevision")
        .and_then(Value::as_str)
        .is_some_and(|revision| revision.starts_with("ow1:")));
    assert_eq!(
        response.pointer("/result/content/0/text"),
        Some(&Value::String(status.to_string()))
    );
    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn official_wiki_status_resolves_resources_from_an_installed_extension_layout() {
    let fixture = TempFixture::new("installed_official_wiki");
    let extension = fixture.path().join("extension");
    let runtime = extension
        .join("dist")
        .join("server")
        .join("test-platform")
        .join("reforger_language_server.exe");
    let wiki_root = extension.join("resources").join("official-wiki");
    fs::create_dir_all(runtime.parent().expect("runtime parent"))
        .expect("create runtime directory");
    fs::create_dir_all(&wiki_root).expect("create installed corpus");
    fs::copy(env!("CARGO_BIN_EXE_reforger_language_server"), &runtime)
        .expect("copy packaged runtime");
    fs::write(
        wiki_root.join("index.md"),
        "# [Official index](https://community.bistudio.com/wiki/Category:Arma_Reforger)\n",
    )
    .expect("write installed index");

    let mut client = McpClient::spawn_program(&runtime, &["mcp"]);
    client.initialize(1);
    client.send(json!({
        "jsonrpc":"2.0", "id":2, "method":"tools/call",
        "params":{"name":"official_wiki_status", "arguments":{}}
    }));
    assert_eq!(
        client
            .response(2)
            .pointer("/result/structuredContent/available"),
        Some(&json!(true))
    );
    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn cancellation_and_eof_with_in_flight_initialization_shutdown_cleanly() {
    let fixture = TempFixture::new("mcp_cancel");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(&scripts_root).expect("create scripts fixture");
    fs::write(
        scripts_root.join("CancellationFixture.c"),
        "class CancellationFixture { int m_Value; }",
    )
    .expect("write cancellation fixture");
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");
    let started_marker = fixture.path().join("initialization-started");
    let mut client = McpClient::spawn_with_env(
        &[
            "mcp",
            "--game-data-scripts",
            scripts_root.to_str().expect("utf-8 scripts path"),
            "--index-cache",
            cache_path.to_str().expect("utf-8 cache path"),
        ],
        &[
            ("REFORGER_MCP_TEST_INITIALIZATION_DELAY_MS", "5000"),
            (
                "REFORGER_MCP_TEST_INITIALIZATION_STARTED_MARKER",
                started_marker.to_str().expect("utf-8 marker path"),
            ),
        ],
    );
    client.initialize(1);
    client.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "search_game_data_symbols",
            "arguments": {"query": "CancellationFixture"}
        }
    }));
    wait_for_file(&started_marker, Duration::from_secs(2));
    client.send(json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancelled",
        "params": {
            "requestId": 2,
            "reason": "wire cancellation test"
        }
    }));
    client.send(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "ping",
        "params": {}
    }));

    let before_ping = client.responses_until(3);
    assert!(
        before_ping
            .iter()
            .all(|message| message.get("id") != Some(&json!(2))),
        "cancelled request must not publish a stale tool result"
    );
    client.close_stdin();
    assert!(
        client.wait_for_exit(Duration::from_secs(5)),
        "EOF must shut down even while blocking initialization unwinds"
    );
    assert!(
        !cache_path.exists(),
        "cancelled initialization must not continue into cache publication"
    );
}

#[test]
fn initialization_deadline_cancels_work_and_returns_stable_tool_error() {
    let fixture = TempFixture::new("mcp_deadline");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(&scripts_root).expect("create scripts fixture");
    fs::write(scripts_root.join("Deadline.c"), "class Deadline {}")
        .expect("write game-data fixture");
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");
    let started_marker = fixture.path().join("initialization-started");
    let mut client = McpClient::spawn_with_env(
        &[
            "mcp",
            "--game-data-scripts",
            scripts_root.to_str().expect("utf-8 scripts path"),
            "--index-cache",
            cache_path.to_str().expect("utf-8 cache path"),
        ],
        &[
            ("REFORGER_MCP_TEST_UNINTERRUPTIBLE_DELAY_MS", "5000"),
            ("REFORGER_MCP_TEST_INITIALIZATION_DEADLINE_MS", "50"),
            (
                "REFORGER_MCP_TEST_INITIALIZATION_STARTED_MARKER",
                started_marker.to_str().expect("utf-8 marker path"),
            ),
        ],
    );
    client.initialize(1);
    let call_started = std::time::Instant::now();
    client.send(status_call(2));
    let response = client.response(2);
    assert!(
        call_started.elapsed() < Duration::from_millis(500),
        "deadline response must not await a stalled blocking worker"
    );
    assert_eq!(response.pointer("/result/isError"), Some(&json!(true)));
    assert!(response
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("deadline_exceeded:")));
    assert!(started_marker.exists());
    for id in 3..6 {
        client.send(status_call(id));
        assert!(client
            .response(id)
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.starts_with("deadline_exceeded:")));
    }
    assert_eq!(
        file_line_count(&started_marker),
        1,
        "post-timeout retries must not spawn more blocking initialization workers"
    );

    client.close_stdin();
    assert!(
        client.wait_for_exit(Duration::from_secs(1)),
        "runtime shutdown must not wait for a stalled blocking worker"
    );
    assert!(
        !cache_path.exists(),
        "deadline cancellation must stop before cache publication"
    );
}

#[test]
fn ready_game_data_operations_use_their_own_five_second_deadline() {
    let fixture = TempFixture::new("mcp_ready_game_data_deadline");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(&scripts_root).expect("create scripts fixture");
    fs::write(scripts_root.join("Deadline.c"), "class Deadline {}")
        .expect("write game-data fixture");
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");
    let mut client = McpClient::spawn_with_env(
        &[
            "mcp",
            "--game-data-scripts",
            scripts_root.to_str().expect("utf-8 scripts path"),
            "--index-cache",
            cache_path.to_str().expect("utf-8 cache path"),
        ],
        &[
            ("REFORGER_MCP_TEST_GAME_DATA_OPERATION_DELAY_MS", "5000"),
            ("REFORGER_MCP_TEST_GAME_DATA_OPERATION_DEADLINE_MS", "50"),
        ],
    );
    client.initialize(1);
    client.send(status_call(2));
    assert_eq!(
        client.response(2).pointer("/result/structuredContent/available"),
        Some(&json!(true)),
        "status prepares the catalogue before the ready-operation deadline is exercised",
    );

    let call_started = std::time::Instant::now();
    client.send(json!({
        "jsonrpc":"2.0",
        "id":3,
        "method":"tools/call",
        "params":{"name":"search_game_data_symbols","arguments":{"query":"Deadline"}},
    }));
    let response = client.response(3);
    assert!(
        call_started.elapsed() < Duration::from_millis(500),
        "a ready Game Data operation must not inherit the cold initialization deadline",
    );
    assert!(response
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("deadline_exceeded:")));
}

#[test]
fn first_game_data_search_uses_the_cold_initialization_deadline() {
    let fixture = TempFixture::new("mcp_first_game_data_search_deadline");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(&scripts_root).expect("create scripts fixture");
    fs::write(scripts_root.join("Deadline.c"), "class Deadline {}")
        .expect("write game-data fixture");
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");
    let mut client = McpClient::spawn_with_env(
        &[
            "mcp",
            "--game-data-scripts",
            scripts_root.to_str().expect("utf-8 scripts path"),
            "--index-cache",
            cache_path.to_str().expect("utf-8 cache path"),
        ],
        &[
            ("REFORGER_MCP_TEST_INITIALIZATION_DELAY_MS", "5000"),
            ("REFORGER_MCP_TEST_INITIALIZATION_DEADLINE_MS", "50"),
            ("REFORGER_MCP_TEST_GAME_DATA_OPERATION_DEADLINE_MS", "50"),
        ],
    );
    client.initialize(1);
    client.send(json!({
        "jsonrpc":"2.0",
        "id":2,
        "method":"tools/call",
        "params":{"name":"search_game_data_symbols","arguments":{"query":"Deadline"}},
    }));
    let response = client.response(2);
    assert!(response
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with("deadline_exceeded: Game Data initialization")));
}

#[test]
fn request_admission_bounds_in_flight_tool_calls() {
    let fixture = TempFixture::new("mcp_admission");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(&scripts_root).expect("create scripts fixture");
    fs::write(scripts_root.join("Admission.c"), "class Admission {}")
        .expect("write game-data fixture");
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");
    let admission_marker = fixture.path().join("admitted-requests");
    let mut client = McpClient::spawn_with_env(
        &[
            "mcp",
            "--game-data-scripts",
            scripts_root.to_str().expect("utf-8 scripts path"),
            "--index-cache",
            cache_path.to_str().expect("utf-8 cache path"),
        ],
        &[
            ("REFORGER_MCP_TEST_INITIALIZATION_DELAY_MS", "750"),
            (
                "REFORGER_MCP_TEST_ADMISSION_MARKER",
                admission_marker.to_str().expect("utf-8 marker path"),
            ),
        ],
    );
    client.initialize(1);
    for id in 10..19 {
        client.send(status_call(id));
    }

    wait_for_lines(&admission_marker, 8, Duration::from_secs(2));
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        file_line_count(&admission_marker),
        8,
        "the ninth call must wait outside the eight-request admission bound"
    );

    let responses = client.take_responses(9);
    assert_eq!(
        responses
            .iter()
            .filter(|response| response.pointer("/result/isError") == Some(&json!(false)))
            .count(),
        9
    );
    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn panicking_initialization_worker_is_isolated_from_the_mcp_process() {
    let mut client = McpClient::spawn_with_env(&["mcp"], &[("REFORGER_MCP_TEST_PANIC_ONCE", "1")]);
    client.initialize(1);
    client.send(status_call(2));
    let failed = client.response(2);
    assert_eq!(failed.pointer("/error/code"), Some(&json!(-32603)));
    assert!(failed
        .pointer("/error/message")
        .and_then(Value::as_str)
        .is_some_and(|message| message.contains("initialization worker failed")));

    let recovered = client.call_status(3);
    assert_eq!(recovered.get("available"), Some(&json!(false)));
    client.send(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "ping",
        "params": {}
    }));
    assert_eq!(client.response(4).get("result"), Some(&json!({})));
    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

struct McpClient {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: Receiver<String>,
}

impl McpClient {
    fn spawn(args: &[&str]) -> Self {
        Self::spawn_with_env(args, &[])
    }

    fn spawn_program(program: &Path, args: &[&str]) -> Self {
        Self::spawn_program_with_env(program, args, &[])
    }

    fn spawn_with_env(args: &[&str], environment: &[(&str, &str)]) -> Self {
        Self::spawn_program_with_env(
            Path::new(env!("CARGO_BIN_EXE_reforger_language_server")),
            args,
            environment,
        )
    }

    fn spawn_program_with_env(program: &Path, args: &[&str], environment: &[(&str, &str)]) -> Self {
        let mut command = Command::new(program);
        command.args(args);
        for (name, value) in environment {
            command.env(name, value);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn MCP server");
        let stdin = child.stdin.take().expect("MCP stdin");
        let stdout = child.stdout.take().expect("MCP stdout");
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            child,
            stdin: Some(stdin),
            stdout: receiver,
        }
    }

    fn send(&mut self, message: Value) {
        let stdin = self.stdin.as_mut().expect("open MCP stdin");
        serde_json::to_writer(&mut *stdin, &message).expect("write MCP JSON");
        stdin.write_all(b"\n").expect("write MCP newline");
        stdin.flush().expect("flush MCP message");
    }

    fn initialize(&mut self, id: u64) -> Value {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "reforger-script-tools-test",
                    "version": "1.0.0"
                }
            }
        }));
        let response = self.response(id);
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
        response
    }

    fn call_status(&mut self, id: u64) -> Value {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": "game_data_status",
                "arguments": {}
            }
        }));
        self.response(id)
            .pointer("/result/structuredContent")
            .cloned()
            .expect("structured Game Data status")
    }

    fn response(&self, id: u64) -> Value {
        let line = self
            .stdout
            .recv_timeout(RESPONSE_TIMEOUT)
            .unwrap_or_else(|error| panic!("timed out waiting for MCP response {id}: {error}"));
        let value: Value = serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("stdout was not an MCP JSON message: {error}: {line}"));
        assert_eq!(value.get("id"), Some(&json!(id)));
        value
    }

    fn responses_until(&self, id: u64) -> Vec<Value> {
        let started = std::time::Instant::now();
        let mut messages = Vec::new();
        while started.elapsed() < RESPONSE_TIMEOUT {
            let remaining = RESPONSE_TIMEOUT.saturating_sub(started.elapsed());
            let line = self
                .stdout
                .recv_timeout(remaining)
                .unwrap_or_else(|error| panic!("timed out waiting for MCP response {id}: {error}"));
            let value: Value = serde_json::from_str(&line)
                .unwrap_or_else(|error| panic!("stdout was not MCP JSON: {error}: {line}"));
            let complete = value.get("id") == Some(&json!(id));
            messages.push(value);
            if complete {
                return messages;
            }
        }
        panic!("timed out waiting for MCP response {id}");
    }

    fn take_responses(&self, count: usize) -> Vec<Value> {
        (0..count)
            .map(|_| {
                let line = self
                    .stdout
                    .recv_timeout(RESPONSE_TIMEOUT)
                    .expect("timed out waiting for MCP response");
                serde_json::from_str(&line).expect("stdout was not MCP JSON")
            })
            .collect()
    }

    fn close_stdin(&mut self) {
        self.stdin.take();
    }

    fn wait_for_exit(&mut self, timeout: Duration) -> bool {
        let started = std::time::Instant::now();
        while started.elapsed() < timeout {
            if self.child.try_wait().expect("poll MCP process").is_some() {
                return true;
            }
            thread::sleep(Duration::from_millis(10));
        }
        false
    }
}

fn status_call(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": "game_data_status",
            "arguments": {}
        }
    })
}

fn wait_for_file(path: &Path, timeout: Duration) {
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        if path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {}", path.display());
}

fn wait_for_lines(path: &Path, count: usize, timeout: Duration) {
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        if file_line_count(path) >= count {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {count} lines in {}", path.display());
}

fn file_line_count(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|contents| contents.lines().count())
        .unwrap_or(0)
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

struct TempFixture {
    path: PathBuf,
}

impl TempFixture {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "reforger-script-tools-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temporary fixture");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
