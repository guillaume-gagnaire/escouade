//! « Planifier » (ESC-262, `docs/FACTORY.md` « « Planifier » : de la spec aux tickets »): once the
//! spec was changed, a planner adds to the Kanban the tickets that implement the changes.
//!
//! - **Where**: on a discussion whose changes were poured, and on top of the Specification while
//!   changes have no tickets (« 3 changements sans tickets · Planifier »).
//! - **What it reads**: the pages changed and the decisions proposed since the last planning (kept
//!   by project in `planning.json` of the data folder: a pour, a page written or a decision
//!   proposed from the window), the code, by the index (the files it links to the changes) and by
//!   reading the folder, and the tickets of the board, not to make one twice nor one for what is
//!   done already.
//! - **What it does**: it is a short agent that reads without changing anything
//!   (`Weight::Reading`, as the comparison to the code); it answers the tickets, which Escouade
//!   makes in « À faire », each with its description, its criteria, what it comes after
//!   (« Après ») and the pages and decisions it implements (`Ticket::pages`, `Ticket::decisions`,
//!   and its key in each page's `tickets`). A ticket the board already has, by its title, is left
//!   out (`sift`). They are not started: the autopilot and the board's rules apply as to any ticket.
//!   What it made is told where it was asked (`Plan`), with « Annuler », which removes the tickets
//!   not started yet and makes the changes unplanned again.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tauri::Runtime;

use crate::core::Core;
use crate::docs::{Decision, Page};
use crate::i18n::Lang;
use crate::model::{Column, Ticket};
use crate::tickets::TicketDraft;

/// How long the planner has.
pub const LIMIT: Duration = Duration::from_secs(600);
/// The tickets one planning makes at most.
pub const MAX_TICKETS: usize = 12;
/// What of a page's text the planner is given.
const MAX_BODY: usize = 12_000;
/// The files the index links to the changes, named to the planner.
const MAX_HINTS: usize = 12;
/// The plans kept by project (the latest ones).
const KEPT_PLANS: usize = 20;

/// The projects being planned now.
static PLANNING: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Whether `project_id` is being planned.
pub fn planning(project_id: &str) -> bool {
    PLANNING
        .lock()
        .map(|p| p.iter().any(|x| x == project_id))
        .unwrap_or(false)
}

/// A ticket a planning made.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Made {
    pub id: String,
    pub key: String,
    pub title: String,
}

/// A ticket the planner proposed and Escouade left out: the board has it already (`key`).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Skipped {
    pub title: String,
    pub key: String,
}

/// What a planning did.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Plan {
    pub id: String,
    /// When (ms).
    pub at: i64,
    /// The discussion it was asked from (its agent); None: from the Specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discussion: Option<String>,
    /// The changes it planned.
    pub pages: Vec<String>,
    pub decisions: Vec<u32>,
    pub made: Vec<Made>,
    pub skipped: Vec<Skipped>,
    /// « Annuler » was clicked.
    pub cancelled: bool,
}

/// What a project has to plan, and its plans, in `planning.json` of its data folder.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Planning {
    /// The pages changed since they were last planned.
    pub pages: Vec<String>,
    /// The decisions proposed since, by number.
    pub decisions: Vec<u32>,
    /// The latest plans, the latest last.
    pub plans: Vec<Plan>,
}

impl Planning {
    /// Notes changes to plan.
    pub fn note(&mut self, pages: &[String], decisions: &[u32]) {
        for p in pages {
            if !self.pages.contains(p) {
                self.pages.push(p.clone());
            }
        }
        for d in decisions {
            if !self.decisions.contains(d) {
                self.decisions.push(*d);
            }
        }
    }

    /// The changes planned: no longer to plan.
    pub fn planned(&mut self, pages: &[String], decisions: &[u32]) {
        self.pages.retain(|p| !pages.contains(p));
        self.decisions.retain(|d| !decisions.contains(d));
    }

    /// The plan `id`.
    pub fn plan_mut(&mut self, id: &str) -> Option<&mut Plan> {
        self.plans.iter_mut().find(|p| p.id == id)
    }

    /// Keeps `plan`, the latest.
    pub fn push(&mut self, plan: Plan) {
        self.plans.push(plan);
        let over = self.plans.len().saturating_sub(KEPT_PLANS);
        self.plans.drain(..over);
    }
}

