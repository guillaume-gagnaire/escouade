//! The server declared in Claude Code: `claude mcp add` and `remove` run with the fake `claude`,
//! which keeps the user scope's servers in the `.claude.json` of a folder of the test's.

use super::install::{
    current, declare, manual_command_in, mask_bearer, wanted, withdraw, Declared, Entry, Target,
    COMMAND_TIMEOUT, MASK,
};
use crate::core_tests::{fake_cli, harness, second_account, Harness};
use crate::model::*;
use crate::paths::test_dir;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// An account's Claude Code of a test: the fake `claude`, its state in `<dir>/home`, its launches
/// logged in `<dir>/log.jsonl`.
fn target(name: &str) -> Target {
    let dir = test_dir(name);
    Target {
        program: fake_cli().into(),
        env: vec![
            // Blank, in case the tests run inside a Claude Code session with a folder of its own.
            ("CLAUDE_CONFIG_DIR".into(), String::new()),
            (
                "FAKE_CLAUDE_HOME".into(),
                dir.join("home").to_string_lossy().into(),
            ),
            (
                "FAKE_CLAUDE_LOG".into(),
                dir.join("log.jsonl").to_string_lossy().into(),
            ),
        ],
        claude_json: dir.join("home").join(".claude.json"),
        cwd: dir,
        timeout: COMMAND_TIMEOUT,
    }
}

/// The arguments of each launch of the fake `claude`, in order.
fn launches(t: &Target) -> Vec<Vec<String>> {
    std::fs::read_to_string(t.cwd.join("log.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|l| {
            let v: Value = serde_json::from_str(l).unwrap();
            serde_json::from_value(v["argv"].clone()).unwrap()
        })
        .collect()
}

fn entry(port: u16, token: &str) -> Entry {
    wanted(port, token)
}

/// What the file holds under the server's name, as JSON.
fn held(t: &Target) -> Value {
    let v: Value = serde_json::from_slice(&std::fs::read(&t.claude_json).unwrap()).unwrap();
    v["mcpServers"]["escouade"].clone()
}

fn write_json(file: &Path, v: &Value) {
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, serde_json::to_vec_pretty(v).unwrap()).unwrap();
}

#[test]
fn the_entry_wanted_and_the_one_claude_code_holds_are_told_from_its_json() {
    let t = target("mcp-install-read");
    let want = entry(47123, "tok-1");
    assert_eq!(want.url, "http://127.0.0.1:47123/mcp");
    assert_eq!(want.token, "tok-1");
    // No file, an empty one, one that is no JSON, no server of that name: no entry.
    assert_eq!(current(&t.claude_json), None);
    write_json(&t.claude_json, &json!({}));
    assert_eq!(current(&t.claude_json), None);
    std::fs::write(&t.claude_json, "{ not json").unwrap();
    assert_eq!(current(&t.claude_json), None);
    write_json(
        &t.claude_json,
        &json!({ "oauthAccount": { "emailAddress": "a@b.c" }, "mcpServers": { "autre": { "type": "http", "url": "http://x" } } }),
    );
    assert_eq!(current(&t.claude_json), None);
    // As `claude mcp add --transport http` writes it.
    write_json(
        &t.claude_json,
        &json!({ "mcpServers": { "escouade": {
            "type": "http",
            "url": "http://127.0.0.1:47123/mcp",
            "headers": { "Authorization": "Bearer tok-1" },
        } } }),
    );
    assert_eq!(current(&t.claude_json), Some(want.clone()));
    // An entry of that name in another shape is there, but is not the one wanted.
    for odd in [
        json!({ "type": "sse", "url": "http://127.0.0.1:47123/mcp", "headers": { "Authorization": "Bearer tok-1" } }),
        json!({ "type": "stdio", "command": "node" }),
        json!({ "type": "http", "url": "http://127.0.0.1:47123/mcp" }),
        json!({ "type": "http", "url": "http://127.0.0.1:47123/mcp", "headers": { "Authorization": "Basic abc" } }),
    ] {
        write_json(
            &t.claude_json,
            &json!({ "mcpServers": { "escouade": odd } }),
        );
        let found = current(&t.claude_json).expect("an entry");
        assert_ne!(found, want, "{odd}");
    }
}

