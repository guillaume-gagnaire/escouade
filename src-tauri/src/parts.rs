//! The parts of a project switched on or off (ESC-658, spec « Les parties d'un projet »), in the core: the register and
//! the order of who says it are `escouade_model::parts`; here, what a part takes with it in one project when it is off.
//!
//! - **Its MCP tools** ([`of_tool`]): gone from the list an agent of the project is given, and refused with the reason
//!   when they are called for the project all the same (by its agents or anyone naming it). Its data stay.
//! - **What its agents are told of it**: the spec's and the documents' sentences (Docs), the shared memory's (Mémoire),
//!   the launch's (`lancement` of the ticket's protocol).
//! - **What it does in the background** for the project: the planners and the comparison to the code (Docs), the
//!   autopilot, the tickets' starts, the imports and the workflows (Kanban), the probes of its launches (Lancement).
//! - Its views: the window's (`src/lib/parts.ts`).
//!
//! A part on only lets the rest go as its own switches say: it starts nothing by itself, whoever turned it on (the
//! repository's `.escouade.yml` included).

use std::sync::Arc;

use crate::core::Core;
use crate::features::Feature;
use crate::model::Project;
pub use escouade_model::parts::{on, state, Part, PartBy, PartState};

/// The shared memory's tools: of a project when they name one (the Overseer's own memory, named by none, stays).
pub const MEMORY_TOOLS: &[&str] = &[
    "search_memory",
    "read_memory",
    "write_memory",
    "forget_memory",
];

/// The part the MCP tool `tool` belongs to in a project: the Docs' and the Kanban's (with the workflows and the
/// scheduled tasks, whose runs are tickets) as their features say, and the shared memory's.
pub fn of_tool(tool: &str) -> Option<Part> {
    if MEMORY_TOOLS.contains(&tool) {
        return Some(Part::Memory);
    }
    match crate::features::of_tool(tool)? {
        Feature::Docs => Some(Part::Docs),
        Feature::Kanban | Feature::Workflows | Feature::Scheduled => Some(Part::Kanban),
        _ => None,
    }
}

/// Every tool of `part`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn tools_of(part: Part) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = crate::features::TOOLS
        .iter()
        .flat_map(|(_, tools)| tools.iter().copied())
        .chain(MEMORY_TOOLS.iter().copied())
        .filter(|t| of_tool(t) == Some(part))
        .collect();
    out.dedup();
    out
}

/// Why something of `part` is refused in the project `name`, as `s` says it is off.
pub fn refusal(part: Part, name: &str, s: &PartState) -> String {
    let fr = crate::i18n::ui() == crate::i18n::Lang::Fr;
    let part = part.name(fr);
    match s.by {
        PartBy::TeamEnforced => {
            let team = s.team.clone().unwrap_or_default();
            tr!(
                "La partie « {part} » est coupée pour le projet « {name} » par l’équipe {team} : ce qu’elle fait n’est pas disponible dans ce projet. Ses données sont gardées.",
                "The part “{part}” is turned off for the project “{name}” by the team {team}: what it does is not available in this project. Its data are kept."
            )
        }
        PartBy::Repo | PartBy::TeamDefault => tr!(
            "La partie « {part} » est coupée pour le projet « {name} », comme le propose son .escouade.yml (Réglages › Projets › Parties du projet) : ce qu’elle fait n’est pas disponible dans ce projet. L’utilisateur peut la rallumer, ses données sont gardées.",
            "The part “{part}” is turned off for the project “{name}”, as its .escouade.yml proposes (Settings › Projects › Project parts): what it does is not available in this project. The user may turn it on again, its data are kept."
        ),
        _ => tr!(
            "La partie « {part} » est coupée pour le projet « {name} » (Réglages › Projets › Parties du projet) : ce qu’elle fait n’est pas disponible dans ce projet. L’utilisateur peut la rallumer, ses données sont gardées.",
            "The part “{part}” is turned off for the project “{name}” (Settings › Projects › Project parts): what it does is not available in this project. The user may turn it on again, its data are kept."
        ),
    }
}

/// The refusal of `part` in `p` while its switch is off; None when it is on (the feature of the app is not looked at).
pub fn refusal_in(p: &Project, part: Part) -> Option<String> {
    let s = state(p, part);
    (!s.on).then(|| refusal(part, &p.name, &s))
}

impl Core {
    /// Whether `part` is on in the project `project_id`: the feature of the app it is of, and its own switch. A project
    /// the core does not have has every part.
    pub fn part_on(&self, project_id: &str, part: Part) -> bool {
        if part.feature().is_some_and(|f| !self.feature_on(f)) {
            return false;
        }
        self.projects
            .read()
            .iter()
            .find(|p| p.id == project_id)
            .is_none_or(|p| on(p, part))
    }

