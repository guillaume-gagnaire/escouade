//! The tools of the Overseer alone (ESC-462): its instructions (`add_directive`, `list_directives`,
//! `revoke_directive`), the undo of what it did (`undo_action`), and a request left to the user (`leave_to_user`).
//! Refused to anyone else. What they check is `overseer`'s and `overseer_core`'s: an instruction is born of a message of
//! the user only, by the user's own words.

use super::read::json;
use super::resolve;
use super::tools::{
    AddDirectiveArgs, LeaveToUserArgs, RevokeDirectiveArgs, ToolError, UndoActionArgs,
};
use super::Caller;
use crate::core::Core;
use crate::model::now_ms;
use crate::overseer::{Outgoing, Place, Scope};
use serde_json::json;
use std::sync::Arc;

/// The Overseer calls, else the refusal.
fn overseer_only(core: &Core, caller: &Caller) -> Result<(), ToolError> {
    if core.overseer_calls(caller) {
        return Ok(());
    }
    Err(ToolError::Refused(tr!(
        "Cet outil est réservé à l’Overseer d’Escouade.",
        "This tool is reserved to Escouade’s Overseer."
    )))
}

/// The actions named, as the tools take them.
pub(super) fn actions_of(asked: &[String]) -> Result<Vec<Outgoing>, ToolError> {
    asked
        .iter()
        .map(|a| {
            serde_json::from_value::<Outgoing>(json!(a.trim().to_lowercase())).map_err(|_| {
                ToolError::Failed(tr!(
                    "Action inconnue « {a} » : merge, push, tag, publish ou approve.",
                    "Unknown action “{a}”: merge, push, tag, publish or approve."
                ))
            })
        })
        .collect()
}

/// An end given as RFC 3339, or as a day (its end, in the machine's time).
fn until_of(asked: Option<&str>) -> Result<Option<i64>, ToolError> {
    let Some(text) = asked.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if let Ok(at) = chrono::DateTime::parse_from_rfc3339(text) {
        return Ok(Some(at.timestamp_millis()));
    }
    if let Ok(day) = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        use chrono::TimeZone;
        let end = day.and_hms_opt(23, 59, 59).expect("a time of day");
        if let Some(at) = chrono::Local.from_local_datetime(&end).earliest() {
            return Ok(Some(at.timestamp_millis()));
        }
    }
    Err(ToolError::Failed(tr!(
        "Fin illisible « {text} » : une date et une heure RFC 3339, ou une date AAAA-MM-JJ.",
        "Unreadable end “{text}”: a date and time in RFC 3339, or a date YYYY-MM-DD."
    )))
}

/// `add_directive`.
pub(super) fn add(core: &Core, caller: &Caller, a: AddDirectiveArgs) -> Result<String, ToolError> {
    overseer_only(core, caller)?;
    let actions = actions_of(&a.actions)?;
    let project = a
        .project
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .map(|p| resolve::project(core, caller, p))
        .transpose()?;
    let ticket = a
        .ticket
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .map(|t| resolve::ticket(core, caller, t))
        .transpose()?;
    let scope = Scope {
        project_id: project.as_ref().map(|p| p.id.clone()),
        ticket_id: ticket.as_ref().map(|t| t.id.clone()),
        release: a
            .release
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty()),
        until: until_of(a.until.as_deref())?,
        once: a.once.unwrap_or(false),
    };
    let d = core.overseer_add_directive(&a.message, &a.quote, &a.summary, actions, scope)?;
    json(&json!({
        "id": d.id,
        "summary": d.summary,
        "actions": d.actions,
        "project": project.map(|p| p.name),
        "ticket": ticket.map(|t| t.key),
        "release": d.scope.release,
        "until": d.scope.until,
        "once": d.scope.once,
        "source": d.source,
    }))
}

