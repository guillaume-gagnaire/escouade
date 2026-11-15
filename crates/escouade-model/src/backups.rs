//! The copies of the files Escouade cannot make again, and their recovery at the start (ESC-469).
//!
//! On 2026-11-15 a power cut left `state.json` its size and nothing but zeros; the app started from an empty state, said
//! nothing, and wrote it over the file: every project, ticket and agent was lost. Since then:
//! - `paths::write_atomic` forces what it writes onto the disk;
//! - `back_up` keeps copies of `state.json`, `settings.json`, `plugins.json` and `integrations.json` in `backups/` of the
//!   data folder, made of a content read back and found whole, compressed, kept by age (`kept`);
//! - `recover`, at the start, before anything reads them: a file that does not read (zeros, cut short, no JSON) is set
//!   aside under a dated name, the newest copy that reads takes its place, and what happened is noted for the window
//!   (`note`, `notices`) until the user closes the notice (`dismiss`).

use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// The folder of the copies, in the data folder: one folder per file within it.
pub const DIR: &str = "backups";

/// The files copied and recovered, in the data folder.
pub const FILES: [&str; 4] = [
    "state.json",
    "settings.json",
    "plugins.json",
    "integrations.json",
];

/// How often a copy is made of a file that changed (and once at each start).
pub const EVERY: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// The file of the recoveries not yet read by the user, in the data folder.
pub const NOTICES: &str = "recovered.json";

/// Which copies stay (`kept`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// The newest copies, whatever their age.
    pub latest: usize,
    /// One copy per hour over that many hours: the first of each hour.
    pub hours: i64,
    /// One copy per day over that many days: the first of each day (UTC).
    pub days: i64,
    /// The copies of one file, all together, at most this many bytes: past it the oldest go (never the newest one).
    pub max_bytes: u64,
}

/// 6 + 48 + 30 copies at most per file; `state.json` of 9 MB is some 1 MB compressed: about 85 MB at most for it, a
/// few MB for the three others, and never more than 256 MB per file.
pub const POLICY: Policy = Policy {
    latest: 6,
    hours: 48,
    days: 30,
    max_bytes: 256 << 20,
};

/// A copy as the rotation sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Copy {
    pub at: DateTime<Utc>,
    pub size: u64,
}

/// Which of `copies` stay at `now`: the `latest` newest; the first of each hour of the last `hours`; the first of each
/// day of the last `days`; then, newest first, as long as their sizes add up to `max_bytes` at most (the newest always).
///
/// The first of an hour or of a day, not its last: once a period has its copy, no later copy takes its place. A state
/// emptied or damaged by mistake, copied again and again, only ever pushes out the `latest` copies: the hours and the
/// days before it keep theirs (the rotation goes by age, not by count).
pub fn kept(copies: &[Copy], now: DateTime<Utc>, policy: Policy) -> Vec<bool> {
    let mut order: Vec<usize> = (0..copies.len()).collect();
    // Newest first; the same time, the first listed first.
    order.sort_by(|&a, &b| copies[b].at.cmp(&copies[a].at).then(a.cmp(&b)));
    let mut keep = vec![false; copies.len()];
    for &i in order.iter().take(policy.latest) {
        keep[i] = true;
    }
    let mut first_of =
        |within: Duration, period: &dyn Fn(DateTime<Utc>) -> (i32, u32, u32, u32)| {
            let mut seen: std::collections::HashMap<(i32, u32, u32, u32), usize> =
                Default::default();
            // Oldest first: the first of each period is the one met first.
            for &i in order.iter().rev() {
                let at = copies[i].at;
                if now - at > within {
                    continue;
                }
                seen.entry(period(at)).or_insert(i);
            }
            for i in seen.into_values() {
                keep[i] = true;
            }
        };
    first_of(Duration::hours(policy.hours), &|at| {
        (at.year(), at.month(), at.day(), at.hour())
    });
    first_of(Duration::days(policy.days), &|at| {
        (at.year(), at.month(), at.day(), 0)
    });
    let mut total = 0u64;
    for (n, &i) in order.iter().enumerate() {
        if !keep[i] {
            continue;
        }
        total = total.saturating_add(copies[i].size);
        if n > 0 && total > policy.max_bytes {
            keep[i] = false;
        }
    }
    keep
}

/// The stem of a file of the data folder (`state` of `state.json`).
fn stem(file: &str) -> &str {
    file.strip_suffix(".json").unwrap_or(file)
}

/// The folder of the copies of `file`.
pub fn copies_dir(data: &Path, file: &str) -> PathBuf {
    data.join(DIR).join(stem(file))
}

const STAMP: &str = "%Y%m%dT%H%M%SZ";

/// A copy's name: `state-20261115T093600Z.json.gz`.
fn copy_name(file: &str, at: DateTime<Utc>) -> String {
    format!("{}-{}.json.gz", stem(file), at.format(STAMP))
}

