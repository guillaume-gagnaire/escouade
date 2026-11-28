//! The crate's tests: every event read back as written and closed, the list of §1, the trapped data, the signature and
//! the chain, the buffer and its taking up, the outputs (the turned file, the standard output, syslog over TCP and TLS
//! against a local collector, CEF).

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use ed25519_dalek::SigningKey;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::event::*;
use crate::fill::{Filler, FREE};
use crate::never;
use crate::record::*;
use crate::sink::{Lines, RotatingFile, Sink};
use crate::syslog::{self, Format, Syslog, Target};
use crate::verify::{self, Keys, Place, Problem};
use crate::words::*;
use crate::{Audit, Limits, Outbox};

const T0: i64 = 1_795_257_727_120; // 2026-11-21T10:42:07.120Z

fn team() -> Team {
    Team {
        id: Id::new("9f2c").unwrap(),
        name: Some(Name::new("Acme").unwrap()),
    }
}

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn emitter(seed: u8, device: &str) -> Emitter {
    Emitter::new(
        key(seed),
        Id::new(device).unwrap(),
        Source::Machine,
        team(),
        Chain::default(),
    )
}

/// An event of every slot's sample, its enums at `pick`.
fn sample(name: &str, pick: usize) -> Fact {
    Fact::fill(name, Filler::new(None, pick)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn draft(fact: Fact, at: i64) -> Draft {
    Draft {
        fact,
        at: Time::from_millis(at),
        anchor: Some(Anchor {
            stream: Token::new("team").unwrap(),
            n: 1287,
            epoch: 41,
            ct: Digest::of(b"ciphertext"),
        }),
    }
}

fn session(n: i64) -> Draft {
    Draft {
        fact: Fact::SessionOpened {
            actor: Actor {
                device: Some(Id::new("d-77e0").unwrap()),
                ip: Some(Ip::new("192.0.2.1").unwrap()),
                ..Default::default()
            },
            target: Nothing {},
            detail: Nothing {},
        },
        at: Time::from_millis(T0 + n),
        anchor: None,
    }
}

fn format(cef: bool) -> Format {
    Format {
        hostname: "audit-1.acme.fr".into(),
        cef,
        version: "2.3.0".into(),
    }
}

// ---------- the model ----------

#[test]
fn every_event_reads_back_as_it_was_written_in_each_of_its_variants() {
    let mut e = emitter(1, "d-0b5e");
    for name in Fact::NAMES {
        for pick in 0..6 {
            let fact = sample(name, pick);
            assert_eq!(fact.name(), *name);
            let sealed = e
                .seal(draft(fact, T0), Time::from_millis(T0 + 290))
                .unwrap();
            let back = Record::from_line(&sealed.line)
                .unwrap_or_else(|err| panic!("{name}: {err}\n{}", sealed.line));
            assert_eq!(back, sealed.record, "{name}");
            let v: Value = serde_json::from_str(&sealed.line).unwrap();
            assert_eq!(v["schema"], "escouade.audit/v1");
            assert_eq!(v["event"], *name);
            assert!((1..=5).contains(&v["severity"].as_u64().unwrap()));
        }
    }
}

#[test]
fn a_field_an_event_does_not_know_is_refused_at_every_level() {
    let mut e = emitter(1, "d-0b5e");
    for name in Fact::NAMES {
        let sealed = e
            .seal(draft(sample(name, 0), T0), Time::from_millis(T0))
            .unwrap();
        let v: Value = serde_json::from_str(&sealed.line).unwrap();
        for at in [
            "", "/actor", "/target", "/detail", "/team", "/emitter", "/anchor",
        ] {
            let mut bad = v.clone();
            let Some(Value::Object(m)) = bad.pointer_mut(at) else {
                continue;
            };
            m.insert("title".into(), json!("Rachat de la société Vertbois"));
            assert!(
                Record::from_value(bad).is_err(),
                "{name}: a title taken at « {at} »"
            );
        }
    }
    let mut v: Value =
        serde_json::from_str(&e.seal(session(0), Time::from_millis(T0)).unwrap().line).unwrap();
    v["event"] = json!("ticket.read");
    assert!(
        Record::from_value(v.clone()).is_err(),
        "an event out of the list"
    );
    v["event"] = json!("session.opened");
    v["severity"] = json!(5);
    assert!(
        Record::from_value(v.clone()).is_err(),
        "a severity not the event's"
    );
    v["severity"] = json!(1);
    v["schema"] = json!("escouade.audit/v2");
    assert!(Record::from_value(v).is_err(), "another schema");
}

/// AU-3: the events a station seals for the key of the audit are those §1 gives to S3, and only them.
#[test]
fn the_station_events_are_those_of_s3_in_section_1() {
    let doc = include_str!("../../../docs/TEAM-AUDIT.md");
    let s1 = &doc[doc.find("## 1. ").unwrap()..doc.find("## 2. ").unwrap()];
    let mut from_doc = BTreeSet::new();
    for line in s1.lines().filter(|l| l.starts_with("| `")) {
        let cells: Vec<&str> = line.split('|').collect();
        if cells[2].contains("S3") {
            for name in cells[1].split('`').skip(1).step_by(2) {
                from_doc.insert(name.to_owned());
            }
        }
    }
    let ours: BTreeSet<String> = STATION_EVENTS.iter().map(|s| s.to_string()).collect();
    assert_eq!(ours, from_doc);
    assert_eq!(ours.len(), STATION_EVENTS.len(), "a name given twice");
    for name in STATION_EVENTS {
        assert!(Fact::NAMES.contains(name), "{name} of the list");
    }
    assert!(!ours.contains("stream.written") && !ours.contains("member.added"));
}

#[test]
fn the_list_is_the_one_of_team_audit_section_1_with_its_severities() {
    let doc = include_str!("../../../docs/TEAM-AUDIT.md");
    let s1 = &doc[doc.find("## 1. ").unwrap()..doc.find("## 2. ").unwrap()];
    let mut from_doc = Vec::new();
    for line in s1.lines().filter(|l| l.starts_with("| `")) {
        let cells: Vec<&str> = line.split('|').collect();
        let severity: u8 = cells[3].trim()[..1].parse().unwrap();
        for name in cells[1].split('`').skip(1).step_by(2) {
            from_doc.push((name.to_owned(), severity));
        }
    }
    assert!(from_doc.len() >= 50, "{from_doc:?}");
    for (name, severity) in &from_doc {
        assert_eq!(Fact::severity_of(name), Some(*severity), "{name} of §1");
    }
    // Beyond §1: S1's own words (§3.1) and the audit's own (§4.4, §5.2).
    let ours: BTreeSet<&str> = Fact::NAMES.iter().copied().collect();
    let doc_names: BTreeSet<&str> = from_doc.iter().map(|(n, _)| n.as_str()).collect();
    let beyond: Vec<&&str> = ours.difference(&doc_names).collect();
    assert_eq!(
        beyond,
        [
            &"audit.checkpoint",
            &"audit.dropped",
            &"device.enrolled",
            &"stream.written"
        ]
    );
    assert_eq!(ours.len(), Fact::NAMES.len(), "a name given twice");
    for name in Fact::NAMES {
        assert!(name.len() <= 32, "{name}: over the 32 characters of MSGID");
        assert!(
            name.split('.').count() >= 2
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_')
        );
    }
    // The kill switch (TS-15) and the private store (TS-14) are in it.
    for name in [
        "killswitch.raised",
        "killswitch.lifted",
        "killswitch.applied",
        "store.namespace.declared",
        "store.plugin.published",
        "store.plugin.revoked",
    ] {
        assert!(ours.contains(name), "{name}");
    }
}

#[test]
fn a_permission_always_allowed_is_of_severity_4() {
    let mut fact = sample("permission.answered", 0);
    assert_eq!(fact.severity(), 2);
    if let Fact::PermissionAnswered { detail, .. } = &mut fact {
        detail.answer = Answer::AllowAlways;
    }
    assert_eq!(fact.severity(), 4);
}

// ---------- what never goes out ----------

/// Data put on purpose in every slot: (what, the data, what of it must never be found, whether a free string — a
/// name, a reason — must lose it too). A title, a text or the recovery words in a reason are an administrator's own
/// words (§2: the free strings are the reasons and the names), which no motif tells from a reason.
const TRAPS: &[(&str, &str, &[&str], bool)] = &[
    ("a GitHub token", "ghp_R4chatV3rtb0isS3cr3tT0k3nXYZ123456", &["R4chatV3rtb0isS3cr3tT0k3n"], true),
    ("an Anthropic key", "sk-ant-api03-R4chatV3rtb0is-S3cr3t-k3y-0123456789", &["R4chatV3rtb0is-S3cr3t"], true),
    ("an AWS key", "AKIAR4CHATVERTB0IS12", &["AKIAR4CHATVERTB0IS12"], true),
    ("a bearer token", "Bearer eyJhbGciOiJIUzI1NiJ9.R4chatV3rtb0is.S3cr3tS1gn", &["eyJhbGciOiJIUzI1NiJ9"], true),
    (
        "a private key",
        "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmU\n-----END OPENSSH PRIVATE KEY-----",
        &["b3BlbnNzaC1rZXktdjEAAAAABG5vbmU"],
        true,
    ),
    (
        "an invitation's secret",
        "https://team.acme.fr/join/inv-42#Zk9vQmFyQmF6UXV4MTIzNDU2Nzg5MGFi",
        &["Zk9vQmFyQmF6UXV4MTIzNDU2Nzg5MGFi"],
        true,
    ),
    ("a harness's account", "claire.martin@acme.fr", &["claire.martin@acme.fr"], true),
    ("a Unix path", "/home/claire/projets/vertbois/plan.md", &["/home/claire/projets/vertbois/plan.md"], true),
    (
        "a Windows path",
        "C:\\Users\\claire\\Documents\\Rachat Vertbois - offre.docx",
        &["C:\\Users\\claire\\Documents\\Rachat"],
        true,
    ),
    (
        "a whole command",
        "GITHUB_TOKEN=ghp_R4chatV3rtb0isS3cr3tT0k3nXYZ123456 git push origin rachat-vertbois",
        &["GITHUB_TOKEN=ghp_R4chatV3rtb0isS3cr3tT0k3nXYZ123456 git push", "R4chatV3rtb0isS3cr3tT0k3n"],
        true,
    ),
    ("a title", "Rachat de la société Vertbois", &["Rachat de la société Vertbois"], false),
    ("a file name", "Rachat Vertbois - offre.docx", &["Rachat Vertbois - offre.docx"], false),
    (
        "a text",
        "Claude : voici le plan de rachat de Vertbois.\nÉtape 1 : négocier le prix.\n```diff\n+ prix = 12\n```",
        &["voici le plan de rachat de Vertbois"],
        false,
    ),
    (
        "the recovery words",
        "abandon ability able about above absent absorb abstract absurd abuse access accident account accuse achieve acid acoustic acquire across act action actor actress actual",
        &["absorb abstract absurd abuse"],
        false,
    ),
];

/// What goes out of a record, in the three formats.
fn outputs(r: &Record) -> Vec<String> {
    vec![
        r.to_line(),
        syslog::message(r, &format(false)),
        syslog::message(r, &format(true)),
    ]
}

/// A record of `fact`, not signed (the signature is a sample: what goes out is what is checked).
fn unsigned(team: Team, fact: Fact) -> Record {
    Record {
        id: Ulid::of(Time::from_millis(T0), [1; 10]),
        severity: fact.severity(),
        fact,
        at: Time::from_millis(T0),
        seen: Time::from_millis(T0 + 290),
        team,
        source: Source::Machine,
        emitter: EmitterRef {
            device: Id::new("d-0b5e").unwrap(),
            seq: 48213,
            prev: Some(Digest::of(b"before")),
        },
        anchor: None,
        sig: Some(Signature::new(format!("ed25519:{}==", "A".repeat(86))).unwrap()),
    }
}

/// Without its escapes (JSON's, CEF's, syslog's `\`), to find a trap however it was written.
fn bare(s: &str) -> String {
    s.replace("\\n", " ").replace('\\', "").replace('\n', " ")
}

#[test]
fn trapped_data_put_in_every_slot_of_every_event_is_found_in_nothing_that_goes_out() {
    let mut slots_tried = 0;
    let mut refused = 0;
    let mut scrubbed = 0;
    for name in Fact::NAMES {
        for pick in 0..6 {
            let count = {
                let f = Filler::new(None, pick);
                Team::deserialize(f.clone()).unwrap();
                Fact::fill(name, f.clone()).unwrap();
                f.kinds()
            };
            for (slot, kind) in count.iter().enumerate() {
                let free = FREE.contains(kind);
                for (what, trap, needles, in_free) in TRAPS {
                    if free && !in_free {
                        continue;
                    }
                    slots_tried += 1;
                    let f = Filler::new(Some((slot, trap)), pick);
                    let made = Team::deserialize(f.clone())
                        .and_then(|t| Fact::fill(name, f).map(|fact| (t, fact)));
                    let Ok((t, fact)) = made else {
                        refused += 1;
                        continue;
                    };
                    let r = unsigned(t, fact);
                    for out in outputs(&r) {
                        for needle in *needles {
                            assert!(
                                !bare(&out).contains(&bare(needle)),
                                "{what} put in slot {slot} ({kind}) of {name} goes out:\n{out}"
                            );
                        }
                    }
                    if free {
                        scrubbed += 1;
                        assert!(
                            [never::SECRET_MARK, never::PATH_MARK, never::ADDRESS_MARK]
                                .iter()
                                .any(|m| r.to_line().contains(m)),
                            "{what} in {kind} of {name} is not said removed"
                        );
                    }
                }
            }
        }
    }
    assert!(slots_tried > 5000, "{slots_tried}");
    assert!(
        refused > 0 && scrubbed > 0,
        "{refused} refused, {scrubbed} scrubbed"
    );
}

#[test]
fn the_emitter_says_how_many_strings_lost_a_secret() {
    let mut e = emitter(1, "d-0b5e");
    let fact = Fact::KillswitchRaised {
        actor: Actor {
            name: Some(Name::new("Claire Martin").unwrap()),
            ..Default::default()
        },
        target: ScopeTarget {
            scope: Scope::Agents,
        },
        detail: KillSwitch {
            serial: 3,
            reason: Some(
                Reason::new("fuite de ghp_R4chatV3rtb0isS3cr3tT0k3nXYZ123456 dans /srv/x/y")
                    .unwrap(),
            ),
        },
    };
    let sealed = e.seal(draft(fact, T0), Time::from_millis(T0)).unwrap();
    assert_eq!(sealed.redacted, 2);
    assert!(!sealed.line.contains("ghp_"));
    assert_eq!(
        e.seal(session(1), Time::from_millis(T0)).unwrap().redacted,
        0
    );
}

#[test]
fn a_permission_says_the_program_and_the_fingerprint_never_the_command() {
    let command = "NPM_TOKEN=npm_R4chatV3rtb0isS3cr3t npm publish --tag beta";
    let fact = Fact::PermissionAnswered {
        actor: Actor::default(),
        target: TicketTarget {
            item: Id::new("i-1").unwrap(),
            project: None,
            ticket: Some(TicketKey::new("DEM-41").unwrap()),
        },
        detail: Permission {
            by: Decider::Overseer,
            directive: Some(Id::new("dir-7").unwrap()),
            tool: Token::new("Bash").unwrap(),
            command: Some(CommandPrint::of(command)),
            answer: Answer::AllowOnce,
        },
    };
    let r = unsigned(team(), fact);
    for out in outputs(&r) {
        assert!(
            !out.contains("npm_R4chat") && !out.contains("publish --tag"),
            "{out}"
        );
    }
    let v: Value = serde_json::from_str(&r.to_line()).unwrap();
    assert_eq!(v["detail"]["command"]["program"], "npm");
    assert_eq!(
        v["detail"]["command"]["digest"],
        Digest::of(command.as_bytes()).as_str()
    );
}

// ---------- the signature and the chain ----------

fn chain_of(e: &mut Emitter, n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            e.seal(session(i as i64), Time::from_millis(T0 + i as i64))
                .unwrap()
                .line
        })
        .collect()
}

