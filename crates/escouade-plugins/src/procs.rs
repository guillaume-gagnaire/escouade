//! `proc/*` (ESC-333, docs/PLUGINS-DESIGN.md §8.4.7): the programs a plugin in WASM runs, on `permissions.commands`.
//!
//! **The module is isolated, the program it runs is not**: it has the user's rights. So the host holds what can be held:
//! - the first word is a program the core finds on the PATH ([`locate`]), never a path; no shell, and under Windows no
//!   `.cmd` nor `.bat`, which only run through one;
//! - the command and its arguments are an entry of `permissions.commands`, word for word; only an entry that ends in `*`
//!   lets more follow ([`allowed`]). A joker of the entry (`{int}`, `{port}`, `{name}`, `{pid}`, `{path:temp}`,
//!   `{path:plugin}`, ESC-528) stands for one whole word of a closed form, never an option nor `--`: a `{pid}` is one of
//!   its programs alive, a path is a file under its root, which the host resolves and gives as its real absolute path;
//! - `dir` is a root (`agent`, `project`), never a path, and a private project refuses it unless the plugin was turned on
//!   for it; without `dir`, its temporary folder;
//! - the environment is an agent's, without Escouade's own variables, plus those the plugin names ([`environment`]);
//! - each program is in a Job object held by the plugin's instance: it dies with it, or at `proc/kill` ([`Procs::end`]);
//! - 4 at once, or what its manifest declares (`limits.processes`, ESC-529), never more than 16; what it writes is lines on the plugin's channel, and the pipe is not read past a credit of 256 KiB not yet
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
/// How many programs a plugin has at once without asking (`limits.processes` absent).
pub const AT_ONCE: usize = 4;
/// The most programs at once the host gives a plugin, whatever its manifest says (`manifest::PROCESSES`, §5.6 of
/// docs/PLUGINS-CONTRACT-STUDY.md): at about 1.3 MiB of the app each at worst, some 20 MiB.
pub const AT_ONCE_MAX: usize = 16;

/// The programs at once of a plugin that declares `processes` (`limits.processes`): 4 without it, from 1 to 16.
pub fn at_once(processes: Option<u64>) -> usize {
    processes.map_or(AT_ONCE, |n| {
        usize::try_from(n)
            .unwrap_or(AT_ONCE_MAX)
            .clamp(1, AT_ONCE_MAX)
    })
}
/// What may be on its way to the module, the credit of a program's output.
const CREDIT: usize = 256 << 10;
/// A read of its output, at most.
const CHUNK: usize = 16 << 10;
/// What may wait for a program that does not read its input.
const WAITING_INPUT: usize = 1 << 20;
/// How long what it left running may keep its pipes open, once it ended.
const LINGER: Duration = Duration::from_secs(1);

/// Why a plugin that declares `declared` (`permissions.env`) may not set the variable `name` on a program it runs, None
/// when it may (ESC-424): never one of [`forbidden_var`]; one it declares alone, and for anyone's plugin none of the
/// families `manifest::env_refused` names, even declared. A plugin of Escouade's own hand from before the field keeps what
/// it did.
pub fn var_refused(declared: Option<&[String]>, name: &str, first_hand: bool) -> Option<String> {
    use super::manifest::{env_allowed, env_refused, Hand};
    let hand = if first_hand { Hand::First } else { Hand::Third };
    if forbidden_var(name) || env_refused(name, hand) {
        return Some(format!(
            "the variable {name} is one Escouade refuses a plugin: it would change what the program runs"
        ));
    }
    (!env_allowed(declared, name, hand))
        .then(|| format!("the variable {name} is not in permissions.env"))
}

/// The variables no plugin names, of Escouade's own hand or not: Escouade's own, and those that would run another
/// program in the one declared.
pub fn forbidden_var(name: &str) -> bool {
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
    #[cfg(any(test, feature = "testing"))]
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

/// A word of an entry of `permissions.commands` (§5.3 of docs/PLUGINS-CONTRACT-STUDY.md, ESC-528): a literal, compared
/// byte for byte, or a joker, a whole word of a closed form that the host checks.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Word {
    Literal(String),
    /// `{int}`: `0|[1-9][0-9]{0,9}`, 4 294 967 295 at most.
    Int,
    /// `{port}`: an `{int}` of 1 to 65 535.
    Port,
    /// `{name}`: `[A-Za-z0-9_][A-Za-z0-9_.-]{0,63}`, never a device of Windows.
    Name,
    /// `{pid}`: the pid of a program alive this very instance runs.
    Pid,
    /// `{path:temp}`: a file of its temporary folder, given as its real absolute path.
    Temp,
    /// `{path:plugin}`: a file of its own folder, the same.
    Plugin,
    /// A brace this host does not know (`{float}`, `--pid={int}`), or a joker first: it matches nothing, never more.
    Never,
}

