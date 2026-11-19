//! Where the secrets are kept (the app's keychain implements the store): what a plugin reads by `secrets/get`, and the
//! accounts' tokens.

#[cfg(any(test, feature = "testing"))]
use anyhow::bail;
use anyhow::Result;

/// Where the secrets are kept, one entry per integration: the system's keychain, or memory in
/// tests (they never touch the machine's).
pub trait SecretStore: Send + Sync {
    /// The secret kept under `name`, if there is one.
    fn get(&self, name: &str) -> Result<Option<String>>;
    /// Keeps `secret` under `name`, in place of the one there.
    fn set(&self, name: &str, secret: &str) -> Result<()>;
    /// Forgets the secret kept under `name` (none there is fine).
    fn delete(&self, name: &str) -> Result<()>;
    /// Whether there is a keychain at all: false where the system has none the app knows of (Linux, WSL), whose
    /// `set` always fails with `NoKeychain` (ESC-499).
    fn present(&self) -> bool {
        true
    }
}

/// The error of a store where the system has no keychain the app knows of (Linux, WSL): nothing is kept there. Told
/// apart from a keychain that refuses (locked, out of reach) by `is_no_keychain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoKeychain;

impl std::fmt::Display for NoKeychain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("no system keychain here")
    }
}

impl std::error::Error for NoKeychain {}

/// Whether `e` says the system has no keychain (`NoKeychain`), rather than one that refused.
pub fn is_no_keychain(e: &anyhow::Error) -> bool {
    e.chain().any(|c| c.downcast_ref::<NoKeychain>().is_some())
}

/// The most a secret may hold, in bytes (UTF-8): 64 KiB (ESC-527). A configuration with its certificates is 4 to 8 KB.
pub const SECRET_MAX: usize = 64 * 1024;

/// The most an entry of the keychain holds before the secret is cut into pieces, in UTF-16 units: Windows refuses more
/// than 1,280 (`CRED_MAX_CREDENTIAL_BLOB_SIZE`, 2,560 bytes), and the pieces stay under it.
pub const PIECE: usize = 1200;

/// The most pieces a header may name: 64 KiB in pieces of 1,200 units, with room.
const PIECES_MAX: usize = 64;

/// What starts the entry of a secret kept in pieces: `escouade-pieces:v1:<slot>:<count>:<sha256>`.
const HEADER: &str = "escouade-pieces:v1:";

/// The error of a secret longer than `SECRET_MAX`: refused before anything is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLong;

impl std::fmt::Display for TooLong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "a secret holds {} KiB at most", SECRET_MAX / 1024)
    }
}

impl std::error::Error for TooLong {}

/// Whether `e` says the secret is longer than the store takes (`TooLong`).
pub fn is_too_long(e: &anyhow::Error) -> bool {
    e.chain().any(|c| c.downcast_ref::<TooLong>().is_some())
}

/// Whether `name` is the name of a piece (`…#a1`, `…#b12`): never a secret's own name, so that no key can be taken for
/// a piece of another one.
pub fn is_piece_name(name: &str) -> bool {
    name.rsplit_once('#').is_some_and(|(_, tail)| {
        let mut chars = tail.chars();
        matches!(chars.next(), Some('a' | 'b'))
            && !chars.as_str().is_empty()
            && chars.all(|c| c.is_ascii_digit())
    })
}

/// The name of the piece `i` (from 1) of the secret `name`, in `slot`.
fn piece_name(name: &str, slot: char, i: usize) -> String {
    format!("{name}#{slot}{i}")
}

/// The SHA-256 of `text`, in hexadecimal.
fn sha256(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `text` cut into pieces of `PIECE` UTF-16 units at most, never inside a character.
fn cut(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let (mut start, mut units) = (0, 0);
    for (at, c) in text.char_indices() {
        if units + c.len_utf16() > PIECE {
            pieces.push(&text[start..at]);
            (start, units) = (at, 0);
        }
        units += c.len_utf16();
    }
    pieces.push(&text[start..]);
    pieces
}

/// A header read: the pieces' slot, their count, the whole's SHA-256.
struct Header {
    slot: char,
    count: usize,
    sha256: String,
}

impl Header {
    /// The header `entry` holds; `None` for a secret kept whole (any entry that does not start with `HEADER`), an error
    /// for one that starts with it and is not a header.
    fn of(entry: &str) -> Result<Option<Self>> {
        let Some(rest) = entry.strip_prefix(HEADER) else {
            return Ok(None);
        };
        let mut parts = rest.split(':');
        let (Some(slot), Some(count), Some(sha256), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            anyhow::bail!("the header of a secret in pieces is unreadable");
        };
        let slot = match slot {
            "a" => 'a',
            "b" => 'b',
            _ => anyhow::bail!("the header of a secret in pieces is unreadable"),
        };
        let count: usize = count
            .parse()
            .ok()
            .filter(|n| (1..=PIECES_MAX).contains(n))
            .ok_or_else(|| anyhow::anyhow!("the header of a secret in pieces is unreadable"))?;
        if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            anyhow::bail!("the header of a secret in pieces is unreadable");
        }
        Ok(Some(Self {
            slot,
            count,
            sha256: sha256.to_string(),
        }))
    }

    fn text(&self) -> String {
        format!("{HEADER}{}:{}:{}", self.slot, self.count, self.sha256)
    }
}

