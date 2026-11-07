//! Copilot's events (`copilot --server --stdio`, docs/PROTOCOL.md, « Copilot SDK ») translated into the normalized
//! items of the conversation (`Item`), which the core applies to the agent. The state of the agent stays the core's:
//! what a rule needs of it comes in `Ctx`.
//!
//! Its tools take the names Claude Code gives them (docs/TODO-1.8.md, point 0, « Noms des outils »): `powershell` and
//! `bash` are `Bash`, `view` `Read`, `glob` `Glob`, `grep` and `rg` `Grep`, `create` `Write`, `edit` and `apply_patch`
//! `Edit` (or `Write` for a file added), `web_fetch` `WebFetch`, `task` `Task`, `skill` `Skill`, and a tool of an MCP
//! server (`mcpServerName`) `mcp__<server>__<tool>`. A subagent (`task`) shares its parent's session: its events carry
//! its `agentId`, and are shown under its `Task`. Its tasks come from the `sql` tool's table, read by the client once
//! `session.todos_changed` says they changed (`escouade.todos`).
//!
//! In `manual`, Copilot asks for everything, reads included: Escouade's rules answer the safe requests without a card
//! (reads in the agent's folder; writes there but in the mode Plan; `git add` and `git commit` of its worktree in
//! « Accepter les modifications » and « Auto »). `Translator::approved` lists them, for the client to answer.
//!
//! A recorded stream (`tests/fixtures/copilot/`, one `{"dir", "msg"}` a line) is read as it came: what Copilot said
//! (`s2c`), the session its `session.create` opened, the tasks it read.

use crate::words::{self, said};
use escouade_acp::git;
use escouade_acp::tools::absolute;
use escouade_plugin_protocol::engine::{hunks_of, Item, TurnUsage};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// What the conversation holds, as a rule needs it.
pub trait Seen {
    /// Whether an item of that id is in the conversation.
    fn has(&self, id: &str) -> bool;
    /// The call `id` (`{ name, input, status }`), if it is in the conversation.
    fn call(&self, id: &str) -> Option<Value>;
}

/// What the translation reads of the agent.
pub struct Ctx<'a> {
    pub cwd: &'a str,
    /// Escouade's mode.
    pub mode: &'a str,
    /// A turn runs.
    pub active: bool,
    /// A discussion of the Docs tab: it writes nothing.
    pub discussion: bool,
    /// A private project's fence is set: `words::fenced` must keep seeing what it asks.
    pub fenced: bool,
    /// A process answers its permissions: else (a recorded stream, its interface's log) each has its card.
    pub process: bool,
    /// The language of the app.
    pub lang: &'a str,
    /// The session the agent means to resume: another one is a new start, which its conversation is told.
    pub resumed: Option<&'a str>,
    pub seen: &'a dyn Seen,
}