impl Word {
    fn of(word: &str) -> Self {
        match word {
            "{int}" => Self::Int,
            "{port}" => Self::Port,
            "{name}" => Self::Name,
            "{pid}" => Self::Pid,
            "{path:temp}" => Self::Temp,
            "{path:plugin}" => Self::Plugin,
            w if w.contains(['{', '}']) => Self::Never,
            w => Self::Literal(w.to_string()),
        }
    }

    /// How the journal names it.
    fn said(&self) -> &str {
        match self {
            Self::Literal(w) => w,
            Self::Int => "{int}",
            Self::Port => "{port}",
            Self::Name => "{name}",
            Self::Pid => "{pid}",
            Self::Temp => "{path:temp}",
            Self::Plugin => "{path:plugin}",
            Self::Never => "a word no command matches",
        }
    }
}

/// An entry of `permissions.commands`, read once: its words, and whether a last `*` lets any more follow. Its first word
/// is a program's name, never a joker; an entry that breaks that matches nothing.
#[derive(Debug, Clone)]
pub struct Entry {
    words: Vec<Word>,
    more: bool,
}

impl Entry {
    /// The entry `line`, its words split as the manifest splits them (`command_holds`).
    pub fn of(line: &str) -> Self {
        let mut words: Vec<Word> = line.split_whitespace().map(Word::of).collect();
        let more = words.len() > 1 && words.last() == Some(&Word::Literal("*".into()));
        if more {
            words.pop();
        }
        // Never a bare `*`, nor a joker for a program: it would be any program.
        if !matches!(words.first(), Some(Word::Literal(w)) if w != "*") {
            words = vec![Word::Never];
        }
        Self { words, more }
    }

    /// The words to run for `asked`, its jokers checked and its paths replaced; or how near it came and why not: the
    /// words it matched, twice, and one more when it has the length asked.
    fn check(&self, asked: &[String], held: &Held) -> Result<Matched, (usize, String)> {
        let n = self.words.len();
        let fits = usize::from(asked.len() == n || (self.more && asked.len() > n));
        let mut out = Matched::default();
        for (i, (word, a)) in self.words.iter().zip(asked).enumerate() {
            let at = |why: String| {
                (
                    2 * i + fits,
                    format!("word {} ({a:?}) is no {}: {why}", i + 1, word.said()),
                )
            };
            match word {
                Word::Literal(l) if l == a => out.words.push(a.clone()),
                Word::Literal(_) | Word::Never => {
                    return Err((
                        2 * i + fits,
                        format!("word {} is {a:?}, not {:?}", i + 1, word.said()),
                    ))
                }
                _ if a == "--" || a.starts_with('-') => return Err(at("an option".into())),
                Word::Int => {
                    whole(a).map_err(at)?;
                    out.words.push(a.clone());
                }
                Word::Port => {
                    let n = whole(a).map_err(at)?;
                    if !(1..=65_535).contains(&n) {
                        return Err(at("out of 1 to 65535".into()));
                    }
                    out.words.push(a.clone());
                }
                Word::Name => {
                    name(a).map_err(at)?;
                    out.words.push(a.clone());
                }
                Word::Pid => {
                    let n = whole(a).map_err(at)?;
                    let pid = u32::try_from(n).map_err(|_| at("too big".into()))?;
                    if !(held.pids)(pid) {
                        return Err(at("no program it runs is alive with this pid".into()));
                    }
                    out.pids.push(pid);
                    out.words.push(a.clone());
                }
                Word::Temp | Word::Plugin => {
                    let (root, hidden) = if *word == Word::Temp {
                        (held.temp, true)
                    } else {
                        (held.plugin, false)
                    };
                    out.words
                        .push(super::files::file_beneath(root, a, hidden).map_err(at)?);
                }
            }
        }
        if asked.len() < n || (!self.more && asked.len() > n) {
            let entry = if self.more { "at least " } else { "" };
            return Err((
                2 * asked.len().min(n),
                format!("{} words where the entry has {entry}{n}", asked.len()),
            ));
        }
        // What a last `*` lets follow, as it is.
        out.words.extend(asked[n..].iter().cloned());
        Ok(out)
    }
}

