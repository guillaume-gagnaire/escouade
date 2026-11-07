//! « Vue Codex »: the agent's conversation as Codex's own interface (in a terminal) writes it in
//! its session's rollout (`sessions/…/rollout-<date>-<session>.jsonl`), which its plugin translates
//! (`view/translate`, `plugins/codex/src/view.rs`), and what its hooks tell while it runs
//! (`tui`, `engine/codex/terminal.rs`).
//!
//! Its hooks have the payloads of Claude Code's (docs/TODO-1.8.md, point 0, « Vue terminal »,
//! `tests/fixtures/codex/hooks-payloads.jsonl`): `UserPromptSubmit`, `PermissionRequest`,
//! `PostToolUse`, `Stop`, and `Interrupt` on Escape.

use super::*;

impl AgentRt {
    /// One entry of the rollout its Codex interface writes.
    #[cfg(test)]
    pub(crate) fn codex_rollout(&mut self, e: &Value, fx: &mut Effects) {
        self.view_line(e, fx);
    }

    /// One of the hooks its Codex interface runs (`escouade-hook`, `engine/codex/terminal.rs`).
    pub(crate) fn codex_hook(&mut self, h: &Value, fx: &mut Effects) -> HookOutcome {
        let mut out = HookOutcome::default();
        if let Some(sid) = h["session_id"].as_str() {
            let known = self.meta.session_id.clone();
            if known.as_deref() != Some(sid) {
                // A new session of the interface tells its id by its first hook; another one later
                // (`/new`, `/resume`) is another conversation, whose rollout is another file.
                if known.is_some() {
                    self.reset_plan(fx);
                    self.notice(
                        "info",
                        tr!(
                            "Nouvelle conversation Codex (contexte vidé)",
                            "New Codex conversation (context cleared)"
                        ),
                        fx,
                    );
                }
                self.view_reset();
                self.meta.session_id = Some(sid.to_string());
                self.meta.last_entry = None;
                self.meta.fork_of = None;
                self.meta.fork_at = None;
                self.unwritten_session = None;
                fx.save = true;
                out.transcript = h["transcript_path"].as_str().map(str::to_string);
            }
        }
        match h["hook_event_name"].as_str().unwrap_or_default() {
            "UserPromptSubmit" => {
                // Told once the file had told that turn's end (the hook waited for the agent, under load): over already.
                let over = self.view.turn.is_none()
                    && h["turn_id"].as_str().is_some()
                    && h["turn_id"].as_str() == self.view.last_turn.as_deref();
                if !over && !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "PermissionRequest" => {
                let tool = h["tool_name"].as_str().unwrap_or_default();
                let body = notify::permission_body(tool, &h["tool_input"], &self.meta.cwd);
                self.tui_wait(body, fx);
            }
            "PostToolUse" => {
                if self.meta.status == AgentStatus::Waiting {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "Stop" => out.stopped = true,
            // Escape in the interface: the rollout tells the turn interrupted right after.
            "Interrupt" => {
                if self.meta.status.is_active() {
                    self.interrupted = true;
                }
                out.stopped = true;
            }
            _ => {}
        }
        fx.agent_changed = true;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROLLOUT: &str = include_str!("../../../tests/fixtures/codex/tui-rollout.jsonl");
    const HOOKS: &str = include_str!("../../../tests/fixtures/codex/hooks-payloads.jsonl");
    const SESSION: &str = "01a1466d-084c-71a3-acb9-dadb322d905f";

    fn agent(session: Option<&str>) -> AgentRt {
        let dir = std::env::temp_dir().join(format!("escouade-codex-tui-{}", new_id()));
        let meta = AgentMeta {
            id: "a1".into(),
            engine: EngineId::CODEX,
            cwd: "/work/demo".into(),
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
            .collect()
    }

    #[test]
    fn a_real_rollout_gives_the_conversation_and_its_turns() {
        let mut a = agent(Some(SESSION));
        let mut fx = Effects::default();
        let mut ends = Vec::new();
        for e in lines(ROLLOUT) {
            a.codex_rollout(&e, &mut fx);
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
                "Réponds seulement : A",
                "Lance la commande `Start-Sleep -Seconds 60` puis réponds B."
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
        assert_eq!(
            texts,
            ["A", "Je lance la commande demandée, puis je répondrai B."]
        );
        // A finished turn, then one interrupted by Escape; the command the interruption cut short
        // is told after the turn's end, and left out.
        assert_eq!(ends.len(), 2, "{ends:?}");
        assert!(matches!(&ends[0], TurnEnd::Finished(t) if t == "A"));
        assert_eq!(ends[1], TurnEnd::Interrupted);
        assert!(!items.iter().any(|i| i["kind"] == "tool"));
        let turns: Vec<&Value> = items.iter().filter(|i| i["kind"] == "turn").collect();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0]["durationMs"], 4087);
        assert_eq!(turns[1]["interrupted"], true);
        assert_eq!(a.meta.prompts, 2);
        assert_eq!(a.meta.status, AgentStatus::Done);
        // Its tokens and its window, from its `token_count`s.
        assert!(a.meta.tokens > 0);
        assert_eq!(a.context_window, 258_400);
        assert!(fx.rate.is_some());
        assert_eq!(a.tui_prompt, None);
    }

    #[test]
    fn a_command_and_a_change_of_a_turn_show_as_their_tools() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        let ev = |payload: Value| json!({ "type": "event_msg", "payload": payload });
        for e in [
            ev(json!({ "type": "task_started", "turn_id": "t1" })),
            ev(
                json!({ "type": "item_completed", "thread_id": "s1", "turn_id": "t1",
                "item": { "type": "UserMessage", "id": "u1", "content": [{ "type": "text", "text": "Va" }] } }),
            ),
            ev(
                json!({ "type": "item_completed", "thread_id": "s1", "turn_id": "t1",
                "item": { "type": "CommandExecution", "id": "c1", "command": ["sh", "-c", "ls"],
                          "parsed_cmd": [{ "type": "list_files", "cmd": "ls" }], "status": "completed",
                          "aggregated_output": "a.txt", "exit_code": 0 } }),
            ),
            ev(
                json!({ "type": "item_completed", "thread_id": "s1", "turn_id": "t1",
                "item": { "type": "FileChange", "id": "f1",
                          "changes": { "a.txt": { "type": "add", "content": "x\n" } } } }),
            ),
            ev(json!({ "type": "task_complete", "turn_id": "t1", "duration_ms": 5 })),
        ] {
            a.codex_rollout(&e, &mut fx);
        }
        let items = a.conv.items();
        let bash = items.iter().find(|i| i["name"] == "Bash").unwrap();
        assert_eq!(bash["input"]["command"], "ls");
        assert_eq!(bash["status"], "ok");
        let write = items.iter().find(|i| i["name"] == "Write").unwrap();
        assert_eq!(write["input"]["file_path"], "a.txt");
        assert_eq!(a.meta.status, AgentStatus::Done);
        // Read again (from the start of the file): its message is listed once.
        a.codex_rollout(
            &ev(json!({ "type": "item_completed", "thread_id": "s1", "turn_id": "t1",
                "item": { "type": "UserMessage", "id": "u1", "content": [{ "type": "text", "text": "Va" }] } })),
            &mut fx,
        );
        let users = a
            .conv
            .items()
            .iter()
            .filter(|i| i["kind"] == "user")
            .count();
        assert_eq!(users, 1);
    }

    #[test]
    fn a_turn_the_core_ended_after_its_stop_is_not_ended_again_by_the_rollout() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        let ev = |payload: Value| json!({ "type": "event_msg", "payload": payload });
        a.codex_rollout(
            &ev(json!({ "type": "task_started", "turn_id": "t1" })),
            &mut fx,
        );
        a.codex_hook(
            &json!({ "hook_event_name": "Stop", "session_id": "s1" }),
            &mut fx,
        );
        // The end the rollout had not told yet, made by the core after a moment.
        a.end_tui_turn(None, &mut fx);
        assert!(matches!(fx.turn_end.take(), Some(TurnEnd::Finished(_))));
        a.codex_rollout(
            &ev(json!({ "type": "task_complete", "turn_id": "t1", "duration_ms": 5 })),
            &mut fx,
        );
        assert_eq!(fx.turn_end, None);
        let turns = a
            .conv
            .items()
            .iter()
            .filter(|i| i["kind"] == "turn")
            .count();
        assert_eq!(turns, 1);
    }

    #[test]
    fn the_end_of_an_earlier_turn_read_late_does_not_end_the_one_that_runs() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        let ev = |payload: Value| json!({ "type": "event_msg", "payload": payload });
        a.codex_rollout(
            &ev(json!({ "type": "task_started", "turn_id": "t1" })),
            &mut fx,
        );
        a.codex_hook(
            &json!({ "hook_event_name": "Stop", "session_id": "s1" }),
            &mut fx,
        );
        // Its end not written yet (its Stop hook slow to quit, under load): ended by the core.
        a.end_tui_turn(None, &mut fx);
        fx.turn_end.take();
        // The next message's turn starts, then the first one's end is read.
        a.codex_rollout(
            &ev(json!({ "type": "task_started", "turn_id": "t2" })),
            &mut fx,
        );
        a.codex_rollout(
            &ev(json!({ "type": "task_complete", "turn_id": "t1", "duration_ms": 5 })),
            &mut fx,
        );
        assert_eq!(fx.turn_end, None);
        assert_eq!(a.meta.status, AgentStatus::Running);
        a.codex_rollout(
            &ev(json!({ "type": "turn_aborted", "turn_id": "t2", "reason": "interrupted" })),
            &mut fx,
        );
        assert_eq!(fx.turn_end, Some(TurnEnd::Interrupted));
    }

    #[test]
    fn the_end_the_core_makes_after_a_stop_is_that_of_its_turn_alone() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        let ev = |payload: Value| json!({ "type": "event_msg", "payload": payload });
        a.codex_rollout(
            &ev(json!({ "type": "task_started", "turn_id": "t1" })),
            &mut fx,
        );
        a.codex_hook(
            &json!({ "hook_event_name": "Stop", "session_id": "s1" }),
            &mut fx,
        );
        let stopped = a.tui_turn_mark();
        // Its end read, then the next message's turn started, before the core's moment is up.
        a.codex_rollout(
            &ev(json!({ "type": "task_complete", "turn_id": "t1", "duration_ms": 5 })),
            &mut fx,
        );
        fx.turn_end.take();
        a.codex_rollout(
            &ev(json!({ "type": "task_started", "turn_id": "t2" })),
            &mut fx,
        );
        a.end_stopped_tui_turn(&stopped, &mut fx);
        assert_eq!(fx.turn_end, None);
        assert_eq!(a.meta.status, AgentStatus::Running);
        // That of its own turn, whose end never came: made.
        a.codex_hook(
            &json!({ "hook_event_name": "Stop", "session_id": "s1" }),
            &mut fx,
        );
        let stopped = a.tui_turn_mark();
        a.end_stopped_tui_turn(&stopped, &mut fx);
        assert!(matches!(fx.turn_end, Some(TurnEnd::Finished(_))));
    }