/// What the translation keeps of Copilot's conversation between its events (not saved).
#[derive(Default)]
pub struct Translator {
    /// The subagents (`agentId`), and the call (`Task`) of each.
    children: HashMap<String, String>,
    /// What each subagent said last, by its call: its report.
    reports: HashMap<String, String>,
    /// The totals of each subagent once over, by its call.
    totals: HashMap<String, Value>,
    /// The reasoning ids seen growing (a reasoning given whole only may be an opaque blob).
    streamed: HashSet<String>,
    /// The turn's usage so far, its subagents' included.
    usage: Spent,
    /// The last event of its own kept by Copilot: where a copy of it starts from.
    last_event: Option<String>,
    /// The turn was stopped (`abort`).
    pub aborted: bool,
    /// What its interface asks the user, as its `permissionRequest` hook told it (« Vue Copilot »), until its
    /// `notification` says it waits for the answer.
    pub asked: Option<String>,
    /// What went wrong in the turn, as Copilot said it.
    error: Option<String>,
    /// The turn was stopped by the account's limit (`session.error` of type `quota`): when it ends, if told (the
    /// credits given again).
    limited: Option<Option<i64>>,
    /// When the month's credits are given again, as the last snapshot said it.
    credits_reset: Option<i64>,
    /// The prices of the account's models (`billing.tokenPrices` of `models.list`), for a call whose cost Copilot does
    /// not tell.
    prices: HashMap<String, Value>,
    /// The requests Escouade's rules accepted, for the client to answer.
    pub approved: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Spent {
    input: u64,
    cache: u64,
    output: u64,
    /// What its calls cost, in nano-AIU (`copilotUsage.totalNanoAiu` of each).
    nano: u64,
}

/// Where an event of Copilot belongs.
enum Owner {
    Own,
    Child(String),
    Other,
}

/// Now, in epoch milliseconds.
fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

impl Translator {
    /// One message of its `copilot --server --stdio`, of a recorded stream, or of the client (`escouade.*`).
    pub fn frame(&mut self, f: &Value, c: &Ctx, out: &mut Vec<Item>) {
        // A recorded stream: what Copilot said, and the session `session.create` opened.
        if let Some(dir) = f.get("dir").and_then(Value::as_str) {
            let msg = &f["msg"];
            if dir == "s2c" {
                if let Some(id) = msg["result"]["sessionId"].as_str() {
                    if msg["result"]["workspacePath"].is_string() {
                        let opened = json!({ "method": "escouade.session", "params": {
                            "sessionId": id, "start": "recorded",
                        } });
                        self.frame(&opened, c, out);
                    }
                } else if let Some(rows) = msg["result"]["rows"].as_array() {
                    let todos = json!({ "method": "escouade.todos", "params": {
                        "todos": { "rows": rows },
                    } });
                    self.frame(&todos, c, out);
                } else if msg["method"].is_string() {
                    self.frame(msg, c, out);
                }
            }
            return;
        }
        let p = &f["params"];
        match f["method"].as_str().unwrap_or_default() {
            "escouade.session" => self.session(p, c, out),
            "escouade.mode" => {
                if let Some(mode) = p["mode"].as_str() {
                    out.push(Item::Mode { mode: mode.into() });
                }
            }
            "escouade.refused" if c.active => {
                self.error = p["message"].as_str().map(str::to_string);
                self.turn_end(out);
            }
            "escouade.todos" => self.todos(&p["todos"], out),
            "escouade.prices" => {
                if let Some(prices) = p["prices"].as_object() {
                    self.prices = prices.clone().into_iter().collect();
                }
            }
            "session.event" => self.event(&p["event"], c, out),
            _ => {}
        }
    }

    /// The prices of the account's models.
    pub fn set_prices(&mut self, prices: HashMap<String, Value>) {
        self.prices = prices;
    }

    fn owner(&self, agent: Option<&str>) -> Owner {
        match agent {
            None => Owner::Own,
            Some(a) => match self.children.get(a) {
                Some(call) => Owner::Child(call.clone()),
                None => Owner::Other,
            },
        }
    }

    /// The agent's session, new, resumed or copied.
    fn session(&mut self, p: &Value, c: &Ctx, out: &mut Vec<Item>) {
        let Some(id) = p["sessionId"].as_str() else {
            return;
        };
        let start = p["start"].as_str().unwrap_or("new");
        if c.resumed.is_some_and(|s| s != id) && start == "new" {
            out.push(Item::Notice {
                text: said(
                    c.lang,
                    "La session Copilot n’a pas pu être reprise : nouvelle session (contexte vidé)"
                        .into(),
                    "The Copilot session couldn’t be resumed: new session (context cleared)".into(),
                ),
                level: "warn".into(),
            });
        }
        out.push(Item::Session { session: id.into() });
        // Signed in to no account of its own, Copilot takes the GitHub CLI's by itself.
        if p["authType"] == "gh-cli" {
            out.push(Item::Notice {
                text: said(
                    c.lang,
                    "Copilot utilise le compte du GitHub CLI (gh), faute de connexion à lui : connecte-toi avec copilot login pour choisir son compte.".into(),
                    "Copilot uses the GitHub CLI’s account (gh), as it isn’t signed in itself: sign in with copilot login to choose its account.".into(),
                ),
                level: "warn".into(),
            });
        }
    }

    /// The tasks of the session (`session.plan.readSqlTodosWithDependencies`), as a task list.
    fn todos(&mut self, todos: &Value, out: &mut Vec<Item>) {
        let Some(rows) = todos["rows"].as_array() else {
            return;
        };
        let todos: Vec<Value> = rows
            .iter()
            .map(|t| {
                let status = match t["status"].as_str() {
                    Some("in_progress") => "in_progress",
                    Some("done" | "completed" | "cancelled") => "completed",
                    _ => "pending",
                };
                json!({ "content": t["title"], "status": status, "activeForm": t["title"] })
            })
            .collect();
        out.push(Item::Todos {
            todos: Value::Array(todos),
        });
    }

