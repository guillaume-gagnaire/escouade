//! The tools of the scheduled tasks (ESC-448): list, create, update, pause and delete a project's
//! tasks, as the window's section « Programmées » and its form do, through the same functions of
//! the core. A task is named within its project (by its id or its title), and its project as any
//! tool names one: an agent of Escouade sees and acts where it does for the tickets
//! (`resolve::Sight`), a private project being found by its own agents alone.

use super::act::{
    account_setting, carrier, criteria, described, failure, may_act_on, model_and_effort, title,
};
use super::read::json;
use super::resolve;
use super::tools::{
    CreateScheduledTaskArgs, DeleteScheduledTaskArgs, ListScheduledTasksArgs,
    PauseScheduledTaskArgs, RhythmArg, ScheduledFieldsArgs, ToolError, UpdateScheduledTaskArgs,
};
use super::Caller;
use crate::accounts;
use crate::core::Core;
use crate::i18n;
use crate::model::{Project, Rhythm, RhythmKind, ScheduledAuthor, ScheduledTask};
use crate::scheduled::{cron_of, shortest_gap, ScheduledDraft, ScheduledView, AGENT_MIN_GAP_MS};
use chrono::{Local, TimeZone};
use serde::Serialize;
use std::sync::Arc;

/// A task as the tools answer it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskRow {
    id: String,
    title: String,
    description: String,
    criteria: Vec<String>,
    rhythm: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    days: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    day: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<String>,
    /// The cron expression it runs by (the simple rhythms written as one).
    cron: String,
    catch_up: bool,
    paused: bool,
    /// Its next due time, in the machine's time (none while paused).
    next: Option<String>,
    /// The key of the ticket of its last run, and whether it is « À faire » or « En cours » still.
    last_ticket: Option<String>,
    running: bool,
    last_run_at: Option<String>,
    /// What became of its last due time when it made no ticket.
    last_note: Option<String>,
    account: String,
    model: String,
    effort: String,
    /// Made or changed by an agent, it waits for the user's go-ahead (ESC-647): no ticket until then.
    awaiting: bool,
    /// The agent that made or last changed it (ESC-647); none: the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<AuthorRow>,
}

/// Who made a task, as the tools answer it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorRow {
    agent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ticket: Option<String>,
    project: String,
}

/// `ms` in the machine's time, as RFC 3339.
fn local(ms: i64) -> Option<String> {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
}

fn row(core: &Core, v: &ScheduledView) -> TaskRow {
    let t = &v.task;
    let r = &t.rhythm;
    let simple = r.kind != RhythmKind::Cron;
    let (last_ticket, running) = {
        let tickets = core.tickets.read();
        let last = t
            .last_ticket
            .as_deref()
            .and_then(|id| tickets.iter().find(|x| x.id == id));
        (
            last.map(|x| x.key.clone()),
            crate::scheduled::running(&tickets, t),
        )
    };
    let account = if t.account.is_empty() {
        "auto".to_string()
    } else {
        let settings = core.settings.read();
        accounts::find(&settings, &t.account)
            .map_or_else(|| t.account.clone(), |a| accounts::name_in(i18n::ui(), a))
    };
    let auto = |s: &str| {
        if s.is_empty() {
            "auto".into()
        } else {
            s.to_string()
        }
    };
    TaskRow {
        id: t.id.clone(),
        title: t.title.clone(),
        description: t.description.clone(),
        criteria: t.criteria.clone(),
        rhythm: match r.kind {
            RhythmKind::Daily => "daily",
            RhythmKind::Weekly => "weekly",
            RhythmKind::Monthly => "monthly",
            RhythmKind::Cron => "cron",
        },
        days: (r.kind == RhythmKind::Weekly).then(|| r.days.clone()),
        day: (r.kind == RhythmKind::Monthly).then_some(r.day),
        time: simple.then(|| format!("{:02}:{:02}", r.hour, r.minute)),
        cron: cron_of(r).unwrap_or_else(|_| r.cron.clone()),
        catch_up: t.catch_up,
        paused: t.paused,
        next: v.next.and_then(local),
        last_ticket,
        running,
        last_run_at: t.last_run_at.and_then(local),
        last_note: t.last_note.clone(),
        account,
        model: auto(&t.model),
        effort: auto(&t.effort),
        awaiting: t.awaiting,
        author: t.author.as_ref().map(|a| AuthorRow {
            agent: a.agent_name.clone(),
            ticket: a.ticket_key.clone(),
            project: core
                .project(&a.project_id)
                .map_or_else(|_| a.project_id.clone(), |p| p.name.clone()),
        }),
    }
}

