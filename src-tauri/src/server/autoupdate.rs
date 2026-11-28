//! The server updates itself (ESC-702, docs/REMOTE.md « Le mode serveur », « La mise à jour automatique »): a running
//! `escouade-server` looks for a version of its channel as the app does, and puts it in place by the very functions of
//! `escouade-server update` (`update::read_target`, `update::download`, `update::install`: never a path of its own),
//! then stops at rest for systemd to start it again on the new files.
//!
//! - **When**: [`FIRST`] after its start (the agents resumed, the plugins loaded first), then every [`EVERY`]; at once
//!   when its switches change, when a client of a newer version is refused (`remoteapi::server`) or when an
//!   administrator asks for it ([`ServerUpdate::ask`]).
//! - **Its channel and its switch** are the server's settings, those of the app: `autoUpdate` (« Installer les mises à
//!   jour automatiquement », on by default as in the app) and `betaUpdates` (« Recevoir les versions bêta », off by
//!   default; `install.sh --version X.Y.Z-beta.N` and `update --beta` turn it on). The policies of the machine and of a
//!   team bring them within theirs (`updates.automatic`, `updates.channel`), as for the app: [`choice_of`].
//!   `escouade-server config` sets them ([`configure`]), and a client sets them from Réglages › Général of the space.
//! - **Never older** than the program (`update::target_of`), never a version whose signature was refused again by
//!   itself, never one that did not take after its restart (the program still at its version: a loop of restarts).
//! - **At rest only**: the version put in place, it waits for `updates::busy` to say nothing (no turn, no question, no
//!   background task, no launch, no terminal served lately), then stops as at SIGTERM: [`ServerUpdate::restarting`].
//! - **In the image** (`ESCOUADE_SERVER_IMAGE`), nothing is replaced: it says that a new image exists, with its
//!   `docker pull`. Without systemd, or off Linux, it says the version and `escouade-server update`, and changes nothing.
//! - **Its log** says each version found, checked, put in place and started again, and each refusal with its reason;
//!   its state ([`State`]) goes in `server-status.json` and to the clients (`server_update_state`).

use super::status::Supervisor;
use super::update::{self, Asked};
use crate::paths::DataDir;
use anyhow::{anyhow, bail, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long after the server's start it looks for the first time: the agents and the plugins first.
pub const FIRST: Duration = Duration::from_secs(2 * 60);
/// How often it looks then. The app looks every 5 minutes and is refused by a server of another version: a quarter of
/// an hour keeps that gap short, for a JSON of a few hundred bytes (an archive is downloaded only when it is new).
pub const EVERY: Duration = Duration::from_secs(15 * 60);
/// How often the loop looks whether a check is due, whether the server is at rest, whether `config` asked something.
pub const LOOK: Duration = Duration::from_secs(2);
/// In a build of development only, [`FIRST`] and [`EVERY`] in seconds (the test of the systemd service).
pub const EVERY_VAR: &str = "ESCOUADE_SERVER_UPDATE_EVERY";
/// The restart it made, written before it stops: read at the next start, to say it was started again (or that the
/// version did not take).
pub const PENDING: &str = "server-update.json";
/// What `escouade-server config` asks a running server, and its answer.
pub const REQUEST: &str = "server-config.json";
pub const ANSWER: &str = "server-config.answer.json";

/// What the server's settings choose, within the policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Choice {
    /// It puts a version in place and restarts by itself.
    pub automatic: bool,
    /// Its channel is the beta's (`beta.json` too), else the stable's.
    pub beta: bool,
}

/// The settings `settings` within the locks `locks`: `updates.automatic` at false turns the automatic off, and
/// `updates.channel` at `stable` keeps to the stable channel (the app's rule, `Locks::clamp_settings`).
pub fn choice_of(settings: &crate::model::Settings, locks: &crate::locks::Locks) -> Choice {
    use serde_json::Value;
    Choice {
        automatic: settings.auto_update
            && locks.value(crate::locks::AUTO_UPDATE) != Some(&Value::Bool(false)),
        beta: settings.beta_updates && locks.text(crate::locks::CHANNEL) != Some("stable"),
    }
}

