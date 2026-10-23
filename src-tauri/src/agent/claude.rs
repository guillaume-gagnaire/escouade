//! Claude Code's frames (stream-json, docs/PROTOCOL.md) translated into the agent's conversation:
//! the engine `Claude` hands them here (`Engine::translate`). The transcript of the Claude Code
//! view goes through the same handlers (`transcript`, `sidechain`).

use super::*;

impl AgentRt {
    /// One frame of its `claude` process.
    pub(crate) fn claude_frame(&mut self, f: &Value, fx: &mut Effects) {
        match f["type"].as_str().unwrap_or("") {
            "system" => self.on_system(f, fx),
            "stream_event" => self.on_stream(f, fx),
            "assistant" => self.on_assistant(f, fx),
            "user" => self.on_user(f, fx),
            "result" => self.on_result(f, fx),
            "control_request" => self.on_control_request(f, fx),
            "control_cancel_request" => self.on_cancel(f, fx),
            "rate_limit_event" => {
                let info = &f["rate_limit_info"];
                if info["status"] == "rejected" {
                    self.limit_resets_at = info["resetsAt"].as_i64().map(|s| s * 1000);
                }
                fx.rate = Some(parse_rate_event(info));
                fx.overage = Some(info["isUsingOverage"].as_bool().unwrap_or(false));
            }
            _ => {}
        }
    }

