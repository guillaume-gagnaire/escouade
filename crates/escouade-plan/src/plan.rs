//! What an agent is getting through, worked out from its stream: the task list Claude Code keeps
//! (`TodoWrite`, or `TaskCreate` / `TaskUpdate`), the subagents it launches, the workflows it
//! runs, the commands, monitors and other tasks it leaves running in the background. One
//! reducer, here, fed by `agent.rs` frame by frame; the window only shows the result
//! (`AgentMeta::plan`), it never works it out again. The forms of the frames are in
//! docs/PROTOCOL.md (« Avancée : ce que le flux dit »).

use crate::planfiles::FileList;
use escouade_base::paths::safe_origin;
use escouade_base::text::visible_line;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Arc;

/// The most tasks a plan keeps; the ones beyond are not listed.
pub const MAX_TASKS: usize = 100;
/// The most subagent rows kept: all those running, then the latest finished.
pub const MAX_AGENTS: usize = 30;
/// The most workflow runs kept, the same way.
pub const MAX_WORKFLOWS: usize = 10;
/// The longest title (a task's, a subagent's…), in characters, the ellipsis included.
pub const MAX_TITLE: usize = 160;

/// What a frame changed of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Change {
    /// Nothing the window shows.
    #[default]
    None,
    /// A live detail (what a subagent does now, a workflow's phase): the view goes out, nothing is
    /// saved for it.
    View,
    /// The structure changed (a task, a row opened or closed): the view goes out and is saved.
    Saved,
}

impl Change {
    /// The larger of two changes.
    pub fn and(self, other: Change) -> Change {
        self.max(other)
    }
}

/// Where the task list comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanSource {
    /// The agent's task tools (what the stream tells).
    Tools,
    /// A plan file of its folder (`planfiles`).
    Plan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum RunStatus {
    #[default]
    Running,
    Done,
    Failed,
    Stopped,
    /// The process that ran it is gone (or the turn was stopped) before it ended.
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PlanTask {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    /// What it is doing while in progress (« Running tests »).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    /// Steps done and in all, when a plan file counts them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<(u32, u32)>,
    /// The ids of the tasks it waits for.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    /// The `TaskCreate` call whose result will tell its id.
    #[serde(skip)]
    pub call: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SubAgent {
    /// The id of the call that launched it.
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The model it runs on: the one its messages tell (`claude-haiku-5-5`), once one came,
    /// else the one its call asked for, as it wrote it (`haiku`). Its call may name none (its
    /// agent's definition chooses, or it inherits the agent's).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub status: RunStatus,
    pub background: bool,
    /// Tools it used.
    pub tools: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    /// What it does now (« Édite src/a.ts »), while it runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doing: Option<String>,
    /// The id of the task it works on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_task: Option<String>,
    /// The call of the subagent that launched it, when one did (a subagent's subagent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Claude Code's id of its task, once a frame told it: what « Arrêter » stops (`stop_task`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WorkflowRun {
    /// Claude Code's id of its task.
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub title: String,
    pub status: RunStatus,
    /// The phase and the agent running now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
    pub tools: u32,
    pub tokens: u64,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
}

/// What a task Claude Code runs in the background is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BgKind {
    /// A command (`Bash`, `PowerShell`) run in the background.
    #[default]
    Command,
    /// The `Monitor` tool: a command whose output lines are events for the agent.
    Monitor,
    /// An agent running remotely.
    Remote,
    /// An in-process teammate.
    Teammate,
}

/// A task Claude Code runs in the background for the agent, other than a subagent or a workflow
/// (they have rows of their own): a command, a monitor, a remote agent, a teammate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BgTask {
    /// Claude Code's id of the task (what `stop_task` and `get_task_output` name).
    pub id: String,
    pub kind: BgKind,
    /// What the tool said it was for (« Ticker »), not the command.
    pub title: String,
    /// The command line as the tool received it, `MAX_COMMAND` characters at most.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub status: RunStatus,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    /// Claude Code's last words on it (« Background command "Ticker" completed (exit code 0) »);
    /// for one stopped, only its description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The id of the call that launched it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call: Option<String>,
    /// The task of the subagent that owns it, when one does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Where Claude Code writes its output (`<task>.output`), once a frame said it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file: Option<String>,
}

/// What brings an agent back by itself later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WakeKind {
    /// A job of its session (`CronCreate`), until it fires (once) or is deleted (`CronDelete`).
    #[default]
    Cron,
    /// The next wake-up of a dynamic `/loop` (`ScheduleWakeup`): one at a time.
    Loop,
}

/// A time Claude Code will start a turn of the agent by itself, as the result of the call that
/// asked for it says (ESC-138): nothing runs meanwhile, and its process must live until then.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Wake {
    /// The job's id (`CronCreate`), `loop` for the wake-up of a loop.
    pub id: String,
    pub kind: WakeKind,
    /// When it fires (the next time, for a recurring job), in ms.
    pub at: i64,
    /// A recurring job: it goes on after `at`.
    pub recurring: bool,
    /// A job Claude Code wrote down, which outlives its process.
    pub durable: bool,
    /// The job's schedule as Claude Code tells it (« 17 8 22 10 * »).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    /// What it will do: the job's prompt, the reason of a loop's wake-up.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// An agent of Escouade the agent drives through its MCP server (ESC-138): one it wrote to or
/// created (`agent`, its id), or the ticket whose agent it started, sent back or resumed
/// (`ticket`, as the call named it: a key or an id). Their work goes on in Escouade, after the
/// agent's turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Driven {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket: Option<String>,
    /// When it was last driven, in ms.
    pub at: i64,
}

/// The most wake-ups a plan keeps, and agents or tickets driven: the latest.
pub const MAX_WAKES: usize = 10;
pub const MAX_DRIVEN: usize = 20;

/// The tools of Escouade's MCP server that set another agent to work, and whether they name it by
/// its ticket (else the answer gives its id).
const DRIVING_TOOLS: &[(&str, bool)] = &[
    ("send_message", false),
    ("create_agent", false),
    ("start_ticket", true),
    ("resume_ticket", true),
    ("reject_ticket", true),
    ("resolve_conflict", true),
];

/// How Claude Code names the tools of Escouade's MCP server.
const ESCOUADE_TOOLS: &str = "mcp__escouade__";

/// Whether `tool` is one of `DRIVING_TOOLS`, and if it names a ticket.
fn driving(tool: &str) -> Option<bool> {
    let name = tool.strip_prefix(ESCOUADE_TOOLS)?;
    DRIVING_TOOLS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, ticket)| *ticket)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PlanState {
    pub source: Option<PlanSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub tasks: Vec<PlanTask>,
    pub agents: Vec<SubAgent>,
    /// Every subagent launched since the plan was last reset, those whose row went included.
    pub launched: u32,
    pub workflows: Vec<WorkflowRun>,
    /// The tasks running in the background (commands, monitors, remote agents, teammates), then
    /// the latest ended: `MAX_BG` at most.
    pub bg_tasks: Vec<BgTask>,
    /// The times Claude Code will start a turn by itself (`MAX_WAKES` at most). Kept over a new
    /// plan: they are still to come.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub wakes: Vec<Wake>,
    /// The agents of Escouade it drives, the latest last (`MAX_DRIVEN` at most). Kept over a new
    /// plan and the end of its process: they work on in Escouade.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub driven: Vec<Driven>,
    /// The latest calls looked at: a frame that comes twice counts once.
    // Public (as `memory`) for the app to write a plan as a literal, `..Default::default()` taking the rest.
    #[serde(skip)]
    pub seen: Seen,
    /// What the frames of background tasks need of the ones before: the calls of commands, and the
    /// commands that run in the foreground.
    #[serde(skip)]
    pub memory: BgMemory,
}

/// How many calls `Seen` remembers: a frame that comes twice comes close to itself.
const SEEN: usize = 512;

/// The latest calls a plan looked at. Not part of what the plan says (two plans are equal whatever
/// they have seen), and not copied with it: the window's view copies the plan at each frame, and
/// the copies share the one list instead of each carrying 512 ids.
#[derive(Clone, Default)]
pub struct Seen(Arc<Mutex<VecDeque<String>>>);

