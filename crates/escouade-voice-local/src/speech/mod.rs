//! Speech: what the agent writes to be read read aloud, on the machine, by Piper (ESC-97). Its voices are those of
//! `catalog` (their licences checked), downloaded on demand into the app's data folder, their size and SHA-256 checked;
//! the sound goes out through cpal.

pub mod catalog;
pub mod output;
pub mod piper;

use anyhow::Result;
use catalog::VoiceInfo;
use parking_lot::Mutex;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// Sound made: mono samples in [-1, 1], and their rate.
#[derive(Debug, Clone, PartialEq)]
pub struct Audio {
    pub samples: Vec<f32>,
    pub rate: u32,
}

/// How a voice reads, from the settings.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeakOptions {
    /// The speaker of a voice that has several (0 otherwise).
    pub speaker: u32,
    /// How fast: 1 is the voice's own pace, 2 twice as fast.
    pub rate: f32,
}

impl Default for SpeakOptions {
    fn default() -> Self {
        Self {
            speaker: 0,
            rate: 1.0,
        }
    }
}

/// What turns text into sound: Piper, or a fake one (tests, end-to-end tests).
pub trait Engine: Send + Sync {
    /// Whether it can speak on this machine (no ONNX Runtime for Intel Macs).
    fn available(&self) -> bool;
    /// `text` cut into the pieces made one at a time (its sentences): the first one is heard sooner.
    fn sentences(&self, text: &str) -> Vec<String>;
    /// The sound of `sentence` in the voice whose model is `model` (`<id>.onnx`, its `.onnx.json` beside it).
    fn synthesize(&self, model: &Path, sentence: &str, opts: &SpeakOptions) -> Result<Audio>;
    /// Loads the voice whose model is `model` (and what it needs) ahead of its first sentence, which then does not
    /// wait for it (ESC-115). Blocks: called on a thread of its own. A voice already loaded is not loaded again.
    fn preload(&self, model: &Path) -> Result<()>;
    /// The voice loaded lets go of its memory (its file is about to be deleted).
    fn unload(&self);
}

/// A voice that is no voice: a tone, 50 ms a character (4 s at most), and the sentences it was asked written down
/// (`said`, and the file of `ESCOUADE_SPEECH_FAKE` in end-to-end tests). It loads as Piper does: a voice once, on
/// its first sentence or ahead of it (`preload`), `load_time` long, each load written down (`loads`); ahead of it,
/// it fails when `preload_fails` says so.
pub struct Fake {
    pub said: Mutex<Vec<String>>,
    pub log: Option<PathBuf>,
    pub load_time: std::time::Duration,
    pub loads: Mutex<Vec<FakeLoad>>,
    pub preload_fails: bool,
    loaded: Mutex<Option<PathBuf>>,
}

/// A voice the fake loaded: which, and whether ahead of its first sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct FakeLoad {
    pub model: PathBuf,
    pub ahead: bool,
}

impl Fake {
    pub fn new(log: Option<PathBuf>) -> Self {
        Self {
            said: Mutex::new(Vec::new()),
            log,
            load_time: std::time::Duration::ZERO,
            loads: Mutex::new(Vec::new()),
            preload_fails: false,
            loaded: Mutex::new(None),
        }
    }

    fn load(&self, model: &Path, ahead: bool) -> Result<()> {
        let mut loaded = self.loaded.lock();
        if loaded.as_deref() == Some(model) {
            return Ok(());
        }
        *loaded = None;
        if !model.exists() {
            anyhow::bail!("{}: no such voice", model.display());
        }
        std::thread::sleep(self.load_time);
        self.loads.lock().push(FakeLoad {
            model: model.to_path_buf(),
            ahead,
        });
        *loaded = Some(model.to_path_buf());
        Ok(())
    }
}

pub const FAKE_RATE: u32 = 16_000;

impl Engine for Fake {
    fn available(&self) -> bool {
        true
    }

    fn sentences(&self, text: &str) -> Vec<String> {
        piper::sentences(text)
            .into_iter()
            .map(|s| {
                s.iter()
                    .map(|(c, m)| format!("{c}{m}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    }

    fn synthesize(&self, model: &Path, sentence: &str, _opts: &SpeakOptions) -> Result<Audio> {
        self.load(model, false)?;
        self.said.lock().push(sentence.to_string());
        if let Some(log) = &self.log {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)?;
            writeln!(f, "{sentence}")?;
        }
        let ms = (sentence.chars().count() as u32 * 50).min(4_000);
        let n = (FAKE_RATE * ms / 1000) as usize;
        let samples = (0..n)
            .map(|i| (i as f32 * 2.0 * std::f32::consts::PI * 300.0 / FAKE_RATE as f32).sin() * 0.3)
            .collect();
        Ok(Audio {
            samples,
            rate: FAKE_RATE,
        })
    }

    fn preload(&self, model: &Path) -> Result<()> {
        if self.preload_fails {
            anyhow::bail!("ONNX Runtime: not loaded");
        }
        self.load(model, true)
    }

    fn unload(&self) {
        *self.loaded.lock() = None;
    }
}

/// A voice as the settings show it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoiceView {
    #[serde(flatten)]
    pub info: VoiceInfo,
    /// Its two files on the disk, of their sizes.
    pub downloaded: bool,
}

/// The voices' folder, and what it holds.
#[derive(Debug, Clone)]
pub struct VoiceStore {
    pub dir: PathBuf,
}

impl VoiceStore {
    pub fn model(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.onnx"))
    }

    pub fn config(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.onnx.json"))
    }

    pub fn has(&self, v: &VoiceInfo) -> bool {
        let size = |p: PathBuf| std::fs::metadata(p).map(|m| m.len()).ok();
        size(self.model(&v.id)) == Some(v.onnx_size)
            && size(self.config(&v.id)) == Some(v.json_size)
    }

    pub fn list(&self) -> Vec<VoiceView> {
        catalog::shipped()
            .iter()
            .map(|v| VoiceView {
                info: v.clone(),
                downloaded: self.has(v),
            })
            .collect()
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        for p in [self.model(id), self.config(id)] {
            for p in [p.clone(), PathBuf::from(format!("{}.part", p.display()))] {
                match std::fs::remove_file(&p) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Ok(())
    }

    /// Downloads `v` from `base` (`https://huggingface.co`: `<base>/<repo>/resolve/main/<file>`), its settings first
    /// then its model, `progress` told the bytes of both so far.
    pub async fn download(
        &self,
        client: &reqwest::Client,
        base: &str,
        v: &VoiceInfo,
        cancel: &AtomicBool,
        mut progress: impl FnMut(u64),
    ) -> Result<()> {
        let url = |file: &str| {
            format!(
                "{}/{}/resolve/main/{}",
                base.trim_end_matches('/'),
                v.repo,
                file
            )
        };
        crate::models::fetch_checked(
            client,
            &url(&v.json),
            v.json_size,
            &v.json_sha256,
            &self.config(&v.id),
            cancel,
            |_| {},
        )
        .await?;
        let json = v.json_size;
        crate::models::fetch_checked(
            client,
            &url(&v.onnx),
            v.onnx_size,
            &v.onnx_sha256,
            &self.model(&v.id),
            cancel,
            |d| progress(json + d),
        )
        .await
    }
}

/// Where the voices are downloaded from.
pub const VOICES_URL: &str = "https://huggingface.co";
