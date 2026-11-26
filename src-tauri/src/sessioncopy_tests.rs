//! The session of Claude Code passed to a teammate (TS-17b): its rewrite and its check on a session in the real format of
//! Claude Code 2.1.330 (`tests/fixtures/claude-session-copy.jsonl`, rebuilt from the fields of
//! `docs/TEAM-SESSION-COPY.md` §1 and anonymized: no account, organization or content of anyone), the version, the
//! folder it is placed in, and its removal.

use super::*;
use serde_json::{json, Value};

/// The session, as Claude Code wrote it in Alice's worktree.
const SESSION: &str = include_str!("../../tests/fixtures/claude-session-copy.jsonl");
const ALICE: &str = r"C:\Users\dev\repo\.claude\worktrees\dem-41";
const BOB: &str = r"D:\work\bob\wt-dem41";

fn lines(text: &str) -> Vec<Value> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// Every string of the session, keys too.
fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(m) => m.iter().for_each(|(k, x)| {
            out.push(k.clone());
            strings(x, out)
        }),
        _ => {}
    }
}

fn copy(lines: &str) -> SessionCopy {
    SessionCopy {
        id: "7d1f6a2e-0b3c-4e5d-8f90-1a2b3c4d5e6f".into(),
        worktree: ALICE.into(),
        version: "2.1.330".into(),
        lines: lines.into(),
    }
}

#[test]
fn the_folder_of_a_session_is_its_worktree_with_each_character_neither_letter_nor_digit_a_dash() {
    assert_eq!(encoded(r"C:\Users\a\wt"), "C--Users-a-wt");
    assert_eq!(
        encoded(r"C:\Users\dev\repo\.claude\worktrees\dem-41"),
        "C--Users-dev-repo--claude-worktrees-dem-41"
    );
    assert_eq!(encoded("/home/a/.wt"), "-home-a--wt");
    assert_eq!(encoded("C:/é/x"), "C----x");
}

#[test]
fn a_raw_replace_of_the_text_misses_every_path_its_json_doubles_the_backslash_of() {
    // Why it is rewritten string by string: in the file, `C:\Users\…` is `C:\\Users\\…`.
    assert!(!SESSION.contains(ALICE));
    assert!(SESSION.contains(&ALICE.replace('\\', r"\\")));
}

