//! Codex, the second engine: a `codex app-server` process per agent. Since INT-10.4 its agents converse through its
//! plugin in process (`dev.escouade.codex`: the client of its app-server and the translation of its messages,
//! `plugins::harness::BUILTIN_HARNESSES`); what stays here is what the contract leaves in the core: its place in the
//! registry, its accounts (`CODEX_HOME`), an account's quota and models (`process`), the texts it writes, its terminal
//! view and the trust of its hooks.
//!
//! What it does as Escouade drives it comes from the real runs of Codex 0.163.0 (ESC-22,
//! docs/TODO-1.8.md, point 0, « Codex »), whose streams are in `tests/fixtures/codex/` and whose
//! schema is `tests/fixtures/engines/codex-0.163.0.schema.json`. Its texts written by a model go
//! through `codex exec` (`exec`).

#[cfg(test)]
mod bench;
mod exec;
mod locate;
pub(crate) mod prices;
mod process;
#[cfg(test)]
mod schema_tests;
pub(crate) mod terminal;
mod words;

#[cfg(test)]
pub(crate) use bench::fake_codex;
#[cfg(test)]
pub(crate) use words::{
    answers, is_plan_approval, mode_of, mode_params, Settings, FENCE_PLAN_PROFILE, FENCE_PROFILE,
};
use words::{declares_escouade, TOKEN_VAR};
pub(crate) use words::{fence_args, fence_profile, sandbox_mode, sandboxed};

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
use crate::model::{EngineId, QuotaMeter};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::Path;

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
    serde_json::from_value(words::catalog(list)?).ok()
}

/// The windows of a quota of Codex (`rateLimits` of `account/rateLimits/*`), as meters.
pub(crate) fn meters(limits: &Value) -> Vec<QuotaMeter> {
    process::meters_of(limits)
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

/// Escouade's server for its interface: its address and the variable of its token, by `-c` alone; refused, an entry of
/// that name turned off, the user's own included. The token is in the process's environment.
fn escouade_launch(escouade: &Escouade) -> mcp::Launch {
    match escouade {
        Escouade::Granted { url, token, .. } => mcp::Launch {
            args: words::granted_args(url),
            env: vec![(TOKEN_VAR.into(), token.clone())],
        },
        Escouade::Denied { .. } => mcp::Launch {
            args: words::denied_args(),
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
        // Its conversation goes through its plugin (INT-10.4): the launch it describes, checked and started by the core,
        // what Codex writes translated there.
        crate::plugins::harness::spawn_plugged(
            crate::plugins::builtin::CODEX,
            self.name(),
            spawn,
            on_frame,
            on_exit,
        )
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        // As its harness takes it, passed by the plugin's client as it is (`content` of `agent/sendMessage`).
        user_input(text, attachments).map(|prompt| Message(json!({ "prompt": prompt })))
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        match frame["items"].as_array() {
            Some(items) => rt.apply_items(items, fx),
            // A recorded stream, replayed through the same translation.
            #[cfg(test)]
            None => rt.codex_frame(frame, fx),
            #[cfg(not(test))]
            None => {}
        }
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

    fn answers_without_tools(&self) -> bool {
        true
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
