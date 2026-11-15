//! A team, on this device: its MLS device (`escouade-team-crypto`), its state on the disk (`storage.rs`), its keys
//! and checkpoint in the keychain (`keychain.rs`), its server (`client.rs`).
//!
//! - **Reading the team's stream** ([`Team::sync`]) from the last number this device read. Each entry is taken in its
//!   own transaction: a commit is checked by MLS and by the team's rules, then merged; a message is a history link,
//!   a wrap, a beacon, a join request or a Welcome (`wire::TeamMessage`). Before reading on, the device reads again
//!   the last entry it read: a server that hid it (a rollback) or changed it is caught there.
//! - **Committing** ([`Team::apply`]): the change is staged, its commit sent, and merged only once the server took it.
//!   Refused with 409 `epoch_taken`, the device drops it, reads the commit that won, and stages again if the change is
//!   still to do. It never holds two epochs that diverge; and a commit it sent without hearing back is found again in
//!   the stream (merged if the server has it, dropped if not).
//! - **Forks**: a commit for an epoch already seen that is not the one this device saw, a history link or a beacon
//!   of a member that shows another head for an epoch this device holds: [`escouade_team_crypto::Error::Fork`].
//! - **Rollbacks**: the state on the disk older than the checkpoint of the keychain (an old backup) is refused at
//!   [`Team::open`]; the server that hides its last entries is refused at [`Team::sync`].
//! - **The history links** of the epochs whose committer did not write them are written by any member who holds the
//!   epoch and the one before. **Head beacons** go at each new epoch and every [`Options::beacon_every`].
//! - **What goes out** (links, wraps, Welcomes, beacons) waits in an outbox kept on the disk until the server takes
//!   it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ed25519_dalek::SigningKey;
use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::invite::{InviteLink, JoinRequest, WelcomeDelivery, DEFAULT_LIFETIME};
use escouade_team_crypto::keys::EpochRecord;
use escouade_team_crypto::roster::EscrowPublic;
use escouade_team_crypto::{
    Change, Checkpoint, DeviceId, DeviceKind, Identity, MemberId, ObjectId, Processed, ProjectId,
    Publication, Purpose, RecoveryKey, Role, Roster, SavedState, TeamDevice, CIPHERSUITE,
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_traits::OpenMlsProvider;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::client::{ClientError, LiveFeed, ServerClient, Transport};
use crate::keychain::{DeviceKeys, KeyPlace, Vault};
use crate::storage::{SqliteStorage, TeamProvider, STATE_FILE};
use crate::wire::{self, Entry, Kind, TeamMessage};
use crate::{CryptoError, Error};

/// How many times a change is staged again after losing its epoch, before giving up for now.
const TRIES: usize = 8;

/// Where a team lives on this device, and how it reaches its server.
#[derive(Clone)]
pub struct Setup {
    /// The folder of the teams: each one's state is in `<folder>/<team id>/` (the core's data folder, `teams`).
    pub folder: PathBuf,
    pub vault: Vault,
    pub transport: Arc<dyn Transport>,
    /// The server's host as invitation links name it (`team.example.org`).
    pub server: String,
    pub options: Options,
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Seconds between two beacons of this device at the same epoch.
    pub beacon_every: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            beacon_every: 6 * 3600,
        }
    }
}

/// What applying a change did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    /// The commit is in, and the group at this epoch.
    Done { epoch: u64 },
    /// Another commit did it already.
    AlreadyDone,
}

/// What a reading of the stream did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Synced {
    /// The entries read.
    pub entries: usize,
    /// The commits merged (the device's own found again included).
    pub commits: usize,
    /// The device was removed from the team.
    pub removed: bool,
    /// The device joined by its Welcome.
    pub joined: bool,
}

/// What this device keeps of the team's stream, beside the MLS state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Meta {
    /// The last number read.
    cursor: u64,
    /// The last entry read, its number and its digest: read again before reading on.
    last: Option<(u64, [u8; 32])>,
    /// The digest of the commit that moved each epoch on, as this device saw it.
    commits: BTreeMap<u64, [u8; 32]>,
    /// The epochs whose history link the stream holds.
    links: BTreeSet<u64>,
    /// The commits read before this device joined (while it waits for its Welcome).
    early: BTreeMap<u64, Vec<u8>>,
    /// The join requests an administrator may admit.
    requests: Vec<JoinRequest>,
    /// What waits to go to the server, in order.
    outbox: Vec<TeamMessage>,
    /// The epoch and the hour of this device's last beacon.
    beacon: Option<(u64, u64)>,
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn entry_digest(e: &Entry) -> [u8; 32] {
    digest(&wire::encode(e))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn random<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    getrandom::fill(&mut out).expect("the system gives randomness");
    out
}

