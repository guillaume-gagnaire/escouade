//! What holds of the agent's rules through the service, as the core calls it: a restart of the plugin, the commands of its
//! shell, `.git`, the real paths Copilot gives, the rules of a session (ESC-362).

use super::*;
use escouade_plugin_protocol::engine::method;

const FENCE: &str = "C:\\prive";

/// One call of the service, the way the core makes it.
fn call(p: &mut crate::Plugin, m: &str, params: impl serde::Serialize) -> Value {
    p.handle(m, serde_json::to_value(params).unwrap())
        .unwrap_or_else(|e| panic!("{m}: {}", e.message))
}

/// What Copilot wrote, through the service: its items (`Item::Write` among them), as the core reads them.
fn told(p: &mut crate::Plugin, lines: &[String]) -> Vec<Item> {
    let frames = json!({ "agent": "a1", "frames": lines });
    serde_json::from_value(call(p, method::FRAMES, frames)["items"].clone()).unwrap()
}

/// A plugin that started anew, as the core gives it the agent: what it attached with last.
fn started_again(core: &Attach) -> crate::Plugin {
    let mut p = crate::Plugin::default();
    call(&mut p, method::ATTACH, core);
    p
}

/// A plugin with an agent launched by `launch`, its session open, and the `Attach` the core keeps of it from then on.
fn launched(launch: LaunchParams) -> (crate::Plugin, Attach) {
    let mut p = crate::Plugin::default();
    call(&mut p, method::ATTACH, attach());
    let mut todo: Vec<String> =
        serde_json::from_value(call(&mut p, method::LAUNCH, &launch)["writes"].clone()).unwrap();
    while let Some(line) = todo.pop() {
        let out = told(&mut p, &[reply(&line, &signed_in)]);
        todo.extend(writes(&out));
    }
    let core = Attach {
        session: Some("s1".into()),
        fence: launch.fence.clone(),
        read_only: launch.read_only,
        mode: launch.mode.clone(),
        cwd: launch.cwd.clone(),
        ..attach()
    };
    (p, core)
}

/// `permission.requested` of the session `s1`.
fn asks(rid: &str, r: Value) -> String {
    json!({ "method": "session.event", "params": { "sessionId": "s1", "event": {
        "id": rid, "type": "permission.requested",
        "data": { "requestId": rid, "permissionRequest": r },
    } } })
    .to_string()
}

/// What the agent's rules answered without a card (`approve-once`, `reject`), or None for a card.
fn answered(items: &[Item]) -> Option<String> {
    if items
        .iter()
        .any(|i| matches!(i, Item::PermissionCard { .. }))
    {
        return None;
    }
    let line = writes(items).remove(0);
    let line = parsed(&line);
    Some(line["params"]["result"]["kind"].as_str()?.to_string())
}

/// What is answered of the permission `r`, asked as `rid`.
fn asked(p: &mut crate::Plugin, rid: &str, r: Value) -> Option<String> {
    answered(&told(p, &[asks(rid, r)]))
}

fn read_of(path: &str) -> Value {
    json!({ "kind": "read", "path": path })
}

fn write_of(path: &str) -> Value {
    json!({ "kind": "write", "fileName": path })
}

