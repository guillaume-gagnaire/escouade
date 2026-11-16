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

/// The other files that are not made again and have copies (ESC-487), beside `FILES`: the accounts of the ticket sources
/// after the first of each service and the syncs waiting. Then, found on the disk (`copied`), each closed project's
/// history (`closed-projects/<id>.json`) and where each agent of a project is in its workflows
/// (`projects/<id>/workflow-tracks.json`). They are not recovered at the start (their readers set aside a file that does
/// not read): their copies are taken back from the screen of the copies.
pub const MORE: [&str; 2] = ["ticket-accounts.json", "sync-queue.json"];

/// The folder of the closed projects' histories, in the data folder (`DataDir::closed_projects`).
pub const CLOSED: &str = "closed-projects";

/// The copies of one of the files of `MORE` or found on the disk: the same rotation, 32 MB at most per file.
pub const POLICY_MORE: Policy = Policy {
    max_bytes: 32 << 20,
    ..POLICY
};

/// Every copy, of every file, at most this many bytes together: past it the oldest go first, never the newest copy of a
/// file (`prune_total`). The state's 85 MB at most, a few MB for the other three, a closed project's history rarely
/// over a MB compressed: far from it in use.
pub const TOTAL_MAX: u64 = 512 << 20;

/// The copies of a file that is no longer there (a closed project reopened or erased) stay this long after the newest
/// of them, then go: long enough to find a history lost by mistake.
pub const ORPHAN_DAYS: i64 = 30;

/// The policy of `file`'s copies.
pub fn policy(file: &str) -> Policy {
    if FILES.contains(&file) {
        POLICY
    } else {
        POLICY_MORE
    }
}

/// Whether `file` is a path of the data folder whose copies Escouade keeps: one of `FILES` or `MORE`, a closed project's
/// history or a project's workflow tracks, each named by plain ids (never `..`, never a folder above).
pub fn is_copied(file: &str) -> bool {
    if FILES.contains(&file) || MORE.contains(&file) {
        return true;
    }
    let parts: Vec<&str> = file.split('/').collect();
    match parts.as_slice() {
        [CLOSED, name] => name
            .strip_suffix(".json")
            .is_some_and(crate::paths::is_plain_id),
        ["projects", id, "workflow-tracks.json"] => crate::paths::is_plain_id(id),
        _ => false,
    }
}

/// The files of the data folder that have copies now: `FILES`, `MORE`, then each closed project's history and each
/// project's workflow tracks on the disk.
pub fn copied(data: &Path) -> Vec<String> {
    let mut out: Vec<String> = FILES
        .iter()
        .chain(MORE.iter())
        .map(|f| f.to_string())
        .collect();
    let plain = |e: &std::fs::DirEntry, suffix: &str| -> Option<String> {
        let name = e.file_name().to_str()?.to_string();
        let id = name.strip_suffix(suffix)?;
        crate::paths::is_plain_id(id).then(|| id.to_string())
    };
    let mut closed: Vec<String> = std::fs::read_dir(data.join(CLOSED))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| plain(&e, ".json"))
        .map(|id| format!("{CLOSED}/{id}.json"))
        .collect();
    closed.sort();
    out.extend(closed);
    let mut tracks: Vec<String> = std::fs::read_dir(data.join("projects"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("workflow-tracks.json").is_file())
        .filter_map(|e| plain(&e, ""))
        .map(|id| format!("projects/{id}/workflow-tracks.json"))
        .collect();
    tracks.sort();
    out.extend(tracks);
    out
}

/// Every file that is copied now or has copies still (`copied`, then those whose copies are left of a file gone: a
/// closed project reopened, erased or lost).
pub fn known(data: &Path) -> Vec<String> {
    let mut all = copied(data);
    let backups = data.join(DIR);
    let mut found = Vec::new();
    for (folder, file_of) in [
        (
            backups.join(CLOSED),
            &(|id: &str| format!("{CLOSED}/{id}.json")) as &dyn Fn(&str) -> String,
        ),
        (backups.join("projects"), &|id: &str| {
            format!("projects/{id}/workflow-tracks.json")
        }),
    ] {
        for e in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            if let Some(id) = e.file_name().to_str() {
                let file = file_of(id);
                if is_copied(&file) && !all.contains(&file) && !list(data, &file).is_empty() {
                    found.push(file);
                }
            }
        }
    }
    found.sort();
    all.extend(found);
    all
}

