//! OpenCode, the third engine: OpenCode 1 (`opencode-ai`, 1.19) driven through `opencode serve`
//! (HTTP + SSE, docs/PROTOCOL.md, « OpenCode serve »), one server for the agents of an account
//! (`server`), each agent a session of its own (`process`), its events translated by
//! `AgentRt::opencode_frame` (`agent/opencode.rs`).
//!
//! What it does as Escouade drives it comes from the real runs of OpenCode 1.19.3 with local
//! models (ESC-25, docs/TODO-1.8.md, point 0, « OpenCode »), whose streams are in
//! `tests/fixtures/opencode/` and whose schema is `tests/fixtures/engines/opencode-1.19.3.openapi.json`.
//! OpenCode 2 (`opencode v2.…`) is another API, told unsupported (`unsupported_line`).

#[cfg(test)]
mod bench;
mod locate;
mod process;
mod run;
mod server;

#[cfg(test)]
pub(crate) use bench::fake_opencode;

use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
    ToldModel, Version,
};
use super::userconfig::{Format, UserConfig};
use super::{
    mcp, BoxFuture, Engine, Escouade, Message, OneShot, Process, Program, Room, Spawn, StartLost,
    Weight,
};
use crate::agent::{AgentRt, Effects};
use crate::core::{size_label, Attachment, MAX_IMAGE, MAX_TOTAL};
use crate::i18n::{self, Lang};
use crate::model::EngineId;
use anyhow::{bail, Result};
use process::OpencodeProcess;
use reqwest::Method;
use serde_json::{json, Value};

/// OpenCode.
pub struct Opencode;

pub static OPENCODE: Opencode = Opencode;

/// The versions of OpenCode Escouade was tested with: its server as the real run of 2026-10-17
/// found it (ESC-25).
pub const TESTED: Tested = Tested {
    oldest: "1.19.3",
    newest: "1.19.3",
};

/// Its installer, npm's package (OpenCode 1; OpenCode 2 is `@opencode/cli`).
const INSTALL: Install = Install {
    windows: "npm install -g opencode-ai",
    unix: "npm install -g opencode-ai",
};

/// The efforts Escouade's menu has, in its order: a model's `variants` are offered among them.
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// What the next prompt is sent with (OpenCode takes them at each prompt).
#[derive(Debug, Clone, Default)]
pub(crate) struct Settings {
    /// `<provider>/<model>` (`ollama/qwen3-coder:30b-32k`).
    pub model: String,
    /// The model's variant (`low`…), when it has it.
    pub effort: String,
    /// Escouade's mode.
    pub mode: String,
    /// The paths its tools are refused (« Projet privé »), as OpenCode names them: rules of its
    /// session, after its mode's (`rules`).
    pub fence: Vec<String>,
    /// A discussion of the Docs tab (ESC-261): its edits and commands refused, whatever its mode
    /// (`session_rules`).
    pub read_only: bool,
}

/// The provider and the model of `<provider>/<model>` (a model's id may hold more slashes).
pub(crate) fn split_model(model: &str) -> Option<(&str, &str)> {
    let (p, m) = model.split_once('/')?;
    (!p.is_empty() && !m.is_empty()).then_some((p, m))
}

/// OpenCode's agent of Escouade's mode: `plan` studies without changing anything, `build` works.
pub(crate) fn agent_of(mode: &str) -> &'static str {
    if mode == "plan" {
        "plan"
    } else {
        "build"
    }
}

/// The rules of a session in Escouade's mode (OpenCode has no sandbox: only they protect), as
/// Codex's modes are: the files of the agent's folder changed without a question, a command asked,
/// `git add` and `git commit` accepted at once in « Accepter les modifications ». The last rule that
/// matches wins, and a session merges them: each mode gives all of them.
pub(crate) fn rules(mode: &str, fence: &[String]) -> Value {
    let (bash, git, edit, web, outside) = match mode {
        "bypassPermissions" => ("allow", "allow", "allow", "allow", "allow"),
        "acceptEdits" => ("ask", "allow", "allow", "ask", "ask"),
        // The plan agent edits nothing but its plan.
        "plan" => ("ask", "ask", "ask", "ask", "ask"),
        _ => ("ask", "ask", "allow", "ask", "ask"),
    };
    let rule = |p: &str, pattern: &str, a: &str| json!({ "permission": p, "pattern": pattern, "action": a });
    let mut all = vec![
        rule("bash", "*", bash),
        rule("bash", "git add *", git),
        rule("bash", "git commit *", git),
        rule("edit", "*", edit),
        rule("webfetch", "*", web),
        rule("external_directory", "*", outside),
    ];
    all.extend(fence_rules(fence));
    Value::Array(all)
}

