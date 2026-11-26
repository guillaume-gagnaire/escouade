//! The session of Claude Code of an agent passed to a teammate (TS-17b, `docs/TEAM-SESSION-COPY.md` §4, choice C16-B):
//! what is done to it on each side, apart from the passage itself (`teamhandoff.rs`).
//!
//! - **Who passes it** sends the session's file as Claude Code wrote it (`<config>/projects/<folder>/<id>.jsonl`, its
//!   `transcript_path`), the worktree it ran in and the version of `claude` that wrote it ([`SessionCopy`]), in the
//!   encrypted deposit of the passage.
//! - **Who takes it with Claude Code** checks the version (theirs the same or newer, [`version_ok`]), rewrites the
//!   worktree of who passed it into theirs in each string of each JSON object of each line, never in the raw text (the
//!   `\` of Windows is doubled in JSON), under both separators ([`rewrite`]); then checks that no occurrence of the old
//!   worktree is left, in any form, else nothing is placed. The paths out of the worktree (the session's temporary
//!   folder, an `--add-dir`, the user's folders) are not rewritten. The copy goes in the account's folder chosen, under
//!   the folder Claude Code looks a session up in (`projects/<worktree encoded>/<id>.jsonl`, [`place`]), and the agent
//!   starts by `--resume <id> --fork-session`: a new id, never two histories under the same one.
//! - **Any failure** (the version, a path left, the session not found, an error at the first turn) falls back on ESC-88's
//!   handover file, and the copy placed is taken out of the account's folder ([`remove`]).

use std::path::{Path, PathBuf};

use escouade_team::handoff::SessionCopy;
use serde_json::Value;

use crate::engine::registry::Version;

/// The folder of `projects` Claude Code keeps the sessions of `dir` in: each character that is neither an ASCII letter
/// nor a digit becomes `-` (`C:\Users\a\wt` gives `C--Users-a-wt`).
pub(crate) fn encoded(dir: &str) -> String {
    dir.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Why a session passed is not resumed natively: the agent goes on with the handover file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NotNative {
    /// This machine's `claude` is older than the session's.
    Older { mine: String, theirs: String },
    /// This machine's version of `claude`, or the session's, is not known.
    UnknownVersion,
    /// A line of the session is not JSON: it is not rewritten blind.
    Unreadable,
    /// The worktree of who passed it is still in the session once rewritten.
    PathLeft,
    /// The session's id is not one Claude Code gives.
    BadId,
    /// A session of that id is already in the account's folder: it is not overwritten.
    Taken,
    /// It could not be written in the account's folder.
    Io(String),
}

impl NotNative {
    /// Said in the conversation of who takes it, before the handover file.
    pub(crate) fn said(&self) -> String {
        match self {
            NotNative::Older { mine, theirs } => tr!(
                "sa version de claude ({theirs}) est plus récente que celle de ce poste ({mine})",
                "its version of claude ({theirs}) is newer than this machine’s ({mine})"
            ),
            NotNative::UnknownVersion => tr!(
                "la version de claude de la session ou de ce poste n’est pas connue",
                "the version of claude of the session or of this machine isn’t known"
            ),
            NotNative::Unreadable => tr!(
                "une ligne de la session ne se lit pas",
                "a line of the session can’t be read"
            ),
            NotNative::PathLeft => tr!(
                "le dossier de travail d’avant y resterait après la réécriture des chemins",
                "the folder it worked in before would stay in it after its paths are rewritten"
            ),
            NotNative::BadId => tr!(
                "son identifiant n’est pas celui d’une session de Claude Code",
                "its id isn’t one of a session of Claude Code"
            ),
            NotNative::Taken => tr!(
                "une session du même identifiant est déjà dans le dossier du compte",
                "a session of the same id is already in the account’s folder"
            ),
            NotNative::Io(e) => tr!(
                "elle ne s’écrit pas dans le dossier du compte ({e})",
                "it can’t be written in the account’s folder ({e})"
            ),
        }
    }
}

/// The version of `claude` that wrote the session: the newest `version` of its lines (Claude Code writes it on each).
pub(crate) fn session_version(lines: &str) -> Option<String> {
    lines
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| v["version"].as_str().map(str::to_string))
        .filter(|v| Version::find(v).is_some())
        .max_by(|a, b| Version::parse(a).cmp(&Version::parse(b)))
}

/// This machine's `claude` (`mine`) takes the session of `theirs`: the same version or a newer one.
pub(crate) fn version_ok(mine: Option<&str>, theirs: &str) -> Result<(), NotNative> {
    let (Some(m), Some(t)) = (mine.and_then(Version::find), Version::find(theirs)) else {
        return Err(NotNative::UnknownVersion);
    };
    if m < t {
        return Err(NotNative::Older {
            mine: m.to_string(),
            theirs: t.to_string(),
        });
    }
    Ok(())
}

