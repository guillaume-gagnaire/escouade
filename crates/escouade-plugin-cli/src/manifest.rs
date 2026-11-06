//! The manifest of a plugin, `escouade-plugin.json` at the root of its folder (docs/PLUGINS.md
//! §3.1, ESC-173): read, checked field by field, and its range of Escouade compared to this one.
//!
//! The checks are those of the store's library (`schemas/plugin-manifest.mjs`, which the store
//! runs on a submitted plugin, STORE-3): both readers go through the same cases
//! (`tests/fixtures/plugins/manifests.json`). The schema published for the editors' completion
//! (`schemas/plugin-1.json`) is made from the same rules (`schema`). A field Escouade does not
//! know is ignored, and listed (`Read::ignored`): a manifest written for a later Escouade loads.
//!
//! Its `icon` is a file of the plugin's folder (PLG-13): `check_with` reads its header through a
//! reader of the folder (a PNG, square, 256 px a side at least, 1 MB at most); `icon_file` never
//! leaves the folder.

use crate::i18n::Lang;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::path::Path;

/// A version number (`2.1.301`), compared number by number (a missing one counts as 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version(Vec<u64>);

impl Version {
    /// The first version number of `text` (`2.1.301 (Claude Code)`, `codex-cli 0.163.0`).
    pub fn find(text: &str) -> Option<Version> {
        text.split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .map(|w| w.trim_matches('.'))
            .find(|w| w.contains('.') && w.split('.').all(|n| !n.is_empty()))
            .and_then(|w| w.split('.').map(|n| n.parse().ok()).collect::<Option<_>>())
            .map(Version)
    }

    pub fn parse(text: &str) -> Version {
        Version::find(text).unwrap_or(Version(Vec::new()))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<String> = self.0.iter().map(u64::to_string).collect();
        f.write_str(&parts.join("."))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let n = self.0.len().max(other.0.len());
        let at = |v: &Version, i: usize| v.0.get(i).copied().unwrap_or(0);
        (0..n)
            .map(|i| at(self, i).cmp(&at(other, i)))
            .find(|o| o.is_ne())
            .unwrap_or(Ordering::Equal)
    }
}

/// The manifest's file in a plugin's folder.
pub const FILE: &str = "escouade-plugin.json";
/// Where the site serves the schema (`schemas/plugin-1.json` of the repository).
pub const SCHEMA_URL: &str = "https://escouade.dev/schemas/plugin-1.json";
/// The major version of the plugins' protocol this Escouade serves (`escouade.api`).
pub const API: u64 = 1;
/// The ids of Escouade's own plugins: those the app gives (`Hand::Builtin`) and those of its
/// repository's `plugins/<id>/` (`Hand::First`, ESC-314).
pub const RESERVED_PREFIX: &str = "dev.escouade.";

/// Whose hand a manifest is read for: what it may take of Escouade's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hand {
    /// Anyone's plugin: neither the ids `dev.escouade.*` nor the app's programs.
    #[default]
    Third,
    /// A plugin of Escouade's own hand that the app does not give (`plugins/<id>/` of its
    /// repository, ESC-314): a version of the store signed for it, or its folder in development.
    /// It may take the ids `dev.escouade.*`, never the app's programs (`{escouade}/…`).
    First,
    /// A plugin the app itself gives (INT-4's origin `builtin`): its ids and its programs.
    Builtin,
}
/// The folder of the app's programs, in the `run` of a plugin of Escouade's own.
pub const ESCOUADE_DIR: &str = "{escouade}";
/// The side an icon has at least, in pixels (512 advised).
pub const ICON_MIN: u32 = 256;
/// The weight an icon has at most, in bytes.
pub const ICON_BYTES: u64 = 1024 * 1024;

/// The values `contributes.settings[].type` takes.
const SETTING_TYPES: [&str; 4] = ["string", "number", "boolean", "select"];
/// The extension points whose items carry an `id`, unique in the list.
const POINTS_WITH_ID: [&str; 6] = [
    "accounts",
    "ticketSources",
    "forges",
    "harnesses",
    "workflowReaders",
    "launchEnvironments",
];
/// The extension points whose items are left to their own tickets (PLG-5, PLG-7…): a list of
/// objects.
const OPEN_POINTS: [&str; 7] = [
    "themes",
    "webhooks",
    "commands",
    "menus",
    "statusBar",
    "badges",
    "projectTemplates",
];

/// What is wrong with a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Code {
    /// Missing.
    Required,
    /// Of another type than `Problem::expected`.
    Type,
    /// An empty text.
    Empty,
    /// An id that is no reversed domain name (`dev.exemple.slack`).
    Id,
    /// An id or a program of Escouade's own, in a plugin that is not.
    Reserved,
    /// No version number (`1.2.0`).
    Version,
    /// No major version of the protocol (`"1"`).
    Api,
    /// `escouade.max` before `escouade.min`.
    Range,
    /// No `http(s)://` address.
    Url,
    /// A path that leaves the plugin's folder.
    Path,
    /// A key of a setting, a secret or a point that is no name (`channel`).
    Key,
    /// A value out of `Problem::expected`'s list.
    Enum,
    /// A second item of the same key or id.
    Duplicate,
    /// An account the plugin does not declare.
    Account,
    /// An icon that is no file of the plugin's folder.
    File,
    /// An icon that is no PNG (an SVG, a JPEG…).
    Png,
    /// An icon over `ICON_BYTES`.
    Heavy,
    /// An icon that is not square.
    Square,
    /// An icon under `ICON_MIN` a side.
    Small,
    /// A webhook to a host `permissions.network` does not cover (`host_covered`).
    Network,
    /// Two of `run.command`, `run.js` and `run.wasm`: a program is one of them (ESC-320, ESC-331).
    Exclusive,
    /// A `run.wasm` that names no `Cargo.toml`: a module is built, never brought (ESC-331).
    Cargo,
    /// A command that does not start with a program's name, or a `*` that is not its last word (ESC-333).
    Command,
    /// Writing in the plugin's own folder: what the store signed stays as it was (ESC-333).
    ReadOnly,
}

/// A field that is wrong, by its path (`contributes.settings[0].key`; empty for the whole file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub path: String,
    pub code: Code,
    /// The type, or the values, the field takes, for `Type` and `Enum`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
}

