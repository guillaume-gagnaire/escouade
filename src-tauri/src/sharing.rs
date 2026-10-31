//! The shared Kanban (ESC-190): a ticket shared with the team is a file of its own,
//! `.escouade/tickets/<id>.yml`, in the project's folder. Escouade writes it and keeps its status up
//! to date; the user commits and pushes it with their work, never Escouade; the others get it at
//! their pull, and Escouade reads it again whenever the folder changes.
//!
//! `FIELDS` is the one list of what a file holds: the reader, the writer and the JSON schema
//! (`schema`, published as `schemas/escouade-ticket.schema.json`) all go by it. `reconcile` brings
//! the tickets of a project and the files of its folder in step, by the fingerprint of each file as
//! last read or written (`Shared::seen`): a file that changed outside Escouade is read, one that did
//! not is written again when its ticket says something else.

use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use yaml_edit::{Document, YamlNode};

use crate::i18n::Lang;
use crate::model::*;
use crate::projectfile::{node_of, plain_safe, Node, N};

/// Where the files are, in the project's folder (or in its first repository, ESC-169).
pub const DIR: &str = ".escouade/tickets";
/// The version this app writes, and reads as it is.
pub const VERSION: u32 = 1;
/// Where the site serves the schema (`schemas/escouade-ticket.schema.json` of the repository).
pub const SCHEMA_URL: &str = "https://escouade.dev/schemas/escouade-ticket.schema.json";
/// A joined file above this size stays on its machine (choice 107); images always go.
pub const MAX_SHARED_FILE: u64 = 5 * 1024 * 1024;
/// The letters of the ids: lower case and digits, without 0/o nor 1/l.
const ALPHABET: &[u8] = b"23456789abcdefghijkmnpqrstuvwxyz";

/// The statuses of a file, in the board's order.
pub const STATUSES: [&str; 4] = ["todo", "in-progress", "review", "done"];

/// What a field's value is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Version,
    Text,
    Status,
    Person,
    /// A person, or `null`.
    MaybePerson,
    /// Ids of other tickets' files.
    Ids,
    Texts,
    /// `{ name, file }` items.
    Media,
    /// `{ source, key, url }`, or `null`.
    External,
    /// A text, or `null`.
    MaybeText,
}

/// A field of the file, in the order it is written.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub name: &'static str,
    pub kind: Kind,
    pub required: bool,
    #[cfg_attr(not(test), allow(dead_code))]
    pub about: &'static str,
}

const fn field(name: &'static str, kind: Kind, required: bool, about: &'static str) -> Field {
    Field {
        name,
        kind,
        required,
        about,
    }
}

pub const FIELDS: &[Field] = &[
    field(
        "version",
        Kind::Version,
        true,
        "The version of this file's format (1 today).",
    ),
    field(
        "id",
        Kind::Text,
        true,
        "The ticket's stable id: the file's name, never its key.",
    ),
    field(
        "key",
        Kind::Text,
        true,
        "Its key, as its author's board gave it (DEM-12).",
    ),
    field("title", Kind::Text, true, "Its title."),
    field(
        "status",
        Kind::Status,
        true,
        "Its column: todo, in-progress, review or done.",
    ),
    field("author", Kind::Person, true, "Who shared it: { name, id }."),
    field(
        "taken_by",
        Kind::MaybePerson,
        false,
        "Who started it, or null.",
    ),
    field(
        "updated_by",
        Kind::MaybePerson,
        false,
        "Who last changed this file.",
    ),
    field("created", Kind::Text, false, "When it was made (ISO 8601)."),
    field(
        "updated",
        Kind::Text,
        false,
        "When this file last changed (ISO 8601).",
    ),
    field(
        "after",
        Kind::Ids,
        false,
        "The ids of the shared tickets it comes after.",
    ),
    field("description", Kind::Text, false, "Its description."),
    field("criteria", Kind::Texts, false, "Its acceptance criteria."),
    field(
        "images",
        Kind::Media,
        false,
        "Its images, in the folder of its id beside this file.",
    ),
    field(
        "files",
        Kind::Media,
        false,
        "Its joined files up to 5 MB, in the same folder.",
    ),
    field(
        "external",
        Kind::External,
        false,
        "The outside ticket it was imported from: { source, key, url }, or null.",
    ),
    field(
        "outcome",
        Kind::MaybeText,
        false,
        "What its validation did, once done.",
    ),
];

/// A shared ticket's file, as read or to be written.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TicketDoc {
    pub version: u32,
    pub id: String,
    pub key: String,
    pub title: String,
    pub status: Column,
    pub author: Person,
    pub taken_by: Option<Person>,
    pub updated_by: Option<Person>,
    pub created: String,
    pub updated: String,
    pub after: Vec<String>,
    pub description: String,
    pub criteria: Vec<String>,
    pub images: Vec<SharedMedia>,
    pub files: Vec<SharedMedia>,
    pub external: Option<SharedExternal>,
    pub outcome: Option<String>,
}

impl TicketDoc {
    /// The same ticket, whoever wrote it last and when: whether writing `self` would change
    /// anything that matters in a file that says `other`.
    pub fn same(&self, other: &TicketDoc) -> bool {
        let strip = |f: &TicketDoc| TicketDoc {
            updated: String::new(),
            updated_by: None,
            version: 0,
            ..f.clone()
        };
        strip(self) == strip(other)
    }
}

/// What reading a file gives.
#[derive(Debug, Clone, PartialEq)]
pub enum Read {
    /// Read; `newer`: written by a newer Escouade (what this one knows of it is read).
    Ok(Box<TicketDoc>, bool),
    /// Git's conflict markers are in it.
    Conflict,
    /// It cannot be read: the line in cause (from 1) if known, and why.
    Invalid(Option<u32>, String),
}

pub fn status_name(c: Column) -> &'static str {
    match c {
        Column::Todo => STATUSES[0],
        Column::Doing => STATUSES[1],
        Column::Review => STATUSES[2],
        Column::Done => STATUSES[3],
    }
}

pub fn column_of(status: &str) -> Option<Column> {
    Some(match status {
        "todo" => Column::Todo,
        "in-progress" => Column::Doing,
        "review" => Column::Review,
        "done" => Column::Done,
        _ => return None,
    })
}

/// A new id: 8 letters of `ALPHABET`, from the system's random generator.
pub fn new_share_id() -> String {
    let mut bytes = [0u8; 8];
    if getrandom::fill(&mut bytes).is_err() {
        // Never seen; the time then, which is enough for one machine.
        let n = now_ms().to_le_bytes();
        bytes.copy_from_slice(&n);
    }
    bytes
        .iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect()
}

