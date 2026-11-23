//! The keychain's entries of the sandboxed runs (end-to-end tests, demos: `ESCOUADE_DATA_DIR`), each run's apart (ESC-584).
//!
//! Every run writes its entries under a name of its own, `<name>.run-<tag>.dev.gagnaire.escouade.sandbox`, its tag drawn
//! from its data folder, the folder named in their comment. So its end takes back what it wrote (`clean`), and nothing of a
//! run of another worktree; and the start of a run takes back what dead runs left (`sweep`): their folder gone, or older
//! than a day. Only an entry whose name ends with `.escouade.sandbox` and carries a run's tag is ever touched: never the
//! user's, nor one a run wrote before ESC-584 (the user's to clear, a-18).
//!
//! Windows only: the end-to-end tests run there (WebView2). Elsewhere a sandboxed run keeps its entries under the shared
//! service, as before.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// What ends every name this module may touch.
pub const SUFFIX: &str = ".escouade.sandbox";

/// What starts the comment of a run's entry, before its data folder.
const MARK: &str = "escouade-sandbox-run:";

/// How old an entry of a run whose folder is still there may be before it counts as dead.
pub const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// The longest comment Windows keeps (`CRED_MAX_STRING_LENGTH`): a longer folder is left out of it, its entries then swept by
/// their age only.
const MAX_COMMENT: usize = 256;

/// A sandboxed run: its data folder, and the tag drawn from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    tag: String,
    dir: String,
}

impl Run {
    /// The run on the data folder `dir`: the same folder, the same run (a restart finds its secrets again).
    pub fn of(dir: &Path) -> Self {
        let dir = normalized(dir);
        Self {
            tag: format!("{:016x}", fnv1a(dir.as_bytes())),
            dir,
        }
    }

    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The keychain's name of the entry `name` of this run, under `service` (the sandbox's).
    pub fn target(&self, service: &str, name: &str) -> String {
        format!("{name}.run-{}.{service}", self.tag)
    }

    /// The comment of its entries: its folder, unless too long for the keychain.
    pub fn comment(&self) -> String {
        let full = format!("{MARK}{}", self.dir);
        if full.encode_utf16().count() <= MAX_COMMENT {
            full
        } else {
            MARK.to_string()
        }
    }
}

/// The folder as a run names it: absolute as given, its separators and (on Windows) its case made one.
fn normalized(dir: &Path) -> String {
    let s = dir.to_string_lossy();
    #[cfg(windows)]
    let s = s.replace('/', "\\").to_lowercase();
    s.trim_end_matches(['\\', '/']).to_string()
}

/// FNV-1a, 64 bits: a tag that stays the same from one build to the next.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The run's tag of the keychain's name `target`, when it is one of a run's: it ends with `.escouade.sandbox` and carries
/// `.run-<16 hex digits>.` before its service.
pub fn tag_of(target: &str) -> Option<&str> {
    if !target.ends_with(SUFFIX) {
        return None;
    }
    let (_, rest) = target.rsplit_once(".run-")?;
    let (tag, service) = rest.split_once('.')?;
    let ok = tag.len() == 16
        && tag
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && service.ends_with(SUFFIX);
    ok.then_some(tag)
}

/// Whether this module may touch the entry `target`: a run's, under the sandbox.
pub fn ours(target: &str) -> bool {
    tag_of(target).is_some()
}

/// An entry of the keychain, as listed.
#[derive(Clone, Debug)]
pub struct Found {
    pub target: String,
    pub comment: String,
    pub written: SystemTime,
}

impl Found {
    /// Whether it is left by a dead run at `now`: its folder gone, or written more than a day ago. Never one that is not a
    /// run's, nor one without the run's comment.
    pub fn dead(&self, now: SystemTime) -> bool {
        if !ours(&self.target) {
            return false;
        }
        let Some(dir) = self.comment.strip_prefix(MARK) else {
            return false;
        };
        if !dir.is_empty() && !Path::new(dir).exists() {
            return true;
        }
        now.duration_since(self.written)
            .is_ok_and(|age| age > MAX_AGE)
    }
}

/// The entries of the runs `runs` among `found`.
pub fn of_runs<'a>(found: &'a [Found], runs: &[Run]) -> Vec<&'a Found> {
    found
        .iter()
        .filter(|f| tag_of(&f.target).is_some_and(|t| runs.iter().any(|r| r.tag == t)))
        .collect()
}

/// The entries of the dead runs among `found`, at `now`.
pub fn of_dead(found: &[Found], now: SystemTime) -> Vec<&Found> {
    found.iter().filter(|f| f.dead(now)).collect()
}

