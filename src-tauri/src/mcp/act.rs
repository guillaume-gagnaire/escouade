//! What the tools that act do (`tools`): what the window does, through the same functions of the
//! core (so the window is told, the external tickets kept in step and the board scheduled as for
//! an action made there), under the guardrails of the window and the autopilot:
//!
//! - while the autopilot is paused (a quota window over the threshold, the pause after a usage
//!   limit, an agent waiting for its quota) nothing that spends quota is started: a ticket, an
//!   agent, a message. The refusal says until when;
//! - a project's most tickets in parallel (« En parallèle » in its Kanban settings) are not
//!   exceeded by a ticket launched, nor its most agents working at once by an agent created;
//! - a ticket that comes after tickets not done is launched by hand only once the user agreed in
//!   the window (a confirmation): here it is refused, and the refusal says which tickets;
//! - an agent never writes to nor stops itself, and acts on its own project alone (it reads the
//!   others: the tools that read are open);
//! - a message sent to an agent, and the first one of an agent created, reach it under its author's
//!   name (« Message de <author> : »): never as the user's own words.
//!
//! A refusal (`ToolError::Refused`) is a guard saying no; a failure (`ToolError::Failed`) is
//! something asked that does not fit (a name, an empty title…). `report_progress` and
//! `split_ticket` are for the agents of Escouade alone.

use super::activity;
use super::read::{json, ticket_view, todo_keys};
use super::resolve;
use super::tools::{
    CreateAgentArgs, CreateTicketArgs, MoveTicketArgs, PositionArg, ReportProgressArgs,
    SendMessageArgs, SplitTicketArgs, StartTicketArgs, StopAgentArgs, SubTicket, ToolError,
    UpdateTicketArgs, SPLIT_MAX,
};
use super::Caller;
use crate::board::MoveTo;
use crate::core::{AgentOptions, Core};
use crate::model::{Column, Project, PROGRESS_LINE_MAX};
use crate::tickets::{title_missing, TicketDraft};
use serde_json::json;
use std::sync::Arc;
use tauri::Runtime;

/// The longest title a ticket made here has, in characters.
const TITLE_MAX: usize = 200;
/// The longest description, and criterion, in characters; the most criteria.
const DESCRIPTION_MAX: usize = 10_000;
const CRITERION_MAX: usize = 500;
const CRITERIA_MAX: usize = 30;

/// A guard said no.
fn refusal(e: anyhow::Error) -> ToolError {
    ToolError::Refused(format!("{e:#}"))
}

/// Something asked does not fit.
fn failure(e: anyhow::Error) -> ToolError {
    ToolError::Failed(format!("{e:#}"))
}

/// A tool for the agents of Escouade alone: the calling agent's id.
fn agent_only(caller: &Caller) -> Result<&str, ToolError> {
    match caller {
        Caller::Agent(id) => Ok(id),
        Caller::External => Err(ToolError::Refused(tr!(
            "Cet outil est réservé aux agents d’Escouade : Claude hors Escouade ne peut pas l’appeler.",
            "This tool is reserved to Escouade’s own agents: Claude outside Escouade cannot call it."
        ))),
    }
}

/// An agent of Escouade acts on its own project alone (it may read the others); Claude outside
/// Escouade acts on any.
fn own_project_only<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    project_id: &str,
) -> Result<(), ToolError> {
    let Caller::Agent(id) = caller else {
        return Ok(());
    };
    let (name, own) = match core.agent(id) {
        Ok(h) => {
            let rt = h.lock();
            (rt.meta.name.clone(), rt.meta.project_id.clone())
        }
        Err(_) => (id.clone(), String::new()),
    };
    if own == project_id {
        return Ok(());
    }
    let project_name = |id: &str| core.project(id).map_or_else(|_| id.to_string(), |p| p.name);
    let (own_name, other) = (project_name(&own), project_name(project_id));
    Err(ToolError::Refused(tr!(
        "L’agent {name} n’agit que dans son propre projet ({own_name}) : le projet {other} n’est pas le sien.",
        "The agent {name} only acts in its own project ({own_name}): the project {other} is not its own."
    )))
}

