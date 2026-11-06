//! A voice service that a plugin serves, in the app (INT-13, ESC-285; `docs/PLUGINS.md` §4.13): `ProcessVoiceService:
//! VoiceService` answers the core's calls by JSON-RPC to the plugin's module (`voice/*`, `escouade_voice::wire`), as
//! `ProcessTicketSource` does for the tickets, so that the dictation, the stream, the reading and the catalog do not
//! see the difference. The key stays in the app's keychain (`dictation-<id>`): each call carries it to the plugin of
//! this service, which keeps nothing.

use super::live::{Cmd, Final, LiveStream, Partial, FINAL_WAIT};
use super::online::{OnlineError, Provider, Spec};
use super::online_catalog::{Listed, ServiceCatalog};
use super::online_speech::SpeakChoice;
use super::service::{LocalFuture, SendFuture, VoiceService};
use super::speech::Audio;
use crate::plugins::runtime::{CallError, Runtime};
use escouade_voice::wire;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

type Partials = Mutex<HashMap<String, Arc<Mutex<Partial>>>>;
type Sounds = Mutex<HashMap<String, mpsc::UnboundedSender<Audio>>>;

/// How often a call that may be given up checks whether it was.
const CANCEL_POLL: Duration = Duration::from_millis(25);

/// A voice service of the plugin `plugin`, as it described itself.
pub struct ProcessVoiceService {
    rt: Arc<Runtime>,
    plugin: String,
    provider: Provider,
    label: String,
    default_model: String,
    offers_live: bool,
    live_model: Option<String>,
    max_chars: usize,
    partials: Arc<Partials>,
    sounds: Arc<Sounds>,
}

impl ProcessVoiceService {
    /// One of Escouade's five, as `escouade_voice` describes it, listened to without starting its module: its first call
    /// does, so that a start of the app pays nothing for a voice nobody uses. The service is not in the registry yet:
    /// `service::register` puts it there.
    pub fn described(rt: Arc<Runtime>, plugin: &str, provider: Provider) -> Arc<Self> {
        let (partials, sounds) = Self::listen(&rt, plugin);
        Arc::new(Self {
            rt,
            plugin: plugin.to_string(),
            provider,
            label: provider.label().to_string(),
            default_model: provider.model().to_string(),
            offers_live: provider.offers_live(),
            live_model: Some(provider.live_model("")).filter(|m| !m.is_empty()),
            max_chars: escouade_voice::speech::max_chars(provider),
            partials,
            sounds,
        })
    }

    /// Starts the plugin if it does not run, asks it what it is, and listens to it.
    #[cfg(test)]
    pub async fn connect(rt: Arc<Runtime>, plugin: &str) -> Result<Arc<Self>, CallError> {
        let d = rt.call(plugin, wire::DESCRIBE, json!({})).await?;
        let text = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let id = text("id");
        if id.is_empty() {
            return Err(CallError::Invalid("voice/describe: no id".into()));
        }
        let (partials, sounds) = Self::listen(&rt, plugin);
        Ok(Arc::new(Self {
            rt,
            plugin: plugin.to_string(),
            // The id is the service's, not the plugin's.
            provider: Provider::of(&id),
            label: text("label"),
            default_model: text("defaultModel"),
            offers_live: d["offersLive"].as_bool().unwrap_or(false),
            live_model: d["liveModel"].as_str().map(String::from),
            max_chars: d["maxChars"].as_u64().unwrap_or(1000) as usize,
            partials,
            sounds,
        }))
    }

    /// What the plugin tells while it works goes to whom waits for it.
    fn listen(rt: &Runtime, plugin: &str) -> (Arc<Partials>, Arc<Sounds>) {
        let partials: Arc<Partials> = Arc::default();
        let sounds: Arc<Sounds> = Arc::default();
        let (texts, audio) = (partials.clone(), sounds.clone());
        rt.on_notification(plugin, move |method, params| match method {
            wire::PARTIAL => {
                let stream = params["stream"].as_str().unwrap_or("");
                let found = texts.lock().get(stream).cloned();
                if let Some(p) = found {
                    (p.lock())(params["text"].as_str().unwrap_or("").to_string());
                }
            }
            wire::AUDIO => {
                let call = params["call"].as_str().unwrap_or("");
                let rate = params["rate"].as_u64().unwrap_or(24_000) as u32;
                let samples = params["pcm"].as_str().and_then(wire::pcm16_from_b64);
                if let (Some(samples), Some(tx)) = (samples, audio.lock().get(call)) {
                    let _ = tx.send(Audio { samples, rate });
                }
            }
            _ => {}
        });
        (partials, sounds)
    }

    /// The error of a failed call, as the core says it.
    fn error(&self, e: CallError) -> OnlineError {
        error_of(self.provider, &self.plugin, e)
    }
}

