//! The discussions of the Docs tab (ESC-261, `docs/FACTORY.md` § 3, « Nouvelle discussion » and
//! « Une discussion ouverte »): an agent of Escouade like the others (the same conversation, the
//! same answers from the list), with a role of its own:
//!
//! - **Discussion**: the spec writer, to ask the spec or change it simply;
//! - **Brainstorm**: a product agent, in the manner of BMAD, that matures an idea (uses, discovery,
//!   compliance and GDPR, parallels with other trades) and proposes pages, decisions and tickets.
//!
//! It starts on the project's harness, account and model (the window's footer may change them), in
//! the folder of the docs branch (`Core::docs_space`), told by its instructions to read the code
//! (by the index) and never change it. Its harness holds it to that, in any mode
//! (`engine::Spawn::read_only`): Claude Code has its tools that write taken away, Codex a sandbox
//! that reads only, OpenCode its edits and commands refused, Copilot CLI no write let through by
//! Escouade's rules. Cursor CLI and Mistral Vibe write the files of their folder without going
//! through Escouade: no discussion runs on them (`holds`). What is still asked of the user is
//! answered at once (`AgentRt::discussion_answers`): a write or a command refused, Escouade's tools
//! that read and write the docs let through (`allowed_tool`), the rest left to the user.
//!
//! What it writes in the spec and the decisions, through the docs tools, does not go in the docs:
//! it is kept on the discussion (`Discussion::pages`, `Discussion::decisions`), shown in « Ce qui
//! ira dans la spec », and written on the docs branch by « Verser dans la spec »
//! (`Core::discussion_pour`). Nothing is committed: the status bar says what is left to share.

use std::sync::Arc;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tauri::Runtime;

use crate::core::{AgentOptions, Core};
use crate::docs::{Change, Decision, Gap, Page};
use crate::i18n::Lang;
use crate::model::{AgentView, EngineId};

/// Its role.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The spec writer.
    #[default]
    Discussion,
    /// The product agent.
    Brainstorm,
}

impl Kind {
    /// Its word as a decision's origin.
    pub fn origin(self) -> &'static str {
        match self {
            Kind::Discussion => "discussion",
            Kind::Brainstorm => "brainstorm",
        }
    }
}

/// What makes an agent a discussion (`AgentMeta::discussion`).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Discussion {
    pub kind: Kind,
    pub subject: String,
    /// The pages it proposes, one change each (the latest of the page), not poured yet.
    pub pages: Vec<PageChange>,
    /// The decisions it proposes, not poured yet.
    pub decisions: Vec<Decision>,
    /// What was poured into the spec so far; None until then.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poured: Option<Poured>,
    /// The gap it was opened on (« Mettre la spec à jour »): gone from its page once a change of
    /// that page is poured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settles: Option<Settles>,
    /// What « Planifier » made of what was poured (ESC-262); None until then, and once cancelled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned: Option<crate::docsplan::Plan>,
}

/// A gap of a page a discussion settles.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settles {
    pub page: String,
    pub gap: Gap,
}

/// The gap of a page, by its place, as the window names it.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GapRef {
    pub page: String,
    pub index: usize,
}

/// A page as the discussion proposes it.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PageChange {
    pub path: String,
    /// The page's text when its first change was proposed; None: a new page.
    pub before: Option<String>,
    /// The page as it would be written.
    pub page: Page,
    /// The lines it adds and removes from `before`.
    pub added: u32,
    pub removed: u32,
}

/// What « Verser dans la spec » wrote, every time added up.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Poured {
    /// When it was last poured (ms).
    pub at: i64,
    /// The pages written, each once.
    pub pages: Vec<String>,
    /// The decisions proposed in the register (their ids, ESC-303).
    pub decisions: Vec<crate::docs::Ref>,
}

