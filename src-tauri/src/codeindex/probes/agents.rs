//! The bench of the real agents (ESC-299, IDX-4): does the index spare a real agent its searches,
//! its readings and its tokens, without it being wrong more often? `docs/CODEINDEX-PROBES.md`
//! « Avec de vrais agents » says what it found.
//!
//! `cargo run --release --example codeindex_agents -- <command>`, from `src-tauri/`:
//! - `run [--only <id,…>] [--modes <m,…>] [--passes <n>] [--out <dir>] [--root <dir>] [--claude
//!   <program>]`: the tasks of `docs/codeindex-agents.json`, each played by a real Claude Code
//!   with no window (`claude -p --output-format stream-json`) in each mode, so many passes. A
//!   mode has its own copy of the repository at the bench's commit (`git clone --shared`, out of
//!   any worktree, put back after each run) and its own index of it; the runs already in
//!   `runs.jsonl` are not played again. Then `report`.
//! - `report [--out <dir>] [--runs <file>]`: `results.md` and `results.json` from `runs.jsonl` (the
//!   out folder's, or the one given: `docs/codeindex-agents-runs.jsonl`, ESC-299's), the
//!   questions checked again by the checks as they are now.
//! - `check [--root <dir>]`: each task's check tried on its good answer and on its bad one.
//! - `serve --dir <copy> --data <dir> [--log <file>]` and `hook [--log <file>]`: what the agents
//!   are given (`agents_mcp`).
//!
//! The modes: `grep` (Grep, Glob and Read, no index), `index-hook` (the index's MCP tools too,
//! the rule of the agents' instructions and the ESC-253 hook), `index-only` (the index, Glob
//! and Read, no Grep) and `tokensave` (as `index-hook`, tokensave's server and the rules its
//! `install` writes in place of Escouade's, no hook; `--tokensave <exe>`). The agent has Edit and
//! Write for a change, never a terminal.
//!
//! tokensave is run by ESC-304's rules: its worldwide counter turned off first (and checked off),
//! `TOKENSAVE_UPDATE_CHECK=off`, its copy without `.git` indexed by `init --no-git-hook` in that
//! copy only; never `install`, never a hook.
//!
//! Each Claude Code is launched as it is installed, on the default account of the machine: no
//! file of its connection is touched nor read, no global setting changed. What a run needs comes
//! by its own flags, from its folder: `--settings`, `--mcp-config` with `--strict-mcp-config`,
//! `--setting-sources project` (the user's plugins and hooks left out), `--tools`,
//! `--allowedTools` with `--permission-mode dontAsk`, `--no-session-persistence`.

use super::agents_mcp::{self, LANG};
use super::{git, remove_all};
use anyhow::{anyhow, bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bench {
    /// The commit the copies are at, the one the checks were written for.
    pub commit: String,
    pub model: String,
    pub effort: String,
    pub passes: u32,
    /// The most a run may take, in seconds.
    pub timeout_s: u64,
    /// Added to the prompt of a question, then of a change.
    pub answer_rule: String,
    pub change_rule: String,
    /// The rules `tokensave install` would write for Claude Code, given to the agents of the
    /// mode `tokensave` as Escouade's rule is to those of its index.
    pub tokensave_rule: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    /// Where it comes from: a probe « tâche », a ticket.
    pub from: String,
    pub prompt: String,
    /// A question: what its answer must say, each need by one of its alternatives (case and the
    /// slashes of a path aside).
    #[serde(default)]
    pub answer: Vec<Vec<String>>,
    /// A change: what it must leave in the code.
    pub change: Option<Change>,
    /// The checks tried by hand: an answer that passes and one that does not (a question)…
    pub good: Option<String>,
    pub bad: Option<String>,
    /// …or an edit that passes and one that does not (a change).
    pub good_edit: Option<Edit>,
    pub bad_edit: Option<Edit>,
    /// Why the check was widened after the runs' answers were read, when it was.
    pub widened: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub file: String,
    /// The line the symbol begins with (a part of it): the change must be within the symbol.
    pub scope: String,
    /// Each need is one of its alternatives on a line of the symbol, as written.
    pub added: Vec<Vec<String>>,
    /// Texts no line of the symbol may still have.
    #[serde(default)]
    pub gone: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Edit {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    Grep,
    IndexHook,
    IndexOnly,
    /// tokensave's MCP server in place of Escouade's (IDX-5's question, with real agents).
    Tokensave,
}

impl Mode {
    pub const ALL: [Mode; 4] = [
        Mode::Grep,
        Mode::IndexHook,
        Mode::IndexOnly,
        Mode::Tokensave,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Mode::Grep => "grep",
            Mode::IndexHook => "index-hook",
            Mode::IndexOnly => "index-only",
            Mode::Tokensave => "tokensave",
        }
    }

    fn of(s: &str) -> Result<Mode> {
        Mode::ALL
            .into_iter()
            .find(|m| m.name() == s)
            .ok_or_else(|| anyhow!("unknown mode {s} (grep, index-hook, index-only, tokensave)"))
    }

    /// Whether the agent has Escouade's index.
    fn escouade(self) -> bool {
        matches!(self, Mode::IndexHook | Mode::IndexOnly)
    }

    /// Claude Code's own tools of the mode.
    fn tools(self) -> &'static [&'static str] {
        match self {
            Mode::IndexOnly => &["Read", "Glob", "Edit", "Write"],
            _ => &["Read", "Glob", "Grep", "Edit", "Write"],
        }
    }
}

const INDEX_TOOLS: [&str; 6] = [
    "index_search",
    "index_context",
    "index_body",
    "index_callers",
    "index_callees",
    "index_impact",
];
const MCP_PREFIX: &str = "mcp__escouade__";
/// What a run of tokensave's server is told: no check of a new version.
const TOKENSAVE_ENV: (&str, &str) = ("TOKENSAVE_UPDATE_CHECK", "off");

// ---------------------------------------------------------------------------------------------
// The checks.

/// A path as an agent may write it (`src-tauri\src\core.rs`), as the needs write it.
fn normalized(text: &str) -> String {
    text.replace('\\', "/").to_lowercase()
}

/// Whether `text` says every need, each by one of its alternatives.
pub fn answer_ok(needs: &[Vec<String>], text: &str) -> bool {
    let text = normalized(text);
    !needs.is_empty()
        && needs
            .iter()
            .all(|alts| alts.iter().any(|a| text.contains(&normalized(a))))
}

