//! The in-process adapter of a harness (INT-9, `docs/PLUGINS.md` §4.5 and §4.6): `ProcessEngine: Engine` and
//! `ProcessEngineProcess: EngineProcess` speak the contract of `escouade_plugin_protocol::engine` to the plugin's
//! process, so that the rest of the core does not see the difference (the conformance bench plays them as it plays
//! the harnesses written in the core).
//!
//! - **The launch stays the core's.** The plugin describes it (`engine/launch`): arguments, variables, files of the
//!   agent's own, first lines. The core checks the description (`check_launch`: the fence of private projects applied
//!   and every refused path in it, read-only held, Escouade's token never among the arguments, files only in the
//!   agent's own folder), then starts the program it found itself (`locate`) in its Job object, under WSL when need
//!   be. The harness outlives a crash of its plugin.
//! - **The frames go in batches**, as raw lines the core never reads again: what was read while the last batch
//!   was with the plugin, up to 64 KiB, a lone line at once (`agent/frames`). The plugin answers normalized items and effects, which the core applies
//!   to the agent (`AgentRt::apply_items`, through `Engine::translate`); its `write` items, and the `writes` of its
//!   other answers, go to the harness's input, in their order.
//! - **A crash in the middle of a turn**: the runtime starts the plugin again (three times within five minutes, PLG-2)
//!   and gives it the agent again (`agent/attach`); the batch it had not answered is given again, the frames read
//!   meanwhile wait in their queue, and the conversation says « Le plugin … a redémarré ». What was being translated
//!   may be lost: a call under way stays so until the turn's end, where the core closes it. After the last crash the
//!   plugin is stopped: the agent's harness is stopped too, the conversation and its end say why, and its session
//!   is kept for the next message once the plugin is started again.
//! - **The terminal view** keeps its PTY in the core: the plugin describes its launch (`terminal/launch`, checked
//!   as the agent's) and reads its hooks (`terminal/hook`), whose answer it puts in its harness's form (the code
//!   index's context, a refusal).

// The harnesses' plugins are registered from their manifests as each harness comes out in process (INT-10); the
// tests play this adapter on the demonstration plugin until then.
#![allow(dead_code)]

use super::runtime::{CallError, Runtime, Stop};
use super::shared::{self, Shared};
use crate::agent::{AgentRt, Effects};
use crate::core::Attachment;
use crate::engine::registry::{
    AccountStore, CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind,
    Tested, ToldModel,
};
use crate::engine::{
    mcp, terminal, BoxFuture, Engine, EngineProcess, Escouade, Message, OnExit, OnFrame, OneShot,
    PermissionAnswer, Process, Program, Ready, Remote, Room, Spawn, Start, StartLost, Weight,
};
use crate::job::{Job, JobUsage};
use crate::model::EngineId;
use anyhow::{anyhow, bail, Context, Result};
use escouade_plugin_protocol::engine::{self as proto, method};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, watch};

/// How many lines wait for the plugin at most while it starts again; past them the oldest are dropped, and said.
pub const FRAME_QUEUE: usize = 20_000;

/// How long a batch of frames waits for its answer.
const FRAMES_WITHIN: Duration = Duration::from_secs(10);

/// How long the first call of a launch waits: it may start the plugin's process (its `initialize` has 10 s), on a
/// machine under load.
const ATTACH_WITHIN: Duration = Duration::from_secs(30);

/// How long a harness may take to tell its quota.
const USAGE_WITHIN: Duration = Duration::from_secs(60);

/// How long a compaction may take: a call to the model that summarizes the whole conversation.
const COMPACT_WITHIN: Duration = Duration::from_secs(600);

/// How long an agent's session may take to open on a shared harness (`agent/open`): the harness's start, then requests.
const OPEN_WITHIN: Duration = Duration::from_secs(150);

/// How long a request of the plugin to its harness may take (`agent/letGo`, `agent/rename`, `agent/compact`).
const REQUEST_WITHIN: Duration = Duration::from_secs(60);

/// How long a harness may take to answer what it was asked (a change of model, effort or mode).
const ANSWER_WITHIN: Duration = Duration::from_secs(15);

/// How long a harness may take to take a title.
const RENAME_WITHIN: Duration = Duration::from_secs(10);

/// How often an agent on a shared harness looks at its plugin: no call is under way to tell it stopped.
const WATCH_EVERY: Duration = Duration::from_millis(250);

/// How long a harness whose plugin opens its session may take: each request its opening makes had a minute.
const SESSION_WITHIN: Duration = Duration::from_secs(180);

/// How many times a call is given again to a plugin that crashed before it answered.
const RETRIES: usize = 8;

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

/// The harnesses a manifest declares (`contributes.harnesses`), those that read.
pub fn harnesses_of(contributes: &serde_json::Map<String, Value>) -> Vec<proto::HarnessDecl> {
    contributes
        .get("harnesses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|h| match serde_json::from_value(h.clone()) {
            Ok(d) => Some(d),
            Err(e) => {
                log::warn!("a harness of a plugin's manifest left aside: {e}");
                None
            }
        })
        .collect()
}

/// Registers the harnesses of the plugin `plugin` (its manifest's `contributes`), each one once: answers their ids.
pub fn register(
    rt: &Arc<Runtime>,
    plugin: &str,
    contributes: &serde_json::Map<String, Value>,
    escouade_dir: &Path,
) -> Vec<EngineId> {
    harnesses_of(contributes)
        .into_iter()
        .map(|decl| {
            let engine: &'static ProcessEngine = Box::leak(Box::new(ProcessEngine::new(
                rt.clone(),
                plugin,
                decl,
                escouade_dir,
            )));
            crate::engine::registry::register(engine);
            engine.id
        })
        .collect()
}

/// The harnesses the manifests of the plugins on describe (`acp`), in the registry: by id, their plugin and their
/// description, so that one changed is made again and one gone is taken out.
static DESCRIBED: Mutex<Vec<(EngineId, String, proto::HarnessDecl)>> = Mutex::new(Vec::new());

/// The harnesses described by the plugins `plugins` (each its id and its manifest's `contributes`), served by the
/// generic ACP plugin when it is on (`acp_on`): those not yet in the registry are put there, those whose plugin is
/// gone, off or changed are taken out (their agents « harness absent », their data kept) or made again. Answers the
/// ids in the registry.
pub fn sync_described(
    rt: &Arc<Runtime>,
    acp_on: bool,
    plugins: &[(String, serde_json::Map<String, Value>)],
    escouade_dir: &Path,
) -> Vec<EngineId> {
    let wanted: Vec<(EngineId, String, proto::HarnessDecl)> = if acp_on {
        plugins
            .iter()
            .flat_map(|(id, contributes)| {
                harnesses_of(contributes)
                    .into_iter()
                    .filter(|d| d.acp.is_some())
                    .map(move |d| (described_id(id, &d.id), id.clone(), d))
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut have = DESCRIBED.lock();
    have.retain(|(id, owner, decl)| {
        let keep = wanted
            .iter()
            .any(|(i, o, d)| i == id && o == owner && d == decl);
        if !keep {
            crate::engine::registry::unregister(*id);
        }
        keep
    });
    for (id, owner, decl) in wanted {
        if have.iter().any(|(i, _, _)| *i == id) {
            continue;
        }
        // A built-in harness keeps its place: a plugin does not take its id.
        if crate::engine::registry::BUILT_IN
            .iter()
            .any(|e| e.id() == id)
        {
            log::warn!(
                "plugin {owner}: its harness {id} has the id of a harness of the app, left aside"
            );
            continue;
        }
        let engine: &'static ProcessEngine = Box::leak(Box::new(ProcessEngine::described(
            rt.clone(),
            &owner,
            decl.clone(),
            escouade_dir,
        )));
        crate::engine::registry::register(engine);
        have.push((id, owner, decl));
    }
    have.iter().map(|(id, _, _)| *id).collect()
}

/// `sync_described` from the plugins installed: those that loaded and are on for the app.
pub fn sync(rt: &Arc<Runtime>, plugins: &super::Plugins) -> Vec<EngineId> {
    let on = |v: &super::PluginView| v.state == super::State::Ready && v.choices.enabled;
    let list = plugins.list(crate::i18n::Lang::En);
    let acp_on = list.iter().any(|v| v.id == super::builtin::ACP && on(v));
    *UNINSTALLED.lock() = BUILTIN_HARNESSES
        .iter()
        .map(|h| h.plugin)
        .filter(|p| plugins.uninstalled(p))
        .collect();
    for h in BUILTIN_HARNESSES {
        let plugin_on = list.iter().any(|v| v.id == h.plugin && on(v));
        sync_builtin(rt, h, plugin_on, plugins.app_dir());
    }
    // The harnesses of Escouade's own plugins keep their place in the registry (above): only the others are described.
    let described: Vec<(String, serde_json::Map<String, Value>)> = list
        .iter()
        .filter(|v| on(v) && !BUILTIN_HARNESSES.iter().any(|h| h.plugin == v.id))
        .filter_map(|v| Some((v.id.clone(), v.manifest.as_ref()?.contributes.clone())))
        .collect();
    sync_described(rt, acp_on, &described, plugins.app_dir())
}

/// What the core knows of a launch when it prepares it (`Prepare`).
pub struct Preparing<'a> {
    /// The agent's own folder (the variable of `HarnessDecl::account_var`), when it has one.
    pub home: Option<&'a Path>,
    /// The agent's folder.
    pub cwd: &'a str,
    /// The fence, as its program spells it.
    pub fence: &'a [String],
    /// A discussion, which changes nothing.
    pub read_only: bool,
    /// The variables its process is given by the core (its account's).
    pub env: &'a [(String, String)],
}

/// What the core writes in the agent's own folder before its harness is launched, and the arguments that adds to the
/// launch: Mistral Vibe's trust of the folder and the refusals of a private project in its `config.toml`, which the
/// rules its « Toujours » wrote there keep beside them; Gemini CLI's settings and its rules, given by `--policy`.
pub type Prepare = fn(&Preparing) -> Result<Vec<String>>;

/// A harness of a plugin the app gives (INT-10), behind the core's engine of the same id: the engine keeps its place
/// in the registry (its accounts, its models, its texts, its terminal view), its agents' conversation goes through the
/// plugin's `ProcessEngine`, and an agent started while the plugin is off is told so (`spawn_plugged`).
pub struct BuiltinHarness {
    /// The plugin, by its id.
    pub plugin: &'static str,
    /// How the launcher of its program is run (`ProcessEngine::with_native`).
    pub native: Option<Native>,
    /// What the core writes in the agent's folder before the launch (`ProcessEngine::with_prepare`).
    pub prepare: Option<Prepare>,
}

/// The harnesses that left the core: a line each.
pub const BUILTIN_HARNESSES: &[BuiltinHarness] = &[
    // Cursor CLI (INT-10.1): its installer's launcher run as its `node` and `index.js`.
    BuiltinHarness {
        plugin: super::builtin::CURSOR,
        native: Some(crate::engine::cursor::native),
        prepare: None,
    },
    // Mistral Vibe (INT-10.6, in the plugin Mistral): the trust and the `denylist` of a private project in its `VIBE_HOME`.
    BuiltinHarness {
        plugin: super::builtin::MISTRAL,
        native: None,
        prepare: Some(crate::engine::vibe::prepare),
    },
    // Gemini CLI (INT-10.7): its settings and its rules in its `GEMINI_CLI_HOME`, `--policy` added to its launch.
    BuiltinHarness {
        plugin: super::builtin::GEMINI,
        native: None,
        prepare: Some(crate::engine::gemini::prepare),
    },
    // OpenCode (INT-10.2): one `opencode serve` for the agents of an account (`Launch::shared`).
    BuiltinHarness {
        plugin: super::builtin::OPENCODE,
        native: None,
        prepare: None,
    },
    // Copilot CLI (INT-10.3): no ACP harness, nothing to prepare; its framing and its close are in its manifest.
    BuiltinHarness {
        plugin: super::builtin::COPILOT,
        native: None,
        prepare: None,
    },
    // Codex (INT-10.4): no ACP harness, nothing to prepare; its plugin opens its thread (`opens`).
    BuiltinHarness {
        plugin: super::builtin::CODEX,
        native: None,
        prepare: None,
    },
    // Antigravity CLI (INT-10.8): no ACP harness, several programs in a row (`relaunches`), the hooks of its view taken away.
    BuiltinHarness {
        plugin: super::builtin::ANTIGRAVITY,
        native: None,
        prepare: Some(crate::engine::agy::prepare),
    },
    // Claude Code (INT-10.5): nothing to prepare; its plugin asks its first request (`opens`), and its fence is a settings
    // file the core writes.
    BuiltinHarness {
        plugin: super::builtin::CLAUDE,
        native: None,
        prepare: None,
    },
];