// ---------- what is asked ----------

/// A title: on one line, not blank, `TITLE_MAX` characters at most.
fn title(asked: &str) -> Result<String, ToolError> {
    let line = one_line(asked);
    if line.is_empty() {
        return Err(failure(title_missing()));
    }
    let n = line.chars().count();
    if n > TITLE_MAX {
        return Err(ToolError::Failed(tr!(
            "Le titre fait {n} caractères : {TITLE_MAX} au plus.",
            "The title is {n} characters long: {TITLE_MAX} at most."
        )));
    }
    Ok(line)
}

/// A description: trimmed, `DESCRIPTION_MAX` characters at most.
fn description(asked: &str) -> Result<String, ToolError> {
    let text = asked.trim();
    let n = text.chars().count();
    if n > DESCRIPTION_MAX {
        return Err(ToolError::Failed(tr!(
            "La description fait {n} caractères : {DESCRIPTION_MAX} au plus.",
            "The description is {n} characters long: {DESCRIPTION_MAX} at most."
        )));
    }
    Ok(text.to_string())
}

/// The criteria, one per line (as the window's form takes them), blank lines dropped.
fn criteria(asked: &[String]) -> Result<Vec<String>, ToolError> {
    let lines: Vec<String> = asked
        .iter()
        .flat_map(|c| c.lines())
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() > CRITERIA_MAX {
        return Err(ToolError::Failed(tr!(
            "{count} critères : {CRITERIA_MAX} au plus.",
            "{count} criteria: {CRITERIA_MAX} at most.",
            count = lines.len()
        )));
    }
    if let Some(long) = lines.iter().find(|l| l.chars().count() > CRITERION_MAX) {
        let n = long.chars().count();
        return Err(ToolError::Failed(tr!(
            "Un critère fait {n} caractères : {CRITERION_MAX} au plus.",
            "A criterion is {n} characters long: {CRITERION_MAX} at most."
        )));
    }
    Ok(lines)
}