fn places(lines: &[String]) -> Vec<(Place, String)> {
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            (
                Place {
                    file: "export.jsonl".into(),
                    line: i + 1,
                },
                l.clone(),
            )
        })
        .collect()
}

fn keys_of(e: &Emitter) -> Keys {
    let mut k = Keys::default();
    k.insert(e.device().as_str(), e.public_key().key().unwrap());
    k
}

#[test]
fn each_event_is_signed_on_its_canonical_json_and_chained_to_the_one_before() {
    let mut e = emitter(1, "d-0b5e");
    let lines = chain_of(&mut e, 3);
    let vs: Vec<Value> = lines
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let k = e.public_key().key().unwrap();
    for (i, v) in vs.iter().enumerate() {
        assert!(signature_holds(v, &k));
        assert_eq!(v["emitter"]["seq"], i as u64 + 1);
        // The signature is on the canonical form: the same members in another order hold it.
        let reordered: Value =
            serde_json::from_str(&crate::jcs::canonical_text(&lines[i]).unwrap()).unwrap();
        assert!(signature_holds(&reordered, &k));
    }
    assert!(vs[0].get("emitter").unwrap().get("prev").is_none());
    assert_eq!(
        vs[1]["emitter"]["prev"],
        chain_digest(&vs[0]).unwrap().as_str()
    );
    assert_eq!(
        vs[2]["emitter"]["prev"],
        chain_digest(&vs[1]).unwrap().as_str()
    );
    assert!(!signature_holds(&vs[0], &key(2).verifying_key()));
    let report = verify::verify(places(&lines), Some(&keys_of(&e)), vec![]);
    assert!(report.ok(), "{report}");
    assert_eq!(
        report.to_string(),
        "3 events, 1 emitters, whole chains, 0 problems"
    );
}