#[test]
fn the_worktree_of_who_passes_it_becomes_the_teammates_in_each_string_under_both_separators() {
    let (out, n) = rewrite(SESSION, ALICE, BOB).unwrap();
    let before = lines(SESSION);
    let after = lines(&out);
    assert_eq!(after.len(), before.len(), "no line lost or added");
    // Every form of the old worktree, as Claude Code writes it: with `\` (doubled in JSON), with `/`, and in a command of
    // Git Bash.
    // 16 with `\`, 3 with `/`, 1 of Git Bash (counted apart, by a walk of the fixture in Node).
    assert_eq!(n, 20, "the paths rewritten");
    let env = &after[2];
    assert_eq!(env["cwd"], BOB);
    let snap = &env["attachment"]["snapshot"];
    assert_eq!(snap["workingDirectory"], BOB);
    assert_eq!(
        snap["additionalWorkingDirectories"],
        json!(["D:/work/bob/wt-dem41/docs", "C:/Users/dev/notes"]),
        "the `/` form of the worktree, and a folder of the user out of it left as it is"
    );
    // The session's temporary folder is out of the worktree: Claude Code gives the resumed session its own.
    assert_eq!(
        snap["scratchpadDirectory"],
        before[2]["attachment"]["snapshot"]["scratchpadDirectory"]
    );
    let rendered = env["attachment"]["rendered"][0]["content"]
        .as_str()
        .unwrap();
    assert!(rendered.contains(r"Primary working directory: D:\work\bob\wt-dem41"));
    assert!(rendered.contains("D:/work/bob/wt-dem41/docs, C:/Users/dev/notes"));
    // The first message sent to the API (its environment too): the model never reads the old folder.
    assert!(after[5]["message"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains(r"Primary working directory: D:\work\bob\wt-dem41"));
    // The call of a tool, and its result.
    let read = r"D:\work\bob\wt-dem41\note.txt";
    assert_eq!(
        after[7]["message"]["content"][0]["input"]["file_path"],
        read
    );
    assert_eq!(
        after[7]["wireToolInputs"]["toolu_00000000000000000000001"]["file_path"],
        read
    );
    assert_eq!(after[8]["toolUseResult"]["file"]["filePath"], read);
    // A command of Git Bash: `/c/…` becomes `/d/…`; a folder beside the worktree (`dem-41-old`) is another path.
    let command = after[9]["message"]["content"][0]["input"]["command"]
        .as_str()
        .unwrap();
    assert_eq!(
        command,
        "cd /d/work/bob/wt-dem41 && ls ../dem-41-old && git -C D:/work/bob/wt-dem41 status --short"
    );
    assert_eq!(
        after[10]["toolUseResult"]["stdout"], before[10]["toolUseResult"]["stdout"],
        "`…\\dem-41-old\\reste.txt` is not of the worktree"
    );
    assert_eq!(
        after[13]["records"][0]["path"], read,
        "its own lines too (data-flow-records)"
    );
    // What is not a path stays: the content read, the accounts (the essay at two accounts says what they do).
    assert_eq!(
        after[8]["toolUseResult"]["file"]["content"],
        "MOT: girafe\n"
    );
    assert_eq!(after[0], before[0]);
    // A line without the worktree stays as Claude Code wrote it, byte for byte.
    assert_eq!(out.lines().next(), SESSION.lines().next());
    // Nothing of the old worktree is left, in any string or key, in any form.
    let mut all = Vec::new();
    after.iter().for_each(|v| strings(v, &mut all));
    let old = [
        r"C:\Users\dev\repo\.claude\worktrees\dem-41",
        "C:/Users/dev/repo/.claude/worktrees/dem-41",
        "/c/Users/dev/repo/.claude/worktrees/dem-41",
    ];
    for s in &all {
        let s = s.replace("dem-41-old", "");
        for o in old {
            assert!(!s.contains(o), "{o} is left in {s}");
        }
    }
    verify(&out, ALICE, BOB).unwrap();
}

#[test]
fn the_check_refuses_a_session_where_the_old_worktree_is_left_in_any_form() {
    let line =
        |s: &str| json!({ "type": "user", "cwd": BOB, "message": { "content": s } }).to_string();
    for left in [
        format!(r"{ALICE}\note.txt"),
        "C:/Users/dev/repo/.claude/worktrees/dem-41/note.txt".to_string(),
        "cd /c/Users/dev/repo/.claude/worktrees/dem-41".to_string(),
        // Windows does not care about case.
        r"c:\users\DEV\repo\.claude\worktrees\DEM-41".to_string(),
        ALICE.to_string(),
    ] {
        assert_eq!(
            verify(&line(&left), ALICE, BOB),
            Err(NotNative::PathLeft),
            "{left}"
        );
    }
    // In a key too.
    let key = json!({ "wireToolInputs": { ALICE: 1 } }).to_string();
    assert_eq!(verify(&key, ALICE, BOB), Err(NotNative::PathLeft));
    // Beside it, or the new worktree under the old one: nothing of the old is left.
    assert_eq!(
        verify(
            &line(r"C:\Users\dev\repo\.claude\worktrees\dem-41-old"),
            ALICE,
            BOB
        ),
        Ok(())
    );
    assert_eq!(
        verify(
            &line(&format!(r"{ALICE}\bob")),
            ALICE,
            &format!(r"{ALICE}\bob")
        ),
        Ok(())
    );
    // A line that is not JSON is never taken blind.
    assert_eq!(
        verify("pas du JSON", ALICE, BOB),
        Err(NotNative::Unreadable)
    );
    assert_eq!(
        rewrite("{\"cwd\":1}\npas du JSON\n", ALICE, BOB),
        Err(NotNative::Unreadable)
    );
}

#[test]
fn a_session_of_unix_goes_to_a_worktree_of_windows_and_the_same_folder_changes_nothing() {
    let s = [
        json!({ "type": "user", "cwd": "/home/alice/wt", "message": { "content": "lis /home/alice/wt/src/a.rs" } }),
        json!({ "type": "user", "cwd": "/home/alice/wt-2" }),
    ]
    .map(|v| v.to_string())
    .join("\n");
    let (out, n) = rewrite(&s, "/home/alice/wt/", r"C:\bob\wt").unwrap();
    assert_eq!(n, 2);
    let after = lines(&out);
    assert_eq!(after[0]["cwd"], r"C:\bob\wt");
    // The worktree in the form of its machine; what follows it, as it was (Windows takes both separators).
    assert_eq!(after[0]["message"]["content"], r"lis C:\bob\wt/src/a.rs");
    assert_eq!(after[1]["cwd"], "/home/alice/wt-2");
    // Unix cares about case: `/HOME/alice/wt` is another folder.
    let upper = json!({ "cwd": "/HOME/alice/wt" }).to_string();
    assert_eq!(
        rewrite(&upper, "/home/alice/wt", "/home/bob/wt").unwrap().1,
        0
    );
    // The same folder on both machines: the session as it is.
    assert_eq!(
        rewrite(SESSION, ALICE, ALICE).unwrap(),
        (SESSION.to_string(), 0)
    );
}

#[test]
fn the_version_of_the_session_is_the_newest_of_its_lines_and_this_machines_must_be_the_same_or_newer(
) {
    assert_eq!(session_version(SESSION).as_deref(), Some("2.1.330"));
    let mixed =
        "{\"version\":\"2.1.329\"}\n{\"version\":\"2.1.331\"}\n{\"version\":\"2.1.40\"}\n{}";
    assert_eq!(session_version(mixed).as_deref(), Some("2.1.331"));
    assert_eq!(session_version("{\"type\":\"user\"}"), None);
    assert_eq!(version_ok(Some("2.1.330 (Claude Code)"), "2.1.330"), Ok(()));
    assert_eq!(version_ok(Some("2.1.331 (Claude Code)"), "2.1.330"), Ok(()));
    assert_eq!(version_ok(Some("2.2.0"), "2.1.330"), Ok(()));
    assert_eq!(
        version_ok(Some("2.1.329 (Claude Code)"), "2.1.330"),
        Err(NotNative::Older {
            mine: "2.1.329".into(),
            theirs: "2.1.330".into()
        })
    );
    assert_eq!(version_ok(None, "2.1.330"), Err(NotNative::UnknownVersion));
    assert_eq!(
        version_ok(Some("2.1.330"), ""),
        Err(NotNative::UnknownVersion)
    );
}

#[test]
fn the_copy_goes_in_the_folder_of_the_worktree_in_the_accounts_folder_and_out_again() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("compte-b");
    let c = copy(SESSION);
    let path = place(&config, BOB, &c, Some("2.1.330 (Claude Code)")).unwrap();
    assert_eq!(
        path,
        config
            .join("projects")
            .join("D--work-bob-wt-dem41")
            .join("7d1f6a2e-0b3c-4e5d-8f90-1a2b3c4d5e6f.jsonl")
    );
    let placed = std::fs::read_to_string(&path).unwrap();
    assert_eq!(lines(&placed)[2]["cwd"], BOB);
    verify(&placed, ALICE, BOB).unwrap();
    // Found where Claude Code looks it up.
    assert_eq!(transcript_of(&config, BOB, &c.id), Some(path.clone()));
    // Never over a session already there.
    assert_eq!(
        place(&config, BOB, &c, Some("2.1.330")),
        Err(NotNative::Taken)
    );
    // Out again, its folder too when nothing else is in it.
    remove(&path);
    assert!(!path.exists());
    assert!(!path.parent().unwrap().exists());
    // Another session in the folder: it stays.
    let path = place(&config, BOB, &c, Some("2.1.330")).unwrap();
    let other = path.with_file_name("autre.jsonl");
    std::fs::write(&other, "{}\n").unwrap();
    remove(&path);
    assert!(!path.exists() && other.exists());
}

