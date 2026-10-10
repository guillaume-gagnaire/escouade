//! The plan an agent follows and the ledger it keeps, read from its folder (superpowers 6.4.1:
//! `writing-plans`, `executing-plans`, `subagent-driven-development`), for the agent that has no
//! task list of its own to show. Read-only, inside the repository of the agent and nowhere else
//! (`paths::contained`), bounded in size; the forms read are in docs/PROTOCOL.md (« Avancée : ce
//! que le flux dit »). The parsers are pure; `discover`, `fallback` and `scan` read the disk and
//! are made to run off the agent's lock.

use crate::mcp::visible_line;
use crate::paths;
use crate::plan::{clean, PlanTask, TaskStatus, MAX_TASKS};
use std::collections::BTreeSet;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// The most bytes read of a file; what is past it is not read. A ledger bigger than that gives
/// its two ends, half each: its first line names the plan, its last lines say how far it is.
pub const MAX_READ: usize = 256 * 1024;

/// Where the superpowers skills keep what a plan's execution leaves, in the repository.
const SDD: &str = ".superpowers/sdd";
/// Where `writing-plans` writes a plan, and the only place a plan the agent wrote is read from
/// (without a ledger that names it, the plan can be any file of the repository).
const PLANS: &str = "docs/superpowers/plans/";
/// The most workspaces listed, and of those the most opened, newest first.
const MAX_WORKSPACES: usize = 256;
const MAX_TRIED: usize = 8;
/// The most entries of a workspace looked at for briefs.
const MAX_ENTRIES: usize = 4096;
/// The most bytes read of a line that names a plan (a ledger's first, a marker).
const MAX_HEAD: usize = 4096;

/// A plan file as `parse_plan` reads it: no status yet (every task is pending), its steps counted.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParsedPlan {
    pub title: Option<String>,
    pub tasks: Vec<PlanTask>,
}

/// A ledger (`progress.md`) as `parse_ledger` reads it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ledger {
    /// The plan its first line names.
    pub plan: Option<String>,
    /// The tasks whose latest line about their progress says `complete`.
    pub done: BTreeSet<String>,
    /// The tasks with a line of work (`dispatched`, `fix round`, `review`…) and no `complete` after.
    pub active: BTreeSet<String>,
}

/// What a scan saw of the files, to tell the next one whether anything moved: the files read with
/// their date and size, and the briefs of the workspace.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stamp {
    files: Vec<(PathBuf, i64, u64)>,
    briefs: BTreeSet<String>,
}

/// Where the plan of an agent was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The ledger, when the plan was found through one.
    pub ledger: Option<PathBuf>,
    pub plan: PathBuf,
    /// The plan, relative to the repository, with forward slashes.
    pub plan_rel: String,
    /// The ids of the `task-<id>-brief.md` files of the workspace.
    pub briefs: BTreeSet<String>,
    /// When the agent last worked on it, in milliseconds: its ledger and briefs for a workspace,
    /// the plan itself for one the agent wrote.
    pub modified: i64,
    pub stamp: Stamp,
}

/// What `plan::PlanState::set_files` takes: the plan's name, title and tasks, with the status the
/// ledger gives them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileList {
    pub plan_file: String,
    pub title: Option<String>,
    pub tasks: Vec<PlanTask>,
}

/// What a scan found.
#[derive(Debug, Clone, PartialEq)]
pub enum Scan {
    /// No plan of this agent's, or one that could not be read.
    Nothing,
    /// The files are as the last scan saw them.
    Unchanged,
    /// The files, read.
    Read(Stamp, FileList),
}

// ---------- the text of a plan ----------

fn without_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// A code fence opens (or closes) with a run of at least three backticks or tildes: which one,
/// and how long. A line of backticks with more of them after the run is inline code.
fn fence_run(line: &str) -> Option<(char, usize)> {
    let text = line.trim_start();
    let c = text.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let run = text.chars().take_while(|x| *x == c).count();
    let inline = c == '`' && text[run..].contains('`');
    (run >= 3 && !inline).then_some((c, run))
}

/// The fence `open` ends with this line: only its character, at least as many times.
fn closes_fence(line: &str, (c, run): (char, usize)) -> bool {
    let text = line.trim();
    text.chars().count() >= run && text.chars().all(|x| x == c)
}