fn record_name(epoch: u64) -> Vec<u8> {
    let mut out = b"record/".to_vec();
    out.extend_from_slice(&epoch.to_be_bytes());
    out
}

fn link_prefix(epoch: u64) -> Vec<u8> {
    let mut out = b"link/".to_vec();
    out.extend_from_slice(&epoch.to_be_bytes());
    out
}

/// A team on this device.
pub struct Team {
    id: [u8; 16],
    dir: PathBuf,
    setup: Setup,
    keys: DeviceKeys,
    key_place: KeyPlace,
    client: Arc<ServerClient>,
    device: TeamDevice<TeamProvider>,
    meta: Meta,
    /// The records already on the disk.
    stored: BTreeSet<u64>,
    /// The checkpoint the keychain holds.
    saved: Option<Checkpoint>,
}

impl Team {
    /// Creates a team on the server with the token of `escouade-team-server init`: this device enrols, makes its
    /// keys (kept in the keychain), creates the MLS group with `member` as its owner, and sends the recovery wrap.
    /// The recovery key's 24 words are given once, to show.
    pub async fn create(
        setup: Setup,
        token: &str,
        member: MemberId,
        escrow: Option<EscrowPublic>,
        created_at: u64,
    ) -> Result<(Team, RecoveryKey), Error> {
        let mut team = Self::enrol(setup, token, member).await?;
        let recovery = team.step(|t| {
            let (recovery, publication) = t.device.create_team(created_at, escrow)?;
            t.queue(publication);
            Ok(recovery)
        })?;
        team.flush().await?;
        Ok((team, recovery))
    }

    /// Joins a team with an invitation: the link (its secret after the `#`) and the enrolment token the inviting
    /// administrator asked of the server. This device enrols, makes its keys and sends its join request; it is in
    /// once an administrator admitted it and [`Team::sync`] found the Welcome ([`Synced::joined`]).
    pub async fn join(
        setup: Setup,
        token: &str,
        link: &InviteLink,
        member: MemberId,
    ) -> Result<Team, Error> {
        let mut team = Self::enrol(setup, token, member).await?;
        team.step(|t| {
            let request = t.device.request_join(link)?;
            t.meta.outbox.push(TeamMessage::JoinRequest(request));
            Ok(())
        })?;
        team.flush().await?;
        Ok(team)
    }

    async fn enrol(setup: Setup, token: &str, member: MemberId) -> Result<Team, Error> {
        let auth = SigningKey::from_bytes(&random::<32>());
        let enrolled = ServerClient::enroll(&*setup.transport, token, &auth).await?;
        let keys = DeviceKeys {
            team: enrolled.team,
            member: member.0,
            device: enrolled.device,
            auth: auth.to_bytes(),
            mls: random::<32>(),
            state_key: random::<32>(),
        };
        let place = setup.vault.put_keys(&keys)?;
        let dir = setup.folder.join(hex::encode(keys.team));
        std::fs::create_dir_all(&dir).map_err(|e| Error::Storage(e.to_string()))?;
        let device = TeamDevice::with_identity(
            TeamProvider::new(open_storage(&dir, &keys)?),
            signer(&keys),
            identity(&keys),
        );
        Ok(Self::assemble(
            setup,
            keys,
            place,
            device,
            Meta::default(),
            None,
        ))
    }

    /// Opens a team this device is in, from its folder and the keychain. A state older than the keychain's
    /// checkpoint (an old backup put back) is refused: [`CryptoError::Rollback`].
    pub fn open(setup: Setup, team: [u8; 16]) -> Result<Team, Error> {
        let (keys, place) = setup.vault.keys(&team)?.ok_or(Error::NoKeys)?;
        let dir = setup.folder.join(hex::encode(team));
        let (device, meta, stored) = load(&dir, &keys)?;
        let saved = setup.vault.checkpoint(&team)?;
        if let (Some(cp), true) = (&saved, device.group_id().is_ok()) {
            device.check_restored(cp)?;
        }
        let mut t = Self::assemble(setup, keys, place, device, meta, saved);
        t.stored = stored;
        Ok(t)
    }

