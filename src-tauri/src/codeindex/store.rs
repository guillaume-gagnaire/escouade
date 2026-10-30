//! The database: its schema, its scopes.

use anyhow::Result;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Raised when the tables change: an older database is made again from the files, once, without
/// asking (2: the full-text index of tokensave, ESC-296; 3: the calls resolved as tokensave does,
/// ESC-297).
const SCHEMA: i64 = 3;

const TABLES: &str = "
CREATE TABLE files (
  id INTEGER PRIMARY KEY,
  scope TEXT NOT NULL,
  path TEXT NOT NULL,
  lang TEXT NOT NULL,
  size INTEGER NOT NULL,
  mtime INTEGER NOT NULL,
  hash TEXT NOT NULL,
  -- 0: as in the project's folder (or the project's own file), 1: its own symbols, 2: removed
  state INTEGER NOT NULL DEFAULT 0,
  UNIQUE (scope, path)
);
CREATE TABLE symbols (
  id INTEGER PRIMARY KEY,
  file_id INTEGER NOT NULL,
  scope TEXT NOT NULL,
  path TEXT NOT NULL,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  parent TEXT NOT NULL,
  line INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  sig TEXT NOT NULL,
  doc TEXT NOT NULL,
  -- pub, crate, super or private, as tokensave reads it
  vis TEXT NOT NULL DEFAULT 'private',
  -- tokensave's qualified name: `path::Holder::name`
  qname TEXT NOT NULL,
  -- the type a field calls the methods of
  ty TEXT NOT NULL
);
CREATE INDEX symbols_name ON symbols (name COLLATE NOCASE);
CREATE INDEX symbols_file ON symbols (file_id);
CREATE INDEX symbols_path ON symbols (scope, path);
CREATE INDEX symbols_qname ON symbols (qname);
CREATE TABLE edges (
  id INTEGER PRIMARY KEY,
  file_id INTEGER NOT NULL,
  scope TEXT NOT NULL,
  path TEXT NOT NULL,
  from_id INTEGER,
  kind TEXT NOT NULL,
  target TEXT NOT NULL,
  line INTEGER NOT NULL,
  col INTEGER NOT NULL,
  -- 0: as written; 1: `Type::method`; 2: `field::method`
  via INTEGER NOT NULL,
  -- tokensave's `simple_ref_name` of the target: what a reading that touched a name finds it by
  simple TEXT NOT NULL,
  -- the field of `field::method` (`via` 2)
  recv TEXT NOT NULL
);
CREATE INDEX edges_from ON edges (from_id);
-- the imports, read whole by each resolution
CREATE INDEX edges_import ON edges (scope) WHERE kind = 'import';
CREATE INDEX edges_file ON edges (file_id);
CREATE INDEX edges_simple ON edges (scope, simple);
CREATE INDEX edges_recv ON edges (scope, recv) WHERE via = 2;
-- the references of one call site, judged together
CREATE INDEX edges_site ON edges (file_id, line, col);
-- The references resolved: from a symbol (none at a file's top) to a symbol.
CREATE TABLE links (
  id INTEGER PRIMARY KEY,
  ref_id INTEGER NOT NULL,
  scope TEXT NOT NULL,
  path TEXT NOT NULL,
  from_id INTEGER,
  to_id INTEGER NOT NULL,
  kind TEXT NOT NULL,
  line INTEGER NOT NULL,
  conf REAL NOT NULL,
  by TEXT NOT NULL
);
CREATE INDEX links_to ON links (to_id, kind);
CREATE INDEX links_from ON links (from_id, kind);
CREATE INDEX links_ref ON links (ref_id);
CREATE INDEX links_scope ON links (scope);
-- The calls left unresolved by a tie, with their candidates (ids, `,`-joined).
CREATE TABLE ambiguous (
  id INTEGER PRIMARY KEY,
  ref_id INTEGER NOT NULL,
  scope TEXT NOT NULL,
  path TEXT NOT NULL,
  from_id INTEGER,
  name TEXT NOT NULL,
  line INTEGER NOT NULL,
  candidates TEXT NOT NULL,
  -- the simple name of the call, which every candidate bears: how a symbol finds its ties
  simple TEXT NOT NULL
);
CREATE INDEX ambiguous_ref ON ambiguous (ref_id);
CREATE INDEX ambiguous_simple ON ambiguous (simple);
CREATE INDEX ambiguous_scope ON ambiguous (scope);
CREATE INDEX ambiguous_from ON ambiguous (from_id);
-- The project's references a worktree resolves again (its links of them hide the project's).
CREATE TABLE overrides (
  scope TEXT NOT NULL,
  ref_id INTEGER NOT NULL,
  PRIMARY KEY (scope, ref_id)
);
CREATE TABLE scopes (
  scope TEXT PRIMARY KEY,
  dir TEXT NOT NULL,
  refreshed INTEGER NOT NULL,
  -- raised when the project's links change
  gen INTEGER NOT NULL DEFAULT 0,
  -- the project's `gen` a worktree's links were made against
  base_gen INTEGER NOT NULL DEFAULT -1
);
-- tokensave's nodes_fts (src/db/migrations.rs): the same columns, weighted 10, 5, 1, 2, 3 by the
-- queries, the same tokenizer; the rowid is the symbol's id.
CREATE VIRTUAL TABLE sym_fts USING fts5(
  name, qualified, doc, sig, terms,
  tokenize = 'porter unicode61'
);
-- tokensave's executable_body_fts: the body of each function and method, the rowid its id.
CREATE VIRTUAL TABLE body_fts USING fts5(
  path UNINDEXED, body,
  tokenize = 'unicode61'
);
";

