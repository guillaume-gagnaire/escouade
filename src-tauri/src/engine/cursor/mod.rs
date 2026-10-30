//! Cursor CLI, the fifth harness: an `agent acp` process per agent, driven over ACP v1 by a client
//! of Escouade's own (`process`), its updates translated by `AgentRt::cursor_frame`
//! (`agent/cursor.rs`).
//!
//! What it does as Escouade drives it comes from the real runs of Cursor CLI 2026.10.22 on a
//! Cursor Free account (ESC-24, docs/TODO-1.8.md, point 0, « Cursor CLI »), whose streams are in
//! `tests/fixtures/cursor/` and whose protocol, extensions `cursor/*` included, is surveyed in
//! `tests/fixtures/engines/cursor-2026.10.22-27dcf5f.acp.json`. ACP tells no tokens nor cost, and
//! copies no session: none is counted, and « Dupliquer » is hidden.
//!
//! Each agent has a configuration folder of its own (`CURSOR_CONFIG_DIR`, `Engine::agent_home_var`):
//! Cursor's « Toujours » writes its rule there (`permissions.allow` of its `cli-config.json`), so
//! that it holds for that agent alone, never for the user's own Cursor (decided by the user on
//! 2026-10-25); its model and its sessions are kept there too. Its sign-in is not moved
//! (`%APPDATA%\Cursor\auth.json`).

#[cfg(test)]
pub(crate) mod bench;
mod locate;
mod process;
pub(crate) mod terminal;

#[cfg(test)]
pub(crate) use bench::fake_cursor;
use process::CursorProcess;

use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
    ToldModel,
};
use super::{
    mcp, BoxFuture, Engine, EngineProcess, Escouade, Message, OneShot, PermissionAnswer, Process,
    Program, Room, Spawn, StartLost, Weight,
};
use crate::agent::{AgentRt, Effects};
use crate::core::{size_label, Attachment, MAX_IMAGE, MAX_TOTAL};
use crate::i18n::{self, Lang};
use crate::model::EngineId;
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

/// Cursor CLI.
pub struct Cursor;

pub static CURSOR: Cursor = Cursor;

/// The versions of Cursor CLI Escouade was tested with: as the real run of 2026-10-25 found it
/// (ESC-24). Its versions are dates (`2026.10.22-27dcf5f`).
pub const TESTED: Tested = Tested {
    oldest: "2026.10.22",
    newest: "2026.10.22",
};

/// Its installer.
const INSTALL: Install = Install {
    windows: "irm 'https://cursor.com/install?win32=true' | iex",
    unix: "curl https://cursor.com/install -fsS | bash",
};

/// The variable that moves its configuration folder (its rules, its model, its sessions).
pub(crate) const CONFIG_VAR: &str = "CURSOR_CONFIG_DIR";

/// Auto, the model Cursor picks itself, by the id Escouade keeps (`default[]` for Cursor): the only
/// one a Free account runs (tried).
pub(crate) const AUTO: &str = "default";

/// The models of its menu until a process of the account has listed them.
const MODELS: [(&str, &str); 1] = [(AUTO, "Auto")];

/// The efforts Escouade's menu has, in its order: a model's own among them.
const EFFORTS: [&str; 4] = ["low", "medium", "high", "xhigh"];

/// What Cursor answers, for any message, on a model its plan does not offer: a text, no error
/// (tried with `composer-2.5` on a Free account).
pub(crate) const UPGRADE: &str = "Upgrade your plan to continue";

/// The model, effort and mode in force (Cursor takes them at once).
#[derive(Debug, Clone, Default)]
pub(crate) struct Settings {
    pub model: String,
    pub effort: String,
    pub mode: String,
}

/// The mode of Cursor for Escouade's `mode`: `plan` to plan (it changes nothing there), else
/// `agent`; `ask` for the texts written (`one_shot`), never offered to an agent.
pub(crate) fn acp_mode(mode: &str) -> &'static str {
    match mode {
        "plan" => "plan",
        "ask" => "ask",
        _ => "agent",
    }
}

/// The modes Escouade gives Cursor, in the menu's order (its `ask` is not offered: Escouade has
/// nothing like it).
const MODES: [&str; 4] = ["default", "plan", "acceptEdits", "bypassPermissions"];

/// What a mode means in Cursor, as the menu says it: nothing is confined under Windows in ACP
/// (tried), only the permissions protect.
fn meaning(lang: Lang, mode: &str) -> String {
    match mode {
        "plan" => tr_in!(
            lang,
            "Cursor étudie et propose un plan sans rien modifier (son mode Plan). Pas de bac à sable : seules les permissions protègent.",
            "Cursor studies and proposes a plan without changing anything (its Plan mode). No sandbox: only the permissions protect."
        ),
        "acceptEdits" => tr_in!(
            lang,
            "Comme « Demander », et git add et git commit du dossier de l’agent passent sans demander. Cursor modifie les fichiers sans rien soumettre ni passer par Escouade, même hors du dossier. Pas de bac à sable : une commande acceptée peut tout faire.",
            "As “Ask”, and git add and git commit in the agent’s folder go through without asking. Cursor edits files without submitting them nor going through Escouade, even outside the folder. No sandbox: an accepted command can do anything."
        ),
        "bypassPermissions" => tr_in!(
            lang,
            "Escouade accepte tout ce que Cursor demande : commandes, fichiers, web et tout le disque. Pas de bac à sable.",
            "Escouade accepts all Cursor asks: commands, files, the web and the whole disk. No sandbox."
        ),
        _ => tr_in!(
            lang,
            "Ses commandes, le web et les outils MCP demandent. Cursor lit et modifie les fichiers sans demander, même hors du dossier de l’agent : il ne les fait pas passer par Escouade, qui ne peut pas l’en empêcher. Une écriture qu’il soumet passe sans demander dans le dossier, et ailleurs avec son diff. Pas de bac à sable sous Windows : une commande acceptée peut tout faire.",
            "Its commands, the web and MCP tools ask. Cursor reads and edits files without asking, even outside the agent’s folder: it doesn’t pass them through Escouade, which can’t stop it. A write it submits goes through in the folder, and asks elsewhere, with its diff. No sandbox on Windows: an accepted command can do anything."
        ),
    }
}

