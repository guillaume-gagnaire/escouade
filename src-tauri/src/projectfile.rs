//! `.escouade.yml` (ESC-146): the settings of a project its team shares through its repository,
//! read when the project opens and whenever the file changes, written by « Exporter vers
//! .escouade.yml ».
//!
//! Its format is versioned (`version`, `VERSION` today): a file of an older version is migrated
//! (`MIGRATIONS`) before the reader of the current one reads it; one of a newer version is read as
//! well as this one can. A key it does not know, a value it cannot take, are told (`Warning`) and
//! left out, the rest of the file read all the same. `KEYS` is the one list of what it holds: the
//! reader, the writer, what applies it to a project and the JSON schema (`schema`) all go by it.
//!
//! Nothing personal and nothing of one machine is in it: no name, color, account (nor the accounts
//! reserved to a private project), Remote Control, agents in parallel, autopilot, ticket key, absolute
//! path or token (ESC-163). « Projet privé » is (`private`): the repository is sensitive for all;
//! but the file may make a project private, never public again. Its commands (`Key::command`) run
//! nowhere until the user trusts them, their fingerprint kept (`ProjectShared::trusted`) and asked
//! for again when they change.

use std::collections::BTreeMap;
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use yaml_edit::{
    CoreScalarType, Document, Mapping, ScalarStyle, ScalarType, ScalarValue, Sequence, YamlNode,
};

use crate::model::{CommitMode, EngineId, Project, RunCommand, WorktreeBase, WorktreeStep};

pub const FILE: &str = ".escouade.yml";
/// The version this app writes, and reads without migrating.
pub const VERSION: u32 = 1;
/// Where the site serves the schema (`schemas/escouade.schema.json` of the repository).
pub const SCHEMA_URL: &str = "https://escouade.dev/schemas/escouade.schema.json";

/// What a key's value is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Bool,
    Text,
    Enum(&'static [&'static str]),
    /// Texts (a single one stands for a list of one).
    List,
    /// Worktree steps: `command`, and `shell`, `cwd` if need be (a text alone is the command).
    Steps,
    /// Launch commands: `name`, `command`, and `shell`, `cwd` if need be.
    Launch,
    /// A folder relative to the repository.
    Dir,
    /// The start of a branch name.
    Prefix,
    /// Plugins' ids in the store (`dev.exemple.slack`; a single one stands for a list of one).
    PluginIds,
}

/// A setting of the file.
#[derive(Debug, Clone, Copy)]
pub struct Key {
    /// Its place in the file, its groups joined by dots (`board.merge_strategy`).
    pub path: &'static str,
    pub kind: Kind,
    /// A command something runs: inactive until the user trusts it.
    pub command: bool,
    /// The file's alone (`plugins.*`): no field of the project holds it, so it is read, never
    /// applied to the project nor exported from it.
    pub file_only: bool,
    /// What the schema says of it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub about: &'static str,
}

const fn key(path: &'static str, kind: Kind, about: &'static str) -> Key {
    Key {
        path,
        kind,
        command: false,
        file_only: false,
        about,
    }
}

const fn command(path: &'static str, kind: Kind, about: &'static str) -> Key {
    Key {
        path,
        kind,
        command: true,
        file_only: false,
        about,
    }
}

const fn file_key(path: &'static str, kind: Kind, about: &'static str) -> Key {
    Key {
        path,
        kind,
        command: false,
        file_only: true,
        about,
    }
}

/// The plugins' group (PLG-14): `plugins.recommended`, and each plugin's settings under its id.
pub const PLUGINS: &str = "plugins";
/// The plugins the project proposes.
pub const RECOMMENDED: &str = "plugins.recommended";

/// Whether `path` is the file's alone (`plugins.*`): no field of the project holds it.
pub fn file_only(path: &str) -> bool {
    path.split('.').next() == Some(PLUGINS)
}

/// The settings the file gives the plugin `id` (`plugins.<id>`), as read; none when it gives none.
pub fn plugin_settings<'a>(
    values: &'a BTreeMap<String, Value>,
    id: &str,
) -> Option<&'a Map<String, Value>> {
    values
        .get(&format!("{PLUGINS}.{id}"))
        .and_then(Value::as_object)
}

pub const ENGINES: &[&str] = &["claude", "codex", "copilot", "opencode"];
/// The efforts of the tickets' agents (the window's `EFFORTS`); empty: the account's.
pub const EFFORTS: &[&str] = &["", "low", "medium", "high", "xhigh", "max"];
/// The modes of the tickets' agents the file may set; empty: the account's. Not
/// `bypassPermissions`: a repository does not lift the permissions of whoever opens it.
pub const MODES: &[&str] = &["", "auto", "default", "plan", "acceptEdits"];
/// The mode the file never carries.
const BYPASS: &str = "bypassPermissions";
/// « Projet privé »: the file may turn it on, never off.
pub const PRIVATE: &str = "private";
const MODE: &str = "board.mode";

/// Every setting of version 1, in the order the file is written.
pub const KEYS: &[Key] = &[
    key(
        "agents.engine",
        Kind::Enum(ENGINES),
        "The harness the project's new agents and tickets run on.",
    ),
    key(
        PRIVATE,
        Kind::Bool,
        "« Projet privé »: the agents of the other projects do not know it, nor read its files. The accounts reserved to it stay each machine's. A project private on a machine stays so when the file says false.",
    ),
    key(
        "worktrees.per_agent",
        Kind::Bool,
        "Each new agent works in a worktree of its own.",
    ),
    key(
        "worktrees.at_first_message",
        Kind::Bool,
        "A new agent's worktree is made at its first message, its branch and its folder under the name it is given then (true when absent).",
    ),
    key(
        "worktrees.base",
        Kind::Text,
        "The branch a new agent's worktree starts from, its base: head (the branch the project's folder is on, when absent), target (the board's target branch) or a branch's name (local, or remote such as origin/main: through the local branch that tracks it). A branch named head or target is written refs/heads/<name>.",
    ),
    key(
        "worktrees.dir",
        Kind::Dir,
        "Where the worktrees go, relative to the repository (empty: .claude/worktrees).",
    ),
    key(
        "worktrees.copy",
        Kind::List,
        "Untracked files copied into every new worktree (glob patterns).",
    ),
    command(
        "worktrees.setup",
        Kind::Steps,
        "Run in every new worktree before its agent starts. Needs the user's approval.",
    ),
    command(
        "worktrees.teardown",
        Kind::Steps,
        "Run in a worktree before it is removed. Needs the user's approval.",
    ),
    command(
        "launch",
        Kind::Launch,
        "The commands that launch the project (« Lancement »). Need the user's approval.",
    ),
    key(
        "git.agent_branch_prefix",
        Kind::Prefix,
        "A new agent's branch is <prefix><name>.",
    ),
    key(
        "git.ticket_branch_prefix",
        Kind::Prefix,
        "A ticket's branch is <prefix><key>.",
    ),
    key(
        "git.commit_mode",
        Kind::Enum(&["agent", "direct"]),
        "Who writes the commits asked from the files panel: the agent, or the app with a proposed message.",
    ),
    key(
        "git.conventional_commits",
        Kind::Bool,
        "Commit messages in the Conventional Commits form.",
    ),
    key(
        "board.on_approve",
        Kind::Enum(&["merge", "pr", "push", "keep"]),
        "What approving a ticket does with its branch.",
    ),
    key(
        "board.target_branch",
        Kind::Text,
        "Where tickets merge: the base of their branch and of their pull request.",
    ),
    key(
        "board.merge_strategy",
        Kind::Enum(&["merge", "squash", "rebase"]),
        "How a ticket's branch merges.",
    ),
    key("board.draft_pr", Kind::Bool, "Pull requests open as drafts."),
    command(
        "board.test_command",
        Kind::Text,
        "The project's tests. Needs the user's approval.",
    ),
    key(
        "board.tests_before_approve",
        Kind::Bool,
        "The tests run again before a ticket is approved.",
    ),
    key(
        "board.on_conflict",
        Kind::Enum(&["ask", "agent", "abort"]),
        "What a merge conflict does: ask, have the agent resolve it, or give up.",
    ),
    key(
        "board.remove_worktree",
        Kind::Bool,
        "A ticket's worktree is removed once it is done.",
    ),
    key(
        "board.model",
        Kind::Text,
        "The model of the tickets' agents, as the harness names it (empty: the account's).",
    ),
    key(
        "board.effort",
        Kind::Enum(EFFORTS),
        "The effort of the tickets' agents (empty: the account's).",
    ),
    key(
        MODE,
        Kind::Enum(MODES),
        "The permission mode of the tickets' agents (empty: the account's). Never bypassPermissions: that one is set on each machine.",
    ),
    file_key(
        RECOMMENDED,
        Kind::PluginIds,
        "The plugins the project proposes, by their id in the store (store.escouade.dev). Escouade names those not installed in Settings › Project, each with its page in the store: it never installs nor turns on anything by itself. A plugin's own settings go under plugins.<id>, never a secret.",
    ),
];