/// What the Specification shows of the planning (in `docs_spec_tree`).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct View {
    /// The changes without tickets: the pages still there, the decisions still proposed or
    /// accepted.
    pub pages: Vec<String>,
    pub decisions: Vec<u32>,
    /// The latest plan, unless cancelled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<Plan>,
    pub running: bool,
}

/// The view of `planning`, against the pages and decisions there are.
pub fn view(planning: &Planning, pages: &[Page], decisions: &[Decision]) -> View {
    View {
        pages: planning
            .pages
            .iter()
            .filter(|p| pages.iter().any(|x| &x.path == *p))
            .cloned()
            .collect(),
        decisions: planning
            .decisions
            .iter()
            .filter(|n| {
                decisions
                    .iter()
                    .any(|d| d.number == **n && d.state != crate::docs::State::Superseded)
            })
            .copied()
            .collect(),
        last: planning.plans.last().filter(|p| !p.cancelled).cloned(),
        running: false,
    }
}

// ---------- what the planner is told, and what is read of its answer ----------

pub fn system(lang: Lang) -> String {
    tr_in!(
        lang,
        "Tu es l'agent planificateur d'un projet. La spécification du produit a changé : tu découpes ces changements en tickets de Kanban pour les agents qui coderont, en lisant le code de ce dépôt sans rien modifier. \
         Un ticket par morceau de travail livrable seul, dans l'ordre où les faire ; pas de ticket pour ce que le code fait déjà, ni pour ce qu'un ticket du Kanban (<board>, même terminé) couvre déjà. \
         Chaque ticket a un titre court, une description (ce qu'il faut faire, où dans le code), ses critères d'acceptation vérifiables, les tickets qu'il suit (\"after\" : le numéro d'un ticket de ta liste, à partir de 1, ou la clé d'un ticket du Kanban), et les pages et décisions qu'il implémente. \
         Réponds par un bloc ```json qui contient \
         {{\"tickets\": [{{\"title\": \"…\", \"description\": \"…\", \"criteria\": [\"…\"], \"after\": [\"1\"], \"pages\": [\"sujet/page\"], \"decisions\": [3]}}]}}, \
         avec \"tickets\": [] quand il n'y a rien à faire. {max} tickets au plus.",
        "You are the planner of a project. The product's specification changed: you cut these changes into Kanban tickets for the agents who will code, reading the code of this repository without changing anything. \
         One ticket per piece of work that can be delivered alone, in the order to do them; no ticket for what the code does already, nor for what a ticket of the board (<board>, even done) covers already. \
         Each ticket has a short title, a description (what to do, where in the code), its acceptance criteria that can be checked, the tickets it comes after (\"after\": the number of a ticket of your list, from 1, or the key of a ticket of the board), and the pages and decisions it implements. \
         Answer with a ```json block that holds \
         {{\"tickets\": [{{\"title\": \"…\", \"description\": \"…\", \"criteria\": [\"…\"], \"after\": [\"1\"], \"pages\": [\"subject/page\"], \"decisions\": [3]}}]}}, \
         with \"tickets\": [] when there is nothing to do. {max} tickets at most.",
        max = MAX_TICKETS
    )
}

fn cut(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

fn column_word(c: Column) -> &'static str {
    match c {
        Column::Todo => "todo",
        Column::Doing => "doing",
        Column::Review => "review",
        Column::Done => "done",
    }
}