#[tokio::test]
async fn an_account_without_the_server_gets_it_added_with_the_header_last() {
    let t = target("mcp-install-add");
    // The state Claude Code keeps beside its servers is kept.
    write_json(
        &t.claude_json,
        &json!({ "oauthAccount": { "emailAddress": "a@b.c" } }),
    );
    let done = declare(&t, &entry(47123, "tok-1")).await.unwrap();
    assert_eq!(done, Declared::Added);
    assert_eq!(
        launches(&t),
        [[
            "mcp",
            "add",
            "--scope",
            "user",
            "--transport",
            "http",
            "escouade",
            "http://127.0.0.1:47123/mcp",
            "--header",
            "Authorization: Bearer tok-1",
        ]]
    );
    assert_eq!(
        held(&t),
        json!({
            "type": "http",
            "url": "http://127.0.0.1:47123/mcp",
            "headers": { "Authorization": "Bearer tok-1" },
        })
    );
    assert_eq!(
        current(&t.claude_json),
        Some(entry(47123, "tok-1")),
        "what it reads is what was wanted"
    );
    let json: Value = serde_json::from_slice(&std::fs::read(&t.claude_json).unwrap()).unwrap();
    assert_eq!(json["oauthAccount"]["emailAddress"], "a@b.c");
}

#[tokio::test]
async fn the_server_declared_as_wanted_is_left_as_it_is_and_nothing_is_run() {
    let t = target("mcp-install-same");
    declare(&t, &entry(47123, "tok-1")).await.unwrap();
    assert_eq!(launches(&t).len(), 1);
    let again = declare(&t, &entry(47123, "tok-1")).await.unwrap();
    assert_eq!(again, Declared::Unchanged);
    assert_eq!(launches(&t).len(), 1, "no second launch");
}

#[tokio::test]
async fn a_server_declared_otherwise_is_removed_then_added() {
    let t = target("mcp-install-replace");
    declare(&t, &entry(47123, "tok-1")).await.unwrap();
    // Another port, then another token, then an entry of another kind.
    for next in [entry(47200, "tok-1"), entry(47200, "tok-2")] {
        let before = launches(&t).len();
        assert_eq!(declare(&t, &next).await.unwrap(), Declared::Replaced);
        let ran = &launches(&t)[before..];
        assert_eq!(ran.len(), 2, "{ran:?}");
        assert_eq!(ran[0], ["mcp", "remove", "escouade", "--scope", "user"]);
        assert_eq!(&ran[1][..2], ["mcp", "add"]);
        assert_eq!(current(&t.claude_json), Some(next));
    }
    write_json(
        &t.claude_json,
        &json!({ "mcpServers": { "escouade": { "type": "sse", "url": "http://127.0.0.1:47200/mcp" } } }),
    );
    assert_eq!(
        declare(&t, &entry(47200, "tok-2")).await.unwrap(),
        Declared::Replaced
    );
    assert_eq!(held(&t)["type"], "http");
}

#[tokio::test]
async fn a_declaration_that_fails_says_why_and_leaves_nothing_half_done() {
    // Claude Code refuses.
    let t = target("mcp-install-refused");
    std::fs::create_dir_all(t.claude_json.parent().unwrap()).unwrap();
    std::fs::write(
        t.claude_json.parent().unwrap().join("fake-mcp-fail"),
        "Failed to write to config",
    )
    .unwrap();
    let e = declare(&t, &entry(47123, "tok-1")).await.unwrap_err();
    assert!(
        format!("{e:#}").contains("Failed to write to config"),
        "{e:#}"
    );
    assert_eq!(current(&t.claude_json), None);

    // Its `claude` is not there.
    let mut gone = target("mcp-install-gone");
    gone.program = gone.cwd.join("pas-de-claude");
    let e = declare(&gone, &entry(47123, "tok-1")).await.unwrap_err();
    assert!(format!("{e:#}").contains("claude"), "{e:#}");

    // It never answers: killed after the time given.
    let mut slow = target("mcp-install-slow");
    std::fs::create_dir_all(slow.claude_json.parent().unwrap()).unwrap();
    std::fs::write(slow.claude_json.parent().unwrap().join("fake-mcp-hang"), "").unwrap();
    slow.timeout = Duration::from_millis(1500);
    let started = std::time::Instant::now();
    let e = declare(&slow, &entry(47123, "tok-1")).await.unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(20));
    assert!(
        format!("{e:#}").contains("1 s") || format!("{e:#}").contains("1,5"),
        "{e:#}"
    );

    // An entry to replace that cannot be removed stays as it was: no add on top of it.
    let t = target("mcp-install-stuck");
    declare(&t, &entry(47123, "tok-1")).await.unwrap();
    std::fs::write(
        t.claude_json.parent().unwrap().join("fake-mcp-fail"),
        "locked",
    )
    .unwrap();
    let e = declare(&t, &entry(47123, "tok-2")).await.unwrap_err();
    assert!(format!("{e:#}").contains("locked"), "{e:#}");
    assert_eq!(current(&t.claude_json), Some(entry(47123, "tok-1")));
    assert_eq!(
        launches(&t).len(),
        2,
        "the add after the failed remove did not run"
    );
}

