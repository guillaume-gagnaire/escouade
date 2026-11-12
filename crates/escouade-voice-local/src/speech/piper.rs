//! Piper, in the app: the text cut into sentences and clauses, each clause made phonemes by eSpeak NG, the phonemes
//! made the ids of the voice's alphabet (its `.onnx.json`), and the voice's VITS model run by ONNX Runtime on them.
//! What piper1-gpl does (`phonemize_espeak`, `phonemes_to_ids`, `synthesize`), written again here so that nothing but
//! the app is needed: no program of its own, no Python.

use super::{Audio, Engine, SpeakOptions};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// What a voice's `.onnx.json` tells the app.
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceConfig {
    pub audio: AudioConfig,
    pub espeak: EspeakConfig,
    #[serde(default)]
    pub inference: Inference,
    #[serde(default)]
    pub num_speakers: u32,
    pub phoneme_id_map: HashMap<String, Vec<i64>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EspeakConfig {
    pub voice: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Inference {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
}

impl Default for Inference {
    fn default() -> Self {
        Self {
            noise_scale: 0.667,
            length_scale: 1.0,
            noise_w: 0.8,
        }
    }
}

const PAD: &str = "_";
const BOS: &str = "^";
const EOS: &str = "$";

/// The ids of `phonemes` in the voice's alphabet: its start, each phoneme followed by the pad, its end. A phoneme the
/// voice does not know is left out, as Piper does.
pub fn phoneme_ids(map: &HashMap<String, Vec<i64>>, phonemes: &str) -> Vec<i64> {
    let id = |k: &str| map.get(k).cloned().unwrap_or_default();
    let mut ids = id(BOS);
    let pad = id(PAD);
    let mut buf = [0u8; 4];
    for c in phonemes.chars() {
        if let Some(p) = map.get(c.encode_utf8(&mut buf) as &str) {
            ids.extend_from_slice(p);
            ids.extend_from_slice(&pad);
        }
    }
    ids.extend(id(EOS));
    ids
}

/// `text` cut where it is read with a pause: its sentences (each read on its own, the first heard sooner) and, inside,
/// its clauses with the mark that ends them, which Piper's voices were trained to hear (« , », « . », « ? »…).
pub fn sentences(text: &str) -> Vec<Vec<(String, char)>> {
    let mut out = Vec::new();
    let mut sentence = Vec::new();
    let mut clause = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        let end_of_sentence = matches!(c, '.' | '!' | '?' | '…' | '\n');
        let end_of_clause = matches!(c, ',' | ';' | ':');
        // « 3.5 », « 1,2 », « e.g »: no pause inside a word or a number.
        let inside = chars.peek().is_some_and(|n| !n.is_whitespace()) && c != '\n';
        if (end_of_sentence || end_of_clause) && !inside {
            let mark = match c {
                '…' | '\n' => '.',
                other => other,
            };
            if !clause.trim().is_empty() {
                sentence.push((clause.trim().to_string(), mark));
            }
            clause.clear();
            if end_of_sentence && !sentence.is_empty() {
                out.push(std::mem::take(&mut sentence));
            }
        } else {
            clause.push(c);
        }
    }
    if !clause.trim().is_empty() {
        sentence.push((clause.trim().to_string(), '.'));
    }
    if !sentence.is_empty() {
        out.push(sentence);
    }
    out
}

/// The voices' phonemizer: eSpeak NG, its data for French and English written out once into the app's data folder.
pub struct Phonemizer {
    dir: PathBuf,
    ready: Mutex<bool>,
    engines: Mutex<HashMap<String, espeak_ng::EspeakNg>>,
}

impl Phonemizer {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            ready: Mutex::new(false),
            engines: Mutex::new(HashMap::new()),
        }
    }

    fn data(&self) -> Result<()> {
        let mut ready = self.ready.lock();
        if *ready {
            return Ok(());
        }
        // Written again when the app's eSpeak data may have changed (another version of the app).
        let mark = self.dir.join(".escouade-version");
        let version = env!("CARGO_PKG_VERSION");
        if std::fs::read_to_string(&mark).ok().as_deref() != Some(version) {
            std::fs::create_dir_all(&self.dir)?;
            espeak_ng::install_bundled_languages(&self.dir, &["fr", "en"])
                .context("eSpeak NG's data")?;
            std::fs::write(&mark, version)?;
        }
        *ready = true;
        Ok(())
    }

    /// The phonemes of `clause` in eSpeak's `voice` (`fr`, `en-us`…), IPA as Piper's voices take them.
    pub fn phonemes(&self, voice: &str, clause: &str) -> Result<String> {
        self.data()?;
        let mut engines = self.engines.lock();
        if !engines.contains_key(voice) {
            let e = espeak_ng::EspeakNg::with_data_dir(voice, &self.dir)
                .or_else(|_| {
                    espeak_ng::EspeakNg::with_data_dir(
                        voice.split('-').next().unwrap_or(voice),
                        &self.dir,
                    )
                })
                .map_err(|e| anyhow::anyhow!("eSpeak NG ({voice}): {e:?}"))?;
            engines.insert(voice.to_string(), e);
        }
        let ipa = engines[voice]
            .text_to_phonemes(clause)
            .map_err(|e| anyhow::anyhow!("eSpeak NG: {e:?}"))?;
        // eSpeak cuts its own clauses with line breaks: one phrase for the voice.
        Ok(ipa
            .split('\n')
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" "))
    }
}

