//! The conversation (ESC-97): the microphone stays open from « Conversation » to « Arrêter ». While it listens, a
//! sentence is found by its silence (`vad::Endpointer`), transcribed by Whisper (« Transcription : En ligne », by the
//! service of the dictation, ESC-134), and its text told to the window, which
//! sends it to the agent. While the agent's passage is read aloud (`speak`), what the microphone hears is never
//! transcribed: it only tells whether the user speaks over the agent (`vad::BargeIn`, which knows the echo of its
//! voice), which stops the reading and listens. Once the reading is over, it listens again, after the echo's tail.
//!
//! With « Transcription en temps réel » (ESC-136), a sentence's sound goes to the service while it is said, from its
//! start (its first syllables, kept from before) to its end, its pauses and the silence that ends it included, as the
//! request sends it (ESC-137): its text is written in the window as it comes (`ConvPartial`), committed at the end
//! only, and final a few hundred milliseconds after it. Nothing else goes: neither the room before it, nor what is
//! heard while the agent is read or the conversation holds. A stream that fails sends the sentence whole, by
//! request, as before, and so do the sentences after it.
//!
//! The window leads (`conversation.svelte.ts`): it says when to listen, to hold (the agent works), and what to read.

use super::live::{self, LiveStream};
use super::online::{Online, OnlineError, Spec};
use super::online_speech::SpeakChoice;
use super::speech::output::Playing;
use super::speech::{Audio, SpeakOptions};
use super::transcribe::Transcriber;
use super::vad::{self, BargeIn, Endpointer, Heard};
use super::{audio, Voice, VoiceError, VoiceEvent};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// What a conversation is asked to use, from the settings.
#[derive(Debug, Clone)]
pub struct ConvChoice {
    pub device: String,
    /// Whisper's model and the language it is told (the service's too, on line).
    pub model: String,
    pub language: String,
    /// « Transcription : En ligne » (ESC-134): the service of the dictation hears each sentence, in place of Whisper,
    /// whose model is then neither asked for nor loaded.
    pub transcription: Option<Spec>,
    /// It cannot be asked (no key): the conversation does not start, as the dictation does not.
    pub transcription_error: Option<VoiceError>,
    /// The voice that reads (`speech::catalog`), its speaker, pace and volume.
    pub voice: String,
    pub speak: SpeakOptions,
    pub volume: f32,
    /// The silence that ends a sentence, in milliseconds.
    pub silence_ms: u32,
    /// « Voix générée par » a service on line (ESC-132): what reads in place of Piper.
    pub online: Option<OnlineVoice>,
    /// The service chosen could not be asked (no key): Piper reads, or the conversation does not start.
    pub online_error: Option<OnlineError>,
    /// What to say of it as it starts (a model the service no longer offers).
    pub online_notice: Option<String>,
}

/// The voice of a service on line: the service with its key, how it reads, and its default voice (taken when the one
/// chosen is refused).
#[derive(Debug, Clone)]
pub struct OnlineVoice {
    pub spec: Spec,
    pub choice: SpeakChoice,
    pub default_voice: String,
}

impl PartialEq for OnlineVoice {
    fn eq(&self, o: &Self) -> bool {
        self.spec.provider == o.spec.provider
            && self.spec.base == o.spec.base
            && self.spec.key == o.spec.key
            && self.choice == o.choice
            && self.default_voice == o.default_voice
    }
}

/// What `conversation_start` answers.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConvStarted {
    Listening {
        session: u64,
        /// What the conversation says as it starts (the service on line cannot read: Piper does).
        notice: Option<String>,
    },
    /// Whisper's model must be downloaded first.
    NeedsModel { model: String, size: u64 },
    /// The voice must be downloaded first.
    NeedsVoice { voice: String, size: u64 },
    /// No speech on this machine (Intel Macs).
    Unavailable,
}

/// What the conversation does with what the microphone hears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Waits for a sentence, and takes it.
    Listening,
    /// Hears nothing: the agent works, a sentence is being transcribed.
    Holding,
    /// Reads aloud: only listens for the user speaking over it.
    Speaking,
    /// The reading is over: listens again once the echo's tail has passed.
    After(Instant),
}

