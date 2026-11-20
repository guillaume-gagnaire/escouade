//! What the core does with the Overseer (ESC-462, `overseer` for what needs no core): its agent, made in the system
//! folder of Escouade (`<data>/overseer`, its memory in `memory/`) on the harness, account, model and effort of
//! Réglages › Overseer; the user's messages it is sent, each kept for the provenance of an instruction; the events of
//! the facts' bus that wake it, gathered into one message (`overseer_loop`); the guard of the actions that go out; its
//! journal of actions, with what can be undone; and what the window reads of it (`OverseerView`).

use crate::agent::{AgentHandle, AgentRt};
use crate::core::Core;
use crate::facts::{Body, Fact};
use crate::i18n;
use crate::mcp::tools::ToolError;
use crate::mcp::Caller;
use crate::model::{new_id, now_ms, AgentMeta, UiEvent};
use crate::overseer::{
    self, check_source, covering, Action, Batch, Directive, Event, News, NewsKind, Outcome,
    Outgoing, Place, Scope, Source, State, Undo, UndoWhat, UserMessage,
};
use crate::paths;
use anyhow::{anyhow, bail, Context, Result};
use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Weak};

/// What the core holds of the Overseer.
pub struct Hub {
    pub(crate) state: Mutex<State>,
    batch: Mutex<Batch>,
    wake: tokio::sync::Notify,
    /// The time events are gathered (`overseer::GROUP_MS`; the tests make it short).
    pub(crate) group_ms: AtomicU64,
    /// How long a message to an agent is held (`overseer::HOLD_MESSAGE_MS`; the tests make it short).
    pub(crate) hold_ms: AtomicI64,
    /// How many wake-up messages were sent (the tests wait for them).
    pub(crate) wakes: AtomicU64,
    /// Its global shortcut (ESC-468), registered while it is on.
    pub(crate) hotkey: crate::hotkey::Hotkey,
    dir: PathBuf,
}

impl Hub {
    /// Its state as the last run left it, in `<data>/overseer/state.json`.
    pub fn load(data: &crate::paths::DataDir) -> Self {
        let dir = data.root().join("overseer");
        let state: State = std::fs::read(dir.join("state.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let group = std::env::var("ESCOUADE_OVERSEER_GROUP_MS")
            .ok()
            .filter(|_| cfg!(debug_assertions))
            .and_then(|v| v.parse().ok())
            .unwrap_or(overseer::GROUP_MS);
        let hold = std::env::var("ESCOUADE_OVERSEER_HOLD_MS")
            .ok()
            .filter(|_| cfg!(debug_assertions))
            .and_then(|v| v.parse().ok())
            .unwrap_or(overseer::HOLD_MESSAGE_MS);
        Self {
            state: Mutex::new(state),
            batch: Mutex::new(Batch::default()),
            wake: tokio::sync::Notify::new(),
            group_ms: AtomicU64::new(group),
            hold_ms: AtomicI64::new(hold),
            wakes: AtomicU64::new(0),
            hotkey: crate::hotkey::Hotkey::default(),
            dir,
        }
    }

    /// Its folder, in the system folder of Escouade: where its agent works.
    pub fn dir(&self) -> PathBuf {
        self.dir.clone()
    }

    /// Its own memory, which covers every project.
    pub fn memory_dir(&self) -> PathBuf {
        self.dir.join("memory")
    }

    fn save(&self) {
        let bytes = match serde_json::to_vec_pretty(&*self.state.lock()) {
            Ok(b) => b,
            Err(e) => return log::error!("overseer: state not serialized: {e}"),
        };
        let written = std::fs::create_dir_all(&self.dir)
            .and_then(|()| paths::write_atomic(&self.dir.join("state.json"), &bytes));
        if let Err(e) = written {
            log::error!("overseer: state not saved: {e:#}");
        }
    }
}

/// An instruction as the window shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectiveView {
    #[serde(flatten)]
    pub directive: Directive,
    /// Its project's name, its ticket's key, when its scope names them.
    pub project: Option<String>,
    pub ticket: Option<String>,
    /// The message it was born of, whole.
    pub message: Option<String>,
}

/// An action as the window shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionView {
    #[serde(flatten)]
    pub action: Action,
    pub undoable: bool,
}

