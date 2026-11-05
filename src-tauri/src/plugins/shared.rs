//! A harness's process shared by agents (`Launch::shared`, INT-10.2): OpenCode's `opencode serve`, one for the agents of
//! an account. The core keeps what it keeps of every harness: it starts the process from the plugin's description, in its
//! Job object, under WSL when need be, reads the line that says it is ready, and stops it with its last agent; the process
//! outlives a crash of the plugin. What is said to it goes by the plugin's own way (an HTTP client): the core reads
//! nothing but that first line.
//!
//! It runs in a folder of its own, none of the agents' (a worktree removed would find it held). Which agents share it:
//! those of the same plugin, program and variables, with one agent a folder when the plugin asks it (its server's
//! entries are given folder by folder).

use super::harness::command_of;
use crate::engine::Program;
use crate::job::Job;
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{oneshot, watch};

/// How long a process may take to say it is ready (2,1 to 2,6 s for OpenCode 1.19.3, tried).
const START_LIMIT: Duration = Duration::from_secs(60);

/// How it ended: its exit code, and the end of what it wrote.
pub type End = (Option<i32>, String);

pub struct Shared {
    key: String,
    /// The port the core read in its output (`SharedLaunch::listens`), 0 before and without: what `local:shared` reaches.
    port: Arc<AtomicU16>,
    /// The line that said it is ready, or why it never will.
    line: watch::Receiver<Option<Result<String, String>>>,
    /// What the plugin wants back with each agent (`SharedLaunch::keep`).
    keep: Value,
    job: Option<Arc<Job>>,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    exited: Arc<AtomicBool>,
    end: watch::Receiver<Option<End>>,
    /// How many agents work in each folder.
    dirs: Mutex<HashMap<String, usize>>,
    /// The folders being let go (`agent/letGo`) that have not answered yet: not to be given to an agent before.
    disposing: Mutex<HashSet<String>>,
}

/// What runs, by what it was started with, with how many agents use each.
type Running = HashMap<String, (Arc<Shared>, usize)>;
static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

/// The process of `key` for one more agent, which works in `dir`: one that runs (and, `one_a_folder`, has no agent in that
/// folder), else one started by `start` (given its own key).
pub fn acquire(
    key: &str,
    dir: &str,
    one_a_folder: bool,
    start: impl FnOnce(&str) -> Result<Arc<Shared>>,
) -> Result<Arc<Shared>> {
    let mut all = RUNNING.lock();
    let map = all.get_or_insert_with(HashMap::new);
    let mut n = 0;
    let own = loop {
        let k = if n == 0 {
            key.to_string()
        } else {
            format!("{key}#{n}")
        };
        match map.get_mut(&k) {
            Some((shared, users)) if shared.is_alive() => {
                if !(one_a_folder && shared.holds(dir)) {
                    *users += 1;
                    shared.enter(dir);
                    return Ok(shared.clone());
                }
            }
            _ => break k,
        }
        n += 1;
    };
    let shared = start(&own)?;
    shared.enter(dir);
    map.insert(own, (shared.clone(), 1));
    Ok(shared)
}

