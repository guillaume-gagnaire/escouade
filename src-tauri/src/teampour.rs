//! « Verser à l'équipe » (TS-7, `docs/TEAM-SERVER.md` §2.7, C5-A): the tickets a project had before its Kanban became
//! its team's, poured into the team's journal by the member's gesture, after a preview. Nothing leaves without it.
//!
//! - **The tickets of ESC-190's files** (`.escouade/tickets/*.yml`, read straight from the folder) keep their id: the
//!   object of each in the journal is drawn from it ([`object_of_file`]), the same on every device. Pouring twice, or
//!   from two devices at once, writes the same object: never a second ticket (the journal merges field by field). One
//!   already in the journal (poured by a teammate) is not written again: this device's ticket of that file becomes the
//!   team's at the next reading. One the journal deleted is not brought back.
//! - **The tickets of this machine** (« Sur ce poste seulement ») go, those chosen in the list.
//! - **Their keys**: a ticket poured takes a key of the server's counter (`DEM-7`), but one imported from Jira, GitHub…
//!   keeps its own. The key it had (`DEM-12`, `DEM-12@alice`, `DEM-L3`) stays in `Ticket::former_keys`, shared: it is
//!   found by it still.
//! - **The files** are frozen (`sharing::freeze`): their first line says the project joined a team server, Escouade
//!   writes them no more; the window proposes to remove them, and names them. Nothing is removed nor committed.
//! - **The docs** of `.escouade/` (spec, decisions, documents: TS-9, `teamdocs`) go by the same gesture when the team
//!   has none on its server yet (`docs`), named in the preview; they never leave by themselves at the attachment. Those
//!   of a team that has them already, or chose to keep them in the repository, or of a private project, do not go: the
//!   preview says why. The shared memory (`.escouade/memory`, TS-10, `teammemory`) goes with them, its memories written
//!   by hand by the member who pours; once the team's docs are on its server, it goes alone by this gesture when no
//!   device poured it yet (`memory`: `pending`).

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use escouade_team::crypto::ops::{OpBody, Value};
use escouade_team::crypto::{ObjectId, ProjectId};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::core::Core;
use crate::model::{new_id, now_ms, Column, Criterion, TeamTicket, Ticket, UiEvent};
use crate::sharing::{self, Read};
use crate::teamboard::fields_of;

/// The object, in the team's journal, of the ticket of the file `<id>.yml` (hex): the same on every device, unknown
/// to the server (it is in the encrypted operations alone).
pub fn object_of_file(id: &str) -> String {
    let d = Sha256::digest(format!("escouade/ticket-file/{id}").as_bytes());
    hex::encode(&d[..16])
}

/// A ticket of the preview.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PourItem {
    /// The id of its file (`<id>.yml`), or of this machine's ticket.
    pub id: String,
    pub key: String,
    pub title: String,
    pub column: Column,
    /// Imported from Jira, GitHub…: it keeps its key.
    pub external: bool,
    /// Already in the team's Kanban (poured before, here or by a teammate): not written again.
    pub poured: bool,
    /// Why its file cannot be read: it is not poured.
    pub problem: Option<String>,
}

/// What « Verser à l'équipe » would do, before it does anything.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PourPreview {
    /// The tickets of the files of `.escouade/tickets/`.
    pub files: Vec<PourItem>,
    /// The tickets « Sur ce poste seulement ».
    pub locals: Vec<PourItem>,
    /// The folder of the files, from the project's.
    pub folder: String,
    /// The docs of `.escouade/` and what becomes of them.
    pub docs: PourDocs,
}

/// The docs of the repository's `.escouade/` in the preview (TS-9's docs on the team server).
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PourDocs {
    /// `pending`: they go with « Verser » (the team has none on its server yet); `server`: the team's are on the
    /// server already, this machine reads them from there, nothing of its repository goes; `repo`: the team keeps its
    /// docs in the repository; `private`: a private project's never go; `none`: the repository has none.
    pub state: String,
    /// The pages of the spec, the decisions and the documents that would go (relative to `.escouade/`).
    pub pages: Vec<String>,
    pub decisions: Vec<String>,
    pub documents: Vec<String>,
    /// The memory of `.escouade/memory/` (TS-10): `pending` (it goes with « Verser »), `server` (the team's is there
    /// already), `repo`, `private` or `none`, as the docs; and the names of its memories that would go.
    pub memory: String,
    pub memories: Vec<String>,
}