/// `text` on one line: breaks, tabs and runs of spaces are single spaces.
fn one_line(text: &str) -> String {
    let spaced: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A character that is not drawn, or drawn as something else than itself: the controls, the
/// format characters (zero-width marks, direction marks and overrides…), the line and paragraph
/// separators and the ones ignored when drawn (variation selectors, tag characters…). Written as
/// ranges as the window's `revealHidden` has them (it decides by Unicode property, which `std`
/// does not know).
fn is_hidden(c: char) -> bool {
    const RANGES: &[(u32, u32)] = &[
        (0x00AD, 0x00AD),   // soft hyphen
        (0x034F, 0x034F),   // combining grapheme joiner
        (0x0600, 0x0605),   // Arabic number signs
        (0x061C, 0x061C),   // Arabic letter mark
        (0x06DD, 0x06DD),   // Arabic end of ayah
        (0x070F, 0x070F),   // Syriac abbreviation mark
        (0x0890, 0x0891),   // Arabic pound and piastre marks
        (0x08E2, 0x08E2),   // Arabic disputed end of ayah
        (0x115F, 0x1160),   // hangul fillers
        (0x17B4, 0x17B5),   // Khmer inherent vowels
        (0x180B, 0x180F),   // Mongolian free variation selectors
        (0x200B, 0x200F),   // zero-width marks, direction marks
        (0x2028, 0x202E),   // separators, direction embeddings and overrides
        (0x2060, 0x206F),   // word joiner, invisible operators, deprecated format characters
        (0x2800, 0x2800),   // braille blank
        (0x3164, 0x3164),   // hangul filler
        (0xFE00, 0xFE0F),   // variation selectors
        (0xFEFF, 0xFEFF),   // zero-width no-break space
        (0xFFA0, 0xFFA0),   // halfwidth hangul filler
        (0xFFF0, 0xFFFB),   // unassigned, interlinear annotation
        (0x110BD, 0x110BD), // Kaithi number sign
        (0x110CD, 0x110CD), // Kaithi number sign above
        (0x13430, 0x1343F), // Egyptian hieroglyph format controls
        (0x1BCA0, 0x1BCA3), // shorthand format controls
        (0x1D173, 0x1D17A), // musical format controls
        (0xE0000, 0xE0FFF), // tag characters, variation selectors supplement
    ];
    c.is_control()
        || RANGES
            .iter()
            .any(|&(from, to)| (from..=to).contains(&(c as u32)))
}

/// `text` as a status line is kept: on one line (breaks, tabs and runs of spaces are single
/// spaces) and with none of the characters that are not drawn (`is_hidden`), which could hide
/// part of it or reverse its reading (U+202E).
fn visible_line(text: &str) -> String {
    let spaced: String = text
        .chars()
        .filter_map(|c| match c {
            c if c.is_whitespace() || c.is_control() => Some(' '),
            c if is_hidden(c) => None,
            c => Some(c),
        })
        .collect();
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The ids of the tickets of `project` the keys (or ids) name.
fn after_ids<R: Runtime>(
    core: &Core<R>,
    project: &Project,
    asked: &[String],
) -> Result<Vec<String>, ToolError> {
    asked
        .iter()
        .map(|k| resolve::ticket_in(core, k, Some(project)).map(|t| t.id))
        .collect()
}

/// The project a ticket is in.
fn project_of<R: Runtime>(core: &Core<R>, project_id: &str) -> Result<Project, ToolError> {
    core.project(project_id).map_err(failure)
}

// ---------- guardrails ----------

/// A time as the refusals say it: "14:30" today, "2026-10-12 14:30" another day.
fn when<R: Runtime>(core: &Core<R>, ms: i64) -> String {
    use chrono::TimeZone;
    let at = chrono::Local.timestamp_millis_opt(ms).single();
    let now = chrono::Local
        .timestamp_millis_opt(core.pause_now())
        .single();
    match (at, now) {
        (Some(at), Some(now)) if at.date_naive() == now.date_naive() => {
            at.format("%H:%M").to_string()
        }
        (Some(at), _) => at.format("%Y-%m-%d %H:%M").to_string(),
        _ => String::new(),
    }
}

/// Why nothing that spends quota is started now, and until when (`Core::held`): the autopilot's
/// pause, else an agent waiting for its quota.
fn hold<R: Runtime>(core: &Core<R>) -> Option<String> {
    use crate::model::PauseReason;
    if let Some(p) = core.autopilot_pause() {
        let until = when(core, p.until);
        let pct = p.pct.map_or(0, |pct| pct.round() as i64);
        return Some(match p.reason {
            PauseReason::Limit => tr!(
                "Le pilote auto est en pause jusqu’à {until} : la limite d’usage est atteinte. Réessaie à ce moment-là.",
                "The autopilot is paused until {until}: the usage limit is reached. Try again then."
            ),
            PauseReason::FiveHour => tr!(
                "Le pilote auto est en pause jusqu’à {until} : la fenêtre de 5 h est utilisée à {pct} %. Réessaie à ce moment-là.",
                "The autopilot is paused until {until}: the 5-hour window is {pct}% used. Try again then."
            ),
            PauseReason::Week => tr!(
                "Le pilote auto est en pause jusqu’à {until} : la fenêtre hebdomadaire est utilisée à {pct} %. Réessaie à ce moment-là.",
                "The autopilot is paused until {until}: the weekly window is {pct}% used. Try again then."
            ),
        });
    }
    if core.quota_paused() {
        let resume = core
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                rt.meta.resume_at.filter(|_| !rt.meta.archived)
            })
            .max()?;
        let until = when(core, resume);
        return Some(tr!(
            "Un agent attend son quota et reprend à {until} : rien ne démarre d’ici là. Réessaie à ce moment-là.",
            "An agent is waiting for its quota and resumes at {until}: nothing starts until then. Try again then."
        ));
    }
    None
}

/// Held: a refusal that says why.
fn not_held<R: Runtime>(core: &Core<R>) -> Result<(), ToolError> {
    match hold(core) {
        Some(why) => Err(ToolError::Refused(why)),
        None => Ok(()),
    }
}

// ---------- tickets ----------