/// An ATX heading (`# …` to `###### …`, at the start of the line): its level and its text, without
/// the closing hashes.
fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    let rest = line.get(level..)?;
    if !(1..=6).contains(&level) || !rest.starts_with([' ', '\t']) {
        return None;
    }
    let text = rest.trim();
    let bare = text.trim_end_matches('#');
    let closed = bare.len() < text.len() && (bare.is_empty() || bare.ends_with([' ', '\t']));
    Some((level, if closed { bare.trim_end() } else { text }))
}

/// `Task <id><sep> <title>` in a heading of level 2 to 4: the id (letters, digits, one letter
/// more: `7`, `L1`, `3b`) and the title.
fn task_heading(level: usize, text: &str) -> Option<(String, String)> {
    if !(2..=4).contains(&level) {
        return None;
    }
    let rest = text.strip_prefix("Task")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    let letters = rest.chars().take_while(char::is_ascii_alphabetic).count();
    let digits = rest[letters..]
        .chars()
        .take_while(char::is_ascii_digit)
        .count();
    if digits == 0 {
        return None;
    }
    let mut end = letters + digits;
    if rest[end..].starts_with(|c: char| c.is_ascii_alphabetic()) {
        end += 1;
    }
    let title = rest[end..]
        .trim_start()
        .strip_prefix([':', '.', '—', '–', '-'])?
        .trim();
    (!title.is_empty()).then(|| (rest[..end].to_string(), title.to_string()))
}

/// A line that is a step: `- [ ] …`, `* [x] …`, `+ [X] …` (indented or not); whether it is checked.
fn step_box(line: &str) -> Option<bool> {
    let text = line.trim_start().strip_prefix(['-', '*', '+'])?;
    let text = text.strip_prefix([' ', '\t'])?.trim_start();
    let text = text.strip_prefix('[')?;
    let mark = text.chars().next()?;
    let after = text[mark.len_utf8()..].strip_prefix(']')?;
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    match mark {
        ' ' => Some(false),
        'x' | 'X' => Some(true),
        _ => None,
    }
}

/// The plan's title: its heading, on one line, without the « Implementation Plan » that
/// `writing-plans` has every title end with (or its French, « plan d'implémentation »), cut at
/// 160 characters.
fn plan_title(text: &str) -> Option<String> {
    let line = visible_line(text);
    let title = clean(without_plan_suffix(&line));
    (!title.is_empty()).then_some(title)
}

fn without_plan_suffix(line: &str) -> &str {
    let is_break = |c: char| c.is_whitespace() || matches!(c, '—' | '–' | '-' | ':' | '|');
    for suffix in [
        "implementation plan",
        "plan d'implémentation",
        "plan d’implémentation",
    ] {
        let keep = line.chars().count().saturating_sub(suffix.chars().count());
        let at = line.char_indices().nth(keep).map_or(line.len(), |(i, _)| i);
        let (head, tail) = line.split_at(at);
        if tail.to_lowercase() != suffix || !(head.is_empty() || head.ends_with(is_break)) {
            continue;
        }
        let head = head.trim_end_matches(is_break);
        // Nothing but the suffix: the title stays as it is written.
        if !head.is_empty() {
            return head;
        }
    }
    line
}

/// The title and the tasks of a plan, by the shapes `writing-plans` writes: the first `# ` is the
/// title; `## Task N: …` to `#### Task N: …` open a task, which lasts until the next heading of its
/// level or above; its steps are the `- [ ]` and `- [x]` boxes of that stretch. Nothing in a code
/// block counts. An id met twice is the first; a plan keeps 100 tasks.
pub fn parse_plan(text: &str) -> ParsedPlan {
    /// The task whose stretch is being read (`at` is none for one that is not kept).
    struct Open {
        at: Option<usize>,
        level: usize,
        done: u32,
        total: u32,
    }
    fn close(plan: &mut ParsedPlan, open: &mut Option<Open>) {
        if let Some(Open {
            at: Some(at),
            done,
            total,
            ..
        }) = open.take()
        {
            if total > 0 {
                plan.tasks[at].steps = Some((done, total));
            }
        }
    }
    let mut plan = ParsedPlan::default();
    let mut fence = None;
    let mut open: Option<Open> = None;
    for line in without_bom(text).lines() {
        if let Some(fence_open) = fence {
            if closes_fence(line, fence_open) {
                fence = None;
            }
            continue;
        }
        if let Some(run) = fence_run(line) {
            fence = Some(run);
            continue;
        }
        if let Some((level, text)) = heading(line) {
            let task = task_heading(level, text);
            if task.is_some() || open.as_ref().is_some_and(|o| level <= o.level) {
                close(&mut plan, &mut open);
            }
            if level == 1 && plan.title.is_none() {
                plan.title = plan_title(text);
            }
            if let Some((id, title)) = task {
                let kept = plan.tasks.len() < MAX_TASKS && !plan.tasks.iter().any(|t| t.id == id);
                let at = kept.then(|| {
                    plan.tasks.push(PlanTask {
                        id,
                        title: clean(&title),
                        ..PlanTask::default()
                    });
                    plan.tasks.len() - 1
                });
                open = Some(Open {
                    at,
                    level,
                    done: 0,
                    total: 0,
                });
            }
        } else if let Some(o) = open.as_mut().filter(|o| o.at.is_some()) {
            if let Some(checked) = step_box(line) {
                o.total += 1;
                o.done += u32::from(checked);
            }
        }
    }
    close(&mut plan, &mut open);
    plan
}

