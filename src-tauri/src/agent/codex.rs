//! Codex's messages (`codex app-server`, docs/PROTOCOL.md, « Codex app-server ») translated into
//! the agent's conversation: the engine `Codex` hands them here (`Engine::translate`).
//!
//! Its tools take the names Claude Code gives them (docs/TODO-1.8.md, « Un moteur par agent »):
//! `commandExecution` is `Bash`, `fileChange` is `Edit` or `Write` (with its +N −M), `mcpToolCall`
//! is `mcp__<server>__<tool>`, `webSearch` is `WebSearch`, and a subagent (`subAgentActivity`) is
//! a `Task`. A subagent has a thread of its own, whose messages come on the same connection: they
//! are told apart by their `threadId`, and shown under its `Task`.
//!
//! A recorded stream (`tests/fixtures/codex/`, one `{"dir", "msg"}` a line) is read as it came:
//! what Codex said (`s2c`) is translated, what was said to it is left out.

use super::*;
use crate::engine::{codex_mode_of, codex_request_id, plan_approval};

/// What the translation keeps of Codex's conversation between its messages (not saved).
#[derive(Default)]
pub(crate) struct CodexState {
    /// The threads of its subagents, and the call (`Task`) of each.
    children: HashMap<String, String>,
    /// What each subagent said last: its report, given to its call when it ends.
    reports: HashMap<String, String>,
    /// The calls of subagents that ended.
    ended: HashSet<String>,
    /// The turn of its thread under way.
    turn: Option<String>,
    /// The tokens of each thread so far (`total` of `thread/tokenUsage/updated`), and those of the
    /// threads when the turn began: a turn's are the difference.
    tokens: HashMap<String, Tokens>,
    tokens_at_start: HashMap<String, Tokens>,
    /// The plan of the mode Plan, its turn over on it: it waits for its approval.
    plan: Option<String>,
    /// The updates of the plan tool, numbered (each one a list of tasks of its own).
    plan_updates: u32,
    /// The files of each `fileChange`, by call, for the card of its approval.
    changes: HashMap<String, Value>,
    /// When a usage limit met resets (ms).
    limit_resets: Option<i64>,
    /// What went wrong in the turn, as Codex said it (`error`).
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Tokens {
    input: u64,
    cached: u64,
    output: u64,
}

impl Tokens {
    fn of(v: &Value) -> Self {
        let n = |k: &str| v[k].as_u64().unwrap_or(0);
        Tokens {
            input: n("inputTokens"),
            cached: n("cachedInputTokens") + n("cacheWriteInputTokens"),
            output: n("outputTokens"),
        }
    }

