//! A harness project by project (ESC-404): the switch of its plugin in a project (`plugins::active_in`) holds for its
//! agents too. The module of a harness receives all its agents do: off for a project, none of its agents there starts,
//! resumes or is sent a message, and the menus « Harness » of the project do not offer it. A private project has every
//! plugin off until the user turns it on (§3.3): its first agent on a harness asks the user once, in the window
//! (« Activer » turns the plugin on for it, `enable_harness_in`). No tool of the MCP server turns one on, nor does the
//! Kanban: a ticket there is blocked, with why. A harness of a third party's plugin (not `dev.escouade.*`, described or
//! served) is never launched in a private project (SEC-02, ESC-590), nor in any project while a private project is on
//! the machine (SEC-02b, ESC-623): nothing proves it holds the fence that keeps the private project's folders from it,
//! whatever the user turns on; one already running stops, at rest at once, at work at the end of its turn.

use crate::core::Core;
use crate::model::EngineId;

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Where a harness stands in a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InProject {
    /// Its plugin is on there (set on, or a project that is not private), or it goes through none.
    On,
    /// The user turned its plugin off for the project: not offered, none of its agents runs there.
    Off,
    /// A private project the user has not turned it on for yet: offered, its first agent asks first.
    Asks,
    /// A harness of a third party's plugin while a private project is on the machine: launched in no project, whatever
    /// its switch (SEC-02, ESC-590; SEC-02b, ESC-623). The core could only find the fence's paths as words of its
    /// launch, which a variable no one reads satisfies (audit test W-04), and that fence is also what keeps an agent of
    /// another project out of the private project's folders.
    ThirdParty,
}

/// The harnesses of a project that are not on (`InProject`), as the window reads them: `off` not offered, `asks`
/// offered once the user agrees, `third_party` not offered (a third party's while a private project exists, SEC-02).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessScope {
    pub off: Vec<EngineId>,
    pub asks: Vec<EngineId>,
    pub third_party: Vec<EngineId>,
}

/// Why a harness of a third party's plugin, `name`, is launched in no project while a private project is on the machine
/// (SEC-02, ESC-590; SEC-02b, ESC-623): the limit, and what the user may do.
pub fn third_party_refusal(name: &str) -> String {
    third_party_refusal_in(crate::i18n::ui(), name)
}

/// `third_party_refusal` in `lang`.
pub fn third_party_refusal_in(lang: crate::i18n::Lang, name: &str) -> String {
    tr_in!(
        lang,
        "Un projet privé existe sur ce poste : un harness d’un plugin tiers ne peut pas garantir sa clôture, {name} n’est lancé dans aucun projet. Prends un harness d’Escouade, ou retire « Projet privé » de tes projets.",
        "A private project exists on this machine: a harness from a third-party plugin can’t guarantee its fence, so {name} isn’t launched in any project. Pick one of Escouade’s harnesses, or turn “Private project” off for your projects.",
        name = name
    )
}

/// The file whose presence says the private projects had their harnesses turned on once (`migrate_private_harnesses`).
const MIGRATED: &str = "harness-projects-migrated";

impl Core {
    /// Where `engine` stands in the project `project_id` (`InProject`). A plugin not installed or off for the app is
    /// `engine_plugged`'s to say: here, the project's switch alone.
    pub fn harness_in(&self, engine: EngineId, project_id: &str) -> InProject {
        let Some(plugin) = crate::plugins::harness::plugin_of(engine) else {
            return InProject::On;
        };
        let (private, any_private) = {
            let projects = self.projects.read();
            (
                projects
                    .iter()
                    .find(|p| p.id == project_id)
                    .is_some_and(|p| p.private),
                projects.iter().any(|p| p.private),
            )
        };
        // A third party's: in no project while a private one is on the machine, whatever its switch (SEC-02, SEC-02b).
        if any_private && !crate::plugins::builtin::first_hand(&plugin) {
            return InProject::ThirdParty;
        }
        let Some(choices) = self.plugins.choices(&plugin) else {
            return InProject::On;
        };
        match choices.projects.get(project_id) {
            Some(true) => InProject::On,
            Some(false) => InProject::Off,
            None if private => InProject::Asks,
            None => InProject::On,
        }
    }

    /// Whether `engine`'s plugin is on for the project.
    pub fn harness_on_in(&self, engine: EngineId, project_id: &str) -> bool {
        self.harness_in(engine, project_id) == InProject::On
    }

    /// Why an agent of `engine` does not run in the project `project_id`, None when it may.
    pub fn harness_refusal(&self, engine: EngineId, project_id: &str) -> Option<String> {
        let name = crate::engine::registry::find(engine)
            .map(|e| e.name().to_string())
            .unwrap_or_else(|| engine.as_str().to_string());
        match self.harness_in(engine, project_id) {
            InProject::On => None,
            InProject::Off => Some(tr!(
                "{name} est coupé pour ce projet : son plugin n’en reçoit rien. Active-le pour ce projet (« Activer », ou Réglages › Plugins).",
                "{name} is off for this project: its plugin gets nothing of it. Turn it on for this project (“Turn on”, or Settings › Plugins).",
                name = name
            )),
            InProject::Asks => Some(tr!(
                "{name} n’est pas activé pour ce projet privé : son plugin n’en reçoit rien tant que tu ne l’y actives pas (« Activer »).",
                "{name} is not turned on for this private project: its plugin gets nothing of it until you turn it on there (“Turn on”).",
                name = name
            )),
            InProject::ThirdParty => Some(third_party_refusal(&name)),
        }
    }

