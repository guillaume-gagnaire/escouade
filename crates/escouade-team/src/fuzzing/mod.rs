//! The bodies of the fuzzing targets (the package `fuzz/` at the root of the repository, by cargo-fuzz; TS-18). The
//! CI runs each a short while on Linux (`scripts/fuzz.sh`), `.github/workflows/fuzz.yml` long; the test below plays
//! each on a few inputs and on the seeds committed in `fuzz/seeds/<target>/`, so they build and hold in every run.
//!
//! Each target takes what comes from the network or the disk before it is verified, and checks properties, not only
//! that nothing panics: an entry whose signature is false is never applied, a forged head or an older state is seen,
//! the same operations in any order give the same state, and what a peer gives never makes the device allocate
//! without a bound (libFuzzer's `-malloc_limit_mb`, `scripts/fuzz.sh`).
//!
//! - [`commit`]: the verification of the commits (`policy::check_commit`, behind MLS). The fuzzer's bytes drive a
//!   commit built by raw MLS on a device of a small team (an administrator or a member, who adds strangers or devices
//!   of others, removes leaves, rewrites the roster), and a third device processes it. Whatever it takes must leave
//!   the team sound: the genesis kept, an administrator left, every device leaf of a member of the roster, and
//!   nothing a member may not do. The raw bytes are also given to the decoder of the commits.
//! - [`roster`]: the decoding of what a device reads from others: the roster (the group context's extension), the
//!   identities of the leaves, the messages of the team's stream, the entries and frames of the server.
//! - [`operation`]: a project's operation, sealed and opened (TS-5), and one signed by a key that is no leaf's.
//! - [`journal`]: the members' journal as a device checks it: heads cited for an epoch (a fork), the state the server
//!   shows (a rollback), a checkpoint put back, a member's beacon, a sealed object, a history link, all changed.
//! - [`convergence`]: operations of three devices, in two orders, some played twice, with a snapshot in between.
//! - [`snapshot`]: a snapshot's state as it comes, and a sealed snapshot changed.
//! - [`envelope`]: the escrow and recovery wraps (HPKE), the escrow key's texts.
//! - [`invite`]: an invitation's link, the MAC of a request to join, the confirmation of a Welcome.
//! - [`handshake`]: the short code and the PAKE of the pairing (TS-3).
//! - [`catalog`]: an entry of the team's list of projects (TS-8): its addresses, what `check` lets through.
//! - [`settings`]: the team's signed `.escouade.yml` (TS-13): the file, and the rules that choose its version.

mod catalog;
mod convergence;
mod envelope;
mod handshake;
mod invite;
mod journal;
mod settings;
mod snapshot;

pub use catalog::catalog;
pub use convergence::convergence;
pub use envelope::envelope;
pub use handshake::handshake;
pub use invite::invite;
pub use journal::journal;
pub use settings::settings;
pub use snapshot::snapshot;

/// A target's body.
pub type Target = fn(&[u8]);

/// The targets by name, as `fuzz/fuzz_targets/` and `fuzz/seeds/` name them.
pub const TARGETS: &[(&str, Target)] = &[
    ("commit", commit),
    ("roster", roster),
    ("operation", operation),
    ("journal", journal),
    ("convergence", convergence),
    ("snapshot", snapshot),
    ("envelope", envelope),
    ("invite", invite),
    ("handshake", handshake),
    ("catalog", catalog),
    ("settings", settings),
];

/// `base` changed by the fuzzer's bytes: the first says where, the next ones are xored from there; past them, a byte
/// of 0xff cuts the result there (a truncated message).
pub(crate) fn alter(base: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = base.to_vec();
    let Some((&at, rest)) = data.split_first() else {
        return out;
    };
    let start = at as usize % out.len().max(1);
    for (i, b) in rest.iter().enumerate() {
        if let Some(x) = out.get_mut(start + i) {
            *x ^= b;
        }
    }
    if rest.len() > 1 && rest[rest.len() - 1] == 0xff {
        out.truncate(start + rest.len() / 2);
    }
    out
}

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
        member: None,
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

