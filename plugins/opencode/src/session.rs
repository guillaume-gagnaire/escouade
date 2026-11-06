//! An agent's session on its account's `opencode serve` (`server`).
//!
//! It creates, resumes or copies its session, sends its messages (`prompt_async`), answers its
//! permissions and questions, and passes on the events of its session and of its subagents'
//! (`server::Listener`). A message sent while a turn runs would be taken into it at once, the
//! first one's request lost (`mid-turn.jsonl`): Escouade holds it until the turn ends
//! (`session.idle` of the agent's own session, once it was seen busy: an `idle` of the turn before
//! can come after the next message is sent).

use crate::host::Host;
use crate::rules::{self, agent_of, session_rules, split_model, tr, Settings, SERVER};
use crate::server::{self, Listener, Server};
use crate::translate::{contexts_event, Translator};
use anyhow::{anyhow, bail, Result};
use escouade_plugin_protocol::engine::{
    method, AnswerPermission, Attach, EnvVar, Grant, Item, Launch, Open, Opened, Pushed, StartAt,
};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

/// What the session knows of its conversation.
#[derive(Default)]
struct State {
    /// Its session, once created, resumed or copied.
    session: Option<String>,
    /// A turn of its session runs: one Escouade started, or one seen busy (a message typed in
    /// `opencode attach`).
    busy: bool,
    /// Its session was seen busy since the last message sent: its next `idle` ends the turn.
    saw_busy: bool,
    /// The parts of the messages held while a turn runs: sent as one once it ends.
    held: Vec<Value>,
    settings: Settings,
    /// The variants of each model (`variants`): an effort is passed to a model that has it.
    variants: HashMap<String, Vec<String>>,
    /// Stopped gently while a turn ran: released once it ends.
    closing: bool,
    /// The prompts sent whose message the session has not shown yet.
    unseen_prompts: u32,
    /// Whether each message of the user in the session was Escouade's (true) or typed in another
    /// client (`opencode attach`), by its id: decided once, at its first `message.updated`.
    by_escouade: HashMap<String, bool>,
}

pub(crate) struct Session {
    /// The agent's process, as `agent/*` names it.
    agent: String,
    /// Its server, once it runs.
    server: Mutex<Option<Arc<Server>>>,
    cwd: String,
    lang: String,
    /// The instructions added to it (a ticket's protocol): the `system` of each prompt, which
    /// holds for that prompt alone (tried, docs/TODO-1.8.md, point 0).
    instructions: Mutex<Option<String>>,
    state: Mutex<State>,
    translator: Mutex<Translator>,
    host: Arc<Host>,
    /// Its session is heard (`Server::listen`).
    heard: AtomicBool,
    /// Closed gently: `Item::Closed` said once.
    closed: AtomicBool,
}

/// Hears the server for the session (weakly: a session dropped is not kept alive by it).
struct Hears(Weak<Session>);

impl Listener for Hears {
    fn event(&self, frame: Value) {
        if let Some(s) = self.0.upgrade() {
            s.on_event(frame);
        }
    }
}

impl Session {
    pub(crate) fn new(a: &Attach, host: Arc<Host>) -> Arc<Self> {
        let settings = Settings {
            model: a.model.clone(),
            effort: a.effort.clone(),
            mode: a.mode.clone(),
            fence: a.fence.clone(),
            read_only: a.read_only,
        };
        Arc::new(Session {
            agent: a.agent.clone(),
            server: Mutex::new(None),
            cwd: a.cwd.clone(),
            lang: a.lang.clone(),
            instructions: Mutex::new(instructions(a)),
            state: Mutex::new(State {
                session: a.session.clone(),
                settings,
                ..State::default()
            }),
            translator: Mutex::new(Translator::new(
                &a.cwd,
                &a.model,
                a.session.clone(),
                &a.lang,
            )),
            host,
            heard: AtomicBool::new(false),
            closed: AtomicBool::new(false),
        })
    }

    /// Given again (with what it runs on, or after the plugin started again).
    pub(crate) fn attached(&self, a: &Attach, server: Option<Arc<Server>>) {
        {
            let mut s = self.state.lock();
            s.settings.model = a.model.clone();
            s.settings.effort = a.effort.clone();
            s.settings.mode = a.mode.clone();
            s.settings.fence = a.fence.clone();
            s.settings.read_only = a.read_only;
            if s.session.is_none() {
                s.session = a.session.clone();
            }
        }
        *self.instructions.lock() = instructions(a);
        if let Some(server) = server {
            *self.server.lock() = Some(server);
        }
    }

