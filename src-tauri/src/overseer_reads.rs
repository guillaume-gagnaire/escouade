//! What the Overseer may grant, and read, without the user (ESC-646, SR4-02 and SR4-03 of the 2.3's security review):
//! reads alone, of places named. Never a command, a write, the web or a tool of another server; never a harness's
//! credentials nor a key, whatever folder holds them. A place is judged by where it lands, its links and junctions
//! followed: a link of an allowed folder that leads out of it leads out.

use serde_json::Value;
use std::path::{Component, Path, PathBuf};

/// The requests of an agent the Overseer may allow: reads and searches of files, in the folder of the agent's project
/// or of its worktree. Anything else waits for the user.
pub const AGENT_READS: [&str; 3] = ["Read", "Grep", "Glob"];

/// The reads the Overseer's own harness asks for that go through without a question, in its places.
pub const OWN_READS: [&str; 4] = ["Read", "Grep", "Glob", "LS"];

/// Why a read is not granted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// No read: a command, a write, the web, another server's tool.
    NotRead,
    /// A read whose place cannot be told for sure (no path, a `..` after a wildcard, a link that leads nowhere).
    Unjudged,
    /// A place that holds credentials or keys.
    Secret(PathBuf),
    /// A place out of the folders allowed.
    Outside(PathBuf),
}

/// Under the user's folder, what holds a harness's credentials, a key or the system's keychain: never read by the
/// Overseer, nor granted by it, even when an allowed folder holds it.
const SECRET_DIRS: &[&str] = &[
    ".claude",
    ".claude.json",
    ".codex",
    ".copilot",
    ".config/github-copilot",
    ".config/gh",
    ".config/opencode",
    ".local/share/opencode",
    ".cursor",
    ".gemini",
    ".vibe",
    ".escouade-signing",
    ".ssh",
    ".gnupg",
    ".aws",
    ".azure",
    ".docker",
    ".kube",
    ".git-credentials",
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".cargo/credentials",
    ".cargo/credentials.toml",
    "Library/Keychains",
    ".local/share/keyrings",
    "AppData/Roaming/Microsoft/Credentials",
    "AppData/Local/Microsoft/Credentials",
    "AppData/Roaming/Microsoft/Protect",
    "AppData/Roaming/GitHub CLI",
];

/// The names of files that hold credentials or keys, wherever they are.
fn secret_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    matches!(
        name.as_str(),
        ".credentials.json"
            | "credentials.json"
            | "auth.json"
            | ".env"
            | "id_rsa"
            | "id_ed25519"
            | "id_ecdsa"
    ) || name.starts_with(".env.")
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
        || [".key", ".pem", ".p12", ".pfx", ".keychain", ".keychain-db"]
            .iter()
            .any(|ext| name.ends_with(ext))
}

/// `path` as it lands, its links followed as far as it exists (what is left of it appended); None for a link that
/// leads nowhere, or a `..` past what exists.
pub fn landing(path: &Path) -> Option<PathBuf> {
    crate::plugins::files::resolved(path)
}

/// Whether `real` (a path as it lands) is a place of credentials or keys.
pub fn secret(real: &Path, home: Option<&Path>) -> bool {
    if real
        .components()
        .any(|c| matches!(c, Component::Normal(n) if secret_name(&n.to_string_lossy())))
    {
        return true;
    }
    let Some(home) = home.and_then(landing) else {
        return false;
    };
    let lower = |p: &Path| p.to_string_lossy().replace('\\', "/").to_lowercase();
    let at = lower(real);
    SECRET_DIRS.iter().any(|d| {
        let dir = lower(&home.join(d));
        at == dir || at.starts_with(&format!("{dir}/"))
    })
}

/// `p` made absolute: `~` is the user's folder, a relative path is under `cwd`.
fn absolute(p: &str, cwd: &Path, home: Option<&Path>) -> Option<PathBuf> {
    let p = p.trim();
    if p.is_empty() || p.contains('\0') {
        return None;
    }
    if p == "~" || p.starts_with("~/") || p.starts_with("~\\") {
        return Some(home?.join(p[1..].trim_start_matches(['/', '\\'])));
    }
    if p.starts_with('~') {
        return None;
    }
    let path = Path::new(p);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    })
}

