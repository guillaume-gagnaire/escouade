use super::*;
use crate::engine::CLAUDE;
use crate::i18n::Lang::{En, Fr};

#[test]
fn a_version_is_read_from_what_the_program_prints_and_compared_number_by_number() {
    assert_eq!(
        Version::find("2.1.301 (Claude Code)\n")
            .unwrap()
            .to_string(),
        "2.1.301"
    );
    assert_eq!(
        Version::find("codex-cli 0.163.0").unwrap().to_string(),
        "0.163.0"
    );
    assert_eq!(
        Version::find("OpenCode v1.19.2").unwrap().to_string(),
        "1.19.2"
    );
    assert_eq!(Version::find("no version here"), None);
    assert_eq!(Version::find("build 7"), None);
    // 2.1.301 is newer than 2.1.283 (not as text), and 2.1 is 2.1.0.
    assert!(Version::parse("2.1.301") > Version::parse("2.1.283"));
    assert!(Version::parse("2.10.0") > Version::parse("2.9.99"));
    assert_eq!(
        Version::parse("2.1").cmp(&Version::parse("2.1.0")),
        Ordering::Equal
    );
}

#[test]
fn a_version_is_placed_against_the_tested_range_bounds_included() {
    let t = Tested {
        oldest: "2.1.283",
        newest: "2.1.301",
    };
    assert_eq!(t.place(&Version::parse("2.1.282")), Range::Older);
    assert_eq!(t.place(&Version::parse("1.9.999")), Range::Older);
    assert_eq!(t.place(&Version::parse("2.1.283")), Range::Tested);
    assert_eq!(t.place(&Version::parse("2.1.290")), Range::Tested);
    assert_eq!(t.place(&Version::parse("2.1.301")), Range::Tested);
    assert_eq!(t.place(&Version::parse("2.1.302")), Range::Newer);
    assert_eq!(t.place(&Version::parse("3.0.0")), Range::Newer);
}

#[test]
fn the_state_of_a_harness_says_what_keeps_it_from_running_and_why() {
    let state =
        |found, version: Option<&str>, connected| state_of(Fr, &CLAUDE, found, version, connected);
    assert_eq!(state(false, None, false), (EngineState::Absent, None));
    assert_eq!(
        state(true, Some("2.1.301"), true),
        (EngineState::Ready, None)
    );
    // A version it does not tell is taken as it is.
    assert_eq!(state(true, None, true), (EngineState::Ready, None));
    assert_eq!(
        state(true, Some("2.1.290"), false),
        (EngineState::NotConnected, None)
    );
    // Too old: refused, signed in or not.
    let (s, note) = state(true, Some("2.0.14"), false);
    assert_eq!(s, EngineState::TooOld);
    assert_eq!(
        note.unwrap(),
        "Claude Code 2.0.14 est trop ancien pour Escouade, qui demande au moins la version 2.1.283 : son flux JSON n’a été vérifié qu’à partir de celle-ci (docs/PROTOCOL.md). Mets-le à jour (claude update)."
    );
}

/// Newer than the tested ones: ready, with no warning nor text of the tested versions (ESC-159).
#[test]
fn a_harness_newer_than_the_tested_ones_is_ready_without_a_warning() {
    for lang in [Fr, En] {
        assert_eq!(
            state_of(lang, &CLAUDE, true, Some("2.1.400 (Claude Code)"), true),
            (EngineState::Ready, None)
        );
        assert_eq!(
            state_of(lang, &CLAUDE, true, Some("3.0.0"), false),
            (EngineState::NotConnected, None)
        );
    }
}

#[test]
fn the_refusal_is_written_in_english_too() {
    assert_eq!(
        too_old(En, &CLAUDE, "2.0.14"),
        "Claude Code 2.0.14 is too old for Escouade, which needs version 2.1.283 or later: its JSON stream was only checked from that one on (docs/PROTOCOL.md). Update it (claude update)."
    );
}

