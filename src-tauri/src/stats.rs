//! Token / cost statistics of the agents launched by the app (SQLite).

use crate::agent::TurnRow;
use crate::i18n::Lang;
use crate::model::now_ms;
use chrono::{Datelike, Duration, Local, Months, NaiveDate, TimeZone};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
CREATE TABLE IF NOT EXISTS turns (
  id INTEGER PRIMARY KEY, ts INTEGER NOT NULL, agent_id TEXT NOT NULL, project_id TEXT NOT NULL,
  model TEXT NOT NULL, input INTEGER NOT NULL, cache INTEGER NOT NULL, output INTEGER NOT NULL, cost REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS turns_ts ON turns(ts);
CREATE TABLE IF NOT EXISTS prompts (
  id INTEGER PRIMARY KEY, ts INTEGER NOT NULL, agent_id TEXT NOT NULL, project_id TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS prompts_ts ON prompts(ts);
";

pub struct Stats {
    conn: Mutex<Option<Connection>>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub label: String,
    pub start: i64,
    pub input: u64,
    pub cache: u64,
    pub output: u64,
    pub cost: f64,
    pub prompts: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub key: String,
    pub tokens: u64,
    pub cost: f64,
}

/// What an agent used over the period, archived or not.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentShare {
    pub agent_id: String,
    /// None for an agent that is gone (deleted): only its turns remain.
    pub name: Option<String>,
    pub project_id: String,
    pub tokens: u64,
    pub cost: f64,
}

/// What the agents of a ticket used over the period, added up.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TicketShare {
    pub id: String,
    pub key: String,
    pub title: String,
    pub loops: u32,
    pub cost: f64,
}

/// What the turns table does not keep of an agent: its name, and the ticket it works or worked on.
#[derive(Debug, Clone, Default)]
pub struct AgentLabel {
    pub name: String,
    pub ticket_id: Option<String>,
}

/// What a ticket is listed under in the statistics.
#[derive(Debug, Clone, Default)]
pub struct TicketLabel {
    pub key: String,
    pub title: String,
    pub loops: u32,
}

/// The names the statistics read in the app's state, by id; the turns table only has ids.
#[derive(Debug, Clone, Default)]
pub struct Labels {
    pub agents: HashMap<String, AgentLabel>,
    pub tickets: HashMap<String, TicketLabel>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatsView {
    pub range: String,
    pub buckets: Vec<Bucket>,
    pub tokens: u64,
    pub tokens_prev: u64,
    pub cost: f64,
    pub cost_all: f64,
    pub first_ts: Option<i64>,
    pub prompts: u64,
    pub by_project: Vec<Share>,
    pub by_model: Vec<Share>,
    pub by_agent: Vec<AgentShare>,
    pub by_ticket: Vec<TicketShare>,
}

impl Stats {
    pub fn open(path: &Path) -> Self {
        let conn = Connection::open(path).and_then(|c| c.execute_batch(SCHEMA).map(|_| c));
        match conn {
            Ok(c) => Self {
                conn: Mutex::new(Some(c)),
            },
            Err(e) => {
                log::error!("stats database unavailable: {e}");
                Self {
                    conn: Mutex::new(None),
                }
            }
        }
    }

    #[cfg(test)]
    fn memory() -> Self {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(SCHEMA).unwrap();
        Self {
            conn: Mutex::new(Some(c)),
        }
    }

    pub fn record_turns(&self, agent_id: &str, project_id: &str, rows: &[TurnRow]) {
        self.record_turns_at(now_ms(), agent_id, project_id, rows);
    }

    fn record_turns_at(&self, ts: i64, agent_id: &str, project_id: &str, rows: &[TurnRow]) {
        let guard = self.conn.lock();
        let Some(c) = guard.as_ref() else { return };
        for r in rows {
            let res = c.execute(
                "INSERT INTO turns (ts, agent_id, project_id, model, input, cache, output, cost) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![ts, agent_id, project_id, r.model, r.input as i64, r.cache as i64, r.output as i64, r.cost],
            );
            if let Err(e) = res {
                log::warn!("stats insert failed: {e}");
            }
        }
    }

