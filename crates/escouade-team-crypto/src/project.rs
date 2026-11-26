//! The MLS group of a project kept to some members of the team (TS-5, `docs/TEAM-SERVER.md` §2.4 and §3.6).
//!
//! A project of the whole team takes its key from the team's group (`keys.rs`: the project's key of each team
//! epoch). A project the team keeps to some members has **its own MLS group**: its leaves are the devices of those
//! members only, and its operations are sealed under the keys of that group's epochs. A member of the team outside
//! it never holds them.
//!
//! - **Its roster** ([`ProjectRoster`]) is a private extension of the group context (0xff01): the project's id, the
//!   team's group, and the members who have access. Only an administrator of the team changes it.
//! - **Its leaves** are the team's: the same identity and the same signature key as in the team's group. A device is
//!   added by its **KeyPackage** ([`TeamDevice::key_package`], a last-resort one, which the device publishes in the
//!   team's stream), and only when its member is in the project's roster and its leaf is a leaf of the team. Any
//!   device of the project keeps the group in step with the team ([`TeamDevice::project_stage`]): it adds the new
//!   devices of its members, and removes the devices that left the team (C8-A: the next operations are at the next
//!   epoch, which they never hold; the history is not encrypted again).
//! - **Its commits** travel in the project's stream, the first one of an epoch winning on the server, as the team's.
//!   Each carries, in its authenticated data, the team's epoch its committer saw: a device behind it reads the team
//!   first ([`Error::Behind`]), and its [`CommitScope`] (SR2-02): an administrator's, or a member's for their own
//!   devices (and the removal of devices that lost their place). Every device checks every commit by the rules of
//!   [`check_project_commit`].
//! - **Its history**: as the team's (`history.rs`), a link per epoch, so that a device added later reads the
//!   operations sealed before it.

use std::collections::{BTreeMap, BTreeSet};

use openmls::prelude::{
    Capabilities, CredentialType, Extension, ExtensionType, Extensions, GroupContext, GroupId,
    KeyPackage, LeafNodeIndex, MlsGroup, MlsMessageBodyIn, MlsMessageOut, ProcessedMessageContent,
    Proposal, ProtocolVersion, RequiredCapabilitiesExtension, Sender, StagedCommit, StagedWelcome,
    UnknownExtension, MIXED_CIPHERTEXT_WIRE_FORMAT_POLICY,
};
use openmls_traits::signatures::Signer;
use openmls_traits::OpenMlsProvider;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::device::{
    decode, encode, join_config, random_nonce, PendingCommit, Processed, TeamDevice,
};
use crate::history::HistoryLink;
use crate::identity::{DeviceId, DeviceKind, Identity, MemberId, ProjectId};
use crate::keys::{head, EpochMember, EpochRecord, CONTENT_LABEL, HISTORY_LABEL};
use crate::ops::{Scope, Sealed, What};
use crate::policy::CommitScope;
use crate::roster::{Role, Roster, ROSTER_EXTENSION};
use crate::{mls, Error, CIPHERSUITE};

/// The project roster's extension type, in the private-use range, next to the team's.
pub const PROJECT_EXTENSION: u16 = 0xff01;

/// Who has access to a project kept to some members.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRoster {
    pub project: ProjectId,
    /// The team's group id: a project's group belongs to one team.
    #[serde(with = "hex::serde")]
    pub team: Vec<u8>,
    pub members: BTreeSet<MemberId>,
}

impl ProjectRoster {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a roster always serializes")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes).map_err(|e| Error::Encoding(format!("project roster: {e}")))
    }

    fn to_extensions(&self) -> Result<Extensions<GroupContext>, Error> {
        Extensions::from_vec(vec![
            Extension::Unknown(PROJECT_EXTENSION, UnknownExtension(self.to_bytes())),
            Extension::RequiredCapabilities(RequiredCapabilitiesExtension::new(
                &[ExtensionType::Unknown(PROJECT_EXTENSION)],
                &[],
                &[],
            )),
        ])
        .map_err(|e| Error::Mls(format!("project extension: {e:?}")))
    }

    pub fn from_extensions(extensions: &Extensions<GroupContext>) -> Result<Self, Error> {
        let ext = extensions
            .unknown(PROJECT_EXTENSION)
            .ok_or_else(|| Error::Policy("the group context has no project roster".into()))?;
        Self::from_bytes(&ext.0)
    }
}

