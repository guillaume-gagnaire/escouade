//! The changes at many places of the real agents' bench (ESC-306, IDX-6): what a change must leave
//! in the code (each place it touches, no old name left, the checks of the project passing) and
//! what an agent had missed when it first checked its work. `docs/CODEINDEX-PROBES.md`
//! « Changements à plusieurs endroits (IDX-6) » says what it found.

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

/// A change at many places: its places, its old names, its checks, its good answer.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Multi {
    /// Each place the change must reach: a text found at least so many times in a file.
    pub places: Vec<Place>,
    /// The old names: none may be left in their files.
    #[serde(default)]
    pub gone: Vec<Gone>,
    /// The homonyms the change must not touch: each text still so many times in its file.
    #[serde(default)]
    pub kept: Vec<Place>,
    /// The checks of the project that must pass, run from the copy's root.
    pub commands: Vec<String>,
    /// The good answer, a patch of `docs/codeindex-agents-multi/`; the bad one is the same with
    /// this file left out.
    pub patch: String,
    pub bad_skip: String,
}

#[derive(Debug, Deserialize)]
pub struct Place {
    pub file: String,
    pub text: String,
    pub count: u32,
}

#[derive(Debug, Deserialize)]
pub struct Gone {
    pub text: String,
    /// The files it may not be left in, by their path from the root.
    pub files: Vec<String>,
}

impl Multi {
    /// The places to reach: each text so many times, each old name of each file once at least.
    pub fn expected(&self) -> u32 {
        self.places.iter().map(|p| p.count).sum()
    }
}

/// What a copy still misses of a change: the places not reached yet (what a text lacks of its
/// count) and the old names left; and the homonyms it touched (`wrong`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Missed {
    pub places: u32,
    pub left: u32,
    pub wrong: u32,
    pub detail: Vec<String>,
}

impl Missed {
    pub fn total(&self) -> u32 {
        self.places + self.left
    }
}

fn count(text: &str, needle: &str) -> u32 {
    text.matches(needle).count() as u32
}

/// A file of the copy as text, its line ends as git writes them (`\r\n` taken as `\n`).
fn text_of(copy: &Path, file: &str) -> String {
    std::fs::read_to_string(copy.join(file))
        .unwrap_or_default()
        .replace("\r\n", "\n")
}

pub fn missed(m: &Multi, copy: &Path) -> Missed {
    let mut out = Missed::default();
    for p in &m.places {
        let found = count(&text_of(copy, &p.file), &p.text);
        if found < p.count {
            out.places += p.count - found;
            out.detail
                .push(format!("{}: « {} » {found}/{}", p.file, p.text, p.count));
        }
    }
    for g in &m.gone {
        for f in &g.files {
            let left = count(&text_of(copy, f), &g.text);
            if left > 0 {
                out.left += left;
                out.detail.push(format!("{f}: « {} » left {left}×", g.text));
            }
        }
    }
    for k in &m.kept {
        let found = count(&text_of(copy, &k.file), &k.text);
        if found < k.count {
            out.wrong += k.count - found;
            out.detail.push(format!(
                "{}: homonym « {} » touched, {found}/{} left",
                k.file, k.text, k.count
            ));
        }
    }
    out
}

/// A command line of the checks, in the shell of the machine.
pub fn shell(line: &str) -> Command {
    if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(line);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(line);
        c
    }
}

/// The end of a command's output, for why it failed.
fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    all[all.len().saturating_sub(lines)..].join(" ⏎ ")
}

/// Runs the checks of a change in `copy` (its cargo target in `target`): right, or why not.
pub fn verify(m: &Multi, copy: &Path, target: &Path) -> Result<(), String> {
    let miss = missed(m, copy);
    if miss.total() + miss.wrong > 0 {
        return Err(format!(
            "{} place(s) missed, {} homonym(s) touched: {}",
            miss.total(),
            miss.wrong,
            miss.detail.join("; ")
        ));
    }
    // One lane checks at a time: tests that start processes fail when six lanes build at once.
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    for line in &m.commands {
        let run = || {
            shell(line)
                .current_dir(copy)
                .env("CARGO_TARGET_DIR", target)
                .stdin(Stdio::null())
                .output()
                .map_err(|e| format!("{line}: {e}"))
        };
        let mut out = run()?;
        // A second chance, for a test that failed by the load of the machine.
        if !out.status.success() {
            out = run()?;
        }
        if !out.status.success() {
            let text = format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            return Err(format!("`{line}` failed: {}", tail(&text, 6)));
        }
    }
    Ok(())
}

