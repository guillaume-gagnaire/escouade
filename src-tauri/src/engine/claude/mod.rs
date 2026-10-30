//! Claude Code, the first engine: a `claude` process driven over the stream-json protocol
//! (docs/PROTOCOL.md), its frames translated by `AgentRt::claude_frame` (`agent/claude.rs`).

#[cfg(test)]
mod bench;
mod locate;
mod mcp;
mod process;
pub(crate) mod terminal;

#[cfg(test)]
pub(crate) use mcp::all_tools;
pub(crate) use process::{ClaudeProcess, SpawnOpts};

use super::registry::{
    CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind, Tested,
    ToldModel,
};
use super::Program;
use super::{
    BoxFuture, Engine, EngineProcess, Escouade, Message, OneShot, PermissionAnswer, Process, Ready,
    Remote, Spawn, Start, StartLost, Weight,
};
use crate::agent::{AgentRt, Effects};
use crate::core::{size_label, Attachment, MAX_IMAGE, MAX_TOTAL, MB};
use crate::i18n::{self, Lang};
use crate::job::JobUsage;
use crate::mcp::install::{Entry, Standing, Target};
#[cfg(test)]
use crate::model::AgentMeta;
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::time::Duration;

/// Claude Code.
pub struct Claude;

pub static CLAUDE: Claude = Claude;

/// The versions of Claude Code Escouade was tested with: its stream-json protocol from 2.1.283
/// (docs/PROTOCOL.md), up to the one of the last real run (2.1.301, 2026-10-16).
pub const TESTED: Tested = Tested {
    oldest: "2.1.283",
    newest: "2.1.301",
};

/// Its installers (claude.ai), as the Welcome screen gave the one of Windows.
const INSTALL: Install = Install {
    windows: "irm https://claude.ai/install.ps1 | iex",
    unix: "curl -fsSL https://claude.ai/install.sh | bash",
};

/// What turns off Claude Code's own update (docs: « Disable auto-updates »): only its check in the
/// background, `claude update` still works. Claude Code also leaves its plugins as they are then
/// (`FORCE_AUTOUPDATE_PLUGINS` would bring their update back), nothing else.
pub const NO_AUTOUPDATER: (&str, &str) = ("DISABLE_AUTOUPDATER", "1");

/// The command that puts back an installation of Claude Code by npm.
pub const NPM_INSTALL: &str = "npm install -g @anthropic-ai/claude-code";

/// What is said when no `claude` is found (ESC-133), in `lang`: its installation by npm cut
/// half-way, what it left and the command that puts it back (`locate::interrupted_install`), else
/// not found, with its installer.
pub fn missing_line(lang: Lang) -> String {
    missing_line_with(lang, locate::interrupted_install().as_deref())
}

/// What is said of a `claude` that stopped before it did anything, when an installation of it by
/// npm was cut half-way (ESC-133): what it left, and the command that puts it back.
pub fn interrupted_line(lang: Lang) -> Option<String> {
    let left = locate::interrupted_install()?;
    Some(missing_line_with(lang, Some(&left)))
}

/// `missing_line`, `leftover` what an installation cut half-way left.
pub fn missing_line_with(lang: Lang, leftover: Option<&std::path::Path>) -> String {
    let npm = NPM_INSTALL;
    if let Some(left) = leftover {
        let left = left.display();
        return tr_in!(
            lang,
            "L’installation de Claude Code a été interrompue (il en reste {left}) : relance `{npm}` dans un terminal.",
            "Claude Code’s installation was interrupted ({left} is left over): run `{npm}` again in a terminal."
        );
    }
    let install = if cfg!(windows) {
        INSTALL.windows
    } else {
        INSTALL.unix
    };
    tr_in!(
        lang,
        "Claude Code est introuvable : réinstalle-le (`{install}`, ou `{npm}`), ou indique son chemin dans Réglages › Harness.",
        "Claude Code can’t be found: install it again (`{install}`, or `{npm}`), or set its path in Settings › Harness."
    )
}

/// The aliases the composer offers: Claude Code runs the latest model of the family it knows
/// (`MODELS` of `src/lib/models.ts`, which names them once it has told which version).
const MODELS: [(&str, &str); 4] = [
    ("fable", "Fable"),
    ("opus", "Opus"),
    ("sonnet", "Sonnet"),
    ("haiku", "Haiku"),
];

/// The efforts it takes (`--effort`).
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// What Claude Code does, as Escouade drives it today.
pub fn caps() -> EngineCaps {
    let mode = |id| ModeCap { id, meaning: None };
    EngineCaps {
        modes: [
            "auto",
            "default",
            "plan",
            "acceptEdits",
            "bypassPermissions",
        ]
        .into_iter()
        .map(mode)
        .collect(),
        effort: true,
        images: true,
        pdf: true,
        fork: ForkPoint::Entry,
        resume_other_account: true,
        mid_turn: MidTurn::Queued,
        slash_commands: true,
        quota: QuotaKind::Windows,
        quota_windows: vec![crate::model::FIVE_HOURS, crate::model::WEEK],
        cost: CostKind::Exact,
        terminal_view: true,
        remote_control: true,
        background_tasks: true,
        todo_tools: true,
        default_refusal: true,
        sandbox: false,
        usage_page: None,
        accounts_by_key: false,
        read_only: true,
    }
}

const KB: usize = 1024;
/// The only image formats the API reads.
const IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];
/// Past this, a text file alone would fill Claude's context (~4 bytes a token).
const MAX_TEXT: usize = 256 * KB;

/// Content of a user message: its text alone, or the attached files as content blocks
/// followed by the text.
pub fn user_content(text: &str, attachments: &[Attachment]) -> Result<Value> {
    if attachments.is_empty() {
        return Ok(Value::String(text.to_string()));
    }
    let (mut blocks, sizes): (Vec<Value>, Vec<usize>) = attachments
        .iter()
        .map(attachment_block)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .unzip();
    if sizes.iter().sum::<usize>() > MAX_TOTAL {
        let lang = i18n::ui();
        bail!(tr_in!(
            lang,
            "Les fichiers joints à un message sont limités à {max} en tout.",
            "The files attached to a message are limited to {max} in all.",
            max = size_label(lang, MAX_TOTAL)
        ));
    }
    if !text.is_empty() {
        blocks.push(json!({ "type": "text", "text": text }));
    }
    Ok(Value::Array(blocks))
}

/// The content block of a file, and its size.
fn attachment_block(a: &Attachment) -> Result<(Value, usize)> {
    // Base64 carries 3 bytes in 4 characters, the padding none.
    let padding = a.data.bytes().rev().take_while(|&b| b == b'=').count();
    let decoded = (a.data.len() / 4 * 3).saturating_sub(padding);
    let (block, size, max) = match a.media_type.as_str() {
        t if IMAGE_TYPES.contains(&t) => (
            json!({ "type": "image", "source": { "type": "base64", "media_type": t, "data": a.data } }),
            decoded,
            MAX_IMAGE,
        ),
        "application/pdf" => (
            json!({
                "type": "document",
                "title": a.name,
                "source": { "type": "base64", "media_type": "application/pdf", "data": a.data },
            }),
            decoded,
            18 * MB,
        ),
        "text/plain" => (
            json!({
                "type": "document",
                "title": a.name,
                "source": { "type": "text", "media_type": "text/plain", "data": a.data },
            }),
            a.data.len(),
            MAX_TEXT,
        ),
        t => bail!(tr!(
            "« {name} » ({t}) ne peut pas être joint : les fichiers acceptés sont les images (PNG, JPEG, GIF, WebP), les PDF et les fichiers texte.",
            "“{name}” ({t}) can’t be attached: the files accepted are images (PNG, JPEG, GIF, WebP), PDFs and text files.",
            name = a.name
        )),
    };
    if size > max {
        let lang = i18n::ui();
        bail!(tr_in!(
            lang,
            "{name} dépasse {max}",
            "{name} is over {max}",
            name = a.name,
            max = size_label(lang, max)
        ));
    }
    Ok((block, size))
}

