//! The engine of Gemini CLI alone: its modes, models, rules, settings and questions.

use super::*;

#[test]
fn each_mode_of_escouade_has_its_mode_in_gemini_and_says_there_is_no_sandbox() {
    assert_eq!(gemini_mode("default"), "default");
    assert_eq!(gemini_mode("plan"), "plan");
    assert_eq!(gemini_mode("acceptEdits"), "autoEdit");
    assert_eq!(gemini_mode("bypassPermissions"), "yolo");
    for mode in MODES {
        assert_eq!(escouade_mode(gemini_mode(mode)), Some(mode));
        let (fr, en) = (meaning(Lang::Fr, mode), meaning(Lang::En, mode));
        assert!(fr != en && fr.contains("bac à sable") && en.contains("sandbox"));
    }
    assert_eq!(mode_update("[MODE_UPDATE] plan"), Some("plan"));
    assert_eq!(mode_update("[MODE_UPDATE] autoEdit"), Some("acceptEdits"));
    assert_eq!(mode_update("Le mode [MODE_UPDATE] plan"), None);
    let c = caps();
    assert!(!c.modes.iter().any(|m| m.id == "auto"));
    assert_eq!(c.fork, ForkPoint::None);
    assert!(!c.effort && c.read_only);
    assert!(!GEMINI.supports_auto(DEFAULT) && !GEMINI.supports_effort(DEFAULT));
}

#[test]
fn the_models_of_a_session_make_the_menu() {
    let new = bench::stream("02-simple-turn.jsonl")
        .into_iter()
        .find(|m| m["result"]["models"].is_object())
        .unwrap();
    let told = models_of(&new["result"]["models"]);
    assert!(told.len() >= 2, "{told:?}");
    assert_eq!(told[0].id, "auto");
    assert!(told[0].default && told.iter().filter(|m| m.default).count() == 1);
    assert!(told.iter().all(|m| m.efforts.is_empty()));
    assert_eq!(catalog(&[])[0].id, DEFAULT);
}

