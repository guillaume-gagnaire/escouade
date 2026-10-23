//! A project closed keeps its history. Its settings, its agents' metadata and its tickets go into
//! one file of `closed-projects/`; its conversations and the logs of its worktrees' setups stay
//! where they are, and so do its place in the accounts' « Réservé aux projets » lists. Adding a
//! project in the same folder gives it all back, with its id, so with its statistics.
//!
//! What the app needs to know of the files (the list, the folder a history is of, the ports its
//! agents hold, the tickets it has) is kept in memory (`ClosedEntry`), read once when the app
//! starts and kept up to date by what the app writes and erases: asking never reads the folder.

use crate::agent::{AgentHandle, AgentRt};
use crate::conv;
use crate::core::{approval_interrupted, Core};
use crate::i18n::Lang;
use crate::model::*;
use crate::paths::{self, DataDir};
use crate::ticketimages;
use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tauri::Runtime;

/// What a project closed leaves in `closed-projects/<id>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedProject {
    pub project: Project,
    /// Every agent it had, archived ones too.
    #[serde(default)]
    pub agents: Vec<AgentMeta>,
    #[serde(default)]
    pub tickets: Vec<Ticket>,
    /// The agent that was selected in its sidebar.
    #[serde(default)]
    pub selected_agent: Option<String>,
    /// When it was closed (ms).
    #[serde(default)]
    pub closed_at: i64,
}

/// A closed project as the window lists it, and as the window that adds a project is told it found.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClosedProjectInfo {
    pub id: String,
    pub name: String,
    pub color: String,
    pub path: String,
    pub worktree_per_agent: bool,
    /// Its agents that are not archived.
    pub agents: usize,
    pub tickets: usize,
    /// The latest of its agents' and tickets' dates (ms); when it was closed, if it has none.
    pub last_activity: i64,
    pub closed_at: i64,
    /// What it takes on the disk (bytes): its file, its conversations, its setups' logs.
    pub size: u64,
    /// Its file cannot be read (or is not named after its project): all but the id, the size and
    /// the dates are empty, and the file is all « Effacer » removes.
    pub unreadable: bool,
}

/// What the app knows of a history on the disk without looking at it again (`Core::closed_index`).
#[derive(Debug, Clone)]
pub(crate) struct ClosedEntry {
    pub id: String,
    /// `paths::folder_key` of its path; empty for a file that cannot be read.
    pub key: String,
    pub info: ClosedProjectInfo,
    /// The blocks of ports its agents that are not archived hold.
    pub ports: Vec<u16>,
    /// The ids of its tickets.
    pub tickets: HashSet<String>,
}

impl ClosedEntry {
    /// The entry of a history just written or read from its file.
    fn of(closed: &ClosedProject, data: &DataDir) -> Self {
        let conversations = data.conversations();
        // The logs of its agents, and the copies of the files joined to their messages (they stay
        // with the history: closing a project does not remove them).
        let logs: u64 = closed
            .agents
            .iter()
            .filter(|a| paths::is_plain_id(&a.id))
            .map(|a| {
                size_of(&conv::log_path(&conversations, &a.id))
                    + size_of(&data.setup_log(&a.id))
                    + size_of_dir(&data.agent_files(&a.id))
            })
            .sum();
        // The images and the files of its tickets, kept in their folders with it.
        let images: u64 = closed
            .tickets
            .iter()
            .filter(|t| paths::is_plain_id(&t.id))
            .map(|t| size_of_dir(&data.ticket_dir(&t.id)))
            .sum();
        let id = closed.project.id.clone();
        let info = ClosedProjectInfo {
            id: id.clone(),
            name: closed.project.name.clone(),
            color: closed.project.color.clone(),
            path: closed.project.path.clone(),
            worktree_per_agent: closed.project.worktree_per_agent,
            agents: closed.agents.iter().filter(|a| !a.archived).count(),
            tickets: closed.tickets.len(),
            last_activity: closed.last_activity(),
            closed_at: closed.closed_at,
            size: size_of(&data.closed_project_file(&id)) + logs + images,
            unreadable: false,
        };
        Self {
            key: paths::folder_key(&closed.project.path),
            ports: closed
                .agents
                .iter()
                .filter(|a| !a.archived)
                .filter_map(|a| a.port_base)
                .collect(),
            tickets: closed.tickets.iter().map(|t| t.id.clone()).collect(),
            id,
            info,
        }
    }

