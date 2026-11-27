//! The tools of the agents' pages (ESC-673, AP-2; docs/AGENT-PAGES.md § 4): `show_page`, `get_page`,
//! `close_page`, on the model of `crate::pages`.
//!
//! Who shows a page:
//! - « Pages des agents » (Réglages › Agents, `Settings::agent_pages`) or the MCP server off: the tools
//!   leave the list (`Tools::list_tools`), and a call is refused;
//! - an agent of Escouade, on any harness: its page, in its project alone; a discussion too, without a
//!   `file` (it has no working folder of its own);
//! - Claude outside Escouade: not yet (P9 would let it show a page with a request alone);
//!
//! The request a page carries (`ask`, AP-4) is left in the box by the same call, as `ask_user` leaves
//! one (`Core::ask_deposit`): a template draws its options, a click in the page only proposes
//! (`_/choose`, checked by the core), and the band of the window sends on the user's gesture. A
//! go-ahead (`gate`) is never drawn in the page: the user answers it in Escouade alone.
//! - a page is known to its agent alone: another's, in this project or another (a private one among
//!   them), is not found, in the same words as one that does not exist.

use super::tools::ToolError;
use super::Caller;
use crate::agentpage_templates as templates;
use crate::core::Core;
use crate::pages::{self, Author, Show};
pub use escouade_mcp::args::{PageArgs, PageAskArg, PageAskKindArg, PageTemplateArg, ShowPageArgs};
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

/// How a template looks: the app's language and theme, the project's tint, and the request it
/// carries.
fn look<R: Runtime>(
    core: &Core<R>,
    project: &str,
    choice: Option<templates::Choice>,
) -> templates::Look {
    let theme = match core.settings.read().theme {
        crate::model::ThemeChoice::Light => templates::Theme::Light,
        _ => templates::Theme::Dark,
    };
    templates::Look {
        lang: crate::i18n::ui(),
        theme,
        tint: core.project(project).ok().map(|p| p.color),
        choice,
    }
}

/// The labels of a confirm's yes and no: its own, else the box's.
fn yes_no(a: &crate::asks::Ask) -> (String, String) {
    let yes = if a.yes_label.trim().is_empty() {
        tr!("Oui", "Yes")
    } else {
        a.yes_label.clone()
    };
    let no = if a.no_label.trim().is_empty() {
        tr!("Non", "No")
    } else {
        a.no_label.clone()
    };
    (yes, no)
}

/// The request `a` as a template draws it: a choice's options, or a confirm's yes and no; none for a
/// go-ahead, never drawn in a page (the user answers it in Escouade).
pub(crate) fn choice_of(a: &crate::asks::Ask) -> Option<templates::Choice> {
    use crate::asks::AskKind;
    if a.gate {
        return None;
    }
    let options = match a.kind {
        AskKind::Choice => a
            .options
            .iter()
            .map(|o| templates::ChoiceOption {
                id: o.id.clone(),
                label: o.label.clone(),
                description: (!o.description.is_empty()).then(|| o.description.clone()),
            })
            .collect(),
        AskKind::Confirm => {
            let (yes, no) = yes_no(a);
            vec![
                templates::ChoiceOption {
                    id: "yes".into(),
                    label: yes,
                    description: None,
                },
                templates::ChoiceOption {
                    id: "no".into(),
                    label: no,
                    description: None,
                },
            ]
        }
        _ => return None,
    };
    Some(templates::Choice {
        key: a.id.clone(),
        title: a.title.clone(),
        options,
        recommend: a.recommend.as_ref().and_then(|r| r.option.clone()),
        multi: a.multi,
        answered: crate::pages::answered_options(a),
    })
}

/// The request as a page of one's own reads it (`window.escouade.ask`): its title, options,
/// recommendation, `multi`, and the options answered (none before); null for a go-ahead.
fn ask_json(a: &crate::asks::Ask) -> Value {
    match choice_of(a) {
        Some(c) => json!({
            "title": c.title,
            "options": c.options.iter().map(|o| json!({ "id": o.id, "label": o.label })).collect::<Vec<_>>(),
            "recommend": c.recommend,
            "multi": c.multi,
            "answered": if c.answered.is_empty() { Value::Null } else { json!(c.answered) },
        }),
        None => Value::Null,
    }
}

