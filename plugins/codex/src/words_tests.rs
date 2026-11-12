//! What the core tested of Codex's words before they left it (ESC-359), on the plugin's own.

use crate::words::*;
use serde_json::json;

#[test]
fn a_discussion_is_read_only_in_every_mode_its_collaboration_its_own() {
    let of = |mode: &str, fence: Vec<String>| {
        mode_params(
            &Settings {
                mode: mode.into(),
                fence,
                read_only: true,
                ..Default::default()
            },
            "C:/p",
        )
    };
    for mode in ["default", "acceptEdits", "auto", "bypassPermissions"] {
        let p = of(mode, Vec::new());
        assert_eq!(p["sandboxPolicy"]["type"], "readOnly", "{mode}");
        assert_eq!(p["approvalPolicy"], "on-request", "{mode}");
        assert_eq!(p["approvalsReviewer"], "user", "{mode}");
        // Not made to plan: it answers.
        assert_eq!(p["collaborationMode"]["mode"], "default", "{mode}");
        // Fenced: the read only profile.
        let p = of(mode, vec!["C:/code/secret".into()]);
        assert_eq!(p["permissions"], FENCE_PLAN_PROFILE, "{mode}");
    }
    let s = Settings {
        mode: "default".into(),
        read_only: true,
        ..Default::default()
    };
    assert_eq!(sandbox_mode(s.sandboxed()), "read-only");
    assert_eq!(sandboxed("acceptEdits", false), "acceptEdits");
}

#[test]
fn a_fenced_agent_runs_in_escouades_profile_of_its_mode_with_the_paths_refused() {
    let of = |mode: &str| {
        mode_params(
            &Settings {
                mode: mode.into(),
                fence: vec!["C:/code/secret".into()],
                ..Default::default()
            },
            "C:/p",
        )
    };
    // In place of a sandbox policy: Codex takes one or the other.
    for (mode, profile) in [
        ("default", FENCE_PROFILE),
        ("acceptEdits", FENCE_PROFILE),
        ("auto", FENCE_PROFILE),
        ("plan", FENCE_PLAN_PROFILE),
    ] {
        let p = of(mode);
        assert_eq!(p["permissions"], profile, "{mode}");
        assert!(p.get("sandboxPolicy").is_none(), "{mode}");
    }
    // « Sans permissions » has no sandbox to refuse anything.
    let bypass = of("bypassPermissions");
    assert!(bypass.get("permissions").is_none());
    assert_eq!(bypass["sandboxPolicy"]["type"], "dangerFullAccess");
    // Declared for its process: the paths once, in the plan's, which the other extends.
    let args = fence_args(
        &[r"C:\code\secret".into(), "/home/me/x".into()],
        FENCE_PLAN_PROFILE,
    );
    assert_eq!(
        args,
        [
            "-c",
            "default_permissions=\"escouade-fence-plan\"",
            "-c",
            "permissions.escouade-fence-plan={extends=\":read-only\", filesystem={\"C:/code/secret\"=\"deny\", \"/home/me/x\"=\"deny\"}}",
            "-c",
            "permissions.escouade-fence={extends=\"escouade-fence-plan\", filesystem={\":workspace_roots\"=\"write\", \":tmpdir\"=\"write\"}}",
        ]
    );
}

#[test]
fn a_quota_window_is_labelled_by_its_length() {
    let m = meters(&json!({
        "primary": { "usedPercent": 2, "windowDurationMins": 43200, "resetsAt": 1794772340 },
        "secondary": null,
    }));
    assert_eq!(m.len(), 1);
    assert_eq!(m[0]["minutes"], 43_200);
    assert_eq!(m[0]["resetsAt"], 1_794_772_340_000_i64);
    assert_eq!(m[0]["pct"], 2.0);
}