    /// The entry of a file that cannot be read, named `id` (its name without `.json`): it has its
    /// size and the date it was last changed, nothing else.
    fn unreadable(id: &str, file: &Path) -> Self {
        let meta = std::fs::metadata(file).ok();
        let size = meta.as_ref().map_or(0, |m| m.len());
        let at = meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as i64);
        Self {
            id: id.to_string(),
            key: String::new(),
            info: ClosedProjectInfo {
                id: id.to_string(),
                name: String::new(),
                color: String::new(),
                path: String::new(),
                worktree_per_agent: false,
                agents: 0,
                tickets: 0,
                last_activity: at,
                closed_at: at,
                size,
                unreadable: true,
            },
            ports: Vec::new(),
            tickets: HashSet::new(),
        }
    }
}

/// What the folder of the closed projects holds, read once (`scan`).
#[derive(Default)]
pub(crate) struct Scan {
    /// One for each file that counts, readable or not.
    pub entries: Vec<ClosedEntry>,
    /// The histories of those that can be read.
    pub histories: Vec<ClosedProject>,
}

/// The block of a ticket « En cours » when its project is closed.
fn project_closed_during(lang: Lang) -> String {
    tr_in!(
        lang,
        "Projet fermé pendant le ticket",
        "Project closed during the ticket"
    )
}

/// A ticket as its closed project keeps it. A validation under way is cut and a ticket at work
/// is held, both to be taken up by hand (« Reprendre »), as after the app's own stop.
pub(crate) fn closed_ticket(mut t: Ticket, lang: Lang) -> Ticket {
    if t.step.take().is_some() {
        t.blocked = Some(approval_interrupted(lang));
        t.conflict = false;
    } else if t.column == Column::Doing && t.blocked.is_none() {
        t.blocked = Some(project_closed_during(lang));
    }
    t
}

/// Refused: the history cannot be written before the project closes, nowhere to keep it.
pub(crate) fn history_refused(e: &anyhow::Error) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "L'historique du projet n'a pas pu être écrit ({e:#}) : le projet reste ouvert.",
        "The project’s history couldn’t be written ({e:#}): the project stays open."
    ))
}

/// The toast of a history that could not be written: the project is closed all the same.
fn history_not_kept(lang: Lang, name: &str, e: &anyhow::Error) -> String {
    tr_in!(
        lang,
        "L'historique du projet « {name} » n'a pas pu être gardé : {e:#}",
        "The history of the project “{name}” couldn’t be kept: {e:#}"
    )
}

/// Refused: a validation of the ticket `key` runs, closing the project would cut it half way.
pub(crate) fn approval_under_way(lang: Lang, key: &str) -> anyhow::Error {
    anyhow::anyhow!(tr_in!(
        lang,
        "La validation du ticket {key} est en cours : attends sa fin pour fermer le projet.",
        "The ticket {key} is being approved: wait for the end of its approval to close the project."
    ))
}

/// Refused: no closed project has this id (erased or reopened meanwhile).
fn closed_not_found() -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "Ce projet fermé n'existe plus.",
        "This closed project no longer exists."
    ))
}

fn size_of(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// What the files in the folder `dir` take, those of the folders below it too (a ticket's images
/// are right in its folder, its other files in `files/<uniq>/`); none if it is not there. A link is
/// not followed.
fn size_of_dir(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| match e.file_type() {
                    Ok(t) if t.is_dir() => size_of_dir(&e.path()),
                    Ok(t) if t.is_file() => size_of(&e.path()),
                    _ => 0,
                })
                .sum()
        })
        .unwrap_or(0)
}

/// `path` removed; one that is not there is no matter. False when it is still there.
fn remove_if_there(path: &Path) -> bool {
    match std::fs::remove_file(path) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => {
            log::warn!("cannot remove {}: {e}", path.display());
            false
        }
    }
}

