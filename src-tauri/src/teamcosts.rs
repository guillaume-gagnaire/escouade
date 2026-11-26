//! The costs and the quotas of a team in the app (TS-16, `docs/TEAM-SERVER.md` §10).
//!
//! - **What goes**: once an hour (`every`), this device's totals of the team's projects, made from its statistics
//!   (`Stats::turn_lines`): by day, project (its journal), harness, model and plan, the tokens, the cost, the highest
//!   quota of the day; and the total of each ticket by its key. [`totals_of`] is the only place they are made; the
//!   crate seals them to the administrators' key (`escouade_team::costs`).
//! - **The team's choice (C15)**: `costs.grain` of the team's file, `project` unless the team says `member`. « Par
//!   membre », the totals carry the member, but only once this device's member saw the screen that tells it
//!   ([`Core::team_costs_ack`]); before, they go without it.
//! - **The budgets**: `costs.budgets` (by month: `team`, `projects` by name, `members` by id or `*`), and
//!   `costs.stopAutopilot`. Before a start, this device weighs its own month and the team's last summary
//!   ([`Core::team_cost_notice`]); over a budget, with `costs.stopAutopilot`, the autopilot of the project stops (the
//!   board's pass holds its tickets, the lock greys its switch). An administrator's device weighs the whole team and
//!   leaves a request in its box « À toi », once per budget, month and threshold.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use chrono::{Datelike, Local, NaiveDate, TimeZone};
use escouade_team::costs::{Day, Row, Summary, TicketRow, Totals};
use escouade_team::settings::Effective;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::asks::{AskAuthor, AskKind};
use crate::core::Core;
use crate::model::now_ms;
use crate::stats::TurnLine;

/// The keys of the team's file (C15, the budgets).
pub const GRAIN: &str = "costs.grain";
pub const BUDGETS: &str = "costs.budgets";
pub const STOP: &str = "costs.stopAutopilot";

/// From what share of a budget a start is told, and an administrator warned.
pub const WARN_PCT: f64 = 80.0;

const HOUR_MS: i64 = 3_600_000;

/// How often the totals go: once an hour (`ESCOUADE_TEAM_COSTS_EVERY_MS` in a debug build, for the e2e).
pub fn every_ms() -> i64 {
    std::env::var("ESCOUADE_TEAM_COSTS_EVERY_MS")
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .and_then(|v| v.parse::<i64>().ok())
        .map_or(HOUR_MS, |ms| ms.max(200))
}

/// What a team's file says of the costs.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostRules {
    /// « Par membre » (C15-A); « par projet » otherwise.
    pub by_member: bool,
    pub team: Option<f64>,
    /// By the project's name (any case) or its journal.
    pub projects: BTreeMap<String, f64>,
    /// By the member's id (hex), or `*` for each member.
    pub members: BTreeMap<String, f64>,
    /// Over a budget, the autopilot stops.
    pub stop: bool,
}

fn amount(v: &Value) -> Option<f64> {
    v.as_f64().filter(|x| x.is_finite() && *x > 0.0)
}

fn amounts(v: Option<&Value>) -> BTreeMap<String, f64> {
    v.and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.trim().to_lowercase(), amount(v)?)))
                .collect()
        })
        .unwrap_or_default()
}

impl CostRules {
    /// The rules of the version in force: imposed first, then recommended.
    pub fn of(e: &Effective) -> CostRules {
        let get = |k: &str| e.enforced.get(k).or_else(|| e.defaults.get(k));
        let budgets = get(BUDGETS);
        CostRules {
            by_member: get(GRAIN).and_then(Value::as_str) == Some("member"),
            team: budgets.and_then(|b| b.get("team")).and_then(amount),
            projects: amounts(budgets.and_then(|b| b.get("projects"))),
            members: amounts(budgets.and_then(|b| b.get("members"))),
            stop: get(STOP).and_then(Value::as_bool) == Some(true),
        }
    }

    fn project_budget(&self, name: &str, board: &str) -> Option<f64> {
        self.projects
            .get(&name.trim().to_lowercase())
            .or_else(|| self.projects.get(board))
            .copied()
    }

    fn member_budget(&self, member: &str) -> Option<f64> {
        self.members
            .get(member)
            .or_else(|| self.members.get("*"))
            .copied()
    }
}

