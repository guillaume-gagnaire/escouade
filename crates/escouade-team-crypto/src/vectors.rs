//! The test vectors of what is ours above MLS (`vectors/team-crypto-v1.json`): fixed inputs, the outputs expected.
//! Another implementation (the mobile app, an auditor's script) checks itself against them. MLS's own vectors
//! (RFC 9420's test vectors, from the MLS working group) are run by openmls in its own suite.
//!
//! `ESCOUADE_WRITE_VECTORS=1 cargo nextest run -p escouade-team-crypto vectors` writes the file again; the HPKE wrap
//! is randomised, so its bytes change each time, and the check opens it rather than compares it.

use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::signatures::Signer;
use openmls_traits::OpenMlsProvider;
use serde::{Deserialize, Serialize};

use crate::content::{ObjectHeader, SealedObject};
use crate::history::HistoryLink;
use crate::identity::{MemberId, ObjectId, ProjectId};
use crate::invite::{join_mac, welcome_confirmation, InviteLink};
use crate::keys::{head, object_key, project_key, EpochMember, EpochRecord};
use crate::recovery::{RecoveryKey, RecoveryWrap};
use crate::roster::{Genesis, TeamFingerprint};

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vectors/team-crypto-v1.json");

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Vectors {
    about: String,
    suite: String,
    recovery: RecoveryVector,
    fingerprint: FingerprintVector,
    head: HeadVector,
    keys: KeysVector,
    object: ObjectVector,
    history: HistoryVector,
    invitation: InvitationVector,
    recovery_wrap: WrapVector,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct RecoveryVector {
    entropy: String,
    words: String,
    signature_key: String,
    hpke_key: String,
    identity: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct FingerprintVector {
    group_id: String,
    genesis: Genesis,
    fingerprint: String,
    words: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct HeadVector {
    group_id: String,
    epoch: u64,
    epoch_authenticator: String,
    head: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct KeysVector {
    content_secret: String,
    project: ProjectId,
    object: ObjectId,
    project_key: String,
    object_key: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct ObjectVector {
    record: EpochRecord,
    signing_seed: String,
    nonce: String,
    plaintext: String,
    sealed: SealedObject,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct HistoryVector {
    group_id: String,
    current: EpochRecord,
    previous: EpochRecord,
    link: HistoryLink,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct InvitationVector {
    server: String,
    id: String,
    secret: String,
    team: String,
    url: String,
    key_package: String,
    join_mac: String,
    group_id: String,
    epoch: u64,
    epoch_authenticator: String,
    confirmation: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct WrapVector {
    group_id: String,
    group_info: String,
    record: EpochRecord,
    wrap: RecoveryWrap,
}

fn seq<const N: usize>(start: u8) -> [u8; N] {
    std::array::from_fn(|i| start.wrapping_add(i as u8))
}

fn h(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(bytes)
}

/// Everything but the HPKE wrap, made again from the fixed inputs.
fn make(wrap: Option<RecoveryWrap>) -> Vectors {
    let provider = OpenMlsRustCrypto::default();
    let crypto = provider.crypto();

    let entropy: [u8; 32] = seq(0);
    let recovery = RecoveryKey::from_entropy(entropy);
    let recovery_public = recovery.public(crypto).unwrap();

    let group_id: [u8; 32] = seq(0xa0);
    let genesis = Genesis {
        owner: MemberId([1; 16]),
        owner_key: vec![0x11; 32],
        created_at: 1_800_000_000,
    };
    let fingerprint = TeamFingerprint::of(&group_id, &genesis);

    let epoch_authenticator = [0x42u8; 32];
    let head7 = head(&group_id, 7, &epoch_authenticator);

    let content_secret: [u8; 32] = seq(0x30);
    let project = ProjectId([2; 16]);
    let object = ObjectId([3; 16]);
    let pkey = project_key(&content_secret, &project);

    let seed = [9u8; 32];
    let public = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes()
        .to_vec();
    let signer = SignatureKeyPair::from_raw(
        crate::CIPHERSUITE.signature_algorithm(),
        seed.to_vec(),
        public.clone(),
    );
    let record = EpochRecord {
        epoch: 7,
        content_secret,
        history_key: [5; 32],
        head: head7,
        members: vec![EpochMember {
            identity: vec![0x77; 37],
            signature_key: public.clone(),
        }],
    };
    let nonce = [0x24u8; 24];
    let plaintext = "Ticket ESC-1 : le Kanban de l'équipe";
    let header = ObjectHeader {
        project,
        object,
        version: 1,
        epoch: 7,
        head: head7,
        author: public,
    };
    let sealed = SealedObject::seal(&record, header, nonce, plaintext.as_bytes(), |b| {
        signer.sign(b).map_err(crate::mls)
    })
    .unwrap();

    let current = EpochRecord {
        epoch: 8,
        content_secret: seq(0x50),
        history_key: [6; 32],
        head: head(&group_id, 8, &epoch_authenticator),
        members: Vec::new(),
    };
    let link = HistoryLink::make(&group_id, &current, &record, [0x18; 24]).unwrap();

    let invite = InviteLink {
        server: "team.example.org".into(),
        id: [0x1d; 16],
        secret: [0x5e; 32],
        team: fingerprint.0,
    };
    let key_package = [0x6b; 40];

    let group_info = b"a GroupInfo, as the committer exports it".to_vec();
    let wrap = wrap.unwrap_or_else(|| {
        RecoveryWrap::seal(
            crypto,
            &recovery_public,
            &group_id,
            &record,
            group_info.clone(),
        )
        .unwrap()
    });

    Vectors {
        about: "Escouade team server, mock-up TS-0a: vectors of the layers above MLS (crates/escouade-team-crypto)"
            .into(),
        suite: "MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519 (0x0003)".into(),
        recovery: RecoveryVector {
            entropy: h(entropy),
            words: recovery.words().join(" "),
            signature_key: h(&recovery_public.signature_key),
            hpke_key: h(&recovery_public.hpke_key),
            identity: h(recovery.identity().to_bytes()),
        },
        fingerprint: FingerprintVector {
            group_id: h(group_id),
            genesis,
            fingerprint: h(fingerprint.0),
            words: fingerprint.words().join(" "),
        },
        head: HeadVector {
            group_id: h(group_id),
            epoch: 7,
            epoch_authenticator: h(epoch_authenticator),
            head: h(head7),
        },
        keys: KeysVector {
            content_secret: h(content_secret),
            project,
            object,
            project_key: h(pkey),
            object_key: h(object_key(&pkey, &object)),
        },
        object: ObjectVector {
            record: record.clone(),
            signing_seed: h(seed),
            nonce: h(nonce),
            plaintext: plaintext.into(),
            sealed,
        },
        history: HistoryVector {
            group_id: h(group_id),
            current,
            previous: record.clone(),
            link,
        },
        invitation: InvitationVector {
            server: invite.server.clone(),
            id: h(invite.id),
            secret: h(invite.secret),
            team: h(invite.team),
            url: invite.to_url(),
            key_package: h(key_package),
            join_mac: h(join_mac(&invite.secret, &invite.id, &key_package)),
            group_id: h(group_id),
            epoch: 1,
            epoch_authenticator: h(epoch_authenticator),
            confirmation: h(welcome_confirmation(&invite.secret, &group_id, 1, &epoch_authenticator)),
        },
        recovery_wrap: WrapVector {
            group_id: h(group_id),
            group_info: h(&group_info),
            record,
            wrap,
        },
    }
}

#[test]
fn the_vectors_hold() {
    if std::env::var_os("ESCOUADE_WRITE_VECTORS").is_some() {
        let vectors = make(None);
        let text = serde_json::to_string_pretty(&vectors).unwrap() + "\n";
        std::fs::create_dir_all(std::path::Path::new(PATH).parent().unwrap()).unwrap();
        std::fs::write(PATH, text).unwrap();
    }
    let text = std::fs::read_to_string(PATH).expect("vectors/team-crypto-v1.json");
    let stored: Vectors = serde_json::from_str(&text).unwrap();

    // Every deterministic output, made again from the inputs.
    let made = make(Some(stored.recovery_wrap.wrap.clone()));
    assert_eq!(made, stored);

    // The randomised one: the recovery key of the vector opens the wrap.
    let provider = OpenMlsRustCrypto::default();
    let entropy: [u8; 32] = hex::decode(&stored.recovery.entropy)
        .unwrap()
        .try_into()
        .unwrap();
    let key = RecoveryKey::from_words(&stored.recovery.words).unwrap();
    assert_eq!(key.words(), RecoveryKey::from_entropy(entropy).words());
    let group_id = hex::decode(&stored.recovery_wrap.group_id).unwrap();
    let (record, group_info) = stored
        .recovery_wrap
        .wrap
        .open(provider.crypto(), &key, &group_id)
        .unwrap();
    assert_eq!(record, stored.recovery_wrap.record);
    assert_eq!(h(group_info), stored.recovery_wrap.group_info);

    // And what each vector's outputs open.
    assert_eq!(
        stored.object.sealed.open(&stored.object.record).unwrap(),
        stored.object.plaintext.as_bytes()
    );
    let group_id = hex::decode(&stored.history.group_id).unwrap();
    assert_eq!(
        stored
            .history
            .link
            .open(&group_id, &stored.history.current)
            .unwrap(),
        stored.history.previous
    );
    let parsed = InviteLink::parse(&stored.invitation.url).unwrap();
    assert_eq!(h(parsed.secret), stored.invitation.secret);
    assert_eq!(h(parsed.team), stored.fingerprint.fingerprint);
}