#[test]
fn verify_finds_a_line_changed_removed_or_forked_and_takes_an_export_out_of_order() {
    let mut e = emitter(1, "d-0b5e");
    let lines = chain_of(&mut e, 5);
    let keys = keys_of(&e);

    let mut changed = lines.clone();
    changed[2] = changed[2].replace("192.0.2.1", "192.0.2.9");
    let r = verify::verify(places(&changed), Some(&keys), vec![]);
    assert!(r.findings.iter().any(|f| f.problem
        == Problem::BadSignature {
            emitter: "d-0b5e".into(),
            seq: 3
        }));
    assert!(r.findings.iter().any(|f| f.problem
        == Problem::BrokenLink {
            emitter: "d-0b5e".into(),
            seq: 4
        }));

    let mut removed = lines.clone();
    removed.remove(2);
    let r = verify::verify(places(&removed), Some(&keys), vec![]);
    assert_eq!(
        r.findings.iter().map(|f| &f.problem).collect::<Vec<_>>(),
        [&Problem::Gap {
            emitter: "d-0b5e".into(),
            from: 3,
            to: 3
        }]
    );
    assert_eq!(r.findings[0].place.as_ref().unwrap().line, 3);

    let mut reversed = lines.clone();
    reversed.reverse();
    reversed.push(lines[1].clone());
    let r = verify::verify(places(&reversed), Some(&keys), vec![]);
    assert!(r.ok(), "{r}");
    assert_eq!(r.doubles, 1);

    // Another chain that takes the same number: a fork.
    let mut other = emitter(1, "d-0b5e");
    let fork = chain_of(&mut other, 2).pop().unwrap();
    let mut forked = lines.clone();
    forked.push(fork.replace(&T0.to_string(), "x"));
    let r = verify::verify(places(&forked), Some(&keys), vec![]);
    assert!(
        r.findings
            .iter()
            .any(|f| matches!(f.problem, Problem::Fork { seq: 2, .. })),
        "{r}"
    );

    let stranger = emitter(9, "d-other");
    let r = verify::verify(places(&lines), Some(&keys_of(&stranger)), vec![]);
    assert!(matches!(r.findings[0].problem, Problem::UnknownEmitter(_)));
    let mut forged = keys_of(&stranger);
    forged.insert("d-0b5e", key(9).verifying_key());
    let r = verify::verify(places(&lines), Some(&forged), vec![]);
    assert_eq!(r.findings.len(), 5, "{r}");

    let r = verify::verify(places(&["{\"schema\":".into()]), None, vec![]);
    assert!(matches!(r.findings[0].problem, Problem::Unreadable(_)));
}