/// The id of the person whose git e-mail is `email`: 8 hex digits of its fingerprint, the e-mail
/// itself written nowhere. The same on all their machines. None without an e-mail.
pub fn person_id(email: &str) -> Option<String> {
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return None;
    }
    let hash = Sha256::digest(email.as_bytes());
    Some(hash.iter().take(4).map(|b| format!("{b:02x}")).collect())
}

/// The fingerprint of a file's text, as `Shared::seen` keeps it.
pub fn fingerprint(text: &str) -> String {
    let hash = Sha256::digest(text.as_bytes());
    hash.iter().take(12).map(|b| format!("{b:02x}")).collect()
}

/// A file name of the folder, its id when it is one this app writes: `<id>.yml`, the id plain.
pub fn id_of_file(name: &str) -> Option<&str> {
    let id = name.strip_suffix(".yml")?;
    plain_id(id).then_some(id)
}

/// An id that is safe as a file's name: letters, digits, `-` and `_`, 40 at most, and not a name
/// Windows keeps for its devices (`con`, `aux`, `com1`…: a file of a teammate named so is never opened).
pub fn plain_id(id: &str) -> bool {
    !id.is_empty()
        && !crate::workflowfiles::reserved_stem(&id.to_ascii_lowercase())
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A name of an image or file of the folder: one segment (or `<uniq>/<name>` for a file), no way up.
pub fn plain_media(file: &str) -> bool {
    !file.is_empty()
        && file.split('/').count() <= 2
        && file.split('/').all(|seg| {
            !seg.is_empty()
                && seg != "."
                && seg != ".."
                && !seg.contains(['\\', ':'])
                && crate::attachfiles::safe_name(seg) == seg
        })
}

// ---------- reading ----------

fn invalid(line: Option<u32>, text: String) -> Read {
    Read::Invalid(line, text)
}

fn get<'a>(t: &'a Node, key: &str) -> Option<&'a Node> {
    match &t.v {
        N::Map(entries) => entries.iter().find(|(k, _, _)| k == key).map(|(_, _, n)| n),
        _ => None,
    }
}

fn is_null(n: &Node) -> bool {
    matches!(&n.v, N::Scalar { null: true, .. })
        || matches!(&n.v, N::Scalar { text, quoted: false, .. } if text == "~" || text.is_empty())
}

fn line(n: &Node) -> Option<u32> {
    (n.line > 0).then_some(n.line)
}

fn text(name: &str, n: &Node) -> Result<String, Read> {
    match &n.v {
        N::Scalar { text, .. } => Ok(text.trim_end_matches(['\n', '\r']).to_string()),
        _ => Err(invalid(
            line(n),
            tr!("{name} doit être un texte", "{name} must be a text"),
        )),
    }
}

fn person(name: &str, n: &Node) -> Result<Person, Read> {
    let bad = || {
        invalid(
            line(n),
            tr!(
                "{name} doit être {{ name, id }}",
                "{name} must be {{ name, id }}"
            ),
        )
    };
    if !matches!(n.v, N::Map(_)) {
        return Err(bad());
    }
    let who = Person {
        name: get(n, "name")
            .map(|v| text("name", v))
            .transpose()?
            .unwrap_or_default(),
        id: get(n, "id")
            .map(|v| text("id", v))
            .transpose()?
            .unwrap_or_default(),
    };
    if who.name.is_empty() && who.id.is_empty() {
        return Err(bad());
    }
    Ok(who)
}

fn texts(name: &str, n: &Node) -> Result<Vec<String>, Read> {
    match &n.v {
        N::Seq(items) => items.iter().map(|i| text(name, i)).collect(),
        _ if is_null(n) => Ok(Vec::new()),
        _ => Err(invalid(
            line(n),
            tr!("{name} doit être une liste", "{name} must be a list"),
        )),
    }
}

fn media(name: &str, n: &Node) -> Result<Vec<SharedMedia>, Read> {
    let N::Seq(items) = &n.v else {
        if is_null(n) {
            return Ok(Vec::new());
        }
        return Err(invalid(
            line(n),
            tr!("{name} doit être une liste", "{name} must be a list"),
        ));
    };
    let mut out = Vec::new();
    for i in items {
        let m = match &i.v {
            N::Scalar { text, .. } => SharedMedia {
                name: text.rsplit('/').next().unwrap_or_default().to_string(),
                file: text.clone(),
            },
            N::Map(_) => SharedMedia {
                name: get(i, "name").map(text_of).unwrap_or_default(),
                file: get(i, "file").map(text_of).unwrap_or_default(),
            },
            _ => SharedMedia::default(),
        };
        if !plain_media(&m.file) {
            return Err(invalid(
                line(i),
                tr!(
                    "{name} : « {} » n’est pas un fichier du dossier du ticket",
                    "{name}: “{}” is no file of the ticket’s folder",
                    m.file
                ),
            ));
        }
        let name = if m.name.is_empty() {
            m.file.rsplit('/').next().unwrap_or_default().to_string()
        } else {
            m.name
        };
        out.push(SharedMedia { name, file: m.file });
    }
    Ok(out)
}

fn text_of(n: &Node) -> String {
    match &n.v {
        N::Scalar { text, .. } => text.trim().to_string(),
        _ => String::new(),
    }
}

/// Whether git left its conflict markers in `text`.
pub fn has_conflict(text: &str) -> bool {
    let mut lines = text.lines();
    lines.any(|l| l.starts_with("<<<<<<<")) && text.lines().any(|l| l.starts_with(">>>>>>>"))
}

/// Reads a file: what it says, or why it cannot be read. A key it does not know is left out.
pub fn read(text: &str) -> Read {
    if has_conflict(text) {
        return Read::Conflict;
    }
    let doc = match Document::from_str(text) {
        Ok(doc) => doc,
        Err(yaml_edit::YamlError::Parse { message, line, .. }) => {
            return invalid(line.map(|l| l as u32), message)
        }
        Err(e) => return invalid(None, e.to_string()),
    };
    let Some(m) = doc.as_mapping() else {
        return invalid(
            Some(1),
            tr!(
                "le fichier doit être une suite de clés (version: 1…)",
                "the file must be a set of keys (version: 1…)"
            ),
        );
    };
    let t = node_of(text, &YamlNode::Mapping(m));
    match read_tree(&t) {
        Ok(read) => read,
        Err(e) => e,
    }
}

