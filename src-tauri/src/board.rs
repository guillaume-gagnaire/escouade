//! The board's rules, without I/O: ticket keys, the report an agent ends each turn with
//! (```escouade block), what a turn's end does to its ticket, which tickets start, the messages
//! sent to the agents.

use crate::claude::truncate;
use crate::core::{slugify, RESUME_MARGIN_MS};
use crate::i18n::{self, Lang};
use crate::model::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// A ticket's criteria when none was given, in `lang`: they stay in the language they were made in.
pub fn default_criteria(lang: Lang) -> [String; 2] {
    [
        tr_in!(
            lang,
            "Implémentation conforme au ticket",
            "Implementation matches the ticket"
        ),
        tr_in!(lang, "Tests verts", "Tests pass"),
    ]
}
// What the agents are told (`lang` is the language of the texts for Claude): the markers they
// write back (the ```escouade fence, the keys of its JSON) are the same in every language, as
// `parse_report` and the window read them.

/// Sent once when a turn ended without its report.
pub fn reminder(lang: Lang) -> String {
    tr_in!(
        lang,
        "Termine par le bilan des critères (bloc escouade).",
        "End with the criteria report (escouade block)."
    )
}
const FENCE: &str = "```escouade";

/// How an agent's turn ended, as the core tells the board.
#[derive(Debug, Clone, PartialEq)]
pub enum TurnEnd {
    /// Stopped by the user.
    Interrupted,
    /// Stopped by the usage limit: the automatic resume takes it up.
    Limited,
    Error(String),
    /// The assistant's text of the turn.
    Finished(String),
}

/// The first three letters of the project's name, upper case (accents folded); TIC without any.
pub fn key_prefix(project_name: &str) -> String {
    let letters: String = slugify(project_name)
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .take(3)
        .collect();
    if letters.is_empty() {
        "TIC".into()
    } else {
        letters.to_uppercase()
    }
}

/// The name of a ticket's agent: `atl-42-limiter-les-tentatives`.
pub fn agent_name(key: &str, title: &str) -> String {
    slugify(&format!("{key} {title}"))
}

/// The branch of a ticket: `ticket/atl-42`.
pub fn branch_of(key: &str) -> String {
    format!("ticket/{}", key.to_lowercase())
}

/// One criterion per non-empty line; the default two without any.
pub fn criteria_from(lines: &[String]) -> Vec<Criterion> {
    let mut texts: Vec<String> = lines
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if texts.is_empty() {
        texts = default_criteria(i18n::ui()).into();
    }
    texts
        .into_iter()
        .map(|text| Criterion {
            text,
            ..Default::default()
        })
        .collect()
}

/// 3, 5 or 8 loops; 5 for anything else.
pub fn max_loops(n: u32) -> u32 {
    if [3, 5, 8].contains(&n) {
        n
    } else {
        5
    }
}

/// What an agent's last ```escouade block said.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// (n from 1, reached, note); None when the block had no list of criteria.
    pub criteria: Option<Vec<(usize, bool, String)>>,
    pub recipe: Option<TestRecipe>,
    /// The features in place, from `avancement` (or `progress`); never empty. None when the
    /// block had no such list (an empty one, or one with nothing usable, is none), which leaves
    /// the ticket's previous one as it is.
    pub progress: Option<Vec<String>>,
}

/// The most items of a progress list that are kept, and the most bytes of each.
const PROGRESS_ITEMS: usize = 8;
const PROGRESS_ITEM_BYTES: usize = 120;

/// The content of the last ```escouade block of `text`, if it is closed.
fn last_block(text: &str) -> Option<&str> {
    let start = text.rfind(FENCE)?;
    let after = &text[start + FENCE.len()..];
    let content = &after[after.find('\n')? + 1..];
    Some(content[..content.find("```")?].trim())
}

/// A folder of the worktree, written relative: no drive, no root, no `..`.
fn safe_dir(d: &str) -> bool {
    let d = d.trim();
    !(d.starts_with(['/', '\\'])
        || d.get(1..2) == Some(":")
        || d.split(['/', '\\']).any(|s| s == ".."))
}

fn recipe_is_safe(r: &TestRecipe) -> bool {
    r.prepare
        .iter()
        .all(|s| safe_dir(&s.dir) && !s.command.trim().is_empty())
        && r.processes
            .iter()
            .all(|p| safe_dir(&p.dir) && !p.command.trim().is_empty())
        // Something to run, or at least the address to open (isola runs the services).
        && (!r.prepare.is_empty() || !r.processes.is_empty() || !r.open.trim().is_empty())
}

/// The progress list of a report: the first 8 usable items of the `avancement` (or `progress`)
/// array, each on one line and cut at 120 bytes. Items that are not text, or empty once cleaned,
/// are skipped; a value that is not an array, an empty array, or one with no usable item is no
/// list either (the ticket keeps the one it has).
fn progress_from(v: &Value) -> Option<Vec<String>> {
    let list = v
        .get("avancement")
        .or_else(|| v.get("progress"))?
        .as_array()?;
    let items: Vec<String> = list
        .iter()
        .filter_map(Value::as_str)
        .map(|item| one_line(item, PROGRESS_ITEM_BYTES))
        .filter(|item| !item.is_empty())
        .take(PROGRESS_ITEMS)
        .collect();
    (!items.is_empty()).then_some(items)
}

/// A criterion's number (from 1): a JSON number, or a string of digits.
fn criterion_number(n: &Value) -> Option<usize> {
    let n = match n {
        Value::String(s) => s.trim().parse::<u64>().ok()?,
        n => n.as_u64()?,
    };
    usize::try_from(n).ok().filter(|n| *n >= 1)
}

/// The report of the last ```escouade block of a turn's text: None when it is missing or is not
/// a JSON object. `criteres` (or `criteria`) items without a valid `n` are skipped, and a list
/// none of whose items has one counts as no list of criteria; a recipe whose folder could leave
/// the worktree is dropped.
pub fn parse_report(text: &str) -> Option<Report> {
    let v: Value = serde_json::from_str(last_block(text)?).ok()?;
    if !v.is_object() {
        return None;
    }
    let criteria = v
        .get("criteres")
        .or_else(|| v.get("criteria"))
        .and_then(Value::as_array)
        .and_then(|list| {
            let reported: Vec<_> = list
                .iter()
                .filter_map(|c| {
                    let n = criterion_number(&c["n"])?;
                    let ok = c["ok"].as_bool().unwrap_or(false);
                    let note = c["note"].as_str().unwrap_or_default().trim().to_string();
                    Some((n, ok, note))
                })
                .collect();
            // Nothing usable in a list that was not empty: it must not read as "no criterion
            // reached", which would spend a loop.
            (!reported.is_empty() || list.is_empty()).then_some(reported)
        });
    let recipe = v
        .get("lancement")
        .and_then(|l| serde_json::from_value::<TestRecipe>(l.clone()).ok())
        .filter(recipe_is_safe);
    Some(Report {
        criteria,
        recipe,
        progress: progress_from(&v),
    })
}

/// What the end of a turn asks of the orchestrator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Next {
    /// The message to send the agent.
    pub send: Option<String>,
    /// The ticket reached "À tester" (to notify).
    pub ready: bool,
    /// The ticket got blocked (to notify).
    pub blocked: bool,
}

/// The first non-empty line of an error, cut to 200 characters: what a card and a notification show.
pub(crate) fn first_line(e: &str) -> String {
    let line = e
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| tr!("erreur inconnue", "unknown error"));
    truncate(&line, 200)
}

/// Why a ticket is blocked, as its card shows it (kept in the language it was written in).
pub(crate) fn interrupted(lang: Lang) -> String {
    tr_in!(lang, "Interrompu", "Interrupted")
}

/// Blocked by an error: `why` is its first line.
pub(crate) fn error_blocked(lang: Lang, why: &str) -> String {
    tr_in!(lang, "Erreur : {why}", "Error: {why}")
}

pub(crate) fn report_missing(lang: Lang) -> String {
    tr_in!(
        lang,
        "Bilan des critères manquant",
        "Criteria report missing"
    )
}

/// What the end of its agent's turn does to a ticket "En cours" (nothing to any other); what its
/// agent is sent next is written in `lang`.
pub fn turn_end(
    lang: Lang,
    t: &mut Ticket,
    end: &TurnEnd,
    report: Option<&Report>,
    now: i64,
) -> Next {
    let mut next = Next::default();
    if t.column != Column::Doing {
        return next;
    }
    t.blocked = None;
    t.conflict = false;
    // The progress is the agent's own account, whatever became of its criteria: the last list
    // given replaces the previous, and none leaves it.
    if let (TurnEnd::Finished(_), Some(progress)) = (end, report.and_then(|r| r.progress.as_ref()))
    {
        t.progress = progress.clone();
    }
    match end {
        // A stop or an error ends the exchange: once the user resumes, a missing report is
        // reminded again instead of blocking at once.
        TurnEnd::Interrupted => {
            t.reminded = false;
            t.blocked = Some(interrupted(i18n::ui()));
            next.blocked = true;
        }
        TurnEnd::Limited => {}
        TurnEnd::Error(e) => {
            t.reminded = false;
            t.blocked = Some(error_blocked(i18n::ui(), &first_line(e)));
            next.blocked = true;
        }
        TurnEnd::Finished(_) => match report.and_then(|r| r.criteria.as_ref()) {
            None if !t.reminded => {
                t.reminded = true;
                next.send = Some(reminder(lang));
            }
            None => {
                t.reminded = false;
                t.blocked = Some(report_missing(i18n::ui()));
                next.blocked = true;
            }
            Some(reported) => {
                t.reminded = false;
                for (i, c) in t.criteria.iter_mut().enumerate() {
                    match reported.iter().rev().find(|(n, _, _)| *n == i + 1) {
                        Some((_, ok, note)) => {
                            c.ok = *ok;
                            c.note = note.clone();
                        }
                        None => {
                            c.ok = false;
                            c.note.clear();
                        }
                    }
                }
                let all = t.criteria.iter().all(|c| c.ok);
                if all || t.iteration >= t.max_loops {
                    t.column = Column::Review;
                    t.partial = !all;
                    t.review_at = Some(now);
                    next.ready = true;
                } else {
                    t.iteration += 1;
                    t.loops += 1;
                    next.send = Some(loop_message(lang, t));
                }
            }
        },
    }
    next
}

/// The tickets of `project_id` to start now, in order: "À faire" by rank (with the autopilot, or
/// launched by hand), while tickets "En cours" and not blocked leave places; none while `paused`
/// (an agent waits for its quota, or `autopilot_pause` holds them back). The autopilot leaves a
/// ticket that comes after one not "Terminé" yet for the next; one launched by hand starts all the
/// same (the window asked first).
pub fn to_start(
    tickets: &[Ticket],
    project_id: &str,
    s: &BoardSettings,
    paused: bool,
) -> Vec<String> {
    if paused {
        return Vec::new();
    }
    let mine = || tickets.iter().filter(|t| t.project_id == project_id);
    let busy = mine()
        .filter(|t| t.column == Column::Doing && t.blocked.is_none())
        .count();
    let free = (s.max_parallel.clamp(1, 6) as usize).saturating_sub(busy);
    // A ticket gone, or of another project, holds nothing back.
    let unfinished: HashSet<&str> = mine()
        .filter(|t| t.column != Column::Done)
        .map(|t| t.id.as_str())
        .collect();
    let waits = |t: &Ticket| t.after.iter().any(|a| unfinished.contains(a.as_str()));
    let mut todo: Vec<&Ticket> = mine()
        .filter(|t| t.column == Column::Todo && (t.forced || (s.autopilot && !waits(t))))
        .collect();
    todo.sort_by_key(|t| (t.rank, t.created_at));
    todo.into_iter().take(free).map(|t| t.id.clone()).collect()
}

/// What a ticket's form asks it to come after, as kept: tickets of `project_id` other than `id`
/// itself, each once, in the order given. One gone meanwhile (deleted), or another project's (a
/// window behind), is left out.
pub fn after_of(tickets: &[Ticket], project_id: &str, id: &str, wanted: &[String]) -> Vec<String> {
    let mut after: Vec<String> = Vec::new();
    for w in wanted {
        let known = tickets
            .iter()
            .any(|t| t.id == *w && t.project_id == project_id);
        if known && w != id && !after.contains(w) {
            after.push(w.clone());
        }
    }
    after
}

/// Why ticket `id` cannot come after `after`: one of them already waits for it, directly or
/// through others, and neither would ever start by itself. None when none does.
pub fn cycle_refusal(lang: Lang, tickets: &[Ticket], id: &str, after: &[String]) -> Option<String> {
    let by_id: HashMap<&str, &Ticket> = tickets.iter().map(|t| (t.id.as_str(), t)).collect();
    let me = by_id.get(id)?;
    after.iter().find_map(|first| {
        let first = by_id.get(first.as_str())?;
        // Every ticket `first` waits for, directly or not, once each: a loop already there (a
        // hand-edited file) cannot hang the search.
        let mut seen: HashSet<&str> = HashSet::new();
        let mut next: Vec<&str> = first.after.iter().map(String::as_str).collect();
        while let Some(x) = next.pop() {
            if x == id {
                return Some(tr_in!(
                    lang,
                    "{first} attend déjà {me} (directement ou non).",
                    "{first} already waits for {me} (directly or not).",
                    first = first.key,
                    me = me.key
                ));
            }
            if seen.insert(x) {
                if let Some(t) = by_id.get(x) {
                    next.extend(t.after.iter().map(String::as_str));
                }
            }
        }
        None
    })
}

/// How long no ticket starts once one met the usage limit with no resume planned (turned off, an
/// API key, no reset known): the next one would meet it too.
pub const LIMIT_PAUSE_MS: i64 = 30 * 60_000;

/// The choices of "Pause au-delà du quota", in percent of a window.
pub const QUOTA_PAUSES: [u32; 4] = [80, 90, 95, 100];

/// The threshold of "Pause au-delà du quota" as saved, 100 % when it is not one of the choices (a
/// hand-edited file: 0 would hold every ticket back for good).
pub fn quota_threshold(saved: u32) -> u32 {
    if QUOTA_PAUSES.contains(&saved) {
        saved
    } else {
        100
    }
}

