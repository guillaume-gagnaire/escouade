//! Codex, the second engine: a `codex app-server` process per agent, driven over its JSON-RPC
//! protocol (docs/PROTOCOL.md, « Codex app-server »), its messages translated by
//! `AgentRt::codex_frame` (`agent/codex.rs`).
//!
//! What it does as Escouade drives it comes from the real runs of Codex 0.163.0 (ESC-22,
//! docs/TODO-1.8.md, point 0, « Codex »), whose streams are in `tests/fixtures/codex/` and whose
//! schema is `tests/fixtures/engines/codex-0.163.0.schema.json`. Its texts written by a model go
//! through `codex exec` (`exec`).

#[cfg(test)]
mod bench;
mod exec;
pub(crate) mod git;
mod locate;
mod process;
#[cfg(test)]
mod schema_tests;
pub(crate) mod terminal;

#[cfg(test)]
pub(crate) use bench::fake_codex;
pub(crate) use process::id_key;
use process::CodexProcess;

use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
    ToldModel,
};
use super::userconfig::{Format, UserConfig};
use super::{
    mcp, BoxFuture, Compaction, Engine, Escouade, Message, OneShot, Process, Program, Room, Spawn,
    StartLost, Weight,
};
use crate::agent::{AgentRt, Effects};
use crate::core::{size_label, Attachment, MAX_IMAGE, MAX_TOTAL};
use crate::i18n::{self, Lang};
use crate::model::{EngineId, QuotaMeter, RateWindow};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Codex.
pub struct Codex;

pub static CODEX: Codex = Codex;

/// The versions of Codex Escouade was tested with: its app-server as the real run of 2026-10-16
/// found it (ESC-22).
pub const TESTED: Tested = Tested {
    oldest: "0.163.0",
    newest: "0.163.0",
};

/// Its installer, npm's package (the desktop app of Codex comes with an older one of its own).
const INSTALL: Install = Install {
    windows: "npm install -g @openai/codex",
    unix: "npm install -g @openai/codex",
};

/// The model of its menu until an app-server of the account has listed them (`model/list`): the
/// one its real run had by default.
const MODELS: [(&str, &str); 1] = [("gpt-6-luna", "GPT-6-Luna")];

/// The efforts of its models (`supportedReasoningEfforts`, those Escouade's menu has).
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// What the next turn is started with (Codex takes them at `turn/start`).
#[derive(Debug, Clone, Default)]
pub(crate) struct Settings {
    pub model: String,
    pub effort: String,
    /// Escouade's mode (`default`, `plan`…), turned into Codex's by `mode_params`.
    pub mode: String,
    /// The paths its sandbox refuses (« Projet privé »), as Codex names them: a permissions
    /// profile of Escouade's in place of the sandbox of its mode (`fence_profile`).
    pub fence: Vec<String>,
    /// A discussion of the Docs tab (ESC-261): read only whatever its mode (`Settings::sandboxed`).
    pub read_only: bool,
}

impl Settings {
    /// The mode its sandbox is of: Plan's, read only, for a discussion.
    pub(crate) fn sandboxed(&self) -> &str {
        sandboxed(&self.mode, self.read_only)
    }
}

/// The mode whose sandbox an agent in `mode` has: Plan's, read only, when it is `read_only` (a
/// discussion, ESC-261), which writes nothing but through Escouade's tools; its own otherwise.
pub(crate) fn sandboxed(mode: &str, read_only: bool) -> &str {
    if read_only {
        "plan"
    } else {
        mode
    }
}

/// « Projet privé » (ESC-140): Codex's permissions profiles, its sandbox with the paths of a fence
/// refused (`deny`: neither read nor written by its commands, tried on 0.163.0 under Windows).
/// Its sandbox enforces them: none in « Sans permissions », which has none.
pub(crate) const FENCE_PROFILE: &str = "escouade-fence";
/// The same, read only: the mode Plan.
pub(crate) const FENCE_PLAN_PROFILE: &str = "escouade-fence-plan";

/// The profile of a fenced agent in `mode`; None in « Sans permissions » (no sandbox).
pub(crate) fn fence_profile(mode: &str) -> Option<&'static str> {
    match mode {
        "bypassPermissions" => None,
        "plan" => Some(FENCE_PLAN_PROFILE),
        _ => Some(FENCE_PROFILE),
    }
}

