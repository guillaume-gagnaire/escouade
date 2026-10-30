//! The code index through the tools (ESC-253, `docs/FACTORY.md` § 1): search a symbol or a notion,
//! the context of a task, who calls a function and what it calls, what a change would touch. Five
//! tools, so that an agent of any harness asks the index instead of reading twenty files.
//!
//! Each answer says what it saved: the tokens of the files it draws on, read whole, against its own
//! (`saving`, and a line that says it in words). The saving is counted per agent and per project
//! (`Stats::record_index`), for the statistics.
//!
//! The project is named through `resolve::project`, so a private project says nothing of its code
//! to the agents of the others (`Sight`); a private project's index holds only what its fence
//! allows. An agent asking about its own project is answered for the folder it works in (its
//! worktree), the files checked first. A project that has not turned the index on is told so, with
//! how to turn it on.

use super::read::json;
use super::resolve;
use super::tools::ToolError;
use super::Caller;
use crate::codeindex::{
    Answer, BuildContextOptions, Reader, Retrieval, Wanted, MAX_MATCHES_DEFAULT,
};
use crate::core::Core;
use crate::model::Project;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tauri::Runtime;

use rmcp::schemars;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 50;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out: its own."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "What to find: the words of a symbol's name (parseConfig is parse and config), of its doc or of its path, by prefix."
    )]
    pub query: String,
    #[schemars(
        description = "Only this kind of symbol: function, method, class, struct, type, const, field, component, macro, heading..."
    )]
    pub kind: Option<String>,
    #[schemars(description = "The most symbols to give (10 by default, 50 at most).")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ContextArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out: its own."
    )]
    pub project: Option<String>,
    #[schemars(description = "The task, told in words, as you would tell it to a colleague.")]
    pub task: String,
    #[schemars(
        description = "Extra search keywords for synonym expansion. Use this when the task uses conceptual terms that may not match symbol names — e.g. for 'authentication', pass [\"login\", \"session\", \"credential\", \"token\", \"auth\"]. The index is searched for each keyword independently; a keyword of several words is also looked for verbatim in the code."
    )]
    pub keywords: Option<Vec<String>>,
    #[schemars(
        description = "The most symbols to give, entry points and the symbols linked to them (20 by default, 100 at most)."
    )]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CallArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out: its own."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The symbol, by its id (the id index_search gives, or that of a match of an earlier answer). Takes precedence over name; one of the two is needed."
    )]
    pub id: Option<i64>,
    #[schemars(
        description = "The function, method or class, by its name: forget, Runtime::forget or Runtime.forget (its last segments). Each definition it names is answered in its own group (matches), up to max_matches; the others are listed in more, to ask again by their id."
    )]
    pub name: Option<String>,
    #[schemars(
        description = "The file it is in (its path, or the end of it): only the definitions of this file are answered."
    )]
    pub path: Option<String>,
    #[schemars(
        description = "The most results to give for each definition (10 by default, 50 at most)."
    )]
    pub limit: Option<u32>,
    #[schemars(
        description = "When the name names several definitions, how many to answer (3 by default, 20 at most)."
    )]
    pub max_matches: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ImpactArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out: its own."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "What changes, by its id (the id index_search gives). Takes precedence over target; one of the two is needed."
    )]
    pub id: Option<i64>,
    #[schemars(
        description = "What changes: a symbol's name (each definition it names in its own group), or a file's path."
    )]
    pub target: Option<String>,
    #[schemars(
        description = "When the name names several definitions, how many to answer (3 by default, 20 at most)."
    )]
    pub max_matches: Option<u32>,
    #[schemars(
        description = "How far to follow the callers (2 by default, 4 at most): the callers of the callers..."
    )]
    pub depth: Option<u32>,
    #[schemars(description = "The most results of each sort to give (10 by default, 50 at most).")]
    pub limit: Option<u32>,
}

fn limit_of(limit: Option<u32>) -> usize {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize
}

fn failed(e: anyhow::Error) -> ToolError {
    ToolError::Failed(e.to_string())
}

fn joined(e: tokio::task::JoinError) -> ToolError {
    ToolError::Failed(e.to_string())
}

/// What a call is about: the project, the folder an agent of it works in, the agent.
struct Asked {
    project: Project,
    dir: Option<String>,
    agent: String,
}