/// The rules of the session of an agent of `s`: its mode's, its fence's, and for a discussion
/// (ESC-261) every edit and command refused, last so that they win whatever came before. Refused by
/// OpenCode itself, never a card: Escouade's tools (an MCP server's) stay as they were.
pub(crate) fn session_rules(s: &Settings) -> Value {
    let mut all = rules(&s.mode, &s.fence);
    if s.read_only {
        if let Value::Array(a) = &mut all {
            for p in ["edit", "bash"] {
                a.push(json!({ "permission": p, "pattern": "*", "action": "deny" }));
            }
        }
    }
    all
}

/// « Projet privé » (ESC-140): each path of a fence refused to the tools that take a path (`read`,
/// `edit`, `list`, and `external_directory`, which OpenCode asks of any tool reaching out of the
/// agent's folder), and to a command that names it (`bash`, a guard: a command can reach it
/// without naming it). After the mode's rules: the last that matches wins, whatever the mode. Under
/// Windows, in both spellings (`\` and `/`): `*` of OpenCode's patterns is any character.
pub(crate) fn fence_rules(fence: &[String]) -> Vec<Value> {
    let rule =
        |p: &str, pattern: String| json!({ "permission": p, "pattern": pattern, "action": "deny" });
    let mut out = Vec::new();
    for path in fence {
        let mut spellings = vec![path.clone()];
        if path.contains('\\') {
            spellings.push(path.replace('\\', "/"));
        }
        for p in &spellings {
            let sep = if p.contains('\\') { '\\' } else { '/' };
            for permission in ["read", "edit", "list", "external_directory"] {
                out.push(rule(permission, p.clone()));
                out.push(rule(permission, format!("{p}{sep}*")));
            }
            out.push(rule("bash", format!("*{p}*")));
        }
    }
    out
}

/// The id of the card of a plan to approve, its turn over on it (`agent/opencode.rs`).
pub(crate) fn plan_approval(message: &str) -> String {
    format!("plan:{message}")
}

pub(crate) fn is_plan_approval(request_id: &str) -> bool {
    request_id.starts_with("plan:")
}

/// The answers of a question's card (`input.answers`, by question as Escouade keeps them) as
/// `POST /question/:id/reply` takes them: for each question in order, the labels chosen.
pub(crate) fn answers(input: &Value) -> Value {
    let given = &input["answers"];
    let all: Vec<Value> = input["questions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|q| {
            let text = q["question"].as_str().unwrap_or_default();
            let labels: Vec<String> = match &given[text] {
                Value::String(s) => {
                    // Several chosen, as the card joins them.
                    let options: Vec<&str> = q["options"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|o| o["label"].as_str())
                        .collect();
                    let parts: Vec<&str> = s.split(", ").collect();
                    if q["multiSelect"] == true
                        && parts.len() > 1
                        && parts.iter().all(|p| options.contains(p))
                    {
                        parts.into_iter().map(str::to_string).collect()
                    } else {
                        vec![s.clone()]
                    }
                }
                Value::Array(a) => a
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                _ => Vec::new(),
            };
            json!(labels)
        })
        .collect();
    Value::Array(all)
}

/// The models of the connected providers (`GET /config/providers`), the user's default one
/// (`model` of `GET /config`) marked, each with its variants.
pub(crate) struct Catalog {
    pub models: Vec<ToldModel>,
    /// The context window of each model (`limit.context`), by its id, when OpenCode knows it.
    pub contexts: std::collections::BTreeMap<String, u64>,
}

