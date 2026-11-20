//! The scheduled tasks (ESC-448): their rhythms and cron expressions, their next due times (summer
//! time included), what a due time does (run, catch up, pass its turn, never twice), and on the
//! core, with its clock set by the tests, the tickets they make and launch.

use crate::core_tests::{harness, Harness};
use crate::model::*;
use crate::scheduled::{
    cron_of, decide, last_until, next_after, parse, running, upcoming, Due, Pass, ScheduledDraft,
};
use chrono::{
    DateTime, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc,
};

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;

fn rhythm(kind: RhythmKind) -> Rhythm {
    Rhythm {
        kind,
        hour: 7,
        minute: 0,
        ..Default::default()
    }
}

fn cron(expr: &str) -> Rhythm {
    Rhythm {
        kind: RhythmKind::Cron,
        cron: expr.into(),
        ..Default::default()
    }
}

fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> i64 {
    Utc.with_ymd_and_hms(y, m, d, h, min, 0)
        .unwrap()
        .timestamp_millis()
}

/// A zone like Paris's in 2026: UTC+1, then UTC+2 from 29 March, 02:00 (which becomes 03:00).
#[derive(Debug, Clone, Copy)]
struct Paris;

fn switch_utc() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 3, 29)
        .unwrap()
        .and_hms_opt(1, 0, 0)
        .unwrap()
}

fn plus(h: i32) -> FixedOffset {
    FixedOffset::east_opt(h * 3600).unwrap()
}

impl TimeZone for Paris {
    type Offset = FixedOffset;

    fn from_offset(_: &FixedOffset) -> Self {
        Paris
    }

    #[allow(deprecated)]
    fn offset_from_local_date(&self, local: &NaiveDate) -> LocalResult<FixedOffset> {
        self.offset_from_local_datetime(&local.and_hms_opt(12, 0, 0).unwrap())
    }

    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<FixedOffset> {
        let gap = switch_utc() + chrono::Duration::hours(1);
        if *local < gap {
            LocalResult::Single(plus(1))
        } else if *local < gap + chrono::Duration::hours(1) {
            LocalResult::None
        } else {
            LocalResult::Single(plus(2))
        }
    }

    #[allow(deprecated)]
    fn offset_from_utc_date(&self, utc: &NaiveDate) -> FixedOffset {
        self.offset_from_utc_datetime(&utc.and_hms_opt(12, 0, 0).unwrap())
    }

    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> FixedOffset {
        if *utc < switch_utc() {
            plus(1)
        } else {
            plus(2)
        }
    }
}

fn task(r: Rhythm, since: i64) -> ScheduledTask {
    ScheduledTask {
        id: "s1".into(),
        project_id: "p1".into(),
        title: "Revue des dépendances".into(),
        rhythm: r,
        catch_up: true,
        since,
        created_at: since,
        ..Default::default()
    }
}

// ---------- rhythms ----------

#[test]
fn each_simple_rhythm_is_a_cron_expression() {
    assert_eq!(cron_of(&rhythm(RhythmKind::Daily)).unwrap(), "0 7 * * *");
    let weekly = Rhythm {
        days: vec![7, 1, 5, 1],
        minute: 30,
        ..rhythm(RhythmKind::Weekly)
    };
    // Sunday is cron's 0; the days in order, once each.
    assert_eq!(cron_of(&weekly).unwrap(), "30 7 * * 1,5,0");
    let monthly = Rhythm {
        day: 1,
        hour: 9,
        ..rhythm(RhythmKind::Monthly)
    };
    assert_eq!(cron_of(&monthly).unwrap(), "0 9 1 * *");
    assert_eq!(cron_of(&cron("  0  7 * *   1 ")).unwrap(), "0 7 * * 1");
}