impl std::fmt::Debug for Seen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Seen({})", self.0.lock().len())
    }
}

impl PartialEq for Seen {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Seen {
    /// `call` is met for the first time.
    fn first_time(&self, call: &str) -> bool {
        let mut ring = self.0.lock();
        if ring.iter().any(|s| s == call) {
            return false;
        }
        if ring.len() >= SEEN {
            ring.pop_front();
        }
        ring.push_back(call.to_string());
        true
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.lock().len()
    }

    /// Both are the same list (tests: a copy of a plan does not copy it).
    #[cfg(test)]
    pub(crate) fn same_ring(&self, other: &Seen) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
/// How many calls of commands `BgMemory` remembers.
const MAX_CALLS: usize = 64;
/// How many commands in the foreground `BgMemory` remembers.
const MAX_FOREGROUND: usize = 32;

/// A call of a command or of a monitor: what its task's frames do not say (a task's description
/// is the tool's, not the command; the tool tells apart a monitor from a command).
#[derive(Debug, Clone, Default)]
struct Call {
    id: String,
    tool: String,
    command: Option<String>,
    description: Option<String>,
    /// For a call that leaves something to happen later (`later_result`): what it names (a job's
    /// id, a ticket, an agent) or what it asks (a job's prompt, a wake-up's reason).
    target: Option<String>,
    /// `ScheduleWakeup` with `stop`.
    stop: bool,
}

#[derive(Debug, Default)]
struct BgRings {
    calls: VecDeque<Call>,
    /// Tasks seen starting in the foreground, until they end, go to the background, or the turn
    /// ends: not rows (nothing shows them), but one that goes to the background keeps its start.
    foreground: VecDeque<BgTask>,
}

/// What the plan remembers of the calls and tasks it does not show. Like `Seen`: not part of what
/// the plan says (two plans are equal whatever they remember), not copied with it (the window's
/// view copies the plan at each frame; the copies share the lists), and not saved.
#[derive(Clone, Default)]
pub struct BgMemory(Arc<Mutex<BgRings>>);

impl std::fmt::Debug for BgMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rings = self.0.lock();
        write!(
            f,
            "BgMemory({} calls, {} foreground)",
            rings.calls.len(),
            rings.foreground.len()
        )
    }
}

impl PartialEq for BgMemory {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl BgMemory {
    /// A call of `Bash`, `PowerShell` or `Monitor` (any thread): its command and description, kept
    /// until its task says it started. A call met again replaces itself.
    fn remember_call(&self, id: &str, name: &str, input: &Value) {
        let target = match name {
            "Bash" | "PowerShell" | "Monitor" => None,
            "CronCreate" => words(&input["prompt"]),
            "CronDelete" => words(&input["id"]),
            "ScheduleWakeup" => words(&input["reason"]).or_else(|| words(&input["prompt"])),
            _ => match driving(name) {
                Some(true) => words(&input["ticket"]),
                Some(false) => None,
                None => return,
            },
        };
        let call = Call {
            id: id.to_string(),
            tool: name.to_string(),
            command: command_text(&input["command"]),
            description: words(&input["description"]),
            target,
            stop: input["stop"] == true,
        };
        let mut rings = self.0.lock();
        if let Some(known) = rings.calls.iter_mut().find(|c| c.id == id) {
            *known = call;
            return;
        }
        if rings.calls.len() >= MAX_CALLS {
            rings.calls.pop_front();
        }
        rings.calls.push_back(call);
    }

    fn call(&self, id: &str) -> Option<Call> {
        if id.is_empty() {
            return None;
        }
        self.0.lock().calls.iter().find(|c| c.id == id).cloned()
    }

    /// A task that started in the foreground.
    fn keep_foreground(&self, task: BgTask) {
        let mut rings = self.0.lock();
        rings.foreground.retain(|t| t.id != task.id);
        if rings.foreground.len() >= MAX_FOREGROUND {
            rings.foreground.pop_front();
        }
        rings.foreground.push_back(task);
    }

    /// The foreground task `id`, forgotten: it ended, or it goes to the background.
    fn take_foreground(&self, id: &str) -> Option<BgTask> {
        let mut rings = self.0.lock();
        let at = rings.foreground.iter().position(|t| t.id == id)?;
        rings.foreground.remove(at)
    }

    fn in_foreground(&self, id: &str) -> bool {
        self.0.lock().foreground.iter().any(|t| t.id == id)
    }

    fn set_foreground_title(&self, id: &str, title: &str) {
        if let Some(t) = self.0.lock().foreground.iter_mut().find(|t| t.id == id) {
            t.title = title.to_string();
        }
    }

    fn clear_foreground(&self) {
        self.0.lock().foreground.clear();
    }

    #[cfg(test)]
    pub(crate) fn calls_len(&self) -> usize {
        self.0.lock().calls.len()
    }

    #[cfg(test)]
    pub(crate) fn foreground_len(&self) -> usize {
        self.0.lock().foreground.len()
    }