/// The capabilities of a leaf in a project's group (and of the KeyPackage that brings it): the team's, the
/// project's roster, and the last-resort KeyPackage.
pub fn project_capabilities() -> Capabilities {
    Capabilities::new(
        None,
        Some(&[CIPHERSUITE]),
        Some(&[
            ExtensionType::Unknown(ROSTER_EXTENSION),
            ExtensionType::Unknown(PROJECT_EXTENSION),
            ExtensionType::LastResort,
        ]),
        None,
        Some(&[CredentialType::Basic]),
    )
}

/// The authenticated data of a project's commit: the team's epoch its committer saw, then its scope (SR2-02).
fn project_aad(team_epoch: u64, scope: CommitScope) -> Vec<u8> {
    let mut out = team_epoch.to_be_bytes().to_vec();
    out.push(scope.byte());
    out
}

/// The team's epoch and the scope a project's commit says; a commit that says no scope (made before SR2-02) is an
/// administrator's.
fn of_project_aad(aad: &[u8]) -> Result<(u64, CommitScope), Error> {
    let epoch = aad
        .get(..8)
        .and_then(|b| <[u8; 8]>::try_from(b).ok())
        .map(u64::from_be_bytes)
        .ok_or_else(|| refuse("a project's commit says the team's epoch its committer saw"))?;
    let scope = match &aad[8..] {
        [] | [0] => CommitScope::Admin,
        [1] => CommitScope::Own,
        _ => {
            return Err(refuse(
                "a project's commit whose authenticated data says no scope",
            ))
        }
    };
    Ok((epoch, scope))
}

/// A project's group id: one per project of one team.
pub fn project_group_id(team_group: &[u8], project: &ProjectId) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(b"escouade/project-group/v1\0");
    h.update((team_group.len() as u32).to_be_bytes());
    h.update(team_group);
    h.update(project.0);
    h.finalize().to_vec()
}

/// A project's group on this device.
pub struct ProjectGroup {
    pub(crate) group_id: Vec<u8>,
    pub(crate) group: Option<MlsGroup>,
    pub(crate) records: BTreeMap<u64, EpochRecord>,
    pub(crate) pending: Option<PendingCommit>,
    /// A commit removed this device: it keeps its records (it read the content before), nothing new.
    pub(crate) removed: bool,
}

/// What a device keeps of a project's group beside its MLS state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedProject {
    pub project: ProjectId,
    #[serde(with = "hex::serde")]
    pub group_id: Vec<u8>,
    pub removed: bool,
    pub pending: Option<PendingCommit>,
}

/// What goes out after a change of a project's group.
#[derive(Debug, Default)]
pub struct ProjectPublication {
    /// The commit, and the epoch it applies to, for the project's stream.
    pub commit: Option<(u64, Vec<u8>)>,
    /// The Welcome of the devices added, for the team's stream.
    pub welcome: Option<Vec<u8>>,
    /// The history link of the new epoch, for the team's stream.
    pub history: Option<HistoryLink>,
}

/// A leaf as the rules see it.
#[derive(Clone, PartialEq, Eq)]
struct Leaf {
    identity: Identity,
    signature_key: Vec<u8>,
}

fn leaf_of(member: &openmls::prelude::Member) -> Result<Leaf, Error> {
    Ok(Leaf {
        identity: Identity::from_credential(&member.credential)?,
        signature_key: member.signature_key.clone(),
    })
}

fn refuse(why: impl Into<String>) -> Error {
    Error::Policy(why.into())
}

/// The team as a project's rules see it: its leaves (identity and key) and its roster, at the verifier's epoch.
pub struct TeamView {
    pub epoch: u64,
    pub leaves: Vec<(Identity, Vec<u8>)>,
    pub roster: Roster,
}

impl TeamView {
    fn has_leaf(&self, leaf: &Leaf) -> bool {
        self.leaves
            .iter()
            .any(|(i, k)| *i == leaf.identity && *k == leaf.signature_key)
    }

    fn is_admin(&self, member: &MemberId) -> bool {
        self.roster.role(member) == Some(Role::Admin)
    }
}