/// What this device keeps of the costs beside the teams' state: `team-costs.json` of the data folder.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CostsFile {
    /// The teams « par membre » whose screen the member saw.
    pub told: BTreeSet<String>,
    /// The warnings an administrator's device left in its box: `team|month|scope|threshold`.
    pub alerts: BTreeSet<String>,
    /// The highest quota of each day and harness: `2026-11-20|claude`.
    pub quota: BTreeMap<String, f64>,
    /// When each team's totals last went (ms).
    pub sent: BTreeMap<String, i64>,
    /// The consents asked in the box « À toi » (`team|fingerprint`): the mode « auto » a team imposes (SR3-07).
    pub consents: BTreeSet<String>,
}

/// What the core holds of the teams' costs.
#[derive(Default)]
pub struct TeamCosts {
    /// By team: its name and its rules.
    rules: BTreeMap<String, (String, CostRules)>,
    /// The projects whose autopilot a budget stops, and why.
    stops: BTreeMap<String, String>,
    /// The teams whose every project stops (a budget of the team, or of this member): their lock.
    team_stops: BTreeMap<String, String>,
    file: Option<CostsFile>,
}

/// A budget, weighed.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetState {
    /// `team`, `project` or `member`.
    pub scope: String,
    /// The project (its local id), the member (hex), or empty for the team.
    pub of: String,
    pub name: String,
    pub spent: f64,
    pub budget: f64,
    pub pct: f64,
    /// « Le projet api a dépensé 92 % de son budget du mois ».
    pub text: String,
}

/// What a start is told.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostNotice {
    pub text: String,
    pub pct: f64,
    /// 100 % or more.
    pub over: bool,
    /// The autopilot stops (the team imposes it).
    pub stops: bool,
    pub team: String,
}

fn day_of(ms: i64) -> String {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map_or_else(|| Local::now().date_naive(), |t| t.date_naive())
        .format("%Y-%m-%d")
        .to_string()
}

fn start_of(day: NaiveDate) -> i64 {
    let naive = day.and_hms_opt(0, 0, 0).expect("midnight");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|t| t.timestamp_millis())
        .unwrap_or_else(|| naive.and_utc().timestamp_millis())
}

/// What a team's totals are made of, besides the turns: how each local project and agent is named to the team.
#[derive(Clone, Debug, Default)]
pub struct Names {
    /// Local project id → its journal (hex).
    pub boards: HashMap<String, String>,
    /// Agent id → the key of its ticket.
    pub tickets: HashMap<String, String>,
    /// The accounts on a plan (a quota, no bill per turn).
    pub plans: BTreeSet<String>,
}

/// The totals of the days of `days`, made of the turns `lines`: nothing but sums (the only place they are made).
pub fn totals_of(
    lines: &[TurnLine],
    days: &BTreeSet<String>,
    names: &Names,
    quota: &BTreeMap<String, f64>,
    member: Option<String>,
    hour: i64,
) -> Totals {
    // A row by (project, harness, model, plan); a ticket by (project, key).
    type Rows = BTreeMap<(String, String, String, bool), Row>;
    type Tickets = BTreeMap<(String, String), TicketRow>;
    let mut by_day: BTreeMap<String, (Rows, Tickets)> = days
        .iter()
        .map(|d| (d.clone(), Default::default()))
        .collect();
    for l in lines {
        let Some(board) = names.boards.get(&l.project) else {
            continue;
        };
        let day = day_of(l.ts);
        let Some((rows, tickets)) = by_day.get_mut(&day) else {
            continue;
        };
        let plan = names.plans.contains(&l.account);
        let key = (board.clone(), l.engine.clone(), l.model.clone(), plan);
        let row = rows.entry(key).or_insert_with(|| Row {
            project: board.clone(),
            harness: l.engine.clone(),
            model: l.model.clone(),
            plan,
            input: 0,
            output: 0,
            cache: 0,
            cost: 0.0,
            quota: quota.get(&format!("{day}|{}", l.engine)).copied(),
        });
        row.input += l.input;
        row.output += l.output;
        row.cache += l.cache;
        // A cost not well said (NaN, infinite, negative) counts nothing: the administrators would refuse the day.
        let cost = if l.cost.is_finite() && l.cost > 0.0 {
            l.cost
        } else {
            0.0
        };
        row.cost += cost;
        if let Some(ticket) = names.tickets.get(&l.agent) {
            let t = tickets
                .entry((board.clone(), ticket.clone()))
                .or_insert_with(|| TicketRow {
                    project: board.clone(),
                    ticket: ticket.clone(),
                    input: 0,
                    output: 0,
                    cache: 0,
                    cost: 0.0,
                });
            t.input += l.input;
            t.output += l.output;
            t.cache += l.cache;
            t.cost += cost;
        }
    }
    Totals {
        member,
        hour,
        days: by_day
            .into_iter()
            .map(|(day, (rows, tickets))| Day {
                day,
                rows: rows.into_values().collect(),
                tickets: tickets.into_values().collect(),
            })
            .collect(),
    }
}

