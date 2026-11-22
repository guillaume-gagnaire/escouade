//! The administrator's circuit breaker in the core (TS-15, `docs/TEAM-SERVER.md` §9).
//!
//! `escouade-team::breaker` holds the entries on the server: signed by an administrator, numbered by a serial, judged
//! on each device. `teams_breakers.rs` reads them first at each reading of the team. Here the core applies them, through
//! its locks (TS-11) and nothing else: a breaker is a restriction of rank 5 whose `locked_by` names it
//! (`LockedBy::Breaker`, `Locks::apply_breakers`). So:
//!
//! - a harness or a model taken out: every start refuses it (`check_engine`, `check_model`, `judge_start`), an agent at
//!   rest stops at once, one at work at the end of its turn (`settle_agents`, `settle_locked_mode`);
//! - every agent stopped (`Locks::halt`): as quitting the app, at once, the turn under way lost (`halt_agent`); each agent
//!   resumes its session once the breaker is lifted;
//! - a plugin taken out: disabled (`Plugins::set_cuts`, the user's switch untouched), then uninstalled when the breaker
//!   says so.
//!
//! It only restricts: it starts, installs and writes nothing. It holds for the team's space alone: a project of no team
//! (a personal one) has the app's locks, which no team's breaker reaches but a plugin's (a plugin is of the app).

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{bail, Result};
use escouade_team::breaker::{Breaker, View, What};

use crate::core::Core;
use crate::locks::{LockedBy, MAX_OFFLINE_HOURS, PLUGINS_DENIED};

/// What a team's circuit breakers say on this device: its name and the breakers in force.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TeamBreakers {
    pub name: String,
    pub breakers: Vec<Breaker>,
}

/// What the team's revocation list withdraws in every version (TS-14, `disable`): its name and, for each plugin, its
/// id and the list's reason. The circuit breaker reads it (§9).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TeamWithdrawals {
    pub name: String,
    pub withdrawn: Vec<(String, String)>,
}

/// The withdrawals of every version of a revocation list (`disable`, `*`), with their reason in the window's language.
/// A withdrawal of some versions only stays the list's: `Plugins::apply_team` disables those versions.
pub fn withdrawn_of(list: &crate::plugins::store::RevocationList) -> Vec<(String, String)> {
    let lang = crate::i18n::ui();
    list.entries
        .iter()
        .filter(|e| e.action == "disable" && e.versions.as_str() == Some("*"))
        .map(|e| (e.id.clone(), e.reason.get(lang).to_string()))
        .collect()
}

/// « Arrêt demandé par Claire Martin : fuite de clé chez le fournisseur X. »
pub fn asked_by(by: &LockedBy) -> String {
    match by {
        LockedBy::Breaker { by, reason, .. } => tr!(
            "Arrêt demandé par {by} : {reason}.",
            "Stop asked by {by}: {reason}."
        ),
        LockedBy::Withdrawn { team, reason } => tr!(
            "Retiré par la liste de révocation de l’équipe {team} : {reason}.",
            "Withdrawn by the revocation list of the team {team}: {reason}."
        ),
        other => tr!("Arrêt demandé {by}.", "Stop asked {by}.", by = other.by()),
    }
}

/// The local hour of `at` (seconds): « le 12/11 à 14:05 ».
fn hour_of(at: u64) -> String {
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(at as i64, 0) {
        chrono::LocalResult::Single(t) => t.format("%d/%m %H:%M").to_string(),
        _ => at.to_string(),
    }
}

