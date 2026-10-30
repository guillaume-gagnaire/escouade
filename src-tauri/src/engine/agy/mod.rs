//! Antigravity CLI, the seventh harness: Google's agent in a terminal (`agy`), driven by its mode
//! without a screen (`-p`) in stream-json (`process`), its events translated by `AgentRt::agy_frame`
//! (`agent/agy.rs`), which hands most of them to Cursor's translation in ACP's form.
//!
//! What it does as Escouade drives it comes from the real runs of `agy` 1.4.9 (ESC-269,
//! docs/TODO-1.8.md, point 0, « Antigravity CLI »), whose streams are in `tests/fixtures/agy/`.
//!
//! Each agent has a home of its own (`ESCOUADE_AGY_HOME`, given to `agy` as its `USERPROFILE` and
//! `HOME`: `agy` keeps all it writes in `<home>/.gemini`, which it shares with Gemini CLI on the
//! machine). There Escouade writes its `antigravity-cli/settings.json` (the permission rules of its
//! mode, of its fence and of the user's « Toujours ») and `config/mcp_config.json` (Escouade's
//! server, its token in the file, never on a command line: `agy` expands no variable there). The
//! user's `~/.gemini` is never read nor written. Its account stays the machine's (the system's
//! keychain, which a home of its own does not move).
//!
//! `-p` asks nothing: what a permission would let through is refused, and its questions skipped;
//! the process asks the user at the end of the turn, writes the rules allowed and has the turn go
//! on (`process`). Its mode `plan` writes its plan in a file of its conversation: shown on the card
//! « Plan à approuver ».

#[cfg(test)]
pub(crate) mod bench;
mod locate;
mod process;
pub(crate) mod terminal;

use process::{AgyProcess, Denied};

use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
    ToldModel,
};
use super::{
    mcp, BoxFuture, Engine, Escouade, Message, OneShot, Process, Program, Room, Spawn, StartLost,
    Weight,
};
use crate::agent::{AgentRt, Effects};
use crate::core::Attachment;
use crate::i18n::{self, Lang};
use crate::model::EngineId;
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Antigravity CLI.
pub struct Agy;

pub static AGY: Agy = Agy;

/// The versions of Antigravity CLI Escouade was tested with (ESC-269, ESC-270).
pub const TESTED: Tested = Tested {
    oldest: "1.4.9",
    newest: "1.4.9",
};

/// Its installers (antigravity.google/docs/getting-started).
const INSTALL: Install = Install {
    windows: "irm https://antigravity.google/cli/install.ps1 | iex",
    unix: "curl -fsSL https://antigravity.google/cli/install.sh | bash",
};

/// The variable the core gives each agent's own home in (`Engine::agent_home_var`): its process
/// gives it to `agy` as `USERPROFILE` and `HOME`.
pub(crate) const HOME_VAR: &str = "ESCOUADE_AGY_HOME";

/// The keys of the machine that would take the place of its Google account: the first, an
/// account added by its key goes by (`modelProvider: gemini`, ESC-271).
pub(crate) const KEY_VARS: [&str; 2] = ["GEMINI_API_KEY", "GOOGLE_API_KEY"];

/// Its default model: Gemini Flash, its effort apart.
pub(crate) const DEFAULT: &str = "gemini-3.8-flash";

/// The models of its menu until `agy models` has listed them.
const MODELS: [(&str, &str); 1] = [(DEFAULT, "Gemini 3.8 Flash")];

/// The efforts of a Gemini model: part of its id in `agy models` (`gemini-3.8-flash-low`), its
/// `--effort` beside its id without them (`xhigh` and `max` refused by Flash, tried).
const EFFORTS: [&str; 3] = ["low", "medium", "high"];

/// Where its plans and their quotas are told: its usage itself is in its interface alone (`/usage`).
pub(crate) const USAGE_PAGE: &str = "https://antigravity.google/docs/plans";

/// How long after a limit of the model's capacity (not the account's) its agent tries again.
pub(crate) const CAPACITY_RESUME_MS: i64 = 60_000;

/// How long after its quota was met its agent tries again when `agy` tells no reset: its plans'
/// quotas are weekly and refreshed by parts (`/usage`: « Refreshes in 167h 49m »), a turn tried
/// for nothing costs none.
pub(crate) const QUOTA_RESUME_MS: i64 = 3_600_000;

/// The usage limit an error of `agy` tells (its words read in its program, ESC-269: « You have
/// exhausted your quota on this model. », « individual quota reached », `RESOURCE_EXHAUSTED`,
/// `MODEL_CAPACITY_EXHAUSTED`, an HTTP 429): in how many ms to try again, the reset it tells (« in
/// 2h 5m », « after 30s ») else a capacity's minute or a quota's hour. None for another error.
pub(crate) fn limit_of(error: &str) -> Option<i64> {
    let lower = error.to_lowercase();
    let capacity = lower.contains("model_capacity_exhausted") || lower.contains("capacity");
    let quota = [
        "exhausted your quota",
        "quota reached",
        "quota exceeded",
        "resource_exhausted",
        "rate limit",
        "too many requests",
        " 429",
    ]
    .iter()
    .any(|w| lower.contains(w))
        || lower.starts_with("429");
    if !capacity && !quota {
        return None;
    }
    Some(
        told_wait(&lower).unwrap_or(if quota && !lower.contains("capacity") {
            QUOTA_RESUME_MS
        } else {
            CAPACITY_RESUME_MS
        }),
    )
}

