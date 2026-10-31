//! Mistral Vibe, the sixth harness: a `vibe-acp` process per agent, driven over ACP v1 by a client
//! of Escouade's own (`process`), its updates translated by `AgentRt::vibe_frame`
//! (`agent/vibe.rs`), which hands most of them to Cursor's translation (both speak ACP).
//!
//! What it does as Escouade drives it comes from the real runs of Mistral Vibe 3.0.3 (ESC-265,
//! docs/TODO-1.8.md, point 0, « Mistral Vibe »), whose streams are in `tests/fixtures/vibe/`.
//!
//! Each agent has a `VIBE_HOME` of its own (`Engine::agent_home_var`), as Cursor's
//! `CURSOR_CONFIG_DIR`: Vibe's « Toujours » writes its rule there (`config.toml`), its sessions are
//! kept there, and Escouade writes there the trust of the agent's folder (`trusted_folders.toml`),
//! never in the user's `~/.vibe`. Its API key stays where Vibe keeps it (the system's keychain, or
//! `MISTRAL_API_KEY`), never read by Escouade.
//!
//! Vibe has no mode Plan: Escouade's is played by its process (`process`), the plan approved on
//! the card « Plan à approuver ». Its `smart-approve` (a classifier, as Claude Code's Auto) is not
//! offered: a session of a new `VIBE_HOME` had it once in two in the real runs.
//!
//! Its accounts (ESC-267): the machine's goes by the key `vibe --setup` kept (or `MISTRAL_API_KEY`),
//! another by an API key of its own, kept in the system's keychain and given by `MISTRAL_API_KEY`.
//! A turn's cost and tokens are what its session's grew by; no quota is told (its usage is on
//! Mistral's console), and a rate limit ends the turn « limité », resumed a minute later. The texts
//! written go to its interface's `vibe -p`, without a tool, within a price.

#[cfg(test)]
pub(crate) mod bench;
mod locate;
mod process;
pub(crate) mod terminal;

use process::VibeProcess;

use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
    ToldModel,
};
use super::{
    mcp, BoxFuture, Engine, EngineProcess, Escouade, Message, OneShot, Process, Program, Room,
    Spawn, StartLost, Weight,
};
use crate::agent::{AgentRt, Effects};
use crate::core::{size_label, Attachment, MAX_IMAGE, MAX_TOTAL};
use crate::i18n::{self, Lang};
use crate::model::EngineId;
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

/// Mistral Vibe.
pub struct Vibe;

pub static VIBE: Vibe = Vibe;

/// The versions of Mistral Vibe Escouade was tested with: as the real run of 2026-10-28 found it
/// (ESC-265).
pub const TESTED: Tested = Tested {
    oldest: "3.0.3",
    newest: "3.0.3",
};

/// Its installer (its README): a tool of `uv`.
const INSTALL: Install = Install {
    windows: "uv tool install mistral-vibe",
    unix: "uv tool install mistral-vibe",
};

/// The variable that moves all it keeps (its config, its sessions, its trust, its hooks).
pub(crate) const HOME_VAR: &str = "VIBE_HOME";

/// Its default model, the only one of an account in the real run.
pub(crate) const DEFAULT: &str = "mistral-medium-3.5";

/// Where an account's usage is seen: Mistral's console (Vibe tells no quota, ESC-267).
pub(crate) const USAGE_PAGE: &str = "https://admin.mistral.ai/organization/usage";

/// The most a text written may cost (`--max-price`, in dollars): a question of a few words costs
/// about 0.01 $ (its system prompt weighs 7 500 to 9 000 tokens); Vibe stops after the turn that
/// passes it.
const TEXT_MAX_PRICE: &str = "0.05";

/// The models of its menu until a process of the account has listed them.
const MODELS: [(&str, &str); 1] = [(DEFAULT, "Mistral Medium 3.5")];

/// The efforts Escouade's menu offers: its thinking `off` (« Bas ») or `high` (« Élevé »).
const EFFORTS: [&str; 2] = ["low", "high"];

/// What every prompt says in Escouade's mode Plan (Vibe has none): the model plans without
/// changing anything, its last words the plan.
pub(crate) const PLAN_MODE: &str = "<escouade_plan_mode>\nYou are in plan mode: study what is asked and propose a plan, without changing anything (no file written, no command that writes). End your turn with the whole plan, in Markdown, as your last words: the user approves it before any change.\n</escouade_plan_mode>";

/// The request of its process's own that asks the plan of a turn in the mode Plan approved.
pub(crate) const PLAN_REQUEST: &str = "escouade/plan";

/// What an approved plan is followed by.
pub(crate) const PLAN_GO: &str = "The plan is approved: carry it out now.";

/// The model, effort and mode in force.
#[derive(Debug, Clone, Default)]
pub(crate) struct Settings {
    pub model: String,
    pub effort: String,
    pub mode: String,
}

