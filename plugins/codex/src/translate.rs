//! Codex's messages (`codex app-server`, docs/PROTOCOL.md, « Codex app-server ») translated into the normalized items of
//! the conversation (`Item`), which the core applies to the agent. The state of the agent stays the core's: what a rule
//! needs of it comes in `Ctx`.
//!
//! Its tools take the names Claude Code gives them (docs/TODO-1.8.md, « Un moteur par agent »): `commandExecution` is
//! `Bash`, `fileChange` is `Edit` or `Write` (with its +N −M), `mcpToolCall` is `mcp__<server>__<tool>`, `webSearch` is
//! `WebSearch`, and a subagent (`subAgentActivity`) is a `Task`. A subagent has a thread of its own, whose messages come
//! on the same connection: they are told apart by their `threadId`, and shown under its `Task`.
//!
//! A recorded stream (`tests/fixtures/codex/`, one `{"dir", "msg"}` a line) is read as it came: what Codex said (`s2c`)
//! is translated, what was said to it is left out.

use crate::prices;
use crate::words::{self, id_key, mode_of, plan_approval, said};
use escouade_acp::git;
use escouade_plugin_protocol::engine::{hunks_of, Item, TurnUsage};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// What the conversation holds, as a rule needs it.
pub trait Seen {
    /// Whether an item of that id is in the conversation.
    fn has(&self, id: &str) -> bool;
    /// The call `id` (`{ name, status }`), if it is in the conversation.
    fn call(&self, id: &str) -> Option<Value>;
}

/// What the translation reads of the agent.
pub struct Ctx<'a> {
    pub cwd: &'a str,
    /// Escouade's mode.
    pub mode: &'a str,
    pub model: &'a str,
    /// A turn runs.
    pub active: bool,
    /// The user stopped the turn.
    pub interrupted: bool,
    /// A private project's fence is set: git reads no file for it out of the sandbox.
    pub fenced: bool,
    /// A process answers its requests: else (a recorded stream, its interface) each has its card.
    pub process: bool,
    /// The language of the app.
    pub lang: &'a str,
    /// The agent's session, or the one it means to resume.
    pub session: Option<&'a str>,
    /// It is a copy of another session, which has none of its own yet.
    pub copy: bool,
    pub seen: &'a dyn Seen,
}

/// What the translation keeps of Codex's conversation between its messages (not saved).
#[derive(Default)]
pub struct Translator {
    /// The threads of its subagents, and the call (`Task`) of each.
    children: HashMap<String, String>,
    /// What each subagent said last: its report, given to its call when it ends.
    reports: HashMap<String, String>,
    /// The calls of subagents that ended.
    ended: HashSet<String>,
    /// The turn of its thread under way.
    pub turn: Option<String>,
    /// The tokens of each thread so far (`total` of `thread/tokenUsage/updated`), and those of the
    /// threads when the turn began: a turn's are the difference.
    tokens: HashMap<String, Tokens>,
    tokens_at_start: HashMap<String, Tokens>,
    /// The plan of the mode Plan, its turn over on it: it waits for its approval.
    plan: Option<String>,
    /// The files of each `fileChange`, by call, for the card of its approval.
    changes: HashMap<String, Value>,
    /// When a usage limit met resets (ms).
    limit_resets: Option<i64>,
    /// What went wrong in the turn, as Codex said it (`error`).
    error: Option<String>,
    /// The requests Escouade's rules accepted, for the client to answer: `git add` and `git commit` of its worktree.
    pub approved: Vec<Accepted>,
}

/// A request Escouade's rules accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    pub request: String,
    pub command: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Tokens {
    input: u64,
    cached: u64,
    output: u64,
}

impl Tokens {
    fn of(v: &Value) -> Self {
        let n = |k: &str| v[k].as_u64().unwrap_or(0);
        Tokens {
            input: n("inputTokens"),
            cached: n("cachedInputTokens") + n("cacheWriteInputTokens"),
            output: n("outputTokens"),
        }
    }

    fn minus(self, before: Tokens) -> Tokens {
        Tokens {
            input: self.input.saturating_sub(before.input),
            cached: self.cached.saturating_sub(before.cached),
            output: self.output.saturating_sub(before.output),
        }
    }
}

