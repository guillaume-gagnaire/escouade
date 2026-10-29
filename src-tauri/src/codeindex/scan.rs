//! Reading the files again: what changed since the last time, and only that.

use super::lang::{self, Lang};
use super::store;
use crate::fence::Fence;
use anyhow::Result;
use rusqlite::{params, Connection, Transaction};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

/// A bigger file is generated or data, not code worth cutting.
const MAX_FILE: u64 = 400 * 1024;

/// What one reading did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Refresh {
    /// Files of the language the index knows, listed.
    pub seen: usize,
    /// Files read and cut again.
    pub parsed: usize,
    /// Files whose rows went (removed or no longer of this scope).
    pub dropped: usize,
    pub millis: u128,
}

impl Refresh {
    pub fn merge(&mut self, o: Refresh) {
        self.seen += o.seen;
        self.parsed += o.parsed;
        self.dropped += o.dropped;
        self.millis += o.millis;
    }
}

struct Known {
    id: i64,
    size: i64,
    mtime: i64,
    hash: String,
    state: i64,
}

/// Brings `scope` (`""` for the project's folder) up to date with `dir`.
pub fn refresh(
    conn: &mut Connection,
    scope: &str,
    dir: &Path,
    fence: &Fence,
    exclude: &[PathBuf],
) -> Result<Refresh> {
    let started = Instant::now();
    store::prune(conn)?;
    let listed = candidates(dir, fence, exclude);
    let mut known: HashMap<String, Known> = HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT id, path, size, mtime, hash, state FROM files WHERE scope = ?1")?;
        let rows = stmt.query_map([scope], |r| {
            Ok((
                r.get::<_, String>(1)?,
                Known {
                    id: r.get(0)?,
                    size: r.get(2)?,
                    mtime: r.get(3)?,
                    hash: r.get(4)?,
                    state: r.get(5)?,
                },
            ))
        })?;
        for row in rows.flatten() {
            known.insert(row.0, row.1);
        }
    }
    let base: HashMap<String, String> = if scope.is_empty() {
        HashMap::new()
    } else {
        let mut stmt = conn.prepare("SELECT path, hash FROM files WHERE scope = ''")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.flatten().collect()
    };
    let tx = conn.transaction()?;
    let mut out = Refresh::default();
    let mut present: HashSet<String> = HashSet::new();
    for (rel, lang) in listed {
        let full = dir.join(&rel);
        let Ok(meta) = std::fs::metadata(&full) else {
            continue;
        };
        if !meta.is_file() || meta.len() > MAX_FILE {
            continue;
        }
        let size = meta.len() as i64;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);
        let prev = known.get(&rel);
        let unchanged = prev.is_some_and(|k| k.size == size && k.mtime == mtime && k.state != 2);
        let mut bytes: Option<Vec<u8>> = None;
        let hash = match prev.filter(|_| unchanged) {
            Some(k) => k.hash.clone(),
            None => {
                let Ok(b) = std::fs::read(&full) else {
                    continue;
                };
                // A binary file in a source extension.
                if b.contains(&0) {
                    continue;
                }
                let h = hash_of(&b);
                bytes = Some(b);
                h
            }
        };
        out.seen += 1;
        present.insert(rel.clone());
        // What it holds of its own: everything in the project's folder, only what differs in a worktree.
        let own = scope.is_empty() || base.get(&rel).map(String::as_str) != Some(hash.as_str());
        let state = if scope.is_empty() || !own { 0 } else { 1 };
        let had_symbols = prev.is_some_and(|k| scope.is_empty() || k.state == 1);
        let needs_parse = own && (!unchanged || !had_symbols);
        if !needs_parse {
            if let Some(k) = prev {
                if !own && had_symbols {
                    clear_rows(&tx, k.id)?;
                    out.dropped += 1;
                }
                if k.hash != hash || k.size != size || k.mtime != mtime || k.state != state {
                    tx.execute(
                        "UPDATE files SET size = ?2, mtime = ?3, hash = ?4, state = ?5 WHERE id = ?1",
                        params![k.id, size, mtime, hash, state],
                    )?;
                }
            } else {
                insert_file(&tx, scope, &rel, lang, size, mtime, &hash, state)?;
            }
            continue;
        }
        let bytes = match bytes {
            Some(b) => b,
            None => match std::fs::read(&full) {
                Ok(b) => b,
                Err(_) => continue,
            },
        };
        let src = String::from_utf8_lossy(&bytes);
        let parsed = lang::parse(lang, &rel, &src);
        let file_id = match prev {
            Some(k) => {
                clear_rows(&tx, k.id)?;
                tx.execute(
                    "UPDATE files SET size = ?2, mtime = ?3, hash = ?4, state = ?5, lang = ?6 WHERE id = ?1",
                    params![k.id, size, mtime, hash, state, lang.name()],
                )?;
                k.id
            }
            None => insert_file(&tx, scope, &rel, lang, size, mtime, &hash, state)?,
        };
        store_parsed(&tx, scope, &rel, file_id, parsed, &src)?;
        out.parsed += 1;
    }
    // Files gone.
    for (rel, k) in &known {
        if !present.contains(rel) && k.state != 2 {
            clear_rows(&tx, k.id)?;
            tx.execute("DELETE FROM files WHERE id = ?1", [k.id])?;
            out.dropped += 1;
        }
    }
    // A worktree knows the files it removed: the project's folder's symbols are not its.
    if !scope.is_empty() {
        for rel in base.keys() {
            let gone = !present.contains(rel);
            match (gone, known.get(rel)) {
                (true, None) => {
                    insert_file(
                        &tx,
                        scope,
                        rel,
                        Lang::of(rel).unwrap_or(Lang::Markdown),
                        0,
                        0,
                        "",
                        2,
                    )?;
                }
                (true, Some(k)) if k.state != 2 => {
                    // Already dropped above: the marker takes its place.
                    insert_file(
                        &tx,
                        scope,
                        rel,
                        Lang::of(rel).unwrap_or(Lang::Markdown),
                        0,
                        0,
                        "",
                        2,
                    )?;
                }
                _ => {}
            }
        }
        for (rel, k) in &known {
            if k.state == 2 && !base.contains_key(rel) {
                tx.execute("DELETE FROM files WHERE id = ?1", [k.id])?;
            }
        }
    }
    let now = chrono::Utc::now().timestamp_millis();
    store::touch_scope(&tx, scope, dir, now)?;
    tx.commit()?;
    out.millis = started.elapsed().as_millis();
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn insert_file(
    tx: &Transaction,
    scope: &str,
    rel: &str,
    lang: Lang,
    size: i64,
    mtime: i64,
    hash: &str,
    state: i64,
) -> Result<i64> {
    tx.execute(
        "INSERT OR REPLACE INTO files (scope, path, lang, size, mtime, hash, state) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![scope, rel, lang.name(), size, mtime, hash, state],
    )?;
    Ok(tx.last_insert_rowid())
}

