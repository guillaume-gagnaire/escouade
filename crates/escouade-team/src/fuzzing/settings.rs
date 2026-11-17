//! The team's signed `.escouade.yml` (TS-13, `settings.rs`), once its entries are opened: the versions of the file
//! and the administrators' cosignatures, which a member or the server may write in any order, replay or invent.
//! - The bytes as a file (JSON, as a version carries it): what `never` lets through imposes nothing that runs.
//! - The bytes as entries: versions of a small vocabulary of keys, by three members (two administrators now, one who
//!   was one before), each citing a version before it or none, and cosignatures. The chain the rules choose goes
//!   1, 2, 3… each citing the one before; each of its versions reads, is of this format, holds nothing of the list
//!   of what never enters, and is signed by as many administrators of now as it needs (two under the quorum for a
//!   guarded change). Read again with what came later, the chain chosen before is kept: never undone.

use std::collections::BTreeSet;

use escouade_team_crypto::MemberId;
use serde_json::{json, Value};

use crate::settings::{
    digest, judge, needs_quorum, never, Classify, Entry, Part, Sort, Status, TeamFile, Version,
    FORMAT,
};
use crate::wire;

struct Keys;

impl Classify for Keys {
    fn sort(&self, key: &str) -> Option<Sort> {
        Some(match key {
            "permissions.modes" | "harnesses.allowed" | "private" => Sort::Restrict,
            "board.on_approve" | "permissions.defaultMode" | "store.namespace" => Sort::Value,
            "proxy.url" | "proxy.exclusions" => Sort::Sensitive,
            "worktrees.setup" | "launch" | "plugins.install" => Sort::Executable,
            _ => return None,
        })
    }
}

const KEYS: [&str; 10] = [
    "permissions.modes",
    "private",
    "board.on_approve",
    "permissions.defaultMode",
    "store.namespace",
    "proxy.url",
    "worktrees.setup",
    "plugins.install",
    "hooks.pre",
    "unknown.key",
];

fn value(n: u8) -> Value {
    match n % 6 {
        0 => json!(["plan", "default"]),
        1 => json!(false),
        2 => json!("bypassPermissions"),
        3 => json!("http://proxy.acme:3128"),
        4 => json!("dev.escouade.x"),
        _ => json!(format!("v{n}")),
    }
}

/// A file of the vocabulary: a few keys in a part each.
fn file_of(bytes: &[u8]) -> TeamFile {
    let mut f = TeamFile {
        version: FORMAT,
        ..Default::default()
    };
    for pair in bytes.chunks(2) {
        let k = KEYS[usize::from(pair[0]) % KEYS.len()].to_string();
        let v = value(pair.get(1).copied().unwrap_or(0));
        match pair[0] / 16 % 4 {
            0 => f.defaults.insert(k, v),
            1 => f.enforced.insert(k, v),
            2 => f.proposed.insert(k, v),
            _ => {
                f.quorum = Some(pair.get(1).is_some_and(|b| b % 2 == 0));
                None
            }
        };
    }
    f
}

fn member(n: u8) -> MemberId {
    MemberId([n; 16])
}

