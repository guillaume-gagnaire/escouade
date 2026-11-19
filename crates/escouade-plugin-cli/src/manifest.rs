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
/// The ids of Escouade's own plugins: those of its repository's `plugins/<id>/` (`Hand::First`, ESC-314).
pub const RESERVED_PREFIX: &str = "dev.escouade.";

/// Whose hand a manifest is read for: what it may take of Escouade's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hand {
    /// Anyone's plugin: not the ids `dev.escouade.*`.
    #[default]
    Third,
    /// A plugin of Escouade's own hand (`plugins/<id>/` of its repository, ESC-314): a version of the store signed for
    /// it, or its folder in development. It may take the ids `dev.escouade.*`.
    First,
}

/// Why a plugin whose `run` is a command does not start (ESC-355, docs/PLUGINS-DESIGN.md §8.8): Escouade runs a plugin in WASM
/// alone, whatever its origin; the manifest still reads, and the plugin's sheet says this.
pub fn command_refused(lang: Lang) -> String {
    tr_in!(
        lang,
        "Escouade ne lance plus `run.command` : un plugin se publie en `run.js` ou `run.wasm`, que le store compile pour tous les systèmes.",
        "Escouade no longer starts `run.command`: a plugin is published as `run.js` or `run.wasm`, which the store compiles for every system."
    )
}
/// The side an icon has at least, in pixels (512 advised).
pub const ICON_MIN: u32 = 256;
/// The weight an icon has at most, in bytes.
pub const ICON_BYTES: u64 = 1024 * 1024;

/// The values `contributes.settings[].type` takes: `profile` names a kind of `contributes.profiles` (`of`, ESC-525).
const SETTING_TYPES: [&str; 5] = ["string", "number", "boolean", "select", "profile"];
/// The values a field of a profile takes as its `type` (a secret field has none: `secret: true`).
const FIELD_TYPES: [&str; 4] = ["string", "number", "boolean", "select"];
/// Where a setting or a secret holds (`scope`, docs/PLUGINS-CONTRACT-STUDY.md §2): for every project (`app`, the
/// default) or for each one (`project`).
pub const SCOPES: [&str; 2] = ["app", "project"];
/// How many lines a field on several lines shows (`lines`): 6 without it.
pub const LINES: std::ops::RangeInclusive<u64> = 3..=20;
/// How many programs a plugin may run at once (`limits.processes`): 4 without it, 16 at most (§5.6).
pub const PROCESSES: std::ops::RangeInclusive<u64> = 1..=16;
/// The programs at once a plugin has without asking: beyond, `limits.reason` is needed, the install says it and an
/// update that raises it asks again (ESC-529).
pub const PROCESSES_UNASKED: u64 = 4;
/// The words of `permissions.commands` that stand for one argument of a closed form (§5.3): each a whole word, never the
/// first. The host checks and replaces them when the program is run.
pub const JOKERS: [&str; 6] = [
    "{int}",
    "{port}",
    "{name}",
    "{pid}",
    "{path:temp}",
    "{path:plugin}",
];
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
/// The points the manifest reserves: read for their shape, served by nothing, so said by `validate`
/// as « reserved, no effect in this version »: a warning, never a refusal, for a published manifest
/// to hold (ESC-524).
pub const RESERVED_POINTS: [&str; 5] = [
    "menus",
    "statusBar",
    "badges",
    "projectTemplates",
    "escouadeYml",
];