fn read_tree(t: &Node) -> Result<Read, Read> {
    for f in FIELDS.iter().filter(|f| f.required) {
        if get(t, f.name).is_none_or(is_null) {
            return Err(invalid(line(t), tr!("{} manque", "{} is missing", f.name)));
        }
    }
    let mut out = TicketDoc::default();
    let version = get(t, "version").map(text_of).unwrap_or_default();
    out.version = match version.parse::<u32>() {
        Ok(v) if v >= 1 => v,
        _ => {
            return Err(invalid(
                get(t, "version").and_then(line),
                tr!(
                    "version doit être un nombre (1)",
                    "version must be a number (1)"
                ),
            ))
        }
    };
    let newer = out.version > VERSION;
    for f in FIELDS {
        let Some(n) = get(t, f.name) else { continue };
        let name = f.name;
        match f.kind {
            Kind::Version => {}
            Kind::Text => {
                let v = if is_null(n) {
                    String::new()
                } else {
                    text(name, n)?
                };
                match name {
                    "id" => out.id = v,
                    "key" => out.key = v,
                    "title" => out.title = v,
                    "created" => out.created = v,
                    "updated" => out.updated = v,
                    "description" => out.description = v,
                    _ => {}
                }
            }
            Kind::Status => {
                let v = text(name, n)?;
                out.status = match column_of(&v) {
                    Some(c) => c,
                    // A status of a newer format: as « À faire », never written.
                    None if newer => Column::Todo,
                    None => {
                        return Err(invalid(
                            line(n),
                            tr!(
                                "status doit être todo, in-progress, review ou done",
                                "status must be todo, in-progress, review or done"
                            ),
                        ))
                    }
                };
            }
            Kind::Person => out.author = person(name, n)?,
            Kind::MaybePerson => {
                let v = if is_null(n) {
                    None
                } else {
                    Some(person(name, n)?)
                };
                match name {
                    "taken_by" => out.taken_by = v,
                    "updated_by" => out.updated_by = v,
                    _ => {}
                }
            }
            Kind::Ids => {
                out.after = texts(name, n)?;
                if let Some(bad) = out.after.iter().find(|id| !plain_id(id)) {
                    return Err(invalid(
                        line(n),
                        tr!(
                            "after : « {bad} » n’est pas un id",
                            "after: “{bad}” is no id"
                        ),
                    ));
                }
            }
            Kind::Texts => {
                out.criteria = texts(name, n)?
                    .into_iter()
                    .filter(|c| !c.is_empty())
                    .collect()
            }
            Kind::Media => {
                let v = media(name, n)?;
                if name == "images" {
                    out.images = v;
                } else {
                    out.files = v;
                }
            }
            Kind::External => {
                out.external = if is_null(n) {
                    None
                } else if matches!(n.v, N::Map(_)) {
                    Some(SharedExternal {
                        source: get(n, "source").map(text_of).unwrap_or_default(),
                        key: get(n, "key").map(text_of).unwrap_or_default(),
                        url: get(n, "url").map(text_of).unwrap_or_default(),
                    })
                } else {
                    return Err(invalid(
                        line(n),
                        tr!(
                            "external doit être {{ source, key, url }} ou null",
                            "external must be {{ source, key, url }} or null"
                        ),
                    ));
                }
            }
            Kind::MaybeText => {
                out.outcome = if is_null(n) {
                    None
                } else {
                    Some(text(name, n)?).filter(|s| !s.is_empty())
                }
            }
        }
    }
    if !plain_id(&out.id) {
        return Err(invalid(
            get(t, "id").and_then(line),
            tr!(
                "id doit être fait de lettres, de chiffres, de - et de _",
                "id must be made of letters, digits, - and _"
            ),
        ));
    }
    Ok(Read::Ok(Box::new(out), newer))
}

// ---------- writing ----------

/// A text as a scalar: plain when YAML reads it back as it is, else double-quoted (JSON's quoting,
/// which YAML reads).
fn scalar(s: &str) -> String {
    if plain_safe(s) {
        s.to_string()
    } else {
        serde_json::to_string(s).unwrap_or_default()
    }
}

/// A long text: a literal block when it runs over several lines and reads back as it is.
fn long_text(name: &str, s: &str) -> String {
    let block = s.contains('\n')
        && !s.starts_with([' ', '\t'])
        && !s.contains('\r')
        && s.lines().all(|l| !l.ends_with([' ', '\t']))
        && !s.chars().any(|c| c.is_control() && c != '\n' && c != '\t');
    if block {
        let body: Vec<String> = s
            .lines()
            .map(|l| {
                if l.is_empty() {
                    String::new()
                } else {
                    format!("  {l}")
                }
            })
            .collect();
        format!("{name}: |-\n{}\n", body.join("\n"))
    } else {
        format!("{name}: {}\n", scalar(s))
    }
}

fn person_yaml(p: &Option<Person>) -> String {
    match p {
        None => "null".into(),
        Some(p) => format!("{{ name: {}, id: {} }}", scalar(&p.name), scalar(&p.id)),
    }
}

