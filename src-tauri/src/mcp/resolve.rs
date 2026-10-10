//! What a tool's arguments name: a project by its id or its name (any case), an agent by its id
//! or its name (in a project when one is given), a ticket by its id or its key (any case). A name
//! that fits none, or several, is an error the model reads, with the choices it has (`CHOICES`
//! at most): it asks again with one of them.

use super::tools::ToolError;
use crate::core::Core;
use crate::model::{Project, Ticket};
use std::collections::HashMap;
use tauri::Runtime;

/// The most choices an error lists.
pub(crate) const CHOICES: usize = 5;

/// The longest title a ticket's choice shows, in characters.
const TITLE_MAX: usize = 60;

/// The project `asked` names: its id, else its name in any case.
pub(crate) fn project<R: Runtime>(core: &Core<R>, asked: &str) -> Result<Project, ToolError> {
    let asked = asked.trim();
    if asked.is_empty() {
        return Err(ToolError::Failed(tr!(
            "Donne un projet : son id ou son nom.",
            "Give a project: its id or its name."
        )));
    }
    let projects = core.projects.read().clone();
    let label = |p: &Project| format!("{} (id {})", p.name, p.id);
    match named(&projects, asked, |p| &p.id, |p| &p.name) {
        Named::One(p) => Ok(p.clone()),
        Named::Several(found) => Err(ToolError::Failed(tr!(
            "Plusieurs projets s’appellent « {asked} » : {choices} Donne son id.",
            "Several projects are named “{asked}”: {choices} Give its id.",
            choices = listed(&found, asked, |p| &p.name, label)
        ))),
        Named::None if projects.is_empty() => Err(ToolError::Failed(tr!(
            "Aucun projet « {asked} » : Escouade n’en a aucun.",
            "No project “{asked}”: Escouade has none."
        ))),
        Named::None => Err(ToolError::Failed(tr!(
            "Aucun projet « {asked} ». Projets : {choices}",
            "No project “{asked}”. Projects: {choices}",
            choices = listed(
                &projects.iter().collect::<Vec<_>>(),
                asked,
                |p| &p.name,
                label
            )
        ))),
    }
}

/// An agent, as resolving a name finds it.
#[derive(Debug, Clone)]
pub(crate) struct AgentRef {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub archived: bool,
    pub created_at: i64,
}

