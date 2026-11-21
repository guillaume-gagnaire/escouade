//! Read with the code of the 2.2 (`origin/main`) what the 2.3 wrote (ESC-586): `scripts/compat-2.2.sh` copies this
//! file into a copy of `origin/main` as `src-tauri/src/compat23_read.rs`, declares it in its `lib.rs`, and runs it with
//! `ESCOUADE_COMPAT23_IN`, the folder `src-tauri/src/compat22_tests.rs` (on the 2.3) wrote. It is never compiled on the
//! 2.3: it names the 2.2's own functions.
//!
//! What it checks, as the 2.2 does at its start and when it opens the project: no file set aside (`recover_all`
//! would put an unreadable state or settings aside, and the app start empty), each file read without an error, and
//! what the 2.3 wrote found in it (the projects, the agents, the tickets, the reviews, the requests, the settings, the
//! project's file, a shared ticket, the spec, the decisions, the memory). What the 2.2 knows and the 2.3 did not write
//! (a field the 2.3 has not got yet) is told as a warning: it would be lost on the way back.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn input() -> Option<PathBuf> {
    std::env::var_os("ESCOUADE_COMPAT23_IN").map(PathBuf::from)
}

fn ids(v: &Value, key: &str) -> Vec<String> {
    v[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| x.to_string()))
        .collect()
}

/// The paths of the fields the 2.2 writes back (`theirs`) that the 2.3's file (`ours`) does not have.
fn missing(ours: &Value, theirs: &Value, at: &str, out: &mut Vec<String>) {
    match (ours, theirs) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                let path = format!("{at}.{k}");
                match a.get(k) {
                    Some(w) => missing(w, v, &path, out),
                    None => out.push(path),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            for (i, (w, v)) in a.iter().zip(b).enumerate() {
                missing(w, v, &format!("{at}[{i}]"), out);
            }
        }
        _ => {}
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &target);
        } else {
            std::fs::copy(e.path(), target).unwrap();
        }
    }
}

#[test]
fn the_2_2_reads_what_the_2_3_wrote() {
    let Some(dir) = input() else {
        eprintln!("ESCOUADE_COMPAT23_IN is not set: nothing to read");
        return;
    };
    let expect: Value = serde_json::from_slice(&std::fs::read(dir.join("expect.json")).unwrap()).unwrap();
    let mut warnings = Vec::new();

    // The start: what `Core::load` does first. A copy, so the files stay as the 2.3 wrote them.
    let data = std::env::temp_dir().join(format!("compat23-data-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    copy_dir(&dir.join("data"), &data);
    let recovered = escouade_model::backups::recover_all(&data, chrono::Utc::now());
    assert!(
        recovered.is_empty(),
        "the 2.2 sets aside what the 2.3 wrote, and starts without it: {recovered:?}"
    );
    for file in ["state.json", "settings.json"] {
        assert!(data.join(file).is_file(), "{file} is gone after the start");
    }

    // The state.
    let text = std::fs::read(data.join("state.json")).unwrap();
    let state: escouade_model::model::PersistedState =
        serde_json::from_slice(&text).unwrap_or_else(|e| panic!("the 2.2 does not read state.json: {e}"));
    let got: Vec<String> = state.projects.iter().map(|p| p.id.clone()).collect();
    assert_eq!(got, ids(&expect, "projects"), "the projects");
    let got: Vec<String> = state.agents.iter().map(|a| a.id.clone()).collect();
    assert_eq!(got, ids(&expect, "agents"), "the agents");
    let got: Vec<String> = state.tickets.iter().map(|t| t.key.clone()).collect();
    assert_eq!(got, ids(&expect, "tickets"), "the tickets");
    let got: Vec<String> = state.pr_reviews.iter().flat_map(|r| r.notes.iter().map(|n| n.id.to_string())).collect();
    assert_eq!(got, ids(&expect, "reviewNotes"), "the remarks of the reviews");
    let got: Vec<String> = state.asks.iter().map(|a| a.id.clone()).collect();
    assert_eq!(got, ids(&expect, "asks"), "the requests of « À toi »");
    let ours: Value = serde_json::from_slice(&text).unwrap();
    missing(&ours, &serde_json::to_value(&state).unwrap(), "state", &mut warnings);

    // The settings.
    let text = std::fs::read(data.join("settings.json")).unwrap();
    let settings: escouade_model::model::Settings =
        serde_json::from_slice(&text).unwrap_or_else(|e| panic!("the 2.2 does not read settings.json: {e}"));
    let back = serde_json::to_value(&settings).unwrap();
    for (k, v) in expect["settings"].as_object().unwrap() {
        assert_eq!(&back[k], v, "the setting {k}");
    }
    missing(&serde_json::from_slice(&text).unwrap(), &back, "settings", &mut warnings);

    // The project's file: read, its known keys taken (an unknown one is a warning of the file, never an error).
    let repo = dir.join("repo");
    let yml = std::fs::read_to_string(repo.join(crate::projectfile::FILE)).unwrap();
    let read = crate::projectfile::read(&yml).unwrap_or_else(|w| panic!("the 2.2 does not read .escouade.yml: {w:?}"));
    for (k, v) in expect["projectFile"].as_object().unwrap() {
        assert_eq!(read.values.get(k), Some(v), "the key {k} of .escouade.yml");
    }
    for w in &read.warnings {
        warnings.push(format!(".escouade.yml: {w:?}"));
    }

    // A shared ticket, as the 2.3 writes it and frozen.
    for file in [
        repo.join(".escouade/tickets/q4w7c2na.yml"),
        dir.join("frozen/.escouade/tickets/q4w7c2na.yml"),
    ] {
        let text = std::fs::read_to_string(&file).unwrap();
        match crate::sharing::read(&text) {
            crate::sharing::Read::Ok(doc, _) => {
                assert_eq!(doc.key, expect["sharedTicket"]["key"].as_str().unwrap(), "{}", file.display());
                assert_eq!(doc.title, expect["sharedTicket"]["title"].as_str().unwrap(), "{}", file.display());
            }
            other => panic!("the 2.2 does not read {}: {other:?}", file.display()),
        }
    }

    // The spec and the decisions.
    let space = crate::docs::Space::new(&repo, None);
    let page = space
        .spec_read(expect["page"]["path"].as_str().unwrap())
        .unwrap_or_else(|e| panic!("the 2.2 does not read the page of the spec: {e}"));
    assert_eq!(page.title, expect["page"]["title"].as_str().unwrap());
    let titles: BTreeSet<String> = space.decisions().into_iter().map(|d| d.title).collect();
    for t in ids(&expect, "decisions") {
        assert!(titles.contains(&t), "the decision « {t} » is not read: {titles:?}");
    }

    // The memory.
    let memories = crate::memory::list(&repo.join(crate::memory::DIR)).unwrap();
    assert!(
        memories.iter().any(|m| m.name == expect["memory"].as_str().unwrap()),
        "the memory is not read"
    );

    let _ = std::fs::remove_dir_all(&data);
    for w in &warnings {
        println!("warning: {w}");
    }
    println!("the 2.2 reads what the 2.3 wrote ({} warnings)", warnings.len());
}
