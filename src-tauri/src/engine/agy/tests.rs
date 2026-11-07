use super::*;

#[test]
fn the_modes_of_escouade_are_its_modes_or_its_rules_and_say_there_is_no_sandbox() {
    assert_eq!(agy_mode("default"), None);
    assert_eq!(agy_mode("plan"), Some("plan"));
    // As its plugin: `agy`'s accept-edits would let a write in `.git` through unseen (ESC-376).
    assert_eq!(agy_mode("acceptEdits"), None);
    // « Tout accepter » by its rules, never `--dangerously-skip-permissions`.
    assert_eq!(agy_mode("bypassPermissions"), None);
    for mode in MODES {
        let (fr, en) = (meaning(Lang::Fr, mode), meaning(Lang::En, mode));
        assert!(fr != en && fr.contains("bac à sable") && en.contains("sandbox"));
    }
    let c = caps();
    assert!(!c.modes.iter().any(|m| m.id == "auto"));
    assert_eq!(c.fork, ForkPoint::None);
    assert_eq!(c.cost, CostKind::None);
    assert!(c.read_only);
}

#[test]
fn agy_models_makes_a_menu_with_the_efforts_of_gemini_apart() {
    let listed = "Fetching available models...\ngemini-3.8-flash-high\tGemini 3.8 Flash (High)\ngemini-3.8-flash-medium\tGemini 3.8 Flash (Medium)\ngemini-3.8-flash-low\tGemini 3.8 Flash (Low)\nclaude-sonnet-4-6\tClaude Sonnet 4.6 (Thinking)\nclaude-opus-4-6-thinking\tClaude Opus 4.6 (Thinking)\ngpt-oss-120b-medium\tGPT-OSS 120B (Medium)\n";
    let told = models_of(listed);
    let ids: Vec<&str> = told.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "gemini-3.8-flash",
            "claude-sonnet-4-6",
            "claude-opus-4-6-thinking",
            "gpt-oss-120b-medium"
        ]
    );
    assert_eq!(told[0].label, "Gemini 3.8 Flash");
    assert_eq!(told[0].efforts, ["low", "medium", "high"]);
    assert_eq!(told[0].default_effort.as_deref(), Some("medium"));
    assert!(told[0].default);
    // A model of another maker keeps its id, effort included (`gpt-oss-120b-medium`), without a menu.
    assert!(told[3].efforts.is_empty());
    assert!(AGY.supports_effort("gemini-3.8-flash"));
    assert!(!AGY.supports_effort("gemini-3.8-flash-low"));
    assert!(!AGY.supports_effort("claude-sonnet-4-6"));
    assert_eq!(catalog(&[])[0].id, DEFAULT);
}

#[test]
fn its_launch_names_its_model_effort_mode_and_conversation_never_a_token() {
    let s = |model: &str, effort: &str, mode: &str| Settings {
        model: model.into(),
        effort: effort.into(),
        mode: mode.into(),
    };
    assert_eq!(
        launch_args(&s("gemini-3.8-flash", "high", "plan"), Some("abc")),
        [
            "-p=",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--model",
            "gemini-3.8-flash",
            "--effort",
            "high",
            "--mode",
            "plan",
            "--conversation",
            "abc"
        ]
    );
    // A model whose id says its effort takes none beside it; « Demander » and « Tout accepter » no
    // `--mode`.
    let a = launch_args(
        &s("gemini-3.8-flash-low", "high", "bypassPermissions"),
        None,
    );
    assert!(!a.contains(&"--effort".to_string()) && !a.contains(&"--mode".to_string()));
    assert!(!a
        .iter()
        .any(|x| x.contains("dangerously") || x.contains("Bearer")));
}

