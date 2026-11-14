//! The language servers of the core (ESC-446), common to every way of the editor (docs/EDITOR-STUDY.md, ticket A1):
//! which for which languages (`catalog`), where they are (`locate`), what the settings and the project's
//! `.escouade.yml` say of them (`config`), whether they may run (`launch_for`), and those running (`manager`), their
//! messages carried by `transport`.
//!
//! A language server is a program of the machine, chosen by the user or found in the project. One the repository chose
//! (a `node_modules/.bin` of the project, the path or the arguments of its `.escouade.yml`) runs only once the user
//! trusted the project (`Settings::language_servers_trusted`): a cloned repository can hold a false server.

pub mod app;
pub mod catalog;
pub mod locate;
pub mod manager;
pub mod transport;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};
use tauri::{AppHandle, Emitter, Runtime};

use crate::model::LanguageServerSetting;
use catalog::Server;
use locate::{Found, NotFound, Places};
pub use manager::{Info, Launch, Manager, Policy, Sink};

/// The group of `.escouade.yml` that holds them: `language_servers.<id>` (`enabled`, `path`, `args`).
pub const FILE_GROUP: &str = "language_servers";

/// What the app's settings and the project's file say of a server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pub enabled: bool,
    /// Who set `enabled`: `project` (`.escouade.yml`, which goes first), `app`, or `default`.
    pub enabled_by: &'static str,
    /// The path set in the app's settings (empty for none): it wins.
    pub manual: String,
    /// The path of `.escouade.yml`.
    pub file_path: Option<String>,
    pub args: Vec<String>,
    /// The arguments are the file's: a choice of the repository, as its path.
    pub file_args: bool,
}

impl Config {
    /// Whether the repository chose what runs, beyond the program found.
    fn of_project(&self) -> bool {
        self.file_args
    }
}

/// The file's settings of a server (`language_servers.<id>`, as `projectfile` read them).
pub fn file_settings<'a>(
    values: &'a BTreeMap<String, Value>,
    id: &str,
) -> Option<&'a Map<String, Value>> {
    values
        .get(&format!("{FILE_GROUP}.{id}"))
        .and_then(Value::as_object)
}

/// What holds for `server`: the project's file over the app's settings over the catalog, but for the path set by
/// hand in the app, which wins.
pub fn config(
    server: &Server,
    app: &BTreeMap<String, LanguageServerSetting>,
    file: Option<&Map<String, Value>>,
) -> Config {
    let own = app.get(server.id).cloned().unwrap_or_default();
    let file_enabled = file.and_then(|f| f.get("enabled")).and_then(Value::as_bool);
    let (enabled, enabled_by) = match (file_enabled, own.enabled) {
        (Some(on), _) => (on, "project"),
        (None, Some(on)) => (on, "app"),
        (None, None) => (true, "default"),
    };
    let file_path = file
        .and_then(|f| f.get("path"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let file_args: Option<Vec<String>> = file.and_then(|f| f.get("args")).and_then(|a| {
        a.as_array().map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
    });
    let (args, from_file) = match (file_args, own.args) {
        (Some(a), _) => (a, true),
        (None, Some(a)) => (a, false),
        (None, None) => (server.args.iter().map(|s| s.to_string()).collect(), false),
    };
    Config {
        enabled,
        enabled_by,
        manual: own.path.trim().to_string(),
        file_path,
        args,
        file_args: from_file,
    }
}

/// A server, as the settings and the editor show it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub id: &'static str,
    pub name: &'static str,
    pub languages: &'static [&'static str],
    pub enabled: bool,
    pub enabled_by: &'static str,
    pub found: Option<Found>,
    pub missing: Option<NotFound>,
    /// The command that installs it.
    pub install: &'static str,
    pub args: Vec<String>,
    /// The repository chose what runs (a program of its own, its arguments): asked to the user first.
    pub of_project: bool,
    /// It runs code of the project (`Server::runs_project_code`): asked to the user first too.
    pub runs_project_code: bool,
    /// It waits for the project's trust to start in it: `of_project` or `runs_project_code`.
    pub needs_trust: bool,
    /// The project is trusted with its own servers.
    pub trusted: bool,
}

pub fn status(server: &'static Server, config: &Config, places: &Places, trusted: bool) -> Status {
    let found = locate::find(server, &config.manual, config.file_path.as_deref(), places);
    let of_project =
        config.of_project() || found.as_ref().is_ok_and(|f: &Found| f.origin.of_project());
    let (found, missing) = match found {
        Ok(f) => (Some(f), None),
        Err(m) => (None, Some(m)),
    };
    Status {
        id: server.id,
        name: server.name,
        languages: server.languages,
        enabled: config.enabled,
        enabled_by: config.enabled_by,
        found,
        missing,
        install: server.install,
        args: config.args.clone(),
        of_project,
        runs_project_code: server.runs_project_code,
        needs_trust: of_project || server.runs_project_code,
        trusted,
    }
}

/// Why a server waits for the project's trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrustReason {
    /// The repository chose it: a program of its `node_modules/.bin`, the path or the arguments of its `.escouade.yml`.
    Chosen,
    /// It runs code of the project, wherever it comes from (`Server::runs_project_code`).
    RunsProjectCode,
}

