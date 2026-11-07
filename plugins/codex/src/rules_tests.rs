//! What holds of the agent's rules through the service, as the core calls it: a restart of the plugin, a commit that reads
//! its message from a file, git's own rule (ESC-362).

use super::*;
use escouade_plugin_protocol::engine::method;

const FENCE: &str = r"C:\code\secret";

/// One call of the service, the way the core makes it.
fn call(p: &mut crate::Plugin, m: &str, params: impl serde::Serialize) -> Value {
    p.handle(m, serde_json::to_value(params).unwrap())
        .unwrap_or_else(|e| panic!("{m}: {}", e.message))
}

/// What Codex wrote, through the service, as the core reads it.
fn told(p: &mut crate::Plugin, lines: &[String]) -> Vec<Item> {
    let frames = json!({ "agent": "a1", "frames": lines });
    serde_json::from_value(call(p, method::FRAMES, frames)["items"].clone()).unwrap()
}

/// A plugin that started anew, as the core gives it the agent.
fn started_again(core: &Attach) -> crate::Plugin {
    let mut p = crate::Plugin::default();
    call(&mut p, method::ATTACH, core);
    p
}

/// What the core keeps of an agent launched by `launch`, its thread open.
fn kept(launch: &LaunchParams) -> Attach {
    Attach {
        session: Some("thr".into()),
        fence: launch.fence.clone(),
        read_only: launch.read_only,
        mode: launch.mode.clone(),
        cwd: launch.cwd.clone(),
        ..attach()
    }
}

/// A plugin with an agent launched by `launch`, its thread open.
fn launched(launch: &LaunchParams) -> crate::Plugin {
    let mut p = crate::Plugin::default();
    call(&mut p, method::ATTACH, attach());
    let mut todo: Vec<String> =
        serde_json::from_value(call(&mut p, method::LAUNCH, launch)["writes"].clone()).unwrap();
    while !todo.is_empty() {
        let mut next = Vec::new();
        for line in todo {
            let m = parsed(&line);
            if m["id"].is_null() {
                continue;
            }
            let answer = plain(m["method"].as_str().unwrap_or_default());
            let reply = json!({ "id": m["id"], "result": answer }).to_string();
            next.extend(writes(&told(&mut p, &[reply])));
        }
        todo = next;
    }
    p
}

/// The next message of the agent, as Codex is told it.
fn turn_of(p: &mut crate::Plugin) -> Value {
    let message = json!({ "agent": "a1", "uuid": "u1", "text": "suite" });
    let sent = call(p, method::SEND_MESSAGE, message);
    let line = parsed(sent["writes"][0].as_str().expect("a message written"));
    assert_eq!(line["method"], "turn/start");
    line["params"].clone()
}

/// The turn ends, for the next one.
fn turn_ended(p: &mut crate::Plugin, params: &Value) {
    let turn = json!({ "id": "t1", "status": "completed", "items": [] });
    let thread = &params["threadId"];
    let started =
        json!({ "method": "turn/started", "params": { "threadId": thread, "turn": turn } });
    let done =
        json!({ "method": "turn/completed", "params": { "threadId": thread, "turn": turn } });
    told(p, &[started.to_string(), done.to_string()]);
}

#[test]
fn the_fence_is_in_each_turn_before_and_after_a_restart_of_the_plugin() {
    let launch = LaunchParams {
        fence: vec![FENCE.into()],
        ..params()
    };
    let mut before = launched(&launch);
    let turn = turn_of(&mut before);
    assert_eq!(turn["permissions"], FENCE_PROFILE);
    assert!(turn.get("sandboxPolicy").is_none());
    drop(before);
    let mut after = started_again(&kept(&launch));
    let turn = turn_of(&mut after);
    assert_eq!(turn["permissions"], FENCE_PROFILE, "{turn}");
    assert!(turn.get("sandboxPolicy").is_none(), "{turn}");
}

#[test]
fn a_discussion_reads_only_before_and_after_a_restart_of_the_plugin() {
    let launch = LaunchParams {
        read_only: true,
        ..params()
    };
    let mut before = launched(&launch);
    assert_eq!(turn_of(&mut before)["sandboxPolicy"]["type"], "readOnly");
    drop(before);
    let mut after = started_again(&kept(&launch));
    let turn = turn_of(&mut after);
    assert_eq!(turn["sandboxPolicy"]["type"], "readOnly", "{turn}");
}

