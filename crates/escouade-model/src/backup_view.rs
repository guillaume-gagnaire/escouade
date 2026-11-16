//! What a copy of a file of the data folder holds, told in clear, and what taking it back would change (ESC-487).
//!
//! The screen of the copies (Réglages › Avancé) and the MCP tool `list_backups` say of each copy how many projects,
//! agents and tickets it holds (the state), how many accounts (the settings, the integrations), how many plugins, and
//! whether it reads at all; « Comparer » says, by name, what a copy has that the file of today has not and the other way
//! round. Nothing here writes, and nothing here ever gives a value of a field: the copies of `settings.json` and of
//! `integrations.json` may hold tokens, only counts and names (of projects, tickets, agents, accounts, plugins) come out.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a copy (or the file of today) holds, in counts. The fields that do not apply to the file are None.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    /// The content reads as Escouade reads that file (`backups::reads`).
    pub readable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tickets: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accounts: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<usize>,
    /// The entries of a list (the syncs waiting, the agents followed in the workflows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<usize>,
    /// The name of the project a closed project's history is of.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// What taking a copy back gains and loses against the file of today, by kind (`projects`, `tickets`, `agents`,
/// `accounts`, `plugins`): the names of what the copy has and today has not (`gained`), and of what today has and the
/// copy has not (`lost`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diff {
    pub copy: Summary,
    pub today: Summary,
    pub kinds: Vec<KindDiff>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KindDiff {
    /// `projects`, `tickets`, `agents`, `accounts` or `plugins`.
    pub kind: String,
    pub gained: Vec<String>,
    pub lost: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Named {
    id: String,
    name: String,
    archived: bool,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct TicketLite {
    id: String,
    key: String,
    title: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct StateLite {
    projects: Vec<Named>,
    agents: Vec<Named>,
    tickets: Vec<TicketLite>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ClosedLite {
    project: Named,
    agents: Vec<Named>,
    tickets: Vec<TicketLite>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct SettingsLite {
    accounts: Vec<Named>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PluginLite {
    id: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PluginsLite {
    plugins: Vec<PluginLite>,
}

/// The kind of a file of the data folder, by its path in it (`state.json`, `closed-projects/<id>.json`…).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    State,
    Settings,
    Plugins,
    Integrations,
    TicketAccounts,
    Closed,
    List,
    Other,
}

fn kind(file: &str) -> Kind {
    match file {
        "state.json" => Kind::State,
        "settings.json" => Kind::Settings,
        "plugins.json" => Kind::Plugins,
        "integrations.json" => Kind::Integrations,
        "ticket-accounts.json" => Kind::TicketAccounts,
        "sync-queue.json" => Kind::List,
        f if f.starts_with("closed-projects/") => Kind::Closed,
        f if f.ends_with("/workflow-tracks.json") => Kind::List,
        _ => Kind::Other,
    }
}

/// The names of what a content holds, by kind, each with its id (what tells two apart): never a value of a field but
/// the names of projects, tickets (`KEY title`), agents, accounts (the settings' names, the integrations' services) and
/// plugins (their ids).
fn names(file: &str, bytes: &[u8]) -> BTreeMap<&'static str, Vec<(String, String)>> {
    let mut out: BTreeMap<&'static str, Vec<(String, String)>> = BTreeMap::new();
    let ticket = |t: TicketLite| {
        let name = match (t.key.is_empty(), t.title.is_empty()) {
            (false, false) => format!("{} {}", t.key, t.title),
            (false, true) => t.key.clone(),
            _ => t.title.clone(),
        };
        (t.id, name)
    };
    let agents = |a: Vec<Named>| {
        a.into_iter()
            .filter(|a| !a.archived)
            .map(|a| (a.id, a.name))
            .collect::<Vec<_>>()
    };
    match kind(file) {
        Kind::State => {
            let s: StateLite = serde_json::from_slice(bytes).unwrap_or_default();
            out.insert(
                "projects",
                s.projects.into_iter().map(|p| (p.id, p.name)).collect(),
            );
            out.insert("agents", agents(s.agents));
            out.insert("tickets", s.tickets.into_iter().map(ticket).collect());
        }
        Kind::Closed => {
            let c: ClosedLite = serde_json::from_slice(bytes).unwrap_or_default();
            out.insert("agents", agents(c.agents));
            out.insert("tickets", c.tickets.into_iter().map(ticket).collect());
        }
        Kind::Settings => {
            let s: SettingsLite = serde_json::from_slice(bytes).unwrap_or_default();
            out.insert(
                "accounts",
                s.accounts.into_iter().map(|a| (a.id, a.name)).collect(),
            );
        }
        Kind::Plugins => {
            let p: PluginsLite = serde_json::from_slice(bytes).unwrap_or_default();
            out.insert(
                "plugins",
                p.plugins
                    .into_iter()
                    .map(|p| (p.id.clone(), p.id))
                    .collect(),
            );
        }
        Kind::Integrations => {
            let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_default();
            let mut accounts = Vec::new();
            // One account per service that has one (its name only), then the code hosts' (by their host's name).
            for service in ["jira", "trello", "github", "linear", "clickup"] {
                if v.get(service).is_some_and(|a| a.is_object()) {
                    accounts.push((service.to_string(), service.to_string()));
                }
            }
            for f in v
                .get("forges")
                .and_then(|f| f.as_array())
                .into_iter()
                .flatten()
            {
                // Told apart by their instance, named by their provider's kind (`gitlab`) alone.
                let field = |k: &str| f.get(k).and_then(|x| x.as_str()).unwrap_or_default();
                let kind = if field("kind").is_empty() {
                    "forge"
                } else {
                    field("kind")
                };
                accounts.push((format!("{kind} {}", field("url")), kind.to_string()));
            }
            out.insert("accounts", accounts);
        }
        Kind::TicketAccounts => {
            let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_default();
            let accounts = v
                .get("accounts")
                .and_then(|a| a.as_array())
                .into_iter()
                .flatten()
                .map(|a| {
                    let id = a.get("id").and_then(|x| x.as_str()).unwrap_or_default();
                    let service = a
                        .get("service")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default();
                    (id.to_string(), service.to_string())
                })
                .collect();
            out.insert("accounts", accounts);
        }
        Kind::List | Kind::Other => {}
    }
    out
}

/// What `bytes`, a content of `file`, hold.
pub fn summarize(file: &str, bytes: &[u8]) -> Summary {
    let readable = crate::backups::reads(file, bytes);
    let mut s = Summary {
        readable,
        ..Default::default()
    };
    if !readable {
        return s;
    }
    let n = names(file, bytes);
    let count = |k: &str| n.get(k).map(Vec::len);
    s.projects = count("projects");
    s.agents = count("agents");
    s.tickets = count("tickets");
    s.accounts = count("accounts");
    s.plugins = count("plugins");
    match kind(file) {
        Kind::Closed => {
            let c: ClosedLite = serde_json::from_slice(bytes).unwrap_or_default();
            s.name = Some(c.project.name);
        }
        Kind::List => {
            let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_default();
            s.entries = Some(match &v {
                serde_json::Value::Array(a) => a.len(),
                serde_json::Value::Object(o) => o.len(),
                _ => 0,
            });
        }
        _ => {}
    }
    s
}

/// What taking back `copy` in the place of `today` (both contents of `file`; `today` empty when the file is not there)
/// gains and loses, by name. Nothing is written.
pub fn diff(file: &str, copy: &[u8], today: &[u8]) -> Diff {
    let summary_of_today = if today.is_empty() {
        Summary::default()
    } else {
        summarize(file, today)
    };
    let copy_summary = summarize(file, copy);
    let a = if copy_summary.readable {
        names(file, copy)
    } else {
        Default::default()
    };
    let b = if summary_of_today.readable {
        names(file, today)
    } else {
        Default::default()
    };
    let mut kinds = Vec::new();
    let empty = Vec::new();
    // The names of `from` whose id `other` has not.
    let not_in = |from: &[(String, String)], other: &[(String, String)]| -> Vec<String> {
        let ids: std::collections::HashSet<&str> =
            other.iter().map(|(id, _)| id.as_str()).collect();
        from.iter()
            .filter(|(id, _)| !ids.contains(id.as_str()))
            .map(|(_, n)| n.clone())
            .collect()
    };
    for k in ["projects", "tickets", "agents", "accounts", "plugins"] {
        if !a.contains_key(k) && !b.contains_key(k) {
            continue;
        }
        let x = a.get(k).unwrap_or(&empty);
        let y = b.get(k).unwrap_or(&empty);
        kinds.push(KindDiff {
            kind: k.to_string(),
            gained: not_in(x, y),
            lost: not_in(y, x),
        });
    }
    Diff {
        copy: copy_summary,
        today: summary_of_today,
        kinds,
    }
}

#[cfg(test)]
#[path = "backup_view_tests.rs"]
mod tests;
