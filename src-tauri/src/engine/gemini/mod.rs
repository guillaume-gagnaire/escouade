//! Gemini CLI, the eighth harness. Since INT-10.7 its conversation goes through its plugin (`dev.escouade.gemini-cli`,
//! `plugins::harness::BUILTIN_HARNESSES`): the client of `crates/escouade-acp` with its extension `gemini` speaks ACP v1 to
//! a `gemini --acp` per agent, whose launch the core checks and starts. Here stay its capabilities, its accounts, its
//! models, its texts (`gemini -p`), its terminal view, and what the core writes in the agent's home (`prepare`).
//!
//! What it does as Escouade drives it comes from the study of Gemini CLI 0.66.0 without an account
//! (ESC-273, docs/TODO-1.8.md, point 0, « Gemini CLI »): its documentation, its installed code, and
//! the real binary against a fake Gemini API, whose streams are in `tests/fixtures/gemini/`. No turn
//! with a real model was ever run: what an account would change is said there.
//!
//! Each agent has a `GEMINI_CLI_HOME` of its own (`Engine::agent_home_var`, the folder Gemini makes
//! its `.gemini` in), never the user's `~/.gemini`, shared with Antigravity CLI and never read nor
//! written: its sessions are kept there, and Escouade writes there its `settings.json` (no update,
//! no statistics, how it signs in) and the rules of a private project (`--policy`). Its folder is
//! trusted by `GEMINI_CLI_TRUST_WORKSPACE` (without it, the MCP servers of the session are left out
//! without a word). Its sign-in stays where Gemini keeps it (the system's keychain, or
//! `GEMINI_API_KEY`), never read by Escouade.
//!
//! What it lacks, and how Escouade makes up for it:
//! - no questions in ACP (`ask_user` is left out): the first message tells the model to end its
//!   turn on a block `<escouade_question>`, which the process takes out of the text and shows as
//!   the card of a question, its answers the next message;
//! - no task list (`plan` is never sent): its mode Plan writes a plan in Markdown, approved on the
//!   card « Plan à approuver » (its `exit_plan_mode` asks a permission, the plan read from its file);
//! - its change of mode is a text (« [MODE_UPDATE] default »): read as such, left out of the thread;
//! - no copy of a session, no effort to set from ACP, no cost (tokens alone), no quota.

#[cfg(test)]
pub(crate) mod bench;
mod locate;
pub(crate) mod terminal;
#[cfg(test)]
mod tests;

use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
    ToldModel,
};
use super::{
    mcp, BoxFuture, Engine, Escouade, Message, OneShot, Process, Program, Room, Spawn, StartLost,
    Weight,
};
use crate::agent::{AgentRt, Effects};
use crate::core::{size_label, Attachment, MAX_IMAGE, MAX_TOTAL};
use crate::i18n::{self, Lang};
use crate::model::EngineId;
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Gemini CLI.
pub struct Gemini;

pub static GEMINI: Gemini = Gemini;

/// The versions of Gemini CLI Escouade was tested with: the one of the study (ESC-273).
pub const TESTED: Tested = Tested {
    oldest: "0.66.0",
    newest: "0.66.0",
};

/// Its installer (its README): npm's package, or Homebrew.
const INSTALL: Install = Install {
    windows: "npm install -g @google/gemini-cli",
    unix: "npm install -g @google/gemini-cli",
};

/// The variable that moves all it keeps: the folder it makes its `.gemini` in.
pub(crate) const HOME_VAR: &str = "GEMINI_CLI_HOME";

/// The variables of the app's that would take the place of an account's sign-in: its API keys,
/// Vertex AI (ESC-275). An account's own, its key or its Google sign-in, is given in their place.
pub(crate) const KEY_VARS: [&str; 4] = [
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "GOOGLE_GENAI_USE_VERTEXAI",
    CREDENTIALS_VAR,
];

/// The variable of the key an account added by its key goes by.
pub(crate) const KEY_VAR: &str = KEY_VARS[0];

/// The file of credentials Gemini reads after its home's own (its code, 0.66.0:
/// `fetchCachedCredentialsList`): an agent's process is given its account's Google sign-in so,
/// from the agent's own home (ESC-275).
pub(crate) const CREDENTIALS_VAR: &str = "GOOGLE_APPLICATION_CREDENTIALS";

/// Where `gemini` keeps the Google sign-in it made in `home` (its `GEMINI_CLI_HOME`): never read by
/// Escouade, only looked for.
pub(crate) fn sign_in_file(home: &Path) -> PathBuf {
    home.join(".gemini").join("oauth_creds.json")
}