/// What Cursor does, as Escouade drives it.
pub fn caps() -> EngineCaps {
    let lang = i18n::ui();
    EngineCaps {
        modes: MODES
            .into_iter()
            .map(|id| ModeCap {
                id,
                meaning: Some(meaning(lang, id)),
            })
            .collect(),
        effort: true,
        // ACP's `promptCapabilities.image`.
        images: true,
        pdf: false,
        // No copy in ACP (its interface has `/fork`).
        fork: ForkPoint::None,
        resume_other_account: false,
        // A second prompt would cancel the running turn: the process holds it until its end.
        mid_turn: MidTurn::Queued,
        slash_commands: false,
        // Its usage of the month is told by its interface alone (`/usage`, from the API of its
        // dashboard with the token of the sign-in, which Escouade never reads): its dashboard is
        // linked instead (`DASHBOARD`), and its limit ends a turn « limité » (ESC-43).
        quota: QuotaKind::None,
        quota_windows: Vec::new(),
        // Neither tokens nor cost in ACP.
        cost: CostKind::None,
        // Its interface, with what its transcript tells (`terminal`): ESC-49.
        terminal_view: true,
        remote_control: false,
        background_tasks: false,
        todo_tools: false,
        default_refusal: false,
        sandbox: false,
        usage_page: Some(DASHBOARD),
        // Its sign-in is the machine's alone: another account is an API key (ESC-43).
        accounts_by_key: true,
        read_only: false,
    }
}

/// A model of the account as `session/new` lists it: its id without its options (`claude-opus-5-5`,
/// `default` for Auto), what Cursor takes (`claude-opus-5-5[context=300k,effort=medium,…]`), its
/// name, and its efforts with the option that sets them (`effort`, `reasoning_effort`…).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToldOf {
    pub base: String,
    pub full: String,
    pub name: String,
    pub efforts: Vec<String>,
    pub default_effort: Option<String>,
    pub effort_key: Option<String>,
}

/// A model's id without its options (`grok-4.7[context=256k]` → `grok-4.7`).
pub(crate) fn base_of(id: &str) -> &str {
    id.split('[').next().unwrap_or(id)
}

/// What Cursor takes for `model`, not listed: Auto's own id, else the id as it is.
pub(crate) fn full_id(model: &str) -> String {
    if model == AUTO {
        "default[]".into()
    } else {
        model.to_string()
    }
}

/// The models of `session/new` (`listed`: `availableModels`), each with its name and efforts from
/// `cursor/list_available_models` (`options`: its `thought_level` option).
pub(crate) fn models_of(listed: &Value, options: Option<&Value>) -> Vec<ToldOf> {
    let described = |base: &str| {
        options
            .and_then(|o| o["models"].as_array())
            .and_then(|l| l.iter().find(|m| m["value"] == base))
    };
    listed["availableModels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let full = m["modelId"].as_str()?;
            let base = base_of(full);
            let d = described(base);
            let thought = d
                .and_then(|d| d["configOptions"].as_array())
                .and_then(|o| o.iter().find(|c| c["category"] == "thought_level"));
            let efforts: Vec<String> = thought
                .and_then(|t| t["options"].as_array())
                .into_iter()
                .flatten()
                .filter_map(|o| o["value"].as_str().map(str::to_string))
                .collect();
            Some(ToldOf {
                base: base.to_string(),
                full: full.to_string(),
                name: d
                    .and_then(|d| d["name"].as_str())
                    .or(m["name"].as_str())
                    .unwrap_or(base)
                    .to_string(),
                default_effort: thought
                    .and_then(|t| t["currentValue"].as_str())
                    .map(str::to_string),
                effort_key: thought
                    .filter(|_| !efforts.is_empty())
                    .and_then(|t| t["id"].as_str())
                    .map(str::to_string),
                efforts,
            })
        })
        .collect()
}

/// The composer's menu of the models `told`: Auto, the one a new agent runs (a Free account runs
/// no other).
pub(crate) fn catalog(told: &[ToldOf]) -> Vec<ToldModel> {
    let mut models: Vec<ToldModel> = told
        .iter()
        .map(|m| ToldModel {
            from_config: false,
            id: m.base.clone(),
            label: m.name.clone(),
            efforts: m.efforts.clone(),
            default_effort: m.default_effort.clone(),
            images: true,
            default: m.base == AUTO,
            provider: None,
            multiplier: None,
            price: None,
        })
        .collect();
    if models.is_empty() {
        models = MODELS
            .iter()
            .map(|&(id, label)| ToldModel {
                from_config: false,
                id: id.into(),
                label: label.into(),
                efforts: Vec::new(),
                default_effort: None,
                images: true,
                default: true,
                provider: None,
                multiplier: None,
                price: None,
            })
            .collect();
    }
    if !models.iter().any(|m| m.default) {
        models[0].default = true;
    }
    models
}