/// The wait a text tells after « in », « after » or « retry »: `2h 5m`, `167h 49m`, `30s`, `1d`.
fn told_wait(lower: &str) -> Option<i64> {
    [
        "refreshes in",
        "resets in",
        "reset in",
        "retry in",
        "retry after",
        "try again in",
        "after",
    ]
    .iter()
    .flat_map(|w| lower.match_indices(w).map(move |(i, _)| i + w.len()))
    .find_map(|at| wait_at(&lower[at..]))
}

/// The wait `text` starts with (` 2h 5m…`), none when it starts with none.
fn wait_at(text: &str) -> Option<i64> {
    let mut total = 0i64;
    let mut num = String::new();
    for c in text.chars() {
        match c {
            '0'..='9' => num.push(c),
            ' ' | ',' if num.is_empty() => {}
            _ if num.is_empty() => break,
            _ => {
                let unit = match c {
                    'd' => 86_400_000,
                    'h' => 3_600_000,
                    'm' => 60_000,
                    's' => 1_000,
                    _ => break,
                };
                total += num.parse::<i64>().ok()? * unit;
                num.clear();
            }
        }
    }
    (total > 0).then_some(total)
}

/// What the card of a turn stopped by its limit says (`said`: `agy`'s own words).
pub(crate) fn limited_error(lang: Lang, said: &str) -> String {
    tr_in!(
        lang,
        "Limite d’usage d’Antigravity atteinte : {said}",
        "Antigravity’s usage limit reached: {said}"
    )
}

/// The request of its process's own that asks the plan of a turn in the mode Plan approved.
pub(crate) const PLAN_REQUEST: &str = "escouade/plan";

/// What an approved plan is followed by.
pub(crate) const PLAN_GO: &str = "The plan is approved: carry it out now.";

/// What the first message tells the model of its questions: `-p` skips `ask_question`, Escouade
/// asks the user once the turn is over.
pub(crate) const QUESTIONS: &str = "<escouade>\nWhen you ask the user something with ask_question, end your turn right after it, without commenting on the answer you get: Escouade shows your questions to the user and sends the answers as the next message.\n</escouade>";

/// The model, effort and mode in force.
#[derive(Debug, Clone, Default)]
pub(crate) struct Settings {
    pub model: String,
    pub effort: String,
    pub mode: String,
}

/// What the rules of an agent hold beside its mode: its folder, the folders it reads, its fence, a
/// discussion that changes nothing.
#[derive(Debug, Clone, Default)]
pub(crate) struct AgyRules {
    pub cwd: String,
    pub read_dirs: Vec<String>,
    pub fence: Vec<String>,
    pub read_only: bool,
}

/// The `--mode` of `agy` for Escouade's `mode`: « Tout accepter » is its rules', « Demander » its
/// default.
pub(crate) fn agy_mode(mode: &str) -> Option<&'static str> {
    match mode {
        "plan" => Some("plan"),
        "acceptEdits" => Some("accept-edits"),
        _ => None,
    }
}

/// The modes Escouade gives Antigravity, in the menu's order (its `auto` is not offered: « not
/// available for this account » in the real run).
const MODES: [&str; 4] = ["default", "plan", "acceptEdits", "bypassPermissions"];

fn meaning(lang: Lang, mode: &str) -> String {
    match mode {
        "plan" => tr_in!(
            lang,
            "Son mode plan : Antigravity étudie et écrit un plan sans rien changer au projet. Le plan s’approuve sur sa carte, puis Antigravity le met en œuvre. Pas de bac à sable.",
            "Its plan mode: Antigravity studies and writes a plan without changing the project. The plan is approved on its card, then Antigravity carries it out. No sandbox."
        ),
        "acceptEdits" => tr_in!(
            lang,
            "Son mode accept-edits : ses écritures dans le dossier passent sans demander ; ses commandes (sauf git add et git commit), ses écritures ailleurs et les outils MCP demandent. Pas de bac à sable.",
            "Its accept-edits mode: its writes in the folder go through without asking; its commands (but git add and git commit), its writes elsewhere and MCP tools ask. No sandbox."
        ),
        "bypassPermissions" => tr_in!(
            lang,
            "Escouade lui permet tout par ses règles : commandes, fichiers, outils MCP et adresses, sans rien demander. Les chemins d’un projet privé restent refusés. Pas de bac à sable.",
            "Escouade allows it everything by its rules: commands, files, MCP tools and addresses, without asking. A private project’s paths stay refused. No sandbox."
        ),
        _ => tr_in!(
            lang,
            "Ses écritures dans le dossier de l’agent passent ; ses commandes, ses écritures ailleurs et les outils MCP demandent, une fois son tour fini (Antigravity ne demande rien en cours de tour), puis il les refait. Il lit sans demander. Pas de bac à sable.",
            "Its writes in the agent’s folder go through; its commands, its writes elsewhere and MCP tools ask, once its turn is over (Antigravity asks nothing during a turn), then it does them again. It reads without asking. No sandbox."
        ),
    }
}

/// What Antigravity does, as Escouade drives it.
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
        // Its stream-json takes text alone.
        images: false,
        pdf: false,
        // No copy of a conversation.
        fork: ForkPoint::None,
        // Its conversations are in the agent's own home, whatever the account.
        resume_other_account: true,
        mid_turn: MidTurn::Queued,
        slash_commands: false,
        // Its quota is in its interface's `/usage` alone, its cost in no stream.
        quota: QuotaKind::None,
        quota_windows: Vec::new(),
        cost: CostKind::None,
        terminal_view: true,
        remote_control: false,
        background_tasks: false,
        todo_tools: false,
        default_refusal: false,
        sandbox: false,
        usage_page: Some(USAGE_PAGE),
        // Another account than the machine's Google sign-in goes by its Gemini API key (ESC-271).
        accounts_by_key: true,
        read_only: true,
    }
}