    /// Both are the same lists (tests: a copy of a plan does not copy them).
    #[cfg(test)]
    pub(crate) fn same_ring(&self, other: &BgMemory) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// The longest command a background task keeps, in characters, the ellipsis included.
pub const MAX_COMMAND: usize = 2000;
/// The most background tasks a plan keeps ended: the latest.
pub const MAX_BG_FINISHED: usize = 10;
/// The most background tasks a plan keeps: all those running, then the latest ended.
pub const MAX_BG: usize = 40;

/// The command of a call as the tool received it, cut to `MAX_COMMAND` characters with an
/// ellipsis; None when there is none.
fn command_text(v: &Value) -> Option<String> {
    let text = v.as_str().filter(|c| !c.trim().is_empty())?;
    if text.chars().count() <= MAX_COMMAND {
        return Some(text.to_string());
    }
    let cut: String = text.chars().take(MAX_COMMAND - 1).collect();
    Some(format!("{cut}…"))
}

/// The most tasks one lists as the ones it waits for.
const MAX_BLOCKERS: usize = 20;

/// `text` as a title is kept: one visible line (`visible_line`), `MAX_TITLE` characters at most.
pub(crate) fn clean(text: &str) -> String {
    let line = visible_line(text);
    if line.chars().count() <= MAX_TITLE {
        return line;
    }
    let cut: String = line.chars().take(MAX_TITLE - 1).collect();
    format!("{}…", cut.trim_end())
}

/// `title` in words only: lowercase, letters and digits, single spaces.
fn plain_words(title: &str) -> String {
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The agent's task titled `mine` is the plan's `task`: the same title, or the form the skills
/// give to their todos (« Task 2: Deux »), or a longer title that holds the other whole when that
/// is long enough to say something.
fn same_task(mine: &str, task: &PlanTask) -> bool {
    let (mine, plan) = (plain_words(mine), plain_words(&task.title));
    if mine.is_empty() || plan.is_empty() {
        return false;
    }
    if mine == plan || mine == plain_words(&format!("task {} {}", task.id, task.title)) {
        return true;
    }
    let (short, long) = if mine.len() <= plan.len() {
        (mine, plan)
    } else {
        (plan, mine)
    };
    short.len() >= 8 && format!(" {long} ").contains(&format!(" {short} "))
}

/// A text of a call, cleaned; None when it says nothing.
fn words(v: &Value) -> Option<String> {
    v.as_str().map(clean).filter(|s| !s.is_empty())
}

/// A task's id as a call gives it: a string, or a number a model wrote.
fn task_id(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(clean(s)),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
    .filter(|id| !id.is_empty())
}

/// The id Claude Code gives a task it created, from the result of `TaskCreate`:
/// « Task #<id> created successfully: <subject> ».
fn created_id(text: &str) -> Option<String> {
    let rest = text.trim_start().strip_prefix("Task #")?;
    let (id, tail) = rest.split_at(rest.find(char::is_whitespace)?);
    tail.trim_start()
        .starts_with("created successfully")
        .then(|| clean(id))
        .filter(|id| !id.is_empty())
}

/// The ids `text` names by the files of the superpowers workspace: `task-<id>-brief.md`,
/// `task-<id>-report.md`.
fn file_ids(text: &str) -> Vec<&str> {
    text.match_indices("task-")
        .filter_map(|(i, found)| {
            let rest = &text[i + found.len()..];
            let end = rest
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(rest.len());
            let tail = &rest[end..];
            (end > 0 && (tail.starts_with("-brief.md") || tail.starts_with("-report.md")))
                .then(|| &rest[..end])
        })
        .collect()
}

/// The ids `text` names in words: « Task 3 », « task #3 », « Task L1 ».
fn word_ids(text: &str) -> Vec<&str> {
    let b = text.as_bytes();
    let mut ids = Vec::new();
    let mut i = 0;
    while i + 4 <= b.len() {
        // `b"task"` is ASCII: wherever it matches is a character boundary.
        if !b[i..i + 4].eq_ignore_ascii_case(b"task") || (i > 0 && b[i - 1].is_ascii_alphanumeric())
        {
            i += 1;
            continue;
        }
        let mut j = i + 4;
        let spaces = j;
        while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
            j += 1;
        }
        if j > spaces {
            if b.get(j) == Some(&b'#') {
                j += 1;
            }
            let start = j;
            while j < b.len() && b[j].is_ascii_alphanumeric() {
                j += 1;
            }
            if j > start {
                ids.push(&text[start..j]);
            }
        }
        i += 4;
    }
    ids
}

/// How a task of Claude Code's says it is.
fn task_status(v: &Value) -> Option<TaskStatus> {
    match v.as_str()? {
        "pending" => Some(TaskStatus::Pending),
        "in_progress" => Some(TaskStatus::InProgress),
        "completed" => Some(TaskStatus::Done),
        _ => None,
    }
}

/// The row ends as `status` says, if it is still running (an ended row is never opened again).
fn end_agent(row: &mut SubAgent, status: RunStatus, at: i64) -> bool {
    if row.status != RunStatus::Running {
        return false;
    }
    row.status = status;
    row.ended_at = Some(at);
    row.doing = None;
    true
}

fn end_workflow(run: &mut WorkflowRun, status: RunStatus, at: i64) -> bool {
    if run.status != RunStatus::Running {
        return false;
    }
    run.status = status;
    run.ended_at = Some(at);
    run.now = None;
    true
}

fn end_bg(task: &mut BgTask, status: RunStatus, at: i64) -> bool {
    if task.status != RunStatus::Running {
        return false;
    }
    task.status = status;
    task.ended_at = Some(at);
    true
}

/// How a background task ended, as its notification says; None for anything else (a monitor's
/// event, replayed, has no status: the task goes on).
fn bg_ending(status: Option<&str>) -> Option<RunStatus> {
    match status? {
        "completed" => Some(RunStatus::Done),
        "failed" => Some(RunStatus::Failed),
        "stopped" | "killed" => Some(RunStatus::Stopped),
        _ => None,
    }
}

/// What a task of `task_type` is, among those kept as background tasks; `tool` is the tool of the
/// call that launched it, when known (a monitor is a `local_bash` task, only its tool tells).
fn bg_kind(task_type: &str, tool: Option<&str>) -> Option<BgKind> {
    match task_type {
        "local_bash" if tool == Some("Monitor") => Some(BgKind::Monitor),
        "local_bash" => Some(BgKind::Command),
        "monitor_mcp" => Some(BgKind::Monitor),
        "remote_agent" => Some(BgKind::Remote),
        "in_process_teammate" => Some(BgKind::Teammate),
        _ => None,
    }
}

/// The first line of `text` that says something, as a title.
fn first_line(text: &str) -> String {
    text.lines()
        .map(clean)
        .find(|l| !l.is_empty())
        .unwrap_or_default()
}

/// The longest file path a result is believed to name.
const MAX_PATH: usize = 1000;

/// The file a result says a task's output is written to: « Output is being written to: <path> »,
/// when the file is named after the task (`<task>.output`), the path ending there (at the end of
/// the text, at a space, or at a full stop that ends the sentence). Never another file name: the
/// file will be read. Nor a path to another machine (`safe_origin`: a share, a verbatim path, a
/// device), which Windows would open over the network.
fn output_path(text: &str, task: &str) -> Option<String> {
    const MARK: &str = "Output is being written to:";
    let line = text[text.find(MARK)? + MARK.len()..].lines().next()?;
    let line = line.trim_start();
    let name = format!("{task}.output");
    let mut from = 0;
    while let Some(found) = line[from..].find(&name) {
        let start = from + found;
        let end = start + name.len();
        from = end;
        let after = &line[end..];
        let boundary = after.is_empty()
            || after.starts_with(char::is_whitespace)
            || after
                .strip_prefix('.')
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace));
        let named = line[..start].ends_with(['/', '\\']);
        if boundary && named {
            let path = &line[..end];
            return (path.len() <= MAX_PATH && safe_origin(path)).then(|| path.to_string());
        }
    }
    None
}

/// The totals Claude Code gives of a subagent's work replace the count made on the way. Kept
/// without a word: each progress frame would send the window the whole agent, and they go out
/// with the next change of what it shows (what the subagent does, which changes with each tool it
/// calls), soon enough for the head of its view.
fn set_totals(row: &mut SubAgent, tools: Option<u64>, tokens: Option<u64>) {
    if let Some(n) = tools.map(|n| n.min(u32::MAX as u64) as u32) {
        row.tools = n;
    }
    if let Some(t) = tokens {
        row.tokens = Some(t);
    }
}

