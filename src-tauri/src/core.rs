//! Application core: projects, agents and their Claude processes, git, usage, persistence.

use crate::agent::{AgentHandle, AgentRt, Effects, NotifyKind};
use crate::claude::{self, ClaudeProcess, SpawnOpts};
use crate::git::{self, GitService};
use crate::hub::Hub;
use crate::job::JobUsage;
use crate::model::*;
use crate::notify;
use crate::paths::{self, DataDir};
use crate::pty::PtyManager;
use crate::resources;
use crate::stats::Stats;
use crate::usage;
use anyhow::{anyhow, bail, Context, Result};
use parking_lot::{Mutex, RwLock};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Runtime, Wry};
use tokio::sync::mpsc;

/// The Claude process exited while starting; the exit handler recorded why in the conversation.
#[derive(Debug)]
pub struct StartupFailure;

impl std::fmt::Display for StartupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Claude Code n'a pas pu démarrer : voir le détail dans la conversation.")
    }
}

impl std::error::Error for StartupFailure {}

/// A sync of a project's checkout with its remote, asked by the user.
#[derive(Debug, Clone, Copy)]
pub enum SyncOp {
    Fetch,
    Pull,
    Push,
}

/// After the usage limit resets, before sending "continue": clocks may differ a little.
const RESUME_MARGIN_MS: i64 = 30_000;

/// How often every repository with a remote is fetched in the background.
const FETCH_EVERY: Duration = Duration::from_secs(5 * 60);

/// A file attached to a message: `data` is base64 for images and PDFs, the text itself for
/// text files (`text/plain`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub name: String,
    pub media_type: String,
    pub data: String,
}

impl Attachment {
    fn is_image(&self) -> bool {
        self.media_type.starts_with("image/")
    }
}

/// The only image formats the API reads.
const IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];
const KB: usize = 1024;
const MB: usize = 1024 * KB;
/// Past this, a text file alone would fill Claude's context (~4 bytes a token).
const MAX_TEXT: usize = 256 * KB;
/// All the files of a message: sent in base64 (4/3 bigger), under the API's 32 MB a request.
const MAX_TOTAL: usize = 18 * MB;

fn size_label(bytes: usize) -> String {
    if bytes >= MB {
        format!("{} Mo", bytes / MB)
    } else {
        format!("{} Ko", bytes / KB)
    }
}

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
        bail!(
            "Les fichiers joints à un message sont limités à {} en tout.",
            size_label(MAX_TOTAL)
        );
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
            5 * MB,
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
        t => bail!(
            "« {} » ({t}) ne peut pas être joint : les fichiers acceptés sont les images (PNG, JPEG, GIF, WebP), les PDF et les fichiers texte.",
            a.name
        ),
    };
    if size > max {
        bail!("{} dépasse {}", a.name, size_label(max));
    }
    Ok((block, size))
}

pub struct Core<R: Runtime = Wry> {
    pub app: AppHandle<R>,
    pub data: DataDir,
    pub hub: Hub,
    pub settings: RwLock<Settings>,
    pub projects: RwLock<Vec<Project>>,
    /// Every project's tickets.
    pub tickets: RwLock<Vec<Ticket>>,
    pub ui: RwLock<UiState>,
    pub agents: RwLock<HashMap<String, AgentHandle>>,
    pub stats: Stats,
    pub usage: Mutex<UsageSnapshot>,
    /// Claude Code's models as it last reported them (the version each alias runs).
    pub models: RwLock<Vec<ModelInfo>>,
    pub git: GitService,
    pub git_cache: RwLock<HashMap<String, GitInfo>>,
    pub pty: PtyManager,
    spawn_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// Serializes agent creation so that concurrent creations get distinct names.
    create_lock: tokio::sync::Mutex<()>,
    conv_buffer: Mutex<HashMap<String, Vec<ConvOp>>>,
    conv_flush: tokio::sync::Notify,
    last_oauth_call: Mutex<Option<i64>>,
    resources: Mutex<resources::Sampler>,
    git_inflight: Mutex<std::collections::HashSet<String>>,
    /// One fetch, pull or push at a time per repository.
    sync_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    toplevels: Mutex<HashMap<String, Option<String>>>,
    dirty: AtomicBool,
    waiting: AtomicUsize,
    pub quitting: AtomicBool,
    /// Files left unsaved in the editor, as the window last said.
    pub unsaved: AtomicUsize,
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            log::error!("invalid {}: {e}", path.display());
            let _ = std::fs::copy(path, path.with_extension("broken.json"));
            None
        }
    }
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

pub(crate) fn claude_args(m: &AgentMeta) -> Vec<String> {
    let mut a: Vec<String> = CLAUDE_BASE_ARGS.iter().map(|s| s.to_string()).collect();
    a.extend(["--model".into(), m.model.clone()]);
    if supports_effort(&m.model) {
        a.extend(["--effort".into(), m.effort.clone()]);
    }
    a.extend(["--permission-mode".into(), m.mode.clone()]);
    if let Some(s) = &m.session_id {
        a.push(format!("--resume={s}"));
    }
    a
}

/// Kebab-case ASCII slug from free text (accents folded).
pub fn slugify(s: &str) -> String {
    let folded: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ÿ' => 'y',
            'ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    let mut out = String::new();
    for part in folded.split('-').filter(|p| !p.is_empty()) {
        if out.len() + part.len() + 1 > 40 {
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(part);
    }
    out
}

/// Instructions + the task framed as text to name (so that the model does not try to do it).
pub(crate) fn naming_prompt(task: &str) -> String {
    format!(
        "Donne un nom court à la tâche de développement ci-dessous. Ne la réalise pas.\n\
         Réponds uniquement par un slug kebab-case de 2 ou 3 mots (minuscules ASCII, sans accents), \
         par exemple refacto-auth ou tests-e2e.\n\n<tache>\n{}\n</tache>",
        claude::truncate(task.trim(), 2000)
    )
}

/// The model's answer as an agent name, if it looks like one: a sentence (an attempt at the
/// task, a refusal, "Voici le slug : …") is never a name.
pub(crate) fn name_from_answer(raw: &str) -> Option<String> {
    let line = raw
        .trim()
        .lines()
        .next()?
        .trim()
        .trim_matches(|c| matches!(c, '`' | '"' | '\'' | '*'));
    if line.is_empty() || line.contains(':') || line.ends_with(['.', '!', '?']) {
        return None;
    }
    let words = line
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|w| !w.is_empty())
        .count();
    if words > 4 {
        return None;
    }
    let slug = slugify(line);
    (!slug.is_empty()).then_some(slug)
}

fn sub_prefix(root: &str, sub: &str) -> String {
    let norm = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_lowercase();
    if norm(root) == norm(sub) {
        String::new()
    } else {
        let rel = paths::relative_slash(root, sub);
        if rel.contains(':') {
            String::new()
        } else {
            format!("{rel}/")
        }
    }
}

impl<R: Runtime> Core<R> {
    pub fn load(app: AppHandle<R>, data: DataDir) -> (Arc<Self>, mpsc::UnboundedReceiver<String>) {
        if let Err(e) = data.ensure() {
            log::error!("cannot create data dir: {e}");
        }
        let settings: Settings = read_json(&data.settings_file()).unwrap_or_default();
        let state: PersistedState = read_json(&data.state_file()).unwrap_or_default();
        let conv_dir = data.conversations();
        let agents = state
            .agents
            .into_iter()
            .map(|m| {
                (
                    m.id.clone(),
                    Arc::new(Mutex::new(AgentRt::new(m, &conv_dir))),
                )
            })
            .collect();
        let (git, rx) = GitService::new();
        let core = Arc::new(Self {
            stats: Stats::open(&data.stats_db()),
            app,
            data,
            hub: Hub::default(),
            settings: RwLock::new(settings),
            projects: RwLock::new(state.projects),
            tickets: RwLock::new(state.tickets),
            ui: RwLock::new(state.ui),
            agents: RwLock::new(agents),
            usage: Mutex::new(UsageSnapshot::default()),
            models: RwLock::new(state.models),
            git,
            git_cache: RwLock::default(),
            pty: PtyManager::default(),
            spawn_locks: Mutex::default(),
            create_lock: tokio::sync::Mutex::new(()),
            conv_buffer: Mutex::default(),
            conv_flush: tokio::sync::Notify::new(),
            last_oauth_call: Mutex::new(None),
            resources: Mutex::default(),
            git_inflight: Mutex::default(),
            sync_locks: Mutex::default(),
            toplevels: Mutex::default(),
            dirty: AtomicBool::new(false),
            waiting: AtomicUsize::new(usize::MAX),
            quitting: AtomicBool::new(false),
            unsaved: AtomicUsize::new(0),
        });
        core.usage.lock().today_cost = core.stats.today_cost();
        (core, rx)
    }