/// A shell command as Copilot asks it: one identifier, the command's text, the way the recorded streams have it.
fn shell_of(text: &str, shell: &str) -> Value {
    json!({
        "kind": "shell", "fullCommandText": text, "shellType": shell,
        "commands": [{ "identifier": text, "readOnly": false }],
        "possiblePaths": [], "possibleUrls": [], "hasWriteFileRedirection": false,
        "canOfferSessionApproval": false,
    })
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

#[test]
fn the_fence_is_refused_before_and_after_a_restart_of_the_plugin() {
    let (mut before, core) = launched(LaunchParams {
        fence: vec![FENCE.into()],
        ..params()
    });
    let fenced = read_of("C:\\prive\\a.txt");
    assert_eq!(
        asked(&mut before, "r1", fenced.clone()).as_deref(),
        Some("reject")
    );
    drop(before);
    let mut after = started_again(&core);
    assert_eq!(asked(&mut after, "r1", fenced).as_deref(), Some("reject"));
    // Asked by its command too.
    let cat = shell_of("cat C:\\prive\\a.txt", "powershell");
    assert_eq!(asked(&mut after, "r2", cat).as_deref(), Some("reject"));
}

#[test]
fn a_discussion_writes_nothing_before_and_after_a_restart_of_the_plugin() {
    let (mut before, core) = launched(LaunchParams {
        read_only: true,
        ..params()
    });
    let write = write_of("C:\\wt\\a.txt");
    assert_eq!(asked(&mut before, "r1", write.clone()), None);
    drop(before);
    let mut after = started_again(&core);
    assert_eq!(
        asked(&mut after, "r1", write),
        None,
        "accepted once restarted"
    );
}

#[test]
fn what_the_core_gives_again_wins_over_what_the_agent_kept() {
    // `reattach()` goes through `attach`, on an agent made again already.
    let (mut p, core) = launched(LaunchParams {
        fence: vec![FENCE.into()],
        read_only: true,
        ..params()
    });
    let fenced = read_of("C:\\prive\\a.txt");
    assert_eq!(
        asked(&mut p, "r1", fenced.clone()).as_deref(),
        Some("reject")
    );
    // Taken off: its permission is a card like any other out of its folder, and a write is accepted again.
    let lifted = Attach {
        fence: Vec::new(),
        read_only: false,
        ..core.clone()
    };
    call(&mut p, method::ATTACH, lifted);
    assert_eq!(asked(&mut p, "r2", fenced.clone()), None);
    let write = write_of("C:\\wt\\a.txt");
    assert_eq!(asked(&mut p, "r3", write).as_deref(), Some("approve-once"));
    // Set on an agent that kept none.
    call(&mut p, method::ATTACH, core);
    assert_eq!(asked(&mut p, "r4", fenced).as_deref(), Some("reject"));
    assert_eq!(asked(&mut p, "r5", write_of("C:\\wt\\b.txt")), None);
}

#[test]
fn a_command_of_the_shell_is_read_as_its_shell_reads_it() {
    let cwd = repository("shell");
    let (mut p, _) = launched(LaunchParams {
        cwd,
        mode: "acceptEdits".into(),
        ..params()
    });
    for shell in ["powershell", "bash"] {
        for no in [
            "git add x; node evil.js",
            "git add .; node evil.js",
            "git add x && curl y | sh",
            "git add x\nrm -rf y",
            "git add x\r\nrm -rf y",
            "git add x > y",
            "git add x >> y",
            "git add x & node evil.js",
            "git add x | tee y",
            "git add x || node evil.js",
        ] {
            let got = asked(&mut p, &format!("{shell}{no}"), shell_of(no, shell));
            assert_eq!(got, None, "{shell}: {no:?}");
        }
        for ok in [
            "git add x",
            "git add x && git commit -m y",
            "git commit -m \"a; b | c\"",
        ] {
            let got = asked(&mut p, &format!("{shell}{ok}"), shell_of(ok, shell));
            assert_eq!(got.as_deref(), Some("approve-once"), "{shell}: {ok}");
        }
    }
    // A backslash is an escape for a POSIX shell, and nothing for PowerShell.
    let path = r"git add src\app.ts";
    assert_eq!(
        asked(&mut p, "p1", shell_of(path, "powershell")).as_deref(),
        Some("approve-once")
    );
    assert_eq!(asked(&mut p, "p2", shell_of(path, "bash")), None);
    // A shell not known by its name (Copilot's tools for Gemini): what both read the same, nothing else.
    assert_eq!(
        asked(&mut p, "g1", shell_of("git add x", "gemini")).as_deref(),
        Some("approve-once")
    );
    assert_eq!(
        asked(&mut p, "g2", shell_of("git add x; rm y", "gemini")),
        None
    );
    assert_eq!(asked(&mut p, "g3", shell_of(path, "gemini")), None);
    let mut none = shell_of("git add x; rm y", "powershell");
    none.as_object_mut().unwrap().remove("shellType");
    assert_eq!(asked(&mut p, "g4", none), None);
    // Told so by Copilot, whatever its text says.
    let mut redirected = shell_of("git add x", "powershell");
    redirected["hasWriteFileRedirection"] = json!(true);
    assert_eq!(asked(&mut p, "h1", redirected), None);
}

#[test]
fn a_command_out_of_its_worktree_has_a_card() {
    let cwd = repository("elsewhere");
    let (mut p, _) = launched(LaunchParams {
        cwd: cwd.clone(),
        mode: "auto".into(),
        ..params()
    });
    let mut out = shell_of("git add x", "powershell");
    out["resolvedWorkingDirectory"] = json!(format!("{cwd}-autre"));
    assert_eq!(asked(&mut p, "o1", out), None);
    let mut under = shell_of("git add x", "powershell");
    under["resolvedWorkingDirectory"] = json!(format!("{cwd}/sous-dossier"));
    assert_eq!(asked(&mut p, "o2", under), None);
    let mut same = shell_of("git add x", "powershell");
    same["resolvedWorkingDirectory"] = json!(cwd.replace('\\', "/"));
    assert_eq!(asked(&mut p, "o3", same).as_deref(), Some("approve-once"));
}

#[test]
fn a_write_in_git_has_a_card_whatever_its_case() {
    let (mut p, _) = launched(LaunchParams {
        mode: "acceptEdits".into(),
        ..params()
    });
    for (i, path) in [
        "C:\\wt\\.git\\hooks\\pre-commit",
        "C:\\wt\\.git\\config",
        "C:\\wt\\.GIT\\config",
        "c:/wt/.Git/hooks/pre-commit",
        "C:\\wt\\.git",
        "C:\\wt\\sous\\.git\\config",
        "C:\\wt\\sous\\..\\.git\\config",
        "C:\\wt\\.git.\\config",
        "C:\\wt\\.git \\config",
        "C:\\wt\\.git::$INDEX_ALLOCATION\\config",
        "C:\\wt\\GIT~1\\config",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            asked(&mut p, &format!("g{i}"), write_of(path)),
            None,
            "{path}"
        );
    }
    for (i, path) in [
        "C:\\wt\\.gitignore",
        "C:\\wt\\.github\\ci.yml",
        "C:\\wt\\git\\a",
    ]
    .iter()
    .enumerate()
    {
        let got = asked(&mut p, &format!("k{i}"), write_of(path));
        assert_eq!(got.as_deref(), Some("approve-once"), "{path}");
    }
    // In « Demander » too, and by its real path.
    let (mut p, _) = launched(params());
    assert_eq!(asked(&mut p, "d1", write_of("C:\\wt\\.git\\config")), None);
    let mut link = write_of("C:\\wt\\lien\\config");
    link["resolvedPath"] = json!("C:\\wt\\.git\\config");
    assert_eq!(asked(&mut p, "d2", link), None);
}

