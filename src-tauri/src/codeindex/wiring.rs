//! The index in the app: the project's setting, the folder an agent asks about, the fence, the git
//! events that bring the index up to date.

use super::{Reader, Status, Target};
use crate::core::Core;
use crate::fence::Fence;
use anyhow::{bail, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::Runtime;

/// « Index du code » of a project, as the settings read it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeIndexInfo {
    /// On: its own choice, or the app's (`Core::code_index_on`).
    pub enabled: bool,
    /// The project made its own choice (ESC-293); else it follows « Index du code sur tous les
    /// projets ».
    pub chosen: bool,
    /// Being read for « Index du code sur tous les projets », or waiting for its turn.
    pub reading: bool,
    pub waiting: bool,
    pub status: Status,
    /// The folders of the project's own that its fence leaves out of the index (ESC-254): those of
    /// another project inside it, of the other side of a private project. Relative to the project,
    /// with `/`.
    pub excluded: Vec<String>,
}

impl<R: Runtime> Core<R> {
    /// What the agent working in `dir` (the project's folder when None) is asked about: refused when
    /// the folder is none of the project's.
    pub(crate) fn index_target(&self, project_id: &str, dir: Option<&str>) -> Result<Target> {
        let project = self.project(project_id)?;
        let root = PathBuf::from(&project.path);
        let worktrees = project.worktrees_root();
        let mut agents: Vec<PathBuf> = self
            .project_agents(project_id)
            .iter()
            .map(|h| PathBuf::from(&h.lock().meta.cwd))
            .filter(|c| !c.as_os_str().is_empty())
            .collect();
        agents.retain(|c| !crate::paths::same_folder(&c.to_string_lossy(), &project.path));
        let dir = match dir.map(str::trim).filter(|d| !d.is_empty()) {
            None => root.clone(),
            Some(d) => {
                let known = crate::paths::same_folder(d, &project.path)
                    || Fence::new(vec![worktrees.clone()]).covers(d)
                    || agents
                        .iter()
                        .any(|a| crate::paths::same_folder(d, &a.to_string_lossy()));
                if !known {
                    bail!(tr!(
                        "Ce dossier n’est pas celui du projet ni d’un de ses worktrees.",
                        "This folder is neither the project's nor one of its worktrees'."
                    ));
                }
                PathBuf::from(d)
            }
        };
        let mut exclude = vec![worktrees];
        exclude.extend(agents);
        Ok(Target {
            project_id: project_id.to_string(),
            root,
            dir,
            fence: self.project_fence(project_id),
            exclude,
        })
    }

    /// A reader of the project's index for the folder `dir`, the files checked first (at most once
    /// every few seconds). An error when the project has not turned the index on.
    pub async fn code_index(
        self: &Arc<Self>,
        project_id: &str,
        dir: Option<&str>,
    ) -> Result<Reader> {
        if !self.code_index_on(&self.project(project_id)?) {
            bail!(tr!(
                "L’index du code est coupé pour ce projet.",
                "The code index is off for this project."
            ));
        }
        let target = self.index_target(project_id, dir)?;
        let me = self.clone();
        tokio::task::spawn_blocking(move || me.codeindex.reader(&target)).await?
    }