pub fn settings(data: &[u8]) {
    // The bytes as a file.
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(file) = serde_json::from_str::<TeamFile>(text) {
            if never(&file, &Keys).is_none() {
                for (part, map) in [
                    (Part::Defaults, &file.defaults),
                    (Part::Enforced, &file.enforced),
                ] {
                    for key in map.keys() {
                        assert_ne!(
                            Keys.sort(key),
                            Some(Sort::Executable),
                            "{} imposes {key}, which runs",
                            part.name()
                        );
                    }
                }
            }
            let _ = needs_quorum(None, &file, &Keys);
        }
    }
    if let Ok(v) = wire::decode::<Version>(data) {
        let _ = serde_json::from_str::<TeamFile>(&v.file);
    }
    let _ = wire::decode::<Entry>(data);

    // The bytes as entries: [who, epoch, kind, cites, len, file bytes…].
    let mut seen = Vec::new();
    let mut bodies: Vec<([u8; 32], u64)> = Vec::new();
    let mut rest = data;
    let mut seq = 0;
    while rest.len() >= 5 {
        let (head, tail) = rest.split_at(5);
        let len = usize::from(head[4] % 12).min(tail.len());
        let (bytes, tail) = tail.split_at(len);
        rest = tail;
        seq += 1;
        let who = member(1 + head[0] % 3);
        let epoch = u64::from(head[1] % 4);
        let cites = (head[3] as usize).checked_rem(bodies.len() + 1);
        let entry = if head[2] % 3 == 0 && !bodies.is_empty() {
            let (d, number) = bodies[usize::from(head[3]) % bodies.len()];
            Entry::Cosign { number, digest: d }
        } else {
            let (previous, number) = match cites {
                Some(i) if i < bodies.len() => (Some(bodies[i].0), bodies[i].1 + 1),
                _ => (None, 1),
            };
            let version = Version {
                number,
                previous,
                file: serde_json::to_string(&file_of(bytes)).expect("a file"),
                by: format!("m{}", head[0] % 3),
                at: seq,
                note: String::new(),
            };
            let body = if head[2] % 7 == 6 {
                bytes.to_vec()
            } else {
                wire::encode(&version)
            };
            bodies.push((digest(&body), number));
            Entry::Version { body }
        };
        seen.push((seq, epoch, who, entry));
    }
    // Members 1 and 2 are administrators now; member 3 was one at epochs 0 and 1.
    let now: BTreeSet<MemberId> = [member(1), member(2)].into_iter().collect();
    let then = |e: u64| -> Option<BTreeSet<MemberId>> {
        Some(if e < 2 {
            [member(1), member(3)].into_iter().collect()
        } else {
            [member(1), member(2)].into_iter().collect()
        })
    };
    let half = seen.len() / 2;
    let mut chosen = Vec::new();
    let (first, chain_before, _) = judge(
        seen[..half].to_vec(),
        now.clone(),
        &then,
        &mut chosen,
        &Keys,
    );
    let before: Vec<[u8; 32]> = chain_before.iter().map(|&i| first[i].digest).collect();
    let (all, chain, quorum) = judge(seen, now.clone(), &then, &mut chosen, &Keys);
    let digests: Vec<[u8; 32]> = chain.iter().map(|&i| all[i].digest).collect();
    assert!(digests.starts_with(&before), "a chain chosen before undone");

    let mut prev: Option<&TeamFile> = None;
    let mut prev_digest = None;
    for (n, &i) in chain.iter().enumerate() {
        let j = &all[i];
        assert_eq!(j.version.number, n as u64 + 1, "a chain with a gap");
        assert_eq!(
            j.version.previous, prev_digest,
            "a version that does not cite the one before"
        );
        let file = j.file.as_ref().expect("a version in the chain reads");
        assert_eq!(file.version, FORMAT);
        assert!(
            never(file, &Keys).is_none(),
            "a version that imposes what never enters"
        );
        assert!(matches!(j.status, Status::Applied | Status::Superseded));
        let sticky = before.get(n) == Some(&j.digest);
        if !sticky {
            assert!(
                j.counted.len() >= j.need,
                "a version without its signatures"
            );
            assert!(
                j.counted.is_subset(&now),
                "a signature of no administrator of now"
            );
        }
        let quorum_then = prev.and_then(|p| p.quorum).unwrap_or(now.len() >= 2);
        if quorum_then && needs_quorum(prev, file, &Keys) {
            assert_eq!(
                j.need, 2,
                "a guarded change under the quorum with one signature"
            );
        }
        prev = j.file.as_ref();
        prev_digest = Some(j.digest);
    }
    let _ = quorum;
}
