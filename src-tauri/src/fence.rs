//! « Projet privé », its files (ESC-140): the paths an agent's harness is told to refuse it.
//!
//! The MCP server keeps a private project out of sight (`mcp::resolve::Sight`); its files are kept
//! out by the rules each harness takes at its start (`Spawn::fence`):
//! - an agent of a project that is not private: the folders of the private projects (their own,
//!   their worktrees', those of their agents), their data in Escouade's folder, and their sessions
//!   in the harnesses' folders;
//! - an agent of a private project: those of every other project, the same;
//! - both: the rest of Escouade's folder (`state.json` holds every project) but the folders of the
//!   files joined to messages and to tickets, and the sessions a harness keeps mixed, whatever
//!   their project (Codex's are by date).
//!
//! With no private project, nothing is refused. The paths are those of this machine: each harness
//! writes them its way (`Engine::fence_settings`, Codex's profiles, OpenCode's rules). What is on
//! the disk is read at the agent's start: a folder made after (an agent made since) is the next
//! start's.

// The paths refused and their rules, in `escouade-model` (docs/CRATES.md); what reads the projects and the agents stays.
pub use escouade_model::fence::*;

use std::path::PathBuf;

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// What the files of project `id` keep out of its index: the fence of its own agents, as the
    /// projects are now (`compute`, with no agent). Nothing without a private project.
    pub(crate) fn project_fence(&self, id: &str) -> Fence {
        let projects = self.projects.read().clone();
        if !projects.iter().any(|p| p.private) {
            return Fence::default();
        }
        let sides = projects
            .iter()
            .map(|p| Side {
                id: p.id.clone(),
                private: p.private,
                folders: vec![PathBuf::from(&p.path), p.worktrees_root()],
                data: Vec::new(),
            })
            .collect();
        let settings = self.settings.read().clone();
        let (homes, files) = homes(&settings);
        compute(&Ground {
            own: id.to_string(),
            projects: sides,
            data_root: self.data.root().to_path_buf(),
            keep: Vec::new(),
            homes,
            files,
        })
    }

    /// The fence of the agent `id` as the projects are now (`compute`); none for an agent the
    /// core does not have.
    pub(crate) fn agent_fence(&self, id: &str) -> Fence {
        let Ok(h) = self.agent(id) else {
            return Fence::default();
        };
        let (own, cwd, ticket) = {
            let m = &h.lock().meta;
            (m.project_id.clone(), m.cwd.clone(), m.ticket_id.clone())
        };
        let projects = self.projects.read().clone();
        if !projects.iter().any(|p| p.private) {
            return Fence::default();
        }
        // Read apart: an agent's lock is never held with the list's.
        let handles: Vec<crate::agent::AgentHandle> =
            self.agents.read().values().cloned().collect();
        let agents: Vec<(String, String, String)> = handles
            .iter()
            .map(|h| {
                let m = &h.lock().meta;
                (m.id.clone(), m.project_id.clone(), m.cwd.clone())
            })
            .collect();
        let tickets: Vec<(String, String)> = self
            .tickets
            .read()
            .iter()
            .map(|t| (t.id.clone(), t.project_id.clone()))
            .collect();
        let sides = projects
            .iter()
            .map(|p| {
                let mut folders = vec![PathBuf::from(&p.path), p.worktrees_root()];
                let mut data = Vec::new();
                for (agent, project, cwd) in &agents {
                    if *project == p.id {
                        folders.push(PathBuf::from(cwd));
                        if crate::paths::is_agent_folder_id(agent) {
                            data.push(self.data.agent_files(agent));
                        }
                    }
                }
                for (ticket, project) in &tickets {
                    if *project == p.id {
                        data.push(self.data.ticket_dir(ticket));
                    }
                }
                Side {
                    id: p.id.clone(),
                    private: p.private,
                    folders,
                    data,
                }
            })
            .collect();
        let mut keep = vec![PathBuf::from(&cwd), self.data.agent_files(id)];
        if let Some(t) = &ticket {
            keep.push(self.data.ticket_dir(t));
        }
        let settings = self.settings.read().clone();
        let (homes, files) = homes(&settings);
        compute(&Ground {
            own,
            projects: sides,
            data_root: self.data.root().to_path_buf(),
            keep,
            homes,
            files,
        })
    }
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// What the agent `id` is fenced by of the projects' privacy, now (`Privacy`).
    pub(crate) fn agent_privacy(&self, id: &str) -> Privacy {
        let own = self
            .agent(id)
            .ok()
            .map(|h| h.lock().meta.project_id.clone())
            .unwrap_or_default();
        let projects = self.projects.read();
        let mut private: Vec<String> = projects
            .iter()
            .filter(|p| p.private)
            .map(|p| p.id.clone())
            .collect();
        private.sort();
        Privacy {
            own: private.contains(&own),
            private,
        }
    }

    /// Whether the live process of the agent `id` (`h`) was stopped to start again with its fence:
    /// a project made private, or no longer, since it started (`Privacy`), not a path made since. Only between its turns, as the idle
    /// stop does it (never one that must stay reachable, or whose process holds work): its session
    /// is kept, the next start resumes it. An agent at work keeps its fence until its turn is over.
    pub(crate) async fn refence(
        &self,
        h: &crate::agent::AgentHandle,
        id: &str,
    ) -> anyhow::Result<bool> {
        let now = self.agent_privacy(id);
        let stopped = {
            let mut rt = h.lock();
            let idle = rt.proc.is_some()
                && !rt.meta.remote_control
                && !rt.meta.status.is_active()
                && rt.setup.is_none()
                && !rt
                    .meta
                    .plan
                    .as_ref()
                    .is_some_and(|p| p.holds_process(crate::model::now_ms()));
            if !idle || rt.privacy == now {
                return Ok(false);
            }
            rt.detach()
        };
        let Some(p) = stopped else {
            return Ok(false);
        };
        log::info!("agent {id}: its fence changed, its process starts again");
        self.emit_agent(h);
        if !crate::accounts::ended(&p).await {
            anyhow::bail!(tr!(
                "Le process de l’agent ne s’arrête pas.",
                "The agent’s process won’t stop."
            ));
        }
        Ok(true)
    }

    /// Writes `settings` (`Engine::fence_settings`) as the agent `id`'s, and gives its path.
    pub(crate) fn write_fence_settings(
        &self,
        id: &str,
        settings: &serde_json::Value,
    ) -> anyhow::Result<PathBuf> {
        use anyhow::Context;
        let file = self.data.fence_settings(id);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        crate::paths::write_atomic(&file, &serde_json::to_vec_pretty(settings)?)
            .with_context(|| format!("{} not written", file.display()))?;
        Ok(file)
    }
}

