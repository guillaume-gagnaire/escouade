//! Cursor's updates (`agent acp`, ACP v1, docs/TODO-1.8.md, point 0, « Cursor CLI ») translated into
//! the agent's conversation: the engine `Cursor` hands them here (`Engine::translate`).
//!
//! Its tools take the names Claude Code gives them, by their `kind`: `execute` is `Bash`, `edit`
//! `Edit` (or `Write` for a file added), `read` `Read`, `search` `Grep` (or `Glob`), `fetch`
//! `WebFetch`; its own name is in `rawInput._toolName` (`task` is `Task`), and a tool of an MCP
//! server (`rawInput.providerIdentifier`, `toolName`) `mcp__<server>__<tool>`. A write has its diff
//! in its permission only (`content` of type `diff`): kept for its result.
//!
//! Its extensions: `cursor/ask_question` is a question's card, `cursor/create_plan` the card « Plan à
//! approuver » (its tasks in the plan too), `cursor/update_todos` the task list of the plan, and a
//! subagent (`task`, `subagent_spawned`) speaks under its own session, shown under its `Task`.
//!
//! Escouade's rules answer the safe permissions without a card: the edits in the agent's folder but
//! in the mode Plan; `git add` and `git commit` of its worktree in « Accepter les modifications »;
//! all in « Tout accepter ». A permission « Toujours » (`allow-always`) holds for the agent alone:
//! Cursor writes its rule in the agent's own configuration folder (`engine::cursor`).
//!
//! A recorded stream (`tests/fixtures/cursor/`, one `{"dir", "msg"}` a line) is read as it came:
//! what Cursor said (`s2c`), the session `session/new` opened or `session/load` resumed, the
//! history `session/load` replays left out as the process leaves it out.

use super::*;

