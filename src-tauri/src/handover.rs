//! « Continuer sur <harness> » (ESC-88): an agent goes on on another harness, by a handover. A
//! session does not pass from one harness to another (their formats differ): the agent keeps its
//! conversation in Escouade, its folder and its branch, and a new process of the other harness
//! starts a new session, whose first message is a handover file Escouade writes from its own
//! journal (`conv.rs`), never asking the harness it leaves (its quota may be spent).
//!
//! What may be handed over to: the project's fallback harnesses (`accounts::fallback_engines`),
//! written and with an account serving the project, but the one the agent left while its quota
//! has not reset (`AgentMeta::handed_from`). Automatic only with « Continuer sur le harness
//! suivant… » (`Settings::handover_on_limit`), when every account of the agent's harness serving
//! its project is spent; never back by itself.

use crate::accounts;
use crate::agent::Effects;
use crate::board::{self, TurnEnd};
use crate::core::Core;
use crate::engine::{self, Engine};
use crate::i18n::Lang;
use crate::model::{AgentStatus, EngineId, HandedFrom, Settings};
use crate::pace::{self, Standing};
use crate::plan::{PlanState, TaskStatus};
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::Runtime;

/// Escouade's modes, from the one that allows the least to the one that allows the most.
const MODES: [&str; 5] = [
    "plan",
    "default",
    "acceptEdits",
    "auto",
    "bypassPermissions",
];

fn rank(mode: &str) -> Option<usize> {
    MODES.iter().position(|m| *m == mode)
}

/// The mode an agent in `current` runs in on a harness that offers `offered`: the same when it is
/// there, else the closest one that never allows more. None when every mode offered allows more.
/// A mode Escouade does not know is taken as « Demander » (`default`).
pub fn closest_mode(current: &str, offered: &[&str]) -> Option<String> {
    if offered.contains(&current) {
        return Some(current.to_string());
    }
    let at = rank(current).unwrap_or(1);
    offered
        .iter()
        .filter_map(|m| rank(m).map(|r| (r, *m)))
        .filter(|(r, _)| *r <= at)
        .max_by_key(|(r, _)| *r)
        .map(|(_, m)| m.to_string())
}

/// The modes `engine` takes, `auto` only where its model takes it (`auto`: `Core::takes_auto`,
/// what the `claude` of its account told).
pub fn modes_of(engine: &dyn Engine, auto: bool) -> Vec<&'static str> {
    engine
        .caps()
        .modes
        .iter()
        .map(|m| m.id)
        .filter(|m| *m != "auto" || auto)
        .collect()
}

/// The model an agent handed over to `engine` runs on: « Modèle de passation » when it names one
/// of its models, else its best.
pub fn model_for(settings: &Settings, engine: &dyn Engine) -> String {
    let set = settings
        .handover_models
        .get(&engine.id())
        .map(|m| m.trim())
        .filter(|m| engine.models().iter().any(|d| d.id == *m));
    match set {
        Some(m) => m.to_string(),
        None => engine.handover_model(),
    }
}

/// What the handover file may weigh, in characters, for a model of `window` tokens: a quarter of
/// its window, at about three characters a token (French and code take more tokens than English).
pub fn budget(window: u64) -> usize {
    (window as usize / 4).saturating_mul(3).max(1_000)
}

/// What git says of the agent's folder: its uncommitted changes (`git status --short`), their
/// size (`git diff --stat HEAD`), and the commits of its branch since its base.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    pub status: String,
    pub stat: String,
    pub commits: String,
}

/// What the handover file is made of.
pub struct Dossier<'a> {
    pub lang: Lang,
    /// The harness left, as the interface names it.
    pub from: &'a str,
    /// It left because its quota was spent (else, it was asked).
    pub limited: bool,
    /// The agent's conversation, as Escouade keeps it.
    pub items: &'a [Value],
    pub plan: Option<&'a PlanState>,
    /// What the agent last said it did (`report_progress`).
    pub progress: Option<&'a str>,
    pub changes: &'a Changes,
    /// Its ticket (« ESC-88 — Titre »), whose protocol its added instructions give.
    pub ticket: Option<&'a str>,
    /// What it may weigh, in characters (`budget`).
    pub budget: usize,
}

/// `text` cut to `max` characters at most, « … » marking the cut.
fn clip(text: &str, max: usize) -> String {
    // The spaces that start a line stay (`git status --short` begins with them).
    let text = text.trim_start_matches(['\n', '\r']).trim_end();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", kept.trim_end())
}

/// Whether the item is a handover file sent before (a way back): not one of the user's requests.
fn is_dossier(item: &Value) -> bool {
    item["kind"] == "user" && item["origin"] == "handover"
}