/// The request `ask` of `show_page` as a draft of the box, checked as `ask_user` checks one; its
/// options take no file (their renders are the page).
fn page_draft<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    who: &Who,
    a: PageAskArg,
) -> Result<crate::asks::AskDraft, ToolError> {
    if a.options
        .iter()
        .flatten()
        .any(|o| o.files.as_ref().is_some_and(|f| !f.is_empty()))
    {
        return Err(ToolError::Failed(tr!(
            "Les options de la demande d’une page n’ont pas de fichiers : leurs rendus sont dans la page.",
            "The options of a page's request have no files: their renders are in the page."
        )));
    }
    if a.kind == PageAskKindArg::Confirm && a.options.as_ref().is_some_and(|o| !o.is_empty()) {
        return Err(ToolError::Failed(tr!(
            "Une demande confirm n’a pas d’options : son oui et son non (yesLabel, noLabel).",
            "A confirm request has no options: its yes and its no (yesLabel, noLabel)."
        )));
    }
    let project = core
        .project(&who.author.project_id)
        .map_err(|e| ToolError::Failed(e.to_string()))?;
    let mut tickets = Vec::new();
    for t in a.tickets.unwrap_or_default() {
        let found = super::resolve::ticket_in(core, caller, &t, Some(&project))?;
        if !tickets.contains(&found.id) {
            tickets.push(found.id);
        }
    }
    let (options, _) = super::asks::options_of(a.options.unwrap_or_default());
    let draft = crate::asks::AskDraft {
        kind: match a.kind {
            PageAskKindArg::Choice => crate::asks::AskKind::Choice,
            PageAskKindArg::Confirm => crate::asks::AskKind::Confirm,
        },
        title: a.title,
        body: a.body.unwrap_or_default(),
        options,
        multi: a.multi.unwrap_or(false),
        steps: Vec::new(),
        recommend: a.recommend.map(super::asks::recommend_of),
        unblocks: a.unblocks.unwrap_or_default(),
        tickets,
        urgency: a
            .urgency
            .map_or(crate::asks::Urgency::Normal, super::asks::urgency_of),
        expires_in_days: a.expires_in_days,
        decision: a.decision.unwrap_or(false),
        wake: a.wake.unwrap_or(true),
        yes_label: a.yes_label.unwrap_or_default(),
        no_label: a.no_label.unwrap_or_default(),
        gate: a.gate.unwrap_or(false),
    };
    crate::asks::checked(crate::i18n::ui(), draft.clone()).map_err(ToolError::Refused)?;
    Ok(draft)
}

/// What the page of one's own receives (`window.escouade`, docs/AGENT-PAGES.md § 7.4), but its number
/// and version (`pages::with_escouade`): **what a template is given, no more**: its title, the agent's
/// name, its ticket, the project's name, the app's theme and language, the agent's data, and its request
/// (`ask`, AP-4). Neither a path, a token nor an id of Escouade.
fn escouade_of<R: Runtime>(
    core: &Core<R>,
    who: &Who,
    title: &str,
    data: Option<Value>,
    ask: Value,
) -> Value {
    let look = look(core, &who.author.project_id, None);
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
        "ask": ask,
    })
}