    /// One event of the session (`session.event`).
    pub fn event(&mut self, e: &Value, c: &Ctx, out: &mut Vec<Item>) {
        let Some(kind) = e["type"].as_str() else {
            return;
        };
        let d = &e["data"];
        let owner = self.owner(e["agentId"].as_str());
        let parent = match &owner {
            Owner::Child(call) => Some(call.clone()),
            _ => None,
        };
        let own = matches!(owner, Owner::Own);
        if own && e["ephemeral"] != true {
            if let Some(id) = e["id"].as_str() {
                self.last_event = Some(id.to_string());
            }
        }
        // A subagent's start links its id to its call before its own events come.
        if kind == "subagent.started" {
            if let (Some(agent), Some(call)) = (e["agentId"].as_str(), d["toolCallId"].as_str()) {
                self.children.insert(agent.to_string(), call.to_string());
            }
            return;
        }
        if matches!(owner, Owner::Other) {
            return;
        }
        // A subagent's message tells the model it runs on: not always the one its start named (its definition's), the
        // first it was sent to may differ.
        if let (Some(call), "assistant.message" | "assistant.usage", Some(model)) =
            (parent.as_deref(), kind, d["model"].as_str())
        {
            out.push(Item::Subagent {
                call: call.into(),
                model: model.into(),
            });
        }
        match kind {
            // A turn started elsewhere.
            "user.message" | "assistant.turn_start" if own && !c.active => {
                out.push(Item::Start { session: None });
            }
            "assistant.message_delta" | "assistant.reasoning_delta" => {
                let id = d["messageId"].as_str().or(d["reasoningId"].as_str());
                let (Some(id), Some(text)) = (id, d["deltaContent"].as_str()) else {
                    return;
                };
                if text.is_empty() {
                    return;
                }
                let thinking = kind != "assistant.message_delta";
                if thinking {
                    self.streamed.insert(id.to_string());
                }
                out.push(Item::Delta {
                    id: id.into(),
                    text: text.into(),
                    thinking,
                    parent,
                });
            }
            "assistant.message" => {
                let (Some(id), text) = (
                    d["messageId"].as_str(),
                    d["content"].as_str().unwrap_or_default(),
                ) else {
                    return;
                };
                if text.trim().is_empty() && !c.seen.has(id) {
                    return;
                }
                if let (Some(call), false) = (&parent, text.trim().is_empty()) {
                    self.reports.insert(call.clone(), text.to_string());
                }
                out.push(Item::Text {
                    text: text.into(),
                    parent,
                    id: Some(id.into()),
                });
            }
            "assistant.reasoning" => {
                let (Some(id), text) = (
                    d["reasoningId"].as_str(),
                    d["content"].as_str().unwrap_or_default(),
                ) else {
                    return;
                };
                // Given whole without words that grew, it may be the model's opaque blob.
                let opaque = !self.streamed.contains(id) && !text.contains(' ');
                if text.trim().is_empty() || opaque {
                    return;
                }
                out.push(Item::Thinking {
                    text: text.into(),
                    parent,
                    id: Some(id.into()),
                });
            }
            "tool.execution_start" => self.tool_start(d, parent, c, out),
            "tool.execution_complete" => self.tool_end(d, c, out),
            "subagent.completed" | "subagent.failed" => {
                if let Some(call) = d["toolCallId"].as_str() {
                    self.totals.insert(
                        call.to_string(),
                        json!({
                            "totalTokens": d["totalTokens"], "totalToolUseCount": d["totalToolCalls"],
                        }),
                    );
                }
            }
            "permission.requested" => self.permission(d, parent, c, out),
            "user_input.requested" => self.question(d, parent, c, out),
            "elicitation.requested" => self.form(d, parent, c, out),
            "exit_plan_mode.requested" => self.plan(d, parent, c, out),
            "session_limits_exhausted.requested" => self.session_limit(d, parent, c, out),
            "permission.completed"
            | "user_input.completed"
            | "elicitation.completed"
            | "exit_plan_mode.completed"
            | "session_limits_exhausted.completed" => self.answered_elsewhere(kind, d, out),
            "assistant.usage" => {
                let n = |k: &str| d[k].as_u64().unwrap_or(0);
                let cache = n("cacheReadTokens") + n("cacheWriteTokens");
                let nano = words::call_nano(d, &self.prices);
                let u = &mut self.usage;
                u.input += n("inputTokens").saturating_sub(cache);
                u.cache += cache;
                u.output += n("outputTokens");
                u.nano += nano;
                // The month's credits, as every call tells them.
                if let Some(m) = words::credits(&d["quotaSnapshots"], now_ms()) {
                    self.credits_reset = Some(m.resets_at);
                    out.push(Item::Quota {
                        meters: json!([m.meter()]),
                    });
                }
            }
            "session.usage_info" if own => out.push(Item::Context {
                tokens: Some(d["currentTokens"].as_u64().unwrap_or(0)),
                window: d["tokenLimit"].as_u64(),
            }),
            "session.model_change" if own => {
                let model = d["newModel"].as_str().unwrap_or_default();
                if !model.is_empty() && model != "auto" {
                    out.push(Item::Model {
                        model: model.into(),
                    });
                }
            }
            "session.mode_changed" if own => {
                let mode = match d["newMode"].as_str() {
                    Some("plan") => "plan",
                    Some("autopilot") => "auto",
                    // Out of the mode Plan (or of autopilot): « Demander », unless Escouade's mode already says how it
                    // works.
                    _ if matches!(c.mode, "plan" | "auto") => "default",
                    _ => return,
                };
                out.push(Item::Mode { mode: mode.into() });
            }
            // Asked (`/compact`, ESC-205) or by Copilot itself: from its start to its end.
            "session.compaction_start" if own => out.push(Item::Notice {
                text: said(
                    c.lang,
                    "Compaction du contexte…".into(),
                    "Compacting the context…".into(),
                ),
                level: "info".into(),
            }),
            "session.compaction_complete" if own && d["success"] == true => {
                out.push(Item::Notice {
                    text: said(
                        c.lang,
                        "Contexte compacté".into(),
                        "Context compacted".into(),
                    ),
                    level: "info".into(),
                });
            }
            "abort" if own => self.aborted = true,
            "session.error" if own => {
                // The month's credits spent (`quota`), or a weekly limit: the turn waits.
                let limit = match (d["errorType"].as_str(), d["errorCode"].as_str()) {
                    (Some("quota"), code) if code != Some("billing_not_configured") => {
                        Some(self.credits_reset)
                    }
                    (Some("rate_limit"), Some(code)) if code.contains("weekly") => Some(None),
                    _ => None,
                };
                if limit.is_some() {
                    self.limited = limit;
                }
                if self.error.is_none() {
                    let message = d["message"].as_str().unwrap_or_default().trim();
                    if !message.is_empty() {
                        self.error = Some(message.to_string());
                    }
                }
            }
            "session.idle" if own => {
                if d["aborted"] == true {
                    self.aborted = true;
                }
                if c.active {
                    self.turn_end(out);
                }
            }
            _ => {}
        }
    }

