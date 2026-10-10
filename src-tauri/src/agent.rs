//! Runtime state of one agent: turns raw Claude Code frames into conversation items,
//! tracks status, pending questions/permissions and per-turn usage.

use crate::board::TurnEnd;
use crate::claude::{truncate, ClaudeProcess};
use crate::conv::Conv;
use crate::i18n::{self, Lang};
use crate::model::*;
use crate::notify;
use crate::paths::relative_slash;
use crate::plan::{Change, PlanState};
use crate::planfiles::FileList;
use crate::pricing::{self, Usage};
use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub type AgentHandle = Arc<parking_lot::Mutex<AgentRt>>;

/// What a notification is about: each kind has its own choice in the settings ("Me prévenir pour").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyKind {
    /// A question or a permission waits for the user.
    Question,
    /// A turn ended.
    Done,
    /// A turn failed, or the process died.
    Error,
    /// A ticket is ready to test, or blocked (raised by the board, never by an agent's own turn).
    Ticket,
}

/// A notification an agent raises: its kind, and the words it says (its title is the agent's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAlert {
    pub kind: NotifyKind,
    pub body: String,
}

impl AgentAlert {
    fn new(kind: NotifyKind, body: impl Into<String>) -> Self {
        Self {
            kind,
            body: body.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TurnRow {
    pub model: String,
    pub input: u64,
    pub cache: u64,
    pub output: u64,
    pub cost: f64,
}

/// Side effects collected while the agent lock is held, applied once it is released.
#[derive(Default)]
pub struct Effects {
    pub ops: Vec<ConvOp>,
    pub agent_changed: bool,
    pub save: bool,
    pub notify: Option<AgentAlert>,
    pub turns: Vec<TurnRow>,
    pub rate: Option<(Option<RateWindow>, Option<RateWindow>)>,
    /// The turn was stopped by the usage limit, which resets then (if Claude Code told).
    pub limited: Option<Option<i64>>,
    pub files_changed: bool,
    /// A turn ended (or the process died during one): how, with the assistant's text.
    pub turn_end: Option<TurnEnd>,
    /// What the agent just did may have changed the files of its plan (a command ended, a
    /// subagent came back, the turn ended): the core has them looked at.
    pub plan_files: bool,
}

struct Block {
    index: u64,
    item_id: String,
    kind: &'static str,
    finalized: bool,
}

struct PendingReq {
    item_id: String,
    tool_use_id: String,
    input: Value,
    suggestions: Value,
    /// What the agent's view says of it, worked out once when it comes.
    view: PendingView,
}

#[derive(Debug, Clone, Default)]
struct Counters {
    input: u64,
    output: u64,
    cache: u64,
    cost: f64,
}

pub struct AgentRt {
    pub meta: AgentMeta,
    pub proc: Option<Arc<ClaudeProcess>>,
    pub gen: u64,
    pub conv: Conv,
    pub active_since: Option<i64>,
    pub context_tokens: u64,
    pub context_window: u64,
    /// Model of the conversation's latest message (it may be switched mid-process).
    main_model: String,
    /// This turn met the usage limit, and when the limit resets.
    rate_limited: bool,
    limit_resets_at: Option<i64>,
    /// Tasks run in the foreground: their tool rows show them end.
    foreground_tasks: HashSet<String>,
    /// Background tasks whose end was shown, until Claude Code also replays it to Claude.
    announced_tasks: HashMap<String, u32>,
    pub commands: Vec<Value>,
    pub interrupted: bool,
    pub saw_init: bool,
    /// Remote Control link state ("ready", "connected"…) of the live process.
    pub remote_state: Option<String>,
    /// The live process was linked to claude.ai (Remote Control).
    pub remote_linked: bool,
    /// What the agent does now, from its latest tool, thinking or text (main thread only).
    pub activity: Option<String>,
    /// The plans it wrote (docs/superpowers/plans/*.md, as absolute paths), oldest first, five at
    /// most: the plan of its folder when no ledger names one (`planfiles`).
    pub(crate) plan_written: Vec<String>,
    /// The calls of its main thread whose result calls for a look at the files of its plan.
    plan_calls: HashSet<String>,
    /// When its conversation began, in milliseconds: the files of a plan that were last touched
    /// before are not its own.
    pub(crate) plan_since: i64,
    /// The assistant's text of the running turn (main thread), for the board's report.
    turn_text: String,
    /// The latest text of the running turn's main thread that no tool followed: its final reply,
    /// for the notification.
    final_text: String,
    /// The setup of its new worktree under way: the step running ("npm ci (1/2)").
    pub setup: Option<String>,
    /// What the step of that setup wrote, its last lines: sent to the window as it comes, apart
    /// from its view (`UiEvent::SetupOutput`).
    pub setup_output: Option<SetupOutput>,
    /// Why the setup of its worktree failed, until its ticket's first message tells it (written in
    /// the language of the texts for Claude, as that message).
    pub setup_failure: Option<String>,
    blocks: HashMap<String, Vec<Block>>,
    current_msg: HashMap<String, String>,
    /// Requests waiting for an answer, in the order they came (an item's id is its request's).
    pending: Vec<PendingReq>,
    last_usage: HashMap<String, Counters>,
    /// Usage of the running turn's API messages (by message id, with their model), until the
    /// turn's result brings the exact figures.
    live: HashMap<String, (String, Usage)>,
    queued: u32,
}

const MAX_TEXT: usize = 8000;
/// The plans of the plans folder an agent wrote that are remembered, and the calls waited for.
const MAX_PLANS_WRITTEN: usize = 5;
const MAX_PLAN_CALLS: usize = 64;
const MAX_INPUT_STR: usize = 4000;
const MAX_PATCH_LINES: usize = 800;

/// `path` is in the folder where the superpowers skills keep what a plan's execution leaves
/// (`.superpowers/sdd`, anywhere in the repository).
fn in_sdd_workspace(path: &str) -> bool {
    const SDD: &str = ".superpowers/sdd/";
    let slashed = path.replace('\\', "/").to_ascii_lowercase();
    slashed.starts_with(SDD) || slashed.contains(&format!("/{SDD}"))
}

impl AgentRt {
    pub fn new(mut meta: AgentMeta, conv_dir: &std::path::Path) -> Self {
        // A turn cannot survive an app restart.
        if meta.status.is_active() {
            meta.status = AgentStatus::Done;
        }
        // Neither can what ran for it: the plan was saved with rows running.
        if let Some(plan) = meta.plan.as_mut() {
            plan.close_running(now_ms());
        }
        Self {
            conv: Conv::new(conv_dir, &meta.id),
            meta,
            proc: None,
            gen: 0,
            active_since: None,
            context_tokens: 0,
            context_window: 0,
            main_model: String::new(),
            rate_limited: false,
            limit_resets_at: None,
            foreground_tasks: HashSet::new(),
            announced_tasks: HashMap::new(),
            commands: Vec::new(),
            interrupted: false,
            saw_init: false,
            remote_state: None,
            remote_linked: false,
            activity: None,
            plan_written: Vec::new(),
            plan_calls: HashSet::new(),
            plan_since: now_ms(),
            turn_text: String::new(),
            final_text: String::new(),
            setup: None,
            setup_output: None,
            setup_failure: None,
            blocks: HashMap::new(),
            current_msg: HashMap::new(),
            pending: Vec::new(),
            last_usage: HashMap::new(),
            live: HashMap::new(),
            queued: 0,
        }
    }

    /// Tokens and estimated cost of the running turn so far.
    fn live_totals(&self) -> (u64, f64) {
        self.live.values().fold((0, 0.0), |(t, c), (model, u)| {
            (
                t + u.tokens(),
                c + pricing::estimate(model, u).unwrap_or(0.0),
            )
        })
    }

    pub fn view(&self) -> AgentView {
        let (live_tokens, live_cost) = self.live_totals();
        AgentView {
            meta: self.meta.clone(),
            active_since: self.active_since,
            alive: self.proc.is_some(),
            pending: self.pending.iter().map(|p| p.item_id.clone()).collect(),
            requests: self.pending.iter().map(|p| p.view.clone()).collect(),
            context_tokens: self.context_tokens,
            context_window: self.context_window,
            live_tokens,
            live_cost,
            remote_state: self.remote_state.clone(),
            activity: self.activity.clone(),
            setup: self.setup.clone(),
            isola: self
                .meta
                .worktree
                .as_ref()
                .is_some_and(|w| crate::isola::manages(&w.path)),
        }
    }

    /// Resets per-process bookkeeping when a new `claude` process is attached.
    pub fn attach(&mut self, proc: Arc<ClaudeProcess>) {
        self.proc = Some(proc);
        // A process that just started is not idle.
        self.meta.last_activity = now_ms();
        self.blocks.clear();
        self.current_msg.clear();
        self.pending.clear();
        self.last_usage.clear();
        self.live.clear();
        self.remote_state = None;
        self.remote_linked = false;
        self.forget_turn();
        self.saw_init = false;
        self.queued = 0;
        self.foreground_tasks.clear();
        self.announced_tasks.clear();
        // What ran in the process before is gone with it.
        self.interrupt_plan();
    }

    /// Runs `f` on the agent's plan (made when it has none yet), keeps it only when it holds
    /// something, and tells what changed: a detail goes to the window, the rest is saved too.
    fn plan_apply(&mut self, fx: &mut Effects, f: impl FnOnce(&mut PlanState) -> Change) {
        let mut plan = self.meta.plan.take().unwrap_or_default();
        let change = f(&mut plan);
        if !plan.is_empty() {
            self.meta.plan = Some(plan);
        }
        match change {
            Change::None => {}
            Change::View => fx.agent_changed = true,
            Change::Saved => {
                fx.agent_changed = true;
                fx.save = true;
            }
        }
    }

    /// The process is gone: what the plan has running is interrupted (the caller sends the view).
    fn interrupt_plan(&mut self) -> Change {
        let now = now_ms();
        self.meta
            .plan
            .as_mut()
            .map_or(Change::None, |plan| plan.close_running(now))
    }

    /// A new plan: what the last forgets, and the files it will read are those touched from now on.
    fn reset_plan(&mut self, fx: &mut Effects) {
        self.plan_apply(fx, PlanState::reset);
        self.plan_since = now_ms();
        self.plan_written.clear();
        self.plan_calls.clear();
    }

    /// What a look at the agent's folder read (`planfiles`, off its lock), unless its conversation
    /// is not the one the look was made for any more: then nothing is applied (false).
    pub fn plan_from_files(
        &mut self,
        since: i64,
        list: Option<FileList>,
        fx: &mut Effects,
    ) -> bool {
        if since != self.plan_since {
            return false;
        }
        self.plan_apply(fx, |plan| plan.set_files(list));
        true
    }

    fn push(&mut self, op: ConvOp, fx: &mut Effects) {
        self.conv.apply(&op);
        fx.ops.push(op);
    }

    fn append(&mut self, item: Value, fx: &mut Effects) {
        self.push(ConvOp::Append { item }, fx);
    }

    fn patch(&mut self, id: &str, patch: Value, fx: &mut Effects) {
        self.push(
            ConvOp::Patch {
                id: id.to_string(),
                patch,
            },
            fx,
        );
    }

    pub fn notice(&mut self, level: &str, text: impl Into<String>, fx: &mut Effects) {
        let item = json!({ "kind": "notice", "id": new_id(), "ts": now_ms(), "level": level, "text": text.into() });
        self.append(item, fx);
    }

    /// Drops what the running turn showed (its activity, its text): the turn is over, or lost.
    fn forget_turn(&mut self) {
        self.activity = None;
        self.turn_text.clear();
        self.final_text.clear();
    }

    /// What the agent does now, shown live; unchanged by a tool it has no words for, and never
    /// blurred: a bare verb ("Lit", a tool that only began) does not replace what the same verb
    /// already told ("Lit src/a.ts").
    fn set_activity(&mut self, activity: Option<String>, fx: &mut Effects) {
        let Some(activity) = activity else {
            return;
        };
        let blurs = !activity.contains(' ')
            && self
                .activity
                .as_deref()
                .is_some_and(|current| current.starts_with(&format!("{activity} ")));
        if blurs || self.activity.as_ref() == Some(&activity) {
            return;
        }
        self.activity = Some(activity);
        fx.agent_changed = true;
    }

    pub fn set_status(&mut self, status: AgentStatus, fx: &mut Effects) {
        if self.meta.status == status {
            return;
        }
        let now = now_ms();
        let (was, is) = (self.meta.status.is_active(), status.is_active());
        if was && !is {
            if let Some(since) = self.active_since.take() {
                self.meta.active_ms += (now - since).max(0) as u64;
            }
        } else if !was && is {
            self.active_since = Some(now);
        }
        self.meta.status = status;
        self.meta.last_activity = now;
        fx.agent_changed = true;
        fx.save = true;
    }

    /// Records a user message delivered to Claude under `id` (the frame uuid), with its image
    /// count and the names of its other attached files. Returns true when it was queued behind
    /// a running turn.
    pub fn push_user(
        &mut self,
        id: &str,
        text: &str,
        images: u32,
        files: &[String],
        fx: &mut Effects,
    ) -> bool {
        self.push_user_from(id, text, images, files, None, fx)
    }

    /// `origin`: "remote" for a message sent from claude.ai / the Claude app (Remote Control).
    fn push_user_from(
        &mut self,
        id: &str,
        text: &str,
        images: u32,
        files: &[String],
        origin: Option<&str>,
        fx: &mut Effects,
    ) -> bool {
        let queued = self.meta.status.is_active();
        let mut item = json!({ "kind": "user", "id": id, "text": text, "images": images, "ts": now_ms(), "queued": queued });
        if !files.is_empty() {
            item["files"] = json!(files);
        }
        if let Some(o) = origin {
            item["origin"] = json!(o);
        }
        // Someone took over: no resume by itself after the usage limit.
        self.meta.resume_at = None;
        self.append(item, fx);
        if queued {
            self.queued += 1;
        } else {
            self.turn_text.clear();
            self.final_text.clear();
            self.interrupted = false;
            self.set_status(AgentStatus::Running, fx);
            // A new turn after a plan that is over (or that was none) starts a plan of its own; one
            // that is under way goes on, and so does a message queued behind a turn.
            if self.meta.plan.as_ref().is_some_and(PlanState::is_finished) {
                self.reset_plan(fx);
            }
        }
        self.meta.prompts += 1;
        self.meta.last_activity = now_ms();
        fx.agent_changed = true;
        fx.save = true;
        queued
    }

    /// Deliberate stop (idle, archive): detaches the process so that its exit is ignored and
    /// the next action spawns a fresh one resuming the same session.
    pub fn detach(&mut self) -> Option<Arc<ClaudeProcess>> {
        // Nothing more is heard from the process, so a turn it was running never ends here, nor
        // what it ran for the plan (the caller sends the view).
        self.forget_turn();
        self.interrupt_plan();
        let p = self.proc.take()?;
        self.gen += 1;
        Some(p)
    }

    pub fn clear_pending(&mut self, fx: &mut Effects) {
        let ids: Vec<String> = self.pending.drain(..).map(|p| p.item_id).collect();
        for id in ids {
            self.patch(&id, json!({ "cancelled": true }), fx);
        }
    }

    /// Closes what a turn left open: partial text is kept (and persisted, deltas being
    /// memory-only), tools that never returned are marked interrupted.
    fn close_open_items(&mut self, fx: &mut Effects) {
        for (id, text) in self.conv.open_items() {
            let patch = match text {
                Some(text) => json!({ "text": text, "streaming": false }),
                None => json!({ "status": "interrupted" }),
            };
            self.patch(&id, patch, fx);
        }
        for blocks in self.blocks.values_mut() {
            for b in blocks.iter_mut() {
                b.finalized = true;
            }
        }
    }

    /// A request comes after those waiting already: the one asked first stays the one shown
    /// first. Asked again, a request keeps its place.
    fn add_pending(&mut self, req: PendingReq) {
        match self.pending.iter_mut().find(|p| p.item_id == req.item_id) {
            Some(p) => *p = req,
            None => self.pending.push(req),
        }
    }

    /// Takes the request `request_id` out of those waiting, the others keeping their order.
    fn take_pending(&mut self, request_id: &str) -> Option<PendingReq> {
        let i = self.pending.iter().position(|p| p.item_id == request_id)?;
        Some(self.pending.remove(i))
    }

    #[cfg(test)]
    pub fn has_pending(&self, request_id: &str) -> bool {
        self.pending.iter().any(|p| p.item_id == request_id)
    }

    pub fn answer_question(
        &mut self,
        request_id: &str,
        answers: Value,
        fx: &mut Effects,
    ) -> Result<()> {
        let proc = self.proc.clone().ok_or_else(not_running)?;
        let p = self
            .take_pending(request_id)
            .ok_or_else(|| anyhow!(tr!("question introuvable", "question not found")))?;
        let mut input = p.input.clone();
        input["answers"] = answers.clone();
        proc.respond(
            request_id,
            json!({ "behavior": "allow", "updatedInput": input, "toolUseID": p.tool_use_id }),
        )?;
        self.patch(&p.item_id, json!({ "answers": answers }), fx);
        self.after_answer(fx);
        Ok(())
    }

    pub fn answer_permission(
        &mut self,
        request_id: &str,
        decision: &str,
        message: Option<String>,
        fx: &mut Effects,
    ) -> Result<()> {
        let proc = self.proc.clone().ok_or_else(not_running)?;
        let p = self
            .take_pending(request_id)
            .ok_or_else(|| anyhow!(tr!("demande introuvable", "request not found")))?;
        let response = match decision {
            "allow" => {
                json!({ "behavior": "allow", "updatedInput": p.input, "toolUseID": p.tool_use_id })
            }
            "always" => json!({
                "behavior": "allow", "updatedInput": p.input, "toolUseID": p.tool_use_id,
                "updatedPermissions": p.suggestions,
            }),
            _ => json!({
                "behavior": "deny", "toolUseID": p.tool_use_id,
                "message": message.clone().filter(|m| !m.trim().is_empty())
                    .unwrap_or_else(|| refused_action(i18n::claude())),
            }),
        };
        proc.respond(request_id, response)?;
        self.patch(
            &p.item_id,
            json!({ "decision": decision, "message": message }),
            fx,
        );
        self.after_answer(fx);
        Ok(())
    }

    fn after_answer(&mut self, fx: &mut Effects) {
        if self.pending.is_empty() && self.meta.status == AgentStatus::Waiting {
            self.set_status(AgentStatus::Running, fx);
        }
        fx.agent_changed = true;
    }

    /// The process exited. `gen` guards against a stale exit of a replaced process.
    pub fn on_exit(&mut self, gen: u64, code: Option<i32>, stderr: &str, fx: &mut Effects) {
        if gen != self.gen {
            return;
        }
        self.proc = None;
        self.live.clear();
        self.remote_state = None;
        self.remote_linked = false;
        self.forget_turn();
        self.plan_apply(fx, |plan| plan.close_running(now_ms()));
        self.clear_pending(fx);
        self.close_open_items(fx);
        let was_running = self.meta.status.is_active();
        // Gone before it started: what it was to resume could not be.
        let [session_lost, fork_point_lost] = exit_notices(i18n::ui());
        let lost = if self.saw_init {
            None
        } else if stderr.contains("No conversation found") {
            self.meta.session_id = None;
            self.meta.last_entry = None;
            // A copy whose original's session is gone: the same, a new session.
            self.meta.fork_of = None;
            self.meta.fork_at = None;
            Some(session_lost)
        } else if self.meta.fork_at.is_some()
            && stderr.contains("No message found with message.uuid")
        {
            // A copy whose point in its original's session is not there: still a copy of that
            // session, of all of it.
            self.meta.fork_at = None;
            Some(fork_point_lost)
        } else {
            None
        };
        if let Some(lost) = lost {
            self.notice("warn", lost.clone(), fx);
            self.set_status(AgentStatus::Done, fx);
            // The message it was to carry never ran: that turn ends here, though not as a failure
            // of the agent.
            if was_running {
                fx.turn_end = Some(TurnEnd::Error(lost));
            }
        } else if was_running || !self.saw_init {
            let detail = if stderr.trim().is_empty() {
                String::new()
            } else {
                format!("\n\n{}", truncate(stderr.trim(), 2000))
            };
            let code = code.map(|c| c.to_string()).unwrap_or_else(|| "?".into());
            let reason = stopped(i18n::ui(), &code);
            self.notice("error", format!("{reason}.{detail}"), fx);
            self.set_status(AgentStatus::Error, fx);
            fx.notify = Some(AgentAlert::new(
                NotifyKind::Error,
                notify::error_body(&reason),
            ));
            if was_running {
                fx.turn_end = Some(TurnEnd::Error(reason));
            }
        }
        fx.agent_changed = true;
    }

    pub fn handle_frame(&mut self, f: &Value, fx: &mut Effects) {
        match f["type"].as_str().unwrap_or("") {
            "system" => self.on_system(f, fx),
            "stream_event" => self.on_stream(f, fx),
            "assistant" => self.on_assistant(f, fx),
            "user" => self.on_user(f, fx),
            "result" => self.on_result(f, fx),
            "control_request" => self.on_control_request(f, fx),
            "control_cancel_request" => self.on_cancel(f, fx),
            "rate_limit_event" => {
                let info = &f["rate_limit_info"];
                if info["status"] == "rejected" {
                    self.limit_resets_at = info["resetsAt"].as_i64().map(|s| s * 1000);
                }
                fx.rate = Some(parse_rate_event(info));
            }
            _ => {}
        }
    }

    fn on_system(&mut self, f: &Value, fx: &mut Effects) {
        let subtype = f["subtype"].as_str().unwrap_or("");
        // The tasks of the plan (the rows of events below are about the same frames).
        if matches!(
            subtype,
            "task_started" | "task_updated" | "task_notification" | "task_progress"
        ) {
            self.plan_apply(fx, |plan| plan.on_system(f, now_ms()));
        }
        match subtype {
            "init" => {
                if let Some(sid) = f["session_id"].as_str() {
                    if self.meta.session_id.as_deref() != Some(sid) {
                        // Another conversation: what the plan was about is not in its context.
                        self.reset_plan(fx);
                        if self.saw_init {
                            self.notice(
                                "info",
                                tr!(
                                    "Nouvelle conversation Claude (contexte vidé)",
                                    "New Claude conversation (context cleared)"
                                ),
                                fx,
                            );
                        }
                        self.meta.session_id = Some(sid.to_string());
                        // None of the entries of the session it leaves is one of this one's.
                        self.meta.last_entry = None;
                        fx.save = true;
                    }
                    // A copy has a session of its own from its first turn on: its next starts
                    // resume that one, never the original's again.
                    let forked = self.meta.fork_of.take().is_some();
                    self.meta.fork_at = None;
                    if forked {
                        fx.save = true;
                    }
                }
                self.saw_init = true;
            }
            "status" => {
                if let Some(mode) = f["permissionMode"].as_str() {
                    if self.meta.mode != mode {
                        self.meta.mode = mode.to_string();
                        fx.agent_changed = true;
                        fx.save = true;
                    }
                }
            }
            "compact_boundary" => {
                // Its session's chain starts again at the boundary: the entries before it are no
                // longer in it (a copy forking at one would be refused), and the next one to come
                // is where a copy forks it.
                self.set_last_entry(None, fx);
                self.notice("info", tr!("Contexte compacté", "Context compacted"), fx);
            }
            "task_started" => {
                if let (Some(id), false) = (f["task_id"].as_str(), f["is_backgrounded"] == true) {
                    self.foreground_tasks.insert(id.to_string());
                }
            }
            // A task ended. Idle, Claude Code only tells this before starting a turn by itself;
            // mid-turn, it also replays the notification to Claude (not shown again).
            "task_notification" => {
                let Some(id) = f["task_id"].as_str() else {
                    return;
                };
                if self.foreground_tasks.remove(id) {
                    return;
                }
                let field = |k: &str| f[k].as_str().unwrap_or_default();
                // Written as Claude Code writes it for Claude, read the same way.
                let text = format!(
                    "<task-notification>\n<task-id>{id}</task-id>\n<tool-use-id>{}</tool-use-id>\n<status>{}</status>\n<summary>{}</summary>\n</task-notification>",
                    field("tool_use_id"),
                    field("status"),
                    field("summary")
                );
                let item_id = f["uuid"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(new_id);
                self.append(
                    json!({ "kind": "event", "id": item_id, "source": "task", "text": text, "ts": now_ms() }),
                    fx,
                );
                *self.announced_tasks.entry(id.to_string()).or_default() += 1;
                fx.save = true;
            }
            "bridge_state" => {
                self.remote_state = f["state"].as_str().map(str::to_string);
                fx.agent_changed = true;
            }
            "local_command_output" => {
                let text = f["content"].as_str().unwrap_or("").to_string();
                if !text.trim().is_empty() {
                    self.notice("info", strip_tags(&text), fx);
                }
            }
            _ => {}
        }
    }

    fn on_stream(&mut self, f: &Value, fx: &mut Effects) {
        let ev = &f["event"];
        let parent = f["parent_tool_use_id"].as_str().map(str::to_string);
        let pkey = parent.clone().unwrap_or_default();
        match ev["type"].as_str().unwrap_or("") {
            "message_start" => {
                let Some(mid) = ev["message"]["id"].as_str() else {
                    return;
                };
                self.current_msg.insert(pkey, mid.to_string());
                self.blocks.entry(mid.to_string()).or_default();
                let model = ev["message"]["model"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                self.live.insert(
                    mid.to_string(),
                    (model, Usage::from_api(&ev["message"]["usage"])),
                );
                fx.agent_changed = true;
                if parent.is_none() {
                    if let Some(m) = ev["message"]["model"].as_str() {
                        self.main_model = m.to_string();
                    }
                    let u = &ev["message"]["usage"];
                    self.context_tokens = [
                        &u["input_tokens"],
                        &u["cache_read_input_tokens"],
                        &u["cache_creation_input_tokens"],
                    ]
                    .iter()
                    .filter_map(|v| v.as_u64())
                    .sum();
                    fx.agent_changed = true;
                }
                // A turn started without a user message from us (queued message, background task).
                if !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "content_block_start" => {
                let Some(mid) = self.current_msg.get(&pkey).cloned() else {
                    return;
                };
                let index = ev["index"].as_u64().unwrap_or(0);
                let cb = &ev["content_block"];
                let (kind, item) = match cb["type"].as_str().unwrap_or("") {
                    "text" => (
                        "text",
                        json!({ "kind": "text", "id": format!("{mid}:{index}"), "text": "", "parent": parent, "streaming": true }),
                    ),
                    "thinking" => (
                        "thinking",
                        json!({ "kind": "thinking", "id": format!("{mid}:{index}"), "text": "", "parent": parent, "streaming": true }),
                    ),
                    "tool_use" | "server_tool_use" => (
                        "tool",
                        json!({
                            "kind": "tool", "id": cb["id"], "name": cb["name"], "input": {}, "status": "running",
                            "parent": parent, "ts": now_ms(),
                        }),
                    ),
                    _ => return,
                };
                if parent.is_none() {
                    let activity = block_activity(i18n::ui(), kind).or_else(|| {
                        tool_activity(
                            cb["name"].as_str().unwrap_or_default(),
                            &Value::Null,
                            &self.meta.cwd,
                        )
                    });
                    self.set_activity(activity, fx);
                }
                let item_id = item["id"].as_str().unwrap_or_default().to_string();
                self.append(item, fx);
                self.blocks.entry(mid).or_default().push(Block {
                    index,
                    item_id,
                    kind,
                    finalized: false,
                });
            }
            "content_block_delta" => {
                let Some(mid) = self.current_msg.get(&pkey) else {
                    return;
                };
                let index = ev["index"].as_u64().unwrap_or(0);
                let Some(block) = self
                    .blocks
                    .get(mid)
                    .and_then(|b| b.iter().find(|b| b.index == index))
                else {
                    return;
                };
                let d = &ev["delta"];
                let text = match d["type"].as_str().unwrap_or("") {
                    "text_delta" => d["text"].as_str(),
                    "thinking_delta" => d["thinking"].as_str(),
                    _ => None,
                };
                if let Some(text) = text.filter(|t| !t.is_empty()) {
                    let id = block.item_id.clone();
                    self.push(
                        ConvOp::Delta {
                            id,
                            text: text.to_string(),
                        },
                        fx,
                    );
                }
            }
            // The message's final output token count (message_start only announces its input).
            "message_delta" => {
                let entry = self
                    .current_msg
                    .get(&pkey)
                    .and_then(|mid| self.live.get_mut(mid));
                if let (Some((_, u)), Some(out)) = (entry, ev["usage"]["output_tokens"].as_u64()) {
                    u.output = out;
                    fx.agent_changed = true;
                }
            }
            _ => {}
        }
    }

    fn on_assistant(&mut self, f: &Value, fx: &mut Effects) {
        let msg = &f["message"];
        let mid = msg["id"].as_str().unwrap_or("").to_string();
        let parent = f["parent_tool_use_id"].as_str().map(str::to_string);
        if let Some(err) = f["error"].as_str() {
            if err == "rate_limit" && parent.is_none() {
                self.rate_limited = true;
            }
            let text = msg["content"][0]["text"]
                .as_str()
                .unwrap_or(err)
                .to_string();
            self.notice("error", text, fx);
            return;
        }
        // An entry of its session's main chain (not a subagent's, nor one Claude Code makes up):
        // where a copy made afterwards forks the session.
        if parent.is_none() && f["is_meta"] != true {
            if let Some(uuid) = f["uuid"].as_str() {
                self.set_last_entry(Some(uuid.to_string()), fx);
            }
        }
        let Some(content) = msg["content"].as_array() else {
            return;
        };
        for block in content {
            let kind = match block["type"].as_str().unwrap_or("") {
                "text" => "text",
                "thinking" | "redacted_thinking" => "thinking",
                "tool_use" | "server_tool_use" => "tool",
                _ => continue,
            };
            if parent.is_none() {
                match kind {
                    "text" => {
                        let text = block["text"].as_str().unwrap_or_default();
                        if !text.trim().is_empty() {
                            if !self.turn_text.is_empty() {
                                self.turn_text.push_str("\n\n");
                            }
                            self.turn_text.push_str(text);
                            self.final_text = text.to_string();
                        }
                    }
                    "tool" => {
                        // What was said before a tool was a step on the way, not the final reply.
                        self.final_text.clear();
                        let activity = tool_activity(
                            block["name"].as_str().unwrap_or_default(),
                            &block["input"],
                            &self.meta.cwd,
                        );
                        self.set_activity(activity, fx);
                    }
                    _ => {}
                }
            }
            if kind == "tool" {
                self.plan_tool_use(parent.as_deref(), block, fx);
            }
            let streamed = self
                .blocks
                .get_mut(&mid)
                .and_then(|v| v.iter_mut().find(|b| !b.finalized && b.kind == kind))
                .map(|b| {
                    b.finalized = true;
                    b.item_id.clone()
                });
            match (kind, streamed) {
                ("text", Some(id)) => self.patch(
                    &id,
                    json!({ "text": block["text"], "streaming": false }),
                    fx,
                ),
                ("thinking", Some(id)) => {
                    let text = block["thinking"].as_str().unwrap_or("");
                    self.patch(&id, json!({ "text": text, "streaming": false }), fx)
                }
                ("tool", Some(id)) => {
                    self.patch(&id, json!({ "input": trim_strings(&block["input"]) }), fx)
                }
                (_, None) => {
                    let n = self.blocks.get(&mid).map_or(0, Vec::len);
                    let item = match kind {
                        "text" => {
                            json!({ "kind": "text", "id": format!("{mid}:a{n}"), "text": block["text"], "parent": parent, "streaming": false })
                        }
                        "thinking" => {
                            json!({ "kind": "thinking", "id": format!("{mid}:a{n}"), "text": block["thinking"].as_str().unwrap_or(""), "parent": parent, "streaming": false })
                        }
                        _ => json!({
                            "kind": "tool", "id": block["id"], "name": block["name"], "input": trim_strings(&block["input"]),
                            "status": "running", "parent": parent, "ts": now_ms(),
                        }),
                    };
                    let item_id = item["id"].as_str().unwrap_or_default().to_string();
                    self.append(item, fx);
                    self.blocks.entry(mid.clone()).or_default().push(Block {
                        index: u64::MAX,
                        item_id,
                        kind,
                        finalized: true,
                    });
                }
                _ => {}
            }
        }
    }

    /// A tool the assistant calls, for the plan: its task list, a subagent it launches, a tool one
    /// of its subagents (`parent`) uses and what it makes of it.
    fn plan_tool_use(&mut self, parent: Option<&str>, block: &Value, fx: &mut Effects) {
        let (Some(id), Some(name)) = (block["id"].as_str(), block["name"].as_str()) else {
            return;
        };
        let input = &block["input"];
        if parent.is_none() {
            self.watch_plan_files(id, name, input);
        }
        let doing = parent.and_then(|_| tool_activity(name, input, &self.meta.cwd));
        let now = now_ms();
        self.plan_apply(fx, |plan| {
            let change = plan.on_tool_use(parent, id, name, input, now);
            match parent {
                Some(p) => change.and(plan.on_child_activity(p, doing.as_deref())),
                None => change,
            }
        });
    }

    /// A call of the main thread after which the files of the plan may have moved: a command (the
    /// ledger is written by scripts and commands), a subagent, the write of a plan or of a file of
    /// its workspace (the ledger, a brief). Its result has them looked at
    /// (`Effects::plan_files`); a plan it writes is remembered.
    fn watch_plan_files(&mut self, id: &str, name: &str, input: &Value) {
        let wrote = matches!(name, "Write" | "Edit" | "MultiEdit")
            && input["file_path"].as_str().is_some_and(|path| {
                let plan = self.note_written_plan(path);
                plan || in_sdd_workspace(path)
            });
        if wrote || matches!(name, "Bash" | "Task" | "Agent") {
            if self.plan_calls.len() >= MAX_PLAN_CALLS {
                self.plan_calls.clear();
            }
            self.plan_calls.insert(id.to_string());
        }
    }

    /// `path`, as a tool was given it, is a plan of the plans folder: kept (absolute) as the
    /// latest the agent wrote, five at most. Whether it is inside the repository, and there, is
    /// for `planfiles` to say when it reads it.
    fn note_written_plan(&mut self, path: &str) -> bool {
        const PLANS: &str = "docs/superpowers/plans/";
        // Behind a slash, or at the start of a path relative to its folder.
        let slashed = path.replace('\\', "/").to_ascii_lowercase();
        let in_plans = slashed.starts_with(PLANS) || slashed.contains(&format!("/{PLANS}"));
        if !slashed.ends_with(".md") || !in_plans {
            return false;
        }
        let absolute = slashed.starts_with('/') || slashed.chars().nth(1) == Some(':');
        let path = if absolute {
            path.to_string()
        } else {
            format!("{}/{}", self.meta.cwd.trim_end_matches(['/', '\\']), path)
        };
        self.plan_written.retain(|p| *p != path);
        self.plan_written.push(path);
        if self.plan_written.len() > MAX_PLANS_WRITTEN {
            self.plan_written.remove(0);
        }
        true
    }

    /// A user message re-emitted by Claude Code (`--replay-user-messages`): the echo of one sent
    /// from the app (same uuid, already listed), or one sent from claude.ai (Remote Control).
    fn on_replay(&mut self, f: &Value, fx: &mut Effects) {
        let id = f["uuid"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(new_id);
        if self.conv.contains(&id) {
            return;
        }
        let content = &f["message"]["content"];
        let blocks = content.as_array().map(Vec::as_slice).unwrap_or_default();
        let text = match content.as_str() {
            Some(s) => s.to_string(),
            None => blocks
                .iter()
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        };
        // Only a human's message is the user's (no origin, or "human", as Claude Code tells). The
        // rest Claude Code passes on to Claude by itself (a subagent's report, a background task
        // that ended, an MCP channel, a meta message): an event.
        let kind = f["origin"]["kind"].as_str();
        if kind.is_some_and(|k| k != "human") || f["isSynthetic"] == true {
            if text.trim().is_empty() {
                return;
            }
            if kind == Some("task-notification") {
                let task = text
                    .split_once("<task-id>")
                    .and_then(|(_, rest)| rest.split_once("</task-id>"))
                    .map(|(id, _)| id.trim().to_string());
                if let Some(n) = task.and_then(|t| self.announced_tasks.get_mut(&t)) {
                    if *n > 0 {
                        *n -= 1;
                        return;
                    }
                }
            }
            let source = match kind {
                Some("task-notification") => "task",
                Some("peer") => "agent",
                Some(k) => k,
                None => "synthetic",
            };
            let mut item = json!({ "kind": "event", "id": id, "source": source, "text": text, "ts": now_ms() });
            if let Some(from) = f["origin"]["from"].as_str() {
                item["from"] = from.into();
            }
            self.append(item, fx);
            fx.save = true;
            return;
        }
        let images = blocks.iter().filter(|b| b["type"] == "image").count() as u32;
        let files: Vec<String> = blocks
            .iter()
            .filter(|b| b["type"] == "document")
            .map(|b| b["title"].as_str().unwrap_or("document").to_string())
            .collect();
        if text.trim().is_empty() && images == 0 && files.is_empty() {
            return;
        }
        self.push_user_from(&id, &text, images, &files, Some("remote"), fx);
    }

    fn on_user(&mut self, f: &Value, fx: &mut Effects) {
        if f["isReplay"] == true {
            return self.on_replay(f, fx);
        }
        let content = &f["message"]["content"];
        if let Some(text) = content.as_str() {
            if text.contains("<local-command-stdout>") || text.contains("<local-command-stderr>") {
                let clean = strip_tags(text);
                if !clean.trim().is_empty() {
                    self.notice("info", clean, fx);
                }
            }
            return;
        }
        let Some(blocks) = content.as_array() else {
            return;
        };
        // A tool's result in its main chain is an entry of its session too: the last one of a turn
        // stopped (or failed) before any reply, which a copy must not lose.
        if f["parent_tool_use_id"].is_null() && blocks.iter().any(|b| b["type"] == "tool_result") {
            if let Some(uuid) = f["uuid"].as_str() {
                self.set_last_entry(Some(uuid.to_string()), fx);
            }
        }
        let tur = &f["tool_use_result"];
        for b in blocks.iter().filter(|b| b["type"] == "tool_result") {
            let Some(id) = b["tool_use_id"].as_str() else {
                continue;
            };
            let is_error = b["is_error"].as_bool().unwrap_or(false);
            if self.plan_calls.remove(id) {
                fx.plan_files = true;
            }
            let mut text = tool_result_text(&b["content"]);
            self.plan_apply(fx, |plan| {
                plan.on_tool_result(id, is_error, &text, tur, now_ms())
            });
            let mut result = json!({ "isError": is_error });
            if let Some(stdout) = tur["stdout"].as_str() {
                text = stdout.to_string();
                if let Some(stderr) = tur["stderr"].as_str().filter(|s| !s.is_empty()) {
                    text = format!("{text}\n{stderr}");
                }
            }
            result["text"] = Value::String(truncate(text.trim_end(), MAX_TEXT));
            if let Some(hunks) = tur["structuredPatch"].as_array() {
                let (add, del, patch) = if hunks.is_empty() && tur["type"] == "create" {
                    created_file_patch(tur["content"].as_str().unwrap_or(""))
                } else {
                    summarize_patch(hunks)
                };
                result["add"] = add.into();
                result["del"] = del.into();
                result["patch"] = patch;
                if let Some(path) = tur["filePath"].as_str() {
                    result["filePath"] = path.into();
                    self.touch(path, fx);
                }
            }
            self.patch(
                id,
                json!({ "status": if is_error { "error" } else { "ok" }, "result": result }),
                fx,
            );
        }
    }

    fn touch(&mut self, path: &str, fx: &mut Effects) {
        let rel = relative_slash(&self.meta.cwd, path);
        if !self
            .meta
            .touched_files
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&rel))
        {
            self.meta.touched_files.push(rel);
            fx.save = true;
        }
        fx.files_changed = true;
    }

    /// Takes `entry` as the latest entry of its session, saved once it changes, not at the end of
    /// the turn only: an app stopped for good mid-turn would leave a copy to fork at an older one.
    /// The saves asked for are grouped (one every 400 ms at most), not one per frame.
    fn set_last_entry(&mut self, entry: Option<String>, fx: &mut Effects) {
        if self.meta.last_entry != entry {
            self.meta.last_entry = entry;
            fx.save = true;
        }
    }

    fn on_result(&mut self, f: &Value, fx: &mut Effects) {
        let interrupted = std::mem::take(&mut self.interrupted);
        let resets = self.limit_resets_at.take();
        let limited = std::mem::take(&mut self.rate_limited);
        // The exact figures below replace the running estimate.
        self.live.clear();
        let is_error = f["is_error"].as_bool().unwrap_or(false) || f["subtype"] != "success";
        if limited && is_error && !interrupted {
            fx.limited = Some(resets);
        } else if self.meta.resume_at.take().is_some() {
            // It went on anyway (a turn Claude Code started by itself): nothing left to resume.
            fx.save = true;
            fx.agent_changed = true;
        }
        let (mut tokens, mut cost) = (0u64, 0f64);
        if let Some(models) = f["modelUsage"].as_object() {
            // The conversation's latest model, else the one that read the most (subagents may use
            // others; the figures are cumulative, so an earlier model may still read more).
            let m = &self.main_model;
            let current = models
                .iter()
                .find(|(k, _)| {
                    !m.is_empty() && (k.starts_with(m.as_str()) || m.starts_with(k.as_str()))
                })
                .map(|(_, u)| u);
            let main = current.or_else(|| {
                models.values().max_by_key(|u| {
                    [
                        "inputTokens",
                        "cacheReadInputTokens",
                        "cacheCreationInputTokens",
                    ]
                    .iter()
                    .filter_map(|k| u[*k].as_u64())
                    .sum::<u64>()
                })
            });
            if let Some(window) = main.and_then(|u| u["contextWindow"].as_u64()) {
                self.context_window = window;
            }
            for (model, u) in models {
                let cur = Counters {
                    input: u["inputTokens"].as_u64().unwrap_or(0),
                    output: u["outputTokens"].as_u64().unwrap_or(0),
                    cache: u["cacheReadInputTokens"].as_u64().unwrap_or(0)
                        + u["cacheCreationInputTokens"].as_u64().unwrap_or(0),
                    cost: u["costUSD"].as_f64().unwrap_or(0.0),
                };
                let prev = self.last_usage.get(model).cloned().unwrap_or_default();
                let delta = if cur.cost + 1e-12 < prev.cost || cur.output < prev.output {
                    cur.clone()
                } else {
                    Counters {
                        input: cur.input.saturating_sub(prev.input),
                        output: cur.output.saturating_sub(prev.output),
                        cache: cur.cache.saturating_sub(prev.cache),
                        cost: (cur.cost - prev.cost).max(0.0),
                    }
                };
                self.last_usage.insert(model.clone(), cur);
                let total = delta.input + delta.output + delta.cache;
                if total == 0 && delta.cost == 0.0 {
                    continue;
                }
                tokens += total;
                cost += delta.cost;
                let name = u["canonicalModel"].as_str().unwrap_or(model).to_string();
                fx.turns.push(TurnRow {
                    model: name,
                    input: delta.input,
                    cache: delta.cache,
                    output: delta.output,
                    cost: delta.cost,
                });
            }
        }
        self.meta.tokens += tokens;
        self.meta.cost += cost;
        let error = if is_error && !interrupted {
            f["result"].as_str().map(str::to_string).or_else(|| {
                f["errors"].as_array().map(|e| {
                    e.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
            })
        } else {
            None
        };
        let failed = is_error && !interrupted;
        // Always with words: the frame's, else the kind of result it is, else a generic one.
        let failure = failed.then(|| {
            error
                .clone()
                .filter(|e| !e.trim().is_empty())
                .or_else(|| {
                    f["subtype"]
                        .as_str()
                        .filter(|s| !s.is_empty() && *s != "success")
                        .map(str::to_string)
                })
                .unwrap_or_else(|| {
                    tr!(
                        "Claude Code a signalé une erreur",
                        "Claude Code reported an error"
                    )
                })
        });
        // The final reply: the text that no tool followed, else what Claude Code makes of it.
        let reply = match std::mem::take(&mut self.final_text) {
            text if !text.trim().is_empty() => text,
            _ => f["result"].as_str().unwrap_or_default().to_string(),
        };
        let end = if interrupted {
            TurnEnd::Interrupted
        } else if limited && is_error {
            TurnEnd::Limited
        } else if let Some(reason) = failure.clone() {
            TurnEnd::Error(reason)
        } else {
            let text = std::mem::take(&mut self.turn_text);
            TurnEnd::Finished(if text.trim().is_empty() {
                f["result"].as_str().unwrap_or_default().to_string()
            } else {
                text
            })
        };
        self.forget_turn();
        self.close_open_items(fx);
        // A subagent in the foreground did not outlive the turn: it was stopped.
        self.plan_apply(fx, |plan| plan.end_turn(now_ms()));
        // The turn's last commands may have written the ledger.
        fx.plan_files = true;
        let item = json!({
            "kind": "turn", "id": f["uuid"].as_str().map(str::to_string).unwrap_or_else(new_id), "ts": now_ms(),
            "durationMs": f["duration_ms"], "cost": cost, "tokens": tokens,
            "isError": is_error && !interrupted, "interrupted": interrupted, "error": error,
        });
        self.append(item, fx);
        if !self.pending.is_empty() {
            self.clear_pending(fx);
        }
        let had_queue = self.queued > 0;
        if had_queue {
            self.queued = 0;
            let ids = self
                .conv
                .ids_where(|v| v["kind"] == "user" && v["queued"] == true);
            for id in ids {
                self.patch(&id, json!({ "queued": false }), fx);
            }
        }
        self.set_status(
            if failed {
                AgentStatus::Error
            } else {
                AgentStatus::Done
            },
            fx,
        );
        if !interrupted && !had_queue {
            fx.notify = Some(match &failure {
                Some(reason) => AgentAlert::new(NotifyKind::Error, notify::error_body(reason)),
                None => AgentAlert::new(NotifyKind::Done, notify::done_body(&reply)),
            });
        }
        fx.agent_changed = true;
        fx.save = true;
        fx.turn_end = Some(end);
    }

    fn on_control_request(&mut self, f: &Value, fx: &mut Effects) {
        let Some(rid) = f["request_id"].as_str() else {
            return;
        };
        let req = &f["request"];
        match req["subtype"].as_str().unwrap_or("") {
            "can_use_tool" => {
                let tool = req["tool_name"].as_str().unwrap_or("").to_string();
                let input = req["input"].clone();
                let tool_use_id = req["tool_use_id"].as_str().unwrap_or("").to_string();
                let suggestions = req["permission_suggestions"].clone();
                let body = if tool == "AskUserQuestion" {
                    notify::question_body(&input)
                } else {
                    notify::permission_body(&tool, &input, &self.meta.cwd)
                };
                let item = if tool == "AskUserQuestion" {
                    json!({
                        "kind": "question", "id": rid, "toolUseId": tool_use_id,
                        "questions": input["questions"], "answers": null, "ts": now_ms(),
                    })
                } else {
                    let can_always = suggestions.as_array().is_some_and(|a| !a.is_empty())
                        && !req["suppress_always_allow_rule"].as_bool().unwrap_or(false);
                    json!({
                        "kind": "permission", "id": rid, "toolUseId": tool_use_id, "toolName": tool,
                        "title": req["title"], "description": req["description"],
                        "input": trim_strings(&input),
                        "reason": req["decision_reason"].as_str().map(strip_ansi),
                        "canAlways": can_always, "defaultNo": req["default_to_no"].as_bool().unwrap_or(false),
                        "decision": null, "ts": now_ms(),
                    })
                };
                self.append(item, fx);
                let view = pending_view(rid, &tool, req, &input, &self.meta.cwd);
                self.add_pending(PendingReq {
                    item_id: rid.to_string(),
                    tool_use_id,
                    input,
                    suggestions,
                    view,
                });
                self.set_status(AgentStatus::Waiting, fx);
                fx.notify = Some(AgentAlert::new(NotifyKind::Question, body));
                fx.agent_changed = true;
            }
            "elicitation" => {
                if let Some(p) = &self.proc {
                    let _ = p.respond(rid, json!({ "action": "decline" }));
                }
            }
            other => {
                if let Some(p) = &self.proc {
                    let _ = p.respond_error(rid, &format!("unsupported request: {other}"));
                }
            }
        }
    }

    fn on_cancel(&mut self, f: &Value, fx: &mut Effects) {
        let Some(rid) = f["request_id"].as_str() else {
            return;
        };
        if let Some(p) = self.take_pending(rid) {
            self.patch(&p.item_id, json!({ "cancelled": true }), fx);
            if self.pending.is_empty() && self.meta.status == AgentStatus::Waiting {
                self.set_status(AgentStatus::Running, fx);
            }
            fx.agent_changed = true;
        }
    }
}

pub fn parse_rate_event(info: &Value) -> (Option<RateWindow>, Option<RateWindow>) {
    let w = &info["unifiedWindows"];
    let parse = |k: &str| {
        w.get(k).filter(|v| v.is_object()).map(|v| RateWindow {
            pct: v["utilization"].as_f64().unwrap_or(0.0) * 100.0,
            resets_at: v["resetsAt"].as_i64().map(|s| s * 1000),
        })
    };
    (parse("five_hour"), parse("seven_day"))
}

fn tool_result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn summarize_patch(hunks: &[Value]) -> (u32, u32, Value) {
    let (mut add, mut del, mut kept_total) = (0u32, 0u32, 0usize);
    let mut out = Vec::new();
    for h in hunks {
        let lines = h["lines"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        for l in lines {
            match l.as_str().unwrap_or("").chars().next() {
                Some('+') => add += 1,
                Some('-') => del += 1,
                _ => {}
            }
        }
        if kept_total < MAX_PATCH_LINES {
            let kept: Vec<Value> = lines
                .iter()
                .take(MAX_PATCH_LINES - kept_total)
                .map(|l| Value::String(truncate(l.as_str().unwrap_or(""), 400)))
                .collect();
            kept_total += kept.len();
            out.push(
                json!({ "oldStart": h["oldStart"], "newStart": h["newStart"], "lines": kept }),
            );
        }
    }
    (add, del, Value::Array(out))
}

fn created_file_patch(content: &str) -> (u32, u32, Value) {
    let lines: Vec<&str> = content.lines().collect();
    let kept: Vec<Value> = lines
        .iter()
        .take(MAX_PATCH_LINES)
        .map(|l| Value::String(format!("+{}", truncate(l, 400))))
        .collect();
    (
        lines.len() as u32,
        0,
        json!([{ "oldStart": 0, "newStart": 1, "lines": kept }]),
    )
}

/// Truncates long strings inside tool inputs (file contents, big edits) before they reach the UI.
fn trim_strings(v: &Value) -> Value {
    match v {
        Value::String(s) if s.len() > MAX_INPUT_STR => Value::String(truncate(s, MAX_INPUT_STR)),
        Value::Array(a) => Value::Array(a.iter().map(trim_strings).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), trim_strings(v)))
                .collect::<Map<_, _>>(),
        ),
        other => other.clone(),
    }
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    strip_ansi(out.trim())
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `text` in `MAX_PENDING_FIELD` characters at most; `cut` is set when it was longer.
fn capped(text: &str, cut: &mut bool) -> String {
    if text.chars().count() <= MAX_PENDING_FIELD {
        return text.to_string();
    }
    *cut = true;
    text.chars().take(MAX_PENDING_FIELD).collect()
}

/// How Claude Code names the tools of Escouade's own MCP server: `mcp__escouade__<tool>`.
const ESCOUADE_TOOLS: &str = "mcp__escouade__";

/// What a tool of Escouade is about, first found first (`toolArg` in `src/lib/tools.ts` reads
/// them in the same order): the ticket it makes, the ticket or the agent it acts on, what an agent
/// it starts is told, the line an agent reports, the project it reads.
const ESCOUADE_MAIN_ARGS: &[&str] = &["title", "ticket", "agent", "message", "line", "project"];

/// What a tool is asked to act on, as the conversation's card sums it up (`toolArg` in
/// `src/lib/tools.ts`): the command, the file from the agent's folder, the search…, its line
/// breaks kept. `SUMMED_UP` there lists the tools whose summary says all a permission asks:
/// only those are answered from « Vue d'ensemble ».
fn tool_arg(tool: &str, input: &Value, cwd: &str) -> String {
    tool_arg_in(i18n::ui(), tool, input, cwd)
}

fn tool_arg_in(lang: Lang, tool: &str, input: &Value, cwd: &str) -> String {
    let text = |key: &str| input[key].as_str().unwrap_or_default().to_string();
    let path = |key: &str| {
        input[key]
            .as_str()
            .map(|p| relative_slash(cwd, p))
            .unwrap_or_default()
    };
    match tool {
        "Bash" | "PowerShell" | "SlashCommand" => text("command"),
        "Read" | "Edit" | "Write" | "MultiEdit" => path("file_path"),
        "NotebookEdit" => path("notebook_path"),
        "Grep" | "Glob" => match path("path") {
            p if p.is_empty() => text("pattern"),
            p => format!("{}  {p}", text("pattern")),
        },
        "WebFetch" => text("url"),
        "WebSearch" => text("query"),
        "Task" | "Agent" => input["description"]
            .as_str()
            .or(input["subagent_type"].as_str())
            .unwrap_or_default()
            .to_string(),
        "TodoWrite" => input["todos"]
            .as_array()
            .map(|t| {
                let n = t.len();
                tr_n_in!(lang, n, "{n} tâche", "{n} tâches", "{n} task", "{n} tasks")
            })
            .unwrap_or_default(),
        "Skill" => input["skill"]
            .as_str()
            .or(input["command"].as_str())
            .unwrap_or_default()
            .to_string(),
        // A tool of Escouade's own server: its main argument (the ticket's title, the ticket, the
        // agent…), whatever comes first in what Claude sends. Its card lists them all.
        t if t.starts_with(ESCOUADE_TOOLS) => ESCOUADE_MAIN_ARGS
            .iter()
            .find_map(|k| input[*k].as_str())
            .unwrap_or_default()
            .to_string(),
        // The first text of its input (the keys in their order, as the card reads them).
        _ => input
            .as_object()
            .and_then(|o| o.values().find_map(Value::as_str))
            .unwrap_or_default()
            .to_string(),
    }
}

/// The first `MAX_PENDING_ITEMS` of the list `list`; `cut` is set when it had more.
fn few<'a>(list: &'a Value, cut: &mut bool) -> &'a [Value] {
    let list = list.as_array().map(Vec::as_slice).unwrap_or_default();
    *cut |= list.len() > MAX_PENDING_ITEMS;
    &list[..list.len().min(MAX_PENDING_ITEMS)]
}

/// What the agent's view says of the request `rid` (`req`, Claude Code's `can_use_tool`) for
/// « Vue d'ensemble », which shows it without the conversation: each text capped, `cut` set
/// when one was longer.
fn pending_view(rid: &str, tool: &str, req: &Value, input: &Value, cwd: &str) -> PendingView {
    let mut cut = false;
    let text = |v: &Value, cut: &mut bool| v.as_str().map(|s| capped(s, cut));
    if tool == "AskUserQuestion" {
        let questions = few(&input["questions"], &mut cut)
            .iter()
            .map(|q| PendingQuestion {
                question: text(&q["question"], &mut cut).unwrap_or_default(),
                options: few(&q["options"], &mut cut)
                    .iter()
                    .map(|o| text(&o["label"], &mut cut).unwrap_or_default())
                    .collect(),
            })
            .collect();
        return PendingView {
            id: rid.to_string(),
            kind: "question".into(),
            tool: tool.to_string(),
            arg: String::new(),
            description: None,
            reason: None,
            questions,
            cut,
            default_no: false,
        };
    }
    let arg = capped(&tool_arg(tool, input, cwd), &mut cut);
    let description = text(&req["description"], &mut cut);
    let reason = req["decision_reason"]
        .as_str()
        .map(|r| capped(&strip_ansi(r), &mut cut));
    PendingView {
        id: rid.to_string(),
        kind: "permission".into(),
        tool: tool.to_string(),
        arg,
        description,
        reason,
        questions: vec![],
        cut,
        default_no: req["default_to_no"].as_bool().unwrap_or(false),
    }
}

/// What Claude is told of an action the user refused without saying why, in `lang` (the language
/// of the texts for Claude).
fn refused_action(lang: Lang) -> String {
    tr_in!(
        lang,
        "L'utilisateur a refusé cette action.",
        "The user refused this action."
    )
}

/// Refused: the agent's process is gone, nothing can be answered.
fn not_running() -> anyhow::Error {
    anyhow!(tr!(
        "Claude ne tourne plus pour cet agent",
        "Claude is no longer running for this agent"
    ))
}

/// What the agent's conversation is told when Claude Code went before it started: the session to
/// resume is gone, the point of the original's session a copy starts from is gone.
fn exit_notices(lang: Lang) -> [String; 2] {
    [
        tr_in!(
            lang,
            "Session Claude introuvable : une nouvelle session sera démarrée au prochain message.",
            "Claude session not found: a new session will start with your next message."
        ),
        tr_in!(
            lang,
            "Point de copie introuvable dans la session de l'original : Claude reprend toute sa conversation, jusqu'à maintenant.",
            "Copy point not found in the original’s session: Claude picks up its whole conversation, up to now."
        ),
    ]
}

/// Why the agent's turn ended in an error, when Claude Code stopped with `code`.
fn stopped(lang: Lang, code: &str) -> String {
    tr_in!(
        lang,
        "Claude Code s'est arrêté (code {code})",
        "Claude Code stopped (code {code})"
    )
}

/// What the agent does while it streams a block of text or of thinking; None for a tool's.
fn block_activity(lang: Lang, kind: &str) -> Option<String> {
    match kind {
        "text" => Some(tr_in!(lang, "Rédige", "Replying")),
        "thinking" => Some(tr_in!(lang, "Réfléchit", "Thinking")),
        _ => None,
    }
}

/// What the agent does, from the tool it runs: "Lit src/db.ts", "Lance npm test"… None for a tool
/// without words of its own.
pub fn tool_activity(name: &str, input: &Value, cwd: &str) -> Option<String> {
    tool_activity_in(i18n::ui(), name, input, cwd)
}

/// `tool_activity` in `lang`. A verb alone is the start of the same verb followed by what it acts
/// on ("Lit", "Lit src/a.ts"; “Reading”, “Reading src/a.ts”): `set_activity` relies on it.
fn tool_activity_in(lang: Lang, name: &str, input: &Value, cwd: &str) -> Option<String> {
    let file = || {
        input["file_path"]
            .as_str()
            .or(input["notebook_path"].as_str())
            .map(|p| relative_slash(cwd, p))
            .unwrap_or_default()
    };
    let with = |verb: &str, what: String| {
        if what.is_empty() {
            verb.to_string()
        } else {
            format!("{verb} {what}")
        }
    };
    // The first line that says something, cut short.
    let gist = |text: &Value| {
        let first = text
            .as_str()
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or_default();
        truncate(first, 60)
    };
    Some(match name {
        "Read" => with(&tr_in!(lang, "Lit", "Reading"), file()),
        "Grep" | "Glob" => with(
            &tr_in!(lang, "Cherche", "Searching"),
            gist(&input["pattern"]),
        ),
        "Edit" | "MultiEdit" | "NotebookEdit" => with(&tr_in!(lang, "Modifie", "Editing"), file()),
        "Write" => with(&tr_in!(lang, "Écrit", "Writing"), file()),
        "Bash" => with(&tr_in!(lang, "Lance", "Running"), gist(&input["command"])),
        "Task" | "Agent" => tr_in!(lang, "Délègue", "Delegating"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt() -> AgentRt {
        AgentRt::new(
            AgentMeta {
                id: new_id(),
                cwd: "C:/p".into(),
                ..Default::default()
            },
            &std::env::temp_dir().join(format!("ccm-agent-tests-{}", std::process::id())),
        )
    }

    #[test]
    fn streaming_text_then_final_message() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m1","usage":{}}},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Bon"}},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"jour"}},"parent_tool_use_id":null}), &mut fx);
        assert_eq!(a.conv.get("m1:0").unwrap()["text"], "Bonjour");
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"Bonjour !"}]},"parent_tool_use_id":null}), &mut fx);
        let item = a.conv.get("m1:0").unwrap();
        assert_eq!(item["text"], "Bonjour !");
        assert_eq!(item["streaming"], false);
        assert_eq!(a.meta.status, AgentStatus::Running);
    }

    #[test]
    fn question_then_result_usage_deltas() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"AskUserQuestion","tool_use_id":"tu1","input":{"questions":[{"question":"Q?","options":[]}]}}}), &mut fx);
        assert_eq!(a.meta.status, AgentStatus::Waiting);
        assert_eq!(fx.notify, Some(AgentAlert::new(NotifyKind::Question, "Q?")));
        assert!(a.has_pending("r1"));

        let usage = |inp: u64, out: u64, cost: f64| {
            json!({"type":"result","subtype":"success","is_error":false,"duration_ms":10,
            "modelUsage":{"claude-sonnet-5":{"inputTokens":inp,"outputTokens":out,"cacheReadInputTokens":100,"cacheCreationInputTokens":0,"costUSD":cost}}})
        };
        let mut fx = Effects::default();
        a.handle_frame(&usage(10, 20, 0.5), &mut fx);
        assert_eq!(fx.turns.len(), 1);
        assert_eq!(fx.turns[0].input, 10);
        assert_eq!(fx.turns[0].cache, 100);
        assert!(!a.has_pending("r1"));
        assert_eq!(a.meta.status, AgentStatus::Done);

        let mut fx = Effects::default();
        a.handle_frame(&usage(15, 50, 0.8), &mut fx);
        assert_eq!(fx.turns[0].input, 5);
        assert_eq!(fx.turns[0].output, 30);
        assert_eq!(fx.turns[0].cache, 0);
        assert!((fx.turns[0].cost - 0.3).abs() < 1e-9);
        assert!((a.meta.cost - 0.8).abs() < 1e-9);
    }

    fn can_use_tool(a: &mut AgentRt, tool: &str, input: Value) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"control_request","request_id":"r1","request":{
                "subtype":"can_use_tool","tool_name":tool,"tool_use_id":"tu1","input":input}}),
            &mut fx,
        );
        fx
    }

    fn says(a: &mut AgentRt, id: &str, text: &str) {
        a.handle_frame(
            &json!({"type":"assistant","message":{"id":id,"content":[{"type":"text","text":text}]},"parent_tool_use_id":null}),
            &mut Effects::default(),
        );
    }

    fn runs_tool(a: &mut AgentRt, id: &str) {
        a.handle_frame(
            &json!({"type":"assistant","message":{"id":id,"content":[{"type":"tool_use","id":format!("t-{id}"),"name":"Bash","input":{"command":"npm test"}}]},"parent_tool_use_id":null}),
            &mut Effects::default(),
        );
    }

    /// The turn's result frame, with Claude Code's own `result` text when given.
    fn ends(a: &mut AgentRt, result: Option<&str>) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,
                "result":result,"modelUsage":{}}),
            &mut fx,
        );
        fx
    }

    #[test]
    fn a_permission_notifies_with_the_tool_and_what_it_acts_on() {
        let mut a = rt();
        let fx = can_use_tool(&mut a, "Bash", json!({"command":"rm -rf build"}));
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(
                NotifyKind::Question,
                "Autoriser Bash : rm -rf build ?"
            ))
        );
        // A file is told from the agent's folder.
        let fx = can_use_tool(&mut a, "Edit", json!({"file_path":"C:/p/src/app.ts"}));
        assert_eq!(
            fx.notify.map(|n| n.body),
            Some("Autoriser Edit : src/app.ts ?".to_string())
        );
        let fx = can_use_tool(&mut a, "WebFetch", json!({"url":"https://example.com/a"}));
        assert_eq!(
            fx.notify.map(|n| n.body),
            Some("Autoriser WebFetch : https://example.com/a ?".to_string())
        );
    }

    /// Claude Code asks `rid`: a permission, or a question for AskUserQuestion (`request` without its subtype).
    fn asks(a: &mut AgentRt, rid: &str, mut request: Value) {
        request["subtype"] = json!("can_use_tool");
        a.handle_frame(
            &json!({"type":"control_request","request_id":rid,"request":request}),
            &mut Effects::default(),
        );
    }

    #[test]
    fn the_view_lists_the_pending_requests_in_the_order_they_came() {
        let mut a = rt();
        // Enough of them for any other order to show.
        let ids: Vec<String> = (0..12).map(|i| format!("r{i}")).collect();
        for id in &ids {
            asks(
                &mut a,
                id,
                json!({"tool_name":"Bash","tool_use_id":format!("t-{id}"),"input":{"command":"npm test"}}),
            );
        }
        let v = a.view();
        assert_eq!(v.pending, ids);
        let shown: Vec<String> = v.requests.iter().map(|r| r.id.clone()).collect();
        assert_eq!(shown, ids);
        // One cancelled, the others keep their order.
        a.handle_frame(
            &json!({"type":"control_cancel_request","request_id":"r0"}),
            &mut Effects::default(),
        );
        assert_eq!(a.view().pending, ids[1..]);
    }

    #[test]
    fn the_view_sums_up_each_pending_request_without_the_conversation() {
        let mut a = rt();
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"Bash","tool_use_id":"t1","input":{"command":"npm ci\nnpm test"},
                "description":"Installe et teste","decision_reason":"Commande hors de la liste"}),
        );
        asks(
            &mut a,
            "r2",
            json!({"tool_name":"Edit","tool_use_id":"t2",
                "input":{"file_path":"C:/p/src/db.ts","old_string":"a","new_string":"b"}}),
        );
        asks(
            &mut a,
            "r3",
            json!({"tool_name":"AskUserQuestion","tool_use_id":"t3","input":{"questions":[
                {"question":"Quelle base ?","options":[{"label":"SQLite"},{"label":"Postgres","description":"un serveur"}]}]}}),
        );
        let v = a.view();
        assert_eq!(
            v.requests[0],
            PendingView {
                id: "r1".into(),
                kind: "permission".into(),
                tool: "Bash".into(),
                // Its lines kept: the overview shows them as they run.
                arg: "npm ci\nnpm test".into(),
                description: Some("Installe et teste".into()),
                reason: Some("Commande hors de la liste".into()),
                questions: vec![],
                cut: false,
                default_no: false,
            }
        );
        // A file from the agent's folder, as the conversation's card shows it.
        assert_eq!(v.requests[1].arg, "src/db.ts");
        assert_eq!(v.requests[2].kind, "question");
        assert_eq!(
            v.requests[2].questions,
            vec![PendingQuestion {
                question: "Quelle base ?".into(),
                options: vec!["SQLite".into(), "Postgres".into()],
            }]
        );
        let sent = serde_json::to_value(&v).unwrap();
        assert_eq!(sent["requests"][0]["arg"], "npm ci\nnpm test");
        assert_eq!(
            sent["requests"][2]["questions"][0]["options"][1],
            "Postgres"
        );
    }

    #[test]
    fn the_view_names_what_a_tool_of_escouade_acts_on_its_title_ticket_or_agent() {
        use crate::i18n::Lang::Fr;
        let arg = |tool: &str, input: Value| {
            tool_arg_in(Fr, &format!("mcp__escouade__{tool}"), &input, "C:/p")
        };
        // Its main argument, whatever comes first in what Claude sends.
        assert_eq!(
            arg(
                "create_ticket",
                json!({"project":"demo","title":"Corriger la connexion","description":"Le jeton expire"})
            ),
            "Corriger la connexion"
        );
        assert_eq!(
            arg(
                "update_ticket",
                json!({"description":"Plus court","ticket":"DEM-4"})
            ),
            "DEM-4"
        );
        assert_eq!(arg("start_ticket", json!({"ticket":"DEM-3"})), "DEM-3");
        assert_eq!(
            arg(
                "send_message",
                json!({"text":"Relis le test","agent":"fix-login"})
            ),
            "fix-login"
        );
        assert_eq!(
            arg(
                "get_agent_summary",
                json!({"project":"demo","agent":"fix-login"})
            ),
            "fix-login"
        );
        // Without one: what the agent is told, its line, its project.
        assert_eq!(
            arg(
                "create_agent",
                json!({"project":"demo","message":"Écris la doc","worktree":true})
            ),
            "Écris la doc"
        );
        assert_eq!(
            arg("report_progress", json!({"line":"Tests verts"})),
            "Tests verts"
        );
        assert_eq!(
            arg("list_tickets", json!({"project":"demo","column":"todo"})),
            "demo"
        );
        assert_eq!(arg("list_projects", json!({})), "");
        // Another server's tool: its first text, as before.
        assert_eq!(
            tool_arg_in(
                Fr,
                "mcp__github__create_issue",
                &json!({"title":"Bug"}),
                "C:/p"
            ),
            "Bug"
        );
        // And the overview is told it.
        let mut a = rt();
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"mcp__escouade__create_ticket","tool_use_id":"t1",
                "input":{"project":"demo","title":"Corriger la connexion"}}),
        );
        assert_eq!(a.view().requests[0].arg, "Corriger la connexion");
    }

    #[test]
    fn the_view_tells_a_request_claude_code_would_refuse_by_default() {
        let mut a = rt();
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"Bash","tool_use_id":"t1","input":{"command":"rm -rf /"},"default_to_no":true}),
        );
        asks(
            &mut a,
            "r2",
            json!({"tool_name":"Bash","tool_use_id":"t2","input":{"command":"ls"}}),
        );
        let v = a.view();
        assert!(v.requests[0].default_no);
        assert!(!v.requests[1].default_no);
        assert_eq!(
            serde_json::to_value(&v).unwrap()["requests"][0]["defaultNo"],
            true
        );
    }

    #[test]
    fn the_view_caps_what_a_pending_request_says_and_tells_it_was_cut() {
        let mut a = rt();
        let long = "é".repeat(MAX_PENDING_FIELD + 10);
        asks(
            &mut a,
            "r1",
            json!({"tool_name":"Bash","tool_use_id":"t1","input":{"command":long}}),
        );
        asks(
            &mut a,
            "r2",
            json!({"tool_name":"AskUserQuestion","tool_use_id":"t2","input":{"questions":[{"question":long,"options":[]}]}}),
        );
        asks(
            &mut a,
            "r3",
            json!({"tool_name":"Bash","tool_use_id":"t3","input":{"command":"ls"},"description":long}),
        );
        let v = a.view();
        assert_eq!(v.requests[0].arg.chars().count(), MAX_PENDING_FIELD);
        assert!(v.requests[0].cut);
        assert_eq!(
            v.requests[1].questions[0].question.chars().count(),
            MAX_PENDING_FIELD
        );
        assert!(v.requests[1].cut);
        assert!(v.requests[2].cut);
        assert_eq!(v.requests[2].arg, "ls");
    }

    #[test]
    fn a_plan_to_approve_notifies_in_the_apps_words() {
        let mut a = rt();
        let fx = can_use_tool(
            &mut a,
            "ExitPlanMode",
            json!({"plan":"# Découper le module\n\n1. Extraire le parseur"}),
        );
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(
                NotifyKind::Question,
                "Approuver le plan : Découper le module ?"
            ))
        );
    }

    #[test]
    fn the_end_of_a_turn_notifies_with_the_first_line_of_the_final_reply() {
        let mut a = rt();
        a.push_user("u1", "Corrige", 0, &[], &mut Effects::default());
        // What was said on the way is no final reply: only what follows the last tool is.
        says(&mut a, "m1", "Je regarde le filtre.");
        runs_tool(&mut a, "m2");
        says(&mut a, "m3", "## Bilan\n\nLe **filtre** accepte les PDF.");
        let fx = ends(&mut a, Some("## Bilan\n\nLe filtre accepte les PDF."));
        assert_eq!(fx.notify, Some(AgentAlert::new(NotifyKind::Done, "Bilan")));
    }

    #[test]
    fn a_turn_that_ends_on_a_tool_tells_claude_codes_own_result_else_it_is_just_done() {
        let mut a = rt();
        a.push_user("u1", "Lance", 0, &[], &mut Effects::default());
        says(&mut a, "m1", "Je lance les tests.");
        runs_tool(&mut a, "m2");
        let fx = ends(&mut a, Some("Les tests passent.\nDétail"));
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(NotifyKind::Done, "Les tests passent."))
        );
        // Another turn, wordless: nothing of the previous one is told again.
        a.push_user("u2", "Encore", 0, &[], &mut Effects::default());
        let fx = ends(&mut a, None);
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(NotifyKind::Done, "Tâche terminée"))
        );
    }

    #[test]
    fn a_failed_turn_notifies_with_its_reason() {
        let mut a = rt();
        a.push_user("u1", "Lance", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":true,"duration_ms":1,
                "result":"API Error: 500\nrequest id 42","modelUsage":{}}),
            &mut fx,
        );
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(
                NotifyKind::Error,
                "Erreur : API Error: 500"
            ))
        );
        // Without words of its own, the kind of result it is.
        a.push_user("u2", "Encore", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"error_during_execution","is_error":true,
                "duration_ms":1,"modelUsage":{}}),
            &mut fx,
        );
        assert_eq!(
            fx.notify.map(|n| n.body),
            Some("Erreur : error_during_execution".to_string())
        );
    }

    #[test]
    fn a_process_that_dies_mid_turn_notifies_with_its_exit_code() {
        let mut a = rt();
        a.push_user("u1", "Lance", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.on_exit(a.gen, Some(3), "", &mut fx);
        assert_eq!(
            fx.notify,
            Some(AgentAlert::new(
                NotifyKind::Error,
                "Erreur : Claude Code s'est arrêté (code 3)"
            ))
        );
    }

    #[test]
    fn tool_result_counts_patch_and_touches_file() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"tu9","name":"Edit","input":{"file_path":"C:/p/src/a.ts"}}]},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu9","content":"ok"}]},
            "tool_use_result":{"filePath":"C:/p/src/a.ts","structuredPatch":[{"oldStart":1,"newStart":1,"lines":[" a","-b","+c","+d"]}]}}), &mut fx);
        let item = a.conv.get("tu9").unwrap();
        assert_eq!(item["status"], "ok");
        assert_eq!(item["result"]["add"], 2);
        assert_eq!(item["result"]["del"], 1);
        assert_eq!(a.meta.touched_files, vec!["src/a.ts".to_string()]);
    }

    #[test]
    fn follows_the_permission_mode_reported_by_claude() {
        let mut a = rt();
        a.meta.mode = "plan".into();
        let mut fx = Effects::default();
        // After an approved plan Claude leaves plan mode for the default (ask) mode.
        a.handle_frame(
            &json!({"type":"system","subtype":"status","status":null,"permissionMode":"default"}),
            &mut fx,
        );
        assert_eq!(a.meta.mode, "default");
        assert!(fx.agent_changed);
    }

    #[test]
    fn usage_counters_that_go_backwards_never_underflow() {
        let mut a = rt();
        let result = |input: u64, output: u64, cost: f64| {
            json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,
                "modelUsage":{"m":{"inputTokens":input,"outputTokens":output,"cacheReadInputTokens":0,"cacheCreationInputTokens":0,"costUSD":cost}}})
        };
        a.handle_frame(&result(100, 50, 1.0), &mut Effects::default());
        let mut fx = Effects::default();
        a.handle_frame(&result(90, 60, 1.2), &mut fx);
        assert_eq!(fx.turns[0].input, 0);
        assert_eq!(fx.turns[0].output, 10);
    }

    fn message_start(
        a: &mut AgentRt,
        id: &str,
        parent: Option<&str>,
        usage: Value,
        fx: &mut Effects,
    ) {
        a.handle_frame(
            &json!({"type":"stream_event","parent_tool_use_id":parent,
                "event":{"type":"message_start","message":{"id":id,"model":"claude-haiku-4-5-20251001","usage":usage}}}),
            fx,
        );
    }

    fn message_delta(a: &mut AgentRt, parent: Option<&str>, output: u64, fx: &mut Effects) {
        a.handle_frame(
            &json!({"type":"stream_event","parent_tool_use_id":parent,
                "event":{"type":"message_delta","usage":{"output_tokens":output}}}),
            fx,
        );
    }

    #[test]
    fn tokens_and_an_estimated_cost_add_up_while_the_turn_runs() {
        let mut a = rt();
        let mut fx = Effects::default();
        let input = json!({"input_tokens":10,"cache_read_input_tokens":17513,"cache_creation_input_tokens":10045,
            "cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":10045},"output_tokens":1});
        message_start(&mut a, "m1", None, input, &mut fx);
        assert_eq!(a.view().live_tokens, 10 + 17513 + 10045 + 1);
        let mut fx = Effects::default();
        message_delta(&mut a, None, 257, &mut fx);
        assert!(fx.agent_changed);
        assert_eq!(a.view().live_tokens, 10 + 17513 + 10045 + 257);
        // A second API call of the same turn, and one made by a subagent.
        let more = json!({"input_tokens":8,"cache_read_input_tokens":27558,"cache_creation_input_tokens":489,
            "cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":489},"output_tokens":1});
        message_start(&mut a, "m2", None, more, &mut fx);
        message_delta(&mut a, None, 65, &mut fx);
        message_start(
            &mut a,
            "s1",
            Some("tu1"),
            json!({"input_tokens":100,"output_tokens":1}),
            &mut fx,
        );
        message_delta(&mut a, Some("tu1"), 20, &mut fx);
        let v = a.view();
        assert_eq!(v.live_tokens, 18 + 322 + 45071 + 10534 + 120);
        // Measured with the real CLI for the first two messages, plus 100 input and 20 output tokens.
        assert!(
            (v.live_cost - (0.0272031 + 100e-6 + 20.0 * 5e-6)).abs() < 1e-9,
            "{}",
            v.live_cost
        );
        assert_eq!(a.meta.tokens, 0);
    }

    #[test]
    fn the_exact_cost_replaces_the_estimate_when_the_turn_ends() {
        let mut a = rt();
        let mut fx = Effects::default();
        message_start(
            &mut a,
            "m1",
            None,
            json!({"input_tokens":1000,"output_tokens":1}),
            &mut fx,
        );
        message_delta(&mut a, None, 50, &mut fx);
        assert!(a.view().live_cost > 0.0);
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,
                "modelUsage":{"claude-haiku-4-5-20251001":{"inputTokens":1000,"outputTokens":50,"cacheReadInputTokens":0,"cacheCreationInputTokens":0,"costUSD":0.00125}}}),
            &mut fx,
        );
        let v = a.view();
        assert_eq!((v.live_tokens, v.live_cost), (0, 0.0));
        assert_eq!(a.meta.tokens, 1050);
        assert!((a.meta.cost - 0.00125).abs() < 1e-12);
    }

    #[test]
    fn a_message_sent_from_claude_ai_shows_in_the_conversation() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":"Ajoute les tests"},"parent_tool_use_id":null,
                "uuid":"r1","isReplay":true,"origin":{"kind":"human"}}),
            &mut fx,
        );
        let item = a.conv.get("r1").expect("remote message listed");
        assert_eq!(item["kind"], "user");
        assert_eq!(item["text"], "Ajoute les tests");
        assert_eq!(item["origin"], "remote");
        assert_eq!(a.meta.status, AgentStatus::Running);
        assert_eq!(a.meta.prompts, 1);
        assert!(fx.agent_changed);
    }

    #[test]
    fn a_file_sent_from_claude_ai_is_listed_with_its_message() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":[
                {"type":"document","title":"rapport.pdf","source":{"type":"base64","media_type":"application/pdf","data":"JVBE"}},
                {"type":"document","source":{"type":"text","media_type":"text/plain","data":"x"}},
                {"type":"image","source":{"type":"base64","media_type":"image/png","data":"iVBO"}}
            ]},"parent_tool_use_id":null,"uuid":"r2","isReplay":true}),
            &mut fx,
        );
        let item = a
            .conv
            .get("r2")
            .expect("a message made only of files is listed");
        assert_eq!(item["text"], "");
        assert_eq!(item["images"], 1);
        assert_eq!(item["files"], json!(["rapport.pdf", "document"]));
    }

    #[test]
    fn the_names_of_the_attached_files_are_kept_with_the_message() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("u1", "Lis ça", 1, &["rapport.pdf".into()], &mut fx);
        let item = a.conv.get("u1").unwrap();
        assert_eq!(item["images"], 1);
        assert_eq!(item["files"], json!(["rapport.pdf"]));
        a.push_user("u2", "Et ça", 0, &[], &mut fx);
        assert!(a.conv.get("u2").unwrap().get("files").is_none());
    }

    #[test]
    fn the_echo_of_a_message_sent_from_the_app_is_not_shown_twice() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("u1", "Bonjour", 0, &[], &mut fx);
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":"Bonjour"},"parent_tool_use_id":null,"uuid":"u1","isReplay":true}),
            &mut fx,
        );
        let users = a.conv.ids_where(|v| v["kind"] == "user");
        assert_eq!(users, vec!["u1".to_string()]);
        assert_eq!(a.meta.prompts, 1);
    }

    #[test]
    fn what_claude_code_passes_on_by_itself_is_an_event_not_a_message_from_the_user() {
        let mut a = rt();
        let mut fx = Effects::default();
        // Queued during a turn, then passed to Claude: a background task that ended…
        let notification = "<task-notification>\n<task-id>b1</task-id>\n<status>completed</status>\n<summary>Background command \"ping\" completed (exit code 0)</summary>\n</task-notification>";
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":notification},"parent_tool_use_id":null,
                    "uuid":"n1","isReplay":true,"origin":{"kind":"task-notification","producer":"session-task"}}),
            &mut fx,
        );
        // …and the report of a subagent.
        let report = "<agent-message from=\"a42\">\n[Subagent hand-back] The report follows:\n  **Fini**\n</agent-message>";
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":[{"type":"text","text":report}]},"parent_tool_use_id":null,
                    "uuid":"n2","isReplay":true,"origin":{"kind":"peer","from":"a42","senderTaskId":"a42"}}),
            &mut fx,
        );
        let items = a.conv.items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["kind"], "event");
        assert_eq!(items[0]["source"], "task");
        assert_eq!(items[0]["text"], notification);
        assert_eq!(items[1]["kind"], "event");
        assert_eq!(items[1]["source"], "agent");
        assert_eq!(items[1]["from"], "a42");
        assert_eq!(items[1]["text"], report);
        // Not a prompt of the user's, nor a turn started for them.
        assert_eq!(a.meta.prompts, 0);
        assert!(!a.meta.status.is_active());
        // Replayed again (resume): kept once.
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":notification},"uuid":"n1","isReplay":true,
                    "origin":{"kind":"task-notification"}}),
            &mut fx,
        );
        assert_eq!(a.conv.items().len(), 2);
    }

    #[test]
    fn a_turn_stopped_by_the_usage_limit_tells_when_it_resets() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1790800000,"rateLimitType":"five_hour"}}),
            &mut fx,
        );
        // Claude Code's own message, typed as a rate limit.
        a.handle_frame(
            &json!({"type":"assistant","error":"rate_limit","parent_tool_use_id":null,
                    "message":{"id":"m1","role":"assistant","content":[{"type":"text","text":"You've hit your limit · resets 3pm"}]}}),
            &mut fx,
        );
        let mut end = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":true,"result":"You've hit your limit · resets 3pm"}),
            &mut end,
        );
        assert_eq!(end.limited, Some(Some(1_790_800_000_000)));
        // The next turn is a normal one.
        let mut next = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"result":"ok"}),
            &mut next,
        );
        assert_eq!(next.limited, None);
    }

    #[test]
    fn only_a_failed_turn_of_the_conversation_s_own_is_stopped_by_the_limit() {
        let limit = |parent: Value| {
            json!({"type":"assistant","error":"rate_limit","parent_tool_use_id":parent,
                   "message":{"id":"m1","role":"assistant","content":[{"type":"text","text":"You've hit your limit"}]}})
        };
        let end = |is_error: bool| json!({"type":"result","subtype":"success","is_error":is_error,"result":"x"});
        // A subagent met it: the conversation's turn goes on.
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&limit(json!("toolu_sub")), &mut fx);
        let mut done = Effects::default();
        a.handle_frame(&end(true), &mut done);
        assert_eq!(done.limited, None);
        // The turn succeeded all the same (a fallback model).
        a.handle_frame(&limit(Value::Null), &mut fx);
        let mut done = Effects::default();
        a.handle_frame(&end(false), &mut done);
        assert_eq!(done.limited, None);
    }

    #[test]
    fn a_turn_that_goes_through_drops_an_old_planned_resume() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.meta.resume_at = Some(1_790_800_030_000);
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"result":"ok"}),
            &mut fx,
        );
        assert_eq!(a.meta.resume_at, None);
        assert!(fx.save && fx.agent_changed);
    }

    #[test]
    fn a_message_of_the_user_s_cancels_the_planned_resume() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.meta.resume_at = Some(1_790_800_030_000);
        a.push_user("u1", "je reprends la main", 0, &[], &mut fx);
        assert_eq!(a.meta.resume_at, None);
        assert!(fx.save);
    }

    #[test]
    fn a_background_task_that_ends_while_the_agent_waits_is_shown_once() {
        let mut a = rt();
        let mut fx = Effects::default();
        let system = |a: &mut AgentRt, f: Value, fx: &mut Effects| a.handle_frame(&f, fx);
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"bg1","tool_use_id":"t1","description":"npm test","is_backgrounded":true}),
            &mut fx,
        );
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"fg1","tool_use_id":"t2","description":"ls","is_backgrounded":false}),
            &mut fx,
        );
        // A command in the foreground: its tool row already shows it ended.
        system(
            &mut a,
            json!({"type":"system","subtype":"task_notification","task_id":"fg1","tool_use_id":"t2","status":"completed","summary":"ls","uuid":"s0"}),
            &mut fx,
        );
        // Idle, Claude Code only tells this (then starts a turn without a user message).
        system(
            &mut a,
            json!({"type":"system","subtype":"task_notification","task_id":"bg1","tool_use_id":"t1","status":"completed",
                    "summary":"Background command \"npm test\" completed (exit code 0)","uuid":"s1"}),
            &mut fx,
        );
        let events = a.conv.ids_where(|v| v["kind"] == "event");
        assert_eq!(events, vec!["s1".to_string()]);
        let text = a.conv.get("s1").unwrap()["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(text.contains("<status>completed</status>"), "{text}");
        assert!(
            text.contains(
                "<summary>Background command \"npm test\" completed (exit code 0)</summary>"
            ),
            "{text}"
        );
        // Mid-turn, the same notification is also replayed to Claude: not shown twice.
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":"<task-notification>\n<task-id>bg1</task-id>\n<status>completed</status>\n</task-notification>"},
                    "uuid":"r1","isReplay":true,"origin":{"kind":"task-notification"}}),
            &mut fx,
        );
        assert_eq!(a.conv.ids_where(|v| v["kind"] == "event").len(), 1);
    }

    #[test]
    fn only_a_human_s_message_is_the_user_s() {
        let mut a = rt();
        let mut fx = Effects::default();
        let replay = |uuid: &str, content: &str, extra: Value| {
            let mut f = json!({"type":"user","message":{"role":"user","content":content},"parent_tool_use_id":null,
                               "uuid":uuid,"isReplay":true});
            f.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            f
        };
        for f in [
            // From claude.ai (Remote Control), or with no origin at all: the user's.
            replay(
                "h1",
                "depuis le téléphone",
                json!({"origin":{"kind":"human"}}),
            ),
            replay("n1", "sans origine", json!({})),
            // Queued by Claude Code itself: an MCP channel, a meta message…
            replay(
                "c1",
                "alerte du canal",
                json!({"origin":{"kind":"channel","server":"slack"}}),
            ),
            replay("s1", "continue", json!({"isSynthetic":true})),
            // …and nothing to show.
            replay("e1", "  ", json!({"origin":{"kind":"peer"}})),
        ] {
            a.handle_frame(&f, &mut fx);
        }
        let kinds: Vec<(String, String, String)> = a
            .conv
            .items()
            .iter()
            .map(|i| {
                let s = |k: &str| i[k].as_str().unwrap_or_default().to_string();
                (
                    s("id"),
                    s("kind"),
                    format!("{}{}", s("source"), s("origin")),
                )
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("h1".into(), "user".into(), "remote".into()),
                ("n1".into(), "user".into(), "remote".into()),
                ("c1".into(), "event".into(), "channel".into()),
                ("s1".into(), "event".into(), "synthetic".into()),
            ]
        );
        assert_eq!(a.meta.prompts, 2);
    }

    #[test]
    fn the_context_window_follows_a_switch_of_model() {
        let mut a = rt();
        let mut fx = Effects::default();
        let start = |a: &mut AgentRt, id: &str, model: &str, fx: &mut Effects| {
            a.handle_frame(
                &json!({"type":"stream_event","parent_tool_use_id":null,
                        "event":{"type":"message_start","message":{"id":id,"model":model,"usage":{"input_tokens":10}}}}),
                fx,
            );
        };
        let usage = |read: u64, window: u64| json!({"inputTokens":10,"outputTokens":10,"cacheReadInputTokens":read,"cacheCreationInputTokens":0,"costUSD":0.1,"contextWindow":window});
        start(&mut a, "m1", "claude-sonnet-5", &mut fx);
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,
                    "modelUsage":{"claude-sonnet-5":usage(900_000, 200_000)}}),
            &mut fx,
        );
        assert_eq!(a.view().context_window, 200_000);
        // Switched to Opus mid-process: Sonnet's figures, cumulative, still read the most.
        start(&mut a, "m2", "claude-opus-5-5", &mut fx);
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,
                    "modelUsage":{"claude-sonnet-5":usage(900_000, 200_000),"claude-opus-5-5":usage(50_000, 1_000_000)}}),
            &mut fx,
        );
        assert_eq!(a.view().context_window, 1_000_000);
    }

    #[test]
    fn knows_the_context_window_of_the_conversation_s_model() {
        let mut a = rt();
        let mut fx = Effects::default();
        assert_eq!(a.view().context_window, 0);
        // The conversation's model reads the most; a subagent's may have a smaller window.
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"modelUsage":{
                "claude-opus-5-5":{"inputTokens":120,"outputTokens":900,"cacheReadInputTokens":80000,"cacheCreationInputTokens":4000,"costUSD":0.5,"contextWindow":1000000},
                "claude-haiku-4-5":{"inputTokens":3000,"outputTokens":200,"cacheReadInputTokens":0,"cacheCreationInputTokens":0,"costUSD":0.01,"contextWindow":200000}}}),
            &mut fx,
        );
        assert_eq!(a.view().context_window, 1_000_000);
    }

    #[test]
    fn follows_the_remote_control_link_state() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"system","subtype":"bridge_state","state":"connected"}),
            &mut fx,
        );
        assert_eq!(a.view().remote_state.as_deref(), Some("connected"));
        assert!(fx.agent_changed);
        a.on_exit(a.gen, Some(0), "", &mut fx);
        assert_eq!(a.view().remote_state, None);
    }

    #[test]
    fn a_process_that_dies_mid_turn_drops_its_estimate() {
        let mut a = rt();
        let mut fx = Effects::default();
        message_start(
            &mut a,
            "m1",
            None,
            json!({"input_tokens":1000,"output_tokens":1}),
            &mut fx,
        );
        a.on_exit(a.gen, Some(1), "boom", &mut fx);
        let v = a.view();
        assert_eq!((v.live_tokens, v.live_cost), (0, 0.0));
    }

    fn start_streaming(a: &mut AgentRt, fx: &mut Effects) {
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m9","usage":{}}},"parent_tool_use_id":null}), fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}},"parent_tool_use_id":null}), fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Je comm"}},"parent_tool_use_id":null}), fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"tu7","name":"Bash"}},"parent_tool_use_id":null}), fx);
    }

    #[test]
    fn an_interrupted_turn_closes_open_blocks_and_tools() {
        let mut a = rt();
        let mut fx = Effects::default();
        start_streaming(&mut a, &mut fx);
        a.interrupted = true;
        a.handle_frame(&json!({"type":"result","subtype":"error_during_execution","is_error":true,"duration_ms":5}), &mut fx);
        let text = a.conv.get("m9:0").unwrap().clone();
        assert_eq!(text["streaming"], false);
        assert_eq!(text["text"], "Je comm");
        assert_eq!(a.conv.get("tu7").unwrap()["status"], "interrupted");
    }

    #[test]
    fn a_process_exit_closes_open_blocks_and_persists_their_text() {
        let dir = std::env::temp_dir().join(format!("ccm-agent-tests-{}", std::process::id()));
        let mut a = AgentRt::new(
            AgentMeta {
                id: new_id(),
                cwd: "C:/p".into(),
                ..Default::default()
            },
            &dir,
        );
        let mut fx = Effects::default();
        start_streaming(&mut a, &mut fx);
        a.on_exit(a.gen, Some(1), "", &mut fx);
        assert_eq!(a.conv.get("m9:0").unwrap()["streaming"], false);
        assert_eq!(a.conv.get("tu7").unwrap()["status"], "interrupted");
        // Reloading the log (e.g. after a restart) shows the partial text, not an empty block.
        let mut reloaded = Conv::new(&dir, &a.meta.id);
        let items = reloaded.items();
        let text = items.iter().find(|i| i["id"] == "m9:0").unwrap();
        assert_eq!(text["text"], "Je comm");
        assert_eq!(text["streaming"], false);
    }

    #[test]
    fn ansi_and_tags_are_stripped() {
        assert_eq!(strip_ansi("\u{1b}[31mrouge\u{1b}[0m"), "rouge");
        assert_eq!(
            strip_tags("<local-command-stdout>ok</local-command-stdout>"),
            "ok"
        );
    }

    #[test]
    fn the_activity_follows_the_main_threads_tools_and_clears_at_the_end_of_the_turn() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m1","usage":{}}},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}},"parent_tool_use_id":null}), &mut fx);
        assert_eq!(a.view().activity.as_deref(), Some("Réfléchit"));
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"C:/p/src/db.ts"}}]},"parent_tool_use_id":null}), &mut fx);
        assert_eq!(a.view().activity.as_deref(), Some("Lit src/db.ts"));
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m3","content":[{"type":"tool_use","id":"t2","name":"Bash","input":{"command":"npm test -- --run\nnpm run lint"}}]},"parent_tool_use_id":null}), &mut fx);
        assert_eq!(
            a.view().activity.as_deref(),
            Some("Lance npm test -- --run")
        );
        // A subagent's tools are its own business.
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m4","content":[{"type":"tool_use","id":"t3","name":"Write","input":{"file_path":"C:/p/x.ts"}}]},"parent_tool_use_id":"t9"}), &mut fx);
        assert_eq!(
            a.view().activity.as_deref(),
            Some("Lance npm test -- --run")
        );
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1}),
            &mut fx,
        );
        assert_eq!(a.view().activity, None);
    }

    #[test]
    fn tool_activities_name_what_the_agent_does() {
        let cwd = "C:/p";
        let act = |name: &str, input: Value| tool_activity(name, &input, cwd);
        assert_eq!(
            act("Grep", json!({"pattern":"TODO"})).as_deref(),
            Some("Cherche TODO")
        );
        assert_eq!(
            act("Glob", json!({"pattern":"**/*.ts"})).as_deref(),
            Some("Cherche **/*.ts")
        );
        assert_eq!(
            act("Edit", json!({"file_path":"C:/p/src/a.ts"})).as_deref(),
            Some("Modifie src/a.ts")
        );
        assert_eq!(
            act("MultiEdit", json!({"file_path":"C:/p/src/a.ts"})).as_deref(),
            Some("Modifie src/a.ts")
        );
        assert_eq!(
            act("Write", json!({"file_path":"C:/p/b.ts"})).as_deref(),
            Some("Écrit b.ts")
        );
        assert_eq!(act("Task", json!({})).as_deref(), Some("Délègue"));
        assert_eq!(act("Read", Value::Null).as_deref(), Some("Lit"));
        assert_eq!(act("WebSearch", json!({})), None);
    }

    #[test]
    fn an_action_refused_without_a_word_is_told_to_claude_in_its_language() {
        use crate::i18n::Lang::{En, Fr};
        assert_eq!(refused_action(Fr), "L'utilisateur a refusé cette action.");
        assert_eq!(refused_action(En), "The user refused this action.");
    }

    #[test]
    fn what_the_agent_does_and_what_its_conversation_is_told_read_in_english() {
        use crate::i18n::Lang::{En, Fr};
        let act = |name: &str, input: Value| tool_activity_in(En, name, &input, "C:/p");
        assert_eq!(
            act("Read", json!({"file_path":"C:/p/src/db.ts"})).as_deref(),
            Some("Reading src/db.ts")
        );
        assert_eq!(
            act("Bash", json!({"command":"npm test"})).as_deref(),
            Some("Running npm test")
        );
        // A bare verb is the start of the same verb with what it acts on, as in French.
        assert_eq!(act("Read", Value::Null).as_deref(), Some("Reading"));
        assert_eq!(act("Task", json!({})).as_deref(), Some("Delegating"));
        assert_eq!(
            [block_activity(En, "text"), block_activity(En, "thinking")],
            [Some("Replying".to_string()), Some("Thinking".to_string())]
        );
        assert_eq!(
            exit_notices(En),
            [
                "Claude session not found: a new session will start with your next message.",
                "Copy point not found in the original’s session: Claude picks up its whole conversation, up to now.",
            ]
        );
        assert_eq!(stopped(En, "3"), "Claude Code stopped (code 3)");
        assert_eq!(stopped(Fr, "3"), "Claude Code s'est arrêté (code 3)");
        // The count of a to-do list, as the conversation's card writes it (`conv.tools.tasks`).
        let todos = |n: usize, lang| {
            tool_arg_in(
                lang,
                "TodoWrite",
                &json!({ "todos": vec![json!({}); n] }),
                "C:/p",
            )
        };
        assert_eq!([todos(1, En), todos(2, En)], ["1 task", "2 tasks"]);
        assert_eq!([todos(1, Fr), todos(2, Fr)], ["1 tâche", "2 tâches"]);
    }

    #[test]
    fn the_end_of_a_turn_tells_how_it_ended_with_the_assistants_text() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("u1", "Go", 0, &[], &mut fx);
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"Fait."}]},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m2","content":[{"type":"text","text":"```escouade\n{}\n```"}]},"parent_tool_use_id":null}), &mut fx);
        a.handle_frame(&json!({"type":"assistant","message":{"id":"m3","content":[{"type":"text","text":"sous-agent"}]},"parent_tool_use_id":"t1"}), &mut fx);
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,"result":"ignoré"}), &mut fx);
        assert_eq!(
            fx.turn_end,
            Some(TurnEnd::Finished("Fait.\n\n```escouade\n{}\n```".into()))
        );
        // Without text of its own, the result's.
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,"result":"Résumé"}), &mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Finished("Résumé".into())));
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"error_during_execution","is_error":true,"duration_ms":1,"result":"Boom"}), &mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Error("Boom".into())));
        a.interrupted = true;
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"error_during_execution","is_error":true,"duration_ms":1}), &mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Interrupted));
        a.handle_frame(&json!({"type":"assistant","error":"rate_limit","message":{"id":"m9","content":[{"type":"text","text":"limit"}]},"parent_tool_use_id":null}), &mut Effects::default());
        let mut fx = Effects::default();
        a.handle_frame(&json!({"type":"result","subtype":"success","is_error":true,"duration_ms":1,"result":"limit"}), &mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Limited));
    }

    #[test]
    fn a_process_dying_during_a_turn_ends_it_in_error() {
        let mut a = rt();
        a.saw_init = true;
        a.push_user("u1", "Bonjour", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.on_exit(a.gen, Some(3), "", &mut fx);
        assert_eq!(
            fx.turn_end,
            Some(TurnEnd::Error("Claude Code s'est arrêté (code 3)".into()))
        );
        // Idle, its exit ends no turn.
        let mut fx = Effects::default();
        a.on_exit(a.gen, Some(0), "", &mut fx);
        assert_eq!(fx.turn_end, None);
    }

    #[test]
    fn a_copy_forks_no_more_once_it_has_a_session_or_once_the_original_is_gone() {
        let forks = |a: &AgentRt| {
            (
                a.meta.session_id.clone(),
                a.meta.fork_of.clone(),
                a.meta.fork_at.clone(),
            )
        };
        // Its first turn gives it a session of its own.
        let mut a = rt();
        a.meta.fork_of = Some("s1".into());
        a.meta.fork_at = Some("e1".into());
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({ "type": "system", "subtype": "init", "session_id": "s2" }),
            &mut fx,
        );
        assert_eq!(forks(&a), (Some("s2".into()), None, None));
        assert!(fx.save);
        // The original's session is gone: a new session, as for a lost one of its own.
        let mut a = rt();
        a.meta.fork_of = Some("missing-1".into());
        a.meta.fork_at = Some("e1".into());
        a.on_exit(
            a.gen,
            Some(1),
            "No conversation found with session ID: missing-1",
            &mut Effects::default(),
        );
        assert_eq!(forks(&a), (None, None, None));
    }

    #[test]
    fn a_copy_whose_fork_point_is_gone_forks_the_whole_session() {
        let mut a = rt();
        a.meta.fork_of = Some("s1".into());
        a.meta.fork_at = Some("missing-e1".into());
        let mut fx = Effects::default();
        a.on_exit(
            a.gen,
            Some(1),
            "No message found with message.uuid of: missing-e1",
            &mut fx,
        );
        // Still a copy of the original's session, from its end.
        assert_eq!(
            (a.meta.fork_of.as_deref(), a.meta.fork_at.as_deref()),
            (Some("s1"), None)
        );
        assert_eq!(a.meta.status, AgentStatus::Done);
        assert_eq!(fx.notify, None);
        let notices: Vec<Value> = a
            .conv
            .items()
            .into_iter()
            .filter(|i| i["kind"] == "notice")
            .collect();
        assert_eq!(notices.len(), 1, "{notices:?}");
        assert_eq!(notices[0]["level"], "warn");
        assert_eq!(
            notices[0]["text"],
            "Point de copie introuvable dans la session de l'original : Claude reprend toute sa conversation, jusqu'à maintenant."
        );
    }

    #[test]
    fn the_last_entry_of_its_session_is_kept_for_a_copy_to_fork_at() {
        let mut a = rt();
        let mut fx = Effects::default();
        let frame = |uuid: &str, extra: Value| {
            let mut f = json!({ "type": "assistant", "uuid": uuid, "parent_tool_use_id": null,
                "message": { "id": format!("m-{uuid}"), "role": "assistant", "content": [{ "type": "text", "text": "ok" }] } });
            if let (Some(f), Some(extra)) = (f.as_object_mut(), extra.as_object()) {
                f.extend(extra.clone());
            }
            f
        };
        a.handle_frame(
            &json!({ "type": "system", "subtype": "init", "session_id": "s1" }),
            &mut fx,
        );
        a.handle_frame(&frame("e1", json!({})), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e1"));
        // Not entries of its main chain: a subagent's, a message Claude Code makes up, an error.
        a.handle_frame(
            &frame("e2", json!({ "parent_tool_use_id": "tu1" })),
            &mut fx,
        );
        a.handle_frame(&frame("e3", json!({ "is_meta": true })), &mut fx);
        a.handle_frame(&frame("e4", json!({ "error": "rate_limit" })), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e1"));
        // Same session, next turn: its later entry.
        a.handle_frame(
            &json!({ "type": "system", "subtype": "init", "session_id": "s1" }),
            &mut fx,
        );
        a.handle_frame(&frame("e5", json!({})), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e5"));
        // A new session (the context cleared): none of its entries is the old one's.
        a.handle_frame(
            &json!({ "type": "system", "subtype": "init", "session_id": "s2" }),
            &mut fx,
        );
        assert_eq!(a.meta.last_entry, None);
    }

    #[test]
    fn a_tool_result_that_ends_a_turn_is_its_last_entry() {
        let mut a = rt();
        let mut fx = Effects::default();
        let result = |uuid: &str, extra: Value| {
            let mut f = json!({ "type": "user", "uuid": uuid, "parent_tool_use_id": null,
                "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "tu1", "content": "fait" }] } });
            if let (Some(f), Some(extra)) = (f.as_object_mut(), extra.as_object()) {
                f.extend(extra.clone());
            }
            f
        };
        a.handle_frame(
            &json!({ "type": "assistant", "uuid": "e1", "parent_tool_use_id": null,
                "message": { "id": "m1", "role": "assistant", "content": [{ "type": "tool_use", "id": "tu1", "name": "Bash", "input": {} }] } }),
            &mut fx,
        );
        // Interrupted, or an API error, before any reply: the result of its tool ends the turn.
        a.handle_frame(&result("e2", json!({})), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e2"));
        // Not entries of its main chain: a subagent's result, a message sent again from claude.ai,
        // a message with no tool result (a local command's output).
        a.handle_frame(
            &result("e3", json!({ "parent_tool_use_id": "tu0" })),
            &mut fx,
        );
        a.handle_frame(&result("e4", json!({ "isReplay": true })), &mut fx);
        a.handle_frame(
            &json!({ "type": "user", "uuid": "e5", "parent_tool_use_id": null,
                "message": { "role": "user", "content": [{ "type": "text", "text": "<local-command-stdout>ok</local-command-stdout>" }] } }),
            &mut fx,
        );
        assert_eq!(a.meta.last_entry.as_deref(), Some("e2"));
    }

    #[test]
    fn a_compaction_leaves_no_entry_to_fork_at_until_the_next_one() {
        let mut a = rt();
        let mut fx = Effects::default();
        let reply = |uuid: &str| {
            json!({ "type": "assistant", "uuid": uuid, "parent_tool_use_id": null,
                "message": { "id": format!("m-{uuid}"), "role": "assistant", "content": [{ "type": "text", "text": "ok" }] } })
        };
        a.handle_frame(&reply("e1"), &mut fx);
        // The chain starts again at the boundary: what came before is no longer in it.
        a.handle_frame(
            &json!({ "type": "system", "subtype": "compact_boundary", "uuid": "b1" }),
            &mut fx,
        );
        assert_eq!(a.meta.last_entry, None);
        a.handle_frame(&reply("e2"), &mut fx);
        assert_eq!(a.meta.last_entry.as_deref(), Some("e2"));
    }

    #[test]
    fn a_new_last_entry_is_saved_without_waiting_for_the_end_of_the_turn() {
        let mut a = rt();
        let reply = |uuid: &str| {
            json!({ "type": "assistant", "uuid": uuid, "parent_tool_use_id": null,
                "message": { "id": format!("m-{uuid}"), "role": "assistant", "content": [{ "type": "text", "text": "ok" }] } })
        };
        // What a frame asks to save.
        let saves = |a: &mut AgentRt, frame: Value| {
            let mut fx = Effects::default();
            a.handle_frame(&frame, &mut fx);
            fx.save
        };
        // The app stopping for good mid-turn: a copy still forks at its latest entry.
        assert!(saves(&mut a, reply("e1")));
        // The same entry told again: nothing new to save.
        assert!(!saves(&mut a, reply("e1")));
        // A tool's result in its main chain.
        assert!(saves(
            &mut a,
            json!({ "type": "user", "uuid": "e2", "parent_tool_use_id": null,
                "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "tu1", "content": "fait" }] } })
        ));
        assert_eq!(a.meta.last_entry.as_deref(), Some("e2"));
        // A compaction, which leaves none.
        assert!(saves(
            &mut a,
            json!({ "type": "system", "subtype": "compact_boundary", "uuid": "b1" })
        ));
        assert_eq!(a.meta.last_entry, None);
    }

    #[test]
    fn a_session_that_cannot_be_resumed_still_ends_the_turn_it_was_to_carry() {
        let lost =
            "Session Claude introuvable : une nouvelle session sera démarrée au prochain message.";
        let mut a = rt();
        a.meta.session_id = Some("missing-1".into());
        a.push_user("u1", "Bonjour", 0, &[], &mut Effects::default());
        let mut fx = Effects::default();
        a.on_exit(
            a.gen,
            Some(1),
            "No conversation found with session ID: missing-1",
            &mut fx,
        );
        assert_eq!(a.meta.status, AgentStatus::Done);
        assert_eq!(a.meta.session_id, None);
        assert_eq!(fx.notify, None);
        assert_eq!(fx.turn_end, Some(TurnEnd::Error(lost.into())));
        // Idle, it had no turn to end.
        let mut fx = Effects::default();
        a.on_exit(
            a.gen,
            Some(1),
            "No conversation found with session ID: missing-1",
            &mut fx,
        );
        assert_eq!(a.meta.status, AgentStatus::Done);
        assert_eq!(fx.turn_end, None);
    }

    #[test]
    fn a_failed_turn_always_says_what_went_wrong() {
        let mut a = rt();
        let mut end = |frame: Value| {
            let mut fx = Effects::default();
            a.handle_frame(&frame, &mut fx);
            fx.turn_end
        };
        let said = |text: &str| Some(TurnEnd::Error(text.into()));
        // The frame's own words first.
        assert_eq!(
            end(
                json!({"type":"result","subtype":"error_during_execution","is_error":true,"result":"Boom"})
            ),
            said("Boom")
        );
        assert_eq!(
            end(
                json!({"type":"result","subtype":"error_max_turns","is_error":true,"errors":["a","b"]})
            ),
            said("a\nb")
        );
        // Without any, the kind of result it is.
        assert_eq!(
            end(json!({"type":"result","subtype":"error_during_execution","is_error":true})),
            said("error_during_execution")
        );
        assert_eq!(
            end(
                json!({"type":"result","subtype":"error_max_turns","is_error":true,"result":"  ","errors":[]})
            ),
            said("error_max_turns")
        );
        // A "success" that is an error says nothing by its kind: the generic words.
        assert_eq!(
            end(json!({"type":"result","subtype":"success","is_error":true})),
            said("Claude Code a signalé une erreur")
        );
        assert_eq!(
            end(json!({"type":"result","is_error":true})),
            said("Claude Code a signalé une erreur")
        );
    }

    #[test]
    fn an_activity_gist_is_its_first_real_line_cut_short() {
        let cwd = "C:/p";
        let act = |name: &str, input: Value| tool_activity(name, &input, cwd);
        // A leading blank line does not leave a bare verb.
        assert_eq!(
            act("Bash", json!({"command":"\n\n  npm test\nnpm run lint"})).as_deref(),
            Some("Lance npm test")
        );
        assert_eq!(
            act("Bash", json!({"command":" \n "})).as_deref(),
            Some("Lance")
        );
        assert_eq!(
            act("Grep", json!({"pattern":"\nTODO\nFIXME"})).as_deref(),
            Some("Cherche TODO")
        );
        // Long ones are cut at 60 bytes, on a character boundary.
        let long = "é".repeat(100);
        let cut = format!("{}…", "é".repeat(30));
        assert_eq!(
            act("Grep", json!({"pattern":long})).as_deref(),
            Some(format!("Cherche {cut}").as_str())
        );
        assert_eq!(
            act("Glob", json!({"pattern":long})).as_deref(),
            Some(format!("Cherche {cut}").as_str())
        );
        assert_eq!(
            act("Bash", json!({"command":long})).as_deref(),
            Some(format!("Lance {cut}").as_str())
        );
    }

    #[test]
    fn a_tool_starting_to_stream_does_not_blur_the_activity_of_the_same_tool() {
        let mut a = rt();
        let mut fx = Effects::default();
        let frame = |a: &mut AgentRt, f: Value| a.handle_frame(&f, &mut Effects::default());
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m1","usage":{}}},"parent_tool_use_id":null}),
        );
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"t1","name":"Read"}},"parent_tool_use_id":null}),
        );
        // Nothing more precise is known yet.
        assert_eq!(a.view().activity.as_deref(), Some("Lit"));
        frame(
            &mut a,
            json!({"type":"assistant","message":{"id":"m1","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"C:/p/src/a.ts"}}]},"parent_tool_use_id":null}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Lit src/a.ts"));
        // Another Read starts: the bare verb does not replace the precise one...
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m2","usage":{}}},"parent_tool_use_id":null}),
        );
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"t2","name":"Read"}},"parent_tool_use_id":null}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Lit src/a.ts"));
        // ...but another verb does, and so does the next precise one.
        frame(
            &mut a,
            json!({"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"t3","name":"Grep"}},"parent_tool_use_id":null}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Cherche"));
        frame(
            &mut a,
            json!({"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"t3","name":"Grep","input":{"pattern":"TODO"}}]},"parent_tool_use_id":null}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Cherche TODO"));
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false}),
            &mut fx,
        );
        assert_eq!(a.view().activity, None);
    }

    // ---------- the plan ----------

    use crate::plan::{RunStatus, TaskStatus};

    /// The assistant calls `name` (in the thread of the subagent `parent`, or the main one).
    fn call(a: &mut AgentRt, parent: Option<&str>, id: &str, name: &str, input: Value) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"assistant","message":{"id":format!("m-{id}"),"content":[{"type":"tool_use","id":id,"name":name,"input":input}]},"parent_tool_use_id":parent}),
            &mut fx,
        );
        fx
    }

    /// The result of the call `id`, with the frame's structured `tur`.
    fn returns(a: &mut AgentRt, id: &str, text: &str, tur: Value) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":id,"content":text}]},"tool_use_result":tur,"parent_tool_use_id":null}),
            &mut fx,
        );
        fx
    }

    fn system(a: &mut AgentRt, f: Value) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(&f, &mut fx);
        fx
    }

    fn session(a: &mut AgentRt, id: &str) -> Effects {
        system(a, json!({"type":"system","subtype":"init","session_id":id}))
    }

    /// The turn's result frame.
    fn turn_ends(a: &mut AgentRt) {
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1}),
            &mut Effects::default(),
        );
    }

    /// A task made through `TaskCreate`, with its result.
    fn creates(a: &mut AgentRt, id: &str, subject: &str, number: u32) {
        call(a, None, id, "TaskCreate", json!({ "subject": subject }));
        returns(
            a,
            id,
            &format!("Task #{number} created successfully: {subject}"),
            Value::Null,
        );
    }

    fn plan(a: &AgentRt) -> PlanState {
        a.view().meta.plan.expect("a plan")
    }

    #[test]
    fn the_plan_follows_the_task_tools_and_the_subagents_of_the_stream() {
        let mut a = rt();
        assert!(a.view().meta.plan.is_none());
        creates(&mut a, "c0", "Lire", 1);
        creates(&mut a, "c1", "Écrire", 2);
        call(
            &mut a,
            None,
            "u1",
            "TaskUpdate",
            json!({"taskId":"1","status":"in_progress","activeForm":"Lit"}),
        );
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Lire le code","prompt":"Read task-1-brief.md","model":"haiku","subagent_type":"Explore"}),
        );
        // What the agent does, as before: the subagent's tools are its own business.
        assert_eq!(a.view().activity.as_deref(), Some("Délègue"));
        call(
            &mut a,
            Some("a1"),
            "k1",
            "Read",
            json!({"file_path":"C:/p/src/a.ts"}),
        );
        assert_eq!(a.view().activity.as_deref(), Some("Délègue"));

        let p = plan(&a);
        assert_eq!(
            serde_json::to_value(&p.tasks).unwrap(),
            json!([
                {"id":"1","title":"Lire","status":"inProgress","active":"Lit"},
                {"id":"2","title":"Écrire","status":"pending"},
            ])
        );
        let row = &p.agents[0];
        assert_eq!(
            (row.id.as_str(), row.title.as_str(), row.status, row.tools),
            ("a1", "Lire le code", RunStatus::Running, 1)
        );
        assert_eq!(row.doing.as_deref(), Some("Lit src/a.ts"));
        assert_eq!(row.model.as_deref(), Some("haiku"));
        assert_eq!(row.plan_task.as_deref(), Some("1"));
        assert_eq!(p.launched, 1);

        returns(
            &mut a,
            "a1",
            "Trouvé",
            json!({"status":"completed","agentId":"x","totalToolUseCount":3,"totalDurationMs":1500,"totalTokens":900}),
        );
        let p = plan(&a);
        assert_eq!(p.agents[0].status, RunStatus::Done);
        assert_eq!((p.agents[0].tools, p.agents[0].tokens), (3, Some(900)));
        assert_eq!(p.agents[0].doing, None);
        call(
            &mut a,
            None,
            "u2",
            "TaskUpdate",
            json!({"taskId":"1","status":"completed"}),
        );
        assert_eq!(plan(&a).tasks[0].status, TaskStatus::Done);
    }

    #[test]
    fn a_frame_that_comes_twice_is_one_task_and_one_subagent() {
        let mut a = rt();
        for _ in 0..2 {
            call(&mut a, None, "c0", "TaskCreate", json!({"subject":"Lire"}));
            call(
                &mut a,
                None,
                "a1",
                "Agent",
                json!({"description":"Aide","prompt":""}),
            );
        }
        let p = plan(&a);
        assert_eq!((p.tasks.len(), p.agents.len(), p.launched), (1, 1, 1));
    }

    #[test]
    fn a_detail_of_a_subagent_goes_to_the_window_and_only_a_change_is_saved() {
        let mut a = rt();
        // A subagent starts: the structure changed.
        let fx = call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Aide","prompt":""}),
        );
        assert!(fx.agent_changed && fx.save);
        // What it does: the window hears of it, nothing is saved for it.
        let read = json!({"file_path":"C:/p/src/a.ts"});
        let fx = call(&mut a, Some("a1"), "k1", "Read", read.clone());
        assert!(fx.agent_changed);
        assert!(!fx.save);
        assert_eq!(plan(&a).agents[0].doing.as_deref(), Some("Lit src/a.ts"));
        // The same step by another call: nothing to tell.
        let fx = call(&mut a, Some("a1"), "k2", "Read", read);
        assert!(!fx.agent_changed && !fx.save);
        assert_eq!(plan(&a).agents[0].tools, 2);
        // Another step is news again.
        let fx = call(&mut a, Some("a1"), "k3", "Grep", json!({"pattern":"TODO"}));
        assert!(fx.agent_changed && !fx.save);
        // Its end is saved.
        let fx = returns(
            &mut a,
            "a1",
            "Fini",
            json!({"status":"completed","totalToolUseCount":3}),
        );
        assert!(fx.agent_changed && fx.save);
        // A tool of the main thread that is no business of the plan changes nothing of it.
        let fx = call(&mut a, None, "b1", "Bash", json!({"command":"npm test"}));
        assert!(fx.agent_changed, "the activity changed");
        assert_eq!(plan(&a).agents.len(), 1);
    }

    #[test]
    fn the_notification_of_a_background_subagent_ends_its_row_and_still_says_it_in_the_conversation(
    ) {
        let mut a = rt();
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Tests","prompt":"","run_in_background":true}),
        );
        returns(
            &mut a,
            "a1",
            "Async agent launched successfully.\nagentId: bg1",
            json!({"status":"async_launched","agentId":"bg1"}),
        );
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"bg1","tool_use_id":"a1",
                "description":"Tests","task_type":"local_agent","is_backgrounded":true}),
        );
        assert_eq!(plan(&a).agents[0].status, RunStatus::Running);
        let fx = system(
            &mut a,
            json!({"type":"system","subtype":"task_notification","task_id":"bg1","tool_use_id":"a1",
                "status":"completed","summary":"Agent \"Tests\" finished","uuid":"n1",
                "usage":{"total_tokens":700,"tool_uses":4,"duration_ms":10}}),
        );
        let p = plan(&a);
        assert_eq!(p.agents[0].status, RunStatus::Done);
        assert_eq!((p.agents[0].tools, p.agents[0].tokens), (4, Some(700)));
        assert!(fx.save);
        // The event row of the conversation is still made.
        assert_eq!(a.conv.get("n1").unwrap()["source"], "task");
    }

    #[test]
    fn a_workflow_is_followed_from_the_system_frames() {
        let mut a = rt();
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"w1","tool_use_id":"tw",
                "description":"Revue","task_type":"local_workflow","workflow_name":"review"}),
        );
        let fx = system(
            &mut a,
            json!({"type":"system","subtype":"task_progress","task_id":"w1","tool_use_id":"tw",
                "description":"Phase 1 : lecteur A","usage":{"total_tokens":50,"tool_uses":2,"duration_ms":1}}),
        );
        assert!(fx.agent_changed && !fx.save);
        let p = plan(&a);
        assert_eq!(p.workflows[0].now.as_deref(), Some("Phase 1 : lecteur A"));
        assert_eq!(p.workflows[0].status, RunStatus::Running);
        // A background command is not a workflow, nor a subagent.
        system(
            &mut a,
            json!({"type":"system","subtype":"task_started","task_id":"b1","tool_use_id":"tb",
                "description":"npm test","task_type":"local_bash","is_backgrounded":true}),
        );
        assert_eq!(plan(&a).workflows.len(), 1);
        assert!(plan(&a).agents.is_empty());
    }

    #[test]
    fn a_new_conversation_starts_a_plan_of_its_own() {
        let mut a = rt();
        session(&mut a, "s1");
        creates(&mut a, "c0", "Lire", 1);
        // The same conversation, a new process: the plan goes on.
        session(&mut a, "s1");
        assert_eq!(plan(&a).tasks.len(), 1);
        // « /clear »: another session, nothing of the plan is in the context.
        let fx = session(&mut a, "s2");
        assert!(a.view().meta.plan.is_none());
        assert!(fx.save && fx.agent_changed);
    }

    #[test]
    fn a_message_starts_a_new_plan_only_after_a_plan_that_is_over() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("h1", "Fais ceci", 0, &[], &mut fx);
        creates(&mut a, "c0", "Lire", 1);
        turn_ends(&mut a);
        // The plan is under way: a message goes on with it.
        a.push_user("h2", "Continue", 0, &[], &mut fx);
        assert_eq!(plan(&a).tasks.len(), 1);
        call(
            &mut a,
            None,
            "u1",
            "TaskUpdate",
            json!({"taskId":"1","status":"completed"}),
        );
        // A message queued behind the running turn is not the start of one.
        assert!(a.push_user("h3", "Et aussi", 0, &[], &mut fx));
        assert_eq!(plan(&a).tasks.len(), 1);
        turn_ends(&mut a);
        // The plan is over, and so is the turn: the next message starts another.
        let mut fx = Effects::default();
        a.push_user("h4", "Autre chose", 0, &[], &mut fx);
        assert!(a.view().meta.plan.is_none());
        assert!(fx.save);
        // No plan at all: nothing to reset, nothing made.
        turn_ends(&mut a);
        a.push_user("h5", "Encore", 0, &[], &mut Effects::default());
        assert!(a.view().meta.plan.is_none());
    }

    #[test]
    fn a_background_subagent_still_running_keeps_the_plan_across_a_message() {
        let mut a = rt();
        let mut fx = Effects::default();
        a.push_user("h1", "Fais ceci", 0, &[], &mut fx);
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Tests","prompt":"","run_in_background":true}),
        );
        turn_ends(&mut a);
        a.push_user("h2", "Où en est-il ?", 0, &[], &mut fx);
        assert_eq!(plan(&a).agents[0].status, RunStatus::Running);
    }

    #[test]
    fn the_end_of_a_turn_ends_the_subagents_of_the_foreground_only() {
        let mut a = rt();
        call(
            &mut a,
            None,
            "a1",
            "Agent",
            json!({"description":"Avant","prompt":""}),
        );
        call(
            &mut a,
            None,
            "a2",
            "Agent",
            json!({"description":"Fond","prompt":"","run_in_background":true}),
        );
        turn_ends(&mut a);
        let p = plan(&a);
        assert_eq!(p.agents[0].status, RunStatus::Interrupted);
        assert_eq!(p.agents[1].status, RunStatus::Running);
    }

    #[test]
    fn what_ran_in_a_process_that_is_gone_is_interrupted() {
        let busy = |a: &mut AgentRt| {
            creates(a, "c0", "Lire", 1);
            call(
                a,
                None,
                "u1",
                "TaskUpdate",
                json!({"taskId":"1","status":"in_progress"}),
            );
            call(
                a,
                None,
                "a1",
                "Agent",
                json!({"description":"Fond","prompt":"","run_in_background":true}),
            );
        };
        let interrupted = |a: &AgentRt| {
            let p = plan(a);
            assert_eq!(p.agents[0].status, RunStatus::Interrupted);
            // The tasks keep their status: the agent picks them up again.
            assert_eq!(p.tasks[0].status, TaskStatus::InProgress);
        };
        // Stopped on purpose.
        let mut a = rt();
        busy(&mut a);
        a.detach();
        interrupted(&a);
        // The process died.
        let mut a = rt();
        busy(&mut a);
        let mut fx = Effects::default();
        a.on_exit(a.gen, Some(1), "", &mut fx);
        interrupted(&a);
        assert!(fx.save && fx.agent_changed);
        // The app was closed: the plan saved with it is loaded interrupted.
        let mut a = rt();
        busy(&mut a);
        let b = AgentRt::new(
            a.meta.clone(),
            &std::env::temp_dir().join("ccm-agent-tests-plan"),
        );
        interrupted(&b);
        assert_eq!(
            b.view().meta.plan.unwrap().agents[0].status,
            RunStatus::Interrupted
        );
    }

    // ---------- the plan of the folder ----------

    /// The result of the call `id`, in the thread of `parent`.
    fn returns_in(a: &mut AgentRt, parent: Option<&str>, id: &str) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":id,"content":"ok"}]},"tool_use_result":null,"parent_tool_use_id":parent}),
            &mut fx,
        );
        fx
    }

    fn turn_result(a: &mut AgentRt) -> Effects {
        let mut fx = Effects::default();
        a.handle_frame(
            &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1}),
            &mut fx,
        );
        fx
    }

    #[test]
    fn a_command_a_subagent_or_the_end_of_a_turn_calls_for_a_look_at_the_plans_files() {
        let mut a = rt();
        // The call itself does not: what it did is known with its result.
        assert!(!call(&mut a, None, "b1", "Bash", json!({"command":"task-done"})).plan_files);
        assert!(returns(&mut a, "b1", "ledger: Task 1: complete", Value::Null).plan_files);
        // Once: the same result again says nothing new.
        assert!(!returns(&mut a, "b1", "again", Value::Null).plan_files);
        for (id, name) in [("s1", "Agent"), ("s2", "Task")] {
            call(
                &mut a,
                None,
                id,
                name,
                json!({"description":"Aide","prompt":"x"}),
            );
            assert!(
                returns(&mut a, id, "done", Value::Null).plan_files,
                "{name}"
            );
        }
        // Reading, searching, editing source: not what the files of a plan move with.
        for (id, name, input) in [
            ("r1", "Read", json!({"file_path":"C:/p/src/a.ts"})),
            ("g1", "Grep", json!({"pattern":"x"})),
            ("e1", "Edit", json!({"file_path":"C:/p/src/a.ts"})),
            ("w1", "Write", json!({"file_path":"C:/p/docs/notes.md"})),
        ] {
            call(&mut a, None, id, name, input);
            assert!(!returns(&mut a, id, "ok", Value::Null).plan_files, "{name}");
        }
        // A command of a subagent is its own: the ledger is the main thread's.
        call(
            &mut a,
            None,
            "s3",
            "Agent",
            json!({"description":"Aide","prompt":"x"}),
        );
        call(
            &mut a,
            Some("s3"),
            "k1",
            "Bash",
            json!({"command":"npm test"}),
        );
        assert!(!returns_in(&mut a, Some("s3"), "k1").plan_files);
        // The end of the turn.
        assert!(turn_result(&mut a).plan_files);
    }

    #[test]
    fn a_plan_the_agent_writes_is_remembered_for_the_look_and_its_write_calls_for_one() {
        let mut a = rt();
        let abs = "C:/p/docs/superpowers/plans/2026-10-10-demo.md";
        call(&mut a, None, "w1", "Write", json!({"file_path":abs}));
        assert_eq!(a.plan_written, [abs]);
        assert!(returns(&mut a, "w1", "ok", Value::Null).plan_files);
        // Relative to its folder, with either slash, edited or written: as an absolute path.
        call(
            &mut a,
            None,
            "w2",
            "Edit",
            json!({"file_path":"docs/superpowers/plans/other.md"}),
        );
        call(
            &mut a,
            None,
            "w3",
            "MultiEdit",
            json!({"file_path":"C:\\p\\docs\\superpowers\\plans\\third.md"}),
        );
        assert_eq!(
            a.plan_written,
            [
                abs,
                "C:/p/docs/superpowers/plans/other.md",
                "C:\\p\\docs\\superpowers\\plans\\third.md"
            ]
        );
        // Written again: it is the latest, once.
        call(&mut a, None, "w4", "Write", json!({"file_path":abs}));
        assert_eq!(a.plan_written.last().map(String::as_str), Some(abs));
        assert_eq!(a.plan_written.len(), 3);
        // What is no plan of the plans folder, a subagent's writes, a Write without a path.
        for (parent, input) in [
            (None, json!({"file_path":"C:/p/src/a.ts"})),
            (
                None,
                json!({"file_path":"C:/p/docs/superpowers/specs/s.md"}),
            ),
            (
                None,
                json!({"file_path":"C:/p/docs/superpowers/plans/notes.txt"}),
            ),
            (None, json!({"content":"x"})),
            (
                Some("s1"),
                json!({"file_path":"C:/p/docs/superpowers/plans/sub.md"}),
            ),
        ] {
            call(&mut a, parent, "wx", "Write", input);
        }
        assert_eq!(a.plan_written.len(), 3);
        // Five at most: the oldest go.
        for i in 0..4 {
            call(
                &mut a,
                None,
                &format!("n{i}"),
                "Write",
                json!({"file_path": format!("C:/p/docs/superpowers/plans/n{i}.md")}),
            );
        }
        assert_eq!(a.plan_written.len(), 5);
        assert!(!a.plan_written.iter().any(|p| p.ends_with("/demo.md")));
        assert!(a.plan_written[4].ends_with("/n3.md"));
    }

    #[test]
    fn a_new_conversation_forgets_the_plans_it_wrote_and_dates_what_it_will_read() {
        let mut a = rt();
        let born = a.plan_since;
        assert!(born > now_ms() - 60_000 && born <= now_ms());
        session(&mut a, "s1");
        call(
            &mut a,
            None,
            "w1",
            "Write",
            json!({"file_path":"C:/p/docs/superpowers/plans/a.md"}),
        );
        call(&mut a, None, "b1", "Bash", json!({"command":"ls"}));
        assert_eq!(a.plan_written.len(), 1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        // The same conversation again (the process restarted): nothing changes.
        session(&mut a, "s1");
        assert_eq!(a.plan_written.len(), 1);
        // Another one.
        session(&mut a, "s2");
        assert!(a.plan_written.is_empty());
        assert!(a.plan_since > born);
        // A result of a call of the one before does not look at the files of this one.
        assert!(!returns(&mut a, "b1", "ok", Value::Null).plan_files);
    }

    #[test]
    fn a_message_after_a_finished_plan_forgets_what_it_wrote_and_dates_what_it_reads() {
        let mut a = rt();
        a.push_user("h1", "Fais ceci", 0, &[], &mut Effects::default());
        creates(&mut a, "c0", "Lire", 1);
        call(
            &mut a,
            None,
            "w1",
            "Write",
            json!({"file_path":"C:/p/docs/superpowers/plans/a.md"}),
        );
        call(
            &mut a,
            None,
            "u1",
            "TaskUpdate",
            json!({"taskId":"1","status":"completed"}),
        );
        turn_ends(&mut a);
        // A plan under way keeps what it has: here the plan is over.
        let before = a.plan_since;
        std::thread::sleep(std::time::Duration::from_millis(5));
        a.push_user("h2", "Autre chose", 0, &[], &mut Effects::default());
        assert!(a.plan_written.is_empty());
        assert!(a.plan_since > before);

        let mut b = rt();
        b.push_user("h1", "Fais ceci", 0, &[], &mut Effects::default());
        creates(&mut b, "c0", "Lire", 1);
        call(
            &mut b,
            None,
            "w1",
            "Write",
            json!({"file_path":"C:/p/docs/superpowers/plans/a.md"}),
        );
        turn_ends(&mut b);
        b.push_user("h2", "Continue", 0, &[], &mut Effects::default());
        assert_eq!(b.plan_written.len(), 1);
    }

    fn demo_list() -> Option<FileList> {
        Some(FileList {
            plan_file: "docs/superpowers/plans/demo.md".into(),
            title: Some("Démo".into()),
            tasks: vec![crate::plan::PlanTask {
                id: "1".into(),
                title: "Un".into(),
                status: TaskStatus::InProgress,
                ..Default::default()
            }],
        })
    }

    #[test]
    fn the_files_read_come_in_if_the_conversation_is_still_the_one_they_were_read_for() {
        let mut a = rt();
        session(&mut a, "s1");
        let since = a.plan_since;
        let mut fx = Effects::default();
        assert!(a.plan_from_files(since, demo_list(), &mut fx));
        assert!(fx.agent_changed && fx.save);
        let p = plan(&a);
        assert_eq!(p.tasks[0].status, TaskStatus::InProgress);
        assert_eq!(p.title.as_deref(), Some("Démo"));
        assert_eq!(
            serde_json::to_value(a.view()).unwrap()["plan"]["planFile"],
            "docs/superpowers/plans/demo.md"
        );
        // Again: nothing to tell.
        let mut fx = Effects::default();
        assert!(a.plan_from_files(since, demo_list(), &mut fx));
        assert!(!fx.agent_changed && !fx.save);
        // Read for a conversation that has ended since: not applied.
        std::thread::sleep(std::time::Duration::from_millis(5));
        session(&mut a, "s2");
        assert!(a.view().meta.plan.is_none());
        let mut fx = Effects::default();
        assert!(!a.plan_from_files(since, demo_list(), &mut fx));
        assert!(a.view().meta.plan.is_none());
        assert!(!fx.agent_changed && !fx.save);
        // Nothing read: nothing done.
        assert!(a.plan_from_files(a.plan_since, None, &mut Effects::default()));
        assert!(a.view().meta.plan.is_none());
    }

    #[test]
    fn the_tasks_of_the_files_give_way_to_the_ones_the_agent_lists() {
        let mut a = rt();
        let since = a.plan_since;
        a.plan_from_files(since, demo_list(), &mut Effects::default());
        assert_eq!(plan(&a).tasks[0].title, "Un");
        creates(&mut a, "c0", "Task 1: Un", 1);
        let p = plan(&a);
        assert_eq!(p.tasks.len(), 1);
        assert_eq!(p.tasks[0].title, "Task 1: Un");
        assert_eq!(p.source, Some(crate::plan::PlanSource::Tools));
        // The files go on being read: its list is the agent's, named and titled by the plan its
        // tasks are the tasks of.
        a.plan_from_files(since, demo_list(), &mut Effects::default());
        assert_eq!(plan(&a).tasks[0].title, "Task 1: Un");
        assert_eq!(plan(&a).title.as_deref(), Some("Démo"));

        // Another agent in the same repository, with tasks of its own: the plan is not its plan.
        let mut b = rt();
        creates(&mut b, "c0", "Ma tâche", 1);
        b.plan_from_files(b.plan_since, demo_list(), &mut Effects::default());
        assert_eq!(plan(&b).tasks[0].title, "Ma tâche");
        assert_eq!(plan(&b).title, None);
        assert_eq!(plan(&b).plan_file, None);
    }

    #[test]
    fn writing_the_workspace_of_a_plan_calls_for_a_look_but_is_no_plan_written() {
        let mut a = rt();
        for (id, name, path) in [
            ("w1", "Write", "C:/p/.superpowers/sdd/demo/progress.md"),
            ("w2", "Edit", ".superpowers/sdd/demo/task-3-brief.md"),
            (
                "w3",
                "MultiEdit",
                "C:\\p\\.superpowers\\sdd\\demo\\progress.md",
            ),
            (
                "w4",
                "Write",
                "C:/p/sub/.superpowers/sdd/demo/task-1-brief.md",
            ),
        ] {
            call(&mut a, None, id, name, json!({ "file_path": path }));
            assert!(
                returns(&mut a, id, "ok", Value::Null).plan_files,
                "{name} {path}"
            );
        }
        assert!(a.plan_written.is_empty());
        // Other files of that folder, and anything a subagent writes, do not.
        call(
            &mut a,
            None,
            "s1",
            "Agent",
            json!({"description":"Aide","prompt":"x"}),
        );
        for (parent, id, path) in [
            (None, "x1", "C:/p/.superpowers/other/notes.md"),
            (None, "x2", "C:/p/.superpowers/sdd-notes.md"),
            (Some("s1"), "x3", "C:/p/.superpowers/sdd/demo/progress.md"),
        ] {
            call(&mut a, parent, id, "Write", json!({ "file_path": path }));
            let fx = if parent.is_some() {
                returns_in(&mut a, parent, id)
            } else {
                returns(&mut a, id, "ok", Value::Null)
            };
            assert!(!fx.plan_files, "{path}");
        }
    }

    #[test]
    fn an_agent_saved_before_the_plan_loads_without_one_and_a_saved_plan_comes_back() {
        let meta: AgentMeta = serde_json::from_value(json!({"id":"a1","name":"ancien"})).unwrap();
        assert!(meta.plan.is_none());
        // Nothing is written for it.
        assert!(serde_json::to_value(&meta).unwrap().get("plan").is_none());
        let mut a = rt();
        creates(&mut a, "c0", "Lire", 1);
        let saved = serde_json::to_value(&a.meta).unwrap();
        assert_eq!(saved["plan"]["tasks"][0]["title"], "Lire");
        let back: AgentMeta = serde_json::from_value(saved.clone()).unwrap();
        assert_eq!(serde_json::to_value(&back).unwrap(), saved);
        // The window gets it with the agent.
        assert_eq!(
            serde_json::to_value(a.view()).unwrap()["plan"]["tasks"][0]["status"],
            "pending"
        );
    }
}