/// One agent less uses `shared`: stopped once none does.
pub fn release(shared: &Arc<Shared>) {
    let last = {
        let mut all = RUNNING.lock();
        let Some(map) = all.as_mut() else { return };
        match map.get_mut(&shared.key) {
            Some((s, users)) if Arc::ptr_eq(s, shared) => {
                *users = users.saturating_sub(1);
                if *users == 0 {
                    map.remove(&shared.key);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    };
    if last {
        log::info!("the last agent of a shared harness stopped, the harness too");
        shared.kill();
    }
}

/// The ports the processes of `plugin` in the runtime `rt` said they listen on, while they live: all `local:shared` may
/// reach. Never a port the plugin named.
pub fn ports_of(rt: usize, plugin: &str) -> Vec<u16> {
    let prefix = format!("{rt}|{plugin}|");
    RUNNING.lock().as_ref().map_or(Vec::new(), |map| {
        map.values()
            .filter(|(s, _)| s.is_alive() && s.key.starts_with(&prefix))
            .map(|(s, _)| s.port.load(Ordering::Acquire))
            .filter(|p| *p != 0)
            .collect()
    })
}

/// The port the line `line` says, by the template `listens`: its text around `{host}` and `{port}`, the host being
/// `127.0.0.1` and nothing else.
pub fn port_in(listens: &str, line: &str) -> Option<u16> {
    let (before, rest) = listens.split_once("{host}")?;
    let (between, after) = rest.split_once("{port}")?;
    let head = format!("{before}127.0.0.1{between}");
    let tail = &line[line.find(&head)? + head.len()..];
    let digits = tail.chars().take_while(char::is_ascii_digit).count();
    tail[digits..].starts_with(after).then_some(())?;
    tail[..digits].parse().ok().filter(|p| *p != 0)
}

/// How many run (the tests).
#[cfg(test)]
pub fn running() -> usize {
    RUNNING
        .lock()
        .as_ref()
        .map_or(0, |m| m.values().filter(|(s, _)| s.is_alive()).count())
}

/// The folder of a shared process: none of the agents'.
pub fn folder(harness: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("escouade-{harness}"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Which process the agents of `program` and `env` share, in the runtime `rt` of the plugins.
pub fn key_of(rt: usize, plugin: &str, program: &Program, env: &[(String, String)]) -> String {
    let mut env: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();
    env.sort();
    format!("{rt}|{plugin}|{}|{}", program.describe(), env.join("|"))
}

/// A process's line as text: invalid UTF-8 replaced, without its `\r`.
fn text_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\r')
        .replace('\0', "")
}

/// Keeps `line` among the last 30 of `tail`.
fn note(tail: &Mutex<VecDeque<String>>, line: String) {
    let mut t = tail.lock();
    t.push_back(line);
    if t.len() > 30 {
        t.pop_front();
    }
}

/// How a shared process is started.
pub struct Start<'a> {
    pub key: &'a str,
    pub name: &'a str,
    pub program: &'a Program,
    pub args: &'a [String],
    pub env: &'a [(String, String)],
    pub remove_env: &'a [String],
    /// What the line that says it is ready holds.
    pub ready: &'a str,
    /// The template of the line that says its address (`SharedLaunch::listens`).
    pub listens: Option<&'a str>,
    pub keep: Value,
    pub dir: &'a std::path::Path,
}

impl Shared {
    /// Starts it.
    pub fn start(s: Start<'_>) -> Result<Arc<Shared>> {
        let cwd = s.dir.to_string_lossy();
        let mut cmd = command_of(s.program, &cwd, s.args, s.env, s.remove_env)?;
        cmd.stdin(Stdio::null());
        let mut child = cmd.spawn().with_context(|| {
            let program = s.program.describe();
            tr!("impossible de lancer {program}", "can’t start {program}")
        })?;
        let job = Job::for_child(&child).map(Arc::new);
        if job.is_none() {
            log::warn!(
                "{} (pid {:?}) is not in a job: its tree is not killed as a whole",
                s.name,
                child.id()
            );
        }
        let stdout = child.stdout.take().context("stdout")?;
        let stderr = child.stderr.take().context("stderr")?;
        let (line_tx, line) = watch::channel(None);
        let (end_tx, end) = watch::channel(None);
        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
        let exited = Arc::new(AtomicBool::new(false));
        let port = Arc::new(AtomicU16::new(0));
        let shared = Arc::new(Shared {
            key: s.key.to_string(),
            port: port.clone(),
            line,
            keep: s.keep,
            job: job.clone(),
            kill_tx: Mutex::new(Some(kill_tx)),
            exited: exited.clone(),
            end,
            dirs: Mutex::default(),
            disposing: Mutex::default(),
        });
        let tail = Arc::new(Mutex::new(VecDeque::<String>::new()));
        {
            let tail = tail.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).split(b'\n');
                while let Ok(Some(line)) = lines.next_segment().await {
                    note(&tail, text_line(&line));
                }
            });
        }
        let weak = Arc::downgrade(&shared);
        let ready = s.ready.to_string();
        let listens = s.listens.map(str::to_string);
        let key = s.key.to_string();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).split(b'\n');
            let mut line_tx = Some(line_tx);
            loop {
                tokio::select! {
                    line = lines.next_segment() => match line {
                        Ok(Some(line)) => {
                            let line = text_line(&line);
                            if port.load(Ordering::Acquire) == 0 {
                                if let Some(p) = listens.as_deref().and_then(|l| port_in(l, &line)) {
                                    port.store(p, Ordering::Release);
                                }
                            }
                            if line_tx.is_some() && line.contains(&ready) {
                                if let Some(tx) = line_tx.take() {
                                    let _ = tx.send(Some(Ok(line)));
                                }
                            } else {
                                note(&tail, line);
                            }
                        }
                        _ => break,
                    },
                    _ = &mut kill_rx => {
                        let _ = child.start_kill();
                        break;
                    }
                }
            }
            let code = child.wait().await.ok().and_then(|s| s.code());
            exited.store(true, Ordering::Release);
            let said = tail.lock().iter().cloned().collect::<Vec<_>>().join("\n");
            if let Some(tx) = line_tx.take() {
                let _ = tx.send(Some(Err(said.clone())));
            }
            log::info!("{key}: a shared harness exited ({code:?})");
            if let Some(s) = weak.upgrade() {
                // Not to be given to a new agent any more.
                let mut all = RUNNING.lock();
                if let Some(map) = all.as_mut() {
                    if map.get(&s.key).is_some_and(|(x, _)| Arc::ptr_eq(x, &s)) {
                        map.remove(&s.key);
                    }
                }
            }
            let _ = end_tx.send(Some((code, said)));
            drop(job);
        });
        Ok(shared)
    }

    pub fn is_alive(&self) -> bool {
        !self.exited.load(Ordering::Acquire)
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn keep(&self) -> &Value {
        &self.keep
    }

    pub fn pids(&self) -> Vec<u32> {
        self.job.as_ref().map(|j| j.pids()).unwrap_or_default()
    }

    /// Its end, once it comes.
    pub fn end(&self) -> watch::Receiver<Option<End>> {
        self.end.clone()
    }

    fn kill(&self) {
        if let Some(job) = &self.job {
            job.terminate();
        }
        if let Some(k) = self.kill_tx.lock().take() {
            let _ = k.send(());
        }
    }

    /// Waits for the line that says it is ready.
    pub async fn ready(&self, name: &str) -> Result<String> {
        let mut line = self.line.clone();
        let wait = async {
            loop {
                if let Some(r) = line.borrow().clone() {
                    break r;
                }
                if line.changed().await.is_err() {
                    break Err(String::new());
                }
            }
        };
        match tokio::time::timeout(START_LIMIT, wait).await {
            Ok(Ok(line)) => Ok(line),
            Ok(Err(said)) => Err(anyhow!(
                "{}{}",
                tr!(
                    "{name} s’est arrêté avant d’être prêt",
                    "{name} stopped before it was ready"
                ),
                if said.trim().is_empty() {
                    String::new()
                } else {
                    format!(" : {}", crate::text::truncate(said.trim(), 2000))
                }
            )),
            Err(_) => Err(anyhow!(tr!(
                "{name} ne s’est pas mis à l’écoute à temps",
                "{name} didn’t start listening in time"
            ))),
        }
    }

    /// One more agent works in `dir`.
    fn enter(&self, dir: &str) {
        *self.dirs.lock().entry(dir.to_string()).or_default() += 1;
    }

    /// An agent works in `dir`, or it is being let go.
    fn holds(&self, dir: &str) -> bool {
        self.dirs.lock().contains_key(dir) || self.disposing.lock().contains(dir)
    }

    /// `dir` is being let go (`done`: no longer).
    pub fn disposing(&self, dir: &str, done: bool) {
        let mut d = self.disposing.lock();
        if done {
            d.remove(dir);
        } else {
            d.insert(dir.to_string());
        }
    }

    /// One agent less works in `dir`: whether it was the last.
    pub fn leave(&self, dir: &str) -> bool {
        let mut dirs = self.dirs.lock();
        let Some(n) = dirs.get_mut(dir) else {
            return false;
        };
        *n = n.saturating_sub(1);
        if *n == 0 {
            dirs.remove(dir);
            return true;
        }
        false
    }
}