impl Hold {
    /// "Reprendre maintenant" at `now`: the pause after a limit goes, and so do the windows over
    /// the threshold, until their end.
    pub fn lift(&mut self, usage: &UsageSnapshot, threshold: u32, now: i64) {
        self.limit_until = None;
        for (i, w) in windows(usage).into_iter().enumerate() {
            if let Some(end) = w.and_then(|w| over_until(w, threshold, now)) {
                self.lifted[i] = Some(end);
            }
        }
    }
}

/// The 5-hour and the weekly window, in the order of `Hold::lifted`.
fn windows(usage: &UsageSnapshot) -> [Option<&RateWindow>; 2] {
    [usage.five_hour.as_ref(), usage.seven_day.as_ref()]
}

/// When the window stops holding the tickets back, if it is used `threshold` percent or more:
/// its end, and the margin an agent's resume waits after it too (a clock a little ahead of the
/// server's would start them into the limit), if that is still to come.
fn over_until(w: &RateWindow, threshold: u32, now: i64) -> Option<i64> {
    // Claude Code gives a fraction, times 100 here: 0.95 may come as 94.99999….
    let over = w.pct + 1e-6 >= f64::from(threshold);
    w.resets_at
        .map(|end| end + RESUME_MARGIN_MS)
        .filter(|until| over && *until > now)
}

/// Why no ticket of any board starts at `now`, if none does: a window (5 h, weekly) used
/// `threshold` percent or more until its end, unless "Reprendre maintenant" lifted it, or the
/// pause after a usage limit with no resume planned. When several hold, the one that ends last:
/// the tickets wait for it. A window whose end is unknown holds nothing back: a reading that no
/// longer comes would hold them forever.
pub fn autopilot_pause(
    usage: &UsageSnapshot,
    threshold: u32,
    hold: &Hold,
    now: i64,
) -> Option<AutopilotPause> {
    let reasons = [PauseReason::FiveHour, PauseReason::Week];
    let mut pauses: Vec<AutopilotPause> = windows(usage)
        .into_iter()
        .zip(reasons)
        .zip(hold.lifted)
        .filter_map(|((w, reason), lifted)| {
            let w = w?;
            let until = over_until(w, threshold, now)?;
            // Lifted by "Reprendre maintenant" until the end it had then.
            if lifted.is_some_and(|end| now < end) {
                return None;
            }
            Some(AutopilotPause {
                reason,
                pct: Some(w.pct),
                until,
            })
        })
        .collect();
    if let Some(until) = hold.limit_until.filter(|t| *t > now) {
        pauses.push(AutopilotPause {
            reason: PauseReason::Limit,
            pct: None,
            until,
        });
    }
    pauses.into_iter().max_by_key(|p| p.until)
}

/// The refusal of a first ticket on a detached HEAD: there is no branch to start the tickets from
/// (nor to merge them into) until one is chosen.
pub fn no_branch(lang: Lang) -> String {
    tr_in!(
        lang,
        "Le projet n'est sur aucune branche : choisis la branche cible dans les réglages du Kanban.",
        "The project isn’t on any branch: choose the target branch in the Kanban settings."
    )
}

/// Why no ticket starts from `target`, if none can: no branch at all (a board saved without one,
/// on a detached HEAD), a branch without any commit yet (a new repository: its first commit makes
/// it), or one that is gone.
pub fn target_issue(lang: Lang, target: &str, exists: bool, unborn: bool) -> Option<String> {
    if target.is_empty() {
        Some(no_branch(lang))
    } else if exists {
        None
    } else if unborn {
        Some(tr_in!(
            lang,
            "{target} n'a encore aucun commit — aucun ticket ne démarre",
            "{target} has no commit yet — no tickets will start"
        ))
    } else {
        Some(tr_in!(
            lang,
            "Branche cible {target} introuvable — aucun ticket ne démarre",
            "Target branch {target} not found — no tickets will start"
        ))
    }
}

/// Back to "À faire" from scratch: its agent was archived or deleted by hand.
pub fn back_to_todo(t: &mut Ticket) {
    t.column = Column::Todo;
    t.agent_id = None;
    t.iteration = 0;
    t.loops = 0;
    t.partial = false;
    t.blocked = None;
    t.conflict = false;
    t.step = None;
    t.reminded = false;
    t.forced = false;
    t.started_at = None;
    t.review_at = None;
    t.progress.clear();
    for c in &mut t.criteria {
        c.ok = false;
        c.note.clear();
    }
}

/// Back "En cours" with the same agent and worktree, from loop 1: its tests failed, a conflict
/// was handed to it, or it was sent back ("Renvoyer"). What it did and its last report stay; the
/// loop counts with the others.
pub fn back_to_work(t: &mut Ticket) {
    t.column = Column::Doing;
    t.iteration = 1;
    t.loops += 1;
    t.partial = false;
    t.blocked = None;
    t.conflict = false;
    t.step = None;
    t.review_at = None;
    t.reminded = false;
}

/// What the app's start does to a ticket "En cours" and not blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    /// Its agent is asked to go on (`restart_message`).
    GoOn,
    /// Nothing: its agent waits for its quota, and its automatic resume takes it up.
    Wait,
    /// Back to "À faire": the app stopped while starting it.
    Again,
}

/// From its agent (None when gone or archived) and whether the app's stop cut that agent's turn:
/// one that never got its first message, or none, means the start never finished; any other
/// goes on, its turn cut or not (a stop between a turn's end and its reading leaves it idle with
/// nothing to move it), unless it waits for its quota.
pub fn recovery(agent: Option<&AgentMeta>, cut: bool) -> Recovery {
    match agent {
        None => Recovery::Again,
        Some(m) if m.prompts == 0 => Recovery::Again,
        Some(m) if m.resume_at.is_some() && !cut => Recovery::Wait,
        Some(_) => Recovery::GoOn,
    }
}

fn numbered(t: &Ticket) -> String {
    t.criteria
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}. {}", i + 1, c.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first message of a ticket's agent.
pub fn first_message(lang: Lang, t: &Ticket) -> String {
    let (key, title) = (&t.key, &t.title);
    let mut s = tr_in!(lang, "Ticket {key} : {title}\n", "Ticket {key}: {title}\n");
    if !t.description.trim().is_empty() {
        s.push_str(&format!("\n{}\n", t.description.trim()));
    }
    s.push_str(&tr_in!(
        lang,
        "\nCritères d'acceptation :\n{criteria}\n\nBoucle 1/{max}. Travaille jusqu'à atteindre tous les critères, puis termine par le bilan.",
        "\nAcceptance criteria:\n{criteria}\n\nLoop 1/{max}. Work until every criterion is met, then end with the report.",
        criteria = numbered(t),
        max = t.max_loops
    ));
    s
}

/// `message`, followed by what went wrong with the setup of the agent's worktree, if anything
/// (`failure`, written in `lang` too).
pub fn with_setup_failure(lang: Lang, message: String, failure: Option<&str>) -> String {
    match failure {
        Some(f) => tr_in!(
            lang,
            "{message}\n\n{f} Fais le nécessaire pour pouvoir travailler et tester, puis continue.",
            "{message}\n\n{f} Do what it takes to be able to work and test, then go on.",
            f = f.trim()
        ),
        None => message,
    }
}

/// The message of the next loop: the criteria still missing, with the agent's notes.
pub fn loop_message(lang: Lang, t: &Ticket) -> String {
    let missing: Vec<String> = t
        .criteria
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.ok)
        .map(|(i, c)| {
            if c.note.is_empty() {
                (i + 1).to_string()
            } else {
                format!("{} ({})", i + 1, c.note)
            }
        })
        .collect();
    tr_in!(
        lang,
        "Boucle {i}/{max}. Critères non atteints : {missing}. Continue jusqu'à les atteindre, puis termine par le bilan.",
        "Loop {i}/{max}. Criteria not met: {missing}. Keep going until they are met, then end with the report.",
        i = t.iteration,
        max = t.max_loops,
        missing = missing.join(", ")
    )
}

/// The local hour and minute of a time in milliseconds ("15:07"), as the window shows a resume.
pub fn clock(ms: i64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default()
}

pub fn resume_message(lang: Lang, key: &str) -> String {
    tr_in!(
        lang,
        "Reprends le ticket {key} là où tu en étais, puis termine par le bilan.",
        "Pick up ticket {key} where you left off, then end with the report."
    )
}

pub fn restart_message(lang: Lang, key: &str) -> String {
    tr_in!(
        lang,
        "L'app a redémarré pendant ton travail sur {key} : reprends là où tu en étais, puis termine par le bilan.",
        "The app restarted while you were working on {key}: pick up where you left off, then end with the report."
    )
}

pub fn reject_message(lang: Lang, key: &str, comment: &str) -> String {
    let comment = comment.trim().trim_end_matches('.');
    tr_in!(
        lang,
        "Retour de test sur {key} : {comment}. Corrige, revérifie tous les critères, puis termine par le bilan.",
        "Test feedback on {key}: {comment}. Fix it, check every criterion again, then end with the report."
    )
}

pub fn tests_failed_message(lang: Lang, command: &str, tail: &str) -> String {
    tr_in!(
        lang,
        "Les tests (`{command}`) échouent :\n\n```\n{tail}\n```\n\nCorrige, revérifie les critères, puis termine par le bilan.",
        "The tests (`{command}`) fail:\n\n```\n{tail}\n```\n\nFix them, check the criteria again, then end with the report."
    )
}

pub fn conflict_message(lang: Lang, target: &str, files: &[String]) -> String {
    if files.is_empty() {
        return tr_in!(
            lang,
            "J'ai mergé {target} dans ta branche, sans conflit. Revérifie les critères, puis termine par le bilan.",
            "I merged {target} into your branch, without conflicts. Check the criteria again, then end with the report."
        );
    }
    tr_in!(
        lang,
        "Le merge de {target} dans ta branche a des conflits sur : {files}. Résous-les, commite le merge, revérifie les critères, puis termine par le bilan.",
        "The merge of {target} into your branch has conflicts in: {files}. Resolve them, commit the merge, check the criteria again, then end with the report.",
        files = files.join(", ")
    )
}

pub fn rebase_message(lang: Lang, target: &str) -> String {
    tr_in!(
        lang,
        "Rebase ta branche sur {target} et résous ses conflits, puis revérifie les critères et termine par le bilan.",
        "Rebase your branch onto {target} and resolve its conflicts, then check the criteria again and end with the report."
    )
}

/// "Préparer le lancement": the agent is asked how to launch its worktree, with its ports.
pub fn prepare_message(lang: Lang, base: u16) -> String {
    let (end, web) = (base.saturating_add(PORT_BLOCK - 1), base.saturating_add(1));
    tr_in!(
        lang,
        "Prépare le lancement de test de ce worktree. Ports réservés : {base} à {end} (ESCOUADE_PORT_BASE et ESCOUADE_PORT_END dans le lancement) : \
         utilise-les, sans les écrire dans des fichiers versionnés (passe-les en arguments ou en variables d'environnement). \
         Termine ta réponse par un bloc :\n\n```escouade\n{{\"lancement\": {{\"preparation\": [{{\"commande\": \"npm install\", \"dossier\": \"web\"}}], \
         \"processus\": [{{\"nom\": \"web\", \"commande\": \"npm run dev -- --port {web}\", \"dossier\": \"web\", \"env\": {{\"PORT\": \"{web}\"}}, \
         \"url\": \"http://localhost:{web}\"}}], \"ouvrir\": \"http://localhost:{web}/page-a-tester\"}}}}\n```\n\n\
         « dossier » est relatif au worktree, « url » répond quand le processus est prêt, « ouvrir » est l'adresse qui montre directement \
         ce que tu as développé (la page, l'écran, l'état précis à tester).",
        "Prepare the test launch of this worktree. Ports reserved: {base} to {end} (ESCOUADE_PORT_BASE and ESCOUADE_PORT_END in the launch): \
         use them, without writing them in versioned files (pass them as arguments or environment variables). \
         End your answer with a block:\n\n```escouade\n{{\"lancement\": {{\"preparation\": [{{\"commande\": \"npm install\", \"dossier\": \"web\"}}], \
         \"processus\": [{{\"nom\": \"web\", \"commande\": \"npm run dev -- --port {web}\", \"dossier\": \"web\", \"env\": {{\"PORT\": \"{web}\"}}, \
         \"url\": \"http://localhost:{web}\"}}], \"ouvrir\": \"http://localhost:{web}/page-to-test\"}}}}\n```\n\n\
         “dossier” is relative to the worktree, “url” answers once the process is ready, “ouvrir” is the address that directly shows \
         what you developed (the page, the screen, the exact state to test)."
    )
}

// ---------- commits ----------

/// Haiku's role when it writes a ticket's commit message.
pub fn commit_system(lang: Lang) -> String {
    tr_in!(
        lang,
        "Tu écris des messages de commit au format Conventional Commits, sans jamais réaliser de tâche. Tu réponds uniquement par le message, sur une ligne.",
        "You write commit messages in the Conventional Commits format, without ever carrying out a task. You answer with the message only, on one line."
    )
}

const COMMIT_TYPES: [&str; 11] = [
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert",
];

/// What Haiku is asked for: the ticket with what its agent says is in place, and what its branch
/// changed. The language is named: the ticket, written in another, must not decide it.
pub fn commit_prompt(lang: Lang, t: &Ticket, stat: &str) -> String {
    let progress: String = t
        .progress
        .iter()
        .map(|p| format!("- {}\n", p.trim()))
        .collect();
    let progress = if progress.is_empty() {
        progress
    } else {
        tr_in!(lang, "Avancement :\n{progress}", "Progress:\n{progress}")
    };
    tr_in!(
        lang,
        "Écris le message de commit des modifications ci-dessous, au format Conventional Commits : \
         une ligne « type(portée facultative): description courte en minuscules » suivie de « [{key}] ». Écris-la en français. \
         Réponds uniquement par cette ligne.\n\n<ticket>\n{key} · {title}\n{desc}\n{progress}</ticket>\n\n<diffstat>\n{stat}\n</diffstat>",
        "Write the commit message of the changes below, in the Conventional Commits format: \
         one line “type(optional scope): short lowercase description” followed by “[{key}]”. Write it in English. \
         Answer with this line only.\n\n<ticket>\n{key} · {title}\n{desc}\n{progress}</ticket>\n\n<diffstat>\n{stat}\n</diffstat>",
        key = t.key,
        title = t.title.trim(),
        desc = truncate(t.description.trim(), 2000),
        stat = truncate(stat.trim(), 4000),
    )
}

