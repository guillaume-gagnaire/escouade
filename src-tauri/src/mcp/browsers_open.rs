//! An agent opens a real browser tab beside its conversation, and closes it (ESC-678, AB-6;
//! docs/AGENT-BROWSERS.md § 2 bis, § 3.2, § 3.7, § 5.1): `open_browser` (`url`, `launch` or
//! `artifact`, `place`) and `close_browser`. Driving it comes with `drive_browser` (AB-7).
//!
//! The addresses, security first (§ 5.1, and the squad lead's decision, stricter than the study):
//! - http and https alone: never `file:`, `data:`, `javascript:`, `blob:` nor another scheme, nor an
//!   address with a user name or a password, nor one of Escouade's own (`navigation::destination`),
//!   even with the user's leave;
//! - this machine (`localhost`, `127.0.0.0/8`, `[::1]`, `0.0.0.0`) at the ports the project's
//!   launches declare alone, never at Escouade's server's: at another port it is **refused**, not
//!   asked (a service of the machine, or Escouade's own);
//! - the origins of the project's launch commands open without asking, and so do the artifacts
//!   the agent published, found in its `Artifact` tool's results (the harness says so, not the
//!   agent; the artifact alone, not all of `claude.ai`);
//! - any other address of the internet is asked of the user (choice C3), once per origin: a band
//!   of the agent's conversation, « Permettre une fois / Refuser ». Never opened on its own.
//!
//! A tab opened here is a row of « Navigateurs » in its project's storage (`pages/<project>`),
//! never the window's: it inherits none of its cookies. It opens beside the agent's conversation
//! while the user looks at it, else its row waits (`waiting`). Bounds: 3 browsers driven per agent,
//! 10 openings an hour; one pane beside per agent (the window moves it). Agents of Escouade alone,
//! in their own project (a private project's browsers are its agents' alone), while « Agents et
//! navigateurs » is on.

use super::browsers::{self, is_loopback, origin_of, Launch};
use super::tools::ToolError;
use super::Caller;
use crate::core::Core;
use crate::devpage::{self, OpenLeave, Opened, PageEvent, PageView, Pages, Wish};
use crate::model::{AgentMeta, Project, UiEvent};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{Runtime, Url};

pub use escouade_mcp::args::{CloseBrowserArgs, OpenBrowserArgs, PlaceArg};

/// The longest address an agent opens.
const URL_MAX: usize = 2048;

/// Where an address an agent would open leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Judged {
    /// On the project's launch: opened without asking.
    Launch(Url),
    /// Another address of the internet: asked of the user (C3), once per origin.
    Ask(Url, String),
}

/// Why an address is never opened, even with the user's leave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Never {
    NotAnAddress,
    Scheme,
    Credentials,
    Escouade,
    /// This machine at a port no launch of the project declares (or Escouade's server's).
    Loopback,
    TooLong,
}

/// Where `text` leads for an agent of a project whose launch is `launch`. `own`: Escouade's own
/// addresses; `server`: the port of Escouade's MCP server, never opened.
pub(crate) fn judge(
    text: &str,
    launch: &Launch,
    own: impl Fn(&Url) -> bool,
    server: Option<u16>,
) -> Result<Judged, Never> {
    let text = text.trim();
    if text.len() > URL_MAX {
        return Err(Never::TooLong);
    }
    let url = Url::parse(text).map_err(|_| Never::NotAnAddress)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(Never::Scheme);
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(Never::NotAnAddress);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Never::Credentials);
    }
    if own(&url) {
        return Err(Never::Escouade);
    }
    let origin = url.origin().ascii_serialization();
    if is_loopback(&url) {
        let port = url.port_or_known_default();
        return match port {
            Some(p) if launch.declares(p) && Some(p) != server => Ok(Judged::Launch(url)),
            _ => Err(Never::Loopback),
        };
    }
    if launch.has_origin(&origin) {
        return Ok(Judged::Launch(url));
    }
    Ok(Judged::Ask(url, origin))
}