/// Where a message of Codex belongs.
enum Thread {
    /// The agent's own.
    Own,
    /// A subagent's, under its call.
    Child(String),
    /// Neither (one of a subagent not known yet).
    Other,
}

impl Translator {
    /// One message of its `codex app-server`, of a recorded stream, or of its interface's rollout.
    pub fn frame(&mut self, f: &Value, c: &Ctx, out: &mut Vec<Item>) {
        // A recorded stream: what Codex said, as it came.
        if let Some(dir) = f.get("dir").and_then(Value::as_str) {
            if dir == "s2c" {
                self.frame(&f["msg"], c, out);
            }
            return;
        }
        let Some(method) = f["method"].as_str() else {
            return;
        };
        let p = &f["params"];
        let thread = self.thread(p["threadId"].as_str(), c);
        let parent = match &thread {
            Thread::Child(call) => Some(call.clone()),
            _ => None,
        };
        let own = matches!(thread, Thread::Own);
        match method {
            "thread/started" => self.thread_started(&p["thread"], c, out),
            "thread/settings/updated" if own => {
                if let Some(mode) = mode_of(&p["threadSettings"], c.mode) {
                    out.push(Item::Mode { mode: mode.into() });
                }
            }
            "turn/started" if own => {
                self.turn = p["turn"]["id"].as_str().map(str::to_string);
                self.tokens_at_start = self.tokens.clone();
                self.error = None;
                let session = p["threadId"].as_str().or(c.session).map(str::to_string);
                out.push(Item::Start { session });
            }
            "turn/completed" if own => {
                let turn = &p["turn"];
                if self.turn.is_none() || self.turn.as_deref() == turn["id"].as_str() {
                    self.turn_end(turn, c, out);
                }
            }
            "item/started" | "item/completed" if !matches!(thread, Thread::Other) => {
                self.item(&p["item"], parent, method == "item/completed", c, out);
            }
            "item/agentMessage/delta"
            | "item/reasoning/summaryTextDelta"
            | "item/reasoning/textDelta" => {
                let (Some(id), Some(text)) = (p["itemId"].as_str(), p["delta"].as_str()) else {
                    return;
                };
                if text.is_empty() || matches!(thread, Thread::Other) {
                    return;
                }
                out.push(Item::Delta {
                    id: id.to_string(),
                    text: text.to_string(),
                    // A reasoning is shown from its first words (Codex often has none).
                    thinking: method != "item/agentMessage/delta" || !c.seen.has(id),
                    parent,
                });
            }
            "turn/plan/updated" if own => {
                let todos: Vec<Value> = p["plan"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|s| {
                        let status = match s["status"].as_str() {
                            Some("inProgress") => "in_progress",
                            Some("completed") => "completed",
                            _ => "pending",
                        };
                        json!({ "content": s["step"], "status": status, "activeForm": s["step"] })
                    })
                    .collect();
                out.push(Item::Todos {
                    todos: Value::Array(todos),
                });
            }
            "thread/tokenUsage/updated" => {
                let Some(t) = p["threadId"].as_str() else {
                    return;
                };
                let usage = &p["tokenUsage"];
                self.tokens
                    .insert(t.to_string(), Tokens::of(&usage["total"]));
                if own {
                    // What its context holds, as Codex counts it (`totalTokens` of the last
                    // call): after a compaction, its `inputTokens` are 0 (full-turn.jsonl).
                    let last = &usage["last"];
                    out.push(Item::Context {
                        tokens: Some(
                            last["totalTokens"]
                                .as_u64()
                                .unwrap_or_else(|| Tokens::of(last).input),
                        ),
                        window: usage["modelContextWindow"].as_u64(),
                    });
                }
            }
            "account/rateLimits/updated" => {
                let limits = &p["rateLimits"];
                let meters = words::meters(limits);
                if !limits["rateLimitReachedType"].is_null() {
                    let reset = |full: bool| {
                        meters
                            .iter()
                            .filter(|m| !full || m["pct"].as_f64().unwrap_or(0.0) >= 100.0)
                            .filter_map(|m| m["resetsAt"].as_i64())
                            .max()
                    };
                    self.limit_resets = reset(true).or_else(|| reset(false));
                }
                if !meters.is_empty() {
                    out.push(Item::Quota {
                        meters: Value::Array(meters),
                    });
                }
            }
            "item/commandExecution/requestApproval"
            | "item/fileChange/requestApproval"
            | "item/permissions/requestApproval" => self.approval(method, f, parent, c, out),
            "item/tool/requestUserInput" => question(f, parent, out),
            // In Auto, Codex's automatic reviewer decided a request on its own: the conversation
            // says so, as it was asked of nobody.
            "guardianWarning" if !matches!(thread, Thread::Other) => {
                if let Some(message) = p["message"].as_str().filter(|m| !m.trim().is_empty()) {
                    out.push(notice(
                        "info",
                        said(
                            c.lang,
                            format!("Relecteur automatique de Codex : {message}"),
                            format!("Codex’s automatic reviewer: {message}"),
                        ),
                    ));
                }
            }
            // Closed by Codex itself (an interrupted turn): its card goes.
            "serverRequest/resolved" => out.push(Item::Answered {
                request: id_key(&p["requestId"]),
                decision: None,
                answers: None,
            }),
            "error" => {
                if p["willRetry"] == true {
                    return;
                }
                let message = error_text(&p["error"]);
                if p["refused"] == "turn/start" && c.active {
                    // The turn never began: it ends here, in error.
                    let turn = json!({ "status": "failed", "error": { "message": message } });
                    self.turn_end(&turn, c, out);
                } else if !matches!(thread, Thread::Child(_)) {
                    self.error = Some(message);
                }
            }
            _ => {}
        }
    }

