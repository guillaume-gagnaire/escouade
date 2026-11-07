//! GitHub Copilot CLI, the fourth engine: a `copilot --server --stdio` process per agent. Since INT-10.3 its agents
//! converse through its plugin in process (`dev.escouade.copilot-cli`: the client of its server and the translation of
//! its events, `plugins::harness::BUILTIN_HARNESSES`); what stays here is what the contract leaves in the core: its
//! place in the registry, its accounts, an account's quota and models, the texts it writes, its terminal view.
//!
//! What it does as Escouade drives it comes from the real runs of Copilot CLI 1.0.98 on a Copilot
//! Free account (ESC-23, docs/TODO-1.8.md, point 0, « Copilot CLI »), whose streams are in
//! `tests/fixtures/copilot/` and whose schemas are `tests/fixtures/engines/copilot-1.0.98.*`. Its
//! accounts, credits, cost, Escouade's MCP server and its written texts come with ESC-40.

#[cfg(test)]
pub(crate) mod bench;
mod locate;
mod process;
mod prompt;
#[cfg(test)]
mod schema_tests;
pub(crate) mod terminal;
mod words;

#[cfg(test)]
pub(crate) use bench::fake_copilot;
#[cfg(test)]
pub(crate) use words::{
    agent_mode, call_nano, form_content, limit_answer, next_month, permission_mode, prices,
    session_approval,
};
pub(crate) use words::{takes_effort, SECRETS, TOKEN_VAR};

use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
    ToldModel,
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
use crate::model::QuotaMeter;
use anyhow::{bail, Result};
use serde_json::{json, Value};
#[cfg(test)]
use std::collections::HashMap;

/// GitHub Copilot CLI.
pub struct Copilot;

pub static COPILOT: Copilot = Copilot;

/// The versions of Copilot CLI Escouade was tested with: its server as the real run of 2026-10-18
/// found it (ESC-23).
pub const TESTED: Tested = Tested {
    oldest: "1.0.98",
    newest: "1.0.98",
};

/// Its installer, npm's package.
const INSTALL: Install = Install {
    windows: "npm install -g @github/copilot",
    unix: "npm install -g @github/copilot",
};

/// The models of its menu until a process of the account has listed them.
const MODELS: [(&str, &str); 2] = words::MODELS;

/// The efforts Escouade's menu has, in its order: a model's `supportedReasoningEfforts` among them.
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// The modes Escouade gives Copilot, in the menu's order.
const MODES: [&str; 5] = [
    "auto",
    "default",
    "plan",
    "acceptEdits",
    "bypassPermissions",
];

/// What a mode means in Copilot, as the menu says it: no sandbox in its server mode (1.0.98), only
/// Escouade's rules.
fn meaning(lang: Lang, mode: &str) -> String {
    match mode {
        "auto" => tr_in!(
            lang,
            "Copilot enchaîne seul jusqu’au bout de sa tâche (autopilot). Les lectures et écritures du dossier de l’agent, git add et git commit passent ; le reste demande. Pas de bac à sable : une commande acceptée peut tout faire.",
            "Copilot goes on by itself to the end of its task (autopilot). Reads and writes in the agent’s folder, git add and git commit go through; the rest asks. No sandbox: an accepted command can do anything."
        ),
        "plan" => tr_in!(
            lang,
            "Copilot étudie et propose un plan sans rien modifier ; ses lectures dans le dossier de l’agent passent, ses questions arrivent dans ce mode. Pas de bac à sable : seules les règles d’Escouade protègent.",
            "Copilot studies and proposes a plan without changing anything; its reads in the agent’s folder go through, its questions come in this mode. No sandbox: only Escouade’s rules protect."
        ),
        "acceptEdits" => tr_in!(
            lang,
            "Les lectures et écritures du dossier de l’agent, git add et git commit passent sans demander ; les autres commandes, le web et les chemins hors du dossier demandent. Pas de bac à sable : une commande acceptée peut tout faire.",
            "Reads and writes in the agent’s folder, git add and git commit go through without asking; other commands, the web and paths outside the folder ask. No sandbox: an accepted command can do anything."
        ),
        "bypassPermissions" => tr_in!(
            lang,
            "Copilot ne demande rien (allow-all) : commandes, fichiers, web et tout le disque. Pas de bac à sable.",
            "Copilot asks nothing (allow-all): commands, files, the web and the whole disk. No sandbox."
        ),
        _ => tr_in!(
            lang,
            "Les lectures et écritures du dossier de l’agent passent sans demander ; les commandes (git add et git commit compris), le web, les outils MCP et les chemins hors du dossier demandent. Copilot n’a pas de bac à sable en mode serveur : seules ces règles protègent, et une commande acceptée peut tout faire.",
            "Reads and writes in the agent’s folder go through without asking; commands (git add and git commit included), the web, MCP tools and paths outside the folder ask. Copilot has no sandbox in its server mode: only these rules protect, and an accepted command can do anything."
        ),
    }
}