/// The symbols and links of a file, not its row.
fn clear_rows(tx: &Transaction, file_id: i64) -> Result<()> {
    for fts in ["sym_fts", "body_fts"] {
        tx.execute(
            &format!(
                "DELETE FROM {fts} WHERE rowid IN (SELECT id FROM symbols WHERE file_id = ?1)"
            ),
            [file_id],
        )?;
    }
    tx.execute("DELETE FROM edges WHERE file_id = ?1", [file_id])?;
    tx.execute("DELETE FROM symbols WHERE file_id = ?1", [file_id])?;
    Ok(())
}

fn store_parsed(
    tx: &Transaction,
    scope: &str,
    rel: &str,
    file_id: i64,
    parsed: lang::Parsed,
    src: &str,
) -> Result<()> {
    let lines: Vec<&str> = src.lines().collect();
    let mut ids: Vec<i64> = Vec::with_capacity(parsed.symbols.len());
    {
        let mut sym = tx.prepare_cached(
            "INSERT INTO symbols (file_id, scope, path, name, kind, parent, line, end_line, sig, doc, vis)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )?;
        let mut fts = tx.prepare_cached(
            "INSERT INTO sym_fts (rowid, name, qualified, doc, sig, terms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        let mut body =
            tx.prepare_cached("INSERT INTO body_fts (rowid, path, body) VALUES (?1, ?2, ?3)")?;
        for s in &parsed.symbols {
            sym.execute(params![
                file_id, scope, rel, s.name, s.kind, s.parent, s.line, s.end_line, s.sig, s.doc,
                s.vis
            ])?;
            let id = tx.last_insert_rowid();
            ids.push(id);
            let qualified = super::api::qualified(rel, &s.parent, &s.name);
            fts.execute(params![
                id,
                s.name,
                qualified,
                s.doc,
                s.sig,
                super::text::search_terms(&s.name, &qualified)
            ])?;
            // tokensave `build_executable_body_documents`: the lines of each function and method.
            if super::ranking::is_executable_kind(s.kind) {
                let start = s.line.saturating_sub(1) as usize;
                let end = (s.end_line as usize).min(lines.len());
                if start < end {
                    body.execute(params![id, rel, lines[start..end].join("\n")])?;
                }
            }
        }
    }
    let mut edge = tx.prepare_cached(
        "INSERT INTO edges (file_id, scope, path, from_id, kind, target, line) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for e in &parsed.edges {
        let from = e.from.and_then(|i| ids.get(i)).copied();
        edge.execute(params![file_id, scope, rel, from, e.kind, e.target, e.line])?;
    }
    Ok(())
}

fn hash_of(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .take(12)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The files of the languages the index knows that the project has, by what git lists (tracked, and
/// untracked but not ignored), else by walking the folder; those the fence refuses or that are
/// under `exclude` left out. Paths from the folder with `/`.
fn candidates(dir: &Path, fence: &Fence, exclude: &[PathBuf]) -> Vec<(String, Lang)> {
    let listed = git_files(dir).unwrap_or_else(|| walk(dir));
    let out_of = Fence::new(exclude.to_vec());
    let mut seen = HashSet::new();
    listed
        .into_iter()
        .filter(|rel| !rel.ends_with('/') && seen.insert(rel.clone()))
        .filter_map(|rel| Lang::of(&rel).map(|l| (rel, l)))
        .filter(|(rel, _)| !skipped(rel))
        .filter(|(rel, _)| {
            if fence.is_empty() && out_of.is_empty() {
                return true;
            }
            let full = dir.join(rel).to_string_lossy().to_string();
            !fence.covers(&full) && !out_of.covers(&full)
        })
        .collect()
}

fn skipped(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.contains(".min.")
        || lower.ends_with(".d.ts")
        || lower.split('/').any(|c| {
            matches!(
                c,
                "node_modules" | "target" | "dist" | "vendor" | "__pycache__" | ".git"
            )
        })
}

fn git_files(dir: &Path) -> Option<Vec<String>> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C")
        .arg(dir)
        .args([
            "--no-optional-locks",
            "-c",
            "core.quotepath=false",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

/// A folder that is no git repository: walked, the hidden folders and the usual build ones left out.
fn walk(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((path, prefix)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            let rel = format!("{prefix}{name}");
            if ft.is_dir() {
                if name.starts_with('.')
                    || matches!(name.as_str(), "node_modules" | "target" | "dist")
                {
                    continue;
                }
                stack.push((entry.path(), format!("{rel}/")));
            } else {
                out.push(rel);
            }
        }
    }
    out
}
