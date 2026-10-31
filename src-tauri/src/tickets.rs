//! The board on the core: tickets saved with the app, the scheduler that starts them with agents
//! of their own, what each end of turn does to them, their validation (tests, commit, merge, pull
//! request, push) and the test launches of the worktrees.

use crate::accounts;
use crate::agent::NotifyKind;
use crate::board::{self, TurnEnd};
use crate::core::{not_a_repo, project_not_found, target_branch_refusal, AgentOptions, Core};
use crate::engine::{self, Process};
use crate::git;
use crate::i18n::{self, Lang};
use crate::integrations;
use crate::isola;
use crate::leftovers;
use crate::model::*;
use crate::pace::Standing;
use crate::switching::NoOther;
use crate::telemetry::Event;
use crate::testlaunch;
use crate::ticketfiles;
use crate::ticketimages;
use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::Runtime;

/// Why a ticket "En cours" stops when its agent's automatic resume after the usage limit is gone
/// (turned off, cancelled, never planned): it would otherwise hold its place forever.
pub(crate) fn quota_lost(lang: Lang) -> String {
    tr_in!(lang, "limite d'usage atteinte", "usage limit reached")
}

/// Why the pass of the board did not start a ticket launched by hand: the ticket goes back to
/// « À faire » as it was, and the tool or the window that launched it says so.
#[derive(Debug, Clone)]
pub(crate) enum StartRefusal {
    /// A usage limit holds its account(s) back.
    Held(board::Held),
    /// Anything else, said as is.
    Text(String),
}

/// The reason a ticket is blocked by an error, on one line (a git error may run over many; the
/// whole error goes to the log): the contexts the app gave, then git's `fatal:` or `error:` line,
/// the one that tells why, when there is one; else the first line, as `board::turn_end` does with
/// an agent's.
pub(crate) fn error_reason(e: &anyhow::Error) -> String {
    board::error_blocked(i18n::ui(), &error_line(e))
}

/// The folder of a ticket, its images and its files go, off the async workers.
async fn remove_images(dir: PathBuf) {
    if let Err(e) = tokio::task::spawn_blocking(move || ticketimages::remove(&dir)).await {
        log::warn!("ticket images: not removed: {e}");
    }
}

/// The refusals of a ticket's validation and of an agent's test launch, as the window shows them
/// (in the interface's language when they are told).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Refusal {
    /// The block when the ticket left its validation (its agent archived or deleted meanwhile).
    Changed,
    /// The refusal while the ticket's agent works: its files may be half written.
    AgentBusy,
    /// "Renvoyer" refused while the ticket's agent works: the end of that turn would be read as
    /// the end of the rework.
    AgentBusyReject,
    /// An archived agent holds no ports and runs no test launch.
    Archived,
    /// Only an agent with a worktree has ports and a test launch.
    NoWorktree,
    /// The `.isola.toml` changed between what the user read and their « Lancer ».
    IsolaConfigChanged,
    /// The recipe changed between what the user read and their « Lancer ».
    RecipeChanged,
    /// No test launch while its ticket is validated: its servers would take the ports its tests
    /// use.
    Validating,
    /// Approving, sending back, dismissing: only a ticket « À tester ».
    NotInReview,
    /// A second « Valider » (or « Renvoyer », « L'agent résout ») while the first runs.
    ApprovingAlready,
    /// No validation starts on a project that is being closed: closing waits for none, and cuts
    /// what it finds under way.
    ProjectClosing,
    NoAgent,
    AgentWithoutWorktree,
}

impl Refusal {
    pub fn text(self, lang: Lang) -> String {
        match self {
            Refusal::Changed => tr_in!(
                lang,
                "Ce ticket a changé pendant sa validation.",
                "This ticket changed while it was being approved."
            ),
            Refusal::AgentBusy => tr_in!(
                lang,
                "L'agent de ce ticket travaille encore : attends la fin de son tour pour valider.",
                "This ticket’s agent is still working: wait for the end of its turn to approve it."
            ),
            Refusal::AgentBusyReject => tr_in!(
                lang,
                "L'agent de ce ticket travaille encore : attends la fin de son tour pour le renvoyer.",
                "This ticket’s agent is still working: wait for the end of its turn to send it back."
            ),
            Refusal::Archived => tr_in!(
                lang,
                "Cet agent est archivé : il n'a pas de lancement de test.",
                "This agent is archived: it has no test launch."
            ),
            Refusal::NoWorktree => tr_in!(
                lang,
                "Seul un agent à worktree a un lancement de test.",
                "Only an agent with a worktree has a test launch."
            ),
            Refusal::IsolaConfigChanged => tr_in!(
                lang,
                "Le .isola.toml a changé pendant que tu le lisais : relance « ▶ Tester » pour le relire.",
                "The .isola.toml changed while you were reading it: run “▶ Test” again to read it."
            ),
            Refusal::RecipeChanged => tr_in!(
                lang,
                "La recette a changé pendant que tu la lisais : relance « ▶ Tester » pour la relire.",
                "The recipe changed while you were reading it: run “▶ Test” again to read it."
            ),
            Refusal::Validating => tr_in!(
                lang,
                "Validation en cours : le lancement de test attendra sa fin.",
                "Approval under way: the test launch will wait until it’s over."
            ),
            Refusal::NotInReview => tr_in!(
                lang,
                "Ce ticket n'est pas à tester.",
                "This ticket isn’t in “To review”."
            ),
            Refusal::ApprovingAlready => tr_in!(
                lang,
                "La validation de ce ticket est déjà en cours.",
                "This ticket is already being approved."
            ),
            Refusal::ProjectClosing => tr_in!(
                lang,
                "Le projet de ce ticket est en cours de fermeture : la validation ne peut pas démarrer.",
                "This ticket’s project is being closed: the approval can’t start."
            ),
            Refusal::NoAgent => tr_in!(lang, "Ce ticket n'a pas d'agent.", "This ticket has no agent."),
            Refusal::AgentWithoutWorktree => tr_in!(
                lang,
                "L'agent de ce ticket n'a pas de worktree.",
                "This ticket’s agent has no worktree."
            ),
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text(i18n::ui()))
    }
}

impl std::error::Error for Refusal {}

/// The branch a ticket's form names, trimmed; empty for the ticket's own. Never an option.
fn branch_name(branch: &str) -> Result<String> {
    let branch = branch.trim();
    if branch.starts_with('-') {
        bail!(tr!(
            "« {branch} » n'est pas un nom de branche valide",
            "“{branch}” isn’t a valid branch name"
        ));
    }
    Ok(branch.to_string())
}

/// Why a ticket cannot start again on the branch it took up: the agent it had, archived, holds it in
/// its worktree, with changes that no start may throw away.
fn archived_holds(lang: Lang, agent: &str, branch: &str) -> String {
    tr_in!(
        lang,
        "L’agent {agent}, archivé, garde la branche « {branch} » dans son worktree avec des changements non commités : commite-les ou supprime l’agent, puis reprends le ticket.",
        "The archived agent {agent} holds the branch “{branch}” in its worktree with uncommitted changes: commit them or delete the agent, then resume the ticket."
    )
}

/// The model, the effort and the permission mode a ticket's agent goes with. The model and the
/// effort: its ticket's own (`start_ticket` gave it one), else its board's, else those of the
/// account that takes it (`defaults`, ESC-128). The mode: its board's, else the account's, but `default` instead of `auto` where the
/// model has no auto mode on the ticket's harness, as `takes_auto` says (`Core::takes_auto`: what
/// the `claude` of its account told, else the harness's rule; the Composer does not offer it
/// either: Claude Code's Haiku 4.5, OpenCode): the one place that rule applies.
pub(crate) fn ticket_agent_options(
    t: &Ticket,
    board: &BoardSettings,
    defaults: &accounts::Defaults,
    takes_auto: &dyn Fn(&str) -> bool,
) -> (String, String, String) {
    let first = |own: &str, board: &str, app: &str| {
        [own, board]
            .into_iter()
            .find(|v| !v.is_empty())
            .unwrap_or(app)
            .to_string()
    };
    let model = first(&t.model, &board.model, &defaults.model);
    let effort = first(&t.effort, &board.effort, &defaults.effort);
    let mut mode = first("", &board.mode, &defaults.mode);
    if mode == "auto" && !takes_auto(&model) {
        mode = "default".into();
    }
    (model, effort, mode)
}

fn ticket_not_found() -> anyhow::Error {
    anyhow!(tr!("ticket introuvable", "ticket not found"))
}

pub(crate) fn title_missing() -> anyhow::Error {
    anyhow!(tr!(
        "Un ticket a besoin d'un titre.",
        "A ticket needs a title."
    ))
}

pub(crate) fn comment_missing() -> anyhow::Error {
    anyhow!(tr!("Dis ce qui ne va pas.", "Say what’s wrong."))
}

/// The refusal of an edit to a ticket that has started: also said by the tools, before they read
/// the files of the images they were given.
pub(crate) fn only_todo_is_edited() -> anyhow::Error {
    anyhow!(tr!(
        "Seul un ticket « À faire » se modifie.",
        "Only a ticket in “To do” can be edited."
    ))
}

/// Why a ticket set on an account went back to « Auto » (ESC-153).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Lost {
    /// Removed from the settings.
    Gone,
    /// Switched off.
    Off,
    /// Not signed in any more.
    SignedOut,
    /// No longer one its project may use (`accounts::ticket_refusal`, said).
    Refused(String),
}

/// What the card of a ticket says of the account `name` it went back to « Auto » from.
pub(crate) fn lost_line(lang: Lang, name: &str, why: &Lost) -> String {
    match why {
        Lost::Gone => tr_in!(
            lang,
            "Le compte {name} n’existe plus : repassé en « Auto ».",
            "The account {name} no longer exists: back to “Auto”."
        ),
        Lost::Off => tr_in!(
            lang,
            "Le compte {name} est désactivé : repassé en « Auto ».",
            "The account {name} is switched off: back to “Auto”."
        ),
        Lost::SignedOut => tr_in!(
            lang,
            "Le compte {name} n’est pas connecté : repassé en « Auto ».",
            "The account {name} isn’t signed in: back to “Auto”."
        ),
        Lost::Refused(why) => tr_in!(lang, "{why} Repassé en « Auto ».", "{why} Back to “Auto”."),
    }
}

/// The notification of the same, for the ticket `key`.
pub(crate) fn lost_alert(lang: Lang, key: &str, name: &str, why: &Lost) -> String {
    match why {
        Lost::Gone => tr_in!(
            lang,
            "{key} : le compte {name} n’existe plus, repassé en « Auto »",
            "{key}: the account {name} no longer exists, back to “Auto”"
        ),
        Lost::Off => tr_in!(
            lang,
            "{key} : le compte {name} est désactivé, repassé en « Auto »",
            "{key}: the account {name} is switched off, back to “Auto”"
        ),
        Lost::SignedOut => tr_in!(
            lang,
            "{key} : le compte {name} n’est pas connecté, repassé en « Auto »",
            "{key}: the account {name} isn’t signed in, back to “Auto”"
        ),
        Lost::Refused(_) => tr_in!(
            lang,
            "{key} : le compte {name} ne sert plus ce projet, repassé en « Auto »",
            "{key}: the account {name} no longer serves this project, back to “Auto”"
        ),
    }
}

/// The notification of a ticket that reached « À tester », or got blocked (`blocked`, why).
pub(crate) fn ticket_alert(lang: Lang, key: &str, ready: bool, blocked: &str) -> String {
    if ready {
        tr_in!(lang, "{key} prêt à tester", "{key} ready to review")
    } else {
        tr_in!(lang, "{key} bloqué : {blocked}", "{key} blocked: {blocked}")
    }
}

/// Why a validation stopped on a conflict with `target`: the files named when the user is not
/// asked (`ask`) what to do.
pub(crate) fn conflict_reason(lang: Lang, target: &str, files: &[String], ask: bool) -> String {
    if ask {
        tr_in!(lang, "Conflit avec {target}", "Conflict with {target}")
    } else {
        let files = files.join(", ");
        board::first_line(&tr_in!(
            lang,
            "Conflit avec {target} sur : {files}",
            "Conflict with {target} on: {files}"
        ))
    }
}

/// An error on one line, as `error_reason` tells it: a failed validation step is blocked with it
/// as it is (the app's own refusals read as they are written).
pub(crate) fn error_line(e: &anyhow::Error) -> String {
    let layers: Vec<String> = e
        .chain()
        .map(|c| c.to_string())
        .filter(|l| !l.trim().is_empty())
        .collect();
    let says_why = |l: &&str| l.contains("fatal:") || l.contains("error:");
    for (i, layer) in layers.iter().enumerate() {
        if let Some(why) = layer.lines().map(str::trim).find(says_why) {
            let mut parts: Vec<String> = layers[..i].iter().map(|l| board::first_line(l)).collect();
            parts.push(why.to_string());
            return board::first_line(&parts.join(": "));
        }
    }
    board::first_line(&format!("{e:#}"))
}

/// The tests run before a validation stop after this.
const TEST_LIMIT: Duration = Duration::from_secs(20 * 60);

/// A ticket as its form gives it.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TicketDraft {
    pub title: String,
    pub description: String,
    /// One per line of the form; empty lines are dropped.
    pub criteria: Vec<String>,
    pub max_loops: u32,
    /// The ids of the tickets it comes after ("Après"), the whole list.
    pub after: Vec<String>,
    /// An existing branch for its agent to take up; empty: the ticket's own, `ticket/<key>`.
    pub branch: String,
    /// Its images, the whole list, in their order; None: they stay as they are (the tools of the
    /// MCP server, the imports).
    pub images: Option<Vec<DraftImage>>,
    /// Its files other than images, the whole list, in their order; None: they stay as they are
    /// (the tools of the MCP server, the imports).
    pub files: Option<Vec<DraftFile>>,
    /// Its harness (« Harness » in its form): Some(None), the project's; None, absent, it stays
    /// as it is (the tools of the MCP server, the imports).
    #[serde(deserialize_with = "given")]
    pub engine: Option<Option<EngineId>>,
    /// Its account, model and effort (« Compte », « Modèle », « Effort » in its form, ESC-153):
    /// empty, « Auto »; None, absent, they stay as they are.
    pub account: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// « Partager avec l'équipe » (ESC-190): shared, or no longer; None, absent: as it is (a new
    /// ticket: as « Partager chaque nouveau ticket » of its project says).
    pub shared: Option<bool>,
    /// The pages and decisions it implements (« Planifier », ESC-262); None: as they are.
    pub pages: Option<Vec<String>>,
    pub decisions: Option<Vec<crate::docs::Ref>>,
}

/// A field that is there, null or not: Some, its value (null: None); absent, it is None by its
/// `default`.
fn given<'de, D, T>(d: D) -> std::result::Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// A field of a form trimmed, empty when it is not given.
fn trimmed(v: Option<&str>) -> String {
    v.unwrap_or_default().trim().to_string()
}

/// An image of a ticket's form: one the ticket has (`file`, in its folder), or one pasted or
/// dropped (`data`, in base64).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DraftImage {
    pub name: String,
    pub media_type: String,
    pub file: String,
    pub data: String,
}

/// A file of a ticket's form: one the ticket has (`file`, `<uniq>/<name>` in its `files` folder;
/// no `path`), or a copy the form made in the drafts (`path`; no `file`).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DraftFile {
    pub name: String,
    pub file: String,
    pub path: String,
}

impl<R: Runtime> Core<R> {
    // ---------- tickets ----------

    pub fn ticket(&self, id: &str) -> Result<Ticket> {
        self.tickets
            .read()
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(ticket_not_found)
    }

    /// Changes a ticket under the lock, then tells the window and saves. `f` runs with the tickets
    /// locked: it must not take the projects' lock (projects come before tickets).
    pub(crate) fn edit_ticket<T>(
        &self,
        id: &str,
        f: impl FnOnce(&mut Ticket) -> Result<T>,
    ) -> Result<T> {
        self.edit_ticket_with(id, |_| Ok(()), |t, ()| f(t))
    }

    /// `edit_ticket`, `f` given what `check` makes of all the tickets first, under the same lock:
    /// what it found among the others (the tickets it waits for) cannot change in between.
    pub(crate) fn edit_ticket_with<C, T>(
        &self,
        id: &str,
        check: impl FnOnce(&[Ticket]) -> Result<C>,
        f: impl FnOnce(&mut Ticket, C) -> Result<T>,
    ) -> Result<T> {
        let (out, ticket, change, column) = {
            let mut tickets = self.tickets.write();
            let i = tickets
                .iter()
                .position(|t| t.id == id)
                .ok_or_else(ticket_not_found)?;
            let checked = check(&tickets)?;
            let t = &mut tickets[i];
            let before = t.external.is_some().then(|| t.clone());
            let column = t.column;
            let out = f(t, checked)?;
            let change = before.and_then(|b| integrations::sync::change_of(&b, t));
            (out, t.clone(), change, column)
        };
        if ticket.column != column {
            self.facts.emit(
                crate::facts::Body::TicketColumn {
                    from: crate::facts::column_word(column),
                    to: crate::facts::column_word(ticket.column),
                },
                crate::facts::About::ticket(&ticket.id),
            );
        }
        if let Some(change) = change {
            self.sync_external(ticket.clone(), change);
        }
        self.hub.emit(UiEvent::Ticket { ticket });
        self.request_save();
        Ok(out)
    }

    fn emit_project(&self, project_id: &str) {
        if let Ok(project) = self.project(project_id) {
            self.hub.emit(UiEvent::Project { project });
        }
    }

    /// A ticket "À faire" at the end of the column. The first one fixes the key's prefix and the
    /// target branch when not set: the one the project is on, even without any commit yet (a new
    /// repository); on a detached HEAD, none, so the ticket is refused until one is chosen.
    pub async fn ticket_create(
        self: &Arc<Self>,
        project_id: &str,
        d: TicketDraft,
    ) -> Result<Ticket> {
        self.ticket_create_with(project_id, d, None).await
    }