/// A path of Windows (`C:\…` or `C:/…`): compared without case, as Windows does.
fn windows(path: &str) -> bool {
    let b = path.as_bytes();
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

/// The path as Git Bash writes it (`/c/Users/a/wt`).
fn msys(path: &str) -> Option<String> {
    windows(path).then(|| {
        let rest = path[2..].replace('\\', "/");
        format!("/{}{}", path[..1].to_ascii_lowercase(), rest)
    })
}

/// The forms of the old worktree, each with the form of the new one it becomes: the form of its machine (`\` on
/// Windows, `/` elsewhere) becomes the new one's form of its machine, the other the other; and for a path of Windows,
/// as Git Bash writes it. Longest first, none twice.
fn forms(from: &str, to: &str) -> Vec<(String, String)> {
    let from = from.trim_end_matches(['\\', '/']);
    let to = to.trim_end_matches(['\\', '/']);
    let to_fs = to.replace('\\', "/");
    // A worktree of Unix keeps its `/` whatever the form it replaces.
    let to_native = if windows(to) {
        to.replace('/', "\\")
    } else {
        to_fs.clone()
    };
    let (from_native, from_other) = if windows(from) {
        (from.replace('/', "\\"), from.replace('\\', "/"))
    } else {
        (from.replace('\\', "/"), from.replace('/', "\\"))
    };
    let mut out = vec![(from_native, to_native.clone())];
    out.push((
        from_other,
        if windows(from) {
            to_fs.clone()
        } else {
            to_native
        },
    ));
    if let Some(m) = msys(from) {
        out.push((m, msys(to).unwrap_or(to_fs)));
    }
    let mut seen = Vec::new();
    out.retain(|(f, _)| {
        let key = f.to_ascii_lowercase();
        let new = !f.is_empty() && !seen.contains(&key);
        seen.push(key);
        new
    });
    out.sort_by_key(|(f, _)| std::cmp::Reverse(f.len()));
    out
}

/// The character after a match goes on with the same name (`wt2`, `wt-old`, `wt.bak`): it is another path.
fn goes_on(rest: &str) -> bool {
    rest.chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// The byte offsets of the occurrences of `needle` in `hay` that are a whole path or its start (`goes_on`), without
/// case when `ci`.
fn occurrences(hay: &str, needle: &str, ci: bool) -> Vec<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return Vec::new();
    }
    let (h, n) = if ci {
        (hay.to_ascii_lowercase(), needle.to_ascii_lowercase())
    } else {
        (hay.to_string(), needle.to_string())
    };
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = h[from..].find(&n) {
        let at = from + i;
        let end = at + n.len();
        if !goes_on(&hay[end..]) {
            out.push(at);
            from = end;
        } else {
            from = at + 1;
            while !h.is_char_boundary(from) {
                from += 1;
            }
        }
    }
    out
}

/// `s` with each occurrence of each old form made the new one; how many.
fn replaced(s: &str, forms: &[(String, String)], ci: bool) -> (String, usize) {
    let mut out = s.to_string();
    let mut n = 0;
    for (from, to) in forms {
        let at = occurrences(&out, from, ci);
        if at.is_empty() {
            continue;
        }
        let mut next = String::with_capacity(out.len());
        let mut last = 0;
        for i in at {
            next.push_str(&out[last..i]);
            next.push_str(to);
            last = i + from.len();
            n += 1;
        }
        next.push_str(&out[last..]);
        out = next;
    }
    (out, n)
}

/// Whether an old form is left in `s`, once the new worktree's own are set aside (a new worktree under the old one).
fn left_in(s: &str, forms: &[(String, String)], ci: bool) -> bool {
    let mut rest = s.to_string();
    for (_, to) in forms {
        let (r, _) = replaced(&rest, &[(to.clone(), "\u{0}".into())], ci);
        rest = r;
    }
    forms
        .iter()
        .any(|(from, _)| !occurrences(&rest, from, ci).is_empty())
}