/// The agent `asked` names, in `project` when one is given: its id (archived ones too), else its
/// name in any case among the agents not archived, else among the archived ones.
pub(crate) fn agent<R: Runtime>(
    core: &Core<R>,
    asked: &str,
    project: Option<&Project>,
) -> Result<AgentRef, ToolError> {
    let asked = asked.trim();
    if asked.is_empty() {
        return Err(ToolError::Failed(tr!(
            "Donne un agent : son id ou son nom.",
            "Give an agent: its id or its name."
        )));
    }
    let mut agents: Vec<AgentRef> = core
        .agents
        .read()
        .values()
        .map(|h| {
            let rt = h.lock();
            let m = &rt.meta;
            AgentRef {
                id: m.id.clone(),
                name: m.name.clone(),
                project_id: m.project_id.clone(),
                archived: m.archived,
                created_at: m.created_at,
            }
        })
        .filter(|a| project.is_none_or(|p| a.project_id == p.id))
        .collect();
    agents.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
    if let Some(a) = agents.iter().find(|a| a.id == asked) {
        return Ok(a.clone());
    }
    let projects = project_names(core);
    let label = |a: &AgentRef| {
        let project = projects.get(&a.project_id).cloned().unwrap_or_default();
        tr!(
            "{name} (projet {project}, id {id})",
            "{name} (project {project}, id {id})",
            name = a.name,
            id = a.id
        )
    };
    let (live, archived): (Vec<AgentRef>, Vec<AgentRef>) =
        agents.into_iter().partition(|a| !a.archived);
    let found = match named(&live, asked, |a| &a.id, |a| &a.name) {
        Named::None => named(&archived, asked, |a| &a.id, |a| &a.name),
        found => found,
    };
    match found {
        Named::One(a) => Ok(a.clone()),
        Named::Several(found) => Err(ToolError::Failed(tr!(
            "Plusieurs agents s’appellent « {asked} » : {choices} Donne son id ou son projet.",
            "Several agents are named “{asked}”: {choices} Give its id or its project.",
            choices = listed(&found, asked, |a| &a.name, label)
        ))),
        Named::None => {
            let choices = |list: &[AgentRef]| {
                listed(&list.iter().collect::<Vec<_>>(), asked, |a| &a.name, label)
            };
            Err(ToolError::Failed(
                match (project, live.is_empty(), archived.is_empty()) {
                    (Some(p), true, true) => tr!(
                        "Aucun agent « {asked} » dans le projet {project} : il n’en a aucun.",
                        "No agent “{asked}” in the project {project}: it has none.",
                        project = p.name
                    ),
                    // Only archived ones, none of that name: they are the choices.
                    (Some(p), true, false) => tr!(
                        "Aucun agent « {asked} » dans le projet {project} : ses agents sont tous archivés. Agents archivés : {choices}",
                        "No agent “{asked}” in the project {project}: its agents are all archived. Archived agents: {choices}",
                        project = p.name,
                        choices = choices(&archived)
                    ),
                    (Some(p), false, _) => tr!(
                        "Aucun agent « {asked} » dans le projet {project}. Agents : {choices}",
                        "No agent “{asked}” in the project {project}. Agents: {choices}",
                        project = p.name,
                        choices = choices(&live)
                    ),
                    (None, true, true) => tr!(
                        "Aucun agent « {asked} » : Escouade n’en a aucun.",
                        "No agent “{asked}”: Escouade has none."
                    ),
                    (None, true, false) => tr!(
                        "Aucun agent « {asked} » : ceux d’Escouade sont tous archivés. Agents archivés : {choices}",
                        "No agent “{asked}”: Escouade’s agents are all archived. Archived agents: {choices}",
                        choices = choices(&archived)
                    ),
                    (None, false, _) => tr!(
                        "Aucun agent « {asked} ». Agents : {choices}",
                        "No agent “{asked}”. Agents: {choices}",
                        choices = choices(&live)
                    ),
                },
            ))
        }
    }
}

/// The ticket `asked` names: its id, else its key in any case (a key may be in two projects whose
/// keys have the same prefix).
pub(crate) fn ticket<R: Runtime>(core: &Core<R>, asked: &str) -> Result<Ticket, ToolError> {
    ticket_in(core, asked, None)
}

