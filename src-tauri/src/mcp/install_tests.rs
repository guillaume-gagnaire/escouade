//! The server declared in Claude Code: its entry written by Escouade in the account's `.claude.json`, as `claude mcp add
//! --scope user --transport http` would, and never by running `claude` with the token on its command line (SEC-07,
//! ESC-595).

use super::install::{
    current, declare, mask_bearer, wanted, withdraw, Declared, Entry, Target, COMMAND_TIMEOUT,
};
use crate::core_tests::{fake_cli, harness, second_account, Harness};
use crate::engine::Engine;
use crate::model::*;
use crate::paths::test_dir;
use escouade_mcp::install::MASK;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

/// The harness the accounts are of: Claude Code.
const E: &dyn Engine = &crate::engine::CLAUDE;

/// An account's Claude Code of a test: its `.claude.json` in `<dir>/home`. Its program is the fake `claude`, which logs
/// each of its launches in `<dir>/log.jsonl`: none is expected.
fn target(name: &str) -> Target {
    let dir = test_dir(name);
    Target {
        program: crate::engine::Program::Native(fake_cli().into()),
        env: vec![(
            "FAKE_CLAUDE_LOG".into(),
            dir.join("log.jsonl").to_string_lossy().into(),
        )],
        config: dir.join("home").join(".claude.json"),
        cwd: dir,
        timeout: COMMAND_TIMEOUT,
    }
}

/// Whether anything was run: never, the entry is written by Escouade.
fn ran_nothing(t: &Target) -> bool {
    !t.cwd.join("log.jsonl").exists()
}

fn entry(port: u16, token: &str) -> Entry {
    wanted(port, token)
}

/// The file, read as Claude Code reads it: strict JSON (`JSON.parse`), which `serde_json` is too.
fn strict(file: &Path) -> Value {
    let text = std::fs::read_to_string(file).unwrap();
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("not strict JSON ({e}): {text}"))
}

/// What the file holds under the server's name, as JSON.
fn held(t: &Target) -> Value {
    strict(&t.config)["mcpServers"]["escouade"].clone()
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
    assert_eq!(current(E, &t.config), None);
    write_json(&t.config, &json!({}));
    assert_eq!(current(E, &t.config), None);
    std::fs::write(&t.config, "{ not json").unwrap();
    assert_eq!(current(E, &t.config), None);
    write_json(
        &t.config,
        &json!({ "oauthAccount": { "emailAddress": "a@b.c" }, "mcpServers": { "autre": { "type": "http", "url": "http://x" } } }),
    );
    assert_eq!(current(E, &t.config), None);
    // As `claude mcp add --transport http` writes it.
    write_json(
        &t.config,
        &json!({ "mcpServers": { "escouade": {
            "type": "http",
            "url": "http://127.0.0.1:47123/mcp",
            "headers": { "Authorization": "Bearer tok-1" },
        } } }),
    );
    assert_eq!(current(E, &t.config), Some(want.clone()));
    // An entry of that name in another shape is there, but is not the one wanted.
    for odd in [
        json!({ "type": "stdio", "command": "node" }),
        json!({ "type": "http", "url": "http://127.0.0.1:47123/mcp" }),
        json!({ "type": "http", "url": "http://127.0.0.1:47123/mcp", "headers": { "Authorization": "Basic abc" } }),
    ] {
        write_json(&t.config, &json!({ "mcpServers": { "escouade": odd } }));
        let found = current(E, &t.config).expect("an entry");
        assert_ne!(found, want, "{odd}");
    }
}