fn text<'a>(input: &'a Value, key: &str) -> Option<&'a str> {
    input
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
}

/// The places a read of `tool` asks for, made absolute against `cwd`: `Err(NotRead)` for a tool of no read of `reads`,
/// `Err(Unjudged)` when its input does not say a place for sure.
pub fn asked_places(
    reads: &[&str],
    tool: &str,
    input: &Value,
    cwd: &Path,
    home: Option<&Path>,
) -> Result<Vec<PathBuf>, Refusal> {
    if !reads.contains(&tool) {
        return Err(Refusal::NotRead);
    }
    let at = |key: &str| -> Result<PathBuf, Refusal> {
        match text(input, key) {
            Some(p) => absolute(p, cwd, home).ok_or(Refusal::Unjudged),
            None => Ok(cwd.to_path_buf()),
        }
    };
    match tool {
        "Read" => Ok(vec![absolute(
            text(input, "file_path").ok_or(Refusal::Unjudged)?,
            cwd,
            home,
        )
        .ok_or(Refusal::Unjudged)?]),
        "LS" => Ok(vec![absolute(
            text(input, "path").ok_or(Refusal::Unjudged)?,
            cwd,
            home,
        )
        .ok_or(Refusal::Unjudged)?]),
        "Grep" => {
            // Its filter of files stays under its folder: none absolute, none that climbs.
            if let Some(glob) = text(input, "glob") {
                if Path::new(glob).is_absolute() || climbs(glob) || glob.starts_with('~') {
                    return Err(Refusal::Unjudged);
                }
            }
            Ok(vec![at("path")?])
        }
        "Glob" => {
            let base = at("path")?;
            let pattern = text(input, "pattern").ok_or(Refusal::Unjudged)?;
            Ok(vec![glob_base(&base, pattern, home)?])
        }
        _ => Err(Refusal::NotRead),
    }
}

/// Whether a pattern climbs (`..` as one of its parts).
fn climbs(pattern: &str) -> bool {
    pattern.split(['/', '\\']).any(|part| part == "..")
}

/// The folder a glob pattern searches under: its parts before the first that holds a wildcard, under `base` (or the
/// pattern's own root when it is absolute). A `..` after a wildcard is not judged.
fn glob_base(base: &Path, pattern: &str, home: Option<&Path>) -> Result<PathBuf, Refusal> {
    let parts: Vec<&str> = pattern.split(['/', '\\']).collect();
    let wild = |p: &str| p.contains(['*', '?', '[', ']', '{', '}']);
    let first = parts.iter().position(|p| wild(p)).unwrap_or(parts.len());
    if parts[first..].contains(&"..") {
        return Err(Refusal::Unjudged);
    }
    let literal = parts[..first].join("/");
    if literal.is_empty() {
        // `/**` on Unix: from the root.
        return Ok(if pattern.starts_with(['/', '\\']) {
            PathBuf::from("/")
        } else {
            base.to_path_buf()
        });
    }
    absolute(&literal, base, home).ok_or(Refusal::Unjudged)
}

/// Whether the read `tool` with `input`, under `cwd`, lands in one of `roots` and in no place of secrets: what the
/// Overseer may grant (`AGENT_READS`) or read of its own (`OWN_READS`).
pub fn read_within(
    reads: &[&str],
    tool: &str,
    input: &Value,
    cwd: &Path,
    roots: &[PathBuf],
    home: Option<&Path>,
) -> Result<(), Refusal> {
    let places = asked_places(reads, tool, input, cwd, home)?;
    let roots: Vec<PathBuf> = roots
        .iter()
        .filter(|r| !r.as_os_str().is_empty())
        .filter_map(|r| landing(r))
        .collect();
    for place in places {
        let real = landing(&place).ok_or(Refusal::Unjudged)?;
        if secret(&real, home) {
            return Err(Refusal::Secret(place));
        }
        if !roots.iter().any(|r| real.starts_with(r)) {
            return Err(Refusal::Outside(place));
        }
    }
    Ok(())
}