    pub fn record_prompt(&self, agent_id: &str, project_id: &str) {
        let guard = self.conn.lock();
        if let Some(c) = guard.as_ref() {
            let _ = c.execute(
                "INSERT INTO prompts (ts, agent_id, project_id) VALUES (?1,?2,?3)",
                params![now_ms(), agent_id, project_id],
            );
        }
    }

    pub fn today_cost(&self) -> f64 {
        let start = local_ms(Local::now().date_naive());
        let guard = self.conn.lock();
        let Some(c) = guard.as_ref() else { return 0.0 };
        c.query_row(
            "SELECT COALESCE(SUM(cost),0) FROM turns WHERE ts >= ?1",
            [start],
            |r| r.get(0),
        )
        .unwrap_or(0.0)
    }

    pub fn query(&self, range: &str, names: &Labels) -> StatsView {
        self.query_at(range, Local::now().date_naive(), names)
    }

    fn query_at(&self, range: &str, today: NaiveDate, names: &Labels) -> StatsView {
        let (starts, labels, prev_start) = bucket_bounds(range, today, crate::i18n::ui());
        let end = match range {
            "week" => local_ms(*starts.last().unwrap() + Duration::weeks(1)),
            "month" => local_ms(*starts.last().unwrap() + Months::new(1)),
            _ => local_ms(today + Duration::days(1)),
        };
        let bounds: Vec<i64> = starts.iter().map(|d| local_ms(*d)).collect();
        let first = bounds[0];
        let prev = local_ms(prev_start);
        let mut view = StatsView {
            range: range.to_string(),
            buckets: bounds
                .iter()
                .zip(labels)
                .map(|(s, l)| Bucket {
                    label: l,
                    start: *s,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let guard = self.conn.lock();
        let Some(c) = guard.as_ref() else { return view };

        let (mut by_project, mut by_model): (HashMap<String, Share>, HashMap<String, Share>) =
            Default::default();
        let mut by_agent: HashMap<String, AgentShare> = HashMap::new();
        if let Ok(mut stmt) = c.prepare("SELECT ts, project_id, model, input, cache, output, cost, agent_id FROM turns WHERE ts >= ?1 AND ts < ?2") {
            let rows = stmt.query_map([prev, end], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, i64>(5)?, r.get::<_, f64>(6)?, r.get::<_, String>(7)?))
            });
            for (ts, project, model, input, cache, output, cost, agent) in rows.into_iter().flatten().flatten() {
                let tokens = (input + cache + output) as u64;
                if ts < first {
                    view.tokens_prev += tokens;
                    continue;
                }
                let i = bounds.partition_point(|b| *b <= ts).saturating_sub(1);
                let b = &mut view.buckets[i];
                b.input += input as u64;
                b.cache += cache as u64;
                b.output += output as u64;
                b.cost += cost;
                view.tokens += tokens;
                view.cost += cost;
                let a = by_agent.entry(agent.clone()).or_insert_with(|| AgentShare {
                    agent_id: agent,
                    project_id: project.clone(),
                    ..Default::default()
                });
                a.tokens += tokens;
                a.cost += cost;
                for (map, key) in [(&mut by_project, project), (&mut by_model, model)] {
                    let s = map.entry(key.clone()).or_insert_with(|| Share { key, ..Default::default() });
                    s.tokens += tokens;
                    s.cost += cost;
                }
            }
        }
        if let Ok(mut stmt) = c.prepare("SELECT ts FROM prompts WHERE ts >= ?1 AND ts < ?2") {
            let rows = stmt.query_map([first, end], |r| r.get::<_, i64>(0));
            for ts in rows.into_iter().flatten().flatten() {
                let i = bounds.partition_point(|b| *b <= ts).saturating_sub(1);
                view.buckets[i].prompts += 1;
                view.prompts += 1;
            }
        }
        let (cost_all, first_ts): (f64, Option<i64>) = c
            .query_row(
                "SELECT COALESCE(SUM(cost),0), MIN(ts) FROM turns",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((0.0, None));
        view.cost_all = cost_all;
        view.first_ts = first_ts;
        let sort = |m: HashMap<String, Share>| {
            let mut v: Vec<Share> = m.into_values().collect();
            v.sort_by_key(|s| std::cmp::Reverse(s.tokens));
            v
        };
        view.by_project = sort(by_project);
        view.by_model = sort(by_model);
        (view.by_agent, view.by_ticket) = agent_lists(by_agent, names);
        view
    }
}

/// The agents of the period and the tickets they worked on, dearest first. An agent the app no
/// longer knows keeps its line (its turns remain) without a name nor a ticket; a ticket that is
/// gone is no line either, its agents are listed on their own.
fn agent_lists(
    by_agent: HashMap<String, AgentShare>,
    names: &Labels,
) -> (Vec<AgentShare>, Vec<TicketShare>) {
    let mut agents: Vec<AgentShare> = by_agent.into_values().collect();
    // The id breaks ties, so that two agents of the same cost keep their place from one query to
    // the next.
    agents.sort_by(|a, b| {
        b.cost
            .total_cmp(&a.cost)
            .then_with(|| a.agent_id.cmp(&b.agent_id))
    });
    let mut tickets: HashMap<&str, TicketShare> = HashMap::new();
    for a in &mut agents {
        let Some(label) = names.agents.get(&a.agent_id) else {
            continue;
        };
        a.name = Some(label.name.clone());
        let Some((id, t)) = label
            .ticket_id
            .as_deref()
            .and_then(|id| names.tickets.get(id).map(|t| (id, t)))
        else {
            continue;
        };
        tickets
            .entry(id)
            .or_insert_with(|| TicketShare {
                id: id.to_string(),
                key: t.key.clone(),
                title: t.title.clone(),
                loops: t.loops,
                cost: 0.0,
            })
            .cost += a.cost;
    }
    let mut tickets: Vec<TicketShare> = tickets.into_values().collect();
    tickets.sort_by(|a, b| b.cost.total_cmp(&a.cost).then_with(|| a.key.cmp(&b.key)));
    (agents, tickets)
}

fn local_ms(d: NaiveDate) -> i64 {
    let naive = d.and_hms_opt(0, 0, 0).expect("midnight");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|t| t.timestamp_millis())
        .unwrap_or_else(|| naive.and_utc().timestamp_millis())
}

/// The short names of the months, as the chart writes them under its bars.
const MONTHS_FR: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];
const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn month_name(lang: Lang, d: NaiveDate) -> &'static str {
    let names = match lang {
        Lang::Fr => &MONTHS_FR,
        Lang::En => &MONTHS_EN,
    };
    names[d.month0() as usize]
}