#[tokio::test]
async fn withdrawing_removes_the_entry_and_an_entry_already_absent_is_no_error() {
    let t = target("mcp-install-withdraw");
    // Nothing declared, nothing run.
    assert!(!withdraw(&t).await.unwrap());
    assert!(launches(&t).is_empty());
    declare(&t, &entry(47123, "tok-1")).await.unwrap();
    assert!(withdraw(&t).await.unwrap());
    assert_eq!(
        launches(&t).last().unwrap(),
        &["mcp", "remove", "escouade", "--scope", "user"]
    );
    assert_eq!(current(&t.claude_json), None);
    // Gone between the reading and the command: Claude Code answers an absent entry with an
    // error, which is no error here, the entry being gone.
    declare(&t, &entry(47123, "tok-1")).await.unwrap();
    let folder = t.claude_json.parent().unwrap();
    std::fs::write(folder.join("fake-mcp-raced"), "").unwrap();
    assert!(withdraw(&t).await.unwrap());
    assert_eq!(current(&t.claude_json), None);
    std::fs::remove_file(folder.join("fake-mcp-raced")).unwrap();
    // Refused by Claude Code with the entry still there: an error.
    declare(&t, &entry(47123, "tok-1")).await.unwrap();
    std::fs::write(folder.join("fake-mcp-fail"), "locked").unwrap();
    let e = withdraw(&t).await.unwrap_err();
    assert!(format!("{e:#}").contains("locked"), "{e:#}");
    assert!(current(&t.claude_json).is_some());
}

#[test]
fn a_token_is_hidden_wherever_it_could_be_printed() {
    // A {:?} of an entry (a failed assertion, a log line) does not print it.
    let e = entry(47123, "s3cr3t-token");
    assert!(!format!("{e:?}").contains("s3cr3t-token"), "{e:?}");
    assert!(format!("{e:?}").contains(MASK));
    // A command shown: the token goes, the rest stays, wherever the shell has it.
    let add = "claude mcp add --header \"Authorization: Bearer s3cr3t-token\"";
    assert_eq!(
        mask_bearer(add),
        format!("claude mcp add --header \"Authorization: Bearer {MASK}\"")
    );
    assert_eq!(
        mask_bearer("a 'Authorization: Bearer s3cr3t-token'; b Bearer other\nz"),
        format!("a 'Authorization: Bearer {MASK}'; b Bearer {MASK}\nz")
    );
    assert_eq!(
        mask_bearer("claude mcp remove escouade"),
        "claude mcp remove escouade"
    );
}

#[tokio::test]
async fn what_claude_code_says_never_carries_the_token_into_the_error() {
    let t = target("mcp-install-leak");
    std::fs::create_dir_all(t.claude_json.parent().unwrap()).unwrap();
    // Claude Code says what it was given: the header, and the token on its own.
    std::fs::write(
        t.claude_json.parent().unwrap().join("fake-mcp-fail"),
        "Invalid header Authorization: Bearer s3cr3t-token (token s3cr3t-token)",
    )
    .unwrap();
    let e = declare(&t, &entry(47123, "s3cr3t-token"))
        .await
        .unwrap_err();
    let said = format!("{e:#}");
    assert!(!said.contains("s3cr3t-token"), "{said}");
    assert!(said.contains("Invalid header"), "{said}");
    // The token of the entry being replaced is hidden as well, when the removal says it.
    let t = target("mcp-install-leak-old");
    declare(&t, &entry(47123, "old-token-1")).await.unwrap();
    std::fs::write(
        t.claude_json.parent().unwrap().join("fake-mcp-fail"),
        "cannot remove old-token-1",
    )
    .unwrap();
    let e = declare(&t, &entry(47123, "new-token-2")).await.unwrap_err();
    assert!(!format!("{e:#}").contains("old-token-1"), "{e:#}");
    // And withdrawing.
    let e = withdraw(&t).await.unwrap_err();
    assert!(!format!("{e:#}").contains("old-token-1"), "{e:#}");
}