/// A whole number of `{int}`: `0|[1-9][0-9]{0,9}`, 4 294 967 295 at most; no sign, no leading zero, no hexadecimal.
fn whole(word: &str) -> Result<u64, String> {
    let digits = word.bytes().all(|b| b.is_ascii_digit());
    if !digits || word.is_empty() || word.len() > 10 || (word.len() > 1 && word.starts_with('0')) {
        return Err(
            "not a whole number of at most 10 digits, without sign nor leading zero".into(),
        );
    }
    let n: u64 = word.parse().map_err(|_| "not a whole number".to_string())?;
    if n > u64::from(u32::MAX) {
        return Err("over 4294967295".into());
    }
    Ok(n)
}

/// A `{name}`: `[A-Za-z0-9_][A-Za-z0-9_.-]{0,63}`, neither `.` nor `..` nor a device of Windows. Nothing a shell, the CRT
/// of Windows or a program that expands its arguments again would cut or read: no `/`, `\`, `:`, `=`, `$`, `%`, quote…
fn name(word: &str) -> Result<(), String> {
    let bytes = word.as_bytes();
    let ok = (1..=64).contains(&bytes.len())
        && (bytes[0].is_ascii_alphanumeric() || bytes[0] == b'_')
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'));
    if !ok {
        return Err(
            "not 1 to 64 letters, digits, _ . - that start with a letter, a digit or _".into(),
        );
    }
    if super::files::windows_device(word) {
        return Err("a device of Windows".into());
    }
    Ok(())
}

/// What the host knows to judge a joker: the pids of the programs alive this instance runs, and its two folders.
pub struct Held<'a> {
    pub pids: &'a dyn Fn(u32) -> bool,
    pub temp: &'a std::path::Path,
    pub plugin: &'a std::path::Path,
}

/// The words to run, once an entry matched: its paths replaced by the host. And the pids it names, checked again when it
/// starts.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Matched {
    pub words: Vec<String>,
    pub pids: Vec<u32>,
}