/// SEC-07: the entry is written in the account's `.claude.json` with nothing run, the token on no command line; the
/// state Claude Code keeps beside its servers is kept, and the file stays strict JSON.
#[tokio::test]
async fn an_account_without_the_server_gets_it_written_in_its_claude_json_with_nothing_run() {
    let t = target("mcp-install-add");
    write_json(
        &t.config,
        &json!({ "oauthAccount": { "emailAddress": "a@b.c" }, "projects": { "C:/p": { "mcpServers": {} } } }),
    );
    let done = declare(E, &t, &entry(47123, "tok-1")).await.unwrap();
    assert_eq!(done, Declared::Added);
    assert!(ran_nothing(&t), "a program was run");
    assert_eq!(
        held(&t),
        json!({
            "type": "http",
            "url": "http://127.0.0.1:47123/mcp",
            "headers": { "Authorization": "Bearer tok-1" },
        })
    );
    assert_eq!(current(E, &t.config), Some(entry(47123, "tok-1")));
    let json = strict(&t.config);
    assert_eq!(json["oauthAccount"]["emailAddress"], "a@b.c");
    assert_eq!(json["projects"]["C:/p"]["mcpServers"], json!({}));
    // Readable by its owner alone under Unix: it holds the token.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&t.config).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

/// An object of the servers already there, empty (as Claude Code writes it), and no file at all: strict JSON either way,
/// with no trailing comma.
#[tokio::test]
async fn the_entry_written_in_an_empty_object_or_a_new_file_is_strict_json() {
    let t = target("mcp-install-empty");
    std::fs::create_dir_all(t.config.parent().unwrap()).unwrap();
    std::fs::write(
        &t.config,
        "{\n  \"numStartups\": 3,\n  \"mcpServers\": {}\n}\n",
    )
    .unwrap();
    declare(E, &t, &entry(47123, "tok-1")).await.unwrap();
    assert_eq!(held(&t)["url"], "http://127.0.0.1:47123/mcp");
    assert_eq!(strict(&t.config)["numStartups"], 3);
    // Taken out: the file as it was, its empty object left.
    assert!(withdraw(E, &t).await.unwrap());
    assert_eq!(
        std::fs::read_to_string(&t.config).unwrap(),
        "{\n  \"numStartups\": 3,\n  \"mcpServers\": {}\n}\n"
    );
    let fresh = target("mcp-install-fresh");
    declare(E, &fresh, &entry(47123, "tok-1")).await.unwrap();
    assert_eq!(held(&fresh)["type"], "http");
    assert!(ran_nothing(&fresh));
}

#[tokio::test]
async fn the_server_declared_as_wanted_is_left_as_it_is() {
    let t = target("mcp-install-same");
    declare(E, &t, &entry(47123, "tok-1")).await.unwrap();
    let before = std::fs::read_to_string(&t.config).unwrap();
    let again = declare(E, &t, &entry(47123, "tok-1")).await.unwrap();
    assert_eq!(again, Declared::Unchanged);
    assert_eq!(std::fs::read_to_string(&t.config).unwrap(), before);
}

#[tokio::test]
async fn a_server_declared_otherwise_is_replaced() {
    let t = target("mcp-install-replace");
    declare(E, &t, &entry(47123, "tok-1")).await.unwrap();
    // Another port, then another token, then an entry of another kind.
    for next in [entry(47200, "tok-1"), entry(47200, "tok-2")] {
        assert_eq!(declare(E, &t, &next).await.unwrap(), Declared::Replaced);
        assert_eq!(current(E, &t.config), Some(next));
    }
    write_json(
        &t.config,
        &json!({ "mcpServers": { "escouade": { "type": "sse", "url": "http://127.0.0.1:47200/mcp" } } }),
    );
    assert_eq!(
        declare(E, &t, &entry(47200, "tok-2")).await.unwrap(),
        Declared::Replaced
    );
    assert_eq!(held(&t)["type"], "http");
    assert!(ran_nothing(&t));
}

#[tokio::test]
async fn a_claude_json_that_is_not_json_is_left_alone_with_its_reason() {
    let t = target("mcp-install-refused");
    std::fs::create_dir_all(t.config.parent().unwrap()).unwrap();
    std::fs::write(&t.config, "{ not json").unwrap();
    let e = declare(E, &t, &entry(47123, "s3cr3t-token"))
        .await
        .unwrap_err();
    let said = format!("{e:#}");
    assert!(said.contains("n’est pas un JSON valide"), "{said}");
    assert!(!said.contains("s3cr3t-token"), "{said}");
    assert_eq!(std::fs::read_to_string(&t.config).unwrap(), "{ not json");
    // Nor can it be taken out of it.
    assert!(withdraw(E, &t).await.is_err());
    assert_eq!(std::fs::read_to_string(&t.config).unwrap(), "{ not json");
    assert!(ran_nothing(&t));
}

#[tokio::test]
async fn withdrawing_removes_the_entry_and_an_entry_already_absent_is_no_error() {
    let t = target("mcp-install-withdraw");
    // Nothing declared, nothing done.
    assert!(!withdraw(E, &t).await.unwrap());
    write_json(
        &t.config,
        &json!({ "oauthAccount": { "emailAddress": "a@b.c" } }),
    );
    declare(E, &t, &entry(47123, "tok-1")).await.unwrap();
    assert!(withdraw(E, &t).await.unwrap());
    assert_eq!(current(E, &t.config), None);
    assert_eq!(
        strict(&t.config),
        json!({ "oauthAccount": { "emailAddress": "a@b.c" } })
    );
    assert!(!withdraw(E, &t).await.unwrap());
    assert!(ran_nothing(&t));
}

#[test]
fn a_token_is_hidden_wherever_it_could_be_printed() {
    // A {:?} of an entry (a failed assertion, a log line) does not print it.
    let e = entry(47123, "s3cr3t-token");
    assert!(!format!("{e:?}").contains("s3cr3t-token"), "{e:?}");
    assert!(format!("{e:?}").contains(MASK));
    // An entry shown: the token goes, the rest stays.
    let shown = E.manual_declaration(true, "", None, &e, false);
    assert_eq!(
        mask_bearer(&shown)
            .matches(&format!("Bearer {MASK}"))
            .count(),
        1,
        "{shown}"
    );
    assert!(!mask_bearer(&shown).contains("s3cr3t-token"));
    assert_eq!(
        mask_bearer("a 'Authorization: Bearer s3cr3t-token'; b Bearer other\nz"),
        format!("a 'Authorization: Bearer {MASK}'; b Bearer {MASK}\nz")
    );
}

/// By hand: the entry to paste in `.claude.json`, as Escouade writes it; no command that would put the token on a
/// command line.
#[test]
fn the_entry_to_declare_it_by_hand_is_the_one_to_paste_in_claude_json() {
    let e = entry(47123, "tok-1");
    for windows in [true, false] {
        let paste = E.manual_declaration(windows, "claude", Some("/p"), &e, true);
        assert!(!paste.contains("mcp add"), "{paste}");
        let v: Value = serde_json::from_str(&paste).unwrap();
        assert_eq!(
            v,
            json!({ "mcpServers": { "escouade": {
                "type": "http",
                "url": "http://127.0.0.1:47123/mcp",
                "headers": { "Authorization": "Bearer tok-1" },
            } } })
        );
    }
}

// ---------- in the app: each active account, as the settings change ----------

/// The data folder: where the fake `claude` of the core would log a launch.
fn data(h: &Harness) -> PathBuf {
    h.dir.join("data")
}

/// The `claude mcp …` launches so far: none ever (SEC-07).
fn mcp_launches(h: &Harness) -> Vec<Vec<String>> {
    h.launches(&data(h))
        .into_iter()
        .filter(|a| a.first().is_some_and(|c| c == "mcp"))
        .collect()
}

/// Principal's `.claude.json` in a test: in the home the core gives Claude Code in unit tests.
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
async fn turning_it_on_declares_the_server_in_each_active_account_with_no_command_run() {
    let h = harness("mcp-install-on");
    second_account(&h);
    let off = off_account(&h);
    // Off, nothing is declared anywhere.
    assert!(h.core.mcp.declared().is_empty());

    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    assert!(h.core.mcp.status().running);
    let declared = h.core.mcp.declared();
    assert_eq!(
        declared
            .iter()
            .map(|d| (d.account.as_str(), d.ok, d.error.as_deref(), d.paste))
            .collect::<Vec<_>>(),
        [("principal", true, None, true), ("pro", true, None, true)]
    );
    // Both hold the server, on its port, with its token, in the `.claude.json` their `claude` reads; the one switched
    // off holds nothing.
    let want = now_wanted(&h);
    assert_eq!(current(E, &principal_json(&h)), Some(want.clone()));
    assert_eq!(current(E, &pro_json(&h)), Some(want.clone()));
    assert!(!off.join(".claude.json").exists());
    assert_eq!(declared[1].file, pro_json(&h).to_string_lossy());
    // SEC-07: no `claude mcp add`, so no token on a command line.
    assert!(mcp_launches(&h).is_empty(), "{:?}", mcp_launches(&h));
    // The window was told, and the token is nowhere in what it is told: the entry it shows has it hidden.
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
    let shown = told["declared"][1]["command"].as_str().unwrap();
    assert!(shown.contains(&format!("Bearer {MASK}")), "{shown}");
    assert!(shown.contains("\"mcpServers\""), "{shown}");
    let everything = serde_json::to_string(&*h.events.lock()).unwrap();
    assert!(
        !everything.contains(&want.token),
        "a token went to the window"
    );
    let status = serde_json::to_string(&h.core.mcp.declared()).unwrap();
    assert!(!status.contains(&want.token), "{status}");
    // The real entry is built when it is asked for (« Copier »).
    let real = h.core.mcp_manual_command("pro").unwrap();
    assert!(real.contains(&format!("Bearer {}", want.token)), "{real}");
    assert!(real.contains(&want.url), "{real}");

    // The app starts again with it on: what is there as wanted is left alone.
    let written = std::fs::read_to_string(pro_json(&h)).unwrap();
    h.core.mcp.stop();
    let before = runs(&h);
    h.core.start_mcp();
    run_after(&h, before).await;
    assert_eq!(h.core.mcp.status().port, h.core.settings.read().mcp_port);
    assert_eq!(std::fs::read_to_string(pro_json(&h)).unwrap(), written);
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
    assert_eq!(current(E, &pro_json(&h)), Some(now_wanted(&h)));

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
    assert_eq!(current(E, &off.join(".claude.json")), Some(now_wanted(&h)));

    // Switched off again: no longer listed, and its entry is left where it is (it works when it is used by hand), then
    // declared as it stood when it is switched back on, the file untouched.
    let before = runs(&h);
    account.active = false;
    h.core.update_claude_account(account.clone()).unwrap();
    run_after(&h, before).await;
    assert_eq!(h.core.mcp.declared().len(), 2);
    assert!(current(E, &off.join(".claude.json")).is_some());
    let written = std::fs::read_to_string(off.join(".claude.json")).unwrap();
    let before = runs(&h);
    account.active = true;
    h.core.update_claude_account(account).unwrap();
    run_after(&h, before).await;
    assert_eq!(h.core.mcp.declared().len(), 3);
    assert_eq!(
        std::fs::read_to_string(off.join(".claude.json")).unwrap(),
        written
    );
    assert!(mcp_launches(&h).is_empty());
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

    // The server stops, and its port is taken when it starts again: another one is chosen.
    h.core.mcp.stop();
    let taken = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
    let before = runs(&h);
    h.core.sync_mcp();
    run_after(&h, before).await;
    let moved = h.core.mcp.status().port;
    assert_ne!(moved, port);
    for file in [principal_json(&h), pro_json(&h)] {
        let e = current(E, &file).unwrap();
        assert_eq!(e.url, format!("http://127.0.0.1:{moved}/mcp"), "{file:?}");
        assert_eq!(e.token, h.core.mcp.external_token().unwrap());
    }
    assert!(mcp_launches(&h).is_empty());
    drop(taken);
    h.core.shutdown();
}

#[tokio::test]
async fn an_account_that_fails_is_told_with_the_entry_to_paste_by_hand_and_the_others_are_declared()
{
    let h = harness("mcp-install-failure");
    second_account(&h);
    let folder = h.dir.join("claude-pro");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(pro_json(&h), "{ damaged").unwrap();
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let declared = h.core.mcp.declared();
    assert_eq!(declared.len(), 2);
    assert!(declared[0].ok && declared[0].error.is_none());
    assert_eq!(current(E, &principal_json(&h)), Some(now_wanted(&h)));
    let pro = &declared[1];
    assert!(!pro.ok);
    assert!(
        pro.error
            .as_deref()
            .unwrap()
            .contains("n’est pas un JSON valide"),
        "{pro:?}"
    );
    assert!(pro.paste);
    assert_eq!(pro.file, pro_json(&h).to_string_lossy());
    // What it is told is the entry with its token hidden; the real one is built when the user copies it.
    let token = h.core.mcp.external_token().unwrap();
    let port = h.core.mcp.status().port;
    assert!(!pro.command.contains(&token), "{pro:?}");
    assert!(pro.command.contains(&format!("Bearer {MASK}")), "{pro:?}");
    let real = h.core.mcp_manual_command("pro").unwrap();
    assert!(real.contains(&format!("Bearer {token}")), "{real}");
    assert!(
        real.contains(&format!("http://127.0.0.1:{port}/mcp")),
        "{real}"
    );
    assert_eq!(mask_bearer(&real), pro.command);
    // Nothing was written in it.
    assert_eq!(std::fs::read_to_string(pro_json(&h)).unwrap(), "{ damaged");

    // Saving the settings again tries again (the file repaired).
    std::fs::write(pro_json(&h), "{}").unwrap();
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    assert!(h.core.mcp.declared().iter().all(|d| d.ok));
    assert_eq!(current(E, &pro_json(&h)), Some(now_wanted(&h)));
    h.core.shutdown();
}

#[tokio::test]
async fn the_real_entry_is_given_only_while_the_server_runs() {
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
    let principal = h.core.mcp_manual_command("principal").unwrap();
    assert!(principal.contains(&now_wanted(&h).token));
    assert!(!principal.contains("mcp add"), "{principal}");
    h.core.shutdown();
}

/// An account whose `claude` is not found still has the entry written: nothing is run for it, and the entry is there
/// for the day its `claude` is.
#[tokio::test]
async fn an_account_whose_claude_is_missing_is_declared_all_the_same() {
    let h = harness("mcp-install-no-claude");
    let mut pro = second_account(&h);
    pro.claude_path = h.dir.join("pas-de-claude").to_string_lossy().into();
    h.core.update_claude_account(pro).unwrap();
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let declared = h.core.mcp.declared();
    assert!(declared.iter().all(|d| d.ok), "{declared:?}");
    assert_eq!(current(E, &pro_json(&h)), Some(now_wanted(&h)));
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
        assert_eq!(current(E, &file), None, "{file:?}");
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
    assert_eq!(current(E, &principal_json(&h)).unwrap().token, new);
    assert_eq!(current(E, &pro_json(&h)).unwrap().token, new);
    assert!(mcp_launches(&h).is_empty());
    h.core.shutdown();
}

#[tokio::test]
async fn turning_it_off_changes_the_token_before_the_entries_are_taken_out() {
    let h = harness("mcp-install-off-order");
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let old = h.core.mcp.external_token().unwrap();
    // The run that takes the entries out waits: for that time the entry is still in the file.
    let held = h.core.mcp.declaring.lock().await;
    let before = runs(&h);
    h.core.set_mcp_enabled(false).unwrap();
    assert_ne!(h.core.mcp.external_token().unwrap(), old);
    // The old token opens nothing, and the entry that holds it has not gone yet.
    assert_eq!(h.core.mcp.caller(&old), None);
    assert!(current(E, &principal_json(&h)).is_some());
    drop(held);
    run_after(&h, before).await;
    assert_eq!(current(E, &principal_json(&h)), None);
    h.core.shutdown();
}

#[tokio::test]
async fn turning_it_off_changes_the_token_at_once_so_that_quitting_right_after_leaves_the_old_one_dead(
) {
    let h = harness("mcp-install-off-at-once");
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let old = h.core.mcp.external_token().unwrap();
    assert!(h.core.mcp.caller(&old).is_some());
    // The declaration run, on its own task, cannot start: whatever happens next is the save's own.
    let held = h.core.mcp.declaring.lock().await;
    h.core.set_mcp_enabled(false).unwrap();
    // The old token is refused and forgotten by the keychain as the switch is turned off...
    assert_eq!(h.core.mcp.caller(&old), None);
    let new = h.core.mcp.external_token().unwrap();
    assert_ne!(new, old);
    assert_eq!(
        h.core
            .secrets
            .get(crate::integrations::secrets::MCP_ENTRY)
            .unwrap(),
        Some(new.clone())
    );
    // ...so the app quitting before the run ever starts leaves the entry in Claude Code with a
    // token that opens nothing at the next start.
    h.core.shutdown();
    drop(held);
    assert_eq!(h.core.mcp.caller(&old), None);
    assert_eq!(
        h.core
            .secrets
            .get(crate::integrations::secrets::MCP_ENTRY)
            .unwrap(),
        Some(new)
    );
}

#[tokio::test]
async fn removing_an_account_takes_the_server_out_of_its_folder_and_leaves_the_token_to_the_others()
{
    let h = harness("mcp-install-remove-on");
    second_account(&h);
    // What Claude Code keeps of the account, which stays.
    write_json(
        &pro_json(&h),
        &json!({ "oauthAccount": { "emailAddress": "pro@b.c" } }),
    );
    let before = runs(&h);
    h.core.set_mcp_enabled(true).unwrap();
    run_after(&h, before).await;
    let token = h.core.mcp.external_token().unwrap();
    assert_eq!(current(E, &pro_json(&h)), Some(now_wanted(&h)));

    // Removed: its folder stays on the disk, but the entry that holds the token does not.
    // Two runs follow: the declaration the settings call for, and the removal from its folder.
    let before = runs(&h);
    h.core.remove_claude_account("pro").unwrap();
    h.wait("both runs", |h| runs(h) >= before + 2).await;
    assert_eq!(current(E, &pro_json(&h)), None);
    assert_eq!(
        strict(&pro_json(&h)),
        json!({ "oauthAccount": { "emailAddress": "pro@b.c" } })
    );
    // The others are left as they were, with the same token.
    assert_eq!(current(E, &principal_json(&h)), Some(now_wanted(&h)));
    assert_eq!(h.core.mcp.external_token().unwrap(), token);
    assert_eq!(h.core.mcp.declared().len(), 1);
    assert!(mcp_launches(&h).is_empty());
    h.core.shutdown();
}

#[tokio::test]
async fn removing_an_account_while_it_is_off_takes_a_leftover_entry_out_too() {
    let h = harness("mcp-install-remove-off");
    let off = off_account(&h);
    second_account(&h);
    // A leftover of when it was on, in the account that is switched off.
    let held = json!({ "oauthAccount": {}, "mcpServers": { "escouade": { "type": "http", "url": "http://127.0.0.1:1/mcp", "headers": { "Authorization": "Bearer old-token" } } } });
    write_json(&off.join(".claude.json"), &held);
    let before = runs(&h);
    h.core.remove_claude_account("off").unwrap();
    run_after(&h, before).await;
    assert_eq!(current(E, &off.join(".claude.json")), None);
    // Off all along: no server, no token made, no declaration.
    assert!(!h.core.mcp.status().running);
    assert_eq!(
        h.core
            .secrets
            .get(crate::integrations::secrets::MCP_ENTRY)
            .unwrap(),
        None
    );
    // An account with nothing in its folder: no file made.
    let before = runs(&h);
    h.core.remove_claude_account("pro").unwrap();
    run_after(&h, before).await;
    assert!(!pro_json(&h).exists());
    assert!(mcp_launches(&h).is_empty());
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
    assert_eq!(current(E, &principal_json(&h)).unwrap().token, new);
    assert_eq!(h.core.mcp.declared().len(), 1);
    assert!(h.core.mcp.declared()[0].ok);
    h.core.shutdown();
}

#[tokio::test]
async fn nothing_is_declared_nor_taken_out_while_it_was_never_on_and_the_window_cannot_flip_the_switch(
) {
    let h = harness("mcp-install-never");
    // Settings saved with it off, accounts changed: nothing written, no token made.
    second_account(&h);
    let s = h.core.settings.read().clone();
    h.core.save_settings(s).unwrap();
    assert!(!principal_json(&h).exists());
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