#[test]
fn nothing_is_placed_when_the_version_or_the_id_or_the_check_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().to_path_buf();
    let c = copy(SESSION);
    assert!(matches!(
        place(&config, BOB, &c, Some("2.1.300")),
        Err(NotNative::Older { .. })
    ));
    for id in ["", "../../evil", r"..\x", "a/b", "x.jsonl"] {
        let bad = SessionCopy {
            id: id.into(),
            ..c.clone()
        };
        assert_eq!(
            place(&config, BOB, &bad, Some("2.1.330")),
            Err(NotNative::BadId),
            "{id:?}"
        );
    }
    let broken = copy("{\"cwd\":\"x\"}\n{pas du JSON");
    assert_eq!(
        place(&config, BOB, &broken, Some("2.1.330")),
        Err(NotNative::Unreadable)
    );
    assert!(!config.join("projects").exists(), "nothing written");
    // Each says why, in the conversation.
    assert!(NotNative::Older {
        mine: "2.1.300".into(),
        theirs: "2.1.330".into()
    }
    .said()
    .contains("2.1.330"));
}

#[test]
fn the_session_of_who_passes_it_is_found_under_its_folder_else_anywhere_in_the_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path();
    let id = "7d1f6a2e-0b3c-4e5d-8f90-1a2b3c4d5e6f";
    assert_eq!(transcript_of(config, ALICE, id), None);
    // A worktree it went into: another folder of `projects`.
    let elsewhere = config
        .join("projects")
        .join("C--ailleurs")
        .join(format!("{id}.jsonl"));
    std::fs::create_dir_all(elsewhere.parent().unwrap()).unwrap();
    std::fs::write(&elsewhere, SESSION).unwrap();
    assert_eq!(transcript_of(config, ALICE, id), Some(elsewhere));
    assert_eq!(transcript_of(config, ALICE, "../x"), None);
}