/// The lines (from 0, both ends in) of the symbol that begins on the first line holding `scope`:
/// to the brace that closes its first, or that line alone when it ends before one opens (`const
/// X: … = …;`).
pub fn scope_lines(text: &str, scope: &str) -> Option<(usize, usize)> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| l.contains(scope))?;
    let mut depth = 0i32;
    let mut opened = false;
    for (i, line) in lines.iter().enumerate().skip(start) {
        for c in line.chars() {
            match c {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth -= 1,
                ';' if !opened && depth == 0 => return Some((start, i)),
                _ => {}
            }
        }
        if opened && depth <= 0 {
            return Some((start, i));
        }
    }
    Some((start, lines.len().saturating_sub(1)))
}

/// Whether the file's text after a change has what `c` wants in its symbol; else why not.
pub fn change_ok(c: &Change, text: &str) -> Result<(), String> {
    let Some((from, to)) = scope_lines(text, &c.scope) else {
        return Err(format!("« {} » is gone from {}", c.scope, c.file));
    };
    let body: Vec<&str> = text.lines().skip(from).take(to - from + 1).collect();
    for alts in &c.added {
        if !body
            .iter()
            .any(|l| alts.iter().any(|a| l.contains(a.as_str())))
        {
            return Err(format!("none of {alts:?} in « {} »", c.scope));
        }
    }
    for g in &c.gone {
        if body.iter().any(|l| l.contains(g.as_str())) {
            return Err(format!("« {g} » still in « {} »", c.scope));
        }
    }
    Ok(())
}

/// The check of a task after its run in `copy`, given the agent's last words: right, or why not.
fn checked(t: &Task, copy: &Path, result: &str) -> Result<(), String> {
    match &t.change {
        Some(c) => {
            let text = std::fs::read_to_string(copy.join(&c.file)).map_err(|e| e.to_string())?;
            change_ok(c, &text)
        }
        None if answer_ok(&t.answer, result) => Ok(()),
        None => Err("the answer misses what was asked".into()),
    }
}

// ---------------------------------------------------------------------------------------------
// What a run did, read from its stream.

/// What a run cost and did, read from Claude Code's stream (`stream-json`) and the logs of the
/// bench's server and hook.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Measure {
    /// Each tool's calls, by its name (an MCP tool's without its prefix).
    pub calls: BTreeMap<String, u32>,
    /// What each tool's answers weighed, in tokens (`tokens_of` their text), by its name.
    pub result_tokens: BTreeMap<String, u64>,
    /// Of the index's (the MCP tools').
    pub index_tokens: u64,
    pub tool_calls: u32,
    pub index_calls: u32,
    /// Grep after the first call to the index: a search the index did not spare.
    pub regreps: u32,
    /// Glob and Read after the first call to the index.
    pub globs_after: u32,
    pub reads_after: u32,
    /// Read of a file already read in the run.
    pub rereads: u32,
    pub input: u64,
    pub cache_write: u64,
    pub cache_read: u64,
    pub output: u64,
    pub cost_usd: f64,
    /// Claude Code's own count of the run, and its turns.
    pub ms: u64,
    pub turns: u32,
    /// The agent's last words.
    pub result: String,
    /// Whether the index's server was connected at the start (modes with the index).
    pub mcp: String,
    pub denials: u32,
    pub stop: String,
}

impl Measure {
    pub fn tokens(&self) -> u64 {
        self.input + self.cache_write + self.cache_read + self.output
    }
}

/// The name of a tool, an MCP tool's without its prefix (`mcp__<server>__`).
fn short(name: &str) -> String {
    match name.strip_prefix("mcp__").and_then(|n| n.split_once("__")) {
        Some((_, tool)) => tool.to_string(),
        None => name.to_string(),
    }
}

/// The text of a tool's answer: a string, or its blocks of text.
fn result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join(
                "
",
            ),
        _ => String::new(),
    }
}