// ---------- the text of a ledger ----------

/// The plan a ledger's first line names: `# SDD ledger — plan: <path>`.
fn ledger_plan(first: &str) -> Option<String> {
    let rest = without_bom(first)
        .trim()
        .strip_prefix('#')?
        .trim_start_matches('#')
        .trim_start();
    const NAME: &str = "SDD ledger";
    if !rest.get(..NAME.len())?.eq_ignore_ascii_case(NAME) {
        return None;
    }
    let (_, value) = rest[NAME.len()..].split_once("plan:")?;
    let value = value.trim();
    let value = ['`', '"', '\'']
        .iter()
        .find_map(|q| value.strip_prefix(*q)?.strip_suffix(*q))
        .unwrap_or(value)
        .trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// A line `Task <id>: <text>`: the id and the text.
fn task_line(line: &str) -> Option<(&str, &str)> {
    let (token, text) = line
        .strip_prefix("Task ")?
        .split_once(char::is_whitespace)?;
    let id = token.strip_suffix(':').filter(|id| !id.is_empty())?;
    Some((id, text.trim()))
}

/// The first word of `text` (letters, digits, hyphens), lowercase, and what follows it.
fn first_word(text: &str) -> (String, &str) {
    let end = text
        .char_indices()
        .find(|(_, c)| !(c.is_alphanumeric() || *c == '-'))
        .map_or(text.len(), |(i, _)| i);
    (text[..end].to_lowercase(), text[end..].trim_start())
}

/// What a line of the ledger says of its task: `Some(true)` it is complete, `Some(false)` work is
/// going on (a task dispatched, an implementer back, a review, a fix round), none for the rest
/// (`Ruling`, `minor (deferred)`, `parked`, free words).
fn progress_of(text: &str) -> Option<bool> {
    let (word, rest) = first_word(text);
    match word.as_str() {
        "complete" | "completed" => Some(true),
        "dispatched" | "implementer" | "review" | "reviewer" | "re-review" => Some(false),
        "fix" => (first_word(rest).0 == "round").then_some(false),
        _ => None,
    }
}

/// A ledger: the plan its first line names, the tasks done and the tasks at work. Of the lines
/// about one task the last that says something wins: a `fix round` after `complete` reopens it,
/// a `complete` after `review` closes it.
pub fn parse_ledger(text: &str) -> Ledger {
    let text = without_bom(text);
    let mut ledger = Ledger {
        plan: text.lines().next().and_then(ledger_plan),
        ..Ledger::default()
    };
    for (id, text) in text.lines().filter_map(task_line) {
        match progress_of(text) {
            Some(true) => {
                ledger.active.remove(id);
                ledger.done.insert(id.to_string());
            }
            Some(false) => {
                ledger.done.remove(id);
                ledger.active.insert(id.to_string());
            }
            None => {}
        }
    }
    ledger
}

// ---------- the disk ----------

/// The start of `path`, `max` bytes at most, without the line the cut falls in.
fn read_head(path: &Path, max: usize) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    let cut = bytes.len() > max;
    bytes.truncate(max);
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if cut {
        if let Some(end) = text.rfind('\n') {
            text.truncate(end + 1);
        }
    }
    Ok(text)
}