/// The harnesses of the plugins the app gives that are on, by their plugin, each made once for each runtime (by its
/// address: a test makes several).
/// The plugins of the core's harnesses the user uninstalled (`Plugins::uninstalled`): an agent of theirs says so, and that the
/// view proposes them. As `sync` left it.
static UNINSTALLED: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

static PLUGGED: Mutex<Vec<(&'static str, usize, &'static ProcessEngine, bool)>> =
    Mutex::new(Vec::new());

/// The harness of `h` put behind its engine's agents when its plugin is on (`on`), taken away when it is not: its
/// agents are then told so at their start (`spawn_plugged`).
pub fn sync_builtin(rt: &Arc<Runtime>, h: &BuiltinHarness, on: bool, escouade_dir: &Path) {
    let key = Arc::as_ptr(rt) as usize;
    let mut plugged = PLUGGED.lock();
    // One in force for each plugin: the runtime synced last's (a test's, the app's).
    for p in plugged.iter_mut().filter(|p| p.0 == h.plugin) {
        p.3 = on && p.1 == key;
    }
    if !on || plugged.iter().any(|p| p.0 == h.plugin && p.1 == key) {
        return;
    }
    let Some(decl) = super::builtin::manifest_text(h.plugin, "")
        .and_then(|m| serde_json::from_str::<Value>(&m).ok())
        .and_then(|m| {
            serde_json::from_value::<proto::HarnessDecl>(m["contributes"]["harnesses"][0].clone())
                .ok()
        })
    else {
        log::warn!("the plugin {} declares no harness that reads", h.plugin);
        return;
    };
    let mut engine = ProcessEngine::new(rt.clone(), h.plugin, decl, escouade_dir);
    if let Some(native) = h.native {
        engine = engine.with_native(native);
    }
    if let Some(prepare) = h.prepare {
        engine = engine.with_prepare(prepare);
    }
    let e: &'static ProcessEngine = Box::leak(Box::new(engine));
    plugged.push((h.plugin, key, e, true));
}

/// The harness of the plugin `plugin` the app gives, if it is on.
pub fn plugged(plugin: &str) -> Option<&'static ProcessEngine> {
    PLUGGED
        .lock()
        .iter()
        .rev()
        .find(|(p, _, _, on)| *p == plugin && *on)
        .map(|(_, _, e, _)| *e)
}

/// Starts an agent of the engine `name` through the harness of its plugin `plugin`; told when the plugin is off or not
/// loaded (its session kept for the next message).
pub fn spawn_plugged(
    plugin: &str,
    name: &str,
    spawn: Spawn,
    on_frame: OnFrame,
    on_exit: OnExit,
) -> Result<Process> {
    let Some(engine) = plugged(plugin) else {
        // The plugin may be called otherwise than its harness: Mistral, for Mistral Vibe.
        let called = super::builtin::manifest_text(plugin, "")
            .and_then(|m| serde_json::from_str::<Value>(&m).ok())
            .and_then(|m| m["name"].as_str().map(str::to_string))
            .unwrap_or_else(|| name.to_string());
        if UNINSTALLED.lock().contains(&plugin) {
            bail!(tr!(
                "Le plugin {called} ({plugin}) n’est plus installé : la vue Plugins propose de l’installer, pour lancer un agent de {name}.",
                "The {called} plugin ({plugin}) is no longer installed: the Plugins view offers to install it, to start a {name} agent."
            ));
        }
        bail!(tr!(
            "Le plugin {called} ({plugin}) est coupé ou n’est pas chargé : rallume-le dans la vue Plugins pour lancer un agent de {name}.",
            "The {called} plugin ({plugin}) is switched off or not loaded: turn it back on in the Plugins view to start a {name} agent."
        ));
    };
    engine.spawn(spawn, on_frame, on_exit)
}

/// A harness whose plugin runs in its own process.
pub struct ProcessEngine {
    rt: Arc<Runtime>,
    /// The plugin that serves it: its own, or `dev.escouade.acp` for a harness a manifest describes.
    plugin: String,
    /// The plugin whose manifest declares it.
    owner: String,
    /// It is described (`acp`), served by the generic ACP plugin: the description goes with each agent.
    described: bool,
    install: Install,
    update_command: &'static str,
    writer: &'static str,
    decl: proto::HarnessDecl,
    id: EngineId,
    name: &'static str,
    speaker: &'static str,
    caps: EngineCaps,
    models: Vec<ModelDecl>,
    efforts: &'static [&'static str],
    version_args: &'static [&'static str],
    tested: Tested,
    account_var: &'static str,
    /// The variable of an account's key (`HarnessDecl::key_var`).
    key_var: Option<&'static str>,
    escouade_dir: PathBuf,
    /// How long after its input is closed its harness is killed, if it does not end by itself (`close_grace_ms`).
    close_grace: Option<Duration>,
    /// The program its harness's launcher runs, and its first arguments (Cursor CLI's `node` and `index.js`: the
    /// launcher its installer leaves is a script), when the launcher is one.
    native: Option<Native>,
    /// What the core writes in the agent's folder before the launch (`with_prepare`).
    prepare: Option<Prepare>,
}

/// What a harness's launcher runs: the program and its first arguments, if it is a launcher it knows.
pub type Native = fn(&Program) -> Option<(PathBuf, Vec<String>)>;

impl ProcessEngine {
    /// The harness `decl` of the plugin `plugin`; `escouade_dir` is the folder of the app's programs (`{escouade}`).
    pub fn new(
        rt: Arc<Runtime>,
        plugin: &str,
        decl: proto::HarnessDecl,
        escouade_dir: &Path,
    ) -> Self {
        let c = &decl.caps;
        let caps = EngineCaps {
            modes: c
                .modes
                .iter()
                .map(|m| ModeCap {
                    id: leak(&m.id),
                    meaning: m.meaning.as_ref().map(|t| t.get(crate::i18n::ui().tag())),
                })
                .collect(),
            effort: !decl.efforts.is_empty(),
            images: c.images,
            pdf: c.pdf,
            fork: ForkPoint::None,
            resume_other_account: false,
            mid_turn: MidTurn::Refused,
            slash_commands: c.slash_commands,
            quota: QuotaKind::None,
            quota_windows: Vec::new(),
            cost: CostKind::None,
            // Its view's PTY stays in the core: wired with the first harness in process that has one (INT-10.1).
            terminal_view: false,
            remote_control: false,
            background_tasks: false,
            todo_tools: false,
            default_refusal: false,
            sandbox: false,
            usage_page: None,
            accounts_by_key: c.accounts_by_key,
            read_only: c.read_only,
        };
        let models = decl
            .models
            .iter()
            .map(|m| ModelDecl {
                id: leak(&m.id),
                label: leak(m.label.as_deref().unwrap_or(&m.id)),
                effort: m.effort,
                auto: false,
            })
            .collect();
        let efforts: Vec<&'static str> = decl.efforts.iter().map(|e| leak(e)).collect();
        let version_args: Vec<&'static str> = decl.version_args.iter().map(|a| leak(a)).collect();
        let install = decl.install.clone().unwrap_or_default();
        Self {
            rt,
            plugin: plugin.to_string(),
            owner: plugin.to_string(),
            described: false,
            install: Install {
                windows: leak(&install.windows),
                unix: leak(&install.unix),
            },
            update_command: leak(decl.update_command.as_deref().unwrap_or_default()),
            writer: leak(decl.writer.as_deref().unwrap_or(&decl.name)),
            id: EngineId::new(&decl.id),
            name: leak(&decl.name),
            speaker: leak(decl.speaker.as_deref().unwrap_or(&decl.name)),
            caps,
            models,
            efforts: Box::leak(efforts.into_boxed_slice()),
            version_args: Box::leak(version_args.into_boxed_slice()),
            tested: Tested {
                oldest: leak(&decl.tested.oldest),
                newest: leak(&decl.tested.newest),
            },
            account_var: leak(
                decl.account_var
                    .as_deref()
                    .unwrap_or("ESCOUADE_HARNESS_HOME"),
            ),
            key_var: decl.key_var.as_deref().map(leak),
            close_grace: decl.close_grace_ms.map(Duration::from_millis),
            escouade_dir: escouade_dir.to_path_buf(),
            native: None,
            prepare: None,
            decl,
        }
    }

    /// The harness `decl` the plugin `owner` describes (`acp`, PLG-6), served by the generic ACP plugin: its id is
    /// `plugin:<owner>:<harness>` (the bare one for a plugin of Escouade's own, whose harness keeps the id it had in
    /// the core), and its program is looked for by its names alone unless its plugin is Escouade's (a declarative
    /// plugin names no path to run).
    pub fn described(
        rt: Arc<Runtime>,
        owner: &str,
        mut decl: proto::HarnessDecl,
        escouade_dir: &Path,
    ) -> Self {
        let first = owner.starts_with("dev.escouade.");
        if !first {
            decl.locate.program = None;
        }
        let id = described_id(owner, &decl.id);
        let mut e = Self::new(rt, super::builtin::ACP, decl, escouade_dir);
        e.owner = owner.to_string();
        e.described = true;
        e.id = id;
        e
    }

    /// The same, its launcher run as `native` says (Cursor CLI's, INT-10.1).
    pub fn with_native(mut self, native: Native) -> Self {
        self.native = Some(native);
        self
    }

    /// The same, what `prepare` writes in the agent's folder done before each launch (Mistral Vibe's, INT-10.6).
    pub fn with_prepare(mut self, prepare: Prepare) -> Self {
        self.prepare = Some(prepare);
        self
    }

    /// The plugin it is served by.
    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    /// The plugin whose manifest declares it.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// What the plugin is told of the harness with each agent: its description, for the generic ACP plugin.
    fn description(&self) -> Option<proto::HarnessDecl> {
        self.described.then(|| self.decl.clone())
    }

    async fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: impl Serialize,
    ) -> Result<T> {
        let params = serde_json::to_value(params)?;
        let v = self
            .rt
            .call(&self.plugin, method, params)
            .await
            .map_err(|e| down(self.name, e))?;
        serde_json::from_value(v).map_err(|e| anyhow!("{}: {method}: {e}", self.name))
    }

    /// The launch of its interface in a terminal (`terminal/launch`), checked as an agent's launch is.
    pub async fn terminal_launch(&self, params: proto::TerminalParams) -> Result<proto::Launch> {
        let launch: proto::Launch = self.call(method::TERMINAL_LAUNCH, &params).await?;
        check_launch(
            self.name,
            super::builtin::is_builtin(&self.plugin),
            params.escouade.as_ref().map(|g| g.token.as_str()),
            &params.fence,
            params.read_only,
            params.home.as_deref(),
            &launch,
        )
        .map_err(|e| anyhow!(e))?;
        Ok(launch)
    }

    /// One of its interface's hooks, with what the core answers it (`reply`): the items for the agent, and the answer
    /// in its harness's form.
    pub async fn terminal_hook(
        &self,
        agent: &str,
        hook: Value,
        reply: Option<proto::HookReply>,
    ) -> Result<proto::HookAnswer> {
        let h = proto::Hook {
            agent: agent.to_string(),
            hook,
            reply,
        };
        self.call(method::TERMINAL_HOOK, &h).await
    }

    fn program_at(&self, path: &str) -> Option<PathBuf> {
        let path = match path.strip_prefix("{escouade}/") {
            Some(rest) => {
                let p = self.escouade_dir.join(rest);
                // Only the app's own program, in the folder of the app.
                super::builtin::check_program(&p, &self.escouade_dir).ok()?;
                p
            }
            None => PathBuf::from(path),
        };
        with_exe(&path).filter(|p| p.is_file())
    }
}

/// The id of the harness `harness` the plugin `owner` describes.
pub fn described_id(owner: &str, harness: &str) -> EngineId {
    if owner.starts_with("dev.escouade.") {
        EngineId::new(harness)
    } else {
        EngineId::new(&format!("plugin:{owner}:{harness}"))
    }
}