/// What `validate` says of a reserved point at `path` (`contributes.statusBar`), in `lang`.
pub fn reserved_point(path: &str, lang: Lang) -> String {
    tr_in!(
        lang,
        "`{path}` est réservé, sans effet dans cette version : Escouade n'en montre ni n'en appelle rien.",
        "`{path}` is reserved, with no effect in this version: Escouade shows and calls nothing of it."
    )
}

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
    /// A variable that is no name, or one Escouade refuses a plugin even declared (`env_refused`, ESC-424).
    Variable,
    /// A harness of anyone's that names a shell or an interpreter as its program (`interpreter`, ESC-424).
    Interpreter,
    /// What a harness described by ACP reads of its messages that is no JSON pointer (`/…`, ESC-491).
    Pointer,
    /// The arguments of `acp.resume` that do not name the session to resume (`{session}`, ESC-491).
    Session,
    /// A field that does not go where it is (ESC-525): `multiline` or `import` out of a `string` setting or a secret,
    /// `lines` without `multiline`, `of` out of a `profile` setting, a `type` or a `scope` on a field of a profile
    /// that does not take one.
    Misplaced,
    /// `repo: true` on what never comes from .escouade.yml: a secret, a profile, a value imported from a file.
    Repo,
    /// A whole number out of `Problem::expected` (`3..20`).
    Bounds,
    /// An item of `import` that is no extension of a file (`.ovpn`).
    Extension,
    /// A key that names no field fit for it: in `groups[].fields`, a setting or a secret the plugin declares; in
    /// `nameField`, a `string` field of the profile, neither secret nor on several lines.
    Field,
    /// A kind of profile `contributes.profiles` does not declare (`of`), or does without a secret field
    /// (`permissions.secrets`: `profile:<kind>`).
    Profile,
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
                "est réservé aux plugins d'Escouade (`dev.escouade.*`).",
                "is kept for Escouade's own plugins (`dev.escouade.*`)."
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
                "doit commencer par le nom d'un programme, jamais un chemin ni un joker ; un joker (`{{int}}`, `{{port}}`, `{{name}}`, `{{pid}}`, `{{path:temp}}`, `{{path:plugin}}`) est un mot entier, aucune autre accolade, et `*` n'y vient qu'en dernier mot.",
                "must start with a program's name, never a path nor a joker; a joker (`{{int}}`, `{{port}}`, `{{name}}`, `{{pid}}`, `{{path:temp}}`, `{{path:plugin}}`) is a whole word, no other brace, and `*` comes only as its last word."
            ),
            Code::Variable => tr_in!(
                lang,
                "n'est pas une variable qu'un plugin peut poser : un nom (lettres, chiffres, `_`), hors des variables d'Escouade et des familles qui changeraient ce que lance le programme (`LD_*`, `GIT_*`, `PYTHON*`, `*_PROXY`, `HOME`, `PATH`…).",
                "is no variable a plugin may set: a name (letters, digits, `_`), out of Escouade's own and of the families that would change what the program runs (`LD_*`, `GIT_*`, `PYTHON*`, `*_PROXY`, `HOME`, `PATH`…)."
            ),
            Code::Interpreter => tr_in!(
                lang,
                "nomme un shell ou un interpréteur (`bash`, `powershell`, `python`, `node`…) : le harness d'un tiers lance un programme à lui.",
                "names a shell or an interpreter (`bash`, `powershell`, `python`, `node`…): the harness of anyone's runs a program of its own."
            ),
            Code::Pointer => tr_in!(
                lang,
                "n'est pas un pointeur JSON (`/_meta/<agent>/…`) : ce qu'Escouade lit d'un message du harness.",
                "is no JSON pointer (`/_meta/<agent>/…`): what Escouade reads of a message of the harness."
            ),
            Code::Session => tr_in!(
                lang,
                "ne nomme pas la session à reprendre : un argument doit contenir `{{session}}`.",
                "does not name the session to resume: an argument must hold `{{session}}`."
            ),
            Code::Misplaced => tr_in!(
                lang,
                "n'a pas sa place ici : `multiline` et `import` vont sur un réglage `string` ou un secret, `lines` avec `multiline: true`, `of` sur un réglage `profile` ; un champ de profil n'a pas de `scope`, ni de `type` s'il est secret.",
                "does not belong here: `multiline` and `import` go on a `string` setting or a secret, `lines` with `multiline: true`, `of` on a `profile` setting; a field of a profile has no `scope`, nor a `type` when it is secret."
            ),
            Code::Repo => tr_in!(
                lang,
                "ne peut pas être vrai : un secret, un profil ou une valeur importée d'un fichier ne vient jamais de .escouade.yml.",
                "cannot be true: a secret, a profile or a value imported from a file never comes from .escouade.yml."
            ),
            Code::Bounds => {
                let (lo, hi) = expected.split_once("..").unwrap_or((expected, expected));
                tr_in!(
                    lang,
                    "doit être un nombre entier de {lo} à {hi}.",
                    "must be a whole number from {lo} to {hi}."
                )
            }
            Code::Extension => tr_in!(
                lang,
                "n'est pas une extension de fichier (`.ovpn`, `.conf`).",
                "is no extension of a file (`.ovpn`, `.conf`)."
            ),
            Code::Field => tr_in!(
                lang,
                "ne nomme pas un champ qui convient : dans un groupe, un réglage ou un secret du plugin ; en `nameField`, un champ `string` du profil, ni secret ni sur plusieurs lignes.",
                "names no field fit for it: in a group, a setting or a secret of the plugin; as `nameField`, a `string` field of the profile, neither secret nor on several lines."
            ),
            Code::Profile => tr_in!(
                lang,
                "n'est pas un genre de `contributes.profiles` (qui a un champ secret, pour `profile:<genre>`).",
                "is no kind of `contributes.profiles` (one with a secret field, for `profile:<kind>`)."
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

/// The folders a plugin may name (docs/PLUGINS-DESIGN.md §8.4.6), a closed list the host resolves: no path of its own, no `~`.
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
    /// The variables it may set on what it runs (`proc/spawn`, the launches of its harnesses), shown before it is
    /// installed (ESC-424). None when its manifest has no `env`: nothing for anyone's plugin; for one of Escouade's own
    /// hand, of a version from before the field, its launches as they were.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileAccess>,
    pub decide: bool,
}

impl Permissions {
    /// Whether the plugin may set the variable `name` (as written, its case kept): one of `permissions.env`, never one
    /// of `env_refused` for anyone's plugin. A plugin of Escouade's own hand without `env` keeps what it did.
    pub fn env_allowed(&self, name: &str, hand: Hand) -> bool {
        env_allowed(self.env.as_deref(), name, hand)
    }
}

/// Whether a plugin of `hand` that declares `declared` (`permissions.env`) may set the variable `name`.
pub fn env_allowed(declared: Option<&[String]>, name: &str, hand: Hand) -> bool {
    match declared {
        Some(declared) => declared.iter().any(|d| d == name) && !env_refused(name, hand),
        None => hand == Hand::First && is_var_name(name),
    }
}

