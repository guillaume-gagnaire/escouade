//! The features (F-1, ESC-550, `docs/FEATURES.md` § 1, spec « Fonctionnalités »): each whole feature of Escouade is
//! switched on or off, as a feature flag. This is their single register: the core reads it to stop what a feature does
//! in the background and to refuse its MCP tools, the window to take away its views, the policies to impose it
//! (`features.<id>`).
//!
//! What is essential is not here, as it never goes: the agents and the conversation, the Git panel, the terminals and
//! the launches.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A feature that may be switched off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Feature {
    Kanban,
    Docs,
    Editor,
    Cost,
    Stats,
    Workflows,
    Scheduled,
    Overseer,
    Voice,
}

impl Feature {
    /// Every feature, in the order of Réglages › Fonctionnalités.
    pub const ALL: [Feature; 9] = [
        Feature::Kanban,
        Feature::Editor,
        Feature::Docs,
        Feature::Cost,
        Feature::Stats,
        Feature::Workflows,
        Feature::Scheduled,
        Feature::Overseer,
        Feature::Voice,
    ];

    /// Its id: in the settings (`features.<id>`), the window and the policies.
    pub fn id(self) -> &'static str {
        match self {
            Feature::Kanban => "kanban",
            Feature::Docs => "docs",
            Feature::Editor => "editor",
            Feature::Cost => "cost",
            Feature::Stats => "stats",
            Feature::Workflows => "workflows",
            Feature::Scheduled => "scheduled",
            Feature::Overseer => "overseer",
            Feature::Voice => "voice",
        }
    }

    /// The feature of the id `id`.
    pub fn of(id: &str) -> Option<Feature> {
        Feature::ALL.into_iter().find(|f| f.id() == id)
    }

    /// The policy that imposes it (`features.kanban`), as `escouade_policy::list` names it.
    pub fn policy(self) -> &'static str {
        match self {
            Feature::Kanban => "features.kanban",
            Feature::Docs => "features.docs",
            Feature::Editor => "features.editor",
            Feature::Cost => "features.cost",
            Feature::Stats => "features.stats",
            Feature::Workflows => "features.workflows",
            Feature::Scheduled => "features.scheduled",
            Feature::Overseer => "features.overseer",
            Feature::Voice => "features.voice",
        }
    }

    /// The feature it cannot go without: a scheduled task's run is a ticket, which the Kanban holds.
    pub fn needs(self) -> Option<Feature> {
        match self {
            Feature::Scheduled => Some(Feature::Kanban),
            _ => None,
        }
    }

    /// Its name, as Réglages › Fonctionnalités and the refusals say it.
    pub fn name(self, fr: bool) -> &'static str {
        let (f, e) = match self {
            Feature::Kanban => ("Kanban", "Kanban"),
            Feature::Docs => ("Espace Docs", "Docs space"),
            Feature::Editor => ("Éditeur", "Editor"),
            Feature::Cost => ("Coût", "Cost"),
            Feature::Stats => ("Statistiques", "Statistics"),
            Feature::Workflows => ("Workflows", "Workflows"),
            Feature::Scheduled => ("Tâches programmées", "Scheduled tasks"),
            Feature::Overseer => ("Overseer", "Overseer"),
            Feature::Voice => ("Voix", "Voice"),
        };
        if fr {
            f
        } else {
            e
        }
    }
}

/// The switches of the features (`Settings::features`): on unless switched off. A new installation has them all on,
/// until the assistant of the first start chooses (F-2); an installation updated to the version that brings them has
/// what it already uses on (`Core::settle_features`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Features {
    pub kanban: bool,
    pub docs: bool,
    pub editor: bool,
    pub cost: bool,
    pub stats: bool,
    pub workflows: bool,
    pub scheduled: bool,
    pub overseer: bool,
    pub voice: bool,
}

impl Default for Features {
    fn default() -> Self {
        Features::all(true)
    }
}

impl Features {
    /// Every switch at `on`.
    pub fn all(on: bool) -> Self {
        Features {
            kanban: on,
            docs: on,
            editor: on,
            cost: on,
            stats: on,
            workflows: on,
            scheduled: on,
            overseer: on,
            voice: on,
        }
    }

    /// The switch of `f` (its own, whatever it needs).
    pub fn get(&self, f: Feature) -> bool {
        *self.slot(f)
    }

    pub fn set(&mut self, f: Feature, on: bool) {
        *self.slot_mut(f) = on;
    }

    /// Whether `f` is on: its switch, and the one of the feature it needs.
    pub fn on(&self, f: Feature) -> bool {
        self.get(f) && f.needs().is_none_or(|n| self.get(n))
    }

    fn slot(&self, f: Feature) -> &bool {
        match f {
            Feature::Kanban => &self.kanban,
            Feature::Docs => &self.docs,
            Feature::Editor => &self.editor,
            Feature::Cost => &self.cost,
            Feature::Stats => &self.stats,
            Feature::Workflows => &self.workflows,
            Feature::Scheduled => &self.scheduled,
            Feature::Overseer => &self.overseer,
            Feature::Voice => &self.voice,
        }
    }

