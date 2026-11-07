//! Claude Code in the agent's conversation. Since INT-10.5 its agents' frames are translated by its plugin
//! (`dev.escouade.claude-code`, `plugins/claude/src/translate.rs`) and applied as items
//! (`AgentRt::apply_items`, through `Engine::translate`), the transcript of its terminal view too
//! (`plugins/claude/src/view.rs`). What is here is the agent's own: its plan told of the calls, a message that comes back, a
//! file sent to the user.

use super::*;
use escouade_plugin_protocol::engine::ModelTurn;

impl AgentRt {
    /// One frame of Claude Code, in a test: its items applied at once, the next frame reads what they made.
    #[cfg(test)]
    pub(crate) fn claude_frame(&mut self, f: &Value, fx: &mut Effects) {
        self.view_frame(f, fx);
    }

    /// A conversation its harness says it runs (its `init`): another than the agent's starts anew.
    pub(super) fn on_conversation(&mut self, session: String, notice: String, fx: &mut Effects) {
        if !session.is_empty() {
            if self.meta.session_id.as_deref() != Some(session.as_str()) {
                // Another conversation: what the plan was about is not in its context.
                self.reset_plan(fx);
                if self.saw_init {
                    self.notice("info", notice, fx);
                }
                self.meta.session_id = Some(session);
                // None of the entries of the session it leaves is one of this one's.
                self.meta.last_entry = None;
                fx.save = true;
            }
            // A copy has a session of its own from its first turn on: its next starts resume that one, never the
            // original's again.
            let forked = self.meta.fork_of.take().is_some();
            self.meta.fork_at = None;
            if forked {
                fx.save = true;
            }
        }
        self.saw_init = true;
    }

    /// A message of the model begins.
    pub(super) fn on_message(
        &mut self,
        id: String,
        model: String,
        usage: Value,
        main: bool,
        fx: &mut Effects,
    ) {
        self.live
            .insert(id, (model.clone(), Usage::from_api(&usage)));
        fx.agent_changed = true;
        if main {
            if !model.is_empty() {
                self.main_model = model;
            }
            self.context_tokens = [
                &usage["input_tokens"],
                &usage["cache_read_input_tokens"],
                &usage["cache_creation_input_tokens"],
            ]
            .iter()
            .filter_map(|v| v.as_u64())
            .sum();
        }
        // A turn started without a user message from us (queued message, background task).
        if !self.meta.status.is_active() {
            self.set_status(AgentStatus::Running, fx);
        }
    }

    /// A tool's whole call: the plan told of it, its row made or told its input.
    pub(super) fn on_call(
        &mut self,
        id: String,
        name: String,
        input: Value,
        parent: Option<String>,
        fx: &mut Effects,
    ) {
        if parent.is_none() {
            // What was said before a tool was a step on the way, not the final reply.
            self.final_text.clear();
            let activity = tool_activity(&name, &input, &self.meta.cwd);
            self.set_activity(activity, fx);
        }
        let block = json!({ "id": id, "name": name, "input": input });
        self.plan_tool_use(parent.as_deref(), &block, fx);
        // Files sent to the user (ESC-171): those of a call read again (the transcript of the Claude Code view) are kept
        // as they were delivered.
        let sending = name == crate::sentfiles::TOOL;
        let delivered = sending
            .then(|| self.conv.get(&id))
            .flatten()
            .and_then(|v| v.get("sent").cloned());
        if self.conv.contains(&id) {
            self.patch(&id, json!({ "input": item_input(&name, &input) }), fx);
        } else {
            let item = json!({
                "kind": "tool", "id": id, "name": name, "input": item_input(&name, &input),
                "status": "running", "parent": parent, "ts": now_ms(),
            });
            self.append(item, fx);
        }
        if sending {
            match delivered {
                Some(sent) => self.patch(&id, json!({ "sent": sent }), fx),
                None => self.on_sent_files(parent.as_deref(), &block, fx),
            }
        }
    }

    /// A `SendUserFile` call (ESC-171): its files for the core to deliver (`sentfiles::deliver`),
    /// and, sent « proactive » by the agent itself, a notification as for a question.
    pub(super) fn on_sent_files(&mut self, parent: Option<&str>, block: &Value, fx: &mut Effects) {
        let Some(id) = block["id"].as_str() else {
            return;
        };
        let input = &block["input"];
        fx.sent.push((id.to_string(), input.clone()));
        if parent.is_none() && input["status"] == "proactive" {
            let caption = input["caption"]
                .as_str()
                .map(str::trim)
                .filter(|c| !c.is_empty());
            let body = caption
                .map(|c| truncate(c, 200))
                .or_else(|| tool_activity(crate::sentfiles::TOOL, input, &self.meta.cwd))
                .unwrap_or_default();
            fx.notify = Some(AgentAlert::new(NotifyKind::Question, body));
        }
    }