#[test]
fn every_harness_declares_its_program_versions_and_capabilities() {
    let all = all();
    assert!(!all.is_empty());
    for e in all {
        assert!(!e.name().is_empty());
        let t = e.tested();
        let (oldest, newest) = (Version::find(t.oldest), Version::find(t.newest));
        assert!(oldest.is_some() && newest.is_some(), "{}", e.name());
        assert!(oldest <= newest, "{}", e.name());
        assert!(!e.version_args().is_empty());
        assert!(!e.update_command().is_empty());
        assert!(!e.oldest_reason(Fr).is_empty() && !e.oldest_reason(En).is_empty());
        assert!(!e.caps().modes.is_empty(), "{} takes no mode", e.name());
        // `by_id` gives it back from its id.
        assert_eq!(crate::engine::by_id(e.id()).id(), e.id());
    }
    assert_eq!(declared()[0].name, "Claude Code");
}

/// The declaration of Claude Code as the window's tests take it (`src/test/ipc.ts`): the file and
/// the declaration stay the same.
#[test]
fn claude_code_declares_what_it_does_today_as_the_window_tests_take_it() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/engines/claude.json");
    let kept: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let decl = serde_json::to_value(&declared()[0]).unwrap();
    assert_eq!(
        decl,
        kept,
        "{} differs from Claude Code's declaration: {}",
        file.display(),
        serde_json::to_string_pretty(&decl).unwrap()
    );
    let caps = CLAUDE.caps();
    let modes: Vec<&str> = caps.modes.iter().map(|m| m.id).collect();
    assert_eq!(
        modes,
        [
            "auto",
            "default",
            "plan",
            "acceptEdits",
            "bypassPermissions"
        ]
    );
    assert!(caps.effort && caps.images && caps.pdf && caps.slash_commands);
    assert_eq!(caps.fork, ForkPoint::Entry);
    assert_eq!(caps.mid_turn, MidTurn::Queued);
    assert_eq!(caps.quota, QuotaKind::Windows);
    assert_eq!(caps.cost, CostKind::Exact);
    assert!(caps.terminal_view && caps.remote_control && caps.background_tasks);
    assert!(caps.todo_tools && caps.default_refusal && caps.resume_other_account);
}

/// What the interface takes from the registry (ESC-29): how to install each harness, and the
/// models and efforts of its composer, each model saying what it takes as the harness does.
#[test]
fn every_harness_declares_its_installer_and_its_composers_models_and_efforts() {
    for (e, decl) in all().into_iter().zip(declared()) {
        assert!(!decl.install.windows.is_empty() && !decl.install.unix.is_empty());
        // OpenCode's are those of the providers connected to it, listed by its server only.
        assert!(
            !decl.models.is_empty() || e.id() == crate::model::EngineId::Opencode,
            "{} has no model",
            e.name()
        );
        for m in &decl.models {
            assert_eq!(m.effort, e.supports_effort(m.id), "{}", m.id);
            assert_eq!(m.auto, e.supports_auto(m.id), "{}", m.id);
        }
        assert_eq!(decl.efforts.is_empty(), !decl.caps.effort, "{}", e.name());
    }
    // Claude Code's, as the composer offered them before.
    let claude = &declared()[0];
    let models: Vec<(&str, &str, bool)> = claude
        .models
        .iter()
        .map(|m| (m.id, m.label, m.effort))
        .collect();
    assert_eq!(
        models,
        [
            ("fable", "Fable", true),
            ("opus", "Opus", true),
            ("sonnet", "Sonnet", true),
            ("haiku", "Haiku", false)
        ]
    );
    assert_eq!(claude.efforts, ["low", "medium", "high", "xhigh", "max"]);
    assert_eq!(
        claude.install.windows,
        "irm https://claude.ai/install.ps1 | iex"
    );
}

