//! « Index du code » (ESC-251): the structure of a project's code — symbols, imports, calls,
//! inheritance, Markdown headings — cut by tree-sitter into a SQLite database with full-text search
//! (FTS5), in Escouade's data folder, never in the repository. The agents ask it, through the MCP
//! server (another ticket), instead of reading twenty files (`api`).
//!
//! **Always up to date, with no file watcher** (tokensave dropped its own: processor and memory ran
//! away on big repositories). Three things keep it fresh:
//! - each query checks the files first (`Service::reader`): the list git gives (`ls-files`), a
//!   `stat` of each, a re-reading of those that changed alone; at most once per `MIN_GAP`;
//! - the git events Escouade already sees — a commit, a change of branch, the end of an agent's
//!   turn — refresh it at once (`Service::ensure_fresh` with `force`);
//! - nothing else: a folder nobody asks about is not read.
//!
//! **Worktrees**: the project's folder is indexed whole (scope `""`); a worktree keeps only what
//! differs from it (scope `wt-…`): a file of the same content has no symbols of its own, a file
//! changed, added or removed has (`files.state`). A query in a worktree sees the base but for those.
//!
//! **Private projects**: the files `fence` refuses are never listed, read nor indexed
//! (`Target::fence`).

// A few parts of the API (the statuses, some counts) are read by the tests only.
#![allow(dead_code)]

mod api;
mod calls;
mod context;
pub mod hint;
mod lang;
mod links;
pub(crate) mod probes;
mod ranking;
mod receivers;
mod resolver;
mod scan;
mod search;
mod store;
mod text;
mod wiring;

#[cfg(test)]
pub(crate) use api::Context;
pub use api::{Answer, Reader, Retrieval};
pub use calls::{Wanted, MAX_MATCHES_DEFAULT};
pub use context::BuildContextOptions;
#[cfg(test)]
pub(crate) use lang::{Parsed, Sym};
pub use scan::Refresh;
pub use wiring::CodeIndexInfo;

use crate::fence::Fence;
use crate::paths::DataDir;
use anyhow::Result;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The least time between two checks of the files made by queries.
pub const MIN_GAP: Duration = Duration::from_secs(3);

/// What a query is about: a project, and the folder the agent works in (the project's own, or a
/// worktree of it).
#[derive(Debug, Clone)]
pub struct Target {
    pub project_id: String,
    /// The project's folder.
    pub root: PathBuf,
    /// The folder the agent works in: `root`, or a worktree.
    pub dir: PathBuf,
    /// What the project's privacy refuses: never listed, read nor indexed.
    pub fence: Fence,
    /// Folders under `root` that are no part of it (the worktrees).
    pub exclude: Vec<PathBuf>,
}

impl Target {
    /// The scope of the folder: `""` for the project's own, a name of the worktree's else.
    pub fn scope(&self) -> String {
        if crate::paths::same_folder(&self.root.to_string_lossy(), &self.dir.to_string_lossy()) {
            String::new()
        } else {
            store::scope_of(&self.dir)
        }
    }
}

/// What the index holds, for the settings.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub files: u64,
    pub symbols: u64,
    pub links: u64,
    /// Worktrees with a delta.
    pub worktrees: u64,
    /// The database's size on the disk.
    pub bytes: u64,
    /// When the project's folder was last read, in ms; 0 for never.
    pub indexed_at: i64,
}

#[derive(Default)]
pub struct Service {
    data: Option<DataDir>,
    gaps: Mutex<HashMap<String, Instant>>,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    /// The projects « Index du code sur tous les projets » has yet to read (ESC-293), one at a
    /// time: the first is being read, it leaves the list once read.
    pub(crate) queue: Mutex<Vec<String>>,
    /// A task reads the list.
    pub(crate) draining: std::sync::atomic::AtomicBool,
}

impl Service {
    pub fn new(data: DataDir) -> Self {
        Self {
            data: Some(data),
            ..Default::default()
        }
    }

    fn db(&self, project_id: &str) -> PathBuf {
        self.data
            .as_ref()
            .map(|d| d.code_index_db(project_id))
            .unwrap_or_default()
    }