    pub fn start(self: &Arc<Self>, git_rx: mpsc::UnboundedReceiver<String>) {
        for p in self.projects.read().iter() {
            self.git.watch(&p.id, &p.path);
        }
        let c = self.clone();
        tauri::async_runtime::spawn(async move { c.git_loop(git_rx).await });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                c.conv_flush.notified().await;
                tokio::time::sleep(Duration::from_millis(16)).await;
                c.flush_conv();
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(400)).await;
                if c.dirty.swap(false, Ordering::AcqRel) {
                    c.save_now();
                }
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            loop {
                c.refresh_usage().await;
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                c.stop_idle_processes();
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(20)).await;
                c.resume_due().await;
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut shown = false;
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let resources = c.sample_resources();
                // Once more when the last process stops, then quiet until one runs.
                if resources.instances > 0 || shown {
                    shown = resources.instances > 0;
                    c.hub.emit(UiEvent::Resources { resources });
                }
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            // Shortly after startup, then regularly: the commits to pull show up by themselves.
            tokio::time::sleep(Duration::from_secs(15)).await;
            loop {
                c.fetch_all().await;
                tokio::time::sleep(FETCH_EVERY).await;
            }
        });
        self.start_remote_agents();
        self.update_tray();
    }

    // ---------- persistence ----------

    fn snapshot(&self) -> PersistedState {
        let mut agents: Vec<AgentMeta> = self
            .agents
            .read()
            .values()
            .map(|h| h.lock().meta.clone())
            .collect();
        agents.sort_by_key(|m| m.created_at);
        // One lock at a time (each guard ends with its statement).
        let projects = self.projects.read().clone();
        let ui = self.ui.read().clone();
        let models = self.models.read().clone();
        let tickets = self.tickets.read().clone();
        PersistedState {
            projects,
            agents,
            ui,
            models,
            tickets,
        }
    }

    pub fn save_now(&self) {
        let state = self.snapshot();
        match serde_json::to_vec_pretty(&state) {
            Ok(bytes) => {
                if let Err(e) = paths::write_atomic(&self.data.state_file(), &bytes) {
                    log::error!("cannot save state: {e}");
                }
            }
            Err(e) => log::error!("cannot serialize state: {e}"),
        }
    }

    pub fn request_save(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    pub fn save_settings(&self, s: Settings) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&s)?;
        paths::write_atomic(&self.data.settings_file(), &bytes)?;
        let auto_resume = s.auto_resume;
        *self.settings.write() = s;
        if !auto_resume {
            // Turned off: the resumes already planned go too.
            let dropped: Vec<AgentView> = self
                .agents
                .read()
                .values()
                .filter_map(|h| {
                    let mut rt = h.lock();
                    rt.meta.resume_at.take()?;
                    Some(rt.view())
                })
                .collect();
            if !dropped.is_empty() {
                for agent in dropped {
                    self.hub.emit(UiEvent::Agent { agent });
                }
                self.request_save();
            }
        }
        Ok(())
    }

    // ---------- lookups ----------

    pub fn agent(&self, id: &str) -> Result<AgentHandle> {
        self.agents
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("agent introuvable"))
    }

    pub fn project(&self, id: &str) -> Result<Project> {
        self.projects
            .read()
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| anyhow!("projet introuvable"))
    }

    pub fn agent_views(&self) -> Vec<AgentView> {
        let mut v: Vec<AgentView> = self
            .agents
            .read()
            .values()
            .map(|h| h.lock().view())
            .collect();
        v.sort_by_key(|a| a.meta.created_at);
        v
    }

    fn project_agents(&self, project_id: &str) -> Vec<AgentHandle> {
        self.agents
            .read()
            .values()
            .filter(|h| h.lock().meta.project_id == project_id)
            .cloned()
            .collect()
    }

    /// Late work on a deleted agent (naming, remote control…) must not bring it back in the UI.
    fn is_registered(&self, id: &str) -> bool {
        self.agents.read().contains_key(id)
    }

    fn emit_agent(&self, h: &AgentHandle) {
        let view = h.lock().view();
        if !self.is_registered(&view.meta.id) {
            return;
        }
        self.hub.emit(UiEvent::Agent { agent: view });
        self.update_tray();
    }

    fn spawn_lock(&self, id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.spawn_locks
            .lock()
            .entry(id.to_string())
            .or_default()
            .clone()
    }

    async fn toplevel(&self, path: &str) -> Option<String> {
        if let Some(t) = self.toplevels.lock().get(path) {
            return t.clone();
        }
        let t = git::toplevel(path).await;
        self.toplevels.lock().insert(path.to_string(), t.clone());
        t
    }

    // ---------- effects of agent activity ----------

    /// Streaming deltas are buffered and sent at most every 16 ms (merged); any other op first
    /// flushes the agent's buffered deltas so the UI sees every op in order.
    fn emit_conv(&self, agent_id: &str, ops: Vec<ConvOp>) {
        let only_deltas = ops.iter().all(|o| matches!(o, ConvOp::Delta { .. }));
        let mut buf = self.conv_buffer.lock();
        let was_empty = buf.is_empty();
        let pending = buf.entry(agent_id.to_string()).or_default();
        pending.extend(ops);
        if only_deltas {
            if was_empty {
                self.conv_flush.notify_one();
            }
            return;
        }
        let ops = coalesce_ops(std::mem::take(pending));
        buf.remove(agent_id);
        drop(buf);
        self.hub.emit(UiEvent::Conv {
            agent_id: agent_id.to_string(),
            ops,
        });
    }

    /// Sends every buffered delta now.
    pub fn flush_conv(&self) {
        let drained: Vec<(String, Vec<ConvOp>)> = self.conv_buffer.lock().drain().collect();
        for (agent_id, ops) in drained {
            self.hub.emit(UiEvent::Conv {
                agent_id,
                ops: coalesce_ops(ops),
            });
        }
    }

    fn apply(
        self: &Arc<Self>,
        id: &str,
        project_id: &str,
        name: &str,
        fx: Effects,
        view: Option<AgentView>,
    ) {
        if !fx.ops.is_empty() {
            self.emit_conv(id, fx.ops);
        }
        let changed = view.is_some();
        if let Some(v) = view.filter(|_| self.is_registered(id)) {
            self.hub.emit(UiEvent::Agent { agent: v });
        }
        if !fx.turns.is_empty() {
            self.stats.record_turns(id, project_id, &fx.turns);
            let mut u = self.usage.lock();
            u.today_cost = self.stats.today_cost();
            self.hub.emit(UiEvent::Usage { usage: u.clone() });
        }
        if let Some((five, week)) = fx.rate {
            let mut u = self.usage.lock();
            if five.is_some() {
                u.five_hour = five;
            }
            if week.is_some() {
                u.seven_day = week;
            }
            u.updated_at = now_ms();
            self.hub.emit(UiEvent::Usage { usage: u.clone() });
        }
        if let Some(resets_at) = fx.limited {
            self.plan_resume(id, resets_at);
        }
        if fx.files_changed {
            self.git.refresh(project_id);
        }
        if fx.save {
            self.request_save();
        }
        if changed {
            self.update_tray();
        }
        if let Some(kind) = fx.notify {
            self.notify_agent(kind, project_id, id, name);
        }
    }

    fn on_frame(self: &Arc<Self>, h: &AgentHandle, gen: u64, frame: Value) {
        let mut fx = Effects::default();
        let (id, pid, name, view) = {
            let mut rt = h.lock();
            if rt.gen != gen {
                return;
            }
            rt.handle_frame(&frame, &mut fx);
            let view = fx.agent_changed.then(|| rt.view());
            (
                rt.meta.id.clone(),
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                view,
            )
        };
        self.apply(&id, &pid, &name, fx, view);
    }

    fn on_exit(self: &Arc<Self>, h: &AgentHandle, gen: u64, code: Option<i32>, stderr: String) {
        let mut fx = Effects::default();
        let (id, pid, name, view) = {
            let mut rt = h.lock();
            let current = rt.gen == gen;
            log::info!(
                "agent {}: claude exited (code {code:?}, current process: {current})",
                rt.meta.id
            );
            if !current {
                // A replaced, stopped or deleted agent's process: nothing to report.
                return;
            }
            rt.on_exit(gen, code, &stderr, &mut fx);
            (
                rt.meta.id.clone(),
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.view(),
            )
        };
        if self.quitting.load(Ordering::Acquire) {
            return;
        }
        self.apply(&id, &pid, &name, fx, Some(view));
    }

    fn notify_agent(
        self: &Arc<Self>,
        kind: NotifyKind,
        project_id: &str,
        agent_id: &str,
        agent_name: &str,
    ) {
        let settings = self.settings.read().clone();
        if settings.sound {
            notify::play_chime();
        }
        if notify::window_attended(&self.app) {
            return;
        }
        notify::flash(&self.app);
        if settings.os_notifications {
            let project = self.project(project_id).map(|p| p.name).unwrap_or_default();
            let body = match kind {
                NotifyKind::Question => "Claude attend ta réponse",
                NotifyKind::Done => "Tâche terminée",
                NotifyKind::Error => "Erreur : l'agent s'est arrêté",
            };
            let (app, pid, aid) = (
                self.app.clone(),
                project_id.to_string(),
                agent_id.to_string(),
            );
            let hub_core = Arc::downgrade(self);
            notify::toast(
                &self.app,
                &format!("{project} · {agent_name}"),
                body,
                move || {
                    notify::show_main(&app);
                    if let Some(c) = hub_core.upgrade() {
                        c.hub.emit(UiEvent::Focus {
                            project_id: pid.clone(),
                            agent_id: Some(aid.clone()),
                        });
                    }
                },
            );
        }
    }

    pub fn update_tray(&self) {
        let n = self
            .agents
            .read()
            .values()
            .filter(|h| h.lock().meta.status == AgentStatus::Waiting)
            .count();
        if self.waiting.swap(n, Ordering::AcqRel) == n {
            return;
        }
        if let Some(tray) = self.app.tray_by_id("main") {
            let _ = tray.set_icon(notify::tray_icon(&self.app, n));
            let tip = match n {
                0 => "Escouade".to_string(),
                1 => "Escouade — 1 agent en attente".to_string(),
                n => format!("Escouade — {n} agents en attente"),
            };
            let _ = tray.set_tooltip(Some(tip));
        }
    }

    // ---------- claude processes ----------

    /// Returns the agent's live process, starting it (with --resume) when needed.
    pub async fn ensure_process(self: &Arc<Self>, id: &str) -> Result<Arc<ClaudeProcess>> {
        let resumed = self.agent(id)?.lock().meta.session_id.is_some();
        match self.start_process(id).await {
            // The session could not be resumed: its exit handler dropped the session id, so a
            // second start opens a new session instead.
            Err(e)
                if resumed
                    && e.is::<StartupFailure>()
                    && self.agent(id)?.lock().meta.session_id.is_none() =>
            {
                self.start_process(id).await
            }
            other => other,
        }
    }

    async fn start_process(self: &Arc<Self>, id: &str) -> Result<Arc<ClaudeProcess>> {
        let h = self.agent(id)?;
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        if let Some(p) = h.lock().proc.clone() {
            if p.is_alive() {
                return Ok(p);
            }
        }
        let settings = self.settings.read().clone();
        let program = claude::resolve_binary(&settings.claude_path).ok_or_else(|| {
            anyhow!("Claude Code introuvable. Installe-le ou indique son chemin dans les réglages.")
        })?;
        let (opts, gen) = {
            let mut rt = h.lock();
            rt.gen += 1;
            let opts = SpawnOpts {
                program,
                cwd: rt.meta.cwd.clone(),
                args: claude_args(&rt.meta),
                env: settings.proxy_env(),
            };
            (opts, rt.gen)
        };
        if !Path::new(&opts.cwd).is_dir() {
            bail!("Le dossier {} n'existe plus", opts.cwd);
        }
        log::info!(
            "agent {id}: starting {} {} in {}",
            opts.program.display(),
            opts.args.join(" "),
            opts.cwd
        );
        let started = std::time::Instant::now();
        let (w1, w2) = (Arc::downgrade(self), Arc::downgrade(self));
        let (h1, h2) = (h.clone(), h.clone());
        let proc = ClaudeProcess::spawn(
            opts,
            move |frame| {
                if let Some(c) = w1.upgrade() {
                    c.on_frame(&h1, gen, frame);
                }
            },
            move |code, stderr| {
                if let Some(c) = w2.upgrade() {
                    c.on_exit(&h2, gen, code, stderr);
                }
            },
        )?;
        h.lock().attach(proc.clone());
        self.emit_agent(&h);
        match proc
            .control(json!({ "subtype": "initialize" }), Duration::from_secs(90))
            .await
        {
            Ok(resp) => {
                log::info!("agent {id}: ready in {} ms", started.elapsed().as_millis());
                h.lock().commands = resp["commands"].as_array().cloned().unwrap_or_default();
                self.set_models(&resp["models"]);
                if h.lock().meta.remote_control {
                    if let Err(e) = self.link_remote(&h, &proc).await {
                        log::warn!("agent {id}: remote control failed: {e:#}");
                        let _ = self.with_agent(id, |rt, fx| {
                            rt.notice("warn", format!("Remote control indisponible : {e}"), fx);
                            Ok(())
                        });
                    }
                }
            }
            Err(_) if !proc.is_alive() => {
                log::warn!("agent {id}: claude exited while starting");
                return Err(StartupFailure.into());
            }
            Err(e) => log::warn!(
                "agent {id}: initialize failed after {} ms: {e}",
                started.elapsed().as_millis()
            ),
        }
        Ok(proc)
    }

    /// Keeps the models Claude Code reported at a process start, for every label of the UI. What it
    /// no longer reports goes: better a bare "Sonnet" than a version it no longer runs.
    fn set_models(&self, reported: &Value) {
        let models = ModelInfo::list(reported);
        let mut current = self.models.write();
        if *current == models {
            return;
        }
        *current = models.clone();
        // Under the lock: two processes starting together leave the UI with what is kept.
        self.hub.emit(UiEvent::Models { models });
        self.request_save();
    }

    /// Starts the agent's process in the background so the first message answers fast.
    pub fn warm(self: &Arc<Self>, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        {
            let rt = h.lock();
            if rt.proc.is_some() || rt.meta.archived || rt.meta.status == AgentStatus::Error {
                return;
            }
        }
        let (c, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move {
            if let Err(e) = c.ensure_process(&id).await {
                log::warn!("warm-up failed: {e:#}");
            }
        });
    }

    // ---------- remote control ----------

    /// Name of the agent's session in claude.ai / the Claude app.
    fn remote_name(&self, meta: &AgentMeta) -> String {
        let project = self
            .project(&meta.project_id)
            .map(|p| p.name)
            .unwrap_or_default();
        format!("{project} · {}", meta.name)
    }

    /// Links the agent's live process to claude.ai (Remote Control). Its previous remote session
    /// is reattached, so the link opened on a phone stays valid across restarts.
    async fn link_remote(
        self: &Arc<Self>,
        h: &AgentHandle,
        proc: &Arc<ClaudeProcess>,
    ) -> Result<()> {
        let (name, reattach) = {
            let rt = h.lock();
            (self.remote_name(&rt.meta), rt.meta.remote_session.clone())
        };
        let request = |reattach: Option<&String>| {
            let mut r = json!({ "subtype": "remote_control", "enabled": true, "name": name, "keep_session_on_exit": true });
            if let Some(s) = reattach {
                r["reattach_session_id"] = json!(s);
            }
            r
        };
        let timeout = Duration::from_secs(30);
        let resp = match proc.control(request(reattach.as_ref()), timeout).await {
            Ok(r) => r,
            // The previous remote session is gone: open a new one.
            Err(_) if reattach.is_some() && proc.is_alive() => {
                proc.control(request(None), timeout).await?
            }
            Err(e) => return Err(e),
        };
        {
            let mut rt = h.lock();
            rt.remote_linked = true;
            rt.meta.remote_session = resp["bridge_session_id"].as_str().map(str::to_string);
            rt.meta.remote_url = resp["session_url"].as_str().map(str::to_string);
        }
        self.request_save();
        self.emit_agent(h);
        Ok(())
    }

    /// Turns Remote Control on (the agent's process starts if needed and stays up) or off.
    pub async fn set_remote_control(self: &Arc<Self>, id: &str, enabled: bool) -> Result<()> {
        let h = self.agent(id)?;
        h.lock().meta.remote_control = enabled;
        self.request_save();
        if enabled {
            let linked = match self.ensure_process(id).await {
                Ok(_) if h.lock().remote_linked => Ok(()),
                Ok(proc) => self.link_remote(&h, &proc).await,
                Err(e) => Err(e),
            };
            if let Err(e) = linked {
                h.lock().meta.remote_control = false;
                self.emit_agent(&h);
                return Err(e.context("Remote control indisponible"));
            }
        } else {
            let proc = {
                let mut rt = h.lock();
                let linked = std::mem::take(&mut rt.remote_linked);
                rt.meta.remote_session = None;
                rt.meta.remote_url = None;
                rt.remote_state = None;
                rt.proc.clone().filter(|_| linked)
            };
            if let Some(p) = proc {
                p.control(
                    json!({ "subtype": "remote_control", "enabled": false }),
                    Duration::from_secs(15),
                )
                .await?;
            }
        }
        self.emit_agent(&h);
        Ok(())
    }

    /// Starts the Remote Control agents with the app, so they are reachable from claude.ai.
    pub fn start_remote_agents(self: &Arc<Self>) {
        let ids: Vec<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                (rt.meta.remote_control && !rt.meta.archived).then(|| rt.meta.id.clone())
            })
            .collect();
        for id in ids {
            let c = self.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = c.ensure_process(&id).await {
                    log::warn!("agent {id}: remote control start failed: {e:#}");
                }
            });
        }
    }

    /// Stopped by the usage limit: plans to send the agent "continue" once the limit resets (as
    /// Claude Code told, else the saturated window's), unless turned off in the settings.
    pub fn plan_resume(self: &Arc<Self>, id: &str, resets_at: Option<i64>) {
        if !self.settings.read().auto_resume {
            return;
        }
        // Only a reset still to come: past one, retrying would only meet the limit again.
        let now = now_ms();
        let when = resets_at.filter(|t| *t > now).or_else(|| {
            let u = self.usage.lock();
            [u.five_hour.as_ref(), u.seven_day.as_ref()]
                .into_iter()
                .flatten()
                .filter(|w| w.pct >= 100.0)
                .filter_map(|w| w.resets_at)
                .filter(|t| *t > now)
                .max()
        });
        if let Some(when) = when {
            self.set_resume(id, Some(when + RESUME_MARGIN_MS));
        }
    }

    pub fn cancel_resume(self: &Arc<Self>, id: &str) -> Result<()> {
        self.agent(id)?;
        self.set_resume(id, None);
        Ok(())
    }

    fn set_resume(self: &Arc<Self>, id: &str, at: Option<i64>) {
        let Ok(h) = self.agent(id) else {
            return;
        };
        let view = {
            let mut rt = h.lock();
            rt.meta.resume_at = at;
            rt.view()
        };
        self.hub.emit(UiEvent::Agent { agent: view });
        self.request_save();
    }

    /// Sends "continue" to the agents whose planned resume is due.
    pub async fn resume_due(self: &Arc<Self>) {
        if !self.settings.read().auto_resume {
            return;
        }
        let now = now_ms();
        // Taken under the agent's lock: a message or a cancel just before wins.
        let due: Vec<(String, AgentView)> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let mut rt = h.lock();
                let due = rt.meta.resume_at.is_some_and(|t| t <= now)
                    && !rt.meta.status.is_active()
                    && !rt.meta.archived;
                if !due {
                    return None;
                }
                rt.meta.resume_at = None;
                Some((rt.meta.id.clone(), rt.view()))
            })
            .collect();
        for (id, view) in due {
            self.hub.emit(UiEvent::Agent { agent: view });
            self.request_save();
            if let Err(e) = self.send_message(&id, "continue".into(), vec![]).await {
                log::warn!("agent {id}: resume after the usage limit failed: {e:#}");
                let _ = self.with_agent(&id, |rt, fx| {
                    rt.notice(
                        "warn",
                        format!("Reprise automatique impossible : {e:#}"),
                        fx,
                    );
                    Ok(())
                });
            }
        }
    }

    /// The running Claude processes, with what they and everything they started use.
    pub fn sample_resources(&self) -> resources::Resources {
        let procs: Vec<(String, Arc<ClaudeProcess>)> = self
            .agents
            .read()
            .iter()
            .filter_map(|(id, h)| Some((id.clone(), h.lock().proc.clone()?)))
            .filter(|(_, p)| p.is_alive())
            .collect();
        let mut usages: Vec<(String, JobUsage)> = procs
            .into_iter()
            .filter_map(|(id, p)| Some((id, p.usage()?)))
            .collect();
        usages.sort_by(|a, b| a.0.cmp(&b.0));
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        self.resources.lock().sample(Instant::now(), cores, usages)
    }

    /// Stops the processes of agents idle for longer than the configured delay. The session
    /// and the conversation stay: the next action on the agent resumes it (--resume).
    pub(crate) fn stop_idle_processes(&self) {
        let minutes = self.settings.read().idle_stop_minutes;
        if minutes == 0 {
            return;
        }
        let limit = now_ms() - minutes as i64 * 60_000;
        let agents: Vec<AgentHandle> = self.agents.read().values().cloned().collect();
        for h in agents {
            let stopped = {
                let mut rt = h.lock();
                // A Remote Control agent must stay reachable from claude.ai.
                let idle = rt.proc.is_some()
                    && !rt.meta.remote_control
                    && !rt.meta.status.is_active()
                    && rt.meta.last_activity < limit;
                if idle {
                    rt.detach()
                } else {
                    None
                }
            };
            if let Some(p) = stopped {
                p.close_input();
                self.emit_agent(&h);
            }
        }
    }

    /// A window that subscribes (started, or reloaded and so without the files it held) has no unsaved file yet.
    pub fn reset_unsaved(&self) {
        self.unsaved.store(0, Ordering::Release);
    }

    /// "Quitter" goes to the window first when the editor has unsaved files: true when it did.
    pub fn ask_before_quit(&self) -> bool {
        let unsaved = self.unsaved.load(Ordering::Acquire);
        if unsaved == 0 {
            return false;
        }
        self.hub.emit(UiEvent::QuitRequested { unsaved });
        true
    }

    pub fn shutdown(&self) {
        self.quitting.store(true, Ordering::Release);
        for h in self.agents.read().values() {
            let mut rt = h.lock();
            if let Some(p) = rt.proc.take() {
                p.close_input();
                p.kill();
            }
        }
        self.pty.kill_all();
        self.save_now();
    }

    pub async fn send_message(
        self: &Arc<Self>,
        id: &str,
        text: String,
        attachments: Vec<Attachment>,
    ) -> Result<()> {
        // Refused before starting Claude: the composer keeps the message.
        let content = user_content(&text, &attachments)?;
        // Two attempts: the process may die between being started and receiving the message.
        for attempt in 0..2 {
            let proc = self.ensure_process(id).await?;
            if self.deliver(id, &proc, &text, &content, &attachments)? {
                return Ok(());
            }
            log::warn!("message not delivered (attempt {attempt}): the process exited");
        }
        bail!("Claude Code s'est arrêté pendant l'envoi du message : voir le détail dans la conversation.")
    }

    /// Sends the message to `proc` if it is still the agent's live process, then records it.
    fn deliver(
        self: &Arc<Self>,
        id: &str,
        proc: &Arc<ClaudeProcess>,
        text: &str,
        content: &Value,
        attachments: &[Attachment],
    ) -> Result<bool> {
        let h = self.agent(id)?;
        let mut fx = Effects::default();
        let (pid, name, view, first) = {
            let mut rt = h.lock();
            if !rt.proc.as_ref().is_some_and(|p| Arc::ptr_eq(p, proc)) || !proc.is_alive() {
                return Ok(false);
            }
            let uid = uuid::Uuid::new_v4().to_string();
            let frame = json!({ "type": "user", "message": { "role": "user", "content": content }, "parent_tool_use_id": null, "uuid": uid });
            if proc.send(&frame).is_err() {
                return Ok(false);
            }
            let (images, files): (Vec<_>, Vec<_>) = attachments.iter().partition(|a| a.is_image());
            let files: Vec<String> = files.into_iter().map(|a| a.name.clone()).collect();
            rt.push_user(&uid, text, images.len() as u32, &files, &mut fx);
            let first = !rt.meta.named && rt.meta.prompts == 1;
            (
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.view(),
                first,
            )
        };
        self.stats.record_prompt(id, &pid);
        self.apply(id, &pid, &name, fx, Some(view));
        if first && !text.trim().is_empty() && !text.trim_start().starts_with('/') {
            let (c, id, text) = (self.clone(), id.to_string(), text.to_string());
            tauri::async_runtime::spawn(async move { c.auto_name(&id, &text).await });
        }
        Ok(true)
    }

    pub async fn interrupt(self: &Arc<Self>, id: &str) -> Result<()> {
        let h = self.agent(id)?;
        let proc = {
            let mut rt = h.lock();
            // Only a running turn can be interrupted; a stale flag would hide the next turn's end.
            let active = rt.meta.status.is_active();
            rt.interrupted = active;
            rt.proc.clone().filter(|_| active)
        };
        if let Some(p) = proc {
            if let Err(e) = p
                .control(json!({ "subtype": "interrupt" }), Duration::from_secs(15))
                .await
            {
                h.lock().interrupted = false;
                return Err(e);
            }
        }
        Ok(())
    }

    fn with_agent<T>(
        self: &Arc<Self>,
        id: &str,
        f: impl FnOnce(&mut AgentRt, &mut Effects) -> Result<T>,
    ) -> Result<T> {
        let h = self.agent(id)?;
        let mut fx = Effects::default();
        let (out, pid, name, view) = {
            let mut rt = h.lock();
            let out = f(&mut rt, &mut fx)?;
            (
                out,
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.view(),
            )
        };
        self.apply(id, &pid, &name, fx, Some(view));
        Ok(out)
    }

    pub fn answer_question(
        self: &Arc<Self>,
        id: &str,
        request_id: &str,
        answers: Value,
    ) -> Result<()> {
        self.with_agent(id, |rt, fx| rt.answer_question(request_id, answers, fx))
    }

    pub fn answer_permission(
        self: &Arc<Self>,
        id: &str,
        request_id: &str,
        decision: &str,
        message: Option<String>,
    ) -> Result<()> {
        self.with_agent(id, |rt, fx| {
            rt.answer_permission(request_id, decision, message, fx)
        })
    }

    pub async fn set_agent_options(
        self: &Arc<Self>,
        id: &str,
        model: Option<String>,
        effort: Option<String>,
        mode: Option<String>,
    ) -> Result<()> {
        let h = self.agent(id)?;
        let proc = {
            let mut rt = h.lock();
            if let Some(m) = &model {
                rt.meta.model = m.clone();
            }
            if let Some(e) = &effort {
                rt.meta.effort = e.clone();
            }
            if let Some(m) = &mode {
                rt.meta.mode = m.clone();
            }
            rt.proc.clone()
        };
        self.emit_agent(&h);
        self.request_save();
        if let Some(p) = proc {
            let t = Duration::from_secs(15);
            if let Some(m) = model {
                p.control(json!({ "subtype": "set_model", "model": m }), t)
                    .await?;
            }
            let current_model = h.lock().meta.model.clone();
            if let Some(e) = effort.filter(|_| supports_effort(&current_model)) {
                p.control(
                    json!({ "subtype": "apply_flag_settings", "settings": { "effortLevel": e } }),
                    t,
                )
                .await?;
            }
            if let Some(m) = mode {
                p.control(json!({ "subtype": "set_permission_mode", "mode": m }), t)
                    .await?;
            }
        }
        Ok(())
    }

    // ---------- agents lifecycle ----------

    pub async fn create_agent(
        self: &Arc<Self>,
        project_id: &str,
        model: Option<String>,
    ) -> Result<AgentView> {
        let _creating = self.create_lock.lock().await;
        let project = self.project(project_id)?;
        let settings = self.settings.read().clone();
        let existing: Vec<String> = self
            .project_agents(project_id)
            .iter()
            .map(|h| h.lock().meta.name.clone())
            .collect();
        let mut n = existing.len() + 1;
        while existing.iter().any(|e| *e == format!("agent-{n}")) {
            n += 1;
        }
        let name = format!("agent-{n}");
        let mut meta = AgentMeta {
            id: new_id(),
            project_id: project_id.to_string(),
            name: name.clone(),
            model: model.unwrap_or(settings.default_model.clone()),
            effort: settings.default_effort.clone(),
            mode: settings.default_mode.clone(),
            cwd: project.path.clone(),
            created_at: now_ms(),
            last_activity: now_ms(),
            ..Default::default()
        };
        let mut warning = None;
        if project.worktree_per_agent {
            match git::worktree_add(&project.path, &name).await {
                Ok((path, branch, base)) => {
                    meta.cwd = path.clone();
                    meta.worktree = Some(Worktree {
                        path,
                        branch,
                        base_branch: base,
                    });
                }
                Err(e) => {
                    warning = Some(format!(
                        "Worktree non créé, l'agent travaille dans le dossier du projet : {e}"
                    ))
                }
            }
        }
        let id = meta.id.clone();
        let h = Arc::new(Mutex::new(AgentRt::new(meta, &self.data.conversations())));
        if let Some(w) = warning {
            let mut fx = Effects::default();
            h.lock().notice("warn", w, &mut fx);
        }
        self.agents.write().insert(id.clone(), h.clone());
        self.ui
            .write()
            .selected_agent
            .insert(project_id.to_string(), id.clone());
        self.request_save();
        self.emit_agent(&h);
        self.git.refresh(project_id);
        self.warm(&id);
        let view = h.lock().view();
        Ok(view)
    }

    async fn auto_name(self: &Arc<Self>, id: &str, prompt: &str) {
        match self.generate_name(prompt).await {
            Ok(Some(slug)) => {
                if let Err(e) = self.apply_generated_name(id, &slug).await {
                    log::warn!("auto-naming failed: {e:#}");
                }
            }
            Ok(None) => {
                log::info!("agent {id}: no usable name from the model, keeping the default one")
            }
            Err(e) => log::warn!("auto-naming failed: {e:#}"),
        }
    }

    /// A short name for the task, or None when the model did not answer with one.
    async fn generate_name(&self, prompt: &str) -> Result<Option<String>> {
        use tokio::io::AsyncWriteExt;
        let settings = self.settings.read().clone();
        let program =
            claude::resolve_binary(&settings.claude_path).context("claude introuvable")?;
        let mut cmd = tokio::process::Command::new(program);
        cmd.args([
            "-p",
            "--model",
            "haiku",
            "--output-format",
            "json",
            "--no-session-persistence",
            "--tools",
            "",
            "--setting-sources",
            "",
            // No MCP servers (account connectors included): nothing that invites the model to act.
            "--strict-mcp-config",
            "--system-prompt",
            "Tu nommes des tâches de développement sans jamais les réaliser. Tu réponds uniquement par un slug.",
        ])
        .current_dir(std::env::temp_dir())
        .envs(settings.proxy_env())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(claude::CREATE_NO_WINDOW);
        let mut child = cmd.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(naming_prompt(prompt).as_bytes()).await?;
        }
        let out = tokio::time::timeout(Duration::from_secs(90), child.wait_with_output()).await??;
        let v: Value = serde_json::from_slice(&out.stdout)?;
        Ok(name_from_answer(v["result"].as_str().unwrap_or("")))
    }

    async fn apply_generated_name(self: &Arc<Self>, id: &str, slug: &str) -> Result<()> {
        let h = self.agent(id)?;
        let project_id = h.lock().meta.project_id.clone();
        let taken: Vec<String> = self
            .project_agents(&project_id)
            .iter()
            .filter(|a| a.lock().meta.id != id)
            .map(|a| a.lock().meta.name.clone())
            .collect();
        let mut name = slug.to_string();
        let mut n = 2;
        while taken.contains(&name) {
            name = format!("{slug}-{n}");
            n += 1;
        }
        let (worktree, proc) = {
            let mut rt = h.lock();
            if rt.meta.named {
                return Ok(());
            }
            rt.meta.name = name.clone();
            rt.meta.named = true;
            (rt.meta.worktree.clone(), rt.proc.clone())
        };
        self.emit_agent(&h);
        self.request_save();
        if let Some(wt) = worktree {
            let project = self.project(&project_id)?;
            let branch = format!("{}{name}", paths::BRANCH_PREFIX);
            if !git::branch_exists(&project.path, &branch).await
                && git::rename_current_branch(&wt.path, &branch).await.is_ok()
            {
                if let Some(w) = h.lock().meta.worktree.as_mut() {
                    w.branch = branch;
                }
                self.request_save();
            }
        }
        if let Some(p) = proc {
            let _ = p
                .control(
                    json!({ "subtype": "rename_session", "title": name, "source": "host" }),
                    Duration::from_secs(10),
                )
                .await;
        }
        Ok(())
    }

    pub async fn rename_agent(self: &Arc<Self>, id: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            bail!("nom vide");
        }
        let h = self.agent(id)?;
        let proc = {
            let mut rt = h.lock();
            rt.meta.name = name.to_string();
            rt.meta.named = true;
            rt.proc.clone()
        };
        self.emit_agent(&h);
        self.request_save();
        if let Some(p) = proc {
            let _ = p
                .control(
                    json!({ "subtype": "rename_session", "title": name, "source": "host" }),
                    Duration::from_secs(10),
                )
                .await;
        }
        Ok(())
    }

    pub async fn archive_agent(self: &Arc<Self>, id: &str, archived: bool) -> Result<()> {
        if archived {
            // An archived agent is no longer reachable from claude.ai.
            if self.agent(id)?.lock().meta.remote_control {
                let _ = self.set_remote_control(id, false).await;
            }
            // Stop the current turn first: closing stdin alone lets it run to completion.
            let running = {
                let h = self.agent(id)?;
                let rt = h.lock();
                rt.proc.clone().filter(|_| rt.meta.status.is_active())
            };
            if let Some(p) = running {
                let _ = p
                    .control(
                        json!({ "subtype": "interrupt", "cancel_queued": true }),
                        Duration::from_secs(5),
                    )
                    .await;
            }
        }
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        self.with_agent(id, |rt, fx| {
            rt.meta.archived = archived;
            if archived {
                rt.meta.resume_at = None;
                if let Some(p) = rt.detach() {
                    p.close_input();
                }
                rt.clear_pending(fx);
                if rt.meta.status.is_active() {
                    rt.set_status(AgentStatus::Done, fx);
                }
            }
            fx.save = true;
            Ok(())
        })?;
        let pid = self.agent(id)?.lock().meta.project_id.clone();
        self.git.refresh(&pid);
        Ok(())
    }

    /// Removes the agent. Worktree cleanup is best effort: a problem there is returned as a
    /// warning, the agent is removed regardless.
    pub async fn delete_agent(
        self: &Arc<Self>,
        id: &str,
        remove_worktree: bool,
    ) -> Result<Option<String>> {
        // End its claude.ai session rather than leave it behind.
        if self.agent(id)?.lock().meta.remote_control {
            let _ = self.set_remote_control(id, false).await;
        }
        // Wait for an in-flight start (warm-up) so that its process is killed too.
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        let h = self
            .agents
            .write()
            .remove(id)
            .ok_or_else(|| anyhow!("agent introuvable"))?;
        let (pid, worktree) = {
            let mut rt = h.lock();
            rt.gen += 1;
            if let Some(p) = rt.proc.take() {
                p.kill();
            }
            rt.conv.delete_file();
            (rt.meta.project_id.clone(), rt.meta.worktree.clone())
        };
        self.spawn_locks.lock().remove(id);
        {
            let mut ui = self.ui.write();
            if ui.selected_agent.get(&pid).map(String::as_str) == Some(id) {
                ui.selected_agent.remove(&pid);
            }
        }
        self.hub.emit(UiEvent::AgentRemoved {
            id: id.to_string(),
            project_id: pid.clone(),
        });
        self.request_save();
        self.update_tray();
        let mut warning = None;
        if let (true, Some(wt), Ok(project)) = (remove_worktree, worktree, self.project(&pid)) {
            // Give the killed process tree a moment to release its handles on the worktree.
            tokio::time::sleep(Duration::from_millis(300)).await;
            if let Err(e) = git::worktree_remove(&project.path, &wt.path, &wt.branch).await {
                warning = Some(format!(
                    "Agent supprimé, mais le worktree n'a pas pu être nettoyé : {e:#}"
                ));
            }
        }
        self.git.refresh(&pid);
        Ok(warning)
    }

    pub async fn merge_agent(self: &Arc<Self>, id: &str, squash: bool) -> Result<String> {
        let h = self.agent(id)?;
        let (pid, name, wt) = {
            let rt = h.lock();
            (
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.meta.worktree.clone(),
            )
        };
        let wt = wt.ok_or_else(|| anyhow!("cet agent n'a pas de worktree"))?;
        let project = self.project(&pid)?;
        if git::has_tracked_changes(&project.path).await? {
            bail!("Le dépôt principal a des modifications non commitées : commite-les ou mets-les de côté avant de merger.");
        }
        let dirty = git::status(&wt.path).await?.entries.len();
        if dirty > 0 {
            bail!("L'agent a {dirty} fichier(s) non commité(s) : demande-lui de commiter avant de merger.");
        }
        if git::ahead_count(&project.path, &wt.branch).await == 0 {
            bail!(
                "Rien à merger : la branche {} n'a pas de nouveau commit.",
                wt.branch
            );
        }
        let message = if squash {
            let subjects = git::text(
                &project.path,
                &["log", "--format=- %s", &format!("HEAD..{}", wt.branch)],
            )
            .await
            .unwrap_or_default();
            format!("{name}\n\n{subjects}")
        } else {
            format!("Merge branch '{}'", wt.branch)
        };
        let out = git::merge(&project.path, &wt.branch, squash, &message).await?;
        self.git.refresh(&pid);
        Ok(out)
    }

    // ---------- projects ----------

    pub async fn create_project(
        self: &Arc<Self>,
        path: &str,
        name: &str,
        color: &str,
        worktree_per_agent: bool,
        first_agent: Option<String>,
    ) -> Result<Project> {
        let path = path.trim().trim_end_matches(['\\', '/']).to_string();
        if !Path::new(&path).is_dir() {
            bail!("Le dossier {path} n'existe pas");
        }
        if git::toplevel(&path).await.is_none() {
            git::init_repo(&path).await.context("git init")?;
        }
        let project = Project {
            id: new_id(),
            name: if name.trim().is_empty() {
                Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "projet".into())
            } else {
                name.trim().to_string()
            },
            path,
            color: color.to_string(),
            worktree_per_agent,
            created_at: now_ms(),
            run_commands: Vec::new(),
            board: BoardSettings::default(),
            worktree_copy: default_worktree_copy(),
        };
        self.projects.write().push(project.clone());
        {
            let mut ui = self.ui.write();
            ui.active_project = Some(project.id.clone());
            ui.view = "project".into();
        }
        self.request_save();
        self.git.watch(&project.id, &project.path);
        if let Some(model) = first_agent {
            self.create_agent(&project.id, Some(model)).await?;
        }
        Ok(project)
    }

    pub fn update_project(&self, p: Project) -> Result<()> {
        let mut projects = self.projects.write();
        let cur = projects
            .iter_mut()
            .find(|x| x.id == p.id)
            .ok_or_else(|| anyhow!("projet introuvable"))?;
        cur.name = p.name;
        cur.color = p.color;
        cur.worktree_per_agent = p.worktree_per_agent;
        cur.run_commands = p.run_commands;
        cur.worktree_copy = p.worktree_copy;
        drop(projects);
        self.request_save();
        Ok(())
    }

    pub fn reorder_projects(&self, ids: &[String]) {
        let mut projects = self.projects.write();
        projects.sort_by_key(|p| ids.iter().position(|i| *i == p.id).unwrap_or(usize::MAX));
        drop(projects);
        self.request_save();
    }

    pub fn remove_project(self: &Arc<Self>, id: &str) -> Result<()> {
        let agents: Vec<String> = self
            .project_agents(id)
            .iter()
            .map(|h| h.lock().meta.id.clone())
            .collect();
        for aid in agents {
            // Bound first: the map guard must not live across the agent lock and the I/O below.
            let removed = self.agents.write().remove(&aid);
            if let Some(h) = removed {
                let mut rt = h.lock();
                rt.gen += 1;
                if let Some(p) = rt.proc.take() {
                    p.kill();
                }
                rt.conv.delete_file();
            }
            self.hub.emit(UiEvent::AgentRemoved {
                id: aid,
                project_id: id.to_string(),
            });
        }
        self.pty.kill_project(id);
        self.git.unwatch(id);
        self.projects.write().retain(|p| p.id != id);
        // Read before taking the ui lock: never hold ui while waiting on projects.
        let fallback = self.projects.read().first().map(|p| p.id.clone());
        {
            let mut ui = self.ui.write();
            ui.selected_agent.remove(id);
            if ui.active_project.as_deref() == Some(id) {
                ui.active_project = fallback;
            }
        }
        self.request_save();
        self.update_tray();
        Ok(())
    }

    // ---------- git ----------

    async fn git_loop(self: Arc<Self>, mut rx: mpsc::UnboundedReceiver<String>) {
        let mut due: HashMap<String, Instant> = HashMap::new();
        let mut last: HashMap<String, Instant> = HashMap::new();
        loop {
            let next = due.values().min().copied();
            tokio::select! {
                msg = rx.recv() => {
                    let Some(pid) = msg else { break };
                    let now = Instant::now();
                    let earliest = last.get(&pid).map(|t| *t + Duration::from_millis(700)).unwrap_or(now);
                    due.entry(pid).or_insert_with(|| earliest.max(now + Duration::from_millis(150)));
                }
                _ = tokio::time::sleep_until(next.unwrap_or_else(Instant::now).into()), if next.is_some() => {
                    let now = Instant::now();
                    let ready: Vec<String> = due.iter().filter(|(_, t)| **t <= now).map(|(k, _)| k.clone()).collect();
                    for pid in ready {
                        // A refresh still running for this project (large repo): try again later
                        // rather than overlapping and publishing results out of order.
                        if !self.git_inflight.lock().insert(pid.clone()) {
                            due.insert(pid, now + Duration::from_millis(250));
                            continue;
                        }
                        due.remove(&pid);
                        last.insert(pid.clone(), now);
                        self.git.take_flag(&pid);
                        let c = self.clone();
                        tauri::async_runtime::spawn(async move {
                            c.compute_git(&pid).await;
                            c.git_inflight.lock().remove(&pid);
                        });
                    }
                }
            }
        }
    }

    pub(crate) async fn compute_git(self: &Arc<Self>, project_id: &str) {
        let Ok(project) = self.project(project_id) else {
            return;
        };
        let info = match self.toplevel(&project.path).await {
            None => GitInfo::default(),
            Some(root) => {
                let (st, remotes) = tokio::join!(git::status(&root), git::remotes(&root));
                let st = st.unwrap_or_default();
                let prefix = sub_prefix(&root, &project.path);
                let mut info = GitInfo {
                    is_repo: true,
                    branch: st.branch.clone(),
                    upstream: st.upstream.clone(),
                    upstream_gone: st.upstream_gone,
                    ahead: st.ahead,
                    behind: st.behind,
                    has_remote: !remotes.is_empty(),
                    last_fetch: git::last_fetch(&root),
                    ..Default::default()
                };
                let tally = |c: char, info: &mut GitInfo| match c {
                    'A' => info.added += 1,
                    'D' => info.deleted += 1,
                    _ => info.modified += 1,
                };
                for e in &st.entries {
                    tally(e.status, &mut info);
                }
                let agents: Vec<(String, Option<Worktree>, Vec<String>)> = self
                    .project_agents(project_id)
                    .iter()
                    .map(|h| {
                        let rt = h.lock();
                        (
                            rt.meta.id.clone(),
                            rt.meta.worktree.clone(),
                            rt.meta.touched_files.clone(),
                        )
                    })
                    .collect();
                for (aid, wt, touched) in agents {
                    let count = match wt {
                        Some(wt) => match git::status(&wt.path).await {
                            Ok(ws) => {
                                for e in &ws.entries {
                                    tally(e.status, &mut info);
                                }
                                ws.entries.len() as u32
                            }
                            Err(_) => 0,
                        },
                        None => touched
                            .iter()
                            .filter(|t| {
                                let full = format!("{prefix}{t}");
                                st.entries
                                    .iter()
                                    .any(|e| e.path.eq_ignore_ascii_case(&full))
                            })
                            .count() as u32,
                    };
                    info.agents.insert(aid, count);
                }
                info.total = info.modified + info.added + info.deleted;
                info
            }
        };
        self.git.invalidate_files();
        self.git_cache
            .write()
            .insert(project_id.to_string(), info.clone());
        self.hub.emit(UiEvent::Git {
            project_id: project_id.to_string(),
            git: info,
        });
    }

    /// Dirty files for the files panel. `agent_id` + scope "agent" restricts to one agent.
    pub async fn git_files(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<Vec<FileChange>> {
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(|| anyhow!("pas un dépôt git"))?;
        let prefix = sub_prefix(&root, &project.path);
        let agents: Vec<(String, Option<Worktree>, Vec<String>)> = self
            .project_agents(project_id)
            .iter()
            .map(|h| {
                let rt = h.lock();
                (
                    rt.meta.id.clone(),
                    rt.meta.worktree.clone(),
                    rt.meta.touched_files.clone(),
                )
            })
            .filter(|(id, _, _)| agent_id.as_ref().is_none_or(|a| a == id))
            .collect();
        let mut out = Vec::new();
        let main = if agents.iter().all(|(_, wt, _)| wt.is_some()) && agent_id.is_some() {
            Vec::new()
        } else {
            git::file_changes(&root).await?
        };
        let owner = |path: &str| {
            agents
                .iter()
                .find(|(_, wt, touched)| {
                    wt.is_none()
                        && touched
                            .iter()
                            .any(|t| format!("{prefix}{t}").eq_ignore_ascii_case(path))
                })
                .map(|(id, _, _)| id.clone())
        };
        for mut f in main {
            f.agent_id = owner(&f.path);
            if agent_id.is_none() || f.agent_id.is_some() {
                out.push(f);
            }
        }
        for (id, wt, _) in &agents {
            if let Some(wt) = wt {
                for mut f in git::file_changes(&wt.path).await.unwrap_or_default() {
                    f.agent_id = Some(id.clone());
                    f.in_worktree = true;
                    out.push(f);
                }
            }
        }
        Ok(out)
    }

    /// The checkout holding the files listed for `agent_id` (its worktree), else the project's
    /// repository: the files panel's paths are relative to it.
    async fn files_root(&self, project_id: &str, agent_id: Option<String>) -> Result<String> {
        let project = self.project(project_id)?;
        let worktree = match &agent_id {
            Some(a) => self.agent(a)?.lock().meta.worktree.clone(),
            None => None,
        };
        match worktree {
            Some(wt) => Ok(wt.path),
            None => self
                .toplevel(&project.path)
                .await
                .ok_or_else(|| anyhow!("pas un dépôt git")),
        }
    }

    /// The folder the editor shows for `agent_id`: its worktree (with the branch it left), else
    /// the project's repository, else the project's folder when it is not one.
    pub async fn edit_root(
        &self,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<(String, Option<String>)> {
        let project = self.project(project_id)?;
        if let Some(a) = &agent_id {
            let worktree = self.agent(a)?.lock().meta.worktree.clone();
            if let Some(wt) = worktree {
                return Ok((wt.path, Some(wt.base_branch)));
            }
        }
        let root = self.toplevel(&project.path).await.unwrap_or(project.path);
        Ok((root, None))
    }

    pub async fn git_diff(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        paths: Vec<String>,
    ) -> Result<String> {
        let root = self.files_root(project_id, agent_id).await?;
        git::diff(&root, &paths).await
    }

    /// Reverts a file of the files panel to HEAD (a new file is deleted).
    pub async fn git_discard(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        path: &str,
    ) -> Result<()> {
        let root = self.files_root(project_id, agent_id).await?;
        git::discard(&root, path).await?;
        self.git.refresh(project_id);
        Ok(())
    }

    /// The repository graph (every branch, agents' worktree branches included) and the branch
    /// the agent works on.
    pub async fn git_log(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<GitLog> {
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(|| anyhow!("pas un dépôt git"))?;
        let worktree = match &agent_id {
            Some(a) => self.agent(a)?.lock().meta.worktree.clone(),
            None => None,
        };
        let head = match worktree {
            Some(wt) => Some(wt.branch),
            None => Some(git::current_branch(&root).await).filter(|b| !b.is_empty() && b != "HEAD"),
        };
        Ok(GitLog {
            commits: git::log(&root, 300).await?,
            head,
        })
    }

    pub async fn git_show(self: &Arc<Self>, project_id: &str, hash: &str) -> Result<String> {
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(|| anyhow!("pas un dépôt git"))?;
        git::show(&root, hash).await
    }

    fn sync_lock(&self, root: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.sync_locks
            .lock()
            .entry(root.to_string())
            .or_default()
            .clone()
    }

    /// Refreshes the git state of every project in the repository at `root`.
    async fn refresh_repo(&self, root: &str) {
        let projects: Vec<(String, String)> = self
            .projects
            .read()
            .iter()
            .map(|p| (p.id.clone(), p.path.clone()))
            .collect();
        for (id, path) in projects {
            if self.toplevel(&path).await.as_deref() == Some(root) {
                self.git.refresh(&id);
            }
        }
    }

    /// Fetches every project's repository that has a remote, one at a time and without ever
    /// asking for credentials, so that the commits to pull show up.
    pub(crate) async fn fetch_all(self: &Arc<Self>) {
        let paths: Vec<String> = self
            .projects
            .read()
            .iter()
            .map(|p| p.path.clone())
            .collect();
        let mut roots: Vec<String> = Vec::new();
        for path in paths {
            match self.toplevel(&path).await {
                Some(root) if !roots.contains(&root) => roots.push(root),
                _ => {}
            }
        }
        for root in roots {
            if git::remotes(&root).await.is_empty() {
                continue;
            }
            let lock = self.sync_lock(&root);
            // The user's own fetch, pull or push is running: no need for another one.
            let Ok(_guard) = lock.try_lock() else {
                continue;
            };
            match git::fetch(&root, true).await {
                Ok(()) => self.refresh_repo(&root).await,
                Err(e) => log::info!("background fetch of {root} failed: {e:#}"),
            }
        }
    }

    /// Fetch, pull or push of the project's main checkout, asked by the user (so credentials
    /// may be asked for). Returns a summary for the user.
    pub async fn git_sync(self: &Arc<Self>, project_id: &str, op: SyncOp) -> Result<String> {
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(|| anyhow!("pas un dépôt git"))?;
        let lock = self.sync_lock(&root);
        let out = {
            let _guard = lock.lock().await;
            match op {
                SyncOp::Fetch => match git::fetch(&root, false).await {
                    Ok(()) => git::status(&root).await.map(|st| git::fetch_summary(&st)),
                    Err(e) => Err(e),
                },
                SyncOp::Pull => git::pull(&root).await,
                SyncOp::Push => git::push(&root).await,
            }
        };
        // Even after a failure: a pull that could not fast-forward has fetched.
        self.refresh_repo(&root).await;
        out
    }

    pub async fn file_suggestions(
        self: &Arc<Self>,
        agent_id: &str,
        query: &str,
    ) -> Result<Vec<String>> {
        let cwd = self.agent(agent_id)?.lock().meta.cwd.clone();
        let files = self.git.file_index(&cwd).await;
        Ok(git::fuzzy_files(&files, query, 40))
    }

    // ---------- usage ----------

    pub async fn refresh_usage(self: &Arc<Self>) {
        let proc = self
            .agents
            .read()
            .values()
            .find_map(|h| h.lock().proc.clone());
        let mut windows = None;
        if let Some(p) = proc {
            if let Ok(v) = p
                .control(
                    json!({ "subtype": "get_usage", "skip_behaviors": true }),
                    Duration::from_secs(20),
                )
                .await
            {
                if v["rate_limits"].is_object() {
                    windows = Some(usage::parse_windows(&v["rate_limits"]));
                }
            }
        }
        if windows.is_none() && usage::oauth_due(*self.last_oauth_call.lock(), now_ms()) {
            *self.last_oauth_call.lock() = Some(now_ms());
            let settings = self.settings.read().clone();
            match usage::fetch_oauth(&settings).await {
                Ok(w) => windows = Some(w),
                Err(e) => log::debug!("usage endpoint: {e:#}"),
            }
        }
        let snapshot = {
            let mut u = self.usage.lock();
            if let Some((five, week)) = windows {
                u.five_hour = five.or(u.five_hour);
                u.seven_day = week.or(u.seven_day);
                u.updated_at = now_ms();
            }
            u.today_cost = self.stats.today_cost();
            u.clone()
        };
        self.hub.emit(UiEvent::Usage { usage: snapshot });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_taken_only_from_slug_like_answers() {
        assert_eq!(
            name_from_answer("refacto-auth"),
            Some("refacto-auth".into())
        );
        assert_eq!(
            name_from_answer("  `tests-e2e`\n"),
            Some("tests-e2e".into())
        );
        assert_eq!(
            name_from_answer("Migration JWT"),
            Some("migration-jwt".into())
        );
        // The model sometimes answers the task instead of naming it: never a name.
        assert_eq!(
            name_from_answer("Je n'ai accès qu'aux outils Claude Docs."),
            None
        );
        assert_eq!(name_from_answer("Voici le slug : creation-fichier"), None);
        assert_eq!(name_from_answer(""), None);
    }

    #[test]
    fn the_task_is_framed_as_text_to_name() {
        let p = naming_prompt("Crée un fichier hello.txt");
        assert!(p.contains("<tache>\nCrée un fichier hello.txt\n</tache>"));
        assert!(p.contains("slug"));
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("Migration JWT rotation"), "migration-jwt-rotation");
        assert_eq!(
            slugify("  réparer l'écran d'accueil! "),
            "reparer-l-ecran-d-accueil"
        );
        assert!(slugify(&"mot ".repeat(30)).len() <= 40);
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
    fn sub_prefixes() {
        assert_eq!(sub_prefix("C:/code/app", "C:\\code\\app"), "");
        assert_eq!(
            sub_prefix("C:/code/mono", "C:/code/mono/packages/web"),
            "packages/web/"
        );
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
