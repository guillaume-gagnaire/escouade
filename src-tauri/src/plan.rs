//! What an agent is getting through, worked out from its stream: the task list Claude Code keeps
//! (`TodoWrite`, or `TaskCreate` / `TaskUpdate`), the subagents it launches, the workflows it
//! runs. One reducer, here, fed by `agent.rs` frame by frame; the window only shows the result
//! (`AgentMeta::plan`), it never works it out again. The forms of the frames are in
//! docs/PROTOCOL.md (« Avancée : ce que le flux dit »).

use crate::mcp::visible_line;
use crate::planfiles::FileList;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;

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
    /// A live detail (what a subagent does now, its tool count): the view goes out, nothing is
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
    pub(crate) call: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SubAgent {
    /// The id of the call that launched it.
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The model the call asked for, as it wrote it.
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
    /// Claude Code's id of its task, once a frame told it.
    #[serde(skip)]
    pub(crate) task_id: Option<String>,
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
    /// The latest calls looked at: a frame that comes twice counts once.
    #[serde(skip)]
    pub(crate) seen: VecDeque<String>,
}

/// How many calls `PlanState::seen` remembers: a frame that comes twice comes close to itself.
const SEEN: usize = 512;
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

/// The totals Claude Code gives of a subagent's work replace the count made on the way.
fn set_totals(row: &mut SubAgent, tools: Option<u64>, tokens: Option<u64>) -> Change {
    let mut change = Change::None;
    if let Some(n) = tools.map(|n| n.min(u32::MAX as u64) as u32) {
        if row.tools != n {
            row.tools = n;
            change = Change::View;
        }
    }
    if let Some(t) = tokens {
        if row.tokens != Some(t) {
            row.tokens = Some(t);
            change = Change::View;
        }
    }
    change
}

impl PlanState {
    /// Nothing to show.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
            && self.agents.is_empty()
            && self.workflows.is_empty()
            && self.launched == 0
    }

    /// Nothing left to do nor to wait for: every task done (or none), nothing running.
    pub fn is_finished(&self) -> bool {
        self.tasks.iter().all(|t| t.status == TaskStatus::Done)
            && self.agents.iter().all(|a| a.status != RunStatus::Running)
            && self
                .workflows
                .iter()
                .all(|w| w.status != RunStatus::Running)
    }

    /// Forgets everything (a new conversation, a new plan).
    pub fn reset(&mut self) -> Change {
        let had = !self.is_empty() || self.title.is_some() || self.plan_file.is_some();
        *self = Self::default();
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
        let (file, title) = (Some(list.plan_file.clone()), list.title.clone());
        if self.source == Some(PlanSource::Tools) && !self.tasks.is_empty() {
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

    /// The agent has a list of its own now: the one read from the files makes room (the plan
    /// keeps its name and title).
    fn take_over(&mut self) -> Change {
        if self.source != Some(PlanSource::Plan) {
            return Change::None;
        }
        self.tasks.clear();
        self.source = None;
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
        change
    }

    /// A turn ended: a subagent in the foreground cannot be running any more (it was stopped).
    /// One in the background goes on, and so does a workflow.
    pub fn end_turn(&mut self, now: i64) -> Change {
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
        if self.seen.iter().any(|s| s == call) {
            return false;
        }
        if self.seen.len() >= SEEN {
            self.seen.pop_front();
        }
        self.seen.push_back(call.to_string());
        true
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
            _ if agent_call => self.open_agent(id, name, input, now),
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

    /// The result of a tool call: `text` its text, `tool_use_result` the frame's structured one.
    pub fn on_tool_result(
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
        let totals = set_totals(
            row,
            tur["totalToolUseCount"].as_u64(),
            tur["totalTokens"].as_u64(),
        );
        if row.status != RunStatus::Running {
            // Ended already (its task said): the totals only complete it.
            return totals;
        }
        // In the background, the result only says it started; its end comes as a notification.
        if tur["status"] == "async_launched"
            || text.trim_start().starts_with("Async agent launched")
        {
            let mut change = totals;
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
    fn open_agent(&mut self, id: &str, name: &str, input: &Value, now: i64) -> Change {
        let description = input["description"].as_str().unwrap_or_default();
        let (kind, model) = (words(&input["subagent_type"]), words(&input["model"]));
        let background = input["run_in_background"] == true;
        // The prompt is read to find the task, never kept.
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
            _ => Change::None,
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
        self.launched += 1;
        self.agents.push(SubAgent {
            id: id.to_string(),
            title,
            kind,
            background,
            started_at: now,
            plan_task,
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
        let run = self.workflows.iter_mut().find(|w| w.id == task);
        match (status, run) {
            (Some(status), Some(run)) => {
                if end_workflow(run, status, now) {
                    Change::Saved
                } else {
                    Change::None
                }
            }
            _ => Change::None,
        }
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
            let totals = set_totals(
                row,
                usage["tool_uses"].as_u64(),
                usage["total_tokens"].as_u64(),
            );
            return if end_agent(row, status, now) {
                Change::Saved
            } else {
                totals
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
            None => Change::None,
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
            Some(row) => set_totals(row, tools, tokens),
            None => Change::None,
        }
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
