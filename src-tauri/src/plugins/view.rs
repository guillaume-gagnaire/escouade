//! The terminal views through the modules of their harnesses (`view/translate`, ESC-352, docs/PLUGINS.md §8): the core reads
//! the file the harness's own interface writes, and the plugin says what its lines mean, so that a correction published on
//! the store corrects the view too. The core links no plugin.
//!
//! - **An instance of its own.** A plugin's views run in an instance beside the one of its agents, started from the same
//!   module, without `escouade.call`: a view asks nothing of the machine. The core reads a view under the lock of its agent, a
//!   line at a time, each line reading what the last one made of the agent (its status, its turn): the call is waited for,
//!   by the same turn as the rest (`Instance::turn`), bound by the plugin's time for a turn. The instance has a thread of its
//!   own, as any: its WASI blocks on a runtime of its own, which a thread of tokio may not do.
//! - **A fault** ends the instance: the next call makes another, and the views start anew (`generation`), told what their
//!   conversation holds.
//! - **A plugin stopped or changed** drops its instance (`forget`).
//! - **What is said** of a view that is not read is in the language of the app: it goes in a line of the conversation.

use super::harness::plugged;
use super::wasm::{Fault, Instance};
use crate::model::EngineId;
use escouade_plugin_protocol::engine::{method, Item, ViewParams};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::OnceLock;

/// How many turns an answer may take: a module that holds its answer for a turn of its reactor answers in the next.
const TURNS: usize = 3;

/// The plugin that translates the view of the harness `engine`.
pub fn plugin_of(engine: EngineId) -> Option<&'static str> {
    Some(match engine {
        EngineId::CLAUDE => super::builtin::CLAUDE,
        EngineId::CODEX => super::builtin::CODEX,
        EngineId::COPILOT => super::builtin::COPILOT,
        EngineId::CURSOR => super::builtin::CURSOR,
        EngineId::VIBE => super::builtin::MISTRAL,
        EngineId::AGY => super::builtin::ANTIGRAVITY,
        EngineId::GEMINI => super::builtin::GEMINI,
        _ => return None,
    })
}

struct Live {
    instance: Instance,
    /// Which instance this is, of all those made.
    generation: u64,
    next: u64,
    /// What a stand-in of the tests is asked in place of any call.
    #[cfg(test)]
    asked: Option<&'static str>,
}

/// What the thread of a plugin's views is asked.
type Job = Box<dyn FnOnce(&mut Option<Live>) + Send>;

/// The threads of the plugins that have had a view, by plugin.
fn threads() -> &'static Mutex<HashMap<String, mpsc::Sender<Job>>> {
    static ALL: OnceLock<Mutex<HashMap<String, mpsc::Sender<Job>>>> = OnceLock::new();
    ALL.get_or_init(Default::default)
}

/// The thread of the views of `plugin`, which holds its instance.
fn thread_of(plugin: &str) -> mpsc::Sender<Job> {
    threads()
        .lock()
        .entry(plugin.to_string())
        .or_insert_with(|| {
            let (tx, rx) = mpsc::channel::<Job>();
            let name = format!("views of {plugin}");
            let spawned = std::thread::Builder::new().name(name).spawn(move || {
                let mut live = None;
                while let Ok(job) = rx.recv() {
                    job(&mut live);
                }
            });
            if let Err(e) = spawned {
                log::warn!("no thread for the views of a plugin: {e}");
            }
            tx
        })
        .clone()
}

/// `f` on the thread of the views of `plugin`, waited for.
fn on<R: Send + 'static>(
    plugin: &str,
    f: impl FnOnce(&mut Option<Live>) -> R + Send + 'static,
) -> Result<R, String> {
    let (tx, rx) = mpsc::channel();
    let job: Job = Box::new(move |live| {
        let _ = tx.send(f(live));
    });
    thread_of(plugin).send(job).map_err(|_| stopped())?;
    rx.recv().map_err(|_| stopped())
}

/// Why a view is not read: its thread is gone.
fn stopped() -> String {
    tr!("ses vues se sont arrêtées", "its views stopped")
}

/// Why a view is not read: its plugin is not there to read it.
pub(crate) fn off() -> String {
    tr!(
        "il est coupé ou n’est pas installé",
        "it is off or not installed"
    )
}

/// Why a view is not read: what went wrong in its instance. The journal keeps the details.
fn said_of(fault: &Fault) -> String {
    match fault {
        Fault::Timeout(d) => tr!(
            "il n’a pas répondu en {seconds} s",
            "it did not answer within {seconds} s",
            seconds = d.as_secs_f32()
        ),
        Fault::Crashed(_) => tr!("il a planté", "it crashed"),
    }
}

/// A module of the tests: their runtime, the plugins of `scripts/plugins-wasm.sh`.
#[cfg(test)]
pub(crate) fn module_of_the_tests(plugin: &str) -> Result<Instance, String> {
    static RUNTIME: OnceLock<std::sync::Arc<super::runtime::Runtime>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| super::builtin::test_runtime("views"))
        .view_instance(plugin)
}

/// What a test puts in place of the instance of a plugin of its own: the instance, and what to ask it in place of any call.
#[cfg(test)]
pub(crate) type StandIn =
    Box<dyn Fn() -> Result<(Instance, Option<&'static str>), String> + Send + Sync>;

#[cfg(test)]
fn stand_ins() -> &'static Mutex<HashMap<String, std::sync::Arc<StandIn>>> {
    static ALL: OnceLock<Mutex<HashMap<String, std::sync::Arc<StandIn>>>> = OnceLock::new();
    ALL.get_or_init(Default::default)
}

