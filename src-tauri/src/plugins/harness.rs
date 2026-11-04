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
use crate::agent::{AgentRt, Effects};
use crate::core::Attachment;
use crate::engine::registry::{
    AccountStore, CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModeCap, ModelDecl, QuotaKind,
    Tested,
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
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, watch};

/// How many lines wait for the plugin at most while it starts again; past them the oldest are dropped, and said.
pub const FRAME_QUEUE: usize = 20_000;

/// How long a batch of frames waits for its answer.
const FRAMES_WITHIN: Duration = Duration::from_secs(10);

/// How long the first call of a launch waits: it may start the plugin's process (its `initialize` has 10 s), on a
/// machine under load.
const ATTACH_WITHIN: Duration = Duration::from_secs(30);

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
];

/// The harnesses of the plugins the app gives that are on, by their plugin, each made once for each runtime (by its
/// address: a test makes several).
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
        let said = |p: &str| {
            launch.args.iter().any(|a| a.contains(p))
                || launch.env.iter().any(|v| v.value.contains(p))
                || launch.files.iter().any(|f| f.text.contains(p))
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
        let path = Path::new(&f.path);
        let inside = home.is_some_and(|home| {
            path.is_absolute()
                && path.starts_with(home)
                && !path.components().any(|c| matches!(c, Component::ParentDir))
        });
        if !inside {
            let file = &f.path;
            return Err(tr!(
                "Le plugin de {name} veut écrire {file}, hors du dossier de l’agent : l’agent ne démarre pas.",
                "The plugin of {name} wants to write {file}, outside the agent’s folder: the agent doesn’t start."
            ));
        }
    }
    Ok(())
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

fn command_of(
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
        };
        let (stdin_tx, stdin_rx) = mpsc::unbounded_channel();
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        let (ready_tx, _) = watch::channel(None);
        let p = Arc::new(ProcessEngineProcess {
            rt: self.rt.clone(),
            key_var: self.key_var,
            close_grace: self.close_grace,
            me: std::sync::OnceLock::new(),
            native: self.native,
            prepare: self.prepare,
            plugin: self.plugin.clone(),
            name: self.name,
            account_var: self.account_var,
            key,
            attach: Mutex::new(attach),
            stdin: Mutex::new(Some(stdin_tx)),
            control: Mutex::new(Some(control_tx)),
            ready: ready_tx,
            alive: AtomicBool::new(true),
            job: Mutex::new(None),
            kill_tx: Mutex::new(None),
            tail: Mutex::new(VecDeque::new()),
        });
        let _ = p.me.set(Arc::downgrade(&p));
        tokio::spawn(control(Arc::downgrade(&p), control_rx));
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

/// A call the control task makes, in order, and who waits for it.
struct Control {
    method: &'static str,
    params: Value,
    done: Option<oneshot::Sender<Result<()>>>,
}

/// The live process of an agent whose harness's plugin runs apart.
pub struct ProcessEngineProcess {
    rt: Arc<Runtime>,
    /// As `ProcessEngine::close_grace`.
    close_grace: Option<Duration>,
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
    stdin: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    control: Mutex<Option<mpsc::UnboundedSender<Control>>>,
    /// The end of its launch: Ok once its harness runs.
    ready: watch::Sender<Option<std::result::Result<(), String>>>,
    alive: AtomicBool,
    job: Mutex<Option<Arc<Job>>>,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    /// The end of what its harness wrote on its stderr.
    tail: Mutex<VecDeque<String>>,
}

/// The calls of the control task, one after the other: their `writes` go to the harness in their order.
async fn control(
    p: std::sync::Weak<ProcessEngineProcess>,
    mut rx: mpsc::UnboundedReceiver<Control>,
) {
    while let Some(c) = rx.recv().await {
        let Some(p) = p.upgrade() else { break };
        let result = p.call_writing(c.method, c.params).await;
        if let Some(done) = c.done {
            let _ = done.send(result);
        } else if let Err(e) = result {
            log::warn!("{}: {e:#}", p.key);
        }
    }
}

