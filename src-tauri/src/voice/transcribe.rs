//! What turns the sound into text: whisper.cpp on the machine, or what a test puts in its place.

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Takes the sound (mono, 16 kHz) to its text. Called off the main thread, it may take seconds.
pub trait Transcriber: Send + Sync {
    /// `model`: the file of the model; `language`: whisper's code ("fr", "en"…), or "auto" for it
    /// to find out. Gives up early once `cancel` is set.
    fn transcribe(
        &self,
        model: &Path,
        samples: &[f32],
        language: &str,
        cancel: &Arc<AtomicBool>,
    ) -> Result<String>;

    /// Loads `model` ahead (while the user speaks), if it is not already.
    fn preload(&self, _model: &Path) {}

    /// Forgets the model loaded, if it is `model` (deleted).
    fn unload(&self, _model: &Path) {}
}

/// whisper.cpp, its model kept loaded from one dictation to the next.
#[derive(Default)]
pub struct Whisper {
    loaded: Mutex<Option<(PathBuf, Arc<whisper_rs::WhisperContext>)>>,
}

impl Whisper {
    fn context(&self, model: &Path) -> Result<Arc<whisper_rs::WhisperContext>> {
        let mut loaded = self.loaded.lock();
        if let Some((p, ctx)) = &*loaded {
            if p == model {
                return Ok(ctx.clone());
            }
        }
        // The other one first: two models at once could take 3 GB.
        *loaded = None;
        let started = std::time::Instant::now();
        let path = model
            .to_str()
            .ok_or_else(|| anyhow!("model path not UTF-8"))?;
        let ctx = whisper_rs::WhisperContext::new_with_params(
            path,
            // Flash attention, whisper.cpp's default that whisper-rs turns off: an encoder ~10 % faster on the CPU.
            {
                let mut p = whisper_rs::WhisperContextParameters::default();
                p.flash_attn(true);
                p
            },
        )
        .map_err(|e| anyhow!("model not loaded: {e}"))?;
        log::info!(
            "voice: {} loaded in {} ms",
            model.display(),
            started.elapsed().as_millis()
        );
        let ctx = Arc::new(ctx);
        *loaded = Some((model.to_path_buf(), ctx.clone()));
        Ok(ctx)
    }
}

/// What whisper writes for what is no speech ("[BLANK_AUDIO]", "(musique)", "[Applause]"), taken
/// out, and the segments joined into one line.
pub fn clean(segments: &[String]) -> String {
    let mut out = String::new();
    for s in segments {
        let mut kept = String::new();
        let mut depth = 0usize;
        for c in s.chars() {
            match c {
                '[' | '(' => depth += 1,
                ']' | ')' if depth > 0 => depth -= 1,
                _ if depth == 0 => kept.push(c),
                _ => {}
            }
        }
        // Keeps what was in brackets when the whole segment is not one (words said "(like this)"
        // are rare in a dictation, the markers are not): only a segment that is nothing but
        // markers goes.
        let text = if kept.trim().is_empty() {
            String::new()
        } else if s.trim_start().starts_with(['[', '(']) && s.trim_end().ends_with([']', ')']) {
            kept
        } else {
            s.clone()
        };
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&text);
    }
    out
}

/// The encoder's window, in its frames (50 a second), for `samples` of sound: as long as them and a second more,
/// rather than the 30 s it is trained on, which it pads a short sentence to and encodes the whole of (ESC-135: a
/// sentence 4 to 6 times faster, its text the same in the bench). 0, whisper's own 30 s, from 29 s of sound on.
fn audio_ctx(samples: usize) -> i32 {
    let frames = samples.div_ceil(16_000 / 50) + 50;
    if frames >= 1500 {
        0
    } else {
        frames as i32
    }
}

/// The threads whisper.cpp computes on.
fn threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 8)
}

