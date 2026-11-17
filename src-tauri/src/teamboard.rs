//! The Kanban of a team's project (TS-6, `docs/TEAM-SERVER.md` §4, §2.5 and §4.3): its tickets are the team's,
//! live, through the project's journal (TS-5), and a ticket is taken by the server's claim before its agent starts.
//!
//! - **In step** ([`Core::sync_team_board`]): what changed on this machine since the last time (a field that differs
//!   from `TeamTicket::seen`) is written to the journal, a column by its holder under its claim's token; what a
//!   teammate changed is taken; a ticket made here gets its key from the server's counter; the holders and the
//!   conflicts are read. It runs after each change of the tickets (the save loop) and after each reading of the
//!   team (its live feed: a teammate's change is seen within a second).
//! - **Taking a ticket** ([`Core::team_take`]): « Lancer », the autopilot and the MCP server's `start_ticket` go
//!   through it before an agent starts. Offline it is refused, said so; held by another, refused with their name.
//!   The lease is renewed every 3 minutes while it is « En cours » here ([`Core::renew_team_claims`]), and given back
//!   when it leaves the column.
//! - **The autopilot** (C11-A) takes only what it is given: the team's tickets assigned to this machine's member, or
//!   those of « À faire » when `BoardSettings::team_take_todo` is on (`board::to_start`); never one held, nor one
//!   whose holder's lease ran out (« interrompu chez Alice »: « Reprendre » does it by hand).
//! - **« Ne pas partager »** (`Ticket::team_local`): a ticket of this machine alone, of which nothing leaves.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use escouade_team::client::ClientError;
use escouade_team::crypto::ops::{OpBody, Value};
use escouade_team::crypto::{DeviceId, MemberId, ObjectId, ProjectId};
use escouade_team::replica::Fence;
use escouade_team::wire::Claim;
use escouade_team::Error as TeamError;
use serde_json::json;

use crate::core::Core;
use crate::model::{
    new_id, Column, Project, TeamConflict, TeamHolder, TeamTicket, Ticket, UiEvent,
};
use crate::teams::Teams;

/// The fields of a ticket its team shares, as the journal names them.
pub const SHARED_FIELDS: &[&str] = &[
    "key",
    "title",
    "description",
    "criteria",
    "column",
    "rank",
    "maxLoops",
    "progress",
    "partial",
    "iteration",
    "assignee",
    "formerKeys",
];

/// How often the holder renews its lease (of 10 minutes on the server): 3 minutes, or
/// `ESCOUADE_TEAM_RENEW_MS` in a build of development (the end-to-end tests).
pub fn renew_every() -> Duration {
    std::env::var("ESCOUADE_TEAM_RENEW_MS")
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(Duration::from_secs(180), |ms| {
            Duration::from_millis(ms.max(200))
        })
}

/// A machine that renewed nothing for twice the renewal no longer answers (« injoignable depuis 12 min »).
const UNREACHABLE_AFTER_S: i64 = 2 * 180;

/// The fields of a ticket the team shares, as this machine has them.
pub fn fields_of(t: &Ticket) -> BTreeMap<String, Value> {
    let mut m = BTreeMap::new();
    m.insert("key".into(), json!(t.key));
    m.insert("title".into(), json!(t.title));
    m.insert("description".into(), json!(t.description));
    m.insert(
        "criteria".into(),
        serde_json::to_value(&t.criteria).unwrap_or(Value::Null),
    );
    m.insert(
        "column".into(),
        serde_json::to_value(t.column).unwrap_or(Value::Null),
    );
    m.insert("rank".into(), json!(t.rank));
    m.insert("maxLoops".into(), json!(t.max_loops));
    m.insert("progress".into(), json!(t.progress));
    m.insert("partial".into(), json!(t.partial));
    m.insert("iteration".into(), json!(t.iteration));
    m.insert(
        "assignee".into(),
        json!(t.team.as_ref().and_then(|x| x.assignee.clone())),
    );
    // Only once it has some (TS-7): the tickets of before are not written again for an empty list.
    if !t.former_keys.is_empty() {
        m.insert("formerKeys".into(), json!(t.former_keys));
    }
    m
}

/// Puts a field the journal holds into the ticket (one it cannot read is left as it is).
pub fn apply_field(t: &mut Ticket, field: &str, v: &Value) {
    match field {
        "key" => {
            if let Some(s) = v.as_str().filter(|s| !s.is_empty()) {
                t.key = s.to_string();
            }
        }
        "title" => t.title = v.as_str().unwrap_or_default().to_string(),
        "description" => t.description = v.as_str().unwrap_or_default().to_string(),
        "criteria" => {
            if let Ok(c) = serde_json::from_value(v.clone()) {
                t.criteria = c;
            }
        }
        "column" => {
            if let Ok(c) = serde_json::from_value(v.clone()) {
                t.column = c;
            }
        }
        "rank" => {
            if let Some(r) = v.as_i64() {
                t.rank = r;
            }
        }
        "maxLoops" => {
            if let Some(n) = v.as_u64() {
                t.max_loops = n as u32;
            }
        }
        "progress" => {
            if let Ok(p) = serde_json::from_value(v.clone()) {
                t.progress = p;
            }
        }
        "partial" => t.partial = v.as_bool().unwrap_or(false),
        "iteration" => {
            if let Some(n) = v.as_u64() {
                t.iteration = n as u32;
            }
        }
        "assignee" => {
            if let Some(tt) = t.team.as_mut() {
                tt.assignee = v.as_str().map(str::to_string);
            }
        }
        "formerKeys" => {
            if let Ok(k) = serde_json::from_value::<Vec<String>>(v.clone()) {
                t.former_keys = k;
            }
        }
        _ => {}
    }
}

