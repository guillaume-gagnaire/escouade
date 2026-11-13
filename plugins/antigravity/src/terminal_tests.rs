//! « Vue Antigravity »: the launch of its interface (`terminal/launch`) and the models of an account (`account/read`), as
//! the core's tests of its view had them before it left the core (ESC-359).

use super::*;
use escouade_plugin_protocol::engine::Grant;

const HOOK_UNIX: &str = "/opt/escouade/escouade-hook";
const HOOK_WIN: &str = r"C:\Program Files\Escouade\escouade-hook.exe";
const TOKEN: &str = "jeton-secret-de-l-agent";
const CWD: &str = "/x/repo";

/// A folder of the test's own, empty.
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("escouade-agy-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn params(home: &Path) -> TerminalParams {
    TerminalParams {
        agent: "tui-a1".into(),
        cwd: CWD.into(),
        home: Some(home.to_string_lossy().into_owned()),
        hook_url: "http://127.0.0.1:4120/hook".into(),
        hook_token_var: "ESCOUADE_HOOK_TOKEN".into(),
        hook_url_var: "ESCOUADE_HOOK_URL".into(),
        hook_command: Some(HOOK_UNIX.into()),
        escouade: Some(Grant {
            url: "http://127.0.0.1:4121/mcp".into(),
            token: TOKEN.into(),
        }),
        ..TerminalParams::default()
    }
}

fn pair(a: &[String], k: &str) -> Option<String> {
    a.windows(2).find(|w| w[0] == k).map(|w| w[1].clone())
}

fn env_of<'a>(l: &'a Launch, k: &str) -> Option<&'a str> {
    l.env
        .iter()
        .rfind(|v| v.name == k)
        .map(|v| v.value.as_str())
}

fn file_of(l: &Launch, path: &Path) -> Option<Value> {
    let path = path.to_string_lossy();
    l.files
        .iter()
        .find(|f| f.path == path)
        .map(|f| serde_json::from_str(&f.text).unwrap())
}