    fn minus(self, before: Tokens) -> Tokens {
        Tokens {
            input: self.input.saturating_sub(before.input),
            cached: self.cached.saturating_sub(before.cached),
            output: self.output.saturating_sub(before.output),
        }
    }
}

/// Where a message of Codex belongs.
enum Thread {
    /// The agent's own.
    Own,
    /// A subagent's, under its call.
    Child(String),
    /// Neither (one of a subagent not known yet).
    Other,
}

impl AgentRt {
    /// One message of its `codex app-server`.
    pub(crate) fn codex_frame(&mut self, f: &Value, fx: &mut Effects) {
        // A recorded stream: what Codex said, as it came.
        if let Some(dir) = f.get("dir").and_then(Value::as_str) {
            if dir == "s2c" {
                self.codex_frame(&f["msg"], fx);
            }
            return;
        }
        let Some(method) = f["method"].as_str() else {
            return;
        };
        let p = &f["params"];
        let thread = self.codex_thread(p["threadId"].as_str());
        let parent = match &thread {
            Thread::Child(call) => Some(call.clone()),
            _ => None,
        };
        let own = matches!(thread, Thread::Own);
        match method {
            "thread/started" => self.codex_thread_started(&p["thread"], fx),
            "thread/settings/updated" if own => {
                if let Some(mode) = codex_mode_of(&p["threadSettings"], &self.meta.mode) {
                    if self.meta.set_mode(mode) {
                        fx.agent_changed = true;
                        fx.save = true;
                    }
                }
            }
            "turn/started" if own => {
                self.codex.turn = p["turn"]["id"].as_str().map(str::to_string);
                self.codex.tokens_at_start = self.codex.tokens.clone();
                self.codex.error = None;
                // A copy has a session of its own from its first turn on.
                if self.meta.fork_of.take().is_some() {
                    self.meta.fork_at = None;
                    fx.save = true;
                }
                if !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "turn/completed" if own => {
                let turn = &p["turn"];
                if self.codex.turn.is_none() || self.codex.turn.as_deref() == turn["id"].as_str() {
                    self.codex_turn_end(turn, fx);
                }
            }
            "item/started" | "item/completed" if !matches!(thread, Thread::Other) => {
                self.codex_item(&p["item"], parent, method == "item/completed", fx);
            }
            "item/agentMessage/delta"
            | "item/reasoning/summaryTextDelta"
            | "item/reasoning/textDelta" => {
                let (Some(id), Some(text)) = (p["itemId"].as_str(), p["delta"].as_str()) else {
                    return;
                };
                if text.is_empty() || matches!(thread, Thread::Other) {
                    return;
                }
                if !self.conv.contains(id) {
                    // A reasoning is shown from its first words (Codex often has none).
                    let item = json!({ "kind": "thinking", "id": id, "text": "", "parent": parent, "streaming": true });
                    self.append(item, fx);
                }
                self.push(
                    ConvOp::Delta {
                        id: id.to_string(),
                        text: text.to_string(),
                    },
                    fx,
                );
            }
            "turn/plan/updated" if own => {
                self.codex.plan_updates += 1;
                let todos: Vec<Value> = p["plan"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|s| {
                        let status = match s["status"].as_str() {
                            Some("inProgress") => "in_progress",
                            Some("completed") => "completed",
                            _ => "pending",
                        };
                        json!({ "content": s["step"], "status": status, "activeForm": s["step"] })
                    })
                    .collect();
                let block = json!({
                    "id": format!("codex-plan-{}", self.codex.plan_updates),
                    "name": "TodoWrite",
                    "input": { "todos": todos },
                });
                self.plan_tool_use(None, &block, fx);
            }
            "thread/tokenUsage/updated" => {
                let Some(t) = p["threadId"].as_str() else {
                    return;
                };
                let usage = &p["tokenUsage"];
                self.codex
                    .tokens
                    .insert(t.to_string(), Tokens::of(&usage["total"]));
                if own {
                    // What its context holds, as Codex counts it (`totalTokens` of the last
                    // call): after a compaction, its `inputTokens` are 0 (full-turn.jsonl).
                    let last = &usage["last"];
                    self.context_tokens = last["totalTokens"]
                        .as_u64()
                        .unwrap_or_else(|| Tokens::of(last).input);
                    if let Some(w) = usage["modelContextWindow"].as_u64() {
                        self.context_window = w;
                    }
                    fx.agent_changed = true;
                }
            }
            "account/rateLimits/updated" => {
                let limits = &p["rateLimits"];
                let meters = crate::engine::codex_meters(limits);
                if !limits["rateLimitReachedType"].is_null() {
                    self.codex.limit_resets = meters
                        .iter()
                        .filter(|m| m.pct >= 100.0)
                        .filter_map(|m| m.resets_at)
                        .max()
                        .or_else(|| meters.iter().filter_map(|m| m.resets_at).max());
                }
                if !meters.is_empty() {
                    fx.rate = Some(meters);
                }
            }
            "item/commandExecution/requestApproval"
            | "item/fileChange/requestApproval"
            | "item/permissions/requestApproval" => self.codex_approval(method, f, parent, fx),
            "item/tool/requestUserInput" => self.codex_question(f, parent, fx),
            // In Auto, Codex's automatic reviewer decided a request on its own: the conversation
            // says so, as it was asked of nobody.
            "guardianWarning" if !matches!(thread, Thread::Other) => {
                if let Some(message) = p["message"].as_str().filter(|m| !m.trim().is_empty()) {
                    self.notice(
                        "info",
                        tr!(
                            "Relecteur automatique de Codex : {message}",
                            "Codex’s automatic reviewer: {message}"
                        ),
                        fx,
                    );
                }
            }
            "serverRequest/resolved" => {
                // Closed by Codex itself (an interrupted turn): its card goes.
                let rid = codex_request_id(&p["requestId"]);
                if let Some(req) = self.take_pending(&rid) {
                    self.patch(&req.item_id, json!({ "cancelled": true }), fx);
                    if self.pending.is_empty() && self.meta.status == AgentStatus::Waiting {
                        self.set_status(AgentStatus::Running, fx);
                    }
                    fx.agent_changed = true;
                }
            }
            "error" => {
                if p["willRetry"] == true {
                    return;
                }
                let message = codex_error_text(&p["error"]);
                if p["refused"] == "turn/start" && self.meta.status.is_active() {
                    // The turn never began: it ends here, in error.
                    let turn = json!({ "status": "failed", "error": { "message": message } });
                    self.codex_turn_end(&turn, fx);
                } else if !matches!(thread, Thread::Child(_)) {
                    self.codex.error = Some(message);
                }
            }
            _ => {}
        }
    }

    /// The turn of its thread under way.
    pub(super) fn codex_turn(&self) -> Option<String> {
        self.codex.turn.clone()
    }

    /// Another conversation (or another process of it): what was kept of the last one is gone.
    pub(super) fn codex_new_conversation(&mut self) {
        self.codex = CodexState::default();
    }

    /// Whose a message of the thread `thread` is.
    fn codex_thread(&self, thread: Option<&str>) -> Thread {
        let Some(t) = thread else {
            return Thread::Own;
        };
        if let Some(call) = self.codex.children.get(t) {
            return Thread::Child(call.clone());
        }
        match self.meta.session_id.as_deref() {
            Some(own) if own != t => Thread::Other,
            _ => Thread::Own,
        }
    }

    /// The agent's thread, new, resumed or copied: its session.
    fn codex_thread_started(&mut self, thread: &Value, fx: &mut Effects) {
        let Some(id) = thread["id"].as_str() else {
            return;
        };
        if !thread["parentThreadId"].is_null() || self.codex.children.contains_key(id) {
            return;
        }
        if self.meta.session_id.as_deref() != Some(id) {
            let resumed = self.meta.session_id.is_some() && self.meta.fork_of.is_none();
            // Another conversation: what the plan was about is not in its context.
            self.reset_plan(fx);
            if resumed {
                self.notice(
                    "warn",
                    tr!(
                        "La conversation Codex n’a pas pu être reprise : nouvelle conversation (contexte vidé)",
                        "The Codex conversation couldn’t be resumed: new conversation (context cleared)"
                    ),
                    fx,
                );
            }
            self.meta.session_id = Some(id.to_string());
            self.meta.last_entry = None;
            fx.save = true;
        }
        self.saw_init = true;
    }

    /// An item of a turn, as it starts (`done` false) and once it is over.
    fn codex_item(&mut self, item: &Value, parent: Option<String>, done: bool, fx: &mut Effects) {
        let Some(id) = item["id"].as_str() else {
            return;
        };
        let main = parent.is_none();
        match item["type"].as_str().unwrap_or_default() {
            "agentMessage" => {
                let text = item["text"].as_str().unwrap_or_default();
                if !done {
                    if !self.conv.contains(id) {
                        let item = json!({ "kind": "text", "id": id, "text": "", "parent": parent, "streaming": true });
                        self.append(item, fx);
                    }
                    if main {
                        self.set_activity(block_activity(i18n::ui(), "text"), fx);
                    }
                    return;
                }
                if main && !text.trim().is_empty() {
                    if !self.turn_text.is_empty() {
                        self.turn_text.push_str("\n\n");
                    }
                    self.turn_text.push_str(text);
                    self.final_text = text.to_string();
                }
                if let Some(call) = &parent {
                    if !text.trim().is_empty() {
                        self.codex.reports.insert(call.clone(), text.to_string());
                    }
                }
                let whole = json!({ "text": text, "streaming": false });
                if self.conv.contains(id) {
                    self.patch(id, whole, fx);
                } else {
                    let mut item = whole;
                    item["kind"] = json!("text");
                    item["id"] = json!(id);
                    item["parent"] = json!(parent);
                    self.append(item, fx);
                }
            }
            "reasoning" => {
                if !done {
                    if main {
                        self.set_activity(block_activity(i18n::ui(), "thinking"), fx);
                    }
                    return;
                }
                let parts = |k: &str| -> Vec<String> {
                    item[k]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|s| s.as_str().map(str::to_string))
                        .filter(|s| !s.trim().is_empty())
                        .collect()
                };
                let mut text = parts("summary");
                if text.is_empty() {
                    text = parts("content");
                }
                let text = text.join("\n\n");
                if self.conv.contains(id) {
                    self.patch(id, json!({ "text": text, "streaming": false }), fx);
                } else if !text.is_empty() {
                    let item = json!({ "kind": "thinking", "id": id, "text": text, "parent": parent, "streaming": false });
                    self.append(item, fx);
                }
            }
            "commandExecution" => {
                let input = json!({ "command": shown_command(item) });
                let new = parent.is_none() && !self.conv.contains(id);
                self.codex_call(id, "Bash", input, parent, fx);
                if let Some((tool, what)) = command_tool(item).filter(|_| new) {
                    // Said as Claude Code says it: it reads a file with Read, not with a command.
                    let activity = tool_activity(tool, &what, &self.meta.cwd);
                    self.set_activity(activity, fx);
                }
                if !done {
                    return;
                }
                let ok =
                    item["status"] == "completed" && item["exitCode"].as_i64().unwrap_or(0) == 0;
                let text = item["aggregatedOutput"].as_str().unwrap_or_default();
                self.codex_result(id, !ok, text.to_string(), &Value::Null, fx);
            }
            "fileChange" => {
                self.codex
                    .changes
                    .insert(id.to_string(), item["changes"].clone());
                let changes = item["changes"].as_array().cloned().unwrap_or_default();
                for (n, change) in changes.iter().enumerate() {
                    let call = if n == 0 {
                        id.to_string()
                    } else {
                        format!("{id}:{n}")
                    };
                    let (name, input, tur) = file_change(change);
                    self.codex_call(&call, name, input, parent.clone(), fx);
                    if done {
                        let failed = item["status"] != "completed";
                        if let (false, Some(path)) = (failed, change["path"].as_str()) {
                            self.touch(path, fx);
                        }
                        self.codex_result(&call, failed, String::new(), &tur, fx);
                    }
                }
            }
            "mcpToolCall" => {
                let name = crate::engine::mcp::tool_name(
                    item["server"].as_str().unwrap_or_default(),
                    item["tool"].as_str().unwrap_or_default(),
                );
                self.codex_call(id, &name, item["arguments"].clone(), parent, fx);
                if done {
                    let failed = !item["error"].is_null() || item["status"] == "failed";
                    let text = if failed {
                        item["error"]["message"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string()
                    } else {
                        tool_result_text(&item["result"]["content"])
                    };
                    self.codex_result(id, failed, text, &Value::Null, fx);
                }
            }
            "dynamicToolCall" => {
                let name = item["tool"].as_str().unwrap_or("tool").to_string();
                self.codex_call(id, &name, item["arguments"].clone(), parent, fx);
                if done {
                    let failed = item["success"] == false || item["status"] == "failed";
                    let text = tool_result_text(&item["contentItems"]);
                    self.codex_result(id, failed, text, &Value::Null, fx);
                }
            }
            "webSearch" => {
                let query = item["query"]
                    .as_str()
                    .filter(|q| !q.is_empty())
                    .or_else(|| item["action"]["query"].as_str())
                    .unwrap_or_default();
                self.codex_call(id, "WebSearch", json!({ "query": query }), parent, fx);
                if done {
                    self.codex_result(id, false, String::new(), &Value::Null, fx);
                }
            }
            "imageView" => {
                let input = json!({ "file_path": item["path"] });
                self.codex_call(id, "Read", input, parent, fx);
                if done {
                    self.codex_result(id, false, String::new(), &Value::Null, fx);
                }
            }
            "subAgentActivity" => {
                let Some(child) = item["agentThreadId"].as_str() else {
                    return;
                };
                match item["kind"].as_str() {
                    Some("started") => {
                        if self.codex.children.contains_key(child) {
                            return;
                        }
                        self.codex
                            .children
                            .insert(child.to_string(), id.to_string());
                        let name = item["agentPath"]
                            .as_str()
                            .and_then(|p| p.rsplit('/').next())
                            .unwrap_or("subagent")
                            .replace('_', " ");
                        let input = json!({
                            "description": name, "prompt": "", "model": item["model"],
                        });
                        self.codex_call(id, "Task", input, parent, fx);
                    }
                    Some("completed") => {
                        let Some(call) = self.codex.children.get(child).cloned() else {
                            return;
                        };
                        if !self.codex.ended.insert(call.clone()) {
                            return;
                        }
                        let report = self.codex.reports.remove(&call).unwrap_or_default();
                        self.codex_result(&call, false, report, &Value::Null, fx);
                    }
                    _ => {}
                }
            }
            // Asked (`/compact`, ESC-205) or by Codex itself: from its start to its end.
            "contextCompaction" if main => {
                let text = if done {
                    tr!("Contexte compacté", "Context compacted")
                } else {
                    tr!("Compaction du contexte…", "Compacting the context…")
                };
                self.notice("info", text, fx);
            }
            // The plan of the mode Plan: approved once the turn is over on it.
            "plan" if done && main => {
                let text = item["text"].as_str().unwrap_or_default();
                if !text.trim().is_empty() {
                    self.codex.plan = Some(text.to_string());
                }
            }
            _ => {}
        }
    }

    /// The call `id` of the tool `name` (Claude Code's name), shown once as it starts.
    fn codex_call(
        &mut self,
        id: &str,
        name: &str,
        input: Value,
        parent: Option<String>,
        fx: &mut Effects,
    ) {
        if self.conv.contains(id) {
            return;
        }
        if parent.is_none() {
            // What was said before a tool was a step on the way, not the final reply.
            self.final_text.clear();
            let activity = tool_activity(name, &input, &self.meta.cwd);
            self.set_activity(activity, fx);
        }
        let item = json!({
            "kind": "tool", "id": id, "name": name, "input": item_input(name, &input),
            "status": "running", "parent": parent, "ts": now_ms(),
        });
        self.append(item, fx);
        self.plan_tool_use(
            parent.as_deref(),
            &json!({ "id": id, "name": name, "input": input }),
            fx,
        );
    }

    /// The result of the call `id`: `text`, and for a file `tur` (as Claude Code's
    /// `tool_use_result`: its patch and its path).
    fn codex_result(
        &mut self,
        id: &str,
        is_error: bool,
        text: String,
        tur: &Value,
        fx: &mut Effects,
    ) {
        if self.conv.get(id).is_some_and(|i| i["status"] != "running") {
            return;
        }
        if self.plan_calls.remove(id) {
            fx.plan_files = true;
        }
        if self.plan_removals.remove(id) && !is_error {
            self.plan_apply(fx, PlanState::finish_files);
        }
        self.plan_apply(fx, |plan| {
            plan.on_tool_result(id, is_error, &text, tur, now_ms())
        });
        let result = tool_result(text, is_error, tur);
        self.patch(
            id,
            json!({ "status": if is_error { "error" } else { "ok" }, "result": result }),
            fx,
        );
    }

    /// A request of Codex to go on: a command, a change of files or more permissions.
    fn codex_approval(
        &mut self,
        method: &str,
        f: &Value,
        parent: Option<String>,
        fx: &mut Effects,
    ) {
        let p = &f["params"];
        let rid = codex_request_id(&f["id"]);
        let call = p["itemId"].as_str().unwrap_or_default().to_string();
        let asked = self.conv.get(&call).cloned().unwrap_or_default();
        let cwd = p["cwd"].as_str().unwrap_or(&self.meta.cwd).to_string();
        let mut description: Option<String> = None;
        let mut patch = Value::Null;
        let (tool, input) = match method {
            "item/commandExecution/requestApproval" => {
                let command = shown_command(p);
                // « Édits auto » (and Auto): `git add` and `git commit` of its own worktree, which
                // Codex's sandbox never lets write in `.git`, are accepted at once, judged on the
                // command run, not on its summary (`engine::codex_git`).
                let actions: Vec<&str> = p["commandActions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|a| a["command"].as_str())
                    .collect();
                if matches!(self.meta.mode.as_str(), "acceptEdits" | "auto")
                    && same_dir(&cwd, &self.meta.cwd)
                    && crate::engine::codex_git::accepts(
                        p["command"].as_str().unwrap_or_default(),
                        &actions,
                        std::path::Path::new(&self.meta.cwd),
                    )
                {
                    let allowed = PermissionAnswer::Allow {
                        call: call.clone(),
                        input: json!({ "command": command }),
                        resume: None,
                    };
                    if self
                        .proc
                        .as_ref()
                        .is_some_and(|p| p.answer_permission(&rid, allowed).is_ok())
                    {
                        log::info!(
                            "agent {}: git in its worktree, accepted: {command}",
                            self.meta.id
                        );
                        return;
                    }
                }
                if crate::engine::codex_git::writes_repository(&command) {
                    description = Some(tr!(
                        "git écrit dans le dépôt du projet, hors du bac à sable de Codex.",
                        "git writes in the project’s repository, outside Codex’s sandbox."
                    ));
                } else if !same_dir(&cwd, &self.meta.cwd) {
                    description = Some(tr!("Dans {cwd}", "In {cwd}"));
                }
                ("Bash".to_string(), json!({ "command": command }))
            }
            "item/fileChange/requestApproval" => {
                let changes = self.codex.changes.get(&call).cloned().unwrap_or_default();
                let first = changes.get(0).cloned().unwrap_or_default();
                let (name, input, tur) = file_change(&first);
                let hunks = tool_result(String::new(), false, &tur);
                patch = hunks["patch"].clone();
                let more = changes.as_array().map_or(0, |a| a.len().saturating_sub(1));
                if more > 0 {
                    description = Some(tr!(
                        "et {more} autre(s) fichier(s)",
                        "and {more} other file(s)"
                    ));
                }
                let name = asked["name"].as_str().unwrap_or(name).to_string();
                (name, input)
            }
            _ => {
                description = Some(permissions_text(&p["permissions"]));
                (
                    "Permissions".to_string(),
                    json!({ "permissions": p["permissions"] }),
                )
            }
        };
        let reason = p["reason"].as_str().map(str::to_string);
        let mut item = json!({
            "kind": "permission", "id": rid, "toolUseId": call, "toolName": tool,
            "description": description, "input": trim_strings(&input), "reason": reason,
            "canAlways": true, "alwaysScope": "session", "defaultNo": false,
            "decision": null, "ts": now_ms(),
        });
        if !patch.is_null() {
            item["patch"] = patch;
        }
        if let Some(sub) = parent {
            item["sub"] = sub.into();
        }
        let req = json!({ "description": description, "decision_reason": reason });
        let view = pending_view(&rid, &tool, &req, &input, &self.meta.cwd).always_as(&item);
        self.append(item, fx);
        self.add_pending(
            PendingReq {
                item_id: rid.clone(),
                tool_use_id: call,
                input: input.clone(),
                suggestions: Value::Null,
                view,
                since: now_ms(),
            },
            fx,
        );
        self.set_status(AgentStatus::Waiting, fx);
        fx.notify = Some(AgentAlert::new(
            NotifyKind::Question,
            notify::permission_body(&tool, &input, &self.meta.cwd),
        ));
        fx.agent_changed = true;
    }

    /// A question of Codex (`item/tool/requestUserInput`, in the mode Plan), as the card of a
    /// question of Claude Code's shows it; its questions keep their ids, for the answer.
    fn codex_question(&mut self, f: &Value, parent: Option<String>, fx: &mut Effects) {
        let p = &f["params"];
        let rid = codex_request_id(&f["id"]);
        let call = p["itemId"].as_str().unwrap_or_default().to_string();
        let questions: Vec<Value> = p["questions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|q| {
                let options: Vec<Value> = q["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|o| json!({ "label": o["label"], "description": o["description"].as_str().unwrap_or_default() }))
                    .collect();
                json!({
                    "id": q["id"], "question": q["question"], "header": q["header"].as_str().unwrap_or_default(),
                    "multiSelect": false, "options": options,
                })
            })
            .collect();
        let input = json!({ "questions": questions });
        let mut item = json!({
            "kind": "question", "id": rid, "toolUseId": call, "questions": questions,
            "answers": null, "ts": now_ms(),
        });
        if let Some(sub) = parent {
            item["sub"] = sub.into();
        }
        self.append(item, fx);
        let view = pending_view(
            &rid,
            "AskUserQuestion",
            &Value::Null,
            &input,
            &self.meta.cwd,
        );
        self.add_pending(
            PendingReq {
                item_id: rid,
                tool_use_id: call,
                input: input.clone(),
                suggestions: Value::Null,
                view,
                since: now_ms(),
            },
            fx,
        );
        self.set_status(AgentStatus::Waiting, fx);
        fx.notify = Some(AgentAlert::new(
            NotifyKind::Question,
            notify::question_body(crate::engine::of(&self.meta).speaker(), &input),
        ));
        fx.agent_changed = true;
    }

    /// The end of the agent's turn (`turn/completed`, or a `turn/start` refused).
    fn codex_turn_end(&mut self, turn: &Value, fx: &mut Effects) {
        let status = turn["status"].as_str().unwrap_or("completed");
        let interrupted = std::mem::take(&mut self.interrupted) || status == "interrupted";
        if let Some(id) = turn["id"].as_str() {
            // Where a copy made afterwards forks the thread (`thread/fork` with `lastTurnId`).
            if status != "failed" {
                self.set_last_entry(Some(id.to_string()), fx);
            }
        }
        self.codex.turn = None;
        // The mode Plan's plan: the turn stays open on its card until it is approved, as Claude
        // Code's `ExitPlanMode` does (its work, or more planning, is the turn's next part).
        if status == "completed" && !interrupted {
            if let Some(plan) = self.codex.plan.take() {
                let rid = plan_approval(turn["id"].as_str().unwrap_or("turn"));
                let input = json!({ "plan": plan });
                let item = json!({
                    "kind": "permission", "id": rid, "toolUseId": rid, "toolName": "ExitPlanMode",
                    "input": input, "canAlways": true, "defaultNo": false, "decision": null,
                    "ts": now_ms(),
                });
                self.append(item, fx);
                let view = pending_view(&rid, "ExitPlanMode", &Value::Null, &input, &self.meta.cwd);
                self.add_pending(
                    PendingReq {
                        item_id: rid.clone(),
                        tool_use_id: rid,
                        input: input.clone(),
                        suggestions: Value::Null,
                        view,
                        since: now_ms(),
                    },
                    fx,
                );
                self.close_open_items(fx);
                self.set_status(AgentStatus::Waiting, fx);
                fx.notify = Some(AgentAlert::new(
                    NotifyKind::Question,
                    notify::permission_body("ExitPlanMode", &input, &self.meta.cwd),
                ));
                fx.agent_changed = true;
                return;
            }
        }
        self.codex.plan = None;
        let error = &turn["error"];
        let info = error["codexErrorInfo"].as_str();
        let limited = !interrupted
            && status == "failed"
            && matches!(info, Some("usageLimitExceeded" | "rateLimitExceeded"));
        let failed = status == "failed" && !interrupted;
        let message = failed.then(|| {
            Some(codex_error_text(error))
                .filter(|m| !m.trim().is_empty())
                .or_else(|| self.codex.error.take())
                .unwrap_or_else(|| tr!("Codex a signalé une erreur", "Codex reported an error"))
        });
        // Its tokens: what each thread (its subagents' included) counted since the turn began.
        let mut used = Tokens::default();
        for (thread, now) in &self.codex.tokens {
            let before = self
                .codex
                .tokens_at_start
                .get(thread)
                .copied()
                .unwrap_or_default();
            let d = now.minus(before);
            used.input += d.input;
            used.cached += d.cached;
            used.output += d.output;
        }
        self.codex.tokens_at_start = self.codex.tokens.clone();
        let fresh = used.input.saturating_sub(used.cached);
        let tokens = fresh + used.cached + used.output;
        // Codex tells no cost: estimated from the public prices of OpenAI's models ('estimé').
        let usage = crate::pricing::Usage {
            input: fresh,
            cache_read: used.cached,
            output: used.output,
            ..Default::default()
        };
        let cost = crate::pricing::estimate_openai(&self.meta.model, &usage).unwrap_or(0.0);
        if tokens > 0 {
            fx.turns.push(TurnRow {
                engine: self.meta.engine,
                model: self.meta.model.clone(),
                input: fresh,
                cache: used.cached,
                output: used.output,
                cost,
            });
            self.meta.tokens += tokens;
            self.meta.cost += cost;
        }
        if limited {
            fx.limited = Some(self.codex.limit_resets.take());
        } else if self.meta.resume_at.take().is_some() {
            fx.save = true;
            fx.agent_changed = true;
        }
        let reply = std::mem::take(&mut self.final_text);
        let end = if interrupted {
            TurnEnd::Interrupted
        } else if limited {
            TurnEnd::Limited
        } else if let Some(reason) = message.clone() {
            TurnEnd::Error(reason)
        } else {
            TurnEnd::Finished(std::mem::take(&mut self.turn_text))
        };
        self.forget_turn();
        self.close_open_items(fx);
        self.plan_apply(fx, |plan| plan.end_turn(now_ms()));
        fx.plan_files = true;
        if !matches!(end, TurnEnd::Error(_)) && self.meta.moved_from.take().is_some() {
            self.meta.moved_resume_at = None;
            fx.save = true;
            fx.agent_changed = true;
        }
        let item = json!({
            "kind": "turn", "id": new_id(), "ts": now_ms(), "durationMs": turn["durationMs"],
            "cost": cost, "tokens": tokens, "isError": failed, "interrupted": interrupted,
            "error": message, "limited": limited, "engine": self.meta.engine,
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
            if failed && !limited {
                AgentStatus::Error
            } else {
                AgentStatus::Done
            },
            fx,
        );
        if !interrupted && !had_queue {
            fx.notify = Some(match &message {
                Some(reason) => AgentAlert::new(NotifyKind::Error, notify::error_body(reason)),
                None => AgentAlert::new(NotifyKind::Done, notify::done_body(&reply)),
            });
        }
        fx.agent_changed = true;
        fx.save = true;
        fx.turn_end = Some(end);
    }
}

/// What Codex said went wrong: its message, the API's own when it is one in JSON.
fn codex_error_text(error: &Value) -> String {
    let message = error["message"].as_str().unwrap_or_default();
    serde_json::from_str::<Value>(message)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| message.to_string())
}

/// The command as the model wrote it (`commandActions`), not wrapped in its shell
/// (`pwsh.exe -Command '…'` under Windows).
fn shown_command(v: &Value) -> String {
    let actions: Vec<&str> = v["commandActions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| a["command"].as_str())
        .collect();
    if actions.is_empty() {
        v["command"].as_str().unwrap_or_default().to_string()
    } else {
        actions.join(" && ")
    }
}

/// The tool of Claude Code for a command that only reads a file (`Read`) or only searches
/// (`Grep`), as `commandActions` tells it; None for any other command, or several actions.
fn command_tool(v: &Value) -> Option<(&'static str, Value)> {
    let [action] = v["commandActions"].as_array()?.as_slice() else {
        return None;
    };
    match action["type"].as_str()? {
        "read" => Some(("Read", json!({ "file_path": action["path"].as_str()? }))),
        "search" => Some(("Grep", json!({ "pattern": action["query"].as_str()? }))),
        _ => None,
    }
}

/// Whether two folders are the same, whatever their slashes and case.
fn same_dir(a: &str, b: &str) -> bool {
    let norm = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_lowercase();
    norm(a) == norm(b)
}

/// A change of `fileChange`: the tool Claude Code would have called for it, its input, and its
/// result as Claude Code's `tool_use_result` tells a file changed (its patch and its path).
fn file_change(change: &Value) -> (&'static str, Value, Value) {
    let path = change["path"].as_str().unwrap_or_default();
    let diff = change["diff"].as_str().unwrap_or_default();
    match change["kind"]["type"].as_str() {
        Some("add") => (
            "Write",
            json!({ "file_path": path, "content": diff }),
            json!({ "type": "create", "filePath": path, "content": diff, "structuredPatch": [] }),
        ),
        Some("delete") => {
            let lines: Vec<String> = diff.lines().map(|l| format!("-{l}")).collect();
            let hunks = json!([{ "oldStart": 1, "newStart": 0, "lines": lines }]);
            (
                "Edit",
                json!({ "file_path": path }),
                json!({ "filePath": path, "structuredPatch": hunks }),
            )
        }
        _ => {
            let mut input = json!({ "file_path": path });
            if let Some(to) = change["kind"]["move_path"].as_str() {
                input["move_path"] = json!(to);
            }
            (
                "Edit",
                input,
                json!({ "filePath": path, "structuredPatch": hunks_of(diff) }),
            )
        }
    }
}

/// The hunks of a unified diff, as Claude Code's `structuredPatch` lists them.
pub(super) fn hunks_of(diff: &str) -> Value {
    let mut hunks: Vec<Value> = Vec::new();
    for line in diff.lines() {
        if let Some(head) = line.strip_prefix("@@ ") {
            let num = |sign: char| {
                head.split_whitespace()
                    .find_map(|w| w.strip_prefix(sign))
                    .and_then(|w| w.split(',').next())
                    .and_then(|n| n.parse::<u64>().ok())
                    .unwrap_or(0)
            };
            hunks.push(json!({ "oldStart": num('-'), "newStart": num('+'), "lines": [] }));
            continue;
        }
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if hunks.is_empty() {
            if !(line.starts_with('+') || line.starts_with('-') || line.starts_with(' ')) {
                continue;
            }
            hunks.push(json!({ "oldStart": 1, "newStart": 1, "lines": [] }));
        }
        if let Some(lines) = hunks.last_mut().and_then(|h| h["lines"].as_array_mut()) {
            lines.push(json!(line));
        }
    }
    Value::Array(hunks)
}

/// What a request for more permissions asks, in words.
fn permissions_text(p: &Value) -> String {
    let mut parts = Vec::new();
    if p["network"]["enabled"] == true {
        parts.push(tr!("l’accès au réseau", "network access"));
    }
    let paths = |k: &str| -> Vec<String> {
        p["fileSystem"][k]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    };
    let write = paths("write");
    if !write.is_empty() {
        let list = write.join(", ");
        parts.push(tr!("l’écriture dans {list}", "writing in {list}"));
    }
    let read = paths("read");
    if !read.is_empty() {
        let list = read.join(", ");
        parts.push(tr!("la lecture de {list}", "reading {list}"));
    }
    if parts.is_empty() {
        return tr!(
            "Codex demande plus de permissions",
            "Codex asks for more permissions"
        );
    }
    let what = parts.join(", ");
    tr!("Codex demande {what}", "Codex asks for {what}")
}

#[cfg(test)]
#[path = "codex_tests.rs"]
mod tests;