fn list_yaml(items: &[String]) -> String {
    format!(
        "[{}]",
        items
            .iter()
            .map(|s| scalar(s))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// The file's text, its fields in the order of `FIELDS`, the same way every time: a change of
/// status is a change of one or two lines.
pub fn write(f: &TicketDoc) -> String {
    let mut out = String::new();
    out.push_str(
        "# A ticket of the Kanban shared by Escouade: https://escouade.dev/en/docs/shared-board\n",
    );
    out.push_str("# Escouade writes it again at each status; commit it with your work, Escouade never does.\n");
    out.push_str(&format!("# yaml-language-server: $schema={SCHEMA_URL}\n"));
    for field in FIELDS {
        let n = field.name;
        let line = match n {
            "version" => format!("version: {VERSION}\n"),
            "id" => format!("id: {}\n", scalar(&f.id)),
            "key" => format!("key: {}\n", scalar(&f.key)),
            "title" => format!("title: {}\n", scalar(&f.title)),
            "status" => format!("status: {}\n", status_name(f.status)),
            "author" => format!("author: {}\n", person_yaml(&Some(f.author.clone()))),
            "taken_by" => format!("taken_by: {}\n", person_yaml(&f.taken_by)),
            "updated_by" => format!("updated_by: {}\n", person_yaml(&f.updated_by)),
            "created" => format!("created: {}\n", scalar(&f.created)),
            "updated" => format!("updated: {}\n", scalar(&f.updated)),
            "after" => format!("after: {}\n", list_yaml(&f.after)),
            "description" => long_text("description", &f.description),
            "criteria" if f.criteria.is_empty() => "criteria: []\n".into(),
            "criteria" => {
                let items: String = f
                    .criteria
                    .iter()
                    .map(|c| format!("  - {}\n", scalar(c)))
                    .collect();
                format!("criteria:\n{items}")
            }
            "images" | "files" => {
                let list = if n == "images" { &f.images } else { &f.files };
                if list.is_empty() {
                    format!("{n}: []\n")
                } else {
                    let items: String = list
                        .iter()
                        .map(|m| {
                            format!(
                                "  - {{ name: {}, file: {} }}\n",
                                scalar(&m.name),
                                scalar(&m.file)
                            )
                        })
                        .collect();
                    format!("{n}:\n{items}")
                }
            }
            "external" => match &f.external {
                None => "external: null\n".into(),
                Some(e) => format!(
                    "external: {{ source: {}, key: {}, url: {} }}\n",
                    scalar(&e.source),
                    scalar(&e.key),
                    scalar(&e.url)
                ),
            },
            "outcome" => match &f.outcome {
                None => "outcome: null\n".into(),
                Some(o) => format!("outcome: {}\n", scalar(o)),
            },
            _ => String::new(),
        };
        out.push_str(&line);
    }
    out
}

/// The time as the files write it: ISO 8601, the machine's offset, to the second.
pub fn iso(ms: i64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
        .unwrap_or_default()
}

// ---------- the schema ----------

fn schema_of(f: &Field) -> Value {
    let person = json!({
        "type": "object",
        "properties": { "name": { "type": "string" }, "id": { "type": "string" } },
        "required": ["name", "id"],
    });
    let media = json!({
        "type": "array",
        "items": {
            "oneOf": [
                { "type": "string" },
                {
                    "type": "object",
                    "properties": { "name": { "type": "string" }, "file": { "type": "string" } },
                    "required": ["file"],
                },
            ],
        },
    });
    let mut s = match f.kind {
        Kind::Version => json!({ "type": "integer", "minimum": 1 }),
        Kind::Text => json!({ "type": "string" }),
        Kind::Status => json!({ "type": "string", "enum": STATUSES }),
        Kind::Person => person,
        Kind::MaybePerson => json!({ "oneOf": [person, { "type": "null" }] }),
        Kind::Ids => {
            json!({ "type": "array", "items": { "type": "string", "pattern": "^[A-Za-z0-9_-]{1,40}$" } })
        }
        Kind::Texts => json!({ "type": "array", "items": { "type": "string" } }),
        Kind::Media => media,
        Kind::External => json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "source": { "type": "string", "enum": crate::integrations::source::registry().services() },
                        "key": { "type": "string" },
                        "url": { "type": "string" },
                    },
                    "required": ["source", "key"],
                },
                { "type": "null" },
            ],
        }),
        Kind::MaybeText => json!({ "type": ["string", "null"] }),
    };
    s["description"] = json!(f.about);
    s
}

/// The JSON schema of a file (`schemas/escouade-ticket.schema.json`), made from `FIELDS`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn schema() -> Value {
    let mut props = Map::new();
    for f in FIELDS {
        props.insert(f.name.into(), schema_of(f));
    }
    let required: Vec<&str> = FIELDS
        .iter()
        .filter(|f| f.required)
        .map(|f| f.name)
        .collect();
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": SCHEMA_URL,
        "title": "A ticket of Escouade's shared Kanban (.escouade/tickets/<id>.yml)",
        "type": "object",
        "properties": props,
        "required": required,
    })
}

// ---------- tickets and files ----------

/// The external ticket of a ticket of ours, as a file writes it.
pub fn external_of(t: &Ticket) -> Option<SharedExternal> {
    // As its file said it, once read; else the ticket's own.
    if let Some(e) = t.shared.as_ref().and_then(|s| s.external.clone()) {
        return Some(e);
    }
    if let Some(e) = &t.external {
        let source = serde_json::to_value(e.service)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        return Some(SharedExternal {
            source,
            key: e.key.clone(),
            url: e.url.clone(),
        });
    }
    None
}

/// What a ticket's file should say now, `me` changing it at `now`. The ids of the tickets it comes
/// after are those of their files; those not on this machine stay as the file said them.
pub fn file_of(t: &Ticket, tickets: &[Ticket], me: &Person, now: &str) -> TicketDoc {
    let s = t.shared.clone().unwrap_or_default();
    let ids: HashSet<&str> = tickets
        .iter()
        .filter(|o| o.project_id == t.project_id)
        .filter_map(|o| o.shared.as_ref().map(|s| s.id.as_str()))
        .collect();
    let mut after: Vec<String> = t
        .after
        .iter()
        .filter_map(|id| tickets.iter().find(|o| &o.id == id))
        .filter_map(|o| o.shared.as_ref().map(|s| s.id.clone()))
        .collect();
    for id in &s.after {
        if !ids.contains(id.as_str()) && !after.contains(id) {
            after.push(id.clone());
        }
    }
    // Taken here: by this machine's user; else as the file said.
    let taken_by = if t.started_at.is_some() {
        Some(me.clone())
    } else {
        s.taken_by.clone()
    };
    TicketDoc {
        version: VERSION,
        id: s.id.clone(),
        key: if s.key.is_empty() {
            t.key.clone()
        } else {
            s.key.clone()
        },
        title: t.title.clone(),
        status: t.column,
        author: s.author.clone(),
        taken_by,
        updated_by: Some(me.clone()),
        created: if s.created.is_empty() {
            iso(t.created_at)
        } else {
            s.created.clone()
        },
        updated: now.to_string(),
        after,
        description: t.description.clone(),
        criteria: t.criteria.iter().map(|c| c.text.clone()).collect(),
        images: t
            .images
            .iter()
            .map(|i| SharedMedia {
                name: i.name.clone(),
                file: i.file.clone(),
            })
            .collect(),
        files: t
            .files
            .iter()
            .filter(|f| f.size <= MAX_SHARED_FILE)
            .map(|f| SharedMedia {
                name: f.name.clone(),
                file: f.file.clone(),
            })
            .collect(),
        external: external_of(t),
        outcome: if t.column == Column::Done {
            t.outcome.clone()
        } else {
            None
        },
    }
}

/// Whether this machine's agent holds the ticket: its column is then its own, whatever a file says.
fn held_here(t: &Ticket) -> bool {
    t.agent_id.is_some() && matches!(t.column, Column::Doing | Column::Review)
}