#[test]
fn a_checkpoint_says_the_head_and_verify_sees_an_end_taken_away() {
    let mut e = emitter(1, "d-0b5e");
    let mut lines = chain_of(&mut e, 4);
    let head = e.chain().clone();
    let checkpoint = |seq: u64, head: Digest| Draft {
        fact: Fact::AuditCheckpoint {
            actor: Actor::default(),
            target: Nothing {},
            detail: Checkpoint { seq, head },
        },
        at: Time::from_millis(T0 + 10),
        anchor: None,
    };
    lines.push(
        e.seal(
            checkpoint(head.seq, head.prev.clone().unwrap()),
            Time::from_millis(T0 + 10),
        )
        .unwrap()
        .line,
    );
    let keys = keys_of(&e);
    assert!(verify::verify(places(&lines), Some(&keys), vec![]).ok());

    // The end taken away, the checkpoint read from the team's stream (--checkpoints).
    let stream = vec![(
        Place {
            file: "team.jsonl".into(),
            line: 1,
        },
        lines[4].clone(),
    )];
    let r = verify::verify(places(&lines[..2]), Some(&keys), stream);
    assert!(
        r.findings.iter().any(|f| f.problem
            == Problem::EndMissing {
                emitter: "d-0b5e".into(),
                seq: 4,
                last: 2
            }),
        "{r}"
    );

    // A checkpoint whose head is not the event of its number.
    let wrong = e
        .seal(
            checkpoint(2, Digest::of(b"another")),
            Time::from_millis(T0 + 11),
        )
        .unwrap()
        .line;
    lines.push(wrong);
    let r = verify::verify(places(&lines), Some(&keys), vec![]);
    assert!(
        r.findings.iter().any(|f| f.problem
            == Problem::Checkpoint {
                emitter: "d-0b5e".into(),
                seq: 2
            }),
        "{r}"
    );
}