    /// Ok while `part` is on in the project; the refusal otherwise (the feature's first).
    pub fn part_check(&self, project_id: &str, part: Part) -> Result<(), String> {
        if let Some(f) = part.feature() {
            self.feature_check(f)?;
        }
        let p = self
            .projects
            .read()
            .iter()
            .find(|p| p.id == project_id)
            .cloned();
        match p.and_then(|p| refusal_in(&p, part)) {
            Some(why) => Err(why),
            None => Ok(()),
        }
    }

    /// The MCP tool `tool` refused in the project `project_id` while its part is off there: the reason.
    pub fn tool_refusal_in(&self, tool: &str, project_id: &str) -> Option<String> {
        let part = of_tool(tool)?;
        let p = self
            .projects
            .read()
            .iter()
            .find(|p| p.id == project_id)
            .cloned()?;
        refusal_in(&p, part)
    }

    /// « Parties du projet »: this device's choice for `part` (None: « Reprendre le défaut », the repository's or on).
    /// Refused when the team imposes it. What the part does in the background follows at once; nothing is deleted.
    pub fn set_project_part(
        self: &Arc<Self>,
        project_id: &str,
        part: Part,
        on_: Option<bool>,
    ) -> anyhow::Result<Project> {
        let (before, after) = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(crate::core::project_not_found)?;
            let s = state(p, part);
            if s.by == PartBy::TeamEnforced {
                let team = s.team.unwrap_or_default();
                anyhow::bail!(tr!(
                    "Cette partie est imposée par l’équipe {team} : elle ne se change pas sur ce poste.",
                    "This part is imposed by the team {team}: it cannot be changed on this device."
                ));
            }
            let before = p.clone();
            p.parts.set(part, on_);
            (before, p.clone())
        };
        self.request_save();
        self.hub.emit(crate::model::UiEvent::Project {
            project: after.clone(),
        });
        self.parts_changed(&before, &after);
        Ok(after)
    }

    /// What an agent of `p` is told of its shared memory and its documents (`memory::instruction_of`).
    pub(crate) fn told_of(&self, p: &Project) -> crate::memory::Told {
        crate::memory::Told {
            memory: on(p, Part::Memory),
            docs: self.part_on(&p.id, Part::Docs),
        }
    }

    /// What a ticket's agent of `p` is told of its spec and its launch (`board::protocol_prompt_told`).
    pub(crate) fn ticket_told(&self, p: &Project) -> crate::board::Told {
        crate::board::Told {
            docs: self.part_on(&p.id, Part::Docs),
            launch: on(p, Part::Launch),
        }
    }

    /// The launch of the project turned off: the test launches of its agents stop (their commands and recipes stay).
    pub(crate) fn launch_part_off(&self, project_id: &str) {
        let agents: Vec<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                (rt.meta.project_id == project_id).then(|| rt.meta.id.clone())
            })
            .collect();
        for a in agents {
            if self.test_runs.lock().contains_key(&a) {
                self.stop_test_runs(&a);
            }
        }
    }

    /// The parts of a project changed from `before` (this device's choice, its `.escouade.yml` read again, its team):
    /// what a part off does in the background stops for it, what a part on again does goes on.
    pub(crate) fn parts_changed(self: &Arc<Self>, before: &Project, after: &Project) {
        let changed: Vec<Part> = Part::ALL
            .into_iter()
            .filter(|part| on(before, *part) != on(after, *part))
            .collect();
        if changed.is_empty() {
            return;
        }
        log::info!(
            "parts of {}: {}",
            after.id,
            changed
                .iter()
                .map(|p| format!("{} {}", p.id(), if on(after, *p) { "on" } else { "off" }))
                .collect::<Vec<_>>()
                .join(", ")
        );
        for part in changed {
            match part {
                // Off, its autopilot starts nothing and its workflows are no longer followed; on, both go on.
                Part::Kanban => {
                    self.workflow_fixes_changed(&after.id);
                    if on(after, part) {
                        self.schedule();
                    }
                }
                // Off, its agents' test launches stop and their probes with them (`launch_part_off`); the commands stay.
                Part::Launch if !on(after, part) => self.launch_part_off(&after.id),
                _ => {}
            }
        }
        self.hub.emit(crate::model::UiEvent::DiscoverChanged {});
    }
}

#[cfg(test)]
#[path = "parts_tests.rs"]
mod tests;