/// What the window reads of the Overseer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverseerView {
    /// Turned on in the settings.
    pub enabled: bool,
    /// It runs: turned on, with Escouade's MCP server on.
    pub running: bool,
    /// « Claude et les agents peuvent piloter Escouade » is off: it has no tool.
    pub needs_mcp: bool,
    pub agent_id: Option<String>,
    /// The active instructions, newest first.
    pub directives: Vec<DirectiveView>,
    /// Its actions, oldest first.
    pub actions: Vec<ActionView>,
    /// The user's messages, oldest first.
    pub messages: Vec<UserMessage>,
    /// The news of the tickets, oldest first (ESC-468): the cards of its discussion.
    pub news: Vec<News>,
    /// Its global shortcut: registered, or why not (ESC-468).
    pub shortcut: crate::hotkey::HotkeyState,
}

/// The facts' sink that wakes it.
pub struct Sink(pub Weak<Core>);

impl crate::facts::Sink for Sink {
    fn take(&self, fact: &Arc<Fact>) {
        if let Some(core) = self.0.upgrade() {
            core.overseer_fact(fact);
        }
    }
}

impl Core {
    /// It is to run: turned on, Escouade's tools given to the agents, and its feature on (F-1).
    pub fn overseer_wanted(&self) -> bool {
        let s = self.settings.read();
        s.overseer.enabled && s.mcp_enabled && s.feature(crate::features::Feature::Overseer)
    }

    /// Its agent, when made.
    pub fn overseer_agent(&self) -> Option<AgentHandle> {
        let id = self.overseer.state.lock().agent_id.clone()?;
        self.agent(&id).ok()
    }

    /// Whether the agent `id` is the Overseer.
    pub fn is_overseer(&self, id: &str) -> bool {
        self.agent(id)
            .is_ok_and(|h| h.lock().meta.project_id == overseer::PROJECT)
    }

    /// Whether `caller` is the Overseer.
    pub fn overseer_calls(&self, caller: &Caller) -> bool {
        matches!(caller, Caller::Agent(id) if self.is_overseer(id))
    }

    /// What the window reads of it.
    pub fn overseer_view(&self) -> OverseerView {
        let now = now_ms();
        let (enabled, mcp) = {
            let s = self.settings.read();
            (s.overseer.enabled, s.mcp_enabled)
        };
        let st = self.overseer.state.lock().clone();
        let projects = self.projects.read().clone();
        let tickets = self.tickets.read().clone();
        let mut directives: Vec<DirectiveView> = st
            .directives
            .iter()
            .filter(|d| d.active(now))
            .map(|d| DirectiveView {
                project: d
                    .scope
                    .project_id
                    .as_ref()
                    .and_then(|id| projects.iter().find(|p| p.id == *id))
                    .map(|p| p.name.clone()),
                ticket: d
                    .scope
                    .ticket_id
                    .as_ref()
                    .and_then(|id| tickets.iter().find(|t| t.id == *id))
                    .map(|t| t.key.clone()),
                message: st
                    .messages
                    .iter()
                    .find(|m| m.id == d.source.message)
                    .map(|m| m.text.clone()),
                directive: d.clone(),
            })
            .collect();
        directives.reverse();
        OverseerView {
            enabled,
            running: enabled && mcp,
            needs_mcp: enabled && !mcp,
            agent_id: st.agent_id.clone(),
            directives,
            actions: st
                .actions
                .iter()
                .map(|a| ActionView {
                    undoable: a.undoable(now),
                    action: a.clone(),
                })
                .collect(),
            messages: st.messages.clone(),
            news: st.news.clone(),
            shortcut: self.overseer.hotkey.state(),
        }
    }

    /// The window is told, and the state saved.
    pub(crate) fn overseer_changed(&self) {
        self.overseer.save();
        self.hub.emit(UiEvent::Overseer {
            view: Box::new(self.overseer_view()),
        });
    }

    /// The settings were read or saved: its agent made when it is to run, its harness, account, model and effort those
    /// of the settings (taken while it is at rest); not to run, its process stops (its session is kept).
    pub fn overseer_sync(self: &Arc<Self>) {
        self.overseer_hotkey();
        if !self.overseer_wanted() {
            if let Some(h) = self.overseer_agent() {
                let stopped = {
                    let mut rt = h.lock();
                    if rt.meta.status.is_active() {
                        None
                    } else {
                        rt.detach()
                    }
                };
                if let Some(p) = stopped {
                    p.close_input();
                    self.emit_agent(&h);
                }
            }
            self.overseer_changed();
            return;
        }
        if let Err(e) = self.overseer_ensure() {
            log::warn!("overseer: not made: {e:#}");
        }
        self.overseer_changed();
    }