impl PlanState {
    /// Nothing to show.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
            && self.agents.is_empty()
            && self.workflows.is_empty()
            && self.bg_tasks.is_empty()
            && self.launched == 0
            && self.wakes.is_empty()
            && self.driven.is_empty()
    }

    /// The wake-ups still to come at `now`: a one-shot one whose time passed has fired.
    pub fn pending_wakes(&self, now: i64) -> impl Iterator<Item = &Wake> {
        self.wakes.iter().filter(move |w| w.recurring || w.at > now)
    }

    /// Its process must live on: something runs in it (`runs_something`), or Claude Code will
    /// start a turn by itself (`pending_wakes`), which a stopped process never would.
    pub fn holds_process(&self, now: i64) -> bool {
        self.runs_something() || self.pending_wakes(now).next().is_some()
    }

    /// A plan with nothing in it, that goes on with what another remembered of the calls and of the
    /// foreground tasks (the agent keeps those while it has no plan to hold them).
    pub fn with_memory(memory: BgMemory) -> Self {
        Self {
            memory,
            ..Self::default()
        }
    }

    /// Nothing left to do nor to wait for: every task done (or none), nothing running.
    pub fn is_finished(&self) -> bool {
        self.tasks.iter().all(|t| t.status == TaskStatus::Done)
            && self.agents.iter().all(|a| a.status != RunStatus::Running)
            && self
                .workflows
                .iter()
                .all(|w| w.status != RunStatus::Running)
            && self.bg_tasks.iter().all(|t| t.status != RunStatus::Running)
    }

    /// A subagent, a workflow or a command is running: in the background, it goes on after the
    /// turn, in the process (which a restart would kill).
    pub fn runs_something(&self) -> bool {
        self.running_count() > 0
    }

    /// How many subagents, workflows and background tasks (commands, monitors…) run: the rows
    /// that a restart of the agent's process would kill, counted (`runs_something` is whether
    /// there are any).
    pub fn running_count(&self) -> usize {
        self.running_subagents() + self.running_background()
    }

    /// How many subagents run, in the foreground or the background.
    pub fn running_subagents(&self) -> usize {
        self.agents
            .iter()
            .filter(|a| a.status == RunStatus::Running)
            .count()
    }

    /// How many tasks run in the background, as the window counts them (« N en fond »): the
    /// commands, monitors, remote agents and teammates of `bg_tasks`, and the workflows. Never the
    /// subagents, which have their own list (`running_subagents`).
    pub fn running_background(&self) -> usize {
        let running = |status: RunStatus| status == RunStatus::Running;
        self.workflows.iter().filter(|w| running(w.status)).count()
            + self.bg_tasks.iter().filter(|t| running(t.status)).count()
    }

    /// Forgets everything (a new conversation, a new plan), but what is still to come outside the
    /// plan: the wake-ups, the agents it drives.
    pub fn reset(&mut self) -> Change {
        let wakes = std::mem::take(&mut self.wakes);
        let driven = std::mem::take(&mut self.driven);
        let had = !self.is_empty() || self.title.is_some() || self.plan_file.is_some();
        *self = Self {
            wakes,
            driven,
            ..Self::default()
        };
        if had {
            Change::Saved
        } else {
            Change::None
        }
    }

    /// What the plan file and the ledger of the agent's folder say (`planfiles`), or none when
    /// they could not be read (what was known stays).
    ///
    /// The agent's own task list, when it has one, is the list: the files then only name the plan
    /// and give its title. Without one, the tasks are the files'. A task the ledger said done is
    /// not pending again (the ledger was cut or rewritten); the ledger itself can reopen it.
    pub fn set_files(&mut self, list: Option<FileList>) -> Change {
        let Some(list) = list else {
            return Change::None;
        };
        // Names that come from a file are shown: one visible line, as every title.
        let file = Some(clean(&list.plan_file)).filter(|f| !f.is_empty());
        let title = list.title.as_deref().map(clean).filter(|t| !t.is_empty());
        if file.is_none() {
            return Change::None;
        }
        if self.source == Some(PlanSource::Tools) && !self.tasks.is_empty() {
            // The plan names the agent's list only if it is that plan's: a task of the list is
            // one of the plan's. (Agents of one repository read the same folder: the plan of
            // another is not this one's.)
            let ours = list
                .tasks
                .iter()
                .any(|plan| self.tasks.iter().any(|mine| same_task(&mine.title, plan)));
            let (file, title) = if ours { (file, title) } else { (None, None) };
            if self.plan_file == file && self.title == title {
                return Change::None;
            }
            self.plan_file = file;
            self.title = title;
            return Change::Saved;
        }
        if list.tasks.is_empty() {
            // A plan without tasks has nothing to show; what was read of an earlier one goes.
            return self.forget_files();
        }
        let same_plan = self.source == Some(PlanSource::Plan) && self.plan_file == file;
        let tasks: Vec<PlanTask> = list
            .tasks
            .into_iter()
            .map(|mut task| {
                let was_done = same_plan
                    && self
                        .tasks
                        .iter()
                        .any(|t| t.id == task.id && t.status == TaskStatus::Done);
                if was_done && task.status == TaskStatus::Pending {
                    task.status = TaskStatus::Done;
                }
                task
            })
            .collect();
        if self.source == Some(PlanSource::Plan)
            && self.tasks == tasks
            && self.plan_file == file
            && self.title == title
        {
            return Change::None;
        }
        self.source = Some(PlanSource::Plan);
        self.tasks = tasks;
        self.plan_file = file;
        self.title = title;
        self.drop_links();
        Change::Saved
    }

    /// The run of the plan the files gave ended (the agent removed its workspace, as the skills
    /// do once the final review is clean): every task is done.
    pub fn finish_files(&mut self) -> Change {
        if self.source != Some(PlanSource::Plan) {
            return Change::None;
        }
        let mut change = Change::None;
        for task in self
            .tasks
            .iter_mut()
            .filter(|t| t.status != TaskStatus::Done)
        {
            task.status = TaskStatus::Done;
            change = Change::Saved;
        }
        change
    }

    /// Nothing of the files is shown: the tasks they gave, the plan's name and title.
    fn forget_files(&mut self) -> Change {
        let had = self.source == Some(PlanSource::Plan)
            || self.plan_file.is_some()
            || self.title.is_some();
        if self.source == Some(PlanSource::Plan) {
            self.tasks.clear();
            self.source = None;
            self.drop_links();
        }
        self.plan_file = None;
        self.title = None;
        if had {
            Change::Saved
        } else {
            Change::None
        }
    }

    /// The agent has a list of its own now: the one read from the files makes room, and so does
    /// the plan's name and title (the next look gives them back if the agent's tasks are the
    /// plan's).
    fn take_over(&mut self) -> Change {
        if self.source != Some(PlanSource::Plan) {
            return Change::None;
        }
        self.tasks.clear();
        self.source = None;
        self.plan_file = None;
        self.title = None;
        self.drop_links();
        Change::Saved
    }

    /// The process that ran what is running is gone: those rows are interrupted. The tasks keep
    /// their status (the agent picks them up again).
    pub fn close_running(&mut self, now: i64) -> Change {
        let mut change = self.end_agents(now, |_| true);
        for run in &mut self.workflows {
            if end_workflow(run, RunStatus::Interrupted, now) {
                change = Change::Saved;
            }
        }
        for task in &mut self.bg_tasks {
            if end_bg(task, RunStatus::Interrupted, now) {
                change = Change::Saved;
            }
        }
        // The jobs of its session went with it; a durable one is on disk, for the next.
        let wakes = self.wakes.len();
        self.wakes.retain(|w| w.durable);
        if self.wakes.len() != wakes {
            change = Change::Saved;
        }
        self.memory.clear_foreground();
        change
    }

    /// A turn ended: a subagent in the foreground cannot be running any more (it was stopped), nor
    /// a command. One in the background goes on, and so does a workflow.
    pub fn end_turn(&mut self, now: i64) -> Change {
        self.memory.clear_foreground();
        self.end_agents(now, |a| !a.background)
    }

    /// Interrupts the running subagents that `of` accepts.
    fn end_agents(&mut self, now: i64, of: impl Fn(&SubAgent) -> bool) -> Change {
        let mut change = Change::None;
        for a in self.agents.iter_mut().filter(|a| of(a)) {
            if end_agent(a, RunStatus::Interrupted, now) {
                change = Change::Saved;
            }
        }
        change
    }

    /// A call seen for the first time (a frame that comes twice counts once).
    fn first_time(&mut self, call: &str) -> bool {
        self.seen.first_time(call)
    }

    /// A tool call of the assistant, `parent` being the call of the subagent it comes from.
    pub fn on_tool_use(
        &mut self,
        parent: Option<&str>,
        id: &str,
        name: &str,
        input: &Value,
        now: i64,
    ) -> Change {
        // The command (and the tool) of a task that starts later, from any thread: remembered
        // before anything can return, and not a change.
        self.memory.remember_call(id, name, input);
        let agent_call = matches!(name, "Task" | "Agent");
        // Only the main thread keeps the task list.
        let list_call =
            parent.is_none() && matches!(name, "TodoWrite" | "TaskCreate" | "TaskUpdate");
        let of_row = parent.filter(|p| self.agents.iter().any(|a| a.id == *p));
        if !agent_call && !list_call && of_row.is_none() {
            return Change::None;
        }
        if !self.first_time(id) {
            return Change::None;
        }
        // A tool the subagent uses, while it runs: counted without a word (the window does not
        // need to hear of each one; what the subagent does, `on_child_activity`, tells when it
        // changes).
        if let Some(row) = of_row.and_then(|p| self.agents.iter_mut().find(|a| a.id == p)) {
            if row.status == RunStatus::Running {
                row.tools += 1;
            }
        }
        match name {
            _ if agent_call => self.open_agent(parent, id, name, input, now),
            "TodoWrite" if list_call => self.todo_write(input),
            "TaskCreate" if list_call => self.task_create(id, input),
            "TaskUpdate" if list_call => self.task_update(input),
            _ => Change::None,
        }
    }

    /// What a subagent does now, from the tool it just called.
    pub fn on_child_activity(&mut self, parent: &str, activity: Option<&str>) -> Change {
        let Some(activity) = activity.map(clean).filter(|a| !a.is_empty()) else {
            return Change::None;
        };
        let Some(row) = self
            .agents
            .iter_mut()
            .find(|a| a.id == parent && a.status == RunStatus::Running)
        else {
            return Change::None;
        };
        if row.doing.as_deref() == Some(activity.as_str()) {
            return Change::None;
        }
        row.doing = Some(activity);
        Change::View
    }

    /// The model a message of the subagent `parent` came from: the one it runs on, whatever its
    /// call asked for. Claude Code's own messages (`<synthetic>`) tell nothing.
    pub fn on_child_model(&mut self, parent: &str, model: &str) -> Change {
        if !self.tells_model(parent, model) {
            return Change::None;
        }
        if let Some(row) = self.agents.iter_mut().find(|a| a.id == parent) {
            row.model = Some(clean(model));
        }
        Change::Saved
    }

    /// `model` is news for the row of the subagent `parent` (`on_child_model`): the stream asks
    /// before each message, for those that tell nothing new to leave the plan as it is.
    pub fn tells_model(&self, parent: &str, model: &str) -> bool {
        let model = clean(model);
        !model.is_empty()
            && !model.starts_with('<')
            && self
                .agents
                .iter()
                .any(|a| a.id == parent && a.model.as_deref() != Some(model.as_str()))
    }

    /// The result of a tool call: `text` its text, `tool_use_result` the frame's structured one.
    pub fn on_tool_result(
        &mut self,
        tool_use_id: &str,
        is_error: bool,
        text: &str,
        tool_use_result: &Value,
        now: i64,
    ) -> Change {
        self.bg_result(tool_use_id, is_error, text, tool_use_result, now)
            .and(self.call_result(tool_use_id, is_error, text, tool_use_result, now))
            .and(self.later_result(tool_use_id, is_error, text, tool_use_result, now))
    }

    /// The result of a call that leaves something to happen after the turn, outside the tasks
    /// Claude Code tells (ESC-138): a job of its session (`CronCreate`, `CronDelete`), the next
    /// wake-up of a loop (`ScheduleWakeup`), an agent of Escouade set to work (`DRIVING_TOOLS`).
    /// A failed call leaves nothing.
    fn later_result(
        &mut self,
        call_id: &str,
        is_error: bool,
        text: &str,
        tur: &Value,
        now: i64,
    ) -> Change {
        if is_error {
            return Change::None;
        }
        let Some(call) = self.memory.call(call_id) else {
            return Change::None;
        };
        match call.tool.as_str() {
            "CronCreate" => {
                let (Some(id), Some(at)) = (words(&tur["id"]), tur["nextFireAt"].as_i64()) else {
                    return Change::None;
                };
                self.add_wake(
                    Wake {
                        id,
                        kind: WakeKind::Cron,
                        at,
                        recurring: tur["recurring"] == true,
                        durable: tur["durable"] == true,
                        schedule: words(&tur["humanSchedule"]),
                        prompt: call.target,
                    },
                    now,
                )
            }
            "CronDelete" => {
                let before = self.wakes.len();
                self.wakes
                    .retain(|w| w.kind != WakeKind::Cron || Some(&w.id) != call.target.as_ref());
                if self.wakes.len() == before {
                    Change::None
                } else {
                    Change::Saved
                }
            }
            "ScheduleWakeup" => {
                let at = tur["scheduledFor"].as_i64().filter(|at| *at > 0);
                match at {
                    Some(at) if !call.stop && tur["stopped"] != true => self.add_wake(
                        Wake {
                            id: "loop".into(),
                            kind: WakeKind::Loop,
                            at,
                            prompt: call.target,
                            ..Wake::default()
                        },
                        now,
                    ),
                    // None armed: the loop goes on with the one it had, if any.
                    None if !call.stop && tur["stopped"] != true => Change::None,
                    _ => {
                        let before = self.wakes.len();
                        self.wakes.retain(|w| w.kind != WakeKind::Loop);
                        if self.wakes.len() == before {
                            Change::None
                        } else {
                            Change::Saved
                        }
                    }
                }
            }
            tool => {
                let Some(by_ticket) = driving(tool) else {
                    return Change::None;
                };
                let answer: Value = serde_json::from_str(text).unwrap_or(Value::Null);
                let driven = if by_ticket {
                    let ticket = words(&answer["key"]).or(call.target);
                    Driven {
                        agent: None,
                        ticket,
                        at: now,
                    }
                } else {
                    Driven {
                        agent: words(&answer["id"]),
                        ticket: None,
                        at: now,
                    }
                };
                if driven.agent.is_none() && driven.ticket.is_none() {
                    return Change::None;
                }
                self.driven
                    .retain(|d| d.agent != driven.agent || d.ticket != driven.ticket);
                self.driven.push(driven);
                if self.driven.len() > MAX_DRIVEN {
                    let over = self.driven.len() - MAX_DRIVEN;
                    self.driven.drain(..over);
                }
                Change::Saved
            }
        }
    }

    /// `wake` is to come: it replaces the one of the same id (a loop has one), and those that
    /// fired are forgotten.
    fn add_wake(&mut self, wake: Wake, now: i64) -> Change {
        self.wakes
            .retain(|w| w.id != wake.id && (w.recurring || w.durable || w.at > now));
        self.wakes.push(wake);
        if self.wakes.len() > MAX_WAKES {
            let over = self.wakes.len() - MAX_WAKES;
            self.wakes.drain(..over);
        }
        Change::Saved
    }

    /// What the result of a call is to the task list and to the subagents.
    fn call_result(
        &mut self,
        tool_use_id: &str,
        is_error: bool,
        text: &str,
        tool_use_result: &Value,
        now: i64,
    ) -> Change {
        // The id of a task comes with the result of the call that made it.
        if let Some(i) = self
            .tasks
            .iter()
            .position(|t| t.call.as_deref() == Some(tool_use_id))
        {
            if is_error {
                self.tasks.remove(i);
                self.drop_links();
            } else {
                let task = &mut self.tasks[i];
                task.call = None;
                if let Some(id) = created_id(text) {
                    task.id = id;
                }
            }
            return Change::Saved;
        }
        let Some(row) = self.agents.iter_mut().find(|a| a.id == tool_use_id) else {
            return Change::None;
        };
        let tur = tool_use_result;
        set_totals(
            row,
            tur["totalToolUseCount"].as_u64(),
            tur["totalTokens"].as_u64(),
        );
        if row.status != RunStatus::Running {
            // Ended already (its task said): the totals only complete it.
            return Change::None;
        }
        // In the background, the result only says it started; its end comes as a notification.
        if tur["status"] == "async_launched"
            || text.trim_start().starts_with("Async agent launched")
        {
            let mut change = Change::None;
            if !row.background {
                row.background = true;
                change = Change::Saved;
            }
            if row.task_id.is_none() {
                row.task_id = tur["agentId"].as_str().map(str::to_string);
            }
            return change;
        }
        let status = if is_error {
            RunStatus::Failed
        } else {
            RunStatus::Done
        };
        let at = match tur["totalDurationMs"].as_u64() {
            Some(ms) => row
                .started_at
                .saturating_add(ms.min(i64::MAX as u64) as i64),
            None => now,
        };
        end_agent(row, status, at);
        Change::Saved
    }

    /// A `system` frame about tasks.
    pub fn on_system(&mut self, f: &Value, now: i64) -> Change {
        match f["subtype"].as_str().unwrap_or_default() {
            "task_started" => self.task_started(f, now),
            "task_updated" => self.task_updated(f, now),
            "task_notification" => self.task_notification(f, now),
            "task_progress" => self.task_progress(f),
            "background_tasks_changed" => self.bg_level(f, now),
            _ => Change::None,
        }
    }

    // ---------- the task list ----------

    /// `TodoWrite`: the whole list, which replaces the one before.
    fn todo_write(&mut self, input: &Value) -> Change {
        let Some(todos) = input["todos"].as_array() else {
            return Change::None;
        };
        let tasks: Vec<PlanTask> = todos
            .iter()
            .filter_map(|todo| {
                Some(PlanTask {
                    title: words(&todo["content"])?,
                    status: task_status(&todo["status"]).unwrap_or_default(),
                    active: words(&todo["activeForm"]),
                    ..PlanTask::default()
                })
            })
            .take(MAX_TASKS)
            .enumerate()
            .map(|(rank, task)| PlanTask {
                id: (rank + 1).to_string(),
                ..task
            })
            .collect();
        if self.source == Some(PlanSource::Plan) {
            // An empty list says nothing the files do not.
            if tasks.is_empty() {
                return Change::None;
            }
            self.take_over();
        }
        if tasks == self.tasks {
            return Change::None;
        }
        if !tasks.is_empty() {
            self.source = Some(PlanSource::Tools);
        }
        self.tasks = tasks;
        self.drop_links();
        Change::Saved
    }

    /// `TaskCreate`: listed at once, its id known with the result of `call`.
    fn task_create(&mut self, call: &str, input: &Value) -> Change {
        let Some(title) = words(&input["subject"]) else {
            return Change::None;
        };
        let mut change = self.take_over();
        // The list was finished: this is another one.
        if !self.tasks.is_empty() && self.tasks.iter().all(|t| t.status == TaskStatus::Done) {
            self.tasks.clear();
            self.drop_links();
            change = Change::Saved;
        }
        if self.tasks.len() >= MAX_TASKS {
            return change;
        }
        self.source = Some(PlanSource::Tools);
        self.tasks.push(PlanTask {
            id: call.to_string(),
            title,
            active: words(&input["activeForm"]),
            call: Some(call.to_string()),
            ..PlanTask::default()
        });
        Change::Saved
    }

    /// `TaskUpdate`: by the id of its task. A task it does not know is made, as Claude Code does.
    fn task_update(&mut self, input: &Value) -> Change {
        let Some(id) = task_id(&input["taskId"]) else {
            return Change::None;
        };
        // The tasks of the files are not the ones this call is about.
        let mut taken = Change::None;
        if self.source == Some(PlanSource::Plan) {
            if input["status"] == "deleted" {
                return Change::None;
            }
            taken = self.take_over();
        }
        let at = self.tasks.iter().position(|t| t.id == id);
        if input["status"] == "deleted" {
            let Some(i) = at else {
                return Change::None;
            };
            self.tasks.remove(i);
            for t in &mut self.tasks {
                t.blocked_by.retain(|b| *b != id);
            }
            self.drop_links();
            return Change::Saved;
        }
        let (i, mut changed) = match at {
            Some(i) => (i, false),
            None if self.tasks.len() >= MAX_TASKS => return Change::None,
            None => {
                self.source = Some(PlanSource::Tools);
                self.tasks.push(PlanTask {
                    title: format!("#{id}"),
                    id: id.clone(),
                    ..PlanTask::default()
                });
                (self.tasks.len() - 1, true)
            }
        };
        let task = &mut self.tasks[i];
        if let Some(title) = words(&input["subject"]) {
            if task.title != title {
                task.title = title;
                changed = true;
            }
        }
        if let Some(status) = task_status(&input["status"]) {
            if task.status != status {
                task.status = status;
                changed = true;
            }
        }
        if let Some(active) = input["activeForm"].as_str().map(clean) {
            let active = Some(active).filter(|a| !a.is_empty());
            if task.active != active {
                task.active = active;
                changed = true;
            }
        }
        for blocker in input["addBlockedBy"].as_array().into_iter().flatten() {
            changed |= block(task, task_id(blocker));
        }
        for blocked in input["addBlocks"].as_array().into_iter().flatten() {
            // Said from the other end: that task waits for this one.
            if let Some(other) = task_id(blocked).and_then(|b| {
                self.tasks
                    .iter()
                    .position(|t| t.id == b)
                    .filter(|&j| j != i)
            }) {
                changed |= block(&mut self.tasks[other], Some(id.clone()));
            }
        }
        if changed {
            Change::Saved
        } else {
            taken
        }
    }

    /// A subagent works on no task a list no longer has.
    fn drop_links(&mut self) {
        for a in &mut self.agents {
            if a.plan_task
                .as_ref()
                .is_some_and(|t| !self.tasks.iter().any(|x| x.id == *t))
            {
                a.plan_task = None;
            }
        }
    }

    // ---------- subagents ----------

    /// The task a subagent launched with `prompt` and `description` works on: the id its prompt or
    /// its description names (`task-<id>-brief.md` first, then `Task <id>`) if the list has it;
    /// else the one task in progress, if there is only one.
    fn link_task(&self, prompt: &str, description: &str) -> Option<String> {
        let has = |id: &&str| self.tasks.iter().any(|t| t.id == **id);
        let named = [file_ids, word_ids]
            .iter()
            .flat_map(|ids| [prompt, description].into_iter().flat_map(|t| ids(t)))
            .find(has);
        if let Some(id) = named {
            return Some(id.to_string());
        }
        let mut doing = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::InProgress);
        match (doing.next(), doing.next()) {
            (Some(task), None) => Some(task.id.clone()),
            _ => None,
        }
    }

    /// A `Task` / `Agent` call: a subagent starts (or one its `task_started` opened is completed).
    /// `parent`: the call of the subagent whose thread it comes from.
    fn open_agent(
        &mut self,
        parent: Option<&str>,
        id: &str,
        name: &str,
        input: &Value,
        now: i64,
    ) -> Change {
        let description = input["description"].as_str().unwrap_or_default();
        let (kind, model) = (words(&input["subagent_type"]), words(&input["model"]));
        let background = input["run_in_background"] == true;
        // The prompt is read to find the task, never kept here (its call's item keeps it).
        let plan_task = self.link_task(input["prompt"].as_str().unwrap_or_default(), description);
        if let Some(row) = self.agents.iter_mut().find(|a| a.id == id) {
            let mut change = Change::None;
            if row.kind.is_none() && kind.is_some() {
                row.kind = kind;
                change = Change::Saved;
            }
            if row.model.is_none() && model.is_some() {
                row.model = model;
                change = Change::Saved;
            }
            if background && !row.background {
                row.background = true;
                change = Change::Saved;
            }
            if row.plan_task.is_none() && plan_task.is_some() {
                row.plan_task = plan_task;
                change = Change::Saved;
            }
            if row.parent.is_none() && parent.is_some() {
                row.parent = parent.map(str::to_string);
                change = Change::Saved;
            }
            return change;
        }
        let title = [
            &input["description"],
            &input["name"],
            &input["subagent_type"],
        ]
        .into_iter()
        .find_map(words)
        .unwrap_or_else(|| name.to_string());
        self.launched += 1;
        self.agents.push(SubAgent {
            id: id.to_string(),
            title,
            kind,
            model,
            background,
            started_at: now,
            plan_task,
            parent: parent.map(str::to_string),
            ..SubAgent::default()
        });
        self.trim_agents();
        Change::Saved
    }

    /// All the rows running, then the latest ended: the oldest ended go first.
    fn trim_agents(&mut self) {
        while self.agents.len() > MAX_AGENTS {
            let at = self
                .agents
                .iter()
                .position(|a| a.status != RunStatus::Running)
                .unwrap_or(0);
            self.agents.remove(at);
        }
    }

    fn trim_workflows(&mut self) {
        while self.workflows.len() > MAX_WORKFLOWS {
            let at = self
                .workflows
                .iter()
                .position(|w| w.status != RunStatus::Running)
                .unwrap_or(0);
            self.workflows.remove(at);
        }
    }

    // ---------- the system frames ----------

    fn task_started(&mut self, f: &Value, now: i64) -> Change {
        let Some(task) = f["task_id"].as_str().filter(|t| !t.is_empty()) else {
            return Change::None;
        };
        // Not what the conversation does: a task Claude Code runs on its own account.
        if f["ambient"] == true || f["skip_transcript"] == true {
            return Change::None;
        }
        match f["task_type"].as_str() {
            Some("local_agent") => self.agent_started(task, f, now),
            Some("local_workflow") => self.workflow_started(task, f, now),
            Some(other) => self.bg_started(task, other, f, now),
            None => Change::None,
        }
    }

    fn agent_started(&mut self, task: &str, f: &Value, now: i64) -> Change {
        let call = f["tool_use_id"].as_str().filter(|c| !c.is_empty());
        let id = call.unwrap_or(task);
        let background = f["is_backgrounded"] == true;
        if let Some(row) = self
            .agents
            .iter_mut()
            .find(|a| a.id == id || a.task_id.as_deref() == Some(task))
        {
            row.task_id.get_or_insert_with(|| task.to_string());
            if background && !row.background {
                row.background = true;
                return Change::Saved;
            }
            return Change::None;
        }
        // Its call was not seen (it will complete the row if it comes).
        let description = f["description"].as_str().unwrap_or_default();
        let kind = words(&f["subagent_type"]);
        let title = [&f["description"], &f["subagent_type"]]
            .into_iter()
            .find_map(words)
            .unwrap_or_else(|| "Agent".to_string());
        let plan_task = self.link_task(f["prompt"].as_str().unwrap_or_default(), description);
        // Launched by a subagent: the task of that one names its row.
        let parent = f["parent_task_id"].as_str().and_then(|t| {
            self.agents
                .iter()
                .find(|a| a.task_id.as_deref() == Some(t))
                .map(|a| a.id.clone())
        });
        self.launched += 1;
        self.agents.push(SubAgent {
            id: id.to_string(),
            title,
            kind,
            background,
            started_at: now,
            plan_task,
            parent,
            task_id: Some(task.to_string()),
            ..SubAgent::default()
        });
        self.trim_agents();
        Change::Saved
    }

    fn workflow_started(&mut self, task: &str, f: &Value, now: i64) -> Change {
        if self.workflows.iter().any(|w| w.id == task) {
            return Change::None;
        }
        let name = words(&f["workflow_name"]);
        let title = words(&f["description"])
            .or_else(|| name.clone())
            .unwrap_or_default();
        self.workflows.push(WorkflowRun {
            id: task.to_string(),
            name,
            title,
            started_at: now,
            ..WorkflowRun::default()
        });
        self.trim_workflows();
        Change::Saved
    }

    fn task_updated(&mut self, f: &Value, now: i64) -> Change {
        let Some(task) = f["task_id"].as_str() else {
            return Change::None;
        };
        let patch = &f["patch"];
        let status = match patch["status"].as_str() {
            Some("completed") => Some(RunStatus::Done),
            Some("failed") => Some(RunStatus::Failed),
            Some("killed") => Some(RunStatus::Stopped),
            // Still going (or waiting): nothing ended.
            _ => None,
        };
        if let Some(row) = self
            .agents
            .iter_mut()
            .find(|a| a.task_id.as_deref() == Some(task))
        {
            let mut change = Change::None;
            if patch["is_backgrounded"] == true && !row.background {
                row.background = true;
                change = Change::Saved;
            }
            if status.is_some_and(|s| end_agent(row, s, now)) {
                change = Change::Saved;
            }
            return change;
        }
        if let Some(run) = self.workflows.iter_mut().find(|w| w.id == task) {
            return match status {
                Some(status) if end_workflow(run, status, now) => Change::Saved,
                _ => Change::None,
            };
        }
        self.bg_updated(task, patch, status, now)
    }

    fn task_notification(&mut self, f: &Value, now: i64) -> Change {
        let task = f["task_id"].as_str().unwrap_or_default();
        let call = f["tool_use_id"].as_str().unwrap_or_default();
        let status = match f["status"].as_str() {
            Some("failed") => RunStatus::Failed,
            Some("stopped" | "killed") => RunStatus::Stopped,
            _ => RunStatus::Done,
        };
        let by_task = |a: &SubAgent| !task.is_empty() && a.task_id.as_deref() == Some(task);
        let by_call = |a: &SubAgent| !call.is_empty() && a.id == call;
        let found = self
            .agents
            .iter()
            .position(by_task)
            .or_else(|| self.agents.iter().position(by_call));
        if let Some(i) = found {
            let row = &mut self.agents[i];
            if !task.is_empty() {
                row.task_id.get_or_insert_with(|| task.to_string());
            }
            let usage = &f["usage"];
            set_totals(
                row,
                usage["tool_uses"].as_u64(),
                usage["total_tokens"].as_u64(),
            );
            return if end_agent(row, status, now) {
                Change::Saved
            } else {
                Change::None
            };
        }
        match self.workflows.iter_mut().find(|w| w.id == task) {
            Some(run) => {
                if end_workflow(run, status, now) {
                    Change::Saved
                } else {
                    Change::None
                }
            }
            None => self.bg_notified(task, f, now),
        }
    }

    /// A workflow tells the phase and the agent running now, and its totals so far.
    fn task_progress(&mut self, f: &Value) -> Change {
        let Some(task) = f["task_id"].as_str() else {
            return Change::None;
        };
        let usage = &f["usage"];
        let (tools, tokens) = (usage["tool_uses"].as_u64(), usage["total_tokens"].as_u64());
        if let Some(run) = self
            .workflows
            .iter_mut()
            .find(|w| w.id == task && w.status == RunStatus::Running)
        {
            let mut change = Change::None;
            if let Some(now) = words(&f["description"]) {
                if run.now.as_ref() != Some(&now) {
                    run.now = Some(now);
                    change = Change::View;
                }
            }
            if let Some(n) = tools.map(|n| n.min(u32::MAX as u64) as u32) {
                if run.tools != n {
                    run.tools = n;
                    change = Change::View;
                }
            }
            if let Some(t) = tokens {
                if run.tokens != t {
                    run.tokens = t;
                    change = Change::View;
                }
            }
            return change;
        }
        // A subagent has a progress frame only when asked for summaries: its totals, then.
        match self
            .agents
            .iter_mut()
            .find(|a| a.task_id.as_deref() == Some(task) && a.status == RunStatus::Running)
        {
            Some(row) => {
                set_totals(row, tools, tokens);
                Change::None
            }
            None => Change::None,
        }
    }
}

