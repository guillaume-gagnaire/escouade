//! The index of a project's documents: pieces of text with their vectors, in one SQLite file of the app's own folder
//! (never in the project: it is recomputed from the documents). Search by the meaning (the nearest vectors), by the
//! words (FTS5) or both, within the sources asked for.

use super::chunk::{chunks, Format};
use super::embed::{Embedder, Kind};
use anyhow::{Context, Result};
use rusqlite::{params, params_from_iter, Connection};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Where a text comes from: what a search is narrowed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    Spec,
    Decision,
    Document,
    Memory,
    Code,
}

impl Source {
    pub const ALL: [Source; 5] = [
        Source::Spec,
        Source::Decision,
        Source::Document,
        Source::Memory,
        Source::Code,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spec => "spec",
            Self::Decision => "decision",
            Self::Document => "document",
            Self::Memory => "memory",
            Self::Code => "code",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|x| x.as_str() == s)
    }
}

/// What `Index::put` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Put {
    /// The text was the one already indexed: nothing done.
    pub unchanged: bool,
    pub chunks: usize,
    /// Pieces the model had to read (the others kept their vectors).
    pub embedded: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub source: Source,
    pub path: String,
    pub title: String,
    pub text: String,
    /// Higher is closer. A cosine for the search by meaning, a rank fusion for the hybrid one.
    pub score: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    pub documents: usize,
    pub chunks: usize,
}

pub struct Index {
    conn: Connection,
    embedder: Arc<dyn Embedder>,
}

fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}

fn blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn unblob(b: &[u8]) -> impl Iterator<Item = f32> + '_ {
    b.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c))
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS docs (
    id INTEGER PRIMARY KEY, source TEXT NOT NULL, path TEXT NOT NULL, hash TEXT NOT NULL,
    UNIQUE (source, path));
CREATE TABLE IF NOT EXISTS chunks (
    id INTEGER PRIMARY KEY, doc INTEGER NOT NULL REFERENCES docs(id) ON DELETE CASCADE,
    ord INTEGER NOT NULL, title TEXT NOT NULL, text TEXT NOT NULL, hash TEXT NOT NULL, vec BLOB NOT NULL);
CREATE INDEX IF NOT EXISTS chunks_doc ON chunks(doc);
CREATE VIRTUAL TABLE IF NOT EXISTS words USING fts5(title, text, tokenize = 'unicode61 remove_diacritics 2');
";

