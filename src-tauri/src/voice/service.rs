//! The contract of the voice services on line (INT-12, ESC-284): what Escouade asks of a service that
//! transcribes a dictation, transcribes as one speaks, reads a text aloud, lists its models and voices, and
//! keeps an account (a key in the keychain). The core calls only this; who answers is a registry's matter.
//!
//! OpenAI, Groq, Mistral, ElevenLabs and Deepgram answer it from the core for now (`CoreService`, the code of
//! ESC-110, 132, 134, 136 and 137, unchanged); a plugin will answer it later (INT-13), and Wispr Flow
//! (ESC-164) is written on it. Whisper and Piper are not services on line: offline, without an account, they stay
//! in the core.

use super::live::{LiveStream, Partial};
use super::online::{self, OnlineError, Provider, Spec};
use super::online_catalog::{self, Listed, ServiceCatalog};
use super::online_speech::{self, SpeakChoice};
use super::speech::Audio;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

/// A future the caller may send to another thread.
pub type SendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
/// A future driven on the thread that made it (the reading hands its sound to a closure that stays there).
pub type LocalFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// One service on line.
pub trait VoiceService: Send + Sync {
    /// Its id in the settings, the keychain and the statistics.
    fn id(&self) -> String;
    fn label(&self) -> String;
    /// The core's own service of this id, when it is one: the specification (`Spec`) is still built on it.
    fn provider(&self) -> Option<Provider>;

    // The account: a key in the system's keychain, never in a file.
    /// Its key's entry in the keychain.
    fn account_entry(&self) -> String;

    // The dictation of a file.
    /// The model it transcribes with when the user chose none.
    fn default_model(&self) -> String;
    /// The text of `samples` (mono, 16 kHz, in [-1, 1]); gives up, with no text, once `cancel` is set. Blocks.
    fn transcribe(
        &self,
        spec: &Spec,
        samples: &[f32],
        language: &str,
        cancel: &Arc<AtomicBool>,
    ) -> Result<String, OnlineError>;

    // The transcription as one speaks.
    /// Whether it transcribes as one speaks (ESC-136).
    fn offers_live(&self) -> bool;
    /// The model it streams with, `chosen` being the one of the settings.
    fn live_model(&self, chosen: &str) -> String;
    /// Opens the stream of `spec` for sound at `rate`; the text comes through `partial`, the final one at `finish`.
    fn open_live(&self, spec: &Spec, language: &str, rate: u32, partial: Partial) -> LiveStream;

    // The synthesis.
    /// The most characters it reads in one request (a text is cut at sentences before).
    fn max_chars(&self) -> usize;
    /// Reads `sentence`: its sound given to `on` as it comes; gives up, without an error, once `cancel` is set.
    /// Returns how long its first sound took.
    fn speak<'a>(
        &'a self,
        spec: &'a Spec,
        choice: &'a SpeakChoice,
        sentence: &'a str,
        cancel: &'a AtomicBool,
        on: &'a mut dyn FnMut(Audio),
    ) -> LocalFuture<'a, Result<Option<Duration>, OnlineError>>;

    // The catalog.
    /// What the documentation lists, or `listed` when the service was asked.
    fn catalog(&self, listed: Option<&Listed>, error: Option<String>) -> ServiceCatalog;
    /// Reads again what its API lists (its models and voices).
    fn list<'a>(&'a self, spec: &'a Spec) -> SendFuture<'a, Result<Listed, OnlineError>>;
}

/// The services of the core: the five of ESC-110, behind the contract.
pub struct CoreService(pub Provider);

impl VoiceService for CoreService {
    fn id(&self) -> String {
        self.0.id().into()
    }
    fn label(&self) -> String {
        self.0.label().into()
    }
    fn provider(&self) -> Option<Provider> {
        Some(self.0)
    }
    fn account_entry(&self) -> String {
        self.0.entry()
    }
    fn default_model(&self) -> String {
        self.0.model().into()
    }
    fn transcribe(
        &self,
        spec: &Spec,
        samples: &[f32],
        language: &str,
        cancel: &Arc<AtomicBool>,
    ) -> Result<String, OnlineError> {
        // A runtime of its own, on a thread of its own: `Voice::stop` blocks.
        let spec = spec.clone();
        let samples = samples.to_vec();
        let language = language.to_string();
        let cancel = cancel.clone();
        let p = self.0;
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| OnlineError::Network(p, e.to_string()))?
                .block_on(online::transcribe(&spec, &samples, &language, &cancel))
        })
        .join()
        .unwrap_or_else(|_| {
            Err(OnlineError::Network(
                p,
                "the transcription's thread panicked".into(),
            ))
        })
    }
    fn offers_live(&self) -> bool {
        self.0.offers_live()
    }
    fn live_model(&self, chosen: &str) -> String {
        self.0.live_model(chosen)
    }
    fn open_live(&self, spec: &Spec, language: &str, rate: u32, partial: Partial) -> LiveStream {
        LiveStream::open(spec, language, rate, partial)
    }
    fn max_chars(&self) -> usize {
        online_speech::max_chars(self.0)
    }
    fn speak<'a>(
        &'a self,
        spec: &'a Spec,
        choice: &'a SpeakChoice,
        sentence: &'a str,
        cancel: &'a AtomicBool,
        on: &'a mut dyn FnMut(Audio),
    ) -> LocalFuture<'a, Result<Option<Duration>, OnlineError>> {
        Box::pin(online_speech::speak(spec, choice, sentence, cancel, on))
    }
    fn catalog(&self, listed: Option<&Listed>, error: Option<String>) -> ServiceCatalog {
        online_catalog::catalog(self.0, listed, error)
    }
    fn list<'a>(&'a self, spec: &'a Spec) -> SendFuture<'a, Result<Listed, OnlineError>> {
        Box::pin(online_catalog::fetch(spec))
    }
}