/// What the planner is given: the pages changed, the decisions, the board and the files the
/// index links to them.
pub fn prompt(
    lang: Lang,
    pages: &[Page],
    decisions: &[Decision],
    board: &[Ticket],
    hints: &[String],
) -> String {
    let mut p = String::new();
    for page in pages {
        p.push_str(&format!(
            "<changed-page path=\"{}\" title=\"{}\">\n{}\n</changed-page>\n",
            page.path,
            page.title.replace('"', "'"),
            cut(&page.body, MAX_BODY)
        ));
    }
    for d in decisions {
        p.push_str(&format!(
            "<decision number=\"{}\" title=\"{}\">\n{}\n{}\n</decision>\n",
            d.number,
            d.title.replace('"', "'"),
            d.decision.trim(),
            d.consequences.trim()
        ));
    }
    let lines: Vec<String> = board
        .iter()
        .map(|t| {
            let first = t.description.lines().next().unwrap_or("").trim();
            format!(
                "{} [{}] {}{}",
                t.key,
                column_word(t.column),
                t.title,
                if first.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", cut(first, 160))
                }
            )
        })
        .collect();
    p.push_str(&format!("<board>\n{}\n</board>\n", lines.join("\n")));
    if !hints.is_empty() {
        p.push_str(&format!(
            "<code-hints>\n{}\n</code-hints>\n",
            hints.join("\n")
        ));
    }
    p.push_str(&if hints.is_empty() {
        tr_in!(
            lang,
            "Planifie ces changements.",
            "Plan these changes."
        )
    } else {
        tr_in!(
            lang,
            "Planifie ces changements (les fichiers de <code-hints> sont ceux que l'index du code rapproche d'eux).",
            "Plan these changes (the files in <code-hints> are those the code index links to them)."
        )
    });
    p
}

/// A ticket as the planner proposes it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Proposed {
    pub title: String,
    pub description: String,
    pub criteria: Vec<String>,
    /// The number of one of the list (from 1), or a key of the board.
    pub after: Vec<String>,
    pub pages: Vec<String>,
    pub decisions: Vec<u32>,
}

/// The JSON of an answer: its ```json block, else the answer itself.
fn json_of(answer: &str) -> &str {
    if let Some(at) = answer.find("```json") {
        let rest = &answer[at + 7..];
        return rest.split("```").next().unwrap_or(rest).trim();
    }
    let t = answer.trim();
    match (t.find('{'), t.rfind('}')) {
        (Some(a), Some(b)) if a < b => &t[a..=b],
        _ => t,
    }
}

