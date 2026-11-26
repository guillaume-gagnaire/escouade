//! The templates of the agents' pages (docs/AGENT-PAGES.md § 7, AP-5, ESC-674): `review`,
//! `diagram` and `compare`. A template is a page of Escouade (its HTML, written here, and its
//! files `_t/page.css`, `_t/page.js`, `_t/mermaid.js`, shipped with the app and read as its code),
//! served in the same shelter as any page (`agentpage`). The agent gives **data** alone:
//!
//! - **checked by the core** against the template's schema (`check`): a field the schema does not
//!   have, a value of the wrong type, too long or out of bounds, is refused, all of them said at
//!   once by their path (`criteria.2.state`), so that the agent mends them in one go;
//! - **rendered as text** (`render`): every value is escaped, and a `md` field goes through a
//!   Markdown of its own (`md_html`) that writes nothing but its few tags. No HTML of the agent ever
//!   reaches the page, so no script either;
//! - **anchored** (`anchors`, `anchor`): each part of a page has a stable id, and a key of the
//!   template names it (`criteria.2` is `#crit-3`), for `drive_browser` (AB-7) to aim at.
//!
//! The diagram's Mermaid is **embedded** (`mermaid.min.js.gz`, version `MERMAID_VERSION`, checked
//! by its SHA-256), run without network in `securityLevel: 'strict'`.
//!
//! The page follows the app's language (its words are written here, in French and English) and its
//! theme (`data-theme`, which `?theme=` of its address changes without a new render), and the
//! project's tint.

use crate::i18n::Lang;
use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;
use std::fmt::Write as _;
use std::sync::OnceLock;

/// The data of a template, as JSON, at most (§ 3).
pub const DATA_MAX: usize = 256 * 1024;
/// A change's diff, at most (§ 7.1).
pub const DIFF_MAX: usize = 200 * 1024;
/// The lines of a diff shown open; a longer one is folded (§ 7.1).
pub const DIFF_OPEN_LINES: usize = 400;
/// A diagram's source, at most (§ 7.2).
pub const SOURCE_MAX: usize = 64 * 1024;
/// The page's title, at most (§ 4.1).
pub const TITLE_MAX: usize = 120;
/// The version of the embedded Mermaid (its test checks it; `scripts/mermaid-vendor.sh` changes it).
#[cfg_attr(not(test), allow(dead_code))]
pub const MERMAID_VERSION: &str = "12.3.0";
/// The SHA-256 of the embedded Mermaid (`mermaid.min.js` of the npm package, before gzip).
#[cfg_attr(not(test), allow(dead_code))]
pub const MERMAID_SHA256: &str = "336df803d87a38b70a01e736e666c8f7601d2ce42f5e47c8fafe5d721074b9a3";

/// A template of Escouade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Template {
    Review,
    Diagram,
    Compare,
}

impl Template {
    pub const ALL: [Template; 3] = [Template::Review, Template::Diagram, Template::Compare];

    /// The template named `name`: `review`, `diagram` or `compare`.
    pub fn parse(name: &str) -> Option<Template> {
        Template::ALL.into_iter().find(|t| t.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Template::Review => "review",
            Template::Diagram => "diagram",
            Template::Compare => "compare",
        }
    }

    fn schema(self) -> &'static [F] {
        match self {
            Template::Review => REVIEW,
            Template::Diagram => DIAGRAM,
            Template::Compare => COMPARE,
        }
    }
}

/// The page's theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn name(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }
}

/// How a page looks: the app's language and theme, the project's tint, and the request it carries.
#[derive(Debug, Clone)]
pub struct Look {
    pub lang: Lang,
    pub theme: Theme,
    /// The project's tint, `#rrggbb`; anything else is left out.
    pub tint: Option<String>,
    /// The request the page carries (`ask` of `show_page`, AP-4): its options are drawn in the page,
    /// and only proposed from it (the band of Escouade sends).
    pub choice: Option<Choice>,
}

/// A request of the box, as a page draws it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Choice {
    /// Its key, `a-31`.
    pub key: String,
    pub title: String,
    pub options: Vec<ChoiceOption>,
    /// The option the agent recommends.
    pub recommend: Option<String>,
    pub multi: bool,
    /// The options chosen, once it is answered: the page shows them, the others frozen.
    pub answered: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

/// A field of the data that does not go, by its path (`criteria.2.state`, `.` for the whole).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
    pub at: String,
    pub what: String,
}

impl Problem {
    fn new(at: &str, what: String) -> Problem {
        Problem {
            at: if at.is_empty() { ".".into() } else { at.into() },
            what,
        }
    }
}

