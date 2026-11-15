//! The bodies of the fuzzing targets (`fuzz/`, by cargo-fuzz): run by hand, never in the CI (TS-18 runs them long).
//!
//! - [`commit`]: the verification of the commits (`policy::check_commit`, behind MLS). The fuzzer's bytes drive a
//!   commit built by raw MLS on a device of a small team (an administrator or a member, who adds strangers or devices
//!   of others, removes leaves, rewrites the roster), and a third device processes it. Whatever it takes must leave
//!   the team sound: the genesis kept, an administrator left, every device leaf of a member of the roster, and
//!   nothing a member may not do. The raw bytes are also given to the decoder of the commits.
//! - [`roster`]: the decoding of what a device reads from others: the roster (the group context's extension), the
//!   identities of the leaves, the messages of the team's stream, the entries and frames of the server.

use std::collections::BTreeSet;

use escouade_team_crypto::invite::{InviteLink, DEFAULT_LIFETIME};
use escouade_team_crypto::roster::{EscrowPublic, Roster, ROSTER_EXTENSION};
use escouade_team_crypto::{
    DeviceKind, Identity, MemberId, Processed, Publication, Purpose, Role, TeamDevice,
};
use openmls::prelude::tls_codec::Deserialize as _;
use openmls::prelude::tls_codec::Serialize as _;
use openmls::prelude::{
    Extension, Extensions, KeyPackage, LeafNodeIndex, MlsMessageBodyIn, MlsMessageIn,
    ProtocolVersion, UnknownExtension,
};
use openmls_traits::OpenMlsProvider;

use crate::wire;

const NOW: u64 = 1_800_000_000;

fn member(n: u8) -> MemberId {
    MemberId([n; 16])
}

/// A team in memory: Alice (an administrator), Bob (a member), Carol (a member, who receives).
struct Fixture {
    alice: TeamDevice,
    bob: TeamDevice,
    carol: TeamDevice,
}

fn apply_all(publication: Publication, devices: &mut [&mut TeamDevice]) {
    if let Some((_, commit)) = publication.commit {
        for d in devices.iter_mut() {
            d.process_commit(&commit)
                .expect("the fixture's commits pass");
        }
    }
}

fn join(admin: &mut TeamDevice, who: MemberId, others: &mut [&mut TeamDevice]) -> TeamDevice {
    let link = admin
        .invite(
            Purpose::NewMember(Role::Member),
            NOW,
            DEFAULT_LIFETIME,
            "team.example.org",
        )
        .expect("an invitation");
    let mut device = TeamDevice::new(who).expect("a device");
    let request = device.request_join(&link).expect("a request");
    let publication = admin.admit(&request, NOW).expect("admitted");
    let (_, delivery) = publication.welcome.clone().expect("a Welcome");
    apply_all(publication, others);
    device.accept_welcome(&delivery).expect("joined");
    device
}

impl Fixture {
    fn new() -> Fixture {
        let mut alice = TeamDevice::new(member(1)).expect("a device");
        alice.create_team(NOW, None).expect("a team");
        let mut bob = join(&mut alice, member(2), &mut []);
        let carol = join(&mut alice, member(3), &mut [&mut bob]);
        Fixture { alice, bob, carol }
    }
}

/// A KeyPackage of a fresh device of `who`.
fn key_package(who: MemberId) -> KeyPackage {
    let mut device = TeamDevice::new(who).expect("a device");
    let link = InviteLink {
        server: "x".into(),
        id: [0; 16],
        secret: [0; 32],
        team: [0; 32],
    };
    let request = device.request_join(&link).expect("a request");
    match MlsMessageIn::tls_deserialize_exact(&request.key_package)
        .expect("a KeyPackage")
        .extract()
    {
        MlsMessageBodyIn::KeyPackage(kp) => kp
            .validate(device.provider().crypto(), ProtocolVersion::Mls10)
            .expect("valid"),
        _ => unreachable!("a request holds a KeyPackage"),
    }
}

/// The roster the fuzzer asks for, from the current one.
fn mutate(roster: &Roster, op: u8, data: &mut impl Iterator<Item = u8>) -> Vec<u8> {
    let mut r = roster.clone();
    match op % 8 {
        0 => {
            r.members.insert(member(2), Role::Admin);
        }
        1 => {
            r.members.remove(&member(1));
        }
        2 => r.genesis.created_at ^= 1,
        3 => {
            r.members
                .insert(member(data.next().unwrap_or(9)), Role::Member);
        }
        4 => {
            r.escrow = Some(EscrowPublic {
                hpke_key: vec![data.next().unwrap_or(0); 32],
                holder: "x".into(),
            })
        }
        5 => r.recovery.signature_key = vec![data.next().unwrap_or(0); 32],
        6 => {
            for role in r.members.values_mut() {
                *role = Role::Member;
            }
        }
        _ => {
            // Raw bytes for the roster.
            let n = data.next().unwrap_or(0) as usize;
            return data.take(n).collect();
        }
    }
    r.to_bytes()
}