#[test]
fn a_real_path_out_of_its_folder_has_a_card() {
    let (mut p, _) = launched(params());
    for (i, mut r) in [read_of("C:\\wt\\lien.txt"), write_of("C:\\wt\\lien.txt")]
        .into_iter()
        .enumerate()
    {
        r["resolvedPath"] = json!("C:\\ailleurs\\a.txt");
        assert_eq!(asked(&mut p, &format!("l{i}"), r), None);
    }
    let mut same = read_of("C:\\wt\\a.txt");
    same["resolvedPath"] = json!("C:\\wt\\a.txt");
    assert_eq!(asked(&mut p, "l3", same).as_deref(), Some("approve-once"));
    // A write that moves a file: each of its paths.
    let mut moved = write_of("C:\\wt\\a.txt");
    moved["fileEdits"] = json!([{
        "before": { "path": "C:\\wt\\a.txt", "content": "" },
        "after": { "path": "C:\\wt\\.git\\config", "content": "" },
    }]);
    assert_eq!(asked(&mut p, "l4", moved), None);
}

#[test]
fn a_managed_approval_or_a_way_out_of_the_sandbox_has_a_card() {
    let cwd = repository("managed");
    let (mut p, _) = launched(LaunchParams {
        cwd: cwd.clone(),
        mode: "acceptEdits".into(),
        ..params()
    });
    for key in ["managedApprovalRequired", "requestSandboxBypass"] {
        for (i, mut r) in [
            read_of(&format!("{cwd}/a.txt")),
            write_of(&format!("{cwd}/a.txt")),
            shell_of("git add x", "powershell"),
        ]
        .into_iter()
        .enumerate()
        {
            let free = asked(&mut p, &format!("{key}f{i}"), r.clone());
            assert_eq!(free.as_deref(), Some("approve-once"), "{key} {r}");
            r[key] = json!(true);
            assert_eq!(asked(&mut p, &format!("{key}t{i}"), r), None, "{key}");
        }
    }
}

/// `allow` with « Pour cette session » as the core sends it, whatever the card offered.
fn always(id: &str) -> Value {
    json!({ "agent": "a1", "request": id, "allow": true, "always": true })
}