/// Who holds a ticket, as the card says it, from its claim and the server's time (seconds).
pub fn holder_of(
    claim: &Claim,
    now: i64,
    me: &DeviceId,
    name: Option<String>,
) -> Option<TeamHolder> {
    let device = claim.holder.as_ref()?;
    let silent = now - claim.renewed_at;
    Some(TeamHolder {
        name: name.unwrap_or_default(),
        device: hex::encode(device),
        mine: device.as_slice() == me.0.as_slice(),
        since: claim.claimed_at * 1000,
        until: claim.until * 1000,
        interrupted: claim.until <= now,
        unreachable_min: (silent > UNREACHABLE_AFTER_S).then_some((silent / 60) as u32),
    })
}

/// The prefix of the keys and the key of a ticket the server numbered.
pub fn team_key(prefix: &str, n: u64) -> String {
    format!("{prefix}-{n}")
}

/// A key of this machine (« Ne pas partager », or while the server is not reached): never one of the team's numbers.
pub fn provisional_key(prefix: &str, n: u32) -> String {
    format!("{prefix}-L{n}")
}

fn object_of(hex_id: &str) -> Option<ObjectId> {
    let mut out = [0u8; 16];
    hex::decode_to_slice(hex_id, &mut out).ok()?;
    Some(ObjectId(out))
}

fn random16() -> [u8; 16] {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("the system gives randomness");
    b
}

/// What a refused claim says, in the interface's language.
pub fn refusal(key: &str, e: &TeamError, holder: Option<&str>) -> String {
    match e {
        TeamError::Client(ClientError::Net(_)) => tr!(
            "Hors ligne : {key} est un ticket de l’équipe, il ne se prend qu’avec son serveur. Réessaie une fois en ligne.",
            "Offline: {key} is a ticket of the team, taken only through its server. Try again once online.",
            key = key
        ),
        TeamError::Client(ClientError::Claim { code, .. }) if code == "taken" => {
            let who = holder
                .filter(|n| !n.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| tr!("un collègue", "a teammate"));
            tr!(
                "{key} est pris par {who} : un ticket de l’équipe a un seul preneur.",
                "{key} is taken by {who}: a ticket of the team has one holder.",
                key = key,
                who = who
            )
        }
        TeamError::Client(ClientError::Claim { code, .. }) if code == "stale_version" => tr!(
            "{key} vient de changer chez un collègue : relis-le, puis relance-le.",
            "{key} just changed on a teammate’s machine: read it again, then start it again.",
            key = key
        ),
        TeamError::NoAccess => tr!(
            "Cet appareil n’a pas accès à ce projet de l’équipe.",
            "This device has no access to this project of the team."
        ),
        e => tr!(
            "{key} n’a pas pu être pris : {e}",
            "{key} could not be taken: {e}",
            key = key,
            e = e.to_string()
        ),
    }
}

/// What refuses at once the start of a ticket a teammate holds (its lease not run out): their name. The claim decides
/// the rest at the start.
pub fn start_refusal(t: &Ticket) -> Option<String> {
    let h = t.team.as_ref()?.holder.as_ref()?;
    if h.mine || h.interrupted {
        return None;
    }
    Some(refusal(
        &t.key,
        &TeamError::Client(ClientError::Claim {
            code: "taken".into(),
            claim: Box::new(Claim {
                ticket: Vec::new(),
                holder: None,
                until: 0,
                token: 0,
                version: 0,
                claimed_at: 0,
                renewed_at: 0,
                last_holder: None,
            }),
        }),
        Some(&h.name),
    ))
}

/// One reading of the boards at a time; one asked meanwhile runs once after it.
static BUSY: AtomicBool = AtomicBool::new(false);
static AGAIN: AtomicBool = AtomicBool::new(false);
static ONE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A ticket of this machine after a reading: its id, its team's state, its fields as the reading found them, and
/// those a teammate changed.
type Update = (
    String,
    TeamTicket,
    BTreeMap<String, Value>,
    BTreeMap<String, Value>,
);

/// What a reading brings back to the tickets, applied at once under their lock.
#[derive(Default)]
struct Outcome {
    /// For a ticket of this machine: its team's state, and the fields a teammate changed (applied when this machine
    /// did not change them meanwhile).
    updates: Vec<Update>,
    added: Vec<Ticket>,
    removed: Vec<String>,
    prefix: Option<String>,
    /// The claims were read from the server.
    claims_read: bool,
}