#[tokio::test]
async fn a_file_that_cannot_be_read_may_hold_the_entry_so_the_command_is_run_all_the_same() {
    let t = target("mcp-install-unreadable");
    // No file: nothing to take out, nothing run.
    assert!(!withdraw(&t).await.unwrap());
    assert!(launches(&t).is_empty());
    // A file that is no JSON: nothing can be told of it, so Claude Code is asked, and its failure is one.
    std::fs::create_dir_all(t.claude_json.parent().unwrap()).unwrap();
    std::fs::write(&t.claude_json, "{ not json").unwrap();
    assert!(withdraw(&t).await.is_err());
    assert_eq!(
        launches(&t),
        [["mcp", "remove", "escouade", "--scope", "user"]]
    );
    // A file that is JSON and holds no entry: nothing run.
    write_json(&t.claude_json, &json!({ "mcpServers": {} }));
    assert!(!withdraw(&t).await.unwrap());
    assert_eq!(launches(&t).len(), 1);
}

#[test]
fn the_command_to_declare_it_by_hand_reads_in_the_shell_of_the_platform() {
    let e = entry(47123, "tok-1");
    let add = "mcp add --scope user --transport http escouade http://127.0.0.1:47123/mcp --header \"Authorization: Bearer tok-1\"";
    // Principal: no folder to name.
    assert_eq!(
        manual_command_in(true, "claude", None, &e, false),
        format!("claude {add}")
    );
    assert_eq!(
        manual_command_in(false, "claude", None, &e, false),
        format!("claude {add}")
    );
    // An account of its own: its folder, without leaving it set in the user's shell.
    assert_eq!(
        manual_command_in(true, "claude", Some(r"C:\Users\ada\.escouade\claude\pro"), &e, false),
        format!(
            "$old = $env:CLAUDE_CONFIG_DIR; $env:CLAUDE_CONFIG_DIR = 'C:\\Users\\ada\\.escouade\\claude\\pro'; claude {add}; $env:CLAUDE_CONFIG_DIR = $old"
        )
    );
    assert_eq!(
        manual_command_in(
            false,
            "claude",
            Some("/Users/ada/.escouade/claude/pro"),
            &e,
            false
        ),
        format!("CLAUDE_CONFIG_DIR='/Users/ada/.escouade/claude/pro' claude {add}")
    );
    // An entry there already: removed first (the shell goes on whether or not it was there).
    assert_eq!(
        manual_command_in(false, "claude", Some("/p"), &e, true),
        format!(
            "CLAUDE_CONFIG_DIR='/p' claude mcp remove escouade --scope user; CLAUDE_CONFIG_DIR='/p' claude {add}"
        )
    );
    assert_eq!(
        manual_command_in(true, "claude", None, &e, true),
        format!("claude mcp remove escouade --scope user; claude {add}")
    );
    // A `claude` named by its path, spaces and quotes included.
    assert_eq!(
        manual_command_in(true, r"C:\Program Files\claude\claude.exe", None, &e, false),
        format!(r"& 'C:\Program Files\claude\claude.exe' {add}")
    );
    assert_eq!(
        manual_command_in(false, "/opt/it's/claude", None, &e, false),
        format!(r"'/opt/it'\''s/claude' {add}")
    );
}

// ---------- in the app: each active account, as the settings change ----------

/// The data folder: where the declarations run, and so where the fake `claude` logs them.
fn data(h: &Harness) -> PathBuf {
    h.dir.join("data")
}

/// The `claude mcp …` launches so far, their arguments.
fn mcp_launches(h: &Harness) -> Vec<Vec<String>> {
    h.launches(&data(h))
        .into_iter()
        .filter(|a| a.first().is_some_and(|c| c == "mcp"))
        .collect()
}

/// Principal's `.claude.json` in a test: the home the core gives the fake `claude`.
fn principal_json(h: &Harness) -> PathBuf {
    data(h).join("claude-home").join(".claude.json")
}

/// The `.claude.json` of the second account `second_account` makes.
fn pro_json(h: &Harness) -> PathBuf {
    h.dir.join("claude-pro").join(".claude.json")
}

fn runs(h: &Harness) -> usize {
    h.core.mcp.runs.load(Ordering::Acquire)
}

/// Waits until a declaration run that began after `before` is over.
async fn run_after(h: &Harness, before: usize) {
    h.wait("the declaration in Claude Code", |h| runs(h) > before)
        .await;
}

/// The entry the app wants everywhere: the server's port and token as they now are.
fn now_wanted(h: &Harness) -> Entry {
    wanted(
        h.core.mcp.status().port,
        &h.core.mcp.external_token().unwrap(),
    )
}

/// A third account, switched off, with a folder of its own.
fn off_account(h: &Harness) -> PathBuf {
    let dir = h.dir.join("claude-off");
    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: "off".into(),
        name: "Off".into(),
        config_dir: dir.to_string_lossy().into(),
        active: false,
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    dir
}