/// `create_ticket`: a ticket « À faire » at the end of the project's column.
pub(super) async fn create_ticket<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: CreateTicketArgs,
) -> Result<String, ToolError> {
    let project = resolve::project(core, &a.project)?;
    own_project_only(core, caller, &project.id)?;
    let draft = TicketDraft {
        title: title(&a.title)?,
        description: description(a.description.as_deref().unwrap_or_default())?,
        criteria: criteria(a.criteria.as_deref().unwrap_or_default())?,
        // The default number of loops, as the window's form starts with.
        max_loops: 0,
        after: after_ids(core, &project, a.after.as_deref().unwrap_or_default())?,
    };
    let made = core
        .ticket_create(&project.id, draft)
        .await
        .map_err(failure)?;
    json(&ticket_view(core, &made))
}

/// `update_ticket`: the fields given replace the ticket's; the others stay.
pub(super) fn update_ticket<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: UpdateTicketArgs,
) -> Result<String, ToolError> {
    let t = resolve::ticket(core, &a.ticket)?;
    own_project_only(core, caller, &t.project_id)?;
    if a.title.is_none() && a.description.is_none() && a.criteria.is_none() && a.after.is_none() {
        return Err(ToolError::Failed(tr!(
            "Rien à modifier : donne au moins un des champs title, description, criteria ou after.",
            "Nothing to change: give at least one of the fields title, description, criteria or after."
        )));
    }
    let project = project_of(core, &t.project_id)?;
    let draft = TicketDraft {
        title: match &a.title {
            Some(new) => title(new)?,
            None => t.title.clone(),
        },
        description: match &a.description {
            Some(new) => description(new)?,
            None => t.description.clone(),
        },
        criteria: match &a.criteria {
            Some(new) => criteria(new)?,
            None => t.criteria.iter().map(|c| c.text.clone()).collect(),
        },
        max_loops: t.max_loops,
        after: match &a.after {
            Some(new) => after_ids(core, &project, new)?,
            None => t.after.clone(),
        },
    };
    let updated = core.ticket_update(&t.id, draft).map_err(refusal)?;
    json(&ticket_view(core, &updated))
}

/// `move_ticket`: the ticket's place in « À faire »; answers the column's keys in their order.
pub(super) fn move_ticket<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: MoveTicketArgs,
) -> Result<String, ToolError> {
    let t = resolve::ticket(core, &a.ticket)?;
    own_project_only(core, caller, &t.project_id)?;
    let before = a.before.as_deref().map(str::trim).filter(|b| !b.is_empty());
    let target;
    let to = match (a.position, before) {
        (PositionArg::Top, None) => MoveTo::Top,
        (PositionArg::Bottom, None) => MoveTo::Bottom,
        (PositionArg::Before, Some(asked)) => {
            let project = project_of(core, &t.project_id)?;
            target = resolve::ticket_in(core, asked, Some(&project))?.id;
            MoveTo::Before(&target)
        }
        (PositionArg::Before, None) => {
            return Err(ToolError::Failed(tr!(
                "Avec la position « before », donne aussi le ticket devant lequel placer celui-ci (argument before).",
                "With the position “before”, also give the ticket to put this one before (argument before)."
            )))
        }
        (_, Some(_)) => {
            return Err(ToolError::Failed(tr!(
                "L’argument before ne va qu’avec la position « before ».",
                "The argument before only goes with the position “before”."
            )))
        }
    };
    core.ticket_move(&t.id, to).map_err(refusal)?;
    json(&json!({ "ticket": t.key, "todo": todo_keys(core, &t.project_id) }))
}

