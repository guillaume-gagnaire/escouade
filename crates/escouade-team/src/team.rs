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
//! - **Null commits** (SR2-02): a commit of the current epoch that MLS or the team's rules refuse does not stop the
//!   reading. It is kept **null** (its digest, who posted it), the group stays at its epoch, the administrators are
//!   told ([`Team::null_alerts`]); an administrator's device declares it null to the server by a signed void
//!   (`escouade_team_crypto::Void`), which frees the epoch, kept to the administrators until their next commit (a
//!   rotation when nothing else waits). The same in a project's own group.
//! - **Forks**: a commit for an epoch already seen that is not the one this device saw, a history link or a beacon
//!   of a member that shows another head for an epoch this device holds, a void of an administrator for a commit
//!   this device took, a later epoch after a commit this device found null: [`escouade_team_crypto::Error::Fork`].
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
use escouade_team_crypto::anchor::AnchorGrant;
use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::invite::{
    AdmissionCode, InvitationError, InviteLink, JoinRequest, WelcomeDelivery, DEFAULT_LIFETIME,
};
use escouade_team_crypto::keys::EpochRecord;
use escouade_team_crypto::recovery::RecoveryWrap;
use escouade_team_crypto::roster::EscrowPublic;
use escouade_team_crypto::{
    Admission, Change, Checkpoint, DeviceId, DeviceKind, Identity, MemberId, ObjectId, Processed,
    ProjectId, Publication, Purpose, RecoveryKey, Role, Roster, SavedState, TeamDevice, Void,
    CIPHERSUITE,
};
use escouade_team_crypto::{EscrowTeamHalf, TeamFingerprint};
use openmls_basic_credential::SignatureKeyPair;
use openmls_traits::OpenMlsProvider;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::client::{ClientError, LiveFeed, ServerClient, Transport};
use crate::keychain::{DeviceKeys, KeyPlace, Vault};
use crate::pairing::{PairCode, PairState, Pairing};
use crate::storage::{SqliteStorage, TeamProvider, STATE_FILE};
use crate::wire::{self, DroppedInfo, Entry, Kind, TeamMessage};
use crate::{CryptoError, Error};

pub(crate) mod claims;
mod sync;

pub use sync::{ProjectSynced, Refused, SyncState};

/// How many times a change is staged again after losing its epoch, before giving up for now.
const TRIES: usize = 8;

/// The life of an invitation of one's own other device (a QR code, a short code): an hour.
pub const OWN_DEVICE_LIFETIME: u64 = 3600;

/// The objects of the team itself (a request for a token), apart from any project's: a project id no client draws.
pub const TEAM_OBJECTS: ProjectId = ProjectId(*b"escouade/team\0\0\0");

/// The version of the team object an invitation's refusal is sealed as (its id is the invitation's, whose version 1
/// is a member's request for a token).
const REFUSAL_VERSION: u64 = 2;

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
    /// How many entries of a project's stream after its last snapshot make a device of the team write a new one
    /// (TS-5, §2.4): 10 000.
    pub snapshot_every: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            beacon_every: 6 * 3600,
            snapshot_every: 10_000,
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
    /// The commits found null (SR2-02): refused by MLS or the team's rules, passed.
    pub nulls: usize,
}

/// An invitation made: its link (the secret after the `#`) and whether the server already knows its token.
#[derive(Clone, Debug)]
pub struct Invited {
    pub link: InviteLink,
    /// `escouade://team/join/<server>/<id>#…`, to send or to show as a QR code: it opens Escouade, never a page of
    /// the server.
    pub url: String,
    /// Seconds since the epoch.
    pub expires_at: u64,
    /// The server knows its token: the invitee can come. False while an administrator has to register it (this
    /// device is not one on the server).
    pub ready: bool,
}

/// An invitation of this device, as the interface lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvitationInfo {
    pub id: [u8; 16],
    pub purpose: Purpose,
    pub expires_at: u64,
    pub used: bool,
    /// The server knows its token.
    pub ready: bool,
    /// Someone came with it after its first use (a leaked link): when, seconds since the epoch.
    pub reused_at: Option<i64>,
    /// Cancelled: the six words of its request differed from the invitee's (SR2-08).
    pub refused: bool,
}

/// An invitation cancelled because the six words of its request differed from the invitee's (SR2-08), as the
/// administrators are told: by the device that refused it (a sealed object of the team, signed by its leaf), or by
/// this one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefusalAlert {
    #[serde(with = "hex::serde")]
    pub invitation: [u8; 16],
    pub purpose: Purpose,
    /// The member whose device refused it.
    pub by: MemberId,
    /// When, seconds since the epoch.
    pub at: u64,
}

/// What [`Team::serve`] did, an administrator's (or a member's, for their own devices) work.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Served {
    /// The requests of this device's invitations that wait for the six words to be confirmed ([`Team::admissions`]):
    /// nothing is admitted without them (SR2-08).
    pub waiting: usize,
    /// The tokens registered for members who asked.
    pub granted: usize,
    /// The roles set on the server, in step with the roster.
    pub roles: usize,
    /// The invitations found used twice since the last time.
    pub alerts: usize,
    /// The snapshots found unreadable the server told this administrator of, new since the last time (SR2-01).
    pub snapshot_alerts: usize,
    /// The entries the server's operator removed, new since the last time (SR2-02).
    pub drop_alerts: usize,
}

/// An entry the server's operator removed from a stream of the team (`escouade-team-server drop-entry`, SR2-02), as
/// the server lists it to an administrator: the devices read on past the hole; the administrators are told, since
/// only the operator of the server does it, the day an entry blocked the team or a project.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropAlert {
    /// The project's stream, None for the team's.
    pub project: Option<ProjectId>,
    pub seq: u64,
    /// A commit (and its epoch), or a message.
    pub commit: bool,
    pub epoch: Option<u64>,
    /// The device that wrote it, and when (seconds since the epoch, as the server says).
    pub written_by: DeviceId,
    pub written_at: i64,
    /// When the operator removed it.
    pub dropped_at: i64,
}

/// A project's snapshot found unreadable (SR2-01): put aside, the copy read from the journal instead. This device found
/// it, or (an administrator's) another device of the team did and the server said so.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotAlert {
    pub project: ProjectId,
    /// The number it holds up to.
    pub upto: u64,
    /// The device that posted it, and when (seconds since the epoch, as the server says).
    pub posted_by: DeviceId,
    pub posted_at: i64,
    /// When it was found unreadable (seconds since the epoch), and whether this device found it.
    pub found_at: i64,
    pub found_here: bool,
}

/// A commit that won its epoch on the server but that MLS or the team's rules refuse (SR2-02): kept null, the group
/// still at its epoch, the reading goes on. An administrator's device declares it null to the server (a void signed by
/// its leaf), which frees the epoch; the administrators are told, with the device that posted it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NullAlert {
    /// The project's own group, None for the team's.
    pub project: Option<ProjectId>,
    /// Its number in the stream, the epoch it was for, and the SHA-256 of its bytes.
    pub seq: u64,
    pub epoch: u64,
    #[serde(with = "hex::serde")]
    pub digest: [u8; 32],
    /// The device that posted it, and when (seconds since the epoch, as the server says).
    pub written_by: DeviceId,
    pub written_at: i64,
    /// When this device found it null (seconds since the epoch), and why (the rule or the error of MLS).
    pub found_at: u64,
    pub why: String,
    /// An administrator's void of it is in the stream: its epoch was taken again.
    pub voided: bool,
    /// This device sent a void of it (an administrator's).
    #[serde(default)]
    pub void_sent: bool,
}

/// How many null commits a device keeps in mind, the newest.
const NULLS_KEPT: usize = 100;

/// Whether a commit's refusal is the commit's (bytes of nothing, a commit MLS refuses, a rule of the team, a replay):
/// such a commit is null (SR2-02). A fork, a rollback, this device's own state are not.
pub(crate) fn is_null(e: &CryptoError) -> bool {
    matches!(
        e,
        CryptoError::Mls(_)
            | CryptoError::Policy(_)
            | CryptoError::Encoding(_)
            | CryptoError::Stale { .. }
            | CryptoError::BadSignature
            | CryptoError::Decrypt
    )
}

/// A second use of one of this device's invitations, as the server told it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ReuseAlert {
    #[serde(with = "hex::serde")]
    invitation: [u8; 16],
    reused_at: i64,
}

