//! AU-2: the server emits its own audit (`docs/TEAM-AUDIT.md` §3.1, §4.4, §5.2), against the real server, on SQLite
//! then on PostgreSQL: each route its event, in opaque ids, with the anchor of each entry; nothing of the content nor
//! of a secret in all that goes out; the key drawn and kept; `--audit-ip`; syslog over TLS; the buffer and its taking
//! up after a stop; `escouade-audit verify` whole on the output.

#[macro_use]
mod common;

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use common::*;
use escouade_audit::verify::{self, Keys, Place};
use escouade_audit::{Digest, Record};
use escouade_team_server::api::wire::*;
use escouade_team_server::audit::{self, IpMode};
use escouade_team_server::config::Config;
use escouade_team_server::db::Kind;
use escouade_team_server::util::{random, token_hash};
use reqwest::StatusCode;
use serde_json::Value;

both!(
    a_whole_journey_goes_out_in_opaque_ids_without_content_nor_secret,
    the_address_goes_out_whole_truncated_or_not_at_all,
    the_rates_go_out_once_a_minute_with_their_count,
);

/// What the tests trap in the content: none of it may go out.
const TITLE: &str = "Rachat de la société Exemple";
const SECRET: &str = "ghp_0123456789abcdefghijABCDEFGHIJ012345";

fn payload(n: u8) -> Vec<u8> {
    format!("{TITLE} {SECRET} entrée {n}").into_bytes()
}

/// The audit to a file in the test's data folder.
fn to_file(c: &mut Config) {
    c.audit.file = Some(c.data.join("out").join("audit.jsonl"));
}

fn out_file(s: &TestServer) -> PathBuf {
    s.dir.path().join("out").join("audit.jsonl")
}

/// The lines the server wrote, every event handed so far sent.
fn lines(s: &TestServer) -> Vec<String> {
    s.server.app().audit.flush();
    std::fs::read_to_string(out_file(s))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn records(lines: &[String]) -> Vec<Value> {
    lines
        .iter()
        .map(|l| {
            Record::from_line(l).unwrap_or_else(|e| panic!("{e}: {l}"));
            serde_json::from_str(l).unwrap()
        })
        .collect()
}

fn named<'a>(records: &'a [Value], event: &str) -> Vec<&'a Value> {
    records.iter().filter(|r| r["event"] == event).collect()
}

fn path(p: &[u8], rest: &str) -> String {
    format!("/v1/projects/{}{rest}", hex::encode(p))
}

async fn take(d: &Device, p: &[u8; 16], t: &[u8; 16]) -> Answer<ClaimAnswer> {
    d.client
        .post(
            &path(p, &format!("/claims/{}", hex::encode(t))),
            &ClaimRequest { expected: None },
        )
        .await
}

async fn ask<B: serde::Serialize>(
    d: &Device,
    p: &[u8; 16],
    t: &[u8; 16],
    what: &str,
    body: &B,
) -> Answer<ClaimAnswer> {
    d.client
        .post(
            &path(p, &format!("/claims/{}/{what}", hex::encode(t))),
            body,
        )
        .await
}

async fn append(d: &Device, stream: &[u8], req: AppendRequest) -> Answer<Appended> {
    d.client.post(&path(stream, "/stream"), &req).await
}

fn message(payload: Vec<u8>, fence: Option<FenceWire>) -> AppendRequest {
    AppendRequest {
        kind: Kind::Message,
        epoch: None,
        payload,
        fence,
        own: false,
    }
}

fn id(d: &Device) -> [u8; 16] {
    d.id.clone().try_into().unwrap()
}

/// Every needle, as text, in hexadecimal and in base64 (standard and url-safe), is in none of the lines.
fn nothing_of(all: &str, needles: &[(&str, Vec<u8>)]) {
    for (what, bytes) in needles {
        let mut forms = vec![
            hex::encode(bytes),
            base64::engine::general_purpose::STANDARD.encode(bytes),
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes),
        ];
        if let Ok(text) = std::str::from_utf8(bytes) {
            forms.push(text.to_owned());
        }
        for form in forms.into_iter().filter(|f| f.len() >= 8) {
            assert!(!all.contains(&form), "{what} went out: {form}");
        }
    }
}