/// `start_ticket`: the window's « Lancer ». Refused where the window would not offer it, or ask
/// the user first.
pub(super) fn start_ticket<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: StartTicketArgs,
) -> Result<String, ToolError> {
    let t = resolve::ticket(core, &a.ticket)?;
    own_project_only(core, caller, &t.project_id)?;
    // A ticket that left « À faire » is told so by the core.
    if t.column == Column::Todo {
        not_held(core)?;
        let project = project_of(core, &t.project_id)?;
        let issue = core.board_issues.lock().get(&project.id).cloned();
        if let Some(issue) = issue {
            return Err(ToolError::Refused(tr!(
                "Le Kanban du projet {project} ne démarre rien pour l’instant : {issue}",
                "The project {project}’s board starts nothing for now: {issue}",
                project = project.name
            )));
        }
        let all = core.tickets.read().clone();
        let max = project.board.max_parallel.clamp(1, 6);
        let busy = all
            .iter()
            .filter(|x| {
                x.project_id == project.id && x.column == Column::Doing && x.blocked.is_none()
            })
            .count();
        if busy >= max as usize {
            return Err(ToolError::Refused(tr!(
                "Le maximum de tickets en parallèle du projet {project} est atteint ({max}, réglage « En parallèle » du Kanban) : réessaie quand l’un d’eux est fini.",
                "The most tickets in parallel for the project {project} is reached ({max}, the Kanban’s “In parallel” setting): try again when one is done.",
                project = project.name
            )));
        }
        let awaited: Vec<&str> = t
            .after
            .iter()
            .filter_map(|id| all.iter().find(|x| x.id == *id))
            .filter(|x| x.project_id == project.id && x.column != Column::Done)
            .map(|x| x.key.as_str())
            .collect();
        if !awaited.is_empty() {
            return Err(ToolError::Refused(tr!(
                "{key} vient après des tickets qui ne sont pas terminés : {awaited}. Le pilote auto le démarre quand ils le sont ; pour le lancer avant, retire ces dépendances avec update_ticket.",
                "{key} comes after tickets that are not done yet: {awaited}. The autopilot starts it once they are; to launch it sooner, take these dependencies off with update_ticket.",
                key = t.key,
                awaited = awaited.join(", ")
            )));
        }
    }
    core.ticket_start(&t.id).map_err(refusal)?;
    json(&json!({ "key": t.key, "starting": true }))
}

// ---------- agents ----------

/// A model as Claude Code names it, one the window offers when it knows the list.
fn model<R: Runtime>(core: &Core<R>, asked: Option<&str>) -> Result<Option<String>, ToolError> {
    let Some(m) = asked.map(str::trim).filter(|m| !m.is_empty()) else {
        return Ok(None);
    };
    // It rides on Claude Code's command line: nothing a flag could be made of.
    let shaped = m.chars().count() <= 100
        && m.starts_with(|c: char| c.is_ascii_alphanumeric())
        && m.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-:[]/".contains(c));
    if !shaped {
        return Err(ToolError::Failed(tr!(
            "« {m} » n’est pas un nom de modèle.",
            "“{m}” is not a model name."
        )));
    }
    let known = core.models.read().clone();
    if known.is_empty() {
        return Ok(Some(m.to_string()));
    }
    if let Some(k) = known.iter().find(|k| k.value.eq_ignore_ascii_case(m)) {
        return Ok(Some(k.value.clone()));
    }
    if known
        .iter()
        .any(|k| k.resolved_model.eq_ignore_ascii_case(m))
    {
        return Ok(Some(m.to_string()));
    }
    let choices = known
        .iter()
        .map(|k| k.value.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(ToolError::Failed(tr!(
        "Modèle « {m} » inconnu. Modèles : {choices}.",
        "Unknown model “{m}”. Models: {choices}."
    )))
}

/// `create_agent`: a new agent of the project, and its first message. The window's selection is
/// the user's: the agent is not selected.
pub(super) async fn create_agent<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: CreateAgentArgs,
) -> Result<String, ToolError> {
    let project = resolve::project(core, &a.project)?;
    own_project_only(core, caller, &project.id)?;
    let message = a.message.trim();
    if message.is_empty() {
        return Err(ToolError::Failed(tr!(
            "Écris le message qui lance l’agent.",
            "Write the message that starts the agent."
        )));
    }
    let model = model(core, a.model.as_deref())?;
    not_held(core)?;
    // One at a time from the count to the first message: two calls at once do not both find a
    // place free.
    let _one = core.mcp.acting.lock().await;
    let max = project.board.max_parallel.clamp(1, 6) as usize;
    let working = core
        .agents
        .read()
        .values()
        .filter(|h| {
            let rt = h.lock();
            rt.meta.project_id == project.id && !rt.meta.archived && rt.meta.status.is_active()
        })
        .count();
    if working >= max {
        return Err(ToolError::Refused(tr!(
            "Le maximum d’agents qui travaillent en même temps dans le projet {project} est atteint ({max}, réglage « En parallèle » du Kanban) : réessaie quand l’un d’eux a fini son tour.",
            "The most agents working at once in the project {project} is reached ({max}, the Kanban’s “In parallel” setting): try again when one has finished its turn.",
            project = project.name
        )));
    }
    let view = core
        .create_agent_with(
            &project.id,
            AgentOptions {
                model,
                isolated: a.worktree,
                // Its Claude account is the default one until accounts choose (K4).
                ..Default::default()
            },
        )
        .await
        .map_err(failure)?;
    let (id, name) = (view.meta.id, view.meta.name);
    // Under its author's name, as every message from the server: not taken for the user's words.
    let origin = activity::caller_name(core, Some(caller));
    if let Err(e) = core
        .send_message_from(&id, Some(&origin), message.to_string(), vec![])
        .await
    {
        return Err(ToolError::Failed(tr!(
            "L’agent {name} (id {id}) est créé, mais son message n’a pas pu être envoyé : {e:#}",
            "The agent {name} (id {id}) is created, but its message could not be sent: {e:#}"
        )));
    }
    json(&json!({ "id": id, "name": name }))
}