    /// Its global shortcut in step with the settings: the one they keep while it is turned on, none else. Refused (taken
    /// by another app), the reason is kept for Réglages › Overseer.
    pub(crate) fn overseer_hotkey(&self) {
        let (on, spec) = {
            let s = self.settings.read();
            (
                s.overseer.enabled && s.feature(crate::features::Feature::Overseer),
                s.overseer.shortcut.clone(),
            )
        };
        if let Err(e) = self.overseer.hotkey.apply(on.then_some(spec.as_str())) {
            log::warn!("overseer: shortcut {spec} not registered: {}", e.text());
        }
    }

    /// « Raccourci global » of Réglages › Overseer: `spec` registered first (while it is on), then kept in the settings;
    /// one another app holds is refused, and the one before stays.
    pub fn overseer_shortcut_set(self: &Arc<Self>, spec: &str) -> Result<()> {
        let spec = spec.trim();
        if !spec.is_empty() {
            crate::hotkey::accelerator(spec).map_err(|e| anyhow!(e.text()))?;
        }
        if self.settings.read().overseer.enabled {
            let r = self
                .overseer
                .hotkey
                .apply((!spec.is_empty()).then_some(spec));
            if let Err(e) = r {
                self.overseer_changed();
                bail!(e.text());
            }
        }
        let mut settings = self.settings.read().clone();
        settings.overseer.shortcut = spec.to_string();
        self.save_settings(settings)?;
        self.overseer_changed();
        Ok(())
    }

    /// Its agent: made if it is not, in step with the settings otherwise.
    pub(crate) fn overseer_ensure(self: &Arc<Self>) -> Result<AgentHandle> {
        let wanted = self.settings.read().overseer.clone();
        let dir = self.overseer.dir();
        std::fs::create_dir_all(self.overseer.memory_dir())
            .with_context(|| format!("{} not made", dir.display()))?;
        let lang = i18n::claude();
        let prompt = Some(overseer::instructions(lang));
        if let Some(h) = self.overseer_agent() {
            let changed = {
                let mut rt = h.lock();
                let m = &mut rt.meta;
                let account = (!wanted.account.is_empty() && wanted.account != m.account)
                    .then(|| wanted.account.clone());
                let differs = m.engine != wanted.engine
                    || account.is_some()
                    || (!wanted.model.is_empty() && m.model != wanted.model)
                    || m.effort != wanted.effort
                    || m.append_prompt != prompt
                    || m.archived;
                if differs && !m.status.is_active() {
                    if m.engine != wanted.engine {
                        // Another harness: another session.
                        m.session_id = None;
                        m.last_entry = None;
                        m.engine = wanted.engine;
                        m.account = String::new();
                    }
                    if let Some(a) = account {
                        m.account = a;
                    }
                    if !wanted.model.is_empty() {
                        m.model = wanted.model.clone();
                    }
                    m.effort = wanted.effort.clone();
                    m.append_prompt = prompt.clone();
                    m.archived = false;
                    m.cwd = dir.to_string_lossy().into_owned();
                    // What it was started with no longer holds: the next message starts it again.
                    if let Some(p) = rt.detach() {
                        p.close_input();
                    }
                    true
                } else {
                    false
                }
            };
            if changed {
                if h.lock().meta.account.is_empty() {
                    let account = self.account_for_new(overseer::PROJECT, wanted.engine)?;
                    h.lock().meta.account = account;
                }
                self.emit_agent(&h);
                self.request_save();
            }
            return Ok(h);
        }
        if !crate::engine::written(wanted.engine) {
            bail!(crate::engine::absent_line(wanted.engine, i18n::ui()));
        }
        let account = match crate::accounts::find(&self.settings.read(), &wanted.account) {
            Some(a) if a.engine == wanted.engine => a.id.clone(),
            _ => self.account_for_new(overseer::PROJECT, wanted.engine)?,
        };
        let model = if wanted.model.is_empty() {
            crate::accounts::defaults_with(
                crate::accounts::find(&self.settings.read(), &account),
                wanted.engine,
                "",
            )
            .model
        } else {
            wanted.model.clone()
        };
        let meta = AgentMeta {
            id: new_id(),
            project_id: overseer::PROJECT.into(),
            name: overseer::NAME.into(),
            named: true,
            engine: wanted.engine,
            model: self.model_for(wanted.engine, model),
            effort: wanted.effort.clone(),
            // Its tools are Escouade's; what it would write or run is refused (`overseer_answers`).
            mode: "default".into(),
            account,
            cwd: dir.to_string_lossy().into_owned(),
            created_at: now_ms(),
            last_activity: now_ms(),
            append_prompt: prompt,
            ..Default::default()
        };
        let id = meta.id.clone();
        let h: AgentHandle = Arc::new(parking_lot::Mutex::new(AgentRt::new(
            meta,
            &self.data.conversations(),
        )));
        self.agents.write().insert(id.clone(), h.clone());
        self.overseer.state.lock().agent_id = Some(id);
        self.overseer.save();
        self.emit_agent(&h);
        self.request_save();
        Ok(h)
    }