    /// Whose a message of the thread `thread` is.
    fn thread(&self, thread: Option<&str>, c: &Ctx) -> Thread {
        let Some(t) = thread else {
            return Thread::Own;
        };
        if let Some(call) = self.children.get(t) {
            return Thread::Child(call.clone());
        }
        match c.session {
            Some(own) if own != t => Thread::Other,
            _ => Thread::Own,
        }
    }

    /// The agent's thread, new, resumed or copied: its session.
    fn thread_started(&mut self, thread: &Value, c: &Ctx, out: &mut Vec<Item>) {
        let Some(id) = thread["id"].as_str() else {
            return;
        };
        if !thread["parentThreadId"].is_null() || self.children.contains_key(id) {
            return;
        }
        if c.session != Some(id) && c.session.is_some() && !c.copy {
            out.push(notice(
                "warn",
                said(
                    c.lang,
                    "La conversation Codex n’a pas pu être reprise : nouvelle conversation (contexte vidé)",
                    "The Codex conversation couldn’t be resumed: new conversation (context cleared)",
                ),
            ));
        }
        out.push(Item::Session {
            session: id.to_string(),
        });
    }

    /// An item of a turn, as it starts (`done` false) and once it is over.
    fn item(
        &mut self,
        item: &Value,
        parent: Option<String>,
        done: bool,
        c: &Ctx,
        out: &mut Vec<Item>,
    ) {
        let Some(id) = item["id"].as_str() else {
            return;
        };
        let main = parent.is_none();
        match item["type"].as_str().unwrap_or_default() {
            "agentMessage" => {
                let text = item["text"].as_str().unwrap_or_default();
                if !done {
                    // Its item, empty until its words come; the agent is writing.
                    out.push(Item::Delta {
                        id: id.to_string(),
                        text: String::new(),
                        thinking: false,
                        parent,
                    });
                    return;
                }
                if let Some(call) = &parent {
                    if !text.trim().is_empty() {
                        self.reports.insert(call.clone(), text.to_string());
                    }
                }
                out.push(Item::Text {
                    text: text.to_string(),
                    parent,
                    id: Some(id.to_string()),
                });
            }
            "reasoning" => {
                if !done {
                    if main {
                        out.push(Item::Activity {
                            tool: None,
                            input: Value::Null,
                            block: Some("thinking".into()),
                        });
                    }
                    return;
                }
                let parts = |k: &str| -> Vec<String> {
                    item[k]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|s| s.as_str().map(str::to_string))
                        .filter(|s| !s.trim().is_empty())
                        .collect()
                };
                let mut text = parts("summary");
                if text.is_empty() {
                    text = parts("content");
                }
                let text = text.join("\n\n");
                if c.seen.has(id) || !text.is_empty() {
                    out.push(Item::Thinking {
                        text,
                        parent,
                        id: Some(id.to_string()),
                    });
                }
            }
            "commandExecution" => {
                let input = json!({ "command": shown_command(item) });
                let new = parent.is_none() && !c.seen.has(id);
                call(id, "Bash", input, parent, c, out);
                if let Some((tool, what)) = command_tool(item).filter(|_| new) {
                    // Said as Claude Code says it: it reads a file with Read, not with a command.
                    out.push(Item::Activity {
                        tool: Some(tool.into()),
                        input: what,
                        block: None,
                    });
                }
                if !done {
                    return;
                }
                let ok =
                    item["status"] == "completed" && item["exitCode"].as_i64().unwrap_or(0) == 0;
                let text = item["aggregatedOutput"].as_str().unwrap_or_default();
                result(id, !ok, text.to_string(), Value::Null, c, out);
            }
            "fileChange" => {
                self.changes.insert(id.to_string(), item["changes"].clone());
                let changes = item["changes"].as_array().cloned().unwrap_or_default();
                for (n, change) in changes.iter().enumerate() {
                    let at = if n == 0 {
                        id.to_string()
                    } else {
                        format!("{id}:{n}")
                    };
                    let (name, input, tur) = file_change(change);
                    call(&at, name, input, parent.clone(), c, out);
                    if done {
                        let failed = item["status"] != "completed";
                        if let (false, Some(path)) = (failed, change["path"].as_str()) {
                            out.push(Item::Touched { path: path.into() });
                        }
                        result(&at, failed, String::new(), tur, c, out);
                    }
                }
            }
            "mcpToolCall" => {
                let name = mcp_name(
                    item["server"].as_str().unwrap_or_default(),
                    item["tool"].as_str().unwrap_or_default(),
                );
                call(id, &name, item["arguments"].clone(), parent, c, out);
                if done {
                    let failed = !item["error"].is_null() || item["status"] == "failed";
                    let text = if failed {
                        item["error"]["message"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string()
                    } else {
                        content_text(&item["result"]["content"])
                    };
                    result(id, failed, text, Value::Null, c, out);
                }
            }
            "dynamicToolCall" => {
                let name = item["tool"].as_str().unwrap_or("tool").to_string();
                call(id, &name, item["arguments"].clone(), parent, c, out);
                if done {
                    let failed = item["success"] == false || item["status"] == "failed";
                    let text = content_text(&item["contentItems"]);
                    result(id, failed, text, Value::Null, c, out);
                }
            }
            "webSearch" => {
                let query = item["query"]
                    .as_str()
                    .filter(|q| !q.is_empty())
                    .or_else(|| item["action"]["query"].as_str())
                    .unwrap_or_default();
                call(id, "WebSearch", json!({ "query": query }), parent, c, out);
                if done {
                    result(id, false, String::new(), Value::Null, c, out);
                }
            }
            "imageView" => {
                let input = json!({ "file_path": item["path"] });
                call(id, "Read", input, parent, c, out);
                if done {
                    result(id, false, String::new(), Value::Null, c, out);
                }
            }
            "subAgentActivity" => {
                let Some(child) = item["agentThreadId"].as_str() else {
                    return;
                };
                match item["kind"].as_str() {
                    Some("started") => {
                        if self.children.contains_key(child) {
                            return;
                        }
                        self.children.insert(child.to_string(), id.to_string());
                        let name = item["agentPath"]
                            .as_str()
                            .and_then(|p| p.rsplit('/').next())
                            .unwrap_or("subagent")
                            .replace('_', " ");
                        let input = json!({
                            "description": name, "prompt": "", "model": item["model"],
                        });
                        call(id, "Task", input, parent, c, out);
                    }
                    Some("completed") => {
                        let Some(task) = self.children.get(child).cloned() else {
                            return;
                        };
                        if !self.ended.insert(task.clone()) {
                            return;
                        }
                        let report = self.reports.remove(&task).unwrap_or_default();
                        result(&task, false, report, Value::Null, c, out);
                    }
                    _ => {}
                }
            }
            // Asked (`/compact`, ESC-205) or by Codex itself: from its start to its end.
            "contextCompaction" if main => {
                let text = if done {
                    said(c.lang, "Contexte compacté", "Context compacted")
                } else {
                    said(c.lang, "Compaction du contexte…", "Compacting the context…")
                };
                out.push(notice("info", text));
            }
            // The plan of the mode Plan: approved once the turn is over on it.
            "plan" if done && main => {
                let text = item["text"].as_str().unwrap_or_default();
                if !text.trim().is_empty() {
                    self.plan = Some(text.to_string());
                }
            }
            _ => {}
        }
    }

