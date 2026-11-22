//! The features switched on or off (F-1, ESC-550, `docs/FEATURES.md` § 1, spec « Fonctionnalités »), in the core: the
//! register is `escouade_model::features` (the switches live in the settings, the policies may impose them through
//! `features.<id>`); here, what a feature takes with it when it is off.
//!
//! - **Its MCP tools** ([`TOOLS`]): absent from the list the server gives, and refused with the reason if called.
//! - **What it does in the background** stops: the autopilot and the ticket sources' import (Kanban), the scheduled
//!   tasks' runs, the Overseer, the reading of the workflows, the language servers (Éditeur) and the dictation (Voix)
//!   are refused at their start; what runs stops when the switch goes off (`Core::features_changed`).
//! - **Its discoveries** of « À découvrir » ([`discoveries`]).
//! - Its views, menus, shortcuts and place in the palette: the window's (`src/lib/features.ts`).
//!
//! Its data stay (tickets, pages, scheduled tasks, the Overseer's journal…) and come back when it is on again.
//!
//! The switches are settled once, at the first start of the version that brings them (`Core::settle_features`): what
//! the installation already uses is kept on (and noted, for the assistant), the rest is as a new installation has it
//! (`escouade_model::features::NEW_INSTALL`: off, the assistant's choice).
//!
//! **The assistant** (F-2, ESC-559, `docs/FEATURES.md` § 1): one feature per screen, a capture of the real app, what it
//! does in three points and a big switch « Activer ». It is a station of the first launch's journey
//! (`OnboardingStep::Features`), it opens by itself at the first window of the version that brings the switches on an
//! installation updated to it ([`Core::features_tour_view`]), and « Revoir la présentation » of Réglages ›
//! Fonctionnalités opens it again. Its switches are the settings' own (the window saves them as the page does): « Passer »
//! changes none, so it keeps the essential and what is used. A feature a later version brings has its screen in the
//! window of the novelties (`TourView::new`).

use serde::{Deserialize, Serialize};

use crate::core::Core;
use crate::model::{Discovery, Settings};
pub use escouade_model::features::{Feature, FeatureUse, Features};
use std::sync::Arc;

/// The MCP tools of each feature: what goes from the list when it is off. A tool of none is of the essential (the
/// projects, the agents, the box « À toi », the shared memory, the pull requests…).
pub const TOOLS: &[(Feature, &[&str])] = &[
    (
        Feature::Kanban,
        &[
            "get_board",
            "set_board",
            "list_tickets",
            "get_ticket",
            "get_ticket_file",
            "get_ticket_image",
            "create_ticket",
            "update_ticket",
            "move_ticket",
            "delete_ticket",
            "start_ticket",
            "approve_ticket",
            "reject_ticket",
            "resume_ticket",
            "resolve_conflict",
            "dismiss_block",
            "wait_ticket",
            "split_ticket",
        ],
    ),
    (
        Feature::Docs,
        &[
            "get_spec_tree",
            "read_spec_page",
            "write_spec_page",
            "search_spec",
            "list_decisions",
            "read_decision",
            "propose_decision",
            "replace_decision",
            "list_documents",
            "read_document",
            "search_documents",
            "add_document",
            "get_docs_to_share",
            "share_docs",
        ],
    ),
    (Feature::Workflows, &["get_workflow"]),
    (
        Feature::Scheduled,
        &[
            "list_scheduled_tasks",
            "create_scheduled_task",
            "update_scheduled_task",
            "pause_scheduled_task",
            "delete_scheduled_task",
        ],
    ),
    (
        Feature::Overseer,
        &[
            "add_directive",
            "list_directives",
            "revoke_directive",
            "undo_action",
            "leave_to_user",
        ],
    ),
];

/// The feature the MCP tool `tool` belongs to; None for a tool of the essential.
pub fn of_tool(tool: &str) -> Option<Feature> {
    TOOLS
        .iter()
        .find(|(_, tools)| tools.contains(&tool))
        .map(|(f, _)| *f)
}

/// The discoveries of « À découvrir » each feature takes with it: gone from the map while it is off.
pub fn discoveries(f: Feature) -> &'static [Discovery] {
    match f {
        Feature::Kanban => &[
            Discovery::Ticket,
            Discovery::Autopilot,
            Discovery::Approve,
            Discovery::Sources,
        ],
        Feature::Docs => &[Discovery::Docs],
        Feature::Editor => &[Discovery::Editor],
        Feature::Voice => &[Discovery::Dictation, Discovery::Voice],
        _ => &[],
    }
}