impl Problem {
    /// What is wrong, in `lang`, its field first.
    pub fn message(&self, lang: Lang) -> String {
        let field = if self.path.is_empty() {
            tr_in!(lang, "Le manifeste", "The manifest")
        } else {
            format!("`{}`", self.path)
        };
        let expected = self.expected.as_deref().unwrap_or("");
        let what = match self.code {
            Code::Required => tr_in!(lang, "manque.", "is missing."),
            Code::Type => tr_in!(
                lang,
                "devrait être {}.",
                "should be {}.",
                type_name(lang, expected)
            ),
            Code::Empty => tr_in!(
                lang,
                "est vide (un texte, ou `en` ou `fr` d'un texte dans les deux langues).",
                "is empty (a text, or the `en` or `fr` of a text in both languages)."
            ),
            Code::Id => tr_in!(
                lang,
                "n'est pas un nom de domaine inversé en minuscules (`dev.exemple.slack`).",
                "is no reversed domain name in lower case (`dev.exemple.slack`)."
            ),
            Code::Reserved => tr_in!(
                lang,
                "est réservé aux plugins d'Escouade (`dev.escouade.*`, `{{escouade}}`).",
                "is kept for Escouade's own plugins (`dev.escouade.*`, `{{escouade}}`)."
            ),
            Code::Version => tr_in!(
                lang,
                "n'est pas un numéro de version (`1.2.0`).",
                "is no version number (`1.2.0`)."
            ),
            Code::Api => tr_in!(
                lang,
                "n'est pas une version majeure de l'API des plugins (`\"1\"`).",
                "is no major version of the plugins' API (`\"1\"`)."
            ),
            Code::Range => tr_in!(
                lang,
                "est plus ancienne que `escouade.min`.",
                "is older than `escouade.min`."
            ),
            Code::Url => tr_in!(
                lang,
                "n'est pas une adresse en http:// ou https://.",
                "is no http:// or https:// address."
            ),
            Code::Path => tr_in!(
                lang,
                "sort du dossier du plugin (un chemin relatif, sans `..`).",
                "leaves the plugin's folder (a relative path, without `..`)."
            ),
            Code::Key => tr_in!(
                lang,
                "n'est pas un nom (une lettre, puis lettres, chiffres, `_`, `-` ou `.`).",
                "is no name (a letter, then letters, digits, `_`, `-` or `.`)."
            ),
            Code::Enum => tr_in!(lang, "vaut l'un de : {expected}.", "is one of: {expected}."),
            Code::Duplicate => tr_in!(lang, "est déjà pris plus haut.", "is already taken above."),
            Code::Account => tr_in!(
                lang,
                "n'est pas un compte de `contributes.accounts`.",
                "is no account of `contributes.accounts`."
            ),
            Code::File => tr_in!(
                lang,
                "n'est pas un fichier du dossier du plugin.",
                "is no file of the plugin's folder."
            ),
            Code::Png => tr_in!(
                lang,
                "n'est pas un PNG (ni SVG, ni JPEG : un PNG carré de 256 px au moins).",
                "is no PNG (no SVG nor JPEG: a square PNG of 256 px at least)."
            ),
            Code::Heavy => tr_in!(lang, "pèse plus de 1 Mo.", "weighs over 1 MB."),
            Code::Network => tr_in!(
                lang,
                "va vers un hôte que `permissions.network` ne déclare pas : l'utilisateur ne le verrait pas.",
                "goes to a host `permissions.network` does not declare: the user would not see it."
            ),
            Code::Command => tr_in!(
                lang,
                "doit commencer par le nom d'un programme, jamais un chemin, et `*` n'y vient qu'en dernier mot.",
                "must start with a program's name, never a path, and `*` comes only as its last word."
            ),
            Code::ReadOnly => tr_in!(
                lang,
                "ne peut pas être en écriture : le dossier du plugin ne change pas.",
                "cannot be written: the plugin's folder does not change."
            ),
            Code::Exclusive => tr_in!(
                lang,
                "ne va pas avec `run.command`, `run.js` ou `run.wasm` : un programme est une commande, un bundle JS/TS ou un module Rust, un seul des trois.",
                "does not go with `run.command`, `run.js` or `run.wasm`: a program is a command, a JS/TS bundle or a Rust module, one of the three."
            ),
            Code::Cargo => tr_in!(
                lang,
                "doit nommer le `Cargo.toml` du plugin, jamais un binaire.",
                "must name the plugin's `Cargo.toml`, never a binary."
            ),
            Code::Square => tr_in!(
                lang,
                "n'est pas carrée (un PNG carré de 256 px au moins, 512 conseillé).",
                "is not square (a square PNG of 256 px at least, 512 advised)."
            ),
            Code::Small => tr_in!(
                lang,
                "fait moins de 256 px de côté (512 conseillé).",
                "is under 256 px a side (512 advised)."
            ),
        };
        format!("{field} {what}")
    }
}

fn type_name(lang: Lang, expected: &str) -> String {
    match (expected, lang) {
        ("string", Lang::Fr) => "un texte".into(),
        ("string", Lang::En) => "a text".into(),
        ("object", Lang::Fr) => "un objet".into(),
        ("object", Lang::En) => "an object".into(),
        ("array", Lang::Fr) => "une liste".into(),
        ("array", Lang::En) => "a list".into(),
        ("boolean", Lang::Fr) => "true ou false".into(),
        ("boolean", Lang::En) => "true or false".into(),
        ("text", Lang::Fr) => "un texte, ou un objet de textes par langue".into(),
        ("text", Lang::En) => "a text, or an object of texts by language".into(),
        (other, _) => other.into(),
    }
}

/// A text of the manifest: one for every language, or one by language (`en` read first).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Text {
    One(String),
    ByLang(std::collections::BTreeMap<String, String>),
}

impl Text {
    /// The text in `lang`: its own, else the English one, else the French one.
    pub fn get(&self, lang: Lang) -> &str {
        match self {
            Text::One(s) => s,
            Text::ByLang(m) => {
                let own = match lang {
                    Lang::Fr => "fr",
                    Lang::En => "en",
                };
                [own, "en", "fr"]
                    .iter()
                    .find_map(|l| m.get(*l).filter(|s| !s.is_empty()))
                    .map(String::as_str)
                    .unwrap_or("")
            }
        }
    }
}

/// The Escouade a plugin was written for (`escouade`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    /// The major version of the protocol it speaks.
    pub api: u64,
    /// The oldest Escouade it was tried with.
    pub min: String,
    /// The newest, when it says (`2.4` takes every 2.4.x).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
}

/// Its program (`run`): none for a declarative plugin. A command (`command`, a process of its
/// own), a JS/TS bundle of its folder (`js`, run in WASM inside Escouade, ESC-320) or the Rust of a
/// module (`wasm`, ESC-331): one of the three; `command` is empty for the two in WASM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub command: String,
    /// The bundle (`dist/plugin.mjs`), a file of the plugin's folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub js: Option<String>,
    /// The Rust of its module (`Cargo.toml`), a file of the plugin's folder; what runs is its `plugin.wasm`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm: Option<String>,
    pub args: Vec<String>,
    /// The program of each system that has its own (`windows`, `macos`, `linux`).
    pub by_system: std::collections::BTreeMap<String, String>,
}