    /// `ticket_create` of a ticket imported from `external`: it is in step with it from the start
    /// (the scheduler may start it at once).
    pub(crate) async fn ticket_create_with(
        self: &Arc<Self>,
        project_id: &str,
        d: TicketDraft,
        external: Option<ExternalRef>,
    ) -> Result<Ticket> {
        let title = d.title.trim().to_string();
        if title.is_empty() {
            return Err(title_missing());
        }
        let project = self.project(project_id)?;
        // Shared from the start (ESC-190): asked, or the project's « Partager chaque nouveau
        // ticket » when this machine's user gave a name. Asked without a name: refused first.
        let share = d
            .shared
            .unwrap_or(project.share_new && !self.me_in(&project).name.is_empty());
        if d.shared == Some(true) && self.me_in(&project).name.is_empty() {
            return Err(crate::sharing::name_missing());
        }
        // Refused before a key is used up.
        let engine = d.engine.flatten().unwrap_or(project.engine());
        let account = self.ticket_account(&project, engine, d.account.as_deref())?;
        let branch = self.ticket_branch(&project, &d.branch).await?;
        let id = new_id();
        let dir = self.data.ticket_dir(&id);
        let checked = ticketimages::check(&dir, &[], d.images.as_deref().unwrap_or_default())?;
        let files = ticketfiles::check(
            &self.data.drafts_files(),
            &dir,
            &[],
            d.files.as_deref().unwrap_or_default(),
            self.attach_cap(),
        )?;
        let current = git::head_branch(&project.path).await;
        // Refused here, before a file is moved out of the drafts, when the key will be refused for
        // want of a branch (below, under the lock): a copy moved for a ticket that is not made is
        // gone, and the form's chip would point at nothing when the user tries again.
        // Of several repositories (ESC-169): each one's branch, none for the project.
        if project.board.target.is_empty() && current.is_empty() && !project.is_multi() {
            bail!(board::no_branch(i18n::ui()));
        }
        // Written before a key is used up too (a disk that refuses them takes none), and off the
        // async workers.
        let images = if checked.is_empty() {
            Vec::new()
        } else {
            let dir = dir.clone();
            tokio::task::spawn_blocking(move || ticketimages::write(&dir, checked)).await??
        };
        // The same for the files, moved out of the drafts: one that cannot be leaves what the
        // images wrote no more than it leaves itself.
        let files = if files.is_empty() {
            Vec::new()
        } else {
            let folder = dir.clone();
            let placed = tokio::task::spawn_blocking(move || ticketfiles::place(&folder, files))
                .await
                .map_err(anyhow::Error::from)
                .and_then(|placed| placed);
            match placed {
                Ok(files) => files,
                Err(e) => {
                    remove_images(dir).await;
                    return Err(e);
                }
            }
        };
        let key = (|| -> Result<String> {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(project_not_found)?;
            if p.board.target.is_empty() && !p.is_multi() {
                if current.is_empty() {
                    bail!(board::no_branch(i18n::ui()));
                }
                p.board.target = current;
            }
            if p.board.prefix.is_empty() {
                p.board.prefix = board::key_prefix(&p.name);
            }
            let n = p.board.next_number.max(1);
            p.board.next_number = n + 1;
            Ok(format!("{}-{n}", p.board.prefix))
        })();
        let key = match key {
            Ok(key) => key,
            Err(e) => {
                remove_images(dir).await;
                return Err(e);
            }
        };
        let mut ticket = Ticket {
            id,
            project_id: project_id.to_string(),
            key,
            title,
            description: d.description.trim().to_string(),
            criteria: board::criteria_from(&d.criteria),
            max_loops: board::max_loops(d.max_loops),
            created_at: now_ms(),
            external,
            branch,
            images,
            files,
            engine: d.engine.flatten(),
            account,
            model: trimmed(d.model.as_deref()),
            effort: trimmed(d.effort.as_deref()),
            pages: d.pages.clone().unwrap_or_default(),
            decisions: d.decisions.clone().unwrap_or_default(),
            ..Default::default()
        };
        let pushed = {
            // Lock order: projects, then tickets. The project is checked again while both are
            // held (the rank and the push in one go): `remove_project`'s write of the projects
            // waits for this, and it takes the project's tickets once it is out.
            let projects = self.projects.read();
            if projects.iter().any(|p| p.id == project_id) {
                let mut tickets = self.tickets.write();
                ticket.rank = tickets
                    .iter()
                    .filter(|t| t.project_id == project_id)
                    .map(|t| t.rank)
                    .max()
                    .unwrap_or(0)
                    + 1;
                // No ticket waits for a new one yet: it closes no loop.
                ticket.after = board::after_of(&tickets, project_id, &ticket.id, &d.after);
                tickets.push(ticket.clone());
                true
            } else {
                false
            }
        };
        if !pushed {
            // Not under the locks: the folder goes off the async workers.
            remove_images(dir).await;
            return Err(project_not_found());
        }
        self.emit_project(project_id);
        self.hub.emit(UiEvent::Ticket {
            ticket: ticket.clone(),
        });
        self.facts.emit(
            crate::facts::Body::TicketCreated {
                column: crate::facts::column_word(ticket.column),
            },
            crate::facts::About::ticket(&ticket.id),
        );
        self.request_save();
        self.schedule();
        if share {
            return self.ticket_share(&ticket.id, true);
        }
        Ok(ticket)
    }

    /// The account a ticket's form or a tool names (« Compte », ESC-153), trimmed: empty for
    /// « Auto » (and when none is named), else one its project may use on `engine`
    /// (`accounts::ticket_refusal` says why not).
    fn ticket_account(
        &self,
        project: &Project,
        engine: EngineId,
        asked: Option<&str>,
    ) -> Result<String> {
        let id = trimmed(asked);
        if id.is_empty() {
            return Ok(id);
        }
        match accounts::ticket_refusal(&self.settings.read(), project, engine, &id) {
            Some(refusal) => Err(refusal),
            None => Ok(id),
        }
    }

    /// The branch a ticket's form names, trimmed: empty for the ticket's own, else one the
    /// repository has (a local branch, or a remote one), and not the board's target. It may be
    /// taken by then, or be the folder's: the start refuses it, as an agent made on it is.
    async fn ticket_branch(&self, project: &Project, branch: &str) -> Result<String> {
        let branch = branch_name(branch)?;
        // Of several repositories (ESC-169): taken up where it exists, made elsewhere.
        if branch.is_empty() || project.is_multi() {
            return Ok(branch);
        }
        let root = self.toplevel(&project.path).await.ok_or_else(not_a_repo)?;
        let local = git::local_name(&root, &branch).await?;
        let target = project.board.target.trim();
        if !target.is_empty() && local == target {
            bail!(target_branch_refusal(i18n::ui()));
        }
        Ok(branch)
    }

    /// Only a ticket "À faire" changes. It never comes after a ticket that waits for it already.
    pub fn ticket_update(self: &Arc<Self>, id: &str, d: TicketDraft) -> Result<Ticket> {
        let title = d.title.trim().to_string();
        if title.is_empty() {
            return Err(title_missing());
        }
        // (Its existence is the start's to check: it needs git, this does not wait for it. So is a
        // remote branch that would make the target's local one.)
        let branch = branch_name(&d.branch)?;
        let target = self
            .ticket(id)
            .and_then(|t| self.project(&t.project_id))
            .map(|p| p.board.target)
            .unwrap_or_default();
        if !branch.is_empty() && branch == target.trim() {
            bail!(target_branch_refusal(i18n::ui()));
        }
        // Its account, on the harness it will have: one named is checked; a harness changed that
        // the account it had does not run sets it back to « Auto », as the form does.
        let account = {
            let had = self.ticket(id)?;
            let project = self.project(&had.project_id)?;
            let engine = d.engine.unwrap_or(had.engine).unwrap_or(project.engine());
            match d.account.as_deref() {
                Some(asked) => Some(self.ticket_account(&project, engine, Some(asked))?),
                None if !had.account.is_empty()
                    && accounts::find(&self.settings.read(), &had.account)
                        .is_some_and(|a| a.engine != engine) =>
                {
                    Some(String::new())
                }
                None => None,
            }
        };
        // The images pasted are written first, out of the lock, and the files joined moved out of
        // the drafts. Then only this save's files go: those it wrote when it is refused, those it
        // took away once it is done. Everything is checked before anything is written.
        let dir = self.data.ticket_dir(id);
        // A ticket that left « À faire » is refused before a copy is moved out of the drafts: one
        // moved for a save that fails is gone, and the form's chip would point at nothing.
        if d.files.is_some() && self.ticket(id)?.column != Column::Todo {
            bail!(tr!(
                "Seul un ticket « À faire » se modifie.",
                "Only a ticket in “To do” can be edited."
            ));
        }
        let checked_files = match &d.files {
            Some(list) => {
                let had = self.ticket(id)?.files;
                let checked = ticketfiles::check(
                    &self.data.drafts_files(),
                    &dir,
                    &had,
                    list,
                    self.attach_cap(),
                )?;
                Some((checked, had))
            }
            None => None,
        };
        let (images, written) = match &d.images {
            Some(list) => {
                let had = self.ticket(id)?.images;
                let images = ticketimages::write(&dir, ticketimages::check(&dir, &had, list)?)?;
                let written = images
                    .iter()
                    .filter(|i| !had.iter().any(|h| h.file == i.file))
                    .map(|i| i.file.clone())
                    .collect();
                (Some(images), written)
            }
            None => (None, Vec::new()),
        };
        let (files, placed) = match checked_files {
            Some((checked, had)) => {
                match ticketfiles::place(&dir, checked) {
                    Ok(files) => {
                        let placed = files
                            .iter()
                            .filter(|f| !had.iter().any(|h| h.file == f.file))
                            .map(|f| f.file.clone())
                            .collect();
                        (Some(files), placed)
                    }
                    Err(e) => {
                        // The images this save wrote go as they do when it is refused.
                        ticketimages::let_go(&dir, &written);
                        return Err(e);
                    }
                }
            }
            None => (None, Vec::new()),
        };
        let saved = self.edit_ticket_with(
            id,
            |all| {
                let me = all
                    .iter()
                    .find(|t| t.id == id)
                    .ok_or_else(ticket_not_found)?;
                if me.column != Column::Todo {
                    return Err(only_todo_is_edited());
                }
                let after = board::after_of(all, &me.project_id, id, &d.after);
                match board::cycle_refusal(i18n::ui(), all, id, &after) {
                    Some(refusal) => bail!(refusal),
                    None => Ok(after),
                }
            },
            |t, after| {
                t.title = title;
                t.description = d.description.trim().to_string();
                t.criteria = board::criteria_from(&d.criteria);
                t.max_loops = board::max_loops(d.max_loops);
                t.after = after;
                t.branch = branch;
                if let Some(engine) = d.engine {
                    t.engine = engine;
                }
                if let Some(account) = account {
                    t.account = account;
                }
                if let Some(model) = &d.model {
                    t.model = model.trim().to_string();
                }
                if let Some(effort) = &d.effort {
                    t.effort = effort.trim().to_string();
                }
                // Saved again: what its card said of an account it lost is old news.
                t.account_lost = None;
                let before = images.map(|i| std::mem::replace(&mut t.images, i));
                let files_before = files.map(|f| std::mem::replace(&mut t.files, f));
                Ok((t.clone(), before, files_before))
            },
        );
        match &saved {
            Ok((t, before, files_before)) => {
                if let Some(before) = before {
                    let gone: Vec<String> = before
                        .iter()
                        .filter(|b| !t.images.iter().any(|i| i.file == b.file))
                        .map(|b| b.file.clone())
                        .collect();
                    ticketimages::let_go(&dir, &gone);
                }
                if let Some(before) = files_before {
                    let gone: Vec<String> = before
                        .iter()
                        .filter(|b| !t.files.iter().any(|f| f.file == b.file))
                        .map(|b| b.file.clone())
                        .collect();
                    ticketfiles::let_go(&dir, &gone);
                }
            }
            Err(_) => {
                ticketimages::let_go(&dir, &written);
                ticketfiles::let_go(&dir, &placed);
            }
        }
        let (ticket, ..) = saved?;
        self.schedule();
        match d.shared {
            Some(on) if on != ticket.shared.is_some() => self.ticket_share(&ticket.id, on),
            _ => Ok(ticket),
        }
    }

    /// The images of the ticket, with their content: for its form, edited, to show.
    pub fn ticket_images(&self, id: &str) -> Result<Vec<ticketimages::ImageData>> {
        let t = self.ticket(id)?;
        Ok(ticketimages::read(&self.data.ticket_dir(id), &t.images))
    }

    /// The images of the ticket, as its agent's first message carries them, and the names of those
    /// it can't (their file is gone); read off the async workers (up to 18 MB of files).
    async fn ticket_attachments(&self, t: &Ticket) -> (Vec<crate::core::Attachment>, Vec<String>) {
        if t.images.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let (dir, images) = (self.data.ticket_dir(&t.id), t.images.clone());
        tokio::task::spawn_blocking(move || ticketimages::attachments(&dir, &images))
            .await
            .unwrap_or_else(|e| {
                log::warn!("ticket {}: images not read: {e}", t.key);
                // None was read: all of them are missing.
                (
                    Vec::new(),
                    t.images.iter().map(|i| i.name.clone()).collect(),
                )
            })
    }

    /// "Passer en tête": only a ticket "À faire" has a rank to change.
    pub fn ticket_prioritize(self: &Arc<Self>, id: &str) -> Result<()> {
        let t = self.ticket(id)?;
        let first = self
            .tickets
            .read()
            .iter()
            .filter(|x| x.project_id == t.project_id && x.column == Column::Todo)
            .map(|x| x.rank)
            .min()
            .unwrap_or(0);
        self.edit_ticket(id, |t| {
            if t.column != Column::Todo {
                bail!(tr!(
                    "Seul un ticket « À faire » passe en tête.",
                    "Only a ticket in “To do” can move to the top."
                ));
            }
            t.rank = first - 1;
            Ok(())
        })?;
        self.schedule();
        Ok(())
    }

    /// Puts a ticket « À faire » at the top or the bottom of its project's column, or just before
    /// another of them (`board::new_ranks`): only a ticket « À faire » has a place to change.
    pub fn ticket_move(self: &Arc<Self>, id: &str, to: board::MoveTo<'_>) -> Result<()> {
        let changed: Vec<Ticket> = {
            let mut tickets = self.tickets.write();
            let me = tickets
                .iter()
                .find(|t| t.id == id)
                .ok_or_else(ticket_not_found)?;
            if me.column != Column::Todo {
                bail!(tr!(
                    "Seul un ticket « À faire » se déplace.",
                    "Only a ticket in “To do” can be moved."
                ));
            }
            let project_id = me.project_id.clone();
            if let board::MoveTo::Before(target) = to {
                if target == id {
                    bail!(tr!(
                        "Un ticket ne se place pas avant lui-même.",
                        "A ticket cannot be put before itself."
                    ));
                }
                let target = tickets
                    .iter()
                    .find(|t| t.id == target)
                    .ok_or_else(ticket_not_found)?;
                if target.project_id != project_id {
                    bail!(tr!(
                        "{key} est dans un autre projet.",
                        "{key} is in another project.",
                        key = target.key
                    ));
                }
                if target.column != Column::Todo {
                    bail!(tr!(
                        "{key} n’est pas « À faire » : un ticket ne se place qu’avant un ticket « À faire ».",
                        "{key} isn’t in “To do”: a ticket can only be put before a ticket in “To do”.",
                        key = target.key
                    ));
                }
            }
            let ranks = board::new_ranks(&tickets, id, to);
            let mut changed = Vec::with_capacity(ranks.len());
            for (ticket_id, rank) in ranks {
                if let Some(t) = tickets.iter_mut().find(|t| t.id == ticket_id) {
                    t.rank = rank;
                    changed.push(t.clone());
                }
            }
            changed
        };
        if changed.is_empty() {
            return Ok(());
        }
        for ticket in changed {
            self.hub.emit(UiEvent::Ticket { ticket });
        }
        self.request_save();
        self.schedule();
        Ok(())
    }

    /// "Lancer": a ticket that starts even with the autopilot off.
    pub fn ticket_start(self: &Arc<Self>, id: &str) -> Result<()> {
        self.ticket_start_with(id, None, None)
    }

    /// `ticket_start`, giving the ticket's agent a model and an effort of its own (the MCP tool's
    /// `start_ticket`): `Some` writes it on the ticket, an empty one going back to the board's,
    /// `None` leaves it as it is. Nothing is written for a ticket that has started already.
    pub fn ticket_start_with(
        self: &Arc<Self>,
        id: &str,
        model: Option<String>,
        effort: Option<String>,
    ) -> Result<()> {
        self.ticket_start_on(id, None, model, effort)
    }

    /// `ticket_start_with` on `account` too (`start_ticket`): given, it is the ticket's from now
    /// on, as its model and its effort are; empty, « Auto ».
    pub(crate) fn ticket_start_on(
        self: &Arc<Self>,
        id: &str,
        account: Option<String>,
        model: Option<String>,
        effort: Option<String>,
    ) -> Result<()> {
        // A project no account serves starts nothing, nor later on its own once one does.
        let ticket = self.ticket(id)?;
        let project = ticket.project_id.clone();
        let engine = self.ticket_engine(&ticket);
        let account = match account {
            Some(asked) => {
                let p = self.project(&project)?;
                Some(self.ticket_account(&p, engine, Some(&asked))?)
            }
            None => None,
        };
        // No account of its harness at all (ESC-128): one is to be added first.
        if !accounts::has_any(&self.settings.read(), engine) {
            return Err(accounts::no_account_of(engine));
        }
        if !accounts::serves_any(&self.settings.read(), engine, Some(&project)) {
            let name = self.project(&project).map_or(project, |p| p.name);
            return Err(accounts::no_account(&name, engine));
        }
        self.edit_ticket(id, |t| {
            if t.column != Column::Todo {
                bail!(tr!(
                    "Ce ticket est déjà parti.",
                    "This ticket has already started."
                ));
            }
            t.forced = true;
            if let Some(account) = account {
                t.account = account;
                t.account_lost = None;
            }
            if let Some(model) = model {
                t.model = model;
            }
            if let Some(effort) = effort {
                t.effort = effort;
            }
            Ok(())
        })?;
        self.start_refusals.lock().remove(id);
        self.telemetry
            .track(Event::TicketStarted { autopilot: false });
        self.schedule();
        Ok(())
    }