/// What Copilot does, as Escouade drives it.
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
        fork: ForkPoint::Event,
        resume_other_account: false,
        // `immediate`: the running turn takes it at once.
        mid_turn: MidTurn::Immediate,
        // `/compact` alone, by its request (`compaction`, ESC-205).
        slash_commands: true,
        // The credits of the month (`quotaSnapshots`).
        quota: QuotaKind::Credits,
        quota_windows: Vec::new(),
        // Its nano-AIU at the end of each turn (`session.usage_checkpoint`).
        cost: CostKind::Exact,
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

/// The arguments of its server.
fn args() -> Vec<String> {
    words::server_args()
}

/// The models of the account, as `words::catalog` says them.
pub(crate) fn catalog(
    listed: Option<&Value>,
    of_session: Option<&Value>,
) -> Option<Vec<ToldModel>> {
    serde_json::from_value(Value::Array(words::catalog(listed, of_session))).ok()
}

#[cfg(test)]
use words::fenced;

/// What the agent is told of a permission `fenced` refused, in `lang`.
#[cfg(test)]
fn fence_refusal(lang: Lang) -> String {
    words::fence_refusal(lang.tag())
}

/// A message to Copilot: its text, the images joined to it (`blob`), the text files joined to it as
/// text. A PDF is refused.
fn message(text: &str, attachments: &[Attachment]) -> Result<Value> {
    let mut blobs = Vec::new();
    let mut texts = Vec::new();
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
                blobs.push(json!({
                    "type": "blob", "data": a.data, "mimeType": t, "displayName": a.name,
                }));
            }
            "text/plain" => texts.push(format!("{}:\n\n{}", a.name, a.data)),
            t => bail!(tr!(
                "« {name} » ({t}) ne peut pas être joint : Copilot lit les images et les fichiers texte, pas les PDF ni les autres fichiers.",
                "“{name}” ({t}) can’t be attached: Copilot reads images and text files, not PDFs nor other files.",
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
    texts.push(text.to_string());
    let prompt = texts
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut m = json!({ "prompt": prompt });
    if !blobs.is_empty() {
        m["attachments"] = Value::Array(blobs);
    }
    Ok(m)
}

/// The month's credits as Copilot's quota says them, as a meter.
pub(crate) fn credits(snapshots: &Value, now: i64) -> Option<QuotaMeter> {
    words::credits(snapshots, now).map(|c| {
        QuotaMeter::credits(crate::model::RateWindow {
            pct: c.pct,
            resets_at: Some(c.resets_at),
        })
    })
}

/// The private file of a process given Escouade's server: its entry, read by
/// `--additional-mcp-config @<file>`, the token named and not written.
fn grant_file(url: &str) -> Vec<u8> {
    let entry = json!({ "mcpServers": { mcp::SERVER: {
        "type": "http",
        "url": url,
        "headers": { "Authorization": format!("Bearer ${{{TOKEN_VAR}}}") },
        "tools": ["*"],
    } } });
    serde_json::to_vec_pretty(&entry).unwrap_or_default()
}

/// What a process is started with of Escouade's server: given, its entry's file and the token in
/// the environment; refused, `--disable-mcp-server escouade`, which does nothing in the server mode
/// (tried): the process also turns an entry of the user's off for its session
/// (`session.mcp.disable`).
fn escouade_launch(escouade: &Escouade) -> mcp::Launch {
    match escouade {
        Escouade::Granted {
            token,
            file: Some(file),
            ..
        } => mcp::Launch {
            args: vec![
                "--additional-mcp-config".into(),
                format!("@{}", file.to_string_lossy()),
            ],
            env: vec![(TOKEN_VAR.into(), token.clone())],
        },
        _ => mcp::Launch {
            args: vec!["--disable-mcp-server".into(), mcp::SERVER.into()],
            env: Vec::new(),
        },
    }
}

/// « Les agents peuvent piloter Escouade »: the entry in each account's `mcp-config.json`, as
/// `copilot mcp add --transport http` writes it (its `tools` left out: all of them, tried on
/// 1.0.98). An agent of Escouade has its own (`--additional-mcp-config`, which wins), or turns
/// this one off (`session.mcp.disable`).
static USER_CONFIG: UserConfig = UserConfig {
    files: &["mcp-config.json"],
    format: Format::Jsonc,
    servers: &["mcpServers"],
    url: "url",
    headers: "headers",
    fixed: &[("type", "http")],
};

impl Engine for Copilot {
    /// Signed in by `copilot login` in the account's folder (ESC-40).
    fn store(&self) -> crate::accounts::Store {
        use crate::accounts::{Store, COPILOT_SHARED_DIRS, COPILOT_SHARED_FILES};
        Store {
            home: ".copilot",
            var: "COPILOT_HOME",
            files: &COPILOT_SHARED_FILES,
            dirs: &COPILOT_SHARED_DIRS,
            login: &["login"],
            xdg: None,
        }
    }

    /// The user its folder is signed in to (`config.json`), shown `@login`; the token stays in the
    /// keychain.
    fn account_status<'a>(
        &'a self,
        account: &'a crate::model::Account,
        settings: &'a crate::model::Settings,
    ) -> BoxFuture<'a, crate::accounts::AccountStatus> {
        let dir = crate::accounts::config_dir(account, settings);
        let login = crate::accounts::copilot_sign_in(&dir);
        Box::pin(async move {
            crate::accounts::AccountStatus {
                connected: login.is_some(),
                stamp: login.clone(),
                email: login.map(|l| format!("@{l}")),
                dir: dir.to_string_lossy().into_owned(),
            }
        })
    }

    fn workflow_folders(&self) -> crate::workflow::Folders {
        use crate::workflow::DocKind;
        &[
            (".github/prompts", DocKind::Command),
            (".github/skills", DocKind::Skill),
            (".github/agents", DocKind::Agent),
        ]
    }

    fn id(&self) -> EngineId {
        EngineId::COPILOT
    }

    fn session_home(&self) -> Option<crate::fence::HomeKind> {
        Some(crate::fence::HomeKind::Copilot)
    }

    fn principal_sessions(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "Copilot CLI"
    }

    fn speaker(&self) -> &'static str {
        "Copilot"
    }

    /// `systemMessage` of the session: no command line to fit in.
    fn instructions_room(&self) -> Option<Room> {
        None
    }

    /// A subagent of Copilot (its `task` tool) takes no message once it runs.
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

    fn update_command(&self) -> &'static str {
        "npm install -g @github/copilot@latest"
    }

    fn version_args(&self) -> &'static [&'static str] {
        &["--version"]
    }

    fn caps(&self) -> EngineCaps {
        caps()
    }

    fn accounts(&self) -> super::registry::AccountStore {
        super::registry::AccountStore::ConfigDir {
            var: "COPILOT_HOME",
        }
    }

    fn terminal(&self) -> Option<&'static dyn super::TerminalView> {
        Some(&terminal::COPILOT_TERMINAL)
    }

    fn install(&self) -> Install {
        INSTALL
    }

    /// `models.list` of a server of its own (`terminal::models_list`, ESC-186), as `catalog` makes
    /// the menu of it: a Free account's lists none, and keeps the declared models.
    fn list_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<Option<Vec<ToldModel>>>> {
        Box::pin(async move {
            let listed = terminal::models_list(&program, &dir.to_string_lossy(), &env).await?;
            Ok(catalog(Some(&listed), None))
        })
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

    /// `account.getQuota` of a server of the account's own, started then stopped (no session, no
    /// model asked): not signed in as the account, said so.
    fn read_quota<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        _dir: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<Option<crate::usage::Reading>>> {
        Box::pin(async move {
            let cwd = std::env::temp_dir().to_string_lossy().into_owned();
            let refused = escouade_launch(&Escouade::Denied { file: None }).args;
            let Some(snapshots) = terminal::read_quota(&program, &cwd, &env, &refused).await?
            else {
                return Ok(Some(crate::usage::Reading::NotSignedIn));
            };
            let now = chrono::Utc::now().timestamp_millis();
            Ok(credits(&snapshots, now).map(|m| crate::usage::Reading::Windows(vec![m])))
        })
    }

    fn supports_effort(&self, model: &str) -> bool {
        takes_effort(model)
    }

    fn supports_auto(&self, _model: &str) -> bool {
        true
    }

    /// `tokenLimit` of `session.usage_info` on its real run (`gpt-6-luna`).
    fn context_window(&self, _model: &str) -> u64 {
        272_000
    }

    fn spawn(
        &self,
        spawn: Spawn,
        on_frame: super::OnFrame,
        on_exit: super::OnExit,
    ) -> Result<Process> {
        // Its conversation goes through its plugin (INT-10.3): the launch it describes, checked and started by the core,
        // what Copilot writes translated there.
        crate::plugins::harness::spawn_plugged(
            crate::plugins::builtin::COPILOT,
            self.name(),
            spawn,
            on_frame,
            on_exit,
        )
    }

    /// As its client gives it to `session.send` (`content` of `agent/sendMessage`).
    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        message(text, attachments).map(|m| Message(json!({ "prompt": m })))
    }

    /// The items its plugin made of what Copilot wrote (`agent/items`).
    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        match frame["items"].as_array() {
            Some(items) => rt.apply_items(items, fx),
            // A recorded stream, replayed through the same translation.
            #[cfg(test)]
            None => rt.copilot_frame(frame, fx),
            #[cfg(not(test))]
            None => {}
        }
    }

    /// A session that cannot be resumed is told by `session.resume`: a new one is opened.
    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    /// `copilot -p`, the question read from its input, without tools for a short text (`prompt`).
    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(prompt::one_shot(ask))
    }

    fn answers_without_tools(&self) -> bool {
        true
    }

    fn writer(&self, _weight: Weight) -> &'static str {
        prompt::WRITER
    }

    /// Its entry, the token named (`${ESCOUADE_MCP_TOKEN}`), never written.
    fn grant_file(&self, url: &str, _token: &str) -> Option<Vec<u8>> {
        Some(grant_file(url))
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

    /// `session.history.compact`, which takes what to keep (`customInstructions`).
    fn compaction(&self) -> super::Compaction {
        super::Compaction::WithInstructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded(file: &str) -> Vec<Value> {
        bench::recorded(file)
    }

    #[test]
    fn each_mode_of_escouade_has_its_permissions_and_agent_mode_in_copilot() {
        assert_eq!(permission_mode("bypassPermissions"), "allow-all");
        for mode in ["default", "plan", "acceptEdits", "auto"] {
            assert_eq!(permission_mode(mode), "manual", "{mode}");
        }
        assert_eq!(agent_mode("plan"), "plan");
        assert_eq!(agent_mode("auto"), "autopilot");
        assert_eq!(agent_mode("acceptEdits"), "interactive");
        assert_eq!(agent_mode("default"), "interactive");
        for mode in MODES {
            let (fr, en) = (meaning(Lang::Fr, mode), meaning(Lang::En, mode));
            assert!(!fr.is_empty() && !en.is_empty() && fr != en, "{mode}");
            assert!(
                fr.contains("bac à sable") && en.contains("sandbox"),
                "{mode}"
            );
        }
        assert_eq!(caps().modes.len(), 5);
    }

    #[test]
    fn the_server_has_no_builtin_mcp_nor_temporary_folder_and_hides_the_tokens() {
        let a = args();
        for flag in [
            "--server",
            "--stdio",
            "--no-auto-update",
            "--disable-builtin-mcps",
            "--disallow-temp-dir",
        ] {
            assert!(a.iter().any(|x| x == flag), "{flag}");
        }
        assert!(a
            .iter()
            .any(|x| x.starts_with("--secret-env-vars=") && x.contains("GH_TOKEN")));
        assert!(!a.iter().any(|x| x == "--experimental"), "no assisted mode");
    }

    #[test]
    fn a_free_account_lists_no_model_and_the_menu_keeps_its_own_with_their_price() {
        let list = recorded("first-turn.jsonl")
            .into_iter()
            .find(|m| m["result"]["modelPriceCategories"].is_array())
            .expect("the answer to session.model.list");
        let models = catalog(Some(&json!({ "models": [] })), Some(&list["result"])).unwrap();
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["gpt-6-luna", "gpt-5-mini"]);
        assert_eq!(models[0].price.as_deref(), Some("low"));
        assert!(models[0].default && !models[1].default);
        assert!(models.iter().all(|m| m.id != "auto"));
    }

    #[test]
    fn a_listed_model_says_its_multiplier_efforts_and_images() {
        let listed = json!({ "models": [
            { "id": "claude-sonnet-4.5", "name": "Claude Sonnet 4.5",
              "capabilities": { "supports": { "vision": true } },
              "billing": { "multiplier": 3 },
              "supportedReasoningEfforts": ["low", "medium", "high"], "defaultReasoningEffort": "medium" },
            { "id": "gpt-5-mini", "name": "GPT-5 mini", "billing": { "multiplier": 0.33 } },
            { "id": "o9", "name": "O9", "policy": { "state": "disabled" } },
        ] });
        let models = catalog(Some(&listed), None).unwrap();
        assert_eq!(models.len(), 2, "the disabled one left out");
        let sonnet = &models[0];
        assert_eq!(sonnet.multiplier.as_deref(), Some("3"));
        assert_eq!(sonnet.efforts, ["low", "medium", "high"]);
        assert_eq!(sonnet.default_effort.as_deref(), Some("medium"));
        assert!(sonnet.images && sonnet.default);
        assert_eq!(models[1].multiplier.as_deref(), Some("0.33"));
    }

    #[test]
    fn a_permission_that_names_a_path_of_the_fence_is_refused_by_escouade() {
        let fence = vec![r"C:\Code\Secret".to_string()];
        // Read, written, in a command, among the paths a command may touch.
        for r in [
            json!({ "kind": "read", "path": "C:/code/secret/a.txt" }),
            json!({ "kind": "write", "fileName": r"C:\Code\Secret\b.rs" }),
            json!({ "kind": "shell", "fullCommandText": r"Get-Content C:\Code\Secret\a.txt" }),
            json!({ "kind": "shell", "fullCommandText": "x", "possiblePaths": [r"C:\Code\Secret"] }),
        ] {
            assert_eq!(
                fenced(&r, &fence),
                cfg!(windows) || r["kind"] != "read",
                "{r}"
            );
        }
        assert!(!fenced(
            &json!({ "kind": "read", "path": r"C:\Code\Other\a.txt" }),
            &fence
        ));
        assert!(!fenced(
            &json!({ "kind": "url", "url": "https://x" }),
            &fence
        ));
        // No fence, nothing refused.
        assert!(!fenced(
            &json!({ "kind": "read", "path": r"C:\Code\Secret\a.txt" }),
            &[]
        ));
        assert_ne!(fence_refusal(Lang::Fr), fence_refusal(Lang::En));
    }

    #[test]
    fn for_this_session_remembers_each_kind_and_never_writes_a_rule_of_copilots() {
        let of = |r: Value| session_approval(&json!({ "permissionRequest": r }));
        let shell = of(json!({ "kind": "shell", "commands": [{ "identifier": "npm test" }] }));
        assert_eq!(shell["kind"], "approve-for-session");
        assert_eq!(shell["approval"]["commandIdentifiers"], json!(["npm test"]));
        assert_eq!(of(json!({ "kind": "write" }))["approval"]["kind"], "write");
        assert_eq!(of(json!({ "kind": "read" }))["approval"]["kind"], "read");
        let mcp = of(json!({ "kind": "mcp", "serverName": "essai", "toolName": "ping" }));
        assert_eq!(mcp["approval"]["serverName"], "essai");
        assert_eq!(
            of(json!({ "kind": "url", "url": "https://example.com/a" }))["domain"],
            "example.com"
        );
        for kind in ["shell", "write", "read", "mcp", "url", "memory"] {
            let a = of(json!({ "kind": kind, "url": "https://x.y" }));
            let s = a.to_string();
            assert!(
                !s.contains("approve-permanently") && !s.contains("approve-for-location"),
                "{s}"
            );
        }
    }

    #[test]
    fn a_forms_answers_are_typed_as_its_schema_says() {
        let schema = json!({ "type": "object", "properties": {
            "prenom": { "type": "string", "title": "Prénom" },
            "age": { "type": "number", "title": "Âge" },
            "n": { "type": "integer" },
            "ok": { "type": "boolean" },
            "vide": { "type": "string" },
        } });
        let input = json!({
            "questions": [
                { "id": "prenom", "question": "Prénom" }, { "id": "age", "question": "Âge" },
                { "id": "n", "question": "n" }, { "id": "ok", "question": "ok" },
                { "id": "vide", "question": "vide" },
            ],
            "answers": { "Prénom": "Camille", "Âge": "30,5", "n": "4", "ok": "Oui", "vide": "" },
        });
        assert_eq!(
            form_content(&schema, &input),
            json!({ "prenom": "Camille", "age": 30.5, "n": 4, "ok": true })
        );
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
        assert_eq!(m["prompt"], "notes.txt:\n\nbonjour\n\nRegarde");
        assert_eq!(m["attachments"][0]["type"], "blob");
        assert_eq!(m["attachments"][0]["mimeType"], "image/png");
        assert!(message("x", &[file("a.pdf", "application/pdf", "JVBERi0=")]).is_err());
        assert_eq!(
            message("bonjour", &[]).unwrap(),
            json!({ "prompt": "bonjour" })
        );
    }

    #[test]
    fn the_months_credits_come_from_chat_and_a_reading_without_its_end_ends_next_month() {
        let at = |d: &str| {
            chrono::DateTime::parse_from_rfc3339(d)
                .unwrap()
                .timestamp_millis()
        };
        let now = at("2026-10-18T10:00:00Z");
        // A call's snapshot (`assistant.usage`): its own end.
        let call = json!({ "chat": { "entitlementRequests": 200, "usedRequests": 1,
            "remainingPercentage": 99.3, "resetDate": "2026-11-01T00:00:00Z" } });
        let m = credits(&call, now).unwrap();
        assert_eq!((m.minutes, m.pct), (None, 0.7));
        assert_eq!(m.resets_at, Some(at("2026-11-01T00:00:00Z")));
        // `account.getQuota` says the time of its reading: the first of next month instead.
        let read = json!({ "chat": { "entitlementRequests": 200, "remainingPercentage": 80,
            "resetDate": "2026-10-18T09:59:59Z" } });
        let m = credits(&read, now).unwrap();
        assert_eq!(m.pct, 20.0);
        assert_eq!(m.resets_at, Some(at("2026-11-01T00:00:00Z")));
        // `session.quota.get`: in epoch ms; December goes to January.
        let dec = at("2026-12-30T10:00:00Z");
        let m = credits(
            &json!({ "chat": { "entitlementRequests": 200, "remainingPercentage": 0 } }),
            dec,
        )
        .unwrap();
        assert_eq!(m.pct, 100.0);
        assert_eq!(m.resets_at, Some(at("2027-01-01T00:00:00Z")));
        // A plan without a limit, or nothing told: no meter. Premium requests when chat has none.
        let unlimited = json!({ "chat": { "isUnlimitedEntitlement": true,
            "entitlementRequests": -1, "remainingPercentage": 100 } });
        assert_eq!(credits(&unlimited, now), None);
        assert_eq!(credits(&json!({}), now), None);
        let premium = json!({
            "chat": { "entitlementRequests": 0, "remainingPercentage": 0 },
            "premium_interactions": { "entitlementRequests": 300, "remainingPercentage": 75 },
        });
        assert_eq!(credits(&premium, now).unwrap().pct, 25.0);
        // As the real run read them out of a session (ESC-23).
        let got = recorded("first-turn.jsonl")
            .into_iter()
            .filter_map(|m| m["result"]["snapshots"]["chat"]["resetDateEpochMs"].as_i64())
            .next()
            .unwrap();
        assert_eq!(got, at("2026-11-01T00:00:00Z"));
    }

    #[test]
    fn a_calls_cost_is_its_nano_aiu_else_its_details_else_its_models_prices() {
        // Every call of the real runs: their details make their total, to the nano-AIU.
        let mut checked = 0;
        for file in [
            "first-turn.jsonl",
            "assisted-autopilot.jsonl",
            "full-turn.jsonl",
        ] {
            for m in recorded(file) {
                let d = &m["params"]["event"]["data"];
                if m["params"]["event"]["type"] != "assistant.usage" {
                    continue;
                }
                let Some(total) = d["copilotUsage"]["totalNanoAiu"].as_u64() else {
                    continue;
                };
                assert_eq!(call_nano(d, &HashMap::new()), total);
                let mut untold = d.clone();
                untold["copilotUsage"]
                    .as_object_mut()
                    .unwrap()
                    .remove("totalNanoAiu");
                let by_details = call_nano(&untold, &HashMap::new()) as i64;
                assert!((by_details - total as i64).abs() <= 1, "{file}");
                checked += 1;
            }
        }
        assert!(checked >= 3, "{checked}");
        // Neither: the prices of its model, from `models.list`.
        let listed = json!({ "models": [
            { "id": "gpt-6-luna", "billing": { "multiplier": 1, "tokenPrices": {
                "inputPrice": 10, "cacheReadPrice": 1, "cacheWritePrice": 12.5,
                "outputPrice": 50, "batchSize": 1_000_000 } } },
            { "id": "gpt-5-mini", "billing": { "multiplier": 0.33 } },
        ] });
        let prices = prices(&listed);
        assert_eq!(prices.len(), 1);
        let usage = json!({ "model": "gpt-6-luna", "inputTokens": 12 + 40_145 + 13_657,
            "cacheReadTokens": 40_145, "cacheWriteTokens": 13_657, "outputTokens": 325 });
        assert_eq!(call_nano(&usage, &prices), 227_227_500);
        // An unknown model: nothing to count.
        assert_eq!(
            call_nano(&json!({ "model": "x", "inputTokens": 5 }), &prices),
            0
        );
    }

    #[test]
    fn escouades_server_is_given_by_a_file_that_names_the_token_and_refused_by_its_name() {
        let url = "http://127.0.0.1:4321/mcp";
        let file: Value =
            serde_json::from_slice(&COPILOT.grant_file(url, "s3cr3t").unwrap()).unwrap();
        assert_eq!(
            file,
            json!({ "mcpServers": { "escouade": {
                "type": "http", "url": url,
                "headers": { "Authorization": "Bearer ${ESCOUADE_MCP_TOKEN}" },
                "tools": ["*"],
            } } })
        );
        let granted = COPILOT.escouade_launch(&Escouade::Granted {
            url: url.into(),
            token: "s3cr3t".into(),
            file: Some("C:/data/mcp/a1.json".into()),
        });
        assert_eq!(
            granted.args,
            ["--additional-mcp-config", "@C:/data/mcp/a1.json"]
        );
        assert_eq!(
            granted.env,
            [("ESCOUADE_MCP_TOKEN".to_string(), "s3cr3t".to_string())]
        );
        // Never among the variables kept from its commands: Copilot would not put it in the header.
        assert!(!args().iter().any(|a| a.contains("ESCOUADE_MCP_TOKEN")));
        let refused = COPILOT.escouade_launch(&Escouade::Denied { file: None });
        assert_eq!(refused.args, ["--disable-mcp-server", "escouade"]);
        assert!(refused.env.is_empty());
    }
}
