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
    assert_eq!(listed.len(), 1);
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
        "method": "ping",
        "params": {}
    }));
    assert_eq!(client.response(5).get("result"), Some(&json!({})));

    client.close_stdin();
    assert!(client.wait_for_exit(Duration::from_secs(3)));
}

#[test]
fn cancellation_and_eof_with_in_flight_initialization_shutdown_cleanly() {
    let fixture = TempFixture::new("mcp_cancel");
    let scripts_root = fixture.path().join("scripts");
    fs::create_dir_all(&scripts_root).expect("create scripts fixture");
    for index in 0..512 {
        fs::write(
            scripts_root.join(format!("CancellationFixture{index}.c")),
            format!("class CancellationFixture{index} {{ int m_Value; }}"),
        )
        .expect("write cancellation fixture");
    }
    let cache_path = fixture.path().join("cache").join("game-data-index.bin");
    let mut client = McpClient::spawn(&[
        "mcp",
        "--game-data-scripts",
        scripts_root.to_str().expect("utf-8 scripts path"),
        "--index-cache",
        cache_path.to_str().expect("utf-8 cache path"),
    ]);
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
}

struct McpClient {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: Receiver<String>,
}

impl McpClient {
    fn spawn(args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_reforger_language_server"))
            .args(args)
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