impl Transcriber for Whisper {
    fn transcribe(
        &self,
        model: &Path,
        samples: &[f32],
        language: &str,
        cancel: &Arc<AtomicBool>,
    ) -> Result<String> {
        use whisper_rs::{FullParams, SamplingStrategy};
        let ctx = self.context(model)?;
        let mut state = ctx
            .create_state()
            .map_err(|e| anyhow!("whisper state: {e}"))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language));
        let threads = threads();
        params.set_n_threads(threads as i32);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        // Whisper wants a second at least: a shorter one is padded with silence.
        let padded;
        let samples = if samples.len() < 16_000 {
            padded = [samples, &vec![0f32; 16_000 - samples.len()][..]].concat();
            &padded[..]
        } else {
            samples
        };
        params.set_audio_ctx(audio_ctx(samples.len()));
        // The raw callback: whisper-rs 0.16's `set_abort_callback_safe` hands its closure a
        // pointer of another type, which reads garbage and stops every encoding at once.
        unsafe extern "C" fn aborted(data: *mut std::ffi::c_void) -> bool {
            // SAFETY: `data` is the `AtomicBool` of `cancel`, alive until `full` returns.
            unsafe { (*(data as *const AtomicBool)).load(Ordering::Acquire) }
        }
        // SAFETY: the pointer is to `cancel`, borrowed for the whole call of `full` below.
        unsafe {
            params.set_abort_callback(Some(aborted));
            params.set_abort_callback_user_data(Arc::as_ptr(cancel) as *mut std::ffi::c_void);
        }
        let started = std::time::Instant::now();
        state
            .full(params, samples)
            .map_err(|e| anyhow!("transcription failed: {e}"))?;
        if cancel.load(Ordering::Acquire) {
            return Err(anyhow!("cancelled"));
        }
        let segments: Vec<String> = state
            .as_iter()
            .map(|s| s.to_str_lossy().map(|t| t.into_owned()).unwrap_or_default())
            .collect();
        log::info!(
            "voice: {:.1} s of sound transcribed in {} ms ({} threads)",
            samples.len() as f32 / 16_000.0,
            started.elapsed().as_millis(),
            threads
        );
        Ok(clean(&segments))
    }

    fn preload(&self, model: &Path) {
        if let Err(e) = self.context(model) {
            log::warn!("voice: {e:#}");
        }
    }

    fn unload(&self, model: &Path) {
        let mut loaded = self.loaded.lock();
        if loaded.as_ref().is_some_and(|(p, _)| p == model) {
            *loaded = None;
        }
    }
}

/// A transcriber that says what it was given: the end-to-end tests (`ESCOUADE_VOICE_TRANSCRIPT`,
/// debug builds only), and the unit tests, which have no model.
pub struct Fake {
    pub text: String,
    /// What it was asked, for the tests: the number of samples and the language of each call.
    pub calls: Mutex<Vec<(usize, String)>>,
    /// The model of each call (empty: the service on line), and the models loaded ahead.
    pub models: Mutex<Vec<PathBuf>>,
    pub preloaded: Mutex<Vec<PathBuf>>,
    /// How long it takes.
    pub delay: std::time::Duration,
}

impl Fake {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            calls: Mutex::new(Vec::new()),
            models: Mutex::new(Vec::new()),
            preloaded: Mutex::new(Vec::new()),
            delay: std::time::Duration::ZERO,
        }
    }
}

