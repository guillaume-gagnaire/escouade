//! ESC-444, a study, never merged: what the VS Code view (`@codingame/monaco-vscode-api`) needs from the core.
//!
//! - A language server started by the core in the project's folder, its messages carried both ways by the window's IPC
//!   (`lsp_start`, `lsp_send`, `lsp_stop`, the event `vscode-study-lsp`): the page never starts a program itself.
//! - The Node extension host of VS Code (its server, `server-main.js`), started with the system's Node, on the loopback
//!   with a connection token (`vscode_host_start`).
//! - A log of the study's measures (`study_env`, `study_log`, `study_quit`), written where `ESCOUADE_VSCODE_STUDY`
//!   says, for the bench (`scripts/study/editor-bench.mjs`) to read on the three systems.

use anyhow::{anyhow, bail, Context, Result};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// The language servers the study may start: a name the page gives, never a program.
pub fn server_program(name: &str) -> Result<(PathBuf, Vec<String>)> {
    let (program, args, env): (&str, &[&str], &str) = match name {
        "rust" => ("rust-analyzer", &[], "ESCOUADE_STUDY_RUST_LSP"),
        "typescript" => (
            "typescript-language-server",
            &["--stdio"],
            "ESCOUADE_STUDY_TS_LSP",
        ),
        _ => bail!("unknown language server: {name}"),
    };
    let path = std::env::var_os(env)
        .map(PathBuf::from)
        .or_else(|| crate::which::find(program))
        .ok_or_else(|| anyhow!("{program} not found"))?;
    Ok((path, args.iter().map(|s| s.to_string()).collect()))
}

/// Reads one message of the base protocol of LSP (`Content-Length: n`, a blank line, n bytes); None at the end.
pub async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(r: &mut R) -> Result<Option<String>> {
    let mut len: Option<usize> = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).await? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if len.is_some() {
                break;
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                len = Some(v.trim().parse().context("Content-Length")?);
            }
        }
    }
    let mut body = vec![0u8; len.unwrap_or(0)];
    r.read_exact(&mut body).await?;
    Ok(Some(String::from_utf8(body)?))
}

/// One message framed for the server's stdin.
pub fn frame(msg: &str) -> Vec<u8> {
    let mut out = format!("Content-Length: {}\r\n\r\n", msg.len()).into_bytes();
    out.extend_from_slice(msg.as_bytes());
    out
}

#[derive(Clone, Serialize)]
struct LspEvent {
    id: u32,
    /// A message of the server (JSON text), or null when it ended.
    msg: Option<String>,
}

#[derive(Default)]
struct Study {
    next: AtomicU32,
    servers: Mutex<HashMap<u32, mpsc::UnboundedSender<Option<String>>>>,
    host: Mutex<Option<tokio::process::Child>>,
}

fn study() -> &'static Study {
    static S: OnceLock<Study> = OnceLock::new();
    S.get_or_init(Study::default)
}

/// Starts the language server `name` in `root`; its messages come as `vscode-study-lsp` events.
pub fn lsp_start<R: Runtime>(app: AppHandle<R>, name: &str, root: &Path) -> Result<u32> {
    let (program, args) = server_program(name)?;
    let mut cmd = Command::new(&program);
    cmd.args(&args)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    let mut child = cmd
        .spawn()
        .with_context(|| format!("starting {}", program.display()))?;
    let id = study().next.fetch_add(1, Ordering::Relaxed) + 1;
    let mut stdin = child.stdin.take().context("stdin")?;
    let stdout = child.stdout.take().context("stdout")?;
    let (tx, mut rx) = mpsc::unbounded_channel::<Option<String>>();
    study().servers.lock().insert(id, tx);
    tauri::async_runtime::spawn(async move {
        while let Some(Some(msg)) = rx.recv().await {
            if stdin.write_all(&frame(&msg)).await.is_err() {
                break;
            }
            let _ = stdin.flush().await;
        }
        let _ = child.kill().await;
    });
    tauri::async_runtime::spawn(async move {
        let mut out = BufReader::new(stdout);
        while let Ok(Some(msg)) = read_frame(&mut out).await {
            let _ = app.emit("vscode-study-lsp", LspEvent { id, msg: Some(msg) });
        }
        study().servers.lock().remove(&id);
        let _ = app.emit("vscode-study-lsp", LspEvent { id, msg: None });
    });
    Ok(id)
}