/// What `session/new` and `session/load` are given of MCP servers: Escouade's, as the agent (its
/// address and token; the token in a header, on the process's input, never its command line), or
/// none. An entry of the user's of that name is replaced by it (tried).
pub(crate) fn mcp_servers(escouade: Option<&(String, String)>) -> Value {
    match escouade {
        Some((url, token)) => json!([{
            "type": "http", "name": mcp::SERVER, "url": url,
            "headers": [{ "name": "Authorization", "value": format!("Bearer {token}") }],
        }]),
        None => json!([]),
    }
}

/// The instructions added to an agent (a ticket's protocol), as they go before its first message:
/// ACP has no system message, and `session/load` keeps them (tried).
pub(crate) fn preamble(instructions: &str) -> String {
    format!("<instructions>\n{}\n</instructions>", instructions.trim())
}

/// What was said in the agent's interface (« Vue Cursor », `handover::relay`), as it goes before its
/// next message.
pub(crate) fn relayed(relay: &str) -> String {
    format!(
        "<relais_escouade>
{}
</relais_escouade>",
        relay.trim()
    )
}

/// The option of a permission (`session/request_permission`) of `kind` (`allow_once`,
/// `allow_always`, `reject_once`), by the id Cursor gave it.
pub(crate) fn option_of(params: &Value, kind: &str) -> String {
    params["options"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|o| o["kind"] == kind)
        .and_then(|o| o["optionId"].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| kind.replace('_', "-"))
}

/// The answer to `cursor/ask_question` (its `params`) from a card's `answers` (by question, its
/// choices' labels joined by « , »): each by the ids of its choices; an answer that names none of
/// them (written by the user) skips the questions, with what was written as the reason the model
/// reads.
pub(crate) fn question_outcome(params: &Value, answers: &Value) -> Value {
    let mut picked = Vec::new();
    let mut written = Vec::new();
    for q in params["questions"].as_array().into_iter().flatten() {
        let text = q["prompt"].as_str().unwrap_or_default();
        let answer = answers[text].as_str().unwrap_or_default().trim();
        let options = q["options"].as_array().cloned().unwrap_or_default();
        let id_of = |label: &str| {
            options
                .iter()
                .find(|o| o["label"].as_str() == Some(label))
                .and_then(|o| o["id"].as_str())
                .map(str::to_string)
        };
        let ids: Option<Vec<String>> = if answer.is_empty() {
            Some(Vec::new())
        } else {
            id_of(answer).map(|id| vec![id]).or_else(|| {
                answer
                    .split(", ")
                    .map(id_of)
                    .collect::<Option<Vec<String>>>()
            })
        };
        match ids {
            Some(ids) => picked.push(json!({ "questionId": q["id"], "selectedOptionIds": ids })),
            None => written.push(format!("{text} {answer}")),
        }
    }
    if written.is_empty() {
        json!({ "outcome": "answered", "answers": picked })
    } else {
        json!({ "outcome": "skipped", "reason": written.join("\n") })
    }
}

/// « Projet privé » (ESC-140): whether the permission Cursor asks (`params` of
/// `session/request_permission`) names a path of `fence` (as Cursor names them): its command, its
/// title, the files it writes or reads. Cursor reads without asking, and a command can reach a path
/// it does not name: a guard of Escouade's, not a barrier.
pub(crate) fn fenced(params: &Value, fence: &[String]) -> bool {
    if fence.is_empty() {
        return false;
    }
    let tc = &params["toolCall"];
    let mut texts: Vec<&str> = tc["title"].as_str().into_iter().collect();
    for v in tc["rawInput"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(_, v)| v)
    {
        texts.extend(v.as_str());
    }
    for k in ["content", "locations"] {
        texts.extend(
            tc[k]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| c["path"].as_str()),
        );
    }
    texts
        .iter()
        .any(|t| fence.iter().any(|p| crate::fence::names(t, p)))
}

/// Refused: the ACP of the process is not the one Escouade speaks.
fn protocol_refused(lang: Lang, version: &Value) -> String {
    tr_in!(
        lang,
        "Cursor CLI parle ACP {version}, Escouade ACP {ours} : mets Cursor CLI à jour (agent update).",
        "Cursor CLI speaks ACP {version}, Escouade ACP {ours}: update Cursor CLI (agent update).",
        ours = process::PROTOCOL
    )
}

/// No session opened for the agent: what Cursor CLI said (not signed in, say).
fn session_refused(lang: Lang, e: &anyhow::Error) -> String {
    tr_in!(
        lang,
        "Cursor CLI n’a pas ouvert la session de l’agent : {e:#}. S’il n’est pas connecté, lance agent login dans un terminal.",
        "Cursor CLI didn’t open the agent’s session: {e:#}. If it isn’t signed in, run agent login in a terminal."
    )
}

/// Where the user sees the usage of a Cursor account, its limit and its plan: Cursor's dashboard
/// (its `/usage` says it). Escouade reads no usage of Cursor's (ESC-43): ACP tells none, and its
/// dashboard's API takes the token of the sign-in, which Escouade never reads.
pub const DASHBOARD: &str = "https://cursor.com/dashboard?tab=usage";