/// The echo of the reading's last words, which the microphone still hears.
const TAIL: Duration = Duration::from_millis(300);
/// What is kept from before a sentence's start: its first syllables.
const PREROLL_FRAMES: usize = 50;
/// The frames kept from before the speech that started a sentence (`Heard::Started`'s `frames_ago`): its first
/// syllables, said low, under what the endpointer hears as speech (ESC-137).
const LEAD_FRAMES: usize = 15;

/// What the threads of a conversation share.
struct Shared {
    id: u64,
    mode: Mutex<Mode>,
    endpointer: Mutex<Endpointer>,
    barge: Mutex<Option<BargeIn>>,
    /// The reading under way, and the one it is (`speak`'s count).
    playing: Mutex<Option<Arc<dyn Playing>>>,
    reading: AtomicU64,
    /// Set, the reading under way stops making sentences.
    hush: Mutex<Arc<AtomicBool>>,
    /// Samples heard, not yet cut into frames.
    pending: Mutex<Vec<f32>>,
    done: AtomicBool,
    /// A stream on line failed: the sentences go whole, by request, for the rest of the conversation (ESC-136).
    live_off: AtomicBool,
    choice: ConvChoice,
    model: PathBuf,
    /// How it reads and hears, which the settings may change while it goes on (`conversation_tune`).
    reader: Mutex<Reader>,
}

/// The voice that reads: Piper's (its model, when downloaded) or a service's on line; its speaker and pace, its volume,
/// and how what it hears is transcribed: by Whisper or the service on line (`ear`), in what language.
#[derive(Debug, Clone, PartialEq)]
struct Reader {
    voice: Option<PathBuf>,
    online: Option<OnlineVoice>,
    speak: SpeakOptions,
    volume: f32,
    language: String,
    ear: Option<Spec>,
}

pub(super) struct Conversation {
    shared: Arc<Shared>,
    capture: Mutex<Option<Box<dyn super::source::Capture>>>,
}

impl Voice {
    #[cfg(test)]
    pub fn conversing(&self) -> Option<u64> {
        self.conv.lock().as_ref().map(|c| c.shared.id)
    }