/// `path`, or under Windows the same with `.exe` (or `.cmd`, the launcher npm leaves) when that is a file.
fn with_exe(path: &Path) -> Option<PathBuf> {
    // Under Windows the `.exe` or `.cmd` first: a file without extension beside them is a shell script, which
    // Windows does not run.
    if cfg!(windows) && path.extension().is_none() {
        for ext in ["exe", "cmd"] {
            let p = path.with_extension(ext);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    path.is_file().then(|| path.to_path_buf())
}

/// What a call that got no answer is in the core.
fn down(name: &str, e: CallError) -> anyhow::Error {
    match e {
        CallError::Remote { message, .. } => anyhow!("{name} : {message}"),
        e => anyhow!(tr!(
            "Le plugin de {name} ne répond pas : {e}",
            "The plugin of {name} is not answering: {e}"
        )),
    }
}

/// The checks of a launch's description (`engine/launch`, `terminal/launch`): what the core guarantees whatever the
/// plugin. Answers why it is refused. `builtin`: the plugin is one the app gives, the only one whose word that it holds
/// the fence itself (`fence_by_client`, the generic ACP plugin) is taken; any other's launch must spell the fence.
#[allow(clippy::too_many_arguments)]
pub fn check_launch(
    name: &str,
    builtin: bool,
    token: Option<&str>,
    fence: &[String],
    read_only: bool,
    home: Option<&str>,
    launch: &proto::Launch,
) -> std::result::Result<(), String> {
    if let Some(token) = token.filter(|t| !t.is_empty()) {
        if launch.args.iter().any(|a| a.contains(token)) {
            return Err(tr!(
                "Le plugin de {name} met le jeton d’Escouade dans les arguments de son harness, que tout le monde peut lire : l’agent ne démarre pas.",
                "The plugin of {name} puts Escouade’s token among its harness’s arguments, which anyone can read: the agent doesn’t start."
            ));
        }
    }
    if !fence.is_empty() {
        // Whatever its slashes: Codex reads a path of Windows with `/`, a file in JSON doubles its `\`.
        let plain = |s: &str| s.replace('\\', "/").replace("//", "/");
        let said = |p: &str| {
            let p = plain(p);
            let p = p.trim_end_matches('/');
            launch.args.iter().any(|a| plain(a).contains(p))
                || launch.env.iter().any(|v| plain(&v.value).contains(p))
                || launch.files.iter().any(|f| plain(&f.text).contains(p))
        };
        // Held by the plugin on what its harness reads and writes through it (ACP's `fs/*`), or by the launch, every
        // path in it.
        let held = (builtin && launch.fence_by_client)
            || (launch.fence_applied && fence.iter().all(|p| said(p)));
        if !held {
            return Err(tr!(
                "Le plugin de {name} n’applique pas la clôture des projets privés : l’agent ne démarre pas.",
                "The plugin of {name} doesn’t apply the fence of private projects: the agent doesn’t start."
            ));
        }
    }
    if read_only && !launch.read_only {
        return Err(tr!(
            "Le plugin de {name} ne tient pas son harness en lecture seule : la discussion ne démarre pas.",
            "The plugin of {name} doesn’t hold its harness read-only: the discussion doesn’t start."
        ));
    }
    for f in &launch.files {
        check_file(name, home, &f.path)?;
    }
    Ok(())
}

/// A file a plugin wants written: in the agent's own folder alone.
fn check_file(name: &str, home: Option<&str>, file: &str) -> std::result::Result<(), String> {
    let path = Path::new(file);
    let inside = home.is_some_and(|home| {
        path.is_absolute()
            && path.starts_with(home)
            && !path.components().any(|c| matches!(c, Component::ParentDir))
    });
    if inside {
        return Ok(());
    }
    Err(tr!(
        "Le plugin de {name} veut écrire {file}, hors du dossier de l’agent : l’agent ne démarre pas.",
        "The plugin of {name} wants to write {file}, outside the agent’s folder: the agent doesn’t start."
    ))
}

/// The files of a launch, written.
fn write_files(launch: &proto::Launch) -> Result<()> {
    for f in &launch.files {
        let path = Path::new(&f.path);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, &f.text)?;
    }
    Ok(())
}

/// One message of a stream framed by `Content-Length` headers (`HarnessDecl::framing`), read from `r`: None once it ends.
pub(crate) async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(
    r: &mut R,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut len: Option<usize> = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).await? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if let Some(n) = len {
                let mut body = vec![0; n];
                r.read_exact(&mut body).await?;
                return Ok(Some(body));
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                len = v.trim().parse().ok();
            }
        }
    }
}

/// `body` framed by its `Content-Length` header.
pub(crate) fn framed(body: &[u8]) -> Vec<u8> {
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(body);
    out
}

/// The arguments as the log shows them: a prompt among them (up to several KB of ticket text, `--append-system-prompt`) is
/// only counted, not copied.
pub(crate) fn args_for_log(args: &[String]) -> String {
    let mut shown: Vec<String> = Vec::with_capacity(args.len());
    let mut elide = false;
    for a in args {
        if std::mem::take(&mut elide) {
            shown.push(format!("<{} chars>", a.chars().count()));
            continue;
        }
        elide = a.starts_with("--") && a.ends_with("-prompt");
        shown.push(a.clone());
    }
    shown.join(" ")
}

/// The variables of `base` with those of `more` over them.
fn merged(base: &[(String, String)], more: &[proto::EnvVar]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = base
        .iter()
        .filter(|(k, _)| !more.iter().any(|v| &v.name == k))
        .cloned()
        .collect();
    out.extend(more.iter().map(|v| (v.name.clone(), v.value.clone())));
    out
}

pub(crate) fn command_of(
    program: &Program,
    cwd: &str,
    args: &[String],
    env: &[(String, String)],
    remove: &[String],
) -> Result<tokio::process::Command> {
    let launch = program.launch(cwd, args, env)?;
    let mut cmd = tokio::process::Command::new(&launch.program);
    cmd.args(&launch.args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (k, v) in &launch.env {
        cmd.env(k, v);
    }
    for k in remove {
        cmd.env_remove(k);
    }
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    crate::job::isolate(&mut cmd);
    crate::proc::no_inherited_fds(&mut cmd);
    Ok(cmd)
}

impl Engine for ProcessEngine {
    fn id(&self) -> EngineId {
        self.id
    }
    fn name(&self) -> &'static str {
        self.name
    }
    fn speaker(&self) -> &'static str {
        self.speaker
    }
    fn instructions_room(&self) -> Option<Room> {
        None
    }
    fn subagent_relay(&self, _: crate::i18n::Lang, _: &str, _: &str, _: &str) -> Option<String> {
        None
    }

    fn locate(&self, configured: &str, _wsl_distro: &str) -> Option<Program> {
        if !configured.trim().is_empty() {
            return with_exe(Path::new(configured.trim())).map(Program::Native);
        }
        if let Some(p) = self.decl.locate.program.as_deref() {
            return self.program_at(p).map(Program::Native);
        }
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .flat_map(|dir| self.decl.locate.names.iter().map(move |n| dir.join(n)))
            .find_map(|p| with_exe(&p))
            .map(Program::Native)
    }

    fn tested(&self) -> Tested {
        self.tested
    }
    fn oldest_reason(&self, lang: crate::i18n::Lang) -> String {
        let (name, oldest) = (self.name, self.tested.oldest);
        tr_in!(
            lang,
            "{name} est plus ancien que la version {oldest}, la plus ancienne que son plugin tient.",
            "{name} is older than version {oldest}, the oldest its plugin holds."
        )
    }
    fn update_command(&self) -> &'static str {
        self.update_command
    }
    fn version_args(&self) -> &'static [&'static str] {
        self.version_args
    }
    fn caps(&self) -> EngineCaps {
        let mut caps = self.caps.clone();
        // The meanings in the app's language of now.
        let lang = crate::i18n::ui().tag();
        for (m, d) in caps.modes.iter_mut().zip(&self.decl.caps.modes) {
            m.meaning = d.meaning.as_ref().map(|t| t.get(lang));
        }
        caps
    }
    fn accounts(&self) -> AccountStore {
        AccountStore::ConfigDir {
            var: self.account_var,
        }
    }
    fn terminal(&self) -> Option<&'static dyn terminal::TerminalView> {
        None
    }
    fn install(&self) -> Install {
        self.install
    }
    fn models(&self) -> Vec<ModelDecl> {
        self.models.clone()
    }
    fn efforts(&self) -> &'static [&'static str] {
        self.efforts
    }
    fn supports_effort(&self, model: &str) -> bool {
        self.models.iter().any(|m| m.id == model && m.effort)
    }
    fn supports_auto(&self, _: &str) -> bool {
        false
    }

    fn spawn(&self, spawn: Spawn, on_frame: OnFrame, on_exit: OnExit) -> Result<Process> {
        let key = format!("{}-{}", self.id, uuid::Uuid::new_v4().simple());
        let session = match &spawn.start {
            Start::Resume { session } => Some(session.clone()),
            _ => None,
        };
        let attach = proto::Attach {
            agent: key.clone(),
            cwd: spawn.cwd.clone(),
            session,
            model: spawn.model.clone(),
            effort: spawn.effort.clone(),
            mode: spawn.mode.clone(),
            pending: Vec::new(),
            described: self.description(),
            lang: crate::i18n::ui().tag().to_string(),
            ..proto::Attach::default()
        };
        let (stdin_tx, stdin_rx) = mpsc::unbounded_channel();
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        let (ready_tx, _) = watch::channel(None);
        let p = Arc::new(ProcessEngineProcess {
            rt: self.rt.clone(),
            key_var: self.key_var,
            close_grace: self.close_grace,
            framed: self.decl.framing.as_deref() == Some("contentLength"),
            close_line: self.decl.close_line.clone(),
            opens: self.decl.opens,
            relaunches: self.decl.relaunches,
            leaves_out: self.decl.stderr_leaves_out.clone(),
            next: Mutex::new(None),
            paused: AtomicBool::new(false),
            opened: watch::channel(None).0,
            refused: Mutex::new(None),
            compacting: Mutex::new(None),
            reading: Mutex::new(None),
            me: std::sync::OnceLock::new(),
            native: self.native,
            prepare: self.prepare,
            plugin: self.plugin.clone(),
            name: self.name,
            account_var: self.account_var,
            key,
            attach: Mutex::new(attach),
            placed: Mutex::new(None),
            stdin: Mutex::new(Some(stdin_tx)),
            control: Mutex::new(Some(control_tx)),
            ready: ready_tx,
            alive: AtomicBool::new(true),
            job: Mutex::new(None),
            kill_tx: Mutex::new(None),
            tail: Mutex::new(VecDeque::new()),
            shared: Mutex::new(None),
            left: AtomicBool::new(false),
            left_tx: watch::channel(false).0,
            closing: AtomicBool::new(false),
            let_go: Arc::new(AtomicBool::new(true)),
            told: Mutex::new(None),
            pushed: Mutex::new(None),
            joins: Mutex::new(None),
            catalog: Mutex::new(None),
            first_said: Mutex::new(None),
            asks: Mutex::new(std::collections::HashMap::new()),
        });
        let _ = p.me.set(Arc::downgrade(&p));
        tokio::spawn(control(Arc::downgrade(&p), p.ready.subscribe(), control_rx));
        tokio::spawn(p.clone().run(spawn, stdin_rx, on_frame, on_exit));
        Ok(p)
    }

    fn message(&self, text: &str, attachments: &[Attachment]) -> Result<Message> {
        if !attachments.is_empty() {
            let name = self.name;
            bail!(tr!(
                "{name} ne prend pas de fichier joint.",
                "{name} takes no attached file."
            ));
        }
        Ok(Message(json!(text)))
    }

    fn translate(&self, rt: &mut AgentRt, frame: &Value, fx: &mut Effects) {
        if let Some(items) = frame["items"].as_array() {
            rt.apply_items(items, fx);
        }
    }

    fn start_lost(&self, _stderr: &str) -> Option<StartLost> {
        None
    }

    fn one_shot<'a>(&'a self, ask: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let cwd = ask.cwd.to_string_lossy().into_owned();
            let params = proto::OneShotParams {
                weight: match ask.weight {
                    Weight::Light => "light",
                    Weight::Reading => "reading",
                }
                .into(),
                cwd: cwd.clone(),
                system: ask.system.into(),
                prompt: ask.prompt.into(),
                model: ask.model.clone(),
            };
            let launch: proto::OneShotLaunch = self.call(method::ONE_SHOT, &params).await?;
            let env = merged(&ask.env, &launch.env);
            let mut cmd = command_of(&ask.program, &cwd, &launch.args, &env, &launch.remove_env)?;
            if launch.input.is_none() {
                cmd.stdin(Stdio::null());
            }
            let mut child = cmd
                .spawn()
                .with_context(|| format!("{}: {}", self.name, ask.program.describe()))?;
            let _job = Job::for_child(&child);
            if let (Some(input), Some(mut stdin)) = (launch.input, child.stdin.take()) {
                stdin.write_all(input.as_bytes()).await?;
            }
            let out = tokio::time::timeout(ask.limit, crate::proc::wait_output(child))
                .await
                .map_err(|_| {
                    anyhow!(
                        "{}: still not done after {} s",
                        self.name,
                        ask.limit.as_secs()
                    )
                })??;
            let read = proto::OneShotOutput {
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
                code: out.status.code(),
            };
            let text: proto::OneShotText = self.call(method::ONE_SHOT_READ, &read).await?;
            Ok(text.text)
        })
    }

    fn writer(&self, _: Weight) -> &'static str {
        self.writer
    }
    fn grant_file(&self, _: &str, _: &str) -> Option<Vec<u8>> {
        None
    }
    fn refusal(&self) -> Option<Value> {
        None
    }
    /// The description of its launch carries Escouade's server (`engine/launch`).
    fn escouade_launch(&self, _: &Escouade) -> mcp::Launch {
        mcp::Launch {
            args: Vec::new(),
            env: Vec::new(),
        }
    }
    fn memory_dir(&self, config_dir: &Path, project_path: &str) -> Option<PathBuf> {
        let dir = &self.decl.memory.as_ref()?.dir;
        let key: String = project_path
            .trim_end_matches(['/', '\\'])
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let dir = dir
            .replace("{configDir}", &config_dir.to_string_lossy())
            .replace("{projectKey}", &key);
        Some(PathBuf::from(dir))
    }
    fn session_home(&self) -> Option<crate::fence::HomeKind> {
        None
    }
}