/// ONNX Runtime is loaded from a library at run time, not linked in: Intel Macs (pyke builds none for them) and Linux
/// (pyke's needs glibc 2.38), or the `ort-dynamic` feature (to try that path on another system).
const DYNAMIC: bool = cfg!(any(
    feature = "ort-dynamic",
    target_os = "linux",
    all(target_os = "macos", target_arch = "x86_64")
));

/// Microsoft's library, in the app: `Contents/Frameworks` of the bundle on a Mac, `/usr/lib/escouade` of the .deb (and
/// `usr/lib/escouade` of the AppImage) beside `bin/escouade` under Linux, beside the program elsewhere;
/// `ESCOUADE_ORT_DYLIB` in its place.
pub fn runtime_path() -> PathBuf {
    if let Some(p) = std::env::var_os("ESCOUADE_ORT_DYLIB").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    if cfg!(target_os = "macos") {
        dir.join("../Frameworks/libonnxruntime.1.23.2.dylib")
    } else if cfg!(windows) {
        dir.join("onnxruntime.dll")
    } else {
        let bundled = dir.join("../lib/escouade/libonnxruntime.so.1.23.2");
        if bundled.exists() {
            bundled
        } else {
            dir.join("libonnxruntime.so.1.23.2")
        }
    }
}

/// ONNX Runtime made ready, once: loaded from its library where it is not linked in.
pub fn runtime() -> Result<()> {
    static READY: std::sync::OnceLock<std::result::Result<(), String>> = std::sync::OnceLock::new();
    READY
        .get_or_init(|| {
            #[cfg(any(
                feature = "ort-dynamic",
                target_os = "linux",
                all(target_os = "macos", target_arch = "x86_64")
            ))]
            {
                let path = runtime_path();
                let started = std::time::Instant::now();
                ort::init_from(&path)
                    .map_err(|e| format!("ONNX Runtime ({}): {e}", path.display()))?
                    .commit();
                log::info!(
                    "speech: ONNX Runtime loaded from {} in {:?}",
                    path.display(),
                    started.elapsed()
                );
            }
            Ok(())
        })
        .clone()
        .map_err(|e| anyhow::anyhow!(e))
}

/// A pause between two sentences, in seconds.
const SENTENCE_GAP: f32 = 0.15;

/// The voice loaded, kept for the next sentences.
struct Loaded {
    path: PathBuf,
    session: ort::session::Session,
    config: VoiceConfig,
}