/// The mode of Vibe (its agent) for Escouade's `mode`: `plan` is `ask`, its writes refused by the
/// process.
pub(crate) fn vibe_mode(mode: &str) -> &'static str {
    match mode {
        "acceptEdits" => "accept-edits",
        "bypassPermissions" => "auto-approve",
        _ => "ask",
    }
}

/// The modes Escouade gives Vibe, in the menu's order.
const MODES: [&str; 4] = ["default", "plan", "acceptEdits", "bypassPermissions"];

/// What a mode means in Vibe, as the menu says it: nothing is confined, only the permissions
/// protect.
fn meaning(lang: Lang, mode: &str) -> String {
    match mode {
        "plan" => tr_in!(
            lang,
            "Vibe n’a pas de mode Plan : Escouade lui demande d’étudier et de proposer un plan, et refuse ses écritures. Le plan s’approuve sur sa carte, puis Vibe le met en œuvre. Pas de bac à sable : ses commandes demandent.",
            "Vibe has no Plan mode: Escouade asks it to study and propose a plan, and refuses its writes. The plan is approved on its card, then Vibe carries it out. No sandbox: its commands ask."
        ),
        "acceptEdits" => tr_in!(
            lang,
            "Son agent accept-edits : ses écritures de fichiers passent sans demander, même hors du dossier ; ses commandes hors de sa liste (cat, ls, git status…) et les outils MCP demandent. Pas de bac à sable.",
            "Its accept-edits agent: its file writes go through without asking, even outside the folder; its commands off its list (cat, ls, git status…) and MCP tools ask. No sandbox."
        ),
        "bypassPermissions" => tr_in!(
            lang,
            "Son agent auto-approve : Vibe fait tout sans rien demander, commandes, fichiers et outils MCP, sur tout le disque. Avec un projet privé, Vibe reste dans son agent ask et Escouade accepte tout ce qui ne touche pas ses chemins. Pas de bac à sable.",
            "Its auto-approve agent: Vibe does everything without asking, commands, files and MCP tools, on the whole disk. With a private project, Vibe stays in its ask agent and Escouade accepts all that doesn’t touch its paths. No sandbox."
        ),
        _ => tr_in!(
            lang,
            "Son agent ask : ses écritures dans le dossier de l’agent passent sans demander, ailleurs elles demandent avec leur diff ; ses commandes hors de sa liste (cat, ls, git status… passent) et les outils MCP demandent. Il lit sans demander. Pas de bac à sable : une commande acceptée peut tout faire.",
            "Its ask agent: its writes in the agent’s folder go through without asking, elsewhere they ask with their diff; its commands off its list (cat, ls, git status… go through) and MCP tools ask. It reads without asking. No sandbox: an accepted command can do anything."
        ),
    }
}

/// What Vibe does, as Escouade drives it.
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
        // Its thinking, off or high.
        effort: true,
        images: true,
        pdf: false,
        // `session/fork` copies all of the session.
        fork: ForkPoint::Turn,
        // Its session is in the agent's own `VIBE_HOME`, whatever the account (only the key changes).
        resume_other_account: true,
        mid_turn: MidTurn::Queued,
        slash_commands: false,
        // No quota told: its usage is on Mistral's console (`usage_page`).
        quota: QuotaKind::None,
        quota_windows: Vec::new(),
        // `usage_update.cost`, in dollars (ESC-267).
        cost: CostKind::Exact,
        // Its interface `vibe` in a terminal (`terminal`, ESC-268).
        terminal_view: true,
        remote_control: false,
        background_tasks: false,
        todo_tools: false,
        default_refusal: false,
        sandbox: false,
        usage_page: Some(USAGE_PAGE),
        accounts_by_key: true,
        read_only: false,
    }
}

/// The value `id` of the options `options` (`configOptions` of `session/new`) is set to.
pub(crate) fn option_value<'a>(options: &'a Value, id: &str) -> Option<&'a str> {
    options
        .as_array()?
        .iter()
        .find(|o| o["id"] == id)?
        .get("currentValue")?
        .as_str()
}

