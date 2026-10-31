//! Where each agent is in the workflow of its project, in real time, and where it has been
//! (ESC-193, choices 146 to 151, 153 and 154 of `UX-NOTES.md`; the study is `docs/LOOPS.md`).
//!
//! Nothing is asked of the agent: what it does is compared to what the steps of the workflow
//! (`workflow::Workflow`) say signals them. The *facts* (`Fact`) are the message it was sent, the
//! tools its main thread called (a sub-agent by its `subagent_type`, a skill, a file written), the
//! cost and the end of its turns. They are the same whatever the harness: every adapter, and the
//! views of the terminal interfaces, put them in the conversation, where `facts_of_ops` reads them.
//!
//! A step begins at the first fact that signals it and ends at the first fact that signals
//! another; its duration and its cost are those of the turns between the two. A way back to a step
//! already passed carries the reason the step gave. An agent whose message comes from no door, and
//! whose whole turn used tools none of which signals a step, is *outside* the workflow until the
//! next door (never decided on a single command). Each passage keeps the version of the workflow
//! it began in. The history is kept with the agent in the data folder (`Core::track_workflow`).

use crate::model::{ConvOp, EngineId};
use crate::workflow::{Kind, Progress, Scan, Step, Workflow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// How many passages and turns an agent's history keeps (the oldest go first).
pub const MAX_HISTORY: usize = 300;
/// How many things a passage says it did.
const MAX_DID: usize = 12;
/// The longest line of a reason or of what an agent does outside.
const MAX_LINE: usize = 160;

/// One thing the agent did, seen in its conversation.
#[derive(Debug, Clone, PartialEq)]
pub enum Fact {
    /// A message it was sent (not a notification).
    Message { text: String },
    /// A tool its main thread called, with its input once known.
    Tool { name: String, input: Value },
    /// A sub-agent it launched came back.
    SubagentDone { name: String },
    /// What a turn cost (summed over the turns reported together).
    Cost(f64),
    /// A turn ended: how, with the assistant's last text.
    TurnEnd { end: End, text: String },
    /// What the list of a prompt loop says after a turn.
    List(Progress),
    /// The ticket it works on is on its `n`th loop of `max`.
    TicketLoop { n: u32, max: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Finished,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// Still in it.
    #[default]
    Running,
    /// Left for the step after.
    Done,
    /// Left for a step already passed, with a reason.
    Back,
    /// A turn failed in it.
    Failed,
    /// Stopped by the user.
    Stopped,
    /// Left the workflow (the agent did something else).
    Left,
}

/// One stay of an agent in a step (or outside the workflow, `step` None).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Passage {
    /// Counted from the first passage ever kept, not from the first one still kept.
    pub n: u32,
    pub workflow: String,
    /// None outside the workflow.
    pub step: Option<String>,
    /// The version of the workflow when it began (`version_of`): the step is the one of that version.
    pub version: String,
    /// The `pass`th stay of the agent in this step (« 2ᵉ passage »).
    pub pass: u32,
    /// The door typed that began it.
    pub door: Option<String>,
    pub at: i64,
    pub ended: Option<i64>,
    pub ms: u64,
    pub cost: f64,
    /// What was done in it: `agent:<subagent_type>`, `skill:<name>`, `file:<path>`.
    pub did: Vec<String>,
    pub outcome: Outcome,
    /// Why it went back, as the step wrote it.
    pub reason: Option<String>,
    /// The file that decided it (the step's own).
    pub decided_by: Option<String>,
    pub back_to: Option<String>,
    /// What the agent did, outside the workflow.
    pub doing: Option<String>,
}

/// One turn of a prompt run over and over.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LoopTurn {
    pub n: u32,
    pub at: i64,
    pub ended: Option<i64>,
    pub ms: u64,
    pub cost: f64,
    /// The boxes of the list after it.
    pub checked: Option<Progress>,
    /// A box was checked in it (the turn is red when not).
    pub ticked: Option<bool>,
    pub outcome: Outcome,
}

/// What a harness lets be followed (choice 150): `missing` is what the interface says it lacks.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Support {
    pub partial: bool,
    /// `subagents`, `cost`, `timing`.
    pub missing: Vec<String>,
}

/// The agent's message that comes from no door, until its turn says whether it left.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Stray {
    pub text: String,
    pub at: i64,
    /// Tools called since that signal no step.
    pub tools: u32,
}

