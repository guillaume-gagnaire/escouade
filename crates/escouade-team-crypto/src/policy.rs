//! The rules a commit must follow for a device to merge it: MLS's authentication service, ours (RFC 9420 §5.3).
//!
//! MLS proves that a commit comes from a leaf of the group and that every member reaches the same state; it does
//! not say who may add whom. Every device checks each commit before merging it, the same way, so that all honest
//! devices take or refuse the same commits:
//! - **an administrator** (a member whose role is Admin in the roster, or the recovery leaf) adds and removes
//!   members and devices, and changes the roster (roles, recovery key, escrow key);
//! - **a member** adds and removes their own devices only, and never changes the roster;
//! - **a machine of the team** (role of service, OE-1) is checked as a member: its own devices only, never the roster.
//!   Its role is given at its entry and never changes (neither a person made a machine, nor a machine made a person:
//!   a machine leaves, another is enrolled). What it signs beside the commits is [`may_sign`]'s table, which every
//!   device applies at the reading of the team's streams (its settings, its circuit breakers, its private store, its
//!   list of projects, the answers to invitations);
//! - **an external commit** (a joiner from outside) is admitted for the recovery key only, whose signature key is
//!   the roster's; it may remove an older recovery leaf, nothing else;
//! - **a leaf** never changes its identity or its signature key in a commit;
//! - **afterwards**, every device leaf belongs to a member of the roster, a recovery leaf has the roster's recovery
//!   key, and the team keeps an administrator.
//!
//! The server is not a leaf: it cannot commit, so it cannot slip a key in. A GroupInfo would let it try an external
//! commit, which these rules refuse; and it never sees one in the clear (`recovery.rs`).
//!
//! **What a commit says it is** (SR2-02): its authenticated data carries its [`CommitScope`], signed with it. A
//! member's commit says [`CommitScope::Own`] (their own devices, nothing else) and is checked as a member's even from
//! an administrator; a commit that says [`CommitScope::Admin`] must come from an administrator. The client tells the
//! server the same in clear, and the server takes a member's commit only when it says `Own`: what the server reads
//! decides nothing for the devices, the signed scope does.

use openmls::prelude::{LeafNodeIndex, MlsGroup, Proposal, Sender, StagedCommit};

use crate::identity::{DeviceKind, Identity};
use crate::roster::{Role, Roster};
use crate::Error;

fn refuse(why: impl Into<String>) -> Error {
    Error::Policy(why.into())
}

/// What a commit says it does, in its authenticated data (SR2-02): an administrator's change, or a member's change of
/// their own devices only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitScope {
    Admin,
    Own,
}

impl CommitScope {
    /// The byte the commit carries.
    pub fn byte(self) -> u8 {
        match self {
            CommitScope::Admin => 0,
            CommitScope::Own => 1,
        }
    }

    /// The scope a team's commit says by its authenticated data: none (an external commit, the recovery key's) is an
    /// administrator's; anything but one known byte is refused.
    pub fn of_team_aad(aad: &[u8]) -> Result<Self, Error> {
        match aad {
            [] | [0] => Ok(CommitScope::Admin),
            [1] => Ok(CommitScope::Own),
            _ => Err(refuse("a commit whose authenticated data says no scope")),
        }
    }

    /// The authenticated data of a team's commit.
    pub fn team_aad(self) -> Vec<u8> {
        vec![self.byte()]
    }
}

/// What a device signs for the team, beside the content of a project (OE-1): what [`may_sign`] answers by role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Adds or removes a member, or another member's device.
    Member,
    /// Changes a role, the recovery key or the escrow key: the roster.
    Role,
    /// A version of the team's settings (TS-13), or its list of projects (TS-8), or the key of its costs (TS-16).
    Setting,
    /// Sets or lifts a circuit breaker (TS-15).
    Breaker,
    /// Proposes, publishes or dismisses a plugin of the team's private store (TS-14).
    PrivatePlugin,
    /// Answers an invitation: admits its request, or registers its token for another (TS-3).
    Invitation,
    /// Adds or removes one's own devices.
    OwnDevices,
    /// Writes the content of a project opened to it (its Kanban, its docs, its memory), or takes a ticket of it.
    Content,
}

