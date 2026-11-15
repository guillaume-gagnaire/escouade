//! The team's file in the core (TS-13): the sort of each key, the YAML read and written, and where it applies — the
//! policies crossed with the managed device's, a value of the managed device that wins, the keys of a project for the
//! team's projects alone (rank 2 under the repository's file, rank 5 over it, refused when changed).

use super::*;
use crate::core_tests::harness;
use crate::locks::{Managed, HARNESSES, PROXY_URL, TELEMETRY};
use escouade_team::settings::{Effective, Status, VersionView};
use serde_json::json;

fn map(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

/// A view of the team's settings, its version 1 in force, as `escouade-team` gives it.
fn view(effective: Effective) -> View {
    View {
        applied: Some(1),
        versions: vec![VersionView {
            number: 1,
            digest: "00".repeat(32),
            by: "Claire Martin".into(),
            author: "11".repeat(16),
            at: 1_800_000_000,
            note: String::new(),
            file: TeamFile::default(),
            signers: vec!["11".repeat(16)],
            need: 1,
            signed_by_me: false,
            status: Status::Applied,
        }],
        effective,
        ..Default::default()
    }
}

#[test]
fn each_key_has_its_sort_from_the_core() {
    let s = |k| CORE_KEYS.sort(k);
    assert_eq!(s("permissions.modes"), Some(Sort::Restrict));
    assert_eq!(s("telemetry.enabled"), Some(Sort::Restrict));
    assert_eq!(s("proxy.url"), Some(Sort::Sensitive));
    assert_eq!(s("proxy.authorities"), Some(Sort::Sensitive));
    assert_eq!(s("board.on_approve"), Some(Sort::Value));
    assert_eq!(s("worktrees.setup"), Some(Sort::Executable));
    assert_eq!(s("launch"), Some(Sort::Executable));
    assert_eq!(s("board.test_command"), Some(Sort::Executable));
    assert_eq!(s("plugins.install"), Some(Sort::Executable));
    assert_eq!(s("no.such.key"), None);
    assert!(is_policy("harnesses.allowed"));
    assert!(!is_policy("board.on_approve"));
}

#[test]
fn the_file_is_read_from_its_yaml_and_written_back() {
    let text = r#"
version: 1
defaults:
  board:
    merge_strategy: squash
enforced:
  harnesses:
    allowed: [claude, codex]
  permissions.modes: [plan, default, acceptEdits]
  board:
    on_approve: pr
  telemetry.enabled: false
  hooks:
    preToolUse: "sh x"
proposed:
  worktrees:
    setup: [npm ci]
reasons:
  harnesses.allowed: "Seuls les fournisseurs sous contrat DPA."
quorum: true
"#;
    let (file, warnings) = parse(text).unwrap();
    assert_eq!(
        file.defaults,
        map(&[("board.merge_strategy", json!("squash"))])
    );
    assert_eq!(
        file.enforced.get("harnesses.allowed"),
        Some(&json!(["claude", "codex"]))
    );
    assert_eq!(
        file.enforced.get("permissions.modes"),
        Some(&json!(["plan", "default", "acceptEdits"]))
    );
    assert_eq!(file.enforced.get("board.on_approve"), Some(&json!("pr")));
    assert_eq!(file.enforced.get("telemetry.enabled"), Some(&json!(false)));
    // A key nobody knows is kept (what never enters reads it) and said.
    assert_eq!(file.enforced.get("hooks.preToolUse"), Some(&json!("sh x")));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("hooks.preToolUse"));
    assert_eq!(
        file.proposed.get("worktrees.setup"),
        Some(&json!(["npm ci"]))
    );
    assert_eq!(file.quorum, Some(true));
    assert_eq!(
        file.reasons["harnesses.allowed"],
        "Seuls les fournisseurs sous contrat DPA."
    );
    // Written back and read again, the same.
    let (again, _) = parse(&to_yaml(&file)).unwrap();
    assert_eq!(again, file);
    // An unreadable file, a version of tomorrow.
    assert!(parse("version: 2\n").is_err());
    assert!(parse("enforced: [").is_err());
    assert!(parse("quorum: maybe\n").is_err());
}