/// A name of variable: a letter or `_`, then letters, digits and `_`.
pub fn is_var_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// The variables of `GIT_*` a plugin may set: who writes a commit, which loads nothing.
const GIT_SAFE: [&str; 2] = ["GIT_AUTHOR_", "GIT_COMMITTER_"];

/// Whether Escouade refuses the variable `name` to a plugin of `hand`, even declared (ESC-424): no name; for anyone's
/// plugin, Escouade's own (`ESCOUADE_*`) and the families by which a program loads other code, another configuration,
/// another certificate or proxy, or finds its programs and its home elsewhere. The core itself sets what a harness needs
/// of them. A plugin of Escouade's own hand is held to what it declares alone.
pub fn env_refused(name: &str, hand: Hand) -> bool {
    if !is_var_name(name) {
        return true;
    }
    if hand == Hand::First {
        return false;
    }
    let u = name.to_ascii_uppercase();
    let starts = |p: &str| u.starts_with(p);
    starts("ESCOUADE_")
        || starts("LD_")
        || starts("DYLD_")
        || (starts("GIT_") && !GIT_SAFE.iter().any(|p| starts(p)))
        || starts("PYTHON")
        || starts("RUBY")
        || starts("PERL")
        || starts("NODE_")
        || starts("NPM_CONFIG_")
        || starts("SSL_")
        || starts("XDG_")
        || u.ends_with("_PROXY")
        || matches!(
            u.as_str(),
            "JAVA_TOOL_OPTIONS"
                | "_JAVA_OPTIONS"
                | "JDK_JAVA_OPTIONS"
                | "HOME"
                | "USERPROFILE"
                | "APPDATA"
                | "LOCALAPPDATA"
                | "PATH"
                | "PATHEXT"
                | "COMSPEC"
                | "SHELL"
                | "BASH_ENV"
                | "ENV"
                | "ZDOTDIR"
                | "IFS"
                | "PROMPT_COMMAND"
                | "EDITOR"
                | "VISUAL"
                | "PAGER"
                | "BROWSER"
                | "CURL_CA_BUNDLE"
                | "REQUESTS_CA_BUNDLE"
                | "CODEX_CA_CERTIFICATE"
        )
}

/// The shells and interpreters a harness of anyone's may not name as its program (ESC-424): each would run what its
/// arguments say. Compared without case nor `.exe`, and with what follows `python`, `pwsh`… (`python3.12`).
pub fn interpreter(program: &str) -> bool {
    let p = program.to_ascii_lowercase();
    let p = p
        .strip_suffix(".exe")
        .or_else(|| p.strip_suffix(".com"))
        .unwrap_or(&p);
    const EXACT: [&str; 28] = [
        "sh",
        "bash",
        "zsh",
        "dash",
        "ksh",
        "csh",
        "tcsh",
        "fish",
        "ash",
        "busybox",
        "cmd",
        "powershell",
        "pwsh",
        "node",
        "nodejs",
        "deno",
        "bun",
        "ruby",
        "perl",
        "php",
        "lua",
        "osascript",
        "wscript",
        "cscript",
        "mshta",
        "rundll32",
        "env",
        "wsl",
    ];
    const FAMILIES: [&str; 6] = ["python", "pypy", "perl", "ruby", "pwsh", "powershell"];
    EXACT.contains(&p)
        || FAMILIES.iter().any(|f| {
            p.strip_prefix(f).is_some_and(|rest| {
                rest.bytes()
                    .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-')
            })
        })
}