/// What a page is made of, as the call gives it: checked, then copied (out of the async runtime: it
/// reads files).
#[allow(clippy::too_many_arguments)]
fn bundle<R: Runtime>(
    core: &Core<R>,
    who: &Who,
    template: PageTemplateArg,
    title: &str,
    data: Option<Value>,
    html: Option<String>,
    file: Option<String>,
    choice: Option<templates::Choice>,
) -> Result<(pages::Bundle, Vec<templates::Anchor>), ToolError> {
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
            Ok((b, Vec::new()))
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
                (Some(html), None) => pages::from_html(lang, &html)
                    .map(|b| (b, Vec::new()))
                    .map_err(failed),
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
                        .map(|b| (b, Vec::new()))
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
            let rendered =
                templates::render(t, title, &data, &look(core, &who.author.project_id, choice))
                    .map_err(|p| failed(templates::tell(&p)))?;
            let mut b = pages::Bundle::default();
            b.add(lang, "index.html".into(), rendered.html.into_bytes())
                .map_err(ToolError::Refused)?;
            let home = (!who.discussion).then_some(who.home.as_path());
            pages::add_images(lang, &mut b, &rendered.images, home, &pages::open_within)
                .map_err(ToolError::Refused)?;
            Ok((b, rendered.anchors))
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
    let draft = match a.ask {
        Some(x) => Some(page_draft(core, caller, &who, x)?),
        None => None,
    };
    let core2 = core.clone();
    let (shown, carried) = tokio::task::spawn_blocking(move || {
        let agent = who.author.agent.clone();
        // The request: a new one, left in the box by this call; else the one the page carries.
        let before = a
            .page
            .as_deref()
            .and_then(|p| core2.page_get(&agent, p).ok())
            .and_then(|s| s.page.ask);
        let deposited = match draft {
            Some(d) => {
                let chef = core2
                    .project(&who.author.project_id)
                    .is_ok_and(|p| p.chef.as_deref() == Some(agent.as_str()));
                let author = crate::asks::AskAuthor {
                    agent: Some(agent.clone()),
                    name: who.author.name.clone(),
                    chef,
                };
                Some(
                    core2
                        .ask_deposit(
                            author,
                            &who.author.project_id,
                            who.author.ticket_id.clone(),
                            d,
                            Vec::new(),
                        )
                        .map_err(|e| ToolError::Refused(format!("{e:#}")))?,
                )
            }
            None => None,
        };
        let carried = deposited
            .clone()
            .or_else(|| before.as_deref().and_then(|id| core2.ask(id).ok()));
        let choice = carried.as_ref().and_then(choice_of);
        let made = (|| {
            let escouade = (template == PageTemplateArg::Custom).then(|| {
                let ask = carried.as_ref().map_or(Value::Null, ask_json);
                escouade_of(&core2, &who, &title, a.data.clone(), ask)
            });
            let (bundle, anchors) = bundle(&core2, &who, template, &title, a.data, a.html, a.file, choice)?;
            core2
                .page_show(
                    &who.author,
                    Show {
                        title,
                        template: template_name(template),
                        page: a.page,
                        bundle,
                        anchors,
                        quiet,
                        escouade,
                        ask: deposited.as_ref().map(|x| x.id.clone()),
                    },
                )
                .map_err(ToolError::Refused)
        })();
        let asker = crate::asks::Asker::Agent(agent);
        match made {
            Err(e) => {
                // The page was not shown: its request goes with it.
                if let Some(d) = &deposited {
                    let _ = core2.ask_withdraw(
                        &asker,
                        &d.id,
                        &tr!("La page n’a pas été montrée.", "The page was not shown."),
                    );
                }
                Err(e)
            }
            Ok(shown) => {
                // A new request replaces the one the page carried: never two for the same page.
                if let (Some(d), Some(old)) = (&deposited, &before) {
                    if core2.ask(old).is_ok_and(|x| x.state.waiting()) {
                        let _ = core2.ask_withdraw(
                            &asker,
                            old,
                            &tr!(
                                "Remplacée par {new}, la demande d’une nouvelle version de la page.",
                                "Replaced by {new}, the request of a new version of the page.",
                                new = d.id
                            ),
                        );
                    }
                }
                Ok((shown, carried.map(|x| x.id)))
            }
        }
    })
    .await
    .map_err(|e| ToolError::Failed(e.to_string()))??;
    let mut v = json!({
        "page": shown.page,
        "browser": pages::key(&shown.page),
        "version": shown.version,
        "ask": carried,
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
pub(super) const GUIDANCE: &str = "To show the user something that reads better as a page than as text (an end-of-batch or pre-merge review, a diagram, options to compare), show it with show_page: a template (review, diagram, compare) filled with your data, or your own HTML; then update it with a new version of the same page (page: \"p-<n>\") rather than opening another, and walk the user through it with drive_browser (browser page:p-<n>: scroll_to and highlight by a key of its template such as criteria.2, tab, version). It is copied and shown in a shelter of Escouade, without network. Put in it no secret, and no third party's content you have not read again. A Claude Code agent keeps its artifacts for what must be published on claude.ai.";