/// The rules of a project's commit, checked by every device before it merges it:
/// - its committer is a leaf of the project's group and of the team, and keeps its identity and key;
/// - the roster keeps its project and its team; only an administrator of the team changes who has access;
/// - a device added is a leaf of the team (same identity, same key) whose member has access;
/// - a device is removed when it left the team, or its member lost access, or by its own member, or by an
///   administrator;
/// - nothing else than adds, removals, updates of the committer and a new roster;
/// - a commit that says it is an administrator's ([`CommitScope::Admin`]) comes from one; one that says it is a
///   member's is checked as a member's, whoever sent it, and adds the committer's own devices only (SR2-02).
pub fn check_project_commit(
    group: &MlsGroup,
    sender: &Sender,
    staged: &StagedCommit,
    team: &TeamView,
    scope: CommitScope,
) -> Result<ProjectRoster, Error> {
    let old = ProjectRoster::from_extensions(group.extensions())?;
    let new = ProjectRoster::from_extensions(staged.group_context().extensions())?;
    if new.project != old.project || new.team != old.team {
        return Err(refuse("a project's group keeps its project and its team"));
    }
    let Sender::Member(index) = sender else {
        return Err(refuse("a project's commit comes from a leaf of its group"));
    };
    let committer = leaf_of(
        &group
            .member_at(*index)
            .ok_or_else(|| refuse("no leaf for the committer"))?,
    )?;
    if !team.has_leaf(&committer) {
        return Err(refuse("a committer who is not a leaf of the team"));
    }
    let admin = team.is_admin(&committer.identity.member);
    if scope == CommitScope::Admin && !admin {
        return Err(refuse(
            "a project's commit that says it is an administrator's, from a member",
        ));
    }
    let admin = admin && scope == CommitScope::Admin;
    if new.members != old.members && !admin {
        return Err(refuse(
            "only an administrator of the team changes who has access to a project",
        ));
    }
    if let Some(node) = staged.update_path_leaf_node() {
        let identity = Identity::from_credential(node.credential())?;
        if identity != committer.identity
            || node.signature_key().as_slice() != committer.signature_key
        {
            return Err(refuse("a leaf changed its identity or its signature key"));
        }
    }
    for queued in staged.queued_proposals() {
        if queued.sender() != sender {
            return Err(refuse("a proposal by another than the committer"));
        }
        match queued.proposal() {
            Proposal::Add(add) => {
                let node = add.key_package().leaf_node();
                let leaf = Leaf {
                    identity: Identity::from_credential(node.credential())?,
                    signature_key: node.signature_key().as_slice().to_vec(),
                };
                if leaf.identity.kind != DeviceKind::Device
                    || !new.members.contains(&leaf.identity.member)
                {
                    return Err(refuse("a device added whose member has no access"));
                }
                if !team.has_leaf(&leaf) {
                    return Err(refuse("a device added that is not a leaf of the team"));
                }
                if !admin && leaf.identity.member != committer.identity.member {
                    return Err(refuse("a member adds their own devices only"));
                }
            }
            Proposal::Remove(remove) => {
                let target = leaf_of(
                    &group
                        .member_at(remove.removed())
                        .ok_or_else(|| refuse("no leaf to remove"))?,
                )?;
                let allowed = admin
                    || !team.has_leaf(&target)
                    || !new.members.contains(&target.identity.member)
                    || target.identity.member == committer.identity.member;
                if !allowed {
                    return Err(refuse("a device of the project removed without a reason"));
                }
            }
            Proposal::Update(update) => {
                if Identity::from_credential(update.leaf_node().credential())? != committer.identity
                {
                    return Err(refuse("an update changes no identity"));
                }
            }
            Proposal::GroupContextExtensions(_) => {
                if !admin {
                    return Err(refuse("only an administrator changes a project's roster"));
                }
            }
            _ => return Err(refuse("a proposal a project's group does not use")),
        }
    }
    Ok(new)
}

impl<P: OpenMlsProvider> TeamDevice<P> {
    // === Kept on the disk ===

    pub(crate) fn saved_projects(&self) -> Vec<SavedProject> {
        self.projects
            .iter()
            .map(|(project, g)| SavedProject {
                project: *project,
                group_id: g.group_id.clone(),
                removed: g.removed,
                pending: g.pending.clone(),
            })
            .collect()
    }

    pub(crate) fn load_projects(&mut self, saved: &[SavedProject]) -> Result<(), Error> {
        for s in saved {
            let group = if s.removed {
                None
            } else {
                MlsGroup::load(self.provider.storage(), &GroupId::from_slice(&s.group_id))
                    .map_err(mls)?
            };
            self.projects.insert(
                s.project,
                ProjectGroup {
                    group_id: s.group_id.clone(),
                    group,
                    records: BTreeMap::new(),
                    pending: s.pending.clone(),
                    removed: s.removed,
                },
            );
        }
        Ok(())
    }