/// Whether a device of `role` may sign `act` (OE-1). An administrator signs everything; a member their own devices,
/// the content and their proposals of plugins (an administrator publishes them); a machine of the team its own
/// devices and the content of the projects opened to it, nothing else: no member, no role, no setting, no circuit
/// breaker, no private plugin, no answer to an invitation. Every device applies it to what it reads, whatever the
/// author's client let through, and whatever the server says.
pub fn may_sign(role: Role, act: Act) -> bool {
    match role {
        Role::Admin => true,
        Role::Member => matches!(act, Act::OwnDevices | Act::Content | Act::PrivatePlugin),
        Role::Service => matches!(act, Act::OwnDevices | Act::Content),
    }
}

/// A role changed in the roster from `old` to `new` (OE-1): a machine of the team stays one, a person stays one.
pub fn check_roles(old: &Roster, new: &Roster) -> Result<(), Error> {
    for (member, role) in &new.members {
        if let Some(before) = old.role(member) {
            if before.is_person() != role.is_person() {
                return Err(refuse(
                    "the role of service is given at the entry only: a person is never made a machine of the team, nor a machine a person",
                ));
            }
        }
    }
    if new.role(&new.genesis.owner) == Some(Role::Service) {
        return Err(refuse("the owner of a team is a person"));
    }
    Ok(())
}

/// A leaf as the rules see it.
struct Leaf {
    identity: Identity,
    signature_key: Vec<u8>,
}

fn leaf_at(group: &MlsGroup, index: LeafNodeIndex) -> Result<Leaf, Error> {
    let member = group
        .member_at(index)
        .ok_or_else(|| refuse(format!("no leaf at {index:?}")))?;
    Ok(Leaf {
        identity: Identity::from_credential(&member.credential)?,
        signature_key: member.signature_key,
    })
}

/// Whether a leaf may act as an administrator under `roster`.
fn is_admin(roster: &Roster, leaf: &Leaf) -> bool {
    match leaf.identity.kind {
        DeviceKind::Recovery => leaf.signature_key == roster.recovery.signature_key,
        DeviceKind::Device => roster.role(&leaf.identity.member) == Some(Role::Admin),
    }
}

