//! An entry of the team's list of projects (TS-8, `catalog.rs`): « ce qui vient de l'équipe n'est pas de confiance
//! aveugle ». Its addresses go to `git clone`, its folders to the disk.
//! - The bytes as an address: one that passes is https or ssh, names a host, starts with no dash, holds no space nor
//!   control character, no part of its path reads as an option or climbs (`..`), and it passes again the same.
//! - The bytes as an entry (CBOR, as the stream carries it once opened): a project that `check` lets through has
//!   addresses that pass, folders that stay in the project, a branch git takes for one, a Kanban of 32 digits.
//! - An entry sealed by Alice, changed by the bytes: Bob opens the very entry she wrote, or nothing.

use crate::catalog::{address, check, key, Entry, Listed, Scheme, CATALOG_STREAM};
use crate::wire;

use super::{alter, World};

pub(super) fn listed() -> Listed {
    Listed {
        name: "Acme API".into(),
        color: "#336699".into(),
        logo: None,
        repos: vec![crate::catalog::Repo {
            url: "https://github.com/acme/api.git".into(),
            folder: String::new(),
        }],
        branch: "main".into(),
        access: crate::catalog::Access::Team,
        board: "0123456789abcdef0123456789abcdef".into(),
    }
}

fn address_holds(raw: &str) {
    let Ok(a) = address(raw) else {
        return;
    };
    assert!(matches!(a.scheme, Scheme::Https | Scheme::Ssh));
    assert!(!a.url.starts_with('-'), "an option for git: {raw}");
    assert!(
        !a.url.chars().any(|c| c.is_whitespace() || c.is_control()),
        "a blank in {raw:?}"
    );
    assert!(
        !a.host.is_empty() && !a.host.starts_with('-'),
        "a host that is not one: {raw:?}"
    );
    for part in a.path.split('/') {
        assert!(
            !part.is_empty() && !part.starts_with('-') && part != ".." && part != ".",
            "{raw:?}"
        );
    }
    assert!(
        !a.url.to_ascii_lowercase().starts_with("file:"),
        "a local address: {raw:?}"
    );
    // git reads a remote helper only at the head of the address (`<transport>::…`, transport.c); further in, `::` is
    // a part of the path.
    let head = a
        .url
        .find(|c: char| !(c.is_ascii_alphanumeric() || "+-.".contains(c)))
        .unwrap_or(a.url.len());
    assert!(!a.url[head..].starts_with("::"), "a remote helper: {raw:?}");
    assert_eq!(
        address(&a.url).as_ref(),
        Ok(&a),
        "an address that passes once"
    );
    assert_eq!(key(&a.url), Some(a.key()));
}

fn listed_holds(project: &Listed) {
    if check(project).is_err() {
        return;
    }
    for r in &project.repos {
        assert!(address(&r.url).is_ok());
    }
    // A folder never leaves the project nor reads as an option. Several repositories: no hidden one either. One
    // repository: its folder is the clone's, which the member chooses in the wizard; the name the wizard proposes is
    // never hidden (`folderName`, src/lib/team-projects.ts: an address `…/.ssh` found by this target, TS-18).
    let several = project.repos.len() > 1;
    for f in project.folders() {
        assert!(
            !f.is_empty()
                && !f.starts_with('-')
                && !f.contains(['/', '\\'])
                && f != ".."
                && f != "."
                && !(several && f.starts_with('.')),
            "a folder out of the project: {f:?}"
        );
    }
    let b = project.branch.trim();
    assert!(
        !b.starts_with('-') && !b.contains(".."),
        "a branch git reads otherwise: {b:?}"
    );
    if !project.board.trim().is_empty() {
        assert!(project.board_id().is_some());
    }
}

pub fn catalog(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        address_holds(text);
        // An address inside a project of the list.
        let mut project = listed();
        project.repos[0].url = text.to_owned();
        listed_holds(&project);
        project.repos.push(crate::catalog::Repo {
            url: "git@github.com:acme/web.git".into(),
            folder: text.to_owned(),
        });
        listed_holds(&project);
        project.repos.truncate(1);
        project.repos[0].url = "https://github.com/acme/api.git".into();
        project.branch = text.to_owned();
        listed_holds(&project);
    }
    if let Ok(Entry::Put { project, .. }) = wire::decode::<Entry>(data) {
        listed_holds(&project);
    }
    // Sealed once per thread: the same bytes for every input, as the fuzzer wants.
    thread_local! {
        static SEALED: (Vec<u8>, Vec<u8>) = World::with(|w| {
            let entry = Entry::Put {
                id: [1; 16],
                project: listed(),
                at: 1,
            };
            let plain = wire::encode(&entry);
            let sealed = w
                .alice
                .seal(CATALOG_STREAM, escouade_team_crypto::ObjectId([2; 16]), 0, &plain)
                .expect("sealed");
            (plain, wire::encode(&sealed))
        });
    }
    SEALED.with(|(plain, sealed)| {
        let changed = alter(sealed, data);
        if let Ok(o) = wire::decode::<escouade_team_crypto::content::SealedObject>(&changed) {
            if let Ok(opened) = World::with(|w| w.bob.open(&o)) {
                assert_eq!(&opened, plain, "an entry changed opens as another");
            }
        }
    });
}
