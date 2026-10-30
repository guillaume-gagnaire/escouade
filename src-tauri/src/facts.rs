//! The facts' bus (PLG-5, ESC-177; docs/PLUGINS.md §1.6): what happened in Escouade, said once,
//! typed and versioned, where the state changes (an agent's status, a turn's end, a ticket's column,
//! a project added…), and never read back from the window's events (`UiEvent`), which carry the
//! whole state for the interface and change with it.
//!
//! A fact says its project, its ticket and its agent by their ids and their names, never a
//! conversation: what a plugin learns of a turn is how it ended, not what was said in it.
//!
//! Whoever changes the state emits (`Emitter::emit`, cheap, never blocking: an agent's status is
//! told under its lock); a thread of the bus fills in the names (`Resolve`, the core), numbers the
//! fact and hands it to every sink in the order it came. The plugins' sink (`plugins::events`)
//! keeps a bounded queue per plugin and never waits for one.

use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Weak};

/// The version of the facts' shape: a field added keeps it, a field removed or changed in meaning
/// takes the next one.
pub const VERSION: u32 = 1;

/// What happened, and what it says of itself.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "name", content = "data", rename_all_fields = "camelCase")]
pub enum Body {
    /// An agent was made (by hand, for a ticket, by a client of the MCP server).
    #[serde(rename = "agent.created")]
    AgentCreated { harness: String, model: String },
    /// It was named (after its first message, or by hand): the facts after it say the new name.
    #[serde(rename = "agent.renamed")]
    AgentRenamed { from: String, to: String },
    /// Its status changed: `idle`, `running`, `waiting` (a request waits for an answer), `done`,
    /// `error`.
    #[serde(rename = "agent.status")]
    AgentStatus { status: String, was: String },
    /// It was archived (`removed`: false) or deleted.
    #[serde(rename = "agent.stopped")]
    AgentStopped { removed: bool },
    /// It went on with another harness.
    #[serde(rename = "agent.handover")]
    AgentHandover { from: String, to: String },
    /// A turn ended: `finished`, `interrupted`, `limited` (the usage limit), `signIn` (a sign-in
    /// that could not be refreshed) or `error` (with its first line).
    #[serde(rename = "turn.ended")]
    TurnEnded {
        end: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// A permission waits: the tool and what it acts on, as its card sums it up.
    #[serde(rename = "request.permission")]
    RequestPermission {
        request_id: String,
        tool: String,
        arg: String,
    },
    /// A question waits: its questions.
    #[serde(rename = "request.question")]
    RequestQuestion {
        request_id: String,
        questions: Vec<String>,
    },
    /// A request was answered: `permission` with its decision (`allow`, `always`, `deny`), or
    /// `question`; `by`, a client of the MCP server, when not the user.
    #[serde(rename = "request.answered")]
    RequestAnswered {
        request_id: String,
        kind: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        decision: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        by: Option<String>,
    },
    #[serde(rename = "ticket.created")]
    TicketCreated { column: String },
    /// From one column to another: `todo`, `doing`, `review`, `done`.
    #[serde(rename = "ticket.column")]
    TicketColumn { from: String, to: String },
    /// Its agent reported its criteria: how many hold, out of how many.
    #[serde(rename = "ticket.report")]
    TicketReport { met: usize, total: usize },
    /// Its validation started.
    #[serde(rename = "ticket.approved")]
    TicketApproved {},
    /// Sent back to its agent, with the comment.
    #[serde(rename = "ticket.rejected")]
    TicketRejected { comment: String },
    /// Merged into its target branch (`url`: the pull request, when one was opened instead).
    #[serde(rename = "ticket.merged")]
    TicketMerged {
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        commit: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    /// Its merge met a conflict, in these files.
    #[serde(rename = "ticket.conflict")]
    TicketConflict { target: String, files: Vec<String> },
    #[serde(rename = "project.added")]
    ProjectAdded { path: String },
    #[serde(rename = "project.closed")]
    ProjectClosed {},
    #[serde(rename = "worktree.created")]
    WorktreeCreated { branch: String, path: String },
    #[serde(rename = "worktree.removed")]
    WorktreeRemoved { branch: String, path: String },
    /// A test launch started (`run`: its name).
    #[serde(rename = "launch.started")]
    LaunchStarted { run: String },
    /// It answers at `url`.
    #[serde(rename = "launch.ready")]
    LaunchReady { url: String },
    #[serde(rename = "launch.stopped")]
    LaunchStopped {},
    /// The tests of a validation: passed or not.
    #[serde(rename = "tests.result")]
    TestsResult { passed: bool },
    #[serde(rename = "app.started")]
    AppStarted { version: String },
    /// A page of the docs was written (`page`: its path).
    #[serde(rename = "docs.written")]
    DocsWritten { page: String },
    /// A ticket's copy of the docs was poured on the docs' branch, at its validation.
    #[serde(rename = "docs.filed")]
    DocsFiled { pages: usize },
    /// A page of the docs was compared to the code: how many gaps were found.
    #[serde(rename = "docs.gaps")]
    DocsGaps { page: String, gaps: usize },
    /// A memory of the project was written (`new`: it was not there).
    #[serde(rename = "memory.written")]
    MemoryWritten { memory: String, new: bool },
    #[serde(rename = "memory.forgotten")]
    MemoryForgotten { memory: String },
    /// A decision was proposed (or revised).
    #[serde(rename = "decision.proposed")]
    DecisionProposed { number: u32, title: String },
    /// A human accepted it.
    #[serde(rename = "decision.accepted")]
    DecisionAccepted { number: u32, title: String },
    /// A pull request was opened for a ticket.
    #[serde(rename = "pr.opened")]
    PrOpened { url: String, target: String },
    /// An agent's account changed past « Changer de compte à ».
    #[serde(rename = "account.switched")]
    AccountSwitched { from: String, to: String },
}

/// Every fact's name, in the order of the docs: the protocol's schema lists the same
/// (`schemas/plugin-protocol-1.json`, `FactName`), and the bench plays them (`bench::FACTS`).
pub const NAMES: &[&str] = &[
    "agent.created",
    "agent.renamed",
    "agent.status",
    "agent.stopped",
    "agent.handover",
    "turn.ended",
    "request.permission",
    "request.question",
    "request.answered",
    "ticket.created",
    "ticket.column",
    "ticket.report",
    "ticket.approved",
    "ticket.rejected",
    "ticket.merged",
    "ticket.conflict",
    "project.added",
    "project.closed",
    "worktree.created",
    "worktree.removed",
    "launch.started",
    "launch.ready",
    "launch.stopped",
    "tests.result",
    "app.started",
    "docs.written",
    "docs.filed",
    "docs.gaps",
    "memory.written",
    "memory.forgotten",
    "decision.proposed",
    "decision.accepted",
    "pr.opened",
    "account.switched",
];

impl Body {
    /// Its name (`ticket.column`).
    pub fn name(&self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v["name"].as_str().map(str::to_string))
            .unwrap_or_default()
    }
}

/// A thing a fact is about, by its id and its name.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Named {
    pub id: String,
    pub name: String,
}