    fn srv(&self) -> Result<Arc<Server>> {
        self.server
            .lock()
            .clone()
            .ok_or_else(|| anyhow!("OpenCode is not running"))
    }

    fn session(&self) -> Result<String> {
        self.state
            .lock()
            .session
            .clone()
            .ok_or_else(|| anyhow!("OpenCode has no session yet"))
    }

    /// What `frame` says, translated and sent to the core, in its order.
    fn emit(&self, frame: Value) {
        let mut items = Vec::new();
        let mut t = self.translator.lock();
        t.frame(&frame, &mut items);
        self.push(items);
    }

    fn push(&self, items: Vec<Item>) {
        if !items.is_empty() {
            self.host.notify(
                method::ITEMS,
                &Pushed {
                    agent: self.agent.clone(),
                    items,
                },
            );
        }
    }

    /// Lines the core gave: a recorded stream on the bench.
    pub(crate) fn frames(&self, lines: &[String]) -> Vec<Item> {
        let mut items = Vec::new();
        let mut t = self.translator.lock();
        for line in lines {
            if let Ok(frame) = serde_json::from_str::<Value>(line) {
                t.frame(&frame, &mut items);
            }
        }
        items
    }

    /// An event of its session or of a subagent's.
    fn on_event(self: Arc<Self>, mut frame: Value) {
        let kind = frame["type"].as_str().unwrap_or_default().to_string();
        let sid = frame["properties"]["sessionID"]
            .as_str()
            .map(str::to_string);
        let mut next: Option<Vec<Value>> = None;
        let mut close = false;
        {
            let mut s = self.state.lock();
            let own = sid.is_some() && sid == s.session;
            match kind.as_str() {
                "message.updated" if own && frame["properties"]["info"]["role"] == "user" => {
                    let id = frame["properties"]["info"]["id"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    let State {
                        unseen_prompts,
                        by_escouade,
                        ..
                    } = &mut *s;
                    // Escouade's while a prompt it sent is still to be seen: the server keeps the
                    // messages of a session in the order they came.
                    let ours = *by_escouade.entry(id).or_insert_with(|| {
                        let ours = *unseen_prompts > 0;
                        *unseen_prompts = unseen_prompts.saturating_sub(1);
                        ours
                    });
                    if !ours {
                        frame["properties"]["typedElsewhere"] = json!(true);
                    }
                }
                "session.status" if own => {
                    if frame["properties"]["status"]["type"] == "busy" {
                        s.busy = true;
                        s.saw_busy = true;
                    }
                }
                "session.idle" if own => {
                    // An `idle` of the turn before, come after the message: not this turn's end.
                    if !s.saw_busy {
                        return;
                    }
                    s.busy = false;
                    s.saw_busy = false;
                    if !s.held.is_empty() {
                        next = Some(std::mem::take(&mut s.held));
                    } else if s.closing {
                        close = true;
                    }
                }
                _ => {}
            }
        }
        self.emit(frame);
        if let Some(parts) = next {
            let _ = self.prompt(parts);
        } else if close {
            self.say_closed();
        }
    }

    /// Sends `parts` as a prompt of its session, with the settings of its next turn.
    pub(crate) fn prompt(self: &Arc<Self>, parts: Vec<Value>) -> Result<()> {
        let session = self.session()?;
        let body = {
            let mut s = self.state.lock();
            let Some((provider, model)) =
                split_model(&s.settings.model).map(|(p, m)| (p.to_string(), m.to_string()))
            else {
                bail!(rules::no_model(&self.lang));
            };
            s.busy = true;
            s.saw_busy = false;
            s.unseen_prompts += 1;
            let mut body = json!({
                "parts": parts,
                "agent": agent_of(&s.settings.mode),
                "model": { "providerID": provider, "modelID": model },
            });
            let effort = &s.settings.effort;
            if !effort.is_empty()
                && s.variants
                    .get(&s.settings.model)
                    .is_some_and(|v| v.contains(effort))
            {
                body["variant"] = json!(effort);
            }
            if let Some(i) = &*self.instructions.lock() {
                body["system"] = json!(i);
            }
            body
        };
        self.translator.lock().began();
        let me = self.clone();
        self.host.spawn(async move {
            let path = format!("/session/{session}/prompt_async");
            let said = match me.srv() {
                Ok(srv) => srv.call("POST", &path, &me.cwd, Some(body)).await,
                Err(e) => Err(e),
            };
            if let Err(e) = said {
                eprintln!("opencode: {e:#}");
                {
                    let mut s = me.state.lock();
                    s.busy = false;
                    s.unseen_prompts = s.unseen_prompts.saturating_sub(1);
                }
                me.emit(json!({
                    "type": "escouade.prompt.failed",
                    "properties": { "sessionID": session, "message": format!("{e:#}") },
                }));
            }
        });
        Ok(())
    }

    /// A request sent without waiting for its answer (a failure is only logged).
    fn fire(self: &Arc<Self>, method: &'static str, path: String, body: Option<Value>) {
        let me = self.clone();
        self.host.spawn(async move {
            let said = match me.srv() {
                Ok(srv) => srv.call(method, &path, &me.cwd, body).await,
                Err(e) => Err(e),
            };
            if let Err(e) = said {
                eprintln!("opencode: {e:#}");
            }
        });
    }

    /// The rules of its mode set on its session (OpenCode merges them, the last one of a
    /// permission winning: each mode gives all of them).
    fn set_rules(self: &Arc<Self>) {
        let Ok(session) = self.session() else { return };
        let rules = {
            let s = self.state.lock();
            session_rules(&s.settings)
        };
        self.fire(
            "PATCH",
            format!("/session/{session}"),
            Some(json!({ "permission": rules })),
        );
    }

    fn say_closed(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            self.forget();
            self.push(vec![Item::Closed]);
        }
    }

