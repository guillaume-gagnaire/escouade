//! The sort of the seed (ESC-373, docs/PLUGINS.md §8.7): of the plugins of the first hand the app ships, those the user's
//! configuration uses, read in their settings, their projects, their agents and their accounts, never on the network. At
//! the start, `Plugins::sync_seed` installs these and nothing else; the others stay in the seed, proposed by the view
//! Plugins, and install with no network on the gesture that needs one (`Core::need_plugin`).
//!
//! The rule never makes a function of 2.1 lost: what does not read surely (a settings file that does not parse, a
//! project's folder or git configuration out of reach) wants every plugin it could have needed, and an id of the seed the
//! rule does not know is installed.

use super::builtin;
use crate::integrations::Accounts;
use crate::model::{EngineId, Project, Settings};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The plugin « Goose », a harness its manifest describes, served by `dev.escouade.acp`.
pub const GOOSE: &str = "dev.escouade.goose";
/// The plugin « isola », the launch environment of a worktree with an `.isola.toml`.
pub const ISOLA: &str = "dev.escouade.isola";

/// The plugins that install at the start only when they are installed already: their use is the plugin itself.
const ON_REQUEST: &[&str] = &[
    "dev.escouade.discord",
    "dev.escouade.slack",
    "dev.escouade.high-contrast",
    "dev.escouade.team-rules",
];

/// Whether `id` is a plugin of the first hand the seed holds though the registry has it not (`builtin::seeds`): Goose and
/// those on request.
pub fn seeds_beyond_the_registry(id: &str) -> bool {
    id == GOOSE || ON_REQUEST.contains(&id)
}

/// The name of a plugin on request in `lang`, for the view while it is not installed: they are not in the registry, whose
/// manifests name the others (`builtin::name`).
pub fn name_on_request(id: &str, lang: crate::i18n::Lang) -> Option<&'static str> {
    let fr = matches!(lang, crate::i18n::Lang::Fr);
    Some(match id {
        GOOSE => "Goose",
        "dev.escouade.discord" => "Discord",
        "dev.escouade.slack" => "Slack",
        "dev.escouade.high-contrast" if fr => "Contraste élevé",
        "dev.escouade.high-contrast" => "High contrast",
        "dev.escouade.team-rules" if fr => "Règles de l'équipe",
        "dev.escouade.team-rules" => "Team rules",
        _ => return None,
    })
}

/// The voice services of the first hand, by their id in the settings: their plugin is `dev.escouade.<id>`.
const VOICES: &[&str] = &["openai", "groq", "mistral", "elevenlabs", "deepgram"];

/// The code hosts of the first hand, by their kind (`forge::Provider::kind`), and their plugin.
const FORGES: &[(&str, &str)] = &[
    ("github", builtin::GITHUB),
    ("gitlab", builtin::GITLAB),
    ("bitbucket", builtin::BITBUCKET),
    ("gitea", builtin::GITEA),
    ("azure", builtin::AZURE_DEVOPS),
];

/// The plugins of the harness `engine` (an `EngineId`'s text): its own, and `dev.escouade.acp` for a harness a manifest
/// describes (Goose, a third party's). None for an empty id.
pub fn of_engine(engine: &str) -> Vec<&'static str> {
    let own = match engine {
        "" => return Vec::new(),
        "claude" => builtin::CLAUDE,
        "codex" => builtin::CODEX,
        "copilot" => builtin::COPILOT,
        "cursor" => builtin::CURSOR,
        "opencode" => builtin::OPENCODE,
        "vibe" => builtin::MISTRAL,
        "agy" => builtin::ANTIGRAVITY,
        "gemini" => builtin::GEMINI,
        "goose" => return vec![GOOSE, builtin::ACP],
        _ => return vec![builtin::ACP],
    };
    vec![own]
}

/// The plugin of the source of tickets `service` (a `Service`'s text).
pub fn of_service(service: &str) -> Option<&'static str> {
    Some(match service {
        "jira" => builtin::JIRA,
        "trello" => builtin::TRELLO,
        "github" => builtin::GITHUB,
        "linear" => builtin::LINEAR,
        "clickup" => builtin::CLICKUP,
        "gitlab" => builtin::GITLAB,
        "azure" => builtin::AZURE_DEVOPS,
        _ => return None,
    })
}

/// The plugin of the code host of kind `kind`.
pub fn of_forge(kind: &str) -> Option<&'static str> {
    FORGES.iter().find(|(k, _)| *k == kind).map(|(_, p)| *p)
}

/// The plugin of the voice service `id`.
pub fn of_voice(id: &str) -> Option<String> {
    VOICES.contains(&id).then(|| format!("dev.escouade.{id}"))
}

/// Whether the rule knows `id`: the plugins of harnesses, sources, code hosts, voices, isola and those on request. An
/// id of the seed it does not know installs, as in 2.1.
fn known(id: &str) -> bool {
    let harness = [
        builtin::CLAUDE,
        builtin::CODEX,
        builtin::COPILOT,
        builtin::CURSOR,
        builtin::OPENCODE,
        builtin::MISTRAL,
        builtin::ANTIGRAVITY,
        builtin::GEMINI,
        builtin::ACP,
        GOOSE,
    ];
    harness.contains(&id)
        || [
            builtin::JIRA,
            builtin::TRELLO,
            builtin::LINEAR,
            builtin::CLICKUP,
        ]
        .contains(&id)
        || FORGES.iter().any(|(_, p)| *p == id)
        || VOICES
            .iter()
            .any(|v| id.strip_prefix("dev.escouade.") == Some(v))
        || id == ISOLA
        || ON_REQUEST.contains(&id)
}

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