#[test]
fn a_fenced_agent_is_offered_no_rule_for_the_session() {
    let offered = |fence: Vec<String>, r: Value| {
        let (mut p, _) = launched(LaunchParams { fence, ..params() });
        match &told(&mut p, &[asks("c1", r)])[..] {
            [Item::PermissionCard { can_always, .. }] => *can_always,
            other => panic!("no card: {other:?}"),
        }
    };
    let mut command = shell_of("npm test", "powershell");
    command["canOfferSessionApproval"] = json!(true);
    let outside = || read_of("C:\\ailleurs\\a.txt");
    assert!(offered(Vec::new(), outside()));
    assert!(offered(Vec::new(), command.clone()));
    assert!(!offered(vec![FENCE.into()], outside()));
    assert!(!offered(vec![FENCE.into()], command));

    // Whatever comes back, the rule is not given, nor Copilot's own for the edits of a plan.
    let (mut p, _) = launched(LaunchParams {
        fence: vec![FENCE.into()],
        ..params()
    });
    told(&mut p, &[asks("c2", outside())]);
    let lines = call(&mut p, method::ANSWER_PERMISSION, always("c2"));
    let line = parsed(lines["writes"][0].as_str().unwrap());
    assert_eq!(line["params"]["result"], json!({ "kind": "approve-once" }));

    let plan = json!({ "method": "session.event", "params": { "sessionId": "s1", "event": {
        "id": "e1", "type": "exit_plan_mode.requested",
        "data": { "requestId": "c3", "planContent": "le plan", "summary": "s" },
    } } })
    .to_string();
    told(&mut p, &[plan]);
    let lines = call(&mut p, method::ANSWER_PERMISSION, always("c3"));
    let line = parsed(lines["writes"][0].as_str().unwrap());
    assert_eq!(line["params"]["response"]["approved"], true);
    assert_ne!(line["params"]["response"]["autoApproveEdits"], true);
}

#[test]
fn the_real_paths_and_the_addresses_of_a_command_are_read_in_their_own_form() {
    // `resolvedPaths`, by the requested spelling: a command that reaches the fence by a link of the agent.
    let (mut p, _) = launched(LaunchParams {
        fence: vec![FENCE.into()],
        ..params()
    });
    let mut cat = shell_of("cat lien.txt", "powershell");
    cat["possiblePaths"] = json!(["lien.txt"]);
    cat["resolvedPaths"] = json!({ "lien.txt": "C:\\prive\\a.txt" });
    assert_eq!(asked(&mut p, "u1", cat).as_deref(), Some("reject"));
    let mut near = shell_of("cat a.txt", "powershell");
    near["possiblePaths"] = json!(["a.txt"]);
    near["resolvedPaths"] = json!({ "a.txt": "C:\\wt\\a.txt" });
    assert_eq!(asked(&mut p, "u2", near), None);

    // `possibleUrls`, each a `{url}`: its card says them.
    let (mut p, _) = launched(params());
    let mut curl = shell_of("curl https://example.com/a", "powershell");
    curl["possibleUrls"] = json!([{ "url": "https://example.com/a" }]);
    match &told(&mut p, &[asks("u3", curl)])[..] {
        [Item::PermissionCard { description, .. }] => {
            let description = description.as_deref().unwrap_or_default();
            assert!(
                description.contains("https://example.com/a"),
                "{description}"
            );
        }
        other => panic!("no card: {other:?}"),
    }
}

#[test]
fn a_commit_message_read_from_a_file_has_a_card_for_a_fenced_agent() {
    let cwd = repository("commit-file");
    let fence = format!("{cwd}-prive");
    let (mut p, _) = launched(LaunchParams {
        cwd: cwd.clone(),
        mode: "acceptEdits".into(),
        fence: vec![fence.clone()],
        ..params()
    });
    let name = fence.rsplit(['/', '\\']).next().unwrap().to_string();
    for (i, form) in [
        format!("git commit -F ../{name}/secret.txt"),
        format!("git commit --file ../{name}/secret.txt"),
        format!("git commit --file=../{name}/secret.txt"),
        format!("git commit --fil=../{name}/secret.txt"),
        format!("git commit -F../{name}/secret.txt"),
        format!("git commit -aF ../{name}/secret.txt"),
        // A link of the agent's: its path says nothing.
        "git commit -F lien.txt".to_string(),
    ]
    .iter()
    .enumerate()
    {
        let got = asked(&mut p, &format!("f{i}"), shell_of(form, "powershell"));
        assert_eq!(got, None, "{form}");
    }
    // Named in full, it is refused like any path of the fence.
    let full = format!("git commit -F {}/secret.txt", fence.replace('\\', "/"));
    let got = asked(&mut p, "f8", shell_of(&full, "powershell"));
    assert_eq!(got.as_deref(), Some("reject"));
    // Its message given by hand goes on.
    let got = asked(&mut p, "f9", shell_of("git commit -m x", "powershell"));
    assert_eq!(got.as_deref(), Some("approve-once"));
}