    /// A tool starts: its call shown, under Claude Code's name. A question's and a plan's have their card instead, the
    /// task list its plan.
    fn tool_start(&mut self, d: &Value, parent: Option<String>, c: &Ctx, out: &mut Vec<Item>) {
        let Some(call) = d["toolCallId"].as_str() else {
            return;
        };
        let tool = d["toolName"].as_str().unwrap_or_default();
        if matches!(tool, "ask_user" | "exit_plan_mode") || c.seen.has(call) {
            return;
        }
        let (name, input) = tool_of(tool, &d["arguments"], d["mcpServerName"].as_str(), c.cwd);
        out.push(Item::Tool {
            id: call.to_string(),
            name,
            input,
            parent,
            // The table of its tasks, which come by `Todos`.
            unplanned: tool == "sql",
        });
    }

    /// A tool is over: its result, the files it changed, a subagent's report.
    fn tool_end(&mut self, d: &Value, c: &Ctx, out: &mut Vec<Item>) {
        let Some(call) = d["toolCallId"].as_str() else {
            return;
        };
        let Some(item) = c.seen.call(call) else {
            return;
        };
        if item["status"] != "running" {
            return;
        }
        let failed = d["success"] == false
            || d["shellExecution"]["exitCode"]
                .as_i64()
                .is_some_and(|c| c != 0);
        let mut text = if d["success"] == false {
            d["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        } else {
            d["result"]["content"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        };
        let name = item["name"].as_str().unwrap_or_default().to_string();
        let path = item["input"]["file_path"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let edits: Vec<String> = d["fileEdits"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["path"].as_str().map(str::to_string))
            .collect();
        let diff = d["result"]["detailedContent"].as_str().unwrap_or_default();
        let result = match name.as_str() {
            "Edit" | "Write" | "MultiEdit" if !failed => {
                let file = edits.first().cloned().unwrap_or(path);
                if name == "Write" && !diff.contains("@@") {
                    json!({ "type": "create", "filePath": file, "content": item["input"]["content"], "structuredPatch": [] })
                } else {
                    json!({ "filePath": file, "structuredPatch": hunks_of(diff) })
                }
            }
            "Task" => {
                if let Some(report) = self.reports.remove(call) {
                    if text.trim().is_empty() {
                        text = report;
                    }
                }
                self.totals.remove(call).unwrap_or(Value::Null)
            }
            _ => Value::Null,
        };
        // Its own files (the plan of the mode Plan, in its session's folder) are none of the agent's.
        if !failed {
            for f in edits.iter().filter(|f| escouade_acp::inside(f, c.cwd)) {
                out.push(Item::Touched { path: f.clone() });
            }
        }
        out.push(Item::ToolDone {
            id: call.into(),
            output: text,
            error: failed,
            result,
        });
    }

    /// Whether Escouade's rules answer the permission `r` (its `permissionRequest`) without a card: a read in the
    /// agent's folder; a write there, but in the mode Plan and in `.git`; `git add` and `git commit` of its worktree in
    /// « Accepter les modifications » and « Auto », its text read as its shell reads it. Each path as asked and as Copilot
    /// resolved it, links followed. Never what Copilot's policy keeps for a human, or a way out of its sandbox.
    pub fn safe(r: &Value, c: &Ctx) -> bool {
        if r["managedApprovalRequired"] == true || r["requestSandboxBypass"] == true {
            return false;
        }
        let paths = words::paths_of(r);
        let inside = !paths.is_empty() && paths.iter().all(|p| escouade_acp::inside(p, c.cwd));
        // A discussion (ESC-261) writes nothing: its reads alone go through, the rest comes to the core, which refuses a
        // write or a command.
        match r["kind"].as_str().unwrap_or_default() {
            "read" => inside,
            _ if c.discussion => false,
            "write" => {
                c.mode != "plan"
                    && inside
                    && !paths.iter().any(|p| escouade_acp::in_git_folder(p, c.cwd))
            }
            "shell" => {
                let ids: Vec<&str> = r["commands"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|c| c["identifier"].as_str())
                    .collect();
                let in_worktree = r["resolvedWorkingDirectory"].as_str().is_none_or(|d| {
                    escouade_acp::inside(d, c.cwd) && escouade_acp::inside(c.cwd, d)
                });
                matches!(c.mode, "acceptEdits" | "auto")
                    && in_worktree
                    && r["hasWriteFileRedirection"] != true
                    && git::accepts_script(
                        r["fullCommandText"].as_str().unwrap_or_default(),
                        r["shellType"].as_str(),
                        &ids,
                        Path::new(c.cwd),
                        c.fenced,
                    )
            }
            _ => false,
        }
    }

    /// A permission Copilot asks (`permission.requested`), by its kind: a command with the paths it touches, a write
    /// with its diff, a read, an address, a tool of an MCP server.
    fn permission(&mut self, d: &Value, parent: Option<String>, c: &Ctx, out: &mut Vec<Item>) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        if c.seen.has(&rid) {
            return;
        }
        let r = &d["permissionRequest"];
        let call = r["toolCallId"].as_str().unwrap_or(&rid).to_string();
        if c.process && Self::safe(r, c) {
            self.approved.push(rid);
            return;
        }
        let (tool, input, description, hunks) = permission_card(r, c.cwd, c.lang);
        out.push(Item::PermissionCard {
            request: rid,
            call,
            tool,
            input,
            description,
            reason: r["intention"].as_str().map(str::to_string),
            hunks,
            // Fenced, no rule of the session: what it covers is never asked again, so `words::fenced` never sees it.
            can_always: r["canOfferSessionApproval"] != false && !c.fenced,
            always_scope: Some("session".into()),
            parent,
        });
    }

    /// A question of Copilot (`ask_user`, `user_input.requested`): its choices, and a free answer written in the message
    /// field when it takes one.
    fn question(&mut self, d: &Value, parent: Option<String>, c: &Ctx, out: &mut Vec<Item>) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        let call = d["toolCallId"].as_str().unwrap_or(&rid).to_string();
        let options: Vec<Value> = d["choices"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c.as_str())
            .map(|c| json!({ "label": c, "description": "" }))
            .collect();
        let q = json!({
            "question": d["question"], "header": "", "multiSelect": false, "options": options,
        });
        Self::card(rid, call, vec![q], Value::Null, parent, c, out);
    }

    /// A question card, its questions `questions`, made for the request `rid` (of the call `call`).
    fn card(
        rid: String,
        call: String,
        questions: Vec<Value>,
        form: Value,
        parent: Option<String>,
        c: &Ctx,
        out: &mut Vec<Item>,
    ) {
        if c.seen.has(&rid) {
            return;
        }
        out.push(Item::QuestionCard {
            request: rid,
            call,
            questions: Value::Array(questions),
            form,
            parent,
        });
    }

    /// The session's ceiling of AI credits reached (`session_limits_exhausted.requested`, a session given
    /// `--max-ai-credits`): a card whose choices raise it, lift it or give the call up.
    fn session_limit(&mut self, d: &Value, parent: Option<String>, c: &Ctx, out: &mut Vec<Item>) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        let options: Vec<Value> = words::limit_choices(c.lang, d)
            .into_iter()
            .map(|(label, _)| json!({ "label": label, "description": "" }))
            .collect();
        let q = json!({
            "question": words::limit_question(c.lang, d),
            "header": said(c.lang, "Plafond".into(), "Ceiling".into()),
            "multiSelect": false, "options": options,
        });
        Self::card(rid.clone(), rid, vec![q], Value::Null, parent, c, out);
    }