/// The project a fact is about: its id, its name, and whether it is private (a plugin gets a
/// private project's facts only when turned on for it).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ProjectRef {
    pub id: String,
    pub name: String,
    pub private: bool,
}

/// The ticket a fact is about: its id, its key and its title.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct TicketRef {
    pub id: String,
    pub key: String,
    pub title: String,
}

/// What the emitter knows of what a fact is about; the bus fills in the rest (`Resolve`).
#[derive(Debug, Clone, Default)]
pub struct About {
    pub project: Option<Named>,
    pub ticket: Option<TicketRef>,
    pub agent: Option<Named>,
    /// Whether its project is private, when the emitter knows (a project closed is gone).
    pub private: Option<bool>,
}

impl About {
    pub fn project(id: &str) -> Self {
        Self {
            project: Some(Named {
                id: id.to_string(),
                name: String::new(),
            }),
            ..Self::default()
        }
    }

    pub fn ticket(id: &str) -> Self {
        Self {
            ticket: Some(TicketRef {
                id: id.to_string(),
                ..TicketRef::default()
            }),
            ..Self::default()
        }
    }

    pub fn agent(id: &str, name: &str, project: &str) -> Self {
        Self {
            agent: Some(Named {
                id: id.to_string(),
                name: name.to_string(),
            }),
            ..Self::project(project)
        }
    }
}