pub struct Piper {
    phonemizer: Phonemizer,
    loaded: Mutex<Option<Loaded>>,
}

impl Piper {
    pub fn new(espeak_dir: PathBuf) -> Self {
        Self {
            phonemizer: Phonemizer::new(espeak_dir),
            loaded: Mutex::new(None),
        }
    }
}

pub fn read_config(model: &Path) -> Result<VoiceConfig> {
    let json = PathBuf::from(format!("{}.json", model.display()));
    let text = std::fs::read_to_string(&json).with_context(|| json.display().to_string())?;
    Ok(serde_json::from_str(&text)?)
}

/// Brought to a peak of 0.9, as Piper does (its voices come out at very different levels).
pub fn normalize(samples: &mut [f32]) {
    let peak = samples.iter().fold(0f32, |m, s| m.max(s.abs())).max(0.01);
    let gain = 0.9 / peak;
    for s in samples {
        *s *= gain;
    }
}

impl Piper {
    /// `model` in `slot`, loaded there unless it already is (with ONNX Runtime, the first time).
    fn load<'a>(&self, slot: &'a mut Option<Loaded>, model: &Path) -> Result<&'a mut Loaded> {
        runtime()?;
        if slot.as_ref().map(|l| l.path.as_path()) != Some(model) {
            *slot = None;
            let config = read_config(model)?;
            let started = std::time::Instant::now();
            let session = ort::session::Session::builder()
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .commit_from_file(model)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            log::info!("speech: voice loaded in {:?}", started.elapsed());
            *slot = Some(Loaded {
                path: model.to_path_buf(),
                session,
                config,
            });
        }
        Ok(slot.as_mut().unwrap())
    }
}

impl Engine for Piper {
    fn available(&self) -> bool {
        !DYNAMIC || runtime_path().exists()
    }

