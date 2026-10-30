//! « Vue Copilot »: the agent's conversation as Copilot's own interface (`copilot`, in a terminal)
//! writes it in its session's log (`session-state/<id>/events.jsonl`), and what its hooks tell while
//! it runs (`tui`, `engine/copilot/terminal.rs`).
//!
//! The log's entries are the events of the session, as `copilot --server --stdio` sends them
//! (`session.event`), less those it never keeps (`ephemeral`: the deltas, `assistant.usage`,
//! `session.idle`): each goes through the same translation (`cp_event`). What is missing is made
//! up: the user's message from its `user.message`, the usage of each call from its
//! `session.usage_record`, the end of a turn from the `agentStop` hook (or `abort` for Escape).
//!
//! Its hooks (docs/TODO-1.8.md, point 0, « Vue terminal », `tests/fixtures/copilot/tui-hooks.jsonl`)
//! do not say their event: `escouade-hook <event>` names it (`tui::EVENT_KEY`). A permission waits
//! once `notification` says so (`permission_prompt`), after `permissionRequest` told what it is for;
//! a question, a form and the end of a plan wait from their event in the log.

use super::*;
use std::path::Path;

impl AgentRt {
    /// One entry of the log its Copilot interface writes.
    pub(crate) fn copilot_log(&mut self, e: &Value, fx: &mut Effects) {
        // Written since it started: it is past its question about the folder.
        if let Some(run) = self.tui.as_mut() {
            run.spoke = true;
        }
        let Some(kind) = e["type"].as_str() else {
            return;
        };
        let own = e["agentId"].is_null();
        if own && (kind.starts_with("user.") || kind.starts_with("assistant.")) {
            self.unwritten_session = None;
        }
        let d = &e["data"];
        match kind {
            "user.message" if own => self.copilot_typed(e, fx),
            // Answered in its interface, never by Escouade: what waits for the user is told by
            // its hook (a permission) or by its event (the rest).
            "permission.requested" => {}
            "user_input.requested" => {
                let body = d["question"].as_str().unwrap_or_default().to_string();
                self.tui_wait(body, fx);
            }
            "elicitation.requested" => {
                let body = d["message"].as_str().unwrap_or_default().to_string();
                self.tui_wait(body, fx);
            }
            "exit_plan_mode.requested" => {
                let plan = d["planContent"].as_str().or(d["summary"].as_str());
                let body = notify::permission_body(
                    "ExitPlanMode",
                    &json!({ "plan": plan }),
                    &self.meta.cwd,
                );
                self.tui_wait(body, fx);
            }
            "permission.completed"
            | "user_input.completed"
            | "elicitation.completed"
            | "exit_plan_mode.completed" => {
                self.copilot.asked = None;
                if self.meta.status == AgentStatus::Waiting {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            // Each call of the model, with its usage and its cost (`assistant.usage` is not kept).
            "session.usage_record" => {
                let usage = json!({ "type": "assistant.usage", "data": d["usage"], "agentId": e["agentId"] });
                self.cp_event(&usage, fx);
            }
            // Before any turn (its start: a model of `--model` it does not have, and takes `auto`
            // instead, tried on a Free account): said, never the error of the next turn.
            "session.error" if own && !self.meta.status.is_active() => {
                let message = d["message"].as_str().unwrap_or_default().trim();
                if !message.is_empty() {
                    self.notice("warn", message.to_string(), fx);
                }
            }
            // Escape: the turn is over, as interrupted (no `agentStop` comes for it).
            "abort" if own => {
                if self.meta.status.is_active() {
                    self.copilot.aborted = true;
                    self.copilot_tui_end(fx);
                }
            }
            _ => self.cp_event(e, fx),
        }
        fx.agent_changed = true;
    }

    /// What the user typed in the interface (or Escouade pasted there): their message.
    fn copilot_typed(&mut self, e: &Value, fx: &mut Effects) {
        let d = &e["data"];
        let text = d["content"].as_str().unwrap_or_default().to_string();
        let images = d["attachments"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|a| a["type"] == "file" || a["type"] == "image")
            .count() as u32;
        if text.trim().is_empty() && images == 0 {
            return;
        }
        let id = d["messageId"]
            .as_str()
            .or(e["id"].as_str())
            .map(str::to_string)
            .unwrap_or_else(new_id);
        if self.conv.contains(&id) {
            return;
        }
        let item = json!({
            "kind": "user", "id": id, "text": text, "images": images, "ts": now_ms(),
            "queued": false, "origin": "terminal",
        });
        self.meta.resume_at = None;
        self.append(item, fx);
        if self.tui_prompt.is_none() {
            self.turn_text.clear();
            self.final_text.clear();
            self.interrupted = false;
            if self.meta.plan.as_ref().is_some_and(PlanState::is_finished) {
                self.reset_plan(fx);
            }
        }
        if !self.meta.status.is_active() {
            self.set_status(AgentStatus::Running, fx);
        }
        self.meta.prompts += 1;
        self.meta.last_activity = now_ms();
        self.tui_prompt = Some(text);
        fx.agent_changed = true;
        fx.save = true;
    }

    /// One of the hooks its Copilot interface runs (`escouade-hook <event>`,
    /// `engine/copilot/terminal.rs`).
    pub(crate) fn copilot_hook(&mut self, h: &Value, fx: &mut Effects) -> HookOutcome {
        let mut out = HookOutcome::default();
        let event = h[crate::tui::EVENT_KEY].as_str().unwrap_or_default();
        if event != "sessionEnd" {
            if let Some(run) = self.tui.as_mut() {
                run.spoke = true;
            }
        }
        // The end of a session tells nothing of the next one.
        let sid = h["sessionId"].as_str().filter(|_| event != "sessionEnd");
        if let Some(sid) = sid {
            let known = self.meta.session_id.clone();
            if known.as_deref() != Some(sid) {
                // Another session of the interface (`/new`, `/resume`): another conversation, whose
                // log is in its own folder, beside the one read so far.
                if known.is_some() {
                    self.reset_plan(fx);
                    self.notice(
                        "info",
                        tr!(
                            "Nouvelle conversation Copilot (contexte vidé)",
                            "New Copilot conversation (context cleared)"
                        ),
                        fx,
                    );
                }
                let log = self
                    .tui
                    .as_ref()
                    .and_then(|t| t.transcript.as_deref())
                    .and_then(|p| p.parent()?.parent().map(Path::to_path_buf))
                    .map(|states| states.join(sid).join("events.jsonl"));
                self.copilot_new_conversation();
                self.meta.session_id = Some(sid.to_string());
                self.meta.last_entry = None;
                self.meta.fork_of = None;
                self.meta.fork_at = None;
                self.unwritten_session = None;
                fx.save = true;
                out.transcript = h["transcriptPath"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| log.map(|p| p.to_string_lossy().into_owned()));
            }
        }
        match event {
            "userPromptSubmitted" => {
                if !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            // What it is about to ask: told once it waits for the answer.
            "permissionRequest" => {
                let tool = h["toolName"].as_str().unwrap_or_default();
                let (name, input) = copilot::tool_of(tool, &h["toolInput"], None, &self.meta.cwd);
                self.copilot.asked = Some(notify::permission_body(&name, &input, &self.meta.cwd));
            }
            "notification" => {
                let kind = h["notification_type"]
                    .as_str()
                    .or(h["notificationType"].as_str());
                if kind == Some("permission_prompt") {
                    let body =
                        self.copilot.asked.take().unwrap_or_else(|| {
                            h["message"].as_str().unwrap_or_default().to_string()
                        });
                    self.tui_wait(body, fx);
                }
            }
            "postToolUse" | "postToolUseFailure" => {
                if self.meta.status == AgentStatus::Waiting {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "agentStop" => out.stopped = true,
            _ => {}
        }
        fx.agent_changed = true;
        out
    }

    /// The running turn of its Copilot interface is over: after its `agentStop`, its interface
    /// gone, or Escape (`abort`).
    pub(super) fn copilot_tui_end(&mut self, fx: &mut Effects) {
        if !self.meta.status.is_active() {
            return;
        }
        self.copilot.asked = None;
        self.cp_turn_end(fx);
        self.tui_prompt = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = include_str!("../../../tests/fixtures/copilot/tui-events.jsonl");
    const HOOKS: &str = include_str!("../../../tests/fixtures/copilot/tui-hooks.jsonl");
    const SESSION: &str = "0a108c31-29e9-484a-8014-52a9fd3eaa88";

    fn agent(session: Option<&str>) -> AgentRt {
        let dir = std::env::temp_dir().join(format!("escouade-copilot-tui-{}", new_id()));
        let meta = AgentMeta {
            id: "a1".into(),
            engine: EngineId::COPILOT,
            cwd: "C:\\Users\\user\\AppData\\Local\\Temp\\esc23\\wt-agent1".into(),
            session_id: session.map(str::to_string),
            ..Default::default()
        };
        let mut a = AgentRt::new(meta, &dir);
        a.start_tui();
        a
    }

    fn lines(text: &str) -> Vec<Value> {
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .filter(|l: &Value| l["dir"] != "note")
            .collect()
    }

    /// A hook as `escouade-hook <event>` posts it: its payload, its event named.
    fn hook(event: &str, payload: Value) -> Value {
        let mut h = payload;
        h[crate::tui::EVENT_KEY] = json!(event);
        h
    }

    #[test]
    fn a_real_log_gives_the_conversation_and_escape_ends_its_turn() {
        let mut a = agent(Some(SESSION));
        let mut fx = Effects::default();
        let mut ends = Vec::new();
        for e in lines(LOG) {
            // The first turn's end, as the core makes it after its `agentStop`.
            if e["type"] == "hook.start" && e["data"]["hookType"] == "agentStop" {
                a.end_tui_turn(None, &mut fx);
            }
            a.copilot_log(&e, &mut fx);
            ends.extend(fx.turn_end.take());
        }
        let items = a.conv.items();
        let users: Vec<&str> = items
            .iter()
            .filter(|i| i["kind"] == "user")
            .filter_map(|i| i["text"].as_str())
            .collect();
        assert_eq!(
            users,
            [
                "Dis bonjour en deux mots, sans outil.",
                "Lance exactement `Start-Sleep -Seconds 60; echo fini` avec powershell."
            ]
        );
        assert!(items
            .iter()
            .all(|i| i["kind"] != "user" || i["origin"] == "terminal"));
        let texts: Vec<&str> = items
            .iter()
            .filter(|i| i["kind"] == "text")
            .filter_map(|i| i["text"].as_str())
            .collect();
        assert_eq!(texts, ["Bonjour !"]);
        let bash = items.iter().find(|i| i["name"] == "Bash").unwrap();
        assert_eq!(
            bash["input"]["command"],
            "Start-Sleep -Seconds 60; echo fini"
        );
        // A finished turn, then one interrupted by Escape.
        assert_eq!(ends.len(), 2, "{ends:?}");
        assert!(matches!(&ends[0], TurnEnd::Finished(t) if t == "Bonjour !"));
        assert_eq!(ends[1], TurnEnd::Interrupted);
        let turns: Vec<&Value> = items.iter().filter(|i| i["kind"] == "turn").collect();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[1]["interrupted"], true);
        // Their usage and cost, from each call's record.
        assert!(turns[0]["tokens"].as_u64().unwrap() > 0);
        assert!(a.meta.cost > 0.0);
        assert_eq!(a.meta.prompts, 2);
        assert_eq!(a.meta.status, AgentStatus::Done);
        // No card: the permission was answered in the interface.
        assert!(!items.iter().any(|i| i["kind"] == "permission"));
        // Where a copy of it would start: its last event of its own.
        assert!(a.meta.last_entry.is_some());
    }

    #[test]
    fn its_hooks_tell_when_it_works_waits_and_stops() {
        let mut a = agent(Some(SESSION));
        let mut fx = Effects::default();
        let mut seen = Vec::new();
        for line in lines(HOOKS) {
            let Some(event) = line["hook"].as_str().and_then(|h| h.strip_prefix("dir:")) else {
                continue;
            };
            let out = a.copilot_hook(&hook(event, line["input"].clone()), &mut fx);
            seen.push((
                event.to_string(),
                a.meta.status,
                fx.notify.take().map(|n| n.body),
                out,
            ));
        }
        let at = |name: &str| seen.iter().rev().find(|s| s.0 == name).unwrap();
        assert_eq!(at("userPromptSubmitted").1, AgentStatus::Running);
        // Not yet waiting when it says what it will ask, but once it waits for the answer.
        assert_eq!(at("permissionRequest").1, AgentStatus::Running);
        let asked = at("notification");
        assert_eq!(asked.1, AgentStatus::Waiting);
        assert!(
            asked
                .2
                .as_deref()
                .is_some_and(|b| b.contains("Start-Sleep -Seconds 60")),
            "{asked:?}"
        );
        // The end of another session (the first lines, that of the run before) changes nothing.
        assert_eq!(a.meta.session_id.as_deref(), Some(SESSION));
        assert!(!a.conv.items().iter().any(|i| i["kind"] == "notice"));
        let stop = a.copilot_hook(
            &hook(
                "agentStop",
                json!({ "sessionId": SESSION, "stopReason": "end_turn" }),
            ),
            &mut fx,
        );
        assert!(stop.stopped);
        a.copilot_hook(
            &hook("postToolUse", json!({ "sessionId": SESSION })),
            &mut fx,
        );
        assert_eq!(a.meta.status, AgentStatus::Running);
    }

    #[test]
    fn another_session_in_a_hook_is_read_from_its_own_log() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        let log = "/home/u/.copilot/session-state/s2/events.jsonl";
        let out = a.copilot_hook(
            &hook(
                "agentStop",
                json!({ "sessionId": "s2", "transcriptPath": log }),
            ),
            &mut fx,
        );
        assert_eq!(out.transcript.as_deref(), Some(log));
        assert!(out.stopped);
        assert_eq!(a.meta.session_id.as_deref(), Some("s2"));
        assert!(a.conv.items().iter().any(|i| i["kind"] == "notice"));
        // The same session again: nothing more.
        let out = a.copilot_hook(
            &hook("userPromptSubmitted", json!({ "sessionId": "s2" })),
            &mut fx,
        );
        assert_eq!(out.transcript, None);
    }

    #[test]
    fn an_error_before_any_turn_is_a_notice_not_the_next_turns_end() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        let ev = |kind: &str, data: Value| json!({ "type": kind, "data": data, "id": new_id() });
        // As the real interface writes it at its start on a Free account (ESC-48).
        a.copilot_log(
            &ev(
                "session.error",
                json!({ "message": "Model \"gpt-6-luna\" from --model flag is not available. Using \"auto\" instead.", "errorType": "model" }),
            ),
            &mut fx,
        );
        assert!(a.conv.items().iter().any(|i| i["kind"] == "notice"
            && i["text"]
                .as_str()
                .is_some_and(|t| t.contains("not available"))));
        a.copilot_log(
            &ev(
                "user.message",
                json!({ "content": "Va", "messageId": "m1" }),
            ),
            &mut fx,
        );
        a.copilot_log(
            &ev(
                "assistant.message",
                json!({ "messageId": "r1", "content": "A" }),
            ),
            &mut fx,
        );
        a.end_tui_turn(None, &mut fx);
        assert!(
            matches!(fx.turn_end, Some(TurnEnd::Finished(ref t)) if t == "A"),
            "{:?}",
            fx.turn_end
        );
        assert_eq!(a.meta.status, AgentStatus::Done);
    }

    #[test]
    fn a_question_waits_from_its_event_and_its_answer_goes_on() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        let ev = |kind: &str, data: Value| json!({ "type": kind, "data": data, "id": new_id() });
        a.copilot_log(
            &ev(
                "user.message",
                json!({ "content": "Va", "messageId": "m1" }),
            ),
            &mut fx,
        );
        a.copilot_log(
            &ev(
                "user_input.requested",
                json!({ "requestId": "r1", "question": "Quelle couleur ?" }),
            ),
            &mut fx,
        );
        assert_eq!(a.meta.status, AgentStatus::Waiting);
        assert_eq!(
            fx.notify.take().map(|n| n.body).as_deref(),
            Some("Quelle couleur ?")
        );
        a.copilot_log(
            &ev(
                "user_input.completed",
                json!({ "requestId": "r1", "answer": "Bleu" }),
            ),
            &mut fx,
        );
        assert_eq!(a.meta.status, AgentStatus::Running);
        // Read again from the start of the file: its message is listed once.
        a.copilot_log(
            &ev(
                "user.message",
                json!({ "content": "Va", "messageId": "m1" }),
            ),
            &mut fx,
        );
        let users = a
            .conv
            .items()
            .iter()
            .filter(|i| i["kind"] == "user")
            .count();
        assert_eq!(users, 1);
        // Its interface gone: the turn is over, as interrupted.
        a.stop_tui(&mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Interrupted));
    }
}
