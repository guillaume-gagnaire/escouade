//! Agent folders: the pure rules of the sidebar's order. The core reads the agents, builds the
//! `Member`s and calls these under the `projects` lock alone.

// The core, the MCP tools and the window use it from the next tasks; drop this then.
#![allow(dead_code)]

use crate::model::{AgentFolder, Project};
use anyhow::bail;
use std::collections::HashSet;

/// An agent of the project as the sidebar sees it; `waiting` is computed by the caller.
pub struct Member<'a> {
    pub id: &'a str,
    pub created_at: i64,
    pub waiting: bool,
    pub archived: bool,
}

/// What the sidebar lists, top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    Agent(String),
    /// Its shown agents, the principal first.
    Folder {
        id: String,
        agents: Vec<String>,
    },
}

/// Drops the unknown ids and the doubles, then lists at the bottom, oldest first, every member
/// placed nowhere. Idempotent; says whether it changed anything.
pub fn settle(p: &mut Project, members: &[Member]) -> bool {
    let before = (p.agent_folders.clone(), p.agent_order.clone());
    let live: HashSet<&str> = members.iter().map(|m| m.id).collect();
    let mut seen: HashSet<String> = HashSet::new();
    for f in &mut p.agent_folders {
        f.agents
            .retain(|a| live.contains(a.as_str()) && seen.insert(a.clone()));
        if f.principal.as_ref().is_some_and(|x| !f.agents.contains(x)) {
            f.principal = None;
        }
    }
    let folders: HashSet<String> = p.agent_folders.iter().map(|f| f.id.clone()).collect();
    p.agent_order
        .retain(|i| (folders.contains(i) || live.contains(i.as_str())) && seen.insert(i.clone()));
    for f in &p.agent_folders {
        if !seen.contains(&f.id) {
            p.agent_order.push(f.id.clone());
        }
    }
    let mut unlisted: Vec<&Member> = members.iter().filter(|m| !seen.contains(m.id)).collect();
    unlisted.sort_by_key(|m| m.created_at);
    p.agent_order
        .extend(unlisted.into_iter().map(|m| m.id.to_string()));
    (p.agent_folders.clone(), p.agent_order.clone()) != before
}

pub fn folder_of<'a>(p: &'a Project, agent: &str) -> Option<&'a AgentFolder> {
    p.agent_folders
        .iter()
        .find(|f| f.agents.iter().any(|a| a == agent))
}

/// Adds `agent` at the bottom of the folder, or of the list (also when the folder is gone).
pub fn add_agent(p: &mut Project, agent: &str, folder: Option<&str>) {
    remove_agent(p, agent);
    match p
        .agent_folders
        .iter_mut()
        .find(|f| Some(f.id.as_str()) == folder)
    {
        Some(f) => f.agents.push(agent.to_string()),
        None => p.agent_order.push(agent.to_string()),
    }
}

/// Puts `agent` right after `after`, at its level (the bottom of the list if `after` is unplaced).
pub fn place_after(p: &mut Project, agent: &str, after: &str) {
    if agent == after {
        return;
    }
    remove_agent(p, agent);
    let list = match p
        .agent_folders
        .iter_mut()
        .find(|f| f.agents.iter().any(|a| a == after))
    {
        Some(f) => &mut f.agents,
        None => &mut p.agent_order,
    };
    let at = list
        .iter()
        .position(|a| a == after)
        .map_or(list.len(), |i| i + 1);
    list.insert(at, agent.to_string());
}

/// Forgets `agent` everywhere: folders, principal, order.
pub fn remove_agent(p: &mut Project, agent: &str) {
    for f in &mut p.agent_folders {
        f.agents.retain(|a| a != agent);
        if f.principal.as_deref() == Some(agent) {
            f.principal = None;
        }
    }
    p.agent_order.retain(|a| a != agent);
}

