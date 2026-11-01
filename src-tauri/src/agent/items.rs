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
            Item::Thinking { text, parent } => {
                let item = json!({ "kind": "thinking", "id": new_id(), "text": text, "parent": parent, "streaming": false });
                self.append(item, fx);
            }
            Item::Text { text, parent } => {
                if parent.is_none() {
                    if !self.turn_text.is_empty() {
                        self.turn_text.push_str("\n\n");
                    }
                    self.turn_text.push_str(&text);
                    self.final_text = text.clone();
                }
                let item = json!({ "kind": "text", "id": new_id(), "text": text, "parent": parent, "streaming": false });
                self.append(item, fx);
            }
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
            Item::ToolDone { id, output, error } => {
                self.plan_apply(fx, |plan| {
                    plan.on_tool_result(&id, error, &output, &Value::Null, now_ms())
                });
                let result = tool_result(output, error, &Value::Null);
                self.patch(
                    &id,
                    json!({ "status": if error { "error" } else { "ok" }, "result": result }),
                    fx,
                );
            }
            Item::Touched { path } => self.touch(&path, fx),
            Item::Permission { request, call } => {
                let asked = self.conv.get(&call).cloned().unwrap_or_default();
                let tool = asked["name"].as_str().unwrap_or_default().to_string();
                let item = json!({
                    "kind": "permission", "id": request, "toolUseId": call, "toolName": tool,
                    "input": asked["input"], "canAlways": false, "defaultNo": false,
                    "decision": null, "ts": now_ms(),
                });
                self.ask(&request, &call, tool, asked["input"].clone(), item, fx);
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
            Item::Turn {
                interrupted,
                limited,
                usage,
            } => {
                let interrupted = std::mem::take(&mut self.interrupted) || interrupted;
                let text = std::mem::take(&mut self.turn_text);
                self.forget_turn();
                self.close_open_items(fx);
                self.plan_apply(fx, |plan| plan.end_turn(now_ms()));
                let tokens = usage.input + usage.cache + usage.output;
                if tokens > 0 || usage.cost > 0.0 {
                    fx.turns.push(TurnRow {
                        engine: self.meta.engine,
                        model: self.meta.model.clone(),
                        input: usage.input,
                        cache: usage.cache,
                        output: usage.output,
                        cost: usage.cost,
                    });
                    self.meta.tokens += tokens;
                }
                let item = json!({
                    "kind": "turn", "id": new_id(), "ts": now_ms(), "durationMs": null, "cost": usage.cost,
                    "tokens": tokens, "isError": false, "interrupted": interrupted, "error": null,
                    "limited": limited.is_some(), "engine": self.meta.engine,
                });
                self.append(item, fx);
                if !self.pending.is_empty() {
                    self.clear_pending(fx);
                }
                self.set_status(AgentStatus::Done, fx);
                fx.agent_changed = true;
                fx.save = true;
                fx.turn_end = Some(if let Some(reset) = limited {
                    fx.limited = Some(reset);
                    TurnEnd::Limited
                } else if interrupted {
                    TurnEnd::Interrupted
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
        self.append(item, fx);
        let view = pending_view(request, &tool, &Value::Null, &input, &self.meta.cwd);
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