/// What the added instructions (`--append-system-prompt`) may weigh as a command-line argument,
/// once escaped (see `escaped_len`). cmd.exe takes 8191 characters for the whole command line of
/// a `.cmd` launcher (npm's claude.cmd); the rest goes to the other arguments.
pub(crate) const PROTOCOL_BUDGET: usize = 6000;

/// What `s` weighs as an argument of a `.cmd`: Rust escapes it for cmd.exe, where `%` becomes a
/// longer sequence (8 characters) and `"` is doubled, as are the backslashes just before it (and
/// those ending the argument). Every backslash counts 2, the safe bound of those runs. Other
/// characters count their UTF-8 bytes, which is never less than their UTF-16 units.
pub(crate) fn escaped_len(s: &str) -> usize {
    s.chars()
        .map(|c| match c {
            '%' => 8,
            '"' | '\\' => 2,
            c => c.len_utf8(),
        })
        .sum()
}

const CLAUDE_BASE_ARGS: &[&str] = &[
    "--output-format",
    "stream-json",
    "--verbose",
    "--input-format",
    "stream-json",
    "--permission-prompt-tool",
    "stdio",
    "--include-partial-messages",
    // Messages sent from claude.ai (Remote Control) come back on stdout, so the app shows them.
    "--replay-user-messages",
    "--thinking-display",
    "summarized",
    "--allow-dangerously-skip-permissions",
];

pub fn supports_effort(model: &str) -> bool {
    !model.to_lowercase().contains("haiku")
}

/// Whether a model takes the permission mode `auto`, before its account's `claude` has told
/// (`told_auto`): every one but a Haiku older than 5 (`claude-haiku-4-5`, `claude-3-5-haiku`).
/// The alias `haiku` takes it, as from Claude Code 2.1.293 on, where it runs Haiku 5.5 (Haiku 4.5
/// before: `claude` then runs it in `default`). The rule of `supportsAuto` in `src/lib/models.ts`.
pub fn supports_auto(model: &str) -> bool {
    haiku_major(model).is_none_or(|major| major >= 5)
}

/// The major version of a Haiku (`claude-haiku-4-5` → 4, `claude-3-5-haiku` → 3); 99 for the
/// alias, which runs the latest; None for a model that is no Haiku.
fn haiku_major(model: &str) -> Option<u32> {
    let m = model.to_lowercase();
    let at = m.find("haiku")?;
    // A version has a digit or two: more is a date (`claude-3-5-haiku-20241022`).
    let leading = |s: &str| {
        let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().ok().filter(|_| digits.len() <= 2)
    };
    let after = leading(m[at + 5..].trim_start_matches(['-', '_']));
    let before = m[..at].strip_prefix("claude-").and_then(leading);
    Some(after.or(before).unwrap_or(99))
}

/// Whether `model` takes the mode `auto`, as a `claude` told it (`initialize`, its models'
/// `supportsAutoMode`, ESC-121): by its alias or its full id (with or without its date). None
/// when it tells nothing of the mode (a version that tells it of none) or does not list `model`.
pub fn told_auto(told: &[crate::model::ModelInfo], model: &str) -> Option<bool> {
    if told.iter().all(|m| m.supports_auto_mode.is_none()) {
        return None;
    }
    told_model(told, model).map(|t| t.supports_auto_mode == Some(true))
}

/// The model `model` names among those a `claude` told: by its alias or its full id (with or
/// without its date), its long context (`[1m]`) aside.
pub fn told_model<'a>(
    told: &'a [crate::model::ModelInfo],
    model: &str,
) -> Option<&'a crate::model::ModelInfo> {
    let m = model.to_lowercase();
    let m = m.strip_suffix("[1m]").unwrap_or(&m);
    let dated = format!("{m}-");
    told.iter()
        .find(|t| t.value.to_lowercase() == m)
        .or_else(|| {
            told.iter().find(|t| {
                let r = t.resolved_model.to_lowercase();
                r == m || r.starts_with(&dated)
            })
        })
}

/// Whether `model` takes an effort, as a `claude` told it (`supportsEffort`, absent for a model
/// without one, ESC-124). None when it tells no names of its models (a list read before), or
/// does not list `model`: the rule decides (`supports_effort`).
pub fn told_effort(told: &[crate::model::ModelInfo], model: &str) -> Option<bool> {
    if told.iter().all(|m| m.display_name.is_empty()) {
        return None;
    }
    told_model(told, model).map(|t| t.supports_effort == Some(true))
}

/// The composer's menu of the agents of an account, as its `claude` told its models
/// (`initialize`, ESC-124): in its order and by the names of its `/model`, each with its
/// efforts; `default` (Claude Code's choice, the same model as another) is not one, but marks
/// the one it runs. Empty when it tells no names (a list read before ESC-124): the fixed one
/// serves.
pub fn catalog(told: &[crate::model::ModelInfo]) -> Vec<ToldModel> {
    if told.iter().all(|m| m.display_name.is_empty()) {
        return Vec::new();
    }
    let default = told
        .iter()
        .find(|m| m.value == "default")
        .map(|m| m.resolved_model.as_str());
    let mut marked = false;
    let mut out = Vec::new();
    for m in told.iter().filter(|m| m.value != "default") {
        if out.iter().any(|o: &ToldModel| o.id == m.value) {
            continue;
        }
        let is_default = !marked && default == Some(m.resolved_model.as_str());
        marked |= is_default;
        let efforts = if m.supports_effort == Some(true) {
            m.supported_effort_levels.clone()
        } else {
            Vec::new()
        };
        out.push(ToldModel {
            from_config: false,
            id: m.value.clone(),
            label: if m.display_name.is_empty() {
                m.value.clone()
            } else {
                m.display_name.clone()
            },
            efforts,
            default_effort: None,
            images: true,
            default: is_default,
            provider: None,
            multiplier: None,
            price: None,
        });
    }
    if !marked {
        if let Some(first) = out.first_mut() {
            first.default = true;
        }
    }
    out
}

/// What the `claude` of `program` (with `env`, in `dir`) tells of its models, read by a
/// process of its own that is given no turn: `initialize` alone, which asks nothing of the API
/// (ESC-121, ESC-124), then it is stopped. No session is written, no MCP server started.
pub async fn read_models(
    program: super::Program,
    env: Vec<(String, String)>,
    dir: &std::path::Path,
) -> Result<Value> {
    let mut args: Vec<String> = CLAUDE_BASE_ARGS.iter().map(|s| s.to_string()).collect();
    args.extend(
        ["--no-session-persistence", "--strict-mcp-config"]
            .iter()
            .map(|s| s.to_string()),
    );
    let opts = SpawnOpts {
        args,
        program,
        cwd: dir.to_string_lossy().into_owned(),
        env,
    };
    let p = ClaudeProcess::spawn(opts, |_| {}, |_, _| {})?;
    let resp = ClaudeProcess::control(
        &p,
        json!({ "subtype": "initialize" }),
        Duration::from_secs(60),
    )
    .await;
    p.close_input();
    p.kill();
    Ok(resp?["models"].clone())
}

