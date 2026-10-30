//! Ported from tokensave `src/db/queries/search.rs` and `TokenSave::search` of
//! `src/tokensave/query.rs` (github.com/aovestdipaperino/tokensave, commit 580936b, MIT, © 2025
//! Enzo Lombardi; see THIRD-PARTY-NOTICES.md): the queries of the full-text index — a prefix match
//! per word joined by `OR`, the exact-phrase channel, the `LIKE` fallback, a bounded top-K per
//! term, the exact names, the bodies of the functions — and the ranking of `index_search`.
//!
//! What differs, for our architecture: every query sees only what the folder asking sees (the
//! worktree's delta, `Reader::vis`); `index_search` keeps its `kind` filter, applied in the queries;
//! a corrupted index is not rebuilt here (the scan makes it again from the files when its schema
//! changes).

use super::api::{Node, Reader, Scored, NODE_COLS};
use super::ranking::{kind_rank_bonus, kind_tier, path_rank_multiplier};
use anyhow::Result;
use rusqlite::{params_from_iter, types::Value};
use std::collections::{HashMap, HashSet};

/// The weights of the columns: name 10, qualified name 5, doc 1, signature 2, parts of the name 3.
const BM25: &str = "bm25(sym_fts, 10.0, 5.0, 1.0, 2.0, 3.0)";

