//! The box « À toi » (docs/INBOX.md, IN-1): the asynchronous requests the agents leave for the user
//! without stopping (`ask_user` and the other tools of `mcp::asks`), kept in the state with their
//! history (`Core::asks`, `PersistedState::asks`), their files copied in the data folder
//! (`<données>/asks/<id>/files/<uniq>/<name>`, so that they outlive the agent's worktree), and
//! their answer given back to their author (a message, 10 seconds after the user answered, or read
//! by `get_ask`). An answer does nothing else: the agent acts on it, with its own safeguards. No agent
//! answers for the user, but one the user left a request to (`delegate`), and never a go-ahead nor a
//! task. The log never holds a request's text, a path or a file's name: only its id and its shape.
//!
//! The box rests on the MCP server: off, the tools are not there, and the window's commands say the
//! box is closed; the requests are kept as they are (their dates still run out, a deleted agent
//! still takes its own out), and an answer given before still goes.

use crate::core::Core;
use crate::i18n::{self, Lang};
use crate::model::{now_ms, Column, UiEvent};
use anyhow::{anyhow, bail, Result};
pub use escouade_model::asks::*;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Runtime;

// ---------- the bounds (§ 3.3, § 4.2, § 12) ----------

pub(crate) const TITLE_MAX: usize = 140;
pub(crate) const BODY_MAX: usize = 8_000;
pub(crate) const OPTIONS_MIN: usize = 2;
pub(crate) const OPTIONS_MAX: usize = 6;
pub(crate) const LABEL_MAX: usize = 60;
pub(crate) const DESCRIPTION_MAX: usize = 600;
pub(crate) const STEPS_MAX: usize = 10;
pub(crate) const STEP_MAX: usize = 300;
pub(crate) const WHY_MAX: usize = 500;
pub(crate) const UNBLOCKS_MAX: usize = 200;
pub(crate) const NOTE_MAX: usize = 2_000;
pub(crate) const TEXT_MAX: usize = 8_000;
pub(crate) const YES_NO_MAX: usize = 24;
pub(crate) const REASON_MAX: usize = 300;
pub(crate) const TICKETS_MAX: usize = 10;
/// The files of a request, its options' included.
pub(crate) const FILES_MAX: usize = 12;
const MB: u64 = 1 << 20;
pub(crate) const FILE_MAX: u64 = 25 * MB;
pub(crate) const ASK_MAX: u64 = 64 * MB;
/// The files of every request kept, together.
pub(crate) const BOX_MAX: u64 = 1024 * MB;
pub(crate) const OPEN_PER_AUTHOR: usize = 10;
pub(crate) const OPEN_PER_PROJECT: usize = 40;
pub(crate) const OPEN_MAX: usize = 200;
pub(crate) const DEPOSITS_PER_HOUR: usize = 20;
pub(crate) const EXPIRES_DEFAULT: u32 = 14;
pub(crate) const EXPIRES_MAX: u32 = 30;
/// The time to take an answer back before it goes (§ 8.6).
pub(crate) const UNDO_MS: i64 = 10_000;
/// The files of a closed request are kept that long (§ 4.4), unless « Garder les fichiers des demandes closes »
/// says 7 or 90 (`Settings::inbox_keep_days`, ESC-510).
pub(crate) const FILES_KEPT_DAYS: i64 = 30;

/// The days a closed request's files are kept, as « Garder les fichiers des demandes closes » says: 7, 30 or 90.
pub(crate) fn kept_days(setting: u32) -> i64 {
    match setting {
        7 | 90 => i64::from(setting),
        _ => FILES_KEPT_DAYS,
    }
}
/// A closed request stays in the history that long, and so many at most.
pub(crate) const KEPT_DAYS: i64 = 90;
pub(crate) const CLOSED_MAX: usize = 300;
/// The lines of a request's history kept: the first (deposited) and the latest.
const HISTORY_MAX: usize = 40;
const DAY_MS: i64 = 24 * 3600 * 1000;
const HOUR_MS: i64 = 3600 * 1000;

/// The box's requests, and the number of the last one.
#[derive(Debug, Default)]
pub struct AskBox {
    pub list: Vec<Ask>,
    pub seq: u64,
}

// ---------- what is left ----------

/// A request as its author leaves it, before it is checked.
#[derive(Debug, Clone, Default)]
pub struct AskDraft {
    pub kind: AskKind,
    pub title: String,
    pub body: String,
    pub options: Vec<AskOption>,
    pub multi: bool,
    pub steps: Vec<String>,
    pub recommend: Option<AskRecommend>,
    pub unblocks: String,
    /// The tickets that wait for the answer, by id (found by the caller).
    pub tickets: Vec<String>,
    pub urgency: Urgency,
    pub expires_in_days: Option<u32>,
    pub decision: bool,
    pub wake: bool,
    pub yes_label: String,
    pub no_label: String,
    pub gate: bool,
}

/// Where a file comes from: its bytes (given in base64), or a file opened where the caller may give
/// one, with its length.
pub enum Source {
    Bytes(Vec<u8>),
    File(std::fs::File, u64),
}

/// A file given with a request.
pub struct Incoming {
    pub name: String,
    /// What the caller says it is: an indication only, but a file said to be an image must be one.
    pub declared: Option<String>,
    /// The option it goes with.
    pub option: Option<String>,
    pub source: Source,
}

impl Incoming {
    fn length(&self) -> u64 {
        match &self.source {
            Source::Bytes(b) => b.len() as u64,
            Source::File(_, n) => *n,
        }
    }
}

/// An answer, as the window (or a delegated agent) gives it.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AnswerIn {
    pub options: Vec<String>,
    pub yes: Option<bool>,
    pub text: Option<String>,
    pub done: Option<bool>,
    pub steps: Vec<bool>,
    pub note: Option<String>,
    pub via: AskVia,
}

// ---------- the pure parts ----------

fn chars(s: &str) -> usize {
    s.chars().count()
}