/// The tool call as one line: its name and what it acts on.
fn call_line(item: &Value) -> String {
    let input = &item["input"];
    let on = [
        "command",
        "file_path",
        "path",
        "pattern",
        "url",
        "description",
    ]
    .iter()
    .find_map(|k| input[*k].as_str())
    .unwrap_or_default();
    let name = item["name"].as_str().unwrap_or("?");
    if on.is_empty() {
        name.to_string()
    } else {
        format!("{name} {}", clip(on, 200))
    }
}

/// The user's requests: the first one whole (to its share), then the latest ones that fit, in
/// their order, the ones left out counted.
fn requests(d: &Dossier, max: usize) -> String {
    let users: Vec<&str> = d
        .items
        .iter()
        .filter(|i| i["kind"] == "user" && !is_dossier(i))
        .filter_map(|i| i["text"].as_str())
        .filter(|t| !t.trim().is_empty())
        .collect();
    let Some((first, rest)) = users.split_first() else {
        return String::new();
    };
    let lang = d.lang;
    let mut out = tr_in!(lang, "## Demande d’origine\n\n", "## Original request\n\n");
    out.push_str(&clip(first, max / 3));
    out.push_str("\n\n");
    let mut room = max.saturating_sub(out.chars().count());
    let mut kept: Vec<String> = Vec::new();
    for text in rest.iter().rev() {
        let line = format!("- {}\n", clip(text, 2_000).replace('\n', "\n  "));
        let n = line.chars().count();
        if n > room {
            break;
        }
        room -= n;
        kept.push(line);
    }
    if !rest.is_empty() {
        out.push_str(&tr_in!(
            lang,
            "## Messages suivants de l’utilisateur\n\n",
            "## The user’s later messages\n\n"
        ));
        let left = rest.len() - kept.len();
        if left > 0 {
            out.push_str(&tr_in!(
                lang,
                "- ({left} message(s) plus ancien(s) omis)\n",
                "- ({left} older message(s) left out)\n"
            ));
        }
        for line in kept.iter().rev() {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// What was done: the agent's latest messages (its own, not its subagents'), what it last said it
/// did, the tasks of its plan.
fn work(d: &Dossier, max: usize) -> String {
    let lang = d.lang;
    let mut out = String::new();
    if let Some(p) = d.progress.filter(|p| !p.trim().is_empty()) {
        out.push_str(&tr_in!(
            lang,
            "Avancement annoncé : {p}\n\n",
            "Progress reported: {p}\n\n"
        ));
    }
    if let Some(plan) = d.plan.filter(|p| !p.tasks.is_empty()) {
        out.push_str(&tr_in!(lang, "Tâches du plan :\n", "Plan tasks:\n"));
        for t in &plan.tasks {
            let mark = match t.status {
                TaskStatus::Done => "[x]",
                TaskStatus::InProgress => "[~]",
                TaskStatus::Pending => "[ ]",
            };
            out.push_str(&format!("- {mark} {}\n", clip(&t.title, 300)));
        }
        out.push('\n');
    }
    let said: Vec<&str> = d
        .items
        .iter()
        .filter(|i| i["kind"] == "text" && i["parent"].is_null())
        .filter_map(|i| i["text"].as_str())
        .filter(|t| !t.trim().is_empty())
        .collect();
    if !said.is_empty() {
        out.push_str(&tr_in!(
            lang,
            "Derniers messages de l’agent (du plus ancien au plus récent) :\n\n",
            "The agent’s latest messages (oldest first):\n\n"
        ));
        let mut room = max.saturating_sub(out.chars().count());
        let mut kept = Vec::new();
        for text in said.iter().rev().take(8) {
            let block = format!("> {}\n\n", clip(text, 3_000).replace('\n', "\n> "));
            let n = block.chars().count();
            if n > room {
                // The latest one always says something, cut to what is left.
                if kept.is_empty() && room > 40 {
                    kept.push(format!(
                        "> {}\n\n",
                        clip(text, room - 10).replace('\n', "\n> ")
                    ));
                }
                break;
            }
            room -= n;
            kept.push(block);
        }
        for block in kept.iter().rev() {
            out.push_str(block);
        }
    }
    if out.is_empty() {
        return out;
    }
    tr_in!(lang, "## Travail fait\n\n{out}", "## Work done\n\n{out}")
}

/// The files changed in the folder, as git tells them.
fn files(d: &Dossier, max: usize) -> String {
    let lang = d.lang;
    let c = d.changes;
    let mut out = tr_in!(lang, "## Fichiers modifiés\n\n", "## Changed files\n\n");
    if c.status.trim().is_empty() && c.commits.trim().is_empty() {
        out.push_str(&tr_in!(
            lang,
            "Aucune modification dans le dossier de travail.\n\n",
            "No change in the working folder.\n\n"
        ));
        return out;
    }
    let each = max / 3;
    if !c.commits.trim().is_empty() {
        out.push_str(&tr_in!(
            lang,
            "Commits de la branche :\n```\n{commits}\n```\n",
            "Commits of the branch:\n```\n{commits}\n```\n",
            commits = clip(&c.commits, each)
        ));
    }
    if !c.status.trim().is_empty() {
        out.push_str(&tr_in!(
            lang,
            "Non commités (git status --short) :\n```\n{status}\n```\n",
            "Not committed (git status --short):\n```\n{status}\n```\n",
            status = clip(&c.status, each)
        ));
    }
    if !c.stat.trim().is_empty() {
        out.push_str(&format!("```\n{}\n```\n", clip(&c.stat, each)));
    }
    out.push('\n');
    out
}

/// The last turn when it did not end well, and where it was cut; a question or a permission left
/// unanswered.
fn cut(d: &Dossier) -> String {
    let lang = d.lang;
    let mut out = String::new();
    // The items of the last turn, and its end: none when work came after the last end (a turn
    // its process left before it ended). Notes after an end are no turn.
    let work = |i: &Value| matches!(i["kind"].as_str(), Some("user" | "text" | "tool"));
    let turn = |i: &Value| i["kind"] == "turn";
    let (last, end) = match d.items.iter().rposition(turn) {
        Some(at) if !d.items[at + 1..].iter().any(work) => {
            let from = d.items[..at].iter().rposition(turn).map_or(0, |p| p + 1);
            (&d.items[from..at], Some(&d.items[at]))
        }
        Some(at) => (&d.items[at + 1..], None),
        None => (d.items, None),
    };
    let broken = match end {
        None => last.iter().any(work),
        Some(t) => t["limited"] == true || t["interrupted"] == true || t["isError"] == true,
    };
    if broken {
        out.push_str(&tr_in!(
            lang,
            "## Dernier tour coupé\n\n",
            "## Last turn cut\n\n"
        ));
        let why = match end {
            Some(t) if t["limited"] == true => {
                tr_in!(lang, "par la limite d’usage", "by the usage limit")
            }
            Some(t) if t["interrupted"] == true => tr_in!(lang, "interrompu", "interrupted"),
            Some(_) => tr_in!(lang, "par une erreur", "by an error"),
            None => tr_in!(lang, "avant sa fin", "before its end"),
        };
        out.push_str(&tr_in!(lang, "Coupé {why}.", "Cut {why}."));
        if let Some(tool) = last.iter().rev().find(|i| i["kind"] == "tool") {
            let status = tool["status"].as_str().unwrap_or_default();
            out.push_str(&tr_in!(
                lang,
                " Dernier outil : {call} ({status}).",
                " Last tool: {call} ({status}).",
                call = call_line(tool)
            ));
        }
        out.push_str("\n\n");
    }
    let pending: Vec<String> = d
        .items
        .iter()
        .filter(|i| i["cancelled"] != true)
        .filter_map(|i| match i["kind"].as_str() {
            Some("permission") if i["decision"].is_null() => Some(tr_in!(
                lang,
                "- Permission demandée pour {tool} : {on}",
                "- Permission asked for {tool}: {on}",
                tool = i["toolName"].as_str().unwrap_or("?"),
                on = clip(&call_line(&json!({ "name": "", "input": i["input"] })), 300)
            )),
            Some("question") if i["answers"].is_null() => {
                let q: Vec<&str> = i["questions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|q| q["question"].as_str())
                    .collect();
                Some(tr_in!(
                    lang,
                    "- Question posée à l’utilisateur : {q}",
                    "- Question asked to the user: {q}",
                    q = clip(&q.join(" / "), 500)
                ))
            }
            _ => None,
        })
        .collect();
    if !pending.is_empty() {
        out.push_str(&tr_in!(
            lang,
            "## En attente de l’utilisateur (sans réponse)\n\n",
            "## Waiting for the user (unanswered)\n\n"
        ));
        out.push_str(&pending.join("\n"));
        out.push_str("\n\n");
    }
    out
}

/// The handover file: what the new harness is told first, bounded by `d.budget`.
pub fn dossier(d: &Dossier) -> String {
    let lang = d.lang;
    let from = d.from;
    let why = if d.limited {
        tr_in!(
            lang,
            "son quota {from} est épuisé",
            "its {from} quota is spent"
        )
    } else {
        tr_in!(lang, "l’utilisateur l’a demandé", "the user asked for it")
    };
    let mut out = tr_in!(
        lang,
        "# Passation : suite d’un travail commencé sur {from}\n\nTu reprends le travail d’un agent qui tournait sur {from} ({why}). Tu es dans le même dossier de travail, sur la même branche ; ce dossier de passation résume la conversation jusqu’ici. Continue là où il s’est arrêté, sans refaire ce qui est fait ; vérifie l’état des fichiers avant de les modifier.\n\n",
        "# Handover: going on with work begun on {from}\n\nYou take over the work of an agent that ran on {from} ({why}). You are in the same working folder, on the same branch; this handover file sums up the conversation so far. Go on where it stopped, without redoing what is done; check the files before you change them.\n\n"
    );
    if let Some(ticket) = d.ticket {
        out.push_str(&tr_in!(
            lang,
            "Ce travail est le ticket {ticket}. Son protocole (critères d’acceptation, boucle, bilan) est dans tes consignes : suis-le comme avant.\n\n",
            "This work is the ticket {ticket}. Its protocol (acceptance criteria, loop, report) is in your instructions: follow it as before.\n\n"
        ));
    }
    let room = d.budget.saturating_sub(out.chars().count());
    // The end of the work weighs the most: what was cut, then the files, the work, the requests.
    let cut = cut(d);
    let files = files(d, room / 5);
    let rest = room.saturating_sub(cut.chars().count() + files.chars().count());
    let requests = requests(d, rest * 2 / 5);
    let work = work(d, rest.saturating_sub(requests.chars().count()));
    for part in [requests, work, files, cut] {
        out.push_str(&part);
    }
    out.push_str(&tr_in!(
        lang,
        "Commence par dire en une phrase où en est le travail, puis continue.",
        "Start by saying in one sentence where the work stands, then go on."
    ));
    clip(&out, d.budget)
}

/// Which way a relay goes (« Vue Cursor »): Cursor keeps the conversations of its interface apart
/// from its sessions over ACP, so that each is told what was said in the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relay {
    /// Into its interface, as the view opens: the conversation so far, in Escouade.
    ToInterface,
    /// Into its session, back in Escouade's view: what was said in its interface meanwhile.
    ToSession,
}

/// How much a relay may weigh, in characters.
pub const RELAY_BUDGET: usize = 24_000;

/// The exchange of `items` in its order, the user's messages and the agent's (its own, not its
/// subagents'), its tools as one line each; the latest that fit in `max`, the ones left out
/// counted. Empty when nobody said anything.
fn exchange(lang: Lang, items: &[Value], max: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut tools: Vec<String> = Vec::new();
    let flush = |tools: &mut Vec<String>, lines: &mut Vec<String>| {
        if !tools.is_empty() {
            let line = tr_in!(
                lang,
                "  (outils : {t})\n",
                "  (tools: {t})\n",
                t = tools.join(" ; ")
            );
            lines.push(line);
            tools.clear();
        }
    };
    for i in items.iter().filter(|i| i["parent"].is_null()) {
        let text = i["text"].as_str().unwrap_or_default().trim();
        match i["kind"].as_str() {
            Some("user") if !text.is_empty() && !is_dossier(i) => {
                flush(&mut tools, &mut lines);
                let said = clip(text, 4_000).replace('\n', "\n  ");
                lines.push(tr_in!(lang, "- Utilisateur : {said}\n", "- User: {said}\n"));
            }
            Some("text") if !text.is_empty() => {
                flush(&mut tools, &mut lines);
                let said = clip(text, 4_000).replace('\n', "\n  ");
                lines.push(tr_in!(lang, "- Agent : {said}\n", "- Agent: {said}\n"));
            }
            Some("tool") => tools.push(call_line(i)),
            _ => {}
        }
    }
    flush(&mut tools, &mut lines);
    if !lines.iter().any(|l| !l.starts_with("  ")) {
        return String::new();
    }
    let mut room = max;
    let mut kept: Vec<&String> = Vec::new();
    for line in lines.iter().rev() {
        let n = line.chars().count();
        if n > room {
            break;
        }
        room -= n;
        kept.push(line);
    }
    let mut out = String::new();
    let left = lines.len() - kept.len();
    if left > 0 {
        out.push_str(&tr_in!(
            lang,
            "- ({left} ligne(s) plus ancienne(s) omise(s))\n",
            "- ({left} older line(s) left out)\n"
        ));
    }
    for line in kept.iter().rev() {
        out.push_str(line);
    }
    out
}

/// What Cursor is told of the other side of its conversation (`Relay`), from the agent's
/// conversation `items` (for its interface, all of it; for its session, what came since it went to
/// the interface) and the tasks of its plan; None when nothing was said there.
pub fn relay(
    lang: Lang,
    way: Relay,
    items: &[Value],
    plan: Option<&PlanState>,
    max: usize,
) -> Option<String> {
    let mut out = match way {
        Relay::ToInterface => tr_in!(
            lang,
            "# Conversation d’avant, dans Escouade\n\nTu continues le travail d’un agent que l’utilisateur pilotait dans Escouade, par une autre session de Cursor : cette conversation n’en a pas l’historique. Voici ce qui s’y est dit, du plus ancien au plus récent. Continue à partir de là, sans refaire ce qui est fait ; vérifie l’état des fichiers avant de les modifier.\n\n",
            "# The conversation before, in Escouade\n\nYou go on with the work of an agent the user drove in Escouade, through another session of Cursor: this conversation does not have its history. Here is what was said there, oldest first. Go on from there, without redoing what is done; check the files before you change them.\n\n"
        ),
        Relay::ToSession => tr_in!(
            lang,
            "# Ce qui s’est dit dans l’interface de Cursor\n\nEntre-temps, l’utilisateur a continué ce travail dans l’interface de Cursor, une autre conversation dont cette session n’a pas l’historique. Voici ce qui s’y est dit, du plus ancien au plus récent : tiens-en compte pour la suite.\n\n",
            "# What was said in Cursor’s interface\n\nMeanwhile, the user went on with this work in Cursor’s interface, another conversation whose history this session does not have. Here is what was said there, oldest first: keep it in mind from now on.\n\n"
        ),
    };
    if way == Relay::ToInterface {
        if let Some(plan) = plan.filter(|p| !p.tasks.is_empty()) {
            out.push_str(&tr_in!(lang, "Tâches du plan :\n", "Plan tasks:\n"));
            for t in &plan.tasks {
                let mark = match t.status {
                    TaskStatus::Done => "[x]",
                    TaskStatus::InProgress => "[~]",
                    TaskStatus::Pending => "[ ]",
                };
                out.push_str(&format!("- {mark} {}\n", clip(&t.title, 300)));
            }
            out.push('\n');
        }
    }
    let said = exchange(lang, items, max.saturating_sub(out.chars().count()));
    if said.is_empty() {
        return None;
    }
    out.push_str(&said);
    Some(clip(&out, max))
}

/// Why no handover goes to `engine` (`name`): no account of it serves the project.
fn not_served(lang: Lang, name: &str) -> anyhow::Error {
    anyhow!(tr_in!(
        lang,
        "Aucun compte {name} actif ne sert le projet de cet agent.",
        "No active {name} account serves this agent’s project."
    ))
}

/// What the conversation shows at a handover: « Suite sur Codex · quota Claude Code épuisé
/// jusqu’à 18:00 » (the window words it from these).
fn separator(from: EngineId, to: EngineId, limited: bool, until: Option<i64>) -> Value {
    json!({
        "kind": "handover", "id": crate::model::new_id(), "ts": crate::model::now_ms(),
        "from": from, "to": to, "limited": limited, "until": until,
    })
}

impl<R: Runtime> Core<R> {
    /// How the account `id` stands for an agent now: out when it may not take one (not signed in,
    /// an agent of it waiting for its reset, a pause after a usage limit, out for its quota:
    /// `pace`, as `account_to_switch_to` has it), else how its quota stands (`accounts::standing`
    /// under « Pause au-delà du quota »).
    fn account_standing(&self, id: &str) -> Standing {
        let threshold = board::quota_threshold(self.settings.read().quota_pause);
        let usage = self.usage.lock().accounts.clone();
        let hold = self.hold.lock().clone();
        let now = self.pause_now();
        let usable = usage
            .iter()
            .find(|u| u.id == id)
            .is_none_or(|u| u.connected)
            && !self.quota_paused().contains_key(id)
            && board::account_pause(&usage, id, threshold, &hold, now).is_none();
        match usable {
            true => accounts::standing(&usage, id, threshold, now),
            false => Standing::Out,
        }
    }

    /// Whether the account `id` may take an agent now (`account_standing`).
    pub(crate) fn account_usable(&self, id: &str) -> bool {
        self.account_standing(id) != Standing::Out
    }

    /// The accounts of `engine` serving `project` that may take an agent now, in the settings'
    /// order, `spent` (an account just stopped by the limit) left out.
    fn usable_accounts(&self, engine: EngineId, project: &str, spent: Option<&str>) -> Vec<String> {
        let all = self.settings.read().clone();
        let settings = self.project_accounts(&all, engine, project);
        settings
            .accounts
            .iter()
            .filter(|a| a.active && Some(a.id.as_str()) != spent)
            .map(|a| a.id.clone())
            .filter(|id| self.account_usable(id))
            .collect()
    }

    /// Whether the agent's harness is spent for its project: none of its accounts serving it may
    /// take the agent (its own just stopped by the limit).
    fn engine_spent(&self, id: &str) -> Result<bool> {
        let (engine, project, account) = {
            let h = self.agent(id)?;
            let rt = h.lock();
            (
                rt.meta.engine,
                rt.meta.project_id.clone(),
                rt.meta.account.clone(),
            )
        };
        let account = accounts::get(&self.settings.read(), &account).id;
        Ok(self
            .usable_accounts(engine, &project, Some(&account))
            .is_empty())
    }

    /// The harnesses the agent may be handed over to now, in its project's order (« Continuer
    /// sur <harness> »): its project's fallback harnesses but its own, written, found, with an
    /// active account serving the project; the one it left not before its quota resets.
    pub fn handover_targets(&self, id: &str) -> Result<Vec<EngineId>> {
        let h = self.agent(id)?;
        let (engine, project, left) = {
            let rt = h.lock();
            (
                rt.meta.engine,
                rt.meta.project_id.clone(),
                rt.meta.handed_from.clone(),
            )
        };
        let default = self
            .project(&project)
            .map(|p| p.engine())
            .unwrap_or_default();
        let settings = self.settings.read().clone();
        let now = self.pause_now();
        Ok(accounts::fallback_engines(&settings, default, &project)
            .into_iter()
            .filter(|e| *e != engine && engine::written(*e))
            .filter(|e| {
                !left
                    .as_ref()
                    .is_some_and(|l| l.engine == *e && l.until.is_some_and(|t| t > now))
            })
            .filter(|e| self.engine_found_for(*e, &project))
            .collect())
    }

    /// Whether the program of `engine` is found for the account an agent of `project` would go to.
    fn engine_found_for(&self, engine: EngineId, project: &str) -> bool {
        let all = self.settings.read().clone();
        let settings = self.project_accounts(&all, engine, project);
        let Some(first) = settings.accounts.iter().find(|a| a.active) else {
            return false;
        };
        accounts::program(first, &all).is_some()
    }

    /// The account an agent of `project` handed over to `engine` goes to: the project's preferred
    /// one when it is of that harness, else the first of the best usable ones, else the first active one.
    fn handover_account(&self, engine: EngineId, project: &str) -> Option<String> {
        let all = self.settings.read().clone();
        let settings = self.project_accounts(&all, engine, project);
        if !settings.accounts.iter().any(|a| a.active) {
            return None;
        }
        let preferred = self
            .project(project)
            .ok()
            .and_then(|p| accounts::preferred(&settings, &p.account).map(str::to_string));
        accounts::pick_ranked(&settings, preferred.as_deref(), |id| {
            self.account_standing(id)
        })
    }

    /// Whether an agent of `project` may go on its own to `engine` at the usage limit: the
    /// account it would go to there is not one a private project shares (`accounts::may_switch_to`).
    fn may_hand_over_to(&self, engine: EngineId, project: &str) -> bool {
        let Ok(p) = self.project(project) else {
            return true;
        };
        self.handover_account(engine, project)
            .is_some_and(|id| accounts::may_switch_to(&self.settings.read(), &p, &id))
    }

    /// When the quota the agent's account of its harness has spent resets: the resume it waits
    /// for, else the end of its meters that put the account out (`pace`); None when it is not spent.
    fn spent_until(&self, id: &str) -> Option<i64> {
        let h = self.agent(id).ok()?;
        let (resume, account) = {
            let rt = h.lock();
            (rt.meta.resume_at, rt.meta.account.clone())
        };
        if resume.is_some() {
            return resume;
        }
        let account = accounts::get(&self.settings.read(), &account).id;
        let threshold = board::quota_threshold(self.settings.read().quota_pause);
        let now = self.pause_now();
        let u = self.usage.lock();
        let meters = &u.account(&account)?.meters;
        meters
            .iter()
            .filter(|m| pace::judge_meter(m, threshold, now).standing == Standing::Out)
            .filter_map(|m| m.resets_at)
            .max()
    }

    /// What git says of the agent's folder for its handover file.
    async fn changes_of(&self, cwd: &str, base: Option<&str>) -> Changes {
        let text = |args: Vec<String>| async move {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            crate::git::text(cwd, &args).await.unwrap_or_default()
        };
        let commits = match base {
            Some(base) => {
                text(vec![
                    "log".into(),
                    "--oneline".into(),
                    "-n".into(),
                    "50".into(),
                    format!("{base}..HEAD"),
                ])
                .await
            }
            None => String::new(),
        };
        Changes {
            status: text(vec!["status".into(), "--short".into()]).await,
            stat: text(vec!["diff".into(), "--stat".into(), "HEAD".into()]).await,
            commits,
        }
    }

    /// « Continuer sur <harness> » (and « Revenir sur <harness> »): the agent goes on on `target`
    /// (one of `handover_targets`, or the one it left, once its quota reset). Its process stops
    /// and is gone; it takes an account of `target` serving its project, a mode that never allows
    /// more than its own, the handover model of `target`, a new session; the conversation shows
    /// the change; then the handover file is its first message there. In a terminal view before
    /// (« Vue <harness> »), it goes in the view of `target` once the turn of that file ended, if
    /// `target` has one (`view_after_handover`): else it stays in Escouade's.
    pub async fn continue_on_engine(self: &Arc<Self>, id: &str, target: EngineId) -> Result<()> {
        let lang = crate::i18n::ui();
        let h = self.agent(id)?;
        let (current, project, left, terminal) = {
            let rt = h.lock();
            (
                rt.meta.engine,
                rt.meta.project_id.clone(),
                rt.meta.handed_from.clone(),
                // A handover file of a terminal view, its turn not ended yet: the view still holds.
                rt.meta.terminal || rt.meta.attached || rt.view_after_handover,
            )
        };
        let to = engine::by_id(target);
        let to_name = engine::by_name(target);
        let working = || {
            anyhow!(tr_in!(
                lang,
                "L’agent travaille encore : change de harness quand son tour est fini.",
                "The agent is still working: change harness once its turn is over."
            ))
        };
        // Refused before anything changes (its view among them); checked again under the lock.
        if h.lock().meta.status == AgentStatus::Running {
            return Err(working());
        }
        if current == target {
            bail!(tr_in!(
                lang,
                "L’agent tourne déjà sur {to_name}.",
                "The agent already runs on {to_name}."
            ));
        }
        let back = left.as_ref().is_some_and(|l| l.engine == target);
        if !self.handover_targets(id)?.contains(&target) {
            if let Some(until) = left
                .as_ref()
                .filter(|l| l.engine == target)
                .and_then(|l| l.until)
            {
                let when = chrono_time(until);
                bail!(tr_in!(
                    lang,
                    "Le quota {to_name} n’est pas revenu (remise à zéro à {when}).",
                    "The {to_name} quota hasn’t come back (it resets at {when})."
                ));
            }
            return Err(not_served(lang, to_name));
        }
        let account = self
            .handover_account(target, &project)
            .ok_or_else(|| not_served(lang, to_name))?;
        // The mode and model it will run in there, refused before anything stops.
        let settings = self.settings.read().clone();
        let model = model_for(&settings, to);
        let mode = {
            let current_mode = h.lock().meta.mode.clone();
            let auto = self.takes_auto(target, &account, &model);
            closest_mode(&current_mode, &modes_of(to, auto)).ok_or_else(|| {
                anyhow!(tr_in!(
                    lang,
                    "{to_name} n’a aucun mode qui autorise aussi peu que celui de l’agent.",
                    "{to_name} has no mode that allows as little as the agent’s."
                ))
            })?
        };
        // Its own harness's interface stops first (« Vue <harness> »): the handover goes over the
        // stream.
        if terminal {
            self.set_agent_view(id, false).await?;
        }
        let limited = !back && self.engine_spent(id).unwrap_or(false);
        let until = if limited { self.spent_until(id) } else { None };
        let until = until.or_else(|| limited.then(|| self.pause_now() + board::LIMIT_PAUSE_MS));
        let ticket = h.lock().meta.ticket_id.clone();
        let ticket = ticket
            .and_then(|t| self.ticket(&t).ok())
            .map(|t| format!("{} — {}", t.key, t.title));
        let (cwd, base) = {
            let rt = h.lock();
            let base = rt.meta.worktree.as_ref().map(|w| w.base_branch.clone());
            (rt.meta.cwd.clone(), base)
        };
        let changes = self.changes_of(&cwd, base.as_deref()).await;
        let from_name = engine::by_name(current);
        let agent_lang = self.lang().claude;
        // Its process is not started again meanwhile (a message, a warm-up wait for the lock).
        let lock = self.spawn_lock(id);
        let guard = lock.lock().await;
        let (text, stopped, from_account) = {
            let mut rt = h.lock();
            if rt.meta.status == AgentStatus::Running {
                return Err(working());
            }
            let items = rt.conv.items();
            let window = to.context_window(&model);
            let text = dossier(&Dossier {
                lang: agent_lang,
                from: from_name,
                limited,
                items: &items,
                plan: rt.meta.plan.as_ref(),
                progress: rt.meta.progress_line.as_deref(),
                changes: &changes,
                ticket: ticket.as_deref(),
                budget: budget(window),
            });
            (text, rt.detach(), rt.meta.account.clone())
        };
        if let Some(p) = stopped {
            if !accounts::ended(&p).await {
                bail!(tr_in!(
                    lang,
                    "Le process de l’agent ne s’arrête pas : il reste sur {from_name}.",
                    "The agent’s process won’t stop: it stays on {from_name}."
                ));
            }
        }
        let mut fx = Effects::default();
        let (pid, name, view) = {
            let mut rt = h.lock();
            // A question or a permission of the process gone has no one to answer it any more:
            // the handover file tells it.
            rt.clear_pending(&mut fx);
            rt.meta.status = AgentStatus::Done;
            rt.fact(crate::facts::Body::AgentHandover {
                from: rt.meta.engine.to_string(),
                to: target.to_string(),
            });
            rt.meta.engine = target;
            rt.meta.account = account;
            rt.meta.session_id = None;
            rt.meta.last_entry = None;
            rt.meta.fork_of = None;
            rt.meta.fork_at = None;
            rt.meta.model = model;
            rt.meta.set_mode(&mode);
            if !to.supports_effort(&rt.meta.model) {
                rt.meta.effort = String::new();
            } else if !to.efforts().contains(&rt.meta.effort.as_str()) {
                rt.meta.effort = to
                    .efforts()
                    .first()
                    .map(|e| e.to_string())
                    .unwrap_or_default();
            }
            rt.meta.resume_at = None;
            rt.meta.moved_from = None;
            rt.meta.moved_resume_at = None;
            rt.meta.handed_from = (!back).then(|| HandedFrom {
                engine: current,
                account: from_account.clone(),
                until,
            });
            rt.view_after_handover = terminal
                && rt.meta.ticket_id.is_none()
                && (to.terminal().is_some() || to.attaches());
            rt.append(separator(current, target, limited, until), &mut fx);
            fx.save = true;
            (rt.meta.project_id.clone(), rt.meta.name.clone(), rt.view())
        };
        drop(guard);
        log::info!("agent {id}: handed over from {current} to {target}");
        // The account it left meets the limit until its reset: no ticket starts on it meanwhile
        // (its resume, which held it, is dropped).
        if let Some(until) = until {
            self.hold_until(&accounts::get(&settings, &from_account).id, until);
        }
        self.apply(id, &pid, &name, fx, Some(view));
        self.request_save();
        self.schedule();
        let sent = self.send_handover(id, text).await;
        if sent.is_err() {
            // No turn to wait for: it stays in Escouade's view.
            h.lock().view_after_handover = false;
        }
        sent
    }

    /// The turn of its handover file ended (`turn_ended`): an agent handed over from a terminal
    /// view goes in the view of its new harness, on its new session (ESC-120). Stopped by the
    /// usage limit, it stays in Escouade's view, where « Continuer sur … » is offered.
    pub(crate) async fn view_after_handover(self: &Arc<Self>, id: &str, end: &TurnEnd) {
        let Ok(h) = self.agent(id) else { return };
        if !std::mem::take(&mut h.lock().view_after_handover) || *end == TurnEnd::Limited {
            return;
        }
        if let Err(e) = self.set_agent_view(id, true).await {
            log::warn!("agent {id}: not in its harness's view after its handover: {e:#}");
        }
    }

    /// « Continuer sur le harness suivant… »: the agent stopped by the usage limit, whose harness
    /// is spent for its project, goes on on the first of its fallback harnesses that has an
    /// account to take it; never the one it left. True when it did.
    pub(crate) async fn handover_on_limit(self: &Arc<Self>, id: &str) -> bool {
        if !self.settings.read().handover_on_limit {
            return false;
        }
        let Ok(h) = self.agent(id) else { return false };
        let (archived, project, left) = {
            let rt = h.lock();
            (
                rt.meta.archived,
                rt.meta.project_id.clone(),
                rt.meta.handed_from.as_ref().map(|l| l.engine),
            )
        };
        // Its ticket set on its account waits for it, on its harness (ESC-153).
        if archived || self.agent_pinned(id) || !self.engine_spent(id).unwrap_or(false) {
            return false;
        }
        let targets = self.handover_targets(id).unwrap_or_default();
        let Some(target) = targets
            .into_iter()
            .filter(|e| Some(*e) != left)
            .filter(|e| !self.usable_accounts(*e, &project, None).is_empty())
            // A private project goes on its own to none of the accounts it shares (ESC-141).
            .find(|e| self.may_hand_over_to(*e, &project))
        else {
            return false;
        };
        match self.continue_on_engine(id, target).await {
            Ok(()) => true,
            Err(e) => {
                log::warn!("agent {id}: no handover to {target} after the usage limit: {e:#}");
                let _ = self.with_agent(id, |rt, fx| {
                    rt.notice(
                        "warn",
                        tr!(
                            "Suite sur un autre harness impossible : {e:#}",
                            "Couldn’t go on on another harness: {e:#}"
                        ),
                        fx,
                    );
                    Ok(())
                });
                false
            }
        }
    }
}

/// `at` (ms) as the local time of day, `18:00`.
fn chrono_time(at: i64) -> String {
    chrono::DateTime::from_timestamp_millis(at)
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "handover_tests.rs"]
mod tests;
