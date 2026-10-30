//! What the agents ask of the index: find a symbol or a notion, the context of a task, who calls a
//! function and what it calls, what a change would touch. Each answer tells what it saved: the
//! tokens reading the files it draws on would have cost, against its own.

use super::tokens_of;
use crate::fence::Fence;
use anyhow::Result;
use rusqlite::{named_params, Connection, Row};
use serde::Serialize;
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

/// What an answer saved, in estimated tokens (about four bytes each).
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Saving {
    /// The files the answer draws on, read whole.
    pub files_tokens: u64,
    /// The answer itself.
    pub answer_tokens: u64,
    pub saved_tokens: u64,
    /// 0 to 100.
    pub saved_percent: u64,
    /// How many files the answer draws on.
    pub files: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer<T> {
    pub result: T,
    pub saving: Saving,
}

/// A symbol found.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    /// Its id in the index, for `index_callers`, `index_callees` and `index_impact` (ESC-297).
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub parent: String,
    pub path: String,
    pub line: u32,
    pub end_line: u32,
    pub sig: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub path: String,
    pub line: u32,
    pub end_line: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub entry_points: Vec<Hit>,
    pub related: Vec<Hit>,
    pub snippets: Vec<Snippet>,
    /// Whether to trust the answer or ask again (tokensave's « Retrieval »).
    pub retrieval: Retrieval,
}

/// The diagnostic of a context (tokensave `RetrievalDiagnostics`): the results of each term
/// searched, the best score, and the match: `exact`, `strong` or `fts-only`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Retrieval {
    pub terms: Vec<TermHits>,
    pub best_score: Option<f64>,
    #[serde(rename = "match")]
    pub quality: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TermHits {
    pub term: String,
    /// At most the bounded top-K of the term (10).
    pub hits: usize,
}

/// A symbol of the index with what its ranking needs (tokensave's `Node`).
#[derive(Debug, Clone)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub parent: String,
    pub path: String,
    pub line: u32,
    pub end_line: u32,
    pub sig: String,
    pub doc: String,
    pub vis: String,
}

/// The columns `Node::of` reads, of table alias `s`.
pub const NODE_COLS: &str =
    "s.id, s.name, s.kind, s.parent, s.path, s.line, s.end_line, s.sig, s.doc, s.vis";

impl Node {
    pub fn of(r: &Row) -> rusqlite::Result<Node> {
        Ok(Node {
            id: r.get(0)?,
            name: r.get(1)?,
            kind: r.get(2)?,
            parent: r.get(3)?,
            path: r.get(4)?,
            line: r.get(5)?,
            end_line: r.get(6)?,
            sig: r.get(7)?,
            doc: r.get(8)?,
            vis: r.get(9)?,
        })
    }

    pub fn qualified(&self) -> String {
        qualified(&self.path, &self.parent, &self.name)
    }
}

/// A node and its score (tokensave `SearchResult`).
#[derive(Debug, Clone)]
pub struct Scored {
    pub node: Node,
    pub score: f64,
}

/// The qualified name of a symbol, as tokensave writes it: its file, its holders, its name, joined
/// by `::` (`src/shop.rs::Cart::total`).
pub fn qualified(path: &str, parent: &str, name: &str) -> String {
    if parent.is_empty() {
        format!("{path}::{name}")
    } else {
        format!("{path}::{}::{name}", parent.replace('.', "::"))
    }
}

/// A reader of the index, for the folder the agent works in.
pub struct Reader {
    pub(super) conn: Connection,
    pub(super) scope: String,
    dir: PathBuf,
    fence: Fence,
}

impl Reader {
    pub(super) fn new(conn: Connection, scope: String, dir: PathBuf, fence: Fence) -> Self {
        Self {
            conn,
            scope,
            dir,
            fence,
        }
    }

    /// The rows of table alias `a` this folder sees: the project's folder's, but for the files the
    /// worktree changed or removed, and the worktree's own.
    pub(super) fn vis(a: &str) -> String {
        format!(
            "({a}.scope = :w OR ({a}.scope = '' AND NOT EXISTS (SELECT 1 FROM files fw \
             WHERE fw.scope = :w AND fw.path = {a}.path AND fw.state IN (1, 2))))"
        )
    }

    /// `vis` with the scope as the positional parameter `?n`.
    pub(super) fn vis_pos(a: &str, n: usize) -> String {
        format!(
            "({a}.scope = ?{n} OR ({a}.scope = '' AND NOT EXISTS (SELECT 1 FROM files fw \
             WHERE fw.scope = ?{n} AND fw.path = {a}.path AND fw.state IN (1, 2))))"
        )
    }

    pub(super) fn hit(r: &Row) -> rusqlite::Result<Hit> {
        Ok(Hit {
            id: r.get("id")?,
            name: r.get("name")?,
            kind: r.get("kind")?,
            parent: r.get("parent")?,
            path: r.get("path")?,
            line: r.get("line")?,
            end_line: r.get("end_line")?,
            sig: r.get("sig")?,
            doc: r.get("doc")?,
        })
    }

    // ---- search ----

