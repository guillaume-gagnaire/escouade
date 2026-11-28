//! Tests of the canvas summary (`canvas.rs`): the skills and plugins of an agent and of its subagents.

use crate::canvas::*;
use crate::plan::Change;
use serde_json::json;

#[test]
fn a_skill_is_named_by_its_input_and_a_plugin_by_its_server() {
    assert_eq!(
        kit_of("Skill", &json!({"skill": " superpowers:writing-plans "})),
        Some((KitKind::Skill, "superpowers:writing-plans".into()))
    );
    assert_eq!(
        kit_of("mcp__github__get_pr", &json!({})),
        Some((KitKind::Plugin, "github".into()))
    );
    assert_eq!(kit_of("Read", &json!({})), None);
    assert_eq!(kit_of("Skill", &json!({})), None);
    assert_eq!(kit_of("mcp____x", &json!({})), None);
}

#[test]
fn counts_the_calls_and_says_which_one_runs() {
    let mut c = Canvas::default();
    assert_eq!(
        c.on_tool_use(None, "k1", "mcp__github__a", &json!({}), 1),
        Change::Saved
    );
    assert_eq!(
        c.on_tool_use(None, "k2", "mcp__github__b", &json!({}), 2),
        Change::View
    );
    assert_eq!((c.tools[0].calls, c.tools[0].running), (2, 2));
    assert_eq!(c.on_tool_result("k1", false, "", 3), Change::View);
    assert_eq!(c.tools[0].running, 1);
    assert_eq!(c.on_tool_result("nope", false, "", 3), Change::None);
}

#[test]
fn a_call_seen_twice_counts_once() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 1);
    assert_eq!(
        c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 1),
        Change::None
    );
    assert_eq!(c.tools[0].calls, 1);
}

#[test]
fn keeps_the_tools_of_each_subagent_apart() {
    let mut c = Canvas::default();
    c.on_tool_use(Some("a1"), "k1", "Skill", &json!({"skill": "x"}), 1);
    assert!(c.tools.is_empty());
    assert_eq!(c.subs[0].call, "a1");
    assert_eq!(c.subs[0].tools[0].name, "x");
    assert_eq!(c.on_tool_result("k1", false, "", 2), Change::View);
    assert_eq!(c.subs[0].tools[0].running, 0);
}

#[test]
fn nothing_runs_after_a_reload_or_the_end_of_the_process() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 1);
    let back: Canvas = serde_json::from_value(serde_json::to_value(&c).unwrap()).unwrap();
    assert_eq!((back.tools[0].calls, back.tools[0].running), (1, 0));
    assert_eq!(c.close_running(9), Change::View);
    assert_eq!(c.tools[0].running, 0);
    assert_eq!(c.close_running(9), Change::None);
    assert_eq!(c.on_tool_result("k1", false, "", 10), Change::None);
}

#[test]
fn keeps_at_most_40_tools_a_thread() {
    let mut c = Canvas::default();
    for i in 0..45 {
        c.on_tool_use(
            None,
            &format!("k{i}"),
            &format!("mcp__s{i}__t"),
            &json!({}),
            i,
        );
    }
    assert_eq!(c.tools.len(), MAX_KIT);
    assert!(c.tools.iter().all(|t| t.name != "s0"));
}

#[test]
fn keeps_at_most_30_subagent_threads() {
    let mut c = Canvas::default();
    for i in 0..32 {
        c.on_tool_use(
            Some(&format!("a{i}")),
            &format!("k{i}"),
            "Skill",
            &json!({"skill": "x"}),
            i,
        );
    }
    assert_eq!(c.subs.len(), MAX_THREADS);
    assert_eq!(c.subs[0].call, "a2");
    assert!(!c.is_empty());
}

#[test]
fn a_server_named_with_two_underscores_is_kept_whole() {
    assert_eq!(
        kit_of("mcp__my__srv__get", &json!({})),
        Some((KitKind::Plugin, "my__srv".into()))
    );
    assert_eq!(
        kit_of("mcp__plugin_superpowers_chrome__click", &json!({})),
        Some((KitKind::Plugin, "plugin_superpowers_chrome".into()))
    );
}

#[test]
fn keeps_the_time_of_the_first_call() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 3);
    c.on_tool_use(None, "k2", "Skill", &json!({"skill": "x"}), 8);
    assert_eq!((c.tools[0].first_at, c.tools[0].last_at), (3, 8));
}