/// What a refusal of `judge` says.
pub(crate) fn never_said(n: Never) -> ToolError {
    ToolError::Refused(match n {
        Never::NotAnAddress => tr!(
            "Ce n’est pas une adresse : donne une adresse http ou https complète.",
            "This is not an address: give a whole http or https address."
        ),
        Never::Scheme => tr!(
            "Seules les adresses http et https s’ouvrent : jamais file:, data:, javascript:, blob: ni un autre schéma.",
            "Only http and https addresses open: never file:, data:, javascript:, blob: nor another scheme."
        ),
        Never::Credentials => tr!(
            "Une adresse avec un nom d’utilisateur ou un mot de passe ne s’ouvre pas.",
            "An address with a user name or a password does not open."
        ),
        Never::Escouade => tr!(
            "C’est une adresse d’Escouade : elle ne s’ouvre jamais dans un navigateur, même avec l’accord de l’utilisateur.",
            "This is one of Escouade's addresses: it never opens in a browser, even with the user's leave."
        ),
        Never::Loopback => tr!(
            "Ce poste (localhost, 127.0.0.1, [::1]) ne s’ouvre qu’aux ports que déclarent les lancements du projet : à un autre port, c’est peut-être un service du poste ou d’Escouade. Refusé.",
            "This machine (localhost, 127.0.0.1, [::1]) opens only at the ports the project's launches declare: at another port it may be a service of the machine or of Escouade. Refused."
        ),
        Never::TooLong => tr!(
            "Cette adresse est trop longue (2 048 caractères au plus).",
            "This address is too long (2,048 characters at most)."
        ),
    })
}

/// The origins of the artifacts: claude.ai's, and in a debug build the local server of an end-to-end test that asks it
/// (`ESCOUADE_E2E_ARTIFACT_ORIGIN`, an `http://127.0.0.1:<port>`), whose fake agent publishes there. Never in a
/// release build: there, claude.ai alone.
fn artifact_origins() -> Vec<String> {
    #[allow(unused_mut)]
    let mut out = vec!["https://claude.ai".to_string()];
    #[cfg(debug_assertions)]
    if let Some(o) = std::env::var("ESCOUADE_E2E_ARTIFACT_ORIGIN")
        .ok()
        .and_then(|v| Url::parse(v.trim()).ok())
        .filter(|u| u.scheme() == "http" && u.host_str() == Some("127.0.0.1"))
    {
        out.push(o.origin().ascii_serialization());
    }
    out
}

/// An artifact's address as Escouade knows it: `https://claude.ai/artifact/<id>` or
/// `https://claude.ai/code/artifact/<id>`, without its query or its fragment. None for any other.
pub(crate) fn artifact_address(text: &str) -> Option<String> {
    let u = Url::parse(text.trim()).ok()?;
    let origin = u.origin().ascii_serialization();
    if !artifact_origins().contains(&origin) || !u.username().is_empty() || u.password().is_some() {
        return None;
    }
    let parts: Vec<&str> = u.path().trim_end_matches('/').split('/').skip(1).collect();
    let id = match parts.as_slice() {
        ["artifact", id] | ["code", "artifact", id] => *id,
        _ => return None,
    };
    let fine = !id.is_empty()
        && id.len() <= 100
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    fine.then(|| format!("{origin}{}", u.path().trim_end_matches('/')))
}

/// The artifacts' addresses a text says, in their order.
fn artifacts_in_text(text: &str) -> Vec<String> {
    let starts: Vec<String> = artifact_origins()
        .into_iter()
        .map(|o| format!("{o}/"))
        .collect();
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = starts.iter().filter_map(|s| rest.find(s.as_str())).min() {
        let tail = &rest[at..];
        let end = tail
            .find(|c: char| {
                c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '`' | ')' | ']')
            })
            .unwrap_or(tail.len());
        let candidate = tail[..end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
        if let Some(a) = artifact_address(candidate) {
            if !found.contains(&a) {
                found.push(a);
            }
        }
        rest = &tail[end.max(1)..];
    }
    found
}

/// The version a publication's result says (« (Version 3, version id …) » of Claude Code's `Artifact`), if it does.
pub(crate) fn artifact_version(text: &str) -> Option<u32> {
    text.match_indices("Version ").find_map(|(at, w)| {
        let digits: String = text[at + w.len()..]
            .chars()
            .take_while(char::is_ascii_digit)
            .take(6)
            .collect();
        digits.parse().ok().filter(|n| *n > 0)
    })
}