pub fn key_of(path: &str) -> Option<&'static Key> {
    KEYS.iter().find(|k| k.path == path)
}

/// The groups of the file (`worktrees`…), in their order.
fn groups() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for k in KEYS {
        let g = k.path.split('.').next().unwrap_or(k.path);
        if !out.contains(&g) {
            out.push(g);
        }
    }
    out
}

/// Something of the file that was left out, or that the user should know.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Warning {
    /// From 1; none when it is the whole file's.
    pub line: Option<u32>,
    pub text: String,
}

fn warn(line: Option<u32>, text: String) -> Warning {
    Warning { line, text }
}

/// The file as read: the value of each key it sets (as `get` gives it from a project).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Read {
    pub values: BTreeMap<String, Value>,
    pub warnings: Vec<Warning>,
    /// Its `version`, as written (1 when it has none or one that cannot be read).
    pub version: u32,
}

/// A node of the file, with the line it starts on: what the migrations and the reader walk.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub line: u32,
    pub v: N,
}

#[derive(Debug, Clone, PartialEq)]
pub enum N {
    Scalar {
        text: String,
        quoted: bool,
        null: bool,
    },
    Map(Vec<(String, u32, Node)>),
    Seq(Vec<Node>),
    /// An alias or a tagged node: not read.
    Other,
}

impl Node {
    fn get(&self, key: &str) -> Option<&Node> {
        match &self.v {
            N::Map(entries) => entries.iter().find(|(k, _, _)| k == key).map(|(_, _, n)| n),
            _ => None,
        }
    }
}

fn line_at(text: &str, node: &YamlNode) -> u32 {
    let pos = match node {
        YamlNode::Scalar(s) => s.start_position(text),
        YamlNode::Mapping(m) => m.start_position(text),
        YamlNode::Sequence(s) => s.start_position(text),
        _ => return 0,
    };
    pos.line as u32
}

pub(crate) fn node_of(text: &str, node: &YamlNode) -> Node {
    let line = line_at(text, node);
    let v = match node {
        YamlNode::Scalar(s) => N::Scalar {
            text: s.as_string(),
            quoted: s.is_quoted(),
            null: s.is_null(),
        },
        YamlNode::Mapping(m) => N::Map(
            m.iter()
                .map(|(k, v)| {
                    let name = k.as_scalar().map(|s| s.as_string()).unwrap_or_default();
                    (name, line_at(text, &k), node_of(text, &v))
                })
                .collect(),
        ),
        YamlNode::Sequence(s) => N::Seq(s.values().map(|v| node_of(text, &v)).collect()),
        _ => N::Other,
    };
    Node { line, v }
}

/// Parses the file: its tree, or why it cannot be read at all (its YAML, or no mapping at its top).
pub fn tree(text: &str) -> Result<Node, Warning> {
    let doc = Document::from_str(text).map_err(|e| match e {
        yaml_edit::YamlError::Parse { message, line, .. } => warn(
            line.map(|l| l as u32),
            tr!(
                "{FILE} illisible : {message}",
                "{FILE} can’t be read: {message}"
            ),
        ),
        other => warn(
            None,
            tr!(
                "{FILE} illisible : {other}",
                "{FILE} can’t be read: {other}"
            ),
        ),
    })?;
    if text.trim().is_empty() {
        return Ok(Node {
            line: 1,
            v: N::Map(Vec::new()),
        });
    }
    match doc.as_mapping() {
        Some(m) => Ok(node_of(text, &YamlNode::Mapping(m))),
        None => Err(warn(
            Some(1),
            tr!(
                "{FILE} doit être une suite de clés (version: 1…)",
                "{FILE} must be a set of keys (version: 1…)"
            ),
        )),
    }
}

/// A migration takes the tree of a version to the next one, telling what it changed if need be.
pub type Migration = fn(&mut Node, &mut Vec<Warning>);

/// `MIGRATIONS[i]` takes version `i + 1` to version `i + 2`. None yet: version 1 is the first.
pub const MIGRATIONS: &[Migration] = &[];

/// Takes the tree of version `from` to the last version `steps` knows (`steps[i]`: from `i + 1`).
pub fn migrate(tree: &mut Node, from: u32, steps: &[Migration], warnings: &mut Vec<Warning>) {
    for step in steps.iter().skip(from.saturating_sub(1) as usize) {
        step(tree, warnings);
    }
}

/// Reads the file: each key it sets that this app knows, migrated from its version first. An
/// error only when nothing of it can be read.
pub fn read(text: &str) -> Result<Read, Warning> {
    let mut t = tree(text)?;
    let mut warnings = Vec::new();
    let version = match t.get("version") {
        None => {
            warnings.push(warn(
                None,
                tr!(
                    "pas de clé « version » : lu comme la version {VERSION}",
                    "no “version” key: read as version {VERSION}"
                ),
            ));
            VERSION
        }
        Some(n) => match &n.v {
            N::Scalar {
                text,
                quoted: false,
                ..
            } if text.parse::<u32>().is_ok_and(|v| v >= 1) => text.parse().unwrap_or(VERSION),
            _ => {
                warnings.push(warn(
                    Some(n.line),
                    tr!(
                        "« version » doit être un entier (1…) : lu comme la version {VERSION}",
                        "“version” must be a whole number (1…): read as version {VERSION}"
                    ),
                ));
                VERSION
            }
        },
    };
    if version > VERSION {
        warnings.push(warn(
            t.get("version").map(|n| n.line),
            tr!(
                "fait pour une version plus récente d'Escouade (version {version} du fichier, {VERSION} connue ici) : lu au mieux",
                "made for a newer version of Escouade (file version {version}, {VERSION} known here): read as well as possible"
            ),
        ));
    }
    migrate(&mut t, version, MIGRATIONS, &mut warnings);
    let mut values = BTreeMap::new();
    read_v1(&t, &mut values, &mut warnings);
    Ok(Read {
        values,
        warnings,
        version,
    })
}