pub fn measure(stream: &str) -> Measure {
    let mut m = Measure::default();
    let mut read: BTreeSet<String> = BTreeSet::new();
    let mut after_index = false;
    // A call's id: its tool, and whether it is the index's.
    let mut ids: BTreeMap<String, (String, bool)> = BTreeMap::new();
    for line in stream.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match v["type"].as_str().unwrap_or_default() {
            "system" if v["subtype"] == "init" => {
                m.mcp = v["mcp_servers"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|s| {
                        format!(
                            "{}:{}",
                            s["name"].as_str().unwrap_or("?"),
                            s["status"].as_str().unwrap_or("?")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
            }
            "assistant" => {
                for b in v["message"]["content"].as_array().into_iter().flatten() {
                    if b["type"] != "tool_use" {
                        continue;
                    }
                    let raw = b["name"].as_str().unwrap_or_default();
                    let name = short(raw);
                    let index = raw.starts_with("mcp__");
                    ids.insert(
                        b["id"].as_str().unwrap_or_default().to_string(),
                        (name.clone(), index),
                    );
                    *m.calls.entry(name.clone()).or_default() += 1;
                    m.tool_calls += 1;
                    if index {
                        m.index_calls += 1;
                        after_index = true;
                        continue;
                    }
                    match name.as_str() {
                        "Grep" if after_index => m.regreps += 1,
                        "Glob" if after_index => m.globs_after += 1,
                        "Read" => {
                            if after_index {
                                m.reads_after += 1;
                            }
                            let path =
                                normalized(b["input"]["file_path"].as_str().unwrap_or_default());
                            if !read.insert(path) {
                                m.rereads += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
            "user" => {
                for b in v["message"]["content"].as_array().into_iter().flatten() {
                    if b["type"] != "tool_result" {
                        continue;
                    }
                    let Some((name, index)) =
                        ids.get(b["tool_use_id"].as_str().unwrap_or_default())
                    else {
                        continue;
                    };
                    let tokens = super::super::tokens_of(result_text(&b["content"]).len() as u64);
                    *m.result_tokens.entry(name.clone()).or_default() += tokens;
                    if *index {
                        m.index_tokens += tokens;
                    }
                }
            }
            "result" => {
                m.result = v["result"].as_str().unwrap_or_default().to_string();
                m.ms = v["duration_ms"].as_u64().unwrap_or_default();
                m.turns = v["num_turns"].as_u64().unwrap_or_default() as u32;
                m.cost_usd = v["total_cost_usd"].as_f64().unwrap_or_default();
                m.stop = v["subtype"].as_str().unwrap_or_default().to_string();
                m.denials = v["permission_denials"]
                    .as_array()
                    .map_or(0, |a| a.len() as u32);
                // Every model of the run (a side model too), else the main one's.
                let models = v["modelUsage"].as_object().filter(|o| !o.is_empty());
                if let Some(models) = models {
                    for u in models.values() {
                        let n = |k: &str| u[k].as_u64().unwrap_or_default();
                        m.input += n("inputTokens");
                        m.output += n("outputTokens");
                        m.cache_read += n("cacheReadInputTokens");
                        m.cache_write += n("cacheCreationInputTokens");
                    }
                } else {
                    let u = &v["usage"];
                    let n = |k: &str| u[k].as_u64().unwrap_or_default();
                    m.input = n("input_tokens");
                    m.output = n("output_tokens");
                    m.cache_read = n("cache_read_input_tokens");
                    m.cache_write = n("cache_creation_input_tokens");
                }
            }
            _ => {}
        }
    }
    m
}

/// One run: a task, in a mode, at a pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Run {
    pub task: String,
    pub mode: String,
    pub pass: u32,
    pub ok: bool,
    pub why: String,
    /// Wall clock, from the launch to the end of the process.
    pub wall_ms: u64,
    /// The searches the hook saw, and those it answered.
    pub hook_events: u32,
    pub hooks: u32,
    pub changed: Vec<String>,
    #[serde(flatten)]
    pub m: Measure,
}

// ---------------------------------------------------------------------------------------------
// The command line.

pub fn cli(args: Vec<String>) -> i32 {
    let (command, rest) = match args.split_first() {
        Some((c, rest)) => (c.as_str(), rest.to_vec()),
        None => ("", Vec::new()),
    };
    let done = match command {
        "serve" => serve_cli(&rest),
        "hook" => hook_cli(&rest),
        "run" => run_cli(&rest),
        "report" => report_cli(&rest),
        "check" => check_cli(&rest),
        _ => {
            eprintln!(
                "codeindex_agents run [--only <id,…>] [--modes grep,index-hook,index-only] [--passes <n>] [--out <dir>] [--root <dir>] [--claude <program>]\n                 report [--out <dir>] [--runs <runs.jsonl>] | check [--root <dir>] [--mode <m>]\n                 serve --dir <copy> --data <dir> [--log <file>] | hook [--log <file>]"
            );
            return 2;
        }
    };
    match done {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("codeindex_agents: {e:#}");
            1
        }
    }
}

/// The options `--name value` of a command; `--name` with no value is refused.
fn options(args: &[String]) -> Result<BTreeMap<String, String>> {
    let mut o = BTreeMap::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let name = a
            .strip_prefix("--")
            .ok_or_else(|| anyhow!("unexpected argument {a}"))?;
        let value = it.next().ok_or_else(|| anyhow!("{a} wants a value"))?;
        o.insert(name.to_string(), value.clone());
    }
    Ok(o)
}

fn serve_cli(args: &[String]) -> Result<()> {
    let o = options(args)?;
    let dir = PathBuf::from(o.get("dir").ok_or_else(|| anyhow!("--dir"))?);
    let data = PathBuf::from(o.get("data").ok_or_else(|| anyhow!("--data"))?);
    let server = agents_mcp::Server::new(&dir, &data, o.get("log").map(PathBuf::from));
    let stdin = std::io::stdin();
    server.serve(stdin.lock(), std::io::stdout().lock())
}

fn hook_cli(args: &[String]) -> Result<()> {
    let o = options(args)?;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    if let Some(reply) = agents_mcp::hook(&input, o.get("log").map(Path::new)) {
        println!("{reply}");
    }
    Ok(())
}

struct Paths {
    repo: PathBuf,
    bench: Bench,
    out: PathBuf,
    root: PathBuf,
}

fn paths(o: &BTreeMap<String, String>) -> Result<Paths> {
    let here = std::env::current_dir()?;
    let repo = PathBuf::from(git(&here, &["rev-parse", "--show-toplevel"])?);
    let file = o
        .get("bench")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("docs").join("codeindex-agents.json"));
    let bench: Bench = serde_json::from_str(
        &std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?,
    )
    .with_context(|| format!("reading {}", file.display()))?;
    let out = o.get("out").map(PathBuf::from).unwrap_or_else(|| {
        repo.join("src-tauri")
            .join("target")
            .join("codeindex-agents")
    });
    let root = o
        .get("root")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("escouade-codeindex-agents"));
    // The runs' files are named to a Claude Code that works in the copy.
    Ok(Paths {
        repo,
        bench,
        out: here.join(out),
        root: here.join(root),
    })
}

/// The copy a mode works in, at the bench's commit: a clone (`git clone --shared`), or for
/// tokensave, which wants no `.git`, the commit's files (`git archive`) beside an untouched set
/// of them that puts the copy back as it was.
struct Copy {
    dir: PathBuf,
    commit: String,
    pristine: Option<PathBuf>,
}

impl Copy {
    fn of(p: &Paths, mode: Mode) -> Result<Copy> {
        let root = p.root.join(mode.name());
        let dir = root.join("repo");
        let commit = p.bench.commit.clone();
        std::fs::create_dir_all(&root)?;
        if mode != Mode::Tokensave {
            if !dir.join(".git").exists() {
                let _ = remove_all(&dir);
                git(
                    &root,
                    &[
                        "clone",
                        "--quiet",
                        "--shared",
                        "--no-checkout",
                        &p.repo.to_string_lossy(),
                        &dir.to_string_lossy(),
                    ],
                )?;
            }
            let c = Copy {
                dir,
                commit,
                pristine: None,
            };
            c.reset()?;
            return Ok(c);
        }
        let pristine = root.join("pristine");
        if !pristine.join("src-tauri").exists() {
            let _ = remove_all(&pristine);
            std::fs::create_dir_all(&pristine)?;
            let tar = root.join("commit.tar");
            git(
                &p.repo,
                &[
                    "archive",
                    "--format=tar",
                    "-o",
                    &tar.to_string_lossy(),
                    &commit,
                ],
            )?;
            let out = Command::new("tar")
                .arg("-xf")
                .arg(&tar)
                .arg("-C")
                .arg(&pristine)
                .output()
                .context("tar")?;
            if !out.status.success() {
                bail!("tar: {}", String::from_utf8_lossy(&out.stderr).trim());
            }
            let _ = std::fs::remove_file(&tar);
        }
        let c = Copy {
            dir,
            commit,
            pristine: Some(pristine),
        };
        c.reset()?;
        Ok(c)
    }

    /// The files changed since the commit, by their path from the copy's root.
    fn changed(&self) -> Result<Vec<String>> {
        match &self.pristine {
            None => {
                let changed = git(&self.dir, &["diff", "--name-only", "HEAD"])?;
                let added = git(&self.dir, &["ls-files", "--others", "--exclude-standard"])?;
                Ok(changed
                    .lines()
                    .chain(added.lines())
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect())
            }
            Some(pristine) => Ok(plain_changes(&self.dir, pristine)?
                .into_iter()
                .map(|(path, _)| path)
                .collect()),
        }
    }

    /// Puts the copy back at the commit, its index (`.tokensave`) kept.
    fn reset(&self) -> Result<()> {
        let Some(pristine) = &self.pristine else {
            git(
                &self.dir,
                &["checkout", "--quiet", "--force", "--detach", &self.commit],
            )?;
            git(&self.dir, &["clean", "-fdxq"])?;
            return Ok(());
        };
        std::fs::create_dir_all(&self.dir)?;
        for (path, change) in plain_changes(&self.dir, pristine)? {
            let (to, from) = (self.dir.join(&path), pristine.join(&path));
            match change {
                Plain::Added => std::fs::remove_file(&to)?,
                Plain::Changed | Plain::Removed => {
                    if let Some(parent) = to.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(&from, &to)?;
                }
            }
        }
        Ok(())
    }
}

/// How a file of a copy differs from the untouched set.
#[derive(Debug, PartialEq)]
enum Plain {
    Added,
    Changed,
    Removed,
}

/// The files of `dir` under it, by their path with `/` (`.tokensave`, the index, left out).
fn files_under(dir: &Path) -> Result<BTreeSet<String>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeSet<String>) -> std::io::Result<()> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Ok(());
        };
        for e in entries {
            let e = e?;
            let path = e.path();
            if e.file_type()?.is_dir() {
                if e.file_name() != ".tokensave" {
                    walk(root, &path, out)?;
                }
            } else if let Ok(rel) = path.strip_prefix(root) {
                out.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(())
    }
    let mut out = BTreeSet::new();
    walk(dir, dir, &mut out)?;
    Ok(out)
}