    /// Symbols matching `query`, ranked as tokensave's search ranks them (`search::ranked_search`):
    /// each word by prefix in the names, qualified names, docs, signatures and parts of the names,
    /// definitions before references, the exact name and the exact phrase first; `kind` keeps one
    /// kind.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        kind: Option<&str>,
    ) -> Result<Answer<Vec<Hit>>> {
        let hits: Vec<Hit> = self
            .ranked_search(query, limit, kind)?
            .iter()
            .map(|s| s.node.to_hit())
            .collect();
        let paths: Vec<String> = hits.iter().map(|h| h.path.clone()).collect();
        let saving = self.saving(&paths, &hits);
        Ok(Answer {
            result: hits,
            saving,
        })
    }

    // ---- the context of a task ----

    /// The context of a task told in words (`max_nodes`: the most symbols, 20 by default).
    pub fn task_context(&self, task: &str, max_nodes: usize) -> Result<Answer<Context>> {
        self.task_context_with(
            task,
            &super::context::BuildContextOptions::of_tool(Some(max_nodes), Vec::new()),
        )
    }

    /// The context of a task as tokensave's `tokensave_context` builds it (`context`): the
    /// entry points (three at most, but for the exact source hits), the symbols one link away from
    /// them, the code of the entry points, and the diagnostic of the search.
    pub fn task_context_with(
        &self,
        task: &str,
        options: &super::context::BuildContextOptions,
    ) -> Result<Answer<Context>> {
        let empty = Context {
            entry_points: Vec::new(),
            related: Vec::new(),
            snippets: Vec::new(),
            retrieval: Retrieval::default(),
        };
        if task.trim().is_empty() {
            let saving = self.saving(&[], &empty);
            return Ok(Answer {
                result: empty,
                saving,
            });
        }
        let built = self.build_context(task, options)?;
        let entry_ids: HashSet<i64> = built.entry_points.iter().map(|n| n.id).collect();
        let entry: Vec<Hit> = built.entry_points.iter().map(Node::to_hit).collect();
        let related: Vec<Hit> = built
            .subgraph
            .iter()
            .filter(|n| !entry_ids.contains(&n.id))
            .map(Node::to_hit)
            .collect();
        let paths: Vec<String> = entry
            .iter()
            .chain(&related)
            .map(|h| h.path.clone())
            .collect();
        let result = Context {
            entry_points: entry,
            related,
            snippets: built.code_blocks,
            retrieval: built.diagnostics,
        };
        let saving = self.saving(&paths, &result);
        Ok(Answer { result, saving })
    }

    /// A file of this folder, if the fence lets it be read.
    pub(super) fn read_file(&self, rel: &str) -> Option<String> {
        let full = self.dir.join(rel);
        if self.fence.covers(&full.to_string_lossy()) {
            return None;
        }
        let bytes = std::fs::read(&full).ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Lines `from..=to` of a file of this folder, if the fence lets them be read.
    pub(super) fn lines(&self, rel: &str, from: u32, to: u32) -> Option<String> {
        let text = self.read_file(rel)?;
        let out: Vec<&str> = text
            .lines()
            .skip(from.saturating_sub(1) as usize)
            .take((to + 1).saturating_sub(from) as usize)
            .collect();
        Some(out.join("\n"))
    }

    /// `target` as a path of a file of this folder: whole, or its end.
    pub(super) fn file_like(&self, target: &str) -> Result<Option<String>> {
        let t = target.replace('\\', "/");
        if !t.contains('/') && !t.contains('.') {
            return Ok(None);
        }
        let sql = "SELECT f.path FROM files f WHERE f.state <> 2 AND (f.path = :t OR f.path LIKE '%/' || :t)
               AND (f.scope = :w OR (f.scope = '' AND NOT EXISTS (SELECT 1 FROM files fw
                   WHERE fw.scope = :w AND fw.path = f.path AND fw.state IN (1, 2))))
             ORDER BY length(f.path) LIMIT 1";
        Ok(self
            .conn
            .query_row(sql, named_params! {":t": t, ":w": self.scope}, |r| r.get(0))
            .ok())
    }

    // ---- savings ----

    /// The tokens of the files `paths` against those of `result`.
    pub(super) fn saving<T: Serialize>(&self, paths: &[String], result: &T) -> Saving {
        let distinct: BTreeSet<&String> = paths.iter().filter(|p| !p.is_empty()).collect();
        let mut bytes = 0u64;
        for p in &distinct {
            let size: i64 = self
                .conn
                .query_row(
                    "SELECT size FROM files WHERE path = :p AND state <> 2 AND scope IN (:w, '')
                     ORDER BY scope = :w DESC LIMIT 1",
                    named_params! {":p": p, ":w": self.scope},
                    |r| r.get(0),
                )
                .unwrap_or(0);
            bytes += size.max(0) as u64;
        }
        let answer = serde_json::to_vec(result)
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        let files_tokens = tokens_of(bytes);
        let answer_tokens = tokens_of(answer);
        let saved = files_tokens.saturating_sub(answer_tokens);
        Saving {
            files_tokens,
            answer_tokens,
            saved_tokens: saved,
            saved_percent: (saved * 100).checked_div(files_tokens).unwrap_or(0),
            files: distinct.len(),
        }
    }
}
