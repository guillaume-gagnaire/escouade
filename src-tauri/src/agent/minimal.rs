//! The frames of the minimal fake engine of the conformance bench (`engine/minimal/`), translated
//! into the agent's conversation: what the least engine has to write here, as a real one writes
//! `agent/<engine>.rs`. Its protocol is its own, a frame a step (`type`):
//!
//! - `start {session}`: a turn begins, in that session;
//! - `thinking {text}`, `say {text, parent?}`: its reasoning, its text (a subagent's, `parent`);
//! - `call {id, tool, input, parent?}`: a tool (`shell`, `patch`, `agent`, named here as Claude
//!   Code names them: `Bash`, `Edit`, `Task`), and `done {call, output, error?}` its result;
//! - `ask {request, call}`: the approval of a call; `question {request, call, text, choices}`;
//! - `end {interrupted?, usage?}`: the turn is over, its tokens read (`usage {input, output}`).

use super::*;

impl AgentRt {
    /// One frame of the minimal fake engine.
    pub(crate) fn minimal_frame(&mut self, f: &Value, fx: &mut Effects) {
        let parent = f["parent"].as_str().map(str::to_string);
        match f["type"].as_str().unwrap_or("") {
            "start" => {
                if let Some(session) = f["session"].as_str() {
                    if self.meta.session_id.as_deref() != Some(session) {
                        self.meta.session_id = Some(session.to_string());
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
            "thinking" => {
                let item = json!({ "kind": "thinking", "id": new_id(), "text": f["text"], "parent": parent, "streaming": false });
                self.append(item, fx);
            }
            "say" => {
                let text = f["text"].as_str().unwrap_or_default();
                if parent.is_none() {
                    if !self.turn_text.is_empty() {
                        self.turn_text.push_str("\n\n");
                    }
                    self.turn_text.push_str(text);
                    self.final_text = text.to_string();
                }
                let item = json!({ "kind": "text", "id": new_id(), "text": text, "parent": parent, "streaming": false });
                self.append(item, fx);
            }
            "call" => {
                let (Some(id), Some(tool)) = (f["id"].as_str(), f["tool"].as_str()) else {
                    return;
                };
                let input = &f["input"];
                // Its tools take the names Claude Code gives them, with their inputs.
                let (name, input) = match tool {
                    "shell" => ("Bash", json!({ "command": input["command"] })),
                    "patch" => {
                        let file = format!(
                            "{}/{}",
                            self.meta.cwd.trim_end_matches(['/', '\\']),
                            input["path"].as_str().unwrap_or_default()
                        );
                        ("Edit", json!({ "file_path": file, "diff": input["diff"] }))
                    }
                    "agent" => (
                        "Task",
                        json!({ "description": input["task"], "prompt": input["task"] }),
                    ),
                    other => (other, input.clone()),
                };
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
            "done" => {
                let Some(id) = f["call"].as_str() else {
                    return;
                };
                let is_error = f["error"] == true;
                let text = f["output"].as_str().unwrap_or_default().to_string();
                self.plan_apply(fx, |plan| {
                    plan.on_tool_result(id, is_error, &text, &Value::Null, now_ms())
                });
                let call = self.conv.get(id).cloned().unwrap_or_default();
                if call["name"] == "Edit" && !is_error {
                    if let Some(path) = call["input"]["file_path"].as_str() {
                        self.touch(path, fx);
                    }
                }
                let result = tool_result(text, is_error, &Value::Null);
                self.patch(
                    id,
                    json!({ "status": if is_error { "error" } else { "ok" }, "result": result }),
                    fx,
                );
            }
            "ask" | "question" => {
                let (Some(rid), Some(call)) = (f["request"].as_str(), f["call"].as_str()) else {
                    return;
                };
                let (tool, input, item) = if f["type"] == "question" {
                    let options: Vec<Value> = f["choices"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|c| json!({ "label": c, "description": "" }))
                        .collect();
                    let questions = json!([{
                        "question": f["text"], "header": "", "multiSelect": false, "options": options,
                    }]);
                    let item = json!({
                        "kind": "question", "id": rid, "toolUseId": call, "questions": questions,
                        "answers": null, "ts": now_ms(),
                    });
                    (
                        "AskUserQuestion".to_string(),
                        json!({ "questions": questions }),
                        item,
                    )
                } else {
                    let asked = self.conv.get(call).cloned().unwrap_or_default();
                    let tool = asked["name"].as_str().unwrap_or_default().to_string();
                    let item = json!({
                        "kind": "permission", "id": rid, "toolUseId": call, "toolName": tool,
                        "input": asked["input"], "canAlways": false, "defaultNo": false,
                        "decision": null, "ts": now_ms(),
                    });
                    (tool, asked["input"].clone(), item)
                };
                self.append(item, fx);
                let view = pending_view(rid, &tool, &Value::Null, &input, &self.meta.cwd);
                self.add_pending(
                    PendingReq {
                        item_id: rid.to_string(),
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
            "end" => {
                let interrupted = std::mem::take(&mut self.interrupted) || f["interrupted"] == true;
                let text = std::mem::take(&mut self.turn_text);
                self.forget_turn();
                self.close_open_items(fx);
                self.plan_apply(fx, |plan| plan.end_turn(now_ms()));
                let (input, output) = (
                    f["usage"]["input"].as_u64().unwrap_or(0),
                    f["usage"]["output"].as_u64().unwrap_or(0),
                );
                if input + output > 0 {
                    fx.turns.push(TurnRow {
                        engine: self.meta.engine,
                        model: self.meta.model.clone(),
                        input,
                        cache: 0,
                        output,
                        cost: 0.0,
                    });
                    self.meta.tokens += input + output;
                }
                let item = json!({
                    "kind": "turn", "id": new_id(), "ts": now_ms(), "durationMs": null, "cost": 0.0,
                    "tokens": input + output, "isError": false, "interrupted": interrupted, "error": null,
                    "limited": false, "engine": self.meta.engine,
                });
                self.append(item, fx);
                if !self.pending.is_empty() {
                    self.clear_pending(fx);
                }
                self.set_status(AgentStatus::Done, fx);
                fx.agent_changed = true;
                fx.save = true;
                fx.turn_end = Some(if interrupted {
                    TurnEnd::Interrupted
                } else {
                    TurnEnd::Finished(text)
                });
            }
            _ => {}
        }
    }
}