impl Run {
    /// What it runs in WASM, inside Escouade: its bundle, or the `Cargo.toml` of its module; none for a process.
    pub fn in_wasm(&self) -> Option<&str> {
        self.js.as_deref().or(self.wasm.as_deref())
    }
}

/// The folders a plugin may name (docs/PLUGINS.md §8.4.6), a closed list the host resolves: no path of its own, no `~`.
pub const FILE_ROOTS: &[&str] = &["plugin", "agent", "agentRepo", "agentHome", "temp"];
/// What it may do there.
pub const FILE_ACCESS: &[&str] = &["read", "write"];

/// A folder it asks for (`permissions.files`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileAccess {
    pub root: String,
    pub access: String,
}

impl FileAccess {
    /// Whether it may write there.
    pub fn writes(&self) -> bool {
        self.access == "write"
    }
    /// Its word in `asks`.
    pub fn word(&self) -> String {
        format!("{}:{}", self.root, self.access)
    }
}

/// What it asks for (`permissions`); a permission Escouade does not know is not given.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    pub events: Vec<String>,
    pub network: Vec<String>,
    pub secrets: Vec<String>,
    pub commands: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileAccess>,
    pub decide: bool,
}

/// What it asks for beyond the host's defaults (`limits`): the host holds them to its own ceilings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    /// How long a call of `net/fetch` may take, in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch_seconds: Option<u64>,
    /// How much its answer may hold, in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch_bytes: Option<u64>,
    /// How much `fs/read` gives at most.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_bytes: Option<u64>,
    /// How much linear memory its instance may take: for what is as big as a message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    /// How long a turn of its instance may take, in seconds: for what takes time to read whole.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_seconds: Option<u64>,
}

impl Limits {
    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A manifest that holds: what the app reads of a plugin without running anything.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub name: Text,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Text>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub escouade: Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<Run>,
    pub activation: Vec<String>,
    pub permissions: Permissions,
    /// What it asks for beyond the host's defaults.
    #[serde(skip_serializing_if = "Limits::is_empty")]
    pub limits: Limits,
    /// Its icon, a PNG of its folder (`icon_file` reads it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Its extension points Escouade knows, as written: their declarative parts (hosts, `locate`,
    /// capabilities) are read from here, without a process.
    pub contributes: Map<String, Value>,
}

impl Manifest {
    /// Whether it runs a program; without one, nothing of it runs.
    pub fn declarative(&self) -> bool {
        self.run.is_none()
    }

    /// Why this Escouade (`app`) does not load it, in `lang`: an API it does not serve, a version
    /// out of its range (as a harness's `oldest_reason`). None when it loads.
    pub fn refusal(&self, app: &str, lang: Lang) -> Option<String> {
        self.escouade.refusal(app, lang)
    }
}

impl Range {
    /// Why this Escouade (`app`) does not run a plugin of this range, in `lang`; None when it does.
    /// The store's catalogue picks with it the last version this Escouade can run (STORE-9).
    pub fn refusal(&self, app: &str, lang: Lang) -> Option<String> {
        let (api, min) = (self.api, &self.min);
        if api != API {
            return Some(tr_in!(
                lang,
                "Ce plugin parle la version {api} de l'API des plugins ; cette version d'Escouade sert la version {API}.",
                "This plugin speaks version {api} of the plugins' API; this version of Escouade serves version {API}."
            ));
        }
        let mine = version(app);
        if mine < version(min) {
            return Some(tr_in!(
                lang,
                "Ce plugin demande Escouade {min} ou plus récent ; tu as la {app}. Mets Escouade à jour.",
                "This plugin needs Escouade {min} or newer; you have {app}. Update Escouade."
            ));
        }
        if let Some(max) = &self.max {
            if beyond(&mine, max) {
                return Some(tr_in!(
                    lang,
                    "Ce plugin a été essayé jusqu'à Escouade {max} ; tu as la {app}. Cherche une version plus récente du plugin.",
                    "This plugin was tried up to Escouade {max}; you have {app}. Look for a newer version of the plugin."
                ));
            }
        }
        None
    }
}

/// Whether `v` is past `max`, compared on `max`'s own numbers (`2.4` takes every 2.4.x).
fn beyond(v: &Version, max: &str) -> bool {
    let len = max.split('.').count();
    let head: Vec<String> = v
        .to_string()
        .split('.')
        .take(len)
        .map(String::from)
        .collect();
    version(&head.join(".")) > version(max)
}

/// A version of Escouade, `2` as much as `2.1.0`.
pub fn version(s: &str) -> Version {
    if s.contains('.') {
        Version::parse(s)
    } else {
        Version::parse(&format!("{s}.0"))
    }
}

/// What the reader made of a manifest: the manifest when it holds, else every field that is
/// wrong; and the fields it ignored either way.
#[derive(Debug, Clone)]
pub struct Read {
    pub manifest: Result<Manifest, Vec<Problem>>,
    pub ignored: Vec<String>,
}

/// Reads the text of a manifest, for a plugin of `hand`.
pub fn read(text: &str, hand: Hand) -> Read {
    read_with(text, hand, None)
}

/// Reads the text of the manifest of the folder `dir`, and its icon's file there.
pub fn read_in(text: &str, hand: Hand, dir: &Path) -> Read {
    read_with(text, hand, Some(&|p: &str| icon_file(dir, p)))
}

fn read_with(text: &str, hand: Hand, file: Option<FileOf>) -> Read {
    match serde_json::from_str::<Value>(text) {
        Ok(v) => check_with(&v, hand, file),
        Err(_) => Read {
            manifest: Err(vec![Problem {
                path: String::new(),
                code: Code::Type,
                expected: Some("object".into()),
            }]),
            ignored: Vec::new(),
        },
    }
}

/// Checks a manifest's value, field by field; its icon's field, not its file.
pub fn check(v: &Value, hand: Hand) -> Read {
    check_with(v, hand, None)
}

/// The bytes of a file of a plugin's folder by its path (`icon_path`), None when it is none.
pub type FileOf<'a> = &'a dyn Fn(&str) -> Option<Vec<u8>>;

/// Checks a manifest's value, field by field, and its icon's file through `file`.
pub fn check_with(v: &Value, hand: Hand, file: Option<FileOf>) -> Read {
    let mut c = Checker::default();
    let manifest = c.manifest(v, hand, file);
    Read {
        manifest: match manifest {
            Some(m) if c.problems.is_empty() => Ok(m),
            _ => Err(c.problems),
        },
        ignored: c.ignored,
    }
}