/// The efforts in which `id` (`gemini-3.8-flash-low`) ends, and its id without it.
fn effort_of(id: &str) -> Option<(&str, &'static str)> {
    EFFORTS
        .iter()
        .find_map(|e| id.strip_suffix(&format!("-{e}")).map(|base| (base, *e)))
}

/// The models `agy models` lists (`<id>\t<name>` a line): a Gemini model whose efforts are part of
/// its ids once, with its efforts; any other as it is.
pub(crate) fn models_of(listed: &str) -> Vec<ToldModel> {
    let mut out: Vec<ToldModel> = Vec::new();
    for line in listed.lines() {
        let Some((id, label)) = line.split_once('\t') else {
            continue;
        };
        let (id, label) = (id.trim(), label.trim());
        if id.is_empty() {
            continue;
        }
        let model = |id: &str, label: &str| ToldModel {
            id: id.to_string(),
            label: label.to_string(),
            efforts: Vec::new(),
            default_effort: None,
            images: false,
            default: false,
            provider: None,
            multiplier: None,
            price: None,
            from_config: false,
        };
        match effort_of(id).filter(|(base, _)| base.starts_with("gemini-")) {
            Some((base, effort)) => {
                let name = label
                    .rsplit_once(" (")
                    .map(|(n, _)| n)
                    .unwrap_or(label)
                    .to_string();
                let at = match out.iter().position(|m| m.id == base) {
                    Some(at) => at,
                    None => {
                        out.push(model(base, &name));
                        out.len() - 1
                    }
                };
                out[at].efforts.push(effort.to_string());
            }
            None => out.push(model(id, label)),
        }
    }
    for m in out.iter_mut() {
        m.efforts
            .sort_by_key(|e| EFFORTS.iter().position(|x| x == e).unwrap_or(9));
        if !m.efforts.is_empty() {
            m.default_effort = Some(if m.efforts.iter().any(|e| e == "medium") {
                "medium".into()
            } else {
                m.efforts[0].clone()
            });
        }
    }
    let first = out
        .iter()
        .position(|m| m.id == DEFAULT)
        .or((!out.is_empty()).then_some(0));
    if let Some(i) = first {
        out[i].default = true;
    }
    out
}

/// The composer's menu of the models `told`, else Antigravity's default one.
pub(crate) fn catalog(told: &[ToldModel]) -> Vec<ToldModel> {
    if !told.is_empty() {
        return told.to_vec();
    }
    models_of(&format!(
        "{DEFAULT}-high\tGemini 3.8 Flash (High)\n{DEFAULT}-medium\tGemini 3.8 Flash (Medium)\n{DEFAULT}-low\tGemini 3.8 Flash (Low)"
    ))
}

/// The arguments of a program of the agent: stream-json both ways, its model (and effort), its
/// mode, its conversation.
pub(crate) fn launch_args(settings: &Settings, conversation: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = [
        "-p=",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
    ]
    .map(String::from)
    .to_vec();
    let model = settings.model.trim();
    if !model.is_empty() && !model.contains(char::is_whitespace) {
        args.push("--model".into());
        args.push(model.into());
        if AGY.supports_effort(model) && EFFORTS.contains(&settings.effort.as_str()) {
            args.push("--effort".into());
            args.push(settings.effort.clone());
        }
    }
    if let Some(mode) = agy_mode(&settings.mode) {
        args.push("--mode".into());
        args.push(mode.into());
    }
    if let Some(id) = conversation.filter(|c| !c.is_empty()) {
        args.push("--conversation".into());
        args.push(id.into());
    }
    args
}

/// Its folder of config in the agent's home.
pub(crate) fn gemini_dir(home: &Path) -> PathBuf {
    home.join(".gemini")
}

/// The settings of `agy` in the agent's home.
pub(crate) fn settings_file(home: &Path) -> PathBuf {
    gemini_dir(home)
        .join("antigravity-cli")
        .join("settings.json")
}

/// Its MCP config in the agent's home (`agy mcp add` writes the same file).
pub(crate) fn mcp_file(home: &Path) -> PathBuf {
    gemini_dir(home).join("config").join("mcp_config.json")
}

/// What the user allowed this agent for good (« Toujours »), kept beside its home's `.gemini`.
fn always_file(home: &Path) -> PathBuf {
    home.join("escouade-always.json")
}

/// The transcript of the conversation `id`, step by step, as `agy` writes it.
pub(crate) fn transcript(home: &Path, id: &str) -> PathBuf {
    gemini_dir(home)
        .join("antigravity-cli")
        .join("brain")
        .join(id)
        .join(".system_generated")
        .join("logs")
        .join("transcript_full.jsonl")
}