/// What this device keeps of the team's stream, beside the MLS state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
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
    /// The number, the device and the hour of each of them (to tell a null one, SR2-02).
    early_from: BTreeMap<u64, (u64, [u8; 16], i64)>,
    /// The commits found null (SR2-02), the team's and the projects' own groups', the oldest first.
    nulls: Vec<NullAlert>,
    /// The join requests an administrator may admit.
    requests: Vec<JoinRequest>,
    /// What waits to go to the server, in order.
    outbox: Vec<TeamMessage>,
    /// The epoch and the hour of this device's last beacon.
    beacon: Option<(u64, u64)>,
    /// The members' requests for a token, for an administrator of the server to serve (`Team::serve`).
    token_requests: Vec<SealedObject>,
    /// The SHA-256 of the tokens the server knows (registered here, or by an administrator who said so).
    granted: BTreeSet<[u8; 32]>,
    /// The second uses of this device's invitations.
    alerts: Vec<ReuseAlert>,
    /// The invitations of the team cancelled because the six words differed (SR2-08): this device's, and those the
    /// other devices told of.
    refusals: Vec<RefusalAlert>,
    /// The snapshots found unreadable (SR2-01): by this device, and for an administrator the ones the server lists.
    snapshot_alerts: Vec<SnapshotAlert>,
    /// The entries the server's operator removed (SR2-02), as the server lists them to an administrator.
    drop_alerts: Vec<DropAlert>,
    /// The epoch whose roster the server's roles were last put in step with.
    roles_epoch: Option<u64>,
    /// The administrators of each epoch this device lived through, as its roster said (TS-13: a version of the team's
    /// settings counts the signature of an administrator of the epoch it was sealed at).
    admins: BTreeMap<u64, BTreeSet<MemberId>>,
    /// The machines of the team (role of service, OE-1) this device saw in a roster: the role never changes, so one
    /// removed since is still a machine for what it signed before.
    #[serde(default)]
    services: BTreeSet<MemberId>,
    /// The last-resort KeyPackage of each device, read in the stream (TS-5): the groups of the projects kept to some
    /// members add a device by it. Checked against the team's leaves when used.
    key_packages: BTreeMap<DeviceId, Vec<u8>>,
    /// When this device last sent its own.
    key_package_at: Option<u64>,
    /// The projects an invitation of this device gives access to, by the invitation's id.
    invite_projects: Vec<([u8; 16], Vec<ProjectId>)>,
    /// The members to give access to a project kept to some members, once they are in (an administrator's work).
    grants: Vec<(MemberId, ProjectId)>,
    /// The highest epoch whose recovery wrap, signed by the roster's anchor, is in the stream (SR2-09): an
    /// administrator's device writes the one of an epoch a member's commit made.
    recovery_wrapped: Option<u64>,
    /// The same for the escrow wraps.
    escrow_wrapped: Option<u64>,
    /// The anchors' grants in the stream: (device, anchor's public half), whoever sealed them.
    anchors_given: BTreeSet<([u8; 16], Vec<u8>)>,
    /// The recovery anchor of the roster, and every member who was an administrator while this device saw it in
    /// place: one of them who is no longer an administrator may still hold it ([`Team::recovery_stale`]).
    anchor_holders: Option<(Vec<u8>, BTreeSet<MemberId>)>,
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// The SHA-256 of an invitation's token for the server, as the server keeps it.
fn token_hash(link: &InviteLink) -> [u8; 32] {
    digest(link.server_token().as_bytes())
}

fn token_hash_of(secret: &[u8; 32]) -> [u8; 32] {
    digest(escouade_team_crypto::invite::server_token(secret).as_bytes())
}