/// The time a copy's name says, if it is one of `file`'s.
fn copy_time(file: &str, name: &str) -> Option<DateTime<Utc>> {
    let stamp = name
        .strip_prefix(stem(file))?
        .strip_prefix('-')?
        .strip_suffix(".json.gz")?;
    let naive = chrono::NaiveDateTime::parse_from_str(stamp, STAMP).ok()?;
    Some(Utc.from_utc_datetime(&naive))
}

/// The copies of `file`, newest first.
pub fn list(data: &Path, file: &str) -> Vec<(PathBuf, Copy)> {
    let dir = copies_dir(data, file);
    let mut all: Vec<(PathBuf, Copy)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let at = copy_time(file, e.file_name().to_str()?)?;
            let size = e.metadata().ok()?.len();
            Some((e.path(), Copy { at, size }))
        })
        .collect();
    all.sort_by_key(|(_, c)| std::cmp::Reverse(c.at));
    all
}

/// What a copy holds, uncompressed; None when it does not read.
pub fn read_copy(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(file)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

/// Whether the content of `file` (one of `FILES`) reads: JSON as Escouade reads it. The state and the settings by
/// their own types, the two others as a JSON object (their readers leave out an entry they do not read).
pub fn reads(file: &str, bytes: &[u8]) -> bool {
    match file {
        "state.json" => serde_json::from_slice::<crate::model::PersistedState>(bytes).is_ok(),
        "settings.json" => serde_json::from_slice::<crate::model::Settings>(bytes).is_ok(),
        _ => serde_json::from_slice::<serde_json::Value>(bytes).is_ok_and(|v| v.is_object()),
    }
}

/// What a copy did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackedUp {
    /// A copy made.
    Made(PathBuf),
    /// The file is the same as its newest copy.
    Same,
    /// No file.
    Missing,
    /// The file does not read: no copy of it (the good ones stay).
    Unreadable,
}

/// A copy of `file` of the data folder `data` at `now`, when it reads and differs from its newest copy; then the
/// rotation (`kept` by `POLICY`). The copy is written as `write_atomic` writes: whole on the disk, or not there.
pub fn back_up(data: &Path, file: &str, now: DateTime<Utc>) -> std::io::Result<BackedUp> {
    let bytes = match std::fs::read(data.join(file)) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BackedUp::Missing),
        Err(e) => return Err(e),
    };
    if !reads(file, &bytes) {
        return Ok(BackedUp::Unreadable);
    }
    let copies = list(data, file);
    let made = if copies
        .first()
        .and_then(|(p, _)| read_copy(p))
        .is_some_and(|newest| newest == bytes)
    {
        BackedUp::Same
    } else {
        let dir = copies_dir(data, file);
        std::fs::create_dir_all(&dir)?;
        let mut z = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        z.write_all(&bytes)?;
        let path = dir.join(copy_name(file, now));
        // Private: `integrations.json` may hold secrets the keychain refused.
        crate::paths::write_private(&path, &z.finish()?)?;
        BackedUp::Made(path)
    };
    prune(data, file, now);
    Ok(made)
}

/// `back_up` of each of `FILES`, what fails logged.
pub fn back_up_all(data: &Path, now: DateTime<Utc>) {
    for file in FILES {
        match back_up(data, file, now) {
            // Named from the data folder: the journal of every start names no folder of the disk.
            Ok(BackedUp::Made(p)) => log::info!(
                "{file}: copied to {}",
                p.strip_prefix(data).unwrap_or(&p).display()
            ),
            Ok(BackedUp::Unreadable) => {
                log::warn!("{file} does not read: not copied, its copies stay")
            }
            Ok(_) => {}
            Err(e) => log::error!("{file}: no copy made: {e}"),
        }
    }
}

/// The copies of `file` that `kept` lets go, removed.
pub fn prune(data: &Path, file: &str, now: DateTime<Utc>) {
    let copies = list(data, file);
    let keep = kept(
        &copies.iter().map(|(_, c)| *c).collect::<Vec<_>>(),
        now,
        POLICY,
    );
    for ((path, _), keep) in copies.iter().zip(keep) {
        if !keep {
            if let Err(e) = std::fs::remove_file(path) {
                log::warn!("{}: copy not removed: {e}", path.display());
            }
        }
    }
}

/// How a file did not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Damage {
    /// Nothing but zeros (or nothing at all): the data never reached the disk.
    Zeros,
    /// JSON that stops before its end.
    Cut,
    /// Something else than what Escouade wrote.
    Invalid,
}

/// How `bytes`, which do not read, are damaged.
pub fn damage(bytes: &[u8]) -> Damage {
    if bytes.iter().all(|&b| b == 0) {
        return Damage::Zeros;
    }
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Err(e) if e.is_eof() => Damage::Cut,
        _ => Damage::Invalid,
    }
}

