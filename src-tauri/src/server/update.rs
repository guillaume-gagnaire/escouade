//! `escouade-server update` (ESC-515, docs/REMOTE.md « Le mode serveur »): the server put at a version of the releases,
//! signed with the key of the app's updater.
//!
//! 1. The version: `--to X.Y.Z` (the client's, when it offers « Mettre à jour le serveur »), else the one of
//!    `latest.json` (or `beta.json` with `--beta`) of the latest release, as the app's updater reads.
//! 2. The archive `escouade-server-<version>-linux-<x64|arm64>.tar.gz` and its signature (`.sig`, the Tauri updater's:
//!    a minisign signature, in base64) downloaded; the signature checked with [`UPDATER_PUBLIC_KEY`] before anything is
//!    touched. A refused signature changes nothing.
//! 3. Each file of the archive (the program, `seed/`) put in place of the one beside the running program, the old one
//!    kept as `<name>.old`: Linux runs on, from the file it opened.
//! 4. The running server (its `server-status.json`) stopped once nothing would be cut: it waits for the turns, the
//!    questions, the background tasks, a terminal used lately (`updates::busy`), or, asked (`--now`, or `m` at the
//!    question), cuts them as a stop does (each agent resumes its session). Its supervisor (systemd's `Restart=always`)
//!    starts it again on the new files; without one, it says to start it.
//!
//! In the image (`ESCOUADE_SERVER_IMAGE`), it changes nothing: a new image is the update.

use super::status::{self, Status, Supervisor};
use crate::paths::DataDir;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The releases of Escouade.
pub const RELEASES: &str = "https://github.com/guillaume-gagnaire/escouade/releases";
/// Another address of the releases (a mirror, the tests' server): the signature is checked all the same.
pub const URL_VAR: &str = "ESCOUADE_SERVER_UPDATE_URL";
/// In a build of development only, another public key (the tests sign with a key of their own).
pub const KEY_VAR: &str = "ESCOUADE_SERVER_UPDATE_PUBKEY";

/// The public key of the app's updater (`plugins.updater.pubkey` of `tauri.conf.json`, the same: a test says so), which
/// signs the server's archives too.
pub const UPDATER_PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDU3NzMxM0UzRTI4MkZDRjYKUldUMi9JTGk0eE56Vi93TCs0dEUyL1hBOXEwOEU0RmYvUFgrbWpHa0tWNVFLR1lLdVltRXNvaHYK";

/// What `update` was asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Asked {
    /// `--to`: this version, not the latest.
    pub to: Option<String>,
    /// `--beta`: the beta channel's latest.
    pub beta: bool,
    /// `--now`: no waiting for the turns.
    pub now: bool,
    /// `--check`: says what it would do, does nothing.
    pub check: bool,
}

/// The architecture's name in the archives.
pub fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    }
}

pub fn archive_name(version: &str, arch: &str) -> String {
    format!("escouade-server-{version}-linux-{arch}.tar.gz")
}

pub fn archive_url(base: &str, version: &str, arch: &str) -> String {
    format!(
        "{}/download/v{version}/{}",
        base.trim_end_matches('/'),
        archive_name(version, arch)
    )
}

pub fn channel_url(base: &str, beta: bool) -> String {
    format!(
        "{}/latest/download/{}",
        base.trim_end_matches('/'),
        if beta { "beta.json" } else { "latest.json" }
    )
}

/// A version as `X.Y.Z` or `X.Y.Z-beta.N`, nothing else (it goes in an address).
pub fn is_version(v: &str) -> bool {
    let (core, pre) = v.split_once('-').unwrap_or((v, ""));
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        && pre.chars().all(|c| c.is_ascii_alphanumeric() || c == '.')
}