impl Core {
    /// The teams of this data folder, for the Kanban of their projects (set at the start).
    pub(crate) fn set_teams(self: &Arc<Self>, teams: Arc<Teams>) {
        *self.team_link.write() = Some(teams.clone());
        let me = Arc::downgrade(self);
        teams.set_projects_sink(Arc::new(move |_| {
            if let Some(core) = me.upgrade() {
                core.team_boards_soon();
                core.team_join_listed_soon();
            }
        }));
        let me = Arc::downgrade(self);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(renew_every()).await;
                let Some(core) = me.upgrade() else { break };
                core.renew_team_claims().await;
            }
        });
        self.team_boards_soon();
    }

    pub(crate) fn teams_link(&self) -> Option<Arc<Teams>> {
        self.team_link.read().clone()
    }

    /// The team and the journal of a project's Kanban, when it is a team's.
    fn team_project_of(&self, project: &Project) -> Option<(Arc<Teams>, String, ProjectId)> {
        let team = project.team.clone()?;
        let pid = object_of(project.team_project.as_deref()?)?;
        Some((self.teams_link()?, team, ProjectId(pid.0)))
    }

    /// Reads every team's Kanban soon, off the caller: after a change of the tickets, after a reading of a team.
    pub(crate) fn team_boards_soon(self: &Arc<Self>) {
        let any = self
            .projects
            .read()
            .iter()
            .any(|p| p.team.is_some() && p.team_project.is_some());
        if !any || self.teams_link().is_none() {
            return;
        }
        if BUSY.swap(true, Ordering::AcqRel) {
            AGAIN.store(true, Ordering::Release);
            return;
        }
        let core = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                AGAIN.store(false, Ordering::Release);
                let ids: Vec<String> = core
                    .projects
                    .read()
                    .iter()
                    .filter(|p| p.team.is_some() && p.team_project.is_some())
                    .map(|p| p.id.clone())
                    .collect();
                for id in ids {
                    core.sync_team_board(&id).await;
                }
                if !AGAIN.swap(false, Ordering::AcqRel) {
                    break;
                }
            }
            BUSY.store(false, Ordering::Release);
            // Asked between the last check and now.
            if AGAIN.swap(false, Ordering::AcqRel) {
                core.team_boards_soon();
            }
        });
    }

    /// Brings a team's Kanban in step with its journal (the module's doc): what changed here written, what a teammate
    /// changed taken, the keys, the holders and the conflicts read. Offline, what is written waits in the journal's
    /// outbox and the rest is the copy's.
    pub(crate) async fn sync_team_board(self: &Arc<Self>, project_id: &str) {
        let _one = ONE.lock().await;
        let Ok(project) = self.project(project_id) else {
            return;
        };
        let Some((teams, team_id, pid)) = self.team_project_of(&project) else {
            return;
        };
        let Ok(shared) = teams.team(&team_id) else {
            return;
        };
        let my_name = self.me_in(&project).name;
        let snapshot: Vec<Ticket> = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == project_id && !t.team_local)
            .cloned()
            .collect();
        let deleted: Vec<String> = {
            let mut d = self.team_deleted.lock();
            let (mine, rest): (Vec<_>, Vec<_>) = d.drain(..).partition(|(p, _)| p == project_id);
            *d = rest;
            mine.into_iter().map(|(_, o)| o).collect()
        };
        // The claims read again on a claim of the live feed, else every 5 s at most: the server bounds each device's rate.
        let fresh = teams.take_claim_frame(&team_id)
            || self
                .team_claims_at
                .lock()
                .get(project_id)
                .is_none_or(|at| at.elapsed() > Duration::from_secs(5));
        let mut team = shared.lock().await;
        let out = match board_in_step(
            &mut team, pid, &project, &snapshot, &deleted, &my_name, fresh,
        )
        .await
        {
            Ok(out) => out,
            Err(e) => {
                log::info!("team board of {}: {e:#}", project.name);
                return;
            }
        };
        drop(team);
        if out.claims_read {
            self.team_claims_at
                .lock()
                .insert(project_id.to_string(), std::time::Instant::now());
        }
        self.apply_board(project_id, out);
    }

    /// What a reading brought, applied to the tickets: a field a teammate changed is taken when this machine did not
    /// change it since the reading began (else it goes out at the next one).
    fn apply_board(self: &Arc<Self>, project_id: &str, out: Outcome) {
        let mut changed: Vec<Ticket> = Vec::new();
        let mut any = false;
        if let Some(prefix) = out.prefix {
            let mut projects = self.projects.write();
            if let Some(p) = projects.iter_mut().find(|p| p.id == project_id) {
                if p.board.prefix != prefix {
                    p.board.prefix = prefix;
                    any = true;
                }
            }
        }
        {
            let mut tickets = self.tickets.write();
            for (id, tt, read, pulled) in out.updates {
                let Some(t) = tickets.iter_mut().find(|t| t.id == id) else {
                    continue;
                };
                let before = t.clone();
                let here = fields_of(t);
                t.team = Some(tt);
                for (k, v) in &pulled {
                    // Changed here since: this machine's goes out next time.
                    if here.get(k) != read.get(k) {
                        continue;
                    }
                    apply_field(t, k, v);
                }
                if *t != before {
                    changed.push(t.clone());
                }
            }
            for t in out.added {
                if tickets.iter().any(|x| {
                    x.team.as_ref().map(|y| &y.object) == t.team.as_ref().map(|y| &y.object)
                }) {
                    continue;
                }
                // Poured by a teammate from the files of ESC-190 this machine read too (TS-7): its own ticket of that
                // file becomes the team's, never a second one.
                let object = t
                    .team
                    .as_ref()
                    .map(|y| y.object.clone())
                    .unwrap_or_default();
                if let Some(x) = tickets.iter_mut().find(|x| {
                    x.project_id == project_id
                        && x.team.is_none()
                        && x.shared
                            .as_ref()
                            .is_some_and(|s| crate::teampour::object_of_file(&s.id) == object)
                }) {
                    let fields = fields_of(&t);
                    x.team = t.team.clone();
                    x.shared = None;
                    x.team_local = false;
                    for (k, v) in &fields {
                        apply_field(x, k, v);
                    }
                    x.former_keys = t.former_keys.clone();
                    changed.push(x.clone());
                    continue;
                }
                changed.push(t.clone());
                tickets.push(t);
            }
            for id in &out.removed {
                tickets.retain(|t| t.id != *id);
            }
        }
        for id in out.removed {
            any = true;
            self.hub.emit(UiEvent::TicketRemoved {
                id,
                project_id: project_id.to_string(),
            });
        }
        if any {
            self.emit_project(project_id);
        }
        if changed.is_empty() && !any {
            return;
        }
        for t in changed {
            self.hub.emit(UiEvent::Ticket { ticket: t });
        }
        self.request_save();
        self.schedule();
    }

    /// Takes a ticket of the team before its agent starts here (« Lancer », the autopilot, `start_ticket`): Err with
    /// why, said in the interface's language (offline, held by someone, no access). A ticket of no team passes.
    pub(crate) async fn team_take(
        self: &Arc<Self>,
        project: &Project,
        id: &str,
    ) -> Result<(), String> {
        let Some((teams, team_id, pid)) = self.team_project_of(project) else {
            return Ok(());
        };
        let Ok(t) = self.ticket(id) else {
            return Ok(());
        };
        if t.team_local {
            return Ok(());
        }
        if t.team.as_ref().is_none_or(|x| x.object.is_empty()) {
            // Not in the journal yet (made a moment ago): put it there first.
            self.sync_team_board(&project.id).await;
        }
        let t = self.ticket(id).map_err(|e| e.to_string())?;
        let Some(tt) = t.team.clone().filter(|x| !x.object.is_empty()) else {
            return Err(refusal(
                &t.key,
                &TeamError::Client(ClientError::Net("not in the journal".into())),
                None,
            ));
        };
        let claim = object_of(&tt.claim)
            .ok_or_else(|| "a claim id".to_string())?
            .0;
        let shared = teams.team(&team_id)?;
        let mut team = shared.lock().await;
        let expected = (tt.version > 0).then_some(tt.version);
        let taken = team.take_claim(pid, claim, expected).await;
        let answer = match taken {
            Ok(a) => a,
            Err(e) => {
                let holder = match &e {
                    TeamError::Client(ClientError::Claim { claim, .. }) => claim
                        .holder
                        .as_ref()
                        .and_then(|h| <[u8; 16]>::try_from(h.as_slice()).ok())
                        .and_then(|h| team.name_of(pid, &DeviceId(h))),
                    _ => None,
                };
                return Err(refusal(&t.key, &e, holder.as_deref()));
            }
        };
        let me = DeviceId(team.client().device());
        let name = team.name_of(pid, &me);
        drop(team);
        let holder = holder_of(&answer.claim, answer.now, &me, name);
        let _ = self.edit_ticket(id, |x| {
            if let Some(tt) = x.team.as_mut() {
                tt.token = Some(answer.claim.token);
                tt.version = answer.claim.version;
                tt.holder = holder.clone();
                tt.refused = None;
            }
            Ok(())
        });
        Ok(())
    }

    /// Renews the lease of every ticket of the team this machine holds « En cours »: one another took meanwhile says
    /// so on its card (its agent goes on: the user hands over or stops it).
    pub(crate) async fn renew_team_claims(self: &Arc<Self>) {
        let held: Vec<Ticket> = self
            .tickets
            .read()
            .iter()
            .filter(|t| {
                t.column == Column::Doing && t.team.as_ref().is_some_and(|x| x.token.is_some())
            })
            .cloned()
            .collect();
        for t in held {
            let Ok(project) = self.project(&t.project_id) else {
                continue;
            };
            let Some((teams, team_id, pid)) = self.team_project_of(&project) else {
                continue;
            };
            let Ok(shared) = teams.team(&team_id) else {
                continue;
            };
            let tt = t.team.clone().unwrap_or_default();
            let (Some(claim), Some(token)) = (object_of(&tt.claim), tt.token) else {
                continue;
            };
            let mut team = shared.lock().await;
            let me = DeviceId(team.client().device());
            let renewed = team.renew_claim(pid, claim.0, token).await;
            let update = match renewed {
                Ok(a) => {
                    let name = team.name_of(pid, &me);
                    Some((holder_of(&a.claim, a.now, &me, name), Some(token), None))
                }
                Err(TeamError::Client(ClientError::Claim { claim: c, .. })) => {
                    let who = c
                        .holder
                        .as_ref()
                        .and_then(|h| <[u8; 16]>::try_from(h.as_slice()).ok())
                        .and_then(|h| team.name_of(pid, &DeviceId(h)))
                        .unwrap_or_else(|| tr!("un collègue", "a teammate"));
                    Some((
                        None,
                        None,
                        Some(tr!(
                            "Repris par {who} pendant une coupure : ses déplacements ne partent plus d’ici.",
                            "Taken over by {who} during a cut: its moves no longer leave from here.",
                            who = who
                        )),
                    ))
                }
                Err(_) => None,
            };
            drop(team);
            if let Some((holder, token, refused)) = update {
                let _ = self.edit_ticket(&t.id, |x| {
                    if let Some(tt) = x.team.as_mut() {
                        tt.token = token;
                        if holder.is_some() || refused.is_some() {
                            tt.holder = holder.clone();
                        }
                        if refused.is_some() {
                            tt.refused = refused.clone();
                        }
                    }
                    Ok(())
                });
            }
        }
    }

    /// Makes a project's Kanban a team's (TS-6): a new journal of the whole team (`None`), or the one of `team_project`
    /// (its id, hex) a teammate made, followed. The project must be of a team. The tickets already there stay on this
    /// machine (« Ne pas partager »): bringing them in is TS-7's.
    pub(crate) async fn link_team_board(
        self: &Arc<Self>,
        project_id: &str,
        team_project: Option<String>,
    ) -> Result<Project> {
        let project = self.project(project_id)?;
        let team_id = project.team.clone().ok_or_else(|| {
            anyhow!(tr!(
                "Ce projet n’est d’aucune équipe : coche-le d’abord dans l’écran de l’équipe.",
                "This project is of no team: check it first in the team’s screen."
            ))
        })?;
        let teams = self.teams_link().ok_or_else(|| anyhow!("no teams"))?;
        let shared = teams.team(&team_id).map_err(|e| anyhow!(e))?;
        let pid = {
            let mut team = shared.lock().await;
            let pid = match &team_project {
                Some(hex_id) => {
                    let o = object_of(hex_id)
                        .ok_or_else(|| anyhow!("an id of 32 hexadecimal digits"))?;
                    let pid = ProjectId(o.0);
                    team.follow(pid).map_err(|e| anyhow!(e.to_string()))?;
                    team.sync_project(pid)
                        .await
                        .map_err(|e| anyhow!(crate::teams::say(&e)))?;
                    pid
                }
                None => {
                    let pid = team
                        .create_project(None)
                        .await
                        .map_err(|e| anyhow!(crate::teams::say(&e)))?;
                    let prefix = if project.board.prefix.is_empty() {
                        crate::board::key_prefix(&project.name)
                    } else {
                        project.board.prefix.clone()
                    };
                    let o = ObjectId(pid.0);
                    team.set_field(pid, o, "prefix", json!(prefix))
                        .map_err(|e| anyhow!(e.to_string()))?;
                    team.set_field(pid, o, "name", json!(project.name))
                        .map_err(|e| anyhow!(e.to_string()))?;
                    let _ = team.sync_project(pid).await;
                    let _ = team.sync_access(pid).await;
                    pid
                }
            };
            pid
        };
        let project = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(crate::core::project_not_found)?;
            p.team_project = Some(hex::encode(pid.0));
            p.clone()
        };
        {
            let mut tickets = self.tickets.write();
            for t in tickets
                .iter_mut()
                .filter(|t| t.project_id == project_id && t.team.is_none())
            {
                t.team_local = true;
            }
        }
        self.hub.emit(UiEvent::Project {
            project: project.clone(),
        });
        self.request_save();
        // ESC-190's files are frozen from now on (C5-A): Escouade writes them no more, and says so at their head.
        crate::sharing::freeze(&crate::sharing::dir_of(&project));
        // What the project had before stays here until the gesture (TS-7): said once.
        self.hub.emit(UiEvent::Toast {
            text: tr!(
                "{name} : son Kanban est celui de l’équipe. Ses tickets et ses docs d’avant restent sur ce poste : « Verser à l’équipe » les envoie, après un aperçu.",
                "{name}: its Kanban is the team’s. Its tickets and docs of before stay on this machine: “Pour into the team” sends them, after a preview.",
                name = project.name
            ),
        });
        if team_project.is_none() {
            // A new journal: the team's list learns it (TS-7), written by an administrator, proposed to one otherwise.
            if let Some(said) = self
                .team_list_board(&project, &team_id, &hex::encode(pid.0))
                .await
            {
                self.hub.emit(UiEvent::Toast { text: said });
            }
        }
        self.sync_team_board(project_id).await;
        self.project(project_id)
    }

    /// « Reprendre » a ticket whose holder's lease ran out (« interrompu chez Alice »): back to « À faire » here and
    /// launched, which takes it.
    pub(crate) fn team_resume(self: &Arc<Self>, id: &str) -> Result<Ticket> {
        let t = self.edit_ticket(id, |t| {
            let Some(tt) = t.team.as_ref() else {
                bail!(tr!(
                    "Ce ticket n’est pas de l’équipe.",
                    "This ticket is not the team’s."
                ));
            };
            if tt
                .holder
                .as_ref()
                .is_some_and(|h| !h.mine && !h.interrupted)
            {
                bail!(tr!(
                    "Un collègue le tient encore : il se reprend une fois son bail fini.",
                    "A teammate still holds it: it is taken up once their lease is over."
                ));
            }
            t.column = Column::Todo;
            t.forced = true;
            t.started_at = None;
            Ok(t.clone())
        })?;
        self.hub.emit(UiEvent::Ticket { ticket: t.clone() });
        self.request_save();
        self.schedule();
        Ok(t)
    }

    /// « Reprendre le mien » (`keep_mine`) or « Prendre le sien » of a conflict of a field: this machine's value written
    /// again after the teammate's, or theirs kept; the conflict is not shown again.
    pub(crate) fn team_settle(
        self: &Arc<Self>,
        id: &str,
        field: &str,
        keep_mine: bool,
    ) -> Result<Ticket> {
        let t = self.edit_ticket(id, |t| {
            let Some(tt) = t.team.as_mut() else {
                bail!(tr!(
                    "Ce ticket n’est pas de l’équipe.",
                    "This ticket is not the team’s."
                ));
            };
            let Some(i) = tt.conflicts.iter().position(|c| c.field == field) else {
                return Ok(t.clone());
            };
            let c = tt.conflicts.remove(i);
            tt.dismissed.push(c.stamp.clone());
            if keep_mine {
                apply_field(t, &c.field, &c.mine);
            }
            Ok(t.clone())
        })?;
        self.hub.emit(UiEvent::Ticket { ticket: t.clone() });
        self.request_save();
        Ok(t)
    }

    /// « Me l’assigner » (`me`) or « Ne plus me l’assigner »: the autopilot of the member it is assigned to takes it.
    pub(crate) async fn team_assign(self: &Arc<Self>, id: &str, me: bool) -> Result<Ticket> {
        let t = self.ticket(id)?;
        let project = self.project(&t.project_id)?;
        let (teams, team_id, _) = self.team_project_of(&project).ok_or_else(|| {
            anyhow!(tr!(
                "Ce ticket n’est pas de l’équipe.",
                "This ticket is not the team’s."
            ))
        })?;
        let member = {
            let shared = teams.team(&team_id).map_err(|e| anyhow!(e))?;
            let team = shared.lock().await;
            hex::encode(team.identity().member.0)
        };
        let name = self.me_in(&project).name;
        let t = self.edit_ticket(id, |t| {
            let Some(tt) = t.team.as_mut() else {
                bail!(tr!(
                    "Ce ticket n’est pas de l’équipe.",
                    "This ticket is not the team’s."
                ));
            };
            tt.assignee = me.then(|| member.clone());
            tt.assignee_name = me.then(|| name.clone());
            tt.assigned_me = me;
            Ok(t.clone())
        })?;
        self.hub.emit(UiEvent::Ticket { ticket: t.clone() });
        self.request_save();
        self.schedule();
        Ok(t)
    }

    /// « Ne pas partager » (`local`) or « Partager avec l’équipe » of a ticket of a team's Kanban: one shared leaves the
    /// journal (deleted there), one of this machine goes in at the next reading.
    pub(crate) fn team_set_local(self: &Arc<Self>, id: &str, local: bool) -> Result<Ticket> {
        let t = self.edit_ticket(id, |t| {
            if !local && t.team_local {
                t.team_local = false;
            }
            if local && !t.team_local {
                if t.team
                    .as_ref()
                    .is_some_and(|x| x.holder.as_ref().is_some_and(|h| !h.mine && !h.interrupted))
                {
                    bail!(tr!(
                        "Un collègue le tient : il ne quitte pas l’équipe.",
                        "A teammate holds it: it does not leave the team."
                    ));
                }
                t.team_local = true;
            }
            Ok(t.clone())
        })?;
        if local {
            if let Some(tt) = t.team.clone() {
                self.team_deleted
                    .lock()
                    .push((t.project_id.clone(), tt.object));
                let _ = self.edit_ticket(id, |x| {
                    x.team = None;
                    Ok(())
                });
            }
        }
        let t = self.ticket(id)?;
        self.hub.emit(UiEvent::Ticket { ticket: t.clone() });
        self.request_save();
        Ok(t)
    }

    /// A ticket of the team deleted here leaves the journal at the next reading.
    pub(crate) fn team_forget_ticket(&self, t: &Ticket) {
        if let Some(tt) = &t.team {
            self.team_deleted
                .lock()
                .push((t.project_id.clone(), tt.object.clone()));
        }
    }
}