    /// What became of a ticket launched by hand, waited for up to `within`: `Ok(true)` once it
    /// left « À faire », `Err` with the reason when the pass did not start it, `Ok(false)` when
    /// the pass has not come to it yet (behind another launch under way): it stays queued.
    pub(crate) async fn wait_started(
        &self,
        id: &str,
        within: Duration,
    ) -> Result<bool, StartRefusal> {
        let end = tokio::time::Instant::now() + within;
        loop {
            if let Some(why) = self.start_refusals.lock().remove(id) {
                return Err(why);
            }
            match self.ticket(id) {
                Ok(t) if t.column == Column::Todo => {}
                _ => return Ok(true),
            }
            if tokio::time::Instant::now() >= end {
                return Ok(false);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// The ticket goes, and no other waits for it any more; one "En cours" or "À tester" has its
    /// agent archived, one "Terminé" too when it was left as it is in its worktree and the board
    /// removes the worktrees. With that setting, the worktrees of all its agents then go, but
    /// those holding work nothing else keeps (`Core::clean_agent_worktree`, which says it on
    /// their card). The card is gone for good: an agent that cannot be archived is logged, not
    /// reported.
    pub async fn ticket_delete(self: &Arc<Self>, id: &str) -> Result<()> {
        let (t, freed) = {
            let mut tickets = self.tickets.write();
            let i = tickets
                .iter()
                .position(|x| x.id == id)
                .ok_or_else(ticket_not_found)?;
            let t = tickets.remove(i);
            let freed: Vec<Ticket> = tickets
                .iter_mut()
                .filter(|x| x.after.contains(&t.id))
                .map(|x| {
                    x.after.retain(|a| *a != t.id);
                    x.clone()
                })
                .collect();
            (t, freed)
        };
        self.hub.emit(UiEvent::TicketRemoved {
            id: id.to_string(),
            project_id: t.project_id.clone(),
        });
        // Shared: its file goes too, to be committed (« Supprimer pour l'équipe », ESC-190).
        if let (Some(s), Ok(p)) = (&t.shared, self.project(&t.project_id)) {
            crate::sharing::remove_file_of(&crate::sharing::dir_of(&p), &s.id);
        }
        self.request_save();
        if !t.images.is_empty() || !t.files.is_empty() {
            // The state without the ticket is on disk before its images and files go: a crash in
            // between would bring it back without them. Off the async workers, as the saver does.
            let c = self.clone();
            if let Err(e) = tokio::task::spawn_blocking(move || c.save_now()).await {
                log::warn!("{}: state not saved before its files go: {e}", t.key);
            }
        }
        remove_images(self.data.ticket_dir(id)).await;
        // The copy of the docs its agent wrote on a fixed branch, never poured (ESC-263).
        self.docs_drop_ticket(&t.project_id, &t.key);
        for ticket in freed {
            self.hub.emit(UiEvent::Ticket { ticket });
        }
        let cleanup = self.project(&t.project_id).is_ok_and(|p| p.board.cleanup);
        let under_way = matches!(t.column, Column::Doing | Column::Review);
        let mut ours = None;
        if under_way || cleanup {
            if let Some(h) = t.agent_id.as_deref().and_then(|a| self.agent(a).ok()) {
                let (a, proc, ports, archived, worktree) = {
                    let rt = h.lock();
                    let m = &rt.meta;
                    let worktree = m
                        .worktree
                        .as_ref()
                        .is_some_and(|w| Path::new(&w.path).is_dir());
                    (
                        m.id.clone(),
                        rt.proc.clone(),
                        m.port_base,
                        m.archived,
                        worktree,
                    )
                };
                // One left as it is (« Terminé »): archived only for its worktree to go.
                if !archived && (under_way || worktree) {
                    if let Err(e) = self.archive_agent(&a, true).await {
                        log::warn!("{}: its agent could not be archived: {e:#}", t.key);
                    }
                }
                ours = Some((a, proc, ports));
            }
        }
        self.schedule();
        if cleanup {
            let mut except = String::new();
            if let Some((a, proc, ports)) = ours {
                self.clean_agent_worktree(&a, proc, ports).await;
                except = a;
            }
            self.clean_ticket_worktrees(&t.id, &except).await;
        }
        Ok(())
    }

    /// The board's settings as the window sends them: the key's prefix and number stay the
    /// backend's, so does the target when the window has none (its copy is older than the first
    /// ticket), a choice it does not know falls back to the default, and the agents in parallel
    /// stay between 1 and 6.
    pub fn board_set(self: &Arc<Self>, project_id: &str, s: BoardSettings) -> Result<Project> {
        let default = BoardSettings::default();
        let kept;
        let one_of = |v: String, known: &[&str], fallback: String| {
            if known.contains(&v.as_str()) {
                v
            } else {
                fallback
            }
        };
        let project = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(project_not_found)?;
            let before = p.clone();
            let target = s.target.trim();
            p.board = BoardSettings {
                prefix: p.board.prefix.clone(),
                next_number: p.board.next_number,
                target: if target.is_empty() {
                    p.board.target.clone()
                } else {
                    target.to_string()
                },
                action: one_of(s.action, &["merge", "pr", "push", "keep"], default.action),
                strategy: one_of(s.strategy, &["merge", "squash", "rebase"], default.strategy),
                conflict: one_of(s.conflict, &["ask", "agent", "abort"], default.conflict),
                max_parallel: s.max_parallel.clamp(1, 6),
                ..s
            };
            // What the window set of its `.escouade.yml`: kept on this machine, ahead of the file.
            kept = crate::projectfile::keep_local(&before, p).len();
            p.clone()
        };
        if kept > 0 {
            self.kept_on_machine(&project, kept);
        }
        self.hub.emit(UiEvent::Project {
            project: project.clone(),
        });
        self.request_save();
        self.schedule();
        Ok(project)
    }

    pub async fn git_branches(&self, project_id: &str) -> Result<Vec<String>> {
        let p = self.project(project_id)?;
        git::branches(&p.path).await
    }

    /// The project is closed: its tickets leave the board, and what its board said. They are
    /// given back, for its history to keep (no file of theirs goes here: `erase_history_files`).
    pub(crate) fn drop_project_tickets(&self, project_id: &str) -> Vec<Ticket> {
        self.board_issues.lock().remove(project_id);
        let gone: Vec<Ticket> = {
            let mut tickets = self.tickets.write();
            let gone = tickets
                .iter()
                .filter(|t| t.project_id == project_id)
                .cloned()
                .collect();
            tickets.retain(|t| t.project_id != project_id);
            gone
        };
        for t in &gone {
            self.hub.emit(UiEvent::TicketRemoved {
                id: t.id.clone(),
                project_id: project_id.to_string(),
            });
        }
        gone
    }

    // ---------- scheduler ----------

    /// Starts what may start, in the background.
    pub fn schedule(self: &Arc<Self>) {
        #[cfg(test)]
        self.passes_queued.fetch_add(1, Ordering::SeqCst);
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            c.schedule_now().await;
            #[cfg(test)]
            c.passes_queued.fetch_sub(1, Ordering::SeqCst);
        });
    }

    /// The accounts an agent of the app waits for the quota of (usage limit), each with when its
    /// first one resumes: no ticket starts on them meanwhile, but on another account one may.
    pub(crate) fn quota_paused(&self) -> BTreeMap<String, i64> {
        let settings = self.settings.read().clone();
        let mut waiting = BTreeMap::new();
        for h in self.agents.read().values() {
            let rt = h.lock();
            let Some(at) = rt.meta.resume_at.filter(|_| !rt.meta.archived) else {
                continue;
            };
            let first = waiting
                .entry(accounts::get(&settings, &rt.meta.account).id)
                .or_insert(at);
            *first = (*first).min(at);
        }
        waiting
    }

    /// The time the autopilot's pauses go by: the real one (ahead by `clock_ahead` in tests).
    pub(crate) fn pause_now(&self) -> i64 {
        #[cfg(test)]
        return now_ms() + self.clock_ahead.load(Ordering::SeqCst);
        #[cfg(not(test))]
        now_ms()
    }

    /// The harness a ticket's agent runs on: the ticket's, else its project's default one.
    pub(crate) fn ticket_engine(&self, ticket: &Ticket) -> EngineId {
        ticket.engine.unwrap_or_else(|| {
            self.project(&ticket.project_id)
                .map_or_else(|_| engine::default().id(), |p| p.engine())
        })
    }

    /// Why no ticket of a project that goes to any account starts now, if none does
    /// (`board::autopilot_pause`): the quota windows last read, the pause after a usage limit,
    /// what "Reprendre maintenant" lifted, for every active account of the default harness. Each
    /// harness pauses on its own (`project_hold`); one without a quota never does.
    pub(crate) fn autopilot_pause(&self) -> Option<AutopilotPause> {
        let engine = engine::default().id();
        let settings = accounts::for_project(&self.settings.read(), engine, None);
        self.pause_for(&settings, None)
    }

    /// `autopilot_pause` for the projects that go to the account `preferred` (any, when none or
    /// not an active one), with `settings` as a project sees them (`accounts::for_project`).
    fn pause_for(&self, settings: &Settings, preferred: Option<&str>) -> Option<AutopilotPause> {
        let threshold = board::quota_threshold(settings.quota_pause);
        let accounts = accounts::candidates(settings, preferred);
        let usage = self.usage.lock().accounts.clone();
        let hold = self.hold.lock().clone();
        let waiting = self.quota_paused();
        board::autopilot_pause(
            &usage,
            &accounts,
            threshold,
            &hold,
            &waiting,
            self.pause_now(),
        )
    }

    /// Why the tickets of the projects that prefer an account, or that have accounts reserved to
    /// them, or another default harness, do not start, by project: each waits for the accounts of
    /// its own harness.
    pub(crate) fn project_pauses(&self) -> BTreeMap<String, AutopilotPause> {
        let all = self.settings.read().clone();
        let default = engine::default().id();
        let any = accounts::candidates(&accounts::for_project(&all, default, None), None);
        let projects = self.projects.read().clone();
        projects
            .iter()
            .filter_map(|p| {
                let settings = accounts::for_project_of(&all, p.engine(), p);
                let preferred = accounts::preferred(&settings, &p.account);
                if preferred.is_none() && accounts::candidates(&settings, None) == any {
                    return None;
                }
                Some((p.id.clone(), self.pause_for(&settings, preferred)?))
            })
            .collect()
    }

    /// The account a new agent of the project on `engine` goes to (a ticket's or not): the one
    /// the project prefers when it is an active one of that harness, else the first usable one of
    /// it, in the order of the accounts: never one of another harness. Refused when no active
    /// account of the harness serves the project (all reserved to others, Principal too).
    pub(crate) fn account_for_new(&self, project_id: &str, engine: EngineId) -> Result<String> {
        let all = self.settings.read().clone();
        let project = self.project(project_id).ok();
        // No account of the harness at all (ESC-128): one is to be added first.
        if !accounts::has_any(&all, engine) {
            return Err(accounts::no_account_of(engine));
        }
        if !accounts::serves_any(&all, engine, Some(project_id)) {
            let name = project.map_or_else(|| project_id.to_string(), |p| p.name);
            return Err(accounts::no_account(&name, engine));
        }
        let settings = self.project_accounts(&all, engine, project_id);
        let preferred =
            project.and_then(|p| accounts::preferred(&settings, &p.account).map(str::to_string));
        self.pick_account(&settings, preferred.as_deref())
            .ok_or_else(|| accounts::no_account_of(engine))
    }

    /// What a new agent of the project on `engine` goes with unless it says its own: the defaults
    /// of the account it would go to (`account_for_new`), else of its harness (no account yet).
    pub(crate) fn new_agent_defaults(
        &self,
        project_id: &str,
        engine: EngineId,
    ) -> accounts::Defaults {
        let account = self
            .account_for_new(project_id, engine)
            .ok()
            .map(|id| accounts::get(&self.settings.read(), &id));
        accounts::defaults(account.as_ref(), engine)
    }

    /// `accounts::pick` as the board has it: a good account (`pace`: a short window under the
    /// threshold, a long one on pace or under it), holds nothing back (a pause after a usage
    /// limit) and has no agent waiting for its quota; failing that, the first whose only fault is
    /// its pace. When none is, one that holds nothing back although out ("Reprendre maintenant"),
    /// else the first active one. None without an active account.
    pub(crate) fn pick_account(
        &self,
        settings: &Settings,
        preferred: Option<&str>,
    ) -> Option<String> {
        let threshold = board::quota_threshold(settings.quota_pause);
        let usage = self.usage.lock().accounts.clone();
        let hold = self.hold.lock().clone();
        let waiting = self.quota_paused();
        let now = self.pause_now();
        let held = |id: &str| {
            waiting.contains_key(id)
                || board::account_pause(&usage, id, threshold, &hold, now).is_some()
        };
        // Under « Changer de compte à » too, when it is the lower (ESC-148).
        let pick = board::pick_threshold(settings);
        let rank = |id: &str| match held(id) {
            true => Standing::Out,
            false => accounts::standing(&usage, id, pick, now),
        };
        let id = accounts::pick_ranked(settings, preferred, rank)?;
        if !held(&id) {
            return Some(id);
        }
        accounts::candidates(settings, preferred)
            .into_iter()
            .find(|a| !held(a))
            .or(Some(id))
    }

    /// No ticket of the project on `engine` may start now: every account of that harness it may go
    /// to waits for its quota (an agent waiting for its reset, a window over the threshold, a usage
    /// limit with no resume).
    /// Launched by hand (`by_hand`), by a usage limit alone (`project_limit`).
    fn held(&self, project_id: &str, engine: EngineId, by_hand: bool) -> bool {
        self.project_hold_as(project_id, engine, by_hand).is_some()
    }

    /// What holds back the tickets, and the new agents, of the project on `engine`, if anything
    /// does (`board::project_hold`): every account of that harness it may go to is held, and then
    /// until the first of them is free again. Another harness is not held by it, and one without a
    /// quota is never held but by a usage limit. The tools that act refuse by it.
    #[cfg(test)]
    pub(crate) fn project_hold(&self, project_id: &str, engine: EngineId) -> Option<board::Held> {
        self.project_hold_as(project_id, engine, false)
    }

    /// What holds back a ticket or a new agent of the project asked for by hand, by a tool of
    /// Escouade's server as by the window's buttons: a usage limit alone (a window used up, an
    /// agent waiting for its reset, a pause after a limit). « Pause au-delà du quota » and the
    /// day's budget hold back the autopilot, never a hand (the user, 2026-10-28).
    pub(crate) fn project_limit(&self, project_id: &str, engine: EngineId) -> Option<board::Held> {
        self.project_hold_as(project_id, engine, true)
    }

    fn project_hold_as(
        &self,
        project_id: &str,
        engine: EngineId,
        by_hand: bool,
    ) -> Option<board::Held> {
        let settings = accounts::for_project(&self.settings.read(), engine, Some(project_id));
        let preferred = self
            .project(project_id)
            .ok()
            .and_then(|p| accounts::preferred(&settings, &p.account).map(str::to_string));
        let threshold = hold_threshold(settings.quota_pause, by_hand);
        let accounts = accounts::candidates(&settings, preferred.as_deref());
        let usage = hold_usage(&self.usage.lock().accounts, by_hand);
        let hold = self.hold.lock().clone();
        let waiting = self.quota_paused();
        board::project_hold(
            &usage,
            &accounts,
            threshold,
            &hold,
            &waiting,
            self.pause_now(),
        )
    }

    /// What holds back what spends the account's quota when asked for by hand (`project_limit`),
    /// a message to one of its agents included: a usage limit alone.
    pub(crate) fn account_limit(&self, account: &str) -> Option<board::Held> {
        self.account_hold_as(account, true)
    }

    /// What holds back anything that spends the account's quota, if anything does
    /// (`board::account_hold`): for the autopilot (`by_hand` false), « Pause au-delà du quota »
    /// and the day's budget too. An account the settings do not know is Principal, as for
    /// `quota_paused`.
    fn account_hold_as(&self, account: &str, by_hand: bool) -> Option<board::Held> {
        let settings = self.settings.read().clone();
        let id = accounts::get(&settings, account).id;
        let threshold = hold_threshold(settings.quota_pause, by_hand);
        let usage = hold_usage(&self.usage.lock().accounts, by_hand);
        let hold = self.hold.lock().clone();
        let waiting = self.quota_paused();
        board::account_hold(&usage, &id, threshold, &hold, &waiting, self.pause_now())
    }

    /// The autopilot's pauses looked at again, the window told when they changed (in order: looked
    /// at and told under one lock), and saved then: a restart keeps them. True when they changed.
    fn refresh_pause(&self) -> bool {
        let mut shown = self.pause_shown.lock();
        let now = (self.autopilot_pause(), self.project_pauses());
        if *shown == now {
            return false;
        }
        *shown = now.clone();
        let (pause, projects) = now;
        self.hub.emit(UiEvent::AutopilotPause { pause, projects });
        self.request_save();
        true
    }

    /// The autopilot's pause looked at again, as the quotas are read and by the timer: the window
    /// is told when it changed, and once it is over, what may start starts.
    pub fn pause_tick(self: &Arc<Self>) {
        // Past midnight, the day's costs are read again: a day's budget holds no longer.
        if self.usage.lock().day_over(self.pause_now()) {
            self.settle_usage();
        }
        if self.refresh_pause() {
            self.schedule();
        }
    }

    /// A ticket met the usage limit with no resume planned, on the account: the next one would
    /// meet it too, so none starts on it for a while (`board::LIMIT_PAUSE_MS`), or until
    /// "Reprendre maintenant".
    pub(crate) fn pause_after_limit(&self, account: &str) {
        self.hold_until(account, self.pause_now() + board::LIMIT_PAUSE_MS);
    }

    /// The account met the usage limit: nothing starts on it until `until` (its windows may not say
    /// so, and an agent that left it for another must not come back to it at the next limit).
    pub(crate) fn hold_until(&self, account: &str, until: i64) {
        self.hold.lock().limit(account, until);
        // Saved even when a longer pause hides it: it may outlast that one once lifted.
        self.request_save();
        self.refresh_pause();
    }

    /// "Reprendre maintenant": the autopilot's pause is lifted (the one after a usage limit, and
    /// the windows over the threshold until their end), and what may start starts.
    pub fn autopilot_resume(self: &Arc<Self>) {
        let threshold = board::quota_threshold(self.settings.read().quota_pause);
        let usage = self.usage.lock().accounts.clone();
        let now = self.pause_now();
        self.hold.lock().lift(&usage, threshold, now);
        self.request_save();
        self.refresh_pause();
        self.schedule();
    }

