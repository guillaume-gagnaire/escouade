//! `drive_browser` (ESC-679, AB-7 with AP-6; docs/AGENT-BROWSERS.md § 3.3, § 9): one tool drives every row of
//! « Navigateurs », the tabs and the agents' pages, one gesture per call, on the rules of `browserdrive`.
//!
//! Who drives what: an agent of Escouade, a row on screen alone; on a tab, the page of its test, a tab it opened or a
//! launch it drives (`open_browser`), never another agent's nor one the user opened alone; on an agent's page, its own.
//! Claude outside Escouade drives nothing. Off with « Agents et navigateurs » for the tabs, with « Pages des agents »
//! for the pages.
//!
//! A gesture: the tiers (`browserdrive::decide`), the hand (the user's gestures taken first), the pace, then the
//! element found in Escouade's world and what is never done (`browserdrive::forbid`), then the gesture, the page let
//! settle, the band told (`phase: "drive"`). `look` joins a capture; `wait_user` waits for « Continuer ».

use super::browsers::{self, marked, origin_of, pages_of, Launch};
use super::tools::ToolError;
use super::Caller;
use crate::browserdrive::{
    self as drive, Act, BadTarget, Forbidden, Hand, Line, May, Refusal, Target, Verdict, WaitEnd,
};
use crate::core::Core;
use crate::devpage::{self, PageMode, Pages, Placement};
use crate::model::UiEvent;
use rmcp::model::ContentBlock;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Manager, Runtime, Url};

pub use escouade_mcp::args::{
    DriveActionArg, DriveBrowserArgs, DriveWidthArg, PressKeyArg, WaitForArg,
};

/// The row a call drives, as the core found it.
struct Row {
    key: String,
    label: String,
    line: Line,
    agent: String,
    /// The project's tint, for the highlight.
    color: String,
    /// An agent's page: what it knows of it.
    page: Option<crate::pages::DriveInfo>,
    /// A tab: its address now.
    url: String,
}

fn refused(text: String) -> ToolError {
    ToolError::Refused(text)
}

fn not_shown() -> ToolError {
    refused(tr!(
        "Le navigateur n’est pas affiché : l’utilisateur ne le voit pas. Il se pilote quand il est à l’écran (open_browser le montre à côté de ta conversation).",
        "The browser is not shown: the user does not see it. It is driven while on screen (open_browser shows it beside your conversation)."
    ))
}

/// The row `asked` for the agent `agent`, or why it may not drive it.
fn row_of<R: Runtime>(core: &Core<R>, caller: &Caller, asked: &str) -> Result<Row, ToolError> {
    let Caller::Agent(agent) = caller else {
        return Err(refused(tr!(
            "Claude hors d’Escouade ne pilote aucun navigateur : seul un agent d’Escouade, à côté de la conversation duquel l’utilisateur voit la page, les pilote.",
            "Claude outside Escouade drives no browser: only an agent of Escouade, beside whose conversation the user sees the page, drives them."
        )));
    };
    let asked = asked.trim();
    if asked.starts_with(crate::pages::KEY_PREFIX) {
        if !core.pages_on() {
            return Err(refused(crate::pages::switched_off(crate::i18n::ui())));
        }
        let Some(page) = crate::pages::of_key(asked) else {
            return Err(ToolError::Failed(tr!(
                "« {asked} » n’est pas la clé d’une page : page:p-<n>.",
                "“{asked}” is not a page's key: page:p-<n>."
            )));
        };
        let info = core
            .page_drive_info(agent, page)
            .map_err(ToolError::Failed)?;
        if info.row.is_none_or(|p| p == Placement::Hidden) {
            return Err(refused(tr!(
                "La page {page} n’est pas à l’écran : elle se pilote quand l’utilisateur la voit.",
                "Page {page} is not on screen: it is driven while the user sees it."
            )));
        }
        let color = core
            .agent(agent)
            .ok()
            .and_then(|h| core.project(&h.lock().meta.project_id).ok())
            .map(|p| p.color)
            .unwrap_or_default();
        return Ok(Row {
            key: asked.to_string(),
            label: crate::agentpage::label(page),
            line: Line::AgentPage,
            agent: agent.clone(),
            color,
            page: Some(info),
            url: crate::agentpage::url(page).to_string(),
        });
    }
    browsers::switched_on(core)?;
    let who = browsers::reader(core, caller, None)?;
    let pages = pages_of(core).unwrap_or_default();
    let rows = browsers::rows_of(&pages, &who);
    let Some(p) = rows.iter().find(|p| p.agent_id == asked) else {
        let keys = rows
            .iter()
            .take(super::resolve::CHOICES)
            .map(|p| p.agent_id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ToolError::Failed(if keys.is_empty() {
            tr!(
                "Aucun navigateur « {asked} » : ton projet n’en a aucun d’ouvert (list_browsers).",
                "No browser “{asked}”: your project has none open (list_browsers)."
            )
        } else {
            tr!(
                "Aucun navigateur « {asked} » dans ton projet. Navigateurs : {keys}",
                "No browser “{asked}” in your project. Browsers: {keys}"
            )
        }));
    };
    if !drivable(p, agent) {
        return Err(refused(
            if p.opener.is_none() && p.agent_id.starts_with(devpage::BROWSER_PREFIX) {
                tr!(
                "L’utilisateur a ouvert ce navigateur : tu ne le pilotes pas, à moins qu’il ne te le laisse (« Laisser piloter par… » dans le menu de sa ligne). Ouvre le tien par open_browser.",
                "The user opened this browser: you do not drive it unless they let you (« Laisser piloter par… » in its row's menu). Open your own by open_browser."
            )
            } else {
                tr!(
                "Ce navigateur n’est pas le tien : tu pilotes la page de ton test, les navigateurs que tu as ouverts (open_browser) et tes pages.",
                "This browser is not yours: you drive the page of your test, the browsers you opened (open_browser) and your pages."
            )
            },
        ));
    }
    if p.placement == Placement::Hidden {
        return Err(not_shown());
    }
    let launch = Launch::of(core, &who.project);
    let line = if launch.holds(&p.url) {
        Line::Launch
    } else {
        match origin_of(&p.url) {
            Some(o) => Line::Outside(o),
            None => {
                return Err(refused(tr!(
                    "Ce navigateur n’est pas sur une adresse http ou https : il ne se pilote pas.",
                    "This browser is not on an http or https address: it is not driven."
                )))
            }
        }
    };
    Ok(Row {
        key: p.agent_id.clone(),
        label: devpage::page_label(&p.agent_id),
        line,
        agent: agent.clone(),
        color: who.project.color.clone(),
        page: None,
        url: p.url.clone(),
    })
}

/// Whether `agent` drives the tab `p`: the page of its test, one it opened, a launch it drives.
pub(super) fn drivable(p: &devpage::PageView, agent: &str) -> bool {
    p.agent_id == agent || p.opener.as_deref() == Some(agent) || p.driver.as_deref() == Some(agent)
}

