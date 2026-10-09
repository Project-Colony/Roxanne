//! An experimental client for rust-analyzer: hover and go to definition.
//!
//! One [`LspSession`] serves one workspace root. Its server runs on a
//! background thread, so a request never blocks the caller: the answer comes
//! back through a future. The server is asked not to run build scripts, proc
//! macros or `cargo check`, which all run code from the project. A project's
//! own `rust-analyzer.toml` can still override some of these settings, and
//! its `rust-toolchain.toml` picks the toolchain that runs `cargo metadata`.

use iced::futures::channel::oneshot;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::future::Future;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const SERVER: &str = "rust-analyzer";
const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the first request waits for the server to load the project, so
/// it gets an answer instead of an empty result from a half-loaded server.
const LOAD_TIMEOUT: Duration = Duration::from_secs(30);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

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

/// A position in a file. Lines and columns count from 0, and columns count
/// characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspLocation {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}

/// The directory rust-analyzer should load for `file`: the nearest ancestor
/// holding a `Cargo.lock` (a workspace root), else one holding a
/// `Cargo.toml`, else the file's own directory. Files of one Cargo workspace
/// therefore share a session.
pub fn workspace_root(file: &Path) -> PathBuf {
    let dir = file.parent().unwrap_or(file);
    ["Cargo.lock", "Cargo.toml"]
        .iter()
        .find_map(|marker| dir.ancestors().find(|d| d.join(marker).is_file()))
        .unwrap_or(dir)
        .to_path_buf()
}

/// A rust-analyzer server for one workspace root, owned by a background
/// thread. The server starts with the first request. Dropping the session
/// shuts the server down on that thread; if the editor exits first, the
/// server sees its input close and exits on its own.
#[derive(Debug)]
pub struct LspSession {
    root: PathBuf,
    jobs: mpsc::Sender<Job>,
    worker: JoinHandle<()>,
}

struct Job {
    method: &'static str,
    /// Absolute path of the file asked about.
    path: PathBuf,
    /// The file's text as the editor shows it, which may not be saved yet.
    text: String,
    line: u32,
    /// Counted in characters.
    character: u32,
    reply: oneshot::Sender<Result<Value, String>>,
}

impl LspSession {
    pub fn start(root: PathBuf) -> Self {
        Self::with_command(SERVER, root)
    }

    fn with_command(command: &str, root: PathBuf) -> Self {
        let (jobs, receiver) = mpsc::channel();
        let (command, worker_root) = (command.to_string(), root.clone());
        let worker = thread::spawn(move || serve(&command, &worker_root, receiver));
        Self { root, jobs, worker }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// False once the server could not start or has exited. A new session
    /// is needed then.
    pub fn is_running(&self) -> bool {
        !self.worker.is_finished()
    }

    /// The hover text at a position of `path`, whose current text is `text`.
    pub fn hover(
        &self,
        path: PathBuf,
        text: String,
        line: u32,
        character: u32,
    ) -> impl Future<Output = Result<Option<String>, String>> + Send + use<> {
        let answer = self.request("textDocument/hover", path, text, line, character);
        async move {
            let contents = extract_hover_contents(&answer.await?);
            Ok((!contents.is_empty()).then_some(contents))
        }
    }

    /// Where the symbol at a position of `path` is defined.
    pub fn definition(
        &self,
        path: PathBuf,
        text: String,
        line: u32,
        character: u32,
    ) -> impl Future<Output = Result<Option<LspLocation>, String>> + Send + use<> {
        let answer = self.request("textDocument/definition", path, text, line, character);
        async move { Ok(extract_locations(&answer.await?).into_iter().next()) }
    }

    fn request(
        &self,
        method: &'static str,
        path: PathBuf,
        text: String,
        line: u32,
        character: u32,
    ) -> impl Future<Output = Result<Value, String>> + Send + use<> {
        let (reply, answer) = oneshot::channel();
        // Sending fails only when the worker has stopped. The job and its
        // `reply` are dropped then, and the answer reports it.
        let _ = self.jobs.send(Job {
            method,
            path,
            text,
            line,
            character,
            reply,
        });
        async move {
            answer
                .await
                .unwrap_or_else(|_| Err(format!("{SERVER} stopped")))
        }
    }
}

/// The worker: starts the server on the first job, answers jobs one at a
/// time, and shuts the server down once the session is dropped.
fn serve(command: &str, root: &Path, jobs: mpsc::Receiver<Job>) {
    let Ok(first) = jobs.recv() else {
        return;
    };
    let mut server = match Server::spawn(command, root) {
        Ok(server) => server,
        Err(err) => {
            let _ = first.reply.send(Err(err));
            return;
        }
    };
    for job in std::iter::once(first).chain(jobs.iter()) {
        let result = server.query(job.method, &job.path, &job.text, job.line, job.character);
        let _ = job.reply.send(result);
        if server.closed {
            return;
        }
    }
    server.shutdown();
}

struct Server {
    /// `None` when the server is not a child process, as in tests.
    child: Option<Child>,
    writer: Box<dyn Write + Send>,
    incoming: mpsc::Receiver<Value>,
    next_id: i64,
    /// URI, version and text of the one document open in the server.
    document: Option<(String, i64, String)>,
    loaded: bool,
    closed: bool,
}

impl Server {
    fn spawn(command: &str, root: &Path) -> Result<Self, String> {
        let mut child = Command::new(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => format!("{command} was not found in PATH."),
                _ => format!("could not start {command}: {err}"),
            })?;
        let (Some(stdout), Some(stdin)) = (child.stdout.take(), child.stdin.take()) else {
            unreachable!("both pipes are requested above");
        };
        let mut server = Self::connect(stdout, stdin, Some(child));
        server.initialize(root)?;
        Ok(server)
    }