impl Index {
    /// The index in `path` (created if need be), for the vectors of `embedder`. An index made by another model, or by a
    /// vector of another size, is emptied: its vectors would not compare with these.
    pub fn open(path: &Path, embedder: Arc<dyn Embedder>) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::with(
            Connection::open(path).with_context(|| path.display().to_string())?,
            embedder,
        )
    }

    pub fn in_memory(embedder: Arc<dyn Embedder>) -> Result<Self> {
        Self::with(Connection::open_in_memory()?, embedder)
    }

    fn with(conn: Connection, embedder: Arc<dyn Embedder>) -> Result<Self> {
        // The memory's search and a document dropped may write at the same time: one waits for the other.
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(SCHEMA)?;
        let me = Self { conn, embedder };
        let now = format!("{}:{}", me.embedder.id(), me.embedder.dim());
        let had: Option<String> = me
            .conn
            .query_row("SELECT value FROM meta WHERE key = 'model'", [], |r| {
                r.get(0)
            })
            .ok();
        if had.as_deref() != Some(&now) {
            me.conn
                .execute_batch("DELETE FROM chunks; DELETE FROM docs; DELETE FROM words;")?;
            me.conn.execute(
                "INSERT OR REPLACE INTO meta (key, value) VALUES ('model', ?1)",
                [&now],
            )?;
        }
        Ok(me)
    }

    pub fn stats(&self) -> Result<Stats> {
        let n = |t: &str| -> Result<usize> {
            Ok(self
                .conn
                .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))?)
        };
        Ok(Stats {
            documents: n("docs")?,
            chunks: n("chunks")?,
        })
    }

    /// Indexes `text`, the document `path` of `source`, replacing what was indexed under that name. Incremental: the
    /// same text is not read again, and of a changed one only the pieces that changed go through the model.
    pub fn put(&mut self, source: Source, path: &str, text: &str) -> Result<Put> {
        let doc_hash = hash(text);
        let known: Option<(i64, String)> = self
            .conn
            .query_row(
                "SELECT id, hash FROM docs WHERE source = ?1 AND path = ?2",
                params![source.as_str(), path],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        if let Some((id, h)) = &known {
            if *h == doc_hash {
                let n: usize = self.conn.query_row(
                    "SELECT COUNT(*) FROM chunks WHERE doc = ?1",
                    [id],
                    |r| r.get(0),
                )?;
                return Ok(Put {
                    unchanged: true,
                    chunks: n,
                    embedded: 0,
                });
            }
        }
        let pieces = chunks(text, Format::of_path(path));
        // The vectors of the pieces already there, by the hash of what was read.
        let mut reuse: HashMap<String, Vec<u8>> = HashMap::new();
        if let Some((id, _)) = &known {
            let mut st = self
                .conn
                .prepare("SELECT hash, vec FROM chunks WHERE doc = ?1")?;
            for row in st.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))? {
                let (h, v): (String, Vec<u8>) = row?;
                reuse.insert(h, v);
            }
        }
        let hashes: Vec<String> = pieces.iter().map(|c| hash(&c.embedded())).collect();
        let todo: Vec<usize> = (0..pieces.len())
            .filter(|&i| !reuse.contains_key(&hashes[i]))
            .collect();
        let texts: Vec<String> = todo.iter().map(|&i| pieces[i].embedded()).collect();
        // Outside the transaction: the model takes its time, the database is not held.
        let fresh = if texts.is_empty() {
            Vec::new()
        } else {
            self.embedder.embed(&texts, Kind::Passage)?
        };
        for (&i, v) in todo.iter().zip(&fresh) {
            reuse.insert(hashes[i].clone(), blob(v));
        }
        let tx = self.conn.transaction()?;
        if let Some((id, _)) = &known {
            Self::drop_doc(&tx, *id)?;
        }
        tx.execute(
            "INSERT INTO docs (source, path, hash) VALUES (?1, ?2, ?3)",
            params![source.as_str(), path, doc_hash],
        )?;
        let doc = tx.last_insert_rowid();
        for (ord, (c, h)) in pieces.iter().zip(&hashes).enumerate() {
            tx.execute(
                "INSERT INTO chunks (doc, ord, title, text, hash, vec) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![doc, ord as i64, c.title, c.text, h, reuse[h]],
            )?;
            tx.execute(
                "INSERT INTO words (rowid, title, text) VALUES (?1, ?2, ?3)",
                params![tx.last_insert_rowid(), c.title, c.text],
            )?;
        }
        tx.commit()?;
        Ok(Put {
            unchanged: false,
            chunks: pieces.len(),
            embedded: todo.len(),
        })
    }

    fn drop_doc(tx: &rusqlite::Transaction, id: i64) -> Result<()> {
        tx.execute(
            "DELETE FROM words WHERE rowid IN (SELECT id FROM chunks WHERE doc = ?1)",
            [id],
        )?;
        tx.execute("DELETE FROM docs WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Forgets a document. Whether there was one.
    pub fn remove(&mut self, source: Source, path: &str) -> Result<bool> {
        let tx = self.conn.transaction()?;
        let id: Option<i64> = tx
            .query_row(
                "SELECT id FROM docs WHERE source = ?1 AND path = ?2",
                params![source.as_str(), path],
                |r| r.get(0),
            )
            .ok();
        if let Some(id) = id {
            Self::drop_doc(&tx, id)?;
        }
        tx.commit()?;
        Ok(id.is_some())
    }

    /// Forgets the documents of `source` that are not in `keep` (the files that were deleted). How many.
    pub fn retain(&mut self, source: Source, keep: &[&str]) -> Result<usize> {
        let mut st = self
            .conn
            .prepare("SELECT path FROM docs WHERE source = ?1")?;
        let gone: Vec<String> = st
            .query_map([source.as_str()], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        drop(st);
        let mut n = 0;
        for p in gone.iter().filter(|p| !keep.contains(&p.as_str())) {
            n += self.remove(source, p)? as usize;
        }
        Ok(n)
    }

    /// The paths of a source with the hash of what was indexed: what a caller compares with the files to find which
    /// changed.
    pub fn documents(&self, source: Source) -> Result<Vec<(String, String)>> {
        let mut st = self
            .conn
            .prepare("SELECT path, hash FROM docs WHERE source = ?1 ORDER BY path")?;
        let rows = st
            .query_map([source.as_str()], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?;
        Ok(rows)
    }

    /// Each document of `source` as one vector: the mean of its pieces', brought to a length of 1. What a folder of the
    /// drive is compared on (`docsdrive::place`).
    pub fn doc_vectors(&self, source: Source) -> Result<Vec<(String, Vec<f32>)>> {
        let mut st = self.conn.prepare(
            "SELECT d.path, c.vec FROM chunks c JOIN docs d ON d.id = c.doc WHERE d.source = ?1 ORDER BY d.path, c.ord",
        )?;
        let mut rows = st.query([source.as_str()])?;
        let mut out: Vec<(String, Vec<f32>)> = Vec::new();
        while let Some(r) = rows.next()? {
            let path: String = r.get(0)?;
            let v: Vec<f32> = unblob(r.get_ref(1)?.as_blob()?).collect();
            match out.last_mut() {
                Some((p, sum)) if *p == path && sum.len() == v.len() => {
                    sum.iter_mut().zip(&v).for_each(|(a, b)| *a += b);
                }
                _ => out.push((path, v)),
            }
        }
        for (_, v) in &mut out {
            super::embed::normalize(v);
        }
        Ok(out)
    }

    /// A text as one vector, read as a passage (what is searched in), its pieces' mean as `doc_vectors` makes it.
    pub fn passage_vector(&self, text: &str, format: Format) -> Result<Vec<f32>> {
        let texts: Vec<String> = chunks(text, format).iter().map(|c| c.embedded()).collect();
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let vs = self.embedder.embed(&texts, Kind::Passage)?;
        let mut sum = vec![0f32; vs.first().map_or(0, Vec::len)];
        for v in &vs {
            sum.iter_mut().zip(v).for_each(|(a, b)| *a += b);
        }
        super::embed::normalize(&mut sum);
        Ok(sum)
    }

    pub fn embedder_id(&self) -> String {
        self.embedder.id().to_string()
    }

    /// The hash `put` keeps of a text: a caller skips the `put` of a file whose hash is the one in `documents`.
    pub fn text_hash(text: &str) -> String {
        hash(text)
    }

    fn filter_sql(sources: &[Source]) -> (String, Vec<String>) {
        if sources.is_empty() {
            return (String::new(), Vec::new());
        }
        let marks = vec!["?"; sources.len()].join(",");
        (
            format!(" AND d.source IN ({marks})"),
            sources.iter().map(|s| s.as_str().to_string()).collect(),
        )
    }

    fn hits(&self, ranked: &[(i64, f32)]) -> Result<Vec<Hit>> {
        let mut st = self.conn.prepare(
            "SELECT d.source, d.path, c.title, c.text FROM chunks c JOIN docs d ON d.id = c.doc WHERE c.id = ?1",
        )?;
        let mut out = Vec::new();
        for &(id, score) in ranked {
            let (source, path, title, text): (String, String, String, String) =
                st.query_row([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
            out.push(Hit {
                source: Source::parse(&source).unwrap_or(Source::Document),
                path,
                title,
                text,
                score,
            });
        }
        Ok(out)
    }

    /// The `k` nearest pieces to `query` by the meaning, within `sources` (all of them if empty).
    pub fn search(&self, query: &str, k: usize, sources: &[Source]) -> Result<Vec<Hit>> {
        let q = self
            .embedder
            .embed(&[query.to_string()], Kind::Query)?
            .pop()
            .unwrap_or_default();
        let ranked = self.nearest(&q, k, sources)?;
        self.hits(&ranked)
    }

    fn nearest(&self, q: &[f32], k: usize, sources: &[Source]) -> Result<Vec<(i64, f32)>> {
        let (cond, args) = Self::filter_sql(sources);
        let mut st = self.conn.prepare(&format!(
            "SELECT c.id, c.vec FROM chunks c JOIN docs d ON d.id = c.doc WHERE 1 = 1{cond}"
        ))?;
        let mut all: Vec<(i64, f32)> = Vec::new();
        let mut rows = st.query(params_from_iter(args.iter()))?;
        while let Some(r) = rows.next()? {
            let id: i64 = r.get(0)?;
            let v = r.get_ref(1)?.as_blob()?;
            let score: f32 = unblob(v).zip(q).map(|(a, b)| a * b).sum();
            all.push((id, score));
        }
        let k = k.min(all.len());
        if k == 0 {
            return Ok(Vec::new());
        }
        all.select_nth_unstable_by(k - 1, |a, b| b.1.total_cmp(&a.1));
        all.truncate(k);
        all.sort_by(|a, b| b.1.total_cmp(&a.1));
        Ok(all)
    }

    /// The pieces that hold the words of `query`, best first (FTS5's bm25).
    fn by_words(&self, query: &str, k: usize, sources: &[Source]) -> Result<Vec<i64>> {
        let words: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| format!("\"{w}\""))
            .collect();
        if words.is_empty() {
            return Ok(Vec::new());
        }
        let (cond, args) = Self::filter_sql(sources);
        let mut st = self.conn.prepare(&format!(
            "SELECT w.rowid FROM words w JOIN chunks c ON c.id = w.rowid JOIN docs d ON d.id = c.doc
             WHERE words MATCH ?1{} ORDER BY w.rank LIMIT {k}",
            cond
        ))?;
        let mut all = vec![words.join(" OR ")];
        all.extend(args);
        let ids = st
            .query_map(params_from_iter(all.iter()), |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        Ok(ids)
    }

    /// Meaning and words together: the two rankings merged by their ranks (reciprocal rank fusion), so that a rare
    /// word the model blurs (a name, an identifier) still finds its piece, and a paraphrase the words miss too.
    pub fn search_hybrid(&self, query: &str, k: usize, sources: &[Source]) -> Result<Vec<Hit>> {
        let wide = (k * 4).max(20);
        let q = self
            .embedder
            .embed(&[query.to_string()], Kind::Query)?
            .pop()
            .unwrap_or_default();
        let by_meaning = self.nearest(&q, wide, sources)?;
        let by_words = self.by_words(query, wide, sources)?;
        let mut fused: HashMap<i64, f32> = HashMap::new();
        for (rank, (id, _)) in by_meaning.iter().enumerate() {
            *fused.entry(*id).or_default() += 1.0 / (60.0 + rank as f32);
        }
        for (rank, id) in by_words.iter().enumerate() {
            *fused.entry(*id).or_default() += 1.0 / (60.0 + rank as f32);
        }
        let mut ranked: Vec<(i64, f32)> = fused.into_iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        ranked.truncate(k);
        self.hits(&ranked)
    }
}