    /// Gives a project's records back, after [`TeamDevice::restore`].
    pub fn restore_project_records(
        &mut self,
        project: ProjectId,
        records: Vec<EpochRecord>,
    ) -> Result<(), Error> {
        let g = self
            .projects
            .get_mut(&project)
            .ok_or_else(|| Error::Mls("records of an unknown project".into()))?;
        for r in records {
            g.records.insert(r.epoch, r);
        }
        Ok(())
    }

    /// Every project's records, to keep on the disk.
    pub fn project_records(&self) -> impl Iterator<Item = (ProjectId, &EpochRecord)> {
        self.projects
            .iter()
            .flat_map(|(p, g)| g.records.values().map(move |r| (*p, r)))
    }

    // === What a device knows of its projects ===

    /// The projects whose group this device was in.
    pub fn project_ids(&self) -> Vec<ProjectId> {
        self.projects.keys().copied().collect()
    }

    /// In the project's group now (not removed from it).
    pub fn in_project(&self, project: &ProjectId) -> bool {
        self.projects
            .get(project)
            .is_some_and(|g| !g.removed && g.group.is_some())
    }

    fn project_group(&self, project: &ProjectId) -> Result<&MlsGroup, Error> {
        self.projects
            .get(project)
            .filter(|g| !g.removed)
            .and_then(|g| g.group.as_ref())
            .ok_or(Error::NotInProject)
    }

    pub fn project_epoch(&self, project: &ProjectId) -> Result<u64, Error> {
        Ok(self.project_group(project)?.epoch().as_u64())
    }

    pub fn project_roster(&self, project: &ProjectId) -> Result<ProjectRoster, Error> {
        ProjectRoster::from_extensions(self.project_group(project)?.extensions())
    }

    /// The leaves of a project's group.
    pub fn project_leaves(&self, project: &ProjectId) -> Result<Vec<Identity>, Error> {
        self.project_group(project)?
            .members()
            .map(|m| Identity::from_credential(&m.credential))
            .collect()
    }

    pub fn project_record(&self, project: &ProjectId, epoch: u64) -> Option<&EpochRecord> {
        self.projects.get(project)?.records.get(&epoch)
    }

    /// The epochs of a project whose record this device holds.
    pub fn project_epochs(&self, project: &ProjectId) -> Vec<u64> {
        self.projects
            .get(project)
            .map(|g| g.records.keys().copied().collect())
            .unwrap_or_default()
    }

    pub fn project_pending(&self, project: &ProjectId) -> Option<&PendingCommit> {
        self.projects.get(project)?.pending.as_ref()
    }

    /// The team as the project's rules see it now.
    pub fn team_view(&self) -> Result<TeamView, Error> {
        Ok(TeamView {
            epoch: self.epoch()?,
            leaves: self
                .group()?
                .members()
                .map(|m| Ok((Identity::from_credential(&m.credential)?, m.signature_key)))
                .collect::<Result<_, Error>>()?,
            roster: self.roster()?,
        })
    }

    // === KeyPackages ===

    /// A last-resort KeyPackage of this device, for the projects' groups: published in the team's stream, used for
    /// every project it is added to. Its private keys stay in this device's storage.
    pub fn key_package(&self) -> Result<Vec<u8>, Error> {
        let bundle = KeyPackage::builder()
            .leaf_node_capabilities(project_capabilities())
            .mark_as_last_resort()
            .build(
                CIPHERSUITE,
                &self.provider,
                &self.signer,
                self.credential_with_key(),
            )
            .map_err(mls)?;
        encode(&MlsMessageOut::from(bundle.key_package().clone()))
    }

    /// A device's KeyPackage of the team's stream, checked: its signature, a leaf of the team now with the same
    /// identity and key (the server cannot slip one in). Gives the device it is for.
    pub fn check_key_package(&self, bytes: &[u8]) -> Result<DeviceId, Error> {
        let kp = self.validate_key_package(bytes)?;
        let leaf = Leaf {
            identity: Identity::from_credential(kp.leaf_node().credential())?,
            signature_key: kp.leaf_node().signature_key().as_slice().to_vec(),
        };
        if leaf.identity.kind != DeviceKind::Device || !self.team_view()?.has_leaf(&leaf) {
            return Err(Error::BadSignature);
        }
        Ok(leaf.identity.device)
    }