/// The whole journey of a team (its creation, an invitation, a project, its tickets, a passage, its documents, the
/// server's refusals): each route says its event, and nothing of the content nor of a secret goes out.
async fn a_whole_journey_goes_out_in_opaque_ids_without_content_nor_secret(b: Backend) {
    let s = TestServer::start_with(&b, |c| {
        to_file(c);
        c.limits.claim_lease = Duration::from_secs(1);
        c.limits.max_entry = 4096;
    })
    .await;
    let team_token = s.team_token().await;
    let mut alice = Device::enroll(&s, &team_token).await.unwrap();
    alice.sign_in().await.unwrap();
    // A second session of the same device from the same address: not said again.
    alice.sign_in().await.unwrap();
    let team: [u8; 16] = alice.team.clone().try_into().unwrap();

    // The invitation: a token made by the client, served once, then a second time (a leaked link).
    let invite = format!("invitation-{}", hex::encode(random::<16>()));
    let _: Registered = alice
        .client
        .post(
            "/v1/devices/enrollments",
            &EnrollmentRequest {
                expires_in: None,
                token_hash: Some(token_hash(&invite).to_vec()),
            },
        )
        .await
        .ok();
    let mut bob = Device::enroll(&s, &invite).await.unwrap();
    bob.sign_in().await.unwrap();
    let reused = Device::enroll(&s, &invite).await.err().unwrap();
    assert_eq!(reused.1.error, "token_used");
    let carol = s.add_device(&alice).await;

    // The team's stream: a commit and a message; one of the team's own streams; a project, its commit, its
    // operations (not said), its access.
    assert!(matches!(alice.commit(0, &payload(1)).await, Answer::Ok(_)));
    let message_seq = alice.message(&payload(2)).await;
    append(&alice, b"escouade/setting", message(payload(3), None))
        .await
        .ok();
    let project = random::<16>();
    append(&alice, &project, message(payload(4), None))
        .await
        .ok();
    let commit = append(
        &alice,
        &project,
        AppendRequest {
            kind: Kind::Commit,
            epoch: Some(0),
            ..message(payload(5), None)
        },
    )
    .await
    .ok();
    let access = AccessWire {
        everyone: false,
        devices: vec![
            serde_bytes::ByteBuf::from(alice.id.clone()),
            serde_bytes::ByteBuf::from(bob.id.clone()),
        ],
    };
    let r = alice
        .client
        .request(reqwest::Method::PUT, &path(&project, "/access"))
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(&access))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);

    // The tickets: taken, renewed (not said), written under its token, given back; taken by Bob, handed to Alice,
    // taken by her; handed back and withdrawn; a lease that ends; a write under a stale token.
    let t = random::<16>();
    let a = take(&alice, &project, &t).await.ok();
    for _ in 0..3 {
        ask(
            &alice,
            &project,
            &t,
            "renew",
            &TokenRequest {
                token: a.claim.token,
            },
        )
        .await
        .ok();
    }
    let fence = Some(FenceWire {
        ticket: t.to_vec(),
        token: a.claim.token,
    });
    append(&alice, &project, message(payload(6), fence))
        .await
        .ok();
    ask(
        &alice,
        &project,
        &t,
        "release",
        &TokenRequest {
            token: a.claim.token,
        },
    )
    .await
    .ok();
    let held = take(&bob, &project, &t).await.ok();
    let handed = ask(
        &bob,
        &project,
        &t,
        "hand",
        &HandRequest {
            token: held.claim.token,
            to: alice.id.clone(),
        },
    )
    .await
    .ok();
    let taken = take(&alice, &project, &t).await.ok();
    assert_eq!(taken.claim.holder, Some(id(&alice)));
    let again = ask(
        &alice,
        &project,
        &t,
        "hand",
        &HandRequest {
            token: taken.claim.token,
            to: bob.id.clone(),
        },
    )
    .await
    .ok();
    ask(
        &alice,
        &project,
        &t,
        "withdraw",
        &TokenRequest {
            token: again.claim.token,
        },
    )
    .await
    .ok();
    let _ = handed;
    let t2 = random::<16>();
    let old = take(&alice, &project, &t2).await.ok();
    tokio::time::sleep(Duration::from_millis(2100)).await;
    take(&bob, &project, &t2).await.ok();
    let stale = append(
        &alice,
        &project,
        message(
            payload(7),
            Some(FenceWire {
                ticket: t2.to_vec(),
                token: old.claim.token,
            }),
        ),
    )
    .await
    .refused();
    assert_eq!(stale.1.error, "stale_token");

    // The documents: a file and a snapshot (not said: their bytes are content).
    let file = random::<16>();
    assert_eq!(
        alice
            .file_in(reqwest::Method::PUT, &project, &file, Some(payload(8)))
            .await
            .0,
        StatusCode::CREATED
    );
    let snap = alice
        .client
        .request(reqwest::Method::PUT, &path(&project, "/snapshot"))
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(
            &SnapshotPut {
                upto: 2,
                payload: payload(9),
            },
        ))
        .send()
        .await
        .unwrap();
    assert!(snap.status().is_success(), "{}", snap.status());

    // The roles, a revocation, the sign-ins refused (revoked, unknown, a bad signature).
    let role = |d: &Device, r| {
        let c = alice.client.clone();
        let path = format!("/v1/devices/{}/role", d.id_hex());
        async move {
            let r = c
                .request(reqwest::Method::POST, &path)
                .header("content-type", "application/cbor")
                .body(escouade_team_server::transport::cbor::encode(
                    &RoleRequest { role: r },
                ))
                .send()
                .await
                .unwrap();
            assert_eq!(r.status(), StatusCode::NO_CONTENT);
        }
    };
    role(&bob, escouade_team_server::db::Role::Admin).await;
    role(&bob, escouade_team_server::db::Role::Member).await;
    assert_eq!(alice.revoke(&carol).await, StatusCode::NO_CONTENT);
    let mut carol = carol;
    assert!(carol.sign_in().await.is_err());
    let stranger = Device {
        key: ed25519_dalek::SigningKey::from_bytes(&random::<32>()),
        id: random::<16>().to_vec(),
        team: team.to_vec(),
        client: s.client(),
    };
    assert!(matches!(stranger.challenge().await, Answer::No(..)));
    let c = match bob.challenge().await {
        Answer::Ok(c) => c.challenge,
        Answer::No(s, p) => panic!("{s} {p:?}"),
    };
    assert!(matches!(bob.session(&c, vec![0; 64]).await, Answer::No(..)));

    // The refusals before a route: a client too old, a body too large.
    let old_client = reqwest::Client::new()
        .post(format!("{}/v1/auth/challenge", s.base))
        .send()
        .await
        .unwrap();
    assert_eq!(old_client.status(), StatusCode::UPGRADE_REQUIRED);
    let big = alice
        .append_team(Kind::Message, None, &vec![1; 5000])
        .await
        .refused();
    assert_eq!(big.0, StatusCode::PAYLOAD_TOO_LARGE);

    // The recovery token, served.
    let recovery = s.server.new_recovery_token(team).await.unwrap();
    let rescued = Device::enroll(&s, &recovery).await.unwrap();

    let lines = lines(&s);
    let all = lines.join("\n");
    let records = records(&lines);
    let count = |e: &str| named(&records, e).len();

    // Each route its event.
    assert_eq!(count("device.enrolled"), 4, "{all}");
    assert_eq!(
        count("session.opened"),
        3,
        "alice, bob and carol, once each: {all}"
    );
    assert_eq!(count("invitation.reused"), 1);
    assert_eq!(count("recovery.used"), 1);
    assert_eq!(count("project.access.changed"), 1);
    assert_eq!(count("claim.taken"), 5, "never the renewals: {all}");
    assert_eq!(count("claim.released"), 1);
    assert_eq!(count("claim.expired"), 1);
    assert_eq!(count("claim.fenced"), 1);
    assert_eq!(count("handoff.offered"), 2);
    assert_eq!(count("handoff.taken"), 1);
    assert_eq!(count("handoff.withdrawn"), 1);
    assert_eq!(count("role.changed"), 2);
    assert_eq!(count("device.revoked"), 1);
    assert_eq!(count("session.refused"), 3, "{all}");
    assert_eq!(count("client.too_old"), 1);
    assert_eq!(count("request.too_large"), 1);
    // The entries: the team's commit and message, the setting, the project's commit; never its operations.
    assert_eq!(count("stream.written"), 4, "{all}");

    // In opaque ids: every one a device's, a team's, a project's, a ticket's or a token's, in hexadecimal.
    let tokens: Vec<String> = named(&records, "device.enrolled")
        .iter()
        .map(|r| r["detail"]["token"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(tokens, ["init", "invitation", "invitation", "recovery"]);
    let first = &named(&records, "device.enrolled")[0];
    assert_eq!(first["team"]["id"], hex::encode(team));
    assert_eq!(first["target"]["device"], alice.id_hex());
    assert_eq!(first["actor"]["ip"], "127.0.0.1");
    let unknown = named(&records, "session.refused")
        .into_iter()
        .find(|r| r["detail"]["cause"] == "unknown")
        .expect("the stranger");
    assert_eq!(unknown["team"]["id"], audit::NO_TEAM);
    let reuse = &named(&records, "invitation.reused")[0];
    assert_eq!(
        reuse["target"]["invitation"],
        audit::token_id(&token_hash(&invite))
    );
    let fenced = &named(&records, "claim.fenced")[0];
    assert_eq!(fenced["target"]["item"], hex::encode(t2));
    assert_eq!(fenced["target"]["project"], hex::encode(project));
    assert_eq!(
        (
            fenced["detail"]["token"].as_u64(),
            fenced["detail"]["current"].as_u64()
        ),
        (Some(old.claim.token), Some(old.claim.token + 1))
    );
    let expired = &named(&records, "claim.expired")[0];
    assert_eq!(expired["actor"]["device"], alice.id_hex());

    // The anchor: the stream, the number, the epoch, the SHA-256 of the encrypted bytes, as the server serves them.
    let anchors: BTreeMap<(String, u64), &Value> = named(&records, "stream.written")
        .into_iter()
        .map(|r| {
            assert_eq!(r["target"]["stream"], r["anchor"]["stream"]);
            (
                (
                    r["anchor"]["stream"].as_str().unwrap().to_owned(),
                    r["anchor"]["n"].as_u64().unwrap(),
                ),
                &r["anchor"],
            )
        })
        .collect();
    let team_entries = alice.read_team(0).await.entries;
    assert_eq!(team_entries.len(), 2);
    for (e, epoch) in team_entries.iter().zip([0, 1]) {
        let a = anchors[&("team".to_owned(), e.seq)];
        assert_eq!(a["ct"], Digest::of(&e.payload).as_str());
        // The commit's own epoch, then the one its stream stands at after it.
        assert_eq!(a["epoch"], epoch);
    }
    let message_anchor = anchors[&("team".to_owned(), message_seq)];
    assert_eq!(message_anchor["ct"], Digest::of(&payload(2)).as_str());
    let p = hex::encode(project);
    let pa = anchors[&(p.clone(), commit.seq)];
    assert_eq!(pa["ct"], Digest::of(&payload(5)).as_str());
    assert!(anchors.contains_key(&("escouade.setting".to_owned(), 1)));

    // Nothing of the content nor of a secret: the payloads, the tokens, the session tokens, the keys.
    let mut needles: Vec<(&str, Vec<u8>)> = vec![
        ("a title", TITLE.as_bytes().to_vec()),
        ("a secret in the content", SECRET.as_bytes().to_vec()),
        ("the team's token", team_token.as_bytes().to_vec()),
        ("the team token's hash", token_hash(&team_token).to_vec()),
        ("the invitation's token", invite.as_bytes().to_vec()),
        ("the invitation's hash", token_hash(&invite).to_vec()),
        ("the recovery token", recovery.as_bytes().to_vec()),
        ("the recovery token's hash", token_hash(&recovery).to_vec()),
    ];
    for n in 1..=9 {
        needles.push(("an entry's bytes", payload(n)));
    }
    for d in [&alice, &bob, &rescued] {
        needles.push(("a device's private key", d.key.to_bytes().to_vec()));
        needles.push((
            "a device's public key",
            d.key.verifying_key().to_bytes().to_vec(),
        ));
        if let Some(t) = &d.client.token {
            needles.push(("a session's token", t.as_bytes().to_vec()));
        }
    }
    let (key, _) = audit::ensure_key(s.dir.path()).unwrap();
    needles.push(("the audit's private key", key.to_bytes().to_vec()));
    nothing_of(&all, &needles);
    for r in &records {
        assert_eq!(r["source"], "server");
        assert!(r.get("name").is_none() && r["team"].get("name").is_none());
    }

    // escouade-audit verify: every signature against the server's key, the chain whole.
    let keys = Keys::parse(&audit::keys_json(&key)).unwrap();
    let report = verify::verify(
        lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                (
                    Place {
                        file: "audit.jsonl".into(),
                        line: i + 1,
                    },
                    l.clone(),
                )
            })
            .collect(),
        Some(&keys),
        Vec::new(),
    );
    assert!(report.ok(), "{report}");
    assert_eq!(records[0]["emitter"]["device"], audit::emitter_id(&key));
    s.finish().await;
}

