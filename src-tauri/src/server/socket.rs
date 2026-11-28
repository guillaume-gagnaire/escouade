//! `escouade-server --socket <chemin>` (ESC-706, BC-5, `docs/REMOTE.md` « La socket Unix »): the remote API on a Unix
//! socket in a folder that is its user's alone, in place of a port of the loopback. On a machine where other accounts
//! exist, a port of 127.0.0.1 is reached by all of them, and one of them may take it while the server restarts (an
//! update, a crash): the desktop client, back through its tunnel, would then give it its token. A socket in a folder
//! `0700` shuts both.
//!
//! - **Its folder** is made `0700` when missing; one that is there must be the user's and closed to the others (no bit
//!   of the group or the others), never a link.
//! - **Each folder above it** belongs to the user or to root, and no one else may change it (no write bit of the group
//!   or the others), but a sticky folder of root (`/tmp`): its entries are their owners' alone. The folders are read as
//!   the path says them and as they really are (`canonicalize`): a link on the way is judged by both.
//! - **A lock holds it** (`<socket>.lock`, `flock`): a socket whose lock is held is another server's (« un autre serveur
//!   écoute sur … »); a socket left by a server killed is dead (nothing answers it) and is replaced; one that answers
//!   without the lock is kept too. Anything else at that path is never removed.
//! - **The socket is `0600`**, and removed at the clean stop (when it is still the one bound: its inode).
//!
//! Not under Windows: its sockets are not those of `ssh -L`, and `--socket` is refused there (`server::parse`).

use std::path::Path;

/// The most bytes of a socket's path (`sun_path`, 108 under Linux, 104 under macOS, with its final zero).
pub const MAX_PATH: usize = 103;

/// Why a folder above the socket's is refused, from its owner, its mode and the user's id: none when it may hold the
/// way to the socket.
pub fn ancestor_refusal(owner: u32, mode: u32, me: u32) -> Option<String> {
    if owner != me && owner != 0 {
        return Some(tr!(
            "il appartient à un autre utilisateur ({owner})",
            "it belongs to another user ({owner})"
        ));
    }
    let open = mode & 0o022 != 0;
    let sticky = mode & 0o1000 != 0;
    if open && !(owner == 0 && sticky) {
        return Some(tr!(
            "d’autres que toi peuvent le modifier (mode {:o})",
            "others than you may change it (mode {:o})",
            mode & 0o7777
        ));
    }
    None
}

/// Why the socket's own folder is refused: none when it is the user's and closed to the others.
pub fn folder_refusal(owner: u32, mode: u32, me: u32) -> Option<String> {
    if owner != me {
        return Some(tr!(
            "il n’est pas à toi (propriétaire {owner})",
            "it is not yours (owner {owner})"
        ));
    }
    if mode & 0o077 != 0 {
        return Some(tr!(
            "il est ouvert à d’autres (mode {:o}) : chmod 700",
            "it is open to others (mode {:o}): chmod 700",
            mode & 0o7777
        ));
    }
    None
}

/// What a path given to `--socket` must be, whatever the system: absolute, short enough for a socket, a name of its own.
pub fn path_refusal(path: &Path) -> Option<String> {
    let shown = path.display();
    if !path.is_absolute() && !path.to_string_lossy().starts_with('/') {
        return Some(tr!(
            "--socket : « {shown} » n’est pas un chemin absolu",
            "--socket: “{shown}” is not an absolute path"
        ));
    }
    if path.file_name().is_none() || path.to_string_lossy().ends_with('/') {
        return Some(tr!(
            "--socket : « {shown} » ne nomme pas de fichier",
            "--socket: “{shown}” names no file"
        ));
    }
    let text = path.to_string_lossy();
    if text.chars().any(|c| c.is_control()) || text.contains(':') {
        // `ssh -L <port>:<socket>` reads its colons: a path with one could not be reached.
        return Some(tr!(
            "--socket : « {shown} » a un caractère que ssh -L ne lit pas (deux-points, caractère de contrôle)",
            "--socket: “{shown}” has a character ssh -L cannot read (a colon, a control character)"
        ));
    }
    if text.len() > MAX_PATH {
        return Some(tr!(
            "--socket : « {shown} » est trop long pour une socket ({} octets, {MAX_PATH} au plus)",
            "--socket: “{shown}” is too long for a socket ({} bytes, {MAX_PATH} at most)",
            text.len()
        ));
    }
    None
}

#[cfg(unix)]
pub use unix::{bind, Bound};

