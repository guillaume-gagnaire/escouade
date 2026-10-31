//! « Les agents peuvent piloter Escouade » (ESC-119): the one switch declares the server in every
//! harness installed, in each of its active accounts: Claude Code by `claude mcp add`, Codex in
//! its `config.toml` (ESC-50), OpenCode in its `opencode.json`, Copilot CLI in its
//! `mcp-config.json`, kept in step with the settings.

use super::install::{current, wanted, Entry, MASK};
use crate::core_tests::{harness, Harness};
use crate::engine::{Engine, CLAUDE, CODEX, COPILOT, OPENCODE};
use crate::model::*;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

/// What the user had in Principal's `config.toml` before Escouade wrote in it.
const USERS: &str =
    "# mon modèle\nmodel = \"gpt-5\"\n\n[mcp_servers.node_repl]\ncommand = 'node.exe' # le mien\n";

/// What the user had in OpenCode's `opencode.json`: comments, a server of theirs.
const USERS_OPENCODE: &str = "{\n  // mon modèle\n  \"model\": \"ollama/qwen3-coder\",\n  \"mcp\": {\n    \"node\": { \"type\": \"local\", \"command\": [\"node\"] } // le mien\n  }\n}\n";

/// What the user had in Copilot's `mcp-config.json`, as `copilot mcp add` writes it.
const USERS_COPILOT: &str = "{\n  \"mcpServers\": {\n    \"essai\": {\n      \"tools\": [\n        \"*\"\n      ],\n      \"type\": \"http\",\n      \"url\": \"http://127.0.0.1:1/mcp\"\n    }\n  }\n}";

/// A Principal's user config in a test: in the core's data folder (`Core::user_config_file`).
fn principal_file(h: &Harness, engine: EngineId, file: &str) -> PathBuf {
    h.dir
        .join("data")
        .join(format!("{}-home", engine.as_str()))
        .join(file)
}

fn principal_toml(h: &Harness) -> PathBuf {
    principal_file(h, EngineId::CODEX, "config.toml")
}

fn opencode_json(h: &Harness) -> PathBuf {
    principal_file(h, EngineId::OPENCODE, "opencode.json")
}

fn copilot_json(h: &Harness) -> PathBuf {
    principal_file(h, EngineId::COPILOT, "mcp-config.json")
}

fn claude_json(h: &Harness) -> PathBuf {
    h.dir.join("data").join("claude-home").join(".claude.json")
}