    /// One pass at a time: the tickets "À faire" that fit start, project by project.
    pub async fn schedule_now(self: &Arc<Self>) {
        let _pass = self.board_lock.lock().await;
        // The window is told of the pause whatever else holds the tickets back.
        self.refresh_pause();
        let projects = self.projects.read().clone();
        let mut missing = false;
        for p in projects {
            // Without Claude Code (the one of the account it would go to) no agent can work:
            // rather than make every ticket's worktree and agent only to block it, none starts
            // until it is found (saving the settings looks again).
            let settings = self.settings.read().clone();
            // No account serves the project (all reserved to others): none starts, as the window
            // is told below; saving the accounts looks again.
            let account = self.account_for_new(&p.id, p.engine()).ok();
            if let Some(id) = &account {
                let account = accounts::get(&settings, id);
                if accounts::program(&account, &settings).is_none() {
                    missing = true;
                    if !self.claude_missing.swap(true, Ordering::AcqRel) {
                        log::info!("board: Claude Code not found, no ticket starts until it is");
                    }
                    self.refuse_forced(
                        &p,
                        &StartRefusal::Text(tr!(
                            "Claude Code est introuvable : aucun ticket ne démarre tant qu’il ne l’est pas.",
                            "Claude Code was not found: no ticket starts until it is."
                        )),
                    );
                    continue;
                }
            }
            // Held (`ticket_held`), none starts: the autopilot's by « Pause au-delà du quota »
            // too, one launched by hand by a usage limit alone. Looked at again for each project
            // and each start: one started in this pass, or already at work, may meet the usage
            // limit while the pass goes on (each start takes seconds).
            // Each ticket by the accounts of its own harness.
            let engine_of = |t: &Ticket| t.engine.unwrap_or_else(|| p.engine());
            // One launched by hand that a usage limit holds back (it may have been met since the
            // tool checked) says so, rather than wait in « À faire » with nothing said.
            let launched: Vec<Ticket> = self
                .tickets
                .read()
                .iter()
                .filter(|t| t.project_id == p.id && t.column == Column::Todo && t.forced)
                .cloned()
                .collect();
            for t in launched {
                if let Some(held) = self.ticket_hold(&p, &t, engine_of(&t)) {
                    self.refuse_start(&p, &t.id, StartRefusal::Held(held));
                }
            }
            let ids = {
                let tickets = self.tickets.read().clone();
                let mut held: HashMap<(EngineId, bool), bool> = HashMap::new();
                board::to_start(&tickets, &p.id, &p.board, |t| {
                    let e = engine_of(t);
                    // One set on an account waits for that account alone (ESC-153).
                    if !t.account.is_empty() {
                        return self.ticket_held(&p, t, e);
                    }
                    *held
                        .entry((e, t.forced))
                        .or_insert_with(|| self.held(&p.id, e, t.forced))
                })
            };
            // A board that told why nothing starts is looked at again even with nothing to start:
            // its issue goes once its target is there.
            if ids.is_empty() && !self.board_issues.lock().contains_key(&p.id) {
                continue;
            }
            if account.is_none() {
                let issue = accounts::no_account_issue(p.engine());
                if self.set_board_issue(&p.id, Some(issue.clone())) {
                    log::info!("board of {}: {issue}", p.name);
                }
                self.refuse_forced(&p, &StartRefusal::Text(issue));
                continue;
            }
            // Every ticket's branch starts from the target: while it has no commit yet (a new
            // repository) or is gone (deleted, renamed), none starts, rather than each make an
            // agent only to block. The window is told why; choosing another target, any change
            // of the board, or the project's next git refresh (a first commit, the branch made
            // again) looks again.
            let target = self.target_of(&p).await;
            // Of several repositories (ESC-169): each starts from its own (its worktree says why).
            let issue = if p.is_multi() {
                None
            } else {
                let exists = !target.is_empty() && git::branch_exists(&p.path, &target).await;
                let unborn = !exists && git::head_branch(&p.path).await == target;
                board::target_issue(i18n::ui(), &target, exists, unborn)
            };
            if self.set_board_issue(&p.id, issue.clone()) {
                if let Some(why) = &issue {
                    log::info!("board of {}: {why}", p.name);
                }
            }
            if let Some(issue) = issue {
                self.refuse_forced(&p, &StartRefusal::Text(issue));
                continue;
            }
            for id in ids {
                let Ok(t) = self.ticket(&id) else { continue };
                if let Some(held) = self.ticket_hold(&p, &t, engine_of(&t)) {
                    // Met while this pass started the ones before it.
                    if t.forced {
                        self.refuse_start(&p, &id, StartRefusal::Held(held));
                    }
                    continue;
                }
                self.start_ticket(&p, &id).await;
            }
        }
        if !missing {
            self.claude_missing.store(false, Ordering::Release);
        }
    }

    /// Why the project's board starts nothing, as the window is told (only when it changes):
    /// true when it changed.
    fn set_board_issue(&self, project_id: &str, issue: Option<String>) -> bool {
        let changed = {
            let mut issues = self.board_issues.lock();
            let changed = issues.get(project_id) != issue.as_ref();
            match &issue {
                Some(why) => issues.insert(project_id.to_string(), why.clone()),
                None => issues.remove(project_id),
            };
            changed
        };
        if changed {
            self.hub.emit(UiEvent::BoardIssue {
                project_id: project_id.to_string(),
                issue,
            });
        }
        changed
    }

    /// The board's target branch. A board saved without one (its first ticket made by an earlier
    /// version, on a branch-less HEAD) takes the branch the project is on, for good: it no longer
    /// moves with what is checked out. Empty on a detached HEAD.
    pub(crate) async fn target_of(&self, project: &Project) -> String {
        if !project.board.target.is_empty() {
            return project.board.target.clone();
        }
        let current = git::head_branch(&project.path).await;
        if current.is_empty() {
            return current;
        }
        // (Set meanwhile, by another pass or the settings: that one.)
        let (target, fixed) = {
            let mut projects = self.projects.write();
            match projects.iter_mut().find(|p| p.id == project.id) {
                Some(p) if p.board.target.is_empty() => {
                    p.board.target = current.clone();
                    (current, Some(p.clone()))
                }
                Some(p) => (p.board.target.clone(), None),
                None => (current, None),
            }
        };
        if let Some(project) = fixed {
            self.hub.emit(UiEvent::Project { project });
            self.request_save();
        }
        target
    }

    /// "En cours" at once (no other pass takes it), then its agent and its first message. A start
    /// that fails blocks the ticket, which then holds no place: the next one is tried.
    async fn start_ticket(self: &Arc<Self>, project: &Project, id: &str) {
        // From "En cours" to its agent's first turn, with no agent at work for a while.
        let _working = self.working();
        let started = self.edit_ticket(id, |t| {
            if t.column != Column::Todo {
                bail!(tr!(
                    "ce ticket est déjà parti",
                    "this ticket has already started"
                ));
            }
            t.column = Column::Doing;
            t.agent_id = None;
            t.iteration = 1;
            t.loops += 1;
            // Started out of turn: by « Lancer » (counted there), or again after a restart.
            let forced = std::mem::take(&mut t.forced);
            t.partial = false;
            t.blocked = None;
            t.conflict = false;
            t.reminded = false;
            t.started_at = Some(now_ms());
            // A new agent from the target branch: nothing of an earlier attempt is in place.
            t.progress.clear();
            for c in &mut t.criteria {
                c.ok = false;
                c.note.clear();
            }
            Ok((t.clone(), forced))
        });
        let Ok((t, forced)) = started else { return };
        if !forced {
            self.telemetry
                .track(Event::TicketStarted { autopilot: true });
        }
        if let Err(e) = self.launch_ticket_agent(project, &t).await {
            log::warn!("ticket {}: start failed: {e:#}", t.key);
            // Only while it is still this start's: one sent back to do meanwhile (its agent
            // archived or deleted), or started again with another agent, stays as it is.
            let blocked = self.edit_ticket(id, |x| {
                let ours = x.column == Column::Doing && x.started_at == t.started_at;
                if ours {
                    x.blocked = Some(error_reason(&e));
                }
                Ok(ours)
            });
            if blocked.unwrap_or(false) {
                self.notify_ticket(id, false);
            }
            self.schedule();
        }
    }