    /// A request of Codex to go on: a command, a change of files or more permissions.
    fn approval(
        &mut self,
        method: &str,
        f: &Value,
        parent: Option<String>,
        c: &Ctx,
        out: &mut Vec<Item>,
    ) {
        let p = &f["params"];
        let rid = id_key(&f["id"]);
        let at = p["itemId"].as_str().unwrap_or_default().to_string();
        let asked = c.seen.call(&at).unwrap_or_default();
        let cwd = p["cwd"].as_str().unwrap_or(c.cwd);
        let mut description: Option<String> = None;
        let mut hunks = Value::Null;
        let (tool, input) = match method {
            "item/commandExecution/requestApproval" => {
                let command = shown_command(p);
                // « Édits auto » (and Auto): `git add` and `git commit` of its own worktree, which
                // Codex's sandbox never lets write in `.git`, are accepted at once, judged on the
                // command run, not on its summary (`escouade_acp::git`).
                let actions: Vec<&str> = p["commandActions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|a| a["command"].as_str())
                    .collect();
                if c.process
                    && matches!(c.mode, "acceptEdits" | "auto")
                    && same_dir(cwd, c.cwd)
                    && git::accepts_fenced(
                        p["command"].as_str().unwrap_or_default(),
                        &actions,
                        Path::new(c.cwd),
                        c.fenced,
                    )
                {
                    self.approved.push(Accepted {
                        request: rid,
                        command,
                    });
                    return;
                }
                if git::writes_repository(&command) {
                    description = Some(said(
                        c.lang,
                        "git écrit dans le dépôt du projet, hors du bac à sable de Codex.",
                        "git writes in the project’s repository, outside Codex’s sandbox.",
                    ));
                } else if !same_dir(cwd, c.cwd) {
                    description = Some(said(c.lang, format!("Dans {cwd}"), format!("In {cwd}")));
                }
                ("Bash".to_string(), json!({ "command": command }))
            }
            "item/fileChange/requestApproval" => {
                let changes = self.changes.get(&at).cloned().unwrap_or_default();
                let first = changes.get(0).cloned().unwrap_or_default();
                let (name, input, tur) = file_change(&first);
                // A file added has no hunks of its own: shown whole.
                hunks = match tur["content"].as_str().filter(|_| tur["type"] == "create") {
                    Some(content) => {
                        let lines: Vec<String> = content.lines().map(|l| format!("+{l}")).collect();
                        json!([{ "oldStart": 0, "newStart": 1, "lines": lines }])
                    }
                    None => tur["structuredPatch"].clone(),
                };
                let more = changes.as_array().map_or(0, |a| a.len().saturating_sub(1));
                if more > 0 {
                    description = Some(said(
                        c.lang,
                        format!("et {more} autre(s) fichier(s)"),
                        format!("and {more} other file(s)"),
                    ));
                }
                let name = asked["name"].as_str().unwrap_or(name).to_string();
                (name, input)
            }
            _ => {
                description = Some(permissions_text(&p["permissions"], c.lang));
                (
                    "Permissions".to_string(),
                    json!({ "permissions": p["permissions"] }),
                )
            }
        };
        out.push(Item::PermissionCard {
            request: rid,
            call: at,
            tool,
            input,
            description,
            reason: p["reason"].as_str().map(str::to_string),
            hunks,
            can_always: true,
            always_scope: Some("session".into()),
            parent,
        });
    }

    /// The end of the agent's turn (`turn/completed`, or a `turn/start` refused).
    pub fn turn_end(&mut self, turn: &Value, c: &Ctx, out: &mut Vec<Item>) {
        let status = turn["status"].as_str().unwrap_or("completed");
        let interrupted = c.interrupted || status == "interrupted";
        // Where a copy made afterwards forks the thread (`thread/fork` with `lastTurnId`).
        let entry = turn["id"]
            .as_str()
            .filter(|_| status != "failed")
            .map(str::to_string);
        self.turn = None;
        // The mode Plan's plan: the turn stays open on its card until it is approved, as Claude
        // Code's `ExitPlanMode` does (its work, or more planning, is the turn's next part).
        if status == "completed" && !interrupted {
            if let Some(plan) = self.plan.take() {
                let rid = plan_approval(turn["id"].as_str().unwrap_or("turn"));
                out.push(Item::Entry { entry });
                out.push(Item::Permission {
                    request: rid.clone(),
                    call: rid,
                    tool: Some("ExitPlanMode".into()),
                    input: Some(json!({ "plan": plan })),
                    description: None,
                    reason: None,
                    diff: None,
                    can_always: true,
                    parent: None,
                    hunks: None,
                    always_by_harness: true,
                    ends_stream: true,
                });
                return;
            }
        }
        self.plan = None;
        let error = &turn["error"];
        let info = error["codexErrorInfo"].as_str();
        let limited = !interrupted
            && status == "failed"
            && matches!(info, Some("usageLimitExceeded" | "rateLimitExceeded"));
        let failed = status == "failed" && !interrupted;
        let message = failed.then(|| {
            Some(error_text(error))
                .filter(|m| !m.trim().is_empty())
                .or_else(|| self.error.take())
                .unwrap_or_else(|| {
                    said(
                        c.lang,
                        "Codex a signalé une erreur",
                        "Codex reported an error",
                    )
                })
        });
        // Its tokens: what each thread (its subagents' included) counted since the turn began.
        let mut used = Tokens::default();
        for (thread, now) in &self.tokens {
            let before = self
                .tokens_at_start
                .get(thread)
                .copied()
                .unwrap_or_default();
            let d = now.minus(before);
            used.input += d.input;
            used.cached += d.cached;
            used.output += d.output;
        }
        self.tokens_at_start = self.tokens.clone();
        let fresh = used.input.saturating_sub(used.cached);
        // Codex tells no cost: estimated from the public prices of OpenAI's models ('estimé').
        let cost = prices::estimate(c.model, fresh, used.cached, used.output).unwrap_or(0.0);
        if limited {
            out.push(Item::LimitRest);
        }
        if let Some(ms) = turn["durationMs"].as_i64() {
            out.push(Item::Took { ms });
        }
        out.push(Item::Turn {
            interrupted,
            limited: limited.then(|| self.limit_resets.take()),
            usage: TurnUsage {
                input: fresh,
                output: used.output,
                cache: used.cached,
                cost,
            },
            error: message,
            entry,
        });
    }
}

fn notice(level: &str, text: impl Into<String>) -> Item {
    Item::Notice {
        text: text.into(),
        level: level.into(),
    }
}

/// The call `id` of the tool `name` (Claude Code's name), shown once as it starts.
fn call(id: &str, name: &str, input: Value, parent: Option<String>, c: &Ctx, out: &mut Vec<Item>) {
    if c.seen.has(id) {
        return;
    }
    out.push(Item::Tool {
        id: id.to_string(),
        name: name.to_string(),
        input,
        parent,
        unplanned: false,
    });
}

/// The result of the call `id`: `text`, and for a file `tur` (as Claude Code's `tool_use_result`: its patch and its
/// path).
fn result(id: &str, error: bool, output: String, tur: Value, c: &Ctx, out: &mut Vec<Item>) {
    let call = c.seen.call(id);
    // Over already, the turn closed it, or `item/started` and its end were told together.
    if call.is_some_and(|i| i["status"] != "running") {
        return;
    }
    out.push(Item::ToolDone {
        id: id.to_string(),
        output,
        error,
        result: tur,
    });
}

/// A question of Codex (`item/tool/requestUserInput`, in the mode Plan), as the card of a question of Claude Code's
/// shows it; its questions keep their ids, for the answer.
fn question(f: &Value, parent: Option<String>, out: &mut Vec<Item>) {
    let p = &f["params"];
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
                "id": q["id"], "question": q["question"], "header": q["header"].as_str().unwrap_or_default(),
                "multiSelect": false, "options": options,
            })
        })
        .collect();
    out.push(Item::Question {
        request: id_key(&f["id"]),
        call: p["itemId"].as_str().unwrap_or_default().to_string(),
        questions: Value::Array(questions),
        parent,
    });
}

