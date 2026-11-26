//! What the language servers take from the core (ESC-446): the root of a project or of an agent's worktree, the app's
//! settings, the project's `.escouade.yml`, and whether the user trusts the project with its own servers.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;

use crate::core::Core;
use crate::model::LanguageServerSetting;

use super::catalog::Server;
use super::locate::Places;
use super::{Config, Launch, Refused, Status, Trust, WorktreeTrust};

/// The inputs of a project's servers.
pub struct Inputs {
    /// Where they run: the worktree's root, or the project's.
    pub root: PathBuf,
    /// The roots looked in: the worktree's, then the project's.
    pub roots: Vec<PathBuf>,
    pub app: BTreeMap<String, LanguageServerSetting>,
    /// What the project's `.escouade.yml` set, as read.
    pub file: BTreeMap<String, Value>,
    /// The project's trust, and in an agent's worktree that worktree's own (ESC-648).
    pub trust: Trust,
}

impl Inputs {
    pub fn config(&self, server: &Server) -> Config {
        super::config(
            server,
            &self.app,
            super::file_settings(&self.file, server.id),
        )
    }

    pub fn statuses(&self) -> Vec<Status> {
        let places = Places::system(self.roots.clone());
        super::catalog::SERVERS
            .iter()
            .map(|s| super::status(s, &self.config(s), &places, &self.trust))
            .collect()
    }

    pub fn launch(&self, server: &Server) -> Result<Launch, Refused> {
        let places = Places::system(self.roots.clone());
        super::launch_with(
            server,
            &self.config(server),
            &places,
            &self.trust,
            self.root.clone(),
        )
    }
}

impl Core {
    /// The inputs of the servers of `project_id`, in the worktree of `agent_id` when given.
    pub async fn lsp_inputs(&self, project_id: &str, agent_id: Option<String>) -> Result<Inputs> {
        let (root, _) = self.edit_root(project_id, agent_id.clone()).await?;
        let project = self.project(project_id)?;
        let root = PathBuf::from(root);
        let mut roots = vec![root.clone()];
        let own = PathBuf::from(&project.path);
        // An agent's worktree: what its agent wrote there waits for its own trust (SR4-06).
        let worktree = match &agent_id {
            Some(agent) if own != root => Some(WorktreeTrust {
                trusted: self
                    .lsp_worktrees_trusted
                    .lock()
                    .contains(&worktree_key(agent, &root)),
                root: root.clone(),
            }),
            _ => None,
        };
        if own != root {
            roots.push(own);
        }
        let settings = self.settings.read();
        Ok(Inputs {
            root,
            roots,
            app: settings.language_servers.clone(),
            file: project.shared.file.map(|f| f.values).unwrap_or_default(),
            trust: Trust {
                project: settings
                    .language_servers_trusted
                    .iter()
                    .any(|p| p == project_id),
                worktree,
            },
        })
    }

    /// The inputs with no project: the machine's servers alone, no root to look in.
    pub fn lsp_machine_inputs(&self) -> Inputs {
        Inputs {
            root: PathBuf::new(),
            roots: Vec::new(),
            app: self.settings.read().language_servers.clone(),
            file: BTreeMap::new(),
            trust: Trust::default(),
        }
    }

    /// Trusts `project_id` with its own language servers, or no longer: kept in the settings.
    pub fn lsp_trust(&self, project_id: &str, trusted: bool) -> Result<()> {
        self.project(project_id)?;
        {
            let mut s = self.settings.write();
            s.language_servers_trusted.retain(|p| p != project_id);
            if trusted {
                s.language_servers_trusted.push(project_id.to_string());
            }
        }
        self.write_settings_file();
        Ok(())
    }

    /// Trusts the worktree of `agent_id` (in `project_id`) with the language servers that run what its agent wrote, or
    /// no longer (ESC-648): for this run of the app, never by the project's trust. Answers the worktree's root.
    pub async fn lsp_trust_worktree(
        &self,
        project_id: &str,
        agent_id: &str,
        trusted: bool,
    ) -> Result<PathBuf> {
        let (root, _) = self
            .edit_root(project_id, Some(agent_id.to_string()))
            .await?;
        let root = PathBuf::from(root);
        if std::path::Path::new(&self.project(project_id)?.path) == root {
            anyhow::bail!(tr!(
                "cet agent travaille dans le dossier du projet : c’est la confiance du projet qui compte",
                "this agent works in the project’s folder: the project’s trust is the one that counts"
            ));
        }
        let key = worktree_key(agent_id, &root);
        let mut set = self.lsp_worktrees_trusted.lock();
        if trusted {
            set.insert(key);
        } else {
            set.remove(&key);
        }
        Ok(root)
    }
}

/// The key of an agent's worktree trusted: the agent and its root, so that another agent in the same folder is asked
/// again.
fn worktree_key(agent_id: &str, root: &std::path::Path) -> String {
    format!("{agent_id}\n{}", root.display())
}