/// `s` on one line, its controls and its invisible marks taken out.
pub(crate) fn one_line(s: &str) -> String {
    let line: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .filter(|c| !matches!(*c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{feff}'))
        .collect();
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether a text looks like a secret: a key or a token of the usual forms (§ 12).
pub(crate) fn looks_secret(text: &str) -> bool {
    if text.contains("-----BEGIN") && text.contains("PRIVATE KEY") {
        return true;
    }
    let token = |w: &str| {
        let body = |prefix: &str, min: usize| {
            w.strip_prefix(prefix).is_some_and(|rest| {
                rest.chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .count()
                    >= min
            })
        };
        body("ghp_", 30)
            || body("gho_", 30)
            || body("ghs_", 30)
            || body("ghu_", 30)
            || body("github_pat_", 30)
            || body("glpat-", 18)
            || body("sk-", 20)
            || body("xoxb-", 10)
            || body("xoxp-", 10)
            || body("xoxa-", 10)
            || body("xoxs-", 10)
            || body("xapp-", 10)
            || (w.starts_with("AKIA")
                && w.len() == 20
                && w.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
    };
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .any(token)
}

fn secret(lang: Lang) -> String {
    tr_in!(
        lang,
        "Ce texte ressemble à un secret (une clé ou un jeton) : ne le mets pas dans une demande.",
        "This text looks like a secret (a key or a token): don’t put it in a request."
    )
}

fn too_long(lang: Lang, what: &str, max: usize) -> String {
    tr_in!(
        lang,
        "{what} tient en {max} caractères.",
        "{what} fits in {max} characters."
    )
}

/// The draft checked and tidied (§ 3.3), or why it is refused, in one sentence.
pub(crate) fn checked(lang: Lang, mut d: AskDraft) -> Result<AskDraft, String> {
    if d.title.contains(['\n', '\r']) || chars(d.title.trim()) > TITLE_MAX {
        return Err(tr_in!(
            lang,
            "Le titre tient en {TITLE_MAX} caractères, sur une ligne.",
            "The title fits in {TITLE_MAX} characters, on one line."
        ));
    }
    d.title = one_line(&d.title);
    if d.title.is_empty() {
        return Err(tr_in!(
            lang,
            "Donne un titre : la question, en une phrase.",
            "Give a title: the question, in one sentence."
        ));
    }
    if chars(&d.body) > BODY_MAX {
        return Err(too_long(
            lang,
            &tr_in!(lang, "Le contexte (body)", "The context (body)"),
            BODY_MAX,
        ));
    }
    d.unblocks = one_line(&d.unblocks);
    if chars(&d.unblocks) > UNBLOCKS_MAX {
        return Err(too_long(
            lang,
            &tr_in!(
                lang,
                "Ce que la réponse débloque (unblocks)",
                "What the answer unblocks (unblocks)"
            ),
            UNBLOCKS_MAX,
        ));
    }
    match d.kind {
        AskKind::Choice => {
            if !(OPTIONS_MIN..=OPTIONS_MAX).contains(&d.options.len()) {
                return Err(tr_in!(
                    lang,
                    "Une demande a {OPTIONS_MIN} à {OPTIONS_MAX} options.",
                    "A choice has {OPTIONS_MIN} to {OPTIONS_MAX} options."
                ));
            }
            let mut seen = HashSet::new();
            for (i, o) in d.options.iter_mut().enumerate() {
                o.id = o.id.trim().to_ascii_lowercase();
                if o.id.is_empty() {
                    o.id = ((b'a' + i as u8) as char).to_string();
                }
                if !(o.id.len() == 1 && ('a'..='f').contains(&o.id.chars().next().unwrap_or(' '))) {
                    return Err(tr_in!(
                        lang,
                        "L’id d’une option est une lettre, de a à f.",
                        "An option’s id is a letter, a to f."
                    ));
                }
                if !seen.insert(o.id.clone()) {
                    return Err(tr_in!(
                        lang,
                        "Deux options ont l’id « {id} ».",
                        "Two options have the id “{id}”.",
                        id = o.id
                    ));
                }
                o.label = one_line(&o.label);
                if o.label.is_empty() || chars(&o.label) > LABEL_MAX {
                    return Err(tr_in!(
                        lang,
                        "Le libellé d’une option tient en 1 à {LABEL_MAX} caractères.",
                        "An option’s label fits in 1 to {LABEL_MAX} characters."
                    ));
                }
                if chars(&o.description) > DESCRIPTION_MAX {
                    return Err(too_long(
                        lang,
                        &tr_in!(
                            lang,
                            "La description d’une option",
                            "An option’s description"
                        ),
                        DESCRIPTION_MAX,
                    ));
                }
            }
        }
        _ if !d.options.is_empty() => {
            return Err(tr_in!(
                lang,
                "Seul un choix (kind: choice) a des options.",
                "Only a choice (kind: choice) has options."
            ));
        }
        _ => {}
    }
    if d.kind == AskKind::Task {
        if d.steps.len() > STEPS_MAX {
            return Err(tr_in!(
                lang,
                "Une tâche a {STEPS_MAX} étapes au plus.",
                "A task has {STEPS_MAX} steps at most."
            ));
        }
        for s in &mut d.steps {
            *s = one_line(s);
            if s.is_empty() || chars(s) > STEP_MAX {
                return Err(tr_in!(
                    lang,
                    "Une étape tient en 1 à {STEP_MAX} caractères.",
                    "A step fits in 1 to {STEP_MAX} characters."
                ));
            }
        }
    } else if !d.steps.is_empty() {
        return Err(tr_in!(
            lang,
            "Seule une tâche (kind: task) a des étapes.",
            "Only a task (kind: task) has steps."
        ));
    }
    if d.kind == AskKind::Confirm {
        d.yes_label = one_line(&d.yes_label);
        d.no_label = one_line(&d.no_label);
        if chars(&d.yes_label) > YES_NO_MAX || chars(&d.no_label) > YES_NO_MAX {
            return Err(too_long(
                lang,
                &tr_in!(lang, "Un libellé de oui ou de non", "A yes or no label"),
                YES_NO_MAX,
            ));
        }
    } else {
        d.yes_label.clear();
        d.no_label.clear();
        d.gate = false;
    }
    if d.kind != AskKind::Choice {
        d.multi = false;
    }
    if let Some(r) = &mut d.recommend {
        if chars(&r.why) > WHY_MAX {
            return Err(too_long(
                lang,
                &tr_in!(
                    lang,
                    "Le pourquoi de la recommandation",
                    "The recommendation’s why"
                ),
                WHY_MAX,
            ));
        }
        if let Some(o) = &mut r.option {
            *o = o.trim().to_ascii_lowercase();
            if d.kind != AskKind::Choice || !d.options.iter().any(|x| x.id == *o) {
                return Err(tr_in!(
                    lang,
                    "L’option recommandée « {o} » n’est pas une des options.",
                    "The recommended option “{o}” isn’t one of the options.",
                    o = one_line(o)
                ));
            }
        }
        if r.text.as_ref().is_some_and(|t| chars(t) > TEXT_MAX) {
            return Err(too_long(
                lang,
                &tr_in!(lang, "Le texte proposé", "The text offered"),
                TEXT_MAX,
            ));
        }
    }
    if d.tickets.len() > TICKETS_MAX {
        return Err(tr_in!(
            lang,
            "{TICKETS_MAX} tickets au plus attendent une demande.",
            "{TICKETS_MAX} tickets at most wait for a request."
        ));
    }
    if let Some(days) = d.expires_in_days {
        if !(1..=EXPIRES_MAX).contains(&days) {
            return Err(tr_in!(
                lang,
                "Une demande vit de 1 à {EXPIRES_MAX} jours (expiresInDays).",
                "A request lives 1 to {EXPIRES_MAX} days (expiresInDays)."
            ));
        }
    }
    let texts = [d.title.as_str(), &d.body, &d.unblocks]
        .into_iter()
        .chain(
            d.options
                .iter()
                .flat_map(|o| [o.label.as_str(), &o.description]),
        )
        .chain(d.steps.iter().map(String::as_str))
        .chain(
            d.recommend
                .iter()
                .flat_map(|r| [r.why.as_str(), r.text.as_deref().unwrap_or("")]),
        );
    for t in texts {
        if looks_secret(t) {
            return Err(secret(lang));
        }
    }
    Ok(d)
}

/// What the bytes of a file are, as their content says; `name` only tells a text's sort and a
/// Word document from another archive.
pub(crate) fn sniff(b: &[u8], name: &str) -> (AskFileKind, &'static str) {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    if let Some(t) = crate::sentfiles::image_kind(b).filter(|t| *t != "image/svg+xml") {
        return (AskFileKind::Image, t);
    }
    if b.len() >= 12 && &b[4..8] == b"ftyp" && matches!(&b[8..12], b"avif" | b"avis") {
        return (AskFileKind::Image, "image/avif");
    }
    if b.starts_with(b"%PDF-") {
        return (AskFileKind::Pdf, "application/pdf");
    }
    if b.starts_with(b"PK\x03\x04") {
        let has = |needle: &[u8]| b.windows(needle.len()).any(|w| w == needle);
        if ext == "docx" && has(b"word/") {
            return (
                AskFileKind::Word,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            );
        }
        return (AskFileKind::Other, "application/zip");
    }
    let head = &b[..b.len().min(8192)];
    let text = !head.contains(&0) && std::str::from_utf8(b).is_ok();
    if text {
        let lower = String::from_utf8_lossy(head).to_ascii_lowercase();
        if lower.trim_start().starts_with('<') && lower.contains("<svg") {
            return (AskFileKind::Other, "image/svg+xml");
        }
        return match ext.as_str() {
            "md" | "markdown" => (AskFileKind::Markdown, "text/markdown"),
            "csv" => (AskFileKind::Csv, "text/csv"),
            "tsv" => (AskFileKind::Csv, "text/tab-separated-values"),
            _ => (AskFileKind::Other, "text/plain"),
        };
    }
    (AskFileKind::Other, "application/octet-stream")
}

/// Whether a file says it is an image: by its name's extension, or by the type its caller declared.
pub(crate) fn claims_image(name: &str, declared: Option<&str>) -> bool {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif"
    ) || declared.is_some_and(|d| {
        let d = d.trim().to_ascii_lowercase();
        d.starts_with("image/") && d != "image/svg+xml"
    })
}

pub(crate) fn not_an_image(lang: Lang, name: &str) -> String {
    tr_in!(
        lang,
        "Le fichier « {name} » n’est pas une image PNG, JPEG, GIF, WebP ou AVIF.",
        "The file “{name}” isn’t a PNG, JPEG, GIF, WebP or AVIF image."
    )
}

pub(crate) fn too_many_files(lang: Lang, n: usize) -> String {
    tr_in!(
        lang,
        "{FILES_MAX} fichiers au plus par demande ({n} donnés).",
        "{FILES_MAX} files at most per request ({n} given)."
    )
}

pub(crate) fn file_too_big(lang: Lang, name: &str) -> String {
    tr_in!(lang, "« {name} » dépasse 25 Mo.", "“{name}” is over 25 MB.")
}

fn ask_too_heavy(lang: Lang) -> String {
    tr_in!(
        lang,
        "Les fichiers d’une demande pèsent 64 Mo au plus ensemble.",
        "A request’s files weigh 64 MB at most together."
    )
}

fn box_full(lang: Lang) -> String {
    tr_in!(
        lang,
        "La boîte a atteint 1 Go de fichiers : joins moins, ou plus petit.",
        "The box has reached 1 GB of files: attach fewer, or smaller."
    )
}

/// Who an author is, for the bounds: an agent by its id, Claude outside Escouade as one.
fn author_key(a: &AskAuthor) -> &str {
    a.agent.as_deref().unwrap_or("")
}

/// Whether one more request of `author` in `project_id` fits (§ 12), now.
pub(crate) fn room(
    lang: Lang,
    asks: &[Ask],
    author: &AskAuthor,
    project_id: &str,
    now: i64,
) -> Result<(), String> {
    let open = |a: &&Ask| a.state.waiting();
    let mine = |a: &&Ask| author_key(&a.author) == author_key(author);
    if asks.iter().filter(open).filter(mine).count() >= OPEN_PER_AUTHOR {
        return Err(tr_in!(
            lang,
            "Tu as déjà {OPEN_PER_AUTHOR} demandes ouvertes : retire celles qui n’ont plus lieu d’être (withdraw_ask).",
            "You already have {OPEN_PER_AUTHOR} open requests: withdraw the ones that no longer apply (withdraw_ask)."
        ));
    }
    if asks
        .iter()
        .filter(open)
        .filter(|a| a.project_id == project_id)
        .count()
        >= OPEN_PER_PROJECT
    {
        return Err(tr_in!(
            lang,
            "Le projet a {OPEN_PER_PROJECT} demandes ouvertes : la boîte de l’utilisateur est pleine.",
            "The project has {OPEN_PER_PROJECT} open requests: the user’s box is full."
        ));
    }
    if asks.iter().filter(open).count() >= OPEN_MAX {
        return Err(tr_in!(
            lang,
            "La boîte a {OPEN_MAX} demandes ouvertes : elle est pleine.",
            "The box has {OPEN_MAX} open requests: it is full."
        ));
    }
    if asks
        .iter()
        .filter(mine)
        .filter(|a| a.created_at > now - HOUR_MS)
        .count()
        >= DEPOSITS_PER_HOUR
    {
        return Err(tr_in!(
            lang,
            "{DEPOSITS_PER_HOUR} dépôts dans l’heure : regroupe tes demandes.",
            "{DEPOSITS_PER_HOUR} requests within the hour: group your requests."
        ));
    }
    Ok(())
}

/// A line of history, the history kept short: its first line and its latest.
pub(crate) fn record(a: &mut Ask, at: i64, what: &str, detail: String) {
    a.history.push(AskEvent {
        at,
        what: what.into(),
        detail,
    });
    if a.history.len() > HISTORY_MAX {
        a.history.remove(1);
    }
}

/// The answer checked against what the request asks (§ 3.2), or why not.
pub(crate) fn checked_answer(lang: Lang, a: &Ask, mut ans: AnswerIn) -> Result<AnswerIn, String> {
    if ans.note.as_ref().is_some_and(|n| chars(n) > NOTE_MAX) {
        return Err(too_long(
            lang,
            &tr_in!(lang, "Le mot pour l’agent", "The word for the agent"),
            NOTE_MAX,
        ));
    }
    ans.note = ans
        .note
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    let wrong = |what: &str| {
        Err(tr_in!(
            lang,
            "Cette demande est {kind} : la réponse donne {what}.",
            "This request is {kind}: the answer gives {what}.",
            kind = a.kind.as_str()
        ))
    };
    match a.kind {
        AskKind::Choice => {
            ans.options = ans
                .options
                .iter()
                .map(|o| o.trim().to_ascii_lowercase())
                .collect();
            ans.options.dedup();
            let n = ans.options.len();
            if n == 0 || (!a.multi && n > 1) {
                return wrong(if a.multi { "options" } else { "options (one)" });
            }
            if let Some(bad) = ans
                .options
                .iter()
                .find(|o| !a.options.iter().any(|x| x.id == **o))
            {
                return Err(tr_in!(
                    lang,
                    "« {bad} » n’est pas une des options.",
                    "“{bad}” isn’t one of the options.",
                    bad = one_line(bad)
                ));
            }
        }
        AskKind::Confirm if ans.yes.is_none() => return wrong("yes"),
        AskKind::Answer => {
            let text = ans.text.as_deref().map(str::trim).unwrap_or_default();
            if text.is_empty() {
                return wrong("text");
            }
            if chars(text) > TEXT_MAX {
                return Err(too_long(
                    lang,
                    &tr_in!(lang, "La réponse", "The answer"),
                    TEXT_MAX,
                ));
            }
            ans.text = Some(text.to_string());
        }
        AskKind::Task => {
            if ans.done.is_none() {
                return wrong("done");
            }
            ans.steps.resize(a.steps.len(), false);
        }
        _ => {}
    }
    Ok(ans)
}

/// What an answer says, for its author: the sentence after « [Escouade · ta demande …] ».
pub(crate) fn answer_text(lang: Lang, a: &Ask, ans: &AskAnswer) -> String {
    let who = match &ans.by_name {
        Some(name) => tr_in!(
            lang,
            "{name} (à qui l’utilisateur l’a laissée)",
            "{name} (to whom the user left it)"
        ),
        None => tr_in!(lang, "L’utilisateur", "The user"),
    };
    let what = match a.kind {
        AskKind::Choice => {
            let chosen: Vec<String> = ans
                .options
                .iter()
                .map(|id| {
                    let label = a
                        .options
                        .iter()
                        .find(|o| o.id == *id)
                        .map(|o| o.label.as_str())
                        .unwrap_or("");
                    format!("{} · {label}", id.to_ascii_uppercase())
                })
                .collect();
            tr_in!(
                lang,
                "{who} a choisi {c}.",
                "{who} chose {c}.",
                c = chosen.join(", ")
            )
        }
        AskKind::Confirm => match ans.yes {
            Some(true) => tr_in!(lang, "{who} a répondu oui.", "{who} answered yes."),
            _ => tr_in!(lang, "{who} a répondu non.", "{who} answered no."),
        },
        AskKind::Answer => tr_in!(
            lang,
            "{who} a répondu : « {t} »",
            "{who} answered: “{t}”",
            t = ans.text.as_deref().unwrap_or("")
        ),
        AskKind::Task => {
            let steps: Vec<String> = ans
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{} {}", i + 1, if *s { "✓" } else { "✗" }))
                .collect();
            let steps = if steps.is_empty() {
                String::new()
            } else {
                tr_in!(lang, " Étapes : {s}.", " Steps: {s}.", s = steps.join(", "))
            };
            match ans.done {
                Some(true) => tr_in!(lang, "{who} l’a fait.{steps}", "{who} did it.{steps}"),
                _ => tr_in!(
                    lang,
                    "{who} ne le fera pas.{steps}",
                    "{who} won’t do it.{steps}"
                ),
            }
        }
    };
    let mut out = format!(
        "{} {what}",
        tr_in!(
            lang,
            "[Escouade · ta demande {id} « {title} »]",
            "[Escouade · your request {id} “{title}”]",
            id = a.id,
            title = a.title
        )
    );
    if let Some(note) = &ans.note {
        out.push_str(&tr_in!(
            lang,
            " Son mot : « {note} »",
            " Their word: “{note}”"
        ));
    }
    let theirs: Vec<usize> = a
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.by_user)
        .map(|(i, _)| i)
        .collect();
    if !theirs.is_empty() {
        let list = theirs
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&tr_in!(
            lang,
            " Fichiers joints à la réponse : get_ask_file (ask {id}, index {list}).",
            " Files joined to the answer: get_ask_file (ask {id}, index {list}).",
            id = a.id
        ));
    }
    if a.decision {
        out.push_str(&tr_in!(
            lang,
            " Cette réponse tranche une décision du produit : propose-la (propose_decision), avec ce qui a été choisi et pourquoi.",
            " This answer settles a decision of the product: propose it (propose_decision), with what was chosen and why."
        ));
    }
    out.push_str(&tr_in!(
        lang,
        " Rien d’autre n’a été fait : c’est à toi d’agir, avec tes propres garde-fous.",
        " Nothing else was done: it is yours to act on, with your own safeguards."
    ));
    out
}

