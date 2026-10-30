//! The code index through the tools (ESC-253, `docs/FACTORY.md` § 1): search a symbol, a notion or
//! a text, the context of a task, the body of a symbol, who calls a function and what it calls,
//! what a change would touch. Six tools, so that an agent of any harness asks the index instead of
//! reading twenty files.
//!
//! The answers are tokensave's (ESC-298, commit 580936b; see THIRD-PARTY-NOTICES.md): Markdown for
//! the context (5 code blocks at most, 1,500 bytes each, a cut one pointing at `index_body`), one
//! line per symbol or per match for the search, the body as text, JSON for the callers, callees
//! and impact. Each ends with what it saved, counted as tokensave counts it (`codeindex::accounting`):
//! the files touched read whole, capped at 4 times the answer, against the answer, the request and,
//! once per session, the tools' schemas; a shortfall is carried to the next calls. The saving is
//! counted per agent and per project (`Stats::record_index`), for the statistics.
//!
//! The project is named through `resolve::project`, so a private project says nothing of its code
//! to the agents of the others (`Sight`); a private project's index holds only what its fence
//! allows. An agent asking about its own project is answered for the folder it works in (its
//! worktree), the files checked first. A project that has not turned the index on is told so, with
//! how to turn it on.

use super::resolve;
use super::tools::ToolError;
use super::Caller;
use crate::codeindex::accounting::Call;
use crate::codeindex::filters::PathFilter;
use crate::codeindex::{
    format, Answer, BuildContextOptions, Reader, ToolArgs, Wanted, LITERAL_SCAN_DEFAULT_TIMEOUT_MS,
    MAX_MATCHES_DEFAULT,
};
use crate::core::Core;
use crate::i18n::Lang;
use crate::model::Project;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::Runtime;

use rmcp::schemars;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 50;
/// tokensave `handle_search`: 10 by default, 500 at most.
const SEARCH_MAX_LIMIT: u32 = 500;

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct SearchArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out: its own."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "Search query string to match against symbol names: the words of the code (parseConfig is parse and config), by prefix. With literal, the exact text to find."
    )]
    pub query: String,
    #[schemars(
        description = "Only this kind of symbol: function, method, class, struct, type, const, field, component, macro, heading..."
    )]
    pub kind: Option<String>,
    #[schemars(description = "Maximum number of results to return (default: 10, 500 at most).")]
    pub limit: Option<u32>,
    #[schemars(
        description = "Only include results whose file path contains one of these substrings (e.g. \"src\", \"app\"). Empty/absent means no path constraint."
    )]
    pub path_include: Option<Vec<String>>,
    #[schemars(
        description = "Exclude results whose file path contains any of these substrings (e.g. \"node_modules\", \"dist\", \"venv\"). Takes precedence over path_include."
    )]
    pub path_exclude: Option<Vec<String>>,
    #[schemars(
        description = "Exact-substring search over source text (for runtime error strings); returns file/line locations instead of ranked symbols, plus `total` matches and `truncated` when `limit` cut the list. Case-sensitive. Default false."
    )]
    pub literal: Option<bool>,
    #[schemars(
        description = "Include node IDs / enclosing IDs in results. Default false; pass true when you plan a follow-up call (index_body, index_callers, index_callees, index_impact)."
    )]
    pub ids: Option<bool>,
    #[schemars(
        description = "Literal mode only. Count every match in the project, not just the ones shown, so `total` is exact. Default true. Pass false to stop the scan as soon as `limit` matches are found: `total` then becomes a lower bound."
    )]
    pub exact_total: Option<bool>,
    #[schemars(
        description = "Literal mode only. Give up on the scan after this many milliseconds and answer with the matches found so far. Default 30000; 0 disables the limit."
    )]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ContextArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out: its own."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "Natural language description of the task or question, with the words of the code (its identifiers, in English here) where you know them."
    )]
    pub task: String,
    #[schemars(
        description = "Extra search keywords for synonym expansion. Use this when the task uses conceptual terms that may not match symbol names — e.g. for 'authentication', pass [\"login\", \"session\", \"credential\", \"token\", \"auth\"]. The index is searched for each keyword independently; a keyword of several words is also looked for verbatim in the code."
    )]
    pub keywords: Option<Vec<String>>,
    #[schemars(description = "Maximum number of symbols to include (default: 20, 100 at most).")]
    #[serde(alias = "limit")]
    pub max_nodes: Option<u32>,
    #[schemars(
        description = "If true (the default), include source code snippets for the entry points."
    )]
    pub include_code: Option<bool>,
    #[schemars(
        description = "Maximum number of code snippets when include_code is true (default: 5, 20 at most)."
    )]
    pub max_code_blocks: Option<u32>,
    #[schemars(
        description = "Maximum lines per code snippet (default: unlimited, max 1000). Use it to see the signature plus the first lines of several symbols instead of one whole large body. A separate size limit of ~1500 bytes per snippet always applies. Truncated snippets name the index_body call that returns the rest."
    )]
    pub max_code_lines: Option<u32>,
    #[schemars(
        description = "Node IDs to exclude from results (pass seen_node_ids from previous call for session deduplication)."
    )]
    pub exclude_node_ids: Option<Vec<i64>>,
    #[schemars(
        description = "Maximum symbols from a single file in results. Prevents one large file from dominating (default: max_nodes/3, minimum 3)."
    )]
    pub max_per_file: Option<u32>,
    #[schemars(
        description = "Only entry points in this folder or file (its path from the project's root)."
    )]
    pub path: Option<String>,
    #[schemars(
        description = "Only include results whose file path contains one of these substrings (e.g. \"src\", \"app\"). Empty/absent means no path constraint."
    )]
    pub path_include: Option<Vec<String>>,
    #[schemars(
        description = "Exclude results whose file path contains any of these substrings (e.g. \"node_modules\", \"dist\", \"venv\"). Takes precedence over path_include."
    )]
    pub path_exclude: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct BodyArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out: its own."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "Symbol name to look up (e.g. 'resolve_provider_api_key', 'GraphStats'). Qualify it to disambiguate: 'Coordinator::RefreshAsync', 'Coordinator.RefreshAsync' and 'App.Coordinator.RefreshAsync' all work (trailing segments are matched)."
    )]
    pub symbol: Option<String>,
    #[schemars(
        description = "Node id of the symbol (from a candidate list, index_search with ids, index_context's seen_node_ids or a truncated snippet). Takes precedence over `symbol`."
    )]
    #[serde(alias = "id")]
    pub node_id: Option<i64>,
    #[schemars(
        description = "Maximum number of candidates listed when the name is ambiguous (default: 20, max: 50)."
    )]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct CallArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out: its own."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The symbol, by its id (the id index_search gives with ids, or that of a match of an earlier answer). Takes precedence over name; one of the two is needed."
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

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ImpactArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out: its own."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "What changes, by its id (the id index_search gives with ids). Takes precedence over target; one of the two is needed."
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

