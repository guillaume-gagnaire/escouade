//! The way back from the 2.3 to the 2.2 (ESC-586): what the 2.3 writes, written here by its own code, with what the 2.2
//! does not know (the team server, the scheduled tasks, the onboarding, the features, the language servers, a remark
//! published one by one…), so that `scripts/compat-2.2.sh` reads it with the code of `origin/main`
//! (`tests/compat-2.2/read_on_main.rs`).
//!
//! `ESCOUADE_COMPAT22_OUT=<folder>`: the files go there, beside `expect.json`, what the 2.2 must find in them; without
//! it, in a folder of the test's, and the 2.3 reads them back itself.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::docs::{Decision, Page, Register, Space};
use crate::model::{PersistedState, Settings};

const PROJECT: &str = "p-compat";
const TEAM: &str = "0123456789abcdef0123456789abcdef";

/// The data folder and the repository, under `out`.
fn folders(out: &Path) -> (PathBuf, PathBuf) {
    (out.join("data"), out.join("repo"))
}

/// The state as the 2.3 saves it, every part the 2.2 does not know filled.
fn state(repo: &Path) -> PersistedState {
    let path = repo.to_string_lossy();
    serde_json::from_value(json!({
        "projects": [
            { "id": PROJECT, "name": "compat", "path": path, "color": "", "ticketPrefix": "CMP",
              "team": TEAM, "teamProject": "fedcba9876543210fedcba9876543210", "memoryReviewOff": true },
            { "id": "p-plain", "name": "plain", "path": format!("{path}-plain"), "color": "" }
        ],
        "agents": [
            { "id": "a-1", "projectId": PROJECT, "name": "Agent un", "engine": "claude", "createdAt": 1_763_700_000_000i64 },
            { "id": "a-2", "projectId": "p-plain", "name": "Agent deux", "engine": "codex", "createdAt": 1_763_700_000_001i64 }
        ],
        "tickets": [
            { "id": "t-1", "projectId": PROJECT, "key": "CMP-1", "title": "Un ticket de l'équipe", "column": "doing",
              "target": "release/2.3", "otherLine": "main", "formerKeys": ["OLD-1"], "scheduled": "s-1",
              "team": { "object": "obj-1", "claim": "claim-1", "keyed": true, "version": 3, "token": 7,
                        "holder": { "name": "Léa", "device": "dev-1", "mine": false, "since": 1, "until": 2 },
                        "conflicts": [{ "field": "title", "by": "Léa", "stamp": "s", "mine": "a", "theirs": "b" }],
                        "handoff": { "object": "obj-1", "fromName": "Léa", "toName": "Moi", "incoming": true, "at": 3 } },
              "teamLocal": true },
            { "id": "t-2", "projectId": "p-plain", "key": "PLN-1", "title": "Un ticket simple", "column": "todo" }
        ],
        "prReviews": [
            { "projectId": PROJECT, "number": 12, "title": "Une PR relue", "phase": "ready",
              "notes": [
                { "id": 1, "path": "src/a.rs", "line": 3, "body": "Publiée seule", "choice": "published", "severity": "bug",
                  "publishedAt": 1_763_700_000_100i64 },
                { "id": 2, "path": "src/b.rs", "line": 4, "body": "Gardée", "choice": "kept" }
              ] }
        ],
        "pendingReviews": [{ "projectId": PROJECT, "number": 12 }],
        "scheduled": [
            { "id": "s-1", "projectId": PROJECT, "title": "Chaque lundi", "rhythm": { "kind": "weekly", "days": [1], "hour": 9 } }
        ],
        "onboarding": { "version": 1, "started": 1, "finished": 2,
                        "steps": { "harness": "done", "telemetry": "skipped" },
                        "hookups": { "voice": "skipped" }, "discover": { "team": "dismissed" }, "tipped": ["remote"],
                        "seenRelease": "2.3.0-beta.1" },
        "asks": [
            { "id": "a-1", "projectId": PROJECT, "title": "Choisir une direction", "kind": "choice" }
        ],
        "askSeq": 1,
        "inboxSnoozes": { "view-1": 1_763_800_000_000i64 }
    }))
    .expect("the 2.3's state")
}

/// The settings as the 2.3 saves them, its new ones set.
fn settings() -> Settings {
    let mut s: Settings = serde_json::from_value(json!({
        "language": "en",
        "betaUpdates": true,
        "overseer": { "enabled": true, "engine": "claude", "model": "haiku" },
        "languageServers": { "rust-analyzer": { "enabled": true, "path": "/usr/bin/rust-analyzer" } },
        "languageServersTrusted": ["rust-analyzer"],
        "extensionsAllowed": { "*": true },
        "featuresUsed": [],
    }))
    .expect("the 2.3's settings");
    s.features = Some(Default::default());
    s
}