// ---------- tasks in the background ----------

impl PlanState {
    /// Fills in what a call says of the task (`call`, `command`, and `parent`), where the row has
    /// not got it yet; true when something was added.
    fn complete_bg(row: &mut BgTask, call_id: Option<&str>, call: Option<&Call>) -> bool {
        let mut changed = false;
        if row.call.is_none() {
            if let Some(id) = call_id.filter(|c| !c.is_empty()) {
                row.call = Some(id.to_string());
                changed = true;
            }
        }
        if row.command.is_none() {
            if let Some(command) = call.and_then(|c| c.command.clone()) {
                row.command = Some(command);
                changed = true;
            }
        }
        changed
    }

    /// The title of a task: its own description, else the one its call gave the tool, else the first
    /// line of its command, else its id.
    fn bg_title(description: &Value, call: Option<&Call>, task: &str) -> String {
        words(description)
            .or_else(|| call.and_then(|c| c.description.clone()))
            .or_else(|| {
                call.and_then(|c| c.command.as_deref())
                    .map(first_line)
                    .filter(|l| !l.is_empty())
            })
            .unwrap_or_else(|| clean(task))
    }

    /// The finished rows beyond `MAX_BG_FINISHED` go (the one that ended longest ago first), then
    /// the rows beyond `MAX_BG` (a finished one before any that runs, else the oldest).
    fn trim_bg(&mut self) {
        let oldest_finished = |rows: &[BgTask]| {
            rows.iter()
                .enumerate()
                .filter(|(_, t)| t.status != RunStatus::Running)
                .min_by_key(|(_, t)| t.ended_at.unwrap_or(t.started_at))
                .map(|(i, _)| i)
        };
        while self
            .bg_tasks
            .iter()
            .filter(|t| t.status != RunStatus::Running)
            .count()
            > MAX_BG_FINISHED
        {
            let Some(at) = oldest_finished(&self.bg_tasks) else {
                break;
            };
            self.bg_tasks.remove(at);
        }
        while self.bg_tasks.len() > MAX_BG {
            let at = oldest_finished(&self.bg_tasks).unwrap_or(0);
            self.bg_tasks.remove(at);
        }
    }