/// Where the update stands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// Not looked yet.
    #[default]
    Waiting,
    Checking,
    /// Nothing newer on its channel.
    UpToDate,
    /// A newer version, not put in place: the automatic is off, or it cannot be (no systemd, not Linux).
    Available,
    /// A newer image (in the image): nothing is replaced.
    Image,
    Downloading,
    /// Put in place, checked: it restarts once at rest.
    Ready,
    /// At rest: it stops, for systemd to start it again.
    Restarting,
    /// The last try failed (`error` says why); tried again at the next check.
    Failed,
}

/// What the server says of its update: in `server-status.json`, and to a client (`server_update_state`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// The program's version.
    pub version: String,
    pub phase: Phase,
    /// The version found (newer than the program).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available: Option<String>,
    pub automatic: bool,
    /// `stable` or `beta`.
    pub channel: String,
    /// It runs in the image of Escouade: an update is a new image.
    #[serde(default)]
    pub image: bool,
    /// It may put a version in place itself (Linux, under systemd, not in the image).
    #[serde(default)]
    pub installs: bool,
    /// What to run to update it, when it does not itself: `docker pull …`, or `escouade-server update`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Why the last try failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Why a version ready waits (`status::busy_word`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub busy: Option<String>,
    /// When it last looked (ms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<i64>,
}

/// Where the releases are and how they are checked: those of `escouade-server update`.
#[derive(Clone)]
pub struct Source {
    pub client: reqwest::Client,
    pub base: String,
    pub key: String,
    pub arch: String,
}

/// What the server may do of a version: put it in the folder `dir` and restart (Linux under systemd), or only say it.
#[derive(Debug, Clone)]
pub struct Target {
    /// The folder of the program (`current_exe`'s).
    pub dir: PathBuf,
    pub supervisor: Option<Supervisor>,
    /// Linux and systemd: it puts the version in place and restarts.
    pub installs: bool,
}

impl Target {
    /// This process's: beside its program, under its supervisor.
    pub fn here() -> Result<Self> {
        let supervisor = super::status::supervisor();
        let dir = std::env::current_exe()?
            .canonicalize()?
            .parent()
            .ok_or_else(|| anyhow!("the program has no folder"))?
            .to_path_buf();
        Ok(Self {
            dir,
            supervisor,
            installs: cfg!(target_os = "linux") && supervisor == Some(Supervisor::Systemd),
        })
    }
}

/// The restart the server made, for its next start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub from: String,
    pub to: String,
    pub at: i64,
}

/// The update of a running server.
pub struct ServerUpdate {
    current: String,
    data: DataDir,
    source: Source,
    target: Target,
    state: Mutex<State>,
    /// The versions never tried again by themselves: a signature refused, or a restart that did not take.
    refused: Mutex<HashSet<String>>,
    /// The version an administrator asked for (the client's « Mettre à jour le serveur »).
    asked: Mutex<Option<String>>,
    /// What the log said last of each version, so that it says it once.
    said: Mutex<Option<(Phase, String)>>,
    last: Mutex<Option<Instant>>,
    started: Instant,
    poke: tokio::sync::Notify,
    restart: tokio::sync::Notify,
    first: Duration,
    every: Duration,
    /// It looks by itself: never in a build of development without a release of test (`ESCOUADE_SERVER_UPDATE_URL`),
    /// as the app (ESC-543): the e2e never read the real releases.
    looks: bool,
}

/// A check is due: [`FIRST`] after the start, then every [`EVERY`] after the last one.
pub fn due(
    started: Instant,
    last: Option<Instant>,
    now: Instant,
    first: Duration,
    every: Duration,
) -> bool {
    match last {
        None => now.duration_since(started) >= first,
        Some(at) => now.duration_since(at) >= every,
    }
}

/// [`FIRST`] and [`EVERY`], or `ESCOUADE_SERVER_UPDATE_EVERY` seconds for both in a build of development.
fn periods() -> (Duration, Duration) {
    if cfg!(debug_assertions) {
        if let Some(s) = std::env::var(EVERY_VAR)
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
        {
            let d = Duration::from_secs(s.max(1));
            return (d, d);
        }
    }
    (FIRST, EVERY)
}

/// What runs to put a server in the image at `version`.
pub fn image_command(version: &str) -> String {
    format!(
        "docker pull {}:{version} && docker compose up -d",
        crate::remoteapi::proto::SERVER_IMAGE
    )
}

impl ServerUpdate {
    pub fn new(current: &str, data: DataDir, source: Source, target: Target) -> Arc<Self> {
        let (first, every) = periods();
        Self::with_periods(current, data, source, target, first, every)
    }

