//! The scheduled tasks (ESC-448, docs/OVERSEER.md): tickets of an agent that come back at fixed
//! times, project by project. At each due time a ticket is made from the task and launched: it
//! follows the board as any other, its places and its quotas holding it as one launched by hand.
//!
//! - **Never twice**: the due time dealt with is written down (and saved) before its ticket is
//!   made; several missed while Escouade was closed give one run at most, the last of them.
//! - **One at a time**: a due time passes its turn while the ticket of the run before is « À
//!   faire » or « En cours » (« À tester » counts as done).
//! - **Caught up, or not**, task by task (`ScheduledTask::catch_up`): a due time that fell while
//!   Escouade was closed runs at its return, or waits for the next one.
//! - **The machine's time** (`chrono::Local`), summer time included: the cron expressions are
//!   read by `croner`. The clock is the app's own (`Clock`), which the tests set.

use crate::core::{project_not_found, Core};
use crate::model::*;
use crate::tickets::TicketDraft;
use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Local, TimeZone};
use croner::parser::{CronParser, Seconds, Year};
use croner::Cron;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::Runtime;

/// How many due times the form shows ahead (« Prochaines »).
pub(crate) const PREVIEW: usize = 3;

/// How often the due times are looked at: often enough for a task to start within its minute.
/// `ESCOUADE_SCHEDULE_TICK_MS` sets it in a debug build (the end-to-end tests).
pub(crate) fn tick_every() -> Duration {
    let ms = if cfg!(debug_assertions) {
        std::env::var("ESCOUADE_SCHEDULE_TICK_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|ms| *ms >= 50)
    } else {
        None
    };
    Duration::from_millis(ms.unwrap_or(15_000))
}

/// The time the scheduled tasks go by: the real one, unless the tests say otherwise. In a debug
/// build, `ESCOUADE_SCHEDULE_SPEED` makes it run faster from the app's start (an end-to-end test
/// sees a due time come in seconds); the unit tests move it on (`advance`) and say when the app
/// started on it (`set_started`).
pub(crate) struct Clock {
    /// The real time the app started (ms), from which a faster clock runs.
    origin: i64,
    speed: i64,
    ahead: AtomicI64,
    /// When this run of the app began, on this clock: a due time before it fell while Escouade
    /// was closed.
    started: AtomicI64,
}

impl Clock {
    pub(crate) fn from_env() -> Self {
        let speed = if cfg!(debug_assertions) {
            std::env::var("ESCOUADE_SCHEDULE_SPEED")
                .ok()
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|s| (1..=100_000).contains(s))
        } else {
            None
        };
        Self::with_speed(speed.unwrap_or(1))
    }

    fn with_speed(speed: i64) -> Self {
        let origin = now_ms();
        Self {
            origin,
            speed,
            ahead: AtomicI64::new(0),
            started: AtomicI64::new(origin),
        }
    }

    pub(crate) fn now(&self) -> i64 {
        let real = now_ms();
        self.origin + (real - self.origin) * self.speed + self.ahead.load(Ordering::SeqCst)
    }

    pub(crate) fn started(&self) -> i64 {
        self.started.load(Ordering::SeqCst)
    }

    /// The clock moves on by `ms` (tests).
    #[cfg(test)]
    pub(crate) fn advance(&self, ms: i64) {
        self.ahead.fetch_add(ms, Ordering::SeqCst);
    }

    /// The clock is at `at` now (tests).
    #[cfg(test)]
    pub(crate) fn set(&self, at: i64) {
        let off = at - self.now();
        self.advance(off);
    }

    /// This run of the app began at `at` (tests: Escouade closed until then).
    #[cfg(test)]
    pub(crate) fn set_started(&self, at: i64) {
        self.started.store(at, Ordering::SeqCst);
    }
}

/// Why a rhythm is refused, in the language of the window.
macro_rules! refused {
    ($fr:literal, $en:literal $(,)?) => {
        anyhow!(tr!($fr, $en))
    };
}