/// The ticket waits for a request whose answer has not gone yet.
pub(crate) fn waiting_tickets(asks: &[Ask]) -> HashSet<String> {
    asks.iter()
        .filter(|a| {
            a.state.waiting()
                || (a.state == AskState::Answered
                    && a.answer.as_ref().is_some_and(|x| x.sent_at.is_none()))
        })
        .flat_map(|a| a.tickets.iter().cloned())
        .collect()
}

// ---------- the files ----------

/// The copy of a request's file in its folder `dir`: None for a name that is not a valid one (a
/// hand-edited state), which never leads out.
pub(crate) fn file_path(dir: &Path, file: &str) -> Option<PathBuf> {
    let (uniq, name) = crate::ticketfiles::valid_file(file)?;
    Some(dir.join("files").join(uniq).join(name))
}

/// Copies `files` into `dir` (a folder of its own), each read up to its bound and typed by its
/// content; `kept`: what the request's files weigh already, `box_weight` the box's. Nothing is
/// left in `dir` when one fails: the caller removes it.
pub(crate) fn copy_files(
    lang: Lang,
    dir: &Path,
    files: Vec<Incoming>,
    kept: u64,
    box_weight: u64,
    by_user: bool,
) -> Result<Vec<AskFile>, String> {
    let mut total = kept;
    let mut out = Vec::with_capacity(files.len());
    for f in files {
        let name = crate::attachfiles::safe_name(&f.name);
        // The length known before a byte is read.
        if f.length() > FILE_MAX {
            return Err(file_too_big(lang, &name));
        }
        if total + f.length() > ASK_MAX {
            return Err(ask_too_heavy(lang));
        }
        if box_weight + total + f.length() > BOX_MAX {
            return Err(box_full(lang));
        }
        let bytes = match f.source {
            Source::Bytes(b) => b,
            Source::File(file, _) => {
                let mut b = Vec::new();
                file.take(FILE_MAX + 1).read_to_end(&mut b).map_err(|e| {
                    tr_in!(
                        lang,
                        "Le fichier est illisible : {e}",
                        "The file can’t be read: {e}"
                    )
                })?;
                b
            }
        };
        let size = bytes.len() as u64;
        if size > FILE_MAX {
            return Err(file_too_big(lang, &name));
        }
        total += size;
        if total > ASK_MAX {
            return Err(ask_too_heavy(lang));
        }
        let (kind, media_type) = sniff(&bytes, &name);
        if kind != AskFileKind::Image && claims_image(&name, f.declared.as_deref()) {
            return Err(not_an_image(lang, &name));
        }
        let uniq = crate::attachfiles::uniq();
        let file = format!("{uniq}/{name}");
        let path = file_path(dir, &file).ok_or_else(|| not_an_image(lang, &name))?;
        std::fs::create_dir_all(path.parent().unwrap_or(dir))
            .and_then(|()| crate::paths::write_atomic(&path, &bytes))
            .map_err(|e| {
                tr_in!(
                    lang,
                    "La copie du fichier a échoué : {e}",
                    "Copying the file failed: {e}"
                )
            })?;
        out.push(AskFile {
            file,
            name,
            kind,
            media_type: media_type.into(),
            size,
            option: f.option,
            by_user,
        });
    }
    Ok(out)
}

