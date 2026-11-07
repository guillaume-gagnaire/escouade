//! Dictation: the microphone listened to on the backend's side (cpal), the sound brought to what
//! Whisper takes (mono, 16 kHz), its level told to the window while it listens, then transcribed
//! on the machine (whisper.cpp, on a thread of its own) once the user stops. Nothing leaves the
//! machine but the download of a model, through the proxy of the settings; or, « En ligne » in the
//! settings (off by default, `online`), the sound of the dictation to the service the user chose.
//!
//! One listening at a time, for the whole app: it stops when the user says so, when the window is
//! closed or reloaded, when the app quits, and after `MAX_LISTEN`. The text goes back to the window,
//! which puts it where the user dictated (the composer at its cursor, a ticket's description, the
//! terminal of the Claude Code view); it is never sent by itself.

pub mod audio;
pub mod conversation;
pub mod live;
pub mod models;
pub mod online;
pub mod online_catalog;
pub mod online_speech;
#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod plugged;
pub mod plugin_host;
#[cfg(test)]
mod plugin_tests;
pub mod service;
pub mod source;
pub mod speech;
pub mod spoken;
#[cfg(test)]
mod tests;
pub mod transcribe;
pub mod vad;

use crate::i18n::Lang;
use models::{ModelView, Store};
use parking_lot::Mutex;
use serde::Serialize;
use source::{Capture, Device, Sink, Source};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use transcribe::Transcriber;

/// A listening stops by itself after this long: the sound kept would grow without end otherwise.
pub const MAX_LISTEN: Duration = Duration::from_secs(10 * 60);

/// How often the level is told to the window.
const TICK: Duration = Duration::from_millis(50);

/// What went wrong, as the window tells it: each with what to do about it.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceError {
    /// No microphone at all (or the one chosen and the system's both gone).
    NoDevice,
    /// The system keeps the microphone from the app.
    Denied,
    /// Another app holds it.
    Busy,
    /// No sound system answers (Linux: neither PulseAudio, PipeWire nor ALSA).
    NoAudioSystem,
    /// It listened, but not one sample was not zero: what macOS gives an app it keeps the
    /// microphone from (it refuses nothing), and a muted one.
    Silent,
    /// Already listening.
    AlreadyListening,
    /// The service on line did not transcribe it (ESC-110).
    Online(online::OnlineError),
    Other(String),
}

impl VoiceError {
    /// What the window says, in `lang`, on `os` ("windows", "macos", "linux").
    pub fn message(&self, lang: Lang, os: &str) -> String {
        match self {
            VoiceError::NoDevice => tr_in!(
                lang,
                "Aucun micro trouvé. Branche un micro, ou choisis-en un dans Réglages › Voix.",
                "No microphone found. Plug one in, or pick one in Settings › Voice."
            ),
            VoiceError::Denied | VoiceError::Silent => {
                let steps = match (os, lang) {
                    ("macos", Lang::Fr) => "ouvre Réglages Système › Confidentialité et sécurité › Microphone, coche Escouade, puis relance l’app.",
                    ("macos", Lang::En) => "open System Settings › Privacy & Security › Microphone, tick Escouade, then restart the app.",
                    ("windows", Lang::Fr) => "ouvre Paramètres › Confidentialité et sécurité › Microphone, et allume « Accès au microphone » puis « Autoriser les applications de bureau à accéder à votre microphone ».",
                    ("windows", Lang::En) => "open Settings › Privacy & security › Microphone, and turn on “Microphone access”, then “Let desktop apps access your microphone”.",
                    (_, Lang::Fr) => "vérifie que PulseAudio ou PipeWire tourne, que le micro n’est pas coupé (pavucontrol, ou les réglages du son), et que ton compte peut s’en servir (groupe audio).",
                    (_, Lang::En) => "check that PulseAudio or PipeWire is running, that the microphone is not muted (pavucontrol, or the sound settings), and that your account may use it (audio group).",
                };
                if *self == VoiceError::Silent {
                    tr_in!(lang, "Aucun son n’est arrivé du micro : il est coupé, ou le système le refuse à Escouade. Pour l’autoriser, {steps}", "No sound came from the microphone: it is muted, or the system keeps it from Escouade. To allow it, {steps}")
                } else {
                    tr_in!(lang, "Le système refuse le micro à Escouade. Pour l’autoriser, {steps}", "The system keeps the microphone from Escouade. To allow it, {steps}")
                }
            }
            VoiceError::Busy => tr_in!(
                lang,
                "Le micro est pris par une autre application. Ferme-la, ou choisis un autre micro dans Réglages › Voix.",
                "The microphone is taken by another app. Close it, or pick another microphone in Settings › Voice."
            ),
            VoiceError::NoAudioSystem => tr_in!(
                lang,
                "Aucun système de son ne répond. Démarre PulseAudio ou PipeWire (ou vérifie ALSA), puis réessaie.",
                "No sound system answers. Start PulseAudio or PipeWire (or check ALSA), then try again."
            ),
            VoiceError::AlreadyListening => tr_in!(lang, "Une dictée est déjà en cours.", "A dictation is already under way."),
            VoiceError::Online(e) => e.message(crate::i18n::kit(lang)),
            VoiceError::Other(e) => tr_in!(lang, "La dictée a échoué : {e}", "Dictation failed: {e}"),
        }
    }
}