#[test]
fn a_rhythm_that_is_none_is_refused() {
    let no_day = rhythm(RhythmKind::Weekly);
    assert!(parse(&no_day)
        .unwrap_err()
        .to_string()
        .contains("au moins un jour"));
    let late = Rhythm {
        hour: 24,
        ..rhythm(RhythmKind::Daily)
    };
    assert!(parse(&late).is_err());
    let day32 = Rhythm {
        day: 32,
        ..rhythm(RhythmKind::Monthly)
    };
    assert!(parse(&day32).is_err());
    assert!(parse(&cron(""))
        .unwrap_err()
        .to_string()
        .contains("expression cron"));
    // Five fields, no seconds and no years.
    assert!(parse(&cron("0 0 7 * * 1"))
        .unwrap_err()
        .to_string()
        .contains("cinq champs"));
    assert!(parse(&cron("0 25 * * *")).is_err());
    assert!(parse(&cron("lundi à 7 h")).is_err());
    assert!(parse(&cron("@daily")).is_err());
}

#[test]
fn the_next_due_times_follow_the_rhythm() {
    let tz = plus(1);
    let weekly = Rhythm {
        days: vec![1],
        ..rhythm(RhythmKind::Weekly)
    };
    let c = parse(&weekly).unwrap();
    // Friday 31 October 2026, noon in Paris: the next Mondays at 7:00 (6:00 UTC).
    let from = utc(2026, 10, 31, 11, 0);
    assert_eq!(
        upcoming(&c, from, 3, &tz),
        vec![
            utc(2026, 11, 2, 6, 0),
            utc(2026, 11, 9, 6, 0),
            utc(2026, 11, 16, 6, 0)
        ]
    );
    // On the due time itself, the next one is a week on; the last one is itself.
    let monday = utc(2026, 11, 2, 6, 0);
    assert_eq!(next_after(&c, monday, &tz), Some(monday + 7 * DAY));
    assert_eq!(last_until(&c, monday, &tz), Some(monday));
    assert_eq!(last_until(&c, monday + MIN, &tz), Some(monday));
    // The 31st of each month skips the months without one.
    let c = parse(&Rhythm {
        day: 31,
        ..rhythm(RhythmKind::Monthly)
    })
    .unwrap();
    assert_eq!(
        upcoming(&c, utc(2026, 11, 1, 0, 0), 2, &Utc),
        vec![utc(2026, 12, 31, 7, 0), utc(2027, 1, 31, 7, 0)]
    );
}

#[test]
fn a_daily_task_keeps_its_hour_across_summer_time() {
    let c = parse(&rhythm(RhythmKind::Daily)).unwrap();
    let from = utc(2026, 3, 27, 12, 0);
    // 7:00 in Paris: 6:00 UTC before the change, 5:00 UTC after it.
    assert_eq!(
        upcoming(&c, from, 3, &Paris),
        vec![
            utc(2026, 3, 28, 6, 0),
            utc(2026, 3, 29, 5, 0),
            utc(2026, 3, 30, 5, 0)
        ]
    );
    let shown: Vec<u32> = upcoming(&c, from, 3, &Paris)
        .iter()
        .map(|ms| {
            DateTime::from_timestamp_millis(*ms)
                .unwrap()
                .with_timezone(&Paris)
                .hour()
        })
        .collect();
    assert_eq!(shown, vec![7, 7, 7]);
}

// ---------- what a due time does ----------

#[test]
fn a_due_time_runs_once_and_never_again() {
    let since = utc(2026, 11, 2, 5, 0);
    let mut t = task(cron("0 7 * * *"), since);
    let started = since;
    // Before it: nothing.
    assert_eq!(
        decide(&t, utc(2026, 11, 2, 6, 59), started, false, &Utc),
        Due::Nothing
    );
    let due = utc(2026, 11, 2, 7, 0);
    assert_eq!(decide(&t, due, started, false, &Utc), Due::Run(due));
    assert_eq!(
        decide(&t, due + 30 * MIN, started, false, &Utc),
        Due::Run(due)
    );
    // Dealt with: never again, at any later look before the next one.
    t.last_due = Some(due);
    assert_eq!(decide(&t, due, started, false, &Utc), Due::Nothing);
    assert_eq!(
        decide(&t, due + 23 * HOUR, started, false, &Utc),
        Due::Nothing
    );
    assert_eq!(
        decide(&t, due + DAY, started, false, &Utc),
        Due::Run(due + DAY)
    );
}