/// The artifacts an agent published, the oldest first: the addresses in the results of its own
/// `Artifact` tool (a publication, its harness's: never what the agent wrote elsewhere, nor
/// another tool's result, nor a read of another artifact).
pub(crate) fn artifacts_of(items: &[Value]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for v in items {
        if v["kind"] != "tool" || v["name"] != "Artifact" || v["status"] != "ok" {
            continue;
        }
        if v["result"]["isError"] == true {
            continue;
        }
        let action = v["input"]["action"].as_str().unwrap_or("publish");
        // A file uploaded to its assets (`asset`, `file_paths`) is no version of the page.
        if action != "publish"
            || v["input"]["asset"] == true
            || v["input"].get("file_paths").is_some()
        {
            continue;
        }
        for a in artifacts_in_text(v["result"]["text"].as_str().unwrap_or_default()) {
            found.retain(|x| x != &a);
            found.push(a);
        }
    }
    found
}

/// The calling agent: its meta, and its project.
fn caller_agent<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
) -> Result<(AgentMeta, Project), ToolError> {
    let Caller::Agent(_) = caller else {
        return Err(ToolError::Refused(tr!(
            "Claude hors d’Escouade n’ouvre aucun navigateur : seul un agent d’Escouade, à côté de la conversation duquel l’utilisateur voit la page, en ouvre.",
            "Claude outside Escouade opens no browser: only an agent of Escouade, beside whose conversation the user sees the page, opens one."
        )));
    };
    let who = browsers::reader(core, caller, None)?;
    let id = who.agent.clone().unwrap_or_default();
    let meta = core
        .agent(&id)
        .map(|h| h.lock().meta.clone())
        .map_err(|_| {
            ToolError::Refused(tr!(
                "Escouade ne connaît plus cet agent.",
                "Escouade no longer knows this agent."
            ))
        })?;
    Ok((meta, who.project))
}

/// The page a call opens: its key, its address, its row's name, whether it is new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    pub key: String,
    pub url: String,
    pub title: String,
    /// Not open yet: its webview is made.
    pub new: bool,
}

/// What `open_browser` does: open (a plan), or answer that the user is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Step {
    Open(Plan),
    Asked { origin: String, new: bool },
}

/// A new key for a tab an agent opens: `browser:a<time><n>` (`devpage::agentless_key`).
fn new_key() -> String {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{}a{ms:x}{n:x}", devpage::BROWSER_PREFIX)
}

/// The host of `url`, for a row's name.
fn host_of(url: &Url) -> String {
    match url.port() {
        Some(p) => format!("{}:{p}", url.host_str().unwrap_or_default()),
        None => url.host_str().unwrap_or_default().to_string(),
    }
}

/// A tab `agent` opened in its project on `origin`: the one a new opening of that origin goes to.
fn tab_on(pages: &Pages, project: &str, agent: &str, origin: &str) -> Option<PageView> {
    pages.all().into_iter().find(|p| {
        p.project_id == project
            && p.opener.as_deref() == Some(agent)
            && origin_of(&p.url).as_deref() == Some(origin)
    })
}

/// The address of the launch of `meta` in `project`, and the key and name of its row: its test's
/// page when it is open; else, with a worktree, its own launch (the project's launch commands at its
/// ports, then its recipe), keyed by its id; without one, the project's launch command.
fn launch_of(
    pages: &Pages,
    meta: &AgentMeta,
    project: &Project,
) -> Option<(String, String, String)> {
    if let Some(p) = pages.get(&meta.id) {
        return Some((p.agent_id, p.url, p.title));
    }
    let lang = crate::i18n::ui();
    if let Some(base) = meta.port_base {
        let env = crate::testlaunch::port_env(Some(base));
        for run in project
            .run_commands
            .iter()
            .filter(|r| !r.url.trim().is_empty())
        {
            if let Ok(url) = crate::testlaunch::expand_url(run.url.trim(), &env, lang) {
                return Some((
                    meta.id.clone(),
                    url,
                    format!("{} · {}", meta.name, run.name),
                ));
            }
        }
        if let Some(recipe) = &meta.recipe {
            let named = recipe
                .processes
                .iter()
                .find(|p| !p.url.trim().is_empty())
                .map(|p| (p.url.trim().to_string(), p.name.clone()));
            let open = recipe.open.trim();
            let url = if open.is_empty() {
                named.as_ref().map(|(u, _)| u.clone())
            } else {
                Some(open.to_string())
            };
            if let Some(url) = url {
                let title = match named {
                    Some((_, name)) if !name.is_empty() => format!("{} · {name}", meta.name),
                    _ => meta.name.clone(),
                };
                return Some((meta.id.clone(), url, title));
            }
        }
        return None;
    }
    let prefix = format!("{}{}:", devpage::RUN_PREFIX, project.id);
    if let Some(p) = pages
        .all()
        .into_iter()
        .find(|p| p.agent_id.starts_with(&prefix))
    {
        return Some((p.agent_id, p.url, p.title));
    }
    project
        .run_commands
        .iter()
        .filter(|r| !r.url.trim().is_empty())
        .find_map(|run| {
            crate::testlaunch::expand_url(run.url.trim(), &[], lang)
                .ok()
                .map(|url| (format!("{prefix}{}", run.id), url, run.name.clone()))
        })
}