/// Checks a commit staged on `group` (before its merge), sent by `sender`, that says `scope`. Returns the roster of
/// the new epoch.
pub fn check_commit(
    group: &MlsGroup,
    sender: &Sender,
    staged: &StagedCommit,
    scope: CommitScope,
) -> Result<Roster, Error> {
    let old = Roster::from_extensions(group.extensions())?;
    let new = Roster::from_context(staged.group_context())?;
    if new.genesis != old.genesis {
        return Err(refuse("the genesis of a team never changes"));
    }

    // Who commits.
    let external = matches!(sender, Sender::NewMemberCommit);
    let committer = match sender {
        Sender::Member(index) => {
            let leaf = leaf_at(group, *index)?;
            if leaf.identity.kind == DeviceKind::Recovery
                && leaf.signature_key != old.recovery.signature_key
            {
                return Err(refuse(
                    "a recovery leaf whose key is no longer the roster's",
                ));
            }
            if leaf.identity.kind == DeviceKind::Device && old.role(&leaf.identity.member).is_none()
            {
                return Err(refuse("a leaf of a member who is not in the roster"));
            }
            leaf
        }
        Sender::NewMemberCommit => {
            let node = staged
                .update_path_leaf_node()
                .ok_or_else(|| refuse("an external commit without its leaf"))?;
            let leaf = Leaf {
                identity: Identity::from_credential(node.credential())?,
                signature_key: node.signature_key().as_slice().to_vec(),
            };
            if leaf.identity.kind != DeviceKind::Recovery
                || leaf.signature_key != old.recovery.signature_key
            {
                return Err(refuse(
                    "an external commit is admitted for the team's recovery key only",
                ));
            }
            leaf
        }
        _ => return Err(refuse("a commit from outside the group")),
    };
    let admin = is_admin(&old, &committer);
    if scope == CommitScope::Admin && !admin {
        return Err(refuse(
            "a commit that says it is an administrator's, from a member",
        ));
    }
    // A commit that says it is a member's is checked as one, whoever sent it.
    let admin = admin && scope == CommitScope::Admin;

    // A leaf keeps its identity and its key.
    if !external {
        if let Some(node) = staged.update_path_leaf_node() {
            let identity = Identity::from_credential(node.credential())?;
            if identity != committer.identity
                || node.signature_key().as_slice() != committer.signature_key
            {
                return Err(refuse("a leaf changed its identity or its signature key"));
            }
        }
    }

    // What the commit does.
    let mut leaves: Vec<(LeafNodeIndex, Leaf)> = group
        .members()
        .map(|m| {
            Ok((
                m.index,
                Leaf {
                    identity: Identity::from_credential(&m.credential)?,
                    signature_key: m.signature_key,
                },
            ))
        })
        .collect::<Result<_, Error>>()?;
    let mut added: Vec<Leaf> = Vec::new();
    for queued in staged.queued_proposals() {
        if queued.sender() != sender {
            return Err(refuse("a proposal by another than the committer"));
        }
        match queued.proposal() {
            Proposal::Add(add) => {
                if external {
                    return Err(refuse("an external commit adds nobody"));
                }
                let node = add.key_package().leaf_node();
                let identity = Identity::from_credential(node.credential())?;
                if identity.kind != DeviceKind::Device {
                    return Err(refuse(
                        "only devices are added; the recovery key joins by itself",
                    ));
                }
                if new.role(&identity.member).is_none() {
                    return Err(refuse("a device of a member the roster does not have"));
                }
                if !admin
                    && (identity.member != committer.identity.member
                        || old.role(&identity.member).is_none())
                {
                    return Err(refuse("a member adds their own devices only"));
                }
                added.push(Leaf {
                    identity,
                    signature_key: node.signature_key().as_slice().to_vec(),
                });
            }
            Proposal::Remove(remove) => {
                let target = leaf_at(group, remove.removed())?;
                if external && target.identity.kind != DeviceKind::Recovery {
                    return Err(refuse(
                        "an external commit removes an older recovery leaf only",
                    ));
                }
                if !admin && target.identity.member != committer.identity.member {
                    return Err(refuse("a member removes their own devices only"));
                }
                leaves.retain(|(i, _)| *i != remove.removed());
            }
            Proposal::Update(update) => {
                let identity = Identity::from_credential(update.leaf_node().credential())?;
                if identity != committer.identity {
                    return Err(refuse("an update changes no identity"));
                }
            }
            Proposal::GroupContextExtensions(_) => {
                if external || !admin {
                    return Err(refuse("only an administrator changes the roster"));
                }
            }
            Proposal::ExternalInit(_) => {
                if !external {
                    return Err(refuse("an external init outside an external commit"));
                }
            }
            _ => return Err(refuse("a proposal the team does not use")),
        }
    }
    if new != old && (external || !admin) {
        return Err(refuse("only an administrator changes the roster"));
    }
    check_roles(&old, &new)?;

    // The state after it.
    let mut after: Vec<Leaf> = leaves.into_iter().map(|(_, l)| l).collect();
    after.extend(added);
    if external {
        after.push(committer);
    }
    for leaf in &after {
        match leaf.identity.kind {
            DeviceKind::Device if new.role(&leaf.identity.member).is_none() => {
                return Err(refuse(
                    "a device stays whose member left the roster: remove it in the same commit",
                ));
            }
            DeviceKind::Recovery if leaf.signature_key != new.recovery.signature_key => {
                return Err(refuse("a recovery leaf stays whose key the roster replaced: remove it in the same commit"));
            }
            _ => {}
        }
    }
    if new.admins() == 0 {
        return Err(refuse("a team keeps at least one administrator"));
    }
    Ok(new)
}