/// What differs between a copy and its untouched set.
fn plain_changes(copy: &Path, pristine: &Path) -> Result<Vec<(String, Plain)>> {
    let (now, then) = (files_under(copy)?, files_under(pristine)?);
    let mut changes = Vec::new();
    for path in now.union(&then) {
        let change = match (now.contains(path), then.contains(path)) {
            (true, false) => Plain::Added,
            (false, true) => Plain::Removed,
            _ if std::fs::read(copy.join(path))? != std::fs::read(pristine.join(path))? => {
                Plain::Changed
            }
            _ => continue,
        };
        changes.push((path.clone(), change));
    }
    Ok(changes)
}

/// The home folder of tokensave's settings.
fn tokensave_home() -> Result<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|h| PathBuf::from(h).join(".tokensave"))
        .ok_or_else(|| anyhow!("no home folder"))
}

/// A command of tokensave, its check of a new version off.
fn tokensave(exe: &Path, dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new(exe)
        .current_dir(dir)
        .args(args)
        .env(TOKENSAVE_ENV.0, TOKENSAVE_ENV.1)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("tokensave {}", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "tokensave {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Readies tokensave for the runs, by ESC-304's rules: its worldwide counter turned off before
/// anything else (and checked off in its settings), then the copy indexed, in the copy only, with
/// no git hook. Never `install`, never a hook of Claude Code.
fn tokensave_ready(exe: &Path, copy: &Copy) -> Result<()> {
    tokensave(exe, &copy.dir, &["disable-upload-counter"])?;
    let config = tokensave_home()?.join("config.toml");
    let text = std::fs::read_to_string(&config)
        .with_context(|| format!("reading {}", config.display()))?;
    if !text.lines().any(|l| l.trim() == "upload_enabled = false") {
        bail!("{} does not say upload_enabled = false", config.display());
    }
    if !copy.dir.join(".tokensave").exists() {
        tokensave(exe, &copy.dir, &["init", "--no-git-hook"])?;
    }
    Ok(())
}

fn check_cli(args: &[String]) -> Result<()> {
    let o = options(args)?;
    let p = paths(&o)?;
    // Any mode's copy: the tokensave one puts itself back without git.
    let mode = Mode::of(o.get("mode").map_or("grep", String::as_str))?;
    let base = Copy::of(&p, mode)?;
    let copy = base.dir.clone();
    let mut wrong = 0;
    for t in &p.bench.tasks {
        let (good, bad) = match &t.change {
            None => {
                let good = t.good.as_deref().ok_or_else(|| anyhow!("{}: good", t.id))?;
                let bad = t.bad.as_deref().ok_or_else(|| anyhow!("{}: bad", t.id))?;
                (
                    checked(t, &copy, good).map_err(|e| e.to_string()),
                    checked(t, &copy, bad).map_err(|e| e.to_string()),
                )
            }
            Some(c) => {
                let mut tried = Vec::new();
                for e in [&t.good_edit, &t.bad_edit] {
                    let e = e
                        .as_ref()
                        .ok_or_else(|| anyhow!("{}: goodEdit/badEdit", t.id))?;
                    let f = copy.join(&c.file);
                    let text = std::fs::read_to_string(&f)?;
                    if !text.contains(&e.from) {
                        bail!("{}: « {} » not in {}", t.id, e.from, c.file);
                    }
                    std::fs::write(&f, text.replacen(&e.from, &e.to, 1))?;
                    tried.push(checked(t, &copy, ""));
                    if base.changed()? != [c.file.clone()] {
                        bail!("{}: the change is not seen as {} alone", t.id, c.file);
                    }
                    base.reset()?;
                    if !base.changed()?.is_empty() {
                        bail!("{}: the copy is not put back", t.id);
                    }
                }
                let bad = tried.pop().unwrap_or(Ok(()));
                (tried.pop().unwrap_or(Ok(())), bad)
            }
        };
        let right = good.is_ok() && bad.is_err();
        if !right {
            wrong += 1;
        }
        println!(
            "{} {:<28} good: {:<4} bad: {}",
            if right { "ok " } else { "KO " },
            t.id,
            if good.is_ok() { "pass" } else { "FAIL" },
            match &bad {
                Ok(()) => "PASS (should fail)".to_string(),
                Err(why) => format!("fails ({why})"),
            }
        );
    }
    if wrong > 0 {
        bail!("{wrong} check(s) wrong");
    }
    println!(
        "{} tasks, each check takes its good answer and refuses its bad one",
        p.bench.tasks.len()
    );
    Ok(())
}

fn run_cli(args: &[String]) -> Result<()> {
    let o = options(args)?;
    let p = paths(&o)?;
    let modes: Vec<Mode> = match o.get("modes") {
        Some(m) => m
            .split(',')
            .map(|s| Mode::of(s.trim()))
            .collect::<Result<_>>()?,
        None => Mode::ALL.to_vec(),
    };
    let only: Vec<String> = o
        .get("only")
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
        .unwrap_or_default();
    for id in &only {
        if !p.bench.tasks.iter().any(|t| &t.id == id) {
            bail!("no task {id}");
        }
    }
    let passes: u32 = match o.get("passes") {
        Some(n) => n.parse()?,
        None => p.bench.passes,
    };
    let claude = o.get("claude").cloned().unwrap_or_else(|| "claude".into());
    let tokensave_exe = o
        .get("tokensave")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("ESCOUADE_TOKENSAVE").map(PathBuf::from));
    if modes.contains(&Mode::Tokensave) && tokensave_exe.is_none() {
        bail!("the mode tokensave wants --tokensave <exe> (or ESCOUADE_TOKENSAVE)");
    }
    let exe = std::env::current_exe()?;
    std::fs::create_dir_all(&p.out)?;
    let runs_file = p.out.join("runs.jsonl");
    let done: BTreeSet<(String, String, u32)> = read_runs(&runs_file)?
        .into_iter()
        .map(|r| (r.task, r.mode, r.pass))
        .collect();
    let tasks: Vec<&Task> = p
        .bench
        .tasks
        .iter()
        .filter(|t| only.is_empty() || only.contains(&t.id))
        .collect();
    let appended = Mutex::new(());
    std::thread::scope(|s| -> Result<()> {
        let mut handles = Vec::new();
        for &mode in &modes {
            let (p, tasks, done, appended, runs_file, claude, exe, tokensave_exe) = (
                &p,
                &tasks,
                &done,
                &appended,
                &runs_file,
                &claude,
                &exe,
                &tokensave_exe,
            );
            handles.push(s.spawn(move || -> Result<()> {
                let copy = Copy::of(p, mode)?;
                let data = p.root.join(mode.name()).join("data");
                let server = match (mode, tokensave_exe) {
                    (Mode::Tokensave, Some(ts)) => {
                        eprintln!("[tokensave] indexing the copy…");
                        let t = Instant::now();
                        tokensave_ready(ts, &copy)?;
                        eprintln!("[tokensave] ready in {:.0} s", t.elapsed().as_secs_f64());
                        ts.clone()
                    }
                    _ => exe.clone(),
                };
                if mode.escouade() {
                    eprintln!("[{}] indexing the copy…", mode.name());
                    let t = Instant::now();
                    let svc = super::super::Service::new(crate::paths::DataDir::new(data.clone()));
                    svc.ensure_fresh(&agents_mcp::target(&copy.dir), false)?;
                    eprintln!(
                        "[{}] indexed in {:.0} s",
                        mode.name(),
                        t.elapsed().as_secs_f64()
                    );
                }
                for pass in 1..=passes {
                    for t in tasks.iter() {
                        if done.contains(&(t.id.clone(), mode.name().to_string(), pass)) {
                            continue;
                        }
                        let run = play(p, t, mode, pass, &copy, &data, claude, &server)?;
                        // tokensave's index followed the agent's change: back to the commit's.
                        if let (Mode::Tokensave, Some(ts), false) =
                            (mode, tokensave_exe, run.changed.is_empty())
                        {
                            copy.reset()?;
                            tokensave(ts, &copy.dir, &["sync"])?;
                        }
                        eprintln!(
                            "[{}] {} #{pass}: {} · {} calls · {} tokens · {:.3} $ · {:.0} s{}",
                            mode.name(),
                            t.id,
                            if run.ok { "ok" } else { "KO" },
                            run.m.tool_calls,
                            run.m.tokens(),
                            run.m.cost_usd,
                            run.wall_ms as f64 / 1000.0,
                            if run.ok {
                                String::new()
                            } else {
                                format!(" ({})", run.why)
                            }
                        );
                        let _guard = appended.lock().unwrap_or_else(|e| e.into_inner());
                        let mut f = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(runs_file)?;
                        writeln!(f, "{}", serde_json::to_string(&run)?)?;
                    }
                }
                copy.reset()
            }));
        }
        for h in handles {
            h.join()
                .map_err(|_| anyhow!("a mode's thread panicked"))??;
        }
        Ok(())
    })?;
    report(&p, &runs_file)
}

/// The prompt of a task, with what its kind asks of the answer.
fn prompt_of(b: &Bench, t: &Task) -> String {
    let rule = if t.change.is_some() {
        &b.change_rule
    } else {
        &b.answer_rule
    };
    format!("{}\n\n{rule}", t.prompt)
}

#[allow(clippy::too_many_arguments)]
fn play(
    p: &Paths,
    t: &Task,
    mode: Mode,
    pass: u32,
    c: &Copy,
    data: &Path,
    claude: &str,
    server: &Path,
) -> Result<Run> {
    c.reset()?;
    let copy = c.dir.as_path();
    let dir = p
        .out
        .join("runs")
        .join(format!("{}.{}.{pass}", t.id, mode.name()));
    let _ = remove_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let calls_log = dir.join("calls.jsonl");
    let hook_log = dir.join("hook.jsonl");
    let exe_s = server.to_string_lossy().replace('\\', "/");
    let mut settings = json!({ "autoMemoryEnabled": false });
    if mode == Mode::IndexHook {
        let command = format!(
            "\"{exe_s}\" hook --log \"{}\"",
            hook_log.to_string_lossy().replace('\\', "/")
        );
        settings["hooks"] = json!({ "PreToolUse": [{ "matcher": "Grep|Bash",
            "hooks": [{ "type": "command", "command": command }] }] });
    }
    let settings_file = dir.join("settings.json");
    std::fs::write(&settings_file, serde_json::to_string_pretty(&settings)?)?;
    let mcp_file = dir.join("mcp.json");
    let servers = match mode {
        Mode::Grep => json!({}),
        Mode::Tokensave => json!({ "tokensave": { "type": "stdio", "command": exe_s,
            "args": [ "serve", "--path", copy ],
            "env": { TOKENSAVE_ENV.0: TOKENSAVE_ENV.1 } } }),
        _ => json!({ "escouade": { "type": "stdio", "command": exe_s, "args": [
            "serve", "--dir", copy, "--data", data, "--log", calls_log ] } }),
    };
    std::fs::write(
        &mcp_file,
        serde_json::to_string_pretty(&json!({ "mcpServers": servers }))?,
    )?;
    let prompt = prompt_of(&p.bench, t);
    std::fs::write(dir.join("prompt.txt"), &prompt)?;

    let mut allowed: Vec<String> = mode.tools().iter().map(|s| s.to_string()).collect();
    if mode.escouade() {
        allowed.extend(INDEX_TOOLS.iter().map(|n| format!("{MCP_PREFIX}{n}")));
    }
    // Every tool of tokensave's server, its writing ones too, as Edit.
    if mode == Mode::Tokensave {
        allowed.push("mcp__tokensave".into());
    }
    let mut cmd = Command::new(claude);
    cmd.current_dir(copy)
        .arg("-p")
        .arg(&prompt)
        .args(["--output-format", "stream-json", "--verbose"])
        .args(["--model", &p.bench.model, "--effort", &p.bench.effort])
        .args(["--setting-sources", "project"])
        .arg("--settings")
        .arg(&settings_file)
        .arg("--mcp-config")
        .arg(&mcp_file)
        .arg("--strict-mcp-config")
        .args(["--tools", &mode.tools().join(",")])
        .args(["--allowedTools", &allowed.join(",")])
        .args(["--permission-mode", "dontAsk", "--no-session-persistence"]);
    match mode {
        Mode::Grep => {}
        Mode::Tokensave => {
            cmd.arg("--append-system-prompt")
                .arg(&p.bench.tokensave_rule);
        }
        _ => {
            cmd.arg("--append-system-prompt")
                .arg(super::super::hint::rule(LANG));
        }
    }
    // The run is no child of this session: its markers left out (their names only, no value
    // read), the default account's connection as it is.
    for (k, _) in std::env::vars_os() {
        let k = k.to_string_lossy().to_string();
        if k.starts_with("CLAUDE") || k.starts_with("ESCOUADE_") {
            cmd.env_remove(&k);
        }
    }
    cmd.env("DISABLE_AUTOUPDATER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let started = Instant::now();
    let mut child = cmd.spawn().with_context(|| format!("launching {claude}"))?;
    let mut out = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;
    let mut err = child.stderr.take().ok_or_else(|| anyhow!("no stderr"))?;
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let deadline = started + Duration::from_secs(p.bench.timeout_s);
    let mut timed_out = false;
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let wall_ms = started.elapsed().as_millis() as u64;
    let stream = reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    std::fs::write(dir.join("stream.jsonl"), &stream)?;
    std::fs::write(dir.join("stderr.txt"), &stderr)?;
    let m = measure(&stream);
    let verdict = if timed_out {
        Err(format!("timed out after {} s", p.bench.timeout_s))
    } else if m.stop.is_empty() {
        Err(format!(
            "no result: {}",
            stderr.trim().chars().take(200).collect::<String>()
        ))
    } else {
        checked(t, copy, &m.result)
    };
    let changed = c.changed()?;
    let hooked = jsonl(&hook_log);
    let hook_events = hooked.len() as u32;
    let hooks = hooked.iter().filter(|v| v["answered"] == true).count() as u32;
    let run = Run {
        task: t.id.clone(),
        mode: mode.name().into(),
        pass,
        ok: verdict.is_ok(),
        why: verdict.err().unwrap_or_default(),
        wall_ms,
        hook_events,
        hooks,
        changed,
        m,
    };
    std::fs::write(dir.join("run.json"), serde_json::to_string_pretty(&run)?)?;
    Ok(run)
}

fn jsonl(file: &Path) -> Vec<Value> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn read_runs(file: &Path) -> Result<Vec<Run>> {
    let Ok(f) = std::fs::File::open(file) else {
        return Ok(Vec::new());
    };
    let mut runs = Vec::new();
    for line in std::io::BufReader::new(f).lines() {
        let line = line?;
        if !line.trim().is_empty() {
            runs.push(
                serde_json::from_str(&line)
                    .with_context(|| format!("reading {}", file.display()))?,
            );
        }
    }
    Ok(runs)
}

// ---------------------------------------------------------------------------------------------
// The report.

fn report_cli(args: &[String]) -> Result<()> {
    let o = options(args)?;
    let p = paths(&o)?;
    let runs = o
        .get("runs")
        .map(PathBuf::from)
        .unwrap_or_else(|| p.out.join("runs.jsonl"));
    report(&p, &runs)
}

/// The sums of some runs.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub runs: u32,
    pub ok: u32,
    pub calls: BTreeMap<String, u32>,
    pub result_tokens: BTreeMap<String, u64>,
    pub tool_calls: u32,
    pub index_calls: u32,
    pub regreps: u32,
    pub globs_after: u32,
    pub reads_after: u32,
    pub rereads: u32,
    pub input: u64,
    pub cache_write: u64,
    pub cache_read: u64,
    pub output: u64,
    pub tokens: u64,
    pub index_tokens: u64,
    pub cost_usd: f64,
    pub ms: u64,
    pub turns: u32,
    pub hook_events: u32,
    pub hooks: u32,
    /// Runs that called the index and then searched or read the files anyway.
    pub completed: u32,
}

impl Totals {
    pub fn of<'a>(runs: impl IntoIterator<Item = &'a Run>) -> Totals {
        let mut t = Totals::default();
        for r in runs {
            t.runs += 1;
            t.ok += u32::from(r.ok);
            for (k, v) in &r.m.calls {
                *t.calls.entry(k.clone()).or_default() += v;
            }
            for (k, v) in &r.m.result_tokens {
                *t.result_tokens.entry(k.clone()).or_default() += v;
            }
            t.tool_calls += r.m.tool_calls;
            t.index_calls += r.m.index_calls;
            t.regreps += r.m.regreps;
            t.globs_after += r.m.globs_after;
            t.reads_after += r.m.reads_after;
            t.rereads += r.m.rereads;
            t.input += r.m.input;
            t.cache_write += r.m.cache_write;
            t.cache_read += r.m.cache_read;
            t.output += r.m.output;
            t.tokens += r.m.tokens();
            t.index_tokens += r.m.index_tokens;
            t.cost_usd += r.m.cost_usd;
            t.ms += r.m.ms;
            t.turns += r.m.turns;
            t.hook_events += r.hook_events;
            t.hooks += r.hooks;
            t.completed += u32::from(completed(r));
        }
        t
    }
}

