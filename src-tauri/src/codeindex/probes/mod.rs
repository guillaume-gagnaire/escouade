//! The probes bench of the code index (ESC-294): how right, how fast and how cheap the five tools
//! are, against the grep an agent would write, on this repository at a given commit.
//!
//! `cargo run --example codeindex_probes -- [--commit <rev>] [--probes <file>] [--out <dir>]
//! [--only <id,…>] [--rg <program>] [--compare <file>] [--keep]`, from `src-tauri/` (`--rg`, or `ESCOUADE_PROBES_RG`,
//! names ripgrep when the `rg` of the PATH is not a program):
//! - the repository is cloned (`git clone --shared`) at the commit into a temporary folder, and
//!   indexed there by a `Service` of its own: the app's index and the user's settings are never
//!   touched, and no model of the search by meaning is loaded (`index_context` uses none);
//! - the probes are read from `docs/codeindex-probes.json`: a question, the tool's arguments, the
//!   answer expected (written by hand), the searches by `rg` an agent would make;
//! - each probe is played by the index (limit 10 and 50, cold and warm) and by `rg`, then scored
//!   (`score`); a worktree with changes and a file changed just before the call are probed too;
//! - the indexing of the whole repository, of a commit and of a change of branch is timed;
//! - `results.json`, `results.md` and `reference.json` (the figures a later run compares to, with
//!   `--compare`) go to `--out` (`target/codeindex-probes` by default).

pub mod score;

use super::{Answer, Reader, Service, Target, MIN_GAP};
use crate::paths::DataDir;
use anyhow::{anyhow, bail, Context as _, Result};
use score::{economy, good_flags, parse_expect, read_count, tokens, windows, Alt, Item, Score};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// The lines of a search an agent sees: Claude Code's Grep tool gives 250 by default, and cuts a
/// line longer than 500 characters (`--max-columns`).
const GREP_LINES: usize = 250;

/// The tools' limit when the agent gives none (`mcp/index.rs`), and the most it may ask.
const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = 50;
const PROJECT: &str = "probes";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bench {
    /// The commit the expected answers were written for.
    commit: String,
    /// What an agent reads to conclude: so many lines around each result…
    read_radius: u32,
    /// …of the first so many results.
    read_max: usize,
    /// The changes made in the agent's worktree.
    worktree: Vec<Edit>,
    /// The file changed just before a call.
    fresh: Fresh,
    probes: Vec<Probe>,
}