#[test]
fn the_rules_refuse_the_fence_to_its_file_tools_and_commands_whatever_the_separators() {
    assert_eq!(policy(&[], false), None);
    let rules = policy(&["C:\\x\\Privé.v1".into()], false).unwrap();
    let doc: toml_edit::DocumentMut = rules.parse().expect("TOML");
    let list = doc["rule"].as_array_of_tables().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list.get(0).unwrap()["decision"].as_str(), Some("deny"));
    assert_eq!(
        list.get(1).unwrap()["toolName"].as_str(),
        Some("run_shell_command")
    );
    let pattern = list.get(0).unwrap()["argsPattern"]
        .as_str()
        .unwrap()
        .to_string();
    // What JavaScript's RegExp finds (Gemini's), asked of node.
    let finds = |text: &str| {
        std::process::Command::new("node")
            .args([
                "-e",
                "process.exit(new RegExp(process.argv[1]).test(process.argv[2]) ? 0 : 1)",
                &pattern,
                text,
            ])
            .status()
            .unwrap()
            .success()
    };
    assert!(finds(r#"{"file_path":"C:\\x\\Privé.v1\\secret.txt"}"#));
    assert!(finds(r#"{"file_path":"C:/x//Privé.v1"}"#));
    assert!(!finds(r#"{"file_path":"C:\\x\\PrivéXv1"}"#));
    if cfg!(windows) {
        assert!(finds(r#"{"file_path":"c:\\X\\privé.V1"}"#));
    }
    // A discussion: its writes and commands refused.
    let ro = policy(&[], true).unwrap();
    assert!(
        ro.contains("write_file")
            && ro.contains("run_shell_command")
            && ro.contains("priority = 950")
    );
    let dir = crate::paths::test_dir("esc274-gemini-policy");
    let file = write_policy(&dir, Some(&rules)).unwrap().unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), rules);
    assert_eq!(write_policy(&dir, None).unwrap(), None);
    assert!(!file.exists());
}

#[test]
fn the_agents_settings_are_written_in_its_own_home_keeping_what_gemini_wrote() {
    let dir = crate::paths::test_dir("esc274-gemini-settings");
    let file = dir.join(".gemini").join("settings.json");
    write_settings(&dir, "gemini-api-key").unwrap();
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(doc["general"]["enableAutoUpdate"], false);
    assert_eq!(doc["privacy"]["usageStatisticsEnabled"], false);
    assert_eq!(doc["security"]["auth"]["selectedType"], "gemini-api-key");
    // What Gemini wrote (a setting of its own) is kept; how it signs in is its account's.
    std::fs::write(
        &file,
        r#"{"security":{"auth":{"selectedType":"oauth-personal"}},"ui":{"theme":"x"}}"#,
    )
    .unwrap();
    write_settings(&dir, "gemini-api-key").unwrap();
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(doc["security"]["auth"]["selectedType"], "gemini-api-key");
    assert_eq!(doc["ui"]["theme"], "x");
    assert_eq!(doc["general"]["enableAutoUpdateNotification"], false);
    // How it signs in, by the variables given.
    let env = |k: &str, v: &str| vec![(k.to_string(), v.to_string())];
    assert_eq!(
        auth_type(&env("GOOGLE_GENAI_USE_VERTEXAI", "true")),
        "vertex-ai"
    );
    assert_eq!(auth_type(&env("GEMINI_API_KEY", "k")), "gemini-api-key");
}

#[test]
fn escouades_server_goes_by_the_session_never_the_command_line() {
    let granted = GEMINI.escouade_launch(&Escouade::Granted {
        url: "http://127.0.0.1:1/mcp".into(),
        token: "s3cr3t".into(),
        file: None,
    });
    assert!(granted.args.is_empty() && granted.env.is_empty());
    assert!(GEMINI.grant_file("http://x", "s3cr3t").is_none());
    assert_eq!(GEMINI.agent_home_var(), Some("GEMINI_CLI_HOME"));
    // Neither `fs` nor `terminal`: Gemini reads and writes by itself.
    assert_eq!(hello()["clientCapabilities"]["terminal"], false);
    assert_eq!(
        plan_file("Requesting plan approval for: C:\\p\\plan.md"),
        Some("C:\\p\\plan.md")
    );
    assert!(is_plan_exit("exit_plan_mode__exit_plan_mode_1_0"));
}

#[test]
fn a_message_takes_its_images_and_text_files_and_refuses_a_pdf() {
    let file = |name: &str, media: &str, data: &str| Attachment {
        name: name.into(),
        media_type: media.into(),
        data: data.into(),
    };
    let m = message(
        "Regarde",
        &[
            file("a.png", "image/png", "iVBORw0KGgo="),
            file("notes.txt", "text/plain", "bonjour"),
        ],
    )
    .unwrap();
    assert_eq!(m["prompt"].as_array().unwrap().len(), 3);
    assert!(message("x", &[file("a.pdf", "application/pdf", "JVBERi0=")]).is_err());
}

#[test]
fn its_usage_limits_are_read_from_its_words() {
    // Its words, read in its code (0.66.0).
    assert_eq!(
        limit_of(None, "You have exhausted your daily quota on this model."),
        Some(DAILY_RESUME_MS)
    );
    assert_eq!(
        limit_of(Some(429), "Quota exceeded. Please retry in 30.5s."),
        Some(30_500)
    );
    assert_eq!(
        limit_of(Some(429), "Too Many Requests"),
        Some(RATE_RESUME_MS)
    );
    assert_eq!(
        limit_of(Some(503), "MODEL_CAPACITY_EXHAUSTED"),
        Some(RATE_RESUME_MS)
    );
    assert_eq!(limit_of(Some(500), "Internal error"), None);
    assert_eq!(limit_of(None, "Authentication required"), None);
}

#[test]
fn a_text_is_asked_of_gemini_p_without_any_tool_its_question_on_its_input() {
    let policy = Path::new("C:/x/escouade-policy.toml");
    let args = one_shot_args(Weight::Light, policy);
    assert_eq!(&args[..2], ["-p", "Answer the request above."]);
    let at = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .map(|i| args[i + 1].as_str())
    };
    assert_eq!(at("-o"), Some("json"));
    assert_eq!(at("-m"), Some("flash-lite"));
    assert_eq!(at("-e"), Some("none"));
    assert_eq!(at("--policy"), Some("C:/x/escouade-policy.toml"));
    assert_eq!(one_shot_args(Weight::Reading, policy)[5], "flash");
    // A text given: the model of a reading, under the same rules.
    let given = one_shot_args(Weight::Given, policy);
    assert_eq!(given[5], "flash");
    assert_eq!(
        given.last().map(String::as_str),
        Some("C:/x/escouade-policy.toml")
    );
    // Every tool refused, whatever its name.
    let rules: toml_edit::DocumentMut = NO_TOOLS.parse().unwrap();
    let rule = &rules["rule"].as_array_of_tables().unwrap().get(0).unwrap();
    assert_eq!(rule["toolName"].as_str(), Some("*"));
    assert_eq!(rule["decision"].as_str(), Some("deny"));
    // Its answer in JSON, a warning before it.
    let out =
        b"Warning: something\n{\n  \"session_id\": \"s\",\n  \"response\": \"  Titre court \"\n}";
    assert_eq!(response_of(out).unwrap(), "Titre court");
    let err = br#"{"error": {"type": "Error", "message": "You have exhausted your daily quota on this model."}}"#;
    assert!(response_of(err)
        .unwrap_err()
        .to_string()
        .contains("daily quota"));
    assert!(response_of(b"").is_err());
}

#[test]
fn an_agent_goes_by_its_accounts_sign_in_not_the_one_refreshed_in_its_home() {
    let home = crate::paths::test_dir("esc275-home");
    let file = sign_in_file(&home);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "{}").unwrap();
    forget_sign_in(&home);
    assert!(!file.exists());
    // Its settings say how its account signs in, whatever they said before.
    write_settings(&home, "gemini-api-key").unwrap();
    write_settings(&home, "oauth-personal").unwrap();
    let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(home.join(".gemini/settings.json")).unwrap())
            .unwrap();
    assert_eq!(doc["security"]["auth"]["selectedType"], "oauth-personal");
    // The address of its Google account, beside the sign-in.
    std::fs::write(
        home.join(".gemini/google_accounts.json"),
        r#"{"active": "moi@exemple.fr", "old": []}"#,
    )
    .unwrap();
    assert_eq!(signed_in_email(&home).as_deref(), Some("moi@exemple.fr"));
    assert!(caps().usage_page.is_some() && !caps().accounts_by_key);
}