/// The folders where the harnesses keep their sessions, of every account and of the user's own
/// (`~/.claude`, `~/.codex`…), and Claude Code's `.claude.json` of each.
fn homes(settings: &crate::model::Settings) -> (Vec<Home>, Vec<PathBuf>) {
    let principals: Vec<_> = crate::engine::registry::all()
        .into_iter()
        .filter(|e| e.principal_sessions())
        .map(|e| crate::accounts::principal_of(e.id()))
        .collect();
    let mut homes = Vec::new();
    let mut files = Vec::new();
    for a in settings.accounts.iter().chain(principals.iter()) {
        // OpenCode's are in its data folder, not its configuration's (below).
        let Some(kind) = crate::engine::by_id(a.engine).session_home() else {
            continue;
        };
        homes.push(Home {
            kind,
            dir: crate::accounts::config_dir(a, settings),
        });
        if kind == HomeKind::Claude {
            files.push(crate::accounts::claude_json(a, settings));
        }
    }
    homes.push(Home {
        kind: HomeKind::Opencode,
        dir: opencode_data(),
    });
    (homes, files)
}

/// OpenCode's data folder: under `XDG_DATA_HOME` when the app has one (never in the tests), else
/// `~/.local/share/opencode`. An account of its own has its folders in Escouade's, refused whole.
fn opencode_data() -> PathBuf {
    let xdg = (!cfg!(test))
        .then(|| std::env::var_os("XDG_DATA_HOME"))
        .flatten()
        .filter(|v| !v.is_empty());
    match xdg {
        Some(v) => PathBuf::from(v).join("opencode"),
        None => crate::accounts::app_env().1.join(".local/share/opencode"),
    }
}