    #[test]
    fn a_hook_told_late_does_not_start_again_a_turn_its_file_ended() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        let ev = |payload: Value| json!({ "type": "event_msg", "payload": payload });
        let submit = |turn: &str| json!({ "hook_event_name": "UserPromptSubmit", "session_id": "s1", "turn_id": turn });
        a.codex_rollout(
            &ev(json!({ "type": "task_started", "turn_id": "t1" })),
            &mut fx,
        );
        a.codex_rollout(
            &ev(json!({ "type": "task_complete", "turn_id": "t1", "duration_ms": 5 })),
            &mut fx,
        );
        assert_eq!(a.meta.status, AgentStatus::Done);
        a.codex_hook(&submit("t1"), &mut fx);
        assert_eq!(a.meta.status, AgentStatus::Done, "that turn is over");
        // The next turn's, told before the file tells it: a turn starts.
        a.codex_hook(&submit("t2"), &mut fx);
        assert_eq!(a.meta.status, AgentStatus::Running);
    }

    #[test]
    fn the_hooks_tell_when_it_runs_waits_and_stops() {
        let mut a = agent(None);
        let mut fx = Effects::default();
        let mut seen = Vec::new();
        for line in lines(HOOKS) {
            let h = &line["payload"];
            let out = a.codex_hook(h, &mut fx);
            seen.push((
                line["event"].as_str().unwrap().to_string(),
                a.meta.status,
                fx.notify.take().map(|n| n.body),
                out,
            ));
        }
        // Its first hook names its session.
        assert_eq!(
            a.meta.session_id.as_deref(),
            Some("01a14665-43f3-7312-8d1f-f22f147f2d24")
        );
        let at = |name: &str| seen.iter().find(|s| s.0 == name).unwrap();
        assert_eq!(at("UserPromptSubmit").1, AgentStatus::Running);
        let asked = at("PermissionRequest");
        assert_eq!(asked.1, AgentStatus::Waiting);
        assert!(
            asked
                .2
                .as_deref()
                .is_some_and(|b| b.contains("console.log(42)")),
            "{asked:?}"
        );
        assert_eq!(at("PostToolUse").1, AgentStatus::Running);
        assert!(at("Stop").3.stopped);
        // No notice of a new conversation for the first session.
        assert!(!a.conv.items().iter().any(|i| i["kind"] == "notice"));
    }

    #[test]
    fn another_session_in_a_hook_is_another_conversation_read_from_its_rollout() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        let h = json!({
            "hook_event_name": "SessionStart", "session_id": "s2",
            "transcript_path": "/home/u/.codex/sessions/2026/10/17/rollout-x-s2.jsonl",
        });
        let out = a.codex_hook(&h, &mut fx);
        assert_eq!(
            out.transcript.as_deref(),
            Some("/home/u/.codex/sessions/2026/10/17/rollout-x-s2.jsonl")
        );
        assert_eq!(a.meta.session_id.as_deref(), Some("s2"));
        assert!(a.conv.items().iter().any(|i| i["kind"] == "notice"));
    }

    #[test]
    fn escape_ends_the_turn_as_interrupted_and_its_interface_gone_too() {
        let mut a = agent(Some("s1"));
        let mut fx = Effects::default();
        a.codex_hook(
            &json!({ "hook_event_name": "UserPromptSubmit", "session_id": "s1" }),
            &mut fx,
        );
        let out = a.codex_hook(
            &json!({ "hook_event_name": "Interrupt", "session_id": "s1" }),
            &mut fx,
        );
        assert!(out.stopped);
        // The rollout said nothing of it: the end comes from the core, after a moment.
        a.end_tui_turn(None, &mut fx);
        assert_eq!(fx.turn_end.take(), Some(TurnEnd::Interrupted));
        assert!(!a.meta.status.is_active());

        a.codex_hook(
            &json!({ "hook_event_name": "UserPromptSubmit", "session_id": "s1" }),
            &mut fx,
        );
        a.stop_tui(&mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Interrupted));
        assert!(!a.meta.status.is_active());
    }
}