/// The values the option `id` offers.
fn option_values(options: &Value, id: &str) -> Vec<(String, String)> {
    options
        .as_array()
        .into_iter()
        .flatten()
        .find(|o| o["id"] == id)
        .and_then(|o| o["options"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| {
            let value = v["value"].as_str()?.to_string();
            let name = v["name"].as_str().unwrap_or(&value).to_string();
            Some((value, name))
        })
        .collect()
}

/// The thinking levels a session offers (`thinking`: `off`, `high`).
pub(crate) fn thinking_levels(options: &Value) -> Vec<String> {
    option_values(options, "thinking")
        .into_iter()
        .map(|(v, _)| v)
        .collect()
}

/// The thinking level for Escouade's `effort` among `levels`: `low` is `off`, any other its
/// highest; None without an effort or levels.
pub(crate) fn thinking_of(effort: &str, levels: &[String]) -> Option<String> {
    if effort.is_empty() || levels.is_empty() {
        return None;
    }
    let off = levels.iter().find(|l| *l == "off");
    match effort {
        "low" => off.cloned(),
        _ => levels.iter().rev().find(|l| *l != "off").cloned(),
    }
}

/// The models a session offers (`configOptions`, its `model`), each with Escouade's efforts when
/// it has a thinking level to choose.
pub(crate) fn models_of(options: &Value) -> Vec<ToldModel> {
    let current = option_value(options, "model");
    let thinks = thinking_levels(options).len() > 1;
    option_values(options, "model")
        .into_iter()
        .map(|(id, label)| ToldModel {
            from_config: false,
            default: Some(id.as_str()) == current,
            id,
            label,
            efforts: if thinks {
                EFFORTS.iter().map(|e| e.to_string()).collect()
            } else {
                Vec::new()
            },
            default_effort: thinks.then(|| "high".to_string()),
            images: true,
            provider: None,
            multiplier: None,
            price: None,
        })
        .collect()
}

/// The composer's menu of the models `told`, else Vibe's default one.
pub(crate) fn catalog(told: &[ToldModel]) -> Vec<ToldModel> {
    let mut models = told.to_vec();
    if models.is_empty() {
        models = MODELS
            .iter()
            .map(|&(id, label)| ToldModel {
                from_config: false,
                id: id.into(),
                label: label.into(),
                efforts: EFFORTS.iter().map(|e| e.to_string()).collect(),
                default_effort: Some("high".into()),
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

/// The parameters of `initialize`: the questions asked for (`elicitation.form`: without it, Vibe's
/// model has no `ask_user_question`, tried); `fs` and `terminal` not offered (Vibe reads, writes
/// and runs by itself).
pub(crate) fn hello() -> Value {
    json!({
        "protocolVersion": process::PROTOCOL,
        "clientCapabilities": {
            "fs": { "readTextFile": false, "writeTextFile": false },
            "terminal": false,
            "elicitation": { "form": {} },
        },
        "clientInfo": { "name": "escouade", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// The questions of an `elicitation/create` (its `requestedSchema`), as the card shows them: each
/// by its key (`q0`…), its text, its choices, and whether it takes several.
pub(crate) fn elicitation_questions(params: &Value) -> Vec<(String, Value)> {
    let schema = &params["requestedSchema"];
    let mut keys: Vec<(&String, &Value)> = schema["properties"]
        .as_object()
        .into_iter()
        .flatten()
        .collect();
    // `q0`, `q1`… in their order, `q10` after `q9`.
    keys.sort_by_key(|(k, _)| {
        (
            k.trim_start_matches('q').parse::<u32>().unwrap_or(u32::MAX),
            k.to_string(),
        )
    });
    keys.into_iter()
        .map(|(key, p)| {
            let multi = p["type"] == "array";
            let choices = if multi {
                &p["items"]["anyOf"]
            } else {
                &p["oneOf"]
            };
            let options: Vec<Value> = choices
                .as_array()
                .into_iter()
                .flatten()
                .map(|c| {
                    let label = c["title"].as_str().or(c["const"].as_str()).unwrap_or("");
                    json!({ "label": label, "description": "" })
                })
                .collect();
            let text = p["description"]
                .as_str()
                .or(p["title"].as_str())
                .or(params["message"].as_str())
                .unwrap_or_default();
            (
                key.clone(),
                json!({
                    "question": text, "header": "", "multiSelect": multi, "options": options,
                }),
            )
        })
        .collect()
}

/// The `content` of the answer to an `elicitation/create` from a card's `answers` (by question,
/// several choices joined by « , »): each by its key, a choice by its value (`const`), several as
/// a list; a written answer as it is.
pub(crate) fn elicitation_content(params: &Value, answers: &Value) -> Value {
    let props = &params["requestedSchema"]["properties"];
    let mut content = serde_json::Map::new();
    for (key, q) in elicitation_questions(params) {
        let text = q["question"].as_str().unwrap_or_default();
        let answer = answers[text]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string();
        let p = &props[&key];
        let multi = p["type"] == "array";
        let choices = if multi {
            &p["items"]["anyOf"]
        } else {
            &p["oneOf"]
        };
        let value_of = |label: &str| {
            choices
                .as_array()
                .into_iter()
                .flatten()
                .find(|c| c["title"].as_str() == Some(label) || c["const"].as_str() == Some(label))
                .and_then(|c| c["const"].as_str())
                .unwrap_or(label)
                .to_string()
        };
        let value = if multi {
            json!(answer
                .split(", ")
                .filter(|a| !a.is_empty())
                .map(value_of)
                .collect::<Vec<_>>())
        } else {
            json!(value_of(&answer))
        };
        content.insert(key, value);
    }
    Value::Object(content)
}

/// Refused: the ACP of the process is not the one Escouade speaks.
fn protocol_refused(lang: Lang, version: &Value) -> String {
    tr_in!(
        lang,
        "Mistral Vibe parle ACP {version}, Escouade ACP {ours} : mets Mistral Vibe à jour (uv tool upgrade mistral-vibe).",
        "Mistral Vibe speaks ACP {version}, Escouade ACP {ours}: update Mistral Vibe (uv tool upgrade mistral-vibe).",
        ours = process::PROTOCOL
    )
}

/// No session opened for the agent: what Vibe said (no API key, say).
fn session_refused(lang: Lang, e: &anyhow::Error) -> String {
    tr_in!(
        lang,
        "Mistral Vibe n’a pas ouvert la session de l’agent : {e:#}. S’il n’a pas de clé d’API, lance vibe --setup dans un terminal.",
        "Mistral Vibe didn’t open the agent’s session: {e:#}. If it has no API key, run vibe --setup in a terminal."
    )
}

/// The trust of the agent's folder in its own `VIBE_HOME` (`trusted_folders.toml`): Vibe then
/// reads the project's `.vibe/` (its hooks, skills and agents), as the other harnesses read their
/// project's config. Its path as Vibe writes it, a literal string of TOML (tried: `C:\…`, not
/// `C:/…`).
pub(crate) fn trust(home: &std::path::Path, cwd: &str) -> std::io::Result<()> {
    let Some(file) = trusted_folders(cwd) else {
        return Ok(());
    };
    std::fs::create_dir_all(home)?;
    std::fs::write(home.join(TRUSTED_FOLDERS), file)
}

/// The file of the folders Vibe trusts, in its `VIBE_HOME`.
pub(crate) const TRUSTED_FOLDERS: &str = "trusted_folders.toml";

/// What `trusted_folders.toml` holds to trust `cwd` alone; None for a path a literal string of
/// TOML cannot hold.
pub(crate) fn trusted_folders(cwd: &str) -> Option<String> {
    let path = if cfg!(windows) {
        cwd.replace('/', "\\")
    } else {
        cwd.to_string()
    };
    (!path.contains('\'')).then(|| format!("trusted = ['{path}']\n"))
}

/// The file tools of Vibe the paths of a private project are refused to, by their `denylist` in
/// the agent's `config.toml` (read in Vibe 3.0.3: a path matching it is `NEVER`, whatever the mode).
const FENCED_TOOLS: [&str; 3] = ["read_file", "write_file", "edit"];

/// `path` as a pattern of Python's `fnmatch` that matches it alone (its `*`, `?` and `[` literal).
fn literal_glob(path: &str) -> String {
    let mut out = String::new();
    for c in path.chars() {
        match c {
            '*' | '?' | '[' => {
                out.push('[');
                out.push(c);
                out.push(']');
            }
            c => out.push(c),
        }
    }
    out
}

/// « Projet privé » (ESC-140): the paths of `fence` (as Vibe names them) refused to Vibe's file
/// tools in the agent's own `config.toml`, each and all under it, even in its agent `auto-approve`;
/// the refusal taken out again when there is none. The rest of the file (the rules its « Toujours »
/// wrote) is kept as it is.
pub(crate) fn fence_config(home: &std::path::Path, fence: &[String]) -> Result<()> {
    let file = home.join("config.toml");
    let before = std::fs::read_to_string(&file).unwrap_or_default();
    let after = fenced_config(&before, fence)?;
    if after != before {
        std::fs::create_dir_all(home)?;
        std::fs::write(&file, after)?;
    }
    Ok(())
}

/// The agent's `config.toml` (`before`) with the refusals of `fence_config`.
pub(crate) fn fenced_config(before: &str, fence: &[String]) -> Result<String> {
    let mut doc: toml_edit::DocumentMut = before.parse()?;
    let patterns: Vec<String> = fence
        .iter()
        .map(|p| p.trim_end_matches(['/', '\\']))
        .filter(|p| !p.is_empty())
        .flat_map(|p| {
            let sep = if p.contains('\\') { '\\' } else { '/' };
            let p = literal_glob(p);
            [p.clone(), format!("{p}{sep}*")]
        })
        .collect();
    for tool in FENCED_TOOLS {
        if patterns.is_empty() {
            if let Some(t) = doc
                .get_mut("tools")
                .and_then(|t| t.get_mut(tool))
                .and_then(|t| t.as_table_like_mut())
            {
                t.remove("denylist");
            }
            continue;
        }
        let mut list = toml_edit::Array::new();
        for p in &patterns {
            list.push(p.as_str());
        }
        // Tables of their own (`[tools.read_file]`), as Vibe writes them.
        if doc.get("tools").is_none() {
            doc["tools"] = toml_edit::table();
        }
        if doc["tools"].get(tool).is_none() {
            doc["tools"][tool] = toml_edit::table();
        }
        doc["tools"][tool]["denylist"] = toml_edit::value(list);
    }
    if let Some(t) = doc.get_mut("tools").and_then(|t| t.as_table_mut()) {
        t.set_implicit(true);
    }
    Ok(doc.to_string())
}

/// A message to Vibe: its text, the images joined to it (ACP's `image` blocks), the text files
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
                "« {name} » ({t}) ne peut pas être joint : Mistral Vibe lit les images et les fichiers texte, pas les PDF ni les autres fichiers.",
                "“{name}” ({t}) can’t be attached: Mistral Vibe reads images and text files, not PDFs nor other files.",
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

/// A `VIBE_HOME` of its own for a process that keeps nothing (a text written, the models read),
/// in place of any in `env`.
fn throwaway_home(env: &mut Vec<(String, String)>) -> Result<std::path::PathBuf> {
    let home =
        std::env::temp_dir().join(format!("escouade-vibe-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&home)?;
    env.retain(|(k, _)| k != HOME_VAR);
    env.push((HOME_VAR.into(), home.to_string_lossy().into_owned()));
    Ok(home)
}

/// The arguments of `vibe -p` for a text written: no tool (`--enabled-tools` names none it has, so
/// all are off), one turn, within `TEXT_MAX_PRICE`, its answer alone on its output; its question read
/// from its input (`-p` without a text).
pub(crate) fn print_args() -> Vec<String> {
    [
        "--enabled-tools",
        "none",
        "--max-turns",
        "1",
        "--max-price",
        TEXT_MAX_PRICE,
        "--output",
        "text",
        "-p",
    ]
    .map(String::from)
    .to_vec()
}

/// The question as `vibe -p` reads it: it takes no system prompt of its own.
fn print_prompt(system: &str, prompt: &str) -> String {
    if system.trim().is_empty() {
        return prompt.to_string();
    }
    format!(
        "<instructions>
{system}
</instructions>

{prompt}"
    )
}

/// One question to its interface's `vibe -p` (ESC-267), beside the `vibe-acp` found, on the
/// account's key, in a `VIBE_HOME` of its own removed once answered: what it printed. Vibe stops at
/// its price (code 3) after its turn: its answer is taken all the same.
async fn one_shot(ask: OneShot<'_>) -> Result<String> {
    let mut env = ask.env.clone();
    let home = throwaway_home(&mut env)?;
    env.push(("VIBE_ENABLE_TELEMETRY".into(), "false".into()));
    let program = match ask.program {
        Program::Native(path) => Program::Native(locate::interface_of(&path)),
        other => other,
    };
    let question = print_prompt(ask.system, ask.prompt);
    let out = super::oneshot::run(
        &program,
        ask.cwd,
        &print_args(),
        &env,
        &question,
        "Mistral Vibe",
    )
    .await;
    let _ = std::fs::remove_dir_all(&home);
    let text = String::from_utf8_lossy(&out?).trim().to_string();
    if text.is_empty() {
        bail!(tr!(
            "Mistral Vibe n’a rien répondu (sa clé d’API, sa limite ?)",
            "Mistral Vibe gave no answer (its API key, its limit?)"
        ));
    }
    Ok(text)
}

/// The models of an account none of whose agents runs: a `vibe-acp` of its own, in a `VIBE_HOME`
/// of its own removed after, asked `initialize` then `session/new` (its `configOptions`), the
/// session closed at once (`session/close`). Killed once it answered.
async fn read_models(
    program: Program,
    env: Vec<(String, String)>,
    cwd: &std::path::Path,
) -> Result<Value> {
    let mut env = env;
    let home = throwaway_home(&mut env)?;
    let opts = process::Opts {
        program,
        cwd: cwd.to_string_lossy().into_owned(),
        env,
        start: super::Start::New,
        settings: Settings::default(),
        instructions: None,
        escouade: None,
        fence: Vec::new(),
    };
    let p = VibeProcess::spawn(opts, Box::new(|_| {}), Box::new(|_, _| {}))?;
    let limit = std::time::Duration::from_secs(60);
    let asked = async {
        p.request("initialize", hello(), limit).await?;
        let opened = p
            .request(
                "session/new",
                json!({ "cwd": cwd.to_string_lossy(), "mcpServers": [] }),
                limit,
            )
            .await?;
        if let Some(id) = opened["sessionId"].as_str() {
            let _ = p
                .request("session/close", json!({ "sessionId": id }), limit)
                .await;
        }
        Ok::<Value, anyhow::Error>(opened)
    };
    let got = asked.await;
    p.kill();
    p.wait_gone(std::time::Duration::from_secs(5)).await;
    let _ = std::fs::remove_dir_all(&home);
    got
}

/// The variables that keep Vibe from updating itself and from checking for updates (its
/// `config.toml` settings, each also a `VIBE_<FIELD>` variable).
const NO_SELF_UPDATE: [(&str, &str); 2] = [
    ("VIBE_ENABLE_AUTO_UPDATE", "false"),
    ("VIBE_ENABLE_UPDATE_CHECKS", "false"),
];

impl Engine for Vibe {
    /// Its API key is the machine's (the system's keychain, set by `vibe --setup`), not moved by
    /// its folder, and each agent has a `VIBE_HOME` of its own (`agent_home_var`); its other
    /// accounts go by their key (ESC-267).
    fn store(&self) -> crate::accounts::Store {
        crate::accounts::Store {
            home: ".vibe",
            var: HOME_VAR,
            files: &[],
            dirs: &[],
            login: &["--setup"],
            xdg: None,
        }
    }

    fn keys(&self) -> crate::accounts::KeyUse {
        crate::accounts::KeyUse::Added
    }

    /// An account added by its key goes by it, before the keychain's and a key of the app's; its
    /// Principal as Vibe goes in the user's terminal (`MISTRAL_API_KEY` when set, else the key
    /// `vibe --setup` kept).
    fn account_env(
        &self,
        account: &crate::model::Account,
        _inherited: &dyn Fn(&str) -> bool,
    ) -> Vec<(String, String)> {
        use crate::accounts::{account_key, by_key, MISTRAL_KEY_VAR};
        account_key(account)
            .filter(|_| by_key(account))
            .map(|key| (MISTRAL_KEY_VAR.to_string(), key))
            .into_iter()
            .collect()
    }

    fn key_missing(&self, name: &str) -> String {
        tr!(
            "Le compte Mistral Vibe « {name} » n’a pas de clé d’API : donne-la dans Réglages › Comptes, « Changer la clé… ».",
            "The Mistral Vibe account “{name}” has no API key: give it in Settings › Accounts, “Change the key…”."
        )
    }

    fn key_malformed(&self) -> String {
        tr!(
            "Ce n’est pas une clé d’API de Mistral : copie-la entière depuis console.mistral.ai, API Keys.",
            "That isn’t a Mistral API key: copy it whole from console.mistral.ai, API Keys."
        )
    }

    /// Its Principal goes by the key `vibe --setup` kept in the keychain, never read: signed in as
    /// far as can be told. Another account by its key, shown by its end.
    fn account_status<'a>(
        &'a self,
        account: &'a crate::model::Account,
        settings: &'a crate::model::Settings,
    ) -> BoxFuture<'a, crate::accounts::AccountStatus> {
        Box::pin(async move {
            if !crate::accounts::by_key(account) {
                let dir = crate::accounts::config_dir(account, settings);
                return crate::accounts::AccountStatus {
                    connected: true,
                    stamp: None,
                    email: None,
                    dir: dir.to_string_lossy().into_owned(),
                };
            }
            crate::accounts::key_status(account, settings, (false, None))
        })
    }

    /// In the agent's own `VIBE_HOME`, whatever its account (only its key changes, ESC-267).
    fn sessions_in_agent_home(&self) -> bool {
        true
    }

    fn id(&self) -> EngineId {
        EngineId::VIBE
    }

    fn list_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<Option<Vec<ToldModel>>>> {
        Box::pin(async move {
            let opened = read_models(program, env, dir).await?;
            Ok(Some(catalog(&models_of(&opened["configOptions"]))))
        })
    }

    fn name(&self) -> &'static str {
        "Mistral Vibe"
    }

    fn speaker(&self) -> &'static str {
        "Vibe"
    }

    /// Before the first message: no command line to fit in.
    fn instructions_room(&self) -> Option<Room> {
        None
    }

    /// Its subagents (`agents.spawn`) take no message once they run.
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
            "son ACP n’a été vérifié qu’à partir de celle-ci (docs/TODO-1.8.md, « Mistral Vibe »)",
            "its ACP was only checked from that one on (docs/TODO-1.8.md, “Mistral Vibe”)"
        )
    }

    fn update_command(&self) -> &'static str {
        "uv tool upgrade mistral-vibe"
    }

    fn version_args(&self) -> &'static [&'static str] {
        &["--version"]
    }

    fn caps(&self) -> EngineCaps {
        caps()
    }

    fn accounts(&self) -> super::registry::AccountStore {
        super::registry::AccountStore::ConfigDir { var: HOME_VAR }
    }

    fn agent_home_var(&self) -> Option<&'static str> {
        Some(HOME_VAR)
    }

    fn no_self_update(&self) -> &'static [(&'static str, &'static str)] {
        &NO_SELF_UPDATE
    }

    fn terminal(&self) -> Option<&'static dyn super::TerminalView> {
        Some(&terminal::VIBE_TERMINAL)
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
                effort: true,
                auto: false,
            })
            .collect()
    }

    fn efforts(&self) -> &'static [&'static str] {
        &EFFORTS
    }

    fn supports_effort(&self, _model: &str) -> bool {
        true
    }

    /// Its `smart-approve` is not offered: a new agent starts in « Demander ».
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
            "{}: starting {} in {} (account {})",
            spawn.label,
            spawn.program.describe(),
            spawn.cwd,
            spawn.account
        );
        let fence = crate::fence::spelled(&spawn.fence, &spawn.program);
        // The trust of its folder and the refusals of its fence in its own `VIBE_HOME`, never the
        // user's. A fence that cannot be written keeps the agent from starting.
        if let Some((_, home)) = spawn.env.iter().find(|(k, _)| k == HOME_VAR) {
            let home = std::path::Path::new(home);
            if let Err(e) = trust(home, &spawn.cwd) {
                log::warn!("vibe: the trust of {} not written: {e}", spawn.cwd);
            }
            // The hooks of its view are its interface's alone (`terminal`).
            terminal::forget_hooks(home);
            fence_config(home, &fence).map_err(|e| {
                anyhow!(tr!(
                    "Les chemins des projets privés n’ont pas pu être refusés à Mistral Vibe ({e}) : l’agent ne démarre pas.",
                    "The private projects’ paths couldn’t be refused to Mistral Vibe ({e}): the agent doesn’t start."
                ))
            })?;
        } else if !fence.is_empty() {
            bail!(tr!(
                "Mistral Vibe n’a pas de dossier de configuration où refuser les chemins des projets privés : l’agent ne démarre pas.",
                "Mistral Vibe has no configuration folder where to refuse the private projects’ paths: the agent doesn’t start."
            ));
        }
        let mut env = spawn.env;
        env.push(("VIBE_ENABLE_TELEMETRY".into(), "false".into()));
        let opts = process::Opts {
            program: spawn.program,
            cwd: spawn.cwd,
            env,
            start: spawn.start,
            settings: Settings {
                model: spawn.model,
                effort: spawn.effort,
                mode: spawn.mode,
            },
            instructions: spawn.append_prompt,
            escouade,
            fence,
        };
        let p: Process = VibeProcess::spawn(opts, on_frame, on_exit)?;
        Ok(p)
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        message(text, attachments).map(Message)
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        rt.vibe_frame(frame, fx);
    }

    /// A session that cannot be loaded gives a new one (`process::open`), said by the conversation.
    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(one_shot(ask))
    }

    fn writer(&self, _weight: Weight) -> &'static str {
        "Mistral Vibe"
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

    /// `/compact` is one of the commands its ACP reads in a prompt (`available_commands_update`).
    fn compaction(&self) -> super::Compaction {
        super::Compaction::Message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded(file: &str) -> Vec<Value> {
        bench::recorded(file)
    }

    #[test]
    fn each_mode_of_escouade_has_its_agent_in_vibe_and_says_there_is_no_sandbox() {
        assert_eq!(vibe_mode("default"), "ask");
        assert_eq!(vibe_mode("plan"), "ask");
        assert_eq!(vibe_mode("acceptEdits"), "accept-edits");
        assert_eq!(vibe_mode("bypassPermissions"), "auto-approve");
        for mode in MODES {
            let (fr, en) = (meaning(Lang::Fr, mode), meaning(Lang::En, mode));
            assert!(fr != en && fr.contains("bac à sable") && en.contains("sandbox"));
        }
        let c = caps();
        assert!(!c.modes.iter().any(|m| m.id == "auto"));
        assert!(c.modes.iter().any(|m| m.id == "plan"));
        assert_eq!(c.fork, ForkPoint::Turn);
        assert!(!VIBE.supports_auto(DEFAULT));
    }

    #[test]
    fn the_models_and_thinking_of_a_session_make_the_menu() {
        let new = recorded("init-session.jsonl")
            .into_iter()
            .find(|m| m["result"]["configOptions"].is_array())
            .unwrap();
        let options = &new["result"]["configOptions"];
        let told = models_of(options);
        assert_eq!(told.len(), 1);
        assert_eq!(
            (told[0].id.as_str(), told[0].label.as_str()),
            (DEFAULT, "Mistral Medium 3.5")
        );
        assert!(told[0].default);
        assert_eq!(told[0].efforts, ["low", "high"]);
        assert_eq!(option_value(options, "mode"), Some("accept-edits"));
        let levels = thinking_levels(options);
        assert_eq!(levels, ["off", "high"]);
        assert_eq!(thinking_of("low", &levels).as_deref(), Some("off"));
        assert_eq!(thinking_of("high", &levels).as_deref(), Some("high"));
        assert_eq!(thinking_of("medium", &levels).as_deref(), Some("high"));
        assert_eq!(thinking_of("", &levels), None);
        assert_eq!(catalog(&[])[0].id, DEFAULT);
    }

    #[test]
    fn a_question_is_answered_by_the_keys_and_values_of_its_form() {
        let asked = recorded("question-elicitation.jsonl")
            .into_iter()
            .find(|m| m["method"] == "elicitation/create")
            .unwrap();
        let qs = elicitation_questions(&asked["params"]);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].0, "q0");
        assert_eq!(qs[0].1["question"], "blue or red?");
        assert_eq!(qs[0].1["options"][1]["label"], "red");
        assert_eq!(
            elicitation_content(&asked["params"], &json!({ "blue or red?": "red" })),
            json!({ "q0": "red" })
        );
        // A written answer, as it is (Vibe takes it).
        assert_eq!(
            elicitation_content(&asked["params"], &json!({ "blue or red?": "vert" })),
            json!({ "q0": "vert" })
        );
        let multi = json!({ "requestedSchema": { "properties": {
            "q0": { "description": "Fruits ?", "type": "array",
                    "items": { "anyOf": [ { "const": "p", "title": "Pomme" }, { "const": "o", "title": "Orange" } ] } },
        } } });
        assert_eq!(elicitation_questions(&multi)[0].1["multiSelect"], true);
        assert_eq!(
            elicitation_content(&multi, &json!({ "Fruits ?": "Pomme, Orange" })),
            json!({ "q0": ["p", "o"] })
        );
    }

    #[test]
    fn escouades_server_goes_by_the_session_never_the_command_line() {
        let granted = VIBE.escouade_launch(&Escouade::Granted {
            url: "http://127.0.0.1:1/mcp".into(),
            token: "s3cr3t".into(),
            file: None,
        });
        assert!(granted.args.is_empty() && granted.env.is_empty());
        assert!(VIBE.grant_file("http://x", "s3cr3t").is_none());
        assert_eq!(VIBE.agent_home_var(), Some("VIBE_HOME"));
        // The questions asked for.
        assert_eq!(
            hello()["clientCapabilities"]["elicitation"],
            json!({ "form": {} })
        );
    }

    #[test]
    fn the_trust_of_the_agents_folder_is_written_in_its_own_home() {
        let dir = crate::paths::test_dir("esc266-vibe-trust");
        let home = dir.join("home");
        trust(&home, "C:/x/repo").unwrap();
        let written = std::fs::read_to_string(home.join("trusted_folders.toml")).unwrap();
        if cfg!(windows) {
            assert_eq!(written, "trusted = ['C:\\x\\repo']\n");
        } else {
            assert_eq!(written, "trusted = ['C:/x/repo']\n");
        }
        // A path TOML's literal string cannot hold: nothing written.
        let other = dir.join("other");
        trust(&other, "C:/it's").unwrap();
        assert!(!other.join("trusted_folders.toml").exists());
    }

    #[test]
    fn a_fence_is_refused_to_the_file_tools_in_the_agents_config_and_taken_out_after() {
        let dir = crate::paths::test_dir("esc266-vibe-fence-config");
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        // What its « Toujours » wrote is kept.
        let rules = "[tools.bash]\nallowlist = [\"npm\"]\n";
        std::fs::write(home.join("config.toml"), rules).unwrap();
        fence_config(&home, &["C:\\x\\privé[1]".into(), "/srv/secret/".into()]).unwrap();
        let written = std::fs::read_to_string(home.join("config.toml")).unwrap();
        let doc: toml_edit::DocumentMut = written.parse().unwrap();
        assert_eq!(doc["tools"]["bash"]["allowlist"][0].as_str(), Some("npm"));
        for tool in FENCED_TOOLS {
            let list: Vec<&str> = doc["tools"][tool]["denylist"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|v| v.as_str())
                .collect();
            assert_eq!(
                list,
                [
                    "C:\\x\\privé[[]1]",
                    "C:\\x\\privé[[]1]\\*",
                    "/srv/secret",
                    "/srv/secret/*"
                ],
                "{tool}"
            );
        }
        // No fence any more: its refusals taken out, the rest as it was.
        fence_config(&home, &[]).unwrap();
        let after = std::fs::read_to_string(home.join("config.toml")).unwrap();
        assert!(!after.contains("denylist"), "{after}");
        assert!(after.contains("allowlist = [\"npm\"]"));
        // No fence and no file: none written.
        let bare = dir.join("bare");
        fence_config(&bare, &[]).unwrap();
        assert!(!bare.join("config.toml").exists());
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
        assert_eq!(m["prompt"].as_array().unwrap().len(), 3);
        assert!(message("x", &[file("a.pdf", "application/pdf", "JVBERi0=")]).is_err());
    }
}