// ---------- the core ----------

/// Who calls on a request: the user at the window, or an agent (Claude outside Escouade for None).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asker {
    Agent(String),
    External,
}

impl Asker {
    fn is(&self, a: &AskAuthor) -> bool {
        match self {
            Self::Agent(id) => a.agent.as_deref() == Some(id),
            Self::External => a.agent.is_none(),
        }
    }
}

/// The changes `ask_update` makes: what is None stays.
#[derive(Debug, Default)]
pub struct AskPatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub options: Option<Vec<AskOption>>,
    pub multi: Option<bool>,
    pub steps: Option<Vec<String>>,
    pub recommend: Option<Option<AskRecommend>>,
    pub unblocks: Option<String>,
    pub urgency: Option<Urgency>,
    pub expires_in_days: Option<u32>,
    pub yes_label: Option<String>,
    pub no_label: Option<String>,
    /// The files to take away, by their `file`.
    pub remove_files: Vec<String>,
}

fn not_found(lang: Lang, id: &str) -> anyhow::Error {
    anyhow!(tr_in!(
        lang,
        "Aucune demande « {id} ».",
        "No request “{id}”.",
        id = crate::asks::one_line(id)
            .chars()
            .take(40)
            .collect::<String>()
    ))
}

impl<R: Runtime> Core<R> {
    /// The box is open: the MCP server is wanted (« Les agents peuvent piloter Escouade »).
    pub fn inbox_enabled(&self) -> bool {
        self.settings.read().mcp_enabled
    }

    fn inbox_closed(&self) -> anyhow::Error {
        anyhow!(tr!(
            "La boîte « À toi » est fermée avec le serveur MCP : allume-le dans Réglages › Agents › Piloter Escouade.",
            "The “Your turn” box is closed with the MCP server: turn it on in Settings › Agents › Drive Escouade."
        ))
    }

    fn inbox_told(&self, project_id: &str) {
        self.request_save();
        self.hub.emit(UiEvent::Inbox {
            project_id: project_id.to_string(),
        });
    }

    /// A request, as it is now.
    pub fn ask(&self, id: &str) -> Result<Ask> {
        self.asks
            .lock()
            .list
            .iter()
            .find(|a| a.id == id)
            .cloned()
            .ok_or_else(|| not_found(i18n::ui(), id))
    }

    /// Every request kept, newest first.
    pub fn asks_all(&self) -> Vec<Ask> {
        let mut all = self.asks.lock().list.clone();
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
        all
    }

    /// The folder of request `id`'s files.
    pub(crate) fn ask_dir(&self, id: &str) -> PathBuf {
        self.data.ask_dir(id)
    }

