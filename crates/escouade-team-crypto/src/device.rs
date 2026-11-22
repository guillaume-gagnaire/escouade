//! A device of the team: its MLS client, and what it keeps above MLS (the records of its epochs, its checkpoint).
//!
//! Every change of the group is **staged** first ([`TeamDevice::stage`]): its commit goes to the server, and the
//! device merges it only once the server took it ([`TeamDevice::confirm_pending`]). A server that already holds a
//! commit for that epoch refuses the second one; the device then drops its own ([`TeamDevice::abandon_pending`]),
//! processes the commit that won, and stages again if the change is still to do: it never holds two epochs that
//! diverge. The calls of the mock-up that commit and merge at once (`rotate`, `admit`…) stay, for the tests that play
//! the server in memory (`relay.rs`).
//!
//! The device is generic over its provider (`openmls_traits::OpenMlsProvider`): the mock-up's tests take the one in
//! memory, the client (`escouade-team`) its SQLite storage. What MLS does not keep (the records, the invitations,
//! the pending commit) is given out by [`TeamDevice::saved_state`] and [`TeamDevice::records`], and taken back by
//! [`TeamDevice::restore`].

use std::collections::BTreeMap;

use openmls::prelude::tls_codec::{Deserialize as _, Serialize as _};
use openmls::prelude::{
    Capabilities, CredentialType, CredentialWithKey, ExtensionType, GroupId, KeyPackage,
    LeafNodeIndex, LeafNodeParameters, MlsGroup, MlsGroupJoinConfig, MlsMessageBodyIn,
    MlsMessageIn, MlsMessageOut, ProcessedMessageContent, ProtocolVersion, Sender, StagedWelcome,
    MIXED_CIPHERTEXT_WIRE_FORMAT_POLICY,
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::random::OpenMlsRand;
use openmls_traits::signatures::Signer;
use openmls_traits::OpenMlsProvider;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::beacon::Beacon;
use crate::content::{ObjectHeader, SealedObject};
use crate::history::{walk_back, HistoryLink};
use crate::identity::{DeviceId, DeviceKind, Identity, MemberId, ObjectId, ProjectId};
use crate::invite::{
    join_mac, verify_join_mac, verify_welcome_confirmation, welcome_confirmation, Invitation,
    InvitationError, InviteLink, JoinRequest, Purpose, WelcomeDelivery,
};
use crate::keys::{head, EpochMember, EpochRecord, CONTENT_LABEL, HISTORY_LABEL};
use crate::policy::check_commit;
use crate::recovery::{EscrowWrap, RecoveryKey, RecoveryWrap};
use crate::roster::{
    EscrowPublic, Genesis, RecoveryPublic, Role, Roster, TeamFingerprint, ROSTER_EXTENSION,
};
use crate::{mls, Error, CIPHERSUITE};

/// What goes to the server after an operation.
#[derive(Debug, Default)]
pub struct Publication {
    /// The commit, and the epoch it applies to (the server keeps the first one for each epoch).
    pub commit: Option<(u64, Vec<u8>)>,
    /// The Welcome of an admitted invitee, under the invitation's id.
    pub welcome: Option<([u8; 16], WelcomeDelivery)>,
    pub history: Option<HistoryLink>,
    pub recovery: Option<RecoveryWrap>,
    pub escrow: Option<EscrowWrap>,
}

impl Publication {
    /// The second half of a change (after the server took its commit) joined to the first.
    fn join(mut self, rest: Publication) -> Publication {
        self.welcome = rest.welcome;
        self.history = rest.history;
        self.recovery = rest.recovery;
        self.escrow = rest.escrow;
        self
    }
}

/// The highest epoch a device has seen, and its head: kept beside its keys (in the keychain), it refuses to go
/// back, like the `serial` of the store's revocation list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub epoch: u64,
    #[serde(with = "hex::serde")]
    pub head: [u8; 32],
}

/// What a device makes of a head someone cites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadCheck {
    /// The same as this device's for that epoch.
    Same,
    /// A later epoch: the device has commits to process first.
    Behind,
    /// An epoch before what this device knows (before it joined, its history not read).
    Unknown,
}

/// What processing a commit did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Processed {
    /// The group is now at this epoch.
    Epoch(u64),
    /// The commit removed this device.
    Removed,
}

/// A change of the group, as a device stages it (and stages it again after losing its epoch to another commit).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Change {
    /// Removes a member: all their devices, and their entry in the roster.
    RemoveMember(MemberId),
    /// Revokes a device (a stolen one): by its member, or by an administrator.
    RemoveDevice(DeviceId),
    /// "Change the team's keys": an empty commit with a path.
    Rotate,
    SetRole(MemberId, Role),
    SetEscrow(Option<EscrowPublic>),
    ReplaceRecovery(RecoveryPublic),
    /// Admits an invitee's request, at the hour `now`.
    Admit(JoinRequest, u64),
}

/// The commit a device staged and sent, until the server takes it or refuses it. Kept in the device's saved state:
/// a device that stops between the two finds it again, and the server's log says which way it went.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct PendingCommit {
    /// The epoch the commit applies to.
    pub from_epoch: u64,
    /// The commit, as it goes to the server.
    pub commit: Vec<u8>,
    /// The Welcome of an admission, sent with its confirmation once the commit is merged.
    pub welcome: Option<Vec<u8>>,
    pub invitation_id: Option<[u8; 16]>,
    pub invitation_secret: Option<[u8; 32]>,
}