/// `data` signed as the Tauri updater signs (`sig`: the minisign signature file in base64) with `key` (the minisign
/// public key file in base64, as `tauri.conf.json` holds it).
pub fn verify(data: &[u8], sig: &str, key: &str) -> Result<()> {
    use base64::Engine as _;
    let b64 = |s: &str| -> Result<String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(s.trim())
            .context("base64")?;
        Ok(String::from_utf8(bytes)?)
    };
    let key =
        minisign_verify::PublicKey::decode(&b64(key)?).map_err(|e| anyhow!("public key: {e}"))?;
    let sig =
        minisign_verify::Signature::decode(&b64(sig)?).map_err(|e| anyhow!("signature: {e}"))?;
    key.verify(data, &sig, false).map_err(|e| anyhow!("{e}"))
}

/// The key that checks the archives: the updater's; another in a build of development when `ESCOUADE_SERVER_UPDATE_PUBKEY`
/// gives one (never in a release: a variable never weakens it).
fn key() -> String {
    if cfg!(debug_assertions) {
        if let Ok(k) = std::env::var(KEY_VAR) {
            if !k.trim().is_empty() {
                return k;
            }
        }
    }
    UPDATER_PUBLIC_KEY.to_string()
}

fn base() -> String {
    std::env::var(URL_VAR)
        .ok()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| RELEASES.to_string())
}

/// The files of the archive `bytes` (a tar.gz, its files at its top or in one folder) put in `dir`, each in place of
/// the one of the same name, which is kept as `<name>.old` (the one before it removed). The program must be in it.
/// Nothing in `dir` is touched until the whole archive is read.
pub fn install(bytes: &[u8], dir: &Path) -> Result<Vec<String>> {
    let staging = dir.join(format!(".escouade-server-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)?;
    let unpacked = (|| -> Result<PathBuf> {
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
        archive.set_preserve_permissions(true);
        for entry in archive.entries()? {
            let mut entry = entry?;
            // `unpack_in` refuses a path that leaves the folder (`..`, absolute).
            if !entry.unpack_in(&staging)? {
                bail!("the archive holds a path out of its folder");
            }
        }
        if staging.join(PROGRAM).is_file() {
            return Ok(staging.clone());
        }
        // Or in one folder at the top.
        let top: Vec<_> = std::fs::read_dir(&staging)?.collect::<std::io::Result<_>>()?;
        match &top[..] {
            [one] if one.path().join(PROGRAM).is_file() => Ok(one.path()),
            _ => bail!(tr!(
                "l’archive ne contient pas {PROGRAM}",
                "the archive holds no {PROGRAM}"
            )),
        }
    })();
    let root = match unpacked {
        Ok(root) => root,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e);
        }
    };
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let dest = dir.join(&name);
        let old = dir.join(format!("{name}.old"));
        if dest.exists() || dest.is_symlink() {
            remove_any(&old);
            std::fs::rename(&dest, &old)
                .with_context(|| format!("{} → {}", dest.display(), old.display()))?;
        }
        std::fs::rename(entry.path(), &dest).with_context(|| format!("{}", dest.display()))?;
        names.push(name);
    }
    let _ = std::fs::remove_dir_all(&staging);
    names.sort();
    Ok(names)
}

fn remove_any(path: &Path) {
    if path.is_dir() && !path.is_symlink() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

/// The program's name in the archives and beside the seed.
pub const PROGRAM: &str = "escouade-server";

/// What the running server must do before a restart, as `update` decides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wait {
    /// Nothing runs: the new files serve at its next start.
    NotRunning,
    /// At rest: stopped now.
    Now,
    /// Busy for this reason (`status::busy_text`).
    Busy(String),
}

/// What the server behind `status` makes `update` do, at `now`.
pub fn wait_of(status: Option<&Status>, alive: impl Fn(u32) -> bool) -> Wait {
    match status {
        None => Wait::NotRunning,
        Some(s) if !alive(s.pid) => Wait::NotRunning,
        Some(s) => match &s.busy {
            Some(why) => Wait::Busy(why.clone()),
            None => Wait::Now,
        },
    }
}