/// The services of the plugins, by the registry (`register`): the core's five give way to a plugin of the same id.
static PLUGINS: parking_lot::RwLock<Vec<Arc<dyn VoiceService>>> =
    parking_lot::RwLock::new(Vec::new());

/// Adds the service of a plugin, replacing the one of the same id (the core's or an earlier plugin's).
pub fn register(service: Arc<dyn VoiceService>) {
    let mut all = PLUGINS.write();
    all.retain(|s| s.id() != service.id());
    all.push(service);
}

/// Takes the service of this id back from the plugins (the plugin stopped or removed): the core's own, if any,
/// answers again.
pub fn unregister(id: &str) {
    PLUGINS.write().retain(|s| s.id() != id);
}

/// The services Escouade knows, in the order of the settings: the core's, a plugin's of the same id in its place,
/// then the plugins' others.
pub fn services() -> Vec<Arc<dyn VoiceService>> {
    let plugins = PLUGINS.read().clone();
    let mut out: Vec<Arc<dyn VoiceService>> = online::PROVIDERS
        .into_iter()
        .map(|p| {
            plugins
                .iter()
                .find(|s| s.id() == p.id())
                .cloned()
                .unwrap_or_else(|| Arc::new(CoreService(p)) as Arc<dyn VoiceService>)
        })
        .collect();
    out.extend(
        plugins
            .into_iter()
            .filter(|s| online::Provider::core(&s.id()).is_none()),
    );
    out
}

/// The service of this id.
pub fn find(id: &str) -> Option<Arc<dyn VoiceService>> {
    services().into_iter().find(|s| s.id() == id)
}

/// The service that answers for `p`: a plugin's, else the core's. A service whose plugin went away answers every
/// call with an error (`Gone`) rather than a panic.
pub fn of(p: Provider) -> Arc<dyn VoiceService> {
    if let Some(s) = PLUGINS.read().iter().find(|s| s.id() == p.id()) {
        return s.clone();
    }
    if Provider::core(p.id()).is_some() {
        Arc::new(CoreService(p))
    } else {
        Arc::new(Gone(p))
    }
}

/// A service no one answers for any more (its plugin was removed while a setting still names it).
struct Gone(Provider);

impl VoiceService for Gone {
    fn id(&self) -> String {
        self.0.id().into()
    }
    fn label(&self) -> String {
        self.0.id().into()
    }
    fn provider(&self) -> Option<Provider> {
        None
    }
    fn account_entry(&self) -> String {
        self.0.entry()
    }
    fn default_model(&self) -> String {
        String::new()
    }
    fn transcribe(
        &self,
        _: &Spec,
        _: &[f32],
        _: &str,
        _: &Arc<AtomicBool>,
    ) -> Result<String, OnlineError> {
        Err(OnlineError::Network(self.0, gone()))
    }
    fn offers_live(&self) -> bool {
        false
    }
    fn live_model(&self, chosen: &str) -> String {
        chosen.into()
    }
    fn open_live(&self, spec: &Spec, language: &str, rate: u32, partial: Partial) -> LiveStream {
        LiveStream::open(spec, language, rate, partial)
    }
    fn max_chars(&self) -> usize {
        1000
    }
    fn speak<'a>(
        &'a self,
        _: &'a Spec,
        _: &'a SpeakChoice,
        _: &'a str,
        _: &'a AtomicBool,
        _: &'a mut dyn FnMut(Audio),
    ) -> LocalFuture<'a, Result<Option<Duration>, OnlineError>> {
        let p = self.0;
        Box::pin(async move { Err(OnlineError::Network(p, gone())) })
    }
    fn catalog(&self, _: Option<&Listed>, _: Option<String>) -> ServiceCatalog {
        ServiceCatalog::empty(&self.0.id())
    }
    fn list<'a>(&'a self, _: &'a Spec) -> SendFuture<'a, Result<Listed, OnlineError>> {
        let p = self.0;
        Box::pin(async move { Err(OnlineError::Network(p, gone())) })
    }
}

fn gone() -> String {
    tr!(
        "le plugin de ce service n’est plus là",
        "this service’s plugin is gone"
    )
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
