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

    fn apply_item(&mut self, item: Item, fx: &mut Effects) {
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
            }
            Item::Thinking { text, parent, id } => {
                if let Some(id) = id.filter(|id| self.conv.contains(id)) {
                    self.patch(&id, json!({ "text": text, "streaming": false }), fx);
                    return;
                }
                let item = json!({ "kind": "thinking", "id": new_id(), "text": text, "parent": parent, "streaming": false });
                self.append(item, fx);
            }
            Item::Text { text, parent, id } => {
                if parent.is_none() {
                    if !self.turn_text.is_empty() {
                        self.turn_text.push_str("\n\n");
                    }
                    self.turn_text.push_str(&text);
                    self.final_text = text.clone();
                }
                if let Some(id) = id.filter(|id| self.conv.contains(id)) {
                    self.patch(&id, json!({ "text": text, "streaming": false }), fx);
                    return;
                }
                let item = json!({ "kind": "text", "id": new_id(), "text": text, "parent": parent, "streaming": false });
                self.append(item, fx);
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
                self.push(ConvOp::Delta { id, text }, fx);
            }
            Item::Subagent { call, model } => self.child_model(&call, &model, fx),
            Item::Tool {
                id,
                name,
                input,
                parent,
            } => {
                let item = json!({
                    "kind": "tool", "id": id, "name": name, "input": input, "status": "running",
                    "parent": parent, "ts": now_ms(),
                });
                self.append(item, fx);
                self.plan_tool_use(
                    parent.as_deref(),
                    &json!({ "id": id, "name": name, "input": input }),
                    fx,
                );
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
                if can_always {
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
            Item::Question {
                request,
                call,
                questions,
            } => {
                let item = json!({
                    "kind": "question", "id": request, "toolUseId": call, "questions": questions,
                    "answers": null, "ts": now_ms(),
                });
                let input = json!({ "questions": questions });
                self.ask(&request, &call, "AskUserQuestion".into(), input, item, fx);
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
            } => {
                let interrupted = std::mem::take(&mut self.interrupted) || interrupted;
                let text = std::mem::take(&mut self.turn_text);
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
                let failed = error.is_some();
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
                let took = self.active_since.map(|since| (now_ms() - since).max(0));
                let item = json!({
                    "kind": "turn", "id": new_id(), "ts": now_ms(), "durationMs": took, "cost": usage.cost,
                    "tokens": tokens, "isError": failed, "interrupted": interrupted, "error": error,
                    "limited": limited.is_some(), "engine": self.meta.engine,
                });
                self.append(item, fx);
                if !self.pending.is_empty() {
                    self.clear_pending(fx);
                }
                self.set_status(
                    if failed {
                        AgentStatus::Error
                    } else {
                        AgentStatus::Done
                    },
                    fx,
                );
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
                    "warning" | "error" => level,
                    _ => "info".to_string(),
                };
                let item = json!({ "kind": "notice", "id": new_id(), "ts": now_ms(), "level": level, "text": text });
                self.append(item, fx);
            }
            // Written by the process's adapter, in its order, before the items reach the agent.
            Item::Write { .. } | Item::Unknown => {}
        }
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
