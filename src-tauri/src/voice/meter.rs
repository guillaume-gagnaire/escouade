//! « Tester » of Réglages › Voix (ESC-665): the microphone chosen, heard by the conversation's detector
//! (`vad::Endpointer`) at the sensitivity of the slider, moved while it listens. The window is told levels only, every
//! `TICK`: how loud it is, the room's noise learnt, the threshold, and whether a sentence is heard (from its start to
//! the silence that ends it). The sound itself is dropped frame by frame: nothing of it is kept, transcribed nor sent.
//!
//! One at a time; it stops by « Arrêter », when the settings close, when a dictation or a conversation starts (they
//! take the microphone), when the window is closed or reloaded (`cancel`), and after `MAX_LISTEN`.

use super::source::{Capture, Sink};
use super::vad::{self, Endpointer};
use super::{Voice, VoiceError, VoiceEvent, TICK};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// The tester under way.
pub(super) struct Meter {
    id: u64,
    done: Arc<AtomicBool>,
    sensitivity: Arc<AtomicI32>,
    /// Dropped, the microphone is let go.
    capture: Arc<Mutex<Option<Box<dyn Capture>>>>,
}

impl Meter {
    fn stop(&self) {
        self.done.store(true, Ordering::Release);
        drop(self.capture.lock().take());
    }
}

impl Voice {
    /// Opens `device` (empty: the system's) and tells its levels as the detector hears them at `sensitivity`, a
    /// sentence ending after `silence_ms`. A tester already under way stops first; a dictation or a conversation
    /// keeps the microphone (`AlreadyListening`).
    pub fn meter_start(
        &self,
        device: &str,
        sensitivity: i32,
        silence_ms: u32,
    ) -> Result<u64, VoiceError> {
        if self.session.lock().is_some()
            || self.conv.lock().is_some()
            || self.opening.load(Ordering::Acquire)
        {
            return Err(VoiceError::AlreadyListening);
        }
        self.meter_stop_any();
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let pending = Arc::new(Mutex::new(Vec::<f32>::new()));
        let failure = Arc::new(Mutex::new(None::<VoiceError>));
        let sink = {
            let pending = pending.clone();
            let failure = failure.clone();
            Sink {
                samples: Box::new(move |chunk: &[f32]| pending.lock().extend_from_slice(chunk)),
                error: Box::new(move |e| {
                    failure.lock().get_or_insert(e);
                }),
            }
        };
        let capture = self.source.open(device, sink)?;
        let rate = capture.rate();
        log::info!("voice: the tester listens at {rate} Hz");
        let done = Arc::new(AtomicBool::new(false));
        let level = Arc::new(AtomicI32::new(sensitivity));
        let capture = Arc::new(Mutex::new(Some(capture)));
        *self.meter.lock() = Some(Meter {
            id,
            done: done.clone(),
            sensitivity: level.clone(),
            capture: capture.clone(),
        });
        let emit = self.emit.clone();
        let max = self.max_listen;
        let silence = silence_ms.clamp(400, 5000);
        std::thread::Builder::new()
            .name("voice-meter".into())
            .spawn(move || {
                let started = Instant::now();
                let per = (rate * vad::FRAME_MS / 1000).max(1) as usize;
                let mut e =
                    Endpointer::new(silence).with_sensitivity(level.load(Ordering::Relaxed));
                let mut buf: Vec<f32> = Vec::new();
                while !done.load(Ordering::Acquire) {
                    std::thread::sleep(TICK);
                    if done.load(Ordering::Acquire) {
                        return;
                    }
                    let s = level.load(Ordering::Relaxed);
                    if s != e.sensitivity() {
                        e.set_sensitivity(s);
                    }
                    buf.extend(std::mem::take(&mut *pending.lock()));
                    let whole = buf.len() / per * per;
                    let mut loudest = None::<f32>;
                    for frame in buf.drain(..whole).collect::<Vec<_>>().chunks(per) {
                        let l = vad::rms(frame);
                        loudest = Some(loudest.map_or(l, |m| m.max(l)));
                        e.push(l);
                    }
                    // What was heard is gone: only its levels are told.
                    if let Some(level) = loudest {
                        emit(VoiceEvent::Meter {
                            session: id,
                            level,
                            noise: e.noise(),
                            threshold: e.threshold(),
                            speech: e.in_sentence(),
                        });
                    }
                    let failed = failure.lock().take();
                    if failed.is_some() || started.elapsed() >= max {
                        done.store(true, Ordering::Release);
                        drop(capture.lock().take());
                        emit(VoiceEvent::MeterEnded {
                            session: id,
                            error: failed
                                .map(|e| e.message(crate::i18n::ui(), std::env::consts::OS)),
                        });
                        return;
                    }
                }
            })
            .map_err(|e| VoiceError::Other(e.to_string()))?;
        Ok(id)
    }

    /// The slider moved: the tester `id` hears at `sensitivity` from its next frames, the noise learnt kept.
    pub fn meter_set(&self, id: u64, sensitivity: i32) {
        if let Some(m) = self.meter.lock().as_ref().filter(|m| m.id == id) {
            m.sensitivity.store(sensitivity, Ordering::Relaxed);
        }
    }

    /// « Arrêter », the page left: the tester `id` stops, if it is the one under way.
    pub fn meter_stop(&self, id: u64) {
        let mut slot = self.meter.lock();
        if slot.as_ref().is_some_and(|m| m.id == id) {
            if let Some(m) = slot.take() {
                m.stop();
                log::info!("voice: the tester stopped");
            }
        }
    }

    /// Whatever tester is under way stops (a dictation, a conversation, the window gone).
    pub(super) fn meter_stop_any(&self) {
        if let Some(m) = self.meter.lock().take() {
            m.stop();
        }
    }

    /// The tester under way.
    #[cfg(test)]
    pub fn metering(&self) -> Option<u64> {
        self.meter.lock().as_ref().map(|m| m.id)
    }
}