/// An answer of the tools: its text, then the metrics line, in a block of their own.
pub(super) type Reply = (String, String);

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

/// The path filters of a call: its lists, its folder, and the project's `.escouade/queryignore`
/// (the folder's, unless the fence refuses it).
fn filter_of(
    r: &Reader,
    prefix: Option<&str>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
) -> PathFilter {
    let ignore = r.query_ignore();
    PathFilter::new(
        prefix,
        include.unwrap_or_default(),
        exclude.unwrap_or_default(),
        ignore,
    )
}

/// The answer as sent and what it saved: counted in the caller's session (`accounting`), recorded
/// for the agent and the project, then said in the metrics line.
fn answered<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    a: &Asked,
    tool: &str,
    arguments: Value,
    schema_tokens: u64,
    (text, files_tokens): Text,
) -> Reply {
    let lang = crate::i18n::ui();
    let key = match caller {
        Caller::Agent(id) => id.clone(),
        Caller::External => String::new(),
    };
    let m = core.codeindex.ledger.account(
        &key,
        &Call {
            tool,
            arguments: &arguments,
            response: &text,
            full_file_tokens: files_tokens,
            schema_tokens,
        },
        |m| format::metrics_line(lang, m),
    );
    // Priced at the model the agent last ran on, else its setting (ESC-254).
    let model = core
        .agent(&a.agent)
        .map(|h| h.lock().meta.model.clone())
        .unwrap_or_default();
    core.stats
        .record_index(&a.agent, &a.project.id, tool, &m, &model);
    let line = if m.before > 0 {
        format::metrics_line(lang, &m)
    } else {
        String::new()
    };
    (text, line)
}

/// The arguments as the client sent them (those it left out dropped), for the request's cost.
pub(super) fn sent<T: serde::Serialize>(args: &T) -> Value {
    let mut v = serde_json::to_value(args).unwrap_or_default();
    if let Some(o) = v.as_object_mut() {
        o.retain(|_, x| !x.is_null());
    }
    v
}