/// Bucket start dates, labels in `lang` (« 27/09 », « S39 », « sept. » ; “Sep 27”, “W39”, “Sep”)
/// and the start of the previous (comparison) period.
fn bucket_bounds(
    range: &str,
    today: NaiveDate,
    lang: Lang,
) -> (Vec<NaiveDate>, Vec<String>, NaiveDate) {
    match range {
        "week" => {
            let monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
            let starts: Vec<NaiveDate> =
                (0..12).map(|i| monday - Duration::weeks(11 - i)).collect();
            let labels = starts
                .iter()
                .map(|d| tr_in!(lang, "S{w}", "W{w}", w = d.iso_week().week()))
                .collect();
            let prev = starts[0] - Duration::weeks(12);
            (starts, labels, prev)
        }
        "month" => {
            let first = today.with_day(1).expect("first of month");
            let starts: Vec<NaiveDate> = (0..12).map(|i| first - Months::new(11 - i)).collect();
            let labels = starts
                .iter()
                .map(|d| month_name(lang, *d).to_string())
                .collect();
            let prev = starts[0] - Months::new(12);
            (starts, labels, prev)
        }
        _ => {
            let starts: Vec<NaiveDate> = (0..14).map(|i| today - Duration::days(13 - i)).collect();
            let labels = starts
                .iter()
                .map(|d| match lang {
                    Lang::Fr => format!("{:02}/{:02}", d.day(), d.month()),
                    Lang::En => format!("{} {}", month_name(lang, *d), d.day()),
                })
                .collect();
            let prev = starts[0] - Duration::days(14);
            (starts, labels, prev)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(model: &str, input: u64, cost: f64) -> TurnRow {
        TurnRow {
            model: model.into(),
            input,
            cache: 100,
            output: 10,
            cost,
        }
    }

    #[test]
    fn buckets_and_shares() {
        let s = Stats::memory();
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let noon = |d: NaiveDate| local_ms(d) + 12 * 3600 * 1000;
        s.record_turns_at(noon(today), "a1", "p1", &[row("claude-opus-5-5", 50, 1.0)]);
        s.record_turns_at(
            noon(today - Duration::days(2)),
            "a2",
            "p2",
            &[row("claude-sonnet-5", 20, 0.25)],
        );
        s.record_turns_at(
            noon(today - Duration::days(20)),
            "a2",
            "p2",
            &[row("claude-sonnet-5", 5, 0.1)],
        );
        let v = s.query_at("day", today, &Labels::default());
        assert_eq!(v.buckets.len(), 14);
        assert_eq!(v.buckets[13].label, "27/09");
        assert_eq!(v.buckets[13].input, 50);
        assert_eq!(v.buckets[11].input, 20);
        assert_eq!(v.tokens, 160 + 130);
        assert_eq!(v.tokens_prev, 115);
        assert!((v.cost - 1.25).abs() < 1e-9);
        assert!((v.cost_all - 1.35).abs() < 1e-9);
        assert_eq!(v.by_project[0].key, "p1");
        assert_eq!(v.by_model.len(), 2);

        let m = s.query_at("month", today, &Labels::default());
        assert_eq!(m.buckets.last().unwrap().label, "sept.");
        assert_eq!(m.buckets[0].label, "oct.");
        let w = s.query_at("week", today, &Labels::default());
        assert_eq!(w.buckets.len(), 12);
    }

    #[test]
    fn the_periods_are_named_in_the_language_asked_for() {
        use crate::i18n::Lang::{En, Fr};
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let labels = |range, lang| bucket_bounds(range, today, lang).1;
        let days = labels("day", En);
        assert_eq!((days[0].as_str(), days[13].as_str()), ("Sep 14", "Sep 27"));
        assert_eq!(labels("day", Fr)[13], "27/09");
        let months = labels("month", En);
        assert_eq!(
            months,
            ["Oct", "Nov", "Dec", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep"]
        );
        assert_eq!(labels("month", Fr)[4], "févr.");
        // ISO weeks: 2026-09-27 is a Sunday, in week 39.
        assert_eq!(labels("week", En).last().unwrap(), "W39");
        assert_eq!(labels("week", Fr).last().unwrap(), "S39");
    }

    /// The names of agents `(id, name, ticket id)` and tickets `(id, key, title, loops)`.
    fn labels(
        agents: &[(&str, &str, Option<&str>)],
        tickets: &[(&str, &str, &str, u32)],
    ) -> Labels {
        Labels {
            agents: agents
                .iter()
                .map(|(id, name, ticket)| {
                    (
                        id.to_string(),
                        AgentLabel {
                            name: name.to_string(),
                            ticket_id: ticket.map(str::to_string),
                        },
                    )
                })
                .collect(),
            tickets: tickets
                .iter()
                .map(|(id, key, title, loops)| {
                    (
                        id.to_string(),
                        TicketLabel {
                            key: key.to_string(),
                            title: title.to_string(),
                            loops: *loops,
                        },
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn agents_of_the_period_are_listed_by_cost_with_their_name_and_project() {
        let s = Stats::memory();
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let noon = |d: NaiveDate| local_ms(d) + 12 * 3600 * 1000;
        // Two turns for a1 add up; a2 is cheaper; a3 is gone from the app (no name) but its
        // turns remain; a4 only worked in the previous period.
        s.record_turns_at(noon(today), "a1", "p1", &[row("claude-opus-5-5", 50, 1.0)]);
        s.record_turns_at(
            noon(today - Duration::days(1)),
            "a1",
            "p1",
            &[row("claude-opus-5-5", 10, 0.5)],
        );
        s.record_turns_at(
            noon(today - Duration::days(2)),
            "a2",
            "p2",
            &[row("claude-sonnet-5", 20, 0.25)],
        );
        s.record_turns_at(noon(today), "a3", "p2", &[row("claude-sonnet-5", 1, 3.0)]);
        s.record_turns_at(
            noon(today - Duration::days(20)),
            "a4",
            "p1",
            &[row("claude-sonnet-5", 5, 9.0)],
        );
        let names = labels(
            &[
                ("a1", "refacto-auth", None),
                ("a2", "api-docs", None),
                ("a4", "vieux", None),
            ],
            &[],
        );
        let v = s.query_at("day", today, &names);
        let share =
            |id: &str, name: Option<&str>, project: &str, tokens: u64, cost: f64| AgentShare {
                agent_id: id.into(),
                name: name.map(str::to_string),
                project_id: project.into(),
                tokens,
                cost,
            };
        assert_eq!(
            v.by_agent,
            vec![
                share("a3", None, "p2", 111, 3.0),
                share("a1", Some("refacto-auth"), "p1", 160 + 120, 1.5),
                share("a2", Some("api-docs"), "p2", 130, 0.25),
            ]
        );
    }

    #[test]
    fn tickets_add_up_the_cost_of_their_agents() {
        let s = Stats::memory();
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let noon = |d: NaiveDate| local_ms(d) + 12 * 3600 * 1000;
        let turn = |agent: &str, days_ago: i64, cost: f64| {
            s.record_turns_at(
                noon(today - Duration::days(days_ago)),
                agent,
                "p1",
                &[row("claude-sonnet-5", 10, cost)],
            );
        };
        // DEM-1 was resumed by a second agent after the first was archived: both count.
        turn("a1", 0, 1.0);
        turn("a2", 0, 0.5);
        turn("a3", 1, 2.0);
        // Not on a ticket, or on one that is gone: listed as agents, not as tickets.
        turn("a4", 0, 5.0);
        turn("a5", 0, 4.0);
        // DEM-3 only cost something in the previous period.
        turn("a6", 20, 7.0);
        let names = labels(
            &[
                ("a1", "premier", Some("t1")),
                ("a2", "reprise", Some("t1")),
                ("a3", "cache", Some("t2")),
                ("a4", "libre", None),
                ("a5", "orphelin", Some("deleted")),
                ("a6", "ancien", Some("t3")),
            ],
            &[
                ("t1", "DEM-1", "Ajouter le login", 3),
                ("t2", "DEM-2", "Corriger le cache", 1),
                ("t3", "DEM-3", "Écrire la doc", 2),
            ],
        );
        let v = s.query_at("day", today, &names);
        assert_eq!(
            v.by_ticket,
            vec![
                TicketShare {
                    id: "t2".into(),
                    key: "DEM-2".into(),
                    title: "Corriger le cache".into(),
                    loops: 1,
                    cost: 2.0
                },
                TicketShare {
                    id: "t1".into(),
                    key: "DEM-1".into(),
                    title: "Ajouter le login".into(),
                    loops: 3,
                    cost: 1.5
                },
            ]
        );
        assert_eq!(v.by_agent.len(), 5);
    }

    #[test]
    fn no_turns_means_no_agent_and_no_ticket() {
        let s = Stats::memory();
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let names = labels(
            &[("a1", "refacto-auth", Some("t1"))],
            &[("t1", "DEM-1", "x", 1)],
        );
        let v = s.query_at("day", today, &names);
        assert!(v.by_agent.is_empty() && v.by_ticket.is_empty());
    }
}