/// The address of the Google account `gemini` signed in to in `home` (`google_accounts.json`, its
/// `active`): an address, no secret.
pub(crate) fn signed_in_email(home: &Path) -> Option<String> {
    let text = std::fs::read_to_string(home.join(".gemini").join("google_accounts.json")).ok()?;
    let doc: Value = serde_json::from_str(&text).ok()?;
    doc["active"]
        .as_str()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(str::to_string)
}

/// The Google sign-in a process of Gemini wrote in the agent's own home (its tokens refreshed)
/// taken away before it starts again: its account's (`CREDENTIALS_VAR`) is the one it reads, the
/// account it goes by now, whichever it went by before. Never read.
pub(crate) fn forget_sign_in(home: &Path) {
    let _ = std::fs::remove_file(sign_in_file(home));
}

/// Whether the app's environment gives Gemini a key or Vertex AI: the machine's account then goes
/// by it (its Google sign-in, in `~/.gemini`, is never given).
pub(crate) fn machine_has_key() -> bool {
    let set = |k: &str| std::env::var(k).is_ok_and(|v| !v.trim().is_empty());
    set("GEMINI_API_KEY")
        || set("GOOGLE_API_KEY")
        || std::env::var("GOOGLE_GENAI_USE_VERTEXAI").is_ok_and(|v| v.eq_ignore_ascii_case("true"))
}

/// Where its usage and its quotas are told: Gemini tells them in its interface alone (`/stats`).
pub(crate) const USAGE_PAGE: &str = "https://aistudio.google.com/usage";

#[cfg(test)]
use escouade_acp::gemini::{
    escouade_mode, is_plan_exit, limit_of, mode_update, plan_file, DAILY_RESUME_MS, RATE_RESUME_MS,
};

/// Its default model: its router, which picks Pro or Flash at each turn.
pub(crate) const DEFAULT: &str = "auto";

/// The models of its menu until a process of the account has listed them.
const MODELS: [(&str, &str); 1] = [(DEFAULT, "Auto")];

/// The mode of Gemini for Escouade's `mode`.
#[cfg(test)]
pub(crate) fn gemini_mode(mode: &str) -> &'static str {
    match mode {
        "plan" => "plan",
        "acceptEdits" => "autoEdit",
        "bypassPermissions" => "yolo",
        _ => "default",
    }
}

/// The modes Escouade gives Gemini, in the menu's order.
const MODES: [&str; 4] = ["default", "plan", "acceptEdits", "bypassPermissions"];

/// What a mode means in Gemini, as the menu says it.
fn meaning(lang: Lang, mode: &str) -> String {
    match mode {
        "plan" => tr_in!(
            lang,
            "Son mode plan : Gemini lit et cherche sans rien changer, écrit son plan dans le dossier de l’agent, puis demande de l’approuver sur sa carte ; approuvé, il repasse en « Demander ». Pas de bac à sable.",
            "Its plan mode: Gemini reads and searches without changing anything, writes its plan in the agent’s folder, then asks to approve it on its card; approved, it goes back to “Ask”. No sandbox."
        ),
        "acceptEdits" => tr_in!(
            lang,
            "Son mode autoEdit : ses écritures de fichiers passent sans demander ; ses commandes et les outils MCP demandent. Pas de bac à sable.",
            "Its autoEdit mode: its file writes go through without asking; its commands and MCP tools ask. No sandbox."
        ),
        "bypassPermissions" => tr_in!(
            lang,
            "Son mode yolo : Gemini fait tout sans rien demander, commandes, fichiers, sous-agents et outils MCP, sur tout le disque. Les chemins d’un projet privé restent refusés par ses règles. Pas de bac à sable.",
            "Its yolo mode: Gemini does everything without asking, commands, files, subagents and MCP tools, on the whole disk. A private project’s paths stay refused by its rules. No sandbox."
        ),
        _ => tr_in!(
            lang,
            "Son mode default : il lit sans demander ; ses écritures dans le dossier de l’agent passent par les règles d’Escouade, ailleurs elles demandent avec leur diff ; ses commandes et les outils MCP demandent. Ses sous-agents sont refusés. Pas de bac à sable : une commande acceptée peut tout faire.",
            "Its default mode: it reads without asking; its writes in the agent’s folder go by Escouade’s rules, elsewhere they ask with their diff; its commands and MCP tools ask. Its subagents are refused. No sandbox: an accepted command can do anything."
        ),
    }
}