    /// Starts a conversation: the microphone opens and listens for a sentence; or says what is missing first.
    pub fn conversation_start(&self, mut choice: ConvChoice) -> Result<ConvStarted, VoiceError> {
        // The service of the transcription cannot be asked: said before anything is listened to, as the dictation.
        if let Some(e) = choice.transcription_error.take() {
            return Err(e);
        }
        let on_line = choice.online.is_some() || choice.online_error.is_some();
        if !on_line && !self.engine.available() {
            return Ok(ConvStarted::Unavailable);
        }
        let model = super::models::find(&choice.model)
            .or_else(|| super::models::find(super::models::DEFAULT_MODEL))
            .expect("the default model is in the catalog");
        // On line, the service hears: no model of Whisper's.
        if choice.transcription.is_none() && !self.store.has(model) {
            return Ok(ConvStarted::NeedsModel {
                model: model.id.into(),
                size: model.size,
            });
        }
        let local = self.local_voice(&choice.voice);
        let mut notice = choice.online_notice.take();
        match choice.online_error.take() {
            // On line, Piper's voice is only what reads if the service fails.
            _ if choice.online.is_some() => {}
            // The service cannot be asked: Piper reads, if its voice is here, and it is said once.
            Some(e) => {
                if local.is_none() {
                    return Err(VoiceError::Online(e));
                }
                let said = fallback_message(&e);
                log::warn!("speech: the voice on line cannot read: Piper reads");
                notice = Some(match notice {
                    Some(n) => format!("{n} {said}"),
                    None => said,
                });
            }
            None => {
                let Some(voice) = super::speech::catalog::find(&choice.voice) else {
                    return Err(VoiceError::Other(format!("unknown voice {}", choice.voice)));
                };
                if !self.voices.has(voice) {
                    return Ok(ConvStarted::NeedsVoice {
                        voice: voice.id.clone(),
                        size: voice.onnx_size + voice.json_size,
                    });
                }
            }
        }
        if self.session.lock().is_some() || self.conv.lock().is_some() {
            return Err(VoiceError::AlreadyListening);
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let shared = Arc::new(Shared {
            id,
            mode: Mutex::new(Mode::Listening),
            endpointer: Mutex::new(Endpointer::new(choice.silence_ms.clamp(400, 5000))),
            barge: Mutex::new(None),
            playing: Mutex::new(None),
            reading: AtomicU64::new(0),
            hush: Mutex::new(Arc::new(AtomicBool::new(false))),
            pending: Mutex::new(Vec::new()),
            done: AtomicBool::new(false),
            live_off: AtomicBool::new(false),
            model: self.store.path(model.id),
            reader: Mutex::new(Reader {
                voice: local,
                online: choice.online.clone(),
                speak: choice.speak.clone(),
                volume: choice.volume,
                language: choice.language.clone(),
                ear: choice.transcription.clone(),
            }),
            choice,
        });
        let failure = Arc::new(Mutex::new(None::<VoiceError>));
        let sink = {
            let shared = shared.clone();
            let failure = failure.clone();
            super::source::Sink {
                samples: Box::new(move |chunk: &[f32]| {
                    shared.pending.lock().extend_from_slice(chunk)
                }),
                error: Box::new(move |e| {
                    failure.lock().get_or_insert(e);
                }),
            }
        };
        let capture = self.source.open(&shared.choice.device, sink)?;
        let rate = capture.rate();
        log::info!("voice: conversation listening at {rate} Hz");
        match &shared.choice.transcription {
            Some(spec) => log::info!(
                "voice: the conversation is transcribed by {} ({}{})",
                spec.provider.label(),
                spec.live.as_deref().unwrap_or(&spec.model),
                if spec.live.is_some() { ", live" } else { "" }
            ),
            None => {
                let t = self.transcriber.clone();
                let p = shared.model.clone();
                std::thread::spawn(move || t.preload(&p));
            }
        }
        // The voice too (and ONNX Runtime): the first sentence read does not wait for it (ESC-115). On line, nothing
        // to load.
        let reader = shared.reader.lock().clone();
        if let (Some(v), None) = (reader.voice, &reader.online) {
            self.preload_voice(v);
        }
        if let Some(o) = &reader.online {
            log::info!(
                "speech: the conversation reads with {} ({})",
                o.spec.provider.label(),
                o.choice.model
            );
        }
        *self.conv.lock() = Some(Conversation {
            shared: shared.clone(),
            capture: Mutex::new(Some(capture)),
        });
        let emit = self.emit.clone();
        let ears = Ears {
            whisper: self.transcriber.clone(),
            fake_online: self.fake_online,
        };
        std::thread::Builder::new()
            .name("voice-conversation".into())
            .spawn(move || listen(shared, rate, failure, emit, ears))
            .map_err(|e| VoiceError::Other(e.to_string()))?;
        Ok(ConvStarted::Listening {
            session: id,
            notice,
        })
    }

    /// The model of Piper's voice `id`, when it is downloaded and the app can speak here; none else.
    fn local_voice(&self, id: &str) -> Option<PathBuf> {
        let v = super::speech::catalog::find(id)?;
        (self.engine.available() && self.voices.has(v)).then(|| self.voices.model(&v.id))
    }

    /// Loads `model` on a thread of its own, ahead of its first sentence. Failing, it is only written down: the first
    /// sentence loads it then, as it did before.
    fn preload_voice(&self, model: PathBuf) {
        let engine = self.engine.clone();
        let spawned = std::thread::Builder::new()
            .name("voice-preload".into())
            .spawn(move || {
                if let Err(e) = engine.preload(&model) {
                    log::warn!(
                        "speech: voice not loaded ahead ({e:#}): it will be at its first sentence"
                    );
                }
            });
        if let Err(e) = spawned {
            log::warn!("speech: voice not loaded ahead ({e})");
        }
    }

    /// The settings changed while a conversation goes on: its next readings take the voice, the pace and the volume
    /// they now say, and what it hears is transcribed in their language; a new voice is loaded at once, ahead of its
    /// first sentence. A voice not downloaded is not taken (the next conversation offers it).
    pub fn conversation_tune(&self, choice: &ConvChoice) {
        let Some(s) = self.conv.lock().as_ref().map(|c| c.shared.clone()) else {
            return;
        };
        let local = self.local_voice(&choice.voice);
        if local.is_none() && choice.online.is_none() {
            log::info!(
                "speech: {} not downloaded, the conversation keeps its voice",
                choice.voice
            );
            return;
        }
        let mut reader = s.reader.lock();
        let ear = match (&choice.transcription, &choice.transcription_error) {
            (Some(t), _) => Some(t.clone()),
            // The service can no longer be asked (its key gone): it hears as it did.
            (None, Some(_)) => reader.ear.clone(),
            (None, None) => None,
        };
        let tuned = Reader {
            voice: local,
            online: choice.online.clone(),
            speak: choice.speak.clone(),
            volume: choice.volume,
            language: choice.language.clone(),
            ear,
        };
        if *reader == tuned {
            return;
        }
        let new_voice = reader.voice != tuned.voice || reader.online != tuned.online;
        let new_ear = reader.ear != tuned.ear;
        *reader = tuned;
        if new_ear {
            match &reader.ear {
                Some(spec) => log::info!(
                    "voice: the conversation is now transcribed by {} ({})",
                    spec.provider.label(),
                    spec.model
                ),
                None => {
                    log::info!("voice: the conversation is now transcribed by Whisper");
                    let t = self.transcriber.clone();
                    let p = s.model.clone();
                    std::thread::spawn(move || t.preload(&p));
                }
            }
        }
        if new_voice {
            match (&reader.online, &reader.voice) {
                (Some(o), _) => log::info!(
                    "speech: the conversation now reads with {} ({})",
                    o.spec.provider.label(),
                    o.choice.model
                ),
                (None, Some(v)) => {
                    log::info!("speech: the conversation now reads in {}", choice.voice);
                    self.preload_voice(v.clone());
                }
                (None, None) => {}
            }
        }
    }

    fn conv_shared(&self, session: u64) -> Option<Arc<Shared>> {
        self.conv
            .lock()
            .as_ref()
            .filter(|c| c.shared.id == session)
            .map(|c| c.shared.clone())
    }

    /// Listens for the next sentence (the agent's turn is over, or nothing was read).
    pub fn conversation_listen(&self, session: u64) {
        if let Some(s) = self.conv_shared(session) {
            hush(&s);
            *s.mode.lock() = Mode::Listening;
        }
    }

    /// Hears nothing until told to listen or to speak: the agent works. A reading under way goes on (a hold the
    /// window sent before it started may arrive after): it ends as it would.
    pub fn conversation_hold(&self, session: u64) {
        if let Some(s) = self.conv_shared(session) {
            let mut mode = s.mode.lock();
            if *mode != Mode::Speaking {
                *mode = Mode::Holding;
            }
        }
    }

    /// Reads `text` aloud in the conversation's voice; once read, listens again (`Spoken`), or listens at once when
    /// the user speaks over it (`BargeIn`). Returns the reading's number, which those events carry.
    pub fn conversation_say(&self, session: u64, text: &str) -> Result<u64, VoiceError> {
        let s = self
            .conv_shared(session)
            .ok_or_else(|| VoiceError::Other("no conversation under way".into()))?;
        hush(&s);
        let reading = s.reading.fetch_add(1, Ordering::AcqRel) + 1;
        let stop = Arc::new(AtomicBool::new(false));
        *s.hush.lock() = stop.clone();
        *s.barge.lock() = Some(BargeIn::new(s.endpointer.lock().noise()));
        *s.mode.lock() = Mode::Speaking;
        let engine = self.engine.clone();
        let output = self.output.clone();
        let emit = self.emit.clone();
        let reader = s.reader.lock().clone();
        let sentences = match &reader.online {
            Some(o) => super::online_speech::sentences(
                text,
                super::service::of(o.spec.provider).max_chars(),
            ),
            None => engine.sentences(text),
        };
        std::thread::Builder::new()
            .name("voice-speak".into())
            .spawn(move || {
                let mut feed = Feed {
                    s: s.clone(),
                    output,
                    emit: emit.clone(),
                    stop: stop.clone(),
                    reading,
                    volume: reader.volume,
                    rate: None,
                    heard: None,
                };
                let mut online = reader.online.clone();
                let mut blocking: Option<super::online_speech::Blocking> = None;
                let mut error = None;
                let mut paused = false;
                'sentences: for (index, sentence) in sentences.into_iter().enumerate() {
                    if stop.load(Ordering::Acquire) {
                        return;
                    }
                    // Its subtitle, when it starts to be heard.
                    feed.heard = Some({
                        let s = s.clone();
                        let emit = emit.clone();
                        let stop = stop.clone();
                        let text = sentence.clone();
                        Box::new(move || {
                            if !stop.load(Ordering::Acquire) {
                                emit(VoiceEvent::ConvSentence {
                                    session: s.id,
                                    reading,
                                    index: index as u32,
                                    text,
                                });
                            }
                        })
                    });
                    while let Some(o) = online.clone() {
                        if blocking.is_none() {
                            match super::online_speech::Blocking::new() {
                                Ok(b) => blocking = Some(b),
                                Err(e) => {
                                    error = Some(e.to_string());
                                    break 'sentences;
                                }
                            }
                        }
                        let b = blocking.as_ref().expect("made above");
                        let mut failed_output = None;
                        let r = b.speak(&o.spec, &o.choice, &sentence, &stop, &mut |audio| {
                            if failed_output.is_none() {
                                if let Err(e) = feed.push(audio) {
                                    failed_output = Some(e);
                                }
                            }
                        });
                        if let Some(e) = failed_output {
                            error = Some(e);
                            break 'sentences;
                        }
                        if stop.load(Ordering::Acquire) {
                            return;
                        }
                        let e = match r {
                            Ok(_) => continue 'sentences,
                            Err(e) => e,
                        };
                        let p = o.spec.provider.label();
                        // A voice the service does not know (any more): its default one, said once.
                        if super::online_speech::refuses_the_voice(&e)
                            && o.choice.voice != o.default_voice
                            && feed.heard.is_some()
                        {
                            let voice = o.choice.voice.clone();
                            let default = o.default_voice.clone();
                            let why = e.message(crate::i18n::kit(crate::i18n::ui()));
                            let message = tr!(
                                "{why} La voix « {voice} » n’est pas celle de {p} : sa voix par défaut, « {default} », la remplace.",
                                "{why} The voice “{voice}” is not one of {p}: its default voice, “{default}”, stands in."
                            );
                            let mut o = o.clone();
                            o.choice.voice = o.default_voice.clone();
                            s.reader.lock().online = Some(o.clone());
                            online = Some(o);
                            emit(VoiceEvent::ConvNotice {
                                session: s.id,
                                message,
                            });
                            continue;
                        }
                        let said = fallback_message(&e);
                        let local = reader.voice.is_some() && engine.available();
                        log::warn!(
                            "speech: {p} failed: {}",
                            if local {
                                "Piper reads"
                            } else {
                                "the conversation pauses"
                            }
                        );
                        if local {
                            // Piper reads from here on, for the rest of the conversation.
                            s.reader.lock().online = None;
                            online = None;
                            emit(VoiceEvent::ConvNotice {
                                session: s.id,
                                message: said,
                            });
                        } else {
                            error = Some(pause_message(&e));
                            paused = true;
                            break 'sentences;
                        }
                    }
                    let Some(voice) = &reader.voice else {
                        error = Some("no voice".into());
                        break;
                    };
                    let audio = match engine.synthesize(voice, &sentence, &reader.speak) {
                        Ok(a) => a,
                        Err(e) => {
                            error = Some(format!("{e:#}"));
                            break;
                        }
                    };
                    if stop.load(Ordering::Acquire) {
                        return;
                    }
                    if let Err(e) = feed.push(audio) {
                        error = Some(e);
                        break;
                    }
                }
                if stop.load(Ordering::Acquire) {
                    return;
                }
                log::info!("speech: reading {reading} made");
                let playing = s.playing.lock().clone();
                match (playing, error) {
                    (Some(p), None) => p.finish(),
                    (_, error) => {
                        // Nothing to hear (an empty text), or it failed: it listens again at once; or, the service on
                        // line failing without a voice of Piper's to stand in, it pauses.
                        if s.reading.load(Ordering::Acquire) == reading
                            && !stop.load(Ordering::Acquire)
                        {
                            if let Some(e) = &error {
                                log::warn!("speech: {e}");
                            }
                            s.playing.lock().take();
                            *s.mode.lock() = if paused {
                                Mode::Holding
                            } else {
                                Mode::Listening
                            };
                            emit(VoiceEvent::ConvSpoken {
                                session: s.id,
                                reading,
                                error,
                                paused,
                            });
                        }
                    }
                }
            })
            .map_err(|e| VoiceError::Other(e.to_string()))?;
        Ok(reading)
    }