#[test]
fn the_caps_spare_what_runs() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "run", "mcp__s0__t", &json!({}), 0);
    for i in 1..MAX_KIT as i64 {
        let id = format!("k{i}");
        c.on_tool_use(None, &id, &format!("mcp__s{i}__t"), &json!({}), i);
        c.on_tool_result(&id, false, "", i);
    }
    c.on_tool_use(None, "new", "mcp__new__t", &json!({}), 99);
    assert!(c.tools.iter().any(|t| t.name == "s0"));
    assert!(c.tools.iter().all(|t| t.name != "s1"));

    let mut c = Canvas::default();
    c.on_tool_use(Some("a0"), "run", "Skill", &json!({"skill": "x"}), 0);
    for i in 1..=MAX_THREADS {
        let id = format!("k{i}");
        c.on_tool_use(
            Some(&format!("a{i}")),
            &id,
            "Skill",
            &json!({"skill": "x"}),
            i as i64,
        );
        c.on_tool_result(&id, false, "", i as i64);
    }
    assert_eq!(c.subs.len(), MAX_THREADS);
    assert_eq!(
        (c.subs[0].call.as_str(), c.subs[1].call.as_str()),
        ("a0", "a2")
    );
}

const URL: &str = "https://claude.ai/artifact/AbC_12-x";

#[test]
fn finds_the_address_an_artifact_was_published_at() {
    assert_eq!(
        artifact_url(&format!("Published to {URL}.")),
        Some(URL.into())
    );
    assert_eq!(
        artifact_url("see https://team.claude.ai/artifact/z9"),
        Some("https://team.claude.ai/artifact/z9".into())
    );
    assert_eq!(artifact_url("x/https://claude.ai/artifact/z9"), None);
    assert_eq!(artifact_url("nothing"), None);
}

#[test]
fn a_page_published_again_is_one_artifact() {
    let mut c = Canvas::default();
    c.on_tool_use(
        None,
        "p1",
        "Artifact",
        &json!({"file_path": "/tmp/report.html"}),
        1,
    );
    assert_eq!(c.publishing[0].name, "report.html");
    assert_eq!(
        c.on_tool_result("p1", false, &format!("Published {URL}"), 2),
        Change::Saved
    );
    for (i, call) in ["p2", "p3"].iter().enumerate() {
        c.on_tool_use(
            None,
            call,
            "Artifact",
            &json!({"title": "Rapport", "file_path": "/tmp/report.html"}),
            3 + i as i64,
        );
        c.on_tool_result(call, false, &format!("Updated {URL}"), 4 + i as i64);
    }
    assert!(c.publishing.is_empty());
    assert_eq!(c.artifacts.len(), 1);
    assert_eq!(
        (c.artifacts[0].versions, c.artifacts[0].title.as_str()),
        (3, "Rapport")
    );
}

#[test]
fn a_failed_or_other_artifact_call_lands_nothing() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "p1", "Artifact", &json!({"title": "x"}), 1);
    c.on_tool_result("p1", true, URL, 2);
    c.on_tool_use(None, "p2", "Artifact", &json!({"action": "list"}), 3);
    assert!(c.artifacts.is_empty() && c.publishing.is_empty());
}

#[test]
fn keeps_the_last_commands_of_each_thread() {
    let mut c = Canvas::default();
    for i in 0..7 {
        c.on_tool_use(
            None,
            &format!("b{i}"),
            "Bash",
            &json!({"command": format!("echo {i}")}),
            i,
        );
    }
    c.on_tool_use(
        Some("a1"),
        "sb",
        "Bash",
        &json!({"command": " cargo test "}),
        20,
    );
    c.on_tool_use(
        None,
        "bg",
        "Bash",
        &json!({"command": "npm run dev", "run_in_background": true}),
        21,
    );
    assert_eq!(c.commands.len(), MAX_COMMANDS);
    assert_eq!(c.commands[0].command, "echo 2");
    assert_eq!(
        (
            c.subs[0].call.as_str(),
            c.subs[0].commands[0].command.as_str()
        ),
        ("a1", "cargo test")
    );
}

#[test]
fn a_command_ends_failed_or_not_by_its_result_and_is_cut_on_a_character() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "b1", "Bash", &json!({"command": "é".repeat(400)}), 1);
    c.on_tool_use(Some("a1"), "b2", "Bash", &json!({"command": "npm test"}), 2);
    assert_eq!(c.commands[0].command.chars().count(), MAX_COMMAND);
    assert_eq!(
        c.on_tool_result("b1", true, "exit code 101", 5),
        Change::View
    );
    c.on_tool_result("b2", false, "", 6);
    assert_eq!(
        (c.commands[0].ended_at, c.commands[0].failed),
        (Some(5), true)
    );
    assert_eq!(
        (c.subs[0].commands[0].ended_at, c.subs[0].commands[0].failed),
        (Some(6), false)
    );
}