/// What it did.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Poured {
    /// Tickets of the files written into the journal, and those already there.
    pub imported: usize,
    pub already: usize,
    /// Tickets of this machine that went.
    pub locals: usize,
    /// The files frozen, that may be removed from the repository (by the user).
    pub files: Vec<String>,
    pub folder: String,
    /// The docs' files of `.escouade/` imported to the server, left in the repository (the user removes them).
    pub docs: Vec<String>,
}

/// The docs of the repository's `.escouade/`, by kind (relative to `.escouade/`).
fn doc_files(project: &crate::model::Project) -> (Vec<String>, Vec<String>, Vec<String>) {
    let repo = crate::docsbranch::base_of(project);
    let (mut pages, mut decisions, mut documents) = (Vec::new(), Vec::new(), Vec::new());
    for f in crate::teamdocs::repo_doc_files(&repo) {
        let rel = f
            .strip_prefix(&format!("{}/", crate::docs::ROOT))
            .unwrap_or(&f)
            .to_string();
        match rel.split('/').next() {
            Some(crate::docs::SPEC) => pages.push(rel),
            Some(crate::docs::DECISIONS) => decisions.push(rel),
            Some("memory") => {}
            _ => documents.push(rel),
        }
    }
    (pages, decisions, documents)
}

fn relative_folder(project: &crate::model::Project) -> String {
    let dir = sharing::dir_of(project);
    dir.strip_prefix(&project.path)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| sharing::DIR.to_string())
}

fn problem_of(read: &Read) -> Option<String> {
    match read {
        Read::Ok(..) => None,
        Read::Conflict => Some(tr!(
            "son fichier est en conflit après un pull",
            "its file is in conflict after a pull"
        )),
        Read::Invalid(line, why) => Some(match line {
            Some(l) => tr!("ligne {l} : {why}", "line {l}: {why}"),
            None => why.clone(),
        }),
    }
}

/// The tickets of this machine that « Verser à l'équipe » offers: of the project, of no team and of no file.
fn local_choices(tickets: &[Ticket], project_id: &str) -> Vec<Ticket> {
    tickets
        .iter()
        .filter(|t| t.project_id == project_id && t.team.is_none() && t.shared.is_none())
        .cloned()
        .collect()
}

impl Core {
    fn team_board_of(
        &self,
        project_id: &str,
    ) -> Result<(
        crate::model::Project,
        Arc<crate::teams::Teams>,
        String,
        ProjectId,
    )> {
        let project = self.project(project_id)?;
        let (Some(team), Some(hex_id)) = (project.team.clone(), project.team_project.clone())
        else {
            bail!(tr!(
                "Ce Kanban n’est pas encore celui de l’équipe : partage-le ou rejoins-le d’abord.",
                "This Kanban is not the team’s yet: share it or join it first."
            ));
        };
        let mut b = [0u8; 16];
        hex::decode_to_slice(&hex_id, &mut b).map_err(|_| anyhow!("a journal's id"))?;
        let teams = self.teams_link().ok_or_else(|| anyhow!("no teams"))?;
        Ok((project, teams, team, ProjectId(b)))
    }

    /// The preview of « Verser à l'équipe »: the tickets of the files and those of this machine, which are already in
    /// the team's Kanban. Reads nothing from the server.
    pub(crate) async fn team_pour_preview(
        self: &Arc<Self>,
        project_id: &str,
    ) -> Result<PourPreview> {
        let (project, teams, team_id, pid) = self.team_board_of(project_id)?;
        let shared = teams.team(&team_id).map_err(|e| anyhow!(e))?;
        let files = sharing::read_dir(&sharing::dir_of(&project));
        let mut in_journal = BTreeMap::new();
        {
            let mut team = shared.lock().await;
            for sid in files.keys() {
                let o = object_of_file(sid);
                let mut b = [0u8; 16];
                let _ = hex::decode_to_slice(&o, &mut b);
                let r = team.replica(&pid).map_err(|e| anyhow!(e.to_string()))?;
                let there = r.object(&ObjectId(b)).ok().flatten().is_some()
                    || r.is_deleted(&ObjectId(b)).unwrap_or(false);
                in_journal.insert(sid.clone(), there);
            }
        }
        let tickets = self.tickets.read().clone();
        let mine = |sid: &str| {
            let o = object_of_file(sid);
            tickets.iter().any(|t| {
                t.project_id == project_id && t.team.as_ref().is_some_and(|x| x.object == o)
            })
        };
        let files = files
            .iter()
            .map(|(sid, text)| {
                let read = sharing::read(text);
                let problem = problem_of(&read);
                let (key, title, column, external) = match &read {
                    Read::Ok(f, _) => (
                        f.key.clone(),
                        f.title.clone(),
                        f.status,
                        f.external.is_some(),
                    ),
                    _ => (sid.clone(), format!("{sid}.yml"), Column::Todo, false),
                };
                PourItem {
                    id: sid.clone(),
                    key,
                    title,
                    column,
                    external,
                    poured: in_journal.get(sid).copied().unwrap_or(false) || mine(sid),
                    problem,
                }
            })
            .collect();
        let locals = local_choices(&tickets, project_id)
            .into_iter()
            .map(|t| PourItem {
                external: t.external.is_some(),
                id: t.id,
                key: t.key,
                title: t.title,
                column: t.column,
                poured: false,
                problem: None,
            })
            .collect();
        let docs = self.pour_docs(&project).await;
        Ok(PourPreview {
            files,
            locals,
            folder: relative_folder(&project),
            docs,
        })
    }