#[tokio::test]
async fn turning_it_on_declares_the_server_in_each_active_account_as_that_account_runs_claude() {
    let h = harness("mcp-install-on");
    second_account(&h);
    let off = off_account(&h);
    // Off, nothing is declared anywhere.
    assert!(mcp_launches(&h).is_empty());
    assert!(h.core.mcp.declared().is_empty());

    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    assert!(h.core.mcp.status().running);
    let declared = h.core.mcp.declared();
    assert_eq!(
        declared
            .iter()
            .map(|d| (d.account.as_str(), d.ok, d.error.as_deref()))
            .collect::<Vec<_>>(),
        [("principal", true, None), ("pro", true, None)]
    );
    // Both hold the server, on its port, with its token; the one switched off holds nothing.
    let want = now_wanted(&h);
    assert_eq!(current(&principal_json(&h)), Some(want.clone()));
    assert_eq!(current(&pro_json(&h)), Some(want.clone()));
    assert!(!off.join(".claude.json").exists());
    // Each ran in the data folder, with its own folder for `claude` (Principal's: none, whatever
    // the app's own variable).
    let adds: Vec<Value> = h
        .launch_log(&data(&h))
        .into_iter()
        .filter(|l| l["argv"][0] == "mcp")
        .collect();
    assert_eq!(adds.len(), 2, "{adds:?}");
    // (Blank, not unset: the tests blank the variable in case they run inside a Claude Code session.)
    assert!(
        adds[0]["configDir"].is_null() || adds[0]["configDir"] == "",
        "{adds:?}"
    );
    let pro_dir = h.dir.join("claude-pro").to_string_lossy().to_string();
    assert_eq!(adds[1]["configDir"], pro_dir.as_str());
    for l in &adds {
        assert_eq!(l["argv"][1], "add", "{l}");
        assert_eq!(
            l["argv"][9],
            format!("Authorization: Bearer {}", want.token)
        );
    }
    // The window was told, and the token is nowhere in what it is told or can ask for: the
    // commands it shows have it hidden.
    let told = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "mcpDeclared")
        .cloned()
        .expect("an event");
    assert_eq!(told["declared"][0]["account"], "principal");
    assert_eq!(told["declared"][0]["ok"], true);
    assert_eq!(told["declared"][0]["error"], Value::Null);
    let shown = told["declared"][1]["command"].as_str().unwrap();
    assert!(shown.contains(&format!("Bearer {MASK}")), "{shown}");
    assert!(shown.contains("mcp add --scope user"), "{shown}");
    let everything = serde_json::to_string(&*h.events.lock()).unwrap();
    assert!(
        !everything.contains(&want.token),
        "a token went to the window"
    );
    let status = serde_json::to_string(&h.core.mcp.declared()).unwrap();
    assert!(!status.contains(&want.token), "{status}");
    // The real command is built when it is asked for (« Copier la commande »).
    let real = h.core.mcp_manual_command("pro").unwrap();
    assert!(real.contains(&format!("Bearer {}", want.token)), "{real}");
    assert!(real.contains(&want.url), "{real}");

    // The app starts again with it on: what is there as wanted is left alone.
    let launched = mcp_launches(&h).len();
    h.core.mcp.stop();
    let before = runs(&h);
    h.core.start_mcp();
    run_after(&h, before).await;
    assert_eq!(h.core.mcp.status().port, h.core.settings.read().mcp_port);
    assert_eq!(mcp_launches(&h).len(), launched, "nothing was run");
    assert_eq!(h.core.mcp.declared(), declared);
    h.core.shutdown();
}

#[tokio::test]
async fn an_account_added_or_switched_on_later_receives_the_declaration_and_one_switched_off_keeps_it(
) {
    let h = harness("mcp-install-later");
    let off = off_account(&h);
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    assert_eq!(h.core.mcp.declared().len(), 1);

    // Added: declared in it at once.
    let before = runs(&h);
    second_account(&h);
    run_after(&h, before).await;
    assert_eq!(h.core.mcp.declared().len(), 2);
    assert_eq!(current(&pro_json(&h)), Some(now_wanted(&h)));

    // Switched on later (its folder is there, with nothing in it).
    let before = runs(&h);
    let mut account = h
        .core
        .settings
        .read()
        .accounts
        .iter()
        .find(|a| a.id == "off")
        .cloned()
        .unwrap();
    account.active = true;
    h.core.update_claude_account(account.clone()).unwrap();
    run_after(&h, before).await;
    assert_eq!(h.core.mcp.declared().len(), 3);
    assert_eq!(current(&off.join(".claude.json")), Some(now_wanted(&h)));

    // Switched off again: no longer listed, and its entry is left where it is (it works when it
    // is used by hand), then declared as it stood when it is switched back on, with no command.
    let before = runs(&h);
    account.active = false;
    h.core.update_claude_account(account.clone()).unwrap();
    run_after(&h, before).await;
    assert_eq!(h.core.mcp.declared().len(), 2);
    assert!(current(&off.join(".claude.json")).is_some());
    let launched = mcp_launches(&h).len();
    let before = runs(&h);
    account.active = true;
    h.core.update_claude_account(account).unwrap();
    run_after(&h, before).await;
    assert_eq!(h.core.mcp.declared().len(), 3);
    assert_eq!(mcp_launches(&h).len(), launched);
    h.core.shutdown();
}

