//! The gate of `escouade-server`, in the core (RP-1, `docs/PRIVATE-NETWORK.md` § 4.3; spec « Escouade à distance ›
//! Atteindre un réseau privé »). The gate itself is `escouade-gate`; here, what the core does with it:
//!
//! - **It starts** with the server when the private network is on (`escouade-server --gate`, `ESCOUADE_SERVER_GATE`;
//!   RP-4 turns it on with the connectors' entry): never on the desktop, whose processes keep the proxy set.
//! - **The variables**: an agent's processes get the gate in place of the space's proxy, with a token of its own
//!   (`agent_env`); its project's terminals, launches and the core's own `git` over the network with a token of the
//!   project (`project_env`); `GIT_SSH_COMMAND` with `escouade-server via %h %p`, set by the core alone.
//! - **`core.sshCommand`** written in the repository of an agent's worktree as it starts, the user's own ssh command
//!   kept in it, for a `git` run without the agent's variables.
//! - **The lists** of the projects read from `private-network.json` (read again when it changes), and the space's
//!   proxy behind the gate following the settings.
//!
//! The tokens live in memory only, in the variables of the processes they are given to: never in a file nor in the log.

use crate::core::Core;
use crate::model::Settings;
use anyhow::{Context, Result};
use escouade_gate::{Config, Connectors, Gate, Holder, Lang, Lists, Outside};
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

/// What holds a project's token, but its agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Terminal,
    Launch,
    /// The core's own `git` over the network.
    Git,
}

impl Kind {
    fn who(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Launch => "launch",
            Self::Git => "git",
        }
    }
}

/// The gate of the core, when it has one.
#[derive(Default)]
pub struct GateSlot {
    gate: RwLock<Option<Gate>>,
    /// Each agent's token, with its project.
    agents: Mutex<HashMap<String, (String, String)>>,
    /// Each project's tokens, by what holds them.
    projects: Mutex<HashMap<(String, Kind), String>>,
    /// The lists' file, and its time when it was read.
    lists: Mutex<Option<(PathBuf, Option<SystemTime>)>>,
    /// `escouade-server`'s own path, for `via`.
    exe: OnceLock<String>,
}