#[derive(Debug, Deserialize)]
struct Edit {
    file: String,
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct Fresh {
    file: String,
    append: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Probe {
    id: String,
    tool: String,
    lang: String,
    #[serde(default)]
    tags: Vec<String>,
    question: String,
    /// `base` (the project's folder), `worktree`, `fresh` (a file changed just before the call,
    /// the last check less than `MIN_GAP` old) or `fresh-gap` (the same, `MIN_GAP` later).
    #[serde(default = "base")]
    stage: String,
    args: Args,
    expect: Vec<String>,
    grep: Vec<Grep>,
    /// A first search made of the question's words alone, by an agent that does not know the
    /// code (« grep à l'aveugle »), played before `grep` when it finds nothing.
    #[serde(default)]
    blind: Vec<Grep>,
}

fn base() -> String {
    "base".into()
}

#[derive(Debug, Deserialize, Default)]
struct Args {
    query: Option<String>,
    task: Option<String>,
    name: Option<String>,
    path: Option<String>,
    target: Option<String>,
    kind: Option<String>,
    depth: Option<u32>,
    /// The same task told with the words of the code, in English, as a tool's description could
    /// ask the agent to (a lead with no model).
    #[serde(rename = "codeWords")]
    code_words: Option<String>,
}

/// One search by `rg`, run from the folder probed.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct Grep {
    pattern: String,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexSide {
    at10: Score,
    at50: Score,
    cold_ms: f64,
    warm_ms: f64,
    ms50: f64,
    /// The tool's answer as the MCP server gives it.
    response_tokens: u64,
    /// The part of it that is wrong results (and their code).
    wrong_tokens: u64,
    read_tokens: u64,
    total_tokens: u64,
    /// The cost when the index found nothing right and the agent searched again with grep.
    real_tokens: u64,
    announced_percent: u64,
    announced_files_tokens: u64,
    /// The calls of the path to the answer: the tool, then each reading (and grep's when it fell
    /// back).
    calls: usize,
    real_calls: usize,
    /// The task told with the words of the code (`codeWords`).
    code_words: Option<Score>,
    /// The results at the default limit, `✓`/`✗` and where they point.
    results: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrepSide {
    score: Score,
    cold_ms: f64,
    warm_ms: f64,
    output_tokens: u64,
    read_tokens: u64,
    total_tokens: u64,
    /// Each search, then each reading.
    calls: usize,
    searches: Vec<Grep>,
    results: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Outcome {
    id: String,
    tool: String,
    lang: String,
    tags: Vec<String>,
    stage: String,
    question: String,
    index: IndexSide,
    grep: GrepSide,
    /// The naive search alone, then the path of an agent that starts with it: its tokens and
    /// calls, the informed search added when it found nothing.
    blind: Option<GrepSide>,
    blind_tokens: u64,
    blind_calls: usize,
    /// What the index saves against grep, its reading and its fallback counted.
    real_percent: i64,
    winner: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct Indexing {
    files: u64,
    symbols: u64,
    links: u64,
    bytes: u64,
    full_ms: u128,
    worktree_ms: u128,
    worktree_parsed: usize,
    commit_ms: u128,
    commit_parsed: usize,
    branch_files_changed: usize,
    branch_ms: u128,
    branch_parsed: usize,
    back_ms: u128,
    back_parsed: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    commit: String,
    date: String,
    command: String,
    rg: String,
    read_radius: u32,
    read_max: usize,
    indexing: Indexing,
    probes: Vec<Outcome>,
}

struct Opts {
    commit: Option<String>,
    probes: Option<PathBuf>,
    out: Option<PathBuf>,
    only: Vec<String>,
    rg: PathBuf,
    /// A reference to compare to (`docs/codeindex-probes-reference.json`).
    compare: Option<PathBuf>,
    keep: bool,
}

/// The command (`examples/codeindex_probes.rs`). Gives the exit code.
pub fn cli(args: Vec<String>) -> i32 {
    match parse(args).and_then(run) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("codeindex_probes: {e:#}");
            1
        }
    }
}

fn parse(args: Vec<String>) -> Result<Opts> {
    let mut o = Opts {
        commit: None,
        probes: None,
        out: None,
        only: Vec::new(),
        rg: std::env::var_os("ESCOUADE_PROBES_RG")
            .map(PathBuf::from)
            .unwrap_or_else(|| "rg".into()),
        compare: None,
        keep: false,
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or_else(|| anyhow!("{a} wants a value"));
        match a.as_str() {
            "--commit" => o.commit = Some(value()?),
            "--probes" => o.probes = Some(value()?.into()),
            "--out" => o.out = Some(value()?.into()),
            "--only" => o.only = value()?.split(',').map(|s| s.trim().to_string()).collect(),
            "--rg" => o.rg = value()?.into(),
            "--compare" => o.compare = Some(value()?.into()),
            "--keep" => o.keep = true,
            "-h" | "--help" => {
                println!(
                    "codeindex_probes [--commit <rev>] [--probes <file>] [--out <dir>] [--only <id,…>] [--rg <program>] [--compare <reference.json>] [--keep]"
                );
                std::process::exit(0);
            }
            _ => bail!("unknown argument {a}"),
        }
    }
    Ok(o)
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=probes",
            "-c",
            "user.email=probes@localhost",
        ])
        .args(args)
        .output()
        .with_context(|| format!("git {}", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn ms(t: Instant) -> f64 {
    (t.elapsed().as_secs_f64() * 10_000.0).round() / 10.0
}

fn run(o: Opts) -> Result<()> {
    let here = std::env::current_dir()?;
    let repo = PathBuf::from(git(&here, &["rev-parse", "--show-toplevel"])?);
    let probes_file = o
        .probes
        .clone()
        .unwrap_or_else(|| repo.join("docs").join("codeindex-probes.json"));
    let bench: Bench = serde_json::from_str(
        &std::fs::read_to_string(&probes_file)
            .with_context(|| format!("reading {}", probes_file.display()))?,
    )
    .with_context(|| format!("reading {}", probes_file.display()))?;
    let out_dir = o.out.clone().unwrap_or_else(|| {
        repo.join("src-tauri")
            .join("target")
            .join("codeindex-probes")
    });
    let commit = o.commit.clone().unwrap_or_else(|| bench.commit.clone());
    let rg_version = Command::new(&o.rg)
        .arg("--version")
        .output()
        .context("ripgrep: `rg` on the PATH, or --rg <program>")?;
    let rg_version = String::from_utf8_lossy(&rg_version.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();

    let tmp =
        std::env::temp_dir().join(format!("escouade-codeindex-probes-{}", std::process::id()));
    let _ = remove_all(&tmp);
    std::fs::create_dir_all(&tmp)?;
    let clone = tmp.join("repo");
    let wt = tmp.join("wt");
    let data = DataDir::new(tmp.join("data"));
    eprintln!(
        "cloning {} at {commit} into {}",
        repo.display(),
        clone.display()
    );
    git(
        &tmp,
        &[
            "clone",
            "--quiet",
            "--shared",
            "--no-checkout",
            &repo.to_string_lossy(),
            &clone.to_string_lossy(),
        ],
    )?;
    git(&clone, &["checkout", "--quiet", "--detach", &commit])?;
    let sha = git(&clone, &["rev-parse", "HEAD"])?;

    let target = |dir: &Path| Target {
        project_id: PROJECT.into(),
        root: clone.clone(),
        dir: dir.to_path_buf(),
        fence: Default::default(),
        exclude: Vec::new(),
    };
    let base_t = target(&clone);
    let svc = Service::new(data.clone());
    let mut ix = Indexing::default();
    eprintln!("indexing the whole repository…");
    let t = Instant::now();
    svc.ensure_fresh(&base_t, true)?;
    ix.full_ms = t.elapsed().as_millis();
    let st = svc.status(PROJECT);
    (ix.files, ix.symbols, ix.links, ix.bytes) = (st.files, st.symbols, st.links, st.bytes);

    git(
        &clone,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            &wt.to_string_lossy(),
            "HEAD",
        ],
    )?;
    for e in &bench.worktree {
        let f = wt.join(&e.file);
        let text = std::fs::read_to_string(&f).with_context(|| e.file.clone())?;
        if !text.contains(&e.from) {
            bail!("the worktree's edit finds no « {} » in {}", e.from, e.file);
        }
        std::fs::write(&f, text.replace(&e.from, &e.to))?;
    }
    let wt_t = target(&wt);
    let t = Instant::now();
    let r = svc.ensure_fresh(&wt_t, true)?.unwrap_or_default();
    ix.worktree_ms = t.elapsed().as_millis();
    ix.worktree_parsed = r.parsed;

    let fresh_file = clone.join(&bench.fresh.file);
    let fresh_before = std::fs::read_to_string(&fresh_file)?;
    // The service of the agents' queries, kept from one to the next as the app keeps it.
    let live = Service::new(data.clone());
    let mut fresh_done = false;
    let mut outcomes = Vec::new();
    for p in bench
        .probes
        .iter()
        .filter(|p| o.only.is_empty() || o.only.contains(&p.id))
    {
        eprintln!("probe {}…", p.id);
        let (t, dir) = match p.stage.as_str() {
            "worktree" => (&wt_t, wt.as_path()),
            _ => (&base_t, clone.as_path()),
        };
        let expect: Vec<Vec<Alt>> = p.expect.iter().map(|e| parse_expect(e)).collect();
        let (a10, cold_ms, warm_ms, a50, ms50) = match p.stage.as_str() {
            "base" | "worktree" => {
                // Cold: a service that has checked nothing yet; warm: the same, at once.
                let s = Service::new(data.clone());
                let t0 = Instant::now();
                let a10 = call(&s.reader(t)?, p, DEFAULT_LIMIT)?;
                let cold = ms(t0);
                let t0 = Instant::now();
                call(&s.reader(t)?, p, DEFAULT_LIMIT)?;
                let warm = ms(t0);
                let t0 = Instant::now();
                let a50 = call(&s.reader(t)?, p, MAX_LIMIT)?;
                (a10, cold, warm, a50, ms(t0))
            }
            "fresh" | "fresh-gap" => {
                if !fresh_done {
                    // An agent's query, then its edit of a file (no git event), then its probe.
                    call(&live.reader(t)?, p, DEFAULT_LIMIT)?;
                    std::fs::write(&fresh_file, format!("{fresh_before}{}", bench.fresh.append))?;
                    fresh_done = true;
                }
                if p.stage == "fresh-gap" {
                    std::thread::sleep(MIN_GAP + std::time::Duration::from_millis(250));
                }
                let t0 = Instant::now();
                let a10 = call(&live.reader(t)?, p, DEFAULT_LIMIT)?;
                let at = ms(t0);
                let t0 = Instant::now();
                let a50 = call(&live.reader(t)?, p, MAX_LIMIT)?;
                (a10, at, at, a50, ms(t0))
            }
            other => bail!("{}: unknown stage {other}", p.id),
        };
        let index = index_side(&bench, &expect, dir, &a10, &a50, cold_ms, warm_ms, ms50);
        let grep = grep_side(&bench, &expect, dir, &p.grep, &o.rg)?;
        let mut index = index;
        (index.real_tokens, index.real_calls) = if !expect.is_empty() && index.at10.found == 0 {
            (
                index.total_tokens + grep.total_tokens,
                index.calls + grep.calls,
            )
        } else {
            (index.total_tokens, index.calls)
        };
        if let Some(words) = &p.args.code_words {
            let r = Service::new(data.clone()).reader(t)?;
            let c = r.task_context(words, DEFAULT_LIMIT)?.result;
            let items: Vec<Item> = c
                .entry_points
                .iter()
                .chain(&c.related)
                .map(|h| Item::new(&h.path, h.line))
                .collect();
            index.code_words = Some(score::score(&expect, &items));
        }
        let blind = if p.blind.is_empty() {
            None
        } else {
            Some(grep_side(&bench, &expect, dir, &p.blind, &o.rg)?)
        };
        let (blind_tokens, blind_calls) = match &blind {
            Some(b) if b.score.found > 0 => (b.total_tokens, b.calls),
            Some(b) => (b.total_tokens + grep.total_tokens, b.calls + grep.calls),
            None => (grep.total_tokens, grep.calls),
        };
        let real_percent = economy(index.real_tokens, grep.total_tokens);
        let winner = winner(&index, &grep);
        outcomes.push(Outcome {
            id: p.id.clone(),
            tool: p.tool.clone(),
            lang: p.lang.clone(),
            tags: p.tags.clone(),
            stage: p.stage.clone(),
            question: p.question.clone(),
            index,
            grep,
            blind,
            blind_tokens,
            blind_calls,
            real_percent,
            winner,
        });
    }
    std::fs::write(&fresh_file, &fresh_before)?;
    drop(live);

    // The updates: a commit, a change of branch, the way back.
    eprintln!("timing the updates…");
    svc.ensure_fresh(&base_t, true)?;
    std::fs::write(&fresh_file, format!("{fresh_before}{}", bench.fresh.append))?;
    git(&clone, &["commit", "--quiet", "-am", "probe: a commit"])?;
    let t = Instant::now();
    let r = svc.ensure_fresh(&base_t, true)?.unwrap_or_default();
    (ix.commit_ms, ix.commit_parsed) = (t.elapsed().as_millis(), r.parsed);
    let older = format!("{sha}~30");
    ix.branch_files_changed = git(&clone, &["diff", "--name-only", "HEAD", &older])?
        .lines()
        .count();
    git(
        &clone,
        &["checkout", "--quiet", "-b", "probe-older", &older],
    )?;
    let t = Instant::now();
    let r = svc.ensure_fresh(&base_t, true)?.unwrap_or_default();
    (ix.branch_ms, ix.branch_parsed) = (t.elapsed().as_millis(), r.parsed);
    git(&clone, &["checkout", "--quiet", "--detach", &sha])?;
    let t = Instant::now();
    let r = svc.ensure_fresh(&base_t, true)?.unwrap_or_default();
    (ix.back_ms, ix.back_parsed) = (t.elapsed().as_millis(), r.parsed);
    drop(svc);

    let report = Report {
        commit: sha,
        date: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        command: "cd src-tauri && cargo run --release --example codeindex_probes".into(),
        rg: rg_version,
        read_radius: bench.read_radius,
        read_max: bench.read_max,
        indexing: ix,
        probes: outcomes,
    };
    std::fs::create_dir_all(&out_dir)?;
    std::fs::write(
        out_dir.join("results.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    let reference = reference_of(&report);
    std::fs::write(
        out_dir.join("reference.json"),
        serde_json::to_string_pretty(&reference)?,
    )?;
    let mut md = markdown(&report);
    if let Some(file) = &o.compare {
        let before: Value = serde_json::from_str(
            &std::fs::read_to_string(file).with_context(|| file.display().to_string())?,
        )
        .with_context(|| file.display().to_string())?;
        md.push_str(&compared(&before, &reference));
    }
    std::fs::write(out_dir.join("results.md"), &md)?;
    println!("{md}");
    eprintln!("written to {}", out_dir.display());
    if o.keep {
        eprintln!("kept: {}", tmp.display());
    } else if let Err(e) = remove_all(&tmp) {
        eprintln!("could not remove {}: {e}", tmp.display());
    }
    Ok(())
}

/// The index's answer to a probe, as JSON, and what it said it saved.
fn call(r: &Reader, p: &Probe, limit: usize) -> Result<Answer<Value>> {
    let a = &p.args;
    let need = |v: &Option<String>, what: &str| {
        v.clone()
            .ok_or_else(|| anyhow!("{}: {} wants `{what}`", p.id, p.tool))
    };
    fn to<T: Serialize>(a: Answer<T>) -> Result<Answer<Value>> {
        Ok(Answer {
            result: serde_json::to_value(&a.result)?,
            saving: a.saving,
        })
    }
    match p.tool.as_str() {
        "index_search" => to(r.search(&need(&a.query, "query")?, limit, a.kind.as_deref())?),
        "index_context" => to(r.task_context(&need(&a.task, "task")?, limit)?),
        "index_callers" => to(r.callers(&need(&a.name, "name")?, a.path.as_deref(), limit)?),
        "index_callees" => to(r.callees(&need(&a.name, "name")?, a.path.as_deref(), limit)?),
        "index_impact" => to(r.impact(
            &need(&a.target, "target")?,
            a.depth.unwrap_or(2).clamp(1, 4),
            limit,
        )?),
        other => bail!("{}: unknown tool {other}", p.id),
    }
}

/// A result of the index: where it points, its JSON, whether its code is in the answer.
struct Found {
    item: Item,
    json: String,
    shown: Option<String>,
}

/// The results of an answer, in the order the agent reads them.
fn found(tool: &str, v: &Value) -> Vec<Found> {
    let of = |x: &Value| Found {
        item: Item::new(
            x["path"].as_str().unwrap_or_default(),
            x["line"].as_u64().unwrap_or(0) as u32,
        ),
        json: x.to_string(),
        shown: None,
    };
    let list = |key: &str| -> Vec<Value> { v[key].as_array().cloned().unwrap_or_default() };
    match tool {
        "index_search" => v.as_array().into_iter().flatten().map(of).collect(),
        "index_context" => {
            let snippets = list("snippets");
            let mut out: Vec<Found> = list("entryPoints")
                .iter()
                .map(|x| {
                    let mut f = of(x);
                    f.shown = snippets
                        .iter()
                        .find(|s| s["path"] == x["path"] && s["line"] == x["line"])
                        .map(|s| s.to_string());
                    f
                })
                .collect();
            out.extend(list("related").iter().map(of));
            out
        }
        "index_callers" => list("defs")
            .iter()
            .chain(&list("callers"))
            .map(of)
            .collect(),
        "index_callees" => list("defs").iter().chain(&list("calls")).map(of).collect(),
        "index_impact" => {
            let mut out: Vec<Found> = list("targets")
                .iter()
                .chain(&list("callers"))
                .chain(&list("subtypes"))
                .map(of)
                .collect();
            out.extend(list("importers").iter().map(|p| Found {
                item: Item::new(p.as_str().unwrap_or_default(), 0),
                json: p.to_string(),
                shown: None,
            }));
            out
        }
        _ => Vec::new(),
    }
}

/// The MCP server's answer (`mcp/index.rs`): the result, the saving and its sentence.
fn response(a: &Answer<Value>) -> String {
    let s = &a.saving;
    let summary = if s.files == 0 {
        "Rien de trouvé : aucun fichier n’a été lu pour cette réponse.".to_string()
    } else {
        format!(
            "Économie : environ {} tokens ({} %). Lire les {} fichiers concernés en entier aurait coûté environ {} tokens ; cette réponse en coûte environ {}.",
            s.saved_tokens, s.saved_percent, s.files, s.files_tokens, s.answer_tokens
        )
    };
    json!({
        "project": PROJECT,
        "projectId": PROJECT,
        "result": a.result,
        "saving": a.saving,
        "summary": summary,
    })
    .to_string()
}

/// The tokens of the lines `windows` of the files of `dir`, each cut at 2000 characters as Claude
/// Code's Read tool cuts them.
fn read_tokens(dir: &Path, w: &[(String, u32, u32)]) -> u64 {
    w.iter()
        .map(|(path, from, to)| {
            let text = std::fs::read_to_string(dir.join(path)).unwrap_or_default();
            let part: String = text
                .lines()
                .skip(*from as usize - 1)
                .take((*to - *from + 1) as usize)
                .map(|l| format!("{}\n", l.chars().take(2000).collect::<String>()))
                .collect();
            tokens(&part)
        })
        .sum()
}

fn marks(items: &[Item], flags: &[bool]) -> Vec<String> {
    items
        .iter()
        .zip(flags)
        .map(|(i, g)| format!("{} {}:{}", if *g { "✓" } else { "✗" }, i.path, i.line))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn index_side(
    bench: &Bench,
    expect: &[Vec<Alt>],
    dir: &Path,
    a10: &Answer<Value>,
    a50: &Answer<Value>,
    cold_ms: f64,
    warm_ms: f64,
    ms50: f64,
) -> IndexSide {
    let tool = |a: &Answer<Value>| found(tool_of(a), &a.result);
    let f10 = tool(a10);
    let f50 = tool(a50);
    let items10: Vec<Item> = f10.iter().map(|f| f.item.clone()).collect();
    let items50: Vec<Item> = f50.iter().map(|f| f.item.clone()).collect();
    let flags = good_flags(expect, &items10);
    let wrong_tokens = f10
        .iter()
        .zip(&flags)
        .filter(|(_, g)| !**g)
        .map(|(f, _)| tokens(&f.json) + f.shown.as_deref().map(tokens).unwrap_or(0))
        .sum();
    // The code of an entry point is in the answer: it needs no reading.
    let to_read: Vec<Item> = f10
        .iter()
        .take(read_count(expect.len(), &flags))
        .filter(|f| f.shown.is_none())
        .map(|f| f.item.clone())
        .collect();
    let response_tokens = tokens(&response(a10));
    let read_windows = windows(&to_read, bench.read_radius, bench.read_max);
    let read = read_tokens(dir, &read_windows);
    IndexSide {
        at10: score::score(expect, &items10),
        at50: score::score(expect, &items50),
        cold_ms,
        warm_ms,
        ms50,
        response_tokens,
        wrong_tokens,
        read_tokens: read,
        total_tokens: response_tokens + read,
        real_tokens: 0,
        announced_percent: a10.saving.saved_percent,
        announced_files_tokens: a10.saving.files_tokens,
        calls: 1 + read_windows.len(),
        real_calls: 0,
        code_words: None,
        results: marks(&items10, &flags),
    }
}

/// Which tool answered: the shape of the result says it.
fn tool_of(a: &Answer<Value>) -> &'static str {
    let v = &a.result;
    if v.is_array() {
        "index_search"
    } else if v.get("entryPoints").is_some() {
        "index_context"
    } else if v.get("external").is_some() {
        "index_callees"
    } else if v.get("importers").is_some() {
        "index_impact"
    } else {
        "index_callers"
    }
}

fn grep_side(
    bench: &Bench,
    expect: &[Vec<Alt>],
    dir: &Path,
    searches: &[Grep],
    program: &Path,
) -> Result<GrepSide> {
    let mut output = String::new();
    let mut items: Vec<Item> = Vec::new();
    let (mut cold, mut warm) = (0.0, 0.0);
    for g in searches {
        let rg = || {
            let mut c = Command::new(program);
            c.current_dir(dir)
                .args([
                    "-n",
                    "-H",
                    "--no-heading",
                    "--color",
                    "never",
                    "--max-columns",
                    "500",
                ])
                .args(["--path-separator", "/", "--sort", "path"])
                .args(&g.args);
            // No pattern: the files found by their name (`--files -g`).
            if !g.pattern.is_empty() {
                c.arg("-e").arg(&g.pattern);
            }
            c.args(&g.paths);
            c
        };
        let t0 = Instant::now();
        let out = rg().output().context("rg")?;
        cold += ms(t0);
        let t0 = Instant::now();
        rg().output().context("rg")?;
        warm += ms(t0);
        let text: String = String::from_utf8_lossy(&out.stdout)
            .lines()
            .take(GREP_LINES)
            .map(|l| {
                format!(
                    "{l}
"
                )
            })
            .collect();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let mut parts = line.splitn(3, ':');
            let item = match (parts.next(), parts.next().map(str::parse::<u32>)) {
                (Some(path), Some(Ok(n))) => Item::new(path, n),
                // A file listed whole (`--files`, `-l`).
                _ if !line.contains(':') => Item::new(line, 0),
                _ => continue,
            };
            if !items.contains(&item) {
                items.push(item);
            }
        }
        // What the agent wrote and what it got back.
        output.push_str(&format!(
            "rg {} {} {}\n",
            g.args.join(" "),
            g.pattern,
            g.paths.join(" ")
        ));
        output.push_str(&text);
    }
    let flags = good_flags(expect, &items);
    let output_tokens = tokens(&output);
    let read_windows = windows(
        &items[..read_count(expect.len(), &flags)],
        bench.read_radius,
        bench.read_max,
    );
    let read = read_tokens(dir, &read_windows);
    Ok(GrepSide {
        score: score::score(expect, &items),
        cold_ms: (cold * 10.0).round() / 10.0,
        warm_ms: (warm * 10.0).round() / 10.0,
        output_tokens,
        read_tokens: read,
        total_tokens: output_tokens + read,
        calls: searches.len() + read_windows.len(),
        searches: searches.to_vec(),
        results: marks(&items, &flags).into_iter().take(MAX_LIMIT).collect(),
    })
}

/// Who answers better: the one that finds more of what is expected, else the cheaper.
fn winner(i: &IndexSide, g: &GrepSide) -> String {
    let (ri, rg) = (i.at10.recall, g.score.recall);
    if (ri - rg).abs() > 1e-9 {
        return if ri > rg { "index" } else { "grep" }.into();
    }
    // As right as each other: precision, then the tokens.
    let (pi, pg) = (i.at10.precision, g.score.precision);
    if i.real_tokens < g.total_tokens && pi + 1e-9 >= pg {
        "index".into()
    } else if g.total_tokens < i.real_tokens && pg + 1e-9 >= pi {
        "grep".into()
    } else {
        "=".into()
    }
}

fn pct(x: f64) -> String {
    format!("{:.0}", x * 100.0)
}

fn rank(r: Option<usize>) -> String {
    r.map(|n| n.to_string()).unwrap_or_else(|| "—".into())
}

/// The tables of the report.
fn markdown(r: &Report) -> String {
    let mut s = String::new();
    let ix = &r.indexing;
    s.push_str(&format!(
        "Commit `{}`, {}, {} ; lecture pour conclure : ±{} lignes autour des {} premiers résultats.\n\n",
        r.commit, r.date, r.rg, r.read_radius, r.read_max
    ));
    s.push_str("| Indexation | Mesure |\n|---|---|\n");
    s.push_str(&format!(
        "| Dépôt entier | {} ms, {} fichiers, {} symboles, {} liens, {:.1} Mo |\n",
        ix.full_ms,
        ix.files,
        ix.symbols,
        ix.links,
        ix.bytes as f64 / 1_048_576.0
    ));
    s.push_str(&format!(
        "| Delta d'un worktree (2 fichiers changés) | {} ms, {} fichiers relus |\n",
        ix.worktree_ms, ix.worktree_parsed
    ));
    s.push_str(&format!(
        "| Après un commit (1 fichier) | {} ms, {} fichier(s) relu(s) |\n",
        ix.commit_ms, ix.commit_parsed
    ));
    s.push_str(&format!(
        "| Changement de branche (30 commits en arrière, {} fichiers différents) | {} ms, {} fichiers relus |\n",
        ix.branch_files_changed, ix.branch_ms, ix.branch_parsed
    ));
    s.push_str(&format!(
        "| Retour sur la branche | {} ms, {} fichiers relus |\n\n",
        ix.back_ms, ix.back_parsed
    ));
    s.push_str("| Sonde | Outil | Lang. | P@10 | R@10 | Rang@10 | R@50 | Rang@50 | P grep | R grep | Rang grep | Index froid / chaud (ms) | rg (ms) | Tok. réponse (faux) | Tok. lectures index | Tok. grep + lectures | Éco. annoncée | Éco. réelle | Gagnant |\n");
    s.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for p in &r.probes {
        let (i, g) = (&p.index, &p.grep);
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} / {} | {} | {} ({}) | {} | {} + {} | {} % | {} % | {} |\n",
            p.id,
            p.tool.trim_start_matches("index_"),
            p.lang,
            pct(i.at10.precision),
            pct(i.at10.recall),
            rank(i.at10.first_good),
            pct(i.at50.recall),
            rank(i.at50.first_good),
            pct(g.score.precision),
            pct(g.score.recall),
            rank(g.score.first_good),
            i.cold_ms,
            i.warm_ms,
            g.cold_ms,
            i.response_tokens,
            i.wrong_tokens,
            i.read_tokens,
            g.output_tokens,
            g.read_tokens,
            i.announced_percent,
            p.real_percent,
            p.winner
        ));
    }
    s.push_str("\n| Outil | Sondes | P@10 moy. | R@10 moy. | R@50 moy. | P grep moy. | R grep moy. | Index chaud moy. (ms) | rg moy. (ms) | Tok. index (réel) | dont faux | Tok. grep | Éco. annoncée moy. | Éco. réelle (totaux) | Index / grep / = |\n");
    s.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    let tools = [
        "index_search",
        "index_context",
        "index_callers",
        "index_callees",
        "index_impact",
    ];
    for t in tools.iter().map(|t| Some(*t)).chain([None]) {
        let ps: Vec<&Outcome> = r
            .probes
            .iter()
            .filter(|p| t.is_none_or(|t| p.tool == t))
            .collect();
        if ps.is_empty() {
            continue;
        }
        let n = ps.len() as f64;
        let mean = |f: &dyn Fn(&Outcome) -> f64| ps.iter().map(|p| f(p)).sum::<f64>() / n;
        let sum = |f: &dyn Fn(&Outcome) -> u64| ps.iter().map(|p| f(p)).sum::<u64>();
        let real = sum(&|p| p.index.real_tokens);
        let grep = sum(&|p| p.grep.total_tokens);
        let wins = |w: &str| ps.iter().filter(|p| p.winner == w).count();
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {:.1} | {:.1} | {} | {} | {} | {:.0} % | {} % | {} / {} / {} |\n",
            t.map(|t| t.trim_start_matches("index_")).unwrap_or("**Tout**"),
            ps.len(),
            pct(mean(&|p| p.index.at10.precision)),
            pct(mean(&|p| p.index.at10.recall)),
            pct(mean(&|p| p.index.at50.recall)),
            pct(mean(&|p| p.grep.score.precision)),
            pct(mean(&|p| p.grep.score.recall)),
            mean(&|p| p.index.warm_ms),
            mean(&|p| p.grep.warm_ms),
            real,
            sum(&|p| p.index.wrong_tokens),
            grep,
            mean(&|p| p.index.announced_percent as f64),
            economy(real, grep),
            wins("index"),
            wins("grep"),
            wins("=")
        ));
    }
    s.push_str(&tasks_table(r));
    s.push_str(&code_words_table(r));
    s
}