/// The discoveries of the features `s` has off.
pub fn discoveries_off(s: &Settings) -> Vec<Discovery> {
    Feature::ALL
        .into_iter()
        .filter(|f| !s.feature(*f))
        .flat_map(|f| discoveries(f).iter().copied())
        .collect()
}

/// Why something of `f` is refused while it is off: « La fonctionnalité Kanban est coupée (Réglages ›
/// Fonctionnalités) ». `by`: who imposes it off (« par la stratégie de ce poste »), when someone does.
pub fn refusal(f: Feature, by: Option<String>) -> String {
    let fr = crate::i18n::ui() == crate::i18n::Lang::Fr;
    let name = f.name(fr);
    let needs = f.needs().map(|n| n.name(fr)).unwrap_or_default();
    match by {
        Some(by) => tr!(
            "La fonctionnalité « {name} » est coupée {by} : ce qu’elle fait n’est pas disponible sur ce poste.",
            "The feature “{name}” is turned off {by}: what it does is not available on this device."
        ),
        None if !needs.is_empty() => tr!(
            "La fonctionnalité « {name} » est coupée (Réglages › Fonctionnalités ; elle part aussi avec « {needs} ») : ce qu’elle fait n’est pas disponible. L’utilisateur peut la rallumer, ses données sont gardées.",
            "The feature “{name}” is turned off (Settings › Features; it also goes with “{needs}”): what it does is not available. The user may turn it on again, its data are kept."
        ),
        None => tr!(
            "La fonctionnalité « {name} » est coupée (Réglages › Fonctionnalités) : ce qu’elle fait n’est pas disponible. L’utilisateur peut la rallumer, ses données sont gardées.",
            "The feature “{name}” is turned off (Settings › Features): what it does is not available. The user may turn it on again, its data are kept."
        ),
    }
}

/// Whether the directory `dir` holds anything.
fn holds(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut d| d.next().is_some())
}

impl Core {
    /// Whether the feature `f` is on (and the one it needs).
    pub fn feature_on(&self, f: Feature) -> bool {
        self.settings.read().feature(f)
    }

    /// Ok while `f` is on; its refusal (`refusal`) otherwise, with who imposes it off.
    pub fn feature_check(&self, f: Feature) -> Result<(), String> {
        if self.feature_on(f) {
            return Ok(());
        }
        // The feature it needs may be the one imposed off.
        let off = if self
            .settings
            .read()
            .features
            .as_ref()
            .is_some_and(|s| s.get(f))
        {
            f.needs().unwrap_or(f)
        } else {
            f
        };
        let by = self
            .locks
            .read()
            .get(off.policy())
            .and_then(|l| l.by.as_ref())
            .map(crate::locks::LockedBy::by);
        Err(refusal(f, by))
    }

    /// The MCP tool `tool` refused while its feature is off: the reason.
    pub fn tool_refusal(&self, tool: &str) -> Option<String> {
        self.feature_check(of_tool(tool)?).err()
    }

