//! The normalized items and effects a harness's plugin gives (INT-9, `escouade_plugin_protocol::engine::Item`),
//! applied to the agent: the state stays the core's, the plugin only says what happened. The same rules as the
//! translations written in the core (`agent/minimal.rs`, `agent/claude.rs`): the session kept, the turn's text
//! gathered, the plan's rows told of the calls, a permission or a question waiting, the turn closed with what was
//! left open. An item the core does not know is left aside.

use super::*;
use escouade_plugin_protocol::engine::Item;

impl AgentRt {
    /// The items of `items` (each an `Item` as JSON), in their order.
    pub(crate) fn apply_items(&mut self, items: &[Value], fx: &mut Effects) {
        for item in items {
            match serde_json::from_value::<Item>(item.clone()) {
                Ok(item) => self.apply_item(item, fx),
                Err(e) => log::debug!("an item of a harness's plugin left aside: {e}"),
            }
        }
    }

    pub(super) fn apply_item(&mut self, item: Item, fx: &mut Effects) {
        match item {
            Item::Start { session } => {
                if let Some(session) = session {
                    if self.meta.session_id.as_deref() != Some(session.as_str()) {
                        self.meta.session_id = Some(session);
                        fx.save = true;
                    }
                    // A copy has a session of its own from its first turn on.
                    if self.meta.fork_of.take().is_some() {
                        self.meta.fork_at = None;
                        fx.save = true;
                    }
                }
                if !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            Item::Session { session } => {
                if self.meta.session_id.as_deref() != Some(session.as_str()) {
                    if self.meta.session_id.is_some() {
                        self.reset_plan(fx);
                    }
                    self.meta.session_id = Some(session);
                    self.meta.last_entry = None;
                    fx.save = true;
                }
                if self.meta.fork_of.take().is_some() {
                    self.meta.fork_at = None;
                    fx.save = true;
                }
                self.saw_init = true;
            }
            Item::Thinking { text, parent, id } => self.whole("thinking", text, parent, id, fx),
            Item::Text { text, parent, id } => {
                if parent.is_none() && !text.trim().is_empty() {
                    if !self.turn_text.is_empty() {
                        self.turn_text.push_str("\n\n");
                    }
                    self.turn_text.push_str(&text);
                    self.final_text = text.clone();
                }
                self.whole("text", text, parent, id, fx);
            }
            Item::Delta {
                id,
                text,
                thinking,
                parent,
            } => {
                let kind = if thinking { "thinking" } else { "text" };
                if !self.conv.contains(&id) {
                    let item = json!({ "kind": kind, "id": id, "text": "", "parent": parent, "streaming": true });
                    self.append(item, fx);
                }
                if parent.is_none() {
                    self.set_activity(block_activity(i18n::ui(), kind), fx);
                }
                if !text.is_empty() {
                    self.push(ConvOp::Delta { id, text }, fx);
                }
            }
            Item::Subagent { call, model } => self.child_model(&call, &model, fx),
            Item::Tool {
                id,
                name,
                input,
                parent,
                unplanned,
            } => {
                // Told again (its plugin started again): it is there already.
                if self.conv.contains(&id) {
                    return;
                }
                if parent.is_none() {
                    // What was said before a tool was a step on the way, not the final reply.
                    self.final_text.clear();
                    let activity = tool_activity(&name, &input, &self.meta.cwd);
                    self.set_activity(activity, fx);
                }
                let item = json!({
                    "kind": "tool", "id": id, "name": name, "input": item_input(&name, &input),
                    "status": "running", "parent": parent, "ts": now_ms(),
                });
                self.append(item, fx);
                if !unplanned {
                    self.plan_tool_use(
                        parent.as_deref(),
                        &json!({ "id": id, "name": name, "input": input }),
                        fx,
                    );
                }
            }
            Item::ToolInput { id, name, input } => {
                let Some(known) = self.conv.get(&id).cloned() else {
                    return;
                };
                let mut patch = json!({});
                let name = match name {
                    Some(name) => {
                        patch["name"] = json!(name);
                        name
                    }
                    None => known["name"].as_str().unwrap_or_default().to_string(),
                };
                if !input.is_null() {
                    patch["input"] = item_input(&name, &input);
                }
                self.patch(&id, patch, fx);
            }
            Item::ToolDone {
                id,
                output,
                error,
                result: tur,
            } => {
                // Told again: its result is there already.
                if self.conv.get(&id).is_some_and(|i| i["status"] != "running") {
                    return;
                }
                self.tool_done(id, output, error, tur, fx);
            }
            Item::ToolResult {
                id,
                output,
                error,
                result,
            } => self.tool_done(id, output, error, result, fx),
            Item::Conversation { session, notice } => self.on_conversation(session, notice, fx),
            Item::Message {
                id,
                model,
                usage,
                main,
            } => self.on_message(id, model, usage, main, fx),
            Item::MessageOutput { id, output } => {
                if let Some((_, u)) = self.live.get_mut(&id) {
                    u.output = output;
                    fx.agent_changed = true;
                }
            }
            Item::Tasks { frame } => {
                self.plan_apply(fx, |plan| plan.on_system(&frame, now_ms()));
                if frame["subtype"] == "task_notification" && frame["task_id"].is_string() {
                    self.keep_report(&frame, fx);
                }
            }
            Item::Event {
                id,
                source,
                text,
                from,
            } => {
                if source == "task" {
                    if let Some(task) = tag(&text, "task-id") {
                        *self.announced_tasks.entry(task.to_string()).or_default() += 1;
                    }
                }
                let mut item = json!({
                    "kind": "event", "id": id.unwrap_or_else(new_id), "source": source, "text": text,
                    "ts": now_ms(),
                });
                if let Some(from) = from {
                    item["from"] = from.into();
                }
                self.append(item, fx);
                fx.save = true;
            }
            Item::Output { id, level, text } => {
                let id = id.unwrap_or_else(new_id);
                if !self.conv.contains(&id) {
                    let item = json!({ "kind": "notice", "id": id, "ts": now_ms(), "level": level, "text": text });
                    self.append(item, fx);
                }
            }
            Item::Call {
                id,
                name,
                input,
                parent,
            } => self.on_call(id, name, input, parent, fx),
            Item::Replayed {
                id,
                text,
                origin,
                from,
                synthetic,
                images,
                files,
            } => self.on_replay(id, text, origin, from, synthetic, images, files, fx),
            Item::Ask {
                request,
                call,
                tool,
                input,
                title,
                description,
                reason,
                suggestions,
                no_always,
                default_no,
                task,
            } => self.on_ask(
                claude::Asking {
                    request,
                    call,
                    tool,
                    input,
                    title,
                    description,
                    reason,
                    suggestions,
                    no_always,
                    default_no,
                    task,
                },
                fx,
            ),
            Item::Result {
                id,
                failed,
                error,
                failure,
                reply,
                limited,
                resets,
                sign_in,
                duration,
                window,
                cost,
                tokens,
                models,
            } => self.on_result(
                claude::TurnResult {
                    id,
                    failed,
                    error,
                    failure,
                    reply,
                    limited,
                    resets,
                    sign_in,
                    duration,
                    window,
                    cost,
                    tokens,
                    models,
                },
                fx,
            ),
            Item::Limit { scoped } => {
                self.meta.limit_model = scoped.then(|| self.model_in_use());
            }
            Item::Overage { on } => fx.overage = Some(on),
            Item::Remote { state } => {
                self.remote_state = state;
                fx.agent_changed = true;
            }
            Item::Touched { path } => self.touch(&path, fx),
            Item::Permission {
                request,
                call,
                tool,
                input,
                description,
                reason,
                diff,
                can_always,
                parent,
                hunks,
                always_by_harness,
                ends_stream,
            } => {
                if self.conv.contains(&request) {
                    return;
                }
                let asked = self.conv.get(&call).cloned().unwrap_or_default();
                let tool =
                    tool.unwrap_or_else(|| asked["name"].as_str().unwrap_or_default().to_string());
                let input = input.unwrap_or_else(|| asked["input"].clone());
                let mut item = json!({
                    "kind": "permission", "id": request, "toolUseId": call, "toolName": tool,
                    "description": description, "input": trim_strings(&input), "reason": reason,
                    "canAlways": can_always, "defaultNo": false, "decision": null, "ts": now_ms(),
                });
                if can_always && !always_by_harness {
                    item["alwaysScope"] = json!("agent");
                }
                if let Some(d) = &diff {
                    let hunks = escouade_acp::line_hunks(
                        d.old_text.as_deref().unwrap_or_default(),
                        &d.new_text,
                    );
                    let shown = tool_result(
                        String::new(),
                        false,
                        &json!({ "filePath": d.path, "structuredPatch": hunks }),
                    );
                    item["patch"] = shown["patch"].clone();
                }
                if let Some(hunks) = hunks {
                    let shown =
                        tool_result(String::new(), false, &json!({ "structuredPatch": hunks }));
                    if !shown["patch"].is_null() {
                        item["patch"] = shown["patch"].clone();
                    }
                }
                if let Some(sub) = parent {
                    item["sub"] = sub.into();
                }
                if ends_stream {
                    self.close_open_items(fx);
                }
                let req = json!({ "description": description, "decision_reason": reason });
                fx.notify = Some(AgentAlert::new(
                    NotifyKind::Question,
                    notify::permission_body(&tool, &input, &self.meta.cwd),
                ));
                self.ask_with(&request, &call, tool, input, &req, item, fx);
            }
            Item::Question {
                request,
                call,
                questions,
                parent,
            } => {
                if self.conv.contains(&request) {
                    return;
                }
                let mut item = json!({
                    "kind": "question", "id": request, "toolUseId": call, "questions": questions,
                    "answers": null, "ts": now_ms(),
                });
                if let Some(sub) = parent {
                    item["sub"] = sub.into();
                }
                let input = json!({ "questions": questions });
                fx.notify = Some(AgentAlert::new(
                    NotifyKind::Question,
                    notify::question_body(crate::engine::of(&self.meta).speaker(), &input),
                ));
                self.ask(&request, &call, "AskUserQuestion".into(), input, item, fx);
            }
            Item::PermissionCard {
                request,
                call,
                tool,
                input,
                description,
                reason,
                hunks,
                can_always,
                always_scope,
                parent,
            } => {
                if self.conv.contains(&request) {
                    return;
                }
                let mut item = json!({
                    "kind": "permission", "id": request, "toolUseId": call, "toolName": tool,
                    "description": description, "input": trim_strings(&input), "reason": reason,
                    "canAlways": can_always, "defaultNo": false, "decision": null, "ts": now_ms(),
                });
                if let Some(scope) = always_scope {
                    item["alwaysScope"] = json!(scope);
                }
                if !hunks.is_null() {
                    let shown =
                        tool_result(String::new(), false, &json!({ "structuredPatch": hunks }));
                    if !shown["patch"].is_null() {
                        item["patch"] = shown["patch"].clone();
                    }
                }
                if let Some(sub) = parent {
                    item["sub"] = sub.into();
                }
                let req = json!({ "description": description, "decision_reason": reason });
                fx.notify = Some(AgentAlert::new(
                    NotifyKind::Question,
                    notify::permission_body(&tool, &input, &self.meta.cwd),
                ));
                self.ask_with(&request, &call, tool, input, &req, item, fx);
            }
            Item::QuestionCard {
                request,
                call,
                questions,
                form,
                parent,
            } => {
                if self.conv.contains(&request) {
                    return;
                }
                let input = json!({ "questions": questions });
                let mut item = json!({
                    "kind": "question", "id": request, "toolUseId": call, "questions": questions,
                    "answers": null, "ts": now_ms(),
                });
                if !form.is_null() {
                    item["form"] = form.clone();
                }
                if let Some(sub) = parent {
                    item["sub"] = sub.into();
                }
                self.append(item, fx);
                let mut view = pending_view(
                    &request,
                    "AskUserQuestion",
                    &Value::Null,
                    &input,
                    &self.meta.cwd,
                );
                // A form says what it is for: its message, on the list too.
                view.description = form["message"].as_str().map(|m| {
                    let mut cut = false;
                    let m = capped(m, &mut cut);
                    view.cut |= cut;
                    m
                });
                self.add_pending(
                    PendingReq {
                        item_id: request,
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
            Item::Answered {
                request,
                decision,
                answers,
            } => {
                let Some(req) = self.take_pending(&request) else {
                    return;
                };
                let patch = if let Some(decision) = decision {
                    json!({ "decision": decision })
                } else if let Some(answers) = answers {
                    let questions = req.input["questions"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    let mut by_question = Map::new();
                    for (q, a) in questions
                        .iter()
                        .zip(answers.as_array().into_iter().flatten())
                    {
                        let labels: Vec<&str> = a
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                            .collect();
                        if let Some(text) = q["question"].as_str() {
                            by_question.insert(text.to_string(), json!(labels.join(", ")));
                        }
                    }
                    json!({ "answers": by_question })
                } else {
                    json!({ "cancelled": true })
                };
                self.patch(&req.item_id, patch, fx);
                if self.pending.is_empty() && self.meta.status == AgentStatus::Waiting {
                    self.set_status(AgentStatus::Running, fx);
                }
                fx.agent_changed = true;
            }
            Item::Quota { meters } => {
                let read: Vec<crate::model::QuotaMeter> =
                    serde_json::from_value(meters).unwrap_or_default();
                if !read.is_empty() {
                    fx.rate = Some(read);
                }
            }
            Item::User { id, text } => {
                if self.conv.contains(&id) {
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
            Item::Model { model } => {
                if self.meta.model != model {
                    self.meta.model = model;
                    fx.agent_changed = true;
                    fx.save = true;
                }
            }
            Item::LimitRest => self.limit_rest = true,
            Item::Entry { entry } => self.set_last_entry(entry, fx),
            Item::Activity { tool, input, block } => {
                let activity = match (tool, block) {
                    (Some(tool), _) => tool_activity(&tool, &input, &self.meta.cwd),
                    (None, Some(block)) => block_activity(i18n::ui(), &block),
                    (None, None) => return,
                };
                self.set_activity(activity, fx);
            }
            Item::Took { ms } => self.turn_took = Some(ms),
            Item::Context { tokens, window } => {
                if let Some(tokens) = tokens {
                    self.context_tokens = tokens;
                    fx.agent_changed = true;
                }
                if let Some(window) = window.filter(|w| *w != self.context_window) {
                    self.context_window = window;
                    fx.agent_changed = true;
                }
            }
            Item::Todos { todos } => {
                self.todo_writes += 1;
                let block = json!({
                    "id": format!("plugin-todo-{}", self.todo_writes),
                    "name": "TodoWrite",
                    "input": { "todos": todos },
                });
                self.plan_tool_use(None, &block, fx);
            }
            Item::Mode { mode } => {
                if self.meta.set_mode(&mode) {
                    fx.agent_changed = true;
                    fx.save = true;
                }
            }
            Item::Turn {
                interrupted,
                limited,
                usage,
                error,
                entry,
            } => {
                let interrupted = std::mem::take(&mut self.interrupted) || interrupted;
                let limited = limited.filter(|_| !interrupted);
                if entry.is_some() {
                    self.set_last_entry(entry, fx);
                }
                let text = std::mem::take(&mut self.turn_text);
                let reply = std::mem::take(&mut self.final_text);
                self.forget_turn();
                self.close_open_items(fx);
                self.plan_apply(fx, |plan| plan.end_turn(now_ms()));
                let tokens = usage.input + usage.cache + usage.output;
                if tokens > 0 || usage.cost > 0.0 {
                    // An agent on its harness's default model: that one's row (Gemini's router).
                    let model = if self.meta.model.is_empty() {
                        crate::engine::of(&self.meta)
                            .models()
                            .first()
                            .map_or_else(String::new, |m| m.id.to_string())
                    } else {
                        self.meta.model.clone()
                    };
                    fx.turns.push(TurnRow {
                        engine: self.meta.engine,
                        model,
                        input: usage.input,
                        cache: usage.cache,
                        output: usage.output,
                        cost: usage.cost,
                    });
                    self.meta.tokens += tokens;
                    // What the harness says the turn cost (Mistral Vibe's `usage_update`, INT-10.6).
                    self.meta.cost += usage.cost;
                }
                let error = error.filter(|_| !interrupted);
                // Stopped by a limit its harness says it waits for: told, not an error of the agent's.
                let rest = std::mem::take(&mut self.limit_rest) && limited.is_some();
                let failed = error.is_some() && !rest;
                let ended_badly = failed || rest;
                // A resume planned is over (the core plans the next one, `fx.limited`), as is a move once a turn
                // went well after it; the plan's files are read again (as the core's own translations do).
                if self.meta.resume_at.take().is_some() {
                    fx.save = true;
                    fx.agent_changed = true;
                }
                if !failed && self.meta.moved_from.take().is_some() {
                    self.meta.moved_resume_at = None;
                    fx.save = true;
                    fx.agent_changed = true;
                }
                fx.plan_files = true;
                let took = self
                    .turn_took
                    .take()
                    .or_else(|| self.active_since.map(|since| (now_ms() - since).max(0)));
                let item = json!({
                    "kind": "turn", "id": new_id(), "ts": now_ms(), "durationMs": took, "cost": usage.cost,
                    "tokens": tokens, "isError": ended_badly, "interrupted": interrupted, "error": error,
                    "limited": limited.is_some(), "engine": self.meta.engine,
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
                if !interrupted && !had_queue && limited.is_none() {
                    fx.notify = Some(match &error {
                        Some(reason) => {
                            AgentAlert::new(NotifyKind::Error, notify::error_body(reason))
                        }
                        None => AgentAlert::new(NotifyKind::Done, notify::done_body(&reply)),
                    });
                }
                fx.agent_changed = true;
                fx.save = true;
                fx.turn_end = Some(if let Some(reset) = limited {
                    fx.limited = Some(reset);
                    TurnEnd::Limited
                } else if interrupted {
                    TurnEnd::Interrupted
                } else if let Some(reason) = error {
                    TurnEnd::Error(reason)
                } else {
                    TurnEnd::Finished(text)
                });
            }
            Item::Notice { text, level } => {
                let level = match level.as_str() {
                    "warn" | "warning" | "error" => level,
                    _ => "info".to_string(),
                };
                let item = json!({ "kind": "notice", "id": new_id(), "ts": now_ms(), "level": level, "text": text });
                self.append(item, fx);
            }
            // Written by the process's adapter, in its order, before the items reach the agent; `Closed` is its own too.
            Item::Write { .. }
            | Item::Told { .. }
            | Item::Answer { .. }
            | Item::Said { .. }
            | Item::Usage { .. }
            | Item::Ready { .. }
            | Item::Compacted { .. }
            | Item::Stop { .. }
            | Item::Relaunch { .. }
            | Item::Pause
            | Item::File { .. }
            | Item::Closed
            | Item::Unknown => {}
        }
    }

    /// A tool's result: what it means for the plan, its row told.
    fn tool_done(&mut self, id: String, output: String, error: bool, tur: Value, fx: &mut Effects) {
        if self.plan_calls.remove(&id) {
            fx.plan_files = true;
        }
        if self.plan_removals.remove(&id) && !error {
            self.plan_apply(fx, PlanState::finish_files);
        }
        self.plan_apply(fx, |plan| {
            plan.on_tool_result(&id, error, &output, &tur, now_ms())
        });
        let result = tool_result(output, error, &tur);
        self.patch(
            &id,
            json!({ "status": if error { "error" } else { "ok" }, "result": result }),
            fx,
        );
    }

    /// A text or a reasoning whole: the end of what streamed under `id`, else an item of its own (under `id` when it has
    /// one, nothing when it says nothing).
    fn whole(
        &mut self,
        kind: &str,
        text: String,
        parent: Option<String>,
        id: Option<String>,
        fx: &mut Effects,
    ) {
        if let Some(known) = id.as_ref().filter(|id| self.conv.contains(id)) {
            self.patch(known, json!({ "text": text, "streaming": false }), fx);
            return;
        }
        if id.is_some() && text.trim().is_empty() {
            return;
        }
        let id = id.unwrap_or_else(new_id);
        let item =
            json!({ "kind": kind, "id": id, "text": text, "parent": parent, "streaming": false });
        self.append(item, fx);
    }

    /// A request of the harness waiting for the user: its card, and the agent waiting.
    fn ask(
        &mut self,
        request: &str,
        call: &str,
        tool: String,
        input: Value,
        item: Value,
        fx: &mut Effects,
    ) {
        self.ask_with(request, call, tool, input, &Value::Null, item, fx);
    }

    /// `ask`, with what the card says of it (`req`: its `description` and `decision_reason`).
    #[allow(clippy::too_many_arguments)]
    fn ask_with(
        &mut self,
        request: &str,
        call: &str,
        tool: String,
        input: Value,
        req: &Value,
        item: Value,
        fx: &mut Effects,
    ) {
        let view = pending_view(request, &tool, req, &input, &self.meta.cwd).always_as(&item);
        self.append(item, fx);
        self.add_pending(
            PendingReq {
                item_id: request.to_string(),
                tool_use_id: call.to_string(),
                input,
                suggestions: Value::Null,
                view,
                since: now_ms(),
            },
            fx,
        );
        self.set_status(AgentStatus::Waiting, fx);
        fx.agent_changed = true;
    }
}