/// A team of two after three epochs, made once per thread (libFuzzer runs one): Alice, an administrator, made it
/// with an escrow key; Bob, a member, joined at epoch 1; two rotations since. What it published at epoch 3 is kept,
/// with an object, a beacon and a history link of Alice's, for the targets to change.
pub(crate) struct World {
    pub alice: TeamDevice,
    pub bob: TeamDevice,
    pub group: Vec<u8>,
    pub recovery: escouade_team_crypto::recovery::RecoveryKey,
    pub escrow: escouade_team_crypto::recovery::EscrowKey,
    pub recovery_wrap: escouade_team_crypto::recovery::RecoveryWrap,
    pub escrow_wrap: escouade_team_crypto::recovery::EscrowWrap,
    /// The link of epoch 3: it holds the record of epoch 2.
    pub link: escouade_team_crypto::history::HistoryLink,
    pub object: escouade_team_crypto::content::SealedObject,
    pub plain: Vec<u8>,
    pub beacon: escouade_team_crypto::beacon::Beacon,
}

impl World {
    fn new() -> World {
        use escouade_team_crypto::recovery::EscrowKey;
        let provider = openmls_rust_crypto::OpenMlsRustCrypto::default();
        let escrow =
            EscrowKey::generate(provider.crypto(), provider.rand(), "Org").expect("an escrow");
        let mut alice = TeamDevice::new(member(1)).expect("a device");
        let (recovery, _) = alice
            .create_team(NOW, Some(escrow.public()))
            .expect("a team");
        let mut bob = join(&mut alice, member(2), &mut []);
        let p = alice.rotate().expect("epoch 2");
        apply_all(p, &mut [&mut bob]);
        let p = alice.rotate().expect("epoch 3");
        let link = p.history.clone().expect("a history link");
        let recovery_wrap = p.recovery.clone().expect("a recovery wrap");
        let escrow_wrap = p.escrow.clone().expect("an escrow wrap");
        apply_all(p, &mut [&mut bob]);
        let plain = b"{\"title\":\"Un ticket\"}".to_vec();
        let object = alice
            .seal(PROJECT, escouade_team_crypto::ObjectId([3; 16]), 1, &plain)
            .expect("sealed");
        let beacon = alice.beacon(NOW).expect("a beacon");
        let group = alice.group_id().expect("a group");
        World {
            alice,
            bob,
            group,
            recovery,
            escrow,
            recovery_wrap,
            escrow_wrap,
            link,
            object,
            plain,
            beacon,
        }
    }

    /// The world of this thread.
    pub fn with<T>(f: impl FnOnce(&World) -> T) -> T {
        thread_local! {
            static WORLD: World = World::new();
        }
        WORLD.with(f)
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

/// A team of one in memory, made once: an epoch's record and an operation sealed under it, to mutate.
fn sealed_fixture() -> &'static (escouade_team_crypto::keys::EpochRecord, Vec<u8>, Vec<u8>) {
    use escouade_team_crypto::ops::{OpBody, Operation, Scope, What};
    static FIXTURE: std::sync::OnceLock<(
        escouade_team_crypto::keys::EpochRecord,
        Vec<u8>,
        Vec<u8>,
    )> = std::sync::OnceLock::new();
    FIXTURE.get_or_init(|| {
        let mut alice = TeamDevice::new(member(1)).expect("a device");
        alice.create_team(NOW, None).expect("a team");
        let record = alice.record(0).expect("epoch 0").clone();
        let op = Operation {
            project: PROJECT,
            author: alice.author(),
            dseq: 1,
            prev: [0; 32],
            team_epoch: 0,
            team_head: record.head,
            hlc: escouade_team_crypto::hlc::Hlc {
                wall: NOW * 1000,
                count: 0,
                device: alice.identity().device,
            },
            body: OpBody::Set {
                object: escouade_team_crypto::ObjectId([2; 16]),
                field: "title".into(),
                value: serde_json::json!("Un titre"),
                base: None,
            },
        };
        let sealed = alice
            .seal_for(What::Operation, &PROJECT, Scope::Team, op.to_bytes())
            .expect("sealed");
        (record, wire::encode(&sealed), op.to_bytes())
    })
}

const PROJECT: escouade_team_crypto::ProjectId = escouade_team_crypto::ProjectId([7; 16]);

/// A snapshot sealed by the device of [`sealed_fixture`]'s team, made once: its epoch's record, its bytes as the
/// server keeps them, and its plain bytes.
fn sealed_snapshot() -> &'static (escouade_team_crypto::keys::EpochRecord, Vec<u8>, Vec<u8>) {
    use escouade_team_crypto::ops::{Chain, Scope, Snapshot, What};
    static FIXTURE: std::sync::OnceLock<(
        escouade_team_crypto::keys::EpochRecord,
        Vec<u8>,
        Vec<u8>,
    )> = std::sync::OnceLock::new();
    FIXTURE.get_or_init(|| {
        let mut alice = TeamDevice::new(member(1)).expect("a device");
        alice.create_team(NOW, None).expect("a team");
        let record = alice.record(0).expect("epoch 0").clone();
        let copy = crate::replica::Replica::in_memory().expect("a copy");
        let (op, digest) = &convergence::operations(&[[0, 0, 0, 0, 1, 1]])[0];
        copy.apply(op, digest, Some(1)).expect("applied");
        let snapshot = Snapshot {
            project: PROJECT,
            author: alice.author(),
            upto: 1,
            team_epoch: 0,
            team_head: record.head,
            chains: [(
                op.author.device,
                Chain {
                    dseq: 1,
                    digest: *digest,
                },
            )]
            .into_iter()
            .collect(),
            state: copy.dump().expect("a dump"),
        };
        let sealed = alice
            .seal_for(What::Snapshot, &PROJECT, Scope::Team, snapshot.to_bytes())
            .expect("sealed");
        (record, wire::encode(&sealed), snapshot.to_bytes())
    })
}

