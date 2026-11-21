use super::*;
use serde_json::json;

#[test]
fn every_key_is_once_in_the_list_and_well_formed() {
    let mut seen = std::collections::BTreeSet::new();
    for d in POLICIES {
        assert!(seen.insert(d.key), "{} twice", d.key);
        assert!(crate::check_key(d.key).is_ok(), "{}", d.key);
        // The ADMX id is unique too (dots made underscores).
        assert_eq!(
            POLICIES.iter().filter(|o| o.id() == d.id()).count(),
            1,
            "{}",
            d.key
        );
    }
}

#[test]
fn the_list_covers_what_the_managed_device_imposes_in_6_4() {
    // docs/TEAM-SERVER.md §6.4, the rows whose « Poste géré » is « oui », and the enrollment of §6.3.
    for key in [
        "permissions.allowBypass",
        "permissions.modes",
        "permissions.defaultMode",
        "harnesses.allowed",
        "harnesses.allowedModels",
        "harnesses.maxEffort",
        "plugins.allowedOrigins",
        "plugins.allowed",
        "plugins.denied",
        "plugins.verifiedOnly",
        // The file of the plugins' secrets without a keychain (ESC-499, ESC-520).
        "secrets.fileFallback",
        "projects.forcePrivate",
        "projects.deniedFiles",
        "proxy.url",
        "proxy.exclusions",
        "proxy.denyInsecureTls",
        "proxy.authorities",
        "telemetry.enabled",
        "voice.onlineAllowed",
        "voice.dictation",
        "remoteControl.enabled",
        "updates.channel",
        "updates.automatic",
        "servers.allowed",
        "servers.personalSpace",
        "servers.aggregateView",
        "servers.contentlessNotifications",
        // The remote API of the core (ESC-478, docs/REMOTE.md).
        "servers.remoteApi",
        // The phone's tunnel (ESC-567).
        "servers.phone",
        "mcp.externalClients",
        "team.server",
        "team.fingerprint",
        "team.onlyThisServer",
        "extensions.allowed",
    ] {
        assert!(def(key).is_some(), "{key} missing from the list");
    }
    // The switches of the features (F-1, ESC-550): one per feature, imposed on or off.
    for key in [
        "features.kanban",
        "features.docs",
        "features.editor",
        "features.cost",
        "features.stats",
        "features.workflows",
        "features.scheduled",
        "features.overseer",
        "features.voice",
    ] {
        assert!(def(key).is_some(), "{key} missing from the list");
    }
    assert_eq!(POLICIES.len(), 43);
}

#[test]
fn each_key_says_its_type_from_its_form() {
    let kind = |k: &str| def(k).unwrap().kind();
    assert_eq!(kind(ALLOW_BYPASS), Type::Boolean);
    assert_eq!(kind(MODES_KEY), Type::List);
    assert_eq!(kind(DEFAULT_MODE), Type::String);
    assert_eq!(kind(MODELS), Type::Json);
    assert_eq!(kind(MAX_EFFORT), Type::String);
    assert_eq!(kind(DENIED_FILES), Type::List);
    assert_eq!(kind(PROXY_URL), Type::String);
    assert_eq!(kind(TEAM_FINGERPRINT), Type::String);
}

#[test]
fn a_key_that_restricts_says_the_one_value_it_may_impose_and_its_example_is_it() {
    for d in POLICIES {
        match d.form {
            // A feature's switch (F-1) is imposed either way: on or off.
            Form::Flag { restricted: None } if d.category == Category::Features => {
                assert_eq!(d.sort, Sort::Value, "{}", d.key);
            }
            Form::Flag { restricted } => {
                assert_eq!(d.sort, Sort::Restrict, "{}", d.key);
                let r = restricted.unwrap_or_else(|| panic!("{} says its restricted value", d.key));
                assert_eq!(d.example_value(), Value::Bool(r), "{}", d.key);
            }
            Form::Choice {
                restricted: Some(r),
                ..
            } => assert_eq!(d.example_value(), Value::String(r.into()), "{}", d.key),
            _ => {}
        }
    }
}

#[test]
fn every_example_is_of_its_type_and_converts_to_itself() {
    for d in POLICIES {
        let v = d.example_value();
        assert_eq!(convert(d, &v).as_ref(), Ok(&v), "{}", d.key);
    }
}

#[test]
fn the_sensitive_keys_are_the_proxy_and_the_authorities() {
    let sensitive: Vec<&str> = POLICIES
        .iter()
        .filter(|d| d.sort == Sort::Sensitive)
        .map(|d| d.key)
        .collect();
    assert_eq!(sensitive, [PROXY_URL, PROXY_EXCLUSIONS, AUTHORITIES]);
    // No key of the list is executable: a command is never a policy of this list (§6.5).
    assert!(POLICIES.iter().all(|d| d.sort != Sort::Executable));
}

#[test]
fn every_text_is_written_in_both_languages() {
    for d in POLICIES {
        for t in [&d.fr, &d.en] {
            assert!(
                !t.title.is_empty() && !t.help.is_empty() && !t.locks.is_empty(),
                "{}",
                d.key
            );
            // The texts go into XML and Markdown: no straight quote, no angle bracket.
            for s in [t.title, t.help, t.locks] {
                assert!(!s.contains(['"', '\'', '<', '>', '&']), "{}: {s}", d.key);
            }
        }
        assert_ne!(d.fr.title, d.en.title.to_lowercase(), "{}", d.key);
    }
}

