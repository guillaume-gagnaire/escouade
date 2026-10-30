//! Ported from tokensave `src/mcp/tools/handlers/graph.rs` (`handle_search`,
//! `handle_literal_search`, `scan_literal_file`) and `src/mcp/tools/handlers/info.rs`
//! (`handle_body`, `body_result`, `candidates_result`) with `body_kind_preference` of
//! `src/mcp/tools/handlers/symbol_target.rs` (github.com/aovestdipaperino/tokensave, commit
//! 580936b, MIT, © 2025 Enzo Lombardi; see THIRD-PARTY-NOTICES.md): what `index_search` and
//! `index_body` find, before `format` writes it.
//!
//! What differs, for our architecture: the literal scan reads the files this folder sees (the
//! worktree's delta) one after the other, through the fence, and has no report of the tracked
//! files the index holds no row for (tokensave's `unscanned`: our index lists the files git gives,
//! and says nothing of the others).

use super::api::{Answer, Hit, Reader};
use super::calls::segments;
use super::filters::PathFilter;
use anyhow::Result;
use serde::Serialize;
use std::time::{Duration, Instant};

/// Upper bound on the size of a single source file scanned in literal mode.
const LITERAL_SCAN_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// How long a literal scan may run before it answers with what it found.
pub const LITERAL_SCAN_DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// The most candidates `index_body` lists for an ambiguous name: 20 by default, 50 at most.
pub const BODY_CANDIDATES_DEFAULT: usize = 20;
pub const BODY_CANDIDATES_MAX: usize = 50;

/// The symbols found by a ranked search, and whether more matched than `limit`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub hits: Vec<Hit>,
    pub truncated: bool,
}

/// A line of a literal search.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteralMatch {
    pub path: String,
    pub line: u32,
    pub text: String,
    /// The innermost symbol whose lines hold it.
    pub enclosing: Option<String>,
    pub enclosing_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Literal {
    pub query: String,
    pub matches: Vec<LiteralMatch>,
    /// Every match in scope, including those past `limit`.
    pub total: usize,
    /// Whether `total` is the number of matches in the project (the scan went to the end).
    pub total_exact: bool,
    pub truncated: bool,
    pub timed_out: bool,
}

/// What `index_body` gives.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "found")]
pub enum Body {
    /// No symbol by that id or name (`label`: « with node_id 12 », « named 'x' »).
    None { label: String },
    /// One definition: its whole body.
    One { hit: Hit, body: String },
    /// Several definitions as good: no body, the candidates to ask again by.
    Many {
        symbol: String,
        total: usize,
        candidates: Vec<(Hit, String)>,
    },
}

/// tokensave `body_kind_preference`: a function before a type, before an `impl`, a constant, a
/// field.
fn body_kind_preference(kind: &str) -> u8 {
    match kind {
        "function" | "method" => 0,
        "struct" | "enum" | "trait" | "class" | "interface" | "type" | "component" => 1,
        "impl" => 2,
        "const" | "macro" => 3,
        "field" => 4,
        _ => 6,
    }
}

impl Reader {
    /// tokensave `handle_search` (ranked): one more result fetched, to tell `truncated`; with
    /// path filters, a larger set first (5 × `limit`, 50 at least), filtered, then cut; the
    /// patterns of `queryignore` dropped.
    pub fn search_filtered(
        &self,
        query: &str,
        limit: usize,
        kind: Option<&str>,
        filter: &PathFilter,
    ) -> Result<Answer<Found>> {
        let probe_limit = limit.saturating_add(1);
        let lists =
            filter.prefix.is_some() || !filter.include.is_empty() || !filter.exclude.is_empty();
        let mut results = if lists {
            let candidate_limit = limit.saturating_mul(5).max(50);
            let mut results = self.ranked_search(query, candidate_limit, kind)?;
            results.retain(|r| {
                super::filters::path_lists_keep(&r.node.path, &filter.include, &filter.exclude)
                    && filter
                        .prefix
                        .as_deref()
                        .is_none_or(|p| super::filters::prefix_keeps(&r.node.path, p))
            });
            results.truncate(probe_limit);
            results
        } else {
            self.ranked_search(query, probe_limit, kind)?
        };
        if !filter.ignore.is_empty() {
            results.retain(|r| !filter.ignore.is_ignored(&r.node.path));
        }
        let truncated = results.len() > limit;
        results.truncate(limit);
        let hits: Vec<Hit> = results.iter().map(|s| s.node.to_hit()).collect();
        let paths: Vec<String> = hits.iter().map(|h| h.path.clone()).collect();
        let saving = self.saving(&paths);
        Ok(Answer {
            result: Found { hits, truncated },
            saving,
        })
    }