/// What Gemini does, as Escouade drives it.
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
        // Its thinking is set in its settings at its start only (`modelConfigs`): not offered.
        effort: false,
        images: true,
        pdf: false,
        // No `session/fork`.
        fork: ForkPoint::None,
        // Its session is in the agent's own `GEMINI_CLI_HOME`, whatever the account.
        resume_other_account: true,
        mid_turn: MidTurn::Queued,
        slash_commands: false,
        quota: QuotaKind::None,
        quota_windows: Vec::new(),
        // Its tokens alone.
        cost: CostKind::None,
        // « Vue Gemini » (ESC-275, `terminal`).
        terminal_view: true,
        remote_control: false,
        background_tasks: false,
        todo_tools: false,
        default_refusal: false,
        sandbox: false,
        // Its usage and quotas are told in its interface alone (`/stats`), and on its page for a key.
        usage_page: Some(USAGE_PAGE),
        // An account goes by its key or its Google sign-in, as it was added (`Account::api_key`).
        accounts_by_key: false,
        // A discussion of the Docs tab: its writes and commands refused by its rules.
        read_only: true,
    }
}

/// The models a session offers (`models` of `session/new`: `availableModels`, `currentModelId`).
pub(crate) fn models_of(models: &Value) -> Vec<ToldModel> {
    let current = models["currentModelId"].as_str();
    models["availableModels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let id = m["modelId"].as_str()?.to_string();
            let label = m["name"].as_str().unwrap_or(&id).to_string();
            Some(ToldModel {
                from_config: false,
                default: Some(id.as_str()) == current,
                id,
                label,
                efforts: Vec::new(),
                default_effort: None,
                images: true,
                provider: None,
                multiplier: None,
                price: None,
            })
        })
        .collect()
}

