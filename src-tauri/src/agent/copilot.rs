//! Copilot's events (`copilot --server --stdio`, docs/PROTOCOL.md, « Copilot SDK ») translated into
//! the agent's conversation: the engine `Copilot` hands them here (`Engine::translate`).
//!
//! Its tools take the names Claude Code gives them (docs/TODO-1.8.md, point 0, « Noms des outils »):
//! `powershell` and `bash` are `Bash`, `view` `Read`, `glob` `Glob`, `grep` and `rg` `Grep`, `create`
//! `Write`, `edit` and `apply_patch` `Edit` (or `Write` for a file added), `web_fetch` `WebFetch`,
//! `task` `Task`, `skill` `Skill`, and a tool of an MCP server (`mcpServerName`)
//! `mcp__<server>__<tool>`. A subagent (`task`) shares its parent's session: its events carry its
//! `agentId`, and are shown under its `Task`. Its tasks come from the `sql` tool's table, read by the
//! process once `session.todos_changed` says they changed (`escouade.todos`).
//!
//! In `manual`, Copilot asks for everything, reads included: Escouade's rules answer the safe
//! requests without a card (reads in the agent's folder; writes there but in the mode Plan; `git
//! add` and `git commit` of its worktree in « Accepter les modifications » and « Auto »).
//!
//! A recorded stream (`tests/fixtures/copilot/`, one `{"dir", "msg"}` a line) is read as it came:
//! what Copilot said (`s2c`), the session its `session.create` opened, the tasks it read.

use super::*;

/// What the translation keeps of Copilot's conversation between its events (not saved).
#[derive(Default)]
pub(crate) struct CopilotState {
    /// The subagents (`agentId`), and the call (`Task`) of each.
    children: HashMap<String, String>,
    /// What each subagent said last, by its call: its report.
    reports: HashMap<String, String>,
    /// The totals of each subagent once over, by its call.
    totals: HashMap<String, Value>,
    /// The reasoning ids seen growing (a reasoning given whole only may be an opaque blob).
    streamed: HashSet<String>,
    /// The turn's usage so far, its subagents' included.
    usage: TurnUsage,
    /// The last event of its own kept by Copilot: where a copy of it starts from.
    last_event: Option<String>,
    /// The turn was stopped (`abort`).
    pub(super) aborted: bool,
    /// What its interface asks the user, as its `permissionRequest` hook told it (« Vue Copilot »),
    /// until its `notification` says it waits for the answer.
    pub(super) asked: Option<String>,
    /// What went wrong in the turn, as Copilot said it.
    error: Option<String>,
    /// The task lists read, numbered.
    todo_updates: u32,
    /// The plan of the mode Plan (`exit_plan_mode.requested`), by its card.
    plans: HashMap<String, Value>,
    /// The turn was stopped by the account's limit (`session.error` of type `quota`): when it
    /// ends, if told (the credits given again).
    limited: Option<Option<i64>>,
    /// When the month's credits are given again, as the last snapshot said it.
    credits_reset: Option<i64>,
    /// The prices of the account's models (`billing.token_prices` of `models.list`, credits per
    /// million tokens), for a call whose cost Copilot does not tell.
    prices: HashMap<String, Value>,
}

#[derive(Debug, Clone, Copy, Default)]
struct TurnUsage {
    input: u64,
    cache: u64,
    output: u64,
    /// What its calls cost, in nano-AIU (`copilotUsage.totalNanoAiu` of each).
    nano: u64,
}

/// Where an event of Copilot belongs.
enum Owner {
    Own,
    Child(String),
    Other,
}

impl AgentRt {
    /// One message of its `copilot --server --stdio`, or of the process itself (`escouade.*`).
    pub(crate) fn copilot_frame(&mut self, f: &Value, fx: &mut Effects) {
        // A recorded stream: what Copilot said, and the session `session.create` opened.
        if let Some(dir) = f.get("dir").and_then(Value::as_str) {
            let msg = &f["msg"];
            if dir == "s2c" {
                if let Some(id) = msg["result"]["sessionId"].as_str() {
                    if msg["result"]["workspacePath"].is_string() {
                        let opened = json!({ "method": "escouade.session", "params": {
                            "sessionId": id, "start": "recorded",
                        } });
                        self.copilot_frame(&opened, fx);
                    }
                } else if let Some(rows) = msg["result"]["rows"].as_array() {
                    let todos = json!({ "method": "escouade.todos", "params": {
                        "todos": { "rows": rows },
                    } });
                    self.copilot_frame(&todos, fx);
                } else if msg["method"].is_string() {
                    self.copilot_frame(msg, fx);
                }
            }
            return;
        }
        let p = &f["params"];
        match f["method"].as_str().unwrap_or_default() {
            "escouade.session" => self.cp_session(p, fx),
            "escouade.mode" => {
                if let Some(mode) = p["mode"].as_str() {
                    self.cp_set_mode(mode, fx);
                }
            }
            "escouade.refused" if self.meta.status.is_active() => {
                self.copilot.error = p["message"].as_str().map(str::to_string);
                self.cp_turn_end(fx);
            }
            "escouade.todos" => self.cp_todos(&p["todos"], fx),
            "escouade.prices" => {
                if let Some(prices) = p["prices"].as_object() {
                    self.copilot.prices = prices.clone().into_iter().collect();
                }
            }
            "session.event" => self.cp_event(&p["event"], fx),
            _ => {}
        }
    }