pub fn lsp_send(id: u32, msg: String) -> Result<()> {
    let servers = study().servers.lock();
    let tx = servers
        .get(&id)
        .ok_or_else(|| anyhow!("no language server {id}"))?;
    tx.send(Some(msg))
        .map_err(|_| anyhow!("language server {id} ended"))
}

pub fn lsp_stop(id: u32) {
    if let Some(tx) = study().servers.lock().remove(&id) {
        let _ = tx.send(None);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    /// `127.0.0.1:<port>`: the remote authority the view connects to.
    pub authority: String,
    pub token: String,
    /// `node --version` of the Node that runs it.
    pub node: String,
}

/// A connection token: 32 hex characters from the system's randomness.
fn token() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("randomness");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Starts the Node extension host of VS Code (the server of `ESCOUADE_STUDY_VSCODE_SERVER`, its `out/server-main.js`)
/// with `node` (the system's by default, `ESCOUADE_STUDY_NODE` otherwise), on the loopback, with a token. Its log goes
/// to `log` when given.
pub async fn host_start(log: Option<PathBuf>) -> Result<HostInfo> {
    let server = std::env::var_os("ESCOUADE_STUDY_VSCODE_SERVER")
        .map(PathBuf::from)
        .context("ESCOUADE_STUDY_VSCODE_SERVER is not set")?;
    let node = std::env::var_os("ESCOUADE_STUDY_NODE")
        .map(PathBuf::from)
        .or_else(|| crate::which::find("node"))
        .context("node not found")?;
    let version = std::process::Command::new(&node)
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if let Some(mut old) = study().host.lock().take() {
        let _ = old.start_kill();
    }
    // A free port of the loopback, taken then given back for the server to listen on.
    let port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    let token = token();
    let mut cmd = Command::new(&node);
    cmd.arg(server.join("out").join("server-main.js"))
        .args(["--host", "127.0.0.1", "--port", &port.to_string()])
        .arg(format!("--connection-token={token}"))
        .args(["--accept-server-license-terms", "--disable-telemetry"])
        .stdin(Stdio::null())
        .kill_on_drop(true);
    match &log {
        Some(f) => {
            let file = std::fs::File::create(f)?;
            cmd.stdout(file.try_clone()?).stderr(file);
        }
        None => {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
    }
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    let child = cmd.spawn().context("starting the VS Code server")?;
    *study().host.lock() = Some(child);
    // Ready when it accepts a connection.
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return Ok(HostInfo {
                authority: format!("127.0.0.1:{port}"),
                token,
                node: version,
            });
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    bail!("the VS Code server did not listen within 10 s")
}

/// The file of the study's measures, when the app was started for them.
pub fn study_file() -> Option<PathBuf> {
    std::env::var_os("ESCOUADE_VSCODE_STUDY").map(PathBuf::from)
}

/// Appends a line to the study's file: milliseconds since the epoch, a tab, the line.
pub fn study_log(line: &str) -> Result<()> {
    use std::io::Write;
    let Some(file) = study_file() else {
        return Ok(());
    };
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)?;
    writeln!(f, "{ms}\t{}", line.replace('\n', " "))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_framed_messages_one_after_the_other() {
        let mut bytes = frame(r#"{"a":1}"#);
        bytes.extend(b"Content-Type: x\r\nContent-Length: 2\r\n\r\n{}");
        let mut r = BufReader::new(&bytes[..]);
        assert_eq!(
            read_frame(&mut r).await.unwrap().as_deref(),
            Some(r#"{"a":1}"#)
        );
        assert_eq!(read_frame(&mut r).await.unwrap().as_deref(), Some("{}"));
        assert_eq!(read_frame(&mut r).await.unwrap(), None);
    }

    #[test]
    fn frames_by_bytes_not_characters() {
        assert_eq!(frame("é"), b"Content-Length: 2\r\n\r\n\xc3\xa9".to_vec());
    }

    #[test]
    fn only_named_servers() {
        assert!(server_program("sh").is_err());
    }

    #[test]
    fn tokens_are_32_hex() {
        let t = token();
        assert_eq!(t.len(), 32);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(t, token());
    }
}