    pub fn with_periods(
        current: &str,
        data: DataDir,
        source: Source,
        target: Target,
        first: Duration,
        every: Duration,
    ) -> Arc<Self> {
        let image = target.supervisor == Some(Supervisor::Docker);
        Arc::new(Self {
            current: current.to_string(),
            state: Mutex::new(State {
                version: current.to_string(),
                image,
                installs: target.installs && !image,
                channel: "stable".into(),
                ..State::default()
            }),
            data,
            source,
            target,
            refused: Mutex::new(HashSet::new()),
            asked: Mutex::new(None),
            said: Mutex::new(None),
            last: Mutex::new(None),
            started: Instant::now(),
            poke: tokio::sync::Notify::new(),
            restart: tokio::sync::Notify::new(),
            first,
            every,
            looks: true,
        })
    }

    /// The same, never looking by itself (a build of development without a release of test): an administrator's
    /// request alone reads the releases.
    pub fn without_looking(self: Arc<Self>) -> Arc<Self> {
        match Arc::try_unwrap(self) {
            Ok(mut me) => {
                me.looks = false;
                Arc::new(me)
            }
            Err(me) => me,
        }
    }

    /// What it says of itself.
    pub fn state(&self) -> State {
        self.state.lock().clone()
    }

    /// The choice of the settings, as the state says it.
    pub fn set_choice(&self, choice: Choice) {
        let mut s = self.state.lock();
        let changed = s.automatic != choice.automatic || s.channel != channel_of(choice);
        s.automatic = choice.automatic;
        s.channel = channel_of(choice);
        drop(s);
        // A switch turned: looked at once (an update turned on, the beta's channel chosen).
        if changed && self.last.lock().is_some() {
            *self.last.lock() = None;
            self.poke.notify_one();
        }
    }

    /// Looks at once (a client of a newer version knocked).
    pub fn poke(&self) {
        *self.last.lock() = None;
        self.poke.notify_one();
    }

    /// Waits until it is time to restart: the server stops then, as at SIGTERM.
    pub async fn restarting(&self) {
        self.restart.notified().await
    }

    /// An administrator asks for `version` (the client's « Mettre à jour le serveur »): never older than the program,
    /// never in the image, never without systemd. Answers the version aimed at; the work goes on in the loop.
    pub fn ask(&self, version: &str) -> Result<String> {
        let state = self.state();
        if state.image {
            bail!(tr!(
                "dans l’image Docker, la mise à jour est une nouvelle image : « {} ». Rien n’est changé.",
                "in the Docker image, an update is a new image: “{}”. Nothing is changed.",
                image_command(version.trim().trim_start_matches('v'))
            ));
        }
        if !state.installs {
            bail!(tr!(
                "ce serveur ne tourne pas sous systemd : lance « escouade-server update --to {} » sur le serveur, puis relance-le. Rien n’est changé.",
                "this server does not run under systemd: run “escouade-server update --to {}” on the server, then start it again. Nothing is changed.",
                version.trim()
            ));
        }
        let target = update::target_of(
            &self.current,
            &Asked {
                to: Some(version.to_string()),
                ..Asked::default()
            },
            None,
            None,
        )?;
        if target == self.current {
            bail!(tr!(
                "le serveur est déjà en {target}",
                "the server is at {target} already"
            ));
        }
        if matches!(state.phase, Phase::Ready | Phase::Restarting) {
            return Ok(state.available.unwrap_or(target));
        }
        log::info!("escouade-server update: {target} asked by an administrator");
        *self.asked.lock() = Some(target.clone());
        self.poke.notify_one();
        Ok(target)
    }

    /// The log says it once per version and phase.
    fn say(&self, phase: Phase, version: &str, line: impl FnOnce() -> String, warn: bool) {
        let mut said = self.said.lock();
        if said.as_ref() == Some(&(phase, version.to_string())) {
            return;
        }
        *said = Some((phase, version.to_string()));
        drop(said);
        if warn {
            log::warn!("escouade-server update: {}", line());
        } else {
            log::info!("escouade-server update: {}", line());
        }
    }

    fn set(&self, f: impl FnOnce(&mut State)) {
        f(&mut self.state.lock());
    }