/// The cron expression of a rhythm: the simple ones written as one, the cron one trimmed.
pub(crate) fn cron_of(r: &Rhythm) -> Result<String> {
    if r.kind != RhythmKind::Cron && (r.hour > 23 || r.minute > 59) {
        return Err(refused!(
            "L'heure n'en est pas une (de 00:00 à 23:59).",
            "That is not a time (00:00 to 23:59).",
        ));
    }
    let (m, h) = (r.minute, r.hour);
    Ok(match r.kind {
        RhythmKind::Daily => format!("{m} {h} * * *"),
        RhythmKind::Weekly => {
            let mut days: Vec<u8> = r
                .days
                .iter()
                .copied()
                .filter(|d| (1..=7).contains(d))
                .collect();
            days.sort_unstable();
            days.dedup();
            if days.is_empty() {
                return Err(refused!(
                    "Choisis au moins un jour de la semaine.",
                    "Choose at least one day of the week.",
                ));
            }
            // Cron counts Sunday as 0.
            let list: Vec<String> = days.iter().map(|d| (d % 7).to_string()).collect();
            format!("{m} {h} * * {}", list.join(","))
        }
        RhythmKind::Monthly => {
            if !(1..=31).contains(&r.day) {
                return Err(refused!(
                    "Le jour du mois va de 1 à 31.",
                    "The day of the month goes from 1 to 31.",
                ));
            }
            format!("{m} {h} {} * *", r.day)
        }
        RhythmKind::Cron => r.cron.split_whitespace().collect::<Vec<_>>().join(" "),
    })
}

/// The rhythm read, refused when it is no cron expression of five fields or never comes.
pub(crate) fn parse(r: &Rhythm) -> Result<Cron> {
    let expr = cron_of(r)?;
    if expr.is_empty() {
        return Err(refused!(
            "Écris une expression cron, comme « 0 7 * * 1 ».",
            "Write a cron expression, such as “0 7 * * 1”.",
        ));
    }
    if expr.split(' ').count() != 5 {
        return Err(refused!(
            "Une expression cron a cinq champs : minute, heure, jour du mois, mois, jour de la semaine.",
            "A cron expression has five fields: minute, hour, day of the month, month, day of the week.",
        ));
    }
    let cron = CronParser::builder()
        .seconds(Seconds::Disallowed)
        .year(Year::Disallowed)
        .build()
        .parse(&expr)
        .map_err(|e| {
            let why = e.to_string();
            anyhow!(tr!(
                "Cette expression cron ne se lit pas : {why}",
                "This cron expression cannot be read: {why}"
            ))
        })?;
    Ok(cron)
}

fn at<Tz: TimeZone>(tz: &Tz, ms: i64) -> Option<DateTime<Tz>> {
    tz.timestamp_millis_opt(ms).single()
}

/// The first due time strictly after `after` (ms), in the zone `tz`.
pub(crate) fn next_after<Tz: TimeZone>(cron: &Cron, after: i64, tz: &Tz) -> Option<i64> {
    let from = at(tz, after)?;
    cron.find_next_occurrence(&from, false)
        .ok()
        .map(|d| d.timestamp_millis())
}

/// The last due time at or before `until` (ms), in the zone `tz`.
pub(crate) fn last_until<Tz: TimeZone>(cron: &Cron, until: i64, tz: &Tz) -> Option<i64> {
    let from = at(tz, until)?;
    cron.find_previous_occurrence(&from, true)
        .ok()
        .map(|d| d.timestamp_millis())
}

/// The `n` due times after `after`, in the zone `tz`.
pub(crate) fn upcoming<Tz: TimeZone>(cron: &Cron, after: i64, n: usize, tz: &Tz) -> Vec<i64> {
    let mut out = Vec::with_capacity(n);
    let mut from = after;
    while out.len() < n {
        let Some(next) = next_after(cron, from, tz) else {
            break;
        };
        out.push(next);
        from = next;
    }
    out
}

/// Why a due time made no ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pass {
    /// The task is paused.
    Paused,
    /// It fell while Escouade was closed, and the task waits for the next one then.
    Missed,
    /// The ticket of the run before is « À faire » or « En cours » still.
    Running,
}