/// The agent `asked` names (in the project `project` when given), who is not the caller and,
/// for an agent calling, is of its project.
fn other_agent<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    asked: &str,
    project: Option<&str>,
    itself: String,
) -> Result<resolve::AgentRef, ToolError> {
    let scope = project.map(|p| resolve::project(core, p)).transpose()?;
    let found = resolve::agent(core, asked, scope.as_ref())?;
    if matches!(caller, Caller::Agent(id) if *id == found.id) {
        return Err(ToolError::Refused(itself));
    }
    own_project_only(core, caller, &found.project_id)?;
    Ok(found)
}

/// `send_message`: a message to an agent, as typed in its conversation.
pub(super) async fn send_message<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: SendMessageArgs,
) -> Result<String, ToolError> {
    let found = other_agent(
        core,
        caller,
        &a.agent,
        a.project.as_deref(),
        tr!(
            "Un agent ne s’envoie pas de message à lui-même.",
            "An agent does not send a message to itself."
        ),
    )?;
    if found.archived {
        let name = &found.name;
        return Err(ToolError::Refused(tr!(
            "L’agent {name} est archivé : il ne reçoit plus de message.",
            "The agent {name} is archived: it no longer takes messages."
        )));
    }
    let text = a.text.trim();
    if text.is_empty() {
        return Err(ToolError::Failed(tr!(
            "Écris le message à envoyer.",
            "Write the message to send."
        )));
    }
    not_held(core)?;
    // A message sent to an agent at work waits for the end of its turn.
    let queued = core
        .agent(&found.id)
        .is_ok_and(|h| h.lock().meta.status.is_active());
    // Under its author's name: the agent and the conversation tell it from the user's words.
    let origin = activity::caller_name(core, Some(caller));
    core.send_message_from(&found.id, Some(&origin), text.to_string(), vec![])
        .await
        .map_err(failure)?;
    json(&json!({ "id": found.id, "name": found.name, "queued": queued }))
}

/// `stop_agent`: the agent's running turn is interrupted, as the window's stop button does.
pub(super) async fn stop_agent<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: StopAgentArgs,
) -> Result<String, ToolError> {
    let found = other_agent(
        core,
        caller,
        &a.agent,
        a.project.as_deref(),
        tr!(
            "Un agent ne peut pas arrêter son propre tour.",
            "An agent cannot stop its own turn."
        ),
    )?;
    let interrupted = core
        .agent(&found.id)
        .is_ok_and(|h| h.lock().meta.status.is_active());
    core.interrupt(&found.id).await.map_err(failure)?;
    json(&json!({ "id": found.id, "name": found.name, "interrupted": interrupted }))
}