/// A call the control task makes, in order, and who waits for it: its answer, its harness's when it asks it (`proto::Asked`,
/// within `within`).
struct Control {
    method: &'static str,
    params: Value,
    done: Option<oneshot::Sender<Result<Value>>>,
    within: Duration,
}

/// The live process of an agent whose harness's plugin runs apart.
pub struct ProcessEngineProcess {
    rt: Arc<Runtime>,
    /// As `ProcessEngine::close_grace`.
    close_grace: Option<Duration>,
    /// Its messages go with `Content-Length` headers (`HarnessDecl::framing`), not by lines.
    framed: bool,
    /// As `HarnessDecl::close_line`.
    close_line: Option<String>,
    /// As `HarnessDecl::opens`: it is ready at `Item::Ready`.
    opens: bool,
    /// As `HarnessDecl::relaunches`.
    relaunches: bool,
    /// As `HarnessDecl::stderr_leaves_out`.
    leaves_out: Vec<String>,
    /// Who starts its next program (`run`).
    next: Mutex<Option<mpsc::UnboundedSender<Next>>>,
    /// No program runs for now (`Item::Pause`).
    paused: AtomicBool,
    /// What `Item::Ready` said (the models of the account), or why it will not come.
    opened: watch::Sender<Option<std::result::Result<Value, String>>>,
    /// Why its plugin stopped its harness (`Item::Stop`).
    refused: Mutex<Option<String>>,
    /// Who waits for the end of a compaction (`Item::Compacted`), and for the quota it asked (`Item::Usage`).
    compacting: Mutex<Option<oneshot::Sender<Result<bool>>>>,
    reading: Mutex<Option<oneshot::Sender<Value>>>,
    /// Itself, for what it does later (the kill after its input is closed).
    me: std::sync::OnceLock<std::sync::Weak<Self>>,
    /// The variable of an account's key (`HarnessDecl::key_var`).
    key_var: Option<&'static str>,
    /// What its harness's launcher runs (`ProcessEngine::with_native`).
    native: Option<Native>,
    /// What the core writes in the agent's folder before the launch (`ProcessEngine::with_prepare`).
    prepare: Option<Prepare>,
    plugin: String,
    name: &'static str,
    /// The variable that gives the agent its own folder (`HarnessDecl::account_var`): where its launch may write.
    account_var: &'static str,
    /// The agent's process, as `agent/*` names it.
    key: String,
    /// What the plugin is given again after a restart: the session, the model, the mode, the requests waiting.
    attach: Mutex<proto::Attach>,
    /// Where the agent lives, as the core resolved it (`fs/*`, `proc/*`).
    placed: Mutex<Option<super::files::AgentPlace>>,
    stdin: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    control: Mutex<Option<mpsc::UnboundedSender<Control>>>,
    /// The end of its launch: Ok once its harness runs.
    ready: watch::Sender<Option<std::result::Result<(), String>>>,
    alive: AtomicBool,
    job: Mutex<Option<Arc<Job>>>,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    /// The end of what its harness wrote on its stderr.
    tail: Mutex<VecDeque<String>>,
    /// The process it shares with other agents (`Launch::shared`).
    shared: Mutex<Option<Arc<Shared>>>,
    /// It left that process.
    left: AtomicBool,
    left_tx: watch::Sender<bool>,
    /// Stopped before it was on that process: it leaves once it is.
    closing: AtomicBool,
    /// Its folder let go by that process, when it was its last agent there (or no need).
    let_go: Arc<AtomicBool>,
    /// Where what its plugin says unasked goes (`agent/items`).
    told: Mutex<Option<Arc<OnFrame>>>,
    /// The way to its `pump`, while its own program runs: what the plugin says unasked comes behind its batches.
    pushed: Mutex<Option<mpsc::Sender<Heard>>>,
    /// How its harness's interface joins it, as `agent/open` said it.
    joins: Mutex<Option<terminal::Attach>>,
    /// The models of its account, as `agent/open` said them.
    catalog: Mutex<Option<Vec<ToldModel>>>,
    /// What its harness said when it started (`Item::Told`).
    first_said: Mutex<Option<(Value, Value)>>,
    /// Who waits for its harness's answer to a call (`proto::Asked`), by its `ask`.
    asks: Mutex<
        std::collections::HashMap<String, oneshot::Sender<std::result::Result<Value, String>>>,
    >,
}

/// What is asked of its programs (`Item::Relaunch`, `Item::Pause`).
enum Next {
    Launch(Box<proto::Launch>, mpsc::UnboundedReceiver<Vec<u8>>),
    Pause,
}

/// What its harness's reader gives the pump: a line, or the end of its program by itself (told to the plugin behind the
/// lines it wrote), answered by whether another starts; and what the plugin said unasked (`agent/items`).
enum Heard {
    Line(String),
    Items(Vec<Value>),
    Gone {
        code: Option<i32>,
        again: oneshot::Sender<bool>,
    },
}

/// What a launch came to.
enum Launched {
    Child(tokio::process::Child, Option<Arc<Job>>),
    /// A process shared by agents, from this description.
    Shared(proto::Launch),
}

/// The agents of a plugin, by their key: what it says unasked comes to them.
static TOLD: Mutex<Vec<(String, std::sync::Weak<ProcessEngineProcess>)>> = Mutex::new(Vec::new());

/// What a plugin said of its agent without being asked (`agent/items`).
fn told(params: &Value) {
    let Some(agent) = params["agent"].as_str() else {
        return;
    };
    let p = TOLD
        .lock()
        .iter()
        .find(|(k, _)| k == agent)
        .and_then(|(_, p)| p.upgrade());
    let (Some(p), Some(items)) = (p, params["items"].as_array()) else {
        return;
    };
    // Behind the answer of the batch before it, for an agent whose program is its own (docs/PLUGINS.md §8.4.6).
    if let Some(lines) = p.pushed.lock().as_ref() {
        if lines.try_send(Heard::Items(items.clone())).is_err() {
            log::warn!("{agent}: what its plugin said left aside: its queue is full");
        }
        return;
    }
    let on_frame = p.told.lock().clone();
    if let Some(on_frame) = on_frame {
        p.deliver(items.clone(), &on_frame);
    }
}

/// Where its conversation starts from, in the contract's words.
pub(crate) fn start_at(start: &Start) -> proto::StartAt {
    match start {
        Start::New => proto::StartAt::New,
        Start::Resume { session } => proto::StartAt::Resume {
            session: session.clone(),
        },
        Start::Fork { of, at } => proto::StartAt::Fork {
            of: of.clone(),
            at: at.clone(),
        },
    }
}

/// Escouade's server for it, in the contract's words.
fn grant_of(escouade: &Escouade) -> Option<proto::Grant> {
    match escouade {
        Escouade::Granted { url, token, .. } => Some(proto::Grant {
            url: url.clone(),
            token: token.clone(),
        }),
        Escouade::Denied { .. } => None,
    }
}

/// The calls of the control task, one after the other: their `writes` go to the harness in their order.
async fn control(
    p: std::sync::Weak<ProcessEngineProcess>,
    mut launched: watch::Receiver<Option<std::result::Result<(), String>>>,
    mut rx: mpsc::UnboundedReceiver<Control>,
) {
    // Once its plugin was given the agent, however soon a message is sent.
    let _ = launched.wait_for(Option::is_some).await;
    while let Some(c) = rx.recv().await {
        let Some(p) = p.upgrade() else { break };
        let result = p.call_writing(c.method, c.params).await;
        let Some(done) = c.done else {
            if let Err(e) = result {
                log::warn!("{}: {e:#}", p.key);
            }
            continue;
        };
        match result {
            // Its harness tells the end: the next call does not wait for it.
            Ok(Called::Asked(rx)) => {
                let within = c.within;
                tokio::spawn(async move {
                    let _ = done.send(answer_of(rx, within).await);
                });
            }
            Ok(Called::Over(v)) => {
                let _ = done.send(Ok(v));
            }
            Err(e) => {
                let _ = done.send(Err(e));
            }
        }
    }
}

/// What a call came to.
enum Called {
    /// Its answer.
    Over(Value),
    /// Its harness's to tell.
    Asked(oneshot::Receiver<std::result::Result<Value, String>>),
}

/// What its harness says of a call, within `within`.
async fn answer_of(
    rx: oneshot::Receiver<std::result::Result<Value, String>>,
    within: Duration,
) -> Result<Value> {
    match tokio::time::timeout(within, rx).await {
        Ok(Ok(Ok(v))) => Ok(v),
        Ok(Ok(Err(e))) => Err(anyhow!(e)),
        Ok(Err(_)) => Err(anyhow!("process exited")),
        Err(_) => Err(anyhow!("control request timed out")),
    }
}

/// The folder of the account of an agent on the default account, which has no variable of its own: where the harness
/// looks without it, under the folder `home`. For the harnesses that read something of it (`agentHome`).
pub(crate) fn default_home_in(var: &str, home: &Path) -> Option<PathBuf> {
    let folder = match var {
        "CODEX_HOME" => ".codex",
        "COPILOT_HOME" => ".copilot",
        "CURSOR_CONFIG_DIR" => ".cursor",
        // Its variable names the folder that holds `.gemini`; this one is all an agent needs of the user's home.
        "GEMINI_CLI_HOME" => ".gemini",
        _ => return None,
    };
    Some(home.join(folder))
}

/// See `default_home_in`: the variable as the user has it, else under their home. Never the machine user's in the tests.
fn default_home(var: &str) -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    std::env::var_os(var)
        .filter(|v| var != "GEMINI_CLI_HOME" && !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| default_home_in(var, &dirs::home_dir()?))
}

impl ProcessEngineProcess {
    /// Writes `line` on its harness's input.
    fn write(&self, line: &str) {
        let bytes = if self.framed {
            framed(line.as_bytes())
        } else {
            let mut bytes = Vec::with_capacity(line.len() + 1);
            bytes.extend_from_slice(line.as_bytes());
            bytes.push(b'\n');
            bytes
        };
        if let Some(tx) = self.stdin.lock().as_ref() {
            let _ = tx.send(bytes);
        }
    }

    fn attach_params(&self) -> Value {
        // Each time: a plugin started again from nothing (its folder reloaded) knows the agent from here.
        if let Some(place) = self.placed.lock().clone() {
            if let Ok(roots) = self.rt.place_agent(&self.plugin, &self.key, place) {
                self.attach.lock().roots = roots;
            }
        }
        serde_json::to_value(&*self.attach.lock()).unwrap_or_default()
    }

    /// Gives the agent again to a plugin that started again; Err when it is stopped.
    async fn reattach(&self) -> std::result::Result<(), CallError> {
        match self
            .rt
            .attach(
                &self.plugin,
                &self.key,
                method::ATTACH,
                self.attach_params(),
            )
            .await
        {
            Err(e @ (CallError::Stopped(_) | CallError::Unavailable(_))) => Err(e),
            _ => Ok(()),
        }
    }

    /// Its plugin knows the agent: one forgotten since (the language or the network changed, and the plugins the app gives
    /// start anew) is given it again before it is asked anything; one switched off stops it.
    async fn known(&self) -> std::result::Result<(), CallError> {
        match self.rt.status(&self.plugin).phase {
            super::runtime::PhaseName::Idle => self.reattach().await,
            _ => Ok(()),
        }
    }

    /// `writes` to its harness, what it will answer waited for when they ask it (`ask`).
    fn asking(&self, v: Value) -> Called {
        let asked: proto::Asked = serde_json::from_value(v.clone()).unwrap_or_default();
        let waiting = (!asked.ask.is_empty()).then(|| {
            let (tx, rx) = oneshot::channel();
            self.asks.lock().insert(asked.ask.clone(), tx);
            rx
        });
        for w in &asked.writes {
            self.write(w);
        }
        match waiting {
            Some(rx) => Called::Asked(rx),
            None => Called::Over(v),
        }
    }