/// A fact, as the sinks get it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fact {
    pub version: u32,
    /// Its number, from 1 at the app's start: two facts come in this order.
    pub seq: u64,
    /// When, in RFC 3339.
    pub at: String,
    #[serde(flatten)]
    pub body: Body,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket: Option<TicketRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<Named>,
    /// Its project is private (or unknown): it goes only to the plugins activated for it.
    #[serde(skip)]
    pub private: bool,
}

impl Fact {
    pub fn name(&self) -> String {
        self.body.name()
    }
}

/// Fills in what a fact is about: the names, the ticket of its agent, its project of its ticket.
/// Gives whether its project is private; None when its project is unknown.
pub trait Resolve: Send + Sync {
    fn resolve(&self, about: &mut About) -> Option<bool>;
}

/// What takes the facts, each in the order they came. Must not block: the bus waits for it.
pub trait Sink: Send + Sync {
    fn take(&self, fact: &Arc<Fact>);
}

enum Msg {
    Fact(Box<(Body, About)>),
    /// Answered once everything sent before was handed to the sinks.
    #[cfg_attr(not(test), allow(dead_code))]
    Flush(mpsc::Sender<()>),
}

/// What emits facts: a handle on the bus, cheap to clone, which does nothing when there is none
/// (an agent's runtime outside the core, in a test).
#[derive(Clone, Default)]
pub struct Emitter(Option<mpsc::Sender<Msg>>);

impl std::fmt::Debug for Emitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() {
            "Emitter"
        } else {
            "Emitter(none)"
        })
    }
}

impl Emitter {
    /// Says `body`, about `about`. Never blocks.
    pub fn emit(&self, body: Body, about: About) {
        if let Some(tx) = &self.0 {
            let _ = tx.send(Msg::Fact(Box::new((body, about))));
        }
    }
}

/// The bus of the app.
pub struct Bus {
    tx: mpsc::Sender<Msg>,
    sinks: Arc<RwLock<Vec<Arc<dyn Sink>>>>,
    resolver: Arc<RwLock<Option<Weak<dyn Resolve>>>>,
}

impl Bus {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Msg>();
        let sinks: Arc<RwLock<Vec<Arc<dyn Sink>>>> = Arc::default();
        let resolver: Arc<RwLock<Option<Weak<dyn Resolve>>>> = Arc::default();
        let seq = AtomicU64::new(0);
        let (s, r) = (sinks.clone(), resolver.clone());
        std::thread::Builder::new()
            .name("facts".into())
            .spawn(move || {
                while let Ok(msg) = rx.recv() {
                    match msg {
                        Msg::Flush(done) => {
                            let _ = done.send(());
                        }
                        Msg::Fact(fact) => {
                            let (body, mut about) = *fact;
                            let found = r
                                .read()
                                .as_ref()
                                .and_then(Weak::upgrade)
                                .map(|r| r.resolve(&mut about));
                            let private = match (about.private, found) {
                                (Some(p), _) => p,
                                // No project: a fact of the app.
                                _ if about.project.is_none() => false,
                                (None, Some(Some(p))) => p,
                                // A project no one knows: kept from every plugin not activated for it.
                                (None, _) => true,
                            };
                            let fact = Arc::new(Fact {
                                version: VERSION,
                                seq: seq.fetch_add(1, Ordering::Relaxed) + 1,
                                at: chrono::Utc::now()
                                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                                body,
                                project: about.project.map(|p| ProjectRef {
                                    id: p.id,
                                    name: p.name,
                                    private,
                                }),
                                ticket: about.ticket,
                                agent: about.agent,
                                private,
                            });
                            let sinks: Vec<Arc<dyn Sink>> = s.read().clone();
                            for sink in sinks {
                                sink.take(&fact);
                            }
                        }
                    }
                }
            })
            .expect("the facts' thread starts");
        Self {
            tx,
            sinks,
            resolver,
        }
    }

    pub fn emitter(&self) -> Emitter {
        Emitter(Some(self.tx.clone()))
    }

    pub fn emit(&self, body: Body, about: About) {
        let _ = self.tx.send(Msg::Fact(Box::new((body, about))));
    }

    /// From now on, `r` fills in what the facts are about.
    pub fn resolve_with(&self, r: Weak<dyn Resolve>) {
        *self.resolver.write() = Some(r);
    }

    pub fn subscribe(&self, sink: Arc<dyn Sink>) {
        self.sinks.write().push(sink);
    }

    /// Waits until every fact emitted before was handed to the sinks.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn flush(&self) {
        let (tx, rx) = mpsc::channel();
        if self.tx.send(Msg::Flush(tx)).is_ok() {
            let _ = rx.recv_timeout(std::time::Duration::from_secs(5));
        }
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

