//! The subagents and background tasks of an agent through the server (ESC-107): the same ones the
//! window shows under its card (`PlanState::agents`, `bg_tasks`, `workflows`), listed
//! (`list_background_work`), read (`get_background_output`), stopped (`stop_background_work`) and
//! written to (`message_subagent`). Each does what the window does through the same function of
//! the core, which keeps the window's refusals and bounds: nothing is decided twice.

use super::act::{account_not_held, may_act_on, message_text, other_agent};
use super::activity;
use super::read::{json, WorkCounts};
use super::resolve;
use super::tools::{
    GetBackgroundOutputArgs, MessageSubagentArgs, StopBackgroundWorkArgs, ToolError,
};
use super::Caller;
use crate::core::Core;
use crate::model::AgentStatus;
use crate::plan::{BgKind, BgTask, PlanState, RunStatus, SubAgent, WorkflowRun};
use crate::taskout::{TaskOutput, MAX_OUTPUT};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;

/// The most bytes of an output `get_background_output` gives: 64 KiB. The window shows up to 256 KiB
/// of a task that is over (`MAX_FINISHED_OUTPUT`), which JSON-escaped is more than Claude Code takes
/// of the answer to a tool of an MCP server (about 25,000 tokens): a client cannot ask for that much.
const MAX_BYTES: usize = 65_536;

/// Something went wrong where the core did it: the reason it gives.
fn failed(e: anyhow::Error) -> ToolError {
    ToolError::Failed(format!("{e:#}"))
}

/// What an id names in an agent's plan, as the rows `list_background_work` lists.
enum Work {
    Subagent(SubAgent),
    Task(BgTask),
    Workflow(WorkflowRun),
}

impl Work {
    /// The entry's id in `list_background_work`.
    fn id(&self) -> &str {
        match self {
            Work::Subagent(a) => &a.id,
            Work::Task(t) => &t.id,
            Work::Workflow(w) => &w.id,
        }
    }

    /// Its `type` there.
    fn kind(&self) -> &'static str {
        match self {
            Work::Subagent(_) => "subagent",
            Work::Task(t) => kind_name(t.kind),
            Work::Workflow(_) => "workflow",
        }
    }

    fn status(&self) -> RunStatus {
        match self {
            Work::Subagent(a) => a.status,
            Work::Task(t) => t.status,
            Work::Workflow(w) => w.status,
        }
    }
}

/// The subagent, task or workflow `asked` names: the id of an entry of the list, else, for a
/// subagent, the id of its task. A task comes before a workflow of the same id, as in the list.
fn find(plan: Option<&PlanState>, asked: &str) -> Option<Work> {
    let (plan, asked) = (plan?, asked.trim());
    // A row that never had an id (a frame said none) is no one's to name by a blank.
    if asked.is_empty() {
        return None;
    }
    plan.agents
        .iter()
        .find(|a| a.id == asked)
        .cloned()
        .map(Work::Subagent)
        .or_else(|| {
            plan.bg_tasks
                .iter()
                .find(|t| t.id == asked)
                .cloned()
                .map(Work::Task)
        })
        .or_else(|| {
            plan.workflows
                .iter()
                .find(|w| w.id == asked)
                .cloned()
                .map(Work::Workflow)
        })
        .or_else(|| {
            plan.agents
                .iter()
                .find(|a| a.task_id.as_deref() == Some(asked))
                .cloned()
                .map(Work::Subagent)
        })
}

/// The agent has nothing of that name.
fn unknown(agent: &str, asked: &str) -> ToolError {
    let (name, id) = (agent, asked.trim());
    ToolError::Failed(tr!(
        "L’agent {name} n’a ni sous-agent ni tâche de fond « {id} » : list_background_work les liste.",
        "The agent {name} has neither a subagent nor a background task “{id}”: list_background_work lists them."
    ))
}