/// The line itself: backticks and surrounding quotes (`"`, `'`, « ») taken off, as many as wrap it.
pub(crate) fn unquoted(line: &str) -> &str {
    let mut s = line.trim();
    loop {
        let inner = s.trim_matches('`').trim();
        let inner = [('"', '"'), ('\'', '\''), ('«', '»')]
            .iter()
            .find_map(|&(open, close)| inner.strip_prefix(open).and_then(|x| x.strip_suffix(close)))
            .map_or(inner, str::trim);
        if inner == s {
            return s;
        }
        s = inner;
    }
}

/// Haiku's answer as a commit message, if it is one: `type(scope)!: description [KEY]`, on one
/// line of at most 100 characters. Its first line counts, fences (```, ```text) left aside, and
/// quotes around it do not.
pub fn commit_from_answer(raw: &str, key: &str) -> Option<String> {
    let fence = |l: &str| l.starts_with("```") && !l.trim_start_matches('`').contains([' ', ':']);
    let line = raw
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !fence(l))?;
    let line = unquoted(line);
    let head = line.strip_suffix(&format!(" [{key}]"))?;
    let (kind, desc) = head.split_once(": ")?;
    let kind = kind.strip_suffix('!').unwrap_or(kind);
    let ty = match kind.split_once('(') {
        Some((ty, scope)) => {
            let scope = scope.strip_suffix(')')?;
            if scope.is_empty() || scope.contains([' ', '(', ')']) {
                return None;
            }
            ty
        }
        None => kind,
    };
    (COMMIT_TYPES.contains(&ty) && !desc.trim().is_empty() && line.chars().count() <= 100)
        .then(|| line.to_string())
}

/// Why a validation stops on files copied from the project that the ticket's branch carries:
/// those in its tree (`in_tree`) first, else those only in its history, so that the user knows
/// what to fix. None when there are none.
pub fn copied_refusal(lang: Lang, in_tree: &[String], in_history: &[String]) -> Option<String> {
    let in_branch = !in_tree.is_empty();
    let files = if in_branch { in_tree } else { in_history };
    if files.is_empty() {
        return None;
    }
    let (n, list) = (files.len(), files.join(", "));
    Some(if in_branch {
        tr_n_in!(
            lang,
            n,
            "{list} copié du projet est commité dans la branche",
            "{list} copiés du projet sont commités dans la branche",
            "{list}, copied from the project, is committed in the branch",
            "{list}, copied from the project, are committed in the branch"
        )
    } else {
        tr_n_in!(
            lang,
            n,
            "{list} copié du projet est dans l'historique de la branche",
            "{list} copiés du projet sont dans l'historique de la branche",
            "{list}, copied from the project, is in the branch’s history",
            "{list}, copied from the project, are in the branch’s history"
        )
    })
}

/// When Haiku gave nothing usable.
pub fn fallback_commit(t: &Ticket) -> String {
    format!("feat: {} [{}]", t.title.trim(), t.key)
}

/// Without "Message de commit généré".
pub fn plain_commit(t: &Ticket) -> String {
    format!("{} {}", t.key, t.title.trim())
}

// ---------- outcomes ----------

fn strategy_label(strategy: &str) -> &'static str {
    match strategy {
        "merge" => "merge commit",
        "rebase" => "rebase",
        _ => "squash",
    }
}

// What became of a ticket approved, on its card: written once, in the language of that moment.

/// What became of a ticket approved, written in the two languages it is read in: the interface's
/// (`ui`, the card) and that of the texts for Claude (`claude`, what the comment published for the
/// team quotes: in the card's, it would mix two languages), with the link that goes with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub ui: String,
    pub claude: String,
    pub url: Option<String>,
}

impl Outcome {
    /// `text`, written in both (`claude`: the language of the texts for Claude).
    pub fn of(claude: Lang, text: impl Fn(Lang) -> String) -> Self {
        Self {
            ui: text(i18n::ui()),
            claude: text(claude),
            url: None,
        }
    }

    pub fn with_url(self, url: String) -> Self {
        Self {
            url: Some(url),
            ..self
        }
    }
}

pub fn merged_outcome(lang: Lang, target: &str, strategy: &str) -> String {
    let how = strategy_label(strategy);
    tr_in!(
        lang,
        "⤵ Mergé dans {target} · {how}",
        "⤵ Merged into {target} · {how}"
    )
}

pub fn pr_outcome(number: u32, target: &str) -> String {
    format!("⇡ PR #{number} → {target}")
}

/// Pushed to GitHub without `gh`: the PR is finished in the browser.
pub fn pushed_for_pr_outcome(lang: Lang, branch: &str) -> String {
    tr_in!(
        lang,
        "⇡ {branch} poussée · PR à finaliser",
        "⇡ {branch} pushed · PR to finish"
    )
}

/// Pushed to another host than GitHub.
pub fn pushed_elsewhere_outcome(lang: Lang, branch: &str) -> String {
    tr_in!(
        lang,
        "⇡ {branch} poussée · PR à ouvrir sur ton hébergeur",
        "⇡ {branch} pushed · PR to open on your host"
    )
}

pub fn pushed_outcome(lang: Lang, branch: &str) -> String {
    tr_in!(lang, "⇡ Poussé sur {branch}", "⇡ Pushed to {branch}")
}

pub fn kept_outcome(lang: Lang) -> String {
    tr_in!(lang, "◇ Laissé dans le worktree", "◇ Left in the worktree")
}

/// Validated with nothing to merge, propose or push: its branch brings no change.
pub fn nothing_outcome(lang: Lang) -> String {
    tr_in!(lang, "∅ Aucune modification", "∅ No changes")
}

/// Added to a merge's outcome when its worktree and branch could not be removed after it.
pub fn worktree_kept(lang: Lang) -> String {
    tr_in!(lang, " · worktree gardé", " · worktree kept")
}

// ---------- GitHub ----------

/// (owner, repository) of a GitHub remote's address (https, ssh, scp-like).
pub fn github_repo(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let path = match url.strip_prefix("git@github.com:") {
        Some(p) => p,
        None => {
            let rest = url.split_once("://")?.1;
            let host_path = rest.rsplit_once('@').map_or(rest, |(_, h)| h);
            host_path.strip_prefix("github.com/")?
        }
    };
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    (!owner.is_empty() && !repo.is_empty() && !repo.contains('/'))
        .then(|| (owner.to_string(), repo.to_string()))
}

/// Percent-encoding of everything but the unreserved characters (and `/` when `keep_slash`).
pub(crate) fn encode(s: &str, keep_slash: bool) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) || (keep_slash && b == b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// What the compare page's address keeps of the title and of the description, in bytes before
/// encoding (each may triple): the address stays a few KB, which browsers and the system's opener
/// take (a longer one meets "414 URI Too Long").
const COMPARE_TITLE_MAX: usize = 200;
const COMPARE_BODY_MAX: usize = 1500;

/// GitHub's page that opens a pull request of `branch` into `target`, filled in (the title and
/// the description cut, with a "…", when long).
pub fn compare_url(
    owner: &str,
    repo: &str,
    target: &str,
    branch: &str,
    title: &str,
    body: &str,
) -> String {
    format!(
        "https://github.com/{owner}/{repo}/compare/{}...{}?expand=1&title={}&body={}",
        encode(target, true),
        encode(branch, true),
        encode(&truncate(title, COMPARE_TITLE_MAX), false),
        encode(&truncate(body, COMPARE_BODY_MAX), false)
    )
}

/// The number and address of the PR `gh pr create` printed.
pub fn pr_number(gh_output: &str) -> Option<(u32, String)> {
    gh_output.split_whitespace().find_map(|w| {
        let (_, n) = w.rsplit_once("/pull/")?;
        Some((n.trim_end_matches('/').parse().ok()?, w.to_string()))
    })
}

/// A PR's description, in `lang`: the ticket's, what its agent says it did (when it said), then
/// its criteria as a checklist.
pub fn pr_body(lang: Lang, t: &Ticket) -> String {
    let mut parts = Vec::new();
    if !t.description.trim().is_empty() {
        parts.push(t.description.trim().to_string());
    }
    let done: Vec<String> = t
        .progress
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| format!("- {p}"))
        .collect();
    if !done.is_empty() {
        let done = done.join("\n");
        parts.push(tr_in!(
            lang,
            "Ce qui a été fait :\n{done}",
            "What was done:\n{done}"
        ));
    }
    let list = t
        .criteria
        .iter()
        .map(|c| format!("- [{}] {}", if c.ok { "x" } else { " " }, c.text))
        .collect::<Vec<_>>()
        .join("\n");
    parts.push(tr_in!(lang, "Critères :\n{list}", "Criteria:\n{list}"));
    parts.join("\n\n")
}

// ---------- ports ----------

pub const PORT_FIRST: u16 = 4100;
pub const PORT_BLOCK: u16 = 10;

/// The first block of 10 ports from 4100 (by steps of 10) that no agent holds and whose ports all
/// bind (`bindable`).
pub fn allocate_ports(taken: &[u16], bindable: impl Fn(u16) -> bool) -> Option<u16> {
    let mut base = PORT_FIRST;
    while base <= u16::MAX - PORT_BLOCK {
        if !taken.contains(&base) && (base..base + PORT_BLOCK).all(&bindable) {
            return Some(base);
        }
        base += PORT_BLOCK;
    }
    None
}

/// Text on one line (a command-line argument), cut to `max` characters: whitespace collapsed to
/// a space, other control characters (NUL, which makes the launch fail, bell, escape…) dropped.
fn one_line(s: &str, max: usize) -> String {
    let printable: String = s
        .chars()
        .filter(|c| c.is_whitespace() || !c.is_control())
        .collect();
    truncate(
        &printable.split_whitespace().collect::<Vec<_>>().join(" "),
        max,
    )
}

/// What the protocol may weigh as a command-line argument, once escaped (see `escaped_len`).
/// cmd.exe takes 8191 characters for the whole command line; the rest goes to the other arguments.
pub(crate) const PROTOCOL_BUDGET: usize = 6000;

/// What goes between two criteria of the protocol, with the punctuation of `lang`.
fn between(lang: Lang) -> &'static str {
    match lang {
        Lang::Fr => " ; ",
        Lang::En => "; ",
    }
}

/// Ends the criteria when some did not fit.
fn more(lang: Lang) -> &'static str {
    match lang {
        Lang::Fr => " ; …",
        Lang::En => "; …",
    }
}

/// What `s` weighs as an argument of a `.cmd`: Rust escapes it for cmd.exe, where `%` becomes a
/// longer sequence (8 characters) and `"` is doubled, as are the backslashes just before it (and
/// those ending the argument). Every backslash counts 2, the safe bound of those runs. Other
/// characters count their UTF-8 bytes, which is never less than their UTF-16 units.
pub(crate) fn escaped_len(s: &str) -> usize {
    s.chars()
        .map(|c| match c {
            '%' => 8,
            '"' | '\\' => 2,
            c => c.len_utf8(),
        })
        .sum()
}

/// The protocol around `criteria`, already joined, in `lang`: with the reserved ports and the
/// recipe asked for them, or isola's services when `isola` runs the worktree's.
fn protocol(lang: Lang, t: &Ticket, criteria: &str, ports: Option<u16>, isola: bool) -> String {
    let mut p = tr_in!(
        lang,
        "Tu travailles en autonomie sur le ticket {key} « {title} » d'Escouade. \
         Critères d'acceptation ({n}) : {criteria}. \
         Vérifie toi-même chaque critère (tests, exécution) avant de le déclarer atteint. \
         Ne commite pas les fichiers copiés du projet (.env…). \
         S'il te faut une décision, pose la question avec l'outil de question. \
         À la fin de CHAQUE réponse, termine par un bloc de code ouvert par ```escouade et fermé par ``` \
         qui contient un JSON {{\"criteres\": [{{\"n\": 1, \"ok\": true, \"note\": \"vérifié par …\"}}, \
         {{\"n\": 2, \"ok\": false, \"note\": \"ce qui manque\"}}], \
         \"avancement\": [\"Tokens d'accès signés\", \"Middleware réécrit\", \"Adaptateur des sessions\"]}} \
         avec un élément par critère dans \"criteres\" (n à partir de 1). \
         Ajoute au bilan « avancement » : la liste succincte (3 à 8 éléments courts) des fonctionnalités en place jusque-là.",
        "You are working autonomously on Escouade ticket {key} “{title}”. \
         Acceptance criteria ({n}): {criteria}. \
         Check each criterion yourself (tests, runs) before declaring it met. \
         Don’t commit the files copied from the project (.env…). \
         If you need a decision, ask the question with the question tool. \
         At the end of EVERY answer, finish with a code block opened by ```escouade and closed by ``` \
         that holds a JSON {{\"criteres\": [{{\"n\": 1, \"ok\": true, \"note\": \"checked by …\"}}, \
         {{\"n\": 2, \"ok\": false, \"note\": \"what is missing\"}}], \
         \"avancement\": [\"Signed access tokens\", \"Middleware rewritten\", \"Session adapter\"]}} \
         with one item per criterion in \"criteres\" (n from 1). \
         Add “avancement” to the report: the brief list (3 to 8 short items) of the features in place so far.",
        key = t.key,
        title = one_line(&t.title, 200),
        n = t.criteria.len(),
    );
    if let Some(base) = ports {
        let (end, web) = (base.saturating_add(9), base.saturating_add(1));
        p.push_str(&tr_in!(
            lang,
            " Ports réservés à ce worktree : {base} à {end} (ESCOUADE_PORT_BASE et ESCOUADE_PORT_END dans ses lancements de test) ; \
             ne les écris pas dans des fichiers versionnés, passe-les en arguments ou en variables d'environnement. \
             Au plus tard quand tous les critères sont atteints, ajoute au JSON une clé \"lancement\" qui dit comment lancer \
             ce worktree pour le tester, par exemple \"lancement\": {{\"preparation\": [{{\"commande\": \"npm install\", \"dossier\": \"web\"}}], \
             \"processus\": [{{\"nom\": \"web\", \"commande\": \"npm run dev -- --port {web}\", \"dossier\": \"web\", \
             \"env\": {{\"PORT\": \"{web}\"}}, \"url\": \"http://localhost:{web}\"}}], \"ouvrir\": \"http://localhost:{web}/page-de-la-fonctionnalite\"}} : \
             « dossier » est relatif au worktree, « url » répond quand le processus est prêt, « ouvrir » est l'adresse qui montre \
             directement la fonctionnalité développée (la page, l'écran, l'état précis à tester).",
            " Ports reserved for this worktree: {base} to {end} (ESCOUADE_PORT_BASE and ESCOUADE_PORT_END in its test launches); \
             don’t write them in versioned files, pass them as arguments or environment variables. \
             At the latest when every criterion is met, add to the JSON a key \"lancement\" that says how to launch \
             this worktree to test it, for example \"lancement\": {{\"preparation\": [{{\"commande\": \"npm install\", \"dossier\": \"web\"}}], \
             \"processus\": [{{\"nom\": \"web\", \"commande\": \"npm run dev -- --port {web}\", \"dossier\": \"web\", \
             \"env\": {{\"PORT\": \"{web}\"}}, \"url\": \"http://localhost:{web}\"}}], \"ouvrir\": \"http://localhost:{web}/feature-page\"}}: \
             “dossier” is relative to the worktree, “url” answers once the process is ready, “ouvrir” is the address that directly \
             shows the feature developed (the page, the screen, the exact state to test)."
        ));
    } else if isola {
        p.push_str(&tr_in!(
            lang,
            " isola gère les services de ce worktree (.isola.toml) : « isola up » les lance sur leurs propres ports, \
             « isola ls --json » donne leurs adresses, « isola down » les arrête ; ne choisis aucun port toi-même. \
             Au plus tard quand tous les critères sont atteints, ajoute au JSON une clé \"lancement\": {{\"ouvrir\": \"<adresse>\"}} \
             avec l'adresse isola qui montre directement la fonctionnalité développée (la page, l'écran, l'état précis à tester).",
            " isola runs the services of this worktree (.isola.toml): “isola up” starts them on their own ports, \
             “isola ls --json” gives their addresses, “isola down” stops them; don’t pick any port yourself. \
             At the latest when every criterion is met, add to the JSON a key \"lancement\": {{\"ouvrir\": \"<address>\"}} \
             with the isola address that directly shows the feature developed (the page, the screen, the exact state to test)."
        ));
    }
    p
}

