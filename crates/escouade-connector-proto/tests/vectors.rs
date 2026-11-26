//! The vectors of the protocol: each frame and each payload of the handshake, byte for byte, as `src/frame.rs` and
//! `src/version.rs` write them. A peer of another version, or another implementation, reads these; changing one is
//! changing the protocol (a new kind of frame, never a new meaning for an old one). The handshake's vectors are its
//! sizes and its checks, with keys made in the test: no key is written anywhere.

use escouade_connector_proto::frame::DecodeError;
use escouade_connector_proto::noise::{HandshakeError, Initiator, Responder, PATTERN, PROLOGUE};
use escouade_connector_proto::version::{Hello, Welcome};
use escouade_connector_proto::*;

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

fn vectors() -> Vec<(Frame, Vec<u8>)> {
    vec![
        (
            Frame::Open(Open {
                id: 1,
                host: "gitlab.acme.internal".into(),
                port: 22,
                agent: "a3".into(),
                project: "Démo".into(),
            }),
            cat(&[
                &[0x01, 0, 0, 0, 1, 0x00, 0x16, 20],
                b"gitlab.acme.internal",
                &[2],
                b"a3",
                &[5],
                "Démo".as_bytes(),
            ]),
        ),
        (Frame::Opened { id: 258 }, vec![0x02, 0, 0, 1, 2]),
        (
            Frame::Refused {
                id: 3,
                refusal: Refusal::new(RefusalCode::NotAllowed, "no"),
            },
            vec![0x03, 0, 0, 0, 3, 1, 0, 2, b'n', b'o'],
        ),
        (
            Frame::Refused {
                id: 3,
                refusal: Refusal::new(RefusalCode::Limits, ""),
            },
            vec![0x03, 0, 0, 0, 3, 4, 0, 0],
        ),
        (
            Frame::Data {
                id: 0x0a0b0c0d,
                bytes: b"SSH-2.0".to_vec(),
            },
            cat(&[&[0x04, 0x0a, 0x0b, 0x0c, 0x0d], b"SSH-2.0"]),
        ),
        (
            Frame::Data {
                id: 1,
                bytes: vec![],
            },
            vec![0x04, 0, 0, 0, 1],
        ),
        (Frame::Close { id: 1 }, vec![0x05, 0, 0, 0, 1]),
        (
            Frame::Window {
                id: 1,
                increment: 65536,
            },
            vec![0x06, 0, 0, 0, 1, 0, 1, 0, 0],
        ),
        (
            Frame::Ping {
                nonce: 0x0102030405060708,
            },
            vec![0x07, 1, 2, 3, 4, 5, 6, 7, 8],
        ),
        (Frame::Pong { nonce: 1 }, vec![0x08, 0, 0, 0, 0, 0, 0, 0, 1]),
        (
            Frame::Ceiling(Ceiling {
                rules: vec!["a:22".into(), "*.b:443".into()],
            }),
            cat(&[&[0x09, 0, 2, 0, 4], b"a:22", &[0, 7], b"*.b:443"]),
        ),
        (Frame::Ceiling(Ceiling::default()), vec![0x09, 0, 0]),
    ]
}

#[test]
fn the_frames_are_these_bytes() {
    for (frame, bytes) in vectors() {
        assert_eq!(frame.encode().unwrap(), bytes, "{frame:?}");
        assert_eq!(Frame::decode(&bytes), Ok(frame));
    }
}

#[test]
fn a_kind_of_a_later_minor_is_known_as_such() {
    assert_eq!(Frame::decode(&[0x0a]), Err(DecodeError::UnknownKind(0x0a)));
    assert_eq!(
        Frame::decode(&[0xff, 1, 2]),
        Err(DecodeError::UnknownKind(0xff))
    );
}

#[test]
fn the_payloads_of_the_handshake_are_these_bytes() {
    let hello = Hello {
        version: Version::new(2, 3, 1),
    };
    assert_eq!(hello.encode(), [0, 2, 0, 3, 0, 1]);
    let welcome = Welcome::for_hello(Version::new(2, 5, 0), &hello);
    // 2.5.0, to update (two minors before), the oldest accepted 2.3.
    assert_eq!(welcome.encode(), [0, 2, 0, 5, 0, 0, 1, 0, 2, 0, 3]);
    let too_old = Welcome::for_hello(
        Version::new(2, 5, 0),
        &Hello {
            version: Version::new(2, 2, 0),
        },
    );
    assert_eq!(too_old.encode(), [0, 2, 0, 5, 0, 0, 2, 0, 2, 0, 3]);
}

#[test]
fn the_handshake_is_noise_ik_with_keys_made_here() {
    assert_eq!(PATTERN, "Noise_IK_25519_ChaChaPoly_BLAKE2s");
    assert_eq!(PROLOGUE, b"escouade-connector-proto/1");
    let server = Keypair::generate();
    let connector = Keypair::generate();
    let hello = Hello {
        version: Version::new(2, 3, 0),
    };
    let (initiator, first) = Initiator::start(&connector, &server.public(), &hello).unwrap();
    // → e (32), s encrypted (32 + 16), the hello encrypted (6 + 16).
    assert_eq!(first.len(), 32 + 48 + 22);
    let responder = Responder::read(&server, &first).unwrap();
    assert_eq!(responder.remote(), connector.public());
    let (second, mut at_server) = responder
        .respond(&Welcome::for_hello(Version::new(2, 3, 0), &hello))
        .unwrap();
    // ← e (32), the welcome encrypted (11 + 16).
    assert_eq!(second.len(), 32 + 27);
    let (_, mut at_connector) = initiator.finish(&second).unwrap();
    // A frame sealed: its bytes and a tag of 16.
    let ping = Frame::Ping { nonce: 1 }.encode().unwrap();
    let sealed = at_connector.seal(&ping).unwrap();
    assert_eq!(sealed.len(), ping.len() + 16);
    assert_eq!(at_server.open(&sealed).unwrap(), ping);

    // Each first message is new (a new ephemeral key): one replayed is a new handshake that the replayer cannot
    // finish, without the connector's ephemeral key.
    let (_, again) = Initiator::start(&connector, &server.public(), &hello).unwrap();
    assert_ne!(again, first);
    // A byte changed anywhere in the first message: it does not open.
    for i in [0, 40, first.len() - 1] {
        let mut bent = first.clone();
        bent[i] ^= 0x80;
        assert_eq!(
            Responder::read(&server, &bent).err(),
            Some(HandshakeError::NotNoise),
            "byte {i}"
        );
    }
}

#[test]
fn the_crate_depends_on_nothing_of_the_core_nor_of_tauri() {
    let manifest = include_str!("../Cargo.toml");
    let deps: String = manifest
        .split("\n[")
        .filter(|section| {
            section.starts_with("dependencies]") || section.starts_with("dev-dependencies]")
        })
        .collect();
    for line in deps.lines().filter(|l| !l.trim_start().starts_with('#')) {
        let name = line.split('=').next().unwrap_or("").trim();
        assert!(
            !name.starts_with("escouade") && !name.contains("tauri") && !line.contains("path ="),
            "{line}"
        );
    }
}