    /// The `task_started` of a task of `task_type`, which may be one of those kept here. In the
    /// foreground (`is_backgrounded` false, or none for a command), it is not listed, only
    /// remembered, for it may be sent to the background (`task_updated`).
    fn bg_started(&mut self, task: &str, task_type: &str, f: &Value, now: i64) -> Change {
        let call_id = f["tool_use_id"].as_str().filter(|c| !c.is_empty());
        let call = call_id.and_then(|c| self.memory.call(c));
        let Some(kind) = bg_kind(task_type, call.as_ref().map(|c| c.tool.as_str())) else {
            return Change::None;
        };
        let parent = f["parent_task_id"]
            .as_str()
            .filter(|p| !p.is_empty())
            .map(str::to_string);
        // A command is in the background only if it says so; the others are, unless they say not.
        let background = match task_type {
            "local_bash" => f["is_backgrounded"] == true,
            _ => f["is_backgrounded"] != false,
        };
        let title = Self::bg_title(&f["description"], call.as_ref(), task);
        if let Some(row) = self.bg_tasks.iter_mut().find(|t| t.id == task) {
            // Opened by the level that comes before the task, or by the result of its call.
            let mut changed = Self::complete_bg(row, call_id, call.as_ref());
            // The level took a monitor for a command; the call of its task knows better.
            if row.kind == BgKind::Command && kind == BgKind::Monitor {
                row.kind = kind;
                changed = true;
            }
            if row.parent.is_none() && parent.is_some() {
                row.parent = parent;
                changed = true;
            }
            if row.title.is_empty() {
                row.title = title;
                changed = true;
            }
            return if changed { Change::Saved } else { Change::None };
        }
        let row = BgTask {
            id: task.to_string(),
            kind,
            title,
            command: call.as_ref().and_then(|c| c.command.clone()),
            status: RunStatus::Running,
            started_at: now,
            call: call_id.map(str::to_string),
            parent,
            ..BgTask::default()
        };
        if !background {
            self.memory.keep_foreground(row);
            return Change::None;
        }
        self.bg_tasks.push(row);
        self.trim_bg();
        Change::Saved
    }