/// The histories on the disk, whether their projects are open or not: those that can be read and
/// are named after their own project make an entry and a history; any other `.json` file named
/// with a plain id (letters, digits, `-` and `_`) makes an unreadable entry; the rest is left
/// alone. The ids in `hidden` are not looked at (`Core::leftover_histories`).
pub(crate) fn scan(data: &DataDir, hidden: &[String]) -> Scan {
    let mut found = Scan::default();
    let Ok(dir) = std::fs::read_dir(data.closed_projects()) else {
        return found;
    };
    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|n| n.to_str()) else {
            continue;
        };
        if hidden.iter().any(|id| id == name) {
            continue;
        }
        let read = std::fs::read_to_string(&path)
            .map_err(anyhow::Error::from)
            .and_then(|text| Ok(serde_json::from_str::<ClosedProject>(&text)?));
        match read {
            Ok(closed) if data.closed_project_file(&closed.project.id) == path => {
                found.entries.push(ClosedEntry::of(&closed, data));
                found.histories.push(closed);
            }
            other => {
                match other {
                    Ok(_) => log::warn!(
                        "closed project {} is not named after its id",
                        path.display()
                    ),
                    Err(e) => log::warn!("closed project {} cannot be read: {e:#}", path.display()),
                }
                if paths::is_plain_id(name) {
                    found.entries.push(ClosedEntry::unreadable(name, &path));
                }
            }
        }
    }
    found
}

/// The history of the closed project `id`, read from its file; none if it is not there, cannot be
/// read or is not its project's.
pub(crate) fn read_history(data: &DataDir, id: &str) -> Option<ClosedProject> {
    let file = data.closed_project_file(id);
    let text = std::fs::read_to_string(&file).ok()?;
    match serde_json::from_str::<ClosedProject>(&text) {
        Ok(closed) => (closed.project.id == id).then_some(closed),
        Err(e) => {
            log::warn!("closed project {} cannot be read: {e}", file.display());
            None
        }
    }
}

/// The histories whose file could not be removed once the state held the truth
/// (`PersistedState::leftover_histories`): tried again now. The ids whose file is still there stay
/// in `ids`.
pub(crate) fn retry_leftovers(data: &DataDir, ids: &mut Vec<String>) {
    ids.retain(|id| paths::is_plain_id(id) && !remove_if_there(&data.closed_project_file(id)));
}

/// What the start of the app finds of a closing a stop cut. A save in the middle of it (the
/// periodic one, the one of the quit) can have the project with some of its agents and tickets
/// only, while its file, written before anything stopped, has them all: `agents` and `tickets` get
/// those they lack, for the projects `projects` that are open. The ids of the files to forget once
/// the state is saved with them (`Core::settle_recovered`) are returned.
pub(crate) fn recover_open(
    histories: Vec<ClosedProject>,
    projects: &[Project],
    agents: &mut Vec<AgentMeta>,
    tickets: &mut Vec<Ticket>,
) -> Vec<String> {
    let mut ids = Vec::new();
    let mut have_agents: HashSet<String> = agents.iter().map(|a| a.id.clone()).collect();
    let mut have_tickets: HashSet<String> = tickets.iter().map(|t| t.id.clone()).collect();
    for closed in histories {
        let id = closed.project.id;
        if !projects.iter().any(|p| p.id == id) {
            continue;
        }
        for m in closed.agents {
            if m.project_id == id && paths::is_plain_id(&m.id) && have_agents.insert(m.id.clone()) {
                agents.push(m);
            }
        }
        for t in closed.tickets {
            if t.project_id == id && have_tickets.insert(t.id.clone()) {
                tickets.push(t);
            }
        }
        ids.push(id);
    }
    ids
}

/// A project being closed, until this is dropped (`Core::closing`).
pub(crate) struct Closing<'a> {
    set: &'a Mutex<HashSet<String>>,
    id: String,
}