    /// A form of an MCP server (`elicitation.requested`): a card of its fields, each a question (its choices for a list
    /// or yes or no, a field to fill otherwise).
    fn form(&mut self, d: &Value, parent: Option<String>, c: &Ctx, out: &mut Vec<Item>) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        let questions = form_questions(&d["requestedSchema"], c.lang);
        let form = json!({ "message": d["message"], "source": d["elicitationSource"] });
        Self::card(rid.clone(), rid, questions, form, parent, c, out);
    }

    /// The end of the mode Plan (`exit_plan_mode.requested`): the card « Plan à approuver », as for Claude Code; the turn
    /// waits for it.
    fn plan(&mut self, d: &Value, parent: Option<String>, c: &Ctx, out: &mut Vec<Item>) {
        let Some(rid) = d["requestId"].as_str().map(str::to_string) else {
            return;
        };
        if c.seen.has(&rid) {
            return;
        }
        let plan = d["planContent"]
            .as_str()
            .filter(|p| !p.trim().is_empty())
            .or(d["summary"].as_str())
            .unwrap_or_default();
        out.push(Item::PermissionCard {
            request: rid.clone(),
            call: rid,
            tool: "ExitPlanMode".into(),
            input: json!({ "plan": plan }),
            description: None,
            reason: None,
            hunks: Value::Null,
            can_always: true,
            always_scope: None,
            parent,
        });
    }