    /// A message of the user to the Overseer (its discussion): kept, with its id, then sent marked as the user's.
    pub async fn overseer_send(self: &Arc<Self>, text: &str, said: bool) -> Result<String> {
        self.overseer_message(text, said, false).await
    }

    /// A sentence said in the voice conversation of its screen (ESC-468): a message « dit », its answer ending with a
    /// passage to be read aloud, as a conversation with an agent asks (`send_spoken`).
    pub async fn overseer_said(self: &Arc<Self>, text: &str) -> Result<String> {
        self.overseer_message(text, true, true).await
    }

    async fn overseer_message(
        self: &Arc<Self>,
        text: &str,
        said: bool,
        spoken: bool,
    ) -> Result<String> {
        let text = text.trim();
        if text.is_empty() {
            bail!(tr!("Écris un message.", "Write a message."));
        }
        if !self.overseer_wanted() {
            bail!(tr!(
                "L’Overseer est éteint : allume-le dans Réglages › Overseer.",
                "The Overseer is off: turn it on in Settings › Overseer."
            ));
        }
        let h = self.overseer_ensure()?;
        let id = h.lock().meta.id.clone();
        let m = {
            let mut st = self.overseer.state.lock();
            let m = UserMessage {
                id: st.next_id("m"),
                text: text.to_string(),
                said,
                at: now_ms(),
            };
            st.keep_message(m.clone());
            m
        };
        self.overseer_changed();
        let body = self.overseer_renewed(&h, overseer::user_text(i18n::claude(), &m));
        if spoken {
            self.send_spoken(&id, body).await?;
        } else {
            self.send_message_from(&id, None, body, Vec::new()).await?;
        }
        Ok(m.id)
    }

    /// `text`, after the thread of the session that ends when this one has been long (`RENEW_AFTER` messages, or its
    /// context nearly full): its process stops, its next one starts a new session, told the thread.
    fn overseer_renewed(&self, h: &AgentHandle, text: String) -> String {
        let renew = {
            let mut st = self.overseer.state.lock();
            st.session_messages += 1;
            let rt = h.lock();
            let full = rt.context_window > 0 && rt.context_tokens * 10 >= rt.context_window * 7;
            if (st.session_messages > overseer::RENEW_AFTER || full) && rt.meta.session_id.is_some()
            {
                st.session_messages = 1;
                true
            } else {
                false
            }
        };
        if !renew {
            return text;
        }
        {
            let mut rt = h.lock();
            rt.meta.session_id = None;
            rt.meta.last_entry = None;
            if let Some(p) = rt.detach() {
                p.close_input();
            }
        }
        self.emit_agent(h);
        self.request_save();
        let lang = i18n::claude();
        let now = now_ms();
        let st = self.overseer.state.lock().clone();
        let directives: Vec<String> = st
            .active(now)
            .iter()
            .map(|d| format!("{} ({}, « {} »)", d.summary, d.id, d.source.quote))
            .collect();
        let mut recent: Vec<(i64, String)> = st
            .messages
            .iter()
            .rev()
            .take(10)
            .map(|m| (m.at, format!("{} : {}", m.id, m.text)))
            .collect();
        recent.extend(
            st.actions
                .iter()
                .rev()
                .take(10)
                .map(|a| (a.at, format!("{} {} ({:?})", a.tool, a.text, a.outcome))),
        );
        recent.sort_by_key(|(at, _)| *at);
        let recent: Vec<String> = recent.into_iter().map(|(_, t)| t).collect();
        format!(
            "{}\n\n{text}",
            overseer::renewal_text(lang, &directives, &recent)
        )
    }