#[test]
fn two_events_of_one_entry_of_different_encrypted_bytes_are_found() {
    let mut server = Emitter::new(
        key(3),
        Id::new("srv-1").unwrap(),
        Source::Server,
        team(),
        Chain::default(),
    );
    let mut machine = emitter(1, "d-0b5e");
    let s1 = server
        .seal(
            Draft {
                fact: Fact::StreamWritten {
                    actor: Actor {
                        device: Some(Id::new("d-77e0").unwrap()),
                        ..Default::default()
                    },
                    target: StreamTarget {
                        stream: Token::new("team").unwrap(),
                    },
                    detail: Nothing {},
                },
                at: Time::from_millis(T0),
                anchor: Some(Anchor {
                    stream: Token::new("team").unwrap(),
                    n: 1287,
                    epoch: 41,
                    ct: Digest::of(b"served to the server"),
                }),
            },
            Time::from_millis(T0),
        )
        .unwrap()
        .line;
    let s2 = machine
        .seal(
            draft(sample("settings.changed", 0), T0),
            Time::from_millis(T0),
        )
        .unwrap()
        .line;
    let mut keys = keys_of(&machine);
    keys.insert("srv-1", server.public_key().key().unwrap());
    let r = verify::verify(places(&[s1, s2]), Some(&keys), vec![]);
    assert_eq!(r.emitters, 2);
    assert_eq!(
        r.findings.iter().map(|f| &f.problem).collect::<Vec<_>>(),
        [&Problem::Anchor {
            stream: "team".into(),
            n: 1287
        }]
    );
}

#[test]
fn an_event_over_8_kib_in_syslog_is_not_sealed() {
    let mut e = emitter(1, "d-0b5e");
    let words = List::new(
        (0..MAX_LIST)
            .map(|i| Token::new(format!("w{i}-{}", "x".repeat(58))).unwrap())
            .collect(),
    )
    .unwrap();
    let changes = (0..MAX_LIST)
        .map(|i| {
            Change::Value(PlainChange {
                key: SettingKey::new(format!("agents.k{i}")).unwrap(),
                from: Some(Plain::Words(words.clone())),
                to: Some(Plain::Words(words.clone())),
            })
        })
        .collect();
    let fact = Fact::SettingsChanged {
        actor: Actor::default(),
        target: KeysTarget {
            keys: List::new(vec![]).unwrap(),
        },
        detail: SettingsChanged {
            version: 2,
            changes: List::new(changes).unwrap(),
            signers: List::new(vec![]).unwrap(),
            reason: None,
        },
    };
    assert!(matches!(
        e.seal(draft(fact, T0), Time::from_millis(T0)),
        Err(SealError::TooLong(_))
    ));
    assert_eq!(e.chain().seq, 0, "a refused event takes no number");
    assert!(List::<Token>::new(vec![Token::new("a").unwrap(); MAX_LIST + 1]).is_err());
}

// ---------- the outputs ----------

#[test]
fn json_lines_go_to_the_standard_output_a_line_each() {
    let mut e = emitter(1, "d-0b5e");
    let mut out = Lines::new(Vec::new());
    for l in chain_of(&mut e, 2) {
        out.send(&l).unwrap();
    }
    let text = String::from_utf8(out.into_inner()).unwrap();
    assert_eq!(text.lines().count(), 2);
    assert!(text.lines().all(|l| Record::from_line(l).is_ok()));
}

