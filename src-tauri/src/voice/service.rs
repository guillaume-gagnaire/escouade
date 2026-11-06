//! The contract of the voice services on line (INT-12, ESC-284): what Escouade asks of a service that transcribes a
//! dictation, transcribes as one speaks, reads a text aloud, lists its models and voices, and keeps an account (a key in
//! the keychain). The core calls only this; who answers is a registry's matter: its plugin (`plugin_host`, a module since
//! ESC-350), or no one (`Gone`). Whisper and Piper are not services on line: offline, without an account, they stay in the
//! core.

use super::live::{LiveStream, Partial};
use super::online::{self, OnlineError, Provider, Spec};
use super::online_catalog::{Listed, ServiceCatalog};
use super::online_speech::SpeakChoice;
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

/// The services of the plugins, by the registry (`register`).
static PLUGINS: parking_lot::RwLock<Vec<Arc<dyn VoiceService>>> =
    parking_lot::RwLock::new(Vec::new());

/// The services of Escouade that the user turned off with their plugin: neither listed nor found.
static HIDDEN: parking_lot::RwLock<Vec<String>> = parking_lot::RwLock::new(Vec::new());

/// The service `id` is off (its plugin turned off): it is neither listed nor found.
pub fn hide(id: &str) {
    PLUGINS.write().retain(|s| s.id() != id);
    let mut hidden = HIDDEN.write();
    if !hidden.iter().any(|h| h == id) {
        hidden.push(id.to_string());
    }
}

/// Adds the service of a plugin, replacing an earlier plugin's of the same id.
pub fn register(service: Arc<dyn VoiceService>) {
    HIDDEN.write().retain(|h| *h != service.id());
    let mut all = PLUGINS.write();
    all.retain(|s| s.id() != service.id());
    all.push(service);
}

/// Whether a plugin serves the service `id`.
pub fn is_plugin(id: &str) -> bool {
    PLUGINS.read().iter().any(|s| s.id() == id)
}

/// The services Escouade knows, in the order of the settings: its five, each by its plugin (`Gone` while none serves it, so
/// that its settings and its key stay reachable), then the plugins' others.
pub fn services() -> Vec<Arc<dyn VoiceService>> {
    let plugins = PLUGINS.read().clone();
    let hidden = HIDDEN.read().clone();
    let mut out: Vec<Arc<dyn VoiceService>> = online::PROVIDERS
        .into_iter()
        .filter(|p| !hidden.iter().any(|h| h == p.id()))
        .map(|p| {
            plugins
                .iter()
                .find(|s| s.id() == p.id())
                .cloned()
                .unwrap_or_else(|| Arc::new(Gone(p)) as Arc<dyn VoiceService>)
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

/// The service that answers for `p`: its plugin. A service whose plugin is not there answers every call with an error
/// (`Gone`) rather than a panic.
pub fn of(p: Provider) -> Arc<dyn VoiceService> {
    if let Some(s) = PLUGINS.read().iter().find(|s| s.id() == p.id()) {
        return s.clone();
    }
    // The tests of the app play the real plugins (`plugged`).
    #[cfg(test)]
    if Provider::core(p.id()).is_some() && !HIDDEN.read().iter().any(|h| h == p.id()) {
        let service = super::plugged::plugged(p);
        let mut all = PLUGINS.write();
        return match all.iter().find(|s| s.id() == p.id()) {
            Some(first) => first.clone(),
            None => {
                all.push(service.clone());
                service
            }
        };
    }
    Arc::new(Gone(p))
}

/// A service no one answers for: its plugin does not run, or was removed while a setting still names it. What the app
/// shows of one of the five stays right, from the tables of `escouade_voice`.
struct Gone(Provider);

impl VoiceService for Gone {
    fn id(&self) -> String {
        self.0.id().into()
    }
    fn label(&self) -> String {
        self.0.label().into()
    }
    fn account_entry(&self) -> String {
        self.0.entry()
    }
    fn default_model(&self) -> String {
        self.0.model().into()
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
        Provider::core(self.0.id()).is_some() && self.0.offers_live()
    }
    fn live_model(&self, chosen: &str) -> String {
        self.0.live_model(chosen)
    }
    fn open_live(&self, _: &Spec, _: &str, _: u32, _: Partial) -> LiveStream {
        LiveStream::failed(self.0, OnlineError::Network(self.0, gone()))
    }
    fn max_chars(&self) -> usize {
        escouade_voice::speech::max_chars(self.0)
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
    fn catalog(&self, listed: Option<&Listed>, error: Option<String>) -> ServiceCatalog {
        if Provider::core(self.0.id()).is_some() {
            super::online_catalog::catalog(self.0, listed, error)
        } else {
            ServiceCatalog::empty(self.0.id())
        }
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