/// The reader of version 1: every key of `KEYS`, the others told.
fn read_v1(t: &Node, values: &mut BTreeMap<String, Value>, warnings: &mut Vec<Warning>) {
    let N::Map(entries) = &t.v else { return };
    let top: Vec<&str> = groups();
    let mut legacy = None;
    for (name, line, node) in entries {
        if name == "version" {
            continue;
        }
        if let Some(k) = key_of(name) {
            take(k, node, values, warnings);
            continue;
        }
        if !top.contains(&name.as_str()) {
            warnings.push(unknown(name, *line, &top));
            continue;
        }
        let N::Map(inner) = &node.v else {
            if !matches!(node.v, N::Scalar { null: true, .. }) {
                warnings.push(warn(
                    Some(node.line),
                    tr!(
                        "« {name} » doit regrouper des clés : ignoré",
                        "“{name}” must hold keys: ignored"
                    ),
                ));
            }
            continue;
        };
        let names: Vec<&str> = KEYS
            .iter()
            .filter_map(|k| k.path.strip_prefix(&format!("{name}.")))
            .collect();
        for (sub, line, node) in inner {
            let path = format!("{name}.{sub}");
            match key_of(&path) {
                Some(k) => take(k, node, values, warnings),
                None if path == FROM_TARGET => legacy = Some(node),
                None if name == PLUGINS && crate::plugins::manifest::is_id(sub) => {
                    take_settings(&path, node, values, warnings)
                }
                None => warnings.push(unknown(&path, *line, &names)),
            }
        }
    }
    // The key `worktrees.base` took the place of, read when the file has no `worktrees.base`.
    if let Some(node) = legacy.filter(|_| !values.contains_key(BASE)) {
        let k = key(FROM_TARGET, Kind::Bool, "");
        let mut read = BTreeMap::new();
        take(&k, node, &mut read, warnings);
        if let Some(on) = read.get(FROM_TARGET).and_then(Value::as_bool) {
            let base = if on { "target" } else { "head" };
            values.insert(BASE.to_string(), json!(base));
            warnings.push(warn(
                Some(node.line),
                tr!(
                    "{FROM_TARGET} est l’ancienne clé de {BASE} : lue comme « {BASE}: {base} »",
                    "{FROM_TARGET} is the former key of {BASE}: read as “{BASE}: {base}”"
                ),
            ));
        }
    }
}

/// « Branche de départ des nouveaux worktrees » (ESC-160).
const BASE: &str = "worktrees.base";
/// The key it took the place of: `true` is `target`, `false` is `head`. Still read, never written.
const FROM_TARGET: &str = "board.agents_from_target";

fn take(k: &Key, node: &Node, values: &mut BTreeMap<String, Value>, warnings: &mut Vec<Warning>) {
    match value_of(k, node, warnings) {
        Ok(v) => {
            values.insert(k.path.to_string(), v);
        }
        Err((line, why)) => {
            let path = k.path;
            warnings.push(warn(
                Some(line),
                tr!(
                    "valeur invalide pour {path} : {why} (ignorée)",
                    "invalid value for {path}: {why} (ignored)"
                ),
            ))
        }
    }
}

/// A plugin's settings (`plugins.<id>`): simple values by key (a text, a number, true or false);
/// one of another form is told and left out. Held to the plugin's own settings once it is
/// installed (`plugins::project_settings`).
fn take_settings(
    path: &str,
    node: &Node,
    values: &mut BTreeMap<String, Value>,
    warnings: &mut Vec<Warning>,
) {
    let entries = match &node.v {
        N::Map(entries) => entries,
        N::Scalar { null: true, .. } => return,
        _ => {
            warnings.push(warn(
                Some(node.line),
                tr!(
                    "« {path} » doit regrouper les réglages du plugin : ignoré",
                    "“{path}” must hold the plugin's settings: ignored"
                ),
            ));
            return;
        }
    };
    let mut out = Map::new();
    for (k, line, n) in entries {
        let v = match &n.v {
            N::Scalar { null: true, .. } => continue,
            N::Scalar {
                text, quoted: true, ..
            } => json!(text),
            N::Scalar { text, .. } => {
                let t = text.trim();
                if t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("false") {
                    json!(t.eq_ignore_ascii_case("true"))
                } else if let Ok(i) = t.parse::<i64>() {
                    json!(i)
                } else if let Some(f) = t.parse::<f64>().ok().filter(|f| f.is_finite()) {
                    json!(f)
                } else {
                    json!(t)
                }
            }
            _ => {
                warnings.push(warn(
                    Some(*line),
                    tr!(
                        "valeur invalide pour {path}.{k} : un texte, un nombre, true ou false est attendu (ignorée)",
                        "invalid value for {path}.{k}: a text, a number, true or false is expected (ignored)"
                    ),
                ));
                continue;
            }
        };
        out.insert(k.clone(), v);
    }
    if !out.is_empty() {
        values.insert(path.to_string(), Value::Object(out));
    }
}

/// « clé inconnue : … », with the known key it looks like.
fn unknown(path: &str, line: u32, near: &[&str]) -> Warning {
    let leaf = path.rsplit('.').next().unwrap_or(path);
    let close = near
        .iter()
        .map(|c| (distance(leaf, c), *c))
        .filter(|(d, c)| *d <= 2.max(c.len() / 3))
        .min()
        .map(|(_, c)| match path.rsplit_once('.') {
            Some((group, _)) => format!("{group}.{c}"),
            None => c.to_string(),
        })
        // The same name in another group (`worktrees.test_command`).
        .or_else(|| {
            KEYS.iter()
                .find(|k| k.path.rsplit('.').next() == Some(leaf) && k.path != path)
                .map(|k| k.path.to_string())
        });
    let text = match close {
        Some(c) => tr!(
            "clé inconnue : {path} (voulais-tu dire {c} ?), ignorée",
            "unknown key: {path} (did you mean {c}?), ignored"
        ),
        None => tr!(
            "clé inconnue : {path}, ignorée",
            "unknown key: {path}, ignored"
        ),
    };
    warn(Some(line), text)
}

/// Levenshtein's distance between two names.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for j in 0..b.len() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != b[j]))
                .min(row[j] + 1)
                .min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

type Invalid = (u32, String);

fn text_of(node: &Node) -> Result<String, Invalid> {
    match &node.v {
        N::Scalar { null: true, .. } => Ok(String::new()),
        N::Scalar { text, .. } => Ok(text.trim().to_string()),
        _ => Err((node.line, tr!("un texte est attendu", "a text is expected"))),
    }
}

fn is_absolute(p: &str) -> bool {
    let p = p.trim();
    p.starts_with('/')
        || p.starts_with('\\')
        || p.starts_with('~')
        || (p.len() >= 2 && p.as_bytes()[1] == b':')
}

