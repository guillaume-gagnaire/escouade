//! The synchronization of a team's projects (TS-5, `docs/TEAM-SERVER.md` §2.4, §2.5, §3.6 and §3.8).
//!
//! - **Writing** ([`Team::write`] and the tickets' operations): the operation is made on this device (its place in
//!   the device's chain, the members' head it knows, its stamp), applied to the local copy at once, and put in the
//!   outbox. Nothing waits for the server: offline, it waits there, in its order.
//! - **Sending** ([`Team::flush_project`]): each operation of the outbox, in order, signed and sealed at the current
//!   epoch of its project (a member gone since it was written never reads it), then sent. The server's number is
//!   kept until the operation is read back: one the server drops in silence shows ([`Error::Withheld`]).
//! - **Reading** ([`Team::sync_project`]), from the last number seen: the last entry read is read again first (a
//!   rollback, an entry changed); each operation opened (its author a leaf of its epoch, its signature), its
//!   members' head checked (a fork), its place in its device's chain checked (played again, reordered, dropped,
//!   two for one place), then applied. The commits of a project's own group are checked and merged on the way.
//! - **Snapshots**: past [`super::Options::snapshot_every`] entries after the last one, a device of the team seals
//!   and signs the state, and the server forgets the operations before it. A device that comes later (or that was
//!   away past it) starts from the snapshot, its signature and its chains checked against what it saw.

use std::collections::BTreeSet;

use escouade_team_crypto::hlc::Hlc;
use escouade_team_crypto::ops::{Chain, OpBody, Operation, Scope, Sealed, Snapshot, Value, What};
use escouade_team_crypto::{DeviceId, HeadCheck, MemberId, ObjectId, Processed, ProjectId};

use super::{project_link_prefix, random, Team, TRIES};
use crate::client::ClientError;
use crate::replica::{Fence, ProjectMeta, Replica};
use crate::wire::{self, Entry, Kind, TeamMessage};
use crate::{CryptoError, Error};

/// What a reading of a project's stream did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectSynced {
    /// The entries read.
    pub entries: usize,
    /// The operations new to the copy.
    pub applied: usize,
    /// The commits of the project's own group merged.
    pub commits: usize,
    /// The copy started again from a snapshot.
    pub from_snapshot: bool,
    /// The operations of this device sent.
    pub sent: usize,
    /// This device wrote a snapshot.
    pub snapshot_made: bool,
}

/// Writes of a ticket's holder the server refused (TS-6): another device took the ticket over meanwhile (a device
/// back from a cut). They are dropped from the outbox and from the copy, which goes back to what the journal says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub project: ProjectId,
    /// The claim as the server holds it now (its holder, its token).
    pub claim: wire::Claim,
    /// The operations dropped.
    pub dropped: usize,
}