/// Moves an agent or a folder to `folder` (None: the list) at rank `index` of that level once
/// it is taken out, the bottom if None.
pub fn move_item(
    p: &mut Project,
    item: &str,
    folder: Option<&str>,
    index: Option<usize>,
) -> anyhow::Result<()> {
    let is_folder = p.agent_folders.iter().any(|f| f.id == item);
    if !is_folder && !p.agent_order.iter().any(|a| a == item) && folder_of(p, item).is_none() {
        bail!(tr!(
            "« {item} » n'existe pas dans ce projet",
            "“{item}” doesn’t exist in this project"
        ));
    }
    if let Some(t) = folder {
        if !p.agent_folders.iter().any(|f| f.id == t) {
            bail!(tr!(
                "Le dossier {t} n'existe pas",
                "The folder {t} doesn’t exist"
            ));
        }
        if is_folder {
            bail!(tr!(
                "Un dossier ne se range pas dans un dossier",
                "A folder can’t go into a folder"
            ));
        }
    }
    for f in &mut p.agent_folders {
        f.agents.retain(|a| a != item);
        if f.principal.as_deref() == Some(item) && folder != Some(f.id.as_str()) {
            f.principal = None;
        }
    }
    p.agent_order.retain(|a| a != item);
    let list = match folder {
        Some(t) => {
            &mut p
                .agent_folders
                .iter_mut()
                .find(|f| f.id == t)
                .unwrap()
                .agents
        }
        None => &mut p.agent_order,
    };
    let at = index.map_or(list.len(), |i| i.min(list.len()));
    list.insert(at, item.to_string());
    Ok(())
}

/// Adds the folder in the place of the agent `at` (the bottom without it); `f.agents` leave
/// theirs.
pub fn insert_folder(p: &mut Project, f: AgentFolder, at: Option<&str>) {
    for a in &f.agents {
        remove_agent_keep_order(p, a);
    }
    let mut order = Vec::with_capacity(p.agent_order.len() + 1);
    let mut placed = false;
    for id in std::mem::take(&mut p.agent_order) {
        if Some(id.as_str()) == at {
            order.push(f.id.clone());
            placed = true;
        } else if !f.agents.contains(&id) {
            order.push(id);
        }
    }
    if !placed {
        order.push(f.id.clone());
    }
    p.agent_order = order;
    p.agent_folders.push(f);
}

/// Only the folders' side of `remove_agent`: `insert_folder` rebuilds the order itself.
fn remove_agent_keep_order(p: &mut Project, agent: &str) {
    for f in &mut p.agent_folders {
        f.agents.retain(|a| a != agent);
        if f.principal.as_deref() == Some(agent) {
            f.principal = None;
        }
    }
}

/// Removes the folder; its agents take its place, in their order.
pub fn remove_folder(p: &mut Project, folder: &str) -> anyhow::Result<()> {
    let Some(i) = p.agent_folders.iter().position(|f| f.id == folder) else {
        bail!(tr!(
            "Le dossier {folder} n'existe pas",
            "The folder {folder} doesn’t exist"
        ));
    };
    let f = p.agent_folders.remove(i);
    match p.agent_order.iter().position(|a| a == folder) {
        Some(at) => {
            p.agent_order.splice(at..=at, f.agents);
        }
        None => p.agent_order.extend(f.agents),
    }
    Ok(())
}

/// The sidebar's list: archived agents out, the unlisted ones at the bottom by age, the loose
/// agents that wait first, a folder's principal on top of its own.
pub fn shown(p: &Project, members: &[Member]) -> Vec<Shown> {
    let mut p = p.clone();
    settle(&mut p, members);
    let get = |id: &str| members.iter().find(|m| m.id == id);
    let live = |id: &String| get(id).filter(|m| !m.archived);
    let (waiting, rest): (Vec<&String>, Vec<&String>) = p
        .agent_order
        .iter()
        .filter(|i| live(i).is_some() || p.agent_folders.iter().any(|f| &f.id == *i))
        .partition(|i| live(i).is_some_and(|m| m.waiting));
    let mut out: Vec<Shown> = waiting
        .into_iter()
        .map(|i| Shown::Agent(i.clone()))
        .collect();
    for i in rest {
        match p.agent_folders.iter().find(|f| &f.id == i) {
            None => out.push(Shown::Agent(i.clone())),
            Some(f) => {
                let mut agents: Vec<&String> =
                    f.agents.iter().filter(|a| live(a).is_some()).collect();
                // Stable: the principal, then the waiting, then the others.
                agents.sort_by_key(|a| {
                    if f.principal.as_ref() == Some(*a) {
                        0
                    } else if live(a).is_some_and(|m| m.waiting) {
                        1
                    } else {
                        2
                    }
                });
                out.push(Shown::Folder {
                    id: f.id.clone(),
                    agents: agents.into_iter().cloned().collect(),
                });
            }
        }
    }
    out
}

/// `shown`, flat.
pub fn shown_agents(p: &Project, members: &[Member]) -> Vec<String> {
    shown(p, members)
        .into_iter()
        .flat_map(|s| match s {
            Shown::Agent(id) => vec![id],
            Shown::Folder { agents, .. } => agents,
        })
        .collect()
}