async fn enrolled_ip(b: &Backend, mode: IpMode) -> Option<String> {
    let s = TestServer::start_with(b, |c| {
        to_file(c);
        c.audit.ip = mode;
    })
    .await;
    s.new_team().await;
    let records = records(&lines(&s));
    let ip = named(&records, "device.enrolled")[0]["actor"]["ip"]
        .as_str()
        .map(str::to_owned);
    s.finish().await;
    ip
}

/// A6-A: whole by default; truncated (/24, /48) or removed by `--audit-ip`.
async fn the_address_goes_out_whole_truncated_or_not_at_all(b: Backend) {
    assert_eq!(
        enrolled_ip(&b, IpMode::Full).await.as_deref(),
        Some("127.0.0.1")
    );
    assert_eq!(
        enrolled_ip(&b, IpMode::Truncated).await.as_deref(),
        Some("127.0.0.0")
    );
    assert_eq!(enrolled_ip(&b, IpMode::Off).await, None);
    assert_eq!(
        IpMode::Truncated.apply("2001:db8:1:2:3:4:5:6".parse().unwrap()),
        Some("2001:db8:1::".parse().unwrap())
    );
}

/// A device and an address over their rate: one event the first time, then once a minute with the count since.
async fn the_rates_go_out_once_a_minute_with_their_count(b: Backend) {
    let s = TestServer::start_with(&b, |c| {
        to_file(c);
        c.rates.per_device.burst = 2.0;
        c.rates.per_device.per_second = 0.001;
    })
    .await;
    let alice = s.new_team().await;
    let mut limited = 0;
    for _ in 0..6 {
        if let Answer::No(status, _) = alice.client.get::<Devices>("/v1/devices").await {
            assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
            limited += 1;
        }
    }
    assert!(limited >= 3);
    let records = records(&lines(&s));
    let rated = named(&records, "rate.limited");
    assert_eq!(rated.len(), 1, "once a minute");
    assert_eq!(rated[0]["detail"]["scope"], "device");
    assert_eq!(rated[0]["detail"]["count"], 1);
    assert_eq!(rated[0]["actor"]["device"], alice.id_hex());
    s.finish().await;
}