    /// Calls `method`, given again after a crash; writes its `writes` to the harness.
    async fn call_writing(&self, method: &str, params: Value) -> Result<Called> {
        self.known().await.map_err(|e| down(self.name, e))?;
        for _ in 0..RETRIES {
            match self.rt.call(&self.plugin, method, params.clone()).await {
                Ok(v) => {
                    let items = v["items"].as_array().cloned().unwrap_or_default();
                    let called = self.asking(v);
                    let told = self.told.lock().clone();
                    if let (false, Some(on_frame)) = (items.is_empty(), told) {
                        self.deliver(items, &on_frame);
                    }
                    return Ok(called);
                }
                Err(CallError::Crashed | CallError::Dropped) => {
                    self.reattach().await.map_err(|e| down(self.name, e))?;
                }
                Err(e) => return Err(down(self.name, e)),
            }
        }
        Err(down(self.name, CallError::Crashed))
    }

    /// Asks the control task to call `method`, in order; `wait`: until it is done.
    async fn ordered(
        &self,
        method: &'static str,
        params: impl Serialize,
        wait: bool,
    ) -> Result<()> {
        if !wait {
            return self.send_ordered(method, serde_json::to_value(params)?);
        }
        self.ordered_within(method, params, ANSWER_WITHIN)
            .await
            .map(drop)
    }

    /// `ordered`, until it is done, its answer: its harness's within `within` when it is its to tell.
    async fn ordered_within(
        &self,
        method: &'static str,
        params: impl Serialize,
        within: Duration,
    ) -> Result<Value> {
        let params = serde_json::to_value(params)?;
        let (tx, rx) = oneshot::channel();
        let sent = self.control.lock().as_ref().map(|c| {
            c.send(Control {
                method,
                params,
                done: Some(tx),
                within,
            })
            .is_ok()
        });
        if sent != Some(true) {
            bail!("{} is gone", self.name);
        }
        rx.await.map_err(|_| anyhow!("{} is gone", self.name))?
    }

    fn agent(&self) -> String {
        self.key.clone()
    }

    /// Tells the plugin's host where the agent lives (`fs/*`, `proc/*`): what the core resolved, which the plugin cannot
    /// change; `agent/attach` tells it the roots it declares.
    fn place(&self, spawn: &Spawn) {
        let cwd = PathBuf::from(&spawn.cwd);
        *self.placed.lock() = Some(super::files::AgentPlace {
            repo: super::files::repo_of(&cwd),
            home: spawn
                .env
                .iter()
                .find(|(k, _)| k == self.account_var)
                .map(|(_, v)| PathBuf::from(v))
                .or_else(|| default_home(self.account_var)),
            cwd,
            fence: spawn.fence.clone(),
            project: spawn.project.clone(),
        });
    }

    /// Launches its harness from the plugin's description, checked: its child. The first call may start the plugin's
    /// process: it is given the time of a start (`ATTACH_WITHIN`), not that of a call.
    async fn launch(&self, spawn: &Spawn) -> Result<Launched> {
        self.place(spawn);
        self.rt
            .attach_within(
                &self.plugin,
                &self.key,
                method::ATTACH,
                self.attach_params(),
                ATTACH_WITHIN,
            )
            .await
            .map_err(|e| down(self.name, e))?;
        let fence = crate::fence::spelled(&spawn.fence, &spawn.program);
        let token = spawn.escouade.token().map(str::to_string);
        let home = spawn
            .env
            .iter()
            .find(|(k, _)| k == self.account_var)
            .map(|(_, v)| v.clone());
        let params = proto::LaunchParams {
            agent: self.key.clone(),
            cwd: spawn.cwd.clone(),
            model: spawn.model.clone(),
            effort: spawn.effort.clone(),
            mode: spawn.mode.clone(),
            append_prompt: spawn.append_prompt.clone(),
            relay: spawn.relay.clone(),
            by_key: self
                .key_var
                .is_some_and(|var| spawn.env.iter().any(|(k, v)| k == var && !v.is_empty())),
            start: start_at(&spawn.start),
            read_dirs: spawn
                .read_dirs
                .iter()
                .map(|d| d.to_string_lossy().into_owned())
                .collect(),
            escouade: grant_of(&spawn.escouade),
            escouade_file: match &spawn.escouade {
                Escouade::Granted { file, .. } => {
                    file.as_ref().map(|f| f.to_string_lossy().into_owned())
                }
                Escouade::Denied { .. } => None,
            },
            fence: fence.clone(),
            read_only: spawn.read_only,
            program: spawn.program.describe(),
            home: home.clone(),
            described: self.attach.lock().described.clone(),
            lang: crate::i18n::ui().tag().to_string(),
            takes_effort: spawn.takes_effort,
            settings: spawn
                .settings
                .as_ref()
                .map(|f| f.to_string_lossy().into_owned()),
            refusal_file: match &spawn.escouade {
                Escouade::Denied { file } => {
                    file.as_ref().map(|f| f.to_string_lossy().into_owned())
                }
                Escouade::Granted { .. } => None,
            },
        };
        let settings = params.settings.clone();
        let v = self
            .rt
            .call(&self.plugin, method::LAUNCH, serde_json::to_value(&params)?)
            .await
            .map_err(|e| down(self.name, e))?;
        let launch: proto::Launch = serde_json::from_value(v)?;
        // The settings file the core wrote holds the fence: a launch that names it holds the fence too.
        let in_settings = launch.fence_applied
            && settings
                .as_ref()
                .is_some_and(|file| launch.args.iter().any(|a| a == file));
        check_launch(
            self.name,
            super::builtin::is_builtin(&self.plugin),
            token.as_deref(),
            if in_settings { &[] } else { &fence },
            spawn.read_only,
            home.as_deref(),
            &launch,
        )
        .map_err(|e| anyhow!(e))?;
        {
            let mut a = self.attach.lock();
            a.home = home.clone();
            a.read_dirs = params.read_dirs.clone();
            a.by_key = params.by_key;
            a.fence = fence.clone();
            a.read_only = spawn.read_only;
            a.append_prompt = spawn.append_prompt.clone();
        }
        let added = match self.prepare {
            Some(prepare) => prepare(&Preparing {
                home: home.as_deref().map(Path::new),
                cwd: &spawn.cwd,
                fence: &fence,
                read_only: spawn.read_only,
                env: &spawn.env,
            })?,
            None => Vec::new(),
        };
        write_files(&launch)?;
        if launch.shared.is_some() {
            return Ok(Launched::Shared(launch));
        }
        let (child, job) = self.start_program(spawn, &launch, &added)?;
        for w in &launch.writes {
            self.write(w);
        }
        Ok(Launched::Child(child, job))
    }

    /// Starts its program from `launch`, `added` to its arguments.
    fn start_program(
        &self,
        spawn: &Spawn,
        launch: &proto::Launch,
        added: &[String],
    ) -> Result<(tokio::process::Child, Option<Arc<Job>>)> {
        log::info!(
            "{}: starting {} ({}) {} in {} (account {})",
            spawn.label,
            spawn.program.describe(),
            self.name,
            args_for_log(&launch.args),
            spawn.cwd,
            spawn.account
        );
        let env = merged(&spawn.env, &launch.env);
        let told = launch.args.iter().chain(added).cloned();
        let (program, args) = match self.native.and_then(|n| n(&spawn.program)) {
            Some((program, mut first)) => {
                first.extend(told);
                (Program::Native(program), first)
            }
            None => (spawn.program.clone(), told.collect()),
        };
        let mut cmd = command_of(&program, &spawn.cwd, &args, &env, &launch.remove_env)?;
        let child = cmd.spawn().with_context(|| {
            let program = spawn.program.describe();
            tr!("impossible de lancer {program}", "can’t start {program}")
        })?;
        let job = Job::for_child(&child).map(Arc::new);
        Ok((child, job))
    }

    /// Starts the next of its programs, its description checked as the first one's was.
    fn start_again(
        &self,
        spawn: &Spawn,
        launch: &proto::Launch,
    ) -> Result<(tokio::process::Child, Option<Arc<Job>>)> {
        let fence = crate::fence::spelled(&spawn.fence, &spawn.program);
        let home = self.attach.lock().home.clone();
        check_launch(
            self.name,
            super::builtin::is_builtin(&self.plugin),
            spawn.escouade.token(),
            &fence,
            spawn.read_only,
            home.as_deref(),
            launch,
        )
        .map_err(|e| anyhow!(e))?;
        write_files(launch)?;
        self.start_program(spawn, launch, &[])
    }

    /// What its plugin says of it unasked (`agent/items`) comes to it.
    fn hear_what_it_tells(&self) {
        TOLD.lock()
            .push((self.key.clone(), self.me.get().cloned().unwrap_or_default()));
        self.rt.on_notification(&self.plugin, |m, params| {
            if m == method::ITEMS {
                told(&params);
            }
        });
    }

    /// Its launch failed: why.
    fn failed(&self, e: &anyhow::Error, on_exit: OnExit) {
        let why = format!("{e:#}");
        self.alive.store(false, Ordering::Release);
        self.control.lock().take();
        let _ = self.ready.send(Some(Err(why.clone())));
        let _ = self.opened.send(Some(Err(why.clone())));
        self.rt.detach(&self.plugin, &self.key);
        on_exit(None, why);
    }

    /// On its shared process: started by its first agent, ready, then the agent's session opened on it by the plugin.
    async fn join(self: &Arc<Self>, spawn: &Spawn, launch: &proto::Launch) -> Result<()> {
        let how = launch.shared.clone().unwrap_or_default();
        let env = merged(&spawn.env, &launch.env);
        let key = shared::key_of(
            Arc::as_ptr(&self.rt) as usize,
            &self.plugin,
            &spawn.program,
            &spawn.env,
        );
        let dir = shared::folder(&self.plugin);
        let s = shared::acquire(&key, &spawn.cwd, how.one_agent_a_folder, |own| {
            log::info!(
                "{}: starting {} ({}) in {} (account {})",
                spawn.label,
                spawn.program.describe(),
                self.name,
                dir.display(),
                spawn.account
            );
            Shared::start(shared::Start {
                key: own,
                name: self.name,
                program: &spawn.program,
                args: &launch.args,
                env: &env,
                remove_env: &launch.remove_env,
                ready: &how.ready,
                listens: how.listens.as_deref(),
                keep: how.keep.clone(),
                dir: &dir,
            })
        })?;
        *self.shared.lock() = Some(s.clone());
        let line = s.ready(self.name).await?;
        self.hear_what_it_tells();
        {
            let mut a = self.attach.lock();
            a.shared = Some(proto::SharedRef {
                key: s.key().to_string(),
                line,
                keep: s.keep().clone(),
            });
            a.fence = crate::fence::spelled(&spawn.fence, &spawn.program);
            a.read_only = spawn.read_only;
            a.append_prompt = spawn.append_prompt.clone();
        }
        self.place(spawn);
        self.rt
            .attach_within(
                &self.plugin,
                &self.key,
                method::ATTACH,
                self.attach_params(),
                ATTACH_WITHIN,
            )
            .await
            .map_err(|e| down(self.name, e))?;
        let open = proto::Open {
            agent: self.key.clone(),
            start: start_at(&spawn.start),
            escouade: grant_of(&spawn.escouade),
        };
        let opened: proto::Opened = serde_json::from_value(
            self.rt
                .call_within(
                    &self.plugin,
                    method::OPEN,
                    serde_json::to_value(&open)?,
                    OPEN_WITHIN,
                )
                .await
                .map_err(|e| down(self.name, e))?,
        )?;
        if let Some(a) = opened.attach {
            *self.joins.lock() = Some(terminal::Attach {
                args: a.args,
                env: a.env.into_iter().map(|v| (v.name, v.value)).collect(),
            });
        }
        *self.catalog.lock() = opened.catalog.and_then(|c| serde_json::from_value(c).ok());
        Ok(())
    }