/// The problems, one per line, as the refusal tells the agent.
pub fn tell(problems: &[Problem]) -> String {
    problems
        .iter()
        .map(|p| format!("- {}: {}", p.at, p.what))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------- the schemas ----------

/// What a value of the data must be.
#[derive(Debug)]
enum S {
    /// Text on one line: no control character, `max` characters at most, not empty.
    Line(usize),
    /// Text on several lines (Markdown, a source, a diff): no control character but line breaks and
    /// tabs, `max` bytes at most, not empty.
    Text(usize),
    /// A whole number between the two.
    Int(i64, i64),
    /// One of these words.
    Word(&'static [&'static str]),
    /// An id: 1 to 40 of `a-z`, `0-9`, `-` and `_`, starting with a letter or a digit.
    Id,
    /// A ticket's key: 1 to 40 of letters, digits and `-`.
    Key,
    /// An image of the version: a relative path (`shots/a.png`), PNG, JPEG, GIF, WebP or AVIF by
    /// its name (its content is checked when it is copied, AP-2), or `{ "dark", "light" }`.
    Image,
    /// An object with these fields and no other.
    Obj(&'static [F]),
    /// A list of `min` to `max` of these.
    List(&'static S, usize, usize),
}

/// A field of an object.
#[derive(Debug)]
struct F {
    name: &'static str,
    s: S,
    required: bool,
}

const fn req(name: &'static str, s: S) -> F {
    F {
        name,
        s,
        required: true,
    }
}

const fn opt(name: &'static str, s: S) -> F {
    F {
        name,
        s,
        required: false,
    }
}

const COUNT: S = S::Int(0, 10_000_000);
const MD: S = S::Text(8_000);

const REVIEW: &[F] = &[
    req(
        "subject",
        S::Obj(&[
            opt("ticket", S::Key),
            opt("tickets", S::List(&S::Key, 1, 50)),
            opt("branch", S::Line(200)),
            opt("target", S::Line(200)),
            opt("loop", S::Line(20)),
        ]),
    ),
    opt(
        "facts",
        S::Obj(&[
            opt("files", COUNT),
            opt("added", COUNT),
            opt("removed", COUNT),
            opt(
                "tests",
                S::Obj(&[
                    req("passed", COUNT),
                    req("total", COUNT),
                    opt("failed", COUNT),
                ]),
            ),
        ]),
    ),
    opt(
        "criteria",
        S::List(
            &S::Obj(&[
                req("text", S::Line(500)),
                req("state", S::Word(&["met", "partial", "unmet", "unknown"])),
                opt("md", MD),
                opt("ticket", S::Key),
            ]),
            0,
            100,
        ),
    ),
    opt(
        "changes",
        S::List(
            &S::Obj(&[
                req("path", S::Line(500)),
                opt("added", COUNT),
                opt("removed", COUNT),
                opt(
                    "status",
                    S::Word(&["added", "modified", "deleted", "renamed"]),
                ),
                opt("diff", S::Text(DIFF_MAX)),
                opt("ticket", S::Key),
            ]),
            0,
            500,
        ),
    ),
    opt(
        "risks",
        S::List(
            &S::Obj(&[
                req("md", MD),
                opt("level", S::Word(&["low", "medium", "high"])),
                opt("ticket", S::Key),
            ]),
            0,
            50,
        ),
    ),
    opt(
        "sections",
        S::List(
            &S::Obj(&[
                req("id", S::Id),
                req("title", S::Line(120)),
                req("md", S::Text(20_000)),
                opt("ticket", S::Key),
            ]),
            0,
            30,
        ),
    ),
];

const DIAGRAM: &[F] = &[
    req("kind", S::Word(&["mermaid"])),
    req("source", S::Text(SOURCE_MAX)),
    opt("md", MD),
];

const COMPARE: &[F] = &[
    req(
        "options",
        S::List(
            &S::Obj(&[
                req("id", S::Id),
                req("title", S::Line(120)),
                opt("md", MD),
                opt("images", S::List(&S::Image, 0, 4)),
                opt("pros", S::List(&S::Line(300), 0, 20)),
                opt("cons", S::List(&S::Line(300), 0, 20)),
            ]),
            2,
            8,
        ),
    ),
    opt("criteria", S::List(&S::Line(300), 0, 20)),
    opt("recommend", S::Id),
];

/// Checks `data` against `template`'s schema: every problem, at once, in `lang`.
pub fn check(template: Template, data: &Value, lang: Lang) -> Result<(), Vec<Problem>> {
    let mut out = Vec::new();
    let size = serde_json::to_vec(data).map(|v| v.len()).unwrap_or(0);
    if size > DATA_MAX {
        out.push(Problem::new(
            "",
            tr_in!(
                lang,
                "les données font {size} octets, {DATA_MAX} au plus",
                "the data is {size} bytes, {DATA_MAX} at most"
            ),
        ));
        return Err(out);
    }
    object(template.schema(), data, "", lang, &mut out);
    if out.is_empty() {
        rules(template, data, lang, &mut out);
    }
    if out.is_empty() {
        Ok(())
    } else {
        Err(out)
    }
}

fn join(at: &str, name: &str) -> String {
    if at.is_empty() {
        name.to_string()
    } else {
        format!("{at}.{name}")
    }
}

fn object(fields: &[F], v: &Value, at: &str, lang: Lang, out: &mut Vec<Problem>) {
    let Some(map) = v.as_object() else {
        out.push(Problem::new(at, expected(lang, "object")));
        return;
    };
    for key in map.keys() {
        if !fields.iter().any(|f| f.name == key) {
            let known = fields.iter().map(|f| f.name).collect::<Vec<_>>().join(", ");
            out.push(Problem::new(
                &join(at, key),
                tr_in!(
                    lang,
                    "champ inconnu (champs permis : {known})",
                    "unknown field (fields allowed: {known})"
                ),
            ));
        }
    }
    for f in fields {
        match map.get(f.name) {
            Some(Value::Null) | None if f.required => out.push(Problem::new(
                &join(at, f.name),
                tr_in!(lang, "manquant", "missing"),
            )),
            Some(Value::Null) | None => {}
            Some(value) => one(&f.s, value, &join(at, f.name), lang, out),
        }
    }
}

fn expected(lang: Lang, what: &str) -> String {
    let (fr, en) = match what {
        "object" => ("un objet", "an object"),
        "list" => ("une liste", "a list"),
        "text" => ("du texte", "text"),
        "number" => ("un nombre entier", "a whole number"),
        _ => ("une image", "an image"),
    };
    tr_in!(lang, "attendu : {fr}", "expected: {en}")
}

fn one(s: &S, v: &Value, at: &str, lang: Lang, out: &mut Vec<Problem>) {
    match s {
        S::Obj(fields) => object(fields, v, at, lang, out),
        S::List(of, min, max) => {
            let Some(items) = v.as_array() else {
                out.push(Problem::new(at, expected(lang, "list")));
                return;
            };
            if items.len() < *min || items.len() > *max {
                let n = items.len();
                out.push(Problem::new(
                    at,
                    tr_in!(
                        lang,
                        "{n} éléments, de {min} à {max} permis",
                        "{n} items, {min} to {max} allowed"
                    ),
                ));
            }
            for (i, item) in items.iter().take(*max).enumerate() {
                one(of, item, &join(at, &i.to_string()), lang, out);
            }
        }
        S::Int(min, max) => match v.as_i64() {
            Some(n) if n >= *min && n <= *max => {}
            Some(n) => out.push(Problem::new(
                at,
                tr_in!(
                    lang,
                    "{n} hors des bornes ({min} à {max})",
                    "{n} out of bounds ({min} to {max})"
                ),
            )),
            None => out.push(Problem::new(at, expected(lang, "number"))),
        },
        S::Image => match v {
            Value::String(path) => image_path(path, at, lang, out),
            Value::Object(map) => {
                object(
                    &[req("dark", S::Line(300)), req("light", S::Line(300))],
                    v,
                    at,
                    lang,
                    out,
                );
                for k in ["dark", "light"] {
                    if let Some(Value::String(p)) = map.get(k) {
                        image_path(p, &join(at, k), lang, out);
                    }
                }
            }
            _ => out.push(Problem::new(at, expected(lang, "image"))),
        },
        S::Line(_) | S::Text(_) | S::Word(_) | S::Id | S::Key => {
            let Some(text) = v.as_str() else {
                out.push(Problem::new(at, expected(lang, "text")));
                return;
            };
            if let Some(what) = text_problem(s, text, lang) {
                out.push(Problem::new(at, what));
            }
        }
    }
}

/// What is wrong with `text` as a value of `s`, if anything.
fn text_problem(s: &S, text: &str, lang: Lang) -> Option<String> {
    if text.trim().is_empty() {
        return Some(tr_in!(lang, "vide", "empty"));
    }
    let lines = matches!(s, S::Text(_));
    if text
        .chars()
        .any(|c| c.is_control() && !(lines && matches!(c, '\n' | '\r' | '\t')))
    {
        return Some(tr_in!(
            lang,
            "caractère de contrôle refusé",
            "control character refused"
        ));
    }
    match s {
        S::Line(max) if text.chars().count() > *max => {
            let n = text.chars().count();
            Some(tr_in!(
                lang,
                "{n} caractères, {max} au plus",
                "{n} characters, {max} at most"
            ))
        }
        S::Text(max) if text.len() > *max => {
            let n = text.len();
            Some(tr_in!(
                lang,
                "{n} octets, {max} au plus",
                "{n} bytes, {max} at most"
            ))
        }
        S::Word(words) if !words.contains(&text) => {
            let words = words.join(", ");
            Some(tr_in!(lang, "un de : {words}", "one of: {words}"))
        }
        S::Id if !is_id(text) => Some(tr_in!(
            lang,
            "un id de 1 à 40 caractères parmi a-z, 0-9, - et _",
            "an id of 1 to 40 characters among a-z, 0-9, - and _"
        )),
        S::Key if !is_key(text) => Some(tr_in!(
            lang,
            "une clé de ticket (lettres, chiffres et -, 40 au plus)",
            "a ticket's key (letters, digits and -, 40 at most)"
        )),
        _ => None,
    }
}

/// An id of the data: 1 to 40 of `a-z`, `0-9`, `-`, `_`, starting with a letter or a digit.
pub fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 40
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// A ticket's key: 1 to 40 of letters, digits and `-`.
fn is_key(s: &str) -> bool {
    !s.is_empty() && s.len() <= 40 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// An image's path: relative, under the working folder, by an image's name.
fn image_path(path: &str, at: &str, lang: Lang, out: &mut Vec<Problem>) {
    let clean = !path.is_empty()
        && path.len() <= 300
        && !path.starts_with(['/', '\\'])
        && !path.contains(['\\', ':', '?', '#', '%'])
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..");
    if !clean {
        out.push(Problem::new(
            at,
            tr_in!(
                lang,
                "un chemin relatif du dossier de travail, sans .. ni lien",
                "a relative path of the working folder, without .. nor link"
            ),
        ));
        return;
    }
    if !crate::asks::claims_image(path, None) {
        out.push(Problem::new(
            at,
            tr_in!(
                lang,
                "une image PNG, JPEG, GIF, WebP ou AVIF (jamais un SVG)",
                "a PNG, JPEG, GIF, WebP or AVIF image (never an SVG)"
            ),
        ));
    }
}

/// The rules a schema does not say: ids unique, a recommendation that names an option.
fn rules(template: Template, data: &Value, lang: Lang, out: &mut Vec<Problem>) {
    let twice = |list: &str, out: &mut Vec<Problem>| {
        let mut seen = std::collections::HashSet::new();
        for (i, item) in data[list].as_array().into_iter().flatten().enumerate() {
            if let Some(id) = item["id"].as_str() {
                if !seen.insert(id) {
                    out.push(Problem::new(
                        &format!("{list}.{i}.id"),
                        tr_in!(lang, "id « {id} » déjà pris", "id “{id}” already taken"),
                    ));
                }
            }
        }
    };
    match template {
        Template::Review => twice("sections", out),
        Template::Compare => {
            twice("options", out);
            if let Some(r) = data["recommend"].as_str() {
                let found = data["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|o| o["id"].as_str() == Some(r));
                if !found {
                    out.push(Problem::new(
                        "recommend",
                        tr_in!(
                            lang,
                            "« {r} » n'est pas l'id d'une option",
                            "“{r}” is not the id of an option"
                        ),
                    ));
                }
            }
        }
        Template::Diagram => {}
    }
}

/// The images a page's data names, to be copied into its version (AP-2), each once.
pub fn images(template: Template, data: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if template != Template::Compare {
        return out;
    }
    for o in data["options"].as_array().into_iter().flatten() {
        for img in o["images"].as_array().into_iter().flatten() {
            let paths: Vec<&str> = match img {
                Value::String(p) => vec![p.as_str()],
                _ => ["dark", "light"]
                    .iter()
                    .filter_map(|k| img[*k].as_str())
                    .collect(),
            };
            for p in paths {
                if !out.iter().any(|x| x == p) {
                    out.push(p.to_string());
                }
            }
        }
    }
    out
}

// ---------- the anchors ----------

/// A part of a page that `drive_browser` aims at: by a key of its template, the selector it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Anchor {
    pub key: String,
    pub selector: String,
}

fn a(key: impl Into<String>, selector: impl Into<String>) -> Anchor {
    Anchor {
        key: key.into(),
        selector: selector.into(),
    }
}

/// Every anchor of a page of `template` with this (checked) data and request. A diagram's nodes are
/// not listed: they are Mermaid's, `nodes.<id>` names any (`anchor`).
pub fn anchors(template: Template, data: &Value, choice: Option<&Choice>) -> Vec<Anchor> {
    let list = |name: &str| data[name].as_array().cloned().unwrap_or_default();
    let mut out = Vec::new();
    match template {
        Template::Review => {
            out.push(a("subject", "#subject"));
            if data["facts"].is_object() {
                out.push(a("facts", "#facts"));
            }
            let tickets = list_tickets(data);
            if !tickets.is_empty() {
                out.push(a("tabs.all", "#tab-all"));
                for (i, t) in tickets.iter().enumerate() {
                    out.push(a(format!("tabs.{t}"), format!("#tab-{}", i + 1)));
                }
            }
            for (name, short) in [
                ("criteria", "crit"),
                ("changes", "change"),
                ("risks", "risk"),
            ] {
                let items = list(name);
                if !items.is_empty() {
                    out.push(a(name, format!("#{name}")));
                }
                for i in 0..items.len() {
                    out.push(a(format!("{name}.{i}"), format!("#{short}-{}", i + 1)));
                }
            }
            for (i, s) in list("sections").iter().enumerate() {
                let id = s["id"].as_str().unwrap_or_default();
                out.push(a(format!("sections.{i}"), format!("#sec-{id}")));
                out.push(a(format!("sections.{id}"), format!("#sec-{id}")));
            }
            if let Some(c) = choice {
                out.push(a("verdict", "#verdict"));
                for o in c.options.iter().filter(|o| is_id(&o.id)) {
                    out.push(a(format!("verdict.{}", o.id), format!("#option-{}", o.id)));
                }
            }
        }
        Template::Diagram => {
            out.push(a("diagram", "#diagram"));
            if data["md"].is_string() {
                out.push(a("legend", "#legend"));
            }
        }
        Template::Compare => {
            let criteria = list("criteria");
            if !criteria.is_empty() {
                out.push(a("criteria", "#criteria"));
            }
            for i in 0..criteria.len() {
                out.push(a(format!("criteria.{i}"), format!("#ccrit-{}", i + 1)));
            }
            out.push(a("options", "#options"));
            for (i, o) in list("options").iter().enumerate() {
                let id = o["id"].as_str().unwrap_or_default();
                out.push(a(format!("options.{i}"), format!("#option-{id}")));
                out.push(a(format!("options.{id}"), format!("#option-{id}")));
            }
        }
    }
    out
}

/// The selector a key of `template` names in a page with this data (`criteria.2` → `#crit-3`,
/// `nodes.A` → the node `A` of a diagram); an error that lists the keys there are. For
/// `drive_browser` (AB-7), which aims at a page by these keys.
#[cfg_attr(not(test), allow(dead_code))]
pub fn anchor(
    template: Template,
    data: &Value,
    choice: Option<&Choice>,
    key: &str,
) -> Result<String, String> {
    if template == Template::Diagram {
        if let Some(node) = key.strip_prefix("nodes.") {
            let ok = !node.is_empty()
                && node.len() <= 64
                && node
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
            if !ok {
                return Err(format!("not a node's id: {node}"));
            }
            return Ok(format!("#diagram [data-node=\"{node}\"]"));
        }
    }
    let all = anchors(template, data, choice);
    all.iter()
        .find(|x| x.key == key)
        .map(|x| x.selector.clone())
        .ok_or_else(|| {
            let mut keys: Vec<String> = all.iter().map(|x| x.key.clone()).collect();
            if template == Template::Diagram {
                keys.push("nodes.<id>".into());
            }
            format!(
                "no part “{key}” in this page; its keys: {}",
                keys.join(", ")
            )
        })
}

/// The tickets of a lot (`subject.tickets`), each once.
fn list_tickets(data: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in data["subject"]["tickets"].as_array().into_iter().flatten() {
        if let Some(t) = t.as_str() {
            if !out.iter().any(|x| x == t) {
                out.push(t.to_string());
            }
        }
    }
    out
}

// ---------- the text, escaped ----------

/// `text` as HTML text or attribute value: nothing of it is markup.
pub fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// A `md` field as HTML: the Markdown of a message, of its few forms alone (paragraphs, lists,
/// quotes, fenced code, headings as bold lines, `code`, **bold**, *italics*, links to `http(s)`),
/// every character of the text escaped. Any HTML in it stays text.
pub fn md_html(text: &str) -> String {
    let mut out = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    let mut para: Vec<&str> = Vec::new();
    let flush = |para: &mut Vec<&str>, out: &mut String| {
        if !para.is_empty() {
            let _ = write!(out, "<p>{}</p>", inline(&para.join(" ")));
            para.clear();
        }
    };
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            flush(&mut para, &mut out);
            let fence = &t[..3];
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with(fence) {
                code.push(lines[i]);
                i += 1;
            }
            let _ = write!(out, "<pre><code>{}</code></pre>", esc(&code.join("\n")));
            i += 1;
            continue;
        }
        if t.is_empty() {
            flush(&mut para, &mut out);
            i += 1;
            continue;
        }
        if let Some(h) = heading(t) {
            flush(&mut para, &mut out);
            let _ = write!(out, "<p class=\"h\"><strong>{}</strong></p>", inline(h));
            i += 1;
            continue;
        }
        if let Some(q) = t.strip_prefix('>') {
            flush(&mut para, &mut out);
            let mut quote = vec![q.trim_start()];
            i += 1;
            while i < lines.len() {
                match lines[i].trim_start().strip_prefix('>') {
                    Some(q) => quote.push(q.trim_start()),
                    None => break,
                }
                i += 1;
            }
            let _ = write!(
                out,
                "<blockquote>{}</blockquote>",
                md_html(&quote.join("\n"))
            );
            continue;
        }
        if let Some((ordered, _)) = item(t) {
            flush(&mut para, &mut out);
            let tag = if ordered { "ol" } else { "ul" };
            let _ = write!(out, "<{tag}>");
            while i < lines.len() {
                let t = lines[i].trim_start();
                match item(t) {
                    Some((o, rest)) if o == ordered => {
                        let _ = write!(out, "<li>{}</li>", inline(rest));
                    }
                    _ => break,
                }
                i += 1;
            }
            let _ = write!(out, "</{tag}>");
            continue;
        }
        para.push(t.trim_end());
        i += 1;
    }
    flush(&mut para, &mut out);
    out
}

fn heading(t: &str) -> Option<&str> {
    let hashes = t.bytes().take_while(|b| *b == b'#').count();
    if (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') {
        Some(t[hashes..].trim())
    } else {
        None
    }
}

/// A list's item: whether it is ordered, and its text.
fn item(t: &str) -> Option<(bool, &str)> {
    for mark in ["- ", "* ", "+ "] {
        if let Some(rest) = t.strip_prefix(mark) {
            return Some((false, rest));
        }
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if (1..=3).contains(&digits) {
        let rest = &t[digits..];
        if let Some(rest) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return Some((true, rest));
        }
    }
    None
}

/// The inline forms of a line: `code`, **bold**, *italics* (or `_`), [links](https://…).
fn inline(text: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            if let Some(end) = find(&chars, i + 1, "`") {
                let code: String = chars[i + 1..end].iter().collect();
                let _ = write!(out, "<code>{}</code>", esc(&code));
                i = end + 1;
                continue;
            }
        }
        if (c == '*' || c == '_') && chars.get(i + 1) == Some(&c) {
            let mark: String = [c, c].iter().collect();
            if let Some(end) = find(&chars, i + 2, &mark).filter(|e| *e > i + 2) {
                let inner: String = chars[i + 2..end].iter().collect();
                let _ = write!(out, "<strong>{}</strong>", inline(&inner));
                i = end + 2;
                continue;
            }
        }
        if (c == '*' || c == '_') && chars.get(i + 1).is_some_and(|n| !n.is_whitespace()) {
            let left_ok = c == '*' || i == 0 || !chars[i - 1].is_alphanumeric();
            if left_ok {
                if let Some(end) = find(&chars, i + 1, &c.to_string()).filter(|e| *e > i + 1) {
                    let inner: String = chars[i + 1..end].iter().collect();
                    let _ = write!(out, "<em>{}</em>", inline(&inner));
                    i = end + 1;
                    continue;
                }
            }
        }
        if c == '[' {
            if let Some(close) = find(&chars, i + 1, "](") {
                if let Some(end) = closing(&chars, close + 2) {
                    let label: String = chars[i + 1..close].iter().collect();
                    let url: String = chars[close + 2..end].iter().collect();
                    let url = url.trim();
                    if (url.starts_with("https://") || url.starts_with("http://"))
                        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
                    {
                        let _ = write!(
                            out,
                            "<a href=\"{}\" rel=\"noreferrer noopener\">{}</a>",
                            esc(url),
                            inline(&label)
                        );
                    } else {
                        let _ = write!(out, "{}", inline(&label));
                    }
                    i = end + 1;
                    continue;
                }
            }
        }
        out.push_str(&esc(&c.to_string()));
        i += 1;
    }
    out
}