    // ---------- the events ----------

    /// A fact of the bus: an event for it, unless it is of a private project or of itself.
    pub(crate) fn overseer_fact(&self, fact: &Fact) {
        if !self.overseer_wanted() {
            return;
        }
        let agent_id = fact
            .agent
            .as_ref()
            .map(|a| a.id.clone())
            .unwrap_or_default();
        if !agent_id.is_empty() && self.is_overseer(&agent_id) {
            // Its own turn over: what waited goes now.
            if matches!(fact.body, Body::TurnEnded { .. }) {
                self.overseer.wake.notify_one();
            }
            return;
        }
        if fact.private || fact.project.as_ref().is_some_and(|p| p.private) {
            return;
        }
        let project = fact
            .project
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let agent = fact
            .agent
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let key = fact.ticket.as_ref().map(|t| t.key.clone());
        let event = match &fact.body {
            Body::TurnEnded { end, error } => Some(Event::Turn {
                agent,
                project,
                ticket: key,
                end: end.clone(),
                error: error.clone(),
            }),
            Body::RequestPermission {
                request_id,
                tool,
                arg,
            } => Some(Event::Permission {
                agent,
                agent_id,
                project,
                request: request_id.clone(),
                tool: tool.clone(),
                arg: arg.clone(),
            }),
            Body::RequestQuestion {
                request_id,
                questions,
            } => Some(Event::Question {
                agent,
                agent_id,
                project,
                request: request_id.clone(),
                questions: questions.clone(),
            }),
            Body::TicketColumn { to, .. } if to == "review" || to == "done" => {
                fact.ticket.as_ref().map(|t| Event::Column {
                    ticket: t.key.clone(),
                    title: t.title.clone(),
                    project,
                    to: to.clone(),
                })
            }
            Body::TicketConflict { files, .. } => key.map(|ticket| Event::Blocked {
                ticket,
                project,
                why: tr_in!(
                    i18n::claude(),
                    "conflit de fusion ({files})",
                    "merge conflict ({files})",
                    files = files.join(", ")
                ),
            }),
            Body::TestsResult { passed: false } => key.map(|ticket| Event::Blocked {
                ticket,
                project,
                why: tr_in!(i18n::claude(), "tests en échec", "tests failed"),
            }),
            _ => None,
        };
        if let (Some(e), Some(t), Some(p)) = (&event, &fact.ticket, &fact.project) {
            let news = match e {
                Event::Column { to, .. } if to == "done" => Some((NewsKind::Done, String::new())),
                Event::Column { .. } => Some((NewsKind::Review, String::new())),
                Event::Blocked { why, .. } => Some((NewsKind::Blocked, why.clone())),
                _ => None,
            };
            if let Some((kind, text)) = news {
                self.overseer_news(kind, t, &p.id, &p.name, text);
            }
        }
        if let Some(e) = event {
            self.overseer_note(e);
        }
    }

    /// A piece of news of the ticket `t`: a card of its discussion (ESC-468), the window told.
    fn overseer_news(
        &self,
        kind: NewsKind,
        t: &crate::facts::TicketRef,
        project_id: &str,
        project: &str,
        text: String,
    ) {
        {
            let mut st = self.overseer.state.lock();
            let id = st.next_id("n");
            st.keep_news(News {
                id,
                at: now_ms(),
                kind,
                ticket_id: t.id.clone(),
                ticket: t.key.clone(),
                title: t.title.clone(),
                project_id: project_id.to_string(),
                project: project.to_string(),
                text,
            });
        }
        self.overseer_changed();
    }

    /// An event that wakes it, gathered with those that come close to it.
    pub(crate) fn overseer_note(&self, e: Event) {
        if !self.overseer_wanted() {
            return;
        }
        self.overseer.batch.lock().push(e, now_ms());
        self.overseer.wake.notify_one();
    }