    /// What becomes of the docs of the repository's `.escouade/` at « Verser » (the module's doc).
    async fn pour_docs(&self, project: &crate::model::Project) -> PourDocs {
        let (pages, decisions, documents) = doc_files(project);
        let memories: Vec<String> = crate::memory::list(&crate::memory::dir_of(project))
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.name)
            .collect();
        let state = if project.private {
            "private"
        } else {
            match self.team_docs_state(project).await {
                Some((Some(p), _)) if p == "repo" => "repo",
                Some((_, true)) | Some((Some(_), _)) => "server",
                _ if pages.is_empty()
                    && decisions.is_empty()
                    && documents.is_empty()
                    && memories.is_empty() =>
                {
                    "none"
                }
                _ => "pending",
            }
        };
        // The memory follows the docs; on the server already, it goes once by this gesture when no device poured it.
        let memory = match state {
            "pending" if !memories.is_empty() => "pending",
            "server" if memories.is_empty() => "none",
            "server" if self.team_memory_pending(project).await => "pending",
            "pending" => "none",
            s => s,
        };
        let memories = if memory == "pending" {
            memories
        } else {
            Vec::new()
        };
        if state != "pending" {
            return PourDocs {
                state: state.into(),
                memory: memory.into(),
                memories,
                ..Default::default()
            };
        }
        PourDocs {
            state: state.into(),
            pages,
            decisions,
            documents,
            memory: memory.into(),
            memories,
        }
    }

    /// « Verser à l'équipe »: the tickets of the files (`files`), and those of this machine named (`tickets`, by id),
    /// into the team's journal (the module's doc). The files frozen; their names, to remove them by hand.
    pub(crate) async fn team_pour(
        self: &Arc<Self>,
        project_id: &str,
        files: bool,
        tickets: Vec<String>,
        docs: bool,
    ) -> Result<Poured> {
        let (project, teams, team_id, pid) = self.team_board_of(project_id)?;
        let shared = teams.team(&team_id).map_err(|e| anyhow!(e))?;
        let dir = sharing::dir_of(&project);
        let texts = if files {
            sharing::read_dir(&dir)
        } else {
            BTreeMap::new()
        };
        let mut done = Poured {
            imported: 0,
            already: 0,
            locals: 0,
            files: Vec::new(),
            folder: relative_folder(&project),
            docs: Vec::new(),
        };
        // The tickets of the files.
        let mut linked: Vec<(Option<String>, Ticket)> = Vec::new();
        {
            let snapshot = self.tickets.read().clone();
            let mut team = shared.lock().await;
            for (sid, text) in &texts {
                let Read::Ok(f, _) = sharing::read(text) else {
                    continue;
                };
                let o = object_of_file(sid);
                let mut b = [0u8; 16];
                hex::decode_to_slice(&o, &mut b).map_err(|_| anyhow!("an object"))?;
                let object = ObjectId(b);
                let here = snapshot.iter().find(|t| {
                    t.project_id == project_id && t.team.as_ref().is_some_and(|x| x.object == o)
                });
                let r = team.replica(&pid).map_err(|e| anyhow!(e.to_string()))?;
                let there = r.object(&object).ok().flatten().is_some()
                    || r.is_deleted(&object).unwrap_or(false);
                if here.is_some() || there {
                    // Poured before (here, or by a teammate: this machine's ticket of the file joins it at the next
                    // reading), or deleted since: never written again.
                    done.already += 1;
                    continue;
                }
                let local = snapshot.iter().find(|t| {
                    t.project_id == project_id && t.shared.as_ref().is_some_and(|s| s.id == *sid)
                });
                let mut t = match local {
                    Some(t) => t.clone(),
                    None => Ticket {
                        id: new_id(),
                        project_id: project_id.to_string(),
                        key: f.key.clone(),
                        title: f.title.clone(),
                        description: f.description.clone(),
                        criteria: f
                            .criteria
                            .iter()
                            .map(|c| Criterion {
                                text: c.clone(),
                                ..Default::default()
                            })
                            .collect(),
                        column: f.status,
                        created_at: now_ms(),
                        max_loops: 5,
                        ..Default::default()
                    },
                };
                let mut former = vec![f.key.clone()];
                if !t.key.is_empty() && t.key != f.key {
                    former.push(t.key.clone());
                }
                for k in t.former_keys.drain(..) {
                    if !former.contains(&k) {
                        former.push(k);
                    }
                }
                t.former_keys = former;
                let mut claim = [0u8; 16];
                getrandom::fill(&mut claim).map_err(|e| anyhow!(e.to_string()))?;
                let seen = fields_of(&t);
                let mut first = seen.clone();
                first.insert("claim".into(), json!(hex::encode(claim)));
                team.write(
                    pid,
                    OpBody::Create {
                        object,
                        kind: "ticket".into(),
                        fields: first,
                    },
                )
                .map_err(|e| anyhow!(e.to_string()))?;
                t.team = Some(TeamTicket {
                    object: o,
                    claim: hex::encode(claim),
                    seen: seen.into_iter().collect::<BTreeMap<String, Value>>(),
                    // An imported ticket keeps its key (§4.1); the others take the server's at the next reading.
                    keyed: t.external.is_some() || f.external.is_some(),
                    ..Default::default()
                });
                t.shared = None;
                t.team_local = false;
                done.imported += 1;
                linked.push((local.map(|l| l.id.clone()), t));
            }
            if done.imported > 0 {
                if let Err(e) = team.sync_project(pid).await {
                    log::info!("team pour: not sent yet: {e}");
                }
            }
        }
        // Into the board, at once: each file's ticket as the team's.
        let mut changed = Vec::new();
        {
            let mut all = self.tickets.write();
            for (was, t) in linked {
                match was.and_then(|id| all.iter_mut().find(|x| x.id == id)) {
                    Some(x) => *x = t.clone(),
                    None => all.push(t.clone()),
                }
                changed.push(t);
            }
            // The tickets of this machine chosen: they go at the next reading, with a key of the server.
            for x in all.iter_mut().filter(|x| {
                x.project_id == project_id
                    && x.team.is_none()
                    && x.shared.is_none()
                    && tickets.contains(&x.id)
            }) {
                if !x.key.is_empty() && !x.former_keys.contains(&x.key) && x.external.is_none() {
                    x.former_keys.push(x.key.clone());
                }
                x.team_local = false;
                done.locals += 1;
                changed.push(x.clone());
            }
        }
        for t in changed {
            self.hub.emit(UiEvent::Ticket { ticket: t });
        }
        if !texts.is_empty() {
            done.files = sharing::freeze(&dir);
        }
        // The docs of `.escouade/`, when the team has none on its server yet: imported as « Sur le serveur d'équipe »
        // does it (ids and numbers kept, nothing removed from the repository).
        let before = self.pour_docs(&project).await;
        if docs && before.state == "pending" {
            let view = self.team_docs_choose(project_id, true, None).await?;
            done.docs = view.repo_files;
        } else if docs && before.memory == "pending" {
            // The docs on the server already: the memory alone (TS-10).
            self.team_memory_pour(project_id).await?;
            done.docs = crate::teamdocs::repo_memory_files(&crate::docsbranch::base_of(&project));
        }
        self.request_save();
        self.sync_team_board(project_id).await;
        // A ticket of a file this machine's agent runs: taken, as « Lancer » would.
        let running: Vec<String> = self
            .tickets
            .read()
            .iter()
            .filter(|t| {
                t.project_id == project_id
                    && t.agent_id.is_some()
                    && t.column == Column::Doing
                    && t.team
                        .as_ref()
                        .is_some_and(|x| x.token.is_none() && x.holder.is_none())
            })
            .map(|t| t.id.clone())
            .collect();
        for id in running {
            if let Err(why) = self.team_take(&project, &id).await {
                log::info!("team pour: {id} not taken: {why}");
            }
        }
        Ok(done)
    }
}

#[cfg(test)]
#[path = "teampour_tests.rs"]
mod tests;