/// What `asked` (the command and its arguments) runs, when it is an entry of `commands` (§5.3 of
/// docs/PLUGINS-CONTRACT-STUDY.md): its literal words byte for byte, each joker one whole word of its form, never an
/// option nor `--`, the entry of the same length but for a last `*`, after which any words follow. Or why not, said of
/// the entry that matched furthest.
pub fn allowed(commands: &[Entry], asked: &[String], held: &Held) -> Result<Matched, String> {
    let mut best: Option<(usize, String)> = None;
    for entry in commands {
        match entry.check(asked, held) {
            Ok(m) => return Ok(m),
            Err((far, why)) => {
                // The program at least: an entry of another tells nothing.
                if far >= 2 && best.as_ref().is_none_or(|b| far > b.0) {
                    best = Some((far, why));
                }
            }
        }
    }
    let shown = asked.join(" ");
    Err(match best {
        Some((_, why)) => format!("{shown} is not in permissions.commands: {why}"),
        None => format!("{shown} is not in permissions.commands"),
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
    /// Its pid, what `{pid}` admits (ESC-528): taken off as soon as the host sees its end, before anything lets the
    /// system give it again (the host holds the child: under Windows its handle, under Unix its zombie, until then).
    pid: Option<u32>,
}

/// What the journal is told: its kind, its line.
pub type Told = Arc<dyn Fn(Kind, &str) + Send + Sync>;
/// The folder of a project the plugin may run in, or why not.
pub type Projects = Arc<dyn Fn(&str) -> Result<PathBuf, String> + Send + Sync>;

/// The programs of an instance of a plugin.
pub struct Procs {
    commands: Vec<Entry>,
    /// Its folder, the root of `{path:plugin}`.
    plugin: PathBuf,
    /// The variables it declares (`permissions.env`, ESC-424).
    env: Option<Vec<String>>,
    /// Its manifest is of Escouade's own hand (`builtin::first_hand`).
    first_hand: bool,
    temp: PathBuf,
    agents: super::files::Agents,
    projects: Projects,
    agent_env: Vec<(String, String)>,
    back: Back,
    told: Told,
    live: Arc<Mutex<HashMap<u64, Live>>>,
    /// How many may live at once ([`at_once`]).
    at_once: usize,
    next: AtomicU64,
    /// Once the instance ended, no more.
    over: Arc<std::sync::atomic::AtomicBool>,
}

/// What a `Procs` is made of.
pub struct ProcsOf {
    pub commands: Vec<String>,
    pub env: Option<Vec<String>>,
    pub first_hand: bool,
    pub temp: PathBuf,
    /// Its folder (`{path:plugin}`).
    pub plugin: PathBuf,
    pub agents: super::files::Agents,
    pub projects: Projects,
    pub agent_env: Vec<(String, String)>,
    pub back: mpsc::WeakUnboundedSender<Frame>,
    pub told: Told,
    /// What its manifest declares of programs at once (`limits.processes`): [`at_once`] bounds it.
    pub processes: Option<u64>,
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
            commands: of.commands.iter().map(|c| Entry::of(c)).collect(),
            plugin: of.plugin,
            env: of.env,
            first_hand: of.first_hand,
            temp: of.temp,
            agents: of.agents,
            projects: of.projects,
            agent_env: of.agent_env,
            back: Back(of.back),
            told: of.told,
            live: Arc::default(),
            at_once: at_once(of.processes),
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

    /// `{ command, args, dir?, agent?, project?, env? }` → `{ proc, pid }`.
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
        let alive: Vec<u32> = self.live.lock().values().filter_map(|l| l.pid).collect();
        let held = Held {
            pids: &|pid| alive.contains(&pid),
            temp: &self.temp,
            plugin: &self.plugin,
        };
        let matched = allowed(&self.commands, &words, &held).map_err(|why| (REFUSED, why))?;
        // The words it runs: its paths are the host's.
        let words = matched.words;
        let shown = words.join(" ");
        let (dir, root) = self.folder(params)?;
        let mut named = Vec::new();
        if let Some(env) = params.get("env").filter(|e| !e.is_null()) {
            let env = env
                .as_object()
                .ok_or_else(|| invalid("env is an object of texts"))?;
            for (k, v) in env {
                if let Some(why) = var_refused(self.env.as_deref(), k, self.first_hand) {
                    return Err((REFUSED, why));
                }
                let v = v
                    .as_str()
                    .ok_or_else(|| invalid("env is an object of texts"))?;
                named.push((k.clone(), v.to_string()));
            }
        }
        if self.live.lock().len() >= self.at_once {
            return Err((REFUSED, self.full()));
        }
        if self.over.load(Ordering::Acquire) {
            return Err((FAILED, "its instance has ended".into()));
        }
        let program = locate(command)
            .ok_or_else(|| (FAILED, format!("no program {command} on this machine")))?;
        // From here to its start, what it names is held: a `{pid}` is checked again, and no end takes it off meanwhile.
        let mut live = self.live.lock();
        if live.len() >= self.at_once {
            return Err((REFUSED, self.full()));
        }
        if let Some(gone) = matched
            .pids
            .iter()
            .find(|p| !live.values().any(|l| l.pid == Some(**p)))
        {
            return Err((
                REFUSED,
                format!("{shown}: no program it runs is alive with the pid {gone} any more"),
            ));
        }
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
        let pid = child.id();
        let stdin = child.stdin.take().expect("piped");
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        let waiting = Arc::new(AtomicUsize::new(0));
        let (input, inputs) = mpsc::unbounded_channel();
        let (kill, killed) = oneshot::channel();
        live.insert(
            id,
            Live {
                input,
                waiting: waiting.clone(),
                kill: Some(kill),
                job: job.clone(),
                pid,
            },
        );
        drop(live);
        (self.told)(Kind::Message, &format!("proc/spawn: ran {shown} in {root}"));
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
        Ok(json!({ "proc": id, "pid": pid }))
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

    /// Why one more is refused: its own limit, said in the answer and the journal (ESC-529).
    fn full(&self) -> String {
        let n = self.at_once;
        let programs = if n == 1 { "program" } else { "programs" };
        format!("{n} {programs} at once at most for this plugin (limits.processes)")
    }

    /// The commands under way.
    pub(crate) fn running(&self) -> usize {
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
    // Its pid is no `{pid}` any more, before what it left running is waited for.
    if let Some(l) = w.live.lock().get_mut(&w.id) {
        l.pid = None;
    }
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
pub(crate) mod tests {
    use super::*;
    use crate::files::AgentPlace;
    use std::time::Instant;
    use tokio::sync::mpsc::UnboundedReceiver;

    pub(crate) fn words(c: &str) -> Vec<String> {
        c.split_whitespace().map(str::to_string).collect()
    }

    fn commands(list: &[&str]) -> Vec<Entry> {
        list.iter().map(|c| Entry::of(c)).collect()
    }

    /// Whether `asked` runs under `c`, for a host that runs nothing and has no folder.
    fn runs(c: &[Entry], asked: &[String]) -> bool {
        let nowhere = std::path::Path::new("");
        let held = Held {
            pids: &|_| false,
            temp: nowhere,
            plugin: nowhere,
        };
        allowed(c, asked, &held).is_ok()
    }

    #[test]
    fn a_command_is_an_entry_word_for_word_and_only_a_star_lets_more_follow() {
        let c = commands(&["isola ls --json", "isola down", "isola logs *", "*"]);
        assert!(runs(&c, &words("isola ls --json")));
        assert!(runs(&c, &words("isola down")));
        assert!(runs(&c, &words("isola logs")));
        assert!(runs(&c, &words("isola logs web --tail 10")));
        assert!(!runs(&c, &words("isola ls")));
        assert!(!runs(&c, &words("isola ls --json --all")));
        assert!(!runs(&c, &words("isola down now")));
        assert!(!runs(&c, &words("isola destroy")));
        assert!(!runs(&c, &words("ISOLA down")));
        // A bare `*` is no program.
        assert!(!runs(&c, &words("rm -rf x")));
        assert!(!runs(&c, &words("*")));
        assert!(!runs(&c, &[]));
        // One word, not a line split again by a shell.
        assert!(!runs(&c, &["isola down; rm -rf x".to_string()]));
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

    /// What the plugin of the bed declares (`permissions.env`): the variables of git's configuration among them, which
    /// Escouade refuses anyone's plugin even declared.
    const DECLARED: [&str; 7] = [
        "GIT_AUTHOR_NAME",
        "GIT_AUTHOR_EMAIL",
        "MA_VAR",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_KEY_0",
        "GIT_CONFIG_VALUE_0",
        "HTTPS_PROXY",
    ];

    /// SEC-05 (audit 2026-11): a variable passes declared alone, and none of the families that would make the program
    /// declared run something else, even declared; a plugin of Escouade's own hand is held to what it declares.
    #[test]
    fn a_variable_passes_declared_and_out_of_the_refused_families() {
        let declared: Vec<String> = DECLARED.iter().map(|v| v.to_string()).collect();
        let d = Some(declared.as_slice());
        assert_eq!(var_refused(d, "MA_VAR", false), None);
        assert_eq!(var_refused(d, "GIT_AUTHOR_NAME", false), None);
        let not_declared = var_refused(d, "AUTRE_VAR", false).unwrap();
        assert!(
            not_declared.contains("not in permissions.env"),
            "{not_declared}"
        );
        // Nothing declared, nothing passes.
        assert!(var_refused(None, "MA_VAR", false).is_some());
        assert!(var_refused(Some(&[]), "MA_VAR", false).is_some());
        // Those of the audit, each declared: all refused.
        let audit = [
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_KEY_0",
            "GIT_CONFIG_VALUE_0",
            "GIT_CONFIG_GLOBAL",
            "GIT_SSH_COMMAND",
            "GIT_EXEC_PATH",
            "GIT_EXTERNAL_DIFF",
            "GIT_ASKPASS",
            "GIT_SSL_NO_VERIFY",
            "PYTHONPATH",
            "PYTHONSTARTUP",
            "RUBYOPT",
            "PERL5OPT",
            "JAVA_TOOL_OPTIONS",
            "_JAVA_OPTIONS",
            "NODE_OPTIONS",
            "NODE_PATH",
            "NODE_EXTRA_CA_CERTS",
            "NODE_TLS_REJECT_UNAUTHORIZED",
            "npm_config_script_shell",
            "LD_PRELOAD",
            "DYLD_INSERT_LIBRARIES",
            "HOME",
            "USERPROFILE",
            "APPDATA",
            "XDG_CONFIG_HOME",
            "HTTPS_PROXY",
            "http_proxy",
            "ALL_PROXY",
            "SSL_CERT_FILE",
            "PATH",
            "EDITOR",
            "PAGER",
            "BROWSER",
            "ESCOUADE_MCP_TOKEN",
        ];
        let all: Vec<String> = audit.iter().map(|v| v.to_string()).collect();
        for v in audit {
            let why = var_refused(Some(&all), v, false).unwrap_or_else(|| panic!("{v} passes"));
            assert!(why.contains("refuses a plugin"), "{v}: {why}");
        }
        // Escouade's own hand: what it declares, no more; never the core's list.
        let own = vec!["HOME".to_string()];
        assert_eq!(var_refused(Some(&own), "HOME", true), None);
        assert!(var_refused(Some(&own), "MA_VAR", true).is_some());
        assert!(var_refused(Some(&own), "PATH", true).is_some());
        // Of a version from before `permissions.env`: as it was.
        assert_eq!(var_refused(None, "NO_COLOR", true), None);
        assert!(var_refused(None, "ESCOUADE_X", true).is_some());
    }

    /// SEC-05, the audit's trial: a plugin allowed `git status` and nothing else asks git's configuration by its
    /// variables, declared; it is refused, and git does not run.
    #[tokio::test]
    async fn git_status_with_a_configuration_by_variables_is_refused_even_declared() {
        let b = bed(&["git status"]);
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(b.base.join("wt"))
            .status()
            .unwrap();
        let marker = b.base.join("wt").join("marker.txt");
        b.refused(json!({
            "command": "git", "args": ["status"], "dir": "agent", "agent": "a",
            "env": {
                "GIT_CONFIG_COUNT": "1",
                "GIT_CONFIG_KEY_0": "core.fsmonitor",
                "GIT_CONFIG_VALUE_0": "echo marker > marker.txt; false",
            },
        }));
        // Not declared either.
        b.refused(json!({ "command": "git", "args": ["status"], "env": { "AUTRE_VAR": "1" } }));
        assert_eq!(b.procs.running(), 0);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!marker.exists());
        let told = b.told.lock().join("\n");
        assert!(
            told.contains("GIT_CONFIG_COUNT") && told.contains("AUTRE_VAR"),
            "{told}"
        );
    }

    pub(crate) struct Bed {
        pub(crate) procs: Procs,
        pub(crate) rx: UnboundedReceiver<Frame>,
        _tx: mpsc::UnboundedSender<Frame>,
        pub(crate) told: Arc<Mutex<Vec<String>>>,
        pub(crate) base: PathBuf,
    }

    pub(crate) fn bed(list: &[&str]) -> Bed {
        bed_of(list, None)
    }

    /// The same, for a plugin that declares `processes` programs at once.
    pub(crate) fn bed_of(list: &[&str], processes: Option<u64>) -> Bed {
        let (tx, rx) = mpsc::unbounded_channel();
        let base =
            std::env::temp_dir().join(format!("plugin-procs-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(base.join("wt")).unwrap();
        std::fs::create_dir_all(base.join("project")).unwrap();
        std::fs::create_dir_all(base.join("plugin")).unwrap();
        let told: Arc<Mutex<Vec<String>>> = Arc::default();
        let t = told.clone();
        let agents: crate::files::Agents = Arc::default();
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
            env: Some(DECLARED.iter().map(|v| v.to_string()).collect()),
            first_hand: false,
            temp: base.join("temp"),
            plugin: base.join("plugin"),
            agents,
            projects: Arc::new(move |p| match p {
                "open" => Ok(b.join("project")),
                _ => Err(format!("the project {p} is private")),
            }),
            agent_env: Vec::new(),
            back: tx.downgrade(),
            told: Arc::new(move |_, l| t.lock().push(l.to_string())),
            processes,
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
        pub(crate) fn refused(&self, params: Value) {
            let r = self.procs.serve("proc/spawn", &params);
            assert_eq!(
                r.as_ref().err().map(|e| e.0),
                Some(REFUSED),
                "{params}: {r:?}"
            );
        }

        pub(crate) fn spawn(&self, params: Value) -> u64 {
            self.procs.serve("proc/spawn", &params).unwrap()["proc"]
                .as_u64()
                .unwrap()
        }

        /// What comes until the end of `id`: what it wrote on `stream`, and its code.
        pub(crate) async fn run(&mut self, id: u64, stream: &str) -> (String, Value) {
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

    #[test]
    fn the_limit_is_4_without_a_word_what_is_declared_else_and_never_more_than_16() {
        assert_eq!(at_once(None), 4);
        assert_eq!(at_once(Some(1)), 1);
        assert_eq!(at_once(Some(4)), 4);
        assert_eq!(at_once(Some(8)), 8);
        assert_eq!(at_once(Some(16)), 16);
        // The reader refuses these; the host holds even so.
        assert_eq!(at_once(Some(0)), 1);
        assert_eq!(at_once(Some(17)), 16);
        assert_eq!(at_once(Some(u64::MAX)), 16);
    }

    /// `n` programs that live until killed, in a repository of the bed.
    fn batches(b: &Bed, n: usize) -> (Value, Vec<u64>) {
        let batch = json!({ "command": "git", "args": ["cat-file", "--batch"], "dir": "agent", "agent": "a" });
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(b.base.join("wt"))
            .status()
            .unwrap();
        let ids = (0..n).map(|_| b.spawn(batch.clone())).collect();
        (batch, ids)
    }

    /// Ends what `b` runs and waits for each end.
    async fn end_all(b: &mut Bed, mut left: Vec<u64>) {
        b.procs.end();
        let end = Instant::now() + Duration::from_secs(30);
        while !left.is_empty() {
            let got = tokio::time::timeout_at(end.into(), b.rx.recv()).await;
            let (bytes, written) = got.expect("in time").expect("open");
            if let Some(w) = written {
                let _ = w.send(());
            }
            let v: Value = serde_json::from_slice(&bytes).unwrap();
            if v["method"] == "proc/exit" {
                left.retain(|i| v["params"]["proc"] != *i);
            }
        }
    }

    #[tokio::test]
    async fn the_limit_by_default_is_said_when_a_fifth_is_refused() {
        let mut b = bed(&["git cat-file --batch"]);
        let (batch, ids) = batches(&b, 4);
        let (code, message) = b.procs.serve("proc/spawn", &batch).unwrap_err();
        assert_eq!(code, REFUSED);
        assert_eq!(
            message,
            "4 programs at once at most for this plugin (limits.processes)"
        );
        let told = b.told.lock().join("\n");
        assert!(
            told.contains("proc/spawn: refused, 4 programs at once at most for this plugin"),
            "{told}"
        );
        end_all(&mut b, ids).await;
    }

    #[tokio::test]
    async fn a_declared_limit_holds_its_own_number() {
        let mut b = bed_of(&["git cat-file --batch"], Some(6));
        let (batch, ids) = batches(&b, 6);
        assert_eq!(b.procs.running(), 6);
        let (_, message) = b.procs.serve("proc/spawn", &batch).unwrap_err();
        assert!(
            message.starts_with("6 programs at once at most"),
            "{message}"
        );
        end_all(&mut b, ids).await;
        // A limit below 4 holds too.
        let mut b = bed_of(&["git cat-file --batch"], Some(1));
        let (batch, ids) = batches(&b, 1);
        let (_, message) = b.procs.serve("proc/spawn", &batch).unwrap_err();
        assert_eq!(
            message,
            "1 program at once at most for this plugin (limits.processes)"
        );
        end_all(&mut b, ids).await;
    }

    #[tokio::test]
    async fn sixteen_at_most_the_seventeenth_is_refused_and_an_end_gives_its_place_back() {
        // What the manifest says beyond 16 is never given: the host stops at 16.
        let mut b = bed_of(&["git cat-file --batch"], Some(40));
        let (batch, mut ids) = batches(&b, AT_ONCE_MAX);
        let (code, message) = b.procs.serve("proc/spawn", &batch).unwrap_err();
        assert_eq!(code, REFUSED);
        assert!(
            message.starts_with("16 programs at once at most"),
            "{message}"
        );
        b.procs
            .serve("proc/kill", &json!({ "proc": ids[0] }))
            .unwrap();
        let (_, code) = b.run(ids[0], "stdout").await;
        assert_ne!(code, 0);
        assert_eq!(b.procs.running(), AT_ONCE_MAX - 1);
        ids.remove(0);
        ids.push(b.spawn(batch.clone()));
        assert_eq!(b.procs.running(), AT_ONCE_MAX);
        b.refused(batch);
        end_all(&mut b, ids).await;
        assert_eq!(b.procs.running(), 0);
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