/// What `open_browser` will do, its arguments and bounds checked: nothing is opened here.
pub(crate) fn plan<R: Runtime>(
    core: &Core<R>,
    pages: &Pages,
    meta: &AgentMeta,
    project: &Project,
    args: &OpenBrowserArgs,
) -> Result<Step, ToolError> {
    let url = args.url.as_deref().map(str::trim).filter(|u| !u.is_empty());
    let artifact = args
        .artifact
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty());
    let launch = args.launch == Some(true);
    if artifact.is_some() && (url.is_some() || launch) {
        return Err(ToolError::Failed(tr!(
            "artifact s’ouvre seul : sans url ni launch.",
            "artifact opens alone: without url nor launch."
        )));
    }
    if artifact.is_none() && url.is_none() && !launch {
        return Err(ToolError::Failed(tr!(
            "Dis quoi ouvrir : url, launch: true ou artifact.",
            "Say what to open: url, launch: true or artifact."
        )));
    }
    let own = devpage::own_of(&core.app);
    let server = Some(core.mcp.status().port).filter(|p| *p != 0);
    let bounds = Launch::of(core, project);
    let wish = wish_of(args.place);
    let step = if let Some(asked) = artifact {
        let items = core
            .agent(&meta.id)
            .map(|h| h.lock().conv.items())
            .unwrap_or_default();
        let published = artifacts_of(&items);
        let found = if asked.eq_ignore_ascii_case("last") {
            published.last().cloned()
        } else {
            artifact_address(asked).filter(|a| published.contains(a))
        };
        let Some(address) = found else {
            return Err(ToolError::Failed(if published.is_empty() {
                tr!(
                    "Aucun artefact publié dans ta conversation (le résultat de ton outil Artifact). Passe url : elle sera demandée à l’utilisateur comme toute autre adresse.",
                    "No artifact published in your conversation (your Artifact tool's result). Give url: it is asked of the user like any other address."
                )
            } else {
                tr!(
                    "Ce n’est pas un artefact que tu as publié dans cette conversation : passe \"last\" ou l’adresse que ton outil Artifact a rendue.",
                    "This is not an artifact you published in this conversation: give \"last\" or the address your Artifact tool gave."
                )
            }));
        };
        let parsed = Url::parse(&address).map_err(|_| never_said(Never::NotAnAddress))?;
        if own(&parsed) {
            return Err(never_said(Never::Escouade));
        }
        let reuse = pages.all().into_iter().find(|p| {
            p.project_id == project.id
                && p.opener.as_deref() == Some(meta.id.as_str())
                && artifact_address(&p.url).as_deref() == Some(address.as_str())
        });
        Step::Open(match reuse {
            Some(p) => Plan {
                key: p.agent_id,
                url: address,
                title: p.title,
                new: false,
            },
            None => Plan {
                key: new_key(),
                title: host_of(&parsed),
                url: address,
                new: true,
            },
        })
    } else if launch {
        let Some((key, home, title)) = launch_of(pages, meta, project) else {
            return Err(ToolError::Failed(tr!(
                "Ton projet n’a pas de lancement à ouvrir : ni page de ton test, ni commande de lancement avec une adresse. Passe url.",
                "Your project has no launch to open: neither your test's page nor a launch command with an address. Give url."
            )));
        };
        let to = url.unwrap_or(&home).to_string();
        if url.is_some() && origin_of(&to) != origin_of(&home) {
            return Err(ToolError::Failed(tr!(
                "Avec launch, url est de l’origine de ton lancement ({origin}).",
                "With launch, url is of your launch's origin ({origin}).",
                origin = origin_of(&home).unwrap_or_default()
            )));
        }
        // The address of a test the agent wrote: on this machine at its declared ports alone.
        let checked = match judge(&to, &bounds, |u| own(u), server) {
            Ok(Judged::Launch(u)) => u,
            Ok(Judged::Ask(..)) => {
                return Err(ToolError::Refused(tr!(
                    "L’adresse de ton lancement n’est ni sur ce poste à un port déclaré, ni d’une commande de lancement du projet : passe-la par url, elle sera demandée à l’utilisateur.",
                    "Your launch's address is neither on this machine at a declared port nor one of the project's launch commands: give it as url, it is asked of the user."
                )))
            }
            Err(n) => return Err(never_said(n)),
        };
        Step::Open(Plan {
            new: pages.get(&key).is_none(),
            key,
            url: checked.to_string(),
            title,
        })
    } else {
        let asked = url.unwrap_or_default();
        match judge(asked, &bounds, |u| own(u), server).map_err(never_said)? {
            Judged::Launch(u) => {
                let origin = u.origin().ascii_serialization();
                Step::Open(open_tab(pages, project, meta, &u, &origin))
            }
            Judged::Ask(u, origin) => {
                // A tab of the agent where the user let this origin open: it goes there.
                let allowed = pages.all().into_iter().find(|p| {
                    p.project_id == project.id
                        && p.opener.as_deref() == Some(meta.id.as_str())
                        && pages.origin_allowed(&p.agent_id, &origin)
                });
                match allowed {
                    Some(p) => Step::Open(Plan {
                        key: p.agent_id,
                        url: u.to_string(),
                        title: p.title,
                        new: false,
                    }),
                    None => match pages.ask_open(&meta.id, &project.id, &origin, u.as_str(), wish) {
                        OpenLeave::Asked(_) => Step::Asked { origin, new: true },
                        OpenLeave::Waiting(_) => Step::Asked { origin, new: false },
                        OpenLeave::Refused => {
                            return Err(ToolError::Refused(tr!(
                                "L’utilisateur a refusé que tu ouvres {origin} : ne le redemande pas ; écris-lui si tu en as besoin.",
                                "The user refused to let you open {origin}: do not ask again; write to them if you need it."
                            )))
                        }
                        OpenLeave::Full => {
                            return Err(ToolError::Refused(tr!(
                                "{n} de tes demandes d’ouvrir une adresse attendent déjà l’utilisateur : attends ses réponses.",
                                "{n} of your requests to open an address already wait for the user: wait for their answers.",
                                n = devpage::OPEN_ASKS_MAX
                            )))
                        }
                    },
                }
            }
        }
    };
    if let Step::Open(p) = &step {
        let driven = pages
            .get(&p.key)
            .is_some_and(|v| v.driver.as_deref() == Some(meta.id.as_str()));
        if !driven && pages.driven_by(&meta.id, &p.key) >= devpage::DRIVEN_MAX {
            return Err(ToolError::Refused(tr!(
                "Tu pilotes déjà {n} navigateurs : fermes-en un par close_browser avant d’en ouvrir un autre.",
                "You already drive {n} browsers: close one by close_browser before you open another.",
                n = devpage::DRIVEN_MAX
            )));
        }
    }
    Ok(step)
}