    /// The device a KeyPackage is for (its identity), its signature checked (not yet against the team's leaves: the device may be
    /// one this device does not know yet; [`Self::project_stage`] checks it when it uses it).
    pub fn key_package_identity(&self, bytes: &[u8]) -> Result<Identity, Error> {
        let kp = self.validate_key_package(bytes)?;
        let identity = Identity::from_credential(kp.leaf_node().credential())?;
        if identity.kind != DeviceKind::Device {
            return Err(Error::BadSignature);
        }
        Ok(identity)
    }

    fn validate_key_package(&self, bytes: &[u8]) -> Result<KeyPackage, Error> {
        match decode(bytes)?.extract() {
            MlsMessageBodyIn::KeyPackage(kp) => kp
                .validate(self.provider.crypto(), ProtocolVersion::Mls10)
                .map_err(mls),
            _ => Err(Error::Encoding("not a KeyPackage".into())),
        }
    }

    // === Making and changing a project's group ===

    /// Makes a project kept to `members` (an administrator of the team): its group, this device its first leaf,
    /// and a commit that adds the devices of the members whose KeyPackage is known. The commit (if any) waits for
    /// the server: [`Self::project_confirm`].
    pub fn create_project(
        &mut self,
        project: ProjectId,
        members: BTreeSet<MemberId>,
        key_packages: &BTreeMap<DeviceId, Vec<u8>>,
    ) -> Result<ProjectPublication, Error> {
        self.require_admin()?;
        if self.projects.contains_key(&project) {
            return Err(Error::Policy("this project has a group already".into()));
        }
        if !members.contains(&self.identity.member) {
            return Err(Error::Policy(
                "the administrator who makes a project has access to it".into(),
            ));
        }
        let roster = self.roster()?;
        if members.iter().any(|m| roster.role(m).is_none()) {
            return Err(Error::Policy("a member the team does not have".into()));
        }
        let team_group = self.group_id()?;
        let project_roster = ProjectRoster {
            project,
            team: team_group.clone(),
            members,
        };
        let group_id = project_group_id(&team_group, &project);
        let group = MlsGroup::builder()
            .with_group_id(GroupId::from_slice(&group_id))
            .ciphersuite(CIPHERSUITE)
            .with_wire_format_policy(MIXED_CIPHERTEXT_WIRE_FORMAT_POLICY)
            .use_ratchet_tree_extension(true)
            .max_past_epochs(0)
            .with_group_context_extensions(project_roster.to_extensions()?)
            .with_capabilities(project_capabilities())
            .build(&self.provider, &self.signer, self.credential_with_key())
            .map_err(mls)?;
        self.projects.insert(
            project,
            ProjectGroup {
                group_id,
                group: Some(group),
                records: BTreeMap::new(),
                pending: None,
                removed: false,
            },
        );
        self.enter_project_epoch(&project)?;
        Ok(self
            .project_stage(&project, None, key_packages)?
            .unwrap_or_default())
    }

    /// Stages the commit that keeps a project's group in step: the devices of its members that are leaves of the
    /// team (and whose KeyPackage is known) added, the devices that left the team or whose member lost access
    /// removed; and, from an administrator, a new list of members. None when there is nothing to do.
    pub fn project_stage(
        &mut self,
        project: &ProjectId,
        members: Option<BTreeSet<MemberId>>,
        key_packages: &BTreeMap<DeviceId, Vec<u8>>,
    ) -> Result<Option<ProjectPublication>, Error> {
        self.stage_project(project, members, key_packages, false)
    }

    /// As [`Self::project_stage`], and a commit even when there is nothing to do (a new path): the administrator's,
    /// after a void (SR2-02), so that the epoch kept to the administrators is taken again.
    pub fn project_rotate(
        &mut self,
        project: &ProjectId,
        key_packages: &BTreeMap<DeviceId, Vec<u8>>,
    ) -> Result<ProjectPublication, Error> {
        Ok(self
            .stage_project(project, None, key_packages, true)?
            .unwrap_or_default())
    }