/// `list_directives`: the active instructions, and the latest actions with what can still be undone.
pub(super) fn list(core: &Core, caller: &Caller) -> Result<String, ToolError> {
    overseer_only(core, caller)?;
    let view = core.overseer_view();
    let actions: Vec<_> = view
        .actions
        .iter()
        .rev()
        .take(20)
        .map(|a| {
            json!({
                "id": a.action.id, "tool": a.action.tool, "text": a.action.text,
                "outcome": a.action.outcome, "undoable": a.undoable,
            })
        })
        .collect();
    json(&json!({ "directives": view.directives, "actions": actions }))
}

/// `revoke_directive`: one by its id, or every one that allows the actions given.
pub(super) fn revoke(
    core: &Core,
    caller: &Caller,
    a: RevokeDirectiveArgs,
) -> Result<String, ToolError> {
    overseer_only(core, caller)?;
    let ids: Vec<String> = match (
        a.directive
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty()),
        a.actions,
    ) {
        (Some(id), _) => vec![id.to_string()],
        (None, Some(actions)) => {
            let actions = actions_of(&actions)?;
            let now = now_ms();
            core.overseer
                .state
                .lock()
                .active(now)
                .iter()
                .filter(|d| actions.iter().any(|x| d.names(*x)))
                .map(|d| d.id.clone())
                .collect()
        }
        (None, None) => {
            return Err(ToolError::Failed(tr!(
                "Donne la consigne (directive) ou les actions à ne plus permettre.",
                "Give the instruction (directive) or the actions no longer to allow."
            )))
        }
    };
    let mut withdrawn = Vec::new();
    for id in ids {
        let d = core
            .overseer_revoke(&id)
            .map_err(|e| ToolError::Failed(format!("{e:#}")))?;
        withdrawn.push(json!({ "id": d.id, "summary": d.summary }));
    }
    json(&json!({ "withdrawn": withdrawn }))
}

/// `undo_action`.
pub(super) async fn undo(
    core: &Arc<Core>,
    caller: &Caller,
    a: UndoActionArgs,
) -> Result<String, ToolError> {
    overseer_only(core, caller)?;
    core.overseer_undo(a.action.trim())
        .await
        .map_err(|e| ToolError::Refused(format!("{e:#}")))?;
    json(&json!({ "action": a.action.trim(), "undone": true }))
}

/// `leave_to_user`.
pub(super) fn leave(
    core: &Arc<Core>,
    caller: &Caller,
    a: LeaveToUserArgs,
) -> Result<String, ToolError> {
    overseer_only(core, caller)?;
    let scope = a
        .project
        .as_deref()
        .map(|p| resolve::project(core, caller, p))
        .transpose()?;
    let found = resolve::agent(core, caller, &a.agent, scope.as_ref())?;
    super::act::waiting_request(core, &found, &a.request)?;
    core.overseer_leave(
        &found.id,
        &found.name,
        &found.project_id,
        a.request.trim(),
        a.why.trim(),
    );
    json(&json!({ "agent": found.name, "request": a.request.trim(), "left": true }))
}

/// Where the agent `id` works: its project and its ticket.
pub(super) fn place_of_agent(core: &Core, id: &str) -> Place {
    let Ok(h) = core.agent(id) else {
        return Place::default();
    };
    let m = &h.lock().meta;
    Place {
        project_id: Some(m.project_id.clone()),
        ticket_id: m.ticket_id.clone(),
        release: None,
    }
}

/// The guard of a text the Overseer sends an agent (a message, an answer, a first message): the commands that go out
/// in it (`git push`, `gh pr merge`…) pass only under an instruction that covers them where the agent works.
pub(super) fn gate_text(
    core: &Core,
    caller: &Caller,
    agent_id: &str,
    project_id: &str,
    text: &str,
) -> Result<(), ToolError> {
    if !core.overseer_calls(caller) {
        return Ok(());
    }
    let mut place = if agent_id.is_empty() {
        Place::default()
    } else {
        place_of_agent(core, agent_id)
    };
    if place.project_id.is_none() {
        place.project_id = Some(project_id.to_string());
    }
    for (action, release) in crate::overseer::outgoing_in(text) {
        let at = Place {
            release,
            ..place.clone()
        };
        core.overseer_gate(caller, &[action], &at)?;
    }
    Ok(())
}