    /// The news of a ticket's pull request (CI failed, changes asked…), for it.
    pub(crate) fn overseer_pr_news(&self, ticket_id: &str, news: &[String]) {
        if news.is_empty() || !self.overseer_wanted() {
            return;
        }
        let Ok(t) = self.ticket(ticket_id) else {
            return;
        };
        let Ok(p) = self.project(&t.project_id) else {
            return;
        };
        if p.private {
            return;
        }
        let tref = crate::facts::TicketRef {
            id: t.id.clone(),
            key: t.key.clone(),
            title: t.title.clone(),
        };
        self.overseer_news(NewsKind::Ci, &tref, &p.id, &p.name, news.join(", "));
        self.overseer_note(Event::Ci {
            ticket: t.key.clone(),
            project: p.name,
            news: news.join(", "),
        });
    }

    /// A scheduled task's due time made its ticket, for it.
    pub(crate) fn overseer_scheduled(&self, task: &str, project_id: &str, ticket: &str) {
        let Ok(p) = self.project(project_id) else {
            return;
        };
        if p.private {
            return;
        }
        self.overseer_note(Event::Scheduled {
            task: task.to_string(),
            project: p.name,
            ticket: ticket.to_string(),
        });
    }

    /// While the app runs: the events gathered go to it in one message once `group_ms` passed, when it is not at work
    /// (what comes during its turn goes after it).
    pub(crate) async fn overseer_loop(self: Arc<Self>) {
        loop {
            let wait = {
                let b = self.overseer.batch.lock();
                b.due_at(self.overseer.group_ms.load(Ordering::Relaxed))
                    .map(|at| (at - now_ms()).max(0) as u64)
            };
            match wait {
                Some(ms) => {
                    let _ = tokio::time::timeout(
                        std::time::Duration::from_millis(ms.max(20)),
                        self.overseer.wake.notified(),
                    )
                    .await;
                }
                None => self.overseer.wake.notified().await,
            }
            if self.quitting.load(Ordering::Acquire) {
                return;
            }
            self.overseer_flush().await;
        }
    }

    /// The events gathered, sent when they are ready.
    pub(crate) async fn overseer_flush(self: &Arc<Self>) {
        if !self.overseer_wanted() {
            // Off: what came is dropped, not told later.
            self.overseer.batch.lock().take();
            return;
        }
        let Some(h) = self.overseer_agent() else {
            return;
        };
        let busy = {
            let rt = h.lock();
            rt.meta.status.is_active() || rt.meta.archived
        };
        let (events, dropped) = {
            let mut b = self.overseer.batch.lock();
            if !b.ready(
                now_ms(),
                self.overseer.group_ms.load(Ordering::Relaxed),
                busy,
            ) {
                return;
            }
            b.take()
        };
        let id = h.lock().meta.id.clone();
        let text = self.overseer_renewed(&h, overseer::wake_text(i18n::claude(), &events, dropped));
        self.overseer.wakes.fetch_add(1, Ordering::AcqRel);
        if let Err(e) = self.send_by_app(&id, text).await {
            log::warn!("overseer: not woken: {e:#}");
        }
    }

    // ---------- what it does ----------

    /// What its call of `tool` did, in its journal: an action (what can be undone of it, from its answer `answer`), or a
    /// refusal. Its readings are none.
    pub(crate) fn overseer_record(
        &self,
        caller: &Caller,
        tool: &str,
        summary: &str,
        result: &std::result::Result<String, ToolError>,
    ) {
        if overseer::reads_only(tool) || !self.overseer_calls(caller) {
            return;
        }
        // A message held is in the journal already (`overseer_hold`).
        if tool == "send_message" && result.is_ok() {
            return;
        }
        let now = now_ms();
        let (outcome, why, undo) = match result {
            Ok(text) => {
                let answer: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
                (Outcome::Done, None, overseer::undo_of(tool, &answer, now))
            }
            Err(ToolError::Refused(m)) => (Outcome::Refused, Some(m.clone()), None),
            Err(ToolError::Failed(m)) => (Outcome::Failed, Some(m.clone()), None),
        };
        {
            let mut st = self.overseer.state.lock();
            let id = st.next_id("a");
            st.keep_action(Action {
                id,
                at: now,
                tool: tool.to_string(),
                text: summary.to_string(),
                outcome,
                why,
                undo,
            });
        }
        self.overseer_changed();
    }

