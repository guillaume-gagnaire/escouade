//! OpenCode's events (`opencode serve`, docs/PROTOCOL.md, « OpenCode serve ») translated into the
//! agent's conversation: the engine `Opencode` hands them here (`Engine::translate`).
//!
//! Its tools take the names Claude Code gives them (docs/TODO-1.8.md, « Un moteur par agent »):
//! `bash` is `Bash`, `edit` `Edit`, `write` `Write`, `read` `Read`, `grep` `Grep`, `glob` `Glob`,
//! `list` `LS`, `patch` `Edit`, `webfetch` `WebFetch`, `todowrite` `TodoWrite`, `task` `Task`, and a
//! tool `escouade_<tool>` of Escouade's server `mcp__escouade__<tool>`. A subagent (`task`) has a
//! session of its own, whose events come on the same stream: they are told apart by their
//! `sessionID`, and shown under its `Task`. Its task list comes with `todo.updated`.
//!
//! A recorded stream (`tests/fixtures/opencode/`, one `{"dir", "msg"}` a line) is read as it came:
//! the events, and the session its `POST /session` made.

use super::*;
use crate::engine::opencode::plan_approval;

/// What the translation keeps of OpenCode's conversation between its events (not saved).
#[derive(Default)]
pub(crate) struct OpencodeState {
    /// A compaction under way (ESC-205): its call to the model writes the summary the context
    /// holds alone afterwards.
    compacting: bool,
    /// The sessions of its subagents, and the call (`Task`) of each.
    children: HashMap<String, String>,
    /// Sessions of subagents not linked to their call yet.
    unlinked: Vec<String>,
    /// The role of each message (`user`, `assistant`): the parts of the user's are its own words.
    roles: HashMap<String, String>,
    /// The messages of the user typed in another client (`opencode attach`), which Escouade did not
    /// show when it sent them: their words are shown as they come.
    elsewhere: std::collections::HashSet<String>,
    /// The kind of each part (`text`, `reasoning`), for its deltas.
    kinds: HashMap<String, String>,
    /// The tokens and tools of each subagent, by its call.
    child_tokens: HashMap<String, u64>,
    /// The context window of each model of the server (`limit.context`), by its id.
    contexts: HashMap<String, u64>,
    /// The steps already counted (`step-finish` parts, by id): one told again is not counted twice.
    steps: std::collections::HashSet<String>,
    child_tools: HashMap<String, u64>,
    /// The agent's last message: where a copy forks it, and the plan of the mode Plan.
    last_message: Option<String>,
    /// Its session was seen busy since its last turn ended: its next `session.idle` ends this one.
    busy: bool,
    /// The turn was stopped (`MessageAbortedError`).
    aborted: bool,
    /// What went wrong in the turn, as OpenCode said it first.
    error: Option<String>,
    /// The turn's usage so far, its subagents' included.
    usage: TurnUsage,
    /// The task lists of `todo.updated`, numbered.
    todo_updates: u32,
    /// The agent's last message was its plan agent's (`plan`): the turn ends on a plan to approve.
    plan_agent: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct TurnUsage {
    input: u64,
    cache: u64,
    output: u64,
    cost: f64,
}

/// Where an event of OpenCode belongs.
enum Owner {
    /// The agent's own session.
    Own,
    /// A subagent's, under its call.
    Child(String),
    /// Neither.
    Other,
}

impl AgentRt {
    /// One event of its session (or a subagent's) on its `opencode serve`.
    pub(crate) fn opencode_frame(&mut self, f: &Value, fx: &mut Effects) {
        // A recorded stream: the events, and the session its `POST /session` made.
        if let Some(dir) = f.get("dir").and_then(Value::as_str) {
            let msg = &f["msg"];
            if dir == "s2c" {
                if msg["event"].is_object() {
                    self.opencode_frame(&msg["event"], fx);
                } else if msg["path"] == "/session" && msg["body"]["id"].is_string() {
                    let session = json!({ "type": "escouade.session", "properties": {
                        "sessionID": msg["body"]["id"], "info": msg["body"], "start": "new",
                    } });
                    self.opencode_frame(&session, fx);
                }
            }
            return;
        }
        let Some(kind) = f["type"].as_str() else {
            return;
        };
        let p = &f["properties"];
        let sid = p["sessionID"]
            .as_str()
            .or(p["part"]["sessionID"].as_str())
            .or(p["info"]["sessionID"].as_str());
        let owner = self.oc_owner(sid);
        let parent = match &owner {
            Owner::Child(call) => Some(call.clone()),
            _ => None,
        };
        let own = matches!(owner, Owner::Own);
        match kind {
            "escouade.session" => self.oc_session(p, fx),
            "escouade.model" => {
                if let Some(model) = p["model"].as_str() {
                    if self.meta.model != model {
                        self.meta.model = model.to_string();
                        fx.agent_changed = true;
                        fx.save = true;
                    }
                }
            }
            "escouade.contexts" => {
                if let Some(all) = p["contexts"].as_object() {
                    self.opencode.contexts = all
                        .iter()
                        .filter_map(|(k, v)| Some((k.clone(), v.as_u64()?)))
                        .collect();
                }
                self.oc_window(fx);
            }
            "escouade.mode" => {
                if let Some(mode) = p["mode"].as_str() {
                    if self.meta.set_mode(mode) {
                        fx.agent_changed = true;
                        fx.save = true;
                    }
                }
            }
            "escouade.prompt.failed" if self.meta.status.is_active() => {
                self.opencode.error = p["message"].as_str().map(str::to_string);
                self.oc_turn_end(fx);
            }
            "session.created" => {
                let info = &p["info"];
                let (Some(child), Some(parent)) = (info["id"].as_str(), info["parentID"].as_str())
                else {
                    return;
                };
                if !matches!(self.oc_owner(Some(parent)), Owner::Other)
                    && !self.opencode.children.contains_key(child)
                {
                    self.opencode.unlinked.push(child.to_string());
                }
            }
            "message.updated" => {
                let info = &p["info"];
                let Some(id) = info["id"].as_str() else {
                    return;
                };
                let role = info["role"].as_str().unwrap_or("assistant");
                self.opencode.roles.insert(id.to_string(), role.to_string());
                // A subagent's message tells the model it runs on, as the agent's is written.
                if let (Some(call), "assistant", Some(model)) =
                    (parent.as_deref(), role, info["modelID"].as_str())
                {
                    let model = match info["providerID"].as_str() {
                        Some(provider) => format!("{provider}/{model}"),
                        None => model.to_string(),
                    };
                    self.child_model(call, &model, fx);
                }
                if own && role == "user" && p["typedElsewhere"] == true {
                    self.opencode.elsewhere.insert(id.to_string());
                }
                if own && role == "assistant" {
                    self.opencode.last_message = Some(id.to_string());
                    self.opencode.plan_agent = info["agent"] == "plan" || info["mode"] == "plan";
                    if !self.meta.status.is_active() {
                        // A turn started elsewhere (a message typed in `opencode attach`).
                        self.set_status(AgentStatus::Running, fx);
                    }
                    if let Some(e) = info["error"].as_object() {
                        self.oc_error(&Value::Object(e.clone()));
                    }
                }
            }
            "message.part.updated" if !matches!(owner, Owner::Other) => {
                self.oc_part(&p["part"], parent, fx);
            }
            // A subagent's session not linked to its call yet: its cost is the turn's all the same.
            "message.part.updated"
                if p["part"]["type"] == "step-finish"
                    && sid.is_some_and(|s| self.opencode.unlinked.iter().any(|c| c == s)) =>
            {
                self.oc_step(&p["part"]);
            }
            "message.part.delta" if !matches!(owner, Owner::Other) => {
                let (Some(part), Some(text)) = (p["partID"].as_str(), p["delta"].as_str()) else {
                    return;
                };
                if p["field"] != "text" || text.is_empty() {
                    return;
                }
                let msg = p["messageID"].as_str().unwrap_or_default();
                if self.opencode.roles.get(msg).is_some_and(|r| r == "user") {
                    return;
                }
                if !self.conv.contains(part) {
                    let kind = match self.opencode.kinds.get(part).map(String::as_str) {
                        Some("reasoning") => "thinking",
                        _ => "text",
                    };
                    let item = json!({ "kind": kind, "id": part, "text": "", "parent": parent, "streaming": true });
                    self.append(item, fx);
                }
                self.push(
                    ConvOp::Delta {
                        id: part.to_string(),
                        text: text.to_string(),
                    },
                    fx,
                );
            }
            "permission.asked" if !matches!(owner, Owner::Other) => {
                self.oc_permission(p, parent, fx)
            }
            "question.asked" if !matches!(owner, Owner::Other) => self.oc_question(p, parent, fx),
            "permission.replied" | "question.replied" | "question.rejected" => {
                // Answered elsewhere (`opencode attach`): its card says so and goes.
                let Some(rid) = p["requestID"].as_str() else {
                    return;
                };
                let Some(req) = self.take_pending(rid) else {
                    return;
                };
                let patch = match kind {
                    "permission.replied" => json!({ "decision": match p["reply"].as_str() {
                        Some("always") => "always",
                        Some("reject") => "deny",
                        _ => "allow",
                    } }),
                    "question.replied" => {
                        let questions = req.input["questions"]
                            .as_array()
                            .cloned()
                            .unwrap_or_default();
                        let mut answers = Map::new();
                        for (q, a) in questions
                            .iter()
                            .zip(p["answers"].as_array().into_iter().flatten())
                        {
                            let labels: Vec<&str> = a
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .collect();
                            if let Some(text) = q["question"].as_str() {
                                answers.insert(text.to_string(), json!(labels.join(", ")));
                            }
                        }
                        json!({ "answers": answers })
                    }
                    _ => json!({ "cancelled": true }),
                };
                self.patch(&req.item_id, patch, fx);
                if self.pending.is_empty() && self.meta.status == AgentStatus::Waiting {
                    self.set_status(AgentStatus::Running, fx);
                }
                fx.agent_changed = true;
            }
            "todo.updated" if own => {
                self.opencode.todo_updates += 1;
                let todos: Vec<Value> = p["todos"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|t| {
                        let status = match t["status"].as_str() {
                            Some("in_progress") => "in_progress",
                            Some("completed" | "cancelled") => "completed",
                            _ => "pending",
                        };
                        json!({ "content": t["content"], "status": status, "activeForm": t["content"] })
                    })
                    .collect();
                let block = json!({
                    "id": format!("opencode-todo-{}", self.opencode.todo_updates),
                    "name": "TodoWrite",
                    "input": { "todos": todos },
                });
                self.plan_tool_use(None, &block, fx);
            }
            "session.status" if own => {
                if p["status"]["type"] == "busy" {
                    self.opencode.busy = true;
                    if !self.meta.status.is_active() {
                        self.set_status(AgentStatus::Running, fx);
                    }
                }
            }
            "session.compacted" if own => {
                self.opencode.compacting = false;
                self.notice("info", tr!("Contexte compacté", "Context compacted"), fx);
            }
            "session.idle" if own => {
                if std::mem::take(&mut self.opencode.busy) {
                    self.oc_turn_end(fx);
                }
            }
            "session.error" if own => {
                if p["error"]["name"] == "MessageAbortedError" {
                    self.opencode.aborted = true;
                } else {
                    self.oc_error(&p["error"]);
                }
            }
            _ => {}
        }
    }