/// `escouade-server update` on the data folder `data`.
pub fn run(data: &DataDir, asked: &Asked) -> Result<()> {
    if status::supervisor() == Some(Supervisor::Docker) {
        bail!(tr!(
            "dans l’image Docker, la mise à jour est une nouvelle image : « docker compose pull && docker compose up -d » (ou « docker pull » puis le conteneur recréé sur le même volume). Rien n’est changé.",
            "in the Docker image, an update is a new image: “docker compose pull && docker compose up -d” (or “docker pull”, then the container made again on the same volume). Nothing is changed."
        ));
    }
    if !cfg!(target_os = "linux") && !asked.check {
        bail!(tr!(
            "la mise à jour du serveur ne se fait que sous Linux",
            "the server updates itself under Linux only"
        ));
    }
    let current = env!("CARGO_PKG_VERSION");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let settings: crate::model::Settings = std::fs::read(data.settings_file())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let client = crate::tls::app_builder(&settings)?
        .timeout(Duration::from_secs(300))
        .user_agent(concat!("escouade-server/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let base = base();
    let target = match &asked.to {
        Some(v) => v.trim_start_matches('v').to_string(),
        None => {
            let url = channel_url(&base, asked.beta);
            let body: serde_json::Value = runtime
                .block_on(async {
                    let r = client.get(&url).send().await?.error_for_status()?;
                    anyhow::Ok(r.json().await?)
                })
                .with_context(|| url.clone())?;
            body["version"]
                .as_str()
                .ok_or_else(|| anyhow!("{url}: no version"))?
                .trim_start_matches('v')
                .to_string()
        }
    };
    if !is_version(&target) {
        bail!(tr!(
            "« {target} » n’est pas une version (X.Y.Z)",
            "“{target}” is not a version (X.Y.Z)"
        ));
    }
    let running = status::read(data).filter(|s| status::alive(s.pid));
    if asked.check {
        println!(
            "{}",
            tr!(
                "Programme : {current}\nServeur lancé : {}\nVersion visée : {target}",
                "Program: {current}\nServer running: {}\nTarget version: {target}",
                running
                    .as_ref()
                    .map(|s| s.version.clone())
                    .unwrap_or_else(|| tr!("aucun", "none"))
            )
        );
        return Ok(());
    }
    let dir = std::env::current_exe()?
        .canonicalize()?
        .parent()
        .ok_or_else(|| anyhow!("the program has no folder"))?
        .to_path_buf();
    if target != current {
        let arch = arch();
        let url = archive_url(&base, &target, arch);
        println!(
            "{}",
            tr!(
                "Téléchargement de {} …",
                "Downloading {} …",
                archive_name(&target, arch)
            )
        );
        let (bytes, sig) = runtime
            .block_on(async {
                let get = |u: String| {
                    let client = client.clone();
                    async move {
                        let r = client.get(&u).send().await?.error_for_status()?;
                        anyhow::Ok(r.bytes().await?.to_vec())
                    }
                };
                let bytes = get(url.clone()).await?;
                let sig = get(format!("{url}.sig")).await?;
                anyhow::Ok((bytes, String::from_utf8(sig)?))
            })
            .with_context(|| url.clone())?;
        verify(&bytes, &sig, &key()).map_err(|e| {
            anyhow!(tr!(
                "signature refusée ({e}) : l’archive ne vient pas de la release d’Escouade, rien n’est changé",
                "signature refused ({e}): the archive does not come from Escouade's release, nothing is changed"
            ))
        })?;
        println!("{}", tr!("Signature vérifiée.", "Signature checked."));
        let names = install(&bytes, &dir).map_err(|e| {
            anyhow!(tr!(
                "{e:#} : lance la mise à jour en tant que l’utilisateur à qui appartient {}",
                "{e:#}: run the update as the user who owns {}",
                dir.display()
            ))
        })?;
        println!(
            "{}",
            tr!(
                "{target} posé dans {} ({}) ; l’ancien est gardé en .old.",
                "{target} put in {} ({}); the old one is kept as .old.",
                dir.display(),
                names.join(", ")
            )
        );
    } else if running.as_ref().is_none_or(|s| s.version == target) {
        println!("{}", tr!("Déjà en {target}.", "Already at {target}."));
        return Ok(());
    }
    restart(data, asked.now, &target)
}

/// The running server stopped once at rest (or now), for its supervisor to start it again on `target`.
fn restart(data: &DataDir, now: bool, target: &str) -> Result<()> {
    let mut asked_now = now;
    let mut said = String::new();
    let interactive = std::io::stdin().is_terminal();
    loop {
        let current = status::read(data);
        match wait_of(current.as_ref(), status::alive) {
            Wait::NotRunning => {
                println!(
                    "{}",
                    tr!(
                        "Le serveur n’est pas lancé : il démarrera en {target}.",
                        "The server is not running: it will start at {target}."
                    )
                );
                return Ok(());
            }
            Wait::Busy(why) if !asked_now => {
                if said.is_empty() && interactive {
                    print!(
                        "{}",
                        tr!(
                            "Le serveur est occupé ({}). Attendre que ce soit fini (A, par défaut) ou redémarrer maintenant en coupant les tours (m) ? ",
                            "The server is busy ({}). Wait until it is done (W, the default) or restart now, cutting the turns (n)? ",
                            status::busy_text(&why)
                        )
                    );
                    let _ = std::io::stdout().flush();
                    let mut answer = String::new();
                    let _ = std::io::stdin().read_line(&mut answer);
                    if matches!(
                        answer.trim().to_lowercase().as_str(),
                        "m" | "n" | "now" | "maintenant"
                    ) {
                        asked_now = true;
                        continue;
                    }
                }
                if said != why {
                    println!(
                        "{}",
                        tr!(
                            "En attente : {}… (Ctrl+C arrête d’attendre ; « update » relancé reprend)",
                            "Waiting: {}… (Ctrl+C stops waiting; “update” run again picks up)",
                            status::busy_text(&why)
                        )
                    );
                    said = why;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
            Wait::Busy(_) | Wait::Now => {
                let Some(s) = current else { continue };
                stop(s.pid)?;
                println!(
                    "{}",
                    tr!(
                        "Serveur arrêté (processus {}).",
                        "Server stopped (process {}).",
                        s.pid
                    )
                );
                if s.supervisor != Some(Supervisor::Systemd) {
                    println!(
                        "{}",
                        tr!(
                            "Relance-le : il démarrera en {target}.",
                            "Start it again: it will run {target}."
                        )
                    );
                    return Ok(());
                }
                for _ in 0..90 {
                    std::thread::sleep(Duration::from_secs(1));
                    if let Some(again) = status::read(data) {
                        if again.pid != s.pid && status::alive(again.pid) {
                            println!(
                                "{}",
                                tr!(
                                    "Redémarré par systemd en {} (processus {}).",
                                    "Started again by systemd at {} (process {}).",
                                    again.version,
                                    again.pid
                                )
                            );
                            return Ok(());
                        }
                    }
                }
                bail!(tr!(
                    "systemd ne l’a pas relancé en 90 s : « journalctl -u escouade-server » dit pourquoi",
                    "systemd did not start it again in 90 s: “journalctl -u escouade-server” says why"
                ));
            }
        }
    }
}

/// SIGTERM to `pid`: the clean stop (the turns cut, the state written).
fn stop(pid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        // SAFETY: a signal to a process this user runs.
        if unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) } != 0 {
            return Err(std::io::Error::last_os_error()).context(format!("SIGTERM {pid}"));
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        bail!("SIGTERM {pid}: Unix only")
    }
}