impl std::fmt::Display for VoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message(Lang::En, std::env::consts::OS))
    }
}

/// What the window is told while it listens and downloads.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VoiceEvent {
    /// How loud it is now (0 to 1), and for how long it has listened.
    #[serde(rename_all = "camelCase")]
    Level { session: u64, level: f32, ms: u64 },
    /// What the service on line has understood so far of the dictation, as one speaks (ESC-136).
    #[serde(rename_all = "camelCase")]
    Partial { session: u64, text: String },
    /// It stopped listening by itself (`MAX_LISTEN`, the microphone gone): `error` says why when it
    /// failed. The window then asks for the text (`stop`), or drops it.
    #[serde(rename_all = "camelCase")]
    Ended { session: u64, error: Option<String> },
    /// A model's download: bytes so far, of `total`.
    #[serde(rename_all = "camelCase")]
    Download {
        model: String,
        done: u64,
        total: u64,
    },
    /// A voice's download (the conversation's).
    #[serde(rename_all = "camelCase")]
    SpeechDownload {
        voice: String,
        done: u64,
        total: u64,
    },
    /// The conversation: how loud the microphone is now (0 to 1), and what is played (the agent's voice, 0 to 1).
    #[serde(rename_all = "camelCase")]
    ConvLevel {
        session: u64,
        level: f32,
        played: f32,
    },
    /// The sentence `index` of the reading `reading` starts to be heard: its subtitle.
    #[serde(rename_all = "camelCase")]
    ConvSentence {
        session: u64,
        reading: u64,
        index: u32,
        text: String,
    },
    /// A sentence started.
    #[serde(rename_all = "camelCase")]
    ConvSpeech { session: u64 },
    /// What the service on line has understood so far of the sentence under way, as one speaks (ESC-136): the whole
    /// of it, replaced by the next.
    #[serde(rename_all = "camelCase")]
    ConvPartial { session: u64, text: String },
    /// It ended (its silence came): it is being transcribed.
    #[serde(rename_all = "camelCase")]
    ConvTranscribing { session: u64 },
    /// Its text (empty: nothing understood), or why it could not be transcribed. The conversation holds until told.
    #[serde(rename_all = "camelCase")]
    ConvHeard {
        session: u64,
        text: String,
        error: Option<String>,
    },
    /// The reading `reading` was heard to its end (or could not be made: `error`); it listens again.
    #[serde(rename_all = "camelCase")]
    ConvSpoken {
        session: u64,
        reading: u64,
        error: Option<String>,
        /// The voice on line failed and no voice of Piper's stands in: the conversation is on pause (ESC-132).
        paused: bool,
    },
    /// What the conversation says once (ESC-132): the voice on line failed and Piper reads now, or the voice chosen
    /// is refused and the service's default one reads.
    #[serde(rename_all = "camelCase")]
    ConvNotice { session: u64, message: String },
    /// The user spoke over the reading `reading`: it stopped, and the sentence is being listened to.
    #[serde(rename_all = "camelCase")]
    ConvBargeIn { session: u64, reading: u64 },
    /// The microphone was lost: the conversation is over.
    #[serde(rename_all = "camelCase")]
    ConvEnded { session: u64, error: Option<String> },
}