    /// The guard of the actions that go out: the Overseer passes only under an active instruction that covers one of
    /// `actions` at `place` (one for once is used up); anyone else is not its concern.
    pub(crate) fn overseer_gate(
        &self,
        caller: &Caller,
        actions: &[Outgoing],
        place: &Place,
    ) -> std::result::Result<(), ToolError> {
        if actions.is_empty() || !self.overseer_calls(caller) {
            return Ok(());
        }
        let now = now_ms();
        let used = {
            let mut st = self.overseer.state.lock();
            let found =
                covering(&st.directives, actions, place, now).map(|d| (d.id.clone(), d.scope.once));
            match found {
                Some((id, once)) => {
                    if once {
                        if let Some(d) = st.directives.iter_mut().find(|d| d.id == id) {
                            d.revoked_at = Some(now);
                        }
                    }
                    Some(once)
                }
                None => None,
            }
        };
        match used {
            Some(once) => {
                if once {
                    self.overseer_changed();
                }
                Ok(())
            }
            None => {
                let lang = i18n::claude();
                let what: Vec<String> = actions.iter().map(|a| a.label(lang)).collect();
                Err(ToolError::Refused(tr_in!(
                    lang,
                    "Escouade refuse : aucune consigne active de l’utilisateur ne te permet de {what} ici. Les actions qui sortent ne passent que sous une consigne qu’il t’a donnée dans un de ses messages : demande-lui, en une phrase.",
                    "Escouade refuses: no active instruction of the user lets you {what} here. Actions that go out pass only under an instruction the user gave you in one of their messages: ask the user, in one sentence.",
                    what = what.join(tr_in!(lang, " ou ", " or ").as_str())
                )))
            }
        }
    }

