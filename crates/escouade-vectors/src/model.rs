//! The embedding model: which one, where it is kept, and its download on first activation (through the proxy of the
//! settings, its progress told, its SHA-256 checked, taken up again where a cut left it).

use super::embed::{Onnx, Prefixes};
use anyhow::{bail, Result};
use escouade_voice_local::models::{Cancelled, ChecksumMismatch};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// A file of the model.
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    /// Its name on the disk, and after the model's URL.
    pub name: &'static str,
    /// Where it is, from the model's URL.
    pub url: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    pub id: &'static str,
    /// The licence of the model's weights.
    pub license: &'static str,
    pub dim: usize,
    pub prefixes: Prefixes,
    pub files: &'static [File],
}

/// Chosen on `vectors::tests::candidates_compared` (2026-10-28, see docs/SPEC.md): Microsoft's multilingual E5 base,
/// 278 M parameters quantized to 8 bits, 768 dimensions, MIT licence, 295 MB to download. XLM-R's vocabulary (100
/// languages), 512 tokens. It beat the small E5 and the multilingual MiniLM on the French and English set.
pub const MODEL: Spec = Spec {
    id: "multilingual-e5-base",
    license: "MIT",
    dim: 768,
    prefixes: Prefixes {
        query: "query: ",
        passage: "passage: ",
    },
    files: &[
        File {
            name: "model.onnx",
            url: "onnx/model_int8.onnx",
            size: 278_184_162,
            sha256: "9ddfd8b45086dabc59a7e1bb00463225dace8954962418b240840f2153bc87da",
        },
        File {
            name: "tokenizer.json",
            url: "tokenizer.json",
            size: 17_082_660,
            sha256: "62c24cdc13d4c9952d63718d6c9fa4c287974249e16b7ade6d5a85e7bbb75626",
        },
    ],
};

/// Where the model is downloaded from: `<base>/<file.url>`.
pub const MODEL_URL: &str = "https://huggingface.co/Xenova/multilingual-e5-base/resolve/main";

impl Spec {
    pub fn size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

/// The model's folder (`vectors/<id>` of the app's data), and what it holds.
#[derive(Debug, Clone)]
pub struct Store {
    pub dir: PathBuf,
}

impl Store {
    pub fn path(&self, f: &File) -> PathBuf {
        self.dir.join(f.name)
    }

    fn part(&self, f: &File) -> PathBuf {
        self.dir.join(format!("{}.part", f.name))
    }

    /// Every file is there, of the right size (the SHA-256 was checked as each came; `verify` reads them again).
    pub fn has(&self, spec: &Spec) -> bool {
        spec.files
            .iter()
            .all(|f| std::fs::metadata(self.path(f)).is_ok_and(|m| m.len() == f.size))
    }

    /// Bytes on the disk toward the model, for a progress shown on a return after a cut.
    pub fn have(&self, spec: &Spec) -> u64 {
        spec.files
            .iter()
            .map(|f| {
                [self.path(f), self.part(f)]
                    .iter()
                    .filter_map(|p| std::fs::metadata(p).ok())
                    .map(|m| m.len())
                    .next()
                    .unwrap_or(0)
            })
            .sum()
    }

    /// Reads the files again and compares their SHA-256 (a file changed on the disk, a disk that failed).
    pub fn verify(&self, spec: &Spec) -> bool {
        spec.files
            .iter()
            .all(|f| sha_of(&self.path(f)).is_ok_and(|h| h == f.sha256))
    }