fn write(path: &Path, text: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Writes everything in `out` as the 2.3 does, and what the 2.2 must find in it (`expect.json`).
fn write_all(out: &Path) {
    let (data, repo) = folders(out);
    let state = state(&repo);
    write(
        &data.join("state.json"),
        &serde_json::to_vec_pretty(&state).unwrap(),
    );
    write(
        &data.join("settings.json"),
        &serde_json::to_vec_pretty(&settings()).unwrap(),
    );

    // .escouade.yml: the project's settings, with the team's keys and those of the language servers.
    let project = &state.projects[0];
    let mut values = crate::projectfile::export_values(project);
    values.push(("team.server", json!("https://team.example.org")));
    values.push(("team.id", json!(TEAM)));
    values.push(("team.name", json!("L'équipe")));
    values.push(("memory.review", json!(false)));
    let mut yml = crate::projectfile::write(None, &values).expect("the 2.3 writes .escouade.yml");
    // Keys a person writes by hand in it (the app never does): the extensions of the editor.
    yml.push_str("editor:\n  extensions_allowed:\n    \"*\": true\n");
    write(&repo.join(crate::projectfile::FILE), yml.as_bytes());

    // The shared tickets: one as the 2.3 writes it, one frozen when the project joined its team (TS-7).
    let me = crate::model::Person {
        name: "Moi".into(),
        id: "a91f03c2".into(),
    };
    let tickets_dir = repo.join(crate::sharing::DIR);
    let mut shared = state.tickets[1].clone();
    shared.shared = Some(crate::model::Shared {
        id: "q4w7c2na".into(),
        key: shared.key.clone(),
        author: me.clone(),
        mine: true,
        ..Default::default()
    });
    let doc = crate::sharing::file_of(&shared, &state.tickets, &me, "2026-11-21T10:00:00+01:00");
    write(
        &tickets_dir.join("q4w7c2na.yml"),
        crate::sharing::write(&doc).as_bytes(),
    );
    let frozen_dir = out.join("frozen").join(crate::sharing::DIR);
    write(
        &frozen_dir.join("q4w7c2na.yml"),
        crate::sharing::write(&doc).as_bytes(),
    );
    assert_eq!(crate::sharing::freeze(&frozen_dir), vec!["q4w7c2na.yml"]);

    // The spec, and decisions: a proposal, one accepted with its number, one accepted on a team server without its
    // number yet (TS-9).
    let space = Space::new(&repo, None);
    space
        .spec_write("introduction/livraison", |p: &mut Page| {
            p.title = "Livraison".into();
            p.body = "## Ce que c'est\n\nUne page.".into();
            p.tickets = vec!["ESC-586".into()];
        })
        .expect("a page of the spec");
    let proposal = |title: &str| Decision {
        title: title.into(),
        context: "Pourquoi".into(),
        decision: "Quoi".into(),
        ..Default::default()
    };
    let numbered = space
        .decision_propose(None, proposal("Une décision numérotée"))
        .unwrap();
    space
        .decision_accept(&numbered.reference(), "Moi", &Register::default(), None)
        .unwrap();
    space
        .decision_propose(None, proposal("Une proposition ouverte"))
        .unwrap();
    let pending = space
        .decision_propose(None, proposal("Acceptée sur le serveur d'équipe"))
        .unwrap();
    space
        .decision_accept_pending(&pending.reference(), "Moi", Some("ESC-586"))
        .unwrap();

    // The memory: a note as the 2.3 writes it.
    crate::memory::write(
        &repo.join(crate::memory::DIR),
        crate::memory::Draft {
            name: "pieges-de-la-beta".into(),
            description: "Ce qu'il faut savoir de la bêta".into(),
            kind: crate::memory::Kind::Feedback,
            body: "Revenir à la 2.2 garde les données.".into(),
        },
    )
    .expect("a memory");

    let expect = json!({
        "projects": ["p-compat", "p-plain"],
        "agents": ["a-1", "a-2"],
        "tickets": ["CMP-1", "PLN-1"],
        "reviewNotes": [1, 2],
        "asks": ["a-1"],
        "settings": { "language": "en", "betaUpdates": true },
        "projectFile": { "agents.engine": project_value(project, "agents.engine") },
        "sharedTicket": { "key": "PLN-1", "title": "Un ticket simple" },
        "page": { "path": "introduction/livraison", "title": "Livraison" },
        "decisions": ["Une décision numérotée", "Une proposition ouverte", "Acceptée sur le serveur d'équipe"],
        "memory": "pieges-de-la-beta",
    });
    write(
        &out.join("expect.json"),
        &serde_json::to_vec_pretty(&expect).unwrap(),
    );
}

fn project_value(p: &crate::model::Project, key: &str) -> Value {
    crate::projectfile::get(p, key)
}

#[test]
fn the_2_3_writes_what_the_way_back_to_the_2_2_reads() {
    let out = match std::env::var_os("ESCOUADE_COMPAT22_OUT") {
        Some(dir) => PathBuf::from(dir),
        None => std::env::temp_dir().join("escouade-compat22"),
    };
    let _ = std::fs::remove_dir_all(&out);
    write_all(&out);
    // The 2.3 reads back what it wrote: the files are whole.
    let (data, repo) = folders(&out);
    let state: PersistedState =
        serde_json::from_slice(&std::fs::read(data.join("state.json")).unwrap()).unwrap();
    assert_eq!(state.tickets.len(), 2);
    assert!(escouade_model::backups::reads(
        "settings.json",
        &std::fs::read(data.join("settings.json")).unwrap()
    ));
    let yml = std::fs::read_to_string(repo.join(crate::projectfile::FILE)).unwrap();
    assert!(yml.contains("team:"), "{yml}");
    let read = crate::projectfile::read(&yml).expect("the 2.3 reads its file");
    assert!(read.warnings.is_empty(), "{:?}", read.warnings);
    assert_eq!(Space::new(&repo, None).decisions().len(), 3);
    let frozen =
        std::fs::read_to_string(out.join("frozen/.escouade/tickets/q4w7c2na.yml")).unwrap();
    assert!(crate::sharing::is_frozen(&frozen));
}