    /// A ticket started again on a branch it took up: the agent it had, archived, still has that
    /// branch in its worktree, which would keep the new agent from taking it up. That folder goes,
    /// after the project's teardown (the branch is the user's and stays, and restoring the agent
    /// makes the folder again, if the branch is free then). One that holds changes, new files
    /// included, is left as it is, and the start is refused, saying what to do.
    async fn release_archived_holder(&self, project: &Project, t: &Ticket) -> Result<()> {
        let Some(root) = self.toplevel(&project.path).await else {
            return Ok(());
        };
        let Ok(Some(local)) = git::local_of(&root, &t.branch).await else {
            return Ok(());
        };
        let holders: Vec<(String, Worktree)> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                let m = &rt.meta;
                let wt = m.worktree.clone()?;
                (m.archived
                    && m.ticket_id.as_deref() == Some(&t.id)
                    && wt.existing
                    && wt.branch == local)
                    .then(|| (m.name.clone(), wt))
            })
            .collect();
        for (name, wt) in holders {
            if !Path::new(&wt.path).is_dir() {
                continue;
            }
            // Whatever is not committed there (new files included) would be lost.
            if !git::status(&wt.path)
                .await
                .is_ok_and(|s| s.entries.is_empty())
            {
                bail!(archived_holds(i18n::ui(), &name, &local));
            }
            if let Some(problem) = self.teardown_worktree(project, &wt, None).await {
                log::warn!("ticket {}: {name}'s worktree: {problem}", t.key);
            }
            if let Err(e) = git::worktree_remove_dir(&project.path, &wt.path).await {
                log::warn!("ticket {}: {name}'s worktree not released: {e:#}", t.key);
            }
        }
        self.git.refresh(&project.id);
        Ok(())
    }

    /// The account a ticket's agent starts on (ESC-153): the one the ticket is set to while it may
    /// still take it (`lost_account`); one that may not sends the ticket back to « Auto », which
    /// its card and a notification say. « Auto »: the one a new agent of its project goes to.
    fn ticket_start_account(
        self: &Arc<Self>,
        project: &Project,
        t: &Ticket,
        engine: EngineId,
    ) -> Result<String> {
        if !t.account.is_empty() {
            match self.lost_account(project, engine, &t.account) {
                None => return Ok(t.account.clone()),
                Some(why) => self.back_to_auto(&t.id, &t.account, &why),
            }
        }
        self.account_for_new(&project.id, engine)
    }

    /// Why the ticket set on the account `id` may no longer go to it: gone, switched off, signed
    /// out, or refused to its project since (reserved to others, `accounts::ticket_refusal`).
    fn lost_account(&self, project: &Project, engine: EngineId, id: &str) -> Option<Lost> {
        let settings = self.settings.read().clone();
        match accounts::find(&settings, id) {
            None => return Some(Lost::Gone),
            Some(a) if !a.active => return Some(Lost::Off),
            Some(_) => {}
        }
        let signed_out = self
            .usage
            .lock()
            .accounts
            .iter()
            .any(|u| u.id == id && !u.connected);
        if signed_out {
            return Some(Lost::SignedOut);
        }
        accounts::ticket_refusal(&settings, project, engine, id)
            .map(|e| Lost::Refused(e.to_string()))
    }

    /// The ticket set on the account `id` (named `name`, the settings may not have it any more)
    /// goes back to « Auto », for `why`: its card says it, and so does a notification.
    fn back_to_auto_named(self: &Arc<Self>, ticket_id: &str, name: &str, why: &Lost) {
        let lang = i18n::ui();
        let card = lost_line(lang, name, why);
        let back = self.edit_ticket(ticket_id, |t| {
            if t.account.is_empty() {
                return Ok(None);
            }
            t.account.clear();
            t.account_lost = Some(card.clone());
            Ok(Some(t.clone()))
        });
        let Ok(Some(t)) = back else { return };
        log::info!("ticket {}: its account is lost, back to Auto", t.key);
        let project = self
            .project(&t.project_id)
            .map(|p| p.name)
            .unwrap_or_default();
        self.alert(
            NotifyKind::Ticket,
            project,
            lost_alert(lang, &t.key, name, why),
            UiEvent::FocusBoard {
                project_id: t.project_id,
            },
        );
    }

    /// `back_to_auto_named`, the account named as the settings have it (by its id when gone).
    fn back_to_auto(self: &Arc<Self>, ticket_id: &str, id: &str, why: &Lost) {
        let name = accounts::find(&self.settings.read(), id)
            .map_or_else(|| id.to_string(), |a| accounts::name_in(i18n::ui(), a));
        self.back_to_auto_named(ticket_id, &name, why);
    }

    /// The settings saved: a ticket set on an account removed or switched off goes back to
    /// « Auto » at once (`before`: the settings before, which name it). One done keeps it, as its
    /// history.
    pub(crate) fn release_lost_accounts(self: &Arc<Self>, before: &Settings) {
        let settings = self.settings.read().clone();
        let lost: Vec<(String, String, Lost)> = self
            .tickets
            .read()
            .iter()
            .filter(|t| !t.account.is_empty() && t.column != Column::Done)
            .filter_map(|t| {
                let why = match accounts::find(&settings, &t.account) {
                    None => Lost::Gone,
                    Some(a) if !a.active => Lost::Off,
                    Some(_) => return None,
                };
                let name = accounts::find(before, &t.account)
                    .or_else(|| accounts::find(&settings, &t.account))
                    .map_or_else(|| t.account.clone(), |a| accounts::name_in(i18n::ui(), a));
                Some((t.id.clone(), name, why))
            })
            .collect();
        for (id, name, why) in lost {
            self.back_to_auto_named(&id, &name, &why);
        }
    }

    /// Whether the ticket may not start now for want of an account (`held`): one set on an account
    /// waits for that account alone (ESC-153), else for those of its harness.
    /// Launched by hand, a ticket waits for a usage limit alone: « Pause au-delà du quota » and the
    /// day's budget hold back the autopilot (the user, 2026-10-28).
    fn ticket_held(&self, project: &Project, t: &Ticket, engine: EngineId) -> bool {
        self.ticket_hold(project, t, engine).is_some()
    }

    /// What holds the ticket back (`ticket_held`), if anything does.
    fn ticket_hold(&self, project: &Project, t: &Ticket, engine: EngineId) -> Option<board::Held> {
        match self.pinned(project, t, engine) {
            Some(id) => self.account_hold_as(&id, t.forced),
            None => self.project_hold_as(&project.id, engine, t.forced),
        }
    }

    /// A ticket launched by hand that the pass did not start goes back as it was (« À faire », not
    /// forced) and the reason is kept for whoever launched it (`start_refusals`), and told to the
    /// window: else it stays « À faire » with nothing said.
    fn refuse_start(self: &Arc<Self>, project: &Project, id: &str, why: StartRefusal) {
        let back = self.edit_ticket(id, |t| {
            let ours = t.column == Column::Todo && t.forced;
            if ours {
                t.forced = false;
            }
            Ok(ours)
        });
        if !back.unwrap_or(false) {
            return;
        }
        let text = self.refusal_text(&why);
        log::info!("ticket {id}: not started: {text}");
        self.start_refusals.lock().insert(id.to_string(), why);
        let key = self.ticket(id).map(|t| t.key).unwrap_or_default();
        self.alert(
            NotifyKind::Ticket,
            project.name.clone(),
            tr!(
                "{key} n’est pas parti : {text}",
                "{key} did not start: {text}",
                key = key,
                text = text
            ),
            UiEvent::FocusBoard {
                project_id: project.id.clone(),
            },
        );
    }

    /// `refuse_start` for every ticket of the project launched by hand and waiting.
    fn refuse_forced(self: &Arc<Self>, project: &Project, why: &StartRefusal) {
        let ids: Vec<String> = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == project.id && t.column == Column::Todo && t.forced)
            .map(|t| t.id.clone())
            .collect();
        for id in ids {
            self.refuse_start(project, &id, why.clone());
        }
    }

    /// What was said of a refused start, as the tools say it.
    pub(crate) fn refusal_text(&self, why: &StartRefusal) -> String {
        match why {
            StartRefusal::Held(held) => crate::mcp::act::why(self, held),
            StartRefusal::Text(text) => text.clone(),
        }
    }

    /// The account the ticket is set to, while it may still take it (`lost_account`).
    pub(crate) fn pinned(&self, project: &Project, t: &Ticket, engine: EngineId) -> Option<String> {
        (!t.account.is_empty() && self.lost_account(project, engine, &t.account).is_none())
            .then(|| t.account.clone())
    }

    /// Whether the agent works on a ticket set on an account (ESC-153): it never leaves it on its
    /// own, at the limit as past « Changer de compte à ».
    pub(crate) fn agent_pinned(&self, agent_id: &str) -> bool {
        self.tickets.read().iter().any(|t| {
            t.agent_id.as_deref() == Some(agent_id)
                && t.column != Column::Done
                && !t.account.is_empty()
        })
    }

    /// The instructions added to a ticket's agent: its protocol, within the room its harness gives
    /// it, then the code index's rule when the project has turned the index on (the room is kept
    /// for it first, so the criteria give way before the rule does).
    pub(crate) fn ticket_instructions(
        &self,
        project: &Project,
        lang: i18n::Lang,
        t: &Ticket,
        ports: Option<u16>,
        isola: bool,
        follow: Option<&str>,
    ) -> String {
        let engine = crate::engine::by_id(t.engine.unwrap_or(project.engine()));
        let rule = self
            .code_index_on(project)
            .then(|| crate::codeindex::hint::rule(lang));
        let room = engine.instructions_room().map(|r| crate::engine::Room {
            max: r
                .max
                .saturating_sub(rule.as_deref().map_or(0, |x| (r.weigh)(&format!(" {x}"))))
                // The shared memory's line, added after it (`core::with_memory`).
                .saturating_sub((r.weigh)(&crate::memory::instruction(lang)) + 1)
                // And the spec in Escouade's (ESC-278), added before it.
                .saturating_sub(follow.map_or(0, |f| (r.weigh)(f) + 1)),
            weigh: r.weigh,
        });
        let mut p = board::protocol_prompt_for(lang, t, ports, isola, room);
        if let Some(rule) = rule {
            p.push(' ');
            p.push_str(&rule);
        }
        p
    }

    /// The ticket's agent: its worktree on `<prefix><key>` (`ticket/<key>`) from the target branch, its block of
    /// ports (none when isola runs the project's services), the protocol appended to its system
    /// prompt; then, its worktree set up, its first message.
    async fn launch_ticket_agent(self: &Arc<Self>, project: &Project, t: &Ticket) -> Result<()> {
        let s = &project.board;
        let target = self.target_of(project).await;
        let settings = self.settings.read().clone();
        // What it is told, from its protocol to its first message.
        let lang = self.lang().claude;
        // Its account, picked now: whether its model takes the mode `auto` is what its `claude` told.
        let engine = t.engine.unwrap_or(project.engine());
        let account = self.ticket_start_account(project, t, engine)?;
        // Its ticket's own model and effort come first, then the board's; the mode is the board's.
        let defaults = accounts::defaults(accounts::find(&settings, &account), engine);
        let (model, effort, mode) =
            ticket_agent_options(t, s, &defaults, &|m| self.takes_auto(engine, &account, m));
        // The branch it takes up may still be in the worktree of its earlier agent, archived.
        if !t.branch.is_empty() {
            self.release_archived_holder(project, t).await?;
        }
        // The ticket's own branch from the target, or one that exists, which the agent takes up.
        let (new_branch, taken_up, base) = match t.branch.as_str() {
            "" => (
                Some((
                    board::ticket_branch(&project.ticket_branch_prefix, t),
                    target.clone(),
                )),
                None,
                None,
            ),
            branch => (None, Some(branch.to_string()), Some(target.clone())),
        };
        // As its worktree will be: the branch it is on has isola's configuration.
        let from = taken_up.as_deref().unwrap_or(&target);
        let isola = isola::cli().is_some() && isola::configured_on(&project.path, from).await;
        // Reserved at once: no other start or launch preparation gets this block meanwhile.
        let ports = if isola { None } else { self.reserve_ports() };
        let follow = self.docs_follow(project, lang).await;
        let made = self
            .create_agent_with(
                &project.id,
                AgentOptions {
                    // The ticket's own, else the project's.
                    engine: t.engine,
                    model: Some(model),
                    effort: Some(effort),
                    mode: Some(mode),
                    name: Some(board::ticket_agent_name(t)),
                    worktree: new_branch,
                    // Within the room its harness gives it, the code index's rule included, and what
                    // it is told of the shared memory after it (`core::with_memory`).
                    append_prompt: Some(self.ticket_instructions(
                        project,
                        lang,
                        t,
                        ports,
                        isola,
                        follow.as_deref(),
                    )),
                    ticket_id: Some(t.id.clone()),
                    port_base: ports,
                    select: false,
                    copy_of: None,
                    account: Some(account),
                    isolated: None,
                    branch: taken_up,
                    base,
                    new_branch: false,
                    // Set apart from Remote Control by hand for its earlier agent: so is this one.
                    remote_off: t.remote_off,
                    cwd: None,
                    discussion: None,
                },
            )
            .await;
        // The agent holds the block from now on (or it is free again).
        self.unreserve_ports(ports);
        let agent_id = made?.meta.id;
        if let Err(e) = self.edit_ticket(&t.id, |x| {
            x.agent_id = Some(agent_id.clone());
            Ok(())
        }) {
            // The ticket went meanwhile (deleted): its agent goes with it.
            let _ = self.archive_agent(&agent_id, true).await;
            return Err(e);
        }
        // Its worktree set up first; what failed there is said with its first message.
        self.wait_setup(&agent_id).await;
        // Archived or deleted meanwhile (the ticket sent back to do, maybe started again with
        // another agent), or the ticket deleted: it is not this agent's to take any more.
        let still_ours = self
            .ticket(&t.id)
            .is_ok_and(|x| x.column == Column::Doing && x.agent_id.as_deref() == Some(&agent_id));
        if !still_ours || self.live_agent(&agent_id).is_none() {
            return Ok(());
        }
        let failure = self
            .agent(&agent_id)
            .ok()
            .and_then(|h| h.lock().setup_failure.take());
        let (images, missing) = self.ticket_attachments(t).await;
        // After the text and the images, a line for each of its other files.
        let first = self.with_ticket_files(
            lang,
            &agent_id,
            t,
            board::first_message_with(lang, t, images.len(), &missing),
        );
        let first = board::with_setup_failure(lang, first, failure.as_deref());
        self.send_message(&agent_id, first, images).await
    }

    // ---------- ends of turns ----------

    /// Called by `apply` when an agent's turn ended: its recipe is kept, its ticket moves on.
    pub(crate) fn on_turn_end(self: &Arc<Self>, agent_id: &str, end: TurnEnd) {
        let (c, id) = (self.clone(), agent_id.to_string());
        tauri::async_runtime::spawn(async move { c.turn_ended(&id, end).await });
    }

    pub(crate) async fn turn_ended(self: &Arc<Self>, agent_id: &str, end: TurnEnd) {
        let report = match &end {
            TurnEnd::Finished(text) => board::parse_report(text),
            _ => None,
        };
        let Ok(h) = self.agent(agent_id) else { return };
        if let Some(recipe) = report.as_ref().and_then(|r| r.recipe.clone()) {
            h.lock().meta.recipe = Some(recipe);
            self.emit_agent(&h);
            self.request_save();
        }
        let (ticket_id, resumes) = {
            let rt = h.lock();
            (rt.meta.ticket_id.clone(), rt.meta.resume_at.is_some())
        };
        let limited = end == TurnEnd::Limited;
        // Its account past « Changer de compte à »: its next turn goes to another one (ESC-148),
        // before the board sends it one. At the limit, `switch_on_limit` decides.
        // Stopped by a sign-in not refreshed: its process starts again there (`signin`).
        if !limited && end != TurnEnd::SignIn {
            self.switch_if_over(agent_id).await;
        }
        let Some(ticket_id) = ticket_id else {
            // Its harness spent, an agent goes on on another one, with the option (ESC-88).
            if limited && self.handover_on_limit(agent_id).await {
                return;
            }
            // The turn of a handover file from a terminal view: in the new harness's (ESC-120).
            self.view_after_handover(agent_id, &end).await;
            return;
        };
        // Stopped by the usage limit with another account to go on on: it does, rather than wait
        // for its own to reset; with none left on its harness, on another harness (the option of
        // the handover). The ticket stays as it is: its agent's next turn is read.
        if limited
            && (self.switch_on_limit(agent_id).await || self.handover_on_limit(agent_id).await)
        {
            return;
        }
        // A usage limit with no resume planned (turned off, no reset known) would leave the
        // ticket waiting forever: it is blocked. The next ticket would meet the limit too: the
        // autopilot pauses first, so that no pass starts one in the place this frees.
        let end = match end {
            TurnEnd::Limited if !resumes => {
                self.pause_after_limit(&self.account_of(agent_id));
                TurnEnd::Error(quota_lost(i18n::ui()))
            }
            end => end,
        };
        let lang = self.lang().claude;
        let mut reported = None;
        let Ok(next) = self.edit_ticket(&ticket_id, |t| {
            // A ticket started again has another agent: the old one's turns never move it.
            if t.agent_id.as_deref() != Some(agent_id) {
                return Ok(board::Next::default());
            }
            let reports = t.column == Column::Doing
                && matches!(end, TurnEnd::Finished(_))
                && report.as_ref().is_some_and(|r| r.criteria.is_some());
            // Stopped by the usage limit, a blocked ticket stays as it is: its block already
            // tells why it does not move (its resume lost, cancelled or failed, whether that
            // came before this reading or after), and its agent's next finished turn is read.
            if limited && t.blocked.is_some() {
                return Ok(board::Next::default());
            }
            let next = board::turn_end(lang, t, &end, report.as_ref(), now_ms());
            if reports {
                reported = Some((t.criteria.iter().filter(|c| c.ok).count(), t.criteria.len()));
            }
            Ok(next)
        }) else {
            return;
        };
        if let Some((met, total)) = reported {
            self.facts.emit(
                crate::facts::Body::TicketReport { met, total },
                crate::facts::About::ticket(&ticket_id),
            );
        }
        if let Some(text) = next.send {
            let _ = self.send_or_block(&ticket_id, agent_id, text).await;
        }
        if next.ready {
            self.notify_ticket(&ticket_id, true);
        }
        if next.blocked {
            self.notify_ticket(&ticket_id, false);
        }
        self.schedule();
    }

    /// The agent of a ticket "En cours", stopped by the usage limit, goes on by itself on another
    /// account (`account_to_switch_to`), its session with it. True when it did; when it could not
    /// (a failure), it waits for the reset as it does with no other account.
    async fn switch_on_limit(self: &Arc<Self>, agent_id: &str) -> bool {
        let Some(target) = self.account_to_switch_to(agent_id) else {
            return false;
        };
        // The account it leaves is at its limit until the reset its resume waited for (planned
        // when the turn ended), else for a while.
        let (from, resume) = match self.agent(agent_id) {
            Ok(h) => {
                let rt = h.lock();
                (rt.meta.account.clone(), rt.meta.resume_at)
            }
            Err(_) => return false,
        };
        let from = accounts::get(&self.settings.read(), &from).id;
        self.hold_until(
            &from,
            resume.unwrap_or(self.pause_now() + board::LIMIT_PAUSE_MS),
        );
        match self.resume_on_account(agent_id, &target).await {
            Ok(()) => true,
            Err(e) => {
                log::warn!("agent {agent_id}: no move to the account {target} after the usage limit: {e:#}");
                let _ = self.with_agent(agent_id, |rt, fx| {
                    rt.notice(
                        "warn",
                        tr!(
                            "Reprise sur un autre compte impossible : {e:#}",
                            "Couldn’t resume on another account: {e:#}"
                        ),
                        fx,
                    );
                    Ok(())
                });
                false
            }
        }
    }

    /// The account a ticket's agent stopped by the usage limit goes on on, if there is one and
    /// the user did not turn that off: `other_account` under « Pause au-delà du quota ».
    fn account_to_switch_to(&self, agent_id: &str) -> Option<String> {
        let settings = self.settings.read().clone();
        let ticket = self
            .doing_ticket_of(agent_id)
            .and_then(|id| self.ticket(&id).ok())?;
        // A blocked ticket stays as it is, as the board reads the end of a turn at the limit
        // (`turn_ended`): its block tells why it does not move.
        if !settings.switch_on_limit || ticket.blocked.is_some() {
            return None;
        }
        let threshold = board::quota_threshold(settings.quota_pause);
        self.other_account(agent_id, threshold).ok()
    }

    /// The account an agent goes on on by itself, leaving its own: the one `accounts::pick_ranked`
    /// gives (the project's preferred account, else the first of the best active ones) when it is
    /// another than the agent's, usable (signed in, one an agent would not leave at `threshold`:
    /// `pace`, no pause after a usage limit, no agent of it waiting for its reset), on the same side
    /// of WSL. A project that prefers the agent's account keeps it there. Why there is none
    /// otherwise: `NoOther::Private` when only the accounts a confined private project shares are
    /// left (ESC-141).
    pub(crate) fn other_account(
        &self,
        agent_id: &str,
        threshold: u32,
    ) -> std::result::Result<String, NoOther> {
        let settings = self.settings.read().clone();
        let (project, current, engine) = {
            let rt = self.agent(agent_id).map_err(|_| NoOther::Nobody)?;
            let rt = rt.lock();
            if rt.meta.archived {
                return Err(NoOther::Nobody);
            }
            (
                rt.meta.project_id.clone(),
                accounts::get(&settings, &rt.meta.account).id,
                rt.meta.engine,
            )
        };
        // Its ticket is set on its account: never another (ESC-153).
        if self.agent_pinned(agent_id) {
            return Err(NoOther::Pinned);
        }
        // No account of its harness serves the project any more: none to go on on. Another
        // harness's would be a handover, not a switch (ESC-88).
        if !accounts::serves_any(&settings, engine, Some(&project)) {
            return Err(NoOther::Nobody);
        }
        let all = settings;
        let settings = self.project_accounts(&all, engine, &project);
        let preferred = self
            .project(&project)
            .ok()
            .and_then(|p| accounts::preferred(&settings, &p.account).map(str::to_string));
        let usage = self.usage.lock().accounts.clone();
        let hold = self.hold.lock().clone();
        let waiting = self.quota_paused();
        let now = self.pause_now();
        // A session does not go between Claude Code on Windows and in WSL (`move_session`).
        let in_wsl = |id: &str| {
            accounts::program(&accounts::get(&settings, id), &settings)
                .is_some_and(|p| p.wsl().is_some())
        };
        let side = in_wsl(&current);
        // The verdict on the account's quota, when it may take the agent at all: arriving on one
        // it would leave at once (the same test) would send it back and forth.
        let rank = |id: &str| {
            let verdict = accounts::verdict(&usage, id, threshold, now);
            let usable = id != current
                && in_wsl(id) == side
                // Not signed in: its first turn would only fail (an account not read yet is).
                && usage.iter().find(|u| u.id == id).is_none_or(|u| u.connected)
                && !waiting.contains_key(id)
                && board::account_pause(&usage, id, threshold, &hold, now).is_none()
                && !verdict.leave;
            usable.then_some(verdict.standing)
        };
        let target = accounts::pick_ranked(&settings, preferred.as_deref(), |id| {
            rank(id).unwrap_or(Standing::Out)
        })
        .filter(|t| rank(t).is_some())
        .ok_or(NoOther::Nobody)?;
        // A private project goes on its own to none of the accounts it shares (ESC-141): its agent
        // stays, unless « Comptes partagés à la limite » allows it.
        let project = self.project(&project).ok();
        match project.is_none_or(|p| accounts::may_switch_to(&all, &p, &target)) {
            true => Ok(target),
            false => Err(NoOther::Private),
        }
    }

    /// The accounts of `engine` as a new agent of the project `project_id` sees them
    /// (`accounts::for_project_of`); one the app does not know, as any project
    /// (`accounts::for_project`).
    pub(crate) fn project_accounts(
        &self,
        all: &Settings,
        engine: EngineId,
        project_id: &str,
    ) -> Settings {
        match self.project(project_id) {
            Ok(p) => accounts::for_project_of(all, engine, &p),
            Err(_) => accounts::for_project(all, engine, Some(project_id)),
        }
    }

    /// Sends the ticket's agent a message; when it cannot, the ticket is blocked with the reason.
    pub(crate) async fn send_or_block(
        self: &Arc<Self>,
        ticket_id: &str,
        agent_id: &str,
        text: String,
    ) -> Result<()> {
        self.send_or_block_with(ticket_id, agent_id, text, Vec::new())
            .await
    }

    /// `send_or_block` of a message with `attachments`.
    async fn send_or_block_with(
        self: &Arc<Self>,
        ticket_id: &str,
        agent_id: &str,
        text: String,
        attachments: Vec<crate::core::Attachment>,
    ) -> Result<()> {
        if let Err(e) = self.send_message(agent_id, text, attachments).await {
            log::warn!("agent {agent_id}: message to its ticket's agent not sent: {e:#}");
            let _ = self.edit_ticket(ticket_id, |t| {
                t.blocked = Some(error_reason(&e));
                Ok(())
            });
            self.notify_ticket(ticket_id, false);
            return Err(e);
        }
        Ok(())
    }

    /// The automatic resume its agent waited for after the usage limit is gone (cancelled, turned
    /// off): its ticket "En cours" is blocked rather than hold its place forever, and the autopilot
    /// pauses as after any usage limit with no resume (the limit is still there).
    pub(crate) fn resume_lost(self: &Arc<Self>, agent_id: &str) {
        let Some(id) = self.doing_ticket_of(agent_id) else {
            return;
        };
        // Before its place is free: no pass starts the next ticket, which would meet the limit.
        self.pause_after_limit(&self.account_of(agent_id));
        let blocked = self.edit_ticket(&id, |t| {
            let waiting = t.column == Column::Doing
                && t.blocked.is_none()
                && t.agent_id.as_deref() == Some(agent_id);
            if waiting {
                let lang = i18n::ui();
                t.blocked = Some(board::error_blocked(lang, &quota_lost(lang)));
            }
            Ok(waiting)
        });
        if blocked.unwrap_or(false) {
            self.notify_ticket(&id, false);
            self.schedule();
        }
    }

    /// "ATL-42 prêt à tester" or "ATL-42 bloqué : <raison>"; a click shows the project's board.
    pub(crate) fn notify_ticket(self: &Arc<Self>, ticket_id: &str, ready: bool) {
        let Ok(t) = self.ticket(ticket_id) else {
            return;
        };
        let project = self
            .project(&t.project_id)
            .map(|p| p.name)
            .unwrap_or_default();
        let blocked = t.blocked.clone().unwrap_or_default();
        let body = ticket_alert(i18n::ui(), &t.key, ready, &blocked);
        self.alert(
            NotifyKind::Ticket,
            project,
            body,
            UiEvent::FocusBoard {
                project_id: t.project_id,
            },
        );
    }

    /// The ticket "En cours" the agent works on.
    pub(crate) fn doing_ticket_of(&self, agent_id: &str) -> Option<String> {
        self.tickets
            .read()
            .iter()
            .find(|t| t.agent_id.as_deref() == Some(agent_id) && t.column == Column::Doing)
            .map(|t| t.id.clone())
    }

    /// The agent's ticket is "En cours": its turns are the board's to tell about.
    pub(crate) fn ticket_doing(&self, agent_id: &str) -> bool {
        self.doing_ticket_of(agent_id).is_some()
    }

    // ---------- recovery and manual management ----------

    /// The agent, unless it is gone or archived.
    fn live_agent(&self, id: &str) -> Option<AgentMeta> {
        let meta = self.agent(id).ok()?.lock().meta.clone();
        (!meta.archived).then_some(meta)
    }

    /// At startup, for each ticket "En cours" and not blocked (`board::recovery`): its agent is
    /// asked to go on (its turn cut by the app's stop, or ended but not read), or waits for its
    /// quota; a ticket whose start the stop cut goes back to "À faire", and the agent made for it,
    /// if any, is archived. Then what may start starts.
    pub fn recover_tickets(self: &Arc<Self>) {
        let lang = self.lang().claude;
        let cut = std::mem::take(&mut *self.cut_turns.lock());
        let doing: Vec<(String, String, Option<String>)> = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.column == Column::Doing && t.blocked.is_none())
            // Under way on a teammate's machine (ESC-190): theirs to go on with.
            .filter(|t| !crate::sharing::elsewhere(t))
            .map(|t| (t.id.clone(), t.key.clone(), t.agent_id.clone()))
            .collect();
        for (ticket_id, key, agent_id) in doing {
            let agent = agent_id.as_deref().and_then(|a| self.live_agent(a));
            let cut_off = agent_id.as_ref().is_some_and(|a| cut.contains(a));
            match (board::recovery(agent.as_ref(), cut_off), agent) {
                (board::Recovery::GoOn, Some(agent)) => {
                    let c = self.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = c
                            .send_or_block(
                                &ticket_id,
                                &agent.id,
                                board::restart_message(lang, &key),
                            )
                            .await;
                    });
                }
                (board::Recovery::Again, _) => self.start_over(&ticket_id, agent_id.as_deref()),
                _ => {}
            }
        }
        self.schedule();
    }

    /// The app stopped while starting the ticket: back to "À faire", and the agent made for it
    /// (never sent its first message), if any, is archived. It was under way: it starts again
    /// even with the autopilot off (one started by "Lancer" would wait otherwise).
    fn start_over(self: &Arc<Self>, ticket_id: &str, agent_id: Option<&str>) {
        let _ = self.edit_ticket(ticket_id, |t| {
            if t.column == Column::Doing && t.blocked.is_none() && t.agent_id.as_deref() == agent_id
            {
                board::back_to_todo(t);
                t.forced = true;
            }
            Ok(())
        });
        let made: Vec<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                let m = &rt.meta;
                (m.ticket_id.as_deref() == Some(ticket_id) && !m.archived && m.prompts == 0)
                    .then(|| m.id.clone())
            })
            .collect();
        for a in made {
            let c = self.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = c.archive_agent(&a, true).await {
                    log::warn!(
                        "agent {a}: made for a ticket that never started, not archived: {e:#}"
                    );
                }
            });
        }
    }

    /// "Reprendre" on a blocked ticket "En cours": its agent goes on (an agent that never got its
    /// first message gets it, with the ticket's description and criteria), or, when it has none
    /// left, the ticket starts again. The block goes as the message is sent.
    pub async fn ticket_resume(self: &Arc<Self>, id: &str) -> Result<()> {
        let lang = self.lang().claude;
        // Its agent waits for its quota: a message now would meet the limit again (and drop the
        // automatic resume, which takes it up).
        let waiting = self
            .ticket(id)
            .ok()
            .filter(|t| t.column == Column::Doing && t.blocked.is_some())
            .and_then(|t| t.agent_id)
            .and_then(|a| self.live_agent(&a))
            .and_then(|m| m.resume_at);
        if let Some(at) = waiting {
            bail!(tr!(
                "En attente du quota — reprise à {at}",
                "Waiting for the quota — resumes at {at}",
                at = board::clock(at)
            ));
        }
        let t = self.edit_ticket(id, |t| {
            if t.column != Column::Doing {
                bail!(tr!(
                    "Ce ticket n'est pas en cours.",
                    "This ticket isn’t in progress."
                ));
            }
            // Taken up already (a second click): its agent is not asked twice.
            if t.blocked.is_none() {
                bail!(tr!(
                    "Ce ticket n'est pas bloqué.",
                    "This ticket isn’t blocked."
                ));
            }
            t.blocked = None;
            t.reminded = false;
            Ok(t.clone())
        })?;
        match t.agent_id.as_deref().and_then(|a| self.live_agent(a)) {
            Some(agent) => {
                // Its start failed once it was made: the ticket was never given to it, nor its
                // images, nor its files.
                let (text, images) = if agent.prompts == 0 {
                    let (images, missing) = self.ticket_attachments(&t).await;
                    let text = board::first_message_with(lang, &t, images.len(), &missing);
                    (self.with_ticket_files(lang, &agent.id, &t, text), images)
                } else {
                    (board::resume_message(lang, &t.key), Vec::new())
                };
                self.send_or_block_with(id, &agent.id, text, images).await
            }
            None => {
                self.edit_ticket(id, |t| {
                    board::back_to_todo(t);
                    t.forced = true;
                    Ok(())
                })?;
                self.schedule();
                Ok(())
            }
        }
    }

    /// An agent was archived or deleted: its ticket "En cours" or "À tester" goes back to
    /// "À faire", from scratch; and what may start starts (its quota wait went with it too).
    pub(crate) fn release_ticket(self: &Arc<Self>, agent_id: &str) {
        self.unlink_ticket(agent_id);
        self.schedule();
    }

    /// What the agents of a ticket cost together: every agent whose `ticket_id` is the ticket's, archived
    /// or not, as `stats_view` attaches them.
    fn ticket_agents_cost(&self, ticket_id: &str) -> f64 {
        self.agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                (rt.meta.ticket_id.as_deref() == Some(ticket_id)).then_some(rt.meta.cost)
            })
            .sum()
    }

    /// The agent's ticket "En cours" or "À tester" goes back to "À faire", from scratch: from
    /// then on, none of the agent's turns moves it.
    pub(crate) fn unlink_ticket(&self, agent_id: &str) {
        let id = self
            .tickets
            .read()
            .iter()
            .find(|t| {
                t.agent_id.as_deref() == Some(agent_id)
                    && matches!(t.column, Column::Doing | Column::Review)
            })
            .map(|t| t.id.clone());
        if let Some(id) = id {
            let _ = self.edit_ticket(&id, |t| {
                // Checked again under the lock: it may have moved since.
                if t.agent_id.as_deref() == Some(agent_id)
                    && matches!(t.column, Column::Doing | Column::Review)
                {
                    board::back_to_todo(t);
                }
                Ok(())
            });
        }
    }

    /// The user set the agent apart from Remote Control, or turned it on for it by hand: its
    /// ticket remembers it for the agents it gets next (sent back to do and started again).
    pub(crate) fn remember_remote_off(&self, agent_id: &str, off: bool) {
        let ticket = self
            .tickets
            .read()
            .iter()
            .find(|t| t.agent_id.as_deref() == Some(agent_id) && t.remote_off != off)
            .map(|t| t.id.clone());
        if let Some(id) = ticket {
            let _ = self.edit_ticket(&id, |t| {
                // Checked again under the lock: it may have moved since.
                if t.agent_id.as_deref() == Some(agent_id) {
                    t.remote_off = off;
                }
                Ok(())
            });
        }
    }

    // ---------- validation ----------

    pub(crate) fn set_step(&self, id: &str, step: &str) {
        let _ = self.edit_ticket(id, |t| {
            if t.column == Column::Review {
                t.step = Some(step.to_string());
            }
            Ok(())
        });
    }

    /// The ticket is still being validated with this agent (archiving or deleting the agent
    /// meanwhile sent it back to do: nothing of its validation goes on), and the agent is not at
    /// work (a message sent during the tests: its files may be half written).
    pub(crate) fn still_validating(&self, id: &str, agent_id: &str) -> Result<()> {
        if !validating(&self.ticket(id)?, agent_id) {
            bail!(Refusal::Changed);
        }
        if self
            .live_agent(agent_id)
            .is_some_and(|m| m.status.is_active())
        {
            bail!(Refusal::AgentBusy);
        }
        Ok(())
    }

    /// The lock held while merging into the repository at `repo`: two validations never merge
    /// into the same folder at once.
    pub(crate) fn merge_lock(&self, repo: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.merge_locks
            .lock()
            .entry(merge_lock_key(repo))
            .or_default()
            .clone()
    }

    /// "Valider…" on a ticket "À tester" (even partial): tests, commit, then what the board's
    /// settings say, each step shown on the card. A failed step leaves it "À tester", blocked
    /// with the reason ("Réessayer" runs it again). Refused while its agent works (its files would
    /// be committed mid-write) and while a validation of it runs (a second click).
    pub async fn ticket_approve(self: &Arc<Self>, id: &str) -> Result<()> {
        // To the end of its cleanup (agent archived, worktree removed), after its step is over.
        let _working = self.working();
        let t = self.approve_begin(id)?;
        self.approve_run(id, t).await;
        Ok(())
    }

    /// "Valider…" for a client that does not wait for its end (the MCP server): refused as the
    /// window's, then run in the background; follow it on the ticket (step, blocked, column).
    pub fn ticket_approve_detached(self: &Arc<Self>, id: &str) -> Result<()> {
        // Held before the first step is set on the ticket and to the end of the background run: the
        // app does not restart for an update in between.
        let working = self.working_owned();
        let t = self.approve_begin(id)?;
        let (core, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move {
            let _working = working;
            core.approve_run(&id, t).await;
        });
        Ok(())
    }

    /// The checks of an approval, and its first step set on the ticket ("Validation…"): from then
    /// on it is held, and a second one is refused.
    fn approve_begin(self: &Arc<Self>, id: &str) -> Result<Ticket> {
        let t = self.ticket(id)?;
        if t.column != Column::Review {
            bail!(Refusal::NotInReview);
        }
        let busy = t
            .agent_id
            .as_deref()
            .and_then(|a| self.live_agent(a))
            .is_some_and(|m| m.status.is_active());
        if busy {
            bail!(Refusal::AgentBusy);
        }
        self.edit_ticket(id, |t| {
            if t.column != Column::Review {
                bail!(Refusal::NotInReview);
            }
            if t.step.is_some() {
                bail!(Refusal::ApprovingAlready);
            }
            // Under the tickets' lock, as the closing looks for a validation under way: one of
            // the two sees the other.
            if self.is_closing(&t.project_id) {
                bail!(Refusal::ProjectClosing);
            }
            t.blocked = None;
            t.conflict = false;
            t.step = Some(tr!("Validation…", "Approving…"));
            Ok(t.clone())
        })
        .inspect(|t| {
            self.facts.emit(
                crate::facts::Body::TicketApproved {},
                crate::facts::About::ticket(&t.id),
            )
        })
    }

    /// The approval `approve_begin` started: its validation, a failed step blocking the ticket,
    /// then what may start starts.
    async fn approve_run(self: &Arc<Self>, id: &str, t: Ticket) {
        if let Err(e) = self.validate(&t).await {
            log::warn!("ticket {}: validation failed: {e:#}", t.key);
            let reason = error_line(&e);
            // One that left "À tester" meanwhile (back with its agent, or to do, maybe started
            // again with another agent since) stays as it is.
            let blocked = self.edit_ticket(id, |x| {
                let here = x.column == Column::Review && x.agent_id == t.agent_id;
                if here {
                    x.step = None;
                    x.blocked = Some(reason);
                }
                Ok(here)
            });
            if blocked.unwrap_or(false) {
                self.notify_ticket(id, false);
            }
        }
        self.schedule();
    }

    async fn validate(self: &Arc<Self>, t: &Ticket) -> Result<()> {
        let lang = self.lang().claude;
        let project = self.project(&t.project_id)?;
        let s = project.board.clone();
        let agent_id = t.agent_id.clone().ok_or(Refusal::NoAgent)?;
        let meta = self.agent(&agent_id)?.lock().meta.clone();
        let wt = meta.worktree.clone().ok_or(Refusal::AgentWithoutWorktree)?;
        let target = self.target_of(&project).await;
        // Its test launches stop first: their servers would hold the worktree's files and ports.
        self.stop_test_runs(&agent_id);
        // Tests.
        if s.tests_first_on() {
            self.set_step(&t.id, "Tests…");
            let settings = self.settings.read().clone();
            let shell = crate::pty::detect_shells(&settings)
                .into_iter()
                .next()
                .ok_or_else(|| {
                    anyhow!(tr!(
                        "Aucun shell pour lancer les tests.",
                        "No shell to run the tests."
                    ))
                })?;
            let command = s.test_command.trim();
            let env = testlaunch::port_env(meta.port_base);
            let run =
                testlaunch::run_tests(&shell, &wt.path, command, &env, TEST_LIMIT, lang).await?;
            self.facts.emit(
                crate::facts::Body::TestsResult { passed: run.passed },
                crate::facts::About::ticket(&t.id),
            );
            if !run.passed {
                // Its agent archived or deleted during the tests: the ticket went back to do (and
                // maybe on with another agent); the old agent is told nothing (it would start again).
                self.edit_ticket(&t.id, |x| {
                    if !validating(x, &agent_id) {
                        bail!(Refusal::Changed);
                    }
                    board::back_to_work(x);
                    Ok(())
                })?;
                // Not a failure of the validation: the ticket is back with its agent.
                let _ = self
                    .send_or_block(
                        &t.id,
                        &agent_id,
                        board::tests_failed_message(lang, command, &run.tail),
                    )
                    .await;
                return Ok(());
            }
        }
        // What it changed, for the pages of the spec it touches (ESC-263), read before the commit and
        // the merge, against where its branch left the target.
        let changed = if wt.is_multi() {
            Vec::new()
        } else {
            crate::docscompare::changed_files(&wt.path, &target).await
        };
        // The decisions its agent left are accepted by this validation (docs/FACTORY.md), before
        // the commit, so they are merged with the ticket. One that cannot be written stops nothing.
        match self
            .docs_accept_ticket_decisions(&project.id, &agent_id, &t.key, &target)
            .await
        {
            Ok(n) if !n.is_empty() => {
                log::info!("ticket {}: decisions {n:?} accepted", t.key);
                self.docs_changed(&project.id).await;
            }
            Ok(_) => {}
            Err(e) => log::warn!("ticket {}: its decisions not accepted: {e:#}", t.key),
        }
        // The files copied from the project into the worktree (`.env`…), the list the copy took:
        // never committed, merged or pushed. Git ignores them, so only an agent that forced one in
        // could; checked all the same.
        let (outcome, nothing) = if wt.is_multi() {
            // Of several repositories (ESC-169): each one's, checked first, all or nothing.
            match self.validate_repos(t, &s, &project, &wt, &agent_id).await? {
                Some(done) => done,
                // Stopped on a conflict, handled as the settings say.
                None => return Ok(()),
            }
        } else {
            let copied = testlaunch::matching_ignored(&project.path, &project.worktree_copy).await;
            self.still_validating(&t.id, &agent_id)?;
            let mut message = None;
            // The branch brings nothing (no change, or only changes taken back): the ticket is done
            // all the same, with nothing to merge, propose or push.
            let mut nothing = false;
            if s.action != "keep" {
                if !git::branch_exists(&wt.path, &target).await {
                    bail!(tr!(
                        "La branche cible {target} n'existe pas.",
                        "The target branch {target} doesn’t exist."
                    ));
                }
                // Commit what is left in the worktree.
                self.set_step(&t.id, "Commit…");
                let unmerged = git::unmerged(&wt.path).await;
                if !unmerged.is_empty() {
                    bail!(tr!(
                        "Conflits non résolus dans le worktree sur : {files}",
                        "Unresolved conflicts in the worktree on: {files}",
                        files = unmerged.join(", ")
                    ));
                }
                // A commit elsewhere (another branch, a detached HEAD) would never reach the target.
                if git::current_branch(&wt.path).await != wt.branch {
                    bail!(tr!(
                        "Le worktree n'est plus sur la branche {b}",
                        "The worktree is no longer on the branch {b}",
                        b = wt.branch
                    ));
                }
                self.still_validating(&t.id, &agent_id)?;
                let keep_out = not_committed(&wt.path, &copied).await;
                if git::stage_all(&wt.path, &keep_out).await? {
                    let m = self.commit_message(t, &s, &wt.path, &target).await;
                    git::commit_staged(&wt.path, &m).await?;
                    message = Some(m);
                }
                // One the agent committed itself goes no further than its branch: in its tree, or
                // only in its history (taken out again since, which a merge commit or a rebase would
                // bring along all the same).
                let in_tree = git::changed_in(&wt.path, &target, &wt.branch, &copied).await?;
                let in_history = git::touched_by(&wt.path, &target, &wt.branch, &copied).await?;
                if let Some(refusal) = board::copied_refusal(i18n::ui(), &in_tree, &in_history) {
                    bail!(refusal);
                }
                nothing = git::ahead_of(&wt.path, &target, &wt.branch).await? == 0;
                self.still_validating(&t.id, &agent_id)?;
            }
            // Written in the interface's language (the card) and in that of the texts for Claude (the
            // comment published for the team).
            let lang = self.lang().claude;
            let outcome: board::Outcome = match s.action.as_str() {
                _ if nothing => board::Outcome::of(lang, board::nothing_outcome),
                "merge" => {
                    self.set_step(&t.id, "Merge…");
                    match self
                        .merge_ticket(t, &s, &project, &wt, &target, message)
                        .await?
                    {
                        Some(done) => done,
                        // Stopped on a conflict, handled as the settings say.
                        None => return Ok(()),
                    }
                }
                "pr" => {
                    self.set_step(&t.id, "Push…");
                    let o = self.open_pr(t, &s, &wt, &target, message).await?;
                    if let Some(url) = &o.url {
                        self.facts.emit(
                            crate::facts::Body::PrOpened {
                                url: url.clone(),
                                target: target.clone(),
                            },
                            crate::facts::About::ticket(&t.id),
                        );
                    }
                    o
                }
                "push" => {
                    self.set_step(&t.id, "Push…");
                    git::push_branch(&wt.path, &wt.branch).await?;
                    board::Outcome::of(lang, |l| board::pushed_outcome(l, &wt.branch))
                }
                _ => board::Outcome::of(lang, board::kept_outcome),
            };
            (outcome, nothing)
        };
        // What every agent of the ticket cost, archived ones included (a ticket sent back and taken
        // over has several), as « À tester » and the statistics add it up; at least its own.
        let own = self
            .agent(&agent_id)
            .map(|h| h.lock().meta.cost)
            .unwrap_or(meta.cost);
        let cost = self.ticket_agents_cost(&t.id).max(own);
        // Its process, killed for good before its worktree goes (see `remove_worktree_of`).
        let proc = self
            .agent(&agent_id)
            .ok()
            .and_then(|h| h.lock().proc.clone());
        let merged = (s.action == "merge" && !nothing).then(|| crate::facts::Body::TicketMerged {
            target: target.clone(),
            commit: outcome.commit.clone(),
            url: outcome.url.clone(),
        });
        self.edit_ticket(&t.id, |x| {
            // Its agent archived or deleted meanwhile: it went back to do, and stays there.
            if !validating(x, &agent_id) {
                bail!(Refusal::Changed);
            }
            x.column = Column::Done;
            x.step = None;
            x.blocked = None;
            x.conflict = false;
            x.outcome = Some(outcome.ui);
            x.outcome_claude = Some(outcome.claude);
            x.outcome_url = outcome.url;
            x.outcome_commit = outcome.commit;
            x.done_at = Some(now_ms());
            x.cost = cost;
            Ok(())
        })?;
        if let Some(merged) = merged {
            self.facts.emit(merged, crate::facts::About::ticket(&t.id));
        }
        // Validated: on a fixed branch of the docs, the spec and decisions its agent wrote are poured
        // there now, never before; then the pages it touches are compared to the code, once merged.
        let poured = match self.docs_pour_ticket(&project.id, &t.key).await {
            Ok(p) => p,
            Err(e) => {
                log::warn!(
                    "ticket {}: its docs not poured on the docs branch: {e:#}",
                    t.key
                );
                None
            }
        };
        if let Some(p) = &poured {
            self.facts.emit(
                crate::facts::Body::DocsFiled {
                    pages: p.pages.len(),
                },
                crate::facts::About::ticket(&t.id),
            );
            self.docs_changed(&project.id).await;
        }
        if s.action == "merge" && !nothing {
            self.docs_after_ticket(&project.id, &t.key, changed, poured);
        }
        if s.action != "keep" {
            // Done whatever becomes of its agent: its work went where it was sent, and "Réessayer"
            // on it would have nothing left to do.
            if let Err(e) = self.archive_agent(&agent_id, true).await {
                log::warn!("ticket {}: its agent was not archived: {e:#}", t.key);
            }
            if s.cleanup {
                // Merged, or with nothing on it, its branch goes too; pushed or proposed, it stays
                // (and an agent restored gets its worktree back from it).
                let keep_branch = board::keeps_branch(wt.existing, nothing, &s.action);
                let ports = meta.port_base;
                self.remove_worktree_of(t, &project, &wt, proc, ports, keep_branch)
                    .await;
            }
        } else if s.cleanup && wt.is_multi() {
            // Left as it is, of several repositories: each worktree that holds nothing goes.
            if let Err(e) = self.archive_agent(&agent_id, true).await {
                log::warn!("ticket {}: its agent was not archived: {e:#}", t.key);
            }
            if let crate::leftovers::Cleaned::Kept(why) = self
                .clean_agent_worktree(&agent_id, proc, meta.port_base)
                .await
            {
                self.append_outcome(&t.id, |_| format!(" · {why}"));
            }
        } else if s.cleanup {
            // Left as it is (ESC-143): its worktree goes too, with its agent archived, when it
            // holds nothing that would be lost; else both stay, and the outcome says why.
            let found = leftovers::unsaved_work(&project.path, &wt, &target).await;
            match found.unsaved {
                Some(unsaved) => self.append_outcome(&t.id, |l| format!(" · {}", unsaved.text(l))),
                None => {
                    if let Err(e) = self.archive_agent(&agent_id, true).await {
                        log::warn!("ticket {}: its agent was not archived: {e:#}", t.key);
                    }
                    let keep_branch = wt.existing || !found.merged;
                    let ports = meta.port_base;
                    self.remove_worktree_of(t, &project, &wt, proc, ports, keep_branch)
                        .await;
                }
            }
        }
        if s.cleanup {
            // Its earlier agents (sent back, then taken over by another): their worktrees too.
            self.clean_ticket_worktrees(&t.id, &agent_id).await;
        }
        Ok(())
    }

    /// Haiku's commit message in the Conventional Commits form, checked, else
    /// `feat: <title> [<key>]`; without "Message de commit généré", `<key> <title>`.
    pub(crate) async fn commit_message(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        cwd: &str,
        target: &str,
    ) -> String {
        if !s.conventional {
            return board::plain_commit(t);
        }
        let stat = git::staged_stat(cwd, target).await;
        let lang = self.lang().claude;
        // Asked of the account of the ticket's agent.
        let agent = t.agent_id.as_deref();
        match self
            .one_shot(
                &board::commit_system(lang),
                &board::commit_prompt(lang, t, &stat),
                agent,
                Some(&t.project_id),
            )
            .await
        {
            Ok(answer) => board::commit_from_answer(&answer, board::shown_key(t))
                .unwrap_or_else(|| board::fallback_commit(t)),
            Err(e) => {
                log::info!("ticket {}: no commit message from Haiku: {e:#}", t.key);
                board::fallback_commit(t)
            }
        }
    }

    /// The merge step: in the project's folder when the target is checked out there (refused with
    /// uncommitted changes), refused when an agent's worktree has it, else in a temporary
    /// worktree. None when it stopped on a conflict.
    async fn merge_ticket(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        project: &Project,
        wt: &Worktree,
        target: &str,
        message: Option<String>,
    ) -> Result<Option<board::Outcome>> {
        let repo = self.toplevel(&project.path).await.ok_or_else(not_a_repo)?;
        self.merge_ticket_in(t, s, project, wt, target, message, &repo)
            .await
    }

    /// `merge_ticket` into the repository `repo` (one of several, ESC-169), `wt` its worktree.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn merge_ticket_in(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        project: &Project,
        wt: &Worktree,
        target: &str,
        message: Option<String>,
        repo: &str,
    ) -> Result<Option<board::Outcome>> {
        let repo = repo.to_string();
        let message = match message {
            Some(m) => m,
            None => self.commit_message(t, s, &wt.path, target).await,
        };
        // Two validations never merge into the same folder at once: the second one waits.
        let lock = self.merge_lock(&repo);
        let merging = lock.lock().await;
        // Its agent may have been archived or set to work during Haiku's answer and that wait.
        self.still_validating(&t.id, t.agent_id.as_deref().unwrap_or_default())?;
        let merges = merges_root(project, &repo);
        // Named after the project too: a folder of worktrees may be shared with another
        // repository, whose validations do not wait for this one's.
        let tmp = merges.join(format!(".merge-{}-{}", project.id, t.key.to_lowercase()));
        let tmp_s = tmp.to_string_lossy().to_string();
        // One left by a validation the app's stop cut: registered, it would still hold the target
        // and every merge into it would be refused.
        drop_worktree(&repo, &tmp).await;
        let mut checkout = git::checkout_of(&repo, target).await?;
        // Another ticket's, left the same way (no other merge into this repository runs now).
        if let Some(p) = checkout.clone().filter(|p| merge_leftover(p)) {
            drop_worktree(&repo, Path::new(&p)).await;
            checkout = git::checkout_of(&repo, target).await?;
        }
        let (dir, temporary) = match checkout {
            Some(p) if same_dir(&p, &repo) => {
                if git::has_tracked_changes_but_tickets(&repo).await? {
                    bail!(tr!(
                        "Le dossier du projet a des modifications non commitées sur {target}",
                        "The project’s folder has uncommitted changes on {target}"
                    ));
                }
                (repo.clone(), false)
            }
            Some(p) => {
                let who = self
                    .project_agents(&project.id)
                    .iter()
                    .map(|h| h.lock().meta.clone())
                    .find(|m| m.worktree.as_ref().is_some_and(|w| same_dir(&w.path, &p)))
                    .map(|m| m.name)
                    .unwrap_or(p);
                bail!(tr!(
                    "{target} est extraite dans le worktree de {who}",
                    "{target} is checked out in the worktree of {who}"
                ));
            }
            None => {
                git::exclude_worktrees(&repo, &merges).await?;
                git::run_hooks(&repo, &["worktree", "add", &tmp_s, target]).await?;
                (tmp_s.clone(), true)
            }
        };
        let result =
            git::integrate(&dir, &wt.branch, &s.strategy, &message, &wt.path, target).await;
        if temporary {
            drop_worktree(&repo, &tmp).await;
        }
        self.git.refresh(&project.id);
        // The commit the merge made, read before another merge into the target may go.
        let commit = match &result {
            Ok(git::Integrated::Done) => git::branch_head(&repo, target).await,
            _ => None,
        };
        // The target is as it was: the next merge into this repository may go.
        drop(merging);
        match result? {
            git::Integrated::Done => Ok(Some(
                board::Outcome::of(self.lang().claude, |l| {
                    board::merged_outcome(l, target, &s.strategy)
                })
                .with_commit(commit),
            )),
            git::Integrated::Conflict(files) => {
                self.on_conflict(t, s, target, files).await?;
                Ok(None)
            }
        }
    }

    /// Once validated, with "Supprimer le worktree": the ticket's worktree goes, its branch too
    /// unless `keep_branch`. Its agent's process is killed first with all it started: the archive
    /// closed its input, but it may take its time to end (or something still holds it), and on
    /// Windows a live process keeps the folder. The project's teardown then runs in it (`ports`:
    /// those its agent held). The removal is tried again for a few seconds; when it still fails,
    /// the ticket's outcome says the worktree is kept.
    async fn remove_worktree_of(
        &self,
        t: &Ticket,
        project: &Project,
        wt: &Worktree,
        proc: Option<Process>,
        ports: Option<u16>,
        keep_branch: bool,
    ) {
        let removed = self
            .remove_worktree(project, wt, proc, ports, keep_branch)
            .await;
        if let Err(e) = removed {
            log::warn!("ticket {}: worktree not removed: {e:#}", t.key);
            self.append_outcome(&t.id, board::worktree_kept);
        }
    }

    /// Adds `more` (in each language) to the ticket's outcome, on its card and for Claude.
    fn append_outcome(&self, ticket_id: &str, more: impl Fn(Lang) -> String) {
        let claude = self.lang().claude;
        let _ = self.edit_ticket(ticket_id, |x| {
            for (outcome, lang) in [
                (&mut x.outcome, i18n::ui()),
                (&mut x.outcome_claude, claude),
            ] {
                if let Some(outcome) = outcome.as_mut() {
                    outcome.push_str(&more(lang));
                }
            }
            Ok(())
        });
    }

    // ---------- pull requests, conflicts, rejection ----------

    /// The GitHub CLI: the one known, else looked for again on the PATH (installed since the app
    /// started), and kept once found.
    pub(crate) fn gh_cli(&self) -> Option<PathBuf> {
        let known = self.gh.read().clone();
        if known.is_some() {
            return known;
        }
        #[cfg(test)]
        let found = self.gh_on_path.read().clone();
        #[cfg(not(test))]
        let found = crate::core::locate_gh();
        if let Some(gh) = &found {
            *self.gh.write() = Some(gh.clone());
        }
        found
    }

    /// Pushes the ticket's branch, then opens its PR with `gh` on GitHub; without `gh` (or when it
    /// fails), GitHub's page to finish it opens in the browser; another host: pushed only.
    pub(crate) async fn open_pr(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        wt: &Worktree,
        target: &str,
        message: Option<String>,
    ) -> Result<board::Outcome> {
        let remote = git::push_branch(&wt.path, &wt.branch).await?;
        let title = match message {
            Some(m) => m,
            None => self.commit_message(t, s, &wt.path, target).await,
        };
        let lang = self.lang().claude;
        let mut body = board::pr_body(lang, t);
        let item = crate::agentprs::work_item_of(t);
        let github = git::remote_url(&wt.path, &remote)
            .await
            .as_deref()
            .and_then(board::github_repo);
        let Some((owner, repo)) = github else {
            if let Some(outcome) = self
                .open_azure_pr(t, s, wt, target, &title, &body, item)
                .await
            {
                return Ok(outcome);
            }
            return Ok(board::Outcome::of(lang, |l| {
                board::pushed_elsewhere_outcome(l, &wt.branch)
            }));
        };
        let mut draft = crate::forge::PullRequestDraft {
            body,
            ..Default::default()
        };
        crate::agentprs::link_work_item(item, "github", &mut draft);
        body = draft.body;
        if let Some(gh) = self.gh_cli() {
            let pr = PullRequest {
                repo: &format!("{owner}/{repo}"),
                base: target,
                head: &wt.branch,
                title: &title,
                body: &body,
                draft: s.draft,
            };
            match gh_pr_create(&gh, &wt.path, &pr).await {
                Ok(out) => {
                    if let Some((n, url)) = board::pr_number(&out) {
                        let outcome = board::Outcome::of(lang, |_| board::pr_outcome(n, target));
                        return Ok(outcome.with_url(url));
                    }
                    log::warn!("ticket {}: no pull request in gh's answer: {out}", t.key);
                }
                Err(e) => log::warn!("ticket {}: gh pr create failed: {e:#}", t.key),
            }
        }
        let url = board::compare_url(&owner, &repo, target, &wt.branch, &title, &body);
        self.hub.emit(UiEvent::OpenUrl { url: url.clone() });
        let outcome = board::Outcome::of(lang, |l| board::pushed_for_pr_outcome(l, &wt.branch));
        Ok(outcome.with_url(url))
    }

    /// The ticket's PR on Azure Repos, opened with the Azure DevOps account (`open_pr` asks it of
    /// a remote not on GitHub), its Azure Boards work item linked (`workItemRefs`). None on
    /// another host, without an account, or when Azure refuses: the branch is pushed only.
    #[allow(clippy::too_many_arguments)]
    async fn open_azure_pr(
        &self,
        t: &Ticket,
        s: &BoardSettings,
        wt: &Worktree,
        target: &str,
        title: &str,
        body: &str,
        item: Option<u64>,
    ) -> Option<board::Outcome> {
        let remote = self.remote_of(&wt.path, target).await?;
        if remote.kind.as_deref() != Some("azure") {
            return None;
        }
        let (client, account) = match self.forge_client(&remote.host).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("ticket {}: no Azure DevOps client: {e:#}", t.key);
                return None;
            }
        };
        let mut draft = crate::forge::PullRequestDraft {
            title: title.to_string(),
            body: body.to_string(),
            head: wt.branch.clone(),
            base: target.to_string(),
            draft: s.draft,
            ..Default::default()
        };
        crate::agentprs::link_work_item(item, &account.kind, &mut draft);
        match client.create_pr(&remote.repo, &draft).await {
            Ok(pr) => {
                let n = pr.number as u32;
                Some(
                    board::Outcome::of(self.lang().claude, |_| board::pr_outcome(n, target))
                        .with_url(pr.url),
                )
            }
            Err(e) => {
                log::warn!("ticket {}: Azure DevOps pull request failed: {e:#}", t.key);
                None
            }
        }
    }

    /// A merge stopped on conflicts (undone): blocked with "L'agent résout" / "Annuler" ("ask"),
    /// handed to the agent ("agent"), or blocked with the files ("abort").
    pub(crate) async fn on_conflict(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        target: &str,
        files: Vec<String>,
    ) -> Result<()> {
        if s.conflict == "agent" {
            return self.agent_resolves(&t.id).await;
        }
        let ask = s.conflict != "abort";
        self.facts.emit(
            crate::facts::Body::TicketConflict {
                target: target.to_string(),
                files: files.clone(),
            },
            crate::facts::About::ticket(&t.id),
        );
        let reason = conflict_reason(i18n::ui(), target, &files, ask);
        let agent_id = t.agent_id.clone().unwrap_or_default();
        self.edit_ticket(&t.id, |x| {
            // Its agent archived or deleted meanwhile: it went back to do, and stays there.
            if !validating(x, &agent_id) {
                bail!(Refusal::Changed);
            }
            x.step = None;
            x.conflict = ask;
            x.blocked = Some(reason);
            Ok(())
        })?;
        self.notify_ticket(&t.id, false);
        Ok(())
    }

    /// "L'agent résout": the target is merged into the ticket's branch, its conflicts left in the
    /// worktree for the agent, or the agent is asked to rebase; the ticket goes back "En cours",
    /// loop 1. A merge git refuses to start (changes in the way…) is the error: the agent is told
    /// nothing. Runs while the ticket is held, its step set (its validation, or
    /// `ticket_resolve_conflict`).
    async fn agent_resolves(self: &Arc<Self>, id: &str) -> Result<()> {
        let lang = self.lang().claude;
        let t = self.ticket(id)?;
        let project = self.project(&t.project_id)?;
        let agent_id = t.agent_id.clone().ok_or(Refusal::NoAgent)?;
        let wt = self
            .agent(&agent_id)?
            .lock()
            .meta
            .worktree
            .clone()
            .ok_or(Refusal::AgentWithoutWorktree)?;
        let target = self.target_of(&project).await;
        // Nothing goes into the worktree of an agent archived meanwhile, or at work.
        self.still_validating(id, &agent_id)?;
        let rebase = project.board.strategy == "rebase";
        let text = if wt.is_multi() {
            // Of several repositories (ESC-169): each one's target into its worktree.
            let mut files = Vec::new();
            if !rebase {
                for c in wt.checkouts() {
                    let base = if c.base.is_empty() {
                        target.clone()
                    } else {
                        c.base.clone()
                    };
                    if let Err(e) = git::run_hooks(&c.dir, &["merge", "--no-edit", &base]).await {
                        let unmerged = git::unmerged(&c.dir).await;
                        if unmerged.is_empty() {
                            return Err(e.context(crate::repos::label(&c.repo)));
                        }
                        files.extend(unmerged.iter().map(|f| crate::repos::prefixed(&c.repo, f)));
                    }
                }
            }
            if rebase {
                board::rebase_message(lang, &target)
            } else {
                board::conflict_message(lang, &target, &files)
            }
        } else if rebase {
            board::rebase_message(lang, &target)
        } else {
            match git::run_hooks(&wt.path, &["merge", "--no-edit", &target]).await {
                Ok(_) => board::conflict_message(lang, &target, &[]),
                Err(e) => {
                    let files = git::unmerged(&wt.path).await;
                    if files.is_empty() {
                        return Err(e.context(tr!(
                            "Merge de {target} dans {b} impossible",
                            "Can’t merge {target} into {b}",
                            b = wt.branch
                        )));
                    }
                    board::conflict_message(lang, &target, &files)
                }
            }
        };
        let handed = self.edit_ticket(id, |x| {
            if !validating(x, &agent_id) {
                bail!(Refusal::Changed);
            }
            board::back_to_work(x);
            Ok(())
        });
        if let Err(e) = handed {
            // It left meanwhile (its agent archived): its worktree is not left mid-merge.
            if !rebase {
                for c in wt.checkouts() {
                    let _ = git::run(&c.dir, &["merge", "--abort"]).await;
                }
            }
            return Err(e);
        }
        self.send_or_block(id, &agent_id, text).await
    }

    /// "L'agent résout" on the card of a ticket "À tester": refused while a validation of it runs
    /// (its merge may be under way), and held as one meanwhile (a second click, "Réessayer").
    pub async fn ticket_resolve_conflict(self: &Arc<Self>, id: &str) -> Result<()> {
        let agent_id = self.edit_ticket(id, |t| {
            if t.column != Column::Review {
                bail!(Refusal::NotInReview);
            }
            if t.step.is_some() {
                bail!(Refusal::ApprovingAlready);
            }
            if self.is_closing(&t.project_id) {
                bail!(Refusal::ProjectClosing);
            }
            t.step = Some("Merge…".into());
            Ok(t.agent_id.clone())
        })?;
        let resolved = self.agent_resolves(id).await;
        if resolved.is_err() {
            // Not handed over: the ticket is as it was, its block still shown.
            let _ = self.edit_ticket(id, |x| {
                if x.column == Column::Review && x.agent_id == agent_id {
                    x.step = None;
                }
                Ok(())
            });
        }
        resolved
    }

    /// "Annuler" on a ticket "À tester" (a conflict, a failed step): the block goes, the ticket
    /// stays to test.
    pub fn ticket_dismiss(&self, id: &str) -> Result<()> {
        self.edit_ticket(id, |t| {
            if t.column != Column::Review {
                bail!(Refusal::NotInReview);
            }
            t.blocked = None;
            t.conflict = false;
            Ok(())
        })
    }

    /// "Renvoyer": back "En cours" with the same agent and worktree, loop 1, with what is wrong.
    /// Refused while its agent works: the end of that turn would be read as the end of the
    /// rework.
    pub async fn ticket_reject(self: &Arc<Self>, id: &str, comment: &str) -> Result<()> {
        let comment = comment.trim();
        if comment.is_empty() {
            return Err(comment_missing());
        }
        let busy = self
            .ticket(id)?
            .agent_id
            .and_then(|a| self.live_agent(&a))
            .is_some_and(|m| m.status.is_active());
        if busy {
            bail!(Refusal::AgentBusyReject);
        }
        let (key, agent_id) = self.edit_ticket(id, |t| {
            if t.column != Column::Review {
                bail!(Refusal::NotInReview);
            }
            if t.step.is_some() {
                bail!(Refusal::ApprovingAlready);
            }
            let agent_id = t.agent_id.clone().ok_or(Refusal::NoAgent)?;
            board::back_to_work(t);
            Ok((t.key.clone(), agent_id))
        })?;
        self.facts.emit(
            crate::facts::Body::TicketRejected {
                comment: comment.chars().take(2000).collect(),
            },
            crate::facts::About::ticket(id),
        );
        self.send_or_block(
            id,
            &agent_id,
            board::reject_message(self.lang().claude, &key, comment),
        )
        .await
    }

    // ---------- test launches ----------

    /// "Préparer le lancement": reserves the agent's block of ports if it has none, then asks it
    /// for its recipe, as any message (during a turn, Claude Code takes it once the turn is over).
    /// Only for an agent with a worktree, and not archived (it would hold a block for nothing).
    pub async fn agent_prepare_launch(self: &Arc<Self>, id: &str) -> Result<()> {
        let h = self.agent(id)?;
        let (worktree, archived, base) = {
            let rt = h.lock();
            (
                rt.meta.worktree.clone(),
                rt.meta.archived,
                rt.meta.port_base,
            )
        };
        let Some(worktree) = worktree else {
            bail!(Refusal::NoWorktree);
        };
        if isola::manages(&worktree.path) {
            bail!(tr!(
                "isola lance ce worktree : « ▶ Tester » suffit, sans recette.",
                "isola runs this worktree: “▶ Test” is enough, without a recipe."
            ));
        }
        if archived {
            bail!(Refusal::Archived);
        }
        let base = self.ensure_ports(&h, base)?;
        // Archived meanwhile, whichever way it got its block: it is not woken up for nothing.
        if h.lock().meta.archived {
            bail!(Refusal::Archived);
        }
        // The app's own text, not a task of the user's: it never names the agent. The project's
        // launch commands are where its recipe starts from.
        let pid = h.lock().meta.project_id.clone();
        let commands = self
            .project(&pid)
            .map(|p| p.run_commands)
            .unwrap_or_default();
        self.send_by_app(
            id,
            board::prepare_message(self.lang().claude, base, &commands),
        )
        .await
    }

    /// The agent's block of ports (`base`, as it was read), reserved now if it has none.
    fn ensure_ports(&self, h: &crate::agent::AgentHandle, base: Option<u16>) -> Result<u16> {
        if let Some(b) = base {
            return Ok(b);
        }
        // Reserved, then written on the agent with no wait in between: no other start
        // or preparation gets this block meanwhile.
        let reserved = self.reserve_ports().ok_or_else(|| {
            anyhow!(tr!(
                "Aucun bloc de ports libre à partir de 4100.",
                "No free block of ports from 4100 on."
            ))
        })?;
        let held = {
            let mut rt = h.lock();
            // Archived meanwhile, it holds no block; given one meanwhile (another
            // preparation at once), it keeps that one.
            (!rt.meta.archived).then(|| *rt.meta.port_base.get_or_insert(reserved))
        };
        // The agent holds its block from now on (or the one reserved is free again).
        self.unreserve_ports(Some(reserved));
        let base = held.ok_or_else(|| anyhow!(Refusal::Archived))?;
        self.emit_agent(h);
        self.request_save();
        Ok(base)
    }

    /// What the project's launch command `command_id` runs in the worktree of its agent
    /// `agent_id` (`testlaunch::worktree_run_spec`), with the agent's block of ports, reserved now
    /// if it has none. Refused while its ticket is validated, whose tests would find its ports
    /// taken. Once started, it is kept with its test launches (`track_test_run`), to stop with them.
    pub fn worktree_run(
        &self,
        project_id: &str,
        agent_id: &str,
        command_id: &str,
    ) -> Result<testlaunch::RunSpec> {
        let lang = i18n::ui();
        let project = self.project(project_id)?;
        let run = project
            .run_commands
            .iter()
            .find(|c| c.id == command_id)
            .ok_or_else(|| {
                anyhow!(tr!(
                    "commande de lancement introuvable",
                    "launch command not found"
                ))
            })?;
        let h = self.agent(agent_id)?;
        let meta = h.lock().meta.clone();
        if meta.project_id != project_id {
            bail!(tr!(
                "Cet agent n'est pas de ce projet.",
                "This agent isn't in this project."
            ));
        }
        // Its own reasons first (archived, no worktree, folder missing), then the validation's.
        testlaunch::worktree_run_spec(&meta, run, lang)?;
        if self.tickets.read().iter().any(|t| validating(t, agent_id)) {
            bail!(tr_in!(
                lang,
                "Le ticket de « {name} » est en validation : rien ne se lance dans son worktree avant sa fin.",
                "The ticket of “{name}” is being approved: nothing runs in its worktree before it’s over.",
                name = meta.name
            ));
        }
        self.ensure_ports(&h, meta.port_base)?;
        let meta = h.lock().meta.clone();
        testlaunch::worktree_run_spec(&meta, run, lang)
    }

    /// Why the agent may run no test launch now: archived (its launches stopped with it), without
    /// a worktree, or its ticket being validated (or its conflict handed to it), whose tests
    /// would find its ports taken.
    fn launch_refusal(&self, meta: &AgentMeta) -> Option<Refusal> {
        if meta.archived {
            Some(Refusal::Archived)
        } else if meta.worktree.is_none() {
            Some(Refusal::NoWorktree)
        } else if self.tickets.read().iter().any(|t| validating(t, &meta.id)) {
            Some(Refusal::Validating)
        } else {
            None
        }
    }

    /// The user read `recipe` in the test modal and lets it run. Only the recipe the agent holds
    /// right now is approved: one that changed since (the agent answered again while the user
    /// read) is refused, so that what runs is always what was shown. Kept with the agent.
    pub fn approve_recipe(&self, agent_id: &str, recipe: TestRecipe) -> Result<()> {
        let h = self.agent(agent_id)?;
        {
            let mut rt = h.lock();
            if rt.meta.recipe.as_ref() != Some(&recipe) {
                bail!(Refusal::RecipeChanged);
            }
            rt.meta.approved_recipe = Some(recipe);
        }
        self.emit_agent(&h);
        self.request_save();
        Ok(())
    }

    /// The `.isola.toml` of the agent's worktree as isola will read it, to show before it runs.
    pub fn isola_config(&self, agent_id: &str) -> Result<String> {
        let meta = self.agent(agent_id)?.lock().meta.clone();
        let wt = isola_worktree(&meta)?;
        isola::read_config(&wt.path)
    }

    /// The user read `config` (the worktree's `.isola.toml`) and `open` (the address the agent gave) in
    /// the test modal and lets `isola up` run. Refused when the file or the address is no longer what
    /// they were shown. Kept with the agent, whole: any other content is asked again.
    pub fn approve_isola(&self, agent_id: &str, config: String, open: String) -> Result<()> {
        let h = self.agent(agent_id)?;
        let meta = h.lock().meta.clone();
        let wt = isola_worktree(&meta)?;
        if isola::read_config(&wt.path)? != config {
            bail!(Refusal::IsolaConfigChanged);
        }
        {
            let mut rt = h.lock();
            let current = rt.meta.recipe.as_ref().map_or("", |r| r.open.as_str());
            if current != open {
                bail!(Refusal::RecipeChanged);
            }
            rt.meta.approved_isola = Some(IsolaApproval { config, open });
        }
        self.emit_agent(&h);
        self.request_save();
        Ok(())
    }

    /// What step `index` of `kind` ("prep" or "run") of the agent's recipe runs, when it may run
    /// one (`launch_refusal`).
    pub fn test_run_spec(
        &self,
        agent_id: &str,
        kind: &str,
        index: usize,
    ) -> Result<testlaunch::RunSpec> {
        let meta = self.agent(agent_id)?.lock().meta.clone();
        if let Some(why) = self.launch_refusal(&meta) {
            bail!(why);
        }
        if kind == "isola" {
            let wt = isola_worktree(&meta)?;
            testlaunch::check_isola_approved(&meta, &isola::read_config(&wt.path)?)?;
            return Ok(testlaunch::RunSpec {
                name: "isola up".into(),
                command: isola::UP.into(),
                cwd: wt.path,
                env: Vec::new(),
            });
        }
        testlaunch::run_spec(&meta, kind, index)
    }

    /// The services isola runs for the agent's worktree (`isola ls --json`), when it may launch
    /// them now.
    pub async fn isola_services(&self, agent_id: &str) -> Result<Vec<isola::Service>> {
        let wt = self.isola_launch(agent_id)?;
        isola::services(&wt.path, &wt.branch).await
    }

    /// Stops the services isola runs for the agent's worktree (`isola down`).
    pub async fn isola_down(&self, agent_id: &str) -> Result<()> {
        let meta = self.agent(agent_id)?.lock().meta.clone();
        let wt = isola_worktree(&meta)?;
        isola::run(&wt.path, &["down"], isola::LIMIT)
            .await
            .map(|_| ())
    }

    /// The agent's worktree, whose services isola runs, when it may launch them now.
    fn isola_launch(&self, agent_id: &str) -> Result<Worktree> {
        let meta = self.agent(agent_id)?.lock().meta.clone();
        if let Some(why) = self.launch_refusal(&meta) {
            bail!(why);
        }
        isola_worktree(&meta)
    }

    /// The terminal `term_id` is one of the agent's test launches, kept to be stopped with them.
    /// One started as the agent was archived or deleted, or as its ticket's validation began
    /// (after its launches were stopped), is stopped at once: the error tells why.
    pub fn track_test_run(&self, agent_id: &str, term_id: &str, name: &str) -> Result<()> {
        self.test_runs
            .lock()
            .entry(agent_id.to_string())
            .or_default()
            .push(term_id.to_string());
        // Read after it is kept: an archive, a deletion or a validation that this does not see
        // stops it when it comes.
        let refused = match self.agent(agent_id) {
            Ok(h) => {
                let meta = h.lock().meta.clone();
                self.launch_refusal(&meta).map(|why| anyhow!(why))
            }
            Err(e) => Some(e),
        };
        if let Some(e) = refused {
            self.stop_test_runs(agent_id);
            return Err(e);
        }
        self.facts.emit(
            crate::facts::Body::LaunchStarted {
                run: name.to_string(),
            },
            self.about_agent(agent_id),
        );
        Ok(())
    }

    /// A test launch of the agent answers at `url` (the window's look at it, `http_ready`).
    pub fn test_run_ready(&self, agent_id: &str, url: &str) {
        if self.test_runs.lock().contains_key(agent_id) {
            self.facts.emit(
                crate::facts::Body::LaunchReady {
                    url: url.to_string(),
                },
                self.about_agent(agent_id),
            );
        }
    }

    /// The terminal `term_id` ended: when it was an agent's test launch, it stopped.
    pub fn test_run_ended(&self, term_id: &str) {
        let agent = {
            let mut runs = self.test_runs.lock();
            let agent = runs
                .iter()
                .find(|(_, ids)| ids.iter().any(|i| i == term_id))
                .map(|(a, _)| a.clone());
            if let Some(a) = &agent {
                if let Some(ids) = runs.get_mut(a) {
                    ids.retain(|i| i != term_id);
                    if ids.is_empty() {
                        runs.remove(a);
                    }
                }
            }
            agent
        };
        if let Some(a) = agent {
            self.facts
                .emit(crate::facts::Body::LaunchStopped {}, self.about_agent(&a));
        }
    }

    /// The agent's test launches stop (its validation, archive or deletion, its project closed),
    /// isola's services of its worktree included.
    pub fn stop_test_runs(&self, agent_id: &str) {
        let ids = self.test_runs.lock().remove(agent_id).unwrap_or_default();
        for id in ids {
            self.pty.kill(&id);
            self.facts.emit(
                crate::facts::Body::LaunchStopped {},
                self.about_agent(agent_id),
            );
        }
        let dir = self
            .agent(agent_id)
            .ok()
            .and_then(|h| h.lock().meta.worktree.as_ref().map(|w| w.path.clone()));
        if let Some(dir) = dir.filter(|d| isola::manages(d)) {
            isola::down_later(dir);
        }
    }
}

