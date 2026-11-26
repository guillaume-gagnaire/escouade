//! Passing an agent to a teammate or to the team's machine (TS-17, `docs/TEAM-SERVER.md` §11, choice C16-A).
//!
//! - **What passes**, by the gesture of who passes it ([`Core::team_handoff`]), never by an agent (the MCP server has
//!   no tool for it, `get_ticket` says a passage under way):
//!   - the ticket: the server hands its claim to the device chosen, under a new token (§4.2), and this machine's old
//!     token writes nothing more;
//!   - the branch: pushed on its remote, by this gesture alone (the window says it before);
//!   - the uncommitted work: a commit « en cours » on the branch, or a patch in the deposit, as the user chose;
//!   - the conversation: Escouade's journal of the agent, in the deposit, a file of the server encrypted by the
//!     project's key; erased once taken, or when the passage is abandoned ([`Core::team_handoff_cancel`]).
//! - **When**: the agent is stopped between two turns; a turn under way is interrupted first.
//! - **On the teammate's machine** ([`Core::team_handoff_take`]): « Alice te passe DEM-41 · Prendre », in the box
//!   « À toi » and on the ticket's card. Escouade fetches the branch, makes a worktree, applies the patch, then a new
//!   session on this machine's harness and account starts, whose first message is ESC-88's handover file; the
//!   conversation of before is above it, to read. No account ever passes: a teammate without the agent's harness
//!   picks theirs.
//! - **To a machine in server mode**: the same gesture; the machine takes it at once, on its own harness.
//! - **Refused, with why, nothing lost**: a device out of the project, an agent of a personal project (or of a ticket
//!   kept on this machine), a branch whose remote does not answer (the commit « en cours » undone).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use base64::Engine as _;
use escouade_team::client::ClientError;
use escouade_team::crypto::DeviceId;
use escouade_team::handoff::Offer;
use escouade_team::Error as TeamError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::Core;
use crate::engine;
use crate::handover::{self, Dossier};
use crate::model::EngineId;
use crate::model::{new_id, now_ms, Column, TeamHandoff, UiEvent};
use crate::plan::PlanState;
use crate::teamboard::{holder_of, object_of, refusal};

/// The key of the team's file that says whose passages a machine in server mode takes on its own (SR3-05).
pub const MACHINE_TAKES: &str = "handoff.machineTakes";

/// Whose passages a machine in server mode takes on its own; the others wait in its box « À toi », for a human of the
/// machine to take them (« Prendre »). An administrator's by default: a member cannot make the team's machine run, day
/// and night with no one looking, what they pass to it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MachineTakes {
    Nobody,
    #[default]
    Admins,
    Members,
}

impl MachineTakes {
    /// As the version in force of the team's file says it (imposed first, then recommended): `nobody`, `admins` or
    /// `members`; anything else, an administrator's.
    pub fn of(e: &escouade_team::settings::Effective) -> MachineTakes {
        let v = e
            .enforced
            .get(MACHINE_TAKES)
            .or_else(|| e.defaults.get(MACHINE_TAKES));
        match v.and_then(Value::as_str) {
            Some("nobody") => MachineTakes::Nobody,
            Some("members") => MachineTakes::Members,
            _ => MachineTakes::Admins,
        }
    }

    /// Whether the machine takes on its own the passage of a device whose member is (or not) an administrator.
    pub fn takes(self, admin: bool) -> bool {
        match self {
            MachineTakes::Nobody => false,
            MachineTakes::Admins => admin,
            MachineTakes::Members => true,
        }
    }
}

/// The devices (hex) of the team's administrators now.
fn admin_devices(team: &escouade_team::Team) -> std::collections::HashSet<String> {
    team.members()
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, role, _)| *role == escouade_team::crypto::Role::Admin)
        .flat_map(|(_, _, ds)| ds.into_iter().map(|d| hex::encode(d.0)))
        .collect()
}

/// The branch of a passage, before it is fetched over the local one (SR3-06): a name git takes as a branch, and that
/// crushes no local branch other than the ticket's own (`release/2.3` passed would lose its commits not pushed).
pub(crate) async fn passed_branch_ok(root: &str, branch: &str, ticket_branch: &str) -> Result<()> {
    crate::git::check_ref_format(branch).await?;
    if branch != ticket_branch && crate::git::branch_exists(root, branch).await {
        bail!(tr!(
            "La branche {branch} de ce passage existe déjà sur ce poste : elle n’est pas écrasée. Le passage attend.",
            "The branch {branch} of this passage already exists on this machine: it isn’t overwritten. The passage waits."
        ));
    }
    Ok(())
}

/// The remote a passage's branch is fetched from: the one the offer names when the repository has it, else its first;
/// never an address the offer gives (SR3-06). None: the repository has no remote.
pub(crate) fn remote_for(remotes: &[String], named: &str) -> Option<String> {
    remotes
        .iter()
        .find(|r| *r == named)
        .or_else(|| remotes.first())
        .cloned()
}

/// The most a deposit weighs (the server keeps files of 25 MiB at most): the oldest items of the conversation are left
/// out past it.
const MAX_DEPOSIT: usize = 20 << 20;