/// A line of the table: a day of a project, a harness and a model, of a member when the team shows them.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    /// Hex; None « par projet ».
    pub member: Option<String>,
    pub mine: bool,
    pub project: String,
    pub day: String,
    pub harness: String,
    pub model: String,
    pub plan: bool,
    pub input: u64,
    pub output: u64,
    pub cache: u64,
    pub cost: f64,
    pub quota: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketRecord {
    pub member: Option<String>,
    pub project: String,
    pub day: String,
    pub ticket: String,
    pub tokens: u64,
    pub cost: f64,
}

/// What the window shows of a team's costs (Réglages › Équipe › Coûts).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamCostsView {
    pub team: String,
    pub name: String,
    pub admin: bool,
    pub me: String,
    pub rules: CostRules,
    /// « Par membre » and this member was told (its totals carry its name).
    pub told: bool,
    pub epoch: Option<u64>,
    pub holds: bool,
    pub stale: bool,
    /// The days waiting to go (offline, or no key yet).
    pub pending: Vec<String>,
    /// The last totals that went, exactly.
    pub sent: Option<Totals>,
    pub records: Vec<Record>,
    pub tickets: Vec<TicketRecord>,
    pub summary: Option<Summary>,
    /// Journal (hex) → the project's name, as this device knows it.
    pub projects: BTreeMap<String, String>,
    pub budgets: Vec<BudgetState>,
}

fn budget_text(scope: &str, name: &str, pct: f64) -> String {
    let pct = pct.round() as i64;
    match scope {
        "project" => tr!(
            "Le projet {name} a dépensé {pct} % de son budget du mois",
            "The project {name} spent {pct}% of its budget for the month"
        ),
        "member" if name.is_empty() => tr!(
            "Tu as dépensé {pct} % de ton budget du mois",
            "You spent {pct}% of your budget for the month"
        ),
        "member" => tr!(
            "Le membre {name} a dépensé {pct} % de son budget du mois",
            "The member {name} spent {pct}% of their budget for the month"
        ),
        _ => tr!(
            "L’équipe {name} a dépensé {pct} % de son budget du mois",
            "The team {name} spent {pct}% of its budget for the month"
        ),
    }
}

fn weigh(scope: &str, of: &str, name: &str, spent: f64, budget: f64, say: &str) -> BudgetState {
    let pct = if budget > 0.0 {
        spent / budget * 100.0
    } else {
        0.0
    };
    BudgetState {
        scope: scope.into(),
        of: of.into(),
        name: name.into(),
        spent,
        budget,
        pct,
        text: budget_text(scope, say, pct),
    }
}

fn short(member: &str) -> String {
    member.chars().take(8).collect()
}

impl Core {
    fn costs_file_path(&self) -> std::path::PathBuf {
        self.data.root().join("team-costs.json")
    }

    fn with_costs<R>(&self, f: impl FnOnce(&mut TeamCosts) -> R) -> R {
        let mut c = self.team_costs.lock();
        if c.file.is_none() {
            c.file = Some(
                std::fs::read(self.costs_file_path())
                    .ok()
                    .and_then(|b| serde_json::from_slice(&b).ok())
                    .unwrap_or_default(),
            );
        }
        f(&mut c)
    }

    fn save_costs_file(&self) {
        let file = self.with_costs(|c| c.file.clone().unwrap_or_default());
        if let Ok(b) = serde_json::to_vec_pretty(&file) {
            if let Err(e) = std::fs::write(self.costs_file_path(), b) {
                log::warn!("team costs: {e}");
            }
        }
    }

    /// The rules of a team's version in force (None: the device left it). « Par projet » again: the next « par
    /// membre » is told again.
    pub(crate) fn set_team_cost_rules(&self, id: &str, name: &str, effective: Option<&Effective>) {
        let rules = effective.map(CostRules::of);
        let reset = self.with_costs(|c| {
            match &rules {
                Some(r) => {
                    c.rules.insert(id.to_owned(), (name.to_owned(), r.clone()));
                }
                None => {
                    c.rules.remove(id);
                }
            }
            let file = c.file.get_or_insert_with(Default::default);
            !rules.as_ref().is_some_and(|r| r.by_member) && file.told.remove(id)
        });
        if reset {
            self.save_costs_file();
        }
    }