/// What `start` answers.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Started {
    /// It listens.
    Listening { session: u64 },
    /// The model must be downloaded first: its size, for the window to ask.
    NeedsModel { model: String, size: u64 },
}

/// What a listening is asked to use, from the settings.
#[derive(Debug, Clone)]
pub struct Choice {
    pub device: String,
    pub model: String,
    /// Whisper's code, or "auto".
    pub language: String,
    /// « En ligne »: the service the sound goes to, with its key (ESC-110); none: Whisper, on the machine.
    pub online: Option<online::Spec>,
}

/// The service of « En ligne », when the settings choose it (ESC-110); none: Whisper, on the machine.
pub fn online_provider(settings: &crate::model::VoiceSettings) -> Option<online::Provider> {
    (settings.transcription == "online")
        .then(|| online::find(&settings.online_service))
        .flatten()
}

/// The language Whisper is told: the setting's ("ui": the interface's, "auto": found by Whisper).
pub fn whisper_language(setting: &str, ui: Lang) -> String {
    match setting.trim() {
        "" | "ui" => match ui {
            Lang::Fr => "fr".into(),
            Lang::En => "en".into(),
        },
        other => other.to_string(),
    }
}

struct Session {
    id: u64,
    /// Shared with the ticker, which lets the microphone go when the listening ends by itself.
    capture: Arc<Mutex<Option<Box<dyn Capture>>>>,
    samples: Arc<Mutex<Vec<f32>>>,
    rate: u32,
    /// Set, the ticker stops.
    done: Arc<AtomicBool>,
    /// Whisper's model; empty on line.
    model: PathBuf,
    language: String,
    transcriber: Arc<dyn Transcriber>,
    /// « Transcription en temps réel » (ESC-136): the stream the sound goes to as it is heard.
    live: Arc<Mutex<Option<live::LiveStream>>>,
    provider: Option<online::Provider>,
}

/// How the window is told, and in what language.
pub type Emit = Arc<dyn Fn(VoiceEvent) + Send + Sync>;

pub struct Voice {
    source: Box<dyn Source>,
    transcriber: Arc<dyn Transcriber>,
    pub store: Store,
    /// What the services on line listed of their models and voices (ESC-132).
    pub catalog: online_catalog::CatalogStore,
    /// The conversation's voice: what speaks, where it is heard, its voices and where they come from.
    engine: Arc<dyn speech::Engine>,
    output: Arc<dyn speech::output::Output>,
    pub voices: speech::VoiceStore,
    pub voices_url: String,
    conv: Mutex<Option<conversation::Conversation>>,
    /// What « Écouter » of the settings plays.
    preview: Mutex<Option<Arc<dyn speech::output::Playing>>>,
    /// Where the models are downloaded from (`models::MODELS_URL`; a fake server in tests).
    pub models_url: String,
    emit: Emit,
    session: Mutex<Option<Session>>,
    next: AtomicU64,
    /// The transcription under way, its cancel flag.
    transcribing: Mutex<Option<Arc<AtomicBool>>>,
    /// The downloads under way, by model, their cancel flags.
    downloads: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// A start is opening the microphone.
    opening: AtomicBool,
    /// Called off while the microphone opened: it is let go as soon as it is open.
    called_off: AtomicBool,
    max_listen: Duration,
    /// The fake transcriber stands for the service on line too (`ESCOUADE_VOICE_TRANSCRIPT`, tests).
    pub fake_online: bool,
}

impl Voice {
    pub fn new(
        source: Box<dyn Source>,
        transcriber: Arc<dyn Transcriber>,
        store: Store,
        models_url: String,
        emit: Emit,
    ) -> Self {
        let voices = speech::VoiceStore {
            dir: store
                .dir
                .parent()
                .map(|p| p.join("voices"))
                .unwrap_or_else(|| PathBuf::from("voices")),
        };
        let catalog = online_catalog::CatalogStore::in_dir(
            store.dir.parent().unwrap_or(std::path::Path::new(".")),
        );
        Self {
            source,
            transcriber,
            catalog,
            store,
            engine: Arc::new(speech::Fake::new(None)),
            output: Arc::new(speech::output::Silent),
            voices,
            voices_url: speech::VOICES_URL.into(),
            conv: Mutex::new(None),
            preview: Mutex::new(None),
            models_url,
            emit,
            session: Mutex::new(None),
            next: AtomicU64::new(1),
            transcribing: Mutex::new(None),
            downloads: Mutex::new(HashMap::new()),
            max_listen: MAX_LISTEN,
            opening: AtomicBool::new(false),
            called_off: AtomicBool::new(false),
            fake_online: false,
        }
    }