impl Pass {
    /// What the task's row says of it until the next run (none for a pause, which it shows).
    fn note(self) -> Option<String> {
        match self {
            Pass::Paused => None,
            Pass::Missed => Some(tr!(
                "Escouade était fermé à la dernière échéance : la tâche attend la suivante.",
                "Escouade was closed at the last due time: the task waits for the next one."
            )),
            Pass::Running => Some(tr!(
                "La dernière échéance a passé son tour : le ticket d'avant tournait encore.",
                "The last due time passed its turn: the ticket before was still running."
            )),
        }
    }
}

/// What a task does now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Due {
    /// No due time since the last one dealt with.
    Nothing,
    /// The due time `.0` makes its ticket.
    Run(i64),
    /// The due time `.0` passes, for the reason `.1`.
    Pass(i64, Pass),
}

/// What `task` does at `now` (ms), this run of the app having begun at `started`, the ticket of
/// its run before still under way (`running`) or not. Only the last due time since the one dealt
/// with counts: several missed give one run at most.
pub(crate) fn decide<Tz: TimeZone>(
    task: &ScheduledTask,
    now: i64,
    started: i64,
    running: bool,
    tz: &Tz,
) -> Due {
    let Ok(cron) = parse(&task.rhythm) else {
        return Due::Nothing;
    };
    let after = task.last_due.unwrap_or(i64::MIN).max(task.since);
    let Some(last) = last_until(&cron, now, tz) else {
        return Due::Nothing;
    };
    if last <= after {
        return Due::Nothing;
    }
    if task.paused {
        Due::Pass(last, Pass::Paused)
    } else if last < started && !task.catch_up {
        Due::Pass(last, Pass::Missed)
    } else if running {
        Due::Pass(last, Pass::Running)
    } else {
        Due::Run(last)
    }
}

/// The ticket of the task's run before is « À faire » or « En cours » still: « À tester » and
/// after count as done, and so does a ticket gone.
pub(crate) fn running(tickets: &[Ticket], task: &ScheduledTask) -> bool {
    task.last_ticket.as_deref().is_some_and(|id| {
        tickets
            .iter()
            .any(|t| t.id == id && matches!(t.column, Column::Todo | Column::Doing))
    })
}

/// A scheduled task as the window and the tools see it: with its next due time (none while
/// paused, or for a rhythm that never comes).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledView {
    #[serde(flatten)]
    pub task: ScheduledTask,
    pub next: Option<i64>,
}

/// A scheduled task as its form or a tool gives it: what is absent stays as it is (for a new
/// one, its default).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ScheduledDraft {
    pub title: Option<String>,
    pub description: Option<String>,
    pub criteria: Option<Vec<String>>,
    pub max_loops: Option<u32>,
    /// Null: the project's harness.
    #[serde(deserialize_with = "crate::tickets::given")]
    pub engine: Option<Option<EngineId>>,
    /// Empty: « Auto ».
    pub account: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub rhythm: Option<Rhythm>,
    /// « Lancer au retour d'Escouade, une seule fois »: on for a new one unless said.
    pub catch_up: Option<bool>,
    pub paused: Option<bool>,
}

fn task_not_found() -> anyhow::Error {
    anyhow!(tr!(
        "Cette tâche programmée n'existe plus.",
        "This scheduled task no longer exists."
    ))
}

impl<R: Runtime> Core<R> {
    /// Every scheduled task, with its next due time.
    pub fn scheduled_views(&self) -> Vec<ScheduledView> {
        let now = self.schedule_clock.now();
        self.scheduled
            .read()
            .iter()
            .map(|t| ScheduledView {
                next: if t.paused {
                    None
                } else {
                    parse(&t.rhythm)
                        .ok()
                        .and_then(|c| next_after(&c, now, &Local))
                },
                task: t.clone(),
            })
            .collect()
    }

    pub(crate) fn scheduled_view(&self, id: &str) -> Result<ScheduledView> {
        self.scheduled_views()
            .into_iter()
            .find(|v| v.task.id == id)
            .ok_or_else(task_not_found)
    }

    /// The window is told of every task, and they are saved.
    fn scheduled_changed(&self) {
        self.request_save();
        self.hub.emit(UiEvent::Scheduled {
            tasks: self.scheduled_views(),
        });
    }

