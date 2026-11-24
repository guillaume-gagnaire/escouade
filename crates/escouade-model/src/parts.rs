//! The parts of a project that may be switched off (ESC-658, spec « Les parties d'un projet »): the Docs, the shared
//! memory, the Kanban, the launch and the browsers. Where a feature (`features`) goes for the whole app, a part goes
//! for one project: its views, its MCP tools, what its agents are told of it and what it does in the background, never
//! its data (its files, its tickets, its memories stay, and come back when it is on again).
//!
//! Who says whether a part is on, the first who says it winning:
//!
//! 1. the project's team, when it imposes it (`enforced` of the team's `.escouade.yml`, rank 5);
//! 2. this device (Réglages › Projets › Parties du projet): [`PartChoices`], saved with the project, never in a file;
//! 3. the repository (`parts.<id>` of its `.escouade.yml`): a default, « proposé par le dépôt », while the device chose
//!    nothing;
//! 4. the team, when it recommends it (`defaults`, rank 2);
//! 5. on: a project where nobody says anything has every part, as before.
//!
//! A part only takes away: on, it starts nothing that its own switches do not start (the autopilot, a planner, a
//! comparison to the code each keep theirs), so a repository that says `true` runs nothing more than what the user
//! already allowed, and one that says `false` lifts no protection (the trust of its commands, the privacy, the
//! permissions and the policies stay as they are).

use serde::{Deserialize, Serialize};

use crate::features::Feature;
use crate::model::Project;

/// A part of a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    Docs,
    Memory,
    Kanban,
    Launch,
}

impl Part {
    /// Every part, in the order of the page.
    pub const ALL: [Part; 4] = [Part::Docs, Part::Memory, Part::Kanban, Part::Launch];

    /// Its id: in the window, the `.escouade.yml` (`parts.<id>`) and the refusals.
    pub fn id(self) -> &'static str {
        match self {
            Part::Docs => "docs",
            Part::Memory => "memory",
            Part::Kanban => "kanban",
            Part::Launch => "launch",
        }
    }

    /// The part of the id `id`.
    pub fn of(id: &str) -> Option<Part> {
        Part::ALL.into_iter().find(|p| p.id() == id)
    }

    /// Its key in a `.escouade.yml`, the repository's and the team's: `parts.docs`.
    pub fn key(self) -> &'static str {
        match self {
            Part::Docs => "parts.docs",
            Part::Memory => "parts.memory",
            Part::Kanban => "parts.kanban",
            Part::Launch => "parts.launch",
        }
    }

    /// The feature of the whole app it is a part of: off there, it is off in every project, whatever its switch.
    pub fn feature(self) -> Option<Feature> {
        match self {
            Part::Docs => Some(Feature::Docs),
            Part::Kanban => Some(Feature::Kanban),
            Part::Memory | Part::Launch => None,
        }
    }

    /// Its name, as the page and the refusals say it.
    pub fn name(self, fr: bool) -> &'static str {
        let (f, e) = match self {
            Part::Docs => ("Docs", "Docs"),
            Part::Memory => ("Mémoire partagée", "Shared memory"),
            Part::Kanban => ("Kanban", "Kanban"),
            Part::Launch => ("Lancement et navigateurs", "Launch and browsers"),
        };
        if fr {
            f
        } else {
            e
        }
    }
}

/// What this device chose for the parts of a project (Réglages › Projets › Parties du projet): None while it chose
/// nothing, and the repository's default (`parts.<id>`) or on holds then. Of this device: never in `.escouade.yml`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PartChoices {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kanban: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch: Option<bool>,
}

impl PartChoices {
    /// Nothing chosen.
    pub fn is_empty(&self) -> bool {
        *self == PartChoices::default()
    }

    pub fn get(&self, part: Part) -> Option<bool> {
        *self.slot(part)
    }

    /// `on` chosen for `part`; None forgets the choice (« Reprendre le défaut »).
    pub fn set(&mut self, part: Part, on: Option<bool>) {
        *self.slot_mut(part) = on;
    }

    fn slot(&self, part: Part) -> &Option<bool> {
        match part {
            Part::Docs => &self.docs,
            Part::Memory => &self.memory,
            Part::Kanban => &self.kanban,
            Part::Launch => &self.launch,
        }
    }

    fn slot_mut(&mut self, part: Part) -> &mut Option<bool> {
        match part {
            Part::Docs => &mut self.docs,
            Part::Memory => &mut self.memory,
            Part::Kanban => &mut self.kanban,
            Part::Launch => &mut self.launch,
        }
    }
}

/// Who says whether a part is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PartBy {
    /// Nobody: on.
    Default,
    /// The team recommends it (rank 2).
    TeamDefault,
    /// The repository's `.escouade.yml` proposes it.
    Repo,
    /// This device chose it.
    Device,
    /// The team imposes it (rank 5): it cannot be changed here.
    TeamEnforced,
}

/// Whether a part is on in a project, and who says so.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartState {
    pub part: Part,
    pub on: bool,
    pub by: PartBy,
    /// The team's name, when it is the team that says it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
}

/// The value a `.escouade.yml`'s map gives `part`, when it is a boolean.
fn said(
    values: &std::collections::BTreeMap<String, serde_json::Value>,
    part: Part,
) -> Option<bool> {
    values.get(part.key()).and_then(serde_json::Value::as_bool)
}

/// Whether `part` is on in `p` (its own switch, whatever the feature of the app says), and who says so: the team that
/// imposes it, this device, the repository, the team that recommends it, else on.
pub fn state(p: &Project, part: Part) -> PartState {
    let team = p.shared.team_parts.as_ref();
    let at = |on: bool, by: PartBy| PartState {
        part,
        on,
        by,
        team: matches!(by, PartBy::TeamDefault | PartBy::TeamEnforced)
            .then(|| team.map(|t| t.team.clone()).unwrap_or_default()),
    };
    if let Some(on) = team.and_then(|t| said(&t.enforced, part)) {
        return at(on, PartBy::TeamEnforced);
    }
    if let Some(on) = p.parts.get(part) {
        return at(on, PartBy::Device);
    }
    if let Some(on) = p.shared.file.as_ref().and_then(|f| said(&f.values, part)) {
        return at(on, PartBy::Repo);
    }
    if let Some(on) = team.and_then(|t| said(&t.defaults, part)) {
        return at(on, PartBy::TeamDefault);
    }
    at(true, PartBy::Default)
}

/// Whether `part` is on in `p` (its own switch alone: the core adds the feature of the app).
pub fn on(p: &Project, part: Part) -> bool {
    state(p, part).on
}

/// The state of every part of `p`, in their order.
pub fn states(p: &Project) -> Vec<PartState> {
    Part::ALL.into_iter().map(|part| state(p, part)).collect()
}

impl Project {
    /// Whether its workflows are read and followed: « Workflows du projet » (ESC-309) and its Kanban (ESC-658), which
    /// they are of. Off, their files stay as they are.
    pub fn workflows_on(&self) -> bool {
        self.workflows_enabled && on(self, Part::Kanban)
    }
}

#[cfg(test)]
#[path = "parts_tests.rs"]
mod tests;