    /// A request closed without the card (answered the other way, or by Copilot itself): its card says so and goes.
    fn answered_elsewhere(&mut self, kind: &str, d: &Value, out: &mut Vec<Item>) {
        let Some(rid) = d["requestId"].as_str() else {
            return;
        };
        let decision = match kind {
            "permission.completed" => Some(match d["result"]["kind"].as_str() {
                Some("approved-for-session") => "always",
                Some(k) if k.starts_with("denied") => "deny",
                Some(k) if k.starts_with("approved") => "allow",
                _ => "deny",
            }),
            "exit_plan_mode.completed" => Some(if d["approved"] == true {
                "allow"
            } else {
                "deny"
            }),
            _ => None,
        };
        let answers = (kind == "user_input.completed").then(|| json!([[d["answer"]]]));
        out.push(Item::Answered {
            request: rid.into(),
            decision: decision.map(str::to_string),
            answers,
        });
    }

    /// The end of the agent's turn (`session.idle`, or a message refused).
    pub fn turn_end(&mut self, out: &mut Vec<Item>) {
        let interrupted = std::mem::take(&mut self.aborted);
        let error = std::mem::take(&mut self.error).filter(|_| !interrupted);
        let limited = std::mem::take(&mut self.limited).filter(|_| !interrupted);
        let entry = self
            .last_event
            .clone()
            .filter(|_| error.is_none() && limited.is_none());
        if limited.is_some() {
            out.push(Item::LimitRest);
        }
        let u = std::mem::take(&mut self.usage);
        // Its cost: the nano-AIU of its calls (credits = nano-AIU / 10⁹, a credit $0.01, checked at point 0: their sum
        // is the session's `usage_checkpoint`).
        let cost = u.nano as f64 / 1e11;
        self.totals.clear();
        self.reports.clear();
        out.push(Item::Turn {
            interrupted,
            limited,
            usage: TurnUsage {
                input: u.input,
                output: u.output,
                cache: u.cache,
                cost,
            },
            error,
            entry,
        });
    }
}