    /// `harness_refusal` as an error.
    pub(crate) fn check_harness_in(&self, engine: EngineId, project_id: &str) -> Result<()> {
        match self.harness_refusal(engine, project_id) {
            Some(why) => Err(anyhow!(why)),
            None => Ok(()),
        }
    }

    /// The harnesses of the registry that are not on, project by project (those with none left out): what the menus
    /// « Harness » of a project leave out (`off`) or offer behind the question (`asks`), and the agents and tickets
    /// whose card says so.
    pub fn harness_scope(&self) -> BTreeMap<String, HarnessScope> {
        let engines: Vec<EngineId> = crate::engine::registry::all()
            .into_iter()
            .map(|e| e.id())
            .collect();
        let projects: Vec<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let mut out = BTreeMap::new();
        for p in projects {
            let mut scope = HarnessScope::default();
            for &e in &engines {
                match self.harness_in(e, &p) {
                    InProject::On => {}
                    InProject::Off => scope.off.push(e),
                    InProject::Asks => scope.asks.push(e),
                    InProject::ThirdParty => scope.third_party.push(e),
                }
            }
            if scope != HarnessScope::default() {
                out.insert(p, scope);
            }
        }
        out
    }

    /// The harnesses that can be used (`engines_usable`, ESC-391) and are on for the project: those the core picks by
    /// itself for it (the texts' writer). The window's menus add a private project's not turned on yet, behind the
    /// question (`harness_scope`).
    pub fn engines_usable_in(&self, project_id: &str) -> Vec<EngineId> {
        self.engines_usable()
            .into_iter()
            .filter(|&e| self.harness_on_in(e, project_id))
            .collect()
    }

    /// « Activer » (the user's gesture, never a tool's): the plugin of `engine` turned on for the project, its agents
    /// free to run there.
    pub fn enable_harness_in(&self, project_id: &str, engine: EngineId) -> Result<()> {
        self.project(project_id)?;
        let plugin = crate::plugins::harness::plugin_of(engine)
            .ok_or_else(|| anyhow!("{engine}: no plugin to turn on"))?;
        // Turned on or not, a third party's harness runs nowhere while a private project exists (SEC-02): « Activer »
        // says why.
        if self.harness_in(engine, project_id) == InProject::ThirdParty {
            return Err(anyhow!(self
                .harness_refusal(engine, project_id)
                .unwrap_or_default()));
        }
        self.plugins
            .choose(&plugin, |c| {
                c.projects.insert(project_id.to_string(), true);
            })
            .map_err(|e| anyhow!(e))?;
        log::info!(
            "plugin {plugin}: turned on for the project {project_id} (its harness {engine})"
        );
        Ok(())
    }

    /// A plugin's switch in a project set by the user (Réglages › Plugins). Off, its harnesses' agents of the project
    /// stop, as turning the plugin off for the app stops them all.
    pub async fn set_plugin_project(
        self: &Arc<Self>,
        plugin: &str,
        project_id: &str,
        on: bool,
    ) -> Result<()> {
        self.plugins
            .choose(plugin, |c| {
                c.projects.insert(project_id.to_string(), on);
            })
            .map_err(|e| anyhow!(e))?;
        if !on {
            self.stop_harness_agents(plugin, project_id).await;
        }
        Ok(())
    }

    /// The project `project_id` made private (ESC-582): what runs of each of its agents whose harness is not on there
    /// (its process, its interface) stops as a start would be refused, said in its conversation; at rest at once, at
    /// work at the end of its turn (`settle_locked_mode`, through `run_refusal`), as ESC-497 settles a mode no longer
    /// allowed. Its turn is not cut: the user did not cut the harness, only made the project private.
    pub(crate) fn settle_harnesses_in(self: &Arc<Self>, project_id: &str) {
        let running: Vec<(String, EngineId)> = self
            .project_agents(project_id)
            .iter()
            .filter_map(|h| {
                let rt = h.lock();
                (!rt.meta.archived
                    && !rt.meta.status.is_active()
                    && (rt.proc.is_some() || rt.tui.is_some()))
                .then(|| (rt.meta.id.clone(), rt.meta.engine))
            })
            .collect();
        for (id, engine) in running {
            if let Some(why) = self.harness_refusal(engine, project_id) {
                self.stop_out_of_rule(&id, &crate::modes::stopped(crate::i18n::ui(), &why));
            }
        }
    }