#[tokio::test]
async fn the_team_applies_to_its_projects_alone_crossed_with_the_managed_device() {
    let h = harness("team-settings-layers");
    let (a, _) = h.project(false).await;
    let b = h.second_project().await;
    // The managed device allows claude and codex, and sets its own proxy.
    h.core.set_managed(Managed {
        values: [
            (
                HARNESSES.to_owned(),
                (
                    json!(["claude", "codex"]),
                    "the machine's policy".to_owned(),
                ),
            ),
            (
                PROXY_URL.to_owned(),
                (
                    json!("http://managed:8080"),
                    "the machine's policy".to_owned(),
                ),
            ),
        ]
        .into_iter()
        .collect(),
        warnings: Vec::new(),
    });
    // The team allows codex and copilot; it consented proxy (this device accepted it) loses to the managed one.
    let effective = Effective {
        enforced: map(&[
            (HARNESSES, json!(["codex", "copilot"])),
            (TELEMETRY, json!(false)),
            (PROXY_URL, json!("http://team:3128")),
            ("git.agent_branch_prefix", json!("acme/")),
        ]),
        defaults: map(&[("board.merge_strategy", json!("rebase"))]),
        ..Default::default()
    };
    h.core
        .apply_team_settings("acme", "Acme", Some(&view(effective)));
    h.core.set_project_team(&a.id, Some("acme".into())).unwrap();

    // The bound crossed for the team's project: codex alone; the other project keeps the device's.
    let la = h.core.locks_for(&a.id);
    let lb = h.core.locks_for(&b.id);
    assert_eq!(
        la.get(HARNESSES).and_then(|l| l.allowed()),
        Some(vec!["codex".to_owned()])
    );
    assert_eq!(
        lb.get(HARNESSES).and_then(|l| l.allowed()),
        Some(vec!["claude".to_owned(), "codex".to_owned()])
    );
    let why = la.check_engine("claude").unwrap_err().to_string();
    assert!(why.contains("Acme"), "{why}");
    assert!(lb.check_engine("claude").is_ok());
    // A value: the managed device's wins.
    assert_eq!(la.text(PROXY_URL), Some("http://managed:8080"));
    // A key of the app: it holds for the app (no spaces yet), imposed by the team.
    assert_eq!(h.core.locks.read().value(TELEMETRY), Some(&json!(false)));
    assert!(h
        .core
        .locks
        .read()
        .get(HARNESSES)
        .is_some_and(|l| l.bounds.len() == 1));

    // The keys of a project: rank 5 over the file, rank 2 where nothing else sets it; the other project untouched.
    let pa = h.core.project(&a.id).unwrap();
    let pb = h.core.project(&b.id).unwrap();
    assert_eq!(pa.agent_branch_prefix, "acme/");
    assert_eq!(
        crate::projectfile::get(&pa, "board.merge_strategy"),
        json!("rebase")
    );
    assert_ne!(pb.agent_branch_prefix, "acme/");
    // What the team imposes is refused when changed by the window.
    let mut changed = pa.clone();
    changed.agent_branch_prefix = "mine/".into();
    let why = h.core.update_project(changed).unwrap_err().to_string();
    assert!(why.contains("Acme"), "{why}");
    // What it recommends, the device changes.
    let mut mine = h.core.project(&a.id).unwrap();
    crate::projectfile::set(&mut mine, "board.merge_strategy", &json!("squash"));
    h.core.update_project(mine).unwrap();

    // No longer of the team: its locks are the app's again.
    h.core.set_project_team(&a.id, None).unwrap();
    assert_eq!(
        h.core
            .locks_for(&a.id)
            .get(HARNESSES)
            .and_then(|l| l.allowed()),
        Some(vec!["claude".to_owned(), "codex".to_owned()])
    );
    // The device leaves the team: nothing of it holds any more.
    h.core.apply_team_settings("acme", "Acme", None);
    assert!(h.core.locks.read().value(TELEMETRY).is_none());
    assert!(h.core.team_locks.read().is_empty());
}
