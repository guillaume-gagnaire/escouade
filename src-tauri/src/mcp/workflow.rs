//! What the tools say of the workflows of a project (ESC-198, choices 156 and 157 of
//! `UX-NOTES.md`): `get_workflow` (its steps, doors and ways back, and the agents on each step),
//! and the `workflow` of an agent in `list_agents` and `get_agent_summary` (the latter with the
//! history of its steps). Only what the caller sees (`Sight`): the project is named through
//! `resolve::project`, the agents listed are the ones the other tools list, so a private project
//! says nothing of its workflows to the agents of the others.

use super::read::json;
use super::resolve;
use super::tools::ToolError;
use super::Caller;
use crate::core::Core;
use crate::workflow::Scan;
use crate::workflow_track::{AgentTrack, Passage};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// The latest passages of an agent's history `get_agent_summary` gives.
pub(crate) const HISTORY_MAX: usize = 100;

/// Where an agent is in the workflow of its project, as `list_agents` says it.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentWorkflow {
    /// The workflow's name.
    name: String,
    /// Its step now; null between two steps, or while it does something else (`outside`).
    step: Option<String>,
    /// What it does outside the workflow, until the next door.
    #[serde(skip_serializing_if = "Option::is_none")]
    outside: Option<String>,
    /// The turn of a prompt run over and over, or the loop of its ticket; null otherwise.
    turn: Option<u32>,
    /// When it entered the workflow, in epoch milliseconds.
    since: i64,
}

/// `AgentWorkflow` with the steps it went through, as `get_agent_summary` gives them.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentWorkflowHistory {
    #[serde(flatten)]
    now: AgentWorkflow,
    /// Its time and cost in the workflow, in milliseconds and US dollars.
    ms: u64,
    cost: f64,
    /// The latest `HISTORY_MAX` steps, oldest first: the step (null outside), the door typed, when,
    /// how long, what it cost, what it did, how it ended and why it went back.
    history: Vec<Passage>,
}

/// The reading of a project's workflows and the tracks of its agents, as last kept.
pub(crate) struct Tracked {
    scan: std::sync::Arc<Scan>,
    tracks: HashMap<String, AgentTrack>,
}

impl Tracked {
    pub(crate) fn of(core: &Core, project_id: &str) -> Self {
        Self {
            scan: core.tracking_scan(project_id),
            tracks: core
                .workflow_tracks(project_id)
                .into_iter()
                .map(|v| (v.track.agent_id.clone(), v.track))
                .collect(),
        }
    }

    /// The agent's track, when it follows a workflow.
    fn track(&self, agent_id: &str) -> Option<&AgentTrack> {
        self.tracks.get(agent_id).filter(|t| t.workflow.is_some())
    }

    fn name_of(&self, id: &str) -> String {
        self.scan
            .workflows
            .iter()
            .find(|w| w.id == id)
            .map_or_else(|| id.to_string(), |w| w.name.clone())
    }

    fn now(&self, t: &AgentTrack) -> AgentWorkflow {
        let id = t.workflow.clone().unwrap_or_default();
        AgentWorkflow {
            name: self.name_of(&id),
            step: t.step.clone(),
            outside: t.outside.clone(),
            turn: t
                .turns
                .last()
                .map(|l| l.n)
                .or(t.ticket_loop.map(|(n, _)| n)),
            since: t.since,
        }
    }

    /// Where the agent is, null when it follows no workflow.
    pub(crate) fn of_agent(&self, agent_id: &str) -> Option<AgentWorkflow> {
        self.track(agent_id).map(|t| self.now(t))
    }

    /// Where the agent is and where it has been.
    pub(crate) fn history_of(&self, agent_id: &str) -> Option<AgentWorkflowHistory> {
        self.track(agent_id).map(|t| {
            let from = t.passages.len().saturating_sub(HISTORY_MAX);
            AgentWorkflowHistory {
                now: self.now(t),
                ms: t.ms,
                cost: (t.cost * 10_000.0).round() / 10_000.0,
                history: t.passages[from..].to_vec(),
            }
        })
    }
}

/// `get_workflow`: the workflows of the project `project` names, read again if a file they read
/// changed, with the agents on each step.
pub(super) async fn get(core: &Core, caller: &Caller, project: &str) -> Result<String, ToolError> {
    let p = resolve::project(core, caller, project)?;
    let read = core
        .refresh_workflows(&p.id, false)
        .await
        .map_err(|e| ToolError::Failed(e.to_string()))?;
    // As the user corrected them (ESC-195), the way the agents are followed in them.
    let scan = crate::workflowfix::corrected(&read, &p.workflow_fixes);
    let tracked = Tracked::of(core, &p.id);
    // The agents not archived, by id.
    let names: HashMap<String, String> = core
        .agent_views()
        .into_iter()
        .filter(|v| !v.meta.archived && v.meta.project_id == p.id)
        .map(|v| (v.meta.id, v.meta.name))
        .collect();
    let mut workflows = Vec::with_capacity(scan.workflows.len());
    for wf in &scan.workflows {
        let mut v = serde_json::to_value(wf).map_err(|e| ToolError::Failed(e.to_string()))?;
        let here = |step: &str| -> Vec<Value> {
            let mut on: Vec<(&String, &AgentTrack)> = tracked
                .tracks
                .iter()
                .filter(|(id, t)| {
                    names.contains_key(*id)
                        && t.workflow.as_deref() == Some(wf.id.as_str())
                        && t.step.as_deref() == Some(step)
                })
                .collect();
            on.sort_by_key(|(id, t)| (t.since, (*id).clone()));
            on.into_iter()
                .map(|(id, t)| {
                    let pass = t.passages.last().map_or(0, |p| p.pass);
                    json!({ "id": id, "name": names[id], "since": t.since, "pass": pass })
                })
                .collect()
        };
        if let Some(steps) = v["steps"].as_array_mut() {
            for s in steps {
                let name = s["name"].as_str().unwrap_or_default().to_string();
                s["agentsHere"] = Value::Array(here(&name));
            }
        }
        workflows.push(v);
    }
    // The agents of the project that follow a workflow but are on no step: between two, or outside.
    let mut off: Vec<(&String, &AgentTrack)> = tracked
        .tracks
        .iter()
        .filter(|(id, t)| names.contains_key(*id) && t.workflow.is_some() && t.step.is_none())
        .collect();
    off.sort_by_key(|(id, _)| (*id).clone());
    let elsewhere: Vec<Value> = off
        .into_iter()
        .map(|(id, t)| {
            json!({
                "id": id,
                "name": names[id],
                "workflow": tracked.name_of(t.workflow.as_deref().unwrap_or_default()),
                "outside": t.outside,
                "since": t.since,
            })
        })
        .collect();
    json(&json!({
        "project": p.name,
        "projectId": p.id,
        "readAt": scan.read_at,
        "workflows": workflows,
        "alone": scan.alone,
        "agentsElsewhere": elsewhere,
    }))
}