impl Transcriber for Fake {
    fn transcribe(
        &self,
        model: &Path,
        samples: &[f32],
        language: &str,
        cancel: &Arc<AtomicBool>,
    ) -> Result<String> {
        // No model on line (ESC-110).
        if !model.as_os_str().is_empty() && !model.exists() {
            return Err(anyhow!("no model at {}", model.display()));
        }
        self.models.lock().push(model.to_path_buf());
        self.calls
            .lock()
            .push((samples.len(), language.to_string()));
        let end = std::time::Instant::now() + self.delay;
        while std::time::Instant::now() < end {
            if cancel.load(Ordering::Acquire) {
                return Err(anyhow!("cancelled"));
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // « a|b|c »: one text after the other, the last one again after them (a conversation's sentences).
        let parts: Vec<&str> = self.text.split('|').collect();
        let n = self.calls.lock().len();
        Ok(parts[(n - 1).min(parts.len() - 1)].to_string())
    }

    fn preload(&self, model: &Path) {
        self.preloaded.lock().push(model.to_path_buf());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn the_encoder_window_is_the_sound_and_a_second_more_up_to_whispers_own() {
        assert_eq!(audio_ctx(16_000), 100);
        assert_eq!(audio_ctx(8 * 16_000), 450);
        // Rounded up: the last samples are in it.
        assert_eq!(audio_ctx(16_001), 101);
        assert_eq!(audio_ctx(28 * 16_000), 1450);
        assert_eq!(audio_ctx(29 * 16_000), 0);
        assert_eq!(audio_ctx(60 * 16_000), 0);
    }

    #[test]
    fn the_segments_join_into_one_line_without_the_markers_of_no_speech() {
        assert_eq!(
            clean(&v(&[" Bonjour,", "  ajoute un test ", "[BLANK_AUDIO]"])),
            "Bonjour, ajoute un test"
        );
        assert_eq!(clean(&v(&["(musique)", " [Applause] "])), "");
        assert_eq!(clean(&v(&[])), "");
        // Words in brackets within a sentence are the user's.
        assert_eq!(
            clean(&v(&[" Lance les tests (tous) puis commite."])),
            "Lance les tests (tous) puis commite."
        );
    }

    /// The bench of ESC-135: `testdata/question-fr.wav` transcribed by each model, the encoder (one window of 30 s)
    /// timed alone, then the whole of what the app does. In release, whisper.cpp built as it ships:
    /// `cargo test --release --lib whisper_bench -- --ignored --nocapture`. The models: `ESCOUADE_VOICE_BENCH_MODELS`
    /// (paths, separated by `;`), by default base and large-v3-turbo where the app downloads them; the encoder's
    /// threads: `ESCOUADE_VOICE_BENCH_THREADS` (`4;8;16`…, by default the app's).
    #[test]
    #[ignore]
    fn whisper_bench() {
        let models: Vec<PathBuf> = match std::env::var("ESCOUADE_VOICE_BENCH_MODELS") {
            Ok(v) => v.split(';').map(PathBuf::from).collect(),
            Err(_) => ["ggml-base.bin", "ggml-large-v3-turbo.bin"]
                .iter()
                .map(|m| {
                    dirs::home_dir()
                        .unwrap()
                        .join(".escouade/voice/models")
                        .join(m)
                })
                .collect(),
        };
        let counts: Vec<usize> = std::env::var("ESCOUADE_VOICE_BENCH_THREADS")
            .map(|v| v.split(';').map(|t| t.parse().unwrap()).collect())
            .unwrap_or_else(|_| vec![threads()]);
        let wav = std::env::var("ESCOUADE_VOICE_BENCH_WAV")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("src/voice/testdata/question-fr.wav")
            });
        let (samples, rate) = crate::voice::source::read_wav(&wav).unwrap();
        let samples = crate::voice::audio::resample(&samples, rate, 16_000);
        println!("whisper.cpp: {}", whisper_rs::print_system_info());
        // Three runs after a first one (the model's pages read from the disk), their median.
        let timed = |f: &mut dyn FnMut()| {
            let mut runs: Vec<u128> = (0..4)
                .map(|_| {
                    let started = std::time::Instant::now();
                    f();
                    started.elapsed().as_millis()
                })
                .skip(1)
                .collect();
            runs.sort();
            (runs[1], runs)
        };
        let language = std::env::var("ESCOUADE_VOICE_BENCH_LANGUAGE").unwrap_or("fr".into());
        for model in &models {
            assert!(model.exists(), "no model at {}", model.display());
            let whisper = Whisper::default();
            let ctx = whisper.context(model).unwrap();
            let name = model.file_name().unwrap().to_string_lossy();
            for &n in &counts {
                let mut state = ctx.create_state().unwrap();
                let (ms, runs) = timed(&mut || {
                    state.pcm_to_mel(&samples, n).unwrap();
                    state.encode(0, n).unwrap();
                });
                println!("{name}: encoder in {ms} ms ({n} threads, {runs:?})");
            }
            let cancel = Arc::new(AtomicBool::new(false));
            let mut text = String::new();
            let (ms, runs) = timed(&mut || {
                text = whisper
                    .transcribe(model, &samples, &language, &cancel)
                    .unwrap();
            });
            println!(
                "{name}: {:.1} s of sound transcribed in {ms} ms ({} threads, {runs:?}): {text}",
                samples.len() as f32 / 16_000.0,
                threads()
            );
        }
    }
}
