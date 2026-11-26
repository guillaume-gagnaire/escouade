//! The tools of the agents' pages (ESC-673, AP-2; docs/AGENT-PAGES.md § 4): `show_page`, `get_page`,
//! `close_page`, on the model of `crate::pages`.
//!
//! Who shows a page:
//! - « Pages des agents » (Réglages › Agents, `Settings::agent_pages`) or the MCP server off: the tools
//!   leave the list (`Tools::list_tools`), and a call is refused;
//! - an agent of Escouade, on any harness: its page, in its project alone; a discussion too, without a
//!   `file` (it has no working folder of its own);
//! - Claude outside Escouade: not yet. It shows a page with a request alone (P9), the request that
//!   comes with AP-4;
//! - a page is known to its agent alone: another's, in this project or another (a private one among
//!   them), is not found, in the same words as one that does not exist.

use super::tools::ToolError;
use super::Caller;
use crate::agentpage_templates as templates;
use crate::core::Core;
use crate::pages::{self, Author, Show};
pub use escouade_mcp::args::{PageArgs, PageTemplateArg, ShowPageArgs};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Runtime;

fn iso(ms: i64) -> Value {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| json!(d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)))
        .unwrap_or(Value::Null)
}

/// Who shows a page.
struct Who {
    author: Author,
    /// The agent's working folder.
    home: PathBuf,
    discussion: bool,
}

/// The caller as the author of a page; refused while the pages are off, and to Claude outside Escouade.
fn who<R: Runtime>(core: &Core<R>, caller: &Caller) -> Result<Who, ToolError> {
    let lang = crate::i18n::ui();
    if !core.pages_on() {
        return Err(ToolError::Refused(pages::switched_off(lang)));
    }
    let Caller::Agent(id) = caller else {
        return Err(ToolError::Refused(tr!(
            "Claude hors d’Escouade ne montre pas de page : une page s’ouvre à côté de la conversation de l’agent d’Escouade qui la montre. Laisse plutôt une demande dans la boîte (ask_user).",
            "Claude outside Escouade shows no page: a page opens beside the conversation of the agent of Escouade that shows it. Leave a request in the box instead (ask_user)."
        )));
    };
    let agent = core.agent(id).map_err(|_| {
        ToolError::Refused(tr!(
            "Escouade ne connaît plus cet agent.",
            "Escouade no longer knows this agent."
        ))
    })?;
    let a = agent.lock();
    let m = &a.meta;
    // Its ticket, if it is one of its project.
    let ticket_id = m
        .ticket_id
        .clone()
        .filter(|t| core.ticket(t).is_ok_and(|t| t.project_id == m.project_id));
    Ok(Who {
        author: Author {
            agent: m.id.clone(),
            name: m.name.clone(),
            project_id: m.project_id.clone(),
            ticket_id,
        },
        home: PathBuf::from(&m.cwd),
        discussion: m.discussion.is_some(),
    })
}

fn template_name(t: PageTemplateArg) -> &'static str {
    match t {
        PageTemplateArg::Review => "review",
        PageTemplateArg::Diagram => "diagram",
        PageTemplateArg::Compare => "compare",
        PageTemplateArg::Html => "html",
        PageTemplateArg::Custom => "custom",
    }
}

/// How a template looks: the app's language and theme, the project's tint.
fn look<R: Runtime>(core: &Core<R>, project: &str) -> templates::Look {
    let theme = match core.settings.read().theme {
        crate::model::ThemeChoice::Light => templates::Theme::Light,
        _ => templates::Theme::Dark,
    };
    templates::Look {
        lang: crate::i18n::ui(),
        theme,
        tint: core.project(project).ok().map(|p| p.color),
        choice: None,
    }
}

/// What the page of one's own receives (`window.escouade`, docs/AGENT-PAGES.md § 7.4), but its number
/// and version (`pages::with_escouade`): **what a template is given, no more**: its title, the agent's
/// name, its ticket, the project's name, the app's theme and language, the agent's data, and its request
/// (none until AP-4). Neither a path, a token nor an id of Escouade.
fn escouade_of<R: Runtime>(core: &Core<R>, who: &Who, title: &str, data: Option<Value>) -> Value {
    let look = look(core, &who.author.project_id);
    let ticket = who
        .author
        .ticket_id
        .as_deref()
        .and_then(|t| core.ticket(t).ok())
        .map(|t| json!({ "key": t.key, "title": t.title }));
    let project = core
        .project(&who.author.project_id)
        .map(|p| p.name)
        .unwrap_or_default();
    json!({
        "template": "custom",
        "title": title,
        "agent": { "name": who.author.name },
        "ticket": ticket,
        "project": { "name": project },
        "theme": match look.theme { templates::Theme::Light => "light", _ => "dark" },
        "lang": if look.lang == crate::i18n::Lang::En { "en" } else { "fr" },
        "data": data,
        "ask": null,
    })
}