#[test]
fn the_file_turns_past_its_size_and_at_each_day() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = emitter(1, "d-0b5e");
    let day22 = Time::parse("2026-11-22T00:00:01.000Z").unwrap().millis();
    let lines: Vec<String> = (0..6)
        .map(|i| {
            let seen = if i < 3 { T0 + i } else { day22 + i };
            e.seal(session(i), Time::from_millis(seen)).unwrap().line
        })
        .collect();
    let longest = lines.iter().map(|l| l.len() as u64 + 1).max().unwrap();
    let mut f = RotatingFile::new(dir.path(), "audit", longest * 2);
    for l in &lines {
        f.send(l).unwrap();
    }
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "audit-2026-11-21.1.jsonl",
            "audit-2026-11-21.2.jsonl",
            "audit-2026-11-22.1.jsonl",
            "audit.jsonl"
        ]
    );
    let all: Vec<String> = names
        .iter()
        .flat_map(|n| {
            std::fs::read_to_string(dir.path().join(n))
                .unwrap()
                .lines()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(all.len(), 6);
    let mut sorted = all.clone();
    sorted.sort_by_key(|l| Record::from_line(l).unwrap().emitter.seq);
    assert_eq!(sorted, lines, "nothing lost nor doubled by the turns");
    drop(f);

    // Taken up by a new process: an event of the same day goes on in the file, one of the next day turns it.
    let mut again = RotatingFile::new(dir.path(), "audit", 1 << 20);
    again
        .send(
            &e.seal(session(6), Time::from_millis(day22 + 6))
                .unwrap()
                .line,
        )
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("audit.jsonl"))
            .unwrap()
            .lines()
            .count(),
        2
    );
    drop(again);
    let mut next = RotatingFile::new(dir.path(), "audit", 1 << 20);
    next.send(
        &e.seal(session(7), Time::from_millis(day22 + 86_400_000))
            .unwrap()
            .line,
    )
    .unwrap();
    assert!(dir.path().join("audit-2026-11-22.2.jsonl").exists());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("audit.jsonl"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn a_syslog_message_is_rfc_5424_with_its_structured_data_and_cef_in_option() {
    let mut e = emitter(1, "d-0b5e");
    let sealed = e
        .seal(
            draft(sample("settings.changed", 0), T0),
            Time::from_millis(T0 + 290),
        )
        .unwrap();
    let m = syslog::message(&sealed.record, &format(false));
    let id = sealed.record.id.as_str();
    let want = format!(
        "<107>1 2026-11-21T10:42:07.120Z audit-1.acme.fr escouade - settings.changed [escouade@32473 schema=\"escouade.audit/v1\" id=\"{id}\" event=\"settings.changed\" severity=\"4\" team=\"9f2c\" source=\"machine\" device=\"d-0b5e\" seq=\"1\"] "
    );
    assert!(m.starts_with(&want), "{m}");
    assert_eq!(&m[want.len()..], sealed.line, "the whole JSON as MSG");
    assert_eq!(syslog::frame("abc"), "3 abc");
    assert_eq!(syslog::syslog_severity(5), 2);
    assert_eq!(syslog::syslog_severity(1), 6);

    let c = syslog::message(&sealed.record, &format(true));
    let cef = &c[c.find("CEF:").unwrap()..];
    assert!(cef.starts_with("CEF:0|Escouade|escouade-team|2.3.0|settings.changed|settings.changed|8|rt=1795257727120 externalId="), "{cef}");
    for k in [
        "cs1Label=team cs1=9f2c",
        "cs3Label=target cs3={\"keys\":[\"agents.engines\"]}",
        "cs5Label=emitter cs5=d-0b5e 1 -",
        "cs6Label=anchor cs6=team 1287 41 sha256:",
    ] {
        assert!(cef.contains(k), "{k} in {cef}");
    }
    assert!(cef.contains("suser=Claire Martin"), "{cef}");
    // CEF's escapes: `=` and `\` in a value.
    assert_eq!(syslog::cef_ext("a=b\\c\nd"), "a\\=b\\\\c\\nd");
    assert_eq!(syslog::cef_header("a|b\\"), "a\\|b\\\\");

    for name in Fact::NAMES {
        let sealed = e
            .seal(draft(sample(name, 0), T0), Time::from_millis(T0))
            .unwrap();
        assert!(
            syslog::longest(&sealed.record) <= syslog::MAX_MESSAGE,
            "{name}"
        );
    }
    assert_eq!(
        Target::parse("tls://siem.acme.fr:6514").unwrap(),
        Target {
            tls: true,
            host: "siem.acme.fr".into(),
            port: 6514
        }
    );
    assert!(
        Target::parse("udp://siem.acme.fr:514").is_err(),
        "never UDP"
    );
    assert!(Target::parse("tls://siem.acme.fr").is_err());
}

/// A syslog collector of the tests, on a port of the system: it reads the frames of RFC 5425 and keeps the messages.
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
        let n: usize = String::from_utf8_lossy(&len[..len.len() - 1])
            .parse()
            .expect("octet counting");
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
        for _ in 0..500 {
            if self.got.lock().unwrap().len() >= n {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        self.got.lock().unwrap().clone()
    }
}

fn syslog_to(port: u16, tls: bool, ca: Option<&[u8]>, cef: bool) -> Syslog {
    let target = Target {
        tls,
        host: "localhost".into(),
        port,
    };
    let roots = ca.map(|pem| syslog::roots(Some(pem)).unwrap());
    Syslog::new(target, format(cef), roots).unwrap()
}

#[test]
fn syslog_goes_over_tls_to_a_collector_and_refuses_a_certificate_it_does_not_trust() {
    let (pem, config) = tls_material();
    let collector = Collector::start(TcpListener::bind("127.0.0.1:0").unwrap(), Some(config));
    let mut e = emitter(1, "d-0b5e");
    let lines = chain_of(&mut e, 3);
    let mut s = syslog_to(collector.port, true, Some(&pem), false);
    for l in &lines {
        s.send(l).unwrap();
    }
    let got = collector.wait_for(3);
    assert_eq!(got.len(), 3);
    for (m, l) in got.iter().zip(&lines) {
        assert!(m.starts_with("<110>1 "), "{m}");
        assert!(m.ends_with(l.as_str()));
    }
    let mut cef = syslog_to(collector.port, true, Some(&pem), true);
    cef.send(&lines[0]).unwrap();
    assert!(collector.wait_for(4)[3].contains(" CEF:0|Escouade|escouade-team|"));

    // Another certificate than the collector's: refused, nothing sent.
    let (other, _) = tls_material();
    let mut wrong = syslog_to(collector.port, true, Some(&other), false);
    assert!(wrong.send(&lines[0]).is_err());
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(collector.got.lock().unwrap().len(), 4);
}

#[test]
fn syslog_goes_over_plain_tcp_too() {
    let collector = Collector::start(TcpListener::bind("127.0.0.1:0").unwrap(), None);
    let mut e = emitter(1, "d-0b5e");
    let mut s = syslog_to(collector.port, false, None, false);
    s.send(&chain_of(&mut e, 1)[0]).unwrap();
    assert_eq!(collector.wait_for(1).len(), 1);
}

// ---------- the buffer and its taking up ----------

/// An output that fails while it is down.
struct Flaky {
    up: bool,
    got: Vec<String>,
}

impl Sink for Flaky {
    fn send(&mut self, line: &str) -> std::io::Result<()> {
        if !self.up {
            return Err(std::io::Error::other("down"));
        }
        self.got.push(line.to_owned());
        Ok(())
    }
}

fn audit_in(dir: &std::path::Path, limits: Limits, up: bool) -> Audit<Flaky> {
    let outbox = Outbox::open(dir, limits).unwrap();
    let chain = outbox.chain().unwrap();
    Audit {
        emitter: Emitter::new(
            key(1),
            Id::new("d-0b5e").unwrap(),
            Source::Machine,
            team(),
            chain,
        ),
        outbox,
        sink: Flaky { up, got: vec![] },
    }
}

#[test]
fn events_wait_on_the_disk_while_the_output_is_down_and_go_in_order_when_it_is_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = audit_in(
        dir.path(),
        Limits {
            segment_bytes: 2048,
            ..Limits::default()
        },
        false,
    );
    for i in 0..10 {
        a.emit(session(i), Time::from_millis(T0 + i)).unwrap();
    }
    assert_eq!(a.outbox.waiting().unwrap(), 10);
    assert!(
        std::fs::read_dir(dir.path().join("spool")).unwrap().count() > 1,
        "several segments"
    );
    a.sink.up = true;
    assert_eq!(a.flush().unwrap(), 10);
    let seqs: Vec<u64> = a
        .sink
        .got
        .iter()
        .map(|l| Record::from_line(l).unwrap().emitter.seq)
        .collect();
    assert_eq!(seqs, (1..=10).collect::<Vec<_>>());
    assert_eq!(a.outbox.waiting().unwrap(), 0);
    assert_eq!(
        std::fs::read_dir(dir.path().join("spool")).unwrap().count(),
        1,
        "the segments sent are gone"
    );
    let r = verify::verify(places(&a.sink.got), Some(&keys_of(&a.emitter)), vec![]);
    assert!(r.ok(), "{r}");
}