    /// Leaves a request (`ask_user`): checked, its files copied, kept. Refused, nothing is left (no
    /// request, no file).
    pub fn ask_deposit(
        self: &Arc<Self>,
        author: AskAuthor,
        project_id: &str,
        ticket_id: Option<String>,
        draft: AskDraft,
        files: Vec<Incoming>,
    ) -> Result<Ask> {
        let lang = i18n::ui();
        let d = checked(lang, draft).map_err(|e| anyhow!(e))?;
        if files.len() > FILES_MAX {
            bail!(too_many_files(lang, files.len()));
        }
        for f in &files {
            if let Some(o) = &f.option {
                if !d.options.iter().any(|x| x.id == *o) {
                    bail!(tr!(
                        "Un fichier va avec l’option « {o} », qui n’est pas une des options.",
                        "A file goes with the option “{o}”, which isn’t one of the options.",
                        o = one_line(o)
                    ));
                }
            }
        }
        let now = now_ms();
        let box_weight = {
            let b = self.asks.lock();
            room(lang, &b.list, &author, project_id, now).map_err(|e| anyhow!(e))?;
            b.list.iter().map(Ask::weight).sum::<u64>()
        };
        // Copied in a folder of its own, renamed once the request is kept.
        let staging = self
            .data
            .asks()
            .join(format!(".new-{}", uuid::Uuid::new_v4().simple()));
        let copied = copy_files(lang, &staging, files, 0, box_weight, false);
        let copied = match copied {
            Ok(c) => c,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&staging);
                bail!(e);
            }
        };
        let ticket_started = ticket_id
            .as_deref()
            .and_then(|t| self.ticket(t).ok())
            .is_some_and(|t| t.column != Column::Todo);
        let kept = {
            let mut b = self.asks.lock();
            // Checked again: another request may have come in meanwhile.
            if let Err(e) = room(lang, &b.list, &author, project_id, now) {
                drop(b);
                let _ = std::fs::remove_dir_all(&staging);
                bail!(e);
            }
            b.seq += 1;
            let id = format!("a-{}", b.seq);
            if !copied.is_empty() {
                let dir = self.ask_dir(&id);
                let _ = std::fs::remove_dir_all(&dir);
                if let Err(e) = std::fs::create_dir_all(self.data.asks())
                    .and_then(|()| std::fs::rename(&staging, &dir))
                {
                    drop(b);
                    let _ = std::fs::remove_dir_all(&staging);
                    bail!(tr!(
                        "La copie des fichiers a échoué : {e}",
                        "Copying the files failed: {e}"
                    ));
                }
            }
            let mut a = Ask {
                id,
                project_id: project_id.to_string(),
                ticket_id,
                ticket_started,
                author,
                kind: d.kind,
                title: d.title,
                body: d.body,
                options: d.options,
                multi: d.multi,
                steps: d.steps,
                recommend: d.recommend,
                unblocks: d.unblocks,
                tickets: d.tickets,
                urgency: d.urgency,
                expires_at: now + i64::from(d.expires_in_days.unwrap_or(EXPIRES_DEFAULT)) * DAY_MS,
                decision: d.decision,
                wake: d.wake,
                yes_label: d.yes_label,
                no_label: d.no_label,
                gate: d.gate,
                files: copied,
                state: AskState::Open,
                created_at: now,
                updated_at: now,
                ..Default::default()
            };
            let by = a.author.name.clone();
            record(&mut a, now, "deposited", by);
            b.list.push(a.clone());
            a
        };
        let _ = std::fs::remove_dir_all(&staging);
        log::info!(
            "ask {} deposited by {} ({}, {} options, {} files)",
            kept.id,
            kept.author
                .agent
                .as_deref()
                .map_or("an external client".to_string(), |id| format!(
                    "agent {}",
                    short(id)
                )),
            kept.kind.as_str(),
            kept.options.len(),
            kept.files.len()
        );
        self.inbox_told(project_id);
        // Its summary, once the requests that come with it are gathered (ESC-513).
        self.inbox_summary_soon();
        self.telemetry.track(crate::telemetry::Event::AskCreated {
            kind: kept.kind.as_str(),
            author: match (&kept.author.agent, kept.author.chef) {
                (None, _) => "client",
                (Some(_), true) => "chef",
                (Some(_), false) => "agent",
            },
            files: match kept.files.len() {
                0 => "0",
                1..=3 => "1-3",
                _ => "4-12",
            },
            recommendation: kept.recommend.is_some(),
            decision: kept.decision,
        });
        Ok(kept)
    }

    /// The request `id` of `asker`, open or put off, changed by `f` under the box's lock.
    fn own_waiting<T>(
        &self,
        asker: &Asker,
        id: &str,
        f: impl FnOnce(&mut Ask) -> Result<T>,
    ) -> Result<T> {
        let lang = i18n::ui();
        let mut b = self.asks.lock();
        let a = b
            .list
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| not_found(lang, id))?;
        if !asker.is(&a.author) {
            bail!(tr!(
                "Seul l’auteur d’une demande la change ou la retire.",
                "Only a request’s author changes or withdraws it."
            ));
        }
        if !a.state.waiting() {
            bail!(tr!(
                "La demande {id} n’attend plus : elle est {state}.",
                "Request {id} no longer waits: it is {state}.",
                state = a.state.as_str()
            ));
        }
        f(a)
    }

    /// `update_ask`: the fields given change, the others stay; files added and taken away.
    pub fn ask_update(
        &self,
        asker: &Asker,
        id: &str,
        patch: AskPatch,
        add: Vec<Incoming>,
    ) -> Result<Ask> {
        let lang = i18n::ui();
        let now = now_ms();
        let before = self.ask(id)?;
        if !asker.is(&before.author) || !before.state.waiting() {
            // The same refusal as under the lock.
            self.own_waiting(asker, id, |_| Ok(()))?;
        }
        let mut draft = AskDraft {
            kind: before.kind,
            title: patch.title.clone().unwrap_or_else(|| before.title.clone()),
            body: patch.body.clone().unwrap_or_else(|| before.body.clone()),
            options: patch
                .options
                .clone()
                .unwrap_or_else(|| before.options.clone()),
            multi: patch.multi.unwrap_or(before.multi),
            steps: patch.steps.clone().unwrap_or_else(|| before.steps.clone()),
            recommend: patch
                .recommend
                .clone()
                .unwrap_or_else(|| before.recommend.clone()),
            unblocks: patch
                .unblocks
                .clone()
                .unwrap_or_else(|| before.unblocks.clone()),
            tickets: before.tickets.clone(),
            urgency: patch.urgency.unwrap_or(before.urgency),
            expires_in_days: patch.expires_in_days,
            decision: before.decision,
            wake: before.wake,
            yes_label: patch
                .yes_label
                .clone()
                .unwrap_or_else(|| before.yes_label.clone()),
            no_label: patch
                .no_label
                .clone()
                .unwrap_or_else(|| before.no_label.clone()),
            gate: before.gate,
        };
        draft = checked(lang, draft).map_err(|e| anyhow!(e))?;
        for r in &patch.remove_files {
            if !before.files.iter().any(|f| f.file == *r) {
                bail!(tr!(
                    "La demande n’a pas de fichier « {f} ».",
                    "The request has no file “{f}”.",
                    f = one_line(r)
                ));
            }
        }
        let staying: Vec<AskFile> = before
            .files
            .iter()
            .filter(|f| !patch.remove_files.contains(&f.file))
            // A file of an option that went goes with it.
            .filter(|f| {
                f.option
                    .as_ref()
                    .is_none_or(|o| draft.options.iter().any(|x| x.id == *o))
            })
            .cloned()
            .collect();
        if staying.len() + add.len() > FILES_MAX {
            bail!(too_many_files(lang, staying.len() + add.len()));
        }
        for f in &add {
            if let Some(o) = &f.option {
                if !draft.options.iter().any(|x| x.id == *o) {
                    bail!(tr!(
                        "Un fichier va avec l’option « {o} », qui n’est pas une des options.",
                        "A file goes with the option “{o}”, which isn’t one of the options.",
                        o = one_line(o)
                    ));
                }
            }
        }
        let kept_weight: u64 = staying.iter().map(|f| f.size).sum();
        let box_weight: u64 = self
            .asks
            .lock()
            .list
            .iter()
            .filter(|a| a.id != id)
            .map(Ask::weight)
            .sum();
        let added = add.len();
        let new_files = copy_files(lang, &self.ask_dir(id), add, kept_weight, box_weight, false)
            .map_err(|e| anyhow!(e))?;
        let gone: Vec<AskFile> = before
            .files
            .iter()
            .filter(|f| !staying.contains(f))
            .cloned()
            .collect();
        let mut changed = Vec::new();
        let result = self.own_waiting(asker, id, |a| {
            if a.options != draft.options {
                changed.push("options");
            }
            if a.title != draft.title || a.body != draft.body {
                changed.push("text");
            }
            a.title = draft.title;
            a.body = draft.body;
            a.options = draft.options;
            a.multi = draft.multi;
            a.steps = draft.steps;
            a.recommend = draft.recommend;
            a.unblocks = draft.unblocks;
            a.urgency = draft.urgency;
            a.yes_label = draft.yes_label;
            a.no_label = draft.no_label;
            if let Some(days) = draft.expires_in_days {
                a.expires_at = now + i64::from(days) * DAY_MS;
            }
            a.files = staying.clone();
            a.files.extend(new_files.clone());
            a.updated_at = now;
            a.seen = false;
            let mut detail = changed.join(", ");
            if added > 0 {
                detail.push_str(&format!(
                    "{}+{added} files",
                    if detail.is_empty() { "" } else { ", " }
                ));
            }
            if !gone.is_empty() {
                detail.push_str(&format!(
                    "{}-{} files",
                    if detail.is_empty() { "" } else { ", " },
                    gone.len()
                ));
            }
            record(a, now, "updated", detail);
            Ok(a.clone())
        });
        match &result {
            Ok(a) => {
                for f in &gone {
                    remove_file(&self.ask_dir(id), &f.file);
                }
                log::info!("ask {id} updated");
                self.inbox_told(&a.project_id);
            }
            Err(_) => {
                for f in &new_files {
                    remove_file(&self.ask_dir(id), &f.file);
                }
            }
        }
        result
    }

    /// `withdraw_ask`: its author takes it back, with its reason: the note the user reads in its trace (« tranché
    /// dans la conversation du chef : B », ESC-557), never a secret.
    pub fn ask_withdraw(self: &Arc<Self>, asker: &Asker, id: &str, reason: &str) -> Result<Ask> {
        let reason = one_line(reason);
        if chars(&reason) > REASON_MAX {
            bail!(too_long(
                i18n::ui(),
                &tr!("La raison", "The reason"),
                REASON_MAX
            ));
        }
        if looks_secret(&reason) {
            bail!(secret(i18n::ui()));
        }
        let now = now_ms();
        let a = self.own_waiting(asker, id, |a| {
            a.state = AskState::Withdrawn;
            a.closed_at = Some(now);
            a.reason = (!reason.is_empty()).then(|| reason.clone());
            a.seen = false;
            let by = a.author.name.clone();
            record(a, now, "withdrawn", by);
            Ok(a.clone())
        })?;
        log::info!("ask {id} withdrawn");
        self.telemetry.track(crate::telemetry::Event::AskWithdrawn {
            kind: a.kind.as_str(),
        });
        self.block_waiting(&a);
        self.inbox_told(&a.project_id);
        Ok(a)
    }

    /// The tickets that waited for `a`, which will never be answered now, blocked with why.
    fn block_waiting(self: &Arc<Self>, a: &Ask) {
        for t in &a.tickets {
            let why = tr!(
                "La demande {id} qu’il attendait ne sera pas répondue ({state}) : relance-le à la main, ou retire-la de ce qu’il attend.",
                "The request {id} it waited for will not be answered ({state}): start it by hand, or take it out of what it waits for.",
                id = a.id,
                state = a.state.as_str()
            );
            let blocked = self.edit_ticket(t, |t| {
                if t.column == Column::Todo && t.blocked.is_none() {
                    t.blocked = Some(why);
                    return Ok(true);
                }
                Ok(false)
            });
            if blocked.unwrap_or(false) {
                self.notify_ticket(t, false);
            }
        }
    }

    /// The author read the answer (`get_ask`, `wait_ask`, or its message delivered).
    pub fn ask_mark_read(&self, id: &str) -> Option<Ask> {
        let now = now_ms();
        let mut b = self.asks.lock();
        let a = b.list.iter_mut().find(|a| a.id == id)?;
        let went = a.answer.as_ref().is_some_and(|x| x.send_at.is_none());
        if a.state != AskState::Answered || !went {
            return None;
        }
        a.state = AskState::Read;
        a.read_at = Some(now);
        a.closed_at = Some(now);
        record(a, now, "read", String::new());
        let a = a.clone();
        drop(b);
        log::info!("ask {id} read");
        self.inbox_told(&a.project_id);
        Some(a)
    }

    /// An answer to `id`, from the user (`by` None) or the agent the user left it to: kept, then sent
    /// to its author 10 seconds later (at once for an agent's), the time to take it back.
    pub fn ask_answer(
        self: &Arc<Self>,
        id: &str,
        ans: AnswerIn,
        by: Option<(String, String)>,
        files: Vec<Incoming>,
    ) -> Result<Ask> {
        let lang = i18n::ui();
        let now = now_ms();
        let before = self.ask(id)?;
        if !before.state.waiting() {
            bail!(tr!(
                "La demande {id} n’attend plus de réponse : elle est {state}.",
                "Request {id} no longer waits for an answer: it is {state}.",
                state = before.state.as_str()
            ));
        }
        let ans = checked_answer(lang, &before, ans).map_err(|e| anyhow!(e))?;
        if ans.note.as_deref().is_some_and(looks_secret)
            || ans.text.as_deref().is_some_and(looks_secret)
        {
            bail!(secret(lang));
        }
        if files.len() > FILES_MAX {
            bail!(too_many_files(lang, files.len()));
        }
        let weight: u64 = before.files.iter().map(|f| f.size).sum();
        let box_weight: u64 = self
            .asks
            .lock()
            .list
            .iter()
            .filter(|a| a.id != id)
            .map(Ask::weight)
            .sum();
        let theirs = copy_files(lang, &self.ask_dir(id), files, weight, box_weight, true)
            .map_err(|e| anyhow!(e))?;
        let delegated = by.is_some();
        let answer = AskAnswer {
            options: ans.options,
            yes: ans.yes,
            text: ans.text,
            done: ans.done,
            steps: ans.steps,
            note: ans.note,
            by: by.as_ref().map(|(id, _)| id.clone()),
            by_name: by.as_ref().map(|(_, n)| n.clone()),
            via: if delegated {
                AskVia::Delegated
            } else {
                ans.via
            },
            at: now,
            send_at: Some(if delegated { now } else { now + UNDO_MS }),
            sent_at: None,
        };
        let kept = {
            let mut b = self.asks.lock();
            let a = b
                .list
                .iter_mut()
                .find(|a| a.id == id)
                .ok_or_else(|| not_found(lang, id))?;
            if !a.state.waiting() {
                drop(b);
                for f in &theirs {
                    remove_file(&self.ask_dir(id), &f.file);
                }
                bail!(tr!(
                    "La demande {id} n’attend plus de réponse.",
                    "Request {id} no longer waits for an answer."
                ));
            }
            a.files.retain(|f| !f.by_user);
            a.files.extend(theirs);
            a.state = AskState::Answered;
            a.snoozed_until = None;
            a.answer = Some(answer);
            a.seen = true;
            let who = by
                .as_ref()
                .map(|(_, n)| format!("{n}, delegated"))
                .unwrap_or_else(|| "user".into());
            let via = a
                .answer
                .as_ref()
                .map(|x| format!("{:?}", x.via).to_lowercase())
                .unwrap_or_default();
            record(a, now, "answered", format!("{who} · {via}"));
            a.clone()
        };
        log::info!("ask {id} answered");
        self.inbox_told(&kept.project_id);
        let delay = kept.answer.as_ref().and_then(|x| x.send_at).unwrap_or(now) - now;
        let core = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delay.max(0) as u64)).await;
            core.deliver_answers().await;
        });
        Ok(kept)
    }

    /// « Revenir sur ma réponse »: an answer its author has not read yet is taken back, the request
    /// open again.
    pub fn ask_reopen(&self, id: &str) -> Result<Ask> {
        let lang = i18n::ui();
        let now = now_ms();
        let mut b = self.asks.lock();
        let a = b
            .list
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| not_found(lang, id))?;
        if a.state != AskState::Answered {
            bail!(tr!(
                "La réponse à {id} a été lue : écris plutôt à l’agent (« Changer d’avis… »).",
                "The answer to {id} was read: write to the agent instead (“Change my mind…”)."
            ));
        }
        a.state = AskState::Open;
        a.answer = None;
        a.files.retain(|f| !f.by_user);
        record(a, now, "reopened", String::new());
        let a = a.clone();
        drop(b);
        log::info!("ask {id} reopened");
        self.inbox_told(&a.project_id);
        Ok(a)
    }

    /// « Plus tard »: out of the box until `until` (ms); None brings it back now.
    pub fn ask_snooze(&self, id: &str, until: Option<i64>) -> Result<Ask> {
        let lang = i18n::ui();
        let now = now_ms();
        let mut b = self.asks.lock();
        let a = b
            .list
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| not_found(lang, id))?;
        if !a.state.waiting() {
            bail!(tr!(
                "La demande {id} n’attend plus : rien à reporter.",
                "Request {id} no longer waits: nothing to put off."
            ));
        }
        match until {
            Some(t) if t > now => {
                a.state = AskState::Snoozed;
                a.snoozed_until = Some(t);
                record(a, now, "snoozed", t.to_string());
            }
            _ => {
                a.state = AskState::Open;
                a.snoozed_until = None;
                record(a, now, "woke", String::new());
            }
        }
        let a = a.clone();
        drop(b);
        self.inbox_told(&a.project_id);
        Ok(a)
    }

    /// The user saw it in the box.
    pub fn ask_seen(&self, id: &str) -> Result<()> {
        let lang = i18n::ui();
        let project = {
            let mut b = self.asks.lock();
            let a = b
                .list
                .iter_mut()
                .find(|a| a.id == id)
                .ok_or_else(|| not_found(lang, id))?;
            if a.seen && !a.woke {
                return Ok(());
            }
            a.seen = true;
            a.woke = false;
            a.project_id.clone()
        };
        self.inbox_told(&project);
        Ok(())
    }

    /// The agents the user may leave `a` to (§ 8.5): none for a go-ahead or a task; the project's
    /// squad lead, or any agent of a project that sees it, as « Les agents peuvent répondre aux
    /// demandes des autres agents » says; never its author.
    pub(crate) fn may_delegate_to(&self, a: &Ask, agent: &str) -> Result<()> {
        if a.gate || a.kind == AskKind::Task {
            bail!(tr!(
                "Un feu vert et une tâche sont à toi seul : ils ne se laissent pas à un agent.",
                "A go-ahead and a task are yours alone: they are not left to an agent."
            ));
        }
        if a.author.agent.as_deref() == Some(agent) {
            bail!(tr!(
                "Un agent ne répond jamais à sa propre demande.",
                "An agent never answers its own request."
            ));
        }
        let to = self.agent(agent)?;
        let to_project = to.lock().meta.project_id.clone();
        let projects = self.projects.read().clone();
        let private = |p: &str| projects.iter().any(|x| x.id == p && x.private);
        if to_project != a.project_id && (private(&a.project_id) || private(&to_project)) {
            bail!(tr!(
                "Une demande d’un projet privé ne se laisse qu’à un agent de ce projet.",
                "A request of a private project is only left to an agent of that project."
            ));
        }
        let chef = projects
            .iter()
            .find(|p| p.id == a.project_id)
            .and_then(|p| p.chef.clone());
        // The same rule as for the requests an agent stopped for (ESC-513).
        let setting = self.settings.read().agents_answer;
        match crate::delegation::may_answer(setting, chef.as_deref(), agent) {
            Ok(()) => Ok(()),
            Err(crate::delegation::Refusal::NotTheChef) => bail!(tr!(
                "Seul le chef d’escouade du projet reçoit une demande : il se choisit dans Réglages › Projets › Équipe.",
                "Only the project’s squad lead is left a request: it is chosen in Settings › Projects › Team."
            )),
            Err(crate::delegation::Refusal::Nobody) => bail!(tr!(
                "Aucun agent ne répond aux demandes des autres : permets-le dans Réglages › Agents › Piloter Escouade (« Les agents peuvent répondre aux demandes des autres agents »).",
                "No agent answers the others’ requests: allow it in Settings › Agents › Drive Escouade (“Agents can answer the other agents’ requests”)."
            )),
        }
    }

    /// « Laisser <chef> trancher »: the user leaves `id` to `agent` (None takes it back).
    pub fn ask_delegate(&self, id: &str, agent: Option<&str>) -> Result<Ask> {
        let lang = i18n::ui();
        let a = self.ask(id)?;
        if !a.state.waiting() {
            bail!(tr!(
                "La demande {id} n’attend plus.",
                "Request {id} no longer waits."
            ));
        }
        if let Some(agent) = agent {
            self.may_delegate_to(&a, agent)?;
        }
        let now = now_ms();
        let mut b = self.asks.lock();
        let a = b
            .list
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| not_found(lang, id))?;
        a.delegated_to = agent.map(str::to_string);
        let to = agent
            .and_then(|x| self.agent(x).ok())
            .map(|h| h.lock().meta.name.clone())
            .unwrap_or_default();
        record(a, now, "delegated", to);
        let a = a.clone();
        drop(b);
        self.inbox_told(&a.project_id);
        Ok(a)
    }

    /// The answers whose 10 seconds are over go to their authors: a message to an agent of Escouade
    /// that wakes it at rest (or reaches it during its turn), unless the request said `wake: false`;
    /// kept for `get_ask` for Claude outside Escouade. A message delivered is read.
    pub async fn deliver_answers(self: &Arc<Self>) {
        let now = now_ms();
        let due: Vec<Ask> = {
            let mut b = self.asks.lock();
            let mut due = Vec::new();
            for a in b.list.iter_mut() {
                let Some(ans) = a.answer.as_mut() else {
                    continue;
                };
                if a.state != AskState::Answered || ans.send_at.is_none_or(|t| t > now) {
                    continue;
                }
                ans.send_at = None;
                ans.sent_at = Some(now);
                record(a, now, "sent", String::new());
                due.push(a.clone());
            }
            due
        };
        for a in due {
            self.inbox_told(&a.project_id);
            let Some(agent) = a.author.agent.clone() else {
                continue;
            };
            if !a.wake || self.agent(&agent).is_err() {
                continue;
            }
            let Some(ans) = &a.answer else { continue };
            let text = answer_text(self.lang().claude, &a, ans);
            match self.send_ask_answer(&agent, text).await {
                Ok(()) => {
                    self.ask_mark_read(&a.id);
                }
                Err(e) => log::warn!("ask {}: its answer not sent to its agent: {e:#}", a.id),
            }
        }
    }

    /// What changes by itself (§ 3.4): a request put off comes back at its time; one past its date
    /// expires; one whose agent was deleted or archived, whose ticket was approved, deleted or sent
    /// back to « À faire », or whose project went, is taken out; a closed one's files go after 30
    /// days, and the request itself after 90 (300 kept at most).
    pub fn asks_sweep(self: &Arc<Self>) {
        let now = now_ms();
        let files_kept = kept_days(self.settings.read().inbox_keep_days);
        let projects: HashSet<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let tickets: Vec<(String, Column)> = self
            .tickets
            .read()
            .iter()
            .map(|t| (t.id.clone(), t.column))
            .collect();
        let agents: Vec<(String, bool)> = self
            .agents
            .read()
            .values()
            .map(|h| {
                let rt = h.lock();
                (rt.meta.id.clone(), rt.meta.archived)
            })
            .collect();
        let mut closed = Vec::new();
        let mut erase = Vec::new();
        let mut changed = HashSet::new();
        {
            let mut b = self.asks.lock();
            b.list.retain(|a| {
                if !projects.contains(&a.project_id) {
                    erase.push(a.id.clone());
                    changed.insert(a.project_id.clone());
                    return false;
                }
                true
            });
            for a in b.list.iter_mut() {
                if a.state == AskState::Snoozed && a.snoozed_until.is_some_and(|t| t <= now) {
                    a.state = AskState::Open;
                    a.snoozed_until = None;
                    a.woke = true;
                    record(a, now, "woke", String::new());
                    changed.insert(a.project_id.clone());
                }
                if !a.state.waiting() {
                    continue;
                }
                let gone_agent = a.author.agent.as_ref().and_then(|id| {
                    match agents.iter().find(|(x, _)| x == id) {
                        None => Some(tr!("Son agent a été supprimé.", "Its agent was deleted.")),
                        Some((_, true)) => {
                            Some(tr!("Son agent a été archivé.", "Its agent was archived."))
                        }
                        _ => None,
                    }
                });
                let gone_ticket = a.ticket_id.as_ref().and_then(|id| {
                    match tickets.iter().find(|(x, _)| x == id) {
                        None => Some(tr!("Son ticket a été supprimé.", "Its ticket was deleted.")),
                        Some((_, Column::Done)) => {
                            Some(tr!("Son ticket est terminé.", "Its ticket is done."))
                        }
                        Some((_, Column::Todo)) if a.ticket_started => Some(tr!(
                            "Son ticket a été renvoyé « À faire ».",
                            "Its ticket was sent back to “To do”."
                        )),
                        _ => None,
                    }
                });
                if let Some(why) = gone_agent.or(gone_ticket) {
                    a.state = AskState::Retired;
                    a.reason = Some(why);
                } else if a.expires_at <= now {
                    a.state = AskState::Expired;
                } else {
                    continue;
                }
                a.closed_at = Some(now);
                a.seen = false;
                let what = a.state.as_str();
                record(a, now, what, String::new());
                log::info!("ask {} {what}", a.id);
                changed.insert(a.project_id.clone());
                closed.push(a.clone());
            }
            for a in b.list.iter_mut() {
                let closed_at = match (a.state.closed(), a.closed_at) {
                    (true, Some(t)) => t,
                    _ => continue,
                };
                if a.files_erased_at.is_none()
                    && !a.files.is_empty()
                    && closed_at + files_kept * DAY_MS <= now
                {
                    a.files_erased_at = Some(now);
                    record(a, now, "filesErased", String::new());
                    erase.push(a.id.clone());
                    changed.insert(a.project_id.clone());
                }
            }
            // The closed ones, past their days or beyond the most kept, go with their files.
            let mut old: Vec<(i64, String)> = b
                .list
                .iter()
                .filter(|a| a.state.closed())
                .map(|a| (a.closed_at.unwrap_or(a.created_at), a.id.clone()))
                .collect();
            old.sort();
            let over = old.len().saturating_sub(CLOSED_MAX);
            let dropped: HashSet<String> = old
                .iter()
                .enumerate()
                .filter(|(i, (t, _))| *i < over || t + KEPT_DAYS * DAY_MS <= now)
                .map(|(_, (_, id))| id.clone())
                .collect();
            if !dropped.is_empty() {
                b.list.retain(|a| {
                    if dropped.contains(&a.id) {
                        changed.insert(a.project_id.clone());
                        return false;
                    }
                    true
                });
                erase.extend(dropped);
            }
        }
        for id in erase {
            let _ = std::fs::remove_dir_all(self.ask_dir(&id));
        }
        for a in &closed {
            if a.state == AskState::Expired {
                self.telemetry.track(crate::telemetry::Event::AskExpired {
                    kind: a.kind.as_str(),
                });
            }
            self.block_waiting(a);
        }
        for p in changed {
            self.inbox_told(&p);
        }
    }

    /// « Vider les fichiers des demandes closes »: what the box's closed requests' files weighed.
    pub fn asks_erase_closed_files(&self) -> u64 {
        let now = now_ms();
        let mut freed = 0;
        let mut ids = Vec::new();
        {
            let mut b = self.asks.lock();
            for a in b
                .list
                .iter_mut()
                .filter(|a| a.state.closed() && a.files_erased_at.is_none() && !a.files.is_empty())
            {
                freed += a.weight();
                a.files_erased_at = Some(now);
                record(a, now, "filesErased", String::new());
                ids.push(a.id.clone());
            }
        }
        for id in &ids {
            let _ = std::fs::remove_dir_all(self.ask_dir(id));
        }
        if !ids.is_empty() {
            self.request_save();
        }
        freed
    }

    /// What the box's files weigh (shown with the space the app takes, IN-2).
    pub fn asks_weight(&self) -> u64 {
        self.asks.lock().list.iter().map(Ask::weight).sum()
    }

    /// The tickets that wait for a request not answered yet (§ 3.6).
    pub(crate) fn tickets_waiting_asks(&self) -> HashSet<String> {
        waiting_tickets(&self.asks.lock().list)
    }

    /// The copy of file `file` (or of rank `index`) of request `id`, when it is still there: never a
    /// path out of the request's folder.
    pub(crate) fn ask_file(
        &self,
        a: &Ask,
        file: Option<&str>,
        index: Option<usize>,
    ) -> Result<(AskFile, PathBuf)> {
        let found = match (file, index) {
            (Some(f), None) => a.files.iter().find(|x| x.file == f),
            (None, Some(i)) => a.files.get(i),
            _ => bail!(tr!(
                "Nomme le fichier par son file ou par son index, l’un des deux.",
                "Name the file by its file or by its index, one of the two."
            )),
        };
        let Some(f) = found else {
            bail!(tr!(
                "La demande {id} n’a pas ce fichier : get_ask les liste (files).",
                "Request {id} has no such file: get_ask lists them (files).",
                id = a.id
            ));
        };
        if a.files_erased_at.is_some() {
            let days = kept_days(self.settings.read().inbox_keep_days);
            bail!(tr!(
                "Les fichiers de la demande {id} ont été effacés, {days} jours après sa fin.",
                "The files of request {id} were erased, {days} days after it ended.",
                id = a.id,
                days = days
            ));
        }
        let dir = self.ask_dir(&a.id);
        let path = file_path(&dir, &f.file)
            .and_then(|p| crate::attachfiles::copy_in(&dir.join("files"), &p))
            .ok_or_else(|| {
                anyhow!(tr!(
                    "Le fichier « {n} » est introuvable.",
                    "The file “{n}” wasn’t found.",
                    n = f.name
                ))
            })?;
        Ok((f.clone(), path))
    }
}