    /// The next due times of a rhythm from now (« Prochaines » of the form), or why it is refused.
    pub fn scheduled_preview(&self, rhythm: &Rhythm) -> Result<Vec<i64>> {
        let cron = parse(rhythm)?;
        let next = upcoming(&cron, self.schedule_clock.now(), PREVIEW, &Local);
        if next.is_empty() {
            bail!(tr!(
                "Ce rythme ne tombe jamais.",
                "This rhythm never comes."
            ));
        }
        Ok(next)
    }

    /// `d` applied to `t`, checked: a title, a rhythm that comes, an account its project may use.
    fn scheduled_apply(&self, t: &mut ScheduledTask, d: ScheduledDraft) -> Result<()> {
        let now = self.schedule_clock.now();
        if let Some(title) = d.title {
            t.title = title.trim().to_string();
        }
        if t.title.is_empty() {
            return Err(crate::tickets::title_missing());
        }
        if let Some(v) = d.description {
            t.description = v.trim().to_string();
        }
        if let Some(v) = d.criteria {
            t.criteria = v
                .iter()
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect();
        }
        if let Some(v) = d.max_loops {
            t.max_loops = crate::board::max_loops(v);
        }
        if let Some(v) = d.engine {
            t.engine = v;
        }
        if let Some(v) = d.model {
            t.model = v.trim().to_string();
        }
        if let Some(v) = d.effort {
            t.effort = v.trim().to_string();
        }
        if let Some(v) = d.catch_up {
            t.catch_up = v;
        }
        if let Some(rhythm) = d.rhythm {
            if rhythm != t.rhythm {
                // A new rhythm counts from now: none of its past due times runs.
                t.rhythm = rhythm;
                t.since = now;
            }
        }
        self.scheduled_preview(&t.rhythm)?;
        let project = self.project(&t.project_id)?;
        let engine = t.engine.unwrap_or(project.engine());
        if let Some(account) = d.account {
            t.account = self.ticket_account(&project, engine, Some(&account))?;
        }
        if let Some(paused) = d.paused {
            if t.paused && !paused {
                // Resumed: the due times of the pause are over.
                t.since = now;
            }
            t.paused = paused;
        }
        Ok(())
    }

    /// A new scheduled task of the project: its first due time is the next one from now.
    pub fn scheduled_create(&self, project_id: &str, d: ScheduledDraft) -> Result<ScheduledView> {
        let project = self.project(project_id)?;
        let now = self.schedule_clock.now();
        let mut t = ScheduledTask {
            id: new_id(),
            project_id: project.id.clone(),
            max_loops: crate::board::max_loops(0),
            catch_up: true,
            created_at: now,
            since: now,
            ..Default::default()
        };
        if d.rhythm.is_none() {
            bail!(tr!(
                "Il manque le rythme de la tâche.",
                "The task's rhythm is missing."
            ));
        }
        self.scheduled_apply(&mut t, d)?;
        let id = t.id.clone();
        {
            // Checked again under the lock: a project closed meanwhile takes no task.
            let projects = self.projects.read();
            if !projects.iter().any(|p| p.id == project_id) {
                return Err(project_not_found());
            }
            self.scheduled.write().push(t);
        }
        log::info!("scheduled task {id} made in project {project_id}");
        self.scheduled_changed();
        self.scheduled_view(&id)
    }

    /// The task changed: its rhythm counts from now when it is another one.
    pub fn scheduled_update(&self, id: &str, d: ScheduledDraft) -> Result<ScheduledView> {
        let mut t = self
            .scheduled
            .read()
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(task_not_found)?;
        self.scheduled_apply(&mut t, d)?;
        {
            let mut all = self.scheduled.write();
            let slot = all
                .iter_mut()
                .find(|x| x.id == id)
                .ok_or_else(task_not_found)?;
            // What a due time wrote meanwhile stays.
            t.last_due = slot.last_due;
            t.last_ticket = slot.last_ticket.clone();
            t.last_run_at = slot.last_run_at;
            t.last_note = slot.last_note.clone();
            *slot = t;
        }
        self.scheduled_changed();
        self.scheduled_view(id)
    }

    /// Paused, or resumed: once resumed, the due times of the pause are over.
    pub fn scheduled_set_paused(&self, id: &str, paused: bool) -> Result<ScheduledView> {
        self.scheduled_update(
            id,
            ScheduledDraft {
                paused: Some(paused),
                ..Default::default()
            },
        )
    }