fn rel_dir(node: &Node) -> Result<String, Invalid> {
    let dir = text_of(node)?;
    if is_absolute(&dir) {
        return Err((
            node.line,
            tr!(
                "« {dir} » : le chemin doit être relatif au dépôt",
                "“{dir}”: the path must be relative to the repository"
            ),
        ));
    }
    Ok(dir)
}

/// The steps or launch commands of a list: an item that cannot be read is told and left out.
fn items(k: &Key, node: &Node, warnings: &mut Vec<Warning>) -> Result<Value, Invalid> {
    let list: Vec<&Node> = match &node.v {
        N::Seq(items) => items.iter().collect(),
        N::Scalar { null: true, .. } => Vec::new(),
        // A single step or command is a list of one.
        _ => vec![node],
    };
    let launch = k.kind == Kind::Launch;
    let fields: &[&str] = if launch {
        &["name", "command", "shell", "cwd"]
    } else {
        &["command", "shell", "cwd"]
    };
    let mut out = Vec::new();
    for item in list {
        let mut m = Map::new();
        for f in fields {
            m.insert(f.to_string(), json!(""));
        }
        let read: Result<(), Invalid> = (|| {
            match &item.v {
                N::Scalar { null: false, .. } if !launch => {
                    m.insert("command".into(), json!(text_of(item)?));
                }
                N::Map(entries) => {
                    for (f, line, n) in entries {
                        if !fields.contains(&f.as_str()) {
                            warnings.push(unknown(&format!("{}.{f}", k.path), *line, fields));
                            continue;
                        }
                        let v = if f == "cwd" { rel_dir(n)? } else { text_of(n)? };
                        m.insert(f.clone(), json!(v));
                    }
                }
                _ => {
                    return Err((
                        item.line,
                        if launch {
                            tr!(
                                "chaque commande a un « name » et une « command »",
                                "each command has a “name” and a “command”"
                            )
                        } else {
                            tr!(
                                "chaque étape est une commande, ou a une « command »",
                                "each step is a command, or has a “command”"
                            )
                        },
                    ))
                }
            }
            if m["command"].as_str().unwrap_or_default().is_empty() {
                return Err((item.line, tr!("« command » manque", "“command” is missing")));
            }
            if launch && m["name"].as_str().unwrap_or_default().is_empty() {
                let c = m["command"].clone();
                m.insert("name".into(), c);
            }
            Ok(())
        })();
        match read {
            Ok(()) => out.push(Value::Object(m)),
            Err((line, why)) => {
                let path = k.path;
                warnings.push(warn(
                    Some(line),
                    tr!(
                        "valeur invalide pour {path} : {why} (cette entrée est ignorée)",
                        "invalid value for {path}: {why} (this item is ignored)"
                    ),
                ))
            }
        }
    }
    Ok(Value::Array(out))
}

