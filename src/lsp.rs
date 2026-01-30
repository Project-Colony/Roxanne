use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct LspClient {
    process: Child,
    next_id: AtomicI64,
    pending: Arc<Mutex<HashMap<i64, Value>>>,
    writer: Arc<Mutex<std::process::ChildStdin>>,
    _reader_thread: Option<std::thread::JoinHandle<()>>,
    root_uri: String,
}

#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub severity: LspDiagnosticSeverity,
}

#[derive(Debug, Clone, Copy)]
pub enum LspDiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone)]
pub struct LspHoverResult {
    pub contents: String,
}

#[derive(Debug, Clone)]
pub struct LspLocation {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}

/// Configuration for which language server to use for a given language.
#[derive(Debug, Clone)]
pub struct LspServerConfig {
    pub command: String,
    pub args: Vec<String>,
}

impl LspServerConfig {
    pub fn for_language(language_id: &str) -> Option<Self> {
        match language_id {
            "rust" => Some(Self {
                command: "rust-analyzer".to_string(),
                args: Vec::new(),
            }),
            "python" => Some(Self {
                command: "pylsp".to_string(),
                args: Vec::new(),
            }),
            "javascript" | "typescript" => Some(Self {
                command: "typescript-language-server".to_string(),
                args: vec!["--stdio".to_string()],
            }),
            "go" => Some(Self {
                command: "gopls".to_string(),
                args: Vec::new(),
            }),
            "c" | "cpp" => Some(Self {
                command: "clangd".to_string(),
                args: Vec::new(),
            }),
            _ => None,
        }
    }
}

fn path_to_uri(path: &Path) -> String {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(path)
    };
    format!("file://{}", absolute.display())
}

fn uri_to_path(uri: &str) -> PathBuf {
    if let Some(path) = uri.strip_prefix("file://") {
        PathBuf::from(path)
    } else {
        PathBuf::from(uri)
    }
}

impl LspClient {
    pub fn start(config: &LspServerConfig, workspace_root: &Path) -> Result<Self, String> {
        let mut process = Command::new(&config.command)
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("LSP: impossible de lancer '{}': {e}", config.command))?;

        let stdin = process.stdin.take().ok_or("LSP: pas de stdin")?;
        let stdout = process.stdout.take().ok_or("LSP: pas de stdout")?;

        let writer = Arc::new(Mutex::new(stdin));
        let pending: Arc<Mutex<HashMap<i64, Value>>> = Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending.clone();

        let reader_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_message(&mut reader) {
                    Ok(Some(msg)) => {
                        if let Some(id) = msg.get("id").and_then(|v| v.as_i64()) {
                            if msg.get("method").is_none() {
                                if let Ok(mut map) = pending_clone.lock() {
                                    map.insert(id, msg);
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        });

        let root_uri = path_to_uri(workspace_root);

        let mut client = Self {
            process,
            next_id: AtomicI64::new(1),
            pending,
            writer,
            _reader_thread: Some(reader_thread),
            root_uri,
        };

        client.initialize()?;
        Ok(client)
    }

    fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    fn send_request(&self, method: &str, params: Value) -> Result<i64, String> {
        let id = self.next_id();
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send_message(&msg)?;
        Ok(id)
    }

    fn send_notification(&self, method: &str, params: Value) -> Result<(), String> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.send_message(&msg)
    }

    fn send_message(&self, msg: &Value) -> Result<(), String> {
        let body = serde_json::to_string(msg).map_err(|e| format!("LSP sérialisation: {e}"))?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut writer = self.writer.lock().map_err(|e| format!("LSP lock: {e}"))?;
        writer
            .write_all(header.as_bytes())
            .map_err(|e| format!("LSP écriture: {e}"))?;
        writer
            .write_all(body.as_bytes())
            .map_err(|e| format!("LSP écriture: {e}"))?;
        writer.flush().map_err(|e| format!("LSP flush: {e}"))?;
        Ok(())
    }

    fn wait_response(&self, id: i64, timeout_ms: u64) -> Result<Value, String> {
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        loop {
            if let Ok(mut map) = self.pending.lock() {
                if let Some(response) = map.remove(&id) {
                    return Ok(response);
                }
            }
            if std::time::Instant::now() > deadline {
                return Err("LSP: timeout".to_string());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": self.root_uri,
            "capabilities": {
                "textDocument": {
                    "hover": { "contentFormat": ["plaintext"] },
                    "definition": {},
                    "publishDiagnostics": {},
                    "completion": {
                        "completionItem": { "snippetSupport": false }
                    }
                }
            }
        });

        let id = self.send_request("initialize", params)?;
        let _ = self.wait_response(id, 10000)?;
        self.send_notification("initialized", serde_json::json!({}))?;
        Ok(())
    }

    pub fn did_open(&self, file_path: &Path, language_id: &str, text: &str) -> Result<(), String> {
        let uri = path_to_uri(file_path);
        self.send_notification(
            "textDocument/didOpen",
            serde_json::json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                }
            }),
        )
    }

    pub fn did_change(&self, file_path: &Path, version: i32, text: &str) -> Result<(), String> {
        let uri = path_to_uri(file_path);
        self.send_notification(
            "textDocument/didChange",
            serde_json::json!({
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": text }],
            }),
        )
    }

    pub fn did_save(&self, file_path: &Path) -> Result<(), String> {
        let uri = path_to_uri(file_path);
        self.send_notification(
            "textDocument/didSave",
            serde_json::json!({
                "textDocument": { "uri": uri },
            }),
        )
    }

    pub fn hover(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<LspHoverResult>, String> {
        let uri = path_to_uri(file_path);
        let id = self.send_request(
            "textDocument/hover",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        )?;
        let response = self.wait_response(id, 5000)?;
        let result = response.get("result");
        match result {
            Some(Value::Null) | None => Ok(None),
            Some(value) => {
                // Extract hover contents from the response
                let contents = extract_hover_contents(value);
                if contents.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(LspHoverResult { contents }))
                }
            }
        }
    }

    pub fn goto_definition(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<LspLocation>, String> {
        let uri = path_to_uri(file_path);
        let id = self.send_request(
            "textDocument/definition",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        )?;
        let response = self.wait_response(id, 5000)?;
        let result = response.get("result");
        match result {
            Some(Value::Null) | None => Ok(Vec::new()),
            Some(value) => Ok(extract_locations(value)),
        }
    }

    pub fn shutdown(&self) -> Result<(), String> {
        let id = self.send_request("shutdown", serde_json::json!(null))?;
        let _ = self.wait_response(id, 3000);
        self.send_notification("exit", serde_json::json!(null))
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
        let _ = self.process.kill();
    }
}