#[test]
fn the_rules_of_each_mode_hold_the_fence_first() {
    let rules = AgyRules {
        fence: vec!["C:\\x\\privé\\".into()],
        read_only: false,
    };
    // As its plugin (ESC-376): no rule `allow` but in « Tout accepter » without a fence nor a discussion.
    for mode in ["default", "acceptEdits", "plan", "bypassPermissions"] {
        assert_eq!(mode_rules(&rules, mode), Vec::<String>::new(), "{mode}");
    }
    let unfenced = AgyRules {
        fence: Vec::new(),
        ..rules.clone()
    };
    for mode in ["default", "acceptEdits", "plan"] {
        assert_eq!(mode_rules(&unfenced, mode), Vec::<String>::new(), "{mode}");
    }
    assert!(mode_rules(&unfenced, "bypassPermissions").contains(&"command(*)".to_string()));
    let discussion = AgyRules {
        read_only: true,
        ..unfenced.clone()
    };
    assert_eq!(
        mode_rules(&discussion, "bypassPermissions"),
        Vec::<String>::new()
    );
    // Refused to read, refused to write (`agy`'s rule); `deny` before all.
    assert_eq!(deny_rules(&rules), ["read_file(C:\\x\\privé)"]);
    let discussion = AgyRules {
        read_only: true,
        ..rules.clone()
    };
    assert!(deny_rules(&discussion).contains(&"command(*)".to_string()));

    let dir = crate::paths::test_dir("esc270-agy-settings");
    let file = settings_file(&dir);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    // What `agy` keeps there is kept.
    std::fs::write(&file, r#"{"trustedWorkspaces":["C:\\x"]}"#).unwrap();
    write_settings(
        &dir,
        &rules,
        "bypassPermissions",
        &["command(npm test)".into()],
    )
    .unwrap();
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(doc["trustedWorkspaces"][0], "C:\\x");
    let allow = doc["permissions"]["allow"].as_array().unwrap();
    // Fenced: none of the mode's, what the user allowed alone.
    assert_eq!(allow, &[json!("command(npm test)")]);
    assert_eq!(
        doc["permissions"]["deny"],
        json!(["read_file(C:\\x\\privé)"])
    );
}

#[test]
fn escouades_server_is_in_its_own_mcp_config_and_taken_out_after() {
    let dir = crate::paths::test_dir("esc270-agy-grant");
    write_grant(&dir, Some(("http://127.0.0.1:1/mcp", "s3cr3t"))).unwrap();
    let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(mcp_file(&dir)).unwrap()).unwrap();
    let e = &doc["mcpServers"]["escouade"];
    assert_eq!(e["serverUrl"], "http://127.0.0.1:1/mcp");
    assert_eq!(e["headers"]["Authorization"], "Bearer s3cr3t");
    remove_grant(&dir);
    let after = std::fs::read_to_string(mcp_file(&dir)).unwrap();
    assert!(!after.contains("s3cr3t") && !after.contains("escouade"));
    let launch = AGY.escouade_launch(&Escouade::Granted {
        url: "http://x".into(),
        token: "s3cr3t".into(),
        file: None,
    });
    assert!(launch.args.is_empty() && launch.env.is_empty());
}

#[test]
fn each_refused_call_has_its_rule_and_a_fence_names_its_paths() {
    let r = |tool: &str, args: Value| rule_of(tool, &args).map(|(r, _)| r);
    assert_eq!(
        r("run_command", json!({ "CommandLine": "npm test" })).as_deref(),
        Some("command(npm test)")
    );
    assert_eq!(
        r("write_file", json!({ "Path": "C:\\x\\b.txt" })).as_deref(),
        Some("write_file(C:\\x\\b.txt)")
    );
    assert_eq!(
        r(
            "call_mcp_tool",
            json!({ "ServerName": "escouade", "ToolName": "get_board" })
        )
        .as_deref(),
        Some("mcp(escouade/get_board)")
    );
    assert_eq!(
        r(
            "read_url_content",
            json!({ "Url": "https://example.com/a?b" })
        )
        .as_deref(),
        Some("read_url(example.com)")
    );
    assert_eq!(r("browser_click_element", json!({})), None);
    assert!(fenced(
        &json!({ "Path": "C:\\x\\privé\\secret.txt" }),
        &["C:\\x\\privé".into()]
    ));
    assert!(!fenced(
        &json!({ "Path": "C:\\x\\repo\\a.txt" }),
        &["C:\\x\\privé".into()]
    ));
}

#[test]
fn a_write_has_its_diff_from_its_arguments_and_the_plan_its_file() {
    let w = acp_call(
        "write_file",
        &json!({ "Path": "C:\\x\\b.txt", "Content": "un" }),
    );
    assert_eq!(w["kind"], "edit");
    assert_eq!(w["content"][0]["newText"], "un");
    assert!(w["content"][0]["oldText"].is_null());
    let e = acp_call(
        "edit_file",
        &json!({ "Path": "C:\\x\\a.txt", "Target": "bonjour", "Replacement": "salut" }),
    );
    assert_eq!(e["content"][0]["oldText"], "bonjour");
    let c = acp_call(
        "call_mcp_tool",
        &json!({ "ServerName": "proj", "ToolName": "ping", "Arguments": {} }),
    );
    assert_eq!(c["rawInput"]["providerIdentifier"], "proj");
    assert!(is_plan_file(
        "write_file",
        &json!({ "Path": "C:\\h\\.gemini\\antigravity-cli\\brain\\id\\plan_creation.md" })
    ));
    assert!(!is_plan_file(
        "write_file",
        &json!({ "Path": "C:\\x\\plan.md" })
    ));
}