#[derive(Default)]
struct Checker {
    problems: Vec<Problem>,
    ignored: Vec<String>,
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

fn is_lower_word(w: &str) -> bool {
    let b = w.as_bytes();
    !b.is_empty()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
        && b[0] != b'-'
        && b[b.len() - 1] != b'-'
}

/// The host of an address written whole (`https://hooks.slack.com/x` gives `hooks.slack.com`, in
/// lower case): its user and its port left out. None for no http(s) address, or a host made by a
/// template (`{{secrets.webhook}}`), which is checked once it is made.
pub fn written_host(url: &str) -> Option<String> {
    let rest = url
        .trim()
        .strip_prefix("https://")
        .or_else(|| url.trim().strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains("{{") || authority.contains("}}") {
        return None;
    }
    let host = authority.rsplit('@').next().unwrap_or("");
    let host = match host.strip_prefix('[') {
        // An IPv6 address, in its brackets.
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => host.split(':').next().unwrap_or(""),
    };
    (!host.is_empty()).then(|| host.trim_end_matches('.').to_ascii_lowercase())
}

/// Whether a line of `permissions.commands` holds: its first word is a program's name, never a path, and a `*` comes
/// only as its last word.
pub fn command_holds(command: &str) -> bool {
    let words: Vec<&str> = command.split_whitespace().collect();
    let Some((first, rest)) = words.split_first() else {
        return false;
    };
    let args = rest.len().saturating_sub(1);
    !first.is_empty()
        && !first.contains(['/', '\\', ':', '*', '~'])
        && !first.starts_with('.')
        && !rest[..args].contains(&"*")
}

/// Whether `permissions.network` covers `host`, without regard to case: a host written alone
/// covers itself; `*.exemple.dev` covers its subdomains, at any depth (`hooks.exemple.dev`,
/// `a.b.exemple.dev`), but not `exemple.dev` itself (as a TLS certificate's), which is
/// declared apart; nothing else covers anything.
pub fn host_covered(host: &str, network: &[String]) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    !host.is_empty()
        && network.iter().any(|p| {
            let p = p.trim().trim_end_matches('.').to_ascii_lowercase();
            match p.strip_prefix("*.") {
                Some(domain) => !domain.is_empty() && host.ends_with(&format!(".{domain}")),
                None => !p.is_empty() && !p.contains('*') && host == p,
            }
        })
}

/// A reversed domain name in lower case, of two words at least.
pub fn is_id(s: &str) -> bool {
    s.len() <= 100 && s.split('.').count() >= 2 && s.split('.').all(is_lower_word)
}

/// `1.2.0`, `1.2.0-beta.1`, `1.2.0+build`.
pub fn is_semver(s: &str) -> bool {
    let core = s.split(['-', '+']).next().unwrap_or("");
    let rest = &s[core.len()..];
    let nums: Vec<&str> = core.split('.').collect();
    nums.len() == 3
        && nums
            .iter()
            .all(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
        && rest
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-+".contains(&c))
        && rest.len() != 1
}

/// `2`, `2.1`, `2.1.0`: a version of Escouade in a range.
fn is_app_version(s: &str) -> bool {
    let nums: Vec<&str> = s.split('.').collect();
    nums.len() <= 3
        && nums
            .iter()
            .all(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
}

fn is_key(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b[0].is_ascii_alphabetic()
        && b.iter()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(c))
}

fn is_url(s: &str) -> bool {
    (s.starts_with("https://") && s.len() > 8) || (s.starts_with("http://") && s.len() > 7)
}

const PNG: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/// An icon's path as the folder's files are named: `/` between its parts, without `./` in front.
pub fn icon_path(s: &str) -> String {
    let mut p = s.replace('\\', "/");
    while let Some(rest) = p.strip_prefix("./") {
        p = rest.to_string();
    }
    p
}

/// What is wrong with an icon's bytes, from its header alone (nothing decodes it).
pub fn icon_problems(b: &[u8]) -> Vec<Code> {
    if b.len() < 24 || b[..8] != PNG || &b[12..16] != b"IHDR" {
        return vec![Code::Png];
    }
    let mut out = Vec::new();
    if b.len() as u64 > ICON_BYTES {
        out.push(Code::Heavy);
    }
    let at = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let (width, height) = (at(16), at(20));
    if width != height {
        out.push(Code::Square);
    }
    if width.min(height) < ICON_MIN {
        out.push(Code::Small);
    }
    out
}

/// The bytes of the file `rel` of the folder `dir`, never out of it: a relative path without `..`,
/// which a link does not take elsewhere either; a file, read up to `ICON_BYTES` and one byte more
/// (enough to say it is too heavy). None otherwise.
pub fn icon_file(dir: &Path, rel: &str) -> Option<Vec<u8>> {
    use std::io::Read as _;
    if !is_inner_path(rel) || rel.contains(':') {
        return None;
    }
    let root = std::fs::canonicalize(dir).ok()?;
    let path = std::fs::canonicalize(root.join(icon_path(rel))).ok()?;
    if !path.starts_with(&root) || !path.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .ok()?
        .take(ICON_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

/// A path inside the plugin's folder: relative, without `..`.
fn is_inner_path(s: &str) -> bool {
    let b = s.as_bytes();
    let drive = b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':';
    !s.is_empty()
        && !s.starts_with('/')
        && !s.starts_with('\\')
        && !drive
        && !s.split(['/', '\\']).any(|p| p == "..")
}

impl Checker {
    fn problem(&mut self, path: &str, code: Code) {
        self.problems.push(Problem {
            path: path.to_string(),
            code,
            expected: None,
        });
    }

    fn wrong_type(&mut self, path: &str, expected: &str) {
        self.problems.push(Problem {
            path: path.to_string(),
            code: Code::Type,
            expected: Some(expected.into()),
        });
    }

    /// The fields of `o` out of `known`, ignored.
    fn ignore_others(&mut self, o: &Map<String, Value>, path: &str, known: &[&str]) {
        for k in o.keys() {
            if !known.contains(&k.as_str()) {
                self.ignored.push(join(path, k));
            }
        }
    }

    fn object<'a>(&mut self, v: &'a Value, path: &str) -> Option<&'a Map<String, Value>> {
        let o = v.as_object();
        if o.is_none() {
            self.wrong_type(path, "object");
        }
        o
    }

    fn string<'a>(&mut self, v: &'a Value, path: &str) -> Option<&'a str> {
        let s = v.as_str();
        if s.is_none() {
            self.wrong_type(path, "string");
        }
        s
    }