/// What a target that is not taken says.
fn bad_target(raw: &str, b: BadTarget) -> ToolError {
    ToolError::Failed(match b {
        BadTarget::Empty => tr!(
            "Donne l’élément (target) : #id, un sélecteur CSS, text=… ou une clé du modèle de ta page.",
            "Give the element (target): #id, a CSS selector, text=… or a key of your page's template."
        ),
        BadTarget::TooLong => tr!(
            "Cette cible fait plus de 200 caractères.",
            "This target is longer than 200 characters."
        ),
        BadTarget::Selector => tr!(
            "Ce sélecteur n’est pas permis : 200 caractères au plus, ni :has() ni plus de quatre niveaux, refermé.",
            "This selector is not allowed: 200 characters at most, neither :has() nor more than four levels, closed."
        ),
        BadTarget::Key(keys) => tr!(
            "Aucune partie « {raw} » dans cette page. Ses clés : {keys}",
            "No part “{raw}” in this page. Its keys: {keys}"
        ),
    })
}

/// The gesture the arguments ask for, checked.
fn act_of(args: &DriveBrowserArgs, row: &Row) -> Result<Act, ToolError> {
    let keys = row.page.as_ref().map(|info| move |k: &str| info.anchor(k));
    let target = |need: bool| -> Result<Option<Target>, ToolError> {
        match args
            .target
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            None if need => Err(bad_target("", BadTarget::Empty)),
            None => Ok(None),
            Some(raw) => {
                let k = keys.as_ref().map(|k| k as drive::Keys<'_>);
                drive::target(raw, k)
                    .map(Some)
                    .map_err(|b| bad_target(raw, b))
            }
        }
    };
    let need = |t: Result<Option<Target>, ToolError>| t.map(|t| t.expect("a target"));
    // `frame` (ESC-682, AB-11): the <iframe> by a selector, for scroll_to and highlight alone.
    if let Some(raw) = args
        .frame
        .as_deref()
        .map(str::trim)
        .filter(|f| !f.is_empty())
    {
        let gesture = match args.action {
            DriveActionArg::ScrollTo => drive::FrameGesture::ScrollTo,
            DriveActionArg::Highlight => drive::FrameGesture::Highlight,
            _ => {
                return Err(ToolError::Failed(tr!(
                    "frame vaut pour scroll_to et highlight seuls.",
                    "frame is for scroll_to and highlight alone."
                )))
            }
        };
        let frame = match drive::target(raw, None) {
            Ok(t @ Target::Css(_)) => t,
            Ok(Target::Text(_)) => {
                return Err(ToolError::Failed(tr!(
                    "frame prend le cadre par un sélecteur (#id, iframe[title=…]), pas par text=.",
                    "frame takes the frame by a selector (#id, iframe[title=…]), not by text=."
                )))
            }
            Err(b) => return Err(bad_target(raw, b)),
        };
        let inner = match args
            .target
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            None => None,
            Some(t) => Some(drive::target(t, None).map_err(|b| bad_target(t, b))?),
        };
        let label = args.label.as_deref().unwrap_or("").trim().to_string();
        if label.chars().count() > drive::LABEL_MAX || label.chars().any(char::is_control) {
            return Err(ToolError::Failed(tr!(
                "label tient sur une ligne de 60 caractères au plus.",
                "label is one line of 60 characters at most."
            )));
        }
        return Ok(Act::InFrame {
            frame,
            gesture,
            target: inner,
            label,
        });
    }
    Ok(match args.action {
        DriveActionArg::Navigate => {
            let Some(url) = args.url.as_deref().map(str::trim).filter(|u| !u.is_empty()) else {
                return Err(ToolError::Failed(tr!(
                    "navigate prend une adresse (url).",
                    "navigate takes an address (url)."
                )));
            };
            Act::Navigate(url.to_string())
        }
        DriveActionArg::Back => Act::Back,
        DriveActionArg::Forward => Act::Forward,
        DriveActionArg::Reload => Act::Reload,
        DriveActionArg::Wait => {
            let secs = args.timeout.unwrap_or(10);
            if !(1..=drive::WAIT_MAX.as_secs()).contains(&secs) {
                return Err(ToolError::Failed(tr!(
                    "timeout va de 1 à 30 secondes (10 par défaut).",
                    "timeout goes from 1 to 30 seconds (10 by default)."
                )));
            }
            let t = match args.wait_for {
                Some(WaitForArg::Target) => Some(need(target(true))?),
                _ => None,
            };
            Act::Wait(t, Duration::from_secs(secs))
        }
        DriveActionArg::ScrollTo => Act::ScrollTo(need(target(true))?),
        DriveActionArg::Scroll => {
            let by = args.by.unwrap_or(0);
            if by == 0 || by.abs() > drive::SCROLL_MAX {
                return Err(ToolError::Failed(tr!(
                    "scroll prend by, de -5 000 à 5 000 pixels (pas 0).",
                    "scroll takes by, from -5,000 to 5,000 pixels (not 0)."
                )));
            }
            Act::Scroll(by)
        }
        DriveActionArg::Highlight => {
            let t = need(target(true))?;
            let label = args.label.as_deref().unwrap_or("").trim().to_string();
            if label.chars().count() > drive::LABEL_MAX || label.chars().any(char::is_control) {
                return Err(ToolError::Failed(tr!(
                    "label tient sur une ligne de 60 caractères au plus.",
                    "label is one line of 60 characters at most."
                )));
            }
            Act::Highlight(t, label)
        }
        DriveActionArg::Clear => Act::Clear,
        DriveActionArg::Width => Act::Width(match args.width {
            Some(DriveWidthArg::Full) => PageMode::Full,
            Some(DriveWidthArg::Tablet) => PageMode::Tablet,
            Some(DriveWidthArg::Mobile) => PageMode::Mobile,
            None => {
                return Err(ToolError::Failed(tr!(
                    "width prend width : full, tablet ou mobile.",
                    "width takes width: full, tablet or mobile."
                )))
            }
        }),
        DriveActionArg::Tab => Act::Tab(need(target(true))?),
        DriveActionArg::Click => Act::Click(need(target(true))?),
        DriveActionArg::Type => {
            let t = need(target(true))?;
            let text = args.text.clone().unwrap_or_default();
            if text.chars().count() > drive::TEXT_MAX {
                return Err(ToolError::Failed(tr!(
                    "text fait 2 000 caractères au plus.",
                    "text is 2,000 characters at most."
                )));
            }
            Act::Type {
                target: t,
                text,
                submit: args.submit.unwrap_or(false),
            }
        }
        DriveActionArg::Select => {
            let t = need(target(true))?;
            let Some(v) = args.value.as_deref().filter(|v| !v.is_empty()) else {
                return Err(ToolError::Failed(tr!(
                    "select prend value : la valeur de l’option, ou son libellé.",
                    "select takes value: the option's value, or its label."
                )));
            };
            Act::Select(t, v.chars().take(drive::TARGET_MAX).collect())
        }
        DriveActionArg::Press => {
            let Some(k) = args.key else {
                return Err(ToolError::Failed(tr!(
                    "press prend key : Enter, Escape, Tab, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, PageUp, PageDown, Home ou End.",
                    "press takes key: Enter, Escape, Tab, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, PageUp, PageDown, Home or End."
                )));
            };
            Act::Press(press_key(k))
        }
        DriveActionArg::Version => {
            let Some(n) = args.n.filter(|n| *n > 0) else {
                return Err(ToolError::Failed(tr!(
                    "version prend n : le numéro d’une version de ta page (get_page les liste).",
                    "version takes n: the number of a version of your page (get_page lists them)."
                )));
            };
            Act::Version(n)
        }
    })
}