/// What it asks for beyond the host's defaults (`limits`): the host holds them to its own ceilings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
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
    /// How many programs it may run at once (`PROCESSES`, §5.6): 4 without it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processes: Option<u64>,
    /// Why it asks for them, said at its install and judged at its review: needed beyond 4.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Text>,
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

    /// What it declares of the contract (docs/PLUGINS-CONTRACT-STUDY.md) that this Escouade reads but does not serve
    /// yet, in `lang`: a secret of each project (direction A of the study, not chosen). The jokers of a command are served
    /// since ESC-528, the programs at once of `limits.processes` since ESC-529, the profiles since ESC-530. Such a plugin is
    /// loaded disabled with this reason, never served by half (ESC-525). None when all it asks is served.
    pub fn unserved(&self, lang: Lang) -> Option<String> {
        let mut points = Vec::new();
        let listed = |key: &str| {
            self.contributes
                .get(key)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        if listed("secrets")
            .iter()
            .any(|s| s.get("scope").and_then(Value::as_str) == Some("project"))
        {
            points.push(tr_in!(
                lang,
                "un secret propre à chaque projet (`\"scope\": \"project\"`)",
                "a secret of each project (`\"scope\": \"project\"`)"
            ));
        }
        if points.is_empty() {
            return None;
        }
        let points = points.join(", ");
        Some(tr_in!(
            lang,
            "Ce plugin demande ce que cette version d'Escouade ne sert pas encore : {points}. Il reste coupé plutôt que servi à moitié ; une version plus récente d'Escouade le servira.",
            "This plugin asks for what this version of Escouade does not serve yet: {points}. It stays disabled rather than served by half; a newer version of Escouade will serve it."
        ))
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
/// wrong; and, either way, the fields it ignored and the reserved points it declares.
#[derive(Debug, Clone)]
pub struct Read {
    pub manifest: Result<Manifest, Vec<Problem>>,
    pub ignored: Vec<String>,
    /// The points of `RESERVED_POINTS` it declares, by their path (`contributes.statusBar`): a
    /// warning, never a refusal.
    pub reserved: Vec<String>,
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
            reserved: Vec::new(),
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
        reserved: c.reserved,
    }
}

#[derive(Default)]
struct Checker {
    problems: Vec<Problem>,
    ignored: Vec<String>,
    reserved: Vec<String>,
    hand: Hand,
    /// The kinds of `contributes.profiles` that hold, and whether each has a secret field.
    kinds: Vec<(String, bool)>,
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

/// Whether a line of `permissions.commands` holds (§5.3 of docs/PLUGINS-CONTRACT-STUDY.md): its first word is a program's
/// name, never a path nor a joker; a later word is a joker (`JOKERS`) whole, or a literal word without a brace; a `*`
/// comes only as its last word. Words are split as the host splits them (`procs`, on white space).
pub fn command_holds(command: &str) -> bool {
    let words: Vec<&str> = command.split_whitespace().collect();
    let Some((first, rest)) = words.split_first() else {
        return false;
    };
    let args = rest.len().saturating_sub(1);
    !first.is_empty()
        && !first.contains(['/', '\\', ':', '*', '~', '{', '}'])
        && !first.starts_with('.')
        && !rest[..args].contains(&"*")
        && rest.iter().all(|w| is_joker(w) || !w.contains(['{', '}']))
}

/// Whether a word of a command is one of the jokers (`JOKERS`), whole.
pub fn is_joker(word: &str) -> bool {
    JOKERS.contains(&word)
}

/// Whether a line of `permissions.commands` holds a joker.
pub fn has_joker(command: &str) -> bool {
    command.split_whitespace().any(is_joker)
}

/// `.ovpn`, `.tar.gz`: an extension of a file that `import` filters on.
fn is_extension(s: &str) -> bool {
    s.len() <= 32
        && s.strip_prefix('.').is_some_and(|rest| {
            rest.split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric()))
        })
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

    fn manifest(&mut self, v: &Value, hand: Hand, file: Option<FileOf>) -> Option<Manifest> {
        self.hand = hand;
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
        let run = o.get("run").and_then(|v| self.run(v, file));
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
        self.profile_secrets(o);
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
                "processes",
                "reason",
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
        // The programs at once (§5.6): 16 at most, and why beyond the 4 it has without asking.
        if let Some(v) = o.get("processes") {
            l.processes = self.bounded(v, "limits.processes", PROCESSES);
        }
        if let Some(v) = o.get("reason") {
            l.reason = self.text(v, "limits.reason");
        }
        if l.processes.is_some_and(|n| n > PROCESSES_UNASKED) && !o.contains_key("reason") {
            self.problem("limits.reason", Code::Required);
        }
        l
    }

    /// A whole number of `range`, else `Code::Bounds`; `8.0` is 8, as the store's library reads it.
    fn bounded(
        &mut self,
        v: &Value,
        path: &str,
        range: std::ops::RangeInclusive<u64>,
    ) -> Option<u64> {
        let whole = v.as_u64().or_else(|| {
            v.as_f64()
                .filter(|f| f.fract() == 0.0 && *f >= 0.0 && *f <= *range.end() as f64)
                .map(|f| f as u64)
        });
        let n = whole.filter(|n| range.contains(n));
        if n.is_none() {
            self.problems.push(Problem {
                path: path.to_string(),
                code: Code::Bounds,
                expected: Some(format!("{}..{}", range.start(), range.end())),
            });
        }
        n
    }

    /// A boolean field of `item`, when it has it.
    fn flag(&mut self, item: &Map<String, Value>, at: &str, key: &str) -> Option<bool> {
        let v = item.get(key)?;
        let b = v.as_bool();
        if b.is_none() {
            self.wrong_type(&join(at, key), "boolean");
        }
        b
    }

    /// What a field (a setting, a secret, a field of a profile) says of itself beyond its key and title (ESC-525):
    /// `description`, `multiline` and `lines` (`text_like`: a `string` or a secret), `import`, and `repo` (`never_repo`:
    /// a secret, a profile, a value imported from a file never comes from .escouade.yml).
    fn field_extras(
        &mut self,
        at: &str,
        item: &Map<String, Value>,
        text_like: bool,
        never_repo: bool,
    ) {
        if let Some(d) = item.get("description") {
            self.text(d, &join(at, "description"));
        }
        let multiline = self.flag(item, at, "multiline");
        if multiline == Some(true) && !text_like {
            self.problem(&join(at, "multiline"), Code::Misplaced);
        }
        if let Some(v) = item.get("lines") {
            if multiline == Some(true) {
                self.bounded(v, &join(at, "lines"), LINES);
            } else {
                self.problem(&join(at, "lines"), Code::Misplaced);
            }
        }
        if let Some(v) = item.get("import") {
            let p = join(at, "import");
            if text_like {
                match v.as_array() {
                    None => self.wrong_type(&p, "array"),
                    Some(list) => {
                        for (i, e) in list.iter().enumerate() {
                            let pe = format!("{p}[{i}]");
                            if let Some(e) = self.filled(e, &pe) {
                                if !is_extension(e) {
                                    self.problem(&pe, Code::Extension);
                                }
                            }
                        }
                    }
                }
            } else {
                self.problem(&p, Code::Misplaced);
            }
        }
        if self.flag(item, at, "repo") == Some(true) && (never_repo || item.contains_key("import"))
        {
            self.problem(&join(at, "repo"), Code::Repo);
        }
    }

    /// `scope`: one of `SCOPES`.
    fn scope(&mut self, at: &str, item: &Map<String, Value>) {
        let Some(v) = item.get("scope") else { return };
        let p = join(at, "scope");
        if let Some(s) = self.string(v, &p) {
            if !SCOPES.contains(&s) {
                self.problems.push(Problem {
                    path: p,
                    code: Code::Enum,
                    expected: Some(SCOPES.join(", ")),
                });
            }
        }
    }

    /// `type` of `item`, required, one of `among`.
    fn typed<'a>(
        &mut self,
        at: &str,
        item: &'a Map<String, Value>,
        among: &[&str],
    ) -> Option<&'a str> {
        let p = join(at, "type");
        let Some(t) = item.get("type") else {
            self.problem(&p, Code::Required);
            return None;
        };
        let t = self.string(t, &p)?;
        if !among.contains(&t) {
            self.problems.push(Problem {
                path: p,
                code: Code::Enum,
                expected: Some(among.join(", ")),
            });
            return None;
        }
        Some(t)
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

    fn run(&mut self, v: &Value, file: Option<FileOf>) -> Option<Run> {
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
                let found = file.is_none_or(|file| file(&icon_path(path)).is_some());
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
            (Some(c), _) => self.inner_path(c, "run.command"),
        };
        let args = o
            .get("args")
            .map(|a| self.words(a, "run.args"))
            .unwrap_or_default();
        let mut by_system = std::collections::BTreeMap::new();
        for sys in systems {
            if let Some(p) = o.get(sys) {
                if let Some(p) = self.inner_path(p, &join("run", sys)) {
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
                "events", "network", "secrets", "commands", "env", "files", "decide",
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
        if let Some(v) = o.get("env") {
            let names = self.words(v, "permissions.env");
            for (i, name) in names.iter().enumerate() {
                if env_refused(name, self.hand) {
                    self.problem(&format!("permissions.env[{i}]"), Code::Variable);
                }
            }
            p.env = Some(names);
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
    /// What a harness described by ACP says of its sessions, its turns, its effort and its context (ESC-491): the
    /// arguments of a resume name the session (`{session}`), and what is read of its messages are JSON pointers.
    fn acp_described(&mut self, at: &str, item: &Map<String, Value>) {
        let Some(acp) = item.get("acp").and_then(Value::as_object) else {
            return;
        };
        let at = join(at, "acp");
        let pointer = |me: &mut Self, v: &Value, p: &str| match v.as_str() {
            Some(s) if s.starts_with('/') => {}
            Some(_) => me.problem(p, Code::Pointer),
            None => me.wrong_type(p, "string"),
        };
        if let Some(r) = acp.get("resume") {
            let p = join(&at, "resume");
            if let Some(r) = self.object(r, &p) {
                let pa = join(&p, "args");
                match r.get("args").map(|a| a.as_array()) {
                    None => self.problem(&pa, Code::Required),
                    Some(None) => self.wrong_type(&pa, "array"),
                    Some(Some(args)) => {
                        for (i, a) in args.iter().enumerate() {
                            if !a.is_string() {
                                self.wrong_type(&format!("{pa}[{i}]"), "string");
                            }
                        }
                        let named = args
                            .iter()
                            .any(|a| a.as_str().is_some_and(|a| a.contains("{session}")));
                        if !named {
                            self.problem(&pa, Code::Session);
                        }
                    }
                }
                if let Some(s) = r.get("session") {
                    pointer(self, s, &join(&p, "session"));
                }
            }
        }
        if let Some(ends) = acp.get("turnEnd") {
            let p = join(&at, "turnEnd");
            match ends.as_array() {
                None => self.wrong_type(&p, "array"),
                Some(ends) => {
                    for (i, e) in ends.iter().enumerate() {
                        let pe = format!("{p}[{i}]");
                        match e.as_object() {
                            Some(marks) if !marks.is_empty() => {
                                for k in marks.keys() {
                                    if !k.starts_with('/') {
                                        self.problem(&join(&pe, k), Code::Pointer);
                                    }
                                }
                            }
                            Some(_) => self.problem(&pe, Code::Empty),
                            None => self.wrong_type(&pe, "object"),
                        }
                    }
                }
            }
        }
        if let Some(efforts) = acp.get("efforts") {
            let p = join(&at, "efforts");
            if let Some(efforts) = self.object(efforts, &p) {
                for (k, v) in efforts {
                    if !v.is_string() {
                        self.wrong_type(&join(&p, k), "string");
                    }
                }
            }
        }
        if let Some(o) = acp.get("effortOption") {
            if !o.is_string() {
                self.wrong_type(&join(&at, "effortOption"), "string");
            }
        }
        if let Some(u) = acp.get("usage") {
            let p = join(&at, "usage");
            if let Some(u) = self.object(u, &p) {
                for k in ["context", "window"] {
                    if let Some(v) = u.get(k) {
                        pointer(self, v, &join(&p, k));
                    }
                }
            }
        }
    }

    /// A harness of anyone's (ESC-424): its program (`locate.names`, else its id) no shell nor interpreter, and the
    /// variables it sets (`acp.env`, `accountVar`, `keyVar`) and those it takes off (`acp.removeEnv`, SEC-30) none that
    /// Escouade refuses a plugin: taking off the proxy or the authorities the core gives is as much as setting others.
    fn harness_of_anyone(&mut self, at: &str, item: &Map<String, Value>) {
        let names = item
            .get("locate")
            .and_then(|l| l.get("names"))
            .and_then(Value::as_array);
        match names {
            Some(names) if !names.is_empty() => {
                for (i, n) in names.iter().enumerate() {
                    if n.as_str().is_some_and(interpreter) {
                        self.problem(&format!("{at}.locate.names[{i}]"), Code::Interpreter);
                    }
                }
            }
            _ => {
                if item
                    .get("id")
                    .and_then(Value::as_str)
                    .is_some_and(interpreter)
                {
                    self.problem(&join(at, "id"), Code::Interpreter);
                }
            }
        }
        if let Some(env) = item
            .get("acp")
            .and_then(|a| a.get("env"))
            .and_then(Value::as_object)
        {
            for k in env.keys() {
                if env_refused(k, Hand::Third) {
                    self.problem(&join(&join(at, "acp.env"), k), Code::Variable);
                }
            }
        }
        if let Some(removed) = item
            .get("acp")
            .and_then(|a| a.get("removeEnv"))
            .and_then(Value::as_array)
        {
            for (i, k) in removed.iter().enumerate() {
                if k.as_str().is_some_and(|k| env_refused(k, Hand::Third)) {
                    self.problem(&format!("{at}.acp.removeEnv[{i}]"), Code::Variable);
                }
            }
        }
        for key in ["accountVar", "keyVar"] {
            if item
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(|v| env_refused(v, Hand::Third))
            {
                self.problem(&join(at, key), Code::Variable);
            }
        }
    }

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

    /// `contributes.groups` (§4.1): an id, a title, a description, `folded`, and `fields`, the keys of its settings and
    /// secrets in the order they are shown; a key in one group at most, and one the plugin declares.
    fn groups(&mut self, v: &Value, declared: &[String]) {
        let items = self.items(v, "contributes.groups");
        self.keys(&items, "id");
        self.titled(&items, "title", true);
        let mut taken: Vec<String> = Vec::new();
        for (at, item) in &items {
            if let Some(d) = item.get("description") {
                self.text(d, &join(at, "description"));
            }
            self.flag(item, at, "folded");
            let p = join(at, "fields");
            match item.get("fields") {
                None => self.problem(&p, Code::Required),
                Some(Value::Array(list)) if list.is_empty() => self.problem(&p, Code::Empty),
                Some(Value::Array(list)) => {
                    for (i, k) in list.iter().enumerate() {
                        let pk = format!("{p}[{i}]");
                        let Some(k) = self.string(k, &pk) else {
                            continue;
                        };
                        if !declared.iter().any(|d| d == k) {
                            self.problem(&pk, Code::Field);
                        } else if taken.iter().any(|t| t == k) {
                            self.problem(&pk, Code::Duplicate);
                        } else {
                            taken.push(k.to_string());
                        }
                    }
                }
                Some(_) => self.wrong_type(&p, "array"),
            }
        }
    }

    /// `contributes.profiles` (§2.2): kinds of profile, each an id, a title, its fields (a setting's, or `secret: true`)
    /// and `nameField`, the `string` field that names a profile. Gives the good ids, and whether each has a secret field.
    fn profiles(&mut self, v: &Value) -> Vec<(String, bool)> {
        let items = self.items(v, "contributes.profiles");
        let ids = self.keys(&items, "id");
        self.titled(&items, "title", true);
        let mut kinds = Vec::new();
        for (at, item) in &items {
            let p = join(at, "fields");
            let fields = match item.get("fields") {
                None => {
                    self.problem(&p, Code::Required);
                    Vec::new()
                }
                Some(Value::Array(list)) if list.is_empty() => {
                    self.problem(&p, Code::Empty);
                    Vec::new()
                }
                Some(f) => self.items(f, &p),
            };
            self.keys(&fields, "key");
            self.titled(&fields, "title", true);
            let mut secret = false;
            // The keys a profile can be named by: a `string` field, neither secret nor on several lines.
            let mut names: Vec<&str> = Vec::new();
            for (fat, field) in &fields {
                let is_secret = self.flag(field, fat, "secret") == Some(true);
                if field.contains_key("scope") {
                    self.problem(&join(fat, "scope"), Code::Misplaced);
                }
                let kind = if is_secret {
                    secret = true;
                    if field.contains_key("type") {
                        self.problem(&join(fat, "type"), Code::Misplaced);
                    }
                    None
                } else {
                    self.typed(fat, field, &FIELD_TYPES)
                };
                if field.contains_key("of") {
                    self.problem(&join(fat, "of"), Code::Misplaced);
                }
                self.field_extras(fat, field, is_secret || kind == Some("string"), true);
                let one_line = field.get("multiline").and_then(Value::as_bool) != Some(true);
                if kind == Some("string") && one_line {
                    if let Some(k) = field.get("key").and_then(Value::as_str) {
                        names.push(k);
                    }
                }
            }
            let pn = join(at, "nameField");
            match item.get("nameField") {
                None => self.problem(&pn, Code::Required),
                Some(n) => {
                    if let Some(n) = self.string(n, &pn) {
                        if !names.contains(&n) {
                            self.problem(&pn, Code::Field);
                        }
                    }
                }
            }
            if let Some(id) = item.get("id").and_then(Value::as_str) {
                if ids.iter().any(|i| i == id) && !kinds.iter().any(|(k, _)| k == id) {
                    kinds.push((id.to_string(), secret));
                }
            }
        }
        kinds
    }

    /// `permissions.secrets` that name a profile's (`profile:<kind>`): a kind of `contributes.profiles` with a secret
    /// field, as the store's check n° 9 holds `account:<kind>` (§2.4).
    fn profile_secrets(&mut self, o: &Map<String, Value>) {
        let Some(list) = o
            .get("permissions")
            .and_then(|p| p.get("secrets"))
            .and_then(Value::as_array)
        else {
            return;
        };
        for (i, s) in list.iter().enumerate() {
            let Some(kind) = s.as_str().and_then(|s| s.strip_prefix("profile:")) else {
                continue;
            };
            if !self.kinds.iter().any(|(k, secret)| k == kind && *secret) {
                self.problem(&format!("permissions.secrets[{i}]"), Code::Profile);
            }
        }
    }

    fn contributes(&mut self, v: &Value, network: &[String]) -> Map<String, Value> {
        let mut kept = Map::new();
        let Some(o) = self.object(v, "contributes") else {
            return kept;
        };
        let mut known: Vec<&str> = vec!["settings", "secrets", "groups", "profiles", "escouadeYml"];
        known.extend(POINTS_WITH_ID);
        known.extend(OPEN_POINTS);
        self.ignore_others(o, "contributes", &known);
        let path = |k: &str| join("contributes", k);
        for point in RESERVED_POINTS {
            if o.contains_key(point) {
                self.reserved.push(path(point));
            }
        }
        // The kinds of profile first (§2.2): a setting of type `profile` names one.
        let mut kinds = Vec::new();
        if let Some(v) = o.get("profiles") {
            kinds = self.profiles(v);
        }
        let mut fields: Vec<String> = Vec::new();
        if let Some(v) = o.get("settings") {
            let items = self.items(v, &path("settings"));
            fields.extend(self.keys(&items, "key"));
            for (at, item) in &items {
                let kind = self.typed(at, item, &SETTING_TYPES);
                let profile = kind == Some("profile");
                match item.get("of") {
                    Some(of) if profile => {
                        let p = join(at, "of");
                        if let Some(of) = self.string(of, &p) {
                            if !kinds.iter().any(|(k, _)| k == of) {
                                self.problem(&p, Code::Profile);
                            }
                        }
                    }
                    None if profile => self.problem(&join(at, "of"), Code::Required),
                    Some(_) => self.problem(&join(at, "of"), Code::Misplaced),
                    None => {}
                }
                self.scope(at, item);
                self.field_extras(at, item, kind == Some("string"), profile);
            }
            self.titled(&items, "title", true);
        }
        if let Some(v) = o.get("secrets") {
            let items = self.items(v, &path("secrets"));
            fields.extend(self.keys(&items, "key"));
            self.titled(&items, "title", true);
            for (at, item) in &items {
                self.scope(at, item);
                self.field_extras(at, item, true, true);
            }
        }
        if let Some(v) = o.get("groups") {
            self.groups(v, &fields);
        }
        self.kinds = kinds;
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
            if point == "harnesses" {
                for (at, item) in &items {
                    self.acp_described(at, item);
                }
            }
            if point == "harnesses" && self.hand == Hand::Third {
                for (at, item) in &items {
                    self.harness_of_anyone(at, item);
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
        "description": "The programs it may run (`proc/spawn`): a program's name, never a path nor a joker, then its arguments, word for word. A joker stands for one word of a closed form, written whole: `{int}`, `{port}`, `{name}`, `{pid}` (a program it runs), `{path:temp}`, `{path:plugin}`; no other brace. A last word `*` lets any more follow.",
        "items": { "type": "string", "minLength": 1 },
    });
    // What a field says of itself beyond its key and title (ESC-525).
    let scope = json!({
        "enum": SCOPES,
        "description": "Where it holds: for every project (`app`, the default), or for each one, set in the project's settings (`project`).",
    });
    let multiline = json!({
        "type": "boolean",
        "description": "Typed on several lines (a `string` setting or a secret), 64 KiB at most.",
    });
    let lines = json!({
        "type": "integer",
        "minimum": LINES.start(),
        "maximum": LINES.end(),
        "description": "How many lines it shows, with `multiline`: 6 without it.",
    });
    let import = json!({
        "type": "array",
        "description": "The extensions of the files « Import a file… » offers (`.ovpn`): their text fills the field once; the path is never kept. Never from .escouade.yml.",
        "items": { "type": "string", "pattern": "^(\\.[A-Za-z0-9]+)+$", "maxLength": 32 },
    });
    let description = json!({
        "description": "What it is for, under its title: one text, or a text by language.",
        "oneOf": text["oneOf"].clone(),
    });
    let setting = json!({
        "type": "object",
        "required": ["key", "type", "title"],
        "properties": {
            "key": key,
            "type": { "enum": SETTING_TYPES, "description": "`profile`: one profile of the kind `of` names, chosen by the user." },
            "title": text,
            "description": {
                "description": "Shown under its title: one text, or a text by language (`en` first, then `fr`).",
            },
            "default": {
                "description": "Its value until the user sets one, of its type (for a `select`, one of its options); without it, empty, or false for a `boolean`.",
            },
            "options": {
                "description": "The choices of a `select`, in this order: each a value, or `{ \"value\": …, \"title\": … }` to show a title in its stead (one text, or a text by language).",
            },
            "of": { "type": "string", "description": "With `type: \"profile\"`: the id of a kind of `contributes.profiles`." },
            "scope": scope,
            "repo": {
                "type": "boolean",
                "description": "`false`: its value never comes from .escouade.yml. Never `true` for a profile or an imported value.",
            },
            "multiline": multiline,
            "lines": lines,
            "import": import,
        },
    });
    let secret = json!({
        "type": "object",
        "required": ["key", "title"],
        "properties": {
            "key": key,
            "title": text,
            "description": description,
            "scope": scope,
            "repo": { "const": false, "description": "A secret never comes from .escouade.yml." },
            "multiline": multiline,
            "lines": lines,
            "import": import,
        },
    });
    let group = json!({
        "type": "object",
        "required": ["id", "title", "fields"],
        "properties": {
            "id": key,
            "title": text,
            "description": description,
            "folded": { "type": "boolean", "description": "Folded at first, as « Advanced »." },
            "fields": {
                "type": "array",
                "minItems": 1,
                "description": "The keys of its settings and secrets, mixed, in the order they are shown; a key in one group at most.",
                "items": { "type": "string" },
            },
        },
    });
    let profile_field = json!({
        "type": "object",
        "required": ["key", "title"],
        "properties": {
            "key": key,
            "title": text,
            "description": description,
            "type": { "enum": FIELD_TYPES, "description": "Needed unless it is `secret`." },
            "secret": { "type": "boolean", "description": "Kept in the keychain, never shown again; it has no `type`." },
            "multiline": multiline,
            "lines": lines,
            "import": import,
        },
    });
    let profile = json!({
        "type": "object",
        "required": ["id", "title", "nameField", "fields"],
        "properties": {
            "id": key,
            "title": text,
            "nameField": { "type": "string", "description": "The key of the `string` field that names a profile (neither secret nor on several lines)." },
            "fields": { "type": "array", "minItems": 1, "items": profile_field },
        },
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
            "env": {
                "type": "array",
                "description": "The variables it may set on what it runs (`proc/spawn`, the launches of its harnesses), shown before it is installed; any other is refused. Never Escouade's own nor `LD_*`, `DYLD_*`, `GIT_*` (but `GIT_AUTHOR_*`, `GIT_COMMITTER_*`), `PYTHON*`, `RUBY*`, `PERL*`, `NODE_*`, `JAVA_TOOL_OPTIONS`, `npm_config_*`, `*_PROXY`, `SSL_*`, `XDG_*`, `HOME`, `USERPROFILE`, `APPDATA`, `PATH`.",
                "items": { "type": "string", "pattern": "^[A-Za-z_][A-Za-z0-9_]*$" },
            },
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
                "description": "Its settings, shown on its page in the order of this list, under « Settings » (or in their groups).",
                "items": setting,
            },
            "secrets": { "type": "array", "items": secret },
            "groups": {
                "type": "array",
                "description": "Groups of its settings and secrets, in order, on its page.",
                "items": group,
            },
            "profiles": {
                "type": "array",
                "description": "Kinds of profile: sets of fields the user makes as many times as wanted, and that each project chooses (a setting of `type: \"profile\"`). Their secrets are asked as `profile:<id>` in `permissions.secrets`.",
                "items": profile,
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
            "menus": open("Reserved, with no effect in this version: Escouade shows and calls nothing of it."),
            "statusBar": open("Reserved, with no effect in this version: Escouade shows and calls nothing of it."),
            "badges": open("Reserved, with no effect in this version: Escouade shows and calls nothing of it."),
            "projectTemplates": open("Reserved, with no effect in this version: Escouade shows and calls nothing of it."),
            "escouadeYml": { "type": "object", "description": "Reserved, with no effect in this version: read and ignored." },
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
            "processes": {
                "type": "integer",
                "minimum": PROCESSES.start(),
                "maximum": PROCESSES.end(),
                "description": "How many programs it may run at once (`proc/spawn`): 4 without it. Beyond 4, said at its install and with `reason`.",
            },
            "reason": {
                "description": "Why it asks for more programs at once, in a sentence: needed with `processes` beyond 4.",
                "oneOf": [
                    { "type": "string", "minLength": 1 },
                    { "type": "object", "additionalProperties": { "type": "string" } },
                ],
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
