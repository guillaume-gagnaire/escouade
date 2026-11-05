//! `proc/*` (ESC-333, docs/PLUGINS.md §8.4.7): the programs a plugin in WASM runs, on `permissions.commands`.
//!
//! **The module is isolated, the program it runs is not**: it has the user's rights. So the host holds what can be held:
//! - the first word is a program the core finds on the PATH ([`locate`]), never a path; no shell, and under Windows no
//!   `.cmd` nor `.bat`, which only run through one;
//! - the command and its arguments are an entry of `permissions.commands`, word for word; only an entry that ends in `*`
//!   lets more follow ([`allowed`]);
//! - `dir` is a root (`agent`, `project`), never a path, and a private project refuses it unless the plugin was turned on
//!   for it; without `dir`, its temporary folder;
//! - the environment is an agent's, without Escouade's own variables, plus those the plugin names ([`environment`]);
//! - each program is in a Job object held by the plugin's instance: it dies with it, or at `proc/kill` ([`Procs::end`]);
//! - 4 at once; what it writes is lines on the plugin's channel, and the pipe is not read past a credit of 256 KiB not yet
//!   written on the module's input: a program that writes without end slows down, the app's memory does not grow.
//!
//! Each start goes to the journal, which masks the plugin's secrets.

use super::beside::Back;
use super::journal::Kind;
use super::wasm::Frame;
use crate::job::Job;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot, Semaphore};

/// As `net/fetch`'s: what it may not run.
pub const REFUSED: i64 = super::files::REFUSED;
/// A program that could not start, or is not there any more.
pub const FAILED: i64 = super::files::FAILED;
/// How many programs a plugin has at once.
pub const AT_ONCE: usize = 4;
/// What may be on its way to the module, the credit of a program's output.
const CREDIT: usize = 256 << 10;
/// A read of its output, at most.
const CHUNK: usize = 16 << 10;
/// What may wait for a program that does not read its input.
const WAITING_INPUT: usize = 1 << 20;
/// How long what it left running may keep its pipes open, once it ended.
const LINGER: Duration = Duration::from_secs(1);

/// The variables a plugin may not name: Escouade's own, and those that would run another program in the one declared.
fn forbidden_var(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    name.is_empty()
        || name.contains(['=', '\0'])
        || upper.starts_with("ESCOUADE_")
        || upper.starts_with("LD_")
        || upper.starts_with("DYLD_")
        || matches!(
            upper.as_str(),
            "PATH" | "PATHEXT" | "COMSPEC" | "NODE_OPTIONS" | "BASH_ENV" | "ENV" | "IFS"
        )
}

/// The environment of a program: what the app has but its `ESCOUADE_*`, is left to inherit; `env` an agent gets (the
/// network of Réglages), then what the plugin names. What is taken out, and what is set.
pub fn environment(
    of_the_app: impl Iterator<Item = String>,
    agent: &[(String, String)],
    named: Vec<(String, String)>,
) -> (Vec<String>, Vec<(String, String)>) {
    let out = of_the_app
        .filter(|k| k.to_ascii_uppercase().starts_with("ESCOUADE_"))
        .collect();
    let mut set = agent.to_vec();
    set.retain(|(k, _)| !named.iter().any(|(n, _)| n == k));
    set.extend(named);
    (out, set)
}

/// A program of the PATH by its name, its absolute folders only: no `.cmd` nor `.bat` under Windows, which need a shell,
/// no file that does not run elsewhere.
pub fn locate(name: &str) -> Option<PathBuf> {
    // The test double is a script, which only its tests start.
    #[cfg(test)]
    if name == "isola" {
        return crate::isola::cli();
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|dir| dir.is_absolute())
        .find_map(|dir| {
            if cfg!(windows) {
                ["exe", "com"]
                    .iter()
                    .map(|ext| dir.join(format!("{name}.{ext}")))
                    .find(|p| p.is_file())
            } else {
                #[cfg(unix)]
                {
                    let p = dir.join(name);
                    runs(&p).then_some(p)
                }
                #[cfg(not(unix))]
                {
                    None
                }
            }
        })
}