    fn sentences(&self, text: &str) -> Vec<String> {
        sentences(text)
            .into_iter()
            .map(|s| {
                s.iter()
                    .map(|(c, m)| format!("{c}{m}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    }

    fn synthesize(&self, model: &Path, sentence: &str, opts: &SpeakOptions) -> Result<Audio> {
        use ort::value::Tensor;
        let mut slot = self.loaded.lock();
        let loaded = self.load(&mut slot, model)?;
        let config = &loaded.config;
        let mut phonemes = String::new();
        for (clause, mark) in sentences(sentence).into_iter().flatten() {
            if !phonemes.is_empty() {
                phonemes.push(' ');
            }
            phonemes.push_str(&self.phonemizer.phonemes(&config.espeak.voice, &clause)?);
            phonemes.push(mark);
        }
        let ids = phoneme_ids(&config.phoneme_id_map, &phonemes);
        if ids.len() <= 2 {
            return Ok(Audio {
                samples: Vec::new(),
                rate: config.audio.sample_rate,
            });
        }
        let n = ids.len();
        let speed = opts.rate.clamp(0.5, 2.0);
        let scales = vec![
            config.inference.noise_scale,
            config.inference.length_scale / speed,
            config.inference.noise_w,
        ];
        let input = Tensor::<i64>::from_array(([1usize, n], ids.into_boxed_slice()))?;
        let lengths = Tensor::<i64>::from_array(([1usize], vec![n as i64].into_boxed_slice()))?;
        let scales = Tensor::<f32>::from_array(([3usize], scales.into_boxed_slice()))?;
        let rate = config.audio.sample_rate;
        let multi = config.num_speakers > 1;
        let speaker = opts.speaker.min(config.num_speakers.saturating_sub(1)) as i64;
        let outputs = if multi {
            let sid = Tensor::<i64>::from_array(([1usize], vec![speaker].into_boxed_slice()))?;
            loaded
                .session
                .run(ort::inputs![input, lengths, scales, sid])
        } else {
            loaded.session.run(ort::inputs![input, lengths, scales])
        }
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        let (_, audio) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut samples = audio.to_vec();
        normalize(&mut samples);
        samples.extend(std::iter::repeat_n(
            0.0,
            (rate as f32 * SENTENCE_GAP) as usize,
        ));
        Ok(Audio { samples, rate })
    }

    fn preload(&self, model: &Path) -> Result<()> {
        let started = std::time::Instant::now();
        let espeak = {
            let mut slot = self.loaded.lock();
            self.load(&mut slot, model)?.config.espeak.voice.clone()
        };
        // eSpeak NG too: its data written out, its voice made, as the first sentence would.
        self.phonemizer.phonemes(&espeak, "a")?;
        log::info!("speech: voice ready in {:?}", started.elapsed());
        Ok(())
    }

    fn unload(&self) {
        *self.loaded.lock() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_cut_into_sentences_and_clauses_with_their_marks() {
        let s = sentences("Bonjour ! J'ai corrigé 3.5 tests, il en reste deux… Et toi ?\nFin");
        let flat: Vec<Vec<(String, char)>> = s;
        assert_eq!(flat.len(), 4, "{flat:?}");
        assert_eq!(flat[0], vec![("Bonjour".to_string(), '!')]);
        assert_eq!(
            flat[1],
            vec![
                ("J'ai corrigé 3.5 tests".to_string(), ','),
                ("il en reste deux".to_string(), '.')
            ]
        );
        assert_eq!(flat[2], vec![("Et toi".to_string(), '?')]);
        assert_eq!(flat[3], vec![("Fin".to_string(), '.')]);
        assert!(sentences("  \n ").is_empty());
    }

    #[test]
    fn phonemes_become_the_voice_s_ids_padded_between_start_and_end() {
        let map: HashMap<String, Vec<i64>> =
            [("_", 0), ("^", 1), ("$", 2), ("a", 14), ("b", 15), (" ", 3)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), vec![v]))
                .collect();
        // « ʒ » is unknown to this voice: left out.
        assert_eq!(
            phoneme_ids(&map, "ab ʒa"),
            vec![1, 14, 0, 15, 0, 3, 0, 14, 0, 2]
        );
    }

    #[test]
    fn espeak_gives_the_phonemes_of_french_and_english() {
        let dir = crate::test_dir("speech-espeak");
        let p = Phonemizer::new(dir.join("espeak-ng-data"));
        assert_eq!(p.phonemes("fr", "bonjour").unwrap(), "bɔ̃ʒˈuʁ");
        assert_eq!(p.phonemes("en-us", "hello world").unwrap(), "həlˈoʊ wˈɜːld");
        // Written once: a second phonemizer reads the same folder.
        assert!(dir
            .join("espeak-ng-data")
            .join(".escouade-version")
            .exists());
        let again = Phonemizer::new(dir.join("espeak-ng-data"));
        assert_eq!(again.phonemes("en-gb", "dog").unwrap(), "dˈɒɡ");
    }

    #[test]
    fn a_voice_s_settings_are_read_beside_its_model() {
        let dir = crate::test_dir("speech-config");
        let model = dir.join("v.onnx");
        std::fs::write(
            dir.join("v.onnx.json"),
            r#"{"audio":{"sample_rate":22050},"espeak":{"voice":"fr"},"inference":{"noise_scale":0.5,"length_scale":1.1,"noise_w":0.7},"num_speakers":2,"phoneme_id_map":{"_":[0]}}"#,
        )
        .unwrap();
        let c = read_config(&model).unwrap();
        assert_eq!(
            (
                c.audio.sample_rate,
                c.num_speakers,
                c.inference.length_scale
            ),
            (22050, 2, 1.1)
        );
        assert_eq!(c.espeak.voice, "fr");
    }

    #[test]
    fn the_sound_is_brought_to_the_same_peak() {
        let mut s = vec![0.1, -0.3, 0.2];
        normalize(&mut s);
        assert!((s[1] + 0.9).abs() < 1e-6 && (s[0] - 0.3).abs() < 1e-6);
    }

    /// ESC-115, measured with a real voice (`ESCOUADE_SPEECH_TEST_VOICE`), by hand: the first sentence waits for the
    /// voice when it was not loaded ahead (`ESCOUADE_SPEECH_TEST_COLD`, as before), not once it was.
    #[test]
    fn a_real_voice_loaded_ahead_reads_its_first_sentence_as_fast_as_the_next() {
        let Ok(voice) = std::env::var("ESCOUADE_SPEECH_TEST_VOICE") else {
            return;
        };
        let cold = std::env::var_os("ESCOUADE_SPEECH_TEST_COLD").is_some();
        let dir = crate::test_dir("speech-preload");
        let piper = std::sync::Arc::new(Piper::new(dir.join("espeak-ng-data")));
        let model = PathBuf::from(&voice);
        if !cold {
            let (p, m) = (piper.clone(), model.clone());
            let started = std::time::Instant::now();
            std::thread::spawn(move || p.preload(&m))
                .join()
                .unwrap()
                .unwrap();
            eprintln!("loaded ahead in {:?}", started.elapsed());
        }
        let timed = |text: &str| {
            let started = std::time::Instant::now();
            piper
                .synthesize(&model, text, &SpeakOptions::default())
                .unwrap();
            started.elapsed()
        };
        let first = timed("C'est fait, les tests passent.");
        let next = timed("Je regarde la page des réglages.");
        eprintln!("first sentence in {first:?}, the next in {next:?} (cold: {cold})");
        if !cold {
            assert!(
                first < next * 2 + std::time::Duration::from_millis(150),
                "{first:?} / {next:?}"
            );
        }
    }

    /// The real thing, when a voice is given (`ESCOUADE_SPEECH_TEST_VOICE`, its `.onnx`): speaks a French sentence,
    /// written to `<target>/speech-test.wav`; with a Whisper model too (`ESCOUADE_VOICE_TEST_MODEL`), checks that it
    /// is understood. Run by hand: the CI has no voice.
    #[test]
    fn piper_speaks_with_a_real_voice_when_one_is_given() {
        let Ok(voice) = std::env::var("ESCOUADE_SPEECH_TEST_VOICE") else {
            return;
        };
        let dir = crate::test_dir("speech-real");
        let piper = Piper::new(dir.join("espeak-ng-data"));
        let text = std::env::var("ESCOUADE_SPEECH_TEST_TEXT")
            .unwrap_or_else(|_| "Bonjour ! J'ai corrigé les tests de la page des réglages.".into());
        let started = std::time::Instant::now();
        let mut all = Vec::new();
        let mut rate = 0;
        for s in piper.sentences(&text) {
            let a = piper
                .synthesize(Path::new(&voice), &s, &SpeakOptions::default())
                .unwrap();
            rate = a.rate;
            all.extend(a.samples);
        }
        eprintln!(
            "{:.1} s of speech in {:?}",
            all.len() as f32 / rate as f32,
            started.elapsed()
        );
        assert!(all.len() as f32 / rate as f32 > 1.5);
        let wav = dir.join("speech-test.wav");
        let mut w = hound::WavWriter::create(
            &wav,
            hound::WavSpec {
                channels: 1,
                sample_rate: rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for s in &all {
            w.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .unwrap();
        }
        w.finalize().unwrap();
        eprintln!("written to {}", wav.display());
        if let Ok(model) = std::env::var("ESCOUADE_VOICE_TEST_MODEL") {
            use crate::transcribe::Transcriber;
            let pcm = crate::audio::resample(&all, rate, crate::audio::WHISPER_RATE);
            let heard = crate::transcribe::Whisper::default()
                .transcribe(
                    Path::new(&model),
                    &pcm,
                    "fr",
                    &std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                )
                .unwrap();
            eprintln!("Whisper heard: {heard}");
            let low = heard.to_lowercase();
            assert!(low.contains("bonjour") && low.contains("test"), "{heard}");
        }
    }
}