/// A tab of the agent on `url`: the one it has on its origin, else a new one.
fn open_tab(pages: &Pages, project: &Project, meta: &AgentMeta, url: &Url, origin: &str) -> Plan {
    match tab_on(pages, &project.id, &meta.id, origin) {
        Some(p) => Plan {
            key: p.agent_id,
            url: url.to_string(),
            title: p.title,
            new: false,
        },
        None => Plan {
            key: new_key(),
            url: url.to_string(),
            title: host_of(url),
            new: true,
        },
    }
}

fn wish_of(place: Option<PlaceArg>) -> Wish {
    match place {
        Some(PlaceArg::Full) => Wish::Full,
        Some(PlaceArg::Below) => Wish::Below,
        Some(PlaceArg::Quiet) => Wish::Quiet,
        Some(PlaceArg::Beside) | None => Wish::Beside,
    }
}

fn wish_word(w: Wish) -> &'static str {
    match w {
        Wish::Full => "full",
        Wish::Beside => "beside",
        Wish::Below => "below",
        Wish::Quiet => "quiet",
    }
}

/// Where the window shows a page `agent` opens: where it wishes while the user looks at its
/// conversation, else its row waits; `quiet` shows nothing.
pub(crate) fn place_of(pages: &Pages, agent: &str, wish: Wish) -> &'static str {
    match wish {
        Wish::Quiet => "quiet",
        w if pages.looking(agent) => wish_word(w),
        _ => "waiting",
    }
}

