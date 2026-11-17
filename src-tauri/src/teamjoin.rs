//! « Rattacher à l'équipe » (TS-7, `docs/TEAM-SERVER.md` §2.7): a project of this device made a project of its team,
//! its Kanban joined without an id to copy, and the key `team` of its `.escouade.yml` written.
//!
//! - **The Kanban of the list**: an entry of the team's list names the journal of its Kanban (`Listed::board`). A
//!   project attached here, or cloned from the list, joins it. A project of the team whose entry gains a Kanban later
//!   joins it at the next reading of the team ([`Core::team_join_listed_soon`]). Its tickets of before stay on this
//!   machine (« Sur ce poste seulement ») until « Verser à l'équipe » (`teampour`).
//! - **The list learns a Kanban** when it is shared: written in the entry by an administrator, proposed to one by a
//!   member (`escouade_team::proposals`), whose screen offers « Inscrire le Kanban ».
//! - **The key `team`** of `.escouade.yml` (the server's address, the team's id, the project's id in the list and the
//!   team's name: nothing secret) is written at the attachment; the user commits it. A device that opens the repository
//!   reads it (`projectfile::team_hint`): one of the team attaches the project by its member's gesture, any other sees
//!   the band of the invitation. Nothing goes to the address of the file before a gesture (the band only opens the
//!   screen of the team's entry, the address filled in).

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use escouade_team::catalog::{self, Catalog, Item};

use crate::core::Core;
use crate::model::{Project, UiEvent};
use crate::projectfile::{write_team, TeamKey, FILE};

/// The item of the team's list one of whose repositories is a repository of `project` (by the address of its remotes).
pub async fn item_holding(catalog: &Catalog, project: &Project) -> Option<Item> {
    let local = crate::team_projects::locals(vec![project.clone()])
        .await
        .pop()?;
    catalog
        .projects
        .iter()
        .find(|i| {
            i.project
                .repos
                .iter()
                .any(|r| catalog::key(&r.url).is_some_and(|k| local.keys.contains(&k)))
        })
        .cloned()
}

/// The last time each project of the team without a Kanban was matched with the list (they cost a `git remote` each).
static LISTED_AT: parking_lot::Mutex<Option<Instant>> = parking_lot::Mutex::new(None);

