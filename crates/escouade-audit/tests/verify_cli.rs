//! `escouade-audit verify` on the vector files (`tests/vectors/audit/`, written by the crate's test
//! `the_vectors_of_the_chain_verify_as_they_say`): 0 for a whole export, 1 for a problem, 2 for what it cannot read.

use std::path::PathBuf;
use std::process::{Command, Output};

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/audit")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_escouade-audit"))
        .args(args)
        .current_dir(vectors())
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn a_whole_export_verifies_with_its_keys_and_its_checkpoints() {
    let o = run(&[
        "verify",
        "--keys",
        "keys.json",
        "--checkpoints",
        "checkpoints.jsonl",
        "export.jsonl",
    ]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout).trim(),
        "7 events, 2 emitters, whole chains, 0 problems"
    );
}

#[test]
fn a_line_changed_removed_or_an_end_cut_is_found_with_its_place() {
    let o = run(&["verify", "--keys", "keys.json", "changed.jsonl"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        text(&o).contains("changed.jsonl:2: d-0b5e #2: the signature does not hold"),
        "{}",
        text(&o)
    );

    let o = run(&["verify", "--keys", "keys.json", "removed.jsonl"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        text(&o).contains("removed.jsonl:3: d-0b5e: #3 to #3 missing"),
        "{}",
        text(&o)
    );

    let o = run(&[
        "verify",
        "--keys",
        "keys.json",
        "--checkpoints",
        "checkpoints.jsonl",
        "cut.jsonl",
    ]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        text(&o).contains("a checkpoint says #5, the export ends at #3"),
        "{}",
        text(&o)
    );
}

#[test]
fn without_keys_the_chains_are_checked_and_it_says_so() {
    let o = run(&["verify", "removed.jsonl"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("no --keys: the signatures are not checked"));
    let o = run(&["verify", "changed.jsonl"]);
    assert!(
        text(&o).contains("#3: prev is not the fingerprint of #2"),
        "{}",
        text(&o)
    );
}

#[test]
fn what_it_cannot_read_answers_2() {
    assert_eq!(run(&["verify"]).status.code(), Some(2));
    assert_eq!(run(&["verify", "absent.jsonl"]).status.code(), Some(2));
    assert_eq!(
        run(&["verify", "--keys", "export.jsonl", "export.jsonl"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(run(&[]).status.code(), Some(2));
}

#[test]
fn the_closed_list_and_what_never_goes_out_are_printed() {
    let o = run(&["events"]);
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.lines().any(|l| l == "killswitch.raised\t5"));
    assert!(out.lines().any(|l| l == "lock.refused\t4"));
    let o = run(&["never"]);
    assert!(String::from_utf8_lossy(&o.stdout).contains("its program and its fingerprint only"));
}