    /// A string that is not empty.
    fn filled<'a>(&mut self, v: &'a Value, path: &str) -> Option<&'a str> {
        let s = self.string(v, path)?;
        if s.is_empty() {
            self.problem(path, Code::Empty);
            return None;
        }
        Some(s)
    }

    /// A list of strings that are not empty.
    fn words(&mut self, v: &Value, path: &str) -> Vec<String> {
        let Some(items) = v.as_array() else {
            self.wrong_type(path, "array");
            return Vec::new();
        };
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            if let Some(s) = self.filled(item, &format!("{path}[{i}]")) {
                out.push(s.to_string());
            }
        }
        out
    }

    fn text(&mut self, v: &Value, path: &str) -> Option<Text> {
        match v {
            Value::String(s) if s.is_empty() => {
                self.problem(path, Code::Empty);
                None
            }
            Value::String(s) => Some(Text::One(s.clone())),
            Value::Object(o) => {
                let mut by_lang = std::collections::BTreeMap::new();
                let before = self.problems.len();
                for (lang, t) in o {
                    match t.as_str() {
                        Some(s) => {
                            by_lang.insert(lang.clone(), s.to_string());
                        }
                        None => self.wrong_type(&join(path, lang), "string"),
                    }
                }
                let some = ["en", "fr"]
                    .iter()
                    .any(|l| by_lang.get(*l).is_some_and(|s| !s.is_empty()));
                if !some && self.problems.len() == before {
                    self.problem(path, Code::Empty);
                }
                (self.problems.len() == before).then_some(Text::ByLang(by_lang))
            }
            _ => {
                self.wrong_type(path, "text");
                None
            }
        }
    }

    fn url(&mut self, v: &Value, path: &str) -> Option<String> {
        let s = self.string(v, path)?;
        if !is_url(s) {
            self.problem(path, Code::Url);
            return None;
        }
        Some(s.to_string())
    }

    fn inner_path(&mut self, v: &Value, path: &str) -> Option<String> {
        let s = self.string(v, path)?;
        if !is_inner_path(s) {
            self.problem(path, Code::Path);
            return None;
        }
        Some(s.to_string())
    }

    /// A program of `run`: a path in the plugin's folder, or one of Escouade's (`{escouade}/…`)
    /// for a plugin of its own.
    fn program(&mut self, v: &Value, path: &str, builtin: bool) -> Option<String> {
        let s = self.string(v, path)?;
        if s.starts_with(ESCOUADE_DIR) {
            if !builtin {
                self.problem(path, Code::Reserved);
                return None;
            }
            return Some(s.to_string());
        }
        self.inner_path(v, path)
    }

    fn manifest(&mut self, v: &Value, hand: Hand, file: Option<FileOf>) -> Option<Manifest> {
        let builtin = hand == Hand::Builtin;
        let o = self.object(v, "")?;
        self.ignore_others(
            o,
            "",
            &[
                "$schema",
                "id",
                "name",
                "version",
                "description",
                "author",
                "license",
                "repository",
                "escouade",
                "run",
                "activation",
                "permissions",
                "limits",
                "contributes",
                "store",
                "icon",
            ],
        );
        if let Some(s) = o.get("$schema") {
            self.string(s, "$schema");
        }
        let id = match o.get("id") {
            None => {
                self.problem("id", Code::Required);
                None
            }
            Some(v) => self.string(v, "id").and_then(|s| {
                if !is_id(s) {
                    self.problem("id", Code::Id);
                    None
                } else if s.starts_with(RESERVED_PREFIX) && hand == Hand::Third {
                    self.problem("id", Code::Reserved);
                    None
                } else {
                    Some(s.to_string())
                }
            }),
        };
        let name = match o.get("name") {
            None => {
                self.problem("name", Code::Required);
                None
            }
            Some(v) => self.text(v, "name"),
        };
        let version = match o.get("version") {
            None => {
                self.problem("version", Code::Required);
                None
            }
            Some(v) => self.string(v, "version").and_then(|s| {
                if is_semver(s) {
                    Some(s.to_string())
                } else {
                    self.problem("version", Code::Version);
                    None
                }
            }),
        };
        let description = o
            .get("description")
            .and_then(|v| self.text(v, "description"));
        let author = o.get("author").and_then(|v| self.author(v));
        let license = o.get("license").and_then(|v| self.filled(v, "license"));
        let repository = o.get("repository").and_then(|v| self.url(v, "repository"));
        let escouade = match o.get("escouade") {
            None => {
                self.problem("escouade", Code::Required);
                None
            }
            Some(v) => self.range(v),
        };
        let run = o.get("run").and_then(|v| self.run(v, builtin, file));
        let activation = o
            .get("activation")
            .map(|v| self.words(v, "activation"))
            .unwrap_or_default();
        let permissions = o
            .get("permissions")
            .map(|v| self.permissions(v))
            .unwrap_or_default();
        let limits = o.get("limits").map(|v| self.limits(v)).unwrap_or_default();
        let contributes = o
            .get("contributes")
            .map(|v| self.contributes(v, &permissions.network))
            .unwrap_or_default();
        if let Some(v) = o.get("store") {
            self.store(v);
        }
        let icon = o.get("icon").and_then(|v| self.icon(v, file));
        Some(Manifest {
            id: id?,
            name: name?,
            version: version?,
            description,
            author: author.map(str::to_string),
            license: license.map(str::to_string),
            repository,
            escouade: escouade?,
            run,
            activation,
            permissions,
            limits,
            icon,
            contributes,
        })
    }

    fn limits(&mut self, v: &Value) -> Limits {
        let mut l = Limits::default();
        let Some(o) = self.object(v, "limits") else {
            return l;
        };
        self.ignore_others(
            o,
            "limits",
            &[
                "fetchSeconds",
                "fetchBytes",
                "fileBytes",
                "memoryBytes",
                "turnSeconds",
            ],
        );
        for (key, field) in [
            ("fetchSeconds", &mut l.fetch_seconds),
            ("fetchBytes", &mut l.fetch_bytes),
            ("fileBytes", &mut l.file_bytes),
            ("memoryBytes", &mut l.memory_bytes),
            ("turnSeconds", &mut l.turn_seconds),
        ] {
            if let Some(v) = o.get(key) {
                match v.as_u64().filter(|n| *n > 0) {
                    Some(n) => *field = Some(n),
                    None => self.wrong_type(&join("limits", key), "positive integer"),
                }
            }
        }
        l
    }

    /// A file of the folder: no address, no drive, no `..`; a PNG, never an SVG (it may carry a
    /// script); its header read through `file`, when there is one.
    fn icon(&mut self, v: &Value, file: Option<FileOf>) -> Option<String> {
        let s = self.string(v, "icon")?;
        if !is_inner_path(s) || s.contains(':') {
            self.problem("icon", Code::Path);
            return None;
        }
        if !s.to_ascii_lowercase().ends_with(".png") {
            self.problem("icon", Code::Png);
            return None;
        }
        if let Some(file) = file {
            let Some(bytes) = file(&icon_path(s)) else {
                self.problem("icon", Code::File);
                return None;
            };
            let problems = icon_problems(&bytes);
            if !problems.is_empty() {
                for code in problems {
                    self.problem("icon", code);
                }
                return None;
            }
        }
        Some(s.to_string())
    }

    fn author<'a>(&mut self, v: &'a Value) -> Option<&'a str> {
        let o = self.object(v, "author")?;
        self.ignore_others(o, "author", &["name", "url", "email"]);
        if let Some(u) = o.get("url") {
            self.url(u, "author.url");
        }
        if let Some(e) = o.get("email") {
            self.filled(e, "author.email");
        }
        match o.get("name") {
            None => {
                self.problem("author.name", Code::Required);
                None
            }
            Some(n) => self.filled(n, "author.name"),
        }
    }

    fn app_version(&mut self, o: &Map<String, Value>, key: &str) -> Option<String> {
        let path = join("escouade", key);
        let s = self.string(o.get(key)?, &path)?;
        if !is_app_version(s) {
            self.problem(&path, Code::Version);
            return None;
        }
        Some(s.to_string())
    }

    fn range(&mut self, v: &Value) -> Option<Range> {
        let o = self.object(v, "escouade")?;
        self.ignore_others(o, "escouade", &["api", "min", "max"]);
        let api = match o.get("api") {
            None => {
                self.problem("escouade.api", Code::Required);
                None
            }
            Some(a) => self.string(a, "escouade.api").and_then(|s| {
                let n = s
                    .bytes()
                    .all(|c| c.is_ascii_digit())
                    .then(|| s.parse::<u64>().ok())
                    .flatten()
                    .filter(|n| *n > 0 && !s.starts_with('0'));
                if n.is_none() {
                    self.problem("escouade.api", Code::Api);
                }
                n
            }),
        };
        if !o.contains_key("min") {
            self.problem("escouade.min", Code::Required);
        }
        let min = self.app_version(o, "min");
        let max = self.app_version(o, "max");
        if let (Some(min), Some(max)) = (&min, &max) {
            if beyond(&version(min), max) {
                self.problem("escouade.max", Code::Range);
                return None;
            }
        }
        Some(Range {
            api: api?,
            min: min?,
            max,
        })
    }

    fn run(&mut self, v: &Value, builtin: bool, file: Option<FileOf>) -> Option<Run> {
        let o = self.object(v, "run")?;
        let systems = ["windows", "macos", "linux"];
        self.ignore_others(
            o,
            "run",
            &["command", "js", "wasm", "args", "windows", "macos", "linux"],
        );
        // A bundle (ESC-320) or the Rust of a module (ESC-331): a file of the folder, one of the three kinds of program.
        let in_wasm = o.contains_key("js") || o.contains_key("wasm");
        let both = o.contains_key("wasm") && o.contains_key("js");
        let clash = (in_wasm && o.contains_key("command")) || both;
        if clash {
            self.problem(
                if both || !o.contains_key("js") {
                    "run.wasm"
                } else {
                    "run.js"
                },
                Code::Exclusive,
            );
        }
        let inner = |me: &mut Self, key: &str, cargo: bool| {
            let at = join("run", key);
            me.inner_path(o.get(key)?, &at).filter(|path| {
                if cargo && icon_path(path).rsplit('/').next() != Some("Cargo.toml") {
                    me.problem(&at, Code::Cargo);
                    return false;
                }
                // The app gives a module, not its sources.
                let found =
                    (cargo && builtin) || file.is_none_or(|file| file(&icon_path(path)).is_some());
                if !found {
                    me.problem(&at, Code::File);
                }
                found
            })
        };
        let js = if clash {
            None
        } else {
            inner(self, "js", false)
        };
        let wasm = if clash {
            None
        } else {
            inner(self, "wasm", true)
        };
        let command = match (o.get("command"), in_wasm) {
            _ if clash => None,
            (None, true) => js.as_ref().or(wasm.as_ref()).map(|_| String::new()),
            (None, false) => {
                self.problem("run.command", Code::Required);
                None
            }
            (Some(c), _) => self.program(c, "run.command", builtin),
        };
        let args = o
            .get("args")
            .map(|a| self.words(a, "run.args"))
            .unwrap_or_default();
        let mut by_system = std::collections::BTreeMap::new();
        for sys in systems {
            if let Some(p) = o.get(sys) {
                if let Some(p) = self.program(p, &join("run", sys), builtin) {
                    by_system.insert(sys.to_string(), p);
                }
            }
        }
        Some(Run {
            command: command?,
            js,
            wasm,
            args,
            by_system,
        })
    }

    fn permissions(&mut self, v: &Value) -> Permissions {
        let mut p = Permissions::default();
        let Some(o) = self.object(v, "permissions") else {
            return p;
        };
        self.ignore_others(
            o,
            "permissions",
            &[
                "events", "network", "secrets", "commands", "files", "decide",
            ],
        );
        for (key, list) in [
            ("events", &mut p.events),
            ("network", &mut p.network),
            ("secrets", &mut p.secrets),
            ("commands", &mut p.commands),
        ] {
            if let Some(v) = o.get(key) {
                *list = self.words(v, &join("permissions", key));
            }
        }
        for (i, command) in p.commands.clone().iter().enumerate() {
            if !command_holds(command) {
                self.problem(&format!("permissions.commands[{i}]"), Code::Command);
            }
        }
        if let Some(v) = o.get("files") {
            p.files = self.files(v);
        }
        if let Some(d) = o.get("decide") {
            match d.as_bool() {
                Some(b) => p.decide = b,
                None => self.wrong_type("permissions.decide", "boolean"),
            }
        }
        p
    }

    /// `permissions.files`: a root of the closed list and what it may do there; each once.
    fn files(&mut self, v: &Value) -> Vec<FileAccess> {
        let path = "permissions.files";
        let Some(items) = v.as_array() else {
            self.wrong_type(path, "array");
            return Vec::new();
        };
        let mut out: Vec<FileAccess> = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let at = format!("{path}[{i}]");
            let Some(o) = self.object(item, &at) else {
                continue;
            };
            self.ignore_others(o, &at, &["root", "access"]);
            let mut pick = |key: &str, among: &[&str]| -> Option<String> {
                let at = join(&at, key);
                let Some(v) = o.get(key) else {
                    self.problem(&at, Code::Required);
                    return None;
                };
                let s = self.string(v, &at)?;
                if !among.contains(&s) {
                    self.problems.push(Problem {
                        path: at,
                        code: Code::Enum,
                        expected: Some(among.join(", ")),
                    });
                    return None;
                }
                Some(s.to_string())
            };
            let (root, access) = (pick("root", FILE_ROOTS), pick("access", FILE_ACCESS));
            let (Some(root), Some(access)) = (root, access) else {
                continue;
            };
            let file = FileAccess { root, access };
            if file.root == "plugin" && file.writes() {
                self.problem(&join(&at, "access"), Code::ReadOnly);
            } else if out.iter().any(|f| f.root == file.root) {
                self.problem(&join(&at, "root"), Code::Duplicate);
            } else {
                out.push(file);
            }
        }
        out
    }

    /// The objects of the list of an extension point.
    fn items<'a>(&mut self, v: &'a Value, path: &str) -> Vec<(String, &'a Map<String, Value>)> {
        let Some(items) = v.as_array() else {
            self.wrong_type(path, "array");
            return Vec::new();
        };
        items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let at = format!("{path}[{i}]");
                self.object(item, &at).map(|o| (at, o))
            })
            .collect()
    }

    /// The `key` (or `id`) of each item: a name, once in the list. Gives the good ones.
    fn keys(&mut self, items: &[(String, &Map<String, Value>)], field: &str) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        let mut good = Vec::new();
        for (at, o) in items {
            let path = join(at, field);
            match o.get(field) {
                None => self.problem(&path, Code::Required),
                Some(k) => {
                    if let Some(k) = self.string(k, &path) {
                        if seen.iter().any(|s| s == k) {
                            self.problem(&path, Code::Duplicate);
                        }
                        seen.push(k.to_string());
                        if is_key(k) {
                            good.push(k.to_string());
                        } else {
                            self.problem(&path, Code::Key);
                        }
                    }
                }
            }
        }
        good
    }

    fn titled(&mut self, items: &[(String, &Map<String, Value>)], field: &str, required: bool) {
        for (at, o) in items {
            let path = join(at, field);
            match o.get(field) {
                None if required => self.problem(&path, Code::Required),
                None => {}
                Some(t) => {
                    self.text(t, &path);
                }
            }
        }
    }

    fn contributes(&mut self, v: &Value, network: &[String]) -> Map<String, Value> {
        let mut kept = Map::new();
        let Some(o) = self.object(v, "contributes") else {
            return kept;
        };
        let mut known: Vec<&str> = vec!["settings", "secrets", "escouadeYml"];
        known.extend(POINTS_WITH_ID);
        known.extend(OPEN_POINTS);
        self.ignore_others(o, "contributes", &known);
        let path = |k: &str| join("contributes", k);
        if let Some(v) = o.get("settings") {
            let items = self.items(v, &path("settings"));
            self.keys(&items, "key");
            for (at, item) in &items {
                let p = join(at, "type");
                match item.get("type") {
                    None => self.problem(&p, Code::Required),
                    Some(t) => {
                        if let Some(t) = self.string(t, &p) {
                            if !SETTING_TYPES.contains(&t) {
                                self.problems.push(Problem {
                                    path: p,
                                    code: Code::Enum,
                                    expected: Some(SETTING_TYPES.join(", ")),
                                });
                            }
                        }
                    }
                }
            }
            self.titled(&items, "title", true);
        }
        if let Some(v) = o.get("secrets") {
            let items = self.items(v, &path("secrets"));
            self.keys(&items, "key");
            self.titled(&items, "title", true);
        }
        let mut accounts = Vec::new();
        for point in POINTS_WITH_ID {
            let Some(v) = o.get(point) else { continue };
            let items = self.items(v, &path(point));
            let ids = self.keys(&items, "id");
            self.titled(&items, "name", false);
            if point == "accounts" {
                accounts = ids;
                continue;
            }
            // A workflow reader says which files of the project it is given (ESC-200).
            if point == "workflowReaders" {
                for (at, item) in &items {
                    let p = join(at, "files");
                    match item.get("files") {
                        None => self.problem(&p, Code::Required),
                        Some(Value::Array(list)) => {
                            for (i, f) in list.iter().enumerate() {
                                let at = format!("{p}[{i}]");
                                if let Some(f) = self.filled(f, &at) {
                                    if !is_inner_path(f) {
                                        self.problem(&at, Code::Path);
                                    }
                                }
                            }
                        }
                        Some(_) => self.wrong_type(&p, "array"),
                    }
                }
            }
            for (at, item) in &items {
                let p = join(at, "account");
                if let Some(a) = item.get("account") {
                    if let Some(a) = self.string(a, &p) {
                        if !accounts.iter().any(|x| x == a) {
                            self.problem(&p, Code::Account);
                        }
                    }
                }
            }
        }
        for point in OPEN_POINTS {
            if let Some(v) = o.get(point) {
                let items = self.items(v, &path(point));
                if point == "webhooks" {
                    // A webhook goes only to a host the install window shows (`permissions.network`);
                    // one whose host is a template is checked when it is sent.
                    for (at, item) in &items {
                        let url = item.get("url").and_then(Value::as_str).unwrap_or("");
                        if let Some(host) = written_host(url) {
                            if !host_covered(&host, network) {
                                self.problem(&join(at, "url"), Code::Network);
                            }
                        }
                    }
                }
            }
        }
        if let Some(v) = o.get("escouadeYml") {
            self.object(v, &path("escouadeYml"));
        }
        for k in known {
            if let Some(v) = o.get(k) {
                kept.insert(k.to_string(), v.clone());
            }
        }
        kept
    }

    fn store(&mut self, v: &Value) {
        let Some(o) = self.object(v, "store") else {
            return;
        };
        self.ignore_others(
            o,
            "store",
            &["categories", "keywords", "screenshots", "docs"],
        );
        for key in ["categories", "keywords"] {
            if let Some(v) = o.get(key) {
                self.words(v, &join("store", key));
            }
        }
        if let Some(v) = o.get("screenshots") {
            match v.as_array() {
                Some(items) => {
                    for (i, item) in items.iter().enumerate() {
                        self.inner_path(item, &format!("store.screenshots[{i}]"));
                    }
                }
                None => self.wrong_type("store.screenshots", "array"),
            }
        }
        if let Some(v) = o.get("docs") {
            self.inner_path(v, "store.docs");
        }
    }
}