fn strings(v: Option<&serde_json::Value>) -> Vec<String> {
    v.and_then(|c| c.as_array())
        .map(|c| {
            c.iter()
                .filter_map(|x| match x {
                    serde_json::Value::String(s) => Some(s.trim().to_string()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The tickets of an answer; None when it is not one.
pub fn proposed_from_answer(answer: &str) -> Option<Vec<Proposed>> {
    let v: serde_json::Value = serde_json::from_str(json_of(answer)).ok()?;
    let list = v.get("tickets")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|t| {
                let title = t.get("title")?.as_str()?.trim().to_string();
                Some(Proposed {
                    title,
                    description: t
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                    criteria: strings(t.get("criteria")),
                    after: strings(t.get("after")),
                    pages: strings(t.get("pages")),
                    decisions: strings(t.get("decisions"))
                        .iter()
                        .filter_map(|n| n.trim_start_matches(['D', 'd', '-']).parse().ok())
                        .collect(),
                })
            })
            .filter(|t| !t.title.is_empty())
            .collect(),
    )
}

/// A title as two tickets are told the same: its words, lowercase, without accents nor marks.
pub fn same_title(title: &str) -> String {
    let folded: String = title
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What a ticket to make comes after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum After {
    /// A ticket of the board, by id.
    Board(String),
    /// One made before it in the same planning, by its place among those to make.
    Made(usize),
}

/// A ticket to make, as `sift` leaves it.
#[derive(Debug, Clone, PartialEq)]
pub struct ToMake {
    pub ticket: Proposed,
    pub after: Vec<After>,
}

/// The tickets proposed that are made, and those left out because the board has them already (by
/// their title, in any column) or the list has them twice. Their pages are only those of `pages`,
/// their decisions those of `decisions`; what they come after, a ticket of the board (by key) or
/// one before them in the list (by number, from 1; one left out stands for the ticket of the
/// board it repeats). At most `MAX_TICKETS`.
pub fn sift(
    proposed: Vec<Proposed>,
    board: &[Ticket],
    pages: &[String],
    decisions: &[u32],
) -> (Vec<ToMake>, Vec<Skipped>) {
    let mut make: Vec<ToMake> = Vec::new();
    let mut skipped: Vec<Skipped> = Vec::new();
    // For each one proposed (by its place): what it became.
    let mut became: Vec<Option<After>> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for p in proposed {
        let same = same_title(&p.title);
        if let Some(t) = board.iter().find(|t| same_title(&t.title) == same) {
            skipped.push(Skipped {
                title: p.title.clone(),
                key: t.key.clone(),
            });
            became.push(Some(After::Board(t.id.clone())));
            continue;
        }
        if same.is_empty() || !seen.insert(same.clone()) || make.len() >= MAX_TICKETS {
            if let Some(i) = make
                .iter()
                .position(|m| same_title(&m.ticket.title) == same)
            {
                became.push(Some(After::Made(i)));
            } else {
                became.push(None);
            }
            continue;
        }
        let mut after: Vec<After> = Vec::new();
        for a in &p.after {
            let found = match a.trim().parse::<usize>() {
                Ok(n) if n >= 1 && n <= became.len() => became[n - 1].clone(),
                _ => board
                    .iter()
                    .find(|t| t.key.eq_ignore_ascii_case(a.trim()))
                    .map(|t| After::Board(t.id.clone())),
            };
            if let Some(f) = found {
                if !after.contains(&f) {
                    after.push(f);
                }
            }
        }
        let mut ticket = p;
        let mut kept = Vec::new();
        for page in &ticket.pages {
            let path = page
                .trim()
                .trim_start_matches(".escouade/spec/")
                .trim_end_matches(".md")
                .to_string();
            if pages.contains(&path) && !kept.contains(&path) {
                kept.push(path);
            }
        }
        ticket.pages = kept;
        ticket.decisions.retain(|d| decisions.contains(d));
        ticket.decisions.dedup();
        became.push(Some(After::Made(make.len())));
        make.push(ToMake { ticket, after });
    }
    (make, skipped)
}

/// The paragraph a ticket's description ends with: the pages and decisions it implements, for its
/// agent to read them.
pub fn implements(lang: Lang, pages: &[String], decisions: &[u32]) -> String {
    let mut parts = Vec::new();
    if !pages.is_empty() {
        let list = pages
            .iter()
            .map(|p| format!("`{p}`"))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(tr_in!(
            lang,
            "pages {list} (read_spec_page)",
            "pages {list} (read_spec_page)"
        ));
    }
    if !decisions.is_empty() {
        let list = decisions
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(tr_in!(
            lang,
            "décisions n° {list} (read_decision)",
            "decisions {list} (read_decision)"
        ));
    }
    if parts.is_empty() {
        return String::new();
    }
    tr_in!(
        lang,
        "Implémente ce que dit la spec : {}.",
        "Implements what the spec says: {}.",
        parts.join(" ; ")
    )
}

// ---------- the store ----------

fn read(file: &std::path::Path) -> Planning {
    std::fs::read(file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

struct Marked(String);

impl Drop for Marked {
    fn drop(&mut self) {
        if let Ok(mut p) = PLANNING.lock() {
            p.retain(|x| x != &self.0);
        }
    }
}

/// What « Annuler » did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cancelled {
    /// The tickets removed.
    pub removed: Vec<String>,
    /// The tickets kept: started already, by their key.
    pub kept: Vec<String>,
}

impl<R: Runtime> Core<R> {
    fn planning_file(&self, project_id: &str) -> PathBuf {
        self.data.project_files(project_id).join("planning.json")
    }

    /// A project's planning, as it was saved.
    pub fn planning_of(&self, project_id: &str) -> Planning {
        read(&self.planning_file(project_id))
    }

    fn planning_edit<T>(&self, project_id: &str, edit: impl FnOnce(&mut Planning) -> T) -> T {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let file = self.planning_file(project_id);
        let mut p = read(&file);
        let out = edit(&mut p);
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match serde_json::to_vec_pretty(&p) {
            Ok(b) => {
                if let Err(e) = crate::paths::write_atomic(&file, &b) {
                    log::warn!("planning of {project_id} not saved: {e}");
                }
            }
            Err(e) => log::warn!("planning of {project_id} not saved: {e}"),
        }
        out
    }

    /// The spec changed outside of a ticket (a pour, the window): `pages` and `decisions` are to
    /// plan.
    pub fn docs_to_plan(&self, project_id: &str, pages: &[String], decisions: &[u32]) {
        if pages.is_empty() && decisions.is_empty() {
            return;
        }
        self.planning_edit(project_id, |p| p.note(pages, decisions));
    }

    /// What the Specification shows of the planning.
    pub async fn docs_planning(&self, project_id: &str) -> Result<View> {
        let space = self.docs_space(project_id, None).await?;
        let mut v = view(
            &self.planning_of(project_id),
            &space.pages(),
            &space.decisions(),
        );
        v.running = planning(project_id);
        Ok(v)
    }

    /// « Planifier »: the changes of the discussion `discussion` (its agent) poured and not
    /// planned yet, or every change to plan when None, given to the planner; the tickets it
    /// proposes made in « À faire », not started, linked to their pages.
    pub async fn docs_plan(
        self: &Arc<Self>,
        project_id: &str,
        discussion: Option<&str>,
    ) -> Result<Plan> {
        let project = self.project(project_id)?;
        let planning_now = self.planning_of(project_id);
        let (mut pages, mut decisions) = (planning_now.pages, planning_now.decisions);
        if let Some(id) = discussion {
            let poured = {
                let h = self.agent(id)?;
                let rt = h.lock();
                rt.meta.discussion.as_ref().and_then(|d| d.poured.clone())
            };
            let Some(poured) = poured else {
                bail!(tr!(
                    "Verse d’abord les changements de la discussion dans la spec.",
                    "Pour the discussion’s changes into the spec first."
                ));
            };
            pages.retain(|p| poured.pages.contains(p));
            decisions.retain(|d| poured.decisions.contains(d));
        }
        let space = self.docs_space(project_id, None).await?;
        let all_pages = space.pages();
        let all_decisions = space.decisions();
        let v = view(
            &Planning {
                pages,
                decisions,
                plans: Vec::new(),
            },
            &all_pages,
            &all_decisions,
        );
        if v.pages.is_empty() && v.decisions.is_empty() {
            bail!(tr!(
                "Rien à planifier : ces changements de la spec ont déjà leurs tickets.",
                "Nothing to plan: these changes of the spec have their tickets already."
            ));
        }
        let _marked = {
            let mut p = PLANNING.lock().unwrap_or_else(|e| e.into_inner());
            if p.iter().any(|x| x == project_id) {
                bail!(tr!(
                    "Une planification de ce projet est déjà en cours.",
                    "A planning of this project is already under way."
                ));
            }
            p.push(project_id.to_string());
            Marked(project_id.to_string())
        };
        let changed: Vec<Page> = all_pages
            .iter()
            .filter(|p| v.pages.contains(&p.path))
            .cloned()
            .collect();
        let decided: Vec<Decision> = all_decisions
            .iter()
            .filter(|d| v.decisions.contains(&d.number))
            .cloned()
            .collect();
        let board: Vec<Ticket> = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == project_id)
            .cloned()
            .collect();
        let lang = self.lang().claude;
        let hints = self.plan_hints(&project, &changed, &decided).await;
        let folder = crate::docsbranch::base_of(&project);
        let answer = self
            .read_folder(
                project_id,
                &folder,
                &system(lang),
                &prompt(lang, &changed, &decided, &board, &hints),
                LIMIT,
            )
            .await?;
        let Some(proposed) = proposed_from_answer(&answer) else {
            bail!(tr!(
                "la réponse du planificateur n’est pas une liste de tickets",
                "the planner’s answer is not a list of tickets"
            ));
        };
        // The board as it is now: a ticket made meanwhile is not made twice.
        let board: Vec<Ticket> = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == project_id)
            .cloned()
            .collect();
        let (make, skipped) = sift(proposed, &board, &v.pages, &v.decisions);
        let mut made: Vec<Made> = Vec::new();
        for m in &make {
            let after: Vec<String> = m
                .after
                .iter()
                .filter_map(|a| match a {
                    After::Board(id) => Some(id.clone()),
                    After::Made(i) => made.get(*i).map(|x| x.id.clone()),
                })
                .collect();
            let link = implements(lang, &m.ticket.pages, &m.ticket.decisions);
            let description = match (m.ticket.description.is_empty(), link.is_empty()) {
                (_, true) => m.ticket.description.clone(),
                (true, false) => link,
                (false, false) => format!("{}\n\n{link}", m.ticket.description),
            };
            let t = match self
                .ticket_create(
                    project_id,
                    TicketDraft {
                        title: m.ticket.title.clone(),
                        description,
                        criteria: m.ticket.criteria.clone(),
                        after,
                        pages: Some(m.ticket.pages.clone()),
                        decisions: Some(m.ticket.decisions.clone()),
                        ..Default::default()
                    },
                )
                .await
            {
                Ok(t) => t,
                // One refused stops the others: those after it would miss it.
                Err(e) if made.is_empty() => return Err(e),
                Err(e) => {
                    log::warn!("planning of {project_id}: a ticket not made: {e:#}");
                    break;
                }
            };
            for page in &m.ticket.pages {
                let key = t.key.clone();
                if let Err(e) = space.spec_write(page, |p| {
                    if !p.tickets.iter().any(|k| k.eq_ignore_ascii_case(&key)) {
                        p.tickets.push(key);
                    }
                }) {
                    log::warn!("{}: page {page} not linked: {e:#}", t.key);
                }
            }
            made.push(Made {
                id: t.id,
                key: t.key,
                title: t.title,
            });
        }
        let plan = Plan {
            id: crate::model::new_id(),
            at: crate::model::now_ms(),
            discussion: discussion.map(str::to_string),
            pages: v.pages.clone(),
            decisions: v.decisions.clone(),
            made,
            skipped,
            cancelled: false,
        };
        self.planning_edit(project_id, |p| {
            p.planned(&v.pages, &v.decisions);
            p.push(plan.clone());
        });
        if let Some(id) = discussion {
            let _ = self.discussion_edit(id, |d| d.planned = Some(plan.clone()));
        }
        self.docs_changed(project_id).await;
        Ok(plan)
    }

    /// The files the index links to the changes (none when the project has it off).
    async fn plan_hints(
        self: &Arc<Self>,
        project: &crate::model::Project,
        pages: &[Page],
        decisions: &[Decision],
    ) -> Vec<String> {
        if !self.code_index_on(project) {
            return Vec::new();
        }
        let Ok(reader) = self.code_index(&project.id, None).await else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        let tasks = pages
            .iter()
            .map(|p| format!("{} {}", p.title, cut(&p.body, 400)))
            .chain(
                decisions
                    .iter()
                    .map(|d| format!("{} {}", d.title, d.decision)),
            );
        for task in tasks {
            let Ok(a) = reader.task_context(&task, MAX_HINTS) else {
                continue;
            };
            for p in a
                .result
                .entry_points
                .iter()
                .map(|h| &h.path)
                .chain(a.result.snippets.iter().map(|s| &s.path))
            {
                if !out.contains(p) && out.len() < MAX_HINTS {
                    out.push(p.clone());
                }
            }
        }
        out
    }

    /// « Annuler » of the plan `plan_id`: its tickets not started yet removed (and their key from
    /// their pages), its changes to plan again.
    pub async fn docs_plan_cancel(
        self: &Arc<Self>,
        project_id: &str,
        plan_id: &str,
    ) -> Result<Cancelled> {
        let Some(plan) = self
            .planning_of(project_id)
            .plans
            .into_iter()
            .find(|p| p.id == plan_id)
        else {
            bail!(tr!(
                "Cette planification n’est plus là.",
                "This planning is no longer there."
            ));
        };
        let mut out = Cancelled::default();
        let space = self.docs_space(project_id, None).await?;
        for m in &plan.made {
            let Ok(t) = self.ticket(&m.id) else { continue };
            if t.column != Column::Todo || t.started_at.is_some() || t.agent_id.is_some() {
                out.kept.push(t.key);
                continue;
            }
            self.ticket_delete(&t.id).await?;
            for page in &t.pages {
                let _ = space.spec_write(page, |p| {
                    p.tickets.retain(|k| !k.eq_ignore_ascii_case(&t.key));
                });
            }
            out.removed.push(t.key);
        }
        self.planning_edit(project_id, |p| {
            p.note(&plan.pages, &plan.decisions);
            if let Some(x) = p.plan_mut(plan_id) {
                x.cancelled = true;
            }
        });
        if let Some(id) = plan.discussion.as_deref() {
            let _ = self.discussion_edit(id, |d| {
                if d.planned.as_ref().is_some_and(|p| p.id == plan_id) {
                    d.planned = None;
                }
            });
        }
        self.docs_changed(project_id).await;
        Ok(out)
    }
}