/// The arguments of the conversation of `s`: its model, effort and mode, its added instructions,
/// and where it starts from.
fn conversation_args(s: &Spawn) -> Vec<String> {
    let mut a: Vec<String> = CLAUDE_BASE_ARGS.iter().map(|s| s.to_string()).collect();
    a.extend(["--model".into(), s.model.clone()]);
    if s.takes_effort.unwrap_or_else(|| supports_effort(&s.model)) {
        a.extend(["--effort".into(), s.effort.clone()]);
    }
    a.extend(["--permission-mode".into(), s.mode.clone()]);
    // A discussion (ESC-261): its tools that write or run a command taken away, whatever its mode
    // becomes (a plan approved puts it in another). In one value, comma separated, as the Agent
    // SDK gives them: what follows is not read as more.
    if s.read_only {
        a.push(format!(
            "--disallowedTools={}",
            crate::discussion::WRITES.join(",")
        ));
    }
    if let Some(p) = s.append_prompt.as_deref().filter(|p| !p.trim().is_empty()) {
        // On one line: a `.cmd` launcher (npm's claude.cmd) takes no argument with line breaks.
        let one_line = p.split_whitespace().collect::<Vec<_>>().join(" ");
        a.extend(["--append-system-prompt".into(), one_line]);
    }
    match &s.start {
        Start::New => {}
        Start::Resume { session } => a.push(format!("--resume={session}")),
        Start::Fork { of, at } => {
            // A copy's conversation so far is its original's, under a new session: the original's
            // is left as it is.
            a.extend([format!("--resume={of}"), "--fork-session".into()]);
            // As it was when copied: the turns the original ran since stay out.
            if let Some(at) = at {
                a.push(format!("--resume-session-at={at}"));
            }
        }
    }
    a
}

/// All the arguments of the process of `s`: its conversation's, then the folders it reads without
/// a question, then what it is given of Escouade's server. Never the token, only the path of the
/// file that holds it (a `.cmd` launcher takes 8,191 characters at most, and the app's log writes
/// the arguments). `--disallowedTools` and `--mcp-config` take several values: given last, nothing
/// after them is read as one.
fn args(s: &Spawn) -> Vec<String> {
    let mut a = conversation_args(s);
    a.extend(
        s.read_dirs
            .iter()
            .map(|d| format!("--add-dir={}", d.display())),
    );
    if let (Some(own), Escouade::Granted { .. }) = (&s.settings, &s.escouade) {
        a.extend(["--settings".into(), own.to_string_lossy().into_owned()]);
    }
    a.extend(mcp::launch(&with_settings(s)).args);
    a
}

/// What `s` is given of Escouade's server, its settings file of its own (`Spawn::settings`: its
/// fence) read in place of the refusal's (Claude Code reads the last `--settings` only, and it
/// holds the refusal too); given the server, its settings before it.
fn with_settings(s: &Spawn) -> Escouade {
    let Some(own) = &s.settings else {
        return s.escouade.clone();
    };
    match &s.escouade {
        Escouade::Denied { .. } => Escouade::Denied {
            file: Some(own.clone()),
        },
        granted => granted.clone(),
    }
}

/// The arguments of the conversation of the agent of `m` (the tests').
#[cfg(test)]
pub(crate) fn claude_args(m: &AgentMeta) -> Vec<String> {
    conversation_args(&Spawn::of(
        m,
        Program::Native(std::path::PathBuf::new()),
        Vec::new(),
    ))
}

/// The arguments as the log shows them: the protocol appended to the system prompt (up to several
/// KB of ticket text) is only counted, not copied.
fn args_for_log(args: &[String]) -> String {
    let mut shown: Vec<String> = Vec::with_capacity(args.len());
    let mut elide = false;
    for a in args {
        if std::mem::take(&mut elide) {
            shown.push(format!("<{} chars>", a.chars().count()));
            continue;
        }
        elide = a == "--append-system-prompt";
        shown.push(a.clone());
    }
    shown.join(" ")
}

/// The JSON answer of `claude -p --output-format json`: the whole output, or its last line that
/// is JSON when something came before (a warning of `wsl.exe` for a `claude` of WSL).
fn json_answer(stdout: &[u8]) -> Result<Value> {
    serde_json::from_slice(stdout).or_else(|e| {
        String::from_utf8_lossy(stdout)
            .lines()
            .rev()
            .find_map(|l| {
                // Also without the NULs of a warning written in UTF-16.
                let l = l.trim_matches(|c: char| c.is_whitespace() || c == '\0');
                serde_json::from_str::<Value>(l)
                    .ok()
                    .filter(Value::is_object)
            })
            .ok_or_else(|| e.into())
    })
}

impl Engine for Claude {
    fn store(&self) -> crate::accounts::Store {
        use crate::accounts::{Store, CONFIG_DIR_VAR, SHARED_DIRS, SHARED_FILES};
        Store {
            home: ".claude",
            var: CONFIG_DIR_VAR,
            files: &SHARED_FILES,
            dirs: &SHARED_DIRS,
            login: &[],
            xdg: None,
        }
    }

    fn config_dir(
        &self,
        account: &crate::model::Account,
        settings: &crate::model::Settings,
    ) -> Option<std::path::PathBuf> {
        Some(crate::accounts::claude_config_dir(account, settings))
    }