    /// A project made private, or a private one added (SEC-02, ESC-590; SEC-02b, ESC-623): what runs of each agent of
    /// every project on a third party's harness (its process, its interface) stops, said in its conversation; at rest at
    /// once, at work at the end of its turn (`settle_locked_mode`, through `run_refusal`), as ESC-497 settles a mode no
    /// longer allowed. Its turn is not cut: the user did not cut the harness, only made a project private. When no
    /// private project is left, nothing starts again by itself.
    pub(crate) fn settle_third_party(self: &Arc<Self>) {
        let running: Vec<(String, EngineId, String)> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                (!rt.meta.archived
                    && !rt.meta.status.is_active()
                    && (rt.proc.is_some() || rt.tui.is_some()))
                .then(|| {
                    (
                        rt.meta.id.clone(),
                        rt.meta.engine,
                        rt.meta.project_id.clone(),
                    )
                })
            })
            .collect();
        for (id, engine, project_id) in running {
            if self.harness_in(engine, &project_id) != InProject::ThirdParty {
                continue;
            }
            if let Some(why) = self.harness_refusal(engine, &project_id) {
                self.stop_out_of_rule(&id, &crate::modes::stopped(crate::i18n::ui(), &why));
            }
        }
    }

    /// The agents of the project on a harness of `plugin` stop: their turn interrupted, their process and their
    /// interface closed; a ticket's is blocked, with why.
    pub(crate) async fn stop_harness_agents(self: &Arc<Self>, plugin: &str, project_id: &str) {
        let ids: Vec<(String, EngineId)> = self
            .project_agents(project_id)
            .iter()
            .filter_map(|h| {
                let rt = h.lock();
                let m = &rt.meta;
                (!m.archived
                    && crate::plugins::harness::plugin_of(m.engine).as_deref() == Some(plugin))
                .then(|| (m.id.clone(), m.engine))
            })
            .collect();
        for (id, engine) in ids {
            let active = self
                .agent(&id)
                .is_ok_and(|h| h.lock().meta.status.is_active());
            if active {
                if let Err(e) = self.interrupt(&id).await {
                    log::warn!("agent {id}: its turn not stopped as its harness went off: {e:#}");
                }
            }
            let lock = self.spawn_lock(&id);
            let _guard = lock.lock().await;
            let mut stopped = None;
            let mut interface = None;
            let _ = self.with_agent(&id, |rt, fx| {
                stopped = rt.detach();
                if let Some(t) = rt.tui.as_ref().map(|t| t.term_id.clone()) {
                    rt.gen += 1;
                    rt.stop_tui(fx);
                    interface = Some(t);
                }
                fx.save = true;
                Ok(())
            });
            if let Some(p) = stopped {
                p.close_input();
                p.kill();
            }
            if let Some(t) = interface {
                let pty = self.pty.clone();
                let _ = tokio::task::spawn_blocking(move || pty.kill(&t)).await;
            }
            if let Some(ticket) = self.doing_ticket_of(&id) {
                if let Some(why) = self.harness_refusal(engine, project_id) {
                    let _ = self.edit_ticket(&ticket, |t| {
                        t.blocked = Some(why);
                        Ok(())
                    });
                    self.notify_ticket(&ticket, false);
                }
            }
            log::info!("agent {id}: stopped, its harness {engine} off for its project");
        }
    }

    /// Once, at the update that applies the switch to the harnesses: a private project with agents (not archived) on a
    /// harness has its plugin turned on for it, where the user had set nothing. The user used it there already, and
    /// the project must not stop all at once. The others stay off by default, and so does a plugin of a third party
    /// (not `dev.escouade.*`): the user turns it on there (SEC-08).
    pub(crate) fn migrate_private_harnesses(&self) {
        let marker = self.data.root().join(MIGRATED);
        if marker.exists() {
            return;
        }
        let private: Vec<String> = self
            .projects
            .read()
            .iter()
            .filter(|p| p.private)
            .map(|p| p.id.clone())
            .collect();
        let mut wanted: Vec<(String, String)> = Vec::new();
        for h in self.agents.read().values() {
            let rt = h.lock();
            let m = &rt.meta;
            if m.archived || !private.contains(&m.project_id) {
                continue;
            }
            // A plugin of a third party is never turned on for a private project but by the user (SEC-08).
            if let Some(plugin) = crate::plugins::harness::plugin_of(m.engine)
                .filter(|p| crate::plugins::builtin::first_hand(p))
            {
                let pair = (plugin, m.project_id.clone());
                if !wanted.contains(&pair) {
                    wanted.push(pair);
                }
            }
        }
        for (plugin, project) in wanted {
            let unset = self
                .plugins
                .choices(&plugin)
                .is_some_and(|c| !c.projects.contains_key(&project));
            if !unset {
                continue;
            }
            if let Err(e) = self.plugins.choose(&plugin, |c| {
                c.projects.insert(project.clone(), true);
            }) {
                log::warn!("plugin {plugin}: not turned on for the private project {project}: {e}");
                return;
            }
            log::info!(
                "plugin {plugin}: turned on for the private project {project}, which has agents on it"
            );
        }
        if let Err(e) = std::fs::write(&marker, b"") {
            log::warn!("{}: not written: {e}", marker.display());
        }
    }
}
