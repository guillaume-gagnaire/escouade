//! OpenCode, the third engine: OpenCode 1 (`opencode-ai`, 1.19) driven through `opencode serve`
//! (HTTP + SSE, docs/PROTOCOL.md, « OpenCode serve »), one server for the agents of an account,
//! each agent a session of its own. Since INT-10.2 the conversation of its agents goes through the
//! plugin `dev.escouade.opencode` (`plugins/opencode/`): the client of the
//! server and the translation of its events are there, the server run by the core
//! (`plugins::shared`). What stays here is the rest of its engine: its capabilities and the meaning
//! of its modes, its accounts, its models read without a server, the texts it writes (`run`).
//!
//! What it does as Escouade drives it comes from the real runs of OpenCode 1.19.3 with local
//! models (ESC-25, docs/TODO-1.8.md, point 0, « OpenCode »), whose streams are in
//! `tests/fixtures/opencode/` and whose schema is `tests/fixtures/engines/opencode-1.19.3.openapi.json`.
//! OpenCode 2 (`opencode v2.…`) is another API, told unsupported (`unsupported_line`).

#[cfg(test)]
mod bench;
mod locate;
mod run;

#[cfg(test)]
pub(crate) use bench::{fake_opencode, Through};

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
use escouade_plugin_opencode::{listed, split_model};
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

/// The models OpenCode lists, as the core keeps them.
fn told(models: Vec<escouade_plugin_opencode::rules::Told>) -> Vec<ToldModel> {
    models
        .into_iter()
        .map(|m| ToldModel {
            id: m.id,
            label: m.label,
            efforts: m.efforts,
            default_effort: m.default_effort,
            images: m.images,
            default: m.default,
            provider: m.provider,
            multiplier: None,
            price: None,
            from_config: false,
        })
        .collect()
}

/// How long `opencode models` may take (3 s on 1.19.3, tried).
const LIST_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

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
        mid_turn: MidTurn::TurnEnd,
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
            let models = told(listed(&String::from_utf8_lossy(&out.stdout)).models);
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
            bail!(escouade_plugin_opencode::no_model(i18n::ui().tag()));
        }
        crate::plugins::harness::spawn_plugged(
            crate::plugins::builtin::OPENCODE,
            "OpenCode",
            spawn,
            on_frame,
            on_exit,
        )
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        parts(text, attachments).map(Message)
    }

    /// What its plugin made of its events.
    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        if let Some(items) = frame["items"].as_array() {
            rt.apply_items(items, fx);
        }
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
    /// with its own token, by the API (its plugin).
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
    use escouade_plugin_opencode::catalog;

    fn recorded(file: &str) -> Vec<Value> {
        bench::recorded(file)
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
    fn each_mode_of_opencode_says_what_it_means() {
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
}