/// The ticket's protocol, appended to Claude Code's system prompt (`--append-system-prompt`), so
/// that it outlives compaction and resumes. On one line: it is a command-line argument, which a
/// `.cmd` (npm's claude.cmd) cannot take with line breaks. Once escaped for cmd.exe it weighs at
/// most `PROTOCOL_BUDGET`: the criteria that do not fit are replaced by a "…". In French here
/// (most tests read it so); `protocol_prompt_for` writes it in any language.
#[cfg(test)]
pub fn protocol_prompt(t: &Ticket, ports: Option<u16>) -> String {
    protocol_prompt_for(Lang::Fr, t, ports, false)
}

/// `protocol_prompt` in `lang`, for a worktree whose services isola runs when `isola`.
pub fn protocol_prompt_for(lang: Lang, t: &Ticket, ports: Option<u16>, isola: bool) -> String {
    let room = PROTOCOL_BUDGET.saturating_sub(escaped_len(&protocol(lang, t, "", ports, isola)));
    let mut criteria = String::new();
    let mut used = 0;
    for (i, c) in t.criteria.iter().enumerate() {
        let item = format!(
            "{}{}) {}",
            if i > 0 { between(lang) } else { "" },
            i + 1,
            one_line(&c.text, 200)
        );
        let weight = escaped_len(&item);
        // Room is kept for the "…" unless this is the last criterion.
        let more_weight = if i + 1 < t.criteria.len() {
            escaped_len(more(lang))
        } else {
            0
        };
        if used + weight + more_weight > room {
            criteria.push_str(more(lang));
            break;
        }
        used += weight;
        criteria.push_str(&item);
    }
    protocol(lang, t, &criteria, ports, isola)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(n: usize, max: u32) -> Ticket {
        Ticket {
            id: "t1".into(),
            project_id: "p1".into(),
            key: "ATL-42".into(),
            title: "Limiter les tentatives".into(),
            criteria: (1..=n)
                .map(|i| Criterion {
                    text: format!("critère {i}"),
                    ..Default::default()
                })
                .collect(),
            max_loops: max,
            column: Column::Doing,
            iteration: 1,
            ..Default::default()
        }
    }

    fn block(json: &str) -> String {
        format!("Voilà.\n\n```escouade\n{json}\n```\n")
    }

    fn finished(json: &str) -> (TurnEnd, Option<Report>) {
        let text = block(json);
        (TurnEnd::Finished(text.clone()), parse_report(&text))
    }

    #[test]
    fn keys_names_and_branches_come_from_the_project_and_the_title() {
        assert_eq!(key_prefix("atlas-api"), "ATL");
        assert_eq!(key_prefix("Écoute"), "ECO");
        assert_eq!(key_prefix("ab"), "AB");
        assert_eq!(key_prefix("42"), "TIC");
        assert_eq!(
            agent_name("ATL-42", "Limiter les tentatives"),
            "atl-42-limiter-les-tentatives"
        );
        assert_eq!(branch_of("ATL-42"), "ticket/atl-42");
        let texts = |c: Vec<Criterion>| c.into_iter().map(|c| c.text).collect::<Vec<_>>();
        assert_eq!(
            texts(criteria_from(&["  a ".into(), "".into(), "b".into()])),
            ["a", "b"]
        );
        assert_eq!(
            texts(criteria_from(&[])),
            ["Implémentation conforme au ticket", "Tests verts"]
        );
        assert_eq!(
            (max_loops(3), max_loops(8), max_loops(0), max_loops(7)),
            (3, 8, 5, 5)
        );
    }

    #[test]
    fn the_last_escouade_block_is_read() {
        let text = format!(
            "{}\nPuis :\n{}",
            block(r#"{"criteres": [{"n": 1, "ok": false}]}"#),
            block(
                r#"{"criteres": [{"n": 1, "ok": true, "note": "vu"}, {"n": 2, "ok": false, "note": "reste"}]}"#
            )
        );
        let r = parse_report(&text).unwrap();
        assert_eq!(
            r.criteria,
            Some(vec![
                (1, true, "vu".to_string()),
                (2, false, "reste".to_string())
            ])
        );
        // `criteria` is understood too.
        let r = parse_report(&block(r#"{"criteria": [{"n": 2, "ok": true}]}"#)).unwrap();
        assert_eq!(r.criteria, Some(vec![(2, true, String::new())]));
    }

    #[test]
    fn a_missing_or_unreadable_block_is_no_report() {
        assert_eq!(parse_report("Fini, tout marche."), None);
        assert_eq!(parse_report(&block("{criteres: oui}")), None);
        assert_eq!(parse_report("```escouade\n{\"criteres\": []}"), None);
        // A readable block without criteria (a launch recipe alone): a report without them.
        assert_eq!(parse_report(&block("{}")).unwrap().criteria, None);
    }

    #[test]
    fn a_recipe_is_kept_unless_a_folder_leaves_the_worktree() {
        let r = parse_report(&block(
            r#"{"criteres": [], "lancement": {"preparation": [{"commande": "npm install", "dossier": "web"}], "processus": [{"nom": "web", "commande": "npm run dev", "dossier": "web", "env": {"PORT": 4111}, "url": "http://localhost:4111"}], "ouvrir": "http://localhost:4111/connexion"}}"#,
        ))
        .unwrap();
        let recipe = r.recipe.unwrap();
        assert_eq!(recipe.processes[0].env["PORT"], "4111");
        assert_eq!(recipe.open, "http://localhost:4111/connexion");
        for bad in ["../x", "web/../../x", "/etc", "C:\\x", "..\\x"] {
            let json = format!(
                r#"{{"lancement": {{"processus": [{{"nom": "x", "commande": "y", "dossier": {}}}]}}}}"#,
                serde_json::to_string(bad).unwrap()
            );
            assert_eq!(parse_report(&block(&json)).unwrap().recipe, None, "{bad}");
        }
    }

    #[test]
    fn a_recipe_that_only_gives_the_address_to_open_is_kept_but_an_empty_one_is_not() {
        // What an agent whose services isola runs gives.
        let r = parse_report(&block(
            r#"{"criteres": [], "lancement": {"ouvrir": "http://ticket-atl-42.demo.localhost:3000/connexion"}}"#,
        ))
        .unwrap();
        assert_eq!(
            r.recipe.map(|x| x.open),
            Some("http://ticket-atl-42.demo.localhost:3000/connexion".to_string())
        );
        for empty in [r#"{}"#, r#"{"ouvrir": "  "}"#, r#"{"ouvrir": null}"#] {
            let json = format!(r#"{{"criteres": [], "lancement": {empty}}}"#);
            assert_eq!(parse_report(&block(&json)).unwrap().recipe, None, "{empty}");
        }
    }

    #[test]
    fn a_null_env_in_the_recipe_does_not_cost_the_recipe_or_the_report() {
        let r = parse_report(&block(
            r#"{"criteres": [{"n": 1, "ok": true}], "lancement": {"processus": [{"nom": "web", "commande": "npm run dev", "env": null}, {"nom": "api", "commande": "npm start", "env": {"PORT": 4112, "DEBUG": null}}]}}"#,
        ))
        .unwrap();
        assert_eq!(r.criteria, Some(vec![(1, true, String::new())]));
        let recipe = r.recipe.unwrap();
        assert!(recipe.processes[0].env.is_empty());
        assert_eq!(recipe.processes[1].env.keys().collect::<Vec<_>>(), ["PORT"]);
    }

    #[test]
    fn all_criteria_met_send_the_ticket_to_review() {
        let mut t = ticket(2, 5);
        let (end, r) = finished(
            r#"{"criteres": [{"n": 1, "ok": true}, {"n": 2, "ok": true, "note": "tests verts"}]}"#,
        );
        let next = turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 100);
        assert_eq!(
            (t.column, t.partial, t.review_at),
            (Column::Review, false, Some(100))
        );
        assert_eq!(t.criteria[1].note, "tests verts");
        assert_eq!(
            next,
            Next {
                ready: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn criteria_still_missing_start_the_next_loop_until_the_last_one() {
        let mut t = ticket(4, 3);
        let (end, r) = finished(
            r#"{"criteres": [{"n": 1, "ok": true}, {"n": 2, "ok": false, "note": "reste la rotation"}, {"n": 3, "ok": true}]}"#,
        );
        // Its first loop under way, a loop of an earlier round counted already.
        t.loops = 2;
        let next = turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 1);
        assert_eq!((t.iteration, t.loops), (2, 3));
        // A criterion left out counts as missing.
        assert_eq!(
            next.send.as_deref(),
            Some("Boucle 2/3. Critères non atteints : 2 (reste la rotation), 4. Continue jusqu'à les atteindre, puis termine par le bilan.")
        );
        turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 2);
        assert_eq!((t.iteration, t.loops), (3, 4));
        let next = turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 3);
        assert_eq!((t.column, t.partial, t.loops), (Column::Review, true, 4));
        assert!(next.ready && next.send.is_none());
    }

    #[test]
    fn a_missing_report_is_asked_for_once_then_blocks() {
        let mut t = ticket(1, 5);
        let end = TurnEnd::Finished("Fini.".into());
        let next = turn_end(Lang::Fr, &mut t, &end, None, 1);
        assert_eq!(next.send.as_deref(), Some(reminder(Lang::Fr).as_str()));
        assert_eq!((t.iteration, t.blocked.as_deref()), (1, None));
        let next = turn_end(Lang::Fr, &mut t, &end, None, 2);
        assert_eq!(t.blocked.as_deref(), Some("Bilan des critères manquant"));
        assert!(next.blocked && next.send.is_none());
        // A report with no criteria (a launch recipe alone) is no report either.
        let mut t = ticket(1, 5);
        let (end, r) = finished("{}");
        assert_eq!(
            turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 1)
                .send
                .as_deref(),
            Some(reminder(Lang::Fr).as_str())
        );
    }

    #[test]
    fn a_stop_or_an_error_blocks_and_the_usage_limit_waits() {
        let mut t = ticket(1, 5);
        assert!(turn_end(Lang::Fr, &mut t, &TurnEnd::Interrupted, None, 1).blocked);
        assert_eq!(t.blocked.as_deref(), Some("Interrompu"));
        let mut t = ticket(1, 5);
        turn_end(
            Lang::Fr,
            &mut t,
            &TurnEnd::Error("\nAPI Error: 500\nretry".into()),
            None,
            1,
        );
        assert_eq!(t.blocked.as_deref(), Some("Erreur : API Error: 500"));
        let mut t = ticket(1, 5);
        turn_end(Lang::Fr, &mut t, &TurnEnd::Error(String::new()), None, 1);
        assert_eq!(t.blocked.as_deref(), Some("Erreur : erreur inconnue"));
        let mut t = ticket(1, 5);
        assert_eq!(
            turn_end(Lang::Fr, &mut t, &TurnEnd::Limited, None, 1),
            Next::default()
        );
        assert_eq!((t.column, t.blocked.clone()), (Column::Doing, None));
    }

    #[test]
    fn a_turn_unblocks_its_ticket_but_one_no_longer_in_progress_changes_nothing() {
        let mut t = ticket(1, 5);
        t.blocked = Some("Interrompu".into());
        let (end, r) = finished(r#"{"criteres": [{"n": 1, "ok": true}]}"#);
        turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 1);
        assert_eq!((t.blocked.clone(), t.column), (None, Column::Review));
        let (end, r) = finished(r#"{"criteres": [{"n": 1, "ok": false}]}"#);
        assert_eq!(
            turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 2),
            Next::default()
        );
        assert_eq!(t.column, Column::Review);
    }

    fn todo(id: &str, rank: i64) -> Ticket {
        Ticket {
            id: id.into(),
            project_id: "p1".into(),
            column: Column::Todo,
            rank,
            ..Default::default()
        }
    }

    #[test]
    fn tickets_start_by_rank_while_places_are_free() {
        let s = BoardSettings::default();
        let mut list = vec![
            todo("c", 3),
            todo("a", 1),
            todo("b", 2),
            Ticket {
                project_id: "p2".into(),
                ..todo("other", 0)
            },
        ];
        assert_eq!(to_start(&list, "p1", &s, false), ["a", "b"]);
        list.push(Ticket {
            column: Column::Doing,
            ..todo("busy", 0)
        });
        assert_eq!(to_start(&list, "p1", &s, false), ["a"]);
        // A blocked ticket gives its place up; one to test holds none.
        list.push(Ticket {
            column: Column::Doing,
            blocked: Some("Interrompu".into()),
            ..todo("blocked", 0)
        });
        list.push(Ticket {
            column: Column::Review,
            ..todo("review", 0)
        });
        assert_eq!(to_start(&list, "p1", &s, false), ["a"]);
        list.push(Ticket {
            column: Column::Doing,
            ..todo("busy2", 0)
        });
        assert!(to_start(&list, "p1", &s, false).is_empty());
    }

    #[test]
    fn without_the_autopilot_only_launched_tickets_start_and_nothing_starts_on_a_quota() {
        let off = BoardSettings {
            autopilot: false,
            max_parallel: 6,
            ..Default::default()
        };
        let list = vec![
            todo("a", 1),
            Ticket {
                forced: true,
                ..todo("b", 2)
            },
        ];
        assert_eq!(to_start(&list, "p1", &off, false), ["b"]);
        assert!(to_start(&list, "p1", &BoardSettings::default(), true).is_empty());
        // A count out of bounds (an old file) still lets one run.
        let zero = BoardSettings {
            max_parallel: 0,
            ..Default::default()
        };
        assert_eq!(to_start(&list, "p1", &zero, false), ["a"]);
    }

    #[test]
    fn a_ticket_waiting_for_another_not_done_yet_is_skipped_for_the_next_one() {
        let s = BoardSettings::default();
        let mut list = vec![
            Ticket {
                column: Column::Doing,
                ..todo("before", 0)
            },
            Ticket {
                after: vec!["before".into()],
                ..todo("a", 1)
            },
            todo("b", 2),
        ];
        // The one place left goes to the next ticket of the column.
        assert_eq!(to_start(&list, "p1", &s, false), ["b"]);
        // To test, the ticket it waits for holds no place, but is not done.
        list[0].column = Column::Review;
        assert_eq!(to_start(&list, "p1", &s, false), ["b"]);
        // Done, it starts in its turn.
        list[0].column = Column::Done;
        assert_eq!(to_start(&list, "p1", &s, false), ["a", "b"]);
    }

    #[test]
    fn a_ticket_waits_for_every_ticket_it_comes_after_unless_launched_by_hand() {
        let s = BoardSettings {
            max_parallel: 6,
            ..Default::default()
        };
        let mut list = vec![
            Ticket {
                column: Column::Done,
                ..todo("done", 0)
            },
            Ticket {
                column: Column::Review,
                ..todo("review", 0)
            },
            Ticket {
                after: vec!["done".into(), "review".into()],
                ..todo("a", 1)
            },
            // A ticket that is gone, or another project's (a hand-edited file), holds nothing back.
            Ticket {
                after: vec!["gone".into(), "other".into()],
                ..todo("b", 2)
            },
            Ticket {
                project_id: "p2".into(),
                ..todo("other", 0)
            },
        ];
        assert_eq!(to_start(&list, "p1", &s, false), ["b"]);
        // "Le lancer quand même ?": launched by hand, it starts all the same, with the autopilot
        // or without.
        list[2].forced = true;
        assert_eq!(to_start(&list, "p1", &s, false), ["a", "b"]);
        let off = BoardSettings {
            autopilot: false,
            ..s
        };
        assert_eq!(to_start(&list, "p1", &off, false), ["a"]);
    }

    #[test]
    fn a_ticket_comes_after_other_tickets_of_its_project_each_once() {
        let list = vec![
            todo("a", 1),
            todo("b", 2),
            Ticket {
                project_id: "p2".into(),
                ..todo("x", 1)
            },
        ];
        let wanted = ["b", "a", "b", "x", "gone", "c"].map(String::from);
        // Neither itself, nor a ticket that is gone or another project's.
        assert_eq!(after_of(&list, "p1", "c", &wanted), ["b", "a"]);
        assert_eq!(after_of(&list, "p1", "a", &wanted), ["b"]);
    }

    #[test]
    fn a_ticket_that_already_waits_for_it_is_refused_as_a_dependency() {
        let ticket = |id: &str, key: &str, after: &[&str]| Ticket {
            key: key.into(),
            after: after.iter().map(|a| a.to_string()).collect(),
            ..todo(id, 1)
        };
        let list = vec![
            ticket("t3", "DEM-3", &["t4"]),
            ticket("t4", "DEM-4", &["t5"]),
            ticket("t5", "DEM-5", &[]),
            ticket("t6", "DEM-6", &["t7"]),
            // A loop already there (a hand-edited file) does not hang the search.
            ticket("t7", "DEM-7", &["t6"]),
        ];
        let refusal = |after: &[&str]| {
            let after: Vec<String> = after.iter().map(|a| a.to_string()).collect();
            cycle_refusal(Lang::Fr, &list, "t5", &after)
        };
        // Through DEM-4, then directly.
        assert_eq!(
            refusal(&["t6", "t3"]).as_deref(),
            Some("DEM-3 attend déjà DEM-5 (directement ou non).")
        );
        assert_eq!(
            refusal(&["t4"]).as_deref(),
            Some("DEM-4 attend déjà DEM-5 (directement ou non).")
        );
        assert_eq!(refusal(&["t6", "t7"]), None);
        assert_eq!(refusal(&[]), None);
        // DEM-3 after DEM-5 as well is no loop: it waits for it already.
        let after = vec!["t5".to_string(), "t4".to_string()];
        assert_eq!(cycle_refusal(Lang::Fr, &list, "t3", &after), None);
    }

    fn window(pct: f64, resets_at: Option<i64>) -> Option<RateWindow> {
        Some(RateWindow { pct, resets_at })
    }

    fn usage(five_hour: Option<RateWindow>, seven_day: Option<RateWindow>) -> UsageSnapshot {
        UsageSnapshot {
            five_hour,
            seven_day,
            ..Default::default()
        }
    }

    const NOW: i64 = 1_790_000_000_000;

    #[test]
    fn a_window_used_up_to_the_threshold_pauses_the_autopilot_until_its_end() {
        let free = Hold::default();
        let u = usage(
            window(40.0, Some(NOW + 10)),
            window(96.0, Some(NOW + 5_000)),
        );
        assert_eq!(
            autopilot_pause(&u, 95, &free, NOW),
            Some(AutopilotPause {
                reason: PauseReason::Week,
                pct: Some(96.0),
                until: NOW + 5_000 + RESUME_MARGIN_MS
            })
        );
        // Under the threshold, nothing holds.
        assert_eq!(autopilot_pause(&u, 100, &free, NOW), None);
        // At it, it holds, even read as a fraction times 100 that falls just under.
        let at = usage(window(94.999_999_9, Some(NOW + 10)), None);
        assert_eq!(
            autopilot_pause(&at, 95, &free, NOW).map(|p| p.reason),
            Some(PauseReason::FiveHour)
        );
        // Its end (and the margin after it) past, or its end unknown, it holds nothing back (a
        // reading that no longer comes would hold the tickets forever).
        let over = usage(
            window(100.0, Some(NOW - RESUME_MARGIN_MS)),
            window(100.0, None),
        );
        assert_eq!(autopilot_pause(&over, 100, &free, NOW), None);
        // Both over it: the one that ends last, which the tickets wait for.
        let both = usage(
            window(100.0, Some(NOW + 9_000)),
            window(97.0, Some(NOW + 5_000)),
        );
        assert_eq!(
            autopilot_pause(&both, 95, &free, NOW).map(|p| (p.reason, p.until)),
            Some((PauseReason::FiveHour, NOW + 9_000 + RESUME_MARGIN_MS))
        );
    }

    #[test]
    fn a_window_holds_the_tickets_back_a_little_past_its_end_as_a_resume_waits() {
        // The clocks may differ a little: as an agent's resume, the tickets wait a margin more.
        assert_eq!(RESUME_MARGIN_MS, 30_000);
        let ended = usage(window(100.0, Some(NOW - 1_000)), None);
        let free = Hold::default();
        assert_eq!(
            autopilot_pause(&ended, 100, &free, NOW).map(|p| p.until),
            Some(NOW - 1_000 + RESUME_MARGIN_MS)
        );
        assert_eq!(
            autopilot_pause(&ended, 100, &free, NOW - 1_000 + RESUME_MARGIN_MS),
            None
        );
    }

    #[test]
    fn a_usage_limit_with_no_resume_pauses_the_autopilot_for_a_while() {
        let hold = Hold {
            limit_until: Some(NOW + LIMIT_PAUSE_MS),
            ..Default::default()
        };
        assert_eq!(LIMIT_PAUSE_MS, 30 * 60_000);
        let unknown = UsageSnapshot::default();
        assert_eq!(
            autopilot_pause(&unknown, 100, &hold, NOW),
            Some(AutopilotPause {
                reason: PauseReason::Limit,
                pct: None,
                until: NOW + LIMIT_PAUSE_MS
            })
        );
        assert_eq!(
            autopilot_pause(&unknown, 100, &hold, NOW + LIMIT_PAUSE_MS),
            None
        );
        // A window over the threshold that ends later: it is the one waited for.
        let later = usage(window(100.0, Some(NOW + 2 * LIMIT_PAUSE_MS)), None);
        assert_eq!(
            autopilot_pause(&later, 100, &hold, NOW).map(|p| p.reason),
            Some(PauseReason::FiveHour)
        );
    }

    #[test]
    fn resuming_now_lifts_the_pause_until_the_end_the_windows_had() {
        let u = usage(
            window(100.0, Some(NOW + 9_000)),
            window(50.0, Some(NOW + 50_000)),
        );
        let mut hold = Hold {
            limit_until: Some(NOW + LIMIT_PAUSE_MS),
            ..Default::default()
        };
        hold.lift(&u, 90, NOW);
        assert_eq!(hold.limit_until, None);
        assert_eq!(autopilot_pause(&u, 90, &hold, NOW), None);
        assert_eq!(autopilot_pause(&u, 90, &hold, NOW + 8_999), None);
        // The weekly window, under the threshold then, was not lifted: over it since, it holds.
        let week = usage(
            window(100.0, Some(NOW + 9_000)),
            window(92.0, Some(NOW + 50_000)),
        );
        assert_eq!(
            autopilot_pause(&week, 90, &hold, NOW + 1_000).map(|p| p.reason),
            Some(PauseReason::Week)
        );
        // The next 5-hour window, over it in its turn, holds again.
        let next = usage(window(95.0, Some(NOW + 90_000)), None);
        assert_eq!(
            autopilot_pause(&next, 90, &hold, NOW + 9_000 + RESUME_MARGIN_MS).map(|p| p.reason),
            Some(PauseReason::FiveHour)
        );
    }

    #[test]
    fn a_quota_threshold_not_offered_is_100_percent() {
        for v in QUOTA_PAUSES {
            assert_eq!(quota_threshold(v), v);
        }
        assert_eq!(QUOTA_PAUSES, [80, 90, 95, 100]);
        // A hand-edited settings file: 0 would hold every ticket back for good.
        assert_eq!(quota_threshold(0), 100);
        assert_eq!(quota_threshold(85), 100);
    }

    #[test]
    fn a_target_that_cannot_start_a_ticket_says_why() {
        assert_eq!(target_issue(Lang::Fr, "main", true, false), None);
        assert_eq!(
            target_issue(Lang::Fr, "main", false, true).as_deref(),
            Some("main n'a encore aucun commit — aucun ticket ne démarre")
        );
        assert_eq!(
            target_issue(Lang::Fr, "release", false, false).as_deref(),
            Some("Branche cible release introuvable — aucun ticket ne démarre")
        );
        assert_eq!(
            target_issue(Lang::Fr, "", false, false),
            Some(no_branch(Lang::Fr))
        );
    }

    #[test]
    fn a_ticket_sent_back_to_todo_starts_again_from_scratch() {
        let mut t = ticket(2, 5);
        t.agent_id = Some("a1".into());
        t.iteration = 3;
        t.partial = true;
        t.blocked = Some("x".into());
        t.step = Some("Tests…".into());
        t.criteria[0].ok = true;
        t.criteria[0].note = "vu".into();
        t.progress = vec!["Middleware réécrit".into()];
        t.loops = 4;
        back_to_todo(&mut t);
        // A new agent starts from the target branch: nothing of this attempt is in place, nor
        // counts.
        assert!(t.progress.is_empty());
        assert_eq!(t.loops, 0);
        assert_eq!(
            (
                t.column,
                t.agent_id.clone(),
                t.iteration,
                t.partial,
                t.blocked.clone(),
                t.step.clone()
            ),
            (Column::Todo, None, 0, false, None, None)
        );
        assert!(t.criteria.iter().all(|c| !c.ok && c.note.is_empty()));
    }

    #[test]
    fn a_ticket_sent_back_to_work_keeps_its_agent_and_loops_again_from_one() {
        let mut t = ticket(2, 5);
        t.column = Column::Review;
        t.agent_id = Some("a1".into());
        t.iteration = 3;
        t.partial = true;
        t.blocked = Some("Conflit avec main".into());
        t.conflict = true;
        t.step = Some("Merge…".into());
        t.review_at = Some(1);
        t.reminded = true;
        t.started_at = Some(7);
        t.criteria[0].ok = true;
        t.criteria[0].note = "vu".into();
        t.progress = vec!["Middleware réécrit".into()];
        t.loops = 3;
        back_to_work(&mut t);
        // Another loop, counted with the others.
        assert_eq!(t.loops, 4);
        assert_eq!(
            (
                t.column,
                t.iteration,
                t.partial,
                t.blocked.clone(),
                t.conflict,
                t.step.clone(),
                t.review_at,
                t.reminded
            ),
            (Column::Doing, 1, false, None, false, None, None, false)
        );
        // Same agent, same worktree: what it did and its last report stay until its next one.
        assert_eq!((t.agent_id.as_deref(), t.started_at), (Some("a1"), Some(7)));
        assert_eq!(t.progress, ["Middleware réécrit"]);
        assert!(t.criteria[0].ok && t.criteria[0].note == "vu");
    }

    #[test]
    fn at_startup_a_ticket_under_way_goes_on_waits_for_its_quota_or_starts_again() {
        let worked = AgentMeta {
            prompts: 2,
            ..Default::default()
        };
        // Its turn cut by the app's stop, or stuck between a turn's end and its reading.
        assert_eq!(recovery(Some(&worked), true), Recovery::GoOn);
        assert_eq!(recovery(Some(&worked), false), Recovery::GoOn);
        // Waiting for its quota: its automatic resume takes it up, unless its turn was cut.
        let waits = AgentMeta {
            resume_at: Some(1),
            ..worked.clone()
        };
        assert_eq!(recovery(Some(&waits), false), Recovery::Wait);
        assert_eq!(recovery(Some(&waits), true), Recovery::GoOn);
        // Stopped mid-start: no agent left, or one that never got its first message.
        assert_eq!(recovery(None, false), Recovery::Again);
        let never = AgentMeta::default();
        assert_eq!(recovery(Some(&never), true), Recovery::Again);
        assert_eq!(recovery(Some(&never), false), Recovery::Again);
    }

    #[test]
    fn messages_name_the_ticket_and_the_loop() {
        let mut t = ticket(2, 5);
        t.description = "Contexte : l'API publique.".into();
        assert_eq!(
            first_message(Lang::Fr, &t),
            "Ticket ATL-42 : Limiter les tentatives\n\nContexte : l'API publique.\n\nCritères d'acceptation :\n1. critère 1\n2. critère 2\n\nBoucle 1/5. Travaille jusqu'à atteindre tous les critères, puis termine par le bilan."
        );
        assert_eq!(
            resume_message(Lang::Fr, "ATL-42"),
            "Reprends le ticket ATL-42 là où tu en étais, puis termine par le bilan."
        );
        assert_eq!(
            restart_message(Lang::Fr, "ATL-42"),
            "L'app a redémarré pendant ton travail sur ATL-42 : reprends là où tu en étais, puis termine par le bilan."
        );
        assert_eq!(
            reject_message(Lang::Fr, "ATL-42", " le bouton est mal placé. "),
            "Retour de test sur ATL-42 : le bouton est mal placé. Corrige, revérifie tous les critères, puis termine par le bilan."
        );
        assert_eq!(
            tests_failed_message(Lang::Fr, "npm test", "1 failed"),
            "Les tests (`npm test`) échouent :\n\n```\n1 failed\n```\n\nCorrige, revérifie les critères, puis termine par le bilan."
        );
        assert_eq!(
            conflict_message(Lang::Fr, "main", &["src/a.ts".into(), "src/b.ts".into()]),
            "Le merge de main dans ta branche a des conflits sur : src/a.ts, src/b.ts. Résous-les, commite le merge, revérifie les critères, puis termine par le bilan."
        );
        assert_eq!(
            conflict_message(Lang::Fr, "main", &[]),
            "J'ai mergé main dans ta branche, sans conflit. Revérifie les critères, puis termine par le bilan."
        );
        assert_eq!(
            rebase_message(Lang::Fr, "main"),
            "Rebase ta branche sur main et résous ses conflits, puis revérifie les critères et termine par le bilan."
        );
    }

    /// `ticket`, written in English (what an English text is checked against has no French).
    fn ticket_en(n: usize, max: u32) -> Ticket {
        Ticket {
            title: "Limit the retries".into(),
            criteria: (1..=n)
                .map(|i| Criterion {
                    text: format!("criterion {i}"),
                    ..Default::default()
                })
                .collect(),
            ..ticket(n, max)
        }
    }

    #[test]
    fn what_the_agents_are_told_reads_in_english() {
        use crate::i18n::check::french_in;
        use Lang::En;
        let mut t = ticket_en(2, 5);
        t.description = "Context: the public API.".into();
        let sent = [
            (
                reminder(En),
                "End with the criteria report (escouade block).",
            ),
            (
                first_message(En, &t),
                "Ticket ATL-42: Limit the retries\n\nContext: the public API.\n\nAcceptance criteria:\n1. criterion 1\n2. criterion 2\n\nLoop 1/5. Work until every criterion is met, then end with the report.",
            ),
            (
                resume_message(En, "ATL-42"),
                "Pick up ticket ATL-42 where you left off, then end with the report.",
            ),
            (
                restart_message(En, "ATL-42"),
                "The app restarted while you were working on ATL-42: pick up where you left off, then end with the report.",
            ),
            (
                reject_message(En, "ATL-42", " the button is misplaced. "),
                "Test feedback on ATL-42: the button is misplaced. Fix it, check every criterion again, then end with the report.",
            ),
            (
                tests_failed_message(En, "npm test", "1 failed"),
                "The tests (`npm test`) fail:\n\n```\n1 failed\n```\n\nFix them, check the criteria again, then end with the report.",
            ),
            (
                conflict_message(En, "main", &["src/a.ts".into(), "src/b.ts".into()]),
                "The merge of main into your branch has conflicts in: src/a.ts, src/b.ts. Resolve them, commit the merge, check the criteria again, then end with the report.",
            ),
            (
                conflict_message(En, "main", &[]),
                "I merged main into your branch, without conflicts. Check the criteria again, then end with the report.",
            ),
            (
                rebase_message(En, "main"),
                "Rebase your branch onto main and resolve its conflicts, then check the criteria again and end with the report.",
            ),
            (
                with_setup_failure(
                    En,
                    "Go.".into(),
                    Some("The worktree setup failed on `npm ci` (code 1)."),
                ),
                "Go.\n\nThe worktree setup failed on `npm ci` (code 1). Do what it takes to be able to work and test, then go on.",
            ),
        ];
        for (text, expected) in &sent {
            assert_eq!(text, expected);
            assert_eq!(french_in(text), None, "{text}");
        }
        // A loop asked for by the end of a turn, and the report asked for again, in English.
        let mut t = ticket_en(2, 5);
        let (end, r) = finished(
            r#"{"criteres": [{"n": 1, "ok": true}, {"n": 2, "ok": false, "note": "what is missing"}]}"#,
        );
        assert_eq!(
            turn_end(En, &mut t, &end, r.as_ref(), 1).send.as_deref(),
            Some("Loop 2/5. Criteria not met: 2 (what is missing). Keep going until they are met, then end with the report.")
        );
        let end = TurnEnd::Finished("Done.".into());
        assert_eq!(turn_end(En, &mut t, &end, None, 2).send, Some(reminder(En)));
    }

    #[test]
    fn the_english_protocol_and_launch_request_ask_for_the_markers_the_board_reads_back() {
        use crate::i18n::check::{french_in, json_keys};
        use Lang::{En, Fr};
        let t = ticket_en(2, 5);
        for (ports, isola) in [(None, false), (Some(4120), false), (None, true)] {
            let en = protocol_prompt_for(En, &t, ports, isola);
            assert_eq!(french_in(&en), None, "{en}");
            assert!(!en.contains('\n') && en.contains("```escouade"), "{en}");
            assert!(
                en.contains("Escouade ticket ATL-42 “Limit the retries”")
                    && en.contains("Acceptance criteria (2): 1) criterion 1; 2) criterion 2."),
                "{en}"
            );
            // The same keys as the French protocol asks for: the board reads them back.
            assert_eq!(
                json_keys(&en),
                json_keys(&protocol_prompt_for(Fr, &t, ports, isola)),
                "{en}"
            );
            assert!(escaped_len(&en) <= PROTOCOL_BUDGET);
        }
        let ports = protocol_prompt_for(En, &t, Some(4120), false);
        assert!(
            ports.contains("Ports reserved for this worktree: 4120 to 4129")
                && ports.contains("http://localhost:4121"),
            "{ports}"
        );
        let m = prepare_message(En, 4120);
        assert_eq!(french_in(&m), None, "{m}");
        assert!(
            m.starts_with("Prepare the test launch of this worktree. Ports reserved: 4120 to 4129"),
            "{m}"
        );
        assert_eq!(json_keys(&m), json_keys(&prepare_message(Fr, 4120)));
        // Its example is a recipe the app keeps, as the French one.
        let recipe = parse_report(&m).and_then(|r| r.recipe).expect("{m}");
        assert_eq!(
            (recipe.processes[0].url.as_str(), recipe.prepare.len()),
            ("http://localhost:4121", 1)
        );
    }

    #[test]
    fn a_commit_message_and_a_pull_request_are_asked_for_and_written_in_english() {
        use crate::i18n::check::french_in;
        use Lang::En;
        let mut t = ticket_en(2, 5);
        assert_eq!(french_in(&commit_system(En)), None);
        t.progress = vec!["Signed tokens".into(), "Middleware rewritten".into()];
        let p = commit_prompt(En, &t, " src/a.ts | 3 ++-");
        assert_eq!(french_in(&p), None, "{p}");
        assert!(
            p.contains("“type(optional scope): short lowercase description” followed by “[ATL-42]”. Write it in English.")
                && p.contains("[ATL-42]")
                && p.contains("<ticket>\nATL-42 · Limit the retries")
                && p.contains("Progress:\n- Signed tokens\n- Middleware rewritten\n")
                && p.contains("<diffstat>\nsrc/a.ts | 3 ++-\n</diffstat>"),
            "{p}"
        );
        t.description = "Context".into();
        t.criteria[0].ok = true;
        assert_eq!(
            pr_body(En, &t),
            "Context\n\nWhat was done:\n- Signed tokens\n- Middleware rewritten\n\nCriteria:\n- [x] criterion 1\n- [ ] criterion 2"
        );
    }

    #[test]
    fn the_launch_request_gives_the_ports_and_the_form_of_the_recipe() {
        let m = prepare_message(Lang::Fr, 4120);
        assert!(
            m.starts_with(
                "Prépare le lancement de test de ce worktree. Ports réservés : 4120 à 4129"
            ),
            "{m}"
        );
        assert!(
            m.contains("```escouade\n{\"lancement\"") && m.contains("http://localhost:4121"),
            "{m}"
        );
        assert!(
            m.contains("« ouvrir » est l'adresse qui montre directement"),
            "{m}"
        );
        // The example is a recipe the app would keep, were the agent to give it as it is.
        let recipe = parse_report(&m).and_then(|r| r.recipe).expect("{m}");
        assert_eq!(
            (recipe.processes[0].url.as_str(), recipe.prepare.len()),
            ("http://localhost:4121", 1)
        );
        // A block read back from a damaged file must not panic on its last port.
        assert!(prepare_message(Lang::Fr, u16::MAX).contains("65535 à 65535"));
    }

    #[test]
    fn the_protocol_holds_on_one_line_with_the_criteria_and_the_ports() {
        let mut t = ticket(2, 5);
        t.title = "Gérer\nles « % & \" »".into();
        let p = protocol_prompt(&t, Some(4120));
        assert!(!p.contains('\n') && !p.contains('\r'), "{p}");
        assert!(
            p.contains("le ticket ATL-42 « Gérer les « % & \" » »"),
            "{p}"
        );
        assert!(
            p.contains("Critères d'acceptation (2) : 1) critère 1 ; 2) critère 2."),
            "{p}"
        );
        assert!(p.contains("```escouade"));
        assert!(p.contains("4120 à 4129") && p.contains("http://localhost:4121"));
        assert!(!protocol_prompt(&t, None).contains("Ports réservés"));
        // Hundreds of long criteria stay well under what a command line takes.
        t.criteria = (0..300)
            .map(|i| Criterion {
                text: format!("{i} {}", "x".repeat(300)),
                ..Default::default()
            })
            .collect();
        assert!(protocol_prompt(&t, Some(4100)).len() < 7000);
    }

    #[test]
    fn with_isola_the_protocol_asks_for_the_address_to_open_and_reserves_no_port() {
        let mut t = ticket(2, 5);
        let p = protocol_prompt_for(Lang::Fr, &t, None, true);
        assert!(!p.contains('\n') && !p.contains("Ports réservés"), "{p}");
        assert!(
            p.contains("isola up") && p.contains("isola ls --json"),
            "{p}"
        );
        assert!(p.contains("\"lancement\": {\"ouvrir\": "), "{p}");
        assert!(p.contains("Critères d'acceptation (2) : 1) critère 1 ; 2) critère 2."));
        // Without isola, the same as before.
        assert_eq!(
            protocol_prompt_for(Lang::Fr, &t, Some(4120), false),
            protocol_prompt(&t, Some(4120))
        );
        // The criteria still give way to the budget.
        t.criteria = (0..300)
            .map(|i| Criterion {
                text: format!("{i} {}", "%".repeat(300)),
                ..Default::default()
            })
            .collect();
        let p = protocol_prompt_for(Lang::Fr, &t, None, true);
        assert!(escaped_len(&p) <= PROTOCOL_BUDGET && p.contains(more(Lang::Fr).trim()));
    }

    #[test]
    fn a_setup_that_failed_is_told_after_the_first_message() {
        let t = ticket(1, 5);
        assert_eq!(
            with_setup_failure(Lang::Fr, first_message(Lang::Fr, &t), None),
            first_message(Lang::Fr, &t)
        );
        let m = with_setup_failure(
            Lang::Fr,
            first_message(Lang::Fr, &t),
            Some("La préparation du worktree a échoué sur `npm ci` (code 1)."),
        );
        assert!(m.starts_with(&first_message(Lang::Fr, &t)), "{m}");
        assert!(
            m.ends_with("La préparation du worktree a échoué sur `npm ci` (code 1). Fais le nécessaire pour pouvoir travailler et tester, puis continue."),
            "{m}"
        );
    }

    /// What a `.cmd` argument weighs once Rust has escaped it for cmd.exe, as std's
    /// `append_bat_arg` does (the surrounding quotes left aside): `%` becomes a longer sequence
    /// (8 characters), `"` is doubled and the backslashes just before it are doubled too;
    /// anything else counts its UTF-8 bytes (at least its UTF-16 units). Deliberately not
    /// `escaped_len`: this is the oracle that one is checked against.
    fn escaped_weight(s: &str) -> usize {
        let (mut weight, mut backslashes) = (0, 0);
        for c in s.chars() {
            if c == '\\' {
                backslashes += 1;
            } else {
                if c == '"' {
                    // The backslashes again, then the quote that escapes this one.
                    weight += backslashes + 1;
                } else if c == '%' || c == '\r' {
                    weight += 7;
                }
                backslashes = 0;
            }
            weight += c.len_utf8();
        }
        // Trailing backslashes are doubled before the closing quote.
        weight + backslashes
    }

    #[test]
    fn the_protocol_stays_under_cmds_command_line_once_escaped() {
        // The worst case: a title and criteria made of what a `.cmd` argument escapes the most,
        // long enough to be cut.
        let nasty = "%\"".repeat(150);
        let mut t = ticket(0, 5);
        t.title = nasty.clone();
        t.criteria = (0..50)
            .map(|_| Criterion {
                text: nasty.clone(),
                ..Default::default()
            })
            .collect();
        // In each language the protocol can be written in.
        let langs = [
            (Lang::Fr, "Critères d'acceptation (50) : 1) ", " ; …"),
            (Lang::En, "Acceptance criteria (50): 1) ", "; …"),
        ];
        for (lang, count, more) in langs {
            for ports in [None, Some(4100)] {
                let p = protocol_prompt_for(lang, &t, ports, false);
                assert!(!p.contains('\n') && !p.contains('\r'));
                // cmd.exe takes 8191 characters for the whole command line; the protocol keeps
                // to 6000 of them and leaves the rest to the other arguments.
                let weight = escaped_weight(&p);
                assert!(weight <= 6000, "{lang:?} {ports:?}: {weight}");
                // The criteria are cut, not dropped: the first is there, the count is the real one.
                assert!(p.contains(count), "{lang:?} {ports:?}");
                assert!(p.contains(more), "{lang:?} {ports:?}");
            }
        }
        // Long accented text weighs its bytes, never less.
        t.title = "é".repeat(300);
        t.criteria = (0..300)
            .map(|_| Criterion {
                text: "é".repeat(300),
                ..Default::default()
            })
            .collect();
        for (lang, _, _) in langs {
            assert!(escaped_weight(&protocol_prompt_for(lang, &t, Some(4100), false)) <= 6000);
        }
        // A short list is kept whole.
        let mut t = ticket(30, 5);
        t.title = "Court".into();
        let p = protocol_prompt(&t, Some(4100));
        assert!(p.contains("30) critère 30."), "{p}");
    }

    #[test]
    fn backslashes_before_a_quote_count_double_in_the_protocols_budget() {
        // std doubles the backslashes before a quote on top of the quote itself: `\"` weighs 4.
        let nasty = "\\\"".repeat(100);
        let mut t = ticket(0, 5);
        t.title = nasty.clone();
        t.criteria = (0..50)
            .map(|_| Criterion {
                text: nasty.clone(),
                ..Default::default()
            })
            .collect();
        for ports in [None, Some(4100)] {
            let p = protocol_prompt(&t, ports);
            let weight = escaped_weight(&p);
            assert!(weight <= 6000, "{ports:?}: {weight}");
            assert!(p.contains("Critères d'acceptation (50) : 1) "), "{ports:?}");
            assert!(p.contains(" ; …"), "{ports:?}");
            let en = protocol_prompt_for(Lang::En, &t, ports, false);
            assert!(escaped_weight(&en) <= 6000, "{ports:?}");
            assert!(en.contains("Acceptance criteria (50): 1) ") && en.contains("; …"));
        }
        // Backslashes alone are never doubled by std unless a quote follows; the bound is safe.
        t.criteria = (0..50)
            .map(|_| Criterion {
                text: "\\".repeat(200),
                ..Default::default()
            })
            .collect();
        assert!(escaped_weight(&protocol_prompt(&t, Some(4100))) <= 6000);
        assert!(escaped_weight(&protocol_prompt_for(Lang::En, &t, Some(4100), false)) <= 6000);
    }

    #[test]
    fn a_control_character_never_reaches_the_command_line() {
        let mut t = ticket(1, 5);
        t.title = "Gé\0rer\u{7} les\ttests\u{1b}[0m".into();
        t.criteria[0].text = "cri\0tère\u{7}\u{8}\n1".into();
        let p = protocol_prompt(&t, Some(4100));
        assert!(p.contains("le ticket ATL-42 « Gérer les tests[0m »"), "{p}");
        assert!(p.contains("1) critère 1."), "{p}");
        assert!(p.chars().all(|c| !c.is_control()), "{p:?}");
    }

    #[test]
    fn a_reminder_is_owed_again_once_the_user_resumes_after_a_stop_or_an_error() {
        for stop in [TurnEnd::Interrupted, TurnEnd::Error("boom".into())] {
            let mut t = ticket(1, 5);
            let end = TurnEnd::Finished("Fini.".into());
            let next = turn_end(Lang::Fr, &mut t, &end, None, 1);
            assert_eq!(next.send.as_deref(), Some(reminder(Lang::Fr).as_str()));
            // The reminder's own turn is stopped…
            assert!(turn_end(Lang::Fr, &mut t, &stop, None, 2).blocked);
            // …then the user resumes: the next turn without a report is asked again, not blocked.
            let next = turn_end(Lang::Fr, &mut t, &end, None, 3);
            assert_eq!(
                next.send.as_deref(),
                Some(reminder(Lang::Fr).as_str()),
                "{stop:?}"
            );
            assert_eq!((t.blocked.clone(), next.blocked), (None, false), "{stop:?}");
        }
    }

    #[test]
    fn a_criterion_number_may_be_a_string_but_never_zero() {
        let r = parse_report(&block(
            r#"{"criteres": [{"n": "2", "ok": true, "note": "vu"}, {"n": 0, "ok": true}, {"n": "0", "ok": true}, {"n": " 3 ", "ok": false}, {"n": "x"}, {"n": -1}, {"ok": true}, "4"]}"#,
        ))
        .unwrap();
        assert_eq!(
            r.criteria,
            Some(vec![(2, true, "vu".to_string()), (3, false, String::new())])
        );
    }

    #[test]
    fn a_list_of_criteria_without_any_valid_number_is_no_report_so_it_is_asked_for_again() {
        for json in [
            r#"{"criteres": [{"n": 0, "ok": true}]}"#,
            r#"{"criteres": [{"ok": true}, {"n": "x", "ok": true}]}"#,
            r#"{"criteres": ["un", 2]}"#,
        ] {
            assert_eq!(parse_report(&block(json)).unwrap().criteria, None, "{json}");
            let mut t = ticket(1, 5);
            let (end, r) = finished(json);
            let next = turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 1);
            // The reminder, not a loop spent.
            assert_eq!(
                next.send.as_deref(),
                Some(reminder(Lang::Fr).as_str()),
                "{json}"
            );
            assert_eq!((t.iteration, t.reminded), (1, true), "{json}");
        }
        // An empty list is still a (empty) list.
        assert_eq!(
            parse_report(&block(r#"{"criteres": []}"#))
                .unwrap()
                .criteria,
            Some(vec![])
        );
    }

    #[test]
    fn a_null_in_the_recipe_is_an_absent_value_not_an_unreadable_report() {
        let r = parse_report(&block(
            r#"{"criteres": [{"n": 1, "ok": true}], "lancement": {"preparation": null, "processus": [{"nom": null, "commande": "npm run dev", "dossier": null, "url": null}], "ouvrir": null}}"#,
        ))
        .unwrap();
        assert_eq!(r.criteria, Some(vec![(1, true, String::new())]));
        let recipe = r.recipe.unwrap();
        assert_eq!(recipe.processes[0].command, "npm run dev");
        assert!(recipe.processes[0].url.is_empty() && recipe.open.is_empty());
    }

    fn progress_of(json: &str) -> Option<Vec<String>> {
        parse_report(&block(json)).unwrap().progress
    }

    #[test]
    fn the_progress_is_a_list_of_short_lines_read_from_avancement_or_progress() {
        let want = Some(vec!["A".to_string(), "B".into(), "C suite".into()]);
        assert_eq!(
            progress_of(r#"{"criteres": [], "avancement": ["A", "  B  ", "", 3, "C\nsuite"]}"#),
            want
        );
        // The English key is understood too.
        assert_eq!(
            progress_of(r#"{"progress": ["A", "  B  ", "", 3, "C\nsuite"]}"#),
            want
        );
        // No key, an empty list, or one without a usable item: nothing said, so no list.
        assert_eq!(progress_of(r#"{"criteres": [{"n": 1, "ok": true}]}"#), None);
        assert_eq!(progress_of("{}"), None);
        assert_eq!(progress_of(r#"{"avancement": []}"#), None);
        assert_eq!(progress_of(r#"{"progress": []}"#), None);
        for junk in [
            r#"["", 1, null, " \t "]"#,
            r#"[3, true, {"a": "b"}, ["x"]]"#,
            r#"["\u0000\u001b", "\n"]"#,
        ] {
            let json = format!(r#"{{"avancement": {junk}}}"#);
            assert_eq!(progress_of(&json), None, "{junk}");
        }
        // Whatever the whitespace or the control characters, an item is one clean line; one
        // with nothing left is dropped.
        assert_eq!(
            progress_of(
                r#"{"avancement": ["a\u0000b\u0007", "\u0000\u001b", " \t ", "x\r\n  y\tz"]}"#
            ),
            Some(vec!["ab".to_string(), "x y z".into()])
        );
    }

    #[test]
    fn at_most_eight_items_of_at_most_120_bytes_are_kept() {
        let items: Vec<String> = (1..=10).map(|i| format!("fonction {i}")).collect();
        let json = serde_json::json!({ "avancement": items }).to_string();
        assert_eq!(progress_of(&json).unwrap(), items[..8]);
        // The unusable ones do not take a place.
        let json = r#"{"avancement": ["", 1, null, "a", "b", "c", "d", "e", "f", "g", "h", "i"]}"#;
        assert_eq!(
            progress_of(json).unwrap(),
            ["a", "b", "c", "d", "e", "f", "g", "h"]
        );
        // A long item is cut like elsewhere: at 120 bytes, with the ellipsis.
        let json = serde_json::json!({ "avancement": ["x".repeat(300)] }).to_string();
        let long = &progress_of(&json).unwrap()[0];
        assert_eq!(*long, format!("{}…", "x".repeat(120)));
        // On a character boundary: "é" is two bytes, 60 of them fill the 120.
        let json = serde_json::json!({ "avancement": ["é".repeat(300), "ab".to_string() + &"é".repeat(100)] })
            .to_string();
        let items = progress_of(&json).unwrap();
        assert_eq!(items[0], format!("{}…", "é".repeat(60)));
        assert_eq!(items[1], format!("ab{}…", "é".repeat(59)));
        // An item of exactly 120 bytes is whole.
        let json = serde_json::json!({ "avancement": ["y".repeat(120)] }).to_string();
        assert_eq!(progress_of(&json).unwrap()[0], "y".repeat(120));
    }

    #[test]
    fn a_progress_that_is_not_a_list_is_none_and_costs_nothing_else() {
        for bad in [r#""texte""#, "null", "42", r#"{"a": "b"}"#, "true"] {
            let json = format!(
                r#"{{"criteres": [{{"n": 1, "ok": true, "note": "vu"}}], "avancement": {bad}, "lancement": {{"processus": [{{"nom": "web", "commande": "npm run dev"}}]}}}}"#
            );
            let r = parse_report(&block(&json)).unwrap();
            assert_eq!(r.progress, None, "{bad}");
            assert_eq!(r.criteria, Some(vec![(1, true, "vu".to_string())]), "{bad}");
            assert_eq!(
                r.recipe.unwrap().processes[0].command,
                "npm run dev",
                "{bad}"
            );
        }
        // And the other way: a list does not need the criteria or the recipe.
        let r = parse_report(&block(r#"{"avancement": ["a"]}"#)).unwrap();
        assert_eq!(
            (r.progress, r.criteria, r.recipe),
            (Some(vec!["a".to_string()]), None, None)
        );
    }

    #[test]
    fn the_last_progress_replaces_the_previous_and_a_report_without_one_keeps_it() {
        // 8 loops: enough for the turns below to keep looping until the one that completes.
        let mut t = ticket(1, 8);
        t.progress = vec!["ancien".into()];
        let (end, r) =
            finished(r#"{"criteres": [{"n": 1, "ok": false}], "avancement": ["x", "y"]}"#);
        turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 1);
        assert_eq!(t.progress, ["x", "y"]);
        // A report without `avancement` leaves it as it was (a loop, here).
        let (end, r) = finished(r#"{"criteres": [{"n": 1, "ok": false}]}"#);
        let next = turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 2);
        assert!(next.send.is_some());
        assert_eq!(t.progress, ["x", "y"]);
        // So does one whose `avancement` is not a list, is empty, or has nothing usable in it.
        for bad in [r#""fait""#, "[]", r#"["", 1, null]"#] {
            let json = format!(r#"{{"criteres": [{{"n": 1, "ok": false}}], "avancement": {bad}}}"#);
            let (end, r) = finished(&json);
            turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 3);
            assert_eq!(t.progress, ["x", "y"], "{bad}");
        }
        // The last one wins, on the turn that sends the ticket to review too.
        let (end, r) = finished(r#"{"criteres": [{"n": 1, "ok": true}], "avancement": ["z"]}"#);
        turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 4);
        assert_eq!(
            (t.column, t.progress.clone()),
            (Column::Review, vec!["z".to_string()])
        );
        // Neither a ticket no longer "En cours" nor an end without a report touches it.
        let (end, r) = finished(r#"{"criteres": [{"n": 1, "ok": true}], "avancement": ["autre"]}"#);
        turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 5);
        assert_eq!(t.progress, ["z"]);
        let mut t = ticket(1, 5);
        t.progress = vec!["a".into()];
        for end in [
            TurnEnd::Interrupted,
            TurnEnd::Limited,
            TurnEnd::Error("boom".into()),
            TurnEnd::Finished("Fini.".into()),
        ] {
            turn_end(Lang::Fr, &mut t, &end, None, 6);
            assert_eq!(t.progress, ["a"], "{end:?}");
        }
    }

    #[test]
    fn a_report_with_a_progress_but_no_criteria_still_gives_it_and_is_asked_for_again() {
        let mut t = ticket(1, 5);
        let (end, r) = finished(r#"{"avancement": ["a", "b"]}"#);
        let next = turn_end(Lang::Fr, &mut t, &end, r.as_ref(), 1);
        assert_eq!(next.send.as_deref(), Some(reminder(Lang::Fr).as_str()));
        assert_eq!((t.iteration, t.reminded), (1, true));
        assert_eq!(t.progress, ["a", "b"]);
    }

    #[test]
    fn the_protocol_asks_for_a_short_progress_list_with_the_report() {
        for ports in [None, Some(4100)] {
            let p = protocol_prompt(&ticket(2, 5), ports);
            assert!(!p.contains('\n') && !p.contains('\r'), "{p}");
            assert!(
                p.contains(
                    "Ajoute au bilan « avancement » : la liste succincte (3 à 8 éléments courts) \
                     des fonctionnalités en place jusque-là."
                ),
                "{ports:?}: {p}"
            );
            // The key is in the example of the block, next to the criteria, with three items…
            let example = p.find("{\"criteres\": [").expect("the example");
            let key = p[example..]
                .find("\"avancement\": [\"")
                .expect("the key in the example");
            assert!(
                p[example..][..key].contains("\"ok\": false"),
                "{ports:?}: {p}"
            );
            let items = &p[example + key..];
            let items = &items[..items.find("]}").expect("the end of the example")];
            assert_eq!(items.matches("\", \"").count(), 2, "{ports:?}: {items}");
            // …and "one element per criterion" is said of the criteria, not of the whole block.
            assert!(
                p.contains("un élément par critère dans \"criteres\" (n à partir de 1)"),
                "{ports:?}: {p}"
            );
            assert!(escaped_len(&p) <= PROTOCOL_BUDGET, "{ports:?}");
        }
    }

    #[test]
    fn a_generated_commit_message_is_kept_only_in_the_conventional_form_with_the_key() {
        let ok = |s: &str| commit_from_answer(s, "ATL-42");
        assert_eq!(
            ok("feat(auth): limiter les tentatives [ATL-42]").as_deref(),
            Some("feat(auth): limiter les tentatives [ATL-42]")
        );
        assert_eq!(
            ok("`fix: corrige le 429 [ATL-42]`\n").as_deref(),
            Some("fix: corrige le 429 [ATL-42]")
        );
        assert_eq!(
            ok("refactor!: nouvelle API [ATL-42]").as_deref(),
            Some("refactor!: nouvelle API [ATL-42]")
        );
        // Quoted, or in a fenced block: the message inside is still used.
        for quoted in [
            "\"feat: limiter les tentatives [ATL-42]\"",
            "'feat: limiter les tentatives [ATL-42]'",
            "« feat: limiter les tentatives [ATL-42] »",
            "```\nfeat: limiter les tentatives [ATL-42]\n```",
            "```text\n\"feat: limiter les tentatives [ATL-42]\"\n```\n",
            "\n  `« feat: limiter les tentatives [ATL-42] »`  \n",
        ] {
            assert_eq!(
                ok(quoted).as_deref(),
                Some("feat: limiter les tentatives [ATL-42]"),
                "{quoted}"
            );
        }
        let long = format!("feat: {} [ATL-42]", "x".repeat(120));
        for bad in [
            "```\n```",
            "\"feat: guillemet ouvert [ATL-42]",
            "feat: sans clé",
            "Voici le message : feat: x [ATL-42]",
            "wip: x [ATL-42]",
            "feat(): x [ATL-42]",
            "feat: [ATL-42]",
            long.as_str(),
        ] {
            assert_eq!(ok(bad), None, "{bad}");
        }
    }

    #[test]
    fn fallback_and_plain_commit_messages_and_the_prompt() {
        let t = ticket(1, 5);
        assert_eq!(fallback_commit(&t), "feat: Limiter les tentatives [ATL-42]");
        assert_eq!(plain_commit(&t), "ATL-42 Limiter les tentatives");
        let p = commit_prompt(Lang::Fr, &t, " src/a.ts | 3 ++-");
        assert!(
            p.contains("<ticket>\nATL-42 · Limiter les tentatives"),
            "{p}"
        );
        assert!(
            p.contains("src/a.ts | 3 ++-") && p.contains("[ATL-42]"),
            "{p}"
        );
        // What the agent says is in place goes with the ticket.
        let mut t = t;
        t.progress = vec!["Tokens signés".into(), "Middleware réécrit".into()];
        let p = commit_prompt(Lang::Fr, &t, "");
        let ticket = &p[p.find("<ticket>").unwrap()..p.find("</ticket>").unwrap()];
        assert!(
            ticket.contains("Avancement :\n- Tokens signés\n- Middleware réécrit\n"),
            "{p}"
        );
        assert!(!commit_prompt(Lang::Fr, &ticket_of_progress(&[]), "").contains("Avancement"));
        // The language is asked for apart from the format, which Haiku could echo with it.
        assert!(
            p.contains("« type(portée facultative): description courte en minuscules » suivie de « [ATL-42] ». Écris-la en français."),
            "{p}"
        );
    }

    fn ticket_of_progress(items: &[&str]) -> Ticket {
        let mut t = ticket(1, 5);
        t.progress = items.iter().map(|s| s.to_string()).collect();
        t
    }

    #[test]
    fn the_protocol_tells_not_to_commit_the_files_copied_from_the_project() {
        for ports in [None, Some(4100)] {
            let p = protocol_prompt(&ticket(2, 5), ports);
            assert!(
                p.contains("Ne commite pas les fichiers copiés du projet (.env…)."),
                "{p}"
            );
            assert!(!p.contains('\n') && escaped_len(&p) <= PROTOCOL_BUDGET);
        }
    }

    #[test]
    fn a_copied_file_in_the_branch_is_told_apart_from_one_only_in_its_history() {
        let s = |l: &[&str]| l.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(copied_refusal(Lang::Fr, &s(&[]), &s(&[])), None);
        assert_eq!(
            copied_refusal(Lang::Fr, &s(&[".env"]), &s(&[".env"])).as_deref(),
            Some(".env copié du projet est commité dans la branche")
        );
        assert_eq!(
            copied_refusal(Lang::Fr, &s(&[".env", "web/.env"]), &s(&[".env.local"])).as_deref(),
            Some(".env, web/.env copiés du projet sont commités dans la branche")
        );
        assert_eq!(
            copied_refusal(Lang::Fr, &s(&[]), &s(&[".env"])).as_deref(),
            Some(".env copié du projet est dans l'historique de la branche")
        );
        assert_eq!(
            copied_refusal(Lang::Fr, &s(&[]), &s(&[".env", ".env.local"])).as_deref(),
            Some(".env, .env.local copiés du projet sont dans l'historique de la branche")
        );
    }

    #[test]
    fn says_in_english_why_a_board_or_a_ticket_stops_and_what_became_of_a_ticket() {
        use crate::i18n::Lang::En;
        let s = |l: &[&str]| l.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            default_criteria(En),
            ["Implementation matches the ticket", "Tests pass"]
        );
        // What the board says when no ticket starts (`BoardIssue`).
        assert_eq!(
            target_issue(En, "main", false, true).as_deref(),
            Some("main has no commit yet — no tickets will start")
        );
        assert_eq!(
            target_issue(En, "release", false, false).as_deref(),
            Some("Target branch release not found — no tickets will start")
        );
        assert_eq!(
            target_issue(En, "", false, false).as_deref(),
            Some(
                "The project isn’t on any branch: choose the target branch in the Kanban settings."
            )
        );
        // Why a ticket is blocked.
        assert_eq!(
            [
                interrupted(En),
                error_blocked(En, "boom"),
                report_missing(En)
            ],
            ["Interrupted", "Error: boom", "Criteria report missing"]
        );
        let list = vec![
            Ticket {
                key: "DEM-3".into(),
                after: s(&["t5"]),
                ..todo("t3", 1)
            },
            Ticket {
                key: "DEM-5".into(),
                ..todo("t5", 1)
            },
        ];
        assert_eq!(
            cycle_refusal(En, &list, "t5", &s(&["t3"])).as_deref(),
            Some("DEM-3 already waits for DEM-5 (directly or not).")
        );
        assert_eq!(
            [
                copied_refusal(En, &s(&[".env"]), &s(&[])),
                copied_refusal(En, &s(&[".env", "web/.env"]), &s(&[])),
                copied_refusal(En, &s(&[]), &s(&[".env"])),
                copied_refusal(En, &s(&[]), &s(&[".env", ".env.local"])),
            ]
            .map(Option::unwrap_or_default),
            [
                ".env, copied from the project, is committed in the branch",
                ".env, web/.env, copied from the project, are committed in the branch",
                ".env, copied from the project, is in the branch’s history",
                ".env, .env.local, copied from the project, are in the branch’s history",
            ]
        );
        // What became of a ticket approved.
        assert_eq!(
            [
                merged_outcome(En, "main", "squash"),
                pushed_for_pr_outcome(En, "ticket/a"),
                pushed_elsewhere_outcome(En, "ticket/a"),
                pushed_outcome(En, "ticket/a"),
                kept_outcome(En),
                nothing_outcome(En),
                worktree_kept(En),
            ],
            [
                "⤵ Merged into main · squash",
                "⇡ ticket/a pushed · PR to finish",
                "⇡ ticket/a pushed · PR to open on your host",
                "⇡ Pushed to ticket/a",
                "◇ Left in the worktree",
                "∅ No changes",
                " · worktree kept",
            ]
        );
    }

    #[test]
    fn a_resume_time_reads_as_the_local_hour_and_minute() {
        use chrono::TimeZone;
        let at = chrono::Local
            .with_ymd_and_hms(2026, 10, 3, 15, 7, 0)
            .unwrap()
            .timestamp_millis();
        assert_eq!(clock(at), "15:07");
        let at = chrono::Local
            .with_ymd_and_hms(2026, 10, 3, 9, 0, 59)
            .unwrap()
            .timestamp_millis();
        assert_eq!(clock(at), "09:00");
    }

    #[test]
    fn outcomes_say_where_the_work_went() {
        assert_eq!(
            merged_outcome(Lang::Fr, "main", "squash"),
            "⤵ Mergé dans main · squash"
        );
        assert_eq!(
            merged_outcome(Lang::Fr, "release", "merge"),
            "⤵ Mergé dans release · merge commit"
        );
        assert_eq!(
            merged_outcome(Lang::Fr, "main", "rebase"),
            "⤵ Mergé dans main · rebase"
        );
        assert_eq!(pr_outcome(12, "main"), "⇡ PR #12 → main");
        assert_eq!(
            pushed_for_pr_outcome(Lang::Fr, "ticket/atl-42"),
            "⇡ ticket/atl-42 poussée · PR à finaliser"
        );
        assert_eq!(
            pushed_elsewhere_outcome(Lang::Fr, "ticket/atl-42"),
            "⇡ ticket/atl-42 poussée · PR à ouvrir sur ton hébergeur"
        );
        assert_eq!(
            pushed_outcome(Lang::Fr, "ticket/atl-42"),
            "⇡ Poussé sur ticket/atl-42"
        );
        assert_eq!(kept_outcome(Lang::Fr), "◇ Laissé dans le worktree");
    }

    #[test]
    fn github_repositories_are_recognized_over_https_and_ssh() {
        for url in [
            "https://github.com/acme/demo.git",
            "https://github.com/acme/demo",
            "git@github.com:acme/demo.git",
            "ssh://git@github.com/acme/demo.git",
            "https://x-token@github.com/acme/demo.git",
        ] {
            assert_eq!(
                github_repo(url),
                Some(("acme".into(), "demo".into())),
                "{url}"
            );
        }
        for url in [
            "https://gitlab.com/acme/demo.git",
            "C:/remotes/demo.git",
            "https://github.com/acme",
        ] {
            assert_eq!(github_repo(url), None, "{url}");
        }
    }

    #[test]
    fn the_compare_address_opens_a_pull_request_to_finalize() {
        assert_eq!(
            compare_url(
                "acme",
                "demo",
                "main",
                "ticket/atl-42",
                "feat: x & y [ATL-42]",
                "Ligne 1\nCritères"
            ),
            "https://github.com/acme/demo/compare/main...ticket/atl-42?expand=1&title=feat%3A%20x%20%26%20y%20%5BATL-42%5D&body=Ligne%201%0ACrit%C3%A8res"
        );
        assert_eq!(
            pr_number("Creating pull request…\nhttps://github.com/acme/demo/pull/12\n"),
            Some((12, "https://github.com/acme/demo/pull/12".to_string()))
        );
        assert_eq!(pr_number("rien"), None);
        let mut t = ticket(2, 5);
        t.description = "Contexte".into();
        t.criteria[0].ok = true;
        assert_eq!(
            pr_body(Lang::Fr, &t),
            "Contexte\n\nCritères :\n- [x] critère 1\n- [ ] critère 2"
        );
        t.description.clear();
        assert!(pr_body(Lang::Fr, &t).starts_with("Critères :"));
    }

    #[test]
    fn a_pull_requests_description_tells_what_was_done_before_the_criteria() {
        let mut t = ticket(2, 5);
        t.description = "Contexte".into();
        t.criteria[0].ok = true;
        t.progress = vec!["Tokens signés".into(), " Middleware réécrit ".into()];
        assert_eq!(
            pr_body(Lang::Fr, &t),
            "Contexte\n\nCe qui a été fait :\n- Tokens signés\n- Middleware réécrit\n\nCritères :\n- [x] critère 1\n- [ ] critère 2"
        );
        t.description.clear();
        assert!(pr_body(Lang::Fr, &t).starts_with("Ce qui a été fait :\n- Tokens signés\n"));
    }

    #[test]
    fn the_compare_address_stays_short_whatever_the_ticket_says() {
        let long = "é".repeat(20_000);
        let url = compare_url("acme", "demo", "main", "ticket/atl-42", &long, &long);
        assert!(url.len() < 6 * 1024, "{}", url.len());
        let (head, body) = url.split_once("&body=").unwrap();
        // Cut on a character, and said so.
        assert!(
            body.ends_with("%C3%A9%E2%80%A6"),
            "{}",
            &body[body.len() - 30..]
        );
        assert!(
            head.ends_with("%C3%A9%E2%80%A6"),
            "{}",
            &head[head.len() - 30..]
        );
        assert!(
            body.len() > 3 * 1024,
            "the body keeps a fair part: {}",
            body.len()
        );
    }

    #[test]
    fn port_blocks_skip_those_taken_and_those_with_a_busy_port() {
        assert_eq!(allocate_ports(&[], |_| true), Some(4100));
        assert_eq!(allocate_ports(&[4100, 4110], |_| true), Some(4120));
        // Another program holds 4115, then 4125: their whole blocks are skipped.
        assert_eq!(
            allocate_ports(&[4100], |p| p != 4115 && p != 4125),
            Some(4130)
        );
        assert_eq!(allocate_ports(&[], |_| false), None);
    }
}