/// What happens to the pages, to the window.
fn tell_of<R: Runtime>(core: &Arc<Core<R>>) -> devpage::Tell {
    let hub = Arc::downgrade(core);
    Arc::new(move |page| {
        if let Some(core) = hub.upgrade() {
            core.hub.emit(UiEvent::DevPage { page });
        }
    })
}

/// Tells the window `phase` of the page `key` (`closed`, `undriven`, `opens`).
fn tell_page<R: Runtime>(core: &Core<R>, key: &str, phase: &'static str) {
    core.hub.emit(UiEvent::DevPage {
        page: PageEvent {
            agent_id: key.to_string(),
            phase,
            url: String::new(),
            title: String::new(),
        },
    });
}

/// Opens `plan` for `agent` (its webview, hidden until the window places it), drives it, and tells
/// the window where it goes. The page as it is now.
async fn perform<R: Runtime>(
    core: &Arc<Core<R>>,
    pages: &Arc<Pages>,
    agent: &str,
    project_id: &str,
    plan: &Plan,
    wish: Wish,
) -> Result<(PageView, &'static str), ToolError> {
    let (app, pages2, plan2, project) = (
        core.app.clone(),
        pages.clone(),
        plan.clone(),
        project_id.to_string(),
    );
    let own = devpage::own_of(&core.app);
    let local = super::browsers::local_of(core, project_id);
    let tell = tell_of(core);
    let root = core.data.root().to_path_buf();
    tokio::task::spawn_blocking(move || {
        devpage::open(
            &app,
            &pages2,
            devpage::Open {
                agent_id: &plan2.key,
                project_id: &project,
                title: &plan2.title,
                url: &plan2.url,
                bounds: None,
                data_root: &root,
                visible: false,
                local,
            },
            own,
            tell,
        )
    })
    .await
    .map_err(|e| ToolError::Failed(e.to_string()))?
    .map_err(|e| {
        ToolError::Failed(tr!(
            "Le navigateur ne s’est pas ouvert : {e}",
            "The browser did not open: {e}"
        ))
    })?;
    if plan.new && plan.key.starts_with(devpage::BROWSER_PREFIX) {
        pages.set_opener(&plan.key, agent);
    }
    pages.set_driver(&plan.key, Some(agent));
    let place = place_of(pages, agent, wish);
    let view = pages.get(&plan.key).ok_or_else(|| {
        ToolError::Failed(tr!(
            "Ce navigateur vient de se fermer.",
            "This browser just closed."
        ))
    })?;
    core.hub.emit(UiEvent::BrowserOpened {
        browser: Opened {
            view: view.clone(),
            agent: agent.to_string(),
            wish,
            place,
        },
    });
    Ok((view, place))
}

/// What the tool answers once a page is opened.
fn opened_answer(key: &str, url: &str, place: &str) -> Value {
    let mut v = json!({ "browser": key, "url": url, "place": place });
    if place == "waiting" {
        v["why"] = json!(tr!(
            "L’utilisateur ne regarde pas ta conversation : la ligne attend dans « Navigateurs » et s’ouvrira à côté quand il y reviendra. Dis-lui ce que tu veux lui montrer.",
            "The user is not looking at your conversation: the row waits in « Navigateurs » and opens beside it when they come back. Tell them what you want to show."
        ));
    }
    v
}

/// What `open_browser` answers while the user has closed the agent's pane and not come back to its conversation
/// (AB-9): nothing opens, nothing waits.
pub(crate) fn dismissed_answer() -> Value {
    json!({
        "place": "closed",
        "why": tr!(
            "L’utilisateur a fermé ton volet : rien ne s’ouvre sur son écran tant qu’il n’a pas quitté ta conversation puis n’y est pas revenu. Rien n’est ouvert ni en attente ; dis-lui ce que tu voulais lui montrer, ou redemande quand il sera revenu.",
            "The user closed your pane: nothing opens on their screen until they have left your conversation and come back to it. Nothing is open nor waiting; tell them what you wanted to show, or ask again once they are back."
        ),
    })
}