    /// The only harness with a path in the settings (« Chemin de claude »).
    fn settings_path<'a>(&self, settings: &'a crate::model::Settings) -> &'a str {
        &settings.claude_path
    }

    fn default_model(&self) -> &'static str {
        crate::accounts::CLAUDE_DEFAULT_MODEL
    }

    /// For an account with a folder of its own, those of `IDENTITY_VARS` that are set given empty
    /// (read as not set), and `CLAUDE_SECURESTORAGE_CONFIG_DIR`, when set, its folder: its own
    /// sign-in is the one it uses. Principal, which signs in as the machine's user does, keeps them.
    fn account_env(
        &self,
        account: &crate::model::Account,
        inherited: &dyn Fn(&str) -> bool,
    ) -> Vec<(String, String)> {
        use crate::accounts::{own_dir, IDENTITY_VARS, SECURE_STORAGE_VAR};
        let mut env = Vec::new();
        if let Some(dir) = own_dir(account) {
            env.extend(
                IDENTITY_VARS
                    .iter()
                    .filter(|v| inherited(v))
                    .map(|v| (v.to_string(), String::new())),
            );
            if inherited(SECURE_STORAGE_VAR) {
                env.push((SECURE_STORAGE_VAR.to_string(), dir.to_string()));
            }
        }
        env
    }

    /// The sign-in it keeps for the account (its folder, or the macOS keychain), and the email of
    /// its `.claude.json`.
    fn account_status<'a>(
        &'a self,
        account: &'a crate::model::Account,
        settings: &'a crate::model::Settings,
    ) -> BoxFuture<'a, crate::accounts::AccountStatus> {
        use crate::accounts::{claude_json, config_dir, email_in, AccountStatus};
        Box::pin(async move {
            let sign_in = crate::usage::sign_in(account, settings);
            let stored = match crate::usage::read_stored(&sign_in.dir, &sign_in.service).await {
                Ok(stored) => Some(stored),
                // A file being written, the keychain refused: not signed in as far as can be told.
                Err(e) => {
                    log::debug!("sign-in of account {}: {e:#}", account.id);
                    None
                }
            };
            let connected = stored
                .as_ref()
                .is_some_and(|s| s.credentials != crate::usage::Credentials::Missing);
            AccountStatus {
                connected,
                stamp: stored.and_then(|s| s.stamp),
                email: connected
                    .then(|| email_in(&claude_json(account, settings)))
                    .flatten(),
                dir: config_dir(account, settings).to_string_lossy().into_owned(),
            }
        })
    }

    fn login_shell(&self) -> String {
        "claude".into()
    }

    fn workflow_folders(&self) -> crate::workflow::Folders {
        use crate::workflow::DocKind;
        &[
            (".claude/commands", DocKind::Command),
            (".claude/skills", DocKind::Skill),
            (".claude/agents", DocKind::Agent),
        ]
    }

    fn workflow_user(&self) -> Option<crate::workflow::UserRoot> {
        Some(crate::workflow::UserRoot::Claude)
    }

    fn hook_context(&self) -> bool {
        true
    }

    fn id(&self) -> crate::model::EngineId {
        crate::model::EngineId::CLAUDE
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }

    fn speaker(&self) -> &'static str {
        "Claude"
    }

    fn instructions_room(&self) -> Option<super::Room> {
        // `--append-system-prompt`, on a command line that may be a `.cmd`'s.
        Some(super::Room {
            max: PROTOCOL_BUDGET,
            weigh: escaped_len,
        })
    }

    fn subagent_relay(&self, lang: Lang, title: &str, task: &str, text: &str) -> Option<String> {
        // Claude Code lets no host write to a subagent: its agent passes the message on with its
        // `SendMessage` tool, to the `agentId` Claude Code gave the subagent.
        Some(tr_in!(
            lang,
            "Transmets ce message au sous-agent « {title} » (agentId {task}) avec l’outil SendMessage, tel quel, puis reprends ce que tu faisais :

{text}",
            "Pass this message on to the subagent “{title}” (agentId {task}) with the SendMessage tool, as it is, then go back to what you were doing:

{text}"
        ))
    }

    fn locate(&self, configured: &str, wsl_distro: &str) -> Option<super::Program> {
        locate::resolve_binary(configured, wsl_distro)
    }

    fn tested(&self) -> Tested {
        TESTED
    }

    fn oldest_reason(&self, lang: Lang) -> String {
        tr_in!(
            lang,
            "son flux JSON n’a été vérifié qu’à partir de celle-ci (docs/PROTOCOL.md)",
            "its JSON stream was only checked from that one on (docs/PROTOCOL.md)"
        )
    }

    fn update_command(&self) -> &'static str {
        "claude update"
    }

    fn version_args(&self) -> &'static [&'static str] {
        &["--version"]
    }

    fn accounts(&self) -> super::registry::AccountStore {
        super::registry::AccountStore::ConfigDir {
            var: crate::accounts::CONFIG_DIR_VAR,
        }
    }

    fn no_self_update(&self) -> &'static [(&'static str, &'static str)] {
        &[NO_AUTOUPDATER]
    }

    fn caps(&self) -> EngineCaps {
        caps()
    }

    fn terminal(&self) -> Option<&'static dyn super::TerminalView> {
        Some(&terminal::CLAUDE_TERMINAL)
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
                effort: supports_effort(id),
                auto: supports_auto(id),
            })
            .collect()
    }

    fn efforts(&self) -> &'static [&'static str] {
        &EFFORTS
    }

    fn supports_effort(&self, model: &str) -> bool {
        supports_effort(model)
    }

    fn supports_auto(&self, model: &str) -> bool {
        supports_auto(model)
    }

    /// A million tokens for the models of the long context (`opus[1m]`), else 200 000.
    fn context_window(&self, model: &str) -> u64 {
        if model.to_lowercase().contains("[1m]") {
            1_000_000
        } else {
            200_000
        }
    }

    fn spawn(
        &self,
        spawn: Spawn,
        on_frame: super::OnFrame,
        on_exit: super::OnExit,
    ) -> Result<Process> {
        let mut env = spawn.env.clone();
        env.extend(mcp::launch(&spawn.escouade).env);
        let opts = SpawnOpts {
            args: args(&spawn),
            program: spawn.program,
            cwd: spawn.cwd,
            env,
        };
        log::info!(
            "{}: starting {} {} in {} (account {})",
            spawn.label,
            opts.program.describe(),
            args_for_log(&opts.args),
            opts.cwd,
            spawn.account
        );
        let p: Process = ClaudeProcess::spawn(opts, on_frame, on_exit)?;
        Ok(p)
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        user_content(text, attachments).map(Message)
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        rt.claude_frame(frame, fx);
    }

    fn start_lost(&self, stderr: &str) -> Option<StartLost> {
        if stderr.contains("No conversation found") {
            Some(StartLost::Session)
        } else if stderr.contains("No message found with message.uuid") {
            Some(StartLost::ForkPoint)
        } else {
            None
        }
    }

    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(one_shot(ask))
    }

    fn writer(&self, weight: Weight) -> &'static str {
        writer(weight)
    }

    fn models_by_alias(&self) -> bool {
        true
    }

    fn told_effort(&self, models: &[crate::model::ModelInfo], model: &str) -> Option<bool> {
        told_effort(models, model)
    }

    fn told_auto(&self, models: &[crate::model::ModelInfo], model: &str) -> Option<bool> {
        told_auto(models, model)
    }

    fn read_told_models<'a>(
        &'a self,
        program: Program,
        env: Vec<(String, String)>,
        dir: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<Value>> {
        Box::pin(read_models(program, env, dir))
    }

    fn missing_repair(&self, lang: Lang) -> Option<String> {
        Some(missing_line(lang))
    }

    fn interrupted_line(&self, lang: Lang) -> Option<String> {
        interrupted_line(lang)
    }

    fn command(&self) -> &'static str {
        "claude"
    }

    fn oauth_sign_in(&self) -> bool {
        true
    }

    fn declares_by_program(&self) -> bool {
        true
    }

    fn principal_sessions(&self) -> bool {
        true
    }

    fn session_cwd(&self, config_dir: &std::path::Path, session_id: &str) -> Option<String> {
        crate::agentprs::last_cwd(config_dir, session_id)
    }

    fn memory_dir(
        &self,
        config_dir: &std::path::Path,
        project_path: &str,
    ) -> Option<std::path::PathBuf> {
        Some(crate::memory::claude_dir(config_dir, project_path))
    }

    fn signed_in_by_files(
        &self,
        account: &crate::model::Account,
        settings: &crate::model::Settings,
    ) -> bool {
        crate::accounts::email_in(&crate::accounts::claude_json(account, settings)).is_some()
    }

    /// Its texts' model by its alias, as its prices know it.
    fn writer_priced(&self, _weight: Weight) -> String {
        "haiku".to_string()
    }

    fn grant_file(&self, url: &str, token: &str) -> Option<Vec<u8>> {
        Some(mcp::grant_file(url, token))
    }

    fn refusal(&self) -> Option<Value> {
        Some(mcp::refusal())
    }

    fn escouade_launch(&self, escouade: &Escouade) -> super::mcp::Launch {
        mcp::launch(escouade)
    }

    fn fence_settings(
        &self,
        refused: bool,
        fence: &crate::fence::Fence,
        program: &Program,
    ) -> Option<Value> {
        let mut v = crate::fence::claude_settings(fence, program);
        if refused {
            if let (Some(o), Value::Object(r)) = (v.as_object_mut(), mcp::refusal()) {
                o.extend(r);
            }
        }
        Some(v)
    }

    fn declared(&self, config: &std::path::Path) -> Standing {
        mcp::declared(config)
    }

    fn add_declaration<'a>(
        &'a self,
        target: &'a Target,
        entry: &'a Entry,
        secrets: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(mcp::add(target, entry, secrets))
    }

    fn remove_declaration<'a>(
        &'a self,
        target: &'a Target,
        secrets: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(mcp::remove(target, secrets))
    }

    fn manual_declaration(
        &self,
        windows: bool,
        program: &str,
        config_dir: Option<&str>,
        entry: &Entry,
        replace: bool,
    ) -> String {
        mcp::manual_declaration(windows, program, config_dir, entry, replace)
    }
}