    /// The life of an agent on a shared process: the process's own end, its leaving, or its plugin stopped.
    async fn run_shared(
        self: Arc<Self>,
        spawn: Spawn,
        launch: proto::Launch,
        on_frame: Arc<OnFrame>,
        on_exit: OnExit,
    ) {
        if let Err(e) = self.join(&spawn, &launch).await {
            self.leave();
            TOLD.lock().retain(|(k, _)| *k != self.key);
            let _ = self
                .rt
                .notify(&self.plugin, method::DETACH, json!({ "agent": self.key }));
            self.failed(&e, on_exit);
            return;
        }
        let _ = self.ready.send(Some(Ok(())));
        let Some(s) = self.shared.lock().clone() else {
            return;
        };
        if self.closing.load(Ordering::Acquire) {
            self.leave();
        }
        let mut end = s.end();
        let mut ended = Box::pin(async move {
            let e = end.wait_for(Option::is_some).await.ok();
            e.and_then(|e| e.clone())
        });
        let mut left = self.left_tx.subscribe();
        let mut gone = Box::pin(async move {
            let _ = left.wait_for(|l| *l).await;
        });
        let name = self.name;
        let mut restarts = self.rt.status(&self.plugin).restarts;
        let mut watch = tokio::time::interval(WATCH_EVERY);
        // Its plugin switched off or stopped: why the agent stops, whichever of its ends comes first. The watch below
        // may see it after the agent's input was closed or its server ended (the CI's Linux, 2026-11-05: the agent
        // stopped without a word).
        let off = |me: &Self| {
            let status = me.rt.status(&me.plugin);
            let e = match status
                .stop
                .filter(|_| matches!(status.phase, super::runtime::PhaseName::Stopped))
            {
                Some(stop) => Some(CallError::Stopped(stop)),
                None => me.rt.refusal(&me.plugin),
            }?;
            let why = stop_told(name, &e)?;
            on_frame(json!({ "items": [{ "op": "notice", "level": "error", "text": why }] }));
            Some(why)
        };
        let (code, said) = loop {
            tokio::select! {
                end = &mut ended => {
                    self.left.store(true, Ordering::Release);
                    if let Some(why) = off(&self) {
                        break (None, why);
                    }
                    break end.unwrap_or_default();
                }
                () = &mut gone => {
                    if let Some(why) = off(&self) {
                        break (None, why);
                    }
                    break (Some(0), String::new());
                }
                _ = watch.tick() => {
                    let status = self.rt.status(&self.plugin);
                    if status.stop.is_some() && matches!(status.phase, super::runtime::PhaseName::Stopped) {
                        let why = off(&self);
                        self.leave();
                        break why.map_or((Some(0), String::new()), |why| (None, why));
                    }
                    // Forgotten (switched off, its permissions changed): started anew when it may be.
                    if matches!(status.phase, super::runtime::PhaseName::Idle) {
                        let again = self
                            .rt
                            .attach(&self.plugin, &self.key, method::ATTACH, self.attach_params())
                            .await;
                        if let Err(e @ (CallError::Unavailable(_) | CallError::Stopped(_))) = again {
                            let why = stop_told(name, &e);
                            if let Some(why) = &why {
                                on_frame(json!({ "items": [{ "op": "notice", "level": "error", "text": why }] }));
                            }
                            self.leave();
                            break why.map_or((Some(0), String::new()), |why| (None, why));
                        }
                    }
                    let running = matches!(status.phase, super::runtime::PhaseName::Running);
                    let again = running && status.restarts > restarts;
                    if running {
                        restarts = status.restarts;
                    }
                    if again {
                        // Attached first: whoever reads this acts at once, on a plugin that must know its turn.
                        let _ = self.reattach().await;
                        let text = tr!(
                            "Le plugin de {name} a redémarré.",
                            "The plugin of {name} started again."
                        );
                        on_frame(json!({ "items": [{ "op": "notice", "level": "warning", "text": text }] }));
                    }
                }
            }
        };
        self.alive.store(false, Ordering::Release);
        self.control.lock().take();
        on_exit(code, said);
        // The plugin keeps the agent until its folder is let go.
        while !self.let_go.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let _ = self
            .rt
            .notify(&self.plugin, method::DETACH, json!({ "agent": self.key }));
        self.rt.detach(&self.plugin, &self.key);
        TOLD.lock().retain(|(k, _)| *k != self.key);
    }

    /// It no longer uses its shared process: its folder let go when it was the last agent there, the process stopped
    /// with its last agent.
    fn leave(&self) {
        if self.left.swap(true, Ordering::AcqRel) {
            return;
        }
        let shared = self.shared.lock().clone();
        if let Some(s) = shared {
            let cwd = self.attach.lock().cwd.clone();
            if s.leave(&cwd) && s.is_alive() {
                if let Ok(rt) = tokio::runtime::Handle::try_current() {
                    self.let_go.store(false, Ordering::Release);
                    s.disposing(&cwd, false);
                    let (plugins, plugin, key) =
                        (self.rt.clone(), self.plugin.clone(), self.key.clone());
                    let (s, done) = (s.clone(), self.let_go.clone());
                    rt.spawn(async move {
                        let said = plugins
                            .call_within(
                                &plugin,
                                method::LET_GO,
                                json!({ "agent": key }),
                                REQUEST_WITHIN,
                            )
                            .await;
                        if let Err(e) = said {
                            log::debug!("{key}: its folder let go: {e}");
                        }
                        s.disposing(&cwd, true);
                        done.store(true, Ordering::Release);
                    });
                }
            }
            shared::release(&s);
        }
        let _ = self.left_tx.send(true);
    }

    /// The process's life: its launch, its frames, its end.
    async fn run(
        self: Arc<Self>,
        spawn: Spawn,
        stdin_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        on_frame: OnFrame,
        on_exit: OnExit,
    ) {
        let on_frame = Arc::new(on_frame);
        *self.told.lock() = Some(on_frame.clone());
        let (mut child, job) = match self.launch(&spawn).await {
            Ok(Launched::Child(child, job)) => (child, job),
            Ok(Launched::Shared(launch)) => {
                self.run_shared(spawn, launch, on_frame, on_exit).await;
                return;
            }
            Err(e) => {
                self.failed(&e, on_exit);
                return;
            }
        };
        let mut job = job;
        *self.job.lock() = job.clone();
        let (kill_tx, mut kill_rx) = oneshot::channel();
        *self.kill_tx.lock() = Some(kill_tx);
        let (next_tx, mut next_rx) = mpsc::unbounded_channel();
        *self.next.lock() = Some(next_tx);
        let (lines_tx, lines_rx) = mpsc::channel::<Heard>(FRAME_QUEUE);
        *self.pushed.lock() = Some(lines_tx.clone());
        self.hear_what_it_tells();
        let mut reader = Some(self.hook_up(&mut child, stdin_rx, lines_tx.clone()));
        let mut child = Some(child);
        let _ = self.ready.send(Some(Ok(())));
        let pump = {
            let (me, on_frame) = (self.clone(), on_frame.clone());
            tokio::spawn(async move { me.pump(lines_rx, &on_frame).await })
        };
        let code = loop {
            tokio::select! {
                status = async {
                    match child.as_mut() {
                        Some(c) => c.wait().await,
                        None => std::future::pending().await,
                    }
                } => {
                    let code = status.ok().and_then(|s| s.code());
                    child = None;
                    if let Some(r) = reader.take() {
                        let _ = r.await;
                    }
                    if !self.relaunches || self.closing.load(Ordering::Acquire) {
                        break code;
                    }
                    // Told behind what it wrote; its plugin may start another.
                    let (again, answer) = oneshot::channel();
                    if lines_tx.send(Heard::Gone { code, again }).await.is_err()
                        || !answer.await.unwrap_or(false)
                    {
                        break code;
                    }
                }
                _ = &mut kill_rx => {
                    let mut code = None;
                    if let Some(c) = child.as_mut() {
                        let _ = c.start_kill();
                        if let Some(job) = &job { job.terminate(); }
                        code = c.wait().await.ok().and_then(|s| s.code());
                    }
                    break code;
                }
                Some(next) = next_rx.recv() => {
                    if let Some(mut c) = child.take() {
                        let _ = c.start_kill();
                        if let Some(job) = &job { job.terminate(); }
                        let _ = c.wait().await;
                    }
                    if let Some(r) = reader.take() {
                        let _ = tokio::time::timeout(Duration::from_secs(2), r).await;
                    }
                    job = None;
                    *self.job.lock() = None;
                    let Next::Launch(launch, stdin) = next else { continue };
                    match self.start_again(&spawn, &launch) {
                        Ok((mut c, j)) => {
                            job = j;
                            *self.job.lock() = job.clone();
                            reader = Some(self.hook_up(&mut c, stdin, lines_tx.clone()));
                            child = Some(c);
                        }
                        // A message that was to go to it: its turn ends, said; the next one tries again.
                        Err(e) => self.not_started(&e),
                    }
                }
            }
        };
        self.pushed.lock().take();
        TOLD.lock().retain(|(k, _)| *k != self.key);
        drop(lines_tx);
        if let Some(r) = reader.take() {
            let _ = r.await;
        }
        let stopped = pump.await.ok().flatten();
        self.alive.store(false, Ordering::Release);
        self.control.lock().take();
        let _ = self
            .rt
            .notify(&self.plugin, method::DETACH, json!({ "agent": self.key }));
        self.rt.detach(&self.plugin, &self.key);
        let said = stopped
            .or_else(|| self.refused.lock().take())
            .unwrap_or_else(|| {
                self.tail
                    .lock()
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            });
        // Who still waits for it is told: it is gone.
        self.opened.send_if_modified(|o| {
            let none = o.is_none();
            if none {
                *o = Some(Err(if said.is_empty() {
                    "process exited".to_string()
                } else {
                    said.clone()
                }));
            }
            none
        });
        self.compacting.lock().take();
        self.reading.lock().take();
        // The exit is handled before what waits fails: who sees a call failed sees what the exit did too.
        on_exit(code, said);
        self.asks.lock().clear();
    }

    /// A line its harness wrote, kept for what its end says.
    fn said(&self, line: String) {
        let mut tail = self.tail.lock();
        tail.push_back(line);
        if tail.len() > 30 {
            tail.pop_front();
        }
    }

