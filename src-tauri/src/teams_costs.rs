//! The costs of a team on this device (TS-16): read at each reading of the team (the administrators' key, the summary,
//! and, on an administrator's device, every device's totals), and sent by the core once an hour (`teamcosts`).

use escouade_team::costs::{self, Totals, View};

use super::{say, Teams};

/// What the window and the core read of a team's costs.
#[derive(Clone, Debug, Default)]
pub struct CostsOf {
    pub name: String,
    pub admin: bool,
    /// This device's member (hex).
    pub me: String,
    pub view: View,
}

/// The month in progress, `2026-11`, in local time.
pub fn month_now() -> String {
    chrono::Local::now().format("%Y-%m").to_string()
}

impl Teams {
    /// Reads the streams of the costs after the team's stream; an administrator's device draws and wraps the key, and
    /// publishes the summary of the month. An error is logged: the team's reading goes on.
    pub(super) async fn read_costs(&self, id: &str, t: &mut escouade_team::Team) {
        self.cost_members
            .lock()
            .insert(id.to_owned(), t.identity().member.to_string());
        match costs::read(t, &month_now(), crate::model::now_ms()).await {
            Ok(v) => self.put_costs(id, v),
            Err(e) => log::warn!("teams: costs not read: {}", say(&e)),
        }
    }

    fn put_costs(&self, id: &str, view: View) {
        let changed = {
            let mut views = self.cost_views.lock();
            let changed = views.get(id) != Some(&view);
            views.insert(id.to_owned(), view);
            changed
        };
        if changed {
            (self.changed)();
        }
    }

    /// Puts this device's totals in the queue and sends what waits. `build` is given this device's member (hex) and
    /// whether the whole month is to go again (a new key); None: nothing new. Offline, they wait.
    pub async fn costs_send(
        &self,
        id: &str,
        build: impl FnOnce(&str, bool) -> Option<Totals>,
    ) -> Result<View, String> {
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let me = t.identity().member.to_string();
        let whole = costs::wants_month(&t).map_err(|e| say(&e))?;
        if let Some(totals) = build(&me, whole) {
            costs::queue(&t, totals).map_err(|e| say(&e))?;
        }
        let sent = costs::send(&mut t).await;
        let view = match sent {
            Ok(v) => v,
            Err(e) => {
                log::info!("teams: costs wait to go: {}", say(&e));
                costs::view(&t).map_err(|e| say(&e))?
            }
        };
        drop(t);
        self.put_costs(id, view.clone());
        Ok(view)
    }

    /// What this device knows of a team's costs, as last read.
    pub async fn costs_of(&self, id: &str) -> Result<CostsOf, String> {
        let team = self.team(id)?;
        let t = team.lock().await;
        let view = match self.cost_views.lock().get(id).cloned() {
            Some(v) => v,
            None => costs::view(&t).map_err(|e| say(&e))?,
        };
        Ok(CostsOf {
            name: self.known(id).map(|k| k.name).unwrap_or_default(),
            admin: t.is_admin().unwrap_or(false),
            me: t.identity().member.to_string(),
            view,
        })
    }

    /// The costs as last read, without waiting for a reading under way (the check before a start).
    pub fn costs_seen(&self, id: &str) -> Option<View> {
        self.cost_views.lock().get(id).cloned()
    }

    /// This device's member in a team (hex), as last read.
    pub fn member_of(&self, id: &str) -> Option<String> {
        self.cost_members.lock().get(id).cloned()
    }
}