    /// A tool the assistant calls, for the plan: its task list, a subagent it launches, a tool one
    /// of its subagents (`parent`) uses and what it makes of it.
    pub(super) fn plan_tool_use(&mut self, parent: Option<&str>, block: &Value, fx: &mut Effects) {
        let (Some(id), Some(name)) = (block["id"].as_str(), block["name"].as_str()) else {
            return;
        };
        let input = &block["input"];
        if parent.is_none() {
            self.watch_plan_files(id, name, input);
        }
        let doing = parent.and_then(|_| tool_activity(name, input, &self.meta.cwd));
        let now = now_ms();
        // The agent's own list moved: the plan it follows is named and titled again from what the
        // files said last, for the look that comes finds them as they were and says nothing.
        let files = (parent.is_none() && matches!(name, "TodoWrite" | "TaskCreate" | "TaskUpdate"))
            .then(|| self.plan_list.clone())
            .flatten();
        self.plan_apply(fx, |plan| {
            let change = plan.on_tool_use(parent, id, name, input, now);
            match (parent, files) {
                (Some(p), _) => change.and(plan.on_child_activity(p, doing.as_deref())),
                (None, Some(files)) if change != Change::None => {
                    change.and(plan.set_files(Some(files)))
                }
                (None, _) => change,
            }
        });
    }

    /// A call of the main thread after which the files of the plan may have moved: a command (the
    /// ledger is written by scripts and commands), a subagent, the write of a plan or of a file of
    /// its workspace (the ledger, a brief). Its result has them looked at
    /// (`Effects::plan_files`); a plan it writes is remembered.
    pub(super) fn watch_plan_files(&mut self, id: &str, name: &str, input: &Value) {
        let wrote = matches!(name, "Write" | "Edit" | "MultiEdit")
            && input["file_path"].as_str().is_some_and(|path| {
                let plan = self.note_written_plan(path);
                plan || in_sdd_workspace(path)
            });
        if name == "Bash" {
            for plan in input["command"]
                .as_str()
                .into_iter()
                .flat_map(plans_written_by)
            {
                self.note_written_plan(plan);
            }
            let followed = self.meta.plan.as_ref().and_then(|p| p.plan_file.as_deref());
            let removing = followed
                .zip(input["command"].as_str())
                .is_some_and(|(plan, command)| planfiles::removes_workspace(command, plan));
            if removing && self.plan_removals.len() < MAX_PLAN_CALLS {
                self.plan_removals.insert(id.to_string());
            }
        }
        if wrote || matches!(name, "Bash" | "Task" | "Agent") {
            if self.plan_calls.len() >= MAX_PLAN_CALLS {
                self.plan_calls.clear();
            }
            self.plan_calls.insert(id.to_string());
        }
    }

    /// `path`, as a tool was given it, is a plan of the plans folder: kept (absolute) as the
    /// latest the agent wrote, five at most. Whether it is inside the repository, and there, is
    /// for `planfiles` to say when it reads it.
    pub(super) fn note_written_plan(&mut self, path: &str) -> bool {
        const PLANS: &str = "docs/superpowers/plans/";
        // Behind a slash, or at the start of a path relative to its folder.
        let slashed = path.replace('\\', "/").to_ascii_lowercase();
        let in_plans = slashed.starts_with(PLANS) || slashed.contains(&format!("/{PLANS}"));
        if !slashed.ends_with(".md") || !in_plans {
            return false;
        }
        let absolute = slashed.starts_with('/') || slashed.chars().nth(1) == Some(':');
        let path = if absolute {
            path.to_string()
        } else {
            format!("{}/{}", self.meta.cwd.trim_end_matches(['/', '\\']), path)
        };
        // The same file is the same whatever the slashes and the case it is spelled with.
        let same = |p: &str| p.replace('\\', "/").to_lowercase();
        self.plan_written.retain(|p| same(p) != same(&path));
        self.plan_written.push(path);
        if self.plan_written.len() > MAX_PLANS_WRITTEN {
            self.plan_written.remove(0);
        }
        true
    }

