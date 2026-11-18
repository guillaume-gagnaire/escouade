//! The hosts of the user's `~/.ssh/config` (ESC-537, `docs/SERVER-UX.md` § 2, step 3): what « Se connecter à un
//! serveur » proposes. Read for the list alone: the system's `ssh` reads the file itself when it connects, and its keys,
//! its agent and its `known_hosts` are never touched here.
//!
//! What it understands, as `ssh_config(5)` says it:
//! - `Host` blocks (several names on a line), their `HostName`, `Port`, `User` and `ProxyJump`; a keyword is read in any
//!   case, `Keyword value` or `Keyword=value`, a value in double quotes kept whole; the first value given for a host
//!   wins, whichever block gives it (`Host *` at the end gives what the others left);
//! - `Include` (relative to `~/.ssh`, with `*` and `?`, several files on a line, 16 levels at most), read where it
//!   stands;
//! - `Match` blocks are skipped (what they set depends on the connection).
//!
//! A host is proposed when its name is a name (no `*`, `?` nor `!`) and a `HostName` is set for it: a pattern is not a
//! machine, and a name without `HostName` is the address itself, typed as well in « Ou un autre hôte ».

use serde::Serialize;
use std::path::{Path, PathBuf};

/// A host of the config, as the dialog lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHost {
    /// Its name in the config (`acme-escouade`): what `ssh` is given.
    pub alias: String,
    pub host_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_jump: Option<String>,
}

/// The most files read through `Include`, and how deep.
const MAX_FILES: usize = 64;
const MAX_DEPTH: usize = 16;
/// The largest config file read.
const MAX_SIZE: u64 = 1 << 20;

/// A line of a block: its keyword in lower case and its value.
#[derive(Debug, Clone)]
struct Setting {
    key: String,
    value: String,
}

#[derive(Debug, Clone)]
enum Block {
    /// `Host` with its patterns.
    Host(Vec<String>),
    /// `Match`: skipped.
    Match,
}

/// The config of `file` (`~/.ssh/config`): the hosts it names, in their order.
pub fn hosts(file: &Path) -> Vec<SshHost> {
    let ssh_dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut blocks: Vec<(Block, Vec<Setting>)> = vec![(Block::Host(vec!["*".into()]), Vec::new())];
    let mut files = 0usize;
    read_file(file, &ssh_dir, 0, &mut files, &mut blocks);
    // The names, in the order they come.
    let mut names: Vec<String> = Vec::new();
    for (block, _) in &blocks {
        if let Block::Host(patterns) = block {
            for p in patterns {
                let concrete = !p.contains(['*', '?', '!']) && !p.is_empty();
                if concrete && !names.iter().any(|n| n.eq_ignore_ascii_case(p)) {
                    names.push(p.clone());
                }
            }
        }
    }
    names
        .into_iter()
        .filter_map(|alias| {
            let first = |key: &str| first_value(&blocks, &alias, key);
            let host_name = first("hostname")?;
            let host_name = host_name.replace("%h", &alias);
            Some(SshHost {
                port: first("port").and_then(|p| p.parse().ok()),
                user: first("user"),
                proxy_jump: first("proxyjump").filter(|p| !p.eq_ignore_ascii_case("none")),
                alias,
                host_name,
            })
        })
        .collect()
}

/// The first value of `key` the blocks give `alias`, as `ssh` reads them (the first one wins).
fn first_value(blocks: &[(Block, Vec<Setting>)], alias: &str, key: &str) -> Option<String> {
    for (block, settings) in blocks {
        let Block::Host(patterns) = block else {
            continue;
        };
        if !host_matches(patterns, alias) {
            continue;
        }
        if let Some(s) = settings.iter().find(|s| s.key == key) {
            return Some(s.value.clone());
        }
    }
    None
}

/// Whether `alias` is one of `patterns` (`*`, `?`, a `!` before a pattern refuses it whatever the others say).
pub fn host_matches(patterns: &[String], alias: &str) -> bool {
    let alias = alias.to_ascii_lowercase();
    let mut matched = false;
    for p in patterns {
        let (negated, p) = match p.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, p.as_str()),
        };
        if glob(&p.to_ascii_lowercase(), &alias) {
            if negated {
                return false;
            }
            matched = true;
        }
    }
    matched
}

/// `*` (any run) and `?` (one character), as `ssh` reads them.
pub fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// The words of a line: `Keyword value`, `Keyword=value`, the values in double quotes kept whole.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut had = false;
    for (i, c) in line.chars().enumerate() {
        match c {
            '"' => {
                quoted = !quoted;
                had = true;
            }
            '=' if !quoted && out.is_empty() && i > 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                had = false;
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() || had {
                    out.push(std::mem::take(&mut cur));
                }
                had = false;
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() || had {
        out.push(cur);
    }
    out
}

fn read_file(
    file: &Path,
    ssh_dir: &Path,
    depth: usize,
    files: &mut usize,
    blocks: &mut Vec<(Block, Vec<Setting>)>,
) {
    if depth > MAX_DEPTH || *files >= MAX_FILES {
        return;
    }
    *files += 1;
    let fits = std::fs::metadata(file).is_ok_and(|m| m.is_file() && m.len() <= MAX_SIZE);
    if !fits {
        return;
    }
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let w = words(line);
        let Some((key, rest)) = w.split_first() else {
            continue;
        };
        let key = key.to_ascii_lowercase();
        match key.as_str() {
            "host" => blocks.push((Block::Host(rest.to_vec()), Vec::new())),
            "match" => blocks.push((Block::Match, Vec::new())),
            "include" => {
                for pattern in rest {
                    for included in included_files(pattern, ssh_dir) {
                        read_file(&included, ssh_dir, depth + 1, files, blocks);
                    }
                }
            }
            _ => {
                if let (Some(value), Some((_, settings))) = (rest.first(), blocks.last_mut()) {
                    settings.push(Setting {
                        key,
                        value: value.clone(),
                    });
                }
            }
        }
    }
}

/// The files of an `Include`: `~/` the home folder, a relative path under `~/.ssh`, `*` and `?` in its last part.
fn included_files(pattern: &str, ssh_dir: &Path) -> Vec<PathBuf> {
    let path = if let Some(rest) = pattern.strip_prefix("~/") {
        ssh_dir
            .parent()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(rest))
    } else {
        let p = PathBuf::from(pattern);
        if p.is_absolute() {
            p
        } else {
            ssh_dir.join(p)
        }
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !name.contains(['*', '?']) {
        return vec![path];
    }
    let Some(dir) = path.parent() else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| glob(&name, &e.file_name().to_string_lossy()))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    // As `ssh` does: in the order of their names.
    found.sort();
    found
}

/// The user's `~/.ssh/config` (in a build of development, `ESCOUADE_SSH_CONFIG` for the tests).
pub fn user_config() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        if let Some(p) = std::env::var_os("ESCOUADE_SSH_CONFIG").filter(|v| !v.is_empty()) {
            return Some(PathBuf::from(p));
        }
    }
    dirs::home_dir().map(|h| h.join(".ssh").join("config"))
}