/// Each string of `v`, keys too, through `f`.
fn walk(v: &mut Value, f: &mut dyn FnMut(&str) -> Option<String>) {
    match v {
        Value::String(s) => {
            if let Some(n) = f(s) {
                *s = n;
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| walk(x, f)),
        Value::Object(m) => {
            let old = std::mem::take(m);
            for (k, mut x) in old {
                walk(&mut x, f);
                m.insert(f(&k).unwrap_or(k), x);
            }
        }
        _ => {}
    }
}

/// Whether some string of `v`, or key, holds an old form.
fn any_left(v: &Value, forms: &[(String, String)], ci: bool) -> bool {
    match v {
        Value::String(s) => left_in(s, forms, ci),
        Value::Array(a) => a.iter().any(|x| any_left(x, forms, ci)),
        Value::Object(m) => m
            .iter()
            .any(|(k, x)| left_in(k, forms, ci) || any_left(x, forms, ci)),
        _ => false,
    }
}

/// The session's lines with the worktree `from` made `to` in each string of each JSON object of each line, under both
/// separators (and as Git Bash writes a path of Windows); how many paths were rewritten. Refused when a line is not
/// JSON, or when the old worktree is left in any form.
pub(crate) fn rewrite(lines: &str, from: &str, to: &str) -> Result<(String, usize), NotNative> {
    let forms = forms(from, to);
    let ci = windows(from);
    let same = forms.iter().all(|(f, t)| f.eq_ignore_ascii_case(t));
    let mut out = Vec::new();
    let mut n = 0;
    for line in lines.split('\n') {
        let body = line.trim_end_matches('\r');
        if body.trim().is_empty() {
            out.push(body.to_string());
            continue;
        }
        let mut v: Value = serde_json::from_str(body).map_err(|_| NotNative::Unreadable)?;
        if same {
            out.push(body.to_string());
            continue;
        }
        let mut here = 0;
        walk(&mut v, &mut |s| {
            let (r, k) = replaced(s, &forms, ci);
            here += k;
            (k > 0).then_some(r)
        });
        n += here;
        // A line without the old worktree stays as Claude Code wrote it, byte for byte.
        out.push(if here == 0 {
            body.to_string()
        } else {
            serde_json::to_string(&v).map_err(|_| NotNative::Unreadable)?
        });
    }
    let text = out.join("\n");
    if !same {
        verify(&text, from, to)?;
    }
    Ok((text, n))
}

/// The check after the rewrite: no occurrence of the old worktree `from` is left in the session, in any form (`\`, `/`,
/// as Git Bash writes it, its `\` doubled or not), in a string, a key or the raw text of a line; those of the new
/// worktree `to` set aside (a new worktree under the old one). Refused otherwise: nothing is placed.
pub(crate) fn verify(text: &str, from: &str, to: &str) -> Result<(), NotNative> {
    let forms = forms(from, to);
    let ci = windows(from);
    let escaped: Vec<(String, String)> = forms
        .iter()
        .map(|(f, t)| (f.replace('\\', "\\\\"), t.replace('\\', "\\\\")))
        .collect();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|_| NotNative::Unreadable)?;
        if any_left(&v, &forms, ci) || left_in(line, &escaped, ci) || left_in(line, &forms, ci) {
            return Err(NotNative::PathLeft);
        }
    }
    Ok(())
}

/// An id Claude Code gives a session (a UUID, `sess-…` for the fake): letters, digits, `-` and `_`, never a path.
fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Where Claude Code looks the session `id` of the folder `cwd` up, in the account's folder `config`.
pub(crate) fn session_path(config: &Path, cwd: &str, id: &str) -> PathBuf {
    config
        .join("projects")
        .join(encoded(cwd))
        .join(format!("{id}.jsonl"))
}

/// The session's file on the machine of who passes it: where Claude Code wrote it for its folder, else wherever in the
/// account's folder it is (a worktree it went into).
pub(crate) fn transcript_of(config: &Path, cwd: &str, id: &str) -> Option<PathBuf> {
    if !id_ok(id) {
        return None;
    }
    let exact = session_path(config, cwd, id);
    if exact.is_file() {
        return Some(exact);
    }
    crate::tui::find_transcript(config, id)
}

/// What a session passed becomes here, before the agent starts: its version checked, its paths rewritten and checked,
/// then placed in the account's folder `config` under the worktree `cwd`. Its path, to take it out on a failure.
pub(crate) fn place(
    config: &Path,
    cwd: &str,
    copy: &SessionCopy,
    mine: Option<&str>,
) -> Result<PathBuf, NotNative> {
    if !id_ok(&copy.id) {
        return Err(NotNative::BadId);
    }
    version_ok(mine, &copy.version)?;
    let (text, _) = rewrite(&copy.lines, &copy.worktree, cwd)?;
    let path = session_path(config, cwd, &copy.id);
    if path.exists() {
        return Err(NotNative::Taken);
    }
    let io = |e: std::io::Error| NotNative::Io(e.to_string());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(io)?;
    let mut body = text;
    if !body.ends_with('\n') {
        body.push('\n');
    }
    f.write_all(body.as_bytes()).map_err(io)?;
    Ok(path)
}

/// The copy placed is taken out of the account's folder, its folder too when nothing else is in it.
pub(crate) fn remove(path: &Path) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            log::warn!("the session copied stays in the account's folder: {e}");
        }
    }
    if let Some(dir) = path.parent() {
        // Only when empty: `remove_dir` refuses a folder with something in it.
        let _ = std::fs::remove_dir(dir);
    }
}

#[cfg(test)]
#[path = "sessioncopy_tests.rs"]
mod tests;