    /// A user message re-emitted by its harness (`--replay-user-messages`): the echo of one sent from the app (same uuid,
    /// already listed), or one sent from claude.ai (Remote Control).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn on_replay(
        &mut self,
        id: Option<String>,
        text: String,
        kind: Option<String>,
        from: Option<String>,
        synthetic: bool,
        images: u32,
        files: Vec<String>,
        fx: &mut Effects,
    ) {
        let id = id.unwrap_or_else(new_id);
        if self.conv.contains(&id) {
            return;
        }
        let kind = kind.as_deref();
        let human = kind.is_none_or(|k| k == "human") && !synthetic;
        let parts = localcmd::parse(&text);
        // The wake-up of a loop (`ScheduleWakeup`) runs its prompt again, as a message of no
        // origin (a human's, `<command-name>/loop</command-name>…` for `/loop`): a wake-up, not a
        // message of the user's (ESC-154).
        if human && self.loop_wake_due() {
            let typed = parts.iter().flatten().find_map(|p| match p {
                localcmd::Part::Typed(t) => Some(t.clone()),
                _ => None,
            });
            if let Some(text) = typed.or_else(|| parts.is_none().then(|| text.clone())) {
                // It fired: a message that comes next is no wake-up, unless the loop asks another.
                self.plan_apply(fx, |p| {
                    p.wakes.retain(|w| w.kind != crate::plan::WakeKind::Loop);
                    Change::Saved
                });
                self.append(
                    json!({ "kind": "event", "id": id, "source": "wake", "text": text, "ts": now_ms() }),
                    fx,
                );
                fx.save = true;
                return;
            }
        }
        // A local command (`/compact`, `! ls`): as typed, then what it printed. The one the app
        // sent is listed already, as typed (the replay has its uuid).
        if let Some(parts) = parts {
            for part in &parts {
                if let localcmd::Part::Typed(typed) = part {
                    self.push_user_from(&id, typed, 0, &[], Some("remote"), fx);
                }
            }
            self.local_output(Some(&id), &parts, fx);
            fx.save = true;
            return;
        }
        // Only a human's message is the user's (no origin, or "human", as Claude Code tells). The
        // rest Claude Code passes on to Claude by itself (a subagent's report, a background task
        // that ended, an MCP channel, a meta message): an event.
        if kind.is_some_and(|k| k != "human") || synthetic {
            if text.trim().is_empty() {
                return;
            }
            if kind == Some("task-notification") {
                self.end_replayed_task(&text, fx);
                let task = tag(&text, "task-id").map(str::to_string);
                if let Some(n) = task.and_then(|t| self.announced_tasks.get_mut(&t)) {
                    if *n > 0 {
                        *n -= 1;
                        return;
                    }
                }
            }
            if kind == Some("peer") {
                self.keep_handback(&text, from.as_deref(), fx);
            }
            let source = match kind {
                Some("task-notification") => "task",
                Some("peer") => "agent",
                Some(k) => k,
                None => "synthetic",
            };
            let mut item = json!({ "kind": "event", "id": id, "source": source, "text": text, "ts": now_ms() });
            if let Some(from) = from {
                item["from"] = from.into();
            }
            self.append(item, fx);
            fx.save = true;
            return;
        }
        if text.trim().is_empty() && images == 0 && files.is_empty() {
            return;
        }
        self.push_user_from(&id, &text, images, &files, Some("remote"), fx);
    }

    /// The wake-up of a loop is due: its time has come (it fires once the process is idle, so
    /// late when a turn ran then), not long ago (a loop left without its stop would hold one).
    fn loop_wake_due(&self) -> bool {
        const EARLY_MS: i64 = 30_000;
        const LATE_MS: i64 = 30 * 60_000;
        let now = now_ms();
        self.meta.plan.as_ref().is_some_and(|p| {
            p.wakes.iter().any(|w| {
                w.kind == crate::plan::WakeKind::Loop
                    && w.at <= now + EARLY_MS
                    && w.at >= now - LATE_MS
            })
        })
    }

    /// What a local command printed (`parts`, as `localcmd` read them), under the command: its
    /// output as an output, its error as an error (ESC-154). Given the id of its entry (`id`),
    /// one read again is listed once.
    pub(super) fn local_output(
        &mut self,
        id: Option<&str>,
        parts: &[localcmd::Part],
        fx: &mut Effects,
    ) {
        for (k, part) in parts.iter().enumerate() {
            let (level, text) = match part {
                localcmd::Part::Output(t) => ("output", t),
                localcmd::Part::Error(t) => ("error", t),
                localcmd::Part::Typed(_) => continue,
            };
            let item_id = id.map_or_else(new_id, |id| format!("{id}:out{k}"));
            if self.conv.contains(&item_id) {
                continue;
            }
            self.append(
                json!({ "kind": "notice", "id": item_id, "ts": now_ms(), "level": level, "text": text }),
                fx,
            );
        }
    }

    pub(super) fn touch(&mut self, path: &str, fx: &mut Effects) {
        let rel = relative_slash(&self.meta.cwd, path);
        if !self
            .meta
            .touched_files
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&rel))
        {
            self.meta.touched_files.push(rel);
            fx.save = true;
        }
        fx.files_changed = true;
    }

    /// Takes `entry` as the latest entry of its session, saved once it changes, not at the end of
    /// the turn only: an app stopped for good mid-turn would leave a copy to fork at an older one.
    /// The saves asked for are grouped (one every 400 ms at most), not one per frame.
    pub(super) fn set_last_entry(&mut self, entry: Option<String>, fx: &mut Effects) {
        if self.meta.last_entry != entry {
            self.meta.last_entry = entry;
            fx.save = true;
        }
    }

    /// The end of a turn its harness counted (`Item::Result`).
    pub(super) fn on_result(&mut self, r: TurnResult, fx: &mut Effects) {
        let TurnResult {
            id,
            failed: is_error,
            error,
            failure,
            reply: said,
            limited,
            resets,
            sign_in,
            duration,
            window,
            cost,
            tokens,
            models,
        } = r;
        let interrupted = std::mem::take(&mut self.interrupted);
        // The exact figures below replace the running estimate.
        self.live.clear();
        if limited && is_error && !interrupted {
            fx.limited = Some(resets);
        } else if self.meta.resume_at.take().is_some() {
            // It went on anyway (a turn Claude Code started by itself): nothing left to resume.
            fx.save = true;
            fx.agent_changed = true;
        }
        if let Some(window) = window {
            self.context_window = window;
        }
        for m in models {
            fx.turns.push(TurnRow {
                engine: self.meta.engine,
                model: m.model,
                input: m.input,
                cache: m.cache,
                output: m.output,
                cost: m.cost,
            });
        }
        self.meta.tokens += tokens;
        self.meta.cost += cost;
        let error = error.filter(|_| is_error && !interrupted);
        let failed = is_error && !interrupted;
        // A sign-in that could not be refreshed: the core looks at the account's (ESC-162).
        let sign_in = failed && !limited && sign_in;
        if sign_in {
            fx.sign_in_lost = true;
        } else {
            self.sign_in_retries = 0;
        }
        let failure = failed.then_some(failure);
        // The final reply: the text that no tool followed, else what its harness makes of it.
        let reply = match std::mem::take(&mut self.final_text) {
            text if !text.trim().is_empty() => text,
            _ => said.clone(),
        };
        let end = if interrupted {
            TurnEnd::Interrupted
        } else if limited && is_error {
            TurnEnd::Limited
        } else if sign_in {
            TurnEnd::SignIn
        } else if let Some(reason) = failure.clone() {
            TurnEnd::Error(reason)
        } else {
            let text = std::mem::take(&mut self.turn_text);
            TurnEnd::Finished(if text.trim().is_empty() { said } else { text })
        };
        self.forget_turn();
        self.close_open_items(fx);
        // A subagent in the foreground did not outlive the turn: it was stopped.
        self.plan_apply(fx, |plan| plan.end_turn(now_ms()));
        // The turn's last commands may have written the ledger.
        fx.plan_files = true;
        // Stopped by the usage limit: the card offers to go on, on another account.
        let at_limit = limited && is_error && !interrupted;
        // The turn that follows a resume on another account ended. Failing, it is the resume's failure, and the agent keeps
        // the account it can go back to; otherwise (done, at the limit there too, interrupted) it came from nowhere any more.
        if !matches!(end, TurnEnd::Error(_)) && self.meta.moved_from.take().is_some() {
            self.meta.moved_resume_at = None;
            fx.save = true;
            fx.agent_changed = true;
        }
        let item = json!({
            "kind": "turn", "id": id.unwrap_or_else(new_id), "ts": now_ms(),
            "durationMs": duration, "cost": cost, "tokens": tokens,
            "isError": is_error && !interrupted, "interrupted": interrupted, "error": error,
            "limited": at_limit, "signIn": sign_in, "engine": self.meta.engine,
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
        // Told once the core knows whether it goes on or waits for a sign-in (`signin`).
        if !interrupted && !had_queue && !sign_in {
            fx.notify = Some(match &failure {
                Some(reason) => AgentAlert::new(NotifyKind::Error, notify::error_body(reason)),
                None => AgentAlert::new(NotifyKind::Done, notify::done_body(&reply)),
            });
        }
        fx.agent_changed = true;
        fx.save = true;
        fx.turn_end = Some(end);
    }

    /// Its harness asks leave for a call, or asks the user (`Item::Ask`).
    pub(super) fn on_ask(&mut self, a: Asking, fx: &mut Effects) {
        let Asking {
            request: rid,
            call: tool_use_id,
            tool,
            input,
            title,
            description,
            reason,
            suggestions,
            no_always,
            default_no,
            task,
        } = a;
        // The reading of a file the user joined by its own path (the file itself, nothing else, never a change): allowed on
        // the spot, no card and nobody waited for. The user chose that file; `--add-dir` cannot give a folder that was not
        // known at the start. Any other request, or one the process no longer takes, goes on below.
        if attachfiles::granted(&self.meta.granted_files, &tool, &input) {
            let allowed = PermissionAnswer::Allow {
                call: tool_use_id.clone(),
                input: input.clone(),
                resume: None,
            };
            if self
                .proc
                .as_ref()
                .is_some_and(|p| p.answer_permission(&rid, allowed).is_ok())
            {
                log::info!(
                    "agent {}: a file joined by its path is read without a question",
                    self.meta.id
                );
                return;
            }
        }
        let body = if tool == "AskUserQuestion" {
            notify::question_body(crate::engine::of(&self.meta).speaker(), &input)
        } else {
            notify::permission_body(&tool, &input, &self.meta.cwd)
        };
        let item = if tool == "AskUserQuestion" {
            json!({
                "kind": "question", "id": rid, "toolUseId": tool_use_id,
                "questions": input["questions"], "answers": null, "ts": now_ms(),
            })
        } else {
            let can_always = suggestions.as_array().is_some_and(|a| !a.is_empty()) && !no_always;
            json!({
                "kind": "permission", "id": rid, "toolUseId": tool_use_id, "toolName": tool,
                "title": title, "description": description,
                "input": trim_strings(&input),
                "reason": reason.as_deref().map(strip_ansi),
                "canAlways": can_always, "defaultNo": default_no,
                "decision": null, "ts": now_ms(),
            })
        };
        let mut item = item;
        let asker = json!({ "agent_id": task, "tool_use_id": tool_use_id });
        if let Some(sub) = self.subagent_of(&asker) {
            // Shown in the agent's conversation still (no `parent`), and in its subagent's.
            item["sub"] = sub.into();
        }
        let req = json!({
            "description": description, "decision_reason": reason, "default_to_no": default_no,
        });
        let view = pending_view(&rid, &tool, &req, &input, &self.meta.cwd).always_as(&item);
        self.append(item, fx);
        self.add_pending(
            PendingReq {
                item_id: rid,
                tool_use_id,
                input,
                suggestions,
                view,
                since: now_ms(),
            },
            fx,
        );
        self.set_status(AgentStatus::Waiting, fx);
        fx.notify = Some(AgentAlert::new(NotifyKind::Question, body));
        fx.agent_changed = true;
    }
}

/// `Item::Result`, as `on_result` reads it.
pub(super) struct TurnResult {
    pub id: Option<String>,
    pub failed: bool,
    pub error: Option<String>,
    pub failure: String,
    pub reply: String,
    pub limited: bool,
    pub resets: Option<i64>,
    pub sign_in: bool,
    pub duration: Value,
    pub window: Option<u64>,
    pub cost: f64,
    pub tokens: u64,
    pub models: Vec<ModelTurn>,
}

/// `Item::Ask`, as `on_ask` reads it.
pub(super) struct Asking {
    pub request: String,
    pub call: String,
    pub tool: String,
    pub input: Value,
    pub title: Option<String>,
    pub description: Option<String>,
    pub reason: Option<String>,
    pub suggestions: Value,
    pub no_always: bool,
    pub default_no: bool,
    pub task: Option<String>,
}