/// A store whose long secrets are kept in pieces (ESC-527): Windows' keychain refuses an entry of more than 1,280
/// characters, a configuration with its certificates is 4 to 8 KB. The same format on every system, so that it reads
/// what another wrote and a test covers it everywhere.
///
/// - **A short secret** (`PIECE` UTF-16 units at most) is one entry under its name, as before: what is already kept reads
///   as it was.
/// - **A long one** is cut into pieces of `PIECE` units under `<name>#a1`, `#a2`… (or `#b1`, `#b2`…: two slots, so that
///   the new pieces never take the place of the old ones before the header turns), and the entry `<name>` holds the
///   header, `escouade-pieces:v1:<slot>:<count>:<sha256 of the whole>`. A secret that starts like a header is kept in
///   pieces too, so that a header is never a secret.
/// - **Reading** glues the pieces back and checks the SHA-256: a piece missing, one too many or a wrong SHA-256 is an
///   error (the keychain's, `-32002` for a plugin), never a secret cut short.
/// - **Writing** is whole or nothing: the new pieces in the slot the header does not name, those beyond them in that slot
///   out, then the header turned, then the old pieces out. A piece that fails takes out what was written, and the old
///   secret still reads; a write cut at any step leaves the old one readable until the header turns, the new one after.
/// - **Forgetting** takes out the pieces of both slots (the header's count, then while there are more), then the header.
/// - 64 KiB at most (`TooLong`); a piece's name is never a secret's (`is_piece_name`).
pub struct Chunked {
    inner: std::sync::Arc<dyn SecretStore>,
}

impl Chunked {
    pub fn new(inner: std::sync::Arc<dyn SecretStore>) -> Self {
        Self { inner }
    }

    fn own_name(name: &str) -> Result<()> {
        if is_piece_name(name) {
            anyhow::bail!("{name:?} is the name of a piece of a secret, never of a secret");
        }
        Ok(())
    }

    /// The pieces of `slot` from `from` on, out while there are some (a write cut short may have left some beyond the
    /// header's count), the last first.
    fn clear_from(&self, name: &str, slot: char, from: usize) -> Result<()> {
        let mut end = from;
        while end <= PIECES_MAX + 1 && self.inner.get(&piece_name(name, slot, end))?.is_some() {
            end += 1;
        }
        for i in (from..end).rev() {
            self.inner.delete(&piece_name(name, slot, i))?;
        }
        Ok(())
    }

    /// The pieces `1..=count` of `slot` out, then those beyond them. The last first: cut off in between, what stays is
    /// `1..k`, which the next write or « Oublier » finds again (`clear_from` stops at the first gap).
    fn clear(&self, name: &str, slot: char, count: usize) -> Result<()> {
        self.clear_from(name, slot, count + 1)?;
        for i in (1..=count).rev() {
            self.inner.delete(&piece_name(name, slot, i))?;
        }
        Ok(())
    }

    /// The pieces of every slot but `keep` out: those `old` names by its count, the others while there are some.
    fn clear_slots(&self, name: &str, keep: Option<char>, old: Option<&Header>) -> Result<()> {
        for slot in ['a', 'b'] {
            if Some(slot) == keep {
                continue;
            }
            match old {
                Some(h) if h.slot == slot => self.clear(name, slot, h.count)?,
                _ => self.clear_from(name, slot, 1)?,
            }
        }
        Ok(())
    }

    /// The pieces just written, out again (a write that failed): best effort, the header was not turned.
    fn undo(&self, name: &str, slot: char, written: usize) {
        for i in (1..=written).rev() {
            let _ = self.inner.delete(&piece_name(name, slot, i));
        }
    }
}