#[test]
fn past_its_bytes_or_its_age_the_buffer_drops_the_oldest_and_says_it_with_audit_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let probe = emitter(1, "d-0b5e")
        .seal(session(0), Time::from_millis(T0))
        .unwrap()
        .line
        .len() as u64
        + 1;
    let limits = Limits {
        max_bytes: probe * 5 + probe / 2,
        max_age_ms: 3_600_000,
        segment_bytes: probe * 3,
    };
    let mut a = audit_in(dir.path(), limits, false);
    for i in 0..8 {
        a.emit(session(i), Time::from_millis(T0 + i)).unwrap();
    }
    a.sink.up = true;
    a.flush().unwrap();
    let got: Vec<Record> = a
        .sink
        .got
        .iter()
        .map(|l| Record::from_line(l).unwrap())
        .collect();
    let dropped: Vec<&Dropped> = got
        .iter()
        .filter_map(|r| match &r.fact {
            Fact::AuditDropped { detail, .. } => Some(detail),
            _ => None,
        })
        .collect();
    assert!(!dropped.is_empty());
    // Each number of the chain went, or an audit.dropped that went says it (one that dropped in turn passed what it
    // said to the next).
    let mut covered: BTreeSet<u64> = got.iter().map(|r| r.emitter.seq).collect();
    for d in &dropped {
        covered.extend(d.first..=d.last);
    }
    let last = got.iter().map(|r| r.emitter.seq).max().unwrap();
    assert_eq!(covered, (1..=last).collect::<BTreeSet<_>>());
    assert!(
        got.iter()
            .filter(|r| matches!(r.fact, Fact::SessionOpened { .. }))
            .count()
            < 8
    );
    // An audit.dropped that dropped in turn gave what it said to a later one: the first of the chain is said.
    assert!(
        dropped
            .iter()
            .any(|d| d.first == 1 && d.from == Time::from_millis(T0)),
        "{dropped:?}"
    );
    let r = verify::verify(places(&a.sink.got), Some(&keys_of(&a.emitter)), vec![]);
    assert!(r.ok(), "a gap an audit.dropped says is not one: {r}");
    assert!(r.dropped > 0);

    // By age: a day later, all that waits is older than its hour.
    let dir = tempfile::tempdir().unwrap();
    let mut a = audit_in(dir.path(), limits, false);
    a.emit(session(0), Time::from_millis(T0)).unwrap();
    a.emit(session(1), Time::from_millis(T0 + 86_400_000))
        .unwrap();
    a.sink.up = true;
    a.flush().unwrap();
    let got: Vec<Record> = a
        .sink
        .got
        .iter()
        .map(|l| Record::from_line(l).unwrap())
        .collect();
    assert_eq!(got.len(), 2, "{:?}", a.sink.got);
    assert_eq!(got[0].emitter.seq, 2);
    assert!(
        matches!(&got[1].fact, Fact::AuditDropped { detail, .. } if detail.count == 1 && detail.first == 1 && detail.last == 1)
    );
    assert_eq!(Limits::default().max_bytes, 1 << 30);
    assert_eq!(Limits::default().max_age_ms, 7 * 86_400_000);
}

#[test]
fn a_new_process_takes_up_at_the_last_event_written_without_a_gap_nor_a_double() {
    let dir = tempfile::tempdir().unwrap();
    let mut all = Vec::new();
    {
        let mut a = audit_in(dir.path(), Limits::default(), true);
        for i in 0..3 {
            a.emit(
                draft(sample("claim.taken", 0), T0 + i),
                Time::from_millis(T0 + i),
            )
            .unwrap();
        }
        a.sink.up = false;
        a.emit(session(3), Time::from_millis(T0 + 3)).unwrap();
        all.append(&mut a.sink.got);
    }
    {
        let mut a = audit_in(dir.path(), Limits::default(), true);
        assert_eq!(a.emitter.chain().seq, 4);
        assert_eq!(
            a.emitter.chain().positions.get("team"),
            Some(&1287),
            "the position of the stream read"
        );
        a.emit(session(4), Time::from_millis(T0 + 4)).unwrap();
        all.append(&mut a.sink.got);
    }
    let seqs: Vec<u64> = all
        .iter()
        .map(|l| Record::from_line(l).unwrap().emitter.seq)
        .collect();
    assert_eq!(seqs, [1, 2, 3, 4, 5]);
    let r = verify::verify(places(&all), Some(&keys_of(&emitter(1, "d-0b5e"))), vec![]);
    assert!(r.ok() && r.doubles == 0, "{r}");

    // Stopped between the line and the chain's file: the chain is taken from the buffer's last line.
    std::fs::write(
        dir.path().join("chain.json"),
        br#"{"seq":2,"positions":{}}"#,
    )
    .unwrap();
    let mut a = audit_in(dir.path(), Limits::default(), true);
    assert_eq!(a.emitter.chain().seq, 5);
    a.emit(session(5), Time::from_millis(T0 + 5)).unwrap();
    all.append(&mut a.sink.got);
    let r = verify::verify(places(&all), Some(&keys_of(&a.emitter)), vec![]);
    assert!(r.ok(), "{r}");
}