    /// A task of the foreground goes to the background: its row opens, from when it started.
    fn promote_bg(&mut self, task: &str) -> bool {
        let Some(row) = self.memory.take_foreground(task) else {
            return false;
        };
        self.bg_tasks.push(row);
        self.trim_bg();
        true
    }

    /// The `task_updated` of a task that is neither a subagent nor a workflow: sent to the
    /// background, renamed, ended. A task of the foreground that ends is forgotten without a word.
    fn bg_updated(
        &mut self,
        task: &str,
        patch: &Value,
        status: Option<RunStatus>,
        now: i64,
    ) -> Change {
        if self.memory.in_foreground(task) {
            if patch["is_backgrounded"] == true {
                // Its description may have changed before.
                if let Some(title) = words(&patch["description"]) {
                    self.memory.set_foreground_title(task, &title);
                }
                return if self.promote_bg(task) {
                    Change::Saved
                } else {
                    Change::None
                };
            }
            if status.is_some() {
                self.memory.take_foreground(task);
            } else if let Some(title) = words(&patch["description"]) {
                self.memory.set_foreground_title(task, &title);
            }
            return Change::None;
        }
        let Some(row) = self.bg_tasks.iter_mut().find(|t| t.id == task) else {
            return Change::None;
        };
        let mut change = Change::None;
        if let Some(title) = words(&patch["description"]).filter(|t| *t != row.title) {
            row.title = title;
            change = Change::Saved;
        }
        if status.is_some_and(|s| end_bg(row, s, now)) {
            change = Change::Saved;
        }
        if change == Change::Saved {
            self.trim_bg();
        }
        change
    }

