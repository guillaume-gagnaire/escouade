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
use escouade_engines::locate::copilot as locate;
mod process;
mod prompt;
#[cfg(test)]
mod schema_tests;
pub(crate) mod terminal;

#[cfg(test)]
pub(crate) use bench::fake_copilot;

use super::registry::{
    CostKind, EngineCaps, ForkPoint, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested, ToldModel,
};
use super::userconfig::{Format, UserConfig};
use super::{
    mcp, BoxFuture, Engine, Message, OneShot, Process, Program, Room, Spawn, StartLost, Weight,
};
use crate::agent::{AgentRt, Effects};
use crate::core::{size_label, Attachment, MAX_IMAGE, MAX_TOTAL};
use crate::i18n::{self, Lang};
use crate::model::EngineId;
use anyhow::{bail, Result};
use serde_json::{json, Value};

/// GitHub Copilot CLI.
pub struct Copilot;

pub static COPILOT: Copilot = Copilot;

/// The versions of Copilot CLI Escouade was tested with: its server as the real run of 2026-10-18
/// found it (ESC-23).
pub const TESTED: Tested = Tested {
    oldest: "1.0.98",
    newest: "1.0.98",
};

/// The models of its menu until a process of the account has listed them.
/// The models of its menu until a process of the account has listed them: those its real run on a Free account ran
/// (`gpt-6-luna` chosen by `auto`, `gpt-5-mini` taken by `session.model.switchTo`). Never `auto`, which picks a model
/// itself.
const MODELS: [(&str, &str); 2] = [("gpt-6-luna", "GPT-6 Luna"), ("gpt-5-mini", "GPT-5 mini")];

/// The variables a token of GitHub reaches Copilot in, taken before its own sign-in: kept from the processes of an
/// account of its own (`COPILOT_HOME`), the core's to give.
pub(crate) const INHERITED_TOKENS: [&str; 3] = ["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"];

/// Whether `model` takes a reasoning effort, for the menu: never « auto », which picks the model itself.
pub(crate) fn takes_effort(model: &str) -> bool {
    !model.eq_ignore_ascii_case("auto")
}

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

/// The private file of a process given Escouade's server: its entry, read by
/// `--additional-mcp-config @<file>`, with its token, as Claude Code's (ESC-430). Its interface
/// is given the token in no variable its commands would inherit, and strips that variable from
/// them (`--secret-env-vars`), which keeps Copilot from expanding `${ESCOUADE_MCP_TOKEN}` too
/// (tried on 1.0.98). The file goes with the process (`release_agent`).
fn grant_file(url: &str, token: &str) -> Vec<u8> {
    let entry = json!({ "mcpServers": { mcp::SERVER: {
        "type": "http",
        "url": url,
        "headers": { "Authorization": format!("Bearer {token}") },
        "tools": ["*"],
    } } });
    serde_json::to_vec_pretty(&entry).unwrap_or_default()
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

    /// Its plugin reads them (`account/read` of `models`), by a process of the account the core starts.
    fn list_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<Option<Vec<ToldModel>>>> {
        Box::pin(async move {
            crate::plugins::harness::plugged_for(self.id(), self.name(), false)?
                .account_models(&program, &env, dir)
                .await
        })
    }

    /// Its plugin reads it (`account/read` of `quota`), by a process of the account the core starts.
    fn read_quota<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<Option<crate::usage::Reading>>> {
        Box::pin(async move {
            crate::plugins::harness::plugged_for(self.id(), self.name(), false)?
                .account_quota(&program, &env, dir)
                .await
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

    /// Its entry, with its token.
    fn grant_file(&self, url: &str, token: &str) -> Option<Vec<u8>> {
        Some(grant_file(url, token))
    }

    fn refusal(&self) -> Option<Value> {
        None
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
}