/// What a page is made of, as the call gives it: checked, then copied (out of the async runtime: it
/// reads files).
fn bundle<R: Runtime>(
    core: &Core<R>,
    who: &Who,
    template: PageTemplateArg,
    title: &str,
    data: Option<Value>,
    html: Option<String>,
    file: Option<String>,
) -> Result<pages::Bundle, ToolError> {
    let lang = crate::i18n::ui();
    let failed = ToolError::Failed;
    match template {
        PageTemplateArg::Custom => {
            let project = core
                .project(&who.author.project_id)
                .map_err(|e| ToolError::Failed(e.to_string()))?;
            let page = pages::custom_page_of(&project);
            if matches!(page, pages::CustomPage::None | pages::CustomPage::Waiting) {
                return Err(ToolError::Refused(pages::no_custom_page(
                    lang,
                    page == pages::CustomPage::Waiting,
                )));
            }
            if html.is_some() || file.is_some() {
                return Err(failed(tr!(
                    "La page à soi (custom) prend ses données (data), pas de html ni de file : son document est celui que l’utilisateur a réglé.",
                    "The page of one's own (custom) takes its data (data), neither html nor file: its document is the one the user set."
                )));
            }
            let data = data.unwrap_or(Value::Null);
            let weight = data.to_string().len();
            if weight > templates::DATA_MAX {
                return Err(failed(tr!(
                    "Les données (data) d’une page font 256 Ko au plus.",
                    "A page's data (data) is 256 KB at most."
                )));
            }
            let mut b =
                pages::from_custom(lang, &page, &pages::open_within).map_err(ToolError::Refused)?;
            if b.bytes() + weight as u64 > pages::VERSION_MAX {
                return Err(ToolError::Refused(tr!(
                    "Une version pèse 10 Mo au plus, ses fichiers compris : allège tes données.",
                    "A version weighs 10 MB at most, its files included: make your data lighter."
                )));
            }
            // Its document is the user's: what it left out is not the agent's to know by name.
            b.left_out.clear();
            Ok(b)
        }
        PageTemplateArg::Html => {
            if data.is_some() {
                return Err(failed(tr!(
                    "data va avec un modèle (review, diagram, compare) : une page html se donne par html ou file.",
                    "data goes with a template (review, diagram, compare): an html page is given by html or file."
                )));
            }
            match (html, file) {
                (Some(_), Some(_)) | (None, None) => Err(failed(tr!(
                    "Donne le document (html) ou son fichier (file), l’un des deux.",
                    "Give the document (html) or its file (file), one of the two."
                ))),
                (Some(html), None) => pages::from_html(lang, &html).map_err(failed),
                (None, Some(file)) => {
                    if who.discussion {
                        return Err(ToolError::Refused(tr!(
                            "Une discussion n’a pas de dossier de travail à elle : donne le document (html).",
                            "A discussion has no working folder of its own: give the document (html)."
                        )));
                    }
                    let full = if Path::new(&file).is_absolute() || file.starts_with(['/', '\\']) {
                        PathBuf::from(&file)
                    } else {
                        who.home.join(&file)
                    };
                    pages::from_file(lang, &full, &who.home, &pages::open_within)
                        .map_err(ToolError::Refused)
                }
            }
        }
        t => {
            if html.is_some() || file.is_some() {
                return Err(failed(tr!(
                    "Un modèle (review, diagram, compare) prend ses données (data), pas de html ni de file.",
                    "A template (review, diagram, compare) takes its data (data), neither html nor file."
                )));
            }
            let Some(data) = data else {
                return Err(failed(tr!(
                    "Un modèle prend ses données (data), un objet JSON de son schéma.",
                    "A template takes its data (data), a JSON object of its schema."
                )));
            };
            let t = templates::Template::parse(template_name(t)).ok_or_else(|| {
                failed(tr!(
                    "Ce modèle n’existe pas.",
                    "This template doesn’t exist."
                ))
            })?;
            let rendered = templates::render(t, title, &data, &look(core, &who.author.project_id))
                .map_err(|p| failed(templates::tell(&p)))?;
            let mut b = pages::Bundle::default();
            b.add(lang, "index.html".into(), rendered.html.into_bytes())
                .map_err(ToolError::Refused)?;
            let home = (!who.discussion).then_some(who.home.as_path());
            pages::add_images(lang, &mut b, &rendered.images, home, &pages::open_within)
                .map_err(ToolError::Refused)?;
            Ok(b)
        }
    }
}