/// The status and the plan of the agent `found`: None for an agent that never had a plan. The
/// agent's lock is released before anything is done with them.
fn plan_for(
    core: &Core,
    found: &resolve::AgentRef,
) -> Result<(AgentStatus, Option<PlanState>), ToolError> {
    let h = core.agent(&found.id).map_err(|_| {
        let name = &found.name;
        ToolError::Failed(tr!(
            "L’agent {name} n’existe plus.",
            "The agent {name} is gone."
        ))
    })?;
    let rt = h.lock();
    Ok((rt.meta.status, rt.meta.plan.clone()))
}

/// The agent `asked` names (in `project` when given), its status and its plan.
fn plan_of(
    core: &Core,
    caller: &Caller,
    asked: &str,
    project: Option<&str>,
) -> Result<(resolve::AgentRef, AgentStatus, Option<PlanState>), ToolError> {
    let scope = project
        .map(|p| resolve::project(core, caller, p))
        .transpose()?;
    let found = resolve::agent(core, caller, asked, scope.as_ref())?;
    let (status, plan) = plan_for(core, &found)?;
    Ok((found, status, plan))
}

// ---------- list_background_work ----------

/// A subagent of the plan, as `list_background_work` lists it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubagentRow {
    /// The id of the call that launched it: what the other tools name it by.
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    /// What the window's row says it does: left out of an `outline`.
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Its type ("Explore"…).
    #[serde(skip_serializing_if = "Option::is_none")]
    subagent_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    status: RunStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    background: Option<bool>,
    started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    ended_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tokens: Option<u64>,
    /// What it does now, while it runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    doing: Option<String>,
    /// Claude Code's id of its task, once a frame told it: what stopping it and writing to it need.
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    /// The call of the subagent that launched it.
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<String>,
}

impl SubagentRow {
    /// What says that it exists, how it stands and when it started and ended, and nothing else: no
    /// word of what it does, no model, no task.
    fn outline(self) -> Self {
        Self {
            id: self.id,
            kind: self.kind,
            description: None,
            subagent_type: None,
            model: None,
            status: self.status,
            background: None,
            started_at: self.started_at,
            ended_at: self.ended_at,
            tools: None,
            tokens: None,
            doing: None,
            task_id: None,
            parent: None,
        }
    }
}

impl From<&SubAgent> for SubagentRow {
    fn from(a: &SubAgent) -> Self {
        Self {
            id: a.id.clone(),
            kind: "subagent",
            description: Some(a.title.clone()),
            subagent_type: a.kind.clone(),
            model: a.model.clone(),
            status: a.status,
            background: Some(a.background),
            started_at: a.started_at,
            ended_at: a.ended_at,
            tools: Some(a.tools),
            tokens: a.tokens,
            doing: a.doing.clone(),
            task_id: a.task_id.clone(),
            parent: a.parent.clone(),
        }
    }
}

/// A task Claude Code runs in the background, or a workflow, as `list_background_work` lists it.
/// Never the file its output is written to: a path of the disk is not the model's to read.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskRow {
    /// Claude Code's id of the task.
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    /// What the tool said it was for, not the command: left out of an `outline`.
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    status: RunStatus,
    started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    ended_at: Option<i64>,
    /// Claude Code's last words on it.
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    /// The task of the subagent that owns it.
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<String>,
    /// A workflow's name, the phase and agent running now, and what it used so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    now: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tokens: Option<u64>,
}

/// What a kind of background task is called.
fn kind_name(kind: BgKind) -> &'static str {
    match kind {
        BgKind::Command => "command",
        BgKind::Monitor => "monitor",
        BgKind::Remote => "remote",
        BgKind::Teammate => "teammate",
    }
}

impl TaskRow {
    /// What says that it exists, how it stands and when it started and ended, and nothing else: no
    /// word of what it does, no command line (it may hold a secret), no summary.
    fn outline(self) -> Self {
        Self {
            id: self.id,
            kind: self.kind,
            description: None,
            command: None,
            status: self.status,
            started_at: self.started_at,
            ended_at: self.ended_at,
            summary: None,
            parent: None,
            name: None,
            now: None,
            tools: None,
            tokens: None,
        }
    }
}