    fn assemble(
        setup: Setup,
        keys: DeviceKeys,
        key_place: KeyPlace,
        device: TeamDevice<TeamProvider>,
        meta: Meta,
        saved: Option<Checkpoint>,
    ) -> Team {
        let client = Arc::new(ServerClient::new(
            setup.transport.clone(),
            keys.device,
            SigningKey::from_bytes(&keys.auth),
        ));
        Team {
            id: keys.team,
            dir: setup.folder.join(hex::encode(keys.team)),
            setup,
            keys,
            key_place,
            client,
            device,
            meta,
            stored: BTreeSet::new(),
            saved,
        }
    }

    // === What it says ===

    pub fn id(&self) -> [u8; 16] {
        self.id
    }

    /// This device's identity: its member, its device (the server's id).
    pub fn identity(&self) -> Identity {
        identity(&self.keys)
    }

    /// Where the keys are: the keychain, or a file the interface tells the user about.
    pub fn key_place(&self) -> &KeyPlace {
        &self.key_place
    }

    pub fn folder(&self) -> &Path {
        &self.dir
    }

    pub fn epoch(&self) -> Result<u64, Error> {
        Ok(self.device.epoch()?)
    }

    pub fn roster(&self) -> Result<Roster, Error> {
        Ok(self.device.roster()?)
    }

    /// The verified journal of the members: each member of the roster, their role, and their devices in the group.
    pub fn members(&self) -> Result<Vec<(MemberId, Role, Vec<DeviceId>)>, Error> {
        let roster = self.device.roster()?;
        let leaves = self.device.leaves()?;
        Ok(roster
            .members
            .iter()
            .map(|(m, r)| {
                let devices = leaves
                    .iter()
                    .filter(|i| i.member == *m && i.kind == DeviceKind::Device)
                    .map(|i| i.device)
                    .collect();
                (*m, *r, devices)
            })
            .collect())
    }

    pub fn is_admin(&self) -> Result<bool, Error> {
        Ok(self.device.is_admin()?)
    }

    pub fn is_removed(&self) -> bool {
        self.device.is_removed()
    }

    /// The six words of the current head, to compare out of band.
    pub fn head_words(&self) -> Result<[&'static str; 6], Error> {
        Ok(self.device.head_words()?)
    }

    pub fn checkpoint(&self) -> Option<Checkpoint> {
        self.device.checkpoint()
    }

    /// The join requests read in the stream, for an administrator to admit ([`Team::admit`]).
    pub fn requests(&self) -> &[JoinRequest] {
        &self.meta.requests
    }

    /// How many messages wait to go to the server.
    pub fn outbox_len(&self) -> usize {
        self.meta.outbox.len()
    }

    pub fn client(&self) -> &Arc<ServerClient> {
        &self.client
    }

    /// The MLS device (the tests play a device or a server that breaks the rules with it).
    #[doc(hidden)]
    pub fn device_mut(&mut self) -> &mut TeamDevice<TeamProvider> {
        &mut self.device
    }

    #[doc(hidden)]
    pub fn device(&self) -> &TeamDevice<TeamProvider> {
        &self.device
    }

    /// The live feed of the team: on [`crate::LiveEvent::Connected`], [`crate::LiveEvent::Team`] or
    /// [`crate::LiveEvent::Lagged`], the caller reads the stream ([`Team::sync`]).
    pub fn live(&self) -> LiveFeed {
        self.client.live()
    }

    // === The state, a transaction per step ===

    fn storage(&self) -> &SqliteStorage {
        self.device.provider().storage()
    }