    /// Its program's input, error output and lines hooked up; answers the task that reads its lines.
    fn hook_up(
        self: &Arc<Self>,
        child: &mut tokio::process::Child,
        mut stdin_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        lines_tx: mpsc::Sender<Heard>,
    ) -> tokio::task::JoinHandle<()> {
        if let Some(mut stdin) = child.stdin.take() {
            tokio::spawn(async move {
                while let Some(bytes) = stdin_rx.recv().await {
                    if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
                        break;
                    }
                }
            });
        }
        if let Some(stderr) = child.stderr.take() {
            let me = Arc::downgrade(self);
            tokio::spawn(async move {
                // As bytes: a line that is not UTF-8 (a message of `wsl.exe`) must not stop the reading.
                let mut lines = BufReader::new(stderr).split(b'\n');
                while let Ok(Some(line)) = lines.next_segment().await {
                    let Some(p) = me.upgrade() else { break };
                    let line = String::from_utf8_lossy(&line).replace('\0', "");
                    if p.leaves_out.iter().any(|t| line.contains(t.as_str())) {
                        continue;
                    }
                    p.said(line.trim_end().to_string());
                }
            });
        }
        let stdout = child.stdout.take();
        let by_length = self.framed;
        tokio::spawn(async move {
            let Some(stdout) = stdout else { return };
            if by_length {
                let mut reader = BufReader::new(stdout);
                while let Ok(Some(body)) = read_frame(&mut reader).await {
                    let text = String::from_utf8_lossy(&body).into_owned();
                    if lines_tx.send(Heard::Line(text)).await.is_err() {
                        break;
                    }
                }
                return;
            }
            let mut lines = BufReader::new(stdout).split(b'\n');
            while let Ok(Some(bytes)) = lines.next_segment().await {
                let text = String::from_utf8_lossy(&bytes);
                let text = text.trim();
                if !text.is_empty() && lines_tx.send(Heard::Line(text.to_string())).await.is_err() {
                    break;
                }
            }
        })
    }

    /// A program that was asked for did not start: the conversation says why, and its plugin ends the turn it was for as a
    /// stopped one (the next message starts another).
    fn not_started(&self, e: &anyhow::Error) {
        let why = format!("{e:#}");
        log::warn!("{}: {why}", self.key);
        let told = self.told.lock().clone();
        if let Some(on_frame) = told {
            on_frame(json!({ "items": [{ "op": "notice", "level": "error", "text": why }] }));
        }
        let stop = json!({ "agent": self.key, "cancelQueued": true });
        if let Err(e) = self.send_ordered(method::INTERRUPT, stop) {
            log::warn!("{}: {e:#}", self.key);
        }
    }

    /// Its program ended by itself: what its plugin makes of it. Answers whether another starts.
    async fn program_gone(&self, code: Option<i32>, on_frame: &OnFrame) -> bool {
        let said: Vec<String> = self.tail.lock().iter().cloned().collect();
        let gone = proto::Gone {
            agent: self.key.clone(),
            code,
            said: said.join("\n"),
        };
        let Ok(params) = serde_json::to_value(&gone) else {
            return false;
        };
        let answer = self
            .rt
            .call_within(&self.plugin, method::PROGRAM_GONE, params, FRAMES_WITHIN)
            .await;
        let items = match answer {
            Ok(v) => v["items"].as_array().cloned().unwrap_or_default(),
            Err(e) => {
                log::warn!("{}: {e}", self.key);
                return false;
            }
        };
        let again = items.iter().any(|i| i["op"] == "relaunch");
        self.deliver(items, on_frame);
        again
    }

    /// Reads its harness's lines in batches and gives them to the plugin; answers why it stopped the harness, if it
    /// did (its plugin stopped).
    async fn pump(&self, mut lines: mpsc::Receiver<Heard>, on_frame: &OnFrame) -> Option<String> {
        let mut stopped = None;
        let mut held = None;
        loop {
            let heard = match held.take() {
                Some(h) => Some(h),
                None => lines.recv().await,
            };
            let first = match heard {
                Some(Heard::Line(line)) => line,
                Some(Heard::Items(items)) => {
                    if stopped.is_none() {
                        self.deliver(items, on_frame);
                    }
                    continue;
                }
                Some(Heard::Gone { code, again }) => {
                    let goes_on = stopped.is_none() && self.program_gone(code, on_frame).await;
                    let _ = again.send(goes_on);
                    continue;
                }
                None => break,
            };
            // What was read meanwhile goes with it, up to 64 KiB; nothing waits for more: under a fast stream the
            // lines gather while the last batch is with the plugin, and a lone line goes at once.
            let mut bytes = first.len();
            let mut batch = vec![first];
            while bytes < proto::BATCH_BYTES {
                match lines.try_recv() {
                    Ok(Heard::Line(l)) => {
                        bytes += l.len();
                        batch.push(l);
                    }
                    Ok(gone) => {
                        held = Some(gone);
                        break;
                    }
                    Err(_) => break,
                }
            }
            // Once its plugin is stopped, what its harness still says before it ends is left aside.
            if stopped.is_some() {
                continue;
            }
            if let Err(e) = self.translate_batch(batch, on_frame).await {
                let why = stop_told(self.name, &e);
                if let Some(why) = &why {
                    on_frame(
                        json!({ "items": [{ "op": "notice", "level": "error", "text": why }] }),
                    );
                }
                self.kill();
                stopped = Some(why.unwrap_or_default());
            }
        }
        stopped
    }

    /// One batch, given to the plugin until it answers (again after a crash); its items applied. Err: why the plugin
    /// is stopped.
    async fn translate_batch(
        &self,
        batch: Vec<String>,
        on_frame: &OnFrame,
    ) -> std::result::Result<(), CallError> {
        let params = serde_json::value::to_raw_value(&proto::Frames {
            agent: self.key.clone(),
            frames: batch,
        })
        .map_err(|e| CallError::Unavailable(e.to_string()))?;
        let name = self.name;
        let mut told = false;
        self.known().await?;
        for _ in 0..RETRIES {
            match self
                .rt
                .call_raw(&self.plugin, method::FRAMES, &params, FRAMES_WITHIN)
                .await
            {
                Ok(answer) => {
                    let items: Value = serde_json::from_str(answer.get()).unwrap_or_default();
                    self.deliver(
                        items["items"].as_array().cloned().unwrap_or_default(),
                        on_frame,
                    );
                    return Ok(());
                }
                Err(CallError::Crashed | CallError::Dropped) => {
                    if !told {
                        told = true;
                        let text = tr!(
                            "Le plugin de {name} a redémarré.",
                            "The plugin of {name} started again."
                        );
                        on_frame(
                            json!({ "items": [{ "op": "notice", "level": "warning", "text": text }] }),
                        );
                    }
                    self.reattach().await?;
                }
                Err(e @ CallError::Stopped(_)) => return Err(e),
                Err(e) => {
                    let n = params.get().len();
                    log::warn!("{}: a batch of {n} bytes left untranslated: {e}", self.key);
                    let text = tr!(
                        "Le plugin de {name} n’a pas traduit ce que son harness a dit ({e}).",
                        "The plugin of {name} didn’t translate what its harness said ({e})."
                    );
                    on_frame(
                        json!({ "items": [{ "op": "notice", "level": "warning", "text": text }] }),
                    );
                    return Ok(());
                }
            }
        }
        Err(CallError::Crashed)
    }

    /// The items of a batch: its writes to the harness, what the plugin is to be told again after a restart, the rest
    /// to the agent.
    fn deliver(&self, items: Vec<Value>, on_frame: &OnFrame) {
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item["op"].as_str().unwrap_or_default() {
                "write" => {
                    if let Some(line) = item["line"].as_str() {
                        self.write(line);
                    }
                    continue;
                }
                "closed" => {
                    self.leave();
                    continue;
                }
                "start" | "session" | "conversation" => {
                    let mut a = self.attach.lock();
                    if let Some(s) = item["session"].as_str().filter(|s| !s.is_empty()) {
                        a.session = Some(s.to_string());
                    }
                    a.running |= item["op"] == "start";
                }
                "permission" | "question" | "permissionCard" | "questionCard" | "ask" => {
                    let mut a = self.attach.lock();
                    if let Some(r) = item["request"].as_str() {
                        a.pending.push(r.to_string());
                    }
                    // Its harness's turn is over on this card: nothing to wait for from it.
                    a.running &= item["endsStream"] != true;
                }
                "turn" | "result" => {
                    let mut a = self.attach.lock();
                    a.pending.clear();
                    a.running = false;
                }
                "answered" => {
                    if let Some(r) = item["request"].as_str() {
                        self.answered(r);
                    }
                }
                // The harness left its mode: told again after a restart.
                "mode" => {
                    if let Some(mode) = item["mode"].as_str() {
                        self.attach.lock().mode = mode.to_string();
                    }
                }
                // What is for this process, not the agent.
                "relaunch" => {
                    match serde_json::from_value::<proto::Launch>(item["launch"].clone()) {
                        Ok(launch) => self.relaunch(launch),
                        Err(e) => log::warn!("{}: a relaunch that does not read: {e}", self.key),
                    }
                    continue;
                }
                "pause" => {
                    self.pause();
                    continue;
                }
                "file" => {
                    let (path, text) = (
                        item["path"].as_str().unwrap_or_default(),
                        item["text"].as_str().unwrap_or_default(),
                    );
                    self.write_file(path, text);
                    continue;
                }
                "ready" => {
                    let _ = self.opened.send(Some(Ok(item["models"].clone())));
                    continue;
                }
                "told" => {
                    *self.first_said.lock() =
                        Some((item["commands"].clone(), item["models"].clone()));
                    continue;
                }
                "answer" => {
                    if let Some(tx) = item["ask"]
                        .as_str()
                        .and_then(|a| self.asks.lock().remove(a))
                    {
                        let _ = tx.send(match item["error"].as_str() {
                            Some(e) => Err(e.to_string()),
                            None => Ok(item["answer"].clone()),
                        });
                    }
                    continue;
                }
                "said" => {
                    if let Some(line) = item["line"].as_str() {
                        self.said(line.to_string());
                    }
                    continue;
                }
                "usage" => {
                    if let Some(tx) = self.reading.lock().take() {
                        let _ = tx.send(item["usage"].clone());
                    }
                    continue;
                }
                "compacted" => {
                    if let Some(tx) = self.compacting.lock().take() {
                        let _ = tx.send(match item["error"].as_str() {
                            Some(e) => Err(anyhow!("{e}")),
                            None => Ok(true),
                        });
                    }
                    continue;
                }
                "stop" => {
                    let reason = item["reason"].as_str().unwrap_or_default().to_string();
                    let _ = self.opened.send(Some(Err(reason.clone())));
                    *self.refused.lock() = Some(reason);
                    self.kill();
                    continue;
                }
                _ => {}
            }
            // A card and the end of a turn have their effects of their own (a notification, how it ended), which those of
            // a batch applied whole would replace: each goes alone, what came before it shown first.
            let alone = matches!(
                item["op"].as_str().unwrap_or_default(),
                "turn"
                    | "result"
                    | "ask"
                    | "permission"
                    | "question"
                    | "permissionCard"
                    | "questionCard"
            );
            if alone && !out.is_empty() {
                on_frame(json!({ "items": std::mem::take(&mut out) }));
            }
            out.push(item);
            if alone {
                on_frame(json!({ "items": std::mem::take(&mut out) }));
            }
        }
        if !out.is_empty() {
            on_frame(json!({ "items": out }));
        }
    }

    /// Its next program, once the one running is stopped: the lines written from now on go to it.
    fn relaunch(&self, launch: proto::Launch) {
        if self.closing.load(Ordering::Acquire) {
            return;
        }
        let (tx, rx) = mpsc::unbounded_channel();
        *self.stdin.lock() = Some(tx);
        self.paused.store(false, Ordering::Release);
        if let Some(next) = self.next.lock().as_ref() {
            let _ = next.send(Next::Launch(Box::new(launch), rx));
        }
    }

    /// Its program stopped, the agent kept.
    fn pause(&self) {
        self.stdin.lock().take();
        self.paused.store(true, Ordering::Release);
        if let Some(next) = self.next.lock().as_ref() {
            let _ = next.send(Next::Pause);
        }
    }

    /// A file its plugin wants written while it runs, in the agent's own folder alone.
    fn write_file(&self, path: &str, text: &str) {
        let home = self.attach.lock().home.clone();
        let done = check_file(self.name, home.as_deref(), path)
            .map_err(|e| anyhow!(e))
            .and_then(|()| {
                if let Some(dir) = Path::new(path).parent() {
                    std::fs::create_dir_all(dir)?;
                }
                std::fs::write(path, text)?;
                Ok(())
            });
        if let Err(e) = done {
            log::warn!("{}: a file of its plugin not written: {e:#}", self.key);
        }
    }

    fn answered(&self, request: &str) {
        self.attach.lock().pending.retain(|r| r != request);
    }
}

/// Why the agent stops with its plugin, to say in its conversation. The app closing is no reason to tell: every agent
/// stops with it, and a turn it cuts goes on at the next start.
fn stop_told(name: &str, e: &CallError) -> Option<String> {
    (!matches!(e, CallError::Stopped(Stop::Shutdown))).then(|| stopped_line(name, e))
}

/// Why the agent stops with its plugin.
fn stopped_line(name: &str, e: &CallError) -> String {
    match e {
        CallError::Stopped(Stop::Crashed { times }) => tr!(
            "Le plugin de {name} s’est arrêté après {times} plantages : l’agent s’arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message.",
            "The plugin of {name} stopped after {times} crashes: the agent stops, its session kept. Start the plugin again in Settings › Plugins, then send a message."
        ),
        e => tr!(
            "Le plugin de {name} est arrêté ({e}) : l’agent s’arrête, sa session est gardée.",
            "The plugin of {name} is stopped ({e}): the agent stops, its session kept."
        ),
    }
}

#[cfg(test)]
#[test]
fn the_app_closing_is_no_reason_to_tell_an_agent_why_it_stops() {
    assert_eq!(
        stop_told("OpenCode", &CallError::Stopped(Stop::Shutdown)),
        None
    );
    let crashed = CallError::Stopped(Stop::Crashed { times: 3 });
    assert!(stop_told("OpenCode", &crashed).is_some_and(|why| why.contains("3 plantages")));
    assert!(stop_told("OpenCode", &CallError::Crashed).is_some());
}

fn ready<'a, T: Send + 'a>(v: T) -> BoxFuture<'a, T> {
    Box::pin(std::future::ready(v))
}