    /// « Arrêter »: the reading and the listening stop, the microphone is let go.
    pub fn conversation_stop(&self) {
        if let Some(c) = self.conv.lock().take() {
            hush(&c.shared);
            c.shared.done.store(true, Ordering::Release);
            drop(c.capture.lock().take());
            log::info!("voice: conversation stopped");
        }
    }
}

/// What a reading's sentences are pushed through: the output opened with the first sound, at its rate (a sound of
/// another rate, Piper's standing in for the service's, brought to it), each sentence's subtitle marked where it starts.
struct Feed {
    s: Arc<Shared>,
    output: Arc<dyn super::speech::output::Output>,
    emit: super::Emit,
    stop: Arc<AtomicBool>,
    reading: u64,
    volume: f32,
    rate: Option<u32>,
    /// The subtitle of the sentence under way, until its first sound is pushed.
    heard: Option<super::speech::output::Done>,
}

impl Feed {
    /// Pushes `audio`; stopped meanwhile, pushes nothing. Fails when the output does not open.
    fn push(&mut self, audio: Audio) -> Result<(), String> {
        if self.stop.load(Ordering::Acquire) || audio.samples.is_empty() {
            return Ok(());
        }
        let mut slot = self.s.playing.lock();
        // Stopped while the lock was awaited: the next reading may hold the output by now.
        if self.stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if slot.is_none() {
            let done = {
                let s = self.s.clone();
                let emit = self.emit.clone();
                let reading = self.reading;
                Box::new(move || {
                    if s.reading.load(Ordering::Acquire) != reading {
                        return;
                    }
                    let mut mode = s.mode.lock();
                    if *mode == Mode::Speaking {
                        *mode = Mode::After(Instant::now());
                        drop(mode);
                        emit(VoiceEvent::ConvSpoken {
                            session: s.id,
                            reading,
                            error: None,
                            paused: false,
                        });
                    }
                })
            };
            *slot = Some(self.output.open(audio.rate, self.volume, done)?);
            self.rate = Some(audio.rate);
        }
        let rate = self.rate.unwrap_or(audio.rate);
        let samples = if audio.rate == rate {
            audio.samples
        } else {
            audio::resample(&audio.samples, audio.rate, rate)
        };
        let playing = slot.as_ref().expect("opened above");
        if let Some(heard) = self.heard.take() {
            playing.mark(heard);
        }
        playing.push(&samples);
        Ok(())
    }
}