/// `open_browser`.
pub(super) async fn open<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    args: OpenBrowserArgs,
) -> Result<String, ToolError> {
    browsers::switched_on(core)?;
    let (meta, project) = caller_agent(core, caller)?;
    let Some(pages) = browsers::pages_of(core) else {
        return Err(ToolError::Failed(tr!(
            "La fenêtre d’Escouade n’a pas de navigateurs ici.",
            "Escouade's window has no browsers here."
        )));
    };
    if !pages.may_open(&meta.id) {
        return Err(ToolError::Refused(tr!(
            "Tu as ouvert {n} navigateurs dans l’heure : c’est la borne. Réutilise ceux qui sont ouverts (list_browsers).",
            "You opened {n} browsers in the hour: that is the bound. Use the ones open (list_browsers).",
            n = devpage::OPENS_PER_HOUR
        )));
    }
    let wish = wish_of(args.place);
    if wish != Wish::Quiet && pages.dismissed(&meta.id) {
        return Ok(dismissed_answer().to_string());
    }
    match plan(core, &pages, &meta, &project, &args)? {
        Step::Asked { origin, new } => {
            if new {
                pages.note_open(&meta.id);
                tell_page(core, &meta.id, "opens");
            }
            let host = origin.split("://").nth(1).unwrap_or(&origin).to_string();
            Ok(json!({
                "place": "asked",
                "origin": origin,
                "message": tr!(
                    "{host} n’est pas sur le lancement du projet : la conversation demande à l’utilisateur de te permettre de l’ouvrir (une fois). Rien n’est ouvert ; son accord (avec la clé du navigateur) ou son refus te reviendra par un message. Attends-le sans redemander.",
                    "{host} is not on the project's launch: the conversation asks the user to let you open it (once). Nothing is open; their leave (with the browser's key) or their refusal comes back to you by a message. Wait for it without asking again."
                ),
            })
            .to_string())
        }
        Step::Open(p) => {
            let (view, place) = perform(core, &pages, &meta.id, &project.id, &p, wish).await?;
            pages.note_open(&meta.id);
            Ok(opened_answer(&view.agent_id, &view.url, place).to_string())
        }
    }
}

/// The user's answer in the band of `ask.agent`'s conversation to its request `id` to open an
/// address (« Permettre une fois » or « Refuser »): opened, then told to the agent by a message.
pub(crate) async fn answer<R: Runtime>(
    core: &Arc<Core<R>>,
    pages: &Arc<Pages>,
    id: &str,
    allow: bool,
) -> Result<(), String> {
    let Some(ask) = pages.answer_open(id, allow) else {
        return Ok(());
    };
    tell_page(core, &ask.agent, "opens");
    let host = ask
        .origin
        .split("://")
        .nth(1)
        .unwrap_or(&ask.origin)
        .to_string();
    let text = if !allow {
        tr!(
            "Escouade : l’utilisateur a refusé que tu ouvres {host}. Ne le redemande pas ; écris-lui si tu en as besoin.",
            "Escouade: the user refused to let you open {host}. Do not ask again; write to them if you need it."
        )
    } else {
        let opened = async {
            // Checked again: the switch, the agent, its project, the address and the bounds.
            if !core.settings.read().agent_browsers {
                return Err(tr!(
                    "« Agents et navigateurs » est coupé dans Réglages › Agents.",
                    "“Agents and browsers” is off in Settings › Agents."
                ));
            }
            let meta = core
                .agent(&ask.agent)
                .map(|h| h.lock().meta.clone())
                .map_err(|e| e.to_string())?;
            let project = core.project(&meta.project_id).map_err(|e| e.to_string())?;
            if project.id != ask.project_id {
                return Err("not the project of the request".into());
            }
            let own = devpage::own_of(&core.app);
            let server = Some(core.mcp.status().port).filter(|p| *p != 0);
            let url = match judge(&ask.url, &Launch::of(core, &project), |u| own(u), server) {
                Ok(Judged::Launch(u)) | Ok(Judged::Ask(u, _)) => u,
                Err(_) => return Err("refused address".into()),
            };
            let plan = open_tab(pages, &project, &meta, &url, &ask.origin);
            let driven = pages
                .get(&plan.key)
                .is_some_and(|v| v.driver.as_deref() == Some(meta.id.as_str()));
            if !driven && pages.driven_by(&meta.id, &plan.key) >= devpage::DRIVEN_MAX {
                return Err(tr!(
                    "tu pilotes déjà {n} navigateurs",
                    "you already drive {n} browsers",
                    n = devpage::DRIVEN_MAX
                ));
            }
            let (view, place) = perform(core, pages, &meta.id, &project.id, &plan, ask.wish)
                .await
                .map_err(|e| match e {
                    ToolError::Refused(t) | ToolError::Failed(t) => t,
                })?;
            pages.allow_origin(&view.agent_id, &ask.origin);
            Ok((view, place))
        }
        .await;
        match opened {
            Ok((view, place)) => {
                let key = view.agent_id.clone();
                let shown = opened_answer(&key, &view.url, place);
                tr!(
                    "Escouade : l’utilisateur te permet d’ouvrir {host} : il est ouvert dans le navigateur {key} ({shown}).",
                    "Escouade: the user lets you open {host}: it is open in the browser {key} ({shown})."
                )
            }
            Err(e) => tr!(
                "Escouade : l’utilisateur te permet d’ouvrir {host}, mais le navigateur ne s’est pas ouvert : {e}",
                "Escouade: the user lets you open {host}, but the browser did not open: {e}"
            ),
        }
    };
    if let Err(e) = core.send_by_app(&ask.agent, text).await {
        log::warn!(
            "browsers: the answer to {}'s request to open not sent: {e:#}",
            ask.agent
        );
    }
    Ok(())
}