impl From<&BgTask> for TaskRow {
    fn from(t: &BgTask) -> Self {
        Self {
            id: t.id.clone(),
            kind: kind_name(t.kind),
            description: Some(t.title.clone()),
            command: t.command.clone(),
            status: t.status,
            started_at: t.started_at,
            ended_at: t.ended_at,
            summary: t.summary.clone(),
            parent: t.parent.clone(),
            name: None,
            now: None,
            tools: None,
            tokens: None,
        }
    }
}

impl From<&WorkflowRun> for TaskRow {
    fn from(w: &WorkflowRun) -> Self {
        Self {
            id: w.id.clone(),
            kind: "workflow",
            description: Some(w.title.clone()),
            command: None,
            status: w.status,
            started_at: w.started_at,
            ended_at: w.ended_at,
            summary: None,
            parent: None,
            name: w.name.clone(),
            now: w.now.clone(),
            tools: Some(w.tools),
            tokens: Some(w.tokens),
        }
    }
}

/// The subagents of a plan in the order of the window's sidebar (`sidebarSubs`): those running, in
/// the plan's order, then those that ended, the latest first (one that never said when comes last).
/// Each as an `outline` when `full` is false.
fn subagent_rows(plan: &PlanState, full: bool) -> Vec<SubagentRow> {
    let running = |a: &&SubAgent| a.status == RunStatus::Running;
    let mut ended: Vec<&SubAgent> = plan.agents.iter().filter(|a| !running(a)).collect();
    // Stable: the rows that tie keep the plan's order.
    ended.sort_by_key(|a| std::cmp::Reverse(a.ended_at.unwrap_or(0)));
    plan.agents
        .iter()
        .filter(running)
        .chain(ended)
        .map(SubagentRow::from)
        .map(|r| if full { r } else { r.outline() })
        .collect()
}