/// The agent's worktree, when isola runs its services.
fn isola_worktree(meta: &AgentMeta) -> Result<Worktree> {
    let wt = meta
        .worktree
        .clone()
        .ok_or_else(|| anyhow!(Refusal::NoWorktree))?;
    if !isola::manages(&wt.path) {
        bail!(tr!(
            "isola ne lance pas ce worktree (isola introuvable, ou pas de .isola.toml).",
            "isola doesn’t run this worktree (isola not found, or no .isola.toml)."
        ));
    }
    Ok(wt)
}

/// How long `gh pr create` may take.
const GH_LIMIT: Duration = Duration::from_secs(120);

/// A pull request to open on GitHub.
struct PullRequest<'a> {
    /// `owner/repo`, the repository the branch was pushed to.
    repo: &'a str,
    base: &'a str,
    head: &'a str,
    title: &'a str,
    body: &'a str,
    draft: bool,
}

/// `gh pr create` in `cwd`, on the repository the branch was pushed to (whatever `gh repo
/// set-default` says), its description through the standard input (an argument with line breaks
/// cannot go through a `.cmd`): what it printed.
async fn gh_pr_create(gh: &Path, cwd: &str, pr: &PullRequest<'_>) -> Result<String> {
    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new(gh);
    cmd.args([
        "pr",
        "create",
        "--repo",
        pr.repo,
        "--base",
        pr.base,
        "--head",
        pr.head,
        "--title",
        pr.title,
        "--body-file",
        "-",
    ]);
    if pr.draft {
        cmd.arg("--draft");
    }
    cmd.current_dir(cwd)
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    crate::job::isolate(&mut cmd);
    let mut child = cmd.spawn()?;
    // Dropped when this returns: whatever it left running (a `.cmd`'s node…) ends with it.
    let _job = crate::job::Job::for_child(&child);
    let created = async move {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(pr.body.as_bytes()).await?;
            // Dropped here: the description ends.
        }
        anyhow::Ok(child.wait_with_output().await?)
    };
    let out = tokio::time::timeout(GH_LIMIT, created)
        .await
        .map_err(|_| {
            anyhow!(tr!(
                "gh pr create toujours pas terminé après {s} s",
                "gh pr create still not done after {s} s",
                s = GH_LIMIT.as_secs()
            ))
        })??;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Of the files copied from the project, those the branch checked out in `cwd` has not