#[test]
fn a_conversation_its_store_has_is_resumed_in_its_home_with_its_model_trust_hooks_and_server() {
    let home = temp("launch");
    let p = TerminalParams {
        session: Some("c1".into()),
        resumes: true,
        model: "gemini-3.8-flash".into(),
        effort: "low".into(),
        mode: "acceptEdits".into(),
        ..params(&home)
    };
    let l = launch(&p).launch;
    assert_eq!(pair(&l.args, "--conversation").as_deref(), Some("c1"));
    assert_eq!(
        pair(&l.args, "--model").as_deref(),
        Some("gemini-3.8-flash")
    );
    assert_eq!(pair(&l.args, "--effort").as_deref(), Some("low"));
    // « Édits auto » is no mode of `agy`'s: its interface asks what no rule allows (ESC-376).
    assert!(pair(&l.args, "--mode").is_none(), "{:?}", l.args);
    // The token of Escouade's server is in no argument nor variable: in its MCP config.
    assert!(!l.args.iter().any(|a| a.contains(TOKEN)));
    assert!(!l.env.iter().any(|v| v.value.contains(TOKEN)));
    let mcp = file_of(&l, &mcp_file(&home)).expect("its MCP config");
    assert_eq!(
        mcp["mcpServers"]["escouade"]["serverUrl"],
        "http://127.0.0.1:4121/mcp"
    );
    assert_eq!(
        mcp["mcpServers"]["escouade"]["headers"]["Authorization"],
        format!("Bearer {TOKEN}")
    );
    let home_text = home.to_string_lossy().into_owned();
    assert_eq!(env_of(&l, "USERPROFILE"), Some(home_text.as_str()));
    assert_eq!(env_of(&l, "HOME"), Some(home_text.as_str()));
    assert_eq!(env_of(&l, "AGY_CLI_HIDE_ACCOUNT_INFO"), Some("1"));
    assert_eq!(
        env_of(&l, "ESCOUADE_HOOK_URL"),
        Some("http://127.0.0.1:4120/hook")
    );
    // A key of the machine's would take the account's place.
    assert_eq!(env_of(&l, "GEMINI_API_KEY"), Some(""));
    assert_eq!(env_of(&l, "GOOGLE_API_KEY"), Some(""));
    let settings = file_of(&l, &settings_file(&home)).expect("its settings");
    assert_eq!(settings["trustedWorkspaces"], json!([CWD]));
    assert!(settings["permissions"]["allow"]
        .as_array()
        .unwrap()
        .is_empty());
    let onboarding = file_of(&l, &onboarding_file(&home)).expect("its onboarding");
    assert_eq!(onboarding["onboardingComplete"], true);
    let hooks = file_of(&l, &hooks_file(&home)).expect("its hooks");
    for event in EVENTS {
        let handler = match event.ends_with("ToolUse") {
            true => &hooks["escouade"][event][0]["hooks"][0],
            false => &hooks["escouade"][event][0],
        };
        assert_eq!(handler["command"], format!("'{HOOK_UNIX}' {event}"));
    }
    // Under Windows, a path with a space by its name, its folder first on the PATH; `cmd` never looks in its own folder.
    let win = launch(&TerminalParams {
        hook_command: Some(HOOK_WIN.into()),
        path: Some(r"C:\Windows".into()),
        ..p.clone()
    })
    .launch;
    let hooks = file_of(&win, &hooks_file(&home)).unwrap();
    assert_eq!(
        hooks["escouade"]["Stop"][0]["command"],
        "escouade-hook Stop"
    );
    assert_eq!(
        env_of(&win, "PATH"),
        Some(r"C:\Program Files\Escouade;C:\Windows")
    );
    assert_eq!(env_of(&win, NO_CURRENT_DIR), Some("1"));
    // A path a line holds without quotes: by its whole path, with `/`.
    let plain = launch(&TerminalParams {
        hook_command: Some(r"C:\Users\ana\AppData\Local\Escouade\escouade-hook.exe".into()),
        ..p.clone()
    })
    .launch;
    let hooks = file_of(&plain, &hooks_file(&home)).unwrap();
    assert_eq!(
        hooks["escouade"]["PreToolUse"][0]["hooks"][0]["command"],
        "C:/Users/ana/AppData/Local/Escouade/escouade-hook.exe PreToolUse"
    );
    assert_eq!(env_of(&plain, NO_CURRENT_DIR), Some("1"));
    // Nor a quote, an accent, a `&` or a `%`: by its name.
    for odd in [
        r"C:\Users\Zoé\Escouade\escouade-hook.exe",
        r"C:\a&b\escouade-hook.exe",
        r"C:\a%PATH%\escouade-hook.exe",
        r"C:\O'Brien\escouade-hook.exe",
        r"C:\a(1)\escouade-hook.exe",
    ] {
        let l = launch(&TerminalParams {
            hook_command: Some(odd.into()),
            ..p.clone()
        })
        .launch;
        let hooks = file_of(&l, &hooks_file(&home)).unwrap();
        assert_eq!(
            hooks["escouade"]["Stop"][0]["command"], "escouade-hook Stop",
            "{odd}"
        );
    }
    // Elsewhere, none: a POSIX shell looks in no current folder.
    assert_eq!(env_of(&launch(&p).launch, NO_CURRENT_DIR), None);
}