/// The real trial (TS-17b), not run by the CI: a session of the real `claude` made in a throwaway folder A, copied by
/// Escouade's own functions (its version read, its paths rewritten and checked, placed under a throwaway folder B), the
/// original erased and A renamed as on another machine, then resumed from B by `--resume <id> --fork-session`. Run by
/// hand, out of any other use of the account: `ESCOUADE_REAL_SESSION_COPY=<path of claude>` (its account already signed
/// in; `ESCOUADE_REAL_CLAUDE_DIR`: its configuration folder, `~/.claude` by default), `cargo nextest run -p escouade
/// --lib real_session_copy --run-ignored only`. Two messages on Haiku (`--safe-mode`: no hook, MCP, CLAUDE.md nor
/// plugin of the user); only the two folders of `projects` it made are erased after, nothing else of the account is read.
#[test]
#[ignore]
fn real_session_copy() {
    let Ok(program) = std::env::var("ESCOUADE_REAL_SESSION_COPY") else {
        eprintln!("ESCOUADE_REAL_SESSION_COPY is not set: the trial does not run");
        return;
    };
    let config = std::env::var_os("ESCOUADE_REAL_CLAUDE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".claude"));
    let root = tempfile::tempdir().unwrap();
    let a = root.path().join("essai-a");
    let b = root.path().join("essai-b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(a.join("note.txt"), "MOT: girafe\n").unwrap();
    std::fs::write(b.join("note.txt"), "MOT: zebre\n").unwrap();
    let (a_s, b_s) = (
        a.to_string_lossy().to_string(),
        b.to_string_lossy().to_string(),
    );
    let claude = |cwd: &Path, extra: &[&str], message: &str| -> Value {
        let mut cmd = std::process::Command::new(&program);
        cmd.current_dir(cwd).args([
            "-p",
            "--safe-mode",
            "--model",
            "haiku",
            "--output-format",
            "json",
            "--allowedTools",
            "Read",
        ]);
        cmd.args(extra).args(["--", message]);
        // Not a child of the Claude Code session it may be run from.
        for (k, _) in std::env::vars() {
            if k.starts_with("CLAUDE_CODE_") || k == "CLAUDECODE" {
                cmd.env_remove(k);
            }
        }
        let out = cmd.output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        serde_json::from_str(text.trim())
            .unwrap_or_else(|_| panic!("{text}\n{}", String::from_utf8_lossy(&out.stderr)))
    };
    let cleanup = || {
        for dir in [&a_s, &b_s] {
            let folder = config.join("projects").join(encoded(dir));
            if folder.is_dir() && encoded(dir).contains("essai-") {
                let _ = std::fs::remove_dir_all(&folder);
            }
        }
    };
    let first = claude(
        &a,
        &[],
        "Lis note.txt avec l'outil Read et réponds seulement par le mot qu'il contient.",
    );
    let id = first["session_id"].as_str().unwrap().to_string();
    println!("A: {}", first["result"]);
    let original = session_path(&config, &a_s, &id);
    let lines = std::fs::read_to_string(&original).unwrap();
    let copy = SessionCopy {
        id: id.clone(),
        worktree: a_s.clone(),
        version: session_version(&lines).expect("its version"),
        lines,
    };
    let version = std::process::Command::new(&program)
        .arg("--version")
        .output()
        .unwrap();
    let mine = String::from_utf8_lossy(&version.stdout).to_string();
    let placed = place(&config, &b_s, &copy, Some(&mine));
    // As on another machine: neither the original nor A any more.
    let _ = std::fs::remove_file(&original);
    std::fs::rename(&a, root.path().join("ailleurs")).unwrap();
    let placed = match placed {
        Ok(p) => p,
        Err(e) => {
            cleanup();
            panic!("not placed: {e:?}");
        }
    };
    println!("placed: {}", placed.display());
    let second = claude(
        &b,
        &["--resume", &id, "--fork-session"],
        "Réponds en 3 lignes. 1) Sans outil : le mot lu au tour d'avant et le chemin absolu du fichier lu alors. 2) Relis note.txt avec Read : le mot et le chemin absolu lus maintenant. 3) Ton dossier de travail.",
    );
    println!("B: {second:#}");
    let forked = second["session_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let forked_file = session_path(&config, &b_s, &forked);
    let forked_lines = std::fs::read_to_string(&forked_file).unwrap_or_default();
    cleanup();
    assert_eq!(second["is_error"], false, "{second}");
    assert_ne!(forked, id, "--fork-session gives it a new id");
    let said = second["result"].as_str().unwrap_or_default().to_lowercase();
    assert!(said.contains("girafe"), "the turn before is known: {said}");
    assert!(said.contains("zebre"), "its tools read B: {said}");
    assert!(
        !forked_lines.contains(&a_s.replace('\\', r"\\")) && !forked_lines.contains(&a_s),
        "nothing of A in the session forked"
    );
}