    /// What speaks and where it is heard.
    pub fn with_speech(
        mut self,
        engine: Arc<dyn speech::Engine>,
        output: Arc<dyn speech::output::Output>,
    ) -> Self {
        self.engine = engine;
        self.output = output;
        self
    }

    #[cfg(test)]
    pub fn with_max_listen(mut self, d: Duration) -> Self {
        self.max_listen = d;
        self
    }

    /// The app's: the microphone and whisper.cpp; in a debug build, `ESCOUADE_VOICE_WAV` plays a
    /// WAV file in place of the microphone, `ESCOUADE_VOICE_TRANSCRIPT` gives its text in place of
    /// Whisper, and `ESCOUADE_VOICE_MODELS_URL` is where the models come from (end-to-end tests).
    pub fn for_app(data: PathBuf, emit: Emit) -> Self {
        whisper_rs::install_logging_hooks();
        let var = |k: &str| {
            if cfg!(debug_assertions) {
                std::env::var(k).ok().filter(|v| !v.trim().is_empty())
            } else {
                None
            }
        };
        let source: Box<dyn Source> = match var("ESCOUADE_VOICE_WAV") {
            Some(wav) => Box::new(source::WavFile(wav.into())),
            None => Box::new(source::Microphone),
        };
        let fake = var("ESCOUADE_VOICE_TRANSCRIPT");
        let fake_online = fake.is_some();
        let transcriber: Arc<dyn Transcriber> = match fake {
            Some(text) => Arc::new(transcribe::Fake::new(&text)),
            None => Arc::new(transcribe::Whisper::default()),
        };
        let url = var("ESCOUADE_VOICE_MODELS_URL").unwrap_or_else(|| models::MODELS_URL.into());
        // `ESCOUADE_SPEECH_FAKE`: a fake voice, which writes what it says to that file, heard nowhere;
        // `ESCOUADE_SPEECH_SILENT`: Piper, played in silence (the real voices, without speakers).
        let (engine, output): (Arc<dyn speech::Engine>, Arc<dyn speech::output::Output>) =
            match var("ESCOUADE_SPEECH_FAKE") {
                Some(log) => (
                    Arc::new(speech::Fake::new(Some(log.into()))),
                    Arc::new(speech::output::Silent),
                ),
                None => (
                    Arc::new(speech::piper::Piper::new(
                        data.join("voice").join("espeak-ng-data"),
                    )),
                    if var("ESCOUADE_SPEECH_SILENT").is_some() {
                        Arc::new(speech::output::Silent)
                    } else {
                        Arc::new(speech::output::Speakers)
                    },
                ),
            };
        let voices_url =
            var("ESCOUADE_SPEECH_VOICES_URL").unwrap_or_else(|| speech::VOICES_URL.into());
        let mut voice = Self::new(
            source,
            transcriber,
            Store {
                dir: data.join("voice").join("models"),
            },
            url,
            emit,
        )
        .with_speech(engine, output);
        voice.voices_url = voices_url;
        voice.fake_online = fake_online;
        voice
    }

    pub fn devices(&self) -> Vec<Device> {
        self.source.devices()
    }

    pub fn models(&self) -> Vec<ModelView> {
        self.store.list()
    }

    #[cfg(test)]
    pub fn listening(&self) -> Option<u64> {
        self.session.lock().as_ref().map(|s| s.id)
    }