pub(super) fn press_key(k: PressKeyArg) -> drive::Key {
    use drive::Key;
    match k {
        PressKeyArg::Enter => Key::Enter,
        PressKeyArg::Escape => Key::Escape,
        PressKeyArg::Tab => Key::Tab,
        PressKeyArg::ArrowUp => Key::ArrowUp,
        PressKeyArg::ArrowDown => Key::ArrowDown,
        PressKeyArg::ArrowLeft => Key::ArrowLeft,
        PressKeyArg::ArrowRight => Key::ArrowRight,
        PressKeyArg::PageUp => Key::PageUp,
        PressKeyArg::PageDown => Key::PageDown,
        PressKeyArg::Home => Key::Home,
        PressKeyArg::End => Key::End,
    }
}

/// What a refusal of the tiers says.
fn tier_refused(r: Refusal) -> ToolError {
    refused(match r {
        Refusal::ActOnPage => tr!(
            "Sur une page d’agent, seuls les gestes de montrer, tab et version sont permis : rien ne s’y clique, ne s’y tape ni ne s’y choisit.",
            "On an agent's page, only the gestures that show, tab and version are allowed: nothing is clicked, typed or chosen there."
        ),
        Refusal::TabOnly => tr!(
            "Une page d’agent n’a ni historique ni adresse : montre une autre version par version.",
            "An agent's page has neither history nor address: show another version by version."
        ),
        Refusal::PageOnly => tr!(
            "version vaut pour une page d’agent (page:p-<n>), pas pour un onglet.",
            "version is for an agent's page (page:p-<n>), not for a tab."
        ),
    })
}

/// What a refusal of what is never done says.
fn forbidden(f: Forbidden) -> ToolError {
    refused(match f {
        Forbidden::File => tr!(
            "Envoyer un fichier est refusé, même avec l’accord de l’utilisateur : c’est à lui de le faire.",
            "Sending a file is refused, even with the user's leave: it is theirs to do."
        ),
        Forbidden::Download => tr!(
            "Ce lien télécharge un fichier : refusé, même avec l’accord de l’utilisateur.",
            "This link downloads a file: refused, even with the user's leave."
        ),
        Forbidden::CrossForm => tr!(
            "Ce champ est d’un formulaire qui s’envoie vers une autre origine : refusé, même avec l’accord de l’utilisateur.",
            "This field is of a form that is sent to another origin: refused, even with the user's leave."
        ),
        Forbidden::PasswordOutside => tr!(
            "Taper un mot de passe hors du lancement du projet est refusé, même avec l’accord de l’utilisateur : il s’y connecte lui-même.",
            "Typing a password outside the project's launch is refused, even with the user's leave: they sign in themselves."
        ),
        Forbidden::PasswordAsk => tr!(
            "Ce champ est un mot de passe : l’utilisateur doit te permettre de le remplir dans ce navigateur (une fois). La bande le lui demande ; sa réponse te reviendra par un message. Attends-la sans redemander.",
            "This field is a password: the user must let you fill it in this browser (once). The band asks them; their answer comes back to you by a message. Wait for it without asking again."
        ),
        Forbidden::Secret => tr!(
            "Ce texte contient un secret d’Escouade (ou ce qui en a la forme : une clé, un jeton) : il ne se tape jamais dans une page.",
            "This text holds a secret of Escouade (or what has its form: a key, a token): it is never typed in a page."
        ),
        Forbidden::NotText => tr!(
            "Cet élément n’est pas un champ de texte : type ne tape que dans un champ de texte.",
            "This element is not a text field: type types in a text field alone."
        ),
        Forbidden::NotSelect => tr!(
            "Cet élément n’est pas une liste (<select>) : select ne choisit que dans une liste.",
            "This element is not a list (<select>): select chooses in a list alone."
        ),
        Forbidden::NotTab => tr!(
            "Cet élément n’est pas un onglet déclaré (role=\"tab\") : tab ne clique que ceux-là ; click pour le reste, sur un onglet du lancement.",
            "This element is not a declared tab (role=\"tab\"): tab clicks those alone; click for the rest, on a tab of the launch."
        ),
    })
}

/// The secrets of Escouade an agent never types: the tokens of its MCP server, of the ticket systems' and code
/// hosts' accounts, the proxy's password.
pub(super) fn secrets_of<R: Runtime>(core: &Core<R>) -> Vec<String> {
    let mut out = core.mcp.token_values();
    let a = core.accounts.read();
    for acc in [&a.jira, &a.trello, &a.github, &a.linear, &a.clickup]
        .into_iter()
        .flatten()
    {
        out.push(acc.token.clone());
        out.push(acc.key.clone());
    }
    out.extend(a.forges.iter().map(|f| f.token.clone()));
    drop(a);
    let s = core.settings.read();
    out.push(s.proxy_password_in_file.clone());
    out.retain(|s| !s.trim().is_empty());
    out
}

/// The window reads the driving of the row `key` again (AB-8 draws it).
pub(crate) fn tell<R: Runtime>(core: &Core<R>, key: &str) {
    core.hub.emit(UiEvent::DevPage {
        page: devpage::PageEvent {
            agent_id: key.to_string(),
            phase: "drive",
            url: String::new(),
            title: String::new(),
        },
    });
}

/// The window shows the tab `key` at `mode`, for good (the bar follows).
fn tell_mode<R: Runtime>(core: &Core<R>, key: &str, mode: PageMode) {
    let url = match mode {
        PageMode::Full => "full",
        PageMode::Tablet => "tablet",
        PageMode::Mobile => "mobile",
    };
    core.hub.emit(UiEvent::DevPage {
        page: devpage::PageEvent {
            agent_id: key.to_string(),
            phase: "mode",
            url: url.to_string(),
            title: String::new(),
        },
    });
}

/// What the gesture is, as the band and the conversation say it.
fn what_of(act: &Act, raw: Option<&str>) -> String {
    let t: String = raw
        .unwrap_or("")
        .trim()
        .chars()
        .take(drive::LABEL_MAX)
        .collect();
    match act {
        Act::Navigate(u) => {
            let u: String = u.chars().take(80).collect();
            tr!("Va à {u}", "Goes to {u}")
        }
        Act::Back => tr!("Revient en arrière", "Goes back"),
        Act::Forward => tr!("Avance", "Goes forward"),
        Act::Reload => tr!("Recharge", "Reloads"),
        Act::Wait(None, _) => tr!("Attend le chargement", "Waits for the load"),
        Act::Wait(Some(_), _) => tr!("Attend « {t} »", "Waits for “{t}”"),
        Act::ScrollTo(_) => tr!("Défile jusqu’à « {t} »", "Scrolls to “{t}”"),
        Act::Scroll(by) => tr!("Défile de {by} px", "Scrolls by {by} px"),
        Act::Highlight(_, label) if !label.is_empty() => {
            tr!("Met en évidence « {label} »", "Highlights “{label}”")
        }
        Act::Highlight(..) => tr!("Met en évidence « {t} »", "Highlights “{t}”"),
        Act::Clear => tr!("Retire la mise en évidence", "Clears the highlight"),
        Act::Width(m) => {
            let w = match m {
                PageMode::Full => tr!("pleine", "full"),
                PageMode::Tablet => tr!("tablette", "tablet"),
                PageMode::Mobile => tr!("mobile", "mobile"),
            };
            tr!("Passe en largeur {w}", "Goes to the {w} width")
        }
        Act::Tab(_) => tr!("Ouvre l’onglet « {t} »", "Opens the tab “{t}”"),
        Act::Click(_) => tr!("Clique « {t} »", "Clicks “{t}”"),
        // Never the text: it may be a password.
        Act::Type { .. } => tr!("Tape dans « {t} »", "Types in “{t}”"),
        Act::Select(..) => tr!("Choisit dans « {t} »", "Chooses in “{t}”"),
        Act::Press(k) => {
            let k = k.codes().0;
            tr!("Presse {k}", "Presses {k}")
        }
        Act::Version(n) => tr!("Montre la version {n}", "Shows version {n}"),
        Act::InFrame {
            gesture: drive::FrameGesture::Highlight,
            label,
            ..
        } if !label.is_empty() => {
            tr!("Met en évidence « {label} »", "Highlights “{label}”")
        }
        Act::InFrame {
            gesture: drive::FrameGesture::Highlight,
            ..
        } => tr!("Met en évidence « {t} »", "Highlights “{t}”"),
        Act::InFrame {
            gesture: drive::FrameGesture::ScrollTo,
            ..
        } => tr!("Défile jusqu’à « {t} »", "Scrolls to “{t}”"),
    }
}

