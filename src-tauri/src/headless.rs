//! The core in a process without a window (ESC-451, `docs/REMOTE.md`): what the tests run, then
//! `escouade-server`. Its host publishes what the desktop would show (`host::HeadlessHost`), and
//! the data folder is held by a lock on a file in place of `single-instance`, which needs a desktop
//! (D-Bus under Linux).

use crate::core::Core;
use crate::host::HeadlessHost;
use crate::paths::DataDir;
use crate::remoteapi::server::RemoteApi;
use crate::remoteapi::Services;
use anyhow::{anyhow, Result};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The name of the lock's file, in the data folder.
pub const LOCK_FILE: &str = "instance.lock";

/// The data folder held by one process: one core at a time writes in it. The system lets go of the
/// lock with its file, when the process ends however it ends: a crash leaves nothing to clean.
/// Under Unix, `flock` on the file; under Windows, the file opened shared with no one (Rust 1.88,
/// the crate's, has no `File::try_lock` yet).
#[derive(Debug)]
pub struct InstanceLock {
    _file: File,
    path: PathBuf,
}

impl InstanceLock {
    /// The lock of the data folder `root`; an error when another process holds it.
    pub fn acquire(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root)?;
        let path = root.join(LOCK_FILE);
        let held = || {
            anyhow!(tr!(
                "Escouade tourne déjà sur ce dossier de données ({}) : un seul à la fois.",
                "Escouade already runs on this data folder ({}): one at a time.",
                root.display()
            ))
        };
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).write(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        let file = match options.open(&path) {
            Ok(file) => file,
            // ERROR_SHARING_VIOLATION: another process has it open.
            #[cfg(windows)]
            Err(e) if e.raw_os_error() == Some(32) => return Err(held()),
            Err(e) => return Err(e.into()),
        };
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // SAFETY: the descriptor is the open file's, which lives as long as the lock.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                let e = std::io::Error::last_os_error();
                return Err(if e.kind() == std::io::ErrorKind::WouldBlock {
                    held()
                } else {
                    e.into()
                });
            }
        }
        Ok(Self { _file: file, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// A file written by two processes of the same data folder (the server, and `escouade-server pair` or `devices`
/// beside it: ESC-514) read, changed and written again by one at a time: the lock of `<file>.lock`, held while `f`
/// runs. Under Unix, `flock` (waited for); under Windows, the lock file opened shared with no one, tried again for 5 s.
pub fn with_file_lock<T>(file: &Path, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let path = file.with_extension("lock");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).write(true);
    #[cfg(windows)]
    let _held = {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match options.open(&path) {
                Ok(file) => break file,
                Err(e) if e.raw_os_error() == Some(32) && std::time::Instant::now() < until => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(e) => return Err(anyhow!("{}: {e}", path.display())),
            }
        }
    };
    #[cfg(not(windows))]
    let _held = {
        let file = options.open(&path)?;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // SAFETY: the descriptor is the open file's, which lives until `f` returns.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        file
    };
    f()
}

/// The core started without a window, its host and the lock of its data folder.
pub struct Headless {
    pub core: Arc<Core>,
    pub host: Arc<HeadlessHost>,
    /// The remote API, once served (`serve_remote`).
    remote: parking_lot::Mutex<Option<Arc<RemoteApi>>>,
    /// The teams of the team server, once opened (`open_teams`, `escouade-server`): served to the remote windows.
    teams: parking_lot::Mutex<Option<Arc<crate::teams::Teams>>>,
    _lock: InstanceLock,
}

impl Headless {
    /// The core on `data`, started as the desktop app starts it (its board, its git, its saves),
    /// once the data folder is this process's. On a runtime of Tokio.
    pub fn start(data: DataDir) -> Result<Self> {
        Self::start_with(data, Arc::new(HeadlessHost::default()))
    }

    /// The same, on `host` (`escouade-server`'s: its resources, its secrets in a file).
    pub fn start_with(data: DataDir, host: Arc<HeadlessHost>) -> Result<Self> {
        let lock = InstanceLock::acquire(data.root())?;
        Ok(Self::start_locked(lock, data, host))
    }

    /// The same, the data folder's `lock` taken already (`escouade-server`, which does what needs the folder first).
    pub fn start_locked(lock: InstanceLock, data: DataDir, host: Arc<HeadlessHost>) -> Self {
        let (core, git_rx) = Core::load(host.clone(), data);
        core.start(git_rx);
        Self {
            core,
            host,
            remote: parking_lot::Mutex::new(None),
            teams: parking_lot::Mutex::new(None),
            _lock: lock,
        }
    }

    /// The teams of the team server on this core (OE-1: a machine of the team in server mode is a device of its own),
    /// as the desktop opens them (`teams::open_for`); before `serve_remote`, which serves them to the remote windows.
    pub fn open_teams(&self) -> Arc<crate::teams::Teams> {
        let mut held = self.teams.lock();
        held.get_or_insert_with(|| crate::teams::open_for(&self.core))
            .clone()
    }

    /// The remote API of this core served on 127.0.0.1:`port` (0: a free port), off until then (ESC-478): what
    /// `escouade-server` will turn on. Refused when the policies of the device forbid it.
    pub async fn serve_remote(&self, port: u16) -> Result<Arc<RemoteApi>> {
        self.serve_remote_with(port, crate::remoteapi::server::Limits::default())
            .await
    }

    /// The same, within `limits` (the tests' small ones).
    pub async fn serve_remote_with(
        &self,
        port: u16,
        limits: crate::remoteapi::server::Limits,
    ) -> Result<Arc<RemoteApi>> {
        self.serve_remote_on(crate::remoteapi::server::Listen::Port(port), limits)
            .await
    }

    /// The same, where `listen` says: a port of 127.0.0.1, or a socket bound already (`escouade-server --listen`,
    /// another interface asked for in so many words).
    pub async fn serve_remote_on(
        &self,
        listen: crate::remoteapi::server::Listen,
        limits: crate::remoteapi::server::Limits,
    ) -> Result<Arc<RemoteApi>> {
        let mut services = Services::new(self.core.clone());
        if let Some(teams) = self.teams.lock().clone() {
            services = services.with(teams);
        }
        let services = Arc::new(services);
        let api = RemoteApi::start_with(services, listen, limits).await?;
        *self.remote.lock() = Some(api.clone());
        Ok(api)
    }

    /// The core stopped as when the app quits: its remote API, its agents, terminals and plugins, its state saved.
    pub fn stop(self) {
        if let Some(api) = self.remote.lock().take() {
            api.stop();
        }
        self.core.shutdown();
    }
}