/// One question to Claude (`claude -p`, no session, no settings, no MCP), as `ask` says: its
/// answer (its limit is `oneshot::ask`'s, the question's writing included: a `claude` that never
/// reads it would hold it forever).
async fn one_shot(ask: OneShot<'_>) -> Result<String> {
    let OneShot {
        program,
        env,
        weight,
        cwd,
        system,
        prompt,
        limit: _,
        model: _,
    } = ask;
    let (model, tools) = match weight {
        Weight::Light => ("haiku", ""),
        Weight::Reading => ("sonnet", "Read,Glob,Grep"),
    };
    let args = [
        "-p",
        "--model",
        model,
        "--output-format",
        "json",
        "--no-session-persistence",
        "--tools",
        tools,
        "--setting-sources",
        "",
        // No MCP servers (account connectors included): nothing that invites the model to act.
        "--strict-mcp-config",
        "--system-prompt",
        system,
    ]
    .map(String::from);
    // Reading needs no permission in its folder, and is refused outside of it: nothing more is
    // allowed.
    let out = super::oneshot::run(&program, cwd, &args, &env, prompt, writer(weight)).await?;
    let v = json_answer(&out)?;
    Ok(v["result"].as_str().unwrap_or("").to_string())
}

/// The model that answers a one-shot question of `weight`, as the interface names it.
fn writer(weight: Weight) -> &'static str {
    match weight {
        Weight::Light => "Claude Haiku",
        Weight::Reading => "Claude Sonnet",
    }
}

/// The answer to a control request of the CLI that lets a tool run.
fn allowed(call: &str, input: Value) -> Value {
    json!({ "behavior": "allow", "updatedInput": input, "toolUseID": call })
}

/// How long a change of model, effort or mode may take.
const SETTING: Duration = Duration::from_secs(15);

impl EngineProcess for ClaudeProcess {
    fn is_alive(&self) -> bool {
        ClaudeProcess::is_alive(self)
    }

    fn usage(&self) -> Option<JobUsage> {
        ClaudeProcess::usage(self)
    }

    fn still_running(&self) -> u32 {
        ClaudeProcess::still_running(self)
    }

    fn pids(&self) -> Vec<u32> {
        ClaudeProcess::pids(self)
    }

    fn close_input(&self) {
        ClaudeProcess::close_input(self)
    }

    fn kill(&self) {
        ClaudeProcess::kill(self)
    }