/// A file found unreadable at a start, and what was done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovery {
    /// The file, as named in the data folder (`state.json`).
    pub file: String,
    pub damage: Damage,
    /// When the start found it.
    pub found_at: DateTime<Utc>,
    /// When it was last written, as the system says.
    pub written_at: Option<DateTime<Utc>>,
    /// Where it was set aside, whole: a dated name beside it.
    pub kept_as: Option<PathBuf>,
    /// The copy that took its place, and its time; None when no copy reads (the app starts without it).
    pub restored: Option<PathBuf>,
    pub restored_at: Option<DateTime<Utc>>,
    /// The copies newer than that one that did not read, passed over.
    pub skipped: usize,
    /// The folder of the copies.
    pub copies: PathBuf,
}

/// `path` moved aside to a dated name beside it (`state.unreadable-20261115T093600Z.json`), never over another one.
pub fn set_aside(path: &Path, now: DateTime<Utc>) -> std::io::Result<PathBuf> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let stem = stem(name);
    let stamp = now.format(STAMP);
    let mut n = 1;
    let to = loop {
        let candidate = if n == 1 {
            path.with_file_name(format!("{stem}.unreadable-{stamp}.json"))
        } else {
            path.with_file_name(format!("{stem}.unreadable-{stamp}-{n}.json"))
        };
        if !candidate.exists() {
            break candidate;
        }
        n += 1;
    };
    std::fs::rename(path, &to)?;
    crate::paths::sync_dir_of(&to)?;
    Ok(to)
}

/// At the start, before anything reads it: `file` of the data folder `data`, when it is there and does not read, set
/// aside under a dated name, and the newest of its copies that reads put in its place (those that do not are passed
/// over). None when it reads, or is not there (a new installation): nothing to tell. A copy that reads is not
/// there: the file is set aside all the same, and the app starts without it; the recovery says so.
pub fn recover(data: &Path, file: &str, now: DateTime<Utc>) -> Option<Recovery> {
    let path = data.join(file);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            // Not read, but maybe whole (held by another program): left as it is.
            log::error!("{}: cannot be read: {e}", path.display());
            return None;
        }
    };
    if reads(file, &bytes) {
        return None;
    }
    let damage = damage(&bytes);
    let written_at = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .map(DateTime::<Utc>::from);
    let kept_as = match set_aside(&path, now) {
        Ok(p) => Some(p),
        Err(e) => {
            log::error!("{}: cannot be set aside: {e}", path.display());
            None
        }
    };
    let mut skipped = 0;
    let mut restored = None;
    for (copy, c) in list(data, file) {
        match read_copy(&copy).filter(|b| reads(file, b)) {
            Some(b) => {
                let put = if file == "integrations.json" {
                    crate::paths::write_private(&path, &b)
                } else {
                    crate::paths::write_atomic(&path, &b)
                };
                match put {
                    Ok(()) => {
                        restored = Some((copy, c.at));
                        break;
                    }
                    Err(e) => {
                        log::error!(
                            "{}: copy {} not put back: {e}",
                            path.display(),
                            copy.display()
                        );
                        break;
                    }
                }
            }
            None => {
                log::warn!(
                    "{}: its copy {} does not read, passed over",
                    path.display(),
                    copy.display()
                );
                skipped += 1;
            }
        }
    }
    let r = Recovery {
        file: file.to_string(),
        damage,
        found_at: now,
        written_at,
        kept_as,
        restored_at: restored.as_ref().map(|(_, at)| *at),
        restored: restored.map(|(p, _)| p),
        skipped,
        copies: copies_dir(data, file),
    };
    match (&r.restored, &r.restored_at) {
        (Some(copy), Some(at)) => log::error!(
            "{file} did not read ({damage:?}): set aside as {}, its copy of {at} put back ({}); what changed since is lost",
            r.kept_as.as_deref().unwrap_or(Path::new("?")).display(),
            copy.display()
        ),
        _ => log::error!(
            "{file} did not read ({damage:?}): set aside as {}, no copy of it reads: the app starts without it",
            r.kept_as.as_deref().unwrap_or(Path::new("?")).display()
        ),
    }
    Some(r)
}

/// `recover` of each of `FILES`, what was done noted for the window (`note`).
pub fn recover_all(data: &Path, now: DateTime<Utc>) -> Vec<Recovery> {
    let found: Vec<Recovery> = FILES.iter().filter_map(|f| recover(data, f, now)).collect();
    if !found.is_empty() {
        note(data, &found);
    }
    found
}

/// The recoveries not yet read by the user (they stay until `dismiss`, across starts).
pub fn notices(data: &Path) -> Vec<Recovery> {
    std::fs::read(data.join(NOTICES))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// `found` added to the recoveries the window shows.
pub fn note(data: &Path, found: &[Recovery]) {
    let mut all = notices(data);
    all.extend_from_slice(found);
    let written = serde_json::to_vec_pretty(&all)
        .map_err(std::io::Error::other)
        .and_then(|b| crate::paths::write_atomic(&data.join(NOTICES), &b));
    if let Err(e) = written {
        log::error!("recoveries not noted: {e}");
    }
}

/// The user closed the notice: the recoveries are forgotten (the files set aside and the copies stay).
pub fn dismiss(data: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(data.join(NOTICES)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "backups_tests.rs"]
mod tests;