/// An enrolment refused, as the invitee is told.
fn enrolment_error(e: ClientError) -> Error {
    match e.code() {
        Some("token_used") => Error::InvitationUsed,
        Some("bad_token") => Error::InvitationUnknown,
        _ => Error::Client(e),
    }
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

fn project_record_name(project: &ProjectId, epoch: u64) -> Vec<u8> {
    let mut out = b"precord/".to_vec();
    out.extend_from_slice(&project.0);
    out.extend_from_slice(&epoch.to_be_bytes());
    out
}

fn project_link_prefix(project: &ProjectId, epoch: u64) -> Vec<u8> {
    let mut out = b"plink/".to_vec();
    out.extend_from_slice(&project.0);
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
    /// The records of the projects' own groups already on the disk.
    project_stored: BTreeSet<(ProjectId, u64)>,
    /// The local copies of the projects open (TS-5).
    replicas: BTreeMap<ProjectId, crate::replica::Replica>,
    /// Whether the server answered the last call: « Hors ligne » otherwise.
    online: Option<bool>,
    /// The writes of a ticket's holder the server refused since the last [`Team::take_refused`]: another device took
    /// the ticket over (TS-6).
    refused: Vec<sync::Refused>,
    /// When an administrator's device last read the snapshots found unreadable (seconds, as `serve` was given it):
    /// once a minute at most, the server's rate is the team's.
    snapshot_alerts_at: Option<u64>,
    /// When an administrator's device last read the entries the operator removed (SR2-02), as `serve` was given it;
    /// None to read them at the next `serve` (a hole seen in the team's stream, the last entry read removed).
    drops_at: Option<u64>,
}

/// Seconds between two readings of the snapshots found unreadable by an administrator's device (SR2-01).
const SNAPSHOT_ALERTS_EVERY: u64 = 60;

/// Seconds between two readings of the entries the operator removed, by an administrator's device (SR2-02): they
/// are rare, and a hole seen reads them at once.
const DROPS_EVERY: u64 = 300;

impl Team {
    /// Creates a team on the server with the token of `escouade-team-server init`: this device enrols, makes its
    /// keys (kept in the keychain), creates the MLS group with `member` as its owner, and sends the recovery wrap.
    /// The recovery key's 24 words are given once, to show.
    pub async fn create(
        setup: Setup,
        token: &str,
        member: MemberId,
        escrow: Option<EscrowTeamHalf>,
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

    /// Joins by an invitation link alone (C6-A): its token for the server is derived from the secret after the `#`,
    /// and the member is the link's (another device of a member) or a new one. A link used already says so
    /// ([`Error::InvitationUsed`]: its administrator is told); one the server does not know (expired, or not
    /// registered yet by an administrator): [`Error::InvitationUnknown`].
    pub async fn join_link(setup: Setup, link: &InviteLink) -> Result<Team, Error> {
        let member = link
            .member
            .map(MemberId)
            .unwrap_or_else(|| MemberId(random::<16>()));
        Self::join(setup, &link.server_token(), link, member).await
    }

    async fn enrol(setup: Setup, token: &str, member: MemberId) -> Result<Team, Error> {
        let auth = SigningKey::from_bytes(&random::<32>());
        let enrolled = ServerClient::enroll(&*setup.transport, token, &auth)
            .await
            .map_err(enrolment_error)?;
        let keys = DeviceKeys {
            team: enrolled.team,
            member: member.0,
            device: enrolled.device,
            auth: auth.to_bytes(),
            mls: random::<32>(),
            state_key: random::<32>(),
            recovery_leaf: None,
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
        let (device, meta, stored, project_stored) = load(&dir, &keys)?;
        let saved = setup.vault.checkpoint(&team)?;
        if let (Some(cp), true) = (&saved, device.group_id().is_ok()) {
            device.check_restored(cp)?;
        }
        let mut t = Self::assemble(setup, keys, place, device, meta, saved);
        t.stored = stored;
        t.project_stored = project_stored;
        Ok(t)
    }

    /// The holder of the recovery key comes back, the day every administrator lost their devices (§3.5, C7-B): with
    /// the operator's token (`escouade-team-server recovery-token`) and the 24 words. This device enrols, reads the
    /// stream for the last recovery wrap, opens it with the words, and joins the group by an external commit, as the
    /// recovery leaf: an administrator. It then invites the new devices ([`Team::recover_onto`] does the rest).
    ///
    /// `team_words` are the six words of the team's fingerprint, typed from the rescue sheet: the wrap must be signed
    /// by the anchor the 24 words give, for that team, by an administrator's leaf of its epoch (SR2-09). A wrap anyone
    /// else wrote is passed over; none left is [`Error::NoRecoveryWrap`].
    pub async fn recover(
        setup: Setup,
        token: &str,
        words: &str,
        team_words: &str,
    ) -> Result<Team, Error> {
        // The words first: a typo does not spend the token.
        let key = RecoveryKey::from_words(words)?;
        if team_words.split_whitespace().count() != 6 {
            return Err(Error::Crypto(CryptoError::OtherTeam));
        }
        let auth = SigningKey::from_bytes(&random::<32>());
        let enrolled = ServerClient::enroll(&*setup.transport, token, &auth)
            .await
            .map_err(enrolment_error)?;
        let identity = key.identity();
        let keys = DeviceKeys {
            team: enrolled.team,
            member: identity.member.0,
            device: enrolled.device,
            auth: auth.to_bytes(),
            mls: *key.signing_seed(),
            state_key: random::<32>(),
            recovery_leaf: Some(identity.device.0),
        };
        let client = ServerClient::new(
            setup.transport.clone(),
            keys.device,
            SigningKey::from_bytes(&keys.auth),
        );
        let (group, wrap) = recovery_wrap(&client, &key, words, team_words).await?;
        let dir = setup.folder.join(hex::encode(keys.team));
        if dir.join(STATE_FILE).exists() {
            return Err(Error::Refused("this device already holds this team".into()));
        }
        std::fs::create_dir_all(&dir).map_err(|e| Error::Storage(e.to_string()))?;
        let attempt = async {
            let provider = TeamProvider::new(open_storage(&dir, &keys)?);
            let (device, publication) =
                TeamDevice::recover_with(provider, words, team_words, &wrap, &group)?;
            let (from_epoch, commit) = publication.commit.clone().ok_or_else(|| {
                Error::Crypto(CryptoError::Mls("a recovery without its commit".into()))
            })?;
            client.append_commit(from_epoch, &commit, false).await?;
            Ok::<_, Error>((device, publication, from_epoch, commit))
        };
        let (device, publication, from_epoch, commit) = match attempt.await {
            Ok(done) => done,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dir);
                return Err(match e {
                    Error::Client(ClientError::EpochTaken(_)) => Error::Refused(
                        "the team moved on since its last recovery wrap: try again".into(),
                    ),
                    e => e,
                });
            }
        };
        let place = setup.vault.put_keys(&keys)?;
        let mut team = Self::assemble(setup, keys, place, device, Meta::default(), None);
        team.step(|t| {
            t.meta.commits.insert(from_epoch, digest(&commit));
            t.queue(Publication {
                commit: None,
                ..publication
            });
            Ok(())
        })?;
        team.flush().await?;
        Ok(team)
    }

    /// The whole recovery on this machine: the recovery leaf comes back ([`Team::recover`], with `rescue`, a folder and
    /// a keychain of its own), invites a new administrator device (`setup`, this machine's) and admits it, puts it
    /// administrator on the server too; the new device then replaces the recovery key (the leaf of the old one leaves
    /// the group in the same commit), and the old one's server device is revoked and forgotten. Gives the new device
    /// and the new recovery key, whose 24 words are to show once.
    pub async fn recover_onto(
        rescue: Setup,
        setup: Setup,
        token: &str,
        words: &str,
        team_words: &str,
        now: u64,
    ) -> Result<(Team, RecoveryKey), Error> {
        let mut leaf = Self::recover(rescue, token, words, team_words).await?;
        leaf.sync_at(now).await?;
        let invited = leaf
            .invite_for(Purpose::NewMember(Role::Admin), now, OWN_DEVICE_LIFETIME)
            .await?;
        let mut device = Self::join_link(setup, &invited.link).await?;
        leaf.sync_at(now).await?;
        // Both screens are this one: the six words are the new device's own, read here, not compared by a person.
        let words = device
            .join_words()
            .ok_or_else(|| Error::Server("the new device made no request".into()))?
            .join(" ");
        leaf.admit_invitation(invited.link.id, &words, now).await?;
        let synced = device.sync_at(now).await?;
        if !synced.joined {
            return Err(Error::Server(
                "the new device did not get its Welcome".into(),
            ));
        }
        // The new device administrator on the server too, before the leaf goes.
        leaf.sync_at(now).await?;
        leaf.serve(now).await?;
        let recovery = device.replace_recovery().await?;
        let _ = leaf.client.revoke(&leaf.keys.device).await;
        leaf.forget()?;
        device.sync_at(now).await?;
        Ok((device, recovery))
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
            project_stored: BTreeSet::new(),
            replicas: BTreeMap::new(),
            refused: Vec::new(),
            online: None,
            snapshot_alerts_at: None,
            drops_at: None,
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

    /// This device is a machine of the team (role of service, OE-1): it signs its own devices and the content of the
    /// projects opened to it, nothing else (`policy::may_sign`).
    pub fn is_service(&self) -> bool {
        self.identity().kind == DeviceKind::Device && self.is_machine(&self.identity().member)
    }

    /// Whether `member` is a machine of the team: in the roster now under the role of service, or seen so before (the
    /// role never changes).
    pub fn is_machine(&self, member: &MemberId) -> bool {
        self.meta.services.contains(member)
            || self
                .device
                .roster()
                .is_ok_and(|r| r.role(member) == Some(Role::Service))
    }

    /// Whether this device may sign `act` (OE-1, `policy::may_sign`), by its role now; the recovery leaf is an
    /// administrator's.
    pub fn may_sign(&self, act: escouade_team_crypto::Act) -> bool {
        if self.identity().kind == DeviceKind::Recovery {
            return self.is_admin().unwrap_or(false);
        }
        self.device
            .roster()
            .ok()
            .and_then(|r| r.role(&self.identity().member))
            .is_some_and(|role| escouade_team_crypto::may_sign(role, act))
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

    /// The server's host, as the invitation links name it.
    pub fn server(&self) -> &str {
        &self.setup.server
    }

    /// The team's fingerprint, the hash of its genesis: its six words, to compare out of band, and in every link.
    pub fn fingerprint(&self) -> Result<escouade_team_crypto::TeamFingerprint, Error> {
        Ok(self.device.fingerprint()?)
    }

    /// The organisation's escrow key, when the team has one: shown to every member (C7-B).
    pub fn escrow(&self) -> Result<Option<EscrowPublic>, Error> {
        Ok(self.device.escrow()?)
    }

    /// The administrators of the roster.
    /// The administrators of `epoch`, when this device lived through it (None for an epoch before it joined).
    pub fn admins_at(&self, epoch: u64) -> Option<&BTreeSet<MemberId>> {
        self.meta.admins.get(&epoch)
    }

    /// Whether someone who was an administrator while the current recovery key was in place is no longer one (they
    /// left, or became a member): they may still hold its anchor (SR2-09), so the team makes a new recovery key and
    /// prints the new sheet. As far as this device saw: its own time in the team only.
    pub fn recovery_stale(&self) -> bool {
        let Ok(roster) = self.device.roster() else {
            return false;
        };
        self.meta
            .anchor_holders
            .as_ref()
            .is_some_and(|(key, holders)| {
                *key == roster.recovery.anchor_key
                    && holders.iter().any(|m| roster.role(m) != Some(Role::Admin))
            })
    }

    /// Who signed with `signature_key` at `epoch`: a leaf of that epoch's record, None when this device does not hold
    /// the epoch or no leaf has the key.
    pub fn signer_at(&self, epoch: u64, signature_key: &[u8]) -> Option<Identity> {
        self.device
            .record(epoch)?
            .members
            .iter()
            .find(|m| m.signature_key == signature_key)
            .and_then(|m| Identity::from_bytes(&m.identity).ok())
    }

    /// A value of this device's own beside the team's state (the team's settings, TS-13): in the same encrypted base.
    pub fn load_extra<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
    ) -> Result<Option<T>, Error> {
        Ok(self
            .storage()
            .get_ours(format!("extra/{name}").as_bytes())?)
    }

    pub fn save_extra<T: Serialize>(&self, name: &str, value: &T) -> Result<(), Error> {
        Ok(self
            .storage()
            .put_ours(format!("extra/{name}").as_bytes(), value)?)
    }

    pub fn admins(&self) -> Result<usize, Error> {
        Ok(self.device.roster()?.admins())
    }

    /// This device's invitations, the newest first.
    pub fn invitations(&self) -> Vec<InvitationInfo> {
        let mut out: Vec<InvitationInfo> = self
            .device
            .saved_state()
            .invitations
            .iter()
            .map(|i| InvitationInfo {
                id: i.id,
                purpose: i.purpose.clone(),
                expires_at: i.expires_at,
                used: i.used,
                ready: self.meta.granted.contains(&token_hash_of(&i.secret)),
                reused_at: self
                    .meta
                    .alerts
                    .iter()
                    .filter(|a| a.invitation == i.id)
                    .map(|a| a.reused_at)
                    .max(),
                refused: i.refused,
            })
            .collect();
        out.sort_by_key(|i| std::cmp::Reverse(i.expires_at));
        out
    }

    /// Forgets the team on this device: its folder and its secrets (the keychain's). Nothing is said to the server
    /// nor to the group: an administrator removes the device (TS-4).
    pub fn forget(self) -> Result<(), Error> {
        let dir = self.dir.clone();
        let vault = self.setup.vault.clone();
        let id = self.id;
        drop(self);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| Error::Storage(e.to_string()))?;
        }
        vault.forget_team(&id)
    }

    /// The join requests read in the stream, for an administrator to admit ([`Team::admit`]).
    pub fn requests(&self) -> &[JoinRequest] {
        &self.meta.requests
    }

    /// The requests of this device's invitations that it may admit, checked (invitation, MAC, signed KeyPackage),
    /// each with its six words: the inviting screen shows them, to compare with the invitee's before anything is
    /// admitted ([`Team::admit_invitation`], [`Team::refuse`], SR2-08).
    pub fn admissions(&self, now: u64) -> Vec<Admission> {
        if self.device.group_id().is_err() || self.device.is_removed() {
            return Vec::new();
        }
        let mut out: Vec<Admission> = Vec::new();
        for request in &self.meta.requests {
            if let Ok(a) = self.device.check_request(request, now) {
                if !out.contains(&a) {
                    out.push(a);
                }
            }
        }
        out
    }

    /// The six words of this device's join request, while it waits for its Welcome: to show, for the person who
    /// invites to compare with theirs (SR2-08).
    pub fn join_words(&self) -> Option<[&'static str; 6]> {
        self.device.join_code().map(|c| c.words())
    }

    /// The invitations of the team cancelled because the six words differed (SR2-08), the newest first.
    pub fn refusals(&self) -> Vec<RefusalAlert> {
        let mut out = self.meta.refusals.clone();
        out.sort_by_key(|r| std::cmp::Reverse(r.at));
        out
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
        if let (Ok(epoch), Ok(roster)) = (self.device.epoch(), self.device.roster()) {
            let admins: BTreeSet<MemberId> = roster
                .members
                .iter()
                .filter(|(_, r)| **r == Role::Admin)
                .map(|(m, _)| *m)
                .collect();
            match &mut self.meta.anchor_holders {
                Some((key, holders)) if *key == roster.recovery.anchor_key => {
                    holders.extend(admins.iter().copied());
                }
                _ => {
                    self.meta.anchor_holders =
                        Some((roster.recovery.anchor_key.clone(), admins.clone()));
                }
            }
            self.meta.admins.entry(epoch).or_insert(admins);
            self.meta.services.extend(roster.services().copied());
        }
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
        let mut new = Vec::new();
        for (project, record) in self.device.project_records() {
            if !self.project_stored.contains(&(project, record.epoch)) {
                storage.put_ours(&project_record_name(&project, record.epoch), record)?;
                new.push((project, record.epoch));
            }
        }
        self.project_stored.extend(new);
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
        let (device, meta, stored, project_stored) = load(&self.dir, &self.keys)?;
        self.device = device;
        self.meta = meta;
        self.stored = stored;
        self.project_stored = project_stored;
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
        if let Some(wrap) = p.recovery {
            let group = self.device.group_id().unwrap_or_default();
            self.meta.recovery_wrapped = self.meta.recovery_wrapped.max(Some(wrap.epoch));
            self.meta.outbox.push(TeamMessage::Recovery { group, wrap });
        }
        if let Some(w) = p.escrow {
            self.meta.escrow_wrapped = self.meta.escrow_wrapped.max(Some(w.epoch));
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
        let read = self.sync_at(now()).await;
        // What the status says (§2.5): the server unreachable is « Hors ligne » (TS-9: the docs read the team first).
        match &read {
            Ok(_) => self.online = Some(true),
            Err(Error::Client(crate::client::ClientError::Net(_))) => self.online = Some(false),
            Err(_) => {}
        }
        read
    }

    /// [`Team::sync`] at the hour `now` (seconds), for the beacons.
    pub async fn sync_at(&mut self, now: u64) -> Result<Synced, Error> {
        self.check_last().await?;
        let mut report = Synced::default();
        // The entries the operator removed, read once a hole shows (SR2-02).
        let mut dropped: Option<Vec<DroppedInfo>> = None;
        loop {
            let page = self.client.read_team(self.meta.cursor, None).await?;
            for entry in page.entries {
                let from = self.meta.cursor;
                if entry.seq > from + 1 {
                    if dropped.is_none() {
                        dropped = Some(self.client.dropped().await?);
                        self.drops_at = None;
                    }
                    let list = dropped.as_deref().unwrap_or_default();
                    self.step(|t| {
                        t.forget_dropped_commits(list, from, entry.seq);
                        Ok(())
                    })?;
                }
                self.step(|t| t.take(entry, &mut report))?;
            }
            if !page.more {
                break;
            }
        }
        if self.device.group_id().is_ok() {
            self.void_nulls().await?;
            self.step(|t| t.read_back())?;
            self.step(|t| t.read_back_projects())?;
            self.step(|t| {
                t.write_missing_links()?;
                t.beacon_if_due(now)
            })?;
        }
        self.flush().await?;
        Ok(report)
    }

    /// The last entry this device read must still be there, the same: a server that hid it, or the ones before,
    /// rolled back; one that changed it lies. Unless the server's operator removed that very entry (`drop-entry`,
    /// SR2-02): a hole, said to the administrators, and the reading goes on after it.
    async fn check_last(&mut self) -> Result<(), Error> {
        let Some((seq, digest)) = self.meta.last else {
            return Ok(());
        };
        let page = self.client.read_team(seq - 1, Some(1)).await?;
        match page.entries.first() {
            Some(e) if e.seq == seq && entry_digest(e) == digest => Ok(()),
            Some(e) if e.seq == seq => Err(Error::Rewritten { seq }),
            _ if self.client.was_dropped(None, seq, &digest).await? => {
                self.drops_at = None;
                self.step(|t| {
                    t.meta.last = None;
                    Ok(())
                })
            }
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
                self.take_commit(epoch, &entry, report)?;
            }
            Kind::Void => {
                let epoch = entry
                    .epoch
                    .ok_or_else(|| Error::Server("a void without its epoch".into()))?;
                self.take_void(epoch, &entry)?;
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

    fn take_commit(&mut self, epoch: u64, entry: &Entry, report: &mut Synced) -> Result<(), Error> {
        let commit = entry.payload.as_slice();
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
            if let Ok(device) = <[u8; 16]>::try_from(entry.device.as_slice()) {
                self.meta
                    .early_from
                    .insert(epoch, (entry.seq, device, entry.at));
            }
            return Ok(());
        };
        if epoch < current {
            // Before this device joined: nothing to check it against.
            self.meta.commits.insert(epoch, d);
            return Ok(());
        }
        if epoch > current {
            if self.null_at(None, current).is_some() {
                // The commit this device found null, others took: the server shows two versions of the team.
                return Err(CryptoError::Fork { epoch: current + 1 }.into());
            }
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
        let processed = match self.device.process_commit(commit) {
            Ok(p) => p,
            // Refused, the group still at its epoch: null, the reading goes on (SR2-02).
            Err(e) if is_null(&e) && self.device.epoch().ok() == Some(epoch) => {
                log::warn!(
                    "team: the commit {} of epoch {epoch} is null: {e}",
                    entry.seq
                );
                self.note_null(None, epoch, entry, e.to_string());
                report.nulls += 1;
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };
        self.meta.commits.insert(epoch, d);
        report.commits += 1;
        if processed == Processed::Removed {
            report.removed = true;
        }
        Ok(())
    }

    /// Keeps a commit found null (SR2-02), for the administrators and for the void.
    pub(crate) fn note_null(
        &mut self,
        project: Option<ProjectId>,
        epoch: u64,
        entry: &Entry,
        why: String,
    ) {
        let Ok(device) = <[u8; 16]>::try_from(entry.device.as_slice()) else {
            return;
        };
        self.note_null_from(
            project,
            epoch,
            (entry.seq, device, entry.at),
            digest(&entry.payload),
            why,
        );
    }

    fn note_null_from(
        &mut self,
        project: Option<ProjectId>,
        epoch: u64,
        (seq, device, at): (u64, [u8; 16], i64),
        digest: [u8; 32],
        why: String,
    ) {
        if self
            .meta
            .nulls
            .iter()
            .any(|n| n.project == project && n.seq == seq)
        {
            return;
        }
        self.meta.nulls.push(NullAlert {
            project,
            seq,
            epoch,
            digest,
            written_by: DeviceId(device),
            written_at: at,
            found_at: now(),
            why,
            voided: false,
            void_sent: false,
        });
        let over = self.meta.nulls.len().saturating_sub(NULLS_KEPT);
        self.meta.nulls.drain(..over);
    }

    /// The null commit of `epoch` in the team's group (`project` None) or a project's, if any.
    pub(crate) fn null_at(&self, project: Option<ProjectId>, epoch: u64) -> Option<&NullAlert> {
        self.meta
            .nulls
            .iter()
            .rev()
            .find(|n| n.project == project && n.epoch == epoch)
    }

    /// A void in the team's stream (SR2-02). Signed by a leaf of an administrator of its epoch (as this device lived
    /// it), it marks the null commit it names as passed; it names a commit this device took: the server shows two
    /// versions of the team. Before this device was in the group at that epoch, the commit it names is forgotten (as
    /// one the operator removed), so that the commit that takes the epoch again is not a fork. Anything else changes
    /// nothing.
    fn take_void(&mut self, epoch: u64, entry: &Entry) -> Result<(), Error> {
        let void: Void = match wire::decode(&entry.payload) {
            Ok(v) => v,
            Err(e) => {
                log::debug!("team: a void not understood: {e}");
                return Ok(());
            }
        };
        if void.epoch != epoch {
            log::debug!("team: a void for another epoch than its entry's");
            return Ok(());
        }
        let lived = self.meta.admins.get(&epoch).cloned();
        let by_admin = match self.device.void_signer(None, &void) {
            Ok(Some(i)) => {
                i.kind == DeviceKind::Device
                    && lived.as_ref().is_some_and(|a| a.contains(&i.member))
            }
            _ => false,
        };
        if let Some(n) = self
            .meta
            .nulls
            .iter_mut()
            .find(|n| n.project.is_none() && n.epoch == epoch && n.digest == void.digest)
        {
            if by_admin {
                n.voided = true;
            } else {
                log::warn!("team: a void of a null commit that no administrator signed");
            }
            return Ok(());
        }
        if self.meta.commits.get(&epoch) == Some(&void.digest) {
            if lived.is_none() {
                self.meta.commits.remove(&epoch);
                self.meta.early.remove(&epoch);
                self.meta.early_from.remove(&epoch);
            } else if by_admin {
                return Err(CryptoError::Fork { epoch: epoch + 1 }.into());
            }
        }
        Ok(())
    }

    /// An administrator's device declares null to the server the commits of the current epoch it found null (SR2-02):
    /// a void signed by its leaf. Refused (`void_refused`) when another administrator did it first.
    async fn void_nulls(&mut self) -> Result<(), Error> {
        if !self.device.is_admin().unwrap_or(false) {
            return Ok(());
        }
        let Ok(current) = self.device.epoch() else {
            return Ok(());
        };
        let due: Vec<(u64, [u8; 32])> = self
            .meta
            .nulls
            .iter()
            .filter(|n| n.project.is_none() && n.epoch == current && !n.voided && !n.void_sent)
            .map(|n| (n.seq, n.digest))
            .collect();
        for (seq, digest) in due {
            let void = self.device.sign_void(None, current, seq, digest)?;
            match self
                .client
                .append_void(None, current, &wire::encode(&void))
                .await
            {
                Ok(_) => {}
                Err(e) if e.code() == Some("void_refused") => {}
                // An administrator of the roster the server does not know as one yet (its roles follow at an
                // administrator's next `serve`): tried again at the next reading, the reading goes on.
                Err(e) if e.status() == Some(403) => {
                    log::info!(
                        "team: the server takes a void from its administrators alone: later"
                    );
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
            self.step(|t| {
                for n in t.meta.nulls.iter_mut() {
                    if n.project.is_none() && n.seq == seq {
                        n.void_sent = true;
                    }
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    /// The commits found null (SR2-02), the newest first: shown to the administrators.
    pub fn null_alerts(&self) -> Vec<NullAlert> {
        let mut out = self.meta.nulls.clone();
        out.reverse();
        out
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
            TeamMessage::Refused(sealed) => self.take_refusal(&sealed),
            TeamMessage::Welcome {
                invitation,
                delivery,
            } => self.take_welcome(invitation, &delivery, report)?,
            TeamMessage::TokenRequest(sealed) => {
                if !self
                    .meta
                    .token_requests
                    .iter()
                    .any(|r| r.header.object == sealed.header.object)
                {
                    self.meta.token_requests.push(sealed);
                }
            }
            TeamMessage::TokenGranted { hash } => {
                self.meta.granted.insert(hash);
            }
            TeamMessage::KeyPackage(bytes) => match self.device.key_package_identity(&bytes) {
                Ok(who) if who.device != self.device.identity().device => {
                    self.meta.key_packages.insert(who.device, bytes);
                }
                Ok(_) => {}
                Err(e) => log::debug!("team: a KeyPackage set aside: {e}"),
            },
            TeamMessage::ProjectWelcome { project, welcome } => {
                if self.device.group_id().is_ok() && !self.device.is_removed() {
                    if let Err(e) = self
                        .device
                        .project_accept_welcome(&ProjectId(project), &welcome)
                    {
                        log::warn!("team: a project's Welcome refused: {e}");
                    }
                }
            }
            TeamMessage::ProjectHistory { project, link } => {
                let project = ProjectId(project);
                if self.device.project_record(&project, link.epoch).is_some() {
                    match self.device.project_learn_history(&project, &link) {
                        Ok(_) => return Ok(()),
                        Err(e @ CryptoError::Fork { .. }) => return Err(e.into()),
                        Err(e) => log::debug!("team: a project's history link refused: {e}"),
                    }
                }
                let mut name = project_link_prefix(&project, link.epoch);
                name.extend_from_slice(&random::<8>());
                self.storage().put_ours(&name, &link)?;
            }
            TeamMessage::Recovery { group, wrap } => {
                if self.device.group_id().is_ok_and(|g| g == group)
                    && !self.device.is_removed()
                    && self
                        .device
                        .recovery_wrap_is_anchored(&wrap)
                        .unwrap_or(false)
                {
                    self.meta.recovery_wrapped = self.meta.recovery_wrapped.max(Some(wrap.epoch));
                }
            }
            TeamMessage::Escrow(wrap) => {
                if self.device.group_id().is_ok()
                    && !self.device.is_removed()
                    && self.device.escrow_wrap_is_anchored(&wrap).unwrap_or(false)
                {
                    self.meta.escrow_wrapped = self.meta.escrow_wrapped.max(Some(wrap.epoch));
                }
            }
            TeamMessage::Anchor(grant) => self.take_grant(grant),
        }
        Ok(())
    }

    /// A grant of an anchor (SR2-09): noted as given, and taken when it is for this device.
    fn take_grant(&mut self, grant: AnchorGrant) {
        self.meta
            .anchors_given
            .insert((grant.device, grant.anchor.clone()));
        if self.device.group_id().is_err() || self.device.is_removed() {
            return;
        }
        match self.device.take_anchor(&grant, &self.keys.mls) {
            Ok(true) => log::info!("team: this device holds the team's anchor"),
            Ok(false) => {}
            Err(e) => log::warn!("team: an anchor's grant refused: {e}"),
        }
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
        let from = std::mem::take(&mut self.meta.early_from);
        for (epoch, commit) in early.range(current..) {
            if *epoch != self.device.epoch()? {
                return Err(Error::Server("a gap in the early commits".into()));
            }
            match self.device.process_commit(commit) {
                Ok(Processed::Removed) => {
                    report.removed = true;
                    break;
                }
                Ok(_) => report.commits += 1,
                // A null one (SR2-02): the group stays at its epoch, the commit that takes it again comes next.
                Err(e) if is_null(&e) && self.device.epoch().ok() == Some(*epoch) => {
                    self.meta.commits.remove(epoch);
                    if let Some(f) = from.get(epoch) {
                        self.note_null_from(None, *epoch, *f, digest(commit), e.to_string());
                    }
                    report.nulls += 1;
                    break;
                }
                Err(e) => return Err(e.into()),
            }
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
            let own = self.device.commit_scope()? == escouade_team_crypto::CommitScope::Own;
            match self.client.append_commit(epoch, &commit, own).await {
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
                // The commit that won was declared null: the epoch waits for an administrator's (SR2-02).
                Err(e) if e.code() == Some("epoch_reserved") => {
                    self.step(|t| Ok(t.device.abandon_pending()?))?;
                    return Err(Error::Busy);
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
        let invited = self.invite_for(purpose, now, DEFAULT_LIFETIME).await?;
        let token = invited.ready.then(|| invited.link.server_token());
        Ok((invited.link, token))
    }

    /// Makes an invitation that lasts `lifetime` seconds (72 h by default, `DEFAULT_LIFETIME`), for one use: its link
    /// carries the secret after the `#` (C6-A), and the token for the server derived from it, whose SHA-256 this
    /// device registers with the server when it is an administrator there. When it is not (a member adding their
    /// own device, an administrator named after their enrolment and not yet in step), it asks one in the team's
    /// stream ([`TeamMessage::TokenRequest`]): the invitation is `ready` once one did ([`Team::serve`]).
    pub async fn invite_for(
        &mut self,
        purpose: Purpose,
        now: u64,
        lifetime: u64,
    ) -> Result<Invited, Error> {
        let server = self.setup.server.clone();
        let link =
            self.step(|t| Ok(t.device.invite(purpose.clone(), now, lifetime, &server)?))?;
        let hash = token_hash(&link);
        let ready = match self.client.register_token(hash, lifetime).await {
            Ok(_) => true,
            Err(e) if e.status() == Some(403) => {
                let body = wire::TokenRequestBody {
                    invitation: link.id,
                    hash,
                    expires_in: lifetime,
                    purpose: purpose.clone(),
                };
                let sealed =
                    self.device
                        .seal(TEAM_OBJECTS, ObjectId(link.id), 1, &wire::encode(&body))?;
                self.step(|t| {
                    t.meta.outbox.push(TeamMessage::TokenRequest(sealed));
                    Ok(())
                })?;
                self.flush().await?;
                false
            }
            Err(e) => return Err(e.into()),
        };
        if ready {
            self.step(|t| {
                t.meta.granted.insert(hash);
                Ok(())
            })?;
        }
        Ok(Invited {
            url: link.to_url(),
            expires_at: now.saturating_add(lifetime),
            ready,
            link,
        })
    }

    /// An invitation, as [`Team::invite_for`], that also gives access to `projects` (projects kept to some members:
    /// the member who comes with it is added to their groups once in). Kept on this device, as the invitation.
    pub async fn invite_with_projects(
        &mut self,
        purpose: Purpose,
        now: u64,
        lifetime: u64,
        projects: Vec<ProjectId>,
    ) -> Result<Invited, Error> {
        let invited = self.invite_for(purpose, now, lifetime).await?;
        if !projects.is_empty() {
            self.step(|t| {
                t.meta.invite_projects.push((invited.link.id, projects));
                Ok(())
            })?;
        }
        Ok(invited)
    }

    /// The work of a device online for the others, after a reading of the stream ([`Team::sync`]):
    /// - **lists** the requests of the invitations it made, for the person who invites to compare their six words
    ///   with the invitee's ([`Team::admissions`]): it admits none by itself (SR2-08); a request of an invitation
    ///   used, expired or cancelled, or that does not prove itself, is dropped;
    /// - **registers the tokens** members asked for (their own other device), when it is an administrator of the
    ///   server: the request must be signed by a member of the group, for an invitation that member may make;
    /// - puts **the server's roles in step with the roster** (an administrator named after their enrolment becomes
    ///   one on the server too, a member no longer one is made a member), when the roster changed;
    /// - reads the **second uses** of its invitations the server saw (a leaked link), for the interface to say.
    ///
    /// What needs an administrator of the server and finds this device not one (403) is left for another.
    pub async fn serve(&mut self, now: u64) -> Result<Served, Error> {
        let mut served = Served::default();
        if self.device.group_id().is_err() || self.device.is_removed() {
            return Ok(served);
        }
        self.tidy_requests(now, &mut served)?;
        self.grant_tokens(&mut served).await?;
        self.read_alerts(&mut served).await?;
        self.read_snapshot_alerts(now, &mut served).await?;
        self.read_drops(now, &mut served).await?;
        self.retake_voided_epoch().await?;
        self.roles_in_step(&mut served).await?;
        self.anchors_in_step()?;
        self.serve_projects(now).await?;
        self.flush().await?;
        Ok(served)
    }

    /// An administrator's device holding the anchors (SR2-09): gives them to the administrators' devices that do
    /// not hold them yet, and writes the wraps of the current epoch when no anchored one is in the stream (a
    /// member's commit made it).
    fn anchors_in_step(&mut self) -> Result<(), Error> {
        if !self.device.is_admin()? {
            return Ok(());
        }
        let grants = self.device.anchor_grants(&self.meta.anchors_given)?;
        let epoch = self.device.epoch()?;
        let roster = self.device.roster()?;
        let need_recovery = self.meta.recovery_wrapped < Some(epoch)
            && self.device.holds_anchor(&roster.recovery.anchor_key);
        let need_escrow = self.meta.escrow_wrapped < Some(epoch)
            && roster
                .escrow
                .as_ref()
                .is_some_and(|e| self.device.holds_anchor(&e.anchor_key));
        let wraps = if need_recovery || need_escrow {
            let mut wraps = self.device.current_wraps()?;
            if !need_recovery {
                wraps.recovery = None;
            }
            if !need_escrow {
                wraps.escrow = None;
            }
            Some(wraps)
        } else {
            None
        };
        if grants.is_empty() && wraps.is_none() {
            return Ok(());
        }
        self.step(|t| {
            for grant in grants {
                t.meta
                    .anchors_given
                    .insert((grant.device, grant.anchor.clone()));
                t.meta.outbox.push(TeamMessage::Anchor(grant));
            }
            if let Some(wraps) = wraps {
                t.queue(wraps);
            }
            Ok(())
        })
    }

    /// The requests of this device's invitations: nothing is admitted here (SR2-08), the person who invites confirms
    /// the six words first ([`Team::admit_invitation`]). A request of an invitation used, expired or cancelled, or one
    /// that does not prove itself (its MAC, its KeyPackage), is dropped; the others wait, counted.
    fn tidy_requests(&mut self, now: u64, served: &mut Served) -> Result<(), Error> {
        let invitations = self.device.saved_state().invitations;
        let mut stale = Vec::new();
        for request in &self.meta.requests {
            if !invitations.iter().any(|i| i.id == request.invitation) {
                // Another device's invitation.
                continue;
            }
            match self.device.check_request(request, now) {
                Ok(_) => served.waiting += 1,
                Err(e) => {
                    log::warn!("team: a join request set aside: {e}");
                    stale.push(request.clone());
                }
            }
        }
        if !stale.is_empty() {
            self.step(|t| {
                t.meta.requests.retain(|r| !stale.contains(r));
                Ok(())
            })?;
        }
        Ok(())
    }

    /// An alert of an invitation refused by another device of the team (SR2-08): kept when its seal opens and its
    /// author is a device of a member (an administrator, or the member for their own device).
    fn take_refusal(&mut self, sealed: &SealedObject) {
        let body = match self.device.open(sealed) {
            Ok(plain) => wire::decode::<wire::RefusedBody>(&plain).ok(),
            Err(e) => {
                log::debug!("team: a refusal set aside: {e}");
                None
            }
        };
        let (Some(body), Some(author)) = (body, self.author_of(sealed)) else {
            return;
        };
        if author.kind != DeviceKind::Device || body.invitation != sealed.header.object.0 {
            return;
        }
        if self
            .meta
            .refusals
            .iter()
            .any(|r| r.invitation == body.invitation)
        {
            return;
        }
        self.meta.refusals.push(RefusalAlert {
            invitation: body.invitation,
            purpose: body.purpose,
            by: author.member,
            at: body.at,
        });
    }

    /// Whose a sealed object of the group is: the identity of its author's leaf at its epoch.
    fn author_of(&self, sealed: &SealedObject) -> Option<Identity> {
        let record = self.device.record(sealed.header.epoch)?;
        record
            .members
            .iter()
            .find(|m| m.signature_key == sealed.header.author)
            .and_then(|m| Identity::from_bytes(&m.identity).ok())
    }

    async fn grant_tokens(&mut self, served: &mut Served) -> Result<(), Error> {
        let roster = self.device.roster()?;
        for sealed in self.meta.token_requests.clone() {
            let drop_it = |t: &mut Team| {
                t.meta
                    .token_requests
                    .retain(|r| r.header.object != sealed.header.object);
                Ok(())
            };
            let body = match self.device.open(&sealed) {
                Ok(plain) => wire::decode::<wire::TokenRequestBody>(&plain).ok(),
                Err(e @ CryptoError::Fork { .. }) => return Err(e.into()),
                Err(e) => {
                    log::debug!("team: a token request set aside: {e}");
                    None
                }
            };
            let (Some(body), Some(author)) = (body, self.author_of(&sealed)) else {
                self.step(drop_it)?;
                continue;
            };
            if self.meta.granted.contains(&body.hash) {
                self.step(drop_it)?;
                continue;
            }
            let may = match &body.purpose {
                Purpose::NewMember(_) => roster.role(&author.member) == Some(Role::Admin),
                Purpose::OwnDevice(m) => {
                    *m == author.member || roster.role(&author.member) == Some(Role::Admin)
                }
            } && author.kind == DeviceKind::Device
                && body.invitation == sealed.header.object.0;
            if !may {
                log::warn!(
                    "team: a token request refused: its author may not make that invitation"
                );
                self.step(drop_it)?;
                continue;
            }
            match self
                .client
                .register_token(body.hash, body.expires_in.min(DEFAULT_LIFETIME))
                .await
            {
                Ok(_) => {}
                Err(e) if e.code() == Some("exists") => {}
                // Not an administrator of the server: another will.
                Err(e) if e.status() == Some(403) => return Ok(()),
                Err(e) => return Err(e.into()),
            }
            served.granted += 1;
            self.step(|t| {
                t.meta.granted.insert(body.hash);
                t.meta
                    .outbox
                    .push(TeamMessage::TokenGranted { hash: body.hash });
                drop_it(t)
            })?;
        }
        Ok(())
    }

    async fn read_alerts(&mut self, served: &mut Served) -> Result<(), Error> {
        let invitations = self.device.saved_state().invitations;
        if invitations.is_empty() {
            return Ok(());
        }
        let alerts = match self.client.token_alerts().await {
            Ok(a) => a,
            Err(e) if e.status() == Some(403) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let mut new = Vec::new();
        for alert in alerts {
            let Some(inv) = invitations
                .iter()
                .find(|i| token_hash_of(&i.secret).as_slice() == alert.token_hash.as_slice())
            else {
                continue;
            };
            let a = ReuseAlert {
                invitation: inv.id,
                reused_at: alert.reused_at,
            };
            if !self.meta.alerts.contains(&a) {
                new.push(a);
            }
        }
        if !new.is_empty() {
            served.alerts += new.len();
            self.step(|t| {
                t.meta.alerts.extend(new);
                Ok(())
            })?;
        }
        Ok(())
    }

    /// An administrator's: the snapshots the devices of the team found unreadable, as the server lists them (SR2-01).
    /// The list follows the server's: a snapshot removed leaves it. Once a minute at most.
    async fn read_snapshot_alerts(&mut self, now: u64, served: &mut Served) -> Result<(), Error> {
        if !self.device.is_admin()?
            || self
                .snapshot_alerts_at
                .is_some_and(|at| now.saturating_sub(at) < SNAPSHOT_ALERTS_EVERY)
        {
            return Ok(());
        }
        self.snapshot_alerts_at = Some(now);
        let listed = match self.client.unreadable_snapshots().await {
            Ok(l) => l,
            // Not an administrator on the server yet (the roles follow the roster).
            Err(e) if e.status() == Some(403) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let mut alerts = Vec::new();
        for u in listed {
            let (Ok(project), Ok(device)) = (
                <[u8; 16]>::try_from(u.project.as_slice()),
                <[u8; 16]>::try_from(u.device.as_slice()),
            ) else {
                continue;
            };
            let project = ProjectId(project);
            let here = self
                .meta
                .snapshot_alerts
                .iter()
                .find(|a| a.project == project && a.upto == u.upto);
            alerts.push(SnapshotAlert {
                project,
                upto: u.upto,
                posted_by: DeviceId(device),
                posted_at: u.at,
                found_at: here.map_or(u.flagged_at, |a| a.found_at),
                found_here: here.is_some_and(|a| a.found_here),
            });
        }
        if alerts != self.meta.snapshot_alerts {
            served.snapshot_alerts += alerts
                .iter()
                .filter(|a| {
                    !self
                        .meta
                        .snapshot_alerts
                        .iter()
                        .any(|b| b.project == a.project && b.upto == a.upto)
                })
                .count();
            self.step(|t| {
                t.meta.snapshot_alerts = alerts;
                Ok(())
            })?;
        }
        Ok(())
    }

    /// An administrator's: the epoch whose commit was declared null is kept to the administrators until their next
    /// commit (SR2-02). When nothing else took it, a rotation does, so that the members' own changes go on.
    async fn retake_voided_epoch(&mut self) -> Result<(), Error> {
        if !self.device.is_admin()? {
            return Ok(());
        }
        let current = self.device.epoch()?;
        let voided = self
            .meta
            .nulls
            .iter()
            .any(|n| n.project.is_none() && n.epoch == current && (n.voided || n.void_sent));
        if voided {
            match self.apply(Change::Rotate).await {
                Ok(_) | Err(Error::Busy) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// An administrator's: the entries the server's operator removed (SR2-02), as the server lists them. Every five
    /// minutes at most, at once after a hole seen in the team's stream.
    async fn read_drops(&mut self, now: u64, served: &mut Served) -> Result<(), Error> {
        if !self.device.is_admin()?
            || self
                .drops_at
                .is_some_and(|at| now.saturating_sub(at) < DROPS_EVERY)
        {
            return Ok(());
        }
        self.drops_at = Some(now);
        let mut alerts = Vec::new();
        for d in self.client.dropped().await? {
            let Ok(device) = <[u8; 16]>::try_from(d.device.as_slice()) else {
                continue;
            };
            let project = match d.project.as_deref().map(<[u8; 16]>::try_from) {
                None => None,
                Some(Ok(p)) => Some(ProjectId(p)),
                Some(Err(_)) => continue,
            };
            alerts.push(DropAlert {
                project,
                seq: d.seq,
                commit: d.kind == Kind::Commit,
                epoch: d.epoch,
                written_by: DeviceId(device),
                written_at: d.written_at,
                dropped_at: d.at,
            });
        }
        if alerts != self.meta.drop_alerts {
            served.drop_alerts += alerts
                .iter()
                .filter(|a| !self.meta.drop_alerts.contains(a))
                .count();
            self.step(|t| {
                t.meta.drop_alerts = alerts;
                Ok(())
            })?;
        }
        Ok(())
    }

    /// The entries the server's operator removed (SR2-02), the newest first: an administrator's.
    pub fn drop_alerts(&self) -> Vec<DropAlert> {
        let mut out = self.meta.drop_alerts.clone();
        out.sort_by_key(|a| std::cmp::Reverse((a.dropped_at, a.seq)));
        out
    }

    /// A hole in the team's stream, from `from` (the last number read) to `to` (the entry after it): the commits the
    /// operator removed there that this device did not take (kept before it joined, or never read past) are forgotten,
    /// so that the commit that takes their epoch again is not a fork (SR2-02).
    fn forget_dropped_commits(&mut self, dropped: &[DroppedInfo], from: u64, to: u64) {
        let current = self.device.epoch().ok();
        for d in dropped {
            if d.project.is_some() || d.kind != Kind::Commit || d.seq <= from || d.seq >= to {
                continue;
            }
            let Some(epoch) = d.epoch else { continue };
            if current.is_none_or(|c| c <= epoch) {
                self.meta.commits.remove(&epoch);
                self.meta.early.remove(&epoch);
            }
        }
    }

    /// The snapshots found unreadable (SR2-01), the newest first.
    pub fn snapshot_alerts(&self) -> Vec<SnapshotAlert> {
        let mut out = self.meta.snapshot_alerts.clone();
        out.sort_by_key(|a| std::cmp::Reverse(a.found_at));
        out
    }

    /// Hides an alert of a snapshot found unreadable on this device (a member's: an administrator's follows the
    /// server's list until the snapshot is removed).
    pub fn dismiss_snapshot_alert(&mut self, project: ProjectId, upto: u64) -> Result<(), Error> {
        self.step(|t| {
            t.meta
                .snapshot_alerts
                .retain(|a| !(a.project == project && a.upto == upto));
            Ok(())
        })
    }

    /// Removes a project's snapshot from the server (an administrator's, SR2-01): the one before, and the journal
    /// after it, take its place for every device. Its alert goes. False when the server had none there.
    pub async fn remove_snapshot(&mut self, project: ProjectId, upto: u64) -> Result<bool, Error> {
        if !self.device.is_admin()? {
            return Err(Error::Refused(
                "only an administrator removes a snapshot".into(),
            ));
        }
        let removed =
            match self.client.remove_snapshot(&project.0, upto).await {
                Ok(r) => r,
                Err(e) if e.code() == Some("snapshot_refused") => return Err(Error::Refused(
                    "this snapshot is the start of the project's stream now: its messages are gone"
                        .into(),
                )),
                Err(e) => return Err(e.into()),
            };
        self.dismiss_snapshot_alert(project, upto)?;
        // The server's list read again at the next `serve`.
        self.snapshot_alerts_at = None;
        Ok(removed)
    }

    /// This device found a project's snapshot unreadable: noted, for the interface.
    pub(crate) fn note_snapshot_alert(&mut self, alert: SnapshotAlert) -> Result<(), Error> {
        if self
            .meta
            .snapshot_alerts
            .iter()
            .any(|a| a.project == alert.project && a.upto == alert.upto)
        {
            return Ok(());
        }
        self.step(|t| {
            t.meta.snapshot_alerts.push(alert);
            Ok(())
        })
    }

    async fn roles_in_step(&mut self, served: &mut Served) -> Result<(), Error> {
        let epoch = self.device.epoch()?;
        if self.meta.roles_epoch == Some(epoch) {
            return Ok(());
        }
        let roster = self.device.roster()?;
        let leaves = self.device.leaves()?;
        let devices = self.client.devices().await?;
        for leaf in leaves.iter().filter(|l| l.kind == DeviceKind::Device) {
            let want = match roster.role(&leaf.member) {
                Some(Role::Admin) => wire::Role::Admin,
                // A machine of the team (OE-1): the server keeps it off its administrators' routes.
                Some(Role::Service) => wire::Role::Service,
                _ => wire::Role::Member,
            };
            let Some(on_server) = devices.iter().find(|d| d.id == leaf.device.0) else {
                continue;
            };
            if on_server.revoked || on_server.role == want {
                continue;
            }
            match self.client.set_role(&leaf.device.0, want).await {
                Ok(()) => served.roles += 1,
                // Not an administrator of the server: another will, at their next reading.
                Err(e) if e.status() == Some(403) => return Ok(()),
                Err(e) if e.code() == Some("last_admin") => {}
                Err(e) => return Err(e.into()),
            }
        }
        self.step(|t| {
            t.meta.roles_epoch = Some(epoch);
            Ok(())
        })?;
        Ok(())
    }

    // === The recovery key, the escrow key ===

    /// A new recovery key (an administrator's): the roster takes its public half, the old one's leaf (if it is in
    /// the group) leaves in the same commit. Its 24 words are to show once.
    pub async fn replace_recovery(&mut self) -> Result<RecoveryKey, Error> {
        let key = RecoveryKey::from_entropy(random::<32>());
        let public = key.public(self.device.crypto())?;
        // This device signs the wraps of the new key: it keeps its anchor (never the words).
        self.step(|t| {
            t.device.keep_anchor(key.anchor());
            Ok(())
        })?;
        self.apply(Change::ReplaceRecovery(public)).await?;
        Ok(key)
    }

    /// Sets the organisation's escrow key, or takes it away (an administrator's). Every member sees it (C7-B).
    pub async fn set_escrow(&mut self, escrow: Option<EscrowTeamHalf>) -> Result<Applied, Error> {
        let escrow = match escrow {
            Some(half) => {
                self.step(|t| {
                    t.device.keep_anchor(half.anchor.clone());
                    Ok(())
                })?;
                Some(half.public)
            }
            None => None,
        };
        self.apply(Change::SetEscrow(escrow)).await
    }

    // === Another device of one's own, by a short code (`pairing.rs`) ===

    /// Shows a short code for another device of this member: an invitation of one's own device (an hour), and the
    /// first message of the PAKE on the server's relay. Then [`Team::pair_step`] until it is sent or refused.
    pub async fn pair_start(&mut self, now: u64) -> Result<Pairing, Error> {
        let me = self.identity().member;
        self.pair_start_for(Purpose::OwnDevice(me), now).await
    }

    /// Shows a short code that enrolls a **machine of the team** (OE-1): an invitation under the role of service (an
    /// hour), an administrator's alone, typed on the machine (`escouade-server team pair`, or « Rejoindre par un
    /// code » in its window). The machine then shows its six words, compared before « Admettre », as for anyone.
    pub async fn pair_start_service(&mut self, now: u64) -> Result<Pairing, Error> {
        if !self.may_sign(escouade_team_crypto::Act::Member) {
            return Err(CryptoError::NotAdmin.into());
        }
        self.pair_start_for(Purpose::NewMember(Role::Service), now)
            .await
    }

    async fn pair_start_for(&mut self, purpose: Purpose, now: u64) -> Result<Pairing, Error> {
        let invited = self.invite_for(purpose, now, OWN_DEVICE_LIFETIME).await?;
        let mut code = PairCode::new("");
        let (spake, message) = Pairing::start(&code);
        let opened = self.client.pair_open(&message).await?;
        code.slot = opened.slot;
        Ok(Pairing {
            code,
            link: invited.link,
            expires_at: now.saturating_add(opened.expires_in),
            spake: Some(spake),
            key: None,
            state: PairState::Waiting,
        })
    }

    /// One step of a pairing: reads the new device's try; on a wrong code, closes the slot; on the right one, sends
    /// the answer (the invitation link, sealed under the PAKE's key) once its token is registered.
    pub async fn pair_step(&mut self, pairing: &mut Pairing) -> Result<PairState, Error> {
        if matches!(pairing.state, PairState::Sent | PairState::WrongCode) {
            return Ok(pairing.state.clone());
        }
        if pairing.key.is_none() {
            let tried = match self.client.pair_tried(&pairing.code.slot).await {
                Ok(t) => t,
                Err(e) if e.code() == Some("pair_gone") => return Err(Error::CodeGone),
                Err(e) => return Err(e.into()),
            };
            let Some(tried) = tried else {
                return Ok(PairState::Waiting);
            };
            if !pairing.check_try(&tried.message, &tried.confirm) {
                let _ = self.client.pair_close(&pairing.code.slot).await;
                pairing.state = PairState::WrongCode;
                return Ok(PairState::WrongCode);
            }
        }
        let hash = token_hash(&pairing.link);
        if !self.meta.granted.contains(&hash) {
            self.sync().await?;
        }
        if !self.meta.granted.contains(&hash) {
            pairing.state = PairState::WaitingForToken;
            return Ok(PairState::WaitingForToken);
        }
        let sealed = pairing
            .sealed_answer()
            .ok_or_else(|| Error::Refused("no key for the answer".into()))?;
        self.client.pair_answer(&pairing.code.slot, sealed).await?;
        pairing.state = PairState::Sent;
        Ok(PairState::Sent)
    }

    /// Admits the request of one of this device's invitations whose six words the person who invites confirmed were
    /// those of the invitee's screen (SR2-08): the words shown ([`Team::admissions`]), given back. Other words, or a
    /// request gone: [`InvitationError::CodeMismatch`], nothing admitted.
    pub async fn admit_invitation(
        &mut self,
        invitation: [u8; 16],
        words: &str,
        now: u64,
    ) -> Result<Applied, Error> {
        let request = self
            .meta
            .requests
            .iter()
            .find(|r| {
                r.invitation == invitation && AdmissionCode::of_request(r).matches_words(words)
            })
            .cloned()
            .ok_or(CryptoError::Invitation(InvitationError::CodeMismatch))?;
        self.admit(&request, words, now).await
    }

    /// The person who invites said the six words of a request differed from the invitee's (SR2-08): the invitation
    /// is cancelled (no one comes in by it any more), its requests dropped, the devices that made them cut on the
    /// server (when this device may: an administrator of the server), and the administrators told by a sealed alert
    /// in the team's stream.
    pub async fn refuse(&mut self, invitation: [u8; 16], now: u64) -> Result<(), Error> {
        let purpose = self
            .device
            .saved_state()
            .invitations
            .iter()
            .find(|i| i.id == invitation)
            .map(|i| i.purpose.clone())
            .ok_or(CryptoError::Invitation(InvitationError::Unknown))?;
        let leaves: BTreeSet<DeviceId> = self
            .device
            .leaves()?
            .into_iter()
            .map(|i| i.device)
            .collect();
        // The devices the requests' KeyPackages name, none of the group.
        let askers: Vec<DeviceId> = self
            .meta
            .requests
            .iter()
            .filter(|r| r.invitation == invitation)
            .filter_map(|r| self.device.key_package_identity(&r.key_package).ok())
            .map(|i| i.device)
            .filter(|d| !leaves.contains(d))
            .collect();
        let me = self.identity().member;
        let body = wire::RefusedBody {
            invitation,
            purpose: purpose.clone(),
            at: now,
        };
        let sealed = self.device.seal(
            TEAM_OBJECTS,
            ObjectId(invitation),
            REFUSAL_VERSION,
            &wire::encode(&body),
        )?;
        self.step(|t| {
            t.device.refuse_invitation(&invitation)?;
            t.meta.requests.retain(|r| r.invitation != invitation);
            t.meta.invite_projects.retain(|(id, _)| *id != invitation);
            if !t.meta.refusals.iter().any(|r| r.invitation == invitation) {
                t.meta.refusals.push(RefusalAlert {
                    invitation,
                    purpose,
                    by: me,
                    at: now,
                });
            }
            t.meta.outbox.push(TeamMessage::Refused(sealed));
            Ok(())
        })?;
        self.flush().await?;
        for device in askers {
            match self.client.revoke(&device.0).await {
                Ok(_) => {}
                Err(e) => log::info!("team: the device of a refused request not cut: {e}"),
            }
        }
        Ok(())
    }

    /// Admits a join request read in the stream (an administrator, or the member for their own device), with the six
    /// words the person who invites confirmed were those of the invitee's screen (SR2-08).
    pub async fn admit(
        &mut self,
        request: &JoinRequest,
        words: &str,
        now: u64,
    ) -> Result<Applied, Error> {
        let applied = self
            .apply(Change::Admit(request.clone(), words.to_string(), now))
            .await?;
        let joiner = self.device.key_package_identity(&request.key_package).ok();
        self.step(|t| {
            t.meta.requests.retain(|r| r != request);
            // The projects the invitation gives access to (TS-5), given once the member is in ([`Team::serve`]).
            if let (Some(joiner), Some(i)) = (
                joiner,
                t.meta
                    .invite_projects
                    .iter()
                    .position(|(id, _)| *id == request.invitation),
            ) {
                let (_, projects) = t.meta.invite_projects.remove(i);
                t.meta
                    .grants
                    .extend(projects.into_iter().map(|p| (joiner.member, p)));
            }
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
        Ok(self
            .client
            .put_file(&project.0, &file.0, wire::encode(&sealed))
            .await?)
    }

    /// A file of `project`, opened; None when the server has none there.
    pub async fn get_file(
        &mut self,
        project: ProjectId,
        file: ObjectId,
    ) -> Result<Option<Vec<u8>>, Error> {
        let Some(bytes) = self.client.get_file(&project.0, &file.0).await? else {
            return Ok(None);
        };
        let sealed: SealedObject = wire::decode(&bytes).map_err(Error::Encoding)?;
        if sealed.header.object != file || sealed.header.project != project {
            return Err(Error::Refused("the server gave another file".into()));
        }
        Ok(Some(self.device.open(&sealed)?))
    }

    /// Deletes a file of `project`: the server lets its author or an administrator alone.
    pub async fn delete_file(&mut self, project: ProjectId, file: ObjectId) -> Result<(), Error> {
        Ok(self.client.delete_file(&project.0, &file.0).await?)
    }
}

fn identity(keys: &DeviceKeys) -> Identity {
    match keys.recovery_leaf {
        Some(leaf) => Identity {
            member: MemberId(keys.member),
            device: DeviceId(leaf),
            kind: DeviceKind::Recovery,
        },
        None => Identity {
            member: MemberId(keys.member),
            device: DeviceId(keys.device),
            kind: DeviceKind::Device,
        },
    }
}

/// The recovery wrap to follow (SR2-09): of those in the team's stream that the anchor of these words signed, the
/// newest that passes every check of [`TeamDevice::recover_with`] (tried in memory first, nothing sent), and the
/// group's id it is sealed for. A wrap anyone else wrote is passed over; one that the anchor signed but that fails a
/// check (a former administrator's) does not hide an older good one.
async fn recovery_wrap(
    client: &ServerClient,
    key: &RecoveryKey,
    words: &str,
    team_words: &str,
) -> Result<(Vec<u8>, RecoveryWrap), Error> {
    let mut anchored = anchored_recovery_wraps(client, key).await?;
    // The newest epoch first, and in an epoch the last written first (the sort keeps that order).
    anchored.reverse();
    anchored.sort_by_key(|(_, w)| std::cmp::Reverse(w.epoch));
    let mut first_error = None;
    for (group, wrap) in anchored {
        match TeamDevice::recover(words, team_words, &wrap, &group) {
            Ok(_) => return Ok((group, wrap)),
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    Err(first_error.map_or(Error::NoRecoveryWrap, Error::Crypto))
}

/// The recovery wraps of the team's stream signed by the anchor of these words, in the stream's order (the latest
/// last), with the group's id each is sealed for.
async fn anchored_recovery_wraps(
    client: &ServerClient,
    key: &RecoveryKey,
) -> Result<Vec<(Vec<u8>, RecoveryWrap)>, Error> {
    let anchor = key.anchor().public();
    let mut since = 0;
    let mut found: Vec<(Vec<u8>, RecoveryWrap)> = Vec::new();
    loop {
        let page = client.read_team(since, None).await?;
        for e in page.entries {
            since = e.seq;
            if e.kind != Kind::Message {
                continue;
            }
            if let Ok(TeamMessage::Recovery { group, wrap }) =
                wire::decode::<TeamMessage>(&e.payload)
            {
                let anchored =
                    wrap.is_anchored(&group, &TeamFingerprint(wrap.signed.team), &anchor);
                if anchored {
                    found.push((group, wrap));
                }
            }
        }
        if !page.more {
            break;
        }
    }
    Ok(found)
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
type Loaded = (
    TeamDevice<TeamProvider>,
    Meta,
    BTreeSet<u64>,
    BTreeSet<(ProjectId, u64)>,
);

fn load(dir: &Path, keys: &DeviceKeys) -> Result<Loaded, Error> {
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
    let mut project_records: BTreeMap<ProjectId, Vec<EpochRecord>> = BTreeMap::new();
    for key in storage.keys_with(&crate::storage::ours(b"precord/"))? {
        let name = &key[b"escouade-team/".len()..];
        let Some(rest) = name.strip_prefix(b"precord/".as_slice()) else {
            continue;
        };
        let Ok(project) = <[u8; 16]>::try_from(&rest[..rest.len().min(16)]) else {
            continue;
        };
        if let Some(r) = storage.get_ours::<EpochRecord>(name)? {
            project_records
                .entry(ProjectId(project))
                .or_default()
                .push(r);
        }
    }
    let mut device = TeamDevice::restore(
        TeamProvider::new(storage),
        signer(keys),
        identity(keys),
        state,
        records,
    )?;
    let mut project_stored = BTreeSet::new();
    for (project, records) in project_records {
        project_stored.extend(records.iter().map(|r| (project, r.epoch)));
        if device.project_ids().contains(&project) {
            device.restore_project_records(project, records)?;
        }
    }
    Ok((device, meta, stored, project_stored))
}