/// committed: left out of the validation's commit, staged or not (unstaging one it has would
/// commit its deletion; the branch's check refuses those).
pub(crate) async fn not_committed(cwd: &str, copied: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for f in copied {
        if git::run(cwd, &["cat-file", "-e", &format!("HEAD:{f}")])
            .await
            .is_err()
        {
            out.push(f.clone());
        }
    }
    out
}

/// The key of a repository's merge lock: its folder however it is spelled, as `same_dir` sees it
/// (the real path, else the path written with `/` and in lower case).
pub(crate) fn merge_lock_key(repo: &str) -> String {
    match std::fs::canonicalize(repo) {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(_) => repo.replace('\\', "/").trim_end_matches('/').to_lowercase(),
    }
}

/// `t` is still "À tester" with this agent, its validation under way.
pub(crate) fn validating(t: &Ticket, agent_id: &str) -> bool {
    t.column == Column::Review && t.step.is_some() && t.agent_id.as_deref() == Some(agent_id)
}

/// Where the validations of the tickets of `project` (in the repository whose root is `repo`) make
/// their temporary worktree: `.claude/worktrees` at the repository's root, or the project's own
/// « Dossier des worktrees » when it has one (a repository that wants no worktree in it).
fn merges_root(project: &Project, repo: &str) -> PathBuf {
    if project.worktree_dir.trim().is_empty() {
        crate::paths::worktrees_root(repo, "")
    } else {
        project.worktrees_root()
    }
}