    fn lock(&self, project_id: &str) -> Arc<Mutex<()>> {
        self.locks
            .lock()
            .entry(project_id.to_string())
            .or_default()
            .clone()
    }

    /// The project's lock, for a test to hold its reading.
    #[cfg(test)]
    pub(crate) fn lock_for_tests(&self, project_id: &str) -> Arc<Mutex<()>> {
        self.lock(project_id)
    }

    /// Brings the index of the target up to date: the project's folder, then the worktree's delta.
    /// Without `force`, nothing is read when the last check is less than `MIN_GAP` old. None when
    /// nothing was checked.
    pub fn ensure_fresh(&self, t: &Target, force: bool) -> Result<Option<Refresh>> {
        let scope = t.scope();
        let key = format!("{}/{}", t.project_id, scope);
        let lock = self.lock(&t.project_id);
        let _guard = lock.lock();
        let db = self.db(&t.project_id);
        let known = db.exists();
        if !force && known {
            if let Some(at) = self.gaps.lock().get(&key) {
                if at.elapsed() < MIN_GAP {
                    return Ok(None);
                }
            }
        }
        let mut conn = store::open(&db)?;
        let mut total = scan::refresh(&mut conn, "", &t.root, &t.fence, &t.exclude)?;
        if !scope.is_empty() {
            let more = scan::refresh(&mut conn, &scope, &t.dir, &t.fence, &[])?;
            total.merge(more);
        }
        let mut gaps = self.gaps.lock();
        gaps.insert(format!("{}/", t.project_id), Instant::now());
        gaps.insert(key, Instant::now());
        Ok(Some(total))
    }

    /// A reader of the index, the files checked first.
    pub fn reader(&self, t: &Target) -> Result<Reader> {
        self.ensure_fresh(t, false)?;
        let conn = store::open(&self.db(&t.project_id))?;
        Ok(Reader::new(conn, t.scope(), t.dir.clone(), t.fence.clone()))
    }

    pub fn status(&self, project_id: &str) -> Status {
        let db = self.db(project_id);
        if !db.exists() {
            return Status::default();
        }
        let lock = self.lock(project_id);
        let _guard = lock.lock();
        store::open(&db)
            .and_then(|c| store::status(&c, &db))
            .unwrap_or_default()
    }

    /// Erases the project's index (the setting turned off, the project removed).
    pub fn delete(&self, project_id: &str) {
        let lock = self.lock(project_id);
        let _guard = lock.lock();
        let db = self.db(project_id);
        for suffix in ["", "-wal", "-shm"] {
            let mut name = db.clone().into_os_string();
            name.push(suffix);
            let _ = std::fs::remove_file(PathBuf::from(name));
        }
        // A reader still holds the file (Windows keeps it): emptied, it goes with the reader.
        if db.exists() {
            if let Ok(conn) = store::open(&db) {
                let _ = store::wipe(&conn);
            }
        }
        self.gaps
            .lock()
            .retain(|k, _| !k.starts_with(&format!("{project_id}/")));
    }
}

/// Estimated tokens of a text: about four bytes each.
pub fn tokens_of(bytes: u64) -> u64 {
    bytes.div_ceil(4)
}