/// Converts a raw user query into a safe FTS5 MATCH expression: each whitespace token, stripped of
/// `"`, as a quoted prefix term, joined with `OR`. None when nothing is left.
pub(crate) fn to_fts_match_query(query: &str) -> Option<String> {
    let fts_query = query
        .split_whitespace()
        .filter_map(|w| {
            let sanitized: String = w.chars().filter(|c| *c != '"').collect();
            (!sanitized.is_empty()).then(|| format!("\"{sanitized}\"*"))
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    (!fts_query.is_empty()).then_some(fts_query)
}

/// Converts a multi-word query into an FTS5 *phrase* expression. None for a single word.
pub(crate) fn to_fts_phrase_query(query: &str) -> Option<String> {
    let words: Vec<String> = query
        .split_whitespace()
        .map(|w| w.chars().filter(|c| *c != '"').collect::<String>())
        .filter(|w| !w.is_empty())
        .collect();
    (words.len() >= 2).then(|| format!("\"{}\"", words.join(" ")))
}

pub(crate) fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// The qualified name of table alias `s`, computed as `qualified` writes it.
const QUALIFIED_SQL: &str = "(s.path || '::' || CASE WHEN s.parent = '' THEN '' \
     ELSE replace(s.parent, '.', '::') || '::' END || s.name)";

impl Reader {
    fn nodes(&self, sql: &str, args: Vec<Value>) -> Result<Vec<Node>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params_from_iter(args), Node::of)?;
        Ok(rows.flatten().collect())
    }

    fn scored(&self, sql: &str, args: Vec<Value>) -> Result<Vec<Scored>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params_from_iter(args), |r| {
            Ok(Scored {
                node: Node::of(r)?,
                score: -r.get::<_, f64>("rank")?,
            })
        })?;
        Ok(rows.flatten().collect())
    }

    /// The FTS5 query, ranked (tokensave `search_nodes_fts`; `bounded`: `search_nodes_bounded_fts`,
    /// without file and doc nodes, which our index has not).
    fn search_nodes_fts(
        &self,
        fts_query: &str,
        limit: usize,
        kind: Option<&str>,
    ) -> Result<Vec<Scored>> {
        let sql = format!(
            "SELECT {NODE_COLS}, {BM25} AS rank
             FROM sym_fts JOIN symbols s ON s.id = sym_fts.rowid
             WHERE sym_fts MATCH ?1 AND {} AND (?2 IS NULL OR s.kind = ?2)
             ORDER BY rank LIMIT ?3",
            Self::vis_pos("s", 4)
        );
        self.scored(
            &sql,
            vec![
                Value::from(fts_query.to_string()),
                kind.map(|k| Value::from(k.to_string()))
                    .unwrap_or(Value::Null),
                Value::from(limit as i64),
                Value::from(self.scope.clone()),
            ],
        )
    }

    /// Symbols whose indexed text holds `query` as an exact phrase; empty for one word.
    pub(super) fn search_nodes_phrase(
        &self,
        query: &str,
        limit: usize,
        kind: Option<&str>,
    ) -> Result<Vec<Scored>> {
        match to_fts_phrase_query(query) {
            Some(phrase) => Ok(self
                .search_nodes_fts(&phrase, limit, kind)
                .unwrap_or_default()),
            None => Ok(Vec::new()),
        }
    }

    /// tokensave `search_nodes`: the phrase channel first, then the `OR` of the words, else `LIKE`.
    pub(super) fn search_nodes(
        &self,
        query: &str,
        limit: usize,
        kind: Option<&str>,
    ) -> Result<Vec<Scored>> {
        let Some(fts_query) = to_fts_match_query(query) else {
            return Ok(Vec::new());
        };
        let phrase_hits: Vec<Scored> = match to_fts_phrase_query(query) {
            Some(phrase) => self
                .search_nodes_fts(&phrase, limit, kind)
                .unwrap_or_default(),
            None => Vec::new(),
        };
        let merge = |general: Vec<Scored>| -> Vec<Scored> {
            if phrase_hits.is_empty() {
                return general;
            }
            let seen: HashSet<i64> = phrase_hits.iter().map(|r| r.node.id).collect();
            let mut merged = phrase_hits.clone();
            merged.extend(general.into_iter().filter(|r| !seen.contains(&r.node.id)));
            merged.truncate(limit);
            merged
        };
        let results = self.search_nodes_fts(&fts_query, limit, kind)?;
        if !results.is_empty() {
            return Ok(merge(results));
        }
        // Fallback: LIKE query
        let like_pattern = format!("%{}%", escape_like(query));
        let sql = format!(
            "SELECT {NODE_COLS} FROM symbols s
             WHERE (s.name LIKE ?1 ESCAPE '\\' OR {QUALIFIED_SQL} LIKE ?1 ESCAPE '\\'
                    OR s.doc LIKE ?1 ESCAPE '\\' OR s.sig LIKE ?1 ESCAPE '\\')
               AND {} AND (?2 IS NULL OR s.kind = ?2)
             LIMIT ?3",
            Self::vis_pos("s", 4)
        );
        let nodes = self.nodes(
            &sql,
            vec![
                Value::from(like_pattern),
                kind.map(|k| Value::from(k.to_string()))
                    .unwrap_or(Value::Null),
                Value::from(limit as i64),
                Value::from(self.scope.clone()),
            ],
        )?;
        Ok(merge(
            nodes
                .into_iter()
                .map(|node| Scored { node, score: 1.0 })
                .collect(),
        ))
    }

    /// The BM25-ranked top-`limit` candidates for one term, as a quoted prefix
    /// (tokensave `search_nodes_bounded`).
    pub(super) fn search_nodes_bounded(&self, query: &str, limit: usize) -> Result<Vec<Scored>> {
        let sanitized: String = query.chars().filter(|c| *c != '"').collect();
        if sanitized.is_empty() {
            return Ok(Vec::new());
        }
        self.search_nodes_fts(&format!("\"{sanitized}\"*"), limit, None)
    }

    /// The symbols whose name is one of `names`, in any case (tokensave
    /// `search_nodes_by_exact_name`).
    pub(super) fn search_nodes_by_exact_name(
        &self,
        names: &[String],
        limit: usize,
        kind: Option<&str>,
    ) -> Result<Vec<Node>> {
        if names.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let n = names.len();
        let marks = (1..=n)
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT {NODE_COLS} FROM symbols s
             WHERE LOWER(s.name) IN ({marks}) AND {} AND (?{} IS NULL OR s.kind = ?{})
             LIMIT ?{}",
            Self::vis_pos("s", n + 1),
            n + 2,
            n + 2,
            n + 3
        );
        let mut args: Vec<Value> = names
            .iter()
            .map(|s| Value::from(s.to_lowercase()))
            .collect();
        args.push(Value::from(self.scope.clone()));
        args.push(
            kind.map(|k| Value::from(k.to_string()))
                .unwrap_or(Value::Null),
        );
        args.push(Value::from(limit as i64));
        self.nodes(&sql, args)
    }

    /// The executable symbols whose body holds at least two distinct concepts, each by a
    /// five-character prefix, with how many, and the branches and loops of the body (tokensave
    /// `search_executable_bodies`; the node's `branches + loops` there).
    pub(super) fn search_executable_bodies(
        &self,
        terms: &[String],
        candidate_limit: usize,
    ) -> Result<Vec<(Node, usize, u32)>> {
        if terms.len() < 2 || candidate_limit == 0 {
            return Ok(Vec::new());
        }
        let mut seen = HashSet::new();
        let prefixes: Vec<String> = terms
            .iter()
            .filter_map(|term| {
                let prefix = term
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(5)
                    .collect::<String>()
                    .to_lowercase();
                (prefix.len() >= 4 && seen.insert(prefix.clone())).then_some(prefix)
            })
            .collect();
        if prefixes.len() < 2 {
            return Ok(Vec::new());
        }
        let fts_query = (0..prefixes.len())
            .flat_map(|left| {
                let prefixes = &prefixes;
                ((left + 1)..prefixes.len()).map(move |right| {
                    format!("(\"{}\"* AND \"{}\"*)", prefixes[left], prefixes[right])
                })
            })
            .collect::<Vec<_>>()
            .join(" OR ");
        let sql = format!(
            "SELECT b.rowid, b.body FROM body_fts b JOIN symbols s ON s.id = b.rowid
             WHERE body_fts MATCH ?1 AND {} LIMIT ?2",
            Self::vis_pos("s", 3)
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows: Vec<(i64, String)> = stmt
            .query_map(
                rusqlite::params![fts_query, candidate_limit as i64, self.scope],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?
            .flatten()
            .collect();
        let mut hits: HashMap<i64, (usize, u32)> = HashMap::new();
        for (id, body) in rows {
            let control = super::context::branches_and_loops(&body);
            let body = body.to_lowercase();
            let matched = prefixes
                .iter()
                .filter(|prefix| {
                    body.split(|c: char| !c.is_alphanumeric())
                        .any(|token| token.starts_with(prefix.as_str()))
                })
                .count();
            if matched >= 2 {
                hits.insert(id, (matched, control));
            }
        }
        let ids: Vec<i64> = hits.keys().copied().collect();
        let mut nodes: Vec<(Node, usize, u32)> = self
            .nodes_by_ids(&ids)?
            .into_iter()
            .filter_map(|node| {
                hits.get(&node.id)
                    .copied()
                    .map(|(count, control)| (node, count, control))
            })
            .collect();
        nodes.sort_by(|(a, a_hits, _), (b, b_hits, _)| {
            b_hits.cmp(a_hits).then_with(|| {
                a.end_line
                    .saturating_sub(a.line)
                    .cmp(&b.end_line.saturating_sub(b.line))
            })
        });
        Ok(nodes)
    }

    pub(super) fn nodes_by_ids(&self, ids: &[i64]) -> Result<Vec<Node>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let marks = (1..=ids.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("SELECT {NODE_COLS} FROM symbols s WHERE s.id IN ({marks})");
        self.nodes(&sql, ids.iter().map(|i| Value::from(*i)).collect())
    }

    /// How many calls reach each of `nodes`, by the resolved links (ESC-297). tokensave
    /// `batch_incoming_call_counts`.
    pub(super) fn batch_incoming_call_counts(&self, nodes: &[&Node]) -> Result<HashMap<i64, u64>> {
        let mut ids: Vec<i64> = nodes.iter().map(|n| n.id).collect();
        ids.sort_unstable();
        ids.dedup();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let n = ids.len();
        let marks = (1..=n)
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT l.to_id, COUNT(*) FROM links l
             WHERE l.kind = 'call' AND l.to_id IN ({marks}) AND {}
             GROUP BY l.to_id",
            Self::lvis_pos("l", n + 1)
        );
        let mut args: Vec<Value> = ids.into_iter().map(Value::from).collect();
        args.push(Value::from(self.scope.clone()));
        let mut stmt = self.conn.prepare(&sql)?;
        let counts = stmt
            .query_map(params_from_iter(args), |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)? as u64))
            })?
            .flatten()
            .collect();
        Ok(counts)
    }

    /// `index_search`: tokensave `TokenSave::search`. Over-fetches, seeds the exact names, ranks by
    /// kind tier then score (kind bonus, +10 for the very name, the path's multiplier), and puts
    /// the exact-phrase hits first.
    pub(super) fn ranked_search(
        &self,
        query: &str,
        limit: usize,
        kind: Option<&str>,
    ) -> Result<Vec<Scored>> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let overfetch = limit.saturating_mul(3).max(30);
        let mut raw = self.search_nodes(query, overfetch, kind)?;
        let mut exact_names = vec![trimmed_query.to_string()];
        if let Some(short) = trimmed_query.rsplit("::").next() {
            if short != trimmed_query && !short.is_empty() {
                exact_names.push(short.to_string());
            }
        }
        let exact = self.search_nodes_by_exact_name(&exact_names, overfetch, kind)?;
        raw.extend(exact.into_iter().map(|node| Scored { node, score: 0.0 }));

        let mut seen = HashSet::new();
        let mut ranked: Vec<Scored> = raw
            .into_iter()
            .filter(|r| seen.insert(r.node.id))
            .map(|mut r| {
                r.score += kind_rank_bonus(&r.node.kind);
                if r.node.name == trimmed_query {
                    r.score += 10.0;
                }
                r.score *= path_rank_multiplier(&r.node.path);
                r
            })
            .collect();
        ranked.sort_by(|a, b| {
            kind_tier(&a.node.kind)
                .cmp(&kind_tier(&b.node.kind))
                .then_with(|| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        let phrase_hits = self.search_nodes_phrase(trimmed_query, limit, kind)?;
        if !phrase_hits.is_empty() {
            let promoted: HashSet<i64> = phrase_hits.iter().map(|r| r.node.id).collect();
            ranked.retain(|r| !promoted.contains(&r.node.id));
            let mut merged = phrase_hits;
            merged.append(&mut ranked);
            ranked = merged;
        }
        ranked.truncate(limit);
        Ok(ranked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_query_quotes_each_word_as_a_prefix_joined_by_or() {
        assert_eq!(
            to_fts_match_query("data-api lookup").as_deref(),
            Some("\"data-api\"* OR \"lookup\"*")
        );
        assert_eq!(
            to_fts_match_query("say \"hi\"").as_deref(),
            Some("\"say\"* OR \"hi\"*")
        );
        assert_eq!(to_fts_match_query("  \"\" "), None);
    }

    #[test]
    fn phrase_query_only_for_two_words_or_more() {
        assert_eq!(
            to_fts_phrase_query("Waiting for status").as_deref(),
            Some("\"Waiting for status\"")
        );
        assert_eq!(to_fts_phrase_query("single"), None);
    }

    #[test]
    fn like_escapes_its_wildcards() {
        assert_eq!(escape_like("a_b%c\\d"), "a\\_b\\%c\\\\d");
    }
}