#[test]
fn a_question_and_its_answers_and_what_goes_on_after() {
    let args = json!({ "questions": [{ "question": "Bleu ou rouge ?", "options": ["(Recommended) Bleu", "Rouge"], "is_multi_select": false }] });
    let p = question_params("c1", &args);
    assert_eq!(p["questions"][0]["prompt"], "Bleu ou rouge ?");
    assert_eq!(p["questions"][0]["options"][0]["label"], "Bleu");
    let said = replies(&p, &json!({ "Bleu ou rouge ?": "Rouge" }));
    assert_eq!(said, ["- Bleu ou rouge ? → Rouge"]);
    let text = follow_up(&["the command `npm test`".into()], &[], &said).unwrap();
    assert!(text.contains("allowed the command `npm test`") && text.contains("→ Rouge"));
    assert_eq!(follow_up(&[], &["x".into()], &[]), None);
}

#[test]
fn its_home_is_the_agents_and_the_keys_of_the_machine_are_not_given() {
    let home = std::path::Path::new("C:\\data\\agent-homes\\a1");
    let env = home_env(
        &[
            (HOME_VAR.into(), home.to_string_lossy().into()),
            ("USERPROFILE".into(), "C:\\Users\\x".into()),
            ("A".into(), "b".into()),
        ],
        home,
    );
    let get = |k: &str| {
        env.iter()
            .rev()
            .find(|(e, _)| e == k)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(get("USERPROFILE"), Some("C:\\data\\agent-homes\\a1"));
    assert_eq!(get("HOME"), Some("C:\\data\\agent-homes\\a1"));
    assert_eq!(get("A"), Some("b"));
    assert!(get(HOME_VAR).is_none());
    assert_eq!(KEY_VARS, ["GEMINI_API_KEY", "GOOGLE_API_KEY"]);
}

#[test]
fn a_message_takes_text_files_and_refuses_an_image() {
    let file = |name: &str, media: &str, data: &str| Attachment {
        name: name.into(),
        media_type: media.into(),
        data: data.into(),
    };
    let m = message("Regarde", &[file("notes.txt", "text/plain", "bonjour")]).unwrap();
    assert_eq!(m["text"], "notes.txt:\n\nbonjour\n\nRegarde");
    assert!(message("x", &[file("a.png", "image/png", "iVBORw0KGgo=")]).is_err());
}

#[test]
fn its_usage_limits_are_told_apart_with_the_wait_they_tell() {
    // Its quota, in its words (ESC-269): no reset told, an hour.
    assert_eq!(
        limit_of("You have exhausted your quota on this model."),
        Some(QUOTA_RESUME_MS)
    );
    assert_eq!(limit_of("individual quota reached"), Some(QUOTA_RESUME_MS));
    assert_eq!(
        limit_of("RESOURCE_EXHAUSTED: Weekly Limit, refreshes in 2h 5m"),
        Some(2 * 3_600_000 + 5 * 60_000)
    );
    assert_eq!(
        limit_of("429 Too Many Requests, retry after 1d"),
        Some(86_400_000)
    );
    // The model's capacity: a minute, or what it says.
    assert_eq!(
        limit_of("MODEL_CAPACITY_EXHAUSTED"),
        Some(CAPACITY_RESUME_MS)
    );
    assert_eq!(
        limit_of("MODEL_CAPACITY_EXHAUSTED: retry in 30s"),
        Some(30_000)
    );
    // Another error: none.
    assert_eq!(limit_of("model \"inconnu\" not found"), None);
    assert_eq!(
        limit_of("API key not valid. Please pass a valid API key."),
        None
    );
    assert!(limited_error(Lang::Fr, "x").starts_with("Limite d’usage d’Antigravity atteinte"));
}

#[test]
fn an_account_by_its_key_has_the_gemini_api_as_its_provider_and_a_text_no_tool() {
    let dir = crate::paths::test_dir("esc271-agy-provider");
    let _ = std::fs::remove_dir_all(&dir);
    let key = |v: &str| vec![("GEMINI_API_KEY".to_string(), v.to_string())];
    assert!(by_key(&key("AIza0000")));
    assert!(!by_key(&key("")) && !by_key(&[]));
    // What `agy` keeps there is kept, its provider set then taken out.
    std::fs::create_dir_all(settings_file(&dir).parent().unwrap()).unwrap();
    std::fs::write(settings_file(&dir), r#"{"theme":"dark"}"#).unwrap();
    write_provider(&dir, true).unwrap();
    let read = || -> Value {
        serde_json::from_str(&std::fs::read_to_string(settings_file(&dir)).unwrap()).unwrap()
    };
    assert_eq!(read()["modelProvider"], "gemini");
    assert_eq!(read()["theme"], "dark");
    write_provider(&dir, false).unwrap();
    assert!(read().get("modelProvider").is_none());
    write_no_tools(&dir, false).unwrap();
    let deny = read()["permissions"]["deny"].clone();
    for rule in ["command(*)", "write_file(*)", "read_file(*)", "mcp(*)"] {
        assert!(deny.as_array().unwrap().iter().any(|r| r == rule), "{deny}");
    }
    assert_eq!(read()["permissions"]["allow"], json!([]));
    assert!(caps().accounts_by_key);
}