impl EngineProcess for ProcessEngineProcess {
    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
    fn usage(&self) -> Option<JobUsage> {
        self.job.lock().as_ref().and_then(|j| j.usage())
    }
    fn still_running(&self) -> u32 {
        let in_job = self.usage().map_or(0, |u| u.processes);
        in_job.max(u32::from(self.is_alive()))
    }
    fn pids(&self) -> Vec<u32> {
        if let Some(s) = self.shared.lock().as_ref().filter(|_| self.is_alive()) {
            return s.pids();
        }
        self.job
            .lock()
            .as_ref()
            .map(|j| j.pids())
            .unwrap_or_default()
    }
    fn close_input(&self) {
        self.closing.store(true, Ordering::Release);
        if self.shared.lock().is_some() {
            // Once its turn is over, as its plugin says it (`closed`); at once when it says nothing.
            let close = proto::Close {
                agent: self.key.clone(),
                now: false,
            };
            if let (Some(me), Ok(rt)) = (
                self.me.get().and_then(std::sync::Weak::upgrade),
                tokio::runtime::Handle::try_current(),
            ) {
                rt.spawn(async move {
                    if me.ordered(method::CLOSE, close, true).await.is_err() {
                        me.leave();
                    }
                });
            } else {
                self.leave();
            }
            return;
        }
        // No program running: nothing to wait for.
        if self.paused.load(Ordering::Acquire) {
            return self.kill();
        }
        if let Some(line) = &self.close_line {
            self.write(line);
        }
        self.stdin.lock().take();
        // A harness that does not end when its input closes (Cursor CLI's `agent acp`) is killed a moment later.
        if let (Some(grace), Some(me)) = (self.close_grace, self.me.get().cloned()) {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                rt.spawn(async move {
                    tokio::time::sleep(grace).await;
                    if let Some(p) = me.upgrade().filter(|p| p.is_alive()) {
                        p.kill();
                    }
                });
            }
        }
    }
    fn kill(&self) {
        self.closing.store(true, Ordering::Release);
        if self.shared.lock().is_some() {
            let close = json!({ "agent": self.key, "now": true });
            let _ = self.rt.notify(&self.plugin, method::CLOSE, close);
            self.leave();
            return;
        }
        self.stdin.lock().take();
        if let Some(k) = self.kill_tx.lock().take() {
            let _ = k.send(());
        }
        if let Some(job) = self.job.lock().as_ref() {
            job.terminate();
        }
    }
    fn wait_exit(&self, within: Duration) -> BoxFuture<'_, bool> {
        Box::pin(async move {
            let deadline = Instant::now() + within;
            while self.is_alive() && Instant::now() < deadline {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            !self.is_alive()
        })
    }
    /// Gone, and its folder let go by a shared process that goes on for other agents (a worktree is removed next).
    fn wait_gone(&self, within: Duration) -> BoxFuture<'_, bool> {
        Box::pin(async move {
            let deadline = Instant::now() + within;
            if !self.wait_exit(within).await {
                return false;
            }
            while !self.let_go.load(Ordering::Acquire) {
                if Instant::now() >= deadline {
                    return false;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            // What it started goes with it: its worktree cannot be removed before.
            while cfg!(windows) && self.usage().is_some_and(|u| u.processes > 0) {
                if Instant::now() >= deadline {
                    return false;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            true
        })
    }

    fn initialize(&self) -> BoxFuture<'_, Result<Ready>> {
        let mut rx = self.ready.subscribe();
        Box::pin(async move {
            let done = rx
                .wait_for(Option::is_some)
                .await
                .map_err(|_| anyhow!("{} is gone", self.name))?
                .clone();
            match done {
                Some(Ok(())) if !self.opens => {
                    return Ok(Ready {
                        catalog: self.catalog.lock().take(),
                        ..Ready::default()
                    })
                }
                Some(Ok(())) => {}
                Some(Err(why)) => return Err(anyhow!(why)),
                None => unreachable!("waited for"),
            }
            // Its plugin opens its session over its protocol: the agent is ready when it says so.
            let mut opened = self.opened.subscribe();
            let done = tokio::time::timeout(SESSION_WITHIN, opened.wait_for(Option::is_some))
                .await
                .map_err(|_| anyhow!("{}: its session not open in time", self.name))?
                .map_err(|_| anyhow!("{} is gone", self.name))?
                .clone();
            match done {
                Some(Ok(models)) => {
                    let (commands, told) = self.first_said.lock().take().unwrap_or_default();
                    Ok(Ready {
                        commands: commands.as_array().cloned().unwrap_or_default(),
                        models: told,
                        catalog: serde_json::from_value(models)
                            .ok()
                            .filter(|m: &Vec<_>| !m.is_empty()),
                    })
                }
                // Stopped by its plugin: gone by the time the core hears of it.
                Some(Err(why)) => {
                    self.wait_exit(Duration::from_secs(5)).await;
                    Err(anyhow!(why))
                }
                None => unreachable!("waited for"),
            }
        })
    }
    fn send_message(&self, uuid: &str, message: &Message) -> Result<()> {
        let s = proto::SendMessage {
            agent: self.agent(),
            uuid: uuid.to_string(),
            text: message.0.as_str().unwrap_or_default().to_string(),
            // A message its engine made as its harness takes it (Cursor CLI's `prompt` blocks, OpenCode's parts: its
            // images, its text files), passed as it is.
            content: if message.0.is_array() {
                Some(message.0.clone())
            } else {
                message.0.get("prompt").cloned()
            },
        };
        let params = serde_json::to_value(s)?;
        self.attach.lock().running = true;
        let sent = self.control.lock().as_ref().map(|tx| {
            tx.send(Control {
                method: method::SEND_MESSAGE,
                params,
                done: None,
                within: ANSWER_WITHIN,
            })
            .is_ok()
        });
        if sent != Some(true) {
            bail!("{} is gone", self.name);
        }
        Ok(())
    }
    fn interrupt(&self, cancel_queued: bool, timeout: Duration) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let a = proto::Interrupt {
                agent: self.agent(),
                cancel_queued,
            };
            tokio::time::timeout(timeout, self.ordered_within(method::INTERRUPT, a, timeout))
                .await
                .map_err(|_| anyhow!("{}: no answer to the interruption", self.name))?
                .map(drop)
        })
    }
    fn set_model<'a>(&'a self, model: &'a str) -> BoxFuture<'a, Result<()>> {
        self.attach.lock().model = model.to_string();
        let v = proto::SetValue {
            agent: self.agent(),
            value: model.into(),
        };
        Box::pin(self.ordered(method::SET_MODEL, v, true))
    }
    fn set_effort<'a>(&'a self, effort: &'a str) -> BoxFuture<'a, Result<()>> {
        self.attach.lock().effort = effort.to_string();
        let v = proto::SetValue {
            agent: self.agent(),
            value: effort.into(),
        };
        Box::pin(self.ordered(method::SET_EFFORT, v, true))
    }
    fn set_mode<'a>(&'a self, mode: &'a str) -> BoxFuture<'a, Result<()>> {
        self.attach.lock().mode = mode.to_string();
        let v = proto::SetValue {
            agent: self.agent(),
            value: mode.into(),
        };
        Box::pin(self.ordered(method::SET_MODE, v, true))
    }
    fn answer_permission(&self, request_id: &str, answer: PermissionAnswer) -> Result<()> {
        self.answered(request_id);
        let (allow, always, input, message, resume, call, rules) = match answer {
            PermissionAnswer::Allow {
                call,
                input,
                resume,
            } => (true, false, input, None, resume, call, Value::Null),
            PermissionAnswer::Always { call, input, rules } => {
                (true, true, input, None, None, call, rules)
            }
            PermissionAnswer::Deny { call, message } => (
                false,
                false,
                Value::Null,
                Some(message),
                None,
                call,
                Value::Null,
            ),
        };
        let a = proto::AnswerPermission {
            agent: self.agent(),
            request: request_id.to_string(),
            allow,
            always,
            input,
            message,
            resume,
            call,
            rules,
        };
        self.send_ordered(method::ANSWER_PERMISSION, serde_json::to_value(a)?)
    }
    fn answer_question(&self, request_id: &str, call: &str, input: Value) -> Result<()> {
        self.answered(request_id);
        let a = proto::AnswerQuestion {
            agent: self.agent(),
            request: request_id.to_string(),
            call: call.to_string(),
            answers: input["answers"].clone(),
            questions: input["questions"].clone(),
            input,
        };
        self.send_ordered(method::ANSWER_QUESTION, serde_json::to_value(a)?)
    }
    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        Box::pin(async move {
            let (tx, rx) = oneshot::channel();
            *self.reading.lock() = Some(tx);
            self.known().await.map_err(|e| down(self.name, e))?;
            let v = self
                .rt
                .call(
                    &self.plugin,
                    method::READ_USAGE,
                    json!({ "agent": self.key }),
                )
                .await
                .map_err(|e| down(self.name, e))?;
            // To be asked of the harness (`UsageAsked`): what it says comes back with its frames.
            if let Ok(asked) = serde_json::from_value::<proto::UsageAsked>(v.clone()) {
                if !asked.writes.is_empty() {
                    for w in &asked.writes {
                        self.write(w);
                    }
                    let read = tokio::time::timeout(USAGE_WITHIN, rx)
                        .await
                        .map_err(|_| anyhow!("{}: its quota not read in time", self.name))?
                        .map_err(|_| anyhow!("{} is gone", self.name))?;
                    return Ok((!read.is_null()).then_some(read));
                }
            }
            let u: proto::Usage = serde_json::from_value(v)?;
            Ok((!u.usage.is_null()).then_some(u.usage))
        })
    }
    /// A harness whose plugin does not say how keeps its title.
    fn rename<'a>(&'a self, title: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let v = proto::SetValue {
                agent: self.agent(),
                value: title.into(),
            };
            self.known().await.map_err(|e| down(self.name, e))?;
            match self
                .rt
                .call_within(
                    &self.plugin,
                    method::RENAME,
                    serde_json::to_value(v)?,
                    REQUEST_WITHIN,
                )
                .await
            {
                // A harness told through its input (`Writes`), or by the plugin itself (none).
                Ok(v) => match self.asking(v) {
                    Called::Asked(rx) => answer_of(rx, RENAME_WITHIN).await.map(drop),
                    Called::Over(_) => Ok(()),
                },
                Err(CallError::Remote { code, .. }) if code == super::runtime::METHOD_NOT_FOUND => {
                    Ok(())
                }
                Err(e) => Err(down(self.name, e)),
            }
        })
    }
    fn compact<'a>(&'a self, instructions: &'a str) -> BoxFuture<'a, Result<bool>> {
        Box::pin(async move {
            let c = proto::Compact {
                agent: self.agent(),
                instructions: instructions.into(),
            };
            let (tx, rx) = oneshot::channel();
            *self.compacting.lock() = Some(tx);
            let v = self
                .rt
                .call_within(
                    &self.plugin,
                    method::COMPACT,
                    serde_json::to_value(c)?,
                    REQUEST_WITHIN,
                )
                .await
                .map_err(|e| down(self.name, e))?;
            let c: proto::Compacted = serde_json::from_value(v)?;
            if c.writes.is_empty() {
                return Ok(c.over);
            }
            // Asked of the harness through its input: over when it says so (`Item::Compacted`).
            for line in &c.writes {
                self.write(line);
            }
            // A turn of its harness's, which ends it.
            if c.turn {
                return Ok(false);
            }
            tokio::time::timeout(COMPACT_WITHIN, rx)
                .await
                .map_err(|_| anyhow!("{}: its compaction not over in time", self.name))?
                .map_err(|_| anyhow!("{} is gone", self.name))?
        })
    }
    fn attach(&self) -> Option<terminal::Attach> {
        self.joins.lock().clone().filter(|_| self.is_alive())
    }
    fn stop_task<'a>(&'a self, task: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let t = proto::Task {
                agent: self.agent(),
                task: task.to_string(),
            };
            self.ordered_within(method::STOP_TASK, t, Duration::from_secs(15))
                .await
                .map(drop)
                .map_err(not_supported)
        })
    }
    fn task_output<'a>(&'a self, task: &'a str) -> BoxFuture<'a, Result<Value>> {
        Box::pin(async move {
            let t = proto::Task {
                agent: self.agent(),
                task: task.to_string(),
            };
            self.ordered_within(method::TASK_OUTPUT, t, Duration::from_secs(10))
                .await
                .map_err(not_supported)
        })
    }
    fn remote_control(&self, request: Remote, timeout: Duration) -> BoxFuture<'_, Result<Value>> {
        let link = match request {
            Remote::Off => proto::RemoteLink {
                agent: self.agent(),
                enabled: false,
                name: String::new(),
                keep: false,
                reattach: None,
            },
            Remote::On {
                name,
                keep,
                reattach,
            } => proto::RemoteLink {
                agent: self.agent(),
                enabled: true,
                name,
                keep,
                reattach,
            },
        };
        Box::pin(async move {
            self.ordered_within(method::REMOTE, link, timeout)
                .await
                .map_err(not_supported)
        })
    }
    fn control(&self, request: Value, timeout: Duration) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move {
            let c = proto::Control {
                agent: self.agent(),
                request,
                within_ms: timeout.as_millis() as u64,
            };
            self.known().await.map_err(|e| down(self.name, e))?;
            let v = self
                .rt
                .call_within(
                    &self.plugin,
                    method::CONTROL,
                    serde_json::to_value(c)?,
                    timeout + Duration::from_secs(1),
                )
                .await
                .map_err(|e| down(self.name, e))?;
            match self.asking(v) {
                Called::Asked(rx) => answer_of(rx, timeout).await,
                Called::Over(v) => Ok(v),
            }
        })
    }
    fn reply(&self, _: &str, _: std::result::Result<Value, String>) -> Result<()> {
        bail!("not supported")
    }
}

/// What a harness whose plugin has no such method says of a call it cannot make.
fn not_supported(e: anyhow::Error) -> anyhow::Error {
    if e.to_string().contains("not found") {
        anyhow!("not supported")
    } else {
        e
    }
}

impl ProcessEngineProcess {
    /// `ordered` without waiting, from a method that does not wait.
    fn send_ordered(&self, method: &'static str, params: Value) -> Result<()> {
        let sent = self.control.lock().as_ref().map(|tx| {
            tx.send(Control {
                method,
                params,
                done: None,
                within: ANSWER_WITHIN,
            })
            .is_ok()
        });
        if sent != Some(true) {
            bail!("{} is gone", self.name);
        }
        Ok(())
    }
}