/// The probes « tâche »: the whole path to the answer, the usage that counts.
fn tasks_table(r: &Report) -> String {
    let ps: Vec<&Outcome> = r
        .probes
        .iter()
        .filter(|p| p.tags.iter().any(|t| t == "tâche"))
        .collect();
    if ps.is_empty() {
        return String::new();
    }
    let mut s = String::from("\n### Sondes « tâche »\n\nGrep informé : les motifs écrits en sachant la réponse (huit repris de vrais agents). Grep à l'aveugle : d'abord une recherche faite des seuls mots de la question (en anglais, sans dossier, sans casse), puis la recherche informée si elle n'a rien trouvé.\n\n| Sonde | R@10 index | Rang | Appels index (réel) | Tok. index (réel) | R grep informé | Appels | Tokens | R 1er essai aveugle | Appels aveugle | Tok. aveugle | Éco. / informé | Éco. / aveugle | Gagnant |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    let blind_r = |p: &Outcome| p.blind.as_ref().map(|b| b.score.recall);
    for p in &ps {
        let (i, g) = (&p.index, &p.grep);
        s.push_str(&format!(
            "| {} | {} | {} | {} ({}) | {} ({}) | {} | {} | {} | {} | {} | {} | {} % | {} % | {} |\n",
            p.id,
            pct(i.at10.recall),
            rank(i.at10.first_good),
            i.calls,
            i.real_calls,
            i.total_tokens,
            i.real_tokens,
            pct(g.score.recall),
            g.calls,
            g.total_tokens,
            blind_r(p).map(pct).unwrap_or_else(|| "—".into()),
            p.blind_calls,
            p.blind_tokens,
            p.real_percent,
            economy(i.real_tokens, p.blind_tokens),
            p.winner
        ));
    }
    let n = ps.len() as f64;
    let sum = |f: &dyn Fn(&Outcome) -> u64| ps.iter().map(|p| f(p)).sum::<u64>();
    let real = sum(&|p| p.index.real_tokens);
    let grep = sum(&|p| p.grep.total_tokens);
    let blind = sum(&|p| p.blind_tokens);
    s.push_str(&format!(
        "| **Total ({})** | {} | | {} | {} | {} | {} | {} | {} | {} | {} | {} % | {} % | {} / {} / {} |\n",
        ps.len(),
        pct(ps.iter().map(|p| p.index.at10.recall).sum::<f64>() / n),
        sum(&|p| p.index.real_calls as u64),
        real,
        pct(ps.iter().map(|p| p.grep.score.recall).sum::<f64>() / n),
        sum(&|p| p.grep.calls as u64),
        grep,
        pct(ps.iter().filter_map(|p| blind_r(p)).sum::<f64>() / n),
        sum(&|p| p.blind_calls as u64),
        blind,
        economy(real, grep),
        economy(real, blind),
        ps.iter().filter(|p| p.winner == "index").count(),
        ps.iter().filter(|p| p.winner == "grep").count(),
        ps.iter().filter(|p| p.winner == "=").count()
    ));
    s
}

/// The tasks told in words, as asked and with the code's words (in English), as a tool's
/// description could ask the agent to: a lead with no model.
fn code_words_table(r: &Report) -> String {
    let ps: Vec<&Outcome> = r
        .probes
        .iter()
        .filter(|p| p.index.code_words.is_some())
        .collect();
    if ps.is_empty() {
        return String::new();
    }
    let mut s = String::from("\n### Les mots de la question, ou ceux du code\n\n| Sonde | Question R@10 (rang) | Mots du code R@10 (rang) |\n|---|---|---|\n");
    let cell = |sc: &Score| format!("{} ({})", pct(sc.recall), rank(sc.first_good));
    for p in &ps {
        if let Some(c) = &p.index.code_words {
            s.push_str(&format!(
                "| {} | {} | {} |\n",
                p.id,
                cell(&p.index.at10),
                cell(c)
            ));
        }
    }
    let mean = |task: bool, f: &dyn Fn(&Outcome) -> f64| {
        let v: Vec<f64> = ps
            .iter()
            .filter(|p| p.tags.iter().any(|t| t == "tâche") == task)
            .map(|p| f(p))
            .collect();
        (v.iter().sum::<f64>() / v.len().max(1) as f64, v.len())
    };
    for (name, task) in [("tâche", true), ("autres (fr/en)", false)] {
        let (q, n) = mean(task, &|p| p.index.at10.recall);
        let (c, _) = mean(task, &|p| {
            p.index.code_words.as_ref().map_or(0.0, |c| c.recall)
        });
        s.push_str(&format!("| **{name} ({n})** | {} | {} |\n", pct(q), pct(c)));
    }
    s
}

/// The figures of a run that a later one compares to: per probe, its scores, tokens, calls and
/// times; the indexing.
fn reference_of(r: &Report) -> Value {
    let probes: Vec<Value> = r
        .probes
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "tool": p.tool,
                "tags": p.tags,
                "index": {
                    "at10": p.index.at10,
                    "at50": p.index.at50,
                    "coldMs": p.index.cold_ms,
                    "warmMs": p.index.warm_ms,
                    "responseTokens": p.index.response_tokens,
                    "wrongTokens": p.index.wrong_tokens,
                    "realTokens": p.index.real_tokens,
                    "realCalls": p.index.real_calls,
                    "announcedPercent": p.index.announced_percent,
                    "codeWords": p.index.code_words,
                },
                "grep": {
                    "score": p.grep.score,
                    "totalTokens": p.grep.total_tokens,
                    "calls": p.grep.calls,
                    "blindTokens": p.blind_tokens,
                    "blindCalls": p.blind_calls,
                },
                "realPercent": p.real_percent,
                "winner": p.winner,
            })
        })
        .collect();
    json!({
        "commit": r.commit,
        "date": r.date,
        "readRadius": r.read_radius,
        "readMax": r.read_max,
        "indexing": r.indexing,
        "probes": probes,
    })
}