/// `show_page`.
pub(super) async fn show_page<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    a: ShowPageArgs,
) -> Result<String, ToolError> {
    let who = who(core, caller)?;
    let lang = crate::i18n::ui();
    let title = pages::checked_title(lang, &a.title).map_err(ToolError::Failed)?;
    let template = a.template;
    let quiet = matches!(a.open, Some(escouade_mcp::args::PageOpenArg::Quiet));
    let core2 = core.clone();
    let shown = tokio::task::spawn_blocking(move || {
        let escouade = (template == PageTemplateArg::Custom)
            .then(|| escouade_of(&core2, &who, &title, a.data.clone()));
        let bundle = bundle(&core2, &who, template, &title, a.data, a.html, a.file)?;
        core2
            .page_show(
                &who.author,
                Show {
                    title,
                    template: template_name(template),
                    page: a.page,
                    bundle,
                    quiet,
                    escouade,
                },
            )
            .map_err(ToolError::Refused)
    })
    .await
    .map_err(|e| ToolError::Failed(e.to_string()))??;
    let mut v = json!({
        "page": shown.page,
        "browser": pages::key(&shown.page),
        "version": shown.version,
        "ask": null,
        "opened": opened(core, &shown.page, quiet).await,
        "bytes": shown.bytes,
        "files": shown.files,
    });
    if !shown.left_out.is_empty() {
        v["left_out"] = json!(shown.left_out);
    }
    Ok(v.to_string())
}

/// Where the page went (AP-3): `beside` once the window put it on screen (the user was looking at the
/// agent: it opens beside its conversation, or it was already shown and reloaded where it is), else
/// `waiting`, its row waiting in « Navigateurs ». The window says so within a second and a half.
async fn opened<R: Runtime>(core: &Core<R>, page: &str, quiet: bool) -> &'static str {
    if !quiet && core.pages_shown() {
        for _ in 0..15 {
            if core.page_on_screen(page) {
                return "beside";
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    if core.page_on_screen(page) {
        "beside"
    } else {
        "waiting"
    }
}

/// The author of a call on one of its pages.
fn agent_of<R: Runtime>(core: &Core<R>, caller: &Caller) -> Result<String, ToolError> {
    Ok(who(core, caller)?.author.agent)
}

/// `get_page`: its versions, its row, never what the user did in it.
pub(super) fn get_page<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    a: PageArgs,
) -> Result<String, ToolError> {
    let agent = agent_of(core, caller)?;
    let seen = core.page_get(&agent, &a.page).map_err(ToolError::Failed)?;
    let p = &seen.page;
    let versions: Vec<Value> = p
        .versions
        .iter()
        .map(|v| json!({ "version": v.n, "at": iso(v.at), "bytes": v.bytes, "files": v.files }))
        .collect();
    let ticket = p
        .ticket_id
        .as_deref()
        .and_then(|t| core.ticket(t).ok())
        .map(|t| t.key);
    let placement = seen.row.unwrap_or_default();
    Ok(json!({
        "page": p.id,
        "browser": pages::key(&p.id),
        "title": p.title,
        "template": p.template,
        "ticket": ticket,
        "version": p.latest().map(|v| v.n),
        "versions": versions,
        "row": if seen.row.is_some() { "open" } else { "closed" },
        "placement": placement,
        "on_screen": seen.row.is_some() && placement != crate::devpage::Placement::Hidden,
        "loaded": seen.loaded,
        "ask": p.ask,
        "created": iso(p.created_at),
        "updated": iso(p.updated_at),
    })
    .to_string())
}

/// `close_page`: its row closes; the page stays kept.
pub(super) fn close_page<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    a: PageArgs,
) -> Result<String, ToolError> {
    let agent = agent_of(core, caller)?;
    let was_open = core
        .page_close(&agent, &a.page)
        .map_err(ToolError::Failed)?;
    Ok(
        json!({ "page": a.page.trim(), "closed": true, "was_open": was_open, "kept": true })
            .to_string(),
    )
}

/// The sentence of the server's instructions on the pages, given while they are on.
pub(super) const GUIDANCE: &str = "To show the user something that reads better as a page than as text (an end-of-batch or pre-merge review, a diagram, options to compare), show it with show_page: a template (review, diagram, compare) filled with your data, or your own HTML; then update it with a new version of the same page (page: \"p-<n>\") rather than opening another. It is copied and shown in a shelter of Escouade, without network. Put in it no secret, and no third party's content you have not read again. A Claude Code agent keeps its artifacts for what must be published on claude.ai.";