#[cfg(unix)]
mod unix {
    use super::{ancestor_refusal, folder_refusal, path_refusal};
    use anyhow::{anyhow, bail, Context, Result};
    use std::fs::{File, OpenOptions};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{
        DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt,
    };
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};

    /// The socket bound, its lock held: removed when this is dropped (the clean stop), if it is still the one bound.
    pub struct Bound {
        path: PathBuf,
        inode: u64,
        _lock: File,
    }

    impl std::fmt::Debug for Bound {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Bound").field("path", &self.path).finish()
        }
    }

    impl Bound {
        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for Bound {
        fn drop(&mut self) {
            if std::fs::symlink_metadata(&self.path).is_ok_and(|m| m.ino() == self.inode) {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }

    fn me() -> u32 {
        // SAFETY: geteuid cannot fail.
        unsafe { libc::geteuid() }
    }

    /// The folders above `dir` (itself left out), as the path says them and as they really are.
    fn above(dir: &Path) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = dir.ancestors().skip(1).map(Path::to_path_buf).collect();
        if let Ok(real) = std::fs::canonicalize(dir) {
            for a in real.ancestors().skip(1) {
                if !out.iter().any(|o| o == a) {
                    out.push(a.to_path_buf());
                }
            }
        }
        out.retain(|p| !p.as_os_str().is_empty());
        out
    }

    /// The folder of the socket, made `0700` when missing, then checked with each folder above it.
    fn folder(path: &Path) -> Result<PathBuf> {
        let dir = path
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .ok_or_else(|| anyhow!("--socket: {} has no folder", path.display()))?
            .to_path_buf();
        let me = me();
        // The folders above first: a folder made in one that others may change would be theirs to swap.
        for a in above(&dir) {
            let Ok(m) = std::fs::metadata(&a) else {
                continue;
            };
            if let Some(why) = ancestor_refusal(m.uid(), m.mode(), me) {
                bail!(tr!(
                    "--socket : le dossier {} est refusé : {why}",
                    "--socket: the folder {} is refused: {why}",
                    a.display()
                ));
            }
        }
        match std::fs::symlink_metadata(&dir) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(&dir)
                    .with_context(|| format!("--socket: {}", dir.display()))?;
                // A umask cannot have opened it, but a folder made between two checks could: read it again below.
            }
            Err(e) => return Err(anyhow!("--socket: {}: {e}", dir.display())),
            Ok(_) => {}
        }
        let m = std::fs::symlink_metadata(&dir)
            .with_context(|| format!("--socket: {}", dir.display()))?;
        if m.file_type().is_symlink() || !m.is_dir() {
            bail!(tr!(
                "--socket : {} n’est pas un dossier (un lien n’est pas pris)",
                "--socket: {} is not a folder (a link is not taken)",
                dir.display()
            ));
        }
        if let Some(why) = folder_refusal(m.uid(), m.mode(), me) {
            bail!(tr!(
                "--socket : le dossier {} est refusé : {why}",
                "--socket: the folder {} is refused: {why}",
                dir.display()
            ));
        }
        // The folders made by `create` are checked too (the path's own, above it).
        for a in above(&dir) {
            let Ok(m) = std::fs::metadata(&a) else {
                continue;
            };
            if let Some(why) = ancestor_refusal(m.uid(), m.mode(), me) {
                bail!(tr!(
                    "--socket : le dossier {} est refusé : {why}",
                    "--socket: the folder {} is refused: {why}",
                    a.display()
                ));
            }
        }
        Ok(dir)
    }

    fn taken(path: &Path) -> anyhow::Error {
        anyhow!(tr!(
            "un autre serveur écoute sur {} : un seul à la fois",
            "another server listens on {}: one at a time",
            path.display()
        ))
    }

    /// The socket `path` bound for the API: its folder and those above checked, its lock taken, a dead socket replaced,
    /// a live one never; `0600`.
    pub fn bind(path: &Path) -> Result<(UnixListener, Bound)> {
        if let Some(why) = path_refusal(path) {
            bail!(why);
        }
        folder(path)?;
        let mut lock_name = path.file_name().unwrap_or_default().to_os_string();
        lock_name.push(".lock");
        let lock_path = path.with_file_name(lock_name);
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&lock_path)
            .with_context(|| format!("--socket: {}", lock_path.display()))?;
        // SAFETY: the descriptor is the open file's, which lives as long as the lock.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::WouldBlock {
                return Err(taken(path));
            }
            return Err(anyhow!("--socket: {}: {e}", lock_path.display()));
        }
        match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_socket() => {
                // A socket the lock no longer holds: dead when nothing answers it, else another program's.
                if UnixStream::connect(path).is_ok() {
                    return Err(taken(path));
                }
                std::fs::remove_file(path)
                    .with_context(|| format!("--socket: {}", path.display()))?;
                log::info!(
                    "escouade-server: a dead socket replaced at {}",
                    path.display()
                );
            }
            Ok(_) => bail!(tr!(
                "--socket : {} existe et n’est pas une socket : rien n’est retiré à ta place",
                "--socket: {} exists and is not a socket: nothing is removed in your place",
                path.display()
            )),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(anyhow!("--socket: {}: {e}", path.display())),
        }
        let listener =
            UnixListener::bind(path).with_context(|| format!("--socket: {}", path.display()))?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("--socket: {}", path.display()))?;
        let inode = std::fs::symlink_metadata(path)?.ino();
        Ok((
            listener,
            Bound {
                path: path.to_path_buf(),
                inode,
                _lock: lock,
            },
        ))
    }
}
