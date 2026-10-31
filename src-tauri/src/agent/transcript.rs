//! « Vue Claude Code »: the agent's conversation as Claude Code's own interface (its TUI, in a
//! terminal) writes it in its transcript, and what its hooks tell while it runs (`tui`).
//!
//! The transcript's `assistant` and `user` entries have the shape of the stream's frames: they go
//! through the same handlers. What the stream alone tells is made up from the rest: a turn ends
//! with its `system`/`turn_duration` entry (or the marker of an interruption), and its cost is
//! worked out from the usage of its API messages at list prices, the interface reporting none per
//! turn (`cost-state` comes when it exits).

use super::*;

/// What the core does after a hook (`AgentRt::on_hook`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HookOutcome {
    /// The conversation goes on in another session (`/clear`, `/resume`): its transcript is
    /// read from the start of this file.
    pub transcript: Option<String>,
    /// The turn may be over (`Stop`): the transcript is read again soon, and the turn ended if
    /// it still did not say so.
    pub stopped: bool,
}

/// The texts Claude Code writes as the user's when a turn is interrupted (also in a subagent's
/// transcript: `sidechain`).
pub(super) const INTERRUPTED: &[&str] = &[
    "[Request interrupted by user",
    "[Request interrupted by user for tool use",
];

impl AgentRt {
    /// Its Claude Code interface starts: what was counted of the one before is gone with it.
    pub fn start_tui(&mut self) {
        self.meta.last_activity = now_ms();
        self.blocks.clear();
        self.current_msg.clear();
        self.pending.clear();
        self.last_usage.clear();
        self.live.clear();
        self.tui_usage.clear();
        self.tui_error = None;
        self.tui_exit = None;
        self.forget_turn();
        self.interrupted = false;
        self.queued = 0;
        self.foreground_tasks.clear();
        self.announced_tasks.clear();
        self.interrupt_plan();
        self.codex_new_conversation();
        self.copilot_new_conversation();
        self.cursor_new_conversation();
        self.opencode_new_conversation();
        // It tells the state of its link to claude.ai itself, once it has one.
        self.remote_state = None;
    }

    /// Claude Code could not find the session the agent was to resume (or the original's, for a
    /// copy): it is given up, and the next start is a new session.
    pub(crate) fn forget_session(&mut self) {
        self.meta.session_id = None;
        self.meta.last_entry = None;
        self.meta.fork_of = None;
        self.meta.fork_at = None;
        self.unwritten_session = None;
    }

    /// One entry of the transcript its Claude Code interface writes.
    pub fn on_transcript(&mut self, e: &Value, fx: &mut Effects) {
        // A subagent's entries are in files of their own; one here would be told as the agent's.
        if e["isSidechain"] == true {
            return;
        }
        // Something of its session is written: its absence now would be a loss.
        if matches!(e["type"].as_str(), Some("user" | "assistant")) {
            self.unwritten_session = None;
        }
        match e["type"].as_str().unwrap_or("") {
            "user" => self.on_tui_user(e, fx),
            "assistant" => self.on_tui_assistant(e, fx),
            "system" => match e["subtype"].as_str().unwrap_or("") {
                "turn_duration" => self.end_tui_turn(e["durationMs"].as_u64(), fx),
                "compact_boundary" => self.handle_frame(e, fx),
                // A local command's output, failed ones' (2.1.302: `/compact` that fails).
                "local_command" => {
                    let failed = e["commandOutcome"]["kind"] == "failed";
                    self.on_local_command(
                        e["uuid"].as_str(),
                        e["content"].as_str().unwrap_or(""),
                        failed,
                        fx,
                    )
                }
                // Started with Remote Control, the interface tells the link of its session.
                "bridge_status" => self.on_bridge_status(e, fx),
                _ => {}
            },
            "bridge-session" => {
                if let Some(run) = self.tui.as_mut() {
                    run.spoke = true;
                }
                if e["bridgeSessionId"].as_str().is_some_and(|s| !s.is_empty()) {
                    self.interface_linked(fx);
                }
            }
            // « Shift+Tab » in the interface: the agent's mode is the one it shows.
            "permission-mode" => {
                if let Some(mode) = e["permissionMode"].as_str() {
                    if self.meta.set_mode(mode) {
                        fx.agent_changed = true;
                        fx.save = true;
                    }
                }
            }
            _ => {}
        }
    }