/// An agent in its workflow: where it is and its history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentTrack {
    pub agent_id: String,
    /// The workflow it follows (kept while it is outside, for the way back).
    pub workflow: Option<String>,
    /// Its step now; None between two or outside.
    pub step: Option<String>,
    /// Its version of the step now (`Passage::version`).
    pub version: String,
    /// It is doing something else (what), until the next door.
    pub outside: Option<String>,
    /// The sub-agents it launched that have not come back.
    pub subagents: Vec<String>,
    pub passages: Vec<Passage>,
    pub next_n: u32,
    /// For a prompt run over and over: its turns.
    pub turns: Vec<LoopTurn>,
    pub next_turn: u32,
    pub support: Support,
    /// When it entered the workflow, and its time and cost in it.
    pub since: i64,
    pub ms: u64,
    pub cost: f64,
    pub ticket_loop: Option<(u32, u32)>,
    pub stray: Option<Stray>,
    /// The last line of the assistant's last turn (the reason of a way back).
    pub last_line: String,
    pub updated: i64,
}

/// What the interface gets: the track and whether the workflow changed under the step in course.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackView {
    #[serde(flatten)]
    pub track: AgentTrack,
    pub project_id: String,
    /// The workflow was read again, different, since the step in course began (choice 154): the
    /// agent keeps its version until its next step.
    pub behind: bool,
}

pub fn support_of(engine: EngineId, view: bool) -> Support {
    let (partial, missing): (bool, &[&str]) = match (engine, view) {
        (EngineId::Codex, _) => (true, &["subagents"]),
        (EngineId::Cursor, false) => (true, &["cost"]),
        (EngineId::Cursor, true) => (true, &["subagents", "cost", "timing"]),
        _ => (false, &[]),
    };
    Support {
        partial,
        missing: missing.iter().map(|m| m.to_string()).collect(),
    }
}

fn clip(s: &str) -> String {
    let line = s
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    line.chars().take(MAX_LINE).collect()
}

/// A short stable name of the workflow's steps as they are: what a passage keeps.
pub fn version_of(wf: &Workflow) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut eat = |s: &str| {
        for b in s.bytes().chain(std::iter::once(0u8)) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    for s in &wf.steps {
        eat(&s.name);
        eat(&s.command);
        for a in &s.agents {
            eat(&a.name);
        }
        for w in &s.writes {
            eat(w);
        }
    }
    for t in &wf.transitions {
        eat(&t.from);
        eat(&t.to);
        eat(if t.back { "b" } else { "f" });
        eat(&t.max.unwrap_or(0).to_string());
    }
    format!("{h:016x}")
}

// ---------- the facts of a conversation ----------

/// A call whose input is not known yet (a streaming tool), or a sub-agent not back.
#[derive(Debug, Clone, Default)]
pub struct Pending {
    pub name: String,
    pub sub: Option<String>,
}

fn is_agent_tool(name: &str) -> bool {
    matches!(name, "Task" | "Agent" | "task")
}

fn tool_fact(name: &str, input: &Value, pending: &mut HashMap<String, Pending>, id: &str) -> Fact {
    if is_agent_tool(name) {
        if let Some(sub) = input["subagent_type"].as_str().filter(|s| !s.is_empty()) {
            pending.insert(
                id.to_string(),
                Pending {
                    name: name.to_string(),
                    sub: Some(sub.to_string()),
                },
            );
        }
    }
    Fact::Tool {
        name: name.to_string(),
        input: input.clone(),
    }
}

/// The facts a batch of conversation operations tell: the messages sent, the tools of the main
/// thread (once their input is there) and the sub-agents that came back. `pending` is kept between
/// calls, per agent.
pub fn facts_of_ops(ops: &[ConvOp], pending: &mut HashMap<String, Pending>) -> Vec<Fact> {
    let mut out = Vec::new();
    for op in ops {
        match op {
            ConvOp::Append { item } => match item["kind"].as_str() {
                Some("user") => {
                    let origin = item["origin"].as_str().unwrap_or("");
                    let text = item["text"].as_str().unwrap_or("");
                    if !text.trim().is_empty() && !matches!(origin, "task-notification" | "peer") {
                        out.push(Fact::Message {
                            text: text.to_string(),
                        });
                    }
                }
                Some("tool") if item["parent"].is_null() => {
                    let (id, name) = (
                        item["id"].as_str().unwrap_or_default(),
                        item["name"].as_str().unwrap_or_default(),
                    );
                    let known = item["input"].as_object().is_some_and(|o| !o.is_empty());
                    if known {
                        out.push(tool_fact(name, &item["input"], pending, id));
                    } else {
                        pending.insert(
                            id.to_string(),
                            Pending {
                                name: name.to_string(),
                                sub: None,
                            },
                        );
                    }
                }
                _ => {}
            },
            ConvOp::Patch { id, patch } => {
                let Some(p) = pending.get(id).cloned() else {
                    continue;
                };
                if p.sub.is_none() && patch["input"].as_object().is_some_and(|o| !o.is_empty()) {
                    let name = patch["name"].as_str().unwrap_or(&p.name).to_string();
                    pending.remove(id);
                    out.push(tool_fact(&name, &patch["input"], pending, id));
                } else if let Some(sub) = p.sub {
                    let done = patch["status"]
                        .as_str()
                        .is_some_and(|s| !matches!(s, "running" | "pending" | ""));
                    if done {
                        pending.remove(id);
                        out.push(Fact::SubagentDone { name: sub });
                    }
                }
            }
            ConvOp::Delta { .. } => {}
        }
    }
    out
}