    /// The task goes; the tickets it made stay.
    pub fn scheduled_delete(&self, id: &str) -> Result<()> {
        {
            let mut all = self.scheduled.write();
            let before = all.len();
            all.retain(|t| t.id != id);
            if all.len() == before {
                return Err(task_not_found());
            }
        }
        self.scheduled_changed();
        Ok(())
    }

    /// A project gone for good takes its tasks with it.
    pub(crate) fn scheduled_forget_project(&self, project_id: &str) {
        let had = {
            let mut all = self.scheduled.write();
            let before = all.len();
            all.retain(|t| t.project_id != project_id);
            all.len() != before
        };
        if had {
            self.scheduled_changed();
        }
    }

    /// One task's fields changed under the lock; false when it is gone.
    fn scheduled_edit(&self, id: &str, f: impl FnOnce(&mut ScheduledTask)) -> bool {
        let mut all = self.scheduled.write();
        match all.iter_mut().find(|t| t.id == id) {
            Some(t) => {
                f(t);
                true
            }
            None => false,
        }
    }

    /// The due times that came, task by task: each makes its ticket, or passes its turn. Gives
    /// the tickets made. One look at a time.
    pub async fn scheduled_tick(self: &Arc<Self>) -> Vec<String> {
        let _one = self.scheduled_lock.lock().await;
        let now = self.schedule_clock.now();
        let started = self.schedule_clock.started();
        let tasks = self.scheduled.read().clone();
        let projects: HashSet<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let mut changed = false;
        let mut made = Vec::new();
        for task in tasks {
            // A project closed keeps its tasks with its history: they wait for its return.
            if !projects.contains(&task.project_id) {
                continue;
            }
            let busy = running(&self.tickets.read(), &task);
            match decide(&task, now, started, busy, &Local) {
                Due::Nothing => {}
                Due::Pass(at, why) => {
                    log::info!("scheduled task {}: due time {at} passed ({why:?})", task.id);
                    changed |= self.scheduled_edit(&task.id, |t| {
                        t.last_due = Some(at);
                        if let Some(note) = why.note() {
                            t.last_note = Some(note);
                        }
                    });
                }
                Due::Run(at) => {
                    // Written down and saved before the ticket is made: a stop in between loses
                    // this run, it never makes two.
                    if !self.scheduled_edit(&task.id, |t| {
                        t.last_due = Some(at);
                        t.last_run_at = Some(now);
                        t.last_note = None;
                    }) {
                        continue;
                    }
                    changed = true;
                    let core = self.clone();
                    let _ = tokio::task::spawn_blocking(move || core.save_now()).await;
                    match self.scheduled_run(&task).await {
                        Ok(ticket) => {
                            log::info!("scheduled task {}: ticket {} made", task.id, ticket.key);
                            self.overseer_scheduled(&task.title, &task.project_id, &ticket.key);
                            self.scheduled_edit(&task.id, |t| {
                                t.last_ticket = Some(ticket.id.clone())
                            });
                            made.push(ticket.id);
                        }
                        Err(e) => {
                            let why = crate::tickets::error_line(&e);
                            log::info!("scheduled task {}: no ticket made: {why}", task.id);
                            self.scheduled_edit(&task.id, |t| t.last_note = Some(why));
                        }
                    }
                }
            }
        }
        if changed {
            self.scheduled_changed();
        }
        made
    }

    /// The ticket of a run: the task's prompt, criteria, harness, account, model and effort,
    /// launched at once (`TicketDraft::scheduled`), never shared.
    async fn scheduled_run(self: &Arc<Self>, task: &ScheduledTask) -> Result<Ticket> {
        let draft = TicketDraft {
            title: task.title.clone(),
            description: task.description.clone(),
            criteria: task.criteria.clone(),
            max_loops: task.max_loops,
            engine: Some(task.engine),
            account: Some(task.account.clone()),
            model: Some(task.model.clone()),
            effort: Some(task.effort.clone()),
            shared: Some(false),
            scheduled: Some(task.id.clone()),
            ..Default::default()
        };
        self.ticket_create(&task.project_id, draft).await
    }
}