    fn connect(
        reader: impl Read + Send + 'static,
        writer: impl Write + Send + 'static,
        child: Option<Child>,
    ) -> Self {
        let (sender, incoming) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(reader);
            while let Some(message) = read_message(&mut reader) {
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            writer: Box::new(writer),
            incoming,
            next_id: 0,
            document: None,
            loaded: false,
            closed: false,
        }
    }

    fn initialize(&mut self, root: &Path) -> Result<(), String> {
        self.request("initialize", initialize_params(root))
            .and_then(|id| self.response(id, INITIALIZE_TIMEOUT))
            .map_err(|err| {
                format!("{err} while starting. Check that `{SERVER} --version` works.")
            })?;
        self.notify("initialized", json!({}))
    }

    fn query(
        &mut self,
        method: &str,
        path: &Path,
        text: &str,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        let uri = path_to_uri(path);
        self.sync(&uri, text)?;
        self.wait_until_loaded();
        let id = self.request(
            method,
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
        )?;
        self.response(id, REQUEST_TIMEOUT)
    }

    /// Tells the server the text of `uri`: the whole text the first time,
    /// then the new text whenever it changed since the last request. Only
    /// the document asked about stays open. The one asked about before is
    /// closed, so the server reads it from disk again instead of keeping the
    /// text it had at that request.
    fn sync(&mut self, uri: &str, text: &str) -> Result<(), String> {
        if let Some((open, version, sent)) = &mut self.document
            && open == uri
        {
            if sent == text {
                return Ok(());
            }
            *version += 1;
            text.clone_into(sent);
            let params = json!({
                "textDocument": { "uri": uri, "version": *version },
                "contentChanges": [{ "text": text }],
            });
            return self.notify("textDocument/didChange", params);
        }
        if let Some((open, ..)) = self.document.take() {
            self.notify(
                "textDocument/didClose",
                json!({ "textDocument": { "uri": open } }),
            )?;
        }
        self.document = Some((uri.to_string(), 1, text.to_string()));
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "rust",
                    "version": 1,
                    "text": text,
                }
            }),
        )
    }

    /// Waits once per server, at most `LOAD_TIMEOUT`, until it reports that
    /// it has finished loading the project.
    fn wait_until_loaded(&mut self) {
        let deadline = Instant::now() + LOAD_TIMEOUT;
        while !self.loaded {
            if !matches!(self.next_message(deadline), Ok(Some(_))) {
                break;
            }
        }
        self.loaded = true;
    }

    fn response(&mut self, id: i64, timeout: Duration) -> Result<Value, String> {
        let deadline = Instant::now() + timeout;
        loop {
            let Some(message) = self.next_message(deadline)? else {
                return Err(format!(
                    "{SERVER} did not answer within {} s",
                    timeout.as_secs()
                ));
            };
            // Notifications, requests from the server and late answers to
            // requests that timed out.
            if message.get("method").is_some() || message["id"].as_i64() != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                let reason = error["message"].as_str().unwrap_or("request failed");
                return Err(format!("{SERVER}: {reason}"));
            }
            return Ok(message["result"].clone());
        }
    }

    /// The next message from the server, or `None` once `deadline` passes.
    fn next_message(&mut self, deadline: Instant) -> Result<Option<Value>, String> {
        let message = match self
            .incoming
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(message) => message,
            Err(RecvTimeoutError::Timeout) => return Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                self.closed = true;
                return Err(format!("{SERVER} exited"));
            }
        };
        if let Some(method) = message.get("method").and_then(Value::as_str) {
            if method == "experimental/serverStatus" && message["params"]["quiescent"] == true {
                self.loaded = true;
            }
            // A request from the server: answer it so it never waits on us.
            if let Some(id) = message.get("id") {
                self.send(&json!({ "jsonrpc": "2.0", "id": id, "result": null }))?;
            }
        }
        Ok(Some(message))
    }

    fn shutdown(&mut self) {
        if let Ok(id) = self.request("shutdown", Value::Null) {
            let _ = self.response(id, SHUTDOWN_TIMEOUT);
        }
        let _ = self.notify("exit", Value::Null);
    }

    fn request(&mut self, method: &str, params: Value) -> Result<i64, String> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        Ok(id)
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    fn send(&mut self, message: &Value) -> Result<(), String> {
        let body = message.to_string();
        let written = write!(self.writer, "Content-Length: {}\r\n\r\n{body}", body.len())
            .and_then(|()| self.writer.flush());
        written.map_err(|err| {
            self.closed = true;
            format!("could not write to {SERVER}: {err}")
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn initialize_params(root: &Path) -> Value {
    json!({
        "processId": std::process::id(),
        "rootUri": path_to_uri(root),
        "capabilities": {
            // Columns count characters on both sides.
            "general": { "positionEncodings": ["utf-32"] },
            "textDocument": {
                "hover": { "contentFormat": ["plaintext"] },
                "definition": {},
            },
            "experimental": { "serverStatusNotification": true },
        },
        // Build scripts, proc macros and `cargo check` all run code from the
        // project, so asking about a file must not start any of them.
        "initializationOptions": {
            "cargo": { "buildScripts": { "enable": false } },
            "procMacro": { "enable": false },
            "checkOnSave": false,
        },
    })
}

/// A `file://` URI for an absolute path.
fn path_to_uri(path: &Path) -> String {
    let path = path.to_string_lossy();
    let path = if cfg!(windows) {
        windows_uri_path(&path)
    } else {
        format!("//{path}")
    };
    let mut uri = String::from("file:");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/:".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    uri
}

/// What follows `file:` in the URI of a Windows path, before escaping.
/// `C:\dir` and `\\?\C:\dir`, the form `canonicalize` returns, give
/// `///C:/dir`. `\\server\share` and `\\?\UNC\server\share` give
/// `//server/share`, a URI whose host is `server`.
fn windows_uri_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    if let Some(unc) = path.strip_prefix("//?/UNC/") {
        format!("//{unc}")
    } else if let Some(local) = path.strip_prefix("//?/") {
        format!("///{local}")
    } else if path.starts_with("//") {
        path
    } else {
        format!("///{path}")
    }
}

/// The path a `file://` URI names, or `None` for any other URI.
fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?;
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut rest = encoded.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        let escaped = tail
            .get(..2)
            .filter(|hex| byte == b'%' && hex.iter().all(u8::is_ascii_hexdigit))
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        match escaped {
            Some(decoded) => {
                bytes.push(decoded);
                rest = &tail[2..];
            }
            None => {
                bytes.push(byte);
                rest = tail;
            }
        }
    }
    let path = String::from_utf8(bytes).ok()?;
    if cfg!(windows) {
        return Some(PathBuf::from(windows_path(&path)));
    }
    Some(PathBuf::from(path))
}