/// `close_browser`: a tab the agent opened is closed; on the page of its launch (or one it drives)
/// its driving stops and the row stays. Never one the user opened, nor another agent's.
pub(super) async fn close<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    args: CloseBrowserArgs,
) -> Result<String, ToolError> {
    browsers::switched_on(core)?;
    let (meta, _) = caller_agent(core, caller)?;
    let asked = args.browser.trim();
    if asked.starts_with("page:") {
        return Err(ToolError::Refused(tr!(
            "Une page d’agent se ferme par close_page.",
            "An agent's page is closed by close_page."
        )));
    }
    let who = browsers::reader(core, caller, None)?;
    let pages = browsers::pages_of(core).unwrap_or_default();
    let rows = browsers::rows_of(&pages, &who);
    let Some(page) = rows.into_iter().find(|p| p.agent_id == asked) else {
        return Err(ToolError::Failed(tr!(
            "Aucun navigateur « {asked} » dans ton projet (list_browsers).",
            "No browser “{asked}” in your project (list_browsers)."
        )));
    };
    let me = meta.id.as_str();
    if page.opener.as_deref() == Some(me) {
        devpage::close(&core.app, &pages, &page.agent_id).map_err(ToolError::Failed)?;
        tell_page(core, &page.agent_id, "closed");
        return Ok(json!({ "browser": page.agent_id, "closed": true }).to_string());
    }
    if page.driver.as_deref() == Some(me) || page.agent_id == me {
        pages.set_driver(&page.agent_id, None);
        pages.drives.forget(&page.agent_id);
        tell_page(core, &page.agent_id, "undriven");
        return Ok(json!({
            "browser": page.agent_id,
            "closed": false,
            "message": if page.agent_id.starts_with(devpage::BROWSER_PREFIX) {
                tr!(
                    "Tu ne pilotes plus ce navigateur ; sa ligne reste : l’utilisateur l’a ouvert lui-même.",
                    "You no longer drive this browser; its row stays: the user opened it themselves."
                )
            } else {
                tr!(
                    "Tu ne pilotes plus ce navigateur ; sa ligne reste : c’est celle d’un lancement.",
                    "You no longer drive this browser; its row stays: it is a launch's."
                )
            },
        })
        .to_string());
    }
    if page.agent_id.starts_with(devpage::BROWSER_PREFIX) && page.opener.is_none() {
        return Err(ToolError::Refused(tr!(
            "L’utilisateur a ouvert ce navigateur : tu ne le fermes pas.",
            "The user opened this browser: you do not close it."
        )));
    }
    Err(ToolError::Refused(tr!(
        "Ce navigateur n’est pas le tien : tu ne fermes que ceux que tu as ouverts.",
        "This browser is not yours: you close only the ones you opened."
    )))
}

/// What `devpage_open_asks` gives the window: the requests of agents it still knows.
pub(crate) fn open_asks<R: Runtime>(core: &Core<R>, pages: &Pages) -> Vec<devpage::OpenAsk> {
    let asks = pages.open_asks();
    let gone: Vec<String> = asks
        .iter()
        .filter(|a| core.agent(&a.agent).is_err())
        .map(|a| a.agent.clone())
        .collect();
    for a in &gone {
        pages.forget_open_asks(a);
    }
    pages.open_asks()
}
