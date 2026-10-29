//! The window's thread (ESC-290). Tauri runs a command without `async` on it, and while one runs
//! the window neither draws nor answers: under WebKitGTK a command that reads a big folder, runs
//! git or waits for a lock froze the whole interface. Every command now leaves it, for a lane of
//! its own (`Lanes`): the calls of one command, or of one family (a terminal's writes, an
//! attachment's chunks), keep their order; a slow one holds back none of the others. A command
//! whose own part lasts more than `SLOW` is written in the log, and a watch on the window's thread
//! (`watch`) writes there any pause of more than `PAUSE`, with the commands running then. Never a
//! path nor a content: a command's name, how long, the ids it was called with.

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::ipc::{Invoke, InvokeBody};
use tauri::Runtime;

/// A command whose part on its lane lasts more than this is written in the log.
pub const SLOW: Duration = Duration::from_millis(250);
/// A window's thread that answers nothing for this long is written in the log.
pub const PAUSE: Duration = Duration::from_secs(1);
/// How often the watch asks the window's thread for an answer.
const BEAT: Duration = Duration::from_millis(250);
/// The same command is written as slow once in this long; the times between are counted.
const SLOW_EVERY: Duration = Duration::from_secs(30);

/// The commands that share a lane: the calls of a family keep their order between them (the
/// keystrokes and sizes of a terminal, the chunks of an attachment, the dictation's steps).
const FAMILIES: &[(&str, &str)] = &[
    ("term_", "term"),
    ("attach_", "attach"),
    ("voice_", "voice"),
    ("conversation_", "voice"),
    ("speech_", "voice"),
];

/// The lane of the command `name`: its family's, else its own.
pub fn lane_of(name: &str) -> &str {
    FAMILIES
        .iter()
        .find(|(prefix, _)| name.starts_with(prefix))
        .map_or(name, |(_, lane)| lane)
}

pub type Job = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Lane {
    queue: VecDeque<Job>,
    running: bool,
}

/// What runs now on the lanes, for the watch: the name and since when.
#[derive(Default)]
pub struct Running {
    next: Mutex<u64>,
    now: Mutex<HashMap<u64, (String, Instant)>>,
}

/// One command running, until dropped.
pub struct Entered<'a> {
    running: &'a Running,
    id: u64,
}

impl Drop for Entered<'_> {
    fn drop(&mut self) {
        self.running.now.lock().remove(&self.id);
    }
}

impl Running {
    pub fn enter(&self, name: &str) -> Entered<'_> {
        let id = {
            let mut next = self.next.lock();
            *next += 1;
            *next
        };
        self.now
            .lock()
            .insert(id, (name.to_string(), Instant::now()));
        Entered { running: self, id }
    }

    /// What runs, the longest first: « get_conversation 1200 ms, stats 300 ms ».
    pub fn told(&self) -> String {
        let mut now: Vec<(String, Duration)> = self
            .now
            .lock()
            .values()
            .map(|(name, at)| (name.clone(), at.elapsed()))
            .collect();
        now.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
        now.iter()
            .take(3)
            .map(|(name, d)| format!("{name} {} ms", d.as_millis()))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The lanes the commands run on, away from the window's thread: one per command or family, each
/// running its calls in order on a thread of the blocking pool while it has some.
#[derive(Default)]
pub struct Lanes {
    lanes: Mutex<HashMap<String, Lane>>,
    pub running: Arc<Running>,
    slow: Mutex<HashMap<String, (Option<Instant>, u32)>>,
}

impl Lanes {
    pub fn new(running: Arc<Running>) -> Arc<Self> {
        Arc::new(Self {
            running,
            ..Default::default()
        })
    }

    /// Runs `job` on the lane `lane`, after the jobs it already holds.
    pub fn submit(self: &Arc<Self>, lane: &str, job: Job) {
        {
            let mut lanes = self.lanes.lock();
            let l = lanes.entry(lane.to_string()).or_default();
            l.queue.push_back(job);
            if l.running {
                return;
            }
            l.running = true;
        }
        let (me, lane) = (self.clone(), lane.to_string());
        tauri::async_runtime::spawn_blocking(move || me.drain(&lane));
    }

    fn drain(&self, lane: &str) {
        loop {
            let job = {
                let mut lanes = self.lanes.lock();
                let Some(l) = lanes.get_mut(lane) else { return };
                match l.queue.pop_front() {
                    Some(job) => job,
                    None => {
                        lanes.remove(lane);
                        return;
                    }
                }
            };
            // A command that panics fails its call (Tauri's resolver, dropped, says so), not its lane.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
        }
    }

    /// The line the log gets for the command `name` that took `took`, called with `ids`, if it is
    /// slow and not already written in the last `SLOW_EVERY`.
    pub fn slow_line(&self, name: &str, took: Duration, ids: &str) -> Option<String> {
        if took <= SLOW {
            return None;
        }
        let mut slow = self.slow.lock();
        let (at, skipped) = slow.entry(name.to_string()).or_insert((None, 0));
        if at.is_some_and(|at| at.elapsed() < SLOW_EVERY) {
            *skipped += 1;
            return None;
        }
        let more = match std::mem::take(skipped) {
            0 => String::new(),
            n => format!(" ({n} more slow since the last line)"),
        };
        *at = Some(Instant::now());
        Some(format!(
            "slow command: {name} took {} ms{ids}{more}",
            took.as_millis()
        ))
    }
}

/// The ids a command was called with, as the log may hold them: those of its project, agent or
/// ticket, an id being only letters, digits, `-` and `_` (`model::new_id`). A path, a text, a
/// content never: « project 3f2a9c81d0e4 ».
pub fn ids_of(body: &InvokeBody) -> String {
    let InvokeBody::Json(args) = body else {
        return String::new();
    };
    let is_id = |v: &serde_json::Value| {
        v.as_str()
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 40
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            })
            .map(str::to_string)
    };
    let mut out = String::new();
    for (key, told) in [
        ("projectId", "project"),
        ("project", "project"),
        ("agentId", "agent"),
        ("ticketId", "ticket"),
        ("id", "id"),
    ] {
        let Some(v) = args.get(key) else { continue };
        // `update_project` gets the project whole: its id only.
        let id = is_id(v).or_else(|| v.get("id").and_then(is_id));
        if let Some(id) = id {
            out.push_str(&format!(" {told} {id}"));
        }
    }
    if out.is_empty() {
        out
    } else {
        format!(" ({})", out.trim_start())
    }
}