    /// Another conversation (or another process of it): what was kept of the last one is gone.
    pub(super) fn opencode_new_conversation(&mut self) {
        self.opencode = OpencodeState::default();
    }

    /// Whose an event of the session `sid` is.
    fn oc_owner(&self, sid: Option<&str>) -> Owner {
        let Some(s) = sid else {
            return Owner::Other;
        };
        if let Some(call) = self.opencode.children.get(s) {
            return Owner::Child(call.clone());
        }
        if self.opencode.unlinked.iter().any(|c| c == s) {
            return Owner::Other;
        }
        match self.meta.session_id.as_deref() {
            Some(own) if own == s => Owner::Own,
            // Not known yet: the first session it hears of (a recorded stream).
            None => Owner::Own,
            _ => Owner::Other,
        }
    }

    /// The agent's session, new, resumed or copied.
    fn oc_session(&mut self, p: &Value, fx: &mut Effects) {
        let Some(id) = p["sessionID"].as_str().or(p["info"]["id"].as_str()) else {
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
                        "La session OpenCode n’a pas pu être reprise : nouvelle session (contexte vidé)",
                        "The OpenCode session couldn’t be resumed: new session (context cleared)"
                    ),
                    fx,
                );
            }
            self.meta.session_id = Some(id.to_string());
            self.meta.last_entry = None;
            fx.save = true;
        }
        // A copy has a session of its own from now on.
        if self.meta.fork_of.take().is_some() {
            self.meta.fork_at = None;
            fx.save = true;
        }
        self.saw_init = true;
    }

    /// What OpenCode said went wrong: its first message (a second one adds its stack).
    fn oc_error(&mut self, error: &Value) {
        if self.opencode.error.is_some() || error["name"] == "MessageAbortedError" {
            return;
        }
        let message = error["data"]["message"]
            .as_str()
            .or(error["message"].as_str())
            .or(error["name"].as_str())
            .unwrap_or_default();
        let first = message.lines().next().unwrap_or_default().trim();
        if !first.is_empty() {
            self.opencode.error = Some(first.to_string());
        }
    }

    /// A text of a message typed in another client (`opencode attach`): the user's message in the
    /// conversation, once (Escouade showed its own when it sent them).
    fn oc_typed_elsewhere(&mut self, id: &str, part: &Value, fx: &mut Effects) {
        let text = part["text"].as_str().unwrap_or_default();
        if part["type"] != "text"
            || part["synthetic"] == true
            || text.trim().is_empty()
            || self.conv.contains(id)
        {
            return;
        }
        let item = json!({
            "kind": "user", "id": id, "text": text, "images": 0, "ts": now_ms(),
            "queued": false, "origin": "terminal",
        });
        self.meta.resume_at = None;
        self.append(item, fx);
        self.meta.prompts += 1;
        self.meta.last_activity = now_ms();
        fx.agent_changed = true;
        fx.save = true;
    }

    /// A part of a message, as it starts, grows and ends.
    fn oc_part(&mut self, part: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(id) = part["id"].as_str() else {
            return;
        };
        let msg = part["messageID"].as_str().unwrap_or_default();
        if self.opencode.roles.get(msg).is_some_and(|r| r == "user") {
            // A compaction starts, asked (`/compact`, ESC-205) or by OpenCode itself: a message
            // of the user's that holds a part `compaction` (`session.compacted` tells its end).
            if part["type"] == "compaction" {
                if parent.is_none() && !self.conv.contains(id) {
                    self.opencode.compacting = true;
                    let text = tr!("Compaction du contexte…", "Compacting the context…");
                    let item = json!({ "kind": "notice", "id": id, "ts": now_ms(), "level": "info", "text": text });
                    self.append(item, fx);
                }
            } else if parent.is_none() && self.opencode.elsewhere.contains(msg) {
                self.oc_typed_elsewhere(id, part, fx);
            }
            return;
        }
        let main = parent.is_none();
        let kind = part["type"].as_str().unwrap_or_default();
        self.opencode.kinds.insert(id.to_string(), kind.to_string());
        match kind {
            "text" | "reasoning" => {
                if part["synthetic"] == true || part["ignored"] == true {
                    return;
                }
                let item_kind = if kind == "text" { "text" } else { "thinking" };
                let text = part["text"].as_str().unwrap_or_default();
                let done = part["time"]["end"].is_number();
                if !done {
                    if !self.conv.contains(id) {
                        let item = json!({ "kind": item_kind, "id": id, "text": text, "parent": parent, "streaming": true });
                        self.append(item, fx);
                    }
                    if main {
                        self.set_activity(block_activity(i18n::ui(), item_kind), fx);
                    }
                    return;
                }
                if kind == "text" && main && !text.trim().is_empty() {
                    if !self.turn_text.is_empty() {
                        self.turn_text.push_str("\n\n");
                    }
                    self.turn_text.push_str(text);
                    self.final_text = text.to_string();
                }
                let whole = json!({ "text": text, "streaming": false });
                if self.conv.contains(id) {
                    self.patch(id, whole, fx);
                } else if !text.trim().is_empty() {
                    let mut item = whole;
                    item["kind"] = json!(item_kind);
                    item["id"] = json!(id);
                    item["parent"] = json!(parent);
                    self.append(item, fx);
                }
            }
            "tool" => self.oc_tool(part, parent, fx),
            "step-finish" => {
                let Some((input, cache, output)) = self.oc_step(part) else {
                    return;
                };
                match &parent {
                    None => {
                        // After a compaction, its summary is what the context holds.
                        self.context_tokens = if self.opencode.compacting {
                            output
                        } else {
                            input + cache
                        };
                        self.oc_window(fx);
                        fx.agent_changed = true;
                    }
                    Some(call) => {
                        *self.opencode.child_tokens.entry(call.clone()).or_default() +=
                            input + cache + output;
                    }
                }
            }
            "patch" => {
                let files: Vec<String> = part["files"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| f.as_str().map(str::to_string))
                    .collect();
                for f in files {
                    self.touch(&f, fx);
                }
            }
            _ => {}
        }
    }

    /// A call to the model over (a part `step-finish`): its tokens and its cost added to the turn's,
    /// once (told again, a stream read anew, it is not counted twice). Its input, cache and output
    /// tokens; none when it was counted already.
    fn oc_step(&mut self, part: &Value) -> Option<(u64, u64, u64)> {
        if let Some(pid) = part["id"].as_str() {
            if self.opencode.steps.len() > 4096 {
                self.opencode.steps.clear();
            }
            if !self.opencode.steps.insert(pid.to_string()) {
                return None;
            }
        }
        let t = &part["tokens"];
        let n = |v: &Value| v.as_u64().unwrap_or(0);
        let (input, output) = (n(&t["input"]), n(&t["output"]) + n(&t["reasoning"]));
        let cache = n(&t["cache"]["read"]) + n(&t["cache"]["write"]);
        let u = &mut self.opencode.usage;
        u.input += input;
        u.cache += cache;
        u.output += output;
        u.cost += part["cost"].as_f64().unwrap_or(0.0);
        Some((input, cache, output))
    }

    /// The agent's context window: its model's (`limit.context`), when OpenCode knows it.
    fn oc_window(&mut self, fx: &mut Effects) {
        if let Some(n) = self.opencode.contexts.get(&self.meta.model).copied() {
            if self.context_window != n {
                self.context_window = n;
                fx.agent_changed = true;
            }
        }
    }

    /// A part `tool`: its call shown once it runs, its result once it ends.
    fn oc_tool(&mut self, part: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(call) = part["callID"].as_str().or(part["id"].as_str()) else {
            return;
        };
        let tool = part["tool"].as_str().unwrap_or_default();
        let state = &part["state"];
        let status = state["status"].as_str().unwrap_or("pending");
        // Its card is the question's.
        if tool == "question" || status == "pending" {
            return;
        }
        let cwd = self.meta.cwd.clone();
        let (name, input) = tool_of(tool, &state["input"], &cwd);
        if tool == "task" {
            if let Some(child) = state["metadata"]["sessionId"].as_str() {
                if !self.opencode.children.contains_key(child) {
                    self.opencode.unlinked.retain(|c| c != child);
                    self.opencode
                        .children
                        .insert(child.to_string(), call.to_string());
                }
            }
        }
        self.oc_call(
            call,
            &name,
            input.clone(),
            parent.clone(),
            tool != "todowrite",
            fx,
        );
        if !matches!(status, "completed" | "error") {
            return;
        }
        let meta = &state["metadata"];
        let (failed, mut text) = if status == "error" {
            (
                true,
                state["error"].as_str().unwrap_or_default().to_string(),
            )
        } else {
            let exit = meta["exit"].as_i64();
            (
                tool == "bash" && exit.is_some_and(|c| c != 0),
                state["output"].as_str().unwrap_or_default().to_string(),
            )
        };
        let path = input["file_path"].as_str().unwrap_or_default().to_string();
        let tur = match tool {
            "edit" | "multiedit" if !failed => {
                let diff = meta["filediff"]["patch"]
                    .as_str()
                    .or(meta["diff"].as_str())
                    .unwrap_or_default();
                json!({ "filePath": path, "structuredPatch": codex::hunks_of(diff) })
            }
            "write" if !failed => json!({
                "type": "create", "filePath": path, "content": input["content"], "structuredPatch": [],
            }),
            "task" => {
                if let Some(report) = tag(&text, "task_result") {
                    text = report.to_string();
                }
                json!({
                    "totalTokens": self.opencode.child_tokens.get(call),
                    "totalToolUseCount": self.opencode.child_tools.get(call),
                })
            }
            _ => Value::Null,
        };
        if !failed && matches!(tool, "edit" | "multiedit" | "write") && !path.is_empty() {
            self.touch(&path, fx);
        }
        self.oc_result(call, failed, text, &tur, fx);
    }

    /// The call `id` of the tool `name` (Claude Code's name), shown once as it starts; `plan`:
    /// the plan hears of it (a task list comes from `todo.updated` instead).
    fn oc_call(
        &mut self,
        id: &str,
        name: &str,
        input: Value,
        parent: Option<String>,
        plan: bool,
        fx: &mut Effects,
    ) {
        if self.conv.contains(id) {
            return;
        }
        match &parent {
            None => {
                // What was said before a tool was a step on the way, not the final reply.
                self.final_text.clear();
                let activity = tool_activity(name, &input, &self.meta.cwd);
                self.set_activity(activity, fx);
            }
            Some(call) => {
                *self.opencode.child_tools.entry(call.clone()).or_default() += 1;
            }
        }
        let item = json!({
            "kind": "tool", "id": id, "name": name, "input": item_input(name, &input),
            "status": "running", "parent": parent, "ts": now_ms(),
        });
        self.append(item, fx);
        if plan {
            self.plan_tool_use(
                parent.as_deref(),
                &json!({ "id": id, "name": name, "input": input }),
                fx,
            );
        }
    }

    /// The result of the call `id`: `text`, and for a file `tur` (as Claude Code's
    /// `tool_use_result`: its patch and its path; for a subagent, its totals).
    fn oc_result(&mut self, id: &str, is_error: bool, text: String, tur: &Value, fx: &mut Effects) {
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

    /// A permission OpenCode asks (`permission.asked`): a command, a change of files, the web, a
    /// path outside the agent's folder.
    fn oc_permission(&mut self, p: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(rid) = p["id"].as_str().map(str::to_string) else {
            return;
        };
        if self.conv.contains(&rid) {
            return;
        }
        let call = p["tool"]["callID"].as_str().unwrap_or(&rid).to_string();
        let meta = &p["metadata"];
        let patterns: Vec<String> = p["patterns"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        let always: Vec<String> = p["always"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        let first = patterns.first().cloned().unwrap_or_default();
        let mut description: Option<String> = None;
        let mut patch = Value::Null;
        let (tool, input) = match p["permission"].as_str().unwrap_or_default() {
            "bash" => {
                let command = meta["command"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| patterns.join(" "));
                ("Bash".to_string(), json!({ "command": command }))
            }
            "edit" | "write" => {
                let path = meta["filepath"]
                    .as_str()
                    .or(meta["filePath"].as_str())
                    .map(str::to_string)
                    .unwrap_or(first);
                if let Some(diff) = meta["diff"].as_str() {
                    let hunks = tool_result(
                        String::new(),
                        false,
                        &json!({ "structuredPatch": codex::hunks_of(diff) }),
                    );
                    patch = hunks["patch"].clone();
                }
                ("Edit".to_string(), json!({ "file_path": path }))
            }
            "webfetch" => ("WebFetch".to_string(), json!({ "url": first })),
            "external_directory" => {
                let list = patterns.join(", ");
                description = Some(tr!(
                    "Hors du dossier de l’agent : {list}",
                    "Outside the agent’s folder: {list}"
                ));
                (
                    "Permissions".to_string(),
                    json!({ "permissions": patterns }),
                )
            }
            other => {
                let (name, input) =
                    tool_of(other, &json!({ "patterns": patterns }), &self.meta.cwd);
                (name, input)
            }
        };
        // What « Toujours autoriser » keeps for the rest of the session, when it is not the request itself.
        if !always.is_empty() && always != patterns {
            let kept = always.join(", ");
            let line = tr!(
                "« Toujours autoriser » retiendra : {kept}",
                "“Always allow” will keep: {kept}"
            );
            description = Some(match description {
                Some(d) => format!("{d}\n{line}"),
                None => line,
            });
        }
        let mut item = json!({
            "kind": "permission", "id": rid, "toolUseId": call, "toolName": tool,
            "description": description, "input": trim_strings(&input), "reason": null,
            "canAlways": !always.is_empty(), "defaultNo": false, "decision": null, "ts": now_ms(),
        });
        if !patch.is_null() {
            item["patch"] = patch;
        }
        if let Some(sub) = parent {
            item["sub"] = sub.into();
        }
        let req = json!({ "description": description });
        let view = pending_view(&rid, &tool, &req, &input, &self.meta.cwd).always_as(&item);
        self.append(item, fx);
        self.add_pending(
            PendingReq {
                item_id: rid.clone(),
                tool_use_id: call,
                input: input.clone(),
                suggestions: json!(always),
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

    /// A question of OpenCode (`question.asked`, its `question` tool), as the card of a question
    /// of Claude Code's shows it.
    fn oc_question(&mut self, p: &Value, parent: Option<String>, fx: &mut Effects) {
        let Some(rid) = p["id"].as_str().map(str::to_string) else {
            return;
        };
        if self.conv.contains(&rid) {
            return;
        }
        let call = p["tool"]["callID"].as_str().unwrap_or(&rid).to_string();
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
                    "question": q["question"], "header": q["header"].as_str().unwrap_or_default(),
                    "multiSelect": q["multiple"] == true, "options": options,
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

    /// The end of the agent's turn (`session.idle`, or a prompt refused).
    fn oc_turn_end(&mut self, fx: &mut Effects) {
        let st = &mut self.opencode;
        let interrupted = std::mem::take(&mut self.interrupted) || std::mem::take(&mut st.aborted);
        let error = std::mem::take(&mut st.error).filter(|_| !interrupted);
        st.busy = false;
        let plan_agent = std::mem::take(&mut st.plan_agent);
        let last = st.last_message.clone();
        // The mode Plan's plan: the turn stays open on its card until it is approved, as Claude
        // Code's `ExitPlanMode` does (its work, or more planning, is the turn's next part).
        if plan_agent && !interrupted && error.is_none() {
            let plan = self.final_text.trim().to_string();
            if let (false, Some(msg)) = (plan.is_empty(), &last) {
                let rid = plan_approval(msg);
                if !self.conv.contains(&rid) {
                    let input = json!({ "plan": plan });
                    let item = json!({
                        "kind": "permission", "id": rid, "toolUseId": rid, "toolName": "ExitPlanMode",
                        "input": input, "canAlways": true, "defaultNo": false, "decision": null,
                        "ts": now_ms(),
                    });
                    self.append(item, fx);
                    let view =
                        pending_view(&rid, "ExitPlanMode", &Value::Null, &input, &self.meta.cwd);
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
        }
        if error.is_none() {
            self.set_last_entry(last, fx);
        }
        let failed = error.is_some();
        let u = std::mem::take(&mut self.opencode.usage);
        self.opencode.child_tokens.clear();
        self.opencode.child_tools.clear();
        let tokens = u.input + u.cache + u.output;
        if tokens > 0 {
            fx.turns.push(TurnRow {
                engine: self.meta.engine,
                model: self.meta.model.clone(),
                input: u.input,
                cache: u.cache,
                output: u.output,
                cost: u.cost,
            });
            self.meta.tokens += tokens;
            self.meta.cost += u.cost;
        }
        if self.meta.resume_at.take().is_some() {
            fx.save = true;
            fx.agent_changed = true;
        }
        let reply = std::mem::take(&mut self.final_text);
        let end = if interrupted {
            TurnEnd::Interrupted
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
            "cost": u.cost, "tokens": tokens, "isError": failed, "interrupted": interrupted,
            "error": error, "limited": false, "engine": self.meta.engine,
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

/// A path as OpenCode gave it, absolute (it may give one relative to the agent's folder). One of
/// Windows (`C:\…`, `\\server\…`) or of a POSIX system (`/…`) is absolute on every system, and a
/// relative one is joined with the separator of the agent's folder's kind: an OpenCode of WSL, or
/// a stream recorded elsewhere, reads the same.
pub(super) fn absolute(path: &str, cwd: &str) -> String {
    let of_windows = |p: &str| {
        let b = p.as_bytes();
        (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/'))
            || p.starts_with('\\')
    };
    if path.is_empty() || cwd.is_empty() || of_windows(path) || path.starts_with('/') {
        return path.to_string();
    }
    let sep = if of_windows(cwd) { '\\' } else { '/' };
    format!("{}{sep}{path}", cwd.trim_end_matches(['/', '\\']))
}

/// The first file a patch of `patch` / `apply_patch` changes (`*** Update File: <path>`).
pub(super) fn patch_file(text: &str) -> Option<&str> {
    text.lines().find_map(|l| {
        ["*** Update File: ", "*** Add File: ", "*** Delete File: "]
            .iter()
            .find_map(|p| l.strip_prefix(p))
            .map(str::trim)
    })
}

/// The tool Claude Code would have called for OpenCode's `tool`, and its input as Claude Code's.
fn tool_of(tool: &str, input: &Value, cwd: &str) -> (String, Value) {
    let path = |k: &str| absolute(input[k].as_str().unwrap_or_default(), cwd);
    let (name, input) = match tool {
        "bash" => (
            "Bash",
            json!({ "command": input["command"], "description": input["description"] }),
        ),
        "edit" | "multiedit" => (
            "Edit",
            json!({
                "file_path": path("filePath"), "old_string": input["oldString"],
                "new_string": input["newString"], "replace_all": input["replaceAll"],
            }),
        ),
        "write" => (
            "Write",
            json!({ "file_path": path("filePath"), "content": input["content"] }),
        ),
        "read" => (
            "Read",
            json!({ "file_path": path("filePath"), "offset": input["offset"], "limit": input["limit"] }),
        ),
        "grep" => (
            "Grep",
            json!({ "pattern": input["pattern"], "path": input["path"], "glob": input["include"] }),
        ),
        "glob" => (
            "Glob",
            json!({ "pattern": input["pattern"], "path": input["path"] }),
        ),
        "list" => ("LS", json!({ "path": input["path"] })),
        "patch" | "apply_patch" => {
            let text = input["patchText"].as_str().unwrap_or_default();
            let file = patch_file(text).map(|f| absolute(f, cwd));
            ("Edit", json!({ "file_path": file }))
        }
        "webfetch" => (
            "WebFetch",
            json!({ "url": input["url"], "prompt": input["format"] }),
        ),
        "websearch" => ("WebSearch", json!({ "query": input["query"] })),
        "todowrite" => {
            let todos: Vec<Value> = input["todos"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|t| json!({ "content": t["content"], "status": t["status"], "activeForm": t["content"] }))
                .collect();
            ("TodoWrite", json!({ "todos": todos }))
        }
        "task" => (
            "Task",
            json!({
                "description": input["description"], "prompt": input["prompt"],
                "subagent_type": input["subagent_type"],
            }),
        ),
        "skill" => ("Skill", json!({ "skill": input["name"] })),
        other => {
            // A tool of Escouade's server: `escouade_<tool>`.
            if let Some(t) = other.strip_prefix(&format!("{}_", crate::engine::mcp::SERVER)) {
                let name = crate::engine::mcp::tool_name(crate::engine::mcp::SERVER, t);
                return (name, input.clone());
            }
            return (other.to_string(), input.clone());
        }
    };
    let mut input = input;
    if let Some(o) = input.as_object_mut() {
        o.retain(|_, v| !v.is_null());
    }
    (name.to_string(), input)
}

#[cfg(test)]
#[path = "opencode_tests.rs"]
mod tests;