/// The tasks of a plan in the order of the window's list (`bgRows`): those running, the oldest
/// launched first, then those that ended, the latest first. An id is listed once, the first
/// time: a workflow never shadows a task. Each as an `outline` when `full` is false.
fn task_rows(plan: &PlanState, full: bool) -> Vec<TaskRow> {
    let mut seen = HashSet::new();
    let rows: Vec<TaskRow> = plan
        .bg_tasks
        .iter()
        .map(TaskRow::from)
        .chain(plan.workflows.iter().map(TaskRow::from))
        .filter(|r| seen.insert(r.id.clone()))
        .collect();
    let (mut running, mut ended): (Vec<TaskRow>, Vec<TaskRow>) = rows
        .into_iter()
        .partition(|r| r.status == RunStatus::Running);
    // Stable: the rows that tie keep the order they were given in.
    running.sort_by_key(|r| r.started_at);
    ended.sort_by_key(|r| std::cmp::Reverse(r.ended_at.unwrap_or(r.started_at)));
    running.extend(ended);
    running
        .into_iter()
        .map(|r| if full { r } else { r.outline() })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Listing {
    id: String,
    name: String,
    status: AgentStatus,
    #[serde(flatten)]
    work: WorkCounts,
    subagents: Vec<SubagentRow>,
    tasks: Vec<TaskRow>,
}

/// `list_background_work`: the subagents and the background tasks of the agent `agent` names (in
/// `project` when given), as the window lists them under its card. The list is open to all, what is
/// in it is not: an agent of Escouade is given every field of the agents of the projects it acts on
/// (`may_act_on`), but of the others only which work there is and how it stands (`outline`): a
/// command line may hold a secret, and what the work does is that project's own.
pub(super) fn list(
    core: &Core,
    caller: &Caller,
    agent: &str,
    project: Option<&str>,
) -> Result<String, ToolError> {
    let (found, status, plan) = plan_of(core, caller, agent, project)?;
    // The test of `may_act_on`, which refuses where this only says less.
    let full = may_act_on(core, caller, &found.project_id).is_ok();
    json(&Listing {
        id: found.id,
        name: found.name,
        status,
        work: WorkCounts::of(plan.as_ref()),
        subagents: plan
            .as_ref()
            .map(|p| subagent_rows(p, full))
            .unwrap_or_default(),
        tasks: plan
            .as_ref()
            .map(|p| task_rows(p, full))
            .unwrap_or_default(),
    })
}

// ---------- get_background_output ----------

/// The agent an answer is about.
#[derive(Serialize)]
struct AgentName {
    id: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OutputView {
    agent: AgentName,
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    status: RunStatus,
    /// Where it was read: `process`, `file`, `report` (a subagent's) or `none`.
    source: String,
    output: String,
    /// The size of the whole output, in bytes.
    total_bytes: u64,
    /// The start of the output is not in `output`.
    truncated: bool,
}

/// `out` cut to its last `max` bytes, from the start of a character: what is left out of the start
/// is said (`truncated`).
fn end_of(mut out: TaskOutput, max: usize) -> TaskOutput {
    if out.output.len() <= max {
        return out;
    }
    let mut from = out.output.len() - max;
    while !out.output.is_char_boundary(from) {
        from += 1;
    }
    out.output = out.output[from..].to_string();
    out.truncated = true;
    out
}

/// `get_background_output`: the end of the output of a command or a monitor of the agent (through
/// the core, which keeps the window's bounds and reads no file but the task's own), or the report
/// of a subagent; nothing for the others, which write no output of their own.
pub(super) async fn output(
    core: &Arc<Core>,
    caller: &Caller,
    a: GetBackgroundOutputArgs,
) -> Result<String, ToolError> {
    let (found, _, plan) = plan_of(core, caller, &a.agent, a.project.as_deref())?;
    // The sheet of an agent is open to all, the files of its tasks are the files of its project: an
    // agent of Escouade reads those of the projects it acts on.
    may_act_on(core, caller, &found.project_id)?;
    let work = find(plan.as_ref(), &a.id).ok_or_else(|| unknown(&found.name, &a.id))?;
    // A client takes 8 KiB unless it asks for more, 64 KiB at most: the window's own bound for a
    // task that is over (256 KiB) is more than an answer to a client carries.
    let max = a.bytes.map_or(MAX_OUTPUT, |b| {
        usize::try_from(b).unwrap_or(usize::MAX).clamp(1, MAX_BYTES)
    });
    let out = match &work {
        Work::Subagent(s) => match core
            .subagent_report(&found.id, &s.id)
            .await
            .map_err(failed)?
        {
            Some(report) => TaskOutput {
                total_bytes: report.len() as u64,
                output: report,
                truncated: false,
                source: "report".into(),
            },
            None => TaskOutput::none(),
        },
        Work::Task(t) if matches!(t.kind, BgKind::Command | BgKind::Monitor) => end_of(
            core.bg_task_output(&found.id, &t.id)
                .await
                .map_err(failed)?,
            max,
        ),
        // A remote agent, a teammate and a workflow write no output the core could give.
        Work::Task(_) | Work::Workflow(_) => TaskOutput::none(),
    };
    json(&OutputView {
        agent: AgentName {
            id: found.id,
            name: found.name,
        },
        id: work.id().to_string(),
        kind: work.kind(),
        status: work.status(),
        source: out.source,
        output: out.output,
        total_bytes: out.total_bytes,
        truncated: out.truncated,
    })
}

// ---------- stop_background_work ----------

/// What the core refused, or what failed, when it was asked to stop or write to something. Its
/// guards are `bail!`s of one message (it no longer runs, it is in the foreground, the Claude Code
/// view…): a refusal, in the window's words. A failure of Claude Code's own carries what was being
/// done as the context of its reason (`Claude Code n’a pas arrêté la tâche : …`), a chain of more
/// than one link: it failed.
fn refusal_or_failure(e: anyhow::Error) -> ToolError {
    if e.chain().count() > 1 {
        ToolError::Failed(format!("{e:#}"))
    } else {
        ToolError::Refused(e.to_string())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StopView {
    agent: AgentName,
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    stopped: bool,
}

/// `stop_background_work`: Claude Code is asked to stop the subagent, task or workflow of the agent
/// that `a.id` names, as the window's « Arrêter » asks it. Its end comes with the frames that follow.
pub(super) async fn stop(
    core: &Arc<Core>,
    caller: &Caller,
    a: StopBackgroundWorkArgs,
) -> Result<String, ToolError> {
    let found = other_agent(
        core,
        caller,
        &a.agent,
        a.project.as_deref(),
        tr!(
            "Un agent ne peut pas arrêter ses propres tâches de fond.",
            "An agent cannot stop its own background tasks."
        ),
    )?;
    if found.archived {
        let name = &found.name;
        return Err(ToolError::Refused(tr!(
            "L’agent {name} est archivé : on n’arrête plus ses sous-agents ni ses tâches de fond.",
            "The agent {name} is archived: its subagents and background tasks are no longer stopped."
        )));
    }
    let (_, plan) = plan_for(core, &found)?;
    let work = find(plan.as_ref(), &a.id).ok_or_else(|| unknown(&found.name, &a.id))?;
    // The refusals are the window's own (it no longer runs, the foreground, no task told, the
    // Claude Code view, no process): the core says them.
    match &work {
        Work::Subagent(s) => core.stop_subagent(&found.id, &s.id).await,
        Work::Task(t) => core.stop_bg_task(&found.id, &t.id).await,
        Work::Workflow(w) => core.stop_bg_task(&found.id, &w.id).await,
    }
    .map_err(refusal_or_failure)?;
    json(&StopView {
        agent: AgentName {
            id: found.id,
            name: found.name,
        },
        id: work.id().to_string(),
        kind: work.kind(),
        stopped: true,
    })
}

// ---------- message_subagent ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageView {
    agent: AgentName,
    /// The call that launched the subagent, whichever way it was named.
    subagent: String,
    /// The agent is at work: the message waits for the end of its turn.
    queued: bool,
}

/// `message_subagent`: the agent that `a.agent` names is asked to pass the words on to one of its
/// subagents, with its `SendMessage` tool (Claude Code lets no host write to a subagent). They
/// reach it headed by the caller's name, never as the user's own; it costs the agent a turn, so
/// not while its account is held back, as `send_message`. The words are never in the log.
pub(super) async fn message(
    core: &Arc<Core>,
    caller: &Caller,
    a: MessageSubagentArgs,
) -> Result<String, ToolError> {
    let found = other_agent(
        core,
        caller,
        &a.agent,
        a.project.as_deref(),
        tr!(
            "Un agent n’écrit pas à ses propres sous-agents.",
            "An agent does not write to its own subagents."
        ),
    )?;
    if found.archived {
        let name = &found.name;
        return Err(ToolError::Refused(tr!(
            "L’agent {name} est archivé : ses sous-agents ne reçoivent plus de message.",
            "The agent {name} is archived: its subagents no longer take messages."
        )));
    }
    let text = message_text(&a.text)?;
    let (status, plan) = plan_for(core, &found)?;
    let work = find(plan.as_ref(), &a.subagent).ok_or_else(|| unknown(&found.name, &a.subagent))?;
    let Work::Subagent(subagent) = work else {
        let (id, kind) = (work.id(), work.kind());
        return Err(ToolError::Failed(tr!(
            "« {id} » n’est pas un sous-agent mais une tâche de fond ({kind}) : seuls les sous-agents reçoivent un message.",
            "“{id}” is not a subagent but a background task ({kind}): only subagents take a message."
        )));
    };
    // What waiting would not change is said before the quota is (a caller told to try again at 14:00
    // would be refused all the same): the core's own guards, which it checks again when it sends,
    // the window having changed since.
    core.subagent_target(&found.id, &subagent.id)
        .map_err(refusal_or_failure)?;
    // The agent spends a turn on it.
    account_not_held(core, &found)?;
    let origin = activity::caller_name(core, Some(caller));
    core.message_subagent_from(&found.id, &subagent.id, text, Some(&origin))
        .await
        .map_err(refusal_or_failure)?;
    json(&MessageView {
        agent: AgentName {
            id: found.id,
            name: found.name,
        },
        subagent: subagent.id,
        queued: status.is_active(),
    })
}