/// What the conversation says when the service on line fails: why, and who reads now.
fn fallback_message(e: &OnlineError) -> String {
    let why = e.message(crate::i18n::kit(crate::i18n::ui()));
    tr!(
        "{why} La conversation continue avec la voix locale (Piper).",
        "{why} The conversation goes on with the local voice (Piper)."
    )
}

/// What it says when the service fails with no voice of Piper's to stand in: it pauses.
fn pause_message(e: &OnlineError) -> String {
    let why = e.message(crate::i18n::kit(crate::i18n::ui()));
    tr!(
        "{why} La conversation est en pause : corrige-le, ou télécharge une voix locale dans Réglages › Voix, puis reprends.",
        "{why} The conversation is paused: fix it, or download a local voice in Settings › Voice, then resume."
    )
}

/// The reading under way stops: no more sentences made, nothing more heard.
fn hush(s: &Shared) {
    s.hush.lock().store(true, Ordering::Release);
    s.playing.lock().take();
}

/// What transcribes a sentence: Whisper, or the service on line (the fake transcriber of the tests stands for it too).
struct Ears {
    whisper: Arc<dyn Transcriber>,
    fake_online: bool,
}

impl Ears {
    /// The transcriber of `s`'s sentences now, the model it is given (none on line), and its name for the log.
    fn of(&self, s: &Shared) -> (Arc<dyn Transcriber>, PathBuf, &'static str) {
        match s.reader.lock().ear.clone() {
            Some(spec) if !self.fake_online => {
                let label = spec.provider.label();
                (Arc::new(Online(spec)), PathBuf::new(), label)
            }
            Some(spec) => (self.whisper.clone(), PathBuf::new(), spec.provider.label()),
            None => (self.whisper.clone(), s.model.clone(), "Whisper"),
        }
    }