/// Why no new agent of the team `team` starts under the policy « sans contact depuis N heures » (`hours`), its server
/// last heard at `last` (seconds), now `now`; None when it may.
pub fn contact_refusal(team: &str, hours: u64, last: Option<u64>, now: u64) -> Option<String> {
    let fresh = last.is_some_and(|l| now.saturating_sub(l) <= hours * 3600);
    if fresh {
        return None;
    }
    Some(match last {
        Some(l) => tr!(
            "Pas de nouvel agent de l’équipe {team} : son serveur n’a pas répondu depuis le {at}, et la stratégie de ce poste demande un contact dans les {hours} dernières heures.",
            "No new agent of the team {team}: its server has not answered since {at}, and this device’s policy asks for a contact within the last {hours} hours.",
            at = hour_of(l)
        ),
        None => tr!(
            "Pas de nouvel agent de l’équipe {team} : son serveur n’a encore jamais répondu à ce poste, et la stratégie de ce poste demande un contact dans les {hours} dernières heures.",
            "No new agent of the team {team}: its server never answered this device yet, and this device’s policy asks for a contact within the last {hours} hours."
        ),
    })
}

impl Core {
    /// The breakers in force of the team `id` (None: the device left it): to the locks at once, and what they stop
    /// stopped (`settle_breakers`, through `apply_locks`).
    pub(crate) fn set_team_breakers_of(
        self: &Arc<Self>,
        id: &str,
        name: &str,
        view: Option<&View>,
    ) {
        let next = view.filter(|v| !v.active.is_empty()).map(|v| TeamBreakers {
            name: name.to_owned(),
            breakers: v.active.clone(),
        });
        {
            let mut all = self.team_breakers.write();
            if all.get(id) == next.as_ref() {
                return;
            }
            match next {
                Some(t) => all.insert(id.to_owned(), t),
                None => all.remove(id),
            };
        }
        log::info!("teams: the circuit breakers of a team changed, applied");
        self.apply_locks();
    }

    /// What the revocation list of the team `id` withdraws in every version (None: none, or the device left it): read by
    /// the circuit breaker, to the locks at once when it changed.
    pub(crate) fn set_team_withdrawals(
        self: &Arc<Self>,
        id: &str,
        list: Option<&crate::plugins::store::RevocationList>,
    ) {
        let name = self
            .team_link
            .read()
            .clone()
            .and_then(|t| t.place_of(id))
            .map(|(_, _, name)| name)
            .unwrap_or_else(|| id.to_owned());
        let next = list
            .map(withdrawn_of)
            .filter(|w| !w.is_empty())
            .map(|withdrawn| TeamWithdrawals { name, withdrawn });
        {
            let mut all = self.team_withdrawals.write();
            if all.get(id) == next.as_ref() {
                return;
            }
            match next {
                Some(w) => all.insert(id.to_owned(), w),
                None => all.remove(id),
            };
        }
        log::info!("teams: the revocation list of a team withdraws other plugins, applied");
        self.apply_locks();
    }

    /// The plugins the revocation list of the team `id` withdraws in every version, with the list's reason.
    pub(crate) fn team_withdrawn(&self, id: &str) -> Vec<(String, String)> {
        self.team_withdrawals
            .read()
            .get(id)
            .map(|w| w.withdrawn.clone())
            .unwrap_or_default()
    }

    /// What the circuit breakers stop at once, called with each change of the locks: the plugins they take out
    /// (disabled, uninstalled when they say so), and every agent of a project where one stops them all (as quitting the
    /// app). A harness or a model taken out is `settle_agents`'s (between two turns).
    pub(crate) fn settle_breakers(self: &Arc<Self>) {
        let base = self.locks.read().clone();
        // The plugins: each one cut with the line its sheet shows.
        let mut cuts = BTreeMap::new();
        if let Some(lock) = base.get(PLUGINS_DENIED) {
            for d in &lock.denies {
                for id in &d.denied {
                    cuts.entry(id.clone()).or_insert_with(|| asked_by(&d.by));
                }
            }
        }
        for id in self.plugins.set_cuts(cuts) {
            self.plugin_changed(&id);
            self.sync_voice_plugins();
        }
        let uninstall: Vec<String> = self
            .team_breakers
            .read()
            .values()
            .flat_map(|t| t.breakers.iter())
            .filter_map(|b| match &b.what {
                What::Plugin {
                    id,
                    uninstall: true,
                } => Some(id.clone()),
                _ => None,
            })
            .collect();
        for id in uninstall {
            if self.plugins.get(&id, crate::i18n::ui()).is_some() {
                log::warn!("plugin {id}: uninstalled, as a team's circuit breaker says");
                if let Err(e) = self.plugin_uninstall(&id, false) {
                    log::warn!("plugin {id}: not uninstalled: {e}");
                }
            }
        }
        // Every agent of a project where a breaker stops them all.
        let by_project = self.locks_by_project();
        let halted: Vec<(String, String)> = self
            .agents
            .read()
            .iter()
            .filter_map(|(id, h)| {
                let rt = h.lock();
                let by = by_project.get(&rt.meta.project_id)?.halt.as_ref()?;
                let runs = rt.proc.is_some() || rt.tui.is_some() || rt.meta.status.is_active();
                runs.then(|| (id.clone(), asked_by(by)))
            })
            .collect();
        for (id, why) in halted {
            self.halt_agent(&id, &why);
        }
    }