/// The Windows path named by what follows `file://` in a URI, once
/// unescaped: `/C:/dir` names `C:/dir`, and `server/share`, from a URI whose
/// host is `server`, names `//server/share`.
fn windows_path(uri_path: &str) -> String {
    match uri_path.strip_prefix('/') {
        Some(local) if local.as_bytes().get(1) == Some(&b':') => local.to_string(),
        Some(_) => uri_path.to_string(),
        None => format!("//{uri_path}"),
    }
}

/// One message from the server, or `None` once its output ends or breaks.
fn read_message(reader: &mut impl BufRead) -> Option<Value> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            content_length = value.parse().ok();
        }
    }
    let mut body = vec![0u8; content_length?];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
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
    if let Some(arr) = value.as_array() {
        return arr.iter().filter_map(parse_location).collect();
    }
    parse_location(value).into_iter().collect()
}

fn parse_location(value: &Value) -> Option<LspLocation> {
    let start = &value["range"]["start"];
    Some(LspLocation {
        path: uri_to_path(value["uri"].as_str()?)?,
        line: start["line"].as_u64()? as usize,
        column: start["character"].as_u64()? as usize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::futures::executor::block_on;

    #[test]
    fn initialize_turns_off_build_scripts_and_proc_macros() {
        let params = initialize_params(Path::new("/work/project"));
        let options = &params["initializationOptions"];
        assert_eq!(options["cargo"]["buildScripts"]["enable"], false);
        assert_eq!(options["procMacro"]["enable"], false);
        assert_eq!(options["checkOnSave"], false);
    }

    #[test]
    fn missing_server_fails_quickly() {
        let started = Instant::now();
        let session = LspSession::with_command("roxanne-missing-language-server", PathBuf::new());
        let answer = block_on(session.hover(PathBuf::from("main.rs"), String::new(), 0, 0));
        let error = answer.expect_err("a missing server cannot answer");
        assert!(error.contains("not found in PATH"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn workspace_root_prefers_the_lockfile_directory() {
        let dir = tempfile::tempdir().unwrap();
        let member = dir.path().join("crates").join("app");
        let file = member.join("src").join("main.rs");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(member.join("Cargo.toml"), "").unwrap();
        std::fs::write(dir.path().join("Cargo.lock"), "").unwrap();
        assert_eq!(workspace_root(&file), dir.path());
        std::fs::remove_file(dir.path().join("Cargo.lock")).unwrap();
        assert_eq!(workspace_root(&file), member);
    }

    #[test]
    fn uris_round_trip_paths_with_reserved_characters() {
        let path = std::env::temp_dir().join("a dir#1").join("é%.rs");
        let uri = path_to_uri(&path);
        assert!(!uri.contains(' ') && !uri.contains('#'), "{uri}");
        assert_eq!(uri_to_path(&uri), Some(path));
        assert_eq!(uri_to_path("untitled:1"), None);
    }

    /// Runs on every platform, so the Linux-only CI covers the Windows forms.
    #[test]
    fn windows_paths_map_to_uris_rust_analyzer_accepts() {
        for (path, uri_path) in [
            (r"C:\a\b.rs", "///C:/a/b.rs"),
            (r"\\?\C:\a\b.rs", "///C:/a/b.rs"),
            (r"\\server\share\b.rs", "//server/share/b.rs"),
            (r"\\?\UNC\server\share\b.rs", "//server/share/b.rs"),
        ] {
            assert_eq!(windows_uri_path(path), uri_path, "{path}");
        }
        assert_eq!(windows_path("/C:/a/b.rs"), "C:/a/b.rs");
        assert_eq!(windows_path("server/share/b.rs"), "//server/share/b.rs");
    }

    #[cfg(windows)]
    #[test]
    fn verbatim_windows_paths_give_drive_letter_uris() {
        assert_eq!(
            path_to_uri(Path::new(r"\\?\C:\a\b.rs")),
            "file:///C:/a/b.rs"
        );
        assert_eq!(
            uri_to_path("file:///C:/a/b.rs"),
            Some(PathBuf::from(r"C:\a\b.rs"))
        );
    }

    /// Plays a server that asks the client something and reports progress,
    /// then checks what the client sends and how it reads the answers.
    #[test]
    fn query_syncs_the_document_and_reads_the_answer() {
        let (client_reads, server_writes) = std::io::pipe().unwrap();
        let (server_reads, client_writes) = std::io::pipe().unwrap();
        let fake = thread::spawn(move || {
            let mut input = BufReader::new(server_reads);
            let mut output = server_writes;
            let mut next = || read_message(&mut input).expect("a client message");
            let mut reply = |message: Value| {
                let body = message.to_string();
                write!(output, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
            };

            let initialize = next();
            assert_eq!(initialize["method"], "initialize");
            reply(json!({ "jsonrpc": "2.0", "id": initialize["id"], "result": {} }));
            assert_eq!(next()["method"], "initialized");

            let open = next();
            assert_eq!(open["method"], "textDocument/didOpen");
            assert_eq!(open["params"]["textDocument"]["text"], "fn main() {}\n");
            reply(json!({ "jsonrpc": "2.0", "id": "q", "method": "workspace/configuration" }));
            reply(json!({
                "jsonrpc": "2.0",
                "method": "experimental/serverStatus",
                "params": { "health": "ok", "quiescent": true },
            }));
            assert_eq!(next()["id"], "q");

            let hover = next();
            assert_eq!(hover["method"], "textDocument/hover");
            assert_eq!(hover["params"]["position"]["character"], 3);
            reply(json!({
                "jsonrpc": "2.0",
                "id": hover["id"],
                "result": { "contents": { "kind": "plaintext", "value": "fn main()" } },
            }));

            let change = next();
            assert_eq!(change["method"], "textDocument/didChange");
            assert_eq!(change["params"]["textDocument"]["version"], 2);
            assert_eq!(
                change["params"]["contentChanges"][0]["text"],
                "fn main() { }\n"
            );
            let definition = next();
            assert_eq!(definition["method"], "textDocument/definition");
            reply(json!({
                "jsonrpc": "2.0",
                "id": definition["id"],
                "result": [{
                    "uri": "file:///work/a%20b/lib.rs",
                    "range": {
                        "start": { "line": 4, "character": 2 },
                        "end": { "line": 4, "character": 6 },
                    },
                }],
            }));

            // Asking about another file closes the first one.
            let close = next();
            assert_eq!(close["method"], "textDocument/didClose");
            assert_eq!(
                close["params"]["textDocument"]["uri"],
                "file:///work/project/src/main.rs"
            );
            let open = next();
            assert_eq!(open["method"], "textDocument/didOpen");
            assert_eq!(
                open["params"]["textDocument"]["uri"],
                "file:///work/project/src/lib.rs"
            );
            let hover = next();
            assert_eq!(hover["method"], "textDocument/hover");
            reply(json!({ "jsonrpc": "2.0", "id": hover["id"], "result": null }));
        });

        let mut server = Server::connect(client_reads, client_writes, None);
        server.initialize(Path::new("/work/project")).unwrap();
        let file = Path::new("/work/project/src/main.rs");
        let hover = server
            .query("textDocument/hover", file, "fn main() {}\n", 0, 3)
            .unwrap();
        assert_eq!(extract_hover_contents(&hover), "fn main()");
        let definition = server
            .query("textDocument/definition", file, "fn main() { }\n", 0, 3)
            .unwrap();
        assert_eq!(
            extract_locations(&definition),
            vec![LspLocation {
                path: PathBuf::from("/work/a b/lib.rs"),
                line: 4,
                column: 2,
            }]
        );
        let hover = server
            .query(
                "textDocument/hover",
                Path::new("/work/project/src/lib.rs"),
                "",
                0,
                0,
            )
            .unwrap();
        assert_eq!(hover, Value::Null);
        fake.join().unwrap();
    }
}