/// The key is drawn once, kept in the data folder, and the same at each start; it signs nothing but the audit.
#[test]
fn the_audit_key_is_drawn_once_and_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (a, drawn) = audit::ensure_key(dir.path()).unwrap();
    assert!(drawn);
    let (b, again) = audit::ensure_key(dir.path()).unwrap();
    assert!(!again);
    assert_eq!(a.to_bytes(), b.to_bytes());
    let text = std::fs::read_to_string(dir.path().join(audit::KEY_FILE)).unwrap();
    assert_eq!(text.trim().len(), 64);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(dir.path().join(audit::KEY_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    let id = audit::emitter_id(&a);
    assert!(id.starts_with("server-") && id.len() == 23, "{id}");
    assert!(Keys::parse(&audit::keys_json(&a)).is_ok());
}

/// Off by default: a server without an output writes nothing, makes no buffer.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_audit_is_off_by_default() {
    let s = TestServer::start(&Backend::Sqlite).await;
    s.new_team().await;
    assert!(!s.server.app().audit.is_on());
    assert!(!s.dir.path().join("audit").exists());
    s.finish().await;
}

// ---------- syslog over TLS, and the buffer ----------

/// A syslog collector of RFC 5425 (octet counting), in TLS or in clear, that keeps each message.
struct Collector {
    port: u16,
    got: Arc<Mutex<Vec<String>>>,
}