/// The app's handler of the commands, `handler`, run away from the window's thread: each call on
/// the lane of its command (`lane_of`), timed there.
pub fn off_the_window<R: Runtime>(
    lanes: Arc<Lanes>,
    handler: impl Fn(Invoke<R>) -> bool + Send + Sync + 'static,
) -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    let handler = Arc::new(handler);
    move |invoke: Invoke<R>| {
        let name = invoke.message.command().to_string();
        let (h, l) = (handler.clone(), lanes.clone());
        let lane = lane_of(&name).to_string();
        lanes.submit(
            &lane,
            Box::new(move || {
                let ids = ids_of(invoke.message.payload());
                let resolver = invoke.resolver.clone();
                let started = Instant::now();
                let handled = {
                    let _running = l.running.enter(&name);
                    h(invoke)
                };
                if !handled {
                    resolver.reject(format!("Command {name} not found"));
                }
                if let Some(line) = l.slow_line(&name, started.elapsed(), &ids) {
                    log::warn!("{line}");
                }
            }),
        );
        true
    }
}

/// What the watch saw of the window's thread.
#[derive(Debug, Clone, PartialEq)]
pub enum Pause {
    /// No answer for `PAUSE`: what ran on the lanes then.
    Began { running: String },
    /// The answer came back, `lasted` after it was asked for.
    Ended { lasted: Duration },
}

impl Pause {
    pub fn line(&self) -> String {
        match self {
            Pause::Began { running } if running.is_empty() => format!(
                "window thread: no answer for {} ms, no command running",
                PAUSE.as_millis()
            ),
            Pause::Began { running } => format!(
                "window thread: no answer for {} ms, commands running: {running}",
                PAUSE.as_millis()
            ),
            Pause::Ended { lasted } => {
                format!("window thread: paused {} ms", lasted.as_millis())
            }
        }
    }
}

/// Watches the window's thread until `post` fails (the app is gone): `post` runs its job there.
/// Every `BEAT` a job is posted; one that does not run within `PAUSE` is a pause, told to `tell`
/// when it begins and when it ends.
pub fn watch(
    post: impl Fn(Job) -> bool,
    running: &Running,
    tell: impl Fn(Pause),
    beat: Duration,
    pause: Duration,
) {
    loop {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let asked = Instant::now();
        if !post(Box::new(move || {
            let _ = tx.send(());
        })) {
            return;
        }
        match rx.recv_timeout(pause) {
            Ok(()) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                tell(Pause::Began {
                    running: running.told(),
                });
                if rx.recv().is_err() {
                    return;
                }
                tell(Pause::Ended {
                    lasted: asked.elapsed(),
                });
            }
            // The job was dropped unrun: the loop of the window is gone.
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        }
        std::thread::sleep(beat);
    }
}

/// Starts the watch of the app's window thread, its pauses written in the log.
pub fn watch_app<R: Runtime>(app: tauri::AppHandle<R>, running: Arc<Running>) {
    let spawned = std::thread::Builder::new()
        .name("window-watch".into())
        .spawn(move || {
            watch(
                |job| app.run_on_main_thread(job).is_ok(),
                &running,
                |p| log::warn!("{}", p.line()),
                BEAT,
                PAUSE,
            )
        });
    if let Err(e) = spawned {
        log::warn!("window thread not watched: {e}");
    }
}