/// What a file says, put into its ticket: its text, its criteria (those already met stay so), the
/// tickets it comes after, its people, and its column unless this machine's agent holds it.
pub fn apply(
    t: &mut Ticket,
    f: &TicketDoc,
    newer: bool,
    ids: &BTreeMap<String, String>,
    me: &Person,
) {
    t.title = f.title.clone();
    t.description = f.description.clone();
    let old = std::mem::take(&mut t.criteria);
    t.criteria = f
        .criteria
        .iter()
        .map(|text| {
            old.iter()
                .find(|c| &c.text == text)
                .cloned()
                .unwrap_or(Criterion {
                    text: text.clone(),
                    ..Default::default()
                })
        })
        .collect();
    t.after = f
        .after
        .iter()
        .filter_map(|sid| ids.get(sid).cloned())
        .filter(|id| id != &t.id)
        .collect();
    if !held_here(t) && t.column != f.status {
        t.column = f.status;
        match f.status {
            Column::Done => {
                t.done_at.get_or_insert(now_ms());
                t.outcome = f.outcome.clone();
                t.blocked = None;
            }
            Column::Todo => {
                t.blocked = None;
                t.done_at = None;
            }
            _ => {}
        }
    } else if t.column == Column::Done && f.status == Column::Done && t.outcome.is_none() {
        t.outcome = f.outcome.clone();
    }
    let s = t.shared.get_or_insert_with(Shared::default);
    s.id = f.id.clone();
    s.key = f.key.clone();
    s.author = f.author.clone();
    s.taken_by = f.taken_by.clone();
    s.updated_by = f.updated_by.clone();
    s.created = f.created.clone();
    s.after = f.after.clone();
    s.external = f.external.clone();
    s.mine = !me.id.is_empty() && f.author.id == me.id;
    s.problem = None;
    s.conflict = false;
    s.newer = newer;
    s.gone = false;
}

/// The board of another one's ticket: its key, with `@author` after it when this project already has a
/// ticket of that key.
fn key_for(f: &TicketDoc, tickets: &[Ticket], project_id: &str) -> String {
    let taken = tickets
        .iter()
        .any(|t| t.project_id == project_id && t.key == f.key);
    if !taken {
        return f.key.clone();
    }
    let who: String = f
        .author
        .name
        .split_whitespace()
        .next()
        .unwrap_or("x")
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    format!("{}@{}", f.key, if who.is_empty() { "x" } else { &who })
}

/// What `reconcile` did, for its caller to do the rest: write files, copy images, tell the window.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Files to write: their id and their text (their fingerprint is already kept).
    pub writes: Vec<(String, String, String)>,
    /// Tickets changed or made (by id).
    pub changed: Vec<String>,
    /// Tickets removed (by id): their files are gone and nobody here took them.
    pub removed: Vec<String>,
    /// Tickets whose images and files must come from the folder: (ticket id, file id).
    pub media: Vec<(String, String)>,
}

/// Brings the tickets of `project_id` and the files of its folder (`files`: id → text) in step,
/// `me` being this machine's user, at `now` (as the files write it). See the module's comment.
pub fn reconcile(
    tickets: &mut Vec<Ticket>,
    project_id: &str,
    files: &BTreeMap<String, String>,
    me: &Person,
    now: &str,
    lang: Lang,
) -> Outcome {
    let mut out = Outcome::default();
    let snapshot: Vec<Ticket> = tickets.clone();
    // File id → ticket id, for « Après ».
    let mut ids: BTreeMap<String, String> = tickets
        .iter()
        .filter(|t| t.project_id == project_id)
        .filter_map(|t| t.shared.as_ref().map(|s| (s.id.clone(), t.id.clone())))
        .collect();
    // The files no ticket has yet: new tickets, made first so that « Après » finds them.
    let known: HashSet<String> = ids.keys().cloned().collect();
    for (sid, text) in files.iter().filter(|(sid, _)| !known.contains(*sid)) {
        let seen = fingerprint(text);
        let read = read(text);
        if let Read::Ok(f, _) = &read {
            // An import of the same outside ticket, not started: it becomes the shared one.
            if let Some(ext) = &f.external {
                let twin = tickets.iter().position(|t| {
                    t.project_id == project_id
                        && t.shared.is_none()
                        && t.started_at.is_none()
                        && external_of(t)
                            .is_some_and(|e| e.source == ext.source && e.key == ext.key)
                });
                if let Some(i) = twin {
                    tickets[i].shared = Some(Shared {
                        id: sid.clone(),
                        ..Default::default()
                    });
                    ids.insert(sid.clone(), tickets[i].id.clone());
                    out.changed.push(tickets[i].id.clone());
                    continue;
                }
            }
        }
        let mut t = Ticket {
            id: new_id(),
            project_id: project_id.to_string(),
            created_at: now_ms(),
            max_loops: 5,
            rank: tickets
                .iter()
                .filter(|t| t.project_id == project_id)
                .map(|t| t.rank)
                .max()
                .unwrap_or(0)
                + 1,
            shared: Some(Shared {
                id: sid.clone(),
                seen: String::new(),
                ..Default::default()
            }),
            ..Default::default()
        };
        match &read {
            Read::Ok(f, _) => {
                t.key = key_for(f, tickets, project_id);
                out.media.push((t.id.clone(), sid.clone()));
            }
            _ => {
                t.key = sid.clone();
                t.title = format!("{sid}.yml");
            }
        }
        let _ = seen;
        ids.insert(sid.clone(), t.id.clone());
        out.changed.push(t.id.clone());
        tickets.push(t);
    }
    let all = tickets.clone();
    let mut remove = Vec::new();
    for t in tickets.iter_mut().filter(|t| t.project_id == project_id) {
        let Some(s) = t.shared.clone() else { continue };
        let Some(text) = files.get(&s.id) else {
            if s.gone {
                continue;
            }
            let fresh = s.seen.is_empty();
            let mine = s.mine || (s.author.id.is_empty() && fresh);
            if mine && fresh {
                // Shared a moment ago: its first file.
                let mut f = file_of(t, &all, me, now);
                if f.author.id.is_empty() {
                    f.author = me.clone();
                    t.shared.as_mut().unwrap().author = me.clone();
                    t.shared.as_mut().unwrap().mine = true;
                }
                let text = write(&f);
                t.shared.as_mut().unwrap().seen = fingerprint(&text);
                out.writes.push((s.id.clone(), text, t.id.clone()));
                out.changed.push(t.id.clone());
            } else if mine {
                t.shared.as_mut().unwrap().gone = true;
                out.changed.push(t.id.clone());
            } else if t.started_at.is_some() {
                // Taken here: it stays, as ours.
                let s = t.shared.as_mut().unwrap();
                s.gone = true;
                s.left = true;
                out.changed.push(t.id.clone());
            } else {
                remove.push(t.id.clone());
            }
            continue;
        };
        let h = fingerprint(text);
        if h == s.seen && !s.gone {
            if s.problem.is_some() || s.newer {
                continue;
            }
            let wanted = file_of(t, &all, me, now);
            let same = matches!(read(text), Read::Ok(ref f, _) if wanted.same(f));
            if !same {
                let text = write(&wanted);
                t.shared.as_mut().unwrap().seen = fingerprint(&text);
                out.writes.push((s.id.clone(), text, t.id.clone()));
                out.changed.push(t.id.clone());
            }
            continue;
        }
        // Changed outside Escouade (or back again): read.
        match read(text) {
            Read::Ok(f, newer) => {
                let images_before = t.shared.as_ref().map(|_| t.images.len()).unwrap_or(0);
                let _ = images_before;
                let was = t.clone();
                apply(t, &f, newer, &ids, me);
                if t.key.is_empty() || t.key == s.id {
                    t.key = key_for(&f, &snapshot, project_id);
                }
                let sh = t.shared.as_mut().unwrap();
                sh.seen = h;
                sh.left = false;
                let media_changed = f
                    .images
                    .iter()
                    .map(|m| &m.file)
                    .ne(was.images.iter().map(|i| &i.file))
                    || f.files
                        .iter()
                        .map(|m| &m.file)
                        .ne(was.files.iter().map(|i| &i.file));
                if media_changed && !sh.mine {
                    out.media.push((t.id.clone(), s.id.clone()));
                }
                out.changed.push(t.id.clone());
            }
            Read::Conflict => {
                let sh = t.shared.as_mut().unwrap();
                sh.problem = Some(tr_in!(
                    lang,
                    "Son fichier est en conflit après un pull : choisis la version à garder.",
                    "Its file is in conflict after a pull: choose the version to keep."
                ));
                sh.conflict = true;
                sh.seen = h;
                sh.gone = false;
                out.changed.push(t.id.clone());
            }
            Read::Invalid(line, why) => {
                let sh = t.shared.as_mut().unwrap();
                sh.problem = Some(match line {
                    Some(l) => tr_in!(lang, "Ligne {l} : {why}", "Line {l}: {why}"),
                    None => why,
                });
                sh.conflict = false;
                sh.seen = h;
                sh.gone = false;
                out.changed.push(t.id.clone());
            }
        }
    }
    tickets.retain(|t| !remove.contains(&t.id));
    out.removed = remove;
    out
}