/// What failed in the page, in the agent's words.
fn page_failed(e: &str, raw: Option<&str>) -> ToolError {
    let t = raw.unwrap_or("").trim();
    match e {
        "selector" => ToolError::Failed(tr!(
            "La page ne comprend pas ce sélecteur : « {t} ».",
            "The page does not understand this selector: “{t}”."
        )),
        "missing" => ToolError::Failed(tr!(
            "Aucun élément visible pour « {t} ».",
            "No visible element for “{t}”."
        )),
        "missing-frame" => ToolError::Failed(tr!(
            "Aucun cadre visible pour frame.",
            "No visible frame for frame."
        )),
        "not-frame" => ToolError::Failed(tr!(
            "frame ne désigne pas un <iframe>.",
            "frame does not name an <iframe>."
        )),
        "option" => ToolError::Failed(tr!(
            "Aucune option de cette valeur ni de ce libellé dans « {t} ».",
            "No option of this value nor of this label in “{t}”."
        )),
        "webkit-width" => ToolError::Failed(tr!(
            "La largeur d’une page d’agent ne change pas encore sous macOS et Linux.",
            "An agent's page's width does not change yet under macOS and Linux."
        )),
        "gone" => ToolError::Failed(tr!(
            "Ce navigateur vient de se fermer.",
            "This browser just closed."
        )),
        _ => ToolError::Failed(tr!(
            "Le geste n’a pas été fait : {e}",
            "The gesture was not made: {e}"
        )),
    }
}

/// The user's gestures since the last call, taken before this one: the hand is theirs then.
fn user_moved<R: Runtime>(
    core: &Core<R>,
    pages: &Pages,
    key: &str,
    webview: &tauri::Webview<R>,
) -> bool {
    let n = drive::gestures(webview);
    // What the user did while the hand was theirs is not a new gesture.
    if pages.drives.take_handed_back(key) {
        return false;
    }
    if n > 0 && pages.drives.user_took(key) {
        tell(core, key);
        return true;
    }
    false
}

fn hand_refused() -> ToolError {
    refused(tr!(
        "L’utilisateur a repris la main : il te la rendra par « La rendre », ou écris-lui.",
        "The user took the hand back: they will give it back by “La rendre” (Give it back), or write to them."
    ))
}

fn stopped_refused() -> ToolError {
    refused(tr!(
        "L’utilisateur ne laisse plus piloter ce navigateur, jusqu’à sa fermeture.",
        "The user no longer lets this browser be driven, until it closes."
    ))
}

/// Polls the user's gestures of the row `key` every 500 ms while the agent has the hand (§ 9): the first one gives
/// the hand to the user, ends a `wait_user`, and tells the band.
fn keep_polling<R: Runtime>(core: &Arc<Core<R>>, pages: Arc<Pages>, key: String, label: String) {
    if !pages.drives.start_polling(&key) {
        return;
    }
    let core = Arc::downgrade(core);
    std::thread::spawn(move || loop {
        std::thread::sleep(drive::POLL);
        if !pages.drives.polls(&key) {
            break;
        }
        let Some(core) = core.upgrade() else {
            break;
        };
        let Some(webview) = core.app.get_webview(&label) else {
            pages.drives.stop_polling(&key);
            break;
        };
        if drive::gestures(&webview) > 0 {
            if pages.drives.user_took(&key) {
                tell(&core, &key);
            }
            break;
        }
    });
}

/// The tabs `agent` opened on the artifact `address` (`open_browser` with `artifact`, AB-10), by their key.
pub(crate) fn artifact_tabs(pages: &Pages, agent: &str, address: &str) -> Vec<String> {
    pages
        .all()
        .into_iter()
        .filter(|p| {
            p.opener.as_deref() == Some(agent)
                && super::browsers_open::artifact_address(&p.url).as_deref() == Some(address)
        })
        .map(|p| p.agent_id)
        .collect()
}

/// What the band says of the reload of a new version.
pub(crate) fn reload_what(version: Option<u32>) -> String {
    match version {
        Some(n) => tr!("Recharge : version {n}", "Reloads: version {n}"),
        None => tr!("Recharge : nouvelle version", "Reloads: new version"),
    }
}

/// A new version of an artifact `agent` published, read in its own journal (its `Artifact` tool's result, AB-10;
/// never an address a page, a file or the agent's text gave): each tab it opened on that artifact reloads, the band
/// saying « Recharge : version N ». A tab whose hand is the user's (their gestures since the agent's last one
/// included), or whose driving ended, is left as they have it; « Agents et navigateurs » off, nothing.
pub(crate) fn artifact_published<R: Runtime>(
    core: &Arc<Core<R>>,
    agent: &str,
    address: &str,
    version: Option<u32>,
) {
    if !core.settings.read().agent_browsers {
        return;
    }
    let Some(pages) = pages_of(core) else {
        return;
    };
    for key in artifact_tabs(&pages, agent, address) {
        let (core, pages, agent) = (core.clone(), pages.clone(), agent.to_string());
        std::thread::spawn(move || {
            let label = devpage::page_label(&key);
            let Some(webview) = core.app.get_webview(&label) else {
                return;
            };
            if user_moved(&core, &pages, &key, &webview) {
                return;
            }
            if pages
                .drives
                .reloaded(&key, &agent, &reload_what(version))
                .is_none()
            {
                return;
            }
            if let Err(e) = webview.reload() {
                log::warn!("browser {key}: the artifact's new version not reloaded: {e}");
                return;
            }
            tell(&core, &key);
            keep_polling(&core, pages, key, label);
        });
    }
}