/// The composer's menu of the models `told`, else its router alone.
pub(crate) fn catalog(told: &[ToldModel]) -> Vec<ToldModel> {
    let mut models = told.to_vec();
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

/// The parameters of `initialize`: neither `fs` nor `terminal` offered (Gemini reads, writes and
/// runs by itself, its permissions still asked).
pub(crate) fn hello() -> Value {
    json!({
        "protocolVersion": escouade_acp::PROTOCOL,
        "clientCapabilities": {
            "fs": { "readTextFile": false, "writeTextFile": false },
            "terminal": false,
        },
        "clientInfo": { "name": "escouade", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// The tools of Gemini that read, search or write files: those the rules of a private project
/// refuse when their arguments name its paths.
const FILE_TOOLS: [&str; 7] = [
    "read_file",
    "read_many_files",
    "write_file",
    "replace",
    "list_directory",
    "glob",
    "grep_search",
];

/// The tools a discussion of the Docs tab is refused (it changes nothing).
const WRITE_TOOLS: [&str; 3] = ["write_file", "replace", "run_shell_command"];

/// `path` as a regular expression of JavaScript that finds it in a text, whatever its separators
/// (one or more, `\` or `/`: the arguments are seen as JSON, a `\` doubled) and, under Windows, its
/// case.
pub(crate) fn path_pattern(path: &str) -> String {
    let mut out = String::new();
    let mut sep = false;
    for c in path.trim_end_matches(['/', '\\']).chars() {
        if c == '/' || c == '\\' {
            if !sep {
                out.push_str(r"[\\/]+");
            }
            sep = true;
            continue;
        }
        sep = false;
        if cfg!(windows) && c.is_ascii_alphabetic() {
            out.push('[');
            out.push(c.to_ascii_lowercase());
            out.push(c.to_ascii_uppercase());
            out.push(']');
        } else if r"\^$.|?*+()[]{}/".contains(c) {
            out.push('\\');
            out.push(c);
        } else {
            out.push(c);
        }
    }
    out
}

/// A TOML string holding `s` as it is.
fn toml_string(s: &str) -> String {
    toml_edit::Value::from(s).to_string().trim().to_string()
}

/// A TOML list of `items`.
fn toml_list(items: &[&str]) -> String {
    let list: Vec<String> = items.iter().map(|t| toml_string(t)).collect();
    format!("[{}]", list.join(", "))
}

/// The rules of Gemini's policy engine (`--policy`) for an agent: the paths of `fence` refused to
/// its file tools and its commands, in every mode, `yolo` included (tried, ESC-273); and for a
/// discussion (`read_only`), its writes and commands refused. None when there is none to give.
pub(crate) fn policy(fence: &[String], read_only: bool) -> Option<String> {
    let mut out = String::new();
    let paths: Vec<String> = fence
        .iter()
        .map(|p| path_pattern(p))
        .filter(|p| !p.is_empty())
        .collect();
    if !paths.is_empty() {
        let any = format!("(?:{})", paths.join("|"));
        let message = toml_string("Refused by Escouade: a path of a private project.");
        out.push_str(&format!(
            "[[rule]]\ntoolName = {}\nargsPattern = {}\ndecision = \"deny\"\npriority = 900\ndenyMessage = {message}\n\n",
            toml_list(&FILE_TOOLS),
            toml_string(&any)
        ));
        out.push_str(&format!(
            "[[rule]]\ntoolName = \"run_shell_command\"\ncommandRegex = {}\ndecision = \"deny\"\npriority = 900\ndenyMessage = {message}\n\n",
            toml_string(&format!(".*{any}.*"))
        ));
    }
    if read_only {
        out.push_str(&format!(
            "[[rule]]\ntoolName = {}\ndecision = \"deny\"\npriority = 950\ndenyMessage = {}\n\n",
            toml_list(&WRITE_TOOLS),
            toml_string(
                "Refused by Escouade: this conversation changes no file and runs no command."
            )
        ));
    }
    (!out.is_empty()).then_some(out)
}

/// The file of the agent's rules, in its home.
pub(crate) const POLICY_FILE: &str = "escouade-policy.toml";

/// The rules of `policy` written in the agent's home, their file to give `--policy`; the file taken
/// out when there are none.
pub(crate) fn write_policy(home: &Path, rules: Option<&str>) -> std::io::Result<Option<PathBuf>> {
    let file = home.join(POLICY_FILE);
    match rules {
        Some(rules) => {
            std::fs::create_dir_all(home)?;
            std::fs::write(&file, rules)?;
            Ok(Some(file))
        }
        None => {
            if file.exists() {
                std::fs::remove_file(&file)?;
            }
            Ok(None)
        }
    }
}

/// The harness of the plugin Gemini CLI (`dev.escouade.gemini-cli`, INT-10.7), if it is on: the conversation of Gemini's
/// agents goes through it (`plugins::harness::BUILTIN_HARNESSES`).
#[cfg(test)]
pub fn plugged() -> Option<&'static crate::plugins::harness::ProcessEngine> {
    crate::plugins::harness::plugged(crate::plugins::builtin::GEMINI)
}

/// What the core writes in the agent's own `GEMINI_CLI_HOME` before its launch (`plugins::harness::Prepare`), never in
/// the user's: its settings, how it signs in, and the rules of a private project or a discussion, given by `--policy`.
/// Rules that cannot be written keep the agent from starting.
pub(crate) fn prepare(p: &crate::plugins::harness::Preparing) -> Result<Vec<String>> {
    let rules = policy(p.fence, p.read_only);
    let Some(home) = p.home else {
        if rules.is_none() {
            return Ok(Vec::new());
        }
        bail!(tr!(
            "Gemini CLI n’a pas de dossier de configuration où écrire les règles de l’agent : l’agent ne démarre pas.",
            "Gemini CLI has no configuration folder where to write the agent’s rules: the agent doesn’t start."
        ));
    };
    // Its account's sign-in, not the one it refreshed here before (ESC-275).
    forget_sign_in(home);
    if let Err(e) = write_settings(home, auth_type(p.env)) {
        log::warn!(
            "gemini: the settings of {} not written: {e}",
            home.display()
        );
    }
    let file = write_policy(home, rules.as_deref()).map_err(|e| {
        anyhow!(tr!(
            "Les règles de l’agent n’ont pas pu être écrites pour Gemini CLI ({e}) : l’agent ne démarre pas.",
            "The agent’s rules couldn’t be written for Gemini CLI ({e}): the agent doesn’t start."
        ))
    })?;
    Ok(file
        .map(|file| vec!["--policy".to_string(), file.to_string_lossy().into_owned()])
        .unwrap_or_default())
}

/// How Gemini signs in, from the variables of its process (`env`, then the app's): its API key or
/// Vertex AI when they are given, else the Google account of the machine (its keychain).
pub(crate) fn auth_type(env: &[(String, String)]) -> &'static str {
    let var = |k: &str| {
        env.iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.clone())
            .or_else(|| std::env::var(k).ok())
            .filter(|v| !v.trim().is_empty())
    };
    if var("GOOGLE_GENAI_USE_VERTEXAI").is_some_and(|v| v.eq_ignore_ascii_case("true")) {
        "vertex-ai"
    } else if var("GEMINI_API_KEY").is_some() {
        "gemini-api-key"
    } else {
        "oauth-personal"
    }
}

/// The agent's `settings.json` (`<home>/.gemini/settings.json`) with what Escouade sets: no update
/// nor its notice, no statistics, and how it signs in (its account's, whatever it went by before,
/// ESC-275). What else
/// it holds (Gemini writes there too) is kept.
pub(crate) fn write_settings(home: &Path, auth: &str) -> Result<()> {
    let dir = home.join(".gemini");
    let file = dir.join("settings.json");
    let mut doc: Value = std::fs::read_to_string(&file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let before = doc.clone();
    let mut set = |path: &[&str], v: Value, keep: bool| {
        let mut at = &mut doc;
        for k in &path[..path.len() - 1] {
            if !at[*k].is_object() {
                at[*k] = json!({});
            }
            at = &mut at[*k];
        }
        let last = path[path.len() - 1];
        if !(keep && at.get(last).is_some_and(|x| !x.is_null())) {
            at[last] = v;
        }
    };
    set(&["general", "enableAutoUpdate"], json!(false), false);
    set(
        &["general", "enableAutoUpdateNotification"],
        json!(false),
        false,
    );
    set(&["privacy", "usageStatisticsEnabled"], json!(false), false);
    set(&["security", "auth", "selectedType"], json!(auth), false);
    // What its interface's view left (its hooks, Escouade's server) is not its process's.
    terminal::forget_view(&mut doc);
    if doc != before || !file.exists() {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&file, serde_json::to_string_pretty(&doc)?)?;
    }
    Ok(())
}

/// The variables every process of it gets: its folder trusted (else the MCP servers of its session
/// are left out without a word, tried).
pub(crate) const TRUST: (&str, &str) = ("GEMINI_CLI_TRUST_WORKSPACE", "true");

/// A message to Gemini: its text, the images joined to it (ACP's `image` blocks), the text files
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
                "« {name} » ({t}) ne peut pas être joint : Gemini CLI lit les images et les fichiers texte, pas les PDF ni les autres fichiers.",
                "“{name}” ({t}) can’t be attached: Gemini CLI reads images and text files, not PDFs nor other files.",
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

/// A `GEMINI_CLI_HOME` of its own for a process that keeps nothing (a text written, the models
/// read), in place of any in `env`, with its settings.
fn throwaway_home(env: &mut Vec<(String, String)>) -> Result<PathBuf> {
    let home =
        std::env::temp_dir().join(format!("escouade-gemini-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&home)?;
    write_settings(&home, auth_type(env))?;
    env.retain(|(k, _)| k != HOME_VAR);
    env.push((HOME_VAR.into(), home.to_string_lossy().into_owned()));
    env.push((TRUST.0.into(), TRUST.1.into()));
    Ok(home)
}

/// A line of its ACP.
fn acp_line(id: u64, method: &str, params: Value) -> Vec<u8> {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
    let mut out = msg.to_string().into_bytes();
    out.push(b'\n');
    out
}

/// The result of the request `id` among the lines of `read`, what else it says left aside.
async fn acp_answer<R: tokio::io::AsyncBufRead + Unpin>(
    read: &mut tokio::io::Lines<R>,
    id: u64,
) -> Result<Value> {
    while let Some(line) = read.next_line().await? {
        let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if msg["id"].as_u64() != Some(id) || msg.get("method").is_some() {
            continue;
        }
        if let Some(e) = msg.get("error").filter(|e| !e.is_null()) {
            bail!("{}", e["message"].as_str().unwrap_or("error"));
        }
        return Ok(msg["result"].clone());
    }
    bail!("gemini --acp left before it answered")
}

/// `models` of its `session/new`: a `gemini --acp` of its own, in its throwaway `home`, asked `initialize` then
/// `session/new`, its writes and commands refused. Killed once it answered.
async fn session_models(
    program: &Program,
    env: &[(String, String)],
    cwd: &Path,
    home: &Path,
) -> Result<Value> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let dir = cwd.to_string_lossy().into_owned();
    let mut args = vec!["--acp".to_string()];
    if let Some(file) = write_policy(home, policy(&[], true).as_deref())? {
        args.extend(["--policy".to_string(), file.to_string_lossy().into_owned()]);
    }
    let launch = program.launch(&dir, &args, env)?;
    let mut cmd = tokio::process::Command::new(&launch.program);
    cmd.args(&launch.args)
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    for (k, v) in &launch.env {
        cmd.env(k, v);
    }
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    crate::job::isolate(&mut cmd);
    crate::proc::no_inherited_fds(&mut cmd);
    let mut child = cmd.spawn()?;
    let job = crate::job::Job::for_child(&child);
    let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("stdout"))?;
    let mut lines = tokio::io::BufReader::new(stdout).lines();
    let asked = async {
        stdin.write_all(&acp_line(1, "initialize", hello())).await?;
        stdin.flush().await?;
        acp_answer(&mut lines, 1).await?;
        let params = json!({ "cwd": dir, "mcpServers": [] });
        stdin.write_all(&acp_line(2, "session/new", params)).await?;
        stdin.flush().await?;
        acp_answer(&mut lines, 2).await
    };
    let got = tokio::time::timeout(std::time::Duration::from_secs(60), asked)
        .await
        .map_err(|_| anyhow!("gemini --acp: no answer in time"))?;
    if let Some(job) = &job {
        job.terminate();
    }
    let _ = child.kill().await;
    Ok(got?["models"].clone())
}

/// The models of an account none of whose agents runs.
async fn read_models(
    program: Program,
    env: Vec<(String, String)>,
    cwd: &Path,
) -> Result<Vec<ToldModel>> {
    let mut env = env;
    let home = throwaway_home(&mut env)?;
    let got = session_models(&program, &env, cwd, &home).await;
    let _ = std::fs::remove_dir_all(&home);
    Ok(models_of(&got?))
}

/// The model of a text of `weight`: its smallest Flash for a short text, its Flash for one written
/// after reading (its aliases, which follow its newest models).
fn writer_model(weight: Weight) -> &'static str {
    match weight {
        Weight::Light => "flash-lite",
        Weight::Reading => "flash",
    }
}

/// The rules of a process that writes a text: every tool refused (`toolName = "*"`, which also
/// keeps them from the model's sight).
pub(crate) const NO_TOOLS: &str = "[[rule]]\ntoolName = \"*\"\ndecision = \"deny\"\npriority = 999\ndenyMessage = \"Refused by Escouade: this text is written without tools.\"\n";

/// The arguments of `gemini -p` for a text of `weight`, its rules in `policy`: its answer in JSON,
/// no extension, the question on its input (a command line holds none of it).
pub(crate) fn one_shot_args(weight: Weight, policy: &Path) -> Vec<String> {
    [
        "-p",
        "Answer the request above.",
        "-o",
        "json",
        "-m",
        writer_model(weight),
        "--approval-mode",
        "default",
        "-e",
        "none",
        "--policy",
    ]
    .iter()
    .map(|a| a.to_string())
    .chain([policy.to_string_lossy().into_owned()])
    .collect()
}

/// What `gemini -p -o json` answered: its `response`, else its `error`.
pub(crate) fn response_of(out: &[u8]) -> Result<String> {
    let text = String::from_utf8_lossy(out);
    let start = text.find('{').unwrap_or(text.len());
    let doc: Value = serde_json::from_str(text[start..].trim()).unwrap_or(Value::Null);
    if let Some(e) = doc["error"]["message"].as_str() {
        bail!("{e}");
    }
    let said = doc["response"].as_str().unwrap_or_default().trim();
    if said.is_empty() {
        bail!(tr!(
            "Gemini CLI n’a rien répondu (sa connexion, sa limite ?)",
            "Gemini CLI gave no answer (its sign-in, its limit?)"
        ));
    }
    Ok(said.to_string())
}

/// One question to `gemini -p` (ESC-275), in a throwaway home removed once answered, every tool
/// refused by its rules; the question on its input, its time bounded by `oneshot::LIMIT`.
async fn one_shot(ask: OneShot<'_>) -> Result<String> {
    let mut env = ask.env;
    let home = throwaway_home(&mut env)?;
    let policy = home.join(POLICY_FILE);
    if let Err(e) = std::fs::write(&policy, NO_TOOLS) {
        let _ = std::fs::remove_dir_all(&home);
        bail!("gemini: its tools not refused for a text: {e}");
    }
    let question = if ask.system.trim().is_empty() {
        ask.prompt.to_string()
    } else {
        format!(
            "<instructions>\n{}\n</instructions>\n\n{}",
            ask.system.trim(),
            ask.prompt
        )
    };
    let args = one_shot_args(ask.weight, &policy);
    let out =
        super::oneshot::run(&ask.program, ask.cwd, &args, &env, &question, "Gemini CLI").await;
    let _ = std::fs::remove_dir_all(&home);
    response_of(&out?)
}

/// Refused: a key given that is no Gemini API key (Gemini CLI's, Antigravity CLI's).
pub(crate) fn key_malformed() -> String {
    tr!(
        "Ce n’est pas une clé d’API Gemini : copie-la entière depuis aistudio.google.com, Get API key.",
        "That isn’t a Gemini API key: copy it whole from aistudio.google.com, Get API key."
    )
}

impl Engine for Gemini {
    /// An account is signed in to Google by its interface itself (`gemini`, no argument) in a
    /// terminal with its folder as its `GEMINI_CLI_HOME`, where `gemini` keeps the sign-in; or goes
    /// by its API key, in the keychain. Each agent has a home of its own (`agent_home_var`), its
    /// account's sign-in given by `GOOGLE_APPLICATION_CREDENTIALS` (ESC-275).
    fn store(&self) -> crate::accounts::Store {
        crate::accounts::Store {
            home: ".gemini",
            var: HOME_VAR,
            files: &[],
            dirs: &[],
            login: &[],
            xdg: None,
        }
    }

    /// The one it was added with, its key or its Google sign-in (ESC-275).
    fn keys(&self) -> crate::accounts::KeyUse {
        crate::accounts::KeyUse::Chosen
    }

    /// An account goes by its key (`GEMINI_API_KEY`) or by the Google sign-in `gemini` made in its
    /// folder (`GOOGLE_APPLICATION_CREDENTIALS`, read by `gemini` alone), never by a key or Vertex
    /// AI of the app's; the machine's account by what the app has (ESC-275).
    fn account_env(
        &self,
        account: &crate::model::Account,
        inherited: &dyn Fn(&str) -> bool,
    ) -> Vec<(String, String)> {
        use crate::accounts::{account_key, by_key, is_principal, own_dir};
        if is_principal(account) {
            return Vec::new();
        }
        let key = account_key(account).filter(|_| by_key(account));
        let own_var = match (&key, by_key(account)) {
            (Some(_), _) => Some(KEY_VAR),
            (None, false) => Some(CREDENTIALS_VAR),
            (None, true) => None,
        };
        let mut env: Vec<(String, String)> = KEY_VARS
            .into_iter()
            .filter(|v| inherited(v) && Some(*v) != own_var)
            .map(|v| (v.to_string(), String::new()))
            .collect();
        if let Some(key) = key {
            env.push((KEY_VAR.to_string(), key));
        } else if let (false, Some(dir)) = (by_key(account), own_dir(account)) {
            env.push((
                CREDENTIALS_VAR.to_string(),
                sign_in_file(Path::new(dir)).to_string_lossy().into_owned(),
            ));
        }
        env
    }

    /// Its account signed in to Google by `gemini` in its folder: without it, its agent would ask
    /// its sign-in where nobody sees it.
    fn sign_in_ready(&self, account: &crate::model::Account) -> Option<Result<()>> {
        use crate::accounts::{by_key, is_principal, own_dir};
        if is_principal(account) || by_key(account) {
            return None;
        }
        let signed = own_dir(account).is_some_and(|d| sign_in_file(Path::new(d)).is_file());
        if !signed {
            return Some(Err(anyhow::anyhow!(tr!(
                "Le compte Gemini « {name} » n’est pas connecté à Google : connecte-le dans Réglages › Comptes, « Se connecter… ».",
                "The Gemini account “{name}” isn’t signed in to Google: sign it in in Settings › Accounts, “Sign in…”.",
                name = account.name
            ))));
        }
        Some(Ok(()))
    }

    fn key_missing(&self, name: &str) -> String {
        tr!(
            "Le compte Gemini « {name} » n’a pas de clé d’API : donne-la dans Réglages › Comptes, « Changer la clé… ».",
            "The Gemini account “{name}” has no API key: give it in Settings › Accounts, “Change the key…”."
        )
    }

    fn key_malformed(&self) -> String {
        key_malformed()
    }

    /// Its key, shown by its end; or the Google sign-in `gemini` wrote in its folder, looked for and
    /// never read (its address beside it); the machine's account by the key or Vertex AI of the
    /// app's.
    fn account_status<'a>(
        &'a self,
        account: &'a crate::model::Account,
        settings: &'a crate::model::Settings,
    ) -> BoxFuture<'a, crate::accounts::AccountStatus> {
        use crate::accounts::{account_key, by_key, config_dir, is_principal, key_hint};
        let dir = config_dir(account, settings);
        let key = account_key(account).filter(|_| by_key(account));
        let (connected, stamp, email) = if is_principal(account) {
            // As far as can be told (its key may come from the user's terminal alone).
            (true, None, None)
        } else if by_key(account) {
            (key.is_some(), None, key.as_deref().map(key_hint))
        } else {
            let file = sign_in_file(&dir);
            let stamp = std::fs::metadata(&file)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| format!("{t:?}"));
            (stamp.is_some(), stamp, signed_in_email(&dir))
        };
        Box::pin(async move {
            crate::accounts::AccountStatus {
                connected,
                stamp,
                email,
                dir: dir.to_string_lossy().into_owned(),
            }
        })
    }

    /// Its machine's account is a key or Vertex AI of the app's alone: its Google sign-in is in
    /// `~/.gemini`, never given (ESC-275).
    fn machine_key(&self) -> Option<bool> {
        Some(machine_has_key())
    }

    fn login_shell(&self) -> String {
        "gemini".into()
    }

    /// Its interface straight to the Google sign-in (its settings in the account's folder), the
    /// home folder trusted.
    fn before_login(&self, account: &crate::model::Account, env: &mut Vec<(String, String)>) {
        if let Some(dir) = crate::accounts::own_dir(account) {
            if let Err(e) = write_settings(Path::new(dir), "oauth-personal") {
                log::warn!(
                    "sign-in of account {}: its settings not written: {e:#}",
                    account.id
                );
            }
        }
        env.push((TRUST.0.into(), TRUST.1.into()));
    }

    fn id(&self) -> EngineId {
        EngineId::GEMINI
    }

    fn list_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a Path,
    ) -> BoxFuture<'a, Result<Option<Vec<ToldModel>>>> {
        Box::pin(async move {
            let told = read_models(program, env, dir).await?;
            Ok(Some(catalog(&told)))
        })
    }

    fn name(&self) -> &'static str {
        "Gemini CLI"
    }

    fn speaker(&self) -> &'static str {
        "Gemini"
    }

    /// Before the first message: no command line to fit in.
    fn instructions_room(&self) -> Option<Room> {
        None
    }

    /// Its subagents (`invoke_agent`) take no message once they run.
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
            "son ACP n’a été étudié qu’à partir de celle-ci (docs/TODO-1.8.md, « Gemini CLI »)",
            "its ACP was only studied from that one on (docs/TODO-1.8.md, “Gemini CLI”)"
        )
    }

    fn update_command(&self) -> &'static str {
        "npm install -g @google/gemini-cli"
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

    fn terminal(&self) -> Option<&'static dyn super::TerminalView> {
        Some(&terminal::GEMINI_TERMINAL)
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
        &[]
    }

    fn context_window(&self, _model: &str) -> u64 {
        1_048_576
    }

    fn supports_effort(&self, _model: &str) -> bool {
        false
    }

    /// No classifier mode: a new agent starts in « Demander ».
    fn supports_auto(&self, _model: &str) -> bool {
        false
    }

    fn spawn(
        &self,
        spawn: Spawn,
        on_frame: super::OnFrame,
        on_exit: super::OnExit,
    ) -> Result<Process> {
        // Its conversation goes through its plugin (INT-10.7): the launch it describes, checked and started by the core
        // after `prepare`, what Gemini writes translated there.
        crate::plugins::harness::spawn_plugged(
            crate::plugins::builtin::GEMINI,
            self.name(),
            spawn,
            on_frame,
            on_exit,
        )
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        message(text, attachments).map(Message)
    }

    /// The items its plugin made of what Gemini wrote (`agent/items`).
    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        if let Some(items) = frame["items"].as_array() {
            rt.apply_items(items, fx);
        }
    }

    /// A session that cannot be loaded gives a new one (its plugin's client), said by the conversation.
    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(one_shot(ask))
    }

    fn writer(&self, _weight: Weight) -> &'static str {
        "Gemini CLI"
    }

    /// Its session is given the server (`mcpServers`, its header in the message): no file.
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

    /// Its `/compress` is a command of its interface (« Vue Gemini »).
    fn compaction(&self) -> super::Compaction {
        super::Compaction::InItsInterface("/compress")
    }
}