#[tokio::test]
async fn a_port_that_changes_is_declared_again_in_every_account() {
    let h = harness("mcp-install-port");
    second_account(&h);
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let port = h.core.mcp.status().port;
    let launched = mcp_launches(&h).len();

    // The server stops, and its port is taken when it starts again: another one is chosen.
    h.core.mcp.stop();
    let taken = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
    let before = runs(&h);
    h.core.sync_mcp();
    run_after(&h, before).await;
    let moved = h.core.mcp.status().port;
    assert_ne!(moved, port);
    for file in [principal_json(&h), pro_json(&h)] {
        let e = current(&file).unwrap();
        assert_eq!(e.url, format!("http://127.0.0.1:{moved}/mcp"), "{file:?}");
        assert_eq!(e.token, h.core.mcp.external_token().unwrap());
    }
    // Replaced: removed, then added, in each.
    let ran = &mcp_launches(&h)[launched..];
    let verbs: Vec<&str> = ran.iter().map(|a| a[1].as_str()).collect();
    assert_eq!(verbs, ["remove", "add", "remove", "add"], "{ran:?}");
    drop(taken);
    h.core.shutdown();
}

#[tokio::test]
async fn an_account_that_fails_is_told_with_the_command_to_run_by_hand_and_the_others_are_declared()
{
    let h = harness("mcp-install-failure");
    second_account(&h);
    let folder = h.dir.join("claude-pro");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("fake-mcp-fail"), "Failed to write to config").unwrap();
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let declared = h.core.mcp.declared();
    assert_eq!(declared.len(), 2);
    assert!(declared[0].ok && declared[0].error.is_none());
    assert_eq!(current(&principal_json(&h)), Some(now_wanted(&h)));
    let pro = &declared[1];
    assert!(!pro.ok);
    assert!(
        pro.error
            .as_deref()
            .unwrap()
            .contains("Failed to write to config"),
        "{pro:?}"
    );
    // What it is told is the command with its token hidden; the real one (the real token, the
    // account's folder) is built when the user copies it.
    let token = h.core.mcp.external_token().unwrap();
    let port = h.core.mcp.status().port;
    assert!(!pro.command.contains(&token), "{pro:?}");
    assert!(pro.command.contains(&format!("Bearer {MASK}")), "{pro:?}");
    assert!(pro.command.contains("CLAUDE_CONFIG_DIR"));
    assert!(pro.command.contains(&*folder.to_string_lossy()));
    let real = h.core.mcp_manual_command("pro").unwrap();
    assert!(real.contains(&format!("Bearer {token}")), "{real}");
    assert!(
        real.contains(&format!("http://127.0.0.1:{port}/mcp")),
        "{real}"
    );
    assert!(real.contains("CLAUDE_CONFIG_DIR"));
    assert!(real.contains(&*folder.to_string_lossy()));
    assert!(real.contains("mcp add --scope user --transport http escouade"));
    assert_eq!(mask_bearer(&real), pro.command);
    // The error Claude Code gave is shown as it said it.
    assert!(!format!("{pro:?}").contains(&token));
    // Nothing was written in it.
    assert_eq!(current(&pro_json(&h)), None);

    // Saving the settings again tries again (the folder's trouble gone): only that account is run.
    std::fs::remove_file(folder.join("fake-mcp-fail")).unwrap();
    let launched = mcp_launches(&h).len();
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    assert!(h.core.mcp.declared().iter().all(|d| d.ok));
    assert_eq!(current(&pro_json(&h)), Some(now_wanted(&h)));
    assert_eq!(mcp_launches(&h).len(), launched + 1);
    h.core.shutdown();
}