/// The member names of the team, by member (from the names its devices wrote in the project).
fn member_names(team: &mut escouade_team::Team, pid: ProjectId) -> BTreeMap<MemberId, String> {
    let names = team.names(pid).unwrap_or_default();
    let mut out = BTreeMap::new();
    for (m, _, devices) in team.members().unwrap_or_default() {
        if let Some(n) = devices.iter().find_map(|d| names.get(d)) {
            out.insert(m, n.clone());
        }
    }
    out
}

/// The work of a reading, under the team's lock (the module's doc).
async fn board_in_step(
    team: &mut escouade_team::Team,
    pid: ProjectId,
    project: &Project,
    snapshot: &[Ticket],
    deleted: &[String],
    my_name: &str,
    fresh: bool,
) -> Result<Outcome, TeamError> {
    let me = DeviceId(team.client().device());
    // Offline as last heard (the team's watch and its live feed find the server again): nothing is asked of it, the
    // writes wait in the outbox (a closed port can take seconds to refuse a connection).
    let online = team.sync_state().online != Some(false);
    let my_member = team.identity().member;
    let admin = team.is_admin().unwrap_or(false);
    let _ = team.set_name(pid, my_name);
    for o in deleted.iter().filter_map(|o| object_of(o)) {
        let _ = team.delete_ticket(pid, o);
    }
    let mut out = Outcome::default();
    let pending_before = team.pending(&pid).unwrap_or(0);
    // The project's prefix, as the journal says it: the keys are the team's.
    let prefix = team
        .replica(&pid)?
        .object(&ObjectId(pid.0))?
        .and_then(|o| {
            o.fields
                .get("prefix")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| project.board.prefix.clone());
    if prefix != project.board.prefix {
        out.prefix = Some(prefix.clone());
    }
    // What changed here, written.
    let mut states: BTreeMap<String, TeamTicket> = BTreeMap::new();
    let mut reverts: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    let mut releases: Vec<(String, [u8; 16], u64)> = Vec::new();
    for lt in snapshot {
        let now = fields_of(lt);
        let mut tt = match lt.team.as_ref().filter(|x| !x.object.is_empty()) {
            None => {
                let claim = random16();
                let mut f = now.clone();
                f.insert("claim".into(), json!(hex::encode(claim)));
                let object = team.create_ticket(pid, f)?;
                TeamTicket {
                    object: hex::encode(object.0),
                    claim: hex::encode(claim),
                    seen: now.clone(),
                    // An imported ticket keeps its key (§4.1).
                    keyed: lt.external.is_some(),
                    ..lt.team.clone().unwrap_or_default()
                }
            }
            Some(tt) => tt.clone(),
        };
        let Some(object) = object_of(&tt.object) else {
            continue;
        };
        for (k, v) in &now {
            if tt.seen.get(k) == Some(v) {
                continue;
            }
            if k == "column" {
                // A ticket held: its column changes by its holder (its token) or an administrator alone (§4.3).
                let held = tt
                    .holder
                    .as_ref()
                    .is_some_and(|h| !h.mine && !h.interrupted);
                if held && !admin {
                    if let Some(seen) = tt.seen.get(k) {
                        reverts
                            .entry(lt.id.clone())
                            .or_default()
                            .insert(k.clone(), seen.clone());
                    }
                    tt.refused = Some(tr!(
                        "Pris par {who} : sa colonne ne change que par lui ou un administrateur.",
                        "Taken by {who}: its column changes by its holder or an administrator alone.",
                        who = tt.holder.as_ref().map(|h| h.name.clone()).unwrap_or_default()
                    ));
                    continue;
                }
                let claim = object_of(&tt.claim).map(|o| o.0);
                let fence = match (claim, tt.token) {
                    (Some(claim), Some(token)) => Some(Fence { claim, token }),
                    _ => None,
                };
                let base = team.field_stamp(&pid, &object, "column")?;
                team.write_fenced(
                    pid,
                    OpBody::Move {
                        object,
                        column: v.as_str().unwrap_or("todo").to_string(),
                        rank: None,
                        base,
                    },
                    fence,
                )?;
            } else {
                team.set_field(pid, object, k, v.clone())?;
            }
            tt.seen.insert(k.clone(), v.clone());
        }
        // Gone out of « En cours » here: the claim given back, once its moves left (they carry its token).
        if let (Some(token), Some(claim)) = (tt.token, object_of(&tt.claim)) {
            if lt.column != Column::Doing {
                releases.push((lt.id.clone(), claim.0, token));
            }
        }
        states.insert(lt.id.clone(), tt);
    }
    // The keys of the team, from the server's counter.
    let mut keys: BTreeMap<String, String> = BTreeMap::new();
    for (id, tt) in states.iter_mut().filter(|(_, tt)| !tt.keyed && online) {
        let Some(object) = object_of(&tt.object) else {
            continue;
        };
        match team.next_key(pid).await {
            Ok(n) => {
                let key = team_key(&prefix, n);
                if team.set_field(pid, object, "key", json!(key)).is_err() {
                    break;
                }
                tt.seen.insert("key".into(), json!(key));
                tt.keyed = true;
                keys.insert(id.clone(), key);
            }
            Err(e) => {
                log::debug!("team board: no key from the server: {e}");
                break;
            }
        }
    }
    // Sent, and read (the team's reading reads the streams; what this machine wrote goes at once). A refusal of the
    // server never loses what was written here: it waits in the outbox, and the states of the tickets are kept.
    let wrote = team.pending(&pid).unwrap_or(0) > 0 || pending_before > 0 || !releases.is_empty();
    if online && (wrote || fresh) {
        if let Err(e) = team.sync_project(pid).await {
            log::info!("team board: not sent yet: {e}");
        }
    }
    for (id, claim, token) in releases.into_iter().filter(|_| online) {
        match team.release_claim(pid, claim, token).await {
            Ok(_) | Err(TeamError::Client(ClientError::Claim { .. })) => {
                if let Some(tt) = states.get_mut(&id) {
                    tt.token = None;
                    tt.holder = None;
                }
            }
            Err(_) => {}
        }
    }
    let refused = team.take_refused();
    let claims = if online && (fresh || wrote) {
        match team.claims(pid).await {
            Ok(c) => {
                out.claims_read = true;
                Some(c)
            }
            Err(e) => {
                log::debug!("team board: claims not read: {e}");
                None
            }
        }
    } else {
        None
    };
    let names = team.names(pid).unwrap_or_default();
    let members = member_names(team, pid);
    let name_of = |d: &DeviceId| -> String {
        names
            .get(d)
            .cloned()
            .unwrap_or_else(|| tr!("un collègue", "a teammate"))
    };
    // What the journal holds, taken.
    let objects = team.replica(&pid)?.objects(Some("ticket"))?;
    let by_object: BTreeMap<String, String> = snapshot
        .iter()
        .filter_map(|t| {
            t.team
                .as_ref()
                .filter(|x| !x.object.is_empty())
                .map(|x| (x.object.clone(), t.id.clone()))
        })
        .chain(
            states
                .iter()
                .map(|(id, tt)| (tt.object.clone(), id.clone())),
        )
        .collect();
    let mut merged = false;
    let mut seen_objects: BTreeSet<String> = BTreeSet::new();
    for o in objects {
        let object = hex::encode(o.id.0);
        seen_objects.insert(object.clone());
        let fields: BTreeMap<String, Value> = o
            .fields
            .iter()
            .filter(|(k, _)| SHARED_FIELDS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let claim_hex = o
            .fields
            .get("claim")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let local_id = by_object.get(&object).cloned();
        let mut tt = match &local_id {
            Some(id) => {
                let mut tt = states.get(id).cloned().unwrap_or_default();
                // Two devices poured the same ticket at once (TS-7): the claim the journal kept is the one.
                if !claim_hex.is_empty() && claim_hex != tt.claim && tt.token.is_none() {
                    tt.claim = claim_hex.clone();
                }
                tt
            }
            None => TeamTicket {
                object: object.clone(),
                claim: claim_hex.clone(),
                keyed: true,
                ..Default::default()
            },
        };
        // The holder, from the server's claim.
        if let Some(c) = &claims {
            let claim = c.claims.iter().find(|x| hex::encode(&x.ticket) == tt.claim);
            let holder = claim.and_then(|x| {
                let d = x
                    .holder
                    .as_ref()
                    .and_then(|h| <[u8; 16]>::try_from(h.as_slice()).ok())?;
                holder_of(x, c.now, &me, names.get(&DeviceId(d)).cloned())
            });
            if let Some(x) = claim {
                tt.version = x.version;
            }
            if tt.token.is_some() && !holder.as_ref().is_some_and(|h| h.mine) {
                tt.token = None;
            }
            tt.holder = holder;
        }
        if let Some(r) = refused
            .iter()
            .find(|r| hex::encode(&r.claim.ticket) == tt.claim)
        {
            let who = r
                .claim
                .holder
                .as_ref()
                .and_then(|h| <[u8; 16]>::try_from(h.as_slice()).ok())
                .map(|h| name_of(&DeviceId(h)))
                .unwrap_or_default();
            tt.token = None;
            tt.refused = Some(tr!(
                "Repris par {who} pendant une coupure : ton déplacement n’est pas parti.",
                "Taken over by {who} during a cut: your move did not leave.",
                who = who
            ));
        }
        // The conflicts this machine lost (§4.3); a description changed on both sides merges three ways.
        let mut conflicts = Vec::new();
        for c in team.replica(&pid)?.conflicts(&o.id)? {
            // The same value written on both sides is no conflict.
            if c.stamp.device != me
                || !SHARED_FIELDS.contains(&c.field.as_str())
                || c.value == c.winner
            {
                continue;
            }
            let stamp = hex::encode(c.stamp.to_bytes());
            if tt.dismissed.contains(&stamp) {
                continue;
            }
            let mut unmerged = false;
            if c.field == "description" {
                let base = c.base.as_ref().and_then(Value::as_str).unwrap_or_default();
                let mine = c.value.as_str().unwrap_or_default();
                let theirs = c.winner.as_str().unwrap_or_default();
                match escouade_team::merge::merge3(base, mine, theirs) {
                    Some(m) => {
                        if m != theirs {
                            team.set_field(pid, o.id, "description", json!(m))?;
                            merged = true;
                        }
                        tt.dismissed.push(stamp);
                        continue;
                    }
                    None => unmerged = true,
                }
            }
            conflicts.push(TeamConflict {
                field: c.field.clone(),
                by: name_of(&c.winner_stamp.device),
                stamp,
                mine: c.value.clone(),
                theirs: c.winner.clone(),
                unmerged,
            });
        }
        tt.conflicts = conflicts;
        tt.assignee = fields
            .get("assignee")
            .and_then(Value::as_str)
            .map(str::to_string);
        tt.assigned_me = tt.assignee.as_deref() == Some(hex::encode(my_member.0).as_str());
        tt.assignee_name = tt.assignee.as_ref().and_then(|a| {
            let m = object_of(a)?;
            members.get(&MemberId(m.0)).cloned()
        });
        tt.teammate = ["title", "description", "criteria"].iter().any(|f| {
            team.field_stamp(&pid, &o.id, f)
                .ok()
                .flatten()
                .is_some_and(|s| s.device != me)
        });
        // What a teammate changed: the fields that differ from what was seen.
        let pulled: BTreeMap<String, Value> = fields
            .iter()
            .filter(|(k, v)| tt.seen.get(*k) != Some(*v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (k, v) in &pulled {
            tt.seen.insert(k.clone(), v.clone());
        }
        match local_id {
            Some(id) => {
                let read = snapshot
                    .iter()
                    .find(|t| t.id == id)
                    .map(fields_of)
                    .unwrap_or_default();
                let mut pulled = pulled;
                if let Some(r) = reverts.remove(&id) {
                    pulled.extend(r);
                }
                if let Some(k) = keys.remove(&id) {
                    pulled.insert("key".into(), json!(k));
                }
                out.updates.push((id, tt, read, pulled));
            }
            None => {
                let mut t = Ticket {
                    id: new_id(),
                    project_id: project.id.clone(),
                    created_at: crate::model::now_ms(),
                    max_loops: 5,
                    team: Some(tt),
                    ..Default::default()
                };
                for (k, v) in &fields {
                    apply_field(&mut t, k, v);
                }
                out.added.push(t);
            }
        }
    }
    if merged {
        let _ = team.flush_project(pid).await;
    }
    // A ticket of the team the journal deleted: gone here too, unless this machine's agent holds it.
    for lt in snapshot {
        let Some(tt) = &lt.team else { continue };
        if !seen_objects.contains(&tt.object) {
            let Some(o) = object_of(&tt.object) else {
                continue;
            };
            if team.replica(&pid)?.is_deleted(&o)? && lt.agent_id.is_none() {
                out.removed.push(lt.id.clone());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "teamboard_tests.rs"]
pub(crate) mod tests;