fn asked<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    project: Option<&str>,
) -> Result<Asked, ToolError> {
    let own = match caller {
        Caller::Agent(id) => core.agent(id).ok().map(|h| {
            let m = h.lock().meta.clone();
            (id.clone(), m.project_id, m.cwd)
        }),
        Caller::External => None,
    };
    let named = project.map(str::trim).filter(|p| !p.is_empty());
    let project = match (named, &own) {
        (Some(p), _) => resolve::project(core, caller, p)?,
        (None, Some((_, pid, _))) => resolve::project(core, caller, pid)?,
        (None, None) => resolve::project(core, caller, "")?,
    };
    let (agent, dir) = match own {
        Some((id, pid, cwd)) if pid == project.id => (id, Some(cwd)),
        Some((id, ..)) => (id, None),
        None => (String::new(), None),
    };
    if !core.code_index_on(&project) {
        return Err(ToolError::Failed(tr!(
            "L’index du code est coupé pour le projet « {name} » : l’utilisateur l’allume dans les réglages du projet (« Index du code ») ou pour tous les projets (« Index du code sur tous les projets »). En attendant, cherche dans les fichiers.",
            "The code index is off for the project “{name}”: the user turns it on in the project's settings (“Code index”) or for every project (“Code index on every project”). Meanwhile, search the files.",
            name = project.name
        )));
    }
    Ok(Asked {
        project,
        dir,
        agent,
    })
}

/// A reader of the index for the call, the files checked first.
async fn reader<R: Runtime>(core: &Arc<Core<R>>, a: &Asked) -> Result<Reader, ToolError> {
    core.code_index(&a.project.id, a.dir.as_deref())
        .await
        .map_err(failed)
}

/// The answer: the result, what it saved (counted for the agent and the project), and that in words.
fn answered<R: Runtime, T: serde::Serialize>(
    core: &Core<R>,
    a: &Asked,
    tool: &str,
    answer: Answer<T>,
) -> Result<String, ToolError> {
    let s = &answer.saving;
    // Priced at the model the agent last ran on, else its setting (ESC-254).
    let model = core
        .agent(&a.agent)
        .map(|h| h.lock().meta.model.clone())
        .unwrap_or_default();
    core.stats.record_index(
        &a.agent,
        &a.project.id,
        tool,
        s.files_tokens,
        s.answer_tokens,
        &model,
    );
    let summary = if s.files == 0 {
        tr!(
            "Rien de trouvé : aucun fichier n’a été lu pour cette réponse.",
            "Nothing found: no file was read for this answer."
        )
    } else {
        tr!(
            "Économie : environ {saved} tokens ({percent} %). Lire les {files} fichiers concernés en entier aurait coûté environ {before} tokens ; cette réponse en coûte environ {after}.",
            "Saved: about {saved} tokens ({percent} %). Reading the {files} files concerned whole would have cost about {before} tokens; this answer costs about {after}.",
            saved = s.saved_tokens,
            percent = s.saved_percent,
            files = s.files,
            before = s.files_tokens,
            after = s.answer_tokens
        )
    };
    json(&json!({
        "project": a.project.name,
        "projectId": a.project.id,
        "result": answer.result,
        "saving": answer.saving,
        "summary": summary,
    }))
}

pub(super) async fn search<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    args: SearchArgs,
) -> Result<String, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    let r = reader(core, &a).await?;
    let limit = limit_of(args.limit);
    let kind = args.kind.filter(|k| !k.trim().is_empty());
    let found = tokio::task::spawn_blocking(move || r.search(&args.query, limit, kind.as_deref()))
        .await
        .map_err(joined)?
        .map_err(failed)?;
    answered(core, &a, "index_search", found)
}

pub(super) async fn context<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    args: ContextArgs,
) -> Result<String, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    let r = reader(core, &a).await?;
    let options = BuildContextOptions::of_tool(
        args.limit.map(|l| l as usize),
        args.keywords.unwrap_or_default(),
    );
    let found = tokio::task::spawn_blocking(move || r.task_context_with(&args.task, &options))
        .await
        .map_err(joined)?
        .map_err(failed)?;
    let retrieval = retrieval_text(
        &found.result.retrieval,
        found.result.entry_points.is_empty(),
    );
    let mut text = answered(core, &a, "index_context", found)?;
    // The diagnostic in words, after the rest (tokensave's « ### Retrieval »).
    if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) {
        v["retrieval"] = json!(retrieval);
        text = json(&v)?;
    }
    Ok(text)
}