#[tokio::test]
async fn the_real_command_is_given_only_for_an_account_it_can_be_run_in_while_the_server_runs() {
    let h = harness("mcp-install-real-command");
    second_account(&h);
    // Off: no server, no token to give.
    let e = h.core.mcp_manual_command("pro").unwrap_err();
    assert!(format!("{e:#}").contains("ne tourne pas"), "{e:#}");
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let e = h.core.mcp_manual_command("nope").unwrap_err();
    assert!(format!("{e:#}").contains("« nope »"), "{e:#}");
    // Principal's has no folder to name.
    let principal = h.core.mcp_manual_command("principal").unwrap();
    assert!(!principal.contains("CLAUDE_CONFIG_DIR"), "{principal}");
    assert!(principal.contains(&now_wanted(&h).token));
    // It holds the entry already: the command takes it out first, then adds it.
    assert!(
        principal.contains("mcp remove escouade --scope user; ") && principal.contains("mcp add"),
        "{principal}"
    );
    h.core.shutdown();
}

#[tokio::test]
async fn an_account_whose_claude_is_missing_is_told_and_the_others_are_declared() {
    let h = harness("mcp-install-no-claude");
    let mut pro = second_account(&h);
    pro.claude_path = h.dir.join("pas-de-claude").to_string_lossy().into();
    h.core.update_claude_account(pro).unwrap();
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let declared = h.core.mcp.declared();
    assert!(declared[0].ok, "{declared:?}");
    assert!(!declared[1].ok);
    assert!(
        declared[1]
            .error
            .as_deref()
            .unwrap()
            .contains("introuvable"),
        "{declared:?}"
    );
    // Its command names the `claude` it was told to use.
    assert!(
        declared[1].command.contains("pas-de-claude"),
        "{declared:?}"
    );
    h.core.shutdown();
}

#[tokio::test]
async fn turning_it_off_takes_the_server_out_of_every_account_and_changes_the_token() {
    let h = harness("mcp-install-off");
    second_account(&h);
    let off = off_account(&h);
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let old = h.core.mcp.external_token().unwrap();
    // An entry left in the account that is switched off (it was on once).
    write_json(
        &off.join(".claude.json"),
        &json!({ "mcpServers": { "escouade": { "type": "http", "url": "http://127.0.0.1:1/mcp", "headers": { "Authorization": format!("Bearer {old}") } } } }),
    );
    assert!(h.core.mcp.caller(&old).is_some());

    let before = runs(&h);
    h.core.set_mcp_enabled(false).unwrap();
    run_after(&h, before).await;
    for file in [principal_json(&h), pro_json(&h), off.join(".claude.json")] {
        assert_eq!(current(&file), None, "{file:?}");
    }
    assert!(h.core.mcp.declared().is_empty());
    assert!(!h.core.mcp.status().running);
    // The old token opens nothing from now on; the new one is the one kept.
    let new = h.core.mcp.external_token().unwrap();
    assert_ne!(new, old);
    assert_eq!(h.core.mcp.caller(&old), None);
    assert_eq!(h.core.mcp.caller(&new), Some(super::Caller::External));
    assert_eq!(
        h.core
            .secrets
            .get(crate::integrations::secrets::MCP_ENTRY)
            .unwrap(),
        Some(new.clone())
    );

    // Turned on again: declared with the new token.
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    assert_eq!(current(&principal_json(&h)).unwrap().token, new);
    assert_eq!(current(&pro_json(&h)).unwrap().token, new);
    h.core.shutdown();
}

#[tokio::test]
async fn turning_it_off_changes_the_token_before_the_commands_that_take_the_entries_out() {
    let h = harness("mcp-install-off-order");
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let old = h.core.mcp.external_token().unwrap();
    // Claude Code is slow to remove: for two seconds the entry is still in the file.
    let folder = principal_json(&h).parent().unwrap().to_path_buf();
    std::fs::write(folder.join("fake-mcp-slow"), "2000").unwrap();
    let before = runs(&h);
    h.core.set_mcp_enabled(false).unwrap();
    h.wait("the token to change", |h| {
        h.core.mcp.external_token().unwrap() != old
    })
    .await;
    // The old token opens nothing, and the entry that holds it has not gone yet.
    assert_eq!(h.core.mcp.caller(&old), None);
    assert!(current(&principal_json(&h)).is_some());
    run_after(&h, before).await;
    assert_eq!(current(&principal_json(&h)), None);
    h.core.shutdown();
}