/// An answer of a tool before it is accounted for: its text, and the raw weight of the files it
/// touched (`Saving::files_tokens`). The `*_on` functions compute it on a reader, for the app's
/// server and for the bench of the real agents (`codeindex::probes::agents`) alike.
pub(crate) type Text = (String, u64);

fn text_of<T>(answer: &Answer<T>, text: String) -> Text {
    (text, answer.saving.files_tokens)
}

/// Runs a tool's computation off the async threads.
async fn blocking(
    f: impl FnOnce() -> Result<Text, ToolError> + Send + 'static,
) -> Result<Text, ToolError> {
    tokio::task::spawn_blocking(f).await.map_err(joined)?
}

pub(crate) fn search_on(r: Reader, args: SearchArgs, lang: Lang) -> Result<Text, ToolError> {
    let limit = args
        .limit
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, SEARCH_MAX_LIMIT) as usize;
    let filter = filter_of(&r, None, args.path_include, args.path_exclude);
    let ids = args.ids.unwrap_or(false);
    if args.literal.unwrap_or(false) {
        if args.query.is_empty() {
            return Err(ToolError::Failed(tr_in!(
                lang,
                "La recherche littérale demande un texte (query) non vide.",
                "literal search requires a non-empty query"
            )));
        }
        let exact_total = args.exact_total.unwrap_or(true);
        let timeout_ms = args.timeout_ms.unwrap_or(LITERAL_SCAN_DEFAULT_TIMEOUT_MS);
        let timeout = (timeout_ms > 0).then(|| std::time::Duration::from_millis(timeout_ms));
        let found = r
            .literal_search(&args.query, limit, &filter, exact_total, timeout)
            .map_err(failed)?;
        let text = format::literal_text(lang, &found.result, ids);
        return Ok(text_of(&found, text));
    }
    let kind = args.kind.filter(|k| !k.trim().is_empty());
    let found = r
        .search_filtered(&args.query, limit, kind.as_deref(), &filter)
        .map_err(failed)?;
    let text = format::search_text(lang, &found.result, ids);
    Ok(text_of(&found, text))
}

pub(crate) fn context_on(r: Reader, args: ContextArgs, lang: Lang) -> Result<Text, ToolError> {
    let filter = filter_of(
        &r,
        args.path.as_deref(),
        args.path_include,
        args.path_exclude,
    );
    let options = BuildContextOptions::of_args(ToolArgs {
        max_nodes: args.max_nodes.map(|v| v as usize),
        include_code: args.include_code,
        max_code_blocks: args.max_code_blocks.map(|v| v as usize),
        max_code_lines: args.max_code_lines.map(|v| v as usize),
        keywords: args.keywords.unwrap_or_default(),
        exclude_node_ids: args.exclude_node_ids.unwrap_or_default(),
        max_per_file: args.max_per_file.map(|v| v as usize),
        filter,
    });
    let found = r.task_context_with(&args.task, &options).map_err(failed)?;
    let text = format::context_markdown(lang, &args.task, &found.result);
    Ok(text_of(&found, text))
}

/// Refuses a call to `index_body` that names no symbol.
fn body_wanted(args: &BodyArgs, lang: Lang) -> Result<(), ToolError> {
    if args.node_id.is_none() && args.symbol.as_deref().is_none_or(|s| s.trim().is_empty()) {
        return Err(ToolError::Failed(tr_in!(
            lang,
            "Il faut le symbole : son nom (symbol) ou son id (node_id).",
            "missing required parameter: symbol (or node_id)"
        )));
    }
    Ok(())
}

pub(crate) fn body_on(r: Reader, args: BodyArgs, lang: Lang) -> Result<Text, ToolError> {
    body_wanted(&args, lang)?;
    let limit = args
        .limit
        .map_or(crate::codeindex::BODY_CANDIDATES_DEFAULT, |l| l as usize);
    let found = r
        .body(args.node_id, args.symbol.as_deref(), limit)
        .map_err(failed)?;
    let text = format::body_text(lang, &found.result);
    Ok(text_of(&found, text))
}

pub(crate) fn callers_on(r: Reader, args: CallArgs) -> Result<Text, ToolError> {
    let w = wanted(
        args.id,
        args.name.as_deref(),
        args.path.as_deref(),
        args.max_matches,
    )?;
    let found = r.callers(w, limit_of(args.limit)).map_err(failed)?;
    Ok(text_of(&found, json_text(&found)?))
}