/// What a device keeps beside the MLS group (which its provider's storage holds) and its records (kept one by one):
/// the group's id, the invitations it made, the link it joins by, whether it was removed, the commit it waits on.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SavedState {
    pub group_id: Option<Vec<u8>>,
    pub invitations: Vec<Invitation>,
    /// The invitation link being used to join (`InviteLink::to_url`).
    pub joining: Option<String>,
    pub removed: bool,
    pub pending: Option<PendingCommit>,
    /// The MLS groups of the projects kept to some members, and their commits waiting (their records are kept
    /// apart, as the team's: [`TeamDevice::project_records`]).
    #[serde(default)]
    pub projects: Vec<crate::project::SavedProject>,
}

/// The leaf capabilities every device of the team announces: the suite, the roster's extension, basic credentials.
pub fn capabilities() -> Capabilities {
    Capabilities::new(
        None,
        Some(&[CIPHERSUITE]),
        Some(&[ExtensionType::Unknown(ROSTER_EXTENSION)]),
        None,
        Some(&[CredentialType::Basic]),
    )
}

/// No past epoch's message secrets are kept (`max_past_epochs` 0): only commits travel as MLS messages, each of the
/// epoch it applies to, and the content has its own keys (`keys.rs`). The storage forgets them at each epoch.
pub(crate) fn join_config() -> MlsGroupJoinConfig {
    MlsGroupJoinConfig::builder()
        .wire_format_policy(MIXED_CIPHERTEXT_WIRE_FORMAT_POLICY)
        .use_ratchet_tree_extension(true)
        .max_past_epochs(0)
        .build()
}

pub(crate) fn random_nonce(rand: &impl OpenMlsRand) -> Result<[u8; 24], Error> {
    rand.random_array().map_err(|_| Error::Random)
}