fn remove_file(dir: &Path, file: &str) {
    if let Some(p) = file_path(dir, file) {
        if let Some(parent) = p.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }
}

/// An id cut short for the log.
fn short(id: &str) -> &str {
    &id[..id.len().min(8)]
}

// ---------- the window's commands (IN-2) ----------

/// A request as the window shows it: what it names, by name.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowAsk {
    #[serde(flatten)]
    pub ask: Ask,
    pub project_name: String,
    /// Its project is private: the window marks it so.
    pub private: bool,
    pub ticket_key: Option<String>,
    pub delegated_name: Option<String>,
    /// The agents the user may leave it to (§ 8.5), by id and name.
    pub delegates: Vec<(String, String)>,
}

/// The box as the window reads it.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxView {
    /// The MCP server is wanted: off, no box (and no request in `asks`).
    pub enabled: bool,
    pub asks: Vec<WindowAsk>,
    /// The requests that wait for the user, every project.
    pub open: usize,
}

/// A part of a file of a request, for the window (and the phone): 4 MB at most at a time.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxChunk {
    pub name: String,
    pub kind: AskFileKind,
    pub media_type: String,
    pub size: u64,
    pub offset: u64,
    /// The bytes, in base64.
    pub data: String,
}

/// What the question before turning the server off lists (§ 6.2).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxClosing {
    pub open: usize,
    /// The first three titles.
    pub titles: Vec<String>,
}

