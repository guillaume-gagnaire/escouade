//! The team's list of projects on this device (TS-8): read at each reading of the team (an administrator's device
//! writes again the projects of an administrator who left), shown in « Projets de l'équipe », written by an
//! administrator. What the window makes of it (the projects already here, « Cloner ») is `team_projects.rs`'s.

use escouade_team::catalog::{self, Catalog, Listed};
use escouade_team::crypto::Role;

use super::{say, Teams};

/// A member of the team, as the access of a project names them.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemberView {
    pub id: String,
    pub admin: bool,
    pub me: bool,
}

/// What a team's list is, with what the window needs to write it.
#[derive(Clone, Debug)]
pub struct CatalogOf {
    pub name: String,
    pub admin: bool,
    pub me: String,
    pub members: Vec<MemberView>,
    pub catalog: Catalog,
}

fn refused(e: &escouade_team::Error) -> String {
    match e {
        escouade_team::Error::Refused(why) => tr!(
            "La liste refuse ce projet : {why}",
            "The list refuses this project: {why}"
        ),
        e => say(e),
    }
}

impl Teams {
    /// Reads the team's list after its stream; an administrator's device writes again the projects of an
    /// administrator who left. An error is logged: the team's reading goes on.
    pub(super) async fn read_catalog(&self, t: &mut escouade_team::Team) {
        if let Err(e) = catalog::sync(t).await {
            log::warn!("teams: the list of projects not read: {}", say(&e));
            return;
        }
        if let Err(e) = catalog::serve(t, super::now()).await {
            log::warn!("teams: the list of projects not served: {}", say(&e));
        }
    }

    fn catalog_of(&self, id: &str, t: &escouade_team::Team, catalog: Catalog) -> CatalogOf {
        let me = t.identity().member;
        let members = t
            .members()
            .unwrap_or_default()
            .into_iter()
            .map(|(m, role, _)| MemberView {
                id: m.to_string(),
                admin: role == Role::Admin,
                me: m == me,
            })
            .collect();
        CatalogOf {
            name: self.known(id).map(|k| k.name).unwrap_or_default(),
            admin: t.is_admin().unwrap_or(false),
            me: me.to_string(),
            members,
            catalog,
        }
    }

    /// The team's list, read from the server (offline: as this device read it last).
    pub async fn catalog(&self, id: &str) -> Result<CatalogOf, String> {
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let catalog = match catalog::sync(&mut t).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("teams: the list of projects read offline: {}", say(&e));
                catalog::view(&t).map_err(|e| say(&e))?
            }
        };
        Ok(self.catalog_of(id, &t, catalog))
    }

    /// An administrator adds a project to the list (`project` None) or changes it.
    pub async fn catalog_put(
        &self,
        id: &str,
        project: Option<&str>,
        listed: Listed,
    ) -> Result<(String, CatalogOf), String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let (pid, catalog) = catalog::put(&mut t, project, listed, super::now())
            .await
            .map_err(|e| refused(&e))?;
        Ok((pid, self.catalog_of(id, &t, catalog)))
    }

    /// An administrator takes a project out of the list.
    pub async fn catalog_remove(&self, id: &str, project: &str) -> Result<CatalogOf, String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let catalog = catalog::remove(&mut t, project, super::now())
            .await
            .map_err(|e| say(&e))?;
        Ok(self.catalog_of(id, &t, catalog))
    }

    /// « Réessayer » after an alarm of the list.
    pub async fn catalog_retry(&self, id: &str) -> Result<CatalogOf, String> {
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let catalog = catalog::retry(&mut t).await.map_err(|e| say(&e))?;
        Ok(self.catalog_of(id, &t, catalog))
    }
}