// ---------- for the agents of Escouade ----------

/// `report_progress`: the status line on the agent's card and its ticket's, until its next call.
pub(super) fn report_progress<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: ReportProgressArgs,
) -> Result<String, ToolError> {
    let id = agent_only(caller)?;
    // As the window would show it, nothing hidden in it: a direction override could reverse how
    // a line reads on the cards, a zero-width mark hide part of it.
    let line = visible_line(&a.line);
    let n = line.chars().count();
    if n > PROGRESS_LINE_MAX {
        return Err(ToolError::Failed(tr!(
            "La ligne d’état fait {n} caractères : {PROGRESS_LINE_MAX} au plus. Raccourcis-la.",
            "The status line is {n} characters long: {PROGRESS_LINE_MAX} at most. Shorten it."
        )));
    }
    let h = core.agent(id).map_err(failure)?;
    let line = (!line.is_empty()).then_some(line);
    h.lock().meta.progress_line = line.clone();
    core.emit_agent(&h);
    core.request_save();
    json(&json!({ "line": line }))
}

/// One of `split_ticket`'s tickets, checked: its draft.
fn sub_draft(sub: &SubTicket, after: Vec<String>) -> Result<TicketDraft, ToolError> {
    Ok(TicketDraft {
        title: title(&sub.title)?,
        description: description(sub.description.as_deref().unwrap_or_default())?,
        criteria: criteria(sub.criteria.as_deref().unwrap_or_default())?,
        max_loops: 0,
        after,
    })
}

/// `split_ticket`: tickets « À faire » after the calling agent's own, one after the other (`chain`,
/// the default) or all after it alone; answers their keys.
pub(super) async fn split_ticket<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: SplitTicketArgs,
) -> Result<String, ToolError> {
    let agent_id = agent_only(caller)?;
    let (name, ticket_id) = {
        let h = core.agent(agent_id).map_err(failure)?;
        let rt = h.lock();
        (rt.meta.name.clone(), rt.meta.ticket_id.clone())
    };
    let own = ticket_id
        .and_then(|id| core.ticket(&id).ok())
        .ok_or_else(|| {
            ToolError::Refused(tr!(
                "L’agent {name} n’a pas de ticket : il n’a rien à découper.",
                "The agent {name} has no ticket: it has nothing to split."
            ))
        })?;
    own_project_only(core, caller, &own.project_id)?;
    let count = a.tickets.len();
    if count == 0 {
        return Err(ToolError::Failed(tr!(
            "Donne au moins un ticket à créer.",
            "Give at least one ticket to make."
        )));
    }
    if count > SPLIT_MAX {
        return Err(ToolError::Failed(tr!(
            "{SPLIT_MAX} tickets au plus par appel ({count} donnés).",
            "{SPLIT_MAX} tickets at most per call ({count} given)."
        )));
    }
    // All checked before any is made.
    let mut drafts = Vec::with_capacity(count);
    for (i, sub) in a.tickets.iter().enumerate() {
        let n = i + 1;
        drafts.push(sub_draft(sub, Vec::new()).map_err(|e| match e {
            ToolError::Failed(why) | ToolError::Refused(why) => ToolError::Failed(tr!(
                "Ticket {n} de la liste : {why}",
                "Ticket {n} of the list: {why}"
            )),
        })?);
    }
    let chain = a.chain.unwrap_or(true);
    let mut previous = own.id.clone();
    let mut made: Vec<String> = Vec::with_capacity(count);
    for mut draft in drafts {
        draft.after = vec![if chain {
            previous.clone()
        } else {
            own.id.clone()
        }];
        match core.ticket_create(&own.project_id, draft).await {
            Ok(t) => {
                previous = t.id;
                made.push(t.key);
            }
            Err(e) => {
                let done = made.join(", ");
                return Err(ToolError::Failed(tr!(
                    "Tickets créés avant l’échec : {done}. Les suivants ne le sont pas : {e:#}",
                    "Tickets made before the failure: {done}. The next ones are not: {e:#}"
                )));
            }
        }
    }
    json(&made)
}