/// What the window asks for a new discussion.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Start {
    pub kind: Kind,
    pub subject: String,
    /// The documents joined, already filed in the drive (their paths under `documents`).
    pub documents: Vec<String>,
    /// The first message, when it is not the subject (« Mettre la spec à jour », « Écrire une
    /// première spec »).
    pub message: Option<String>,
    /// The gap it settles (« Mettre la spec à jour »).
    pub gap: Option<GapRef>,
    pub engine: Option<EngineId>,
    pub account: Option<String>,
    pub model: Option<String>,
}

/// The modes a discussion's agent may be in: those where a write is asked for, never done.
pub const MODES: [&str; 2] = ["default", "plan"];

/// The tools that write a file or run a command, as Claude Code names them (every harness's are
/// named so on their cards): taken from a discussion's Claude Code, refused to the others.
pub const WRITES: [&str; 5] = ["Edit", "Write", "MultiEdit", "NotebookEdit", "Bash"];

/// Whether a discussion may run on `engine`: its harness keeps it from writing (see the module).
pub fn holds(engine: EngineId) -> bool {
    crate::engine::by_id(engine).caps().read_only
}

/// Whether a discussion's agent is refused `tool` without asking the user: a write, a command, or
/// more permissions (Codex's).
pub fn refused_tool(tool: &str) -> bool {
    WRITES.contains(&tool) || tool == "Permissions"
}

/// The tools of Escouade's server that write the docs, as proposals (`mcp::docs`).
const DOCS_WRITES: [&str; 3] = ["write_spec_page", "propose_decision", "replace_decision"];

/// Whether a discussion's agent is let use `tool` without asking the user: Escouade's tools that
/// read (the board, the docs, the memory, the code index) and those that write the docs. Any other
/// (a ticket made or started, a message to an agent, the docs shared…) is the user's to allow.
pub fn allowed_tool(tool: &str) -> bool {
    let Some(name) = tool.strip_prefix("mcp__escouade__") else {
        return false;
    };
    DOCS_WRITES.contains(&name)
        || ["get_", "list_", "search_", "read_", "index_"]
            .iter()
            .any(|p| name.starts_with(p))
}

/// What the agent is told when a write is refused, in `lang`.
pub fn refusal(lang: Lang) -> String {
    tr_in!(
        lang,
        "Escouade refuse : dans une discussion, tu ne modifies jamais le code ni aucun fichier, et tu ne lances aucune commande. Lis le code par l’index ou en lisant les fichiers ; écris la spec et les décisions par les outils de docs d’Escouade.",
        "Escouade refuses: in a discussion you never change the code nor any file, and run no command. Read the code through the index or by reading files; write the spec and the decisions through Escouade’s docs tools."
    )
}

