//! The bodies of the protocol's fuzzing targets (the package `fuzz/` at the root, `connector-frames` and
//! `connector-handshake`), and a test that plays them on their seeds in every run.
//!
//! - [`frames`]: whatever the peer sends once decrypted. A frame decoded has one encoding alone, the one it came in;
//!   the handshake's payloads decoded encode to what decodes the same.
//! - [`handshake`]: whatever reaches either side of the handshake. By its first byte: garbage as the connector's first
//!   message (the server never fails, and answers only what it read); garbage as the server's answer (never taken for
//!   the server's); a real first message with any payload (read as a hello exactly when it is one); garbage after a
//!   handshake done (never opened).

use crate::frame::Frame;
use crate::noise::{self, Initiator, Keypair, Responder};
use crate::version::{Hello, Version, Welcome};

/// The names of the targets, as `fuzz/Cargo.toml` and `fuzz/seeds/` call them.
pub const TARGETS: [&str; 2] = ["connector-frames", "connector-handshake"];

pub fn frames(data: &[u8]) {
    if let Ok(frame) = Frame::decode(data) {
        let again = frame.encode().expect("a frame decoded encodes");
        assert_eq!(again, data, "one encoding per frame: {frame:?}");
    }
    if let Ok(hello) = Hello::decode(data) {
        assert_eq!(Hello::decode(&hello.encode()), Ok(hello));
    }
    if let Ok(welcome) = Welcome::decode(data) {
        assert_eq!(Welcome::decode(&welcome.encode()), Ok(welcome));
    }
}

const SERVER: Version = Version::new(2, 3, 0);

fn keys() -> (Keypair, Keypair) {
    (
        Keypair::from_private([0x11; 32]),
        Keypair::from_private([0x22; 32]),
    )
}