/// The agent that calls (ESC-647), as a task keeps its author; none for Claude outside Escouade, which is the user's.
/// An agent other than the Overseer makes and changes the tasks of its own project only.
fn author(
    core: &Core,
    caller: &Caller,
    project: &Project,
) -> Result<Option<ScheduledAuthor>, ToolError> {
    let Caller::Agent(id) = caller else {
        return Ok(None);
    };
    let (name, own, ticket) = {
        let h = core.agent(id).map_err(|_| {
            ToolError::Refused(tr!(
                "L’agent {id} n’est plus dans Escouade : il n’agit plus.",
                "The agent {id} is no longer in Escouade: it no longer acts."
            ))
        })?;
        let rt = h.lock();
        (
            rt.meta.name.clone(),
            rt.meta.project_id.clone(),
            rt.meta.ticket_id.clone(),
        )
    };
    if own != project.id && !core.is_overseer(id) {
        return Err(ToolError::Refused(tr!(
            "Un agent ne programme des tâches que dans son propre projet : {name} n’en crée ni n’en change dans « {project} ».",
            "An agent schedules tasks in its own project only: {name} neither makes nor changes one in “{project}”.",
            project = project.name
        )));
    }
    let ticket_key = ticket
        .as_deref()
        .and_then(|t| core.ticket(t).ok())
        .map(|t| t.key);
    Ok(Some(ScheduledAuthor {
        agent_id: id.clone(),
        agent_name: name,
        project_id: own,
        ticket_id: ticket,
        ticket_key,
        at: core.schedule_clock.now(),
    }))
}

/// An agent's rhythm leaves 15 minutes at least between two due times (ESC-647).
fn agent_rhythm(core: &Core, rhythm: &Rhythm) -> Result<(), ToolError> {
    let gap = shortest_gap(rhythm, core.schedule_clock.now(), &Local).map_err(failure)?;
    if gap.is_some_and(|g| g < AGENT_MIN_GAP_MS) {
        return Err(ToolError::Refused(tr!(
            "Un agent ne programme pas une tâche plus d’une fois par quart d’heure : laisse 15 minutes au moins entre deux échéances (l’utilisateur, lui, le peut dans la fenêtre).",
            "An agent does not schedule a task more than once every quarter of an hour: leave 15 minutes at least between two due times (the user can, in the window)."
        )));
    }
    Ok(())
}