/// `drive_browser`.
pub(super) async fn drive<R: Runtime>(
    core: &Arc<Core<R>>,
    caller: &Caller,
    args: DriveBrowserArgs,
) -> Result<Vec<ContentBlock>, ToolError> {
    let row = row_of(core, caller, &args.browser)?;
    let act = act_of(&args, &row)?;
    let raw = args.target.as_deref().or(args.frame.as_deref());
    let pages = pages_of(core).unwrap_or_default();
    // The tiers: outside the launch, to act waits for the user's leave (C4), the one of read_browser.
    let leave = match &row.line {
        Line::Outside(o) => pages.granted(&row.key, &row.agent, o),
        _ => true,
    };
    match drive::decide(&row.line, &act, leave) {
        Verdict::Go => {}
        Verdict::Refused(r) => return Err(tier_refused(r)),
        Verdict::Leave => {
            let Line::Outside(origin) = &row.line else {
                unreachable!("a leave outside the launch alone")
            };
            return asked(core, &pages, &row, origin);
        }
    }
    // Never a secret of Escouade, whatever the field.
    if let Act::Type { text, .. } = &act {
        if drive::holds_secret(text, &secrets_of(core)) {
            return Err(forbidden(Forbidden::Secret));
        }
    }
    // navigate: an address of the browser's origin or of the launch; another opens by open_browser.
    let to = match &act {
        Act::Navigate(u) => Some(navigate_to(core, &pages, &row, u)?),
        _ => None,
    };
    // The hand, before anything: the user's, or the driving ended.
    match pages.drives.blocked(&row.key) {
        Some(May::UserHasHand) => return Err(hand_refused()),
        Some(_) => return Err(stopped_refused()),
        None => {}
    }
    let webview = core
        .app
        .get_webview(&row.label)
        .ok_or_else(|| page_failed("gone", None))?;
    // The hand: the user's gestures since the last call first, then the pace.
    {
        let (c, p, k, w) = (
            core.clone(),
            pages.clone(),
            row.key.clone(),
            webview.clone(),
        );
        let moved = tokio::task::spawn_blocking(move || user_moved(&c, &p, &k, &w))
            .await
            .unwrap_or(false);
        if moved {
            return Err(hand_refused());
        }
    }
    // The pace: within a second, the gesture waits its turn (1 s at most); an hour full is refused.
    let mut tries = 0;
    loop {
        match pages.drives.take(&row.key, &row.agent, Instant::now()) {
            May::Go => break,
            May::UserHasHand => return Err(hand_refused()),
            May::Stopped => return Err(stopped_refused()),
            May::Slow(1) if tries < 4 => {
                tries += 1;
                tokio::time::sleep(Duration::from_millis(260)).await;
            }
            May::Slow(wait) => {
                return Err(refused(tr!(
                    "4 gestes par seconde et 600 par heure au plus sur un navigateur : le prochain dans {wait} s.",
                    "4 gestures a second and 600 an hour at most on a browser: the next one in {wait} s."
                )))
            }
        }
    }
    // The element, and what is never done to it; where it is, for the ghost cursor.
    let mut at: Option<(f64, f64)> = None;
    if (act.target().is_some() && act.tier() == drive::Tier::Act) || matches!(act, Act::Press(_)) {
        let (w, a) = (webview.clone(), act.clone());
        let (kind, bx) =
            tokio::task::spawn_blocking(move || -> Result<(drive::Kind, Value), String> {
                let v = match &a {
                    Act::Press(_) => drive::run(&w, json!({ "run": "focused" }))?,
                    a => {
                        let t = a.target().expect("a target");
                        let v = drive::run(
                            &w,
                            json!({ "run": "locate", "target": t.arg(), "reveal": true }),
                        )?;
                        if v["found"] != json!(true) {
                            return Err("missing".into());
                        }
                        v
                    }
                };
                Ok((
                    serde_json::from_value(v["kind"].clone()).unwrap_or_default(),
                    v["box"].clone(),
                ))
            })
            .await
            .map_err(|e| ToolError::Failed(e.to_string()))?
            .map_err(|e| page_failed(&e, raw))?;
        let password = kind.password
            && matches!(act, Act::Type { .. })
            && row.line == Line::Launch
            && match pages.drives.password(&row.key, &row.agent) {
                Ok(given) => given,
                Err(()) => {
                    return Err(refused(tr!(
                        "L’utilisateur a refusé que tu remplisses un mot de passe dans ce navigateur : ne le redemande pas.",
                        "The user refused to let you fill a password in this browser: do not ask again."
                    )))
                }
            };
        if let Some(f) = drive::forbid(&row.line, &act, &kind, password) {
            if f == Forbidden::PasswordAsk {
                tell(core, &row.key);
            }
            return Err(forbidden(f));
        }
        at = drive::centre(&bx);
    }
    // « Reprendre la main » stops the agent at once, in the core: taken while this gesture was checked, it is not made.
    match pages.drives.still(&row.key) {
        May::Go => {}
        May::UserHasHand => return Err(hand_refused()),
        _ => return Err(stopped_refused()),
    }
    // The ghost cursor goes where the agent acts: before a click, a key or a text, on the element.
    let who = initials_of(core, &row.agent);
    if let Some(at) = at {
        show_cursor(&pages, &row, &webview, at, &who).await;
    }
    // The gesture.
    let extra = perform(core, &pages, &row, &act, to, webview.clone()).await?;
    // After a scroll or a highlight, where it took the page.
    if at.is_none() {
        if let Some(to) = cursor_after(&act, &extra, &webview).await {
            show_cursor(&pages, &row, &webview, to, &who).await;
        }
    }
    let what = what_of(&act, raw);
    let step = pages.drives.done(&row.key, &what, &extra);
    tell(core, &row.key);
    keep_polling(core, pages.clone(), row.key.clone(), row.label.clone());
    let now = extra["url"].as_str().unwrap_or(&row.url);
    let unread = matches!(&row.line, Line::Outside(_))
        && !origin_of(now).is_some_and(|o| pages.granted(&row.key, &row.agent, &o));
    let (url, title) = told(unread, now, extra["title"].as_str().unwrap_or(""));
    let mut answer = json!({
        "done": true,
        "step": step,
        "what": what,
        "url": url,
        "title": title,
        "hand": pages.drives.hand(&row.key).unwrap_or(Hand::Agent),
    });
    for k in ["scrollY", "innerWidth", "found", "ms", "version", "frame"] {
        if !extra[k].is_null() {
            answer[k] = extra[k].clone();
        }
    }
    if let (Some(info), true) = (&row.page, answer["version"].is_null()) {
        answer["version"] = json!(info.shown);
    }
    // Under WebKit a click, a key or a text is the isolated world's, not the user's (`isTrusted` false).
    if cfg!(not(windows)) && matches!(act, Act::Click(_) | Act::Type { .. } | Act::Press(_)) {
        answer["synthetic"] = json!(true);
    }
    let mut blocks = Vec::new();
    if args.look.unwrap_or(false) {
        match look(&pages, &row, webview.clone()).await {
            Ok(img) => blocks.push(img),
            Err(e) => answer["look"] = json!(e),
        }
    }
    if args.wait_user.unwrap_or(false) {
        let rx = pages.drives.wait(&row.key);
        tell(core, &row.key);
        let end = match tokio::time::timeout(drive::WAIT_USER, rx).await {
            Ok(Ok(end)) => end,
            Ok(Err(_)) => WaitEnd::TookHand,
            Err(_) => {
                pages.drives.unwait(&row.key);
                WaitEnd::Timeout
            }
        };
        tell(core, &row.key);
        answer["user"] = json!(end);
        answer["hand"] = json!(pages.drives.hand(&row.key).unwrap_or(Hand::Agent));
    }
    blocks.extend(marked(&answer));
    Ok(blocks)
}

/// What a gesture's answer says of the page, its address and its title. `unread`, outside the launch without the leave
/// to read and act (C4: to show tells nothing of the page): its origin alone and no title, as `list_browsers` cuts it,
/// for a path or a title may name a document or a conversation of the user's.
pub(super) fn told(unread: bool, url: &str, title: &str) -> (String, String) {
    if unread {
        (origin_of(url).unwrap_or_default(), String::new())
    } else {
        (url.to_string(), title.to_string())
    }
}