/// `catalog` from what the server answered.
pub(crate) fn catalog(providers: &Value, config: &Value) -> Catalog {
    let chosen = config["model"].as_str();
    let mut models = Vec::new();
    let mut contexts = std::collections::BTreeMap::new();
    for p in providers["providers"].as_array().into_iter().flatten() {
        let Some(pid) = p["id"].as_str() else {
            continue;
        };
        let pname = p["name"].as_str().unwrap_or(pid).to_string();
        let Some(list) = p["models"].as_object() else {
            continue;
        };
        let mut ids: Vec<&String> = list.keys().collect();
        ids.sort();
        for mid in ids {
            let m = &list[mid];
            if m["status"] == "deprecated" {
                continue;
            }
            let mut efforts: Vec<String> = m["variants"]
                .as_object()
                .map(|v| v.keys().cloned().collect())
                .unwrap_or_default();
            efforts.sort_by_key(|e| EFFORTS.iter().position(|x| x == e).unwrap_or(EFFORTS.len()));
            if let Some(n) = m["limit"]["context"].as_u64().filter(|n| *n > 0) {
                contexts.insert(format!("{pid}/{mid}"), n);
            }
            models.push(ToldModel {
                from_config: false,
                id: format!("{pid}/{mid}"),
                label: m["name"].as_str().unwrap_or(mid).to_string(),
                efforts,
                default_effort: None,
                images: m["capabilities"]["input"]["image"] == true,
                default: false,
                provider: Some(pname.clone()),
                multiplier: None,
                price: None,
            });
        }
    }
    // The default model of the first provider, in OpenCode's order.
    let first_default = providers["providers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["id"].as_str())
        .find_map(|p| providers["default"][p].as_str().map(|m| format!("{p}/{m}")));
    let default = chosen
        .filter(|c| models.iter().any(|m| m.id == *c))
        .map(str::to_string)
        .or(first_default.filter(|d| models.iter().any(|m| m.id == *d)))
        .or_else(|| models.first().map(|m| m.id.clone()));
    for m in &mut models {
        m.default = Some(&m.id) == default.as_ref();
    }
    Catalog { models, contexts }
}

/// The models `opencode models --verbose` lists (`<provider>/<model>`, then its JSON), as
/// `catalog` reads those of the server: its first one the default.
pub(crate) fn listed(text: &str) -> Catalog {
    let mut providers: Vec<(String, serde_json::Map<String, Value>)> = Vec::new();
    let mut current: Option<(String, String, String)> = None;
    // The first listed: its default (a map keeps its keys sorted).
    let mut first: Option<(String, String)> = None;
    let flush = |cur: &mut Option<(String, String, String)>,
                 providers: &mut Vec<(String, serde_json::Map<String, Value>)>| {
        if let Some((p, m, json)) = cur.take() {
            let model = serde_json::from_str::<Value>(&json).unwrap_or_else(|_| json!({}));
            match providers.iter_mut().find(|(id, _)| *id == p) {
                Some((_, models)) => {
                    models.insert(m, model);
                }
                None => {
                    let mut models = serde_json::Map::new();
                    models.insert(m, model);
                    providers.push((p, models));
                }
            }
        }
    };
    for line in text.lines() {
        let head = line.trim_end();
        let starts = !head.starts_with([' ', '{', '}', '"', '\t']) && !head.is_empty();
        match (starts, split_model(head)) {
            (true, Some((p, m))) if !head.contains(' ') => {
                flush(&mut current, &mut providers);
                first.get_or_insert_with(|| (p.to_string(), m.to_string()));
                current = Some((p.to_string(), m.to_string(), String::new()));
            }
            _ => {
                if let Some((_, _, json)) = current.as_mut() {
                    json.push_str(line);
                    json.push('\n');
                }
            }
        }
    }
    flush(&mut current, &mut providers);
    let list: Vec<Value> = providers
        .into_iter()
        .map(|(id, models)| json!({ "id": id, "name": id, "models": models }))
        .collect();
    let mut default = serde_json::Map::new();
    if let Some((p, m)) = first {
        default.insert(p, json!(m));
    }
    catalog(
        &json!({ "providers": list, "default": default }),
        &Value::Null,
    )
}

/// How long `opencode models` may take (3 s on 1.19.3, tried).
const LIST_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

/// The models of the server, read once.
async fn read_catalog(server: &server::Server, cwd: &str) -> Result<Catalog> {
    let mut cached = server.catalog.lock().await;
    if cached.is_none() {
        let providers = server
            .call(Method::GET, "/config/providers", cwd, None)
            .await?;
        let config = server
            .call(Method::GET, "/config", cwd, None)
            .await
            .unwrap_or(Value::Null);
        *cached = Some(json!({ "providers": providers, "config": { "model": config["model"] } }));
    }
    let v = cached.as_ref().unwrap();
    Ok(catalog(&v["providers"], &v["config"]))
}

/// What a mode means in OpenCode, as the menu says it: no sandbox, only its rules.
fn meaning(lang: Lang, mode: &str) -> String {
    match mode {
        "plan" => tr_in!(
            lang,
            "L’agent plan d’OpenCode étudie et propose un plan sans rien modifier. Ses questions arrivent dans ce mode. OpenCode n’a pas de bac à sable : seules ses règles protègent.",
            "OpenCode’s plan agent studies and proposes a plan without changing anything. Its questions come in this mode. OpenCode has no sandbox: only its rules protect."
        ),
        "acceptEdits" => tr_in!(
            lang,
            "Les fichiers du dossier de l’agent, git add et git commit passent sans demander ; les autres commandes, le web et les chemins hors du dossier demandent. Pas de bac à sable : une commande acceptée peut tout faire.",
            "The files of the agent’s folder, git add and git commit go through without asking; other commands, the web and paths outside the folder ask. No sandbox: an accepted command can do anything."
        ),
        "bypassPermissions" => tr_in!(
            lang,
            "OpenCode ne demande rien : commandes, fichiers, web et tout le disque. Pas de bac à sable.",
            "OpenCode asks nothing: commands, files, the web and the whole disk. No sandbox."
        ),
        _ => tr_in!(
            lang,
            "Les fichiers du dossier de l’agent changent sans demander ; les commandes (git add et git commit compris), le web et les chemins hors du dossier demandent. OpenCode n’a pas de bac à sable : seules ces règles protègent, et une commande acceptée peut tout faire.",
            "The files of the agent’s folder change without asking; commands (git add and git commit included), the web and paths outside the folder ask. OpenCode has no sandbox: only these rules protect, and an accepted command can do anything."
        ),
    }
}

/// The modes Escouade gives OpenCode: no `auto` (no reviewer of its own).
const MODES: [&str; 4] = ["default", "plan", "acceptEdits", "bypassPermissions"];

/// What OpenCode does, as Escouade drives it.
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
        images: true,
        // Whether a model reads them depends on the model: refused for now.
        pdf: false,
        fork: ForkPoint::Message,
        resume_other_account: false,
        // A message during a turn would be taken into it, the first one's request lost: held by
        // Escouade until the turn ends.
        mid_turn: MidTurn::Queued,
        // `/compact` alone, by its request (`compaction`, ESC-205).
        slash_commands: true,
        // It goes through the providers' APIs: no quota of its own.
        quota: QuotaKind::None,
        quota_windows: Vec::new(),
        // `cost` of each step, as the provider's prices give it (0 with a local model).
        cost: CostKind::Exact,
        // `opencode attach`, beside Escouade's own client (`EngineProcess::attach`).
        terminal_view: true,
        remote_control: false,
        background_tasks: false,
        todo_tools: false,
        default_refusal: false,
        sandbox: false,
        usage_page: None,
        accounts_by_key: false,
        read_only: true,
    }
}