pub(crate) fn callees_on(r: Reader, args: CallArgs) -> Result<Text, ToolError> {
    let w = wanted(
        args.id,
        args.name.as_deref(),
        args.path.as_deref(),
        args.max_matches,
    )?;
    let found = r.callees(w, limit_of(args.limit)).map_err(failed)?;
    Ok(text_of(&found, json_text(&found)?))
}

pub(crate) fn impact_on(r: Reader, args: ImpactArgs) -> Result<Text, ToolError> {
    let w = wanted(args.id, args.target.as_deref(), None, args.max_matches)?;
    let depth = args.depth.unwrap_or(2).clamp(1, 4);
    let found = r.impact(w, depth, limit_of(args.limit)).map_err(failed)?;
    Ok(text_of(&found, json_text(&found)?))
}

pub(super) async fn search<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    schema_tokens: u64,
    args: SearchArgs,
    arguments: Value,
) -> Result<Reply, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    let r = reader(core, &a).await?;
    let lang = crate::i18n::ui();
    let text = blocking(move || search_on(r, args, lang)).await?;
    Ok(answered(
        core,
        caller,
        &a,
        "index_search",
        arguments,
        schema_tokens,
        text,
    ))
}

pub(super) async fn context<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    schema_tokens: u64,
    args: ContextArgs,
    arguments: Value,
) -> Result<Reply, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    let r = reader(core, &a).await?;
    let lang = crate::i18n::ui();
    let text = blocking(move || context_on(r, args, lang)).await?;
    Ok(answered(
        core,
        caller,
        &a,
        "index_context",
        arguments,
        schema_tokens,
        text,
    ))
}

pub(super) async fn body<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    schema_tokens: u64,
    args: BodyArgs,
    arguments: Value,
) -> Result<Reply, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    let lang = crate::i18n::ui();
    body_wanted(&args, lang)?;
    let r = reader(core, &a).await?;
    let text = blocking(move || body_on(r, args, lang)).await?;
    Ok(answered(
        core,
        caller,
        &a,
        "index_body",
        arguments,
        schema_tokens,
        text,
    ))
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
                "Il faut le symbole : son id (celui que donne index_search avec ids) ou son nom.",
                "The symbol is needed: its id (the one index_search gives with ids) or its name."
            ))
        })?;
    Ok(Wanted::Name {
        name,
        path: path.map(str::trim).filter(|p| !p.is_empty()),
        max_matches: max_matches.map_or(MAX_MATCHES_DEFAULT, |m| m as usize),
    })
}

/// The JSON of a result, as tokensave answers its callers, callees and impact.
fn json_text<T: serde::Serialize>(answer: &Answer<T>) -> Result<String, ToolError> {
    serde_json::to_string(&answer.result)
        .map(|t| format::truncate_response(&t))
        .map_err(|e| ToolError::Failed(e.to_string()))
}

pub(super) async fn callers<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    schema_tokens: u64,
    args: CallArgs,
    arguments: Value,
) -> Result<Reply, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    wanted(args.id, args.name.as_deref(), None, None)?;
    let r = reader(core, &a).await?;
    let text = blocking(move || callers_on(r, args)).await?;
    Ok(answered(
        core,
        caller,
        &a,
        "index_callers",
        arguments,
        schema_tokens,
        text,
    ))
}

pub(super) async fn callees<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    schema_tokens: u64,
    args: CallArgs,
    arguments: Value,
) -> Result<Reply, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    wanted(args.id, args.name.as_deref(), None, None)?;
    let r = reader(core, &a).await?;
    let text = blocking(move || callees_on(r, args)).await?;
    Ok(answered(
        core,
        caller,
        &a,
        "index_callees",
        arguments,
        schema_tokens,
        text,
    ))
}

pub(super) async fn impact<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    schema_tokens: u64,
    args: ImpactArgs,
    arguments: Value,
) -> Result<Reply, ToolError> {
    let a = asked(core, caller, args.project.as_deref())?;
    wanted(args.id, args.target.as_deref(), None, None)?;
    let r = reader(core, &a).await?;
    let text = blocking(move || impact_on(r, args)).await?;
    Ok(answered(
        core,
        caller,
        &a,
        "index_impact",
        arguments,
        schema_tokens,
        text,
    ))
}