fn read_frames(mut r: impl BufRead, got: &Mutex<Vec<String>>) {
    loop {
        let mut len = Vec::new();
        if r.read_until(b' ', &mut len).unwrap_or(0) == 0 {
            return;
        }
        let Ok(n) = String::from_utf8_lossy(&len).trim().parse::<usize>() else {
            return;
        };
        let mut msg = vec![0; n];
        if r.read_exact(&mut msg).is_err() {
            return;
        }
        got.lock().unwrap().push(String::from_utf8(msg).unwrap());
    }
}

fn tls_material() -> (Vec<u8>, Arc<rustls::ServerConfig>) {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![cert.cert.der().clone()],
        rustls_pki_types::PrivateKeyDer::Pkcs8(cert.signing_key.serialize_der().into()),
    )
    .unwrap();
    (cert.cert.pem().into_bytes(), Arc::new(config))
}

impl Collector {
    fn start(listener: TcpListener, tls: Option<Arc<rustls::ServerConfig>>) -> Collector {
        let port = listener.local_addr().unwrap().port();
        let got = Arc::new(Mutex::new(Vec::new()));
        let kept = got.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let kept = kept.clone();
                let tls = tls.clone();
                std::thread::spawn(move || match tls {
                    None => read_frames(BufReader::new(stream), &kept),
                    Some(config) => {
                        let conn = rustls::ServerConnection::new(config).unwrap();
                        read_frames(
                            BufReader::new(rustls::StreamOwned::new(conn, stream)),
                            &kept,
                        );
                    }
                });
            }
        });
        Collector { port, got }
    }

    fn wait_for(&self, n: usize) -> Vec<String> {
        for _ in 0..1000 {
            if self.got.lock().unwrap().len() >= n {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        self.got.lock().unwrap().clone()
    }
}

/// The JSON record at the end of a syslog message.
fn record_of(message: &str) -> Value {
    let at = message
        .find("] {")
        .expect("structured data, then the record")
        + 2;
    serde_json::from_str(&message[at..]).unwrap()
}

/// syslog over TLS to a local collector, its certificate trusted by `--audit-syslog-ca`; CEF in option.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_audit_goes_to_syslog_over_tls() {
    let (pem, config) = tls_material();
    let collector = Collector::start(TcpListener::bind("127.0.0.1:0").unwrap(), Some(config));
    let ca = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(ca.path(), &pem).unwrap();
    for cef in [false, true] {
        let before = collector.got.lock().unwrap().len();
        let s = TestServer::start_with(&Backend::Sqlite, |c| {
            c.audit.syslog = Some(format!("tls://localhost:{}", collector.port));
            c.audit.syslog_ca = Some(ca.path().to_path_buf());
            c.audit.cef = cef;
        })
        .await;
        let alice = s.new_team().await;
        alice.message(&payload(1)).await;
        s.server.app().audit.flush();
        // Enrolled, signed in, its entry.
        let got = collector.wait_for(before + 3);
        let got = &got[before..];
        assert_eq!(got.len(), 3, "{got:?}");
        assert!(got[0].starts_with("<109>1 "), "{}", got[0]);
        assert!(got.iter().all(|m| m.contains(" escouade - ")));
        assert!(got.iter().all(|m| m.contains("[escouade@32473 ")));
        if cef {
            assert!(got
                .iter()
                .all(|m| m.contains(" CEF:0|Escouade|escouade-team|")));
        } else {
            let written = record_of(&got[2]);
            assert_eq!(written["event"], "stream.written");
            assert_eq!(written["anchor"]["ct"], Digest::of(&payload(1)).as_str());
            Record::from_value(written).unwrap();
        }
        assert!(!got.join("\n").contains(TITLE));
        s.finish().await;
    }
}