pub(crate) fn encode(message: &MlsMessageOut) -> Result<Vec<u8>, Error> {
    message.tls_serialize_detached().map_err(mls)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<MlsMessageIn, Error> {
    MlsMessageIn::tls_deserialize_exact(bytes).map_err(mls)
}

pub struct TeamDevice<P: OpenMlsProvider = OpenMlsRustCrypto> {
    pub(crate) provider: P,
    pub(crate) signer: SignatureKeyPair,
    pub(crate) identity: Identity,
    pub(crate) group: Option<MlsGroup>,
    pub(crate) records: BTreeMap<u64, EpochRecord>,
    pub(crate) checkpoint: Option<Checkpoint>,
    invitations: BTreeMap<[u8; 16], Invitation>,
    joining: Option<InviteLink>,
    removed: bool,
    pending: Option<PendingCommit>,
    /// The MLS groups of the projects kept to some members (`project.rs`).
    pub(crate) projects: BTreeMap<ProjectId, crate::project::ProjectGroup>,
}

impl TeamDevice<OpenMlsRustCrypto> {
    /// A new device of `member`, in memory, its signature key made here (in the client: kept in the keychain, never
    /// exported).
    pub fn new(member: MemberId) -> Result<Self, Error> {
        let provider = OpenMlsRustCrypto::default();
        let signer = SignatureKeyPair::new(CIPHERSUITE.signature_algorithm()).map_err(mls)?;
        let device = DeviceId::random(provider.rand())?;
        Ok(Self::with_identity(
            provider,
            signer,
            Identity {
                member,
                device,
                kind: DeviceKind::Device,
            },
        ))
    }

    /// [`TeamDevice::recover_with`], in memory.
    pub fn recover(
        words: &str,
        wrap: &RecoveryWrap,
        group_id: &[u8],
    ) -> Result<(Self, Publication), Error> {
        Self::recover_with(OpenMlsRustCrypto::default(), words, wrap, group_id)
    }
}

/// An invitation as a device started again keeps it. One made before the links became the app's (SR2-08) is expired
/// at once: its `https://` link may have opened the server's page, which reads the secret. It lets no one in, and
/// is listed so (`stage_admit` refuses it besides).
pub(crate) fn kept_invitation(mut invitation: Invitation) -> Invitation {
    if !invitation.app_link {
        invitation.expires_at = 0;
    }
    invitation
}

impl<P: OpenMlsProvider> TeamDevice<P> {
    /// A device on `provider`, its keys and identity given (the client reads them from the keychain).
    pub fn with_identity(provider: P, signer: SignatureKeyPair, identity: Identity) -> Self {
        Self {
            provider,
            signer,
            identity,
            group: None,
            records: BTreeMap::new(),
            checkpoint: None,
            invitations: BTreeMap::new(),
            joining: None,
            removed: false,
            pending: None,
            projects: BTreeMap::new(),
        }
    }

    /// A device started again: its group loaded from the provider's storage, its records and state given back. Its
    /// checkpoint is that of the group's epoch; the caller compares it to the one kept apart
    /// ([`Self::check_restored`]).
    pub fn restore(
        provider: P,
        signer: SignatureKeyPair,
        identity: Identity,
        state: SavedState,
        records: Vec<EpochRecord>,
    ) -> Result<Self, Error> {
        let mut device = Self::with_identity(provider, signer, identity);
        device.records = records.into_iter().map(|r| (r.epoch, r)).collect();
        device.invitations = state
            .invitations
            .iter()
            .map(|i| (i.id, kept_invitation(i.clone())))
            .collect();
        device.joining = match &state.joining {
            Some(url) => Some(InviteLink::parse_kept(url)?),
            None => None,
        };
        device.removed = state.removed;
        device.pending = state.pending.clone();
        if let Some(id) = &state.group_id {
            let group = MlsGroup::load(device.provider.storage(), &GroupId::from_slice(id))
                .map_err(mls)?
                .ok_or_else(|| Error::Mls("the saved group is not in the storage".into()))?;
            let epoch = group.epoch().as_u64();
            let record = device
                .records
                .get(&epoch)
                .ok_or(Error::UnknownEpoch(epoch))?;
            device.checkpoint = Some(Checkpoint {
                epoch,
                head: record.head,
            });
            device.group = Some(group);
        }
        device.load_projects(&state.projects)?;
        Ok(device)
    }

    /// What to keep beside the group and the records, to [`Self::restore`] the device.
    pub fn saved_state(&self) -> SavedState {
        SavedState {
            group_id: self
                .group
                .as_ref()
                .map(|g| g.group_id().as_slice().to_vec()),
            invitations: self.invitations.values().cloned().collect(),
            joining: self.joining.as_ref().map(InviteLink::to_url),
            removed: self.removed,
            pending: self.pending.clone(),
            projects: self.saved_projects(),
        }
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn identity(&self) -> Identity {
        self.identity
    }

    pub fn signature_key(&self) -> Vec<u8> {
        self.signer.to_public_vec()
    }

    pub fn crypto(&self) -> &impl openmls_traits::crypto::OpenMlsCrypto {
        self.provider.crypto()
    }

    pub(crate) fn credential_with_key(&self) -> CredentialWithKey {
        CredentialWithKey {
            credential: self.identity.credential(),
            signature_key: self.signer.public().into(),
        }
    }

    pub(crate) fn group(&self) -> Result<&MlsGroup, Error> {
        if self.removed {
            return Err(Error::NotInTeam);
        }
        self.group.as_ref().ok_or(Error::NotInTeam)
    }

    pub fn group_id(&self) -> Result<Vec<u8>, Error> {
        Ok(self.group()?.group_id().as_slice().to_vec())
    }

    pub fn epoch(&self) -> Result<u64, Error> {
        Ok(self.group()?.epoch().as_u64())
    }

    pub fn roster(&self) -> Result<Roster, Error> {
        Roster::from_extensions(self.group()?.extensions())
    }

    pub fn fingerprint(&self) -> Result<TeamFingerprint, Error> {
        Ok(TeamFingerprint::of(
            &self.group_id()?,
            &self.roster()?.genesis,
        ))
    }

    /// The leaves of the current epoch: who is in the team, device by device (the verified journal of the members).
    pub fn leaves(&self) -> Result<Vec<Identity>, Error> {
        self.group()?
            .members()
            .map(|m| Identity::from_credential(&m.credential))
            .collect()
    }

    /// The escrow key, when the team has one: the client shows it to every member, always
    /// ("This team has an escrow key: the organisation can read its content").
    pub fn escrow(&self) -> Result<Option<EscrowPublic>, Error> {
        Ok(self.roster()?.escrow)
    }

    pub fn is_admin(&self) -> Result<bool, Error> {
        let roster = self.roster()?;
        Ok(match self.identity.kind {
            DeviceKind::Recovery => {
                self.signer.public() == roster.recovery.signature_key.as_slice()
            }
            DeviceKind::Device => roster.role(&self.identity.member) == Some(Role::Admin),
        })
    }

    pub fn checkpoint(&self) -> Option<Checkpoint> {
        self.checkpoint
    }

    /// The epochs whose record this device holds.
    pub fn known_epochs(&self) -> Vec<u64> {
        self.records.keys().copied().collect()
    }

    /// The record of an epoch (for the tests, the vectors and the audit: in the client it never leaves the device).
    pub fn record(&self, epoch: u64) -> Option<&EpochRecord> {
        self.records.get(&epoch)
    }

    /// Every record this device holds, oldest first (the client keeps each in its encrypted storage).
    pub fn records(&self) -> impl Iterator<Item = &EpochRecord> {
        self.records.values()
    }

    /// The commit this device staged and waits on, if any.
    pub fn pending(&self) -> Option<&PendingCommit> {
        self.pending.as_ref()
    }

    // === The team's birth ===

    /// Creates the team: the group (epoch 0) with this device as its only leaf, the owner an administrator, the
    /// recovery key made and its words to show once, the escrow key when the organisation asks for it.
    pub fn create_team(
        &mut self,
        created_at: u64,
        escrow: Option<EscrowPublic>,
    ) -> Result<(RecoveryKey, Publication), Error> {
        let recovery = RecoveryKey::generate(self.provider.rand())?;
        let roster = Roster {
            genesis: Genesis {
                owner: self.identity.member,
                owner_key: self.signer.to_public_vec(),
                created_at,
            },
            members: [(self.identity.member, Role::Admin)].into_iter().collect(),
            recovery: recovery.public(self.provider.crypto())?,
            escrow,
        };
        let group_id: [u8; 32] = self
            .provider
            .rand()
            .random_array()
            .map_err(|_| Error::Random)?;
        let group = MlsGroup::builder()
            .with_group_id(GroupId::from_slice(&group_id))
            .ciphersuite(CIPHERSUITE)
            .with_wire_format_policy(MIXED_CIPHERTEXT_WIRE_FORMAT_POLICY)
            .use_ratchet_tree_extension(true)
            .max_past_epochs(0)
            .with_group_context_extensions(roster.to_extensions()?)
            .with_capabilities(capabilities())
            .build(&self.provider, &self.signer, self.credential_with_key())
            .map_err(mls)?;
        self.group = Some(group);
        let record = self.enter_epoch()?;
        let (recovery_wrap, escrow_wrap) = self.wraps(&record)?;
        Ok((
            recovery,
            Publication {
                recovery: Some(recovery_wrap),
                escrow: escrow_wrap,
                ..Default::default()
            },
        ))
    }

    /// Records the group's current epoch, and moves the checkpoint.
    fn enter_epoch(&mut self) -> Result<EpochRecord, Error> {
        let group = self.group()?;
        let crypto = self.provider.crypto();
        let epoch = group.epoch().as_u64();
        let group_id = group.group_id().as_slice().to_vec();
        let export = |label: &str| -> Result<[u8; 32], Error> {
            let bytes = group
                .export_secret(crypto, label, &group_id, 32)
                .map_err(mls)?;
            bytes
                .try_into()
                .map_err(|_| Error::Encoding("export length".into()))
        };
        let record = EpochRecord {
            epoch,
            content_secret: export(CONTENT_LABEL)?,
            history_key: export(HISTORY_LABEL)?,
            head: head(&group_id, epoch, group.epoch_authenticator().as_slice()),
            members: group
                .members()
                .map(|m| EpochMember {
                    identity: crate::identity::Identity::from_credential(&m.credential)
                        .map(|i| i.to_bytes())
                        .unwrap_or_default(),
                    signature_key: m.signature_key,
                })
                .collect(),
        };
        self.records.insert(epoch, record.clone());
        self.checkpoint = Some(Checkpoint {
            epoch,
            head: record.head,
        });
        Ok(record)
    }

    /// The recovery wrap of an epoch, and the escrow wrap when the roster has an escrow key.
    fn wraps(&self, record: &EpochRecord) -> Result<(RecoveryWrap, Option<EscrowWrap>), Error> {
        let group = self.group()?;
        let roster = self.roster()?;
        let crypto = self.provider.crypto();
        let group_id = group.group_id().as_slice();
        let group_info = encode(
            &group
                .export_group_info(crypto, &self.signer, true)
                .map_err(mls)?,
        )?;
        let recovery = RecoveryWrap::seal(crypto, &roster.recovery, group_id, record, group_info)?;
        let escrow = match &roster.escrow {
            Some(escrow) => Some(EscrowWrap::seal(crypto, escrow, group_id, record)?),
            None => None,
        };
        Ok((recovery, escrow))
    }

    // === Commits ===

    /// Stages a change: builds its commit, checks it against the rules as every other device will, and keeps it
    /// pending. The publication holds the commit only; the rest comes with [`Self::confirm_pending`], once the
    /// server has taken the commit.
    pub fn stage(&mut self, change: &Change) -> Result<Publication, Error> {
        if self.pending.is_some() {
            return Err(Error::Mls(
                "a commit of this device is pending: confirm or abandon it first".into(),
            ));
        }
        match change {
            Change::RemoveMember(member) => {
                self.require_admin()?;
                let mut roster = self.roster()?;
                roster.members.remove(member);
                let leaves =
                    self.leaves_where(|i| i.member == *member && i.kind == DeviceKind::Device)?;
                self.stage_commit(Vec::new(), leaves, Some(roster), None)
            }
            Change::RemoveDevice(device) => {
                let leaves = self.leaves_where(|i| i.device == *device)?;
                if leaves.is_empty() {
                    return Err(Error::Policy("no such device in the team".into()));
                }
                self.stage_commit(Vec::new(), leaves, None, None)
            }
            Change::Rotate => self.stage_commit(Vec::new(), Vec::new(), None, None),
            Change::SetRole(member, role) => {
                self.require_admin()?;
                let mut roster = self.roster()?;
                if roster.members.insert(*member, *role).is_none() {
                    return Err(Error::Policy("no such member".into()));
                }
                self.stage_commit(Vec::new(), Vec::new(), Some(roster), None)
            }
            Change::SetEscrow(escrow) => {
                self.require_admin()?;
                let mut roster = self.roster()?;
                roster.escrow = escrow.clone();
                self.stage_commit(Vec::new(), Vec::new(), Some(roster), None)
            }
            Change::ReplaceRecovery(recovery) => {
                self.require_admin()?;
                let mut roster = self.roster()?;
                roster.recovery = recovery.clone();
                let own = self.identity;
                let leaves = self.leaves_where(|i| i.kind == DeviceKind::Recovery && *i != own)?;
                self.stage_commit(Vec::new(), leaves, Some(roster), None)
            }
            Change::Admit(request, now) => self.stage_admit(request, *now),
        }
    }

    /// Whether a change has nothing left to do in the current epoch (another commit did it): the device does not
    /// stage it again after losing its epoch.
    pub fn is_done(&self, change: &Change) -> Result<bool, Error> {
        let roster = self.roster()?;
        Ok(match change {
            Change::RemoveMember(member) => {
                !roster.members.contains_key(member)
                    && self
                        .leaves_where(|i| i.member == *member && i.kind == DeviceKind::Device)?
                        .is_empty()
            }
            Change::RemoveDevice(device) => self.leaves_where(|i| i.device == *device)?.is_empty(),
            Change::Rotate | Change::Admit(..) => false,
            Change::SetRole(member, role) => roster.role(member) == Some(*role),
            Change::SetEscrow(escrow) => roster.escrow == *escrow,
            Change::ReplaceRecovery(recovery) => roster.recovery == *recovery,
        })
    }

    fn stage_commit(
        &mut self,
        adds: Vec<KeyPackage>,
        removals: Vec<LeafNodeIndex>,
        roster: Option<Roster>,
        invitation: Option<([u8; 16], [u8; 32])>,
    ) -> Result<Publication, Error> {
        self.current_record()?;
        let provider = &self.provider;
        let group = self.group.as_mut().ok_or(Error::NotInTeam)?;
        let from_epoch = group.epoch().as_u64();
        let mut builder = group
            .commit_builder()
            .force_self_update(true)
            .propose_adds(adds)
            .propose_removals(removals);
        if let Some(roster) = roster {
            builder = builder
                .propose_group_context_extensions(roster.to_extensions()?)
                .map_err(mls)?;
        }
        let bundle = builder
            .load_psks(provider.storage())
            .map_err(mls)?
            .build(provider.rand(), provider.crypto(), &self.signer, |_| true)
            .map_err(mls)?
            .stage_commit(provider)
            .map_err(mls)?;
        let checked = {
            let staged = group
                .pending_commit()
                .ok_or_else(|| Error::Mls("no pending commit".into()))?;
            check_commit(group, &Sender::Member(group.own_leaf_index()), staged)
        };
        if let Err(e) = checked {
            group
                .clear_pending_commit(provider.storage())
                .map_err(mls)?;
            return Err(e);
        }
        let commit = encode(bundle.commit())?;
        let welcome = match bundle.to_welcome_msg() {
            Some(w) => Some(encode(&w)?),
            None => None,
        };
        self.pending = Some(PendingCommit {
            from_epoch,
            commit: commit.clone(),
            welcome,
            invitation_id: invitation.map(|(id, _)| id),
            invitation_secret: invitation.map(|(_, s)| s),
        });
        Ok(Publication {
            commit: Some((from_epoch, commit)),
            ..Default::default()
        })
    }

    /// The server took the pending commit: merges it, records the new epoch, and says the rest to publish (the
    /// history link, the recovery and escrow wraps, the Welcome of an admission).
    pub fn confirm_pending(&mut self) -> Result<Publication, Error> {
        let pending = self
            .pending
            .clone()
            .ok_or_else(|| Error::Mls("no pending commit".into()))?;
        let previous = self
            .records
            .get(&pending.from_epoch)
            .cloned()
            .ok_or(Error::UnknownEpoch(pending.from_epoch))?;
        let group = self.group.as_mut().ok_or(Error::NotInTeam)?;
        if group.epoch().as_u64() != pending.from_epoch || group.pending_commit().is_none() {
            return Err(Error::Mls(
                "the pending commit is no longer in the group".into(),
            ));
        }
        group.merge_pending_commit(&self.provider).map_err(mls)?;
        self.pending = None;

        let record = self.enter_epoch()?;
        let history = HistoryLink::make(
            &self.group_id()?,
            &record,
            &previous,
            random_nonce(self.provider.rand())?,
        )?;
        let (recovery, escrow) = self.wraps(&record)?;
        let welcome = match (
            pending.welcome.as_ref(),
            pending.invitation_id,
            pending.invitation_secret.as_ref(),
        ) {
            (Some(welcome), Some(id), Some(secret)) => {
                let group = self.group()?;
                let confirmation = welcome_confirmation(
                    secret,
                    group.group_id().as_slice(),
                    group.epoch().as_u64(),
                    group.epoch_authenticator().as_slice(),
                );
                Some((
                    id,
                    WelcomeDelivery {
                        welcome: welcome.clone(),
                        confirmation,
                    },
                ))
            }
            _ => None,
        };
        Ok(Publication {
            commit: None,
            welcome,
            history: Some(history),
            recovery: Some(recovery),
            escrow,
        })
    }

    /// The server refused the pending commit (another holds its epoch): drops it, and gives the invitation it used
    /// back. The device then processes the commit that won, and stages again if the change is still to do.
    pub fn abandon_pending(&mut self) -> Result<(), Error> {
        let Some(pending) = self.pending.take() else {
            return Ok(());
        };
        if let Some(id) = pending.invitation_id {
            if let Some(inv) = self.invitations.get_mut(&id) {
                inv.used = false;
            }
        }
        if let Some(group) = self.group.as_mut() {
            group
                .clear_pending_commit(self.provider.storage())
                .map_err(mls)?;
        }
        Ok(())
    }

    /// Stages, then merges at once: for a server in memory that takes every commit in turn (`relay.rs`).
    fn commit_now(&mut self, change: Change) -> Result<Publication, Error> {
        let staged = self.stage(&change)?;
        let rest = self.confirm_pending()?;
        Ok(staged.join(rest))
    }

    pub(crate) fn current_record(&self) -> Result<&EpochRecord, Error> {
        let epoch = self.epoch()?;
        self.records.get(&epoch).ok_or(Error::UnknownEpoch(epoch))
    }

    pub(crate) fn require_admin(&self) -> Result<(), Error> {
        if self.is_admin()? {
            Ok(())
        } else {
            Err(Error::NotAdmin)
        }
    }

    fn leaves_where(&self, keep: impl Fn(&Identity) -> bool) -> Result<Vec<LeafNodeIndex>, Error> {
        Ok(self
            .group()?
            .members()
            .filter(|m| Identity::from_credential(&m.credential).is_ok_and(|i| keep(&i)))
            .map(|m| m.index)
            .collect())
    }

    /// Removes a member: all their devices, and their entry in the roster, in one commit. A new epoch starts that
    /// the member does not have (C8-A: the history is not encrypted again).
    pub fn remove_member(&mut self, member: MemberId) -> Result<Publication, Error> {
        self.commit_now(Change::RemoveMember(member))
    }

    /// Revokes a device (a stolen one): by its member, or by an administrator.
    pub fn remove_device(&mut self, device: DeviceId) -> Result<Publication, Error> {
        self.commit_now(Change::RemoveDevice(device))
    }

    /// "Change the team's keys": an empty commit with a path, after an incident.
    pub fn rotate(&mut self) -> Result<Publication, Error> {
        self.commit_now(Change::Rotate)
    }

    pub fn set_role(&mut self, member: MemberId, role: Role) -> Result<Publication, Error> {
        self.commit_now(Change::SetRole(member, role))
    }

    /// Sets or clears the organisation's escrow key. Every member sees it.
    pub fn set_escrow(&mut self, escrow: Option<EscrowPublic>) -> Result<Publication, Error> {
        self.commit_now(Change::SetEscrow(escrow))
    }

    /// Puts a new recovery key in place of the old one (after a recovery, or when the words may have leaked), and
    /// removes the old recovery leaf if it is in the group.
    pub fn replace_recovery(&mut self, recovery: RecoveryPublic) -> Result<Publication, Error> {
        self.commit_now(Change::ReplaceRecovery(recovery))
    }

    /// Processes a commit of the server's log: MLS checks it, then the team's rules (`policy.rs`), then it is merged.
    pub fn process_commit(&mut self, bytes: &[u8]) -> Result<Processed, Error> {
        if self.pending.is_some() {
            return Err(Error::Mls(
                "a commit of this device is pending: confirm or abandon it first".into(),
            ));
        }
        let message = decode(bytes)?.try_into_protocol_message().map_err(mls)?;
        let current = self.epoch()?;
        let epoch = message.epoch().as_u64();
        if epoch < current {
            return Err(Error::Stale { epoch, current });
        }
        let provider = &self.provider;
        let group = self.group.as_mut().ok_or(Error::NotInTeam)?;
        let processed = group.process_message(provider, message).map_err(mls)?;
        let sender = processed.sender().clone();
        let staged = match processed.into_content() {
            ProcessedMessageContent::StagedCommitMessage(staged) => *staged,
            _ => {
                return Err(Error::Policy(
                    "only commits travel in the team's log".into(),
                ))
            }
        };
        check_commit(group, &sender, &staged)?;
        let removed = staged.self_removed();
        group.merge_staged_commit(provider, staged).map_err(mls)?;
        if removed {
            self.removed = true;
            return Ok(Processed::Removed);
        }
        Ok(Processed::Epoch(self.enter_epoch()?.epoch))
    }

    /// Processes the server's commits from this device's epoch on.
    pub fn sync(&mut self, commits: &[Vec<u8>]) -> Result<(), Error> {
        let from = self.epoch()? as usize;
        for commit in commits.iter().skip(from) {
            if self.process_commit(commit)? == Processed::Removed {
                break;
            }
        }
        Ok(())
    }

    pub fn is_removed(&self) -> bool {
        self.removed
    }

    // === Invitations ===

    /// Makes an invitation and its link. A new member needs an administrator; another device of one's own, oneself.
    pub fn invite(
        &mut self,
        purpose: Purpose,
        now: u64,
        lifetime: u64,
        server: &str,
    ) -> Result<InviteLink, Error> {
        match &purpose {
            Purpose::NewMember(_) => self.require_admin()?,
            Purpose::OwnDevice(member) if *member == self.identity.member => {}
            Purpose::OwnDevice(_) => self.require_admin()?,
        }
        let rand = self.provider.rand();
        let id: [u8; 16] = rand.random_array().map_err(|_| Error::Random)?;
        let secret: [u8; 32] = rand.random_array().map_err(|_| Error::Random)?;
        let team = self.fingerprint()?.0;
        let member = match &purpose {
            Purpose::OwnDevice(m) => Some(m.0),
            Purpose::NewMember(_) => None,
        };
        self.invitations.insert(
            id,
            Invitation {
                id,
                secret,
                purpose,
                expires_at: now.saturating_add(lifetime),
                used: false,
                app_link: true,
            },
        );
        Ok(InviteLink {
            server: server.to_string(),
            id,
            secret,
            team,
            member,
        })
    }

    /// An invitation of this device, to change in a test (one made before the links became the app's).
    #[cfg(test)]
    pub(crate) fn invitation_mut(&mut self, id: &[u8; 16]) -> Option<&mut Invitation> {
        self.invitations.get_mut(id)
    }

    /// The invitee: makes its KeyPackage and proves the link's secret.
    pub fn request_join(&mut self, link: &InviteLink) -> Result<JoinRequest, Error> {
        let bundle = KeyPackage::builder()
            .leaf_node_capabilities(capabilities())
            .build(
                CIPHERSUITE,
                &self.provider,
                &self.signer,
                self.credential_with_key(),
            )
            .map_err(mls)?;
        let key_package = encode(&MlsMessageOut::from(bundle.key_package().clone()))?;
        self.joining = Some(link.clone());
        Ok(JoinRequest {
            invitation: link.id,
            mac: join_mac(&link.secret, &link.id, &key_package),
            key_package,
        })
    }

    /// An administrator (or the member, for their own device) admits a request: checks the invitation and the MAC,
    /// adds the device (and the member to the roster), and makes the Welcome and its confirmation.
    pub fn admit(&mut self, request: &JoinRequest, now: u64) -> Result<Publication, Error> {
        self.commit_now(Change::Admit(request.clone(), now))
    }

    fn stage_admit(&mut self, request: &JoinRequest, now: u64) -> Result<Publication, Error> {
        let invitation = self
            .invitations
            .get(&request.invitation)
            .ok_or(InvitationError::Unknown)?
            .clone();
        if !invitation.app_link {
            return Err(InvitationError::OldForm.into());
        }
        if invitation.used {
            return Err(InvitationError::AlreadyUsed.into());
        }
        if now > invitation.expires_at {
            return Err(InvitationError::Expired.into());
        }
        verify_join_mac(
            &invitation.secret,
            &invitation.id,
            &request.key_package,
            &request.mac,
        )?;
        let key_package = match decode(&request.key_package)?.extract() {
            MlsMessageBodyIn::KeyPackage(kp) => kp
                .validate(self.provider.crypto(), ProtocolVersion::Mls10)
                .map_err(mls)?,
            _ => return Err(Error::Encoding("a join request holds a KeyPackage".into())),
        };
        let joiner = Identity::from_credential(key_package.leaf_node().credential())?;
        let mut roster = self.roster()?;
        let new_roster = match &invitation.purpose {
            Purpose::NewMember(role) => {
                if roster.members.contains_key(&joiner.member) {
                    return Err(Error::Policy("a new member brings a new member id".into()));
                }
                roster.members.insert(joiner.member, *role);
                Some(roster)
            }
            Purpose::OwnDevice(member) => {
                if joiner.member != *member {
                    return Err(Error::Policy(
                        "the device is not of the member it was invited for".into(),
                    ));
                }
                None
            }
        };
        // Used from now on, even if what follows fails: an invitation never serves twice. A commit the server
        // refuses gives it back (`abandon_pending`).
        if let Some(inv) = self.invitations.get_mut(&request.invitation) {
            inv.used = true;
        }
        let publication = self.stage_commit(
            vec![key_package],
            Vec::new(),
            new_roster,
            Some((invitation.id, invitation.secret)),
        )?;
        if self.pending.as_ref().is_none_or(|p| p.welcome.is_none()) {
            return Err(Error::Mls("an Add without a Welcome".into()));
        }
        Ok(publication)
    }

    /// The invitee joins by the Welcome: the group must be the team of its link, and the confirmation must prove the
    /// link's secret. Its history is then read by [`Self::read_history`].
    pub fn accept_welcome(&mut self, delivery: &WelcomeDelivery) -> Result<(), Error> {
        let link = self.joining.clone().ok_or(InvitationError::Unknown)?;
        let welcome = match decode(&delivery.welcome)?.extract() {
            MlsMessageBodyIn::Welcome(welcome) => welcome,
            _ => return Err(Error::Encoding("not a Welcome".into())),
        };
        let staged = StagedWelcome::new_from_welcome(&self.provider, &join_config(), welcome, None)
            .map_err(mls)?;
        let roster = Roster::from_context(staged.group_context())?;
        let group_id = staged.group_context().group_id().as_slice().to_vec();
        if TeamFingerprint::of(&group_id, &roster.genesis) != link.team() {
            return Err(InvitationError::WrongTeam.into());
        }
        let group = staged.into_group(&self.provider).map_err(mls)?;
        verify_welcome_confirmation(
            &link.secret,
            &group_id,
            group.epoch().as_u64(),
            group.epoch_authenticator().as_slice(),
            &delivery.confirmation,
        )?;
        self.group = Some(group);
        self.joining = None;
        self.enter_epoch()?;
        Ok(())
    }

    /// Reads the history back to epoch 0, through the server's links: the records of the epochs before this
    /// device's first one. Returns how many it learnt.
    pub fn read_history(
        &mut self,
        fetch: impl FnMut(u64) -> Option<HistoryLink>,
    ) -> Result<usize, Error> {
        let group_id = self.group_id()?;
        let oldest = self
            .records
            .values()
            .next()
            .cloned()
            .ok_or(Error::NotInTeam)?;
        let found = walk_back(&group_id, &oldest, 0, fetch)?;
        let n = found.len();
        for record in found {
            self.records.insert(record.epoch, record);
        }
        Ok(n)
    }

    /// Learns the record of the epoch before `link.epoch` from one history link, when this device holds
    /// `link.epoch`. True when it is new; false when it was known, with the same head; [`Error::Fork`] when it was
    /// known with another head (two members of one epoch saw two pasts). A link this device cannot open (the
    /// server's, or of a branch it is not on) is an error, and the caller tries another.
    pub fn learn_history(&mut self, link: &HistoryLink) -> Result<bool, Error> {
        let group_id = self.group_id()?;
        let current = self
            .records
            .get(&link.epoch)
            .ok_or(Error::UnknownEpoch(link.epoch))?;
        let previous = link.open(&group_id, current)?;
        match self.records.get(&previous.epoch) {
            Some(known) if known.head == previous.head => Ok(false),
            Some(_) => Err(Error::Fork {
                epoch: previous.epoch,
            }),
            None => {
                self.records.insert(previous.epoch, previous);
                Ok(true)
            }
        }
    }

    /// The history link of `epoch`, made by any member who holds that epoch and the one before: the one its
    /// committer did not write (a device that stopped before sending it) is written by another.
    pub fn history_link(&self, epoch: u64) -> Result<HistoryLink, Error> {
        if epoch == 0 {
            return Err(Error::UnknownEpoch(0));
        }
        let current = self.records.get(&epoch).ok_or(Error::UnknownEpoch(epoch))?;
        let previous = self
            .records
            .get(&(epoch - 1))
            .ok_or(Error::MissingHistory(epoch))?;
        HistoryLink::make(
            &self.group_id()?,
            current,
            previous,
            random_nonce(self.provider.rand())?,
        )
    }

    // === Content ===

    /// Encrypts and signs an object of a project, under the current epoch.
    pub fn seal(
        &self,
        project: ProjectId,
        object: ObjectId,
        version: u64,
        plaintext: &[u8],
    ) -> Result<SealedObject, Error> {
        let record = self.current_record()?;
        let header = ObjectHeader {
            project,
            object,
            version,
            epoch: record.epoch,
            head: record.head,
            author: self.signer.to_public_vec(),
        };
        SealedObject::seal(
            record,
            header,
            random_nonce(self.provider.rand())?,
            plaintext,
            |bytes| self.signer.sign(bytes).map_err(mls),
        )
    }

    /// Checks the object's author and signature, then the head it cites (a fork shows here), then decrypts. The
    /// author first: an object the server forged (no leaf's signature) is refused as such, never taken for a fork.
    pub fn open(&self, sealed: &SealedObject) -> Result<Vec<u8>, Error> {
        if let Some(record) = self.records.get(&sealed.header.epoch) {
            if !record.has_author(&sealed.header.author) {
                return Err(Error::BadSignature);
            }
            crate::content::verify_ed25519(
                &sealed.header.author,
                &SealedObject::signed_bytes(&sealed.header, &sealed.nonce, &sealed.ciphertext),
                &sealed.signature,
            )?;
        }
        match self.check_head(sealed.header.epoch, &sealed.header.head)? {
            HeadCheck::Same => {}
            HeadCheck::Behind => return Err(Error::Behind(sealed.header.epoch)),
            HeadCheck::Unknown => return Err(Error::UnknownEpoch(sealed.header.epoch)),
        }
        sealed.open(&self.records[&sealed.header.epoch])
    }

    // === Forks and rollbacks ===

    /// What this device makes of a head cited for an epoch: by an object, a member's beacon, the server.
    pub fn check_head(&self, epoch: u64, cited: &[u8; 32]) -> Result<HeadCheck, Error> {
        if let Some(record) = self.records.get(&epoch) {
            return if record.head == *cited {
                Ok(HeadCheck::Same)
            } else {
                Err(Error::Fork { epoch })
            };
        }
        if self.checkpoint.is_some_and(|c| epoch > c.epoch) {
            Ok(HeadCheck::Behind)
        } else {
            Ok(HeadCheck::Unknown)
        }
    }

    /// The state the server says the team is at: older than what this device has seen is a rollback, refused.
    pub fn check_server_state(
        &self,
        epoch: u64,
        cited: Option<&[u8; 32]>,
    ) -> Result<HeadCheck, Error> {
        if let Some(seen) = self.checkpoint {
            if epoch < seen.epoch {
                return Err(Error::Rollback {
                    seen: seen.epoch,
                    shown: epoch,
                });
            }
        }
        match cited {
            Some(h) => self.check_head(epoch, h),
            None => Ok(HeadCheck::Same),
        }
    }

    /// When a device starts again from its saved state: a state older than the checkpoint kept in the keychain (an
    /// old backup put back, a disk image) is refused.
    pub fn check_restored(&self, saved: &Checkpoint) -> Result<(), Error> {
        let epoch = self.epoch()?;
        if epoch < saved.epoch {
            return Err(Error::Rollback {
                seen: saved.epoch,
                shown: epoch,
            });
        }
        if epoch == saved.epoch && self.checkpoint.is_some_and(|c| c.head != saved.head) {
            return Err(Error::Fork { epoch });
        }
        Ok(())
    }

    /// A head beacon of the current epoch, signed by this device's leaf (`beacon.rs`).
    pub fn beacon(&self, at: u64) -> Result<Beacon, Error> {
        let record = self.current_record()?;
        Beacon::sign(
            &self.group_id()?,
            record.epoch,
            record.head,
            at,
            self.signer.to_public_vec(),
            |bytes| self.signer.sign(bytes).map_err(mls),
        )
    }

    /// What this device makes of a member's beacon: its signature by a leaf of this device's current epoch (a
    /// member still in the team: one removed, who keeps the keys of old epochs, cannot raise a false alarm), also a
    /// leaf of the beacon's epoch when this device holds it (a leaf keeps its key), then its head, as
    /// [`Self::check_head`]. A beacon signed by nobody known is [`Error::BadSignature`], and changes nothing.
    pub fn check_beacon(&self, beacon: &Beacon) -> Result<HeadCheck, Error> {
        let now = self.current_record()?;
        if !now.has_author(&beacon.author)
            || self
                .records
                .get(&beacon.epoch)
                .is_some_and(|r| !r.has_author(&beacon.author))
        {
            return Err(Error::BadSignature);
        }
        beacon.verify(&self.group_id()?)?;
        self.check_head(beacon.epoch, &beacon.head)
    }

    /// The six words of the current epoch's head, to compare out of band ("we see the same team"): the server
    /// that partitions the team for good shows two sets of words.
    pub fn head_words(&self) -> Result<[&'static str; 6], Error> {
        Ok(TeamFingerprint(self.current_record()?.head).words())
    }

    // === Recovery ===

    /// The recovery key comes back: its words open the last recovery wrap, and it joins the group by an external
    /// commit, as the recovery leaf, an administrator. It then invites the new devices.
    pub fn recover_with(
        provider: P,
        words: &str,
        wrap: &RecoveryWrap,
        group_id: &[u8],
    ) -> Result<(Self, Publication), Error> {
        let key = RecoveryKey::from_words(words)?;
        let (previous, group_info) = wrap.open(provider.crypto(), &key, group_id)?;
        let group_info = match decode(&group_info)?.extract() {
            MlsMessageBodyIn::GroupInfo(gi) => gi,
            _ => return Err(Error::Encoding("a recovery wrap holds a GroupInfo".into())),
        };
        let mut device = Self::with_identity(provider, key.signature_keys(), key.identity());
        let (group, bundle) = MlsGroup::external_commit_builder()
            .with_config(join_config())
            .build_group(&device.provider, group_info, device.credential_with_key())
            .map_err(mls)?
            .leaf_node_parameters(
                LeafNodeParameters::builder()
                    .with_capabilities(capabilities())
                    .build(),
            )
            .load_psks(device.provider.storage())
            .map_err(mls)?
            .build(
                device.provider.rand(),
                device.provider.crypto(),
                &device.signer,
                |_| true,
            )
            .map_err(mls)?
            .finalize(&device.provider)
            .map_err(mls)?;
        let commit = encode(bundle.commit())?;
        device.group = Some(group);
        device
            .group
            .as_mut()
            .expect("set above")
            .merge_pending_commit(&device.provider)
            .map_err(mls)?;
        let from_epoch = previous.epoch;
        device.records.insert(previous.epoch, previous.clone());
        let record = device.enter_epoch()?;
        let history = HistoryLink::make(
            group_id,
            &record,
            &previous,
            random_nonce(device.provider.rand())?,
        )?;
        let (recovery, escrow) = device.wraps(&record)?;
        Ok((
            device,
            Publication {
                commit: Some((from_epoch, commit)),
                welcome: None,
                history: Some(history),
                recovery: Some(recovery),
                escrow,
            },
        ))
    }
}

/// A test door: the raw MLS group, to play a device that breaks the rules (`tests.rs`, and the client's tests with
/// the feature `test-door`).
#[cfg(any(test, feature = "test-door"))]
impl<P: OpenMlsProvider> TeamDevice<P> {
    pub fn mls(&mut self) -> (&P, &SignatureKeyPair, &mut MlsGroup) {
        (
            &self.provider,
            &self.signer,
            self.group.as_mut().expect("in a team"),
        )
    }
}