    /// What the installation already uses, feature by feature (the reprise of an update, F-1): projects with tickets,
    /// a spec or documents, the editor opened, workflows read, scheduled tasks, the Overseer or the voice set, the
    /// statistics opened.
    pub(crate) fn features_in_use(&self) -> Vec<FeatureUse> {
        let projects = self.projects.read().clone();
        let settings = self.settings.read().clone();
        let mut used = Vec::new();
        let mut by_projects = |feature: Feature, n: usize| {
            if n > 0 {
                used.push(FeatureUse {
                    feature,
                    projects: n as u32,
                });
            }
        };
        // Kanban: the projects with tickets; a source of tickets connected (at the first launch, Réglages) feeds it too.
        let with_tickets = {
            let tickets = self.tickets.read();
            projects
                .iter()
                .filter(|p| tickets.iter().any(|t| t.project_id == p.id))
                .count()
        };
        by_projects(Feature::Kanban, with_tickets);
        // Docs: a spec, decisions or documents in the project, a branch of its own for them, or a discussion of the tab.
        let discussing: std::collections::HashSet<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                rt.meta
                    .discussion
                    .is_some()
                    .then(|| rt.meta.project_id.clone())
            })
            .collect();
        let with_docs = projects
            .iter()
            .filter(|p| {
                let dir = std::path::Path::new(&p.path).join(".escouade");
                !p.docs_branch.is_empty()
                    || discussing.contains(&p.id)
                    || ["spec", "decisions", "documents"]
                        .iter()
                        .any(|d| holds(&dir.join(d)))
            })
            .count();
        by_projects(Feature::Docs, with_docs);
        // Workflows: the projects whose workflows were read.
        let with_workflows = projects
            .iter()
            .filter(|p| {
                p.workflows_enabled
                    && (!p.workflow_fixes.is_empty()
                        || !crate::workflowfiles::load_all(&crate::workflowfiles::places(
                            p, &self.data,
                        ))
                        .docs
                        .is_empty())
            })
            .count();
        by_projects(Feature::Workflows, with_workflows);
        // Scheduled tasks: the projects that have some.
        let with_scheduled = {
            let tasks = self.scheduled.read();
            projects
                .iter()
                .filter(|p| tasks.iter().any(|t| t.project_id == p.id))
                .count()
        };
        by_projects(Feature::Scheduled, with_scheduled);
        // The editor: a file opened in it, its extensions, its language servers or its font set.
        let saved = self.onboarding.read().clone();
        let seen =
            |d: Discovery| saved.discover.get(&d) == Some(&crate::model::DiscoverOutcome::Done);
        let ui_has = |what: &str| {
            self.ui
                .read()
                .panes
                .values()
                .any(|v| v.to_string().contains(&format!("\"{what}\"")))
        };
        if seen(Discovery::Editor)
            || ui_has("editor")
            || holds(&self.data.root().join("extensions"))
            || !settings.language_servers.is_empty()
            || !settings.language_servers_trusted.is_empty()
            || settings.editor_font_size != crate::model::DEFAULT_FONT_SIZE
        {
            used.push(FeatureUse {
                feature: Feature::Editor,
                projects: 0,
            });
        }
        // The Overseer: turned on once, or spoken to.
        let overseer = {
            let st = self.overseer.state.lock();
            st.agent_id.is_some() || !st.messages.is_empty()
        };
        if settings.overseer.enabled || overseer {
            used.push(FeatureUse {
                feature: Feature::Overseer,
                projects: 0,
            });
        }
        // The voice: set, or a model downloaded.
        if settings.voice != crate::model::VoiceSettings::default()
            || holds(&self.data.root().join("voice").join("models"))
            || seen(Discovery::Dictation)
            || seen(Discovery::Voice)
        {
            used.push(FeatureUse {
                feature: Feature::Voice,
                projects: 0,
            });
        }
        // The statistics: their view left open.
        if self.ui.read().view == "stats" {
            used.push(FeatureUse {
                feature: Feature::Stats,
                projects: 0,
            });
        }
        // The cost: what it shows needs no gesture; a limit of spending set says it is read.
        if settings
            .accounts
            .iter()
            .any(|a| a.daily_budget.is_some_and(|b| b > 0.0))
            || settings.cache_savings_in_bar
        {
            used.push(FeatureUse {
                feature: Feature::Cost,
                projects: 0,
            });
        }
        if with_tickets == 0 && self.onboarding_facts().sources_connected {
            used.push(FeatureUse {
                feature: Feature::Kanban,
                projects: 0,
            });
        }
        used.sort_by_key(|u| u.feature);
        used
    }

    /// Once, at the first start of the version that brings the switches (F-1): an installation updated keeps on what
    /// it uses (noted in `features_used`), the rest as a new installation has it; a new installation has them all on
    /// already (`Core::load`). Brought within the policies, written at once.
    /// The installation updated has the assistant open by itself at its next window (`Onboarding::features_due`), and every
    /// installation has met the features of this version (`Onboarding::features_known`).
    pub(crate) fn settle_features(&self) {
        let update = self.settings.read().features.is_none();
        {
            let mut saved = self.onboarding.write();
            if saved.features_known.is_empty() {
                saved.features_known = Feature::ALL.into_iter().collect();
            }
            // A new installation has it at its station of the first launch.
            saved.features_due |= update && !self.fresh;
        }
        self.request_save();
        if !update {
            return;
        }
        let used = self.features_in_use();
        let features =
            escouade_model::features::settle(&used, escouade_model::features::NEW_INSTALL);
        log::info!(
            "features: settled at the update, in use {:?}",
            used.iter().map(|u| u.feature.id()).collect::<Vec<_>>()
        );
        {
            let mut s = self.settings.write();
            s.features = Some(features);
            s.features_used = used;
            self.locks.read().clamp_settings(&mut s);
        }
        self.write_settings_file();
    }

    /// The switches changed from `before` (a save, a policy): what an off feature does in the background stops, what an
    /// on one does starts again, and the window reads its discoveries again.
    pub(crate) fn features_changed(self: &Arc<Self>, before: &Settings) {
        let now = self.settings.read().clone();
        let changed: Vec<Feature> = Feature::ALL
            .into_iter()
            .filter(|f| before.feature(*f) != now.feature(*f))
            .collect();
        if changed.is_empty() {
            return;
        }
        log::info!(
            "features: {}",
            changed
                .iter()
                .map(|f| format!("{} {}", f.id(), if now.feature(*f) { "on" } else { "off" }))
                .collect::<Vec<_>>()
                .join(", ")
        );
        for f in &changed {
            match f {
                // Off, the autopilot starts nothing (`schedule_now`); on, it goes on.
                Feature::Kanban if now.feature(*f) => self.schedule(),
                Feature::Overseer => {
                    // Off at work: its turn interrupted, then its process stops as at rest (`overseer_sync`).
                    let busy = self.overseer_agent().and_then(|h| {
                        let rt = h.lock();
                        rt.meta.status.is_active().then(|| rt.meta.id.clone())
                    });
                    match busy.filter(|_| !now.feature(*f)) {
                        Some(id) => {
                            let c = self.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = c.interrupt(&id).await {
                                    log::warn!("overseer: not interrupted: {e:#}");
                                }
                                c.overseer_sync();
                            });
                        }
                        None => self.overseer_sync(),
                    }
                }
                Feature::Workflows if now.feature(*f) => self.refresh_all_workflows(),
                _ => {}
            }
        }
        self.hub.emit(crate::model::UiEvent::DiscoverChanged {});
    }
}