    /// Starts listening with `choice`, unless the model is missing (the window asks to download it
    /// first) or a listening is already under way.
    pub fn start(&self, choice: &Choice) -> Result<Started, VoiceError> {
        let model = models::find(&choice.model)
            .or_else(|| models::find(models::DEFAULT_MODEL))
            .expect("the default model is in the catalog");
        // On line, no model is needed.
        if choice.online.is_none() && !self.store.has(model) {
            return Ok(Started::NeedsModel {
                model: model.id.into(),
                size: model.size,
            });
        }
        // Not held while the microphone opens (macOS asks the user first): a window closed
        // meanwhile is not kept waiting. `opening` keeps a second start out until then.
        if self.session.lock().is_some()
            || self.conv.lock().is_some()
            || self.opening.swap(true, Ordering::AcqRel)
        {
            return Err(VoiceError::AlreadyListening);
        }
        self.called_off.store(false, Ordering::Release);
        let opened = self.open(choice, model);
        self.opening.store(false, Ordering::Release);
        opened
    }

    fn open(&self, choice: &Choice, model: &models::ModelInfo) -> Result<Started, VoiceError> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let peak = Arc::new(AtomicU32::new(0));
        let failure = Arc::new(Mutex::new(None::<VoiceError>));
        let stream = Arc::new(Mutex::new(None::<live::LiveStream>));
        let sink = {
            let samples = samples.clone();
            let peak = peak.clone();
            let failure = failure.clone();
            let stream = stream.clone();
            Sink {
                samples: Box::new(move |chunk: &[f32]| {
                    let l = audio::level(chunk);
                    peak.fetch_max(l.to_bits(), Ordering::Relaxed);
                    let mut kept = samples.lock();
                    kept.extend_from_slice(chunk);
                    if let Some(s) = stream.lock().as_ref() {
                        s.push(chunk);
                    }
                }),
                error: Box::new(move |e| {
                    failure.lock().get_or_insert(e);
                }),
            }
        };
        let capture = self.source.open(&choice.device, sink)?;
        let rate = capture.rate();
        log::info!("voice: listening at {rate} Hz");
        let done = Arc::new(AtomicBool::new(false));
        let capture = Arc::new(Mutex::new(Some(capture)));
        // On line: the service's (the fake transcriber of the tests stands for it too).
        let (transcriber, model_path): (Arc<dyn Transcriber>, PathBuf) = match &choice.online {
            Some(spec) if !self.fake_online => {
                (Arc::new(online::Online(spec.clone())), PathBuf::new())
            }
            Some(_) => (self.transcriber.clone(), PathBuf::new()),
            None => (self.transcriber.clone(), self.store.path(model.id)),
        };
        // As it is heard (ESC-136): the stream opens now, what was heard while the microphone opened sent first.
        if let Some(spec) = choice
            .online
            .as_ref()
            .filter(|s| s.live.is_some() && !self.fake_online)
        {
            let kept = samples.lock();
            let emit = self.emit.clone();
            let opened = service::of(spec.provider).open_live(
                spec,
                &choice.language,
                rate,
                Box::new(move |text| emit(VoiceEvent::Partial { session: id, text })),
            );
            opened.push(&kept);
            *stream.lock() = Some(opened);
            log::info!(
                "voice: the dictation is transcribed by {} as it is said ({})",
                spec.provider.label(),
                spec.live.as_deref().unwrap_or_default()
            );
        }
        let mut slot = self.session.lock();
        if slot.is_some() {
            return Err(VoiceError::AlreadyListening);
        }
        if self.called_off.load(Ordering::Acquire) {
            log::info!("voice: listening called off");
            return Err(VoiceError::Other("called off".into()));
        }
        // Loaded while the user speaks: the text comes sooner once they stop.
        if choice.online.is_none() {
            let t = transcriber.clone();
            let p = model_path.clone();
            std::thread::spawn(move || t.preload(&p));
        }
        *slot = Some(Session {
            id,
            capture: capture.clone(),
            samples,
            rate,
            done: done.clone(),
            model: model_path,
            language: choice.language.clone(),
            transcriber,
            live: stream,
            provider: choice.online.as_ref().map(|s| s.provider),
        });
        drop(slot);
        let emit = self.emit.clone();
        let max = self.max_listen;
        let started = Instant::now();
        std::thread::Builder::new()
            .name("voice-level".into())
            .spawn(move || {
                while !done.load(Ordering::Acquire) {
                    std::thread::sleep(TICK);
                    if done.load(Ordering::Acquire) {
                        return;
                    }
                    // Positive floats order as their bits do.
                    let level = f32::from_bits(peak.swap(0, Ordering::Relaxed));
                    let ms = started.elapsed().as_millis() as u64;
                    emit(VoiceEvent::Level {
                        session: id,
                        level,
                        ms,
                    });
                    let failed = failure.lock().take();
                    if failed.is_some() || started.elapsed() >= max {
                        drop(capture.lock().take());
                        emit(VoiceEvent::Ended {
                            session: id,
                            error: failed
                                .map(|e| e.message(crate::i18n::ui(), std::env::consts::OS)),
                        });
                        return;
                    }
                }
            })
            .map_err(|e| VoiceError::Other(e.to_string()))?;
        Ok(Started::Listening { session: id })
    }

    /// Stops listening (the session `id`, if it is the one under way) and transcribes what was
    /// heard: blocks until the text is there (call it off the async runtime). Empty when nothing
    /// was said.
    pub fn stop(&self, id: u64) -> Result<String, VoiceError> {
        let session = {
            let mut slot = self.session.lock();
            match slot.as_ref() {
                Some(s) if s.id == id => slot.take().unwrap(),
                _ => return Err(VoiceError::Other("no dictation under way".into())),
            }
        };
        session.done.store(true, Ordering::Release);
        // The microphone is let go before the transcription, which may take seconds.
        drop(session.capture.lock().take());
        let stream = session.live.lock().take();
        let stopped = Instant::now();
        let heard = std::mem::take(&mut *session.samples.lock());
        let secs = heard.len() as f32 / session.rate.max(1) as f32;
        log::info!("voice: {secs:.1} s heard");
        if heard.is_empty() {
            return Ok(String::new());
        }
        // Half a second of nothing but zeros: kept from the app, not quiet (a quiet room is
        // never digital silence).
        if secs >= 0.5 && audio::all_zero(&heard) {
            return Err(VoiceError::Silent);
        }
        let pcm = audio::resample(&heard, session.rate, audio::WHISPER_RATE);
        drop(heard);
        // Under a tenth of a second: a click, nothing said.
        if pcm.len() < audio::WHISPER_RATE as usize / 10 {
            return Ok(String::new());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *self.transcribing.lock() = Some(cancel.clone());
        // As it was said: its final text, there a moment after the stop; a stream that failed, by request.
        if let Some(stream) = stream {
            let p = stream_label(&session);
            match stream.finish(live::FINAL_WAIT) {
                Ok(f) => {
                    self.transcribing.lock().take();
                    let ms = f.ready.saturating_duration_since(stopped).as_millis();
                    log::info!("voice: a dictation of {secs:.1} s transcribed by {p}, live, ready {ms} ms after it stopped");
                    if cancel.load(Ordering::Acquire) {
                        return Ok(String::new());
                    }
                    return Ok(f.text);
                }
                Err(e) => log::warn!(
                    "voice: the live transcription by {p} failed ({e}): the dictation goes whole, by request"
                ),
            }
        }
        let result =
            session
                .transcriber
                .transcribe(&session.model, &pcm, &session.language, &cancel);
        if session.model.as_os_str().is_empty() {
            log::info!(
                "voice: a dictation of {secs:.1} s answered by request {} ms after it stopped",
                stopped.elapsed().as_millis()
            );
        }
        self.transcribing.lock().take();
        if cancel.load(Ordering::Acquire) {
            return Ok(String::new());
        }
        result.map_err(|e| match e.downcast::<online::OnlineError>() {
            Ok(e) => VoiceError::Online(e),
            Err(e) => VoiceError::Other(format!("{e:#}")),
        })
    }

    /// Stops everything: the listening (its sound dropped) and the transcription under way. The
    /// window closed or reloaded, the app quitting, the user's Escape.
    pub fn cancel(&self) {
        if self.opening.load(Ordering::Acquire) {
            self.called_off.store(true, Ordering::Release);
        }
        if let Some(s) = self.session.lock().take() {
            s.done.store(true, Ordering::Release);
            drop(s.capture.lock().take());
            log::info!("voice: listening called off");
        }
        if let Some(c) = self.transcribing.lock().as_ref() {
            c.store(true, Ordering::Release);
        }
        self.conversation_stop();
        self.preview.lock().take();
    }

    /// Everything stops for good: what the app does as it quits, the downloads too.
    pub fn shutdown(&self) {
        self.cancel();
        for flag in self.downloads.lock().values() {
            flag.store(true, Ordering::Release);
        }
    }

    /// Downloads the model `id` through `client`, its progress told to the window.
    pub async fn download(&self, client: &reqwest::Client, id: &str) -> anyhow::Result<()> {
        let Some(m) = models::find(id) else {
            anyhow::bail!("unknown model {id}");
        };
        if self.store.has(m) {
            return Ok(());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut downloads = self.downloads.lock();
            if downloads.contains_key(id) {
                anyhow::bail!(tr!(
                    "Ce modèle est déjà en cours de téléchargement.",
                    "This model is already being downloaded."
                ));
            }
            downloads.insert(id.to_string(), cancel.clone());
        }
        let emit = self.emit.clone();
        let total = m.size;
        let model = m.id.to_string();
        let r = self
            .store
            .download(client, &self.models_url, m, &cancel, |done| {
                emit(VoiceEvent::Download {
                    model: model.clone(),
                    done,
                    total,
                })
            })
            .await;
        self.downloads.lock().remove(id);
        match r {
            Ok(_) => Ok(()),
            Err(e) if e.is::<models::ChecksumMismatch>() => Err(anyhow::anyhow!(tr!(
                "Le fichier téléchargé n’est pas le bon modèle (somme de contrôle différente) : il n’est pas gardé. Réessaie ; derrière un proxy qui modifie les fichiers, télécharge-le ailleurs.",
                "The file downloaded is not the right model (checksum mismatch): it was not kept. Try again; behind a proxy that alters files, download it elsewhere."
            ))),
            Err(e) => Err(e),
        }
    }

    /// Calls off the download of `id`, if one is under way.
    pub fn cancel_download(&self, id: &str) {
        if let Some(flag) = self.downloads.lock().get(id) {
            flag.store(true, Ordering::Release);
        }
    }

    pub fn delete_model(&self, id: &str) -> anyhow::Result<()> {
        if models::find(id).is_none() {
            anyhow::bail!("unknown model {id}");
        }
        self.cancel_download(id);
        let path = self.store.path(id);
        self.transcriber.unload(&path);
        self.store.delete(id)
    }
}

