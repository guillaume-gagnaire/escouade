//! The rules a commit must follow for a device to merge it: MLS's authentication service, ours (RFC 9420 §5.3).
//!
//! MLS proves that a commit comes from a leaf of the group and that every member reaches the same state; it does
//! not say who may add whom. Every device checks each commit before merging it, the same way, so that all honest
//! devices take or refuse the same commits:
//! - **an administrator** (a member whose role is Admin in the roster, or the recovery leaf) adds and removes
//!   members and devices, and changes the roster (roles, recovery key, escrow key);
//! - **a member** adds and removes their own devices only, and never changes the roster;
//! - **an external commit** (a joiner from outside) is admitted for the recovery key only, whose signature key is
//!   the roster's; it may remove an older recovery leaf, nothing else;
//! - **a leaf** never changes its identity or its signature key in a commit;
//! - **afterwards**, every device leaf belongs to a member of the roster, a recovery leaf has the roster's recovery
//!   key, and the team keeps an administrator.
//!
//! The server is not a leaf: it cannot commit, so it cannot slip a key in. A GroupInfo would let it try an external
//! commit, which these rules refuse; and it never sees one in the clear (`recovery.rs`).

use openmls::prelude::{LeafNodeIndex, MlsGroup, Proposal, Sender, StagedCommit};

use crate::identity::{DeviceKind, Identity};
use crate::roster::{Role, Roster};
use crate::Error;

fn refuse(why: impl Into<String>) -> Error {
    Error::Policy(why.into())
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

/// Checks a commit staged on `group` (before its merge), sent by `sender`. Returns the roster of the new epoch.
pub fn check_commit(
    group: &MlsGroup,
    sender: &Sender,
    staged: &StagedCommit,
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