/// The assistant of the features (F-2, ESC-559) as the window shows it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TourView {
    /// To open by itself now: the first window of the version that brings the switches, on an installation updated to it.
    /// Never for a client of the remote API.
    pub open: bool,
    /// What the installation uses, feature by feature: what was noted at the update (`featuresUsed`) and what it uses now
    /// (a source of tickets connected, the voice set at the first launch). « Il reste allumé ».
    pub used: Vec<FeatureUse>,
    /// The features a later update brought that this installation has not met: the window of the novelties gives each
    /// its screen, with the same switch.
    pub new: Vec<Feature>,
}

/// A gesture on the assistant.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TourAct {
    /// Ended (« Terminer ») or passed (« Passer : garder l'essentiel »): it never opens again by itself, and every
    /// feature is met. Its switches were saved as they changed; « Passer » changes none.
    End,
    /// The features shown in the window of the novelties: met.
    Met { features: Vec<Feature> },
}

/// `used` and `now` together, each feature once, with the most projects either says.
pub fn merge_used(used: &[FeatureUse], now: &[FeatureUse]) -> Vec<FeatureUse> {
    let mut out: Vec<FeatureUse> = Vec::new();
    for u in used.iter().chain(now) {
        match out.iter_mut().find(|o| o.feature == u.feature) {
            Some(o) => o.projects = o.projects.max(u.projects),
            None => out.push(u.clone()),
        }
    }
    out.sort_by_key(|u| u.feature);
    out
}

/// The view of the assistant, from what is kept and what is used.
pub fn tour_view(
    saved: &crate::model::Onboarding,
    used: Vec<FeatureUse>,
    window: bool,
) -> TourView {
    TourView {
        open: window && saved.features_due,
        used,
        new: if saved.features_known.is_empty() {
            Vec::new()
        } else {
            Feature::ALL
                .into_iter()
                .filter(|f| !saved.features_known.contains(f))
                .collect()
        },
    }
}

/// `act` done on what is kept.
pub fn tour_apply(saved: &mut crate::model::Onboarding, act: &TourAct) {
    match act {
        TourAct::End => {
            saved.features_due = false;
            saved.features_known.extend(Feature::ALL);
            // Its station of the first launch is done too: done again from Aide, it shows so.
            saved.steps.insert(
                crate::model::OnboardingStep::Features,
                crate::model::StepOutcome::Done,
            );
        }
        TourAct::Met { features } => saved.features_known.extend(features.iter().copied()),
    }
}

impl Core {
    /// The assistant of the features as the window shows it (F-2).
    pub fn features_tour_view(&self) -> TourView {
        let window = crate::remoteapi::current() == crate::remoteapi::Client::Window;
        // One lock at a time: a read taken twice waits behind a writer queued in between.
        let noted = self.settings.read().features_used.clone();
        let used = merge_used(&noted, &self.features_in_use());
        let saved = self.onboarding.read().clone();
        tour_view(&saved, used, window)
    }

    /// `act` on the assistant; its view then.
    pub fn features_tour_act(&self, act: TourAct) -> TourView {
        tour_apply(&mut self.onboarding.write(), &act);
        self.request_save();
        self.features_tour_view()
    }
}

#[cfg(test)]
#[path = "features_tests.rs"]
mod tests;