/// Whether the agent called the index and then searched or read the files anyway.
pub fn completed(r: &Run) -> bool {
    r.m.index_calls > 0 && r.m.regreps + r.m.globs_after + r.m.reads_after > 0
}

fn delta(x: f64, base: f64) -> String {
    if base <= 0.0 {
        return "—".into();
    }
    format!("{:+.0} %", (x / base - 1.0) * 100.0)
}

fn k(n: u64) -> String {
    format!("{:.1} k", n as f64 / 1000.0)
}

pub fn markdown(b: &Bench, runs: &[Run]) -> String {
    let modes: Vec<&str> = Mode::ALL
        .iter()
        .map(|m| m.name())
        .filter(|m| runs.iter().any(|r| r.mode == *m))
        .collect();
    let of_mode = |m: &str| Totals::of(runs.iter().filter(|r| r.mode == m));
    let mut s = String::new();
    s.push_str(&format!(
        "{} runs · {} tasks · model {} ({}) · commit {}\n\n",
        runs.len(),
        runs.iter().map(|r| &r.task).collect::<BTreeSet<_>>().len(),
        b.model,
        b.effort,
        b.commit
    ));
    s.push_str("#### Totals by mode\n\n| | ");
    s.push_str(&modes.join(" | "));
    s.push_str(" |\n|---|");
    s.push_str(&"---|".repeat(modes.len()));
    s.push('\n');
    let totals: Vec<Totals> = modes.iter().map(|m| of_mode(m)).collect();
    let base = totals.first();
    let row = |label: &str, f: &dyn Fn(&Totals) -> String| {
        let mut r = format!("| {label} |");
        for t in &totals {
            r.push_str(&format!(" {} |", f(t)));
        }
        r.push('\n');
        r
    };
    s.push_str(&row("Runs (right)", &|t| format!("{} ({})", t.runs, t.ok)));
    s.push_str(&row("Tool calls", &|t| t.tool_calls.to_string()));
    let mut tools: BTreeSet<&String> = BTreeSet::new();
    for t in &totals {
        tools.extend(t.calls.keys());
    }
    for tool in tools {
        s.push_str(&row(&format!("· {tool}"), &|t| {
            t.calls.get(tool).copied().unwrap_or_default().to_string()
        }));
    }
    s.push_str(&row("Grep after the index", &|t| t.regreps.to_string()));
    s.push_str(&row("Glob / Read after the index", &|t| {
        format!("{} / {}", t.globs_after, t.reads_after)
    }));
    s.push_str(&row("Rereads", &|t| t.rereads.to_string()));
    s.push_str(&row("Runs completed after the index", &|t| {
        format!("{} / {}", t.completed, t.runs)
    }));
    s.push_str(&row("Hook: searches seen / answered", &|t| {
        format!("{} / {}", t.hook_events, t.hooks)
    }));
    s.push_str(&row("Tokens: input", &|t| k(t.input)));
    s.push_str(&row("· cache written", &|t| k(t.cache_write)));
    s.push_str(&row("· cache read", &|t| k(t.cache_read)));
    s.push_str(&row("· output", &|t| k(t.output)));
    s.push_str(&row("· total", &|t| k(t.tokens)));
    s.push_str(&row("Tokens of the tools' answers: the index's", &|t| {
        k(t.index_tokens)
    }));
    let mut answered: BTreeSet<&String> = BTreeSet::new();
    for t in &totals {
        answered.extend(t.result_tokens.keys());
    }
    for tool in answered {
        s.push_str(&row(&format!("· {tool}"), &|t| {
            k(t.result_tokens.get(tool).copied().unwrap_or_default())
        }));
    }
    s.push_str(&row("Cost ($, Claude Code's count)", &|t| {
        format!("{:.3}", t.cost_usd)
    }));
    s.push_str(&row("· against grep", &|t| {
        delta(t.cost_usd, base.map_or(0.0, |b| b.cost_usd))
    }));
    s.push_str(&row("Time (s)", &|t| {
        format!("{:.0}", t.ms as f64 / 1000.0)
    }));
    s.push_str(&row("Turns", &|t| t.turns.to_string()));

    s.push_str("\n#### By task: right, calls, cost, against grep\n\n| Task |");
    for m in &modes {
        s.push_str(&format!(" {m} |"));
    }
    s.push_str("\n|---|");
    s.push_str(&"---|".repeat(modes.len()));
    s.push('\n');
    for t in &b.tasks {
        let of = |m: &str| Totals::of(runs.iter().filter(|r| r.task == t.id && r.mode == m));
        let grep = of("grep");
        if modes.iter().all(|m| of(m).runs == 0) {
            continue;
        }
        s.push_str(&format!("| {} |", t.id));
        for m in &modes {
            let x = of(m);
            if x.runs == 0 {
                s.push_str(" — |");
                continue;
            }
            let vs = if *m == "grep" {
                String::new()
            } else {
                format!(" ({})", delta(x.cost_usd, grep.cost_usd))
            };
            s.push_str(&format!(
                " {}/{} · {} calls · {:.3} ${vs} |",
                x.ok, x.runs, x.tool_calls, x.cost_usd
            ));
        }
        s.push('\n');
    }

    s.push_str("\n#### Every run\n\n| Task | Mode | # | Right | Calls by tool | Grep after | Rereads | Tokens (in / cache w / cache r / out) | $ | s | Turns |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
    for t in &b.tasks {
        for m in &modes {
            for r in runs.iter().filter(|r| r.task == t.id && r.mode == *m) {
                let calls =
                    r.m.calls
                        .iter()
                        .map(|(k, v)| format!("{k} {v}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                s.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} | {} | {} / {} / {} / {} | {:.3} | {:.0} | {} |\n",
                    r.task,
                    r.mode,
                    r.pass,
                    if r.ok { "yes".to_string() } else { format!("no ({})", r.why) },
                    calls,
                    r.m.regreps,
                    r.m.rereads,
                    r.m.input,
                    r.m.cache_write,
                    r.m.cache_read,
                    r.m.output,
                    r.m.cost_usd,
                    r.m.ms as f64 / 1000.0,
                    r.m.turns
                ));
            }
        }
    }

    s.push_str("\n#### Completed after the index\n\nThe runs that called the index, then searched or read the files anyway (Grep, Glob, Read after its first call).\n\n| Task | Mode | # | Index calls | Grep / Glob / Read after |\n|---|---|---|---|---|\n");
    for r in runs.iter().filter(|r| completed(r)) {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} / {} / {} |\n",
            r.task, r.mode, r.pass, r.m.index_calls, r.m.regreps, r.m.globs_after, r.m.reads_after
        ));
    }
    s
}