    /// A commit, a change of branch, the end of an agent's turn: the index reads what changed, in
    /// the background. Nothing when the project has not turned it on.
    pub(crate) fn code_index_event(self: &Arc<Self>, project_id: &str) {
        if !self
            .project(project_id)
            .is_ok_and(|p| self.code_index_on(&p))
        {
            return;
        }
        let Ok(target) = self.index_target(project_id, None) else {
            return;
        };
        let me = self.clone();
        let id = project_id.to_string();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = me.codeindex.ensure_fresh(&target, true) {
                log::warn!("code index of {id} not refreshed: {e:#}");
            }
        });
    }

    /// What the hook of an agent's view answers a search for a symbol (`hint`): None when the
    /// project has not turned the index on, the harness cannot read it, or the search is for a text.
    pub(crate) fn index_hint(
        &self,
        agent_id: &str,
        hook: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        let (project_id, engine) = {
            let h = self.agent(agent_id).ok()?;
            let m = h.lock().meta.clone();
            (m.project_id, m.engine)
        };
        if !self.code_index_on(&self.project(&project_id).ok()?) {
            return None;
        }
        super::hint::reply(engine, self.lang().claude, hook)
    }

    /// Whether the project's index is on: its own choice, or « Index du code sur tous les
    /// projets » when it made none (ESC-293). Every reader of the index asks here.
    pub fn code_index_on(&self, project: &crate::model::Project) -> bool {
        project.code_index_on(self.settings.read().code_index_all)
    }

    pub fn code_index_info(&self, project_id: &str) -> Result<CodeIndexInfo> {
        let project = self.project(project_id)?;
        let root = PathBuf::from(&project.path);
        let mut excluded: Vec<String> = self
            .project_fence(project_id)
            .paths()
            .iter()
            .filter_map(|p| p.strip_prefix(&root).ok())
            .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            .filter(|rel| !rel.is_empty())
            .collect();
        excluded.sort();
        excluded.dedup();
        let place = self
            .codeindex
            .queue
            .lock()
            .iter()
            .position(|id| id == project_id);
        Ok(CodeIndexInfo {
            enabled: self.code_index_on(&project),
            chosen: project.code_index.is_some(),
            reading: place == Some(0),
            waiting: place.is_some_and(|at| at > 0),
            status: self.codeindex.status(project_id),
            excluded,
        })
    }

    /// The setting « Index du code » of the project: on or off, its own choice; None follows the
    /// app's setting again. On, the project is read at once in the background; off, its database
    /// is erased.
    pub async fn set_code_index(
        self: &Arc<Self>,
        project_id: &str,
        choice: Option<bool>,
    ) -> Result<CodeIndexInfo> {
        let updated = {
            let mut projects = self.projects.write();
            let cur = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(crate::core::project_not_found)?;
            cur.code_index = choice;
            cur.clone()
        };
        self.hub.emit(crate::model::UiEvent::Project {
            project: updated.clone(),
        });
        self.request_save();
        // Out of the list of « Index du code sur tous les projets »: read here, or erased.
        self.codeindex.queue.lock().retain(|id| id != project_id);
        if self.code_index_on(&updated) {
            let target = self.index_target(project_id, None)?;
            let me = self.clone();
            tokio::task::spawn_blocking(move || me.codeindex.ensure_fresh(&target, true)).await??;
        } else {
            let me = self.clone();
            let id = project_id.to_string();
            tokio::task::spawn_blocking(move || me.codeindex.delete(&id)).await?;
        }
        self.code_index_info(project_id)
    }

    /// The projects that follow « Index du code sur tous les projets » (no choice of their own),
    /// in the order of the projects' bar.
    pub fn code_index_followers(&self) -> Vec<String> {
        self.projects
            .read()
            .iter()
            .filter(|p| p.code_index.is_none())
            .map(|p| p.id.clone())
            .collect()
    }

    /// « Index du code sur tous les projets » turned on or off (ESC-293), saved at once: on, the
    /// projects that follow it are read one after the other, in the background; off, their index
    /// is erased. The number of those projects. The model of the search by meaning is not
    /// downloaded for it.
    pub async fn set_code_index_all(self: &Arc<Self>, enabled: bool) -> Result<usize> {
        self.settings.write().code_index_all = enabled;
        let settings = self.settings.read().clone();
        self.save_settings(settings)?;
        let followers = self.code_index_followers();
        if enabled {
            self.queue_code_index(followers.clone());
        } else {
            self.codeindex
                .queue
                .lock()
                .retain(|id| !followers.contains(id));
            let me = self.clone();
            let ids = followers.clone();
            tokio::task::spawn_blocking(move || {
                for id in &ids {
                    me.codeindex.delete(id);
                }
            })
            .await?;
            for id in &followers {
                self.hub.emit(crate::model::UiEvent::CodeIndex {
                    project_id: id.clone(),
                });
            }
        }
        Ok(followers.len())
    }

    /// The projects to be read by « Index du code sur tous les projets », after those already
    /// waiting: one at a time, never all at once, on a task of their own.
    pub(crate) fn queue_code_index(self: &Arc<Self>, ids: Vec<String>) {
        let added: Vec<String> = {
            let mut queue = self.codeindex.queue.lock();
            let added: Vec<String> = ids.into_iter().filter(|id| !queue.contains(id)).collect();
            queue.extend(added.iter().cloned());
            added
        };
        for id in added {
            self.hub
                .emit(crate::model::UiEvent::CodeIndex { project_id: id });
        }
        if self.codeindex.draining.swap(true, Ordering::AcqRel) {
            return;
        }
        let me = self.clone();
        tauri::async_runtime::spawn(async move { me.drain_code_index().await });
    }

    async fn drain_code_index(self: Arc<Self>) {
        loop {
            let next = self.codeindex.queue.lock().first().cloned();
            let Some(id) = next else {
                self.codeindex.draining.store(false, Ordering::Release);
                // One queued between the look and the store: read by this task still.
                if self.codeindex.queue.lock().is_empty()
                    || self.codeindex.draining.swap(true, Ordering::AcqRel)
                {
                    return;
                }
                continue;
            };
            self.hub.emit(crate::model::UiEvent::CodeIndex {
                project_id: id.clone(),
            });
            // Turned off, chosen by hand or closed meanwhile: not read.
            let target = self
                .project(&id)
                .ok()
                .filter(|p| self.code_index_on(p))
                .and_then(|_| self.index_target(&id, None).ok());
            if let Some(target) = target {
                let me = self.clone();
                let read =
                    tokio::task::spawn_blocking(move || me.codeindex.ensure_fresh(&target, true))
                        .await;
                match read {
                    Ok(Err(e)) => log::warn!("code index of {id} not read: {e:#}"),
                    Err(e) => log::warn!("code index of {id} not read: {e}"),
                    Ok(Ok(_)) => {}
                }
            }
            self.codeindex.queue.lock().retain(|x| x != &id);
            self.hub
                .emit(crate::model::UiEvent::CodeIndex { project_id: id });
        }
    }
}