/// The agent's instructions, for the project `project`, in `lang`.
pub fn instructions(lang: Lang, kind: Kind, project: &str) -> String {
    let role = match kind {
        Kind::Discussion => tr_in!(
            lang,
            "Tu es l’agent rédacteur de spec du projet « {project} », dans une discussion de l’onglet Docs d’Escouade. On t’interroge sur la spécification du produit, ou on te demande de la mettre à jour simplement. Réponds court et précis, en citant les pages, les décisions, les documents et le code d’où tu tiens ce que tu dis.",
            "You are the spec writer of the project “{project}”, in a discussion of Escouade’s Docs tab. You are asked about the product’s specification, or to update it simply. Answer short and precise, citing the pages, decisions, documents and code you take what you say from."
        ),
        Kind::Brainstorm => tr_in!(
            lang,
            "Tu es l’agent produit du projet « {project} », dans un brainstorm de l’onglet Docs d’Escouade, à la manière de BMAD (analyste et chef de produit). Fais mûrir l’idée avec l’utilisateur pour faire gagner le produit en fonctionnalités et en maturité : usages et parcours, découverte (pour qui, quel problème, comment le vérifier), conformité et RGPD (base légale, consentement, durée de conservation), accessibilité, et ce que font d’autres secteurs dans le même cas. Propose des fonctionnalités, des pages de spec (en brouillon, draft, tant que ce n’est pas tranché), des décisions, et des tickets : ceux-là, dis-les dans la conversation sans les créer, « Planifier » les créera une fois versés.",
            "You are the product agent of the project “{project}”, in a brainstorm of Escouade’s Docs tab, in the manner of BMAD (analyst and product manager). Mature the idea with the user so the product gains features and maturity: uses and journeys, discovery (for whom, what problem, how to check it), compliance and GDPR (legal basis, consent, retention), accessibility, and what other trades do in the same case. Propose features, spec pages (as drafts until settled), decisions, and tickets: say those in the conversation without making them, “Plan tickets” will make them once poured."
        ),
    };
    let rules = tr_in!(
        lang,
        "Tu travailles dans le dossier de la branche des docs. Lis la spec (get_spec_tree, read_spec_page, search_spec), les décisions (list_decisions, read_decision), tous les documents du projet (search_documents, read_document) et le code, par l’index du code quand il est là ou en lisant les fichiers. Tu ne modifies jamais le code ni aucun fichier, et tu ne lances aucune commande : Escouade le refuse. Tu n’écris que les docs, par les outils de docs d’Escouade (project « {project} ») : write_spec_page pour une page (chemin sujet/page, rangée à sa place dans l’arbre), propose_decision ou replace_decision pour une décision. Ce que tu écris ainsi est une proposition : elle s’affiche dans « Ce qui ira dans la spec », et l’utilisateur la verse dans la spec par « Verser dans la spec ». Pose tes questions une à une, avec des choix.",
        "You work in the folder of the docs branch. Read the spec (get_spec_tree, read_spec_page, search_spec), the decisions (list_decisions, read_decision), all the project’s documents (search_documents, read_document) and the code, through the code index when there is one or by reading files. You never change the code nor any file, and run no command: Escouade refuses it. You only write the docs, through Escouade’s docs tools (project “{project}”): write_spec_page for a page (path subject/page, filed in its place in the tree), propose_decision or replace_decision for a decision. What you write so is a proposal: it shows in “What will go in the spec”, and the user pours it into the spec with “Pour into the spec”. Ask your questions one at a time, with choices."
    );
    format!("{role} {rules}")
}

/// The first message: the subject, and the documents joined.
pub fn first_message(lang: Lang, start: &Start) -> String {
    let mut text = match start.message.as_deref().filter(|m| !m.trim().is_empty()) {
        Some(m) => m.trim().to_string(),
        None => match start.kind {
            Kind::Discussion => start.subject.trim().to_string(),
            Kind::Brainstorm => tr_in!(
                lang,
                "Sujet du brainstorm : {}",
                "Brainstorm subject: {}",
                start.subject.trim()
            ),
        },
    };
    if !start.documents.is_empty() {
        let list = start
            .documents
            .iter()
            .map(|d| format!("- {d}"))
            .collect::<Vec<_>>()
            .join("\n");
        text.push_str(&tr_in!(
            lang,
            "\n\nFichiers joints, rangés dans les Documents (read_document) :\n{list}",
            "\n\nFiles joined, filed in the Documents (read_document):\n{list}"
        ));
    }
    text
}

/// A name for its agent: the subject, short.
fn agent_name(subject: &str) -> String {
    let s: String = subject.trim().chars().take(40).collect();
    let s = s.trim();
    if s.is_empty() {
        tr!("discussion", "discussion")
    } else {
        s.to_string()
    }
}

impl Discussion {
    /// The page `path` as it proposes it, if it does.
    pub fn page(&self, path: &str) -> Option<&PageChange> {
        self.pages.iter().find(|p| p.path == path)
    }

    /// Keeps `page`, the latest change of its page: its `before` is the first change's.
    pub fn propose_page(&mut self, before: Option<String>, page: Page) -> &PageChange {
        let i = match self.pages.iter().position(|p| p.path == page.path) {
            Some(i) => {
                let c = &mut self.pages[i];
                c.page = page;
                i
            }
            None => {
                self.pages.push(PageChange {
                    path: page.path.clone(),
                    before,
                    page,
                    ..Default::default()
                });
                self.pages.len() - 1
            }
        };
        let c = &mut self.pages[i];
        let (added, removed) =
            crate::docs::line_delta(c.before.as_deref().unwrap_or(""), &c.page.body);
        c.added = added;
        c.removed = removed;
        c
    }