/// What the passage deposits on the server, encrypted by the project's key: the agent's conversation and what its
/// handover file is written from, the patch of its uncommitted work when it goes as one. Never an account.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Deposit {
    pub items: Vec<Value>,
    /// The patch (`git diff --binary`), in base64.
    pub patch: Option<String>,
    pub plan: Option<PlanState>,
    pub progress: Option<String>,
}

/// What an agent passed brings to the agent that goes on with it here (`Core::launch_ticket_agent`).
pub struct HandedIn {
    /// Who passed it, and the harness it ran on there.
    pub by: String,
    pub from: EngineId,
    pub items: Vec<Value>,
    pub patch: Option<Vec<u8>>,
    pub plan: Option<PlanState>,
    pub progress: Option<String>,
}

/// Whom an agent can be passed to: a device of the project.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HandoffTarget {
    pub device: String,
    pub name: String,
    /// A machine in server mode.
    pub machine: bool,
    /// Another device of this machine's member.
    pub mine: bool,
}

/// What the window of the passage shows before the gesture.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HandoffPlan {
    pub ticket_id: String,
    pub key: String,
    pub title: String,
    pub agent_id: Option<String>,
    pub agent_name: String,
    pub engine: Option<EngineId>,
    /// The branch that will be pushed, and the remote it goes to.
    pub branch: String,
    pub remote: String,
    /// The uncommitted changes (`git status --short`), one per line.
    pub dirty: Vec<String>,
    /// Its turn runs: it is interrupted first.
    pub running: bool,
    pub targets: Vec<HandoffTarget>,
    /// Why it cannot be passed; the window shows it and offers nothing.
    pub refusal: Option<String>,
}

/// What becomes of the uncommitted work.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Wip {
    /// A commit « en cours » on the branch, pushed with it.
    Commit,
    /// A patch, in the encrypted deposit.
    Patch,
}

/// The harnesses this machine can take a passage on, the agent's own first when it is one of them.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TakeChoice {
    /// The agent's harness, when this machine has it for the project.
    pub preferred: Option<EngineId>,
    pub engines: Vec<EngineId>,
}

fn personal() -> anyhow::Error {
    anyhow!(tr!(
        "Cet agent est d’un projet personnel : seul l’agent d’un ticket d’un projet de l’équipe se passe à un collègue.",
        "This agent is of a personal project: only the agent of a ticket of a project of the team is passed to a teammate."
    ))
}

fn gone() -> anyhow::Error {
    anyhow!(tr!(
        "Ce passage n’est plus proposé : il a été repris ou abandonné.",
        "This passage is no longer offered: it was taken or abandoned."
    ))
}

fn offline() -> String {
    tr!(
        "Hors ligne : un agent se passe par le serveur de l’équipe. Réessaie une fois en ligne.",
        "Offline: an agent is passed through the team’s server. Try again once online."
    )
}

/// The team's error, said for a passage.
fn said(e: &TeamError) -> String {
    match e {
        TeamError::Client(ClientError::Net(_)) => offline(),
        e => crate::teams::say(e),
    }
}

impl Core {
    /// The harnesses this machine may run an agent of `project` on: written, found, on, with an account serving it.
    pub(crate) fn take_engines(&self, project: &str) -> Vec<EngineId> {
        let settings = self.settings.read().clone();
        engine::registry::all()
            .into_iter()
            .map(|e| e.id())
            .filter(|e| engine::written(*e))
            .filter(|e| crate::accounts::serves_any(&settings, *e, Some(project)))
            .filter(|e| self.engine_found_for(*e, project))
            .filter(|e| self.engine_plugged(*e))
            .filter(|e| self.harness_on_in(*e, project))
            .collect()
    }