#[cfg(windows)]
mod win {
    use super::*;
    use anyhow::{bail, Result};
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND, FILETIME};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredEnumerateW, CredFree, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// # Safety
    /// `p` is null or a string the system ended with a nul.
    unsafe fn text(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    }

    fn time(t: FILETIME) -> SystemTime {
        // 100 ns since 1601-01-01.
        let ticks = (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
        let unix = Duration::from_nanos(ticks.saturating_mul(100))
            .saturating_sub(Duration::from_secs(11_644_473_600));
        SystemTime::UNIX_EPOCH + unix
    }

    /// The generic entries of the sandbox: those whose name ends with `.escouade.sandbox`.
    pub fn list() -> Result<Vec<Found>> {
        let mut count = 0u32;
        let mut creds: *mut *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: the system fills both, `creds` freed below.
        if unsafe { CredEnumerateW(std::ptr::null(), 0, &mut count, &mut creds) } == 0 {
            let e = unsafe { GetLastError() };
            if e == ERROR_NOT_FOUND {
                return Ok(Vec::new());
            }
            bail!("the keychain could not be listed (Windows error code {e})");
        }
        let mut found = Vec::new();
        for i in 0..count as usize {
            // SAFETY: `count` credentials the system gave.
            let c = unsafe { &**creds.add(i) };
            if c.Type != CRED_TYPE_GENERIC {
                continue;
            }
            let target = unsafe { text(c.TargetName) };
            if target.ends_with(SUFFIX) {
                found.push(Found {
                    target,
                    comment: unsafe { text(c.Comment) },
                    written: time(c.LastWritten),
                });
            }
        }
        // SAFETY: what CredEnumerateW allocated.
        unsafe { CredFree(creds as *const _) };
        Ok(found)
    }

    /// Takes back the entry `target`, a run's only.
    pub fn remove(target: &str) -> Result<()> {
        if !ours(target) {
            bail!("not an entry of a sandboxed run: {target}");
        }
        let name = wide(target);
        // SAFETY: a string ended with a nul.
        if unsafe { CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0 {
            let e = unsafe { GetLastError() };
            if e != ERROR_NOT_FOUND {
                bail!("the keychain kept {target} (Windows error code {e})");
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
pub use win::list;

/// The entries of the runs on the folders `dirs`, as the keychain holds them now.
#[cfg(windows)]
pub fn entries_of(dirs: &[PathBuf]) -> anyhow::Result<Vec<String>> {
    let runs: Vec<Run> = dirs.iter().map(|d| Run::of(d)).collect();
    let found = list()?;
    Ok(of_runs(&found, &runs)
        .into_iter()
        .map(|f| f.target.clone())
        .collect())
}

/// Takes back what the runs on the folders `dirs` wrote (the end of an end-to-end test, whatever its outcome): how many went.
#[cfg(windows)]
pub fn clean(dirs: &[PathBuf]) -> anyhow::Result<usize> {
    let targets = entries_of(dirs)?;
    for t in &targets {
        win::remove(t)?;
    }
    Ok(targets.len())
}

/// Takes back what dead runs left (the start of an end-to-end run): how many went.
#[cfg(windows)]
pub fn sweep(now: SystemTime) -> anyhow::Result<usize> {
    let found = list()?;
    let dead = of_dead(&found, now);
    for f in &dead {
        win::remove(&f.target)?;
    }
    Ok(dead.len())
}

#[cfg(not(windows))]
pub fn entries_of(_dirs: &[PathBuf]) -> anyhow::Result<Vec<String>> {
    Ok(Vec::new())
}

#[cfg(not(windows))]
pub fn clean(_dirs: &[PathBuf]) -> anyhow::Result<usize> {
    Ok(0)
}

#[cfg(not(windows))]
pub fn sweep(_now: SystemTime) -> anyhow::Result<usize> {
    Ok(0)
}

/// The command line of a debug build, `--sandbox-keychain <clean <dir>… | sweep | list <dir>…>`, for the end-to-end
/// fixture: None when `args` (without the program) is not one; else what to print and the exit code.
pub fn command(args: &[String]) -> Option<(String, i32)> {
    let (first, rest) = args.split_first()?;
    if first != "--sandbox-keychain" {
        return None;
    }
    let dirs = || rest[1..].iter().map(PathBuf::from).collect::<Vec<_>>();
    let done = match rest.first().map(String::as_str) {
        Some("clean") if rest.len() > 1 => clean(&dirs()).map(|n| format!("{n}")),
        Some("sweep") if rest.len() == 1 => sweep(SystemTime::now()).map(|n| format!("{n}")),
        Some("list") if rest.len() > 1 => entries_of(&dirs()).map(|t| t.join("\n")),
        _ => {
            return Some((
                "usage: --sandbox-keychain clean <dir>… | sweep | list <dir>…".into(),
                2,
            ))
        }
    };
    Some(match done {
        Ok(out) => (out, 0),
        Err(e) => (format!("{e:#}"), 1),
    })
}

#[cfg(test)]
#[path = "sandbox_keychain_tests.rs"]
mod tests;
