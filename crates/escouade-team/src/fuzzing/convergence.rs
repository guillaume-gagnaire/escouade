//! The convergence of the copies (C2-A, `replica.rs`): operations of three devices, made from the bytes six at a time
//! (a device, an action, an object, a field, a value, how far its clock goes), then applied to four copies: in the
//! order they were made; in an order the next bytes choose, with some played again; backwards; and from a snapshot
//! of the first ones, then the rest. Every copy must end the same, and the second again once replayed from its
//! journal. The same rule as the proptest of `replica.rs`, which the fuzzer drives further.

use std::collections::BTreeMap;

use escouade_team_crypto::hlc::{Clock, Hlc};
use escouade_team_crypto::ops::{Author, OpBody, Operation};
use escouade_team_crypto::{DeviceId, MemberId, ObjectId, ProjectId};
use serde_json::Value;

use crate::replica::Replica;

const P: ProjectId = ProjectId([7; 16]);

fn device(n: u8) -> DeviceId {
    DeviceId([n + 1; 16])
}

fn object(n: u8) -> ObjectId {
    ObjectId([n + 1; 16])
}

/// The operations the steps make, as each device writes them (its own clock, its last write of a field as base).
pub(crate) fn operations(steps: &[[u8; 6]]) -> Vec<(Operation, [u8; 32])> {
    let mut clocks = [Clock::default(); 3];
    let mut walls = [0u64; 3];
    let mut seen: BTreeMap<(u8, u8, u8), Hlc> = BTreeMap::new();
    let mut versions: BTreeMap<u8, Vec<ObjectId>> = BTreeMap::new();
    let mut dseq = [0u64; 3];
    let mut out = Vec::new();
    for (i, &[d, action, o, f, v, advance]) in steps.iter().enumerate() {
        let d = d % 3;
        let di = d as usize;
        walls[di] += u64::from(advance % 3);
        let hlc = clocks[di].tick(walls[di], device(d));
        let o = o % 3;
        let f = f % 3;
        let object = object(o);
        let field = ["title", "description", "column"][f as usize].to_string();
        let value = Value::from(format!("v{v}"));
        let body = match action % 6 {
            0 => OpBody::Create {
                object,
                kind: "ticket".into(),
                fields: [(field.clone(), value)].into_iter().collect(),
            },
            1 | 2 => OpBody::Set {
                object,
                field: field.clone(),
                value,
                base: seen.get(&(d, o, f)).copied(),
            },
            3 => OpBody::Move {
                object,
                column: format!("c{v}"),
                rank: (v % 2 == 0).then(|| format!("r{v}")),
                base: seen.get(&(d, o, 2)).copied(),
            },
            4 => OpBody::Delete { object },
            _ => {
                let known = versions.entry(o).or_default();
                let bases = known.iter().rev().take((v % 2) as usize).copied().collect();
                let version = ObjectId([i as u8; 16]);
                known.push(version);
                OpBody::Version {
                    object,
                    version,
                    bases,
                    content: value,
                }
            }
        };
        seen.insert((d, o, f), hlc);
        dseq[di] += 1;
        let op = Operation {
            project: P,
            author: Author {
                member: MemberId([d; 16]),
                device: device(d),
                key: vec![d; 32],
            },
            dseq: dseq[di],
            prev: [0; 32],
            team_epoch: 0,
            team_head: [0; 32],
            hlc,
            body,
        };
        let digest = Operation::digest_of(&op.to_bytes());
        out.push((op, digest));
    }
    out
}

/// Everything the interface reads of a copy.
fn read(r: &Replica) -> impl PartialEq + std::fmt::Debug {
    let ids: Vec<ObjectId> = (0..3).map(object).collect();
    (
        r.state().expect("a state"),
        r.objects(None).expect("the objects"),
        ids.iter()
            .map(|i| r.conflicts(i).expect("the conflicts"))
            .collect::<Vec<_>>(),
        ids.iter()
            .map(|i| r.versions(i).expect("the versions"))
            .collect::<Vec<_>>(),
    )
}

pub fn convergence(data: &[u8]) {
    // At most 48 operations: past it, more bytes only choose the order.
    let n = (data.len() / 6).min(48);
    let steps: Vec<[u8; 6]> = data[..n * 6].as_chunks::<6>().0.to_vec();
    if steps.is_empty() {
        return;
    }
    let rest = &data[n * 6..];
    let ops = operations(&steps);
    // An order from the rest: a shuffle by its bytes (Fisher–Yates), then some played again, and a cut.
    let mut order: Vec<usize> = (0..ops.len()).collect();
    for (i, b) in rest.iter().enumerate().take(ops.len()) {
        let j = *b as usize % (ops.len() - i) + i;
        order.swap(i, j);
    }
    let again: Vec<usize> = rest
        .iter()
        .skip(ops.len())
        .take(8)
        .map(|b| *b as usize % ops.len())
        .collect();
    let cut = rest.last().map_or(0, |b| *b as usize % (ops.len() + 1));

    let a = Replica::in_memory().expect("a copy");
    for (op, d) in &ops {
        a.apply(op, d, None).expect("applied");
    }
    let b = Replica::in_memory().expect("a copy");
    for i in &order {
        b.apply(&ops[*i].0, &ops[*i].1, Some(*i as u64 + 1))
            .expect("applied");
    }
    for i in &again {
        assert!(
            !b.apply(&ops[*i].0, &ops[*i].1, None).expect("again"),
            "an operation counted twice"
        );
    }
    let c = Replica::in_memory().expect("a copy");
    for (op, d) in ops.iter().rev() {
        c.apply(op, d, None).expect("applied");
    }
    let whole = read(&a);
    assert_eq!(whole, read(&b), "another order, another state");
    assert_eq!(whole, read(&c), "backwards, another state");
    b.rebuild().expect("rebuilt");
    assert_eq!(whole, read(&b), "replayed from the journal, another state");

    // A snapshot of the first `cut` in the order, then the rest.
    let first = Replica::in_memory().expect("a copy");
    for (k, j) in order.iter().take(cut).enumerate() {
        first
            .apply(&ops[*j].0, &ops[*j].1, Some(k as u64 + 1))
            .expect("applied");
    }
    let later = Replica::in_memory().expect("a copy");
    later
        .load_snapshot(&first.dump().expect("a dump"), cut as u64)
        .expect("loaded");
    for (k, j) in order.iter().enumerate().skip(cut) {
        later
            .apply(&ops[*j].0, &ops[*j].1, Some(k as u64 + 1))
            .expect("applied");
    }
    assert_eq!(
        whole,
        read(&later),
        "a snapshot then the rest, another state"
    );
}