#[cfg(unix)]
fn runs(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// Whether `words` (the command and its arguments) are an entry of `commands`, word for word; what follows only when the
/// entry ends in `*`.
pub fn allowed(commands: &[Vec<String>], words: &[String]) -> bool {
    commands.iter().any(|entry| match entry.split_last() {
        Some((last, head)) if last == "*" => {
            // Never a bare `*`: it would be any program.
            !head.is_empty() && words.len() >= head.len() && words[..head.len()] == *head
        }
        _ => words == entry.as_slice(),
    })
}

/// A program of a plugin under way.
struct Live {
    /// What it writes on the program's input; None closes it.
    input: mpsc::UnboundedSender<Option<Vec<u8>>>,
    /// What waits on it.
    waiting: Arc<AtomicUsize>,
    kill: Option<oneshot::Sender<()>>,
    job: Option<Arc<Job>>,
}

/// What the journal is told: its kind, its line.
pub type Told = Arc<dyn Fn(Kind, &str) + Send + Sync>;
/// The folder of a project the plugin may run in, or why not.
pub type Projects = Arc<dyn Fn(&str) -> Result<PathBuf, String> + Send + Sync>;

/// The programs of an instance of a plugin.
pub struct Procs {
    commands: Vec<Vec<String>>,
    temp: PathBuf,
    agents: super::files::Agents,
    projects: Projects,
    agent_env: Vec<(String, String)>,
    back: Back,
    told: Told,
    live: Arc<Mutex<HashMap<u64, Live>>>,
    next: AtomicU64,
    /// Once the instance ended, no more.
    over: Arc<std::sync::atomic::AtomicBool>,
}

/// What a `Procs` is made of.
pub struct ProcsOf {
    pub commands: Vec<String>,
    pub temp: PathBuf,
    pub agents: super::files::Agents,
    pub projects: Projects,
    pub agent_env: Vec<(String, String)>,
    pub back: mpsc::WeakUnboundedSender<Frame>,
    pub told: Told,
}

/// `{ proc }`, or why not.
fn proc_of(params: &Value) -> Result<u64, (i64, String)> {
    params.get("proc").and_then(Value::as_u64).ok_or_else(|| {
        (
            super::runtime::INVALID_PARAMS,
            "proc/* takes a proc".to_string(),
        )
    })
}

impl Procs {
    pub fn new(of: ProcsOf) -> Self {
        Self {
            commands: of
                .commands
                .iter()
                .map(|c| c.split_whitespace().map(str::to_string).collect())
                .collect(),
            temp: of.temp,
            agents: of.agents,
            projects: of.projects,
            agent_env: of.agent_env,
            back: Back(of.back),
            told: of.told,
            live: Arc::default(),
            next: AtomicU64::new(1),
            over: Arc::default(),
        }
    }

    /// Serves `proc/<method>`.
    pub fn serve(&self, method: &str, params: &Value) -> Result<Value, (i64, String)> {
        match method {
            "proc/spawn" => self.spawn(params).inspect_err(|(code, message)| {
                if *code == REFUSED {
                    (self.told)(Kind::Refused, &format!("proc/spawn: refused, {message}"));
                }
            }),
            "proc/write" => self.write(params),
            "proc/kill" => self.kill(params),
            _ => Err((
                super::runtime::METHOD_NOT_FOUND,
                format!("unknown method {method}"),
            )),
        }
    }

    /// Where it runs: `dir` a root, never a path.
    fn folder(&self, params: &Value) -> Result<(PathBuf, &'static str), (i64, String)> {
        let refused = |why: String| (REFUSED, why);
        let agent = || -> Result<_, (i64, String)> {
            let key = params.get("agent").and_then(Value::as_str);
            let key = key.ok_or_else(|| refused("this dir needs an agent".into()))?;
            self.agents
                .lock()
                .get(key)
                .cloned()
                .ok_or_else(|| refused(format!("no agent {key:?} was given to this plugin")))
        };
        match params.get("dir").and_then(Value::as_str) {
            None if params.get("dir").is_none_or(Value::is_null) => {
                std::fs::create_dir_all(&self.temp)
                    .map_err(|e| (FAILED, format!("its temporary folder: {e}")))?;
                Ok((self.temp.clone(), "temp"))
            }
            Some("agent") => {
                let place = agent()?;
                if !place.project.is_empty() {
                    (self.projects)(&place.project).map_err(refused)?;
                }
                Ok((place.cwd, "agent"))
            }
            Some("project") => {
                let project = match params.get("project").and_then(Value::as_str) {
                    Some(p) => p.to_string(),
                    None => agent()?.project,
                };
                (self.projects)(&project)
                    .map(|p| (p, "project"))
                    .map_err(refused)
            }
            _ => Err(refused(format!(
                "dir is agent or project, never a path: {}",
                params["dir"]
            ))),
        }
    }

    /// `{ command, args, dir?, agent?, project?, env? }` → `{ proc }`.
    fn spawn(&self, params: &Value) -> Result<Value, (i64, String)> {
        let invalid = |why: &str| (super::runtime::INVALID_PARAMS, why.to_string());
        let command = params
            .get("command")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("proc/spawn takes a command"))?;
        let mut words = vec![command.to_string()];
        if let Some(args) = params.get("args").filter(|a| !a.is_null()) {
            let args = args
                .as_array()
                .ok_or_else(|| invalid("args is a list of words"))?;
            for a in args {
                words.push(
                    a.as_str()
                        .ok_or_else(|| invalid("args is a list of words"))?
                        .to_string(),
                );
            }
        }
        let shown = words.join(" ");
        if !allowed(&self.commands, &words) {
            return Err((REFUSED, format!("{shown} is not in permissions.commands")));
        }
        let (dir, root) = self.folder(params)?;
        let mut named = Vec::new();
        if let Some(env) = params.get("env").filter(|e| !e.is_null()) {
            let env = env
                .as_object()
                .ok_or_else(|| invalid("env is an object of texts"))?;
            for (k, v) in env {
                if forbidden_var(k) {
                    return Err((REFUSED, format!("the variable {k} is not one to name")));
                }
                let v = v
                    .as_str()
                    .ok_or_else(|| invalid("env is an object of texts"))?;
                named.push((k.clone(), v.to_string()));
            }
        }
        if self.live.lock().len() >= AT_ONCE {
            return Err((REFUSED, format!("{AT_ONCE} programs at once at most")));
        }
        if self.over.load(Ordering::Acquire) {
            return Err((FAILED, "its instance has ended".into()));
        }
        let program = locate(command)
            .ok_or_else(|| (FAILED, format!("no program {command} on this machine")))?;
        let (out, set) = environment(
            std::env::vars_os().map(|(k, _)| k.to_string_lossy().into_owned()),
            &self.agent_env,
            named,
        );
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        // The pipes of the child belong to the host's runtime.
        let _inside = super::wasm::net().enter();
        let mut cmd = tokio::process::Command::new(&program);
        cmd.args(&words[1..])
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for k in out {
            cmd.env_remove(k);
        }
        cmd.envs(set);
        #[cfg(windows)]
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
        crate::job::isolate(&mut cmd);
        crate::proc::no_inherited_fds(&mut cmd);
        let mut child = cmd.spawn().map_err(|e| (FAILED, format!("{shown}: {e}")))?;
        let job = Job::for_child(&child).map(Arc::new);
        if job.is_none() {
            log::warn!("{shown} is not in a job: its tree is not killed as a whole");
        }
        (self.told)(Kind::Message, &format!("proc/spawn: ran {shown} in {root}"));
        let stdin = child.stdin.take().expect("piped");
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        let waiting = Arc::new(AtomicUsize::new(0));
        let (input, inputs) = mpsc::unbounded_channel();
        let (kill, killed) = oneshot::channel();
        self.live.lock().insert(
            id,
            Live {
                input,
                waiting: waiting.clone(),
                kill: Some(kill),
                job: job.clone(),
            },
        );
        tokio::spawn(feed(stdin, inputs, waiting));
        let credit = Arc::new(Semaphore::new(CREDIT));
        let pipes = (
            tokio::spawn(pipe(
                stdout,
                id,
                "stdout",
                self.back.clone(),
                credit.clone(),
            )),
            tokio::spawn(pipe(stderr, id, "stderr", self.back.clone(), credit)),
        );
        tokio::spawn(wait(Waited {
            child,
            id,
            shown,
            job,
            killed,
            pipes,
            back: self.back.clone(),
            live: self.live.clone(),
            told: self.told.clone(),
        }));
        Ok(json!({ "proc": id }))
    }

    /// `{ proc, data?, close? }`.
    fn write(&self, params: &Value) -> Result<Value, (i64, String)> {
        let id = proc_of(params)?;
        let live = self.live.lock();
        let Some(p) = live.get(&id) else {
            return Err((FAILED, format!("no program {id}")));
        };
        if let Some(data) = params.get("data").and_then(Value::as_str) {
            if p.waiting.load(Ordering::Acquire) + data.len() > WAITING_INPUT {
                return Err((FAILED, "the program does not read its input".into()));
            }
            p.waiting.fetch_add(data.len(), Ordering::AcqRel);
            let _ = p.input.send(Some(data.as_bytes().to_vec()));
        }
        if params.get("close").and_then(Value::as_bool) == Some(true) {
            let _ = p.input.send(None);
        }
        Ok(Value::Null)
    }

    /// `{ proc }`: gone at once, with what it started; `proc/exit` follows.
    fn kill(&self, params: &Value) -> Result<Value, (i64, String)> {
        let id = proc_of(params)?;
        let mut live = self.live.lock();
        let Some(p) = live.get_mut(&id) else {
            return Err((FAILED, format!("no program {id}")));
        };
        if let Some(job) = &p.job {
            job.terminate();
        }
        if let Some(kill) = p.kill.take() {
            let _ = kill.send(());
        }
        Ok(Value::Null)
    }

    /// The instance ended: every program with it.
    pub fn end(&self) {
        self.over.store(true, Ordering::Release);
        let mut live = self.live.lock();
        for p in live.values_mut() {
            if let Some(job) = &p.job {
                job.terminate();
            }
            if let Some(kill) = p.kill.take() {
                let _ = kill.send(());
            }
        }
    }

    /// The commands under way.
    #[cfg(test)]
    fn running(&self) -> usize {
        self.live.lock().len()
    }
}