/// The options (`-c`) of a process of Codex that declare both profiles with `paths` refused, and
/// `default` its profile (`default_permissions`, which Codex wants once it has profiles). Declared
/// for the process, not a thread's `config`: Codex reads its config again for a turn that names
/// its profile, and a thread's is not in it (« failed to load workspace requirements », tried on
/// 0.163.0). The plan's holds the paths, read only; the other is it with its folder written, as
/// `workspace-write` is (the paths once: a `.cmd` launcher takes 8,191 characters). A path of
/// Windows with `/`: Codex reads it so, its `\` an escape in a string of its config.
pub(crate) fn fence_args(paths: &[String], default: &str) -> Vec<String> {
    let deny: Vec<String> = paths
        .iter()
        .map(|p| {
            let p = if p.starts_with('/') {
                p.clone()
            } else {
                p.replace('\\', "/")
            };
            format!("{}=\"deny\"", toml_string(&p))
        })
        .collect();
    vec![
        "-c".into(),
        format!("default_permissions={}", toml_string(default)),
        "-c".into(),
        format!(
            "permissions.{FENCE_PLAN_PROFILE}={{extends=\":read-only\", filesystem={{{}}}}}",
            deny.join(", ")
        ),
        "-c".into(),
        format!(
            "permissions.{FENCE_PROFILE}={{extends=\"{FENCE_PLAN_PROFILE}\", filesystem={{\":workspace_roots\"=\"write\", \":tmpdir\"=\"write\"}}}}"
        ),
    ]
}

/// `s` as a string of TOML (JSON's escapes are TOML's).
fn toml_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// The sandbox of a thread in `mode` (`thread/start`): read only to plan, none without
/// permissions, else the agent's folder.
pub(crate) fn sandbox_mode(mode: &str) -> &'static str {
    match mode {
        "plan" => "read-only",
        "bypassPermissions" => "danger-full-access",
        _ => "workspace-write",
    }
}

/// Escouade's modes in Codex (docs/TODO-1.8.md, « Composer : modèles, effort, modes »): its approval
/// policy, its sandbox, who reviews the approvals, and its collaboration mode (the mode Plan).
pub(crate) fn mode_params(s: &Settings, _cwd: &str) -> Value {
    let (approval, sandbox, reviewer) = match s.sandboxed() {
        "plan" => ("on-request", json!({ "type": "readOnly" }), "user"),
        "bypassPermissions" => ("never", json!({ "type": "dangerFullAccess" }), "user"),
        "auto" => ("on-request", workspace_write(), "auto_review"),
        // « Accepter les modifications »: the sandbox lets it write in its folder, and Escouade
        // accepts the git commands of its worktree (`agent/codex.rs`).
        _ => ("on-request", workspace_write(), "user"),
    };
    let profile = (!s.fence.is_empty())
        .then(|| fence_profile(s.sandboxed()))
        .flatten();
    let collaboration = if s.mode == "plan" { "plan" } else { "default" };
    let mut settings = json!({ "model": s.model, "developer_instructions": null });
    if !s.effort.is_empty() {
        settings["reasoning_effort"] = json!(s.effort);
    }
    let mut p = json!({
        "approvalPolicy": approval,
        "approvalsReviewer": reviewer,
        "sandboxPolicy": sandbox,
        "collaborationMode": { "mode": collaboration, "settings": settings },
    });
    // Fenced: its profile, which Codex takes in place of a sandbox policy (not both).
    if let (Some(profile), Some(o)) = (profile, p.as_object_mut()) {
        o.remove("sandboxPolicy");
        o.insert("permissions".into(), json!(profile));
    }
    p
}

fn workspace_write() -> Value {
    json!({ "type": "workspaceWrite", "writableRoots": [], "networkAccess": false })
}

/// The mode of Escouade a thread's settings say (`thread/settings/updated`), given the agent's
/// `current` one: « Demander » and « Accepter les modifications » are the same for Codex.
pub(crate) fn mode_of(settings: &Value, current: &str) -> Option<&'static str> {
    let collab = settings["collaborationMode"]["mode"].as_str();
    let approval = settings["approvalPolicy"].as_str()?;
    let sandbox = settings["sandboxPolicy"]["type"].as_str();
    Some(if collab == Some("plan") {
        "plan"
    } else if approval == "never" && sandbox == Some("dangerFullAccess") {
        "bypassPermissions"
    } else if settings["approvalsReviewer"] == "auto_review" {
        "auto"
    } else if current == "acceptEdits" {
        "acceptEdits"
    } else {
        "default"
    })
}

/// The id of the card of a plan to approve, Codex's turn over on it (`agent/codex.rs`).
pub(crate) fn plan_approval(turn: &str) -> String {
    format!("plan:{turn}")
}

pub(crate) fn is_plan_approval(request_id: &str) -> bool {
    request_id.starts_with("plan:")
}

/// The model the `config.toml` of an account names (`model = "…"`, at its top), read only: None
/// when it names none, or is no TOML.
pub(crate) fn config_model(text: &str) -> Option<String> {
    let doc: toml_edit::DocumentMut = text.parse().ok()?;
    let model = doc.get("model")?.as_str()?.trim();
    (!model.is_empty()).then(|| model.to_string())
}

