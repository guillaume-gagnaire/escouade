//! What the connector keeps (§ 4.5, § 4.9): its private key and the server's public key, nothing else.
//!
//! In its state folder (`--state`, `$STATE_DIRECTORY` of systemd, else `/var/lib/escouade-connector` or
//! `%ProgramData%\escouade-connector`):
//!
//! - `connector.key`: the private key. Under Unix, 64 hexadecimal digits in a file of mode `0600` in a folder of
//!   mode `0700` (readable by the service's account alone). Under Windows, sealed by DPAPI for the account that ran
//!   `pair` (`dpapi:` then the blob in hexadecimal): only that account opens it.
//! - `paired.toml`: the server's address, its public key, and the connector's own public key (to show it).
//!
//! `unpair` writes zeros over the key before it removes both files.

use std::io::Write;
use std::path::{Path, PathBuf};

use escouade_connector_proto::{Keypair, PublicKey};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

pub const KEY_FILE: &str = "connector.key";
pub const PAIRED_FILE: &str = "paired.toml";

/// The pairing, as `paired.toml` keeps it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pairing {
    /// `wss://…/connector`.
    pub server: String,
    pub server_key: PublicKey,
    pub connector_key: PublicKey,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PairedFile {
    server: String,
    server_key: String,
    connector_key: String,
}

#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
#[error("{path}: {why}")]
pub struct StateError {
    pub path: String,
    pub why: String,
}

fn err(path: &Path, why: impl ToString) -> StateError {
    StateError {
        path: path.display().to_string(),
        why: why.to_string(),
    }
}

/// The state folder by default: systemd's `$STATE_DIRECTORY`, else the system's place.
pub fn default_state_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("STATE_DIRECTORY").filter(|d| !d.is_empty()) {
        // systemd may give several, separated by colons: the first.
        let first = dir
            .to_string_lossy()
            .split(':')
            .next()
            .unwrap_or("")
            .to_string();
        if !first.is_empty() {
            return PathBuf::from(first);
        }
    }
    if cfg!(windows) {
        let base = std::env::var_os("ProgramData").unwrap_or_else(|| "C:\\ProgramData".into());
        PathBuf::from(base).join("escouade-connector")
    } else {
        PathBuf::from("/var/lib/escouade-connector")
    }
}

/// The state folder.
#[derive(Clone, Debug)]
pub struct State {
    pub dir: PathBuf,
}

impl State {
    pub fn new(dir: impl Into<PathBuf>) -> State {
        State { dir: dir.into() }
    }

    fn key_path(&self) -> PathBuf {
        self.dir.join(KEY_FILE)
    }

    fn paired_path(&self) -> PathBuf {
        self.dir.join(PAIRED_FILE)
    }

    /// The bytes of both files, to tell a change while the service runs (`pair` or `unpair` beside it).
    pub fn stamp(&self) -> Vec<u8> {
        let mut out = std::fs::read(self.paired_path()).unwrap_or_default();
        out.push(0);
        out.extend(std::fs::read(self.key_path()).unwrap_or_default());
        out
    }

    /// The pairing and the keys, `None` when not paired.
    pub fn load(&self) -> Result<Option<(Pairing, Keypair)>, StateError> {
        let paired_path = self.paired_path();
        let key_path = self.key_path();
        if !paired_path.exists() && !key_path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&paired_path).map_err(|e| err(&paired_path, e))?;
        let file: PairedFile = toml::from_str(&text).map_err(|e| err(&paired_path, e.message()))?;
        let server_key = PublicKey::from_hex(&file.server_key)
            .ok_or_else(|| err(&paired_path, "server_key: not a public key"))?;
        let connector_key = PublicKey::from_hex(&file.connector_key)
            .ok_or_else(|| err(&paired_path, "connector_key: not a public key"))?;
        let sealed =
            Zeroizing::new(std::fs::read_to_string(&key_path).map_err(|e| err(&key_path, e))?);
        let private = unseal(sealed.trim()).map_err(|why| err(&key_path, why))?;
        let keys = Keypair::from_private(*private);
        if keys.public() != connector_key {
            return Err(err(
                &key_path,
                "the private key is not the one paired.toml says (pair again)",
            ));
        }
        Ok(Some((
            Pairing {
                server: file.server,
                server_key,
                connector_key,
            },
            keys,
        )))
    }

    /// Keeps a new pairing: the key first, then `paired.toml`.
    pub fn save(&self, pairing: &Pairing, keys: &Keypair) -> Result<(), StateError> {
        make_private_dir(&self.dir)?;
        let sealed =
            Zeroizing::new(seal(keys.private_bytes()).map_err(|why| err(&self.key_path(), why))?);
        write_private(&self.key_path(), sealed.as_bytes())?;
        let file = PairedFile {
            server: pairing.server.clone(),
            server_key: pairing.server_key.to_hex(),
            connector_key: pairing.connector_key.to_hex(),
        };
        let text = format!(
            "# Written by `escouade-connector pair`; removed by `escouade-connector unpair`.\n{}",
            toml::to_string(&file).map_err(|e| err(&self.paired_path(), e))?
        );
        write_private(&self.paired_path(), text.as_bytes())
    }

    /// Forgets the pairing: zeros over the key, then both files removed. `false` when there was none.
    pub fn remove(&self) -> Result<bool, StateError> {
        let key_path = self.key_path();
        let paired_path = self.paired_path();
        let had = key_path.exists() || paired_path.exists();
        if let Ok(meta) = std::fs::metadata(&key_path) {
            if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(&key_path) {
                let _ = f.write_all(&vec![0u8; meta.len() as usize]);
                let _ = f.sync_all();
            }
            std::fs::remove_file(&key_path).map_err(|e| err(&key_path, e))?;
        }
        if paired_path.exists() {
            std::fs::remove_file(&paired_path).map_err(|e| err(&paired_path, e))?;
        }
        Ok(had)
    }
}