fn error_of(provider: Provider, plugin: &str, e: CallError) -> OnlineError {
    match e {
        CallError::Remote { code, message } => wire::error_of(provider, code, &message)
            .unwrap_or_else(|| OnlineError::Network(provider, format!("{message} ({code})"))),
        CallError::Invalid(why) => OnlineError::Network(
            provider,
            tr!(
                "Réponse illisible du plugin {plugin} : {why}",
                "Unreadable answer from the plugin {plugin}: {why}"
            ),
        ),
        down => OnlineError::Network(
            provider,
            tr!(
                "Le plugin {plugin} ne répond pas : {down}",
                "The plugin {plugin} is not answering: {down}"
            ),
        ),
    }
}

/// Runs `f` to its end on a thread and a runtime of its own: the callers of the contract block (`Voice::stop`).
fn block<T: Send + 'static>(f: impl Future<Output = T> + Send + 'static) -> Option<T> {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .map(|rt| rt.block_on(f))
    })
    .join()
    .ok()
    .flatten()
}

/// A call that tells the plugin to give up (`voice/cancel` for `call`) once `cancel` is set.
async fn call_cancellable(
    rt: &Runtime,
    plugin: &str,
    method: &str,
    params: Value,
    call: &str,
    cancel: &AtomicBool,
) -> Result<Value, CallError> {
    let fut = rt.call(plugin, method, params);
    tokio::pin!(fut);
    let mut told = false;
    loop {
        tokio::select! {
            r = &mut fut => return r,
            _ = tokio::time::sleep(CANCEL_POLL) => {
                if !told && cancel.load(Ordering::SeqCst) {
                    told = true;
                    let _ = rt.notify(plugin, wire::CANCEL, json!({ "call": call }));
                }
            }
        }
    }
}

impl VoiceService for ProcessVoiceService {
    fn id(&self) -> String {
        self.provider.id().into()
    }
    fn label(&self) -> String {
        self.label.clone()
    }
    fn account_entry(&self) -> String {
        self.provider.entry()
    }
    fn default_model(&self) -> String {
        self.default_model.clone()
    }

    fn transcribe(
        &self,
        spec: &Spec,
        samples: &[f32],
        language: &str,
        cancel: &Arc<AtomicBool>,
    ) -> Result<String, OnlineError> {
        let call = uuid::Uuid::new_v4().to_string();
        let params = json!({
            "call": call,
            "spec": wire::spec_json(spec),
            "language": language,
            "pcm16": wire::pcm16_to_b64(samples),
        });
        let (rt, plugin, cancel) = (self.rt.clone(), self.plugin.clone(), cancel.clone());
        let r = block(async move {
            call_cancellable(&rt, &plugin, wire::TRANSCRIBE, params, &call, &cancel).await
        });
        match r {
            Some(Ok(v)) => Ok(v["text"].as_str().unwrap_or("").to_string()),
            Some(Err(e)) => Err(self.error(e)),
            None => Err(OnlineError::Network(
                self.provider,
                "the transcription's thread panicked".into(),
            )),
        }
    }

    fn offers_live(&self) -> bool {
        self.offers_live
    }
    fn live_model(&self, chosen: &str) -> String {
        self.live_model.clone().unwrap_or_else(|| chosen.into())
    }