/// The stem of a file of the data folder (`state` of `state.json`, `closed-projects/abc` of `closed-projects/abc.json`).
fn stem(file: &str) -> &str {
    file.strip_suffix(".json").unwrap_or(file)
}

/// The stem of a file's own name (`abc` of `closed-projects/abc.json`).
fn base(file: &str) -> &str {
    stem(file.rsplit('/').next().unwrap_or(file))
}

/// The folder of the copies of `file`.
pub fn copies_dir(data: &Path, file: &str) -> PathBuf {
    let mut dir = data.join(DIR);
    for part in stem(file).split('/') {
        dir.push(part);
    }
    dir
}

/// `file` (a path of the data folder with `/`) on the disk.
pub fn file_path(data: &Path, file: &str) -> PathBuf {
    let mut path = data.to_path_buf();
    for part in file.split('/') {
        path.push(part);
    }
    path
}

const STAMP: &str = "%Y%m%dT%H%M%SZ";

/// A copy's name: `state-20261115T093600Z.json.gz`.
fn copy_name(file: &str, at: DateTime<Utc>) -> String {
    format!("{}-{}.json.gz", base(file), at.format(STAMP))
}

/// The time a copy's name says, if it is one of `file`'s.
fn copy_time(file: &str, name: &str) -> Option<DateTime<Utc>> {
    let stamp = name
        .strip_prefix(base(file))?
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
///
/// A closed project's history reads when it is an object whose project has the id its name says (as `closed::scan`
/// reads it); the syncs waiting are a list; the workflow tracks an object or a list.
pub fn reads(file: &str, bytes: &[u8]) -> bool {
    let value = || serde_json::from_slice::<serde_json::Value>(bytes).ok();
    match file {
        "state.json" => serde_json::from_slice::<crate::model::PersistedState>(bytes).is_ok(),
        "settings.json" => serde_json::from_slice::<crate::model::Settings>(bytes).is_ok(),
        "sync-queue.json" => value().is_some_and(|v| v.is_array()),
        f if f.starts_with("closed-projects/") => value().is_some_and(|v| {
            v.get("project")
                .and_then(|p| p.get("id"))
                .and_then(|id| id.as_str())
                .is_some_and(|id| id == base(f))
        }),
        f if f.ends_with("/workflow-tracks.json") => {
            value().is_some_and(|v| v.is_object() || v.is_array())
        }
        _ => value().is_some_and(|v| v.is_object()),
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
    let bytes = match std::fs::read(file_path(data, file)) {
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

/// `back_up` of each file `copied`, what fails logged; then the copies of the files gone that are past `ORPHAN_DAYS`
/// removed, and all of them kept under `TOTAL_MAX`.
pub fn back_up_all(data: &Path, now: DateTime<Utc>) {
    for file in copied(data) {
        let file = file.as_str();
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
    prune_orphans(data, now);
    prune_total(data, TOTAL_MAX);
}

/// The copies of `file` that `kept` lets go, removed.
pub fn prune(data: &Path, file: &str, now: DateTime<Utc>) {
    let copies = list(data, file);
    let keep = kept(
        &copies.iter().map(|(_, c)| *c).collect::<Vec<_>>(),
        now,
        policy(file),
    );
    for ((path, _), keep) in copies.iter().zip(keep) {
        if !keep {
            remove_copy(data, path);
        }
    }
}

fn remove_copy(data: &Path, path: &Path) {
    if let Err(e) = std::fs::remove_file(path) {
        log::warn!(
            "{}: copy not removed: {e}",
            path.strip_prefix(data).unwrap_or(path).display()
        );
    }
}

/// The copies of a file that is no longer there (`known` but not `copied`: a closed project reopened, erased or lost)
/// whose newest is more than `ORPHAN_DAYS` old, removed with their folder. The four of `FILES` never: theirs stay.
pub fn prune_orphans(data: &Path, now: DateTime<Utc>) {
    let present = copied(data);
    for file in known(data) {
        if present.contains(&file) || FILES.contains(&file.as_str()) {
            continue;
        }
        let copies = list(data, &file);
        let old = copies
            .first()
            .is_none_or(|(_, c)| now - c.at > Duration::days(ORPHAN_DAYS));
        if old {
            for (path, _) in &copies {
                remove_copy(data, path);
            }
            let _ = std::fs::remove_dir(copies_dir(data, &file));
        }
    }
}

/// Every copy of every file `known`, newest first within each file.
pub fn all_copies(data: &Path) -> Vec<(String, PathBuf, Copy)> {
    known(data)
        .into_iter()
        .flat_map(|f| {
            list(data, &f)
                .into_iter()
                .map(move |(p, c)| (f.clone(), p, c))
        })
        .collect()
}

/// What all the copies take on the disk.
pub fn total_size(data: &Path) -> u64 {
    all_copies(data).iter().map(|(_, _, c)| c.size).sum()
}

/// The copies of all the files kept at `max` bytes together: the oldest go first, never the newest copy of a file.
pub fn prune_total(data: &Path, max: u64) {
    let all = all_copies(data);
    let mut total: u64 = all.iter().map(|(_, _, c)| c.size).sum();
    if total <= max {
        return;
    }
    // `all_copies` lists each file's newest first.
    let mut seen = std::collections::HashSet::new();
    let newest: std::collections::HashSet<&Path> = all
        .iter()
        .filter(|(f, _, _)| seen.insert(f.as_str()))
        .map(|(_, p, _)| p.as_path())
        .collect();
    let mut oldest: Vec<&(String, PathBuf, Copy)> = all
        .iter()
        .filter(|(_, p, _)| !newest.contains(p.as_path()))
        .collect();
    oldest.sort_by_key(|(_, _, c)| c.at);
    for (_, path, c) in oldest {
        if total <= max {
            break;
        }
        remove_copy(data, path);
        total = total.saturating_sub(c.size);
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
    set_aside_as(path, now, UNREADABLE)
}

/// The label of a file set aside because it did not read at a start.
pub const UNREADABLE: &str = "unreadable";
/// The label of a file set aside by a copy taken back in its place (ESC-487).
pub const BEFORE_RESTORE: &str = "before-restore";

/// `path` moved aside to `<stem>.<label>-<date>.json` beside it, never over another one.
pub fn set_aside_as(path: &Path, now: DateTime<Utc>, label: &str) -> std::io::Result<PathBuf> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let stem = stem(name);
    let stamp = now.format(STAMP);
    let mut n = 1;
    let to = loop {
        let candidate = if n == 1 {
            path.with_file_name(format!("{stem}.{label}-{stamp}.json"))
        } else {
            path.with_file_name(format!("{stem}.{label}-{stamp}-{n}.json"))
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
            log::error!("{}: cannot be read: {e}", file);
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
            log::error!("{}: cannot be set aside: {e}", file);
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
                            file,
                            copy.strip_prefix(data).unwrap_or(&copy).display()
                        );
                        break;
                    }
                }
            }
            None => {
                log::warn!(
                    "{}: its copy {} does not read, passed over",
                    file,
                    copy.strip_prefix(data).unwrap_or(&copy).display()
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
            r.kept_as.as_deref().and_then(|p| p.strip_prefix(data).ok()).unwrap_or(Path::new("?")).display(),
            copy.strip_prefix(data).unwrap_or(copy).display()
        ),
        _ => log::error!(
            "{file} did not read ({damage:?}): set aside as {}, no copy of it reads: the app starts without it",
            r.kept_as.as_deref().and_then(|p| p.strip_prefix(data).ok()).unwrap_or(Path::new("?")).display()
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

/// Why a copy cannot be taken back.
#[derive(Debug)]
pub enum RestoreError {
    /// Not a file whose copies Escouade keeps, or not one of its copies.
    Unknown,
    /// The copy does not read (or does not read as that file): nothing is touched.
    Unreadable,
    Io(std::io::Error),
}

impl From<std::io::Error> for RestoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "no such copy"),
            Self::Unreadable => write!(f, "the copy does not read"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

/// The copy of `file` named `name` (as `list` gives it), when it is one: never a path out of its folder.
pub fn find_copy(data: &Path, file: &str, name: &str) -> Option<(PathBuf, Copy)> {
    if !is_copied(file) {
        return None;
    }
    list(data, file)
        .into_iter()
        .find(|(p, _)| p.file_name().and_then(|n| n.to_str()) == Some(name))
}

/// What the copy of `file` named `name` holds, read back and found to read as that file.
pub fn read_good_copy(data: &Path, file: &str, name: &str) -> Result<Vec<u8>, RestoreError> {
    let (path, _) = find_copy(data, file, name).ok_or(RestoreError::Unknown)?;
    read_copy(&path)
        .filter(|b| reads(file, b))
        .ok_or(RestoreError::Unreadable)
}

/// The files whose content may hold a secret the keychain refused: written readable by their owner alone.
fn private(file: &str) -> bool {
    matches!(file, "integrations.json" | "ticket-accounts.json")
}

/// The copy of `file` named `name` put in its place: read back and found to read first (else nothing is touched), the
/// file of now set aside under a dated name beside it (`<stem>.before-restore-<date>.json`, never lost, never over
/// another one), then the copy written as `write_atomic` writes. Where the file of now went, if it was there.
///
/// For a file the app holds in memory (the state, the settings…), only when the app does not run on it: at a start
/// (`apply_pending`), or for a closed project's history, which nothing holds.
pub fn restore(
    data: &Path,
    file: &str,
    name: &str,
    now: DateTime<Utc>,
) -> Result<Option<PathBuf>, RestoreError> {
    let bytes = read_good_copy(data, file, name)?;
    let path = file_path(data, file);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let aside = if path.exists() {
        Some(set_aside_as(&path, now, BEFORE_RESTORE)?)
    } else {
        None
    };
    if private(file) {
        crate::paths::write_private(&path, &bytes)?;
    } else {
        crate::paths::write_atomic(&path, &bytes)?;
    }
    log::warn!(
        "{file}: its copy {name} put back by hand; the file before {}",
        match &aside {
            Some(p) => format!(
                "set aside as {}",
                p.strip_prefix(data).unwrap_or(p).display()
            ),
            None => "was not there".into(),
        }
    );
    Ok(aside)
}

/// The copy the user chose to take back, put in place at the next start (the app holds the file in memory and would
/// write it over): `restore-pending.json` of the data folder.
pub const PENDING: &str = "restore-pending.json";

/// A copy to take back at the next start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    /// The file, as named in the data folder (`state.json`).
    pub file: String,
    /// The copy's name in its folder (`state-20261115T093600Z.json.gz`).
    pub copy: String,
    /// The copy's time.
    pub copy_at: DateTime<Utc>,
    /// When it was asked.
    pub asked_at: DateTime<Utc>,
}

/// The copy chosen for the next start, if one is.
pub fn pending(data: &Path) -> Option<Pending> {
    std::fs::read(data.join(PENDING))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
}

/// The copy `name` of `file` chosen to be taken back at the next start: read back and found to read first. One at a
/// time: a choice replaces the one before.
pub fn plan(
    data: &Path,
    file: &str,
    name: &str,
    now: DateTime<Utc>,
) -> Result<Pending, RestoreError> {
    let (_, c) = find_copy(data, file, name).ok_or(RestoreError::Unknown)?;
    read_good_copy(data, file, name)?;
    let p = Pending {
        file: file.to_string(),
        copy: name.to_string(),
        copy_at: c.at,
        asked_at: now,
    };
    let bytes = serde_json::to_vec_pretty(&p).map_err(std::io::Error::other)?;
    crate::paths::write_atomic(&data.join(PENDING), &bytes)?;
    Ok(p)
}

/// The copy chosen for the next start called off.
pub fn cancel(data: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(data.join(PENDING)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// A copy taken back by hand, as the window tells it after the start that put it in place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Restored {
    pub file: String,
    pub copy_at: DateTime<Utc>,
    pub done_at: DateTime<Utc>,
    /// Where the file before went (None: it was not there).
    pub set_aside: Option<PathBuf>,
    /// Why it was not put back (the copy no longer reads, or is gone): the file stays as it was.
    pub error: Option<String>,
}

/// The copies taken back by hand not yet told: `restored.json` of the data folder, until the window's notice closes.
pub const RESTORED: &str = "restored.json";

/// At the start, before `recover_all` and anything that reads the files: the copy chosen (`plan`) put in place, once
/// (the choice is forgotten whether it went or not), and what happened noted for the window.
pub fn apply_pending(data: &Path, now: DateTime<Utc>) -> Option<Restored> {
    let p = pending(data)?;
    let _ = cancel(data);
    let done = restore(data, &p.file, &p.copy, now);
    let r = Restored {
        file: p.file.clone(),
        copy_at: p.copy_at,
        done_at: now,
        set_aside: done.as_ref().ok().cloned().flatten(),
        error: done.as_ref().err().map(|e| e.to_string()),
    };
    if let Err(e) = &done {
        log::error!("{}: its copy {} not put back: {e}", p.file, p.copy);
    }
    let mut all = restored(data);
    all.push(r.clone());
    let written = serde_json::to_vec_pretty(&all)
        .map_err(std::io::Error::other)
        .and_then(|b| crate::paths::write_atomic(&data.join(RESTORED), &b));
    if let Err(e) = written {
        log::error!("copy taken back not noted: {e}");
    }
    Some(r)
}

/// The copies taken back by hand not yet told.
pub fn restored(data: &Path) -> Vec<Restored> {
    std::fs::read(data.join(RESTORED))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// The notice of the copies taken back closed.
pub fn dismiss_restored(data: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(data.join(RESTORED)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// A file set aside, by a start that found it unreadable or by a copy taken back in its place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAside {
    /// Its path on the disk.
    pub path: PathBuf,
    /// The file it was (`state.json`, `closed-projects/<id>.json`).
    pub file: String,
    /// `unreadable` or `before-restore`.
    pub why: String,
    pub size: u64,
    pub at: Option<DateTime<Utc>>,
}

/// The files set aside in the data folder and in the folder of the closed projects' histories, newest first.
pub fn set_aside_files(data: &Path) -> Vec<SetAside> {
    let mut out = Vec::new();
    for (dir, prefix) in [
        (data.to_path_buf(), ""),
        (data.join(CLOSED), "closed-projects/"),
    ] {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let Some(name) = e.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let Some(rest) = name.strip_suffix(".json") else {
                continue;
            };
            let found = [UNREADABLE, BEFORE_RESTORE].iter().find_map(|label| {
                let (stem, after) = rest.split_once(&format!(".{label}-"))?;
                (!stem.is_empty() && after.len() >= 16).then(|| (stem.to_string(), *label))
            });
            let Some((stem, why)) = found else {
                continue;
            };
            let meta = e.metadata().ok();
            out.push(SetAside {
                path: e.path(),
                file: format!("{prefix}{stem}.json"),
                why: why.to_string(),
                size: meta.as_ref().map_or(0, |m| m.len()),
                at: meta
                    .and_then(|m| m.modified().ok())
                    .map(DateTime::<Utc>::from),
            });
        }
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.at));
    out
}

#[cfg(test)]
#[path = "backups_tests.rs"]
mod tests;