fn value_of(k: &Key, node: &Node, warnings: &mut Vec<Warning>) -> Result<Value, Invalid> {
    match k.kind {
        Kind::Bool => match &node.v {
            N::Scalar {
                text,
                quoted: false,
                ..
            } if text.eq_ignore_ascii_case("true") || text.eq_ignore_ascii_case("false") => {
                Ok(json!(text.eq_ignore_ascii_case("true")))
            }
            _ => Err((
                node.line,
                tr!("true ou false est attendu", "true or false is expected"),
            )),
        },
        Kind::Text => text_of(node).map(Value::String),
        Kind::Enum(allowed) => {
            let v = text_of(node)?;
            if allowed.contains(&v.as_str()) {
                Ok(json!(v))
            } else if k.path == MODE && v == BYPASS {
                Err((
                    node.line,
                    tr!(
                        "{BYPASS} ne se partage pas : il se règle sur chaque machine",
                        "{BYPASS} is not shared: it is set on each machine"
                    ),
                ))
            } else {
                // Empty (the account's) goes without saying.
                let list = allowed
                    .iter()
                    .filter(|a| !a.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ");
                Err((
                    node.line,
                    tr!("« {v} » (attendu : {list})", "“{v}” (expected: {list})"),
                ))
            }
        }
        Kind::List => match &node.v {
            N::Seq(items) => items
                .iter()
                .map(text_of)
                .filter(|t| !matches!(t, Ok(s) if s.is_empty()))
                .collect::<Result<Vec<_>, _>>()
                .map(|v| json!(v)),
            N::Scalar { null: true, .. } => Ok(json!([])),
            N::Scalar { .. } => Ok(json!([text_of(node)?])),
            _ => Err((
                node.line,
                tr!("une liste est attendue", "a list is expected"),
            )),
        },
        Kind::Steps | Kind::Launch => items(k, node, warnings),
        Kind::PluginIds => {
            let list: Vec<&Node> = match &node.v {
                N::Seq(items) => items.iter().collect(),
                N::Scalar { null: true, .. } => Vec::new(),
                N::Scalar { .. } => vec![node],
                _ => {
                    return Err((
                        node.line,
                        tr!("une liste est attendue", "a list is expected"),
                    ))
                }
            };
            let mut ids: Vec<String> = Vec::new();
            for item in list {
                let id = text_of(item).unwrap_or_default();
                if crate::plugins::manifest::is_id(&id) {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                    continue;
                }
                let path = k.path;
                warnings.push(warn(
                    Some(item.line),
                    tr!(
                        "valeur invalide pour {path} : « {id} » n'est pas l'identifiant d'un plugin, comme dev.exemple.slack (cette entrée est ignorée)",
                        "invalid value for {path}: “{id}” is not a plugin's id, such as dev.exemple.slack (this item is ignored)"
                    ),
                ));
            }
            Ok(json!(ids))
        }
        Kind::Dir => rel_dir(node).map(Value::String),
        Kind::Prefix => {
            let p = text_of(node)?;
            match crate::git::branch_prefix_problem(&p) {
                Some(why) => Err((node.line, why)),
                None => Ok(json!(p)),
            }
        }
    }
}

// ---------- a project's values ----------

fn steps_value(steps: &[WorktreeStep]) -> Value {
    Value::Array(
        steps
            .iter()
            .map(|s| json!({ "command": s.command.trim(), "shell": s.shell, "cwd": s.cwd.trim() }))
            .collect(),
    )
}

fn launch_value(cmds: &[RunCommand]) -> Value {
    Value::Array(
        cmds.iter()
            .map(|c| json!({ "name": c.name.trim(), "command": c.command.trim(), "shell": c.shell, "cwd": c.cwd.trim() }))
            .collect(),
    )
}

fn str_of(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn field(v: &Value, f: &str) -> String {
    v.get(f).map(str_of).unwrap_or_default()
}

/// A key's value in the project, as the file writes it.
pub fn get(p: &Project, key: &str) -> Value {
    let b = &p.board;
    match key {
        "agents.engine" => json!(p.engine().as_str()),
        PRIVATE => json!(p.private),
        "worktrees.per_agent" => json!(p.worktree_per_agent),
        "worktrees.at_first_message" => json!(p.worktree_at_first_message),
        "worktrees.base" => json!(p.worktree_base.to_text()),
        "worktrees.dir" => json!(p.worktree_dir),
        "worktrees.copy" => json!(p.worktree_copy),
        "worktrees.setup" => steps_value(&p.worktree_setup),
        "worktrees.teardown" => steps_value(&p.worktree_teardown),
        "launch" => launch_value(&p.run_commands),
        "git.agent_branch_prefix" => json!(p.agent_branch_prefix),
        "git.ticket_branch_prefix" => json!(p.ticket_branch_prefix),
        "git.commit_mode" => json!(match p.commit_mode {
            CommitMode::Agent => "agent",
            CommitMode::Direct => "direct",
        }),
        "git.conventional_commits" => json!(b.conventional),
        "board.on_approve" => json!(b.action),
        "board.target_branch" => json!(b.target),
        "board.merge_strategy" => json!(b.strategy),
        "board.draft_pr" => json!(b.draft),
        "board.test_command" => json!(b.test_command),
        "board.tests_before_approve" => json!(b.tests_first),
        "board.on_conflict" => json!(b.conflict),
        "board.remove_worktree" => json!(b.cleanup),
        "board.model" => json!(b.model),
        "board.effort" => json!(b.effort),
        MODE => json!(b.mode),
        _ => Value::Null,
    }
}

/// Steps from the file, each keeping the id of the project's step at its place (a step's id is
/// only the window's handle on it).
fn steps_from(v: &Value, old: &[WorktreeStep]) -> Vec<WorktreeStep> {
    v.as_array()
        .map(|a| {
            a.iter()
                .enumerate()
                .map(|(i, s)| WorktreeStep {
                    id: old
                        .get(i)
                        .map(|o| o.id.clone())
                        .unwrap_or_else(crate::model::new_id),
                    command: field(s, "command"),
                    shell: field(s, "shell"),
                    cwd: field(s, "cwd"),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Launch commands from the file, each keeping the id of the project's command of the same name
/// (its launches running are known by it).
fn launch_from(v: &Value, old: &[RunCommand]) -> Vec<RunCommand> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|c| {
                    let name = field(c, "name");
                    RunCommand {
                        id: old
                            .iter()
                            .find(|o| o.name.trim() == name)
                            .map(|o| o.id.clone())
                            .unwrap_or_else(crate::model::new_id),
                        name,
                        command: field(c, "command"),
                        shell: field(c, "shell"),
                        cwd: field(c, "cwd"),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Sets a key's value in the project.
pub fn set(p: &mut Project, key: &str, v: &Value) {
    let b = &mut p.board;
    let flag = v.as_bool().unwrap_or_default();
    match key {
        "agents.engine" => {
            p.engine = serde_json::from_value::<EngineId>(v.clone())
                .ok()
                .filter(|e| *e != EngineId::Claude)
        }
        PRIVATE => p.private = flag,
        "worktrees.per_agent" => p.worktree_per_agent = flag,
        "worktrees.at_first_message" => p.worktree_at_first_message = flag,
        "worktrees.base" => p.worktree_base = WorktreeBase::from_text(&str_of(v)),
        "worktrees.dir" => p.worktree_dir = str_of(v),
        "worktrees.copy" => {
            p.worktree_copy = serde_json::from_value(v.clone()).unwrap_or_default();
        }
        "worktrees.setup" => p.worktree_setup = steps_from(v, &p.worktree_setup),
        "worktrees.teardown" => p.worktree_teardown = steps_from(v, &p.worktree_teardown),
        "launch" => p.run_commands = launch_from(v, &p.run_commands),
        "git.agent_branch_prefix" => p.agent_branch_prefix = str_of(v),
        "git.ticket_branch_prefix" => p.ticket_branch_prefix = str_of(v),
        "git.commit_mode" => {
            p.commit_mode = if str_of(v) == "direct" {
                CommitMode::Direct
            } else {
                CommitMode::Agent
            }
        }
        "git.conventional_commits" => b.conventional = flag,
        "board.on_approve" => b.action = str_of(v),
        "board.target_branch" => b.target = str_of(v),
        "board.merge_strategy" => b.strategy = str_of(v),
        "board.draft_pr" => b.draft = flag,
        "board.test_command" => b.test_command = str_of(v),
        "board.tests_before_approve" => b.tests_first = flag,
        "board.on_conflict" => b.conflict = str_of(v),
        "board.remove_worktree" => b.cleanup = flag,
        "board.model" => b.model = str_of(v),
        "board.effort" => b.effort = str_of(v),
        MODE => b.mode = str_of(v),
        _ => {}
    }
}

// ---------- what the project knows of its file ----------

/// Commands of the file the user has not trusted yet: inactive until then.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Pending {
    /// Their fingerprint: what « Faire confiance à ces commandes » trusts, no other.
    pub hash: String,
    /// By key (`worktrees.setup`, `launch`…), as the file writes them.
    pub commands: BTreeMap<String, Value>,
}

/// The file as the project last read it (never saved: read again at each start).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SharedFile {
    /// The keys it sets.
    pub keys: Vec<String>,
    /// What the window says of each key (`get`): those kept on this machine show their file's value.
    pub values: BTreeMap<String, Value>,
    pub warnings: Vec<Warning>,
    /// Why it could not be read at all.
    pub error: Option<Warning>,
    pub version: u32,
    pub pending: Option<Pending>,
}

/// A project's own record of its file: saved with it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectShared {
    /// The keys of the file set otherwise on this machine: their own value goes first.
    pub local: Vec<String>,
    /// The fingerprint of the file's commands the user trusted (`Pending::hash`).
    pub trusted: String,
    /// The file as last read: None while the project has none.
    #[serde(skip_deserializing)]
    pub file: Option<SharedFile>,
    /// The fingerprint of the text last read ("" for no file): a file the same is not read again.
    #[serde(skip)]
    pub seen: Option<String>,
}

/// The fingerprint of commands, whatever their order in the file.
pub fn fingerprint(commands: &BTreeMap<String, Value>) -> String {
    let text = serde_json::to_string(commands).unwrap_or_default();
    let digest = Sha256::digest(text.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// What reading the file did to the project.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Applied {
    /// The keys whose value changed.
    pub changed: Vec<String>,
}

/// Applies what was read to the project: every key the file sets, but those kept on this machine;
/// its commands only once trusted (or when the project has them already, nothing new to run).
/// Records the file in `p.shared.file`.
pub fn apply(p: &mut Project, read: &Read) -> Applied {
    let mut changed = Vec::new();
    let mut warnings = read.warnings.clone();
    let mut commands = BTreeMap::new();
    for (k, v) in &read.values {
        // The plugins' keys stay the file's: the window and the plugins read them there.
        if p.shared.local.contains(k) || file_only(k) {
            continue;
        }
        if key_of(k).is_some_and(|k| k.command) {
            commands.insert(k.clone(), v.clone());
            continue;
        }
        // A project private on this machine stays so: the file may make it private, never public
        // (its agents and files would be seen by the other projects' agents here). Kept on this
        // machine, said.
        if k == PRIVATE && p.private && v == &json!(false) {
            p.shared.local.push(k.clone());
            warnings.push(warn(
                None,
                tr!(
                    "{PRIVATE}: false ignoré : ce projet est privé sur cette machine, décoche « Projet privé » pour le rendre public",
                    "{PRIVATE}: false ignored: this project is private on this machine, untick “Private project” to make it public"
                ),
            ));
            continue;
        }
        if k == "worktrees.dir" {
            if let Some(why) = crate::paths::worktrees_root_problem(&p.path, &str_of(v)) {
                warnings.push(warn(None, format!("worktrees.dir : {why}")));
                continue;
            }
        }
        if get(p, k) != *v {
            set(p, k, v);
            changed.push(k.clone());
        }
    }
    let mut pending = None;
    if !commands.is_empty() {
        let hash = fingerprint(&commands);
        // Exactly the project's own, shell and folder too: nothing new would run.
        let known = commands.iter().all(|(k, v)| get(p, k) == *v);
        if hash == p.shared.trusted || known {
            for (k, v) in &commands {
                if get(p, k) != *v {
                    set(p, k, v);
                    changed.push(k.clone());
                }
            }
            p.shared.trusted = hash;
        } else {
            pending = Some(Pending { hash, commands });
        }
    }
    p.shared.file = Some(SharedFile {
        keys: read.values.keys().cloned().collect(),
        values: read.values.clone(),
        warnings,
        error: None,
        version: read.version,
        pending,
    });
    Applied { changed }
}

/// The keys of the file whose value differs between `before` and `after` (the window set them):
/// kept on this machine from now on, ahead of the file.
pub fn keep_local(before: &Project, after: &mut Project) -> Vec<String> {
    let Some(file) = before.shared.file.clone() else {
        return Vec::new();
    };
    let mut kept = Vec::new();
    // The window turns « tests before approval » off while there is no test command (the file's waits
    // for the user's trust, say): no choice of the user's, the file's value stays.
    if after.board.test_command.trim().is_empty() && !after.board.tests_first {
        after.board.tests_first = before.board.tests_first;
    }
    for k in &file.keys {
        if after.shared.local.contains(k) {
            continue;
        }
        if get(before, k) != get(after, k) {
            after.shared.local.push(k.clone());
            kept.push(k.clone());
        }
    }
    kept
}

// ---------- writing ----------

fn scalar_yaml(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

/// The text `s` as a plain scalar of the file, when YAML reads it back as `s` unquoted. yaml-edit
/// quotes more than YAML needs (a `*` anywhere, for one) and takes `a: b` for plain, so the text
/// is checked here, then read back by the parser, which keeps the node as it is written.
fn plain_node(s: &str) -> Option<YamlNode> {
    if !plain_safe(s) {
        return None;
    }
    let doc = Document::from_str(&format!("k: {s}\n")).ok()?;
    let node = doc.as_mapping()?.get("k")?;
    let YamlNode::Scalar(sc) = &node else {
        return None;
    };
    let value = ScalarValue::from_scalar(sc);
    (value.style() == ScalarStyle::Plain
        && value.scalar_type() == ScalarType::String
        && value.value() == s)
        .then_some(node)
}

/// Whether `s` may be written plain: no indicator where a plain scalar starts, no `: ` nor ` #`, no
/// flow indicator, no control character, no word another type reads as (`yes`, a number…).
pub(crate) fn plain_safe(s: &str) -> bool {
    const INDICATORS: &[char] = &[
        '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@',
        '`',
    ];
    const WORDS: &[&str] = &["true", "false", "yes", "no", "on", "off", "null", "~"];
    !s.is_empty()
        && !s.starts_with(INDICATORS)
        && !s.starts_with("---")
        && !s.starts_with("...")
        && !s.starts_with([' ', '\t'])
        && !s.ends_with([' ', '\t', ':'])
        && !s.contains(": ")
        && !s.contains(" #")
        && !s.contains(['[', ']', '{', '}', ','])
        && !s
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{FEFF}'))
        && !WORDS.iter().any(|w| s.eq_ignore_ascii_case(w))
        && s.parse::<f64>().is_err()
        && matches!(ScalarValue::classify_plain(s), CoreScalarType::String)
}

/// `s` as the value of `name`: plain where `plain_node` allows it, quoted as yaml-edit quotes it
/// otherwise.
fn put_text(m: &Mapping, name: &str, s: &str) {
    match plain_node(s) {
        Some(node) => m.set(name, node),
        None => m.set(name, ScalarValue::string(s)),
    }
}

/// `s` as an item of a list, as `put_text` writes it.
fn push_text(list: &Sequence, s: &str) {
    match plain_node(s) {
        Some(node) => list.push(node),
        None => list.push(ScalarValue::string(s)),
    }
}

/// An item of a list of steps or commands, without what is empty.
fn item_mapping(v: &Value) -> Mapping {
    let m = Mapping::new();
    if let Some(o) = v.as_object() {
        for f in ["name", "command", "shell", "cwd"] {
            let s = o.get(f).map(str_of).unwrap_or_default();
            if !s.is_empty() {
                put_text(&m, f, &s);
            }
        }
    }
    m
}

/// A list set under a key the mapping does not have yet comes out with its items at the key's
/// column; set in place of a value the key already holds, it is indented two spaces, as Prettier
/// writes it. So a new key takes a placeholder first, which the list then replaces in place.
fn put_list(m: &Mapping, name: &str, list: Sequence) {
    if !m.contains_key(name) {
        m.set(name, "");
    }
    m.set(name, list);
}

fn set_value(m: &Mapping, name: &str, k: &Key, v: &Value) {
    match k.kind {
        Kind::Bool => m.set(name, v.as_bool().unwrap_or_default()),
        Kind::List | Kind::PluginIds => {
            let s = Sequence::new();
            for item in v.as_array().into_iter().flatten() {
                push_text(&s, &scalar_yaml(item));
            }
            put_list(m, name, s);
        }
        Kind::Steps | Kind::Launch => {
            let s = Sequence::new();
            for item in v.as_array().into_iter().flatten() {
                s.push(item_mapping(item));
            }
            put_list(m, name, s);
        }
        _ => put_text(m, name, &scalar_yaml(v)),
    }
}

/// The file with the keys given set to their values, written over `existing` (its comments, its
/// order and the keys this app does not know kept), or a new one.
pub fn write(existing: Option<&str>, values: &[(&str, Value)]) -> Result<String> {
    let header = format!(
        "# yaml-language-server: $schema={SCHEMA_URL}\n# {}\nversion: {VERSION}\n",
        tr!(
            "Réglages d'Escouade partagés par l'équipe du projet : https://escouade.dev/docs/escouade-yml",
            "Escouade settings shared by the project's team: https://escouade.dev/en/docs/escouade-yml"
        )
    );
    let text = match existing {
        Some(t) if !t.trim().is_empty() => t.to_string(),
        _ => header,
    };
    match read(&text) {
        Err(e) => bail!(e.text),
        Ok(r) if r.version > VERSION => bail!(tr!(
            "{FILE} est fait pour une version plus récente d'Escouade : modifie-le à la main",
            "{FILE} is made for a newer version of Escouade: edit it by hand"
        )),
        Ok(_) => {}
    }
    let doc = Document::from_str(&text).map_err(|e| anyhow::anyhow!("{e}"))?;
    let root = doc
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("{FILE}: no mapping"))?;
    if !root.contains_key("version") {
        root.insert_at_index(0, "version", VERSION as i64);
    }
    for (path, v) in values {
        let Some(k) = key_of(path) else { continue };
        match path.split_once('.') {
            None => set_value(&root, path, k, v),
            Some((group, name)) => {
                if root.get_mapping(group).is_none() {
                    // A group new to the file starts with a placeholder for its first key (see
                    // `put_list`), so that its values are set in place, as in a group the file has.
                    let g = Mapping::new();
                    g.set(name, "");
                    root.set(group, g);
                }
                if let Some(g) = root.get_mapping(group) {
                    set_value(&g, name, k, v);
                }
            }
        }
        // Written, the key it took the place of goes (read only while it was the file's).
        if *path == BASE {
            let (group, name) = FROM_TARGET.split_once('.').unwrap_or_default();
            if let Some(g) = root.get_mapping(group) {
                g.remove(name);
            }
        }
    }
    let mut out = doc.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

/// Whether the project's value of a key goes into the file: not an empty target branch (set by the
/// first ticket), nor `bypassPermissions` (`MODES`).
fn writable(k: &str, v: &Value) -> bool {
    !(k == "board.target_branch" && v.as_str() == Some("")) && !(k == MODE && v == &json!(BYPASS))
}

/// Every key, as the project has them: what « Exporter vers .escouade.yml » writes (but what is not
/// `writable`).
pub fn export_values(p: &Project) -> Vec<(&'static str, Value)> {
    KEYS.iter()
        .filter(|k| !k.file_only)
        .map(|k| (k.path, get(p, k.path)))
        .filter(|(k, v)| writable(k, v))
        .collect()
}

// ---------- the schema ----------

#[cfg_attr(not(test), allow(dead_code))]
fn schema_of(k: &Key) -> Value {
    let item = |launch: bool| {
        let mut props = Map::new();
        if launch {
            props.insert(
                "name".into(),
                json!({ "type": "string", "description": "As the window shows it." }),
            );
        }
        props.insert("command".into(), json!({ "type": "string" }));
        props.insert(
            "shell".into(),
            json!({ "type": "string", "description": "pwsh, powershell, bash, zsh, wsl…; the system's default one when empty or not found." }),
        );
        props.insert(
            "cwd".into(),
            json!({ "type": "string", "description": "A folder relative to the repository (or the worktree)." }),
        );
        json!({
            "type": "object",
            "properties": props,
            "required": if launch { json!(["name", "command"]) } else { json!(["command"]) },
            "additionalProperties": false,
        })
    };
    let mut s = match k.kind {
        Kind::Bool => json!({ "type": "boolean" }),
        Kind::Text | Kind::Dir | Kind::Prefix => json!({ "type": "string" }),
        Kind::Enum(values) => json!({ "type": "string", "enum": values }),
        // A single value is a list of one: the schema says both forms.
        Kind::List => json!({
            "oneOf": [{ "type": "array", "items": { "type": "string" } }, { "type": "string" }],
        }),
        Kind::Steps => {
            let step = json!({ "oneOf": [{ "type": "string" }, item(false)] });
            json!({ "oneOf": [{ "type": "array", "items": step }, step] })
        }
        Kind::Launch => json!({ "oneOf": [{ "type": "array", "items": item(true) }, item(true)] }),
        Kind::PluginIds => {
            let id = json!({ "type": "string", "pattern": PLUGIN_ID_PATTERN });
            json!({ "oneOf": [{ "type": "array", "items": id, "uniqueItems": true }, id] })
        }
    };
    s["description"] = json!(k.about);
    s
}

/// A plugin's id (`plugins::manifest::is_id`): a reversed domain name in lower case, of two words
/// at least.
#[cfg_attr(not(test), allow(dead_code))]
const PLUGIN_ID_PATTERN: &str =
    "^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\\.[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)+$";

/// The JSON schema of the file (`schemas/escouade.schema.json`), made from `KEYS`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn schema() -> Value {
    let mut props = Map::new();
    props.insert(
        "version".into(),
        json!({ "type": "integer", "minimum": 1, "description": format!("The version of this file's format ({VERSION} today).") }),
    );
    for g in groups() {
        match key_of(g) {
            Some(k) => {
                props.insert(g.into(), schema_of(k));
            }
            None => {
                let mut inner = Map::new();
                for k in KEYS {
                    if let Some(name) = k.path.strip_prefix(&format!("{g}.")) {
                        inner.insert(name.into(), schema_of(k));
                    }
                }
                // The former key, still read: the files that have it stay valid.
                if let Some(name) = FROM_TARGET.strip_prefix(&format!("{g}.")) {
                    inner.insert(
                        name.into(),
                        json!({ "type": "boolean", "deprecated": true, "description": format!("Replaced by {BASE}: true reads as target, false as head.") }),
                    );
                }
                let mut group =
                    json!({ "type": "object", "properties": inner, "additionalProperties": false });
                // Each plugin's settings, under its id: simple values, never a secret.
                if g == PLUGINS {
                    let mut by_id = Map::new();
                    by_id.insert(
                        PLUGIN_ID_PATTERN.into(),
                        json!({
                            "type": "object",
                            "description": "The settings of the plugin of this id, by the keys of its contributes.settings: applied to it in this project once it is installed and on. Never a secret: those go to the system's keychain.",
                            "additionalProperties": { "type": ["string", "number", "boolean"] },
                        }),
                    );
                    group["patternProperties"] = Value::Object(by_id);
                }
                props.insert(g.into(), group);
            }
        }
    }
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": SCHEMA_URL,
        "title": ".escouade.yml",
        "description": "The Escouade settings a project's team shares through its repository.",
        "type": "object",
        "properties": props,
        "required": ["version"],
        "additionalProperties": false,
    })
}

// ---------- in the app ----------

use crate::core::{project_not_found, Core};
use crate::model::UiEvent;
use std::sync::Arc;
use tauri::Runtime;

/// The file of the project's folder: None when there is none (or it cannot be opened).
fn file_text(path: &str) -> Option<String> {
    match std::fs::read_to_string(std::path::Path::new(path).join(FILE)) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            log::warn!("{FILE} of {path} not read: {e}");
            None
        }
    }
}

fn text_fingerprint(text: Option<&str>) -> String {
    text.map(|t| {
        Sha256::digest(t.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    })
    .unwrap_or_default()
}

/// Reads the file into the project: what it sets applied, or why it cannot be read.
fn read_into(p: &mut Project, text: Option<&str>) -> Applied {
    p.shared.seen = Some(text_fingerprint(text));
    match text.map(read) {
        None => {
            p.shared.file = None;
            Applied::default()
        }
        Some(Ok(r)) => apply(p, &r),
        Some(Err(w)) => {
            p.shared.file = Some(SharedFile {
                error: Some(w),
                version: VERSION,
                ..Default::default()
            });
            Applied::default()
        }
    }
}

/// A project as `blank_project` makes it, with the file of its folder applied: what the new
/// project's assistant starts from (its commands never: they wait for the project).
pub fn preview(path: &str) -> Option<Project> {
    let path = path.trim().trim_end_matches(['\\', '/']);
    let text = file_text(path)?;
    let mut p = crate::core::blank_project(path);
    read_into(&mut p, Some(&text));
    Some(p)
}

impl<R: Runtime> Core<R> {
    /// Reads the project's `.escouade.yml` if it changed since last read (always when `opened`, the
    /// project just added), and applies it; says what it did. The project, when anything changed.
    pub fn read_shared(self: &Arc<Self>, project_id: &str, opened: bool) -> Option<Project> {
        let path = self.project(project_id).ok()?.path;
        let text = file_text(&path);
        let (before, after, applied) = {
            let mut projects = self.projects.write();
            let p = projects.iter_mut().find(|p| p.id == project_id)?;
            if !opened && p.shared.seen.as_deref() == Some(&text_fingerprint(text.as_deref())) {
                return None;
            }
            let before = p.clone();
            let applied = read_into(p, text.as_deref());
            (before, p.clone(), applied)
        };
        let file = after.shared.file.as_ref();
        let was = before.shared.file.as_ref();
        let name = &after.name;
        // Read at the start of the app (`seen` empty): only what changed since is said.
        let first = before.shared.seen.is_none() && !opened;
        let pending = file.and_then(|f| f.pending.as_ref());
        let pending_new = pending.is_some_and(|n| {
            was.and_then(|f| f.pending.as_ref())
                .is_none_or(|o| o.hash != n.hash)
        });
        let error = file.and_then(|f| f.error.as_ref());
        let toast = |text: String| self.hub.emit(UiEvent::Toast { text });
        if let Some(e) = error {
            if opened || was.and_then(|f| f.error.as_ref()) != Some(e) {
                let at = e.line.map(|l| format!(" ({l})")).unwrap_or_default();
                toast(format!("{name} · {}{at}", e.text));
            }
        } else if opened && file.is_some() {
            // The plugins it proposes that this machine lacks (PLG-14): named in Settings › Project.
            let proposed = file
                .and_then(|f| f.values.get(RECOMMENDED))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|id| self.plugins.get(id, crate::i18n::Lang::En).is_none())
                .count();
            if proposed > 0 {
                toast(tr_n!(
                    proposed,
                    "{name} propose {proposed} plugin : Réglages › Projets.",
                    "{name} propose {proposed} plugins : Réglages › Projets.",
                    "{name} proposes {proposed} plugin: Settings › Projects.",
                    "{name} proposes {proposed} plugins: Settings › Projects."
                ));
            }
            toast(if pending.is_some() {
                tr!(
                    "{name} : réglages lus dans {FILE}. Ses commandes attendent ton accord (Réglages › Projets).",
                    "{name}: settings read from {FILE}. Its commands wait for your approval (Settings › Projects)."
                )
            } else {
                tr!(
                    "{name} : réglages lus dans {FILE}.",
                    "{name}: settings read from {FILE}."
                )
            });
        } else if !opened {
            let n = applied.changed.len();
            if n > 0 {
                toast(tr_n!(
                    n,
                    "{name} : {FILE} relu, {n} réglage appliqué.",
                    "{name} : {FILE} relu, {n} réglages appliqués.",
                    "{name}: {FILE} read again, {n} setting applied.",
                    "{name}: {FILE} read again, {n} settings applied."
                ));
            }
            if pending_new && !first {
                toast(tr!(
                    "{name} : les commandes de {FILE} ont changé et attendent ton accord (Réglages › Projets).",
                    "{name}: the commands of {FILE} changed and wait for your approval (Settings › Projects)."
                ));
            }
        }
        let mut same = before.clone();
        same.shared.seen = after.shared.seen.clone();
        if serde_json::to_value(&same).ok() == serde_json::to_value(&after).ok() {
            return None;
        }
        self.hub.emit(UiEvent::Project {
            project: after.clone(),
        });
        self.request_save();
        if before.worktree_dir != after.worktree_dir {
            self.watch_project(&after);
        }
        if !applied.changed.is_empty() {
            self.schedule();
        }
        Some(after)
    }

    /// « Faire confiance à ces commandes »: the file's commands, if they are still those the user
    /// read (`hash`), apply and run from now on.
    pub fn trust_shared(self: &Arc<Self>, project_id: &str, hash: &str) -> Result<Project> {
        self.read_shared(project_id, false);
        {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(project_not_found)?;
            let now = p
                .shared
                .file
                .as_ref()
                .and_then(|f| f.pending.as_ref())
                .map(|x| x.hash.clone());
            if now.as_deref() != Some(hash) {
                bail!(tr!(
                    "Les commandes de {FILE} ont changé depuis : relis-les.",
                    "The commands of {FILE} changed since: read them again."
                ));
            }
            p.shared.trusted = hash.to_string();
            p.shared.seen = None;
        }
        self.read_shared(project_id, false);
        self.project(project_id)
    }

    /// Writes the keys given (all of them: « Exporter vers .escouade.yml ») with the project's
    /// values into its file: they are no longer kept apart on this machine, and the commands written
    /// are trusted (they are the project's).
    pub fn write_shared(
        self: &Arc<Self>,
        project_id: &str,
        keys: Option<Vec<String>>,
    ) -> Result<Project> {
        let p = self.project(project_id)?;
        let values: Vec<(&str, Value)> = match &keys {
            None => export_values(&p),
            Some(keys) => keys
                .iter()
                .filter_map(|k| key_of(k))
                .map(|k| (k.path, get(&p, k.path)))
                // What is not written stays kept on this machine (`forget_kept` below).
                .filter(|(k, v)| writable(k, v))
                .collect(),
        };
        let text = write(file_text(&p.path).as_deref(), &values)?;
        std::fs::write(std::path::Path::new(&p.path).join(FILE), text)?;
        self.forget_kept(project_id, |k| values.iter().any(|(w, _)| *w == k))
    }

    /// The window set `n` keys of the file otherwise: said, and the window told which.
    pub(crate) fn kept_on_machine(&self, p: &Project, n: usize) {
        self.hub.emit(UiEvent::Project { project: p.clone() });
        self.hub.emit(UiEvent::Toast {
            text: tr_n!(
                n,
                "Réglage gardé sur cette machine : il passe devant {FILE}. Réglages › Projets peut l'écrire dans le fichier.",
                "{n} réglages gardés sur cette machine : ils passent devant {FILE}. Réglages › Projets peut les écrire dans le fichier.",
                "Setting kept on this machine: it goes before {FILE}. Settings › Projects can write it into the file.",
                "{n} settings kept on this machine: they go before {FILE}. Settings › Projects can write them into the file."
            ),
        });
    }

    /// « Reprendre la valeur du fichier »: the key is no longer kept apart on this machine.
    pub fn unkeep_shared(self: &Arc<Self>, project_id: &str, key: &str) -> Result<Project> {
        self.forget_kept(project_id, |k| k == key)
    }

    /// The keys `gone` says are no longer kept apart: the file read again applies them.
    fn forget_kept(
        self: &Arc<Self>,
        project_id: &str,
        gone: impl Fn(&str) -> bool,
    ) -> Result<Project> {
        {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(project_not_found)?;
            p.shared.local.retain(|k| !gone(k));
            p.shared.seen = None;
        }
        self.request_save();
        // Told to the window even when nothing else changed: what it marks did.
        if self.read_shared(project_id, false).is_none() {
            let p = self.project(project_id)?;
            self.hub.emit(UiEvent::Project { project: p });
        }
        self.project(project_id)
    }
}
