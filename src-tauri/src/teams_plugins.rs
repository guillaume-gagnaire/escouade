//! The team's private store on this device (TS-14): read at each reading of the team, its writes for the core
//! (`teamplugins.rs`), which plays the checks, keeps the team's sealed key and takes the plugins. Here, only what needs
//! the team itself (its lock, its server).

use escouade_team::plugins::{self, Store, Submission};
use escouade_team::signing::SigningKey;

use super::{say, Teams};

/// The team's store, with what the window needs to act on it.
#[derive(Clone, Debug)]
pub struct StoreOf {
    pub name: String,
    pub admin: bool,
    /// This member (hex).
    pub me: String,
    /// The team's fingerprint, as its statements name it (hex).
    pub mark: String,
    pub store: Store,
}

impl Teams {
    /// Reads the team's store after its stream. An error is logged: the team's reading goes on.
    pub(super) async fn read_plugins(&self, t: &mut escouade_team::Team) {
        if let Err(e) = plugins::sync(t).await {
            log::warn!("teams: the team's store not read: {}", say(&e));
        }
    }

    fn store_of(&self, id: &str, t: &escouade_team::Team, store: Store) -> StoreOf {
        StoreOf {
            name: self.known(id).map(|k| k.name).unwrap_or_default(),
            admin: t.is_admin().unwrap_or(false),
            me: t.identity().member.to_string(),
            mark: t
                .fingerprint()
                .map(|f| hex::encode(f.0))
                .unwrap_or_default(),
            store,
        }
    }

    /// The team's store as this device read it last, without the server.
    pub fn plugins_view(&self, id: &str) -> Option<StoreOf> {
        let team = self.team(id).ok()?;
        let t = team.try_lock().ok()?;
        let store = plugins::view(&t).ok()?;
        Some(self.store_of(id, &t, store))
    }

    /// The team's store, read from the server (offline: as this device read it last).
    pub async fn plugins_store(&self, id: &str) -> Result<StoreOf, String> {
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let store = match plugins::sync(&mut t).await {
            Ok(s) => s,
            Err(e) => {
                log::warn!("teams: the team's store read offline: {}", say(&e));
                plugins::view(&t).map_err(|e| say(&e))?
            }
        };
        Ok(self.store_of(id, &t, store))
    }

    /// An administrator declares the team's key and its namespace.
    pub async fn plugins_declare(
        &self,
        id: &str,
        keys: Vec<String>,
        namespace: &str,
    ) -> Result<StoreOf, String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let store = plugins::declare(&mut t, keys, namespace, super::now())
            .await
            .map_err(|e| say(&e))?;
        Ok(self.store_of(id, &t, store))
    }

    /// A member proposes a version, its archive and module encrypted on the server.
    pub async fn plugins_submit(
        &self,
        id: &str,
        submission: Submission,
        archive: &[u8],
        module: Option<&[u8]>,
    ) -> Result<(String, StoreOf), String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let (sid, store) = plugins::submit(&mut t, submission, archive, module, super::now())
            .await
            .map_err(|e| say(&e))?;
        Ok((sid, self.store_of(id, &t, store)))
    }

    /// The archive and the module of a proposal or a version, opened and checked against their SHA-256.
    pub async fn plugins_files(
        &self,
        id: &str,
        archive: [u8; 16],
        archive_sha256: &str,
        module: Option<[u8; 16]>,
        module_sha256: Option<&str>,
    ) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
        let team = self.team(id)?;
        let mut t = team.lock().await;
        plugins::files(&mut t, archive, archive_sha256, module, module_sha256)
            .await
            .map_err(|e| say(&e))
    }

    /// An administrator publishes a proposal whose digest their checks found, signed by the team's key.
    pub async fn plugins_publish(
        &self,
        id: &str,
        submission: &str,
        digest: &str,
        key: &SigningKey,
    ) -> Result<StoreOf, String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let sign = |bytes: &[u8], comment: &str| key.sign(bytes, comment);
        let store = plugins::publish(&mut t, submission, digest, &sign, super::now())
            .await
            .map_err(|e| say(&e))?;
        Ok(self.store_of(id, &t, store))
    }

    /// An administrator writes the team's revocation list, signed by the team's key.
    pub async fn plugins_revoke(
        &self,
        id: &str,
        json: Vec<u8>,
        minisig: String,
    ) -> Result<StoreOf, String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let store = plugins::revoke(&mut t, json, minisig, super::now())
            .await
            .map_err(|e| say(&e))?;
        Ok(self.store_of(id, &t, store))
    }

    /// A proposal set aside.
    pub async fn plugins_dismiss(&self, id: &str, submission: &str) -> Result<StoreOf, String> {
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let store = plugins::dismiss(&mut t, submission, "", super::now())
            .await
            .map_err(|e| say(&e))?;
        Ok(self.store_of(id, &t, store))
    }

    /// « Réessayer » after an alarm of the store.
    pub async fn plugins_retry(&self, id: &str) -> Result<StoreOf, String> {
        let team = self.team(id)?;
        let mut t = team.lock().await;
        let store = plugins::retry(&mut t).await.map_err(|e| say(&e))?;
        Ok(self.store_of(id, &t, store))
    }
}