impl ProcessEngineProcess {
    /// Writes `line` on its harness's input.
    fn write(&self, line: &str) {
        let mut bytes = Vec::with_capacity(line.len() + 1);
        bytes.extend_from_slice(line.as_bytes());
        bytes.push(b'\n');
        if let Some(tx) = self.stdin.lock().as_ref() {
            let _ = tx.send(bytes);
        }
    }

    fn attach_params(&self) -> Value {
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
            Err(e @ CallError::Stopped(_)) => Err(e),
            _ => Ok(()),
        }
    }

    /// Calls `method`, given again after a crash; writes its `writes` to the harness.
    async fn call_writing(&self, method: &str, params: Value) -> Result<()> {
        for _ in 0..RETRIES {
            match self.rt.call(&self.plugin, method, params.clone()).await {
                Ok(v) => {
                    let writes: proto::Writes = serde_json::from_value(v).unwrap_or_default();
                    for w in &writes.writes {
                        self.write(w);
                    }
                    return Ok(());
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
        let params = serde_json::to_value(params)?;
        let (done, rx) = if wait {
            let (tx, rx) = oneshot::channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };
        let sent = self.control.lock().as_ref().map(|tx| {
            tx.send(Control {
                method,
                params,
                done,
            })
            .is_ok()
        });
        if sent != Some(true) {
            bail!("{} is gone", self.name);
        }
        match rx {
            Some(rx) => rx.await.map_err(|_| anyhow!("{} is gone", self.name))?,
            None => Ok(()),
        }
    }

    fn agent(&self) -> String {
        self.key.clone()
    }

    /// Launches its harness from the plugin's description, checked: its child. The first call may start the plugin's
    /// process: it is given the time of a start (`ATTACH_WITHIN`), not that of a call.
    async fn launch(&self, spawn: &Spawn) -> Result<(tokio::process::Child, Option<Arc<Job>>)> {
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
            start: match &spawn.start {
                Start::New => proto::StartAt::New,
                Start::Resume { session } => proto::StartAt::Resume {
                    session: session.clone(),
                },
                Start::Fork { of, at } => proto::StartAt::Fork {
                    of: of.clone(),
                    at: at.clone(),
                },
            },
            read_dirs: spawn
                .read_dirs
                .iter()
                .map(|d| d.to_string_lossy().into_owned())
                .collect(),
            escouade: match &spawn.escouade {
                Escouade::Granted { url, token, .. } => Some(proto::Grant {
                    url: url.clone(),
                    token: token.clone(),
                }),
                Escouade::Denied { .. } => None,
            },
            fence: fence.clone(),
            read_only: spawn.read_only,
            program: spawn.program.describe(),
            home: home.clone(),
            described: self.attach.lock().described.clone(),
            lang: crate::i18n::ui().tag().to_string(),
        };
        let v = self
            .rt
            .call(&self.plugin, method::LAUNCH, serde_json::to_value(&params)?)
            .await
            .map_err(|e| down(self.name, e))?;
        let launch: proto::Launch = serde_json::from_value(v)?;
        check_launch(
            self.name,
            super::builtin::is_builtin(&self.plugin),
            token.as_deref(),
            &fence,
            spawn.read_only,
            home.as_deref(),
            &launch,
        )
        .map_err(|e| anyhow!(e))?;
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
        for f in &launch.files {
            let path = Path::new(&f.path);
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(path, &f.text)?;
        }
        log::info!(
            "{}: starting {} ({}) in {} (account {})",
            spawn.label,
            spawn.program.describe(),
            self.name,
            spawn.cwd,
            spawn.account
        );
        let env = merged(&spawn.env, &launch.env);
        let told = launch.args.iter().chain(&added).cloned();
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
        for w in &launch.writes {
            self.write(w);
        }
        Ok((child, job))
    }

    /// The process's life: its launch, its frames, its end.
    async fn run(
        self: Arc<Self>,
        spawn: Spawn,
        mut stdin_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        on_frame: OnFrame,
        on_exit: OnExit,
    ) {
        let (mut child, job) = match self.launch(&spawn).await {
            Ok(c) => c,
            Err(e) => {
                let why = format!("{e:#}");
                self.alive.store(false, Ordering::Release);
                self.control.lock().take();
                let _ = self.ready.send(Some(Err(why.clone())));
                self.rt.detach(&self.plugin, &self.key);
                on_exit(None, why);
                return;
            }
        };
        *self.job.lock() = job.clone();
        let (kill_tx, mut kill_rx) = oneshot::channel();
        *self.kill_tx.lock() = Some(kill_tx);
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
            let me = Arc::downgrade(&self);
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let Some(p) = me.upgrade() else { break };
                    let mut tail = p.tail.lock();
                    tail.push_back(line);
                    if tail.len() > 20 {
                        tail.pop_front();
                    }
                }
            });
        }
        let (lines_tx, lines_rx) = mpsc::channel::<String>(FRAME_QUEUE);
        let stdout = child.stdout.take();
        let reader = tokio::spawn(async move {
            let Some(stdout) = stdout else { return };
            let mut lines = BufReader::new(stdout).split(b'\n');
            while let Ok(Some(bytes)) = lines.next_segment().await {
                let text = String::from_utf8_lossy(&bytes);
                let text = text.trim();
                if !text.is_empty() && lines_tx.send(text.to_string()).await.is_err() {
                    break;
                }
            }
        });
        let _ = self.ready.send(Some(Ok(())));
        let pump = {
            let me = self.clone();
            tokio::spawn(async move { me.pump(lines_rx, &on_frame).await })
        };
        let code = tokio::select! {
            status = child.wait() => status.ok().and_then(|s| s.code()),
            _ = &mut kill_rx => {
                let _ = child.start_kill();
                if let Some(job) = &job { job.terminate(); }
                child.wait().await.ok().and_then(|s| s.code())
            }
        };
        let _ = reader.await;
        let stopped = pump.await.ok().flatten();
        self.alive.store(false, Ordering::Release);
        self.control.lock().take();
        let _ = self
            .rt
            .notify(&self.plugin, method::DETACH, json!({ "agent": self.key }));
        self.rt.detach(&self.plugin, &self.key);
        let said = stopped.unwrap_or_else(|| {
            self.tail
                .lock()
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        });
        on_exit(code, said);
    }

    /// Reads its harness's lines in batches and gives them to the plugin; answers why it stopped the harness, if it
    /// did (its plugin stopped).
    async fn pump(&self, mut lines: mpsc::Receiver<String>, on_frame: &OnFrame) -> Option<String> {
        let mut stopped = None;
        while let Some(first) = lines.recv().await {
            // What was read meanwhile goes with it, up to 64 KiB; nothing waits for more: under a fast stream the
            // lines gather while the last batch is with the plugin, and a lone line goes at once.
            let mut bytes = first.len();
            let mut batch = vec![first];
            while bytes < proto::BATCH_BYTES {
                match lines.try_recv() {
                    Ok(l) => {
                        bytes += l.len();
                        batch.push(l);
                    }
                    Err(_) => break,
                }
            }
            // Once its plugin is stopped, what its harness still says before it ends is left aside.
            if stopped.is_some() {
                continue;
            }
            if let Err(why) = self.translate_batch(batch, on_frame).await {
                on_frame(json!({ "items": [{ "op": "notice", "level": "error", "text": why }] }));
                self.kill();
                stopped = Some(why);
            }
        }
        stopped
    }

    /// One batch, given to the plugin until it answers (again after a crash); its items applied. Err: the plugin is
    /// stopped, and why.
    async fn translate_batch(
        &self,
        batch: Vec<String>,
        on_frame: &OnFrame,
    ) -> std::result::Result<(), String> {
        let params = serde_json::value::to_raw_value(&proto::Frames {
            agent: self.key.clone(),
            frames: batch,
        })
        .map_err(|e| e.to_string())?;
        let name = self.name;
        let mut told = false;
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
                    if let Err(e) = self.reattach().await {
                        return Err(stopped_line(name, &e));
                    }
                }
                Err(e @ CallError::Stopped(_)) => return Err(stopped_line(name, &e)),
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
        Err(stopped_line(name, &CallError::Crashed))
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
                "start" => {
                    if let Some(s) = item["session"].as_str() {
                        self.attach.lock().session = Some(s.to_string());
                    }
                }
                "permission" | "question" => {
                    if let Some(r) = item["request"].as_str() {
                        self.attach.lock().pending.push(r.to_string());
                    }
                }
                "turn" => self.attach.lock().pending.clear(),
                _ => {}
            }
            out.push(item);
        }
        if !out.is_empty() {
            on_frame(json!({ "items": out }));
        }
    }

    fn answered(&self, request: &str) {
        self.attach.lock().pending.retain(|r| r != request);
    }
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
        u32::from(self.is_alive())
    }
    fn pids(&self) -> Vec<u32> {
        self.job
            .lock()
            .as_ref()
            .map(|j| j.pids())
            .unwrap_or_default()
    }
    fn close_input(&self) {
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
    fn wait_gone(&self, within: Duration) -> BoxFuture<'_, bool> {
        self.wait_exit(within)
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
                Some(Ok(())) => Ok(Ready::default()),
                Some(Err(why)) => Err(anyhow!(why)),
                None => unreachable!("waited for"),
            }
        })
    }
    fn send_message(&self, uuid: &str, message: &Message) -> Result<()> {
        let s = proto::SendMessage {
            agent: self.agent(),
            uuid: uuid.to_string(),
            text: message.0.as_str().unwrap_or_default().to_string(),
            // A message its engine made as its harness takes it (Cursor CLI's `prompt` blocks: its images, its text
            // files), passed as it is.
            content: message.0.get("prompt").cloned(),
        };
        let params = serde_json::to_value(s)?;
        let sent = self.control.lock().as_ref().map(|tx| {
            tx.send(Control {
                method: method::SEND_MESSAGE,
                params,
                done: None,
            })
            .is_ok()
        });
        if sent != Some(true) {
            bail!("{} is gone", self.name);
        }
        Ok(())
    }
    fn interrupt(&self, _cancel_queued: bool, timeout: Duration) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let a = proto::AgentRef {
                agent: self.agent(),
            };
            tokio::time::timeout(timeout, self.ordered(method::INTERRUPT, a, true))
                .await
                .map_err(|_| anyhow!("{}: no answer to the interruption", self.name))?
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
        let (allow, always, input, message, resume) = match answer {
            PermissionAnswer::Allow { input, resume, .. } => (true, false, input, None, resume),
            PermissionAnswer::Always { input, .. } => (true, true, input, None, None),
            PermissionAnswer::Deny { message, .. } => {
                (false, false, Value::Null, Some(message), None)
            }
        };
        let a = proto::AnswerPermission {
            agent: self.agent(),
            request: request_id.to_string(),
            allow,
            always,
            input,
            message,
            resume,
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
        };
        self.send_ordered(method::ANSWER_QUESTION, serde_json::to_value(a)?)
    }
    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        Box::pin(async move {
            let v = self
                .rt
                .call(
                    &self.plugin,
                    method::READ_USAGE,
                    json!({ "agent": self.key }),
                )
                .await
                .map_err(|e| down(self.name, e))?;
            let u: proto::Usage = serde_json::from_value(v)?;
            Ok((!u.usage.is_null()).then_some(u.usage))
        })
    }
    fn rename<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        ready(Ok(()))
    }
    fn stop_task<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        ready(Err(anyhow!("not supported")))
    }
    fn task_output<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<Value>> {
        ready(Err(anyhow!("not supported")))
    }
    fn remote_control(&self, _: Remote, _: Duration) -> BoxFuture<'_, Result<Value>> {
        ready(Err(anyhow!("not supported")))
    }
    fn control(&self, _: Value, _: Duration) -> BoxFuture<'_, Result<Value>> {
        ready(Err(anyhow!("not supported")))
    }
    fn reply(&self, _: &str, _: std::result::Result<Value, String>) -> Result<()> {
        bail!("not supported")
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
            })
            .is_ok()
        });
        if sent != Some(true) {
            bail!("{} is gone", self.name);
        }
        Ok(())
    }
}