#[test]
fn a_command_cut_short_by_a_restart_ends_stopped_not_failed() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "b1", "Bash", &json!({"command": "cargo test"}), 1);
    c.on_tool_use(Some("s1"), "b2", "Bash", &json!({"command": "npm test"}), 1);
    c.close_running(5);
    for run in [&c.commands[0], &c.subs[0].commands[0]] {
        assert_eq!(
            (run.ended_at, run.failed, run.stopped),
            (Some(5), false, true)
        );
    }
    // Saved before it knew the word: not stopped.
    let old: CommandRun =
        serde_json::from_value(json!({"call": "b", "command": "x", "failed": true})).unwrap();
    assert!(!old.stopped);
}

#[test]
fn a_summary_with_only_artifacts_or_commands_is_not_empty() {
    let mut c = Canvas::default();
    c.on_tool_use(Some("a1"), "b1", "Bash", &json!({"command": "ls"}), 1);
    assert!(!c.is_empty());
}

#[test]
fn a_tool_saved_before_its_first_call_was_kept_starts_at_its_last() {
    let c: Canvas = serde_json::from_value(json!({
        "tools": [{"kind": "skill", "name": "a", "calls": 2, "lastAt": 7}],
        "subs": [{"call": "s", "tools": [{"kind": "plugin", "name": "b", "firstAt": 3, "lastAt": 9}]}]
    }))
    .unwrap();
    assert_eq!(c.tools[0].first_at, 7);
    assert_eq!(c.subs[0].tools[0].first_at, 3);
}

#[test]
fn an_asset_uploaded_with_artifact_is_no_version_of_the_page() {
    let mut c = Canvas::default();
    let asset =
        json!({"url": "https://claude.ai/artifact/x", "file_path": "logo.png", "asset": true});
    assert_eq!(
        c.on_tool_use(None, "a1", "Artifact", &asset, 1),
        Change::None
    );
    let assets = json!({"url": "https://claude.ai/artifact/x", "file_paths": ["a.png", "b.png"]});
    assert_eq!(
        c.on_tool_use(None, "a2", "Artifact", &assets, 2),
        Change::None
    );
    assert!(c.publishing.is_empty());
}

#[test]
fn a_terminal_call_read_again_after_a_restart_is_one_run() {
    let mut c = Canvas::default();
    let bash = json!({"command": "npm test"});
    c.on_tool_use(None, "b1", "Bash", &bash, 1);
    c.on_tool_use(Some("s1"), "b2", "Bash", &bash, 1);
    // The app restarted: what it remembered of the calls it saw is gone, the canvas is read back.
    let mut c: Canvas = serde_json::from_value(serde_json::to_value(&c).unwrap()).unwrap();
    assert_eq!(c.on_tool_use(None, "b1", "Bash", &bash, 5), Change::None);
    assert_eq!(
        c.on_tool_use(Some("s1"), "b2", "Bash", &bash, 5),
        Change::None
    );
    assert_eq!(c.commands.len(), 1);
    assert_eq!(c.subs[0].commands.len(), 1);
}

#[test]
fn a_command_is_kept_and_sent_with_its_secrets_masked() {
    let gh = format!("ghp_{}", "a".repeat(36));
    let typed = format!(
        "curl -H 'Authorization: Bearer abc.def' -u me:hunter2 'https://x/?token=t0k3n' && GH_TOKEN={gh} gh pr list --password p4ss"
    );
    let mut c = Canvas::default();
    c.on_tool_use(None, "b1", "Bash", &json!({ "command": typed }), 1);
    c.on_tool_use(
        Some("a1"),
        "b2",
        "Bash",
        &json!({"command": "psql --password=s3cr3t"}),
        2,
    );
    let kept = &c.commands[0].command;
    assert_eq!(
        kept,
        "curl -H 'Authorization: Bearer ***' -u me:*** 'https://x/?token=***' && GH_TOKEN=*** gh pr list --password ***"
    );
    assert_eq!(c.subs[0].commands[0].command, "psql --password=***");
    // Neither in the agent's file nor in what goes to the window.
    let sent = serde_json::to_string(&c).unwrap();
    for secret in ["abc.def", "hunter2", "t0k3n", gh.as_str(), "p4ss", "s3cr3t"] {
        assert!(!sent.contains(secret), "{secret} in {sent}");
    }
}

#[test]
fn a_secret_is_masked_before_the_command_is_cut() {
    // Cut first, the token would lose its tail and its form: masked first, nothing of it stays.
    let gh = format!("ghp_{}", "b".repeat(36));
    let long = format!("{} {gh}", "x".repeat(MAX_COMMAND - 10));
    let mut c = Canvas::default();
    c.on_tool_use(None, "b1", "Bash", &json!({ "command": long }), 1);
    assert!(!c.commands[0].command.contains("ghp_"));
}

#[test]
fn a_command_kept_in_the_clear_before_is_masked_as_it_is_read() {
    let old: CommandRun =
        serde_json::from_value(json!({"call": "b", "command": "export API_TOKEN=s3cr3t && make"}))
            .unwrap();
    assert_eq!(old.command, "export API_TOKEN=*** && make");
}
