//! The team's circuit breakers on this device (TS-15, `docs/TEAM-SERVER.md` §9): read first at each reading of the
//! team (the live feed brings an entry within the second), applied by the core through its locks
//! (`team_breakers.rs`), shown on the band, set and lifted by an administrator in the team's window.
//!
//! Offline, what was read last holds (it is on the disk, read again at the start). A team whose last reading failed is
//! « stale »: the core reads its breakers again before any new start of one of its agents (`Core::team_gate`), so that a
//! breaker set meanwhile applies at the return before anything starts.

use std::sync::Arc;

use escouade_team::breaker::{self, Scope, View, What};
use serde::{Deserialize, Serialize};

use super::{now, say, Teams};

/// Where a team's breakers go: the team's id, its name, what holds (None: the device left it).
pub type BreakersSink = Arc<dyn Fn(&str, &str, Option<&View>) + Send + Sync>;

/// What the window shows of a team's breakers.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakersView {
    pub team: String,
    pub name: String,
    /// This device is an administrator's: it sets and lifts.
    pub admin: bool,
    pub view: View,
    /// The name this device signs with (the one of the team's settings).
    pub signer_name: String,
    /// When the server last answered this device (seconds).
    pub last_contact: Option<u64>,
    /// The plugins the team's revocation list withdraws in every version (TS-14): the circuit breaker reads them, the
    /// core fills them in (`Core::team_withdrawn`).
    pub withdrawn: Vec<WithdrawnView>,
}

/// A plugin the team's revocation list withdraws, and why.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawnView {
    pub id: String,
    pub reason: String,
}

/// What an administrator asks to take out, as the window sends it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakerAsk {
    pub scope: Scope,
    pub what: What,
    pub reason: String,
    pub by: String,
}

/// How long a reading of the breakers may hold a start back (`Core::team_gate`).
pub const GATE: std::time::Duration = std::time::Duration::from_secs(3);

impl Teams {
    /// Where the breakers in force go (the core): each team opened is told at once, from what it read last.
    pub fn set_breakers_sink(&self, sink: BreakersSink) {
        *self.on_breakers.lock() = Some(sink);
    }

    /// The breakers of each team opened, from what this device read last (offline): applied at the start. Each team is
    /// stale until its first reading.
    pub async fn start_breakers(&self) {
        let open: Vec<(String, super::Shared)> = self
            .open
            .lock()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (id, team) in open {
            self.stale.lock().insert(id.clone());
            let view = {
                let t = team.lock().await;
                breaker::view(&t)
            };
            match view {
                Ok(v) => self.put_breakers(&id, v, true),
                Err(e) => log::warn!("teams: the breakers of a team not read: {}", say(&e)),
            }
        }
    }

    /// Reads the team's breakers, first of all at each reading: what is new, applied at once; an administrator's
    /// device writes again those whose author left. True when they were read.
    pub(super) async fn read_breakers(&self, id: &str, t: &mut escouade_team::Team) -> bool {
        let mut view = match breaker::sync(t).await {
            Ok(v) => v,
            Err(e) => {
                log::warn!("teams: breakers not read: {}", say(&e));
                return false;
            }
        };
        if t.is_admin().unwrap_or(false) {
            match breaker::serve(t, now()).await {
                Ok(n) if n > 0 => {
                    if let Ok(v) = breaker::view(t) {
                        view = v;
                    }
                }
                Ok(_) => {}
                Err(e) => log::warn!("teams: breakers not written again: {}", say(&e)),
            }
        }
        self.stale.lock().remove(id);
        self.put_breakers(id, view, false);
        true
    }

    /// The team's breakers read now (a start of one of its agents waits for it, `Core::team_gate`).
    pub async fn read_breakers_now(&self, id: &str) -> Result<(), String> {
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let view = breaker::sync(&mut t).await.map_err(|e| say(&e))?;
        self.touch(id);
        self.stale.lock().remove(id);
        self.put_breakers(id, view, false);
        Ok(())
    }

    /// Whether the team's last reading failed (offline): its breakers are read again before a start.
    pub fn breakers_stale(&self, id: &str) -> bool {
        self.stale.lock().contains(id)
    }

    pub(super) fn mark_stale(&self, id: &str) {
        self.stale.lock().insert(id.to_owned());
    }

    /// When the server of the team last answered this device (seconds); None when it never did.
    pub fn last_contact(&self, id: &str) -> Option<u64> {
        self.known(id).ok().and_then(|k| k.last_contact)
    }