/// Why a server is not started: the window tells each one apart by its `code`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum Refused {
    /// No server of this id or for this language.
    Unknown {
        name: String,
    },
    /// Turned off, in the app's settings or by the project.
    Disabled {
        by: &'static str,
    },
    NotFound {
        missing: NotFound,
    },
    /// A server the repository chose, or one that runs its code: the user trusts the project first (`lsp_trust`).
    NeedsTrust {
        server: &'static str,
        program: String,
        args: Vec<String>,
        reason: TrustReason,
    },
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::Unknown { name } => write!(f, "{}", tr!(
                "aucun serveur de langage pour « {name} »",
                "no language server for “{name}”"
            )),
            Refused::Disabled { by } => match *by {
                "project" => write!(f, "{}", tr!(
                    "serveur de langage coupé par le .escouade.yml du projet",
                    "language server turned off by the project's .escouade.yml"
                )),
                _ => write!(f, "{}", tr!(
                    "serveur de langage coupé dans les réglages",
                    "language server turned off in the settings"
                )),
            },
            Refused::NotFound { missing } => match missing {
                NotFound::Manual { path } => write!(f, "{}", tr!(
                    "le chemin réglé à la main ne mène à aucun programme : {path}",
                    "the path set by hand leads to no program: {path}"
                )),
                NotFound::ProjectFile { path } => write!(f, "{}", tr!(
                    "le chemin du .escouade.yml ne mène à aucun programme du dépôt : {path}",
                    "the path of .escouade.yml leads to no program of the repository: {path}"
                )),
                NotFound::Absent { install } => write!(f, "{}", tr!(
                    "serveur de langage introuvable ; pour l’installer : {install}",
                    "language server not found; to install it: {install}"
                )),
            },
            Refused::NeedsTrust {
                program,
                reason: TrustReason::Chosen,
                ..
            } => write!(f, "{}", tr!(
                "ce serveur de langage vient du projet ({program}) : confirme d’abord que tu fais confiance au projet",
                "this language server comes from the project ({program}): confirm first that you trust the project"
            )),
            Refused::NeedsTrust {
                server,
                reason: TrustReason::RunsProjectCode,
                ..
            } => {
                let what = runs_what(server);
                write!(f, "{}", tr!(
                    "ce serveur exécute du code du projet ({what}) : confirme d’abord que tu fais confiance au projet",
                    "this server runs code of the project ({what}): confirm first that you trust the project"
                ))
            }
        }
    }
}

/// What of the project a server runs (`Server::runs_project_code`), as its refusal says it.
pub fn runs_what(server: &str) -> String {
    match server {
        "rust" => tr!("build.rs, macros", "build.rs, macros"),
        "typescript" => tr!(
            "le tsserver du node_modules du projet",
            "the tsserver of the project's node_modules"
        ),
        "go" => tr!(
            "la toolchain de son go.mod, cgo",
            "the toolchain of its go.mod, cgo"
        ),
        _ => String::new(),
    }
}

/// What to run for `server` in `cwd`, when it may.
pub fn launch_for(
    server: &Server,
    config: &Config,
    places: &Places,
    trusted: bool,
    cwd: PathBuf,
) -> Result<Launch, Refused> {
    if !config.enabled {
        return Err(Refused::Disabled {
            by: config.enabled_by,
        });
    }
    let found = locate::find(server, &config.manual, config.file_path.as_deref(), places)
        .map_err(|missing| Refused::NotFound { missing })?;
    let chosen = found.origin.of_project() || config.of_project();
    // What the repository chose is said first: it is the bigger risk.
    let reason = if chosen {
        Some(TrustReason::Chosen)
    } else if server.runs_project_code {
        Some(TrustReason::RunsProjectCode)
    } else {
        None
    };
    if let (Some(reason), false) = (reason, trusted) {
        return Err(Refused::NeedsTrust {
            server: server.id,
            program: found.program.display().to_string(),
            args: config.args.clone(),
            reason,
        });
    }
    // It ran because the project is trusted: it stops when it no longer is.
    let of_project = reason.is_some();
    Ok(Launch {
        program: found.program,
        args: config.args.clone(),
        cwd,
        of_project,
    })
}

/// The server of `name`: a server's id, or a language's.
pub fn server_named(name: &str) -> Result<&'static Server, Refused> {
    catalog::get(name)
        .or_else(|| catalog::for_language(name))
        .ok_or_else(|| Refused::Unknown {
            name: name.to_string(),
        })
}

/// The window's: messages as `lsp-message` (`{ id, msg }`), states as `lsp-state` (`Info`).
pub struct TauriSink<R: Runtime>(pub AppHandle<R>);

#[derive(Clone, Serialize)]
struct Message<'a> {
    id: u32,
    msg: &'a str,
}

impl<R: Runtime> Sink for TauriSink<R> {
    fn message(&self, id: u32, msg: &str) {
        let _ = self.0.emit("lsp-message", Message { id, msg });
    }
    fn state(&self, info: &Info) {
        let _ = self.0.emit("lsp-state", info);
    }
}

/// The servers of the app, its window their sink.
pub fn for_app<R: Runtime>(app: AppHandle<R>) -> Arc<Manager> {
    Manager::new(Arc::new(TauriSink(app)), Policy::default())
}