pub(crate) fn read_always(home: &Path) -> Vec<String> {
    std::fs::read_to_string(always_file(home))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub(crate) fn write_always(home: &Path, rules: &[String]) {
    let _ = std::fs::create_dir_all(home);
    if let Err(e) = std::fs::write(always_file(home), json!(rules).to_string()) {
        log::warn!("agy: « Toujours » not kept: {e}");
    }
}

/// The rules a mode allows without asking: in « Demander », the writes in the agent's folder (as
/// Cursor's are accepted by Escouade's rules); in « Accepter les modifications », `git add` and `git
/// commit` beside its own; in « Tout accepter », all. The folders of the files joined to its
/// messages are read in all.
pub(crate) fn mode_rules(rules: &AgyRules, mode: &str) -> Vec<String> {
    let mut allow: Vec<String> = rules
        .read_dirs
        .iter()
        .map(|d| format!("read_file({d})"))
        .collect();
    match mode {
        "bypassPermissions" => allow.extend(
            [
                "command(*)",
                "write_file(*)",
                "read_file(*)",
                "mcp(*)",
                "read_url(*)",
                "execute_url(*)",
            ]
            .map(String::from),
        ),
        "acceptEdits" => {
            allow.push("command(git add)".into());
            allow.push("command(git commit)".into());
        }
        "plan" => {}
        _ => {
            if !rules.cwd.is_empty() {
                allow.push(format!("write_file({})", rules.cwd));
            }
        }
    }
    allow
}

/// The rules refused whatever the mode: the paths of a private project (refused to read refuses to
/// write too, `agy`'s rule), and for a discussion every write and command.
pub(crate) fn deny_rules(rules: &AgyRules) -> Vec<String> {
    let mut deny: Vec<String> = rules
        .fence
        .iter()
        .map(|p| p.trim_end_matches(['/', '\\']))
        .filter(|p| !p.is_empty())
        .map(|p| format!("read_file({p})"))
        .collect();
    if rules.read_only {
        deny.push("write_file(*)".into());
        deny.push("command(*)".into());
    }
    deny
}

/// The agent's `settings.json`: its `permissions` as Escouade sets them (its mode's rules, what the
/// user allowed, its refusals: `deny` before all, `agy`'s rule); the rest of the file kept.
pub(crate) fn write_settings(
    home: &Path,
    rules: &AgyRules,
    mode: &str,
    allowed: &[String],
) -> Result<()> {
    let file = settings_file(home);
    let before = std::fs::read_to_string(&file).unwrap_or_default();
    let doc = settings_doc(&before, rules, mode, allowed);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&file, serde_json::to_string_pretty(&doc)?)?;
    Ok(())
}

/// The settings `before` (a JSON object, else none) with their `permissions` as Escouade sets them.
pub(crate) fn settings_doc(
    before: &str,
    rules: &AgyRules,
    mode: &str,
    allowed: &[String],
) -> Value {
    let mut doc: Value = serde_json::from_str(before)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let mut allow = mode_rules(rules, mode);
    for r in allowed {
        if !allow.contains(r) {
            allow.push(r.clone());
        }
    }
    doc["permissions"] = json!({ "allow": allow, "deny": deny_rules(rules), "ask": [] });
    doc
}

/// Escouade's server in the agent's MCP config, its token in the file (`agy` expands no variable
/// there, ESC-269), never on a command line; none when it is refused.
pub(crate) fn write_grant(home: &Path, escouade: Option<(&str, &str)>) -> Result<()> {
    let file = mcp_file(home);
    let before = std::fs::read_to_string(&file).unwrap_or_default();
    let doc = grant_doc(&before, escouade);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&file, serde_json::to_string_pretty(&doc)?)?;
    Ok(())
}

/// The provider of its models in the settings of `home`: the Gemini API by `GEMINI_API_KEY` for an
/// account added by its key (its reference: `modelProvider: "gemini"`), else its Google sign-in (the
/// setting taken out). The rest of the file kept.
pub(crate) fn write_provider(home: &Path, by_key: bool) -> Result<()> {
    let file = settings_file(home);
    let mut doc: Value = std::fs::read_to_string(&file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    match doc.as_object_mut() {
        Some(o) if by_key => {
            o.insert("modelProvider".into(), json!("gemini"));
        }
        Some(o) => {
            o.remove("modelProvider");
        }
        None => {}
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&file, serde_json::to_string_pretty(&doc)?)?;
    Ok(())
}

/// Whether `env` gives `agy` a Gemini API key of its own (an account added by its key): its
/// Principal's is given empty.
pub(crate) fn by_key(env: &[(String, String)]) -> bool {
    env.iter()
        .any(|(k, v)| k == KEY_VARS[0] && !v.trim().is_empty())
}

/// The settings of a process that writes a text: every tool refused (`deny` before all), its
/// provider as its account's.
pub(crate) fn write_no_tools(home: &Path, by_key: bool) -> Result<()> {
    write_provider(home, by_key)?;
    let file = settings_file(home);
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&file)?)?;
    doc["permissions"] = json!({
        "allow": [],
        "deny": ["command(*)", "write_file(*)", "read_file(*)", "mcp(*)", "read_url(*)", "execute_url(*)"],
        "ask": [],
    });
    std::fs::write(&file, serde_json::to_string_pretty(&doc)?)?;
    Ok(())
}

/// The MCP config `before` (a JSON object, else none) with Escouade's server in it, or out of it.
pub(crate) fn grant_doc(before: &str, escouade: Option<(&str, &str)>) -> Value {
    let mut doc: Value = serde_json::from_str(before)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    if !doc["mcpServers"].is_object() {
        doc["mcpServers"] = json!({});
    }
    match escouade {
        Some((url, token)) => {
            doc["mcpServers"][mcp::SERVER] = json!({
                "serverUrl": url, "headers": { "Authorization": format!("Bearer {token}") },
                "disabled": false,
            });
        }
        None => {
            if let Some(o) = doc["mcpServers"].as_object_mut() {
                o.remove(mcp::SERVER);
            }
        }
    }
    doc
}