impl Core {
    /// Writes the key `team` of the project's `.escouade.yml` (`Some`), or takes it out (`None`): the file read again.
    /// False when nothing changed.
    pub(crate) fn write_team_key(
        self: &Arc<Self>,
        project_id: &str,
        key: Option<&TeamKey>,
    ) -> Result<bool> {
        let p = self.project(project_id)?;
        let path = std::path::Path::new(&p.path).join(FILE);
        // A file that is a link (a repository's): never written through, it could point anywhere.
        if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
            anyhow::bail!(tr!(
                "{FILE} n’est pas un fichier ordinaire (un lien ?) : la clé « team » n’y est pas écrite.",
                "{FILE} is not a plain file (a link?): the key “team” is not written in it."
            ));
        }
        let existing = std::fs::read_to_string(&path).ok();
        if key.is_none() && existing.is_none() {
            return Ok(false);
        }
        let text = write_team(existing.as_deref(), key)?;
        if existing.as_deref() == Some(text.as_str()) {
            return Ok(false);
        }
        std::fs::write(&path, text)?;
        self.read_shared(project_id, false);
        Ok(true)
    }

    /// « Rattacher à l'équipe »: the project made a project of the team `team_id` (a team of this device), its Kanban
    /// joined when the team's list names it, the key `team` of its file written. By the member's gesture alone.
    pub(crate) async fn team_attach(
        self: &Arc<Self>,
        project_id: &str,
        team_id: &str,
    ) -> Result<Project> {
        let teams = self.teams_link().ok_or_else(|| anyhow!("no teams"))?;
        let (base, _host, name) = teams.place_of(team_id).ok_or_else(|| {
            anyhow!(tr!(
                "Ce poste n’est pas dans cette équipe : demande une invitation.",
                "This device is not in this team: ask for an invitation."
            ))
        })?;
        self.set_project_team(project_id, Some(team_id.to_string()))?;
        let project = self.project(project_id)?;
        let item = match teams.catalog(team_id).await {
            Ok(of) => item_holding(&of.catalog, &project).await,
            Err(e) => {
                log::info!("team attach: the list not read: {e}");
                None
            }
        };
        let key = TeamKey {
            server: base,
            id: team_id.to_string(),
            project: item.as_ref().map(|i| i.id.clone()),
            name,
        };
        match self.write_team_key(project_id, Some(&key)) {
            Ok(true) => self.hub.emit(UiEvent::Toast {
                text: tr!(
                    "{name} : la clé « team » est écrite dans {FILE}. Commite-la : qui ouvrira le dépôt saura qu’il est de l’équipe.",
                    "{name}: the key “team” is written in {FILE}. Commit it: whoever opens the repository will know it is the team’s.",
                    name = project.name
                ),
            }),
            Ok(false) => {}
            Err(e) => self.hub.emit(UiEvent::Toast {
                text: tr!(
                    "{name} : la clé « team » n’est pas écrite dans {FILE} ({e}).",
                    "{name}: the key “team” is not written in {FILE} ({e}).",
                    name = project.name,
                    e = format!("{e:#}")
                ),
            }),
        }
        if project.team_project.is_none() {
            if let Some(board) = item.as_ref().and_then(|i| i.project.board_id()) {
                self.link_team_board(project_id, Some(hex::encode(board.0)))
                    .await?;
            }
        }
        self.project(project_id)
    }

    /// The project taken out of its team (« projet de l'équipe » unticked): the key `team` of its file goes when it
    /// names that team.
    pub(crate) fn team_detach(self: &Arc<Self>, project_id: &str) -> Result<Project> {
        let before = self.project(project_id)?;
        self.set_project_team(project_id, None)?;
        let named = before
            .shared
            .team
            .as_ref()
            .is_some_and(|h| Some(&h.team) == before.team.as_ref());
        if named {
            if let Err(e) = self.write_team_key(project_id, None) {
                log::warn!("team detach: {FILE} not written: {e:#}");
            }
        }
        self.project(project_id)
    }

    /// « Ne plus proposer pour ce projet »: the band of the team its file names is not shown again, until the file
    /// names another team.
    pub(crate) fn team_hint_dismiss(self: &Arc<Self>, project_id: &str) -> Result<Project> {
        let p = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(crate::core::project_not_found)?;
            p.shared.team_dismissed = p.shared.team.as_ref().map(|h| h.fingerprint.clone());
            p.clone()
        };
        self.hub.emit(UiEvent::Project { project: p.clone() });
        self.request_save();
        Ok(p)
    }

    /// A new journal of a project's Kanban, told to the team's list (TS-7): written in the entry of the project by an
    /// administrator, proposed to one by a member. What the toast says; none when the project is not in the list, or
    /// its entry names a Kanban already.
    pub(crate) async fn team_list_board(
        &self,
        project: &Project,
        team_id: &str,
        board: &str,
    ) -> Option<String> {
        let teams = self.teams_link()?;
        let of = teams.catalog(team_id).await.ok()?;
        let item = item_holding(&of.catalog, project).await?;
        if !item.project.board.is_empty() {
            return None;
        }
        if of.admin {
            let mut listed = item.project.clone();
            listed.board = board.to_string();
            return Some(match teams.catalog_put(team_id, Some(&item.id), listed).await {
                Ok(_) => tr!(
                    "Le Kanban est inscrit dans la liste de l’équipe : un collègue qui clone ou rattache « {name} » le rejoint sans identifiant.",
                    "The Kanban is written in the team’s list: a teammate who clones or attaches “{name}” joins it without an id.",
                    name = item.project.name
                ),
                Err(e) => tr!(
                    "Le Kanban n’est pas inscrit dans la liste de l’équipe : {e}",
                    "The Kanban is not written in the team’s list: {e}",
                    e = e
                ),
            });
        }
        Some(match teams.propose_board(team_id, &item.id, board).await {
            Ok(()) => tr!(
                "Seul un administrateur écrit la liste de l’équipe : le Kanban de « {name} » lui est proposé.",
                "Only an administrator writes the team’s list: the Kanban of “{name}” is proposed to them.",
                name = item.project.name
            ),
            Err(e) => tr!(
                "Le Kanban n’a pas pu être proposé à un administrateur : {e}",
                "The Kanban could not be proposed to an administrator: {e}",
                e = e
            ),
        })
    }

    /// An administrator's « Inscrire le Kanban »: the Kanban a member proposed written in the entry of the list.
    pub(crate) async fn team_accept_board(&self, team_id: &str, item_id: &str) -> Result<()> {
        let teams = self.teams_link().ok_or_else(|| anyhow!("no teams"))?;
        let of = teams.catalog(team_id).await.map_err(|e| anyhow!(e))?;
        let item = of.catalog.get(item_id).cloned().ok_or_else(|| {
            anyhow!(tr!(
                "Ce projet n’est plus dans la liste de l’équipe.",
                "This project is no longer in the team’s list."
            ))
        })?;
        let proposed = of.proposals.of(item_id).cloned().ok_or_else(|| {
            anyhow!(tr!(
                "Aucun Kanban n’est proposé pour ce projet.",
                "No Kanban is proposed for this project."
            ))
        })?;
        let mut listed = item.project;
        listed.board = proposed.board;
        teams
            .catalog_put(team_id, Some(item_id), listed)
            .await
            .map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// After a reading of the teams: a project of a team without a Kanban joins the one its entry of the list names
    /// (read as this device last read the list: nothing asked of the server). At most every 30 s.
    pub(crate) fn team_join_listed_soon(self: &Arc<Self>) {
        let waiting: Vec<Project> = self
            .projects
            .read()
            .iter()
            .filter(|p| p.team.is_some() && p.team_project.is_none())
            .cloned()
            .collect();
        if waiting.is_empty() {
            return;
        }
        {
            let mut at = LISTED_AT.lock();
            if at.is_some_and(|t| t.elapsed() < Duration::from_secs(30)) {
                return;
            }
            *at = Some(Instant::now());
        }
        let Some(teams) = self.teams_link() else {
            return;
        };
        let core = self.clone();
        tauri::async_runtime::spawn(async move {
            for p in waiting {
                let Some(team) = p.team.clone() else { continue };
                let Some(view) = teams.catalog_view(&team) else {
                    continue;
                };
                if view.projects.iter().all(|i| i.project.board.is_empty()) {
                    continue;
                }
                let Some(board) = item_holding(&view, &p)
                    .await
                    .and_then(|i| i.project.board_id())
                else {
                    continue;
                };
                if let Err(e) = core
                    .link_team_board(&p.id, Some(hex::encode(board.0)))
                    .await
                {
                    log::info!("team board of {}: the list's not joined: {e:#}", p.name);
                }
            }
        });
    }
}