/// syslog down: the events wait on the disk; the server stops; started again with the collector back, it sends them
/// first, in order, and its chain goes on where it stopped (no gap, no double): `verify` whole on what came.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_buffer_waits_for_syslog_and_takes_up_after_a_stop() {
    let dir = tempfile::tempdir().unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let start = |data: &Path| {
        let (mut config, _) = common::config(&Backend::Sqlite, data);
        config.audit.syslog = Some(format!("tcp://127.0.0.1:{port}"));
        config
    };
    // Down: nothing listens on the port.
    let server = Arc::new(
        escouade_team_server::Server::open(start(dir.path()))
            .await
            .unwrap(),
    );
    let tmp = tempfile::tempdir().unwrap();
    let s = TestServer::serve(server, tmp, None, None).await;
    let alice = s.new_team().await;
    for n in 0..3 {
        alice.message(&payload(n)).await;
    }
    s.server.app().audit.flush();
    let waiting =
        escouade_audit::Outbox::open(dir.path().join("audit").join("syslog"), Default::default())
            .unwrap()
            .waiting()
            .unwrap();
    assert_eq!(waiting, 5, "enrolled, signed in, three entries");
    // The server stops (as `serve` does at its end): its thread gone, nothing more is sent by it.
    s.server.app().audit.stop();
    s.finish().await;

    // Back: the collector listens; the server starts again on the same folder.
    let collector = Collector::start(TcpListener::bind(("127.0.0.1", port)).unwrap(), None);
    let server = Arc::new(
        escouade_team_server::Server::open(start(dir.path()))
            .await
            .unwrap(),
    );
    let s = TestServer::serve(server, tempfile::tempdir().unwrap(), None, None).await;
    let mut alice = alice;
    alice.client = s.client();
    alice.sign_in().await.unwrap();
    alice.message(&payload(9)).await;
    s.server.app().audit.flush();
    let got = collector.wait_for(7);
    assert_eq!(got.len(), 7, "{got:?}");
    let lines: Vec<String> = got
        .iter()
        .map(|m| serde_json::to_string(&record_of(m)).unwrap())
        .collect();
    let seqs: Vec<u64> = lines
        .iter()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["emitter"]["seq"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(seqs, (1..=7).collect::<Vec<_>>());
    let (key, _) = audit::ensure_key(dir.path()).unwrap();
    let report = verify::verify(
        lines
            .into_iter()
            .enumerate()
            .map(|(i, l)| {
                (
                    Place {
                        file: "syslog".into(),
                        line: i + 1,
                    },
                    l,
                )
            })
            .collect(),
        Some(&Keys::parse(&audit::keys_json(&key)).unwrap()),
        Vec::new(),
    );
    assert!(report.ok(), "{report}");
    s.finish().await;
}