    /// What the window of the passage shows for the ticket `id`: its agent, its branch and remote, its uncommitted
    /// changes, the devices of the project it can go to, or why it cannot be passed.
    pub async fn team_handoff_plan(self: &Arc<Self>, id: &str) -> Result<HandoffPlan> {
        let t = self.ticket(id)?;
        let mut plan = HandoffPlan {
            ticket_id: t.id.clone(),
            key: t.key.clone(),
            title: t.title.clone(),
            agent_id: t.agent_id.clone(),
            agent_name: String::new(),
            engine: None,
            branch: String::new(),
            remote: String::new(),
            dirty: Vec::new(),
            running: false,
            targets: Vec::new(),
            refusal: None,
        };
        let project = self.project(&t.project_id)?;
        let refuse = |mut plan: HandoffPlan, e: anyhow::Error| {
            plan.refusal = Some(e.to_string());
            Ok(plan)
        };
        let Some((teams, team_id, pid)) = self.team_project_of(&project) else {
            return refuse(plan, personal());
        };
        if t.team_local || t.team.is_none() {
            return refuse(
                plan,
                anyhow!(tr!(
                    "Ce ticket reste sur ce poste (« Ne pas partager ») : partage-le avec l’équipe pour passer son agent.",
                    "This ticket stays on this machine (“Don’t share”): share it with the team to pass its agent."
                )),
            );
        }
        let tt = t.team.clone().unwrap_or_default();
        if tt.handoff.is_some() {
            return refuse(
                plan,
                anyhow!(tr!(
                    "Un passage de ce ticket est déjà en cours.",
                    "A passage of this ticket is already under way."
                )),
            );
        }
        if tt.token.is_none() || !tt.holder.as_ref().is_some_and(|h| h.mine) {
            return refuse(
                plan,
                anyhow!(tr!(
                    "Ce poste ne tient pas ce ticket : seul son preneur passe son agent.",
                    "This machine does not hold this ticket: only its holder passes its agent."
                )),
            );
        }
        let Some(agent) = t.agent_id.as_deref().and_then(|a| self.agent(a).ok()) else {
            return refuse(
                plan,
                anyhow!(tr!(
                    "Ce ticket n’a pas d’agent sur ce poste.",
                    "This ticket has no agent on this machine."
                )),
            );
        };
        let (wt, cwd, status) = {
            let rt = agent.lock();
            plan.agent_name = rt.meta.name.clone();
            plan.engine = Some(rt.meta.engine);
            (
                rt.meta.worktree.clone(),
                rt.meta.cwd.clone(),
                rt.meta.status,
            )
        };
        plan.running = status.is_active();
        let Some(wt) = wt.filter(|w| w.repos.is_empty()) else {
            return refuse(
                plan,
                anyhow!(tr!(
                    "Seul un agent dans son worktree, sur sa branche, se passe.",
                    "Only an agent in its worktree, on its branch, is passed."
                )),
            );
        };
        plan.branch = wt.branch.clone();
        let remotes = crate::git::remotes(&cwd).await;
        let Some(remote) = crate::git::sync_remote(&remotes, None) else {
            let branch = plan.branch.clone();
            return refuse(
                plan,
                anyhow!(tr!(
                    "La branche {branch} n’a pas d’hébergeur où la pousser : ajoute un dépôt distant (origin) au projet.",
                    "The branch {branch} has no host to be pushed to: add a remote (origin) to the project."
                )),
            );
        };
        plan.remote = remote.to_string();
        plan.dirty = crate::git::text(&cwd, &["status", "--short"])
            .await
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect();
        let shared = teams.team(&team_id).map_err(|e| anyhow!(e))?;
        let mut team = shared.lock().await;
        let me = DeviceId(team.client().device());
        let my_member = team.identity().member;
        let names = team.names(pid).unwrap_or_default();
        let machines = team.machines(pid).unwrap_or_default();
        let kept = team.project_members(&pid).unwrap_or_default();
        let members = team.members().unwrap_or_default();
        drop(team);
        for (m, _, devices) in members {
            if !kept.is_empty() && !kept.contains(&m) {
                continue;
            }
            for d in devices.into_iter().filter(|d| *d != me) {
                let Some(name) = names.get(&d) else { continue };
                plan.targets.push(HandoffTarget {
                    device: hex::encode(d.0),
                    name: name.clone(),
                    machine: machines.contains(&d),
                    mine: m == my_member,
                });
            }
        }
        plan.targets
            .sort_by(|a, b| (a.machine, &a.name).cmp(&(b.machine, &b.name)));
        Ok(plan)
    }