/// Under way or to test on another machine of the team, as its file says (ESC-190): never started here. This machine
/// neither takes it up at its start nor counts it among its places.
pub fn elsewhere(t: &Ticket) -> bool {
    t.shared.is_some()
        && t.started_at.is_none()
        && matches!(t.column, Column::Doing | Column::Review)
}

/// The ticket of another one of the team (shared, not by this machine's user).
pub fn theirs(t: &Ticket) -> bool {
    t.shared.as_ref().is_some_and(|s| !s.mine)
}

/// What refuses an agent the start of another one's shared ticket (choice 130).
pub fn start_refusal(lang: Lang, t: &Ticket) -> Option<String> {
    let s = t.shared.as_ref().filter(|s| !s.mine)?;
    let who = if s.author.name.is_empty() {
        tr_in!(lang, "quelqu’un d’autre", "someone else")
    } else {
        s.author.name.clone()
    };
    Some(tr_in!(
        lang,
        "{} est partagé par {who} : il se lance à la main, depuis le Kanban d’Escouade.",
        "{} is shared by {who}: it is started by hand, from Escouade’s Kanban.",
        t.key
    ))
}

// ---------- on the core ----------

use crate::core::Core;
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Runtime;

/// One reconciliation at a time, whatever the project: the save loop and the folder's watcher may
/// both ask for one.
static SYNC: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

/// The side of a file in conflict that git calls `ours` (the checkout's) or theirs (the one merged
/// in): the text between the markers, the rest as it is.
pub fn conflict_side(text: &str, ours: bool) -> String {
    #[derive(PartialEq)]
    enum At {
        Both,
        Ours,
        Base,
        Theirs,
    }
    let mut at = At::Both;
    let mut out = String::new();
    for l in text.split_inclusive('\n') {
        if l.starts_with("<<<<<<<") {
            at = At::Ours;
        } else if l.starts_with("|||||||") && at == At::Ours {
            at = At::Base;
        } else if l.starts_with("=======") && matches!(at, At::Ours | At::Base) {
            at = At::Theirs;
        } else if l.starts_with(">>>>>>>") && at == At::Theirs {
            at = At::Both;
        } else if at == At::Both || (ours && at == At::Ours) || (!ours && at == At::Theirs) {
            out.push_str(l);
        }
    }
    out
}

/// The folder of the shared tickets' files of `project` (choice 109): in its folder, or in its first
/// repository when it has several and its folder is none.
pub fn dir_of(project: &Project) -> PathBuf {
    let root = Path::new(&project.path);
    let base = match project.repos.first() {
        Some(first) if !first.is_empty() => root.join(first),
        _ => root.to_path_buf(),
    };
    base.join(DIR)
}

/// The files of the folder: id → text. None of them when the folder is not there.
fn read_dir(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if crate::workflowfiles::unsafe_dir(dir).is_some() {
        return out;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Some(id) = id_of_file(&name) else {
            continue;
        };
        // `file_type` of an entry does not follow a link: one is never read (it can point anywhere).
        if !e.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(e.path()) {
            out.insert(id.to_string(), text);
        }
    }
    out
}