/// What Cursor answers in place of a turn it does not run: a text as its last words, the turn ending
/// as any other (`end_turn`, no error), by the action it asks (read in its code, 2026.10.22: `login`,
/// `upgrade`, `payment`, and a key refused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// « Upgrade your plan to continue »: the included usage of the month spent, or a model the
    /// plan does not offer (tried with `composer-2.5` on a Free account).
    Upgrade,
    /// « Add a payment method to continue »: on-demand usage, without a means to pay.
    Payment,
    /// « Please sign in to continue ».
    SignIn,
    /// « Error: [unauthenticated] Backend rejected authentication… »: the account's key refused.
    Unauthenticated,
}

/// The refusal a turn's text `text` ends with, if any.
pub(crate) fn refusal(text: &str) -> Option<Refusal> {
    let last = text.trim().rsplit("\n\n").next().unwrap_or_default().trim();
    match last {
        UPGRADE => Some(Refusal::Upgrade),
        "Add a payment method to continue" => Some(Refusal::Payment),
        "Please sign in to continue" => Some(Refusal::SignIn),
        t if t.starts_with("Error: [unauthenticated]") => Some(Refusal::Unauthenticated),
        _ => None,
    }
}

/// Whether the refusal is the account's usage limit: its turn ends « limité », on the amber card.
pub(crate) fn at_limit(refusal: Refusal) -> bool {
    matches!(refusal, Refusal::Upgrade | Refusal::Payment)
}

/// What the card says of a turn Cursor refused (`refusal`), the agent on `model`, its account by
/// its key or not (`by_key`).
pub(crate) fn refusal_error(lang: Lang, refusal: Refusal, model: &str, by_key: bool) -> String {
    match refusal {
        Refusal::Upgrade if model == AUTO || model.is_empty() => tr_in!(
            lang,
            "Limite d’usage de ce compte Cursor atteinte (« Upgrade your plan to continue ») : l’usage inclus du mois est épuisé jusqu’à la remise à zéro de son offre. Son usage : {url}",
            "This Cursor account’s usage limit is reached (“Upgrade your plan to continue”): the month’s included usage is spent until its plan resets. Its usage: {url}",
            url = DASHBOARD
        ),
        Refusal::Upgrade => tr_in!(
            lang,
            "Cursor refuse le modèle {model} sur ce compte (« Upgrade your plan to continue ») : il n’est pas dans l’offre de ce compte Cursor, ou son usage inclus du mois est épuisé. Choisis Auto dans le menu Modèle ; son usage : {url}",
            "Cursor refuses the model {model} on this account (“Upgrade your plan to continue”): it isn’t in this Cursor account’s plan, or its included usage of the month is spent. Choose Auto in the Model menu; its usage: {url}",
            url = DASHBOARD
        ),
        Refusal::Payment => tr_in!(
            lang,
            "Limite d’usage de ce compte Cursor atteinte : la suite passe par l’usage à la demande, qui demande un moyen de paiement (« Add a payment method to continue »). Son usage : {url}",
            "This Cursor account’s usage limit is reached: what comes next is on-demand usage, which needs a payment method (“Add a payment method to continue”). Its usage: {url}",
            url = DASHBOARD
        ),
        Refusal::SignIn | Refusal::Unauthenticated if by_key => tr_in!(
            lang,
            "Cursor a refusé la clé d’API de ce compte : donne-la de nouveau dans Réglages › Comptes, « Changer la clé… ».",
            "Cursor refused this account’s API key: give it again in Settings › Accounts, “Change the key…”."
        ),
        Refusal::SignIn | Refusal::Unauthenticated => tr_in!(
            lang,
            "Cursor CLI n’est pas connecté sur cette machine (« Please sign in to continue ») : lance agent login dans un terminal.",
            "Cursor CLI isn’t signed in on this machine (“Please sign in to continue”): run agent login in a terminal."
        ),
    }
}

/// What a write of Cursor that changed nothing says (its call ends `completed` with no diff): Cursor
/// gives a write done its diff, and in ACP says nothing of one refused, by a hook of the user's
/// (those of Claude Code run under Cursor too) or by Cursor itself (read in its code, 2026.10.22).
pub(crate) fn write_not_done(lang: Lang) -> String {
    tr_in!(
        lang,
        "Cursor n’a rien écrit : l’écriture a été refusée, par un hook (ceux de Claude Code tournent aussi sous Cursor) ou par Cursor lui-même, qui n’en donne pas la raison.",
        "Cursor wrote nothing: the write was refused, by a hook (those of Claude Code run under Cursor too) or by Cursor itself, which doesn’t give the reason."
    )
}