    /// Passes the agent of the ticket `id` to the device `to` (hex), its uncommitted work as `wip` says (the module's
    /// doc). Refused with why, before anything changes, when it cannot; a remote that does not answer leaves
    /// everything as it was.
    pub async fn team_handoff(self: &Arc<Self>, id: &str, to: &str, wip: Wip) -> Result<()> {
        let plan = self.team_handoff_plan(id).await?;
        if let Some(why) = plan.refusal.clone() {
            bail!(why);
        }
        let project = {
            let t = self.ticket(id)?;
            self.project(&t.project_id)?
        };
        let target = plan
            .targets
            .iter()
            .find(|x| x.device == to)
            .cloned()
            .ok_or_else(|| {
                let name = project.name.clone();
                anyhow!(tr!(
                    "Cet appareil n’est pas du projet {name} : un agent ne se passe qu’à un membre du projet ou à une machine de l’équipe qui y a accès.",
                    "This device is not of the project {name}: an agent is passed only to a member of the project or to a machine of the team that has access to it."
                ))
            })?;
        let to_device = object_of(to)
            .map(|o| DeviceId(o.0))
            .ok_or_else(|| anyhow!("a device id"))?;
        let agent_id = plan.agent_id.clone().unwrap_or_default();
        let (teams, team_id, pid) = self.team_project_of(&project).ok_or_else(personal)?;
        // Between two turns: a turn under way is interrupted first.
        if plan.running {
            self.interrupt(&agent_id).await?;
            let end = tokio::time::Instant::now() + Duration::from_secs(20);
            while self
                .agent(&agent_id)
                .is_ok_and(|h| h.lock().meta.status.is_active())
            {
                if tokio::time::Instant::now() >= end {
                    bail!(tr!(
                        "Le tour de l’agent ne s’arrête pas : rien n’est passé.",
                        "The agent’s turn won’t stop: nothing was passed."
                    ));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        let h = self.agent(&agent_id)?;
        let (cwd, engine) = {
            let rt = h.lock();
            (rt.meta.cwd.clone(), rt.meta.engine)
        };
        let branch = plan.branch.clone();
        let key = plan.key.clone();
        // The uncommitted work: committed « en cours », or in a patch (the folder left as it is).
        let mut committed = false;
        let mut patch = None;
        if !plan.dirty.is_empty() {
            crate::git::run(&cwd, &["add", "-A"]).await?;
            match wip {
                Wip::Commit => {
                    let who = target.name.clone();
                    let message = tr!(
                        "wip: travail en cours de l’agent, passé à {who} [{key}]",
                        "wip: the agent’s work in progress, passed to {who} [{key}]"
                    );
                    crate::git::run(&cwd, &["commit", "--no-verify", "-q", "-m", &message]).await?;
                    committed = true;
                }
                Wip::Patch => {
                    let diff =
                        crate::git::run(&cwd, &["diff", "--cached", "--binary", "HEAD"]).await;
                    let _ = crate::git::run(&cwd, &["reset", "-q"]).await;
                    patch = Some(diff?);
                }
            }
        }
        // The branch pushed, by this gesture alone. A host that does not answer: nothing passed, the commit undone.
        let remote = match crate::git::push_branch(&cwd, &branch).await {
            Ok(r) => r,
            Err(e) => {
                if committed {
                    let _ = crate::git::run(&cwd, &["reset", "-q", "--soft", "HEAD~1"]).await;
                    let _ = crate::git::run(&cwd, &["reset", "-q"]).await;
                }
                let e = crate::tickets::error_line(&e);
                bail!(tr!(
                    "L’hébergeur de la branche {branch} ne répond pas : rien n’est passé, l’agent et son travail restent ici. ({e})",
                    "The host of the branch {branch} does not answer: nothing was passed, the agent and its work stay here. ({e})"
                ));
            }
        };
        // The branch freed here (the folder detached at the same commit, its files and its work kept): the machine that
        // takes it, this one too one day, takes it up in a worktree of its own.
        if let Err(e) = crate::git::run(&cwd, &["switch", "-q", "--detach"]).await {
            log::warn!("ticket {key}: the passed agent's folder stays on its branch: {e:#}");
        }
        // The deposit: the conversation, the patch, what the handover file is written from.
        let deposit = {
            let mut rt = h.lock();
            let mut d = Deposit {
                items: rt.conv.items(),
                patch: patch
                    .as_ref()
                    .map(|p| base64::engine::general_purpose::STANDARD.encode(p)),
                plan: rt.meta.plan.clone(),
                progress: rt.meta.progress_line.clone(),
            };
            let mut bytes = serde_json::to_vec(&d)?;
            while bytes.len() > MAX_DEPOSIT && d.items.len() > 1 {
                let half = d.items.len() / 2;
                d.items.drain(..half);
                bytes = serde_json::to_vec(&d)?;
            }
            bytes
        };
        let t = self.ticket(id)?;
        let tt = t.team.clone().unwrap_or_default();
        let claim = object_of(&tt.claim).ok_or_else(|| anyhow!("a claim id"))?;
        let token = tt.token.unwrap_or_default();
        let shared = teams.team(&team_id).map_err(|e| anyhow!(e))?;
        let mut team = shared.lock().await;
        let me = DeviceId(team.client().device());
        let my_name = self.me_in(&project).name;
        let file = team
            .deposit(pid, &deposit)
            .await
            .map_err(|e| anyhow!(said(&e)))?;
        let offer = Offer {
            ticket: tt.object.clone(),
            claim: tt.claim.clone(),
            from: hex::encode(me.0),
            to: to.to_string(),
            file: hex::encode(file.0),
            branch: branch.clone(),
            remote: remote.clone(),
            patch: patch.is_some(),
            engine: engine.to_string(),
            agent: plan.agent_name.clone(),
            at: now_ms(),
        };
        let object = match team.offer_handoff(pid, &offer) {
            Ok(o) => o,
            Err(e) => {
                let _ = team.delete_file(file).await;
                bail!(said(&e));
            }
        };
        let _ = team.sync_project(pid).await;
        let handed = team.hand_claim(pid, claim.0, token, to_device).await;
        let answer = match handed {
            Ok(a) => a,
            Err(e) => {
                // Nothing passed: the offer and the deposit go.
                let _ = team.end_handoff(pid, object);
                let _ = team.delete_file(file).await;
                let _ = team.sync_project(pid).await;
                drop(team);
                let why = match &e {
                    TeamError::Client(ClientError::Claim { code, .. })
                        if code == "not_in_project" =>
                    {
                        let name = target.name.clone();
                        tr!(
                            "{name} n’a pas accès à ce projet sur le serveur de l’équipe : rien n’est passé.",
                            "{name} has no access to this project on the team’s server: nothing was passed."
                        )
                    }
                    e => refusal(&key, e, None),
                };
                bail!(why);
            }
        };
        drop(team);
        let holder = holder_of(&answer.claim, answer.now, &me, Some(target.name.clone()));
        let handoff = TeamHandoff {
            object: hex::encode(object.0),
            from_name: my_name,
            from_device: hex::encode(me.0),
            to_name: target.name.clone(),
            to_device: to.to_string(),
            incoming: false,
            outgoing: true,
            machine: target.machine,
            branch: branch.clone(),
            remote: remote.clone(),
            patch: offer.patch,
            engine: offer.engine.clone(),
            agent: offer.agent.clone(),
            at: offer.at,
            agent_id: Some(agent_id.clone()),
        };
        let t = self.edit_ticket(id, |x| {
            if let Some(tt) = x.team.as_mut() {
                tt.token = None;
                tt.version = answer.claim.version;
                tt.holder = holder.clone();
                tt.handoff = Some(handoff.clone());
            }
            // Under way on the teammate's machine from now on: none of this machine's passes runs it.
            x.started_at = None;
            x.agent_id = None;
            x.branch = branch.clone();
            Ok(x.clone())
        })?;
        let who = target.name.clone();
        let _ = self.with_agent(&agent_id, |rt, fx| {
            rt.notice(
                "info",
                tr!(
                    "Passé à {who} : la branche {branch} est poussée sur {remote}, la suite tourne chez lui. Cet agent reste ici, arrêté ; « Annuler le passage » sur la carte le reprend tant que {who} ne l’a pas pris.",
                    "Passed to {who}: the branch {branch} is pushed on {remote}, the rest runs on their machine. This agent stays here, stopped; “Cancel the passage” on the card takes it back while {who} hasn’t taken it."
                ),
                fx,
            );
            Ok(())
        });
        log::info!("ticket {key}: agent passed to another device of the team");
        self.hub.emit(UiEvent::Ticket { ticket: t.clone() });
        self.request_save();
        self.inbox_view_told(&t.project_id);
        Ok(())
    }

    /// « Annuler le passage »: the ticket comes back to this machine while it was not taken, its agent with it; the
    /// offer and the deposit are erased.
    pub async fn team_handoff_cancel(self: &Arc<Self>, id: &str) -> Result<()> {
        let t = self.ticket(id)?;
        let tt = t.team.clone().unwrap_or_default();
        let hd = tt.handoff.clone().filter(|h| h.outgoing).ok_or_else(gone)?;
        let project = self.project(&t.project_id)?;
        let (teams, team_id, pid) = self.team_project_of(&project).ok_or_else(personal)?;
        let claim = object_of(&tt.claim).ok_or_else(|| anyhow!("a claim id"))?;
        let shared = teams.team(&team_id).map_err(|e| anyhow!(e))?;
        let mut team = shared.lock().await;
        let me = DeviceId(team.client().device());
        let claims = team.claims(pid).await.map_err(|e| anyhow!(said(&e)))?;
        let current = claims
            .claims
            .iter()
            .find(|c| c.ticket == claim.0)
            .cloned()
            .ok_or_else(gone)?;
        let back = team.withdraw_claim(pid, claim.0, current.token).await;
        let answer = match back {
            Ok(a) => a,
            Err(TeamError::Client(ClientError::Claim { .. })) => {
                let who = hd.to_name.clone();
                bail!(tr!(
                    "{who} l’a déjà pris : le passage est fait.",
                    "{who} already took it: the passage is done."
                ));
            }
            Err(e) => bail!(said(&e)),
        };
        let offers = team.handoffs(pid).unwrap_or_default();
        if let Some((object, offer)) = offers
            .into_iter()
            .find(|(o, _)| hex::encode(o.0) == hd.object)
        {
            let _ = team.end_handoff(pid, object);
            if let Some(file) = object_of(&offer.file) {
                let _ = team.delete_file(file).await;
            }
        }
        let _ = team.sync_project(pid).await;
        let name = team.name_of(pid, &me);
        drop(team);
        let holder = holder_of(&answer.claim, answer.now, &me, name);
        let agent = hd.agent_id.clone().filter(|a| self.live_agent(a).is_some());
        let t = self.edit_ticket(id, |x| {
            if let Some(tt) = x.team.as_mut() {
                tt.token = Some(answer.claim.token);
                tt.version = answer.claim.version;
                tt.holder = holder.clone();
                tt.handoff = None;
            }
            x.started_at = Some(now_ms());
            x.agent_id = agent.clone();
            Ok(x.clone())
        })?;
        if let Some(a) = &agent {
            // Its folder back on its branch, which nobody moved.
            if let Some(cwd) = self.live_agent(a).map(|m| m.cwd) {
                if let Err(e) = crate::git::run(&cwd, &["switch", "-q", &hd.branch]).await {
                    log::warn!(
                        "ticket {}: the agent's folder is not back on its branch: {e:#}",
                        t.key
                    );
                }
            }
            let _ = self.with_agent(a, |rt, fx| {
                rt.notice(
                    "info",
                    tr!(
                        "Passage annulé : l’agent et son ticket restent sur ce poste.",
                        "Passage cancelled: the agent and its ticket stay on this machine."
                    ),
                    fx,
                );
                Ok(())
            });
        }
        self.hub.emit(UiEvent::Ticket { ticket: t.clone() });
        self.request_save();
        self.inbox_view_told(&t.project_id);
        Ok(())
    }

    /// The harnesses this machine can take the passage of the ticket `id` on.
    pub fn team_handoff_engines(&self, id: &str) -> Result<TakeChoice> {
        let t = self.ticket(id)?;
        let engines = self.take_engines(&t.project_id);
        let preferred = t
            .team
            .as_ref()
            .and_then(|x| x.handoff.as_ref())
            .map(|h| EngineId::new(&h.engine))
            .filter(|e| engines.contains(e));
        Ok(TakeChoice { preferred, engines })
    }

    /// « Prendre »: the agent passed to this machine goes on here, on `engine` (its own when this machine has it, else
    /// the one the user picks): the branch fetched, a worktree, the patch, then a new session whose first message is
    /// the handover file. The offer and the deposit are erased once taken.
    pub async fn team_handoff_take(
        self: &Arc<Self>,
        id: &str,
        engine: Option<EngineId>,
    ) -> Result<()> {
        if !self.handoffs_taking.lock().insert(id.to_string()) {
            bail!(tr!(
                "Ce passage est déjà en train d’être pris.",
                "This passage is already being taken."
            ));
        }
        let out = self.handoff_take(id, engine, false).await;
        self.handoffs_taking.lock().remove(id);
        out
    }

    /// A machine in server mode takes on its own the passage of the ticket `id`, when the team's rule lets it
    /// ([`MachineTakes`]); it is checked again here against the device the server says handed it.
    async fn team_handoff_take_alone(self: &Arc<Self>, id: &str) -> Result<()> {
        if !self.handoffs_taking.lock().insert(id.to_string()) {
            return Ok(());
        }
        let out = self.handoff_take(id, None, true).await;
        self.handoffs_taking.lock().remove(id);
        out
    }

    /// Whose passages this machine takes on its own, for the team `id` (its file's rule; an administrator's without).
    pub(crate) fn team_machine_takes(&self, team_id: &str) -> MachineTakes {
        self.team_handoff_takes
            .lock()
            .get(team_id)
            .copied()
            .unwrap_or_default()
    }

    /// The rule of the version in force of the team `id` (None: the device left it).
    pub(crate) fn set_team_machine_takes(
        &self,
        team_id: &str,
        effective: Option<&escouade_team::settings::Effective>,
    ) {
        let mut all = self.team_handoff_takes.lock();
        match effective {
            Some(e) => all.insert(team_id.to_owned(), MachineTakes::of(e)),
            None => all.remove(team_id),
        };
    }

    async fn handoff_take(
        self: &Arc<Self>,
        id: &str,
        engine: Option<EngineId>,
        alone: bool,
    ) -> Result<()> {
        let t = self.ticket(id)?;
        let tt = t.team.clone().unwrap_or_default();
        let hd = tt.handoff.clone().filter(|h| h.incoming).ok_or_else(gone)?;
        let project = self.project(&t.project_id)?;
        let (teams, team_id, pid) = self.team_project_of(&project).ok_or_else(personal)?;
        let choice = self.team_handoff_engines(id)?;
        let engine = match engine {
            Some(e) if choice.engines.contains(&e) => e,
            Some(e) => {
                let name = engine::by_name(e);
                bail!(tr!(
                    "{name} ne sert pas ce projet sur ce poste : choisis un autre harness.",
                    "{name} doesn’t serve this project on this machine: pick another harness."
                ));
            }
            None => match choice.preferred.or_else(|| {
                // A machine in server mode takes it on its own: its first harness that serves the project.
                self.team_machine
                    .load(Ordering::Relaxed)
                    .then(|| choice.engines.first().copied())
                    .flatten()
            }) {
                Some(e) => e,
                None => {
                    let name = if hd.engine.is_empty() {
                        "?"
                    } else {
                        engine::by_name(EngineId::new(&hd.engine))
                    };
                    bail!(tr!(
                        "{name} n’est pas sur ce poste pour ce projet : choisis ton harness pour prendre cet agent.",
                        "{name} isn’t on this machine for this project: pick your harness to take this agent."
                    ));
                }
            },
        };
        let shared = teams.team(&team_id).map_err(|e| anyhow!(e))?;
        let (offer, object, deposit) = {
            let mut team = shared.lock().await;
            let _ = team.sync_project(pid).await;
            let me = hex::encode(team.client().device());
            let (object, offer) = team
                .handoffs(pid)
                .unwrap_or_default()
                .into_iter()
                .find(|(o, _)| hex::encode(o.0) == hd.object)
                .ok_or_else(gone)?;
            // Trusted with the claim alone: the server says it was handed to this device by the one the offer names.
            let claims = team.claims(pid).await.map_err(|e| anyhow!(said(&e)))?;
            let handed = claims.claims.iter().any(|c| {
                hex::encode(&c.ticket) == offer.claim
                    && c.holder.as_ref().is_some_and(|h| hex::encode(h) == me)
                    && c.handed_by
                        .as_ref()
                        .is_some_and(|h| hex::encode(h) == offer.from)
            });
            if offer.to != me || !handed {
                bail!(gone());
            }
            // On its own, only what the team's rule lets it take (SR3-05): the device that handed it, as the server
            // says, is the offer's.
            if alone
                && !self
                    .team_machine_takes(&team_id)
                    .takes(admin_devices(&team).contains(&offer.from))
            {
                bail!(tr!(
                    "Ce passage attend qu’un humain de cette machine le prenne.",
                    "This passage waits for a human of this machine to take it."
                ));
            }
            let file = object_of(&offer.file).ok_or_else(gone)?;
            let bytes = team
                .get_file(file)
                .await
                .map_err(|e| anyhow!(said(&e)))?
                .ok_or_else(gone)?;
            let deposit: Deposit = serde_json::from_slice(&bytes)?;
            (offer, object, deposit)
        };
        // The branch, from the host it was pushed to (the offer stays while it is not reached).
        let root = self
            .toplevel(&project.path)
            .await
            .unwrap_or(project.path.clone());
        let remotes = crate::git::remotes(&root).await;
        // A remote of the repository, never an address the offer gives (SR3-06).
        let Some(remote) = remote_for(&remotes, &offer.remote) else {
            bail!(tr!(
                "Ce dépôt n’a aucun remote : la branche de ce passage ne se récupère pas. Le passage attend.",
                "This repository has no remote: the branch of this passage can’t be fetched. The passage waits."
            ));
        };
        passed_branch_ok(&root, &offer.branch, &t.branch).await?;
        let spec = format!("+refs/heads/{0}:refs/heads/{0}", offer.branch);
        let fetched = crate::git::fetch_ref(&root, &remote, &spec).await;
        if let Err(e) = fetched {
            let branch = offer.branch.clone();
            let e = crate::tickets::error_line(&e);
            bail!(tr!(
                "La branche {branch} ne se récupère pas de son hébergeur : le passage attend, rien n’est perdu. ({e})",
                "The branch {branch} can’t be fetched from its host: the passage waits, nothing is lost. ({e})"
            ));
        }
        // Its remote-tracking branch too, followed: the branch says it is published, as on the machine that pushed it.
        let tracking = format!("+refs/heads/{0}:refs/remotes/{1}/{0}", offer.branch, remote);
        if crate::git::fetch_ref(&root, &remote, &tracking)
            .await
            .is_ok()
        {
            let upstream = format!("{remote}/{}", offer.branch);
            let _ = crate::git::run(
                &root,
                &[
                    "branch",
                    "-q",
                    "--set-upstream-to",
                    &upstream,
                    &offer.branch,
                ],
            )
            .await;
        }
        // The claim, this machine's under a new token; the offer and the deposit erased.
        let claim = object_of(&offer.claim).ok_or_else(gone)?;
        let (answer, me, name) = {
            let mut team = shared.lock().await;
            let answer = team
                .take_claim(pid, claim.0, None)
                .await
                .map_err(|e| anyhow!(refusal(&t.key, &e, None)))?;
            let _ = team.end_handoff(pid, object);
            if let Some(file) = object_of(&offer.file) {
                let _ = team.delete_file(file).await;
            }
            let _ = team.sync_project(pid).await;
            let me = DeviceId(team.client().device());
            let name = team.name_of(pid, &me);
            (answer, me, name)
        };
        let holder = holder_of(&answer.claim, answer.now, &me, name);
        let t = self.edit_ticket(id, |x| {
            if let Some(tt) = x.team.as_mut() {
                tt.token = Some(answer.claim.token);
                tt.version = answer.claim.version;
                tt.holder = holder.clone();
                tt.handoff = None;
            }
            x.column = Column::Doing;
            x.started_at = Some(now_ms());
            x.agent_id = None;
            x.blocked = None;
            x.branch = offer.branch.clone();
            x.engine = Some(engine);
            Ok(x.clone())
        })?;
        self.hub.emit(UiEvent::Ticket { ticket: t.clone() });
        self.request_save();
        self.inbox_view_told(&t.project_id);
        log::info!("ticket {}: agent taken from a teammate, on {engine}", t.key);
        let patch = match deposit.patch.as_deref() {
            Some(p) => Some(base64::engine::general_purpose::STANDARD.decode(p)?),
            None => None,
        };
        let handed = HandedIn {
            by: hd.from_name.clone(),
            from: EngineId::new(&offer.engine),
            items: deposit.items,
            patch,
            plan: deposit.plan,
            progress: deposit.progress,
        };
        let started_at = t.started_at;
        let launched = self.launch_ticket_agent(&project, &t, Some(handed)).await;
        if let Err(e) = &launched {
            log::warn!("ticket {}: the agent passed did not start: {e:#}", t.key);
            let _ = self.edit_ticket(id, |x| {
                if x.column == Column::Doing && x.started_at == started_at {
                    x.blocked = Some(crate::tickets::error_reason(e));
                }
                Ok(())
            });
            self.notify_ticket(id, false);
        }
        self.schedule();
        launched
    }

    /// The agent passed goes on here (`launch_ticket_agent`, its worktree made): the patch applied, the conversation of
    /// before above, then the handover file as its first message.
    pub(crate) async fn go_on_handed(
        self: &Arc<Self>,
        agent_id: &str,
        t: &crate::model::Ticket,
        handed: HandedIn,
    ) -> Result<()> {
        let h = self.agent(agent_id)?;
        let (cwd, base, engine, model) = {
            let rt = h.lock();
            (
                rt.meta.cwd.clone(),
                rt.meta.worktree.as_ref().map(|w| w.base_branch.clone()),
                rt.meta.engine,
                rt.meta.model.clone(),
            )
        };
        let mut patch_failed = None;
        if let Some(patch) = &handed.patch {
            let path = std::env::temp_dir().join(format!("escouade-handoff-{}.patch", new_id()));
            std::fs::write(&path, patch)?;
            let p = path.to_string_lossy().to_string();
            match crate::git::run(&cwd, &["apply", "--binary", "--whitespace=nowarn", &p]).await {
                Ok(_) => {
                    let _ = std::fs::remove_file(&path);
                }
                Err(e) => patch_failed = Some((p, crate::tickets::error_line(&e))),
            }
        }
        let by = handed.by.clone();
        let from = handed.from;
        self.with_agent(agent_id, |rt, fx| {
            for item in &handed.items {
                rt.append(item.clone(), fx);
            }
            rt.append(handover::passed_separator(from, engine, &by), fx);
            if let Some((path, e)) = &patch_failed {
                rt.notice(
                    "warn",
                    tr!(
                        "Le travail non commité de {by} ne s’applique pas tel quel ({e}) : son patch est gardé dans {path}.",
                        "{by}’s uncommitted work doesn’t apply as it is ({e}): its patch is kept in {path}."
                    ),
                    fx,
                );
            }
            fx.save = true;
            Ok(())
        })?;
        let changes = self.changes_of(&cwd, base.as_deref()).await;
        let ticket = format!("{} — {}", t.key, t.title);
        let to = engine::by_id(engine);
        let text = handover::passed(
            &Dossier {
                lang: self.lang().claude,
                from: engine::by_name(from),
                limited: false,
                items: &handed.items,
                plan: handed.plan.as_ref(),
                progress: handed.progress.as_deref(),
                changes: &changes,
                ticket: Some(&ticket),
                budget: handover::budget(to.context_window(&model)),
            },
            &by,
        );
        self.send_handover(agent_id, text).await
    }

    /// The passages of the project's journal, put on its tickets (« Alice te passe DEM-41 · Prendre », « Passé à Bob
    /// · Annuler »); a machine in server mode takes at once those offered to it.
    pub(crate) async fn read_handoffs(self: &Arc<Self>, project_id: &str) {
        let Ok(project) = self.project(project_id) else {
            return;
        };
        let Some((teams, team_id, pid)) = self.team_project_of(&project) else {
            return;
        };
        let Ok(shared) = teams.team(&team_id) else {
            return;
        };
        let (offers, me, names, machines, admins) = {
            let mut team = shared.lock().await;
            let offers = team.handoffs(pid).unwrap_or_default();
            let me = hex::encode(team.client().device());
            let names = team.names(pid).unwrap_or_default();
            let machines = team.machines(pid).unwrap_or_default();
            (offers, me, names, machines, admin_devices(&team))
        };
        let takes = self.team_machine_takes(&team_id);
        let name_of = |hex_id: &str| {
            object_of(hex_id)
                .and_then(|o| names.get(&DeviceId(o.0)).cloned())
                .unwrap_or_default()
        };
        let mut changed = Vec::new();
        let mut take = Vec::new();
        {
            let mut tickets = self.tickets.write();
            for t in tickets.iter_mut().filter(|t| t.project_id == project_id) {
                let Some(tt) = t.team.as_mut() else { continue };
                let found = offers
                    .iter()
                    .find(|(_, o)| !tt.object.is_empty() && o.ticket == tt.object);
                let next = found.map(|(object, o)| TeamHandoff {
                    object: hex::encode(object.0),
                    from_name: name_of(&o.from),
                    from_device: o.from.clone(),
                    to_name: name_of(&o.to),
                    to_device: o.to.clone(),
                    incoming: o.to == me,
                    outgoing: o.from == me,
                    machine: object_of(&o.to).is_some_and(|d| machines.contains(&DeviceId(d.0))),
                    branch: o.branch.clone(),
                    remote: o.remote.clone(),
                    patch: o.patch,
                    engine: o.engine.clone(),
                    agent: o.agent.clone(),
                    at: o.at,
                    agent_id: tt.handoff.as_ref().and_then(|h| h.agent_id.clone()),
                });
                // On its own, only what the team's rule lets it take (SR3-05); the rest waits in « À toi ».
                if next
                    .as_ref()
                    .is_some_and(|h| h.incoming && takes.takes(admins.contains(&h.from_device)))
                    && self.team_machine.load(Ordering::Relaxed)
                {
                    take.push(t.id.clone());
                }
                if tt.handoff != next {
                    tt.handoff = next;
                    changed.push(t.clone());
                }
            }
        }
        for t in &changed {
            self.hub.emit(UiEvent::Ticket { ticket: t.clone() });
        }
        if !changed.is_empty() {
            self.request_save();
            self.inbox_view_told(project_id);
        }
        for id in take {
            if self.handoffs_taking.lock().contains(&id) {
                continue;
            }
            let me = self.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = me.team_handoff_take_alone(&id).await {
                    log::warn!("ticket {id}: the agent passed to this machine is not taken: {e:#}");
                }
            });
        }
    }

    /// Makes this core a machine in server mode, or not, for the tests (the server's core is one from the start).
    #[cfg(test)]
    pub(crate) fn set_team_machine(&self, on: bool) {
        self.team_machine.store(on, Ordering::Relaxed);
    }
}

#[cfg(test)]
#[path = "teamhandoff_tests.rs"]
mod tests;