/// Outside the launch, to act asks the user's leave in the browser's band: « veut lire et agir », the leave of
/// `read_browser` (one per origin and per browser).
fn asked<R: Runtime>(
    core: &Core<R>,
    pages: &Pages,
    row: &Row,
    origin: &str,
) -> Result<Vec<ContentBlock>, ToolError> {
    match pages.leave(&row.key, &row.agent, origin) {
        devpage::Leave::Granted => Err(ToolError::Failed(tr!(
            "L’accord vient d’arriver : refais l’appel.",
            "The leave just came: call again."
        ))),
        devpage::Leave::Refused => Err(refused(tr!(
            "L’utilisateur a refusé que tu lises et agisses sur {origin} dans ce navigateur : ne le redemande pas, écris-lui si tu en as besoin.",
            "The user refused to let you read and act on {origin} in this browser: do not ask again, write to them if you need it."
        ))),
        l => {
            if l == devpage::Leave::Asked {
                core.hub.emit(UiEvent::DevPage {
                    page: devpage::PageEvent {
                        agent_id: row.key.clone(),
                        phase: "consents",
                        url: String::new(),
                        title: String::new(),
                    },
                });
            }
            let host = origin.split("://").nth(1).unwrap_or(origin).to_string();
            let said = json!({
                "asked": true,
                "browser": row.key,
                "origin": origin,
                "message": tr!(
                    "Cette page n’est pas sur le lancement du projet : montrer (défiler, mettre en évidence, largeurs, recharger) passe sans demander, mais agir (cliquer, taper, choisir, naviguer) attend que l’utilisateur te permette de lire et d’agir sur {host} (avec sa session) dans ce navigateur. La bande le lui demande ; sa réponse te reviendra par un message. Attends-la sans redemander.",
                    "This page is not on the project's launch: showing (scrolling, highlighting, widths, reloading) goes without asking, but acting (clicking, typing, choosing, navigating) waits for the user to let you read and act on {host} (with their session) in this browser. The band asks them; their answer comes back to you by a message. Wait for it without asking again."
                ),
            });
            Ok(vec![ContentBlock::text(said.to_string())])
        }
    }
}

/// The address `navigate` goes to: of the launch, or of the browser's origin (the user let it open there); never
/// another origin (open_browser asks it), never what `judge` refuses.
fn navigate_to<R: Runtime>(
    core: &Core<R>,
    pages: &Pages,
    row: &Row,
    asked: &str,
) -> Result<Url, ToolError> {
    use super::browsers_open::{judge, never_said, Judged};
    let project = core
        .agent(&row.agent)
        .ok()
        .and_then(|h| core.project(&h.lock().meta.project_id).ok())
        .ok_or_else(|| {
            refused(tr!(
                "Escouade ne connaît plus ton projet.",
                "Escouade no longer knows your project."
            ))
        })?;
    let own = devpage::own_of(&core.app);
    let server = Some(core.mcp.status().port).filter(|p| *p != 0);
    match judge(asked, &Launch::of(core, &project), |u| own(u), server).map_err(never_said)? {
        Judged::Launch(u) => Ok(u),
        Judged::Ask(u, origin) => {
            let here = origin_of(&row.url).is_some_and(|o| o == origin);
            if here || pages.origin_allowed(&row.key, &origin) {
                Ok(u)
            } else {
                Err(refused(tr!(
                    "Cette adresse est d’une autre origine que ce navigateur : ouvre-la par open_browser, qui la demande à l’utilisateur.",
                    "This address is of another origin than this browser: open it by open_browser, which asks the user."
                )))
            }
        }
    }
}