/// The questions' runs checked again, from their answers, by the checks as they are now (a check
/// widened after reading): the runs whose verdict changed, as they were.
pub fn rescored(b: &Bench, runs: &mut [Run]) -> Vec<Run> {
    let mut changed = Vec::new();
    for r in runs.iter_mut() {
        let Some(t) = b.tasks.iter().find(|t| t.id == r.task) else {
            continue;
        };
        // A change is checked on its copy, gone since; a run without an answer stays as it was.
        if t.change.is_some() || r.m.stop.is_empty() {
            continue;
        }
        let ok = answer_ok(&t.answer, &r.m.result);
        if ok != r.ok {
            changed.push(r.clone());
            r.ok = ok;
            r.why = if ok {
                String::new()
            } else {
                "the answer misses what was asked".into()
            };
        }
    }
    changed
}

fn report(p: &Paths, runs_file: &Path) -> Result<()> {
    let mut runs = read_runs(runs_file)?;
    std::fs::create_dir_all(&p.out)?;
    let changed = rescored(&p.bench, &mut runs);
    let mut md = markdown(&p.bench, &runs);
    if !changed.is_empty() {
        md.push_str("\n#### Checked again\n\nThe runs whose verdict changed with a check widened after their answers were read.\n\n| Task | Mode | # | At the run | Now | Why |\n|---|---|---|---|---|---|\n");
        for r in &changed {
            let why = p
                .bench
                .tasks
                .iter()
                .find(|t| t.id == r.task)
                .and_then(|t| t.widened.clone())
                .unwrap_or_default();
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {why} |\n",
                r.task,
                r.mode,
                r.pass,
                if r.ok { "right" } else { "wrong" },
                if r.ok { "wrong" } else { "right" },
            ));
        }
    }
    std::fs::write(p.out.join("results.md"), &md)?;
    let by_mode: BTreeMap<&str, Totals> = Mode::ALL
        .iter()
        .map(|m| {
            (
                m.name(),
                Totals::of(runs.iter().filter(|r| r.mode == m.name())),
            )
        })
        .collect();
    std::fs::write(
        p.out.join("results.json"),
        serde_json::to_string_pretty(&json!({
            "commit": p.bench.commit, "model": p.bench.model, "effort": p.bench.effort,
            "modes": by_mode, "runs": runs,
        }))?,
    )?;
    let _ = std::io::stdout().write_all(md.as_bytes());
    eprintln!("written to {}", p.out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bench() -> Bench {
        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("docs")
            .join("codeindex-agents.json");
        serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
    }

    #[test]
    fn the_bench_has_twenty_tasks_each_with_its_check_and_its_two_samples() {
        let b = bench();
        assert_eq!(b.tasks.len(), 20);
        let ids: BTreeSet<&str> = b.tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids.len(), 20, "unique ids");
        for t in &b.tasks {
            match &t.change {
                None => {
                    assert!(!t.answer.is_empty(), "{}", t.id);
                    // The question's check, tried on its good answer and on its bad one.
                    assert!(
                        answer_ok(&t.answer, t.good.as_deref().unwrap()),
                        "{} good",
                        t.id
                    );
                    assert!(
                        !answer_ok(&t.answer, t.bad.as_deref().unwrap()),
                        "{} bad",
                        t.id
                    );
                }
                Some(c) => {
                    assert!(!c.added.is_empty(), "{}", t.id);
                    assert!(t.good_edit.is_some() && t.bad_edit.is_some(), "{}", t.id);
                }
            }
        }
    }

    #[test]
    fn an_answer_says_each_need_by_one_of_its_alternatives_whatever_its_slashes() {
        let needs = vec![
            vec!["plan_resume".to_string()],
            vec!["src-tauri/src/core.rs".to_string(), "core.rs".to_string()],
        ];
        assert!(answer_ok(
            &needs,
            "RÉPONSE : src-tauri\\src\\core.rs — Plan_Resume"
        ));
        assert!(!answer_ok(
            &needs,
            "RÉPONSE : src-tauri/src/core.rs — set_resume"
        ));
        assert!(!answer_ok(&[], "anything"));
    }

    #[test]
    fn a_change_is_checked_within_its_symbol_only() {
        let text = "const A: u32 = 1;\nfn start() {\n    let x = {\n        400\n    };\n}\nfn other() {\n    250\n}\n";
        assert_eq!(scope_lines(text, "fn start("), Some((1, 5)));
        assert_eq!(scope_lines(text, "const A"), Some((0, 0)));
        assert_eq!(scope_lines(text, "fn none("), None);
        let c = |added: &str, gone: &[&str]| Change {
            file: "x.rs".into(),
            scope: "fn start(".into(),
            added: vec![vec![added.to_string()]],
            gone: gone.iter().map(|g| g.to_string()).collect(),
        };
        assert!(change_ok(&c("400", &[]), text).is_ok());
        // 250 is there, but in another function.
        assert!(change_ok(&c("250", &[]), text).is_err());
        assert!(change_ok(&c("let x", &["400"]), text).is_err());
    }

    #[test]
    fn a_run_is_measured_from_its_stream() {
        let stream = [
            json!({ "type": "system", "subtype": "init", "mcp_servers": [{ "name": "escouade", "status": "connected" }] }),
            json!({ "type": "assistant", "message": { "content": [
                { "type": "tool_use", "name": "Grep", "input": { "pattern": "x" } },
                { "type": "tool_use", "name": "Read", "input": { "file_path": "C:\\r\\a.rs" } },
            ] } }),
            json!({ "type": "assistant", "message": { "content": [
                { "type": "text", "text": "…" },
                { "type": "tool_use", "id": "t1", "name": "mcp__escouade__index_search", "input": { "query": "x" } },
            ] } }),
            json!({ "type": "user", "message": { "content": [
                { "type": "tool_result", "tool_use_id": "t1", "content": [
                    { "type": "text", "text": "a".repeat(400) }, { "type": "text", "text": "b".repeat(39) } ] },
            ] } }),
            json!({ "type": "assistant", "message": { "content": [
                { "type": "tool_use", "name": "Grep", "input": { "pattern": "y" } },
                { "type": "tool_use", "name": "Read", "input": { "file_path": "C:/r/a.rs" } },
                { "type": "tool_use", "name": "Glob", "input": { "pattern": "*" } },
            ] } }),
            json!({ "type": "result", "subtype": "success", "result": "RÉPONSE : a.rs", "duration_ms": 4200,
                "num_turns": 4, "total_cost_usd": 0.0123, "permission_denials": [],
                "modelUsage": { "claude-haiku-5-5": { "inputTokens": 10, "outputTokens": 200,
                    "cacheReadInputTokens": 3000, "cacheCreationInputTokens": 400 } } }),
        ]
        .map(|v| v.to_string())
        .join("\n");
        let m = measure(&stream);
        assert_eq!(m.mcp, "escouade:connected");
        assert_eq!(m.tool_calls, 6);
        assert_eq!(m.calls["Grep"], 2);
        assert_eq!(m.calls["index_search"], 1);
        assert_eq!(m.index_calls, 1);
        // 400 + 1 + 39 bytes, four to a token.
        assert_eq!(m.index_tokens, 110);
        assert_eq!(m.result_tokens["index_search"], 110);
        assert_eq!(
            short("mcp__tokensave__tokensave_context"),
            "tokensave_context"
        );
        assert_eq!((m.regreps, m.globs_after, m.reads_after), (1, 1, 1));
        // The same file, by its other slashes.
        assert_eq!(m.rereads, 1);
        assert_eq!(
            (m.input, m.cache_write, m.cache_read, m.output),
            (10, 400, 3000, 200)
        );
        assert_eq!(m.tokens(), 3610);
        assert_eq!((m.ms, m.turns, m.stop.as_str()), (4200, 4, "success"));
        assert_eq!(m.result, "RÉPONSE : a.rs");
        let run = Run {
            task: "t".into(),
            mode: "index-hook".into(),
            pass: 1,
            ok: true,
            m,
            ..Default::default()
        };
        assert!(completed(&run));
        let t = Totals::of([&run, &run]);
        assert_eq!((t.runs, t.ok, t.regreps, t.completed), (2, 2, 2, 2));
    }
}