/// `listed` (`model/list`) and, after them, the model of the account's `config.toml` (in its folder
/// `home`) when they lack it (ESC-186: one Codex runs by its name, `gpt-5.6-sol`, that its picker
/// does not list): marked as coming from there, with no effort of its own (its config's serves).
pub(crate) fn with_config_model(mut listed: Vec<ToldModel>, home: &Path) -> Vec<ToldModel> {
    let Some(model) = std::fs::read_to_string(home.join("config.toml"))
        .ok()
        .as_deref()
        .and_then(config_model)
    else {
        return listed;
    };
    if listed.iter().any(|m| m.id == model) {
        return listed;
    }
    listed.push(ToldModel {
        from_config: true,
        label: model.clone(),
        id: model,
        efforts: Vec::new(),
        default_effort: None,
        images: false,
        default: false,
        provider: None,
        multiplier: None,
        price: None,
    });
    listed
}

/// The models of `model/list`, as the composer's menu offers them.
pub(crate) fn catalog(list: &Value) -> Option<Vec<ToldModel>> {
    let models: Vec<ToldModel> = list["data"]
        .as_array()?
        .iter()
        .filter(|m| m["hidden"] != true)
        .filter_map(|m| {
            let id = m["model"].as_str().or_else(|| m["id"].as_str())?;
            Some(ToldModel {
                from_config: false,
                id: id.to_string(),
                label: m["displayName"].as_str().unwrap_or(id).to_string(),
                efforts: m["supportedReasoningEfforts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|e| e["reasoningEffort"].as_str().map(str::to_string))
                    .collect(),
                default_effort: m["defaultReasoningEffort"].as_str().map(str::to_string),
                images: m["inputModalities"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|x| x == "image")),
                default: m["isDefault"] == true,
                provider: None,
                multiplier: None,
                price: None,
            })
        })
        .collect();
    (!models.is_empty()).then_some(models)
}

/// The windows of a quota of Codex (`rateLimits` of `account/rateLimits/*`), as meters: each
/// labelled by its length (5 hours, a week, 30 days on the free plan).
pub(crate) fn meters(limits: &Value) -> Vec<QuotaMeter> {
    let mut all: Vec<QuotaMeter> = ["primary", "secondary"]
        .iter()
        .filter_map(|k| {
            let w = limits.get(*k).filter(|w| w.is_object())?;
            let minutes = w["windowDurationMins"].as_u64()? as u32;
            Some(QuotaMeter::window(
                minutes,
                RateWindow {
                    pct: w["usedPercent"].as_f64().unwrap_or(0.0),
                    resets_at: w["resetsAt"].as_i64().map(|s| s * 1000),
                },
            ))
        })
        .collect();
    all.sort_by_key(|m| m.minutes);
    all
}

/// The answers of a question's card (`input.answers`, by question as Escouade keeps them) in the
/// form `item/tool/requestUserInput` takes: by the question's id, a list of answers.
pub(crate) fn answers(input: &Value) -> Value {
    let given = &input["answers"];
    let mut out = serde_json::Map::new();
    for q in input["questions"].as_array().into_iter().flatten() {
        let (Some(id), Some(text)) = (q["id"].as_str(), q["question"].as_str()) else {
            continue;
        };
        let answer = match &given[text] {
            Value::String(s) => vec![s.clone()],
            Value::Array(a) => a
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => continue,
        };
        out.insert(id.to_string(), json!({ "answers": answer }));
    }
    Value::Object(out)
}

/// What a mode means in Codex, as the menu says it.
fn meaning(lang: Lang, mode: &str) -> String {
    match mode {
        "auto" => tr_in!(
            lang,
            "Un relecteur automatique de Codex décide des demandes à ta place. Bac à sable : écriture dans le dossier de l’agent, réseau coupé.",
            "An automatic reviewer of Codex decides on the requests for you. Sandbox: writing in the agent’s folder, network off."
        ),
        "plan" => tr_in!(
            lang,
            "Codex étudie et propose un plan sans rien modifier (bac à sable en lecture seule). Ses questions arrivent dans ce mode.",
            "Codex studies and proposes a plan without changing anything (read-only sandbox). Its questions come in this mode."
        ),
        "acceptEdits" => tr_in!(
            lang,
            "Bac à sable : écriture dans le dossier de l’agent, réseau coupé. Escouade accepte d’office git add et git commit dans le worktree de l’agent ; le reste demande.",
            "Sandbox: writing in the agent’s folder, network off. Escouade accepts at once git add and git commit in the agent’s worktree; the rest asks."
        ),
        "bypassPermissions" => tr_in!(
            lang,
            "Codex ne demande rien et sort du bac à sable : tout le disque et le réseau.",
            "Codex asks nothing and leaves its sandbox: the whole disk and the network."
        ),
        _ => tr_in!(
            lang,
            "Bac à sable : écriture dans le dossier de l’agent, réseau coupé. Une commande qui en sort demande, git add et git commit compris.",
            "Sandbox: writing in the agent’s folder, network off. A command that leaves it asks, git add and git commit included."
        ),
    }
}