/// Escouade's server taken out of the agent's MCP config once its process is over: its token goes
/// with it.
pub(crate) fn remove_grant(home: &Path) {
    if mcp_file(home).is_file() {
        if let Err(e) = write_grant(home, None) {
            log::warn!("agy: Escouade's server not taken out of its MCP config: {e:#}");
        }
    }
}

/// Whether the call `name` with `args` writes the plan of the mode `plan` (`brain/<id>/plan_*.md`).
pub(crate) fn is_plan_file(name: &str, args: &Value) -> bool {
    if !matches!(name, "write_file" | "write_to_file") {
        return false;
    }
    let path = args["Path"].as_str().unwrap_or_default().replace('\\', "/");
    let file = path.rsplit('/').next().unwrap_or_default();
    path.contains("/brain/") && file.starts_with("plan") && file.ends_with(".md")
}

/// The path a call names (`Path`, `AbsolutePath`, `TargetFile`, `DirectoryPath`).
pub(crate) fn path_of(args: &Value) -> Option<&str> {
    [
        "Path",
        "AbsolutePath",
        "TargetFile",
        "DirectoryPath",
        "SearchPath",
    ]
    .iter()
    .find_map(|k| args[*k].as_str())
}

/// Whether a call of `agy` names a path of `fence`.
pub(crate) fn fenced(args: &Value, fence: &[String]) -> bool {
    !fence.is_empty()
        && args.as_object().into_iter().flatten().any(|(_, v)| {
            v.as_str()
                .is_some_and(|t| fence.iter().any(|p| crate::fence::names(t, p)))
        })
}

/// The rule of `agy` that allows the call `tool` with `args` (`command(echo hi)`,
/// `write_file(C:\x\b.txt)`, `mcp(escouade/get_board)`), and the call as the model is told it was
/// allowed or refused. None for a call no rule names.
pub(crate) fn rule_of(tool: &str, args: &Value) -> Option<(String, String)> {
    let path = path_of(args);
    match tool {
        "run_command" => {
            let c = args["CommandLine"].as_str()?.trim();
            Some((format!("command({c})"), format!("the command `{c}`")))
        }
        "write_file"
        | "write_to_file"
        | "edit_file"
        | "replace_file_content"
        | "multi_replace_file_content"
        | "sed_file"
        | "notebook_edit" => {
            let p = path?;
            Some((format!("write_file({p})"), format!("writing {p}")))
        }
        "read_file" | "view_file" | "list_dir" | "find_by_name" | "grep_search" => {
            let p = path?;
            Some((format!("read_file({p})"), format!("reading {p}")))
        }
        "call_mcp_tool" => {
            let (s, t) = (args["ServerName"].as_str()?, args["ToolName"].as_str()?);
            Some((
                format!("mcp({s}/{t})"),
                format!("the tool {t} of the MCP server {s}"),
            ))
        }
        "read_url_content" | "open_browser_url" => {
            let url = args["Url"].as_str().or(args["url"].as_str())?;
            let host = url
                .split("://")
                .nth(1)
                .unwrap_or(url)
                .split(['/', ':', '?'])
                .next()?;
            Some((format!("read_url({host})"), format!("reading {url}")))
        }
        _ => None,
    }
}

/// The id of the call of step `step` in the conversation `session`, as both the translation and
/// the permissions name it.
pub(crate) fn call_id(session: &str, step: u64) -> String {
    let short: String = session.chars().take(8).collect();
    format!("agy-{short}-{step}")
}

/// A call of `agy` (its tool `name`, its arguments) in the form Cursor's translation reads (ACP's
/// `tool_call`): its `kind`, `title`, `rawInput`, and the diff of a write.
pub(crate) fn acp_call(name: &str, args: &Value) -> Value {
    let path = path_of(args);
    let (kind, raw) = match name {
        "run_command" => ("execute", json!({ "command": args["CommandLine"] })),
        "write_file"
        | "write_to_file"
        | "edit_file"
        | "replace_file_content"
        | "multi_replace_file_content"
        | "sed_file" => ("edit", json!({ "path": path })),
        "read_file" | "view_file" => ("read", json!({ "path": path })),
        "grep_search" | "find_by_name" => (
            "search",
            json!({
                "pattern": args["Query"].as_str().or(args["Pattern"].as_str()).unwrap_or_default(),
                "path": path,
            }),
        ),
        "read_url_content" => ("fetch", json!({ "url": args["Url"] })),
        "call_mcp_tool" => (
            "other",
            json!({
                "providerIdentifier": args["ServerName"], "toolName": args["ToolName"],
                "args": args["Arguments"].clone(),
            }),
        ),
        "invoke_subagent" => (
            "other",
            json!({
                "_toolName": "task",
                "description": args["Role"].as_str().unwrap_or("subagent"),
                "prompt": args["Prompt"].as_str().or(args["InitialPrompt"].as_str()),
                "subagentType": args["TypeName"],
            }),
        ),
        _ => {
            let mut raw = if args.is_object() {
                args.clone()
            } else {
                json!({})
            };
            raw["_toolName"] = json!(name);
            ("other", raw)
        }
    };
    let mut call = json!({ "kind": kind, "title": name, "rawInput": raw });
    if let Some(d) = diff_of(name, args) {
        call["content"] = json!([d]);
    }
    call
}