    /// The agent `id` stopped at once, as quitting the app does: its process and its interface, the turn under way
    /// lost; its session stays, resumed by its next message once the breaker is lifted. Its conversation says why.
    pub(crate) fn halt_agent(self: &Arc<Self>, id: &str, why: &str) -> bool {
        let Ok(h) = self.agent(id) else { return false };
        let (proc, term) = {
            let mut rt = h.lock();
            let proc = rt.detach();
            let term = rt
                .tui
                .as_ref()
                .map(|run| (run.term_id.clone(), crate::tui::quit_key(&rt.meta)));
            if term.is_some() {
                rt.gen += 1;
                rt.stop_tui(&mut crate::agent::Effects::default());
            }
            (proc, term)
        };
        log::info!("agent {id}: stopped by a team's circuit breaker");
        if let Some(p) = proc {
            p.close_input();
            p.kill();
        }
        if let Some((term, key)) = term {
            self.hooks.revoke(id);
            self.quit_soon(id, &term, key);
        }
        let text = tr!(
            "{why} Le tour en cours est perdu ; l’agent reprendra sa session une fois le coupe-circuit levé.",
            "{why} The turn under way is lost; the agent will resume its session once the circuit breaker is lifted."
        );
        let _ = self.with_agent(id, |rt, fx| {
            if rt.meta.status.is_active() {
                rt.set_status(crate::model::AgentStatus::Done, fx);
            }
            rt.notice("warn", text, fx);
            Ok(())
        });
        self.emit_agent(&h);
        self.request_save();
        true
    }

    /// Before any start of an agent of a team's project: its team's breakers read again when its last reading failed
    /// (the server back: a breaker set meanwhile applies before the start, within `BREAKER_GATE`), then the policy
    /// « sans contact depuis N heures » of this device (`team.maxOfflineHours`). A personal project passes.
    pub(crate) async fn team_gate(self: &Arc<Self>, project_id: &str) -> Result<()> {
        let Some(team) = self
            .projects
            .read()
            .iter()
            .find(|p| p.id == project_id)
            .and_then(|p| p.team.clone())
        else {
            return Ok(());
        };
        let Some(teams) = self.team_link.read().clone() else {
            return Ok(());
        };
        if teams.breakers_stale(&team) {
            let read =
                tokio::time::timeout(crate::teams::BREAKER_GATE, teams.read_breakers_now(&team))
                    .await;
            if !matches!(read, Ok(Ok(()))) {
                log::info!("teams: the breakers of a team not read before a start (offline)");
            }
        }
        let hours = self
            .locks
            .read()
            .value(MAX_OFFLINE_HOURS)
            .and_then(serde_json::Value::as_u64);
        if let Some(hours) = hours {
            let name = teams
                .place_of(&team)
                .map(|(_, _, name)| name)
                .unwrap_or_else(|| team.clone());
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            if let Some(why) = contact_refusal(&name, hours, teams.last_contact(&team), now) {
                bail!(why);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "team_breakers_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "team_breakers_live_tests.rs"]
mod live_tests;