/// The most a part of a file weighs.
pub(crate) const CHUNK_MAX: u64 = 4 * MB;

impl<R: Runtime> Core<R> {
    fn window_ask(&self, a: Ask) -> WindowAsk {
        let project = self.project(&a.project_id).ok();
        let ticket_key = a
            .ticket_id
            .as_deref()
            .and_then(|t| self.ticket(t).ok())
            .map(|t| t.key);
        let delegated_name = a.delegated_to.as_deref().map(|id| {
            self.agent(id)
                .map(|h| h.lock().meta.name.clone())
                .unwrap_or_else(|_| id.to_string())
        });
        let delegates = if a.state.waiting() {
            let candidates: Vec<(String, String)> = self
                .agents
                .read()
                .values()
                .filter_map(|h| {
                    let rt = h.lock();
                    (!rt.meta.archived).then(|| (rt.meta.id.clone(), rt.meta.name.clone()))
                })
                .collect();
            let mut ok: Vec<(String, String)> = candidates
                .into_iter()
                .filter(|(id, _)| self.may_delegate_to(&a, id).is_ok())
                .collect();
            ok.sort_by(|x, y| x.1.cmp(&y.1));
            ok
        } else {
            Vec::new()
        };
        WindowAsk {
            project_name: project.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
            private: project.is_some_and(|p| p.private),
            ticket_key,
            delegated_name,
            delegates,
            ask: a,
        }
    }