/// The diff of a write, from its arguments: a file written whole (`Content`, `CodeContent`), or
/// a text replaced (`Target` → `Replacement`).
pub(crate) fn diff_of(name: &str, args: &Value) -> Option<Value> {
    let path = path_of(args)?;
    let pick = |keys: &[&str]| keys.iter().find_map(|k| args[*k].as_str());
    match name {
        "write_file" | "write_to_file" => {
            let new = pick(&["Content", "CodeContent"])?;
            Some(json!({ "type": "diff", "path": path, "oldText": null, "newText": new }))
        }
        "edit_file" | "replace_file_content" => {
            let old = pick(&["Target", "TargetContent"])?;
            let new = pick(&["Replacement", "ReplacementContent"])?;
            Some(json!({ "type": "diff", "path": path, "oldText": old, "newText": new }))
        }
        _ => None,
    }
}

/// The permission of a call `agy` refused, as Cursor's translation reads it
/// (`session/request_permission`).
fn permission_params(session: &str, d: &Denied) -> Value {
    let mut tc = acp_call(&d.tool, &d.args);
    tc["toolCallId"] = json!(call_id(session, d.step));
    json!({
        "sessionId": session, "toolCall": tc,
        "options": [
            { "optionId": "allow-once", "kind": "allow_once", "name": "Allow once" },
            { "optionId": "allow-always", "kind": "allow_always", "name": "Always allow" },
            { "optionId": "reject-once", "kind": "reject_once", "name": "Reject" },
        ],
    })
}

/// The label of a choice of `ask_question`, its mark of the one it recommends left out.
fn choice_label(o: &Value) -> String {
    let text = o
        .as_str()
        .or(o["label"].as_str())
        .unwrap_or_default()
        .trim();
    text.strip_prefix("(Recommended)")
        .unwrap_or(text)
        .trim()
        .to_string()
}

/// The questions of `ask_question` (`questions: [{question, options, is_multi_select}]`) as
/// Cursor's `cursor/ask_question` gives them.
fn question_params(call: &str, args: &Value) -> Value {
    let questions: Vec<Value> = args["questions"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, q)| {
            let options: Vec<Value> = q["options"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|o| {
                    let l = choice_label(o);
                    json!({ "id": l, "label": l })
                })
                .collect();
            json!({
                "id": format!("q{i}"), "prompt": q["question"], "options": options,
                "allowMultiple": q["is_multi_select"] == true,
            })
        })
        .collect();
    json!({ "toolCallId": call, "questions": questions })
}

/// What the user answered to the questions `params`, a line each.
fn replies(params: &Value, answers: &Value) -> Vec<String> {
    params["questions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|q| {
            let text = q["prompt"].as_str().unwrap_or_default();
            let a = answers[text].as_str().unwrap_or_default().trim();
            format!("- {text} → {a}")
        })
        .collect()
}

/// The message that has the turn go on once the user answered (in the model's words): what was
/// allowed, to do again, what was refused, the answers; None when nothing was allowed nor answered.
pub(crate) fn follow_up(
    allowed: &[String],
    refused: &[String],
    replies: &[String],
) -> Option<String> {
    if allowed.is_empty() && replies.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if !allowed.is_empty() {
        parts.push(format!(
            "The user allowed {}: do it again now, then go on.",
            allowed.join(", ")
        ));
    }
    if !refused.is_empty() {
        parts.push(format!(
            "The user refused {}: don't do it.",
            refused.join(", ")
        ));
    }
    if !replies.is_empty() {
        parts.push(format!(
            "The user answered your questions:\n{}",
            replies.join("\n")
        ));
    }
    Some(parts.join("\n"))
}

/// No conversation opened for the agent: what `agy` said.
fn session_refused(lang: Lang, e: &anyhow::Error) -> String {
    tr_in!(
        lang,
        "Antigravity CLI n’a pas démarré la conversation de l’agent : {e:#}. S’il n’est pas connecté, lance agy dans un terminal pour te connecter à ton compte Google.",
        "Antigravity CLI didn’t start the agent’s conversation: {e:#}. If it isn’t signed in, run agy in a terminal to sign in to your Google account."
    )
}

/// A message to `agy`: its text, the text files joined to it as text. An image or a PDF is
/// refused (its stream-json takes text).
fn message(text: &str, attachments: &[Attachment]) -> Result<Value> {
    let mut parts = Vec::new();
    for a in attachments {
        match a.media_type.as_str() {
            "text/plain" => parts.push(format!("{}:\n\n{}", a.name, a.data)),
            t => bail!(tr!(
                "« {name} » ({t}) ne peut pas être joint : Antigravity CLI ne lit que les fichiers texte joints.",
                "“{name}” ({t}) can’t be attached: Antigravity CLI only reads attached text files.",
                name = a.name
            )),
        }
    }
    parts.push(text.to_string());
    Ok(json!({ "text": parts.join("\n\n") }))
}