    fn slot_mut(&mut self, f: Feature) -> &mut bool {
        match f {
            Feature::Kanban => &mut self.kanban,
            Feature::Docs => &mut self.docs,
            Feature::Editor => &mut self.editor,
            Feature::Cost => &mut self.cost,
            Feature::Stats => &mut self.stats,
            Feature::Workflows => &mut self.workflows,
            Feature::Scheduled => &mut self.scheduled,
            Feature::Overseer => &mut self.overseer,
            Feature::Voice => &mut self.voice,
        }
    }
}

/// A feature the installation already used when it was updated to the version that brings the switches (F-1): kept
/// on, and said so by the assistant (F-2): « Tu l'utilises déjà sur 2 projets : il reste allumé ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureUse {
    pub feature: Feature,
    /// On how many projects it is used; 0 when it is not used by project (the Overseer, the voice, the statistics).
    #[serde(default)]
    pub projects: u32,
}

/// What a new installation has, until the assistant of the first start (F-2) chooses: every feature on, so that
/// nothing is hidden from whoever never sees the assistant (a device whose journey is passed, a client of a server).
pub const NEW_INSTALL: bool = true;

/// The switches of an installation updated to the version that brings them: what it uses on, the rest as a new
/// installation has it (`fresh`, `NEW_INSTALL` in the app).
pub fn settle(used: &[FeatureUse], fresh: bool) -> Features {
    let mut f = Features::all(fresh);
    for u in used {
        f.set(u.feature, true);
    }
    f
}

/// What the proposals at the right moment (F-3, ESC-558, `docs/FEATURES.md` § 2) keep, by installation, in `state.json`
/// beside the discovery: what Escouade noticed of a feature switched off (the moments of each signal, kept two weeks;
/// never a text, a message is kept as its fingerprint alone) and what the user answered. Nothing of it leaves the
/// machine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeatureOffers {
    /// Kanban off: when an agent of the user's started on a long first message (ms).
    pub long_prompts: Vec<i64>,
    /// Kanban off: when a call of `create_ticket` came to the MCP server, refused.
    pub mcp_tickets: Vec<i64>,
    /// Espace Docs off: when a document was dropped in a conversation.
    pub documents: Vec<i64>,
    /// Coût off: when an agent met a usage limit.
    pub limits: Vec<i64>,
    /// Overseer off: when agents waited for an answer on several projects at once, and on how many.
    pub waiting: Vec<WaitingSeen>,
    /// Tâches programmées off: the messages sent by hand, by their fingerprint, and when.
    pub messages: Vec<MessageSeen>,
    /// « Pas maintenant », feature by feature, and when: not the same one again for a week.
    pub later: BTreeMap<Feature, i64>,
    /// « Ne plus proposer »: never again, until Réglages › Fonctionnalités › « Propositions écartées » gives it back.
    pub never: BTreeSet<Feature>,
    /// When the last proposal was shown (ms): one a day at most.
    pub last_shown: Option<i64>,
}

/// Agents waiting for an answer on `projects` projects at once, seen at `at` (ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingSeen {
    pub at: i64,
    pub projects: u32,
}

/// A message sent by hand: its fingerprint (a hash of its words, never the text) and when (ms).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSeen {
    pub hash: String,
    pub at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_feature_has_its_own_id_and_policy() {
        let ids: std::collections::BTreeSet<_> = Feature::ALL.iter().map(|f| f.id()).collect();
        assert_eq!(ids.len(), Feature::ALL.len());
        for f in Feature::ALL {
            assert_eq!(Feature::of(f.id()), Some(f));
            assert_eq!(f.policy(), format!("features.{}", f.id()));
            // Its serde name is its id: the settings, the window and the policies say the same.
            assert_eq!(serde_json::to_value(f).unwrap(), f.id());
        }
    }

    #[test]
    fn a_switch_goes_alone_but_scheduled_tasks_go_with_the_kanban() {
        let mut s = Features::default();
        for f in Feature::ALL {
            assert!(s.on(f));
        }
        s.set(Feature::Kanban, false);
        assert!(!s.on(Feature::Kanban));
        assert!(s.get(Feature::Scheduled));
        assert!(!s.on(Feature::Scheduled), "its runs are tickets");
        s.set(Feature::Kanban, true);
        s.set(Feature::Docs, false);
        let off: Vec<_> = Feature::ALL.into_iter().filter(|f| !s.on(*f)).collect();
        assert_eq!(off, vec![Feature::Docs]);
    }

    #[test]
    fn settings_saved_with_some_switches_keep_the_others_on() {
        let s: Features = serde_json::from_str(r#"{"kanban":false}"#).unwrap();
        assert!(!s.kanban);
        assert!(s.docs && s.voice);
    }
}