/// A ledger: all of it if it fits in `MAX_READ`; else its first half-bound, which holds its first
/// line, and its last, which holds its latest lines, each without the line the cut falls in.
fn read_ledger(path: &Path) -> std::io::Result<String> {
    let len = std::fs::metadata(path)?.len();
    if len <= MAX_READ as u64 {
        return read_head(path, MAX_READ);
    }
    let half = MAX_READ / 2;
    let head = read_head(path, half)?;
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::End(-(half as i64)))?;
    let mut bytes = Vec::new();
    file.take(half as u64).read_to_end(&mut bytes)?;
    let tail = String::from_utf8_lossy(&bytes);
    let tail = tail.split_once('\n').map_or("", |(_, rest)| rest);
    Ok(format!("{head}\n{tail}"))
}

/// The first line of a small file (a ledger, a marker).
fn first_line(path: &Path) -> Option<String> {
    let text = read_head(path, MAX_HEAD).ok()?;
    Some(text.lines().next()?.to_string())
}

fn mtime_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis().min(i64::MAX as u128) as i64)
}

/// `raw`, a path as a ledger, a marker or a tool wrote it, as a path relative to `root` with
/// forward slashes; none for an absolute path that is not inside `root`.
fn relative_to(root: &Path, raw: &str) -> Option<String> {
    let slashed = raw.replace('\\', "/");
    let absolute = slashed.starts_with('/')
        || Path::new(raw).is_absolute()
        || slashed.chars().nth(1) == Some(':');
    if !absolute {
        let mut rel = slashed.as_str();
        while let Some(rest) = rel.strip_prefix("./") {
            rel = rest;
        }
        return Some(rel.to_string());
    }
    let root_text = root.to_string_lossy();
    if let Some(rel) = paths::strip_base(&root_text, raw) {
        return Some(rel);
    }
    // Spelled another way (a short Windows name, a link on the way): both read as the system does.
    let real = |p: &str| {
        std::fs::canonicalize(p)
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    };
    paths::strip_base(&real(&root_text)?, &real(raw)?)
}

/// The plan `raw` names, if it is a Markdown file inside `root` (`paths::contained`: no `..`, no
/// link out of it) and, when `only_plans`, one of the plans folder.
fn resolve_plan(root: &Path, raw: &str, only_plans: bool) -> Option<(PathBuf, String)> {
    let rel = relative_to(root, raw.trim())?;
    let lower = rel.to_ascii_lowercase();
    if !lower.ends_with(".md") || (only_plans && !lower.starts_with(PLANS)) {
        return None;
    }
    let full = paths::contained(root, &rel).ok()?;
    std::fs::metadata(&full)
        .ok()?
        .is_file()
        .then_some((full, rel))
}

/// The id of a `task-<id>-brief.md` file.
fn brief_id(name: &str) -> Option<&str> {
    let id = name.strip_prefix("task-")?.strip_suffix("-brief.md")?;
    (!id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric())).then_some(id)
}

/// The briefs of a workspace (`task-start` and `task-brief` write one at the start of each task)
/// and the date of the latest.
fn briefs_of(dir: &Path) -> (BTreeSet<String>, i64) {
    let mut briefs = BTreeSet::new();
    let mut latest = 0;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (briefs, latest);
    };
    for entry in entries.flatten().take(MAX_ENTRIES) {
        let name = entry.file_name();
        let Some(id) = name.to_str().and_then(brief_id) else {
            continue;
        };
        // A link is not what `task-brief` writes.
        let Some(meta) = entry.metadata().ok().filter(|m| m.is_file()) else {
            continue;
        };
        latest = latest.max(mtime_ms(&meta));
        briefs.insert(id.to_string());
    }
    (briefs, latest)
}

/// The plan of the workspace `name`, if its ledger and its plan can be used.
fn open_workspace(root: &Path, name: &str) -> Option<Found> {
    let rel = format!("{SDD}/{name}");
    let dir = paths::contained(root, &rel).ok()?;
    let ledger = paths::contained(root, &format!("{rel}/progress.md")).ok()?;
    let named = ledger_plan(&first_line(&ledger)?)?;
    // The marker `sdd-workspace` writes is repo-relative: it leads to the plan when the ledger's
    // path was written from another folder.
    let marker = paths::contained(root, &format!("{rel}/plan-path"))
        .ok()
        .and_then(|p| first_line(&p));
    let (plan, plan_rel) = [Some(named), marker]
        .into_iter()
        .flatten()
        .find_map(|raw| resolve_plan(root, &raw, false))?;
    let ledger_meta = std::fs::metadata(&ledger).ok()?;
    let plan_meta = std::fs::metadata(&plan).ok()?;
    let (briefs, latest_brief) = briefs_of(&dir);
    Some(Found {
        modified: mtime_ms(&ledger_meta).max(latest_brief),
        stamp: Stamp {
            files: vec![
                (ledger.clone(), mtime_ms(&ledger_meta), ledger_meta.len()),
                (plan.clone(), mtime_ms(&plan_meta), plan_meta.len()),
            ],
            briefs: briefs.clone(),
        },
        ledger: Some(ledger),
        plan,
        plan_rel,
        briefs,
    })
}