impl Drop for Procs {
    fn drop(&mut self) {
        self.end();
    }
}

/// Writes what comes on the program's input; None closes it.
async fn feed(
    mut stdin: tokio::process::ChildStdin,
    mut inputs: mpsc::UnboundedReceiver<Option<Vec<u8>>>,
    waiting: Arc<AtomicUsize>,
) {
    while let Some(Some(bytes)) = inputs.recv().await {
        let n = bytes.len();
        let ok = stdin.write_all(&bytes).await.is_ok() && stdin.flush().await.is_ok();
        waiting.fetch_sub(n, Ordering::AcqRel);
        if !ok {
            break;
        }
    }
}

/// Sends `text` to the module, once it fits in the credit; false when the plugin is gone. The credit comes back when the
/// line is on the module's input.
async fn send(text: &str, id: u64, stream: &str, back: &Back, credit: &Arc<Semaphore>) -> bool {
    let n = text.len().clamp(1, CREDIT) as u32;
    let Ok(permit) = credit.clone().acquire_many_owned(n).await else {
        return false;
    };
    let frame = json!({
        "jsonrpc": "2.0",
        "method": "proc/out",
        "params": { "proc": id, "stream": stream, "data": text },
    });
    let (written, done) = oneshot::channel();
    if !back.send(&frame, Some(written)) {
        return false;
    }
    tokio::spawn(async move {
        let _ = done.await;
        drop(permit);
    });
    true
}

