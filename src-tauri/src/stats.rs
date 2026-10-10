//! Token / cost statistics of the agents launched by the app (SQLite).

use crate::agent::TurnRow;
use crate::i18n::Lang;
use crate::model::now_ms;
use chrono::{Datelike, Duration, Local, Months, NaiveDate, TimeZone};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
CREATE TABLE IF NOT EXISTS turns (
  id INTEGER PRIMARY KEY, ts INTEGER NOT NULL, agent_id TEXT NOT NULL, project_id TEXT NOT NULL,
  model TEXT NOT NULL, input INTEGER NOT NULL, cache INTEGER NOT NULL, output INTEGER NOT NULL, cost REAL NOT NULL,
  account TEXT NOT NULL DEFAULT 'principal'
);
CREATE INDEX IF NOT EXISTS turns_ts ON turns(ts);
CREATE TABLE IF NOT EXISTS prompts (
  id INTEGER PRIMARY KEY, ts INTEGER NOT NULL, agent_id TEXT NOT NULL, project_id TEXT NOT NULL,
  account TEXT NOT NULL DEFAULT 'principal'
);
CREATE INDEX IF NOT EXISTS prompts_ts ON prompts(ts);
";

/// What an older database lacks to be read as `SCHEMA` says. Before the Claude accounts (1.7) its
/// tables had no `account`: every line they hold was written on the only account there was,
/// Principal (`accounts::PRINCIPAL`, the column's default). A table that has the column is left
/// alone, so this is done once, and no line is lost: `ADD COLUMN` rewrites nothing.
fn migrate(c: &Connection) -> rusqlite::Result<()> {
    for table in ["turns", "prompts"] {
        let has_account: i64 = c.query_row(
            &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = 'account'"),
            [],
            |r| r.get(0),
        )?;
        if has_account == 0 {
            c.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN account TEXT NOT NULL DEFAULT 'principal'"
            ))?;
        }
    }
    Ok(())
}

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

/// What an account used over the period: its tokens and cost, how many turns it ran and for how
/// many agents.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountShare {
    pub account: String,
    pub tokens: u64,
    pub cost: f64,
    pub turns: u64,
    pub agents: u64,
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
    /// Every account side by side, whichever one the view is for.
    pub by_account: Vec<AccountShare>,
    /// Every account that ever ran a turn, whichever the period or the account asked for.
    pub accounts: Vec<String>,
}

impl Stats {
    pub fn open(path: &Path) -> Self {
        let conn = Connection::open(path).and_then(|c| {
            c.execute_batch(SCHEMA)?;
            migrate(&c)?;
            Ok(c)
        });
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
        migrate(&c).unwrap();
        Self {
            conn: Mutex::new(Some(c)),
        }
    }

    /// The turn of an agent, written on `account`: the one the agent has at that turn.
    pub fn record_turns(&self, account: &str, agent_id: &str, project_id: &str, rows: &[TurnRow]) {
        self.record_turns_at(now_ms(), account, agent_id, project_id, rows);
    }

    fn record_turns_at(
        &self,
        ts: i64,
        account: &str,
        agent_id: &str,
        project_id: &str,
        rows: &[TurnRow],
    ) {
        let guard = self.conn.lock();
        let Some(c) = guard.as_ref() else { return };
        for r in rows {
            let res = c.execute(
                "INSERT INTO turns (ts, agent_id, project_id, model, input, cache, output, cost, account) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![ts, agent_id, project_id, r.model, r.input as i64, r.cache as i64, r.output as i64, r.cost, account],
            );
            if let Err(e) = res {
                log::warn!("stats insert failed: {e}");
            }
        }
    }

    pub fn record_prompt(&self, account: &str, agent_id: &str, project_id: &str) {
        self.record_prompt_at(now_ms(), account, agent_id, project_id);
    }

    fn record_prompt_at(&self, ts: i64, account: &str, agent_id: &str, project_id: &str) {
        let guard = self.conn.lock();
        if let Some(c) = guard.as_ref() {
            let _ = c.execute(
                "INSERT INTO prompts (ts, agent_id, project_id, account) VALUES (?1,?2,?3,?4)",
                params![ts, agent_id, project_id, account],
            );
        }
    }