impl SecretStore for Chunked {
    fn get(&self, name: &str) -> Result<Option<String>> {
        Self::own_name(name)?;
        let Some(entry) = self.inner.get(name)? else {
            return Ok(None);
        };
        let Some(header) = Header::of(&entry)? else {
            return Ok(Some(entry));
        };
        let mut whole = String::new();
        for i in 1..=header.count {
            match self.inner.get(&piece_name(name, header.slot, i))? {
                Some(piece) => whole.push_str(&piece),
                None => anyhow::bail!("a piece of the secret is missing ({i} of {})", header.count),
            }
        }
        if self
            .inner
            .get(&piece_name(name, header.slot, header.count + 1))?
            .is_some()
        {
            anyhow::bail!("the secret has more pieces than its header says");
        }
        if sha256(&whole) != header.sha256 {
            anyhow::bail!("the pieces of the secret do not give its SHA-256");
        }
        Ok(Some(whole))
    }

    fn set(&self, name: &str, secret: &str) -> Result<()> {
        Self::own_name(name)?;
        if secret.len() > SECRET_MAX {
            return Err(TooLong.into());
        }
        let long = secret.encode_utf16().count() > PIECE || secret.starts_with(HEADER);
        // What is kept now: its pieces go once the new secret is in place. A keychain that does not give it writes
        // nothing (the old pieces would be lost track of).
        let old = match self.inner.get(name)? {
            Some(entry) => Header::of(&entry).unwrap_or(None),
            None => None,
        };
        let mut slot = None;
        if !long {
            self.inner.set(name, secret)?;
        } else {
            let new = match &old {
                Some(h) if h.slot == 'a' => 'b',
                _ => 'a',
            };
            slot = Some(new);
            let slot = new;
            let pieces = cut(secret);
            for (i, piece) in pieces.iter().enumerate() {
                if let Err(e) = self.inner.set(&piece_name(name, slot, i + 1), piece) {
                    self.undo(name, slot, i);
                    return Err(e);
                }
            }
            let header = Header {
                slot,
                count: pieces.len(),
                sha256: sha256(secret),
            };
            if let Err(e) = self
                .clear_from(name, slot, pieces.len() + 1)
                .and_then(|()| self.inner.set(name, &header.text()))
            {
                self.undo(name, slot, pieces.len());
                return Err(e);
            }
        }
        // The header turned: the old pieces are no one's now, nor those a write cut short left in the other slot. Left
        // there (a keychain that refuses now), they go at the next write or at « Oublier ».
        if let Err(e) = self.clear_slots(name, slot, old.as_ref()) {
            log::warn!("the old pieces of a secret stay in the keychain: {e:#}");
        }
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<()> {
        Self::own_name(name)?;
        let old = match self.inner.get(name) {
            Ok(Some(entry)) => Header::of(&entry).ok().flatten(),
            _ => None,
        };
        // The pieces first: cut in between, the header says pieces are missing (an error), never a secret cut short.
        self.clear_slots(name, None, old.as_ref())?;
        self.inner.delete(name)
    }

    fn present(&self) -> bool {
        self.inner.present()
    }
}

/// The secrets in memory (tests).
#[cfg(any(test, feature = "testing"))]
#[derive(Default)]
pub struct MemoryStore {
    entries: parking_lot::Mutex<std::collections::BTreeMap<String, String>>,
    /// Every call fails, as with a keychain out of reach.
    pub refuse: std::sync::atomic::AtomicBool,
    /// Says it kept a secret, and keeps nothing.
    pub lose: std::sync::atomic::AtomicBool,
    /// No keychain at all, as on Linux: nothing given, nothing kept (`NoKeychain`), nothing to forget.
    pub absent: std::sync::atomic::AtomicBool,
}

#[cfg(any(test, feature = "testing"))]
impl MemoryStore {
    /// What it holds under `name`.
    pub fn entry(&self, name: &str) -> Option<String> {
        self.entries.lock().get(name).cloned()
    }

    /// Everything it holds, by name.
    pub fn all(&self) -> std::collections::BTreeMap<String, String> {
        self.entries.lock().clone()
    }

    fn reached(&self) -> Result<()> {
        if self.refuse.load(std::sync::atomic::Ordering::SeqCst) {
            bail!("keychain out of reach");
        }
        Ok(())
    }
}

#[cfg(any(test, feature = "testing"))]
impl SecretStore for MemoryStore {
    fn get(&self, name: &str) -> Result<Option<String>> {
        self.reached()?;
        if !self.present() {
            return Ok(None);
        }
        Ok(self.entry(name))
    }

    fn set(&self, name: &str, secret: &str) -> Result<()> {
        self.reached()?;
        if !self.present() {
            return Err(NoKeychain.into());
        }
        if !self.lose.load(std::sync::atomic::Ordering::SeqCst) {
            self.entries.lock().insert(name.into(), secret.into());
        }
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<()> {
        self.reached()?;
        self.entries.lock().remove(name);
        Ok(())
    }

    fn present(&self) -> bool {
        !self.absent.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
#[path = "secrets_tests.rs"]
mod tests;