    /// tokensave `handle_literal_search`: the exact (case-sensitive) substring in the source of
    /// the files this folder sees, sorted by path; the first `limit` matches kept, the others
    /// counted unless `exact_total` is off; a scan past `timeout` answers with what it found.
    pub fn literal_search(
        &self,
        query: &str,
        limit: usize,
        filter: &PathFilter,
        exact_total: bool,
        timeout: Option<Duration>,
    ) -> Result<Answer<Literal>> {
        let mut files = self.visible_files()?;
        files.sort();
        files.retain(|p| filter.keeps(p));
        let deadline = timeout.map(|budget| Instant::now() + budget);
        let mut matches: Vec<LiteralMatch> = Vec::new();
        let mut touched: Vec<String> = Vec::new();
        let mut total = 0usize;
        let mut complete = true;
        let mut timed_out = false;
        for path in &files {
            if deadline.is_some_and(|d| Instant::now() >= d) {
                complete = false;
                timed_out = true;
                break;
            }
            let full = self.dir_path().join(path);
            let Ok(meta) = std::fs::metadata(&full) else {
                continue;
            };
            if !meta.file_type().is_file() || meta.len() > LITERAL_SCAN_MAX_FILE_BYTES {
                continue;
            }
            let Some(source) = self.read_file(path) else {
                continue;
            };
            let budget = limit.saturating_sub(matches.len());
            let mut hits: Vec<(u32, String)> = Vec::new();
            for (idx, line) in source.lines().enumerate() {
                if !line.contains(query) {
                    continue;
                }
                total += 1;
                if hits.len() < budget {
                    hits.push((idx as u32 + 1, line.trim().to_string()));
                }
            }
            if !hits.is_empty() {
                let nodes = self.nodes_of_file(path)?;
                for (line, text) in hits {
                    let enclosing = nodes
                        .iter()
                        .filter(|n| n.line <= line && line <= n.end_line)
                        .min_by_key(|n| n.end_line.saturating_sub(n.line));
                    matches.push(LiteralMatch {
                        path: path.clone(),
                        line,
                        text,
                        enclosing: enclosing.map(|n| n.name.clone()),
                        enclosing_id: enclosing.map(|n| n.id),
                    });
                    if !touched.contains(path) {
                        touched.push(path.clone());
                    }
                }
            }
            if !exact_total && matches.len() >= limit {
                complete = false;
                break;
            }
        }
        let total_exact = complete;
        let truncated = !total_exact || total > matches.len();
        let saving = self.saving(&touched);
        Ok(Answer {
            result: Literal {
                query: query.to_string(),
                matches,
                total,
                total_exact,
                truncated,
                timed_out,
            },
            saving,
        })
    }

    /// tokensave `handle_body`: by id, or by a name (qualified or its last segments, else the
    /// search's best 20); the best kind kept; one definition gives its body, several their list.
    pub fn body(
        &self,
        id: Option<i64>,
        symbol: Option<&str>,
        limit: usize,
    ) -> Result<Answer<Body>> {
        let symbol = symbol.map(str::trim).filter(|s| !s.is_empty());
        let (label, mut candidates) = match (id, symbol) {
            (Some(id), _) => (
                format!("with node_id '{id}'"),
                self.by_id(id)?
                    .map(|h| {
                        let q = super::api::qualified(&h.path, &h.parent, &h.name);
                        vec![(h, q)]
                    })
                    .unwrap_or_default(),
            ),
            (None, Some(s)) => (format!("named '{s}'"), self.symbol_nodes(s)?),
            (None, None) => anyhow::bail!("missing required parameter: symbol (or node_id)"),
        };
        if candidates.is_empty() && id.is_none() {
            if let Some(s) = symbol {
                candidates = self
                    .ranked_search(s, 20, None)?
                    .into_iter()
                    .map(|r| (r.node.to_hit(), r.node.qualified()))
                    .collect();
            }
        }
        let mut seen = std::collections::HashSet::new();
        candidates.retain(|(h, _)| seen.insert(h.id));
        if let Some(best) = candidates
            .iter()
            .map(|(h, _)| body_kind_preference(&h.kind))
            .min()
        {
            candidates.retain(|(h, _)| body_kind_preference(&h.kind) == best);
        }
        let limit = limit.clamp(1, BODY_CANDIDATES_MAX);
        let (result, paths) = match candidates.len() {
            0 => (Body::None { label }, Vec::new()),
            1 => {
                let (hit, _) = candidates.remove(0);
                let body = self
                    .lines(&hit.path, hit.line, hit.end_line)
                    .unwrap_or_else(|| "<file unreadable>".to_string());
                let paths = vec![hit.path.clone()];
                (Body::One { hit, body }, paths)
            }
            total => {
                candidates.truncate(limit);
                (
                    Body::Many {
                        symbol: symbol.unwrap_or_default().to_string(),
                        total,
                        candidates,
                    },
                    Vec::new(),
                )
            }
        };
        let saving = self.saving(&paths);
        Ok(Answer { result, saving })
    }
}

/// The last two segments of a candidate's qualified name, without its file (`Type::member`): the
/// example of the ambiguous answer.
pub fn member_example(hit: &Hit) -> String {
    let mut segs: Vec<&str> = segments(&hit.parent);
    segs.push(&hit.name);
    segs[segs.len().saturating_sub(2)..].join("::")
}