    /// What the turns cost today, by account (one that spent nothing is not there).
    pub fn today_by_account(&self) -> HashMap<String, f64> {
        let start = local_ms(Local::now().date_naive());
        let guard = self.conn.lock();
        let Some(c) = guard.as_ref() else {
            return HashMap::new();
        };
        let Ok(mut stmt) = c.prepare(
            "SELECT account, COALESCE(SUM(cost),0) FROM turns WHERE ts >= ?1 GROUP BY account",
        ) else {
            return HashMap::new();
        };
        let rows = stmt.query_map([start], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        });
        rows.into_iter().flatten().flatten().collect()
    }

    /// What the turns cost today, on every account.
    #[cfg(test)]
    pub fn today_cost(&self) -> f64 {
        self.today_by_account().values().sum()
    }

    /// The period's statistics: of every account, or of `account` alone (all views but `by_account`
    /// and `accounts`, which put the accounts side by side).
    pub fn query(&self, range: &str, account: Option<&str>, names: &Labels) -> StatsView {
        self.query_at(range, Local::now().date_naive(), account, names)
    }

    fn query_at(
        &self,
        range: &str,
        today: NaiveDate,
        account: Option<&str>,
        names: &Labels,
    ) -> StatsView {
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
        // The accounts side by side take every turn of the period, whichever account the view is
        // for. A turn is what a reply wrote together: the lines of one agent at one moment (one
        // per model).
        let mut by_account: HashMap<String, AccountShare> = HashMap::new();
        let mut turns_seen: HashSet<(String, i64)> = HashSet::new();
        let mut agents_seen: HashSet<(String, String)> = HashSet::new();
        if let Ok(mut stmt) = c.prepare("SELECT ts, project_id, model, input, cache, output, cost, agent_id, account FROM turns WHERE ts >= ?1 AND ts < ?2") {
            let rows = stmt.query_map([prev, end], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, i64>(5)?, r.get::<_, f64>(6)?, r.get::<_, String>(7)?, r.get::<_, String>(8)?))
            });
            for (ts, project, model, input, cache, output, cost, agent, line_account) in rows.into_iter().flatten().flatten() {
                let tokens = (input + cache + output) as u64;
                let wanted = account.is_none_or(|a| a == line_account);
                if ts < first {
                    if wanted {
                        view.tokens_prev += tokens;
                    }
                    continue;
                }
                let share = by_account.entry(line_account.clone()).or_insert_with(|| AccountShare {
                    account: line_account.clone(),
                    ..Default::default()
                });
                share.tokens += tokens;
                share.cost += cost;
                share.turns += u64::from(turns_seen.insert((agent.clone(), ts)));
                share.agents += u64::from(agents_seen.insert((line_account, agent.clone())));
                if !wanted {
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
        if let Ok(mut stmt) = c.prepare(
            "SELECT ts FROM prompts WHERE ts >= ?1 AND ts < ?2 AND (?3 IS NULL OR account = ?3)",
        ) {
            let rows = stmt.query_map(params![first, end, account], |r| r.get::<_, i64>(0));
            for ts in rows.into_iter().flatten().flatten() {
                let i = bounds.partition_point(|b| *b <= ts).saturating_sub(1);
                view.buckets[i].prompts += 1;
                view.prompts += 1;
            }
        }
        let (cost_all, first_ts): (f64, Option<i64>) = c
            .query_row(
                "SELECT COALESCE(SUM(cost),0), MIN(ts) FROM turns WHERE (?1 IS NULL OR account = ?1)",
                [account],
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
        let mut accounts: Vec<AccountShare> = by_account.into_values().collect();
        // The id breaks ties, as for the agents.
        accounts.sort_by(|a, b| {
            b.cost
                .total_cmp(&a.cost)
                .then_with(|| a.account.cmp(&b.account))
        });
        view.by_account = accounts;
        if let Ok(mut stmt) = c.prepare("SELECT DISTINCT account FROM turns ORDER BY account") {
            let rows = stmt.query_map([], |r| r.get::<_, String>(0));
            view.accounts = rows.into_iter().flatten().flatten().collect();
        }
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
    use crate::paths::test_dir;

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
        s.record_turns_at(
            noon(today),
            "principal",
            "a1",
            "p1",
            &[row("claude-opus-5-5", 50, 1.0)],
        );
        s.record_turns_at(
            noon(today - Duration::days(2)),
            "principal",
            "a2",
            "p2",
            &[row("claude-sonnet-5", 20, 0.25)],
        );
        s.record_turns_at(
            noon(today - Duration::days(20)),
            "principal",
            "a2",
            "p2",
            &[row("claude-sonnet-5", 5, 0.1)],
        );
        let v = s.query_at("day", today, None, &Labels::default());
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

        let m = s.query_at("month", today, None, &Labels::default());
        assert_eq!(m.buckets.last().unwrap().label, "sept.");
        assert_eq!(m.buckets[0].label, "oct.");
        let w = s.query_at("week", today, None, &Labels::default());
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
        s.record_turns_at(
            noon(today),
            "principal",
            "a1",
            "p1",
            &[row("claude-opus-5-5", 50, 1.0)],
        );
        s.record_turns_at(
            noon(today - Duration::days(1)),
            "principal",
            "a1",
            "p1",
            &[row("claude-opus-5-5", 10, 0.5)],
        );
        s.record_turns_at(
            noon(today - Duration::days(2)),
            "principal",
            "a2",
            "p2",
            &[row("claude-sonnet-5", 20, 0.25)],
        );
        s.record_turns_at(
            noon(today),
            "principal",
            "a3",
            "p2",
            &[row("claude-sonnet-5", 1, 3.0)],
        );
        s.record_turns_at(
            noon(today - Duration::days(20)),
            "principal",
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
        let v = s.query_at("day", today, None, &names);
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
                "principal",
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
        let v = s.query_at("day", today, None, &names);
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
        let v = s.query_at("day", today, None, &names);
        assert!(v.by_agent.is_empty() && v.by_ticket.is_empty());
    }

    // ---------- accounts ----------

    /// The two tables as the versions before the accounts made them.
    const SCHEMA_BEFORE_THE_ACCOUNTS: &str = "
    CREATE TABLE turns (
      id INTEGER PRIMARY KEY, ts INTEGER NOT NULL, agent_id TEXT NOT NULL, project_id TEXT NOT NULL,
      model TEXT NOT NULL, input INTEGER NOT NULL, cache INTEGER NOT NULL, output INTEGER NOT NULL, cost REAL NOT NULL
    );
    CREATE INDEX turns_ts ON turns(ts);
    CREATE TABLE prompts (
      id INTEGER PRIMARY KEY, ts INTEGER NOT NULL, agent_id TEXT NOT NULL, project_id TEXT NOT NULL
    );
    CREATE INDEX prompts_ts ON prompts(ts);
    ";

    /// A database file as the versions before the accounts left it: two turns (a1 for 1.0, a2 for
    /// 0.5) and one prompt, all at `ts`.
    fn database_before_the_accounts(name: &str, ts: i64) -> std::path::PathBuf {
        let path = test_dir(name).join("stats.db");
        let c = Connection::open(&path).unwrap();
        c.execute_batch(SCHEMA_BEFORE_THE_ACCOUNTS).unwrap();
        for (agent, cost) in [("a1", 1.0), ("a2", 0.5)] {
            c.execute(
                "INSERT INTO turns (ts, agent_id, project_id, model, input, cache, output, cost) VALUES (?1,?2,'p1','claude-opus-5-5',50,100,10,?3)",
                params![ts, agent, cost],
            )
            .unwrap();
        }
        c.execute(
            "INSERT INTO prompts (ts, agent_id, project_id) VALUES (?1,'a1','p1')",
            [ts],
        )
        .unwrap();
        path
    }

    /// The account of each line of a table, in the order they were written.
    fn accounts_in(s: &Stats, table: &str) -> Vec<String> {
        let guard = s.conn.lock();
        let c = guard.as_ref().expect("a database");
        let mut stmt = c
            .prepare(&format!("SELECT account FROM {table} ORDER BY id"))
            .unwrap();
        let rows = stmt.query_map([], |r| r.get(0)).unwrap();
        rows.map(Result::unwrap).collect()
    }

    /// How many columns of a table are named `account`.
    fn account_columns(s: &Stats, table: &str) -> i64 {
        let guard = s.conn.lock();
        let c = guard.as_ref().expect("a database");
        c.query_row(
            &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = 'account'"),
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn principal_share(tokens: u64, cost: f64, turns: u64, agents: u64) -> AccountShare {
        AccountShare {
            account: "principal".into(),
            tokens,
            cost,
            turns,
            agents,
        }
    }

    #[test]
    fn a_database_from_before_the_accounts_keeps_its_lines_and_gives_them_to_principal() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let noon = local_ms(today) + 12 * 3600 * 1000;
        let s = Stats::open(&database_before_the_accounts(
            "k6-stats-before-accounts",
            noon,
        ));
        assert_eq!(accounts_in(&s, "turns"), ["principal", "principal"]);
        assert_eq!(accounts_in(&s, "prompts"), ["principal"]);
        // Every line is still there, and counts for Principal.
        let v = s.query_at("day", today, None, &Labels::default());
        assert!((v.cost - 1.5).abs() < 1e-9);
        assert_eq!(v.prompts, 1);
        assert_eq!(v.by_account, vec![principal_share(320, 1.5, 2, 2)]);
        // The lines written from now on say their own.
        s.record_turns_at(noon, "pro", "a3", "p1", &[row("claude-opus-5-5", 50, 0.25)]);
        s.record_prompt_at(noon, "pro", "a3", "p1");
        assert_eq!(accounts_in(&s, "turns"), ["principal", "principal", "pro"]);
        assert_eq!(accounts_in(&s, "prompts"), ["principal", "pro"]);
    }

    #[test]
    fn the_migration_is_done_once_and_opening_the_database_again_changes_nothing() {
        let noon = local_ms(NaiveDate::from_ymd_opt(2026, 9, 27).unwrap()) + 12 * 3600 * 1000;
        let path = database_before_the_accounts("k6-stats-replayed", noon);
        let s = Stats::open(&path);
        s.record_turns_at(noon, "pro", "a3", "p1", &[row("claude-opus-5-5", 50, 0.25)]);
        s.record_prompt_at(noon, "pro", "a3", "p1");
        drop(s);
        // The next starts of the app: what the lines say stays, and the column is not added twice.
        for _ in 0..2 {
            let s = Stats::open(&path);
            assert_eq!(accounts_in(&s, "turns"), ["principal", "principal", "pro"]);
            assert_eq!(accounts_in(&s, "prompts"), ["principal", "pro"]);
            assert_eq!(account_columns(&s, "turns"), 1);
            assert_eq!(account_columns(&s, "prompts"), 1);
        }
    }

    #[test]
    fn a_new_database_has_the_column_from_the_start() {
        let s = Stats::open(&test_dir("k6-stats-new").join("stats.db"));
        assert_eq!(account_columns(&s, "turns"), 1);
        assert_eq!(account_columns(&s, "prompts"), 1);
        // And an application that already had it (a database made by this version) opens it again.
        let again = Stats::open(&test_dir("k6-stats-new").join("stats.db"));
        assert_eq!(account_columns(&again, "turns"), 1);
    }

    /// Principal's a1 (project p1, Opus) and Pro's a2 (project p2, Sonnet), each with a turn of
    /// the period, and a turn in the previous one; a2's last turn made of two models. Prompts:
    /// one for Principal, two for Pro. 2026-09-27 is `today`.
    fn two_accounts(s: &Stats, today: NaiveDate) {
        let noon = |days_ago: i64| local_ms(today - Duration::days(days_ago)) + 12 * 3600 * 1000;
        s.record_turns_at(
            noon(0),
            "principal",
            "a1",
            "p1",
            &[row("claude-opus-5-5", 50, 1.0)],
        );
        s.record_turns_at(
            noon(1),
            "principal",
            "a1",
            "p1",
            &[row("claude-opus-5-5", 10, 0.5)],
        );
        s.record_turns_at(
            noon(20),
            "principal",
            "a4",
            "p1",
            &[row("claude-sonnet-5", 5, 9.0)],
        );
        s.record_turns_at(
            noon(0),
            "pro",
            "a2",
            "p2",
            &[
                row("claude-sonnet-5", 20, 0.25),
                row("claude-haiku-5", 1, 0.01),
            ],
        );
        s.record_turns_at(
            noon(5),
            "pro",
            "a2",
            "p2",
            &[row("claude-sonnet-5", 30, 0.5)],
        );
        s.record_turns_at(
            noon(20),
            "pro",
            "a5",
            "p2",
            &[row("claude-sonnet-5", 1, 3.0)],
        );
        s.record_prompt_at(noon(0), "principal", "a1", "p1");
        s.record_prompt_at(noon(0), "pro", "a2", "p2");
        s.record_prompt_at(noon(5), "pro", "a2", "p2");
    }

    #[test]
    fn every_view_of_the_period_can_be_for_one_account() {
        let s = Stats::memory();
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        two_accounts(&s, today);
        let names = labels(
            &[
                ("a1", "refacto", Some("t1")),
                ("a2", "api-docs", Some("t2")),
            ],
            &[("t1", "DEM-1", "Login", 1), ("t2", "DEM-2", "Docs", 2)],
        );
        let all = s.query_at("day", today, None, &names);
        assert_eq!((all.tokens, all.tokens_prev), (661, 226));
        assert!((all.cost - 2.26).abs() < 1e-9);
        assert!((all.cost_all - 14.26).abs() < 1e-9);
        assert_eq!(all.prompts, 3);

        let main = s.query_at("day", today, Some("principal"), &names);
        assert_eq!((main.tokens, main.tokens_prev), (280, 115));
        assert!((main.cost - 1.5).abs() < 1e-9);
        assert!((main.cost_all - 10.5).abs() < 1e-9);
        assert_eq!(main.prompts, 1);
        // The chart: Principal's own turns of the two days.
        assert_eq!(main.buckets[13].input, 50);
        assert_eq!(main.buckets[12].input, 10);
        assert_eq!(main.buckets.iter().map(|b| b.prompts).sum::<u64>(), 1);
        let keys = |shares: &[Share]| shares.iter().map(|s| s.key.clone()).collect::<Vec<_>>();
        assert_eq!(keys(&main.by_project), ["p1"]);
        assert_eq!(keys(&main.by_model), ["claude-opus-5-5"]);
        assert_eq!(
            main.by_agent
                .iter()
                .map(|a| a.agent_id.as_str())
                .collect::<Vec<_>>(),
            ["a1"]
        );
        assert_eq!(main.by_ticket.len(), 1);
        assert_eq!(
            (main.by_ticket[0].key.as_str(), main.by_ticket[0].cost),
            ("DEM-1", 1.5)
        );

        let pro = s.query_at("day", today, Some("pro"), &names);
        assert_eq!((pro.tokens, pro.tokens_prev), (381, 111));
        assert!((pro.cost - 0.76).abs() < 1e-9);
        assert!((pro.cost_all - 3.76).abs() < 1e-9);
        assert_eq!(pro.prompts, 2);
        assert_eq!(keys(&pro.by_project), ["p2"]);
        assert_eq!(keys(&pro.by_model), ["claude-sonnet-5", "claude-haiku-5"]);
        assert_eq!(pro.by_ticket.len(), 1);
        assert_eq!(pro.by_ticket[0].key, "DEM-2");
        // Together they are the whole.
        assert_eq!(main.tokens + pro.tokens, all.tokens);
        assert_eq!(main.tokens_prev + pro.tokens_prev, all.tokens_prev);
        assert!((main.cost + pro.cost - all.cost).abs() < 1e-9);
        assert_eq!(main.prompts + pro.prompts, all.prompts);
    }

    #[test]
    fn the_first_turn_of_a_view_is_the_first_of_its_account() {
        let s = Stats::memory();
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let noon = |days_ago: i64| local_ms(today - Duration::days(days_ago)) + 12 * 3600 * 1000;
        s.record_turns_at(
            noon(30),
            "principal",
            "a1",
            "p1",
            &[row("claude-opus-5-5", 1, 1.0)],
        );
        s.record_turns_at(
            noon(3),
            "pro",
            "a2",
            "p2",
            &[row("claude-sonnet-5", 1, 1.0)],
        );
        let first = |account| {
            s.query_at("day", today, account, &Labels::default())
                .first_ts
        };
        assert_eq!(first(None), Some(noon(30)));
        assert_eq!(first(Some("principal")), Some(noon(30)));
        assert_eq!(first(Some("pro")), Some(noon(3)));
        // An account with no turn: nothing, and no error.
        let none = s.query_at("day", today, Some("gone"), &Labels::default());
        assert_eq!((none.first_ts, none.tokens, none.prompts), (None, 0, 0));
        assert_eq!(none.buckets.len(), 14);
        assert!(none.by_project.is_empty() && none.by_agent.is_empty());
    }

    #[test]
    fn the_accounts_are_side_by_side_whichever_one_the_view_is_for() {
        let s = Stats::memory();
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        two_accounts(&s, today);
        let expected = vec![
            principal_share(280, 1.5, 2, 1),
            // The turn of two models counts once, with the tokens and the cost of both.
            AccountShare {
                account: "pro".into(),
                tokens: 381,
                cost: 0.76,
                turns: 2,
                agents: 1,
            },
        ];
        for account in [None, Some("principal"), Some("pro"), Some("gone")] {
            let v = s.query_at("day", today, account, &Labels::default());
            assert_eq!(v.by_account.len(), 2, "{account:?}");
            for (got, want) in v.by_account.iter().zip(&expected) {
                assert_eq!(got.account, want.account);
                assert_eq!(
                    (got.tokens, got.turns, got.agents),
                    (want.tokens, want.turns, want.agents)
                );
                assert!((got.cost - want.cost).abs() < 1e-9, "{account:?}");
            }
        }
    }

    #[test]
    fn the_accounts_are_listed_dearest_first_and_count_their_agents() {
        let s = Stats::memory();
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let noon = local_ms(today) + 12 * 3600 * 1000;
        // Two agents on Pro, one turn each, one of them twice; a third account dearer than Pro.
        s.record_turns_at(noon, "pro", "a1", "p1", &[row("claude-sonnet-5", 1, 0.5)]);
        s.record_turns_at(
            noon + 1,
            "pro",
            "a1",
            "p1",
            &[row("claude-sonnet-5", 1, 0.5)],
        );
        s.record_turns_at(noon, "pro", "a2", "p1", &[row("claude-sonnet-5", 1, 0.5)]);
        s.record_turns_at(noon, "team", "a3", "p1", &[row("claude-sonnet-5", 1, 4.0)]);
        let v = s.query_at("day", today, None, &Labels::default());
        let shares: Vec<_> = v
            .by_account
            .iter()
            .map(|a| (a.account.as_str(), a.turns, a.agents))
            .collect();
        assert_eq!(shares, [("team", 1, 1), ("pro", 3, 2)]);
    }

    #[test]
    fn an_account_with_no_turn_in_the_period_is_not_a_line_but_is_still_known() {
        let s = Stats::memory();
        let today = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let noon = |days_ago: i64| local_ms(today - Duration::days(days_ago)) + 12 * 3600 * 1000;
        s.record_turns_at(
            noon(0),
            "principal",
            "a1",
            "p1",
            &[row("claude-opus-5-5", 1, 1.0)],
        );
        s.record_turns_at(
            noon(400),
            "old",
            "a2",
            "p1",
            &[row("claude-opus-5-5", 1, 1.0)],
        );
        for account in [None, Some("old")] {
            let v = s.query_at("day", today, account, &Labels::default());
            assert_eq!(v.accounts, ["old", "principal"], "{account:?}");
        }
        let v = s.query_at("day", today, None, &Labels::default());
        assert_eq!(
            v.by_account
                .iter()
                .map(|a| a.account.as_str())
                .collect::<Vec<_>>(),
            ["principal"]
        );
        // A database with no turn knows none.
        let empty = Stats::memory().query_at("day", today, None, &Labels::default());
        assert!(empty.accounts.is_empty() && empty.by_account.is_empty());
    }

    #[test]
    fn today_is_told_by_account() {
        let s = Stats::memory();
        let now = now_ms();
        let midnight = local_ms(Local::now().date_naive());
        s.record_turns_at(
            now,
            "principal",
            "a1",
            "p1",
            &[row("claude-opus-5-5", 1, 1.0)],
        );
        s.record_turns_at(
            now,
            "principal",
            "a1",
            "p1",
            &[row("claude-opus-5-5", 1, 0.5)],
        );
        s.record_turns_at(now, "pro", "a2", "p1", &[row("claude-sonnet-5", 1, 0.25)]);
        // Yesterday does not count.
        s.record_turns_at(
            midnight - 1000,
            "pro",
            "a2",
            "p1",
            &[row("claude-sonnet-5", 1, 8.0)],
        );
        let today = s.today_by_account();
        assert_eq!(today.len(), 2);
        assert!((today["principal"] - 1.5).abs() < 1e-9);
        assert!((today["pro"] - 0.25).abs() < 1e-9);
        assert!((s.today_cost() - 1.75).abs() < 1e-9);
        assert!(Stats::memory().today_by_account().is_empty());
    }
}
