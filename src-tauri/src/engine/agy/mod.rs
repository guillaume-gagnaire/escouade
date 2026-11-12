//! Antigravity CLI, the seventh harness: Google's agent in a terminal (`agy`). Since INT-10.8 its agents converse through its
//! plugin in process (`dev.escouade.antigravity-cli`: the client of `agy -p` in stream-json and the translation of its
//! events, `plugins::harness::BUILTIN_HARNESSES`); what stays here is what the contract leaves in the core: its place in the
//! registry, its accounts, the models of an account (`agy models`), the texts it writes (`agy -p`), its terminal view.
//!
//! What it does as Escouade drives it comes from the real runs of `agy` 1.4.9 (ESC-269,
//! docs/TODO-1.8.md, point 0, « Antigravity CLI »), whose streams are in `tests/fixtures/agy/`.
//!
//! Each agent has a home of its own (`ESCOUADE_AGY_HOME`, given to `agy` as its `USERPROFILE` and
//! `HOME`: `agy` keeps all it writes in `<home>/.gemini`, which it shares with Gemini CLI on the
//! machine). There its plugin has the core write its `antigravity-cli/settings.json` (the permission rules of its
//! mode, of its fence and of the user's « Toujours ») and `config/mcp_config.json` (Escouade's
//! server, its token in the file, never on a command line: `agy` expands no variable there). The
//! user's `~/.gemini` is never read nor written. Its account stays the machine's (the system's
//! keychain, which a home of its own does not move).

#[cfg(test)]
pub(crate) mod bench;
mod locate;
pub(crate) mod terminal;

use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
    ToldModel,
};
use super::{
    BoxFuture, Engine, Message, OneShot, Process, Program, Room, Spawn, StartLost, Weight,
};
use crate::agent::{AgentRt, Effects};
use crate::core::Attachment;
use crate::i18n::{self, Lang};
use crate::model::EngineId;
use anyhow::{bail, Result};
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

/// The models of its menu until `agy models` has listed them.
const MODELS: [(&str, &str); 1] = [(DEFAULT, "Gemini 3.8 Flash")];

/// Where its plans and their quotas are told: its usage itself is in its interface alone (`/usage`).
pub(crate) const USAGE_PAGE: &str = "https://antigravity.google/docs/plans";