/// The card of a permission of Copilot (its `permissionRequest`): the tool Claude Code would have asked for, its input,
/// what the card says beside it, and the hunks of a write.
fn permission_card(r: &Value, cwd: &str, lang: &str) -> (String, Value, Option<String>, Value) {
    let list = |k: &str| -> Vec<String> {
        r[k].as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    };
    let outside = || {
        said(
            lang,
            "Hors du dossier de l’agent".into(),
            "Outside the agent’s folder".into(),
        )
    };
    match r["kind"].as_str().unwrap_or_default() {
        "shell" => {
            let command = r["fullCommandText"].as_str().unwrap_or_default();
            let mut told = Vec::new();
            let paths = list("possiblePaths");
            if !paths.is_empty() {
                let paths = paths.join(", ");
                told.push(said(
                    lang,
                    format!("Chemins : {paths}"),
                    format!("Paths: {paths}"),
                ));
            }
            let urls: Vec<&str> = r["possibleUrls"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|u| u["url"].as_str())
                .collect();
            if !urls.is_empty() {
                let urls = urls.join(", ");
                told.push(said(
                    lang,
                    format!("Adresses : {urls}"),
                    format!("Addresses: {urls}"),
                ));
            }
            if r["hasWriteFileRedirection"] == true {
                told.push(said(
                    lang,
                    "Écrit dans un fichier par une redirection".into(),
                    "Writes to a file through a redirection".into(),
                ));
            }
            if git::writes_repository(command) {
                told.push(said(
                    lang,
                    "git écrit dans le dépôt du projet.".into(),
                    "git writes in the project’s repository.".into(),
                ));
            }
            let description = (!told.is_empty()).then(|| told.join("\n"));
            (
                "Bash".into(),
                json!({ "command": command }),
                description,
                Value::Null,
            )
        }
        "write" => {
            let path = r["fileName"].as_str().unwrap_or_default();
            let diff = r["diff"].as_str().unwrap_or_default();
            let created = diff.contains("--- a/dev/null") || diff.contains("new file mode");
            let description = (!escouade_acp::inside(path, cwd)).then(outside);
            (
                if created { "Write" } else { "Edit" }.into(),
                json!({ "file_path": path }),
                description,
                hunks_of(diff),
            )
        }
        "read" => {
            let path = r["path"].as_str().unwrap_or_default();
            let description = (!escouade_acp::inside(path, cwd)).then(outside);
            (
                "Read".into(),
                json!({ "file_path": path }),
                description,
                Value::Null,
            )
        }
        "url" => (
            "WebFetch".into(),
            json!({ "url": r["url"] }),
            None,
            Value::Null,
        ),
        "mcp" => {
            let server = r["serverName"].as_str().unwrap_or_default();
            let tool = mcp_tool(server, r["toolName"].as_str().unwrap_or_default());
            let name = mcp_name(server, &tool);
            let description = if r["readOnly"] == true {
                said(
                    lang,
                    format!("Serveur MCP {server}, outil {tool} : ne fait que lire"),
                    format!("MCP server {server}, tool {tool}: only reads"),
                )
            } else {
                said(
                    lang,
                    format!("Serveur MCP {server}, outil {tool} : peut modifier"),
                    format!("MCP server {server}, tool {tool}: may change things"),
                )
            };
            (name, r["args"].clone(), Some(description), Value::Null)
        }
        other => (other.to_string(), r.clone(), None, Value::Null),
    }
}

/// The name Escouade gives a tool of an MCP server.
fn mcp_name(server: &str, tool: &str) -> String {
    format!("mcp__{server}__{tool}")
}

/// An MCP tool's own name, without its server's (`essai-ping` of `essai`, `github-mcp-server/x`).
fn mcp_tool(server: &str, tool: &str) -> String {
    tool.strip_prefix(&format!("{server}-"))
        .or_else(|| tool.strip_prefix(&format!("{server}/")))
        .unwrap_or(tool)
        .to_string()
}