/// Opens the project's database, made if it is not, made again if it is of another schema.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version != SCHEMA {
        conn.execute_batch(
            "DROP TABLE IF EXISTS sym_fts; DROP TABLE IF EXISTS body_fts; DROP TABLE IF EXISTS edges;
             DROP TABLE IF EXISTS links; DROP TABLE IF EXISTS ambiguous; DROP TABLE IF EXISTS overrides;
             DROP TABLE IF EXISTS symbols;
             DROP TABLE IF EXISTS files; DROP TABLE IF EXISTS scopes;",
        )?;
        conn.execute_batch(TABLES)?;
        conn.pragma_update(None, "user_version", SCHEMA)?;
    }
    Ok(conn)
}

/// The name of a worktree's scope: a hash of its folder.
pub fn scope_of(dir: &Path) -> String {
    let key = crate::paths::folder_key(&dir.to_string_lossy());
    let digest = Sha256::digest(key.as_bytes());
    let hex: String = digest.iter().take(6).map(|b| format!("{b:02x}")).collect();
    format!("wt-{hex}")
}

/// Everything a scope holds.
pub fn drop_scope(conn: &Connection, scope: &str) -> Result<()> {
    for fts in ["sym_fts", "body_fts"] {
        conn.execute(
            &format!("DELETE FROM {fts} WHERE rowid IN (SELECT id FROM symbols WHERE scope = ?1)"),
            [scope],
        )?;
    }
    conn.execute("DELETE FROM links WHERE scope = ?1", [scope])?;
    conn.execute("DELETE FROM ambiguous WHERE scope = ?1", [scope])?;
    conn.execute("DELETE FROM overrides WHERE scope = ?1", [scope])?;
    conn.execute("DELETE FROM edges WHERE scope = ?1", [scope])?;
    conn.execute("DELETE FROM symbols WHERE scope = ?1", [scope])?;
    conn.execute("DELETE FROM files WHERE scope = ?1", [scope])?;
    conn.execute("DELETE FROM scopes WHERE scope = ?1", [scope])?;
    Ok(())
}

/// Everything in the database.
pub fn wipe(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DELETE FROM sym_fts; DELETE FROM body_fts; DELETE FROM edges; DELETE FROM links;
         DELETE FROM ambiguous; DELETE FROM overrides; DELETE FROM symbols; DELETE FROM files;
         DELETE FROM scopes;",
    )?;
    Ok(())
}

/// Forgets the worktrees whose folder is gone.
pub fn prune(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT scope, dir FROM scopes WHERE scope <> ''")?;
    let gone: Vec<String> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .filter(|(_, dir)| !Path::new(dir).is_dir())
        .map(|(s, _)| s)
        .collect();
    drop(stmt);
    for scope in gone {
        drop_scope(conn, &scope)?;
    }
    Ok(())
}

pub fn status(conn: &Connection, db: &Path) -> Result<super::Status> {
    let count =
        |sql: &str| -> u64 { conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0) as u64 };
    let at: i64 = conn
        .query_row("SELECT refreshed FROM scopes WHERE scope = ''", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    let mut bytes = std::fs::metadata(db).map(|m| m.len()).unwrap_or(0);
    let mut wal = db.to_path_buf().into_os_string();
    wal.push("-wal");
    bytes += std::fs::metadata(wal).map(|m| m.len()).unwrap_or(0);
    Ok(super::Status {
        files: count("SELECT COUNT(*) FROM files WHERE scope = '' "),
        symbols: count("SELECT COUNT(*) FROM symbols WHERE scope = ''"),
        links: count("SELECT COUNT(*) FROM links WHERE scope = ''"),
        worktrees: count("SELECT COUNT(*) FROM scopes WHERE scope <> ''"),
        bytes,
        indexed_at: at,
    })
}

pub fn touch_scope(conn: &Connection, scope: &str, dir: &Path, now_ms: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO scopes (scope, dir, refreshed) VALUES (?1, ?2, ?3)
         ON CONFLICT (scope) DO UPDATE SET dir = ?2, refreshed = ?3",
        params![scope, dir.to_string_lossy(), now_ms],
    )?;
    Ok(())
}