    /// Runs `f` in a transaction of the state: kept with what it changed, or rolled back with the device, which is
    /// loaded again from the disk.
    fn step<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, Error>) -> Result<T, Error> {
        self.storage().begin()?;
        let done = f(self).and_then(|t| {
            self.save()?;
            Ok(t)
        });
        match done {
            Ok(t) => {
                self.storage().commit()?;
                self.after_commit()?;
                Ok(t)
            }
            Err(e) => {
                let _ = self.storage().rollback();
                self.reload()?;
                Err(e)
            }
        }
    }

    fn save(&mut self) -> Result<(), Error> {
        let storage = self.device.provider().storage();
        storage.put_ours(b"state", &self.device.saved_state())?;
        storage.put_ours(b"meta", &self.meta)?;
        let mut new = Vec::new();
        for record in self.device.records() {
            if !self.stored.contains(&record.epoch) {
                storage.put_ours(&record_name(record.epoch), record)?;
                new.push(record.epoch);
            }
        }
        self.stored.extend(new);
        Ok(())
    }

    /// After a transaction: a new epoch compacts the journal (what it erased leaves the disk), and moves the
    /// keychain's checkpoint, after the state (a stop between the two leaves the state ahead, never behind).
    fn after_commit(&mut self) -> Result<(), Error> {
        let Some(cp) = self.device.checkpoint() else {
            return Ok(());
        };
        if self.saved != Some(cp) {
            self.storage().compact()?;
            self.setup.vault.put_checkpoint(&self.id, &cp)?;
            self.saved = Some(cp);
        }
        Ok(())
    }

    fn reload(&mut self) -> Result<(), Error> {
        let (device, meta, stored) = load(&self.dir, &self.keys)?;
        self.device = device;
        self.meta = meta;
        self.stored = stored;
        Ok(())
    }

    fn queue(&mut self, p: Publication) {
        if let Some((invitation, delivery)) = p.welcome {
            self.meta.outbox.push(TeamMessage::Welcome {
                invitation,
                delivery,
            });
        }
        if let Some(link) = p.history {
            self.meta.links.insert(link.epoch);
            self.meta.outbox.push(TeamMessage::History(link));
        }
        if let Some(w) = p.recovery {
            self.meta.outbox.push(TeamMessage::Recovery(w));
        }
        if let Some(w) = p.escrow {
            self.meta.outbox.push(TeamMessage::Escrow(w));
        }
    }

    /// Sends what waits, in order; what the server does not take stays for the next time.
    pub async fn flush(&mut self) -> Result<(), Error> {
        while let Some(message) = self.meta.outbox.first().cloned() {
            self.client.append_message(&wire::encode(&message)).await?;
            self.step(|t| {
                t.meta.outbox.remove(0);
                Ok(())
            })?;
        }
        Ok(())
    }

    // === Reading the stream ===

    /// Reads the team's stream from the last number read: commits, links, beacons, requests, Welcomes. Then reads
    /// the history back, writes the links that are missing, sends a beacon when one is due, and empties the outbox.
    pub async fn sync(&mut self) -> Result<Synced, Error> {
        self.sync_at(now()).await
    }

    /// [`Team::sync`] at the hour `now` (seconds), for the beacons.
    pub async fn sync_at(&mut self, now: u64) -> Result<Synced, Error> {
        self.check_last().await?;
        let mut report = Synced::default();
        loop {
            let page = self.client.read_team(self.meta.cursor, None).await?;
            for entry in page.entries {
                self.step(|t| t.take(entry, &mut report))?;
            }
            if !page.more {
                break;
            }
        }
        if self.device.group_id().is_ok() {
            self.step(|t| t.read_back())?;
            self.step(|t| {
                t.write_missing_links()?;
                t.beacon_if_due(now)
            })?;
        }
        self.flush().await?;
        Ok(report)
    }

    /// The last entry this device read must still be there, the same: a server that hid it, or the ones before,
    /// rolled back; one that changed it lies.
    async fn check_last(&self) -> Result<(), Error> {
        let Some((seq, digest)) = self.meta.last else {
            return Ok(());
        };
        let page = self.client.read_team(seq - 1, Some(1)).await?;
        match page.entries.first() {
            Some(e) if e.seq == seq && entry_digest(e) == digest => Ok(()),
            Some(e) if e.seq == seq => Err(Error::Rewritten { seq }),
            _ => Err(Error::ServerRollback { seen: seq }),
        }
    }

    /// Takes one entry of the team's stream.
    fn take(&mut self, entry: Entry, report: &mut Synced) -> Result<(), Error> {
        if entry.seq <= self.meta.cursor {
            return Err(Error::Server(format!(
                "the server numbers the stream backwards ({} after {})",
                entry.seq, self.meta.cursor
            )));
        }
        report.entries += 1;
        match entry.kind {
            Kind::Commit => {
                let epoch = entry
                    .epoch
                    .ok_or_else(|| Error::Server("a commit without its epoch".into()))?;
                self.take_commit(epoch, &entry.payload, report)?;
            }
            Kind::Message => match wire::decode::<TeamMessage>(&entry.payload) {
                Ok(message) => self.take_message(message, report)?,
                Err(e) => log::debug!("team: a message of the stream not understood: {e}"),
            },
        }
        self.meta.cursor = entry.seq;
        self.meta.last = Some((entry.seq, entry_digest(&entry)));
        Ok(())
    }

    fn take_commit(&mut self, epoch: u64, commit: &[u8], report: &mut Synced) -> Result<(), Error> {
        let d = digest(commit);
        if let Some(seen) = self.meta.commits.get(&epoch) {
            if *seen != d {
                return Err(CryptoError::Fork { epoch: epoch + 1 }.into());
            }
            return Ok(());
        }
        if self.device.is_removed() {
            self.meta.commits.insert(epoch, d);
            return Ok(());
        }
        let Ok(current) = self.device.epoch() else {
            // Not in the group yet: kept for when the Welcome comes.
            self.meta.commits.insert(epoch, d);
            self.meta.early.insert(epoch, commit.to_vec());
            return Ok(());
        };
        if epoch < current {
            // Before this device joined: nothing to check it against.
            self.meta.commits.insert(epoch, d);
            return Ok(());
        }
        if epoch > current {
            return Err(Error::Server(format!(
                "the server skipped the commits from epoch {current} to {epoch}"
            )));
        }
        if let Some(pending) = self.device.pending() {
            if pending.from_epoch == epoch && pending.commit == commit {
                // This device's own, sent without hearing back: the server has it.
                let rest = self.device.confirm_pending()?;
                self.queue(rest);
                self.meta.commits.insert(epoch, d);
                report.commits += 1;
                return Ok(());
            }
            self.device.abandon_pending()?;
        }
        let processed = self.device.process_commit(commit)?;
        self.meta.commits.insert(epoch, d);
        report.commits += 1;
        if processed == Processed::Removed {
            report.removed = true;
        }
        Ok(())
    }

    fn take_message(&mut self, message: TeamMessage, report: &mut Synced) -> Result<(), Error> {
        match message {
            TeamMessage::History(link) => {
                self.meta.links.insert(link.epoch);
                let usable = self.device.record(link.epoch).is_some();
                if usable {
                    match self.device.learn_history(&link) {
                        Ok(_) => return Ok(()),
                        Err(e @ CryptoError::Fork { .. }) => return Err(e.into()),
                        Err(e) => log::debug!("team: a history link refused: {e}"),
                    }
                }
                // For later: an epoch this device does not hold yet (its history, read back).
                let mut name = link_prefix(link.epoch);
                name.extend_from_slice(&random::<8>());
                self.storage().put_ours(&name, &link)?;
            }
            TeamMessage::Beacon(beacon) => {
                if self.device.group_id().is_ok() && !self.device.is_removed() {
                    match self.device.check_beacon(&beacon) {
                        Err(e @ CryptoError::Fork { .. }) => return Err(e.into()),
                        Err(e) => log::debug!("team: a beacon set aside: {e}"),
                        Ok(_) => {}
                    }
                }
            }
            TeamMessage::JoinRequest(request) => {
                if !self.meta.requests.contains(&request) {
                    self.meta.requests.push(request);
                }
            }
            TeamMessage::Welcome {
                invitation,
                delivery,
            } => self.take_welcome(invitation, &delivery, report)?,
            TeamMessage::Recovery(_) | TeamMessage::Escrow(_) => {}
        }
        Ok(())
    }

    fn take_welcome(
        &mut self,
        invitation: [u8; 16],
        delivery: &WelcomeDelivery,
        report: &mut Synced,
    ) -> Result<(), Error> {
        let joining = self
            .device
            .saved_state()
            .joining
            .and_then(|url| InviteLink::parse(&url).ok());
        if joining.is_none_or(|l| l.id != invitation) || self.device.group_id().is_ok() {
            return Ok(());
        }
        if let Err(e) = self.device.accept_welcome(delivery) {
            // A false Welcome (the server's): the invitee asks again (`invite.rs`).
            log::warn!("team: a Welcome refused: {e}");
            return Ok(());
        }
        report.joined = true;
        // The commits that came before the Welcome but after its epoch.
        let early = std::mem::take(&mut self.meta.early);
        let current = self.device.epoch()?;
        for (epoch, commit) in early.range(current..) {
            if *epoch != self.device.epoch()? {
                return Err(Error::Server("a gap in the early commits".into()));
            }
            if self.device.process_commit(commit)? == Processed::Removed {
                report.removed = true;
                break;
            }
            report.commits += 1;
        }
        Ok(())
    }

    /// Reads the history back as far as the links on the disk allow.
    fn read_back(&mut self) -> Result<(), Error> {
        loop {
            let Some(oldest) = self.device.known_epochs().first().copied() else {
                return Ok(());
            };
            if oldest == 0 {
                return Ok(());
            }
            let keys = self
                .storage()
                .keys_with(&crate::storage::ours(&link_prefix(oldest)))?;
            let mut learnt = false;
            for key in keys {
                let name = &key[b"escouade-team/".len()..];
                let Some(link) = self
                    .storage()
                    .get_ours::<escouade_team_crypto::history::HistoryLink>(name)?
                else {
                    continue;
                };
                match self.device.learn_history(&link) {
                    Ok(_) => {
                        learnt = true;
                        break;
                    }
                    Err(e @ CryptoError::Fork { .. }) => return Err(e.into()),
                    Err(e) => log::debug!("team: a history link refused: {e}"),
                }
            }
            if !learnt {
                return Ok(());
            }
        }
    }

    /// The links of the epochs this device holds with the one before, that nobody wrote.
    fn write_missing_links(&mut self) -> Result<(), Error> {
        let epochs = self.device.known_epochs();
        let held: BTreeSet<u64> = epochs.iter().copied().collect();
        for e in epochs {
            if e > 0 && held.contains(&(e - 1)) && !self.meta.links.contains(&e) {
                let link = self.device.history_link(e)?;
                self.meta.links.insert(e);
                self.meta.outbox.push(TeamMessage::History(link));
            }
        }
        Ok(())
    }

    fn beacon_if_due(&mut self, now: u64) -> Result<(), Error> {
        if self.device.is_removed() {
            return Ok(());
        }
        let epoch = self.device.epoch()?;
        let due = match self.meta.beacon {
            Some((e, at)) => {
                e != epoch || now.saturating_sub(at) >= self.setup.options.beacon_every
            }
            None => true,
        };
        if due {
            let beacon = self.device.beacon(now)?;
            self.meta.beacon = Some((epoch, now));
            self.meta.outbox.push(TeamMessage::Beacon(beacon));
        }
        Ok(())
    }

    // === Changing the group ===

    /// Applies a change: staged, sent, merged once the server took it; on 409 `epoch_taken`, dropped, the winner
    /// read, and staged again if it is still to do.
    pub async fn apply(&mut self, change: Change) -> Result<Applied, Error> {
        for _ in 0..TRIES {
            self.sync().await?;
            if self.device.is_removed() {
                return Err(CryptoError::NotInTeam.into());
            }
            if self.device.is_done(&change)? {
                return Ok(Applied::AlreadyDone);
            }
            let (epoch, commit) = self.step(|t| {
                let staged = t.device.stage(&change)?;
                staged.commit.ok_or_else(|| {
                    Error::Crypto(CryptoError::Mls("a change without its commit".into()))
                })
            })?;
            match self.client.append_commit(epoch, &commit).await {
                Ok(_) => {
                    self.step(|t| {
                        let rest = t.device.confirm_pending()?;
                        t.meta.commits.insert(epoch, digest(&commit));
                        t.queue(rest);
                        Ok(())
                    })?;
                    self.flush().await?;
                    return Ok(Applied::Done { epoch: epoch + 1 });
                }
                Err(ClientError::EpochTaken(_)) => {
                    // The commit that won comes with the next reading.
                    self.step(|t| Ok(t.device.abandon_pending()?))?;
                }
                // Sent or not, it stays pending: the next reading finds out which.
                Err(e) => return Err(e.into()),
            }
        }
        Err(Error::Busy)
    }

    /// "Change the team's keys" (after an incident): a new epoch, the old content not encrypted again (C8-A).
    pub async fn rotate(&mut self) -> Result<Applied, Error> {
        self.apply(Change::Rotate).await
    }

    pub async fn set_role(&mut self, member: MemberId, role: Role) -> Result<Applied, Error> {
        self.apply(Change::SetRole(member, role)).await
    }

    /// Removes a member (an administrator): the server cuts their devices first (their sessions and live
    /// connections end), then the commit takes them out of the group and the roster, a new epoch that they do not
    /// have (the rotation of a departure, C8-A).
    pub async fn remove_member(&mut self, member: MemberId) -> Result<Applied, Error> {
        self.sync().await?;
        if !self.device.is_admin()? {
            return Err(CryptoError::NotAdmin.into());
        }
        if member == self.identity().member {
            return Err(Error::Refused(
                "a member does not remove themself here".into(),
            ));
        }
        let devices: Vec<DeviceId> = self
            .device
            .leaves()?
            .into_iter()
            .filter(|i| i.member == member && i.kind == DeviceKind::Device)
            .map(|i| i.device)
            .collect();
        for d in devices {
            self.cut(&d).await?;
        }
        self.apply(Change::RemoveMember(member)).await
    }

    /// Revokes a device (a stolen one): an administrator's, or its member's. The server cuts it first, then the
    /// commit takes it out of the group. The server lets only an administrator revoke another device: a member's
    /// own other device is cut by the commit alone (TS-3 gives the server the members' devices).
    pub async fn revoke_device(&mut self, device: DeviceId) -> Result<Applied, Error> {
        self.sync().await?;
        if device == self.identity().device {
            return Err(Error::Refused(
                "a device does not revoke itself here".into(),
            ));
        }
        let leaf = self
            .device
            .leaves()?
            .into_iter()
            .find(|i| i.device == device)
            .ok_or_else(|| Error::Refused("no such device in the team".into()))?;
        let admin = self.device.is_admin()?;
        if !admin && leaf.member != self.identity().member {
            return Err(CryptoError::NotAdmin.into());
        }
        self.cut(&device).await?;
        self.apply(Change::RemoveDevice(device)).await
    }

    /// Cuts a device on the server (its sessions and live connections end). The server lets its administrators
    /// alone do it: a device that is an administrator in the roster but not on the server (an administrator named
    /// after it enrolled, or a member revoking their own other device) leaves it to the commit, which takes the
    /// device out of the group all the same (TS-3 keeps the server's roles in step with the roster).
    async fn cut(&self, device: &DeviceId) -> Result<(), Error> {
        match self.client.revoke(&device.0).await {
            Ok(_) => Ok(()),
            Err(ClientError::Refused { status: 403, .. }) => {
                log::info!("team: the server lets its administrators alone cut a device; the commit does it");
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    // === Invitations (the entry point of TS-3) ===

    /// Makes an invitation: its link (the secret after the `#`), and, for an administrator, the server's enrolment
    /// token for the invitee's device. The server gives that token to an administrator alone: a member who adds
    /// a device of their own gets it from one (TS-3 makes it a request in the stream).
    pub async fn invite(
        &mut self,
        purpose: Purpose,
        now: u64,
    ) -> Result<(InviteLink, Option<String>), Error> {
        let server = self.setup.server.clone();
        let link =
            self.step(|t| Ok(t.device.invite(purpose, now, DEFAULT_LIFETIME, &server)?))?;
        let token = if self.device.is_admin()? {
            Some(self.client.enrollment_token(None).await?)
        } else {
            None
        };
        Ok((link, token))
    }

    /// Admits a join request read in the stream (an administrator, or the member for their own device).
    pub async fn admit(&mut self, request: &JoinRequest, now: u64) -> Result<Applied, Error> {
        let applied = self.apply(Change::Admit(request.clone(), now)).await?;
        self.step(|t| {
            t.meta.requests.retain(|r| r != request);
            Ok(())
        })?;
        Ok(applied)
    }

    // === Content ===

    /// Writes an object in a project's stream: encrypted under the project's key of the current epoch, signed.
    pub async fn write_object(
        &mut self,
        project: ProjectId,
        object: ObjectId,
        version: u64,
        plaintext: &[u8],
    ) -> Result<u64, Error> {
        let sealed = self.device.seal(project, object, version, plaintext)?;
        Ok(self
            .client
            .append_project(&project.0, &wire::encode(&sealed))
            .await?)
    }

    /// The objects of a project's stream after `since`, opened, with their numbers. One that does not open is its
    /// error; a fork stops the reading.
    pub async fn read_objects(
        &mut self,
        project: ProjectId,
        since: u64,
    ) -> Result<Vec<(u64, Result<Vec<u8>, Error>)>, Error> {
        let mut out = Vec::new();
        let mut since = since;
        loop {
            let page = self.client.read_project(&project.0, since, None).await?;
            for e in page.entries {
                since = e.seq;
                let opened = match wire::decode::<SealedObject>(&e.payload) {
                    Ok(sealed) if sealed.header.project == project => {
                        match self.device.open(&sealed) {
                            Err(e @ CryptoError::Fork { .. }) => return Err(e.into()),
                            r => r.map_err(Error::from),
                        }
                    }
                    Ok(_) => Err(Error::Refused("an object of another project".into())),
                    Err(e) => Err(Error::Encoding(e)),
                };
                out.push((e.seq, opened));
            }
            if !page.more {
                return Ok(out);
            }
        }
    }

    /// Writes an encrypted file (a document, an image) under its id: sealed as an object of `project`.
    pub async fn put_file(
        &mut self,
        project: ProjectId,
        file: ObjectId,
        plaintext: &[u8],
    ) -> Result<(), Error> {
        let sealed = self.device.seal(project, file, 1, plaintext)?;
        Ok(self.client.put_file(&file.0, wire::encode(&sealed)).await?)
    }

    /// A file, opened; None when the server has none.
    pub async fn get_file(&mut self, file: ObjectId) -> Result<Option<Vec<u8>>, Error> {
        let Some(bytes) = self.client.get_file(&file.0).await? else {
            return Ok(None);
        };
        let sealed: SealedObject = wire::decode(&bytes).map_err(Error::Encoding)?;
        if sealed.header.object != file {
            return Err(Error::Refused("the server gave another file".into()));
        }
        Ok(Some(self.device.open(&sealed)?))
    }

    pub async fn delete_file(&mut self, file: ObjectId) -> Result<(), Error> {
        Ok(self.client.delete_file(&file.0).await?)
    }
}

fn identity(keys: &DeviceKeys) -> Identity {
    Identity {
        member: MemberId(keys.member),
        device: DeviceId(keys.device),
        kind: DeviceKind::Device,
    }
}

fn signer(keys: &DeviceKeys) -> SignatureKeyPair {
    SignatureKeyPair::from_raw(
        CIPHERSUITE.signature_algorithm(),
        keys.mls.to_vec(),
        SigningKey::from_bytes(&keys.mls)
            .verifying_key()
            .to_bytes()
            .to_vec(),
    )
}

fn open_storage(dir: &Path, keys: &DeviceKeys) -> Result<SqliteStorage, Error> {
    Ok(SqliteStorage::open(&dir.join(STATE_FILE), keys.state_key)?)
}

/// The device, its meta and the epochs of its records, from the disk.
fn load(
    dir: &Path,
    keys: &DeviceKeys,
) -> Result<(TeamDevice<TeamProvider>, Meta, BTreeSet<u64>), Error> {
    if !dir.join(STATE_FILE).exists() {
        return Err(Error::NoState);
    }
    let storage = open_storage(dir, keys)?;
    let state: SavedState = storage.get_ours(b"state")?.unwrap_or_default();
    let meta: Meta = storage.get_ours(b"meta")?.unwrap_or_default();
    let mut records: Vec<EpochRecord> = Vec::new();
    for key in storage.keys_with(&crate::storage::ours(b"record/"))? {
        let name = &key[b"escouade-team/".len()..];
        if let Some(r) = storage.get_ours::<EpochRecord>(name)? {
            records.push(r);
        }
    }
    let stored = records.iter().map(|r| r.epoch).collect();
    let device = TeamDevice::restore(
        TeamProvider::new(storage),
        signer(keys),
        identity(keys),
        state,
        records,
    )?;
    Ok((device, meta, stored))
}