    /// The service that hears `s`'s sentences as they are said: « Transcription en temps réel » on, and no stream
    /// failed yet. The fake transcriber of the end-to-end tests hears them whole.
    fn live(&self, s: &Shared) -> Option<Spec> {
        if self.fake_online || s.live_off.load(Ordering::Acquire) {
            return None;
        }
        s.reader.lock().ear.clone().filter(|e| e.live.is_some())
    }

    /// The stream of a sentence that starts with `start`, its texts told to the window as they come.
    fn open(
        &self,
        s: &Arc<Shared>,
        emit: &super::Emit,
        rate: u32,
        start: &[f32],
    ) -> Option<LiveStream> {
        let spec = self.live(s)?;
        let language = s.reader.lock().language.clone();
        let (session, emit) = (s.id, emit.clone());
        let stream = super::service::of(spec.provider).open_live(
            &spec,
            &language,
            rate,
            Box::new(move |text| emit(VoiceEvent::ConvPartial { session, text })),
        );
        stream.push(start);
        Some(stream)
    }
}

/// The conversation's ear: frames of 20 ms, taken as its mode says.
fn listen(
    s: Arc<Shared>,
    rate: u32,
    failure: Arc<Mutex<Option<VoiceError>>>,
    emit: super::Emit,
    ears: Ears,
) {
    let ears = Arc::new(ears);
    let per = (rate * vad::FRAME_MS / 1000).max(1) as usize;
    let mut buf: Vec<f32> = Vec::new();
    let mut recent: VecDeque<Vec<f32>> = VecDeque::new();
    let mut sentence: Option<Vec<f32>> = None;
    // Its stream, with « Transcription en temps réel », and when the user last spoke in it.
    let mut flow: Option<LiveStream> = None;
    let mut last_word = Instant::now();
    let mut peak = 0f32;
    let mut told = Instant::now();
    while !s.done.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_millis(vad::FRAME_MS as u64));
        if let Some(e) = failure.lock().take() {
            emit(VoiceEvent::ConvEnded {
                session: s.id,
                error: Some(e.message(crate::i18n::ui(), std::env::consts::OS)),
            });
            s.done.store(true, Ordering::Release);
            hush(&s);
            return;
        }
        buf.extend(std::mem::take(&mut *s.pending.lock()));
        let whole = buf.len() / per * per;
        let frames: Vec<Vec<f32>> = buf
            .drain(..whole)
            .collect::<Vec<_>>()
            .chunks(per)
            .map(<[f32]>::to_vec)
            .collect();
        for frame in frames {
            let level = vad::rms(&frame);
            peak = peak.max(level);
            recent.push_back(frame.clone());
            if recent.len() > PREROLL_FRAMES {
                recent.pop_front();
            }
            let mode = *s.mode.lock();
            // Nothing goes on but while it listens: a stream opened before (a hold, a reading) closes, unsent.
            if mode != Mode::Listening {
                flow = None;
            }
            match mode {
                Mode::Holding => {}
                Mode::After(at) => {
                    if at.elapsed() >= TAIL {
                        let mut m = s.mode.lock();
                        if *m == mode {
                            *m = Mode::Listening;
                        }
                    }
                }
                Mode::Speaking => {
                    let played = s.playing.lock().as_ref().map(|p| p.level()).unwrap_or(0.0);
                    let over = s
                        .barge
                        .lock()
                        .as_mut()
                        .is_some_and(|b| b.push(level, played));
                    if over {
                        hush(&s);
                        *s.mode.lock() = Mode::Listening;
                        s.endpointer.lock().start_in_sentence();
                        // What the barge-in heard of the user is the sentence's start (with the echo's last bits).
                        let keep = recent.len().min(20);
                        let start: Vec<f32> = recent
                            .iter()
                            .skip(recent.len() - keep)
                            .flatten()
                            .copied()
                            .collect();
                        flow = ears.open(&s, &emit, rate, &start);
                        last_word = Instant::now();
                        sentence = Some(start);
                        emit(VoiceEvent::ConvBargeIn {
                            session: s.id,
                            reading: s.reading.load(Ordering::Acquire),
                        });
                    }
                }
                Mode::Listening => {
                    let (heard, quiet) = {
                        let mut e = s.endpointer.lock();
                        let heard = e.push(level);
                        (heard, e.quiet_ms())
                    };
                    if let Some(sent) = sentence.as_mut() {
                        sent.extend_from_slice(&frame);
                        if quiet == 0 {
                            last_word = Instant::now();
                        }
                        // All of it goes, its quiet too: what the endpointer takes for quiet may be words said
                        // low, the sentence's end or a soft resume (ESC-137). Committed at its end only.
                        if let Some(f) = flow.as_ref() {
                            f.push(&frame);
                        }
                    }
                    match heard {
                        Some(Heard::Started { frames_ago }) => {
                            let keep = (frames_ago + LEAD_FRAMES).min(recent.len());
                            let start: Vec<f32> = recent
                                .iter()
                                .skip(recent.len() - keep)
                                .flatten()
                                .copied()
                                .collect();
                            emit(VoiceEvent::ConvSpeech { session: s.id });
                            flow = ears.open(&s, &emit, rate, &start);
                            last_word = Instant::now();
                            sentence = Some(start);
                        }
                        Some(Heard::Ended) => {
                            let said = sentence.take().unwrap_or_default();
                            let stream = flow.take();
                            let spoke = last_word;
                            *s.mode.lock() = Mode::Holding;
                            emit(VoiceEvent::ConvTranscribing { session: s.id });
                            let s2 = s.clone();
                            let emit = emit.clone();
                            let ears = ears.clone();
                            std::thread::spawn(move || {
                                let ended = Instant::now();
                                let secs = said.len() as f32 / rate.max(1) as f32;
                                let (_, _, by) = ears.of(&s2);
                                // As it was said: its final text, there by now or soon.
                                let streamed = stream.and_then(|stream| match stream.finish(live::FINAL_WAIT) {
                                    Ok(f) => Some(f),
                                    Err(e) => {
                                        if !s2.live_off.swap(true, Ordering::AcqRel) {
                                            log::warn!(
                                                "voice: the live transcription by {by} failed ({e}): the sentences go whole, by request, from now on"
                                            );
                                        }
                                        None
                                    }
                                });
                                let text = match streamed {
                                    Some(f) => {
                                        let after =
                                            f.ready.saturating_duration_since(spoke).as_millis();
                                        let wait = ended.elapsed().as_millis();
                                        log::info!(
                                            "voice: a sentence of {secs:.1} s transcribed by {by}, live, ready {after} ms after the last word ({wait} ms after its silence)"
                                        );
                                        Ok(f.text)
                                    }
                                    None => {
                                        let pcm = audio::resample(&said, rate, audio::WHISPER_RATE);
                                        let never = Arc::new(AtomicBool::new(false));
                                        let language = s2.reader.lock().language.clone();
                                        let (t, model, by) = ears.of(&s2);
                                        let text = t.transcribe(&model, &pcm, &language, &never);
                                        let after = spoke.elapsed().as_millis();
                                        let ms = ended.elapsed().as_millis();
                                        match &text {
                                            Ok(_) => log::info!(
                                                "voice: a sentence of {secs:.1} s transcribed by {by}, by request, ready {after} ms after the last word ({ms} ms after its silence)"
                                            ),
                                            Err(e) => log::warn!(
                                                "voice: a sentence of {secs:.1} s not transcribed by {by} in {ms} ms: {e:#}"
                                            ),
                                        }
                                        text
                                    }
                                };
                                // Told as the dictation tells it: the service's failure, with what to do about it.
                                let text = text.map_err(|e| match e.downcast::<OnlineError>() {
                                    Ok(e) => VoiceError::Online(e)
                                        .message(crate::i18n::ui(), std::env::consts::OS),
                                    Err(e) => format!("{e:#}"),
                                });
                                if s2.done.load(Ordering::Acquire) {
                                    return;
                                }
                                let (text, error) = match text {
                                    Ok(t) => (t.trim().to_string(), None),
                                    Err(e) => (String::new(), Some(e)),
                                };
                                emit(VoiceEvent::ConvHeard {
                                    session: s2.id,
                                    text,
                                    error,
                                });
                            });
                        }
                        None => {}
                    }
                }
            }
        }
        if told.elapsed() >= Duration::from_millis(50) {
            told = Instant::now();
            let played = s.playing.lock().as_ref().map(|p| p.level()).unwrap_or(0.0);
            emit(VoiceEvent::ConvLevel {
                session: s.id,
                level: audio::level(&[peak]),
                played: audio::level(&[played]),
            });
            peak = 0.0;
        }
    }
}