#[cfg(test)]
impl Service {
    /// The links and the ties a target sees, by qualified names (`from|to|kind|line`,
    /// `from|name|path|line|candidates`), sorted: what a resolution made, comparable by value.
    pub(crate) fn links_for_tests(&self, t: &Target) -> (Vec<String>, Vec<String>) {
        let conn = store::open(&self.db(&t.project_id)).unwrap();
        let scope = t.scope();
        let q = |id: Option<i64>| -> String {
            id.and_then(|id| {
                conn.query_row("SELECT qname FROM symbols WHERE id = ?1", [id], |r| {
                    r.get::<_, String>(0)
                })
                .ok()
            })
            .unwrap_or_else(|| "<top>".into())
        };
        let vis = |a: &str| {
            format!(
                "({a}.scope = ?1 OR ({a}.scope = '' AND NOT EXISTS (SELECT 1 FROM files fw \
                 WHERE fw.scope = ?1 AND fw.path = {a}.path AND fw.state IN (1, 2)) \
                 AND NOT EXISTS (SELECT 1 FROM overrides o WHERE o.scope = ?1 AND o.ref_id = {a}.ref_id)))"
            )
        };
        let mut stmt = conn
            .prepare(&format!(
                "SELECT l.from_id, l.to_id, l.kind, l.line FROM links l WHERE {}",
                vis("l")
            ))
            .unwrap();
        let rows: Vec<(Option<i64>, i64, String, u32)> = stmt
            .query_map([&scope], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .unwrap()
            .flatten()
            .collect();
        let mut links: Vec<String> = rows
            .into_iter()
            .map(|(f, to, k, l)| format!("{}|{}|{k}|{l}", q(f), q(Some(to))))
            .collect();
        links.sort();
        links.dedup();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT a.from_id, a.name, a.path, a.line, a.candidates FROM ambiguous a WHERE {}",
                vis("a")
            ))
            .unwrap();
        let rows: Vec<(Option<i64>, String, String, u32, String)> = stmt
            .query_map([&scope], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .flatten()
            .collect();
        let mut ties: Vec<String> = rows
            .into_iter()
            .map(|(f, n, p, l, c)| {
                let mut cands: Vec<String> = c
                    .split(',')
                    .filter_map(|s| s.parse().ok())
                    .map(|id| q(Some(id)))
                    .collect();
                cands.sort();
                format!("{}|{n}|{p}|{l}|{}", q(f), cands.join(","))
            })
            .collect();
        ties.sort();
        (links, ties)
    }

    /// Every link of the target made again from nothing (tokensave's `TOKENSAVE_FULL_RESOLVE`).
    pub(crate) fn resolve_again_for_tests(&self, t: &Target) {
        let mut conn = store::open(&self.db(&t.project_id)).unwrap();
        links::resolve_all_again(&mut conn, "").unwrap();
        let scope = t.scope();
        if !scope.is_empty() {
            links::resolve_all_again(&mut conn, &scope).unwrap();
        }
    }

    /// The symbols the resolver is built from, and those keys the touched set derives of them.
    pub(crate) fn known_names_for_tests(
        &self,
        project_id: &str,
    ) -> (
        std::collections::HashSet<String>,
        std::collections::HashSet<String>,
    ) {
        let conn = store::open(&self.db(project_id)).unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, qname, kind, path, line, vis, parent, ty FROM symbols WHERE scope = '' AND kind <> 'heading'")
            .unwrap();
        let nodes: Vec<resolver::Node> = stmt
            .query_map([], |r| {
                Ok(resolver::Node {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    qname: r.get(2)?,
                    kind: r.get(3)?,
                    path: r.get(4)?,
                    line: r.get(5)?,
                    vis: r.get::<_, String>(6)? == "pub",
                    parent: r.get(7)?,
                    ty: r.get(8)?,
                })
            })
            .unwrap()
            .flatten()
            .collect();
        let r = resolver::Resolver::new(&nodes, &Default::default(), &[]);
        let known = r.known_names().iter().map(|s| s.to_string()).collect();
        let mut derived = std::collections::HashSet::new();
        for n in &nodes {
            resolver::index_keys(&n.name, &n.qname, &mut derived);
        }
        (known, derived)
    }
}

/// A file cut, for the tests of each language.
#[cfg(test)]
pub(crate) fn lang_for_tests(path: &str, src: &str) -> Parsed {
    lang::parse(lang::Lang::of(path).expect("a known extension"), path, src)
}

#[cfg(test)]
impl Service {
    /// The pages of each table and index (SQLite's `dbstat`), for a measure by hand.
    pub(crate) fn sizes_for_tests(&self, project_id: &str) -> Vec<(String, i64)> {
        let conn = store::open(&self.db(project_id)).unwrap();
        let mut stmt = match conn
            .prepare("SELECT name, SUM(pgsize) FROM dbstat GROUP BY name ORDER BY 2 DESC")
        {
            Ok(s) => s,
            Err(e) => return vec![(e.to_string(), 0)],
        };
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .flatten()
            .collect();
        rows
    }
}

/// The resolver and its types, for the tests ported from tokensave.
#[cfg(test)]
pub(crate) mod resolution {
    pub use super::resolver::*;
}