/// Does the gesture and lets the page settle: what it says after (`url`, `title`, `scrollY`…).
async fn perform<R: Runtime>(
    core: &Arc<Core<R>>,
    pages: &Arc<Pages>,
    row: &Row,
    act: &Act,
    to: Option<Url>,
    webview: tauri::Webview<R>,
) -> Result<Value, ToolError> {
    let raw_t = match act {
        Act::InFrame { frame, target, .. } => Some(target.as_ref().unwrap_or(frame)),
        a => a.target(),
    }
    .map(|t| match t {
        Target::Css(c) => c.clone(),
        Target::Text(t) => format!("text={t}"),
    });
    let fail = |e: String| page_failed(&e, raw_t.as_deref());
    match act {
        Act::Version(n) => {
            let page = row
                .page
                .as_ref()
                .map(|p| p.page.clone())
                .unwrap_or_default();
            core.page_drive_version(&row.agent, &page, *n)
                .map_err(ToolError::Failed)?;
            let w = webview.clone();
            let mut v = blocking(move || Ok(drive::settle(&w)))
                .await
                .map_err(fail)?;
            v["version"] = json!(n);
            return Ok(v);
        }
        Act::Width(mode) if row.line != Line::AgentPage => {
            pages.set_mode(&row.key, *mode);
            tell_mode(core, &row.key, *mode);
            tokio::time::sleep(Duration::from_millis(800)).await;
            let w = webview.clone();
            return blocking(move || drive::run(&w, json!({ "run": "where" })))
                .await
                .map_err(fail);
        }
        Act::Wait(target, timeout) => {
            let (t, timeout, key) = (target.clone(), *timeout, row.key.clone());
            let page = row.page.is_some();
            let pages = pages.clone();
            let start = Instant::now();
            loop {
                // « Reprendre la main » ends the wait at once.
                if pages.drives.blocked(&key).is_some() {
                    return Err(hand_refused());
                }
                let w = webview.clone();
                let tt = t.clone();
                let v = blocking(move || match &tt {
                    Some(t) => drive::run(&w, json!({ "run": "locate", "target": t.arg() })),
                    None => drive::run(&w, json!({ "run": "where" })),
                })
                .await;
                let ok = match (&t, &v) {
                    (Some(_), Ok(v)) => v["found"] == json!(true),
                    (None, Ok(v)) => {
                        v["ready"].as_str() == Some("complete")
                            && (page || pages.get(&key).is_none_or(|p| p.loaded))
                    }
                    _ => false,
                };
                if ok || start.elapsed() >= timeout {
                    let mut v = v.unwrap_or_else(|_| json!({}));
                    v["found"] = json!(ok);
                    v["ms"] = json!(start.elapsed().as_millis() as u64);
                    v.as_object_mut().map(|o| o.remove("kind"));
                    return Ok(v);
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
        _ => {}
    }
    // What the agent highlights, drawn again by Windows’ overlay after each ghost cursor; none after a new document.
    match act {
        Act::Highlight(t, _) => pages.drives.set_mark(&row.key, Some(t.arg())),
        Act::Clear
        | Act::Navigate(_)
        | Act::Back
        | Act::Forward
        | Act::Reload
        | Act::Version(_) => pages.drives.set_mark(&row.key, None),
        _ => {}
    }
    let (a, color) = (act.clone(), row.color.clone());
    let agent_page = row.line == Line::AgentPage;
    blocking(move || {
        let w = &webview;
        match &a {
            Act::Navigate(_) => {
                w.navigate(to.expect("an address"))
                    .map_err(|e| e.to_string())?;
                std::thread::sleep(Duration::from_millis(300));
            }
            Act::Back => {
                drive::run(w, json!({ "run": "history", "dir": "back" }))?;
                std::thread::sleep(Duration::from_millis(300));
            }
            Act::Forward => {
                drive::run(w, json!({ "run": "history", "dir": "forward" }))?;
                std::thread::sleep(Duration::from_millis(300));
            }
            Act::Reload => {
                w.reload().map_err(|e| e.to_string())?;
                std::thread::sleep(Duration::from_millis(300));
            }
            Act::ScrollTo(t) => found(drive::run(
                w,
                json!({ "run": "scroll_to", "target": t.arg() }),
            )?)?,
            Act::Scroll(by) => {
                drive::run(w, json!({ "run": "scroll", "by": by }))?;
            }
            // Windows: the browser’s overlay, out of the page’s DOM; WebKit: Escouade’s element (ESC-680). A highlight
            // in a frame of another origin goes first (ESC-682).
            Act::Highlight(t, label) => {
                drive::clear_frames(w);
                if !drive::highlight(w, &t.arg(), label, &color, true)? {
                    return Err("missing".into());
                }
            }
            Act::Clear => {
                drive::clear_frames(w);
                drive::unhighlight(w)?;
            }
            Act::InFrame {
                frame,
                gesture,
                target,
                label,
            } => {
                let reach = drive::in_frame(w, frame, *gesture, target.as_ref(), label, &color)?;
                let mut v = drive::settle(w);
                v["frame"] = json!(reach);
                return Ok(v);
            }
            Act::Width(mode) if agent_page => drive::emulate_width(w, *mode)?,
            Act::Tab(t) | Act::Click(t) => {
                let v = drive::run(
                    w,
                    json!({ "run": "locate", "target": t.arg(), "reveal": true }),
                )?;
                if v["found"] != json!(true) {
                    return Err("missing".into());
                }
                // An agent's page is Escouade's own: its declared tab clicked in its world.
                if agent_page {
                    found(drive::run(w, json!({ "run": "click", "target": t.arg() }))?)?;
                } else {
                    drive::click_at(w, t, &v["box"])?;
                }
            }
            Act::Type {
                target,
                text,
                submit,
            } => drive::type_in(w, target, text, *submit)?,
            Act::Select(t, value) => {
                let v = drive::run(
                    w,
                    json!({ "run": "choose", "target": t.arg(), "value": value }),
                )?;
                if v["found"] != json!(true) {
                    return Err("missing".into());
                }
                if v["option"] != json!(true) {
                    return Err("option".into());
                }
            }
            Act::Press(k) => drive::key(w, *k)?,
            Act::Version(_) | Act::Wait(..) | Act::Width(_) => {}
        }
        Ok(drive::settle(w))
    })
    .await
    .map_err(fail)
}

/// `Err("missing")` when the library found no element.
fn found(v: Value) -> Result<(), String> {
    if v["found"] == json!(true) {
        Ok(())
    } else {
        Err("missing".into())
    }
}

/// Runs `f` off the async runtime: it waits for the webview's thread.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

/// `look`: a capture of the view after the gesture, within the bounds of `capture_browser` (20 a minute) and its
/// leave outside the launch. Its image, or why there is none.
async fn look<R: Runtime>(
    pages: &Pages,
    row: &Row,
    webview: tauri::Webview<R>,
) -> Result<ContentBlock, String> {
    if let Line::Outside(o) = &row.line {
        if !pages.granted(&row.key, &row.agent, o) {
            return Err(tr!(
                "pas de capture : hors du lancement, elle attend que l’utilisateur te permette de lire cette origine (capture_browser le demande)",
                "no capture: outside the launch, it waits for the user to let you read this origin (capture_browser asks it)"
            ));
        }
    }
    if let Err(wait) = pages.take_shot(&row.agent, Instant::now()) {
        return Err(tr!(
            "pas de capture : 20 par minute au plus, la prochaine dans {wait} s",
            "no capture: 20 a minute at most, the next one in {wait} s"
        ));
    }
    // Under Windows the overlay has one shape at a time, the ghost cursor's while it stays: what the agent highlights
    // is drawn again first, so that it sees what it shows (ESC-683).
    let (mark, color) = (pages.drives.mark(&row.key), row.color.clone());
    let shot = blocking(move || {
        if mark.is_some() {
            drive::cursor_gone(&webview, mark.as_ref(), &color);
        }
        let shot = crate::browserread::capture(&webview, crate::browserread::Ask::default())?;
        crate::browserread::fit(&shot)
    })
    .await
    .map_err(|e| tr!("pas de capture : {e}", "no capture: {e}"))?;
    use base64::Engine;
    Ok(ContentBlock::image(
        base64::engine::general_purpose::STANDARD.encode(&shot.bytes),
        "image/jpeg".to_string(),
    ))
}

// ---------- the ghost cursor (ESC-680, AB-8) ----------

/// The initials of the agent on its ghost cursor (WebKit draws them; Windows' overlay draws shapes alone).
fn initials_of<R: Runtime>(core: &Core<R>, agent: &str) -> String {
    let name = core
        .agent(agent)
        .map(|h| h.lock().meta.name.clone())
        .unwrap_or_else(|_| agent.to_string());
    drive::initials(&name)
}

/// Where the ghost cursor goes after a gesture that has no element to act on first: the element it scrolled to or
/// highlighted, the middle of the view after a scroll.
async fn cursor_after<R: Runtime>(
    act: &Act,
    extra: &Value,
    webview: &tauri::Webview<R>,
) -> Option<(f64, f64)> {
    match act {
        Act::ScrollTo(t) | Act::Highlight(t, _) => {
            let (w, t) = (webview.clone(), t.arg());
            let v = blocking(move || drive::run(&w, json!({ "run": "locate", "target": t })))
                .await
                .ok()?;
            drive::centre(&v["box"])
        }
        Act::Scroll(_) => {
            let w = extra["innerWidth"].as_f64()?;
            let h = extra["view"].as_f64()?;
            Some((w / 2.0, h / 2.0))
        }
        _ => None,
    }
}

/// Shows the ghost cursor of the row at `to`, gliding from where it was; under Windows, once it stayed, the overlay
/// draws again what the agent highlights (it has one shape at a time), unless a later cursor came.
async fn show_cursor<R: Runtime>(
    pages: &Arc<Pages>,
    row: &Row,
    webview: &tauri::Webview<R>,
    to: (f64, f64),
    initials: &str,
) {
    let (from, turn) = pages.drives.cursor_to(&row.key, to);
    let (w, color, who) = (webview.clone(), row.color.clone(), initials.to_string());
    if let Err(e) = blocking(move || drive::cursor(&w, from, to, &color, &who)).await {
        log::debug!("browsers: no ghost cursor on {}: {e}", row.key);
        return;
    }
    if cfg!(windows) {
        let (pages, key, w, color) = (
            pages.clone(),
            row.key.clone(),
            webview.clone(),
            row.color.clone(),
        );
        std::thread::spawn(move || {
            std::thread::sleep(drive::CURSOR_STAY);
            if pages.drives.cursor_still(&key, turn) {
                drive::cursor_gone(&w, pages.drives.mark(&key).as_ref(), &color);
            }
        });
    }
}

// ---------- the rail of an agent's page (ESC-680, AB-8) ----------

/// The label of the webview of the row `key`: an agent's page, or a tab.
fn label_of(key: &str) -> String {
    match crate::pages::of_key(key) {
        Some(page) => crate::agentpage::label(page),
        None => devpage::page_label(key),
    }
}

/// Where the page of the row `key` is (its scroll, height and view): the rail draws the part on screen.
pub(crate) async fn place<R: Runtime>(core: &Arc<Core<R>>, key: &str) -> Option<Value> {
    let webview = core.app.get_webview(&label_of(key))?;
    let v = blocking(move || drive::place(&webview)).await.ok()?;
    Some(json!({ "y": v["scrollY"], "height": v["height"], "view": v["view"] }))
}

/// A pin of the rail clicked: the page goes back where the agent had put it. The user's move, not a gesture of the
/// agent's: the hand does not change.
pub(crate) async fn revisit<R: Runtime>(
    core: &Arc<Core<R>>,
    pages: &Arc<Pages>,
    key: &str,
    n: u32,
) -> Result<(), String> {
    let pin = pages.drives.pin(key, n).ok_or("no such pin")?;
    let webview = core
        .app
        .get_webview(&label_of(key))
        .ok_or("the page is not shown")?;
    blocking(move || drive::go_to(&webview, pin.y)).await
}

// ---------- the user's moves (the band, AB-8) ----------

/// A move of the user's on the row `key`: the hand taken, given back, the driving stopped, « Continuer ». The agent
/// is told when the hand comes back to it, or when the driving ends.
pub(crate) async fn hand<R: Runtime>(
    core: &Arc<Core<R>>,
    pages: &Arc<Pages>,
    key: &str,
    m: drive::HandMove,
) -> drive::DriveView {
    let (changed, agent) = pages.drives.moved(key, m);
    if changed {
        tell(core, key);
        if let Some(agent) = agent {
            let text = match m {
                drive::HandMove::Give => Some(tr!(
                    "Escouade : l’utilisateur te rend la main sur le navigateur {key}. Tu peux reprendre drive_browser.",
                    "Escouade: the user gives you the hand back on the browser {key}. You may call drive_browser again."
                )),
                drive::HandMove::Stop => Some(tr!(
                    "Escouade : l’utilisateur ne laisse plus piloter le navigateur {key}, jusqu’à sa fermeture.",
                    "Escouade: the user no longer lets the browser {key} be driven, until it closes."
                )),
                _ => None,
            };
            if let Some(text) = text {
                if let Err(e) = core.send_by_app(&agent, text).await {
                    log::warn!("browsers: {agent} not told of the hand: {e:#}");
                }
            }
        }
    }
    pages.drives.view(key)
}

/// « Laisser piloter par… » (AB-9): the user lets \`agent\` of the browser's project drive the browser \`key\` they opened
/// alone, or nobody (\`None\`). The agent is told by a message; one it was lent to before, that it no longer drives it.
/// Its driving starts afresh: the hand to the agent, no leave of a password kept.
pub(crate) async fn lend<R: Runtime>(
    core: &Arc<Core<R>>,
    pages: &Arc<Pages>,
    key: &str,
    agent: Option<&str>,
) -> Result<(), String> {
    let page = pages.get(key).ok_or("no such browser")?;
    if !key.starts_with(devpage::BROWSER_PREFIX) || page.opener.is_some() {
        return Err(tr!(
            "Seul un navigateur que tu as ouvert toi-même se laisse piloter par un agent.",
            "Only a browser you opened yourself is let be driven by an agent."
        ));
    }
    if let Some(a) = agent {
        let meta = core.agent(a).map(|h| h.lock().meta.clone()).map_err(|_| {
            tr!(
                "Escouade ne connaît pas cet agent.",
                "Escouade does not know this agent."
            )
        })?;
        if meta.project_id != page.project_id || meta.archived {
            return Err(tr!(
                "Cet agent n’est pas un agent actif du projet de ce navigateur.",
                "This agent is not a live agent of this browser's project."
            ));
        }
        if page.driver.as_deref() != Some(a) && pages.driven_by(a, key) >= devpage::DRIVEN_MAX {
            return Err(tr!(
                "{name} pilote déjà {n} navigateurs : c’est la borne.",
                "{name} drives {n} browsers already: that is the bound.",
                name = meta.name,
                n = devpage::DRIVEN_MAX
            ));
        }
    }
    let before = page.driver.clone();
    if before.as_deref() == agent {
        return Ok(());
    }
    pages.set_driver(key, agent);
    pages.drives.forget(key);
    tell(core, key);
    let host = Url::parse(&page.url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| page.url.clone());
    if let Some(old) = before {
        let text = tr!(
            "Escouade : l’utilisateur ne te laisse plus piloter le navigateur {key} ({host}).",
            "Escouade: the user no longer lets you drive the browser {key} ({host})."
        );
        if let Err(e) = core.send_by_app(&old, text).await {
            log::warn!("browsers: {old} not told it no longer drives {key}: {e:#}");
        }
    }
    if let Some(a) = agent {
        let text = tr!(
            "Escouade : l’utilisateur te laisse piloter le navigateur {key} ({host}), qu’il a ouvert lui-même. Pilote-le par drive_browser avec browser: \"{key}\" tant qu’il est à l’écran ; il peut reprendre la main à tout moment.",
            "Escouade: the user lets you drive the browser {key} ({host}), which they opened themselves. Drive it by drive_browser with browser: \"{key}\" while it is on screen; they may take the hand back at any time."
        );
        if let Err(e) = core.send_by_app(a, text).await {
            log::warn!("browsers: {a} not told it may drive {key}: {e:#}");
        }
    }
    Ok(())
}

/// The user's answer to `agent`'s request to fill a password in the row `key` (C2): told to the agent.
pub(crate) async fn password<R: Runtime>(
    core: &Arc<Core<R>>,
    pages: &Arc<Pages>,
    key: &str,
    agent: &str,
    allow: bool,
) -> drive::DriveView {
    if pages.drives.answer_password(key, agent, allow) {
        tell(core, key);
        let text = if allow {
            tr!(
                "Escouade : l’utilisateur te permet de remplir un mot de passe dans le navigateur {key}, jusqu’à sa fermeture. Tu peux refaire drive_browser type.",
                "Escouade: the user lets you fill a password in the browser {key}, until it closes. You may call drive_browser type again."
            )
        } else {
            tr!(
                "Escouade : l’utilisateur a refusé que tu remplisses un mot de passe dans le navigateur {key}. Ne le redemande pas.",
                "Escouade: the user refused to let you fill a password in the browser {key}. Do not ask again."
            )
        };
        if let Err(e) = core.send_by_app(agent, text).await {
            log::warn!("browsers: {agent} not told of the password: {e:#}");
        }
    }
    pages.drives.view(key)
}

/// The name of an action, as the tool takes it (the log's summary).
pub(super) fn action_name(a: DriveActionArg) -> &'static str {
    match a {
        DriveActionArg::Navigate => "navigate",
        DriveActionArg::Back => "back",
        DriveActionArg::Forward => "forward",
        DriveActionArg::Reload => "reload",
        DriveActionArg::Wait => "wait",
        DriveActionArg::ScrollTo => "scroll_to",
        DriveActionArg::Scroll => "scroll",
        DriveActionArg::Highlight => "highlight",
        DriveActionArg::Clear => "clear",
        DriveActionArg::Width => "width",
        DriveActionArg::Tab => "tab",
        DriveActionArg::Click => "click",
        DriveActionArg::Type => "type",
        DriveActionArg::Select => "select",
        DriveActionArg::Press => "press",
        DriveActionArg::Version => "version",
    }
}
