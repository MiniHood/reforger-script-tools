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
    assert_eq!(listed.len(), 2);
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

    client.send(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": { "name": "search_game_data_symbols", "arguments": { "query": "McpFixture" } }
    }));
    let search = client.response(4);
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

    client.close_stdin();
    assert!(
        client.wait_for_exit(Duration::from_secs(3)),
        "MCP process should exit promptly after stdin EOF"
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
    assert_eq!(client.response(5).pointer("/error/code"), Some(&json!(-32602)));

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
            "name": "game_data_status",
            "arguments": {}
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

    fn spawn_with_env(args: &[&str], environment: &[(&str, &str)]) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_reforger_language_server"));
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