/// A home of its own for a process that keeps nothing (a text written, the models read).
fn throwaway_home() -> Result<PathBuf> {
    let home = std::env::temp_dir().join(format!("escouade-agy-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&home)?;
    Ok(home)
}

/// `env` with `home` as the home of `agy` (`USERPROFILE`, `HOME`), and the home of the user's tools
/// kept where they are (its git config, Rust's), which its commands would no longer find.
pub(crate) fn home_env(env: &[(String, String)], home: &Path) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = env
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), HOME_VAR | "USERPROFILE" | "HOME"))
        .cloned()
        .collect();
    let given = |env: &[(String, String)], k: &str| {
        env.iter().any(|(e, _)| e == k) || std::env::var_os(k).is_some()
    };
    if let Some(real) = dirs::home_dir() {
        for (var, name) in [
            ("GIT_CONFIG_GLOBAL", ".gitconfig"),
            ("CARGO_HOME", ".cargo"),
            ("RUSTUP_HOME", ".rustup"),
        ] {
            let path = real.join(name);
            if !given(&env, var) && path.exists() {
                env.push((var.into(), path.to_string_lossy().into_owned()));
            }
        }
    }
    let home = home.to_string_lossy().into_owned();
    env.push(("USERPROFILE".into(), home.clone()));
    env.push(("HOME".into(), home));
    env
}

/// What `agy` answered in stream-json (its `result`).
fn result_of(out: &[u8]) -> Result<String> {
    let text = String::from_utf8_lossy(out);
    let result = text
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["event"] == "result")
        .ok_or_else(|| {
            anyhow!(tr!(
                "Antigravity CLI n’a rien répondu",
                "Antigravity CLI gave no answer"
            ))
        })?;
    let r = &result["result"];
    if r["status"] == "ERROR" {
        bail!("{}", r["error"].as_str().unwrap_or("error"));
    }
    Ok(r["response"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_string())
}

/// One question to `agy -p`, in a home of its own removed once answered, on its fastest Gemini,
/// every tool refused (ESC-271); its time bounded by `oneshot::LIMIT`.
async fn one_shot(ask: OneShot<'_>) -> Result<String> {
    let home = throwaway_home()?;
    if let Err(e) = write_no_tools(&home, by_key(&ask.env)) {
        let _ = std::fs::remove_dir_all(&home);
        bail!("agy: its tools not refused for a text: {e:#}");
    }
    let env = home_env(&ask.env, &home);
    let model = match ask.weight {
        Weight::Light => format!("{DEFAULT}-low"),
        Weight::Reading => format!("{DEFAULT}-medium"),
    };
    let mut args = launch_args(&Settings::default(), None);
    args.extend(["--model".to_string(), model]);
    let question = if ask.system.trim().is_empty() {
        ask.prompt.to_string()
    } else {
        format!(
            "<instructions>\n{}\n</instructions>\n\n{}",
            ask.system, ask.prompt
        )
    };
    let input = format!(
        "{}\n",
        json!({ "event": "user", "message": { "content": question } })
    );
    let out = super::oneshot::run(
        &ask.program,
        ask.cwd,
        &args,
        &env,
        &input,
        "Antigravity CLI",
    )
    .await;
    let _ = std::fs::remove_dir_all(&home);
    result_of(&out?)
}

/// The models of the account (`agy models`), in a home of its own removed after.
async fn read_models(
    program: Program,
    env: Vec<(String, String)>,
    cwd: &Path,
) -> Result<Vec<ToldModel>> {
    let home = throwaway_home()?;
    // An account by its key lists the Gemini API's models.
    write_provider(&home, by_key(&env))?;
    let env = home_env(&env, &home);
    let out = super::oneshot::run(
        &program,
        cwd,
        &["models".to_string()],
        &env,
        "",
        "Antigravity CLI",
    )
    .await;
    let _ = std::fs::remove_dir_all(&home);
    let told = models_of(&String::from_utf8_lossy(&out?));
    if told.is_empty() {
        bail!("agy models: no model listed");
    }
    Ok(told)
}

/// Keeps `agy` from updating itself (read in its program, ESC-269).
const NO_SELF_UPDATE: [(&str, &str); 1] = [("AGY_CLI_DISABLE_AUTO_UPDATE", "1")];

impl Engine for Agy {
    /// Its sign-in is the machine's (the system's keychain, not moved by its home), made by its
    /// interface itself (`agy`, no argument) in a terminal; each agent has a home of its own
    /// (`agent_home_var`), another account goes by its Gemini API key (ESC-271).
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

    fn keys(&self) -> crate::accounts::KeyUse {
        crate::accounts::KeyUse::Added
    }

    /// An account added by its key goes by `GEMINI_API_KEY` (its process then writes
    /// `modelProvider: gemini` in the agent's settings); its Principal by the machine's Google
    /// sign-in, never by a key of the app's (this machine has one, ESC-273).
    fn account_env(
        &self,
        account: &crate::model::Account,
        inherited: &dyn Fn(&str) -> bool,
    ) -> Vec<(String, String)> {
        use crate::accounts::{account_key, by_key, AGY_KEY_VAR};
        let key = account_key(account).filter(|_| by_key(account));
        let mut env: Vec<(String, String)> = KEY_VARS
            .into_iter()
            .filter(|v| inherited(v) && (*v != AGY_KEY_VAR || key.is_none()))
            .map(|v| (v.to_string(), String::new()))
            .collect();
        if let Some(key) = key {
            env.push((AGY_KEY_VAR.to_string(), key));
        }
        env
    }

    fn key_missing(&self, name: &str) -> String {
        tr!(
            "Le compte Antigravity « {name} » n’a pas de clé d’API Gemini : donne-la dans Réglages › Comptes, « Changer la clé… ».",
            "The Antigravity account “{name}” has no Gemini API key: give it in Settings › Accounts, “Change the key…”."
        )
    }

    fn key_malformed(&self) -> String {
        crate::engine::gemini::key_malformed()
    }