    /// The interface tells how the link of its session to claude.ai went (`bridge_status`): at
    /// `url`, or, with none, that it did not open (a network, a proxy, a rule of the
    /// organization). Either way it is up.
    fn on_bridge_status(&mut self, e: &Value, fx: &mut Effects) {
        let mut unasked = true;
        if let Some(run) = self.tui.as_mut() {
            run.spoke = true;
            // Only what it was asked for is news, and only until a link was seen.
            unasked = !run.remote || run.linked;
        }
        match e["url"].as_str() {
            Some(url) => self.on_interface_link(url, fx),
            None if unasked => {}
            None => {
                let said: String = e["content"]
                    .as_str()
                    .unwrap_or_default()
                    .trim()
                    .chars()
                    .take(200)
                    .collect();
                let text = if said.is_empty() {
                    tr!(
                        "Remote control indisponible : Claude Code n’a pas pu ouvrir la session distante.",
                        "Remote control unavailable: Claude Code could not open the remote session."
                    )
                } else {
                    tr!(
                        "Remote control indisponible : Claude Code n’a pas pu ouvrir la session distante (« {said} »).",
                        "Remote control unavailable: Claude Code could not open the remote session (“{said}”).",
                        said = said
                    )
                };
                self.interface_unlinked(text, fx);
            }
        }
    }

    /// The interface's session is linked to claude.ai at `url` (`bridge_status`). Its session is
    /// its own: the stream's (`meta.remote_session`) is left alone.
    fn on_interface_link(&mut self, url: &str, fx: &mut Effects) {
        self.interface_linked(fx);
        if self.meta.remote_url.as_deref() != Some(url) {
            self.meta.remote_url = Some(url.to_string());
            fx.agent_changed = true;
            fx.save = true;
        }
    }

    /// A sign says the interface's session is linked to claude.ai: connected, whatever the wait
    /// for one had said, and what fails after that is news again.
    pub(crate) fn interface_linked(&mut self, fx: &mut Effects) {
        if let Some(run) = self.tui.as_mut() {
            run.linked = true;
        }
        self.remote_told = false;
        if self.remote_state.as_deref() != Some("connected") {
            self.remote_state = Some("connected".into());
            fx.agent_changed = true;
        }
    }

    /// The interface, up, started with Remote Control, has no link to claude.ai: unavailable, and
    /// the agent is told, once. Remote Control stays in force for the agent: its next start tries
    /// again.
    pub(crate) fn interface_unlinked(&mut self, text: String, fx: &mut Effects) {
        if self.remote_state.as_deref() != Some("unavailable") {
            self.remote_state = Some("unavailable".into());
            fx.agent_changed = true;
        }
        self.remote_unavailable_notice(text, fx);
    }

    /// Its interface takes what is typed in it: a window showed it (the user sees the screen), or
    /// it is up, past its welcome screen and the box about the folder, where a text typed would
    /// choose an option. Claude Code 2.1.301 has no hook for that (its HTTP hooks are skipped for
    /// SessionStart); it says so by the registry of its sessions, which lists the interface
    /// (`tui::find_registered`), or by a `bridge_status` or `bridge-session` entry of its
    /// transcript, written since it started.
    pub fn interface_ready(&self) -> bool {
        self.tui
            .as_ref()
            .is_some_and(|t| t.output.shown() || t.spoke || t.registered.is_some())
    }