    /// One look at the releases, with the choice `choice`: the version found said, put in place when it may be.
    pub async fn check(&self, choice: Choice) -> Phase {
        if matches!(self.state().phase, Phase::Ready | Phase::Restarting) {
            return self.state().phase;
        }
        let asked_for = self.asked.lock().take();
        *self.last.lock() = Some(Instant::now());
        self.set(|s| {
            s.phase = Phase::Checking;
            s.checked_at = Some(crate::model::now_ms());
        });
        let asked = Asked {
            to: asked_for.clone(),
            beta: choice.beta,
            ..Asked::default()
        };
        let read = update::read_target(
            &self.source.client,
            &self.source.base,
            &self.current,
            &asked,
        )
        .await;
        let target = match read {
            Ok(t) => t,
            // A published version older than the program: nothing newer (SR1-07), said once.
            Err(e) if e.downcast_ref::<update::Older>().is_some() => {
                self.say(Phase::UpToDate, "older", || format!("{e:#}"), false);
                self.set(|s| {
                    s.phase = Phase::UpToDate;
                    s.available = None;
                    s.error = None;
                    s.command = None;
                });
                return Phase::UpToDate;
            }
            Err(e) => return self.failed(&format!("{e:#}"), "check"),
        };
        if target == self.current {
            self.set(|s| {
                s.phase = Phase::UpToDate;
                s.available = None;
                s.error = None;
                s.command = None;
            });
            return Phase::UpToDate;
        }
        let state = self.state();
        if state.image {
            let command = image_command(&target);
            self.say(
                Phase::Image,
                &target,
                || format!("{target} is published: a new image exists ({command}); nothing is replaced in the image"),
                false,
            );
            self.set(|s| {
                s.phase = Phase::Image;
                s.available = Some(target.clone());
                s.command = Some(command);
                s.error = None;
            });
            return Phase::Image;
        }
        if self.refused.lock().contains(&target) && asked_for.is_none() {
            self.set(|s| {
                s.available = Some(target.clone());
                s.phase = Phase::Failed;
                if s.error.is_none() {
                    s.error = Some(tr!(
                        "{target} n’a pas pris au dernier redémarrage : elle n’est plus essayée d’elle-même (« escouade-server update --to {target} »)",
                        "{target} did not take at the last restart: it is no longer tried by itself (“escouade-server update --to {target}”)"
                    ));
                }
            });
            return Phase::Failed;
        }
        if !state.installs || (!choice.automatic && asked_for.is_none()) {
            let why = if !state.installs {
                "this server does not run under systemd on Linux"
            } else {
                "the automatic update is off"
            };
            self.say(
                Phase::Available,
                &target,
                || {
                    format!(
                        "{target} is published ({why}): “escouade-server update” puts it in place"
                    )
                },
                false,
            );
            self.set(|s| {
                s.phase = Phase::Available;
                s.available = Some(target.clone());
                s.command = Some(format!("escouade-server update --to {target}"));
                s.error = None;
            });
            return Phase::Available;
        }
        log::info!(
            "escouade-server update: {target} found on the {} channel, downloading {}",
            channel_of(choice),
            update::archive_name(&target, &self.source.arch)
        );
        self.set(|s| {
            s.phase = Phase::Downloading;
            s.available = Some(target.clone());
            s.command = None;
            s.error = None;
        });
        let bytes = match update::download(
            &self.source.client,
            &self.source.base,
            &target,
            &self.source.arch,
            &self.source.key,
        )
        .await
        {
            Ok(b) => b,
            Err(e) => {
                if e.downcast_ref::<update::SignatureRefused>().is_some() {
                    // Never downloaded again by itself: a release that lies stays refused.
                    self.refused.lock().insert(target.clone());
                }
                return self.failed(&format!("{e:#}"), "download");
            }
        };
        log::info!("escouade-server update: {target}: signature checked");
        let dir = self.target.dir.clone();
        let installed = tokio::task::spawn_blocking(move || {
            update::install(&bytes, &dir).map_err(|e| format!("{e:#}"))
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
        match installed {
            Ok(names) => {
                log::info!(
                    "escouade-server update: {target} put in {} ({}); the old files kept as .old; it restarts once at rest",
                    self.target.dir.display(),
                    names.join(", ")
                );
                self.set(|s| s.phase = Phase::Ready);
                Phase::Ready
            }
            Err(e) => self.failed(&e, "install"),
        }
    }

    fn failed(&self, error: &str, step: &str) -> Phase {
        log::warn!("escouade-server update: {step} refused: {error}");
        self.set(|s| {
            s.phase = Phase::Failed;
            s.error = Some(error.to_string());
        });
        Phase::Failed
    }

    /// The server at `busy` (`status::busy_word`, none at rest): a version ready restarts it at rest. True when it is
    /// time: the pending restart written, the log said.
    pub fn at_rest(&self, busy: Option<&str>) -> bool {
        let state = self.state();
        if state.phase != Phase::Ready {
            return false;
        }
        let to = state.available.clone().unwrap_or_default();
        if let Some(why) = busy {
            if state.busy.as_deref() != Some(why) {
                log::info!(
                    "escouade-server update: {to} waits for the server to be at rest ({})",
                    super::status::busy_text(why)
                );
            }
            self.set(|s| s.busy = Some(why.to_string()));
            return false;
        }
        let pending = Pending {
            from: self.current.clone(),
            to: to.clone(),
            at: crate::model::now_ms(),
        };
        if let Ok(bytes) = serde_json::to_vec_pretty(&pending) {
            let _ = crate::paths::write_atomic(&self.data.root().join(PENDING), &bytes);
        }
        log::info!(
            "escouade-server update: at rest, stopping to start again at {to} (systemd starts it again)"
        );
        self.set(|s| {
            s.phase = Phase::Restarting;
            s.busy = None;
        });
        self.restart.notify_one();
        true
    }

    /// At the start: the restart of before said, or the version that did not take (never tried again by itself).
    pub fn after_start(&self) {
        let path = self.data.root().join(PENDING);
        let Some(p) = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Pending>(&b).ok())
        else {
            return;
        };
        let _ = std::fs::remove_file(&path);
        if p.to == self.current {
            log::info!(
                "escouade-server update: started again at {} (from {})",
                p.to,
                p.from
            );
        } else {
            log::warn!(
                "escouade-server update: {} did not take, the program is still at {}: not tried again by itself",
                p.to,
                self.current
            );
            self.refused.lock().insert(p.to);
        }
    }

    /// Whether a check is due now.
    fn is_due(&self) -> bool {
        self.asked.lock().is_some()
            || self.looks
                && due(
                    self.started,
                    *self.last.lock(),
                    Instant::now(),
                    self.first,
                    self.every,
                )
    }

    /// The loop of a running server: the choice of its settings read, a check when due, the restart at rest, what
    /// `escouade-server config` asks.
    pub fn watch(self: &Arc<Self>, core: Arc<crate::core::Core>) {
        let me = self.clone();
        me.after_start();
        tauri::async_runtime::spawn(async move {
            let updates = crate::updates::Updates::default();
            loop {
                let _ = tokio::time::timeout(LOOK, me.poke.notified()).await;
                answer_config(&core);
                let choice = choice_of(&core.settings.read(), &core.locks.read());
                me.set_choice(choice);
                if me.is_due() {
                    me.check(choice).await;
                }
                if me.state().phase == Phase::Ready {
                    let snapshot =
                        crate::updates::snapshot(&core, &updates, crate::model::now_ms());
                    let busy = crate::updates::busy(&snapshot).map(super::status::busy_word);
                    if me.at_rest(busy) {
                        break;
                    }
                }
            }
        });
    }
}

fn channel_of(choice: Choice) -> String {
    if choice.beta { "beta" } else { "stable" }.to_string()
}

/// What `escouade-server config` asks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigRequest {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beta_updates: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigAnswer {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A request of `escouade-server config` left beside a running server: taken (removed first), applied as a client's
/// change of the settings is (`Core::save_settings`, the policies checked), answered.
fn answer_config(core: &Arc<crate::core::Core>) {
    let root = core.data.root();
    let path = root.join(REQUEST);
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let _ = std::fs::remove_file(&path);
    let Ok(request) = serde_json::from_slice::<ConfigRequest>(&bytes) else {
        return;
    };
    let mut s = core.settings.read().clone();
    if let Some(on) = request.auto_update {
        s.auto_update = on;
    }
    if let Some(on) = request.beta_updates {
        s.beta_updates = on;
    }
    let error = core.save_settings(s).err().map(|e| format!("{e:#}"));
    match &error {
        None => log::info!(
            "escouade-server update: settings changed by « escouade-server config »: automatic {}, {} channel",
            if core.settings.read().auto_update { "on" } else { "off" },
            if core.settings.read().beta_updates { "beta" } else { "stable" }
        ),
        Some(e) => log::warn!("escouade-server update: « escouade-server config » refused: {e}"),
    }
    let answer = ConfigAnswer {
        id: request.id,
        error,
    };
    if let Ok(bytes) = serde_json::to_vec(&answer) {
        let _ = crate::paths::write_atomic(&root.join(ANSWER), &bytes);
    }
}

/// `escouade-server config` changes the update's settings of the data folder `data`: `automatic` and `beta` when
/// given. A running server takes them itself (its settings are in its memory), within 15 s; else the file is written
/// here, the policies of this machine checked.
pub fn configure(data: &DataDir, automatic: Option<bool>, beta: Option<bool>) -> Result<()> {
    if automatic.is_none() && beta.is_none() {
        return Ok(());
    }
    let running = super::status::read(data).is_some_and(|s| {
        super::status::healthy(&s, crate::model::now_ms(), super::status::alive).is_ok()
    });
    if running {
        let id = hex::encode(crate::model::now_ms().to_be_bytes());
        let request = ConfigRequest {
            id: id.clone(),
            auto_update: automatic,
            beta_updates: beta,
        };
        let answer_path = data.root().join(ANSWER);
        let _ = std::fs::remove_file(&answer_path);
        crate::paths::write_atomic(&data.root().join(REQUEST), &serde_json::to_vec(&request)?)?;
        let until = Instant::now() + Duration::from_secs(15);
        while Instant::now() < until {
            if let Some(a) = std::fs::read(&answer_path)
                .ok()
                .and_then(|b| serde_json::from_slice::<ConfigAnswer>(&b).ok())
                .filter(|a| a.id == id)
            {
                let _ = std::fs::remove_file(&answer_path);
                return match a.error {
                    Some(e) => Err(anyhow!(e)),
                    None => Ok(()),
                };
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        let _ = std::fs::remove_file(data.root().join(REQUEST));
        bail!(tr!(
            "le serveur lancé n’a pas répondu en 15 s : rien n’est changé",
            "the running server did not answer in 15 s: nothing is changed"
        ));
    }
    write_settings(data, automatic, beta, &crate::locks::Managed::read())
}

/// The settings file of `data` with `autoUpdate` and `betaUpdates` changed, the rest kept as it is; refused when the
/// policies `managed` lock them otherwise.
pub fn write_settings(
    data: &DataDir,
    automatic: Option<bool>,
    beta: Option<bool>,
    managed: &crate::locks::Managed,
) -> Result<()> {
    let file = data.settings_file();
    let mut value: serde_json::Value = std::fs::read(&file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(on) = automatic {
        value["autoUpdate"] = on.into();
    }
    if let Some(on) = beta {
        value["betaUpdates"] = on.into();
    }
    let settings: crate::model::Settings = serde_json::from_value(value.clone())?;
    let locks = crate::locks::Locks::compute(crate::locks::KEYS, None, managed);
    locks.check_settings(&settings)?;
    crate::paths::write_atomic(&file, &serde_json::to_vec_pretty(&value)?)?;
    Ok(())
}

/// `escouade-server config` without a change: what the settings of `data` choose, within the policies of this machine.
pub fn shown(data: &DataDir) -> String {
    let settings: crate::model::Settings = std::fs::read(data.settings_file())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let locks =
        crate::locks::Locks::compute(crate::locks::KEYS, None, &crate::locks::Managed::read());
    let choice = choice_of(&settings, &locks);
    let imposed = |key: &str| {
        if locks.value(key).is_some() {
            tr!(
                " (imposé par la stratégie de ce poste)",
                " (imposed by this machine's policy)"
            )
        } else {
            String::new()
        }
    };
    tr!(
        "Mise à jour automatique : {}{}\nCanal : {}{}\n\nescouade-server config automatic on|off\nescouade-server config channel stable|beta",
        "Automatic update: {}{}\nChannel: {}{}\n\nescouade-server config automatic on|off\nescouade-server config channel stable|beta",
        if choice.automatic { tr!("allumée", "on") } else { tr!("éteinte", "off") },
        imposed(crate::locks::AUTO_UPDATE),
        if choice.beta { tr!("bêta", "beta") } else { tr!("stable", "stable") },
        imposed(crate::locks::CHANNEL)
    )
}

/// The folder of the program, for `Target`, in a test.
pub fn target_in(dir: &Path, supervisor: Option<Supervisor>, installs: bool) -> Target {
    Target {
        dir: dir.to_path_buf(),
        supervisor,
        installs,
    }
}