fn lang() -> Lang {
    match crate::i18n::ui() {
        crate::i18n::Lang::En => Lang::En,
        _ => Lang::Fr,
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn read_lists(path: &Path) -> Lists {
    Lists::read(path).unwrap_or_else(|e| {
        log::warn!(
            "gate: {} unreadable, no project reaches a private network: {e}",
            path.display()
        );
        Lists::default()
    })
}

/// The way out of the gate, from the settings: their proxy behind it, and its `NO_PROXY`.
pub fn outside_of(settings: &Settings) -> Outside {
    Outside::of_proxy(&settings.proxy_url, &settings.no_proxy)
}

/// The user's own ssh command for `dir`: its repository's `core.sshCommand`, else the process's `GIT_SSH_COMMAND`.
fn their_ssh(dir: &str) -> Option<String> {
    let own = std::process::Command::new("git")
        .args(["-C", dir, "config", "--get", "core.sshCommand"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    own.or_else(|| {
        std::env::var("GIT_SSH_COMMAND")
            .ok()
            .filter(|s| !s.trim().is_empty())
    })
}

impl GateSlot {
    /// Whether the gate runs.
    pub fn is_on(&self) -> bool {
        self.gate.read().is_some()
    }

    /// The gate, when it runs.
    pub fn gate(&self) -> Option<Gate> {
        self.gate.read().clone()
    }

    /// The gate started on 127.0.0.1 (`port`, 0 for one the system chooses), the lists of `data_root` read, the
    /// settings' proxy behind it. On the runtime it is called on.
    pub async fn start(
        &self,
        data_root: &Path,
        settings: &Settings,
        connectors: Arc<dyn Connectors>,
        port: u16,
    ) -> Result<Gate> {
        let path = data_root.join(escouade_gate::lists::FILE);
        let lists = read_lists(&path);
        let gate = Gate::start(Config {
            lang: lang(),
            connectors,
            lists,
            outside: outside_of(settings),
            port,
        })
        .await
        .context("the gate")?;
        *self.lists.lock() = Some((path.clone(), modified(&path)));
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "escouade-server".into());
        let _ = self.exe.set(exe);
        *self.gate.write() = Some(gate.clone());
        Ok(gate)
    }

    /// The gate stopped: the processes started after it go out as before.
    pub fn stop(&self) {
        if let Some(g) = self.gate.write().take() {
            g.stop();
        }
        self.agents.lock().clear();
        self.projects.lock().clear();
    }

    /// The space's proxy behind the gate, as the settings now say.
    pub fn follow(&self, settings: &Settings) {
        if let Some(g) = self.gate() {
            g.set_outside(outside_of(settings));
        }
    }

    /// The lists read again when their file changed. True when they were.
    pub fn reload_if_changed(&self) -> bool {
        let Some(g) = self.gate() else { return false };
        let mut held = self.lists.lock();
        let Some((path, seen)) = held.as_mut() else {
            return false;
        };
        let now = modified(path);
        if now == *seen {
            return false;
        }
        *seen = now;
        g.set_lists(read_lists(path));
        log::info!("gate: the projects' lists read again");
        true
    }

    fn exe(&self) -> &str {
        self.exe
            .get()
            .map(String::as_str)
            .unwrap_or("escouade-server")
    }

    /// The ssh command of git in `dir`: the user's own, through `escouade-server via`.
    pub fn ssh_command(&self, dir: &str) -> String {
        escouade_gate::env::ssh_command(their_ssh(dir).as_deref(), self.exe())
    }

    /// `core.sshCommand` of the repository of `dir` through the gate, the user's own ssh command kept in it: for a
    /// `git` run there without the agent's variables. Already so, nothing is written.
    pub fn write_ssh_config(&self, dir: &str) {
        if !self.is_on() || !Path::new(dir).is_dir() {
            return;
        }
        let theirs = their_ssh(dir);
        if theirs.as_deref().is_some_and(escouade_gate::env::is_ours) {
            return;
        }
        let ours = escouade_gate::env::ssh_command(theirs.as_deref(), self.exe());
        let written = std::process::Command::new("git")
            .args(["-C", dir, "config", "core.sshCommand", &ours])
            .stdin(std::process::Stdio::null())
            .output();
        match written {
            Ok(o) if o.status.success() => {
                log::info!("gate: core.sshCommand of {dir} goes through escouade-server via")
            }
            Ok(o) => log::warn!(
                "gate: core.sshCommand of {dir} not written: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ),
            Err(e) => log::warn!("gate: core.sshCommand of {dir} not written: {e}"),
        }
    }

    /// The gate's variables for the agent `agent` of `project` in `dir`, its token given once (a token of another
    /// project given back first). `None` without the gate.
    pub fn agent_env(
        &self,
        agent: &str,
        project: &str,
        project_name: &str,
        dir: &str,
    ) -> Option<Vec<(String, String)>> {
        let g = self.gate()?;
        let token = {
            let mut agents = self.agents.lock();
            match agents.get(agent) {
                Some((p, t)) if p == project && g.holder(t).is_some() => t.clone(),
                held => {
                    if let Some((_, t)) = held {
                        g.revoke(t);
                    }
                    let t = g.issue(Holder {
                        who: agent.to_string(),
                        project: project.to_string(),
                        project_name: project_name.to_string(),
                    });
                    agents.insert(agent.to_string(), (project.to_string(), t.clone()));
                    t
                }
            }
        };
        Some(g.env(&token, &self.ssh_command(dir)))
    }

    /// The gate's variables for what `kind` runs of `project` in `dir`. `None` without the gate.
    pub fn project_env(
        &self,
        kind: Kind,
        project: &str,
        project_name: &str,
        dir: &str,
    ) -> Option<Vec<(String, String)>> {
        let g = self.gate()?;
        let token = {
            let mut projects = self.projects.lock();
            let key = (project.to_string(), kind);
            match projects.get(&key) {
                Some(t) if g.holder(t).is_some() => t.clone(),
                _ => {
                    let t = g.issue(Holder {
                        who: kind.who().to_string(),
                        project: project.to_string(),
                        project_name: project_name.to_string(),
                    });
                    projects.insert(key, t.clone());
                    t
                }
            }
        };
        Some(g.env(&token, &self.ssh_command(dir)))
    }

    /// The agent's token taken back: it is gone.
    pub fn forget_agent(&self, agent: &str) {
        if let Some((_, t)) = self.agents.lock().remove(agent) {
            if let Some(g) = self.gate() {
                g.revoke(&t);
            }
        }
    }
}

impl Core {
    /// The project of `id`'s name, its id when it has none.
    fn project_name(&self, id: &str) -> String {
        self.projects
            .read()
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| id.to_string())
    }

    /// `env` of the agent `id` of `project` in `dir` with the gate in place of the space's proxy, when the server has
    /// it; and the repository's `core.sshCommand` through it.
    pub(crate) fn gate_agent_env(
        &self,
        id: &str,
        project: &str,
        dir: &str,
        env: &mut Vec<(String, String)>,
    ) {
        if let Some(vars) = self
            .gate
            .agent_env(id, project, &self.project_name(project), dir)
        {
            escouade_gate::env::with_gate(env, &vars);
            self.gate.write_ssh_config(dir);
        }
    }

    /// `env` of what `kind` runs of `project` in `dir` with the gate, when the server has it. True when it does: then
    /// the gate is its proxy, whether « le proxy aussi pour les terminaux » is on or not.
    pub(crate) fn gate_project_env(
        &self,
        kind: Kind,
        project: &str,
        dir: &str,
        env: &mut Vec<(String, String)>,
    ) -> bool {
        match self
            .gate
            .project_env(kind, project, &self.project_name(project), dir)
        {
            Some(vars) => {
                escouade_gate::env::with_gate(env, &vars);
                true
            }
            None => false,
        }
    }

    /// The project a folder is of: a project's own, or an agent's worktree.
    pub(crate) fn project_of_dir(&self, dir: &str) -> Option<String> {
        let dir = Path::new(dir);
        let by_project = self
            .projects
            .read()
            .iter()
            .filter(|p| !p.path.is_empty() && dir.starts_with(&p.path))
            .max_by_key(|p| p.path.len())
            .map(|p| p.id.clone());
        by_project.or_else(|| {
            self.agents.read().values().find_map(|h| {
                let rt = h.lock();
                (!rt.meta.cwd.is_empty() && dir.starts_with(&rt.meta.cwd))
                    .then(|| rt.meta.project_id.clone())
            })
        })
    }

    /// The variables of the core's own `git` over the network in `dir` (`escouade_model::git::set_net_env`): the
    /// gate's, with a token of its project. None out of a project.
    pub(crate) fn gate_git_env(&self, dir: &str) -> Vec<(String, String)> {
        if !self.gate.is_on() {
            return Vec::new();
        }
        let Some(project) = self.project_of_dir(dir) else {
            return Vec::new();
        };
        let mut env = Vec::new();
        self.gate_project_env(Kind::Git, &project, dir, &mut env);
        env
    }

    /// The gate started for this server (`escouade-server --gate`): its port chosen by the system, the core's `git`
    /// given it, its lists read again when their file changes.
    pub async fn start_gate(self: &Arc<Self>, connectors: Arc<dyn Connectors>) -> Result<Gate> {
        let settings = self.settings.read().clone();
        let gate = self
            .gate
            .start(self.data.root(), &settings, connectors, 0)
            .await?;
        let weak = Arc::downgrade(self);
        escouade_model::git::set_net_env(Arc::new(move |dir: &str| {
            weak.upgrade()
                .map(|core| core.gate_git_env(dir))
                .unwrap_or_default()
        }));
        let weak = Arc::downgrade(self);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let Some(core) = weak.upgrade() else { return };
                if !core.gate.is_on() {
                    return;
                }
                core.gate.reload_if_changed();
            }
        });
        log::info!(
            "escouade-server: the gate (private network) on 127.0.0.1:{}",
            gate.addr().port()
        );
        Ok(gate)
    }
}