/// A sink that keeps every fact (the tests).
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Default)]
pub struct Recorder(Mutex<Vec<Arc<Fact>>>);

#[cfg_attr(not(test), allow(dead_code))]
impl Recorder {
    pub fn facts(&self) -> Vec<Arc<Fact>> {
        self.0.lock().clone()
    }

    pub fn named(&self, name: &str) -> Vec<Arc<Fact>> {
        self.0
            .lock()
            .iter()
            .filter(|f| f.name() == name)
            .cloned()
            .collect()
    }
}

impl Sink for Recorder {
    fn take(&self, fact: &Arc<Fact>) {
        self.0.lock().push(fact.clone());
    }
}

/// The word of a status, as the facts say it.
pub fn status_word(s: crate::model::AgentStatus) -> String {
    use crate::model::AgentStatus::*;
    match s {
        Idle => "idle",
        Running => "running",
        Waiting => "waiting",
        Done => "done",
        Error => "error",
    }
    .into()
}

/// The word of a column, as the facts say it.
pub fn column_word(c: crate::model::Column) -> String {
    serde_json::to_value(c)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
        .to_lowercase()
}

/// The fact of a turn's end.
pub fn turn_ended(end: &crate::board::TurnEnd) -> Body {
    use crate::board::TurnEnd::*;
    let (end, error) = match end {
        Finished(_) => ("finished", None),
        Interrupted => ("interrupted", None),
        Limited => ("limited", None),
        SignIn => ("signIn", None),
        Error(e) => (
            "error",
            Some(e.lines().next().unwrap_or("").chars().take(300).collect()),
        ),
    };
    Body::TurnEnded {
        end: end.into(),
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_fact_says_its_name_its_data_and_what_it_is_about() {
        let bus = Bus::new();
        let rec = Arc::new(Recorder::default());
        bus.subscribe(rec.clone());
        bus.emit(
            Body::TicketColumn {
                from: "doing".into(),
                to: "review".into(),
            },
            About {
                ticket: Some(TicketRef {
                    id: "t1".into(),
                    key: "ESC-1".into(),
                    title: "Un ticket".into(),
                }),
                ..About::project("p1")
            },
        );
        bus.flush();
        let f = rec.facts();
        assert_eq!(f.len(), 1);
        let v = serde_json::to_value(&*f[0]).unwrap();
        assert_eq!(v["name"], "ticket.column");
        assert_eq!(v["version"], VERSION);
        assert_eq!(v["seq"], 1);
        assert_eq!(v["data"], json!({ "from": "doing", "to": "review" }));
        assert_eq!(v["ticket"]["key"], "ESC-1");
        assert_eq!(v["project"]["id"], "p1");
        // No resolver: a project no one knows is kept as private.
        assert!(f[0].private);
    }

    #[test]
    fn every_name_is_a_body_and_the_reverse() {
        let one = |b: Body| b.name();
        let all = [
            one(Body::AgentCreated {
                harness: String::new(),
                model: String::new(),
            }),
            one(Body::AppStarted {
                version: String::new(),
            }),
            one(Body::TicketApproved {}),
        ];
        for n in all {
            assert!(NAMES.contains(&n.as_str()), "{n}");
        }
        let mut sorted = NAMES.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), NAMES.len(), "no name twice");
    }
}
