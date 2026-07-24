use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Owns the server's best-effort operational logs.  Logging deliberately
/// cannot influence request admission or response delivery.
#[derive(Clone)]
pub(super) struct LspLogger {
    path: Option<PathBuf>,
    lock: Arc<Mutex<()>>,
    diagnostic: Option<Arc<Mutex<BufWriter<File>>>>,
    diagnostic_session: Arc<str>,
}

impl LspLogger {
    pub(super) fn new(path: Option<PathBuf>, diagnostic_path: Option<PathBuf>) -> Self {
        if let Some(log_path) = path.as_ref() {
            if let Some(parent) = log_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
        }
        let diagnostic = diagnostic_path.and_then(|path| {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if path
                .metadata()
                .map(|metadata| metadata.len() > 1024 * 1024)
                .unwrap_or(false)
            {
                let _ = fs::write(&path, b"");
            }
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .ok()
                .map(|file| Arc::new(Mutex::new(BufWriter::new(file))))
        });
        Self {
            path,
            lock: Arc::new(Mutex::new(())),
            diagnostic,
            diagnostic_session: Arc::from(format!("{}-{}", timestamp_millis(), std::process::id())),
        }
    }

    pub(super) fn log(&self, message: &str) {
        let Some(log_path) = self.path.as_ref() else {
            return;
        };
        let Ok(_guard) = self.lock.lock() else {
            return;
        };
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(file, "[{}] {message}", timestamp_millis());
        }
    }

    pub(super) fn log_lazy(&self, message: impl FnOnce() -> String) {
        if self.path.is_some() {
            self.log(&message());
        }
    }

    pub(super) fn operational_enabled(&self) -> bool {
        self.path.is_some()
    }

    pub(super) fn diagnostic_enabled(&self) -> bool {
        self.diagnostic.is_some()
    }

    pub(super) fn diagnostic(&self, event: &str, fields: Value) {
        let Some(writer) = &self.diagnostic else {
            return;
        };
        let Ok(mut writer) = writer.lock() else {
            return;
        };
        let record = json!({
            "timestamp": timestamp_millis(), "component": "languageServer",
            "session": self.diagnostic_session.as_ref(), "event": event, "fields": fields,
        });
        if serde_json::to_writer(&mut *writer, &record).is_ok() {
            let _ = writer.write_all(b"\n");
        }
    }

    pub(super) fn diagnostic_lazy(&self, event: &str, fields: impl FnOnce() -> Value) {
        if self.diagnostic.is_some() {
            self.diagnostic(event, fields());
        }
    }

    pub(super) fn flush_diagnostics(&self) {
        if let Some(writer) = &self.diagnostic {
            if let Ok(mut writer) = writer.lock() {
                let _ = writer.flush();
            }
        }
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
