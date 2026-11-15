//! The tools of the shared memory (ESC-255): `search_memory`, `read_memory`, `write_memory` and `forget_memory`, open
//! to every harness through the server, on the memory of a project the caller sees (`resolve::project`: a private
//! project's to its own agents alone). An agent of Escouade may leave out the project for its own. What they do is
//! `crate::memory`'s.

use super::read::json;
use super::resolve;
use super::tools::ToolError;
use super::Caller;
use crate::core::Core;
use crate::memory::{self, Draft, Kind};
use crate::model::Project;
use crate::vectors::embed::Embedder;
use serde_json::json;
use std::sync::Arc;
use tauri::Runtime;

/// The project the call names, or the calling agent's own when it names none.
pub(super) fn project_of<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    asked: Option<&str>,
) -> Result<Project, ToolError> {
    if let Some(asked) = asked.filter(|a| !a.trim().is_empty()) {
        return resolve::project(core, caller, asked);
    }
    let Caller::Agent(id) = caller else {
        return Err(ToolError::Failed(tr!(
            "Donne un projet : son id ou son nom.",
            "Give a project: its id or its name."
        )));
    };
    let own = core
        .agent(id)
        .map(|h| h.lock().meta.project_id.clone())
        .map_err(|e| ToolError::Failed(e.to_string()))?;
    resolve::project(core, caller, &own)
}

/// The memory a call reads or writes: a project's, or the Overseer's own when it names no project (ESC-462).
struct Place {
    /// The project's name, or the Overseer's.
    name: String,
    dir: std::path::PathBuf,
    /// The project's id (its vectors' index, the facts, the window's list); the Overseer's: none.
    project_id: Option<String>,
}

fn place_of<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    asked: Option<&str>,
) -> Result<Place, ToolError> {
    if core.overseer_calls(caller) && asked.is_none_or(|a| a.trim().is_empty()) {
        return Ok(Place {
            name: crate::overseer::NAME.into(),
            dir: core.overseer.memory_dir(),
            project_id: None,
        });
    }
    let p = project_of(core, caller, asked)?;
    Ok(Place {
        name: p.name.clone(),
        dir: memory::dir_of(&p),
        project_id: Some(p.id),
    })
}

fn failed(e: anyhow::Error) -> ToolError {
    ToolError::Failed(format!("{e:#}"))
}

/// The vectors' model, when it is downloaded: None otherwise (the search goes by the words).
fn embedder<R: Runtime>(core: &Core<R>) -> Option<Arc<dyn Embedder>> {
    core.vectors_embedder()
}

/// `search_memory`: the memories that fit `query`, by the meaning too when the vectors' model is there.
pub(super) async fn search<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    project: Option<&str>,
    query: &str,
    limit: Option<usize>,
) -> Result<String, ToolError> {
    let p = place_of(core, caller, project)?;
    let dir = p.dir.clone();
    let index = core
        .data
        .vectors_index(p.project_id.as_deref().unwrap_or(crate::overseer::PROJECT));
    let vectors = embedder(core);
    let query = query.to_string();
    let (found, how) = tokio::task::spawn_blocking(move || {
        memory::search(
            &dir,
            &query,
            limit.unwrap_or(10),
            vectors.map(|e| (e, index.as_path())),
        )
    })
    .await
    .map_err(|e| ToolError::Failed(e.to_string()))?
    .map_err(failed)?;
    json(&json!({ "project": p.name, "by": how, "memories": found }))
}

/// `read_memory`: a memory whole.
pub(super) fn read<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    project: Option<&str>,
    name: &str,
) -> Result<String, ToolError> {
    let p = place_of(core, caller, project)?;
    let m = memory::read(&p.dir, name).map_err(failed)?;
    json(&json!({ "project": p.name, "memory": m }))
}

/// `write_memory`: a memory written, made or replaced. Answers it, and for the log what changed.
pub(crate) fn write<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    project: Option<&str>,
    draft: Draft,
) -> Result<(String, String), ToolError> {
    let p = place_of(core, caller, project)?;
    if let Some(id) = &p.project_id {
        super::act::may_act_on(core, caller, id)?;
    } else {
        std::fs::create_dir_all(&p.dir).map_err(|e| failed(e.into()))?;
    }
    let (m, new) = memory::write(&p.dir, draft).map_err(failed)?;
    if let Some(id) = &p.project_id {
        core.facts.emit(
            crate::facts::Body::MemoryWritten {
                memory: m.name.clone(),
                new,
            },
            crate::facts::About::project(id),
        );
    }
    let changed = if new {
        tr!("souvenir « {n} » noté", "memory “{n}” noted", n = m.name)
    } else {
        tr!(
            "souvenir « {n} » réécrit",
            "memory “{n}” rewritten",
            n = m.name
        )
    };
    if let Some(id) = &p.project_id {
        core.memory_changed(id);
    }
    Ok((
        json(&json!({ "project": p.name, "memory": m, "new": new }))?,
        changed,
    ))
}

/// `forget_memory`: a memory's file removed.
pub(crate) fn forget<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    project: Option<&str>,
    name: &str,
) -> Result<(String, String), ToolError> {
    let p = place_of(core, caller, project)?;
    if let Some(id) = &p.project_id {
        super::act::may_act_on(core, caller, id)?;
    }
    let name = memory::name_of(name).map_err(failed)?;
    if !memory::forget(&p.dir, &name).map_err(failed)? {
        return Err(ToolError::Failed(tr!(
            "Aucun souvenir « {name} » dans la mémoire du projet.",
            "No memory “{name}” in the project's memory."
        )));
    }
    if let Some(id) = &p.project_id {
        core.facts.emit(
            crate::facts::Body::MemoryForgotten {
                memory: name.clone(),
            },
            crate::facts::About::project(id),
        );
        core.memory_changed(id);
    }
    Ok((
        json(&json!({ "project": p.name, "forgotten": name }))?,
        tr!("souvenir « {name} » oublié", "memory “{name}” forgotten"),
    ))
}

/// The kind a tool's argument gives, as the files say it.
pub(super) fn kind_of(k: Option<Kind>) -> Kind {
    k.unwrap_or_default()
}