    /// An instruction from the user's message `message`, by the user's words `quote` (`check_source`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn overseer_add_directive(
        &self,
        message: &str,
        quote: &str,
        summary: &str,
        actions: Vec<Outgoing>,
        scope: Scope,
    ) -> std::result::Result<Directive, ToolError> {
        let now = now_ms();
        let lang = i18n::claude();
        let d = {
            let mut st = self.overseer.state.lock();
            let source = check_source(&st.messages, message, quote, &actions, now)
                .map_err(|r| ToolError::Refused(r.text(lang)))?
                .clone();
            let mut actions = actions;
            actions.dedup();
            let d = Directive {
                id: st.next_id("c"),
                summary: summary.trim().chars().take(200).collect(),
                actions,
                scope,
                source: Source {
                    message: source.id.clone(),
                    quote: quote.trim().to_string(),
                    at: source.at,
                },
                created_at: now,
                revoked_at: None,
            };
            st.directives.push(d.clone());
            d
        };
        self.overseer_changed();
        Ok(d)
    }

    /// The instruction `id` withdrawn (« Retirer », « arrête de merger seul »).
    pub fn overseer_revoke(&self, id: &str) -> Result<Directive> {
        let d = {
            let mut st = self.overseer.state.lock();
            let d = st
                .directives
                .iter_mut()
                .find(|d| d.id == id && d.revoked_at.is_none())
                .ok_or_else(|| {
                    anyhow!(tr!(
                        "Cette consigne n’est plus active.",
                        "This instruction is no longer active."
                    ))
                })?;
            d.revoked_at = Some(now_ms());
            d.clone()
        };
        self.overseer_changed();
        Ok(d)
    }

    /// A message of the Overseer to an agent, held `hold_ms` before it is delivered: « Annuler » meanwhile keeps it.
    pub(crate) fn overseer_hold(
        self: &Arc<Self>,
        agent_id: &str,
        agent_name: &str,
        text: &str,
    ) -> String {
        let hold = self.overseer.hold_ms.load(Ordering::Relaxed);
        let now = now_ms();
        let action = {
            let mut st = self.overseer.state.lock();
            let id = st.next_id("a");
            st.keep_action(Action {
                id: id.clone(),
                at: now,
                tool: "send_message".into(),
                text: tr!(
                    "agent: {agent_name} · {chars} caractères",
                    "agent: {agent_name} · {chars} characters",
                    chars = text.chars().count()
                ),
                outcome: Outcome::Held,
                why: None,
                undo: Some(Undo {
                    what: UndoWhat::Message {
                        agent_id: agent_id.to_string(),
                    },
                    until: now + hold,
                }),
            });
            id
        };
        self.overseer_changed();
        let (core, to, text, act) = (
            self.clone(),
            agent_id.to_string(),
            text.to_string(),
            action.clone(),
        );
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(hold.max(0) as u64)).await;
            let go = {
                let mut st = core.overseer.state.lock();
                match st.action_mut(&act) {
                    Some(a) if a.outcome == Outcome::Held => {
                        a.outcome = Outcome::Done;
                        a.undo = None;
                        true
                    }
                    _ => false,
                }
            };
            if !go {
                return;
            }
            core.overseer_changed();
            if let Err(e) = core
                .send_message_from(&to, Some(overseer::NAME), text, Vec::new())
                .await
            {
                log::warn!("overseer: message to {to} not sent: {e:#}");
                let mut st = core.overseer.state.lock();
                if let Some(a) = st.action_mut(&act) {
                    a.outcome = Outcome::Failed;
                    a.why = Some(format!("{e:#}"));
                }
                drop(st);
                core.overseer_changed();
            }
        });
        action
    }

    /// « Annuler » of the action `id`, while it can be.
    pub async fn overseer_undo(self: &Arc<Self>, id: &str) -> Result<()> {
        let what = {
            let mut st = self.overseer.state.lock();
            let now = now_ms();
            let a = st
                .action_mut(id)
                .ok_or_else(|| anyhow!(tr!("Action introuvable.", "Action not found.")))?;
            if !a.undoable(now) {
                bail!(tr!(
                    "Cette action ne s’annule plus.",
                    "This action can no longer be undone."
                ));
            }
            let what = a.undo.as_ref().map(|u| u.what.clone()).expect("undoable");
            if matches!(what, UndoWhat::Message { .. }) {
                // Not delivered: the task that holds it sends nothing.
                a.outcome = Outcome::Undone;
                a.undo = None;
            }
            what
        };
        let done: Result<()> = match &what {
            UndoWhat::Message { .. } => Ok(()),
            UndoWhat::Directive { directive_id } => self.overseer_revoke(directive_id).map(|_| ()),
            UndoWhat::Agent { agent_id } => {
                let _ = self.interrupt(agent_id).await;
                self.archive_agent(agent_id, true).await
            }
            UndoWhat::Ticket { ticket_id } => {
                let t = self.ticket(ticket_id)?;
                if t.column != crate::model::Column::Todo || t.agent_id.is_some() {
                    bail!(tr!(
                        "Le ticket {key} a démarré : il ne s’annule plus (supprime-le du Kanban).",
                        "The ticket {key} has started: it can no longer be undone (delete it from the Kanban).",
                        key = t.key
                    ));
                }
                self.ticket_delete(ticket_id).await
            }
            UndoWhat::Scheduled { task_id } => self.scheduled_delete(task_id),
        };
        done?;
        {
            let mut st = self.overseer.state.lock();
            if let Some(a) = st.action_mut(id) {
                a.outcome = Outcome::Undone;
                a.undo = None;
            }
        }
        self.overseer_changed();
        Ok(())
    }

    /// A request of an agent the Overseer leaves to the user, unsure of the answer: it stays in « Attend ta réponse »,
    /// and the user is told why.
    pub(crate) fn overseer_leave(
        self: &Arc<Self>,
        agent_id: &str,
        agent_name: &str,
        project_id: &str,
        request: &str,
        why: &str,
    ) {
        {
            let mut st = self.overseer.state.lock();
            let id = st.next_id("a");
            st.keep_action(Action {
                id,
                at: now_ms(),
                tool: "leave_to_user".into(),
                text: tr!(
                    "agent: {agent_name} · demande: {request}",
                    "agent: {agent_name} · request: {request}"
                ),
                outcome: Outcome::Done,
                why: Some(why.to_string()),
                undo: None,
            });
            st.left
                .entry(agent_id.to_string())
                .or_default()
                .push(request.to_string());
        }
        self.overseer_changed();
        let project = self.project(project_id).map(|p| p.name).unwrap_or_default();
        self.alert(
            crate::agent::NotifyKind::Question,
            format!("{} · {project} · {agent_name}", overseer::NAME),
            tr!(
                "L’Overseer te laisse la réponse : {why}",
                "The Overseer leaves the answer to you: {why}"
            ),
            UiEvent::Focus {
                project_id: project_id.to_string(),
                agent_id: Some(agent_id.to_string()),
            },
        );
    }
}