/// An operation that claims an author but is signed by another key (the server, or a member who knows the epoch's
/// key and signs for someone else): `open_operation` refuses it, whoever it claims.
fn forged(data: &[u8]) {
    use ed25519_dalek::Signer as _;
    use escouade_team_crypto::ops::{Operation, Scope, Sealed, What};
    let (record, _, plain) = sealed_fixture();
    let Ok(op) = Operation::from_bytes(plain) else {
        return;
    };
    let mut seed = [0u8; 32];
    for (i, b) in data.iter().take(32).enumerate() {
        seed[i] = *b;
    }
    let key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let mut claimed = op.clone();
    // Half the time it claims its own key, which no leaf holds.
    if data.first().is_some_and(|b| b % 2 == 1) {
        claimed.author.key = key.verifying_key().to_bytes().to_vec();
    }
    let Ok(sealed) = Sealed::seal(
        What::Operation,
        record,
        Scope::Team,
        &PROJECT,
        [data.len() as u8; 24],
        claimed.to_bytes(),
        |bytes| Ok(key.sign(bytes).to_bytes().to_vec()),
    ) else {
        return;
    };
    assert!(
        sealed.open_operation(record, &PROJECT).is_err(),
        "an operation signed by no leaf opened"
    );
}

/// The decoding of a project's operation (TS-5): what the server serves, sealed, and what it holds once opened.
/// - The bytes as a sealed operation, an operation, a snapshot, a project roster, a stamp: decoded without a panic,
///   and what decodes encodes back to the same;
/// - a sealed operation of the fixture, mutated by the bytes (xor from an offset): it opens to the very operation
///   sealed, or not at all;
/// - an operation that decodes, applied to a copy in memory twice: the second time changes nothing.
pub fn operation(data: &[u8]) {
    use escouade_team_crypto::ops::{Operation, Sealed, Snapshot};
    forged(data);
    let _ = wire::decode::<Sealed>(data);
    let _ = escouade_team_crypto::project::ProjectRoster::from_bytes(data);
    let _ = escouade_team_crypto::hlc::Hlc::from_bytes(data);
    if let Ok(s) = Snapshot::from_bytes(data) {
        assert_eq!(Snapshot::from_bytes(&s.to_bytes()).expect("again"), s);
    }
    let (record, sealed, plain) = sealed_fixture();
    if let Ok(s) = wire::decode::<Sealed>(data) {
        let _ = s.open_operation(record, &PROJECT);
        let _ = s.open_snapshot(record, &PROJECT);
    }
    if let Some((&offset, flips)) = data.split_first() {
        let mut mutated = sealed.clone();
        let start = offset as usize % mutated.len().max(1);
        for (i, b) in flips.iter().enumerate() {
            if let Some(x) = mutated.get_mut(start + i) {
                *x ^= b;
            }
        }
        if let Ok(s) = wire::decode::<Sealed>(&mutated) {
            if let Ok(op) = s.open_operation(record, &PROJECT) {
                assert_eq!(
                    op.to_bytes(),
                    *plain,
                    "a mutated operation opens only as itself"
                );
            }
        }
    }
    if let Ok(op) = Operation::from_bytes(data) {
        let again = Operation::from_bytes(&op.to_bytes()).expect("an operation encoded decodes");
        assert_eq!(again, op);
        let copy = crate::replica::Replica::in_memory().expect("a copy");
        let digest = Operation::digest_of(data);
        if copy.apply(&op, &digest, Some(1)).is_ok() {
            let before = copy.state().expect("its state");
            assert!(!copy.apply(&op, &digest, Some(1)).expect("again"));
            assert_eq!(copy.state().expect("its state"), before);
            let _ = copy.objects(None);
            let _ = copy.conflicts(&op.body.object());
            let _ = copy.versions(&op.body.object());
        }
    }
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
            super::operation(&data);
        }
        // The sealed operation of the fixture, mutated here and there, and an operation that decodes.
        for i in 0..8u8 {
            super::operation(&[i.wrapping_mul(37), 1, 0, 4]);
        }
        super::operation(&super::sealed_fixture().2);
        // The rules' refusals, on purpose: a member who makes himself an administrator, an admin who drops all.
        super::commit(&[1, 3, 0]);
        super::commit(&[0, 3, 6]);
        super::commit(&[1, 0, 9]);
        super::commit(&[0, 2, 0, 2, 1]);
        let roster = super::Fixture::new().alice.roster().unwrap().to_bytes();
        super::roster(&roster);
    }

    /// The targets added by TS-18, on random inputs of every length up to 200 bytes.
    #[test]
    fn the_targets_of_ts_18_hold_on_random_inputs() {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for (name, target) in super::TARGETS.iter().skip(3) {
            for round in 0..40u64 {
                let len = (next() % 200) as usize;
                let data: Vec<u8> = (0..len).map(|_| next() as u8).collect();
                let caught = std::panic::catch_unwind(|| target(&data));
                assert!(caught.is_ok(), "{name} on round {round}: {data:?}");
            }
        }
    }

    /// `ESCOUADE_WRITE_SEEDS=1` writes the seeds of `fuzz/seeds/` again: for each target, a few inputs that reach
    /// its paths, and the vectors of escouade-team-crypto (`vectors/team-crypto-v1.json`) in the form the server
    /// serves them. Without the variable, nothing.
    #[test]
    fn write_the_seeds() {
        use crate::wire;
        use escouade_team_crypto::content::SealedObject;
        use escouade_team_crypto::history::HistoryLink;
        use escouade_team_crypto::keys::EpochRecord;
        use escouade_team_crypto::recovery::RecoveryWrap;
        if std::env::var("ESCOUADE_WRITE_SEEDS").as_deref() != Ok("1") {
            return;
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
        let write = |target: &str, name: &str, bytes: &[u8]| {
            let dir = root.join(target);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(name), bytes).unwrap();
        };
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../escouade-team-crypto/vectors/team-crypto-v1.json"
        ))
        .unwrap();
        let v = |path: &[&str]| -> serde_json::Value {
            let mut x = &vectors;
            for p in path {
                x = &x[*p];
            }
            x.clone()
        };
        let record: EpochRecord = serde_json::from_value(v(&["object", "record"])).unwrap();
        let object: SealedObject = serde_json::from_value(v(&["object", "sealed"])).unwrap();
        let link: HistoryLink = serde_json::from_value(v(&["history", "link"])).unwrap();
        let wrap: RecoveryWrap = serde_json::from_value(v(&["recovery_wrap", "wrap"])).unwrap();
        let url = v(&["invitation", "url"]).as_str().unwrap().to_owned();

        // commit, roster, operation: the inputs of the test above.
        for (i, d) in [
            &[1u8, 3, 0][..],
            &[0, 3, 6],
            &[1, 0, 9],
            &[0, 2, 0, 2, 1],
            &[0, 0, 7, 4],
        ]
        .iter()
        .enumerate()
        {
            write("commit", &format!("rule-{i}"), d);
        }
        write(
            "roster",
            "roster",
            &super::Fixture::new().alice.roster().unwrap().to_bytes(),
        );
        write(
            "roster",
            "vector-identity",
            &hex::decode(v(&["recovery", "identity"]).as_str().unwrap()).unwrap(),
        );
        write("roster", "vector-object", &wire::encode(&object));
        write("roster", "vector-record", &record.to_bytes());
        let (_, sealed, plain) = super::sealed_fixture();
        write("operation", "sealed", sealed);
        write("operation", "plain", plain);
        write("operation", "changed", &[3, 1, 0, 4]);
        // journal: each kind of claim, on each epoch.
        for kind in 0..6u8 {
            write(
                "journal",
                &format!("claim-{kind}"),
                &[kind, 3, 1, kind, 7, kind, 0, 2, 9, 1],
            );
        }
        write("journal", "vector-object", &wire::encode(&object));
        write("journal", "vector-link", &wire::encode(&link));
        // convergence: three devices on the same field, a delete, versions.
        write(
            "convergence",
            "same-field",
            &[
                0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 2, 2, 2, 1, 0, 0, 3, 2, 3, 1, 2, 5, 1, 9, 9,
            ],
        );
        write(
            "convergence",
            "delete-and-versions",
            &[
                0, 0, 1, 0, 1, 1, 1, 4, 1, 0, 0, 1, 2, 5, 1, 0, 3, 1, 0, 5, 1, 0, 4, 2, 7, 3, 1,
            ],
        );
        // snapshot: a copy's state, the sealed snapshot.
        write("snapshot", "sealed", &super::sealed_snapshot().1);
        write("snapshot", "state", &{
            let c = crate::replica::Replica::in_memory().unwrap();
            for (op, d) in super::convergence::operations(&[
                [0, 0, 0, 0, 1, 1],
                [1, 3, 0, 2, 4, 1],
                [2, 5, 1, 0, 2, 2],
            ]) {
                c.apply(&op, &d, None).unwrap();
            }
            c.dump().unwrap()
        });
        // envelope: the recovery wrap of the vectors, the escrow key's texts.
        write("envelope", "vector-recovery-wrap", &wire::encode(&wrap));
        let (public, secret) = crate::escrow::new_pair("Org").unwrap();
        write(
            "envelope",
            "escrow-public",
            crate::escrow::public_text(&public).as_bytes(),
        );
        write("envelope", "escrow-secret", secret.as_bytes());
        write("envelope", "vector-record", &record.to_bytes());
        // invite: the link of the vectors, another device's, a request.
        write("invite", "vector-url", url.as_bytes());
        write(
            "invite",
            "own-device",
            format!("{url}&m=01010101010101010101010101010101").as_bytes(),
        );
        write("invite", "changed", &[9, 1, 2, 3, 4, 5, 6, 7, 8]);
        // handshake: codes as typed, a wrong try.
        write("handshake", "typed", b"abc4-m2xz");
        write("handshake", "loose", b" K7Q o-IL ZZ ");
        write("handshake", "wrong", &[0, 1, 2, 3]);
        // catalog: addresses that pass and that do not.
        for (i, a) in [
            "https://github.com/acme/api.git",
            "git@github.com:acme/api.git",
            "ssh://git@gitlab.example.org:2222/acme/web",
            "--upload-pack=touch /tmp/x",
            "ext::sh -c touch% /tmp/x",
            "file:///etc/passwd",
            "https://user:token@github.com/acme/api",
            "C:repo",
            "https://github.com/acme/-x",
        ]
        .iter()
        .enumerate()
        {
            write("catalog", &format!("address-{i}"), a.as_bytes());
        }
        write(
            "catalog",
            "entry",
            &wire::encode(&crate::catalog::Entry::Put {
                id: [1; 16],
                project: super::catalog::listed(),
                at: 1,
            }),
        );
        // settings: a file, and entries (versions by an administrator, a member, a cosignature).
        write("settings", "file", br#"{"version":1,"enforced":{"permissions.modes":["plan","default"]},"proposed":{"worktrees.setup":"npm ci"},"quorum":true}"#);
        write(
            "settings",
            "never",
            br#"{"version":1,"enforced":{"worktrees.setup":"curl x | sh"}}"#,
        );
        write(
            "settings",
            "chain",
            &[
                0, 2, 1, 0, 4, 1, 0, 17, 5, 1, 2, 1, 1, 4, 2, 1, 2, 33, 3, 1, 3, 0, 1, 0, 1, 1, 0,
                0, 2, 4, 1, 16, 2, 3,
            ],
        );
    }

    /// The seeds committed in `fuzz/seeds/<target>/` (the vectors of escouade-team-crypto among them, and what the
    /// fuzzing found): each target holds on each of its seeds, in every test run.
    #[test]
    fn the_targets_hold_on_their_seeds() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
        let mut played = 0;
        for (name, target) in super::TARGETS {
            let dir = root.join(name);
            let Ok(entries) = std::fs::read_dir(&dir) else {
                panic!("no seeds for {name} in {}", dir.display());
            };
            for e in entries {
                let path = e.unwrap().path();
                let data = std::fs::read(&path).unwrap();
                let caught = std::panic::catch_unwind(|| target(&data));
                assert!(caught.is_ok(), "{name} on {}", path.display());
                played += 1;
            }
        }
        assert!(played >= super::TARGETS.len());
    }
}