    /// Its session is no longer heard.
    pub(crate) fn forget(&self) {
        if let (Some(session), Ok(srv)) = (self.state.lock().session.clone(), self.srv()) {
            srv.forget(&session);
        }
    }

    /// Stopped gently: once its turn is over; at once (`now`): its turn stopped.
    pub(crate) fn close(self: &Arc<Self>, now: bool) {
        let (busy, session) = {
            let mut s = self.state.lock();
            s.closing = true;
            (s.busy, s.session.clone())
        };
        if now {
            if let (true, Some(session)) = (busy, session) {
                self.fire("POST", format!("/session/{session}/abort"), None);
            }
            self.closed.store(true, Ordering::Release);
            self.forget();
        } else if !busy {
            self.say_closed();
        }
    }

    /// The last agent of its folder gone, the server lets it go: its instance watches the folder, which a worktree
    /// removed under Windows would find held.
    pub(crate) async fn let_go(&self) -> Result<()> {
        self.srv()?
            .call("POST", "/instance/dispose", &self.cwd, None)
            .await
            .map(|_| ())
    }

    /// Escouade's MCP server given to its folder's instance, as the agent itself (its own token,
    /// which the server identifies it by), or kept out of it: the server is shared, its config
    /// (`mcp.escouade.enabled: false`) holds for every folder, and an entry added by the API holds
    /// for one folder alone (tried on 1.19.3, docs/PROTOCOL.md). An agent has its folder to itself on
    /// its server (`SharedLaunch::one_agent_a_folder`), and the entry goes with the instance when the agent leaves.
    /// A failure is logged: the agent works on without Escouade's tools.
    async fn give_escouade(&self, srv: &Server, grant: Option<&Grant>) {
        match grant {
            Some(Grant { url, token }) => {
                let body = rules::entry(url, token);
                match srv.call("POST", "/mcp", &self.cwd, Some(body)).await {
                    Ok(said) if said[SERVER]["status"] == "connected" => {}
                    Ok(said) => eprintln!(
                        "opencode: Escouade's server not connected in {}: {}",
                        self.cwd, said[SERVER]
                    ),
                    Err(e) => eprintln!("opencode: Escouade's server: {e:#}"),
                }
            }
            None => {
                // Its config keeps it out; one an instance would still have, disconnected.
                let status = srv.call("GET", "/mcp", &self.cwd, None).await;
                let on = status
                    .as_ref()
                    .is_ok_and(|s| s.get(SERVER).is_some_and(|e| e["status"] != "disabled"));
                if on {
                    let path = format!("/mcp/{SERVER}/disconnect");
                    if let Err(e) = srv.call("POST", &path, &self.cwd, None).await {
                        eprintln!("opencode: Escouade's server kept: {e:#}");
                    }
                }
            }
        }
    }