/// The plan of the most recent workspace of `root`: `.superpowers/sdd/<plan>/progress.md` whose
/// first line names a plan (or whose marker does) that is there, inside `root`.
pub fn discover(root: &Path) -> Option<Found> {
    let base = paths::contained(root, SDD).ok()?;
    let mut spaces: Vec<(i64, String)> = std::fs::read_dir(base)
        .ok()?
        .flatten()
        .take(MAX_WORKSPACES)
        .filter_map(|entry| {
            // A real folder with a real file in it: a link is not what `sdd-workspace` makes.
            let name = entry.file_name().into_string().ok()?;
            let is_dir = entry.file_type().ok()?.is_dir();
            let meta = std::fs::symlink_metadata(entry.path().join("progress.md")).ok()?;
            (is_dir && meta.is_file()).then(|| (mtime_ms(&meta), name))
        })
        .collect();
    spaces.sort_by(|a, b| b.cmp(a));
    spaces
        .iter()
        .take(MAX_TRIED)
        .find_map(|(_, name)| open_workspace(root, name))
}

/// Without a ledger: the latest of the plans the agent wrote (`written`, oldest first) that is
/// still there, inside `root`, in the plans folder.
pub fn fallback(root: &Path, written: &[String]) -> Option<Found> {
    written.iter().rev().find_map(|raw| {
        let (plan, plan_rel) = resolve_plan(root, raw, true)?;
        let meta = std::fs::metadata(&plan).ok()?;
        Some(Found {
            ledger: None,
            modified: mtime_ms(&meta),
            stamp: Stamp {
                files: vec![(plan.clone(), mtime_ms(&meta), meta.len())],
                briefs: BTreeSet::new(),
            },
            plan,
            plan_rel,
            briefs: BTreeSet::new(),
        })
    })
}

/// The plan an agent works on that was touched since `since` (milliseconds): a workspace's, else
/// one it wrote. Someone else's, or an old one's, is not the agent's.
fn find(root: &Path, written: &[String], since: i64) -> Option<Found> {
    discover(root)
        .filter(|f| f.modified >= since)
        .or_else(|| fallback(root, written).filter(|f| f.modified >= since))
}

/// The tasks of the plan of `found` with the status its ledger gives them: done when the ledger
/// says `complete`; in progress when it has a line of work with no `complete` after, or when the
/// workspace holds the task's brief (written when the task begins); to do otherwise. A read that
/// fails gives none.
fn read_list(found: &Found) -> Option<FileList> {
    let plan = parse_plan(&read_head(&found.plan, MAX_READ).ok()?);
    let ledger = match &found.ledger {
        Some(path) => parse_ledger(&read_ledger(path).ok()?),
        None => Ledger::default(),
    };
    let tasks = plan
        .tasks
        .into_iter()
        .map(|mut task| {
            task.status = if ledger.done.contains(&task.id) {
                TaskStatus::Done
            } else if ledger.active.contains(&task.id) || found.briefs.contains(&task.id) {
                TaskStatus::InProgress
            } else {
                TaskStatus::Pending
            };
            task
        })
        .collect();
    Some(FileList {
        plan_file: found.plan_rel.clone(),
        title: plan.title,
        tasks,
    })
}

/// Everything read in one go: the plan of `root` and the status of its tasks (tests: the app reads
/// through `scan`).
#[cfg(test)]
pub fn load(root: &Path, written: &[String]) -> Option<FileList> {
    read_list(&find(root, written, 0)?)
}

/// Looks at the folder of an agent: the files are looked at first (date, size, briefs), and read
/// only when they are not as `last` saw them. `since` is when its conversation began: a plan
/// untouched since is not its own.
pub fn scan(root: &Path, written: &[String], since: i64, last: Option<&Stamp>) -> Scan {
    let Some(found) = find(root, written, since) else {
        return Scan::Nothing;
    };
    if last == Some(&found.stamp) {
        return Scan::Unchanged;
    }
    match read_list(&found) {
        Some(list) => Scan::Read(found.stamp, list),
        None => Scan::Nothing,
    }
}