impl<'a> Closing<'a> {
    /// None when the project is being closed already.
    pub(crate) fn start(set: &'a Mutex<HashSet<String>>, id: &str) -> Option<Self> {
        set.lock().insert(id.to_string()).then(|| Self {
            set,
            id: id.to_string(),
        })
    }
}

impl Drop for Closing<'_> {
    fn drop(&mut self) {
        self.set.lock().remove(&self.id);
    }
}

/// How `Core::closed_entry_for` stands after the memory alone was asked.
enum Search {
    /// The history of the folder.
    Found(Box<ClosedEntry>),
    /// No history can be of any folder: nothing to look for.
    Nothing,
    /// None by the folder's key: another spelling of it (a link, a short name) is for the disk to tell.
    Ask,
}

impl ClosedProject {
    /// The project as it comes back in its folder, written `path` this time: the history's, with
    /// the name, colour and worktree choice the window sent (the name of the history if none, the
    /// colour too), and its preferred account only if it is one of `accounts`. Its path stays the
    /// one its agents' folders and worktrees were made from: the same folder, maybe spelt otherwise.
    pub fn reopened(
        &self,
        path: &str,
        name: &str,
        color: &str,
        worktree_per_agent: bool,
        accounts: &[String],
    ) -> Project {
        let mut project = self.project.clone();
        // Closed before ESC-160: its « Les agents partent de la branche cible » kept.
        project.settle_legacy();
        let name = name.trim();
        if !name.is_empty() {
            project.name = name.to_string();
        } else if project.name.trim().is_empty() {
            project.name = Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| tr!("projet", "project"));
        }
        if !color.is_empty() {
            project.color = color.to_string();
        }
        project.worktree_per_agent = worktree_per_agent;
        if !accounts.contains(&project.account) {
            project.account.clear();
        }
        project
    }

    /// The latest date of its agents and tickets (ms), else when it was closed.
    fn last_activity(&self) -> i64 {
        let agents = self
            .agents
            .iter()
            .flat_map(|a| [a.last_activity, a.created_at]);
        let tickets = self.tickets.iter().flat_map(|t| {
            [Some(t.created_at), t.started_at, t.review_at, t.done_at]
                .into_iter()
                .flatten()
        });
        agents
            .chain(tickets)
            .max()
            .filter(|at| *at > 0)
            .unwrap_or(self.closed_at)
    }
}

/// The index of the histories on the disk, by project id (`Core::closed_index`).
pub(crate) type ClosedIndex = Mutex<HashMap<String, ClosedEntry>>;

impl<R: Runtime> Core<R> {
    /// The history of the project being closed, written to its file: once before anything stops
    /// (an error then keeps the project open), and again once everything stopped. No turn
    /// survives: an agent at work comes back done, as after a restart.
    pub(crate) fn write_history(
        &self,
        project: Project,
        mut agents: Vec<AgentMeta>,
        tickets: Vec<Ticket>,
        selected_agent: Option<String>,
    ) -> Result<()> {
        let lang = self.lang().ui;
        for m in &mut agents {
            if m.status.is_active() {
                m.status = AgentStatus::Done;
            }
        }
        agents.sort_by_key(|m| m.created_at);
        let closed = ClosedProject {
            tickets: tickets
                .into_iter()
                .map(|t| closed_ticket(t, lang))
                .collect(),
            project,
            agents,
            selected_agent,
            closed_at: now_ms(),
        };
        self.write_closed(&closed)
    }

    /// `write_history` once the project stopped. The project is closed all the same when that
    /// fails (its first writing is there): told, never an error that would have the window ask again.
    pub(crate) fn keep_history(
        &self,
        project: Project,
        agents: Vec<AgentMeta>,
        tickets: Vec<Ticket>,
        selected_agent: Option<String>,
    ) {
        let (id, name) = (project.id.clone(), project.name.clone());
        if let Err(e) = self.write_history(project, agents, tickets, selected_agent) {
            log::error!("history of project {id} not kept: {e:#}");
            self.hub.emit(UiEvent::Toast {
                text: history_not_kept(self.lang().ui, &name, &e),
            });
        }
    }