/// The JSON schema of the manifest (`schemas/plugin-1.json`), for the editors' completion: the
/// reader's rules, as far as a schema says them. Unknown fields are allowed (they are ignored).
pub fn schema() -> Value {
    let text = json!({
        "description": "One text, or a text by language (`en` first, then `fr`).",
        "oneOf": [
            { "type": "string", "minLength": 1 },
            { "type": "object", "additionalProperties": { "type": "string" }, "anyOf": [
                { "required": ["en"] }, { "required": ["fr"] }
            ] },
        ],
    });
    let inner_path = json!({
        "type": "string",
        "minLength": 1,
        "pattern": "^(?![/\\\\]|[A-Za-z]:)(?!(.*[/\\\\])?\\.\\.([/\\\\]|$)).*$",
        "description": "A path in the plugin's folder: relative, without `..`.",
    });
    let words = json!({ "type": "array", "items": { "type": "string", "minLength": 1 } });
    let key = json!({ "type": "string", "pattern": "^[A-Za-z][A-Za-z0-9_.-]*$" });
    let app_version = json!({ "type": "string", "pattern": "^[0-9]+(\\.[0-9]+){0,2}$" });
    let url = json!({ "type": "string", "pattern": "^https?://." });
    let with_id = |about: &str, account: bool| {
        let mut props = json!({ "id": key, "name": text });
        if account {
            props["account"] = json!({
                "type": "string",
                "description": "The id of one of `contributes.accounts`.",
            });
        }
        json!({
            "type": "array",
            "description": about,
            "items": { "type": "object", "required": ["id"], "properties": props },
        })
    };
    // A reader of workflows: the files of the project it is given, as globs from its folder.
    let mut readers = with_id(
        "Readers of workflows Escouade does not read by itself (it gives them the project's files).",
        false,
    );
    readers["items"]["required"] = json!(["id", "files"]);
    readers["items"]["properties"]["files"] = json!({
        "type": "array",
        "description": "The files of the project it is given, as globs from the project's folder (`.planning/**`).",
        "items": { "type": "string", "minLength": 1 },
    });
    let open = |about: &str| json!({ "type": "array", "description": about, "items": { "type": "object" } });
    // Apart: the macro of the whole schema would reach its limit of recursion.
    let commands = json!({
        "type": "array",
        "description": "The programs it may run (`proc/spawn`): a program's name, never a path, then its arguments, word for word. A last word `*` lets any more follow.",
        "items": { "type": "string", "minLength": 1 },
    });
    let files = json!({
        "type": "array",
        "description": "The folders it may read or write (`fs/*`): a root of the closed list, which the host resolves; no path of its own. `plugin` is read only.",
        "items": {
            "type": "object",
            "required": ["root", "access"],
            "properties": {
                "root": { "enum": FILE_ROOTS },
                "access": { "enum": FILE_ACCESS },
            },
        },
    });
    let permissions = json!({
        "type": "object",
        "description": "What it asks for, shown before it is installed.",
        "properties": {
            "events": words,
            "network": words,
            "secrets": words,
            "commands": commands,
            "files": files,
            "decide": { "type": "boolean" },
        },
    });
    let contributes = json!({
        "type": "object",
        "description": "Its extension points.",
        "properties": {
            "settings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["key", "type", "title"],
                    "properties": { "key": key, "type": { "enum": SETTING_TYPES }, "title": text },
                },
            },
            "secrets": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["key", "title"],
                    "properties": { "key": key, "title": text },
                },
            },
            "accounts": with_id("A kind of account its points share.", false),
            "ticketSources": with_id("Sources of the Kanban.", true),
            "forges": with_id("Code hosts: pull requests, CI, deployments.", true),
            "harnesses": with_id("Harnesses.", true),
            "launchEnvironments": with_id("Launch environments: the up, ls, down and destroy of a worktree.", false),
            "workflowReaders": readers,
            "themes": open("Themes."),
            "webhooks": open("Webhooks, without code."),
            "commands": open("Commands run on an event."),
            "menus": open("Menus."),
            "statusBar": open("Items of the status bar."),
            "badges": open("Badges of the cards."),
            "projectTemplates": open("Templates of the project wizard."),
            "escouadeYml": { "type": "object", "description": "Its keys of .escouade.yml." },
        },
    });
    let limits = json!({
        "type": "object",
        "description": "What a plugin in WASM asks for beyond the host's defaults; Escouade caps them at 120 s and 32 MiB for `net/fetch`, 4 MiB for `fs/read`, 256 MiB for its memory, 10 s for a turn.",
        "properties": {
            "fetchSeconds": { "type": "integer", "minimum": 1 },
            "fetchBytes": { "type": "integer", "minimum": 1 },
            "fileBytes": {
                "type": "integer",
                "minimum": 1,
                "maximum": 4194304,
                "description": "How much `fs/read` gives at most: 256 KiB without it.",
            },
            "memoryBytes": {
                "type": "integer",
                "minimum": 1,
                "maximum": 268435456,
                "description": "How much linear memory its instance may take: 64 MiB without it.",
            },
            "turnSeconds": {
                "type": "integer",
                "minimum": 1,
                "maximum": 10,
                "description": "How long a turn of its instance may take: 2 s without it.",
            },
        },
    });
    let mut schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": SCHEMA_URL,
        "title": "The manifest of an Escouade plugin (escouade-plugin.json)",
        "type": "object",
        "required": ["id", "name", "version", "escouade"],
        "properties": {
            "$schema": { "type": "string" },
            "id": {
                "type": "string",
                "maxLength": 100,
                "pattern": "^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$",
                "description": "Unique, a reversed domain name (`dev.exemple.slack`); `dev.escouade.*` is kept for Escouade's own plugins.",
            },
            "name": text,
            "version": {
                "type": "string",
                "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+([-+][0-9A-Za-z.+-]+)?$",
                "description": "Its version (`1.2.0`), the same as its git tag.",
            },
            "description": text,
            "author": {
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": { "type": "string", "minLength": 1 },
                    "url": url,
                    "email": { "type": "string", "minLength": 1 },
                },
            },
            "license": { "type": "string", "minLength": 1, "description": "An SPDX identifier (`MIT`)." },
            "repository": url,
            "escouade": {
                "type": "object",
                "required": ["api", "min"],
                "description": "The Escouade it was written for: out of it, the plugin is disabled, and the reason shown.",
                "properties": {
                    "api": { "type": "string", "pattern": "^[1-9][0-9]*$", "description": "The major version of the plugins' protocol it speaks (`\"1\"`)." },
                    "min": app_version,
                    "max": app_version,
                },
            },
            "run": {
                "type": "object",
                "description": "Its program; without one, the plugin is declarative: nothing runs. A command (`command`, a process), a JS/TS bundle of its folder that Escouade runs in WASM (`js`), or the `Cargo.toml` of a Rust module that Escouade runs in WASM (`wasm`): one of the three.",
                "properties": {
                    "command": inner_path,
                    "js": inner_path,
                    "wasm": inner_path,
                    "args": words,
                    "windows": inner_path,
                    "macos": inner_path,
                    "linux": inner_path,
                },
            },
            "activation": {
                "type": "array",
                "description": "What starts it (`event:ticket.column`, `account:github`); without any, it does not run.",
                "items": { "type": "string", "minLength": 1 },
            },
            "permissions": permissions,
            "contributes": contributes,
            "icon": {
                "type": "string",
                "minLength": 1,
                "pattern": "^(?![/\\\\]|[A-Za-z]:)(?!(.*[/\\\\])?\\.\\.([/\\\\]|$))[^:]*\\.[Pp][Nn][Gg]$",
                "description": "Its logo: a square PNG of its folder, 256 px a side at least (512 advised), 1 MB at most; no SVG.",
            },
            "store": {
                "type": "object",
                "description": "What store.escouade.dev shows of it.",
                "properties": {
                    "categories": words,
                    "keywords": words,
                    "screenshots": { "type": "array", "items": inner_path },
                    "docs": inner_path,
                },
            },
        },
    });
    // Beside it: one key more and `json!` runs out of its recursion.
    schema["properties"]["limits"] = limits;
    schema
}