/// A program that prints `text` for its version, in a folder of its own.
fn printing(name: &str, text: &str) -> Program {
    let dir = std::env::temp_dir().join(format!("escouade-registry-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(windows)]
    let file = {
        let f = dir.join("claude.cmd");
        std::fs::write(&f, format!("@echo off\r\necho {text}\r\n")).unwrap();
        f
    };
    #[cfg(not(windows))]
    let file = {
        use std::os::unix::fs::PermissionsExt;
        let f = dir.join("claude");
        std::fs::write(&f, format!("#!/bin/sh\necho '{text}'\n")).unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
        f
    };
    Program::Native(file)
}

#[tokio::test]
async fn an_older_claude_code_is_refused_with_its_reason_and_a_newer_one_taken() {
    let old = printing("old", "2.0.14 (Claude Code)");
    assert_eq!(version(&CLAUDE, &old).await.as_deref(), Some("2.0.14"));
    let e = check(&CLAUDE, &old).await.unwrap_err().to_string();
    assert!(e.contains("2.0.14") && e.contains("2.1.283"), "{e}");
    assert!(e.contains("claude update"), "{e}");
    let newer = printing("newer", "9.0.0 (Claude Code)");
    check(&CLAUDE, &newer).await.unwrap();
    let tested = printing("tested", "2.1.301 (Claude Code)");
    check(&CLAUDE, &tested).await.unwrap();
    // One that says no version is taken.
    let silent = printing("silent", "hello");
    assert_eq!(version(&CLAUDE, &silent).await, None);
    check(&CLAUDE, &silent).await.unwrap();
}

/// The fields Escouade reads of each frame of Claude Code (`agent/claude.rs`): a key is the frame's
/// type, and its subtype for `system` and `result`.
const CLAUDE_READS: &[(&str, &[&str])] = &[
    ("system/init", &["type", "subtype", "session_id"]),
    (
        "system/task_started",
        &["subtype", "task_id", "is_backgrounded"],
    ),
    (
        "system/task_notification",
        &[
            "subtype",
            "task_id",
            "tool_use_id",
            "status",
            "summary",
            "uuid",
        ],
    ),
    ("system/bridge_state", &["subtype", "state"]),
    ("system/local_command_output", &["subtype", "content"]),
    ("stream_event", &["type", "event", "parent_tool_use_id"]),
    (
        "assistant",
        &["type", "message", "parent_tool_use_id", "uuid", "error"],
    ),
    (
        "user",
        &[
            "type",
            "message",
            "parent_tool_use_id",
            "uuid",
            "is_meta",
            "tool_use_result",
            "origin",
        ],
    ),
    (
        "result/success",
        &[
            "type",
            "subtype",
            "is_error",
            "duration_ms",
            "result",
            "modelUsage",
            "uuid",
        ],
    ),
    (
        "result/error_during_execution",
        &["subtype", "is_error", "result", "errors"],
    ),
    ("control_request", &["type", "request_id", "request"]),
    ("control_cancel_request", &["type", "request_id"]),
    ("rate_limit_event", &["type", "rate_limit_info"]),
];

fn claude_schema() -> serde_json::Value {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../tests/fixtures/engines/claude-code-{}.schema.json",
        CLAUDE.tested().newest
    ));
    serde_json::from_str(&std::fs::read_to_string(&file).unwrap_or_else(|e| {
        panic!(
            "{}: the schema of the newest tested version is kept in the repository ({e})",
            file.display()
        )
    }))
    .unwrap()
}

#[test]
fn claude_code_frames_read_by_escouade_are_in_the_schema() {
    let schema = claude_schema();
    assert_eq!(schema["version"], CLAUDE.tested().newest);
    let mut missing = Vec::new();
    for (kind, fields) in CLAUDE_READS {
        let Some(known) = schema["frames"][kind].as_array() else {
            missing.push(kind.to_string());
            continue;
        };
        for field in *fields {
            if !known.iter().any(|k| k == field) {
                missing.push(format!("{kind}.{field}"));
            }
        }
    }
    assert!(missing.is_empty(), "not in the schema: {missing:?}");
}

/// The schema says what the real runs recorded show: every field of each of their frames is there.
#[test]
fn the_schema_of_claude_code_holds_every_field_of_its_recorded_runs() {
    let schema = claude_schema();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures");
    let mut runs = 0;
    let mut missing = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !(name.starts_with("real-run-") && name.ends_with(".jsonl")) {
            continue;
        }
        runs += 1;
        for line in std::fs::read_to_string(&path).unwrap().lines() {
            let Ok(serde_json::Value::Object(frame)) = serde_json::from_str(line) else {
                continue;
            };
            let kind = match frame["type"].as_str().unwrap_or("") {
                t @ ("system" | "result") => {
                    format!("{t}/{}", frame["subtype"].as_str().unwrap_or(""))
                }
                t => t.to_string(),
            };
            let known = schema["frames"][&kind]
                .as_array()
                .cloned()
                .unwrap_or_default();
            for field in frame.keys() {
                if !known.iter().any(|k| k == field) {
                    missing.insert(format!("{name}: {kind}.{field}"));
                }
            }
        }
    }
    assert!(runs >= 5, "the recorded runs are in tests/fixtures");
    assert!(missing.is_empty(), "not in the schema: {missing:?}");
}
