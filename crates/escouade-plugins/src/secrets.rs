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
        Ok(self.entry(name))
    }

    fn set(&self, name: &str, secret: &str) -> Result<()> {
        self.reached()?;
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
}