    /// The model and what is left of a download of it.
    pub fn delete(&self, spec: &Spec) -> Result<()> {
        for f in spec.files {
            for p in [self.path(f), self.part(f)] {
                match std::fs::remove_file(&p) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Ok(())
    }

    /// The model loaded, to compute vectors with.
    pub fn load(&self, spec: &Spec) -> Result<Onnx> {
        if !self.has(spec) {
            bail!("the model {} is not downloaded", spec.id);
        }
        Onnx::load(
            spec.id,
            &self.path(&spec.files[0]),
            &self.path(&spec.files[1]),
            spec.prefixes,
        )
    }

    /// Downloads what of `spec` is missing from `base`, `progress` told `(bytes so far, bytes in all)` every 100 ms at
    /// most and at the end, until `cancel` is set. A file is written to `<name>.part` and only takes its name once whole
    /// and its SHA-256 the catalog's. A cut (the network, `cancel`) keeps the `.part`: the next call asks the server for
    /// the rest (`Range`) and goes on. What is not the model (wrong hash, too long) is deleted.
    pub async fn download(
        &self,
        client: &reqwest::Client,
        base: &str,
        spec: &Spec,
        cancel: &AtomicBool,
        mut progress: impl FnMut(u64, u64),
    ) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let total = spec.size();
        let mut before = 0u64;
        for f in spec.files {
            let url = format!("{}/{}", base.trim_end_matches('/'), f.url);
            if std::fs::metadata(self.path(f)).is_ok_and(|m| m.len() == f.size) {
                before += f.size;
                progress(before, total);
                continue;
            }
            fetch(
                client,
                &url,
                f,
                &self.path(f),
                &self.part(f),
                cancel,
                |done| progress(before + done, total),
            )
            .await?;
            before += f.size;
        }
        Ok(())
    }
}

fn sha_of(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>())
}

async fn fetch(
    client: &reqwest::Client,
    url: &str,
    f: &File,
    path: &Path,
    part: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<()> {
    // What a cut download left: kept if it is shorter than the file, else it is not a start of this one.
    let mut have = std::fs::metadata(part).map_or(0, |m| m.len());
    if have >= f.size {
        let _ = std::fs::remove_file(part);
        have = 0;
    }
    let mut req = client.get(url).header("User-Agent", "escouade");
    if have > 0 {
        req = req.header("Range", format!("bytes={have}-"));
    }
    let mut res = req.send().await?.error_for_status()?;
    // A server that ignores the Range sends everything (200): start again.
    if have > 0 && res.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        have = 0;
    }
    let mut hash = Sha256::new();
    let mut file = if have > 0 {
        let mut old = std::fs::File::open(part)?;
        let mut buf = vec![0u8; 1 << 20];
        let mut left = have;
        while left > 0 {
            let want = left.min(buf.len() as u64) as usize;
            let n = old.read(&mut buf[..want])?;
            if n == 0 {
                bail!("{} is shorter than it was", part.display());
            }
            hash.update(&buf[..n]);
            left -= n as u64;
        }
        std::io::BufWriter::new(std::fs::OpenOptions::new().append(true).open(part)?)
    } else {
        std::io::BufWriter::new(std::fs::File::create(part)?)
    };
    let mut done = have;
    let mut told = Instant::now();
    progress(done);
    let result: Result<()> = async {
        while let Some(chunk) = res.chunk().await? {
            if cancel.load(Ordering::Acquire) {
                file.flush()?;
                bail!(Cancelled);
            }
            file.write_all(&chunk)?;
            hash.update(&chunk);
            done += chunk.len() as u64;
            if done > f.size {
                bail!("{done} bytes and more, {} expected", f.size);
            }
            if told.elapsed() >= Duration::from_millis(100) {
                told = Instant::now();
                progress(done);
            }
        }
        file.flush()?;
        Ok(())
    }
    .await;
    drop(file);
    if let Err(e) = result {
        // A cut keeps what came; a download that is not the file does not.
        if e.downcast_ref::<Cancelled>().is_none() && e.downcast_ref::<reqwest::Error>().is_none() {
            let _ = std::fs::remove_file(part);
        }
        return Err(e);
    }
    if done != f.size {
        // The connection closed early without an error: the next call goes on from here.
        bail!("{done} bytes received, {} expected", f.size);
    }
    if hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        != f.sha256
    {
        let _ = std::fs::remove_file(part);
        bail!(ChecksumMismatch);
    }
    progress(done);
    std::fs::rename(part, path)?;
    Ok(())
}