/// Reads a pipe of the program, text cut only between two characters.
async fn pipe(
    mut from: impl AsyncRead + Unpin,
    id: u64,
    stream: &'static str,
    back: Back,
    credit: Arc<Semaphore>,
) {
    let mut buf = vec![0u8; CHUNK];
    let mut held: Vec<u8> = Vec::new();
    loop {
        let n = match from.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        held.extend_from_slice(&buf[..n]);
        let whole = match std::str::from_utf8(&held) {
            Ok(_) => held.len(),
            // The start of a character the next read ends.
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            Err(_) => held.len(),
        };
        let rest = held.split_off(whole);
        let text = String::from_utf8_lossy(&held).into_owned();
        held = rest;
        if !text.is_empty() && !send(&text, id, stream, &back, &credit).await {
            return;
        }
    }
    if !held.is_empty() {
        send(&String::from_utf8_lossy(&held), id, stream, &back, &credit).await;
    }
}

struct Waited {
    child: tokio::process::Child,
    id: u64,
    shown: String,
    job: Option<Arc<Job>>,
    killed: oneshot::Receiver<()>,
    pipes: (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>),
    back: Back,
    live: Arc<Mutex<HashMap<u64, Live>>>,
    told: Told,
}

/// Waits for the program, then says its end, after all it wrote.
async fn wait(mut w: Waited) {
    let status = tokio::select! {
        s = w.child.wait() => s,
        _ = &mut w.killed => {
            let _ = w.child.start_kill();
            w.child.wait().await
        }
    };
    let code = status.ok().and_then(|s| s.code());
    let (mut out, mut err) = w.pipes;
    // What it left running may hold its pipes: not for long.
    let joined = tokio::time::timeout(LINGER, async {
        let _ = (&mut out).await;
        let _ = (&mut err).await;
    })
    .await;
    if joined.is_err() {
        out.abort();
        err.abort();
    }
    w.live.lock().remove(&w.id);
    drop(w.job);
    (w.told)(
        Kind::Message,
        &format!("proc/exit: {} ended, code {code:?}", w.shown),
    );
    w.back
        .tell("proc/exit", json!({ "proc": w.id, "code": code }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::files::AgentPlace;
    use std::time::Instant;
    use tokio::sync::mpsc::UnboundedReceiver;

    fn words(c: &str) -> Vec<String> {
        c.split_whitespace().map(str::to_string).collect()
    }

    fn commands(list: &[&str]) -> Vec<Vec<String>> {
        list.iter().map(|c| words(c)).collect()
    }

    #[test]
    fn a_command_is_an_entry_word_for_word_and_only_a_star_lets_more_follow() {
        let c = commands(&["isola ls --json", "isola down", "isola logs *", "*"]);
        assert!(allowed(&c, &words("isola ls --json")));
        assert!(allowed(&c, &words("isola down")));
        assert!(allowed(&c, &words("isola logs")));
        assert!(allowed(&c, &words("isola logs web --tail 10")));
        assert!(!allowed(&c, &words("isola ls")));
        assert!(!allowed(&c, &words("isola ls --json --all")));
        assert!(!allowed(&c, &words("isola down now")));
        assert!(!allowed(&c, &words("isola destroy")));
        assert!(!allowed(&c, &words("ISOLA down")));
        // A bare `*` is no program.
        assert!(!allowed(&c, &words("rm -rf x")));
        assert!(!allowed(&c, &[]));
        // One word, not a line split again by a shell.
        assert!(!allowed(&c, &["isola down; rm -rf x".to_string()]));
    }

    #[test]
    fn the_environment_is_an_agents_without_those_of_escouade_plus_those_it_names() {
        let (out, set) = environment(
            ["PATH", "ESCOUADE_MCP_TOKEN", "escouade_x", "HOME"]
                .iter()
                .map(|s| s.to_string()),
            &[("HTTPS_PROXY".into(), "http://p".into())],
            vec![("MA_VAR".into(), "1".into())],
        );
        assert_eq!(out, vec!["ESCOUADE_MCP_TOKEN", "escouade_x"]);
        let names: Vec<&str> = set.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(names, vec!["HTTPS_PROXY", "MA_VAR"]);
        for k in [
            "ESCOUADE_MCP_TOKEN",
            "PATH",
            "path",
            "LD_PRELOAD",
            "DYLD_INSERT_LIBRARIES",
            "A=B",
            "",
        ] {
            assert!(forbidden_var(k), "{k}");
        }
        assert!(!forbidden_var("MA_VAR"));
    }

    #[test]
    fn a_program_is_found_on_the_path_never_by_a_path() {
        let found = locate("git").expect("git on the PATH");
        assert!(found.is_absolute() && found.is_file(), "{found:?}");
        assert!(locate("no-such-program-at-all").is_none());
    }

    struct Bed {
        procs: Procs,
        rx: UnboundedReceiver<Frame>,
        _tx: mpsc::UnboundedSender<Frame>,
        told: Arc<Mutex<Vec<String>>>,
        base: PathBuf,
    }

    fn bed(list: &[&str]) -> Bed {
        let (tx, rx) = mpsc::unbounded_channel();
        let base =
            std::env::temp_dir().join(format!("plugin-procs-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(base.join("wt")).unwrap();
        std::fs::create_dir_all(base.join("project")).unwrap();
        let told: Arc<Mutex<Vec<String>>> = Arc::default();
        let t = told.clone();
        let agents: super::super::files::Agents = Arc::default();
        agents.lock().insert(
            "a".into(),
            AgentPlace {
                cwd: base.join("wt"),
                project: "open".into(),
                ..AgentPlace::default()
            },
        );
        agents.lock().insert(
            "p".into(),
            AgentPlace {
                cwd: base.join("wt"),
                project: "private".into(),
                ..AgentPlace::default()
            },
        );
        let b = base.clone();
        let procs = Procs::new(ProcsOf {
            commands: list.iter().map(|c| c.to_string()).collect(),
            temp: base.join("temp"),
            agents,
            projects: Arc::new(move |p| match p {
                "open" => Ok(b.join("project")),
                _ => Err(format!("the project {p} is private")),
            }),
            agent_env: Vec::new(),
            back: tx.downgrade(),
            told: Arc::new(move |_, l| t.lock().push(l.to_string())),
        });
        Bed {
            procs,
            rx,
            _tx: tx,
            told,
            base,
        }
    }

    impl Bed {
        fn refused(&self, params: Value) {
            let r = self.procs.serve("proc/spawn", &params);
            assert_eq!(
                r.as_ref().err().map(|e| e.0),
                Some(REFUSED),
                "{params}: {r:?}"
            );
        }

        fn spawn(&self, params: Value) -> u64 {
            self.procs.serve("proc/spawn", &params).unwrap()["proc"]
                .as_u64()
                .unwrap()
        }

        /// What comes until the end of `id`: what it wrote on `stream`, and its code.
        async fn run(&mut self, id: u64, stream: &str) -> (String, Value) {
            let mut text = String::new();
            let end = Instant::now() + Duration::from_secs(20);
            loop {
                let got = tokio::time::timeout_at(end.into(), self.rx.recv()).await;
                let (bytes, written) = got.expect("in time").expect("open");
                if let Some(w) = written {
                    let _ = w.send(());
                }
                let v: Value = serde_json::from_slice(&bytes).unwrap();
                if v["params"]["proc"] != id {
                    continue;
                }
                match v["method"].as_str() {
                    Some("proc/out") if v["params"]["stream"] == stream => {
                        text.push_str(v["params"]["data"].as_str().unwrap());
                    }
                    Some("proc/exit") => return (text, v["params"]["code"].clone()),
                    _ => {}
                }
            }
        }
    }

    #[tokio::test]
    async fn a_program_runs_its_output_then_its_end_come_on_the_channel_and_go_to_the_journal() {
        let mut b = bed(&["git --version"]);
        let id = b.spawn(json!({ "command": "git", "args": ["--version"] }));
        let (out, code) = b.run(id, "stdout").await;
        assert!(out.starts_with("git version"), "{out}");
        assert_eq!(code, 0);
        let told = b.told.lock();
        assert!(
            told[0].starts_with("proc/spawn: ran git --version"),
            "{told:?}"
        );
        assert!(told[1].contains("code Some(0)"), "{told:?}");
    }

    #[tokio::test]
    async fn what_is_not_declared_does_not_run_and_the_refusal_is_said() {
        let b = bed(&["git --version", "git log *"]);
        b.refused(json!({ "command": "git", "args": ["status"] }));
        b.refused(json!({ "command": "git", "args": ["--version", "--build-options"] }));
        b.refused(json!({ "command": "sh", "args": ["-c", "true"] }));
        // A path is never a program.
        b.refused(json!({ "command": "/usr/bin/git", "args": ["--version"] }));
        b.refused(json!({ "command": "C:\\Windows\\System32\\cmd.exe" }));
        // No free folder.
        b.refused(json!({ "command": "git", "args": ["--version"], "dir": "/tmp" }));
        // No variable of Escouade.
        b.refused(json!({ "command": "git", "args": ["--version"], "env": { "ESCOUADE_X": "1" } }));
        assert_eq!(b.told.lock().len(), 7);
        assert!(b.told.lock().iter().all(|l| l.contains("refused")));
        assert_eq!(b.procs.running(), 0);
    }

    /// isola is the one plugin of Escouade's own hand that runs a program: what its manifest names runs, and nothing else.
    #[tokio::test]
    async fn isola_runs_what_its_manifest_names_and_nothing_else() {
        // The test double is a script of node's.
        #[cfg(windows)]
        crate::paths::trim_test_path();
        crate::isola::tests::use_fake();
        let manifest: Value =
            serde_json::from_str(include_str!("../../../plugins/isola/escouade-plugin.json"))
                .unwrap();
        let list: Vec<&str> = manifest["permissions"]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap())
            .collect();
        assert_eq!(list, ["isola ls --json", "isola down", "isola destroy"]);
        let mut b = bed(&list);
        let run = |command: &str, args: Value| json!({ "command": command, "args": args, "dir": "agent", "agent": "a" });
        for (command, args) in [
            // Another program, what isola's `.isola.toml` would run, and what could stand for it.
            ("git", json!(["--version"])),
            ("node", json!(["-e", "1"])),
            ("sh", json!(["-c", "isola down"])),
            ("cmd", json!(["/c", "isola down"])),
            // isola, but not as it is declared: `up` only runs from the core's terminal, once the user read the file.
            ("isola", json!(["up"])),
            ("isola", json!(["ls"])),
            ("isola", json!(["ls", "--json", "--all"])),
            ("isola", json!(["down", "--force"])),
            ("isola", json!(["destroy", "; calc"])),
            ("isola", json!([])),
            // A path, one word for two, another case.
            ("./isola", json!(["down"])),
            ("isola down", json!([])),
            ("ISOLA", json!(["down"])),
        ] {
            b.refused(run(command, args));
        }
        let wt = b.base.join("wt");
        assert!(
            crate::isola::tests::calls(&wt).is_empty(),
            "nothing ran for all that"
        );
        assert_eq!(
            b.told
                .lock()
                .iter()
                .filter(|l| l.contains("refused"))
                .count(),
            13,
            "each refusal goes to the journal"
        );
        let id = b.spawn(run("isola", json!(["ls", "--json"])));
        let (out, code) = b.run(id, "stdout").await;
        assert_eq!((out.trim(), code), ("[]", json!(0)));
        assert_eq!(crate::isola::tests::calls(&wt), ["ls --json"]);
    }

    #[tokio::test]
    async fn a_program_runs_in_a_root_and_a_private_project_refuses_it() {
        let mut b = bed(&["git rev-parse --show-prefix"]);
        let run = json!({ "command": "git", "args": ["rev-parse", "--show-prefix"] });
        let with = |extra: Value| {
            let mut p = run.clone();
            p.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            p
        };
        for (d, agent) in [("agent", "a"), ("project", "a")] {
            let id = b.spawn(with(json!({ "dir": d, "agent": agent })));
            // Out of any repository, git says so: it ran where it was told.
            let (_, code) = b.run(id, "stderr").await;
            assert_eq!(code, 128);
        }
        let id = b.spawn(with(json!({ "dir": "project", "project": "open" })));
        b.run(id, "stderr").await;
        b.refused(with(json!({ "dir": "agent", "agent": "p" })));
        b.refused(with(json!({ "dir": "project", "agent": "p" })));
        b.refused(with(json!({ "dir": "project", "project": "private" })));
        b.refused(with(json!({ "dir": "agent", "agent": "nobody" })));
        b.refused(with(json!({ "dir": "agent" })));
    }

    #[tokio::test]
    async fn its_input_goes_to_it_and_a_closed_input_ends_it() {
        let mut b = bed(&["git hash-object --stdin"]);
        let id = b.spawn(json!({ "command": "git", "args": ["hash-object", "--stdin"] }));
        b.procs
            .serve(
                "proc/write",
                &json!({ "proc": id, "data": "bonjour\n", "close": true }),
            )
            .unwrap();
        let (out, code) = b.run(id, "stdout").await;
        assert_eq!(out.trim().len(), 40, "{out}");
        assert_eq!(code, 0);
    }

    #[tokio::test]
    async fn the_variables_it_names_reach_the_program() {
        let mut b = bed(&["git var GIT_AUTHOR_IDENT"]);
        let id = b.spawn(json!({
            "command": "git", "args": ["var", "GIT_AUTHOR_IDENT"],
            "env": { "GIT_AUTHOR_NAME": "Un plugin", "GIT_AUTHOR_EMAIL": "a@b.c" },
        }));
        let (out, _) = b.run(id, "stdout").await;
        assert!(out.starts_with("Un plugin <a@b.c>"), "{out}");
    }

    #[tokio::test]
    async fn four_run_at_once_at_most_a_kill_ends_one_and_the_end_of_the_instance_the_others() {
        let mut b = bed(&["git cat-file --batch"]);
        let batch = json!({ "command": "git", "args": ["cat-file", "--batch"], "dir": "agent", "agent": "a" });
        // Out of a repository it stops at once: a repository for it.
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(b.base.join("wt"))
            .status()
            .unwrap();
        let ids: Vec<u64> = (0..AT_ONCE).map(|_| b.spawn(batch.clone())).collect();
        b.refused(batch.clone());
        b.procs
            .serve("proc/kill", &json!({ "proc": ids[0] }))
            .unwrap();
        let (_, code) = b.run(ids[0], "stdout").await;
        assert_ne!(code, 0);
        assert_eq!(b.procs.running(), AT_ONCE - 1);
        let again = b.spawn(batch.clone());
        b.procs.end();
        let mut left = vec![ids[1], ids[2], ids[3], again];
        let end = Instant::now() + Duration::from_secs(20);
        while !left.is_empty() {
            let got = tokio::time::timeout_at(end.into(), b.rx.recv()).await;
            let (bytes, _) = got.expect("in time").expect("open");
            let v: Value = serde_json::from_slice(&bytes).unwrap();
            if v["method"] == "proc/exit" {
                left.retain(|i| v["params"]["proc"] != *i);
            }
        }
        assert_eq!(b.procs.serve("proc/spawn", &batch).unwrap_err().0, FAILED);
    }

    #[tokio::test]
    async fn a_program_that_writes_much_is_held_by_its_credit() {
        // `git cat-file --batch` on a big blob: more than the credit, before anything comes off the channel.
        let mut b = bed(&["git cat-file --batch"]);
        let wt = b.base.join("wt");
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(&wt)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        git(&["init", "-q"]);
        std::fs::write(wt.join("big"), "x".repeat(4 * CREDIT)).unwrap();
        let blob = git(&["hash-object", "-w", "big"]);
        let id = b.spawn(json!({ "command": "git", "args": ["cat-file", "--batch"], "dir": "agent", "agent": "a" }));
        b.procs
            .serve(
                "proc/write",
                &json!({ "proc": id, "data": format!("{blob}\n"), "close": true }),
            )
            .unwrap();
        // Nothing is written on the module's input: the host stops reading at its credit, and stays there.
        let mut waiting = 0;
        let mut frames = Vec::new();
        let end = Instant::now() + Duration::from_secs(20);
        while waiting < CREDIT - 2 * CHUNK && Instant::now() < end {
            match b.rx.try_recv() {
                Ok(f) => {
                    waiting += f.0.len();
                    frames.push(f);
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
            }
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        while let Ok(f) = b.rx.try_recv() {
            waiting += f.0.len();
            frames.push(f);
        }
        assert!(waiting >= CREDIT - 2 * CHUNK, "{waiting}");
        assert!(waiting <= CREDIT + 4 * CHUNK, "{waiting}");
        // Written, it comes back, and the rest comes.
        for (bytes, written) in frames {
            let v: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(v["method"], "proc/out");
            let _ = written.unwrap().send(());
        }
        let (out, code) = b.run(id, "stdout").await;
        assert_eq!(code, 0);
        assert!(out.ends_with("xxx\n"), "{}", out.len());
    }
}