/// What Codex does, as Escouade drives it.
pub fn caps() -> EngineCaps {
    let lang = i18n::ui();
    EngineCaps {
        modes: [
            "auto",
            "default",
            "plan",
            "acceptEdits",
            "bypassPermissions",
        ]
        .into_iter()
        .map(|id| ModeCap {
            id,
            meaning: Some(meaning(lang, id)),
        })
        .collect(),
        effort: true,
        images: true,
        // Codex reads no PDF (docs/TODO-1.8.md, « Pièces jointes »).
        pdf: false,
        fork: ForkPoint::Turn,
        // To be tried with a second account (docs/TODO-1.8.md, point 0).
        resume_other_account: false,
        mid_turn: MidTurn::Immediate,
        // `/compact` alone, by its request (`compaction`, ESC-205).
        slash_commands: true,
        quota: QuotaKind::Windows,
        quota_windows: vec![crate::model::FIVE_HOURS, crate::model::WEEK],
        // Estimated from its tokens and the public prices of OpenAI's models (`pricing`).
        cost: CostKind::Estimated,
        terminal_view: true,
        remote_control: false,
        background_tasks: false,
        todo_tools: false,
        default_refusal: false,
        sandbox: true,
        usage_page: None,
        accounts_by_key: false,
        read_only: true,
    }
}

/// The arguments of its app-server: the plan tool given to the model (it no longer has it by
/// default, tried), then what it is given of Escouade's server.
fn args(s: &Spawn) -> Vec<String> {
    let mut a: Vec<String> = ["app-server", "-c", "tools.update_plan.enabled=true"]
        .map(String::from)
        .to_vec();
    // Refused, an entry of the user's is turned off; with none, nothing is passed: an entry made
    // of `enabled` alone has no transport, and Codex does not start (« invalid transport », tried
    // on 0.163.0).
    let declared = || {
        let home = s
            .env
            .iter()
            .find(|(k, _)| k == "CODEX_HOME")
            .map(|(_, v)| PathBuf::from(v))
            .or_else(|| dirs::home_dir().map(|h| h.join(".codex")));
        home.and_then(|h| std::fs::read_to_string(h.join("config.toml")).ok())
            .is_some_and(|text| declares_escouade(&text))
    };
    // « Projet privé »: its profiles, which its threads and turns name (`mode_params`).
    if !s.fence.is_empty() {
        let paths = crate::fence::spelled(&s.fence, &s.program);
        a.extend(fence_args(
            &paths,
            fence_profile(sandboxed(&s.mode, s.read_only)).unwrap_or(FENCE_PROFILE),
        ));
    }
    if !matches!(s.escouade, Escouade::Denied { .. }) || declared() {
        a.extend(escouade_launch(&s.escouade).args);
    }
    a
}

/// Whether a `config.toml` of Codex declares a server named `escouade` (`[mcp_servers.escouade]`,
/// one of its subtables, or a key of `[mcp_servers]`).
fn declares_escouade(text: &str) -> bool {
    let mut in_servers = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            let table: String = line
                .chars()
                .filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
                .collect();
            if table.starts_with(&format!("[mcp_servers.{}", mcp::SERVER)) {
                return true;
            }
            in_servers = table == "[mcp_servers]";
            continue;
        }
        let key = line
            .split(['=', '.'])
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches(['"', '\'']);
        if (in_servers && key == mcp::SERVER)
            || line.starts_with(&format!("mcp_servers.{}", mcp::SERVER))
        {
            return true;
        }
    }
    false
}

/// Escouade's server for its app-server, by `-c` alone (never by the `config` of `thread/start`,
/// which would drop them, tried): its address and the variable of its token; refused, an entry of
/// that name turned off, the user's own included. The token is in the process's environment.
fn escouade_launch(escouade: &Escouade) -> mcp::Launch {
    const VAR: &str = "ESCOUADE_MCP_TOKEN";
    let server = mcp::SERVER;
    match escouade {
        Escouade::Granted { url, token, .. } => mcp::Launch {
            args: vec![
                "-c".into(),
                format!("mcp_servers.{server}.url={url}"),
                "-c".into(),
                format!("mcp_servers.{server}.bearer_token_env_var={VAR}"),
            ],
            env: vec![(VAR.into(), token.clone())],
        },
        Escouade::Denied { .. } => mcp::Launch {
            args: vec!["-c".into(), format!("mcp_servers.{server}.enabled=false")],
            env: Vec::new(),
        },
    }
}