    fn on_tui_user(&mut self, e: &Value, fx: &mut Effects) {
        // Claude Code's own (the caveat of a local command, a compaction's summary).
        if e["isMeta"] == true || e["isCompactSummary"] == true {
            return;
        }
        let content = &e["message"]["content"];
        let blocks = content.as_array().map(Vec::as_slice).unwrap_or_default();
        if blocks.iter().any(|b| b["type"] == "tool_result") {
            let frame = json!({
                "type": "user", "message": e["message"], "uuid": e["uuid"],
                "parent_tool_use_id": null, "tool_use_result": e["toolUseResult"],
            });
            return self.on_user(&frame, fx);
        }
        let text = match content.as_str() {
            Some(s) => s.to_string(),
            None => blocks
                .iter()
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        };
        if INTERRUPTED.iter().any(|m| text.starts_with(m)) {
            if self.meta.status.is_active() {
                self.interrupted = true;
                self.end_tui_turn(None, fx);
            }
            return;
        }
        // A local command (`/model opus`, `! ls`): shown as typed, then what it printed.
        let text = match localcmd::parse(&text) {
            Some(parts) => match parts.into_iter().next() {
                Some(localcmd::Part::Typed(typed)) => typed,
                // Its caveat, Claude Code's own.
                None if text.contains("<local-command-caveat>") => return,
                _ => return self.on_local_command(e["uuid"].as_str(), &text, false, fx),
            },
            None => text,
        };
        let kind = e["origin"]["kind"].as_str();
        if kind.is_some_and(|k| k != "human") {
            // A subagent's report, a background task that ended…: as the stream tells them.
            let frame = json!({
                "type": "user", "isReplay": true, "uuid": e["uuid"], "message": e["message"],
                "origin": e["origin"],
            });
            return self.on_replay(&frame, fx);
        }
        let images = blocks.iter().filter(|b| b["type"] == "image").count() as u32;
        if text.trim().is_empty() && images == 0 {
            return;
        }
        // The same prompt again: a command, as typed then as `<command-name>`.
        let prompt_id = e["promptId"].as_str();
        if prompt_id.is_some()
            && prompt_id == self.tui_prompt_id.as_deref()
            && self.tui_prompt.as_deref() == Some(text.as_str())
        {
            return;
        }
        self.tui_prompt_id = prompt_id.map(str::to_string);
        let id = e["uuid"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(new_id);
        if self.conv.contains(&id) {
            return;
        }
        let mut item = json!({
            "kind": "user", "id": id, "text": text, "images": images, "ts": now_ms(),
            "queued": false, "origin": "terminal",
        });
        if text.trim().is_empty() {
            item["text"] = json!("");
        }
        self.meta.resume_at = None;
        self.append(item, fx);
        if self.tui_prompt.is_none() {
            self.turn_text.clear();
            self.final_text.clear();
            self.interrupted = false;
            self.tui_error = None;
            if self.meta.plan.as_ref().is_some_and(PlanState::is_finished) {
                self.reset_plan(fx);
            }
        }
        self.set_status(AgentStatus::Running, fx);
        self.meta.prompts += 1;
        self.meta.last_activity = now_ms();
        self.tui_prompt = Some(text);
        fx.agent_changed = true;
        fx.save = true;
    }

    fn on_tui_assistant(&mut self, e: &Value, fx: &mut Effects) {
        let msg = &e["message"];
        if let (Some(mid), Some(model)) = (msg["id"].as_str(), msg["model"].as_str()) {
            // Each block of a message is an entry of its own, with the message's usage.
            if model != "<synthetic>" {
                self.main_model = model.to_string();
                self.tui_usage.insert(
                    mid.to_string(),
                    (model.to_string(), Usage::from_api(&msg["usage"])),
                );
                let u = &msg["usage"];
                self.context_tokens = [
                    &u["input_tokens"],
                    &u["cache_read_input_tokens"],
                    &u["cache_creation_input_tokens"],
                ]
                .iter()
                .filter_map(|v| v.as_u64())
                .sum();
            }
        }
        let mut frame = json!({
            "type": "assistant", "message": msg, "uuid": e["uuid"], "parent_tool_use_id": null,
        });
        if e["isApiErrorMessage"] == true {
            let error = e["error"].as_str().unwrap_or("api_error");
            frame["error"] = json!(error);
            if error == "rate_limit" {
                self.rate_limited = true;
            }
            let text = msg["content"][0]["text"].as_str().unwrap_or(error);
            self.tui_error = Some(text.to_string());
        }
        if !self.meta.status.is_active() {
            self.set_status(AgentStatus::Running, fx);
        }
        self.on_assistant(&frame, fx);
        fx.agent_changed = true;
    }

    /// Which turn of its interface runs: the messages typed so far, and the id its harness gave
    /// the turn (Codex). A `Stop` takes it, for the end the core makes a moment later.
    pub(crate) fn tui_turn_mark(&self) -> (u32, Option<String>) {
        (self.meta.prompts, self.codex_turn())
    }

    /// What a local command printed (`text`, its `<local-command-stdout>` or `-stderr>`, the `!`
    /// mode's `<bash-stdout>`…, in the entry `id`): shown, and its turn over, as an error when it
    /// failed (`failed`, or its error output), the error then on the turn's card only. Claude Code
    /// ends such a turn with no `turn_duration` (2.1.302, ESC-12).
    fn on_local_command(&mut self, id: Option<&str>, text: &str, failed: bool, fx: &mut Effects) {
        let mut parts = localcmd::parse(text).unwrap_or_else(|| {
            let plain = strip_tags(text);
            let plain = (!plain.is_empty()).then_some(localcmd::Part::Output(plain));
            plain.into_iter().collect()
        });
        // Only the command's own turn: one typed while the model runs a turn leaves that turn be.
        let commands = self
            .tui_prompt
            .as_deref()
            .is_some_and(|p| p.starts_with('/') || p.starts_with("! "));
        let ends = self.meta.status.is_active() && commands;
        if ends {
            let error = |p: &localcmd::Part| match p {
                localcmd::Part::Error(t) => Some(t.clone()),
                _ => None,
            };
            let errors: Vec<String> = parts.iter().filter_map(error).collect();
            if !errors.is_empty() {
                self.tui_error = Some(errors.join("\n"));
                parts.retain(|p| error(p).is_none());
            } else if failed {
                self.tui_error = Some(strip_tags(text));
            }
        }
        self.local_output(id, &parts, fx);
        if ends {
            self.end_tui_turn(None, fx);
        }
    }

    /// The end the core makes when the transcript still did not tell that of the turn a `Stop`
    /// was for (`stopped`, its `tui_turn_mark`): only that turn's. Under load, the transcript
    /// tells its end late, and the next message's turn may have started by then.
    pub(crate) fn end_stopped_tui_turn(
        &mut self,
        stopped: &(u32, Option<String>),
        fx: &mut Effects,
    ) {
        if self.tui_turn_mark() == *stopped {
            self.end_tui_turn(None, fx);
        }
    }

    /// The running turn of its interface is over: told as the stream's result tells it, with the
    /// usage of the process so far (cumulative, as the stream's figures are).
    pub fn end_tui_turn(&mut self, duration_ms: Option<u64>, fx: &mut Effects) {
        if !self.meta.status.is_active() {
            return;
        }
        match self.meta.engine {
            EngineId::CODEX => return self.codex_tui_end(fx),
            EngineId::COPILOT => return self.copilot_tui_end(fx),
            EngineId::CURSOR | EngineId::VIBE | EngineId::AGY => return self.cursor_tui_end(fx),
            _ => {}
        }
        let mut models = serde_json::Map::new();
        for (model, u) in self.tui_usage.values() {
            let entry = models.entry(model.clone()).or_insert_with(|| {
                json!({ "inputTokens": 0, "outputTokens": 0, "cacheReadInputTokens": 0,
                        "cacheCreationInputTokens": 0, "costUSD": 0.0 })
            });
            let add = |v: &mut Value, n: u64| *v = json!(v.as_u64().unwrap_or(0) + n);
            add(&mut entry["inputTokens"], u.input);
            add(&mut entry["outputTokens"], u.output);
            add(&mut entry["cacheReadInputTokens"], u.cache_read);
            add(
                &mut entry["cacheCreationInputTokens"],
                u.write_5m + u.write_1h,
            );
            let cost = entry["costUSD"].as_f64().unwrap_or(0.0)
                + pricing::estimate(model, u).unwrap_or(0.0);
            entry["costUSD"] = json!(cost);
        }
        let error = self.tui_error.take();
        let result = json!({
            "type": "result",
            "subtype": if error.is_some() { "error_during_execution" } else { "success" },
            "is_error": error.is_some(),
            "result": error,
            "duration_ms": duration_ms,
            "modelUsage": models,
        });
        self.tui_prompt = None;
        self.on_result(&result, fx);
    }

    /// One of the hooks its Claude Code interface calls (`tui::tui_settings`).
    pub fn on_hook(&mut self, h: &Value, fx: &mut Effects) -> HookOutcome {
        let mut out = HookOutcome::default();
        if let Some(sid) = h["session_id"].as_str() {
            if self.meta.session_id.as_deref() != Some(sid) {
                // `/clear`, `/resume`: another conversation, whose transcript is another file.
                self.reset_plan(fx);
                self.notice(
                    "info",
                    tr!(
                        "Nouvelle conversation Claude (contexte vidé)",
                        "New Claude conversation (context cleared)"
                    ),
                    fx,
                );
                self.meta.session_id = Some(sid.to_string());
                self.meta.last_entry = None;
                self.meta.fork_of = None;
                self.meta.fork_at = None;
                fx.save = true;
                out.transcript = h["transcript_path"].as_str().map(str::to_string);
            }
        }
        let tool = h["tool_name"].as_str().unwrap_or("");
        match h["hook_event_name"].as_str().unwrap_or("") {
            "UserPromptSubmit" => {
                if !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "PermissionRequest" => {
                let body = notify::permission_body(tool, &h["tool_input"], &self.meta.cwd);
                self.tui_wait(body, fx);
            }
            "PreToolUse" if tool == "AskUserQuestion" => {
                let body = notify::question_body(
                    crate::engine::of(&self.meta).speaker(),
                    &h["tool_input"],
                );
                self.tui_wait(body, fx);
            }
            "Notification" if h["notification_type"] == "elicitation_dialog" => {
                let body = h["message"].as_str().unwrap_or_default().to_string();
                self.tui_wait(body, fx);
            }
            "PostToolUse" | "PostToolUseFailure" => {
                if self.meta.status == AgentStatus::Waiting {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "Stop" | "StopFailure" => out.stopped = true,
            _ => {}
        }
        fx.agent_changed = true;
        out
    }

    /// The interface waits for the user (a permission, a question): it is told, as the stream's
    /// requests are.
    pub(super) fn tui_wait(&mut self, body: String, fx: &mut Effects) {
        if self.meta.status == AgentStatus::Waiting {
            return;
        }
        self.set_status(AgentStatus::Waiting, fx);
        fx.notify = Some(AgentAlert::new(NotifyKind::Question, body));
    }

    /// Its interface is gone: a turn it was running is over, as interrupted.
    pub fn stop_tui(&mut self, fx: &mut Effects) {
        if self.meta.status.is_active() {
            self.interrupted = true;
            self.end_tui_turn(None, fx);
        }
        self.tui = None;
        self.tui_prompt = None;
        // Its link to claude.ai is over with it (the interface archives its session itself).
        self.remote_state = None;
        fx.agent_changed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSCRIPT: &str = include_str!("../../../tests/fixtures/tui-transcript.jsonl");
    const HOOKS: &str = include_str!("../../../tests/fixtures/tui-hooks.jsonl");
    /// The transcript of a real Claude Code 2.1.306 interface (ESC-138): a `Bash` with
    /// `run_in_background` (« Wait until PR 148 checks finish »), the end of the turn, then 15 s
    /// later the replayed notification and the turn Claude Code starts on it. Thinking blocks and
    /// Claude Code's own entries left out, the user's name taken out of the paths.
    const BG_COMMAND: &str = include_str!("../../../tests/fixtures/tui-background-command.jsonl");

    #[test]
    fn a_background_command_of_a_real_interface_is_listed_from_its_launch_to_its_notification() {
        use crate::plan::{BgKind, RunStatus};
        let mut a = agent();
        let mut fx = Effects::default();
        let lines = entries(BG_COMMAND);
        let launched = lines
            .iter()
            .position(|e| e["toolUseResult"]["backgroundTaskId"].is_string())
            .expect("the launch's result");
        let notified = lines
            .iter()
            .position(|e| e["origin"]["kind"] == "task-notification")
            .expect("the notification");
        for (i, e) in lines.iter().enumerate() {
            a.on_transcript(e, &mut fx);
            let row = a
                .meta
                .plan
                .as_ref()
                .and_then(|p| p.bg_tasks.iter().find(|t| t.id == "btzyapwqx").cloned());
            if i < launched {
                assert!(row.is_none(), "line {i}");
                continue;
            }
            let row = row.unwrap_or_else(|| panic!("no row at line {i}"));
            assert_eq!(row.kind, BgKind::Command);
            assert_eq!(row.title, "Wait until PR 148 checks finish");
            if i < notified {
                // Listed from its launch, through the end of the turn.
                assert_eq!(row.status, RunStatus::Running, "line {i}");
            } else {
                assert_eq!(row.status, RunStatus::Done, "line {i}");
                assert!(row
                    .summary
                    .as_deref()
                    .is_some_and(|s| s.contains("exit code 0")));
            }
        }
    }

    fn agent() -> AgentRt {
        let dir = std::env::temp_dir().join(format!("escouade-tui-{}", new_id()));
        let meta = AgentMeta {
            id: "a1".into(),
            cwd: "/work/demo".into(),
            session_id: Some("55555555-2222-3333-4444-555555555555".into()),
            ..Default::default()
        };
        let mut a = AgentRt::new(meta, &dir);
        a.start_tui();
        a
    }

    fn entries(text: &str) -> Vec<Value> {
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[test]
    fn a_real_transcript_gives_the_conversation_and_its_turns() {
        let mut a = agent();
        let mut fx = Effects::default();
        let mut turns = Vec::new();
        for e in entries(TRANSCRIPT) {
            a.on_transcript(&e, &mut fx);
            turns.extend(fx.turn_end.take());
        }
        let items = a.conv.items();
        let users: Vec<&str> = items
            .iter()
            .filter(|i| i["kind"] == "user")
            .filter_map(|i| i["text"].as_str())
            .collect();
        assert_eq!(users.len(), 3, "{users:?}");
        assert!(users[0].starts_with("Run the shell command"));
        assert!(items
            .iter()
            .all(|i| i["kind"] != "user" || i["origin"] == "terminal"));
        // The command, its result and the replies.
        let bash = items
            .iter()
            .find(|i| i["kind"] == "tool" && i["name"] == "Bash")
            .unwrap();
        assert_eq!(bash["status"], "ok");
        assert!(items.iter().any(|i| i["kind"] == "text"
            && i["text"]
                .as_str()
                .is_some_and(|t| t.to_lowercase().contains("bonjour"))));
        // Three turns ended, each counted.
        assert_eq!(turns.len(), 3);
        assert!(turns.iter().all(|t| matches!(t, TurnEnd::Finished(_))));
        assert_eq!(items.iter().filter(|i| i["kind"] == "turn").count(), 3);
        assert_eq!(a.meta.prompts, 3);
        assert_eq!(a.meta.status, AgentStatus::Done);
        assert!(a.meta.cost > 0.0 && a.meta.tokens > 0);
        // Each turn its own share: the rows sum to the agent's cost.
        let rows: f64 = fx.turns.iter().map(|t| t.cost).sum();
        assert!((rows - a.meta.cost).abs() < 1e-9);
        // The mode as the interface showed it.
        assert_eq!(a.meta.mode, "default");
    }

    #[test]
    fn the_hooks_tell_when_it_runs_waits_and_stops() {
        let mut a = agent();
        let mut fx = Effects::default();
        let mut seen = Vec::new();
        for h in entries(HOOKS) {
            let out = a.on_hook(&h, &mut fx);
            assert_eq!(out.transcript, None, "same session");
            seen.push((
                h["hook_event_name"].as_str().unwrap().to_string(),
                a.meta.status,
                fx.notify.take().map(|n| n.body),
                out.stopped,
            ));
        }
        let at = |name: &str| seen.iter().find(|s| s.0 == name).unwrap();
        assert_eq!(at("UserPromptSubmit").1, AgentStatus::Running);
        let asked = at("PermissionRequest");
        assert_eq!(asked.1, AgentStatus::Waiting);
        assert!(
            asked.2.as_deref().is_some_and(|b| b.contains("echo hi")),
            "{asked:?}"
        );
        assert_eq!(at("PostToolUse").1, AgentStatus::Running);
        assert!(at("Stop").3);
    }

    #[test]
    fn another_session_in_a_hook_is_read_from_its_own_transcript() {
        let mut a = agent();
        let mut fx = Effects::default();
        let h = json!({
            "hook_event_name": "UserPromptSubmit", "session_id": "other",
            "transcript_path": "/home/u/.claude/projects/x/other.jsonl",
        });
        let out = a.on_hook(&h, &mut fx);
        assert_eq!(
            out.transcript.as_deref(),
            Some("/home/u/.claude/projects/x/other.jsonl")
        );
        assert_eq!(a.meta.session_id.as_deref(), Some("other"));
    }

    #[test]
    fn a_local_commands_output_during_a_turn_of_the_model_leaves_that_turn_running() {
        let mut a = agent();
        let mut fx = Effects::default();
        let user = |text: &str| {
            json!({ "type": "user", "uuid": new_id(), "origin": { "kind": "human" },
                    "message": { "role": "user", "content": text } })
        };
        a.on_transcript(&user("Count to 200"), &mut fx);
        a.on_transcript(
            &user("<local-command-stdout>Total cost: $0.01</local-command-stdout>"),
            &mut fx,
        );
        a.on_transcript(
            &json!({ "type": "system", "subtype": "local_command", "commandOutcome": { "kind": "failed" },
                     "content": "<local-command-stderr>no</local-command-stderr>" }),
            &mut fx,
        );
        assert_eq!(a.meta.status, AgentStatus::Running);
        assert_eq!(fx.turn_end, None);
    }

    #[test]
    fn an_interruption_ends_the_turn_as_interrupted_and_a_late_end_counts_nothing_more() {
        let mut a = agent();
        let mut fx = Effects::default();
        let user = |text: &str| {
            json!({ "type": "user", "uuid": new_id(), "origin": { "kind": "human" },
                    "message": { "role": "user", "content": text } })
        };
        a.on_transcript(&user("Count to 200"), &mut fx);
        assert_eq!(a.meta.status, AgentStatus::Running);
        a.on_transcript(&user("[Request interrupted by user]"), &mut fx);
        assert_eq!(fx.turn_end.take(), Some(TurnEnd::Interrupted));
        assert_eq!(a.meta.status, AgentStatus::Done);
        a.on_transcript(
            &json!({ "type": "system", "subtype": "turn_duration", "durationMs": 5 }),
            &mut fx,
        );
        assert_eq!(fx.turn_end, None);
        // The marker is no message of the user's.
        let users = a
            .conv
            .items()
            .iter()
            .filter(|i| i["kind"] == "user")
            .count();
        assert_eq!(users, 1);
    }

    /// The (kind, level, text) of the items of the conversation, but its turns'.
    fn shown(a: &mut AgentRt) -> Vec<(String, String, String)> {
        let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
        a.conv
            .items()
            .iter()
            .filter(|i| i["kind"] != "turn")
            .map(|i| (s(&i["kind"]), s(&i["level"]), s(&i["text"])))
            .collect()
    }

    #[test]
    fn a_shell_command_of_the_bang_mode_shows_as_typed_with_its_outputs() {
        let mut a = agent();
        let mut fx = Effects::default();
        let user = |content: Value| json!({ "type": "user", "uuid": new_id(), "message": { "role": "user", "content": content } });
        a.on_transcript(&user(json!("<bash-input>git status</bash-input>")), &mut fx);
        a.on_transcript(
            &user(json!("<bash-stdout>On branch main\n\u{1b}[32mnothing to commit\u{1b}[m</bash-stdout><bash-stderr></bash-stderr>")),
            &mut fx,
        );
        a.on_transcript(&user(json!("<bash-input>false</bash-input>")), &mut fx);
        a.on_transcript(
            &user(json!([{ "type": "text", "text": "<bash-stdout></bash-stdout><bash-stderr>Command failed: false</bash-stderr>" }])),
            &mut fx,
        );
        let s = |k: &str, l: &str, t: &str| (k.to_string(), l.to_string(), t.to_string());
        assert_eq!(
            shown(&mut a),
            [
                s("user", "", "! git status"),
                s("notice", "output", "On branch main\nnothing to commit"),
                s("user", "", "! false"),
            ]
        );
        // Each its turn, over with its output; the error is the failed one's, shown on its card.
        let turns: Vec<Value> = a
            .conv
            .items()
            .into_iter()
            .filter(|i| i["kind"] == "turn")
            .collect();
        assert_eq!(turns.len(), 2, "{turns:?}");
        assert_eq!(turns[0]["isError"], false);
        assert_eq!(turns[1]["error"], "Command failed: false");
        assert!(!a.meta.status.is_active());
    }

    #[test]
    fn a_local_commands_output_in_text_blocks_shows_as_an_output_and_its_caveat_not_at_all() {
        let mut a = agent();
        let mut fx = Effects::default();
        let user = |content: Value| json!({ "type": "user", "uuid": new_id(), "message": { "role": "user", "content": content } });
        // A caveat Claude Code would not mark as its own.
        a.on_transcript(
            &user(json!(
                "<local-command-caveat>Caveat: x</local-command-caveat>"
            )),
            &mut fx,
        );
        a.on_transcript(&user(json!([{ "type": "text", "text": "<command-name>/context</command-name>\n<command-message>context</command-message>\n<command-args></command-args>" }])), &mut fx);
        a.on_transcript(
            &user(json!([{ "type": "text", "text": "<local-command-stdout> \u{1b}[1mContext Usage\u{1b}[22m\n ⛁ ⛀   12k/200k</local-command-stdout>" }])),
            &mut fx,
        );
        let s = |k: &str, l: &str, t: &str| (k.to_string(), l.to_string(), t.to_string());
        assert_eq!(
            shown(&mut a),
            [
                s("user", "", "/context"),
                s("notice", "output", " Context Usage\n ⛁ ⛀   12k/200k"),
            ]
        );
    }

    #[test]
    fn a_command_typed_in_the_interface_shows_as_typed() {
        let mut a = agent();
        let mut fx = Effects::default();
        let e = json!({ "type": "user", "uuid": "u1", "message": { "role": "user",
            "content": "<command-name>/model</command-name>\n<command-message>model</command-message>\n<command-args>opus</command-args>" } });
        a.on_transcript(&e, &mut fx);
        let item = a
            .conv
            .items()
            .into_iter()
            .find(|i| i["kind"] == "user")
            .unwrap();
        assert_eq!(item["text"], "/model opus");
        // Read again (the transcript from its start): listed once.
        a.on_transcript(&e, &mut fx);
        assert_eq!(
            a.conv
                .items()
                .iter()
                .filter(|i| i["kind"] == "user")
                .count(),
            1
        );
    }

    #[test]
    fn its_interface_gone_mid_turn_ends_the_turn() {
        let mut a = agent();
        let mut fx = Effects::default();
        a.on_hook(&json!({ "hook_event_name": "UserPromptSubmit" }), &mut fx);
        a.stop_tui(&mut fx);
        assert_eq!(fx.turn_end, Some(TurnEnd::Interrupted));
        assert!(!a.meta.status.is_active());
    }

    /// A `bridge_status` entry, with its link or, with none, what it says.
    fn bridge_status(content: &str, url: Option<&str>) -> Value {
        let mut e = json!({ "type": "system", "subtype": "bridge_status", "content": content });
        if let Some(url) = url {
            e["url"] = json!(url);
        }
        e
    }

    #[test]
    fn a_bridge_status_entry_gives_the_interfaces_link_and_each_interface_starts_without_it() {
        let mut a = agent();
        let mut fx = Effects::default();
        assert_eq!(a.remote_state, None);
        // No interface to ask: a failure to link is nothing to tell, and an entry without an
        // identifier tells no session.
        a.on_transcript(
            &bridge_status("/remote-control is not active", None),
            &mut fx,
        );
        a.on_transcript(&json!({ "type": "bridge-session" }), &mut fx);
        a.on_transcript(
            &json!({ "type": "bridge-session", "bridgeSessionId": "" }),
            &mut fx,
        );
        assert_eq!(a.remote_state, None);
        assert_eq!(a.meta.remote_url, None);
        assert!(!fx.agent_changed);

        // The entry that tells the session: connected, with no address to give yet.
        a.on_transcript(
            &json!({ "type": "bridge-session", "bridgeSessionId": "cse_abc" }),
            &mut fx,
        );
        assert_eq!(a.remote_state.as_deref(), Some("connected"));
        assert_eq!(a.meta.remote_url, None);

        let url = "https://claude.ai/code/session_abc";
        let mut fx = Effects::default();
        a.on_transcript(
            &bridge_status(&format!("/remote-control is active · {url}"), Some(url)),
            &mut fx,
        );
        assert_eq!(a.remote_state.as_deref(), Some("connected"));
        assert_eq!(a.meta.remote_url.as_deref(), Some(url));
        // The stream's session is another: left alone.
        assert_eq!(a.meta.remote_session, None);
        assert!(fx.save);
        // The same entry read again changes nothing more.
        let mut again = Effects::default();
        a.on_transcript(&bridge_status("", Some(url)), &mut again);
        assert!(!again.save);

        // A new interface knows nothing of the link until it tells it; the link stays recorded.
        a.start_tui();
        assert_eq!(a.remote_state, None);
        assert_eq!(a.meta.remote_url.as_deref(), Some(url));
    }

    fn notices(a: &mut AgentRt) -> Vec<String> {
        a.conv
            .items()
            .iter()
            .filter(|i| i["kind"] == "notice")
            .filter_map(|i| i["text"].as_str().map(str::to_string))
            .collect()
    }

    #[test]
    fn an_interface_started_with_remote_control_that_writes_no_link_is_unavailable_once() {
        let mut a = agent();
        a.tui = Some(TuiRun::quiet(true));
        let mut fx = Effects::default();
        assert!(!a.interface_ready());
        a.on_transcript(
            &bridge_status("/remote-control is not active", None),
            &mut fx,
        );
        // Up, and unavailable.
        assert!(a.interface_ready());
        assert_eq!(a.remote_state.as_deref(), Some("unavailable"));
        assert!(fx.agent_changed);
        assert_eq!(
            notices(&mut a),
            ["Remote control indisponible : Claude Code n’a pas pu ouvrir la session distante (« /remote-control is not active »)."]
        );
        // Told once, however many times it says it.
        a.on_transcript(
            &bridge_status("/remote-control is not active", None),
            &mut fx,
        );
        a.on_transcript(&bridge_status("   ", None), &mut fx);
        assert_eq!(notices(&mut a).len(), 1);

        // The link comes after all: connected, and the next failure is news again.
        let url = "https://claude.ai/code/session_abc";
        a.on_transcript(&bridge_status("", Some(url)), &mut fx);
        assert_eq!(a.remote_state.as_deref(), Some("connected"));
        assert!(!a.remote_told);
        // A link seen, a later entry without one (the user turned it off in the interface) is
        // not a failure to open it.
        a.on_transcript(
            &bridge_status("/remote-control is not active", None),
            &mut fx,
        );
        assert_eq!(a.remote_state.as_deref(), Some("connected"));
        assert_eq!(notices(&mut a).len(), 1);
    }

    #[test]
    fn an_entry_without_a_link_is_no_news_for_an_interface_started_without_remote_control() {
        let mut a = agent();
        a.tui = Some(TuiRun::quiet(false));
        let mut fx = Effects::default();
        a.on_transcript(
            &bridge_status("/remote-control is not active", None),
            &mut fx,
        );
        // It says it is up, and that is all.
        assert!(a.interface_ready());
        assert_eq!(a.remote_state, None);
        assert!(notices(&mut a).is_empty());
    }

    #[test]
    fn the_entries_of_the_link_make_an_interface_ready_and_the_session_start_hook_does_not() {
        let mut a = agent();
        a.tui = Some(TuiRun::quiet(true));
        let mut fx = Effects::default();
        // Claude Code skips its HTTP hooks for SessionStart; if one came, it would change nothing.
        let status = a.meta.status;
        let out = a.on_hook(
            &json!({ "hook_event_name": "SessionStart", "source": "startup",
                "session_id": "55555555-2222-3333-4444-555555555555" }),
            &mut fx,
        );
        assert_eq!(out, HookOutcome::default());
        assert_eq!(a.meta.status, status);
        assert!(a.conv.items().is_empty());
        assert!(!a.interface_ready());
        a.on_transcript(
            &json!({ "type": "bridge-session", "bridgeSessionId": "cse_abc" }),
            &mut fx,
        );
        assert!(a.interface_ready());
        // Registered by the registry of the sessions, or shown by a window: ready all the same.
        let mut b = agent();
        b.tui = Some(TuiRun::quiet(false));
        assert!(!b.interface_ready());
        b.tui.as_mut().unwrap().registered = Some(std::time::Instant::now());
        assert!(b.interface_ready());
        let mut c = agent();
        c.tui = Some(TuiRun::quiet(false));
        c.tui.as_ref().unwrap().output.mark_shown();
        assert!(c.interface_ready());
    }

    #[test]
    fn ending_the_link_keeps_the_remote_session_until_it_is_closed_and_the_interfaces_state() {
        let mut a = agent();
        a.meta.remote_session = Some("cse_abc".into());
        a.meta.remote_url = Some("https://claude.ai/code/session_abc".into());
        a.remote_state = Some("connected".into());
        // No live process linked: nothing can close the session now.
        assert!(a.end_link().is_none());
        assert_eq!(a.meta.remote_session.as_deref(), Some("cse_abc"));
        assert_eq!(a.meta.remote_url, None);
        assert_eq!(a.remote_state, None);
        // Forgotten when it is closed, if it is still the one.
        a.forget_remote_session(Some("cse_other"));
        assert_eq!(a.meta.remote_session.as_deref(), Some("cse_abc"));
        a.forget_remote_session(Some("cse_abc"));
        assert_eq!(a.meta.remote_session, None);

        // In the Claude Code view, the state is the interface's: it keeps its link.
        let mut b = agent();
        b.meta.terminal = true;
        b.remote_state = Some("connected".into());
        b.end_link();
        assert_eq!(b.remote_state.as_deref(), Some("connected"));
    }
}