fn read_message(reader: &mut BufReader<std::process::ChildStdout>) -> Result<Option<Value>, String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let bytes_read = reader
            .read_line(&mut line)
            .map_err(|e| format!("LSP lecture: {e}"))?;
        if bytes_read == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length: ") {
            content_length = value.parse().ok();
        }
    }

    let length = content_length.ok_or("LSP: Content-Length manquant")?;
    let mut body = vec![0u8; length];
    std::io::Read::read_exact(reader, &mut body).map_err(|e| format!("LSP lecture body: {e}"))?;
    let value: Value =
        serde_json::from_slice(&body).map_err(|e| format!("LSP JSON invalide: {e}"))?;
    Ok(Some(value))
}

fn extract_hover_contents(value: &Value) -> String {
    // Hover response has "contents" which can be MarkedString, [MarkedString], or MarkupContent
    if let Some(contents) = value.get("contents") {
        if let Some(s) = contents.as_str() {
            return s.to_string();
        }
        if let Some(obj) = contents.as_object() {
            // MarkupContent or LanguageString
            if let Some(value) = obj.get("value").and_then(|v| v.as_str()) {
                return value.to_string();
            }
        }
        if let Some(arr) = contents.as_array() {
            let parts: Vec<String> = arr
                .iter()
                .filter_map(|item| {
                    if let Some(s) = item.as_str() {
                        Some(s.to_string())
                    } else if let Some(obj) = item.as_object() {
                        obj.get("value").and_then(|v| v.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
                .collect();
            return parts.join("\n");
        }
    }
    String::new()
}

fn extract_locations(value: &Value) -> Vec<LspLocation> {
    if let Some(_obj) = value.as_object() {
        if let Some(loc) = parse_location(value) {
            return vec![loc];
        }
    }
    if let Some(arr) = value.as_array() {
        return arr.iter().filter_map(parse_location).collect();
    }
    Vec::new()
}

fn parse_location(value: &Value) -> Option<LspLocation> {
    let uri = value.get("uri")?.as_str()?;
    let range = value.get("range")?;
    let start = range.get("start")?;
    let line = start.get("line")?.as_u64()? as usize;
    let column = start.get("character")?.as_u64()? as usize;
    Some(LspLocation {
        path: uri_to_path(uri),
        line,
        column,
    })
}

/// Map from file extension to LSP language ID.
pub fn language_id_from_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "rs" => Some("rust"),
        "py" | "pyi" => Some("python"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "ts" | "tsx" => Some("typescript"),
        "go" => Some("go"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
        "json" => Some("json"),
        "toml" => Some("toml"),
        _ => None,
    }
}