// ---------- the signals ----------

enum Sig<'a> {
    Door(&'a str),
    Skill(&'a str),
    Agent(&'a str),
    File(&'a str),
}

fn word(s: &str) -> String {
    s.trim().trim_start_matches('/').to_lowercase()
}

/// The command a message begins with: `/gsd:plan-phase 3` is `gsd:plan-phase`.
fn command_of(text: &str) -> Option<&str> {
    let first = text.split_whitespace().next()?;
    first.strip_prefix('/').filter(|w| !w.is_empty())
}

fn step_matches(step: &Step, sig: &Sig) -> bool {
    match sig {
        Sig::Door(w) | Sig::Skill(w) => {
            let w = word(w);
            let names = |n: &str| {
                let n = word(n);
                n == w || n.ends_with(&format!(":{w}")) || w.ends_with(&format!(":{n}"))
            };
            // A step corrected by the user (ESC-195) is also signalled by `also`: the commands
            // and names of the steps merged into it, its name as read, the commands added.
            !w.is_empty()
                && (word(&step.command) == w
                    || names(&step.name)
                    || step.also.iter().any(|a| names(a)))
        }
        Sig::Agent(a) => step.agents.iter().any(|x| x.name.eq_ignore_ascii_case(a)),
        Sig::File(p) => {
            let p = p.replace('\\', "/");
            step.writes.iter().any(|w| {
                let w = w.replace('\\', "/");
                p == w || p.ends_with(&format!("/{w}")) || w.ends_with(&format!("/{p}"))
            })
        }
    }
}

/// Whether a message is the prompt of a loop of one prompt: it names the prompt's file, or is a
/// `/loop`.
fn runs_prompt(wf: &Workflow, text: &str) -> bool {
    if wf.kind != Kind::Prompt {
        return false;
    }
    let lower = text.to_lowercase();
    if lower.trim_start().starts_with("/loop") {
        return true;
    }
    let file = wf
        .prompt_loop
        .as_ref()
        .map(|p| {
            p.prompt
                .rsplit('/')
                .next()
                .unwrap_or(&p.prompt)
                .to_lowercase()
        })
        .filter(|f| !f.is_empty());
    file.is_some_and(|f| lower.contains(&f))
        || wf.doors.iter().any(|d| {
            !d.command.is_empty()
                && command_of(text).is_some_and(|c| word(&d.command) == c.to_lowercase())
        })
}

/// Where a signal leads: the workflow and step. A door or a skill leads to the first step that
/// bears its name, the one of the current workflow first; a sub-agent or a file only to the step
/// in course if it is one of them, or to the only step that has it.
fn locate<'a>(
    scan: &'a Scan,
    track: &AgentTrack,
    repo: &str,
    sig: &Sig,
) -> Option<(&'a Workflow, &'a Step)> {
    let mut found: Vec<(&Workflow, &Step)> = Vec::new();
    for wf in scan.workflows.iter().filter(|w| w.kind == Kind::Cycle) {
        for step in &wf.steps {
            if step_matches(step, sig) {
                found.push((wf, step));
            }
        }
        // A door that runs several steps enters the first of them.
        if let Sig::Door(w) = sig {
            for d in &wf.doors {
                if d.enters.is_empty() || word(&d.command) != word(w) {
                    continue;
                }
                if let Some(step) = d
                    .enters
                    .iter()
                    .find_map(|e| wf.steps.iter().find(|s| &s.name == e))
                {
                    found.push((wf, step));
                }
            }
        }
    }
    let current = |(wf, step): &(&Workflow, &Step)| {
        track.workflow.as_deref() == Some(wf.id.as_str())
            && track.step.as_deref() == Some(step.name.as_str())
    };
    match sig {
        Sig::Door(_) | Sig::Skill(_) => {
            found.sort_by_key(|(wf, step)| {
                (
                    track.workflow.as_deref() != Some(wf.id.as_str()),
                    wf.repo != repo,
                    track.step.as_deref() != Some(step.name.as_str()),
                )
            });
            found.into_iter().next()
        }
        Sig::Agent(_) | Sig::File(_) => {
            if let Some(c) = found.iter().find(|f| current(f)) {
                return Some(*c);
            }
            // Of the current workflow first; ambiguous among several steps, none.
            let same: Vec<_> = found
                .iter()
                .filter(|(wf, _)| track.workflow.as_deref() == Some(wf.id.as_str()))
                .copied()
                .collect();
            let pool = if same.is_empty() { found } else { same };
            (pool.len() == 1).then(|| pool[0])
        }
    }
}

fn push_did(p: &mut Passage, what: String) {
    if !p.did.contains(&what) && p.did.len() < MAX_DID {
        p.did.push(what);
    }
}

// ---------- the tracking ----------

impl AgentTrack {
    pub fn new(agent_id: &str, engine: EngineId, view: bool) -> Self {
        AgentTrack {
            agent_id: agent_id.to_string(),
            support: support_of(engine, view),
            ..Default::default()
        }
    }

    /// The passage in course.
    fn open(&mut self) -> Option<&mut Passage> {
        self.passages.last_mut().filter(|p| p.ended.is_none())
    }

    /// Whether the agent is in a workflow of one prompt run over and over.
    fn in_loop(&self, scan: &Scan) -> bool {
        self.workflow.as_deref().is_some_and(|id| {
            scan.workflows
                .iter()
                .any(|w| w.id == id && w.kind == Kind::Prompt)
        })
    }

    fn close(&mut self, at: i64, outcome: Outcome, back: Option<(&str, String, Option<String>)>) {
        let last_line = self.last_line.clone();
        let Some(p) = self.open() else { return };
        p.ended = Some(at);
        p.ms = (at - p.at).max(0) as u64;
        if p.outcome == Outcome::Running {
            p.outcome = outcome;
        }
        if let Some((to, reason, by)) = back {
            p.back_to = Some(to.to_string());
            p.reason = Some(if reason.is_empty() { last_line } else { reason });
            p.decided_by = by;
        }
    }

    fn push(&mut self, mut p: Passage) {
        self.next_n += 1;
        p.n = self.next_n;
        self.passages.push(p);
        if self.passages.len() > MAX_HISTORY {
            self.passages.remove(0);
        }
    }

    /// Goes into `step` of `wf` (a door typed when `door` is given): the passage in course ends,
    /// forward or back.
    fn enter(&mut self, wf: &Workflow, step: &Step, door: Option<String>, at: i64) {
        let version = version_of(wf);
        let same_wf = self.workflow.as_deref() == Some(wf.id.as_str());
        // Back: a transition says so, or the step is not after the one left.
        let from = self
            .step
            .clone()
            .filter(|_| same_wf)
            .and_then(|n| wf.steps.iter().find(|s| s.name == n));
        let back = from.and_then(|f| {
            let t = wf
                .transitions
                .iter()
                .find(|t| t.from == f.name && t.to == step.name && t.back);
            (t.is_some() || step.order < f.order).then(|| {
                (
                    t.and_then(|t| t.condition.clone()).unwrap_or_default(),
                    f.path.clone(),
                )
            })
        });
        match back {
            Some((condition, path)) => {
                let reason = if self.last_line.is_empty() {
                    condition
                } else {
                    self.last_line.clone()
                };
                self.close(at, Outcome::Back, Some((&step.name, reason, Some(path))));
            }
            None => self.close(at, Outcome::Done, None),
        }
        if !same_wf {
            self.since = at;
            self.ms = 0;
            self.cost = 0.0;
        }
        let pass = 1 + self
            .passages
            .iter()
            .filter(|p| p.workflow == wf.id && p.step.as_deref() == Some(&step.name))
            .count() as u32;
        self.workflow = Some(wf.id.clone());
        self.step = Some(step.name.clone());
        self.version = version.clone();
        self.outside = None;
        self.stray = None;
        self.last_line.clear();
        self.push(Passage {
            workflow: wf.id.clone(),
            step: Some(step.name.clone()),
            version,
            pass,
            door,
            at,
            ..Default::default()
        });
    }

    /// Goes outside the workflow, doing `doing`.
    fn leave(&mut self, doing: String, at: i64) {
        let Some(wf) = self.workflow.clone() else {
            return;
        };
        self.close(at, Outcome::Left, None);
        self.step = None;
        self.outside = Some(doing.clone());
        self.stray = None;
        self.push(Passage {
            workflow: wf,
            step: None,
            version: self.version.clone(),
            at,
            doing: Some(doing),
            ..Default::default()
        });
    }

    /// Takes in one fact. Returns whether something the interface shows changed.
    pub fn apply(&mut self, scan: &Scan, repo: &str, fact: &Fact, at: i64) -> bool {
        let before = (
            self.workflow.clone(),
            self.step.clone(),
            self.outside.clone(),
            self.subagents.clone(),
            self.passages.len(),
            self.turns.len(),
        );
        let mut visible = false;
        match fact {
            Fact::Message { text } => {
                visible |= self.on_message(scan, repo, text, at);
            }
            Fact::Tool { name, input } => visible |= self.on_tool(scan, repo, name, input, at),
            Fact::SubagentDone { name } => {
                if let Some(i) = self.subagents.iter().position(|s| s == name) {
                    self.subagents.remove(i);
                }
            }
            Fact::Cost(c) => {
                self.cost += c;
                if let Some(p) = self.open() {
                    p.cost += c;
                }
                if let Some(t) = self.turns.last_mut().filter(|t| t.ended.is_none()) {
                    t.cost += c;
                }
                visible = self.workflow.is_some();
            }
            Fact::TurnEnd { end, text } => visible |= self.on_turn_end(scan, *end, text, at),
            Fact::List(progress) => {
                let prev = self
                    .turns
                    .iter()
                    .rev()
                    .skip(1)
                    .find_map(|t| t.checked)
                    .map(|p| p.done)
                    .unwrap_or(0);
                if let Some(t) = self.turns.last_mut() {
                    t.checked = Some(*progress);
                    t.ticked = Some(progress.done > prev);
                    visible = true;
                }
            }
            Fact::TicketLoop { n, max } => {
                if self.ticket_loop != Some((*n, *max)) {
                    self.ticket_loop = Some((*n, *max));
                    visible = self.workflow.is_some();
                }
            }
        }
        let after = (
            self.workflow.clone(),
            self.step.clone(),
            self.outside.clone(),
            self.subagents.clone(),
            self.passages.len(),
            self.turns.len(),
        );
        if before != after {
            visible = true;
        }
        if visible {
            self.updated = at;
            if let Some(since) = self.passages.first().map(|_| self.since).filter(|s| *s > 0) {
                self.ms = (at - since).max(0) as u64;
            }
        }
        visible
    }

    fn on_message(&mut self, scan: &Scan, repo: &str, text: &str, at: i64) -> bool {
        // A prompt run over and over: this message is a turn of it.
        if let Some(wf) = scan
            .workflows
            .iter()
            .filter(|w| w.kind == Kind::Prompt)
            .find(|w| runs_prompt(w, text))
        {
            if self.workflow.as_deref() != Some(wf.id.as_str()) {
                self.close(at, Outcome::Done, None);
                self.workflow = Some(wf.id.clone());
                self.step = None;
                self.outside = None;
                self.since = at;
                self.ms = 0;
                self.cost = 0.0;
                self.version = version_of(wf);
            }
            self.stray = None;
            self.open_turn(at);
            return true;
        }
        if let Some(w) = command_of(text) {
            if let Some((wf, step)) = locate(scan, self, repo, &Sig::Door(w)) {
                let door = Some(format!("/{}", w));
                // The same step typed again is a new pass.
                self.enter(wf, step, door, at);
                return true;
            }
        }
        // No door: a message in the middle of the work. It takes the agent out of the workflow
        // only if its turn then uses tools that signal no step.
        if self.workflow.is_some()
            && (self.step.is_some() || self.outside.is_some() || self.in_loop(scan))
        {
            if self.in_loop(scan) {
                self.open_turn(at);
                return true;
            }
            self.stray = Some(Stray {
                text: clip(text),
                at,
                tools: 0,
            });
        }
        false
    }

    fn open_turn(&mut self, at: i64) {
        if self.turns.last().is_some_and(|t| t.ended.is_none()) {
            return;
        }
        self.next_turn += 1;
        self.turns.push(LoopTurn {
            n: self.next_turn,
            at,
            ..Default::default()
        });
        if self.turns.len() > MAX_HISTORY {
            self.turns.remove(0);
        }
    }

    fn on_tool(&mut self, scan: &Scan, repo: &str, name: &str, input: &Value, at: i64) -> bool {
        let mut sigs: Vec<(Sig, String)> = Vec::new();
        let sub = input["subagent_type"].as_str().filter(|s| !s.is_empty());
        match name {
            n if is_agent_tool(n) => {
                if let Some(s) = sub {
                    sigs.push((Sig::Agent(s), format!("agent:{s}")));
                }
            }
            "Skill" => {
                if let Some(s) = input["skill"].as_str().or(input["command"].as_str()) {
                    sigs.push((Sig::Skill(s), format!("skill:{s}")));
                }
            }
            "SlashCommand" => {
                if let Some(c) = input["command"].as_str().and_then(command_of) {
                    sigs.push((Sig::Door(c), format!("door:/{c}")));
                }
            }
            "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => {
                if let Some(p) = input["file_path"].as_str().or(input["path"].as_str()) {
                    sigs.push((Sig::File(p), format!("file:{}", p.replace('\\', "/"))));
                }
            }
            _ => {}
        }
        let mut visible = false;
        if let Some(s) = sub.filter(|_| is_agent_tool(name)) {
            if !self.subagents.iter().any(|x| x == s) {
                self.subagents.push(s.to_string());
            }
            visible = true;
        }
        let mut signalled = false;
        for (sig, did) in &sigs {
            let Some((wf, step)) = locate(scan, self, repo, sig) else {
                continue;
            };
            signalled = true;
            self.stray = None;
            let here = self.workflow.as_deref() == Some(wf.id.as_str())
                && self.step.as_deref() == Some(step.name.as_str());
            if !here {
                self.enter(wf, step, None, at);
                visible = true;
            }
            if let Some(p) = self.open() {
                push_did(p, did.clone());
            }
        }
        if !signalled {
            if let Some(s) = self.stray.as_mut() {
                s.tools += 1;
            }
            // What a step's sub-agent or skill did is said even when nothing moved.
            if let (Some(p), Some((_, did))) = (self.open(), sigs.first()) {
                push_did(p, did.clone());
            }
        }
        visible
    }

    fn on_turn_end(&mut self, scan: &Scan, end: End, text: &str, at: i64) -> bool {
        let mut visible = false;
        let line = clip(text);
        if !line.is_empty() {
            self.last_line = line;
        }
        let outcome = match end {
            End::Finished => None,
            End::Failed => Some(Outcome::Failed),
            End::Stopped => Some(Outcome::Stopped),
        };
        if let Some(o) = outcome {
            if let Some(p) = self.open() {
                if p.outcome == Outcome::Running {
                    p.outcome = o;
                }
                visible = true;
            }
        }
        if let Some(t) = self.turns.last_mut().filter(|t| t.ended.is_none()) {
            t.ended = Some(at);
            t.ms = (at - t.at).max(0) as u64;
            t.outcome = outcome.unwrap_or(Outcome::Done);
            visible = true;
        }
        if let Some(stray) = self.stray.take() {
            if stray.tools > 0 && self.step.is_some() && !self.in_loop(scan) {
                self.leave(stray.text, at);
                visible = true;
            }
        }
        visible
    }

    /// Whether the workflow changed under the step in course (choice 154).
    pub fn behind(&self, scan: &Scan) -> bool {
        let (Some(id), Some(_)) = (&self.workflow, &self.step) else {
            return false;
        };
        scan.workflows
            .iter()
            .find(|w| &w.id == id)
            .is_none_or(|w| version_of(w) != self.version)
    }

    pub fn view(&self, project_id: &str, scan: &Scan) -> TrackView {
        TrackView {
            track: self.clone(),
            project_id: project_id.to_string(),
            behind: self.behind(scan),
        }
    }
}

// ---------- kept with the agent, told to the window ----------

/// What is held in memory: the last reading of each project's workflows, and the tracks of its
/// agents (loaded from their file the first time the project is met).
#[derive(Default)]
pub struct Store {
    scans: HashMap<String, std::sync::Arc<Scan>>,
    tracks: HashMap<String, HashMap<String, AgentTrack>>,
    pending: HashMap<String, HashMap<String, Pending>>,
}

/// The repository of a project of several an agent works in (its folder under it); empty for the
/// root, or for an agent whose folder is elsewhere (a worktree).
fn repo_of(root: &str, repos: &[String], cwd: &str) -> String {
    let (root, cwd) = (root.replace('\\', "/"), cwd.replace('\\', "/"));
    let rel = cwd
        .strip_prefix(root.trim_end_matches('/'))
        .unwrap_or("")
        .trim_start_matches('/');
    repos
        .iter()
        .find(|r| rel == r.as_str() || rel.starts_with(&format!("{r}/")))
        .cloned()
        .unwrap_or_default()
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// The last reading of a project's workflows (the file `refresh_workflows` keeps, or the last
    /// one it made), as the user corrected it (ESC-195): none read yet is no workflow.
    pub(crate) fn tracking_scan(&self, project_id: &str) -> std::sync::Arc<Scan> {
        if let Some(s) = self.wf_tracks.lock().scans.get(project_id) {
            return s.clone();
        }
        self.read_if_never_read(project_id);
        let scan =
            std::sync::Arc::new(self.corrected_scan(project_id, &self.stored_scan(project_id)));
        self.wf_tracks
            .lock()
            .scans
            .entry(project_id.to_string())
            .or_insert(scan)
            .clone()
    }

    /// A project never read (added while the app runs, ESC-194): read now and kept as « Relire »
    /// would keep it (its reading, the files of its workflows), so that the first door of its
    /// agents is not lost.
    fn read_if_never_read(&self, project_id: &str) {
        let file = self.data.workflows_file(project_id);
        if file.exists() {
            return;
        }
        let Ok(p) = self.project(project_id) else {
            return;
        };
        // « Pas de workflow pour ce projet » (ESC-309): nothing read.
        if !p.workflows_enabled {
            return;
        }
        let scan = crate::workflow::read(
            std::path::Path::new(&p.path),
            &p.repos,
            &crate::workflow::Roots::of_user(),
            &[],
        );
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec_pretty(&scan) {
            let _ = crate::paths::write_atomic(&file, &json);
        }
        // Into the files of the workflows (ESC-195), as `refresh_workflows` puts them: what is followed.
        let places = crate::workflowfiles::places(&p, &self.data);
        let folder = crate::workflowfiles::load_all(&places);
        // Those the user removed are not written again (ESC-309).
        let file = crate::workflowfiles::removals_file(&self.data, project_id);
        let before = crate::workflowfiles::Removals::load(&file);
        let mut removals = before.clone();
        crate::workflowfiles::merge_reading_all(
            &places,
            &folder,
            &scan.workflows,
            std::path::Path::new(&p.path).is_dir(),
            &mut removals,
        );
        removals.save(&file, &before);
    }

    /// The tracks of the project's agents, read from their file the first time.
    fn load_tracks(&self, st: &mut Store, project_id: &str) {
        if st.tracks.contains_key(project_id) {
            return;
        }
        let tracks: HashMap<String, AgentTrack> =
            std::fs::read(self.data.workflow_tracks_file(project_id))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
        st.tracks.insert(project_id.to_string(), tracks);
    }

    fn save_tracks(&self, st: &Store, project_id: &str) {
        let Some(tracks) = st.tracks.get(project_id) else {
            return;
        };
        let file = self.data.workflow_tracks_file(project_id);
        let write = || -> anyhow::Result<()> {
            if let Some(dir) = file.parent() {
                std::fs::create_dir_all(dir)?;
            }
            crate::paths::write_atomic(&file, &serde_json::to_vec(tracks)?)?;
            Ok(())
        };
        if let Err(e) = write() {
            log::warn!("workflow tracks of {project_id}: {e:#}");
        }
    }

    /// What an agent just did, compared to the steps of its project's workflows (called by `apply`
    /// with the operations, the cost and the end of the turn it reports). Told to the window only
    /// when what it shows changed.
    pub(crate) fn track_workflow(
        &self,
        id: &str,
        project_id: &str,
        ops: &[ConvOp],
        cost: f64,
        end: Option<&crate::board::TurnEnd>,
    ) {
        if ops.is_empty() && cost == 0.0 && end.is_none() {
            return;
        }
        let scan = self.tracking_scan(project_id);
        if scan.workflows.is_empty() {
            return;
        }
        let Ok(h) = self.agent(id) else { return };
        let (engine, view, cwd) = {
            let rt = h.lock();
            (
                rt.meta.engine,
                rt.tui.is_some() || rt.attach.is_some(),
                rt.meta.cwd.clone(),
            )
        };
        let (root, repos) = self
            .project(project_id)
            .map(|p| (p.path, p.repos))
            .unwrap_or_default();
        let repo = repo_of(&root, &repos, &cwd);
        let ticket_loop = self
            .doing_ticket_of(id)
            .and_then(|t| self.ticket(&t).ok())
            .map(|t| (t.iteration, t.max_loops))
            .filter(|(n, _)| *n > 0);
        let at = crate::model::now_ms();
        let finished = end.is_some();
        let (mut changed, list) = {
            let mut st = self.wf_tracks.lock();
            self.load_tracks(&mut st, project_id);
            let mut facts = {
                let pend = st.pending.entry(id.to_string()).or_default();
                facts_of_ops(ops, pend)
            };
            if cost > 0.0 {
                facts.push(Fact::Cost(cost));
            }
            if let Some(e) = end {
                use crate::board::TurnEnd;
                let (end, text) = match e {
                    TurnEnd::Finished(t) => (End::Finished, t.clone()),
                    TurnEnd::Error(t) => (End::Failed, t.clone()),
                    TurnEnd::Interrupted | TurnEnd::Limited | TurnEnd::SignIn => {
                        (End::Stopped, String::new())
                    }
                };
                facts.push(Fact::TurnEnd { end, text });
                st.pending.remove(id);
            }
            if facts.is_empty() {
                return;
            }
            let track = st
                .tracks
                .get_mut(project_id)
                .map(|m| {
                    m.entry(id.to_string())
                        .or_insert_with(|| AgentTrack::new(id, engine, view))
                })
                .expect("loaded");
            track.support = support_of(engine, view);
            let mut changed = false;
            if let Some((n, max)) = ticket_loop {
                changed |= track.apply(&scan, &repo, &Fact::TicketLoop { n, max }, at);
            }
            for f in &facts {
                changed |= track.apply(&scan, &repo, f, at);
            }
            // After a turn of a prompt run over and over: what its list says now.
            let list = if finished {
                scan.workflows
                    .iter()
                    .find(|w| Some(&w.id) == track.workflow.as_ref())
                    .and_then(|w| w.prompt_loop.as_ref())
                    .and_then(|p| p.list.as_ref())
                    .map(|l| l.path.clone())
            } else {
                None
            };
            let followed = track.workflow.is_some();
            if !followed {
                // An agent that met no door has no track to keep.
                st.tracks.get_mut(project_id).map(|m| m.remove(id));
            }
            (changed, list)
        };
        if let Some(path) = list {
            if let Some(p) = crate::workflow::list_progress(
                std::path::Path::new(&root),
                &path,
                &crate::workflow::Roots::of_user(),
            ) {
                let mut st = self.wf_tracks.lock();
                if let Some(t) = st.tracks.get_mut(project_id).and_then(|m| m.get_mut(id)) {
                    changed |= t.apply(&scan, &repo, &Fact::List(p), at);
                }
            }
        }
        if changed {
            let st = self.wf_tracks.lock();
            self.save_tracks(&st, project_id);
            if let Some(t) = st.tracks.get(project_id).and_then(|m| m.get(id)) {
                self.hub.emit(crate::model::UiEvent::WorkflowTrack {
                    track: t.view(project_id, &scan),
                });
            }
        }
    }

    /// The tracks of a project's agents, as the window shows them.
    pub fn workflow_tracks(&self, project_id: &str) -> Vec<TrackView> {
        let scan = self.tracking_scan(project_id);
        let mut st = self.wf_tracks.lock();
        self.load_tracks(&mut st, project_id);
        let mut out: Vec<TrackView> = st.tracks[project_id]
            .values()
            .map(|t| t.view(project_id, &scan))
            .collect();
        out.sort_by(|a, b| a.track.agent_id.cmp(&b.track.agent_id));
        out
    }

    /// The reading of a project's workflows was made: kept for the tracking, and when it changed,
    /// every agent that follows one is told whether its step is now behind it (choice 154).
    pub(crate) fn workflow_read(&self, project_id: &str, scan: &Scan, changed: bool) {
        let scan = self.corrected_scan(project_id, scan);
        self.wf_tracks
            .lock()
            .scans
            .insert(project_id.to_string(), std::sync::Arc::new(scan));
        if changed {
            for v in self.workflow_tracks(project_id) {
                if v.track.workflow.is_some() {
                    self.hub
                        .emit(crate::model::UiEvent::WorkflowTrack { track: v });
                }
            }
        }
    }

    /// The reading kept in the data folder, as read.
    fn stored_scan(&self, project_id: &str) -> Scan {
        // The workflows of the project's files (ESC-195), beside this machine's last reading.
        self.workflows_of(project_id)
    }

    /// `scan` with the project's corrections (`workflowfix`).
    fn corrected_scan(&self, project_id: &str, scan: &Scan) -> Scan {
        let fixes = self
            .project(project_id)
            .map(|p| p.workflow_fixes)
            .unwrap_or_default();
        crate::workflowfix::corrected(scan, &fixes)
    }

    /// The corrections of a project's workflows changed (Réglages › Projets › Workflow, its
    /// `.escouade.yml`): the tracking takes them at once, its agents told as for a new reading.
    /// The history kept is not rewritten: its passages keep the names of their steps.
    pub(crate) fn workflow_fixes_changed(&self, project_id: &str) {
        self.wf_tracks.lock().scans.remove(project_id);
        let scan = self.stored_scan(project_id);
        self.workflow_read(project_id, &scan, true);
    }

    /// An agent is deleted: its history goes with it.
    pub(crate) fn forget_workflow_track(&self, project_id: &str, id: &str) {
        let mut st = self.wf_tracks.lock();
        self.load_tracks(&mut st, project_id);
        st.pending.remove(id);
        if st
            .tracks
            .get_mut(project_id)
            .is_some_and(|m| m.remove(id).is_some())
        {
            self.save_tracks(&st, project_id);
        }
    }
}