#[tokio::test]
async fn removing_an_account_takes_the_server_out_of_its_folder_and_leaves_the_token_to_the_others()
{
    let h = harness("mcp-install-remove-on");
    second_account(&h);
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let token = h.core.mcp.external_token().unwrap();
    assert_eq!(current(&pro_json(&h)), Some(now_wanted(&h)));

    // Removed: its folder stays on the disk, but the entry that holds the token does not.
    // Two runs follow: the declaration the settings call for, and the removal from its folder.
    let before = runs(&h);
    h.core.remove_claude_account("pro").unwrap();
    h.wait("both runs", |h| runs(h) >= before + 2).await;
    assert_eq!(current(&pro_json(&h)), None);
    assert!(h.dir.join("claude-pro").join(".claude.json").exists());
    let pro_dir = h.dir.join("claude-pro").to_string_lossy().to_string();
    let removals: Vec<Value> = h
        .launch_log(&data(&h))
        .into_iter()
        .filter(|l| l["argv"][0] == "mcp" && l["argv"][1] == "remove")
        .collect();
    assert_eq!(removals.len(), 1, "{removals:?}");
    assert_eq!(removals[0]["configDir"], pro_dir.as_str());
    assert_eq!(
        removals[0]["argv"],
        json!(["mcp", "remove", "escouade", "--scope", "user"])
    );
    // The others are left as they were, with the same token.
    assert_eq!(current(&principal_json(&h)), Some(now_wanted(&h)));
    assert_eq!(h.core.mcp.external_token().unwrap(), token);
    assert_eq!(h.core.mcp.declared().len(), 1);
    h.core.shutdown();
}

#[tokio::test]
async fn removing_an_account_while_it_is_off_takes_a_leftover_entry_out_too() {
    let h = harness("mcp-install-remove-off");
    let off = off_account(&h);
    second_account(&h);
    // Leftovers of when it was on: in the account that is switched off, and in another.
    let held = json!({ "mcpServers": { "escouade": { "type": "http", "url": "http://127.0.0.1:1/mcp", "headers": { "Authorization": "Bearer old-token" } } } });
    write_json(&off.join(".claude.json"), &held);
    // The one removed with no entry (and no file) is not run for.
    assert!(mcp_launches(&h).is_empty());
    let before = runs(&h);
    h.core.remove_claude_account("off").unwrap();
    run_after(&h, before).await;
    assert_eq!(current(&off.join(".claude.json")), None);
    let removals: Vec<Value> = h
        .launch_log(&data(&h))
        .into_iter()
        .filter(|l| l["argv"][0] == "mcp")
        .collect();
    assert_eq!(removals.len(), 1, "{removals:?}");
    assert_eq!(removals[0]["argv"][1], "remove");
    let off_dir = off.to_string_lossy().to_string();
    assert_eq!(removals[0]["configDir"], off_dir.as_str());
    // Off all along: no server, no token made, no declaration.
    assert!(!h.core.mcp.status().running);
    assert_eq!(
        h.core
            .secrets
            .get(crate::integrations::secrets::MCP_ENTRY)
            .unwrap(),
        None
    );
    // An account with nothing in its folder: nothing run.
    let before = runs(&h);
    h.core.remove_claude_account("pro").unwrap();
    run_after(&h, before).await;
    assert_eq!(mcp_launches(&h).len(), 1);
    h.core.shutdown();
}

#[tokio::test]
async fn a_token_changed_by_turning_it_off_and_on_before_the_run_is_the_one_declared() {
    let h = harness("mcp-install-flip");
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let old = h.core.mcp.external_token().unwrap();
    // Two changes at once: both runs end up declaring the settings as they are, the new token.
    let before = runs(&h);
    h.core.set_mcp_enabled(false).unwrap();
    h.core.set_mcp_enabled(true).unwrap();
    h.wait("both runs", |h| runs(h) >= before + 2).await;
    let new = h.core.mcp.external_token().unwrap();
    assert_ne!(new, old);
    assert_eq!(current(&principal_json(&h)).unwrap().token, new);
    assert_eq!(h.core.mcp.declared().len(), 1);
    assert!(h.core.mcp.declared()[0].ok);
    h.core.shutdown();
}

#[tokio::test]
async fn nothing_is_declared_nor_taken_out_while_it_was_never_on_and_the_window_cannot_flip_the_switch(
) {
    let h = harness("mcp-install-never");
    // Settings saved with it off, accounts changed: no command, no token made.
    second_account(&h);
    let s = h.core.settings.read().clone();
    h.core.save_settings(s).unwrap();
    assert!(mcp_launches(&h).is_empty());
    assert!(!h.core.mcp.status().running);
    assert!(h.core.mcp.declared().is_empty());
    assert_eq!(
        h.core
            .secrets
            .get(crate::integrations::secrets::MCP_ENTRY)
            .unwrap(),
        None
    );

    // What the window saves never turns it on or off: its copy may be older.
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let mut stale = h.core.settings.read().clone();
    stale.mcp_enabled = false;
    stale.sound = !stale.sound;
    h.core.save_window_settings(stale).unwrap();
    assert!(h.core.settings.read().mcp_enabled);
    assert!(h.core.mcp.status().running);
    h.core.shutdown();
}