/// A message to Codex: its text, the images joined to it (by data URL), the text files joined to
/// it as text. A PDF is refused: Codex does not read them.
fn user_input(text: &str, attachments: &[Attachment]) -> Result<Value> {
    let mut input = Vec::new();
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
                input.push(json!({ "type": "image", "url": format!("data:{t};base64,{}", a.data) }));
            }
            "text/plain" => input.push(json!({
                "type": "text",
                "text": format!("{}:\n\n{}", a.name, a.data),
            })),
            t => bail!(tr!(
                "« {name} » ({t}) ne peut pas être joint : Codex lit les images et les fichiers texte, pas les PDF ni les autres fichiers.",
                "“{name}” ({t}) can’t be attached: Codex reads images and text files, not PDFs nor other files.",
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
    if !text.is_empty() || input.is_empty() {
        input.push(json!({ "type": "text", "text": text }));
    }
    Ok(Value::Array(input))
}

/// Its entry in each account's `config.toml` (« Codex peut piloter Escouade »): `codex mcp add`
/// takes no header, and `bearer_token` is refused for an HTTP entry. Tried on 0.163.0: the entry
/// reaches the server with its header; an agent's `-c … bearer_token_env_var` wins over it (its
/// own token sent, not the user's).
static USER_CONFIG: UserConfig = UserConfig {
    files: &["config.toml"],
    format: Format::Toml,
    servers: &["mcp_servers"],
    url: "url",
    headers: "http_headers",
    fixed: &[],
};

impl Engine for Codex {
    fn store(&self) -> crate::accounts::Store {
        use crate::accounts::{Store, CODEX_SHARED_DIRS, CODEX_SHARED_FILES};
        Store {
            home: ".codex",
            var: "CODEX_HOME",
            files: &CODEX_SHARED_FILES,
            dirs: &CODEX_SHARED_DIRS,
            login: &["login"],
            xdg: None,
        }
    }

    /// The sign-in of its folder (`auth.json`), and the email it names.
    fn account_status<'a>(
        &'a self,
        account: &'a crate::model::Account,
        settings: &'a crate::model::Settings,
    ) -> BoxFuture<'a, crate::accounts::AccountStatus> {
        let dir = crate::accounts::config_dir(account, settings);
        let signed = crate::accounts::codex_sign_in(&dir);
        Box::pin(async move {
            crate::accounts::AccountStatus {
                connected: signed.is_some(),
                stamp: signed.as_ref().and_then(|s| s.stamp.clone()),
                email: signed.and_then(|s| s.email),
                dir: dir.to_string_lossy().into_owned(),
            }
        })
    }

    /// `.agents/`, read whole whatever the harness (ESC-195): skills, agents, commands, prompts.
    fn workflow_folders(&self) -> crate::workflow::Folders {
        use crate::workflow::DocKind;
        &[
            (".agents/skills", DocKind::Skill),
            (".agents/agents", DocKind::Agent),
            (".agents/commands", DocKind::Command),
            (".agents/prompts", DocKind::Command),
        ]
    }

    fn workflow_user(&self) -> Option<crate::workflow::UserRoot> {
        Some(crate::workflow::UserRoot::Codex)
    }

    fn hook_context(&self) -> bool {
        true
    }

    /// Its subagents are not told (ESC-196).
    fn track_missing(&self, _view: bool) -> &'static [&'static str] {
        &["subagents"]
    }

    fn id(&self) -> EngineId {
        EngineId::CODEX
    }

    fn listed_models_only(&self) -> bool {
        true
    }

    fn session_home(&self) -> Option<crate::fence::HomeKind> {
        Some(crate::fence::HomeKind::Codex)
    }

    fn principal_sessions(&self) -> bool {
        true
    }

    fn account_models(&self, listed: Vec<ToldModel>, config_dir: &Path) -> Vec<ToldModel> {
        with_config_model(listed, config_dir)
    }

    fn name(&self) -> &'static str {
        "Codex"
    }

    fn speaker(&self) -> &'static str {
        "Codex"
    }

    /// `developerInstructions` of `thread/start`: no command line to fit in.
    fn instructions_room(&self) -> Option<Room> {
        None
    }

    /// Its subagents are written to by Codex's own tools (`send_input`), the agent passing it on.
    fn subagent_relay(&self, lang: Lang, title: &str, task: &str, text: &str) -> Option<String> {
        Some(tr_in!(
            lang,
            "Transmets ce message à ton sous-agent « {title} » ({task}), tel quel, puis reprends ce que tu faisais :

{text}",
            "Pass this message on to your subagent “{title}” ({task}), as it is, then go back to what you were doing:

{text}"
        ))
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
            "son app-server n’a été vérifié qu’à partir de celle-ci (docs/PROTOCOL.md)",
            "its app-server was only checked from that one on (docs/PROTOCOL.md)"
        )
    }

    fn update_command(&self) -> &'static str {
        "npm install -g @openai/codex@latest"
    }

    fn version_args(&self) -> &'static [&'static str] {
        &["--version"]
    }

    fn caps(&self) -> EngineCaps {
        caps()
    }

    fn accounts(&self) -> super::registry::AccountStore {
        super::registry::AccountStore::ConfigDir { var: "CODEX_HOME" }
    }

    fn terminal(&self) -> Option<&'static dyn super::TerminalView> {
        Some(&terminal::CODEX_TERMINAL)
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
                auto: true,
            })
            .collect()
    }

    fn efforts(&self) -> &'static [&'static str] {
        &EFFORTS
    }

    /// Not signed in (no `auth.json`): said so, no process started. Else `account/rateLimits/read`
    /// of an app-server of its own (`process::read_limits`).
    fn read_quota<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a Path,
    ) -> BoxFuture<'a, Result<Option<crate::usage::Reading>>> {
        Box::pin(async move {
            if crate::accounts::codex_sign_in(dir).is_none() {
                return Ok(Some(crate::usage::Reading::NotSignedIn));
            }
            let cwd = std::env::temp_dir().to_string_lossy().into_owned();
            let limits = process::read_limits(program, env, cwd).await?;
            Ok(Some(crate::usage::Reading::Windows(meters(&limits))))
        })
    }

    /// Every page of `model/list`, by an app-server of its own (`process::read_models`, ESC-186).
    fn list_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a Path,
    ) -> BoxFuture<'a, Result<Option<Vec<ToldModel>>>> {
        Box::pin(async move {
            let list =
                process::read_models(program, env, dir.to_string_lossy().into_owned()).await?;
            Ok(catalog(&list))
        })
    }

    fn supports_effort(&self, _model: &str) -> bool {
        true
    }

    fn supports_auto(&self, _model: &str) -> bool {
        true
    }

    /// `modelContextWindow` of its real run.
    fn context_window(&self, _model: &str) -> u64 {
        258_400
    }

    fn spawn(
        &self,
        spawn: Spawn,
        on_frame: super::OnFrame,
        on_exit: super::OnExit,
    ) -> Result<Process> {
        let mut env = spawn.env.clone();
        env.extend(escouade_launch(&spawn.escouade).env);
        let args = args(&spawn);
        log::info!(
            "{}: starting {} {} in {} (account {})",
            spawn.label,
            spawn.program.describe(),
            args.join(" "),
            spawn.cwd,
            spawn.account
        );
        let settings = Settings {
            model: spawn.model.clone(),
            effort: spawn.effort.clone(),
            mode: spawn.mode.clone(),
            fence: crate::fence::spelled(&spawn.fence, &spawn.program),
            read_only: spawn.read_only,
        };
        let instructions = spawn.append_prompt.clone().filter(|p| !p.trim().is_empty());
        let opts = process::Opts {
            program: spawn.program,
            cwd: spawn.cwd,
            args,
            env,
            start: spawn.start,
            settings,
            instructions,
        };
        let p: Process = CodexProcess::spawn(opts, on_frame, on_exit)?;
        Ok(p)
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        user_input(text, attachments).map(Message)
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        rt.codex_frame(frame, fx);
    }

    /// A thread that cannot be resumed is told by `thread/resume`'s answer: its process starts a
    /// new one (`initialize`).
    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    /// `codex exec`, ephemeral, in a read-only sandbox (`exec`).
    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(exec::one_shot(ask))
    }

    fn writer(&self, _weight: Weight) -> &'static str {
        exec::WRITER
    }

    fn grant_file(&self, _url: &str, _token: &str) -> Option<Vec<u8>> {
        None
    }

    fn refusal(&self) -> Option<Value> {
        None
    }

    fn escouade_launch(&self, escouade: &Escouade) -> mcp::Launch {
        escouade_launch(escouade)
    }

    fn user_config(&self) -> Option<&'static UserConfig> {
        Some(&USER_CONFIG)
    }

    /// `thread/compact/start`, which takes only the thread.
    fn compaction(&self) -> Compaction {
        Compaction::Bare
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fenced_agent_runs_in_escouades_profile_of_its_mode_with_the_paths_refused() {
        let of = |mode: &str| {
            mode_params(
                &Settings {
                    mode: mode.into(),
                    fence: vec!["C:/code/secret".into()],
                    ..Default::default()
                },
                "C:/p",
            )
        };
        // In place of a sandbox policy: Codex takes one or the other.
        for (mode, profile) in [
            ("default", FENCE_PROFILE),
            ("acceptEdits", FENCE_PROFILE),
            ("auto", FENCE_PROFILE),
            ("plan", FENCE_PLAN_PROFILE),
        ] {
            let p = of(mode);
            assert_eq!(p["permissions"], profile, "{mode}");
            assert!(p.get("sandboxPolicy").is_none(), "{mode}");
        }
        // « Sans permissions » has no sandbox to refuse anything.
        let bypass = of("bypassPermissions");
        assert!(bypass.get("permissions").is_none());
        assert_eq!(bypass["sandboxPolicy"]["type"], "dangerFullAccess");
        // Declared for its process: the paths once, in the plan's, which the other extends.
        let args = fence_args(
            &[r"C:\code\secret".into(), "/home/me/x".into()],
            FENCE_PLAN_PROFILE,
        );
        assert_eq!(
            args,
            [
                "-c",
                "default_permissions=\"escouade-fence-plan\"",
                "-c",
                "permissions.escouade-fence-plan={extends=\":read-only\", filesystem={\"C:/code/secret\"=\"deny\", \"/home/me/x\"=\"deny\"}}",
                "-c",
                "permissions.escouade-fence={extends=\"escouade-fence-plan\", filesystem={\":workspace_roots\"=\"write\", \":tmpdir\"=\"write\"}}",
            ]
        );
    }

    #[test]
    fn a_discussion_is_read_only_in_every_mode_its_collaboration_its_own() {
        let of = |mode: &str, fence: Vec<String>| {
            mode_params(
                &Settings {
                    mode: mode.into(),
                    fence,
                    read_only: true,
                    ..Default::default()
                },
                "C:/p",
            )
        };
        for mode in ["default", "acceptEdits", "auto", "bypassPermissions"] {
            let p = of(mode, Vec::new());
            assert_eq!(p["sandboxPolicy"]["type"], "readOnly", "{mode}");
            assert_eq!(p["approvalPolicy"], "on-request", "{mode}");
            assert_eq!(p["approvalsReviewer"], "user", "{mode}");
            // Not made to plan: it answers.
            assert_eq!(p["collaborationMode"]["mode"], "default", "{mode}");
            // Fenced: the read only profile.
            let p = of(mode, vec!["C:/code/secret".into()]);
            assert_eq!(p["permissions"], FENCE_PLAN_PROFILE, "{mode}");
        }
        let s = Settings {
            mode: "default".into(),
            read_only: true,
            ..Default::default()
        };
        assert_eq!(sandbox_mode(s.sandboxed()), "read-only");
        assert_eq!(sandboxed("acceptEdits", false), "acceptEdits");
    }

    #[test]
    fn each_mode_of_escouade_has_its_policy_and_sandbox_in_codex() {
        let of = |mode: &str| {
            mode_params(
                &Settings {
                    model: "gpt-6-luna".into(),
                    effort: "low".into(),
                    mode: mode.into(),
                    ..Default::default()
                },
                "C:/p",
            )
        };
        let plan = of("plan");
        assert_eq!(plan["collaborationMode"]["mode"], "plan");
        assert_eq!(plan["sandboxPolicy"]["type"], "readOnly");
        assert_eq!(
            plan["collaborationMode"]["settings"]["reasoning_effort"],
            "low"
        );
        let ask = of("default");
        assert_eq!(
            (
                ask["approvalPolicy"].as_str(),
                ask["sandboxPolicy"]["type"].as_str()
            ),
            (Some("on-request"), Some("workspaceWrite"))
        );
        assert_eq!(ask["collaborationMode"]["mode"], "default");
        assert_eq!(ask["sandboxPolicy"]["networkAccess"], false);
        assert_eq!(of("acceptEdits")["sandboxPolicy"], ask["sandboxPolicy"]);
        assert_eq!(of("auto")["approvalsReviewer"], "auto_review");
        let bypass = of("bypassPermissions");
        assert_eq!(
            (
                bypass["approvalPolicy"].as_str(),
                bypass["sandboxPolicy"]["type"].as_str()
            ),
            (Some("never"), Some("dangerFullAccess"))
        );
        assert_eq!(sandbox_mode("plan"), "read-only");
        assert_eq!(sandbox_mode("acceptEdits"), "workspace-write");
        assert_eq!(sandbox_mode("bypassPermissions"), "danger-full-access");
        // And back, from what a thread's settings say.
        for mode in [
            "plan",
            "default",
            "acceptEdits",
            "auto",
            "bypassPermissions",
        ] {
            assert_eq!(mode_of(&of(mode), mode), Some(mode), "{mode}");
        }
        assert_eq!(mode_of(&of("default"), "plan"), Some("default"));
    }

    #[test]
    fn every_mode_tells_what_it_means_in_codex_in_both_languages() {
        for mode in [
            "auto",
            "default",
            "plan",
            "acceptEdits",
            "bypassPermissions",
        ] {
            let (fr, en) = (meaning(Lang::Fr, mode), meaning(Lang::En, mode));
            assert!(!fr.is_empty() && !en.is_empty() && fr != en, "{mode}");
        }
        assert!(meaning(Lang::Fr, "plan").contains("lecture seule"));
        assert!(meaning(Lang::En, "default").contains("network off"));
        let caps = caps();
        assert_eq!(caps.modes.len(), 5);
        assert!(caps.modes.iter().all(|m| m.meaning.is_some()));
    }

    #[test]
    fn the_models_and_their_efforts_come_from_model_list() {
        let list = crate::engine::codex::bench::recorded("account.jsonl")
            .into_iter()
            .find(|m| m["result"]["data"].is_array())
            .expect("the answer to model/list");
        let models = catalog(&list["result"]).unwrap();
        let luna = models.iter().find(|m| m.id == "gpt-6-luna").unwrap();
        assert_eq!(luna.label, "GPT-6-Luna");
        assert!(luna.default && luna.images);
        assert_eq!(luna.efforts, ["low", "medium", "high", "xhigh", "max"]);
        assert_eq!(luna.default_effort.as_deref(), Some("medium"));
        assert!(models.iter().filter(|m| m.default).count() == 1);
        assert!(catalog(&json!({ "data": [] })).is_none());
    }

    #[test]
    fn a_quota_window_is_labelled_by_its_length() {
        let m = meters(&json!({
            "primary": { "usedPercent": 2, "windowDurationMins": 43200, "resetsAt": 1794772340 },
            "secondary": null,
        }));
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].minutes, Some(43_200));
        assert_eq!(m[0].resets_at, Some(1_794_772_340_000));
        assert_eq!(m[0].pct, 2.0);
    }

    #[test]
    fn the_answers_of_a_card_go_back_by_the_ids_of_codexs_questions() {
        let input = json!({
            "questions": [
                { "id": "language", "question": "Quel langage ?", "options": [] },
                { "id": "os", "question": "Quel système ?", "options": [] },
            ],
            "answers": { "Quel langage ?": "Python (Recommended)", "Quel système ?": ["Linux", "macOS"] },
        });
        assert_eq!(
            answers(&input),
            json!({
                "language": { "answers": ["Python (Recommended)"] },
                "os": { "answers": ["Linux", "macOS"] },
            })
        );
    }

    #[test]
    fn a_message_takes_its_images_and_text_files_and_refuses_a_pdf() {
        let file = |name: &str, media: &str, data: &str| Attachment {
            name: name.into(),
            media_type: media.into(),
            data: data.into(),
        };
        let input = user_input(
            "Regarde",
            &[
                file("a.png", "image/png", "iVBORw0KGgo="),
                file("notes.txt", "text/plain", "bonjour"),
            ],
        )
        .unwrap();
        assert_eq!(input[0]["type"], "image");
        assert_eq!(input[0]["url"], "data:image/png;base64,iVBORw0KGgo=");
        assert_eq!(input[1]["text"], "notes.txt:\n\nbonjour");
        assert_eq!(input[2], json!({ "type": "text", "text": "Regarde" }));
        assert!(user_input("x", &[file("a.pdf", "application/pdf", "JVBERi0=")]).is_err());
        assert_eq!(
            user_input("bonjour", &[]).unwrap(),
            json!([{ "type": "text", "text": "bonjour" }])
        );
    }

    #[test]
    fn a_refusal_turns_off_only_an_entry_the_user_declares() {
        assert!(declares_escouade(
            "[mcp_servers.escouade]
url = \"http://x\"
"
        ));
        assert!(declares_escouade(
            "[mcp_servers.\"escouade\".env]
A = \"1\"
"
        ));
        assert!(declares_escouade(
            "[mcp_servers]
escouade = { url = \"http://x\" }
"
        ));
        assert!(!declares_escouade(
            "[mcp_servers.autre]
url = \"escouade\"
"
        ));
        assert!(!declares_escouade(
            "model = \"gpt-6-luna\"
[projects.'C:/escouade']
trust_level = \"trusted\"
"
        ));
        let dir = crate::paths::test_dir("esc37-codex-refusal");
        let mut spawn = Spawn::of(
            &crate::model::AgentMeta::default(),
            Program::Native(PathBuf::new()),
            vec![("CODEX_HOME".into(), dir.to_string_lossy().into())],
        );
        spawn.escouade = Escouade::Denied { file: None };
        assert!(!args(&spawn).iter().any(|a| a.contains("mcp_servers")));
        std::fs::write(
            dir.join("config.toml"),
            "[mcp_servers.escouade]
command = \"x\"
",
        )
        .unwrap();
        assert!(args(&spawn).contains(&"mcp_servers.escouade.enabled=false".to_string()));
    }

    #[test]
    fn escouades_server_is_given_by_options_and_its_token_by_the_environment() {
        let granted = escouade_launch(&Escouade::Granted {
            url: "http://127.0.0.1:4100/mcp".into(),
            token: "secret".into(),
            file: None,
        });
        assert!(granted.args.iter().all(|a| !a.contains("secret")));
        assert!(granted
            .args
            .contains(&"mcp_servers.escouade.url=http://127.0.0.1:4100/mcp".to_string()));
        assert_eq!(
            granted.env,
            [("ESCOUADE_MCP_TOKEN".to_string(), "secret".to_string())]
        );
        let denied = escouade_launch(&Escouade::Denied { file: None });
        assert_eq!(denied.args, ["-c", "mcp_servers.escouade.enabled=false"]);
    }
}