fn make_private_dir(dir: &Path) -> Result<(), StateError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        if !dir.exists() {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(dir)
                .map_err(|e| err(dir, e))?;
        }
        let mode = std::fs::metadata(dir)
            .map_err(|e| err(dir, e))?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| err(dir, e))?;
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir).map_err(|e| err(dir, e))
    }
}

/// Writes `bytes` to `path` through a file beside it (mode `0600` under Unix), renamed over it.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), StateError> {
    let tmp = path.with_extension("tmp");
    let _ = std::fs::remove_file(&tmp);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(&tmp).map_err(|e| err(&tmp, e))?;
    f.write_all(bytes).map_err(|e| err(&tmp, e))?;
    f.sync_all().map_err(|e| err(&tmp, e))?;
    drop(f);
    std::fs::rename(&tmp, path).map_err(|e| err(path, e))
}

#[cfg(not(windows))]
fn seal(private: &[u8; 32]) -> Result<String, String> {
    Ok(hex::encode(private))
}

#[cfg(not(windows))]
fn unseal(text: &str) -> Result<Zeroizing<[u8; 32]>, String> {
    let mut out = Zeroizing::new([0u8; 32]);
    hex::decode_to_slice(text, &mut out[..]).map_err(|_| "not a private key".to_string())?;
    Ok(out)
}

#[cfg(windows)]
fn seal(private: &[u8; 32]) -> Result<String, String> {
    let blob = dpapi::protect(private).ok_or("DPAPI did not seal the key")?;
    Ok(format!("dpapi:{}", hex::encode(blob)))
}

#[cfg(windows)]
fn unseal(text: &str) -> Result<Zeroizing<[u8; 32]>, String> {
    let blob = text
        .strip_prefix("dpapi:")
        .and_then(|h| hex::decode(h).ok())
        .ok_or("not a key sealed by DPAPI")?;
    let plain = Zeroizing::new(
        dpapi::unprotect(&blob)
            .ok_or("DPAPI did not open the key: it opens for the account that ran `pair` alone")?,
    );
    let mut out = Zeroizing::new([0u8; 32]);
    if plain.len() != 32 {
        return Err("not a private key".into());
    }
    out.copy_from_slice(&plain);
    Ok(out)
}

#[cfg(windows)]
mod dpapi {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        // SAFETY: DPAPI gave `cbData` bytes at `pbData`, freed by LocalFree once copied.
        unsafe {
            let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            std::ptr::write_bytes(out.pbData, 0, out.cbData as usize);
            LocalFree(out.pbData as _);
            v
        }
    }

    pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        };
        // SAFETY: the blobs point at memory alive for the call; DPAPI writes `out`.
        let ok = unsafe {
            CryptProtectData(
                &input,
                null(),
                null(),
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        };
        (ok != 0).then(|| take(out))
    }

    pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        };
        // SAFETY: as in `protect`.
        let ok = unsafe {
            CryptUnprotectData(
                &input,
                null_mut(),
                null(),
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        };
        (ok != 0).then(|| take(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pairing_is_kept_and_forgotten() {
        let dir = tempfile::tempdir().unwrap();
        let state = State::new(dir.path().join("state"));
        assert!(state.load().unwrap().is_none());
        let keys = Keypair::generate();
        let server = Keypair::generate().public();
        let pairing = Pairing {
            server: "wss://escouade.example.com:8443/connector".into(),
            server_key: server,
            connector_key: keys.public(),
        };
        state.save(&pairing, &keys).unwrap();
        let (back, back_keys) = state.load().unwrap().unwrap();
        assert_eq!(back, pairing);
        assert_eq!(back_keys.public(), keys.public());
        // The private key is not written in clear where the system can seal it.
        let sealed = std::fs::read_to_string(state.dir.join(KEY_FILE)).unwrap();
        if cfg!(windows) {
            assert!(sealed.starts_with("dpapi:"));
            assert!(!sealed.contains(&hex::encode(keys.private_bytes())));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(&state.dir.join(KEY_FILE)), 0o600);
            assert_eq!(mode(&state.dir), 0o700);
        }
        let paired = std::fs::read_to_string(state.dir.join(PAIRED_FILE)).unwrap();
        assert!(!paired.contains(&hex::encode(keys.private_bytes())));
        assert!(state.remove().unwrap());
        assert!(state.load().unwrap().is_none());
        assert!(!state.remove().unwrap());
    }

    #[test]
    fn a_key_that_is_not_the_pairings_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let state = State::new(dir.path());
        let keys = Keypair::generate();
        let pairing = Pairing {
            server: "wss://x/connector".into(),
            server_key: Keypair::generate().public(),
            connector_key: Keypair::generate().public(),
        };
        state.save(&pairing, &keys).unwrap();
        let e = state.load().unwrap_err();
        assert!(e.why.contains("pair again"), "{e}");
    }
}