    fn open_live(&self, spec: &Spec, language: &str, rate: u32, partial: Partial) -> LiveStream {
        let stream = uuid::Uuid::new_v4().to_string();
        self.partials
            .lock()
            .insert(stream.clone(), Arc::new(Mutex::new(partial)));
        let (tx, mut rx) = mpsc::unbounded_channel::<Cmd>();
        let (done, result) = std::sync::mpsc::channel();
        let (rt, plugin, provider) = (self.rt.clone(), self.plugin.clone(), self.provider);
        let partials = self.partials.clone();
        let open = json!({
            "stream": stream,
            "spec": wire::spec_json(spec),
            "language": language,
            "rate": rate,
        });
        let started = std::thread::Builder::new()
            .name("voice-live-plugin".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                runtime.block_on(async {
                    let fail = |e| {
                        let _ = done.send(Err(error_of(provider, &plugin, e)));
                    };
                    if let Err(e) = rt.call(&plugin, wire::LIVE_OPEN, open).await {
                        partials.lock().remove(&stream);
                        return fail(e);
                    }
                    while let Some(cmd) = rx.recv().await {
                        match cmd {
                            Cmd::Audio(samples) => {
                                let _ = rt.notify(
                                    &plugin,
                                    wire::LIVE_AUDIO,
                                    json!({ "stream": stream, "pcm": wire::floats_to_b64(&samples) }),
                                );
                            }
                            Cmd::End => {
                                let r = rt
                                    .call(
                                        &plugin,
                                        wire::LIVE_FINISH,
                                        json!({
                                            "stream": stream,
                                            "waitMs": FINAL_WAIT.as_millis() as u64,
                                        }),
                                    )
                                    .await;
                                partials.lock().remove(&stream);
                                let _ = done.send(match r {
                                    Ok(v) => Ok(Final {
                                        text: v["text"].as_str().unwrap_or("").to_string(),
                                        ready: Instant::now(),
                                    }),
                                    Err(e) => Err(error_of(provider, &plugin, e)),
                                });
                                return;
                            }
                        }
                    }
                    // Dropped before the end: the plugin closes it.
                    partials.lock().remove(&stream);
                    let _ = rt.notify(&plugin, wire::LIVE_CLOSE, json!({ "stream": stream }));
                });
            });
        if let Err(e) = started {
            log::warn!("voice: the plugin stream's thread did not start: {e}");
        }
        LiveStream::bridged(self.provider, tx, result)
    }

    fn max_chars(&self) -> usize {
        self.max_chars
    }

    fn speak<'a>(
        &'a self,
        spec: &'a Spec,
        choice: &'a SpeakChoice,
        sentence: &'a str,
        cancel: &'a AtomicBool,
        on: &'a mut dyn FnMut(Audio),
    ) -> LocalFuture<'a, Result<Option<Duration>, OnlineError>> {
        Box::pin(async move {
            let call = uuid::Uuid::new_v4().to_string();
            let (tx, mut rx) = mpsc::unbounded_channel();
            self.sounds.lock().insert(call.clone(), tx);
            let params = json!({
                "call": call,
                "spec": wire::spec_json(spec),
                "model": choice.model,
                "voice": choice.voice,
                "lang": choice.lang,
                "rate": choice.rate,
                "sentence": sentence,
            });
            let fut = self.rt.call(&self.plugin, wire::SPEAK, params);
            tokio::pin!(fut);
            let mut told = false;
            let r = loop {
                tokio::select! {
                    r = &mut fut => break r,
                    Some(a) = rx.recv() => on(a),
                    _ = tokio::time::sleep(CANCEL_POLL) => {
                        if !told && cancel.load(Ordering::SeqCst) {
                            told = true;
                            let _ = self.rt.notify(&self.plugin, wire::CANCEL, json!({ "call": call }));
                        }
                    }
                }
            };
            // What the plugin sent before its answer has been read before it (one pipe, in order).
            while let Ok(a) = rx.try_recv() {
                on(a);
            }
            self.sounds.lock().remove(&call);
            match r {
                Ok(v) => Ok(v["firstMs"]
                    .as_f64()
                    .map(|ms| Duration::from_secs_f64(ms / 1000.0))),
                Err(e) => Err(self.error(e)),
            }
        })
    }

    fn catalog(&self, listed: Option<&Listed>, error: Option<String>) -> ServiceCatalog {
        let params = json!({ "listed": listed, "error": error });
        let (rt, plugin) = (self.rt.clone(), self.plugin.clone());
        block(async move { rt.call(&plugin, wire::CATALOG, params).await })
            .and_then(Result::ok)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_else(|| ServiceCatalog::empty(self.provider.id()))
    }

    fn list<'a>(&'a self, spec: &'a Spec) -> SendFuture<'a, Result<Listed, OnlineError>> {
        Box::pin(async move {
            let v = self
                .rt
                .call(
                    &self.plugin,
                    wire::LIST,
                    json!({ "spec": wire::spec_json(spec) }),
                )
                .await
                .map_err(|e| self.error(e))?;
            serde_json::from_value(v)
                .map_err(|e| self.error(CallError::Invalid(format!("{}: {e}", wire::LIST))))
        })
    }
}

/// The ids of the services whose plugins the app gives (`dev.escouade.<id>`).
pub const BUILTIN: [&str; 5] = ["openai", "groq", "mistral", "elevenlabs", "deepgram"];

/// Puts the registry of the voice services in line with the plugins' switches: a plugin that is on serves its service; one
/// that is off or uninstalled takes its service away, and it stops. A plugin that does not start says so at the first call
/// that needs it, and its journal says why.
pub fn sync_builtin(rt: &Arc<Runtime>, plugins: &crate::plugins::Plugins) {
    for id in BUILTIN {
        let plugin = format!("dev.escouade.{id}");
        let on = plugins.choices(&plugin).map(|c| c.enabled).unwrap_or(false);
        if !on {
            super::service::hide(id);
            rt.stop(&plugin);
            continue;
        }
        if !super::service::is_plugin(id) {
            let service = ProcessVoiceService::described(rt.clone(), &plugin, Provider::of(id));
            super::service::register(service);
        }
    }
}