#[test]
fn the_modes_are_the_cores() {
    // `modes::ORDER` of the app is this list (locks_policies_tests).
    assert_eq!(MODES.first(), Some(&"plan"));
    assert_eq!(MODES.last(), Some(&"bypassPermissions"));
}

// ---------- the conversion of the types ----------

#[test]
fn a_reg_dword_0_or_1_becomes_a_boolean_and_another_number_is_refused() {
    let d = def(TELEMETRY).unwrap();
    assert_eq!(convert(d, &json!(0)), Ok(json!(false)));
    assert_eq!(convert(d, &json!(1)), Ok(json!(true)));
    assert_eq!(convert(d, &json!(true)), Ok(json!(true)));
    assert_eq!(convert(d, &json!("false")), Ok(json!(false)));
    assert!(convert(d, &json!(2)).is_err());
    assert!(convert(d, &json!("off")).is_err());
    assert!(convert(d, &json!([0])).is_err());
}

#[test]
fn a_list_takes_one_string_as_a_list_of_one_drops_doubles_and_keeps_to_its_universe() {
    let d = def(MODES_KEY).unwrap();
    assert_eq!(convert(d, &json!("plan")), Ok(json!(["plan"])));
    assert_eq!(
        convert(d, &json!([" plan", "default", "plan"])),
        Ok(json!(["plan", "default"]))
    );
    assert!(convert(d, &json!(["plan", "yolo"]))
        .unwrap_err()
        .contains("yolo"));
    assert!(convert(d, &json!([1])).is_err());
    assert!(convert(d, &json!({"a": 1})).is_err());
    // A list with no universe takes any name.
    assert_eq!(
        convert(def(HARNESSES).unwrap(), &json!(["claude", "my-harness"])),
        Ok(json!(["claude", "my-harness"]))
    );
}

#[test]
fn a_string_keeps_to_its_choices_and_a_ceiling_too() {
    assert_eq!(
        convert(def(CHANNEL).unwrap(), &json!(" stable ")),
        Ok(json!("stable"))
    );
    assert!(convert(def(CHANNEL).unwrap(), &json!("nightly")).is_err());
    assert_eq!(
        convert(def(MAX_EFFORT).unwrap(), &json!("high")),
        Ok(json!("high"))
    );
    assert!(convert(def(MAX_EFFORT).unwrap(), &json!("ultra")).is_err());
    assert!(convert(def(PROXY_URL).unwrap(), &json!(3128)).is_err());
}

#[test]
fn a_json_setting_is_read_from_a_registry_string_and_must_be_an_object() {
    let d = def(MODELS).unwrap();
    // The registry reader already parses a REG_SZ that opens an object; a string that is JSON is parsed here too.
    assert_eq!(
        convert(d, &json!({"claude": ["a"]})),
        Ok(json!({"claude": ["a"]}))
    );
    assert_eq!(
        convert(d, &json!(r#"{"codex": []}"#)),
        Ok(json!({"codex": []}))
    );
    assert!(convert(d, &json!("[1, 2]")).is_err());
    assert!(convert(d, &json!("{broken")).unwrap_err().contains("JSON"));
}

#[test]
fn a_number_keeps_within_its_bounds() {
    let d = Def {
        key: "test.number",
        sort: Sort::Restrict,
        form: Form::Number { min: 1, max: 10 },
        rows: &[],
        within: None,
        category: Category::Projects,
        later: false,
        example: "3",
        fr: POLICIES[0].fr,
        en: POLICIES[0].en,
    };
    assert_eq!(d.kind(), Type::Number);
    assert_eq!(convert(&d, &json!(3)), Ok(json!(3)));
    assert_eq!(convert(&d, &json!("7")), Ok(json!(7)));
    assert!(convert(&d, &json!(0)).is_err());
    assert!(convert(&d, &json!(11)).is_err());
    assert!(convert(&d, &json!(true)).is_err());
}

#[test]
fn reading_brings_each_key_of_the_list_to_its_type_and_says_what_does_not_fit() {
    let policies = crate::Policies::from_values(
        [
            ("telemetry.enabled".to_owned(), json!(0)),
            ("permissions.modes".to_owned(), json!("plan")),
            ("updates.channel".to_owned(), json!("nightly")),
            ("some.future".to_owned(), json!(1)),
        ],
        crate::Source::Machine,
    );
    assert_eq!(
        policies.get("telemetry.enabled").unwrap().value,
        json!(false)
    );
    assert_eq!(
        policies.get("permissions.modes").unwrap().value,
        json!(["plan"])
    );
    assert!(policies.get("updates.channel").is_none());
    // Outside the list: kept as read, and said.
    assert_eq!(policies.get("some.future").unwrap().value, json!(1));
    let said: Vec<String> = policies.warnings().iter().map(|w| w.to_string()).collect();
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said.iter().any(|w| w.contains("some.future")));
    assert!(said.iter().any(|w| w.contains("updates.channel")));
}