/// The paths of `fence` as `program` names them (its Linux ones for a harness of WSL).
pub(crate) fn spelled(fence: &Fence, program: &crate::engine::Program) -> Vec<String> {
    fence.paths().iter().map(|p| program.file_path(p)).collect()
}

/// `path`, as a harness spells it, in a rule of Claude Code's `Read(…)` and `Edit(…)`: `//`
/// then the path from the root, a Windows one in the form Claude Code matches (`C:\x` is
/// `/c/x`), its characters of a pattern taken literally.
pub(crate) fn claude_rule_path(path: &str) -> String {
    // A path of Windows (a POSIX one starts with `/`, and its `\` is a letter of a name).
    let slashed = if path.starts_with('/') {
        path.to_string()
    } else {
        path.replace('\\', "/")
    };
    let bytes = slashed.as_bytes();
    let posix = if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        format!(
            "/{}{}",
            (bytes[0] as char).to_ascii_lowercase(),
            &slashed[2..]
        )
    } else {
        slashed
    };
    let mut out = String::from("/");
    for c in posix.trim_end_matches('/').chars() {
        if matches!(c, '*' | '?' | '[' | ']' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// What Claude Code is told of `fence` in its settings: `permissions.deny` for its tools that read
/// (`Read`: Read, Grep, Glob, and the commands of its Bash that it reads the paths of) and write
/// (`Edit`: Edit, Write), and, where its sandbox runs (macOS, Linux, WSL2; not native Windows),
/// `sandbox.filesystem`: what holds for every command once the user turned it on.
pub(crate) fn claude_settings(
    fence: &Fence,
    program: &crate::engine::Program,
) -> serde_json::Value {
    let paths = spelled(fence, program);
    let deny: Vec<String> = paths
        .iter()
        .flat_map(|p| {
            let r = claude_rule_path(p);
            [
                format!("Read({r})"),
                format!("Read({r}/**)"),
                format!("Edit({r})"),
                format!("Edit({r}/**)"),
            ]
        })
        .collect();
    let mut v = serde_json::json!({ "permissions": { "deny": deny } });
    let sandboxed = !cfg!(windows) || program.wsl().is_some();
    if sandboxed {
        v["sandbox"] =
            serde_json::json!({ "filesystem": { "denyRead": paths, "denyWrite": paths } });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_code_reads_a_path_from_the_root_in_its_form() {
        assert_eq!(
            claude_rule_path(r"C:\Users\me\Secret"),
            "//c/Users/me/Secret"
        );
        assert_eq!(claude_rule_path("/home/me/secret/"), "//home/me/secret");
        // A character of a pattern, literal.
        assert_eq!(claude_rule_path("/home/me/a[1]*"), r"//home/me/a\[1\]\*");
    }

    #[test]
    fn claude_code_is_refused_reading_and_writing_each_path() {
        let path = if cfg!(windows) {
            r"C:\code\secret"
        } else {
            "/code/secret"
        };
        let rule = claude_rule_path(path);
        let program = crate::engine::Program::Native(PathBuf::from("claude"));
        let v = claude_settings(&Fence::new(vec![PathBuf::from(path)]), &program);
        let deny: Vec<&str> = v["permissions"]["deny"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap())
            .collect();
        assert_eq!(
            deny,
            [
                format!("Read({rule})"),
                format!("Read({rule}/**)"),
                format!("Edit({rule})"),
                format!("Edit({rule}/**)"),
            ]
        );
        // Its sandbox where it runs: not on native Windows, where it would be a setting it does
        // not know the paths of.
        if cfg!(windows) {
            assert!(v.get("sandbox").is_none());
        } else {
            assert_eq!(v["sandbox"]["filesystem"]["denyRead"][0], path);
            assert_eq!(v["sandbox"]["filesystem"]["denyWrite"][0], path);
        }
    }
}