    pub(super) fn on_system(&mut self, f: &Value, fx: &mut Effects) {
        let subtype = f["subtype"].as_str().unwrap_or("");
        // The tasks of the plan (the rows of events below are about the same frames).
        if matches!(
            subtype,
            "task_started"
                | "task_updated"
                | "task_notification"
                | "task_progress"
                | "background_tasks_changed"
        ) {
            self.plan_apply(fx, |plan| plan.on_system(f, now_ms()));
        }
        match subtype {
            "init" => {
                if let Some(sid) = f["session_id"].as_str() {
                    if self.meta.session_id.as_deref() != Some(sid) {
                        // Another conversation: what the plan was about is not in its context.
                        self.reset_plan(fx);
                        if self.saw_init {
                            self.notice(
                                "info",
                                tr!(
                                    "Nouvelle conversation Claude (contexte vidé)",
                                    "New Claude conversation (context cleared)"
                                ),
                                fx,
                            );
                        }
                        self.meta.session_id = Some(sid.to_string());
                        // None of the entries of the session it leaves is one of this one's.
                        self.meta.last_entry = None;
                        fx.save = true;
                    }
                    // A copy has a session of its own from its first turn on: its next starts
                    // resume that one, never the original's again.
                    let forked = self.meta.fork_of.take().is_some();
                    self.meta.fork_at = None;
                    if forked {
                        fx.save = true;
                    }
                }
                self.saw_init = true;
            }
            "status" => {
                if let Some(mode) = f["permissionMode"].as_str() {
                    if self.meta.mode != mode {
                        self.meta.mode = mode.to_string();
                        fx.agent_changed = true;
                        fx.save = true;
                    }
                }
            }
            "compact_boundary" => {
                // Its session's chain starts again at the boundary: the entries before it are no
                // longer in it (a copy forking at one would be refused), and the next one to come
                // is where a copy forks it.
                self.set_last_entry(None, fx);
                self.notice("info", tr!("Contexte compacté", "Context compacted"), fx);
            }
            "task_started" => {
                if let (Some(id), false) = (f["task_id"].as_str(), f["is_backgrounded"] == true) {
                    self.foreground_tasks.insert(id.to_string());
                }
            }
            // A task ended. Idle, Claude Code only tells this before starting a turn by itself;
            // mid-turn, it also replays the notification to Claude (not shown again).
            "task_notification" => {
                let Some(id) = f["task_id"].as_str() else {
                    return;
                };
                self.keep_report(f, fx);
                if self.foreground_tasks.remove(id) {
                    return;
                }
                let field = |k: &str| f[k].as_str().unwrap_or_default();
                // Written as Claude Code writes it for Claude, read the same way.
                let text = format!(
                    "<task-notification>\n<task-id>{id}</task-id>\n<tool-use-id>{}</tool-use-id>\n<status>{}</status>\n<summary>{}</summary>\n</task-notification>",
                    field("tool_use_id"),
                    field("status"),
                    field("summary")
                );
                let item_id = f["uuid"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(new_id);
                self.append(
                    json!({ "kind": "event", "id": item_id, "source": "task", "text": text, "ts": now_ms() }),
                    fx,
                );
                *self.announced_tasks.entry(id.to_string()).or_default() += 1;
                fx.save = true;
            }
            "bridge_state" => {
                self.remote_state = f["state"].as_str().map(str::to_string);
                fx.agent_changed = true;
            }
            "local_command_output" => {
                let text = f["content"].as_str().unwrap_or("");
                let parts = localcmd::parse(text).unwrap_or_else(|| {
                    let plain = strip_tags(text);
                    let plain = (!plain.is_empty()).then_some(localcmd::Part::Output(plain));
                    plain.into_iter().collect()
                });
                self.local_output(f["uuid"].as_str(), &parts, fx);
            }
            _ => {}
        }
    }

    pub(super) fn on_stream(&mut self, f: &Value, fx: &mut Effects) {
        let ev = &f["event"];
        let parent = f["parent_tool_use_id"].as_str().map(str::to_string);
        let pkey = parent.clone().unwrap_or_default();
        match ev["type"].as_str().unwrap_or("") {
            "message_start" => {
                let Some(mid) = ev["message"]["id"].as_str() else {
                    return;
                };
                self.current_msg.insert(pkey, mid.to_string());
                self.blocks.entry(mid.to_string()).or_default();
                let model = ev["message"]["model"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                self.live.insert(
                    mid.to_string(),
                    (model, Usage::from_api(&ev["message"]["usage"])),
                );
                fx.agent_changed = true;
                if parent.is_none() {
                    if let Some(m) = ev["message"]["model"].as_str() {
                        self.main_model = m.to_string();
                    }
                    let u = &ev["message"]["usage"];
                    self.context_tokens = [
                        &u["input_tokens"],
                        &u["cache_read_input_tokens"],
                        &u["cache_creation_input_tokens"],
                    ]
                    .iter()
                    .filter_map(|v| v.as_u64())
                    .sum();
                    fx.agent_changed = true;
                }
                // A turn started without a user message from us (queued message, background task).
                if !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            "content_block_start" => {
                let Some(mid) = self.current_msg.get(&pkey).cloned() else {
                    return;
                };
                let index = ev["index"].as_u64().unwrap_or(0);
                let cb = &ev["content_block"];
                let (kind, item) = match cb["type"].as_str().unwrap_or("") {
                    "text" => (
                        "text",
                        json!({ "kind": "text", "id": format!("{mid}:{index}"), "text": "", "parent": parent, "streaming": true }),
                    ),
                    "thinking" => (
                        "thinking",
                        json!({ "kind": "thinking", "id": format!("{mid}:{index}"), "text": "", "parent": parent, "streaming": true }),
                    ),
                    "tool_use" | "server_tool_use" => (
                        "tool",
                        json!({
                            "kind": "tool", "id": cb["id"], "name": cb["name"], "input": {}, "status": "running",
                            "parent": parent, "ts": now_ms(),
                        }),
                    ),
                    _ => return,
                };
                if parent.is_none() {
                    let activity = block_activity(i18n::ui(), kind).or_else(|| {
                        tool_activity(
                            cb["name"].as_str().unwrap_or_default(),
                            &Value::Null,
                            &self.meta.cwd,
                        )
                    });
                    self.set_activity(activity, fx);
                }
                let item_id = item["id"].as_str().unwrap_or_default().to_string();
                self.append(item, fx);
                self.blocks.entry(mid).or_default().push(Block {
                    index,
                    item_id,
                    kind,
                    finalized: false,
                });
            }
            "content_block_delta" => {
                let Some(mid) = self.current_msg.get(&pkey) else {
                    return;
                };
                let index = ev["index"].as_u64().unwrap_or(0);
                let Some(block) = self
                    .blocks
                    .get(mid)
                    .and_then(|b| b.iter().find(|b| b.index == index))
                else {
                    return;
                };
                let d = &ev["delta"];
                let text = match d["type"].as_str().unwrap_or("") {
                    "text_delta" => d["text"].as_str(),
                    "thinking_delta" => d["thinking"].as_str(),
                    _ => None,
                };
                if let Some(text) = text.filter(|t| !t.is_empty()) {
                    let id = block.item_id.clone();
                    self.push(
                        ConvOp::Delta {
                            id,
                            text: text.to_string(),
                        },
                        fx,
                    );
                }
            }
            // The message's final output token count (message_start only announces its input).
            "message_delta" => {
                let entry = self
                    .current_msg
                    .get(&pkey)
                    .and_then(|mid| self.live.get_mut(mid));
                if let (Some((_, u)), Some(out)) = (entry, ev["usage"]["output_tokens"].as_u64()) {
                    u.output = out;
                    fx.agent_changed = true;
                }
            }
            _ => {}
        }
    }

    pub(super) fn on_assistant(&mut self, f: &Value, fx: &mut Effects) {
        let msg = &f["message"];
        let mid = msg["id"].as_str().unwrap_or("").to_string();
        let parent = f["parent_tool_use_id"].as_str().map(str::to_string);
        if let Some(err) = f["error"].as_str() {
            if err == "rate_limit" && parent.is_none() {
                self.rate_limited = true;
            }
            let text = msg["content"][0]["text"]
                .as_str()
                .unwrap_or(err)
                .to_string();
            // Its turn says it, and is taken up by itself (`signin`): no error in red before.
            if parent.is_none() && crate::signin::lost(&text) {
                return;
            }
            self.notice("error", text, fx);
            return;
        }
        // A subagent's message tells the model it runs on, which its call may not name.
        if let (Some(p), Some(model)) = (parent.as_deref(), msg["model"].as_str()) {
            self.child_model(p, model, fx);
        }
        // An entry of its session's main chain (not a subagent's, nor one Claude Code makes up):
        // where a copy made afterwards forks the session.
        if parent.is_none() && f["is_meta"] != true {
            if let Some(uuid) = f["uuid"].as_str() {
                self.set_last_entry(Some(uuid.to_string()), fx);
            }
        }
        let Some(content) = msg["content"].as_array() else {
            return;
        };
        for block in content {
            let kind = match block["type"].as_str().unwrap_or("") {
                "text" => "text",
                "thinking" | "redacted_thinking" => "thinking",
                "tool_use" | "server_tool_use" => "tool",
                _ => continue,
            };
            if parent.is_none() {
                match kind {
                    "text" => {
                        let text = block["text"].as_str().unwrap_or_default();
                        if !text.trim().is_empty() {
                            if !self.turn_text.is_empty() {
                                self.turn_text.push_str("\n\n");
                            }
                            self.turn_text.push_str(text);
                            self.final_text = text.to_string();
                        }
                    }
                    "tool" => {
                        // What was said before a tool was a step on the way, not the final reply.
                        self.final_text.clear();
                        let activity = tool_activity(
                            block["name"].as_str().unwrap_or_default(),
                            &block["input"],
                            &self.meta.cwd,
                        );
                        self.set_activity(activity, fx);
                    }
                    _ => {}
                }
            }
            if kind == "tool" {
                self.plan_tool_use(parent.as_deref(), block, fx);
            }
            let streamed = self
                .blocks
                .get_mut(&mid)
                .and_then(|v| v.iter_mut().find(|b| !b.finalized && b.kind == kind))
                .map(|b| {
                    b.finalized = true;
                    b.item_id.clone()
                });
            match (kind, streamed) {
                ("text", Some(id)) => {
                    let patch = final_text(block["text"].as_str().unwrap_or_default());
                    self.patch(&id, patch, fx)
                }
                ("thinking", Some(id)) => {
                    let text = block["thinking"].as_str().unwrap_or("");
                    self.patch(&id, json!({ "text": text, "streaming": false }), fx)
                }
                ("tool", Some(id)) => {
                    let name = block["name"].as_str().unwrap_or_default();
                    let input = item_input(name, &block["input"]);
                    self.patch(&id, json!({ "input": input }), fx)
                }
                (_, None) => {
                    let n = self.blocks.get(&mid).map_or(0, Vec::len);
                    let item = match kind {
                        "text" => {
                            let mut item = final_text(block["text"].as_str().unwrap_or_default());
                            item["kind"] = json!("text");
                            item["id"] = json!(format!("{mid}:a{n}"));
                            item["parent"] = json!(parent);
                            item
                        }
                        "thinking" => {
                            json!({ "kind": "thinking", "id": format!("{mid}:a{n}"), "text": block["thinking"].as_str().unwrap_or(""), "parent": parent, "streaming": false })
                        }
                        _ => json!({
                            "kind": "tool", "id": block["id"], "name": block["name"],
                            "input": item_input(block["name"].as_str().unwrap_or_default(), &block["input"]),
                            "status": "running", "parent": parent, "ts": now_ms(),
                        }),
                    };
                    let item_id = item["id"].as_str().unwrap_or_default().to_string();
                    self.append(item, fx);
                    self.blocks.entry(mid.clone()).or_default().push(Block {
                        index: u64::MAX,
                        item_id,
                        kind,
                        finalized: true,
                    });
                }
                _ => {}
            }
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

    /// A user message re-emitted by Claude Code (`--replay-user-messages`): the echo of one sent
    /// from the app (same uuid, already listed), or one sent from claude.ai (Remote Control).
    pub(super) fn on_replay(&mut self, f: &Value, fx: &mut Effects) {
        let id = f["uuid"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(new_id);
        if self.conv.contains(&id) {
            return;
        }
        let content = &f["message"]["content"];
        let blocks = content.as_array().map(Vec::as_slice).unwrap_or_default();
        let text = match content.as_str() {
            Some(s) => s.to_string(),
            None => blocks
                .iter()
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        };
        let human =
            f["origin"]["kind"].as_str().is_none_or(|k| k == "human") && f["isSynthetic"] != true;
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
        let kind = f["origin"]["kind"].as_str();
        if kind.is_some_and(|k| k != "human") || f["isSynthetic"] == true {
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
                self.keep_handback(&text, f["origin"]["from"].as_str(), fx);
            }
            let source = match kind {
                Some("task-notification") => "task",
                Some("peer") => "agent",
                Some(k) => k,
                None => "synthetic",
            };
            let mut item = json!({ "kind": "event", "id": id, "source": source, "text": text, "ts": now_ms() });
            if let Some(from) = f["origin"]["from"].as_str() {
                item["from"] = from.into();
            }
            self.append(item, fx);
            fx.save = true;
            return;
        }
        let images = blocks.iter().filter(|b| b["type"] == "image").count() as u32;
        let files: Vec<String> = blocks
            .iter()
            .filter(|b| b["type"] == "document")
            .map(|b| b["title"].as_str().unwrap_or("document").to_string())
            .collect();
        if text.trim().is_empty() && images == 0 && files.is_empty() {
            return;
        }
        self.push_user_from(&id, &text, images, &files, Some("remote"), fx);
    }

    pub(super) fn on_user(&mut self, f: &Value, fx: &mut Effects) {
        if f["isReplay"] == true {
            return self.on_replay(f, fx);
        }
        let content = &f["message"]["content"];
        if let Some(text) = content.as_str() {
            if let Some(parts) = localcmd::parse(text) {
                self.local_output(f["uuid"].as_str(), &parts, fx);
            }
            return;
        }
        let Some(blocks) = content.as_array() else {
            return;
        };
        if !blocks.iter().any(|b| b["type"] == "tool_result") {
            let text: Vec<&str> = blocks.iter().filter_map(|b| b["text"].as_str()).collect();
            if let Some(parts) = localcmd::parse(&text.join("\n")) {
                self.local_output(f["uuid"].as_str(), &parts, fx);
            }
            return;
        }
        // A tool's result in its main chain is an entry of its session too: the last one of a turn
        // stopped (or failed) before any reply, which a copy must not lose.
        if f["parent_tool_use_id"].is_null() && blocks.iter().any(|b| b["type"] == "tool_result") {
            if let Some(uuid) = f["uuid"].as_str() {
                self.set_last_entry(Some(uuid.to_string()), fx);
            }
        }
        let tur = &f["tool_use_result"];
        for b in blocks.iter().filter(|b| b["type"] == "tool_result") {
            let Some(id) = b["tool_use_id"].as_str() else {
                continue;
            };
            let is_error = b["is_error"].as_bool().unwrap_or(false);
            if self.plan_calls.remove(id) {
                fx.plan_files = true;
            }
            if self.plan_removals.remove(id) && !is_error {
                self.plan_apply(fx, PlanState::finish_files);
            }
            let text = tool_result_text(&b["content"]);
            self.plan_apply(fx, |plan| {
                plan.on_tool_result(id, is_error, &text, tur, now_ms())
            });
            let result = tool_result(text, is_error, tur);
            if let Some(path) = result["filePath"].as_str() {
                self.touch(path, fx);
            }
            self.patch(
                id,
                json!({ "status": if is_error { "error" } else { "ok" }, "result": result }),
                fx,
            );
        }
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

    pub(super) fn on_result(&mut self, f: &Value, fx: &mut Effects) {
        let interrupted = std::mem::take(&mut self.interrupted);
        let resets = self.limit_resets_at.take();
        let limited = std::mem::take(&mut self.rate_limited);
        // The exact figures below replace the running estimate.
        self.live.clear();
        let is_error = f["is_error"].as_bool().unwrap_or(false) || f["subtype"] != "success";
        if limited && is_error && !interrupted {
            fx.limited = Some(resets);
        } else if self.meta.resume_at.take().is_some() {
            // It went on anyway (a turn Claude Code started by itself): nothing left to resume.
            fx.save = true;
            fx.agent_changed = true;
        }
        let (mut tokens, mut cost) = (0u64, 0f64);
        if let Some(models) = f["modelUsage"].as_object() {
            // The conversation's latest model, else the one that read the most (subagents may use
            // others; the figures are cumulative, so an earlier model may still read more).
            let m = &self.main_model;
            let current = models
                .iter()
                .find(|(k, _)| {
                    !m.is_empty() && (k.starts_with(m.as_str()) || m.starts_with(k.as_str()))
                })
                .map(|(_, u)| u);
            let main = current.or_else(|| {
                models.values().max_by_key(|u| {
                    [
                        "inputTokens",
                        "cacheReadInputTokens",
                        "cacheCreationInputTokens",
                    ]
                    .iter()
                    .filter_map(|k| u[*k].as_u64())
                    .sum::<u64>()
                })
            });
            if let Some(window) = main.and_then(|u| u["contextWindow"].as_u64()) {
                self.context_window = window;
            }
            for (model, u) in models {
                let cur = Counters {
                    input: u["inputTokens"].as_u64().unwrap_or(0),
                    output: u["outputTokens"].as_u64().unwrap_or(0),
                    cache: u["cacheReadInputTokens"].as_u64().unwrap_or(0)
                        + u["cacheCreationInputTokens"].as_u64().unwrap_or(0),
                    cost: u["costUSD"].as_f64().unwrap_or(0.0),
                };
                let prev = self.last_usage.get(model).cloned().unwrap_or_default();
                let delta = if cur.cost + 1e-12 < prev.cost || cur.output < prev.output {
                    cur.clone()
                } else {
                    Counters {
                        input: cur.input.saturating_sub(prev.input),
                        output: cur.output.saturating_sub(prev.output),
                        cache: cur.cache.saturating_sub(prev.cache),
                        cost: (cur.cost - prev.cost).max(0.0),
                    }
                };
                self.last_usage.insert(model.clone(), cur);
                let total = delta.input + delta.output + delta.cache;
                if total == 0 && delta.cost == 0.0 {
                    continue;
                }
                tokens += total;
                cost += delta.cost;
                let name = u["canonicalModel"].as_str().unwrap_or(model).to_string();
                fx.turns.push(TurnRow {
                    engine: self.meta.engine,
                    model: name,
                    input: delta.input,
                    cache: delta.cache,
                    output: delta.output,
                    cost: delta.cost,
                });
            }
        }
        self.meta.tokens += tokens;
        self.meta.cost += cost;
        let error = if is_error && !interrupted {
            f["result"].as_str().map(str::to_string).or_else(|| {
                f["errors"].as_array().map(|e| {
                    e.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
            })
        } else {
            None
        };
        let failed = is_error && !interrupted;
        // A sign-in that could not be refreshed: the core looks at the account's (ESC-162).
        let sign_in = failed && !limited && error.as_deref().is_some_and(crate::signin::lost);
        if sign_in {
            fx.sign_in_lost = true;
        } else {
            self.sign_in_retries = 0;
        }
        // Always with words: the frame's, else the kind of result it is, else a generic one.
        let failure = failed.then(|| {
            error
                .clone()
                .filter(|e| !e.trim().is_empty())
                .or_else(|| {
                    f["subtype"]
                        .as_str()
                        .filter(|s| !s.is_empty() && *s != "success")
                        .map(str::to_string)
                })
                .unwrap_or_else(|| {
                    tr!(
                        "Claude Code a signalé une erreur",
                        "Claude Code reported an error"
                    )
                })
        });
        // The final reply: the text that no tool followed, else what Claude Code makes of it.
        let reply = match std::mem::take(&mut self.final_text) {
            text if !text.trim().is_empty() => text,
            _ => f["result"].as_str().unwrap_or_default().to_string(),
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
            TurnEnd::Finished(if text.trim().is_empty() {
                f["result"].as_str().unwrap_or_default().to_string()
            } else {
                text
            })
        };
        self.forget_turn();
        self.close_open_items(fx);
        // A subagent in the foreground did not outlive the turn: it was stopped.
        self.plan_apply(fx, |plan| plan.end_turn(now_ms()));
        // The turn's last commands may have written the ledger.
        fx.plan_files = true;
        // Stopped by the usage limit: the card offers to go on, on another account.
        let at_limit = limited && is_error && !interrupted;
        // The turn that follows a resume on another account ended. Failing, it is the resume's
        // failure, and the agent keeps the account it can go back to; otherwise (done, at the limit
        // there too, interrupted) it came from nowhere any more.
        if !matches!(end, TurnEnd::Error(_)) && self.meta.moved_from.take().is_some() {
            self.meta.moved_resume_at = None;
            fx.save = true;
            fx.agent_changed = true;
        }
        let item = json!({
            "kind": "turn", "id": f["uuid"].as_str().map(str::to_string).unwrap_or_else(new_id), "ts": now_ms(),
            "durationMs": f["duration_ms"], "cost": cost, "tokens": tokens,
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

    pub(super) fn on_control_request(&mut self, f: &Value, fx: &mut Effects) {
        let Some(rid) = f["request_id"].as_str() else {
            return;
        };
        let req = &f["request"];
        match req["subtype"].as_str().unwrap_or("") {
            "can_use_tool" => {
                let tool = req["tool_name"].as_str().unwrap_or("").to_string();
                let input = req["input"].clone();
                let tool_use_id = req["tool_use_id"].as_str().unwrap_or("").to_string();
                // The reading of a file the user joined by its own path (the file itself, nothing
                // else, never a change): allowed on the spot, no card and nobody waited for. The
                // user chose that file; `--add-dir` cannot give a folder that was not known at the
                // start. Any other request, or one the process no longer takes, goes on below.
                if attachfiles::granted(&self.meta.granted_files, &tool, &input) {
                    let allowed = PermissionAnswer::Allow {
                        call: tool_use_id.clone(),
                        input: input.clone(),
                    };
                    if self
                        .proc
                        .as_ref()
                        .is_some_and(|p| p.answer_permission(rid, allowed).is_ok())
                    {
                        log::info!(
                            "agent {}: a file joined by its path is read without a question",
                            self.meta.id
                        );
                        return;
                    }
                }
                let suggestions = req["permission_suggestions"].clone();
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
                    let can_always = suggestions.as_array().is_some_and(|a| !a.is_empty())
                        && !req["suppress_always_allow_rule"].as_bool().unwrap_or(false);
                    json!({
                        "kind": "permission", "id": rid, "toolUseId": tool_use_id, "toolName": tool,
                        "title": req["title"], "description": req["description"],
                        "input": trim_strings(&input),
                        "reason": req["decision_reason"].as_str().map(strip_ansi),
                        "canAlways": can_always, "defaultNo": req["default_to_no"].as_bool().unwrap_or(false),
                        "decision": null, "ts": now_ms(),
                    })
                };
                let mut item = item;
                if let Some(sub) = self.subagent_of(req) {
                    // Shown in the agent's conversation still (no `parent`), and in its subagent's.
                    item["sub"] = sub.into();
                }
                self.append(item, fx);
                let view = pending_view(rid, &tool, req, &input, &self.meta.cwd);
                self.add_pending(PendingReq {
                    item_id: rid.to_string(),
                    tool_use_id,
                    input,
                    suggestions,
                    view,
                    since: now_ms(),
                });
                self.set_status(AgentStatus::Waiting, fx);
                fx.notify = Some(AgentAlert::new(NotifyKind::Question, body));
                fx.agent_changed = true;
            }
            "elicitation" => {
                if let Some(p) = &self.proc {
                    let _ = p.reply(rid, Ok(json!({ "action": "decline" })));
                }
            }
            other => {
                if let Some(p) = &self.proc {
                    let _ = p.reply(rid, Err(format!("unsupported request: {other}")));
                }
            }
        }
    }

    pub(super) fn on_cancel(&mut self, f: &Value, fx: &mut Effects) {
        let Some(rid) = f["request_id"].as_str() else {
            return;
        };
        if let Some(p) = self.take_pending(rid) {
            self.patch(&p.item_id, json!({ "cancelled": true }), fx);
            if self.pending.is_empty() && self.meta.status == AgentStatus::Waiting {
                self.set_status(AgentStatus::Running, fx);
            }
            fx.agent_changed = true;
        }
    }
}

fn parse_rate_event(info: &Value) -> Vec<crate::model::QuotaMeter> {
    let w = &info["unifiedWindows"];
    let parse = |k: &str| {
        w.get(k).filter(|v| v.is_object()).map(|v| RateWindow {
            pct: v["utilization"].as_f64().unwrap_or(0.0) * 100.0,
            resets_at: v["resetsAt"].as_i64().map(|s| s * 1000),
        })
    };
    crate::model::claude_meters(parse("five_hour"), parse("seven_day"))
}

/// A text of the agent once written whole (its passage to be read aloud is kept apart by
/// `AgentRt::push`, for every harness).
fn final_text(text: &str) -> Value {
    json!({ "text": text, "streaming": false })
}