    /// Another conversation (or another process of it): what was kept of the last one is gone.
    pub(super) fn copilot_new_conversation(&mut self) {
        self.copilot = CopilotState::default();
    }

    fn cp_owner(&self, agent: Option<&str>) -> Owner {
        match agent {
            None => Owner::Own,
            Some(a) => match self.copilot.children.get(a) {
                Some(call) => Owner::Child(call.clone()),
                None => Owner::Other,
            },
        }
    }

    fn cp_set_mode(&mut self, mode: &str, fx: &mut Effects) {
        if self.meta.set_mode(mode) {
            fx.agent_changed = true;
            fx.save = true;
        }
    }

    /// The agent's session, new, resumed or copied.
    fn cp_session(&mut self, p: &Value, fx: &mut Effects) {
        let Some(id) = p["sessionId"].as_str() else {
            return;
        };
        let start = p["start"].as_str().unwrap_or("new");
        if self.meta.session_id.as_deref() != Some(id) {
            let resumed = self.meta.session_id.is_some() && self.meta.fork_of.is_none();
            // Another conversation: what the plan was about is not in its context.
            self.reset_plan(fx);
            if resumed && start == "new" {
                self.notice(
                    "warn",
                    tr!(
                        "La session Copilot n’a pas pu être reprise : nouvelle session (contexte vidé)",
                        "The Copilot session couldn’t be resumed: new session (context cleared)"
                    ),
                    fx,
                );
            }
            self.meta.session_id = Some(id.to_string());
            self.meta.last_entry = None;
            fx.save = true;
        }
        if self.meta.fork_of.take().is_some() {
            self.meta.fork_at = None;
            fx.save = true;
        }
        // Signed in to no account of its own, Copilot takes the GitHub CLI's by itself.
        if p["authType"] == "gh-cli" {
            self.notice(
                "warn",
                tr!(
                    "Copilot utilise le compte du GitHub CLI (gh), faute de connexion à lui : connecte-toi avec copilot login pour choisir son compte.",
                    "Copilot uses the GitHub CLI’s account (gh), as it isn’t signed in itself: sign in with copilot login to choose its account."
                ),
                fx,
            );
        }
        self.saw_init = true;
    }

    /// The tasks of the session (`session.plan.readSqlTodosWithDependencies`), as a task list.
    fn cp_todos(&mut self, todos: &Value, fx: &mut Effects) {
        let Some(rows) = todos["rows"].as_array() else {
            return;
        };
        self.copilot.todo_updates += 1;
        let todos: Vec<Value> = rows
            .iter()
            .map(|t| {
                let status = match t["status"].as_str() {
                    Some("in_progress") => "in_progress",
                    Some("done" | "completed" | "cancelled") => "completed",
                    _ => "pending",
                };
                json!({ "content": t["title"], "status": status, "activeForm": t["title"] })
            })
            .collect();
        let block = json!({
            "id": format!("copilot-todo-{}", self.copilot.todo_updates),
            "name": "TodoWrite",
            "input": { "todos": todos },
        });
        self.plan_tool_use(None, &block, fx);
    }