    /// Its session: a new one, the one it resumes, or a copy of its original's.
    async fn start(&self, srv: &Server, start: &StartAt) -> Result<(Value, &'static str)> {
        let rules = {
            let s = self.state.lock();
            session_rules(&s.settings)
        };
        let new = || json!({ "title": "Escouade", "permission": rules.clone() });
        match start {
            StartAt::Resume { session } => {
                match srv
                    .call("GET", &format!("/session/{session}"), &self.cwd, None)
                    .await
                {
                    Ok(info) => return Ok((info, "resume")),
                    Err(e) if server::not_found(&e) => {
                        eprintln!("opencode: session {session} gone, a new one");
                    }
                    Err(e) => return Err(e),
                }
            }
            StartAt::Fork { of, at } => {
                // OpenCode copies the messages before the one it is given: the one after `at`,
                // none when `at` is the last (all of them).
                let messages = srv
                    .call("GET", &format!("/session/{of}/message"), &self.cwd, None)
                    .await;
                match messages {
                    Ok(list) => {
                        let ids: Vec<&str> = list
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|m| m["info"]["id"].as_str())
                            .collect();
                        let cut = at
                            .as_deref()
                            .and_then(|at| ids.iter().position(|id| *id == at))
                            .and_then(|i| ids.get(i + 1));
                        let body = match cut {
                            Some(id) => json!({ "messageID": id }),
                            None => json!({}),
                        };
                        let info = srv
                            .call(
                                "POST",
                                &format!("/session/{of}/fork"),
                                &self.cwd,
                                Some(body),
                            )
                            .await?;
                        return Ok((info, "fork"));
                    }
                    Err(e) if server::not_found(&e) => {
                        eprintln!("opencode: session {of} to copy gone, a new one");
                    }
                    Err(e) => return Err(e),
                }
            }
            StartAt::New => {}
        }
        let info = srv.call("POST", "/session", &self.cwd, Some(new())).await?;
        Ok((info, "new"))
    }

    /// The models of the server, read once.
    async fn read_catalog(&self, srv: &Arc<Server>) -> Result<rules::Catalog> {
        let v = match srv.catalog().await {
            Ok(known) => known,
            Err(reading) => {
                let providers = srv
                    .call("GET", "/config/providers", &self.cwd, None)
                    .await?;
                let config = srv
                    .call("GET", "/config", &self.cwd, None)
                    .await
                    .unwrap_or(Value::Null);
                let v = json!({ "providers": providers, "config": { "model": config["model"] } });
                reading.keep(v.clone());
                v
            }
        };
        Ok(rules::catalog(&v["providers"], &v["config"]))
    }

    /// The server ready, the session opened and heard, then the models of the connected providers.
    pub(crate) async fn open(self: &Arc<Self>, open: &Open) -> Result<Opened> {
        let srv = self.srv()?;
        srv.ready(&self.lang).await?;
        let (info, how) = self.start(&srv, &open.start).await?;
        let id = info["id"]
            .as_str()
            .ok_or_else(|| anyhow!("OpenCode gave no session"))?
            .to_string();
        self.state.lock().session = Some(id.clone());
        srv.listen(&id, Arc::new(Hears(Arc::downgrade(self))));
        self.heard.store(true, Ordering::Release);
        self.give_escouade(&srv, open.escouade.as_ref()).await;
        // A copy or a resumed session takes the rules of its mode (a copy has none).
        if how != "new" {
            self.set_rules();
        }
        self.emit(json!({
            "type": "escouade.session",
            "properties": { "sessionID": id, "info": info, "start": how },
        }));
        let catalog = self.read_catalog(&srv).await;
        let models = match catalog {
            Ok(c) => {
                // The window of each model: the agent's context is told out of its model's.
                self.emit(contexts_event(&id, &c.contexts));
                let chosen = self.known_models(&c);
                if let Some(model) = chosen {
                    self.emit(json!({
                        "type": "escouade.model",
                        "properties": { "sessionID": id, "model": model },
                    }));
                }
                Some(c.models).filter(|m| !m.is_empty())
            }
            Err(e) => {
                eprintln!("opencode: models: {e:#}");
                None
            }
        };
        Ok(Opened {
            attach: Some(self.attach_of(&srv, &id)),
            catalog: models.and_then(|m| serde_json::to_value(m).ok()),
        })
    }

    /// The variants of each model kept, and the model the agent takes when OpenCode does not offer its own (or none):
    /// its default one, never one of its own choosing (OpenCode Zen without an account, tried).
    fn known_models(&self, c: &rules::Catalog) -> Option<String> {
        let mut s = self.state.lock();
        s.variants = c
            .models
            .iter()
            .map(|m| (m.id.clone(), m.efforts.clone()))
            .collect();
        if c.models.iter().any(|m| m.id == s.settings.model) {
            return None;
        }
        let m = c.models.iter().find(|m| m.default).or(c.models.first())?;
        s.settings.model = m.id.clone();
        Some(m.id.clone())
    }

    /// `opencode attach` on its server, on its session, in its folder; the password in its environment only.
    fn attach_of(&self, srv: &Server, session: &str) -> Launch {
        attach_to(srv.base(), session, &self.cwd, srv.password())
    }

    /// The plugin started again: its session heard again, and a turn that ended meanwhile ended.
    pub(crate) async fn listen_again(self: &Arc<Self>, running: bool) {
        let (Some(session), Ok(srv)) = (self.state.lock().session.clone(), self.srv()) else {
            return;
        };
        // At once: a stop that comes while the server is asked would find no turn, and be lost.
        if running {
            self.state.lock().busy = true;
            self.state.lock().saw_busy = true;
            self.translator.lock().in_turn();
        }
        if !self.heard.swap(true, Ordering::AcqRel) {
            if srv.ready(&self.lang).await.is_err() {
                return;
            }
            srv.listen(&session, Arc::new(Hears(Arc::downgrade(self))));
            if let Ok(c) = self.read_catalog(&srv).await {
                self.emit(contexts_event(&session, &c.contexts));
                self.known_models(&c);
            }
        }
        if !running {
            return;
        }
        let status = srv.call("GET", "/session/status", &self.cwd, None).await;
        // Its turn was over before it was heard again: only this says so.
        if status.is_ok_and(|s| s[&session]["type"].as_str().is_none_or(|t| t == "idle")) {
            self.clone().on_event(json!({
                "type": "session.idle",
                "properties": { "sessionID": session },
            }));
        }
    }

    pub(crate) fn send(self: &Arc<Self>, parts: Vec<Value>) -> Result<()> {
        {
            let mut s = self.state.lock();
            if s.session.is_none() {
                bail!("OpenCode has no session yet");
            }
            if s.busy {
                s.held.extend(parts);
                return Ok(());
            }
        }
        self.prompt(parts)
    }

    pub(crate) async fn interrupt(&self, cancel_queued: bool) -> Result<()> {
        let session = {
            let mut s = self.state.lock();
            if cancel_queued {
                s.held.clear();
            }
            if !s.busy {
                return Ok(());
            }
            s.session.clone()
        };
        let Some(session) = session else {
            return Ok(());
        };
        self.translator.lock().interrupted();
        let path = format!("/session/{session}/abort");
        self.srv()?
            .call("POST", &path, &self.cwd, None)
            .await
            .map(|_| ())
    }

    /// OpenCode takes the model, the effort (`variant`) and the agent of the mode at each prompt.
    pub(crate) fn set_model(&self, model: &str) {
        self.state.lock().settings.model = model.to_string();
        self.translator.lock().set_model(model);
    }

    pub(crate) fn set_effort(&self, effort: &str) {
        self.state.lock().settings.effort = effort.to_string();
    }

    /// Its agent at the next prompt; its rules at once.
    pub(crate) fn set_mode(self: &Arc<Self>, mode: &str) {
        self.state.lock().settings.mode = mode.to_string();
        self.set_rules();
    }

    pub(crate) fn answer_permission(self: &Arc<Self>, a: &AnswerPermission) -> Result<()> {
        // The plan of the mode Plan, its turn over on it: approved, its work starts in the mode
        // left for it; sent back, the planning goes on with what the user said.
        if rules::is_plan_approval(&a.request) {
            let (text, mode) = if !a.allow {
                (a.message.clone().unwrap_or_default(), None)
            } else if a.always {
                (implement_plan(&self.lang), Some("acceptEdits".to_string()))
            } else {
                (
                    implement_plan(&self.lang),
                    Some(a.resume.clone().unwrap_or_else(|| "default".into())),
                )
            };
            if let Some(mode) = mode {
                self.state.lock().settings.mode = mode.clone();
                self.set_rules();
                let session = self.session().unwrap_or_default();
                self.emit(json!({
                    "type": "escouade.mode",
                    "properties": { "sessionID": session, "mode": mode },
                }));
            }
            return self.prompt(vec![json!({ "type": "text", "text": text })]);
        }
        let body = if !a.allow {
            // Told the message, the agent goes on with it.
            json!({ "reply": "reject", "message": a.message })
        } else if a.always {
            // OpenCode keeps the pattern it proposed (`always`) for the rest of the session.
            json!({ "reply": "always" })
        } else {
            json!({ "reply": "once" })
        };
        self.fire(
            "POST",
            format!("/permission/{}/reply", a.request),
            Some(body),
        );
        Ok(())
    }

    pub(crate) fn answer_question(
        self: &Arc<Self>,
        request: &str,
        questions: &Value,
        given: &Value,
    ) {
        self.fire(
            "POST",
            format!("/question/{request}/reply"),
            Some(json!({ "answers": rules::answers(questions, given) })),
        );
    }

    /// `POST /session/:id/summarize` (ESC-205), as its own `/compact` does: with the model of the
    /// next turn, no instruction. It answers once the compaction is over, which may take longer
    /// than a request here: not waited for. The compaction is a turn of its session (`busy`, its
    /// `compaction` part, `session.compacted`, `idle`); its refusal ends it in error.
    pub(crate) fn compact(self: &Arc<Self>) -> Result<()> {
        let session = self.session()?;
        let body = {
            let mut s = self.state.lock();
            let Some((provider, model)) = split_model(&s.settings.model) else {
                bail!(rules::no_model(&self.lang));
            };
            let body = json!({ "providerID": provider, "modelID": model });
            s.busy = true;
            s.saw_busy = false;
            body
        };
        self.translator.lock().began();
        let me = self.clone();
        self.host.spawn(async move {
            let path = format!("/session/{session}/summarize");
            let said = match me.srv() {
                Ok(srv) => srv.call("POST", &path, &me.cwd, Some(body)).await,
                Err(e) => Err(e),
            };
            if let Err(e) = said {
                // A compaction that lasts past the request's limit still runs: told by its
                // events, and its end by `idle`.
                if server::late(&e) {
                    return;
                }
                eprintln!("opencode: {e:#}");
                me.state.lock().busy = false;
                me.emit(json!({
                    "type": "escouade.prompt.failed",
                    "properties": { "sessionID": session, "message": format!("{e:#}") },
                }));
            }
        });
        Ok(())
    }

    pub(crate) async fn rename(&self, title: &str) -> Result<()> {
        let session = self.session()?;
        self.srv()?
            .call(
                "PATCH",
                &format!("/session/{session}"),
                &self.cwd,
                Some(json!({ "title": title })),
            )
            .await
            .map(|_| ())
    }

    /// A request to the server: `{"method", "path", "body"}`, in the agent's folder.
    pub(crate) async fn control(&self, request: Value, within: Duration) -> Result<Value> {
        let method = request["method"].as_str().unwrap_or("GET");
        let path = request["path"].as_str().unwrap_or("/").to_string();
        let body = request.get("body").filter(|b| !b.is_null()).cloned();
        let srv = self.srv()?;
        self.host
            .timeout(within, srv.call(method, &path, &self.cwd, body))
            .await
            .ok_or_else(|| anyhow!("{path}: no answer in time"))?
    }
}