/// The questions of a form's card, one by field of its `schema`: its required fields first, in their order, then the
/// others. A field of a list (`enum`) or yes or no gives its choices; another is a field to fill (`field`: `text` or
/// `number`).
pub fn form_questions(schema: &Value, lang: &str) -> Vec<Value> {
    let Some(props) = schema["properties"].as_object() else {
        return Vec::new();
    };
    let required: Vec<&str> = schema["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut names: Vec<&String> = props.keys().collect();
    names.sort_by_key(|n| {
        required
            .iter()
            .position(|r| r == n)
            .unwrap_or(required.len())
    });
    names
        .into_iter()
        .map(|name| {
            let p = &props[name];
            let text = p["title"].as_str().unwrap_or(name);
            let mut q = json!({
                "id": name, "question": text,
                "header": p["description"].as_str().unwrap_or_default(),
                "multiSelect": false, "options": [],
                "required": required.contains(&name.as_str()),
            });
            let choices: Vec<Value> = p["enum"]
                .as_array()
                .or(p["items"]["enum"].as_array())
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(|s| json!({ "label": s, "description": "" })))
                .collect();
            match p["type"].as_str() {
                Some("boolean") => {
                    q["options"] = json!([
                        { "label": words::yes(lang), "description": "" },
                        { "label": words::no(lang), "description": "" },
                    ]);
                }
                _ if !choices.is_empty() => {
                    q["options"] = Value::Array(choices);
                    q["multiSelect"] = json!(p["type"] == "array");
                }
                Some("number" | "integer") => q["field"] = json!("number"),
                _ => q["field"] = json!("text"),
            }
            q
        })
        .collect()
}

/// The first file a patch of `apply_patch` changes (`*** Update File: <path>`).
fn patch_file(text: &str) -> Option<&str> {
    text.lines().find_map(|l| {
        ["*** Update File: ", "*** Add File: ", "*** Delete File: "]
            .iter()
            .find_map(|p| l.strip_prefix(p))
            .map(str::trim)
    })
}

/// The tool Claude Code would have called for Copilot's `tool` (`server`: the MCP server it is of), and its input as
/// Claude Code's.
pub fn tool_of(tool: &str, args: &Value, server: Option<&str>, cwd: &str) -> (String, Value) {
    if let Some(server) = server.filter(|s| !s.is_empty()) {
        return (mcp_name(server, &mcp_tool(server, tool)), args.clone());
    }
    let path = |k: &str| absolute(args[k].as_str().unwrap_or_default(), cwd);
    let (name, input) = match tool {
        "powershell" | "bash" => (
            "Bash",
            json!({ "command": args["command"], "description": args["description"] }),
        ),
        "view" => ("Read", json!({ "file_path": path("path") })),
        "glob" => (
            "Glob",
            json!({ "pattern": args["pattern"], "path": args["path"] }),
        ),
        "grep" | "rg" => (
            "Grep",
            json!({ "pattern": args["pattern"], "path": args["path"], "glob": args["glob"] }),
        ),
        "create" => (
            "Write",
            json!({ "file_path": path("path"), "content": args["file_text"] }),
        ),
        "edit" => (
            "Edit",
            json!({
                "file_path": path("path"), "old_string": args["old_str"], "new_string": args["new_str"],
            }),
        ),
        "apply_patch" => {
            let text = args
                .as_str()
                .or(args["input"].as_str())
                .or(args["patch"].as_str())
                .unwrap_or_default();
            let file = patch_file(text).map(|f| absolute(f, cwd));
            let added = text.contains("*** Add File: ");
            (
                if added { "Write" } else { "Edit" },
                json!({ "file_path": file }),
            )
        }
        "web_fetch" => ("WebFetch", json!({ "url": args["url"] })),
        "task" => (
            "Task",
            json!({
                "description": args["name"].as_str().or(args["description"].as_str()),
                "prompt": args["prompt"], "subagent_type": args["agent_type"],
            }),
        ),
        "skill" => (
            "Skill",
            json!({ "skill": args["skill"].as_str().or(args["name"].as_str()) }),
        ),
        other => return (other.to_string(), args.clone()),
    };
    let mut input = input;
    if let Some(o) = input.as_object_mut() {
        o.retain(|_, v| !v.is_null());
    }
    (name.to_string(), input)
}