/// The verification of the commits: a commit the bytes build, by Alice (an administrator) or Bob (a member), checked
/// by Carol. What Carol takes must leave the team sound.
pub fn commit(data: &[u8]) {
    let mut fx = Fixture::new();
    // The decoder of the commits, on the raw bytes.
    let _ = fx.carol.process_commit(data);
    let mut fx = if fx.carol.epoch().ok() == Some(2) {
        fx
    } else {
        Fixture::new()
    };

    let mut bytes = data.iter().copied();
    let by_admin = bytes.next().unwrap_or(0) % 2 == 0;
    let before = fx.carol.roster().expect("a roster");
    let (committer_member, committer) = if by_admin {
        (member(1), &mut fx.alice)
    } else {
        (member(2), &mut fx.bob)
    };
    let mut adds = Vec::new();
    let mut removals = Vec::new();
    let mut roster_bytes = None;
    let mut self_update = false;
    while let Some(op) = bytes.next() {
        match op % 6 {
            0 => adds.push(key_package(member(bytes.next().unwrap_or(9)))),
            1 => adds.push(key_package(member(1 + bytes.next().unwrap_or(0) % 3))),
            2 => removals.push(LeafNodeIndex::new(u32::from(bytes.next().unwrap_or(0) % 4))),
            3 => {
                let op = bytes.next().unwrap_or(0);
                roster_bytes = Some(mutate(&before, op, &mut bytes));
            }
            4 => self_update = !self_update,
            _ => break,
        }
        if adds.len() + removals.len() > 6 {
            break;
        }
    }
    let commit = {
        let (provider, signer, group) = committer.mls();
        let mut builder = group
            .commit_builder()
            .force_self_update(self_update)
            .propose_adds(adds)
            .propose_removals(removals);
        if let Some(r) = roster_bytes {
            let Ok(ext) = Extensions::from_vec(vec![Extension::Unknown(
                ROSTER_EXTENSION,
                UnknownExtension(r),
            )]) else {
                return;
            };
            builder = match builder.propose_group_context_extensions(ext) {
                Ok(b) => b,
                Err(_) => return,
            };
        }
        let Ok(builder) = builder.load_psks(provider.storage()) else {
            return;
        };
        let Ok(built) = builder.build(provider.rand(), provider.crypto(), signer, |_| true) else {
            return;
        };
        let Ok(bundle) = built.stage_commit(provider) else {
            return;
        };
        let out = bundle
            .commit()
            .tls_serialize_detached()
            .expect("a commit encodes");
        let _ = group.clear_pending_commit(provider.storage());
        out
    };
    let Ok(Processed::Epoch(_)) = fx.carol.process_commit(&commit) else {
        return;
    };

    // Carol took it: the team must be sound.
    let after = fx.carol.roster().expect("a roster after");
    assert_eq!(after.genesis, before.genesis, "the genesis changed");
    assert!(after.admins() > 0, "no administrator left");
    let leaves = fx.carol.leaves().expect("the leaves");
    for leaf in &leaves {
        if leaf.kind == DeviceKind::Device {
            assert!(
                after.role(&leaf.member).is_some(),
                "a device of a member the roster does not have"
            );
        }
    }
    if !by_admin {
        assert_eq!(after, before, "a member changed the roster");
        let members: BTreeSet<MemberId> = leaves.iter().map(|i| i.member).collect();
        let before_members: BTreeSet<MemberId> = before.members.keys().copied().collect();
        assert!(
            members.is_subset(&before_members),
            "a member brought in a stranger"
        );
        let _ = committer_member;
    }
    let _: Vec<Identity> = leaves;
}

/// The decoding of what a device reads from others.
pub fn roster(data: &[u8]) {
    if let Ok(r) = Roster::from_bytes(data) {
        let again = Roster::from_bytes(&r.to_bytes()).expect("a roster encoded decodes");
        assert_eq!(again, r);
        let _ = r.to_extensions();
    }
    if let Ok(i) = Identity::from_bytes(data) {
        assert_eq!(Identity::from_bytes(&i.to_bytes()).expect("again"), i);
    }
    if let Ok(m) = wire::decode::<wire::TeamMessage>(data) {
        let again: wire::TeamMessage =
            wire::decode(&wire::encode(&m)).expect("a message encoded decodes");
        let _ = again;
    }
    let _ = wire::decode::<wire::Entries>(data);
    let _ = wire::decode::<wire::Frame>(data);
    let _ = wire::decode::<wire::Problem>(data);
    let _ = wire::decode::<escouade_team_crypto::content::SealedObject>(data);
    let _ = escouade_team_crypto::keys::EpochRecord::from_bytes(data);
}

#[cfg(test)]
mod tests {
    /// A few inputs of each kind, so that the targets' bodies build and hold in every test run.
    #[test]
    fn the_targets_hold_on_a_few_inputs() {
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for round in 0..24u8 {
            let len = (next() % 24) as usize;
            let mut data: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            if let Some(first) = data.first_mut() {
                *first = round % 2;
            }
            super::commit(&data);
            super::roster(&data);
        }
        // The rules' refusals, on purpose: a member who makes himself an administrator, an admin who drops all.
        super::commit(&[1, 3, 0]);
        super::commit(&[0, 3, 6]);
        super::commit(&[1, 0, 9]);
        super::commit(&[0, 2, 0, 2, 1]);
        let roster = super::Fixture::new().alice.roster().unwrap().to_bytes();
        super::roster(&roster);
    }
}