/// The instructions of an agent, when it has any.
fn instructions(a: &Attach) -> Option<String> {
    a.append_prompt.clone().filter(|p| !p.trim().is_empty())
}

/// What the agent is told when its plan is approved.
fn implement_plan(lang: &str) -> String {
    tr(
        lang,
        "Mets le plan en œuvre.".into(),
        "Implement the plan.".into(),
    )
}

/// `opencode attach` on the server at `url` (its password `password`), on `session`, in `dir`
/// (docs/TODO-1.8.md, point 0, « Vue terminal »).
pub(crate) fn attach_to(url: &str, session: &str, dir: &str, password: &str) -> Launch {
    Launch {
        args: ["attach", url, "--session", session, "--dir", dir]
            .map(str::to_string)
            .to_vec(),
        env: vec![EnvVar {
            name: "OPENCODE_SERVER_PASSWORD".into(),
            value: password.into(),
        }],
        ..Launch::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_attach_joins_the_session_with_its_password_in_the_environment_only() {
        let a = attach_to("http://127.0.0.1:4096", "ses_1", r"C:\wt", "secret");
        assert_eq!(
            a.args,
            [
                "attach",
                "http://127.0.0.1:4096",
                "--session",
                "ses_1",
                "--dir",
                r"C:\wt"
            ]
        );
        assert!(!a.args.iter().any(|x| x.contains("secret")));
        assert_eq!(a.env[0].name, "OPENCODE_SERVER_PASSWORD");
        assert_eq!(a.env[0].value, "secret");
    }
}