pub fn handshake(data: &[u8]) {
    let Some((&mode, rest)) = data.split_first() else {
        return;
    };
    let (server, connector) = keys();
    let hello = Hello { version: SERVER };
    match mode % 4 {
        0 => {
            if let Ok(responder) = Responder::read(&server, rest) {
                let welcome = Welcome::for_hello(SERVER, &responder.hello());
                responder
                    .respond(&welcome)
                    .expect("an answer to what was read");
            }
        }
        1 => {
            let (initiator, _) =
                Initiator::start(&connector, &server.public(), &hello).expect("a first message");
            assert!(
                initiator.finish(rest).is_err(),
                "an answer made without the server's key"
            );
        }
        2 => {
            let server_key = server.public();
            let mut state = snow::Builder::new(noise::params())
                .local_private_key(connector.private_bytes())
                .and_then(|b| b.remote_public_key(&server_key.0))
                .and_then(|b| b.prologue(noise::PROLOGUE))
                .and_then(|b| b.build_initiator())
                .expect("an initiator");
            let mut first = vec![0u8; noise::MAX_MESSAGE];
            let Ok(n) = state.write_message(rest, &mut first) else {
                return;
            };
            let read = Responder::read(&server, &first[..n]);
            assert_eq!(
                read.is_ok(),
                Hello::decode(rest).is_ok(),
                "the hello read exactly"
            );
            if let Ok(responder) = read {
                assert_eq!(responder.remote(), connector.public());
            }
        }
        _ => {
            let (initiator, first) =
                Initiator::start(&connector, &server.public(), &hello).expect("a first message");
            let responder = Responder::read(&server, &first).expect("read");
            let (second, mut at_server) = responder
                .respond(&Welcome::for_hello(SERVER, &hello))
                .expect("an answer");
            let (_, mut at_connector) = initiator.finish(&second).expect("the answer read");
            assert!(at_server.open(rest).is_err(), "a message sealed by no one");
            let sealed = at_connector.seal(rest).ok();
            if let Some(sealed) = sealed {
                assert_eq!(at_server.open(&sealed).expect("opened"), rest);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::frame::{Ceiling, Frame, Open, Refusal, RefusalCode};
    use crate::version::{Hello, Version, Welcome};

    fn seeds() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds")
    }

    /// The frames of the vectors, as seeds of `connector-frames`.
    fn frame_seeds() -> Vec<(&'static str, Vec<u8>)> {
        let f = |name, frame: Frame| (name, frame.encode().unwrap());
        vec![
            f(
                "open",
                Frame::Open(Open {
                    id: 1,
                    host: "gitlab.acme.internal".into(),
                    port: 22,
                    agent: "agent".into(),
                    project: "Démo".into(),
                }),
            ),
            f("opened", Frame::Opened { id: 1 }),
            f(
                "refused",
                Frame::Refused {
                    id: 1,
                    refusal: Refusal::new(RefusalCode::NotAllowed, "not in the ceiling"),
                },
            ),
            f(
                "data",
                Frame::Data {
                    id: 1,
                    bytes: b"SSH-2.0-OpenSSH_9.6\r\n".to_vec(),
                },
            ),
            f(
                "fin",
                Frame::Data {
                    id: 1,
                    bytes: vec![],
                },
            ),
            f("close", Frame::Close { id: 1 }),
            f(
                "window",
                Frame::Window {
                    id: 1,
                    increment: 65536,
                },
            ),
            f("ping", Frame::Ping { nonce: 7 }),
            f(
                "ceiling",
                Frame::Ceiling(Ceiling {
                    rules: vec![
                        "gitlab.acme.internal:22".into(),
                        "*.npm.acme.internal:443".into(),
                    ],
                }),
            ),
            (
                "hello",
                Hello {
                    version: Version::new(2, 3, 1),
                }
                .encode(),
            ),
            (
                "welcome",
                Welcome::for_hello(
                    Version::new(2, 5, 0),
                    &Hello {
                        version: Version::new(2, 3, 1),
                    },
                )
                .encode(),
            ),
        ]
    }

    fn handshake_seeds() -> Vec<(&'static str, Vec<u8>)> {
        let hello = Hello {
            version: Version::new(2, 3, 1),
        }
        .encode();
        vec![
            ("garbage-first", [&[0u8][..], &[7; 102]].concat()),
            ("garbage-answer", [&[1u8][..], &[9; 59]].concat()),
            ("payload-hello", [&[2u8][..], &hello].concat()),
            ("payload-short", vec![2, 0, 2]),
            ("transport", [&[3u8][..], &[5; 40]].concat()),
        ]
    }

    #[test]
    fn the_targets_hold_on_their_seeds() {
        let root = seeds();
        // `ESCOUADE_WRITE_SEEDS=1` writes them again.
        if std::env::var("ESCOUADE_WRITE_SEEDS").as_deref() == Ok("1") {
            for (target, list) in [
                ("connector-frames", frame_seeds()),
                ("connector-handshake", handshake_seeds()),
            ] {
                let dir = root.join(target);
                std::fs::create_dir_all(&dir).unwrap();
                for (name, bytes) in list {
                    std::fs::write(dir.join(name), bytes).unwrap();
                }
            }
        }
        for (name, bytes) in frame_seeds() {
            super::frames(&bytes);
            assert!(Frame::decode(&bytes).is_ok() || name == "hello" || name == "welcome");
        }
        for (_, bytes) in handshake_seeds() {
            super::handshake(&bytes);
        }
        // Some noise, every mode.
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..200 {
            let len = (next() % 120) as usize;
            let data: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            super::frames(&data);
            super::handshake(&data);
        }
        // What is committed in fuzz/seeds/, findings included.
        for target in super::TARGETS {
            let dir = root.join(target);
            let entries = std::fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("the seeds of {target} ({}): {e}", dir.display()));
            let mut played = 0;
            for entry in entries {
                let bytes = std::fs::read(entry.unwrap().path()).unwrap();
                match target {
                    "connector-frames" => super::frames(&bytes),
                    _ => super::handshake(&bytes),
                }
                played += 1;
            }
            assert!(played > 0, "no seed for {target}");
        }
    }
}