    /// One event of the session (`session.event`).
    pub(super) fn cp_event(&mut self, e: &Value, fx: &mut Effects) {
        let Some(kind) = e["type"].as_str() else {
            return;
        };
        let d = &e["data"];
        let owner = self.cp_owner(e["agentId"].as_str());
        let parent = match &owner {
            Owner::Child(call) => Some(call.clone()),
            _ => None,
        };
        let own = matches!(owner, Owner::Own);
        if own && e["ephemeral"] != true {
            if let Some(id) = e["id"].as_str() {
                self.copilot.last_event = Some(id.to_string());
            }
        }
        // A subagent's start links its id to its call before its own events come.
        if kind == "subagent.started" {
            if let (Some(agent), Some(call)) = (e["agentId"].as_str(), d["toolCallId"].as_str()) {
                self.copilot
                    .children
                    .insert(agent.to_string(), call.to_string());
            }
            return;
        }
        if matches!(owner, Owner::Other) {
            return;
        }
        // A subagent's message tells the model it runs on: not always the one its start named
        // (its definition's), the first it was sent to may differ.
        if let (Some(call), "assistant.message" | "assistant.usage", Some(model)) =
            (parent.as_deref(), kind, d["model"].as_str())
        {
            self.child_model(call, model, fx);
        }
        match kind {
            "user.message" | "assistant.turn_start" if own => {
                if !self.meta.status.is_active() {
                    // A turn started elsewhere.
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "assistant.message_delta" | "assistant.reasoning_delta" => {
                let id = d["messageId"].as_str().or(d["reasoningId"].as_str());
                let (Some(id), Some(text)) = (id, d["deltaContent"].as_str()) else {
                    return;
                };
                if text.is_empty() {
                    return;
                }
                let item_kind = if kind == "assistant.message_delta" {
                    "text"
                } else {
                    self.copilot.streamed.insert(id.to_string());
                    "thinking"
                };
                if !self.conv.contains(id) {
                    let item = json!({ "kind": item_kind, "id": id, "text": "", "parent": parent, "streaming": true });
                    self.append(item, fx);
                }
                if parent.is_none() {
                    self.set_activity(block_activity(i18n::ui(), item_kind), fx);
                }
                self.push(
                    ConvOp::Delta {
                        id: id.to_string(),
                        text: text.to_string(),
                    },
                    fx,
                );
            }
            "assistant.message" => {
                let (Some(id), text) = (
                    d["messageId"].as_str(),
                    d["content"].as_str().unwrap_or_default(),
                ) else {
                    return;
                };
                if text.trim().is_empty() && !self.conv.contains(id) {
                    return;
                }
                match &parent {
                    None if !text.trim().is_empty() => {
                        if !self.turn_text.is_empty() {
                            self.turn_text.push_str("\n\n");
                        }
                        self.turn_text.push_str(text);
                        self.final_text = text.to_string();
                    }
                    Some(call) if !text.trim().is_empty() => {
                        self.copilot.reports.insert(call.clone(), text.to_string());
                    }
                    _ => {}
                }
                self.cp_whole(id, "text", text, parent, fx);
            }
            "assistant.reasoning" => {
                let (Some(id), text) = (
                    d["reasoningId"].as_str(),
                    d["content"].as_str().unwrap_or_default(),
                ) else {
                    return;
                };
                // Given whole without words that grew, it may be the model's opaque blob.
                let opaque = !self.copilot.streamed.contains(id) && !text.contains(' ');
                if text.trim().is_empty() || opaque {
                    return;
                }
                self.cp_whole(id, "thinking", text, parent, fx);
            }
            "tool.execution_start" => self.cp_tool_start(d, parent, fx),
            "tool.execution_complete" => self.cp_tool_end(d, fx),
            "subagent.completed" | "subagent.failed" => {
                if let Some(call) = d["toolCallId"].as_str() {
                    self.copilot.totals.insert(
                        call.to_string(),
                        json!({
                            "totalTokens": d["totalTokens"], "totalToolUseCount": d["totalToolCalls"],
                        }),
                    );
                }
            }
            "permission.requested" => self.cp_permission(d, parent, fx),
            "user_input.requested" => self.cp_question(d, parent, fx),
            "elicitation.requested" => self.cp_form(d, parent, fx),
            "exit_plan_mode.requested" => self.cp_plan(d, parent, fx),
            "session_limits_exhausted.requested" => self.cp_session_limit(d, parent, fx),
            "permission.completed"
            | "user_input.completed"
            | "elicitation.completed"
            | "exit_plan_mode.completed"
            | "session_limits_exhausted.completed" => self.cp_answered_elsewhere(kind, d, fx),
            "assistant.usage" => {
                let n = |k: &str| d[k].as_u64().unwrap_or(0);
                let cache = n("cacheReadTokens") + n("cacheWriteTokens");
                let nano = crate::engine::copilot_call_nano(d, &self.copilot.prices);
                let u = &mut self.copilot.usage;
                u.input += n("inputTokens").saturating_sub(cache);
                u.cache += cache;
                u.output += n("outputTokens");
                u.nano += nano;
                // The month's credits, as every call tells them.
                let snapshots = &d["quotaSnapshots"];
                if let Some(m) = crate::engine::copilot_credits(snapshots, now_ms()) {
                    self.copilot.credits_reset = m.resets_at;
                    fx.rate = Some(vec![m]);
                }
            }
            "session.usage_info" if own => {
                if let Some(w) = d["tokenLimit"].as_u64() {
                    self.context_window = w;
                }
                self.context_tokens = d["currentTokens"].as_u64().unwrap_or(0);
                fx.agent_changed = true;
            }
            "session.model_change" if own => {
                let model = d["newModel"].as_str().unwrap_or_default();
                if !model.is_empty() && model != "auto" && self.meta.model != model {
                    self.meta.model = model.to_string();
                    fx.agent_changed = true;
                    fx.save = true;
                }
            }
            "session.mode_changed" if own => {
                let mode = match d["newMode"].as_str() {
                    Some("plan") => "plan",
                    Some("autopilot") => "auto",
                    // Out of the mode Plan (or of autopilot): « Demander », unless Escouade's mode
                    // already says how it works.
                    _ if matches!(self.meta.mode.as_str(), "plan" | "auto") => "default",
                    _ => return,
                };
                self.cp_set_mode(mode, fx);
            }
            // Asked (`/compact`, ESC-205) or by Copilot itself: from its start to its end.
            "session.compaction_start" if own => {
                self.notice(
                    "info",
                    tr!("Compaction du contexte…", "Compacting the context…"),
                    fx,
                );
            }
            "session.compaction_complete" if own && d["success"] == true => {
                self.notice("info", tr!("Contexte compacté", "Context compacted"), fx);
            }
            "abort" if own => self.copilot.aborted = true,
            "session.error" if own => {
                // The month's credits spent (`quota`), or a weekly limit: the turn waits.
                let limit = match (d["errorType"].as_str(), d["errorCode"].as_str()) {
                    (Some("quota"), code) if code != Some("billing_not_configured") => {
                        Some(self.copilot.credits_reset)
                    }
                    (Some("rate_limit"), Some(code)) if code.contains("weekly") => Some(None),
                    _ => None,
                };
                if limit.is_some() {
                    self.copilot.limited = limit;
                }
                if self.copilot.error.is_none() {
                    let message = d["message"].as_str().unwrap_or_default().trim();
                    if !message.is_empty() {
                        self.copilot.error = Some(message.to_string());
                    }
                }
            }
            "session.idle" if own => {
                if d["aborted"] == true {
                    self.copilot.aborted = true;
                }
                if self.meta.status.is_active() {
                    self.cp_turn_end(fx);
                }
            }
            _ => {}
        }
    }

    /// A text or a reasoning given whole: its item ends, or is made.
    fn cp_whole(
        &mut self,
        id: &str,
        kind: &str,
        text: &str,
        parent: Option<String>,
        fx: &mut Effects,
    ) {
        let whole = json!({ "text": text, "streaming": false });
        if self.conv.contains(id) {
            self.patch(id, whole, fx);
        } else {
            let mut item = whole;
            item["kind"] = json!(kind);
            item["id"] = json!(id);
            item["parent"] = json!(parent);
            self.append(item, fx);
        }
    }

    /// A tool starts: its call shown, under Claude Code's name. A question's and a plan's have their
    /// card instead, the task list its plan.
    fn cp_tool_start(&mut self, d: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(call) = d["toolCallId"].as_str() else {
            return;
        };
        let tool = d["toolName"].as_str().unwrap_or_default();
        if matches!(tool, "ask_user" | "exit_plan_mode") || self.conv.contains(call) {
            return;
        }
        let (name, input) = tool_of(
            tool,
            &d["arguments"],
            d["mcpServerName"].as_str(),
            &self.meta.cwd,
        );
        if parent.is_none() {
            self.final_text.clear();
            let activity = tool_activity(&name, &input, &self.meta.cwd);
            self.set_activity(activity, fx);
        }
        let item = json!({
            "kind": "tool", "id": call, "name": name, "input": item_input(&name, &input),
            "status": "running", "parent": parent, "ts": now_ms(),
        });
        self.append(item, fx);
        if tool != "sql" {
            self.plan_tool_use(
                parent.as_deref(),
                &json!({ "id": call, "name": name, "input": input }),
                fx,
            );
        }
    }

    /// A tool is over: its result, the files it changed, a subagent's report.
    fn cp_tool_end(&mut self, d: &Value, fx: &mut Effects) {
        let Some(call) = d["toolCallId"].as_str() else {
            return;
        };
        let Some(item) = self.conv.get(call).cloned() else {
            return;
        };
        if item["status"] != "running" {
            return;
        }
        let failed = d["success"] == false
            || d["shellExecution"]["exitCode"]
                .as_i64()
                .is_some_and(|c| c != 0);
        let mut text = if d["success"] == false {
            d["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        } else {
            d["result"]["content"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        };
        let name = item["name"].as_str().unwrap_or_default().to_string();
        let path = item["input"]["file_path"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let edits: Vec<String> = d["fileEdits"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["path"].as_str().map(str::to_string))
            .collect();
        let diff = d["result"]["detailedContent"].as_str().unwrap_or_default();
        let tur = match name.as_str() {
            "Edit" | "Write" | "MultiEdit" if !failed => {
                let file = edits.first().cloned().unwrap_or(path);
                if name == "Write" && !diff.contains("@@") {
                    json!({ "type": "create", "filePath": file, "content": item["input"]["content"], "structuredPatch": [] })
                } else {
                    json!({ "filePath": file, "structuredPatch": codex::hunks_of(diff) })
                }
            }
            "Task" => {
                if let Some(report) = self.copilot.reports.remove(call) {
                    if text.trim().is_empty() {
                        text = report;
                    }
                }
                self.copilot.totals.remove(call).unwrap_or(Value::Null)
            }
            _ => Value::Null,
        };
        // Its own files (the plan of the mode Plan, in its session's folder) are none of the agent's.
        if !failed {
            let cwd = self.meta.cwd.clone();
            for f in edits.iter().filter(|f| inside(f, &cwd)) {
                self.touch(f, fx);
            }
        }
        if self.plan_calls.remove(call) {
            fx.plan_files = true;
        }
        if self.plan_removals.remove(call) && !failed {
            self.plan_apply(fx, PlanState::finish_files);
        }
        self.plan_apply(fx, |plan| {
            plan.on_tool_result(call, failed, &text, &tur, now_ms())
        });
        let result = tool_result(text, failed, &tur);
        self.patch(
            call,
            json!({ "status": if failed { "error" } else { "ok" }, "result": result }),
            fx,
        );
    }

    /// Whether Escouade's rules answer the permission `r` (its `permissionRequest`) without a card:
    /// a read in the agent's folder; a write there, but in the mode Plan; `git add` and `git commit`
    /// of its worktree in « Accepter les modifications » and « Auto ».
    fn cp_safe(&self, r: &Value) -> bool {
        let cwd = &self.meta.cwd;
        let mode = self.meta.mode.as_str();
        // A discussion (ESC-261) writes nothing: its reads alone go through, the rest comes to
        // `discussion_answers`, which refuses a write or a command.
        let discussion = self.meta.discussion.is_some();
        match r["kind"].as_str().unwrap_or_default() {
            "read" => r["path"].as_str().is_some_and(|p| inside(p, cwd)),
            _ if discussion => false,
            "write" => mode != "plan" && r["fileName"].as_str().is_some_and(|p| inside(p, cwd)),
            "shell" => {
                let ids: Vec<&str> = r["commands"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|c| c["identifier"].as_str())
                    .collect();
                matches!(mode, "acceptEdits" | "auto")
                    && crate::engine::codex_git::accepts(
                        r["fullCommandText"].as_str().unwrap_or_default(),
                        &ids,
                        std::path::Path::new(cwd),
                    )
            }
            _ => false,
        }
    }

    /// A permission Copilot asks (`permission.requested`), by its kind: a command with the paths it
    /// touches, a write with its diff, a read, an address, a tool of an MCP server.
    fn cp_permission(&mut self, d: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        if self.conv.contains(&rid) {
            return;
        }
        let r = &d["permissionRequest"];
        let call = r["toolCallId"].as_str().unwrap_or(&rid).to_string();
        if self.cp_safe(r) {
            let allowed = PermissionAnswer::Allow {
                call: call.clone(),
                input: Value::Null,
                resume: None,
            };
            if self
                .proc
                .as_ref()
                .is_some_and(|p| p.answer_permission(&rid, allowed).is_ok())
            {
                log::info!(
                    "agent {}: {} in its folder, accepted",
                    self.meta.id,
                    r["kind"].as_str().unwrap_or_default()
                );
                return;
            }
        }
        let (tool, input, description, patch) = permission_card(r, &self.meta.cwd);
        let reason = r["intention"].as_str().map(str::to_string);
        let mut item = json!({
            "kind": "permission", "id": rid, "toolUseId": call, "toolName": tool,
            "description": description, "input": trim_strings(&input), "reason": reason,
            "canAlways": r["canOfferSessionApproval"] != false, "alwaysScope": "session",
            "defaultNo": false, "decision": null, "ts": now_ms(),
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

    /// A question card, its questions `questions`, made for the request `rid` (of the call `call`).
    fn cp_question_card(
        &mut self,
        rid: String,
        call: String,
        questions: Vec<Value>,
        extra: Value,
        parent: Option<String>,
        fx: &mut Effects,
    ) {
        if self.conv.contains(&rid) {
            return;
        }
        let input = json!({ "questions": questions });
        let mut item = json!({
            "kind": "question", "id": rid, "toolUseId": call, "questions": questions,
            "answers": null, "ts": now_ms(),
        });
        if let Some(o) = extra.as_object() {
            for (k, v) in o {
                item[k] = v.clone();
            }
        }
        if let Some(sub) = parent {
            item["sub"] = sub.into();
        }
        self.append(item, fx);
        let mut view = pending_view(
            &rid,
            "AskUserQuestion",
            &Value::Null,
            &input,
            &self.meta.cwd,
        );
        // A form says what it is for: its message, on the list too.
        view.description = extra["form"]["message"].as_str().map(|m| {
            let mut cut = false;
            let m = capped(m, &mut cut);
            view.cut |= cut;
            m
        });
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

    /// A question of Copilot (`ask_user`, `user_input.requested`): its choices, and a free answer
    /// written in the message field when it takes one.
    fn cp_question(&mut self, d: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        let call = d["toolCallId"].as_str().unwrap_or(&rid).to_string();
        let options: Vec<Value> = d["choices"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c.as_str())
            .map(|c| json!({ "label": c, "description": "" }))
            .collect();
        let q = json!({
            "question": d["question"], "header": "", "multiSelect": false, "options": options,
        });
        self.cp_question_card(rid, call, vec![q], Value::Null, parent, fx);
    }

    /// The session's ceiling of AI credits reached (`session_limits_exhausted.requested`, a session
    /// given `--max-ai-credits`): a card whose choices raise it, lift it or give the call up.
    fn cp_session_limit(&mut self, d: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        let lang = i18n::ui();
        let options: Vec<Value> = crate::engine::copilot_limit_choices(lang, d)
            .into_iter()
            .map(|(label, _)| json!({ "label": label, "description": "" }))
            .collect();
        let q = json!({
            "question": crate::engine::copilot_limit_question(lang, d),
            "header": tr_in!(lang, "Plafond", "Ceiling"),
            "multiSelect": false, "options": options,
        });
        self.cp_question_card(rid.clone(), rid, vec![q], Value::Null, parent, fx);
    }

    /// A form of an MCP server (`elicitation.requested`): a card of its fields, each a question
    /// (its choices for a list or yes or no, a field to fill otherwise).
    fn cp_form(&mut self, d: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        let schema = &d["requestedSchema"];
        let questions = form_questions(schema, i18n::ui());
        let extra = json!({
            "form": { "message": d["message"], "source": d["elicitationSource"] },
        });
        self.cp_question_card(rid.clone(), rid, questions, extra, parent, fx);
    }

    /// The end of the mode Plan (`exit_plan_mode.requested`): the card « Plan à approuver », as for
    /// Claude Code; the turn waits for it.
    fn cp_plan(&mut self, d: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        if self.conv.contains(&rid) {
            return;
        }
        let plan = d["planContent"]
            .as_str()
            .filter(|p| !p.trim().is_empty())
            .or(d["summary"].as_str())
            .unwrap_or_default();
        let input = json!({ "plan": plan });
        self.copilot.plans.insert(rid.clone(), input.clone());
        let mut item = json!({
            "kind": "permission", "id": rid, "toolUseId": rid, "toolName": "ExitPlanMode",
            "input": input, "canAlways": true, "defaultNo": false, "decision": null,
            "ts": now_ms(),
        });
        if let Some(sub) = parent {
            item["sub"] = sub.into();
        }
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
        self.set_status(AgentStatus::Waiting, fx);
        fx.notify = Some(AgentAlert::new(
            NotifyKind::Question,
            notify::permission_body("ExitPlanMode", &input, &self.meta.cwd),
        ));
        fx.agent_changed = true;
    }

    /// A request closed without the card (answered the other way, or by Copilot itself): its card
    /// says so and goes.
    fn cp_answered_elsewhere(&mut self, kind: &str, d: &Value, fx: &mut Effects) {
        let Some(rid) = d["requestId"].as_str() else {
            return;
        };
        let Some(req) = self.take_pending(rid) else {
            return;
        };
        let patch = match kind {
            "permission.completed" => json!({ "decision": match d["result"]["kind"].as_str() {
                Some("approved-for-session") => "always",
                Some(k) if k.starts_with("denied") => "deny",
                Some(k) if k.starts_with("approved") => "allow",
                _ => "deny",
            } }),
            "user_input.completed" => {
                let q = req.input["questions"][0]["question"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                json!({ "answers": { q: d["answer"] } })
            }
            "exit_plan_mode.completed" => json!({
                "decision": if d["approved"] == true { "allow" } else { "deny" },
            }),
            _ => json!({ "cancelled": true }),
        };
        self.patch(&req.item_id, patch, fx);
        if self.pending.is_empty() && self.meta.status == AgentStatus::Waiting {
            self.set_status(AgentStatus::Running, fx);
        }
        fx.agent_changed = true;
    }

    /// The end of the agent's turn (`session.idle`, or a message refused).
    pub(super) fn cp_turn_end(&mut self, fx: &mut Effects) {
        let st = &mut self.copilot;
        let interrupted = std::mem::take(&mut self.interrupted) || std::mem::take(&mut st.aborted);
        let error = std::mem::take(&mut st.error).filter(|_| !interrupted);
        let limit = std::mem::take(&mut st.limited).filter(|_| !interrupted);
        let limited = limit.is_some();
        let failed = error.is_some() || limited;
        if !failed {
            let last = st.last_event.clone();
            self.set_last_entry(last, fx);
        }
        let st = &mut self.copilot;
        let u = std::mem::take(&mut st.usage);
        // Its cost: the nano-AIU of its calls (credits = nano-AIU / 10⁹, a credit $0.01, checked at
        // point 0: their sum is the session's `usage_checkpoint`).
        let cost = u.nano as f64 / 1e11;
        st.totals.clear();
        st.reports.clear();
        let tokens = u.input + u.cache + u.output;
        if tokens > 0 {
            fx.turns.push(TurnRow {
                engine: self.meta.engine,
                model: self.meta.model.clone(),
                input: u.input,
                cache: u.cache,
                output: u.output,
                cost,
            });
            self.meta.tokens += tokens;
            self.meta.cost += cost;
        }
        if let Some(resets) = limit {
            fx.limited = Some(resets);
        } else if self.meta.resume_at.take().is_some() {
            fx.save = true;
            fx.agent_changed = true;
        }
        let reply = std::mem::take(&mut self.final_text);
        let end = if interrupted {
            TurnEnd::Interrupted
        } else if limited {
            TurnEnd::Limited
        } else if let Some(reason) = error.clone() {
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
        // It tells no duration of its own: the time since the turn began (its permissions waited for
        // included, as the agent's « Durée »).
        let took = self.active_since.map(|since| (now_ms() - since).max(0));
        let item = json!({
            "kind": "turn", "id": new_id(), "ts": now_ms(), "durationMs": took,
            "cost": cost, "tokens": tokens, "isError": failed, "interrupted": interrupted,
            "error": error, "limited": limited, "engine": self.meta.engine,
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
            fx.notify = Some(match &error {
                Some(reason) => AgentAlert::new(NotifyKind::Error, notify::error_body(reason)),
                None => AgentAlert::new(NotifyKind::Done, notify::done_body(&reply)),
            });
        }
        fx.agent_changed = true;
        fx.save = true;
        fx.turn_end = Some(end);
    }
}

/// Whether `path` is in the folder `dir` (or is it), whatever its slashes and case.
fn inside(path: &str, dir: &str) -> bool {
    let norm = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_lowercase();
    let (p, d) = (norm(path), norm(dir));
    !d.is_empty() && (p == d || p.starts_with(&format!("{d}/")))
}

/// The card of a permission of Copilot (its `permissionRequest`): the tool Claude Code would have
/// asked for, its input, what the card says beside it, and the diff of a write.
fn permission_card(r: &Value, cwd: &str) -> (String, Value, Option<String>, Value) {
    let list = |k: &str| -> Vec<String> {
        r[k].as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    };
    match r["kind"].as_str().unwrap_or_default() {
        "shell" => {
            let command = r["fullCommandText"].as_str().unwrap_or_default();
            let mut said = Vec::new();
            let paths = list("possiblePaths");
            if !paths.is_empty() {
                let paths = paths.join(", ");
                said.push(tr!("Chemins : {paths}", "Paths: {paths}"));
            }
            let urls = list("possibleUrls");
            if !urls.is_empty() {
                let urls = urls.join(", ");
                said.push(tr!("Adresses : {urls}", "Addresses: {urls}"));
            }
            if r["hasWriteFileRedirection"] == true {
                said.push(tr!(
                    "Écrit dans un fichier par une redirection",
                    "Writes to a file through a redirection"
                ));
            }
            if crate::engine::codex_git::writes_repository(command) {
                said.push(tr!(
                    "git écrit dans le dépôt du projet.",
                    "git writes in the project’s repository."
                ));
            }
            let description = (!said.is_empty()).then(|| said.join("\n"));
            (
                "Bash".into(),
                json!({ "command": command }),
                description,
                Value::Null,
            )
        }
        "write" => {
            let path = r["fileName"].as_str().unwrap_or_default();
            let diff = r["diff"].as_str().unwrap_or_default();
            let hunks = tool_result(
                String::new(),
                false,
                &json!({ "structuredPatch": codex::hunks_of(diff) }),
            );
            let created = diff.contains("--- a/dev/null") || diff.contains("new file mode");
            let mut description = None;
            if !inside(path, cwd) {
                description = Some(tr!(
                    "Hors du dossier de l’agent",
                    "Outside the agent’s folder"
                ));
            }
            (
                if created { "Write" } else { "Edit" }.into(),
                json!({ "file_path": path }),
                description,
                hunks["patch"].clone(),
            )
        }
        "read" => {
            let path = r["path"].as_str().unwrap_or_default();
            let description = (!inside(path, cwd))
                .then(|| tr!("Hors du dossier de l’agent", "Outside the agent’s folder"));
            (
                "Read".into(),
                json!({ "file_path": path }),
                description,
                Value::Null,
            )
        }
        "url" => (
            "WebFetch".into(),
            json!({ "url": r["url"] }),
            None,
            Value::Null,
        ),
        "mcp" => {
            let server = r["serverName"].as_str().unwrap_or_default();
            let tool = mcp_tool(server, r["toolName"].as_str().unwrap_or_default());
            let name = crate::engine::mcp::tool_name(server, &tool);
            let description = if r["readOnly"] == true {
                tr!(
                    "Serveur MCP {server}, outil {tool} : ne fait que lire",
                    "MCP server {server}, tool {tool}: only reads"
                )
            } else {
                tr!(
                    "Serveur MCP {server}, outil {tool} : peut modifier",
                    "MCP server {server}, tool {tool}: may change things"
                )
            };
            (name, r["args"].clone(), Some(description), Value::Null)
        }
        other => (other.to_string(), r.clone(), None, Value::Null),
    }
}

/// An MCP tool's own name, without its server's (`essai-ping` of `essai`, `github-mcp-server/x`).
fn mcp_tool(server: &str, tool: &str) -> String {
    tool.strip_prefix(&format!("{server}-"))
        .or_else(|| tool.strip_prefix(&format!("{server}/")))
        .unwrap_or(tool)
        .to_string()
}

/// The questions of a form's card, one by field of its `schema`: its required fields first, in
/// their order, then the others. A field of a list (`enum`) or yes or no gives its choices; another
/// is a field to fill (`field`: `text` or `number`).
fn form_questions(schema: &Value, lang: Lang) -> Vec<Value> {
    let Some(props) = schema["properties"].as_object() else {
        return Vec::new();
    };
    let required: Vec<&str> = schema["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut names: Vec<&String> = props.keys().collect();
    names.sort_by_key(|n| {
        required
            .iter()
            .position(|r| r == n)
            .unwrap_or(required.len())
    });
    names
        .into_iter()
        .map(|name| {
            let p = &props[name];
            let text = p["title"].as_str().unwrap_or(name);
            let mut q = json!({
                "id": name, "question": text,
                "header": p["description"].as_str().unwrap_or_default(),
                "multiSelect": false, "options": [],
                "required": required.contains(&name.as_str()),
            });
            let choices: Vec<Value> = p["enum"]
                .as_array()
                .or(p["items"]["enum"].as_array())
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(|s| json!({ "label": s, "description": "" })))
                .collect();
            match p["type"].as_str() {
                Some("boolean") => {
                    q["options"] = json!([
                        { "label": crate::engine::copilot_yes(lang), "description": "" },
                        { "label": crate::engine::copilot_no(lang), "description": "" },
                    ]);
                }
                _ if !choices.is_empty() => {
                    q["options"] = Value::Array(choices);
                    q["multiSelect"] = json!(p["type"] == "array");
                }
                Some("number" | "integer") => q["field"] = json!("number"),
                _ => q["field"] = json!("text"),
            }
            q
        })
        .collect()
}

/// The tool Claude Code would have called for Copilot's `tool` (`server`: the MCP server it is of),
/// and its input as Claude Code's.
pub(super) fn tool_of(
    tool: &str,
    args: &Value,
    server: Option<&str>,
    cwd: &str,
) -> (String, Value) {
    if let Some(server) = server.filter(|s| !s.is_empty()) {
        let name = crate::engine::mcp::tool_name(server, &mcp_tool(server, tool));
        return (name, args.clone());
    }
    let path = |k: &str| opencode::absolute(args[k].as_str().unwrap_or_default(), cwd);
    let (name, input) = match tool {
        "powershell" | "bash" => (
            "Bash",
            json!({ "command": args["command"], "description": args["description"] }),
        ),
        "view" => ("Read", json!({ "file_path": path("path") })),
        "glob" => (
            "Glob",
            json!({ "pattern": args["pattern"], "path": args["path"] }),
        ),
        "grep" | "rg" => (
            "Grep",
            json!({ "pattern": args["pattern"], "path": args["path"], "glob": args["glob"] }),
        ),
        "create" => (
            "Write",
            json!({ "file_path": path("path"), "content": args["file_text"] }),
        ),
        "edit" => (
            "Edit",
            json!({
                "file_path": path("path"), "old_string": args["old_str"], "new_string": args["new_str"],
            }),
        ),
        "apply_patch" => {
            let text = args
                .as_str()
                .or(args["input"].as_str())
                .or(args["patch"].as_str())
                .unwrap_or_default();
            let file = opencode::patch_file(text).map(|f| opencode::absolute(f, cwd));
            let added = text.contains("*** Add File: ");
            (
                if added { "Write" } else { "Edit" },
                json!({ "file_path": file }),
            )
        }
        "web_fetch" => ("WebFetch", json!({ "url": args["url"] })),
        "task" => (
            "Task",
            json!({
                "description": args["name"].as_str().or(args["description"].as_str()),
                "prompt": args["prompt"], "subagent_type": args["agent_type"],
            }),
        ),
        "skill" => (
            "Skill",
            json!({ "skill": args["skill"].as_str().or(args["name"].as_str()) }),
        ),
        other => return (other.to_string(), args.clone()),
    };
    let mut input = input;
    if let Some(o) = input.as_object_mut() {
        o.retain(|_, v| !v.is_null());
    }
    (name.to_string(), input)
}

#[cfg(test)]
#[path = "copilot_tests.rs"]
mod tests;