    fn wait_exit(&self, within: Duration) -> BoxFuture<'_, bool> {
        Box::pin(ClaudeProcess::wait_exit(self, within))
    }

    fn wait_gone(&self, within: Duration) -> BoxFuture<'_, bool> {
        Box::pin(ClaudeProcess::wait_gone(self, within))
    }

    fn initialize(&self) -> BoxFuture<'_, Result<Ready>> {
        Box::pin(async move {
            let resp = ClaudeProcess::control(
                self,
                json!({ "subtype": "initialize" }),
                Duration::from_secs(90),
            )
            .await?;
            Ok(Ready {
                commands: resp["commands"].as_array().cloned().unwrap_or_default(),
                models: resp["models"].clone(),
                catalog: None,
            })
        })
    }

    fn send_message(&self, uuid: &str, message: &Message) -> Result<()> {
        self.send(&json!({
            "type": "user",
            "message": { "role": "user", "content": message.0 },
            "parent_tool_use_id": null,
            "uuid": uuid,
        }))
    }

    fn interrupt(&self, cancel_queued: bool, timeout: Duration) -> BoxFuture<'_, Result<()>> {
        let request = if cancel_queued {
            json!({ "subtype": "interrupt", "cancel_queued": true })
        } else {
            json!({ "subtype": "interrupt" })
        };
        Box::pin(async move {
            ClaudeProcess::control(self, request, timeout)
                .await
                .map(drop)
        })
    }

    fn set_model<'a>(&'a self, model: &'a str) -> BoxFuture<'a, Result<()>> {
        let request = json!({ "subtype": "set_model", "model": model });
        Box::pin(async move {
            ClaudeProcess::control(self, request, SETTING)
                .await
                .map(drop)
        })
    }

    fn set_effort<'a>(&'a self, effort: &'a str) -> BoxFuture<'a, Result<()>> {
        let request =
            json!({ "subtype": "apply_flag_settings", "settings": { "effortLevel": effort } });
        Box::pin(async move {
            ClaudeProcess::control(self, request, SETTING)
                .await
                .map(drop)
        })
    }

    fn set_mode<'a>(&'a self, mode: &'a str) -> BoxFuture<'a, Result<()>> {
        let request = json!({ "subtype": "set_permission_mode", "mode": mode });
        Box::pin(async move {
            ClaudeProcess::control(self, request, SETTING)
                .await
                .map(drop)
        })
    }

    fn answer_permission(&self, request_id: &str, answer: PermissionAnswer) -> Result<()> {
        let response = match answer {
            PermissionAnswer::Allow {
                call,
                input,
                resume,
            } => {
                let mut r = allowed(&call, input);
                // A plan approved: Claude Code would leave Plan for `default` (ESC-277).
                if let Some(mode) = resume {
                    r["updatedPermissions"] =
                        json!([{ "type": "setMode", "mode": mode, "destination": "session" }]);
                }
                r
            }
            PermissionAnswer::Always { call, input, rules } => {
                let mut r = allowed(&call, input);
                r["updatedPermissions"] = rules;
                r
            }
            PermissionAnswer::Deny { call, message } => {
                json!({ "behavior": "deny", "toolUseID": call, "message": message })
            }
        };
        self.respond(request_id, response)
    }

    fn answer_question(&self, request_id: &str, call: &str, input: Value) -> Result<()> {
        self.respond(request_id, allowed(call, input))
    }

    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        Box::pin(async move {
            let v = ClaudeProcess::control(
                self,
                json!({ "subtype": "get_usage", "skip_behaviors": true }),
                Duration::from_secs(20),
            )
            .await?;
            let windows = &v["rate_limits"];
            Ok(windows.is_object().then(|| windows.clone()))
        })
    }

    fn rename<'a>(&'a self, title: &'a str) -> BoxFuture<'a, Result<()>> {
        let request = json!({ "subtype": "rename_session", "title": title, "source": "host" });
        Box::pin(async move {
            ClaudeProcess::control(self, request, Duration::from_secs(10))
                .await
                .map(drop)
        })
    }

    fn stop_task<'a>(&'a self, task: &'a str) -> BoxFuture<'a, Result<()>> {
        let request = json!({ "subtype": "stop_task", "task_id": task });
        Box::pin(async move {
            ClaudeProcess::control(self, request, Duration::from_secs(15))
                .await
                .map(drop)
        })
    }

    fn task_output<'a>(&'a self, task: &'a str) -> BoxFuture<'a, Result<Value>> {
        let request = json!({ "subtype": "get_task_output", "task_id": task });
        Box::pin(ClaudeProcess::control(
            self,
            request,
            Duration::from_secs(10),
        ))
    }

    fn remote_control(&self, request: Remote, timeout: Duration) -> BoxFuture<'_, Result<Value>> {
        let request = match request {
            Remote::Off => json!({ "subtype": "remote_control", "enabled": false }),
            Remote::On {
                name,
                keep,
                reattach,
            } => {
                let mut r = json!({
                    "subtype": "remote_control", "enabled": true, "name": name,
                    "keep_session_on_exit": keep,
                });
                if let Some(s) = reattach {
                    r["reattach_session_id"] = json!(s);
                }
                r
            }
        };
        Box::pin(ClaudeProcess::control(self, request, timeout))
    }

    fn control(&self, request: Value, timeout: Duration) -> BoxFuture<'_, Result<Value>> {
        Box::pin(ClaudeProcess::control(self, request, timeout))
    }

    fn reply(&self, request_id: &str, answer: std::result::Result<Value, String>) -> Result<()> {
        match answer {
            Ok(response) => self.respond(request_id, response),
            Err(error) => self.respond_error(request_id, &error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Criterion, ModelInfo, Ticket};

    #[test]
    fn a_claude_not_found_is_said_with_what_puts_it_back() {
        use crate::i18n::Lang::{En, Fr};
        let install = if cfg!(windows) {
            "irm https://claude.ai/install.ps1 | iex"
        } else {
            "curl -fsSL https://claude.ai/install.sh | bash"
        };
        assert_eq!(
            missing_line_with(Fr, None),
            format!("Claude Code est introuvable : réinstalle-le (`{install}`, ou `npm install -g @anthropic-ai/claude-code`), ou indique son chemin dans Réglages › Harness.")
        );
        assert_eq!(
            missing_line_with(En, None),
            format!("Claude Code can’t be found: install it again (`{install}`, or `npm install -g @anthropic-ai/claude-code`), or set its path in Settings › Harness.")
        );
        // Its installation by npm cut half-way, as on the client's Mac.
        let left = std::path::Path::new("/opt/homebrew/bin/.claude-Xa3k9");
        assert_eq!(
            missing_line_with(Fr, Some(left)),
            "L’installation de Claude Code a été interrompue (il en reste /opt/homebrew/bin/.claude-Xa3k9) : relance `npm install -g @anthropic-ai/claude-code` dans un terminal."
        );
        assert_eq!(
            missing_line_with(En, Some(left)),
            "Claude Code’s installation was interrupted (/opt/homebrew/bin/.claude-Xa3k9 is left over): run `npm install -g @anthropic-ai/claude-code` again in a terminal."
        );
    }

    #[test]
    fn auto_mode_is_taken_by_every_model_but_a_haiku_older_than_5_as_the_composer_offers_it() {
        // The rule of `supportsAuto` (src/lib/models.ts), before an account's `claude` told.
        for model in [
            "claude-haiku-4-5",
            "CLAUDE-HAIKU-4-5-20251001",
            "claude-3-5-haiku-20241022",
            "claude-3-haiku",
        ] {
            assert!(!supports_auto(model), "{model}");
        }
        // The alias runs Haiku 5.5 from Claude Code 2.1.293 on.
        for model in [
            "haiku",
            "Haiku",
            "claude-haiku-5-5",
            "claude-haiku-6",
            "sonnet",
            "opus",
            "claude-sonnet-4-5",
            "claude-opus-4-1",
            "",
        ] {
            assert!(supports_auto(model), "{model}");
        }
        // The effort keeps its rule.
        for model in ["haiku", "claude-haiku-4-5"] {
            assert!(!supports_effort(model), "{model}");
        }
    }

    /// A model as `initialize` lists it.
    fn told(value: &str, resolved: &str, auto: Option<bool>) -> ModelInfo {
        ModelInfo {
            value: value.into(),
            resolved_model: resolved.into(),
            supports_auto_mode: auto,
            ..Default::default()
        }
    }

    /// The models of the `initialize` of Claude Code 2.1.302 on this machine's Principal (real
    /// trial of ESC-124, abridged), with a model more (`mythos`, as another account has it) and
    /// Fable less.
    fn told_2_1_302() -> Vec<ModelInfo> {
        let efforts = json!(["low", "medium", "high", "xhigh", "max"]);
        ModelInfo::list(&json!([
            { "value": "default", "resolvedModel": "claude-opus-5-5", "displayName": "Default (recommended)",
              "supportsEffort": true, "supportedEffortLevels": efforts, "supportsAutoMode": true },
            { "value": "opus", "resolvedModel": "claude-opus-5-5", "displayName": "Opus 5.5",
              "supportsEffort": true, "supportedEffortLevels": efforts, "supportsAutoMode": true },
            { "value": "mythos", "resolvedModel": "claude-mythos-1", "displayName": "Mythos",
              "supportsEffort": true, "supportedEffortLevels": ["high", "max"], "supportsAutoMode": true },
            { "value": "haiku", "resolvedModel": "claude-haiku-5-5", "displayName": "Haiku 5.5",
              "supportsEffort": true, "supportedEffortLevels": efforts, "supportsAutoMode": true },
            { "value": "claude-haiku-4-5-20251001", "resolvedModel": "claude-haiku-4-5-20251001",
              "displayName": "Haiku 4.5", "supportsThinkingOff": true },
        ]))
    }

    #[test]
    fn the_menu_of_an_account_is_what_its_claude_lists() {
        let list = told_2_1_302();
        let menu = catalog(&list);
        let ids: Vec<&str> = menu.iter().map(|m| m.id.as_str()).collect();
        // In its order, `default` aside; a model more is there, the one it lacks is not.
        assert_eq!(
            ids,
            ["opus", "mythos", "haiku", "claude-haiku-4-5-20251001"]
        );
        let labels: Vec<&str> = menu.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, ["Opus 5.5", "Mythos", "Haiku 5.5", "Haiku 4.5"]);
        // `default` marks the one it runs.
        let defaults: Vec<&str> = menu
            .iter()
            .filter(|m| m.default)
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(defaults, ["opus"]);
        // Each with its efforts; none for the one that takes none.
        assert_eq!(menu[1].efforts, ["high", "max"]);
        assert!(menu[3].efforts.is_empty());
        assert!(menu.iter().all(|m| m.images));
        // Without `default`: its first.
        let without: Vec<ModelInfo> = list
            .iter()
            .filter(|m| m.value != "default")
            .cloned()
            .collect();
        assert!(catalog(&without)[0].default);
        // A list kept before ESC-124 (no names) makes none: the fixed one serves.
        let old = vec![told("opus", "claude-opus-5-5", Some(true))];
        assert!(catalog(&old).is_empty());
        assert!(catalog(&[]).is_empty());
    }

    #[test]
    fn the_effort_follows_what_the_claude_of_the_account_told_of_its_models() {
        let list = told_2_1_302();
        assert_eq!(told_effort(&list, "haiku"), Some(true));
        assert_eq!(told_effort(&list, "claude-haiku-4-5"), Some(false));
        assert_eq!(told_effort(&list, "mythos"), Some(true));
        assert_eq!(told_effort(&list, "opus[1m]"), Some(true));
        // Not listed, or a list without names: the rule.
        assert_eq!(told_effort(&list, "sonnet"), None);
        assert_eq!(
            told_effort(&[told("haiku", "claude-haiku-5-5", Some(true))], "haiku"),
            None
        );
        // Its process is given the effort as told, else by the rule.
        let m = AgentMeta {
            model: "haiku".into(),
            effort: "high".into(),
            mode: "auto".into(),
            ..Default::default()
        };
        let mut s = Spawn::of(&m, Program::Native(std::path::PathBuf::new()), Vec::new());
        assert!(!conversation_args(&s).contains(&"--effort".to_string()));
        s.takes_effort = Some(true);
        let a = conversation_args(&s);
        let at = a.iter().position(|x| x == "--effort").unwrap();
        assert_eq!(a[at + 1], "high");
        s.model = "mythos-sans-effort".into();
        s.takes_effort = Some(false);
        assert!(!conversation_args(&s).contains(&"--effort".to_string()));
    }

    #[test]
    fn the_mode_auto_follows_what_the_claude_of_the_account_told_of_its_models() {
        // As Claude Code 2.1.302 answers `initialize` (real trial, ESC-121): Haiku 5.5 takes it,
        // Haiku 4.5 says nothing of it.
        let new = ModelInfo::list(&json!([
            { "value": "opus", "resolvedModel": "claude-opus-5-5", "supportsAutoMode": true },
            { "value": "haiku", "resolvedModel": "claude-haiku-5-5", "supportsAutoMode": true },
            { "value": "claude-haiku-4-5-20251001", "resolvedModel": "claude-haiku-4-5-20251001" },
        ]));
        assert_eq!(told_auto(&new, "haiku"), Some(true));
        assert_eq!(told_auto(&new, "HAIKU"), Some(true));
        assert_eq!(told_auto(&new, "claude-haiku-5-5"), Some(true));
        assert_eq!(told_auto(&new, "claude-haiku-4-5"), Some(false));
        assert_eq!(told_auto(&new, "claude-haiku-4-5-20251001"), Some(false));
        assert_eq!(told_auto(&new, "opus"), Some(true));
        // Not listed: the rule decides.
        assert_eq!(told_auto(&new, "sonnet"), None);
        // As 2.1.292 answers it: its alias runs Haiku 4.5, without the mode.
        let old = vec![
            told("opus", "claude-opus-5-5", Some(true)),
            told("haiku", "claude-haiku-4-5-20251001", None),
        ];
        assert_eq!(told_auto(&old, "haiku"), Some(false));
        // A version that tells it of no model, or none told yet: the rule decides.
        let silent = vec![told("haiku", "claude-haiku-4-5-20251001", None)];
        assert_eq!(told_auto(&silent, "haiku"), None);
        assert_eq!(told_auto(&[], "haiku"), None);
    }

    #[test]
    fn the_answer_of_claude_p_is_found_after_a_warning() {
        let answer = br#"{"type":"result","result":"feat: x"}"#;
        assert_eq!(json_answer(answer).unwrap()["result"], "feat: x");
        let warned =
            "wsl: Une configuration de proxy localhost a été détectée\r\n{\"result\":\"ok\"}\r\n";
        assert_eq!(json_answer(warned.as_bytes()).unwrap()["result"], "ok");
        // The warning in UTF-16, as wsl.exe writes it without WSL_UTF8.
        let utf16: Vec<u8> = "wsl: x\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .chain(*b"{\"result\":\"ok\"}")
            .collect();
        assert_eq!(json_answer(&utf16).unwrap()["result"], "ok");
        assert!(json_answer(b"pas du JSON").is_err());
        assert!(json_answer(b"").is_err());
    }

    #[test]
    fn args_for_haiku_skip_effort() {
        let m = AgentMeta {
            model: "haiku".into(),
            effort: "high".into(),
            mode: "auto".into(),
            session_id: Some("s1".into()),
            ..Default::default()
        };
        let a = claude_args(&m);
        assert!(!a.contains(&"--effort".to_string()));
        assert!(a.contains(&"--resume=s1".to_string()));
        let m = AgentMeta {
            model: "opus".into(),
            effort: "max".into(),
            mode: "plan".into(),
            ..Default::default()
        };
        let a = claude_args(&m);
        assert!(a.windows(2).any(|w| w == ["--effort", "max"]));
    }

    #[test]
    fn a_copy_forks_the_session_of_its_original_until_it_has_its_own() {
        let copy = AgentMeta {
            model: "opus".into(),
            effort: "high".into(),
            mode: "auto".into(),
            fork_of: Some("s1".into()),
            ..Default::default()
        };
        let a = claude_args(&copy);
        assert!(
            a.windows(2).any(|w| w == ["--resume=s1", "--fork-session"]),
            "{a:?}"
        );
        // An original that ran no turn since it was recorded: its session as it is.
        assert!(!a.iter().any(|x| x.starts_with("--resume-session-at")));
        // Where it was copied: the turns the original ran since are left out.
        let pinned = AgentMeta {
            fork_at: Some("e1".into()),
            ..copy
        };
        let a = claude_args(&pinned);
        assert!(
            a.windows(3)
                .any(|w| w == ["--resume=s1", "--fork-session", "--resume-session-at=e1"]),
            "{a:?}"
        );
        // Its own session once a turn gave it one: the original's is read no more.
        let own = AgentMeta {
            session_id: Some("s2".into()),
            ..pinned
        };
        let a = claude_args(&own);
        assert!(a.contains(&"--resume=s2".to_string()), "{a:?}");
        assert!(
            !a.iter().any(|x| x == "--fork-session"
                || x == "--resume=s1"
                || x.starts_with("--resume-session-at")),
            "{a:?}"
        );
    }

    #[test]
    fn claude_codes_agents_are_claude_and_its_added_instructions_go_on_its_command_line() {
        use crate::engine::Engine;
        // « Claude attend ta réponse », as before the other harnesses.
        assert_eq!(CLAUDE.speaker(), "Claude");
        // Its room: a `.cmd` argument's, weighed as cmd.exe escapes it.
        let room = CLAUDE.instructions_room().unwrap();
        assert_eq!(room.max, PROTOCOL_BUDGET);
        assert_eq!((room.weigh)("a%\"\\é"), 1 + 8 + 2 + 2 + 2);
        // The instructions of a spawn reach the process as `--append-system-prompt`.
        let spawn = Spawn::of(
            &AgentMeta {
                append_prompt: Some("Protocole du ticket".into()),
                ..Default::default()
            },
            Program::Native(std::path::PathBuf::new()),
            Vec::new(),
        );
        let args = conversation_args(&spawn);
        let at = args
            .iter()
            .position(|a| a == "--append-system-prompt")
            .unwrap();
        assert_eq!(args[at + 1], "Protocole du ticket");
    }

    #[test]
    fn the_protocol_is_appended_to_the_system_prompt_on_one_line() {
        let m = AgentMeta {
            model: "sonnet".into(),
            effort: "high".into(),
            mode: "auto".into(),
            append_prompt: Some("Ligne 1\nLigne 2 « % & \" »".into()),
            ..Default::default()
        };
        let a = claude_args(&m);
        let i = a
            .iter()
            .position(|x| x == "--append-system-prompt")
            .unwrap();
        assert_eq!(a[i + 1], "Ligne 1 Ligne 2 « % & \" »");
        assert!(!claude_args(&AgentMeta::default()).contains(&"--append-system-prompt".to_string()));
        // Blank: nothing to append.
        let blank = AgentMeta {
            append_prompt: Some(" \n ".into()),
            ..Default::default()
        };
        assert!(!claude_args(&blank).contains(&"--append-system-prompt".to_string()));
    }

    #[test]
    fn a_discussion_has_its_tools_that_write_taken_away_in_every_mode() {
        let refused = "--disallowedTools=Edit,Write,MultiEdit,NotebookEdit,Bash".to_string();
        for mode in ["default", "plan", "acceptEdits", "bypassPermissions"] {
            let m = AgentMeta {
                mode: mode.into(),
                append_prompt: Some("Tu es l’agent rédacteur de spec.".into()),
                discussion: Some(Default::default()),
                ..Default::default()
            };
            let a = claude_args(&m);
            assert!(a.contains(&refused), "{mode}: {a:?}");
            // Before the instructions, which are not read as a tool.
            let at = |x: &str| a.iter().position(|y| y == x).unwrap();
            assert!(at(&refused) < at("--append-system-prompt"));
        }
        // Another agent keeps them.
        assert!(!claude_args(&AgentMeta::default())
            .iter()
            .any(|x| x.starts_with("--disallowedTools")));
    }

    #[test]
    fn the_log_line_of_a_start_does_not_copy_the_protocol() {
        let m = AgentMeta {
            model: "opus".into(),
            effort: "max".into(),
            mode: "plan".into(),
            session_id: Some("s1".into()),
            append_prompt: Some("Protocole « secret » du ticket ATL-42".into()),
            ..Default::default()
        };
        let line = args_for_log(&claude_args(&m));
        assert!(
            !line.contains("secret") && !line.contains("ATL-42"),
            "{line}"
        );
        // The flag stays, its value is only counted; the arguments around it are untouched.
        assert!(
            line.contains("--append-system-prompt <37 chars> --resume=s1"),
            "{line}"
        );
        assert!(line.contains("--model opus --effort max --permission-mode plan"));
        // Without a protocol, the line is the arguments as they are.
        let plain = claude_args(&AgentMeta::default());
        assert_eq!(args_for_log(&plain), plain.join(" "));
        // A flag ending the list has no value to elide.
        assert_eq!(
            args_for_log(&["--append-system-prompt".to_string()]),
            "--append-system-prompt"
        );
    }

    /// The command line of a `.cmd` launcher (npm's claude.cmd) is cmd.exe's, which takes 8191
    /// characters in all: every argument counts as Rust escapes it (`escaped_len`) between
    /// its quotes, with a space before it, behind the launcher's path and cmd.exe's own prefix.
    fn command_line_weight(program: &str, args: &[String]) -> usize {
        const CMD_PREFIX: &str = "C:\\Windows\\System32\\cmd.exe /e:ON /v:OFF /d /c \"";
        CMD_PREFIX.len()
            + program.len()
            + 2
            + args.iter().map(|a| 1 + 2 + escaped_len(a)).sum::<usize>()
    }

    #[test]
    fn a_ticket_agents_whole_command_line_fits_cmds_limit_in_the_worst_case() {
        // Criteria and a title made of what a `.cmd` argument escapes the most (`%`, `"`, `\`),
        // accents, and every kind of line break; far more of them than fit.
        let nasty = "%\"\\é\r\n%\"\\\\\"à\n".repeat(40);
        let t = Ticket {
            id: "t1".into(),
            project_id: "p1".into(),
            key: "ATL-42".into(),
            title: nasty.clone(),
            criteria: (0..60)
                .map(|_| Criterion {
                    text: nasty.clone(),
                    ..Default::default()
                })
                .collect(),
            max_loops: 5,
            ..Default::default()
        };
        // A long launcher path, as npm puts it under a user profile with a long name.
        let program = "C:\\Users\\guillaume.gagnaire-lefebvre\\AppData\\Roaming\\npm\\claude.cmd";
        for ports in [None, Some(4100)] {
            let m = AgentMeta {
                model: "claude-opus-4-5-20251101[1m]".into(),
                effort: "max".into(),
                mode: "bypassPermissions".into(),
                session_id: Some("0f8fad5b-d9cb-469f-a165-70867728950e".into()),
                append_prompt: Some(crate::board::protocol_prompt(&t, ports)),
                ..Default::default()
            };
            let args = claude_args(&m);
            assert!(
                args.contains(&"--append-system-prompt".to_string()),
                "{ports:?}"
            );
            assert!(
                args.iter().all(|a| !a.contains('\n') && !a.contains('\r')),
                "{ports:?}: a line break in an argument"
            );
            let weight = command_line_weight(program, &args);
            assert!(weight < 8191, "{ports:?}: {weight} of 8191");
            // The criteria are cut where they stop fitting, which leaves some of the protocol's
            // budget unused: with all of it used, the line must still fit.
            let protocol = m.append_prompt.as_deref().unwrap();
            let unused = PROTOCOL_BUDGET - escaped_len(protocol);
            println!(
                "worst-case command line, ports {ports:?}: {weight} of 8191, {} with the whole protocol budget used",
                weight + unused
            );
            assert!(
                weight + unused < 8191,
                "{ports:?}: {} of 8191 with the whole protocol budget used",
                weight + unused
            );
        }
    }

    fn att(name: &str, media_type: &str, data: &str) -> Attachment {
        Attachment {
            name: name.into(),
            media_type: media_type.into(),
            data: data.into(),
        }
    }

    #[test]
    fn a_message_without_attachments_is_plain_text() {
        assert_eq!(user_content("Bonjour", &[]).unwrap(), json!("Bonjour"));
    }

    #[test]
    fn attachments_come_before_the_text_as_content_blocks() {
        let c = user_content(
            "Résume",
            &[
                att("capture.png", "image/png", "iVBO"),
                att("rapport.pdf", "application/pdf", "JVBE"),
                att("notes.md", "text/plain", "# Notes\nà faire"),
            ],
        )
        .unwrap();
        assert_eq!(
            c,
            json!([
                { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "iVBO" } },
                { "type": "document", "title": "rapport.pdf",
                  "source": { "type": "base64", "media_type": "application/pdf", "data": "JVBE" } },
                { "type": "document", "title": "notes.md",
                  "source": { "type": "text", "media_type": "text/plain", "data": "# Notes\nà faire" } },
                { "type": "text", "text": "Résume" },
            ])
        );
    }

    #[test]
    fn a_file_alone_is_sent_without_an_empty_text_block() {
        let c = user_content("", &[att("a.pdf", "application/pdf", "JVBE")]).unwrap();
        assert_eq!(c.as_array().unwrap().len(), 1);
    }

    #[test]
    fn unsupported_or_oversized_attachments_are_refused() {
        let e = user_content("x", &[att("plan.docx", "application/msword", "UEsD")]).unwrap_err();
        assert!(e.to_string().contains("plan.docx"), "{e}");
        // The API reads only these image formats.
        assert!(user_content("x", &[att("a.bmp", "image/bmp", "Qk0=")]).is_err());
        let big = "A".repeat(7 * 1024 * 1024);
        let e = user_content("x", &[att("photo.png", "image/png", &big)]).unwrap_err();
        assert!(e.to_string().contains("5 Mo"), "{e}");
    }

    #[test]
    fn text_pdf_and_whole_message_sizes_fit_what_claude_takes() {
        // Past ~256 KB, a text file alone would fill Claude's context.
        let ok = "a".repeat(256 * 1024);
        assert!(user_content("x", &[att("log.txt", "text/plain", &ok)]).is_ok());
        let e = user_content("x", &[att("log.txt", "text/plain", &(ok + "a"))]).unwrap_err();
        assert!(e.to_string().contains("256 Ko"), "{e}");
        // A PDF of 18 MB fits in the whole message's limit.
        let pdf = "A".repeat(19 * MB / 3 * 4);
        let e = user_content("x", &[att("a.pdf", "application/pdf", &pdf)]).unwrap_err();
        assert!(e.to_string().contains("18 Mo"), "{e}");
        let pdf = "A".repeat(18 * MB / 3 * 4);
        assert!(user_content("x", &[att("a.pdf", "application/pdf", &pdf)]).is_ok());
        // Together, the files stay under the API's request size.
        let part = "A".repeat(10 * MB);
        let three = ["a.pdf", "b.pdf", "c.pdf"].map(|n| att(n, "application/pdf", &part));
        let e = user_content("x", &three).unwrap_err();
        assert!(e.to_string().contains("18 Mo en tout"), "{e}");
        assert!(user_content("x", &three[..2]).is_ok());
    }

    #[test]
    fn base64_padding_is_not_counted_as_data() {
        // Exactly 5 MB: 5 MB ≡ 2 (mod 3), so its base64 ends with one '='.
        let n = 5 * MB;
        let b64 = format!("{}=", "A".repeat((n / 3 + 1) * 4 - 1));
        assert!(user_content("x", &[att("p.png", "image/png", &b64)]).is_ok());
    }
}
