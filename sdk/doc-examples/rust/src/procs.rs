//! `proc/*`.

use escouade_plugin_kit::serve::{Cx, Refusal};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, OnceLock};
use std::task::{Context, Poll, Waker};

/// Runs a line of `permissions.commands` in a project: what it writes comes as `proc/out`, its end as `proc/exit`.
pub async fn status(cx: &Cx, project: &str) -> Result<Value, Refusal> {
    let spawned = cx
        .request(
            "proc/spawn",
            json!({
                "command": "git",
                "args": ["status", "--short"],
                "dir": "project",
                "project": project,
            }),
        )
        .await?;
    Ok(spawned["proc"].clone())
}

/// What a program wrote, by its `proc`, until its end.
#[derive(Default)]
pub struct Said {
    pub stdout: String,
    pub stderr: String,
    /// Its exit code once it ended; `null` when it was killed.
    pub code: Option<Value>,
    waiting: Option<Waker>,
}

fn programs() -> &'static Mutex<HashMap<u64, Said>> {
    static PROGRAMS: OnceLock<Mutex<HashMap<u64, Said>>> = OnceLock::new();
    PROGRAMS.get_or_init(Mutex::default)
}

/// `proc/out { proc, stream, data }`: `data` is text (UTF-8, never base64), a piece of what it wrote, not a line.
pub fn out(cx: &Cx, out: &Value) {
    let data = out["data"].as_str().unwrap_or_default();
    cx.log("info", &format!("{}: {data}", out["stream"]));
    let proc = out["proc"].as_u64().unwrap_or_default();
    let mut all = programs().lock().unwrap();
    let said = all.entry(proc).or_default();
    match out["stream"].as_str() {
        Some("stderr") => said.stderr.push_str(data),
        _ => said.stdout.push_str(data),
    }
}

/// `proc/exit { proc, code }`: once, after all it wrote.
pub fn exit(cx: &Cx, exit: &Value) {
    cx.log("info", &format!("it ended: {}", exit["code"]));
    let proc = exit["proc"].as_u64().unwrap_or_default();
    let mut all = programs().lock().unwrap();
    let said = all.entry(proc).or_default();
    said.code = Some(exit["code"].clone());
    if let Some(waker) = said.waiting.take() {
        waker.wake();
    }
}

/// Runs `git --version`, a line of `permissions.commands`, and answers what it wrote and how it ended.
pub async fn version(cx: &Cx) -> Result<Value, Refusal> {
    let spawned = cx
        .request("proc/spawn", json!({ "command": "git", "args": ["--version"] }))
        .await?;
    let proc = spawned["proc"].as_u64().unwrap_or_default();
    // Its output may have come before this answer: it is kept by its `proc` all the same.
    let said = Ended(proc).await;
    Ok(json!({ "stdout": said.stdout.trim_end(), "stderr": said.stderr, "code": said.code }))
}

/// Until the program `proc` ends: what it said.
struct Ended(u64);

impl Future for Ended {
    type Output = Said;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Said> {
        let mut all = programs().lock().unwrap();
        if all.get(&self.0).is_some_and(|s| s.code.is_some()) {
            return Poll::Ready(all.remove(&self.0).unwrap_or_default());
        }
        all.entry(self.0).or_default().waiting = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// What it reads on its input; `close` ends that input.
pub async fn feed(cx: &Cx, proc: &Value) -> Result<Value, Refusal> {
    let input = json!({ "proc": proc, "data": "yes\n", "close": true });
    cx.request("proc/write", input).await
}

pub async fn stop(cx: &Cx, proc: &Value) -> Result<Value, Refusal> {
    cx.request("proc/kill", json!({ "proc": proc })).await
}