/// `made` makes the instance of `plugin` from now on.
#[cfg(test)]
pub(crate) fn stand_in(plugin: &str, made: StandIn) {
    stand_ins()
        .lock()
        .insert(plugin.to_string(), std::sync::Arc::new(made));
}

/// Whether `plugin` has a thread for its views.
#[cfg(test)]
pub(crate) fn has_thread(plugin: &str) -> bool {
    threads().lock().contains_key(plugin)
}

/// The instance of the views of `plugin`, when it is there and on.
fn started(plugin: &str) -> Result<Instance, String> {
    let Some(engine) = plugged(plugin) else {
        #[cfg(test)]
        return module_of_the_tests(plugin);
        #[cfg(not(test))]
        return Err(off());
    };
    engine.view_instance().map_err(|e| {
        log::warn!("{plugin}: no instance for its views: {e}");
        tr!("son module ne démarre pas", "its module does not start")
    })
}

impl Live {
    fn of(plugin: &str) -> Result<Self, String> {
        static MADE: AtomicU64 = AtomicU64::new(0);
        #[cfg(test)]
        let (instance, asked) = {
            let made = stand_ins().lock().get(plugin).cloned();
            match made {
                Some(made) => made()?,
                None => (started(plugin)?, None),
            }
        };
        #[cfg(not(test))]
        let instance = started(plugin)?;
        Ok(Live {
            instance,
            generation: MADE.fetch_add(1, Ordering::Relaxed) + 1,
            next: 0,
            #[cfg(test)]
            asked,
        })
    }

    fn call(&mut self, method: &str, params: &Value) -> Result<Value, Fault> {
        self.next += 1;
        let id = self.next;
        #[cfg(test)]
        let method = self.asked.unwrap_or(method);
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.instance.push(format!("{line}\n").as_bytes());
        let within = self.instance.limits().turn;
        for _ in 0..TURNS {
            for line in self.instance.turn(within)? {
                let Ok(answer) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if answer["id"] == id {
                    return Ok(answer);
                }
            }
        }
        Err(Fault::Crashed(format!("no answer to {method}")))
    }
}

/// The instance of `plugin`, made if it has none.
fn live<'a>(slot: &'a mut Option<Live>, plugin: &str) -> Result<&'a mut Live, String> {
    if slot.is_none() {
        *slot = Some(Live::of(plugin)?);
    }
    slot.as_mut().ok_or_else(stopped)
}

/// Which instance serves the views of `plugin`, made if none does: a view started on another starts anew.
pub fn generation(plugin: &str) -> Result<u64, String> {
    let name = plugin.to_string();
    on(plugin, move |slot| live(slot, &name).map(|l| l.generation))?
}

/// A call on the instance of `plugin`: its answer. Err: why not; an instance at fault is gone, the next call makes another.
fn called(plugin: &str, method: &'static str, params: Value) -> Result<Value, String> {
    let name = plugin.to_string();
    on(plugin, move |slot| {
        let answer = live(slot, &name)?.call(method, &params);
        answer.map_err(|fault| {
            log::warn!("{name}: its views' instance is gone: {fault}");
            *slot = None;
            said_of(&fault)
        })
    })?
}

/// The items of `params`, read by `plugin`.
pub fn translate(plugin: &str, params: &ViewParams) -> Result<Vec<Item>, String> {
    let params = serde_json::to_value(params).map_err(|e| e.to_string())?;
    let answer = called(plugin, method::VIEW_TRANSLATE, params)?;
    if !answer["error"].is_null() {
        let why = answer["error"]["message"].as_str().unwrap_or_default();
        log::warn!("{plugin}: view/translate refused: {why}");
        return Err(tr!("il a refusé ses lignes", "it refused its lines"));
    }
    // An item the core does not know is left aside, as those of `agent/frames`.
    let items = answer["result"]["items"].as_array().cloned();
    Ok(items
        .into_iter()
        .flatten()
        .filter_map(|i| serde_json::from_value::<Item>(i).ok())
        .collect())
}

/// A call of the contract on the instance of `plugin`, for a recorded stream in a test.
#[cfg(test)]
pub fn call(plugin: &str, method: &'static str, params: &Value) -> Result<Value, String> {
    let answer = called(plugin, method, params.clone())?;
    match answer["error"].is_null() {
        true => Ok(answer["result"].clone()),
        false => Err(answer["error"].to_string()),
    }
}

/// The views of `plugin` end here: it stopped, or another version of it comes.
pub fn forget(plugin: &str) {
    // Not waited for: a plugin stops under locks a view may be waiting on. A plugin that had no view has nothing to drop, and
    // gets no thread for it.
    let Some(thread) = threads().lock().get(plugin).cloned() else {
        return;
    };
    let job: Job = Box::new(|live| *live = None);
    let _ = thread.send(job);
}

/// Its module made ready before a view needs it: precompiled, it may take seconds, and a view reads under its agent's lock.
#[cfg(not(test))]
pub fn warm(engine: EngineId) {
    let Some(plugin) = plugin_of(engine) else {
        return;
    };
    std::thread::spawn(move || {
        if let Err(e) = generation(plugin) {
            log::debug!("{plugin}: no module for its views yet: {e}");
        }
    });
}
