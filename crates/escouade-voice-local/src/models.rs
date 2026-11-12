//! Whisper's models: which there are, which are on the disk, their download on demand (through the
//! proxy of the settings, its progress told, its SHA-256 checked before it is kept).

use anyhow::{bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// A model of the catalog.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo {
    /// Its name in whisper.cpp's (`ggml-<id>.bin`), what the settings keep.
    pub id: &'static str,
    pub size: u64,
    pub sha256: &'static str,
    /// How fast it transcribes and how well, from 1 to 5.
    pub speed: u8,
    pub quality: u8,
}

/// The model a first dictation downloads (decided with the user, 2026-10-16): ~150 MB, good in
/// French and in English.
pub const DEFAULT_MODEL: &str = "base";

/// whisper.cpp's models as Hugging Face holds them, their sizes and SHA-256 those of its LFS
/// pointers (2026-10-16).
pub const CATALOG: &[ModelInfo] = &[
    ModelInfo {
        id: "tiny",
        size: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        speed: 5,
        quality: 1,
    },
    ModelInfo {
        id: "base",
        size: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        speed: 4,
        quality: 2,
    },
    ModelInfo {
        id: "small",
        size: 487_601_967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        speed: 3,
        quality: 3,
    },
    ModelInfo {
        id: "large-v3-turbo-q5_0",
        size: 574_041_195,
        sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
        speed: 2,
        quality: 4,
    },
    ModelInfo {
        id: "medium",
        size: 1_533_763_059,
        sha256: "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
        speed: 1,
        quality: 4,
    },
    ModelInfo {
        id: "large-v3-turbo",
        size: 1_624_555_275,
        sha256: "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
        speed: 2,
        quality: 5,
    },
];

/// Where the models are downloaded from: `<base>/ggml-<id>.bin`.
pub const MODELS_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

pub fn find(id: &str) -> Option<&'static ModelInfo> {
    CATALOG.iter().find(|m| m.id == id)
}

/// A model as the settings show it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    pub id: String,
    pub size: u64,
    pub speed: u8,
    pub quality: u8,
    /// On the disk, checked.
    pub downloaded: bool,
}

/// The models' folder, and what it holds.
#[derive(Debug, Clone)]
pub struct Store {
    pub dir: PathBuf,
}

impl Store {
    pub fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("ggml-{id}.bin"))
    }

    fn part(&self, id: &str) -> PathBuf {
        self.dir.join(format!("ggml-{id}.bin.part"))
    }

    /// On the disk: a file of the catalog's size (its SHA-256 was checked as it came; reading
    /// 150 MB to check it again at each look would take a second).
    pub fn has(&self, m: &ModelInfo) -> bool {
        std::fs::metadata(self.path(m.id)).is_ok_and(|md| md.len() == m.size)
    }

    pub fn list(&self) -> Vec<ModelView> {
        CATALOG
            .iter()
            .map(|m| ModelView {
                id: m.id.into(),
                size: m.size,
                speed: m.speed,
                quality: m.quality,
                downloaded: self.has(m),
            })
            .collect()
    }

    /// The model and what is left of a download of it.
    pub fn delete(&self, id: &str) -> Result<()> {
        for p in [self.path(id), self.part(id)] {
            match std::fs::remove_file(&p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e).with_context(|| p.display().to_string()),
            }
        }
        Ok(())
    }

    /// Downloads `m` from `base` with `client`, `progress` told the bytes so far (every 100 ms at
    /// most, and at the end), until `cancel` is set. Written to a `.part` first: the model is only
    /// there once whole and its SHA-256 the catalog's.
    pub async fn download(
        &self,
        client: &reqwest::Client,
        base: &str,
        m: &ModelInfo,
        cancel: &AtomicBool,
        progress: impl FnMut(u64),
    ) -> Result<PathBuf> {
        let url = format!("{}/ggml-{}.bin", base.trim_end_matches('/'), m.id);
        let path = self.path(m.id);
        fetch_checked(client, &url, m.size, m.sha256, &path, cancel, progress).await?;
        Ok(path)
    }
}

/// Downloads `url` with `client` into `path`, `progress` told the bytes so far (every 100 ms at most, and at the
/// end), until `cancel` is set. Written to `<path>.part` first: the file is only there once whole, of `size` bytes
/// and its SHA-256 `sha256`.
pub async fn fetch_checked(
    client: &reqwest::Client,
    url: &str,
    size: u64,
    sha256: &str,
    path: &std::path::Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut res = client
        .get(url)
        .header("User-Agent", "escouade")
        .send()
        .await?
        .error_for_status()?;
    let part = PathBuf::from(format!("{}.part", path.display()));
    let result: Result<()> = async {
        let mut file = std::io::BufWriter::new(std::fs::File::create(&part)?);
        let mut hash = Sha256::new();
        let mut done = 0u64;
        let mut told = Instant::now();
        progress(0);
        while let Some(chunk) = res.chunk().await? {
            if cancel.load(Ordering::Acquire) {
                bail!(Cancelled);
            }
            file.write_all(&chunk)?;
            hash.update(&chunk);
            done += chunk.len() as u64;
            if done > size {
                bail!("{} bytes and more, {} expected", done, size);
            }
            if told.elapsed() >= Duration::from_millis(100) {
                told = Instant::now();
                progress(done);
            }
        }
        file.flush()?;
        drop(file);
        if done != size {
            bail!("{done} bytes received, {} expected", size);
        }
        let got = hash
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if got != sha256 {
            bail!(ChecksumMismatch);
        }
        progress(done);
        Ok(())
    }
    .await;
    if let Err(e) = result {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    std::fs::rename(&part, path)?;
    Ok(())
}

/// A download called off by the user.
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// What was downloaded is not the model: not kept.
#[derive(Debug)]
pub struct ChecksumMismatch;

impl std::fmt::Display for ChecksumMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("checksum mismatch")
    }
}

impl std::error::Error for ChecksumMismatch {}