    /// `inbox_list`: the box, of a project or of all; nothing while the server is off.
    pub fn inbox_list(self: &Arc<Self>, project_id: Option<&str>) -> InboxView {
        if !self.inbox_enabled() {
            return InboxView {
                enabled: false,
                asks: Vec::new(),
                open: 0,
            };
        }
        self.asks_sweep();
        let all = self.asks_all();
        let open = all.iter().filter(|a| a.state.waiting()).count();
        let asks = all
            .into_iter()
            .filter(|a| project_id.is_none_or(|p| a.project_id == p))
            .map(|a| self.window_ask(a))
            .collect();
        InboxView {
            enabled: true,
            asks,
            open,
        }
    }

    /// `inbox_get`.
    pub fn inbox_get(&self, id: &str) -> Result<WindowAsk> {
        if !self.inbox_enabled() {
            return Err(self.inbox_closed());
        }
        Ok(self.window_ask(self.ask(id)?))
    }

    /// `inbox_file`: `length` bytes (4 MB at most) of a request's file from `offset`, read from its
    /// copy, never from a path of the window's or the agent's.
    pub fn inbox_file(
        &self,
        id: &str,
        file: &str,
        offset: u64,
        length: Option<u64>,
    ) -> Result<InboxChunk> {
        use base64::Engine;
        use std::io::{Seek, SeekFrom};
        if !self.inbox_enabled() {
            return Err(self.inbox_closed());
        }
        let a = self.ask(id)?;
        let (f, path) = self.ask_file(&a, Some(file), None)?;
        let mut h = std::fs::File::open(&path)?;
        let size = h.metadata()?.len();
        let offset = offset.min(size);
        h.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        h.take(length.unwrap_or(CHUNK_MAX).clamp(1, CHUNK_MAX))
            .read_to_end(&mut bytes)?;
        Ok(InboxChunk {
            name: f.name,
            kind: f.kind,
            media_type: f.media_type,
            size,
            offset,
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
        })
    }

    /// « Ouvrir » (the system's application; never a program, a script nor a page) or « Afficher
    /// dans le dossier » (`reveal`) of a request's file.
    pub fn inbox_open_file(&self, id: &str, file: &str, reveal: bool) -> Result<()> {
        if !self.inbox_enabled() {
            return Err(self.inbox_closed());
        }
        let a = self.ask(id)?;
        let (f, path) = self.ask_file(&a, Some(file), None)?;
        if !reveal && !opens_safely(&f, &path) {
            bail!(tr!(
                "« {n} » peut lancer un programme ou un script : Escouade ne l’ouvre pas, elle l’affiche dans son dossier.",
                "“{n}” can run a program or a script: Escouade doesn’t open it, it shows it in its folder.",
                n = f.name
            ));
        }
        crate::sentfiles::open_or_reveal(path, reveal)
    }

    /// `inbox_answer`: the user's answer, with the files joined to it (copies of the window's drafts,
    /// `attach_open`: never another file).
    pub fn inbox_answer(
        self: &Arc<Self>,
        id: &str,
        ans: AnswerIn,
        files: Vec<crate::attachfiles::AttachedFile>,
    ) -> Result<WindowAsk> {
        if !self.inbox_enabled() {
            return Err(self.inbox_closed());
        }
        if files.len() > FILES_MAX {
            bail!(too_many_files(i18n::ui(), files.len()));
        }
        let drafts = self.data.drafts_files();
        let mut incoming = Vec::new();
        for f in files {
            let Some(real) = crate::attachfiles::copy_in(&drafts, Path::new(&f.path)) else {
                bail!(tr!(
                    "« {n} » n’est pas un fichier joint dans la fenêtre.",
                    "“{n}” isn’t a file joined in the window.",
                    n = f.name
                ));
            };
            let h = std::fs::File::open(&real)?;
            let length = h.metadata()?.len();
            incoming.push(Incoming {
                name: f.name,
                declared: None,
                option: None,
                source: Source::File(h, length),
            });
        }
        let a = self.ask_answer(id, ans, None, incoming)?;
        Ok(self.window_ask(a))
    }

    /// The copy of a request's file, for « Enregistrer sous… » (ESC-510): the box open, never a path out of the request's
    /// folder.
    pub fn inbox_file_copy(&self, id: &str, file: &str) -> Result<(AskFile, PathBuf)> {
        if !self.inbox_enabled() {
            return Err(self.inbox_closed());
        }
        let a = self.ask(id)?;
        self.ask_file(&a, Some(file), None)
    }

    /// What the question before turning the server off lists.
    pub fn inbox_closing(&self) -> InboxClosing {
        let all = self.asks_all();
        let open: Vec<&Ask> = all.iter().filter(|a| a.state.waiting()).collect();
        InboxClosing {
            open: open.len(),
            titles: open.iter().take(3).map(|a| a.title.clone()).collect(),
        }
    }
}

/// The statistics' event of an answer the user gave (ESC-510): its sort, how long it waited, how, whether the
/// recommendation was followed. None for an answer that is not the user's.
pub(crate) fn answered_event(a: &Ask) -> Option<crate::telemetry::Event> {
    let ans = a.answer.as_ref()?;
    if ans.by.is_some() {
        return None;
    }
    let waited = (ans.at - a.created_at).max(0);
    let delay = if waited < 10 * 60_000 {
        "ten_minutes"
    } else if waited < 3_600_000 {
        "hour"
    } else if waited < DAY_MS {
        "day"
    } else {
        "longer"
    };
    let via = match ans.via {
        AskVia::Click => "click",
        AskVia::Keyboard => "keyboard",
        AskVia::Voice => "voice",
        AskVia::Phone => "phone",
        AskVia::Delegated => "delegated",
    };
    let followed = match (
        &a.kind,
        a.recommend.as_ref().and_then(|r| r.option.as_ref()),
    ) {
        (AskKind::Choice, Some(rec)) => Some(ans.options.len() == 1 && ans.options[0] == *rec),
        _ => None,
    };
    Some(crate::telemetry::Event::AskAnswered {
        kind: a.kind.as_str(),
        delay,
        via,
        followed,
    })
}

/// Whether « Ouvrir » may hand a file to the system: never a program or a script (by its name, or
/// marked executable), nor what a browser would run (an SVG, a page).
pub(crate) fn opens_safely(f: &AskFile, path: &Path) -> bool {
    let lower = f.name.to_ascii_lowercase();
    let page = f.media_type == "image/svg+xml"
        || [".html", ".htm", ".xhtml", ".svg", ".mht", ".mhtml"]
            .iter()
            .any(|e| lower.ends_with(e));
    !(page || crate::sentfiles::runnable_name(&f.name) || crate::sentfiles::runnable(path))
}