/// What the server is given on top of the user's config (`OPENCODE_CONFIG_CONTENT`, merged with
/// it): Escouade's server kept out of every folder (an entry of the user's own included), each
/// agent's folder given its own by the API (`process::give_escouade`), no share nor update of its own,
/// and the model of the agent that starts it, which OpenCode would otherwise pick itself (OpenCode
/// Zen without an account, tried).
fn config_content(model: &str) -> Value {
    let mut c = json!({
        "autoupdate": false,
        "share": "disabled",
        "mcp": { mcp::SERVER: { "enabled": false } },
        // The rules of « Demander » for every agent of the server: a subagent's session (`task`)
        // does not take those of its agent's (seen on the real run), and runs its commands without a
        // question by default. Each agent's session has its mode's on top (`rules`).
        "permission": {
            "bash": { "*": "ask", "git add *": "ask", "git commit *": "ask" },
            "edit": "allow",
            "webfetch": "ask",
            "external_directory": "ask",
        },
    });
    if split_model(model).is_some() {
        c["model"] = json!(model);
        c["small_model"] = json!(model);
    }
    c
}

/// A message to OpenCode: its text, the images joined to it (by data URL), the text files joined
/// to it as text. A PDF is refused.
fn parts(text: &str, attachments: &[Attachment]) -> Result<Value> {
    let mut parts = Vec::new();
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
                parts.push(json!({
                    "type": "file", "mime": t, "filename": a.name,
                    "url": format!("data:{t};base64,{}", a.data),
                }));
            }
            "text/plain" => parts.push(json!({
                "type": "text",
                "text": format!("{}:\n\n{}", a.name, a.data),
            })),
            t => bail!(tr!(
                "« {name} » ({t}) ne peut pas être joint : OpenCode lit les images et les fichiers texte, pas les PDF ni les autres fichiers.",
                "“{name}” ({t}) can’t be attached: OpenCode reads images and text files, not PDFs nor other files.",
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
    if !text.is_empty() || parts.is_empty() {
        parts.push(json!({ "type": "text", "text": text }));
    }
    Ok(Value::Array(parts))
}

/// The server of the agents started with `program` and `env` (an account's): one per account.
fn server_key(program: &Program, env: &[(String, String)]) -> String {
    let mut env: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();
    env.sort();
    format!("{}|{}", program.describe(), env.join("|"))
}

/// The folder a server runs in: none of the agents' (a worktree removed would find it held).
fn server_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("escouade-opencode");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// « Les agents peuvent piloter Escouade »: the entry in OpenCode's global config
/// (`$XDG_CONFIG_HOME/opencode`, `accounts::config_dir`), in its `opencode.json` or the
/// `opencode.jsonc` the user has (tried on 1.19.3: both are read, comments and trailing commas
/// too). Escouade's own servers keep it off (`config_content`), each agent's folder given its own.
static USER_CONFIG: UserConfig = UserConfig {
    files: &["opencode.json", "opencode.jsonc", "config.json"],
    format: Format::Jsonc,
    servers: &["mcp"],
    url: "url",
    headers: "headers",
    fixed: &[("type", "remote")],
};

impl Engine for Opencode {
    /// One account in 1.8: its own config (ESC-45), its providers connected by it.
    fn store(&self) -> crate::accounts::Store {
        crate::accounts::Store {
            home: ".config/opencode",
            var: "OPENCODE_CONFIG_DIR",
            files: &[],
            dirs: &[],
            login: &["auth", "login"],
            xdg: Some("opencode"),
        }
    }

    /// It signs in to no account of its own: its providers are its config's.
    fn account_status<'a>(
        &'a self,
        account: &'a crate::model::Account,
        settings: &'a crate::model::Settings,
    ) -> BoxFuture<'a, crate::accounts::AccountStatus> {
        let dir = crate::accounts::config_dir(account, settings);
        Box::pin(async move {
            crate::accounts::AccountStatus {
                connected: true,
                stamp: None,
                email: None,
                dir: dir.to_string_lossy().into_owned(),
            }
        })
    }

    fn workflow_folders(&self) -> crate::workflow::Folders {
        use crate::workflow::DocKind;
        &[
            (".opencode/command", DocKind::Command),
            (".opencode/commands", DocKind::Command),
            (".opencode/skill", DocKind::Skill),
            (".opencode/skills", DocKind::Skill),
            (".opencode/agent", DocKind::Agent),
            (".opencode/agents", DocKind::Agent),
        ]
    }

    fn id(&self) -> EngineId {
        EngineId::OPENCODE
    }

    /// In its data folder, not its configuration's (`fence::homes`).
    fn session_home(&self) -> Option<crate::fence::HomeKind> {
        None
    }

    /// Its providers are its own: no account to sign in to (ESC-45).
    fn signs_in(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "OpenCode"
    }

    fn speaker(&self) -> &'static str {
        "OpenCode"
    }

    /// The `system` of each prompt: no command line to fit in.
    fn instructions_room(&self) -> Option<Room> {
        None
    }

    /// A subagent of OpenCode (the `task` tool) takes no message once it runs.
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
            "son serveur n’a été vérifié qu’à partir de celle-ci (docs/PROTOCOL.md)",
            "its server was only checked from that one on (docs/PROTOCOL.md)"
        )
    }

    /// OpenCode 2 (`opencode v2.…`): another API, still experimental (docs/TODO-1.8.md, point 0).
    fn unsupported_line(&self, lang: Lang, version: &Version) -> Option<String> {
        (*version >= Version::parse("2.0")).then(|| {
            tr_in!(
                lang,
                "OpenCode {version} est la ligne 2 d’OpenCode, que Escouade ne prend pas en charge : son API est une autre, encore expérimentale. Installe OpenCode 1 ({cmd}).",
                "OpenCode {version} is OpenCode’s line 2, which Escouade doesn’t support: its API is another one, still experimental. Install OpenCode 1 ({cmd}).",
                cmd = INSTALL.windows
            )
        })
    }

    fn update_command(&self) -> &'static str {
        "npm install -g opencode-ai@latest"
    }

    fn version_args(&self) -> &'static [&'static str] {
        &["--version"]
    }

    fn caps(&self) -> EngineCaps {
        caps()
    }

    fn accounts(&self) -> super::registry::AccountStore {
        super::registry::AccountStore::ConfigDir {
            var: "OPENCODE_CONFIG_DIR",
        }
    }

    fn attaches(&self) -> bool {
        true
    }

    fn terminal(&self) -> Option<&'static dyn super::TerminalView> {
        None
    }

    fn install(&self) -> Install {
        INSTALL
    }

    /// None until a server has listed those of the connected providers.
    fn models(&self) -> Vec<ModelDecl> {
        Vec::new()
    }

    fn efforts(&self) -> &'static [&'static str] {
        &EFFORTS
    }

    fn supports_effort(&self, _model: &str) -> bool {
        true
    }

    fn supports_auto(&self, _model: &str) -> bool {
        false
    }

    /// `opencode models --verbose`: the models of the providers connected to OpenCode, without a
    /// server (no model is asked anything).
    fn list_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<Option<Vec<ToldModel>>>> {
        Box::pin(async move {
            let args: Vec<String> = ["models", "--verbose"].map(String::from).to_vec();
            let launch = program.launch(&dir.to_string_lossy(), &args, &env)?;
            let mut cmd = tokio::process::Command::new(&launch.program);
            cmd.args(&launch.args).envs(launch.env).current_dir(dir);
            #[cfg(windows)]
            cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
            crate::proc::no_inherited_fds(&mut cmd);
            let out = crate::proc::output_within(cmd, LIST_LIMIT).await?;
            let models = listed(&String::from_utf8_lossy(&out.stdout)).models;
            Ok(Some(models).filter(|m| !m.is_empty()))
        })
    }

    fn spawn(
        &self,
        spawn: Spawn,
        on_frame: super::OnFrame,
        on_exit: super::OnExit,
    ) -> Result<Process> {
        // Never a server without its `model` and `small_model`: OpenCode would pick them itself
        // for what it does on its own (a title, a name; OpenCode Zen without an account, seen at
        // the real run of ESC-25), for every agent the server serves afterwards.
        if split_model(&spawn.model).is_none() {
            bail!(process::no_model(i18n::ui()));
        }
        let key = server_key(&spawn.program, &spawn.env);
        let server = server::acquire(&key, &spawn.cwd, |own| {
            let mut env = spawn.env.clone();
            env.push((
                "OPENCODE_CONFIG_CONTENT".into(),
                config_content(&spawn.model).to_string(),
            ));
            env.push(("OPENCODE_DISABLE_AUTOUPDATE".into(), "1".into()));
            env.push(("OPENCODE_ENABLE_QUESTION_TOOL".into(), "1".into()));
            let dir = server_dir();
            log::info!(
                "{}: starting {} serve in {} (account {})",
                spawn.label,
                spawn.program.describe(),
                dir.display(),
                spawn.account
            );
            server::Server::start(own, &spawn.program, &dir.to_string_lossy(), env)
        })?;
        log::info!(
            "{}: a session of OpenCode in {} (account {})",
            spawn.label,
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
            server,
            cwd: spawn.cwd,
            start: spawn.start,
            settings,
            instructions,
            title: "Escouade".into(),
            escouade: spawn.escouade.clone(),
        };
        let p: Process = OpencodeProcess::new(opts, on_frame, on_exit);
        Ok(p)
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        parts(text, attachments).map(Message)
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        rt.opencode_frame(frame, fx);
    }

    /// A session that cannot be resumed is told by the server: a new one is opened (`initialize`).
    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    /// `opencode run`, on the model Escouade chooses (`run`).
    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(run::one_shot(ask))
    }

    fn writer_takes_model(&self) -> bool {
        true
    }

    fn writer(&self, _weight: Weight) -> &'static str {
        "OpenCode"
    }

    fn grant_file(&self, _url: &str, _token: &str) -> Option<Vec<u8>> {
        None
    }

    fn refusal(&self) -> Option<Value> {
        None
    }

    /// Nothing at the launch of a server shared by agents: each agent's folder is given its entry,
    /// with its own token, by the API (`process::give_escouade`).
    fn escouade_launch(&self, _escouade: &Escouade) -> mcp::Launch {
        mcp::Launch::default()
    }

    fn user_config(&self) -> Option<&'static UserConfig> {
        Some(&USER_CONFIG)
    }

    /// `POST /session/:id/summarize`, which takes no instruction.
    fn compaction(&self) -> super::Compaction {
        super::Compaction::Bare
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_attach_joins_the_session_with_its_password_in_the_environment_only() {
        let a = process::attach_to("http://127.0.0.1:4096", "ses_1", r"C:\wt", "secret");
        assert_eq!(
            a.args,
            [
                "attach",
                "http://127.0.0.1:4096",
                "--session",
                "ses_1",
                "--dir",
                r"C:\wt"
            ]
        );
        assert!(!a.args.iter().any(|x| x.contains("secret")));
        assert_eq!(
            a.env,
            [("OPENCODE_SERVER_PASSWORD".to_string(), "secret".to_string())]
        );
    }

    fn recorded(file: &str) -> Vec<Value> {
        bench::recorded(file)
    }

    #[test]
    fn a_fenced_session_is_refused_each_path_whatever_its_mode() {
        // As OpenCode reads them: the last rule whose permission and pattern match, `*` any
        // character.
        fn matches(pattern: &str, what: &str) -> bool {
            let parts: Vec<&str> = pattern.split('*').collect();
            let mut rest = what;
            for (i, part) in parts.iter().enumerate() {
                if i == 0 {
                    let Some(r) = rest.strip_prefix(part) else {
                        return false;
                    };
                    rest = r;
                } else if i == parts.len() - 1 {
                    return rest.ends_with(part);
                } else {
                    let Some(at) = rest.find(part) else {
                        return false;
                    };
                    rest = &rest[at + part.len()..];
                }
            }
            rest.is_empty()
        }
        let fence = vec!["C:/code/secret".to_string()];
        let action = |mode: &str, p: &str, what: &str| {
            rules(mode, &fence)
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|r| r["permission"] == p && matches(r["pattern"].as_str().unwrap(), what))
                .map(|r| r["action"].as_str().unwrap().to_string())
                .unwrap()
        };
        for mode in ["default", "acceptEdits", "plan", "bypassPermissions"] {
            for p in ["read", "edit", "list", "external_directory"] {
                assert_eq!(action(mode, p, "C:/code/secret"), "deny", "{mode} {p}");
                assert_eq!(
                    action(mode, p, "C:/code/secret/a.txt"),
                    "deny",
                    "{mode} {p}"
                );
            }
            assert_eq!(
                action(mode, "bash", "cat C:/code/secret/a.txt"),
                "deny",
                "{mode}"
            );
        }
        // The rest as the mode says.
        assert_eq!(
            action("bypassPermissions", "external_directory", "C:/code/other"),
            "allow"
        );
        assert_eq!(action("default", "bash", "ls"), "ask");
        // A path of Windows in both spellings.
        let both = fence_rules(&[r"C:\code\secret".to_string()]);
        let patterns: Vec<&str> = both
            .iter()
            .filter(|r| r["permission"] == "read")
            .map(|r| r["pattern"].as_str().unwrap())
            .collect();
        assert_eq!(
            patterns,
            [
                r"C:\code\secret",
                r"C:\code\secret\*",
                "C:/code/secret",
                "C:/code/secret/*"
            ]
        );
    }

    #[test]
    fn a_discussion_has_its_edits_and_commands_refused_in_every_mode() {
        // As OpenCode reads them: the last rule of the permission (patterns `*` here).
        let last = |s: &Settings, p: &str| {
            session_rules(s)
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|r| r["permission"] == p)
                .map(|r| r["action"].as_str().unwrap().to_string())
                .unwrap()
        };
        for mode in ["default", "acceptEdits", "plan", "bypassPermissions"] {
            let s = Settings {
                mode: mode.into(),
                fence: vec!["C:/code/secret".into()],
                read_only: true,
                ..Default::default()
            };
            assert_eq!(last(&s, "edit"), "deny", "{mode}");
            assert_eq!(last(&s, "bash"), "deny", "{mode}");
            // Reads, the web and the paths outside go as the mode says.
            assert_eq!(
                last(&s, "webfetch"),
                rules(mode, &[])[4]["action"],
                "{mode}"
            );
        }
        // Another agent: its mode's rules alone.
        let s = Settings {
            mode: "default".into(),
            ..Default::default()
        };
        assert_eq!(session_rules(&s), rules("default", &[]));
        assert_eq!(last(&s, "edit"), "allow");
    }

    #[test]
    fn each_mode_of_escouade_has_its_rules_and_agent_in_opencode() {
        // As OpenCode reads them: the last rule whose permission and pattern match.
        let action = |mode: &str, p: &str, what: &str| {
            rules(mode, &[])
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|r| {
                    let pattern = r["pattern"].as_str().unwrap();
                    r["permission"] == p
                        && (pattern == "*" || what.starts_with(pattern.trim_end_matches('*')))
                })
                .map(|r| r["action"].as_str().unwrap().to_string())
                .unwrap()
        };
        for (p, what) in [
            ("bash", "ls"),
            ("bash", "git add a"),
            ("edit", "a.py"),
            ("webfetch", "https://x"),
            ("external_directory", "/x"),
        ] {
            assert_eq!(action("bypassPermissions", p, what), "allow", "{p}");
            assert_eq!(action("plan", p, what), "ask", "{p}");
        }
        assert_eq!(action("default", "edit", "a.py"), "allow");
        assert_eq!(action("default", "bash", "git commit -m x"), "ask");
        assert_eq!(action("acceptEdits", "bash", "git add a"), "allow");
        assert_eq!(action("acceptEdits", "bash", "git commit -m x"), "allow");
        assert_eq!(action("acceptEdits", "bash", "rm -rf build"), "ask");
        assert_eq!(action("default", "external_directory", "/x"), "ask");
        assert_eq!(agent_of("plan"), "plan");
        assert_eq!(agent_of("acceptEdits"), "build");
        assert!(!caps().modes.iter().any(|m| m.id == "auto"));
        for mode in MODES {
            let (fr, en) = (meaning(Lang::Fr, mode), meaning(Lang::En, mode));
            assert!(!fr.is_empty() && !en.is_empty() && fr != en, "{mode}");
            assert!(
                fr.contains("bac à sable") && en.contains("sandbox"),
                "{mode}"
            );
        }
    }

    #[test]
    fn the_models_come_from_the_connected_providers_with_their_variants() {
        let answer = recorded("models.jsonl")
            .into_iter()
            .find(|m| m["path"] == "/config/providers")
            .expect("the answer to /config/providers");
        let c = catalog(&answer["body"], &Value::Null);
        let gpt = c
            .models
            .iter()
            .find(|m| m.id == "ollama/gpt-oss:20b-32k")
            .unwrap();
        assert_eq!(gpt.label, "gpt-oss 20b (32k)");
        assert_eq!(gpt.efforts, ["low", "medium", "high"]);
        assert_eq!(gpt.provider.as_deref(), Some("Ollama (local)"));
        let qwen = c
            .models
            .iter()
            .find(|m| m.id == "ollama/qwen3-coder:30b-32k")
            .unwrap();
        assert!(qwen.efforts.is_empty() && !qwen.images);
        assert_eq!(c.models.iter().filter(|m| m.default).count(), 1);
        // The user's own default model, when it is one of them.
        let c = catalog(
            &answer["body"],
            &json!({ "model": "ollama/gpt-oss:20b-32k" }),
        );
        assert!(c.models.iter().find(|m| m.default).unwrap().id == "ollama/gpt-oss:20b-32k");
        assert!(catalog(&json!({ "providers": [] }), &Value::Null)
            .models
            .is_empty());
    }

    #[test]
    fn the_answers_of_a_card_go_back_in_the_order_of_the_questions() {
        let input = json!({
            "questions": [
                { "question": "Quel nom ?", "options": [{ "label": "a" }, { "label": "b" }] },
                { "question": "Quels systèmes ?", "multiSelect": true,
                  "options": [{ "label": "Linux" }, { "label": "macOS" }] },
            ],
            "answers": { "Quels systèmes ?": "Linux, macOS", "Quel nom ?": "a" },
        });
        assert_eq!(answers(&input), json!([["a"], ["Linux", "macOS"]]));
    }

    #[test]
    fn a_message_takes_its_images_and_text_files_and_refuses_a_pdf() {
        let file = |name: &str, media: &str, data: &str| Attachment {
            name: name.into(),
            media_type: media.into(),
            data: data.into(),
        };
        let p = parts(
            "Regarde",
            &[
                file("a.png", "image/png", "iVBORw0KGgo="),
                file("notes.txt", "text/plain", "bonjour"),
            ],
        )
        .unwrap();
        assert_eq!(p[0]["type"], "file");
        assert_eq!(p[0]["url"], "data:image/png;base64,iVBORw0KGgo=");
        assert_eq!(p[1]["text"], "notes.txt:\n\nbonjour");
        assert_eq!(p[2], json!({ "type": "text", "text": "Regarde" }));
        assert!(parts("x", &[file("a.pdf", "application/pdf", "JVBERi0=")]).is_err());
    }

    #[test]
    fn opencode_2_is_told_unsupported() {
        let e = &OPENCODE;
        let v2 = Version::find("opencode v2.0.32").unwrap();
        let said = e.unsupported_line(Lang::Fr, &v2).unwrap();
        assert!(
            said.contains("2.0.32") && said.contains("opencode-ai"),
            "{said}"
        );
        assert!(e
            .unsupported_line(Lang::En, &v2)
            .unwrap()
            .contains("line 2"));
        assert!(e
            .unsupported_line(Lang::Fr, &Version::parse("1.19.3"))
            .is_none());
    }

    #[test]
    fn the_listing_of_opencode_models_gives_the_models_and_its_first_as_default() {
        let text = "ollama/qwen3-coder:30b-32k\n{\n  \"id\": \"qwen3-coder:30b-32k\",\n  \"name\": \"Qwen3 Coder\"\n}\nollama/gpt-oss:20b-32k\n{\n  \"name\": \"gpt-oss\",\n  \"variants\": { \"low\": {}, \"high\": {} }\n}\nopenrouter/a/b\n{\n  \"name\": \"B\"\n}\n";
        let c = listed(text);
        let ids: Vec<&str> = c.models.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "ollama/gpt-oss:20b-32k",
                "ollama/qwen3-coder:30b-32k",
                "openrouter/a/b"
            ]
        );
        assert_eq!(c.models[0].efforts, ["low", "high"]);
        assert!(c.models[1].default, "the first listed");
        assert!(listed("").models.is_empty());
    }

    #[tokio::test]
    async fn no_server_is_started_for_an_agent_without_a_model_of_a_provider() {
        let dir = crate::paths::test_dir("esc44-spawn-no-model");
        let log = dir.join("fake-opencode.jsonl");
        for model in ["", "sonnet"] {
            let mut meta = crate::model::AgentMeta {
                cwd: dir.to_string_lossy().into_owned(),
                model: model.into(),
                ..Default::default()
            };
            meta.engine = EngineId::OPENCODE;
            let program = OPENCODE.locate(&bench::fake_opencode(), "").unwrap();
            let env = vec![("FAKE_OPENCODE_LOG".into(), log.to_string_lossy().into())];
            let spawn = Spawn::of(&meta, program, env);
            let e = OPENCODE
                .spawn(spawn, Box::new(|_| {}), Box::new(|_, _| {}))
                .err()
                .expect("refused");
            assert!(e.to_string().contains("opencode auth login"), "{e}");
        }
        assert!(!log.exists(), "no server started");
    }

    #[test]
    fn the_config_always_gives_a_model_and_keeps_escouades_server_out() {
        let c = config_content("ollama/qwen3-coder:30b-32k");
        assert_eq!(c["model"], "ollama/qwen3-coder:30b-32k");
        assert_eq!(c["small_model"], c["model"]);
        assert_eq!(c["mcp"]["escouade"]["enabled"], false);
        // A subagent asks for its commands too.
        assert_eq!(c["permission"]["bash"]["*"], "ask");
        assert_eq!(c["permission"]["external_directory"], "ask");
        assert!(config_content("sonnet").get("model").is_none());
        assert_eq!(split_model("openrouter/a/b"), Some(("openrouter", "a/b")));
        assert_eq!(split_model("sonnet"), None);
    }
}