/// What the translation keeps of Cursor's conversation between its updates (not saved).
#[derive(Default)]
pub(crate) struct CursorState {
    /// The sessions of the subagents, and the call (`Task`) of each.
    children: HashMap<String, String>,
    /// The text or reasoning being written, by who writes it (`""` the agent, else the call of its
    /// subagent): its item and its kind.
    writing: HashMap<String, (String, &'static str)>,
    /// What each subagent said last, by its call: its report.
    reports: HashMap<String, String>,
    /// The task list as Cursor last gave it (`cursor/update_todos`, merged by id).
    todos: Vec<Value>,
    todo_updates: u32,
    /// The diff of a write, from its permission, by its call: its file, before and after.
    diffs: HashMap<String, (String, Option<String>, String)>,
    /// What went wrong in the turn.
    pub(super) error: Option<String>,
    /// A recorded stream: the `session/load` under way (its request's id), whose replay is left
    /// out.
    loading: Option<Value>,
    /// What the turn spent, as Mistral Vibe told it (`agent/vibe.rs`): Cursor tells nothing.
    pub(super) spent: Option<TurnRow>,
    /// The turn met Mistral Vibe's rate limit: when it goes on (`agent/vibe.rs`).
    pub(super) limited_until: Option<i64>,
}

/// A call's id as Escouade keeps it: Cursor's has a line break in it (`call-…-0\nfc_…_0`).
fn call_id(id: &str) -> String {
    id.replace(['\n', '\r'], "~")
}

impl AgentRt {
    /// One message of its `agent acp`, or of the process itself (`escouade.*`).
    pub(crate) fn cursor_frame(&mut self, f: &Value, fx: &mut Effects) {
        if let Some(dir) = f.get("dir").and_then(Value::as_str) {
            self.cursor_recorded(dir, &f["msg"], fx);
            return;
        }
        let p = &f["params"];
        match f["method"].as_str().unwrap_or_default() {
            "escouade.session" => self.cu_session(p, fx),
            "escouade.mode" => {
                if let Some(mode) = p["mode"].as_str() {
                    self.cu_set_mode(mode, fx);
                }
            }
            "escouade.prompt_start" => {
                if !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "escouade.prompt_end" => {
                if let Some(e) = p["error"].as_str() {
                    self.cursor.error = Some(e.to_string());
                }
                if p["stopReason"] == "cancelled" {
                    self.interrupted = true;
                }
                if self.meta.status.is_active() {
                    self.cu_turn_end(fx);
                }
            }
            "session/update" => self.cu_update(p, fx),
            "session/request_permission" => {
                if let Some(rid) = f["id"].as_str() {
                    self.cu_permission(rid, p, fx);
                }
            }
            "cursor/ask_question" => {
                if let Some(rid) = f["id"].as_str() {
                    self.cu_question(rid, p, fx);
                }
            }
            "cursor/create_plan" => {
                if let Some(rid) = f["id"].as_str() {
                    self.cu_plan(rid, p, fx);
                }
            }
            "cursor/update_todos" => self.cu_todos(&p["todos"], p["merge"] == true, fx),
            _ => {}
        }
    }

    /// A line of a recorded stream, read as the process would pass it on.
    fn cursor_recorded(&mut self, dir: &str, msg: &Value, fx: &mut Effects) {
        match dir {
            "c2s" if msg["method"] == "session/load" => {
                self.cursor.loading = Some(msg["id"].clone());
                let opened = json!({ "method": "escouade.session", "params": {
                    "sessionId": msg["params"]["sessionId"], "start": "recorded",
                } });
                self.cursor_frame(&opened, fx);
            }
            // The trial's answer to a permission: its card says it.
            "c2s" if msg["method"].is_null() && msg["result"]["outcome"].is_object() => {
                let rid = format!("recorded-{}", msg["id"]);
                if let Some(req) = self.take_pending(&rid) {
                    let decision = match msg["result"]["outcome"]["optionId"].as_str() {
                        Some(o) if o.starts_with("allow-always") => "always",
                        Some(o) if o.starts_with("allow") => "allow",
                        _ => "deny",
                    };
                    self.patch(&req.item_id, json!({ "decision": decision }), fx);
                    self.after_answer(fx);
                }
            }
            "c2s" if msg["method"] == "session/prompt" => {
                let text = msg["params"]["prompt"][0]["text"]
                    .as_str()
                    .unwrap_or_default();
                let id = new_id();
                self.push_user_from(&id, text, 0, &[], None, fx);
            }
            "s2c" => {
                let loading = self.cursor.loading.clone();
                if loading.is_some() && msg["method"].is_null() && msg["id"] == loading.unwrap() {
                    self.cursor.loading = None;
                    return;
                }
                if self.cursor.loading.is_some() {
                    return;
                }
                if let Some(id) = msg["result"]["sessionId"].as_str() {
                    let opened = json!({ "method": "escouade.session", "params": {
                        "sessionId": id, "start": "recorded",
                    } });
                    self.cursor_frame(&opened, fx);
                } else if msg["result"]["stopReason"].is_string() {
                    let end = json!({ "method": "escouade.prompt_end", "params": msg["result"] });
                    self.cursor_frame(&end, fx);
                } else if msg["method"].is_string() {
                    let mut m = msg.clone();
                    if !msg["id"].is_null() {
                        m["id"] = json!(format!("recorded-{}", msg["id"]));
                    }
                    self.cursor_frame(&m, fx);
                }
            }
            _ => {}
        }
    }

    /// Another conversation (or another process of it): what was kept of the last one is gone.
    pub(super) fn cursor_new_conversation(&mut self) {
        self.cursor = CursorState::default();
    }

    fn cu_set_mode(&mut self, mode: &str, fx: &mut Effects) {
        if self.meta.set_mode(mode) {
            fx.agent_changed = true;
            fx.save = true;
        }
    }

    /// The agent's session, new or resumed.
    fn cu_session(&mut self, p: &Value, fx: &mut Effects) {
        let Some(id) = p["sessionId"].as_str() else {
            return;
        };
        let start = p["start"].as_str().unwrap_or("new");
        if self.meta.session_id.as_deref() != Some(id) {
            let resumed = self.meta.session_id.is_some();
            self.reset_plan(fx);
            if resumed && start == "new" {
                // Mistral Vibe and Antigravity CLI, whose translation this is too (`agent/vibe.rs`,
                // `agent/agy.rs`), say their own name.
                let who = crate::engine::of(&self.meta).speaker();
                self.notice(
                    "warn",
                    tr!(
                        "La session {who} n’a pas pu être reprise : nouvelle session (contexte vidé)",
                        "The {who} session couldn’t be resumed: new session (context cleared)"
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
        self.saw_init = true;
    }

    /// Who an update is of: the agent (`Some(None)`), one of its subagents (its call), or none of
    /// them.
    fn cu_owner(&self, session: Option<&str>) -> Option<Option<String>> {
        let session = session?;
        if self.meta.session_id.as_deref() == Some(session) {
            return Some(None);
        }
        self.cursor.children.get(session).map(|c| Some(c.clone()))
    }

    /// One update of a session (`session/update`).
    fn cu_update(&mut self, p: &Value, fx: &mut Effects) {
        let u = &p["update"];
        let kind = u["sessionUpdate"].as_str().unwrap_or_default();
        // A subagent's start links its session to its call before its own updates come.
        if kind == "subagent_spawned" {
            let call = u["_meta"]["cursor"]["toolCallId"].as_str().map(call_id);
            if let (Some(child), Some(call)) = (u["subagentSessionId"].as_str(), call) {
                self.cursor.children.insert(child.to_string(), call.clone());
                if let Some(model) = u["_meta"]["cursor"]["model"].as_str() {
                    self.child_model(&call, model, fx);
                }
            }
            return;
        }
        let Some(parent) = self.cu_owner(p["sessionId"].as_str()) else {
            return;
        };
        match kind {
            "agent_message_chunk" | "agent_thought_chunk" => {
                let Some(text) = u["content"]["text"].as_str().filter(|t| !t.is_empty()) else {
                    return;
                };
                let item_kind = if kind == "agent_message_chunk" {
                    "text"
                } else {
                    "thinking"
                };
                self.cu_write(parent, item_kind, text, fx);
            }
            "tool_call" => self.cu_tool_start(u, parent, fx),
            "tool_call_update" => self.cu_tool_update(u, parent, fx),
            "plan" if parent.is_none() => {
                let todos: Vec<Value> = u["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, e)| json!({ "id": i.to_string(), "content": e["content"], "status": e["status"] }))
                    .collect();
                self.cu_todos(&Value::Array(todos), false, fx);
            }
            "current_mode_update" if parent.is_none() => {
                let mode = match u["currentModeId"].as_str() {
                    Some("plan") => "plan",
                    // Out of the mode Plan: « Demander », unless Escouade's mode says how it works.
                    _ if self.meta.mode == "plan" => "default",
                    _ => return,
                };
                self.cu_set_mode(mode, fx);
            }
            _ => {}
        }
    }

    /// A piece of text or reasoning of `parent` (None: the agent): into the item being written, or
    /// a new one.
    pub(super) fn cu_write(
        &mut self,
        parent: Option<String>,
        kind: &'static str,
        text: &str,
        fx: &mut Effects,
    ) {
        let who = parent.clone().unwrap_or_default();
        let current = self
            .cursor
            .writing
            .get(&who)
            .filter(|(id, k)| *k == kind && self.conv.contains(id))
            .map(|(id, _)| id.clone());
        let id = match current {
            Some(id) => id,
            None => {
                self.cu_close(&who, fx);
                let id = new_id();
                let item = json!({ "kind": kind, "id": id, "text": "", "parent": parent, "streaming": true });
                self.append(item, fx);
                self.cursor.writing.insert(who, (id.clone(), kind));
                id
            }
        };
        if parent.is_none() {
            self.set_activity(block_activity(i18n::ui(), kind), fx);
        }
        self.push(
            ConvOp::Delta {
                id,
                text: text.to_string(),
            },
            fx,
        );
    }

    /// The text or reasoning `who` was writing is whole: its item ends, the agent's text kept as
    /// its reply so far, a subagent's as its report.
    pub(super) fn cu_close(&mut self, who: &str, fx: &mut Effects) {
        let Some((id, kind)) = self.cursor.writing.remove(who) else {
            return;
        };
        let Some(text) = self
            .conv
            .get(&id)
            .and_then(|i| i["text"].as_str().map(str::to_string))
        else {
            return;
        };
        if kind == "text" && !text.trim().is_empty() {
            if who.is_empty() {
                if !self.turn_text.is_empty() {
                    self.turn_text.push_str("\n\n");
                }
                self.turn_text.push_str(&text);
                self.final_text = text.clone();
            } else {
                self.cursor.reports.insert(who.to_string(), text.clone());
            }
        }
        self.patch(&id, json!({ "text": text, "streaming": false }), fx);
    }

    /// A tool starts: its call shown, under Claude Code's name. A question's and a plan's have their
    /// card instead, the task list its plan.
    fn cu_tool_start(&mut self, u: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(call) = u["toolCallId"].as_str().map(call_id) else {
            return;
        };
        let raw = &u["rawInput"];
        let own = raw["_toolName"].as_str().unwrap_or_default();
        if matches!(own, "updateTodos" | "createPlan" | "askQuestion")
            || u["kind"] == "think"
            || self.conv.contains(&call)
        {
            return;
        }
        self.cu_close(parent.as_deref().unwrap_or_default(), fx);
        let (name, input) = tool_of(u, &self.meta.cwd);
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
        self.plan_tool_use(
            parent.as_deref(),
            &json!({ "id": call, "name": name, "input": input }),
            fx,
        );
    }

    /// What Cursor says of a call as it goes: its file or command once known, then its end.
    fn cu_tool_update(&mut self, u: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(call) = u["toolCallId"].as_str().map(call_id) else {
            return;
        };
        let Some(item) = self.conv.get(&call).cloned() else {
            return;
        };
        if item["status"] != "running" {
            return;
        }
        let name = item["name"].as_str().unwrap_or_default().to_string();
        // Its input told later (an edit's file).
        if u["rawInput"].is_object() || u["locations"].is_array() {
            let (_, input) = tool_of(
                &json!({
                    "kind": u.get("kind").cloned().unwrap_or_else(|| kind_of(&name).into()),
                    "title": u["title"], "rawInput": u["rawInput"], "locations": u["locations"],
                }),
                &self.meta.cwd,
            );
            let told = input
                .as_object()
                .is_some_and(|o| o.values().any(|v| !v.is_null()));
            if told && input != item["input"] {
                self.patch(&call, json!({ "input": item_input(&name, &input) }), fx);
            }
        }
        let status = u["status"].as_str().unwrap_or_default();
        if !matches!(status, "completed" | "failed") {
            return;
        }
        let out = &u["rawOutput"];
        let mut failed = status == "failed" || out["exitCode"].as_i64().is_some_and(|c| c != 0);
        // A write done ends with its diff (or had it in the permission it asked): one without wrote
        // nothing (a hook refused it, say), and Cursor tells no more (ESC-43).
        let unwritten = !failed
            && matches!(name.as_str(), "Edit" | "Write")
            && diff_of(&u["content"]).is_none()
            && !self.cursor.diffs.contains_key(&call);
        if unwritten {
            failed = true;
            self.cursor.diffs.remove(&call);
        }
        let mut text = out["content"]
            .as_str()
            .or(out["error"].as_str())
            .unwrap_or_default()
            .to_string();
        if text.is_empty() {
            text = content_text(&u["content"]);
        }
        if unwritten {
            text = crate::engine::cursor::write_not_done(i18n::ui());
        }
        // A write done says its diff as it ends (`content` of type `diff`: Cursor asks for none of
        // them by default, tried on 2026.10.22), else the permission it asked had it.
        if let Some(d) = diff_of(&u["content"]) {
            self.cursor.diffs.insert(call.clone(), d);
        }
        let input = self
            .conv
            .get(&call)
            .map(|i| i["input"].clone())
            .unwrap_or_default();
        let path = input["file_path"].as_str().unwrap_or_default().to_string();
        let tur = match name.as_str() {
            "Bash" => json!({
                "stdout": out["stdout"].as_str().unwrap_or_default(),
                "stderr": out["stderr"].as_str().unwrap_or_default(),
            }),
            "Edit" | "Write" if !failed => match self.cursor.diffs.remove(&call) {
                Some((file, old, new)) => {
                    // A file added: Claude Code's `Write`.
                    if old.is_none() && name == "Edit" {
                        self.patch(&call, json!({ "name": "Write" }), fx);
                    }
                    json!({
                        "filePath": file, "structuredPatch": line_hunks(old.as_deref().unwrap_or_default(), &new),
                    })
                }
                None => json!({ "filePath": path, "structuredPatch": [] }),
            },
            "Task" => {
                self.cu_close(&call, fx);
                if let Some(report) = self.cursor.reports.remove(&call) {
                    if text.trim().is_empty() {
                        text = report;
                    }
                }
                Value::Null
            }
            _ => Value::Null,
        };
        if !failed && matches!(name.as_str(), "Edit" | "Write") {
            let file = tur["filePath"].as_str().unwrap_or(&path).to_string();
            if !file.is_empty() && inside(&file, &self.meta.cwd) {
                self.touch(&file, fx);
            }
        }
        if self.plan_calls.remove(&call) {
            fx.plan_files = true;
        }
        if self.plan_removals.remove(&call) && !failed {
            self.plan_apply(fx, PlanState::finish_files);
        }
        self.plan_apply(fx, |plan| {
            plan.on_tool_result(&call, failed, &text, &tur, now_ms())
        });
        let result = tool_result(text, failed, &tur);
        self.patch(
            &call,
            json!({ "status": if failed { "error" } else { "ok" }, "result": result }),
            fx,
        );
        let _ = parent;
    }

    /// Whether Escouade's rules answer a permission of the call `call` (of `kind`, its tool `name`
    /// and `input`, the files `paths` it writes) without a card.
    fn cu_safe(&self, kind: &str, input: &Value, paths: &[String]) -> bool {
        let cwd = &self.meta.cwd;
        match self.meta.mode.as_str() {
            "bypassPermissions" => true,
            "plan" => false,
            mode => match kind {
                "edit" | "delete" => !paths.is_empty() && paths.iter().all(|p| inside(p, cwd)),
                "execute" => {
                    mode == "acceptEdits"
                        && crate::engine::codex_git::accepts(
                            input["command"].as_str().unwrap_or_default(),
                            &[],
                            std::path::Path::new(cwd),
                        )
                }
                _ => false,
            },
        }
    }

    /// A permission Cursor asks (`session/request_permission`): a command, a write with its diff,
    /// a tool of an MCP server, an address.
    fn cu_permission(&mut self, rid: &str, p: &Value, fx: &mut Effects) {
        if self.conv.contains(rid) {
            return;
        }
        let tc = &p["toolCall"];
        let call = tc["toolCallId"]
            .as_str()
            .map(call_id)
            .unwrap_or_else(|| rid.to_string());
        let parent = self.cu_owner(p["sessionId"].as_str()).flatten();
        let kind = tc["kind"].as_str().unwrap_or_default().to_string();
        // Its diff, kept for the result of its call.
        let diffs: Vec<(String, Option<String>, String)> = tc["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|c| c["type"] == "diff")
            .filter_map(|c| {
                Some((
                    c["path"].as_str()?.to_string(),
                    c["oldText"].as_str().map(str::to_string),
                    c["newText"].as_str().unwrap_or_default().to_string(),
                ))
            })
            .collect();
        if let Some(d) = diffs.first() {
            self.cursor.diffs.insert(call.clone(), d.clone());
        }
        let known = self.conv.get(&call).cloned();
        let (mut tool, mut input) = match &known {
            Some(i) => (
                i["name"].as_str().unwrap_or_default().to_string(),
                i["input"].clone(),
            ),
            None => tool_of(tc, &self.meta.cwd),
        };
        let paths: Vec<String> = diffs
            .iter()
            .map(|d| d.0.clone())
            .chain(input["file_path"].as_str().map(str::to_string))
            .collect();
        if let Some((path, old, _)) = diffs.first() {
            tool = if old.as_deref().unwrap_or_default().is_empty() {
                "Write"
            } else {
                "Edit"
            }
            .into();
            input = json!({ "file_path": path });
            if known.as_ref().is_some_and(|i| i["name"] != tool.as_str()) {
                self.patch(&call, json!({ "name": tool, "input": input }), fx);
            }
        }
        if self.cu_safe(&kind, &input, &paths) {
            let allowed = PermissionAnswer::Allow {
                call: call.clone(),
                input: Value::Null,
                resume: None,
            };
            if self
                .proc
                .as_ref()
                .is_some_and(|proc| proc.answer_permission(rid, allowed).is_ok())
            {
                log::info!(
                    "agent {}: {kind} accepted by Escouade's rules",
                    self.meta.id
                );
                return;
            }
        }
        let mut said: Vec<String> = Vec::new();
        if paths.iter().any(|f| !inside(f, &self.meta.cwd)) {
            said.push(tr!(
                "Hors du dossier de l’agent",
                "Outside the agent’s folder"
            ));
        }
        if tool == "Bash"
            && crate::engine::codex_git::writes_repository(
                input["command"].as_str().unwrap_or_default(),
            )
        {
            said.push(tr!(
                "git écrit dans le dépôt du projet.",
                "git writes in the project’s repository."
            ));
        }
        if let Some((server, mcp_tool)) = mcp_of(tc, known.as_ref()) {
            said.push(tr!(
                "Serveur MCP {server}, outil {mcp_tool}",
                "MCP server {server}, tool {mcp_tool}"
            ));
        }
        // Why Cursor asks (« Not in allowlist: git status, git log »).
        let reason = Some(content_text(&tc["content"])).filter(|t| !t.trim().is_empty());
        let description = (!said.is_empty()).then(|| said.join("\n"));
        let mut item = json!({
            "kind": "permission", "id": rid, "toolUseId": call, "toolName": tool,
            "description": description, "input": trim_strings(&input), "reason": reason,
            "canAlways": p["options"].as_array().is_none_or(|o| o.iter().any(|x| x["kind"] == "allow_always")),
            "alwaysScope": "agent", "defaultNo": false, "decision": null, "ts": now_ms(),
        });
        if let Some((path, old, new)) = diffs.first() {
            let hunks = line_hunks(old.as_deref().unwrap_or_default(), new);
            let shown = tool_result(
                String::new(),
                false,
                &json!({ "filePath": path, "structuredPatch": hunks }),
            );
            item["patch"] = shown["patch"].clone();
        }
        if let Some(sub) = parent {
            item["sub"] = sub.into();
        }
        let req = json!({ "description": description, "decision_reason": reason });
        let view = pending_view(rid, &tool, &req, &input, &self.meta.cwd).always_as(&item);
        self.append(item, fx);
        self.add_pending(
            PendingReq {
                item_id: rid.to_string(),
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

    /// A question of Cursor (`cursor/ask_question`): its questions, each with its choices.
    fn cu_question(&mut self, rid: &str, p: &Value, fx: &mut Effects) {
        if self.conv.contains(rid) {
            return;
        }
        let call = p["toolCallId"]
            .as_str()
            .map(call_id)
            .unwrap_or_else(|| rid.to_string());
        let header = p["title"].as_str().unwrap_or_default();
        let questions: Vec<Value> = p["questions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|q| {
                let options: Vec<Value> = q["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|o| json!({ "label": o["label"], "description": "" }))
                    .collect();
                json!({
                    "question": q["prompt"], "header": header,
                    "multiSelect": q["allowMultiple"] == true, "options": options,
                })
            })
            .collect();
        let input = json!({ "questions": questions });
        let item = json!({
            "kind": "question", "id": rid, "toolUseId": call, "questions": questions,
            "answers": null, "ts": now_ms(),
        });
        self.append(item, fx);
        let view = pending_view(rid, "AskUserQuestion", &Value::Null, &input, &self.meta.cwd);
        self.add_pending(
            PendingReq {
                item_id: rid.to_string(),
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

    /// A plan (`cursor/create_plan`): the card « Plan à approuver », as for Claude Code, and its
    /// tasks in the plan; the turn waits for it.
    fn cu_plan(&mut self, rid: &str, p: &Value, fx: &mut Effects) {
        if self.conv.contains(rid) {
            return;
        }
        let text = [
            p["name"].as_str().map(|n| format!("# {n}")),
            p["overview"].as_str().map(str::to_string),
            p["plan"].as_str().map(str::to_string),
        ]
        .into_iter()
        .flatten()
        .filter(|t| !t.trim().is_empty() && t != "# ")
        .collect::<Vec<_>>()
        .join("\n\n");
        if p["todos"].as_array().is_some_and(|t| !t.is_empty()) {
            self.cu_todos(&p["todos"], false, fx);
        }
        let input = json!({ "plan": text });
        let item = json!({
            "kind": "permission", "id": rid, "toolUseId": rid, "toolName": "ExitPlanMode",
            "input": input, "canAlways": true, "defaultNo": false, "decision": null,
            "ts": now_ms(),
        });
        self.append(item, fx);
        let view = pending_view(rid, "ExitPlanMode", &Value::Null, &input, &self.meta.cwd);
        self.add_pending(
            PendingReq {
                item_id: rid.to_string(),
                tool_use_id: rid.to_string(),
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

    /// The task list (`cursor/update_todos`, `merge`: the tasks it names changed, the others kept),
    /// as Claude Code's `TodoWrite` gives it to the plan.
    fn cu_todos(&mut self, todos: &Value, merge: bool, fx: &mut Effects) {
        let Some(list) = todos.as_array() else {
            return;
        };
        if !merge {
            self.cursor.todos.clear();
        }
        for t in list {
            match self
                .cursor
                .todos
                .iter_mut()
                .find(|k| k["id"] == t["id"] && !t["id"].is_null())
            {
                Some(k) => {
                    for (key, v) in t.as_object().into_iter().flatten() {
                        k[key] = v.clone();
                    }
                }
                None => self.cursor.todos.push(t.clone()),
            }
        }
        self.cursor.todo_updates += 1;
        let todos: Vec<Value> = self
            .cursor
            .todos
            .iter()
            .filter(|t| t["status"] != "cancelled")
            .map(|t| {
                let status = match t["status"].as_str() {
                    Some("in_progress") => "in_progress",
                    Some("completed") => "completed",
                    _ => "pending",
                };
                json!({ "content": t["content"], "status": status, "activeForm": t["content"] })
            })
            .collect();
        let block = json!({
            "id": format!("cursor-todo-{}", self.cursor.todo_updates),
            "name": "TodoWrite",
            "input": { "todos": todos },
        });
        self.plan_tool_use(None, &block, fx);
    }

    /// The end of the agent's turn (the answer to its prompt): a turn Cursor refused (its limit, a
    /// model out of the account's plan, a sign-in or key refused: `refusal`) ends in error with what
    /// to do, « limité » at the account's limit.
    pub(super) fn cu_turn_end(&mut self, fx: &mut Effects) {
        for who in self.cursor.writing.keys().cloned().collect::<Vec<_>>() {
            self.cu_close(&who, fx);
        }
        let interrupted = std::mem::take(&mut self.interrupted);
        // Cursor's refusals are its texts: none for Mistral Vibe nor Antigravity CLI, whose errors
        // are told apart.
        let refused = crate::engine::cursor::refusal(&self.turn_text)
            .filter(|_| !interrupted && self.meta.engine == crate::model::EngineId::Cursor);
        let mut error = std::mem::take(&mut self.cursor.error).filter(|_| !interrupted);
        let mut limited = false;
        // Mistral Vibe's rate limit: « limité », its resume planned.
        if let Some(at) = self.cursor.limited_until.take().filter(|_| !interrupted) {
            limited = true;
            fx.limited = Some(Some(at));
        }
        if let (Some(refusal), None) = (refused, &error) {
            let by_key =
                self.meta.account != crate::accounts::principal_id(crate::model::EngineId::Cursor);
            limited = crate::engine::cursor::at_limit(refusal);
            error = Some(crate::engine::cursor::refusal_error(
                i18n::ui(),
                refusal,
                &self.meta.model,
                by_key,
            ));
        }
        let failed = error.is_some();
        self.cursor.reports.clear();
        let spent = self.cursor.spent.take();
        let (cost, tokens) = spent
            .as_ref()
            .map_or((0.0, 0), |r| (r.cost, r.input + r.cache + r.output));
        if let Some(row) = spent.filter(|_| tokens > 0) {
            self.meta.tokens += tokens;
            self.meta.cost += cost;
            fx.turns.push(row);
        }
        // A resume planned is over (the core plans the next one, `fx.limited`).
        if self.meta.resume_at.take().is_some() {
            fx.save = true;
            fx.agent_changed = true;
        }
        let reply = std::mem::take(&mut self.final_text);
        let end = if interrupted {
            TurnEnd::Interrupted
        } else if limited {
            // No reset Escouade knows of: no resume planned (`fx.limited`), the card says it.
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
        let took = self.active_since.map(|since| (now_ms() - since).max(0));
        let item = json!({
            "kind": "turn", "id": new_id(), "ts": now_ms(), "durationMs": took,
            // Cursor's ACP tells neither: nothing is shown of them (`CostKind::None`); Vibe's both.
            "cost": cost, "tokens": tokens, "isError": failed, "interrupted": interrupted, "error": error, "limited": limited,
            "engine": self.meta.engine,
        });
        self.append(item, fx);
        if !self.pending.is_empty() {
            self.clear_pending(fx);
        }
        // The next message waiting goes now (the process sends it): its turn starts.
        if self.queued > 0 {
            self.queued -= 1;
            if let Some(id) = self
                .conv
                .ids_where(|v| v["kind"] == "user" && v["queued"] == true)
                .into_iter()
                .next()
            {
                self.patch(&id, json!({ "queued": false }), fx);
            }
            self.set_status(AgentStatus::Done, fx);
            self.set_status(AgentStatus::Running, fx);
            fx.agent_changed = true;
            fx.save = true;
            fx.turn_end = Some(end);
            return;
        }
        self.set_status(
            if failed {
                AgentStatus::Error
            } else {
                AgentStatus::Done
            },
            fx,
        );
        if !interrupted {
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

/// The first diff of a call's `content` (`{type: "diff", path, oldText, newText}`): its file, before
/// (none for a file added) and after.
fn diff_of(content: &Value) -> Option<(String, Option<String>, String)> {
    content
        .as_array()?
        .iter()
        .find(|c| c["type"] == "diff")
        .and_then(|c| {
            Some((
                c["path"].as_str()?.to_string(),
                c["oldText"].as_str().map(str::to_string),
                c["newText"].as_str().unwrap_or_default().to_string(),
            ))
        })
}

/// The words of a call's `content` (its `content` blocks of text).
fn content_text(content: &Value) -> String {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c["content"]["text"].as_str().or(c["text"].as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `kind` of Cursor a tool of Claude Code's name stands for (`tool_of` the other way).
fn kind_of(name: &str) -> &'static str {
    match name {
        "Bash" => "execute",
        "Edit" | "Write" => "edit",
        "Read" => "read",
        "Grep" | "Glob" => "search",
        "WebFetch" => "fetch",
        _ => "other",
    }
}

/// The MCP server and tool of a call (`rawInput.providerIdentifier`, `toolName`), from the call
/// `tc` or its item `known`.
fn mcp_of(tc: &Value, known: Option<&Value>) -> Option<(String, String)> {
    if let Some(name) = known.and_then(|i| i["name"].as_str()) {
        let rest = name.strip_prefix("mcp__")?;
        let (server, tool) = rest.split_once("__")?;
        return Some((server.to_string(), tool.to_string()));
    }
    let raw = &tc["rawInput"];
    Some((
        raw["providerIdentifier"].as_str()?.to_string(),
        raw["toolName"].as_str()?.to_string(),
    ))
}

/// The tool Claude Code would have called for Cursor's call `u` (`tool_call`: its `kind`, `title`,
/// `rawInput`, `locations`), and its input as Claude Code's.
pub(super) fn tool_of(u: &Value, cwd: &str) -> (String, Value) {
    let raw = &u["rawInput"];
    if let (Some(server), Some(tool)) =
        (raw["providerIdentifier"].as_str(), raw["toolName"].as_str())
    {
        let name = crate::engine::mcp::tool_name(server, tool);
        return (name, raw["args"].clone());
    }
    let located = u["locations"][0]["path"].as_str();
    let path = raw["path"]
        .as_str()
        .or(raw["filePath"].as_str())
        .or(located)
        .map(|p| opencode::absolute(p, cwd));
    let title = u["title"].as_str().unwrap_or_default();
    let (name, input) = match (raw["_toolName"].as_str(), u["kind"].as_str()) {
        (Some("task"), _) => (
            "Task",
            json!({
                "description": raw["description"].as_str().unwrap_or(title.trim_start_matches("Task: ")),
                "prompt": raw["prompt"], "subagent_type": raw["subagentType"],
            }),
        ),
        (_, Some("execute")) => {
            let command = raw["command"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| title.trim_matches('`').to_string());
            ("Bash", json!({ "command": command }))
        }
        (_, Some("edit")) => ("Edit", json!({ "file_path": path })),
        (_, Some("delete")) => ("Delete", json!({ "file_path": path })),
        (_, Some("read")) => ("Read", json!({ "file_path": path })),
        (_, Some("search")) => {
            let glob = raw["globPattern"].as_str();
            if glob.is_some() {
                (
                    "Glob",
                    json!({ "pattern": glob, "path": raw["targetDirectory"] }),
                )
            } else {
                (
                    "Grep",
                    json!({
                        "pattern": raw["pattern"].as_str().or(raw["query"].as_str()).unwrap_or(title),
                        "path": raw["path"],
                    }),
                )
            }
        }
        (_, Some("fetch")) => ("WebFetch", json!({ "url": raw["url"] })),
        (own, _) => {
            let name = own.unwrap_or(title).to_string();
            let mut input = raw.clone();
            if let Some(o) = input.as_object_mut() {
                o.remove("_toolName");
            }
            return (name, input);
        }
    };
    let mut input = input;
    if let Some(o) = input.as_object_mut() {
        o.retain(|_, v| !v.is_null());
    }
    (name.to_string(), input)
}

/// The lines changed between `old` and `new`, as Claude Code's `structuredPatch` gives them: one
/// hunk around what changed (its first and last lines that differ), with three lines of context.
pub(crate) fn line_hunks(old: &str, new: &str) -> Value {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    if a == b {
        return json!([]);
    }
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let tail = a[head..]
        .iter()
        .rev()
        .zip(b[head..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let context = 3;
    let from = head.saturating_sub(context);
    let mut lines: Vec<Value> = a[from..head]
        .iter()
        .map(|l| json!(format!(" {l}")))
        .collect();
    lines.extend(
        a[head..a.len() - tail]
            .iter()
            .map(|l| json!(format!("-{l}"))),
    );
    lines.extend(
        b[head..b.len() - tail]
            .iter()
            .map(|l| json!(format!("+{l}"))),
    );
    let after = (a.len() - tail + context).min(a.len());
    lines.extend(
        a[a.len() - tail..after]
            .iter()
            .map(|l| json!(format!(" {l}"))),
    );
    let start = |n: usize| if n == 0 { 0 } else { from + 1 };
    json!([{ "oldStart": start(a.len()), "newStart": start(b.len()), "lines": lines }])
}

#[cfg(test)]
#[path = "cursor_tests.rs"]
mod tests;