#[cfg(test)]
pub(crate) use escouade_acp::limits::QUOTA_RESUME_MS;

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
            "Ses écritures d’un fichier à extension dans le dossier de l’agent (hors .git, .husky, .githooks, .agents et .gemini) sont acceptées par Escouade à la fin du tour, puis il les refait ; ses commandes, git add et git commit compris, ses autres écritures et les outils MCP demandent. Ce qu’Escouade accepte vaut pour le tour suivant. Pas de bac à sable.",
            "Its writes of a file with an extension in the agent’s folder (but .git, .husky, .githooks, .agents and .gemini) are accepted by Escouade at the end of the turn, then it does them again; its commands, git add and git commit included, its other writes and MCP tools ask. What Escouade accepts holds for the next turn. No sandbox."
        ),
        "bypassPermissions" => tr_in!(
            lang,
            "Escouade lui permet tout par ses règles : commandes, fichiers, outils MCP et adresses, sans rien demander. Pour un projet privé, chaque appel est jugé par Escouade à la fin du tour : ce qui nomme ses chemins est refusé, et ses commandes demandent. Pas de bac à sable.",
            "Escouade allows it everything by its rules: commands, files, MCP tools and addresses, without asking. For a private project, each call is judged by Escouade at the end of the turn: what names its paths is refused, and its commands ask. No sandbox."
        ),
        _ => tr_in!(
            lang,
            "Ses écritures d’un fichier à extension dans le dossier de l’agent (hors .git, .husky, .githooks, .agents et .gemini) sont acceptées par Escouade ; ses commandes, ses autres écritures et les outils MCP demandent. Tout cela une fois son tour fini (Antigravity ne demande rien en cours de tour), puis il les refait ; ce qu’Escouade accepte vaut pour le tour suivant. Il lit sans demander. Pas de bac à sable.",
            "Its writes of a file with an extension in the agent’s folder (but .git, .husky, .githooks, .agents and .gemini) are accepted by Escouade; its commands, its other writes and MCP tools ask. All of it once its turn is over (Antigravity asks nothing during a turn), then it does them again; what Escouade accepts holds for the next turn. It reads without asking. No sandbox."
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

/// Escouade's server taken out of the agent's MCP config once its process is over (its plugin's launch wrote it there,
/// the token in the file: `agy` expands no variable): the token goes with its process, whatever the plugin does after.
/// The rest of the file is kept.
pub(crate) fn remove_grant(home: &Path) {
    let file = mcp_file(home);
    let Ok(text) = std::fs::read_to_string(&file) else {
        return;
    };
    let Ok(Value::Object(mut doc)) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    doc.insert("mcpServers".into(), json!({}));
    let written = serde_json::to_string_pretty(&Value::Object(doc))
        .map_err(anyhow::Error::from)
        .and_then(|t| std::fs::write(&file, t).map_err(anyhow::Error::from));
    if let Err(e) = written {
        log::warn!("agy: Escouade's server not taken out of its MCP config: {e:#}");
    }
}

// What the core keeps of Antigravity CLI for itself: the variables of its accounts, the menu of its models before an
// account lists them, the files it reads (the transcript of its view) and takes away (Escouade's server after a process,
// what the 2.2.0 kept of « Toujours ») in the agent's home, the home of the user's tools. Its words are its plugin's.

/// The variable the core gives each agent's own home in: its plugin gives it to `agy` as `USERPROFILE` and `HOME`.
pub(crate) const HOME_VAR: &str = "ESCOUADE_AGY_HOME";

/// The keys of the machine that would take the place of its Google account: the first, an account added by its key goes
/// by (`modelProvider: gemini`, ESC-271).
pub(crate) const KEY_VARS: [&str; 2] = ["GEMINI_API_KEY", "GOOGLE_API_KEY"];

/// What keeps `agy` from updating itself (a string of its program, ESC-269): stopped with its tree, an update it began would
/// leave no `agy` behind (ESC-133). In every launch, by `home_env`.
pub(crate) const NO_SELF_UPDATE: (&str, &str) = ("AGY_CLI_DISABLE_AUTO_UPDATE", "1");

/// Its default model: Gemini Flash, its effort apart.
pub(crate) const DEFAULT: &str = "gemini-3.8-flash";

/// The efforts of a Gemini model: part of its id in `agy models` (`gemini-3.8-flash-low`), its `--effort` beside its id
/// without them (`xhigh` and `max` refused by Flash, tried).
pub(crate) const EFFORTS: [&str; 3] = ["low", "medium", "high"];

/// The efforts in which `id` (`gemini-3.8-flash-low`) ends, and its id without it.
pub(crate) fn effort_of(id: &str) -> Option<(&str, &'static str)> {
    EFFORTS
        .iter()
        .find_map(|e| id.strip_suffix(&format!("-{e}")).map(|base| (base, *e)))
}

/// Its folder of config in the agent's home.
pub(crate) fn gemini_dir(home: &Path) -> PathBuf {
    home.join(".gemini")
}

/// Its MCP config in the agent's home (`agy mcp add` writes the same file).
pub(crate) fn mcp_file(home: &Path) -> PathBuf {
    gemini_dir(home).join("config").join("mcp_config.json")
}

/// What the 2.2.0 kept of « Toujours », beside its home's `.gemini`: no longer read, and erased (ESC-376): what the agent
/// could have planted there would hold.
pub(crate) fn always_file(home: &Path) -> PathBuf {
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

/// Where the home of the user's tools stays (its git config, Rust's), which the commands of `agy` would no longer find
/// in the agent's home; what `env` and the machine say already left out. A module knows neither that home nor the machine's
/// variables: the core adds them to what its plugin's launch says (`Agy::spawn`).
#[cfg(not(target_family = "wasm"))]
pub(crate) fn tools_env(env: &[(String, String)]) -> Vec<(String, String)> {
    let Some(real) = dirs::home_dir() else {
        return Vec::new();
    };
    [
        ("GIT_CONFIG_GLOBAL", ".gitconfig"),
        ("CARGO_HOME", ".cargo"),
        ("RUSTUP_HOME", ".rustup"),
    ]
    .into_iter()
    .filter(|(var, _)| !env.iter().any(|(e, _)| e == var) && std::env::var_os(var).is_none())
    .map(|(var, name)| (var, real.join(name)))
    .filter(|(_, path)| path.exists())
    .map(|(var, path)| (var.to_string(), path.to_string_lossy().into_owned()))
    .collect()
}

/// See the native `tools_env`.
#[cfg(target_family = "wasm")]
pub(crate) fn tools_env(_env: &[(String, String)]) -> Vec<(String, String)> {
    Vec::new()
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

/// For what does not go by `home_env`: its version asked.
const NO_UPDATE: [(&str, &str); 1] = [NO_SELF_UPDATE];

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
            "Le compte Antigravity « {name} » n’a pas de clé d’API Gemini : donne-la dans Réglages › Comptes et harness, « Changer la clé… ».",
            "The Antigravity account “{name}” has no Gemini API key: give it in Settings › Accounts and harnesses, “Change the key…”."
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
        &NO_UPDATE
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

    /// A Gemini model named without its effort (`gemini-3.8-flash`): `--effort` beside it.
    /// Its plugin reads them (`account/read` of `models`), by a process of the account the core starts.
    fn list_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        _dir: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<Option<Vec<ToldModel>>>> {
        // In a home of its own for the read, which the core makes and takes away (the account's settings untouched).
        Box::pin(async move {
            let home = throwaway_home()?;
            let read = async {
                crate::plugins::harness::plugged_for(self.id(), self.name(), false)?
                    .account_models(&program, &env, &home)
                    .await
            }
            .await;
            let _ = std::fs::remove_dir_all(&home);
            read
        })
    }

    fn supports_effort(&self, model: &str) -> bool {
        model.starts_with("gemini-") && effort_of(model).is_none()
    }

    fn supports_auto(&self, _model: &str) -> bool {
        false
    }

    /// Its conversation goes through its plugin (INT-10.8): the launch it describes, checked and started by the core, what
    /// `agy` writes translated there. Each agent has its home, which the token of Escouade's server leaves with its process.
    fn spawn(
        &self,
        mut spawn: Spawn,
        on_frame: super::OnFrame,
        on_exit: super::OnExit,
    ) -> Result<Process> {
        // The user's home is the core's to know: its plugin is a module.
        let tools = tools_env(&spawn.env);
        spawn.env.extend(tools);
        let Some(home) = spawn
            .env
            .iter()
            .find(|(k, _)| k == HOME_VAR)
            .map(|(_, v)| PathBuf::from(v))
        else {
            bail!(tr!(
                "Antigravity CLI n’a pas de dossier à lui pour cet agent : l’agent ne démarre pas.",
                "Antigravity CLI has no folder of its own for this agent: the agent doesn’t start."
            ));
        };
        let on_exit: super::OnExit = Box::new(move |code, said| {
            remove_grant(&home);
            on_exit(code, said);
        });
        crate::plugins::harness::spawn_plugged(
            crate::plugins::builtin::ANTIGRAVITY,
            self.name(),
            spawn,
            on_frame,
            on_exit,
        )
    }

    /// Its text alone: `agy`'s stream-json takes nothing else.
    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        message(text, attachments).map(|m| Message(m["text"].clone()))
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        match frame["items"].as_array() {
            Some(items) => rt.apply_items(items, fx),
            // A recorded stream, replayed through the same translation.
            #[cfg(test)]
            None => rt.agy_frame(frame, fx),
            #[cfg(not(test))]
            None => {}
        }
    }

    /// A conversation that cannot be resumed gives a new one (`process`), said by the conversation.
    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        // By its plugin (`engine/oneShot`), in a home of its own the core makes and takes away.
        Box::pin(async move {
            let home = throwaway_home()?;
            let answer = async {
                crate::plugins::harness::plugged_for(self.id(), self.name(), false)?
                    .one_shot_in(ask, Some(&home))
                    .await
            }
            .await;
            let _ = std::fs::remove_dir_all(&home);
            answer
        })
    }

    fn answers_without_tools(&self) -> bool {
        true
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
}

/// What the core does in the agent's home before its launch: the hooks of its interface (« Vue Antigravity ») would run here
/// too, for nothing; what the 2.2.0 kept of « Toujours » is erased (ESC-376), as its plugin no longer reads it.
pub(crate) fn prepare(p: &crate::plugins::harness::Preparing) -> Result<Vec<String>> {
    if let Some(home) = p.home {
        terminal::forget_hooks(home);
        let _ = std::fs::remove_file(always_file(home));
    }
    Ok(Vec::new())
}

#[cfg(test)]
mod tests;