    fn stage_project(
        &mut self,
        project: &ProjectId,
        members: Option<BTreeSet<MemberId>>,
        key_packages: &BTreeMap<DeviceId, Vec<u8>>,
        force: bool,
    ) -> Result<Option<ProjectPublication>, Error> {
        if self.project_pending(project).is_some() {
            return Err(Error::Mls(
                "a commit of this project is pending: confirm or abandon it first".into(),
            ));
        }
        let team = self.team_view()?;
        let mut roster = self.project_roster(project)?;
        let new_roster = match members {
            Some(m) if m != roster.members => {
                if !team.is_admin(&self.identity.member) {
                    return Err(Error::NotAdmin);
                }
                if !m.contains(&self.identity.member) {
                    return Err(Error::Policy(
                        "an administrator keeps their own access to change a project's".into(),
                    ));
                }
                roster.members = m;
                Some(roster.clone())
            }
            _ => None,
        };
        let own = self.identity;
        let group = self.project_group(project)?;
        let mut removals: Vec<LeafNodeIndex> = Vec::new();
        let mut present: BTreeSet<DeviceId> = BTreeSet::new();
        for m in group.members() {
            let leaf = leaf_of(&m)?;
            present.insert(leaf.identity.device);
            if leaf.identity == own {
                continue;
            }
            if !team.has_leaf(&leaf) || !roster.members.contains(&leaf.identity.member) {
                removals.push(m.index);
            }
        }
        // A member's commit adds their own devices only (SR2-02): an administrator's adds the others'.
        let admin = team.is_admin(&own.member);
        let scope = if admin {
            CommitScope::Admin
        } else {
            CommitScope::Own
        };
        let mut adds: Vec<KeyPackage> = Vec::new();
        for (identity, _key) in &team.leaves {
            if identity.kind != DeviceKind::Device
                || !roster.members.contains(&identity.member)
                || present.contains(&identity.device)
                || (!admin && identity.member != own.member)
            {
                continue;
            }
            let Some(bytes) = key_packages.get(&identity.device) else {
                continue;
            };
            match self.validate_key_package(bytes) {
                Ok(kp)
                    if Identity::from_credential(kp.leaf_node().credential())
                        .is_ok_and(|i| i == *identity) =>
                {
                    adds.push(kp)
                }
                _ => {}
            }
        }
        if adds.is_empty() && removals.is_empty() && new_roster.is_none() && !force {
            return Ok(None);
        }
        let provider = &self.provider;
        let signer = &self.signer;
        let g = self.projects.get_mut(project).expect("checked above");
        let group = g.group.as_mut().expect("checked above");
        let from_epoch = group.epoch().as_u64();
        group.set_aad(project_aad(team.epoch, scope));
        let mut builder = group
            .commit_builder()
            .force_self_update(true)
            .propose_adds(adds)
            .propose_removals(removals);
        if let Some(r) = &new_roster {
            builder = builder
                .propose_group_context_extensions(r.to_extensions()?)
                .map_err(mls)?;
        }
        let bundle = builder
            .load_psks(provider.storage())
            .map_err(mls)?
            .build(provider.rand(), provider.crypto(), signer, |_| true)
            .map_err(mls)?
            .stage_commit(provider)
            .map_err(mls)?;
        let checked = {
            let staged = group
                .pending_commit()
                .ok_or_else(|| Error::Mls("no pending commit".into()))?;
            check_project_commit(
                group,
                &Sender::Member(group.own_leaf_index()),
                staged,
                &team,
                scope,
            )
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
        g.pending = Some(PendingCommit {
            from_epoch,
            commit: commit.clone(),
            welcome,
            invitation_id: None,
            invitation_secret: None,
        });
        Ok(Some(ProjectPublication {
            commit: Some((from_epoch, commit)),
            ..Default::default()
        }))
    }

    /// The server took the project's pending commit: merged, the new epoch recorded; the Welcome of the devices it
    /// added and the epoch's history link to publish.
    pub fn project_confirm(&mut self, project: &ProjectId) -> Result<ProjectPublication, Error> {
        let provider = &self.provider;
        let g = self.projects.get_mut(project).ok_or(Error::NotInProject)?;
        let pending = g
            .pending
            .take()
            .ok_or_else(|| Error::Mls("no pending commit".into()))?;
        let group = g.group.as_mut().ok_or(Error::NotInProject)?;
        if group.epoch().as_u64() != pending.from_epoch || group.pending_commit().is_none() {
            return Err(Error::Mls(
                "the pending commit is no longer in the group".into(),
            ));
        }
        group.merge_pending_commit(provider).map_err(mls)?;
        let previous = g
            .records
            .get(&pending.from_epoch)
            .cloned()
            .ok_or(Error::UnknownEpoch(pending.from_epoch))?;
        let record = self.enter_project_epoch(project)?;
        let history = HistoryLink::make(
            &self.projects[project].group_id,
            &record,
            &previous,
            random_nonce(self.provider.rand())?,
        )?;
        Ok(ProjectPublication {
            commit: None,
            welcome: pending.welcome.clone(),
            history: Some(history),
        })
    }

    /// The server refused the project's pending commit (another holds its epoch): dropped.
    pub fn project_abandon(&mut self, project: &ProjectId) -> Result<(), Error> {
        let storage = self.provider.storage();
        let Some(g) = self.projects.get_mut(project) else {
            return Ok(());
        };
        if g.pending.take().is_some() {
            if let Some(group) = g.group.as_mut() {
                group.clear_pending_commit(storage).map_err(mls)?;
            }
        }
        Ok(())
    }

    /// Processes a commit of the project's stream: MLS checks it, then the project's rules against the team as this
    /// device sees it (a commit made against a later team epoch: [`Error::Behind`], read the team first).
    pub fn project_process_commit(
        &mut self,
        project: &ProjectId,
        bytes: &[u8],
    ) -> Result<Processed, Error> {
        if self.project_pending(project).is_some() {
            return Err(Error::Mls(
                "a commit of this project is pending: confirm or abandon it first".into(),
            ));
        }
        let team = self.team_view()?;
        let message = decode(bytes)?.try_into_protocol_message().map_err(mls)?;
        let current = self.project_epoch(project)?;
        let epoch = message.epoch().as_u64();
        if epoch < current {
            return Err(Error::Stale { epoch, current });
        }
        let provider = &self.provider;
        let g = self.projects.get_mut(project).expect("in the project");
        let group = g.group.as_mut().expect("in the project");
        let processed = group.process_message(provider, message).map_err(mls)?;
        let (cited, scope) = of_project_aad(processed.aad())?;
        if cited > team.epoch {
            return Err(Error::Behind(cited));
        }
        let sender = processed.sender().clone();
        let staged = match processed.into_content() {
            ProcessedMessageContent::StagedCommitMessage(staged) => *staged,
            _ => return Err(refuse("only commits travel in a project's stream")),
        };
        check_project_commit(group, &sender, &staged, &team, scope)?;
        let removed = staged.self_removed();
        group.merge_staged_commit(provider, staged).map_err(mls)?;
        if removed {
            g.removed = true;
            g.group = None;
            return Ok(Processed::Removed);
        }
        Ok(Processed::Epoch(self.enter_project_epoch(project)?.epoch))
    }

    /// Joins a project's group by a Welcome of the team's stream. False when it is not for this device (or this
    /// device is in the group already). The group must be of this team and of that project, and every leaf a device
    /// of a member of the team who has access.
    pub fn project_accept_welcome(
        &mut self,
        project: &ProjectId,
        welcome: &[u8],
    ) -> Result<bool, Error> {
        if self.in_project(project) {
            return Ok(false);
        }
        let team_group = self.group_id()?;
        let roster = self.roster()?;
        let welcome = match decode(welcome)?.extract() {
            MlsMessageBodyIn::Welcome(w) => w,
            _ => return Err(Error::Encoding("not a Welcome".into())),
        };
        let Ok(staged) =
            StagedWelcome::new_from_welcome(&self.provider, &join_config(), welcome, None)
        else {
            // Not for one of this device's KeyPackages.
            return Ok(false);
        };
        let project_roster = ProjectRoster::from_extensions(staged.group_context().extensions())?;
        let group_id = staged.group_context().group_id().as_slice().to_vec();
        if project_roster.team != team_group
            || project_roster.project != *project
            || group_id != project_group_id(&team_group, project)
        {
            return Err(refuse("a Welcome of another team or project"));
        }
        for m in staged.members() {
            let identity = Identity::from_credential(&m.credential)?;
            if identity.kind != DeviceKind::Device
                || !project_roster.members.contains(&identity.member)
                || roster.role(&identity.member).is_none()
            {
                return Err(refuse("a project's leaf of no member who has access"));
            }
        }
        let group = staged.into_group(&self.provider).map_err(mls)?;
        let records = self
            .projects
            .remove(project)
            .map(|g| g.records)
            .unwrap_or_default();
        self.projects.insert(
            *project,
            ProjectGroup {
                group_id,
                group: Some(group),
                records,
                pending: None,
                removed: false,
            },
        );
        self.enter_project_epoch(project)?;
        Ok(true)
    }

    fn enter_project_epoch(&mut self, project: &ProjectId) -> Result<EpochRecord, Error> {
        let crypto = self.provider.crypto();
        let g = self.projects.get_mut(project).ok_or(Error::NotInProject)?;
        let group = g.group.as_ref().ok_or(Error::NotInProject)?;
        let epoch = group.epoch().as_u64();
        let group_id = g.group_id.clone();
        let export = |label: &str| -> Result<[u8; 32], Error> {
            group
                .export_secret(crypto, label, &group_id, 32)
                .map_err(mls)?
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
                    identity: Identity::from_credential(&m.credential)
                        .map(|i| i.to_bytes())
                        .unwrap_or_default(),
                    signature_key: m.signature_key,
                })
                .collect(),
        };
        g.records.insert(epoch, record.clone());
        Ok(record)
    }

    // === A project's history ===

    /// The history link of a project's epoch, by any device that holds it and the one before.
    pub fn project_history_link(
        &self,
        project: &ProjectId,
        epoch: u64,
    ) -> Result<HistoryLink, Error> {
        let g = self.projects.get(project).ok_or(Error::NotInProject)?;
        let current = g.records.get(&epoch).ok_or(Error::UnknownEpoch(epoch))?;
        let previous = g
            .records
            .get(&epoch.wrapping_sub(1))
            .filter(|_| epoch > 0)
            .ok_or(Error::MissingHistory(epoch))?;
        HistoryLink::make(
            &g.group_id,
            current,
            previous,
            random_nonce(self.provider.rand())?,
        )
    }

    /// Learns a project's record of the epoch before `link.epoch`. As [`TeamDevice::learn_history`].
    pub fn project_learn_history(
        &mut self,
        project: &ProjectId,
        link: &HistoryLink,
    ) -> Result<bool, Error> {
        let g = self.projects.get_mut(project).ok_or(Error::NotInProject)?;
        let current = g
            .records
            .get(&link.epoch)
            .ok_or(Error::UnknownEpoch(link.epoch))?;
        let previous = link.open(&g.group_id, current)?;
        match g.records.get(&previous.epoch) {
            Some(known) if known.head == previous.head => Ok(false),
            Some(_) => Err(Error::Fork {
                epoch: previous.epoch,
            }),
            None => {
                g.records.insert(previous.epoch, previous);
                Ok(true)
            }
        }
    }

    // === Sealing and opening a project's operations ===

    /// The record a project's operation of `scope` and `epoch` opens with.
    pub fn record_for(
        &self,
        project: &ProjectId,
        scope: Scope,
        epoch: u64,
    ) -> Result<&EpochRecord, Error> {
        match scope {
            Scope::Team => match self.records.get(&epoch) {
                Some(r) => Ok(r),
                None if self.checkpoint.is_some_and(|c| epoch > c.epoch) => {
                    Err(Error::Behind(epoch))
                }
                None => Err(Error::UnknownEpoch(epoch)),
            },
            Scope::Project => {
                let g = self.projects.get(project).ok_or(Error::NotInProject)?;
                match g.records.get(&epoch) {
                    Some(r) => Ok(r),
                    None if !g.removed
                        && g.group
                            .as_ref()
                            .is_some_and(|gr| epoch > gr.epoch().as_u64()) =>
                    {
                        Err(Error::Behind(epoch))
                    }
                    None => Err(Error::UnknownEpoch(epoch)),
                }
            }
        }
    }

    /// Seals an operation or a snapshot of a project at the current epoch of its scope, signed by this device's
    /// leaf.
    pub fn seal_for(
        &self,
        what: What,
        project: &ProjectId,
        scope: Scope,
        body: Vec<u8>,
    ) -> Result<Sealed, Error> {
        let record = match scope {
            Scope::Team => self.current_record()?,
            Scope::Project => {
                let epoch = self.project_epoch(project)?;
                self.project_record(project, epoch)
                    .ok_or(Error::UnknownEpoch(epoch))?
            }
        };
        Sealed::seal(
            what,
            record,
            scope,
            project,
            random_nonce(self.provider.rand())?,
            body,
            |bytes| self.signer.sign(bytes).map_err(mls),
        )
    }

    /// The author part of what this device writes.
    pub fn author(&self) -> crate::ops::Author {
        crate::ops::Author {
            member: self.identity.member,
            device: self.identity.device,
            key: self.signer.to_public_vec(),
        }
    }
}
