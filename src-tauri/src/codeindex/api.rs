//! What the agents ask of the index: find a symbol or a notion, the context of a task, who calls a
//! function and what it calls, what a change would touch. Each answer tells what it saved: the
//! tokens reading the files it draws on would have cost, against its own.

use super::tokens_of;
use crate::fence::Fence;
use anyhow::Result;
use rusqlite::{named_params, Connection, Row};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
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

    /// Symbols matching `query` (its words, by prefix, in the names, holders, docs and paths),
    /// the best first; `kind` keeps one kind.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        kind: Option<&str>,
    ) -> Result<Answer<Vec<Hit>>> {
        let terms = words(query);
        let mut hits = self.fts(&terms, false, limit, kind, query.trim())?;
        if hits.is_empty() && terms.len() > 1 {
            hits = self.fts(&terms, true, limit, kind, query.trim())?;
        }
        let paths: Vec<String> = hits.iter().map(|h| h.path.clone()).collect();
        let saving = self.saving(&paths, &hits);
        Ok(Answer {
            result: hits,
            saving,
        })
    }

    fn fts(
        &self,
        terms: &[String],
        any: bool,
        limit: usize,
        kind: Option<&str>,
        exact: &str,
    ) -> Result<Vec<Hit>> {
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let m = terms
            .iter()
            .map(|t| format!("\"{t}\"*"))
            .collect::<Vec<_>>()
            .join(if any { " OR " } else { " " });
        let sql = format!(
            "SELECT s.id, s.name, s.kind, s.parent, s.path, s.line, s.end_line, s.sig, s.doc
             FROM sym_fts JOIN symbols s ON s.id = sym_fts.rowid
             WHERE sym_fts MATCH :m AND {} AND (:k IS NULL OR s.kind = :k)
             ORDER BY (s.name = :exact COLLATE NOCASE) DESC, bm25(sym_fts, 10.0, 3.0, 1.5, 2.0)
             LIMIT :n",
            Self::vis("s")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            named_params! {":m": m, ":w": self.scope, ":k": kind, ":exact": exact, ":n": limit as i64},
            Self::hit,
        )?;
        Ok(rows.flatten().collect())
    }

    // ---- the context of a task ----

    /// The context of a task told in words: the symbols to start from, what they are linked to,
    /// and their code.
    pub fn task_context(&self, task: &str, limit: usize) -> Result<Answer<Context>> {
        let terms: Vec<String> = words(task)
            .into_iter()
            .filter(|t| t.chars().count() > 2 && !STOP.contains(&t.as_str()))
            .take(24)
            .collect();
        let pool = self.fts(&terms, true, limit * 6 + 20, None, "")?;
        let mut per_file: HashMap<String, usize> = HashMap::new();
        let mut entry: Vec<Hit> = Vec::new();
        // Whole things first, then their parts; no more than three from one file.
        for pass in 0..2 {
            for h in &pool {
                let part = matches!(h.kind.as_str(), "field" | "const" | "type");
                if (pass == 0) == part || entry.contains(h) {
                    continue;
                }
                let n = per_file.entry(h.path.clone()).or_default();
                if *n >= 3 || entry.len() >= limit {
                    continue;
                }
                *n += 1;
                entry.push(h.clone());
            }
        }
        let mut related: Vec<Hit> = Vec::new();
        for h in entry.iter().take(4).filter(|h| is_callable(&h.kind)) {
            for d in self.callees_of_id(h.id, 3)? {
                if !entry.contains(&d) && !related.contains(&d) {
                    related.push(d);
                }
            }
            for r in self.caller_hits(h.id, 3)? {
                if !entry.contains(&r) && !related.contains(&r) {
                    related.push(r);
                }
            }
        }
        let mut snippets = Vec::new();
        for h in entry.iter().take(5) {
            let end = h.end_line.min(h.line + 30);
            if let Some(text) = self.lines(&h.path, h.line, end) {
                snippets.push(Snippet {
                    path: h.path.clone(),
                    line: h.line,
                    end_line: end,
                    text,
                });
            }
        }
        let paths: Vec<String> = entry
            .iter()
            .chain(&related)
            .map(|h| h.path.clone())
            .collect();
        let result = Context {
            entry_points: entry,
            related,
            snippets,
        };
        let saving = self.saving(&paths, &result);
        Ok(Answer { result, saving })
    }

    /// Lines `from..=to` of a file of this folder, if the fence lets them be read.
    fn lines(&self, rel: &str, from: u32, to: u32) -> Option<String> {
        let full = self.dir.join(rel);
        if self.fence.covers(&full.to_string_lossy()) {
            return None;
        }
        let text = std::fs::read_to_string(&full).ok()?;
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

fn is_callable(kind: &str) -> bool {
    matches!(
        kind,
        "function" | "method" | "class" | "struct" | "component" | "macro"
    )
}

/// The words of a text, lower case, the identifiers cut where they change case or at `_`: `parseConfig`
/// is `parse` and `config`. Each once.
pub fn words(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev: Option<char> = None;
    let flush = |cur: &mut String, out: &mut Vec<String>| {
        if cur.chars().count() >= 2 && !out.contains(cur) {
            out.push(cur.clone());
        }
        cur.clear();
    };
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            flush(&mut cur, &mut out);
            prev = None;
            continue;
        }
        let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
        let boundary = match prev {
            Some(p) => {
                (p.is_lowercase() && c.is_uppercase())
                    || (p.is_uppercase() && c.is_uppercase() && next_lower)
            }
            None => false,
        };
        if boundary {
            flush(&mut cur, &mut out);
        }
        cur.extend(c.to_lowercase());
        prev = Some(c);
    }
    flush(&mut cur, &mut out);
    out
}

/// Words that say nothing of the code, in French and English.
const STOP: &[&str] = &[
    "the", "and", "for", "with", "that", "this", "from", "into", "when", "what", "how", "all",
    "are", "can", "not", "has", "have", "its", "pour", "dans", "les", "des", "une", "est", "que",
    "qui", "avec", "sur", "pas", "par", "aux", "ses", "son", "sont", "plus", "mais", "comme",
    "faut", "doit", "peut", "quand", "ajouter", "ajoute", "faire", "fait", "should", "add", "make",
];