/// The `)` that closes a link's address from `from`, its own parentheses balanced.
fn closing(chars: &[char], from: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (j, c) in chars.iter().enumerate().skip(from) {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(j),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Where `mark` is next found from `from`.
fn find(chars: &[char], from: usize, mark: &str) -> Option<usize> {
    let m: Vec<char> = mark.chars().collect();
    (from..chars.len()).find(|&j| chars[j..].starts_with(&m))
}

// ---------- the words of the pages ----------

fn w(lang: Lang, fr: &'static str, en: &'static str) -> &'static str {
    match lang {
        Lang::Fr => fr,
        Lang::En => en,
    }
}

/// A count with its noun: `6 fichiers`, `1 file`.
fn n_of(lang: Lang, n: i64, fr: (&str, &str), en: (&str, &str)) -> String {
    let word = match lang {
        Lang::Fr if n <= 1 => fr.0,
        Lang::Fr => fr.1,
        Lang::En if n == 1 => en.0,
        Lang::En => en.1,
    };
    format!("{} {word}", group(n, lang))
}

/// A number with its thousands apart (a thin space in French, a comma in English).
fn group(n: i64, lang: Lang) -> String {
    let digits = n.unsigned_abs().to_string();
    let sep = w(lang, "\u{202f}", ",");
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push_str(sep);
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

// ---------- the render ----------

/// What a page of a template is made of: its document, and the images to copy beside it.
#[derive(Debug, Clone)]
pub struct Rendered {
    pub html: String,
    pub images: Vec<String>,
    pub anchors: Vec<Anchor>,
}

/// The page of `template` titled `title`, with `data` (checked first: refused with every problem).
pub fn render(
    template: Template,
    title: &str,
    data: &Value,
    look: &Look,
) -> Result<Rendered, Vec<Problem>> {
    check(template, data, look.lang)?;
    let title_count = title.chars().count();
    if title.trim().is_empty() || title_count > TITLE_MAX || title.chars().any(char::is_control) {
        return Err(vec![Problem::new(
            "title",
            tr_in!(
                look.lang,
                "une ligne de 1 à {TITLE_MAX} caractères",
                "one line of 1 to {TITLE_MAX} characters"
            ),
        )]);
    }
    let body = match template {
        Template::Review => review(data, look),
        Template::Diagram => diagram(data, look),
        Template::Compare => compare(data, look),
    };
    Ok(Rendered {
        html: document(template, title, &body, look),
        images: images(template, data),
        anchors: anchors(template, data, look.choice.as_ref()),
    })
}

fn document(template: Template, title: &str, body: &str, look: &Look) -> String {
    let lang = match look.lang {
        Lang::Fr => "fr",
        Lang::En => "en",
    };
    let tint = look
        .tint
        .as_deref()
        .filter(|t| {
            t.len() == 7 && t.starts_with('#') && t[1..].bytes().all(|b| b.is_ascii_hexdigit())
        })
        .map(|t| format!(" style=\"--p: {t}\""))
        .unwrap_or_default();
    let mermaid = if template == Template::Diagram {
        "<script src=\"_t/mermaid.js\"></script>"
    } else {
        ""
    };
    format!(
        "<!doctype html><html lang=\"{lang}\" data-theme=\"{theme}\" data-template=\"{name}\"{tint}>\
         <head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{title}</title><link rel=\"stylesheet\" href=\"_t/page.css\">{mermaid}\
         <script src=\"_t/page.js\" defer></script></head>\
         <body><main class=\"page t-{name}\">{body}</main></body></html>",
        theme = look.theme.name(),
        name = template.name(),
        title = esc(title),
    )
}

/// A section's head: its title, and a count or a word beside it.
fn head(id: &str, title: &str, aside: &str) -> String {
    let aside = if aside.is_empty() {
        String::new()
    } else {
        format!(" <span class=\"aside\">{}</span>", esc(aside))
    };
    format!("<h2 id=\"{id}-title\">{}{aside}</h2>", esc(title))
}

/// The attribute that ties an item to a ticket of a lot (its tab shows it).
fn ticket_attr(v: &Value) -> String {
    v["ticket"]
        .as_str()
        .map(|t| format!(" data-ticket=\"{}\"", esc(t)))
        .unwrap_or_default()
}

fn review(data: &Value, look: &Look) -> String {
    let lang = look.lang;
    let mut out = String::new();
    // The subject: the ticket (or the lot), the branch and its target, the loop.
    let s = &data["subject"];
    let tickets = list_tickets(data);
    out.push_str("<header id=\"subject\" class=\"subject\">");
    let mut bits: Vec<String> = Vec::new();
    if let Some(t) = s["ticket"].as_str() {
        bits.push(format!("<span class=\"key\">{}</span>", esc(t)));
    }
    if !tickets.is_empty() {
        bits.push(format!(
            "<span class=\"key\">{}</span>",
            esc(&n_of(
                lang,
                tickets.len() as i64,
                ("ticket", "tickets"),
                ("ticket", "tickets")
            ))
        ));
    }
    match (s["branch"].as_str(), s["target"].as_str()) {
        (Some(b), Some(t)) => bits.push(format!(
            "<code>{}</code> <span class=\"to\">→</span> <code>{}</code>",
            esc(b),
            esc(t)
        )),
        (Some(b), None) => bits.push(format!("<code>{}</code>", esc(b))),
        (None, Some(t)) => bits.push(format!(
            "{} <code>{}</code>",
            w(lang, "vers", "into"),
            esc(t)
        )),
        (None, None) => {}
    }
    if let Some(l) = s["loop"].as_str() {
        bits.push(format!(
            "<span class=\"loop\">{} {}</span>",
            w(lang, "boucle", "loop"),
            esc(l)
        ));
    }
    let _ = write!(
        out,
        "<p class=\"line\">{}</p>",
        bits.join("<span class=\"dot\">·</span>")
    );
    // The facts.
    let f = &data["facts"];
    if f.is_object() {
        let mut facts = Vec::new();
        if let Some(n) = f["files"].as_i64() {
            facts.push(format!(
                "<span>{}</span>",
                esc(&n_of(lang, n, ("fichier", "fichiers"), ("file", "files")))
            ));
        }
        if f["added"].is_i64() || f["removed"].is_i64() {
            facts.push(format!(
                "<span class=\"diffstat\"><span class=\"add\">+{}</span> <span class=\"del\">−{}</span></span>",
                group(f["added"].as_i64().unwrap_or(0), lang),
                group(f["removed"].as_i64().unwrap_or(0), lang)
            ));
        }
        if f["tests"].is_object() {
            let t = &f["tests"];
            let (passed, total) = (
                t["passed"].as_i64().unwrap_or(0),
                t["total"].as_i64().unwrap_or(0),
            );
            let failed = t["failed"].as_i64().unwrap_or((total - passed).max(0));
            let class = if failed > 0 { "bad" } else { "good" };
            facts.push(format!(
                "<span class=\"tests {class}\">{} {}/{}</span>",
                w(lang, "tests", "tests"),
                group(passed, lang),
                group(total, lang)
            ));
        }
        let _ = write!(
            out,
            "<p id=\"facts\" class=\"facts\">{}</p>",
            facts.join("")
        );
    }
    out.push_str("</header>");
    // A lot: a tab per ticket, declared (drive_browser's `tab`).
    if !tickets.is_empty() {
        let _ = write!(
            out,
            "<nav class=\"tabs\" role=\"tablist\" aria-label=\"{}\">\
             <button id=\"tab-all\" role=\"tab\" aria-selected=\"true\" data-tab=\"\">{}</button>",
            w(lang, "Tickets du lot", "Tickets of the batch"),
            w(lang, "Tout", "All")
        );
        for (i, t) in tickets.iter().enumerate() {
            let _ = write!(
                out,
                "<button id=\"tab-{}\" role=\"tab\" aria-selected=\"false\" data-tab=\"{t}\">{t}</button>",
                i + 1,
                t = esc(t)
            );
        }
        out.push_str("</nav>");
    }
    // The criteria.
    let criteria = data["criteria"].as_array().cloned().unwrap_or_default();
    if !criteria.is_empty() {
        let met = criteria.iter().filter(|c| c["state"] == "met").count();
        let _ = write!(
            out,
            "<section id=\"criteria\" class=\"block\" aria-labelledby=\"criteria-title\">{}<ol class=\"crits\">",
            head(
                "criteria",
                w(lang, "Critères d'acceptation", "Acceptance criteria"),
                &format!("{met}/{}", criteria.len())
            )
        );
        for (i, c) in criteria.iter().enumerate() {
            let state = c["state"].as_str().unwrap_or("unknown");
            let word = match state {
                "met" => w(lang, "Atteint", "Met"),
                "partial" => w(lang, "À moitié", "Partly"),
                "unmet" => w(lang, "Manqué", "Not met"),
                _ => w(lang, "À vérifier", "To check"),
            };
            let md = c["md"]
                .as_str()
                .map(|m| format!("<div class=\"md\">{}</div>", md_html(m)))
                .unwrap_or_default();
            let _ = write!(
                out,
                "<li id=\"crit-{n}\" class=\"crit {state}\" data-key=\"criteria.{i}\"{tk}>\
                 <span class=\"mark\" aria-hidden=\"true\"></span>\
                 <div class=\"what\"><p class=\"text\">{text}</p>{md}</div>\
                 <span class=\"state\">{word}</span></li>",
                n = i + 1,
                tk = ticket_attr(c),
                text = esc(c["text"].as_str().unwrap_or_default()),
            );
        }
        out.push_str("</ol></section>");
    }
    // The changes, each with its diff.
    let changes = data["changes"].as_array().cloned().unwrap_or_default();
    if !changes.is_empty() {
        let _ = write!(
            out,
            "<section id=\"changes\" class=\"block\" aria-labelledby=\"changes-title\">{}<ul class=\"changes\">",
            head(
                "changes",
                w(lang, "Changements", "Changes"),
                &n_of(lang, changes.len() as i64, ("fichier", "fichiers"), ("file", "files"))
            )
        );
        for (i, c) in changes.iter().enumerate() {
            let status = c["status"].as_str().unwrap_or("modified");
            let mut stat = String::new();
            if let Some(n) = c["added"].as_i64().filter(|n| *n > 0) {
                let _ = write!(stat, "<span class=\"add\">+{}</span>", group(n, lang));
            }
            if let Some(n) = c["removed"].as_i64().filter(|n| *n > 0) {
                let _ = write!(stat, " <span class=\"del\">−{}</span>", group(n, lang));
            }
            let path = esc(c["path"].as_str().unwrap_or_default());
            let row = format!(
                "<span class=\"file {status}\" aria-hidden=\"true\"></span><code class=\"path\">{path}</code>\
                 <span class=\"diffstat\">{stat}</span>"
            );
            let _ = write!(
                out,
                "<li id=\"change-{n}\" class=\"change\" data-key=\"changes.{i}\"{tk}>",
                n = i + 1,
                tk = ticket_attr(c)
            );
            match c["diff"].as_str() {
                Some(diff) => {
                    let lines = diff.lines().count();
                    let open = if lines <= DIFF_OPEN_LINES {
                        " open"
                    } else {
                        ""
                    };
                    let more = if open.is_empty() {
                        format!(
                            "<span class=\"folded\">{}</span>",
                            esc(&n_of(
                                lang,
                                lines as i64,
                                ("ligne", "lignes"),
                                ("line", "lines")
                            ))
                        )
                    } else {
                        String::new()
                    };
                    let _ = write!(
                        out,
                        "<details{open}><summary>{row}{more}</summary><pre class=\"diff\">{}</pre></details>",
                        diff_html(diff)
                    );
                }
                None => {
                    let _ = write!(out, "<div class=\"row\">{row}</div>");
                }
            }
            out.push_str("</li>");
        }
        out.push_str("</ul></section>");
    }
    // The risks.
    let risks = data["risks"].as_array().cloned().unwrap_or_default();
    if !risks.is_empty() {
        let _ = write!(
            out,
            "<section id=\"risks\" class=\"block\" aria-labelledby=\"risks-title\">{}<ul class=\"risks\">",
            head("risks", w(lang, "Risques", "Risks"), &risks.len().to_string())
        );
        for (i, r) in risks.iter().enumerate() {
            let level = r["level"].as_str().unwrap_or("medium");
            let _ = write!(
                out,
                "<li id=\"risk-{n}\" class=\"risk {level}\" data-key=\"risks.{i}\"{tk}><div class=\"md\">{}</div></li>",
                md_html(r["md"].as_str().unwrap_or_default()),
                n = i + 1,
                tk = ticket_attr(r)
            );
        }
        out.push_str("</ul></section>");
    }
    // The sections of the agent.
    for (i, s) in data["sections"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let id = s["id"].as_str().unwrap_or_default();
        let ticket = s["ticket"]
            .as_str()
            .map(|t| format!("<span class=\"key\">{}</span> ", esc(t)))
            .unwrap_or_default();
        let _ = write!(
            out,
            "<section id=\"sec-{id}\" class=\"block\" data-key=\"sections.{i}\"{tk} aria-labelledby=\"sec-{id}-title\">\
             <h2 id=\"sec-{id}-title\">{ticket}{}</h2><div class=\"md\">{}</div></section>",
            esc(s["title"].as_str().unwrap_or_default()),
            md_html(s["md"].as_str().unwrap_or_default()),
            tk = ticket_attr(s)
        );
    }
    if let Some(c) = &look.choice {
        out.push_str(&verdict(c, lang));
    }
    out
}

/// A diff, a line at a time: what it adds, takes away, and its hunks' heads, each escaped.
fn diff_html(diff: &str) -> String {
    let mut out = String::with_capacity(diff.len() + diff.len() / 4);
    for line in diff.lines() {
        let class = if line.starts_with("+++") || line.starts_with("---") {
            "meta"
        } else if line.starts_with('+') {
            "add"
        } else if line.starts_with('-') {
            "del"
        } else if line.starts_with("@@") {
            "hunk"
        } else if line.starts_with("diff ") || line.starts_with("index ") {
            "meta"
        } else {
            "ctx"
        };
        // A block each: no line break between them, or each would show as two lines.
        let _ = write!(out, "<span class=\"{class}\">{}</span>", esc(line));
    }
    out
}

/// The verdict a review asks for: the request's options, drawn; a click proposes (AP-4).
fn verdict(c: &Choice, lang: Lang) -> String {
    let mut out = format!(
        "<section id=\"verdict\" class=\"block verdict\" aria-labelledby=\"verdict-title\" data-ask=\"{key}\"{multi}{answered}>\
         <h2 id=\"verdict-title\">{} <span class=\"aside\">{key}</span></h2><p class=\"ask\">{}</p>\
         <div class=\"options\" role=\"group\">",
        w(lang, "Ton verdict", "Your verdict"),
        esc(&c.title),
        key = esc(&c.key),
        multi = if c.multi { " data-multi=\"1\"" } else { "" },
        answered = if c.answered.is_empty() { "" } else { " data-answered=\"1\"" },
    );
    for (i, o) in c.options.iter().enumerate() {
        out.push_str(&option_button(c, o, i, lang));
    }
    out.push_str("</div></section>");
    out
}

/// The letter of an option: its id when it is one letter, else A, B, C… by its place.
fn letter(o: &ChoiceOption, i: usize) -> String {
    if o.id.chars().count() == 1 {
        o.id.to_uppercase()
    } else {
        ((b'A' + (i as u8 % 26)) as char).to_string()
    }
}

fn option_button(c: &Choice, o: &ChoiceOption, i: usize, lang: Lang) -> String {
    let id = if is_id(&o.id) {
        o.id.clone()
    } else {
        format!("o{i}")
    };
    let rec = c.recommend.as_deref() == Some(o.id.as_str());
    let chosen = c.answered.contains(&o.id);
    let desc = o
        .description
        .as_deref()
        .map(|d| format!("<span class=\"d\">{}</span>", esc(d)))
        .unwrap_or_default();
    let seal = if rec {
        format!(
            "<span class=\"rec\">{}</span>",
            w(lang, "Recommandé", "Recommended")
        )
    } else {
        String::new()
    };
    format!(
        "<button id=\"option-{id}\" class=\"option{on}\" type=\"button\" data-option=\"{}\" aria-pressed=\"{chosen}\"{frozen}>\
         <span class=\"l\">{}</span><span class=\"t\"><b>{}</b>{desc}</span>{seal}</button>",
        esc(&o.id),
        esc(&letter(o, i)),
        esc(&o.label),
        on = if chosen { " on" } else { "" },
        frozen = if c.answered.is_empty() { "" } else { " disabled" },
    )
}

fn diagram(data: &Value, look: &Look) -> String {
    let lang = look.lang;
    let legend = data["md"]
        .as_str()
        .map(|m| {
            format!(
                "<div id=\"legend\" class=\"md legend\">{}</div>",
                md_html(m)
            )
        })
        .unwrap_or_default();
    format!(
        "<section id=\"diagram\" class=\"diagram\" data-kind=\"mermaid\" aria-label=\"{label}\">\
         <div class=\"tools\" role=\"toolbar\">\
         <button type=\"button\" id=\"zoom-out\" data-zoom=\"out\" aria-label=\"{out}\">−</button>\
         <button type=\"button\" id=\"zoom-fit\" data-zoom=\"fit\">{fit}</button>\
         <button type=\"button\" id=\"zoom-in\" data-zoom=\"in\" aria-label=\"{zin}\">+</button></div>\
         <div class=\"stage\" tabindex=\"0\"><div class=\"canvas\"></div></div>\
         <pre id=\"diagram-source\" class=\"source\" hidden>{source}</pre>\
         <p class=\"failed\" hidden>{failed}</p></section>{legend}",
        label = w(lang, "Schéma", "Diagram"),
        out = w(lang, "Réduire", "Zoom out"),
        zin = w(lang, "Agrandir", "Zoom in"),
        fit = w(lang, "Ajuster", "Fit"),
        failed = w(
            lang,
            "Mermaid n'a pas pu dessiner ce schéma. Sa source :",
            "Mermaid could not draw this diagram. Its source:"
        ),
        source = esc(data["source"].as_str().unwrap_or_default()),
    )
}

fn compare(data: &Value, look: &Look) -> String {
    let lang = look.lang;
    let mut out = String::new();
    let criteria = data["criteria"].as_array().cloned().unwrap_or_default();
    if !criteria.is_empty() {
        let _ = write!(
            out,
            "<section id=\"criteria\" class=\"block\" aria-labelledby=\"criteria-title\">{}<ol class=\"ccrits\">",
            head("criteria", w(lang, "Ce qui compte", "What matters"), "")
        );
        for (i, c) in criteria.iter().enumerate() {
            let _ = write!(
                out,
                "<li id=\"ccrit-{}\">{}</li>",
                i + 1,
                esc(c.as_str().unwrap_or_default())
            );
        }
        out.push_str("</ol></section>");
    }
    let options = data["options"].as_array().cloned().unwrap_or_default();
    let recommend = data["recommend"]
        .as_str()
        .map(String::from)
        .or_else(|| look.choice.as_ref().and_then(|c| c.recommend.clone()));
    let _ = write!(
        out,
        "<section id=\"options\" class=\"plates\" style=\"--n: {}\" aria-label=\"{}\">",
        options.len().min(3),
        w(lang, "Les options", "The options")
    );
    for (i, o) in options.iter().enumerate() {
        let id = o["id"].as_str().unwrap_or_default();
        let rec = recommend.as_deref() == Some(id);
        let seal = if rec {
            format!(
                "<span class=\"seal\" role=\"img\" aria-label=\"{r}\"><span>{r}</span></span>",
                r = w(lang, "Recommandé", "Recommended")
            )
        } else {
            String::new()
        };
        // Its renders: one, or a dark and a light one (« Sombre · Clair »).
        let imgs = o["images"].as_array().cloned().unwrap_or_default();
        let mut shots = String::new();
        let mut both = false;
        for img in &imgs {
            match img {
                Value::String(p) => {
                    let _ = write!(shots, "<img src=\"{}\" alt=\"\" loading=\"lazy\">", esc(p));
                }
                _ => {
                    both = true;
                    let _ = write!(
                        shots,
                        "<img class=\"for-dark\" src=\"{}\" alt=\"\" loading=\"lazy\">\
                         <img class=\"for-light\" src=\"{}\" alt=\"\" loading=\"lazy\">",
                        esc(img["dark"].as_str().unwrap_or_default()),
                        esc(img["light"].as_str().unwrap_or_default())
                    );
                }
            }
        }
        if shots.is_empty() {
            let _ = write!(
                shots,
                "<span class=\"none\">{}</span>",
                w(lang, "Sans image", "No image")
            );
        }
        let switch = if both {
            format!(
                "<div class=\"modes\" role=\"tablist\" aria-label=\"{}\">\
                 <button type=\"button\" role=\"tab\" id=\"mode-{id}-dark\" data-mode=\"dark\">{}</button>\
                 <button type=\"button\" role=\"tab\" id=\"mode-{id}-light\" data-mode=\"light\">{}</button></div>",
                w(lang, "Rendu", "Render"),
                w(lang, "Sombre", "Dark"),
                w(lang, "Clair", "Light")
            )
        } else {
            String::new()
        };
        let list = |name: &str, class: &str| {
            let items = o[name].as_array().cloned().unwrap_or_default();
            if items.is_empty() {
                return String::new();
            }
            let li: String = items
                .iter()
                .map(|x| format!("<li>{}</li>", esc(x.as_str().unwrap_or_default())))
                .collect();
            format!("<ul class=\"{class}\">{li}</ul>")
        };
        let md = o["md"]
            .as_str()
            .map(|m| format!("<div class=\"md\">{}</div>", md_html(m)))
            .unwrap_or_default();
        let pick = look
            .choice
            .as_ref()
            .and_then(|c| {
                c.options
                    .iter()
                    .position(|x| x.id == id)
                    .map(|k| (c, &c.options[k], k))
            })
            .map(|(c, x, k)| {
                let chosen = c.answered.contains(&x.id);
                format!(
                    "<button class=\"pick{on}\" type=\"button\" data-option=\"{}\" aria-pressed=\"{chosen}\"{frozen}>{} {}</button>",
                    esc(&x.id),
                    w(lang, "Choisir", "Choose"),
                    esc(&letter(x, k)),
                    on = if chosen { " on" } else { "" },
                    frozen = if c.answered.is_empty() { "" } else { " disabled" },
                )
            })
            .unwrap_or_default();
        let ask = look
            .choice
            .as_ref()
            .map(|c| {
                format!(
                    " data-ask=\"{}\"{}",
                    esc(&c.key),
                    if c.multi { " data-multi=\"1\"" } else { "" }
                )
            })
            .unwrap_or_default();
        let _ = write!(
            out,
            "<article id=\"option-{id}\" class=\"plate{rec}\" data-key=\"options.{i}\"{ask} data-theme-shown=\"dark\">{seal}\
             <div class=\"shot{both}\">{shots}</div>{switch}\
             <div class=\"cred\"><span class=\"l\">{letter}</span><b>{title}</b></div>\
             <div class=\"body\">{md}{pros}{cons}{pick}</div></article>",
            rec = if rec { " rec" } else { "" },
            both = if both { " both" } else { "" },
            letter = esc(&id.chars().next().unwrap_or('?').to_uppercase().to_string()),
            title = esc(o["title"].as_str().unwrap_or_default()),
            pros = list("pros", "pros"),
            cons = list("cons", "cons"),
        );
    }
    out.push_str("</section>");
    out
}

// ---------- the template's files ----------

const PAGE_CSS: &str = include_str!("agentpage_templates/page.css");
const PAGE_JS: &str = include_str!("agentpage_templates/page.js");
const MERMAID_GZ: &[u8] = include_bytes!("agentpage_templates/mermaid.min.js.gz");

/// The embedded Mermaid, unzipped once.
pub fn mermaid() -> &'static [u8] {
    static ONCE: OnceLock<Vec<u8>> = OnceLock::new();
    ONCE.get_or_init(|| {
        use std::io::Read;
        let mut out = Vec::with_capacity(6 << 20);
        flate2::read::GzDecoder::new(MERMAID_GZ)
            .read_to_end(&mut out)
            .map(|_| out)
            .unwrap_or_default()
    })
}

/// A file of the templates, served under `_t/` to every page: its type and its bytes.
pub fn asset(name: &str) -> Option<(&'static str, Cow<'static, [u8]>)> {
    match name {
        "page.css" => Some((
            "text/css; charset=utf-8",
            Cow::Borrowed(PAGE_CSS.as_bytes()),
        )),
        "page.js" => Some((
            "text/javascript; charset=utf-8",
            Cow::Borrowed(PAGE_JS.as_bytes()),
        )),
        "mermaid.js" => Some(("text/javascript; charset=utf-8", Cow::Borrowed(mermaid()))),
        _ => None,
    }
}