/// Who transcribes a dictation, for the log.
fn stream_label(s: &Session) -> String {
    s.provider
        .map(|p| p.label().to_string())
        .unwrap_or_else(|| "the service".into())
}

/// What Réglages › Voix shows of the conversation's voices.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpeechInfo {
    /// Whether the app can speak on this machine (not on Intel Macs).
    pub available: bool,
    pub voices: Vec<speech::VoiceView>,
}

impl Voice {
    pub fn speech_info(&self) -> SpeechInfo {
        SpeechInfo {
            available: self.engine.available(),
            voices: self.voices.list(),
        }
    }

    /// Downloads the voice `id` through `client`, its progress told to the window.
    pub async fn download_voice(&self, client: &reqwest::Client, id: &str) -> anyhow::Result<()> {
        let Some(v) = speech::catalog::find(id) else {
            anyhow::bail!("unknown voice {id}");
        };
        if self.voices.has(v) {
            return Ok(());
        }
        let key = format!("voice:{id}");
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut downloads = self.downloads.lock();
            if downloads.contains_key(&key) {
                anyhow::bail!(tr!(
                    "Cette voix est déjà en cours de téléchargement.",
                    "This voice is already being downloaded."
                ));
            }
            downloads.insert(key.clone(), cancel.clone());
        }
        let emit = self.emit.clone();
        let total = v.onnx_size + v.json_size;
        let voice = v.id.clone();
        let r = self
            .voices
            .download(client, &self.voices_url, v, &cancel, |done| {
                emit(VoiceEvent::SpeechDownload {
                    voice: voice.clone(),
                    done,
                    total,
                })
            })
            .await;
        self.downloads.lock().remove(&key);
        match r {
            Ok(()) => Ok(()),
            Err(e) if e.is::<models::ChecksumMismatch>() => Err(anyhow::anyhow!(tr!(
                "Le fichier téléchargé n’est pas la bonne voix (somme de contrôle différente) : il n’est pas gardé. Réessaie ; derrière un proxy qui modifie les fichiers, télécharge-la ailleurs.",
                "The file downloaded is not the right voice (checksum mismatch): it was not kept. Try again; behind a proxy that alters files, download it elsewhere."
            ))),
            Err(e) => Err(e),
        }
    }

    pub fn cancel_voice_download(&self, id: &str) {
        if let Some(flag) = self.downloads.lock().get(&format!("voice:{id}")) {
            flag.store(true, Ordering::Release);
        }
    }

    pub fn delete_voice(&self, id: &str) -> anyhow::Result<()> {
        if speech::catalog::find(id).is_none() {
            anyhow::bail!("unknown voice {id}");
        }
        self.cancel_voice_download(id);
        self.preview.lock().take();
        self.engine.unload();
        self.voices.delete(id)
    }

    /// « Écouter »: `text` read in the voice `id`, as the settings would have it. Blocks until it is heard (call it
    /// off the async runtime); a new one stops the one before.
    pub fn preview(
        &self,
        id: &str,
        text: &str,
        opts: &speech::SpeakOptions,
        volume: f32,
    ) -> anyhow::Result<()> {
        let Some(v) = speech::catalog::find(id) else {
            anyhow::bail!("unknown voice {id}");
        };
        if !self.voices.has(v) {
            anyhow::bail!(tr!(
                "Cette voix n’est pas téléchargée.",
                "This voice is not downloaded."
            ));
        }
        let model = self.voices.model(&v.id);
        self.play_preview(volume, |push| {
            for sentence in self.engine.sentences(text) {
                let audio = self.engine.synthesize(&model, &sentence, opts)?;
                push(audio).map_err(|e| anyhow::anyhow!(e))?;
            }
            Ok(())
        })
    }

    /// « ▶ Écouter » on line (ESC-132): `text` read by the service of `spec` as `choice` says, its sound heard as it
    /// comes. Blocks until it is heard; a voice the service refuses is told at once, with what it answered.
    pub fn preview_online(
        &self,
        spec: &online::Spec,
        choice: &online_speech::SpeakChoice,
        text: &str,
        volume: f32,
    ) -> Result<(), online::OnlineError> {
        let p = spec.provider;
        let b = online_speech::Blocking::new()
            .map_err(|e| online::OnlineError::Network(p, e.to_string()))?;
        let never = AtomicBool::new(false);
        self.play_preview(volume, |push| {
            for sentence in online_speech::sentences(text, service::of(p).max_chars()) {
                let mut failed = None;
                b.speak(spec, choice, &sentence, &never, &mut |audio| {
                    if failed.is_none() {
                        failed = push(audio).err();
                    }
                })?;
                if let Some(e) = failed {
                    return Err(online::OnlineError::Network(p, e));
                }
            }
            Ok(())
        })
    }

    /// Plays what `make` pushes (a new preview stops the one before), and waits until it is heard, or stopped.
    fn play_preview<E>(
        &self,
        volume: f32,
        make: impl FnOnce(&mut dyn FnMut(speech::Audio) -> Result<(), String>) -> Result<(), E>,
    ) -> Result<(), E> {
        self.preview.lock().take();
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let mut playing: Option<(Arc<dyn speech::output::Playing>, u32)> = None;
        make(&mut |audio: speech::Audio| {
            if playing.is_none() {
                let tx = tx.clone();
                let p = self.output.open(
                    audio.rate,
                    volume,
                    Box::new(move || {
                        let _ = tx.send(());
                    }),
                )?;
                *self.preview.lock() = Some(p.clone());
                playing = Some((p, audio.rate));
            }
            let (p, rate) = playing.as_ref().unwrap();
            if audio.rate == *rate {
                p.push(&audio.samples);
            } else {
                p.push(&audio::resample(&audio.samples, audio.rate, *rate));
            }
            Ok(())
        })?;
        drop(tx);
        let Some((p, _)) = playing else { return Ok(()) };
        p.finish();
        // Until heard, or stopped (another preview, the window gone): the sender goes with the playback.
        let _ = rx.recv_timeout(Duration::from_secs(120));
        let mut slot = self.preview.lock();
        if slot.as_ref().is_some_and(|x| Arc::ptr_eq(x, &p)) {
            slot.take();
        }
        Ok(())
    }
}