#[test]
fn each_mode_of_escouade_has_its_policy_and_sandbox_in_codex() {
    let of = |mode: &str| {
        mode_params(
            &Settings {
                model: "gpt-6-luna".into(),
                effort: "low".into(),
                mode: mode.into(),
                ..Default::default()
            },
            "C:/p",
        )
    };
    let plan = of("plan");
    assert_eq!(plan["collaborationMode"]["mode"], "plan");
    assert_eq!(plan["sandboxPolicy"]["type"], "readOnly");
    assert_eq!(
        plan["collaborationMode"]["settings"]["reasoning_effort"],
        "low"
    );
    let ask = of("default");
    assert_eq!(
        (
            ask["approvalPolicy"].as_str(),
            ask["sandboxPolicy"]["type"].as_str()
        ),
        (Some("on-request"), Some("workspaceWrite"))
    );
    assert_eq!(ask["collaborationMode"]["mode"], "default");
    assert_eq!(ask["sandboxPolicy"]["networkAccess"], false);
    assert_eq!(of("acceptEdits")["sandboxPolicy"], ask["sandboxPolicy"]);
    assert_eq!(of("auto")["approvalsReviewer"], "auto_review");
    let bypass = of("bypassPermissions");
    assert_eq!(
        (
            bypass["approvalPolicy"].as_str(),
            bypass["sandboxPolicy"]["type"].as_str()
        ),
        (Some("never"), Some("dangerFullAccess"))
    );
    assert_eq!(sandbox_mode("plan"), "read-only");
    assert_eq!(sandbox_mode("acceptEdits"), "workspace-write");
    assert_eq!(sandbox_mode("bypassPermissions"), "danger-full-access");
    // And back, from what a thread's settings say.
    for mode in [
        "plan",
        "default",
        "acceptEdits",
        "auto",
        "bypassPermissions",
    ] {
        assert_eq!(mode_of(&of(mode), mode), Some(mode), "{mode}");
    }
    assert_eq!(mode_of(&of("default"), "plan"), Some("default"));
}

#[test]
fn the_models_and_their_efforts_come_from_model_list() {
    // As Codex 0.163.0 answered `model/list` (the core's recording `account.jsonl`, two of its models).
    let efforts =
        json!(["low", "medium", "high", "xhigh", "max"].map(|e| json!({ "reasoningEffort": e })));
    let list = json!({ "data": [
        { "id": "gpt-6-luna", "model": "gpt-6-luna", "displayName": "GPT-6-Luna", "isDefault": true,
          "inputModalities": ["text", "image"], "supportedReasoningEfforts": efforts, "defaultReasoningEffort": "medium" },
        { "id": "gpt-5-mini", "model": "gpt-5-mini", "displayName": "GPT-5-Mini", "isDefault": false,
          "inputModalities": ["text"], "supportedReasoningEfforts": [], "defaultReasoningEffort": null },
        { "id": "hidden", "model": "hidden", "hidden": true },
    ] });
    let models = catalog(&list).unwrap();
    let models = models.as_array().unwrap();
    let luna = models.iter().find(|m| m["id"] == "gpt-6-luna").unwrap();
    assert_eq!(luna["label"], "GPT-6-Luna");
    assert!(luna["default"] == true && luna["images"] == true);
    assert_eq!(
        luna["efforts"],
        json!(["low", "medium", "high", "xhigh", "max"])
    );
    assert_eq!(luna["defaultEffort"], "medium");
    assert_eq!(models.iter().filter(|m| m["default"] == true).count(), 1);
    assert!(!models.iter().any(|m| m["id"] == "hidden"));
    assert!(catalog(&json!({ "data": [] })).is_none());
}

#[test]
fn a_refusal_turns_off_only_an_entry_the_user_declares() {
    assert!(declares_escouade(
        "[mcp_servers.escouade]
url = \"http://x\"
"
    ));
    assert!(declares_escouade(
        "[mcp_servers.\"escouade\".env]
A = \"1\"
"
    ));
    assert!(declares_escouade(
        "[mcp_servers]
escouade = { url = \"http://x\" }
"
    ));
    assert!(!declares_escouade(
        "[mcp_servers.autre]
url = \"escouade\"
"
    ));
    assert!(!declares_escouade(
        "model = \"gpt-6-luna\"
[projects.'C:/escouade']
trust_level = \"trusted\"
"
    ));
}