    pub(crate) fn team_cost_rules(&self, id: &str) -> CostRules {
        self.with_costs(|c| c.rules.get(id).map(|r| r.1.clone()).unwrap_or_default())
    }

    /// The member saw the screen « Ton équipe voit désormais tes totaux par jour »: its totals carry its name.
    pub fn team_costs_ack(&self, id: &str) {
        self.with_costs(|c| {
            c.file
                .get_or_insert_with(Default::default)
                .told
                .insert(id.to_owned());
        });
        self.save_costs_file();
    }

    /// The teams « par membre » whose screen this device's member has not seen yet: (id, name).
    pub fn team_costs_untold(&self) -> Vec<(String, String)> {
        self.with_costs(|c| {
            let told = c.file.as_ref().map(|f| f.told.clone()).unwrap_or_default();
            c.rules
                .iter()
                .filter(|(id, (_, r))| r.by_member && !told.contains(*id))
                .map(|(id, (name, _))| (id.clone(), name.clone()))
                .collect()
        })
    }

    /// The local projects of a team that have a team Kanban: (local id, name, journal hex).
    fn team_projects_of(&self, team: &str) -> Vec<(String, String, String)> {
        self.projects
            .read()
            .iter()
            .filter(|p| p.team.as_deref() == Some(team))
            .filter_map(|p| Some((p.id.clone(), p.name.clone(), p.team_project.clone()?)))
            .collect()
    }