/// A group of probes of the comparison: its name, and which probes it holds.
type Group<'a> = (&'a str, &'a dyn Fn(&Value) -> bool);

/// A run against a reference: per tool and for the probes « tâche », the recall at 10, the
/// precision and the real tokens, before and after; then the probes whose recall moved.
pub fn compared(before: &Value, after: &Value) -> String {
    let f = |v: &Value, path: &[&str]| -> f64 {
        path.iter().fold(v, |v, k| &v[*k]).as_f64().unwrap_or(0.0)
    };
    let list = |v: &Value| v["probes"].as_array().cloned().unwrap_or_default();
    let (b, a) = (list(before), list(after));
    let pair = |id: &str| b.iter().find(|x| x["id"] == id).cloned();
    let mut s = format!(
        "\n### Face à la référence ({})\n\n| Groupe | Sondes | R@10 avant → après | P@10 avant → après | Tok. index réels avant → après |\n|---|---|---|---|---|\n",
        before["commit"].as_str().unwrap_or("?")
    );
    let groups: [Group; 7] = [
        ("search", &|x| x["tool"] == "index_search"),
        ("context", &|x| x["tool"] == "index_context"),
        ("callers", &|x| x["tool"] == "index_callers"),
        ("callees", &|x| x["tool"] == "index_callees"),
        ("impact", &|x| x["tool"] == "index_impact"),
        ("« tâche »", &|x| {
            x["tags"]
                .as_array()
                .is_some_and(|t| t.iter().any(|t| t == "tâche"))
        }),
        ("**Tout**", &|_| true),
    ];
    for (name, keep) in groups {
        let both: Vec<(Value, Value)> = a
            .iter()
            .filter(|x| keep(x))
            .filter_map(|x| pair(x["id"].as_str().unwrap_or_default()).map(|y| (y, x.clone())))
            .collect();
        if both.is_empty() {
            continue;
        }
        let n = both.len() as f64;
        let mean = |side: usize, path: &[&str]| {
            both.iter()
                .map(|(x, y)| f(if side == 0 { x } else { y }, path))
                .sum::<f64>()
                / n
        };
        let sum = |side: usize, path: &[&str]| {
            both.iter()
                .map(|(x, y)| f(if side == 0 { x } else { y }, path))
                .sum::<f64>()
        };
        s.push_str(&format!(
            "| {} | {} | {} → {} | {} → {} | {:.0} → {:.0} |\n",
            name,
            both.len(),
            pct(mean(0, &["index", "at10", "recall"])),
            pct(mean(1, &["index", "at10", "recall"])),
            pct(mean(0, &["index", "at10", "precision"])),
            pct(mean(1, &["index", "at10", "precision"])),
            sum(0, &["index", "realTokens"]),
            sum(1, &["index", "realTokens"])
        ));
    }
    let moved: Vec<String> = a
        .iter()
        .filter_map(|x| {
            let y = pair(x["id"].as_str()?)?;
            let (r0, r1) = (
                f(&y, &["index", "at10", "recall"]),
                f(x, &["index", "at10", "recall"]),
            );
            ((r0 - r1).abs() > 1e-9).then(|| {
                format!(
                    "{} {} → {}",
                    x["id"].as_str().unwrap_or_default(),
                    pct(r0),
                    pct(r1)
                )
            })
        })
        .collect();
    if !moved.is_empty() {
        s.push_str(&format!("\nRappel changé : {}.\n", moved.join(" ; ")));
    }
    s
}

/// Removes a folder, read-only files too (git's objects under Windows).
fn remove_all(dir: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    fn writable(p: &Path) {
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                let path = e.path();
                if path.is_dir() {
                    writable(&path);
                } else if let Ok(m) = std::fs::metadata(&path) {
                    let mut perm = m.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    perm.set_readonly(false);
                    let _ = std::fs::set_permissions(&path, perm);
                }
            }
        }
    }
    writable(dir);
    std::fs::remove_dir_all(dir)
}