    /// Its Principal signed in by its Google account, in the keychain: its entry is looked for,
    /// never read (`agy_signed_in`); another account by its key, shown by its end.
    fn account_status<'a>(
        &'a self,
        account: &'a crate::model::Account,
        settings: &'a crate::model::Settings,
    ) -> BoxFuture<'a, crate::accounts::AccountStatus> {
        Box::pin(async move {
            let machine = if crate::accounts::by_key(account) {
                (false, None)
            } else {
                let signed = tokio::task::spawn_blocking(crate::accounts::agy_signed_in)
                    .await
                    .ok()
                    .flatten();
                // Signed in by the terminal of « Se connecter… » once its entry is there.
                (
                    signed.unwrap_or(true),
                    signed.filter(|s| *s).map(|_| "keychain".to_string()),
                )
            };
            crate::accounts::key_status(account, settings, machine)
        })
    }

    fn signs_in_later(&self) -> bool {
        true
    }

    /// In the agent's own home, whatever its account (ESC-271).
    fn sessions_in_agent_home(&self) -> bool {
        true
    }

    fn login_shell(&self) -> String {
        "agy".into()
    }

    fn id(&self) -> EngineId {
        EngineId::AGY
    }

    fn name(&self) -> &'static str {
        "Antigravity CLI"
    }

    fn speaker(&self) -> &'static str {
        "Antigravity"
    }

    /// In its first message: no command line to fit in.
    fn instructions_room(&self) -> Option<Room> {
        None
    }

    /// Its subagents take no message from Escouade.
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
            "son stream-json n’a été vérifié qu’à partir de celle-ci (docs/TODO-1.8.md, « Antigravity CLI »)",
            "its stream-json was only checked from that one on (docs/TODO-1.8.md, “Antigravity CLI”)"
        )
    }

    fn update_command(&self) -> &'static str {
        "agy update"
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
        Some(&terminal::AGY_TERMINAL)
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

    fn list_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a Path,
    ) -> BoxFuture<'a, Result<Option<Vec<ToldModel>>>> {
        Box::pin(async move { Ok(Some(catalog(&read_models(program, env, dir).await?))) })
    }

    /// A Gemini model named without its effort (`gemini-3.8-flash`): `--effort` beside it.
    fn supports_effort(&self, model: &str) -> bool {
        model.starts_with("gemini-") && effort_of(model).is_none()
    }

    fn supports_auto(&self, _model: &str) -> bool {
        false
    }

    fn spawn(
        &self,
        spawn: Spawn,
        on_frame: super::OnFrame,
        on_exit: super::OnExit,
    ) -> Result<Process> {
        let home = spawn
            .env
            .iter()
            .find(|(k, _)| k == HOME_VAR)
            .map(|(_, v)| PathBuf::from(v))
            .ok_or_else(|| {
                anyhow!(tr!(
                    "Antigravity CLI n’a pas de dossier à lui pour cet agent : l’agent ne démarre pas.",
                    "Antigravity CLI has no folder of its own for this agent: the agent doesn’t start."
                ))
            })?;
        log::info!(
            "{}: starting {} in {} (account {})",
            spawn.label,
            spawn.program.describe(),
            spawn.cwd,
            spawn.account
        );
        let escouade = match &spawn.escouade {
            Escouade::Granted { url, token, .. } => Some((url.as_str(), token.as_str())),
            Escouade::Denied { .. } => None,
        };
        write_grant(&home, escouade)?;
        // Its account's key, or the machine's Google sign-in (ESC-271).
        write_provider(&home, by_key(&spawn.env))?;
        // The hooks of its interface (« Vue Antigravity ») would run here too, for nothing.
        terminal::forget_hooks(&home);
        let rules = AgyRules {
            cwd: spawn.cwd.clone(),
            read_dirs: spawn
                .read_dirs
                .iter()
                .map(|d| d.to_string_lossy().into_owned())
                .collect(),
            fence: crate::fence::spelled(&spawn.fence, &spawn.program),
            read_only: spawn.read_only,
        };
        // The refusals of its fence written before it starts: unwritten, it does not start.
        write_settings(&home, &rules, &spawn.mode, &read_always(&home)).map_err(|e| {
            anyhow!(tr!(
                "Les chemins des projets privés n’ont pas pu être refusés à Antigravity CLI ({e}) : l’agent ne démarre pas.",
                "The private projects’ paths couldn’t be refused to Antigravity CLI ({e}): the agent doesn’t start."
            ))
        })?;
        let env = home_env(&spawn.env, &home);
        let instructions = [spawn.append_prompt.clone(), Some(QUESTIONS.to_string())]
            .into_iter()
            .flatten()
            .filter(|t| !t.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let opts = process::Opts {
            program: spawn.program,
            cwd: spawn.cwd,
            env,
            home,
            start: spawn.start,
            settings: Settings {
                model: spawn.model,
                effort: spawn.effort,
                mode: spawn.mode,
            },
            instructions: Some(instructions),
            rules,
        };
        let p: Process = AgyProcess::new(opts, on_frame, on_exit);
        Ok(p)
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        message(text, attachments).map(Message)
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        rt.agy_frame(frame, fx);
    }

    /// A conversation that cannot be resumed gives a new one (`process`), said by the conversation.
    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(one_shot(ask))
    }

    fn writer(&self, _weight: Weight) -> &'static str {
        "Gemini 3.8 Flash"
    }

    /// Written in the agent's own MCP config by its spawn (`write_grant`).
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
}

#[cfg(test)]
mod tests;