/// A message to Cursor: its text, the images joined to it (ACP's `image` blocks), the text files
/// joined to it as text. A PDF is refused.
fn message(text: &str, attachments: &[Attachment]) -> Result<Value> {
    let mut blocks = Vec::new();
    let mut total = 0;
    for a in attachments {
        total += a.data.len() / 4 * 3;
        match a.media_type.as_str() {
            t if t.starts_with("image/") => {
                if a.data.len() / 4 * 3 > MAX_IMAGE {
                    let lang = i18n::ui();
                    bail!(tr_in!(
                        lang,
                        "{name} dépasse {max}",
                        "{name} is over {max}",
                        name = a.name,
                        max = size_label(lang, MAX_IMAGE)
                    ));
                }
                blocks.push(json!({ "type": "image", "mimeType": t, "data": a.data }));
            }
            "text/plain" => {
                blocks.push(json!({ "type": "text", "text": format!("{}:\n\n{}", a.name, a.data) }))
            }
            t => bail!(tr!(
                "« {name} » ({t}) ne peut pas être joint : Cursor lit les images et les fichiers texte, pas les PDF ni les autres fichiers.",
                "“{name}” ({t}) can’t be attached: Cursor reads images and text files, not PDFs nor other files.",
                name = a.name
            )),
        }
    }
    if total > MAX_TOTAL {
        let lang = i18n::ui();
        bail!(tr_in!(
            lang,
            "Les fichiers joints à un message sont limités à {max} en tout.",
            "The files attached to a message are limited to {max} in all.",
            max = size_label(lang, MAX_TOTAL)
        ));
    }
    if !text.is_empty() || blocks.is_empty() {
        blocks.push(json!({ "type": "text", "text": text }));
    }
    Ok(json!({ "prompt": blocks }))
}