/// `ticket`, among the tickets of `project` when one is given: a ticket comes after, or goes
/// before, one of its own project.
pub(crate) fn ticket_in<R: Runtime>(
    core: &Core<R>,
    asked: &str,
    project: Option<&Project>,
) -> Result<Ticket, ToolError> {
    let asked = asked.trim();
    if asked.is_empty() {
        return Err(ToolError::Failed(tr!(
            "Donne un ticket : sa clé ou son id.",
            "Give a ticket: its key or its id."
        )));
    }
    let mut tickets = core.tickets.read().clone();
    tickets.retain(|t| project.is_none_or(|p| t.project_id == p.id));
    // The newest first among the choices.
    tickets.sort_by(|a, b| (b.created_at, &b.id).cmp(&(a.created_at, &a.id)));
    let projects = project_names(core);
    let label = |t: &Ticket| {
        let project = projects.get(&t.project_id).cloned().unwrap_or_default();
        let title = short(&t.title);
        tr!(
            "{key} « {title} » (projet {project}, id {id})",
            "{key} “{title}” (project {project}, id {id})",
            key = t.key,
            id = t.id
        )
    };
    if let Some(t) = tickets.iter().find(|t| t.id == asked) {
        return Ok(t.clone());
    }
    // Those of that key, the oldest first.
    let wanted = asked.to_lowercase();
    let mut by_key: Vec<&Ticket> = tickets
        .iter()
        .filter(|t| t.key.to_lowercase() == wanted)
        .collect();
    by_key.reverse();
    match by_key.len() {
        1 => Ok(by_key[0].clone()),
        0 => {
            let choices = listed(
                &tickets.iter().collect::<Vec<_>>(),
                asked,
                |t| &t.key,
                label,
            );
            Err(ToolError::Failed(match project {
                Some(p) if tickets.is_empty() => tr!(
                    "Aucun ticket « {asked} » dans le projet {project} : il n’en a aucun.",
                    "No ticket “{asked}” in the project {project}: it has none.",
                    project = p.name
                ),
                Some(p) => tr!(
                    "Aucun ticket « {asked} » dans le projet {project}. Tickets : {choices}",
                    "No ticket “{asked}” in the project {project}. Tickets: {choices}",
                    project = p.name
                ),
                None if tickets.is_empty() => tr!(
                    "Aucun ticket « {asked} » : Escouade n’en a aucun.",
                    "No ticket “{asked}”: Escouade has none."
                ),
                None => tr!(
                    "Aucun ticket « {asked} ». Tickets : {choices}",
                    "No ticket “{asked}”. Tickets: {choices}"
                ),
            }))
        }
        _ => Err(ToolError::Failed(tr!(
            "Plusieurs tickets ont la clé « {asked} » : {choices} Donne son id.",
            "Several tickets have the key “{asked}”: {choices} Give its id.",
            choices = listed(&by_key, asked, |t| &t.key, label)
        ))),
    }
}

/// Each project's name, by id.
fn project_names<R: Runtime>(core: &Core<R>) -> HashMap<String, String> {
    core.projects
        .read()
        .iter()
        .map(|p| (p.id.clone(), p.name.clone()))
        .collect()
}

/// What a name or an id names among some items.
enum Named<'a, T> {
    One(&'a T),
    Several(Vec<&'a T>),
    None,
}

/// The item whose id is `asked`, else those whose name is `asked` in any case.
fn named<'a, T>(
    items: &'a [T],
    asked: &str,
    id: impl Fn(&T) -> &str,
    name: impl Fn(&T) -> &str,
) -> Named<'a, T> {
    if let Some(t) = items.iter().find(|t| id(t) == asked) {
        return Named::One(t);
    }
    let wanted = asked.to_lowercase();
    let found: Vec<&T> = items
        .iter()
        .filter(|t| name(t).to_lowercase() == wanted)
        .collect();
    match found.len() {
        0 => Named::None,
        1 => Named::One(found[0]),
        _ => Named::Several(found),
    }
}

/// `CHOICES` of `items` at most, as `label` says them, those whose name holds `asked` (in any
/// case) first: « a, b, c. », or « a, b, c, d, e, … » when there are more.
fn listed<T>(
    items: &[&T],
    asked: &str,
    name: impl Fn(&T) -> &str,
    label: impl Fn(&T) -> String,
) -> String {
    let wanted = asked.to_lowercase();
    let (like, rest): (Vec<&T>, Vec<&T>) = items
        .iter()
        .copied()
        .partition(|t| name(t).to_lowercase().contains(&wanted));
    let shown: Vec<String> = like
        .into_iter()
        .chain(rest)
        .take(CHOICES)
        .map(&label)
        .collect();
    if items.len() > CHOICES {
        format!("{}, …", shown.join(", "))
    } else {
        format!("{}.", shown.join(", "))
    }
}

/// A title as a choice shows it: on one line, `TITLE_MAX` characters at most.
fn short(title: &str) -> String {
    let line: String = title
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if line.chars().count() <= TITLE_MAX {
        return line;
    }
    let mut cut: String = line.chars().take(TITLE_MAX - 1).collect();
    cut.push('…');
    cut
}