#[test]
fn a_due_time_before_the_task_was_made_or_changed_never_runs() {
    let since = utc(2026, 11, 2, 7, 30);
    let t = task(cron("0 7 * * *"), since);
    assert_eq!(
        decide(&t, utc(2026, 11, 2, 8, 0), since, false, &Utc),
        Due::Nothing
    );
}

#[test]
fn several_due_times_missed_while_closed_give_one_run_at_the_return() {
    let since = utc(2026, 11, 2, 5, 0);
    let t = task(cron("0 * * * *"), since);
    // Closed from 5:00 to 14:20: nine due times missed, one run, for the last of them.
    let back = utc(2026, 11, 2, 14, 20);
    assert_eq!(
        decide(&t, back, back, false, &Utc),
        Due::Run(utc(2026, 11, 2, 14, 0))
    );
}

#[test]
fn without_catch_up_a_due_time_missed_while_closed_waits_for_the_next() {
    let since = utc(2026, 11, 2, 5, 0);
    let t = ScheduledTask {
        catch_up: false,
        ..task(cron("0 * * * *"), since)
    };
    let back = utc(2026, 11, 2, 14, 20);
    assert_eq!(
        decide(&t, back, back, false, &Utc),
        Due::Pass(utc(2026, 11, 2, 14, 0), Pass::Missed)
    );
    // A due time that falls while Escouade runs runs.
    let next = utc(2026, 11, 2, 15, 0);
    let t = ScheduledTask {
        last_due: Some(utc(2026, 11, 2, 14, 0)),
        ..t
    };
    assert_eq!(decide(&t, next + MIN, back, false, &Utc), Due::Run(next));
}

#[test]
fn a_due_time_passes_its_turn_while_the_ticket_before_runs_or_the_task_is_paused() {
    let since = utc(2026, 11, 2, 5, 0);
    let t = task(cron("0 7 * * *"), since);
    let due = utc(2026, 11, 2, 7, 0);
    assert_eq!(
        decide(&t, due, since, true, &Utc),
        Due::Pass(due, Pass::Running)
    );
    let paused = ScheduledTask { paused: true, ..t };
    assert_eq!(
        decide(&paused, due, since, false, &Utc),
        Due::Pass(due, Pass::Paused)
    );
}

#[test]
fn only_a_ticket_to_do_or_under_way_holds_the_next_run_back() {
    let t = ScheduledTask {
        last_ticket: Some("k1".into()),
        ..task(cron("0 7 * * *"), 0)
    };
    let of = |column| {
        vec![Ticket {
            id: "k1".into(),
            column,
            ..Default::default()
        }]
    };
    assert!(running(&of(Column::Todo), &t));
    assert!(running(&of(Column::Doing), &t));
    // « À tester » counts as done, and what comes after it too.
    assert!(!running(&of(Column::Review), &t));
    assert!(!running(&of(Column::Pr), &t));
    assert!(!running(&of(Column::Done), &t));
    // Its ticket deleted: nothing holds it back.
    assert!(!running(&[], &t));
}