fn write(path: &PathBuf, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn read(path: &PathBuf) -> String {
    std::fs::read_to_string(path).unwrap()
}

/// The harnesses the test says are installed (no program of theirs is found in a unit test).
fn installed(h: &Harness, engines: &[EngineId]) {
    *h.core.mcp.installed.lock() = engines.to_vec();
}

fn runs(h: &Harness) -> usize {
    h.core.mcp.runs.load(Ordering::Acquire)
}

async fn run_after(h: &Harness, before: usize) {
    h.wait("the declaration run", |h| runs(h) > before).await;
}

/// Turns `change` and waits for the declaration run it causes.
async fn after(h: &Harness, change: impl FnOnce(&Harness)) {
    let before = runs(h);
    change(h);
    run_after(h, before).await;
}

fn now_wanted(h: &Harness) -> Entry {
    wanted(
        h.core.mcp.status().port,
        &h.core.mcp.external_token().unwrap(),
    )
}

/// Principal's `config.toml` with the user's own lines, and a second Codex account, its own
/// folder, and a third one switched off.
fn codex_accounts(h: &Harness) -> (PathBuf, PathBuf) {
    write(&principal_toml(h), USERS);
    let (pro, off) = (h.dir.join("codex-pro"), h.dir.join("codex-off"));
    let mut s = h.core.settings.read().clone();
    s.accounts
        .push(crate::accounts::principal_of(EngineId::CODEX));
    for (id, dir, active) in [("codex-pro", &pro, true), ("codex-off", &off, false)] {
        s.accounts.push(Account {
            id: id.into(),
            name: id.into(),
            engine: EngineId::CODEX,
            config_dir: dir.to_string_lossy().into(),
            active,
            ..Default::default()
        });
    }
    h.core.save_settings(s).unwrap();
    (pro.join("config.toml"), off.join("config.toml"))
}

/// The machine's account of each harness of `engines` added (ESC-128: the server is declared in
/// the accounts added alone).
fn machine_accounts(h: &Harness, engines: &[EngineId]) {
    let mut s = h.core.settings.read().clone();
    s.accounts
        .extend(engines.iter().map(|e| crate::accounts::principal_of(*e)));
    h.core.save_settings(s).unwrap();
}

/// The accounts the window is told of, harness by harness, with whether each is declared.
fn told(h: &Harness) -> Vec<(EngineId, String, bool)> {
    h.core
        .mcp
        .declared()
        .into_iter()
        .map(|d| (d.engine, d.account, d.ok))
        .collect()
}

#[tokio::test]
async fn the_one_switch_declares_the_server_in_every_harness_installed_and_each_active_account() {
    let h = harness("mcp-all-on");
    let (pro, off) = codex_accounts(&h);
    machine_accounts(&h, &[EngineId::OPENCODE, EngineId::COPILOT]);
    write(&opencode_json(&h), USERS_OPENCODE);
    write(&copilot_json(&h), USERS_COPILOT);
    installed(
        &h,
        &[EngineId::CODEX, EngineId::OPENCODE, EngineId::COPILOT],
    );

    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    let want = now_wanted(&h);
    assert_eq!(current(&CLAUDE, &claude_json(&h)), Some(want.clone()));
    for file in [principal_toml(&h), pro.clone()] {
        assert_eq!(current(&CODEX, &file), Some(want.clone()), "{file:?}");
    }
    assert!(!off.exists(), "an account switched off gets nothing");
    assert_eq!(current(&OPENCODE, &opencode_json(&h)), Some(want.clone()));
    assert_eq!(current(&COPILOT, &copilot_json(&h)), Some(want.clone()));
    // The user's lines and comments as they were, the entry after them.
    assert!(read(&principal_toml(&h)).starts_with(USERS));
    let oc = read(&opencode_json(&h));
    assert!(
        oc.contains("\"node\": { \"type\": \"local\", \"command\": [\"node\"] },\n    \"escouade\": {\n      \"type\": \"remote\""),
        "{oc}"
    );
    assert!(oc.contains("    } // le mien\n  }\n}\n"), "{oc}");

    // The window is told each account of each harness, with its file and what to paste by hand.
    assert_eq!(
        told(&h),
        [
            (EngineId::CLAUDE, "principal".into(), true),
            (EngineId::CODEX, "principal-codex".into(), true),
            (EngineId::CODEX, "codex-pro".into(), true),
            (EngineId::COPILOT, "principal-copilot".into(), true),
            (EngineId::OPENCODE, "principal-opencode".into(), true),
        ]
    );
    let declared = h.core.mcp.declared();
    let by = |account: &str| declared.iter().find(|d| d.account == account).unwrap();
    assert_eq!(by("principal").file, claude_json(&h).to_string_lossy());
    assert!(!by("principal").paste);
    assert!(
        by("principal").command.contains("claude"),
        "{}",
        by("principal").command
    );
    assert_eq!(by("codex-pro").file, pro.to_string_lossy());
    assert!(by("codex-pro").paste);
    assert!(by("codex-pro").command.contains("[mcp_servers.escouade]"));
    let copilot = by("principal-copilot");
    assert_eq!(copilot.file, copilot_json(&h).to_string_lossy());
    assert!(
        copilot.command.contains("\"mcpServers\""),
        "{}",
        copilot.command
    );
    assert!(copilot.command.contains(&format!("Bearer {MASK}")));
    assert!(by("principal-opencode")
        .command
        .contains("\"type\": \"remote\""));
    let everything = serde_json::to_string(&*h.events.lock()).unwrap();
    assert!(
        !everything.contains(&want.token),
        "a token went to the window"
    );
    // « Copier » gives the real one.
    for account in ["codex-pro", "principal-opencode", "principal-copilot"] {
        let real = h.core.mcp_manual_command(account).unwrap();
        assert!(real.contains(&format!("Bearer {}", want.token)), "{real}");
    }
    h.core.shutdown();
}

#[tokio::test]
async fn turned_off_the_token_changes_first_then_every_file_is_as_it_was_byte_for_byte() {
    let h = harness("mcp-all-off");
    let (pro, off) = codex_accounts(&h);
    machine_accounts(&h, &[EngineId::OPENCODE, EngineId::COPILOT]);
    write(&opencode_json(&h), USERS_OPENCODE);
    write(&copilot_json(&h), USERS_COPILOT);
    installed(
        &h,
        &[EngineId::CODEX, EngineId::OPENCODE, EngineId::COPILOT],
    );
    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    let old = h.core.mcp.external_token().unwrap();
    assert!(h.core.mcp.caller(&old).is_some());
    // A leftover in the account switched off (it was on once).
    write(&off, &format!("[mcp_servers.escouade]\nurl = \"http://127.0.0.1:1/mcp\"\nhttp_headers = {{ Authorization = \"Bearer {old}\" }}\n"));

    // The run cannot start yet: what happens is the switch's own.
    let held = h.core.mcp.declaring.lock().await;
    let before = runs(&h);
    h.core.set_mcp_enabled(false).unwrap();
    // The old token opens nothing at once, though it is still in the files.
    assert_eq!(h.core.mcp.caller(&old), None);
    assert_eq!(current(&COPILOT, &copilot_json(&h)).unwrap().token, old);
    drop(held);
    run_after(&h, before).await;

    assert_eq!(read(&principal_toml(&h)), USERS);
    assert_eq!(read(&pro), "");
    assert_eq!(current(&CODEX, &off), None);
    assert_eq!(read(&opencode_json(&h)), USERS_OPENCODE);
    assert_eq!(read(&copilot_json(&h)), USERS_COPILOT);
    assert_eq!(current(&CLAUDE, &claude_json(&h)), None);
    assert!(h.core.mcp.declared().is_empty());

    // On again: every harness gets the new token.
    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    let new = h.core.mcp.external_token().unwrap();
    assert_ne!(new, old);
    for (engine, file) in [
        (&CODEX as &dyn Engine, principal_toml(&h)),
        (&OPENCODE, opencode_json(&h)),
        (&COPILOT, copilot_json(&h)),
        (&CLAUDE, claude_json(&h)),
    ] {
        assert_eq!(current(engine, &file).unwrap().token, new, "{file:?}");
    }
    h.core.shutdown();
}

#[tokio::test]
async fn a_harness_not_installed_gets_nothing_until_it_is_then_at_once_and_an_account_added_too() {
    let h = harness("mcp-later");
    write(&copilot_json(&h), USERS_COPILOT);
    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    // Only Claude Code is installed: no file of another harness is touched, or made.
    assert_eq!(read(&copilot_json(&h)), USERS_COPILOT);
    assert!(!principal_toml(&h).exists() && !opencode_json(&h).exists());
    assert_eq!(told(&h), [(EngineId::CLAUDE, "principal".into(), true)]);

    // Copilot installed since (in a terminal), without an account: still nothing (ESC-128).
    installed(&h, &[EngineId::COPILOT]);
    let before = runs(&h);
    h.core.declare_on_focus();
    run_after(&h, before).await;
    assert_eq!(read(&copilot_json(&h)), USERS_COPILOT);
    // The machine's account of it added: declared at once (back to the window, or from Réglages
    // › Harness, too).
    after(&h, |h| machine_accounts(h, &[EngineId::COPILOT])).await;
    assert_eq!(current(&COPILOT, &copilot_json(&h)), Some(now_wanted(&h)));
    // The focus again at once brings no run: one a minute at most.
    let before = runs(&h);
    h.core.declare_on_focus();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(runs(&h), before);
    assert!(h.core.mcp_manual_command("principal-copilot").is_ok());

    // Another Copilot account added: declared in its folder at once.
    let dir = h.dir.join("copilot-pro");
    after(&h, |h| {
        let mut s = h.core.settings.read().clone();
        s.accounts.push(Account {
            id: "copilot-pro".into(),
            name: "Pro".into(),
            engine: EngineId::COPILOT,
            config_dir: dir.to_string_lossy().into(),
            active: true,
            ..Default::default()
        });
        h.core.save_settings(s).unwrap();
    })
    .await;
    let pro = dir.join("mcp-config.json");
    assert_eq!(current(&COPILOT, &pro), Some(now_wanted(&h)));

    // Removed: its entry leaves its file (made by Escouade: the file goes).
    let before = runs(&h);
    h.core.remove_claude_account("copilot-pro").unwrap();
    h.wait("the removal", |h| runs(h) > before + 1).await;
    assert!(!pro.exists());
    assert!(current(&COPILOT, &copilot_json(&h)).is_some());
    h.core.shutdown();
}

#[tokio::test]
async fn a_config_that_cannot_be_read_is_told_with_what_to_paste_and_left_alone() {
    let h = harness("mcp-invalid");
    write(&principal_toml(&h), "model = = 1\n");
    write(&copilot_json(&h), "{\"mcpServers\": [");
    machine_accounts(&h, &[EngineId::CODEX, EngineId::COPILOT]);
    installed(&h, &[EngineId::CODEX, EngineId::COPILOT]);
    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    let declared = h.core.mcp.declared();
    for (engine, says) in [(EngineId::CODEX, "TOML"), (EngineId::COPILOT, "JSON")] {
        let d = declared.iter().find(|d| d.engine == engine).unwrap();
        assert!(!d.ok);
        assert!(
            d.error.as_deref().unwrap_or_default().contains(says),
            "{d:?}"
        );
        assert!(
            d.paste && d.command.contains(&format!("Bearer {MASK}")),
            "{d:?}"
        );
    }
    assert_eq!(read(&principal_toml(&h)), "model = = 1\n");
    assert_eq!(read(&copilot_json(&h)), "{\"mcpServers\": [");
    // The other harnesses are declared all the same.
    assert!(declared
        .iter()
        .any(|d| d.engine == EngineId::CLAUDE && d.ok));
    h.core.shutdown();
}

#[tokio::test]
async fn the_window_never_flips_the_switch_and_settings_of_before_keep_it() {
    let h = harness("mcp-window");
    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    let mut stale = h.core.settings.read().clone();
    stale.mcp_enabled = false;
    stale.sound = !stale.sound;
    h.core.save_window_settings(stale).unwrap();
    assert!(h.core.settings.read().mcp_enabled);
    h.core.shutdown();

    // Settings saved before the one switch (ESC-50): the switches of each harness are dropped, the
    // server's switch says for all of them.
    for on in [true, false] {
        let old =
            format!("{{\"mcpEnabled\": {on}, \"mcpHarnesses\": [\"codex\"], \"sound\": false}}");
        let s: Settings = serde_json::from_str(&old).unwrap();
        assert_eq!(s.mcp_enabled, on);
        assert!(!serde_json::to_string(&s).unwrap().contains("mcpHarnesses"));
    }
}

/// Real run (ESC-50): « Codex peut piloter Escouade » on the machine's own `~/.codex/config.toml`,
/// with the real `codex` that `ESCOUADE_REAL_CODEX` names. The entry Escouade writes is read by
/// Codex, which lists the server's tools with the token it holds; taken out, the file is the one
/// saved before, byte for byte. By hand only (`cargo test --lib real_codex_drives_escouade --
/// --ignored --nocapture`); the file is put back from its copy whatever happens.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_codex_drives_escouade() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let program = std::env::var("ESCOUADE_REAL_CODEX").expect("ESCOUADE_REAL_CODEX");
    let home = dirs::home_dir().unwrap().join(".codex");
    let file = home.join("config.toml");
    let saved = std::fs::read(&file).expect("a config.toml to try it on");
    /// Puts the user's file back, whatever the test did.
    struct Back(PathBuf, Vec<u8>);
    impl Drop for Back {
        fn drop(&mut self) {
            std::fs::write(&self.0, &self.1).unwrap();
        }
    }
    let _back = Back(file.clone(), saved.clone());

    let h = harness("mcp-codex-real");
    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    let want = now_wanted(&h);
    let user = CODEX.user_config().unwrap();
    user.add(&file, &want).unwrap();
    // Every line of the user's, in its order: the entry goes beside their other servers.
    let written = std::fs::read_to_string(&file).unwrap();
    let mut rest = written.lines();
    for line in String::from_utf8_lossy(&saved).lines() {
        assert!(rest.any(|l| l == line), "{line:?} lost:\n{written}");
    }
    println!("{written}");

    // Codex's app-server, on the user's own folder: its MCP servers as it reads them.
    let mut child = tokio::process::Command::new(&program)
        .arg("app-server")
        .env("CODEX_HOME", &home)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let send = |v: serde_json::Value| format!("{v}\n");
    stdin
        .write_all(send(serde_json::json!({"id": 1, "method": "initialize", "params": {"clientInfo": {"name": "escouade-essai", "version": "0"}, "capabilities": {"experimentalApi": true}}})).as_bytes())
        .await
        .unwrap();
    stdin
        .write_all(send(serde_json::json!({"method": "initialized"})).as_bytes())
        .await
        .unwrap();
    let mut tools: Vec<String> = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut id = 2;
    while tools.is_empty() && std::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        stdin
            .write_all(
                send(serde_json::json!({"id": id, "method": "mcpServerStatus/list", "params": {}}))
                    .as_bytes(),
            )
            .await
            .unwrap();
        loop {
            let line = tokio::time::timeout(std::time::Duration::from_secs(30), lines.next_line())
                .await
                .unwrap()
                .unwrap()
                .expect("codex answers");
            let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if msg["id"] != id {
                continue;
            }
            for server in msg["result"]["data"].as_array().into_iter().flatten() {
                if server["name"] == "escouade" {
                    println!(
                        "escouade, as Codex sees it: authStatus {}, toolsError {}",
                        server["authStatus"], server["toolsError"]
                    );
                    tools = server["tools"]
                        .as_object()
                        .map(|t| t.keys().cloned().collect())
                        .unwrap_or_default();
                }
            }
            break;
        }
        id += 1;
    }
    drop(stdin);
    let _ = child.kill().await;
    println!("tools listed by Codex: {}", tools.len());
    assert!(tools.iter().any(|t| t == "list_projects"), "{tools:?}");
    // The server saw Claude outside Escouade's token: Codex came in as the user's client.
    assert!(h
        .core
        .mcp
        .activity
        .entries()
        .iter()
        .all(|e| e.outcome != "refused"));

    user.remove(&file).unwrap();
    assert_eq!(
        std::fs::read(&file).unwrap(),
        saved,
        "the file as it was, byte for byte"
    );
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(!text.contains(&want.token) && !text.contains("[mcp_servers.escouade]"));
    h.core.shutdown();
}

