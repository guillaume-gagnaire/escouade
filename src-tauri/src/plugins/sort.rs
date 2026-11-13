//! The sort of the seed (ESC-373, docs/PLUGINS-DESIGN.md §8.7): of the plugins of the first hand the app ships, those the user's
//! configuration uses, read in their settings, their projects, their agents and their accounts, never on the network. At
//! the start, `Plugins::sync_seed` installs these and nothing else; the others stay in the seed, proposed by the view
//! Plugins, and install with no network on the gesture that needs one (`Core::need_plugin`).
//!
//! The rule never makes a function of 2.1 lost: what does not read surely (a settings file that does not parse, a
//! project's folder or git configuration out of reach) wants every plugin it could have needed, and an id of the seed the
//! rule does not know is installed.

// The tables (the plugins of each harness, source, code host and voice), in `escouade-plugins`; what reads the configuration stays.
use crate::integrations::Accounts;
use crate::model::{EngineId, Project, Settings};
pub use escouade_plugins::sort::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// What the rule reads.
pub struct Config<'a> {
    pub settings: &'a Settings,
    pub projects: &'a [Project],
    /// The harness of each agent, archived or not.
    pub agents: &'a [EngineId],
    /// The accounts of the sources of tickets and of the code hosts (`integrations.json`).
    pub accounts: &'a Accounts,
    /// The voice services whose key is kept (their presence, never their value).
    pub voice_keys: &'a [String],
    /// The settings, the state and the accounts read as they were saved: false when one of them is there and does not
    /// parse (the app then starts with its defaults, which say nothing of the user).
    pub readable: bool,
}

/// What the rule found: the plugins the configuration uses, or all when it does not read.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Sorted {
    pub used: BTreeSet<String>,
    /// Something did not read: every plugin is wanted.
    pub all: bool,
}

impl Sorted {
    /// Whether the start installs the seed's `id`.
    pub fn wants(&self, id: &str) -> bool {
        self.all || self.used.contains(id) || !known(id)
    }

    fn add(&mut self, id: &str) {
        self.used.insert(id.to_string());
    }
}

/// The plugins `c` uses.
pub fn sort(c: &Config) -> Sorted {
    let mut out = Sorted {
        all: !c.readable,
        ..Default::default()
    };
    // The harnesses: an account, an agent, a project's default one, the harness of the texts.
    let engines = c
        .settings
        .accounts
        .iter()
        .map(|a| a.engine.as_str())
        .chain(c.agents.iter().map(|e| e.as_str()))
        .chain(
            c.projects
                .iter()
                .filter_map(|p| p.engine.map(|e| e.as_str())),
        )
        .chain(c.settings.writer_engine.map(|e| e.as_str()));
    for e in engines {
        for p in of_engine(e) {
            out.add(p);
        }
    }
    // The sources and the code hosts: an account of the service, unless another plugin serves it.
    let mine = |plugin: &str, own: &str| plugin.is_empty() || plugin == own;
    for (service, account) in [
        ("jira", &c.accounts.jira),
        ("trello", &c.accounts.trello),
        ("github", &c.accounts.github),
        ("linear", &c.accounts.linear),
        ("clickup", &c.accounts.clickup),
    ] {
        if let (Some(a), Some(p)) = (account, of_service(service)) {
            if mine(&a.plugin, p) {
                out.add(p);
            }
        }
    }
    for a in &c.accounts.forges {
        if let Some(p) = of_forge(&a.kind).filter(|p| mine(&a.plugin, p)) {
            out.add(p);
        }
    }
    // The voice: the service of the dictation or of the voice when it is on line, and a service whose key is kept.
    let voice = &c.settings.voice;
    let chosen = [
        (voice.transcription == "online").then_some(voice.online_service.as_str()),
        (voice.speech.engine == "online").then_some(voice.speech.online_service.as_str()),
    ];
    for id in chosen
        .into_iter()
        .flatten()
        .chain(c.voice_keys.iter().map(String::as_str))
    {
        if let Some(p) = of_voice(id) {
            out.add(&p);
        }
    }
    // The projects: their sources, their code host, isola.
    for project in c.projects {
        for p in of_project(project, c.accounts) {
            out.add(p);
        }
    }
    out
}

/// What the project uses: the plugins of its sources, of the code host of its remotes, isola for an `.isola.toml`. A
/// folder out of reach (a disk not mounted) or a git configuration that does not read: every code host, and isola.
pub fn of_project(project: &Project, accounts: &Accounts) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = project
        .integrations
        .links
        .iter()
        .filter_map(|l| of_service(l.service.as_str()))
        .collect();
    let every_host = |out: &mut Vec<&'static str>| out.extend(FORGES.iter().map(|(_, p)| *p));
    let root = Path::new(&project.path);
    if !root.is_dir() {
        every_host(&mut out);
        out.push(ISOLA);
    } else {
        let repos: Vec<PathBuf> = if project.repos.is_empty() {
            vec![root.to_path_buf()]
        } else {
            project.repos.iter().map(|r| root.join(r)).collect()
        };
        for repo in repos {
            if repo.join(".isola.toml").is_file() {
                out.push(ISOLA);
            }
            match remote_hosts(&repo) {
                Some(hosts) => out.extend(hosts.iter().filter_map(|h| forge_of_host(h, accounts))),
                None => every_host(&mut out),
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The plugin of the code host at `host`: by its name (github.com, gitlab.com…), else by an account connected for it.
fn forge_of_host(host: &str, accounts: &Accounts) -> Option<&'static str> {
    if let Some((provider, _)) = crate::forge::Registry::standard().of_host(host) {
        return of_forge(&provider.kind());
    }
    let bare = crate::forge::bare_host(host);
    accounts
        .forges
        .iter()
        .find(|a| crate::forge::bare_host(&crate::forge::host_of(&a.url)) == bare)
        .and_then(|a| of_forge(&a.kind))
}

/// The hosts of the remotes of the repository at `repo`, read in its git configuration (no `git` run): empty for a folder
/// that is no repository, none when its configuration does not read.
pub fn remote_hosts(repo: &Path) -> Option<Vec<String>> {
    let dot = repo.join(".git");
    let config = if dot.is_dir() {
        dot.join("config")
    } else if dot.is_file() {
        // A worktree or a submodule: `gitdir: <path>`, its configuration in the common folder.
        let text = std::fs::read_to_string(&dot).ok()?;
        let gitdir = text.lines().find_map(|l| l.strip_prefix("gitdir:"))?.trim();
        let gitdir = repo.join(gitdir);
        let common = std::fs::read_to_string(gitdir.join("commondir"))
            .ok()
            .map(|c| gitdir.join(c.trim()))
            .unwrap_or(gitdir);
        common.join("config")
    } else {
        return Some(Vec::new());
    };
    let text = std::fs::read_to_string(config).ok()?;
    let mut hosts = Vec::new();
    let mut in_remote = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_remote = line.starts_with("[remote ");
            continue;
        }
        if !in_remote {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("url") || key.trim().eq_ignore_ascii_case("pushurl") {
            let value = value.trim().trim_matches('"');
            if let Some(a) = crate::forge::remote::parse(value) {
                hosts.push(a.host);
            }
        }
    }
    Some(hosts)
}

/// Whether the JSON file at `path` reads as the app saved it: absent (a first start) or parsed.
pub fn reads(path: &Path) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<serde::de::IgnoredAny>(&bytes).is_ok(),
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    }
}
