//! What the core still asks a `codex app-server` itself, outside any conversation: the quota and the models of an
//! account none of whose agents runs. An app-server of its own (`env`: its `CODEX_HOME`), started for that and stopped
//! once it answered; it starts no thread, nothing is spent, and it costs little (0,6 s and 62 Mo on Codex 0.163.0,
//! tried). The conversation of an agent goes through its plugin (`dev.escouade.codex`).

use crate::engine::Program;
use crate::job::Job;
use anyhow::{anyhow, bail, Context, Result};
use escouade_plugin_codex::words;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// How long a request to Codex may take.
const REQUEST_LIMIT: Duration = Duration::from_secs(60);

/// How many pages of `model/list` are read at most (a cursor that never ends ends there).
const MODEL_PAGES: usize = 50;

/// An app-server started for a few requests.
struct Server {
    child: Child,
    _job: Option<Job>,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    next: u64,
}

impl Server {
    /// Starts it in `cwd` and says hello.
    async fn start(program: &Program, cwd: &str, env: &[(String, String)]) -> Result<Self> {
        let args = ["app-server".to_string()];
        let launch = program.launch(cwd, &args, env)?;
        let mut cmd = Command::new(&launch.program);
        cmd.args(&launch.args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        for (k, v) in &launch.env {
            cmd.env(k, v);
        }
        #[cfg(windows)]
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
        crate::job::isolate(&mut cmd);
        crate::proc::no_inherited_fds(&mut cmd);
        let mut child = cmd.spawn().context("codex app-server")?;
        let _job = Job::for_child(&child);
        let stdin = child.stdin.take().context("stdin")?;
        let stdout = child.stdout.take().context("stdout")?;
        let mut server = Self {
            child,
            _job,
            stdin,
            lines: BufReader::new(stdout).lines(),
            next: 0,
        };
        server
            .ask(
                "initialize",
                json!({
                    "clientInfo": { "name": "escouade", "title": "Escouade", "version": env!("CARGO_PKG_VERSION") },
                }),
            )
            .await?;
        server.write(&json!({ "method": "initialized" })).await?;
        Ok(server)
    }

    async fn write(&mut self, msg: &Value) -> Result<()> {
        self.stdin.write_all(format!("{msg}\n").as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// A request, and its result.
    async fn ask(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next += 1;
        let id = self.next;
        self.write(&json!({ "id": id, "method": method, "params": params }))
            .await?;
        let read = async {
            while let Some(line) = self.lines.next_line().await? {
                let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if msg["method"].is_null() && msg["id"] == id {
                    if let Some(e) = msg.get("error").filter(|e| !e.is_null()) {
                        bail!("{method}: {}", e["message"].as_str().unwrap_or("error"));
                    }
                    return Ok(msg["result"].clone());
                }
            }
            Err(anyhow!("{method}: process exited"))
        };
        tokio::time::timeout(REQUEST_LIMIT, read)
            .await
            .map_err(|_| anyhow!("{method}: no answer in time"))?
    }

    async fn stop(mut self) {
        let _ = self.child.start_kill();
        let _ = tokio::time::timeout(Duration::from_secs(5), self.child.wait()).await;
    }
}

/// The quota of an account none of whose agents runs (`account/rateLimits/read`).
pub(super) async fn read_limits(
    program: Program,
    env: Vec<(String, String)>,
    cwd: String,
) -> Result<Value> {
    let mut p = Server::start(&program, &cwd, &env).await?;
    let read = p.ask("account/rateLimits/read", json!({})).await;
    p.stop().await;
    Ok(read?["rateLimits"].clone())
}

/// The models of an account none of whose agents runs (`model/list`, every page, ESC-186: it gives them by pages,
/// `nextCursor`), as one answer (`data`), up to `MODEL_PAGES` of them.
pub(super) async fn read_models(
    program: Program,
    env: Vec<(String, String)>,
    cwd: String,
) -> Result<Value> {
    let mut p = Server::start(&program, &cwd, &env).await?;
    let read = list_models(&mut p).await;
    p.stop().await;
    read
}

async fn list_models(p: &mut Server) -> Result<Value> {
    let mut data = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MODEL_PAGES {
        let params = match &cursor {
            Some(c) => json!({ "cursor": c }),
            None => json!({}),
        };
        let page = p.ask("model/list", params).await?;
        data.extend(page["data"].as_array().cloned().unwrap_or_default());
        cursor = page["nextCursor"].as_str().map(str::to_string);
        if cursor.is_none() {
            return Ok(json!({ "data": data }));
        }
    }
    log::warn!("codex: model/list: more than {MODEL_PAGES} pages, the rest left out");
    Ok(json!({ "data": data }))
}

/// The meters of what `read_limits` read.
pub(super) fn meters_of(limits: &Value) -> Vec<crate::model::QuotaMeter> {
    words::meters(limits)
        .into_iter()
        .filter_map(|m| serde_json::from_value(m).ok())
        .collect()
}