#[cfg(unix)]
#[tokio::test]
async fn an_account_whose_config_toml_is_a_link_to_principals_keeps_its_link_and_the_file_is_written_once(
) {
    let h = harness("mcp-codex-link");
    write(&principal_toml(&h), USERS);
    // A Codex account that shares its Principal's settings (`accounts::link_file`).
    let dir = h.dir.join("codex-lie");
    std::fs::create_dir_all(&dir).unwrap();
    let linked = dir.join("config.toml");
    std::os::unix::fs::symlink(principal_toml(&h), &linked).unwrap();
    let mut s = h.core.settings.read().clone();
    s.accounts
        .push(crate::accounts::principal_of(EngineId::CODEX));
    s.accounts.push(Account {
        id: "codex-lie".into(),
        name: "Lié".into(),
        engine: EngineId::CODEX,
        config_dir: dir.to_string_lossy().into(),
        active: true,
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    let is_link = || {
        std::fs::symlink_metadata(&linked)
            .unwrap()
            .file_type()
            .is_symlink()
    };

    installed(&h, &[EngineId::CODEX]);
    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    assert!(is_link(), "the link is still a link");
    let text = std::fs::read_to_string(principal_toml(&h)).unwrap();
    assert_eq!(text.matches("[mcp_servers.escouade]").count(), 1, "{text}");
    assert_eq!(current(&CODEX, &principal_toml(&h)), Some(now_wanted(&h)));
    // Each account told what is true: declared, both, in Principal's file.
    let codex: Vec<_> = h
        .core
        .mcp
        .declared()
        .into_iter()
        .filter(|d| d.engine == EngineId::CODEX)
        .collect();
    assert_eq!(codex.len(), 2);
    assert!(codex.iter().all(|d| d.ok), "{codex:?}");
    let real = std::fs::canonicalize(principal_toml(&h)).unwrap();
    assert_eq!(codex[1].file, real.to_string_lossy());

    // The linked account removed: Principal still wants the entry, it stays.
    let before = runs(&h);
    h.core.remove_claude_account("codex-lie").unwrap();
    // Its declaration run, and the removal of its entry.
    h.wait("the removal", |h| runs(h) > before + 1).await;
    assert!(current(&CODEX, &principal_toml(&h)).is_some());

    after(&h, |h| h.core.set_mcp_enabled(false).unwrap()).await;
    assert!(is_link(), "the link is still a link");
    assert_eq!(std::fs::read_to_string(principal_toml(&h)).unwrap(), USERS);
    h.core.shutdown();
}

/// What `program` said, run with `args` in `cwd` and `env` (only these variables added), until it
/// ends, `done` says what was awaited happened (it is stopped then: a model may go on), or
/// `minutes` pass: its output and error output, cut to their end. Written to files, the process
/// waited for alone: what it leaves running (OpenCode does) would hold pipes open.
async fn run_real(
    program: &str,
    args: &[&str],
    cwd: &std::path::Path,
    env: &[(&str, String)],
    minutes: u64,
    done: impl Fn() -> bool,
) -> String {
    let (out, err) = (cwd.join("out.txt"), cwd.join("err.txt"));
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        // OpenCode takes its folder from `PWD` when there is one (the shell's that runs the tests).
        .env("PWD", cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::fs::File::create(&out).unwrap())
        .stderr(std::fs::File::create(&err).unwrap())
        .kill_on_drop(true);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(minutes * 60);
    let status = loop {
        if let Ok(Some(status)) = child.try_wait() {
            break format!("{status}");
        }
        if done() || std::time::Instant::now() > deadline {
            let _ = child.kill().await;
            break "stopped".to_string();
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    };
    let text = format!(
        "{status:?}\n{}\n--- stderr ---\n{}",
        std::fs::read_to_string(&out).unwrap_or_default(),
        std::fs::read_to_string(&err).unwrap_or_default()
    );
    let cut = text.len().saturating_sub(4000);
    text[text.ceil_char_boundary(cut)..].to_string()
}

/// The calls of `tool` the server answered, outside Escouade (Escouade's own token).
fn called(h: &Harness, tool: &str) -> usize {
    h.core
        .mcp
        .activity
        .entries()
        .iter()
        .filter(|e| e.tool == tool && e.outcome == "ok")
        .count()
}

/// Real run (ESC-119): OpenCode, started outside Escouade with nothing but a disposable
/// `XDG_*` whose `opencode.jsonc` is the user's (comments, Ollama as provider) plus the entry
/// Escouade wrote, calls Escouade's `list_projects`; taken out, the file is as it was, byte for
/// byte. `ESCOUADE_REAL_OPENCODE`: the native `opencode`; Ollama's `qwen3-coder:30b-32k` must
/// be there. By hand only (`cargo test --lib real_opencode_drives -- --ignored --nocapture`).
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_opencode_drives_escouade_from_the_entry_escouade_wrote() {
    let program = std::env::var("ESCOUADE_REAL_OPENCODE").expect("ESCOUADE_REAL_OPENCODE");
    let h = harness("mcp-opencode-real");
    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    let want = now_wanted(&h);
    let xdg = h.dir.join("xdg");
    let file = xdg.join("config").join("opencode").join("opencode.jsonc");
    let users = "{\n  // Ollama, en local\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"provider\": {\n    \"ollama\": {\n      \"npm\": \"@ai-sdk/openai-compatible\",\n      \"options\": { \"baseURL\": \"http://127.0.0.1:11434/v1\" },\n      \"models\": { \"qwen3-coder:30b-32k\": { \"tool_call\": true, \"limit\": { \"context\": 32768, \"output\": 8192 } } },\n    },\n  },\n  \"model\": \"ollama/qwen3-coder:30b-32k\",\n  \"small_model\": \"ollama/qwen3-coder:30b-32k\",\n  \"enabled_providers\": [\"ollama\"], // rien d'autre\n}\n";
    write(&file, users);
    let user = OPENCODE.user_config().unwrap();
    // Where Escouade writes it: the user's `opencode.jsonc`, the one there.
    assert_eq!(user.file_in(file.parent().unwrap()), file);
    user.add(&file, &want).unwrap();
    println!("{}", crate::mcp::install::mask_bearer(&read(&file)));
    let env: Vec<(&str, String)> = ["config", "data", "state", "cache"]
        .iter()
        .map(|d| {
            let var = match *d {
                "config" => "XDG_CONFIG_HOME",
                "data" => "XDG_DATA_HOME",
                "state" => "XDG_STATE_HOME",
                _ => "XDG_CACHE_HOME",
            };
            (var, xdg.join(d).to_string_lossy().into_owned())
        })
        .collect();
    let work = h.dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let said = run_real(
        &program,
        &[
            "run",
            "Call the tool escouade_list_projects once, then say how many projects it returned.",
        ],
        &work,
        &env,
        15,
        || called(&h, "list_projects") >= 1,
    )
    .await;
    println!("{}", crate::mcp::install::mask_bearer(&said));
    assert!(called(&h, "list_projects") >= 1, "no call of list_projects");

    user.remove(&file).unwrap();
    assert_eq!(read(&file), users, "the file as it was, byte for byte");
    h.core.shutdown();
}

/// Real run (ESC-119): Copilot CLI, started outside Escouade (`copilot -p`) with a disposable
/// `COPILOT_HOME` whose `mcp-config.json` is as `copilot mcp remove` leaves it plus the entry
/// Escouade wrote, calls Escouade's `list_projects`; taken out, the file is as it was, byte for
/// byte. `ESCOUADE_REAL_COPILOT`: the native `copilot`; signed in by `gh auth token` (the
/// GitHub CLI's), given to its process alone. Uses a request of the account's. By hand only
/// (`cargo test --lib real_copilot_drives -- --ignored --nocapture`).
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_copilot_drives_escouade_from_the_entry_escouade_wrote() {
    let program = std::env::var("ESCOUADE_REAL_COPILOT").expect("ESCOUADE_REAL_COPILOT");
    let gh = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .unwrap();
    let token = String::from_utf8_lossy(&gh.stdout).trim().to_string();
    assert!(!token.is_empty(), "gh auth token");
    let h = harness("mcp-copilot-real");
    after(&h, |h| h.core.set_mcp_enabled(true).unwrap()).await;
    let want = now_wanted(&h);
    let home = h.dir.join("copilot-home");
    let file = home.join("mcp-config.json");
    let users = "{\n  \"mcpServers\": {}\n}";
    write(&file, users);
    let user = COPILOT.user_config().unwrap();
    user.add(&file, &want).unwrap();
    println!("{}", crate::mcp::install::mask_bearer(&read(&file)));
    let work = h.dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let said = run_real(
        &program,
        &[
            "-p",
            "Call the list_projects tool of the escouade MCP server once, then say how many projects it returned.",
            "--allow-all-tools",
        ],
        &work,
        &[
            ("COPILOT_HOME", home.to_string_lossy().into_owned()),
            ("GH_TOKEN", token.clone()),
        ],
        10,
        || called(&h, "list_projects") >= 1,
    )
    .await;
    println!(
        "{}",
        crate::mcp::install::mask_bearer(&said.replace(&token, "<gh>"))
    );
    assert!(called(&h, "list_projects") >= 1, "no call of list_projects");

    user.remove(&file).unwrap();
    assert_eq!(read(&file), users, "the file as it was, byte for byte");
    h.core.shutdown();
}