#[test]
fn a_conversation_without_transcript_is_not_given_its_settings_are_kept_and_a_fence_is_refused() {
    let home = temp("unwritten");
    let settings = settings_file(&home);
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(
        &settings,
        json!({ "theme": "dark", "trustedWorkspaces": [CWD] }).to_string(),
    )
    .unwrap();
    let p = TerminalParams {
        session: Some("c-empty".into()),
        resumes: false,
        model: "gemini-3.8-flash-low".into(),
        effort: "high".into(),
        mode: "bypassPermissions".into(),
        fence: vec!["/secret".into()],
        escouade: None,
        ..params(&home)
    };
    let t = launch(&p).launch;
    assert!(pair(&t.args, "--conversation").is_none(), "{:?}", t.args);
    // Its effort is part of the model's id; « Tout accepter » is its rules', not a mode.
    assert!(pair(&t.args, "--effort").is_none() && pair(&t.args, "--mode").is_none());
    assert!(t.fence_applied);
    let doc = file_of(&t, &settings).unwrap();
    assert_eq!(doc["theme"], "dark");
    assert_eq!(doc["trustedWorkspaces"], json!([CWD]));
    assert_eq!(doc["permissions"]["allow"], json!([]));
    let deny = doc["permissions"]["deny"].as_array().unwrap();
    assert!(
        deny.iter().any(|r| r.as_str().unwrap().contains("secret")),
        "{deny:?}"
    );
    let mcp = file_of(&t, &mcp_file(&home)).unwrap();
    assert!(mcp["mcpServers"].get("escouade").is_none(), "{mcp}");
    // Its temporary folder is the agent's own, made by the launch's files.
    let tmp = home.join("tmp").to_string_lossy().into_owned();
    for var in crate::words::TEMP_VARS {
        assert_eq!(env_of(&t, var), Some(tmp.as_str()), "{var}");
    }
    assert!(t
        .files
        .iter()
        .any(|f| f.path == temp_mark(&home).to_string_lossy()));
    // Its first screens already passed are not written again.
    std::fs::create_dir_all(onboarding_file(&home).parent().unwrap()).unwrap();
    std::fs::write(onboarding_file(&home), r#"{"onboardingComplete":true}"#).unwrap();
    assert!(file_of(&launch(&p).launch, &onboarding_file(&home)).is_none());
}

#[test]
fn the_models_of_an_account_are_what_agy_models_lists_in_a_home_of_its_own() {
    let r = AccountRead {
        read: "models".into(),
        home: Some("/tmp/agy-read".into()),
        env_names: vec!["GEMINI_API_KEY".into()],
        ..AccountRead::default()
    };
    let step = read(&r);
    let l = step.launch.expect("a process");
    assert_eq!(l.args, ["models"]);
    assert_eq!(env_of(&l, "HOME"), Some("/tmp/agy-read"));
    // The account's key stays, the machine's other one is emptied (ESC-433), as in its interface.
    assert_eq!(env_of(&l, "GEMINI_API_KEY"), None);
    assert_eq!(env_of(&l, "GOOGLE_API_KEY"), Some(""));
    // An account signed in by Google: both keys of the machine's emptied, its models are its own.
    let google = read(&AccountRead {
        env_names: Vec::new(),
        ..r.clone()
    })
    .launch
    .unwrap();
    assert_eq!(env_of(&google, "GEMINI_API_KEY"), Some(""));
    assert_eq!(env_of(&google, "GOOGLE_API_KEY"), Some(""));
    let settings = &l.files[0];
    assert!(
        settings.text.contains("\"modelProvider\": \"gemini\""),
        "{}",
        settings.text
    );
    let lines = AccountLines {
        read: "models".into(),
        state: step.state,
        lines: vec![
            "gemini-3.8-flash-high\tGemini 3.8 Flash (High)".into(),
            "gemini-3.8-flash-low\tGemini 3.8 Flash (Low)".into(),
            "claude-sonnet\tClaude Sonnet".into(),
        ],
        ended: true,
        code: Some(0),
    };
    let told = lines_of(&lines);
    assert_eq!(told[0]["id"], "gemini-3.8-flash");
    assert_eq!(told[0]["efforts"], json!(["low", "high"]));
    assert_eq!(told[0]["default"], true);
    assert_eq!(told[1]["id"], "claude-sonnet");
    let none = AccountLines {
        lines: Vec::new(),
        ..lines
    };
    assert!(super::lines(&none).error.is_some());
}

fn lines_of(l: &AccountLines) -> Vec<Value> {
    super::lines(l).done.unwrap().as_array().unwrap().clone()
}

#[test]
fn a_text_is_written_in_its_own_home_every_tool_refused_its_provider_the_accounts() {
    let p = OneShotParams {
        weight: "given".into(),
        cwd: CWD.into(),
        system: "Fais court.".into(),
        prompt: "Nomme ce ticket".into(),
        model: String::new(),
        home: Some("/tmp/agy-text".into()),
        env_names: vec!["GEMINI_API_KEY".into()],
    };
    let l = one_shot(&p).unwrap();
    // A text given has the model of a reading (ESC-66).
    assert_eq!(
        pair(&l.args, "--model").as_deref(),
        Some("gemini-3.8-flash-medium")
    );
    assert_eq!(text_model("light"), "gemini-3.8-flash-low");
    assert!(l.args.iter().any(|a| a == "-p="));
    assert_eq!(
        l.env
            .iter()
            .find(|v| v.name == "HOME")
            .map(|v| v.value.as_str()),
        Some("/tmp/agy-text")
    );
    let settings: Value = serde_json::from_str(&l.files[0].text).unwrap();
    assert_eq!(settings["modelProvider"], "gemini");
    for rule in ["command(*)", "write_file(*)", "read_file(*)", "mcp(*)"] {
        assert!(settings["permissions"]["deny"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == rule));
    }
    // On its input as JSON: its line breaks escaped.
    assert!(l
        .input
        .clone()
        .unwrap()
        .contains(r"<instructions>\nFais court.\n</instructions>"));
    // Its Principal signs in by Google: no provider.
    let google = one_shot(&OneShotParams {
        env_names: Vec::new(),
        ..p.clone()
    })
    .unwrap();
    assert!(!google.files[0].text.contains("modelProvider"));
    // What no family covers is reviewed (refused in `-p`), whatever `agy`'s default; a key of the machine's is taken
    // away, the account's kept.
    assert_eq!(settings["toolPermission"], crate::words::TOOL_PERMISSION);
    let emptied =
        |l: &OneShotLaunch, k: &str| l.env.iter().any(|v| v.name == k && v.value.is_empty());
    assert!(!emptied(&l, "GEMINI_API_KEY") && emptied(&l, "GOOGLE_API_KEY"));
    assert!(emptied(&google, "GEMINI_API_KEY") && emptied(&google, "GOOGLE_API_KEY"));
    assert!(one_shot(&OneShotParams { home: None, ..p }).is_err());
    // Its answer, or why not.
    let out = |stdout: &str| OneShotOutput {
        stdout: stdout.into(),
        stderr: String::new(),
        code: Some(0),
    };
    let said = r#"{"event":"result","result":{"status":"OK","response":" Le titre "}}"#;
    assert_eq!(one_shot_read(&out(said), "fr").unwrap().text, "Le titre");
    let failed = r#"{"event":"result","result":{"status":"ERROR","error":"quota"}}"#;
    assert_eq!(one_shot_read(&out(failed), "fr").unwrap_err(), "quota");
    assert_eq!(
        one_shot_read(&out(""), "en").unwrap_err(),
        "Antigravity CLI gave no answer"
    );
}

/// ESC-433: only a whole folder goes first on the `PATH`; `C:` alone (the drive's current folder), a relative folder or
/// one with `;` or `"` would put another folder first, and the `PATH` is left as it is.
#[test]
fn only_a_whole_folder_of_escouade_hook_goes_first_on_the_path() {
    let path = r"C:\Windows";
    assert_eq!(
        path_with(r"C:\escouade-hook.exe", path).as_deref(),
        Some(r"C:\;C:\Windows")
    );
    assert_eq!(
        path_with(r"\\host\share\escouade-hook.exe", path).as_deref(),
        Some(r"\\host\share;C:\Windows")
    );
    for odd in [
        r"C:escouade-hook.exe",
        r"C:a\escouade-hook.exe",
        r"Escouade\escouade-hook.exe",
        r"\Escouade\escouade-hook.exe",
        r"C:\a;b\escouade-hook.exe",
        r#"C:\a"b\escouade-hook.exe"#,
        r"\\?\C:\Escouade\escouade-hook.exe",
    ] {
        assert_eq!(path_with(odd, path), None, "{odd}");
    }
    // Its view keeps `cmd` from its current folder all the same, the `PATH` it had left alone.
    let dir = temp("path-first");
    let l = launch(&TerminalParams {
        hook_command: Some(r"C:\a;b\escouade-hook.exe".into()),
        path: Some(path.into()),
        ..params(&dir)
    })
    .launch;
    assert_eq!(env_of(&l, NO_CURRENT_DIR), Some("1"));
    assert_eq!(env_of(&l, "PATH"), None);
}