/// `path`, a worktree of the repository, is the temporary one of a validation (`.merge-…`), wherever
/// it was made: the « Dossier des worktrees » of the project then, or `.claude/worktrees` at the
/// repository's root before it had one.
fn merge_leftover(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with(".merge-"))
}

/// Removes the worktree at `path`, registered or not, its folder there or not.
async fn drop_worktree(repo: &str, path: &Path) {
    let p = path.to_string_lossy();
    let _ = git::run(repo, &["worktree", "remove", "--force", &p]).await;
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
    let _ = git::run(repo, &["worktree", "prune"]).await;
}

/// The same folder, however git and Windows spell it (short 8.3 names, slashes, case).
pub(crate) fn same_dir(a: &str, b: &str) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => {
            let norm = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_lowercase();
            norm(a) == norm(b)
        }
    }
}

/// The threshold of a window that holds back a start: « Pause au-delà du quota » for the
/// autopilot, a window used up (100 %) for a start asked by hand (`Core::project_limit`).
fn hold_threshold(saved: u32, by_hand: bool) -> u32 {
    if by_hand {
        100
    } else {
        board::quota_threshold(saved)
    }
}

/// The accounts' usage a hold reads: for a start asked by hand, without their day's budget, which
/// pauses the autopilot alone (`Core::project_limit`).
fn hold_usage(usage: &[AccountUsage], by_hand: bool) -> Vec<AccountUsage> {
    usage
        .iter()
        .cloned()
        .map(|mut u| {
            if by_hand {
                u.daily_budget = None;
            }
            u
        })
        .collect()
}