/// One question to a process of its own (`agent acp`, in its mode Ask, on Auto), in a configuration
/// folder of its own removed once answered: its last words.
async fn one_shot(ask: OneShot<'_>) -> Result<String> {
    let home =
        std::env::temp_dir().join(format!("escouade-cursor-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&home)?;
    let mut env = ask.env.clone();
    env.retain(|(k, _)| k != CONFIG_VAR);
    env.push((CONFIG_VAR.into(), home.to_string_lossy().into_owned()));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<Value>>();
    let exit = tx.clone();
    let opts = process::Opts {
        program: ask.program,
        cwd: ask.cwd.to_string_lossy().into_owned(),
        args: vec!["acp".into()],
        env,
        start: super::Start::New,
        settings: Settings {
            model: AUTO.into(),
            effort: String::new(),
            mode: "ask".into(),
        },
        instructions: Some(ask.system.to_string()).filter(|s| !s.trim().is_empty()),
        relay: None,
        escouade: None,
        fence: Vec::new(),
    };
    let p = CursorProcess::spawn(
        opts,
        Box::new(move |f| {
            let _ = tx.send(Some(f));
        }),
        Box::new(move |_, _| {
            let _ = exit.send(None);
        }),
    )?;
    let asked = async {
        p.initialize().await?;
        let m = message(ask.prompt, &[])?;
        p.send_message("", &Message(m))?;
        let mut text = String::new();
        while let Some(Some(f)) = rx.recv().await {
            let update = &f["params"]["update"];
            match f["method"].as_str().unwrap_or_default() {
                "session/update" if update["sessionUpdate"] == "agent_message_chunk" => {
                    text.push_str(update["content"]["text"].as_str().unwrap_or_default());
                }
                // Nobody to ask: what it asks is refused.
                "session/request_permission" | "cursor/create_plan" => {
                    let rid = f["id"].as_str().unwrap_or_default();
                    let _ = p.answer_permission(
                        rid,
                        PermissionAnswer::Deny {
                            call: String::new(),
                            message: String::new(),
                        },
                    );
                }
                "cursor/ask_question" => {
                    let rid = f["id"].as_str().unwrap_or_default();
                    let _ = p.reply(rid, Err("no user".into()));
                }
                "escouade.prompt_end" => {
                    if let Some(e) = f["params"]["error"].as_str() {
                        bail!("{e}");
                    }
                    break;
                }
                _ => {}
            }
        }
        let text = text.trim().to_string();
        if text.is_empty() {
            return Err(anyhow!(tr!(
                "Cursor n’a rien répondu",
                "Cursor gave no answer"
            )));
        }
        Ok(text)
    };
    let got = asked.await;
    p.kill();
    p.wait_gone(std::time::Duration::from_secs(5)).await;
    let _ = std::fs::remove_dir_all(&home);
    got
}

/// The models of an account none of whose agents runs (ESC-186): an `agent acp` of its own, in a
/// configuration folder of its own removed after (Cursor writes its model and its privacy in its
/// `cli-config.json` as it starts, seen on the real 2026.10.22: never the user's), asked
/// `initialize` then `cursor/list_available_models`, which answers without a session (tried on
/// 2026.10.22: 46 models): no conversation is left in its store. Killed once it answered.
async fn read_models(
    program: Program,
    env: Vec<(String, String)>,
    cwd: &std::path::Path,
) -> Result<Value> {
    let home =
        std::env::temp_dir().join(format!("escouade-cursor-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&home)?;
    let mut env = env;
    env.retain(|(k, _)| k != CONFIG_VAR);
    env.push((CONFIG_VAR.into(), home.to_string_lossy().into_owned()));
    let opts = process::Opts {
        program,
        cwd: cwd.to_string_lossy().into_owned(),
        args: vec!["acp".into()],
        env,
        start: super::Start::New,
        settings: Settings::default(),
        instructions: None,
        relay: None,
        escouade: None,
        fence: Vec::new(),
    };
    let p = CursorProcess::spawn(opts, Box::new(|_| {}), Box::new(|_, _| {}))?;
    let limit = std::time::Duration::from_secs(60);
    let asked = async {
        p.request(
            "initialize",
            json!({
                "protocolVersion": process::PROTOCOL,
                "clientCapabilities": {
                    "fs": { "readTextFile": false, "writeTextFile": false },
                    "terminal": false,
                },
                "clientInfo": { "name": "escouade", "version": env!("CARGO_PKG_VERSION") },
            }),
            limit,
        )
        .await?;
        p.request("cursor/list_available_models", json!({}), limit)
            .await
    };
    let got = asked.await;
    p.kill();
    p.wait_gone(std::time::Duration::from_secs(5)).await;
    let _ = std::fs::remove_dir_all(&home);
    got
}

/// The models of `cursor/list_available_models` (`options`) as those of `session/new` would be
/// (`availableModels`, by their base ids), for `models_of`.
pub(crate) fn listed_of(options: &Value) -> Value {
    let models: Vec<Value> = options["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| Some(json!({ "modelId": m["value"].as_str()?, "name": m["name"] })))
        .collect();
    json!({ "availableModels": models })
}

impl Engine for Cursor {
    /// Its sign-in is the machine's (`agent login`, not moved by its folder), and each agent has a
    /// folder of its own (`agent_home_var`); its other accounts go by their key (ESC-43).
    fn store(&self) -> crate::accounts::Store {
        crate::accounts::Store {
            home: ".cursor",
            var: CONFIG_VAR,
            files: &[],
            dirs: &[],
            login: &["login"],
            xdg: None,
        }
    }

    fn keys(&self) -> crate::accounts::KeyUse {
        crate::accounts::KeyUse::Added
    }

    /// An account added by its key goes by it, never by a token of the app's; its Principal by the
    /// machine's sign-in, never by a key or token of the app's (ESC-43).
    fn account_env(
        &self,
        account: &crate::model::Account,
        inherited: &dyn Fn(&str) -> bool,
    ) -> Vec<(String, String)> {
        use crate::accounts::{account_key, by_key, CURSOR_KEY_VAR, CURSOR_TOKEN_VAR};
        let key = account_key(account).filter(|_| by_key(account));
        let mut env: Vec<(String, String)> = [CURSOR_TOKEN_VAR, CURSOR_KEY_VAR]
            .into_iter()
            .filter(|v| inherited(v) && (*v == CURSOR_TOKEN_VAR || key.is_none()))
            .map(|v| (v.to_string(), String::new()))
            .collect();
        if let Some(key) = key {
            env.push((CURSOR_KEY_VAR.to_string(), key));
        }
        env
    }

    fn key_missing(&self, name: &str) -> String {
        tr!(
            "Le compte Cursor « {name} » n’a pas de clé d’API : donne-la dans Réglages › Comptes, « Changer la clé… ».",
            "The Cursor account “{name}” has no API key: give it in Settings › Accounts, “Change the key…”."
        )
    }

    fn key_malformed(&self) -> String {
        tr!(
            "Ce n’est pas une clé d’API de Cursor : copie-la entière depuis cursor.com, Settings › API Keys.",
            "That isn’t a Cursor API key: copy it whole from cursor.com, Settings › API Keys."
        )
    }

    /// Its Principal signed in by `agent login`, the same for all its folders, never read; another
    /// account by its key, kept in the keychain (ESC-43), shown by its end.
    fn account_status<'a>(
        &'a self,
        account: &'a crate::model::Account,
        settings: &'a crate::model::Settings,
    ) -> BoxFuture<'a, crate::accounts::AccountStatus> {
        Box::pin(async move {
            let machine = if crate::accounts::by_key(account) {
                (false, None)
            } else {
                (crate::accounts::cursor_signed_in(), None)
            };
            crate::accounts::key_status(account, settings, machine)
        })
    }

    fn workflow_folders(&self) -> crate::workflow::Folders {
        &[(".cursor/commands", crate::workflow::DocKind::Command)]
    }

    /// No cost told; in its own interface, neither its subagents nor the time of its steps.
    fn track_missing(&self, view: bool) -> &'static [&'static str] {
        if view {
            &["subagents", "cost", "timing"]
        } else {
            &["cost"]
        }
    }

    fn id(&self) -> EngineId {
        EngineId::CURSOR
    }

    /// `cursor/list_available_models` of an `agent acp` of its own (`read_models`, ESC-186).
    fn list_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<Option<Vec<ToldModel>>>> {
        Box::pin(async move {
            let options = read_models(program, env, dir).await?;
            let told = models_of(&listed_of(&options), Some(&options));
            Ok(Some(catalog(&told)))
        })
    }

    fn name(&self) -> &'static str {
        "Cursor CLI"
    }

    fn speaker(&self) -> &'static str {
        "Cursor"
    }

    /// Before the first message: no command line to fit in.
    fn instructions_room(&self) -> Option<Room> {
        None
    }

    /// A subagent of Cursor (its `task` tool) takes no message once it runs.
    fn subagent_relay(&self, _: Lang, _: &str, _: &str, _: &str) -> Option<String> {
        None
    }

    fn locate(&self, configured: &str, _wsl_distro: &str) -> Option<Program> {
        locate::find(configured)
    }

    fn tested(&self) -> Tested {
        TESTED
    }

    fn oldest_reason(&self, lang: Lang) -> String {
        tr_in!(
            lang,
            "son ACP n’a été vérifié qu’à partir de celle-ci (docs/TODO-1.8.md, « Cursor CLI »)",
            "its ACP was only checked from that one on (docs/TODO-1.8.md, “Cursor CLI”)"
        )
    }

    fn update_command(&self) -> &'static str {
        "agent update"
    }

    fn version_args(&self) -> &'static [&'static str] {
        &["--version"]
    }

    fn caps(&self) -> EngineCaps {
        caps()
    }

    fn accounts(&self) -> super::registry::AccountStore {
        super::registry::AccountStore::ConfigDir { var: CONFIG_VAR }
    }

    fn agent_home_var(&self) -> Option<&'static str> {
        Some(CONFIG_VAR)
    }

    fn terminal(&self) -> Option<&'static dyn super::TerminalView> {
        Some(&terminal::CURSOR_TERMINAL)
    }

    fn install(&self) -> Install {
        INSTALL
    }

    fn models(&self) -> Vec<ModelDecl> {
        MODELS
            .iter()
            .map(|&(id, label)| ModelDecl {
                id,
                label,
                effort: false,
                auto: false,
            })
            .collect()
    }

    fn efforts(&self) -> &'static [&'static str] {
        &EFFORTS
    }

    /// Auto picks its model and its effort itself.
    fn supports_effort(&self, model: &str) -> bool {
        !model.is_empty() && model != AUTO
    }

    /// Escouade's mode Auto is none of Cursor's: a new agent starts in « Demander ».
    fn supports_auto(&self, _model: &str) -> bool {
        false
    }

    fn spawn(
        &self,
        spawn: Spawn,
        on_frame: super::OnFrame,
        on_exit: super::OnExit,
    ) -> Result<Process> {
        let escouade = match &spawn.escouade {
            Escouade::Granted { url, token, .. } => Some((url.clone(), token.clone())),
            Escouade::Denied { .. } => None,
        };
        log::info!(
            "{}: starting {} acp in {} (account {})",
            spawn.label,
            spawn.program.describe(),
            spawn.cwd,
            spawn.account
        );
        let fence = crate::fence::spelled(&spawn.fence, &spawn.program);
        let opts = process::Opts {
            program: spawn.program,
            cwd: spawn.cwd,
            args: vec!["acp".into()],
            env: spawn.env,
            start: spawn.start,
            settings: Settings {
                model: spawn.model,
                effort: spawn.effort,
                mode: spawn.mode,
            },
            instructions: spawn.append_prompt,
            relay: spawn.relay,
            escouade,
            fence,
        };
        let p: Process = CursorProcess::spawn(opts, on_frame, on_exit)?;
        Ok(p)
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        message(text, attachments).map(Message)
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        rt.cursor_frame(frame, fx);
    }

    /// A session that cannot be loaded gives a new one (`process::open`), said by the conversation.
    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(one_shot(ask))
    }

    fn writer(&self, _weight: Weight) -> &'static str {
        "Cursor (Auto)"
    }

    /// Its session is given the server (`mcpServers`): no file.
    fn grant_file(&self, _url: &str, _token: &str) -> Option<Vec<u8>> {
        None
    }

    fn refusal(&self) -> Option<Value> {
        None
    }

    fn escouade_launch(&self, _escouade: &Escouade) -> mcp::Launch {
        mcp::Launch {
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    /// Its ACP reads no `/compact` (its prompt's `/` commands are `/copy-request-id` and the
    /// user's own, read in 2026.10.22): its interface's `/summarize` alone compacts (ESC-205).
    fn compaction(&self) -> super::Compaction {
        super::Compaction::InItsInterface("/summarize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded(file: &str) -> Vec<Value> {
        bench::recorded(file)
    }

    #[test]
    fn each_mode_of_escouade_has_its_mode_in_cursor_and_says_there_is_no_sandbox() {
        assert_eq!(acp_mode("plan"), "plan");
        for mode in ["default", "acceptEdits", "bypassPermissions", "auto"] {
            assert_eq!(acp_mode(mode), "agent", "{mode}");
        }
        for mode in MODES {
            let (fr, en) = (meaning(Lang::Fr, mode), meaning(Lang::En, mode));
            assert!(fr != en && fr.contains("bac à sable") && en.contains("sandbox"));
        }
        let c = caps();
        assert!(!c.modes.iter().any(|m| m.id == "ask" || m.id == "auto"));
        assert_eq!(c.fork, ForkPoint::None);
        assert_eq!(c.mid_turn, MidTurn::Queued);
    }

    #[test]
    fn the_models_of_the_account_are_listed_with_their_names_and_efforts() {
        let new = recorded("init-session.jsonl")
            .into_iter()
            .find(|m| m["result"]["models"].is_object())
            .unwrap();
        let options = recorded("init-session.jsonl")
            .into_iter()
            .find(|m| m["result"]["models"].is_array())
            .unwrap();
        let told = models_of(&new["result"]["models"], Some(&options["result"]));
        assert_eq!(told.len(), 46);
        assert_eq!(
            (told[0].base.as_str(), told[0].full.as_str()),
            (AUTO, "default[]")
        );
        let grok = told.iter().find(|m| m.base == "grok-4.7").unwrap();
        assert_eq!(grok.name, "Grok 4.7");
        assert_eq!(grok.effort_key.as_deref(), Some("reasoning_effort"));
        assert_eq!(grok.efforts, ["low", "medium", "high", "xhigh"]);
        assert_eq!(grok.default_effort.as_deref(), Some("high"));
        let menu = catalog(&told);
        assert!(menu[0].default && menu[0].label == "Auto");
        assert_eq!(menu.iter().filter(|m| m.default).count(), 1);
        // Listed nowhere: Auto alone.
        assert_eq!(catalog(&[])[0].id, AUTO);
        assert_eq!(full_id(AUTO), "default[]");
        assert_eq!(base_of("composer-2.5[fast=true]"), "composer-2.5");
    }

    #[test]
    fn a_question_is_answered_by_the_ids_of_its_choices_and_a_written_answer_skips_it() {
        let params = json!({ "questions": [
            { "id": "q1", "prompt": "Couleur ?", "options": [
                { "id": "r", "label": "Rouge" }, { "id": "b", "label": "Bleu" } ], "allowMultiple": false },
            { "id": "q2", "prompt": "Fruits ?", "options": [
                { "id": "p", "label": "Pomme" }, { "id": "o", "label": "Orange" } ], "allowMultiple": true },
        ] });
        let answered = question_outcome(
            &params,
            &json!({ "Couleur ?": "Bleu", "Fruits ?": "Pomme, Orange" }),
        );
        assert_eq!(
            answered,
            json!({ "outcome": "answered", "answers": [
                { "questionId": "q1", "selectedOptionIds": ["b"] },
                { "questionId": "q2", "selectedOptionIds": ["p", "o"] },
            ] })
        );
        let written = question_outcome(&params, &json!({ "Couleur ?": "Vert d’eau" }));
        assert_eq!(written["outcome"], "skipped");
        assert_eq!(written["reason"], "Couleur ? Vert d’eau");
    }

    #[test]
    fn a_permission_is_answered_by_the_option_of_its_kind() {
        let asked = recorded("full-turn.jsonl")
            .into_iter()
            .find(|m| m["method"] == "session/request_permission")
            .unwrap();
        assert_eq!(option_of(&asked["params"], "allow_once"), "allow-once");
        assert_eq!(option_of(&asked["params"], "allow_always"), "allow-always");
        assert_eq!(option_of(&asked["params"], "reject_once"), "reject-once");
        assert_eq!(option_of(&json!({}), "allow_once"), "allow-once");
    }

    #[test]
    fn a_message_takes_its_images_and_text_files_and_refuses_a_pdf() {
        let file = |name: &str, media: &str, data: &str| Attachment {
            name: name.into(),
            media_type: media.into(),
            data: data.into(),
        };
        let m = message(
            "Regarde",
            &[
                file("a.png", "image/png", "iVBORw0KGgo="),
                file("notes.txt", "text/plain", "bonjour"),
            ],
        )
        .unwrap();
        assert_eq!(
            m["prompt"],
            json!([
                { "type": "image", "mimeType": "image/png", "data": "iVBORw0KGgo=" },
                { "type": "text", "text": "notes.txt:\n\nbonjour" },
                { "type": "text", "text": "Regarde" },
            ])
        );
        assert!(message("x", &[file("a.pdf", "application/pdf", "JVBERi0=")]).is_err());
    }

    #[test]
    fn escouades_server_goes_by_the_session_never_the_command_line() {
        let granted = CURSOR.escouade_launch(&Escouade::Granted {
            url: "http://127.0.0.1:1/mcp".into(),
            token: "s3cr3t".into(),
            file: None,
        });
        assert!(granted.args.is_empty() && granted.env.is_empty());
        assert!(CURSOR.grant_file("http://x", "s3cr3t").is_none());
        // Its session takes it, named `escouade`, the token in its header.
        let given = mcp_servers(Some(&("http://127.0.0.1:1/mcp".into(), "s3cr3t".into())));
        assert_eq!(given[0]["name"], "escouade");
        assert_eq!(given[0]["type"], "http");
        assert_eq!(given[0]["headers"][0]["value"], "Bearer s3cr3t");
        assert_eq!(mcp_servers(None), json!([]));
        assert_eq!(CURSOR.agent_home_var(), Some("CURSOR_CONFIG_DIR"));
        assert!(preamble(" Fais court. ").contains("<instructions>\nFais court.\n</instructions>"));
    }

    /// The texts Cursor answers in place of a turn (read in its code, 2026.10.22), as the last
    /// paragraph of the turn's words; each said in both languages.
    #[test]
    fn the_refusals_of_cursor_are_read_from_the_last_words_of_the_turn() {
        assert_eq!(refusal(UPGRADE), Some(Refusal::Upgrade));
        assert_eq!(
            refusal("Je regarde.\n\nAdd a payment method to continue"),
            Some(Refusal::Payment)
        );
        assert_eq!(
            refusal("\n\nPlease sign in to continue"),
            Some(Refusal::SignIn)
        );
        assert_eq!(
            refusal("\n\nError: [unauthenticated] Backend rejected authentication. Verify…"),
            Some(Refusal::Unauthenticated)
        );
        // Said in passing, it is no refusal.
        assert_eq!(
            refusal("Le message « Upgrade your plan to continue » vient de Cursor."),
            None
        );
        assert_eq!(refusal(""), None);
        assert!(at_limit(Refusal::Upgrade) && at_limit(Refusal::Payment));
        assert!(!at_limit(Refusal::SignIn) && !at_limit(Refusal::Unauthenticated));
        for r in [
            Refusal::Upgrade,
            Refusal::Payment,
            Refusal::SignIn,
            Refusal::Unauthenticated,
        ] {
            for (model, key) in [(AUTO, false), ("composer-2.5", true)] {
                let (fr, en) = (
                    refusal_error(Lang::Fr, r, model, key),
                    refusal_error(Lang::En, r, model, key),
                );
                assert!(fr != en && !fr.is_empty(), "{r:?}");
            }
        }
        assert!(refusal_error(Lang::Fr, Refusal::Upgrade, "composer-2.5", false).contains("Auto"));
        assert_ne!(write_not_done(Lang::Fr), write_not_done(Lang::En));
    }
}