/// What the status bar says (§2.5): « Hors ligne · 3 changements en attente ».
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncState {
    /// The server answered the last call (None: not tried yet).
    pub online: Option<bool>,
    /// The operations of this device waiting to go, every project together.
    pub pending: usize,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn entry_digest(e: &Entry) -> [u8; 32] {
    super::digest(&wire::encode(e))
}

/// The error that says the team is to be read first: the operation cites a later epoch.
fn behind(e: &Error) -> bool {
    matches!(e, Error::Crypto(CryptoError::Behind(_)))
}

impl Team {
    // === The local copies ===

    fn replica_path(&self, project: &ProjectId) -> std::path::PathBuf {
        self.dir
            .join("projects")
            .join(format!("{}.sqlite3", hex::encode(project.0)))
    }

    /// The projects whose copy is on this device.
    pub fn projects(&self) -> Vec<ProjectId> {
        let mut out: BTreeSet<ProjectId> = self.replicas.keys().copied().collect();
        if let Ok(dir) = std::fs::read_dir(self.dir.join("projects")) {
            for e in dir.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if let Some(id) = name.strip_suffix(".sqlite3") {
                    let mut p = [0u8; 16];
                    if hex::decode_to_slice(id, &mut p).is_ok() {
                        out.insert(ProjectId(p));
                    }
                }
            }
        }
        out.extend(self.device.project_ids());
        out.into_iter().collect()
    }

    pub(super) fn open_replica(&mut self, project: &ProjectId) -> Result<&Replica, Error> {
        if !self.replicas.contains_key(project) {
            let r = Replica::open(&self.replica_path(project))?;
            self.replicas.insert(*project, r);
        }
        Ok(&self.replicas[project])
    }

    /// The local copy of a project, which the interface reads (opened, made empty for a project this device follows
    /// from now on).
    pub fn replica(&mut self, project: &ProjectId) -> Result<&Replica, Error> {
        self.open_replica(project)
    }

    /// Follows a project of the team on this device (its copy made, read at the next [`Team::sync_project`]).
    pub fn follow(&mut self, project: ProjectId) -> Result<(), Error> {
        self.open_replica(&project).map(|_| ())
    }

    /// Whose key seals a project: its own group's when this device was ever in it, else as the copy saw it.
    fn scope_of(&mut self, project: &ProjectId) -> Result<Option<Scope>, Error> {
        if self.device.project_ids().contains(project) {
            return Ok(Some(Scope::Project));
        }
        Ok(self.open_replica(project)?.meta()?.scope)
    }

    // === The status ===

    /// What waits to go, and whether the server answers: « Hors ligne · N changements en attente ».
    pub fn sync_state(&mut self) -> SyncState {
        let mut pending = self.meta.outbox.len();
        for p in self.projects() {
            if let Ok(r) = self.open_replica(&p) {
                pending += r.pending().unwrap_or(0);
            }
        }
        SyncState {
            online: self.online,
            pending,
        }
    }

    /// The operations of a project waiting to go.
    pub fn pending(&mut self, project: &ProjectId) -> Result<usize, Error> {
        self.open_replica(project)?.pending()
    }

    /// Notes whether the server answered.
    pub(super) fn heard<T>(&mut self, r: Result<T, ClientError>) -> Result<T, Error> {
        match r {
            Ok(t) => {
                self.online = Some(true);
                Ok(t)
            }
            Err(e @ ClientError::Net(_)) => {
                self.online = Some(false);
                Err(e.into())
            }
            // The server keeps the project to other devices (TS-6).
            Err(e) if e.code() == Some("no_access") => {
                self.online = Some(true);
                Err(Error::NoAccess)
            }
            Err(e) => {
                self.online = Some(true);
                Err(e.into())
            }
        }
    }

    // === Making a project ===

    /// Makes a project of the team: of the whole team (`None`: its key is the team's), or kept to `members` (its own
    /// MLS group, an administrator's: the devices of those members whose KeyPackage is known are added now, the
    /// others when theirs comes). Its id is drawn here, opaque.
    pub async fn create_project(
        &mut self,
        members: Option<BTreeSet<MemberId>>,
    ) -> Result<ProjectId, Error> {
        let project = ProjectId(random::<16>());
        match members {
            None => {
                let r = self.open_replica(&project)?;
                let mut meta = r.meta()?;
                meta.scope = Some(Scope::Team);
                r.set_meta(&meta)?;
                // Its first operation says the project is the whole team's: a device that follows it learns so before
                // it writes (never under the team's key in a project kept to some members).
                self.write(
                    project,
                    OpBody::Create {
                        object: ObjectId(project.0),
                        kind: "project".into(),
                        fields: Default::default(),
                    },
                )?;
            }
            Some(members) => {
                let publication = self.step(|t| {
                    Ok(t.device
                        .create_project(project, members, &t.meta.key_packages)?)
                })?;
                let r = self.open_replica(&project)?;
                let mut meta = r.meta()?;
                meta.scope = Some(Scope::Project);
                r.set_meta(&meta)?;
                // Kept to its members on the server before anything is written in it (TS-6): its first write would
                // open it to the whole team.
                self.sync_access(project).await?;
                if let Some((epoch, commit)) = publication.commit {
                    self.send_project_commit(project, epoch, commit).await?;
                }
            }
        }
        Ok(project)
    }

    /// Who has access to a project kept to some members (an administrator changes it: [`Team::set_project_members`]).
    pub fn project_members(&self, project: &ProjectId) -> Result<BTreeSet<MemberId>, Error> {
        Ok(self.device.project_roster(project)?.members)
    }

    /// Gives or takes access to a project kept to some members (an administrator of the team): the devices of the
    /// members added join its group, those of the members taken out leave it, at a new epoch.
    pub async fn set_project_members(
        &mut self,
        project: ProjectId,
        members: BTreeSet<MemberId>,
    ) -> Result<bool, Error> {
        self.project_in_step(project, Some(members)).await
    }

    /// Keeps a project's own group in step with the team (and with `members` when given): stages, sends, merges;
    /// on 409, reads the project's stream (the commit that won) and stages again. False when there was nothing to do.
    pub async fn project_in_step(
        &mut self,
        project: ProjectId,
        members: Option<BTreeSet<MemberId>>,
    ) -> Result<bool, Error> {
        for _ in 0..TRIES {
            if !self.device.in_project(&project) {
                return Err(Error::NoAccess);
            }
            if self.device.project_pending(&project).is_some() {
                // Sent without an answer: the stream says how it went.
                self.read_project(project).await?;
                continue;
            }
            let staged = self.step(|t| {
                Ok(t.device
                    .project_stage(&project, members.clone(), &t.meta.key_packages)?)
            })?;
            let Some(publication) = staged else {
                return Ok(false);
            };
            let (epoch, commit) = publication
                .commit
                .ok_or_else(|| Error::Server("a staged commit without its commit".into()))?;
            if self.send_project_commit(project, epoch, commit).await? {
                // Who has access on the server follows the group (TS-6): an administrator's device says it.
                if self.device.is_admin().unwrap_or(false) {
                    self.sync_access(project).await?;
                }
                return Ok(true);
            }
        }
        Err(Error::Busy)
    }

    /// Sends a project's staged commit: merged once the server took it (its Welcome and history link then go in the
    /// team's stream); dropped on 409 (false), the winner then read.
    async fn send_project_commit(
        &mut self,
        project: ProjectId,
        epoch: u64,
        commit: Vec<u8>,
    ) -> Result<bool, Error> {
        let sent = self
            .client
            .append_project_commit(&project.0, epoch, &commit)
            .await;
        match self.heard(sent) {
            Ok(_) => {
                self.step(|t| {
                    let rest = t.device.project_confirm(&project)?;
                    t.queue_project(project, rest);
                    Ok(())
                })?;
                self.flush().await?;
                Ok(true)
            }
            Err(Error::Client(ClientError::EpochTaken(_))) => {
                self.step(|t| Ok(t.device.project_abandon(&project)?))?;
                self.read_project(project).await?;
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    pub(super) fn queue_project(
        &mut self,
        project: ProjectId,
        p: escouade_team_crypto::project::ProjectPublication,
    ) {
        if let Some(welcome) = p.welcome {
            self.meta.outbox.push(TeamMessage::ProjectWelcome {
                project: project.0,
                welcome,
            });
        }
        if let Some(link) = p.history {
            self.meta.outbox.push(TeamMessage::ProjectHistory {
                project: project.0,
                link,
            });
        }
    }

    /// Reads back the history of the projects' own groups as far as the links of the team's stream allow.
    pub(super) fn read_back_projects(&mut self) -> Result<(), Error> {
        for project in self.device.project_ids() {
            while let Some(oldest) = self.device.project_epochs(&project).first().copied() {
                if oldest == 0 {
                    break;
                }
                let keys =
                    self.storage()
                        .keys_with(&crate::storage::ours(&project_link_prefix(
                            &project, oldest,
                        )))?;
                let mut learnt = false;
                for key in keys {
                    let name = &key[b"escouade-team/".len()..];
                    let Some(link) = self
                        .storage()
                        .get_ours::<escouade_team_crypto::history::HistoryLink>(name)?
                    else {
                        continue;
                    };
                    match self.device.project_learn_history(&project, &link) {
                        Ok(_) => {
                            learnt = true;
                            break;
                        }
                        Err(e @ CryptoError::Fork { .. }) => return Err(e.into()),
                        Err(e) => log::debug!("team: a project's history link refused: {e}"),
                    }
                }
                if !learnt {
                    break;
                }
            }
        }
        Ok(())
    }

    // === Writing ===

    /// Writes an operation on this device: applied to its copy at once, put in the outbox (sent by
    /// [`Team::flush_project`] or the next [`Team::sync_project`]). Offline, it waits.
    pub fn write(&mut self, project: ProjectId, body: OpBody) -> Result<(), Error> {
        self.write_fenced(project, body, None)
    }

    /// The same, for a write of a ticket's holder (TS-6): it carries the claim and its token, and the server refuses
    /// it once another device took the ticket over ([`Team::take_refused`] then says so).
    pub fn write_fenced(
        &mut self,
        project: ProjectId,
        body: OpBody,
        fence: Option<Fence>,
    ) -> Result<(), Error> {
        let Some(scope) = self.scope_of(&project)? else {
            // Nothing read of it yet says whose key seals it: read it first ([`Team::sync_project`]).
            return Err(if self.open_replica(&project)?.meta()?.cursor > 0 {
                Error::NoAccess
            } else {
                Error::NoProject
            });
        };
        if scope == Scope::Project && !self.device.in_project(&project) {
            return Err(Error::NoAccess);
        }
        let checkpoint = self.device.checkpoint().ok_or(CryptoError::NotInTeam)?;
        let author = self.device.author();
        let device = author.device;
        let r = self.open_replica(&project)?;
        r.atomically(|r| {
            let mut meta = r.meta()?;
            let last = [meta.own, meta.chains.get(&device).copied()]
                .into_iter()
                .flatten()
                .max_by_key(|c| c.dseq);
            let op = Operation {
                project,
                author,
                dseq: last.map_or(1, |c| c.dseq + 1),
                prev: last.map_or([0; 32], |c| c.digest),
                team_epoch: checkpoint.epoch,
                team_head: checkpoint.head,
                hlc: meta.clock.tick(now_ms(), device),
                body,
            };
            let bytes = op.to_bytes();
            let digest = Operation::digest_of(&bytes);
            r.apply(&op, &digest, None)?;
            r.queue_fenced(&digest, &bytes, fence.as_ref())?;
            meta.own = Some(Chain {
                dseq: op.dseq,
                digest,
            });
            r.set_meta(&meta)
        })
    }

    /// Creates a ticket with its first fields (`title`, `description`, `column`…): its id, opaque, drawn here.
    pub fn create_ticket(
        &mut self,
        project: ProjectId,
        fields: std::collections::BTreeMap<String, Value>,
    ) -> Result<ObjectId, Error> {
        let object = ObjectId(random::<16>());
        self.write(
            project,
            OpBody::Create {
                object,
                kind: "ticket".into(),
                fields,
            },
        )?;
        Ok(object)
    }

    /// Changes one field of an object, after the value this device sees (its base: a concurrent write is a conflict
    /// kept to show).
    pub fn set_field(
        &mut self,
        project: ProjectId,
        object: ObjectId,
        field: &str,
        value: Value,
    ) -> Result<(), Error> {
        let base = self.open_replica(&project)?.field_stamp(&object, field)?;
        self.write(
            project,
            OpBody::Set {
                object,
                field: field.to_string(),
                value,
                base,
            },
        )
    }

    /// Moves a ticket to a column (and a place in it).
    pub fn move_ticket(
        &mut self,
        project: ProjectId,
        object: ObjectId,
        column: &str,
        rank: Option<String>,
    ) -> Result<(), Error> {
        let base = self
            .open_replica(&project)?
            .field_stamp(&object, "column")?;
        self.write(
            project,
            OpBody::Move {
                object,
                column: column.to_string(),
                rank,
                base,
            },
        )
    }

    pub fn delete_ticket(&mut self, project: ProjectId, object: ObjectId) -> Result<(), Error> {
        self.write(project, OpBody::Delete { object })
    }

    /// Writes a whole version of a document (a spec page) after `bases`: its id.
    pub fn write_version(
        &mut self,
        project: ProjectId,
        object: ObjectId,
        bases: Vec<ObjectId>,
        content: Value,
    ) -> Result<ObjectId, Error> {
        let version = ObjectId(random::<16>());
        self.write(
            project,
            OpBody::Version {
                object,
                version,
                bases,
                content,
            },
        )?;
        Ok(version)
    }

    // === Sending ===

    /// Sends a project's outbox, in order, each operation sealed at the current epoch of its project. Offline, what
    /// is left stays for the next time (and the status says so). A project kept to some members is put in step with
    /// the team first: the devices gone leave its group before anything new is sealed.
    pub async fn flush_project(&mut self, project: ProjectId) -> Result<usize, Error> {
        let scope = self.scope_of(&project)?.ok_or(Error::NoProject)?;
        if self.open_replica(&project)?.pending()? == 0 {
            return Ok(0);
        }
        if scope == Scope::Project {
            self.project_in_step(project, None).await?;
        }
        let mut sent = 0;
        loop {
            let Some((n, digest, bytes)) = self.open_replica(&project)?.outbox_first()? else {
                return Ok(sent);
            };
            let op = Operation::from_bytes(&bytes)?;
            let fence = self.open_replica(&project)?.outbox_fence(n)?;
            let sealed = self
                .device
                .seal_for(What::Operation, &project, scope, bytes)?;
            let answer = self
                .client
                .append_project_fenced(
                    &project.0,
                    &wire::encode(&sealed),
                    fence.map(|f| wire::FenceWire {
                        ticket: f.claim.to_vec(),
                        token: f.token,
                    }),
                )
                .await;
            let seq = match self.heard(answer) {
                Ok(seq) => seq,
                Err(Error::Client(ClientError::Net(_))) => return Ok(sent),
                Err(Error::Client(ClientError::Claim { code, claim })) if code == "stale_token" => {
                    let stale = fence.expect("a refused fence was sent");
                    self.drop_stale(project, stale, *claim)?;
                    continue;
                }
                Err(e) => return Err(e),
            };
            self.open_replica(&project)?.atomically(|r| {
                r.outbox_remove(n)?;
                r.apply(&op, &digest, Some(seq))?;
                let mut meta = r.meta()?;
                meta.sent.insert(
                    seq,
                    Chain {
                        dseq: op.dseq,
                        digest,
                    },
                );
                r.set_meta(&meta)
            })?;
            sent += 1;
        }
    }

    /// A holder's write refused: the outbox written again without the writes under that stale token (the device's
    /// chain has no hole: what is kept is made again after the last operation the server took), the copy replayed.
    fn drop_stale(
        &mut self,
        project: ProjectId,
        stale: Fence,
        claim: wire::Claim,
    ) -> Result<(), Error> {
        let device = self.device.author().device;
        let r = self.open_replica(&project)?;
        let taken = r.take_unsent()?;
        let mut meta = r.meta()?;
        meta.own = [meta.chains.get(&device).copied()]
            .into_iter()
            .chain(meta.sent.values().copied().map(Some))
            .flatten()
            .max_by_key(|c| c.dseq);
        r.set_meta(&meta)?;
        let mut dropped = 0;
        for (bytes, fence) in taken {
            if fence.is_some_and(|f| f.claim == stale.claim && f.token <= stale.token) {
                dropped += 1;
                continue;
            }
            let op = Operation::from_bytes(&bytes)?;
            self.write_fenced(project, op.body, fence)?;
        }
        log::info!("team: {dropped} write(s) of a ticket taken over dropped");
        self.refused.push(Refused {
            project,
            claim,
            dropped,
        });
        Ok(())
    }

    /// The writes of a holder the server refused since the last call (TS-6): the interface says so.
    pub fn take_refused(&mut self) -> Vec<Refused> {
        std::mem::take(&mut self.refused)
    }

    // === Reading ===

    /// Reads a project's stream from the last number seen, then sends its outbox and writes a snapshot when one is
    /// due. A project that cites a later team epoch than this device's reads the team first, then again.
    pub async fn sync_project(&mut self, project: ProjectId) -> Result<ProjectSynced, Error> {
        let mut report = match self.read_project(project).await {
            Err(e) if behind(&e) => {
                self.sync().await?;
                self.read_project(project).await?
            }
            r => r?,
        };
        report.sent = self.flush_project(project).await?;
        if report.sent > 0 {
            // What was sent read back at once: the chain checked, the server's numbers too.
            let again = self.read_project(project).await?;
            report.entries += again.entries;
        }
        report.snapshot_made = self.snapshot_if_due(project).await?;
        Ok(report)
    }

    /// Reads a project's stream from the last number seen (no sending).
    pub async fn read_project(&mut self, project: ProjectId) -> Result<ProjectSynced, Error> {
        self.open_replica(&project)?;
        self.check_last_project(project).await?;
        let mut report = ProjectSynced::default();
        loop {
            let meta = self.open_replica(&project)?.meta()?;
            let page = self
                .client
                .read_project(&project.0, meta.cursor, None)
                .await;
            let page = self.heard(page)?;
            if let Some(upto) = page.snapshot {
                if meta.snapshot.is_none_or(|s| s < upto) {
                    self.take_snapshot(project, upto).await?;
                    report.from_snapshot = true;
                }
            }
            for entry in page.entries {
                self.take_project_entry(project, entry, &mut report)?;
            }
            if !page.more {
                break;
            }
        }
        // An operation the server took and never showed: dropped.
        let meta = self.open_replica(&project)?.meta()?;
        if let Some((seq, _)) = meta.sent.iter().next() {
            return Err(Error::Withheld { seq: *seq });
        }
        Ok(report)
    }

    /// The last entry read must still be there, the same: unless a snapshot took its place.
    async fn check_last_project(&mut self, project: ProjectId) -> Result<(), Error> {
        let Some((seq, digest)) = self.open_replica(&project)?.meta()?.last else {
            return Ok(());
        };
        let page = self.client.read_project(&project.0, seq - 1, Some(1)).await;
        let page = self.heard(page)?;
        match page.entries.first() {
            Some(e) if e.seq == seq && entry_digest(e) == digest => Ok(()),
            Some(e) if e.seq == seq => Err(Error::Rewritten { seq }),
            _ if page.snapshot.is_some_and(|upto| upto >= seq) => Ok(()),
            _ => Err(Error::ServerRollback { seen: seq }),
        }
    }

    fn take_project_entry(
        &mut self,
        project: ProjectId,
        entry: Entry,
        report: &mut ProjectSynced,
    ) -> Result<(), Error> {
        let meta = self.open_replica(&project)?.meta()?;
        if entry.seq <= meta.cursor {
            return Err(Error::Server(format!(
                "the server numbers a project's stream backwards ({} after {})",
                entry.seq, meta.cursor
            )));
        }
        // An operation of this device the server took at a number now passed: dropped.
        if let Some((seq, _)) = meta.sent.range(..entry.seq).next() {
            return Err(Error::Withheld { seq: *seq });
        }
        report.entries += 1;
        match entry.kind {
            Kind::Commit => self.take_project_commit(project, &entry, report)?,
            Kind::Message => {
                let covered = meta.snapshot.is_some_and(|upto| entry.seq <= upto);
                if !covered {
                    self.take_operation(project, &entry, meta, report)?;
                }
            }
        }
        let r = self.open_replica(&project)?;
        let mut meta = r.meta()?;
        meta.cursor = entry.seq;
        meta.last = Some((entry.seq, entry_digest(&entry)));
        r.set_meta(&meta)
    }

    fn take_project_commit(
        &mut self,
        project: ProjectId,
        entry: &Entry,
        report: &mut ProjectSynced,
    ) -> Result<(), Error> {
        if !self.device.in_project(&project) {
            // A project of the whole team has none; a device outside the group cannot read them.
            return Ok(());
        }
        let epoch = entry
            .epoch
            .ok_or_else(|| Error::Server("a commit without its epoch".into()))?;
        self.step(|t| {
            if let Some(pending) = t.device.project_pending(&project) {
                if pending.from_epoch == epoch && pending.commit == entry.payload {
                    // This device's own, sent without hearing back: the server has it.
                    let rest = t.device.project_confirm(&project)?;
                    t.queue_project(project, rest);
                    report.commits += 1;
                    return Ok(());
                }
                t.device.project_abandon(&project)?;
            }
            match t.device.project_process_commit(&project, &entry.payload) {
                Ok(Processed::Epoch(_)) | Ok(Processed::Removed) => {
                    report.commits += 1;
                    Ok(())
                }
                // Before this device joined, or its own merged already.
                Err(CryptoError::Stale { .. }) => Ok(()),
                Err(e) => Err(e.into()),
            }
        })
    }

    /// Opens, checks and applies one operation of the stream.
    fn take_operation(
        &mut self,
        project: ProjectId,
        entry: &Entry,
        meta: ProjectMeta,
        report: &mut ProjectSynced,
    ) -> Result<(), Error> {
        let seq = entry.seq;
        let sealed: Sealed = wire::decode(&entry.payload).map_err(|_| Error::Forged { seq })?;
        let known = self.scope_of(&project)?;
        if known.is_some_and(|s| s != sealed.scope) {
            // A project kept to some members never takes an operation under the team's key (it would be readable
            // by the whole team), nor the reverse.
            return Err(Error::Forged { seq });
        }
        let record = match self.device.record_for(&project, sealed.scope, sealed.epoch) {
            Ok(r) => r,
            Err(e @ CryptoError::Behind(_)) => return Err(e.into()),
            Err(_) => return Err(Error::NoAccess),
        };
        let (op, bytes) = sealed
            .open_operation_bytes(record, &project)
            .map_err(|_| Error::Forged { seq })?;
        match self.device.check_head(op.team_epoch, &op.team_head)? {
            HeadCheck::Same | HeadCheck::Unknown => {}
            HeadCheck::Behind => return Err(CryptoError::Behind(op.team_epoch).into()),
        }
        let digest = Operation::digest_of(&bytes);
        let device = op.author.device;
        let mut meta = meta;
        let own = device == self.device.identity().device;
        // Its place in its device's chain.
        let fresh = match meta.chains.get(&device) {
            None if op.dseq == 1 => true,
            None => return Err(Error::Reordered { seq }),
            Some(c) if op.dseq == c.dseq && digest == c.digest => false,
            Some(c) if op.dseq == c.dseq => return Err(Error::Equivocation { seq }),
            Some(c) if op.dseq < c.dseq => return Err(Error::Replayed { seq }),
            Some(c) if op.dseq > c.dseq + 1 => return Err(Error::Reordered { seq }),
            Some(c) if op.prev != c.digest => return Err(Error::Equivocation { seq }),
            Some(_) => true,
        };
        if own {
            match meta.sent.get(&seq) {
                Some(c) if c.digest == digest => {
                    meta.sent.remove(&seq);
                }
                Some(_) => return Err(Error::Rewritten { seq }),
                // Sent again after a lost answer: the first copy, or one from before a restart.
                None => {}
            }
        }
        if fresh {
            meta.chains.insert(
                device,
                Chain {
                    dseq: op.dseq,
                    digest,
                },
            );
            meta.clock.observe(&op.hlc, now_ms());
        }
        if meta.scope.is_none() {
            meta.scope = Some(sealed.scope);
        }
        let r = self.open_replica(&project)?;
        r.atomically(|r| {
            if fresh && r.apply(&op, &digest, Some(seq))? {
                report.applied += 1;
            }
            r.set_meta(&meta)
        })
    }

    // === Snapshots ===

    /// Starts again from the project's snapshot: its signature by a leaf of its epoch, its members' head, its
    /// number, and each device's chain at least as far as this device saw it, checked; then the state is the
    /// snapshot's, with the operations after it (and this device's not sent yet).
    async fn take_snapshot(&mut self, project: ProjectId, upto: u64) -> Result<(), Error> {
        let info = self.client.get_snapshot(&project.0).await;
        let Some(info) = self.heard(info)? else {
            return Err(Error::Server("a snapshot said but not served".into()));
        };
        let forged = Error::Forged { seq: info.upto };
        let sealed: Sealed =
            wire::decode(&info.payload).map_err(|_| Error::Forged { seq: info.upto })?;
        let known = self.scope_of(&project)?;
        if known.is_some_and(|s| s != sealed.scope) {
            return Err(forged);
        }
        let record = match self.device.record_for(&project, sealed.scope, sealed.epoch) {
            Ok(r) => r,
            Err(e @ CryptoError::Behind(_)) => return Err(e.into()),
            Err(_) => return Err(Error::NoAccess),
        };
        let snap: Snapshot = sealed
            .open_snapshot(record, &project)
            .map_err(|_| Error::Forged { seq: info.upto })?;
        if snap.upto != info.upto || info.upto < upto {
            return Err(Error::Forged { seq: info.upto });
        }
        if let HeadCheck::Behind = self.device.check_head(snap.team_epoch, &snap.team_head)? {
            return Err(CryptoError::Behind(snap.team_epoch).into());
        }
        let own = self.device.identity().device;
        let r = self.open_replica(&project)?;
        let mut meta = r.meta()?;
        for (device, seen) in &meta.chains {
            match snap.chains.get(device) {
                Some(c)
                    if c.dseq > seen.dseq || (c.dseq == seen.dseq && c.digest == seen.digest) => {}
                _ => return Err(Error::ServerRollback { seen: meta.cursor }),
            }
        }
        // This device's operations the server took up to there are in it.
        for (seq, c) in meta.sent.clone().range(..=snap.upto) {
            if snap.chains.get(&own).is_none_or(|o| o.dseq < c.dseq) {
                return Err(Error::Withheld { seq: *seq });
            }
            meta.sent.remove(seq);
        }
        for (device, c) in &snap.chains {
            meta.chains.insert(*device, *c);
        }
        meta.snapshot = Some(snap.upto);
        if meta.scope.is_none() {
            meta.scope = Some(sealed.scope);
        }
        r.load_snapshot(&snap.state, snap.upto)?;
        r.set_meta(&meta)
    }

    /// Writes a snapshot when [`super::Options::snapshot_every`] entries came after the last one, and nothing of
    /// this device waits: sealed and signed, the server forgets the operations before it.
    pub async fn snapshot_if_due(&mut self, project: ProjectId) -> Result<bool, Error> {
        let r = self.open_replica(&project)?;
        let meta = r.meta()?;
        if r.pending()? > 0
            || meta.cursor.saturating_sub(meta.snapshot.unwrap_or(0))
                < self.setup.options.snapshot_every
        {
            return Ok(false);
        }
        self.make_snapshot(project).await
    }

    /// Writes a snapshot of a project at the last number read. False when the server keeps a later one.
    pub async fn make_snapshot(&mut self, project: ProjectId) -> Result<bool, Error> {
        let scope = self.scope_of(&project)?.ok_or(Error::NoProject)?;
        let checkpoint = self.device.checkpoint().ok_or(CryptoError::NotInTeam)?;
        let author = self.device.author();
        let r = self.open_replica(&project)?;
        let meta = r.meta()?;
        if meta.cursor == 0 {
            return Ok(false);
        }
        let snap = Snapshot {
            project,
            author,
            upto: meta.cursor,
            team_epoch: checkpoint.epoch,
            team_head: checkpoint.head,
            chains: meta.chains.clone(),
            state: r.dump()?,
        };
        let sealed = self
            .device
            .seal_for(What::Snapshot, &project, scope, snap.to_bytes())?;
        let answer = self
            .client
            .put_snapshot(&project.0, snap.upto, wire::encode(&sealed))
            .await;
        match self.heard(answer) {
            Ok(_) => {}
            Err(Error::Client(e)) if e.code() == Some("snapshot_refused") => return Ok(false),
            Err(e) => return Err(e),
        }
        let r = self.open_replica(&project)?;
        r.compact(snap.upto)?;
        let mut meta = r.meta()?;
        meta.snapshot = Some(snap.upto);
        r.set_meta(&meta)?;
        Ok(true)
    }

    // === The work of a device online, for the projects ===

    /// Publishes this device's KeyPackage (once, then every 30 days), and keeps the projects' own groups it is in
    /// in step with the team: the new devices of their members added, the devices gone removed, the members an
    /// invitation gave access to added (an administrator's).
    pub(super) async fn serve_projects(&mut self, now: u64) -> Result<(), Error> {
        let due = self
            .meta
            .key_package_at
            .is_none_or(|at| now.saturating_sub(at) >= 30 * 24 * 3600);
        if due {
            self.step(|t| {
                let kp = t.device.key_package()?;
                t.meta.outbox.push(TeamMessage::KeyPackage(kp));
                t.meta.key_package_at = Some(now);
                Ok(())
            })?;
            self.flush().await?;
        }
        for (member, project) in self.meta.grants.clone() {
            if !self.device.in_project(&project) || !self.device.is_admin()? {
                continue;
            }
            let mut members = self.device.project_roster(&project)?.members;
            members.insert(member);
            match self.project_in_step(project, Some(members)).await {
                Ok(_) => {}
                Err(e) if e.alarm().is_some() => return Err(e),
                Err(e) => log::warn!("team: access to a project not given: {e}"),
            }
            self.step(|t| {
                t.meta.grants.retain(|g| *g != (member, project));
                Ok(())
            })?;
        }
        for project in self.device.project_ids() {
            if !self.device.in_project(&project) {
                continue;
            }
            match self.project_in_step(project, None).await {
                Ok(_) => {}
                Err(e) if e.alarm().is_some() => return Err(e),
                Err(e) => log::debug!("team: a project's group not put in step: {e}"),
            }
        }
        Ok(())
    }

    /// The stamp of a field of an object in a project's copy (tests and the interface: who wrote last, when).
    pub fn field_stamp(
        &mut self,
        project: &ProjectId,
        object: &ObjectId,
        field: &str,
    ) -> Result<Option<Hlc>, Error> {
        self.open_replica(project)?.field_stamp(object, field)
    }

    /// The device of this team on this machine.
    pub fn device_id(&self) -> DeviceId {
        self.device.identity().device
    }
}