/// Ported from tokensave `src/context/formatter.rs` (github.com/aovestdipaperino/tokensave, commit
/// 580936b, MIT, © 2025 Enzo Lombardi; see THIRD-PARTY-NOTICES.md). The diagnostic of a context in
/// words (`format_context_as_markdown`, its
/// « Retrieval » section): the match and its best score, the terms that found something and how
/// much, the terms that found nothing (six named at most).
pub(crate) fn retrieval_text(r: &Retrieval, no_entry: bool) -> String {
    const MAX_MISSES_SHOWN: usize = 6;
    let mut lines: Vec<String> = Vec::new();
    match (r.quality.as_deref(), r.best_score) {
        (Some(quality), Some(score)) => {
            let score = if score < 0.01 {
                "<0.01".to_string()
            } else {
                format!("{score:.2}")
            };
            lines.push(tr!(
                "correspondance : {quality} (meilleur score {score})",
                "match: {quality} (best score {score})",
                quality = quality,
                score = score
            ));
        }
        _ if r.terms.is_empty() => lines.push(tr!(
            "correspondance : aucune (aucun terme à chercher n’a été tiré de la tâche — reformule avec des identifiants concrets ou ajoute des `keywords`)",
            "match: none (no searchable terms extracted — rephrase with concrete identifiers or add `keywords`)"
        )),
        _ => lines.push(tr!(
            "correspondance : aucune (aucun candidat)",
            "match: none (no candidates)"
        )),
    }
    let hits: Vec<String> = r
        .terms
        .iter()
        .filter(|t| t.hits > 0)
        .map(|t| format!("{}({})", t.term, t.hits))
        .collect();
    if !hits.is_empty() {
        lines.push(tr!(
            "termes : {list}",
            "terms: {list}",
            list = hits.join(", ")
        ));
    }
    let misses: Vec<&str> = r
        .terms
        .iter()
        .filter(|t| t.hits == 0)
        .map(|t| t.term.as_str())
        .collect();
    if !misses.is_empty() {
        let shown = misses.len().min(MAX_MISSES_SHOWN);
        let mut listed = misses[..shown].join(", ");
        if misses.len() > shown {
            listed.push_str(&format!(", +{}", misses.len() - shown));
        }
        lines.push(tr!(
            "⚠ sans résultat ({n}) : {listed} — ajoute des synonymes dans `keywords` ou essaie index_search",
            "⚠ no hits ({n}): {listed} — add `keywords` synonyms or try index_search",
            n = misses.len(),
            listed = listed
        ));
    }
    if r.terms.is_empty() && r.best_score.is_none() && !no_entry {
        return String::new();
    }
    lines.join("\n")
}

/// What a tool is asked about: an id, else a name (and a file); neither is said.
fn wanted<'a>(
    id: Option<i64>,
    name: Option<&'a str>,
    path: Option<&'a str>,
    max_matches: Option<u32>,
) -> Result<Wanted<'a>, ToolError> {
    if let Some(id) = id {
        return Ok(Wanted::Id(id));
    }
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| {
            ToolError::Failed(tr!(
                "Il faut le symbole : son id (celui que donne index_search) ou son nom.",
                "The symbol is needed: its id (the one index_search gives) or its name."
            ))
        })?;
    Ok(Wanted::Name {
        name,
        path: path.map(str::trim).filter(|p| !p.is_empty()),
        max_matches: max_matches.map_or(MAX_MATCHES_DEFAULT, |m| m as usize),
    })
}

pub(super) async fn callers<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    args: CallArgs,
) -> Result<String, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    wanted(args.id, args.name.as_deref(), None, None)?;
    let r = reader(core, &a).await?;
    let limit = limit_of(args.limit);
    let found = tokio::task::spawn_blocking(move || {
        let w = wanted(
            args.id,
            args.name.as_deref(),
            args.path.as_deref(),
            args.max_matches,
        );
        r.callers(w.map_err(|_| anyhow::anyhow!("no symbol"))?, limit)
    })
    .await
    .map_err(joined)?
    .map_err(failed)?;
    answered(core, &a, "index_callers", found)
}

pub(super) async fn callees<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    args: CallArgs,
) -> Result<String, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    wanted(args.id, args.name.as_deref(), None, None)?;
    let r = reader(core, &a).await?;
    let limit = limit_of(args.limit);
    let found = tokio::task::spawn_blocking(move || {
        let w = wanted(
            args.id,
            args.name.as_deref(),
            args.path.as_deref(),
            args.max_matches,
        );
        r.callees(w.map_err(|_| anyhow::anyhow!("no symbol"))?, limit)
    })
    .await
    .map_err(joined)?
    .map_err(failed)?;
    answered(core, &a, "index_callees", found)
}

pub(super) async fn impact<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    args: ImpactArgs,
) -> Result<String, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    wanted(args.id, args.target.as_deref(), None, None)?;
    let r = reader(core, &a).await?;
    let limit = limit_of(args.limit);
    let depth = args.depth.unwrap_or(2).clamp(1, 4);
    let found = tokio::task::spawn_blocking(move || {
        let w = wanted(args.id, args.target.as_deref(), None, args.max_matches);
        r.impact(w.map_err(|_| anyhow::anyhow!("no symbol"))?, depth, limit)
    })
    .await
    .map_err(joined)?
    .map_err(failed)?;
    answered(core, &a, "index_impact", found)
}