    /// The server answered: the hour kept (written to the disk at most once a minute).
    pub(super) fn touch(&self, id: &str) {
        let at = now();
        let write = {
            let mut known = self.known.lock();
            match known.iter_mut().find(|k| k.id == id) {
                Some(k) => {
                    let write = k.last_contact.is_none_or(|l| at.saturating_sub(l) >= 60);
                    k.last_contact = Some(at);
                    write
                }
                None => false,
            }
        };
        if write {
            self.save();
        }
    }

    fn put_breakers(&self, id: &str, view: View, always: bool) {
        let changed = {
            let mut views = self.breaker_views.lock();
            let changed = views.get(id) != Some(&view);
            views.insert(id.to_owned(), view.clone());
            changed
        };
        if !(changed || always) {
            return;
        }
        let name = self.known(id).map(|k| k.name).unwrap_or_default();
        let sink = self.on_breakers.lock().clone();
        if let Some(sink) = sink {
            sink(id, &name, Some(&view));
        }
        if changed {
            (self.changed)();
        }
    }

    pub(super) fn forget_breakers(&self, id: &str) {
        self.breaker_views.lock().remove(id);
        self.stale.lock().remove(id);
        let sink = self.on_breakers.lock().clone();
        if let Some(sink) = sink {
            sink(id, "", None);
        }
    }

    async fn breakers_of(&self, id: &str) -> Result<BreakersView, String> {
        let known = self.known(id)?;
        let team = self.team(id)?;
        let t = team.lock().await;
        let view = match self.breaker_views.lock().get(id).cloned() {
            Some(v) => v,
            None => breaker::view(&t).map_err(|e| say(&e))?,
        };
        Ok(BreakersView {
            team: id.to_owned(),
            name: known.name.clone(),
            admin: t.is_admin().unwrap_or(false),
            view,
            signer_name: known.signer_name,
            last_contact: known.last_contact,
            withdrawn: Vec::new(),
        })
    }

    /// The team's breakers, as this device read them.
    pub async fn breakers(&self, id: &str) -> Result<BreakersView, String> {
        self.breakers_of(id).await
    }

    /// The other devices of the team online now: what the confirmation says will stop.
    pub async fn breaker_online(&self, id: &str) -> Result<usize, String> {
        let team = self.team(id)?;
        let t = team.lock().await;
        breaker::online(&t).await.map_err(|e| say(&e))
    }

    /// An administrator sets a breaker: signed by this device, sent, read back.
    pub async fn breaker_set(&self, id: &str, ask: BreakerAsk) -> Result<BreakersView, String> {
        self.unblocked(id)?;
        let by = signer(&ask.by)?;
        if ask.reason.trim().is_empty() {
            return Err(tr!(
                "Dis pourquoi : chaque membre le lira sur sa bande.",
                "Say why: each member will read it on their band."
            ));
        }
        self.update(id, |k| k.signer_name = by.clone());
        let team = self.team(id)?;
        let view = {
            let mut t = team.lock().await;
            breaker::set(&mut t, ask.scope, ask.what, &ask.reason, &by, now())
                .await
                .map_err(|e| say(&e))?
                .1
        };
        self.put_breakers(id, view, false);
        self.breakers_of(id).await
    }

    /// An administrator lifts a breaker in force.
    pub async fn breaker_lift(
        &self,
        id: &str,
        breaker_id: &str,
        by: &str,
    ) -> Result<BreakersView, String> {
        self.unblocked(id)?;
        let by = signer(by)?;
        self.update(id, |k| k.signer_name = by.clone());
        let team = self.team(id)?;
        let view = {
            let mut t = team.lock().await;
            breaker::lift(&mut t, breaker_id, &by, now())
                .await
                .map_err(|e| say(&e))?
        };
        self.put_breakers(id, view, false);
        self.breakers_of(id).await
    }

    /// « Réessayer » after an alarm of the breakers.
    pub async fn breakers_retry(&self, id: &str) -> Result<BreakersView, String> {
        let team = self.team(id)?;
        let view = {
            let mut t = team.lock().await;
            breaker::retry(&mut t).await.map_err(|e| say(&e))?
        };
        self.put_breakers(id, view, false);
        self.breakers_of(id).await
    }
}

/// The name an administrator signs with, on one line.
fn signer(by: &str) -> Result<String, String> {
    let by = by.trim().chars().take(80).collect::<String>();
    if by.is_empty() || by.contains('\n') {
        return Err(tr!(
            "Dis sous quel nom tu signes : les membres le verront.",
            "Say what name you sign with: the members will see it."
        ));
    }
    Ok(by)
}

#[cfg(test)]
impl Teams {
    /// When the server last answered, as a test sets it (« sans contact depuis N heures »).
    pub(crate) fn set_last_contact(&self, id: &str, at: Option<u64>) {
        self.update(id, |k| k.last_contact = at);
    }
}