/// The task `asked` names in `project`: its id, else its title in any case.
fn task(core: &Core, project: &Project, asked: &str) -> Result<ScheduledTask, ToolError> {
    let asked = asked.trim();
    let all: Vec<ScheduledTask> = core
        .scheduled
        .read()
        .iter()
        .filter(|t| t.project_id == project.id)
        .cloned()
        .collect();
    if let Some(t) = all.iter().find(|t| t.id == asked) {
        return Ok(t.clone());
    }
    let named: Vec<&ScheduledTask> = all
        .iter()
        .filter(|t| t.title.trim().to_lowercase() == asked.to_lowercase())
        .collect();
    match named.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(ToolError::Failed(tr!(
            "Aucune tâche programmée « {asked} » dans ce projet : list_scheduled_tasks les donne.",
            "No scheduled task “{asked}” in this project: list_scheduled_tasks gives them."
        ))),
        several => Err(ToolError::Failed(tr!(
            "Plusieurs tâches programmées s’appellent « {asked} » : {ids}. Donne son id.",
            "Several scheduled tasks are named “{asked}”: {ids}. Give its id.",
            ids = several
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// The rhythm the call gives, over `had` (a new task's: none): None when it names none.
fn rhythm_of(had: Option<&Rhythm>, a: &ScheduledFieldsArgs) -> Result<Option<Rhythm>, ToolError> {
    if a.rhythm.is_none()
        && a.days.is_none()
        && a.day.is_none()
        && a.time.is_none()
        && a.cron.is_none()
    {
        return Ok(None);
    }
    let mut r = had.cloned().unwrap_or(Rhythm {
        hour: 9,
        ..Default::default()
    });
    if let Some(kind) = a.rhythm {
        r.kind = match kind {
            RhythmArg::Daily => RhythmKind::Daily,
            RhythmArg::Weekly => RhythmKind::Weekly,
            RhythmArg::Monthly => RhythmKind::Monthly,
            RhythmArg::Cron => RhythmKind::Cron,
        };
    } else if had.is_none() {
        return Err(ToolError::Failed(tr!(
            "Donne le rythme : daily, weekly, monthly ou cron.",
            "Give the rhythm: daily, weekly, monthly or cron."
        )));
    }
    if let Some(days) = &a.days {
        r.days = days.clone();
    }
    if let Some(day) = a.day {
        r.day = day;
    }
    if let Some(cron) = &a.cron {
        r.cron = cron.trim().to_string();
    }
    if let Some(time) = &a.time {
        let parsed = time
            .trim()
            .split_once(':')
            .and_then(|(h, m)| Some((h.trim().parse::<u8>().ok()?, m.trim().parse::<u8>().ok()?)))
            .filter(|(h, m)| *h < 24 && *m < 60);
        let Some((h, m)) = parsed else {
            return Err(ToolError::Failed(tr!(
                "L’heure « {time} » ne se lit pas : écris-la HH:MM, comme 07:00.",
                "The time “{time}” cannot be read: write it HH:MM, like 07:00."
            )));
        };
        (r.hour, r.minute) = (h, m);
    }
    Ok(Some(r))
}

/// The draft of what the call gives, checked as `create_ticket` checks a ticket's fields.
fn draft(
    core: &Core,
    caller: &Caller,
    project: &Project,
    had: Option<&ScheduledTask>,
    title_asked: Option<&str>,
    a: &ScheduledFieldsArgs,
) -> Result<ScheduledDraft, ToolError> {
    let account = account_setting(core, a.account.as_deref());
    let on = carrier(core, project, None, account.as_deref());
    let (model, effort) = model_and_effort(core, &on, a.model.as_deref(), a.effort.as_deref())?;
    Ok(ScheduledDraft {
        title: title_asked.map(title).transpose()?,
        description: a
            .description
            .as_deref()
            .map(|d| described(core, caller, d))
            .transpose()?,
        criteria: a.criteria.as_deref().map(criteria).transpose()?,
        rhythm: rhythm_of(had.map(|t| &t.rhythm), a)?,
        catch_up: a.catch_up,
        account,
        model,
        effort,
        ..Default::default()
    })
}

/// `list_scheduled_tasks`: the project's tasks, in the order they were made.
pub(super) fn list(
    core: &Core,
    caller: &Caller,
    a: ListScheduledTasksArgs,
) -> Result<String, ToolError> {
    let project = resolve::project(core, caller, &a.project)?;
    let rows: Vec<TaskRow> = core
        .scheduled_views()
        .iter()
        .filter(|v| v.task.project_id == project.id)
        .map(|v| row(core, v))
        .collect();
    json(&rows)
}

/// `create_scheduled_task`: a task of the project, its first due time the next one from now.
pub(super) fn create(
    core: &Arc<Core>,
    caller: &Caller,
    a: CreateScheduledTaskArgs,
) -> Result<String, ToolError> {
    let project = resolve::project(core, caller, &a.project)?;
    may_act_on(core, caller, &project.id)?;
    let by = author(core, caller, &project)?;
    let mut d = draft(core, caller, &project, None, Some(&a.title), &a.fields)?;
    if d.catch_up.is_none() {
        d.catch_up = Some(true);
    }
    if let (Some(_), Some(r)) = (&by, &d.rhythm) {
        agent_rhythm(core, r)?;
    }
    let v = core
        .scheduled_create_by(&project.id, d, by)
        .map_err(failure)?;
    json(&row(core, &v))
}

/// `update_scheduled_task`: what the call gives changes, the rest stays.
pub(super) fn update(
    core: &Arc<Core>,
    caller: &Caller,
    a: UpdateScheduledTaskArgs,
) -> Result<String, ToolError> {
    let project = resolve::project(core, caller, &a.project)?;
    may_act_on(core, caller, &project.id)?;
    let by = author(core, caller, &project)?;
    let t = task(core, &project, &a.task)?;
    let d = draft(
        core,
        caller,
        &project,
        Some(&t),
        a.title.as_deref(),
        &a.fields,
    )?;
    if by.is_some() {
        agent_rhythm(core, d.rhythm.as_ref().unwrap_or(&t.rhythm))?;
    }
    let v = core.scheduled_update_by(&t.id, d, by).map_err(failure)?;
    json(&row(core, &v))
}

/// `pause_scheduled_task`: the switch of its row.
pub(super) fn pause(
    core: &Arc<Core>,
    caller: &Caller,
    a: PauseScheduledTaskArgs,
) -> Result<String, ToolError> {
    let project = resolve::project(core, caller, &a.project)?;
    may_act_on(core, caller, &project.id)?;
    let t = task(core, &project, &a.task)?;
    let v = core
        .scheduled_set_paused(&t.id, a.paused.unwrap_or(true))
        .map_err(failure)?;
    json(&row(core, &v))
}

/// `delete_scheduled_task`: the task goes, the tickets it made stay.
pub(super) fn delete(
    core: &Arc<Core>,
    caller: &Caller,
    a: DeleteScheduledTaskArgs,
) -> Result<String, ToolError> {
    let project = resolve::project(core, caller, &a.project)?;
    may_act_on(core, caller, &project.id)?;
    let t = task(core, &project, &a.task)?;
    core.scheduled_delete(&t.id).map_err(failure)?;
    json(&serde_json::json!({ "id": t.id, "title": t.title, "deleted": true }))
}