    /// Keeps the decision `d`, from this discussion: one of the same title is revised. Its place
    /// in the list, from 1.
    pub fn propose_decision(&mut self, mut d: Decision) -> usize {
        d.origin = self.kind.origin().into();
        if d.origin_ref.trim().is_empty() {
            d.origin_ref = self.subject.clone();
        }
        let title = d.title.trim().to_lowercase();
        match self
            .decisions
            .iter()
            .position(|o| o.title.trim().to_lowercase() == title)
        {
            Some(i) => {
                self.decisions[i] = d;
                i + 1
            }
            None => {
                self.decisions.push(d);
                self.decisions.len()
            }
        }
    }
}

impl<R: Runtime> Core<R> {
    /// Changes the discussion of the agent `id` with `edit`, then tells the window and saves.
    pub fn discussion_edit<T>(
        &self,
        id: &str,
        edit: impl FnOnce(&mut Discussion) -> T,
    ) -> Result<T> {
        let h = self.agent(id)?;
        let out = {
            let mut rt = h.lock();
            let Some(d) = rt.meta.discussion.as_mut() else {
                bail!(tr!(
                    "Cet agent n’est pas une discussion.",
                    "This agent is not a discussion."
                ));
            };
            edit(d)
        };
        self.emit_agent(&h);
        self.request_save();
        Ok(out)
    }