    fn cost_names(&self, team: &str) -> Names {
        let boards = self
            .team_projects_of(team)
            .into_iter()
            .map(|(id, _, board)| (id, board))
            .collect();
        let agent_tickets: HashMap<String, String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                Some((rt.meta.id.clone(), rt.meta.ticket_id.clone()?))
            })
            .collect();
        let keys: HashMap<String, String> = self
            .tickets
            .read()
            .iter()
            .map(|t| (t.id.clone(), t.key.clone()))
            .collect();
        let tickets = agent_tickets
            .into_iter()
            .filter_map(|(a, t)| Some((a, keys.get(&t)?.clone())))
            .collect();
        let accounts = self.settings.read().accounts.clone();
        let plans = self
            .usage
            .lock()
            .accounts
            .iter()
            .filter(|u| !u.meters.is_empty())
            .filter(|u| !accounts.iter().any(|a| a.id == u.id && a.api_key))
            .map(|u| u.id.clone())
            .collect();
        Names {
            boards,
            tickets,
            plans,
        }
    }

    /// This device's totals of a team since `since` (ms), for the days `days`.
    fn own_totals(
        &self,
        team: &str,
        days: &BTreeSet<String>,
        since: i64,
        member: Option<String>,
    ) -> Totals {
        let names = self.cost_names(team);
        let projects: Vec<String> = names.boards.keys().cloned().collect();
        let lines = self.stats.turn_lines(since, &projects);
        let quota =
            self.with_costs(|c| c.file.as_ref().map(|f| f.quota.clone()).unwrap_or_default());
        totals_of(&lines, days, &names, &quota, member, now_ms())
    }

    /// The highest quota of today, by harness, kept for the totals.
    fn sample_quota(&self) {
        let today = day_of(now_ms());
        let seen: Vec<(String, f64)> = self
            .usage
            .lock()
            .accounts
            .iter()
            .filter_map(|u| {
                let pct = u.meters.iter().map(|m| m.pct).fold(f64::NAN, f64::max);
                pct.is_finite()
                    .then(|| (u.engine.as_str().to_string(), pct))
            })
            .collect();
        if seen.is_empty() {
            return;
        }
        self.with_costs(|c| {
            let file = c.file.get_or_insert_with(Default::default);
            for (engine, pct) in seen {
                let at = file.quota.entry(format!("{today}|{engine}")).or_insert(0.0);
                *at = at.max(pct);
            }
            // A year and a little more.
            while file.quota.len() > 2000 {
                let first = file.quota.keys().next().cloned();
                if let Some(first) = first {
                    file.quota.remove(&first);
                }
            }
        });
    }

    /// Sends the teams' totals that are due (`force`: all of them now), then weighs their budgets.
    pub async fn team_costs_pass(self: &Arc<Self>, force: bool) {
        let Some(teams) = self.teams_link() else {
            return;
        };
        self.sample_quota();
        let now = now_ms();
        let ids: BTreeSet<String> = self
            .projects
            .read()
            .iter()
            .filter(|p| p.team_project.is_some())
            .filter_map(|p| p.team.clone())
            .collect();
        let every = every_ms();
        for id in ids {
            let due = force
                || self.with_costs(|c| {
                    c.file
                        .as_ref()
                        .and_then(|f| f.sent.get(&id))
                        .is_none_or(|at| now - at >= every)
                });
            if due {
                let rules = self.team_cost_rules(&id);
                let told =
                    self.with_costs(|c| c.file.as_ref().is_some_and(|f| f.told.contains(&id)));
                let named = rules.by_member && told;
                let today = Local::now().date_naive();
                let sent = teams
                    .costs_send(&id, |me, whole| {
                        let first = if whole {
                            today.with_day(1).unwrap_or(today)
                        } else {
                            today.pred_opt().unwrap_or(today)
                        };
                        let days: BTreeSet<String> = first
                            .iter_days()
                            .take_while(|d| *d <= today)
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .collect();
                        Some(self.own_totals(
                            &id,
                            &days,
                            start_of(first),
                            named.then(|| me.to_string()),
                        ))
                    })
                    .await;
                match sent {
                    Ok(_) => {
                        self.with_costs(|c| {
                            c.file
                                .get_or_insert_with(Default::default)
                                .sent
                                .insert(id.clone(), now);
                        });
                        self.save_costs_file();
                    }
                    Err(e) => log::info!("team costs of {id}: {e}"),
                }
            }
            self.weigh_team(&teams, &id).await;
        }
    }

    /// This device's own spending of the month, by local project of the team.
    fn own_month(&self, team: &str) -> BTreeMap<String, f64> {
        let today = Local::now().date_naive();
        let first = today.with_day(1).unwrap_or(today);
        let names = self.cost_names(team);
        let projects: Vec<String> = names.boards.keys().cloned().collect();
        let mut out = BTreeMap::new();
        for l in self.stats.turn_lines(start_of(first), &projects) {
            *out.entry(l.project).or_insert(0.0) += l.cost;
        }
        out
    }

    /// The budgets of a team as this device weighs them: its own month, the team's last summary, and on an
    /// administrator's device every device's totals.
    fn budgets_of(
        &self,
        team: &str,
        view: Option<&escouade_team::costs::View>,
        me: &str,
        admin: bool,
    ) -> Vec<BudgetState> {
        let (team_name, rules) = self
            .with_costs(|c| c.rules.get(team).cloned())
            .unwrap_or_default();
        let month = crate::teams::month_now();
        let summary = view
            .and_then(|v| v.summary.clone())
            .filter(|s| s.month == month);
        let own = self.own_month(team);
        let own_total: f64 = own.values().sum();
        let mut out = Vec::new();
        // The projects of the team: this device's, and those of the summary (an administrator weighs them all).
        let mut boards: BTreeMap<String, (Option<String>, String)> = self
            .team_projects_of(team)
            .into_iter()
            .map(|(id, name, board)| (board, (Some(id), name)))
            .collect();
        if let Some(s) = &summary {
            for b in s.projects.keys() {
                boards.entry(b.clone()).or_insert_with(|| (None, short(b)));
            }
        }
        for (board, (id, name)) in boards {
            if let Some(budget) = rules.project_budget(&name, &board) {
                let theirs = summary
                    .as_ref()
                    .and_then(|s| s.projects.get(&board))
                    .map_or(0.0, |s| s.cost);
                let mine = id.as_ref().and_then(|i| own.get(i)).copied().unwrap_or(0.0);
                let of = id.unwrap_or(board);
                out.push(weigh(
                    "project",
                    &of,
                    &name,
                    theirs.max(mine),
                    budget,
                    &name,
                ));
            }
        }
        if let Some(budget) = rules.team {
            let spent = summary.as_ref().map_or(0.0, |s| s.team.cost).max(own_total);
            out.push(weigh("team", "", &team_name, spent, budget, &team_name));
        }
        if let Some(budget) = rules.member_budget(me) {
            out.push(weigh("member", me, "", own_total, budget, ""));
        }
        // An administrator's device, « par membre »: each member's month.
        if admin && rules.by_member {
            let mut by: BTreeMap<String, f64> = BTreeMap::new();
            for d in view.map(|v| v.received.as_slice()).unwrap_or_default() {
                for (day, got) in &d.days {
                    let Some(m) = &got.member else { continue };
                    if m == me || !day.starts_with(&month) {
                        continue;
                    }
                    *by.entry(m.clone()).or_insert(0.0) +=
                        got.rows.iter().map(|r| r.cost).sum::<f64>();
                }
            }
            for (m, spent) in by {
                if let Some(budget) = rules.member_budget(&m) {
                    out.push(weigh("member", &m, &short(&m), spent, budget, &short(&m)));
                }
            }
        }
        out
    }

    /// Weighs a team's budgets: the projects whose autopilot stops, and, on an administrator's device, the warnings
    /// left in its box.
    async fn weigh_team(self: &Arc<Self>, teams: &crate::teams::Teams, team: &str) {
        let Ok(of) = teams.costs_of(team).await else {
            return;
        };
        let rules = self.team_cost_rules(team);
        let budgets = self.budgets_of(team, Some(&of.view), &of.me, of.admin);
        let (name, _) = self
            .with_costs(|c| c.rules.get(team).cloned())
            .unwrap_or_default();
        let by = tr!("imposé par l’équipe {name}", "imposed by the team {name}");
        // What stops: a project's budget its project; the team's, or this member's own, every project of the team.
        let mut stops = BTreeMap::new();
        let mut team_stop = None;
        if rules.stop {
            for b in budgets.iter().filter(|b| b.pct >= 100.0) {
                let why = format!("{} ({by})", b.text);
                match b.scope.as_str() {
                    "project" => {
                        stops.insert(b.of.clone(), why);
                    }
                    "member" if b.of != of.me => {}
                    _ => team_stop = Some(why),
                }
            }
            if let Some(why) = &team_stop {
                for (id, _, _) in self.team_projects_of(team) {
                    stops.entry(id).or_insert_with(|| why.clone());
                }
            }
        }
        let locks_changed = self.with_costs(|c| {
            let mine: BTreeSet<String> = self
                .team_projects_of(team)
                .into_iter()
                .map(|p| p.0)
                .collect();
            c.stops.retain(|p, _| !mine.contains(p));
            c.stops.extend(stops);
            let before = c.team_stops.get(team).cloned();
            match &team_stop {
                Some(w) => c.team_stops.insert(team.to_owned(), w.clone()),
                None => c.team_stops.remove(team),
            };
            before != team_stop
        });
        if locks_changed {
            self.apply_locks();
        }
        if of.admin {
            self.alert_admin(team, &name, &budgets);
        }
    }

    /// The first time a team's version sets the agents' default mode to « auto », a request in the box « À toi »: it
    /// applies only once this device accepts it (Réglages › Équipe), once per fingerprint (SR3-07).
    pub(crate) fn tell_team_consents(
        self: &Arc<Self>,
        team: &str,
        name: &str,
        effective: &Effective,
    ) {
        // The request goes to a project of the team (the box is by project); none yet: at the next pass.
        let project = self
            .projects
            .read()
            .iter()
            .find(|p| p.team.as_deref() == Some(team))
            .map(|p| p.id.clone());
        let Some(project) = project else {
            return;
        };
        for p in &effective.pending {
            if p.key != escouade_policy::list::DEFAULT_MODE
                || p.value.as_str() != Some(crate::team_settings::AUTO_MODE)
            {
                continue;
            }
            let key = format!("{team}|{}", p.fingerprint);
            let told =
                self.with_costs(|c| c.file.as_ref().is_some_and(|f| f.consents.contains(&key)));
            if told {
                continue;
            }
            let draft = crate::asks::AskDraft {
                kind: AskKind::Task,
                title: tr!(
                    "L’équipe {name} veut le mode « auto » pour tes agents",
                    "The team {name} wants the “auto” mode for your agents"
                ),
                body: tr!(
                    "Le fichier de l’équipe {name} met « auto » comme mode par défaut des agents : un agent y demande presque rien avant d’agir. Il ne s’applique sur ce poste qu’une fois accepté ; d’ici là, ton mode reste le tien.",
                    "The file of the team {name} sets “auto” as the agents’ default mode: an agent there asks almost nothing before acting. It applies on this machine only once accepted; until then, your mode stays yours."
                ),
                steps: vec![tr!(
                    "Accepter ou laisser en attente dans Équipe › Réglages de l’équipe",
                    "Accept it or leave it waiting in Team › Team settings"
                )],
                ..Default::default()
            };
            let author = AskAuthor {
                agent: None,
                name: "Escouade".into(),
                chef: false,
            };
            match self.ask_deposit(author, &project, None, draft, Vec::new()) {
                Ok(_) => {
                    self.with_costs(|c| {
                        c.file
                            .get_or_insert_with(Default::default)
                            .consents
                            .insert(key);
                    });
                    self.save_costs_file();
                }
                Err(e) => log::info!("team settings: the consent not asked: {e:#}"),
            }
        }
    }

    /// Leaves a request in the box « À toi » for each budget at 80 % or 100 %, once per month and threshold.
    fn alert_admin(self: &Arc<Self>, team: &str, name: &str, budgets: &[BudgetState]) {
        let month = crate::teams::month_now();
        let Some(project) = self.team_projects_of(team).into_iter().next().map(|p| p.0) else {
            return;
        };
        for b in budgets {
            let threshold = if b.pct >= 100.0 {
                100
            } else if b.pct >= WARN_PCT {
                80
            } else {
                continue;
            };
            let key = format!("{team}|{month}|{}:{}|{threshold}", b.scope, b.of);
            let told =
                self.with_costs(|c| c.file.as_ref().is_some_and(|f| f.alerts.contains(&key)));
            if told {
                continue;
            }
            let project_id = if b.scope == "project" && self.project(&b.of).is_ok() {
                b.of.clone()
            } else {
                project.clone()
            };
            let draft = crate::asks::AskDraft {
                kind: AskKind::Task,
                title: format!("{} · {name}", b.text),
                body: tr!(
                    "{text} ({spent} $ sur {budget} $). Relève le budget dans le fichier de l’équipe (`costs.budgets`), ou laisse `costs.stopAutopilot` arrêter le pilote auto au-delà.",
                    "{text} (${spent} of ${budget}). Raise the budget in the team’s file (`costs.budgets`), or let `costs.stopAutopilot` stop the autopilot beyond it.",
                    text = b.text,
                    spent = format!("{:.2}", b.spent),
                    budget = format!("{:.2}", b.budget)
                ),
                steps: vec![tr!(
                    "Voir les coûts de l’équipe (Équipe › Coûts)",
                    "See the team’s costs (Team › Costs)"
                )],
                ..Default::default()
            };
            let author = AskAuthor {
                agent: None,
                name: "Escouade".into(),
                chef: false,
            };
            // Told once it is in the box: a box full, or closed, tries again at the next pass.
            match self.ask_deposit(author, &project_id, None, draft, Vec::new()) {
                Ok(_) => {
                    self.with_costs(|c| {
                        c.file
                            .get_or_insert_with(Default::default)
                            .alerts
                            .insert(key);
                    });
                    self.save_costs_file();
                }
                Err(e) => log::info!("team costs: the warning not left: {e:#}"),
            }
        }
    }

    /// What a start in a project is told of its budgets (80 % or more), the highest first.
    pub fn team_cost_notice(&self, project_id: &str) -> Option<CostNotice> {
        let team = self.project(project_id).ok()?.team?;
        let teams = self.teams_link()?;
        let view = teams.costs_seen(&team);
        let member = self.team_member_of(&teams, &team);
        let budgets = self.budgets_of(&team, view.as_ref(), &member, false);
        let rules = self.team_cost_rules(&team);
        let (name, _) = self
            .with_costs(|c| c.rules.get(&team).cloned())
            .unwrap_or_default();
        budgets
            .into_iter()
            .filter(|b| b.scope != "project" || b.of == project_id)
            .filter(|b| b.scope != "member" || b.of == member)
            .filter(|b| b.pct >= WARN_PCT)
            .max_by(|a, b| a.pct.total_cmp(&b.pct))
            .map(|b| CostNotice {
                over: b.pct >= 100.0,
                stops: rules.stop && b.pct >= 100.0,
                text: b.text,
                pct: b.pct,
                team: name,
            })
    }

    fn team_member_of(&self, teams: &crate::teams::Teams, team: &str) -> String {
        teams.member_of(team).unwrap_or_default()
    }

    /// Why the autopilot of a project is stopped by a budget (the board's pass holds its tickets).
    pub(crate) fn team_cost_stop(&self, project_id: &str) -> Option<String> {
        self.with_costs(|c| c.stops.get(project_id).cloned())
    }

    /// The teams whose every project a budget stops, by team id, with their name (`apply_locks` puts their lock).
    pub(crate) fn team_cost_locks(&self) -> BTreeMap<String, String> {
        self.with_costs(|c| {
            c.team_stops
                .keys()
                .map(|id| {
                    let name = c.rules.get(id).map(|r| r.0.clone()).unwrap_or_default();
                    (id.clone(), name)
                })
                .collect()
        })
    }

    /// The table of a team's costs.
    pub async fn team_costs_view(&self, team: &str) -> anyhow::Result<TeamCostsView> {
        let teams = self
            .teams_link()
            .ok_or_else(|| anyhow::anyhow!("no team"))?;
        let of = teams.costs_of(team).await.map_err(anyhow::Error::msg)?;
        let rules = self.team_cost_rules(team);
        let told = rules.by_member
            && self.with_costs(|c| c.file.as_ref().is_some_and(|f| f.told.contains(team)));
        let mut projects: BTreeMap<String, String> = self
            .team_projects_of(team)
            .into_iter()
            .map(|(_, name, board)| (board, name))
            .collect();
        let mut records = Vec::new();
        let mut tickets = Vec::new();
        if of.admin {
            for d in &of.view.received {
                let mine = d.author == of.me;
                for (day, got) in &d.days {
                    let member = if rules.by_member {
                        got.member.clone()
                    } else {
                        None
                    };
                    for r in &got.rows {
                        records.push(Record {
                            member: member.clone(),
                            mine,
                            project: r.project.clone(),
                            day: day.clone(),
                            harness: r.harness.clone(),
                            model: r.model.clone(),
                            plan: r.plan,
                            input: r.input,
                            output: r.output,
                            cache: r.cache,
                            cost: r.cost,
                            quota: r.quota,
                        });
                    }
                    for t in &got.tickets {
                        tickets.push(TicketRecord {
                            member: member.clone(),
                            project: t.project.clone(),
                            day: day.clone(),
                            ticket: t.ticket.clone(),
                            tokens: t.input + t.output + t.cache,
                            cost: t.cost,
                        });
                    }
                }
            }
        } else {
            // A member sees itself: its own totals, from its statistics, over a year.
            let today = Local::now().date_naive();
            let first = today - chrono::Days::new(366);
            let days: BTreeSet<String> = first
                .iter_days()
                .take_while(|d| *d <= today)
                .map(|d| d.format("%Y-%m-%d").to_string())
                .collect();
            let own = self.own_totals(team, &days, start_of(first), None);
            for d in own.days {
                for r in d.rows {
                    records.push(Record {
                        member: Some(of.me.clone()),
                        mine: true,
                        project: r.project,
                        day: d.day.clone(),
                        harness: r.harness,
                        model: r.model,
                        plan: r.plan,
                        input: r.input,
                        output: r.output,
                        cache: r.cache,
                        cost: r.cost,
                        quota: r.quota,
                    });
                }
                for t in d.tickets {
                    tickets.push(TicketRecord {
                        member: Some(of.me.clone()),
                        project: t.project,
                        day: d.day.clone(),
                        ticket: t.ticket,
                        tokens: t.input + t.output + t.cache,
                        cost: t.cost,
                    });
                }
            }
        }
        for r in &records {
            projects
                .entry(r.project.clone())
                .or_insert_with(|| r.project.chars().take(8).collect());
        }
        let budgets = self.budgets_of(team, Some(&of.view), &of.me, of.admin);
        Ok(TeamCostsView {
            team: team.to_owned(),
            name: of.name,
            admin: of.admin,
            me: of.me,
            rules,
            told,
            epoch: of.view.epoch,
            holds: of.view.holds,
            stale: of.view.stale,
            pending: of.view.pending.clone(),
            sent: of.view.sent.clone(),
            records,
            tickets,
            summary: of.view.summary.clone(),
            projects,
            budgets,
        })
    }

    /// Starts the hourly pass of the teams' costs (with the teams, at the start).
    pub(crate) fn start_team_costs(self: &Arc<Self>) {
        let me = Arc::downgrade(self);
        tauri::async_runtime::spawn(async move {
            // A first pass soon after the start, then each minute (the totals go once an hour).
            let tick = std::time::Duration::from_millis(every_ms().clamp(200, 60_000) as u64);
            loop {
                tokio::time::sleep(tick).await;
                let Some(core) = me.upgrade() else { break };
                core.team_costs_pass(false).await;
            }
        });
    }
}