    /// The `task_notification` of a task that is neither a subagent nor a workflow. It ends the row
    /// only by a status that ends a task (the events of a monitor, replayed, have none: they end
    /// nothing), and tells its words and its file. One of the foreground is forgotten.
    fn bg_notified(&mut self, task: &str, f: &Value, now: i64) -> Change {
        if task.is_empty() {
            return Change::None;
        }
        let ending = bg_ending(f["status"].as_str());
        if self.memory.in_foreground(task) {
            if ending.is_some() {
                self.memory.take_foreground(task);
            }
            return Change::None;
        }
        let (Some(ending), Some(row)) = (ending, self.bg_tasks.iter_mut().find(|t| t.id == task))
        else {
            return Change::None;
        };
        let mut changed = end_bg(row, ending, now);
        if let Some(summary) = words(&f["summary"]).filter(|s| row.summary.as_ref() != Some(s)) {
            row.summary = Some(summary);
            changed = true;
        }
        // The file will be read: never one of another machine (`safe_origin`), as the frame says it.
        let file = f["output_file"]
            .as_str()
            .filter(|p| !p.is_empty() && safe_origin(p));
        if let Some(file) = file.filter(|p| row.output_file.as_deref() != Some(*p)) {
            row.output_file = Some(file.to_string());
            changed = true;
        }
        if changed {
            self.trim_bg();
            Change::Saved
        } else {
            Change::None
        }
    }

    /// `background_tasks_changed`: the tasks Claude Code runs in the background now, all of them
    /// (a level, not an edge). A task of a kept kind that no row has opens one; a running row the
    /// level has not got any more is over, whatever lost notification should have said.
    fn bg_level(&mut self, f: &Value, now: i64) -> Change {
        let Some(tasks) = f["tasks"].as_array() else {
            return Change::None;
        };
        let mut change = Change::None;
        let mut alive: Vec<&str> = Vec::new();
        for entry in tasks {
            let Some(task) = entry["task_id"].as_str().filter(|t| !t.is_empty()) else {
                continue;
            };
            alive.push(task);
            let Some(task_type) = entry["task_type"].as_str() else {
                continue;
            };
            if entry["ambient"] == true || self.bg_tasks.iter().any(|t| t.id == task) {
                continue;
            }
            // Listed as in the background: a task of the foreground sent there, before its update.
            if self.promote_bg(task) {
                change = Change::Saved;
                continue;
            }
            // The level cannot tell a monitor from a command: its task does (`bg_started`).
            let Some(kind) = bg_kind(task_type, None) else {
                continue;
            };
            self.bg_tasks.push(BgTask {
                id: task.to_string(),
                kind,
                title: Self::bg_title(&entry["description"], None, task),
                status: RunStatus::Running,
                started_at: now,
                parent: entry["parent_task_id"]
                    .as_str()
                    .filter(|p| !p.is_empty())
                    .map(str::to_string),
                ..BgTask::default()
            });
            self.trim_bg();
            change = Change::Saved;
        }
        for row in &mut self.bg_tasks {
            if !alive.contains(&row.id.as_str()) && end_bg(row, RunStatus::Done, now) {
                change = Change::Saved;
            }
        }
        change
    }

    /// The result of a call, for the tasks it started: a command sent to the background says its
    /// task (`backgroundTaskId`), a monitor too (`taskId`, for a call of `Monitor` only). This is
    /// all the start of a task there is in the Claude Code view, which has no `system` frames. The
    /// text tells the file the output is written to, when it is the task's own (`output_path`).
    fn bg_result(
        &mut self,
        call_id: &str,
        is_error: bool,
        text: &str,
        tur: &Value,
        now: i64,
    ) -> Change {
        if is_error {
            return Change::None;
        }
        let call = self.memory.call(call_id);
        let monitor = call.as_ref().is_some_and(|c| c.tool == "Monitor");
        let task = tur["backgroundTaskId"]
            .as_str()
            .or_else(|| monitor.then(|| tur["taskId"].as_str()).flatten())
            .filter(|t| !t.is_empty());
        let Some(task) = task else {
            return Change::None;
        };
        let file = output_path(text, task);
        if let Some(row) = self.bg_tasks.iter_mut().find(|t| t.id == task) {
            let mut changed = Self::complete_bg(row, Some(call_id), call.as_ref());
            if row.output_file.is_none() && file.is_some() {
                row.output_file = file;
                changed = true;
            }
            return if changed { Change::Saved } else { Change::None };
        }
        // A command of the foreground that goes to the background by its result alone.
        if !self.promote_bg(task) {
            let kind = if monitor {
                BgKind::Monitor
            } else {
                BgKind::Command
            };
            self.bg_tasks.push(BgTask {
                id: task.to_string(),
                kind,
                title: Self::bg_title(&Value::Null, call.as_ref(), task),
                status: RunStatus::Running,
                started_at: now,
                ..BgTask::default()
            });
        }
        if let Some(row) = self.bg_tasks.iter_mut().find(|t| t.id == task) {
            Self::complete_bg(row, Some(call_id), call.as_ref());
            if row.output_file.is_none() {
                row.output_file = file;
            }
        }
        self.trim_bg();
        Change::Saved
    }
}

/// `task` waits for `blocker`, if it is a new one (a task does not wait for itself).
fn block(task: &mut PlanTask, blocker: Option<String>) -> bool {
    match blocker {
        Some(b)
            if b != task.id
                && !task.blocked_by.contains(&b)
                && task.blocked_by.len() < MAX_BLOCKERS =>
        {
            task.blocked_by.push(b);
            true
        }
        _ => false,
    }
}