    /// « Commencer » of « Nouvelle discussion »: its agent made, in the folder of the docs branch,
    /// and sent its first message.
    pub async fn discussion_start(
        self: &Arc<Self>,
        project_id: &str,
        start: Start,
    ) -> Result<AgentView> {
        if start.subject.trim().is_empty() {
            bail!(tr!(
                "Donne un sujet à la discussion.",
                "Give the discussion a subject."
            ));
        }
        let project = self.project(project_id)?;
        let engine = start.engine.unwrap_or_else(|| project.engine());
        if !holds(engine) {
            bail!(tr!(
                "{harness} modifie les fichiers de son dossier sans passer par Escouade : une discussion ne part pas sur lui. Choisis Claude Code, Codex, OpenCode ou Copilot CLI.",
                "{harness} changes the files of its folder without going through Escouade: no discussion runs on it. Choose Claude Code, Codex, OpenCode or Copilot CLI.",
                harness = crate::engine::by_name(engine)
            ));
        }
        // The gap it is opened on, as the page has it now.
        let space = self.docs_space(project_id, None).await?;
        let dir = space.root.to_string_lossy().to_string();
        let settles = start.gap.as_ref().and_then(|g| {
            let page = space.spec_read(&g.page).ok()?;
            Some(Settles {
                gap: page.gaps.get(g.index)?.clone(),
                page: page.path,
            })
        });
        let lang = self.lang().claude;
        let agent = self
            .create_agent_with(
                project_id,
                AgentOptions {
                    name: Some(agent_name(&start.subject)),
                    engine: Some(engine),
                    account: start.account.clone(),
                    model: start.model.clone().filter(|m| !m.trim().is_empty()),
                    mode: Some(MODES[0].into()),
                    isolated: Some(false),
                    cwd: Some(dir),
                    // The code index's rule too, as any agent without a ticket gets it.
                    append_prompt: Some(match self.code_index_on(&project) {
                        true => format!(
                            "{} {}",
                            instructions(lang, start.kind, &project.name),
                            crate::codeindex::hint::rule(lang)
                        ),
                        false => instructions(lang, start.kind, &project.name),
                    }),
                    discussion: Some(Discussion {
                        kind: start.kind,
                        subject: start.subject.trim().to_string(),
                        settles,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await?;
        // Its first message refused (ESC-289): the agent made goes, the window says why and keeps
        // what was chosen, to start again or change its account; no discussion without it is left.
        let id = agent.meta.id.clone();
        if let Err(e) = self
            .send_message(&id, first_message(lang, &start), Vec::new())
            .await
        {
            log::warn!("discussion {id}: first message refused, its agent removed: {e:#}");
            if let Err(d) = self.delete_agent(&id, false).await {
                log::warn!("discussion {id}: not removed: {d:#}");
            }
            return Err(e);
        }
        Ok(self.agent(&id)?.lock().view())
    }

    /// « Verser dans la spec »: the pages and decisions the discussion `id` proposes written where
    /// the docs live (the decisions proposed in the register, for a human to accept), then
    /// forgotten from the discussion. Nothing is committed.
    pub async fn discussion_pour(&self, id: &str) -> Result<Poured> {
        let (project_id, d) = {
            let h = self.agent(id)?;
            let rt = h.lock();
            let Some(d) = rt.meta.discussion.clone() else {
                bail!(tr!(
                    "Cet agent n’est pas une discussion.",
                    "This agent is not a discussion."
                ));
            };
            (rt.meta.project_id.clone(), d)
        };
        if d.pages.is_empty() && d.decisions.is_empty() {
            bail!(tr!(
                "Rien à verser : la discussion ne propose encore aucun changement.",
                "Nothing to pour: the discussion proposes no change yet."
            ));
        }
        let space = self.docs_space(&project_id, Some(id)).await?;
        let mut pages = Vec::new();
        let mut changes = Vec::new();
        for c in &d.pages {
            let settled = d.settles.as_ref().filter(|s| s.page == c.path);
            let (page, added, removed) = space.spec_write(&c.path, |p| {
                *p = c.page.clone();
                // The gap it was opened on is settled by this change.
                if let Some(s) = settled {
                    p.gaps.retain(|g| g != &s.gap);
                }
            })?;
            changes.push(Change {
                page: page.path.clone(),
                added,
                removed,
            });
            pages.push(page.path);
        }
        let mut numbers = Vec::new();
        for dec in &d.decisions {
            let mut dec = dec.clone();
            // What it changed in the spec: its pages' changes poured with it, else all of them.
            if dec.changes.is_empty() {
                dec.changes = changes
                    .iter()
                    .filter(|c| dec.pages.is_empty() || dec.pages.contains(&c.page))
                    .cloned()
                    .collect();
            }
            let replaces = dec.supersedes.first().copied();
            dec.supersedes.clear();
            let made = match replaces {
                Some(old) => space.decision_replace(old, dec)?,
                None => space.decision_propose(None, dec)?,
            };
            numbers.push(made.reference());
        }
        let poured = self.discussion_edit(id, |disc| {
            if disc
                .settles
                .as_ref()
                .is_some_and(|s| pages.contains(&s.page))
            {
                disc.settles = None;
            }
            disc.pages.clear();
            disc.decisions.clear();
            let mut p = disc.poured.take().unwrap_or_default();
            p.at = crate::model::now_ms();
            for page in &pages {
                if !p.pages.contains(page) {
                    p.pages.push(page.clone());
                }
            }
            p.decisions.extend(numbers.iter().cloned());
            disc.poured = Some(p.clone());
            p
        })?;
        // What was poured is to plan (« Planifier », ESC-262).
        self.docs_to_plan(&project_id, &pages, &numbers);
        self.docs_changed(&project_id).await;
        Ok(poured)
    }

    /// Takes a proposal out of « Ce qui ira dans la spec »: the page `page`, or the decision at
    /// `decision` (from 1).
    pub fn discussion_drop(
        &self,
        id: &str,
        page: Option<&str>,
        decision: Option<usize>,
    ) -> Result<()> {
        self.discussion_edit(id, |d| {
            if let Some(path) = page {
                d.pages.retain(|p| p.path != path);
            }
            if let Some(i) = decision.filter(|i| *i >= 1 && *i <= d.decisions.len()) {
                d.decisions.remove(i - 1);
            }
        })
    }
}
