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
mod words;

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
pub(crate) use words::*;

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
pub(crate) use escouade_acp::limits::limit_of;
#[cfg(test)]
pub(crate) use escouade_acp::limits::{CAPACITY_RESUME_MS, QUOTA_RESUME_MS};

/// What the card of a turn stopped by its limit says (`said`: `agy`'s own words).
#[cfg(test)]
pub(crate) fn limited_error(lang: Lang, said: &str) -> String {
    words::limited_error(lang.tag(), said)
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

/// The agent's `settings.json`: its `permissions` as Escouade sets them (its mode's rules, what the
/// user allowed, its refusals: `deny` before all, `agy`'s rule); the rest of the file kept.
#[cfg(test)]
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

/// The provider of its models in the settings of `home` (`set_provider`), the rest of the file kept.
pub(crate) fn write_provider(home: &Path, by_key: bool) -> Result<()> {
    let file = settings_file(home);
    let mut doc: Value = std::fs::read_to_string(&file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    set_provider(&mut doc, by_key);
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

/// Escouade's server taken out of the agent's MCP config once its process is over: its token goes
/// with it.
pub(crate) fn remove_grant(home: &Path) {
    if mcp_file(home).is_file() {
        if let Err(e) = write_grant(home, None) {
            log::warn!("agy: Escouade's server not taken out of its MCP config: {e:#}");
        }
    }
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

/// The model of a text of `weight`: every tool is refused whatever it is (`write_no_tools`).
pub(crate) fn text_model(weight: Weight) -> String {
    match weight {
        Weight::Light => format!("{DEFAULT}-low"),
        Weight::Reading | Weight::Given => format!("{DEFAULT}-medium"),
    }
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
    let model = text_model(ask.weight);
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
        Box::pin(one_shot(ask))
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

    fn escouade_launch(&self, _escouade: &Escouade) -> mcp::Launch {
        mcp::Launch {
            args: Vec::new(),
            env: Vec::new(),
        }
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