#[test]
fn what_the_core_gives_again_wins_over_what_the_agent_kept() {
    let launch = LaunchParams {
        fence: vec![FENCE.into()],
        ..params()
    };
    let core = kept(&launch);
    let mut p = launched(&launch);
    let turn = turn_of(&mut p);
    assert_eq!(turn["permissions"], FENCE_PROFILE);
    turn_ended(&mut p, &turn);
    // Taken off.
    let lifted = Attach {
        fence: Vec::new(),
        ..core.clone()
    };
    call(&mut p, method::ATTACH, lifted);
    let turn = turn_of(&mut p);
    assert!(turn.get("permissions").is_none(), "{turn}");
    assert_eq!(turn["sandboxPolicy"]["type"], "workspaceWrite");
    turn_ended(&mut p, &turn);
    // Read-only given to an agent that was not.
    let read_only = Attach {
        fence: Vec::new(),
        read_only: true,
        ..core.clone()
    };
    call(&mut p, method::ATTACH, read_only);
    let turn = turn_of(&mut p);
    assert_eq!(turn["sandboxPolicy"]["type"], "readOnly", "{turn}");
    turn_ended(&mut p, &turn);
    // Given back.
    call(&mut p, method::ATTACH, core);
    assert_eq!(turn_of(&mut p)["permissions"], FENCE_PROFILE);
}

/// A repository, for the commands of a worktree: its path.
fn repository(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("esc362-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let init = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status();
    assert!(init.is_ok_and(|s| s.success()));
    dir.to_string_lossy().into_owned()
}

/// Codex asks for a command that runs the script `script`.
fn asks(id: u64, cwd: &str, script: &str) -> String {
    json!({
        "id": id, "method": "item/commandExecution/requestApproval",
        "params": {
            "threadId": "thr", "itemId": format!("c{id}"), "cwd": cwd,
            "command": format!("pwsh.exe -Command '{script}'"),
            "commandActions": [{ "command": script }],
        },
    })
    .to_string()
}

fn accepted(items: &[Item]) -> bool {
    items
        .iter()
        .all(|i| !matches!(i, Item::PermissionCard { .. }))
        && writes(items)
            .first()
            .is_some_and(|l| parsed(l)["result"]["decision"] == "accept")
}

#[test]
fn a_commit_message_read_from_a_file_has_a_card_for_a_fenced_agent() {
    let cwd = repository("commit-file");
    let fence = format!("{cwd}-prive");
    let name = fence.rsplit(['/', '\\']).next().unwrap().to_string();
    let launch = LaunchParams {
        cwd: cwd.clone(),
        mode: "acceptEdits".into(),
        fence: vec![fence.clone()],
        ..params()
    };
    let mut p = launched(&launch);
    for (i, script) in [
        format!("git commit -F {fence}/secret.txt"),
        format!("git commit --file {fence}/secret.txt"),
        format!("git commit --file={fence}/secret.txt"),
        format!("git commit -F ../{name}/secret.txt"),
        format!("git commit --fil=../{name}/secret.txt"),
        format!("git commit -F../{name}/secret.txt"),
        format!("git commit -aF ../{name}/secret.txt"),
        // A link of the agent's: its path says nothing.
        "git commit -F lien.txt".to_string(),
    ]
    .iter()
    .enumerate()
    {
        let got = told(&mut p, &[asks(i as u64 + 10, &cwd, script)]);
        assert!(!accepted(&got), "{script}: {got:?}");
    }
    assert!(accepted(&told(
        &mut p,
        &[asks(30, &cwd, "git commit -m x")]
    )));

    // Its fence taken off, a file is no danger.
    let lifted = Attach {
        fence: Vec::new(),
        ..kept(&launch)
    };
    call(&mut p, method::ATTACH, lifted);
    assert!(accepted(&told(
        &mut p,
        &[asks(31, &cwd, "git commit -F msg.txt")]
    )));
}

#[test]
fn a_fenced_agent_after_a_restart_has_the_same_card() {
    let cwd = repository("commit-file-restart");
    let launch = LaunchParams {
        cwd: cwd.clone(),
        mode: "acceptEdits".into(),
        fence: vec![format!("{cwd}-prive")],
        ..params()
    };
    let core = kept(&launch);
    drop(launched(&launch));
    let mut after = started_again(&core);
    let got = told(&mut after, &[asks(10, &cwd, "git commit -F lien.txt")]);
    assert!(!accepted(&got), "{got:?}");
}

#[test]
fn codex_keeps_its_rule_on_an_argv() {
    let cwd = repository("argv");
    let mut p = launched(&LaunchParams {
        cwd: cwd.clone(),
        mode: "acceptEdits".into(),
        ..params()
    });
    // git run as it is, its words its argv: `;` is a letter of its message there, no operator.
    let bare = |id: u64, command: &str, summary: &str| {
        json!({
            "id": id, "method": "item/commandExecution/requestApproval",
            "params": {
                "threadId": "thr", "itemId": format!("c{id}"), "cwd": cwd,
                "command": command, "commandActions": [{ "command": summary }],
            },
        })
        .to_string()
    };
    let argv = "git commit -m 'a; b'";
    let summary = "git commit -m a; b";
    assert!(accepted(&told(&mut p, &[bare(40, argv, summary)])));
    for (i, no) in ["git add .; rm x", "git add . | sh", "git add . > x"]
        .iter()
        .enumerate()
    {
        let got = told(&mut p, &[asks(41 + i as u64, &cwd, no)]);
        assert!(!accepted(&got), "{no}: {got:?}");
    }
}