/// Applies a patch to `copy` (`git apply`, which wants no repository), but `skip` when given.
pub fn apply(copy: &Path, patch: &Path, skip: Option<&str>) -> Result<()> {
    let mut c = Command::new("git");
    c.current_dir(copy)
        .args(["-c", "core.autocrlf=false", "apply", "--whitespace=nowarn"]);
    if let Some(s) = skip {
        c.arg(format!("--exclude={s}"));
    }
    let out = c.arg(patch).output().context("git apply")?;
    if !out.status.success() {
        bail!(
            "git apply {}: {}",
            patch.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Whether a command of the agent's terminal is one of the checks: it begins with one of
/// `terminal`, alone (no second command, pipe, redirection or substitution).
pub fn allowed(command: &str, terminal: &[String]) -> bool {
    let c = command.trim();
    let alone = !["&", "|", ";", ">", "<", "`", "$(", "\n"]
        .iter()
        .any(|s| c.contains(s));
    alone
        && terminal
            .iter()
            .any(|t| c == t || c.starts_with(&format!("{t} ")))
}

/// The hook before each command of the agent's terminal: a command that is no check is refused
/// (Claude Code lets the read-only ones through, `ls` or `grep`, whatever its rules say); before a
/// check, what the copy still misses then is written down. The refusal, or None.
pub fn snap(
    m: &Multi,
    copy: &Path,
    input: &str,
    log: &Path,
    terminal: &[String],
) -> Result<Option<Value>> {
    let event: Value = serde_json::from_str(input).unwrap_or_default();
    let command = event["tool_input"]["command"].as_str().unwrap_or_default();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)?;
    if !allowed(command, terminal) {
        writeln!(f, "{}", json!({ "command": command, "refused": true }))?;
        let why = format!(
            "Ce terminal ne sert qu'aux vérifications, une commande seule depuis la racine : {}.",
            terminal.join(", ")
        );
        return Ok(Some(
            json!({ "hookSpecificOutput": { "hookEventName": "PreToolUse",
            "permissionDecision": "deny", "permissionDecisionReason": why } }),
        ));
    }
    let miss = missed(m, copy);
    writeln!(
        f,
        "{}",
        json!({ "command": command, "places": miss.places, "left": miss.left,
            "wrong": miss.wrong, "detail": miss.detail })
    )?;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_terminal_runs_the_checks_alone() {
        let terminal = vec!["cargo check".to_string(), "npm run check".to_string()];
        assert!(allowed(
            " cargo check --manifest-path src-tauri/Cargo.toml",
            &terminal
        ));
        assert!(allowed("npm run check", &terminal));
        for refused in [
            "ls",
            "grep -rn forget src",
            "cargo checkout",
            "cargo check && rg forget",
            "cargo check | tail",
            "cargo check; ls",
            "cargo check > out.txt",
            "cargo check $(ls)",
        ] {
            assert!(!allowed(refused, &terminal), "{refused}");
        }
        let dir = crate::paths::test_dir("multi-snap");
        let log = dir.join("snap.jsonl");
        let m = Multi {
            places: Vec::new(),
            gone: Vec::new(),
            kept: Vec::new(),
            commands: Vec::new(),
            patch: String::new(),
            bad_skip: String::new(),
        };
        let event = |c: &str| json!({ "tool_input": { "command": c } }).to_string();
        let reply = snap(&m, &dir, &event("ls"), &log, &terminal).unwrap();
        assert_eq!(
            reply.unwrap()["hookSpecificOutput"]["permissionDecision"],
            "deny"
        );
        assert!(snap(&m, &dir, &event("npm run check"), &log, &terminal)
            .unwrap()
            .is_none());
        let lines: Vec<Value> = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines[0]["refused"], true);
        assert_eq!(lines[1]["places"], 0);
    }

    #[test]
    fn what_a_copy_misses_is_each_place_short_of_its_count_and_each_old_name_left() {
        let dir = crate::paths::test_dir("multi-missed");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src/a.rs"),
            "fn unload() {}\r\nfn x() { unload(); forget(); }\r\n",
        )
        .unwrap();
        std::fs::write(dir.join("src/b.rs"), "fn y() { forget(); }\n").unwrap();
        let m = Multi {
            places: vec![
                Place {
                    file: "src/a.rs".into(),
                    text: "unload(".into(),
                    count: 3,
                },
                Place {
                    file: "src/b.rs".into(),
                    text: "unload(".into(),
                    count: 1,
                },
            ],
            gone: vec![Gone {
                text: "forget(".into(),
                files: vec!["src/a.rs".into(), "src/b.rs".into(), "src/none.rs".into()],
            }],
            kept: vec![Place {
                file: "src/b.rs".into(),
                text: "fn y(".into(),
                count: 1,
            }],
            commands: Vec::new(),
            patch: String::new(),
            bad_skip: String::new(),
        };
        assert_eq!(m.expected(), 4);
        let miss = missed(&m, &dir);
        assert_eq!((miss.places, miss.left, miss.total()), (2, 2, 4));
        assert_eq!(miss.detail.len(), 4, "{:?}", miss.detail);
        std::fs::write(
            dir.join("src/a.rs"),
            "fn unload() {}\nfn x() { unload(); unload(); }\n",
        )
        .unwrap();
        std::fs::write(dir.join("src/b.rs"), "fn y() { unload(); }\n").unwrap();
        assert_eq!(missed(&m, &dir), Missed::default());
        // A homonym renamed with it.
        std::fs::write(dir.join("src/b.rs"), "fn z() { unload(); }\n").unwrap();
        assert_eq!(missed(&m, &dir).wrong, 1);
        assert!(verify(&m, &dir, &dir.join("target")).is_err());
        std::fs::write(dir.join("src/b.rs"), "fn y() { unload(); }\n").unwrap();
        assert!(verify(&m, &dir, &dir.join("target")).is_ok());
        let failing = Multi {
            commands: vec!["exit 3".into()],
            ..m
        };
        assert!(verify(&failing, &dir, &dir.join("target"))
            .unwrap_err()
            .contains("exit 3"));
    }
}