/// The name of a tool of an MCP server, as Claude Code gives it.
fn mcp_name(server: &str, tool: &str) -> String {
    format!("mcp__{server}__{tool}")
}

/// The text of an MCP result's blocks.
fn content_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// What Codex said went wrong: its message, the API's own when it is one in JSON.
fn error_text(error: &Value) -> String {
    let message = error["message"].as_str().unwrap_or_default();
    serde_json::from_str::<Value>(message)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| message.to_string())
}

/// The command as the model wrote it (`commandActions`), not wrapped in its shell
/// (`pwsh.exe -Command '…'` under Windows).
fn shown_command(v: &Value) -> String {
    let actions: Vec<&str> = v["commandActions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| a["command"].as_str())
        .collect();
    if actions.is_empty() {
        v["command"].as_str().unwrap_or_default().to_string()
    } else {
        actions.join(" && ")
    }
}

/// The tool of Claude Code for a command that only reads a file (`Read`) or only searches
/// (`Grep`), as `commandActions` tells it; None for any other command, or several actions.
fn command_tool(v: &Value) -> Option<(&'static str, Value)> {
    let [action] = v["commandActions"].as_array()?.as_slice() else {
        return None;
    };
    match action["type"].as_str()? {
        "read" => Some(("Read", json!({ "file_path": action["path"].as_str()? }))),
        "search" => Some(("Grep", json!({ "pattern": action["query"].as_str()? }))),
        _ => None,
    }
}

/// Whether two folders are the same, whatever their slashes and case.
pub fn same_dir(a: &str, b: &str) -> bool {
    let norm = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_lowercase();
    norm(a) == norm(b)
}

/// A change of `fileChange`: the tool Claude Code would have called for it, its input, and its
/// result as Claude Code's `tool_use_result` tells a file changed (its patch and its path).
fn file_change(change: &Value) -> (&'static str, Value, Value) {
    let path = change["path"].as_str().unwrap_or_default();
    let diff = change["diff"].as_str().unwrap_or_default();
    match change["kind"]["type"].as_str() {
        Some("add") => (
            "Write",
            json!({ "file_path": path, "content": diff }),
            json!({ "type": "create", "filePath": path, "content": diff, "structuredPatch": [] }),
        ),
        Some("delete") => {
            let lines: Vec<String> = diff.lines().map(|l| format!("-{l}")).collect();
            let hunks = json!([{ "oldStart": 1, "newStart": 0, "lines": lines }]);
            (
                "Edit",
                json!({ "file_path": path }),
                json!({ "filePath": path, "structuredPatch": hunks }),
            )
        }
        _ => {
            let mut input = json!({ "file_path": path });
            if let Some(to) = change["kind"]["move_path"].as_str() {
                input["move_path"] = json!(to);
            }
            (
                "Edit",
                input,
                json!({ "filePath": path, "structuredPatch": hunks_of(diff) }),
            )
        }
    }
}

/// What a request for more permissions asks, in words.
fn permissions_text(p: &Value, lang: &str) -> String {
    let mut parts = Vec::new();
    if p["network"]["enabled"] == true {
        parts.push(said(lang, "l’accès au réseau", "network access"));
    }
    let paths = |k: &str| -> Vec<String> {
        p["fileSystem"][k]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    };
    let write = paths("write");
    if !write.is_empty() {
        let list = write.join(", ");
        parts.push(said(
            lang,
            format!("l’écriture dans {list}"),
            format!("writing in {list}"),
        ));
    }
    let read = paths("read");
    if !read.is_empty() {
        let list = read.join(", ");
        parts.push(said(
            lang,
            format!("la lecture de {list}"),
            format!("reading {list}"),
        ));
    }
    if parts.is_empty() {
        return said(
            lang,
            "Codex demande plus de permissions",
            "Codex asks for more permissions",
        );
    }
    let what = parts.join(", ");
    said(
        lang,
        format!("Codex demande {what}"),
        format!("Codex asks for {what}"),
    )
}