/// `git config user.name` in `path`, empty when git has none (or no git): `Core::me_in` is called
/// where nothing awaits, and git answers it at once.
fn git_name_now(path: &str) -> String {
    let mut cmd = std::process::Command::new("git");
    cmd.args(["-C", path, "config", "--get", "user.name"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    cmd.output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

impl<R: Runtime> Core<R> {
    /// This machine's user in `project` (choices 113 and 114): the project's name, else the app's,
    /// else the name git knows in the project's folder (ESC-243, never written in `.escouade.yml`);
    /// the id of the settings.
    pub(crate) fn me_in(&self, project: &Project) -> Person {
        let (own, id) = {
            let s = self.settings.read();
            let own = Some(project.user_name.trim())
                .filter(|n| !n.is_empty())
                .unwrap_or(s.user_name.trim())
                .to_string();
            (own, s.user_id.clone())
        };
        let name = if own.is_empty() {
            self.git_name_in(&project.path)
        } else {
            own
        };
        Person { name, id }
    }

    /// `git config user.name` in `path` (its repository's own, else the global one), read once and
    /// kept: `git_user_name` reads it again when the window asks.
    fn git_name_in(&self, path: &str) -> String {
        if let Some(name) = self.git_names.lock().get(path) {
            return name.clone();
        }
        let name = git_name_now(path);
        self.git_names.lock().insert(path.to_string(), name.clone());
        name
    }

    /// The id of this machine's user, made once (from the git e-mail, else drawn) and kept in the
    /// settings.
    pub(crate) async fn ensure_identity(&self) {
        if !self.settings.read().user_id.is_empty() {
            return;
        }
        let cwd = dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".into());
        let email = crate::git::run(&cwd, &["config", "--get", "user.email"])
            .await
            .map(|o| String::from_utf8_lossy(&o).trim().to_string())
            .unwrap_or_default();
        let id = person_id(&email).unwrap_or_else(new_share_id);
        let bytes = {
            let mut s = self.settings.write();
            if !s.user_id.is_empty() {
                return;
            }
            s.user_id = id;
            serde_json::to_vec_pretty(&*s)
        };
        if let Ok(bytes) = bytes {
            if let Err(e) = crate::paths::write_atomic(&self.data.settings_file(), &bytes) {
                log::warn!("settings: the user's id not saved: {e}");
            }
        }
    }

    /// The name git knows the user by (`git config user.name`): in the project's folder (a repository
    /// may have its own), else the global one. Offered by the first launch's window and the settings,
    /// and kept for the project's shared tickets.
    pub async fn git_user_name(&self, project_id: Option<String>) -> String {
        let project = project_id
            .and_then(|id| self.project(&id).ok())
            .map(|p| p.path);
        let cwd = project
            .clone()
            .or_else(|| dirs::home_dir().map(|h| h.to_string_lossy().to_string()))
            .unwrap_or_else(|| ".".into());
        let name = crate::git::run(&cwd, &["config", "--get", "user.name"])
            .await
            .map(|o| String::from_utf8_lossy(&o).trim().to_string())
            .unwrap_or_default();
        if let Some(path) = project {
            self.git_names.lock().insert(path, name.clone());
        }
        name
    }

    /// Every project that has shared tickets, or a folder of them, brought in step with its files.
    pub(crate) fn sync_all_shared(self: &Arc<Self>) {
        let projects: Vec<Project> = self.projects.read().clone();
        for p in projects {
            let has = self
                .tickets
                .read()
                .iter()
                .any(|t| t.project_id == p.id && t.shared.is_some());
            if has || dir_of(&p).is_dir() {
                self.sync_shared(&p.id);
            }
        }
    }

    /// The tickets of a project and the files of its folder brought in step (`reconcile`): files
    /// written, images and files copied both ways, the window told.
    pub fn sync_shared(self: &Arc<Self>, project_id: &str) {
        let _one = SYNC.lock();
        let Ok(project) = self.project(project_id) else {
            return;
        };
        let dir = dir_of(&project);
        // A folder that is a link, or leaves the project: neither read (its files would look removed)
        // nor written.
        if let Some(why) = crate::workflowfiles::unsafe_dir(&dir) {
            log::warn!("shared tickets of {}: {why}", project.name);
            return;
        }
        let files = read_dir(&dir);
        let me = self.me_in(&project);
        let now = iso(now_ms());
        let lang = crate::i18n::ui();
        let (out, tickets) = {
            let mut tickets = self.tickets.write();
            // Whose each ticket is, as the id of the settings says now.
            for t in tickets.iter_mut().filter(|t| t.project_id == project_id) {
                if let Some(s) = t.shared.as_mut() {
                    s.mine = !me.id.is_empty() && s.author.id == me.id;
                }
            }
            let out = reconcile(&mut tickets, project_id, &files, &me, &now, lang);
            let changed: Vec<Ticket> = tickets
                .iter()
                .filter(|t| out.changed.contains(&t.id))
                .cloned()
                .collect();
            (out, changed)
        };
        for (sid, text, tid) in &out.writes {
            if let Err(e) = self.write_shared_file(&dir, sid, text, tid) {
                log::warn!("shared ticket {sid}: not written: {e:#}");
            }
        }
        for (tid, sid) in &out.media {
            self.import_media(&dir, sid, tid);
        }
        let any = !out.changed.is_empty() || !out.removed.is_empty();
        for id in &out.removed {
            self.hub.emit(UiEvent::TicketRemoved {
                id: id.clone(),
                project_id: project_id.to_string(),
            });
            crate::ticketimages::remove(&self.data.ticket_dir(id));
        }
        for t in tickets {
            let ticket = self.ticket(&t.id).unwrap_or(t);
            self.hub.emit(UiEvent::Ticket { ticket });
        }
        if any {
            self.request_save();
            self.schedule();
        }
    }

    /// Writes a ticket's file, and the images and files of a ticket of ours beside it.
    fn write_shared_file(&self, dir: &Path, sid: &str, text: &str, tid: &str) -> Result<()> {
        // Never through a link, nor out of the project (ESC-199).
        if let Some(why) = crate::workflowfiles::unsafe_dir(dir) {
            bail!(why);
        }
        std::fs::create_dir_all(dir)?;
        crate::paths::write_atomic(&dir.join(format!("{sid}.yml")), text.as_bytes())?;
        let Ok(t) = self.ticket(tid) else {
            return Ok(());
        };
        if t.shared.as_ref().is_some_and(|s| !s.mine) {
            return Ok(());
        }
        let from = self.data.ticket_dir(tid);
        let to = dir.join(sid);
        let mut wanted: Vec<(PathBuf, PathBuf)> = t
            .images
            .iter()
            .filter(|i| plain_media(&i.file))
            .map(|i| (from.join(&i.file), to.join(&i.file)))
            .collect();
        wanted.extend(
            t.files
                .iter()
                .filter(|f| f.size <= MAX_SHARED_FILE && plain_media(&f.file))
                .filter_map(|f| {
                    let src = crate::ticketfiles::present(&from, &f.file)?;
                    Some((src, to.join(&f.file)))
                }),
        );
        for (src, dst) in &wanted {
            if dst.exists() || !src.is_file() {
                continue;
            }
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(src, dst)?;
        }
        Ok(())
    }

    /// The images and files of another one's ticket, copied from the folder into the ticket's own.
    fn import_media(&self, dir: &Path, sid: &str, tid: &str) {
        let Ok(text) = std::fs::read_to_string(dir.join(format!("{sid}.yml"))) else {
            return;
        };
        let Read::Ok(f, _) = read(&text) else { return };
        let from = dir.join(sid);
        let to = self.data.ticket_dir(tid);
        let mut images = Vec::new();
        for m in f
            .images
            .iter()
            .filter(|m| plain_media(&m.file) && !m.file.contains('/'))
        {
            let src = from.join(&m.file);
            let dst = to.join(&m.file);
            if src.is_file() && !dst.exists() {
                let _ = std::fs::create_dir_all(&to);
                if let Err(e) = std::fs::copy(&src, &dst) {
                    log::warn!("shared ticket {sid}: image {} not copied: {e}", m.file);
                }
            }
            let ext = m.file.rsplit('.').next().unwrap_or_default().to_lowercase();
            let media_type = match ext.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "image/png",
            };
            images.push(TicketImage {
                name: m.name.clone(),
                media_type: media_type.into(),
                file: m.file.clone(),
            });
        }
        let mut files = Vec::new();
        for m in f
            .files
            .iter()
            .filter(|m| crate::ticketfiles::valid_file(&m.file).is_some())
        {
            let src = from.join(&m.file);
            let Some(dst) = crate::ticketfiles::path_of(&to, &m.file) else {
                continue;
            };
            let size = std::fs::metadata(&src).map(|m| m.len()).unwrap_or(0);
            if src.is_file() && !dst.exists() {
                if let Some(parent) = dst.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = std::fs::copy(&src, &dst) {
                    log::warn!("shared ticket {sid}: file {} not copied: {e}", m.file);
                }
            }
            files.push(TicketFile {
                name: m.name.clone(),
                file: m.file.clone(),
                size,
            });
        }
        let mut tickets = self.tickets.write();
        if let Some(t) = tickets.iter_mut().find(|t| t.id == tid) {
            t.images = images;
            t.files = files;
        }
    }

    /// « Partager avec l'équipe » (`on`) or « Arrêter le partage » (choices 115 and 127). Sharing
    /// asks for this machine's user's name first.
    pub fn ticket_share(self: &Arc<Self>, id: &str, on: bool) -> Result<Ticket> {
        let t = self.ticket(id)?;
        let project = self.project(&t.project_id)?;
        let dir = dir_of(&project);
        if on {
            if t.shared.as_ref().is_some_and(|s| !s.gone) {
                return Ok(t);
            }
            let me = self.me_in(&project);
            if me.name.is_empty() {
                return Err(name_missing());
            }
            self.edit_ticket(id, |t| {
                let created = iso(t.created_at);
                let s = t.shared.get_or_insert_with(Shared::default);
                if s.id.is_empty() {
                    s.id = new_share_id();
                }
                if s.key.is_empty() {
                    s.key = t.key.clone();
                }
                if s.author.id.is_empty() {
                    s.author = me.clone();
                }
                if s.created.is_empty() {
                    s.created = created;
                }
                s.mine = s.author.id == me.id;
                s.seen = String::new();
                s.gone = false;
                s.left = false;
                Ok(())
            })?;
        } else {
            let Some(s) = t.shared.clone() else {
                return Ok(t);
            };
            remove_file_of(&dir, &s.id);
            self.edit_ticket(id, |t| {
                t.shared = None;
                Ok(())
            })?;
        }
        self.sync_shared(&project.id);
        self.ticket(id)
    }

    /// « Masquer chez moi » (choice 118): another one's ticket leaves this board, its file kept.
    pub fn ticket_hide(&self, id: &str, hidden: bool) -> Result<Ticket> {
        self.edit_ticket(id, |t| {
            match t.shared.as_mut() {
                Some(s) => s.hidden = hidden,
                None => bail!(not_shared()),
            }
            Ok(())
        })?;
        self.ticket(id)
    }

    /// A shared ticket's file in conflict settled (choice 125): this machine's version written over
    /// it (`keep_mine`), or the side git merged in.
    pub fn shared_resolve(self: &Arc<Self>, id: &str, keep_mine: bool) -> Result<Ticket> {
        let t = self.ticket(id)?;
        let s = t.shared.clone().ok_or_else(not_shared)?;
        let project = self.project(&t.project_id)?;
        let dir = dir_of(&project);
        // Never through a link, nor out of the project (ESC-199): the folder, then the file itself.
        if let Some(why) = crate::workflowfiles::unsafe_dir(&dir) {
            bail!(why);
        }
        let path = dir.join(format!("{}.yml", s.id));
        if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            bail!(tr!(
                "{p} est un lien symbolique : ignoré",
                "{p} is a symbolic link: ignored",
                p = path.display()
            ));
        }
        if keep_mine {
            let me = self.me_in(&project);
            let text = {
                let tickets = self.tickets.read();
                write(&file_of(&t, &tickets, &me, &iso(now_ms())))
            };
            crate::paths::write_atomic(&path, text.as_bytes())?;
            self.edit_ticket(id, |t| {
                if let Some(s) = t.shared.as_mut() {
                    s.problem = None;
                    s.conflict = false;
                    s.seen = fingerprint(&text);
                }
                Ok(())
            })?;
        } else {
            let text = std::fs::read_to_string(&path)?;
            crate::paths::write_atomic(&path, conflict_side(&text, false).as_bytes())?;
        }
        self.sync_shared(&project.id);
        self.ticket(id)
    }
}

/// A ticket's file and its folder of images and files, gone.
pub(crate) fn remove_file_of(dir: &Path, sid: &str) {
    if !plain_id(sid) {
        return;
    }
    let _ = std::fs::remove_file(dir.join(format!("{sid}.yml")));
    let _ = std::fs::remove_dir_all(dir.join(sid));
}

fn not_shared() -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "Ce ticket n’est pas partagé.",
        "This ticket is not shared."
    ))
}

/// The refusal of a share before this machine's user gave a name.
pub(crate) fn name_missing() -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "Donne d’abord ton nom (Réglages › Application › Équipe) : il est écrit dans les tickets que tu partages.",
        "Give your name first (Settings › Application › Team): it is written in the tickets you share."
    ))
}