#[test]
fn the_buffer_goes_to_syslog_once_the_collector_answers() {
    let (pem, config) = tls_material();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let dir = tempfile::tempdir().unwrap();
    let outbox = Outbox::open(dir.path(), Limits::default()).unwrap();
    let mut a = Audit {
        emitter: emitter(1, "d-0b5e"),
        outbox,
        sink: syslog_to(port, true, Some(&pem), false),
    };
    for i in 0..3 {
        a.emit(session(i), Time::from_millis(T0 + i)).unwrap();
    }
    assert_eq!(a.outbox.waiting().unwrap(), 3);
    let collector = Collector::start(
        TcpListener::bind(("127.0.0.1", port)).unwrap(),
        Some(config),
    );
    assert_eq!(a.flush().unwrap(), 3);
    let got = collector.wait_for(3);
    assert_eq!(got.len(), 3);
    assert!(got[2].contains("seq=\"3\""));
}

// ---------- the vectors ----------

fn vectors() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors")
}

#[test]
fn the_canonical_json_of_the_vectors_of_rfc_8785() {
    let dir = vectors().join("jcs");
    let mut n = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let Some(stem) = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".input.json")
        else {
            continue;
        };
        let input = std::fs::read_to_string(&path).unwrap();
        let want = std::fs::read_to_string(dir.join(format!("{stem}.output.json"))).unwrap();
        assert_eq!(
            crate::jcs::canonical_text(&input).unwrap(),
            want.trim_end_matches('\n'),
            "{stem}"
        );
        n += 1;
    }
    assert!(n >= 4, "{n} vectors");
}

/// The chain's vectors (`tests/vectors/audit/`): written once by `ESCOUADE_AUDIT_VECTORS=write` with the tests' keys
/// (seeds 1 and 3, never a real key), read by the tests of `verify` here and by those of the binary.
#[test]
fn the_vectors_of_the_chain_verify_as_they_say() {
    let dir = vectors().join("audit");
    if std::env::var("ESCOUADE_AUDIT_VECTORS").as_deref() == Ok("write") {
        std::fs::create_dir_all(&dir).unwrap();
        let mut machine = emitter(1, "d-0b5e");
        let mut server = Emitter::new(
            key(3),
            Id::new("srv-1").unwrap(),
            Source::Server,
            team(),
            Chain::default(),
        );
        let mut lines = Vec::new();
        for (i, name) in [
            "member.added",
            "settings.changed",
            "killswitch.raised",
            "permission.answered",
            "store.plugin.published",
        ]
        .iter()
        .enumerate()
        {
            lines.push(
                machine
                    .seal(
                        draft(sample(name, 1), T0 + i as i64),
                        Time::from_millis(T0 + i as i64 + 290),
                    )
                    .unwrap()
                    .line,
            );
        }
        lines.push(
            server
                .seal(session(9), Time::from_millis(T0 + 9))
                .unwrap()
                .line,
        );
        let head = machine.chain().clone();
        let cp = machine
            .seal(
                Draft {
                    fact: Fact::AuditCheckpoint {
                        actor: Actor::default(),
                        target: Nothing {},
                        detail: Checkpoint {
                            seq: head.seq,
                            head: head.prev.unwrap(),
                        },
                    },
                    at: Time::from_millis(T0 + 20),
                    anchor: None,
                },
                Time::from_millis(T0 + 20),
            )
            .unwrap()
            .line;
        lines.push(cp.clone());
        let write = |name: &str, ls: &[String]| {
            std::fs::write(dir.join(name), ls.join("\n") + "\n").unwrap()
        };
        write("export.jsonl", &lines);
        let mut changed = lines.clone();
        changed[1] = changed[1].replace("\"version\":7", "\"version\":8");
        assert_ne!(changed[1], lines[1]);
        write("changed.jsonl", &changed);
        let mut removed = lines.clone();
        removed.remove(2);
        write("removed.jsonl", &removed);
        write("cut.jsonl", &lines[..3]);
        write("checkpoints.jsonl", &[cp]);
        let keys = json!({ "keys": { "d-0b5e": machine.public_key().as_str(), "srv-1": server.public_key().as_str() } });
        std::fs::write(
            dir.join("keys.json"),
            serde_json::to_string_pretty(&keys).unwrap() + "\n",
        )
        .unwrap();
    }
    let read = |name: &str| -> Vec<(Place, String)> {
        std::fs::read_to_string(dir.join(name))
            .unwrap()
            .lines()
            .enumerate()
            .map(|(i, l)| {
                (
                    Place {
                        file: name.into(),
                        line: i + 1,
                    },
                    l.to_owned(),
                )
            })
            .collect()
    };
    let keys = Keys::parse(&std::fs::read_to_string(dir.join("keys.json")).unwrap()).unwrap();
    assert_eq!(
        keys_of(&emitter(1, "d-0b5e")).0.get("d-0b5e"),
        keys.0.get("d-0b5e"),
        "the tests' key, seed 1"
    );
    let r = verify::verify(read("export.jsonl"), Some(&keys), read("checkpoints.jsonl"));
    assert!(r.ok(), "{r}");
    assert_eq!((r.events, r.emitters), (7, 2));
    let r = verify::verify(read("changed.jsonl"), Some(&keys), vec![]);
    assert!(
        r.findings
            .iter()
            .any(|f| matches!(f.problem, Problem::BadSignature { seq: 2, .. })),
        "{r}"
    );
    let r = verify::verify(read("removed.jsonl"), Some(&keys), vec![]);
    assert!(
        r.findings
            .iter()
            .any(|f| matches!(f.problem, Problem::Gap { from: 3, to: 3, .. })),
        "{r}"
    );
    let r = verify::verify(read("cut.jsonl"), Some(&keys), read("checkpoints.jsonl"));
    assert!(
        r.findings.iter().any(|f| matches!(
            f.problem,
            Problem::EndMissing {
                seq: 5,
                last: 3,
                ..
            }
        )),
        "{r}"
    );
}