#[test]
fn the_tasks_are_saved_with_the_state_and_their_tickets_say_whose_they_are() {
    let state = PersistedState {
        scheduled: vec![task(rhythm(RhythmKind::Daily), 5)],
        tickets: vec![Ticket {
            id: "k1".into(),
            scheduled: Some("s1".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let saved = serde_json::to_value(&state).unwrap();
    assert_eq!(saved["scheduled"][0]["rhythm"]["kind"], "daily");
    assert_eq!(saved["scheduled"][0]["catchUp"], true);
    assert_eq!(saved["tickets"][0]["scheduled"], "s1");
    let back: PersistedState = serde_json::from_value(saved).unwrap();
    assert_eq!(back.scheduled, state.scheduled);
    // Another ticket says nothing of it; a state saved before has none.
    let other = serde_json::to_value(Ticket::default()).unwrap();
    assert!(other.get("scheduled").is_none());
    let old: PersistedState = serde_json::from_str("{}").unwrap();
    assert!(old.scheduled.is_empty());
}

// ---------- on the core, with its clock ----------

/// A draft of a task of the rhythm `r`, titled `title` (the fake agent plays it as it says).
fn draft(title: &str, r: Rhythm) -> ScheduledDraft {
    ScheduledDraft {
        title: Some(title.into()),
        description: Some("Mets à jour les dépendances et lance les tests.".into()),
        criteria: Some(vec!["Les tests passent".into(), " ".into()]),
        max_loops: Some(3),
        model: Some("sonnet".into()),
        effort: Some("high".into()),
        rhythm: Some(r),
        ..Default::default()
    }
}

impl Harness {
    /// The scheduled clock at `ms`, the app started long before.
    fn clock_at(&self, ms: i64) {
        self.core.schedule_clock.set(ms);
    }

    fn tickets_of_task(&self, task_id: &str) -> Vec<Ticket> {
        self.core
            .tickets
            .read()
            .iter()
            .filter(|t| t.scheduled.as_deref() == Some(task_id))
            .cloned()
            .collect()
    }

    fn task(&self, id: &str) -> ScheduledTask {
        self.core.scheduled_view(id).unwrap().task
    }
}

/// A moment 30 s past a minute, an hour after the real now: a look at it never crosses a minute.
fn half_minute() -> i64 {
    let now = now_ms() + HOUR;
    now - now.rem_euclid(MIN) + 30_000
}

#[tokio::test]
async fn a_task_makes_and_launches_its_ticket_at_its_due_time() {
    let h = harness("sch-run");
    let (p, _) = h.project(false).await;
    // The autopilot off: the ticket starts all the same, as one launched by hand.
    h.set_board(&p.id, |s| s.autopilot = false);
    let t0 = half_minute();
    h.clock_at(t0);
    let task = h
        .core
        .scheduled_create(&p.id, draft("Revue [ok]", cron("* * * * *")))
        .unwrap();
    assert_eq!(task.next, Some(t0 + 30_000));
    assert!(task.task.catch_up);
    // Not yet.
    assert!(h.core.scheduled_tick().await.is_empty());
    h.clock_at(t0 + MIN);
    let made = h.core.scheduled_tick().await;
    assert_eq!(made.len(), 1);
    let ticket = h.ticket(&made[0]);
    assert_eq!(ticket.scheduled.as_deref(), Some(task.task.id.as_str()));
    assert_eq!(ticket.title, "Revue [ok]");
    assert_eq!(
        ticket.description,
        "Mets à jour les dépendances et lance les tests."
    );
    assert_eq!(ticket.criteria.len(), 1);
    assert_eq!(
        (ticket.model.as_str(), ticket.effort.as_str()),
        ("sonnet", "high")
    );
    assert_eq!(ticket.max_loops, 3);
    assert!(ticket.shared.is_none());
    let after = h.task(&task.task.id);
    assert_eq!(after.last_ticket.as_deref(), Some(ticket.id.as_str()));
    assert_eq!(after.last_due, Some(t0 + 30_000));
    // Launched: it goes to « À tester » though the autopilot is off.
    h.wait_ticket(&ticket.id, "to test", |t| t.column == Column::Review)
        .await;
    // The window was told of the task.
    assert!(h.events.lock().iter().any(|e| e["type"] == "scheduled"));
}

#[tokio::test]
async fn a_due_time_never_makes_two_tickets_and_passes_its_turn_while_the_one_before_runs() {
    let h = harness("sch-once");
    let (p, _) = h.project(false).await;
    let t0 = half_minute();
    h.clock_at(t0);
    let task = h
        .core
        .scheduled_create(&p.id, draft("Longue revue [lent]", cron("* * * * *")))
        .unwrap();
    let id = task.task.id;
    h.clock_at(t0 + MIN);
    // Two looks at once: one ticket.
    let (a, b) = tokio::join!(h.core.scheduled_tick(), h.core.scheduled_tick());
    assert_eq!(a.len() + b.len(), 1);
    assert!(h.core.scheduled_tick().await.is_empty());
    assert_eq!(h.tickets_of_task(&id).len(), 1);
    // The next due time comes while that ticket is « À faire » or « En cours »: it passes.
    h.clock_at(t0 + 2 * MIN);
    assert!(h.core.scheduled_tick().await.is_empty());
    assert_eq!(h.tickets_of_task(&id).len(), 1);
    let after = h.task(&id);
    assert_eq!(after.last_due, Some(t0 + 90_000));
    assert!(after.last_note.unwrap().contains("passé son tour"));
}

#[tokio::test]
async fn at_its_return_a_task_catches_up_once_or_waits_as_it_is_set() {
    let h = harness("sch-return");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t0 = half_minute();
    h.clock_at(t0);
    let caught = h
        .core
        .scheduled_create(&p.id, draft("Rattrape [ok]", cron("0 * * * *")))
        .unwrap()
        .task
        .id;
    let waits = h
        .core
        .scheduled_create(
            &p.id,
            ScheduledDraft {
                catch_up: Some(false),
                ..draft("Attend [ok]", cron("0 * * * *"))
            },
        )
        .unwrap()
        .task
        .id;
    // Escouade closed for five hours, back now.
    let back = t0 + 5 * HOUR;
    h.clock_at(back);
    h.core.schedule_clock.set_started(back);
    let made = h.core.scheduled_tick().await;
    assert_eq!(made.len(), 1);
    assert_eq!(h.tickets_of_task(&caught).len(), 1);
    assert!(h.tickets_of_task(&waits).is_empty());
    assert!(h.task(&waits).last_note.unwrap().contains("fermé"));
    // Nothing more until the next hour, which both run (the first ticket done by then).
    h.wait_ticket(&made[0], "to test", |t| t.column == Column::Review)
        .await;
    assert!(h.core.scheduled_tick().await.is_empty());
    h.clock_at(back + HOUR);
    assert_eq!(h.core.scheduled_tick().await.len(), 2);
    assert_eq!(h.tickets_of_task(&waits).len(), 1);
}

#[tokio::test]
async fn a_paused_task_lets_its_due_times_pass_and_counts_from_its_resume() {
    let h = harness("sch-pause");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t0 = half_minute();
    h.clock_at(t0);
    let id = h
        .core
        .scheduled_create(&p.id, draft("Pause [ok]", cron("* * * * *")))
        .unwrap()
        .task
        .id;
    let paused = h.core.scheduled_set_paused(&id, true).unwrap();
    assert_eq!(paused.next, None);
    h.clock_at(t0 + 3 * MIN);
    assert!(h.core.scheduled_tick().await.is_empty());
    // Resumed: what fell during the pause is over, the next due time runs.
    h.core.scheduled_set_paused(&id, false).unwrap();
    assert!(h.core.scheduled_tick().await.is_empty());
    h.clock_at(t0 + 4 * MIN);
    assert_eq!(h.core.scheduled_tick().await.len(), 1);
}

#[tokio::test]
async fn a_task_is_checked_changed_and_deleted() {
    let h = harness("sch-edit");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t0 = half_minute();
    h.clock_at(t0);
    // Refused: no title, no rhythm, a rhythm that is none, one that never comes, a project gone.
    assert!(h
        .core
        .scheduled_create(&p.id, draft("  ", cron("* * * * *")))
        .is_err());
    assert!(h
        .core
        .scheduled_create(
            &p.id,
            ScheduledDraft {
                rhythm: None,
                ..draft("Sans rythme", cron(""))
            }
        )
        .is_err());
    assert!(h
        .core
        .scheduled_create(&p.id, draft("Jamais", cron("0 0 31 2 *")))
        .is_err());
    assert!(h
        .core
        .scheduled_create(&p.id, draft("Bancale", rhythm(RhythmKind::Weekly)))
        .is_err());
    assert!(h
        .core
        .scheduled_create("nope", draft("Ailleurs", cron("* * * * *")))
        .is_err());
    assert_eq!(
        h.core.scheduled_preview(&cron("* * * * *")).unwrap(),
        vec![t0 + 30_000, t0 + 90_000, t0 + 150_000]
    );
    let id = h
        .core
        .scheduled_create(&p.id, draft("Rapport", cron("0 * * * *")))
        .unwrap()
        .task
        .id;
    // Its rhythm changed after a due time of the new one: that one does not run.
    h.clock_at(t0 + 2 * MIN);
    let changed = h
        .core
        .scheduled_update(
            &id,
            ScheduledDraft {
                rhythm: Some(cron("* * * * *")),
                title: Some("Rapport du soir".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(changed.task.title, "Rapport du soir");
    assert_eq!(
        changed.task.description,
        "Mets à jour les dépendances et lance les tests."
    );
    assert!(h.core.scheduled_tick().await.is_empty());
    h.core.scheduled_delete(&id).unwrap();
    assert!(h.core.scheduled_view(&id).is_err());
    assert!(h.core.scheduled_delete(&id).is_err());
    h.clock_at(t0 + 10 * MIN);
    assert!(h.core.scheduled_tick().await.is_empty());
}

#[tokio::test]
async fn a_scheduled_ticket_waits_for_a_place_of_the_board_as_one_launched_by_hand() {
    let h = harness("sch-place");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.max_parallel = 1);
    let busy = h
        .core
        .ticket_create(&p.id, crate::tickets_tests::draft("Occupe [lent]", &[], 5))
        .await
        .unwrap();
    h.wait_ticket(&busy.id, "under way", |t| t.column == Column::Doing)
        .await;
    let t0 = half_minute();
    h.clock_at(t0);
    let id = h
        .core
        .scheduled_create(&p.id, draft("Revue [ok]", cron("* * * * *")))
        .unwrap()
        .task
        .id;
    h.clock_at(t0 + MIN);
    let made = h.core.scheduled_tick().await;
    assert_eq!(made.len(), 1);
    // Launched, but the only place is taken: it waits in « À faire ».
    h.core.schedule_now().await;
    let t = h.ticket(&made[0]);
    assert_eq!((t.column, t.forced), (Column::Todo, true));
    assert_eq!(h.tickets_of_task(&id).len(), 1);
}

/// Sets the switch of `f` (F-1) as Réglages › Fonctionnalités does, without the save's side effects.
fn switch(h: &Harness, f: crate::features::Feature, on: bool) {
    h.core
        .settings
        .write()
        .features
        .get_or_insert_with(Default::default)
        .set(f, on);
}

#[tokio::test]
async fn the_feature_off_its_tasks_no_longer_run_and_come_back_with_it_kept() {
    use crate::features::Feature;
    let h = harness("sch-feature-off");
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let t0 = half_minute();
    h.clock_at(t0);
    let task = h
        .core
        .scheduled_create(&p.id, draft("Revue [ok]", cron("* * * * *")))
        .unwrap();
    let id = task.task.id.clone();
    // Off: its due time passes, no ticket made, said on its row.
    switch(&h, Feature::Scheduled, false);
    h.clock_at(t0 + MIN);
    assert!(h.core.scheduled_tick().await.is_empty());
    assert!(h.tickets_of_task(&id).is_empty());
    let passed = h.task(&id);
    assert_eq!(passed.last_due, Some(t0 + 30_000));
    assert!(passed.last_note.unwrap().contains("Tâches programmées"));
    // With the Kanban off too: its runs are tickets.
    switch(&h, Feature::Scheduled, true);
    switch(&h, Feature::Kanban, false);
    h.clock_at(t0 + 2 * MIN);
    assert!(h.core.scheduled_tick().await.is_empty());
    // On again: the task is still there and runs at its next due time, not for those that passed.
    switch(&h, Feature::Kanban, true);
    assert_eq!(h.core.scheduled_view(&id).unwrap().task.title, "Revue [ok]");
    h.clock_at(t0 + 3 * MIN);
    assert_eq!(h.core.scheduled_tick().await.len(), 1);
    assert_eq!(h.tickets_of_task(&id).len(), 1);
}