    /// The history written to its file and put in the index.
    fn write_closed(&self, closed: &ClosedProject) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(closed)?;
        std::fs::create_dir_all(self.data.closed_projects())?;
        paths::write_atomic(&self.data.closed_project_file(&closed.project.id), &bytes)
            .context("closed project file")?;
        let entry = ClosedEntry::of(closed, &self.data);
        self.closed_index.lock().insert(entry.id.clone(), entry);
        // The file is the project's new history, not the leftover of an old one: the next start
        // reads it (and a stop in the middle of this closing finds it).
        let id = &closed.project.id;
        let was_left = {
            let mut left = self.leftover_histories.lock();
            let before = left.len();
            left.retain(|x| x != id);
            left.len() != before
        };
        if was_left {
            self.request_save();
        }
        Ok(())
    }

    /// Every history on the disk whose project is not open, the latest closed first. A file that
    /// is not named after its project is left out (an unreadable one is there, as such), and so is
    /// one of a project that is open (the app stopped between the file and the state, or a
    /// closing is under way): its history is the open project's own.
    pub(crate) fn closed_entries(&self) -> Vec<ClosedEntry> {
        let open: HashSet<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let mut list: Vec<ClosedEntry> = self
            .closed_index
            .lock()
            .values()
            .filter(|e| !open.contains(&e.id))
            .cloned()
            .collect();
        list.sort_by(|a, b| {
            b.info
                .closed_at
                .cmp(&a.info.closed_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        list
    }

    /// The closed projects, the latest closed first (« Projets fermés »).
    pub fn closed_projects(&self) -> Vec<ClosedProjectInfo> {
        self.closed_entries().into_iter().map(|e| e.info).collect()
    }

    /// Whether the project is being closed (`remove_project`).
    pub(crate) fn is_closing(&self, project_id: &str) -> bool {
        self.closing.lock().contains(project_id)
    }

    /// The blocks of ports the agents of the closed projects hold, and those of a project being
    /// closed: its agents leave the app one by one, its first history is written before.
    pub(crate) fn closed_ports(&self) -> Vec<u16> {
        let open: HashSet<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let closing = self.closing.lock().clone();
        self.closed_index
            .lock()
            .values()
            .filter(|e| !open.contains(&e.id) || closing.contains(&e.id))
            .flat_map(|e| e.ports.iter().copied())
            .collect()
    }

    /// Whether the ticket is in the history of a closed project: it is not on the board, but it
    /// comes back with its project.
    pub(crate) fn in_closed_history(&self, ticket_id: &str) -> bool {
        let owners: Vec<String> = self
            .closed_index
            .lock()
            .values()
            .filter(|e| e.tickets.contains(ticket_id))
            .map(|e| e.id.clone())
            .collect();
        let projects = self.projects.read();
        owners
            .iter()
            .any(|id| !projects.iter().any(|p| p.id == *id))
    }

    /// The history of the folder `path` by the memory alone.
    fn search_closed(&self, path: &str, history_id: Option<&str>) -> Search {
        let list: Vec<ClosedEntry> = self
            .closed_entries()
            .into_iter()
            .filter(|e| !e.info.unreadable)
            .collect();
        if list.is_empty() {
            return Search::Nothing;
        }
        let key = paths::folder_key(path);
        let named = history_id.and_then(|id| list.iter().find(|e| e.id == id));
        match named {
            Some(e) if e.key == key => Search::Found(Box::new(e.clone())),
            // Another spelling of the folder, maybe: only the disk tells.
            Some(_) => Search::Ask,
            None => match list.iter().find(|e| e.key == key) {
                Some(e) => Search::Found(Box::new(e.clone())),
                None => Search::Ask,
            },
        }
    }

    /// `closed_entry_for` where the disk is asked: the history `history_id` if it is of this
    /// folder in any spelling, else the latest closed in it.
    fn search_closed_on_disk(&self, path: &str, history_id: Option<&str>) -> Option<ClosedEntry> {
        let list: Vec<ClosedEntry> = self
            .closed_entries()
            .into_iter()
            .filter(|e| !e.info.unreadable)
            .collect();
        let named = history_id
            .and_then(|id| list.iter().find(|e| e.id == id))
            .filter(|e| paths::same_folder(&e.info.path, path));
        named
            .or_else(|| paths::find_folder(&list, path, |e| &e.info.path))
            .cloned()
    }

    /// The history of the folder `path`, if a project was closed in it: the one `history_id`
    /// names if that is one of this folder, else the latest closed. Told by the memory when it
    /// can; the disk, asked off the runtime's threads, only to tell the folder's other spellings.
    pub(crate) async fn closed_entry_for(
        self: &Arc<Self>,
        path: &str,
        history_id: Option<&str>,
    ) -> Option<ClosedEntry> {
        match self.search_closed(path, history_id) {
            Search::Found(entry) => Some(*entry),
            Search::Nothing => None,
            Search::Ask => {
                let (core, path) = (self.clone(), path.to_string());
                let history_id = history_id.map(str::to_string);
                tokio::task::spawn_blocking(move || {
                    core.search_closed_on_disk(&path, history_id.as_deref())
                })
                .await
                .ok()
                .flatten()
            }
        }
    }

    /// `closed_entry_for`, as the window that adds a project is told of it.
    pub async fn closed_project_info_for(
        self: &Arc<Self>,
        path: &str,
        history_id: Option<&str>,
    ) -> Option<ClosedProjectInfo> {
        self.closed_entry_for(path, history_id)
            .await
            .map(|entry| entry.info)
    }

    /// The history of a closed project gone for good: its conversations, its setups' logs, the
    /// files joined to its agents' messages and its file, and its place in the accounts' « Réservé
    /// aux projets ». Its worktrees and its statistics stay, as when a project is closed. A file
    /// that cannot be read goes alone.
    pub fn erase_closed_project(self: &Arc<Self>, id: &str) -> Result<()> {
        let entry = self
            .closed_entries()
            .into_iter()
            .find(|e| e.id == id)
            .ok_or_else(closed_not_found)?;
        let file = self.data.closed_project_file(id);
        let history = if entry.info.unreadable {
            None
        } else {
            read_history(&self.data, id)
        };
        let Some(closed) = history else {
            // Nothing to know of its agents and tickets: its file is all there is to erase.
            return match std::fs::remove_file(&file) {
                Ok(()) => {
                    self.closed_index.lock().remove(id);
                    Ok(())
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    self.closed_index.lock().remove(id);
                    Err(closed_not_found())
                }
                Err(e) => Err(e.into()),
            };
        };
        // An agent of the app is no one's history (an id the state and the file share).
        let agents: Vec<String> = {
            let live = self.agents.read();
            closed
                .agents
                .iter()
                .map(|a| a.id.clone())
                .filter(|a| !live.contains_key(a))
                .collect()
        };
        let tickets: Vec<String> = closed.tickets.iter().map(|t| t.id.clone()).collect();
        self.erase_history_files(&agents, &tickets);
        // Last: a file still there lists the project, for another try.
        std::fs::remove_file(&file)?;
        self.closed_index.lock().remove(id);
        // Its place in the accounts' « Réservé aux projets » goes with the history.
        self.forget_project_in_accounts(id);
        Ok(())
    }

    /// The files of a history that goes for good, by the agents and tickets it had, and the
    /// operations its tickets still have to tell their external ones. The only way they go:
    /// closing a project that keeps its history never comes here.
    pub(crate) fn erase_history_files(&self, agent_ids: &[String], ticket_ids: &[String]) {
        let conversations = self.data.conversations();
        for id in agent_ids.iter().filter(|id| paths::is_plain_id(id)) {
            remove_if_there(&conv::log_path(&conversations, id));
            remove_if_there(&self.data.setup_log(id));
            // The files joined to its messages (copies only: a file of the user's own stays).
            crate::attachfiles::remove_agent_files(&self.data, id);
        }
        // The images and the files of its tickets (their folder holds both). A doubtful id is left
        // out: `ticket_dir` puts every such ticket in one shared folder (`tickets/_`), no one
        // ticket's own.
        for id in ticket_ids.iter().filter(|id| paths::is_plain_id(id)) {
            ticketimages::remove(&self.data.ticket_dir(id));
        }
        self.forget_ticket_syncs(ticket_ids);
    }

    /// What a project closed kept, given back to the project `project` that is now in its place
    /// (same id): its agents, without a process as after a restart, its tickets, the agent
    /// selected; the window is told of each agent and ticket. The accounts it is reserved to
    /// still are (closing it kept it in their lists).
    /// The agents to start again, as the app's start does, are returned (Remote Control ones).
    pub(crate) fn give_back(
        self: &Arc<Self>,
        closed: ClosedProject,
        project: &Project,
    ) -> Vec<String> {
        let conversations = self.data.conversations();
        let mut handles: Vec<AgentHandle> = Vec::new();
        {
            let mut agents = self.agents.write();
            for m in closed.agents {
                // An id that is no name of its own (a file edited by hand) never gets a log.
                if m.project_id != project.id
                    || !paths::is_plain_id(&m.id)
                    || agents.contains_key(&m.id)
                {
                    continue;
                }
                let id = m.id.clone();
                let h = Arc::new(Mutex::new(AgentRt::new(m, &conversations)));
                agents.insert(id, h.clone());
                handles.push(h);
            }
        }
        let tickets: Vec<Ticket> = {
            let mut board = self.tickets.write();
            let had: HashSet<String> = board.iter().map(|t| t.id.clone()).collect();
            let back: Vec<Ticket> = closed
                .tickets
                .into_iter()
                .filter(|t| t.project_id == project.id && !had.contains(&t.id))
                .collect();
            board.extend(back.iter().cloned());
            back
        };
        let selected = closed
            .selected_agent
            .filter(|a| handles.iter().any(|h| h.lock().meta.id == *a));
        if let Some(agent) = selected {
            self.ui
                .write()
                .selected_agent
                .insert(project.id.clone(), agent);
        }
        for h in &handles {
            self.emit_agent(h);
        }
        for ticket in tickets {
            self.hub.emit(UiEvent::Ticket { ticket });
        }
        // The Remote Control settings are applied to them as when the app starts (`start_remote`):
        // the project's own setting and the agents set apart come back with them.
        handles
            .iter()
            .filter_map(|h| {
                let rt = h.lock();
                (!rt.meta.archived).then(|| rt.meta.id.clone())
            })
            .collect()
    }

    /// The file of the closed project `id`, once what it kept is in the state (saved): the
    /// history is the open project's now. False when it could not be removed.
    pub(crate) fn forget_closed_file(&self, id: &str) -> bool {
        self.closed_index.lock().remove(id);
        remove_if_there(&self.data.closed_project_file(id))
    }

    /// The history `id` is given back (or erased) and its file is to go: the state remembers it
    /// until it is gone (`settle_history_file`). Called before the state that holds the truth is
    /// saved, so that a start never reads a file the app could not remove.
    pub(crate) fn note_leftover(&self, id: &str) {
        let mut left = self.leftover_histories.lock();
        if !left.iter().any(|x| x == id) {
            left.push(id.to_string());
        }
    }

    /// The file of the history `id` removed, once the state is saved with what it gave back. When
    /// it cannot be (a file an antivirus holds), the id stays noted: the next start tries again
    /// rather than read it, which would bring back what was deleted since.
    pub(crate) fn settle_history_file(&self, id: &str) {
        if !self.forget_closed_file(id) {
            return;
        }
        let changed = {
            let mut left = self.leftover_histories.lock();
            let before = left.len();
            left.retain(|x| x != id);
            left.len() != before
        };
        if changed {
            self.request_save();
        }
    }

    /// The state saved with what `recover_open` gave it, then the files it came from go: only if
    /// it is saved, else they stay for the next start.
    pub(crate) fn settle_recovered(&self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }
        ids.iter().for_each(|id| self.note_leftover(id));
        match self.save_state() {
            Ok(()) => ids.iter().for_each(|id| self.settle_history_file(id)),
            Err(e) => log::error!("histories of a closing a stop cut not forgotten: {e:#}"),
        }
    }
}
